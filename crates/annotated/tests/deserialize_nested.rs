//! `#[derive(Deserialize)]` reads a field checked with `nested`, or `each(nested)`, through `annotated::Deserialize`,
//! so the issues inside it are found at their full path, in the order `validate()` reports them.

#![cfg(feature = "serde")]

use std::collections::{BTreeMap, HashMap};
use std::hash::{BuildHasherDefault, DefaultHasher};

use annotated::{Deserialize, DeserializeError, Issue, Location, Problem, Report, Validate};

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

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Address {
    #[validate(required)]
    street: String,
    #[validate(length(min = 5))]
    postal_code: String,
}

fn gran_via() -> Address {
    Address {
        street: "Gran Vía 1".to_owned(),
        postal_code: "28013".to_owned(),
    }
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Parcel {
    #[validate(required)]
    reference: String,
    #[validate(nested)]
    sender: Address,
    #[validate(range(min = 1))]
    weight_in_grams: u32,
}

#[test]
fn a_field_missing_inside_a_nested_struct_is_reported_at_its_full_path_in_declaration_order() {
    let body = r#"{ "weightInGrams": 0, "reference": "", "sender": { "postalCode": "28013" } }"#;

    assert_eq!(
        issues_reading::<Parcel>(body),
        ["reference: required", "sender.street: missing", "weightInGrams: range"]
    );
}

#[test]
fn a_nested_struct_breaking_no_rule_is_deserialized() {
    let body =
        r#"{ "reference": "PX-1", "sender": { "street": "Gran Vía 1", "postalCode": "28013" }, "weightInGrams": 900 }"#;

    let parcel = deserialize::<Parcel>(body).unwrap();

    assert_eq!(
        parcel,
        Parcel {
            reference: "PX-1".to_owned(),
            sender: gran_via(),
            weight_in_grams: 900,
        }
    );
}

#[test]
fn a_missing_nested_struct_is_missing() {
    assert_eq!(
        issues_reading::<Parcel>(r#"{ "reference": "PX-1", "weightInGrams": 900 }"#),
        ["sender: missing"]
    );
}

#[derive(Deserialize, Debug)]
struct Team {
    #[validate(each(nested))]
    #[allow(dead_code)]
    members: Vec<Member>,
}

#[derive(Deserialize, Debug, PartialEq)]
struct Member {
    #[validate(required)]
    name: String,
}

#[test]
fn every_element_of_a_list_checked_with_each_nested_is_read_at_its_index() {
    let body = r#"{ "members": [{ "name": "Alex" }, {}] }"#;

    assert_eq!(issues_reading::<Team>(body), ["members[1].name: missing"]);
}

#[derive(Deserialize, Debug)]
struct Warehouse {
    #[validate(each(nested))]
    #[allow(dead_code)]
    stock: BTreeMap<String, Stock>,
}

#[derive(Deserialize, Debug, PartialEq)]
struct Stock {
    #[validate(range(max = 100))]
    quantity: u32,
}

#[test]
fn every_value_of_a_map_checked_with_each_nested_is_read_at_its_key() {
    let body = r#"{ "stock": { "sku-2": { "quantity": 3 }, "sku-1": { "quantity": 300 } } }"#;

    assert_eq!(issues_reading::<Warehouse>(body), ["stock[sku-1].quantity: range"]);
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Order {
    #[validate(required, nested)]
    billing_address: Option<Address>,
}

#[test]
fn an_optional_nested_struct_that_is_required_and_absent_breaks_the_required_rule() {
    assert_eq!(issues_reading::<Order>("{}"), ["billingAddress: required"]);
}

#[test]
fn an_optional_nested_struct_that_is_required_and_null_breaks_the_required_rule() {
    assert_eq!(
        issues_reading::<Order>(r#"{ "billingAddress": null }"#),
        ["billingAddress: required"]
    );
}

#[test]
fn an_optional_nested_struct_that_is_present_reports_the_issues_inside_it() {
    let body = r#"{ "billingAddress": { "street": "", "postalCode": "280" } }"#;

    assert_eq!(
        issues_reading::<Order>(body),
        ["billingAddress.street: required", "billingAddress.postalCode: length"]
    );
}

#[test]
fn an_optional_nested_struct_that_is_present_and_valid_is_deserialized() {
    let body = r#"{ "billingAddress": { "street": "Gran Vía 1", "postalCode": "28013" } }"#;

    let order = deserialize::<Order>(body).unwrap();

    assert_eq!(
        order,
        Order {
            billing_address: Some(gran_via())
        }
    );
}

#[derive(Deserialize, Debug)]
struct Company {
    #[validate(nested)]
    #[allow(dead_code)]
    headquarters: Office,
}

#[derive(Deserialize, Debug)]
struct Office {
    #[validate(nested)]
    #[allow(dead_code)]
    address: Address,
}

#[test]
fn a_struct_nested_two_levels_deep_is_reported_at_its_full_path() {
    let body = r#"{ "headquarters": { "address": { "postalCode": "28013" } } }"#;

    assert_eq!(
        issues_reading::<Company>(body),
        ["headquarters.address.street: missing"]
    );
}

#[derive(Deserialize, Debug, PartialEq)]
struct Route {
    #[validate(each(nested))]
    stops: Option<Vec<Address>>,
}

#[test]
fn every_element_of_an_optional_list_is_read_at_its_index() {
    let body =
        r#"{ "stops": [{ "street": "Gran Vía 1", "postalCode": "28013" }, { "street": "", "postalCode": "28013" }] }"#;

    assert_eq!(issues_reading::<Route>(body), ["stops[1].street: required"]);
}

#[test]
fn an_absent_optional_list_has_no_elements_to_check() {
    assert_eq!(deserialize::<Route>("{}").unwrap(), Route { stops: None });
}

fn not_a_restricted_area(_: &Address) -> Result<(), Problem> {
    Err(Problem::custom("restricted_area", "is in a restricted area"))
}

fn not_sent_on_a_holiday(_: &Shipment) -> Result<(), Problem> {
    Err(Problem::custom("holiday", "can't be sent on a holiday"))
}

#[derive(Deserialize, Debug)]
#[validate(custom(not_sent_on_a_holiday))]
struct Shipment {
    #[validate(nested, custom(not_a_restricted_area))]
    #[allow(dead_code)]
    destination: Address,
    #[validate(required)]
    #[allow(dead_code)]
    carrier: String,
}

#[test]
fn a_nested_value_that_could_not_be_built_skips_the_other_rules_of_its_field_and_the_struct_s_rules() {
    let body = r#"{ "destination": { "postalCode": "28013" }, "carrier": "" }"#;

    assert_eq!(
        issues_reading::<Shipment>(body),
        ["destination.street: missing", "carrier: required"]
    );
}

#[test]
fn a_nested_value_breaking_only_rules_is_built_so_the_struct_s_rules_run() {
    let body = r#"{ "destination": { "street": "", "postalCode": "28013" }, "carrier": "Correos" }"#;

    assert_eq!(
        issues_reading::<Shipment>(body),
        ["destination.street: required", "holiday"]
    );
}

#[test]
fn the_rules_after_a_valid_nested_value_are_checked() {
    let body = r#"{ "destination": { "street": "Gran Vía 1", "postalCode": "28013" }, "carrier": "Correos" }"#;

    assert_eq!(
        issues_reading::<Shipment>(body),
        ["destination: restricted_area", "holiday"]
    );
}

#[derive(Debug, PartialEq)]
struct Handle(String);

impl<'de> annotated::Deserialize<'de> for Handle {
    fn deserialize_at<D: serde::Deserializer<'de>>(
        deserializer: D,
        at: &Location<'_>,
        found: &mut Report,
    ) -> Result<Option<Self>, D::Error> {
        let handle: String = serde::Deserialize::deserialize(deserializer)?;
        if !handle.starts_with('@') {
            found.push(at, Problem::custom("handle", "must start with @"));
        }
        Ok(Some(Self(handle)))
    }
}

#[derive(Deserialize, Debug, PartialEq)]
struct Profile {
    #[validate(nested)]
    handle: Handle,
}

#[test]
fn a_nested_value_deserialized_by_hand_reports_its_issues_under_its_field() {
    assert_eq!(
        issues_reading::<Profile>(r#"{ "handle": "lucia" }"#),
        ["handle: handle"]
    );
}

#[test]
fn a_nested_value_deserialized_by_hand_is_read_into_its_field() {
    let profile = deserialize::<Profile>(r#"{ "handle": "@lucia" }"#).unwrap();

    assert_eq!(
        profile,
        Profile {
            handle: Handle("@lucia".to_owned())
        }
    );
}

type Inventory = HashMap<String, Stock, BuildHasherDefault<DefaultHasher>>;

#[derive(Deserialize, Debug)]
struct Depot {
    #[validate(each(nested))]
    #[allow(dead_code)]
    stock: Inventory,
    #[validate(each(nested))]
    #[allow(dead_code)]
    docks: Box<[Address]>,
}

#[test]
fn every_value_of_a_hash_map_with_its_own_hasher_is_read_at_its_key() {
    let body = r#"{ "stock": { "sku-1": { "quantity": 300 } }, "docks": [] }"#;

    assert_eq!(issues_reading::<Depot>(body), ["stock[sku-1].quantity: range"]);
}

#[test]
fn every_element_of_a_boxed_slice_is_read_at_its_index() {
    let body =
        r#"{ "stock": {}, "docks": [{ "street": "Gran Vía 1", "postalCode": "28013" }, { "postalCode": "28013" }] }"#;

    assert_eq!(issues_reading::<Depot>(body), ["docks[1].street: missing"]);
}

#[derive(Deserialize, Debug)]
struct Rota {
    #[validate(each(each(nested)))]
    #[allow(dead_code)]
    shifts: BTreeMap<String, Vec<Member>>,
}

#[test]
fn every_element_of_every_list_in_a_map_is_read_at_its_key_and_index() {
    let body = r#"{ "shifts": { "tuesday": [{ "name": "Alex" }, { "name": "" }], "monday": [{ "name": "" }] } }"#;

    assert_eq!(
        issues_reading::<Rota>(body),
        ["shifts[monday][0].name: required", "shifts[tuesday][1].name: required"]
    );
}

#[derive(Deserialize, Debug, PartialEq)]
struct Page<T> {
    #[validate(each(nested))]
    items: Vec<T>,
}

#[test]
fn a_type_parameter_checked_with_each_nested_is_read_through_annotated_deserialize() {
    assert_eq!(
        issues_reading::<Page<Member>>(r#"{ "items": [{ "name": "" }] }"#),
        ["items[0].name: required"]
    );
}

#[derive(Deserialize, Debug)]
struct Tour {
    #[validate(each(custom(not_a_restricted_stop)), each(required, nested))]
    #[allow(dead_code)]
    stops: Vec<Option<Address>>,
}

#[test]
fn the_issues_inside_an_element_are_reported_when_an_each_without_nested_comes_before_the_one_with_it() {
    let body = r#"{ "stops": [{ "street": "", "postalCode": "28013" }] }"#;

    assert_eq!(issues_reading::<Tour>(body), ["stops[0].street: required"]);
}

#[derive(Deserialize, Debug)]
#[validate(report_all)]
struct Delivery {
    #[validate(nested, custom(not_a_restricted_area))]
    #[allow(dead_code)]
    drop_off: Address,
}

#[test]
fn report_all_checks_the_rules_after_a_nested_value_with_issues() {
    let body = r#"{ "drop_off": { "street": "", "postalCode": "28013" } }"#;

    assert_eq!(
        issues_reading::<Delivery>(body),
        ["drop_off.street: required", "drop_off: restricted_area"]
    );
}

fn not_the_sender(parcel: &Consignment) -> Result<(), Problem> {
    differs_from_the_sender(&parcel.sender, &parcel.recipient)
}

fn not_the_sender_once_validated(parcel: &ConsignmentThenValidated) -> Result<(), Problem> {
    differs_from_the_sender(&parcel.sender, &parcel.recipient)
}

fn differs_from_the_sender<T: PartialEq>(sender: &T, recipient: &T) -> Result<(), Problem> {
    if sender == recipient {
        return Err(Problem::custom("same_as_sender", "can't be the sender"));
    }
    Ok(())
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

fn not_a_restricted_stop_once_validated(address: &AddressThenValidated) -> Result<(), Problem> {
    on_the_map(&address.street)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
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

#[derive(serde::Deserialize, Validate, PartialEq)]
#[serde(rename_all = "camelCase")]
struct AddressThenValidated {
    #[validate(required)]
    street: String,
    #[validate(length(min = 5))]
    postal_code: String,
}

#[derive(serde::Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
#[validate(custom(not_the_sender_once_validated, field = recipient))]
struct ConsignmentThenValidated {
    #[validate(nested)]
    sender: AddressThenValidated,
    #[validate(nested)]
    recipient: AddressThenValidated,
    #[validate(required, nested)]
    return_address: Option<AddressThenValidated>,
    #[validate(length(max = 3), each(custom(not_a_restricted_stop_once_validated), nested))]
    stops: Vec<AddressThenValidated>,
    #[validate(each(required, nested))]
    pickups: Vec<Option<AddressThenValidated>>,
    #[validate(each(nested, custom(not_a_restricted_stop_once_validated)))]
    depots: BTreeMap<String, AddressThenValidated>,
    #[validate(each(each(nested)))]
    rounds: BTreeMap<String, Vec<AddressThenValidated>>,
}

#[test]
fn a_complete_nested_body_breaking_every_rule_is_reported_as_serde_then_validate_report_it() {
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
        issues_found_reading::<Consignment>(body),
        issues_validating_after_serde::<ConsignmentThenValidated>(body)
    );
}

#[test]
fn a_complete_nested_body_breaking_a_rule_before_each_is_reported_as_serde_then_validate_report_it() {
    let body = r#"{
        "sender": { "street": "Gran Vía 1", "postalCode": "28013" },
        "recipient": { "street": "Gran Vía 1", "postalCode": "28013" },
        "returnAddress": { "street": "Atocha 2", "postalCode": "28012" },
        "stops": [
            { "street": "", "postalCode": "1" }, { "street": "", "postalCode": "1" },
            { "street": "", "postalCode": "1" }, { "street": "", "postalCode": "1" }
        ],
        "pickups": [],
        "depots": {},
        "rounds": {}
    }"#;

    assert_eq!(
        issues_found_reading::<Consignment>(body),
        issues_validating_after_serde::<ConsignmentThenValidated>(body)
    );
}

#[test]
fn a_complete_nested_body_breaking_no_rule_is_valid_as_serde_then_validate_find_it() {
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
        issues_found_reading::<Consignment>(body),
        issues_validating_after_serde::<ConsignmentThenValidated>(body)
    );
}

#[test]
fn a_struct_written_as_a_sequence_reads_its_nested_values() {
    let body = r#"["", { "postalCode": "28013" }, 0]"#;

    assert_eq!(
        issues_reading::<Parcel>(body),
        ["reference: required", "sender.street: missing", "weightInGrams: range"]
    );
}

struct Pair<T> {
    first: T,
    second: T,
}

impl<T> annotated::rules::HasElements for Pair<T> {
    type Element = T;

    fn for_each_element(&self, at: &Location<'_>, mut check: impl FnMut(&T, &Location<'_>) -> bool) {
        if check(&self.first, &at.index(0)) {
            check(&self.second, &at.index(1));
        }
    }
}

impl<'de, T: Deserialize<'de>> annotated::Deserialize<'de> for Pair<T> {
    fn deserialize_at<D: serde::Deserializer<'de>>(
        deserializer: D,
        at: &Location<'_>,
        found: &mut Report,
    ) -> Result<Option<Self>, D::Error> {
        let [first, second]: [serde_json::Value; 2] = serde::Deserialize::deserialize(deserializer)?;
        let mut found_in_first = found.new_alike();
        let first = T::deserialize_at(first, &at.index(0), &mut found_in_first).map_err(serde::de::Error::custom)?;
        let second = T::deserialize_at(second, &at.index(1), found).map_err(serde::de::Error::custom)?;
        found.append(&mut found_in_first);
        Ok(first.zip(second).map(|(first, second)| Self { first, second }))
    }
}

#[derive(Deserialize)]
struct Duel {
    #[validate(each(nested))]
    #[allow(dead_code)]
    rivals: Pair<Member>,
}

#[test]
fn the_issues_a_collection_reports_out_of_the_order_of_its_elements_are_still_reported() {
    let body = r#"{ "rivals": [{ "name": "" }, { "name": "" }] }"#;

    assert_eq!(
        issues_reading::<Duel>(body),
        ["rivals[1].name: required", "rivals[0].name: required"]
    );
}
