//! `write_rules_of` writes the rules of a type into the schema that utoipa generated for it, and utoipa keeps every
//! keyword annotated writes. Nothing else in the document changes.

// The types here are only documented, never read.
#![allow(dead_code)]

mod support;

use std::collections::HashMap;

use annotated::{Deserialize, Validate};
use annotated_utoipa::{write_rules, write_rules_of};
use serde_json::json;
use support::schema_named;
use utoipa::openapi::{KnownFormat, ObjectBuilder, OpenApi as Document, RefOr, Schema, SchemaFormat, Type};
use utoipa::{Modify, OpenApi, ToSchema};

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct NewShop {
    #[validate(required, length(min = 3, max = 50))]
    name: String,
    #[validate(email)]
    email: String,
    #[validate(url)]
    website: String,
    #[validate(ipv4)]
    ipv4_address: String,
    #[validate(ipv6)]
    ipv6_address: String,
    #[validate(pattern = r"^[0-9]{5}$")]
    postal_code: String,
    #[validate(contains = "@")]
    handle: String,
    #[validate(range(min = 1, max = 20))]
    employees: u8,
    #[validate(range(min = 0.5))]
    rating: f64,
    #[validate(length(min = 1, max = 5), each(length(max = 20)))]
    tags: Vec<String>,
    #[validate(length(min = 1, max = 7), each(required))]
    opening_hours: HashMap<String, String>,
    #[validate(required)]
    owner_name: Option<String>,
    /// Where the shop is.
    #[validate(required, nested)]
    address: Option<Address>,
}

#[derive(Deserialize, ToSchema)]
struct Address {
    #[validate(required, length(max = 80))]
    street: String,
    city: Option<String>,
}

#[derive(OpenApi)]
#[openapi(components(schemas(NewShop, Address)))]
struct ShopsApi;

struct RulesInTheSchemas;

impl Modify for RulesInTheSchemas {
    fn modify(&self, document: &mut Document) {
        let _ = write_rules_of::<Address>(document);
    }
}

#[derive(OpenApi)]
#[openapi(components(schemas(Address)), modifiers(&RulesInTheSchemas))]
struct AddressesApi;

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct Audited {
    created_by: String,
}

#[derive(Deserialize, ToSchema)]
struct AuditedInvitation {
    #[validate(email)]
    email: String,
    #[serde(flatten)]
    #[validate(nested)]
    audited: Audited,
}

#[derive(OpenApi)]
#[openapi(components(schemas(AuditedInvitation, Audited)))]
struct InvitationsApi;

#[derive(Deserialize, ToSchema)]
struct Page<T> {
    #[validate(length(max = 100))]
    items: Vec<T>,
}

#[derive(OpenApi)]
#[openapi(components(schemas(Page<Address>, Address)))]
struct PagesApi;

#[derive(Validate, ToSchema)]
struct Member {
    #[validate(length(max = 50))]
    name: String,
}

#[derive(OpenApi)]
#[openapi(components(schemas(Member)))]
struct MembersApi;

#[derive(Deserialize, ToSchema)]
struct Catalogue {
    #[schema(max_items = 5)]
    #[validate(each(length(max = 20)))]
    tags: Vec<String>,
    #[schema(max_items = 5)]
    #[validate(length(min = 1))]
    attributes: Vec<serde_json::Value>,
}

#[derive(Deserialize, ToSchema)]
struct Event {
    #[validate(length(max = 20))]
    name: String,
    payload: serde_json::Value,
}

#[derive(Deserialize, ToSchema)]
struct Discount {
    #[validate(range(min = 0.0, max = 1.0))]
    share: f64,
}

#[derive(Deserialize, ToSchema)]
struct Ledger {
    #[validate(range(min = -9_007_199_254_740_993))]
    balance: i64,
    #[validate(range(max = 18_446_744_073_709_551_615))]
    total: u64,
}

#[derive(Deserialize, ToSchema)]
struct NewProduct {
    #[validate(length(max = 20), prefix = "SKU-", suffix = "-X")]
    sku: String,
}

#[derive(OpenApi)]
#[openapi(components(schemas(Catalogue, Event, Discount, Ledger, NewProduct)))]
struct CatalogueApi;

#[derive(Deserialize, ToSchema)]
struct Invitation {
    #[validate(email)]
    email: String,
}

#[derive(OpenApi)]
#[openapi(components(schemas(Invitation)))]
struct InvitationApi;

#[test]
fn every_keyword_annotated_writes_is_kept_in_the_document() {
    let mut document = ShopsApi::openapi();

    let _ = write_rules_of::<NewShop>(&mut document);

    assert_eq!(
        schema_named("NewShop", &document),
        json!({
            "type": "object",
            "required": [
                "name", "email", "website", "ipv4Address", "ipv6Address", "postalCode", "handle", "employees", "rating",
                "tags", "openingHours", "ownerName", "address"
            ],
            "properties": {
                "name": { "type": "string", "minLength": 3, "maxLength": 50 },
                "email": { "type": "string", "format": "email" },
                "website": { "type": "string", "format": "iri" },
                "ipv4Address": { "type": "string", "format": "ipv4" },
                "ipv6Address": { "type": "string", "format": "ipv6" },
                "postalCode": { "type": "string", "pattern": "^[0-9]{5}$" },
                "handle": { "type": "string", "pattern": "@" },
                "employees": { "type": "integer", "format": "int32", "minimum": 1, "maximum": 20 },
                "rating": { "type": "number", "format": "double", "minimum": 0.5 },
                "tags": {
                    "type": "array",
                    "items": { "type": "string", "maxLength": 20 },
                    "minItems": 1,
                    "maxItems": 5
                },
                "openingHours": {
                    "type": "object",
                    "additionalProperties": { "type": "string", "minLength": 1 },
                    "propertyNames": { "type": "string" },
                    "minProperties": 1,
                    "maxProperties": 7
                },
                "ownerName": { "type": ["string", "null"] },
                "address": {
                    "oneOf": [
                        { "$ref": "#/components/schemas/Address", "description": "Where the shop is." },
                        { "type": "null" }
                    ]
                }
            }
        })
    );
}

#[test]
fn a_modifier_writes_the_rules_while_the_document_is_built() {
    let document = AddressesApi::openapi();

    assert_eq!(
        schema_named("Address", &document),
        json!({
            "type": "object",
            "required": ["street"],
            "properties": {
                "street": { "type": "string", "minLength": 1, "maxLength": 80 },
                "city": { "type": ["string", "null"] }
            }
        })
    );
}

#[test]
fn the_rules_of_a_struct_with_a_flattened_field_are_written_beside_its_other_fields() {
    let mut document = InvitationsApi::openapi();

    let _ = write_rules_of::<AuditedInvitation>(&mut document);

    assert_eq!(
        schema_named("AuditedInvitation", &document),
        json!({
            "allOf": [
                { "$ref": "#/components/schemas/Audited" },
                {
                    "type": "object",
                    "required": ["email"],
                    "properties": { "email": { "type": "string", "format": "email" } }
                }
            ]
        })
    );
}

#[test]
fn the_rules_of_a_generic_type_are_written_into_the_schema_named_after_its_arguments() {
    let mut document = PagesApi::openapi();

    let _ = write_rules(&<Page<Address> as Deserialize>::rules(), &mut document, "Page_Address");

    assert_eq!(
        schema_named("Page_Address", &document),
        json!({
            "type": "object",
            "required": ["items"],
            "properties": {
                "items": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["street"],
                        "properties": {
                            "street": { "type": "string" },
                            "city": { "type": ["string", "null"] }
                        }
                    },
                    "maxItems": 100
                }
            }
        })
    );
}

#[test]
fn the_rules_of_a_type_that_only_derives_validate_are_written_by_naming_its_schema() {
    let mut document = MembersApi::openapi();

    let _ = write_rules(&<Member as Validate>::rules(), &mut document, "Member");

    assert_eq!(
        schema_named("Member", &document),
        json!({
            "type": "object",
            "required": ["name"],
            "properties": { "name": { "type": "string", "maxLength": 50 } }
        })
    );
}

#[test]
fn what_utoipa_wrote_is_kept_beside_the_rules() {
    let mut document = CatalogueApi::openapi();

    let _ = write_rules_of::<Catalogue>(&mut document);

    assert_eq!(
        schema_named("Catalogue", &document),
        json!({
            "type": "object",
            "required": ["tags", "attributes"],
            "properties": {
                "tags": { "type": "array", "items": { "type": "string", "maxLength": 20 }, "maxItems": 5 },
                "attributes": { "type": "array", "items": {}, "maxItems": 5, "minItems": 1 }
            }
        })
    );
}

#[test]
fn the_rules_of_a_struct_with_a_field_of_any_value_are_written_and_that_field_is_left_as_it_is() {
    let mut document = CatalogueApi::openapi();

    let _ = write_rules_of::<Event>(&mut document);

    assert_eq!(
        schema_named("Event", &document),
        json!({
            "type": "object",
            "required": ["name", "payload"],
            "properties": {
                "name": { "type": "string", "maxLength": 20 },
                "payload": {}
            }
        })
    );
}

#[test]
fn a_range_between_decimal_numbers_is_written() {
    let mut document = CatalogueApi::openapi();

    let _ = write_rules_of::<Discount>(&mut document);

    assert_eq!(
        schema_named("Discount", &document)["properties"]["share"],
        json!({ "type": "number", "format": "double", "minimum": 0, "maximum": 1 })
    );
}

#[test]
fn a_range_is_written_exactly_where_its_bounds_are_beyond_what_a_decimal_number_holds() {
    let mut document = CatalogueApi::openapi();

    let _ = write_rules_of::<Ledger>(&mut document);

    assert_eq!(
        schema_named("Ledger", &document)["properties"],
        json!({
            "balance": { "type": "integer", "format": "int64", "minimum": -9_007_199_254_740_993_i64 },
            "total": { "type": "integer", "format": "int64", "minimum": 0, "maximum": 18_446_744_073_709_551_615_u64 }
        })
    );
}

#[test]
fn a_format_is_held_as_the_one_utoipa_knows_by_that_name() {
    let mut document = InvitationApi::openapi();

    let _ = write_rules_of::<Invitation>(&mut document);

    let email = ObjectBuilder::new()
        .schema_type(Type::String)
        .format(Some(SchemaFormat::KnownFormat(KnownFormat::Email)));
    assert_eq!(
        document.components.expect("the document has components").schemas["Invitation"],
        RefOr::T(Schema::Object(
            ObjectBuilder::new().property("email", email).required("email").build()
        ))
    );
}

#[test]
fn a_second_call_changes_nothing_in_the_document() {
    let mut document = CatalogueApi::openapi();
    let _ = write_rules_of::<NewProduct>(&mut document);
    let after_the_first_call = serde_json::to_value(&document).expect("the document becomes JSON");

    let _ = write_rules_of::<NewProduct>(&mut document);

    assert_eq!(
        serde_json::to_value(&document).expect("the document becomes JSON"),
        after_the_first_call
    );
}
