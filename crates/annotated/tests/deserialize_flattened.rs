//! `#[derive(Deserialize)]` reads a field with `#[serde(flatten)]` from the keys the struct doesn't know, as serde does,
//! and reports the issues inside it where they are in the input: at the struct's own location.

#![cfg(feature = "serde")]

use std::collections::BTreeMap;

use annotated::{Deserialize, DeserializeError, Issue, Problem, Validate};

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
    issues_found_reading::<T>(json).iter().map(described).collect()
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

fn format_error_reading<'de, T: Deserialize<'de>>(json: &'de str) -> String {
    match deserialize::<T>(json) {
        Err(error @ DeserializeError::Format(_)) => error.to_string(),
        Ok(_) => panic!("expected the format's error, got a value"),
        Err(error) => panic!("expected the format's error, got {error:?}"),
    }
}

fn issues_validating_after_serde<T: serde::de::DeserializeOwned + Validate>(json: &str) -> Vec<Issue> {
    let value = serde_json::from_str::<T>(json).unwrap();
    value
        .validate()
        .map_or_else(|issues| issues.into_iter().collect(), |()| Vec::new())
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Address {
    #[validate(required)]
    street: String,
    #[validate(length(min = 5))]
    postal_code: String,
}

#[derive(Deserialize, Debug, PartialEq)]
struct Order {
    #[validate(required)]
    reference: String,
    #[serde(flatten)]
    #[validate(nested)]
    address: Address,
    #[validate(range(min = 1))]
    parcels: u8,
}

#[test]
fn a_flattened_struct_takes_the_keys_it_knows_from_the_struct_s_map() {
    let body = r#"{ "reference": "A1", "street": "Gran Vía 1", "postalCode": "28013", "parcels": 2 }"#;

    let order = deserialize::<Order>(body).unwrap();

    assert_eq!(
        order,
        Order {
            reference: "A1".to_owned(),
            address: Address {
                street: "Gran Vía 1".to_owned(),
                postal_code: "28013".to_owned(),
            },
            parcels: 2,
        }
    );
}

#[test]
fn the_issues_inside_a_flattened_struct_are_at_the_struct_s_location_in_their_place_among_the_fields() {
    let body = r#"{ "reference": "", "street": "", "postalCode": "1", "parcels": 0 }"#;

    assert_eq!(
        issues_reading::<Order>(body),
        [
            "reference: required",
            "street: required",
            "postalCode: length(min: 5, unit: chars)",
            "parcels: range(min: 1)"
        ]
    );
}

#[test]
fn a_field_missing_inside_a_flattened_struct_is_missing_at_its_name() {
    let body = r#"{ "reference": "A1", "street": "Gran Vía 1", "parcels": 2 }"#;

    assert_eq!(issues_reading::<Order>(body), ["postalCode: missing"]);
}

#[test]
fn a_struct_with_a_flattened_field_can_t_be_read_from_a_sequence() {
    assert_eq!(
        format_error_reading::<Order>(r#"["A1", "Gran Vía 1", "28013", 2]"#),
        "invalid type: sequence, expected struct Order at line 1 column 0"
    );
}

#[derive(Deserialize, Debug, PartialEq)]
struct Product {
    #[validate(required)]
    name: String,
    #[serde(flatten)]
    #[validate(length(max = 2), each(length(max = 5)))]
    attributes: BTreeMap<String, String>,
}

#[test]
fn a_flattened_map_takes_every_key_no_other_field_took() {
    let product = deserialize::<Product>(r#"{ "color": "red", "name": "Scarf", "size": "M" }"#).unwrap();

    assert_eq!(
        product,
        Product {
            name: "Scarf".to_owned(),
            attributes: BTreeMap::from([
                ("color".to_owned(), "red".to_owned()),
                ("size".to_owned(), "M".to_owned()),
            ]),
        }
    );
}

#[test]
fn a_rule_on_a_flattened_map_reports_at_the_struct_s_location() {
    let body = r#"{ "name": "Scarf", "color": "red", "size": "M", "fabric": "wool" }"#;

    assert_eq!(issues_reading::<Product>(body), ["length(max: 2, unit: elements)"]);
}

#[test]
fn a_rule_under_each_on_a_flattened_map_reports_at_each_key() {
    let body = r#"{ "name": "Scarf", "color": "crimson" }"#;

    assert_eq!(
        issues_reading::<Product>(body),
        ["[color]: length(max: 5, unit: chars)"]
    );
}

#[derive(Deserialize, Debug, PartialEq)]
struct Shipment {
    #[serde(flatten)]
    notes: BTreeMap<String, String>,
    #[serde(flatten)]
    address: Address,
}

#[test]
fn a_flattened_map_leaves_the_keys_it_reads_to_the_flattened_fields_after_it_as_with_serde() {
    let body = r#"{ "street": "Sol 1", "postalCode": "28013", "gate": "B" }"#;

    let shipment = deserialize::<Shipment>(body).unwrap();

    assert_eq!(
        shipment,
        Shipment {
            notes: BTreeMap::from([
                ("gate".to_owned(), "B".to_owned()),
                ("postalCode".to_owned(), "28013".to_owned()),
                ("street".to_owned(), "Sol 1".to_owned()),
            ]),
            address: Address {
                street: "Sol 1".to_owned(),
                postal_code: "28013".to_owned(),
            },
        }
    );
}

#[derive(Deserialize, Debug, PartialEq)]
struct Invoice {
    #[validate(required)]
    number: String,
    #[serde(flatten)]
    #[validate(nested)]
    billing_address: Option<Address>,
}

#[test]
fn a_flattened_option_is_none_without_issues_when_its_value_can_t_be_built() {
    let invoice = deserialize::<Invoice>(r#"{ "number": "F-1", "street": "" }"#).unwrap();

    assert_eq!(
        invoice,
        Invoice {
            number: "F-1".to_owned(),
            billing_address: None,
        }
    );
}

#[test]
fn a_flattened_option_whose_value_can_be_built_is_checked_by_its_rules() {
    let body = r#"{ "number": "F-1", "street": "", "postalCode": "28013" }"#;

    assert_eq!(issues_reading::<Invoice>(body), ["street: required"]);
}

#[derive(serde::Deserialize, Debug, PartialEq)]
struct Dimensions {
    width: u32,
    height: u32,
}

#[derive(Deserialize, Debug, PartialEq)]
#[annotated(collect_type_errors)]
struct Poster {
    #[validate(required)]
    title: String,
    #[serde(flatten)]
    size: Dimensions,
}

#[test]
fn under_collect_type_errors_a_flattened_value_whose_keys_don_t_fit_its_type_is_an_issue_at_the_struct_s_location() {
    let body = r#"{ "title": "", "width": "wide", "height": 90 }"#;

    assert_eq!(
        issues_reading::<Poster>(body),
        ["title: required", "invalid_type(expected: u32, found: string)"]
    );
}

#[test]
fn under_collect_type_errors_a_flattened_value_whose_keys_fit_its_type_is_read() {
    let poster = deserialize::<Poster>(r#"{ "title": "Sea", "width": 60, "height": 90 }"#).unwrap();

    assert_eq!(
        poster,
        Poster {
            title: "Sea".to_owned(),
            size: Dimensions { width: 60, height: 90 },
        }
    );
}

#[test]
fn without_collect_type_errors_a_flattened_value_whose_keys_don_t_fit_its_type_is_the_format_s_error() {
    #[derive(Deserialize, Debug)]
    struct Frame {
        #[serde(flatten)]
        #[allow(dead_code)]
        size: Dimensions,
    }

    assert_eq!(
        format_error_reading::<Frame>(r#"{ "width": "wide", "height": 90 }"#),
        "invalid type: string \"wide\", expected u32 at line 1 column 33"
    );
}

#[derive(Deserialize, Debug, PartialEq)]
#[annotated(collect_type_errors)]
struct Measures {
    #[validate(range(min = 1))]
    width: u32,
    #[validate(range(min = 1))]
    height: u32,
}

#[derive(Deserialize, Debug)]
struct Canvas {
    #[serde(flatten)]
    #[validate(nested)]
    #[allow(dead_code)]
    measures: Measures,
}

#[test]
fn a_nested_flattened_struct_follows_its_own_collect_type_errors_option() {
    assert_eq!(
        issues_reading::<Canvas>(r#"{ "width": "wide", "height": 0 }"#),
        [
            "width: invalid_type(expected: u32, found: string)",
            "height: range(min: 1)"
        ]
    );
}

#[derive(serde::Deserialize, Debug, PartialEq)]
enum Payment {
    Card { number: String },
    Transfer { iban: String },
}

#[derive(Deserialize, Debug, PartialEq)]
struct Checkout {
    #[validate(required)]
    email: String,
    #[serde(flatten)]
    payment: Payment,
}

#[test]
fn a_flattened_enum_takes_the_key_that_names_its_variant() {
    let body = r#"{ "email": "ada@example.com", "Transfer": { "iban": "ES91" } }"#;

    let checkout = deserialize::<Checkout>(body).unwrap();

    assert_eq!(
        checkout,
        Checkout {
            email: "ada@example.com".to_owned(),
            payment: Payment::Transfer {
                iban: "ES91".to_owned()
            },
        }
    );
}

#[derive(Deserialize, Debug, PartialEq)]
struct Envelope<T> {
    #[validate(required)]
    recipient: String,
    #[serde(flatten)]
    content: T,
}

#[test]
fn a_flattened_field_whose_type_is_a_type_parameter_is_read_with_the_type_given() {
    let envelope = deserialize::<Envelope<Dimensions>>(r#"{ "recipient": "Ada", "width": 1, "height": 2 }"#).unwrap();

    assert_eq!(
        envelope,
        Envelope {
            recipient: "Ada".to_owned(),
            content: Dimensions { width: 1, height: 2 },
        }
    );
}

fn tags_from_the_rest<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    let rest = <BTreeMap<String, bool> as serde::Deserialize>::deserialize(deserializer)?;
    Ok(rest.into_keys().collect())
}

#[derive(Deserialize, Debug, PartialEq)]
struct Tagged {
    name: String,
    #[serde(flatten, deserialize_with = "tags_from_the_rest")]
    #[validate(length(max = 1))]
    tags: Vec<String>,
}

#[test]
fn a_flattened_field_is_read_by_its_deserialize_with_function() {
    let tagged = deserialize::<Tagged>(r#"{ "name": "Scarf", "new": true }"#).unwrap();

    assert_eq!(
        tagged,
        Tagged {
            name: "Scarf".to_owned(),
            tags: vec!["new".to_owned()],
        }
    );
}

#[derive(Deserialize, Debug, PartialEq, Default)]
#[serde(default)]
struct Search {
    query: String,
    #[serde(flatten)]
    filters: BTreeMap<String, String>,
}

#[test]
fn a_struct_s_default_leaves_a_flattened_field_to_the_keys_it_reads() {
    let search = deserialize::<Search>(r#"{ "color": "red" }"#).unwrap();

    assert_eq!(
        search,
        Search {
            query: String::new(),
            filters: BTreeMap::from([("color".to_owned(), "red".to_owned())]),
        }
    );
}

fn not_a_restricted_area(address: &AddressThenValidated) -> Result<(), Problem> {
    if address.street.starts_with("Calle Falsa") {
        return Err(Problem::custom("restricted_area", "is in a restricted area"));
    }
    Ok(())
}

fn not_restricted(address: &Address) -> Result<(), Problem> {
    if address.street.starts_with("Calle Falsa") {
        return Err(Problem::custom("restricted_area", "is in a restricted area"));
    }
    Ok(())
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct Delivery {
    #[validate(required)]
    reference: String,
    #[serde(flatten)]
    #[validate(custom(not_restricted), nested)]
    address: Address,
    #[serde(flatten)]
    #[validate(each(length(max = 3)))]
    notes: BTreeMap<String, String>,
}

#[derive(serde::Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
struct AddressThenValidated {
    #[validate(required)]
    street: String,
    #[validate(length(min = 5))]
    postal_code: String,
}

#[derive(serde::Deserialize, Validate)]
#[allow(dead_code)]
struct DeliveryThenValidated {
    #[validate(required)]
    reference: String,
    #[serde(flatten)]
    #[validate(custom(not_a_restricted_area), nested)]
    address: AddressThenValidated,
    #[serde(flatten)]
    #[validate(each(length(max = 3)))]
    notes: BTreeMap<String, String>,
}

#[test]
fn a_complete_body_with_flattened_fields_is_reported_as_serde_then_validate_report_it() {
    let body = r#"{ "reference": "", "street": "", "postalCode": "1", "gate": "Back" }"#;

    assert_eq!(
        issues_found_reading::<Delivery>(body),
        issues_validating_after_serde::<DeliveryThenValidated>(body)
    );
}

#[derive(Deserialize, Debug, PartialEq)]
struct Listing {
    #[serde(flatten)]
    address: Address,
    #[serde(flatten)]
    extras: BTreeMap<String, String>,
}

#[test]
fn a_flattened_struct_takes_its_keys_from_the_flattened_fields_after_it() {
    let body = r#"{ "street": "Sol 1", "postalCode": "28013", "gate": "B" }"#;

    let listing = deserialize::<Listing>(body).unwrap();

    assert_eq!(
        listing,
        Listing {
            address: Address {
                street: "Sol 1".to_owned(),
                postal_code: "28013".to_owned(),
            },
            extras: BTreeMap::from([("gate".to_owned(), "B".to_owned())]),
        }
    );
}

#[test]
fn a_key_given_as_bytes_is_read_as_the_field_it_names() {
    let order = Order {
        reference: "A1".to_owned(),
        address: Address {
            street: "Sol 1".to_owned(),
            postal_code: "28013".to_owned(),
        },
        parcels: 2,
    };

    serde_test::assert_de_tokens(
        &order,
        &[
            serde_test::Token::Map { len: Some(4) },
            serde_test::Token::Bytes(b"reference"),
            serde_test::Token::Str("A1"),
            serde_test::Token::ByteBuf(b"street"),
            serde_test::Token::Str("Sol 1"),
            serde_test::Token::Bytes(b"postalCode"),
            serde_test::Token::Str("28013"),
            serde_test::Token::ByteBuf(b"parcels"),
            serde_test::Token::U8(2),
            serde_test::Token::MapEnd,
        ],
    );
}

#[derive(serde::Deserialize, Debug, PartialEq)]
struct Extras(BTreeMap<String, String>);

#[derive(Deserialize, Debug, PartialEq)]
struct Tagline {
    text: String,
    #[serde(flatten)]
    extras: Extras,
}

#[test]
fn a_flattened_newtype_reads_what_it_wraps() {
    let tagline = deserialize::<Tagline>(r#"{ "text": "Hola", "tone": "warm" }"#).unwrap();

    assert_eq!(
        tagline,
        Tagline {
            text: "Hola".to_owned(),
            extras: Extras(BTreeMap::from([("tone".to_owned(), "warm".to_owned())])),
        }
    );
}

#[derive(Deserialize, Debug)]
struct Counter {
    #[serde(flatten)]
    #[allow(dead_code)]
    count: u32,
}

#[test]
fn a_flattened_value_that_is_neither_a_struct_nor_a_map_is_the_format_s_error() {
    assert_eq!(
        format_error_reading::<Counter>(r#"{ "count": 3 }"#),
        "can only flatten structs and maps at line 1 column 14"
    );
}

#[test]
fn a_flattened_enum_without_a_key_naming_one_of_its_variants_is_the_format_s_error() {
    assert_eq!(
        format_error_reading::<Checkout>(r#"{ "email": "ada@example.com", "Cash": {} }"#),
        "no variant of enum Payment found in flattened data at line 1 column 42"
    );
}

type MaybeDimensions = Option<Dimensions>;

#[derive(Deserialize, Debug, PartialEq)]
struct Photo {
    #[serde(flatten)]
    size: MaybeDimensions,
}

#[test]
fn a_flattened_option_written_through_an_alias_is_some_of_what_it_reads() {
    let photo = deserialize::<Photo>(r#"{ "width": 4, "height": 3 }"#).unwrap();

    assert_eq!(
        photo,
        Photo {
            size: Some(Dimensions { width: 4, height: 3 })
        }
    );
}

#[derive(Deserialize, Debug, PartialEq)]
struct Marked<T> {
    name: String,
    #[serde(flatten)]
    marker: std::marker::PhantomData<T>,
}

#[test]
fn a_flattened_unit_struct_reads_nothing() {
    let marked = deserialize::<Marked<u8>>(r#"{ "name": "Ada", "other": 1 }"#).unwrap();

    assert_eq!(
        marked,
        Marked {
            name: "Ada".to_owned(),
            marker: std::marker::PhantomData,
        }
    );
}
