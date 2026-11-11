//! `from_deserializer_fail_fast` answers only the first violation: the first that `from_deserializer` lists.

#![cfg(feature = "serde")]

use annotated::{Deserialize, DeserializeError, Problem, Violation};

fn violations_reading<'de, T: Deserialize<'de>>(json: &'de str) -> Vec<String> {
    let outcome = annotated::from_deserializer::<T, _>(&mut serde_json::Deserializer::from_str(json));
    paths_and_codes(outcome)
}

fn fail_fast_violations_reading<'de, T: Deserialize<'de>>(json: &'de str) -> Vec<String> {
    let outcome = annotated::from_deserializer_fail_fast::<T, _>(&mut serde_json::Deserializer::from_str(json));
    paths_and_codes(outcome)
}

fn paths_and_codes<T>(outcome: Result<T, DeserializeError<serde_json::Error>>) -> Vec<String> {
    match outcome {
        Ok(_) => Vec::new(),
        Err(DeserializeError::Violations(violations)) => violations.iter().map(path_and_code).collect(),
        Err(error) => panic!("expected violations, got the format's error: {error}"),
    }
}

fn path_and_code(violation: &Violation) -> String {
    let code = violation.problem().code();
    if violation.path().is_root() {
        return code.to_owned();
    }
    format!("{}: {code}", violation.path())
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct NewMember {
    #[validate(required, length(max = 50))]
    full_name: String,
    #[validate(email)]
    email: String,
    #[validate(range(min = 18, max = 120))]
    age: u8,
}

#[test]
fn a_body_breaking_no_rule_is_deserialized_when_failing_fast() {
    let body = r#"{ "fullName": "Alex Morgan", "email": "alex@example.com", "age": 30 }"#;
    let mut deserializer = serde_json::Deserializer::from_str(body);

    let member = annotated::from_deserializer_fail_fast::<NewMember, _>(&mut deserializer).unwrap();

    assert_eq!(
        member,
        NewMember {
            full_name: "Alex Morgan".to_owned(),
            email: "alex@example.com".to_owned(),
            age: 30,
        }
    );
}

#[test]
fn failing_fast_answers_only_the_first_violation_of_the_full_report() {
    let body = r#"{ "fullName": "  ", "age": 12 }"#;

    assert_eq!(
        violations_reading::<NewMember>(body),
        ["fullName: required", "email: missing", "age: range"]
    );
    assert_eq!(fail_fast_violations_reading::<NewMember>(body), ["fullName: required"]);
}

#[test]
fn failing_fast_answers_a_missing_field_declared_before_a_broken_rule() {
    let body = r#"{ "fullName": "Alex Morgan", "age": 12 }"#;

    assert_eq!(violations_reading::<NewMember>(body), ["email: missing", "age: range"]);
    assert_eq!(fail_fast_violations_reading::<NewMember>(body), ["email: missing"]);
}

#[test]
fn failing_fast_answers_a_broken_rule_declared_before_a_missing_field() {
    let body = r#"{ "fullName": "Alex Morgan", "email": "alex" }"#;

    assert_eq!(violations_reading::<NewMember>(body), ["email: email", "age: missing"]);
    assert_eq!(fail_fast_violations_reading::<NewMember>(body), ["email: email"]);
}

#[test]
fn failing_fast_answers_the_first_violation_in_declaration_order_whatever_order_fields_are_read_in() {
    let body = r#"{ "age": 12, "fullName": "" }"#;

    assert_eq!(fail_fast_violations_reading::<NewMember>(body), ["fullName: required"]);
}

#[test]
fn failing_fast_still_reads_the_input_to_its_end() {
    let body = r#"{ "fullName": "", "email": "alex@example.com", "age": 30 "#;
    let mut deserializer = serde_json::Deserializer::from_str(body);

    let error = annotated::from_deserializer_fail_fast::<NewMember, _>(&mut deserializer).unwrap_err();

    assert_eq!(error.to_string(), "EOF while parsing an object at line 1 column 57");
}

#[derive(Deserialize, Debug)]
#[validate(report_all)]
struct Handle {
    #[validate(length(min = 3), prefix = "@")]
    #[allow(dead_code)]
    handle: String,
}

#[test]
fn failing_fast_answers_only_the_first_rule_a_field_breaks_under_report_all() {
    let body = r#"{ "handle": "a" }"#;

    assert_eq!(violations_reading::<Handle>(body), ["handle: length", "handle: prefix"]);
    assert_eq!(fail_fast_violations_reading::<Handle>(body), ["handle: length"]);
}

fn passwords_match(new_password: &NewPassword) -> Result<(), Problem> {
    if new_password.repeated_password != new_password.password {
        return Err(Problem::custom("passwords_differ", "must match the password"));
    }
    Ok(())
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[validate(report_all, custom(passwords_match))]
struct NewPassword {
    #[validate(length(min = 8))]
    password: String,
    repeated_password: String,
}

#[test]
fn failing_fast_runs_no_struct_level_rule_after_the_first_violation() {
    let body = r#"{ "password": "horse", "repeatedPassword": "battery" }"#;

    assert_eq!(
        violations_reading::<NewPassword>(body),
        ["password: length", "passwords_differ"]
    );
    assert_eq!(fail_fast_violations_reading::<NewPassword>(body), ["password: length"]);
}

#[test]
fn failing_fast_answers_a_struct_level_rule_when_it_breaks_first() {
    let body = r#"{ "password": "correct horse", "repeatedPassword": "battery" }"#;

    assert_eq!(fail_fast_violations_reading::<NewPassword>(body), ["passwords_differ"]);
}

fn must_not_be_checked(_: &str) -> Result<(), Problem> {
    panic!("checked after failing fast found its violation")
}

fn signpost_must_not_be_checked(_: &Signpost) -> Result<(), Problem> {
    panic!("checked after failing fast found its violation")
}

#[derive(Deserialize, Debug)]
#[validate(custom(signpost_must_not_be_checked))]
struct Signpost {
    #[validate(required)]
    #[allow(dead_code)]
    destination: String,
    #[validate(custom(must_not_be_checked))]
    #[allow(dead_code)]
    note: String,
}

#[test]
fn failing_fast_checks_nothing_after_a_missing_field() {
    let body = r#"{ "note": "" }"#;

    assert_eq!(fail_fast_violations_reading::<Signpost>(body), ["destination: missing"]);
}

#[test]
fn failing_fast_checks_nothing_after_a_broken_rule() {
    let body = r#"{ "destination": "", "note": "" }"#;

    assert_eq!(
        fail_fast_violations_reading::<Signpost>(body),
        ["destination: required"]
    );
}
