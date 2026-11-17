//! `ValidJson` reads a JSON body with annotated, and answers every refusal as RFC 9457 problem details: the issues in
//! the body, up to 1,000, with 422, and a request it can't read with the status axum's own `Json` answers it with.

mod support;

use annotated::Deserialize;
use annotated_axum::ValidJson;
use axum::body::Body;
use axum::extract::DefaultBodyLimit;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use serde::Serialize;
use serde_json::json;
use support::{content_type_of, json_body_of, text_body_of};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct NewMember {
    #[validate(required, length(max = 50))]
    full_name: String,
    #[validate(email)]
    email: String,
    #[validate(range(min = 18, max = 120))]
    age: u8,
}

#[derive(Deserialize, Serialize)]
#[annotated(collect_type_errors)]
struct NewExpense {
    #[validate(required(message = "needs a description"))]
    description: String,
    #[validate(range(min = 0.01))]
    amount: f64,
}

/// An app that creates the `T` it reads at `POST /`, and answers with it.
fn creating<T>() -> Router
where
    T: for<'de> annotated::Deserialize<'de> + Serialize + Send + 'static,
{
    Router::new().route("/", post(create::<T>))
}

async fn create<T: Serialize>(ValidJson(value): ValidJson<T>) -> (StatusCode, Json<T>) {
    (StatusCode::CREATED, Json(value))
}

async fn post_json<T>(body: impl Into<Body>) -> axum::response::Response
where
    T: for<'de> annotated::Deserialize<'de> + Serialize + Send + 'static,
{
    support::post(creating::<T>(), "application/json", body).await
}

#[tokio::test]
async fn a_body_keeping_every_rule_is_read() {
    let response =
        post_json::<NewMember>(r#"{ "fullName": "Alex Morgan", "email": "alex@example.com", "age": 30 }"#).await;

    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(
        json_body_of(response).await,
        json!({ "fullName": "Alex Morgan", "email": "alex@example.com", "age": 30 })
    );
}

#[tokio::test]
async fn the_issues_in_a_body_are_answered_with_422_as_problem_details() {
    let response = post_json::<NewMember>(r#"{ "fullName": "  ", "age": 12 }"#).await;

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(content_type_of(&response), "application/problem+json");
    assert_eq!(
        json_body_of(response).await,
        json!({
            "type": "about:blank",
            "title": "Unprocessable Content",
            "status": 422,
            "errors": [
                { "pointer": "/fullName", "code": "required", "params": {}, "detail": "is required" },
                { "pointer": "/email", "code": "missing", "params": {}, "detail": "is missing" },
                {
                    "pointer": "/age",
                    "code": "range",
                    "params": { "min": 18, "max": 120 },
                    "detail": "must be between 18 and 120"
                }
            ]
        })
    );
}

#[tokio::test]
async fn a_body_whose_issues_are_all_listed_has_no_detail() {
    let response = post_json::<NewMember>(r#"{ "fullName": "  ", "age": 12 }"#).await;

    let problem = json_body_of(response).await;

    assert_eq!(problem.get("detail"), None);
}

#[tokio::test]
async fn a_body_with_more_issues_than_the_limit_is_answered_with_the_first_1000_and_a_detail_saying_it_may_have_more() {
    let response = post_json::<Vec<NewMember>>(support::a_json_list_of_empty_objects(1_001)).await;

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let problem = json_body_of(response).await;
    assert_eq!(
        problem["detail"],
        json!("there may be more issues than the 1000 listed")
    );
    assert_eq!(problem["errors"].as_array().map(Vec::len), Some(1_000));
}

#[tokio::test]
async fn a_bound_with_a_fraction_is_answered_as_a_number() {
    let response = post_json::<NewExpense>(r#"{ "description": "Lunch", "amount": 0 }"#).await;

    assert_eq!(
        json_body_of(response).await,
        json!({
            "type": "about:blank",
            "title": "Unprocessable Content",
            "status": 422,
            "errors": [
                { "pointer": "/amount", "code": "range", "params": { "min": 0.01 }, "detail": "must be at least 0.01" }
            ]
        })
    );
}

#[tokio::test]
async fn a_rule_s_own_message_is_answered_as_the_issue_s_detail() {
    let response = post_json::<NewExpense>(r#"{ "description": " ", "amount": 12.5 }"#).await;

    assert_eq!(
        json_body_of(response).await,
        json!({
            "type": "about:blank",
            "title": "Unprocessable Content",
            "status": 422,
            "errors": [
                { "pointer": "/description", "code": "required", "params": {}, "detail": "needs a description" }
            ]
        })
    );
}

#[tokio::test]
async fn a_value_of_the_wrong_type_is_answered_as_an_issue_when_its_struct_collects_type_errors() {
    let response = post_json::<NewExpense>(r#"{ "description": "Lunch", "amount": "twelve" }"#).await;

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
                    "pointer": "/amount",
                    "code": "invalid_type",
                    "params": { "expected": "f64", "found": "string" },
                    "detail": "must be f64, found string"
                }
            ]
        })
    );
}

#[tokio::test]
async fn a_value_of_the_wrong_type_is_refused_with_422_as_problem_details_when_its_struct_does_not_collect_type_errors()
{
    let response =
        post_json::<NewMember>(r#"{ "fullName": "Alex Morgan", "email": "alex@example.com", "age": "thirty" }"#).await;

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(content_type_of(&response), "application/problem+json");
    assert_eq!(
        json_body_of(response).await,
        json!({
            "type": "about:blank",
            "title": "Unprocessable Content",
            "status": 422,
            "detail": "a value at line 1 column 73 can't be read into the type expected"
        })
    );
}

#[tokio::test]
async fn a_refusal_of_a_value_of_the_wrong_type_never_echoes_the_value() {
    let response =
        post_json::<NewMember>(r#"{ "fullName": "Alex Morgan", "email": "alex@example.com", "age": "s3cr3t" }"#).await;

    let body = text_body_of(response).await;
    assert!(!body.contains("s3cr3t"), "the body echoes the client's value: {body}");
}

#[tokio::test]
async fn a_body_that_is_not_json_is_refused_with_400_as_problem_details() {
    let response = post_json::<NewMember>("not json").await;

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
async fn a_body_that_ends_before_its_json_does_is_refused_with_400_as_problem_details() {
    let response = post_json::<NewMember>(r#"{ "fullName": "#).await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(content_type_of(&response), "application/problem+json");
    assert_eq!(
        json_body_of(response).await,
        json!({
            "type": "about:blank",
            "title": "Bad Request",
            "status": 400,
            "detail": "the body is not JSON: unexpected end at line 1 column 14"
        })
    );
}

#[tokio::test]
async fn a_body_not_sent_as_json_is_refused_with_415_as_problem_details() {
    let response = support::post(
        creating::<NewMember>(),
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
async fn a_body_sent_as_json_text_is_refused_with_415() {
    let response = support::post(creating::<NewMember>(), "text/json", r#"{ "fullName": "Alex Morgan" }"#).await;

    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
}

#[tokio::test]
async fn a_body_sent_without_a_content_type_is_refused_with_415() {
    let response =
        support::post_without_a_content_type(creating::<NewMember>(), r#"{ "fullName": "Alex Morgan" }"#).await;

    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
}

#[tokio::test]
async fn a_body_whose_content_type_is_not_a_media_type_is_refused_with_415() {
    let response = support::post(creating::<NewMember>(), "json", r#"{ "fullName": "Alex Morgan" }"#).await;

    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
}

#[tokio::test]
async fn a_body_sent_as_a_media_type_built_on_json_is_read() {
    let response = support::post(
        creating::<NewMember>(),
        "application/vnd.api+json",
        r#"{ "fullName": "Alex Morgan", "email": "alex@example.com", "age": 30 }"#,
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn a_body_larger_than_the_limit_is_refused_with_413_as_problem_details() {
    let app = creating::<NewMember>().layer(DefaultBodyLimit::max(16));

    let response = support::post(app, "application/json", r#"{ "fullName": "Alex Morgan" }"#).await;

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(content_type_of(&response), "application/problem+json");
    assert_eq!(
        json_body_of(response).await,
        json!({
            "type": "about:blank",
            "title": "Content Too Large",
            "status": 413,
            "detail": "the body is larger than the server accepts"
        })
    );
}

#[tokio::test]
async fn a_body_that_fails_while_it_is_read_is_refused_with_400_as_problem_details() {
    let response = support::post_a_body_that_fails(creating::<NewMember>()).await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(content_type_of(&response), "application/problem+json");
    assert_eq!(
        json_body_of(response).await,
        json!({
            "type": "about:blank",
            "title": "Bad Request",
            "status": 400,
            "detail": "the body couldn't be read"
        })
    );
}
