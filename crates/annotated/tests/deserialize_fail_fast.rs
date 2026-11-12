//! `from_deserializer_fail_fast` answers only the first issue: the first that `from_deserializer` lists.

#![cfg(feature = "serde")]

use annotated::{Deserialize, DeserializeError, Issue, Problem};

fn issues_reading<'de, T: Deserialize<'de>>(json: &'de str) -> Vec<String> {
    let outcome = annotated::from_deserializer::<T, _>(&mut serde_json::Deserializer::from_str(json));
    paths_and_codes(outcome)
}

fn fail_fast_issues_reading<'de, T: Deserialize<'de>>(json: &'de str) -> Vec<String> {
    let outcome = annotated::from_deserializer_fail_fast::<T, _>(&mut serde_json::Deserializer::from_str(json));
    paths_and_codes(outcome)
}

fn paths_and_codes<T>(outcome: Result<T, DeserializeError<serde_json::Error>>) -> Vec<String> {
    match outcome {
        Ok(_) => Vec::new(),
        Err(DeserializeError::Issues(issues)) => issues.iter().map(path_and_code).collect(),
        Err(error) => panic!("expected issues, got the format's error: {error}"),
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
fn failing_fast_answers_only_the_first_issue_of_the_full_report() {
    let body = r#"{ "fullName": "  ", "age": 12 }"#;

    assert_eq!(
        issues_reading::<NewMember>(body),
        ["fullName: required", "email: missing", "age: range"]
    );
    assert_eq!(fail_fast_issues_reading::<NewMember>(body), ["fullName: required"]);
}

#[test]
fn failing_fast_answers_a_missing_field_declared_before_a_broken_rule() {
    let body = r#"{ "fullName": "Alex Morgan", "age": 12 }"#;

    assert_eq!(issues_reading::<NewMember>(body), ["email: missing", "age: range"]);
    assert_eq!(fail_fast_issues_reading::<NewMember>(body), ["email: missing"]);
}

#[test]
fn failing_fast_answers_a_broken_rule_declared_before_a_missing_field() {
    let body = r#"{ "fullName": "Alex Morgan", "email": "alex" }"#;

    assert_eq!(issues_reading::<NewMember>(body), ["email: email", "age: missing"]);
    assert_eq!(fail_fast_issues_reading::<NewMember>(body), ["email: email"]);
}

#[test]
fn failing_fast_answers_the_first_issue_in_declaration_order_whatever_order_fields_are_read_in() {
    let body = r#"{ "age": 12, "fullName": "" }"#;

    assert_eq!(fail_fast_issues_reading::<NewMember>(body), ["fullName: required"]);
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

    assert_eq!(issues_reading::<Handle>(body), ["handle: length", "handle: prefix"]);
    assert_eq!(fail_fast_issues_reading::<Handle>(body), ["handle: length"]);
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
fn failing_fast_runs_no_struct_level_rule_after_the_first_issue() {
    let body = r#"{ "password": "horse", "repeatedPassword": "battery" }"#;

    assert_eq!(
        issues_reading::<NewPassword>(body),
        ["password: length", "passwords_differ"]
    );
    assert_eq!(fail_fast_issues_reading::<NewPassword>(body), ["password: length"]);
}

#[test]
fn failing_fast_answers_a_struct_level_rule_when_it_breaks_first() {
    let body = r#"{ "password": "correct horse", "repeatedPassword": "battery" }"#;

    assert_eq!(fail_fast_issues_reading::<NewPassword>(body), ["passwords_differ"]);
}

fn must_not_be_checked(_: &str) -> Result<(), Problem> {
    panic!("checked after failing fast found its issue")
}

fn signpost_must_not_be_checked(_: &Signpost) -> Result<(), Problem> {
    panic!("checked after failing fast found its issue")
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

    assert_eq!(fail_fast_issues_reading::<Signpost>(body), ["destination: missing"]);
}

#[test]
fn failing_fast_checks_nothing_after_a_broken_rule() {
    let body = r#"{ "destination": "", "note": "" }"#;

    assert_eq!(fail_fast_issues_reading::<Signpost>(body), ["destination: required"]);
}

#[derive(Deserialize, Debug)]
struct Member {
    #[validate(required)]
    #[allow(dead_code)]
    name: String,
}

#[derive(Deserialize, Debug)]
struct Team {
    #[validate(each(nested))]
    #[allow(dead_code)]
    members: Vec<Member>,
    #[validate(required)]
    #[allow(dead_code)]
    name: String,
}

#[test]
fn failing_fast_answers_an_issue_inside_the_second_element_of_a_list_before_a_later_missing_field() {
    let body = r#"{ "members": [{ "name": "Alex" }, { "name": "" }] }"#;

    assert_eq!(
        issues_reading::<Team>(body),
        ["members[1].name: required", "name: missing"]
    );
    assert_eq!(fail_fast_issues_reading::<Team>(body), ["members[1].name: required"]);
}

#[derive(Deserialize, Debug)]
struct Crew {
    #[validate(length(max = 1), each(nested))]
    #[allow(dead_code)]
    members: Vec<Member>,
}

#[test]
fn failing_fast_answers_a_rule_a_list_breaks_before_the_issues_inside_its_elements() {
    let body = r#"{ "members": [{ "name": "" }, { "name": "" }] }"#;

    assert_eq!(issues_reading::<Crew>(body), ["members: length"]);
    assert_eq!(fail_fast_issues_reading::<Crew>(body), ["members: length"]);
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Address {
    #[validate(required)]
    #[allow(dead_code)]
    street: String,
    #[validate(length(min = 5))]
    #[allow(dead_code)]
    postal_code: String,
}

fn not_a_restricted_area(_: &Address) -> Result<(), Problem> {
    Err(Problem::custom("restricted_area", "is in a restricted area"))
}

#[derive(Deserialize, Debug)]
struct Shipment {
    #[validate(custom(not_a_restricted_area), nested)]
    #[allow(dead_code)]
    destination: Address,
}

#[test]
fn failing_fast_answers_a_rule_a_nested_value_breaks_before_the_issues_inside_it() {
    let body = r#"{ "destination": { "street": "", "postalCode": "28013" } }"#;

    assert_eq!(issues_reading::<Shipment>(body), ["destination: restricted_area"]);
    assert_eq!(
        fail_fast_issues_reading::<Shipment>(body),
        ["destination: restricted_area"]
    );
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Parcel {
    #[validate(nested)]
    #[allow(dead_code)]
    sender: Address,
    #[validate(range(min = 1))]
    #[allow(dead_code)]
    weight_in_grams: u32,
}

#[test]
fn failing_fast_answers_the_first_issue_inside_a_nested_struct() {
    let body = r#"{ "sender": { "street": "", "postalCode": "1" }, "weightInGrams": 0 }"#;

    assert_eq!(
        issues_reading::<Parcel>(body),
        [
            "sender.street: required",
            "sender.postalCode: length",
            "weightInGrams: range"
        ]
    );
    assert_eq!(fail_fast_issues_reading::<Parcel>(body), ["sender.street: required"]);
}

#[test]
fn failing_fast_answers_a_field_missing_inside_a_nested_struct() {
    let body = r#"{ "weightInGrams": 0, "sender": { "postalCode": "1" } }"#;

    assert_eq!(
        issues_reading::<Parcel>(body),
        [
            "sender.street: missing",
            "sender.postalCode: length",
            "weightInGrams: range"
        ]
    );
    assert_eq!(fail_fast_issues_reading::<Parcel>(body), ["sender.street: missing"]);
}

#[derive(Deserialize, Debug)]
struct Warehouse {
    #[validate(each(nested))]
    #[allow(dead_code)]
    stock: std::collections::BTreeMap<String, Stock>,
}

#[derive(Deserialize, Debug)]
struct Stock {
    #[validate(range(max = 100))]
    #[allow(dead_code)]
    quantity: u32,
}

#[test]
fn failing_fast_answers_the_first_issue_of_a_map_s_values_in_the_order_of_their_keys() {
    let body = r#"{ "stock": { "sku-2": { "quantity": 300 }, "sku-1": { "quantity": 300 } } }"#;

    assert_eq!(
        issues_reading::<Warehouse>(body),
        ["stock[sku-1].quantity: range", "stock[sku-2].quantity: range"]
    );
    assert_eq!(
        fail_fast_issues_reading::<Warehouse>(body),
        ["stock[sku-1].quantity: range"]
    );
}

#[derive(Deserialize, Debug)]
struct Pickups {
    #[validate(each(required, nested))]
    #[allow(dead_code)]
    members: Vec<Option<Member>>,
}

#[test]
fn failing_fast_answers_an_element_breaking_a_rule_before_one_with_issues_inside() {
    let body = r#"{ "members": [{ "name": "Alex" }, null, { "name": "" }] }"#;

    assert_eq!(
        issues_reading::<Pickups>(body),
        ["members[1]: required", "members[2].name: required"]
    );
    assert_eq!(fail_fast_issues_reading::<Pickups>(body), ["members[1]: required"]);
}

#[derive(Deserialize, Debug)]
struct Route {
    #[validate(nested)]
    #[allow(dead_code)]
    first_stop: Signpost,
    #[validate(custom(must_not_be_checked))]
    #[allow(dead_code)]
    last_stop: String,
}

#[test]
fn failing_fast_checks_nothing_after_an_issue_inside_a_nested_struct() {
    let body = r#"{ "first_stop": { "destination": "", "note": "" }, "last_stop": "" }"#;

    assert_eq!(
        fail_fast_issues_reading::<Route>(body),
        ["first_stop.destination: required"]
    );
}

#[test]
fn failing_fast_checks_nothing_after_a_nested_struct_that_could_not_be_built() {
    let body = r#"{ "first_stop": { "note": "" }, "last_stop": "" }"#;

    assert_eq!(
        fail_fast_issues_reading::<Route>(body),
        ["first_stop.destination: missing"]
    );
}

fn member_must_not_be_checked(_: &Member) -> Result<(), Problem> {
    panic!("checked after failing fast found its issue")
}

#[derive(Deserialize, Debug)]
struct Squad {
    #[validate(each(nested, custom(member_must_not_be_checked)))]
    #[allow(dead_code)]
    members: Vec<Member>,
}

#[test]
fn failing_fast_checks_no_element_after_one_with_an_issue_inside() {
    let body = r#"{ "members": [{ "name": "" }, { "name": "Alex" }] }"#;

    assert_eq!(fail_fast_issues_reading::<Squad>(body), ["members[0].name: required"]);
}
