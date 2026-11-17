//! What the document still doesn't say about a type's rules is returned, for the caller to decide about: a rule
//! annotated couldn't write, a type the document has no schema for, and what utoipa's schema can't hold.

// The types here are only documented, never read.
#![allow(dead_code)]

use annotated::Deserialize;
use annotated_utoipa::{Unwritten, write_rules, write_rules_of};
use serde_json::json;
use utoipa::openapi::OpenApi as Document;
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

#[test]
fn a_rule_annotated_could_not_write_is_returned() {
    let mut document = ServersApi::openapi();

    let unwritten = write_rules_of::<Server>(&mut document);

    assert_eq!(
        unwritten.iter().map(ToString::to_string).collect::<Vec<_>>(),
        ["`ip` can't be written at `/properties/address`: no keyword says what the rule requires"]
    );
}

#[test]
fn a_type_the_document_has_no_schema_for_is_returned() {
    let mut document = ServersApi::openapi();

    let unwritten = write_rules_of::<Invitation>(&mut document);

    assert_eq!(unwritten, [Unwritten::NoSchema("Invitation".to_owned())]);
}

#[test]
fn a_generic_type_is_not_found_by_the_name_of_its_type_alone() {
    let mut document = ServersApi::openapi();

    let unwritten = write_rules_of::<Page<Server>>(&mut document);

    assert_eq!(unwritten, [Unwritten::NoSchema("Page".to_owned())]);
}

#[test]
fn what_utoipa_s_schema_can_not_hold_is_returned() {
    let mut document = document_with_a_string_or_a_list("Labels", "tags");

    let unwritten = write_rules(&mut document, "Labels", &Labels::rules());

    assert_eq!(unwritten, [Unwritten::LeftOut("/properties/tags/maxLength".to_owned())]);
}

#[test]
fn a_property_name_in_the_pointer_of_what_utoipa_can_not_hold_is_escaped() {
    let mut document = document_with_a_string_or_a_list("Shelf", "tags/all");

    let unwritten = write_rules(&mut document, "Shelf", &Shelf::rules());

    assert_eq!(
        unwritten,
        [Unwritten::LeftOut("/properties/tags~1all/maxLength".to_owned())]
    );
}

#[test]
fn what_utoipa_leaves_out_of_its_own_schema_is_not_returned() {
    let mut document = NicknamesApi::openapi();

    let unwritten = write_rules_of::<Nickname>(&mut document);

    assert_eq!(unwritten, []);
}

#[test]
fn a_type_the_document_has_no_schema_for_reads_as_such() {
    let unwritten = Unwritten::NoSchema("Invitation".to_owned());

    assert_eq!(unwritten.to_string(), "the document has no schema named `Invitation`");
}

#[test]
fn what_utoipa_s_schema_can_not_hold_reads_as_such() {
    let unwritten = Unwritten::LeftOut("/properties/tags/maxLength".to_owned());

    assert_eq!(
        unwritten.to_string(),
        "what was written at `/properties/tags/maxLength` is left out: utoipa's schema can't hold it"
    );
}
