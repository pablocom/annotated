//! Scenario 7: the order and the warehouse of scenario 4, from JSON text to a validated value, valid and with 5
//! of their 100 elements invalid. Every library reads them with `serde_json::from_str`, then validates. annotated
//! also reads them with its collecting deserialization, `annotated::json::from_str`, which checks each line and
//! each stock entry as it reads it, at its path. `serde_json` alone, deserializing the same body into types
//! without rules, is the floor every library builds on.
//!
//! wary goes through no map, so it reads only the order.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde::de::DeserializeOwned;

use super::count;
use super::end_to_end::{received, violations_received};
use super::nested::{
    self, with_annotated, with_annotated_deserialize, with_garde, with_serde_valid, with_validator, with_wary,
};
use crate::{Case, Library};

#[derive(Deserialize)]
#[expect(dead_code, reason = "deserialized for its cost, never read")]
pub struct PlainOrder {
    customer: PlainCustomer,
    lines: Vec<PlainOrderLine>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(dead_code, reason = "deserialized for its cost, never read")]
pub struct PlainCustomer {
    name: String,
    postal_code: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(dead_code, reason = "deserialized for its cost, never read")]
pub struct PlainOrderLine {
    sku: String,
    quantity: u32,
    unit_price: f64,
    note: Option<String>,
}

#[derive(Deserialize)]
#[expect(dead_code, reason = "deserialized for its cost, never read")]
pub struct PlainWarehouse {
    stock: BTreeMap<String, PlainStock>,
}

#[derive(Deserialize)]
#[expect(dead_code, reason = "deserialized for its cost, never read")]
pub struct PlainStock {
    quantity: u32,
    location: String,
}

#[must_use]
pub fn cases() -> Vec<Case> {
    let mut cases = Vec::new();
    for (scenario, body) in [
        ("end_to_end_nested_valid", nested::order_body(false)),
        ("end_to_end_nested_5pct_invalid", nested::order_body(true)),
    ] {
        cases.extend(order_cases(scenario, &body));
    }
    for (scenario, body) in [
        ("end_to_end_map_nested_valid", nested::warehouse_body(false)),
        ("end_to_end_map_nested_5pct_invalid", nested::warehouse_body(true)),
    ] {
        cases.extend(warehouse_cases(scenario, &body));
    }
    cases
}

fn order_cases(scenario: &'static str, body: &str) -> Vec<Case> {
    vec![
        serde_json_alone_case::<PlainOrder>(scenario, body),
        annotated_deserialize_case::<with_annotated_deserialize::Order>(scenario, body),
        annotated_case::<with_annotated::Order>(scenario, body),
        validator_case::<with_validator::Order>(scenario, body),
        garde_case::<with_garde::Order>(scenario, body),
        wary_case::<with_wary::Order>(scenario, body),
        serde_valid_case::<with_serde_valid::Order>(scenario, body),
    ]
}

fn warehouse_cases(scenario: &'static str, body: &str) -> Vec<Case> {
    vec![
        serde_json_alone_case::<PlainWarehouse>(scenario, body),
        annotated_deserialize_case::<with_annotated_deserialize::Warehouse>(scenario, body),
        annotated_case::<with_annotated::Warehouse>(scenario, body),
        validator_case::<with_validator::Warehouse>(scenario, body),
        garde_case::<with_garde::Warehouse>(scenario, body),
        serde_valid_case::<with_serde_valid::Warehouse>(scenario, body),
    ]
}

fn serde_json_alone_case<T: DeserializeOwned + 'static>(scenario: &'static str, body: &str) -> Case {
    Case::new(
        scenario,
        Library::SerdeJsonAlone,
        body.to_owned(),
        |body| serde_json::from_str::<T>(body).map(drop),
        |received| usize::from(received.is_err()),
    )
}

fn annotated_deserialize_case<T>(scenario: &'static str, body: &str) -> Case
where
    T: for<'de> annotated::Deserialize<'de> + 'static,
{
    Case::new(
        scenario,
        Library::Annotated,
        body.to_owned(),
        |body| annotated::json::from_str::<T>(body),
        count::annotated_deserialized,
    )
    .variant("json::from_str")
}

fn annotated_case<T: DeserializeOwned + annotated::Validate + 'static>(scenario: &'static str, body: &str) -> Case {
    Case::new(
        scenario,
        Library::Annotated,
        body.to_owned(),
        |body| received(body, T::validate),
        |received| violations_received(received, count::annotated),
    )
    .variant("serde_json, then validate")
}

fn validator_case<T: DeserializeOwned + validator::Validate + 'static>(scenario: &'static str, body: &str) -> Case {
    Case::new(
        scenario,
        Library::Validator,
        body.to_owned(),
        |body| received(body, T::validate),
        |received| violations_received(received, count::validator),
    )
}

fn garde_case<T: DeserializeOwned + garde::Validate<Context = ()> + 'static>(
    scenario: &'static str,
    body: &str,
) -> Case {
    Case::new(
        scenario,
        Library::Garde,
        body.to_owned(),
        |body| received(body, |value: &T| value.validate()),
        |received| violations_received(received, count::garde),
    )
}

fn wary_case<T: DeserializeOwned + wary::Validate<Context = ()> + 'static>(scenario: &'static str, body: &str) -> Case {
    Case::new(
        scenario,
        Library::Wary,
        body.to_owned(),
        |body| received(body, |value: &T| value.validate(&())),
        |received| violations_received(received, count::wary),
    )
}

fn serde_valid_case<T: DeserializeOwned + serde_valid::Validate + 'static>(scenario: &'static str, body: &str) -> Case {
    Case::new(
        scenario,
        Library::SerdeValid,
        body.to_owned(),
        |body| received(body, T::validate),
        |received| violations_received(received, count::serde_valid),
    )
    .variant("graphemes")
}
