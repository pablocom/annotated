//! A schema can say one `pattern`, and annotated requires any other through `allOf`. utoipa's schema of a string has
//! room for one `pattern` and no `allOf`, so the property stays a string with the first pattern, which every client
//! generated from the document reads, and the `allOf` is returned as what utoipa's schema can't hold.

// The types here are only documented, never read.
#![allow(dead_code)]

mod support;

use std::collections::HashMap;

use annotated::Deserialize;
use annotated_utoipa::{Unwritten, write_rules_of};
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
fn a_second_pattern_leaves_the_property_a_string_with_the_first() {
    let mut document = ProductsApi::openapi();

    let _ = write_rules_of::<NewProduct>(&mut document);

    assert_eq!(
        schema_named("NewProduct", &document)["properties"]["sku"],
        json!({
            "type": "string",
            "description": "The code the warehouse knows it by.",
            "maxLength": 20,
            "pattern": "^SKU-"
        })
    );
}

#[test]
fn a_second_pattern_is_returned_where_annotated_wrote_it() {
    let mut document = ProductsApi::openapi();

    let unwritten = write_rules_of::<NewProduct>(&mut document);

    assert_eq!(
        unwritten.iter().map(ToString::to_string).collect::<Vec<_>>(),
        [
            "what was written at `/properties/sku/allOf` in the schema `NewProduct` is left out: utoipa's schema \
             can't hold it"
        ]
    );
}

#[test]
fn a_second_pattern_on_an_optional_string_leaves_it_accepting_null() {
    let mut document = ProductsApi::openapi();

    let _ = write_rules_of::<ProductChange>(&mut document);

    assert_eq!(
        schema_named("ProductChange", &document)["properties"]["sku"],
        json!({ "type": ["string", "null"], "pattern": "^SKU-" })
    );
}

#[test]
fn a_second_pattern_on_each_element_leaves_the_elements_strings_with_the_first() {
    let mut document = ProductsApi::openapi();

    let _ = write_rules_of::<Bundle>(&mut document);

    assert_eq!(
        schema_named("Bundle", &document)["properties"]["skus"],
        json!({ "type": "array", "items": { "type": "string", "pattern": "^SKU-" } })
    );
}

#[test]
fn a_second_pattern_on_each_element_is_returned_in_the_elements() {
    let mut document = ProductsApi::openapi();

    let unwritten = write_rules_of::<Bundle>(&mut document);

    assert_eq!(
        unwritten.iter().map(Unwritten::pointer).collect::<Vec<_>>(),
        ["/properties/skus/items/allOf"]
    );
}

#[test]
fn a_second_pattern_on_each_value_of_a_map_leaves_the_values_strings_with_the_first() {
    let mut document = ProductsApi::openapi();

    let _ = write_rules_of::<Stock>(&mut document);

    assert_eq!(
        schema_named("Stock", &document)["properties"]["sku_by_shelf"],
        json!({
            "type": "object",
            "additionalProperties": { "type": "string", "pattern": "^SKU-" },
            "propertyNames": { "type": "string" }
        })
    );
}

#[test]
fn a_second_pattern_on_each_value_of_a_map_is_returned_in_the_values() {
    let mut document = ProductsApi::openapi();

    let unwritten = write_rules_of::<Stock>(&mut document);

    assert_eq!(
        unwritten.iter().map(Unwritten::pointer).collect::<Vec<_>>(),
        ["/properties/sku_by_shelf/additionalProperties/allOf"]
    );
}

#[test]
fn a_second_pattern_on_a_field_beside_a_flattened_one_leaves_it_a_string_with_the_first() {
    let mut document = ProductsApi::openapi();

    let _ = write_rules_of::<AuditedProduct>(&mut document);

    assert_eq!(
        schema_named("AuditedProduct", &document)["allOf"][1]["properties"]["sku"],
        json!({ "type": "string", "pattern": "^SKU-" })
    );
}

#[test]
fn a_second_pattern_on_a_field_beside_a_flattened_one_is_returned_in_the_member_that_declares_it() {
    let mut document = ProductsApi::openapi();

    let unwritten = write_rules_of::<AuditedProduct>(&mut document);

    assert_eq!(
        unwritten.iter().map(Unwritten::pointer).collect::<Vec<_>>(),
        ["/allOf/1/properties/sku/allOf"]
    );
}
