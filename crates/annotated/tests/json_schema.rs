//! With the `json-schema` feature, `annotated::json_schema::write_rules` writes a struct's rules into its JSON Schema.
//! The schema never rejects a value the rules accept.

#![cfg(feature = "json-schema")]
// The types here are only described, never built.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fmt::{self, Display};

use annotated::json_schema::Reason;
use annotated::{Problem, Validate, json_schema};
use serde_json::{Map, Value, json};

fn with_the_rules_of<T: Validate>(mut schema: Value) -> Value {
    let schema_object = schema
        .as_object_mut()
        .expect("a schema to write rules into is an object");
    let _ = json_schema::write_rules(&T::rules(), schema_object);
    schema
}

fn unwritten_rules_of<T: Validate>(mut schema: Value) -> Vec<(String, String, Reason)> {
    let schema_object = schema
        .as_object_mut()
        .expect("a schema to write rules into is an object");
    json_schema::write_rules(&T::rules(), schema_object)
        .iter()
        .map(|rule| left_unwritten(rule.pointer(), rule.problem().code(), rule.reason()))
        .collect()
}

fn left_unwritten(pointer: &str, code: &str, reason: Reason) -> (String, String, Reason) {
    (pointer.to_owned(), code.to_owned(), reason)
}

fn object_with(properties: Value) -> Value {
    Value::Object(Map::from_iter([
        ("type".to_owned(), Value::from("object")),
        ("properties".to_owned(), properties),
    ]))
}

fn not_reserved(handle: &str) -> Result<(), Problem> {
    if handle == "admin" {
        return Err(Problem::custom("reserved", "is reserved"));
    }
    Ok(())
}

#[test]
fn length_on_a_string_becomes_its_shortest_and_longest_length() {
    #[derive(Validate)]
    struct Member {
        #[validate(length(min = 2, max = 50))]
        name: String,
    }

    let schema = with_the_rules_of::<Member>(object_with(json!({ "name": { "type": "string" } })));

    assert_eq!(
        schema,
        object_with(json!({ "name": { "type": "string", "minLength": 2, "maxLength": 50 } }))
    );
}

#[test]
fn length_on_a_list_becomes_its_fewest_and_most_items() {
    #[derive(Validate)]
    struct Post {
        #[validate(length(min = 1, max = 5))]
        tags: Vec<String>,
    }

    let tags = json!({ "type": "array", "items": { "type": "string" } });
    let schema = with_the_rules_of::<Post>(object_with(json!({ "tags": tags })));

    assert_eq!(
        schema,
        object_with(json!({
            "tags": { "type": "array", "items": { "type": "string" }, "minItems": 1, "maxItems": 5 }
        }))
    );
}

#[test]
fn length_on_a_map_becomes_its_fewest_and_most_properties() {
    #[derive(Validate)]
    struct Post {
        #[validate(length(min = 1, max = 5))]
        labels: BTreeMap<String, String>,
    }

    let labels = json!({ "type": "object", "additionalProperties": { "type": "string" } });
    let schema = with_the_rules_of::<Post>(object_with(json!({ "labels": labels })));

    assert_eq!(
        schema,
        object_with(json!({
            "labels": {
                "type": "object",
                "additionalProperties": { "type": "string" },
                "minProperties": 1,
                "maxProperties": 5
            }
        }))
    );
}

#[test]
fn length_in_bytes_caps_the_chars_and_asks_for_a_quarter_of_the_shortest() {
    #[derive(Validate)]
    struct Member {
        #[validate(length(min = 5, max = 10, unit = "bytes"))]
        name: String,
    }

    let schema = with_the_rules_of::<Member>(object_with(json!({ "name": { "type": "string" } })));

    assert_eq!(
        schema,
        object_with(json!({ "name": { "type": "string", "minLength": 2, "maxLength": 10 } }))
    );
}

#[cfg(feature = "unicode")]
#[test]
fn length_in_graphemes_asks_for_as_many_chars_and_caps_none() {
    #[derive(Validate)]
    struct Member {
        #[validate(length(min = 2, max = 10, unit = "graphemes"))]
        name: String,
    }

    let schema = with_the_rules_of::<Member>(object_with(json!({ "name": { "type": "string" } })));

    assert_eq!(
        schema,
        object_with(json!({ "name": { "type": "string", "minLength": 2 } }))
    );
}

#[test]
fn range_on_a_number_becomes_its_minimum_and_maximum() {
    #[derive(Validate)]
    struct Member {
        #[validate(range(min = 18, max = 120))]
        age: u8,
        #[validate(range(min = 0.5))]
        share: f64,
    }

    let schema = with_the_rules_of::<Member>(object_with(json!({
        "age": { "type": "integer" },
        "share": { "type": "number" }
    })));

    assert_eq!(
        schema,
        object_with(json!({
            "age": { "type": "integer", "minimum": 18, "maximum": 120 },
            "share": { "type": "number", "minimum": 0.5 }
        }))
    );
}

#[test]
fn a_maximum_on_an_f32_is_the_f32_the_rule_compares_against_not_the_decimal_it_is_written_as() {
    #[derive(Validate)]
    struct Measure {
        #[validate(range(max = 0.1))]
        ratio: f32,
    }

    let schema = with_the_rules_of::<Measure>(object_with(json!({ "ratio": { "type": "number" } })));

    assert_eq!(
        schema,
        object_with(json!({ "ratio": { "type": "number", "maximum": 0.100_000_001_490_116_12 } }))
    );
}

#[test]
fn a_minimum_on_an_f32_is_the_f32_the_rule_compares_against_not_the_decimal_it_is_written_as() {
    #[derive(Validate)]
    struct Measure {
        #[validate(range(min = 0.7))]
        ratio: f32,
    }

    let schema = with_the_rules_of::<Measure>(object_with(json!({ "ratio": { "type": "number" } })));

    assert_eq!(
        schema,
        object_with(json!({ "ratio": { "type": "number", "minimum": 0.699_999_988_079_071 } }))
    );
}

#[test]
fn a_bound_on_an_f64_is_written_as_it_is() {
    #[derive(Validate)]
    struct Measure {
        #[validate(range(min = 0.7, max = 0.1))]
        ratio: f64,
    }

    let schema = with_the_rules_of::<Measure>(object_with(json!({ "ratio": { "type": "number" } })));

    assert_eq!(
        schema,
        object_with(json!({ "ratio": { "type": "number", "minimum": 0.7, "maximum": 0.1 } }))
    );
}

#[test]
fn the_bounds_of_the_widest_64_bit_integers_are_written_exactly() {
    #[derive(Validate)]
    struct Counter {
        #[validate(range(max = 18_446_744_073_709_551_615))]
        visits: u64,
        #[validate(range(min = -9_223_372_036_854_775_808))]
        balance: i64,
    }

    let schema = with_the_rules_of::<Counter>(object_with(json!({
        "visits": { "type": "integer" },
        "balance": { "type": "integer" }
    })));

    assert_eq!(
        schema,
        object_with(json!({
            "visits": { "type": "integer", "maximum": 18_446_744_073_709_551_615_u64 },
            "balance": { "type": "integer", "minimum": -9_223_372_036_854_775_808_i64 }
        }))
    );
}

#[test]
fn a_bound_beyond_the_integers_json_holds_exactly_is_unwritten() {
    #[derive(Validate)]
    struct Counter {
        #[validate(range(min = -170_141_183_460_469_231_731_687_303_715_884_105_728))]
        balance: i128,
        #[validate(range(max = 340_282_366_920_938_463_463_374_607_431_768_211_455))]
        visits: u128,
    }

    let unwritten = unwritten_rules_of::<Counter>(object_with(json!({
        "balance": { "type": "integer" },
        "visits": { "type": "integer" }
    })));

    assert_eq!(
        unwritten,
        vec![
            left_unwritten("/properties/balance", "range", Reason::NoKeyword),
            left_unwritten("/properties/visits", "range", Reason::NoKeyword)
        ]
    );
}

#[test]
fn range_with_bounds_that_are_not_numbers_writes_nothing() {
    #[derive(PartialEq, PartialOrd)]
    struct Euros(f64);

    impl Display for Euros {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "{} €", self.0)
        }
    }

    #[derive(Validate)]
    struct Donation {
        #[validate(range(min = Euros(0.5)))]
        amount: Euros,
    }

    let schema = with_the_rules_of::<Donation>(object_with(json!({ "amount": { "type": "number" } })));

    assert_eq!(schema, object_with(json!({ "amount": { "type": "number" } })));
}

#[test]
fn required_on_a_string_asks_for_one_char() {
    #[derive(Validate)]
    struct Member {
        #[validate(required)]
        name: String,
    }

    let schema = with_the_rules_of::<Member>(json!({
        "type": "object",
        "properties": { "name": { "type": "string" } },
        "required": ["name"]
    }));

    assert_eq!(
        schema,
        json!({
            "type": "object",
            "properties": { "name": { "type": "string", "minLength": 1 } },
            "required": ["name"]
        })
    );
}

#[test]
fn required_on_a_list_or_a_map_asks_for_one_element() {
    #[derive(Validate)]
    struct Post {
        #[validate(required)]
        tags: Vec<String>,
        #[validate(required)]
        labels: BTreeMap<String, String>,
    }

    let schema = with_the_rules_of::<Post>(json!({
        "type": "object",
        "properties": { "tags": { "type": "array" }, "labels": { "type": "object" } },
        "required": ["tags", "labels"]
    }));

    assert_eq!(
        schema,
        json!({
            "type": "object",
            "properties": {
                "tags": { "type": "array", "minItems": 1 },
                "labels": { "type": "object", "minProperties": 1 }
            },
            "required": ["tags", "labels"]
        })
    );
}

#[test]
fn required_on_an_option_lists_the_field_as_required_and_leaves_null_allowed() {
    #[derive(Validate)]
    struct Member {
        #[validate(required)]
        nickname: Option<String>,
        age: u8,
    }

    let schema = with_the_rules_of::<Member>(json!({
        "type": "object",
        "properties": { "nickname": { "type": ["string", "null"] }, "age": { "type": "integer" } },
        "required": ["age"]
    }));

    assert_eq!(
        schema,
        json!({
            "type": "object",
            "properties": { "nickname": { "type": ["string", "null"] }, "age": { "type": "integer" } },
            "required": ["age", "nickname"]
        })
    );
}

#[test]
fn required_on_an_option_leaves_null_among_its_alternatives() {
    #[derive(Validate)]
    struct Address {
        street: String,
    }

    #[derive(Validate)]
    struct Member {
        #[validate(required)]
        address: Option<Address>,
        #[validate(required)]
        billing: Option<Address>,
    }

    let schema = with_the_rules_of::<Member>(object_with(json!({
        "address": { "oneOf": [{ "type": "null" }, { "$ref": "#/components/schemas/Address" }] },
        "billing": { "anyOf": [{ "$ref": "#/$defs/Address" }, { "type": "null" }] }
    })));

    assert_eq!(
        schema,
        json!({
            "type": "object",
            "properties": {
                "address": { "oneOf": [{ "type": "null" }, { "$ref": "#/components/schemas/Address" }] },
                "billing": { "anyOf": [{ "$ref": "#/$defs/Address" }, { "type": "null" }] }
            },
            "required": ["address", "billing"]
        })
    );
}

#[test]
fn required_on_an_option_with_a_default_lists_the_field_as_required() {
    #[derive(Validate)]
    struct Member {
        #[validate(required)]
        #[serde(default)]
        nickname: Option<String>,
    }

    let schema = with_the_rules_of::<Member>(object_with(json!({ "nickname": { "type": ["string", "null"] } })));

    assert_eq!(
        schema,
        json!({
            "type": "object",
            "properties": { "nickname": { "type": ["string", "null"] } },
            "required": ["nickname"]
        })
    );
}

#[test]
fn required_on_an_option_in_a_struct_with_a_default_lists_the_field_as_required() {
    #[derive(Validate)]
    #[serde(default)]
    struct Member {
        #[validate(required)]
        nickname: Option<String>,
    }

    let schema = with_the_rules_of::<Member>(object_with(json!({ "nickname": { "type": ["string", "null"] } })));

    assert_eq!(
        schema,
        json!({
            "type": "object",
            "properties": { "nickname": { "type": ["string", "null"] } },
            "required": ["nickname"]
        })
    );
}

#[test]
fn required_on_optional_elements_writes_nothing_since_an_element_is_always_present() {
    #[derive(Validate)]
    struct Review {
        #[validate(each(required))]
        reviewers: Vec<Option<String>>,
    }

    let schema = with_the_rules_of::<Review>(object_with(json!({
        "reviewers": { "type": "array", "items": { "type": ["string", "null"] } }
    })));

    assert_eq!(
        schema,
        object_with(json!({
            "reviewers": { "type": "array", "items": { "type": ["string", "null"] } }
        }))
    );
}

#[test]
fn a_required_field_with_a_default_is_not_listed_as_required() {
    #[derive(Validate)]
    struct Member {
        #[validate(required)]
        #[serde(default = "anonymous")]
        nickname: String,
    }

    fn anonymous() -> String {
        "anonymous".to_owned()
    }

    let schema = with_the_rules_of::<Member>(object_with(json!({ "nickname": { "type": "string" } })));

    assert_eq!(
        schema,
        object_with(json!({ "nickname": { "type": "string", "minLength": 1 } }))
    );
}

#[test]
fn a_rule_on_an_optional_field_is_written_beside_null() {
    #[derive(Validate)]
    struct Member {
        #[validate(length(max = 160))]
        bio: Option<String>,
    }

    let schema = with_the_rules_of::<Member>(object_with(json!({ "bio": { "type": ["string", "null"] } })));

    assert_eq!(
        schema,
        object_with(json!({ "bio": { "type": ["string", "null"], "maxLength": 160 } }))
    );
}

#[test]
fn email_and_ip_addresses_become_their_formats() {
    #[derive(Validate)]
    struct Host {
        #[validate(email)]
        owner: String,
        #[validate(ipv4)]
        address: String,
        #[validate(ipv6)]
        address_v6: String,
    }

    let schema = with_the_rules_of::<Host>(object_with(json!({
        "owner": { "type": "string" },
        "address": { "type": "string" },
        "address_v6": { "type": "string" }
    })));

    assert_eq!(
        schema,
        object_with(json!({
            "owner": { "type": "string", "format": "email" },
            "address": { "type": "string", "format": "ipv4" },
            "address_v6": { "type": "string", "format": "ipv6" }
        }))
    );
}

#[test]
fn url_becomes_the_iri_format_since_it_accepts_international_characters() {
    #[derive(Validate)]
    struct Member {
        #[validate(url)]
        website: String,
    }

    let schema = with_the_rules_of::<Member>(object_with(json!({ "website": { "type": "string" } })));

    assert_eq!(
        schema,
        object_with(json!({ "website": { "type": "string", "format": "iri" } }))
    );
}

#[test]
fn an_address_of_either_ip_version_and_a_rule_that_says_nothing_write_nothing() {
    #[derive(Validate)]
    struct Host {
        #[validate(ip)]
        address: String,
        #[validate(custom(not_reserved))]
        name: String,
    }

    let schema = with_the_rules_of::<Host>(object_with(json!({
        "address": { "type": "string" },
        "name": { "type": "string" }
    })));

    assert_eq!(
        schema,
        object_with(json!({ "address": { "type": "string" }, "name": { "type": "string" } }))
    );
}

#[cfg(feature = "regex")]
#[test]
fn a_pattern_is_copied_as_written() {
    #[derive(Validate)]
    struct Address {
        #[validate(pattern = r"^\d{5}$")]
        postal_code: String,
    }

    let schema = with_the_rules_of::<Address>(object_with(json!({ "postal_code": { "type": "string" } })));

    assert_eq!(
        schema,
        object_with(json!({ "postal_code": { "type": "string", "pattern": r"^\d{5}$" } }))
    );
}

#[test]
fn text_a_string_must_contain_becomes_a_pattern_with_the_text_escaped() {
    #[derive(Validate)]
    struct Upload {
        #[validate(contains = "(1).")]
        name: String,
    }

    let schema = with_the_rules_of::<Upload>(object_with(json!({ "name": { "type": "string" } })));

    assert_eq!(
        schema,
        object_with(json!({ "name": { "type": "string", "pattern": r"\(1\)\." } }))
    );
}

#[test]
fn a_prefix_and_a_suffix_become_patterns_anchored_at_their_end() {
    #[derive(Validate)]
    struct Upload {
        #[validate(prefix = "img-")]
        name: String,
        #[validate(suffix = ".png")]
        path: String,
    }

    let schema = with_the_rules_of::<Upload>(object_with(json!({
        "name": { "type": "string" },
        "path": { "type": "string" }
    })));

    assert_eq!(
        schema,
        object_with(json!({
            "name": { "type": "string", "pattern": "^img-" },
            "path": { "type": "string", "pattern": r"\.png$" }
        }))
    );
}

#[test]
fn a_second_pattern_goes_into_all_of() {
    #[derive(Validate)]
    struct Upload {
        #[validate(prefix = "img-", suffix = ".png")]
        name: String,
    }

    let schema = with_the_rules_of::<Upload>(object_with(json!({ "name": { "type": "string" } })));

    assert_eq!(
        schema,
        object_with(json!({
            "name": { "type": "string", "pattern": "^img-", "allOf": [{ "pattern": r"\.png$" }] }
        }))
    );
}

#[test]
fn a_second_pattern_is_unwritten_when_the_all_of_beside_it_is_not_a_list() {
    #[derive(Validate)]
    struct Upload {
        #[validate(prefix = "img-", suffix = ".png")]
        name: String,
    }

    let unwritten = unwritten_rules_of::<Upload>(object_with(json!({
        "name": { "type": "string", "allOf": "not a list" }
    })));

    assert_eq!(
        unwritten,
        vec![left_unwritten("/properties/name", "suffix", Reason::NoSchema)]
    );
}

#[test]
fn the_rules_of_each_element_are_written_into_items() {
    #[derive(Validate)]
    struct Post {
        #[validate(each(length(max = 20)))]
        tags: Vec<String>,
    }

    let tags = json!({ "type": "array", "items": { "type": "string" } });
    let schema = with_the_rules_of::<Post>(object_with(json!({ "tags": tags })));

    assert_eq!(
        schema,
        object_with(json!({
            "tags": { "type": "array", "items": { "type": "string", "maxLength": 20 } }
        }))
    );
}

#[test]
fn the_rules_of_each_value_of_a_map_are_written_into_additional_properties() {
    #[derive(Validate)]
    struct Post {
        #[validate(each(length(max = 20)))]
        labels: BTreeMap<String, String>,
    }

    let labels = json!({ "type": "object", "additionalProperties": { "type": "string" } });
    let schema = with_the_rules_of::<Post>(object_with(json!({ "labels": labels })));

    assert_eq!(
        schema,
        object_with(json!({
            "labels": { "type": "object", "additionalProperties": { "type": "string", "maxLength": 20 } }
        }))
    );
}

#[test]
fn a_field_is_found_under_the_name_serde_reads_it_by() {
    #[derive(Validate)]
    #[serde(rename_all = "camelCase")]
    struct Member {
        #[validate(length(max = 50))]
        full_name: String,
    }

    let schema = with_the_rules_of::<Member>(object_with(json!({ "fullName": { "type": "string" } })));

    assert_eq!(
        schema,
        object_with(json!({ "fullName": { "type": "string", "maxLength": 50 } }))
    );
}

#[test]
fn a_rule_writes_nothing_on_a_schema_of_another_type() {
    #[derive(Validate)]
    struct Upload {
        #[validate(length(max = 1024))]
        content: Vec<u8>,
        #[validate(range(max = 10))]
        priority: u8,
    }

    let schema = with_the_rules_of::<Upload>(object_with(json!({
        "content": { "type": "string", "contentEncoding": "base64" },
        "priority": { "type": "string" }
    })));

    assert_eq!(
        schema,
        object_with(json!({
            "content": { "type": "string", "contentEncoding": "base64" },
            "priority": { "type": "string" }
        }))
    );
}

#[test]
fn a_property_that_refers_to_another_schema_and_one_the_schema_lacks_are_left_alone() {
    #[derive(Validate)]
    struct Member {
        #[validate(length(max = 50))]
        name: String,
        #[validate(length(max = 50))]
        nickname: String,
    }

    let schema = with_the_rules_of::<Member>(object_with(json!({ "name": { "$ref": "#/components/schemas/Name" } })));

    assert_eq!(
        schema,
        object_with(json!({ "name": { "$ref": "#/components/schemas/Name" } }))
    );
}

#[test]
fn a_looser_keyword_already_in_the_schema_is_tightened() {
    #[derive(Validate)]
    struct Member {
        #[validate(length(min = 2, max = 50))]
        name: String,
        #[validate(range(min = 18, max = 120))]
        age: u8,
    }

    let schema = with_the_rules_of::<Member>(object_with(json!({
        "name": { "type": "string", "minLength": 1, "maxLength": 80 },
        "age": { "type": "integer", "minimum": 0, "maximum": 120.5 }
    })));

    assert_eq!(
        schema,
        object_with(json!({
            "name": { "type": "string", "minLength": 2, "maxLength": 50 },
            "age": { "type": "integer", "minimum": 18, "maximum": 120 }
        }))
    );
}

#[test]
fn a_tighter_keyword_already_in_the_schema_is_kept() {
    #[derive(Validate)]
    struct Member {
        #[validate(length(min = 2, max = 50))]
        name: String,
        #[validate(range(min = 18, max = 120))]
        age: u8,
    }

    let schema = with_the_rules_of::<Member>(object_with(json!({
        "name": { "type": "string", "minLength": 5, "maxLength": 20 },
        "age": { "type": "integer", "minimum": 18.5, "maximum": 99 }
    })));

    assert_eq!(
        schema,
        object_with(json!({
            "name": { "type": "string", "minLength": 5, "maxLength": 20 },
            "age": { "type": "integer", "minimum": 18.5, "maximum": 99 }
        }))
    );
}

#[test]
fn a_format_already_in_the_schema_is_kept() {
    #[derive(Validate)]
    struct Member {
        #[validate(email)]
        email: String,
    }

    let email = json!({ "type": "string", "format": "idn-email" });
    let schema = with_the_rules_of::<Member>(object_with(json!({ "email": email })));

    assert_eq!(
        schema,
        object_with(json!({ "email": { "type": "string", "format": "idn-email" } }))
    );
}

#[test]
fn writing_the_rules_a_second_time_changes_nothing() {
    #[derive(Validate)]
    struct Upload {
        #[validate(required, length(max = 50), prefix = "img-", suffix = ".png")]
        name: Option<String>,
    }

    let written_once = with_the_rules_of::<Upload>(object_with(json!({ "name": { "type": ["string", "null"] } })));
    let written_twice = with_the_rules_of::<Upload>(written_once.clone());

    assert_eq!(
        (written_once, written_twice),
        (
            json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": ["string", "null"],
                        "maxLength": 50,
                        "pattern": "^img-",
                        "allOf": [{ "pattern": r"\.png$" }]
                    }
                },
                "required": ["name"]
            }),
            json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": ["string", "null"],
                        "maxLength": 50,
                        "pattern": "^img-",
                        "allOf": [{ "pattern": r"\.png$" }]
                    }
                },
                "required": ["name"]
            })
        )
    );
}

#[test]
fn a_field_declared_in_a_member_of_all_of_is_written_there() {
    #[derive(Validate)]
    struct Member {
        #[validate(length(max = 50))]
        name: String,
    }

    let schema = with_the_rules_of::<Member>(json!({
        "allOf": [
            { "$ref": "#/components/schemas/Audited" },
            { "type": "object", "properties": { "name": { "type": "string" } } }
        ]
    }));

    assert_eq!(
        schema,
        json!({
            "allOf": [
                { "$ref": "#/components/schemas/Audited" },
                { "type": "object", "properties": { "name": { "type": "string", "maxLength": 50 } } }
            ]
        })
    );
}

#[test]
fn a_field_declared_in_a_member_of_a_member_of_all_of_is_written_there() {
    #[derive(Validate)]
    struct Member {
        #[validate(length(max = 50))]
        name: String,
    }

    let schema = with_the_rules_of::<Member>(json!({
        "allOf": [{ "allOf": [{ "type": "object", "properties": { "name": { "type": "string" } } }] }]
    }));

    assert_eq!(
        schema,
        json!({
            "allOf": [{
                "allOf": [{ "type": "object", "properties": { "name": { "type": "string", "maxLength": 50 } } }]
            }]
        })
    );
}

#[test]
fn a_required_field_declared_in_a_member_of_all_of_is_listed_as_required_there() {
    #[derive(Validate)]
    struct Member {
        #[validate(required)]
        nickname: Option<String>,
    }

    let schema = with_the_rules_of::<Member>(json!({
        "allOf": [
            { "$ref": "#/components/schemas/Audited" },
            { "type": "object", "properties": { "nickname": { "type": ["string", "null"] } } }
        ]
    }));

    assert_eq!(
        schema,
        json!({
            "allOf": [
                { "$ref": "#/components/schemas/Audited" },
                {
                    "type": "object",
                    "properties": { "nickname": { "type": ["string", "null"] } },
                    "required": ["nickname"]
                }
            ]
        })
    );
}

#[test]
fn a_required_field_is_unwritten_when_the_required_of_the_schema_is_not_a_list() {
    #[derive(Validate)]
    struct Member {
        #[validate(required)]
        nickname: Option<String>,
    }

    let unwritten = unwritten_rules_of::<Member>(json!({
        "type": "object",
        "properties": { "nickname": { "type": ["string", "null"] } },
        "required": true
    }));

    assert_eq!(
        unwritten,
        vec![left_unwritten("/properties/nickname", "required", Reason::NoSchema)]
    );
}

#[test]
fn a_required_of_the_schema_that_is_not_a_list_is_left_as_it_is() {
    #[derive(Validate)]
    struct Member {
        #[validate(required)]
        nickname: Option<String>,
    }

    let schema = with_the_rules_of::<Member>(json!({
        "type": "object",
        "properties": { "nickname": { "type": ["string", "null"] } },
        "required": true
    }));

    assert_eq!(
        schema,
        json!({
            "type": "object",
            "properties": { "nickname": { "type": ["string", "null"] } },
            "required": true
        })
    );
}

#[test]
fn a_required_field_is_unwritten_in_the_member_of_all_of_whose_required_is_not_a_list() {
    #[derive(Validate)]
    struct Member {
        #[validate(required)]
        nickname: Option<String>,
    }

    let unwritten = unwritten_rules_of::<Member>(json!({
        "allOf": [
            { "$ref": "#/components/schemas/Audited" },
            {
                "type": "object",
                "properties": { "nickname": { "type": ["string", "null"] } },
                "required": "nickname"
            }
        ]
    }));

    assert_eq!(
        unwritten,
        vec![left_unwritten(
            "/allOf/1/properties/nickname",
            "required",
            Reason::NoSchema
        )]
    );
}

#[test]
fn rules_written_in_full_leave_none_unwritten() {
    #[derive(Validate)]
    struct Member {
        #[validate(required, length(max = 50))]
        name: Option<String>,
        #[validate(each(length(max = 20)))]
        tags: Vec<String>,
    }

    let unwritten = unwritten_rules_of::<Member>(object_with(json!({
        "name": { "type": ["string", "null"] },
        "tags": { "type": "array", "items": { "type": "string" } }
    })));

    assert_eq!(unwritten, vec![]);
}

#[test]
fn a_rule_on_a_field_the_schema_has_no_property_for_is_unwritten() {
    #[derive(Validate)]
    struct Member {
        #[validate(required, length(max = 50))]
        nickname: String,
    }

    let unwritten = unwritten_rules_of::<Member>(object_with(json!({})));

    assert_eq!(
        unwritten,
        vec![
            left_unwritten("/properties/nickname", "required", Reason::NoSchema),
            left_unwritten("/properties/nickname", "length", Reason::NoSchema)
        ]
    );
}

#[test]
fn a_rule_on_each_element_of_a_list_without_items_is_unwritten() {
    #[derive(Validate)]
    struct Post {
        #[validate(each(length(max = 20)))]
        tags: Vec<String>,
    }

    let unwritten = unwritten_rules_of::<Post>(object_with(json!({ "tags": { "type": "array" } })));

    assert_eq!(
        unwritten,
        vec![left_unwritten("/properties/tags/items", "length", Reason::NoSchema)]
    );
}

#[test]
fn a_rule_on_a_property_that_refers_to_another_schema_is_unwritten() {
    #[derive(Validate)]
    struct Member {
        #[validate(length(max = 50))]
        name: String,
    }

    let unwritten =
        unwritten_rules_of::<Member>(object_with(json!({ "name": { "$ref": "#/components/schemas/Name" } })));

    assert_eq!(
        unwritten,
        vec![left_unwritten("/properties/name", "length", Reason::Reference)]
    );
}

#[test]
fn a_format_and_a_pattern_on_a_property_that_refers_to_another_schema_are_unwritten() {
    #[derive(Validate)]
    struct Member {
        #[validate(email)]
        email: String,
        #[validate(prefix = "@")]
        handle: String,
    }

    let unwritten = unwritten_rules_of::<Member>(object_with(json!({
        "email": { "$ref": "#/components/schemas/Email" },
        "handle": { "$ref": "#/components/schemas/Handle" }
    })));

    assert_eq!(
        unwritten,
        vec![
            left_unwritten("/properties/email", "email", Reason::Reference),
            left_unwritten("/properties/handle", "prefix", Reason::Reference)
        ]
    );
}

#[test]
fn a_rule_on_each_element_of_a_field_the_schema_has_no_property_for_is_unwritten() {
    #[derive(Validate)]
    struct Post {
        #[validate(each(length(max = 20)))]
        tags: Vec<String>,
    }

    let unwritten = unwritten_rules_of::<Post>(object_with(json!({})));

    assert_eq!(
        unwritten,
        vec![left_unwritten("/properties/tags", "length", Reason::NoSchema)]
    );
}

#[test]
fn a_rule_on_each_element_of_a_property_that_refers_to_another_schema_is_unwritten() {
    #[derive(Validate)]
    struct Post {
        #[validate(each(length(max = 20)))]
        tags: Vec<String>,
    }

    let unwritten = unwritten_rules_of::<Post>(object_with(json!({ "tags": { "$ref": "#/components/schemas/Tags" } })));

    assert_eq!(
        unwritten,
        vec![left_unwritten("/properties/tags", "length", Reason::Reference)]
    );
}

#[test]
fn a_rule_on_a_schema_of_another_type_is_unwritten() {
    #[derive(Validate)]
    struct Upload {
        #[validate(length(max = 1024))]
        content: Vec<u8>,
        #[validate(range(max = 10))]
        priority: u8,
    }

    let unwritten = unwritten_rules_of::<Upload>(object_with(json!({
        "content": { "type": "string", "contentEncoding": "base64" },
        "priority": { "type": "string" }
    })));

    assert_eq!(
        unwritten,
        vec![
            left_unwritten("/properties/content", "length", Reason::OtherType),
            left_unwritten("/properties/priority", "range", Reason::OtherType)
        ]
    );
}

#[test]
fn a_rule_that_no_keyword_says_is_unwritten() {
    #[derive(PartialEq, PartialOrd)]
    struct Euros(f64);

    impl Display for Euros {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "{} €", self.0)
        }
    }

    #[derive(Validate)]
    struct Donation {
        #[validate(ip)]
        donor_address: String,
        #[validate(range(min = Euros(0.5)))]
        amount: Euros,
    }

    let unwritten = unwritten_rules_of::<Donation>(object_with(json!({
        "donor_address": { "type": "string" },
        "amount": { "type": "number" }
    })));

    assert_eq!(
        unwritten,
        vec![
            left_unwritten("/properties/donor_address", "ip", Reason::NoKeyword),
            left_unwritten("/properties/amount", "range", Reason::NoKeyword)
        ]
    );
}

#[cfg(feature = "unicode")]
#[test]
fn a_length_in_graphemes_with_only_a_maximum_is_unwritten() {
    #[derive(Validate)]
    struct Member {
        #[validate(length(max = 10, unit = "graphemes"))]
        name: String,
    }

    let unwritten = unwritten_rules_of::<Member>(object_with(json!({ "name": { "type": "string" } })));

    assert_eq!(
        unwritten,
        vec![left_unwritten("/properties/name", "length", Reason::NoKeyword)]
    );
}

#[test]
fn a_rule_unwritten_in_a_member_of_all_of_is_located_there() {
    #[derive(Validate)]
    struct Task {
        #[validate(range(max = 10))]
        priority: u8,
    }

    let unwritten = unwritten_rules_of::<Task>(json!({
        "allOf": [
            { "$ref": "#/components/schemas/Audited" },
            { "type": "object", "properties": { "priority": { "type": "string" } } }
        ]
    }));

    assert_eq!(
        unwritten,
        vec![left_unwritten(
            "/allOf/1/properties/priority",
            "range",
            Reason::OtherType
        )]
    );
}

#[test]
fn the_pointer_to_an_unwritten_rule_escapes_the_field_name() {
    #[derive(Validate)]
    struct Measure {
        #[serde(rename = "size/unit~name")]
        #[validate(length(max = 5))]
        unit: String,
    }

    let unwritten = unwritten_rules_of::<Measure>(object_with(json!({})));

    assert_eq!(
        unwritten,
        vec![left_unwritten(
            "/properties/size~1unit~0name",
            "length",
            Reason::NoSchema
        )]
    );
}

#[test]
fn an_unwritten_rule_says_which_rule_where_and_why() {
    #[derive(Validate)]
    struct Host {
        #[validate(length(max = 50))]
        nickname: String,
        #[validate(length(max = 50))]
        name: String,
        #[validate(range(max = 10))]
        priority: u8,
        #[validate(ip)]
        address: String,
    }
    let mut schema = object_with(json!({
        "name": { "$ref": "#/components/schemas/Name" },
        "priority": { "type": "string" },
        "address": { "type": "string" }
    }));

    let unwritten = json_schema::write_rules(&Host::rules(), schema.as_object_mut().expect("the schema is an object"));

    assert_eq!(
        unwritten.iter().map(ToString::to_string).collect::<Vec<_>>(),
        vec![
            "`length` can't be written at `/properties/nickname`: the schema has no subschema there",
            "`length` can't be written at `/properties/name`: the subschema there is a `$ref`, which isn't followed",
            "`range` can't be written at `/properties/priority`: the subschema there isn't of a type the rule constrains",
            "`ip` can't be written at `/properties/address`: no keyword says what the rule requires"
        ]
    );
}
