# Architecture

The [README](README.md) shows how to use `annotated`. This document is for working on it: the principles the library keeps, the terms it uses, and how the code is built and why. Read it before changing the crates.

## Principles

- **Every problem at once.** A client fixing a request sees everything that is wrong with it in one response, instead of one field per round trip.
- **serde decides the names.** A field is reported under the name serde reads it by, whatever `rename_all`, `rename` or `alias` it has. That name is worked out by the same code serde's derive uses, so the crate never has a naming scheme of its own.
- **The format is the user's.** The library works with any serde format. It reports a path as a list of segments (field, index, key), and the user chooses how to render it: as a JSON Pointer, a dotted path or anything else.
- **Problems are data.** Each problem has a stable code and parameters, for code that translates it or reacts to it, and an English message for people. A rule can replace that message with its own `message`.
- **Nothing to pay on the success path.** Validating a valid value allocates nothing. A path is built only when something fails, rules are plain functions the compiler inlines, and each regular expression is compiled once.
- **Open for the user's rules.** Built-in rules are ordinary functions, and a user's own rules work the same way: on a field with `custom(...)`, or on the whole struct.
- **Rules are pure.** A rule looks at the value and nothing else. A check that needs I/O, like "is this email already taken", belongs in the application, after validation.

## Terms

Use these words, with these meanings, in code, docs and conversation.

- **Rule:** a check a value must pass, written in `#[validate(...)]`: `required`, `length(max = 50)`, `email`.
- **Issue:** one thing wrong with a value, and the path where it was found. It is a broken rule, a missing field or a value of the wrong type.
- **Problem:** the kind of issue: a stable code, its parameters and an English message.
- **Path:** where in the value an issue was found: a field, an element's index, a map's key, or the value itself.
- **Issues:** all the issues found in one validation. This is the error type, and it is never empty.
- **Report:** the issues found so far, while a value is being checked. It can be empty, and when it fails fast it keeps only its first issue. At the end it becomes `Issues`, or `Ok(())` when it is empty.
- **Validation:** checking a value that already exists against its rules, with `Validate::validate`.
- **Collecting deserialization:** deserializing through `annotated::Deserialize`. It reports missing fields and broken rules together, and values of the wrong type too when the struct opts in.

## The two ways in

1. **Validation** checks a value that already exists. `#[derive(Validate)]` implements `Validate`, and `value.validate()` returns `Ok(())` or every issue found. It works on any struct, whether or not serde built it.
2. **Collecting deserialization** builds a value and validates it in one go. `#[derive(Deserialize)]` generates a serde visitor for the struct. The visitor passes its location down into nested values, so a missing field at any depth becomes an issue instead of stopping serde. It runs the same rules as validation and reports every problem together. The derive also implements `serde::Deserialize`, so the struct still works anywhere serde does.

Both report a field under the name serde reads it by. To find that name, the derives read `#[serde(...)]` attributes with `serde_derive_internals`, which publishes the code serde's own derive reads them with.

## Issues, problems and paths

```text
Issues ──1..*── Issue ──── Path ──0..*── PathSegment (Field | Index | Key)
                  │
                  ├──── Problem (#[non_exhaustive] enum, with code() and params())
                  └──── message override (Option<Cow<'static, str>>)
```

**`Issues`** is the error type.

- It is never empty.
- It implements `std::error::Error` and `Display`, which prints one issue per line.
- Iterating over it yields each `Issue` in the order it was found: fields in declaration order, then struct-level rules.

A **`Issue`** has a `Path`, a `Problem` and an optional message override. `message()` returns the override when there is one, with its `{param}` placeholders filled from the problem's parameters. Otherwise it returns the problem's English message.

**`Problem`** is `#[non_exhaustive]`, so adding a built-in problem is not a breaking change.

- Every variant has a stable snake_case `code()`. For a problem reported by a built-in rule, the code is the rule's name: the `ipv4` rule reports the code `ipv4`.
- Every variant has named `params()`, for code that translates messages or reacts to a problem.
- A parameter is a `ParamValue`: an integer, a float or text. Because parameters are typed, a translator can pick a plural form or compare a bound without parsing text. An integer is stored as an `i128`, so every primitive integer fits exactly, except a `u128` too large for an `i128`, which is kept as text.
- In a message, a `{param}` placeholder is replaced by the parameter as `Display` prints it.
- Your own rules report with `Problem::custom(code, message)`. `.with_param(name, value)` adds a parameter, which is a number or text.

A **`Path`** is a list of segments.

- A segment is a field (`&'static str`, the serde name), an element's index, or a map's key (rendered with `Display`).
- `Display` prints a path in dotted form: `members[1].name`. A map's key goes in brackets, like an index: `labels[en]`.
- `json_pointer()` returns it as an RFC 6901 JSON Pointer: `/members/1/name`.
- The root path has no segments. An issue a struct-level rule finds on the validated struct itself has the root path.

## Nothing to pay on the success path

- `Validate::validate_into(&self, at: &Location<'_>, found: &mut Report)` does the work. `validate()` is a provided method that calls it at the root.
- A `Location` is a linked list on the stack: it is `Root`, or a parent location plus one segment. So descending into a field or an element allocates nothing. A `Location` is turned into an owned `Path` only when an issue is recorded.
- `Report` allocates when it records its first issue, not before.
- A rule is a small value built inline where it is used: `rules::length().max(50)`. Its constructor and builder methods are `const fn` wherever they can be, so the compiler folds them away. A check is a call to `Rule::check(&rule, value) -> Result<(), Problem>`, and a `Problem` is built only when the check fails.
- `pattern` is the one rule with expensive setup. The derive generates a `static` `LazyLock<Regex>` where the rule is written, so each regular expression is compiled once. The derive also parses the expression at compile time, and one that doesn't parse is a compile error.

Claims about speed are backed by the [benchmarks](benchmarks/README.md), a workspace of their own beside the crates. They compare validation and collecting deserialization against `validator`, `garde`, `serde_valid` and `wary`.

## Rules

A rule is a type that implements `annotated::Rule<T>` for every `T` it can check:

```rust
pub trait Rule<T: ?Sized> {
    fn check(&self, value: &T) -> Result<(), Problem>;
}
```

Built-in rules are generic over small capability traits, not over concrete types:

- `length` checks anything that has a length: `str`, `String`, `Cow<str>`, `Vec`, slices, sets, maps.
- `email` and `url` check anything that is `AsRef<str>`.
- `range` checks anything that is `PartialOrd` and `Display`.

So the rules work unchanged on `Box<str>`, `Arc<str>` or your own string type.

`range` needs no capability trait of its own. When a value is out of range, it reports its bounds as numbers if the checked type is a primitive number type, and as text rendered with `Display` for any other type. The two cases are told apart at run time, and only once a value is out of range.

### How `#[validate(...)]` turns into code

Each entry in `#[validate(...)]` is one rule. The derive turns the entry into a call to the rule's constructor, then checks the field with the rule it returns:

| Written | Becomes |
|---|---|
| `email` | `::annotated::rules::email()` |
| `length(min = 1, max = 50)` | `::annotated::rules::length().min(1).max(50)` |
| `pattern = r"^\d{5}$"` or `pattern(r"^\d{5}$")` | `::annotated::rules::pattern(&GENERATED_STATIC_REGEX)` |
| `contains = "@"` or `contains("@")` | `::annotated::rules::contains("@")` |
| `custom(not_reserved)` | `::annotated::rules::custom(not_reserved)` |
| `my_rules::nif` | `my_rules::nif()` |
| `my_rules::iban(country = "ES")` | `my_rules::iban().country("ES")` |

- A single name is a built-in rule, looked up in `annotated::rules`. A path is your own rule, looked up at that path and written with the same syntax. So a team's rules module reads exactly like the built-ins.
- `name = value` and `name(value)` pass one value to the constructor. `name(key = value, …)` calls the builder method `key` for each pair. Mixing the two forms in one rule is a compile error.
- `message = "…"` is reserved in every rule's arguments. The derive takes it out and attaches it to the issue, so the rule never sees it.
- `custom(f)` takes any `Fn(&T) -> Result<(), Problem>`.

### Rules the derive handles itself

- **`required`** means the value is present: an `Option` is `Some`, a string is not empty and not only whitespace, and a collection is not empty. It is an ordinary rule over the `Required` capability trait. The only special handling is that, on an `Option` field, it checks the `Option` itself and not the value inside.
- **Other rules on an `Option` field** check the value inside, and are skipped when the field is `None`. The derive recognizes an `Option` field by how its type is written: a path ending in `Option<T>`.
- **`nested`** validates a field whose type implements `Validate`. The field's segment is added to the location, so issues inside it are reported at their full path.
- **`each(...)`** applies its rules to every element of a collection, reported at the element's index, or to every value of a map, reported at its key. `each(nested)` validates every element as a struct.
  - It works on anything that implements the `HasElements` capability trait: `Vec`, `VecDeque`, arrays, slices, `HashMap` and `BTreeMap`, and any of these behind a reference or a `Box`, like `&'a [T]` or `Box<[T]>`.
  - Elements that are `Option`s are treated like `Option` fields. To find the element type, the derive again goes by how the collection's type is written, looking through `&` and `Box`. A map's values are its second type argument, whatever comes after it (`HashMap<K, V, S>`). Any other collection's elements are its first type argument. A type counts as a map when its name ends in `Map`.
- **Struct-level `#[validate(custom(f))]`** takes an `Fn(&Self) -> Result<(), Problem>` and reports at the struct's own location. `custom(f, field = repeated_password)` reports at that field instead, under its serde name. It accepts `message` like any rule. On the struct, `#[validate(...)]` accepts only `custom(...)` rules and `report_all`, in one attribute or in several. Any other rule there is a compile error.

## Semantics, settled

### Every field is checked, and each stops at its first failure

Every field is checked. Within one field, rules run in the order written, and the first one that fails stops the rest of that field's rules. So a blank value isn't also reported as too short, and a cheap rule can guard an expensive one.

- An issue found inside the field by `nested` or `each` counts as a failure of the field too.
- With `#[validate(report_all)]` on the struct, every failing rule is reported instead.
- Under `each`, every element is checked, whatever happened to the others.

### Struct-level rules come last

They run after every field, in the order written.

- A rule with `field = name` counts as that field's last rule. It is skipped when the field has already failed, and once one of them fails for a field, the later ones for the same field are skipped. So a blank repeated password is reported as required, and not also as different from the password. With `report_all` on the struct, none of them is skipped.
- A rule without `field` runs whether or not any field failed.
- The generated code keeps whether a field named by a struct-level rule passed in a local `bool`, so this costs nothing on the success path.

### Failing fast is chosen per call

`value.validate_fail_fast()`, `annotated::from_deserializer_fail_fast`, and `annotated::json::from_str_fail_fast` and `from_slice_fail_fast` stop at the first issue of any kind and return exactly that one.

- It is always the issue the full report would list first, so the two modes agree on it.
- The same struct is checked either way, and nested values follow the mode of the call.
- `Report::failing_fast()` starts a fail-fast report, and `is_finished()` returns whether it already has its one issue. Generated code calls `is_finished()` only right after recording an issue, so the success path pays nothing for it. When the report is finished, the generated code stops. If it was checking an element, it also tells `HasElements` to skip the remaining elements.
- During validation, when a nested value finishes the report, its parent doesn't stop at once, because checking for that after every nested value would slow down the valid path. The parent records nothing more, and stops at its own next issue or at its end. Under `each(nested)`, the remaining elements are still checked, and the parent stops when that `each` ends.
- During deserialization, the parent does stop at once. It appends the nested value's buffered issues itself, and calls `is_finished()` right after.
- Deserializing still reads the input to its end, because a field is known to be missing only then. Nested values are checked while they are read, into a buffer that fails fast too, so their rules run whatever was found before them. The checking pass that follows is what stops at the first issue.

### Strings are never trimmed

Rules check the value exactly as the code that receives it will see it. Only `required` gives whitespace a meaning: it treats a string of only whitespace as empty.

### `length` counts `char`s on strings by default

A `char` is a Unicode scalar value.

- `unit = "bytes"` counts UTF-8 bytes.
- `unit = "graphemes"`, behind the `unicode` feature, counts what a person sees as one character.
- On a collection, `length` counts elements, and writing `unit` is a compile error. A unit turns `length()` into a `StringLength` rule, which checks only strings.

### Field names come from serde

With no `#[serde(...)]` attributes, a field's name is its name in the struct, without `r#`.

### A URL is RFC 3986's, with RFC 3987's characters

`url` accepts an absolute URI with an authority, as [RFC 3986](https://www.rfc-editor.org/rfc/rfc3986) defines it in section 3 and appendix A:

```text
scheme "://" authority path-abempty [ "?" query ] [ "#" fragment ]
```

- **International characters.** [RFC 3987](https://www.rfc-editor.org/rfc/rfc3987) section 2.2 extends that grammar with `ucschar` in the host, path, query and fragment, and `iprivate` in the query. So `https://bücher.de/straße` passes.
- **Two departures from the grammar.** The host can't be empty, as [RFC 9110](https://www.rfc-editor.org/rfc/rfc9110) section 4.2.1 requires of `http`. And userinfo, which section 4.2.4 deprecates for `http` and `https`, is rejected for every scheme, so the rule keeps a single definition.
- **Scheme.** It is compared with the accepted schemes ignoring ASCII case.
- **Host and port.** The host is an IP literal (`[IPv6address]` or `IPvFuture`), an IPv4 address or an `ireg-name`. The port is `*DIGIT`.
- **Percent-encoding.** A `%` must be followed by two hexadecimal digits.
- **Everything else breaks the rule:** whitespace, including before or after the URL, a backslash or a control character.

The recognizer is written by hand, as small functions named after the grammar's productions, and allocates nothing.

## Collecting deserialization

### The `Deserialize` trait

```rust
pub trait Deserialize<'de>: Sized {
    fn deserialize_at<D: serde::Deserializer<'de>>(
        deserializer: D,
        at: &Location<'_>,
        found: &mut Report,
    ) -> Result<Option<Self>, D::Error>;
}
```

It mirrors `Validate::validate_into`: the value is read at the location `at`, and every issue in it is pushed into `found`. It returns one of three things:

| Returned | Meaning |
|---|---|
| `Ok(Some(value))` | The value could be built. Rules in it may still have been broken, and those issues are in `found`. |
| `Ok(None)` | The value couldn't be built, because a field inside it is missing or has the wrong type. `found` says why. |
| `Err(error)` | The format's own error: a syntax error, an unknown field under `deny_unknown_fields`, a duplicate field, or a type error when type errors aren't collected. It ends deserialization, as it would with serde. |

A value whose fields were all present is returned as `Some` even when `found` is already finished from failing fast. The parent needs the value: it may check it with other rules before its `nested` rule, and an issue from one of those comes first in the full report. Returning the value is what lets failing fast return that same first issue.

The trait shares its name with serde's trait, the way serde's own derive and trait share one. `#[derive(annotated::Deserialize)]` implements it.

The crate implements the trait for the containers a `nested` or `each(nested)` field can hold, whenever their elements implement it: `Option<T>`, `Box<T>`, `Vec<T>`, `VecDeque<T>`, `Box<[T]>`, `HashMap<K, V, S>` and `BTreeMap<K, V>`.

- An element of a sequence is read at its index, and a value of a map at its key (`K: serde::Deserialize + Display`). Both are read through a `DeserializeSeed` that carries the location.
- The implementations compose, so `Option<Vec<Address>>` works with no code of its own.
- A container is built only when everything in it could be built.
- A map keeps each value's issues apart until it has read every entry. It then reports them in the order it iterates over its values, which is the order `Validate` checks them in.
- When a key appears twice, the later value replaces the earlier one, and its issues replace the earlier ones too. serde keeps the last value in the same way.
- A `HashMap`'s iteration order is known only once the map is built. To report in that order, it clones the key of a value that was built but has issues, which is why it needs `K: Clone`.

### What `#[derive(Deserialize)]` generates

**A serde visitor for the struct.** It has a field identifier that matches the serde names and aliases (read with `serde_derive_internals`, as the `Validate` derive reads them), `visit_map` and `visit_seq`, and one slot per field.

- A field without `nested` is read with serde's `Deserialize`.
- A field with `nested`, or with `each(...)` holding `nested` at any depth, is read through `annotated::Deserialize`, with a `DeserializeSeed` at `at.field(name)`. Its issues go into a `Report` of the field's own, where they wait until the field's turn in declaration order. The buffer allocates only if something inside the field fails.

**A checking pass after the last field is read.** The pass goes through the fields in declaration order. It runs inside the visitor, where the slots are, so the struct is built straight from them.

- A field that was absent from the input is `Problem::Missing` (code `missing`) at its name. An absent `Option` field is `None` instead, as with serde.
- A field that was present runs its rules exactly as `Validate` does. The same expansion code generates them, reading the slot instead of `self`.
- At a field's `nested` rule, the field's buffered issues are appended to `found`, and they count as a failure of the field.
- When a nested field's value couldn't be built, its buffered issues are reported and its other rules are skipped, since they need the value.
- At the end, if every field could be built, the struct is built and its struct-level rules run, as in `Validate`. Otherwise they are skipped, and `deserialize_at` returns `None`.

**Elements under `each(...)`.** The issues of a collection's elements wait in the field's buffer, in the order `HasElements` goes through the elements. The crate's containers report them in that same order.

- When the `each` holds nothing but `nested`, the whole buffer is appended to `found` at the `each` rule, as `nested` on the field would do. The buffer holds exactly the elements' issues, already in order.
- Otherwise the pass goes through the elements as `Validate` does, running every rule inside `each(...)` on each one. At an element's `nested` rule, it moves the issues at the front of the buffer that belong to that element's location into `found`. When an element's `nested` rule didn't run, because an earlier rule on that element failed, the element's buffered issues are dropped.
- If a collection reported issues in any other order, they are appended after its elements, so none is lost.
- A field's rules can name `nested` only once, because the field's value is read only once.

**Failing fast.** A nested field's buffer fails fast too, so it holds only the first issue of that value's full report. The pass checks `found.is_finished()` after recording an issue or appending a buffer, and stops as soon as it is true. If all the fields were present, the struct is still built, but its struct-level rules don't run.

**The same report as serde followed by `validate()`.** For input that has every field with the right type, collecting deserialization reports exactly what deserializing with serde and then calling `validate()` would report, in the same order. Tests pin that equivalence.

**An implementation of `serde::Deserialize`.** It calls `annotated::from_deserializer` and turns `Issues` into `D::Error::custom`, with every issue in the message. So plain `serde_json::from_str::<T>` still validates, and so does a struct that holds this one as an ordinary serde field.

**Attributes.** The derive reads:

- `#[validate(...)]`, like the `Validate` derive.
- `#[annotated(...)]`, for its own options.
- The `#[serde(...)]` attributes in [Serde attributes](#serde-attributes).
- Serde attributes that only change serializing, which it accepts and ignores, since deserializing never uses them: `skip_serializing`, `skip_serializing_if`, `serialize_with`, `getter`, `into`, `bound(serialize = ...)`, and the serializing half of `rename` and `rename_all`.

Any other serde attribute is a compile error that names the accepted ones, until each is supported deliberately.

**Generics.** Type parameters are supported. As in serde's derive, the bounds come from the fields: each field type that mentions a type parameter is bounded by the trait the field is read through. A field type that can't be read is then an error at the field, not a bound that no caller can meet. Lifetime parameters are supported for borrowing, as [Borrowing](#borrowing) says.

- A field whose default is `Default::default()` is also bounded by `Default`.
- A nested value checked with `Validate`, as [Serde attributes](#serde-attributes) says, is bounded by `Validate`. Its type is found the way `each` finds an element's type, by how the field's type is written.

### Serde attributes

The derive reads a field the way serde's derive reads it with the same attributes. It reads these:

| On | Attributes |
|---|---|
| The struct | `rename_all`, `rename`, `deny_unknown_fields`, `expecting`, `default` |
| A field | `rename`, `alias`, `default`, `skip_deserializing`, `skip`, `deserialize_with`, `with`, `borrow`, `flatten` |

**Rules check every value the struct ends up with.** A value from `default`, a skipped field's value, and a value read by `deserialize_with` are checked by the field's rules like any other. So the report stays the one serde followed by `validate()` gives.

- annotated doesn't read such a value itself, so it can't check a `nested` rule while reading it. It checks the value with `Validate::validate_into` at the field's location instead, after the last key. The field's type must implement `Validate`, and when it doesn't, the compile error points at the field's type.

#### `default`

- On a field, `#[serde(default)]` gives an absent field `Default::default()`, and `#[serde(default = "path")]` gives it `path()`. The field is never `Problem::Missing`.
- On the struct, `#[serde(default)]` gives each absent field its value in `Self::default()`, and `#[serde(default = "path")]` its value in `path()`. A field's own `default` comes first.
- annotated makes a default only when a field is absent, so a complete input pays nothing for it.
- Only an absent field takes its default. An `Option` field given as `null` is `None`, as with serde.
- A struct read from a sequence gives a missing element at the end its default the same way.

#### `skip_deserializing` and `skip`

- The field is never read. Its value is its own `default`, else the struct's `default`, else `Default::default()`.
- Its name isn't one of the struct's field names. A key with that name is an unknown key: ignored, or an error under `deny_unknown_fields`, as with serde.

#### `deserialize_with` and `with`

- The field is read by `path(deserializer)`, or by `module::deserialize(deserializer)` for `with = "module"`. Its rules check the value the function returns.
- Without `collect_type_errors`, the function's error ends deserialization, as with serde.
- As with serde, the field is `Problem::Missing` when it is absent, even an `Option`, unless it has a default: only the function knows what an absent value would be.
- With `collect_type_errors`, the field is read into `Content` first, and the function reads it from there:
  - When the function fails because a value has the wrong type, is out of range or has the wrong length, through serde's `invalid_type`, `invalid_value` or `invalid_length`, the failure is a `Problem::InvalidType` at the field, as for any other field. None of these quote the input.
  - Any other error, such as one the function makes with `Error::custom`, ends deserialization as the format's error. Its message may repeat the input, so it never becomes an issue.
- As in serde's derive, the function reads its field through a type of its own, whose `Deserialize` calls the function, since the function is generic over the deserializer. A field read by a function is bounded by no trait: the function says what it needs, in bounds on the struct.

#### Borrowing

- A struct can have lifetime parameters. As with serde, a `&str` or `&[u8]` field borrows from the input, and any other field borrows with `#[serde(borrow)]`, or with `#[serde(borrow = "'a + 'b")]` for some of its lifetimes.
- The impl requires `'de` to outlive each borrowed lifetime, worked out the way serde's derive works it out. When one of them is `'static`, serde's derive implements `Deserialize<'static>` instead. annotated writes `'de: 'static`, which allows the same inputs, so the generated code keeps one shape.
- For a `Cow<str>` or `Cow<[u8]>` field with `borrow`, `serde_derive_internals` names a reading function in serde's private module. The derive calls annotated's own copy instead, in `annotated::__private`.
- When the input can't lend a value, for example a JSON string with an escape in it, a borrowed field fails as with serde: the format's error, or `Problem::InvalidType` under `collect_type_errors`. `Content` keeps lent strings and bytes borrowed, so borrowing works the same with the option.

#### `flatten`

- A flattened field's keys sit in the struct's own map, as with serde. The struct keeps every key that isn't one of its fields in a buffer, and reads each flattened field from the buffer after the last key. A flattened struct takes the keys it knows, and a flattened map takes every key no other field took.
- A flattened field has the struct's own location. Issues inside a flattened `Address` are at `street`, not at `address.street`, because that is where they are in the input. The `Validate` derive agrees: `nested` on a flattened field validates it at the struct's location.
  - A rule on the flattened value as a whole, such as `length` on a flattened map, reports at the struct's location. A rule under `each` reports at each key.
  - A flattened field's issues come at its place among the fields.
- A flattened `Option<T>` is `None` when `T` can't be built from the buffer, as with serde, and the issues found trying are dropped.
- With `collect_type_errors` on the struct, a flattened field without `nested` whose buffered keys don't fit its type is a `Problem::InvalidType` at the struct's location. A `nested` flattened field follows its own type's option.
- As in serde, a struct with a flattened field is read with `deserialize_map`, so it can't be read from a sequence.
- `flatten` together with `deny_unknown_fields` on the same struct is a compile error: a key meant for a flattened field can't be told from an unknown one while the struct reads its own keys.
- Buffering allocates: a `Content` for each key that isn't one of the struct's fields. Only a struct with a flattened field buffers. Every other struct reads as before.

**No serde dependency for the user.** The generated code reaches serde through `annotated::__private::serde`, so a crate using the derive depends on `annotated` alone.

### Entry points and the error

- `annotated::from_deserializer(deserializer) -> Result<T, DeserializeError<D::Error>>` starts at the root with an empty `Report`.
- `DeserializeError<E>` is a `#[non_exhaustive]` enum: `Format(E)` or `Issues(Issues)`. `issues()` returns the issues as a slice, which is empty for the format's own error, so a caller can read them without matching. The type implements `Display`, `Debug` and `std::error::Error`.
- The `json` feature adds `annotated::json::from_str` and `from_slice`. Like `serde_json::from_str`, they also reject characters after the JSON value.

### Values of the wrong type

By default, the first value of the wrong type ends deserialization with the format's error, as it does with serde. A streaming parser that has rejected a value part-way through can't skip to the end of it and carry on.

`#[annotated(collect_type_errors)]` on a struct reads each of its fields in two steps:

1. **Into a buffer.** The field is read into `Content`, a format-independent tree of the value: bool, integers, floats, char, string, bytes, unit, option, sequence or map. Strings and bytes are borrowed when the format lends them. `Content` accepts any well-formed value, with one exception: an enum written in a syntax the format has for enums. The shape of such a value can't be known before its type is, so it stays the format's error.
2. **From the buffer into the field's type**, through `ContentDeserializer`.

When the second step fails, the parser is already past the value. So the failure becomes `Problem::InvalidType` (code `invalid_type`) at the field, and deserialization goes on.

JSON writes every map key as a string. So a key written as a string is read as the integer or boolean its type asks for, as `serde_json` reads it. A well-typed body therefore reads the same with the option as without it.

`Content` and `ContentDeserializer` are the crate's own and are kept private. serde has an equivalent, but it is private to serde.

`Problem::InvalidType` has two parameters:

- `expected` is serde's description of what the type accepts: `u8`, `a string`, `struct Address`. When serde's error doesn't say, it is the type's short name, and for an `Option`, the name of the type inside.
- `found` is the kind of value that was in the input: `boolean`, `integer`, `float`, `character`, `string`, `bytes`, `null`, `sequence` or `map`.

Its message is `must be {expected}, found {found}`.

- **A failure inside the field.** The value that failed may be inside the field: an element of a list, say, or a field of a nested struct that doesn't collect type errors. Then both parameters describe that inner value, as serde's message does.
- **The value is never included.** Nothing the client sent, a password for example, is echoed back.
- **Cost.** Buffering allocates once for each sequence, map, option or newtype in a field. Strings and bytes are borrowed when the format lends them, and moved when the format hands over its own copy. So a flat struct read from `serde_json` allocates no more than it does without the option.
- **Formats.** It needs a self-describing format, such as JSON, YAML or TOML.

### Using it from a web framework

There is no crate for any framework. [`examples/axum`](examples/axum) shows a short extractor instead:

- It reads the body with `annotated::json::from_slice`.
- It responds to a body that isn't JSON as axum's own `Json` does.
- It responds to issues with 422 and an [RFC 9457](https://www.rfc-editor.org/rfc/rfc9457) `application/problem+json` body. The body's `errors` member lists each issue's JSON Pointer, code, params and message.

## Crates and features

| Crate | What it holds | Depends on |
|---|---|---|
| `annotated` | The `Validate` trait, issues, problems and paths, the built-in rules, and collecting deserialization | `annotated-derive`, which it re-exports, and the optional dependencies its features add |
| `annotated-derive` | The `Validate` and `Deserialize` derive macros. Derive macros have to live in a crate of their own | `syn`, `quote`, `proc-macro2` and `serde_derive_internals`. Its `regex` feature adds `regex-syntax`, to check patterns at compile time |

| Feature of `annotated` | Dependency it adds | What it enables |
|---|---|---|
| `serde`, on by default | `serde` | The `Deserialize` trait and derive, and `from_deserializer` |
| `json` | `serde_json` | `annotated::json`. It turns on `serde` too |
| `regex` | `regex` | The `pattern` rule |
| `unicode` | `unicode-segmentation` | `length` counted in graphemes |

## Tests

- **Unit tests** sit beside each rule and each piece of the error model.
- **Integration tests** in `crates/annotated/tests/` use the derives from the outside. They pin what a struct accepts and the issues it reports, comparing whole values.
- **Compile-fail tests** in `crates/annotated/tests/ui/` pin every error the derives emit, including its span. `trybuild` runs them on the stable toolchain only: compiler output differs between versions, so the MSRV job skips them.
- **Doc tests** are the examples in the docs, so every example compiles and runs. The README's test runs as a copy in `tests/json.rs`. Change both together.
- **Property tests** check that the `url` rule agrees, on generated text, with [`iri-string`](https://crates.io/crates/iri-string), a strict RFC 3986 and RFC 3987 validator.
