//! A schema can say one `pattern`, and annotated requires any other through `allOf`. utoipa's schema of a string
//! can't hold an `allOf`, so the patterns are required through an `allOf` of their own, each beside the string's type.

// The types here are only documented, never read.
#![allow(dead_code)]

mod support;

use std::collections::HashMap;

use annotated::Deserialize;
use annotated_utoipa::write_rules_of;
use serde_json::json;
use support::schema_named;
use utoipa::{OpenApi, ToSchema};

#[derive(Deserialize, ToSchema)]
struct NewProduct {
    /// The code the warehouse knows it by.
    #[validate(length(max = 20), prefix = "SKU-", suffix = "-X")]
    sku: String,
}

#[derive(Deserialize, ToSchema)]
struct ProductChange {
    #[validate(prefix = "SKU-", suffix = "-X")]
    sku: Option<String>,
}

#[derive(Deserialize, ToSchema)]
struct Bundle {
    #[validate(each(prefix = "SKU-", suffix = "-X"))]
    skus: Vec<String>,
}

#[derive(Deserialize, ToSchema)]
struct Stock {
    #[validate(each(prefix = "SKU-", suffix = "-X"))]
    sku_by_shelf: HashMap<String, String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct Audited {
    created_by: String,
}

#[derive(Deserialize, ToSchema)]
struct AuditedProduct {
    #[validate(prefix = "SKU-", suffix = "-X")]
    sku: String,
    #[serde(flatten)]
    audited: Audited,
}

#[derive(OpenApi)]
#[openapi(components(schemas(NewProduct, ProductChange, Bundle, Stock, AuditedProduct, Audited)))]
struct ProductsApi;

#[test]
fn a_second_pattern_is_required_beside_the_first_through_all_of() {
    let mut document = ProductsApi::openapi();

    let _ = write_rules_of::<NewProduct>(&mut document);

    assert_eq!(
        schema_named("NewProduct", &document)["properties"]["sku"],
        json!({
            "allOf": [
                {
                    "type": "string",
                    "description": "The code the warehouse knows it by.",
                    "maxLength": 20,
                    "pattern": "^SKU-"
                },
                { "type": "string", "pattern": "-X$" }
            ]
        })
    );
}

#[test]
fn a_second_pattern_on_an_optional_string_still_accepts_null() {
    let mut document = ProductsApi::openapi();

    let _ = write_rules_of::<ProductChange>(&mut document);

    assert_eq!(
        schema_named("ProductChange", &document)["properties"]["sku"],
        json!({
            "allOf": [
                { "type": ["string", "null"], "pattern": "^SKU-" },
                { "type": ["string", "null"], "pattern": "-X$" }
            ]
        })
    );
}

#[test]
fn a_second_pattern_on_each_element_is_required_through_all_of() {
    let mut document = ProductsApi::openapi();

    let _ = write_rules_of::<Bundle>(&mut document);

    assert_eq!(
        schema_named("Bundle", &document)["properties"]["skus"],
        json!({
            "type": "array",
            "items": {
                "allOf": [
                    { "type": "string", "pattern": "^SKU-" },
                    { "type": "string", "pattern": "-X$" }
                ]
            }
        })
    );
}

#[test]
fn a_second_pattern_on_each_value_of_a_map_is_required_through_all_of() {
    let mut document = ProductsApi::openapi();

    let _ = write_rules_of::<Stock>(&mut document);

    assert_eq!(
        schema_named("Stock", &document)["properties"]["sku_by_shelf"],
        json!({
            "type": "object",
            "additionalProperties": {
                "allOf": [
                    { "type": "string", "pattern": "^SKU-" },
                    { "type": "string", "pattern": "-X$" }
                ]
            },
            "propertyNames": { "type": "string" }
        })
    );
}

#[test]
fn a_second_pattern_on_a_field_beside_a_flattened_one_is_required_through_all_of() {
    let mut document = ProductsApi::openapi();

    let _ = write_rules_of::<AuditedProduct>(&mut document);

    assert_eq!(
        schema_named("AuditedProduct", &document)["allOf"][1]["properties"]["sku"],
        json!({
            "allOf": [
                { "type": "string", "pattern": "^SKU-" },
                { "type": "string", "pattern": "-X$" }
            ]
        })
    );
}
