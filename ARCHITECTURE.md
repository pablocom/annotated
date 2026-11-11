# Architecture

How `annotated` is built and why. The [README](README.md) says what it promises; this file says how the code keeps those promises. Read it before changing the crates.

## The two ways in

1. **Validation** checks a value that already exists. `#[derive(Validate)]` implements `Validate`, and `value.validate()` answers `Ok(())` or every violation found. It works on any struct, whether or not serde built it.
2. **Collecting deserialization** builds a value and validates it in one go. `#[derive(Deserialize)]` deserializes through a hidden twin of the struct whose required fields are `Option`s, so a missing field becomes a violation instead of stopping serde. It then runs the same rules and reports every problem together. It also implements `serde::Deserialize`, so the struct still works anywhere serde does.

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
- `Problem` is `#[non_exhaustive]`, so adding a built-in problem is not a breaking change. Every variant has a stable snake_case `code()` and named `params()`, for code that translates messages or reacts to them. A param is a `ParamValue`: an integer (`i128`, so every primitive integer fits exactly), a float or a text, so a translator picks a plural form or compares a bound without parsing, and a message reads it as it renders. `Problem::custom(code, message)` is how user rules report, with `.with_param(name, value)` taking a number or a text.
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
- `each(...)` applies its rules to every element of a collection, with the element's index, or every value of a map, with its key. `each(nested)` validates every element as a struct. It goes through anything implementing the `HasElements` capability trait: `Vec`, `VecDeque`, arrays, slices, `HashMap` and `BTreeMap`, and any of them behind a reference or a `Box`, like `&'a [T]` or `Box<[T]>`. Elements that are `Option`s get the same treatment as `Option` fields, known by the collection's type as written, seen through `&` and `Box`.
- A struct-level `#[validate(custom(f))]` takes `Fn(&Self) -> Result<(), Problem>` and reports at the struct's own location; `custom(f, field = repeated_password)` reports at that field instead, by its serde name. It takes `message` like any rule. Struct-level rules sit beside `report_all`, in the same `#[validate(...)]` or another, and any other struct-level rule is refused at compile time.

## Semantics, settled

- **Every field, first failure per field by default.** Every field is checked. Within one field, rules run in the order written and the first failure stops that field's rules, so a blank value isn't also reported as too short, and cheap rules can guard expensive ones. A struct-level `#[validate(report_all)]` reports every failing rule instead. Elements under `each` are each checked independently. Struct-level rules run after every field, whatever the fields broke, and each of them is checked, in the order written.
- **Strings are never trimmed.** Rules check the value exactly as the handler will receive it. Only `required` treats whitespace-only as empty.
- **`length` counts `char`s on strings by default** (Unicode scalar values). `unit = "bytes"` counts UTF-8 bytes. `unit = "graphemes"`, behind the `unicode` feature, counts what a person sees as one character. On collections it counts elements, and `unit` is refused at compile time: a unit turns `length()` into a `StringLength` rule, which checks only strings.
- **Field names come from serde.** With no `#[serde(...)]` attributes, that is the Rust field name, without `r#`.

## Collecting deserialization

> Planned; the details settle when it's built.

- `#[derive(Deserialize)]` emits a hidden twin, `__AnnotatedUnvalidated{Name}`, deriving serde's `Deserialize`. It carries the struct's and fields' naming attributes (`rename_all`, `rename`, `alias`, `deny_unknown_fields`, `expecting`). Each required field becomes `Option<T>`, and each optional field keeps its `Option<T>` with `#[serde(default)]`. Other serde attributes are refused with an error naming the accepted ones, until each is supported deliberately.
- From the twin, it reports every missing field, runs every rule on the fields present, and builds the struct only when nothing failed.
- `annotated::from_deserializer` returns `Result<T, DeserializeError<D::Error>>`: either the format's own error (syntax, or a type error when not collecting them) or `Violations`. The `json` feature adds `annotated::json::from_str` and `from_slice`.
- The generated `serde::Deserialize` impl turns `Violations` into `D::Error::custom` with every violation in its message, so plain `serde_json::from_str::<T>` still validates.
- `#[annotated(collect_type_errors)]` makes each twin field buffer its value into a format-independent content tree first, then deserialize it into the field's type, so a value of the wrong type joins the violations as `Problem::InvalidType`. It costs an allocation per field and needs a self-describing format (JSON, YAML, TOML); without it, the first type error fails the whole deserialization, as serde does.

## Crates and features

| Crate | Depends on |
|---|---|
| `annotated` | `annotated-derive` (re-exported); optional `serde` (default), `regex` (`pattern`), `url` (`url`), `unicode-segmentation` (`unicode`), `serde_json` (`json`) |
| `annotated-derive` | `syn`, `quote`, `proc-macro2`, `serde_derive_internals`; optional `regex-syntax` to check patterns at compile time |
| `annotated-axum` | `annotated` with `json`, `axum` |

## Tests

- **Unit tests** sit beside each rule and each piece of the error model.
- **Integration tests** in `crates/annotated/tests/` drive the derives from the outside: what a struct accepts and the violations it reports, with whole values compared.
- **Compile-fail tests** in `crates/annotated/tests/ui/`, run by `trybuild` on the stable toolchain only, pin every error the derives emit, including its span. Compiler output differs between versions, so the MSRV job skips them.
- **Doc tests** are the examples in the docs, so every example compiles and runs.
- **Benchmarks** in `benches/` compare validation and collecting deserialization against `validator` and `garde`.
