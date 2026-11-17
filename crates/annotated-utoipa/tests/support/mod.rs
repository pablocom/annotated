use serde_json::Value;
use utoipa::openapi::OpenApi;

/// The schema named `name` among the components of `document`, as JSON.
pub fn schema_named(name: &str, document: &OpenApi) -> Value {
    let document = serde_json::to_value(document).expect("the document becomes JSON");
    document["components"]["schemas"][name].clone()
}
