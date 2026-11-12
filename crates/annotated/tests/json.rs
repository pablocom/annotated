//! `annotated::json` reads JSON text or bytes collecting every violation, and refuses trailing characters as
//! `serde_json` does.

#![cfg(feature = "json")]

use annotated::{Deserialize, ParamValue, Path, Problem, Validate, Violation};

#[derive(Validate, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct NewMember {
    #[validate(required, length(max = 50))]
    full_name: String,
    #[validate(email)]
    email: String,
    #[validate(range(min = 18, max = 120))]
    age: u8,
}

fn alex() -> NewMember {
    NewMember {
        full_name: "Alex Morgan".to_owned(),
        email: "alex@example.com".to_owned(),
        age: 30,
    }
}

const ALEX: &str = r#"{ "fullName": "Alex Morgan", "email": "alex@example.com", "age": 30 }"#;

#[test]
fn every_problem_with_a_new_member_is_reported_at_once() {
    let body = r#"{ "fullName": "  ", "age": 12 }"#;

    let error = annotated::json::from_str::<NewMember>(body).unwrap_err();

    let [blank_name, missing_email, too_young] = error.violations() else {
        panic!("expected three violations, got {error:?}");
    };
    assert_eq!(blank_name.path(), &Path::root().field("fullName"));
    assert_eq!(blank_name.problem(), &Problem::Required);
    assert_eq!(missing_email.path(), &Path::root().field("email"));
    assert_eq!(missing_email.problem(), &Problem::Missing);
    assert_eq!(too_young.path(), &Path::root().field("age"));
    assert_eq!(too_young.problem().code(), "range");
    assert_eq!(
        too_young.problem().params(),
        [("min", ParamValue::Integer(18)), ("max", ParamValue::Integer(120))],
    );
}

#[test]
fn text_breaking_no_rule_is_read() {
    assert_eq!(annotated::json::from_str::<NewMember>(ALEX).unwrap(), alex());
}

#[test]
fn bytes_breaking_no_rule_are_read() {
    assert_eq!(
        annotated::json::from_slice::<NewMember>(ALEX.as_bytes()).unwrap(),
        alex()
    );
}

#[test]
fn every_violation_in_bytes_is_answered() {
    let body = br#"{ "fullName": "Alex Morgan", "age": 12 }"#;

    let error = annotated::json::from_slice::<NewMember>(body).unwrap_err();

    assert_eq!(error.to_string(), "email: is missing\nage: must be between 18 and 120");
}

#[test]
fn trailing_characters_after_text_are_the_format_s_error() {
    let body = r#"{ "fullName": "Alex Morgan", "email": "alex@example.com", "age": 30 } }"#;

    let error = annotated::json::from_str::<NewMember>(body).unwrap_err();

    assert_eq!(error.to_string(), "trailing characters at line 1 column 71");
}

#[test]
fn trailing_characters_after_bytes_are_the_format_s_error() {
    let body = br#"{ "fullName": "Alex Morgan", "email": "alex@example.com", "age": 30 } }"#;

    let error = annotated::json::from_slice::<NewMember>(body).unwrap_err();

    assert_eq!(error.to_string(), "trailing characters at line 1 column 71");
}

#[test]
fn trailing_characters_are_the_format_s_error_even_after_a_value_breaking_rules() {
    let body = r#"{ "fullName": "", "age": 12 } }"#;

    let error = annotated::json::from_str::<NewMember>(body).unwrap_err();

    assert_eq!(
        (error.to_string(), error.violations()),
        ("trailing characters at line 1 column 31".to_owned(), [].as_slice())
    );
}

#[test]
fn failing_fast_answers_only_the_first_violation_in_text() {
    let body = r#"{ "fullName": "", "age": 12 }"#;

    let error = annotated::json::from_str_fail_fast::<NewMember>(body).unwrap_err();

    assert_eq!(
        error.violations(),
        [Violation::new(Path::root().field("fullName"), Problem::Required)]
    );
}

#[test]
fn failing_fast_answers_only_the_first_violation_in_bytes() {
    let body = br#"{ "fullName": "", "age": 12 }"#;

    let error = annotated::json::from_slice_fail_fast::<NewMember>(body).unwrap_err();

    assert_eq!(
        error.violations(),
        [Violation::new(Path::root().field("fullName"), Problem::Required)]
    );
}

#[test]
fn failing_fast_refuses_trailing_characters() {
    let body = r#"{ "fullName": "", "age": 12 } }"#;

    let error = annotated::json::from_str_fail_fast::<NewMember>(body).unwrap_err();

    assert_eq!(error.to_string(), "trailing characters at line 1 column 31");
}
