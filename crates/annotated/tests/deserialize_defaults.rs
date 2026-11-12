//! `#[derive(Deserialize)]` gives an absent field the value `#[serde(default)]` names, and checks that value by the
//! field's rules like any other, so the issues are the ones serde followed by `validate()` finds.

#![cfg(feature = "serde")]

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
    issues_found_reading::<T>(json).iter().map(path_and_code).collect()
}

fn path_and_code(issue: &Issue) -> String {
    let code = issue.problem().code();
    if issue.path().is_root() {
        return code.to_owned();
    }
    format!("{}: {code}", issue.path())
}

fn issues_validating_after_serde<T: serde::de::DeserializeOwned + Validate>(json: &str) -> Vec<Issue> {
    let value = serde_json::from_str::<T>(json).unwrap();
    value
        .validate()
        .map_or_else(|issues| issues.into_iter().collect(), |()| Vec::new())
}

fn one_night() -> u8 {
    1
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct RoomBooking {
    #[validate(required)]
    guest: String,
    #[serde(default)]
    #[validate(range(max = 4))]
    extra_beds: u8,
    #[serde(default = "one_night")]
    #[validate(range(min = 1))]
    nights: u8,
}

#[test]
fn an_absent_field_with_a_default_takes_its_type_s_default() {
    let booking = deserialize::<RoomBooking>(r#"{ "guest": "Ada", "nights": 3 }"#).unwrap();

    assert_eq!(
        booking,
        RoomBooking {
            guest: "Ada".to_owned(),
            extra_beds: 0,
            nights: 3,
        }
    );
}

#[test]
fn an_absent_field_with_a_default_function_takes_what_the_function_returns() {
    let booking = deserialize::<RoomBooking>(r#"{ "guest": "Ada", "extraBeds": 2 }"#).unwrap();

    assert_eq!(
        booking,
        RoomBooking {
            guest: "Ada".to_owned(),
            extra_beds: 2,
            nights: 1,
        }
    );
}

#[test]
fn a_present_field_with_a_default_is_read_from_the_input() {
    let booking = deserialize::<RoomBooking>(r#"{ "guest": "Ada", "extraBeds": 2, "nights": 3 }"#).unwrap();

    assert_eq!(
        booking,
        RoomBooking {
            guest: "Ada".to_owned(),
            extra_beds: 2,
            nights: 3,
        }
    );
}

#[test]
fn an_element_missing_at_the_end_of_a_sequence_takes_its_default() {
    let booking = deserialize::<RoomBooking>(r#"["Ada"]"#).unwrap();

    assert_eq!(
        booking,
        RoomBooking {
            guest: "Ada".to_owned(),
            extra_beds: 0,
            nights: 1,
        }
    );
}

#[derive(Deserialize, Debug)]
struct Stay {
    #[serde(default)]
    #[validate(range(min = 1))]
    #[allow(dead_code)]
    nights: u8,
}

#[test]
fn a_default_is_checked_by_the_field_s_rules() {
    assert_eq!(issues_reading::<Stay>("{}"), ["nights: range"]);
}

fn no_default_expected() -> String {
    panic!("a default was made for a field present in the input")
}

#[derive(Deserialize, Debug, PartialEq)]
struct Greeting {
    #[serde(default = "no_default_expected")]
    text: String,
}

#[test]
fn a_default_is_made_only_when_the_field_is_absent() {
    let greeting = deserialize::<Greeting>(r#"{ "text": "Hola" }"#).unwrap();

    assert_eq!(
        greeting,
        Greeting {
            text: "Hola".to_owned()
        }
    );
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "serde calls a default function for an `Option` field"
)]
fn spanish() -> Option<String> {
    Some("es".to_owned())
}

#[derive(Deserialize, Debug, PartialEq)]
struct Preferences {
    #[serde(default = "spanish")]
    #[validate(length(min = 2))]
    language: Option<String>,
}

#[test]
fn an_absent_optional_field_with_a_default_takes_the_default_instead_of_none() {
    let preferences = deserialize::<Preferences>("{}").unwrap();

    assert_eq!(
        preferences,
        Preferences {
            language: Some("es".to_owned())
        }
    );
}

#[test]
fn an_optional_field_with_a_default_given_as_null_is_none() {
    let preferences = deserialize::<Preferences>(r#"{ "language": null }"#).unwrap();

    assert_eq!(preferences, Preferences { language: None });
}

#[derive(Deserialize, Validate, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
struct Address {
    #[validate(required)]
    street: String,
    #[validate(length(min = 5))]
    postal_code: String,
}

fn gran_via_and_an_unnamed_stop() -> Vec<Address> {
    vec![
        Address {
            street: "Gran Vía 1".to_owned(),
            postal_code: "28013".to_owned(),
        },
        Address {
            street: String::new(),
            postal_code: "28013".to_owned(),
        },
    ]
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Shipment {
    #[serde(default)]
    #[validate(nested)]
    #[allow(dead_code)]
    return_address: Address,
    #[serde(default = "gran_via_and_an_unnamed_stop")]
    #[validate(each(nested))]
    #[allow(dead_code)]
    stops: Vec<Address>,
}

#[test]
fn an_absent_nested_field_with_a_default_is_validated_at_its_name() {
    assert_eq!(
        issues_reading::<Shipment>(r#"{ "stops": [] }"#),
        ["returnAddress.street: required", "returnAddress.postalCode: length"]
    );
}

#[test]
fn every_element_of_an_absent_list_with_a_default_is_validated_at_its_index() {
    let body = r#"{ "returnAddress": { "street": "Atocha 2", "postalCode": "28012" } }"#;

    assert_eq!(issues_reading::<Shipment>(body), ["stops[1].street: required"]);
}

#[test]
fn a_present_nested_field_with_a_default_is_read_with_the_issues_inside_it() {
    let body = r#"{ "returnAddress": { "postalCode": "28012" }, "stops": [] }"#;

    assert_eq!(issues_reading::<Shipment>(body), ["returnAddress.street: missing"]);
}

#[derive(Deserialize, Debug, PartialEq)]
#[annotated(collect_type_errors)]
struct RoomCollectingTypeErrors {
    #[validate(required)]
    guest: String,
    #[serde(default = "one_night")]
    nights: u8,
}

#[test]
fn a_struct_collecting_type_errors_gives_an_absent_field_its_default() {
    let room = deserialize::<RoomCollectingTypeErrors>(r#"{ "guest": "Ada" }"#).unwrap();

    assert_eq!(
        room,
        RoomCollectingTypeErrors {
            guest: "Ada".to_owned(),
            nights: 1,
        }
    );
}

#[test]
fn a_present_value_of_the_wrong_type_is_reported_instead_of_taking_the_default() {
    let body = r#"{ "guest": "Ada", "nights": "two" }"#;

    assert_eq!(
        issues_reading::<RoomCollectingTypeErrors>(body),
        ["nights: invalid_type"]
    );
}

fn leaves_after_arriving(visit: &Visit) -> Result<(), Problem> {
    if visit.leaves_on <= visit.arrives_on {
        return Err(Problem::custom("leaves_before_arriving", "must be after the arrival"));
    }
    Ok(())
}

#[derive(Deserialize, Debug)]
#[validate(custom(leaves_after_arriving, field = leaves_on))]
struct Visit {
    arrives_on: u32,
    #[serde(default)]
    #[validate(range(max = 365))]
    leaves_on: u32,
}

#[test]
fn a_struct_level_rule_at_a_defaulted_field_checks_its_default() {
    assert_eq!(
        issues_reading::<Visit>(r#"{ "arrives_on": 5 }"#),
        ["leaves_on: leaves_before_arriving"]
    );
}

#[derive(Deserialize, Debug, PartialEq)]
struct Labelled<T> {
    #[serde(default)]
    label: T,
    #[validate(required)]
    name: String,
}

#[test]
fn a_defaulted_field_whose_type_is_a_type_parameter_takes_the_type_s_default() {
    let labelled = deserialize::<Labelled<u8>>(r#"{ "name": "Ada" }"#).unwrap();

    assert_eq!(
        labelled,
        Labelled {
            label: 0,
            name: "Ada".to_owned(),
        }
    );
}

#[derive(Deserialize, Debug)]
struct Envelope<T> {
    #[serde(default)]
    #[validate(nested)]
    #[allow(dead_code)]
    content: T,
}

#[test]
fn a_defaulted_nested_field_whose_type_is_a_type_parameter_is_validated_at_its_name() {
    assert_eq!(
        issues_reading::<Envelope<Address>>("{}"),
        ["content.street: required", "content.postalCode: length"]
    );
}

fn not_a_restricted_stop(address: &Address) -> Result<(), Problem> {
    on_the_map(&address.street)
}

fn not_a_restricted_stop_once_validated(address: &AddressThenValidated) -> Result<(), Problem> {
    on_the_map(&address.street)
}

fn on_the_map(street: &str) -> Result<(), Problem> {
    if street.starts_with("Calle Falsa") {
        return Err(Problem::custom("not_on_the_map", "isn't on the map"));
    }
    Ok(())
}

fn an_unnamed_address() -> Address {
    Address::default()
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "serde calls a default function for an `Option` field"
)]
fn maybe_an_unnamed_address() -> Option<Address> {
    Some(Address::default())
}

fn a_restricted_stop_then_an_unnamed_one() -> Vec<Address> {
    vec![
        Address {
            street: "Calle Falsa 123".to_owned(),
            postal_code: "1".to_owned(),
        },
        Address::default(),
    ]
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct Delivery {
    #[validate(required)]
    reference: String,
    #[serde(default = "an_unnamed_address")]
    #[validate(nested)]
    sender: Address,
    #[serde(default = "maybe_an_unnamed_address")]
    #[validate(required, nested)]
    recipient: Option<Address>,
    #[serde(default = "a_restricted_stop_then_an_unnamed_one")]
    #[validate(each(custom(not_a_restricted_stop), nested))]
    stops: Vec<Address>,
    #[serde(default)]
    #[validate(range(min = 1))]
    parcels: u8,
}

#[derive(serde::Deserialize, Validate, Default)]
#[serde(rename_all = "camelCase")]
struct AddressThenValidated {
    #[validate(required)]
    street: String,
    #[validate(length(min = 5))]
    postal_code: String,
}

fn an_unnamed_address_then_validated() -> AddressThenValidated {
    AddressThenValidated::default()
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "serde calls a default function for an `Option` field"
)]
fn maybe_an_unnamed_address_then_validated() -> Option<AddressThenValidated> {
    Some(AddressThenValidated::default())
}

fn a_restricted_stop_then_an_unnamed_one_then_validated() -> Vec<AddressThenValidated> {
    vec![
        AddressThenValidated {
            street: "Calle Falsa 123".to_owned(),
            postal_code: "1".to_owned(),
        },
        AddressThenValidated::default(),
    ]
}

#[derive(serde::Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
struct DeliveryThenValidated {
    #[validate(required)]
    reference: String,
    #[serde(default = "an_unnamed_address_then_validated")]
    #[validate(nested)]
    sender: AddressThenValidated,
    #[serde(default = "maybe_an_unnamed_address_then_validated")]
    #[validate(required, nested)]
    recipient: Option<AddressThenValidated>,
    #[serde(default = "a_restricted_stop_then_an_unnamed_one_then_validated")]
    #[validate(each(custom(not_a_restricted_stop_once_validated), nested))]
    stops: Vec<AddressThenValidated>,
    #[serde(default)]
    #[validate(range(min = 1))]
    parcels: u8,
}

#[test]
fn a_body_without_its_defaulted_fields_is_reported_as_serde_then_validate_report_it() {
    let body = r#"{ "reference": "" }"#;

    assert_eq!(
        issues_found_reading::<Delivery>(body),
        issues_validating_after_serde::<DeliveryThenValidated>(body)
    );
}

#[test]
fn a_body_without_its_defaulted_fields_fails_fast_on_the_issue_serde_then_validate_find_first() {
    let body = r#"{ "reference": "Gran Vía" }"#;

    let error = annotated::from_deserializer_fail_fast::<Delivery, _>(&mut serde_json::Deserializer::from_str(body))
        .unwrap_err();

    let first_after_serde = serde_json::from_str::<DeliveryThenValidated>(body)
        .unwrap()
        .validate_fail_fast()
        .unwrap_err();
    assert_eq!(error.issues(), first_after_serde.as_slice());
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
struct SearchFilters {
    #[validate(length(max = 20))]
    query: String,
    #[validate(range(min = 1, max = 100))]
    page_size: u32,
    sort: Option<String>,
}

impl Default for SearchFilters {
    fn default() -> Self {
        Self {
            query: String::new(),
            page_size: 20,
            sort: Some("relevance".to_owned()),
        }
    }
}

#[test]
fn an_absent_field_takes_its_value_in_the_struct_s_default() {
    let filters = deserialize::<SearchFilters>(r#"{ "query": "boots", "sort": "price" }"#).unwrap();

    assert_eq!(
        filters,
        SearchFilters {
            query: "boots".to_owned(),
            page_size: 20,
            sort: Some("price".to_owned()),
        }
    );
}

#[test]
fn an_absent_optional_field_takes_its_value_in_the_struct_s_default_instead_of_none() {
    let filters = deserialize::<SearchFilters>(r#"{ "query": "boots", "pageSize": 10 }"#).unwrap();

    assert_eq!(
        filters,
        SearchFilters {
            query: "boots".to_owned(),
            page_size: 10,
            sort: Some("relevance".to_owned()),
        }
    );
}

#[test]
fn an_element_missing_at_the_end_of_a_sequence_takes_its_value_in_the_struct_s_default() {
    let filters = deserialize::<SearchFilters>(r#"["boots"]"#).unwrap();

    assert_eq!(
        filters,
        SearchFilters {
            query: "boots".to_owned(),
            page_size: 20,
            sort: Some("relevance".to_owned()),
        }
    );
}

fn fifty_results_by_date_in_spanish() -> ResultsPage {
    ResultsPage {
        size: 50,
        order: "date".to_owned(),
        language: "es".to_owned(),
    }
}

fn english() -> String {
    "en".to_owned()
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(default = "fifty_results_by_date_in_spanish")]
struct ResultsPage {
    #[validate(range(max = 40))]
    size: u32,
    order: String,
    #[serde(default = "english")]
    language: String,
}

#[test]
fn the_struct_s_default_function_gives_an_absent_field_its_value() {
    let page = deserialize::<ResultsPage>(r#"{ "size": 10, "language": "fr" }"#).unwrap();

    assert_eq!(
        page,
        ResultsPage {
            size: 10,
            order: "date".to_owned(),
            language: "fr".to_owned(),
        }
    );
}

#[test]
fn a_field_s_own_default_comes_before_the_struct_s() {
    let page = deserialize::<ResultsPage>(r#"{ "size": 10, "order": "price" }"#).unwrap();

    assert_eq!(
        page,
        ResultsPage {
            size: 10,
            order: "price".to_owned(),
            language: "en".to_owned(),
        }
    );
}

#[test]
fn a_value_from_the_struct_s_default_is_checked_by_its_field_s_rules() {
    let body = r#"{ "order": "price", "language": "fr" }"#;

    assert_eq!(issues_reading::<ResultsPage>(body), ["size: range"]);
}

fn no_struct_default_expected() -> Coordinates {
    panic!("the struct's default was made for an input with every field")
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(default = "no_struct_default_expected")]
struct Coordinates {
    latitude: i32,
    longitude: i32,
}

#[test]
fn the_struct_s_default_is_made_only_when_a_field_is_absent() {
    let coordinates = deserialize::<Coordinates>(r#"{ "latitude": 40, "longitude": -3 }"#).unwrap();

    assert_eq!(
        coordinates,
        Coordinates {
            latitude: 40,
            longitude: -3,
        }
    );
}

#[derive(Deserialize, Debug, Default)]
#[serde(default, rename_all = "camelCase")]
struct Itinerary {
    #[validate(nested)]
    #[allow(dead_code)]
    starting_point: Address,
    #[validate(range(min = 1))]
    #[allow(dead_code)]
    days: u8,
}

#[test]
fn an_absent_nested_field_is_validated_on_its_value_in_the_struct_s_default() {
    assert_eq!(
        issues_reading::<Itinerary>(r#"{ "days": 2 }"#),
        ["startingPoint.street: required", "startingPoint.postalCode: length"]
    );
}

#[derive(Deserialize, Debug, PartialEq, Default)]
#[serde(default)]
struct Paged<T> {
    items: Vec<T>,
    #[validate(range(min = 1))]
    page: u32,
}

#[test]
fn a_struct_with_a_type_parameter_takes_an_absent_field_from_its_default() {
    let paged = deserialize::<Paged<u8>>(r#"{ "page": 2 }"#).unwrap();

    assert_eq!(
        paged,
        Paged {
            items: Vec::new(),
            page: 2,
        }
    );
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(default = "fifty_results_collecting_type_errors")]
#[annotated(collect_type_errors)]
struct ResultsPageCollectingTypeErrors {
    size: u32,
    order: String,
}

fn fifty_results_collecting_type_errors() -> ResultsPageCollectingTypeErrors {
    ResultsPageCollectingTypeErrors {
        size: 50,
        order: "date".to_owned(),
    }
}

#[test]
fn a_struct_collecting_type_errors_takes_an_absent_field_from_its_default() {
    let page = deserialize::<ResultsPageCollectingTypeErrors>(r#"{ "order": "price" }"#).unwrap();

    assert_eq!(
        page,
        ResultsPageCollectingTypeErrors {
            size: 50,
            order: "price".to_owned(),
        }
    );
}

#[derive(Deserialize, Debug, Default)]
#[serde(default, rename_all = "camelCase")]
#[allow(dead_code)]
struct Route {
    #[validate(required)]
    name: String,
    #[validate(nested)]
    origin: Address,
    #[validate(each(custom(not_a_restricted_stop), nested))]
    stops: Vec<Address>,
    #[serde(default = "a_restricted_stop_then_an_unnamed_one")]
    #[validate(each(nested))]
    detours: Vec<Address>,
}

#[derive(serde::Deserialize, Validate, Default)]
#[serde(default, rename_all = "camelCase")]
struct RouteThenValidated {
    #[validate(required)]
    name: String,
    #[validate(nested)]
    origin: AddressThenValidated,
    #[validate(each(custom(not_a_restricted_stop_once_validated), nested))]
    stops: Vec<AddressThenValidated>,
    #[serde(default = "a_restricted_stop_then_an_unnamed_one_then_validated")]
    #[validate(each(nested))]
    detours: Vec<AddressThenValidated>,
}

#[test]
fn a_body_without_the_fields_the_struct_s_default_gives_is_reported_as_serde_then_validate_report_it() {
    let body =
        r#"{ "stops": [{ "street": "Calle Falsa 1", "postalCode": "1" }, { "street": "", "postalCode": "2" }] }"#;

    assert_eq!(
        issues_found_reading::<Route>(body),
        issues_validating_after_serde::<RouteThenValidated>(body)
    );
}

fn unsaved() -> String {
    "unsaved".to_owned()
}

#[derive(Deserialize, Debug, PartialEq)]
struct Draft {
    #[validate(required)]
    title: String,
    #[serde(skip_deserializing)]
    revision: u32,
    #[serde(skip, default = "unsaved")]
    status: String,
}

#[test]
fn a_skipped_field_takes_its_type_s_default() {
    let draft = deserialize::<Draft>(r#"{ "title": "Notes" }"#).unwrap();

    assert_eq!(
        draft,
        Draft {
            title: "Notes".to_owned(),
            revision: 0,
            status: "unsaved".to_owned(),
        }
    );
}

#[test]
fn a_key_with_a_skipped_field_s_name_is_ignored() {
    let draft = deserialize::<Draft>(r#"{ "title": "Notes", "revision": 7, "status": "saved" }"#).unwrap();

    assert_eq!(
        draft,
        Draft {
            title: "Notes".to_owned(),
            revision: 0,
            status: "unsaved".to_owned(),
        }
    );
}

#[derive(Deserialize, Debug, PartialEq)]
struct Revision {
    title: String,
    #[serde(skip)]
    number: u32,
    author: String,
}

#[test]
fn a_skipped_field_is_not_an_element_of_a_struct_read_from_a_sequence() {
    let revision = deserialize::<Revision>(r#"["Notes", "Ada"]"#).unwrap();

    assert_eq!(
        revision,
        Revision {
            title: "Notes".to_owned(),
            number: 0,
            author: "Ada".to_owned(),
        }
    );
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct LogIn {
    #[allow(dead_code)]
    email: String,
    #[serde(skip)]
    #[allow(dead_code)]
    attempts: u32,
}

#[test]
fn a_key_with_a_skipped_field_s_name_is_the_format_s_error_when_the_struct_denies_unknown_fields() {
    let error = deserialize::<LogIn>(r#"{ "email": "ada@example.com", "attempts": 3 }"#).unwrap_err();

    assert_eq!(
        error.to_string(),
        "unknown field `attempts`, expected `email` at line 1 column 40"
    );
}

#[derive(Deserialize, Debug)]
#[annotated(collect_type_errors)]
struct DraftCollectingTypeErrors {
    #[allow(dead_code)]
    title: String,
    #[serde(skip)]
    #[allow(dead_code)]
    revision: u32,
}

#[test]
fn a_struct_collecting_type_errors_reads_no_type_for_a_skipped_field() {
    let body = r#"{ "title": "Notes", "revision": "seventh" }"#;

    assert_eq!(issues_reading::<DraftCollectingTypeErrors>(body), Vec::<String>::new());
}

#[derive(Deserialize, Debug)]
struct Counter {
    #[serde(skip)]
    #[validate(range(min = 1))]
    #[allow(dead_code)]
    step: u32,
}

#[test]
fn a_skipped_field_is_checked_by_its_rules() {
    assert_eq!(issues_reading::<Counter>("{}"), ["step: range"]);
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Invoice {
    #[validate(required)]
    #[allow(dead_code)]
    number: String,
    #[serde(skip)]
    #[validate(nested)]
    #[allow(dead_code)]
    billing_address: Address,
}

#[test]
fn a_skipped_nested_field_is_validated_at_its_name() {
    assert_eq!(
        issues_reading::<Invoice>(r#"{ "number": "" }"#),
        [
            "number: required",
            "billingAddress.street: required",
            "billingAddress.postalCode: length"
        ]
    );
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(default = "fifty_results_by_date_in_spanish_skipping_the_order")]
struct ResultsPageWithAFixedOrder {
    size: u32,
    #[serde(skip)]
    order: String,
}

fn fifty_results_by_date_in_spanish_skipping_the_order() -> ResultsPageWithAFixedOrder {
    ResultsPageWithAFixedOrder {
        size: 50,
        order: "date".to_owned(),
    }
}

#[test]
fn a_skipped_field_takes_its_value_in_the_struct_s_default() {
    let page = deserialize::<ResultsPageWithAFixedOrder>(r#"{ "size": 10, "order": "price" }"#).unwrap();

    assert_eq!(
        page,
        ResultsPageWithAFixedOrder {
            size: 10,
            order: "date".to_owned(),
        }
    );
}

#[derive(Debug, PartialEq, Default)]
struct Session {
    started_at: u64,
}

#[derive(Deserialize, Debug, PartialEq)]
struct Cached<T> {
    key: String,
    #[serde(skip)]
    value: T,
}

#[test]
fn a_skipped_field_whose_type_is_a_type_parameter_needs_no_deserialize() {
    let cached = deserialize::<Cached<Session>>(r#"{ "key": "ada" }"#).unwrap();

    assert_eq!(
        cached,
        Cached {
            key: "ada".to_owned(),
            value: Session { started_at: 0 },
        }
    );
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct Trip {
    #[validate(required)]
    name: String,
    #[serde(skip)]
    #[validate(nested)]
    origin: Address,
    #[serde(skip, default = "a_restricted_stop_then_an_unnamed_one")]
    #[validate(each(custom(not_a_restricted_stop), nested))]
    stops: Vec<Address>,
    #[validate(range(min = 1))]
    days: u8,
}

#[derive(serde::Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
struct TripThenValidated {
    #[validate(required)]
    name: String,
    #[serde(skip)]
    #[validate(nested)]
    origin: AddressThenValidated,
    #[serde(skip, default = "a_restricted_stop_then_an_unnamed_one_then_validated")]
    #[validate(each(custom(not_a_restricted_stop_once_validated), nested))]
    stops: Vec<AddressThenValidated>,
    #[validate(range(min = 1))]
    days: u8,
}

#[test]
fn a_body_for_a_struct_with_skipped_fields_is_reported_as_serde_then_validate_report_it() {
    let body = r#"{ "name": "", "origin": { "street": "Sol 1", "postalCode": "28013" }, "days": 0 }"#;

    assert_eq!(
        issues_found_reading::<Trip>(body),
        issues_validating_after_serde::<TripThenValidated>(body)
    );
}
