# Changelog

This file lists every change that users of the crates can notice. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crates follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- `Rule::problem`: a rule says what it requires, as the problem that every value breaking it has.
  - Each built-in rule answers its own problem: `length().max(50)` answers a `Problem::Length` of at most 50, counted in chars for a string and in elements for a collection. A `length` or a `range` without bounds answers `None`.
  - It answers `None` by default, so `custom(...)` and rules of your own say nothing until they implement it.
- `#[validate(custom(check))]` on a variant of an enum, in both derives: a rule across the variant's fields.
  - `check` gets the enum, and runs only when the value is that variant, after the variant's fields and before the enum's rules. When deserializing, it runs once the variant could be built.
  - The rule reports where the variant's content is in the input, such as `card`. `custom(check, field = name)` reports at that field of the variant instead, and counts as its last rule, as on a struct. A tuple variant's element is named by its index: `field = 1`.
  - A unit variant takes no rules, since it has no fields to check.

### Changed

- `Bounds` is `#[non_exhaustive]`, as `Problem` is, so a `match` on it needs a wildcard arm. A new kind of bounds can then be added without a breaking change.

## [0.2.0] - 2026-11-12

### Added

- `#[derive(Deserialize)]` reads `#[serde(default)]` and `#[serde(default = "path")]` on a field.
  - An absent field takes `Default::default()`, or what the function returns, instead of being `Problem::Missing`. An absent `Option` field takes it instead of `None`.
  - The field's rules check the default like a value read. A `nested` rule checks it with `Validate`, so the nested type must implement `Validate`.
  - The default is made only when the field is absent.
- `#[derive(Deserialize)]` reads `#[serde(default)]` and `#[serde(default = "path")]` on the struct.
  - An absent field without a default of its own takes its value in `Self::default()`, or in what the function returns. Its rules check that value like a value read.
  - The struct's default is made once, and only when such a field is absent.
- `#[derive(Deserialize)]` reads `#[serde(skip_deserializing)]` and `#[serde(skip)]` on a field.
  - The field is never read. It takes its own default, or its value in the struct's default, or `Default::default()`, and its rules check that value like a value read.
  - A key with its name is an unknown field: ignored, or the format's error under `deny_unknown_fields`. A struct read from a sequence has no element for it.
  - Its type needs no `Deserialize` implementation.
- `#[derive(Deserialize)]` reads `#[serde(deserialize_with = "path")]` and `#[serde(with = "module")]` on a field.
  - The field is read by the function, and its rules check the value the function returns. A `nested` rule checks it with `Validate`.
  - As with serde, an absent field read by a function is `Problem::Missing`, even an `Option`, unless it has a default.
  - Without `collect_type_errors`, the function's error ends deserialization.
  - With `collect_type_errors`, a value of the wrong type for the function, one it rejects with `invalid_value`, or a sequence of the wrong length is `Problem::InvalidType` at the field. Any other error the function returns ends deserialization with its message, which may repeat the input and so is never an issue.
- `#[derive(Deserialize)]` reads a struct with lifetime parameters, and `#[serde(borrow)]` and `#[serde(borrow = "'a + 'b")]` on a field.
  - As with serde, a `&str` or `&[u8]` field borrows from the input, and any other field borrows with `borrow`. A `Cow<str>` or `Cow<[u8]>` with `borrow` borrows what the input lends, and owns the rest.
  - The implementation requires the input to outlive each lifetime a field borrows, as serde's does.
  - A value the input can't lend, such as a JSON string with an escape in it, is the format's error, or `Problem::InvalidType` under `collect_type_errors`.
- `#[derive(Deserialize)]` reads `#[serde(flatten)]` on a field.
  - As with serde, the struct keeps every key that isn't one of its fields, and reads each flattened field from them once the input is read: a struct takes the keys it knows, a map or an enum the keys left.
  - A flattened field's issues are at the struct's own location, where its keys are in the input, in the field's place among the others.
  - A flattened `Option` is `None` when its value can't be built from the keys, and the issues found trying are dropped.
  - With `collect_type_errors`, keys that don't fit a flattened type are `Problem::InvalidType` at the struct's location. A `nested` flattened value follows its own type's option.
  - As with serde, a struct with a flattened field is read as a map, so it can't be read from a sequence. Only such a struct keeps the keys it doesn't know, which allocates for each of them.
  - `flatten` in a struct with `deny_unknown_fields` is a compile error.
- `Problem::UnknownVariant`, with the code `unknown_variant`: a value names no variant of its enum. Its parameter `expected` lists the names the enum accepts, comma-separated, and its message is `must be one of card, transfer`. It never repeats the name sent.
- `#[derive(Validate)]` works on enums, with unit, newtype, tuple and struct variants.
  - Rules go on the fields of the variants, named or not, and the variant the value holds is checked.
  - Each issue is at the path where serde reads its value, following the enum's tagging: under the variant's name by default, at the enum's own location with `tag` or `untagged`, and under the content's name with `tag` and `content`. A newtype variant's value is at the variant's location, and a tuple variant's elements are at their index.
  - `#[validate(custom(check))]` and `report_all` work on the enum as on a struct. A `custom` rule reports at the enum's own location, and takes no `field`.
- `#[derive(Deserialize)]` reads enums, as serde reads them by default: from the variant's name.
  - The variant read is checked as a struct's fields are, with each issue at its path in the input: `card.number`, a newtype variant's value at `card`, a tuple variant's elements at `point[1]`. A field missing from a variant is `Problem::Missing` there.
  - The enum takes `rename_all`, `rename_all_fields`, `rename`, `deny_unknown_fields` and `expecting`, a variant takes `rename`, `alias` and `rename_all`, and a variant's fields take what a struct's fields take.
  - A name that is no variant is the format's error. With `collect_type_errors` on the enum, the enum is buffered whole, so it is `Problem::UnknownVariant` at the enum's location instead, and a value that isn't the enum is `Problem::InvalidType` there.
  - `#[validate(custom(check))]` on the enum runs once the variant could be built.
  - Reading an enum buffers nothing, unless it collects type errors.
- `#[derive(Deserialize)]` reads internally tagged enums, with `#[serde(tag = "type")]`.
  - As with serde, every key but the tag is buffered, and the variant is read from them. A sequence starting with the tag is read too.
  - The variant's fields are at the enum's own location, beside the tag.
  - A missing tag is `Problem::Missing` at the tag's name, and a tag that names no variant is `Problem::UnknownVariant` there, even without `collect_type_errors`, so reading goes on.
  - With `collect_type_errors`, a value that isn't a map or a sequence is `Problem::InvalidType` at the enum's location.
- `#[derive(Deserialize)]` reads adjacently tagged enums, with `#[serde(tag = "type", content = "data")]`.
  - As with serde, the content is read straight from the input when the tag comes first, and buffered when it comes after. A sequence of the tag and the content is read too.
  - The variant's fields are under the content's name: `data.email`.
  - A missing tag, or one that names no variant, is reported at the tag's name, as for an internally tagged enum. Missing content is `Problem::Missing` at the content's name, unless the variant needs none: a unit variant, or a newtype variant whose value is an `Option`, which is then `None`, as with serde.
  - With `collect_type_errors`, content of the wrong type is `Problem::InvalidType` at the content's name, and a value that isn't a map or a sequence is `Problem::InvalidType` at the enum's location.
  - With `deny_unknown_fields`, a key other than the tag and the content is the format's error. serde words it as an invalid value. annotated words it as an unknown field, as for a struct.
- `#[derive(Deserialize)]` reads untagged enums, with `#[serde(untagged)]`, and untagged variants in a tagged enum.
  - As with serde, the value is buffered once, and the variants are tried in order on that buffer, after the tagged ones. The value is the first variant that can be built. Its broken rules are reported, and the issues found trying the other variants are dropped.
  - The variant's fields are at the enum's own location.
  - A value no variant fits is `Problem::InvalidType` at the enum's location, even without `collect_type_errors`, so reading goes on. Its `expected` is the enum's `expecting`, or `untagged enum Name`.
- `#[derive(Deserialize)]` reads `skip_deserializing`, `skip`, `other`, `deserialize_with`, `with` and `borrow` on a variant, as serde does.
  - A skipped variant is never read: its name is unknown, and its fields need no `Deserialize`.
  - A unit variant with `other` is read for any tag that names no other variant, so the enum never reports `Problem::UnknownVariant`.
  - A variant with `deserialize_with` or `with` is read by the function, and its fields' rules check the values the function returns. A `nested` rule checks one with `Validate`.

### Changed

- `#[derive(Validate)]` reports a field with `#[serde(flatten)]` at the struct's own location, where serde reads its keys, instead of under the field's name.
- Under `collect_type_errors`, a field holding a serde enum reports a name that is none of its variants as `Problem::UnknownVariant`, instead of `Problem::InvalidType`.
- `Problem::InvalidType` is also reported, always, for a value that fits no variant of an untagged enum.
- The minimum supported Rust version is 1.98, since Rust 1.100 is out.

## [0.1.0] - 2026-11-12

### Added

- The `Validate` trait. `validate()` returns every issue found. With it comes the error model: `Issues`, `Issue`, `Problem` with a stable `code()` and `params()`, `ParamValue` for each parameter (an integer, a float or text), and `Path` with `json_pointer()`. `Location` and `Report` are there for implementing `Validate` by hand.
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
  - A rule with `field` counts as that field's last rule, so it is skipped once the field has an issue, unless the struct has `report_all`.
- `Validate::validate_fail_fast()`, which returns only the first issue. It is the one `validate()` lists first, even under `report_all`.
  - Checking stops right after that issue, with one exception. When a nested value finds it, the struct around that value may check more before it stops, but it records nothing more.
  - For implementing `Validate` by hand, `Report::failing_fast()` starts such a report, `is_finished()` returns whether it has its issue, and a finished report records nothing more.
  - The `check` closure of `HasElements::for_each_element` returns whether to go on. Once it returns `false`, the remaining elements are skipped.
- `Problem::Missing`: a field is absent from the input being deserialized. Its code is `missing` and its message is `is missing`.
- `Problem::InvalidType`: a value has the wrong type for its field. Its code is `invalid_type` and its message is `must be {expected}, found {found}`. The `expected` parameter is what the field's type accepts, as serde describes it. The `found` parameter is the kind of value in the input (`string`, `integer`, `null`, …), never the value itself.
- The `serde` feature, on by default, and with it the `Deserialize` trait for collecting deserialization.
  - `deserialize_at` reads a value at a location and pushes every issue in it into a `Report`. It returns the value, `None` when the value couldn't be built, or the format's own error.
  - `from_deserializer` reads a value from the root. It returns the value or a `DeserializeError`, which is either the format's error or every issue found. `issues()` returns the issues as a slice in both cases, empty for the format's error.
  - `from_deserializer_fail_fast` returns only the first issue.
- `#[derive(Deserialize)]`, behind the `serde` feature, for a struct with named fields.
  - It implements `annotated::Deserialize`, and serde's `Deserialize` through it.
  - The struct is read by its serde names, following `rename_all`, `rename` and `alias`, and it supports `deny_unknown_fields` and `expecting`.
  - Once the input is read, every field is checked in declaration order. A field absent from the input is `Problem::Missing`, unless it is an `Option`, which is `None`. A field that is present is checked by its rules as `Validate` checks it.
  - Struct-level rules run last, and only when every field was present.
  - Through serde's `Deserialize`, a struct that breaks its rules is an error with every issue in its message.
  - Other `#[serde(...)]` attributes and lifetime parameters are compile errors for now.
- `annotated::Deserialize` for the containers that hold a nested value:
  - `Option<T>` and `Box<T>`.
  - `Box<[T]>`, `Vec<T>` and `VecDeque<T>`, which read each element at its index.
  - `BTreeMap<K, V>` and `HashMap<K, V, S>`, which read each value at its key.
  - A container is built only when everything in it could be built.
  - A map reports its values' issues in the order it iterates over them. When a key appears twice, the later value replaces the earlier one, along with its issues.
  - A sequence or a `HashMap` reserves room for the number of values the format announces, up to a megabyte, as serde does.
  - For implementing `annotated::Deserialize` by hand, `Report::new_alike` starts a report for one part of a value, gathering as its parent does (every issue, or only the first), and `append` moves its issues into the parent.
- `#[derive(Deserialize)]` reads nested values through `annotated::Deserialize`: a field checked with `nested`, or with `each(...)` holding `nested` at any depth.
  - A field missing inside a nested value is reported at its full path, beside the broken rules, in the order `Validate` reports them.
  - When a nested value couldn't be built, its issues are reported. The other rules of its field are skipped, and so are the struct-level rules of the struct around it.
  - Failing fast still returns the first issue of the full report. A struct whose fields were all present is built even once its report is finished, so that the struct around it can check it.
  - A field's rules can name `nested` only once.
- `#[derive(Deserialize)]` accepts and ignores the `#[serde(...)]` attributes that only change serializing, so a struct can derive serde's `Serialize` beside it. On the struct, these are `into` and `bound(serialize = "...")`. On a field, they are `skip_serializing`, `skip_serializing_if`, `serialize_with`, `getter` and `bound(serialize = "...")`. Attributes that change deserializing are still compile errors.
- `#[annotated(collect_type_errors)]` on a struct that derives `Deserialize`. A value of the wrong type becomes `Problem::InvalidType` at its field, beside the other issues in declaration order, instead of the format's error.
  - Each field is read in two steps: first into a buffer that holds any well-formed value, borrowing the strings and bytes the format lends, then from the buffer as the field's type.
  - `expected` is what serde says the type expects. When the type rejects the value with a message of its own, it is the type's name instead, and for an `Option`, the name of the type inside. For a value inside the field, such as an element of a list, both parameters describe that value.
  - A value of the wrong type is checked by no other rule, and the struct-level rules are skipped.
  - Inside a nested value, a value of the wrong type replaces the issues found in that nested value before it. A nested struct that collects type errors too reports the ones inside it at their full paths.
  - Failing fast still returns the first issue of the full report.
  - JSON writes every map key as a string. So a key written as a string is read as the integer or boolean its type asks for, as `serde_json` reads it.
  - It needs a self-describing format.
  - Any other option in `#[annotated(...)]`, and any option on a field, is a compile error.
- The `json` feature, which adds `serde_json` and turns on `serde`. `annotated::json::from_str` and `from_slice` read a JSON document and collect every issue in it. Like `serde_json`, they reject characters after the value, and that error comes before any issue. `from_str_fail_fast` and `from_slice_fail_fast` return only the first issue.
- `examples/axum`, an axum extractor to copy into your app. It reads a JSON body with `annotated::json::from_slice`, and rejects a request it can't read the way axum's `Json` does. It responds to issues with 422 and RFC 9457 problem details, which list each issue's JSON Pointer, code, params and message.

[Unreleased]: https://github.com/pablocom/annotated/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/pablocom/annotated/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/pablocom/annotated/releases/tag/v0.1.0
