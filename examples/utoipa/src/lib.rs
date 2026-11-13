//! Writes the rules of a type into its schema in an OpenAPI document that utoipa generated, so the document says
//! what `#[validate(...)]` requires. Copy [`write_rules_of`] into your app.

use annotated::json_schema;
use utoipa::ToSchema;
use utoipa::openapi::OpenApi;

/// Writes the rules of `T` into its schema, among the components of `openapi`.
///
/// utoipa's schema becomes JSON for annotated to write into, and is read back from it.
///
/// # Panics
///
/// When the document has no schema for `T`. A type whose rules were meant to be documented would otherwise be left
/// out without a word.
pub fn write_rules_of<T>(openapi: &mut OpenApi)
where
    T: ToSchema + for<'de> annotated::Deserialize<'de>,
{
    let name = T::name();
    let schema = openapi
        .components
        .as_mut()
        .and_then(|components| components.schemas.get_mut(name.as_ref()))
        .unwrap_or_else(|| panic!("the document has no schema named `{name}`"));
    let mut schema_as_json = serde_json::to_value(&*schema).expect("a schema of utoipa's becomes JSON");
    if let Some(schema_object) = schema_as_json.as_object_mut() {
        json_schema::write_rules(&<T as annotated::Deserialize>::rules(), schema_object);
    }
    *schema = serde_json::from_value(schema_as_json).expect("utoipa reads back a schema with more keywords");
}
