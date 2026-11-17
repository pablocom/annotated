//! `ValidJsonRejection` is an error, which says why a request was refused, with the status it is answered with, so an
//! app can log it, and the error it came from as its source.

#[allow(dead_code, reason = "each test file uses some of the helpers")]
mod support;

use std::error::Error;

use annotated::Deserialize;
use annotated_axum::{ValidJson, ValidJsonRejection};
use axum::body::Body;
use axum::extract::FromRequest;
use axum::http::{Request, StatusCode, header};
use http_body_util::Limited;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(dead_code, reason = "the fields are read from requests, and never used")]
struct NewMember {
    #[validate(required, length(max = 50))]
    full_name: String,
    #[validate(email)]
    email: String,
    #[validate(range(min = 18, max = 120))]
    age: u8,
}

/// How a rejection describes itself: the status it is answered with, its `Display`, and its source's `Display`.
#[derive(Debug, PartialEq)]
struct Description {
    status: StatusCode,
    text: String,
    source: Option<String>,
}

fn description_of(rejection: &ValidJsonRejection) -> Description {
    Description {
        status: rejection.status(),
        text: rejection.to_string(),
        source: rejection.source().map(ToString::to_string),
    }
}

async fn rejection_of(content_type: &'static str, body: Body) -> ValidJsonRejection {
    rejection_of_reading::<NewMember>(content_type, body).await
}

async fn rejection_of_reading<T>(content_type: &'static str, body: Body) -> ValidJsonRejection
where
    T: for<'de> annotated::Deserialize<'de> + std::fmt::Debug,
{
    let request = Request::post("/")
        .header(header::CONTENT_TYPE, content_type)
        .body(body)
        .expect("a request with a body");

    <ValidJson<T> as FromRequest<()>>::from_request(request, &())
        .await
        .expect_err("a request ValidJson refuses")
}

async fn rejection_of_json(body: &'static str) -> ValidJsonRejection {
    rejection_of("application/json", Body::from(body)).await
}

async fn rejection_of_a_json_list_of_members(body: String) -> ValidJsonRejection {
    rejection_of_reading::<Vec<NewMember>>("application/json", Body::from(body)).await
}

#[tokio::test]
async fn a_rejection_of_a_request_not_sent_as_json_says_so() {
    let rejection = rejection_of("text/plain", Body::from(r#"{ "fullName": "Alex Morgan" }"#)).await;

    assert_eq!(
        description_of(&rejection),
        Description {
            status: StatusCode::UNSUPPORTED_MEDIA_TYPE,
            text: "the request isn't sent as JSON: its Content-Type must be application/json or end in +json"
                .to_owned(),
            source: Some("Expected request with `Content-Type: application/json`".to_owned()),
        }
    );
}

#[tokio::test]
async fn a_rejection_of_a_body_larger_than_the_limit_says_so() {
    let body_over_its_limit = Body::new(Limited::new(Body::from(r#"{ "fullName": "Alex Morgan" }"#), 16));

    let rejection = rejection_of("application/json", body_over_its_limit).await;

    assert_eq!(
        description_of(&rejection),
        Description {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            text: "the body is larger than the server accepts".to_owned(),
            source: Some("Failed to buffer the request body: length limit exceeded".to_owned()),
        }
    );
}

#[tokio::test]
async fn a_rejection_of_a_body_that_fails_while_it_is_read_says_so() {
    let rejection = rejection_of("application/json", support::a_body_that_fails()).await;

    assert_eq!(
        description_of(&rejection),
        Description {
            status: StatusCode::BAD_REQUEST,
            text: "the body couldn't be read".to_owned(),
            source: Some("Failed to buffer the request body: the connection broke".to_owned()),
        }
    );
}

#[tokio::test]
async fn a_rejection_of_a_body_that_is_not_json_says_where_and_has_serde_json_s_error_as_its_source() {
    let rejection = rejection_of_json("not json").await;

    assert_eq!(
        description_of(&rejection),
        Description {
            status: StatusCode::BAD_REQUEST,
            text: "the body is not JSON: syntax error at line 1 column 2".to_owned(),
            source: Some("expected ident at line 1 column 2".to_owned()),
        }
    );
}

#[tokio::test]
async fn a_rejection_of_a_value_of_the_wrong_type_says_where_and_has_serde_json_s_error_as_its_source() {
    let rejection =
        rejection_of_json(r#"{ "fullName": "Alex Morgan", "email": "alex@example.com", "age": "thirty" }"#).await;

    assert_eq!(
        description_of(&rejection),
        Description {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            text: "a value at line 1 column 73 can't be read into the type expected".to_owned(),
            source: Some(r#"invalid type: string "thirty", expected u8 at line 1 column 73"#.to_owned()),
        }
    );
}

#[tokio::test]
async fn a_rejection_of_issues_has_the_issues_as_its_source() {
    let rejection = rejection_of_json(r#"{ "fullName": "  ", "age": 12 }"#).await;

    assert_eq!(
        description_of(&rejection),
        Description {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            text: "the body has 3 issues".to_owned(),
            source: Some("fullName: is required\nemail: is missing\nage: must be between 18 and 120".to_owned()),
        }
    );
}

#[tokio::test]
async fn a_rejection_of_one_issue_says_how_many_in_the_singular() {
    let rejection = rejection_of_json(r#"{ "fullName": "Alex Morgan", "email": "alex@example.com", "age": 12 }"#).await;

    let text = rejection.to_string();

    assert_eq!(text, "the body has 1 issue");
}

#[tokio::test]
async fn a_rejection_of_three_issues_says_how_many() {
    let rejection = rejection_of_json(r#"{ "fullName": "  ", "age": 12 }"#).await;

    let text = rejection.to_string();

    assert_eq!(text, "the body has 3 issues");
}

#[tokio::test]
async fn a_rejection_of_issues_cut_off_at_the_limit_says_the_body_may_have_more() {
    let rejection = rejection_of_a_json_list_of_members(support::a_json_list_of_empty_objects(1_001)).await;

    let text = rejection.to_string();

    assert_eq!(text, "the body has 1000 issues, and may have more");
}

#[tokio::test]
async fn the_body_text_of_a_rejection_of_issues_says_how_many() {
    let rejection = rejection_of_json(r#"{ "fullName": "  ", "age": 12 }"#).await;

    let body_text = rejection.body_text();

    assert_eq!(body_text, "the body has 3 issues");
}

#[tokio::test]
async fn the_body_text_of_a_rejection_says_why_it_was_refused() {
    let rejection = rejection_of_json("not json").await;

    let body_text = rejection.body_text();

    assert_eq!(body_text, "the body is not JSON: syntax error at line 1 column 2");
}
