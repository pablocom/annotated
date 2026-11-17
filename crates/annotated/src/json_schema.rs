//! Writing a type's rules into its JSON Schema, so that an API's document says what its rules require.
//!
//! [`write_rules`] takes the rules of a type, as [`Validate::rules`](crate::Validate::rules) or
//! [`Deserialize::rules`](crate::Deserialize::rules) return them, and the JSON Schema that a schema library generated
//! for that type. It adds a keyword for each rule that has one: `length(max = 50)` becomes `maxLength: 50`.
//!
//! - **The schema never rejects a value the rules accept.** Where no keyword says exactly what a rule requires, a
//!   looser one is written, or none.
//! - **What isn't written is returned.** Each rule left out is an [`UnwrittenRule`]: where its keyword belongs in the
//!   schema, and the [`Reason`] it isn't there. Which of them a document can do without is the caller's choice.
//! - **The schema is JSON.** It is a [`serde_json::Map`], so this module depends on no schema library. An OpenAPI 3.1
//!   schema is a JSON Schema, and a library that generates one can turn it into JSON and back.
//!
//! ```
//! use annotated::{Validate, json_schema};
//! use serde_json::json;
//!
//! #[derive(Validate)]
//! struct Member {
//!     #[validate(required, length(max = 50))]
//!     name: String,
//!     #[validate(range(min = 18))]
//!     age: Option<u8>,
//! }
//!
//! let mut schema = json!({
//!     "type": "object",
//!     "properties": {
//!         "name": { "type": "string" },
//!         "age": { "type": ["integer", "null"] }
//!     },
//!     "required": ["name"]
//! });
//!
//! let unwritten = json_schema::write_rules(&Member::rules(), schema.as_object_mut().unwrap());
//!
//! assert!(unwritten.is_empty());
//! assert_eq!(
//!     schema,
//!     json!({
//!         "type": "object",
//!         "properties": {
//!             "name": { "type": "string", "minLength": 1, "maxLength": 50 },
//!             "age": { "type": ["integer", "null"], "minimum": 18 }
//!         },
//!         "required": ["name"]
//!     })
//! );
//! ```
//!
//! # What each rule writes
//!
//! A keyword is written only on a schema whose own `type` says the value is of the type that keyword constrains.
//! Otherwise, the rule is returned as unwritten.
//!
//! | Rule | Written on | Keywords |
//! |---|---|---|
//! | `required` | an `Option` | none, so `null` stays allowed: it asks for the field's key, which takes no default |
//! | | a string, an array, an object | `minLength: 1`, `minItems: 1`, `minProperties: 1` |
//! | | a field without a default | its name is added to the object's `required` |
//! | `length` | a string | `minLength`, `maxLength` |
//! | | an array, an object | `minItems`, `maxItems`, `minProperties`, `maxProperties` |
//! | `length` in bytes | a string | `maxLength`, and `minLength` of a quarter of the shortest, rounded up |
//! | `length` in graphemes | a string | `minLength` |
//! | `range` | an integer, a number | `minimum`, `maximum`, when the bounds are numbers |
//! | `email`, `ipv4`, `ipv6` | a string | `format` |
//! | `url` | a string | `format: iri`, since the rule accepts international characters |
//! | `pattern` | a string | `pattern`, as written |
//! | `contains`, `prefix`, `suffix` | a string | `pattern`, with the text escaped |
//! | `each(...)` | an array, an object | the same, in `items` or in `additionalProperties` |
//!
//! - A field's property is looked for among the schema's `properties`, then among those of its `allOf` members, at any
//!   depth: utoipa documents a struct with a flattened field as `allOf`.
//! - A keyword already in the schema is never loosened: a maximum keeps the smaller value and a minimum the larger.
//!   A `format` already there is kept, and a second `pattern` goes into `allOf`.
//! - Nothing is written for `ip`, which no single format covers, or for a property that is a `$ref`, whose schema
//!   isn't followed, and both are returned as unwritten. `custom(...)` and a rule across fields say nothing of what
//!   they require, so [`Rules`] doesn't list them, and they are neither written nor returned.
//! - `pattern` is read by the `regex` crate when it is checked, and by a schema's reader as ECMA-262 says. The two
//!   differ at the edges: `\d` is any Unicode digit for the rule, and `0` to `9` in a schema.
//! - The keywords are those of JSON Schema 2020-12, which OpenAPI 3.1 uses.

use std::fmt;

use serde_json::{Map, Number, Value};

use crate::{Bounds, FieldRules, LengthUnit, ParamValue, Problem, Rules, ValueRules};

/// Adds what `rules` require to `schema`, the JSON Schema of the type they are the rules of, and returns each rule it
/// couldn't write.
///
/// Each field's keywords go into the property of its name, among the `properties` of the schema or of one of its
/// `allOf` members. Writing the same rules a second time changes nothing.
#[must_use = "a rule left out of the schema is only reported here"]
pub fn write_rules(rules: &Rules, schema: &mut Map<String, Value>) -> Vec<UnwrittenRule> {
    let mut unwritten = Vec::new();
    for field in rules.fields() {
        write_field_rules(field, schema, &mut unwritten);
    }
    unwritten
}

/// A rule that [`write_rules`] couldn't write into a schema: which rule, where its keyword belongs, and why it isn't
/// there.
///
/// ```
/// use annotated::json_schema::{self, Reason};
/// use annotated::Validate;
/// use serde_json::json;
///
/// #[derive(Validate)]
/// struct Server {
///     #[validate(ip)]
///     address: String,
/// }
///
/// let mut schema = json!({ "type": "object", "properties": { "address": { "type": "string" } } });
///
/// let unwritten = json_schema::write_rules(&Server::rules(), schema.as_object_mut().unwrap());
///
/// assert_eq!(unwritten[0].pointer(), "/properties/address");
/// assert_eq!(unwritten[0].problem().code(), "ip");
/// assert_eq!(unwritten[0].reason(), Reason::NoKeyword);
/// assert_eq!(
///     unwritten[0].to_string(),
///     "`ip` can't be written at `/properties/address`: no keyword says what the rule requires"
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct UnwrittenRule {
    pointer: String,
    problem: Problem,
    reason: Reason,
}

impl UnwrittenRule {
    /// Where in the schema the rule's keyword belongs, as an RFC 6901 JSON Pointer: `/properties/name`, or
    /// `/properties/tags/items` for a rule on each element.
    #[must_use]
    pub fn pointer(&self) -> &str {
        &self.pointer
    }

    /// The rule, as the problem that every value breaking it has.
    #[must_use]
    pub fn problem(&self) -> &Problem {
        &self.problem
    }

    /// Why the rule isn't written.
    #[must_use]
    pub fn reason(&self) -> Reason {
        self.reason
    }
}

impl fmt::Display for UnwrittenRule {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "`{}` can't be written at `{}`: {}",
            self.problem.code(),
            self.pointer,
            self.reason
        )
    }
}

/// Why a rule isn't written into a schema.
///
/// New reasons may be added without a breaking change, so a `match` on it needs a wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Reason {
    /// The schema has no subschema where the rule's keyword belongs: no property for the field, or no `items` or
    /// `additionalProperties` for its elements.
    NoSchema,
    /// The subschema there refers to another with `$ref`, which isn't followed.
    Reference,
    /// The subschema there isn't of a type the rule constrains: a `length` on bytes documented as a base64 string, or a
    /// `range` on a number documented as a string.
    OtherType,
    /// No keyword says what the rule requires: `ip`, a rule of your own, a `range` whose bounds aren't numbers, or a
    /// `length` in graphemes with only a maximum.
    NoKeyword,
}

impl fmt::Display for Reason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let explanation = match self {
            Self::NoSchema => "the schema has no subschema there",
            Self::Reference => "the subschema there is a `$ref`, which isn't followed",
            Self::OtherType => "the subschema there isn't of a type the rule constrains",
            Self::NoKeyword => "no keyword says what the rule requires",
        };
        formatter.write_str(explanation)
    }
}

/// A type of value whose size two keywords bound: `minLength` and `maxLength` count the chars of a string.
struct Counted {
    value_type: &'static str,
    fewest: &'static str,
    most: &'static str,
}

const CHARS: Counted = Counted {
    value_type: "string",
    fewest: "minLength",
    most: "maxLength",
};
const ITEMS: Counted = Counted {
    value_type: "array",
    fewest: "minItems",
    most: "maxItems",
};
const PROPERTIES: Counted = Counted {
    value_type: "object",
    fewest: "minProperties",
    most: "maxProperties",
};

fn write_field_rules(field: &FieldRules, schema: &mut Map<String, Value>, unwritten: &mut Vec<UnwrittenRule>) {
    if !write_where_declared(field, schema, "", unwritten) {
        let pointer = property_pointer("", field.name());
        leave_unwritten(field.rules(), &pointer, Reason::NoSchema, unwritten);
    }
}

/// Writes the field's rules into whichever of `schema` and its `allOf` members, at any depth, declares its property,
/// and answers whether one does. `pointer` points at `schema`.
fn write_where_declared(
    field: &FieldRules,
    schema: &mut Map<String, Value>,
    pointer: &str,
    unwritten: &mut Vec<UnwrittenRule>,
) -> bool {
    if let Some(property) = property_named(field.name(), schema) {
        let property_pointer = property_pointer(pointer, field.name());
        write_value_rules(field.rules(), property, &property_pointer, unwritten);
        let is_broken_by_its_absence = field.rules().problems().contains(&Problem::Required) && !field.has_default();
        if is_broken_by_its_absence {
            list_as_required(field.name(), schema);
        }
        return true;
    }
    let Some(Value::Array(members)) = schema.get_mut("allOf") else {
        return false;
    };
    members.iter_mut().enumerate().any(|(index, member)| {
        let member_pointer = format!("{pointer}/allOf/{index}");
        member
            .as_object_mut()
            .is_some_and(|member| write_where_declared(field, member, &member_pointer, unwritten))
    })
}

fn property_named<'a>(name: &str, schema: &'a mut Map<String, Value>) -> Option<&'a mut Map<String, Value>> {
    schema.get_mut("properties")?.get_mut(name)?.as_object_mut()
}

fn property_pointer(pointer: &str, name: &str) -> String {
    let name = name.replace('~', "~0").replace('/', "~1");
    format!("{pointer}/properties/{name}")
}

fn list_as_required(name: &str, schema: &mut Map<String, Value>) {
    let required = schema.entry("required").or_insert_with(|| Value::Array(Vec::new()));
    let Value::Array(required) = required else {
        return;
    };
    if !required.iter().any(|listed| listed == name) {
        required.push(name.into());
    }
}

fn write_value_rules(
    rules: &ValueRules,
    schema: &mut Map<String, Value>,
    pointer: &str,
    unwritten: &mut Vec<UnwrittenRule>,
) {
    for problem in rules.problems() {
        if let Err(reason) = write_problem(problem, rules.is_optional(), schema) {
            unwritten.push(UnwrittenRule {
                pointer: pointer.to_owned(),
                problem: problem.clone(),
                reason,
            });
        }
    }
    let Some(element_rules) = rules.elements() else {
        return;
    };
    let Some(elements) = keyword_of_the_elements(schema) else {
        leave_unwritten(element_rules, pointer, why_unconstrained(schema), unwritten);
        return;
    };
    let element_pointer = format!("{pointer}/{elements}");
    match schema.get_mut(elements).and_then(Value::as_object_mut) {
        Some(element_schema) => write_value_rules(element_rules, element_schema, &element_pointer, unwritten),
        None => leave_unwritten(element_rules, &element_pointer, Reason::NoSchema, unwritten),
    }
}

/// Reports every rule in `rules`, those of its elements included, as unwritten at `pointer` for `reason`.
fn leave_unwritten(rules: &ValueRules, pointer: &str, reason: Reason, unwritten: &mut Vec<UnwrittenRule>) {
    unwritten.extend(rules.problems().iter().map(|problem| UnwrittenRule {
        pointer: pointer.to_owned(),
        problem: problem.clone(),
        reason,
    }));
    if let Some(element_rules) = rules.elements() {
        leave_unwritten(element_rules, pointer, reason, unwritten);
    }
}

fn keyword_of_the_elements(schema: &Map<String, Value>) -> Option<&'static str> {
    if accepts("array", schema) {
        return Some("items");
    }
    if accepts("object", schema) {
        return Some("additionalProperties");
    }
    None
}

/// Why a schema that isn't of the type a keyword constrains can't take it.
fn why_unconstrained(schema: &Map<String, Value>) -> Reason {
    if schema.contains_key("$ref") {
        return Reason::Reference;
    }
    Reason::OtherType
}

fn write_problem(problem: &Problem, is_value_optional: bool, schema: &mut Map<String, Value>) -> Result<(), Reason> {
    match problem {
        Problem::Required if is_value_optional => Ok(()),
        Problem::Required => write_counts(&[CHARS, ITEMS, PROPERTIES], Some(1), None, schema),
        Problem::Length { bounds, unit } => write_length(bounds, *unit, schema),
        Problem::Range { bounds } => write_range(bounds, schema),
        Problem::Email => write_format("email", schema),
        Problem::Url { .. } => write_format("iri", schema),
        Problem::Ipv4 => write_format("ipv4", schema),
        Problem::Ipv6 => write_format("ipv6", schema),
        Problem::Pattern { pattern } => write_pattern((*pattern).to_owned(), schema),
        Problem::Contains { text } => write_pattern(matching_exactly(text), schema),
        Problem::Prefix { prefix } => write_pattern(format!("^{}", matching_exactly(prefix)), schema),
        Problem::Suffix { suffix } => write_pattern(format!("{}$", matching_exactly(suffix)), schema),
        _ => Err(Reason::NoKeyword),
    }
}

fn accepts(value_type: &str, schema: &Map<String, Value>) -> bool {
    match schema.get("type") {
        Some(Value::String(only_type)) => only_type == value_type,
        Some(Value::Array(types)) => types.iter().any(|accepted| accepted == value_type),
        _ => false,
    }
}

fn write_length(bounds: &Bounds<usize>, unit: LengthUnit, schema: &mut Map<String, Value>) -> Result<(), Reason> {
    let (shortest, longest) = bounds.ends();
    let (shortest, longest) = (shortest.copied(), longest.copied());
    match unit {
        LengthUnit::Chars => write_counts(&[CHARS], shortest, longest, schema),
        // A string has no more chars than bytes, and at least a quarter as many, since a char takes up to four bytes.
        LengthUnit::Bytes => {
            let fewest_chars = shortest.map(|bytes| bytes.div_ceil(char::MAX_LEN_UTF8));
            write_counts(&[CHARS], fewest_chars, longest, schema)
        }
        // A grapheme has one char or more, so graphemes cap no number of chars.
        LengthUnit::Graphemes => write_counts(&[CHARS], shortest, None, schema),
        LengthUnit::Elements => write_counts(&[ITEMS, PROPERTIES], shortest, longest, schema),
    }
}

/// Bounds the size of the value with the keywords of each type in `counted` that the schema says it is of.
fn write_counts(
    counted: &[Counted],
    fewest: Option<usize>,
    most: Option<usize>,
    schema: &mut Map<String, Value>,
) -> Result<(), Reason> {
    if fewest.is_none() && most.is_none() {
        return Err(Reason::NoKeyword);
    }
    let mut is_bounded = false;
    for counted in counted {
        if !accepts(counted.value_type, schema) {
            continue;
        }
        if let Some(fewest) = fewest {
            raise_to(Number::from(fewest), counted.fewest, schema);
        }
        if let Some(most) = most {
            lower_to(Number::from(most), counted.most, schema);
        }
        is_bounded = true;
    }
    if !is_bounded {
        return Err(why_unconstrained(schema));
    }
    Ok(())
}

fn write_range(bounds: &Bounds<ParamValue>, schema: &mut Map<String, Value>) -> Result<(), Reason> {
    let (least, greatest) = bounds.ends();
    let (least, greatest) = (least.and_then(as_number), greatest.and_then(as_number));
    if least.is_none() && greatest.is_none() {
        return Err(Reason::NoKeyword);
    }
    if !accepts("integer", schema) && !accepts("number", schema) {
        return Err(why_unconstrained(schema));
    }
    if let Some(least) = least {
        raise_to(least, "minimum", schema);
    }
    if let Some(greatest) = greatest {
        lower_to(greatest, "maximum", schema);
    }
    Ok(())
}

fn as_number(bound: &ParamValue) -> Option<Number> {
    match bound {
        ParamValue::Integer(integer) => Number::from_i128(*integer),
        ParamValue::Float(float) => Number::from_f64(*float),
        ParamValue::Text(_) => None,
    }
}

fn raise_to(minimum: Number, keyword: &str, schema: &mut Map<String, Value>) {
    let is_high_enough_already =
        matches!(schema.get(keyword), Some(Value::Number(current)) if !is_less(current, &minimum));
    if !is_high_enough_already {
        schema.insert(keyword.to_owned(), Value::Number(minimum));
    }
}

fn lower_to(maximum: Number, keyword: &str, schema: &mut Map<String, Value>) {
    let is_low_enough_already =
        matches!(schema.get(keyword), Some(Value::Number(current)) if !is_less(&maximum, current));
    if !is_low_enough_already {
        schema.insert(keyword.to_owned(), Value::Number(maximum));
    }
}

fn is_less(left: &Number, right: &Number) -> bool {
    match (left.as_i128(), right.as_i128()) {
        (Some(left), Some(right)) => left < right,
        _ => left.as_f64() < right.as_f64(),
    }
}

fn write_format(format: &str, schema: &mut Map<String, Value>) -> Result<(), Reason> {
    if !accepts("string", schema) {
        return Err(why_unconstrained(schema));
    }
    if !schema.contains_key("format") {
        schema.insert("format".to_owned(), format.into());
    }
    Ok(())
}

// A schema has room for one `pattern`, so any other is required through `allOf`.
fn write_pattern(pattern: String, schema: &mut Map<String, Value>) -> Result<(), Reason> {
    if !accepts("string", schema) {
        return Err(why_unconstrained(schema));
    }
    let Some(first_pattern) = schema.get("pattern") else {
        schema.insert("pattern".to_owned(), pattern.into());
        return Ok(());
    };
    if *first_pattern == pattern {
        return Ok(());
    }
    let also_required = schema.entry("allOf").or_insert_with(|| Value::Array(Vec::new()));
    let Value::Array(also_required) = also_required else {
        return Err(Reason::NoSchema);
    };
    let requiring_the_pattern = Value::Object(Map::from_iter([("pattern".to_owned(), Value::String(pattern))]));
    if !also_required.contains(&requiring_the_pattern) {
        also_required.push(requiring_the_pattern);
    }
    Ok(())
}

/// A regular expression that matches `text` and nothing else, in the syntax ECMA-262 gives a schema's `pattern`.
fn matching_exactly(text: &str) -> String {
    let mut expression = String::with_capacity(text.len());
    for character in text.chars() {
        if r"\^$.*+?()[]{}|".contains(character) {
            expression.push('\\');
        }
        expression.push(character);
    }
    expression
}
