//! Scenario 4: an order with a nested customer and 100 nested lines, valid and with 5 lines invalid; a
//! warehouse's stock map of 100 nested entries, valid and with 5 entries invalid; and a map of 100 string
//! labels, each value checked.
//!
//! The rules, the same in every library:
//!
//! | Type | Field | Rules |
//! |---|---|---|
//! | `Order` | `customer: Customer` | nested |
//! | `Order` | `lines: Vec<OrderLine>` | every element nested |
//! | `Customer` | `name: String` | length 1 to 50 chars |
//! | `Customer` | `postalCode: String` | pattern `^\d{5}$` |
//! | `OrderLine` | `sku: String` | length 4 to 16 chars |
//! | `OrderLine` | `quantity: u32` | range 1 to 99 |
//! | `OrderLine` | `unitPrice: f64` | range 0.0 to 10000.0 |
//! | `OrderLine` | `note: Option<String>` | length at most 200 chars, when present |
//! | `Warehouse` | `stock: BTreeMap<String, Stock>` | every value nested |
//! | `Stock` | `quantity: u32` | range at most 10000 |
//! | `Stock` | `location: String` | length 1 to 32 chars |
//! | `Labels` | `labels: BTreeMap<String, String>` | every value: length at most 64 chars |
//!
//! The maps are `BTreeMap`s so every run goes through them in the same order.

use std::fmt::Write as _;

use serde::de::DeserializeOwned;

use super::count;
use super::flat::parsed;
use crate::{Case, Library};

const ELEMENTS: usize = 100;

fn is_one_of_every_twenty(index: usize) -> bool {
    index % 20 == 19
}

/// An order of 100 lines; with `has_invalid_lines`, every twentieth line has a quantity of 0.
#[must_use]
pub fn order_body(has_invalid_lines: bool) -> String {
    let mut lines = String::new();
    for index in 0..ELEMENTS {
        let quantity = if has_invalid_lines && is_one_of_every_twenty(index) {
            0
        } else {
            index % 9 + 1
        };
        let note = if index % 10 == 0 {
            r#""Gift wrap, please""#
        } else {
            "null"
        };
        let separator = if index == 0 { "" } else { "," };
        write!(
            lines,
            r#"{separator}{{"sku":"SKU-{index:04}","quantity":{quantity},"unitPrice":{price}.95,"note":{note}}}"#,
            price = index + 9,
        )
        .expect("a String takes any text");
    }
    format!(r#"{{"customer":{{"name":"Alex Morgan","postalCode":"28013"}},"lines":[{lines}]}}"#)
}

/// A warehouse of 100 stock entries; with `has_invalid_entries`, every twentieth entry has a quantity over
/// 10000.
#[must_use]
pub fn warehouse_body(has_invalid_entries: bool) -> String {
    let mut stock = String::new();
    for index in 0..ELEMENTS {
        let quantity = if has_invalid_entries && is_one_of_every_twenty(index) {
            20_000
        } else {
            index * 10
        };
        let separator = if index == 0 { "" } else { "," };
        write!(
            stock,
            r#"{separator}"warehouse-{index:03}":{{"quantity":{quantity},"location":"Aisle {index}"}}"#
        )
        .expect("a String takes any text");
    }
    format!(r#"{{"stock":{{{stock}}}}}"#)
}

/// 100 labels, each within its 64 chars.
#[must_use]
pub fn labels_body() -> String {
    let mut labels = String::new();
    for index in 0..ELEMENTS {
        let separator = if index == 0 { "" } else { "," };
        write!(labels, r#"{separator}"label-{index:03}":"Etiqueta número {index}""#).expect("a String takes any text");
    }
    format!(r#"{{"labels":{{{labels}}}}}"#)
}

pub mod with_annotated {
    use std::collections::BTreeMap;

    use serde::Deserialize;

    #[derive(Deserialize, annotated::Validate)]
    pub struct Order {
        #[validate(nested)]
        pub customer: Customer,
        #[validate(each(nested))]
        pub lines: Vec<OrderLine>,
    }

    #[derive(Deserialize, annotated::Validate)]
    #[serde(rename_all = "camelCase")]
    pub struct Customer {
        #[validate(length(min = 1, max = 50))]
        pub name: String,
        #[validate(pattern = r"^\d{5}$")]
        pub postal_code: String,
    }

    #[derive(Deserialize, annotated::Validate)]
    #[serde(rename_all = "camelCase")]
    pub struct OrderLine {
        #[validate(length(min = 4, max = 16))]
        pub sku: String,
        #[validate(range(min = 1, max = 99))]
        pub quantity: u32,
        #[validate(range(min = 0.0, max = 10_000.0))]
        pub unit_price: f64,
        #[validate(length(max = 200))]
        pub note: Option<String>,
    }

    #[derive(Deserialize, annotated::Validate)]
    pub struct Warehouse {
        #[validate(each(nested))]
        pub stock: BTreeMap<String, Stock>,
    }

    #[derive(Deserialize, annotated::Validate)]
    pub struct Stock {
        #[validate(range(max = 10_000))]
        pub quantity: u32,
        #[validate(length(min = 1, max = 32))]
        pub location: String,
    }

    #[derive(Deserialize, annotated::Validate)]
    pub struct Labels {
        #[validate(each(length(max = 64)))]
        pub labels: BTreeMap<String, String>,
    }
}

/// The order and the warehouse read with annotated's collecting deserialization: `annotated::Deserialize` in place
/// of serde's derive, so each nested value is checked as it is read, at its path. Their serde `Deserialize`
/// validates too, so these types are only ever read with `annotated::json`.
pub mod with_annotated_deserialize {
    use std::collections::BTreeMap;

    #[derive(annotated::Deserialize)]
    pub struct Order {
        #[validate(nested)]
        pub customer: Customer,
        #[validate(each(nested))]
        pub lines: Vec<OrderLine>,
    }

    #[derive(annotated::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Customer {
        #[validate(length(min = 1, max = 50))]
        pub name: String,
        #[validate(pattern = r"^\d{5}$")]
        pub postal_code: String,
    }

    #[derive(annotated::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct OrderLine {
        #[validate(length(min = 4, max = 16))]
        pub sku: String,
        #[validate(range(min = 1, max = 99))]
        pub quantity: u32,
        #[validate(range(min = 0.0, max = 10_000.0))]
        pub unit_price: f64,
        #[validate(length(max = 200))]
        pub note: Option<String>,
    }

    #[derive(annotated::Deserialize)]
    pub struct Warehouse {
        #[validate(each(nested))]
        pub stock: BTreeMap<String, Stock>,
    }

    #[derive(annotated::Deserialize)]
    pub struct Stock {
        #[validate(range(max = 10_000))]
        pub quantity: u32,
        #[validate(length(min = 1, max = 32))]
        pub location: String,
    }
}

pub mod with_validator {
    use std::collections::BTreeMap;
    use std::sync::LazyLock;

    use regex::Regex;
    use serde::Deserialize;
    use validator::Validate;

    static POSTAL_CODE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\d{5}$").unwrap());

    #[derive(Deserialize, Validate)]
    pub struct Order {
        #[validate(nested)]
        pub customer: Customer,
        #[validate(nested)]
        pub lines: Vec<OrderLine>,
    }

    #[derive(Deserialize, Validate)]
    #[serde(rename_all = "camelCase")]
    pub struct Customer {
        #[validate(length(min = 1, max = 50))]
        pub name: String,
        #[validate(regex(path = *POSTAL_CODE))]
        pub postal_code: String,
    }

    #[derive(Deserialize, Validate)]
    #[serde(rename_all = "camelCase")]
    pub struct OrderLine {
        #[validate(length(min = 4, max = 16))]
        pub sku: String,
        #[validate(range(min = 1, max = 99))]
        pub quantity: u32,
        #[validate(range(min = 0.0, max = 10_000.0))]
        pub unit_price: f64,
        #[validate(length(max = 200))]
        pub note: Option<String>,
    }

    #[derive(Deserialize, Validate)]
    pub struct Warehouse {
        #[validate(nested)]
        pub stock: BTreeMap<String, Stock>,
    }

    #[derive(Deserialize, Validate)]
    pub struct Stock {
        #[validate(range(max = 10_000))]
        pub quantity: u32,
        #[validate(length(min = 1, max = 32))]
        pub location: String,
    }
}

pub mod with_garde {
    use std::collections::BTreeMap;

    use serde::Deserialize;

    #[derive(Deserialize, garde::Validate)]
    pub struct Order {
        #[garde(dive)]
        pub customer: Customer,
        #[garde(dive)]
        pub lines: Vec<OrderLine>,
    }

    #[derive(Deserialize, garde::Validate)]
    #[serde(rename_all = "camelCase")]
    pub struct Customer {
        #[garde(length(chars, min = 1, max = 50))]
        pub name: String,
        #[garde(pattern(r"^\d{5}$"))]
        pub postal_code: String,
    }

    #[derive(Deserialize, garde::Validate)]
    #[serde(rename_all = "camelCase")]
    pub struct OrderLine {
        #[garde(length(chars, min = 4, max = 16))]
        pub sku: String,
        #[garde(range(min = 1, max = 99))]
        pub quantity: u32,
        #[garde(range(min = 0.0, max = 10_000.0))]
        pub unit_price: f64,
        #[garde(length(chars, max = 200))]
        pub note: Option<String>,
    }

    #[derive(Deserialize, garde::Validate)]
    pub struct Warehouse {
        #[garde(dive)]
        pub stock: BTreeMap<String, Stock>,
    }

    #[derive(Deserialize, garde::Validate)]
    pub struct Stock {
        #[garde(range(max = 10_000))]
        pub quantity: u32,
        #[garde(length(chars, min = 1, max = 32))]
        pub location: String,
    }
}

pub mod with_wary {
    use serde::Deserialize;

    #[derive(Deserialize, wary::Wary)]
    pub struct Order {
        #[validate(dive)]
        pub customer: Customer,
        #[validate(inner(dive))]
        pub lines: Vec<OrderLine>,
    }

    #[derive(Deserialize, wary::Wary)]
    #[serde(rename_all = "camelCase")]
    pub struct Customer {
        #[validate(length(chars, 1..=50))]
        pub name: String,
        #[validate(regex(pat = r"^\d{5}$"))]
        pub postal_code: String,
    }

    #[derive(Deserialize, wary::Wary)]
    #[serde(rename_all = "camelCase")]
    pub struct OrderLine {
        #[validate(length(chars, 4..=16))]
        pub sku: String,
        #[validate(range(1..=99))]
        pub quantity: u32,
        #[validate(range(0.0..=10_000.0))]
        pub unit_price: f64,
        #[validate(length(chars, ..=200))]
        pub note: Option<String>,
    }
}

pub mod with_serde_valid {
    use std::collections::BTreeMap;

    use serde::Deserialize;

    #[derive(Deserialize, serde_valid::Validate)]
    pub struct Order {
        #[validate]
        pub customer: Customer,
        #[validate]
        pub lines: Vec<OrderLine>,
    }

    #[derive(Deserialize, serde_valid::Validate)]
    #[serde(rename_all = "camelCase")]
    pub struct Customer {
        #[validate(min_length = 1)]
        #[validate(max_length = 50)]
        pub name: String,
        #[validate(pattern = r"^\d{5}$")]
        pub postal_code: String,
    }

    #[derive(Deserialize, serde_valid::Validate)]
    #[serde(rename_all = "camelCase")]
    pub struct OrderLine {
        #[validate(min_length = 4)]
        #[validate(max_length = 16)]
        pub sku: String,
        #[validate(minimum = 1)]
        #[validate(maximum = 99)]
        pub quantity: u32,
        #[validate(minimum = 0.0)]
        #[validate(maximum = 10_000.0)]
        pub unit_price: f64,
        #[validate(max_length = 200)]
        pub note: Option<String>,
    }

    #[derive(Deserialize, serde_valid::Validate)]
    pub struct Warehouse {
        #[validate]
        pub stock: BTreeMap<String, Stock>,
    }

    #[derive(Deserialize, serde_valid::Validate)]
    pub struct Stock {
        #[validate(maximum = 10_000)]
        pub quantity: u32,
        #[validate(min_length = 1)]
        #[validate(max_length = 32)]
        pub location: String,
    }
}

#[must_use]
pub fn cases() -> Vec<Case> {
    let mut cases = Vec::new();
    for (scenario, body) in [
        ("nested_valid", order_body(false)),
        ("nested_5pct_invalid", order_body(true)),
    ] {
        cases.extend(order_cases(scenario, &body));
    }
    for (scenario, body) in [
        ("map_nested_valid", warehouse_body(false)),
        ("map_nested_5pct_invalid", warehouse_body(true)),
    ] {
        cases.extend(warehouse_cases(scenario, &body));
    }
    cases.extend(labels_cases("map_string_values_valid", &labels_body()));
    cases
}

fn order_cases(scenario: &'static str, body: &str) -> Vec<Case> {
    vec![
        annotated_case::<with_annotated::Order>(scenario, body),
        validator_case::<with_validator::Order>(scenario, body),
        garde_case::<with_garde::Order>(scenario, body),
        wary_case::<with_wary::Order>(scenario, body),
        serde_valid_case::<with_serde_valid::Order>(scenario, body),
    ]
}

/// wary goes through no map: it implements neither `Validate` nor `inner` for one.
fn warehouse_cases(scenario: &'static str, body: &str) -> Vec<Case> {
    vec![
        annotated_case::<with_annotated::Warehouse>(scenario, body),
        validator_case::<with_validator::Warehouse>(scenario, body),
        garde_case::<with_garde::Warehouse>(scenario, body),
        serde_valid_case::<with_serde_valid::Warehouse>(scenario, body),
    ]
}

/// Only annotated checks a map's values with rules of their own; the others check a map's values only as
/// nested structs.
fn labels_cases(scenario: &'static str, body: &str) -> Vec<Case> {
    vec![annotated_case::<with_annotated::Labels>(scenario, body)]
}

fn annotated_case<T: DeserializeOwned + annotated::Validate + 'static>(scenario: &'static str, body: &str) -> Case {
    Case::new(
        scenario,
        Library::Annotated,
        parsed::<T>(body),
        annotated::Validate::validate,
        count::annotated,
    )
}

fn validator_case<T: DeserializeOwned + validator::Validate + 'static>(scenario: &'static str, body: &str) -> Case {
    Case::new(
        scenario,
        Library::Validator,
        parsed::<T>(body),
        validator::Validate::validate,
        count::validator,
    )
}

fn garde_case<T: DeserializeOwned + garde::Validate<Context = ()> + 'static>(
    scenario: &'static str,
    body: &str,
) -> Case {
    Case::new(
        scenario,
        Library::Garde,
        parsed::<T>(body),
        garde::Validate::validate,
        count::garde,
    )
}

fn wary_case<T: DeserializeOwned + wary::Validate<Context = ()> + 'static>(scenario: &'static str, body: &str) -> Case {
    Case::new(
        scenario,
        Library::Wary,
        parsed::<T>(body),
        |value| wary::Validate::validate(value, &()),
        count::wary,
    )
}

fn serde_valid_case<T: DeserializeOwned + serde_valid::Validate + 'static>(scenario: &'static str, body: &str) -> Case {
    Case::new(
        scenario,
        Library::SerdeValid,
        parsed::<T>(body),
        serde_valid::Validate::validate,
        count::serde_valid,
    )
    .variant("graphemes")
}
