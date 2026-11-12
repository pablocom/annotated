//! Scenario 6: a request body from JSON text to a validated value, on the flat sign-up bodies of scenarios 1 to 3.
//! Every library reads it with `serde_json::from_str`, then validates. annotated also reads it with its collecting
//! deserialization, `annotated::json::from_str`, which checks each field as it is read, in one pass. `serde_json`
//! alone, deserializing the same body into a struct without rules, is the floor every library builds on.
//!
//! On the valid body, annotated also reads a struct with `#[annotated(collect_type_errors)]`, to measure what the
//! opt-in costs a body that has no value of the wrong type. On the invalid ones, it also stops at the first
//! issue, reading with `annotated::json::from_str_fail_fast` and validating with `validate_fail_fast`: those
//! answer one issue where the other cases answer every one.

use serde::Deserialize;
use serde::de::DeserializeOwned;

use super::count;
use super::flat::{
    self, ONE_FIELD_INVALID_BODY, VALID_BODY, with_annotated, with_annotated_deserialize, with_garde, with_validator,
    with_wary,
};
use crate::{Case, Library};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(dead_code, reason = "deserialized for its cost, never read")]
pub struct PlainSignUp {
    username: String,
    full_name: Option<String>,
    email: String,
    website: Option<String>,
    age: u8,
    rating: f64,
    postal_code: String,
    bio: Option<String>,
}

pub(crate) type Received<Issues> = Result<Result<(), Issues>, serde_json::Error>;

/// Reads `body` with `serde_json::from_str`, then validates what it read.
pub(crate) fn received<T: DeserializeOwned, Issues>(
    body: &str,
    validate: impl Fn(&T) -> Result<(), Issues>,
) -> Received<Issues> {
    let value = serde_json::from_str::<T>(body)?;
    Ok(validate(&value))
}

/// The issues validation found, or `usize::MAX` when serde refused the body, so a case that never validated
/// can't pass for one that found nothing.
pub(crate) fn issues_received<Issues>(
    received: &Received<Issues>,
    count: impl Fn(&Result<(), Issues>) -> usize,
) -> usize {
    received.as_ref().map_or(usize::MAX, count)
}

#[must_use]
pub fn cases() -> Vec<Case> {
    let every_field_invalid_body = flat::every_field_invalid_body();
    [
        cases_of("end_to_end_valid", VALID_BODY, collecting_type_errors_cases),
        cases_of(
            "end_to_end_every_field_invalid",
            &every_field_invalid_body,
            fail_fast_cases,
        ),
        cases_of("end_to_end_one_field_invalid", ONE_FIELD_INVALID_BODY, fail_fast_cases),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Every library reading `body`, with the ways of annotated's that `cases_particular_to_the_body` adds after its
/// default ones.
fn cases_of(
    scenario: &'static str,
    body: &str,
    cases_particular_to_the_body: fn(&'static str, &str) -> Vec<Case>,
) -> Vec<Case> {
    [
        vec![serde_json_alone_case(scenario, body)],
        annotated_cases(scenario, body),
        cases_particular_to_the_body(scenario, body),
        other_libraries_cases(scenario, body),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn serde_json_alone_case(scenario: &'static str, body: &str) -> Case {
    Case::new(
        scenario,
        Library::SerdeJsonAlone,
        body.to_owned(),
        |body| serde_json::from_str::<PlainSignUp>(body).map(drop),
        |received| usize::from(received.is_err()),
    )
}

fn annotated_cases(scenario: &'static str, body: &str) -> Vec<Case> {
    use annotated::Validate as _;

    vec![
        Case::new(
            scenario,
            Library::Annotated,
            body.to_owned(),
            |body| annotated::json::from_str::<with_annotated_deserialize::SignUp>(body),
            count::annotated_deserialized,
        )
        .variant("json::from_str"),
        Case::new(
            scenario,
            Library::Annotated,
            body.to_owned(),
            |body| received(body, with_annotated::SignUp::validate),
            |received| issues_received(received, count::annotated),
        )
        .variant("serde_json, then validate"),
        Case::new(
            scenario,
            Library::Annotated,
            body.to_owned(),
            |body| annotated::json::from_str::<with_annotated_deserialize::SignUpReportingAll>(body),
            count::annotated_deserialized,
        )
        .variant("json::from_str, report_all"),
        Case::new(
            scenario,
            Library::Annotated,
            body.to_owned(),
            |body| received(body, with_annotated::SignUpReportingAll::validate),
            |received| issues_received(received, count::annotated),
        )
        .variant("serde_json, then validate, report_all"),
    ]
}

fn collecting_type_errors_cases(scenario: &'static str, body: &str) -> Vec<Case> {
    vec![
        Case::new(
            scenario,
            Library::Annotated,
            body.to_owned(),
            |body| annotated::json::from_str::<with_annotated_deserialize::SignUpCollectingTypeErrors>(body),
            count::annotated_deserialized,
        )
        .variant("json::from_str, collect_type_errors"),
    ]
}

fn fail_fast_cases(scenario: &'static str, body: &str) -> Vec<Case> {
    use annotated::Validate as _;

    vec![
        Case::new(
            scenario,
            Library::Annotated,
            body.to_owned(),
            |body| annotated::json::from_str_fail_fast::<with_annotated_deserialize::SignUp>(body),
            count::annotated_deserialized,
        )
        .variant("json::from_str_fail_fast"),
        Case::new(
            scenario,
            Library::Annotated,
            body.to_owned(),
            |body| received(body, with_annotated::SignUp::validate_fail_fast),
            |received| issues_received(received, count::annotated),
        )
        .variant("serde_json, then validate_fail_fast"),
    ]
}

fn other_libraries_cases(scenario: &'static str, body: &str) -> Vec<Case> {
    use garde::Validate as _;
    use validator::Validate as _;
    use wary::Validate as _;

    vec![
        Case::new(
            scenario,
            Library::Validator,
            body.to_owned(),
            |body| received(body, with_validator::SignUp::validate),
            |received| issues_received(received, count::validator),
        ),
        Case::new(
            scenario,
            Library::Garde,
            body.to_owned(),
            |body| received(body, |sign_up: &with_garde::SignUp| sign_up.validate()),
            |received| issues_received(received, count::garde),
        ),
        Case::new(
            scenario,
            Library::Wary,
            body.to_owned(),
            |body| received(body, |sign_up: &with_wary::SignUp| sign_up.validate(&())),
            |received| issues_received(received, count::wary),
        ),
    ]
}
