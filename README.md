# annotated

[![crates.io](https://img.shields.io/crates/v/annotated)](https://crates.io/crates/annotated)
[![docs.rs](https://img.shields.io/docsrs/annotated)](https://docs.rs/annotated)

Validation for structs and enums that works with serde. You write rules on a struct's fields. When the struct is deserialized, you get every problem back in one error, up to 1,000: broken rules and missing fields together, each with the path of the field it belongs to. If you would rather stop at the first problem, the `_fail_fast` functions do that.

```rust
use annotated::{Deserialize, ParamValue, Path, Problem, Validate};

#[derive(Validate, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct NewMember {
    #[validate(required, length(max = 50))]
    full_name: String,
    #[validate(email)]
    email: String,
    #[validate(range(min = 18, max = 120))]
    age: u8,
}

#[test]
fn every_problem_with_a_new_member_is_reported_at_once() {
    let body = r#"{ "fullName": "  ", "age": 12 }"#;

    let error = annotated::json::from_str::<NewMember>(body).unwrap_err();

    let [blank_name, missing_email, too_young] = error.issues() else {
        panic!("expected three issues, got {error:?}");
    };
    assert_eq!(blank_name.path(), &Path::root().field("fullName"));
    assert_eq!(blank_name.problem(), &Problem::Required);
    assert_eq!(missing_email.path(), &Path::root().field("email"));
    assert_eq!(missing_email.problem(), &Problem::Missing);
    assert_eq!(too_young.path(), &Path::root().field("age"));
    assert_eq!(too_young.problem().code(), "range");
    assert_eq!(
        too_young.problem().params(),
        [("min", ParamValue::Integer(18)), ("max", ParamValue::Integer(120))],
    );
}
```

- **Every problem at once.** A client fixing a request sees everything that is wrong with it in one response, instead of one field per round trip.
- **Field names as serde reads them.** The example reports `fullName`, not `full_name`, because of its `rename_all`.
- **Issues are data.** Each one has a path, a stable code, typed parameters and an English message you can replace.
- **Nothing to pay for valid input.** Validating a valid value allocates nothing.
- **Any serde format.** JSON has ready-made functions, and every other format works through its `Deserializer`.

> **Status:** before 1.0. Until 1.0, a release may change the API. When one does, the changelog says what changed.

## Getting started

```sh
cargo add annotated --features json
```

`annotated` is the only crate to add. It re-exports its derive macros, and the code they generate doesn't need serde among your dependencies. With axum, add [annotated-axum](#web-frameworks) too, and with utoipa, [annotated-utoipa](#openapi-and-json-schema).

1. Derive `annotated::Deserialize` where you would derive `serde::Deserialize`.
2. Write each field's rules in `#[validate(...)]`.
3. Deserialize with `annotated::json::from_str`, and read the issues from the error.

To check a value that already exists, derive `annotated::Validate` and call `value.validate()`. It works on its own or beside `Deserialize`, from the same `#[validate(...)]` rules.

## Where it fits

[validator](https://crates.io/crates/validator) and [garde](https://crates.io/crates/garde) check a struct after serde has built it. If a field is missing or has the wrong type, serde stops at that first error, and no rule ever runs.

`annotated` validates while it deserializes, so a missing field is one more issue instead of the end of deserialization. The struct still implements `serde::Deserialize`, so it works anywhere serde does.

## Rules

| Rule | The value passes when |
|---|---|
| `required` | It is present: a string isn't empty or only whitespace, a collection isn't empty. On an `Option`, its key is in the input, and `null` is accepted |
| `length(min = 1, max = 50)` | Its length is within the bounds, or exactly `length(equal = 5)`. A string is counted in `char`s, or in bytes or graphemes with `unit = "bytes"` or `unit = "graphemes"`. A collection is counted in elements |
| `range(min = 18, max = 120)` | It is within the bounds, both included |
| `email` | It is an email address, as HTML's `<input type="email">` accepts one, of at most 254 bytes |
| `url` | It is an absolute URL with a host and no userinfo (`user@`), using `http`, `https` or `ftp`. `url(schemes = &["https"])` accepts other schemes instead. A host that ends in a number must be an IPv4 address like `192.0.2.16`, the port is at most 65535, and whitespace or invisible characters break the rule anywhere |
| `ip`, `ipv4`, `ipv6` | It is an IP address: of either version, only version 4, or only version 6 |
| `contains = "@"`, `prefix = "SKU-"`, `suffix = ".pdf"` | The string contains, starts with or ends with the text |
| `pattern = r"^\d{5}$"` | The string matches the regular expression, anywhere in it unless the expression is anchored with `^` and `$`. Needs the `regex` feature |
| `custom(function)` | Your function, any `Fn(&T) -> Result<(), Problem>`, returns `Ok(())` |
| `nested` | The field is a struct or an enum with rules of its own, and it passes them. It can be behind a `Box`, an `Rc`, an `Arc` or a reference, and a tuple's elements are each checked at their index |
| `each(...)` | Every element of the collection, or every value of the map, passes the rules inside: `each(length(max = 20))`, `each(nested)` |
| `not_nested` | Always: it checks nothing, and says that the rules of the field's type are left unchecked on purpose |

- **Nested values.** A field whose type has rules of its own, a struct deriving `Validate` or `Deserialize`, or an `Option`, a `Vec` or a map of one, needs `nested`, or `each(nested)` for a collection's elements. Without it, the field is a compile error: `validate()` would never check those rules, and deserializing would have serde read the value, so its issues would end deserialization as one error. `#[validate(not_nested)]` leaves them unchecked on purpose: `validate()` doesn't check them, and deserializing reads the field as serde does. For a type that mentions a type parameter, the derives can't tell, as [Generic types](#generic-types-and-re-exports) says.
- **Order.** A field's rules run in the order written, and the field stops at its first broken rule. So a blank name isn't also reported as too short. Put `#[validate(report_all)]` on the struct to report every broken rule instead.
- **`Option` fields.** `required` asks for the field's key. Deserializing reports an absent key as `missing`, as for a field of any other type, even when serde would give it a `default`, and reads `null` as `None`. A value that already exists can't say whether its key was there, so `validate()` checks nothing for `required` on an `Option`. To refuse `null` too, declare the field as `T` instead. Every other rule checks the value inside, and is skipped when the field is `None`.
- **Arguments.** A rule takes each named argument once. Bounds that contradict each other, such as `length(min = 10, max = 5)` or `equal` beside `min`, are a compile error when they are written as literals. Written as constants, they compile, and the rule refuses every value.
- **Messages.** Give a rule `message = "..."` to replace its English message. `{param}` placeholders are filled from the problem's parameters: `length(max = 50, message = "use {max} characters or fewer")`. So are those in the message of your own `Problem::custom(...)`, from the parameters `with_param` gives it.
- **Rules across fields.** `#[validate(custom(function))]` on the struct checks the whole struct, after its fields, with any `Fn(&Self) -> Result<(), Problem>`. `custom(function, field = repeated_password)` reports the problem at that field.
- **Enums.** Rules go on the fields of an enum's variants, named or not. The variant the value holds is checked, and each issue is at the path where serde reads its value, such as `card.number` under the variant's name. For a rule across a variant's fields, put `#[validate(custom(function))]` on the variant: the function gets the enum, and runs when the value is that variant. `custom(function, field = expires_in)` reports the problem at that field.
- **Rules of your own.** Besides `custom`, a rule can be a type of yours that implements `Rule`. Write it by its path, with the same syntax as the built-in rules: `my_rules::iban(country = "ES")`.
- **No I/O.** A rule looks at the value and nothing else. A check that needs I/O, like "is this email already taken", belongs in your application, after validation. To report its issue with the validation's, push it into the report with `Report::push_issue(Issue::new(path, problem))`.

## Reading the issues

The error holds its issues in the order of the struct's fields, followed by those of struct-level rules. Each `Issue` gives you:

| Call | What you get | For the age in the example |
|---|---|---|
| `path()` | Where the problem is. `to_string()` gives a dotted path, and `json_pointer()` an RFC 6901 JSON Pointer | `age` and `/age`. Deeper in a value, `members[1].name` and `/members/1/name` |
| `problem().code()` | A stable code, for code that translates the problem or reacts to it | `range` |
| `problem().params()` | The rule's parameters, as numbers or text | `min` is `18` and `max` is `120` |
| `message()` | A message for people: the rule's `message` when it has one, or else the English one | `must be between 18 and 120` |

In a path, a map's key longer than 256 characters is cut to its first 256 followed by `…`, so an issue holds a bounded copy of the key however long it is in the input.

Printed, the error is one issue per line: `age: must be between 18 and 120`. A map's key is printed with a newline, and any other char that could break a line or hide text, escaped: `labels[a\nb]`.

`Issues`, `Issue`, `Path`, `Problem` and `ParamValue` implement serde's `Serialize`, so an app can send the issues as they are. In JSON, the issues are a list, and each issue is its path as a list of segments, a field or a key as a string and an index as a number, its code, its params as an object in their order, and its message:

```json
[
  { "path": ["fullName"], "code": "required", "params": {}, "message": "is required" },
  { "path": ["age"], "code": "range", "params": { "min": 18, "max": 120 }, "message": "must be between 18 and 120" }
]
```

## Ways to validate

| You call | For input that breaks the rules, you get |
|---|---|
| `annotated::json::from_str` or `from_slice`, with the `json` feature | A `DeserializeError` holding the issues as data, as in the example |
| `annotated::from_deserializer`, with any serde `Deserializer` | The same |
| `serde_json::from_str`, or anything else that goes through `serde::Deserialize` | A serde error with the issues in its message. A serde error can only carry a message, not data |
| `value.validate()`, on a struct that derives `Validate` | `Issues`, for a value that already exists, whether or not serde built it |

To stop at the first issue, call `from_str_fail_fast`, `from_slice_fail_fast`, `from_deserializer_fail_fast` or `validate_fail_fast` instead. Each returns only the issue the full report would list first.

Each way keeps at most 1,000 issues, `Report::DEFAULT_LIMIT`, since a body can have as many issues as it is long: a list of a million `{}` is a million missing fields for each field its elements require.

- The issues kept are listed in the order the full report lists them. For a struct, an enum or a list, they are its first ones. Past the limit, which of a map's issues are kept is unspecified, except that failing fast keeps the first.
- `Issues::is_complete()` is `false` when the limit was reached, so the value may have more issues than are listed.
- To keep another number, call `from_str_limited_to`, `from_slice_limited_to`, `from_deserializer_limited_to` or `validate_limited_to`. A limit of 0 is taken as 1. `Report::unlimited()` keeps every issue, for values you trust.
- Going through `serde::Deserialize` always keeps at most 1,000.

## Values of the wrong type

A value of the wrong type, like `"age": "twelve"`, ends deserialization with the format's own error, as it does with serde. Put `#[annotated(collect_type_errors)]` on the struct to get it as one more issue instead, with the code `invalid_type` and the message `must be u8, found string`. The issue names the kind of value found, never the value itself, so nothing a client sent is echoed back. A key given twice, or an unknown key a struct denies, is still the format's error. This needs a self-describing format, such as JSON. The struct buffers each value before it reads it, and a buffer holds the value, not the input's text, so a few values a format reads only for their own type are of the wrong type: an integer beyond 64 bits in JSON, unless `serde_json`'s `arbitrary_precision` feature is on, a 128-bit integer in MessagePack, and `serde_json`'s `RawValue`.

## Features

| Feature | What it adds |
|---|---|
| `serde`, on by default | `#[derive(Deserialize)]`, `annotated::from_deserializer`, and `Serialize` for the issues |
| `json` | `annotated::json`, built on `serde_json` |
| `regex` | The `pattern` rule |
| `unicode` | `length(unit = "graphemes")` |
| `json-schema` | `annotated::json_schema`, which writes a struct's rules into its JSON Schema, built on `serde_json` |

## Generic types and re-exports

- **Generic types.** As serde's derive does, both derives bound each type parameter a field reads or checks: by `annotated::Validate` for the values `nested` checks, by `annotated::Deserialize<'de>` for those it reads, and by serde's `Deserialize<'de>` for the rest. When a bound asks more than a type needs, give `Deserialize`'s bounds with `#[serde(bound = "...")]`, as with serde. Whether a `T` has rules of its own is known only where the type is used, so the derives can't tell: a field such as `items: Vec<T>` compiles without `nested`, whatever `T` is.
- **Re-exports.** A crate that re-exports annotated, so that its users don't depend on it, puts `#[annotated(crate = "my_api::checks")]` on each type deriving through it. `#[serde(crate = "...")]` names another path to serde, as with serde's derive.

## Web frameworks

[annotated-axum](crates/annotated-axum) is an axum extractor, `ValidJson<T>`. It answers the issues in a body, up to 1,000, with 422 and RFC 9457 problem details, which list each issue's JSON Pointer, code, params and message. Its rejection holds the issues as data, for an app that answers them with a response of its own.

## OpenAPI and JSON Schema

A schema library documents a type's fields, and knows nothing of its rules. With the `json-schema` feature, annotated writes them into the schema that library generated:

```rust
let unwritten = annotated::json_schema::write_rules(&<NewMember as Validate>::rules(), schema);
```

- `length(max = 50)` becomes `maxLength: 50`, `range(min = 18)` becomes `minimum: 18`, and `required` on an `Option` makes the field required, with `null` still allowed. The [module's documentation](https://docs.rs/annotated/latest/annotated/json_schema/) lists what each rule writes.
- The schema never rejects a value the rules accept. Where no keyword says exactly what a rule requires, a looser one is written, or none.
- Each rule it couldn't write is returned, with where its keyword belongs and why: `ip`, which no keyword says, or a property that is a `$ref`. Your app decides which of them its document can do without.
- The schema is JSON, so this works with any library that generates JSON Schema or OpenAPI 3.1, and annotated depends on none of them.
- It covers the rules on a struct's fields. Enums, flattened fields and rules across fields are not written yet.
- `Validate` and `Deserialize` both have `rules()`, since a type can derive either. `NewMember` derives both, so the call says which one it means.

[annotated-utoipa](crates/annotated-utoipa) writes the rules into the document utoipa generates, and returns what the document still doesn't say.

## Learn more

- The [API documentation](https://docs.rs/annotated) describes every rule, type and option.
- The [changelog](CHANGELOG.md) lists what each release changed.
- [ARCHITECTURE.md](ARCHITECTURE.md) explains how the library is built, with the principles and terms it keeps.

## Minimum supported Rust version

The minimum is Rust 1.88. It is raised only when annotated needs a newer Rust, and never to a release less than six months old. Raising the minimum is not a breaking change, and the changelog notes each time it happens.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
