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
//! Each call returns what the document doesn't say about the type's rules, each as an [`Unwritten`]: the schema it is
//! about, where in it, and the [`Reason`]. It is a rule that no keyword says, such as `ip`, a type the document has no
//! schema for, or a keyword utoipa's schema can't hold. Which of them a document can do without is the caller's
//! choice. An app that wants every rule in its document can panic on any, so a rule missing from the document is
//! noticed when the app starts.
//!
//! An empty result means that every rule annotated describes as data is in the document. It doesn't mean that the
//! document says everything the type requires: annotated describes the rules on a struct's fields only, and not those
//! of an enum, of a flattened field or of a `nested` type, nor a rule across fields or `custom(...)`.
//!
//! # How the schema is written
//!
//! annotated's [`json_schema::write_rules`] is the one place that knows what keyword each rule writes, and it writes
//! into JSON. So the schema utoipa generated becomes JSON, annotated writes the rules into a copy, and each keyword
//! the copy adds or tightens is set in utoipa's own schema, at the same place: in the [`Object`] of a property, the
//! `items` of an [`Array`], the `additionalProperties` of a map, or a member of an [`AllOf`], which is how utoipa
//! documents a struct with a flattened field. Nothing else in the document changes.
//!
//! - **A keyword utoipa's schema has no field for** is left out, and returned for [`Reason::NotHeld`]. One is the
//!   `allOf` that annotated requires a second `pattern` through, since utoipa's schema of a string has room for one
//!   `pattern` and no `allOf`. The property stays a string with its first pattern, which every client generated from
//!   the document reads.
//! - **The result is checked.** Once the keywords are set, utoipa's schema becomes JSON again and is compared with
//!   annotated's copy, numbers as numbers, since utoipa writes `1.0` as `1`. Whatever still differs is returned too.
//!
//! Writing the same rules a second time changes nothing, and returns what the first time returned.
//!
//! # Other types
//!
//! [`write_rules`] takes the rules and the name of the schema, for the types [`write_rules_of`] can't name:
//!
//! - **A generic type** is named after its arguments in the document, `Page_Address` for `Page<Address>`, and
//!   [`ToSchema::name`] only knows `Page`.
//! - **A type that derives `annotated::Validate`** and is read by serde's own `Deserialize` has its rules in
//!   `Validate::rules`.
//!
//! # Versions
//!
//! This crate works with utoipa 6. A new major version of utoipa comes with a new version of this crate.
//!
//! [`Object`]: utoipa::openapi::schema::Object
//! [`Array`]: utoipa::openapi::schema::Array
//! [`AllOf`]: utoipa::openapi::schema::AllOf

use std::fmt;

use annotated::Rules;
use annotated::json_schema::{self, UnwrittenRule};
use serde_json::{Map, Number, Value};
use utoipa::ToSchema;
use utoipa::openapi::schema::{AdditionalProperties, AllOf, Array, ArrayItems, Object, Schema, SchemaFormat};
use utoipa::openapi::{OpenApi, RefOr};

/// Writes the rules of `T` into the schema utoipa generated for it, among the components of `openapi`, and returns
/// what the document still doesn't say about them.
///
/// The schema is the one named [`ToSchema::name`], and the rules are those `annotated::Deserialize::rules` describes.
/// Only the keywords the rules add or tighten change in the document. Writing the same rules a second time changes
/// nothing, and returns what the first time returned.
///
/// An empty result means that every rule annotated describes as data is in the document. It doesn't mean that the
/// document says everything the type requires: annotated describes the rules on a struct's fields only, and not those
/// of an enum, of a flattened field or of a `nested` type, nor a rule across fields or `custom(...)`.
#[must_use = "a rule left out of the document is only reported here"]
pub fn write_rules_of<T>(openapi: &mut OpenApi) -> Vec<Unwritten>
where
    T: ToSchema + for<'de> annotated::Deserialize<'de>,
{
    write_rules(&<T as annotated::Deserialize>::rules(), openapi, &T::name())
}

/// Writes `rules` into the schema named `schema_name` among the components of `openapi`, and returns what the
/// document still doesn't say about them.
///
/// Only the keywords the rules add or tighten change in the document. Writing the same rules a second time changes
/// nothing, and returns what the first time returned.
///
/// An empty result means that every rule annotated describes as data is in the document. It doesn't mean that the
/// document says everything the type requires: annotated describes the rules on a struct's fields only, and not those
/// of an enum, of a flattened field or of a `nested` type, nor a rule across fields or `custom(...)`.
///
/// It is [`write_rules_of`] for a schema that isn't named after its type, such as a generic type's, which is named
/// after its arguments:
///
/// ```
/// use annotated::Deserialize;
/// use utoipa::{OpenApi, ToSchema};
///
/// #[derive(Deserialize, ToSchema)]
/// struct Page<T> {
///     #[validate(length(max = 100))]
///     items: Vec<T>,
/// }
///
/// #[derive(Deserialize, ToSchema)]
/// struct Address {
///     street: String,
/// }
///
/// #[derive(OpenApi)]
/// #[openapi(components(schemas(Page<Address>, Address)))]
/// struct AddressesApi;
///
/// let mut document = AddressesApi::openapi();
///
/// let rules = <Page<Address> as Deserialize>::rules();
/// let unwritten = annotated_utoipa::write_rules(&rules, &mut document, "Page_Address");
///
/// assert_eq!(unwritten, []);
/// ```
///
/// And for rules from elsewhere, such as those of a type that derives `annotated::Validate` and is read by serde's own
/// `Deserialize`:
///
/// ```
/// use annotated::Validate;
/// use utoipa::{OpenApi, ToSchema};
///
/// #[derive(Validate, ToSchema)]
/// struct NewMember {
///     #[validate(length(max = 50))]
///     name: String,
/// }
///
/// #[derive(OpenApi)]
/// #[openapi(components(schemas(NewMember)))]
/// struct MembersApi;
///
/// let mut document = MembersApi::openapi();
///
/// let unwritten = annotated_utoipa::write_rules(&<NewMember as Validate>::rules(), &mut document, "NewMember");
///
/// assert_eq!(unwritten, []);
/// ```
#[must_use = "a rule left out of the document is only reported here"]
pub fn write_rules(rules: &Rules, openapi: &mut OpenApi, schema_name: &str) -> Vec<Unwritten> {
    let Some(schema) = openapi
        .components
        .as_mut()
        .and_then(|components| components.schemas.get_mut(schema_name))
    else {
        return vec![Unwritten::new(schema_name, String::new(), Reason::NoSchema)];
    };
    let as_generated = json_of(schema);
    let mut with_rules = as_generated.clone();
    let unwritten_rules = json_schema::write_rules(rules, &mut with_rules);
    let mut not_held = write_changes(&as_generated, &with_rules, schema);
    not_held.extend(places_still_different(schema, &with_rules, &not_held));

    let unwritten_rules = unwritten_rules
        .into_iter()
        .map(|rule| Unwritten::new(schema_name, rule.pointer().to_owned(), Reason::Rule(rule)));
    let not_held = not_held
        .iter()
        .map(|place| Unwritten::new(schema_name, json_pointer(place), Reason::NotHeld));
    unwritten_rules.chain(not_held).collect()
}

/// Something the document doesn't say about a type's rules, which [`write_rules_of`] and [`write_rules`] return: the
/// schema it is about, where in it, and why.
///
/// ```
/// use annotated::Deserialize;
/// use annotated_utoipa::Reason;
/// use utoipa::{OpenApi, ToSchema};
///
/// #[derive(Deserialize, ToSchema)]
/// struct Server {
///     #[validate(ip)]
///     address: String,
/// }
///
/// #[derive(OpenApi)]
/// #[openapi(components(schemas(Server)))]
/// struct ServersApi;
///
/// let mut document = ServersApi::openapi();
///
/// let unwritten = annotated_utoipa::write_rules_of::<Server>(&mut document);
///
/// assert_eq!(unwritten[0].schema_name(), "Server");
/// assert_eq!(unwritten[0].pointer(), "/properties/address");
/// assert!(matches!(unwritten[0].reason(), Reason::Rule(rule) if rule.problem().code() == "ip"));
/// assert_eq!(
///     unwritten[0].to_string(),
///     "`ip` can't be written at `/properties/address` in the schema `Server`: no keyword says what the rule requires"
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Unwritten {
    schema_name: String,
    pointer: String,
    reason: Reason,
}

impl Unwritten {
    fn new(schema_name: &str, pointer: String, reason: Reason) -> Self {
        Self {
            schema_name: schema_name.to_owned(),
            pointer,
            reason,
        }
    }

    /// The name of the schema it is about, among the document's components: the one [`write_rules_of`] looked for,
    /// or the one given to [`write_rules`].
    #[must_use]
    pub fn schema_name(&self) -> &str {
        &self.schema_name
    }

    /// Where in that schema, as an RFC 6901 JSON Pointer: where a rule's keyword belongs, `/properties/address`, or
    /// what annotated wrote that utoipa's schema can't hold, `/properties/sku/allOf`. It is empty, which points at the
    /// whole schema, when the document has no schema of that name.
    #[must_use]
    pub fn pointer(&self) -> &str {
        &self.pointer
    }

    /// Why the document doesn't say it.
    #[must_use]
    pub fn reason(&self) -> &Reason {
        &self.reason
    }
}

impl fmt::Display for Unwritten {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            schema_name,
            pointer,
            reason,
        } = self;
        match reason {
            Reason::NoSchema => write!(formatter, "the document has no schema named `{schema_name}`"),
            Reason::Rule(rule) => write!(
                formatter,
                "`{}` can't be written at `{pointer}` in the schema `{schema_name}`: {}",
                rule.problem().code(),
                rule.reason()
            ),
            Reason::NotHeld => write!(
                formatter,
                "what was written at `{pointer}` in the schema `{schema_name}` is left out: utoipa's schema can't hold it"
            ),
        }
    }
}

/// Why the document doesn't say something about a type's rules.
///
/// New reasons may be added without a breaking change, so a `match` on it needs a wildcard arm.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Reason {
    /// The document has no schema of that name among its components, so it says none of the type's rules.
    NoSchema,
    /// annotated couldn't write the rule into the schema. [`UnwrittenRule::reason`] says why.
    Rule(UnwrittenRule),
    /// annotated wrote a keyword there that utoipa's schema has no room for, or writes otherwise than it was set, so
    /// the document leaves it out.
    NotHeld,
}

/// Where something is in a schema: the keys and indices that lead to it from the schema's root.
type Place = Vec<String>;

/// A place where two schemas differ, and what the second one has there.
struct Difference<'a> {
    place: Place,
    value: Option<&'a Value>,
}

/// utoipa's schema has nowhere to hold a keyword: no field for it, no subschema to walk to, or a value of another
/// type than its field's.
struct NoRoom;

fn json_of(schema: &RefOr<Schema>) -> Map<String, Value> {
    match serde_json::to_value(schema) {
        Ok(Value::Object(json)) => json,
        _ => unreachable!("utoipa writes a schema as a JSON object"),
    }
}

/// Sets in `schema` each keyword that `with_rules` adds to `as_generated` or changes in it, and returns the places of
/// those it has no room for.
fn write_changes(
    as_generated: &Map<String, Value>,
    with_rules: &Map<String, Value>,
    schema: &mut RefOr<Schema>,
) -> Vec<Place> {
    let mut not_held = Vec::new();
    for change in differences(as_generated, with_rules) {
        let is_held = change
            .value
            .is_some_and(|value| write_at(&change.place, value, schema).is_ok());
        if !is_held {
            not_held.push(change.place);
        }
    }
    not_held
}

/// The places where `schema`, as JSON, still differs from `with_rules`, apart from those in `not_held` already.
fn places_still_different(schema: &RefOr<Schema>, with_rules: &Map<String, Value>, not_held: &[Place]) -> Vec<Place> {
    differences(&json_of(schema), with_rules)
        .into_iter()
        .map(|difference| difference.place)
        .filter(|place| !not_held.iter().any(|returned| place.starts_with(returned)))
        .collect()
}

fn json_pointer(place: &[String]) -> String {
    let mut pointer = String::new();
    for key in place {
        pointer.push('/');
        pointer.push_str(&key.replace('~', "~0").replace('/', "~1"));
    }
    pointer
}

/// Each place where `left` and `right` differ, as deep as both are objects, or arrays of the same length. Numbers are
/// compared as numbers, since utoipa writes `1.0` as `1`.
fn differences<'a>(left: &Map<String, Value>, right: &'a Map<String, Value>) -> Vec<Difference<'a>> {
    let mut differences = Vec::new();
    collect_differences_in_keys(left, right, &mut Vec::new(), &mut differences);
    differences
}

fn collect_differences_in_keys<'a>(
    left: &Map<String, Value>,
    right: &'a Map<String, Value>,
    place: &mut Place,
    differences: &mut Vec<Difference<'a>>,
) {
    let keys_only_on_the_right = right.keys().filter(|key| !left.contains_key(*key));
    for key in left.keys().chain(keys_only_on_the_right) {
        place.push(key.clone());
        match (left.get(key), right.get(key)) {
            (Some(left), Some(right)) => collect_differences(left, right, place, differences),
            (_, value) => differences.push(Difference {
                place: place.clone(),
                value,
            }),
        }
        place.pop();
    }
}

fn collect_differences<'a>(left: &Value, right: &'a Value, place: &mut Place, differences: &mut Vec<Difference<'a>>) {
    match (left, right) {
        (Value::Object(left), Value::Object(right)) => collect_differences_in_keys(left, right, place, differences),
        (Value::Array(left), Value::Array(right)) if left.len() == right.len() => {
            for (index, (left, right)) in left.iter().zip(right).enumerate() {
                place.push(index.to_string());
                collect_differences(left, right, place, differences);
                place.pop();
            }
        }
        (Value::Number(left), Value::Number(right)) if is_same_number(left, right) => {}
        _ if left == right => {}
        _ => differences.push(Difference {
            place: place.clone(),
            value: Some(right),
        }),
    }
}

fn is_same_number(left: &Number, right: &Number) -> bool {
    match (left.as_i128(), right.as_i128()) {
        (Some(left), Some(right)) => left == right,
        _ => left.as_f64() == right.as_f64(),
    }
}

/// Sets what is at `place` in `schema` to `value`, through the typed subschemas that lead there.
fn write_at(place: &[String], value: &Value, schema: &mut RefOr<Schema>) -> Result<(), NoRoom> {
    let RefOr::T(schema) = schema else {
        return Err(NoRoom);
    };
    match schema {
        Schema::Object(object) => write_into_object(place, value, object),
        Schema::Array(array) => write_into_array(place, value, array),
        Schema::AllOf(all_of) => write_into_member(place, value, all_of),
        _ => Err(NoRoom),
    }
}

fn write_into_object(place: &[String], value: &Value, object: &mut Object) -> Result<(), NoRoom> {
    match place {
        [keyword] => set_keyword_of_object(keyword, value, object),
        [properties, name, place @ ..] if properties == "properties" => {
            let property = object.properties.get_mut(name.as_str()).ok_or(NoRoom)?;
            write_at(place, value, property)
        }
        [additional_properties, place @ ..] if additional_properties == "additionalProperties" => {
            match object.additional_properties.as_deref_mut() {
                Some(AdditionalProperties::RefOr(values)) => write_at(place, value, values),
                Some(AdditionalProperties::FreeForm(_)) | None => Err(NoRoom),
            }
        }
        _ => Err(NoRoom),
    }
}

fn write_into_array(place: &[String], value: &Value, array: &mut Array) -> Result<(), NoRoom> {
    match place {
        [keyword] => set_keyword_of_array(keyword, value, array),
        [items, place @ ..] if items == "items" => match &mut array.items {
            ArrayItems::RefOrSchema(elements) => write_at(place, value, elements),
            ArrayItems::False => Err(NoRoom),
        },
        _ => Err(NoRoom),
    }
}

fn write_into_member(place: &[String], value: &Value, all_of: &mut AllOf) -> Result<(), NoRoom> {
    let [all_of_keyword, index, place @ ..] = place else {
        return Err(NoRoom);
    };
    if all_of_keyword != "allOf" {
        return Err(NoRoom);
    }
    let member = index
        .parse::<usize>()
        .ok()
        .and_then(|index| all_of.items.get_mut(index))
        .ok_or(NoRoom)?;
    write_at(place, value, member)
}

/// Sets each keyword annotated writes that utoipa's `Object` has a field for.
fn set_keyword_of_object(keyword: &str, value: &Value, object: &mut Object) -> Result<(), NoRoom> {
    match keyword {
        "minLength" => object.min_length = Some(count(value)?),
        "maxLength" => object.max_length = Some(count(value)?),
        "pattern" => object.pattern = Some(text(value)?),
        "format" => object.format = Some(format(value)?),
        "minimum" => object.minimum = Some(number(value)?),
        "maximum" => object.maximum = Some(number(value)?),
        "minProperties" => object.min_properties = Some(count(value)?),
        "maxProperties" => object.max_properties = Some(count(value)?),
        "required" => object.required = names(value)?,
        _ => return Err(NoRoom),
    }
    Ok(())
}

/// Sets each keyword annotated writes that utoipa's `Array` has a field for.
fn set_keyword_of_array(keyword: &str, value: &Value, array: &mut Array) -> Result<(), NoRoom> {
    match keyword {
        "minItems" => array.min_items = Some(count(value)?),
        "maxItems" => array.max_items = Some(count(value)?),
        _ => return Err(NoRoom),
    }
    Ok(())
}

fn count(value: &Value) -> Result<usize, NoRoom> {
    value
        .as_u64()
        .and_then(|count| usize::try_from(count).ok())
        .ok_or(NoRoom)
}

fn number(value: &Value) -> Result<utoipa::Number, NoRoom> {
    let Value::Number(number) = value else {
        return Err(NoRoom);
    };
    if let Some(natural) = number.as_u64() {
        return usize::try_from(natural).map(utoipa::Number::UInt).map_err(|_| NoRoom);
    }
    if let Some(integer) = number.as_i64() {
        return isize::try_from(integer).map(utoipa::Number::Int).map_err(|_| NoRoom);
    }
    number.as_f64().map(utoipa::Number::Float).ok_or(NoRoom)
}

fn text(value: &Value) -> Result<String, NoRoom> {
    value.as_str().map(str::to_owned).ok_or(NoRoom)
}

// utoipa's `SchemaFormat` reads any string, as one of its known formats when it is one and as a format of its own
// otherwise, so it holds a format as utoipa holds one it generated. That loses nothing, unlike reading a schema.
fn format(value: &Value) -> Result<SchemaFormat, NoRoom> {
    serde_json::from_value(value.clone()).map_err(|_| NoRoom)
}

fn names(value: &Value) -> Result<Vec<String>, NoRoom> {
    let Value::Array(names) = value else {
        return Err(NoRoom);
    };
    names.iter().map(text).collect()
}
