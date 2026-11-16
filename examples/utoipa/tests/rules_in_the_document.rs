//! `write_rules_of` writes the rules of a type into the schema that utoipa generated for it.

// The types here are only documented, never read.
#![allow(dead_code)]

use annotated::Deserialize;
use annotated_example_utoipa::write_rules_of;
use serde_json::{Value, json};
use utoipa::openapi::OpenApi as Document;
use utoipa::{Modify, OpenApi, ToSchema};

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct NewMember {
    #[validate(required, length(max = 50))]
    full_name: String,
    #[validate(email)]
    email: String,
    #[validate(range(min = 18, max = 120))]
    age: u8,
    #[validate(length(max = 5), each(required, length(max = 20)))]
    interests: Vec<String>,
    #[validate(required, nested)]
    address: Option<Address>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct Address {
    #[validate(required, length(max = 80))]
    street: String,
    postal_code: Option<String>,
}

#[derive(Deserialize, ToSchema)]
struct Invitation {
    #[validate(email)]
    email: String,
}

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
struct AuditedInvitationsApi;

#[derive(Deserialize, ToSchema)]
struct Server {
    #[validate(ip)]
    address: String,
}

#[derive(OpenApi)]
#[openapi(components(schemas(Server)))]
struct ServersApi;

#[derive(OpenApi)]
#[openapi(components(schemas(NewMember, Address)))]
struct MembersApi;

struct RulesInTheSchemas;

impl Modify for RulesInTheSchemas {
    fn modify(&self, document: &mut Document) {
        let _ = write_rules_of::<NewMember>(document);
        let _ = write_rules_of::<Address>(document);
    }
}

#[derive(OpenApi)]
#[openapi(components(schemas(NewMember, Address)), modifiers(&RulesInTheSchemas))]
struct MembersApiWithItsRules;

fn schema_named(name: &str, document: &Document) -> Value {
    let document = serde_json::to_value(document).expect("the document becomes JSON");
    document["components"]["schemas"][name].clone()
}

#[test]
fn the_rules_of_a_type_are_written_into_its_schema() {
    let mut document = MembersApi::openapi();

    let _ = write_rules_of::<NewMember>(&mut document);

    assert_eq!(
        schema_named("NewMember", &document),
        json!({
            "type": "object",
            "required": ["fullName", "email", "age", "interests", "address"],
            "properties": {
                "fullName": { "type": "string", "minLength": 1, "maxLength": 50 },
                "email": { "type": "string", "format": "email" },
                "age": { "type": "integer", "format": "int32", "minimum": 18, "maximum": 120 },
                "interests": {
                    "type": "array",
                    "items": { "type": "string", "minLength": 1, "maxLength": 20 },
                    "maxItems": 5
                },
                "address": { "$ref": "#/components/schemas/Address" }
            }
        })
    );
}

#[test]
fn a_modifier_writes_the_rules_while_the_document_is_built() {
    let document = MembersApiWithItsRules::openapi();

    assert_eq!(
        schema_named("Address", &document),
        json!({
            "type": "object",
            "required": ["street"],
            "properties": {
                "street": { "type": "string", "minLength": 1, "maxLength": 80 },
                "postalCode": { "type": ["string", "null"] }
            }
        })
    );
}

#[test]
fn the_rules_of_a_struct_with_a_flattened_field_are_written_beside_its_other_fields() {
    let mut document = AuditedInvitationsApi::openapi();

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
fn each_rule_that_could_not_be_written_is_returned() {
    let mut document = ServersApi::openapi();

    let unwritten = write_rules_of::<Server>(&mut document);

    assert_eq!(
        unwritten.iter().map(ToString::to_string).collect::<Vec<_>>(),
        vec!["`ip` can't be written at `/properties/address`: no keyword says what the rule requires"]
    );
}

#[test]
#[should_panic(expected = "the document has no schema named `Invitation`")]
fn a_type_the_document_has_no_schema_for_is_reported() {
    let mut document = MembersApi::openapi();

    let _ = write_rules_of::<Invitation>(&mut document);
}
