//! `#[derive(Deserialize)]` reads an enum as serde does, and collects every missing field and broken rule in the
//! variant it reads, each at the path where its value is in the input.

#![cfg(feature = "serde")]

use std::borrow::Cow;
use std::collections::BTreeMap;

use annotated::{Deserialize, DeserializeError, Issue, Problem, Validate};

fn deserialize<'de, T: Deserialize<'de>>(json: &'de str) -> Result<T, DeserializeError<serde_json::Error>> {
    annotated::from_deserializer(&mut serde_json::Deserializer::from_str(json))
}

fn read<'de, T: Deserialize<'de>>(json: &'de str) -> Result<T, String> {
    deserialize(json).map_err(|error| error.to_string())
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
        Err(DeserializeError::Issues(issues)) => issues.iter().map(described).collect(),
        Err(error) => vec![format!("the format's error: {error}")],
    }
}

fn described(issue: &Issue) -> String {
    let params = issue.problem().params();
    let params = params
        .iter()
        .map(|(name, value)| format!("{name}: {value}"))
        .collect::<Vec<_>>();
    let code_and_params = if params.is_empty() {
        issue.problem().code().to_owned()
    } else {
        format!("{}({})", issue.problem().code(), params.join(", "))
    };
    if issue.path().is_root() {
        return code_and_params;
    }
    format!("{}: {code_and_params}", issue.path())
}

fn issues_found_reading<'de, T: Deserialize<'de>>(json: &'de str) -> Vec<Issue> {
    match deserialize::<T>(json) {
        Ok(_) => Vec::new(),
        Err(DeserializeError::Issues(issues)) => issues.into_iter().collect(),
        Err(error) => panic!("expected issues, got the format's error: {error}"),
    }
}

fn issues_validating_after_serde<'de, T: serde::Deserialize<'de> + Validate>(json: &'de str) -> Vec<Issue> {
    let value = serde_json::from_str::<T>(json).unwrap();
    value
        .validate()
        .map_or_else(|issues| issues.into_iter().collect(), |()| Vec::new())
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
enum Payment {
    Card {
        #[validate(length(min = 16))]
        number: String,
        #[validate(required)]
        holder: String,
    },
    #[serde(alias = "wire")]
    BankTransfer {
        #[validate(required)]
        iban: String,
    },
    Cash,
}

#[test]
fn a_struct_variant_is_read_from_its_name_and_its_fields() {
    let body = r#"{ "card": { "number": "4111111111111111", "holder": "Ada Lovelace" } }"#;

    assert_eq!(
        read::<Payment>(body),
        Ok(Payment::Card {
            number: "4111111111111111".to_owned(),
            holder: "Ada Lovelace".to_owned(),
        })
    );
}

#[test]
fn a_unit_variant_is_read_from_its_name() {
    assert_eq!(read::<Payment>(r#""cash""#), Ok(Payment::Cash));
}

#[test]
fn a_variant_is_read_by_its_alias() {
    assert_eq!(
        read::<Payment>(r#"{ "wire": { "iban": "ES9121000418450200051332" } }"#),
        Ok(Payment::BankTransfer {
            iban: "ES9121000418450200051332".to_owned(),
        })
    );
}

#[test]
fn an_issue_in_a_variant_is_under_the_variant_s_name() {
    let body = r#"{ "card": { "number": "4111", "holder": "Ada Lovelace" } }"#;

    assert_eq!(
        issues_reading::<Payment>(body),
        ["card.number: length(min: 16, unit: chars)"]
    );
}

#[test]
fn a_field_missing_from_a_variant_is_missing_under_the_variant_s_name() {
    let body = r#"{ "card": { "number": "4111111111111111" } }"#;

    assert_eq!(issues_reading::<Payment>(body), ["card.holder: missing"]);
}

#[test]
fn a_struct_variant_is_read_from_a_sequence_as_with_serde() {
    let body = r#"{ "card": ["4111111111111111", ""] }"#;

    assert_eq!(issues_reading::<Payment>(body), ["card.holder: required"]);
}

#[test]
fn a_name_that_is_no_variant_is_the_format_s_error() {
    assert_eq!(
        issues_reading::<Payment>(r#"{ "cheque": {} }"#),
        [
            "the format's error: unknown variant `cheque`, expected one of `card`, `bankTransfer`, `wire`, `cash` at \
          line 1 column 10"
        ]
    );
}

#[test]
fn without_collect_type_errors_a_value_of_the_wrong_type_in_a_variant_is_the_format_s_error() {
    let body = r#"{ "card": { "number": 4111, "holder": "Ada Lovelace" } }"#;

    assert_eq!(
        issues_reading::<Payment>(body),
        ["the format's error: invalid type: integer `4111`, expected a string at line 1 column 26"]
    );
}

#[test]
fn failing_fast_an_enum_answers_its_first_issue() {
    let body = r#"{ "card": { "number": "4111", "holder": "" } }"#;

    assert_eq!(
        fail_fast_issues_reading::<Payment>(body),
        ["card.number: length(min: 16, unit: chars)"]
    );
}

#[test]
fn serde_s_deserialize_reports_the_issues_in_the_variant() {
    let body = r#"{ "card": { "number": "4111", "holder": "" } }"#;

    let error = serde_json::from_str::<Payment>(body).unwrap_err();

    assert_eq!(
        error.to_string(),
        "card.number: must be at least 16 characters long\ncard.holder: is required"
    );
}

#[derive(serde::Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
enum PaymentThenValidated {
    Card {
        #[validate(length(min = 16))]
        number: String,
        #[validate(required)]
        holder: String,
    },
    #[serde(alias = "wire")]
    BankTransfer {
        #[validate(required)]
        iban: String,
    },
    Cash,
}

#[test]
fn a_complete_body_is_reported_as_serde_then_validate_report_it() {
    let body = r#"{ "card": { "number": "4111", "holder": "" } }"#;

    assert_eq!(
        issues_found_reading::<Payment>(body),
        issues_validating_after_serde::<PaymentThenValidated>(body)
    );
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
enum Shape {
    Circle(#[validate(range(min = 1))] u32),
    Rectangle(#[validate(range(min = 1))] u32, #[validate(range(min = 1))] u32),
}

#[test]
fn a_newtype_variant_s_value_is_read() {
    assert_eq!(read::<Shape>(r#"{ "circle": 3 }"#), Ok(Shape::Circle(3)));
}

#[test]
fn a_newtype_variant_s_value_is_checked_at_the_variant_s_name() {
    assert_eq!(issues_reading::<Shape>(r#"{ "circle": 0 }"#), ["circle: range(min: 1)"]);
}

#[test]
fn a_tuple_variant_s_elements_are_read() {
    assert_eq!(read::<Shape>(r#"{ "rectangle": [3, 4] }"#), Ok(Shape::Rectangle(3, 4)));
}

#[test]
fn a_tuple_variant_s_elements_are_checked_at_their_index() {
    assert_eq!(
        issues_reading::<Shape>(r#"{ "rectangle": [3, 0] }"#),
        ["rectangle[1]: range(min: 1)"]
    );
}

#[test]
fn an_element_missing_from_a_tuple_variant_is_missing_at_its_index() {
    assert_eq!(
        issues_reading::<Shape>(r#"{ "rectangle": [3] }"#),
        ["rectangle[1]: missing"]
    );
}

#[derive(Deserialize, Validate, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Address {
    #[validate(required)]
    street: String,
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
enum Shipping {
    Courier {
        #[validate(nested)]
        address: Address,
    },
    Locker(#[validate(nested)] Address),
    Relay(#[validate(nested)] Address, #[validate(nested)] Address),
    Pallet {
        #[serde(flatten)]
        #[validate(nested)]
        address: Address,
    },
}

#[test]
fn a_field_missing_inside_a_nested_value_in_a_variant_is_missing_at_its_full_path() {
    assert_eq!(
        issues_reading::<Shipping>(r#"{ "courier": { "address": {} } }"#),
        ["courier.address.street: missing"]
    );
}

#[test]
fn a_nested_value_in_a_newtype_variant_is_read_at_the_variant_s_name() {
    assert_eq!(
        issues_reading::<Shipping>(r#"{ "locker": {} }"#),
        ["locker.street: missing"]
    );
}

#[test]
fn a_field_missing_inside_a_nested_element_of_a_tuple_variant_is_missing_at_its_full_path() {
    assert_eq!(
        issues_reading::<Shipping>(r#"{ "relay": [{ "street": "Sol 1" }, {}] }"#),
        ["relay[1].street: missing"]
    );
}

#[test]
fn a_flattened_field_in_a_variant_is_read_from_the_variant_s_keys() {
    assert_eq!(
        read::<Shipping>(r#"{ "pallet": { "street": "Gran Vía 1" } }"#),
        Ok(Shipping::Pallet {
            address: Address {
                street: "Gran Vía 1".to_owned(),
            },
        })
    );
}

#[test]
fn a_flattened_field_in_a_variant_is_checked_at_the_variant_s_location() {
    assert_eq!(
        issues_reading::<Shipping>(r#"{ "pallet": { "street": "" } }"#),
        ["pallet.street: required"]
    );
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Order {
    #[validate(nested)]
    shipping: Shipping,
}

#[test]
fn an_enum_nested_in_a_struct_is_read_under_the_field_s_name() {
    let body = r#"{ "shipping": { "courier": { "address": { "street": "" } } } }"#;

    assert_eq!(
        issues_reading::<Order>(body),
        ["shipping.courier.address.street: required"]
    );
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all_fields = "camelCase")]
enum Delivery {
    PickUp {
        #[validate(required)]
        store_id: String,
    },
    #[serde(rename = "home")]
    Home {
        #[validate(required)]
        street: String,
    },
    #[serde(rename_all = "kebab-case")]
    Locker {
        #[validate(required)]
        locker_code: String,
    },
}

#[test]
fn a_variant_s_fields_are_read_by_the_names_the_enum_s_rename_all_fields_gives_them() {
    assert_eq!(
        issues_reading::<Delivery>(r#"{ "PickUp": { "storeId": "" } }"#),
        ["PickUp.storeId: required"]
    );
}

#[test]
fn a_renamed_variant_is_read_by_its_new_name() {
    assert_eq!(
        issues_reading::<Delivery>(r#"{ "home": {} }"#),
        ["home.street: missing"]
    );
}

#[test]
fn a_variant_s_own_rename_all_names_its_fields_over_the_enum_s_rename_all_fields() {
    assert_eq!(
        issues_reading::<Delivery>(r#"{ "Locker": { "locker-code": "" } }"#),
        ["Locker.locker-code: required"]
    );
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[allow(dead_code)]
enum Voucher {
    Gift {
        #[validate(required)]
        code: String,
    },
}

#[test]
fn an_enum_denying_unknown_fields_denies_them_in_its_variants() {
    assert_eq!(
        issues_reading::<Voucher>(r#"{ "gift": { "code": "XMAS", "pin": "1234" } }"#),
        ["the format's error: unknown field `pin`, expected `code` at line 1 column 33"]
    );
}

fn within_the_limit(refund: &Refund) -> Result<(), Problem> {
    match refund {
        Refund::Partial { cents, .. } if *cents > 10_000 => {
            Err(Problem::custom("over_the_limit", "must not exceed the limit"))
        }
        Refund::Partial { .. } | Refund::Full => Ok(()),
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[validate(custom(within_the_limit))]
#[allow(dead_code)]
enum Refund {
    Partial {
        #[validate(range(max = 5_000))]
        cents: u32,
        #[validate(required)]
        reason: String,
    },
    Full,
}

#[test]
fn an_enum_level_rule_reports_at_the_enum_s_location_after_the_variant_s_fields() {
    let body = r#"{ "partial": { "cents": 20000, "reason": "" } }"#;

    assert_eq!(
        issues_reading::<Refund>(body),
        [
            "partial.cents: range(max: 5000)",
            "partial.reason: required",
            "over_the_limit"
        ]
    );
}

#[test]
fn an_enum_level_rule_is_skipped_when_the_variant_can_t_be_built() {
    let body = r#"{ "partial": { "cents": 20000 } }"#;

    assert_eq!(
        issues_reading::<Refund>(body),
        ["partial.cents: range(max: 5000)", "partial.reason: missing"]
    );
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[annotated(collect_type_errors)]
enum Donation {
    Card {
        #[validate(length(min = 16))]
        number: String,
        #[validate(required)]
        holder: String,
    },
    Cash,
}

#[test]
fn under_collect_type_errors_a_value_of_the_wrong_type_in_a_variant_is_an_issue_at_its_path() {
    let body = r#"{ "card": { "number": 4111, "holder": "" } }"#;

    assert_eq!(
        issues_reading::<Donation>(body),
        [
            "card.number: invalid_type(expected: a string, found: integer)",
            "card.holder: required"
        ]
    );
}

#[test]
fn under_collect_type_errors_a_name_that_is_no_variant_is_an_issue_at_the_enum_s_location() {
    assert_eq!(
        issues_reading::<Donation>(r#"{ "cheque": {} }"#),
        ["unknown_variant(expected: card, cash)"]
    );
}

#[test]
fn under_collect_type_errors_a_value_that_is_not_the_enum_is_an_issue_at_its_location() {
    assert_eq!(
        issues_reading::<Donation>("12"),
        ["invalid_type(expected: enum Donation, found: integer)"]
    );
}

#[test]
fn under_collect_type_errors_a_variant_of_its_type_is_read() {
    assert_eq!(read::<Donation>(r#""cash""#), Ok(Donation::Cash));
}

#[derive(Deserialize, Debug, PartialEq)]
enum Lookup<T> {
    Found(#[validate(nested)] T),
    NotFound,
}

#[test]
fn an_enum_with_a_type_parameter_reads_the_value_it_holds() {
    assert_eq!(
        issues_reading::<Lookup<Address>>(r#"{ "Found": { "street": "" } }"#),
        ["Found.street: required"]
    );
}

#[derive(Deserialize, Debug, PartialEq)]
enum Reply<'a> {
    Text(#[validate(required)] &'a str),
    Empty,
}

#[test]
fn an_enum_with_a_lifetime_borrows_from_the_input() {
    assert_eq!(read::<Reply>(r#"{ "Text": "Hola" }"#), Ok(Reply::Text("Hola")));
}

fn comma_separated<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    let text = <String as serde::Deserialize>::deserialize(deserializer)?;
    Ok(text.split(',').map(ToOwned::to_owned).collect())
}

fn one() -> u32 {
    1
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
enum Post {
    Article {
        #[serde(deserialize_with = "comma_separated")]
        #[validate(length(max = 2))]
        tags: Vec<String>,
        #[serde(default = "one")]
        #[validate(range(min = 1))]
        revision: u32,
    },
}

#[test]
fn a_field_of_a_variant_is_read_with_its_serde_attributes() {
    assert_eq!(
        read::<Post>(r#"{ "article": { "tags": "rust,serde" } }"#),
        Ok(Post::Article {
            tags: vec!["rust".to_owned(), "serde".to_owned()],
            revision: 1,
        })
    );
}

#[test]
fn a_field_of_a_variant_read_by_a_function_is_checked_by_its_rules() {
    assert_eq!(
        issues_reading::<Post>(r#"{ "article": { "tags": "rust,serde,json" } }"#),
        ["article.tags: length(max: 2, unit: elements)"]
    );
}

fn route_must_not_be_checked(_: &Route) -> Result<(), Problem> {
    panic!("checked after failing fast found its issue")
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[validate(custom(route_must_not_be_checked))]
#[allow(dead_code)]
enum Route {
    Direct {
        #[validate(required)]
        destination: String,
    },
}

#[test]
fn failing_fast_checks_no_enum_level_rule_after_the_first_issue() {
    assert_eq!(
        fail_fast_issues_reading::<Route>(r#"{ "direct": { "destination": "" } }"#),
        ["direct.destination: required"]
    );
}

#[derive(Deserialize, Validate, Debug, PartialEq)]
struct Device {
    #[validate(required)]
    token: String,
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
enum Notification {
    #[serde(alias = "mail")]
    Email {
        #[validate(email)]
        address: String,
    },
    Push(#[validate(nested)] Device),
    Muted,
}

#[test]
fn an_internally_tagged_variant_is_read_from_its_tag_and_the_keys_beside_it() {
    assert_eq!(
        read::<Notification>(r#"{ "type": "email", "address": "ada@example.com" }"#),
        Ok(Notification::Email {
            address: "ada@example.com".to_owned(),
        })
    );
}

#[test]
fn the_tag_of_an_internally_tagged_variant_may_come_after_its_keys() {
    assert_eq!(
        read::<Notification>(r#"{ "address": "ada@example.com", "type": "mail" }"#),
        Ok(Notification::Email {
            address: "ada@example.com".to_owned(),
        })
    );
}

#[test]
fn an_internally_tagged_unit_variant_is_read_from_its_tag_alone() {
    assert_eq!(read::<Notification>(r#"{ "type": "muted" }"#), Ok(Notification::Muted));
}

#[test]
fn an_issue_in_an_internally_tagged_variant_is_at_the_enum_s_location() {
    assert_eq!(
        issues_reading::<Notification>(r#"{ "type": "email", "address": "ada" }"#),
        ["address: email"]
    );
}

#[test]
fn a_field_missing_from_an_internally_tagged_variant_is_missing_at_the_enum_s_location() {
    assert_eq!(
        issues_reading::<Notification>(r#"{ "type": "email" }"#),
        ["address: missing"]
    );
}

#[test]
fn an_internally_tagged_newtype_variant_reads_its_value_from_the_keys_beside_the_tag() {
    assert_eq!(
        issues_reading::<Notification>(r#"{ "type": "push", "token": "" }"#),
        ["token: required"]
    );
}

#[test]
fn an_internally_tagged_variant_is_read_from_a_sequence_led_by_its_tag_as_with_serde() {
    assert_eq!(
        issues_reading::<Notification>(r#"["email", "ada"]"#),
        ["address: email"]
    );
}

#[test]
fn a_missing_tag_is_missing_at_the_tag_s_name() {
    assert_eq!(
        issues_reading::<Notification>(r#"{ "address": "ada@example.com" }"#),
        ["type: missing"]
    );
}

#[test]
fn a_tag_that_names_no_variant_is_an_issue_at_the_tag_even_without_collect_type_errors() {
    assert_eq!(
        issues_reading::<Notification>(r#"{ "type": "fax", "number": "+34 600 000 000" }"#),
        ["type: unknown_variant(expected: email, mail, push, muted)"]
    );
}

#[test]
fn a_tag_that_isn_t_a_name_names_no_variant() {
    assert_eq!(
        issues_reading::<Notification>(r#"{ "type": { "kind": "email" } }"#),
        ["type: unknown_variant(expected: email, mail, push, muted)"]
    );
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct Inbox {
    #[validate(nested)]
    first: Notification,
    #[validate(nested)]
    second: Notification,
}

#[test]
fn reading_goes_on_after_a_tag_that_names_no_variant() {
    let body = r#"{ "first": { "type": "fax" }, "second": { "type": "email", "address": "ada" } }"#;

    assert_eq!(
        issues_reading::<Inbox>(body),
        [
            "first.type: unknown_variant(expected: email, mail, push, muted)",
            "second.address: email"
        ]
    );
}

#[test]
fn a_tag_given_twice_is_the_format_s_error() {
    assert_eq!(
        issues_reading::<Notification>(r#"{ "type": "email", "type": "push" }"#),
        ["the format's error: duplicate field `type` at line 1 column 25"]
    );
}

#[test]
fn without_collect_type_errors_a_value_that_isn_t_an_internally_tagged_enum_is_the_format_s_error() {
    assert_eq!(
        issues_reading::<Notification>("12"),
        [
            "the format's error: invalid type: integer `12`, expected internally tagged enum Notification at line 1 \
          column 2"
        ]
    );
}

#[test]
fn without_collect_type_errors_a_value_of_the_wrong_type_in_an_internally_tagged_variant_is_the_format_s_error() {
    assert_eq!(
        issues_reading::<Notification>(r#"{ "type": "email", "address": 12 }"#),
        ["the format's error: invalid type: integer `12`, expected a string"]
    );
}

#[derive(serde::Deserialize, Validate)]
#[allow(dead_code)]
struct DeviceThenValidated {
    #[validate(required)]
    token: String,
}

#[derive(serde::Deserialize, Validate)]
#[serde(tag = "type", rename_all = "camelCase")]
#[allow(dead_code)]
enum NotificationThenValidated {
    #[serde(alias = "mail")]
    Email {
        #[validate(email)]
        address: String,
    },
    Push(#[validate(nested)] DeviceThenValidated),
    Muted,
}

#[test]
fn a_complete_internally_tagged_body_is_reported_as_serde_then_validate_report_it() {
    let body = r#"{ "type": "push", "token": "" }"#;

    assert_eq!(
        issues_found_reading::<Notification>(body),
        issues_validating_after_serde::<NotificationThenValidated>(body)
    );
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
#[annotated(collect_type_errors)]
#[allow(dead_code)]
enum Alert {
    Email {
        #[validate(email)]
        address: String,
    },
}

#[test]
fn under_collect_type_errors_a_value_of_the_wrong_type_in_an_internally_tagged_variant_is_an_issue_at_its_path() {
    assert_eq!(
        issues_reading::<Alert>(r#"{ "type": "email", "address": 12 }"#),
        ["address: invalid_type(expected: a string, found: integer)"]
    );
}

#[test]
fn under_collect_type_errors_a_value_that_isn_t_an_internally_tagged_enum_is_an_issue_at_its_location() {
    assert_eq!(
        issues_reading::<Alert>("12"),
        ["invalid_type(expected: internally tagged enum Alert, found: integer)"]
    );
}

#[test]
fn under_collect_type_errors_an_internally_tagged_variant_is_read_from_a_sequence_led_by_its_tag() {
    assert_eq!(issues_reading::<Alert>(r#"["email", "ada"]"#), ["address: email"]);
}

#[test]
fn under_collect_type_errors_a_tag_given_twice_is_the_format_s_error() {
    assert_eq!(
        issues_reading::<Alert>(r#"{ "type": "email", "type": "email" }"#),
        ["the format's error: duplicate field `type`"]
    );
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
enum Event {
    SignedUp {
        #[validate(email)]
        email: String,
    },
    Renamed(#[validate(required)] String),
    Moved(#[validate(range(min = 1))] u32, #[validate(range(min = 1))] u32),
    Snoozed(Option<u32>),
    LoggedOut,
}

#[test]
fn an_adjacently_tagged_variant_is_read_from_its_tag_and_its_content() {
    assert_eq!(
        read::<Event>(r#"{ "type": "signedUp", "data": { "email": "ada@example.com" } }"#),
        Ok(Event::SignedUp {
            email: "ada@example.com".to_owned(),
        })
    );
}

#[test]
fn the_content_of_an_adjacently_tagged_variant_may_come_before_its_tag() {
    assert_eq!(
        read::<Event>(r#"{ "data": { "email": "ada@example.com" }, "type": "signedUp" }"#),
        Ok(Event::SignedUp {
            email: "ada@example.com".to_owned(),
        })
    );
}

#[test]
fn an_issue_in_an_adjacently_tagged_variant_is_under_the_content_s_name() {
    assert_eq!(
        issues_reading::<Event>(r#"{ "type": "signedUp", "data": { "email": "ada" } }"#),
        ["data.email: email"]
    );
}

#[test]
fn an_issue_in_content_read_before_its_tag_is_under_the_content_s_name() {
    assert_eq!(
        issues_reading::<Event>(r#"{ "data": { "email": "ada" }, "type": "signedUp" }"#),
        ["data.email: email"]
    );
}

#[test]
fn an_adjacently_tagged_newtype_variant_s_value_is_at_the_content_s_name() {
    assert_eq!(
        issues_reading::<Event>(r#"{ "type": "renamed", "data": "" }"#),
        ["data: required"]
    );
}

#[test]
fn an_adjacently_tagged_tuple_variant_s_elements_are_at_their_index_under_the_content_s_name() {
    assert_eq!(
        issues_reading::<Event>(r#"{ "type": "moved", "data": [1, 0] }"#),
        ["data[1]: range(min: 1)"]
    );
}

#[test]
fn an_adjacently_tagged_unit_variant_is_read_from_its_tag_alone() {
    assert_eq!(read::<Event>(r#"{ "type": "loggedOut" }"#), Ok(Event::LoggedOut));
}

#[test]
fn an_adjacently_tagged_unit_variant_is_read_with_null_content() {
    assert_eq!(
        read::<Event>(r#"{ "type": "loggedOut", "data": null }"#),
        Ok(Event::LoggedOut)
    );
}

#[test]
fn missing_content_is_missing_at_the_content_s_name() {
    assert_eq!(issues_reading::<Event>(r#"{ "type": "signedUp" }"#), ["data: missing"]);
}

#[test]
fn missing_content_of_an_optional_newtype_variant_is_none_as_with_serde() {
    assert_eq!(read::<Event>(r#"{ "type": "snoozed" }"#), Ok(Event::Snoozed(None)));
}

#[test]
fn a_missing_adjacent_tag_is_missing_at_the_tag_s_name() {
    assert_eq!(
        issues_reading::<Event>(r#"{ "data": { "email": "ada@example.com" } }"#),
        ["type: missing"]
    );
}

#[test]
fn an_adjacent_tag_that_names_no_variant_is_an_issue_at_the_tag() {
    assert_eq!(
        issues_reading::<Event>(r#"{ "type": "deleted", "data": {} }"#),
        ["type: unknown_variant(expected: signedUp, renamed, moved, snoozed, loggedOut)"]
    );
}

#[test]
fn an_adjacent_tag_after_its_content_that_names_no_variant_is_an_issue_at_the_tag() {
    assert_eq!(
        issues_reading::<Event>(r#"{ "data": {}, "type": "deleted" }"#),
        ["type: unknown_variant(expected: signedUp, renamed, moved, snoozed, loggedOut)"]
    );
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct Timeline {
    #[validate(nested)]
    first: Event,
    #[validate(nested)]
    second: Event,
}

#[test]
fn reading_goes_on_after_an_adjacent_tag_that_names_no_variant() {
    let body = r#"{
        "first": { "type": "deleted", "data": { "id": 7 } },
        "second": { "type": "renamed", "data": "" }
    }"#;

    assert_eq!(
        issues_reading::<Timeline>(body),
        [
            "first.type: unknown_variant(expected: signedUp, renamed, moved, snoozed, loggedOut)",
            "second.data: required"
        ]
    );
}

#[test]
fn keys_beside_the_tag_and_the_content_are_ignored() {
    assert_eq!(
        read::<Event>(r#"{ "type": "loggedOut", "at": "2026-11-12" }"#),
        Ok(Event::LoggedOut)
    );
}

#[test]
fn an_adjacent_tag_given_twice_is_the_format_s_error() {
    assert_eq!(
        issues_reading::<Event>(r#"{ "type": "loggedOut", "type": "loggedOut" }"#),
        ["the format's error: duplicate field `type` at line 1 column 29"]
    );
}

#[test]
fn content_given_twice_is_the_format_s_error() {
    assert_eq!(
        issues_reading::<Event>(r#"{ "type": "renamed", "data": "a", "data": "b" }"#),
        ["the format's error: duplicate field `data` at line 1 column 40"]
    );
}

#[test]
fn an_adjacently_tagged_variant_is_read_from_a_sequence_of_its_tag_and_its_content() {
    assert_eq!(
        issues_reading::<Event>(r#"["signedUp", { "email": "ada" }]"#),
        ["data.email: email"]
    );
}

#[test]
fn a_sequence_without_content_is_missing_it_at_the_content_s_name() {
    assert_eq!(issues_reading::<Event>(r#"["signedUp"]"#), ["data: missing"]);
}

#[test]
fn without_collect_type_errors_content_of_the_wrong_type_is_the_format_s_error() {
    assert_eq!(
        issues_reading::<Event>(r#"{ "type": "renamed", "data": 12 }"#),
        ["the format's error: invalid type: integer `12`, expected a string at line 1 column 31"]
    );
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", content = "data", rename_all = "camelCase", deny_unknown_fields)]
#[allow(dead_code)]
enum Command {
    Stop,
}

#[test]
fn an_adjacently_tagged_enum_denying_unknown_fields_denies_keys_beside_the_tag_and_the_content() {
    assert_eq!(
        issues_reading::<Command>(r#"{ "type": "stop", "at": "noon" }"#),
        ["the format's error: unknown field `at`, expected `type` or `data` at line 1 column 22"]
    );
}

#[derive(serde::Deserialize, Validate)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
#[allow(dead_code)]
enum EventThenValidated {
    SignedUp {
        #[validate(email)]
        email: String,
    },
    Renamed(#[validate(required)] String),
    Moved(#[validate(range(min = 1))] u32, #[validate(range(min = 1))] u32),
    Snoozed(Option<u32>),
    LoggedOut,
}

#[test]
fn a_complete_adjacently_tagged_body_is_reported_as_serde_then_validate_report_it() {
    let body = r#"{ "type": "moved", "data": [0, 0] }"#;

    assert_eq!(
        issues_found_reading::<Event>(body),
        issues_validating_after_serde::<EventThenValidated>(body)
    );
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
#[annotated(collect_type_errors)]
#[allow(dead_code)]
enum Change {
    SignedUp {
        #[validate(email)]
        email: String,
    },
    Renamed(#[validate(required)] String),
}

#[test]
fn under_collect_type_errors_content_of_the_wrong_type_is_an_issue_at_the_content_s_name() {
    assert_eq!(
        issues_reading::<Change>(r#"{ "type": "renamed", "data": 12 }"#),
        ["data: invalid_type(expected: a string, found: integer)"]
    );
}

#[test]
fn under_collect_type_errors_a_value_of_the_wrong_type_in_adjacent_content_is_an_issue_at_its_path() {
    assert_eq!(
        issues_reading::<Change>(r#"{ "type": "signedUp", "data": { "email": 12 } }"#),
        ["data.email: invalid_type(expected: a string, found: integer)"]
    );
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
#[annotated(collect_type_errors)]
#[allow(dead_code)]
enum Reminder {
    Text(#[validate(required)] String),
}

#[test]
fn under_collect_type_errors_an_internally_tagged_variant_its_entries_don_t_fit_is_an_issue_at_the_enum_s_location() {
    assert_eq!(
        issues_reading::<Reminder>(r#"{ "type": "text" }"#),
        ["invalid_type(expected: a string, found: map)"]
    );
}

#[test]
fn content_given_twice_after_a_tag_that_names_no_variant_is_the_format_s_error() {
    assert_eq!(
        issues_reading::<Event>(r#"{ "type": "deleted", "data": 1, "data": 2 }"#),
        ["the format's error: duplicate field `data` at line 1 column 38"]
    );
}

#[test]
fn a_sequence_whose_tag_names_no_variant_is_an_issue_at_the_tag() {
    assert_eq!(
        issues_reading::<Event>(r#"["deleted", {}]"#),
        ["type: unknown_variant(expected: signedUp, renamed, moved, snoozed, loggedOut)"]
    );
}

#[test]
fn an_empty_sequence_is_missing_its_tag() {
    assert_eq!(issues_reading::<Event>("[]"), ["type: missing"]);
}

#[test]
fn under_collect_type_errors_struct_variant_content_of_the_wrong_type_is_an_issue_at_the_content_s_name() {
    assert_eq!(
        issues_reading::<Change>(r#"{ "type": "signedUp", "data": 12 }"#),
        ["data: invalid_type(expected: struct variant Change::SignedUp, found: integer)"]
    );
}

#[test]
fn under_collect_type_errors_a_value_that_isn_t_an_adjacently_tagged_enum_is_an_issue_at_its_location() {
    assert_eq!(
        issues_reading::<Change>("12"),
        ["invalid_type(expected: adjacently tagged enum Change, found: integer)"]
    );
}

#[test]
fn an_adjacently_tagged_unit_variant_given_content_is_the_format_s_error_as_with_serde() {
    assert_eq!(
        issues_reading::<Event>(r#"{ "type": "loggedOut", "data": { "at": 1 } }"#),
        ["the format's error: invalid type: map, expected unit variant Event::LoggedOut at line 1 column 33"]
    );
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(untagged)]
enum Recipient {
    ByEmail {
        #[validate(email)]
        email: String,
    },
    ByPhone {
        #[validate(length(min = 9))]
        phone: String,
    },
    ById(#[validate(range(min = 1))] u64),
    Nobody,
}

#[test]
fn an_untagged_enum_is_read_as_the_first_variant_that_fits() {
    assert_eq!(
        read::<Recipient>(r#"{ "phone": "600000000" }"#),
        Ok(Recipient::ByPhone {
            phone: "600000000".to_owned(),
        })
    );
}

#[test]
fn an_issue_in_an_untagged_variant_is_at_the_enum_s_location() {
    assert_eq!(issues_reading::<Recipient>(r#"{ "email": "ada" }"#), ["email: email"]);
}

#[test]
fn a_variant_that_fits_with_broken_rules_is_chosen_over_a_later_one() {
    assert_eq!(
        issues_reading::<Recipient>(r#"{ "email": "ada", "phone": "600000000" }"#),
        ["email: email"]
    );
}

#[test]
fn an_untagged_newtype_variant_is_read_from_its_value() {
    assert_eq!(read::<Recipient>("7"), Ok(Recipient::ById(7)));
}

#[test]
fn an_untagged_newtype_variant_s_value_is_checked_at_the_enum_s_location() {
    assert_eq!(issues_reading::<Recipient>("0"), ["range(min: 1)"]);
}

#[test]
fn an_untagged_unit_variant_is_read_from_null() {
    assert_eq!(read::<Recipient>("null"), Ok(Recipient::Nobody));
}

#[test]
fn a_value_no_variant_fits_is_an_issue_at_the_enum_s_location_even_without_collect_type_errors() {
    assert_eq!(
        issues_reading::<Recipient>(r#"{ "fax": "+34 900 000 000" }"#),
        ["invalid_type(expected: untagged enum Recipient, found: map)"]
    );
}

#[derive(Deserialize, Debug)]
#[serde(untagged, expecting = "an email or a phone")]
#[allow(dead_code)]
enum Contact {
    ByEmail { email: String },
    ByPhone { phone: String },
}

#[test]
fn a_value_no_variant_fits_is_reported_as_the_enum_s_expecting_says() {
    assert_eq!(
        issues_reading::<Contact>(r#""ada""#),
        ["invalid_type(expected: an email or a phone, found: string)"]
    );
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct Delivery2 {
    #[validate(nested)]
    recipient: Recipient,
    #[validate(nested)]
    copy_to: Recipient,
}

#[test]
fn reading_goes_on_after_a_value_no_variant_fits() {
    let body = r#"{ "recipient": { "fax": "1" }, "copy_to": { "email": "ada" } }"#;

    assert_eq!(
        issues_reading::<Delivery2>(body),
        [
            "recipient: invalid_type(expected: untagged enum Recipient, found: map)",
            "copy_to.email: email"
        ]
    );
}

#[derive(serde::Deserialize, Validate)]
#[serde(untagged)]
#[allow(dead_code)]
enum RecipientThenValidated {
    ByEmail {
        #[validate(email)]
        email: String,
    },
    ByPhone {
        #[validate(length(min = 9))]
        phone: String,
    },
    ById(#[validate(range(min = 1))] u64),
    Nobody,
}

#[test]
fn a_complete_untagged_body_is_reported_as_serde_then_validate_report_it() {
    let body = r#"{ "phone": "600" }"#;

    assert_eq!(
        issues_found_reading::<Recipient>(body),
        issues_validating_after_serde::<RecipientThenValidated>(body)
    );
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
enum Quantity {
    Exact(#[validate(range(min = 1))] u32),
    #[serde(untagged)]
    Described(#[validate(required)] String),
}

#[test]
fn a_tagged_variant_of_an_enum_with_untagged_ones_is_read_by_its_tag() {
    assert_eq!(read::<Quantity>(r#"{ "exact": 3 }"#), Ok(Quantity::Exact(3)));
}

#[test]
fn an_untagged_variant_is_tried_when_no_tagged_variant_fits() {
    assert_eq!(
        read::<Quantity>(r#""a dozen""#),
        Ok(Quantity::Described("a dozen".to_owned()))
    );
}

#[test]
fn an_issue_in_an_untagged_variant_of_a_tagged_enum_is_at_the_enum_s_location() {
    assert_eq!(issues_reading::<Quantity>(r#""""#), ["required"]);
}

#[test]
fn a_value_no_variant_of_a_tagged_enum_with_untagged_ones_fits_is_reported_as_for_an_untagged_enum() {
    assert_eq!(
        issues_reading::<Quantity>("12"),
        ["invalid_type(expected: untagged enum Quantity, found: integer)"]
    );
}

#[test]
fn the_name_of_an_untagged_variant_names_no_variant_by_its_tag() {
    assert_eq!(
        issues_reading::<Quantity>(r#"{ "described": "a dozen" }"#),
        ["invalid_type(expected: untagged enum Quantity, found: map)"]
    );
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
enum Measure {
    Weight {
        #[validate(range(min = 1))]
        grams: u32,
    },
    #[serde(untagged)]
    Unknown {
        #[validate(required)]
        note: String,
    },
}

#[test]
fn an_internally_tagged_variant_of_an_enum_with_untagged_ones_is_read_by_its_tag() {
    assert_eq!(
        issues_reading::<Measure>(r#"{ "type": "weight", "grams": 0 }"#),
        ["grams: range(min: 1)"]
    );
}

#[test]
fn a_value_whose_tag_names_no_variant_is_tried_as_the_untagged_variants() {
    assert_eq!(
        issues_reading::<Measure>(r#"{ "type": "volume", "note": "" }"#),
        ["note: required"]
    );
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
enum Reading {
    Celsius(#[validate(range(min = -90))] i32),
    #[serde(untagged)]
    Raw(#[validate(required)] String),
}

#[test]
fn an_adjacently_tagged_variant_of_an_enum_with_untagged_ones_is_read_by_its_tag() {
    assert_eq!(
        issues_reading::<Reading>(r#"{ "type": "celsius", "data": -100 }"#),
        ["data: range(min: -90)"]
    );
}

#[test]
fn a_value_without_an_adjacent_tag_is_tried_as_the_untagged_variants() {
    assert_eq!(read::<Reading>(r#""22 C""#), Ok(Reading::Raw("22 C".to_owned())));
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
enum Sample {
    Weight {
        #[validate(range(min = 1))]
        grams: u32,
    },
    #[serde(untagged)]
    Nothing,
}

#[test]
fn an_untagged_unit_variant_of_an_internally_tagged_enum_is_read_from_null() {
    assert_eq!(read::<Sample>("null"), Ok(Sample::Nothing));
}

#[test]
fn an_untagged_unit_variant_of_an_internally_tagged_enum_isn_t_read_from_a_map() {
    assert_eq!(
        issues_reading::<Sample>(r#"{ "grams": 1 }"#),
        ["invalid_type(expected: untagged enum Sample, found: map)"]
    );
}

#[derive(serde::Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
enum Level {
    Low,
    High,
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(untagged)]
enum Setting {
    Full {
        #[validate(range(max = 10))]
        volume: u8,
        muted: bool,
        balance: f64,
        label: Option<String>,
        presets: Vec<u32>,
        names: BTreeMap<u8, String>,
        level: Level,
        initial: char,
    },
}

#[test]
fn an_untagged_variant_reads_every_kind_of_value_from_the_buffer() {
    let body = r#"{
        "volume": 3, "muted": false, "balance": -0.5, "label": null, "presets": [1, 2],
        "names": { "1": "one" }, "level": "high", "initial": "A"
    }"#;

    assert_eq!(
        read::<Setting>(body),
        Ok(Setting::Full {
            volume: 3,
            muted: false,
            balance: -0.5,
            label: None,
            presets: vec![1, 2],
            names: BTreeMap::from([(1, "one".to_owned())]),
            level: Level::High,
            initial: 'A',
        })
    );
}

#[derive(Debug, PartialEq)]
struct LegacyTerms;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
enum Plan {
    Free,
    Pro {
        #[validate(range(min = 1))]
        seats: u32,
    },
    #[serde(skip)]
    Legacy(LegacyTerms),
}

#[test]
fn a_skipped_variant_s_name_is_unknown() {
    assert_eq!(
        issues_reading::<Plan>(r#""legacy""#),
        ["the format's error: unknown variant `legacy`, expected `free` or `pro` at line 1 column 8"]
    );
}

#[test]
fn the_variants_after_a_skipped_one_are_read_by_their_names() {
    assert_eq!(
        issues_reading::<Plan>(r#"{ "pro": { "seats": 0 } }"#),
        ["pro.seats: range(min: 1)"]
    );
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
#[allow(dead_code)]
enum Tier {
    Basic,
    #[serde(skip_deserializing)]
    Legacy,
}

#[test]
fn a_tag_naming_a_skipped_variant_names_no_variant() {
    assert_eq!(
        issues_reading::<Tier>(r#"{ "type": "legacy" }"#),
        ["type: unknown_variant(expected: basic)"]
    );
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(untagged)]
enum Amount {
    #[serde(skip)]
    #[allow(dead_code)]
    Cents(u32),
    Text(String),
}

#[test]
fn a_skipped_untagged_variant_is_never_tried() {
    assert_eq!(
        issues_reading::<Amount>("12"),
        ["invalid_type(expected: untagged enum Amount, found: integer)"]
    );
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
enum Status {
    Active,
    #[serde(other)]
    Unknown,
}

#[test]
fn a_name_that_is_no_other_variant_reads_as_the_other_variant() {
    assert_eq!(read::<Status>(r#""archived""#), Ok(Status::Unknown));
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
enum Attachment {
    Image {
        #[validate(url)]
        url: String,
    },
    #[serde(other)]
    Unsupported,
}

#[test]
fn a_tag_that_names_no_other_variant_reads_as_the_other_variant() {
    assert_eq!(
        read::<Attachment>(r#"{ "type": "video", "url": "https://example.com/v.mp4" }"#),
        Ok(Attachment::Unsupported)
    );
}

fn rgb<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<(u8, u8, u8), D::Error> {
    let text = <String as serde::Deserialize>::deserialize(deserializer)?;
    let mut parts = text.split(',').map(|part| part.trim().parse::<u8>());
    match (parts.next(), parts.next(), parts.next()) {
        (Some(Ok(red)), Some(Ok(green)), Some(Ok(blue))) => Ok((red, green, blue)),
        _ => Err(serde::de::Error::custom("not a colour")),
    }
}

fn coordinates<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<(f64, f64), D::Error> {
    let [latitude, longitude] = <[f64; 2] as serde::Deserialize>::deserialize(deserializer)?;
    Ok((latitude, longitude))
}

fn street<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Address, D::Error> {
    let street = <String as serde::Deserialize>::deserialize(deserializer)?;
    Ok(Address { street })
}

fn anything<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<(), D::Error> {
    <serde::de::IgnoredAny as serde::Deserialize>::deserialize(deserializer)?;
    Ok(())
}

mod hex_colour {
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<u32, D::Error> {
        let text = <String as serde::Deserialize>::deserialize(deserializer)?;
        u32::from_str_radix(text.trim_start_matches('#'), 16).map_err(serde::de::Error::custom)
    }
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
enum Colour {
    #[serde(deserialize_with = "rgb")]
    Rgb(#[validate(range(max = 200))] u8, u8, u8),
    #[serde(deserialize_with = "coordinates")]
    Spot {
        #[validate(range(min = -90.0, max = 90.0))]
        latitude: f64,
        longitude: f64,
    },
    #[serde(deserialize_with = "street")]
    Mural(#[validate(nested)] Address),
    #[serde(with = "hex_colour")]
    Hex(#[validate(range(max = 0xFF_FF_FF))] u32),
    #[serde(deserialize_with = "anything")]
    Unknown,
}

#[test]
fn a_variant_read_by_a_function_is_built_from_what_it_returns() {
    assert_eq!(
        read::<Colour>(r#"{ "rgb": "10, 20, 30" }"#),
        Ok(Colour::Rgb(10, 20, 30))
    );
}

#[test]
fn a_variant_read_by_a_function_is_checked_by_its_fields_rules() {
    assert_eq!(
        issues_reading::<Colour>(r#"{ "rgb": "250, 0, 0" }"#),
        ["rgb[0]: range(max: 200)"]
    );
}

#[test]
fn a_struct_variant_read_by_a_function_is_checked_at_its_fields_names() {
    assert_eq!(
        issues_reading::<Colour>(r#"{ "spot": [91.5, 2.1] }"#),
        ["spot.latitude: range(min: -90, max: 90)"]
    );
}

#[test]
fn a_nested_value_in_a_variant_read_by_a_function_is_validated_at_its_path() {
    assert_eq!(
        issues_reading::<Colour>(r#"{ "mural": "" }"#),
        ["mural.street: required"]
    );
}

#[test]
fn a_variant_read_with_a_module_is_read_by_its_deserialize_function() {
    assert_eq!(read::<Colour>(r##"{ "hex": "#00ff00" }"##), Ok(Colour::Hex(0x00_FF_00)));
}

#[test]
fn a_unit_variant_read_by_a_function_is_built_once_the_function_reads_its_content() {
    assert_eq!(read::<Colour>(r#"{ "unknown": [1, 2] }"#), Ok(Colour::Unknown));
}

#[test]
fn the_error_of_a_function_reading_a_variant_ends_deserialization() {
    assert_eq!(
        issues_reading::<Colour>(r#"{ "rgb": "red" }"#),
        ["the format's error: not a colour"]
    );
}

#[derive(Deserialize, Debug)]
enum Note<'a> {
    #[serde(borrow)]
    Text(Cow<'a, str>),
}

#[test]
fn a_newtype_variant_with_borrow_borrows_from_the_input() {
    let note = deserialize::<Note>(r#"{ "Text": "Hola" }"#).unwrap();

    let Note::Text(Cow::Borrowed(text)) = note else {
        panic!("expected a borrowed text, got {note:?}");
    };
    assert_eq!(text, "Hola");
}

#[derive(Deserialize, Debug, PartialEq)]
#[allow(dead_code)]
enum Held<T> {
    Value(#[validate(range(min = 1))] u32),
    #[serde(skip)]
    Kept(T),
}

#[test]
fn a_skipped_variant_s_type_parameter_needs_no_deserialize() {
    assert_eq!(read::<Held<LegacyTerms>>(r#"{ "Value": 3 }"#), Ok(Held::Value(3)));
}

#[test]
fn a_tag_given_as_an_index_beyond_the_variants_reads_as_the_other_variant() {
    assert_eq!(read::<Attachment>(r#"{ "type": 7 }"#), Ok(Attachment::Unsupported));
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
enum Envelope<T> {
    Data {
        #[validate(nested)]
        payload: T,
    },
    Empty,
}

#[test]
fn an_internally_tagged_enum_with_a_type_parameter_reads_the_value_it_holds() {
    assert_eq!(
        issues_reading::<Envelope<Address>>(r#"{ "type": "data", "payload": { "street": "" } }"#),
        ["payload.street: required"]
    );
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
enum Message<'a, T> {
    Text(#[validate(required)] &'a str),
    Value(#[validate(nested)] T),
}

#[test]
fn an_adjacently_tagged_enum_with_a_lifetime_and_a_type_parameter_reads_with_both() {
    assert_eq!(
        issues_reading::<Message<Address>>(r#"{ "data": { "street": "" }, "type": "value" }"#),
        ["data.street: required"]
    );
}

#[test]
fn an_adjacently_tagged_enum_with_a_lifetime_borrows_from_the_input() {
    assert_eq!(
        read::<Message<Address>>(r#"{ "type": "text", "data": "Hola" }"#),
        Ok(Message::Text("Hola"))
    );
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(untagged)]
enum Either<L, R> {
    Left(L),
    Right(R),
}

#[test]
fn an_untagged_enum_with_type_parameters_reads_the_first_that_fits() {
    assert_eq!(
        read::<Either<u8, String>>(r#""ocho""#),
        Ok(Either::Right("ocho".to_owned()))
    );
}
