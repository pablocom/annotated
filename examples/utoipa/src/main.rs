//! Prints the OpenAPI document of an API that takes new members, with their rules in its schemas.

// The types here are only documented: an app would read them in its handlers.
#![allow(dead_code)]

use annotated::Deserialize;
use annotated_example_utoipa::write_rules_of;
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
    #[validate(pattern = r"^[0-9]{5}$")]
    postal_code: String,
}

struct RulesInTheSchemas;

impl Modify for RulesInTheSchemas {
    fn modify(&self, document: &mut Document) {
        write_rules_of::<NewMember>(document);
        write_rules_of::<Address>(document);
    }
}

#[derive(OpenApi)]
#[openapi(components(schemas(NewMember, Address)), modifiers(&RulesInTheSchemas))]
struct MembersApi;

fn main() {
    let document = MembersApi::openapi()
        .to_pretty_json()
        .expect("the document becomes JSON");
    println!("{document}");
}
