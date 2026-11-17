//! A rejection of `ValidJson` holds the issues as data, so an app can answer them with a response of its own. And
//! `ProblemDetails` answers the issues `Validate::validate` finds.

// The fields here are read from requests, and never used.
#![allow(dead_code)]

mod support;

use annotated::{DeserializeError, Issues, Validate};
use annotated_axum::{ProblemDetails, ValidJson, ValidJsonRejection};
use axum::extract::FromRequest;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use serde_json::json;
use support::{content_type_of, json_body_of};

#[derive(annotated::Deserialize)]
#[serde(rename_all = "camelCase")]
struct NewMember {
    #[validate(required, length(max = 50))]
    full_name: String,
    #[validate(range(min = 18, max = 120))]
    age: u8,
}

/// A JSON body read by `ValidJson`, refused with the app's own response.
#[derive(FromRequest)]
#[from_request(via(ValidJson), rejection(ApiError))]
struct AppJson<T>(T);

enum ApiError {
    Invalid(Issues),
    Unreadable(ValidJsonRejection),
}

impl From<ValidJsonRejection> for ApiError {
    fn from(rejection: ValidJsonRejection) -> Self {
        match rejection {
            ValidJsonRejection::Deserialize(DeserializeError::Issues(issues)) => Self::Invalid(issues),
            rejection => Self::Unreadable(rejection),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            Self::Invalid(issues) => {
                let fields: Vec<String> = issues.iter().map(|issue| issue.path().to_string()).collect();
                let body = json!({ "message": "Some fields are wrong", "fields": fields });
                (StatusCode::BAD_REQUEST, Json(body)).into_response()
            }
            Self::Unreadable(rejection) => rejection.into_response(),
        }
    }
}

async fn add_member(AppJson(_new_member): AppJson<NewMember>) -> StatusCode {
    StatusCode::CREATED
}

#[derive(serde::Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
struct NewPassword {
    #[validate(length(min = 8))]
    password: String,
}

async fn change_password(Json(new_password): Json<NewPassword>) -> Result<StatusCode, ProblemDetails> {
    new_password.validate()?;
    Ok(StatusCode::NO_CONTENT)
}

#[tokio::test]
async fn issues_can_be_answered_with_a_response_of_your_own() {
    let app = Router::new().route("/", post(add_member));

    let response = support::post(app, "application/json", r#"{ "fullName": "  ", "age": 12 }"#).await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        json_body_of(response).await,
        json!({ "message": "Some fields are wrong", "fields": ["fullName", "age"] })
    );
}

#[tokio::test]
async fn a_rejection_of_your_own_can_refuse_the_rest_as_valid_json_does() {
    let app = Router::new().route("/", post(add_member));

    let response = support::post(app, "application/json", "not json").await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(content_type_of(&response), "application/problem+json");
    assert_eq!(
        json_body_of(response).await,
        json!({
            "type": "about:blank",
            "title": "Bad Request",
            "status": 400,
            "detail": "the body is not JSON: syntax error at line 1 column 2"
        })
    );
}

#[tokio::test]
async fn issues_found_by_validate_are_answered_as_problem_details() {
    let app = Router::new().route("/", post(change_password));

    let response = support::post(app, "application/json", r#"{ "password": "hunter2" }"#).await;

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(content_type_of(&response), "application/problem+json");
    assert_eq!(
        json_body_of(response).await,
        json!({
            "type": "about:blank",
            "title": "Unprocessable Content",
            "status": 422,
            "errors": [
                {
                    "pointer": "/password",
                    "code": "length",
                    "params": { "min": 8, "unit": "chars" },
                    "detail": "must be at least 8 characters long"
                }
            ]
        })
    );
}
