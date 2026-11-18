# Changelog

This file lists every change that users of the crates can notice. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crates follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.5.1] - 2026-11-18

### Added

- `each(...)` goes through the elements of a `HashSet` and a `BTreeSet`, `each(nested)` included.
  - `validate()` reports an element at its index in the set's order, which for a `HashSet` is arbitrary.
  - `#[derive(Deserialize)]` reports an element at its index in the input, where a client can find it, in the order of the input. Two equal elements are both checked, and the set keeps the first, as serde's does. Other rules on the field check the set, so `length` counts its distinct elements.
  - A set that isn't read from the input, such as a default or one returned by `deserialize_with`, is checked in the set's order.
  - Under `#[derive(Deserialize)]`, the field's type has to name the set, alone or in an `Option`. `each` through a set inside another value, such as `Vec<HashSet<String>>` with `each(each(...))` or a `Box<HashSet<String>>`, is a compile error, and so is a set behind a type alias.
  - A forgotten `nested` on a set of values with rules of their own suggests `each(nested)`.

### Fixed

- The compile error for `each(...)` on a value without elements names what `each` goes through, and says to implement `rules::HasElements` for a collection of your own. It told you to implement it for the value's type, which Rust forbids for a type of another crate, such as `u8`.

## [0.5.0] - 2026-11-18

### Added

- Functions that take the most issues to keep: `annotated::from_deserializer_limited_to`, `annotated::json::from_str_limited_to`, `annotated::json::from_slice_limited_to` and `Validate::validate_limited_to`. A limit of 0 is taken as 1, so that a value with issues never passes.
- `Report::limited_to(most_issues)`, `Report::unlimited()` and `Report::DEFAULT_LIMIT`, 1,000, for code that starts a report itself.
- `Issues::is_complete()`: `false` when the report the issues were found in reached its limit, so the value may have more issues than are listed. It is always `false` for the issue returned when failing fast.
- `nested` works on a value behind a `Box`, an `Rc`, an `Arc`, a `&`, a `&mut` or a `Cow` under `#[derive(Validate)]`, as `Box` already did under `#[derive(Deserialize)]`, and on a tuple of up to 16 elements under both derives, each element reported at its index: `leg[1].street`. `Validate` is implemented for each of them, and with the `json-schema` feature a pointer's `rules()` are those of the value it points to.
- `annotated::Deserialize` is implemented for `Rc<T>`, `Arc<T>`, arrays `[T; N]` and tuples of up to 16 elements, so `nested` reads a value behind an `Rc` or an `Arc` and a tuple's elements, and `each(nested)` an array's, with every issue at its path. Each accepts what serde's own implementation accepts and fails as it fails, and `Rc` and `Arc` don't need serde's `rc` feature.
- `Issues`, `Issue`, `Path`, `PathSegment`, `Problem` and `ParamValue` implement serde's `Serialize`, with the `serde` feature, on by default, so an app sends the issues without writing its own copy of them. An issue is `{"path": ["members", 1, "name"], "code": "range", "params": {"min": 18, "max": 120}, "message": "must be between 18 and 120"}`: its path as a list of segments, a field or a key as a string and an index as a number, its params as an object in their order, empty when there are none, and the message `Issue::message()` returns. `Issues` is the list of its issues, a `Problem` its `code` and `params`, and a `ParamValue` its bare number or text.
- `Report::push_issue(issue)`, which records an `Issue` an app already has, from another validation or from a check of its own, at its own path and with its own message, unless the report is at its limit. With `Issue::new` and a problem made by its rule, as `Rule::<str>::problem(&rules::length().max(50))`, an app joins its own issues to those of a validation.
- `#[annotated(crate = "path")]` on a struct or an enum names the path the generated code finds annotated by, for both derives, so that a crate re-exporting annotated can offer them to users who don't depend on it: `#[derive(my_api::checks::Validate)]` with `#[annotated(crate = "my_api::checks")]`. `#[derive(Deserialize)]` also reads `#[serde(crate = "path")]`, as serde does, for the path to serde. Both were compile errors.
- `#[derive(Validate)]` reads `#[annotated(...)]` too, and refuses an option it doesn't know, or one on a variant or a field, as `#[derive(Deserialize)]` does. It ignores `collect_type_errors`, which is `Deserialize`'s.
- `#[derive(Deserialize)]` reads `#[serde(bound = "...")]` and `#[serde(bound(deserialize = "..."))]` on a struct, an enum, a variant and a field, as serde does: the bounds given replace those the derive decides. They were a compile error.
- `#[validate(not_nested)]` on a field says that the rules of its type are left unchecked on purpose: `validate()` doesn't check them, and `#[derive(Deserialize)]` reads the field as serde does. It checks nothing itself, describes nothing in `rules()` and takes no arguments. It can stand beside other rules, but beside `nested` or `each(nested)` it is a compile error, and so is `each(not_nested)`.

### Changed

- **A field holding a value with rules of its own no longer compiles without `nested` or `not_nested`.** Add `#[validate(nested)]`, or `#[validate(each(nested))]` for a collection's elements, to check those rules, or `#[validate(not_nested)]` to leave the field as it was. The error is at the field's type, and says which of `nested` and `each(nested)` fits it.
  - A value has rules of its own when its type implements `Validate`, or, for a field `#[derive(Deserialize)]` has serde read, annotated's `Deserialize`. Without `nested`, `validate()` never checked those rules, and deserializing had serde read the value, so its issues ended deserialization as the format's error, at paths inside the value, and the struct's other issues were lost. Under `#[annotated(collect_type_errors)]`, they read as `address: must be Address, found map`.
  - The derives ask about the field's type and each type written in it, so `Option<Address>`, `Vec<Address>`, `Box<Address>` and `HashMap<String, Vec<Address>>` need it too. They can't tell for a type that mentions a type, const or lifetime parameter of the type deriving, such as `Vec<T>`, which compiles as before, and a `PhantomData<Address>` holds no value to check.
  - A field `#[derive(Deserialize)]` doesn't read, because it is skipped or read by a function, needs `nested` when its type implements `Validate`, which `nested` checks it with.

- `#[derive(Deserialize)]` bounds the type parameters a field uses, as serde's derive does, where it bounded the field's type: a `Vec<T>` with `each(nested)` bounds `T: annotated::Deserialize<'de>`, where it bounded `Vec<T>`, and a `Vec<T>` read by serde bounds `T: serde::Deserialize<'de>`. A type parameter used only in `PhantomData`, or only in the type deriving, as in a recursive `Vec<Tree<T>>`, is bounded by nothing. **A bound can now ask more than a type needs, and code that compiled may not:**
  - A `Wrapper<T>` with `nested` bounds `T` by `annotated::Deserialize`, whatever `Wrapper<T>` needs, and a `Foo<T>` read by serde bounds `T` by serde's `Deserialize`, even when `Foo<T>` deserializes whatever `T` is. Give the bounds with `#[serde(bound = "...")]` instead.
  - A `HashMap<K, V>` with `each(nested)` bounds `V` by annotated's trait and `K` by serde's, so the struct now has to bound `K` by what reading the map needs, `Eq + Hash + Display + Clone`, which the bound on the whole field carried before.
- `required` on an `Option` field asks for the field's key, and accepts `null`, where it asked for a `Some`. **Nothing tells you at compile time: a `required` `Option` that refused `null` now accepts it, as `None`.**
  - `#[derive(Deserialize)]` reports an absent key as `Problem::Missing`, code `missing`, as for a field of any other type, where it reported `Problem::Required`. It does so even when the field or its struct has `#[serde(default)]`, and the issue carries the `message` of `required` when it has one. In an untagged enum, a variant without that key isn't built, as for any other missing field.
  - A value that already exists can't say whether its key was there, so `validate()` checks nothing for `required` on an `Option`, and `rules::required()` passes every `Option`, `None` included. `each(required)` no longer refuses an element that is `None`.
  - A skipped or flattened `Option`, and one in a tuple variant, has no key of its own, and `required` on it checks nothing.
  - With the `json-schema` feature, `json_schema::write_rules` lists a `required` `Option` in the object's `required`, even when it has a serde default, and leaves `null` allowed, where it removed `null` from the field's schema. `rules()` describes such a field as taking no default.
  - To keep refusing `null`, declare the field as `T` instead of `Option<T>`. An absent key is then `missing`, and `null` is a value of the wrong type: the format's error, or a `Problem::InvalidType` under `#[annotated(collect_type_errors)]`. Keep `required` on it only to check the value too: on a `String`, it refuses blank text.
- A report keeps at most 1,000 issues by default, where it kept every issue. A 2 MiB body that was a list of `{}` against a struct with four required fields gave 2.8 million issues and held 704 MiB. Now `json::from_str`, `json::from_slice`, `from_deserializer`, `Validate::validate` and serde's own `Deserialize` return at most 1,000, and `Report::new()` and `Report::default()` keep at most 1,000.
  - The input is still read to its end, and nothing more is kept. The issues kept are listed in the order the full report lists them, and for a struct, an enum or a list they are its first ones. Past the limit, which of a map's issues are kept is unspecified, except that failing fast still keeps exactly the first.
  - A report may stop below its limit, when issues may be missing among those it holds: when a list's elements are checked by a rule before `nested`, and the elements' issues reached the limit. `Issues::is_complete()` is then `false`.
  - To keep every issue, as before, pass `Report::unlimited()` to `validate_into` or `deserialize_at`, or call a `_limited_to` function with `usize::MAX`.
- `Report::is_finished()` returns whether the report holds as many issues as its limit, so a report that doesn't fail fast is finished at its limit. `Report::new_alike()` starts a report with the same limit. `Report::append` takes only as many issues as the report has room for, and once it takes every issue of a finished report and still has room, the report keeps no more.
- Two `Issues` are equal only when `is_complete()` is equal too.
- The `url` rule refuses what no one means by a URL, and accepts URLs that browsers send, where RFC 3986 and RFC 3987 say otherwise.
  - A port above 65535 breaks it: `http://example.com:99999`.
  - A host that ends in a number must be an IPv4 address of four decimal numbers up to 255, which browsers would otherwise read as another address: `http://256.0.0.1`, `http://2130706433`, `http://1.2.3` and `http://0x7f.0.0.1` break it.
  - A host with percent-encoding breaks it: `http://%31%32%37.0.0.1`, `http://r%C3%A9sum%C3%A9.example.org`.
  - An `IPvFuture` host breaks it: `http://[v1.x]`.
  - Whitespace and invisible characters break it anywhere, even those RFC 3987 allows: a no-break space (U+00A0), a zero-width space (U+200B), a line separator (U+2028), a right-to-left override (U+202E), and every other character in Unicode's `White_Space` or `Default_Ignorable_Code_Point`.
  - A query and a fragment take `[`, `]` and `|`: `https://fonts.example/css?family=A|B` and `https://api.example/items?filter[x]=1` follow it.
- The minimum supported Rust version is 1.88, where it was 1.98. It is now raised only when annotated needs a newer Rust, and never to a release less than six months old, where it followed the three most recent stable releases.

### Fixed

- A rule given the same named argument twice, `length(max = 5, max = 6)`, or two `message`s, is a compile error naming the argument. Before, the builder was called twice and the last value won, and the last `message` replaced the first. The same goes for `field` and `message` in a struct's or a variant's `custom(...)`.
- A built-in `length` or `range` whose `min` is greater than its `max`, `length(min = 10, max = 5)`, is a compile error, and so is `equal` beside `min` or `max` in a `length`. Each compiled before, and the rule refused every value, or `equal` silently overrode the other bound. Bounds can be compared only when both are literals: written as constants, they still compile.
- With the `json-schema` feature, `rules()` reads a `pattern` from the `static` its check compiled, so a derive compiles each regular expression once, and holds it once, where it compiled it twice. Rules with the same expression share one `static` too.
- `#[validate(pattern = r"...")]` is a compile error when the regular expression is too big for the `regex` crate to compile, such as `^[\p{L}\p{N}_-]{1,1000}$`. It compiled before, then panicked the first time the field was validated, and every validation after that panicked too. The derive builds the expression with the same `regex` crate, so one that compiles can't panic at run time. The error says to use a smaller repetition or a simpler class. The `regex` feature now builds the `regex` crate for the derive too, which adds about 0.2 seconds to a clean build.
- A map's key in an issue's path is at most 256 `char`s: a longer key is cut to its first 256 followed by `…`, in `path()`, its `Display` and its `json_pointer()`. Before, every issue under a key held a whole copy of it, so a body with a 100 kB key and thousands of issues under it took hundreds of MiB. `Path::key` cuts a key the same way, and `PathSegment::MAX_KEY_CHARS` is the bound.
- A map's key is printed escaped in a path: `\`, control chars such as a newline, and chars that hide text or break a line, such as U+202E and U+2028, as `str::escape_debug` writes them. Quotes and printable non-ASCII chars are printed as they are. Before, a key holding a newline printed what looked like another issue in `Issues`' `Display` and in the message of a serde error. `json_pointer()` keeps a key as it is.
- `{param}` placeholders in the message of a `Problem::custom(...)` are filled from its parameters, as in a rule's `message`: `Problem::custom("too_many_guests", "use {max} guests or fewer").with_param("max", 8)` reads as `use 8 guests or fewer`, where it read as `use {max} guests or fewer`. `{{` and `}}` in such a message now stand for one brace each.
- `length(unit = "graphemes")` stops counting once it knows the answer, instead of splitting the whole string into graphemes: `length(max = 50, unit = "graphemes")` counts at most 51 graphemes, `length(min = 3, unit = "graphemes")` at most 3. A 640 kB string checked against `max = 50` took milliseconds, and now takes under a microsecond. The problem reported is the same.
- Types that collect type errors and nest in one another buffer the input once, at the outermost of them, and the others read their values from that buffer. Each buffered its own value again before, so a body cost its size times its depth: 200 kB of structs nested 60 deep allocated 55 MB in 345,331 allocations, and now 2.3 MB in 11,450. A struct or an enum nesting itself sixty deep now allocates about as much as one nested once.
- With the `json-schema` feature, `json_schema::write_rules` writes a `range` bound on an `f32` as the value the rule compares against, not as the decimal it was written as. `range(max = 0.1)` wrote `maximum: 0.1`, below the `f32` 0.1, which is 0.10000000149011612, so the schema rejected a number the rule accepts: the one a client sends when it widens its float. It writes `maximum: 0.10000000149011612` now, and `range(min = 0.7)` writes `minimum: 0.699999988079071`, where it wrote `0.7`. Bounds on an `f64` and on integers were exact. `rules()` holds the exact bound too, since `Rule::problem` says it, while the problem of a failed check still reports the decimal: `0.1`.
- With the `json-schema` feature, `json_schema::write_rules` returns a `required` rule it can't write because the `required` of the schema isn't a list, such as `"required": true`, as unwritten for `Reason::NoSchema`, at the field's property. It left that `required` as it was and returned nothing, so the rule was lost without a word. The schema is still left as it was.
- `#[derive(Validate)]` bounds the type parameters of a nested value by `Validate`, so `struct Page<T> { #[validate(each(nested))] items: Vec<T> }` compiles. It added no bound, and the impl failed to compile.
- A recursive generic struct, `struct Tree<T> { #[validate(each(nested))] children: Vec<Tree<T>> }`, derives `Deserialize`, and reads a `Tree<u8>`. It failed with E0275, since the bound on the field asked for the impl being written.
- `#[derive(Validate)]` reports the field of a struct with `#[serde(transparent)]` at the struct's own location, where serde reads its value: an issue in an `EmailAddress` held at `email` is at `email`, where it was at `email.address`. With the `json-schema` feature, `rules()` no longer describes that field under its name.
- `#[derive(Validate)]` leaves out a serde attribute it can't read, such as one a later serde added, and reads the others, where it refused to compile. serde's own derive reports the attribute, when the type has it. `#[derive(Deserialize)]` still refuses it, since it reads the input as serde would.
- `Self` in a rule's arguments, `range(max = Self::MAX_AGE)` or `custom(Self::check)`, and in a field's type, `Vec<Self>`, is the type deriving `Deserialize`. It failed to compile, since the code reading the fields runs in a visitor of the derive's own, where `Self` was the visitor.
- A deprecated struct or enum, or an enum with a deprecated variant, derives both traits without a deprecation warning in the code they generate, which named the type. As serde's derive does, that code allows the use of deprecated items then. Every impl the derives generate is now marked `#[automatically_derived]`, as serde's are.
- A type, a type parameter or a lifetime named as the derives name items of their own, `__Visitor`, `__Field`, `__D` or `'de` among them, is a compile error that names it, as serde's derive refuses a lifetime named `'de`. It failed to compile with errors about the derive's items, and a type named `__Visitor` with one about a field it has. A field or a variant can have any name.
- A user type named `str`, `u8` or `u64` in scope no longer breaks `#[derive(Deserialize)]`, which named those primitive types without a path.
- A struct passed through a `macro_rules!` derives `Deserialize` with a flattened field. It failed to compile with `E0424`, ``cannot find value `self` ``, since the code generated at the field took the macro's hygiene. That code is no longer linted as yours either: `clippy::expect_used` fired on the code generated for `pattern`, and `clippy::question_mark_used` and `clippy::absolute_paths` on the code generated at a field's type and at a rule.
- A field whose type is passed to a `macro_rules!` as `$t:ty`, or written in parentheses, is an `Option` to the derives when it is one. `#[derive(Deserialize)]` reads its absent key as `None`, as serde does, where it reported it `missing`, and a rule on it checks the value inside, where it failed to compile.
- A struct variant of an untagged enum, or of one with `#[serde(tag = "...", content = "...")]`, is no longer read from a sequence, as serde never reads one there. `["ES12"]` built `Transfer { iban: String }` in an untagged enum, where serde tries the next variant; and `{"type": "Transfer", "data": ["ES12"]}` built it in an adjacently tagged enum, where serde refuses the body. A variant with `#[serde(untagged)]` in a tagged enum is read the same way. A struct variant read by its name, or beside its tag with `tag` alone, still takes a sequence, as with serde.
- An internally tagged newtype variant of `()` or of a unit struct reads from its tag alone, as serde reads it: `{"type": "Ping"}` is `Ping(())`, where it was refused with `invalid type: map, expected unit`, and `{"type": "Pong"}` or `["Pong"]` is `Pong(Marker)`. An internally tagged unit variant given more than its tag in a sequence, `["Cash", 1]`, is refused, as with serde, where it was read. A unit is still refused from an empty map where serde reads it from the format, as in a field of a struct collecting type errors.
- A struct variant written as a sequence, `{"Transfer": ["ES12", "rent"]}`, which `rmp-serde` writes by default, is read as serde reads it in an externally tagged enum collecting type errors, in one inside a type collecting them, in a flattened enum, and in a variant an untagged enum tries. It was refused: with `invalid_type` under `collect_type_errors`, and with the format's error `invalid type: sequence, expected struct variant` in a flattened enum.
- A flattened enum collecting type errors takes the first key left that names one of its variants, as serde does, and leaves the other keys to the flattened fields after it. `{"id": 1, "Card": {"number": "4111"}, "note": "leave at the door"}` was refused with `invalid_type(expected: enum Payment, found: map)`, since the enum read every key left, and a flattened map after it got the variant's key too. A body with no key naming a variant is now the format's error, `no variant of enum Payment found in flattened data`, as with serde, where it was an `unknown_variant` or `invalid_type` issue.
- Under `#[annotated(collect_type_errors)]`, a key given twice, or an unknown key a struct with `deny_unknown_fields` denies, is the format's error at any depth, as without the option and with serde. `{"street": {"name": "Gran Vía", "name": "Alcalá"}}` was refused with `street: invalid_type(expected: Street, found: map)`, and is now refused with ``duplicate field `name` ``. The same goes for such a key in a value serde's derive reads, in a flattened field, in the variant of an enum collecting type errors, and beside the tag and the content of an adjacently tagged enum that denies unknown fields. The error is found once the buffered value is read, so `serde_json` places it past that value rather than at the key, and nowhere when that value is the whole body.
- A tag is read as serde reads it. An integer names no variant in JSON, as with serde: `{"type": 0}` was read as the first variant of an internally or adjacently tagged enum, and is now `type: unknown_variant`, as `{"type": true}` was. A format that reads a variant's index for its name, such as MessagePack, still reads one, and so does an externally tagged enum collecting type errors in it, which refused `0` with `invalid_type`. An adjacent tag given as a unit variant, `{"type": {"Cash": null}}`, is read as with serde, where it was `type: unknown_variant`. Read from serde's own buffer, as inside an untagged enum or beside an internal tag, an index is a name only when its format wrote it as a `u8` or a `u64`, and an adjacent tag is never an index, as with serde. Under `collect_type_errors`, an internal tag given twice is the format's error with its line and column, as without the option.
- Under `#[annotated(collect_type_errors)]`, with `serde_json`'s `arbitrary_precision` feature on anywhere in the build, a float and an integer beyond 64 bits are read as with serde: `{"ratio": 0.5}` was refused with `ratio: must be f64, found map`, and `{"total": 1180591620717411303424}` for a `u128` with `total: must be u128, found map`. A number of the wrong type is found as the number it is, where it was found as a map. A float beside an internal tag is still refused, as serde's own buffer refuses it with that feature.
- An adjacently tagged enum read from a sequence needs its content whatever the variant, as serde reads one: `["Cash"]` was read as the unit variant `Cash`, and is now `data: missing`.

## [0.4.0] - 2026-11-16

### Changed

- `json_schema::write_rules` returns each rule it couldn't write, as a `json_schema::UnwrittenRule`, where it returned nothing. Each has the JSON Pointer where its keyword belongs, its problem, and a `json_schema::Reason`: no subschema there, a `$ref`, a subschema of another type, or no keyword that says what the rule requires. Before, those rules were left out without a word. The function is `#[must_use]`, so a call that ignores the rules returned now warns.

### Fixed

- `json_schema::write_rules` finds a field's property among the `properties` of the schema's `allOf` members too, at any depth. utoipa documents a struct with a flattened field as `allOf`, so none of its fields got their rules before.

## [0.3.1] - 2026-11-16

No change that users can notice.

## [0.3.0] - 2026-11-13

### Added

- `Rule::problem`: a rule says what it requires, as the problem that every value breaking it has.
  - Each built-in rule answers its own problem: `length().max(50)` answers a `Problem::Length` of at most 50, counted in chars for a string and in elements for a collection. A `length` or a `range` without bounds answers `None`.
  - It answers `None` by default, so `custom(...)` and rules of your own say nothing until they implement it.
- The `json-schema` feature, which writes a struct's rules into the JSON Schema that a schema library generated for it. It needs `serde_json`, and depends on no schema library.
  - `Validate::rules` and `Deserialize::rules` return a type's rules as data: `Rules`, with a `FieldRules` for each field that has rules and the `ValueRules` of its value. Both derives implement them for a struct. An enum and a type implemented by hand answer no rules.
  - `annotated::json_schema::write_rules` adds a keyword for each rule that has one: `maxLength`, `minItems`, `minimum`, `format`, `pattern` and the others its documentation lists. `required` on an `Option` removes `null` and lists the field as required.
  - The schema never rejects a value the rules accept: where no keyword says exactly what a rule requires, a looser one is written, or none. A keyword already in the schema is never loosened.
  - A field read by a function, a skipped field and a flattened field are left out, and so are `nested`, `custom(...)` and rules across fields.
  - Without the feature, the derives generate what they did before.
  - `examples/utoipa` writes the rules into an OpenAPI document generated by utoipa.
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

[Unreleased]: https://github.com/pablocom/annotated/compare/v0.5.1...HEAD
[0.5.1]: https://github.com/pablocom/annotated/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/pablocom/annotated/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/pablocom/annotated/compare/v0.3.1...v0.4.0
[0.3.1]: https://github.com/pablocom/annotated/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/pablocom/annotated/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/pablocom/annotated/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/pablocom/annotated/releases/tag/v0.1.0
