//! Writing a type's rules into its JSON Schema, so that an API's document says what its rules require.
//!
//! [`write_rules`] takes the rules of a type, as [`Validate::rules`](crate::Validate::rules) or
//! [`Deserialize::rules`](crate::Deserialize::rules) return them, and the JSON Schema that a schema library generated
//! for that type. It adds a keyword for each rule that has one: `length(max = 50)` becomes `maxLength: 50`.
//!
//! - **The schema never rejects a value the rules accept.** Where no keyword says exactly what a rule requires, a
//!   looser one is written, or none.
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
//! json_schema::write_rules(&Member::rules(), schema.as_object_mut().unwrap());
//!
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
//!
//! | Rule | Written on | Keywords |
//! |---|---|---|
//! | `required` | an `Option` | `null` is removed from `type`, and from `oneOf` or `anyOf` |
//! | | | An alternative left alone there is written in their place |
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
//! - A keyword already in the schema is never loosened: a maximum keeps the smaller value and a minimum the larger.
//!   A `format` already there is kept, and a second `pattern` goes into `allOf`.
//! - Nothing is written for `ip`, which no single format covers, for `custom(...)`, for a rule across fields, or for
//!   a property that is a `$ref`, whose schema isn't followed.
//! - `pattern` is read by the `regex` crate when it is checked, and by a schema's reader as ECMA-262 says. The two
//!   differ at the edges: `\d` is any Unicode digit for the rule, and `0` to `9` in a schema.
//! - The keywords are those of JSON Schema 2020-12, which OpenAPI 3.1 uses.

use serde_json::{Map, Number, Value};

use crate::{Bounds, FieldRules, LengthUnit, ParamValue, Problem, Rules, ValueRules};

/// Adds what `rules` require to `schema`, the JSON Schema of the type they are the rules of.
///
/// Each field's keywords go into the property of its name, among the schema's `properties`. A field the schema has
/// no property for is skipped. Writing the same rules a second time changes nothing.
pub fn write_rules(rules: &Rules, schema: &mut Map<String, Value>) {
    for field in rules.fields() {
        write_field_rules(field, schema);
    }
}

fn write_field_rules(field: &FieldRules, schema: &mut Map<String, Value>) {
    let Some(property) = property_named(field.name(), schema) else {
        return;
    };
    write_value_rules(field.rules(), property);
    let is_broken_by_its_absence = field.rules().problems().contains(&Problem::Required) && !field.has_default();
    if is_broken_by_its_absence {
        list_as_required(field.name(), schema);
    }
}

fn property_named<'a>(name: &str, schema: &'a mut Map<String, Value>) -> Option<&'a mut Map<String, Value>> {
    schema.get_mut("properties")?.get_mut(name)?.as_object_mut()
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

fn write_value_rules(rules: &ValueRules, schema: &mut Map<String, Value>) {
    for problem in rules.problems() {
        write_problem(problem, rules.is_optional(), schema);
    }
    let Some(element_rules) = rules.elements() else {
        return;
    };
    if let Some(element_schema) = element_schema_of(schema) {
        write_value_rules(element_rules, element_schema);
    }
}

fn element_schema_of(schema: &mut Map<String, Value>) -> Option<&mut Map<String, Value>> {
    let elements = if accepts("array", schema) {
        "items"
    } else if accepts("object", schema) {
        "additionalProperties"
    } else {
        return None;
    };
    schema.get_mut(elements)?.as_object_mut()
}

fn write_problem(problem: &Problem, is_value_optional: bool, schema: &mut Map<String, Value>) {
    match problem {
        Problem::Required if is_value_optional => refuse_null(schema),
        Problem::Required => {
            write_counts("string", "Length", Some(1), None, schema);
            write_counts("array", "Items", Some(1), None, schema);
            write_counts("object", "Properties", Some(1), None, schema);
        }
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
        _ => {}
    }
}

fn accepts(value_type: &str, schema: &Map<String, Value>) -> bool {
    match schema.get("type") {
        Some(Value::String(only_type)) => only_type == value_type,
        Some(Value::Array(types)) => types.iter().any(|accepted| accepted == value_type),
        _ => false,
    }
}

fn refuse_null(schema: &mut Map<String, Value>) {
    if let Some(Value::Array(types)) = schema.get_mut("type") {
        types.retain(|accepted| accepted != "null");
        if let [only_type] = types.as_slice() {
            let only_type = only_type.clone();
            schema.insert("type".to_owned(), only_type);
        }
    }
    for alternatives in ["oneOf", "anyOf"] {
        if let Some(Value::Array(schemas)) = schema.get_mut(alternatives) {
            schemas.retain(|alternative| !accepts_only_null(alternative));
        }
        replace_by_its_only_alternative(alternatives, schema);
    }
}

// One alternative alone says what its own keywords say, so they are written in its place: a `$ref` beside `null`
// becomes that `$ref`. It stays an alternative when one of its keywords would replace one the schema has.
fn replace_by_its_only_alternative(alternatives: &str, schema: &mut Map<String, Value>) {
    let Some(Value::Array(schemas)) = schema.get(alternatives) else {
        return;
    };
    let [Value::Object(only_alternative)] = schemas.as_slice() else {
        return;
    };
    if only_alternative.keys().any(|keyword| schema.contains_key(keyword)) {
        return;
    }
    let only_alternative = only_alternative.clone();
    schema.remove(alternatives);
    schema.extend(only_alternative);
}

fn accepts_only_null(schema: &Value) -> bool {
    schema
        .as_object()
        .is_some_and(|schema| schema.len() == 1 && schema.get("type").is_some_and(|only_type| only_type == "null"))
}

fn write_length(bounds: &Bounds<usize>, unit: LengthUnit, schema: &mut Map<String, Value>) {
    let (shortest, longest) = bounds.ends();
    let (shortest, longest) = (shortest.copied(), longest.copied());
    match unit {
        LengthUnit::Chars => write_counts("string", "Length", shortest, longest, schema),
        // A string has no more chars than bytes, and at least a quarter as many, since a char takes up to four bytes.
        LengthUnit::Bytes => {
            let fewest_chars = shortest.map(|bytes| bytes.div_ceil(char::MAX_LEN_UTF8));
            write_counts("string", "Length", fewest_chars, longest, schema);
        }
        // A grapheme has one char or more, so graphemes cap no number of chars.
        LengthUnit::Graphemes => write_counts("string", "Length", shortest, None, schema),
        LengthUnit::Elements => {
            write_counts("array", "Items", shortest, longest, schema);
            write_counts("object", "Properties", shortest, longest, schema);
        }
    }
}

fn write_counts(
    counted_type: &str,
    counted: &str,
    fewest: Option<usize>,
    most: Option<usize>,
    schema: &mut Map<String, Value>,
) {
    if !accepts(counted_type, schema) {
        return;
    }
    if let Some(fewest) = fewest {
        raise_to(Number::from(fewest), &format!("min{counted}"), schema);
    }
    if let Some(most) = most {
        lower_to(Number::from(most), &format!("max{counted}"), schema);
    }
}

fn write_range(bounds: &Bounds<ParamValue>, schema: &mut Map<String, Value>) {
    if !accepts("integer", schema) && !accepts("number", schema) {
        return;
    }
    let (least, greatest) = bounds.ends();
    if let Some(least) = least.and_then(as_number) {
        raise_to(least, "minimum", schema);
    }
    if let Some(greatest) = greatest.and_then(as_number) {
        lower_to(greatest, "maximum", schema);
    }
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

fn write_format(format: &str, schema: &mut Map<String, Value>) {
    if accepts("string", schema) && !schema.contains_key("format") {
        schema.insert("format".to_owned(), format.into());
    }
}

// A schema has room for one `pattern`, so any other is required through `allOf`.
fn write_pattern(pattern: String, schema: &mut Map<String, Value>) {
    if !accepts("string", schema) {
        return;
    }
    let Some(first_pattern) = schema.get("pattern") else {
        schema.insert("pattern".to_owned(), pattern.into());
        return;
    };
    if *first_pattern == pattern {
        return;
    }
    let also_required = schema.entry("allOf").or_insert_with(|| Value::Array(Vec::new()));
    let Value::Array(also_required) = also_required else {
        return;
    };
    let requiring_the_pattern = Value::Object(Map::from_iter([("pattern".to_owned(), Value::String(pattern))]));
    if !also_required.contains(&requiring_the_pattern) {
        also_required.push(requiring_the_pattern);
    }
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
