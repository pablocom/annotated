//! An axum extractor that reads a JSON body with [collecting deserialization](annotated::Deserialize), and
//! answers every violation in it at once as [RFC 9457](https://www.rfc-editor.org/rfc/rfc9457) problem details.

use annotated::{DeserializeError, ParamValue, Violation};
use axum::Json;
use axum::body::Bytes;
use axum::extract::rejection::MissingJsonContentType;
use axum::extract::{FromRequest, Request};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::{Serialize, Serializer};

/// A JSON body that came whole and breaks none of its rules, read with [`annotated::json::from_slice`].
///
/// It refuses a request as axum's [`Json`] does: 415 when it isn't sent as JSON, 400 when its body isn't JSON,
/// and 422 when a value is of the wrong type for a struct that doesn't collect type errors. A body with
/// violations is answered with 422 and [`ProblemDetails`] listing every one.
pub struct ValidJson<T>(pub T);

impl<T, S> FromRequest<S> for ValidJson<T>
where
    T: for<'de> annotated::Deserialize<'de>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        if !has_json_content_type(request.headers()) {
            return Err(MissingJsonContentType::default().into_response());
        }
        let body = Bytes::from_request(request, state)
            .await
            .map_err(IntoResponse::into_response)?;
        match annotated::json::from_slice(&body) {
            Ok(value) => Ok(Self(value)),
            Err(DeserializeError::Format(error)) => Err(refusal_of_unreadable_json(&error)),
            Err(error) => Err(ProblemDetails::from(error.violations()).into_response()),
        }
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

fn refusal_of_unreadable_json(error: &serde_json::Error) -> Response {
    if error.is_data() {
        let message = format!("Failed to deserialize the JSON body into the target type: {error}");
        return (StatusCode::UNPROCESSABLE_ENTITY, message).into_response();
    }
    let message = format!("Failed to parse the request body as JSON: {error}");
    (StatusCode::BAD_REQUEST, message).into_response()
}

/// Every violation in a body, answered with 422 as [RFC 9457](https://www.rfc-editor.org/rfc/rfc9457) problem
/// details, `application/problem+json`, whose `errors` member lists each one's JSON Pointer, code, params and
/// message.
#[derive(Serialize)]
pub struct ProblemDetails {
    r#type: &'static str,
    title: &'static str,
    status: u16,
    errors: Vec<ViolationDetails>,
}

#[derive(Serialize)]
struct ViolationDetails {
    pointer: String,
    code: String,
    #[serde(serialize_with = "params_as_object")]
    params: Vec<(&'static str, ParamValue)>,
    message: String,
}

impl From<&[Violation]> for ProblemDetails {
    fn from(violations: &[Violation]) -> Self {
        Self {
            r#type: "about:blank",
            title: "Unprocessable Content",
            status: StatusCode::UNPROCESSABLE_ENTITY.as_u16(),
            errors: violations.iter().map(ViolationDetails::from).collect(),
        }
    }
}

impl From<&Violation> for ViolationDetails {
    fn from(violation: &Violation) -> Self {
        Self {
            pointer: violation.path().json_pointer(),
            code: violation.problem().code().to_owned(),
            params: violation.problem().params(),
            message: violation.message(),
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
