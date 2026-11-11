# annotated

Validation for structs that works with serde. You write rules on a struct's fields, and deserializing it reports every problem in one error: broken rules and missing fields together, each at the path of the field it's about.

```rust
use annotated::{Deserialize, Validate};

#[derive(Validate, Deserialize)]
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

    assert_eq!(
        error.to_string(),
        "fullName: is required\n\
         email: is missing\n\
         age: must be between 18 and 120",
    );
}
```

> **Status:** under construction. Nothing is on crates.io yet, and the API above is where it's heading, not what's there today.

## Where it fits

[validator](https://crates.io/crates/validator) and [garde](https://crates.io/crates/garde) check a struct after serde has built it. When a field is missing or has the wrong type, serde stops at that first error, and no rule runs.

`annotated` validates as part of deserializing. Derive `annotated::Deserialize` where you'd derive `serde::Deserialize`: the struct still implements `serde::Deserialize` and works anywhere serde does, but a missing field becomes one more violation instead of the end of deserialization. Values of the wrong type join them when a struct opts in with `#[annotated(collect_type_errors)]`.

To check a value however it was built, derive `Validate` alone: `value.validate()` reports every broken rule, whether serde built the value or not.

## Principles

- **Every problem at once.** A client fixing a request learns about all of it in one answer, not one field per round trip.
- **serde decides the names.** Fields are reported by the name serde reads them by, under any `rename_all`, `rename` or `alias`, worked out by the same code serde's derive uses. The crate never invents a naming scheme of its own.
- **The format is yours.** The library works with any serde format and reports paths as segments (field, index, key). Rendering them, as a JSON Pointer or a dotted path, is a choice its user makes.
- **Problems are data.** Each one has a stable code and its parameters for code that translates or reacts to it, an English message for people, and a per-rule `message` override.
- **Nothing to pay on the success path.** Validating a valid value allocates nothing. Paths are built only when something fails, rules are plain functions the compiler inlines, and a regular expression compiles once.
- **Open for your rules.** Built-in rules are ordinary functions, and yours sit beside them: on a field with `custom(...)`, or on the whole struct.
- **Rules are pure.** A rule looks only at the value. Anything that needs I/O, like "is this email already taken", belongs to your application, after validation.

## Terms

- **Rule:** a check a value must pass, written in `#[validate(...)]`: `required`, `length(max = 50)`, `email`.
- **Violation:** one broken rule, missing field or value of the wrong type, at a path.
- **Problem:** what kind of violation it is: a stable code, its parameters and an English message.
- **Path:** where in the value a violation happened: a field, an element's index, a map's key, or the value itself.
- **Violations:** every violation found in one validation. It's the error type, and is never empty.
- **Validation:** checking a value that already exists against its rules, with `Validate::validate`.
- **Collecting deserialization:** deserializing through `annotated::Deserialize`, which reports missing fields, rule violations and, when a struct opts in, values of the wrong type, all together.

## Crates

| Crate | What it holds |
|---|---|
| `annotated` | The `Validate` trait, violations, problems and paths, the built-in rules, and collecting deserialization. Re-exports the derives. |
| `annotated-derive` | The `Validate` and `Deserialize` derive macros. A derive macro needs a crate of its own. |
| `annotated-axum` | An axum extractor that answers 422 with every violation. |

## Minimum supported Rust version

The last three stable releases, today Rust 1.97. Raising it is not a breaking change, and each raise is noted in the changelog.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
