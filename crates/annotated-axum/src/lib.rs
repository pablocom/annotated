//! An [axum](https://docs.rs/axum) extractor, [`ValidJson`], that reads a JSON body with
//! [annotated](https://docs.rs/annotated)'s collecting deserialization, and answers every issue in it at once.
//!
//! Every refusal is [RFC 9457](https://www.rfc-editor.org/rfc/rfc9457) problem details, as
//! `application/problem+json`: [`ProblemDetails`].
//!
//! - **Issues** are answered with 422, and the problem details list each issue's JSON Pointer, code, params and
//!   message. That is the answer this crate recommends. An app that answers with a body of its own reads the issues
//!   from the [`ValidJsonRejection`] instead.
//! - **A request it can't read** is answered with the status axum's own `Json` answers it with: 415 when it isn't
//!   sent as JSON, 413 when its body is larger than the app's limit, 400 when the body isn't JSON, and 422 when a
//!   value has the wrong type in a struct that doesn't collect type errors. The problem details' `detail` says what
//!   is wrong and where, and never repeats the value the client sent. A struct with
//!   `#[annotated(collect_type_errors)]` has its values of the wrong type answered as issues instead, at their JSON
//!   Pointers.
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
use axum::extract::rejection::{BytesRejection, FailedToBufferBody, MissingJsonContentType};
use axum::extract::{FromRequest, Request};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::{Serialize, Serializer};
use serde_json::error::Category;

/// A JSON body that could be read as a `T` and breaks none of its rules, read with [`annotated::json::from_slice`].
///
/// A request it doesn't read is refused with a [`ValidJsonRejection`], answered as [`ProblemDetails`].
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

/// Why [`ValidJson`] didn't read a request. As a response, it is [`ProblemDetails`], with the status axum's own
/// `Json` answers the same request with.
///
/// New kinds may be added without a breaking change, so a `match` on it needs a wildcard arm.
#[derive(Debug)]
#[non_exhaustive]
pub enum ValidJsonRejection {
    /// The request isn't sent as JSON: its `Content-Type` is neither `application/json` nor a type built on JSON,
    /// such as `application/vnd.api+json`. Answered with 415.
    MissingJsonContentType(MissingJsonContentType),
    /// The body couldn't be read: 413 when it is larger than the app's limit, and 400 when reading it failed.
    BytesRejection(BytesRejection),
    /// The body couldn't be read as a `T`. Its issues are [`DeserializeError::Issues`], answered with 422 and listed
    /// in the problem details. `serde_json`'s own error is answered with 400 when the body isn't JSON, and with 422
    /// when a value has the wrong type in a struct that doesn't collect type errors, with a `detail` that says
    /// where, and never `serde_json`'s message, which repeats the value.
    Deserialize(DeserializeError<serde_json::Error>),
}

impl ValidJsonRejection {
    fn status(&self) -> StatusCode {
        match self {
            Self::MissingJsonContentType(rejection) => rejection.status(),
            Self::BytesRejection(rejection) => rejection.status(),
            Self::Deserialize(DeserializeError::Format(error)) if !error.is_data() => StatusCode::BAD_REQUEST,
            Self::Deserialize(_) => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }

    fn detail(&self) -> String {
        match self {
            Self::MissingJsonContentType(_) => {
                "the request isn't sent as JSON: its Content-Type must be application/json or end in +json".to_owned()
            }
            Self::BytesRejection(BytesRejection::FailedToBufferBody(FailedToBufferBody::LengthLimitError(_))) => {
                "the body is larger than the server accepts".to_owned()
            }
            Self::BytesRejection(_) => "the body couldn't be read".to_owned(),
            Self::Deserialize(DeserializeError::Format(error)) => what_is_wrong_with_the_json(error),
            Self::Deserialize(_) => "the body has issues".to_owned(),
        }
    }
}

fn what_is_wrong_with_the_json(error: &serde_json::Error) -> String {
    let (line, column) = (error.line(), error.column());
    match error.classify() {
        Category::Data => format!("a value at line {line} column {column} can't be read into the type expected"),
        Category::Eof => format!("the body is not JSON: unexpected end at line {line} column {column}"),
        Category::Syntax | Category::Io => format!("the body is not JSON: syntax error at line {line} column {column}"),
    }
}

impl IntoResponse for ValidJsonRejection {
    fn into_response(self) -> Response {
        match self {
            Self::Deserialize(DeserializeError::Issues(issues)) => ProblemDetails::from(issues).into_response(),
            rejection => ProblemDetails::of_a_refusal(rejection.status(), rejection.detail()).into_response(),
        }
    }
}

/// A refusal, as the response to send: [RFC 9457](https://www.rfc-editor.org/rfc/rfc9457) problem details, as
/// `application/problem+json`.
///
/// The problem details of issues are answered with 422, and their `errors` member lists each issue's JSON Pointer,
/// code, params and message, in the order found:
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
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<&'static str>,
    #[serde(serialize_with = "status_as_number")]
    status: StatusCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
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
    fn new(status: StatusCode) -> Self {
        Self {
            r#type: "about:blank",
            title: reason_phrase(status),
            status,
            detail: None,
            errors: Vec::new(),
        }
    }

    fn of_a_refusal(status: StatusCode, detail: String) -> Self {
        Self {
            detail: Some(detail),
            ..Self::new(status)
        }
    }
}

/// The reason phrase of `status` as [RFC 9110](https://www.rfc-editor.org/rfc/rfc9110#section-15) names it, which
/// renamed two that the `http` crate still calls by their old names.
fn reason_phrase(status: StatusCode) -> Option<&'static str> {
    match status {
        StatusCode::PAYLOAD_TOO_LARGE => Some("Content Too Large"),
        StatusCode::UNPROCESSABLE_ENTITY => Some("Unprocessable Content"),
        status => status.canonical_reason(),
    }
}

impl From<Issues> for ProblemDetails {
    fn from(issues: Issues) -> Self {
        Self {
            errors: issues.iter().map(IssueDetails::from).collect(),
            ..Self::new(StatusCode::UNPROCESSABLE_ENTITY)
        }
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
        (self.status, problem_json, Json(self)).into_response()
    }
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's `serialize_with` passes the field by reference"
)]
fn status_as_number<S: Serializer>(status: &StatusCode, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_u16(status.as_u16())
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
