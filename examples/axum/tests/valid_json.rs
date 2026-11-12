//! `ValidJson` reads a JSON body as axum's `Json` does, and answers every issue in it with 422 as RFC 9457
//! problem details.

use annotated::Deserialize;
use annotated_example_axum::ValidJson;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use axum::routing::post;
use axum::{Json, Router};
use http_body_util::BodyExt;
use serde::Serialize;
use serde_json::{Value, json};
use tower::ServiceExt;

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

#[tokio::test]
async fn a_new_member_keeping_every_rule_is_created() {
    let response =
        send_json::<NewMember>(r#"{ "fullName": "Alex Morgan", "email": "alex@example.com", "age": 30 }"#).await;

    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(
        json_body_of(response).await,
        json!({ "fullName": "Alex Morgan", "email": "alex@example.com", "age": 30 })
    );
}

#[tokio::test]
async fn every_issue_in_a_body_is_answered_with_422_as_problem_details() {
    let response = send_json::<NewMember>(r#"{ "fullName": "  ", "age": 12 }"#).await;

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(content_type_of(&response), "application/problem+json");
    assert_eq!(
        json_body_of(response).await,
        json!({
            "type": "about:blank",
            "title": "Unprocessable Content",
            "status": 422,
            "errors": [
                { "pointer": "/fullName", "code": "required", "params": {}, "message": "is required" },
                { "pointer": "/email", "code": "missing", "params": {}, "message": "is missing" },
                {
                    "pointer": "/age",
                    "code": "range",
                    "params": { "min": 18, "max": 120 },
                    "message": "must be between 18 and 120"
                }
            ]
        })
    );
}

#[tokio::test]
async fn a_bound_with_a_fraction_is_answered_as_a_number() {
    let response = send_json::<NewExpense>(r#"{ "description": "Lunch", "amount": 0 }"#).await;

    assert_eq!(
        json_body_of(response).await,
        json!({
            "type": "about:blank",
            "title": "Unprocessable Content",
            "status": 422,
            "errors": [
                { "pointer": "/amount", "code": "range", "params": { "min": 0.01 }, "message": "must be at least 0.01" }
            ]
        })
    );
}

#[tokio::test]
async fn a_rule_s_own_message_is_answered_as_the_issue_s_message() {
    let response = send_json::<NewExpense>(r#"{ "description": " ", "amount": 12.5 }"#).await;

    assert_eq!(
        json_body_of(response).await,
        json!({
            "type": "about:blank",
            "title": "Unprocessable Content",
            "status": 422,
            "errors": [
                { "pointer": "/description", "code": "required", "params": {}, "message": "needs a description" }
            ]
        })
    );
}

#[tokio::test]
async fn a_value_of_the_wrong_type_is_answered_as_an_issue_when_its_struct_collects_type_errors() {
    let response = send_json::<NewExpense>(r#"{ "description": "Lunch", "amount": "twelve" }"#).await;

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
                    "message": "must be f64, found string"
                }
            ]
        })
    );
}

#[tokio::test]
async fn a_value_of_the_wrong_type_is_refused_as_axum_refuses_it_when_its_struct_does_not_collect_type_errors() {
    let response =
        send_json::<NewMember>(r#"{ "fullName": "Alex Morgan", "email": "alex@example.com", "age": "thirty" }"#).await;

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        text_body_of(response).await,
        r#"Failed to deserialize the JSON body into the target type: invalid type: string "thirty", expected u8 at line 1 column 73"#
    );
}

#[tokio::test]
async fn a_body_that_is_not_json_is_refused_with_400() {
    let response = send_json::<NewMember>("not json").await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        text_body_of(response).await,
        "Failed to parse the request body as JSON: expected ident at line 1 column 2"
    );
}

#[tokio::test]
async fn a_body_not_sent_as_json_is_refused_with_415() {
    let response = send::<NewMember>("text/plain", r#"{ "fullName": "Alex Morgan" }"#).await;

    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_eq!(
        text_body_of(response).await,
        "Expected request with `Content-Type: application/json`"
    );
}

#[tokio::test]
async fn a_body_sent_as_a_media_type_built_on_json_is_read() {
    let response = send::<NewMember>(
        "application/vnd.api+json",
        r#"{ "fullName": "Alex Morgan", "email": "alex@example.com", "age": 30 }"#,
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);
}

async fn send_json<T>(body: &'static str) -> Response
where
    T: for<'de> annotated::Deserialize<'de> + Serialize + Send + 'static,
{
    send::<T>("application/json", body).await
}

async fn send<T>(content_type: &'static str, body: &'static str) -> Response
where
    T: for<'de> annotated::Deserialize<'de> + Serialize + Send + 'static,
{
    let app = Router::new().route("/", post(create::<T>));
    let request = Request::post("/")
        .header(header::CONTENT_TYPE, content_type)
        .body(Body::from(body))
        .expect("a request with a body");

    app.oneshot(request).await.expect("a router always answers")
}

async fn create<T: Serialize>(ValidJson(value): ValidJson<T>) -> (StatusCode, Json<T>) {
    (StatusCode::CREATED, Json(value))
}

fn content_type_of(response: &Response) -> &str {
    response.headers()[header::CONTENT_TYPE]
        .to_str()
        .expect("a content type in ASCII")
}

async fn text_body_of(response: Response) -> String {
    let body = response.into_body().collect().await.expect("a whole body").to_bytes();
    String::from_utf8(body.to_vec()).expect("a body in UTF-8")
}

async fn json_body_of(response: Response) -> Value {
    let body = response.into_body().collect().await.expect("a whole body").to_bytes();
    serde_json::from_slice(&body).expect("a body in JSON")
}
