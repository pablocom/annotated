//! An [axum](https://docs.rs/axum) extractor, [`ValidJson`], that reads a JSON body with
//! [annotated](https://docs.rs/annotated)'s collecting deserialization, and answers every issue in it at once.
//!
//! - **Issues** are answered with 422 and [RFC 9457](https://www.rfc-editor.org/rfc/rfc9457) problem details, which
//!   list each issue's JSON Pointer, code, params and message: [`ProblemDetails`]. That is the answer this crate
//!   recommends. An app that answers with a body of its own reads the issues from the [`ValidJsonRejection`] instead.
//! - **A request it can't read** is refused as axum's own `Json` refuses it: 415 when it isn't sent as JSON, 400
//!   when the body isn't JSON, and 422 when a value has the wrong type in a struct that doesn't collect type errors.
//!
//! ```
//! use annotated::Deserialize;
//! use annotated_axum::ValidJson;
//! use axum::Router;
//! use axum::http::StatusCode;
//! use axum::routing::post;
//!
//! #[derive(Deserialize)]
//! #[serde(rename_all = "camelCase")]
//! struct NewMember {
//!     #[validate(required, length(max = 50))]
//!     full_name: String,
//!     #[validate(range(min = 18, max = 120))]
//!     age: u8,
//! }
//!
//! async fn add_member(ValidJson(new_member): ValidJson<NewMember>) -> StatusCode {
//!     # let _ = (new_member.full_name, new_member.age);
//!     StatusCode::CREATED
//! }
//!
//! let app: Router = Router::new().route("/members", post(add_member));
//! ```
//!
//! # A response of your own
//!
//! [`ValidJsonRejection`] holds the issues as data. To answer them with a body of your own, convert the rejection
//! into an error type of yours, and read the body through `ValidJson` with axum's `FromRequest` derive:
//!
//! ```
//! use annotated::{DeserializeError, Issues};
//! use annotated_axum::{ValidJson, ValidJsonRejection};
//! use axum::extract::FromRequest;
//! use axum::http::StatusCode;
//! use axum::response::{IntoResponse, Response};
//!
//! #[derive(FromRequest)]
//! #[from_request(via(ValidJson), rejection(ApiError))]
//! struct AppJson<T>(T);
//!
//! enum ApiError {
//!     Invalid(Issues),
//!     Unreadable(ValidJsonRejection),
//! }
//!
//! impl From<ValidJsonRejection> for ApiError {
//!     fn from(rejection: ValidJsonRejection) -> Self {
//!         match rejection {
//!             ValidJsonRejection::Deserialize(DeserializeError::Issues(issues)) => Self::Invalid(issues),
//!             rejection => Self::Unreadable(rejection),
//!         }
//!     }
//! }
//!
//! impl IntoResponse for ApiError {
//!     fn into_response(self) -> Response {
//!         match self {
//!             Self::Invalid(issues) => (StatusCode::BAD_REQUEST, issues.to_string()).into_response(),
//!             Self::Unreadable(rejection) => rejection.into_response(),
//!         }
//!     }
//! }
//! ```
//!
//! # Versions
//!
//! This crate works with axum 0.8. A new version of axum that breaks its API comes with a new version of this crate.

use annotated::{DeserializeError, Issue, Issues, ParamValue};
use axum::Json;
use axum::body::Bytes;
use axum::extract::rejection::{BytesRejection, MissingJsonContentType};
use axum::extract::{FromRequest, Request};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::{Serialize, Serializer};

/// A JSON body that could be read as a `T` and breaks none of its rules, read with [`annotated::json::from_slice`].
///
/// A request it doesn't read is refused with a [`ValidJsonRejection`]: issues as [`ProblemDetails`], and anything
/// else as axum's own `Json` refuses it.
#[derive(Debug)]
pub struct ValidJson<T>(pub T);

impl<T, S> FromRequest<S> for ValidJson<T>
where
    T: for<'de> annotated::Deserialize<'de>,
    S: Send + Sync,
{
    type Rejection = ValidJsonRejection;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        if !has_json_content_type(request.headers()) {
            return Err(ValidJsonRejection::MissingJsonContentType(
                MissingJsonContentType::default(),
            ));
        }
        let body = Bytes::from_request(request, state)
            .await
            .map_err(ValidJsonRejection::BytesRejection)?;
        annotated::json::from_slice(&body)
            .map(Self)
            .map_err(ValidJsonRejection::Deserialize)
    }
}

fn has_json_content_type(headers: &HeaderMap) -> bool {
    let Some(content_type) = headers.get(header::CONTENT_TYPE).and_then(|value| value.to_str().ok()) else {
        return false;
    };
    let Ok(media_type) = content_type.parse::<mime::Mime>() else {
        return false;
    };
    media_type.type_() == "application"
        && (media_type.subtype() == "json" || media_type.suffix().is_some_and(|suffix| suffix == "json"))
}

/// Why [`ValidJson`] didn't read a request. As a response, it is the issues as [`ProblemDetails`], and anything else
/// as axum's own `Json` refuses it.
///
/// New kinds may be added without a breaking change, so a `match` on it needs a wildcard arm.
#[derive(Debug)]
#[non_exhaustive]
pub enum ValidJsonRejection {
    /// The request isn't sent as JSON: its `Content-Type` is neither `application/json` nor a type built on JSON,
    /// such as `application/vnd.api+json`. 415, as axum answers it.
    MissingJsonContentType(MissingJsonContentType),
    /// The body couldn't be read, as when it is larger than the app's limit. Answered as axum answers it.
    BytesRejection(BytesRejection),
    /// The body couldn't be read as a `T`. Its issues are [`DeserializeError::Issues`], answered with 422 as
    /// [`ProblemDetails`]. A body that isn't JSON is answered with 400, and a value of the wrong type in a struct
    /// that doesn't collect type errors with 422, both with `serde_json`'s message, as axum's `Json` answers them.
    Deserialize(DeserializeError<serde_json::Error>),
}

impl IntoResponse for ValidJsonRejection {
    fn into_response(self) -> Response {
        match self {
            Self::MissingJsonContentType(rejection) => rejection.into_response(),
            Self::BytesRejection(rejection) => rejection.into_response(),
            Self::Deserialize(DeserializeError::Format(error)) => refusal_of_unreadable_json(&error),
            Self::Deserialize(error) => ProblemDetails::of(error.issues()).into_response(),
        }
    }
}

fn refusal_of_unreadable_json(error: &serde_json::Error) -> Response {
    if error.is_data() {
        let message = format!("Failed to deserialize the JSON body into the target type: {error}");
        return (StatusCode::UNPROCESSABLE_ENTITY, message).into_response();
    }
    let message = format!("Failed to parse the request body as JSON: {error}");
    (StatusCode::BAD_REQUEST, message).into_response()
}

/// Every issue in a value, as the response to send: 422 with [RFC 9457](https://www.rfc-editor.org/rfc/rfc9457)
/// problem details, as `application/problem+json`.
///
/// Its `errors` member lists each issue's JSON Pointer, code, params and message, in the order found:
///
/// ```json
/// {
///   "type": "about:blank",
///   "title": "Unprocessable Content",
///   "status": 422,
///   "errors": [
///     { "pointer": "/age", "code": "range", "params": { "min": 18, "max": 120 }, "message": "must be between 18 and 120" }
///   ]
/// }
/// ```
///
/// It converts from [`Issues`], so a handler that returns `Result<_, ProblemDetails>` can answer the issues
/// `Validate::validate` finds with `?`.
#[derive(Debug, Serialize)]
pub struct ProblemDetails {
    r#type: &'static str,
    title: &'static str,
    status: u16,
    errors: Vec<IssueDetails>,
}

#[derive(Debug, Serialize)]
struct IssueDetails {
    pointer: String,
    code: String,
    #[serde(serialize_with = "params_as_object")]
    params: Vec<(&'static str, ParamValue)>,
    message: String,
}

impl ProblemDetails {
    fn of(issues: &[Issue]) -> Self {
        Self {
            r#type: "about:blank",
            title: "Unprocessable Content",
            status: StatusCode::UNPROCESSABLE_ENTITY.as_u16(),
            errors: issues.iter().map(IssueDetails::from).collect(),
        }
    }
}

impl From<Issues> for ProblemDetails {
    fn from(issues: Issues) -> Self {
        Self::of(issues.as_slice())
    }
}

impl From<&Issue> for IssueDetails {
    fn from(issue: &Issue) -> Self {
        Self {
            pointer: issue.path().json_pointer(),
            code: issue.problem().code().to_owned(),
            params: issue.problem().params(),
            message: issue.message(),
        }
    }
}

impl IntoResponse for ProblemDetails {
    fn into_response(self) -> Response {
        let problem_json = [(header::CONTENT_TYPE, "application/problem+json")];
        (StatusCode::UNPROCESSABLE_ENTITY, problem_json, Json(self)).into_response()
    }
}

fn params_as_object<S: Serializer>(params: &[(&'static str, ParamValue)], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_map(params.iter().map(|(name, value)| (name, Param(value))))
}

struct Param<'a>(&'a ParamValue);

impl Serialize for Param<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            ParamValue::Integer(integer) => serializer.serialize_i128(*integer),
            ParamValue::Float(float) => serializer.serialize_f64(*float),
            text => serializer.collect_str(text),
        }
    }
}
