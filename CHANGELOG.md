# Changelog

This file lists every change that users of the crates can notice. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crates follow [Semantic Versioning](https://semver.org/).

## Unreleased

### Changed

- **Breaking:** a violation is now called an issue, a plainer word for one thing wrong with a value. `Violation` is now `Issue`, `Violations` is `Issues` and `FoundViolations` is `FoundIssues`. `DeserializeError::Violations` is now `DeserializeError::Issues`, and its `violations()` method is `issues()`. Nothing else changes: codes, parameters and messages stay the same.

## [0.1.0] - 2026-11-12

### Added

- The `Validate` trait. `validate()` returns every violation found. With it comes the error model: `Violations`, `Violation`, `Problem` with a stable `code()` and `params()`, `ParamValue` for each parameter (an integer, a float or text), and `Path` with `json_pointer()`. `Location` and `FoundViolations` are there for implementing `Validate` by hand.
- The `Rule` trait, and the first built-in rules in `annotated::rules`:
  - `required` checks anything that implements the `Required` capability trait.
  - `length` counts the `char`s of a string, and the elements of anything that implements `HasLength`. `in_bytes()` counts a string's bytes instead, and `in_graphemes()`, behind the `unicode` feature, its grapheme clusters.
- `#[derive(Validate)]`, re-exported by `annotated`.
  - Rules are written in `#[validate(...)]` on fields: a built-in rule by its name, or your own rule by its path.
  - A rule takes one constructor value (`name = value` or `name(value)`) or named builder arguments (`name(key = value)`), and a `message` override with `{param}` placeholders.
  - Every field is checked. A field stops at its first broken rule, unless the struct has `#[validate(report_all)]`.
  - On an `Option` field, `required` checks the `Option` itself, and every other rule checks the value inside.
  - A field is reported under the name serde reads it by, following `rename_all` and `rename`.
- The `range` rule: the value is at least `min` and at most `max`. It works on anything that is `PartialOrd` and `Display`, and the bounds take their type from the field. Bounds of a primitive number type are reported as numbers, and bounds of any other type as text.
- The `email` rule: the string is an email address, as HTML's `<input type="email">` accepts one, and at most 254 bytes long.
- The `url` rule: the string is an absolute URI with a host, as RFC 3986 defines one, with RFC 3987's international characters and without userinfo. Its scheme must be `http`, `https` or `ftp`, or one of those set with `schemes`, compared ignoring case. The rule allocates nothing.
- The `ip`, `ipv4` and `ipv6` rules: the string is an IP address of either version, of version 4 only, or of version 6 only. Each rule reports a problem whose code is the rule's name.
- The `contains`, `prefix` and `suffix` rules: the string contains, starts with or ends with a given text.
- The `custom` rule: a function of your own, any `Fn(&V) -> Result<(), Problem>`. It checks any value that borrows as a `V`, so a function taking `&str` also checks a `String` field. It reports with `Problem::custom`, and `with_param` adds a parameter that is a number or text.
- The `pattern` rule, behind the `regex` feature: the string matches a regular expression. In `#[validate(...)]`, the expression is a string literal. It is checked at compile time and compiled once, into a `static` at the field.
- `nested` and `each(...)`.
  - `nested` validates a field whose type implements `Validate`, and reports under the field's name.
  - `each(...)` applies its rules to every element of a collection, reported at its index, or to every value of a map, reported at its key. It works through the `HasElements` capability trait, also behind a reference or a `Box`, like `&'a [T]` or `Box<[T]>`.
  - `each(nested)` validates every element, and an `each` can hold another `each`.
- Struct-level `#[validate(custom(f))]`: any `Fn(&Self) -> Result<(), Problem>`.
  - It runs after every field and reports at the struct's own location. `custom(f, field = name)` reports at that field instead, under its serde name.
  - A struct can have several, which run in the order written, and they can sit beside `report_all`.
  - A rule with `field` counts as that field's last rule, so it is skipped once the field has a violation, unless the struct has `report_all`.
- `Validate::validate_fail_fast()`, which returns only the first violation. It is the one `validate()` lists first, even under `report_all`.
  - Checking stops right after that violation, with one exception. When a nested value finds it, the struct around that value may check more before it stops, but it records nothing more.
  - For implementing `Validate` by hand, `FoundViolations::failing_fast()` starts such a report, `is_finished()` returns whether it has its violation, and a finished report records nothing more.
  - The `check` closure of `HasElements::for_each_element` returns whether to go on. Once it returns `false`, the remaining elements are skipped.
- `Problem::Missing`: a field is absent from the input being deserialized. Its code is `missing` and its message is `is missing`.
- `Problem::InvalidType`: a value has the wrong type for its field. Its code is `invalid_type` and its message is `must be {expected}, found {found}`. The `expected` parameter is what the field's type accepts, as serde describes it. The `found` parameter is the kind of value in the input (`string`, `integer`, `null`, …), never the value itself.
- The `serde` feature, on by default, and with it the `Deserialize` trait for collecting deserialization.
  - `deserialize_at` reads a value at a location and pushes every violation in it into a `FoundViolations`. It returns the value, `None` when the value couldn't be built, or the format's own error.
  - `from_deserializer` reads a value from the root. It returns the value or a `DeserializeError`, which is either the format's error or every violation found. `violations()` returns the violations as a slice in both cases, empty for the format's error.
  - `from_deserializer_fail_fast` returns only the first violation.
- `#[derive(Deserialize)]`, behind the `serde` feature, for a struct with named fields.
  - It implements `annotated::Deserialize`, and serde's `Deserialize` through it.
  - The struct is read by its serde names, following `rename_all`, `rename` and `alias`, and it supports `deny_unknown_fields` and `expecting`.
  - Once the input is read, every field is checked in declaration order. A field absent from the input is `Problem::Missing`, unless it is an `Option`, which is `None`. A field that is present is checked by its rules as `Validate` checks it.
  - Struct-level rules run last, and only when every field was present.
  - Through serde's `Deserialize`, a struct that breaks its rules is an error with every violation in its message.
  - Other `#[serde(...)]` attributes and lifetime parameters are compile errors for now.
- `annotated::Deserialize` for the containers that hold a nested value:
  - `Option<T>` and `Box<T>`.
  - `Box<[T]>`, `Vec<T>` and `VecDeque<T>`, which read each element at its index.
  - `BTreeMap<K, V>` and `HashMap<K, V, S>`, which read each value at its key.
  - A container is built only when everything in it could be built.
  - A map reports its values' violations in the order it iterates over them. When a key appears twice, the later value replaces the earlier one, along with its violations.
  - A sequence or a `HashMap` reserves room for the number of values the format announces, up to a megabyte, as serde does.
  - For implementing `annotated::Deserialize` by hand, `FoundViolations::new_alike` starts a report for one part of a value, gathering as its parent does (every violation, or only the first), and `append` moves its violations into the parent.
- `#[derive(Deserialize)]` reads nested values through `annotated::Deserialize`: a field checked with `nested`, or with `each(...)` holding `nested` at any depth.
  - A field missing inside a nested value is reported at its full path, beside the broken rules, in the order `Validate` reports them.
  - When a nested value couldn't be built, its violations are reported. The other rules of its field are skipped, and so are the struct-level rules of the struct around it.
  - Failing fast still returns the first violation of the full report. A struct whose fields were all present is built even once its report is finished, so that the struct around it can check it.
  - A field's rules can name `nested` only once.
- `#[derive(Deserialize)]` accepts and ignores the `#[serde(...)]` attributes that only change serializing, so a struct can derive serde's `Serialize` beside it. On the struct, these are `into` and `bound(serialize = "...")`. On a field, they are `skip_serializing`, `skip_serializing_if`, `serialize_with`, `getter` and `bound(serialize = "...")`. Attributes that change deserializing are still compile errors.
- `#[annotated(collect_type_errors)]` on a struct that derives `Deserialize`. A value of the wrong type becomes `Problem::InvalidType` at its field, beside the other violations in declaration order, instead of the format's error.
  - Each field is read in two steps: first into a buffer that holds any well-formed value, borrowing the strings and bytes the format lends, then from the buffer as the field's type.
  - `expected` is what serde says the type expects. When the type rejects the value with a message of its own, it is the type's name instead, and for an `Option`, the name of the type inside. For a value inside the field, such as an element of a list, both parameters describe that value.
  - A value of the wrong type is checked by no other rule, and the struct-level rules are skipped.
  - Inside a nested value, a value of the wrong type replaces the violations found in that nested value before it. A nested struct that collects type errors too reports the ones inside it at their full paths.
  - Failing fast still returns the first violation of the full report.
  - JSON writes every map key as a string. So a key written as a string is read as the integer or boolean its type asks for, as `serde_json` reads it.
  - It needs a self-describing format.
  - Any other option in `#[annotated(...)]`, and any option on a field, is a compile error.
- The `json` feature, which adds `serde_json` and turns on `serde`. `annotated::json::from_str` and `from_slice` read a JSON document and collect every violation in it. Like `serde_json`, they reject characters after the value, and that error comes before any violation. `from_str_fail_fast` and `from_slice_fail_fast` return only the first violation.
- `examples/axum`, an axum extractor to copy into your app. It reads a JSON body with `annotated::json::from_slice`, and rejects a request it can't read the way axum's `Json` does. It responds to violations with 422 and RFC 9457 problem details, which list each violation's JSON Pointer, code, params and message.

[0.1.0]: https://github.com/pablocom/annotated/releases/tag/v0.1.0
