//! What the document still doesn't say about a type's rules is returned, for the caller to decide about: a rule
//! annotated couldn't write, a type the document has no schema for, and what utoipa's schema can't hold. Each names
//! the schema it is about and where in it.

// The types here are only documented, never read.
#![allow(dead_code)]

use annotated::Deserialize;
use annotated_utoipa::{Reason, Unwritten, write_rules, write_rules_of};
use serde_json::json;
use utoipa::openapi::extensions::Extensions;
use utoipa::openapi::{ComponentsBuilder, ObjectBuilder, OpenApi as Document, OpenApiBuilder, Type};
use utoipa::{OpenApi, ToSchema};

#[derive(Deserialize, ToSchema)]
struct Server {
    #[validate(ip)]
    address: String,
}

#[derive(Deserialize, ToSchema)]
struct Invitation {
    #[validate(email)]
    email: String,
}

#[derive(Deserialize, ToSchema)]
struct Page<T> {
    #[validate(length(max = 100))]
    items: Vec<T>,
}

#[derive(OpenApi)]
#[openapi(components(schemas(Server, Page<Server>)))]
struct ServersApi;

#[derive(OpenApi)]
#[openapi(components(schemas(Nickname)))]
struct NicknamesApi;

#[derive(Deserialize)]
struct Labels {
    #[validate(length(max = 3))]
    tags: String,
}

#[derive(Deserialize)]
struct Shelf {
    #[serde(rename = "tags/all")]
    #[validate(length(max = 3))]
    tags: String,
}

#[derive(Deserialize, ToSchema)]
struct Nickname {
    #[schema(default = json!(null))]
    #[validate(length(max = 20))]
    nickname: Option<String>,
}

#[derive(Deserialize, ToSchema)]
struct Discount {
    #[validate(range(min = 0.0, max = 1.0))]
    share: f64,
}

#[derive(Deserialize, ToSchema)]
struct NewProduct {
    #[validate(length(max = 20), prefix = "SKU-", suffix = "-X")]
    sku: String,
}

#[derive(OpenApi)]
#[openapi(components(schemas(Discount, NewProduct)))]
struct ProductsApi;

#[derive(Deserialize)]
struct Member {
    #[validate(length(max = 50))]
    name: String,
}

/// A document whose schema `name` has one property, `property`, which is a string or a list of strings. utoipa reads
/// it as the schema of a list, which has no room for `maxLength`.
fn document_with_a_string_or_a_list(name: &str, property: &str) -> Document {
    serde_json::from_value(json!({
        "openapi": "3.1.0",
        "info": { "title": "Shelves", "version": "1.0.0" },
        "paths": {},
        "components": {
            "schemas": {
                name: {
                    "type": "object",
                    "properties": { property: { "type": ["array", "string"], "items": { "type": "string" } } }
                }
            }
        }
    }))
    .expect("utoipa reads the document")
}

/// A document whose schema `Member` has one property, `name`, a string with a `maxLength` of 100 among its extensions.
/// utoipa writes an extension after the keywords its schema has fields for, so it hides the `maxLength` of the field.
fn document_with_a_maximum_length_among_the_extensions() -> Document {
    let name = ObjectBuilder::new()
        .schema_type(Type::String)
        .extensions(Some(Extensions::from_iter([("maxLength", 100)])));
    let member = ObjectBuilder::new().property("name", name);
    OpenApiBuilder::new()
        .components(Some(ComponentsBuilder::new().schema("Member", member).build()))
        .build()
}

#[test]
fn a_rule_annotated_could_not_write_is_returned() {
    let mut document = ServersApi::openapi();

    let unwritten = write_rules_of::<Server>(&mut document);

    assert_eq!(
        unwritten.iter().map(ToString::to_string).collect::<Vec<_>>(),
        [
            "`ip` can't be written at `/properties/address` in the schema `Server`: no keyword says what the rule requires"
        ]
    );
}

#[test]
fn a_type_the_document_has_no_schema_for_is_returned() {
    let mut document = ServersApi::openapi();

    let unwritten = write_rules_of::<Invitation>(&mut document);

    assert_eq!(
        unwritten.iter().map(ToString::to_string).collect::<Vec<_>>(),
        ["the document has no schema named `Invitation`"]
    );
}

#[test]
fn a_generic_type_is_not_found_by_the_name_of_its_type_alone() {
    let mut document = ServersApi::openapi();

    let unwritten = write_rules_of::<Page<Server>>(&mut document);

    assert_eq!(
        unwritten.iter().map(ToString::to_string).collect::<Vec<_>>(),
        ["the document has no schema named `Page`"]
    );
}

#[test]
fn what_utoipa_s_schema_can_not_hold_is_returned() {
    let mut document = document_with_a_string_or_a_list("Labels", "tags");

    let unwritten = write_rules(&Labels::rules(), &mut document, "Labels");

    assert_eq!(
        unwritten.iter().map(ToString::to_string).collect::<Vec<_>>(),
        [
            "what was written at `/properties/tags/maxLength` in the schema `Labels` is left out: utoipa's schema \
             can't hold it"
        ]
    );
}

#[test]
fn what_utoipa_s_schema_does_not_write_as_it_was_set_is_returned() {
    let mut document = document_with_a_maximum_length_among_the_extensions();

    let unwritten = write_rules(&Member::rules(), &mut document, "Member");

    assert_eq!(
        unwritten.iter().map(ToString::to_string).collect::<Vec<_>>(),
        [
            "what was written at `/properties/name/maxLength` in the schema `Member` is left out: utoipa's schema \
             can't hold it"
        ]
    );
}

#[test]
fn a_default_of_null_utoipa_wrote_is_not_returned() {
    let mut document = NicknamesApi::openapi();

    let unwritten = write_rules_of::<Nickname>(&mut document);

    assert_eq!(unwritten, []);
}

#[test]
fn a_range_between_decimal_numbers_is_not_returned() {
    let mut document = ProductsApi::openapi();

    let unwritten = write_rules_of::<Discount>(&mut document);

    assert_eq!(unwritten, []);
}

#[test]
fn a_second_call_returns_what_the_first_returned() {
    let mut document = ProductsApi::openapi();
    let unwritten_by_the_first_call = write_rules_of::<NewProduct>(&mut document);

    let unwritten = write_rules_of::<NewProduct>(&mut document);

    assert_eq!(unwritten, unwritten_by_the_first_call);
}

#[test]
fn each_unwritten_rule_names_the_schema_it_is_about() {
    let mut document = ServersApi::openapi();

    let unwritten = write_rules_of::<Server>(&mut document);

    assert_eq!(
        unwritten.iter().map(Unwritten::schema_name).collect::<Vec<_>>(),
        ["Server"]
    );
}

#[test]
fn where_an_unwritten_rule_belongs_is_a_json_pointer() {
    let mut document = ServersApi::openapi();

    let unwritten = write_rules_of::<Server>(&mut document);

    assert_eq!(
        unwritten.iter().map(Unwritten::pointer).collect::<Vec<_>>(),
        ["/properties/address"]
    );
}

#[test]
fn a_property_name_in_the_pointer_of_what_utoipa_can_not_hold_is_escaped() {
    let mut document = document_with_a_string_or_a_list("Shelf", "tags/all");

    let unwritten = write_rules(&Shelf::rules(), &mut document, "Shelf");

    assert_eq!(
        unwritten.iter().map(Unwritten::pointer).collect::<Vec<_>>(),
        ["/properties/tags~1all/maxLength"]
    );
}

#[test]
fn the_whole_schema_is_unwritten_when_the_document_has_none_of_that_name() {
    let mut document = ServersApi::openapi();

    let unwritten = write_rules_of::<Invitation>(&mut document);

    assert_eq!(unwritten.iter().map(Unwritten::pointer).collect::<Vec<_>>(), [""]);
}

#[test]
fn a_type_the_document_has_no_schema_for_is_unwritten_for_that_reason() {
    let mut document = ServersApi::openapi();

    let unwritten = write_rules_of::<Invitation>(&mut document);

    assert_eq!(
        unwritten.iter().map(Unwritten::reason).collect::<Vec<_>>(),
        [&Reason::NoSchema]
    );
}

#[test]
fn what_utoipa_s_schema_can_not_hold_is_unwritten_for_that_reason() {
    let mut document = document_with_a_string_or_a_list("Labels", "tags");

    let unwritten = write_rules(&Labels::rules(), &mut document, "Labels");

    assert_eq!(
        unwritten.iter().map(Unwritten::reason).collect::<Vec<_>>(),
        [&Reason::NotHeld]
    );
}
