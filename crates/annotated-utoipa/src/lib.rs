//! Writes the rules of [annotated](https://docs.rs/annotated)'s types into the OpenAPI document that
//! [utoipa](https://docs.rs/utoipa) generates, so the document says what `#[validate(...)]` requires:
//! `length(max = 50)` on a field becomes `maxLength: 50` in its schema.
//!
//! utoipa generates each type's schema from its fields, and knows nothing of its rules. [`write_rules_of`] finds the
//! schema utoipa generated for a type, among the document's components, and writes the type's rules into it with
//! [`annotated::json_schema::write_rules`], whose documentation lists what each rule writes. The type derives
//! `annotated::Deserialize`, which describes its rules with annotated's `json-schema` feature, and `utoipa::ToSchema`.
//!
//! ```
//! use annotated::Deserialize;
//! use utoipa::{OpenApi, ToSchema};
//!
//! #[derive(Deserialize, ToSchema)]
//! struct NewMember {
//!     #[validate(required, length(max = 50))]
//!     name: String,
//! }
//!
//! #[derive(OpenApi)]
//! #[openapi(components(schemas(NewMember)))]
//! struct MembersApi;
//!
//! let mut document = MembersApi::openapi();
//!
//! let unwritten = annotated_utoipa::write_rules_of::<NewMember>(&mut document);
//!
//! assert_eq!(unwritten, []);
//! let name = serde_json::to_value(&document).unwrap()["components"]["schemas"]["NewMember"]["properties"]["name"].clone();
//! assert_eq!(name, serde_json::json!({ "type": "string", "minLength": 1, "maxLength": 50 }));
//! ```
//!
//! # Where to call it
//!
//! Call it once for each type, where the document is built:
//!
//! - **With `#[derive(OpenApi)]`**, from a [`Modify`](utoipa::Modify) in its `modifiers`.
//! - **With utoipa-axum's `OpenApiRouter`**, on the document that `split_for_parts()` returns. A `Modify` runs before
//!   the routes add their schemas, so it would find none of them.
//!
//! A type `nested` in another has rules of its own, so it takes a call of its own.
//!
//! # What the document still doesn't say
//!
//! Each call returns what the document doesn't say about the type's rules, as an [`Unwritten`]: a rule that no
//! keyword says, such as `ip`, a type the document has no schema for, or a keyword utoipa's schema can't hold. Which
//! of them a document can do without is the caller's choice. An app that wants every rule in its document can
//! panic on any, so a rule missing from the document is noticed when the app starts.
//!
//! # How the schema is written
//!
//! utoipa's schema becomes JSON, annotated writes the rules into it, and utoipa reads it back. A schema of utoipa's
//! has room for one `pattern` and no `allOf` beside its `type`, so a second pattern, which annotated requires
//! through `allOf`, is required through an `allOf` of its own instead: the schema's keywords are its first member,
//! and each other member takes the schema's `type`. Whatever utoipa still leaves out when it reads the schema back is
//! returned as [`Unwritten::LeftOut`].
//!
//! # Other types
//!
//! - **A generic type** is named after its arguments in the document, `Page_Address` for `Page<Address>`, and
//!   [`ToSchema::name`] only knows `Page`. Name its schema with [`write_rules`].
//! - **A type that derives `annotated::Validate`** and is read by serde's own `Deserialize` has its rules in
//!   `Validate::rules`. Pass them to [`write_rules`].
//!
//! # Versions
//!
//! This crate works with utoipa 6. A new major version of utoipa comes with a new version of this crate.

use std::fmt;

use annotated::Rules;
use annotated::json_schema::{self, UnwrittenRule};
use serde_json::{Map, Value};
use utoipa::ToSchema;
use utoipa::openapi::schema::Schema;
use utoipa::openapi::{OpenApi, RefOr};

/// Writes the rules of `T` into the schema utoipa generated for it, among the components of `openapi`, and returns
/// what the document still doesn't say about them.
///
/// The schema is the one named [`ToSchema::name`]. Writing the same rules a second time changes nothing.
#[must_use = "a rule left out of the document is only reported here"]
pub fn write_rules_of<T>(openapi: &mut OpenApi) -> Vec<Unwritten>
where
    T: ToSchema + for<'de> annotated::Deserialize<'de>,
{
    write_rules(openapi, &T::name(), &<T as annotated::Deserialize>::rules())
}

/// Writes `rules` into the schema named `schema_name` among the components of `openapi`, and returns what the
/// document still doesn't say about them.
///
/// It is [`write_rules_of`] for a schema that isn't named after its type, such as a generic type's
/// (`Page_Address`), and for rules from elsewhere, such as `Validate::rules`.
#[must_use = "a rule left out of the document is only reported here"]
pub fn write_rules(openapi: &mut OpenApi, schema_name: &str, rules: &Rules) -> Vec<Unwritten> {
    let Some(schema) = openapi
        .components
        .as_mut()
        .and_then(|components| components.schemas.get_mut(schema_name))
    else {
        return vec![Unwritten::NoSchema(schema_name.to_owned())];
    };
    let mut with_rules = json_as_utoipa_reads_it(schema);
    let unwritten_rules = json_schema::write_rules(rules, &mut with_rules);
    require_all_of_on_its_own(&mut with_rules);
    let with_rules = Value::Object(with_rules);

    if let Some(schema_with_rules) = read_back(&with_rules) {
        *schema = schema_with_rules;
    }
    let mut unwritten: Vec<Unwritten> = unwritten_rules.into_iter().map(Unwritten::Rule).collect();
    left_out(&with_rules, Some(&json_of(schema)), "", &mut unwritten);
    unwritten
}

/// Something the document doesn't say about a type's rules, which [`write_rules_of`] and [`write_rules`] return.
///
/// New kinds may be added without a breaking change, so a `match` on it needs a wildcard arm.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Unwritten {
    /// The document has no schema of this name among its components, so it says none of the type's rules.
    NoSchema(String),
    /// A rule annotated couldn't write into the type's schema: where its keyword belongs, and why it isn't there.
    Rule(UnwrittenRule),
    /// What annotated wrote at this JSON Pointer in the type's schema, which utoipa's schema can't hold, so the
    /// document leaves it out.
    LeftOut(String),
}

impl fmt::Display for Unwritten {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoSchema(name) => write!(formatter, "the document has no schema named `{name}`"),
            Self::Rule(rule) => rule.fmt(formatter),
            Self::LeftOut(pointer) => write!(
                formatter,
                "what was written at `{pointer}` is left out: utoipa's schema can't hold it"
            ),
        }
    }
}

// utoipa leaves a few things out of its own schemas when it reads them back, such as a `default` of `null`. Starting
// from what it reads back keeps those from being reported as rules left out.
fn json_as_utoipa_reads_it(schema: &RefOr<Schema>) -> Map<String, Value> {
    let json = json_of(schema);
    let Value::Object(json) = read_back(&json).map_or(json, |schema| json_of(&schema)) else {
        unreachable!("utoipa writes a schema as a JSON object");
    };
    json
}

fn json_of(schema: &RefOr<Schema>) -> Value {
    serde_json::to_value(schema).expect("utoipa's schema becomes JSON")
}

fn read_back(json: &Value) -> Option<RefOr<Schema>> {
    serde_json::from_value(json.clone()).ok()
}

/// utoipa reads a schema with a `type` as a schema of that type, which has no `allOf`. So an `allOf` beside a `type`
/// is made to stand on its own: the schema's other keywords become its first member, and each member takes the
/// `type`, which an `allOf` member needs to be read.
fn require_all_of_on_its_own(schema: &mut Map<String, Value>) {
    for subschema in subschemas(schema) {
        require_all_of_on_its_own(subschema);
    }
    let Some(schema_type) = schema.get("type").cloned() else {
        return;
    };
    let Some(Value::Array(members)) = schema.get_mut("allOf") else {
        return;
    };
    let mut members = std::mem::take(members);
    schema.remove("allOf");
    for member in &mut members {
        if let Value::Object(member) = member {
            member.entry("type").or_insert_with(|| schema_type.clone());
        }
    }
    members.insert(0, Value::Object(std::mem::take(schema)));
    schema.insert("allOf".to_owned(), Value::Array(members));
}

/// The subschemas annotated writes rules into: the properties, the `allOf` members, and the schema of the elements.
fn subschemas(schema: &mut Map<String, Value>) -> Vec<&mut Map<String, Value>> {
    let mut subschemas = Vec::new();
    for (keyword, value) in schema {
        match (keyword.as_str(), value) {
            ("properties", Value::Object(properties)) => {
                subschemas.extend(properties.values_mut().filter_map(Value::as_object_mut));
            }
            ("allOf", Value::Array(members)) => subschemas.extend(members.iter_mut().filter_map(Value::as_object_mut)),
            ("items" | "additionalProperties", Value::Object(elements)) => subschemas.push(elements),
            _ => {}
        }
    }
    subschemas
}

/// Adds a [`Unwritten::LeftOut`] for each JSON Pointer, from `pointer` down, where `written` has something that
/// `held` doesn't.
fn left_out(written: &Value, held: Option<&Value>, pointer: &str, unwritten: &mut Vec<Unwritten>) {
    match (written, held) {
        (Value::Object(written), Some(Value::Object(held))) => {
            for (keyword, value) in written {
                let pointer = format!("{pointer}/{}", keyword.replace('~', "~0").replace('/', "~1"));
                left_out(value, held.get(keyword), &pointer, unwritten);
            }
        }
        (written, Some(held)) if written == held => {}
        _ => unwritten.push(Unwritten::LeftOut(pointer.to_owned())),
    }
}
