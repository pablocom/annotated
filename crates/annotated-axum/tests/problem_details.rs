//! `ProblemDetails` is RFC 9457 problem details, whose members an app can set to its own.

#[allow(dead_code, reason = "each test file uses some of the helpers")]
mod support;

use annotated::{Issues, Validate};
use annotated_axum::ProblemDetails;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_json::json;
use support::{content_type_of, json_body_of};

#[derive(Validate)]
struct NewPassword {
    #[validate(length(min = 8))]
    password: String,
}

fn issues_of_a_short_password() -> Issues {
    let new_password = NewPassword {
        password: "hunter2".to_owned(),
    };
    new_password
        .validate()
        .expect_err("a password shorter than 8 characters")
}

#[tokio::test]
async fn a_problem_is_answered_with_its_status_as_problem_json() {
    let problem = ProblemDetails::new(StatusCode::CONFLICT);

    let response = problem.into_response();

    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(content_type_of(&response), "application/problem+json");
}

#[tokio::test]
async fn a_problem_has_the_type_about_blank_and_its_status_s_reason_phrase_as_its_title_by_default() {
    let problem = ProblemDetails::new(StatusCode::CONFLICT);

    let response = problem.into_response();

    assert_eq!(
        json_body_of(response).await,
        json!({ "type": "about:blank", "title": "Conflict", "status": 409 })
    );
}

#[tokio::test]
async fn a_problem_whose_status_has_no_reason_phrase_has_no_title() {
    let status_without_a_reason_phrase = StatusCode::from_u16(499).expect("a status code");
    let problem = ProblemDetails::new(status_without_a_reason_phrase);

    let response = problem.into_response();

    assert_eq!(
        json_body_of(response).await,
        json!({ "type": "about:blank", "status": 499 })
    );
}

#[tokio::test]
async fn a_problem_answers_the_members_an_app_sets() {
    let problem = ProblemDetails::new(StatusCode::CONFLICT)
        .with_type("https://example.com/problems/email-taken")
        .with_title("The email is taken")
        .with_detail(String::from("alex@example.com belongs to another member"))
        .with_instance("/members/42");

    let response = problem.into_response();

    assert_eq!(
        json_body_of(response).await,
        json!({
            "type": "https://example.com/problems/email-taken",
            "title": "The email is taken",
            "status": 409,
            "detail": "alex@example.com belongs to another member",
            "instance": "/members/42"
        })
    );
}

#[tokio::test]
async fn the_problem_details_of_issues_keep_their_errors_under_a_type_of_the_app_s() {
    let problem = ProblemDetails::from(issues_of_a_short_password())
        .with_type("https://example.com/problems/invalid-body")
        .with_title("The body is invalid");

    let response = problem.into_response();

    assert_eq!(
        json_body_of(response).await,
        json!({
            "type": "https://example.com/problems/invalid-body",
            "title": "The body is invalid",
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

#[test]
fn the_problem_details_of_issues_have_the_status_422() {
    let problem = ProblemDetails::from(issues_of_a_short_password());

    let status = problem.status();

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}
