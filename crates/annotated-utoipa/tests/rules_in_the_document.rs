//! `write_rules_of` writes the rules of a type into the schema that utoipa generated for it, and utoipa keeps every
//! keyword annotated writes.

// The types here are only documented, never read.
#![allow(dead_code)]

mod support;

use std::collections::HashMap;

use annotated::Deserialize;
use annotated_utoipa::{write_rules, write_rules_of};
use serde_json::json;
use support::schema_named;
use utoipa::openapi::OpenApi as Document;
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
    #[validate(length(max = 7), each(required))]
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
                    "maxProperties": 7
                },
                "ownerName": { "type": "string" },
                "address": { "$ref": "#/components/schemas/Address", "description": "Where the shop is." }
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

    let _ = write_rules(&mut document, "Page_Address", &<Page<Address> as Deserialize>::rules());

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
