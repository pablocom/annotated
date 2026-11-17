//! `Option<ValidJson<T>>` is `None` for a request sent without a `Content-Type`, as `Option<Json<T>>` is in axum, and
//! refuses any other request `ValidJson` refuses.

#[allow(dead_code, reason = "each test file uses some of the helpers")]
mod support;

use annotated::Deserialize;
use annotated_axum::ValidJson;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use serde::Serialize;
use serde_json::json;
use support::{content_type_of, json_body_of};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct NewMember {
    #[validate(required, length(max = 50))]
    full_name: String,
}

/// An app that creates the member it reads at `POST /`, and answers 204 when it reads none.
fn creating_a_member_when_given_one() -> Router {
    Router::new().route("/", post(create_when_given))
}

async fn create_when_given(
    new_member: Option<ValidJson<NewMember>>,
) -> Result<(StatusCode, Json<NewMember>), StatusCode> {
    let ValidJson(new_member) = new_member.ok_or(StatusCode::NO_CONTENT)?;
    Ok((StatusCode::CREATED, Json(new_member)))
}

#[tokio::test]
async fn a_body_sent_without_a_content_type_is_read_as_none() {
    let response =
        support::post_without_a_content_type(creating_a_member_when_given_one(), r#"{ "fullName": "Alex Morgan" }"#)
            .await;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn a_body_sent_as_json_is_read_as_some() {
    let response = support::post(
        creating_a_member_when_given_one(),
        "application/json",
        r#"{ "fullName": "Alex Morgan" }"#,
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(json_body_of(response).await, json!({ "fullName": "Alex Morgan" }));
}

#[tokio::test]
async fn a_body_not_sent_as_json_is_refused_with_415_as_problem_details() {
    let response = support::post(
        creating_a_member_when_given_one(),
        "text/plain",
        r#"{ "fullName": "Alex Morgan" }"#,
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_eq!(content_type_of(&response), "application/problem+json");
    assert_eq!(
        json_body_of(response).await,
        json!({
            "type": "about:blank",
            "title": "Unsupported Media Type",
            "status": 415,
            "detail": "the request isn't sent as JSON: its Content-Type must be application/json or end in +json"
        })
    );
}

#[tokio::test]
async fn a_body_sent_as_json_with_issues_is_refused_with_422_as_problem_details() {
    let response = support::post(
        creating_a_member_when_given_one(),
        "application/json",
        r#"{ "fullName": " " }"#,
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(content_type_of(&response), "application/problem+json");
    assert_eq!(
        json_body_of(response).await,
        json!({
            "type": "about:blank",
            "title": "Unprocessable Content",
            "status": 422,
            "errors": [
                { "pointer": "/fullName", "code": "required", "params": {}, "detail": "is required" }
            ]
        })
    );
}
