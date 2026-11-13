# annotated

[![crates.io](https://img.shields.io/crates/v/annotated)](https://crates.io/crates/annotated)
[![docs.rs](https://img.shields.io/docsrs/annotated)](https://docs.rs/annotated)

Validation for structs and enums that works with serde. You write rules on a struct's fields. When the struct is deserialized, you get every problem back in one error: broken rules and missing fields together, each with the path of the field it belongs to. If you would rather stop at the first problem, the `_fail_fast` functions do that.

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

> **Status:** 0.2. Until 1.0, a release may change the API. When one does, the changelog says what changed.

## Getting started

```toml
[dependencies]
annotated = { version = "0.2", features = ["json"] }
```

`annotated` is the only crate to add. It re-exports its derive macros, and the code they generate doesn't need serde among your dependencies.

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
| `required` | It is present: an `Option` is `Some`, a string isn't empty or only whitespace, a collection isn't empty |
| `length(min = 1, max = 50)` | Its length is within the bounds, or exactly `length(equal = 5)`. A string is counted in `char`s, or in bytes or graphemes with `unit = "bytes"` or `unit = "graphemes"`. A collection is counted in elements |
| `range(min = 18, max = 120)` | It is within the bounds, both included |
| `email` | It is an email address, as HTML's `<input type="email">` accepts one, of at most 254 bytes |
| `url` | It is an absolute URL with a host and no userinfo (`user@`), using `http`, `https` or `ftp`. `url(schemes = &["https"])` accepts other schemes instead |
| `ip`, `ipv4`, `ipv6` | It is an IP address: of either version, only version 4, or only version 6 |
| `contains = "@"`, `prefix = "SKU-"`, `suffix = ".pdf"` | The string contains, starts with or ends with the text |
| `pattern = r"^\d{5}$"` | The string matches the regular expression, anywhere in it unless the expression is anchored with `^` and `$`. Needs the `regex` feature |
| `custom(function)` | Your function, any `Fn(&T) -> Result<(), Problem>`, returns `Ok(())` |
| `nested` | The field is a struct or an enum with rules of its own, and it passes them |
| `each(...)` | Every element of the collection, or every value of the map, passes the rules inside: `each(length(max = 20))`, `each(nested)` |

- **Order.** A field's rules run in the order written, and the field stops at its first broken rule. So a blank name isn't also reported as too short. Put `#[validate(report_all)]` on the struct to report every broken rule instead.
- **`Option` fields.** `required` checks that the field is `Some`. Every other rule checks the value inside, and is skipped when the field is `None`.
- **Messages.** Give a rule `message = "..."` to replace its English message. `{param}` placeholders are filled from the problem's parameters: `length(max = 50, message = "use {max} characters or fewer")`.
- **Rules across fields.** `#[validate(custom(function))]` on the struct checks the whole struct, after its fields, with any `Fn(&Self) -> Result<(), Problem>`. `custom(function, field = repeated_password)` reports the problem at that field.
- **Enums.** Rules go on the fields of an enum's variants, named or not. The variant the value holds is checked, and each issue is at the path where serde reads its value, such as `card.number` under the variant's name. For a rule across a variant's fields, put `#[validate(custom(function))]` on the variant: the function gets the enum, and runs when the value is that variant. `custom(function, field = expires_in)` reports the problem at that field.
- **Rules of your own.** Besides `custom`, a rule can be a type of yours that implements `Rule`. Write it by its path, with the same syntax as the built-in rules: `my_rules::iban(country = "ES")`.
- **No I/O.** A rule looks at the value and nothing else. A check that needs I/O, like "is this email already taken", belongs in your application, after validation.

## Reading the issues

The error holds its issues in the order of the struct's fields, followed by those of struct-level rules. Each `Issue` gives you:

| Call | What you get | For the age in the example |
|---|---|---|
| `path()` | Where the problem is. `to_string()` gives a dotted path, and `json_pointer()` an RFC 6901 JSON Pointer | `age` and `/age`. Deeper in a value, `members[1].name` and `/members/1/name` |
| `problem().code()` | A stable code, for code that translates the problem or reacts to it | `range` |
| `problem().params()` | The rule's parameters, as numbers or text | `min` is `18` and `max` is `120` |
| `message()` | A message for people: the rule's `message` when it has one, or else the English one | `must be between 18 and 120` |

Printed, the error is one issue per line: `age: must be between 18 and 120`.

## Ways to validate

| You call | For input that breaks the rules, you get |
|---|---|
| `annotated::json::from_str` or `from_slice`, with the `json` feature | A `DeserializeError` holding the issues as data, as in the example |
| `annotated::from_deserializer`, with any serde `Deserializer` | The same |
| `serde_json::from_str`, or anything else that goes through `serde::Deserialize` | A serde error with every issue in its message. A serde error can only carry a message, not data |
| `value.validate()`, on a struct that derives `Validate` | `Issues`, for a value that already exists, whether or not serde built it |

To stop at the first issue, call `from_str_fail_fast`, `from_slice_fail_fast`, `from_deserializer_fail_fast` or `validate_fail_fast` instead. Each returns only the issue the full report would list first. Going through `serde::Deserialize` always collects every issue.

## Values of the wrong type

A value of the wrong type, like `"age": "twelve"`, ends deserialization with the format's own error, as it does with serde. Put `#[annotated(collect_type_errors)]` on the struct to get it as one more issue instead, with the code `invalid_type` and the message `must be u8, found string`. The issue names the kind of value found, never the value itself, so nothing a client sent is echoed back. This needs a self-describing format, such as JSON.

## Features

| Feature | What it adds |
|---|---|
| `serde`, on by default | `#[derive(Deserialize)]` and `annotated::from_deserializer` |
| `json` | `annotated::json`, built on `serde_json` |
| `regex` | The `pattern` rule |
| `unicode` | `length(unit = "graphemes")` |
| `json-schema` | `annotated::json_schema`, which writes a struct's rules into its JSON Schema, built on `serde_json` |

## Web frameworks

There is no crate for any framework. [`examples/axum`](examples/axum) is an axum extractor to copy into your app instead. It responds with 422 and lists every issue as RFC 9457 problem details.

## OpenAPI and JSON Schema

A schema library documents a type's fields, and knows nothing of its rules. With the `json-schema` feature, annotated writes them into the schema that library generated:

```rust
annotated::json_schema::write_rules(&<NewMember as Validate>::rules(), schema);
```

- `length(max = 50)` becomes `maxLength: 50`, `range(min = 18)` becomes `minimum: 18`, and `required` on an `Option` makes the field required and never `null`. The [module's documentation](https://docs.rs/annotated/latest/annotated/json_schema/) lists what each rule writes.
- The schema never rejects a value the rules accept. Where no keyword says exactly what a rule requires, a looser one is written, or none.
- The schema is JSON, so this works with any library that generates JSON Schema or OpenAPI 3.1, and annotated depends on none of them.
- It covers the rules on a struct's fields. Enums, flattened fields and rules across fields are not written yet.
- `Validate` and `Deserialize` both have `rules()`, since a type can derive either. `NewMember` derives both, so the call says which one it means.

[`examples/utoipa`](examples/utoipa) writes the rules into a document that utoipa generated.

## Learn more

- The [API documentation](https://docs.rs/annotated) describes every rule, type and option.
- The [changelog](CHANGELOG.md) lists what each release changed.
- [ARCHITECTURE.md](ARCHITECTURE.md) explains how the library is built, with the principles and terms it keeps.

## Minimum supported Rust version

The three most recent stable releases are supported, so today the minimum is Rust 1.98. Raising the minimum is not a breaking change, and the changelog notes each time it happens.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
