//! `#[derive(Deserialize)]` reads an enum as serde does, and collects every missing field and broken rule in the
//! variant it reads, each at the path where its value is in the input.

#![cfg(feature = "serde")]

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
