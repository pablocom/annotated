# Architecture

How `annotated` is built and why. The [README](README.md) says what it promises; this file says how the code keeps those promises. Read it before changing the crates.

## The two ways in

1. **Validation** checks a value that already exists. `#[derive(Validate)]` implements `Validate`, and `value.validate()` answers `Ok(())` or every violation found. It works on any struct, whether or not serde built it.
2. **Collecting deserialization** builds a value and validates it in one go. `#[derive(Deserialize)]` generates the struct's own serde visitor, which passes the location down into nested values, so a missing field at any depth becomes a violation instead of stopping serde. It runs the same rules and reports every problem together. It also implements `serde::Deserialize`, so the struct still works anywhere serde does.

Both report fields by the name serde reads them by: the derives read `#[serde(...)]` through `serde_derive_internals`, the crate serde's own derive is built on.

## Violations, problems and paths

```text
Violations ──1..*── Violation ──── Path ──0..*── PathSegment (Field | Index | Key)
                        │
                        ├──── Problem (#[non_exhaustive] enum, with code() and params())
                        └──── message override (Option<Cow<'static, str>>)
```

- `Violations` is the error type. It is never empty, implements `std::error::Error` and `Display` (one violation per line), and iterates over `Violation`s in the order they were found: fields in declaration order, then struct-level rules.
- A `Violation` has a `Path`, a `Problem` and an optional message override. `message()` answers the override, with `{param}` placeholders filled from the problem's parameters, or the problem's English message.
- `Problem` is `#[non_exhaustive]`, so adding a built-in problem is not a breaking change. Every variant has a stable snake_case `code()`, the name of the built-in rule that reports it (`ipv4` for `ipv4`), and named `params()`, for code that translates messages or reacts to them. A param is a `ParamValue`: an integer (`i128`, so every primitive integer fits exactly), a float or a text, so a translator picks a plural form or compares a bound without parsing, and a message reads it as it renders. `Problem::custom(code, message)` is how user rules report, with `.with_param(name, value)` taking a number or a text.
- A `Path` is a list of segments: a field (`&'static str`, serde's name), an element's index, or a map's key (rendered with `Display`). It renders as a dotted path (`members[1].name`, with a key in brackets like an index: `labels[en]`) through `Display`, and as an RFC 6901 JSON Pointer through `json_pointer()` (`/members/1/name`). The root path is empty: a struct-level violation has it.

## Nothing to pay on the success path

- `Validate::validate_into(&self, at: &Location<'_>, found: &mut FoundViolations)` does the work; `validate()` is a provided method that starts at the root. A `Location` is a linked list on the stack (`Root`, or a parent plus one segment), so descending into a field or an element costs nothing. It is turned into an owned `Path` only when a violation is pushed.
- `FoundViolations` allocates on its first violation, not before.
- A rule is a small value built inline at the call site (`rules::length().max(50)`); its constructor and builder methods are `const fn` where they can be, so the compiler folds them away. Checking calls `Rule::check(&rule, value) -> Result<(), Problem>`, and a `Problem` is built only when the check fails.
- The one rule with expensive setup, `pattern`, gets a `static` `LazyLock<Regex>` generated at its call site, so each regular expression compiles once. The derive also checks the expression at compile time and refuses one that doesn't parse.

Claims about speed are backed by the benchmarks in `benches/`, which compare against `validator` and `garde`.

## Rules

A rule is a type implementing `annotated::Rule<T>` for every `T` it can check:

```rust
pub trait Rule<T: ?Sized> {
    fn check(&self, value: &T) -> Result<(), Problem>;
}
```

Built-in rules are generic over small capability traits, not over concrete types: `length` checks anything with a length (`str`, `String`, `Cow<str>`, `Vec`, slices, sets, maps), `email` and `url` anything that is `AsRef<str>`, `range` anything `PartialOrd` and `Display`. So they work on `Box<str>`, `Arc<str>` or a user's own string type without changes. `range` reports its bounds as numbers when the type checked is a primitive number type, told apart at run time once a value is out of range, and as text rendered with `Display` for any other type, so it needs no capability trait of its own.

### How `#[validate(...)]` turns into code

Each entry in `#[validate(...)]` is one rule, and the derive turns it into a constructor call, then checks the field with it:

| Written | Becomes |
|---|---|
| `email` | `::annotated::rules::email()` |
| `length(min = 1, max = 50)` | `::annotated::rules::length().min(1).max(50)` |
| `pattern = r"^\d{5}$"` or `pattern(r"^\d{5}$")` | `::annotated::rules::pattern(&GENERATED_STATIC_REGEX)` |
| `contains = "@"` or `contains("@")` | `::annotated::rules::contains("@")` |
| `custom(not_reserved)` | `::annotated::rules::custom(not_reserved)` |
| `my_rules::nif` | `my_rules::nif()` |
| `my_rules::iban(country = "ES")` | `my_rules::iban().country("ES")` |

- A single name is a built-in, looked up in `annotated::rules`. A path is the user's own rule, looked up where the path says, with the same syntax. So a team's rules module reads exactly like the built-ins.
- `name = value` and `name(value)` pass one value to the constructor. `name(key = value, …)` calls the builder method `key` for each pair. Mixing the two forms in one rule is refused.
- `message = "…"` is reserved in every rule's arguments: the derive keeps it and attaches it to the violation, and the rule never sees it.
- `custom(f)` takes any `Fn(&T) -> Result<(), Problem>`.

### Rules the derive handles itself

- `required` means present: on an `Option`, it means `Some`; on a string, not empty and not only whitespace; on a collection, not empty. It's an ordinary rule over a `Required` capability trait. What's special is only that, on an `Option` field, it checks the `Option` itself.
- On an `Option` field, every rule other than `required` checks the value inside, and is skipped when there is none. The derive knows an `Option` field by its type as written: a path ending in `Option<T>`.
- `nested` validates a field whose type implements `Validate`, with the field's segment added to the location.
- `each(...)` applies its rules to every element of a collection, with the element's index, or every value of a map, with its key. `each(nested)` validates every element as a struct. It goes through anything implementing the `HasElements` capability trait: `Vec`, `VecDeque`, arrays, slices, `HashMap` and `BTreeMap`, and any of them behind a reference or a `Box`, like `&'a [T]` or `Box<[T]>`. Elements that are `Option`s get the same treatment as `Option` fields, known by the collection's type as written, seen through `&` and `Box`: a map's values are its second type argument, whatever follows them (`HashMap<K, V, S>`), and any other collection's elements its first. A map is a type whose name ends in `Map`.
- A struct-level `#[validate(custom(f))]` takes `Fn(&Self) -> Result<(), Problem>` and reports at the struct's own location; `custom(f, field = repeated_password)` reports at that field instead, by its serde name. It takes `message` like any rule. Struct-level rules sit beside `report_all`, in the same `#[validate(...)]` or another, and any other struct-level rule is refused at compile time.

## Semantics, settled

- **Every field, first failure per field by default.** Every field is checked. Within one field, rules run in the order written and the first failure stops that field's rules, so a blank value isn't also reported as too short, and cheap rules can guard expensive ones. A violation found under the field by `nested` or `each` counts as its failure too. A struct-level `#[validate(report_all)]` reports every failing rule instead. Elements under `each` are each checked independently.
- **Struct-level rules come last.** They run after every field, in the order written. One with `field = name` counts as that field's last rule: it is skipped when the field already failed, and the first one at a field that fails stops the later ones there, unless the struct has `report_all`. So a blank repeated password is reported as required, not also as different. One at the struct itself always runs. A field a struct-level rule names keeps whether it passed in a local `bool`, so this costs nothing on the success path.
- **Failing fast is chosen per call.** `value.validate_fail_fast()`, `annotated::from_deserializer_fail_fast` and `annotated::json::from_str_fail_fast` / `from_slice_fail_fast` stop at the first violation of any kind and answer exactly one: the first the full report would list, so the two always agree on it. The same struct is checked either way, and nested values follow the call. `FoundViolations::failing_fast()` starts such a report, and `is_finished()` answers whether it has its one violation; generated code asks it only right after recording a violation, and `HasElements` only after an element failed, so the success path pays nothing for it. Deserializing still reads the input to its end, since a missing field is known only then, but runs no rule once finished.
- **Strings are never trimmed.** Rules check the value exactly as the handler will receive it. Only `required` treats whitespace-only as empty.
- **`length` counts `char`s on strings by default** (Unicode scalar values). `unit = "bytes"` counts UTF-8 bytes. `unit = "graphemes"`, behind the `unicode` feature, counts what a person sees as one character. On collections it counts elements, and `unit` is refused at compile time: a unit turns `length()` into a `StringLength` rule, which checks only strings.
- **Field names come from serde.** With no `#[serde(...)]` attributes, that is the Rust field name, without `r#`.
- **A URL is RFC 3986's, with RFC 3987's characters.** `url` accepts an absolute URI with an authority by [RFC 3986](https://www.rfc-editor.org/rfc/rfc3986) (section 3 and appendix A), `scheme "://" authority path-abempty [ "?" query ] [ "#" fragment ]`, with `ucschar` in the host, path, query and fragment and `iprivate` in the query, as [RFC 3987](https://www.rfc-editor.org/rfc/rfc3987) section 2.2 extends it, so `https://bücher.de/straße` passes. It departs from the grammar twice: the host can't be empty, as [RFC 9110](https://www.rfc-editor.org/rfc/rfc9110) section 4.2.1 requires of `http`, and userinfo, which section 4.2.4 deprecates for `http` and `https`, is refused for every scheme, so the rule stays one definition. The scheme is compared with the accepted ones ignoring ASCII case. The host is an IP literal (`[IPv6address]` or `IPvFuture`), an IPv4 address or an `ireg-name`, the port is `*DIGIT`, and a `%` must be followed by two hexadecimal digits. Anything else, whitespace (surrounding included), a backslash or a control character, breaks it. The recognizer is written by hand, in small functions named after the grammar's productions, and allocates nothing.

## Collecting deserialization

> Being built. This is the design it's built to; the sections above describe what's in place.

### The `Deserialize` trait

```rust
pub trait Deserialize<'de>: Sized {
    fn deserialize_at<D: serde::Deserializer<'de>>(
        deserializer: D,
        at: &Location<'_>,
        found: &mut FoundViolations,
    ) -> Result<Option<Self>, D::Error>;
}
```

It mirrors `Validate::validate_into`: the value is read at the location `at`, and every violation in it is pushed into `found`. `Ok(Some(value))` means the value could be built, though rules in it may have broken (they are in `found`); `Ok(None)` means it couldn't, because a field inside is missing or of the wrong type; `Err` is the format's own error (syntax, an unknown field under `deny_unknown_fields`, a duplicate field, or a type error when they aren't collected), which ends deserialization as serde's would. It lives beside serde's trait of the same name the way serde's derive and trait share one: `#[derive(annotated::Deserialize)]` implements it.

The crate implements it for the containers a `nested` or `each(nested)` field can hold, whenever their elements implement it: `Option<T>`, `Box<T>`, `Vec<T>`, `VecDeque<T>`, `Box<[T]>`, `HashMap<K, V, S>` and `BTreeMap<K, V>`. A sequence's element is read at its index, a map's value at its key (`K: serde::Deserialize + Display`), through `DeserializeSeed`s carrying the location. They compose, so `Option<Vec<Address>>` works with no code of its own.

### What `#[derive(Deserialize)]` generates

- **The struct's own serde visitor**: a field identifier matching the serde names and aliases (from `serde_derive_internals`, as the `Validate` derive reads them), `visit_map` and `visit_seq`, and one slot per field. A field without `nested` is read with serde's `Deserialize`. A field with `nested` or `each(nested)`, at any depth, is read through `annotated::Deserialize` with `DeserializeSeed`, at `at.field(name)`, into a `FoundViolations` of its own, so its violations wait for their turn in declaration order. That buffer allocates only if something inside fails.
- **After the last field, a pass in declaration order.** A field that never came is `Problem::Missing` (code `missing`) at its name, unless it's an `Option`, which is `None` as with serde. A present field runs its rules exactly as `Validate` does, by the same expansion code, reading the slot instead of `self`; at its `nested` rule, the field's buffered violations join `found` and count as its failure. A nested field whose value couldn't be built reports its buffered violations, and its other rules, which need the value, are skipped. Then, if every field could be built, the struct is built and its struct-level rules run, as in `Validate`; if not, they're skipped and `deserialize_at` answers `None`.
- So, for a complete, well-typed input, collecting deserialization reports exactly what deserializing with serde and then calling `validate()` would, in the same order. Tests pin that equivalence.
- **`serde::Deserialize`**, through `annotated::from_deserializer`, turning `Violations` into `D::Error::custom` with every violation in its message, so plain `serde_json::from_str::<T>`, or a struct nesting this one through serde, still validates.
- Attributes: it reads `#[validate(...)]` like the `Validate` derive, `#[annotated(...)]` for its own options, and these `#[serde(...)]` ones: on the struct `rename_all`, `rename`, `deny_unknown_fields` and `expecting`; on a field `rename` and `alias`. Any other serde attribute is refused at compile time with an error naming the accepted ones, until each is supported deliberately. Type parameters are supported, each field type bounded by the trait it is read through; a struct with lifetime parameters is refused, since borrowing needs `#[serde(borrow)]`.
- Its generated code reaches serde through `annotated::__private::serde`, so users depend on `annotated` alone.

### Entry points and the error

- `annotated::from_deserializer(deserializer) -> Result<T, DeserializeError<D::Error>>` starts at the root with an empty `FoundViolations`.
- `DeserializeError<E>` is `#[non_exhaustive]`, either `Format(E)` or `Violations(Violations)`. `violations()` answers them as a slice, empty for the format's own error, so a caller reads them without matching. It implements `Display`, `Debug` and `std::error::Error`.
- The `json` feature adds `annotated::json::from_str` and `from_slice`, which also refuse trailing characters, as `serde_json::from_str` does.

### Values of the wrong type

Without an opt-in, the first value of the wrong type ends deserialization with the format's error, as serde does: a streaming parser that has refused a value can't skip past it. `#[annotated(collect_type_errors)]` on a struct reads each of its fields in two steps: first into `Content`, a format-independent tree of the value (bool, integers, floats, char, string, bytes, unit, option, sequence, map; borrowing strings and bytes when the format lends them), which accepts any well-formed value, then from that tree into the field's type through `ContentDeserializer`. A failure in the second step leaves the parser past the value, so it becomes `Problem::InvalidType` (code `invalid_type`) at the field and deserialization goes on. `Content` and `ContentDeserializer` are the crate's own, kept private; serde's equivalent is private to serde.

`Problem::InvalidType` has the params `expected`, serde's description of what the type accepts (`u8`, `a string`, `struct Address`), or the type's short name when the error doesn't say, and `found`, the kind of value that came: `boolean`, `integer`, `float`, `character`, `string`, `bytes`, `null`, `sequence` or `map`. It never carries the value itself, so nothing the client sent, a password say, is echoed back. Its message is `must be {expected}, found {found}`. The buffering costs allocations per field, and needs a self-describing format (JSON, YAML, TOML).

### Using it from a web framework

There is no framework crate. `examples/axum` shows the extractor, about thirty lines: it reads the body with `annotated::json::from_slice`, answers a body that isn't JSON as axum's own `Json` does, and answers violations with 422 and an [RFC 9457](https://www.rfc-editor.org/rfc/rfc9457) `application/problem+json` body whose `errors` member lists each violation's JSON Pointer, code, params and message.

## Crates and features

| Crate | Depends on |
|---|---|
| `annotated` | `annotated-derive` (re-exported); `serde` (default feature: the `Deserialize` trait and derive, `from_deserializer`), `regex` (`pattern`), `unicode-segmentation` (`unicode`), `serde_json` (`json`) |
| `annotated-derive` | `syn`, `quote`, `proc-macro2`, `serde_derive_internals`; optional `regex-syntax` to check patterns at compile time |

## Tests

- **Unit tests** sit beside each rule and each piece of the error model.
- **Integration tests** in `crates/annotated/tests/` drive the derives from the outside: what a struct accepts and the violations it reports, with whole values compared.
- **Compile-fail tests** in `crates/annotated/tests/ui/`, run by `trybuild` on the stable toolchain only, pin every error the derives emit, including its span. Compiler output differs between versions, so the MSRV job skips them.
- **Doc tests** are the examples in the docs, so every example compiles and runs.
- **Property tests** check that the `url` rule agrees, on generated text, with [`iri-string`](https://crates.io/crates/iri-string), a strict RFC 3986 and RFC 3987 validator.
- **Benchmarks** in `benches/` compare validation and collecting deserialization against `validator` and `garde`.
