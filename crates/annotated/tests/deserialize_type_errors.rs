//! `#[annotated(collect_type_errors)]` reads each field of a struct in two steps, so a value of the wrong type is one
//! more violation, `invalid_type`, beside the missing fields and broken rules, instead of the format's error.

#![cfg(feature = "serde")]

use std::collections::BTreeMap;

use annotated::{Deserialize, DeserializeError, Problem, Violation};
use serde_test::{Readable, Token, assert_de_tokens_error};

fn deserialize<'de, T: Deserialize<'de>>(json: &'de str) -> Result<T, DeserializeError<serde_json::Error>> {
    annotated::from_deserializer(&mut serde_json::Deserializer::from_str(json))
}

fn violations_found_reading<'de, T: Deserialize<'de>>(json: &'de str) -> Vec<Violation> {
    match deserialize::<T>(json) {
        Ok(_) => Vec::new(),
        Err(DeserializeError::Violations(violations)) => violations.into_iter().collect(),
        Err(error) => panic!("expected violations, got the format's error: {error}"),
    }
}

fn violations_reading<'de, T: Deserialize<'de>>(json: &'de str) -> Vec<String> {
    described_outcome(deserialize::<T>(json))
}

fn fail_fast_violations_reading<'de, T: Deserialize<'de>>(json: &'de str) -> Vec<String> {
    described_outcome(annotated::from_deserializer_fail_fast::<T, _>(
        &mut serde_json::Deserializer::from_str(json),
    ))
}

fn described_outcome<T>(outcome: Result<T, DeserializeError<serde_json::Error>>) -> Vec<String> {
    match outcome {
        Ok(_) => Vec::new(),
        Err(DeserializeError::Violations(violations)) => violations.iter().map(described).collect(),
        Err(error) => vec![format!("the format's error: {error}")],
    }
}

fn described(violation: &Violation) -> String {
    let params = violation.problem().params();
    let params = params
        .iter()
        .map(|(name, value)| format!("{name}: {value}"))
        .collect::<Vec<_>>();
    let code_and_params = if params.is_empty() {
        violation.problem().code().to_owned()
    } else {
        format!("{}({})", violation.problem().code(), params.join(", "))
    };
    if violation.path().is_root() {
        return code_and_params;
    }
    format!("{}: {code_and_params}", violation.path())
}

fn format_error_reading<'de, T: Deserialize<'de>>(json: &'de str) -> String {
    match deserialize::<T>(json) {
        Err(error @ DeserializeError::Format(_)) => error.to_string(),
        Ok(_) => panic!("expected the format's error, got a value"),
        Err(error) => panic!("expected the format's error, got {error:?}"),
    }
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[annotated(collect_type_errors)]
struct NewMember {
    #[validate(required, length(max = 50))]
    full_name: String,
    #[validate(email)]
    email: String,
    #[validate(range(min = 18, max = 120))]
    age: u8,
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct NewMemberStoppingAtATypeError {
    #[validate(required, length(max = 50))]
    full_name: String,
    #[validate(email)]
    email: String,
    #[validate(range(min = 18, max = 120))]
    age: u8,
}

#[test]
fn a_value_of_the_wrong_type_is_reported_beside_the_other_violations_in_declaration_order() {
    let body = r#"{ "age": "twelve", "email": "x" }"#;

    assert_eq!(
        violations_reading::<NewMember>(body),
        [
            "fullName: missing",
            "email: email",
            "age: invalid_type(expected: u8, found: string)"
        ]
    );
}

#[test]
fn without_collect_type_errors_a_value_of_the_wrong_type_is_the_format_s_error() {
    let body = r#"{ "age": "twelve", "email": "x" }"#;

    assert_eq!(
        format_error_reading::<NewMemberStoppingAtATypeError>(body),
        r#"invalid type: string "twelve", expected u8 at line 1 column 17"#
    );
}

#[test]
fn a_new_member_of_the_right_types_breaking_no_rule_is_deserialized() {
    let body = r#"{ "fullName": "Lucía Ramos", "email": "lucia@example.com", "age": 30 }"#;

    assert_eq!(
        deserialize::<NewMember>(body).unwrap(),
        NewMember {
            full_name: "Lucía Ramos".to_owned(),
            email: "lucia@example.com".to_owned(),
            age: 30,
        }
    );
}

#[derive(Deserialize, Debug)]
#[annotated(collect_type_errors)]
struct Age {
    #[validate(range(min = 18))]
    #[allow(dead_code)]
    age: u8,
}

#[test]
fn a_value_of_the_wrong_type_reads_as_what_it_must_be_and_what_was_found() {
    let error = deserialize::<Age>(r#"{ "age": "twelve" }"#).unwrap_err();

    assert_eq!(error.to_string(), "age: must be u8, found string");
}

#[derive(Deserialize, Debug)]
#[annotated(collect_type_errors)]
struct Preferences {
    #[allow(dead_code)]
    age: Option<u8>,
    #[allow(dead_code)]
    name: Option<String>,
}

#[test]
fn a_boolean_where_a_number_is_expected_is_found_as_a_boolean() {
    assert_eq!(
        violations_reading::<Preferences>(r#"{ "age": true }"#),
        ["age: invalid_type(expected: u8, found: boolean)"]
    );
}

#[test]
fn an_integer_where_a_string_is_expected_is_found_as_an_integer() {
    assert_eq!(
        violations_reading::<Preferences>(r#"{ "name": 5 }"#),
        ["name: invalid_type(expected: a string, found: integer)"]
    );
}

#[test]
fn a_float_where_an_integer_is_expected_is_found_as_a_float() {
    assert_eq!(
        violations_reading::<Preferences>(r#"{ "age": 1.5 }"#),
        ["age: invalid_type(expected: u8, found: float)"]
    );
}

#[test]
fn a_string_where_an_optional_number_is_expected_is_found_as_a_string() {
    assert_eq!(
        violations_reading::<Preferences>(r#"{ "age": "x" }"#),
        ["age: invalid_type(expected: u8, found: string)"]
    );
}

#[test]
fn a_sequence_where_a_number_is_expected_is_found_as_a_sequence() {
    assert_eq!(
        violations_reading::<Preferences>(r#"{ "age": [30] }"#),
        ["age: invalid_type(expected: u8, found: sequence)"]
    );
}

#[test]
fn a_map_where_a_number_is_expected_is_found_as_a_map() {
    assert_eq!(
        violations_reading::<Preferences>(r#"{ "age": { "years": 30 } }"#),
        ["age: invalid_type(expected: u8, found: map)"]
    );
}

#[test]
fn null_where_a_value_is_required_is_found_as_null() {
    assert_eq!(
        violations_reading::<Age>(r#"{ "age": null }"#),
        ["age: invalid_type(expected: u8, found: null)"]
    );
}

#[test]
fn null_where_an_optional_value_is_expected_is_none() {
    assert_eq!(
        violations_reading::<Preferences>(r#"{ "age": null }"#),
        Vec::<String>::new()
    );
}

#[test]
fn a_character_where_a_number_is_expected_is_found_as_a_character() {
    assert_de_tokens_error::<Readable<Preferences>>(
        &[
            Token::Struct {
                name: "Preferences",
                len: 1,
            },
            Token::Str("age"),
            Token::Char('a'),
            Token::StructEnd,
        ],
        "age: must be u8, found character",
    );
}

#[test]
fn bytes_where_a_number_is_expected_are_found_as_bytes() {
    assert_de_tokens_error::<Readable<Preferences>>(
        &[
            Token::Struct {
                name: "Preferences",
                len: 1,
            },
            Token::Str("age"),
            Token::Bytes(b"30"),
            Token::StructEnd,
        ],
        "age: must be u8, found bytes",
    );
}

#[test]
fn an_integer_out_of_the_range_of_its_type_is_of_the_wrong_type() {
    assert_eq!(
        violations_reading::<Age>(r#"{ "age": 300 }"#),
        ["age: invalid_type(expected: u8, found: integer)"]
    );
}

#[derive(Deserialize, Debug)]
#[annotated(collect_type_errors)]
struct Scores {
    #[validate(length(max = 3))]
    #[allow(dead_code)]
    scores: Vec<u8>,
}

#[test]
fn an_element_of_the_wrong_type_is_reported_at_its_collection_by_what_the_element_must_be() {
    assert_eq!(
        violations_reading::<Scores>(r#"{ "scores": [1, "x"] }"#),
        ["scores: invalid_type(expected: u8, found: string)"]
    );
}

#[derive(Debug, PartialEq)]
struct Handle(String);

impl<'de> serde::Deserialize<'de> for Handle {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let handle = String::deserialize(deserializer)?;
        if !handle.starts_with('@') {
            return Err(serde::de::Error::custom(format!("{handle} doesn't start with @")));
        }
        Ok(Self(handle))
    }
}

#[derive(Deserialize, Debug)]
#[annotated(collect_type_errors)]
struct Profile {
    #[allow(dead_code)]
    handle: Handle,
    #[allow(dead_code)]
    previous_handle: Option<Handle>,
}

#[test]
fn a_value_its_type_refuses_with_a_message_of_its_own_must_be_that_type() {
    assert_eq!(
        violations_reading::<Profile>(r#"{ "handle": "lucia" }"#),
        ["handle: invalid_type(expected: Handle, found: string)"]
    );
}

#[test]
fn an_optional_value_its_type_refuses_with_a_message_of_its_own_must_be_the_type_inside_the_option() {
    assert_eq!(
        violations_reading::<Profile>(r#"{ "handle": "@lucia", "previous_handle": "lucia" }"#),
        ["previous_handle: invalid_type(expected: Handle, found: string)"]
    );
}

#[derive(Debug, PartialEq)]
struct Password(String);

impl<'de> serde::Deserialize<'de> for Password {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let password = String::deserialize(deserializer)?;
        if password.len() < 8 {
            return Err(serde::de::Error::custom(format!("{password:?} is too short")));
        }
        Ok(Self(password))
    }
}

#[derive(Deserialize, Debug)]
#[annotated(collect_type_errors)]
struct LogIn {
    #[allow(dead_code)]
    email: String,
    #[allow(dead_code)]
    password: Password,
}

#[test]
fn a_value_of_the_wrong_type_is_never_repeated_back() {
    let body = r#"{ "email": "lucia@example.com", "password": "hunter2" }"#;

    let error = deserialize::<LogIn>(body).unwrap_err();

    let everything_said = format!("{error}\n{error:?}");
    assert!(!everything_said.contains("hunter2"), "{everything_said}");
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[annotated(collect_type_errors)]
struct Address {
    #[validate(required)]
    street: String,
    #[validate(length(min = 5))]
    postal_code: String,
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct AddressStoppingAtATypeError {
    #[validate(required)]
    street: String,
    #[validate(length(min = 5))]
    postal_code: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[annotated(collect_type_errors)]
struct Parcel {
    #[validate(required)]
    #[allow(dead_code)]
    reference: String,
    #[validate(nested)]
    #[allow(dead_code)]
    sender: Address,
    #[validate(nested)]
    #[allow(dead_code)]
    recipient: AddressStoppingAtATypeError,
    #[validate(range(min = 1))]
    #[allow(dead_code)]
    weight_in_grams: u32,
}

#[test]
fn a_nested_struct_given_a_number_is_of_the_wrong_type_at_its_field() {
    let body = r#"{ "reference": "", "sender": 5, "recipient": { "street": "Atocha 2", "postalCode": "28012" },
        "weightInGrams": 0 }"#;

    assert_eq!(
        violations_reading::<Parcel>(body),
        [
            "reference: required",
            "sender: invalid_type(expected: struct Address, found: integer)",
            "weightInGrams: range(min: 1)"
        ]
    );
}

#[test]
fn a_field_missing_inside_a_nested_struct_is_reported_at_its_full_path() {
    let body = r#"{ "reference": "PX-1", "sender": { "postalCode": "28013" },
        "recipient": { "street": "Atocha 2", "postalCode": "28012" }, "weightInGrams": 900 }"#;

    assert_eq!(violations_reading::<Parcel>(body), ["sender.street: missing"]);
}

#[test]
fn a_value_of_the_wrong_type_inside_a_nested_struct_collecting_them_is_reported_at_its_full_path() {
    let body = r#"{ "reference": "PX-1", "sender": { "street": 5, "postalCode": "1" },
        "recipient": { "street": "Atocha 2", "postalCode": "28012" }, "weightInGrams": 900 }"#;

    assert_eq!(
        violations_reading::<Parcel>(body),
        [
            "sender.street: invalid_type(expected: a string, found: integer)",
            "sender.postalCode: length(min: 5, unit: chars)"
        ]
    );
}

#[test]
fn a_value_of_the_wrong_type_inside_a_nested_struct_not_collecting_them_is_reported_at_the_nested_field() {
    let body = r#"{ "reference": "PX-1", "sender": { "street": "Gran Vía 1", "postalCode": "28013" },
        "recipient": { "street": 5, "postalCode": "1" }, "weightInGrams": 900 }"#;

    assert_eq!(
        violations_reading::<Parcel>(body),
        ["recipient: invalid_type(expected: a string, found: integer)"]
    );
}

#[derive(Deserialize, Debug)]
#[annotated(collect_type_errors)]
struct Route {
    #[validate(each(nested))]
    #[allow(dead_code)]
    stops: Vec<Address>,
    #[validate(each(nested))]
    #[allow(dead_code)]
    depots: BTreeMap<String, Address>,
}

#[test]
fn an_element_of_the_wrong_type_replaces_the_violations_found_in_its_collection_before_it() {
    let body = r#"{ "stops": [{ "street": "", "postalCode": "28013" }, 5], "depots": {} }"#;

    assert_eq!(
        violations_reading::<Route>(body),
        ["stops: invalid_type(expected: struct Address, found: integer)"]
    );
}

#[test]
fn a_value_of_the_wrong_type_inside_an_element_collecting_them_is_reported_at_its_full_path() {
    let body = r#"{ "stops": [], "depots": { "north": { "street": "Alcalá 1", "postalCode": 28014 } } }"#;

    assert_eq!(
        violations_reading::<Route>(body),
        ["depots[north].postalCode: invalid_type(expected: a string, found: integer)"]
    );
}

fn must_not_be_checked(_: &str) -> Result<(), Problem> {
    panic!("a value of the wrong type was checked by its other rules")
}

fn must_not_be_checked_as_a_whole(_: &Stay) -> Result<(), Problem> {
    panic!("a struct with a value of the wrong type was checked by its struct-level rules")
}

#[derive(Deserialize, Debug)]
#[annotated(collect_type_errors)]
#[validate(custom(must_not_be_checked_as_a_whole))]
struct Stay {
    #[validate(custom(must_not_be_checked))]
    #[allow(dead_code)]
    guest: String,
    #[validate(range(min = 1))]
    #[allow(dead_code)]
    nights: u32,
}

#[test]
fn a_value_of_the_wrong_type_skips_its_other_rules_and_the_struct_level_rules() {
    assert_eq!(
        violations_reading::<Stay>(r#"{ "guest": 5, "nights": 0 }"#),
        [
            "guest: invalid_type(expected: a string, found: integer)",
            "nights: range(min: 1)"
        ]
    );
}

#[test]
fn a_struct_written_as_a_sequence_collects_its_values_of_the_wrong_type() {
    assert_eq!(
        violations_reading::<NewMember>(r#"["", "alex", "twelve"]"#),
        [
            "fullName: required",
            "email: email",
            "age: invalid_type(expected: u8, found: string)"
        ]
    );
}

#[test]
fn a_value_that_is_not_well_formed_is_still_the_format_s_error() {
    assert_eq!(
        format_error_reading::<Age>(r#"{ "age": tru }"#),
        "expected ident at line 1 column 13"
    );
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
#[annotated(collect_type_errors)]
struct Invitation {
    #[allow(dead_code)]
    email: String,
}

#[test]
fn an_unknown_field_is_still_the_format_s_error_when_the_struct_denies_unknown_fields() {
    assert_eq!(
        format_error_reading::<Invitation>(r#"{ "email": 5, "role": "admin" }"#),
        "unknown field `role`, expected `email` at line 1 column 20"
    );
}

#[derive(Deserialize, Debug)]
#[annotated(collect_type_errors)]
struct Page<T> {
    #[validate(length(max = 2))]
    #[allow(dead_code)]
    items: Vec<T>,
}

#[test]
fn a_struct_with_a_type_parameter_collects_its_values_of_the_wrong_type() {
    assert_eq!(
        violations_reading::<Page<u8>>(r#"{ "items": ["x"] }"#),
        ["items: invalid_type(expected: u8, found: string)"]
    );
}

#[test]
fn plain_serde_json_refuses_a_value_of_the_wrong_type_with_every_violation_in_its_message() {
    let error = serde_json::from_str::<NewMember>(r#"{ "fullName": "", "email": "x", "age": "twelve" }"#).unwrap_err();

    assert_eq!(
        error.to_string(),
        "fullName: is required\nemail: must be an email address\nage: must be u8, found string"
    );
}

#[test]
fn failing_fast_answers_a_value_of_the_wrong_type_declared_first() {
    let body = r#"{ "fullName": 5, "email": "x", "age": 12 }"#;

    assert_eq!(
        violations_reading::<NewMember>(body),
        [
            "fullName: invalid_type(expected: a string, found: integer)",
            "email: email",
            "age: range(min: 18, max: 120)"
        ]
    );
    assert_eq!(
        fail_fast_violations_reading::<NewMember>(body),
        ["fullName: invalid_type(expected: a string, found: integer)"]
    );
}

#[test]
fn failing_fast_answers_a_broken_rule_declared_before_a_value_of_the_wrong_type() {
    let body = r#"{ "fullName": "", "email": "x", "age": "twelve" }"#;

    assert_eq!(
        violations_reading::<NewMember>(body),
        [
            "fullName: required",
            "email: email",
            "age: invalid_type(expected: u8, found: string)"
        ]
    );
    assert_eq!(fail_fast_violations_reading::<NewMember>(body), ["fullName: required"]);
}

#[test]
fn failing_fast_answers_a_value_of_the_wrong_type_inside_a_nested_struct() {
    let body = r#"{ "reference": "", "sender": { "street": 5, "postalCode": "1" },
        "recipient": { "street": "Atocha 2", "postalCode": "28012" }, "weightInGrams": 0 }"#;

    assert_eq!(
        violations_reading::<Parcel>(body),
        [
            "reference: required",
            "sender.street: invalid_type(expected: a string, found: integer)",
            "sender.postalCode: length(min: 5, unit: chars)",
            "weightInGrams: range(min: 1)"
        ]
    );
    assert_eq!(fail_fast_violations_reading::<Parcel>(body), ["reference: required"]);
}

#[test]
fn failing_fast_answers_the_first_violation_inside_a_nested_struct_when_it_is_of_the_wrong_type() {
    let body = r#"{ "reference": "PX-1", "sender": { "street": 5, "postalCode": "1" },
        "recipient": { "street": "Atocha 2", "postalCode": "28012" }, "weightInGrams": 0 }"#;

    assert_eq!(
        fail_fast_violations_reading::<Parcel>(body),
        ["sender.street: invalid_type(expected: a string, found: integer)"]
    );
}

#[test]
fn failing_fast_answers_a_nested_struct_of_the_wrong_type() {
    let body = r#"{ "reference": "PX-1", "sender": 5, "recipient": 5, "weightInGrams": 0 }"#;

    assert_eq!(
        fail_fast_violations_reading::<Parcel>(body),
        ["sender: invalid_type(expected: struct Address, found: integer)"]
    );
}

fn no_tag_is_the_name(full_name: &str, tags: &[String]) -> Result<(), Problem> {
    if tags.iter().any(|tag| tag == full_name) {
        return Err(Problem::custom("tag_is_the_name", "can't repeat the name"));
    }
    Ok(())
}

fn tags_differ_from_the_name(sign_up: &SignUp) -> Result<(), Problem> {
    no_tag_is_the_name(&sign_up.full_name, &sign_up.tags)
}

fn tags_differ_from_the_name_stopping_at_a_type_error(sign_up: &SignUpStoppingAtATypeError) -> Result<(), Problem> {
    no_tag_is_the_name(&sign_up.full_name, &sign_up.tags)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[annotated(collect_type_errors)]
#[validate(custom(tags_differ_from_the_name, field = tags))]
struct SignUp {
    #[validate(required, length(max = 10))]
    full_name: String,
    #[serde(rename = "mail", alias = "email")]
    #[validate(email)]
    #[allow(dead_code)]
    email: String,
    #[validate(length(min = 2))]
    #[allow(dead_code)]
    nickname: Option<String>,
    #[validate(required)]
    #[allow(dead_code)]
    phone: Option<String>,
    #[validate(range(min = 18))]
    #[allow(dead_code)]
    age: u8,
    #[validate(length(max = 2), each(length(min = 2)))]
    tags: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[validate(custom(tags_differ_from_the_name_stopping_at_a_type_error, field = tags))]
struct SignUpStoppingAtATypeError {
    #[validate(required, length(max = 10))]
    full_name: String,
    #[serde(rename = "mail", alias = "email")]
    #[validate(email)]
    #[allow(dead_code)]
    email: String,
    #[validate(length(min = 2))]
    #[allow(dead_code)]
    nickname: Option<String>,
    #[validate(required)]
    #[allow(dead_code)]
    phone: Option<String>,
    #[validate(range(min = 18))]
    #[allow(dead_code)]
    age: u8,
    #[validate(length(max = 2), each(length(min = 2)))]
    tags: Vec<String>,
}

#[test]
fn a_well_typed_body_breaking_every_rule_is_reported_as_without_collect_type_errors() {
    let body = r#"{ "fullName": "Alexandra Morgan", "email": "alex", "nickname": "A", "phone": null, "age": 12,
        "tags": ["a", "b", "Alexandra Morgan"] }"#;

    assert_eq!(
        violations_found_reading::<SignUp>(body),
        violations_found_reading::<SignUpStoppingAtATypeError>(body)
    );
}

#[test]
fn a_well_typed_body_missing_fields_is_reported_as_without_collect_type_errors() {
    let body = r#"{ "nickname": "A" }"#;

    assert_eq!(
        violations_found_reading::<SignUp>(body),
        violations_found_reading::<SignUpStoppingAtATypeError>(body)
    );
}

#[test]
fn a_well_typed_body_breaking_a_struct_level_rule_is_reported_as_without_collect_type_errors() {
    let body = r#"{ "fullName": "Alex", "mail": "alex@example.com", "phone": "600 000 000", "age": 30,
        "tags": ["Alex"] }"#;

    assert_eq!(
        violations_found_reading::<SignUp>(body),
        violations_found_reading::<SignUpStoppingAtATypeError>(body)
    );
}

#[test]
fn a_well_typed_body_breaking_no_rule_is_valid_as_without_collect_type_errors() {
    let body = r#"{ "fullName": "Alex", "mail": "alex@example.com", "phone": "600 000 000", "age": 30, "tags": [] }"#;

    assert_eq!(
        violations_found_reading::<SignUp>(body),
        violations_found_reading::<SignUpStoppingAtATypeError>(body)
    );
}

#[derive(serde::Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ShelfId(u32);

#[derive(Deserialize, Debug)]
#[annotated(collect_type_errors)]
struct Shelves {
    #[validate(length(max = 2))]
    #[allow(dead_code)]
    stock_by_shelf: BTreeMap<u32, String>,
    #[allow(dead_code)]
    labels: BTreeMap<ShelfId, bool>,
}

#[derive(Deserialize, Debug)]
struct ShelvesStoppingAtATypeError {
    #[validate(length(max = 2))]
    #[allow(dead_code)]
    stock_by_shelf: BTreeMap<u32, String>,
    #[allow(dead_code)]
    labels: BTreeMap<ShelfId, bool>,
}

#[test]
fn a_well_typed_map_keyed_by_numbers_written_as_strings_is_read_as_without_collect_type_errors() {
    let body = r#"{ "stock_by_shelf": { "1": "rice", "2": "beans", "3": "lentils" }, "labels": { "1": true } }"#;

    assert_eq!(
        violations_reading::<Shelves>(body),
        violations_reading::<ShelvesStoppingAtATypeError>(body)
    );
}

fn on_the_map(street: &str) -> Result<(), Problem> {
    if street.starts_with("Calle Falsa") {
        return Err(Problem::custom("not_on_the_map", "isn't on the map"));
    }
    Ok(())
}

fn not_a_restricted_stop(address: &Address) -> Result<(), Problem> {
    on_the_map(&address.street)
}

fn not_a_restricted_stop_stopping_at_a_type_error(address: &AddressStoppingAtATypeError) -> Result<(), Problem> {
    on_the_map(&address.street)
}

fn not_the_sender(consignment: &Consignment) -> Result<(), Problem> {
    differs_from_the_sender(&consignment.sender, &consignment.recipient)
}

fn not_the_sender_stopping_at_a_type_error(consignment: &ConsignmentStoppingAtATypeError) -> Result<(), Problem> {
    differs_from_the_sender(&consignment.sender, &consignment.recipient)
}

fn differs_from_the_sender<T: PartialEq>(sender: &T, recipient: &T) -> Result<(), Problem> {
    if sender == recipient {
        return Err(Problem::custom("same_as_sender", "can't be the sender"));
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[annotated(collect_type_errors)]
#[validate(custom(not_the_sender, field = recipient))]
struct Consignment {
    #[validate(nested)]
    sender: Address,
    #[validate(nested)]
    recipient: Address,
    #[validate(required, nested)]
    #[allow(dead_code)]
    return_address: Option<Address>,
    #[validate(length(max = 3), each(custom(not_a_restricted_stop), nested))]
    #[allow(dead_code)]
    stops: Vec<Address>,
    #[validate(each(required, nested))]
    #[allow(dead_code)]
    pickups: Vec<Option<Address>>,
    #[validate(each(nested, custom(not_a_restricted_stop)))]
    #[allow(dead_code)]
    depots: BTreeMap<String, Address>,
    #[validate(each(each(nested)))]
    #[allow(dead_code)]
    rounds: BTreeMap<String, Vec<Address>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[validate(custom(not_the_sender_stopping_at_a_type_error, field = recipient))]
struct ConsignmentStoppingAtATypeError {
    #[validate(nested)]
    sender: AddressStoppingAtATypeError,
    #[validate(nested)]
    recipient: AddressStoppingAtATypeError,
    #[validate(required, nested)]
    #[allow(dead_code)]
    return_address: Option<AddressStoppingAtATypeError>,
    #[validate(
        length(max = 3),
        each(custom(not_a_restricted_stop_stopping_at_a_type_error), nested)
    )]
    #[allow(dead_code)]
    stops: Vec<AddressStoppingAtATypeError>,
    #[validate(each(required, nested))]
    #[allow(dead_code)]
    pickups: Vec<Option<AddressStoppingAtATypeError>>,
    #[validate(each(nested, custom(not_a_restricted_stop_stopping_at_a_type_error)))]
    #[allow(dead_code)]
    depots: BTreeMap<String, AddressStoppingAtATypeError>,
    #[validate(each(each(nested)))]
    #[allow(dead_code)]
    rounds: BTreeMap<String, Vec<AddressStoppingAtATypeError>>,
}

#[test]
fn a_well_typed_nested_body_breaking_every_rule_is_reported_as_without_collect_type_errors() {
    let body = r#"{
        "sender": { "street": "", "postalCode": "280" },
        "recipient": { "street": "Gran Vía 1", "postalCode": "1" },
        "returnAddress": null,
        "stops": [
            { "street": "Calle Falsa 123", "postalCode": "1" },
            { "street": "", "postalCode": "28013" },
            { "street": "Calle Falsa 124", "postalCode": "28013" }
        ],
        "pickups": [{ "street": "", "postalCode": "28013" }, null, { "street": "Atocha 2", "postalCode": "2" }],
        "depots": {
            "south": { "street": "", "postalCode": "28013" },
            "north": { "street": "Calle Falsa 1", "postalCode": "28013" },
            "east": { "street": "Alcalá 1", "postalCode": "1" }
        },
        "rounds": {
            "tuesday": [{ "street": "", "postalCode": "1" }],
            "monday": [{ "street": "Sol 1", "postalCode": "28013" }, { "street": "", "postalCode": "28013" }]
        }
    }"#;

    assert_eq!(
        violations_found_reading::<Consignment>(body),
        violations_found_reading::<ConsignmentStoppingAtATypeError>(body)
    );
}

#[test]
fn a_well_typed_nested_body_missing_fields_inside_is_reported_as_without_collect_type_errors() {
    let body = r#"{
        "sender": { "postalCode": "28013" },
        "recipient": { "street": "Gran Vía 1" },
        "stops": [{ "street": "Sol 1" }, { "street": "" }],
        "pickups": [null, {}],
        "depots": { "north": { "postalCode": "1" } },
        "rounds": { "monday": [{}] }
    }"#;

    assert_eq!(
        violations_found_reading::<Consignment>(body),
        violations_found_reading::<ConsignmentStoppingAtATypeError>(body)
    );
}

#[test]
fn a_well_typed_nested_body_breaking_a_struct_level_rule_is_reported_as_without_collect_type_errors() {
    let body = r#"{
        "sender": { "street": "Gran Vía 1", "postalCode": "28013" },
        "recipient": { "street": "Gran Vía 1", "postalCode": "28013" },
        "returnAddress": { "street": "Atocha 2", "postalCode": "28012" },
        "stops": [], "pickups": [], "depots": {}, "rounds": {}
    }"#;

    assert_eq!(
        violations_found_reading::<Consignment>(body),
        violations_found_reading::<ConsignmentStoppingAtATypeError>(body)
    );
}

#[test]
fn a_well_typed_nested_body_breaking_no_rule_is_valid_as_without_collect_type_errors() {
    let body = r#"{
        "sender": { "street": "Gran Vía 1", "postalCode": "28013" },
        "recipient": { "street": "Atocha 2", "postalCode": "28012" },
        "returnAddress": { "street": "Gran Vía 1", "postalCode": "28013" },
        "stops": [{ "street": "Sol 1", "postalCode": "28013" }],
        "pickups": [{ "street": "Sol 1", "postalCode": "28013" }],
        "depots": { "north": { "street": "Alcalá 1", "postalCode": "28014" } },
        "rounds": { "monday": [{ "street": "Sol 1", "postalCode": "28013" }] }
    }"#;

    assert_eq!(
        violations_found_reading::<Consignment>(body),
        violations_found_reading::<ConsignmentStoppingAtATypeError>(body)
    );
}

#[test]
fn failing_fast_on_a_well_typed_nested_body_answers_as_without_collect_type_errors() {
    let body = r#"{
        "sender": { "street": "Gran Vía 1", "postalCode": "28013" },
        "recipient": { "street": "Atocha 2", "postalCode": "28012" },
        "returnAddress": null,
        "stops": [{ "street": "Sol 1", "postalCode": "1" }],
        "pickups": [], "depots": {}, "rounds": {}
    }"#;

    assert_eq!(
        fail_fast_violations_reading::<Consignment>(body),
        fail_fast_violations_reading::<ConsignmentStoppingAtATypeError>(body)
    );
}
