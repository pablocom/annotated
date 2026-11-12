use std::collections::BTreeMap;

use annotated::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Product {
    #[validate(required)]
    name: String,
    #[serde(flatten)]
    attributes: BTreeMap<String, String>,
}

fn main() {}
