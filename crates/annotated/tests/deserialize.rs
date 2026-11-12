//! `#[derive(Deserialize)]` reads a struct collecting every issue in it, missing fields and broken rules
//! together, in the order the struct declares its fields.

#![cfg(feature = "serde")]

use annotated::{Deserialize, DeserializeError, Issue, ParamValue, Path, Problem, Validate};

fn deserialize<'de, T: Deserialize<'de>>(json: &'de str) -> Result<T, DeserializeError<serde_json::Error>> {
    annotated::from_deserializer(&mut serde_json::Deserializer::from_str(json))
}

fn issues_found_reading<'de, T: Deserialize<'de>>(json: &'de str) -> Vec<Issue> {
    match deserialize::<T>(json) {
        Ok(_) => Vec::new(),
        Err(DeserializeError::Issues(issues)) => issues.into_iter().collect(),
        Err(error) => panic!("expected issues, got the format's error: {error}"),
    }
}

fn issues_reading<'de, T: Deserialize<'de>>(json: &'de str) -> Vec<String> {
    issues_found_reading::<T>(json).iter().map(path_and_code).collect()
}

fn path_and_code(issue: &Issue) -> String {
    let code = issue.problem().code();
    if issue.path().is_root() {
        return code.to_owned();
    }
    format!("{}: {code}", issue.path())
}

fn format_error_reading<'de, T: Deserialize<'de>>(json: &'de str) -> (String, Vec<Issue>) {
    match deserialize::<T>(json) {
        Err(error @ DeserializeError::Format(_)) => (error.to_string(), error.issues().to_vec()),
        Ok(_) => panic!("expected the format's error, got a value"),
        Err(error) => panic!("expected the format's error, got {error:?}"),
    }
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
fn every_problem_with_a_new_member_is_reported_at_once() {
    let body = r#"{ "fullName": "  ", "age": 12 }"#;

    let error = deserialize::<NewMember>(body).unwrap_err();

    let [blank_name, missing_email, too_young] = error.issues() else {
        panic!("expected three issues, got {error:?}");
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
fn a_new_member_breaking_no_rule_is_deserialized() {
    let body = r#"{ "fullName": "Alex Morgan", "email": "alex@example.com", "age": 30 }"#;

    let member = deserialize::<NewMember>(body).unwrap();

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
fn every_missing_field_is_reported_by_its_serde_name() {
    assert_eq!(
        issues_reading::<NewMember>("{}"),
        ["fullName: missing", "email: missing", "age: missing"]
    );
}

#[test]
fn issues_come_in_the_order_the_fields_are_declared_whatever_order_they_are_read_in() {
    let body = r#"{ "age": 12, "email": "alex", "fullName": "" }"#;

    assert_eq!(
        issues_reading::<NewMember>(body),
        ["fullName: required", "email: email", "age: range"]
    );
}

#[derive(Deserialize, Debug, PartialEq)]
struct Profile {
    #[validate(length(min = 2))]
    nickname: Option<String>,
    #[validate(required)]
    phone: Option<String>,
}

#[test]
fn a_missing_optional_field_is_none_without_an_issue() {
    let profile = deserialize::<Profile>(r#"{ "phone": "600 000 000" }"#).unwrap();

    assert_eq!(
        profile,
        Profile {
            nickname: None,
            phone: Some("600 000 000".to_owned()),
        }
    );
}

#[test]
fn a_missing_optional_field_that_is_required_breaks_the_required_rule() {
    assert_eq!(issues_reading::<Profile>("{}"), ["phone: required"]);
}

#[test]
fn the_value_inside_an_optional_field_is_checked_by_its_rules() {
    assert_eq!(
        issues_reading::<Profile>(r#"{ "nickname": "A", "phone": "600 000 000" }"#),
        ["nickname: length"]
    );
}

#[derive(Deserialize, Debug, PartialEq)]
struct Contact {
    #[serde(rename = "e-mail", alias = "mail", alias = "correo")]
    #[validate(email)]
    email: String,
}

#[test]
fn a_renamed_field_is_read_and_reported_by_its_new_name() {
    assert_eq!(issues_reading::<Contact>(r#"{ "e-mail": "alex" }"#), ["e-mail: email"]);
}

#[test]
fn an_aliased_field_is_read_by_every_alias_and_reported_by_its_serde_name() {
    assert_eq!(issues_reading::<Contact>(r#"{ "mail": "alex" }"#), ["e-mail: email"]);
    assert_eq!(issues_reading::<Contact>(r#"{ "correo": "alex" }"#), ["e-mail: email"]);
}

#[test]
fn a_renamed_field_is_not_read_by_its_rust_name() {
    assert_eq!(
        issues_reading::<Contact>(r#"{ "email": "alex@example.com" }"#),
        ["e-mail: missing"]
    );
}

#[derive(serde::Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Listing {
    #[validate(required)]
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(max = 10))]
    subtitle: Option<String>,
}

#[test]
fn a_struct_serialized_without_its_absent_values_is_read_and_checked_as_usual() {
    let body = r#"{ "title": "", "subtitle": "Far too long a subtitle" }"#;

    assert_eq!(issues_reading::<Listing>(body), ["title: required", "subtitle: length"]);
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde passes a field to `serialize_with` by reference"
)]
fn nights_in_words<S: serde::Serializer>(nights: &u8, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&format!("{nights} nights"))
}

#[derive(serde::Serialize, Deserialize, Debug)]
#[serde(rename_all(serialize = "UPPERCASE"), bound(serialize = ""))]
struct Booking {
    #[serde(rename(serialize = "guest"), skip_serializing_if = "String::is_empty")]
    #[validate(required)]
    guest_name: String,
    #[serde(serialize_with = "nights_in_words")]
    #[validate(range(min = 1))]
    nights: u8,
    #[serde(skip_serializing)]
    #[validate(length(max = 20))]
    #[allow(dead_code)]
    note: Option<String>,
}

#[test]
fn attributes_that_only_change_serializing_leave_a_struct_read_and_checked_as_usual() {
    let body = r#"{ "guest_name": "", "nights": 0, "note": "Late arrival, after midnight" }"#;

    assert_eq!(
        issues_reading::<Booking>(body),
        ["guest_name: required", "nights: range", "note: length"]
    );
}

#[derive(serde::Serialize, Deserialize, Debug, Clone)]
#[serde(into = "ReservationCode")]
struct Reservation {
    #[validate(length(min = 6))]
    code: String,
}

#[derive(serde::Serialize)]
struct ReservationCode(String);

impl From<Reservation> for ReservationCode {
    fn from(reservation: Reservation) -> Self {
        Self(reservation.code)
    }
}

#[test]
fn a_struct_serialized_into_another_type_is_read_and_checked_as_usual() {
    assert_eq!(issues_reading::<Reservation>(r#"{ "code": "AB1" }"#), ["code: length"]);
}

#[derive(Deserialize, Debug)]
struct Guest {
    #[serde(rename(serialize = "who", deserialize = "guest"))]
    #[validate(required)]
    #[allow(dead_code)]
    name: String,
}

#[test]
fn a_field_renamed_apart_for_serializing_and_deserializing_is_read_by_its_deserializing_name() {
    assert_eq!(issues_reading::<Guest>(r#"{ "guest": "" }"#), ["guest: required"]);
}

#[derive(Deserialize, Debug)]
struct Expense {
    #[validate(required)]
    #[allow(dead_code)]
    r#type: String,
}

#[test]
fn a_raw_identifier_is_read_and_reported_without_its_prefix() {
    assert_eq!(issues_reading::<Expense>(r#"{ "type": "" }"#), ["type: required"]);
}

#[test]
fn an_unknown_field_is_ignored() {
    let body = r#"{ "fullName": "Alex Morgan", "nickname": "Al", "email": "alex@example.com", "age": 30 }"#;

    let member = deserialize::<NewMember>(body).unwrap();

    assert_eq!(
        member,
        NewMember {
            full_name: "Alex Morgan".to_owned(),
            email: "alex@example.com".to_owned(),
            age: 30,
        }
    );
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct LogIn {
    #[validate(email)]
    #[allow(dead_code)]
    email: String,
}

#[test]
fn an_unknown_field_is_the_format_s_error_when_the_struct_denies_unknown_fields() {
    let body = r#"{ "email": "alex", "password": "correct horse" }"#;

    assert_eq!(
        format_error_reading::<LogIn>(body),
        (
            "unknown field `password`, expected `email` at line 1 column 29".to_owned(),
            Vec::new()
        )
    );
}

#[test]
fn a_known_field_is_read_when_the_struct_denies_unknown_fields() {
    assert_eq!(issues_reading::<LogIn>(r#"{ "email": "alex" }"#), ["email: email"]);
}

#[test]
fn a_field_given_twice_is_the_format_s_error() {
    let body = r#"{ "email": "alex@example.com", "email": "alex" }"#;

    assert_eq!(
        format_error_reading::<LogIn>(body),
        ("duplicate field `email` at line 1 column 38".to_owned(), Vec::new())
    );
}

#[test]
fn a_value_of_the_wrong_type_is_the_format_s_error() {
    let body = r#"{ "fullName": "", "email": "alex@example.com", "age": "twelve" }"#;

    assert_eq!(
        format_error_reading::<NewMember>(body),
        (
            r#"invalid type: string "twelve", expected u8 at line 1 column 62"#.to_owned(),
            Vec::new()
        )
    );
}

#[derive(Deserialize, Debug)]
#[serde(expecting = "a new member as an object")]
struct Invitation {
    #[allow(dead_code)]
    email: String,
}

#[test]
fn a_value_that_is_not_a_struct_is_the_format_s_error_saying_what_was_expected() {
    assert_eq!(
        format_error_reading::<Invitation>(r#""alex@example.com""#),
        (
            r#"invalid type: string "alex@example.com", expected a new member as an object at line 1 column 18"#
                .to_owned(),
            Vec::new()
        )
    );
}

#[test]
fn a_struct_written_as_a_sequence_is_read_in_declaration_order() {
    assert_eq!(
        issues_reading::<NewMember>(r#"["", "alex", 12]"#),
        ["fullName: required", "email: email", "age: range"]
    );
}

#[test]
fn the_fields_missing_at_the_end_of_a_sequence_are_missing() {
    assert_eq!(
        issues_reading::<NewMember>(r#"["Alex Morgan"]"#),
        ["email: missing", "age: missing"]
    );
}

#[test]
fn an_optional_field_missing_at_the_end_of_a_sequence_is_none() {
    let profile = deserialize::<Profile>(r#"[null, "600 000 000"]"#).unwrap();

    assert_eq!(
        profile,
        Profile {
            nickname: None,
            phone: Some("600 000 000".to_owned()),
        }
    );
}

fn passwords_match(new_password: &NewPassword) -> Result<(), Problem> {
    if new_password.repeated_password != new_password.password {
        return Err(Problem::custom("passwords_differ", "must match the password"));
    }
    Ok(())
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[validate(custom(passwords_match, field = repeated_password))]
struct NewPassword {
    #[validate(length(min = 8))]
    password: String,
    repeated_password: String,
}

#[test]
fn a_struct_level_rule_runs_once_every_field_is_read() {
    let body = r#"{ "password": "correct horse", "repeatedPassword": "correct" }"#;

    assert_eq!(
        issues_reading::<NewPassword>(body),
        ["repeatedPassword: passwords_differ"]
    );
}

#[test]
fn a_struct_level_rule_at_a_field_is_skipped_when_the_field_is_missing() {
    let body = r#"{ "password": "correct horse" }"#;

    assert_eq!(issues_reading::<NewPassword>(body), ["repeatedPassword: missing"]);
}

fn nickname_differs_from_the_name(player: &Player) -> Result<(), Problem> {
    if player.nickname.as_deref() == Some(player.full_name.as_str()) {
        return Err(Problem::custom("nickname_is_the_name", "can't repeat the name"));
    }
    Ok(())
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[validate(custom(nickname_differs_from_the_name, field = nickname))]
struct Player {
    full_name: String,
    #[validate(length(min = 2))]
    nickname: Option<String>,
}

#[test]
fn a_struct_level_rule_at_an_optional_field_is_skipped_when_the_value_inside_breaks_its_own_rules() {
    let body = r#"{ "fullName": "A", "nickname": "A" }"#;

    assert_eq!(issues_reading::<Player>(body), ["nickname: length"]);
}

#[test]
fn a_struct_level_rule_at_an_optional_field_runs_when_the_value_inside_follows_its_own_rules() {
    let body = r#"{ "fullName": "Alex", "nickname": "Alex" }"#;

    assert_eq!(issues_reading::<Player>(body), ["nickname: nickname_is_the_name"]);
}

fn ends_after_it_starts(stay: &Stay) -> Result<(), Problem> {
    if stay.ends_on <= stay.starts_on {
        return Err(Problem::custom("ends_before_it_starts", "must end after it starts"));
    }
    Ok(())
}

#[derive(Deserialize, Debug)]
#[validate(custom(ends_after_it_starts))]
struct Stay {
    starts_on: u32,
    ends_on: u32,
    #[allow(dead_code)]
    guest: String,
}

#[test]
fn a_struct_level_rule_is_skipped_when_any_field_is_missing() {
    assert_eq!(
        issues_reading::<Stay>(r#"{ "starts_on": 5, "ends_on": 3 }"#),
        ["guest: missing"]
    );
}

#[test]
fn a_struct_level_rule_at_the_struct_runs_when_every_field_is_there() {
    assert_eq!(
        issues_reading::<Stay>(r#"{ "starts_on": 5, "ends_on": 3, "guest": "Lucía" }"#),
        ["ends_before_it_starts"]
    );
}

#[derive(Deserialize, Debug)]
#[validate(report_all)]
struct Handle {
    #[validate(length(min = 3), prefix = "@")]
    #[allow(dead_code)]
    handle: String,
}

#[test]
fn every_rule_a_field_breaks_is_reported_under_report_all() {
    assert_eq!(
        issues_reading::<Handle>(r#"{ "handle": "a" }"#),
        ["handle: length", "handle: prefix"]
    );
}

#[derive(Deserialize, Debug, PartialEq)]
struct Page<T> {
    #[validate(length(max = 2))]
    items: Vec<T>,
    #[validate(range(min = 1))]
    number: u32,
}

#[test]
fn a_struct_with_a_type_parameter_is_read_with_the_type_given() {
    let page = deserialize::<Page<u8>>(r#"{ "items": [7, 8], "number": 1 }"#).unwrap();

    assert_eq!(
        page,
        Page {
            items: vec![7, 8],
            number: 1
        }
    );
}

#[test]
fn a_struct_with_a_type_parameter_reports_its_issues() {
    assert_eq!(
        issues_reading::<Page<u8>>(r#"{ "items": [7, 8, 9], "number": 0 }"#),
        ["items: length", "number: range"]
    );
}

#[derive(Deserialize, Debug, PartialEq)]
struct Envelope<T> {
    content: T,
    #[validate(required)]
    recipient: String,
}

#[test]
fn a_field_whose_type_is_the_type_parameter_itself_is_read_with_the_type_given() {
    let envelope = deserialize::<Envelope<u8>>(r#"{ "content": 7, "recipient": "Sam" }"#).unwrap();

    assert_eq!(
        envelope,
        Envelope {
            content: 7,
            recipient: "Sam".to_owned()
        }
    );
}

#[derive(Deserialize, Debug, PartialEq)]
struct NamesOfTheGeneratedCode {
    #[validate(range(min = 1))]
    value: u32,
    #[validate(required)]
    found: String,
    #[validate(required)]
    at: String,
    #[validate(required)]
    problem: String,
    map: u32,
    seq: u32,
    key: u32,
    deserializer: u32,
    deserialized: u32,
}

#[test]
fn fields_named_like_the_generated_code_s_own_names_are_read_and_reported() {
    let body = r#"{ "value": 0, "found": "", "at": "", "problem": "", "key": 1 }"#;

    assert_eq!(
        issues_reading::<NamesOfTheGeneratedCode>(body),
        [
            "value: range",
            "found: required",
            "at: required",
            "problem: required",
            "map: missing",
            "seq: missing",
            "deserializer: missing",
            "deserialized: missing",
        ]
    );
}

#[test]
fn plain_serde_json_refuses_a_body_breaking_a_rule_with_every_issue_in_its_message() {
    let body = r#"{ "fullName": "  ", "age": 12 }"#;

    let error = serde_json::from_str::<NewMember>(body).unwrap_err();

    assert_eq!(
        error.to_string(),
        "fullName: is required\nemail: is missing\nage: must be between 18 and 120"
    );
}

#[test]
fn plain_serde_json_accepts_a_body_breaking_no_rule() {
    let body = r#"{ "fullName": "Alex Morgan", "email": "alex@example.com", "age": 30 }"#;

    let member = serde_json::from_str::<NewMember>(body).unwrap();

    assert_eq!(
        member,
        NewMember {
            full_name: "Alex Morgan".to_owned(),
            email: "alex@example.com".to_owned(),
            age: 30,
        }
    );
}

fn tags_differ_from_the_name(sign_up: &SignUp) -> Result<(), Problem> {
    no_tag_is_the_name(&sign_up.full_name, &sign_up.tags)
}

fn tags_differ_from_the_name_once_validated(sign_up: &SignUpThenValidated) -> Result<(), Problem> {
    no_tag_is_the_name(&sign_up.full_name, &sign_up.tags)
}

fn no_tag_is_the_name(full_name: &str, tags: &[String]) -> Result<(), Problem> {
    if tags.iter().any(|tag| tag == full_name) {
        return Err(Problem::custom("tag_is_the_name", "can't repeat the name"));
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
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
    #[validate(length(max = 2), each(length(min = 2)))]
    tags: Vec<String>,
}

#[derive(serde::Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
#[validate(custom(tags_differ_from_the_name_once_validated, field = tags))]
struct SignUpThenValidated {
    #[validate(required, length(max = 10))]
    full_name: String,
    #[serde(rename = "mail", alias = "email")]
    #[validate(email)]
    email: String,
    #[validate(length(min = 2))]
    nickname: Option<String>,
    #[validate(required)]
    phone: Option<String>,
    #[validate(length(max = 2), each(length(min = 2)))]
    tags: Vec<String>,
}

fn issues_validating_after_serde(json: &str) -> Vec<Issue> {
    let sign_up = serde_json::from_str::<SignUpThenValidated>(json).unwrap();
    sign_up
        .validate()
        .map_or_else(|issues| issues.into_iter().collect(), |()| Vec::new())
}

#[test]
fn a_complete_body_breaking_every_rule_is_reported_as_serde_then_validate_report_it() {
    let body = r#"{ "fullName": "Alexandra Morgan", "email": "alex", "nickname": "A", "phone": null,
        "tags": ["a", "b", "Alexandra Morgan"] }"#;

    assert_eq!(
        issues_found_reading::<SignUp>(body),
        issues_validating_after_serde(body)
    );
}

#[test]
fn a_complete_body_breaking_a_struct_level_rule_is_reported_as_serde_then_validate_report_it() {
    let body = r#"{ "fullName": "Alex", "mail": "alex@example.com", "phone": "600 000 000", "tags": ["Alex"] }"#;

    assert_eq!(
        issues_found_reading::<SignUp>(body),
        issues_validating_after_serde(body)
    );
}

#[test]
fn a_complete_body_breaking_no_rule_is_valid_as_serde_then_validate_find_it() {
    let body = r#"{ "fullName": "Alex", "mail": "alex@example.com", "phone": "600 000 000", "tags": [] }"#;

    assert_eq!(
        issues_found_reading::<SignUp>(body),
        issues_validating_after_serde(body)
    );
}
