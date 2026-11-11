# Changelog

Every change users of the crates can notice is listed here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crates follow [Semantic Versioning](https://semver.org/).

## Unreleased

### Added

- The `Validate` trait, with `validate()` answering every violation found, and the error model it reports with: `Violations`, `Violation`, `Problem` with its stable `code()` and `params()`, each param a `ParamValue`: an integer, a float or a text, `Path` with `json_pointer()`, and `Location` and `FoundViolations` for implementing `Validate` by hand.
- The `Rule` trait and the first built-in rules in `annotated::rules`: `required`, which checks anything implementing the `Required` capability trait, and `length`, which counts the `char`s of a string, or its bytes with `in_bytes()` and its grapheme clusters with `in_graphemes()` behind the `unicode` feature, and the elements of anything implementing `HasLength`.
- `#[derive(Validate)]`, re-exported by `annotated`. Rules are written in `#[validate(...)]` on fields: a built-in rule by name, your own by path, given a constructor value (`name = value` or `name(value)`) or named builder arguments (`name(key = value)`), and a `message` override with `{param}` placeholders. Every field is checked, stopping at a field's first broken rule unless the struct has `#[validate(report_all)]`. On an `Option` field, `required` checks the `Option` and every other rule the value inside. Fields are reported by the name serde reads them by, under `rename_all` and `rename`.
- The `range` rule: a value no less than `min` and no more than `max`, for anything `PartialOrd` and `Display`, with the bounds' type taken from the field. Bounds of a primitive number type are reported as numbers, and those of any other type as text.
- The `email` rule: an email address as HTML's `<input type="email">` accepts one, at most 254 bytes.
- The `url` rule: an absolute URI with a host by RFC 3986, with RFC 3987's international characters, and without userinfo, whose scheme is `http`, `https` or `ftp`, or one of those set with `schemes`, compared ignoring case. It allocates nothing.
- The `ip`, `ipv4` and `ipv6` rules: an IP address of either version, or only of version 4 or only of version 6, each reporting a problem coded by its own name.
- The `contains`, `prefix` and `suffix` rules: a string containing, starting with or ending with a text.
- The `custom` rule: any `Fn(&V) -> Result<(), Problem>` of your own, checking any value that borrows as a `V`, so a check of `&str` checks a `String` field. It reports with `Problem::custom`, whose `with_param` takes a number or a text.
- The `pattern` rule, behind the `regex` feature: a string matching a regular expression. Written in `#[validate(...)]`, the expression is a string literal checked at compile time and compiled once, into a `static` at the field.
- `nested`, which validates a field implementing `Validate` under the field's name, and `each(...)`, which applies its rules to every element of a collection at its index, or every value of a map at its key, through the `HasElements` capability trait, also behind a reference or a `Box`, like `&'a [T]` or `Box<[T]>`. `each(nested)` validates every element, and `each` nests.
- Struct-level `#[validate(custom(f))]`: any `Fn(&Self) -> Result<(), Problem>`, checked after every field and reported at the struct's own location, or at a field by its serde name with `custom(f, field = name)`. Several are allowed, checked in the order written, beside `report_all`. One at a field counts as that field's last rule, so it is skipped once the field has a violation, unless the struct has `report_all`.
