//! `required` on an `Option` field asks for its key: `#[derive(Deserialize)]` reports an absent key as `missing`,
//! whatever serde's defaults say, and reads `null` as `None`. Every other rule checks the value inside.

#![cfg(feature = "serde")]

use annotated::{Deserialize, DeserializeError, Issue, Path, Problem};
use serde::Deserializer;

fn deserialize<'de, T: Deserialize<'de>>(json: &'de str) -> Result<T, DeserializeError<serde_json::Error>> {
    annotated::from_deserializer(&mut serde_json::Deserializer::from_str(json))
}

fn issues_reading<'de, T: Deserialize<'de>>(json: &'de str) -> Vec<String> {
    described_outcome(deserialize::<T>(json))
}

fn fail_fast_issues_reading<'de, T: Deserialize<'de>>(json: &'de str) -> Vec<String> {
    described_outcome(annotated::from_deserializer_fail_fast::<T, _>(
        &mut serde_json::Deserializer::from_str(json),
    ))
}

fn described_outcome<T>(outcome: Result<T, DeserializeError<serde_json::Error>>) -> Vec<String> {
    match outcome {
        Ok(_) => Vec::new(),
        Err(DeserializeError::Issues(issues)) => issues.iter().map(path_and_code).collect(),
        Err(error) => vec![format!("the format's error: {error}")],
    }
}

fn path_and_code(issue: &Issue) -> String {
    let code = issue.problem().code();
    if issue.path().is_root() {
        return code.to_owned();
    }
    format!("{}: {code}", issue.path())
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Contact {
    #[validate(required)]
    phone_number: Option<String>,
}

#[test]
fn an_absent_key_of_a_required_option_is_missing_at_its_serde_name() {
    let error = deserialize::<Contact>("{}").unwrap_err();

    assert_eq!(
        error.issues(),
        [Issue::new(Path::root().field("phoneNumber"), Problem::Missing)]
    );
}

#[test]
fn null_in_a_required_option_is_none_without_an_issue() {
    let contact = deserialize::<Contact>(r#"{ "phoneNumber": null }"#).unwrap();

    assert_eq!(contact, Contact { phone_number: None });
}

#[test]
fn a_value_in_a_required_option_is_some() {
    let contact = deserialize::<Contact>(r#"{ "phoneNumber": "600 000 000" }"#).unwrap();

    assert_eq!(
        contact,
        Contact {
            phone_number: Some("600 000 000".to_owned())
        }
    );
}

#[test]
fn a_required_option_missing_at_the_end_of_a_sequence_is_missing() {
    assert_eq!(issues_reading::<Contact>("[]"), ["phoneNumber: missing"]);
}

#[test]
fn plain_serde_json_refuses_an_absent_key_of_a_required_option() {
    let error = serde_json::from_str::<Contact>("{}").unwrap_err();

    assert_eq!(error.to_string(), "phoneNumber: is missing");
}

#[test]
fn plain_serde_json_reads_null_in_a_required_option_as_none() {
    let contact = serde_json::from_str::<Contact>(r#"{ "phoneNumber": null }"#).unwrap();

    assert_eq!(contact, Contact { phone_number: None });
}

#[derive(Deserialize, Debug)]
struct Rsvp {
    #[validate(required(message = "say whether you are coming, or null when you don't know yet"))]
    #[allow(dead_code)]
    attending: Option<bool>,
}

#[test]
fn the_message_of_required_is_the_message_of_the_missing_key() {
    let error = deserialize::<Rsvp>("{}").unwrap_err();

    assert_eq!(
        error.issues(),
        [Issue::new(Path::root().field("attending"), Problem::Missing)
            .with_message("say whether you are coming, or null when you don't know yet")]
    );
}

#[derive(Deserialize, Debug)]
struct Preferences {
    #[serde(default)]
    #[validate(required)]
    #[allow(dead_code)]
    theme: Option<String>,
}

#[test]
fn a_required_option_with_a_default_of_its_own_is_missing_when_its_key_is_absent() {
    assert_eq!(issues_reading::<Preferences>("{}"), ["theme: missing"]);
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
struct Settings {
    #[validate(required)]
    #[allow(dead_code)]
    theme: Option<String>,
    #[allow(dead_code)]
    language: String,
}

#[test]
fn a_required_option_in_a_struct_with_a_default_is_missing_when_its_key_is_absent() {
    assert_eq!(issues_reading::<Settings>("{}"), ["theme: missing"]);
}

#[derive(Deserialize, Debug, PartialEq)]
struct Profile {
    #[validate(required, length(min = 1))]
    nickname: Option<String>,
}

#[test]
fn an_empty_string_in_a_required_option_passes_required_and_breaks_the_rule_beside_it() {
    assert_eq!(issues_reading::<Profile>(r#"{ "nickname": "" }"#), ["nickname: length"]);
}

#[test]
fn null_in_a_required_option_skips_the_rules_beside_required() {
    let profile = deserialize::<Profile>(r#"{ "nickname": null }"#).unwrap();

    assert_eq!(profile, Profile { nickname: None });
}

#[derive(Deserialize, Debug)]
struct SignUp {
    #[validate(required)]
    #[allow(dead_code)]
    phone: Option<String>,
    #[validate(email)]
    #[allow(dead_code)]
    email: String,
}

#[test]
fn failing_fast_answers_the_absent_key_of_a_required_option_when_it_comes_first() {
    let body = r#"{ "email": "alex" }"#;

    assert_eq!(fail_fast_issues_reading::<SignUp>(body), ["phone: missing"]);
}

#[derive(Deserialize, Debug, PartialEq)]
#[annotated(collect_type_errors)]
struct ContactCollectingTypeErrors {
    #[validate(required)]
    phone: Option<String>,
}

#[test]
fn collecting_type_errors_an_absent_key_of_a_required_option_is_missing() {
    assert_eq!(issues_reading::<ContactCollectingTypeErrors>("{}"), ["phone: missing"]);
}

#[test]
fn collecting_type_errors_null_in_a_required_option_is_none_without_an_issue() {
    let contact = deserialize::<ContactCollectingTypeErrors>(r#"{ "phone": null }"#).unwrap();

    assert_eq!(contact, ContactCollectingTypeErrors { phone: None });
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
enum Delivery {
    Courier {
        #[validate(required)]
        tracking: Option<String>,
    },
}

#[test]
fn an_absent_key_of_a_required_option_in_a_struct_variant_is_missing_under_the_variant_s_name() {
    assert_eq!(
        issues_reading::<Delivery>(r#"{ "courier": {} }"#),
        ["courier.tracking: missing"]
    );
}

#[test]
fn null_in_a_required_option_of_a_struct_variant_is_none_without_an_issue() {
    let delivery = deserialize::<Delivery>(r#"{ "courier": { "tracking": null } }"#).unwrap();

    assert_eq!(delivery, Delivery::Courier { tracking: None });
}

#[derive(Deserialize, Debug)]
#[serde(tag = "method", rename_all = "camelCase")]
enum InternallyTaggedDelivery {
    Courier {
        #[validate(required)]
        #[allow(dead_code)]
        tracking: Option<String>,
    },
}

#[test]
fn an_absent_key_of_a_required_option_in_an_internally_tagged_variant_is_missing_beside_the_tag() {
    assert_eq!(
        issues_reading::<InternallyTaggedDelivery>(r#"{ "method": "courier" }"#),
        ["tracking: missing"]
    );
}

#[derive(Deserialize, Debug)]
#[serde(tag = "method", content = "details", rename_all = "camelCase")]
enum AdjacentlyTaggedDelivery {
    Courier {
        #[validate(required)]
        #[allow(dead_code)]
        tracking: Option<String>,
    },
}

#[test]
fn an_absent_key_of_a_required_option_in_an_adjacently_tagged_variant_is_missing_under_the_content() {
    assert_eq!(
        issues_reading::<AdjacentlyTaggedDelivery>(r#"{ "method": "courier", "details": {} }"#),
        ["details.tracking: missing"]
    );
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(untagged)]
enum UntaggedDelivery {
    Courier {
        #[validate(required)]
        tracking: Option<String>,
        address: String,
    },
    Pickup {
        address: String,
    },
}

#[test]
fn a_variant_whose_required_option_has_no_key_does_not_fit_an_untagged_enum() {
    let delivery = deserialize::<UntaggedDelivery>(r#"{ "address": "Gran Vía 1" }"#).unwrap();

    assert_eq!(
        delivery,
        UntaggedDelivery::Pickup {
            address: "Gran Vía 1".to_owned()
        }
    );
}

#[derive(Deserialize, Debug, PartialEq)]
enum Shape {
    Segment(u32, #[validate(required)] Option<u32>),
}

#[test]
fn a_required_option_in_a_tuple_variant_is_none_when_the_sequence_ends_before_it() {
    let shape = deserialize::<Shape>(r#"{ "Segment": [1] }"#).unwrap();

    assert_eq!(shape, Shape::Segment(1, None));
}

#[derive(Deserialize, Debug, PartialEq)]
struct Session {
    #[serde(skip)]
    #[validate(required)]
    cached_name: Option<String>,
    user: String,
}

#[test]
fn required_on_a_skipped_option_checks_nothing_since_the_input_never_has_it() {
    let session = deserialize::<Session>(r#"{ "user": "alex" }"#).unwrap();

    assert_eq!(
        session,
        Session {
            cached_name: None,
            user: "alex".to_owned()
        }
    );
}

#[derive(Deserialize, Debug, PartialEq)]
struct Address {
    street: String,
}

#[derive(Deserialize, Debug, PartialEq)]
struct Parcel {
    #[serde(flatten)]
    #[validate(required)]
    destination: Option<Address>,
}

#[test]
fn required_on_a_flattened_option_checks_nothing_since_it_has_no_key_of_its_own() {
    let parcel = deserialize::<Parcel>("{}").unwrap();

    assert_eq!(parcel, Parcel { destination: None });
}

fn trimmed<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    let text = <Option<String> as serde::Deserialize>::deserialize(deserializer)?;
    Ok(text.map(|text| text.trim().to_owned()))
}

#[derive(Deserialize, Debug, PartialEq)]
struct Comment {
    #[serde(default, deserialize_with = "trimmed")]
    #[validate(required)]
    signature: Option<String>,
}

#[test]
fn a_required_option_read_by_a_function_with_a_default_is_missing_when_its_key_is_absent() {
    assert_eq!(issues_reading::<Comment>("{}"), ["signature: missing"]);
}

#[test]
fn null_in_a_required_option_read_by_a_function_is_what_the_function_returns() {
    let comment = deserialize::<Comment>(r#"{ "signature": null }"#).unwrap();

    assert_eq!(comment, Comment { signature: None });
}

macro_rules! with_a_field_of_type {
    ($name:ident, $field_type:ty) => {
        #[derive(Deserialize, Debug, PartialEq)]
        struct $name {
            nickname: $field_type,
        }
    };
}

with_a_field_of_type!(Alias, Option<String>);

#[test]
fn an_option_passed_to_a_macro_as_a_type_is_none_when_its_key_is_absent() {
    let alias = deserialize::<Alias>("{}").unwrap();

    assert_eq!(alias, Alias { nickname: None });
}
