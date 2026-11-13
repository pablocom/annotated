//! With the `json-schema` feature, `annotated::json_schema::write_rules` writes a struct's rules into its JSON Schema.
//! The schema never rejects a value the rules accept.

#![cfg(feature = "json-schema")]
// The types here are only described, never built.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fmt::{self, Display};

use annotated::{Problem, Validate, json_schema};
use serde_json::{Map, Value, json};

fn with_the_rules_of<T: Validate>(mut schema: Value) -> Value {
    let schema_object = schema
        .as_object_mut()
        .expect("a schema to write rules into is an object");
    json_schema::write_rules(&T::rules(), schema_object);
    schema
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
fn required_on_an_option_refuses_null_and_lists_the_field_as_required() {
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
            "properties": { "nickname": { "type": "string" }, "age": { "type": "integer" } },
            "required": ["age", "nickname"]
        })
    );
}

#[test]
fn required_on_an_option_refuses_null_among_alternatives() {
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
                "address": { "$ref": "#/components/schemas/Address" },
                "billing": { "$ref": "#/$defs/Address" }
            },
            "required": ["address", "billing"]
        })
    );
}

#[test]
fn the_alternative_left_once_null_is_refused_stays_one_when_it_would_replace_a_keyword() {
    #[derive(Validate)]
    struct Member {
        #[validate(required)]
        nickname: Option<String>,
        #[validate(required)]
        pronouns: Option<String>,
    }

    let schema = with_the_rules_of::<Member>(object_with(json!({
        "nickname": {
            "description": "What to call them",
            "oneOf": [{ "type": "null" }, { "type": "string", "description": "A name" }]
        },
        "pronouns": {
            "oneOf": [{ "type": "null" }, { "type": "string" }, { "type": "array" }]
        }
    })));

    assert_eq!(
        schema,
        json!({
            "type": "object",
            "properties": {
                "nickname": {
                    "description": "What to call them",
                    "oneOf": [{ "type": "string", "description": "A name" }]
                },
                "pronouns": {
                    "oneOf": [{ "type": "string" }, { "type": "array" }]
                }
            },
            "required": ["nickname", "pronouns"]
        })
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
                        "type": "string",
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
                        "type": "string",
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
