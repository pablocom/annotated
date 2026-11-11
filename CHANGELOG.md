# Changelog

Every change users of the crates can notice is listed here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crates follow [Semantic Versioning](https://semver.org/).

## Unreleased

### Added

- The `Validate` trait, with `validate()` answering every violation found, and the error model it reports with: `Violations`, `Violation`, `Problem` with its stable `code()` and `params()`, `Path` with `json_pointer()`, and `Location` and `FoundViolations` for implementing `Validate` by hand.
- The `Rule` trait and the first built-in rules in `annotated::rules`: `required`, which checks anything implementing the `Required` capability trait, and `length`, which counts the `char`s of a string, or its bytes with `in_bytes()` and its grapheme clusters with `in_graphemes()` behind the `unicode` feature, and the elements of anything implementing `HasLength`.
- `#[derive(Validate)]`, re-exported by `annotated`. Rules are written in `#[validate(...)]` on fields: a built-in rule by name, your own by path, given a constructor value (`name = value` or `name(value)`) or named builder arguments (`name(key = value)`), and a `message` override with `{param}` placeholders. Every field is checked, stopping at a field's first broken rule unless the struct has `#[validate(report_all)]`. On an `Option` field, `required` checks the `Option` and every other rule the value inside. Fields are reported by the name serde reads them by, under `rename_all` and `rename`.
- The `range` rule: a value no less than `min` and no more than `max`, for anything `PartialOrd` and `Display`, with the bounds' type taken from the field.
- The `email` rule: an email address as HTML's `<input type="email">` accepts one, at most 254 bytes.
- The `url` rule, behind the `url` feature: an absolute URL valid by the WHATWG URL Standard, of the scheme `http` or `https`, or of those set with `schemes`.
- The `ip` rule: an IPv4 or IPv6 address, or only one of them with `v4()` or `v6()`.
- The `contains`, `prefix` and `suffix` rules: a string containing, starting with or ending with a text.
- The `custom` rule: any `Fn(&V) -> Result<(), Problem>` of your own, checking any value that borrows as a `V`, so a check of `&str` checks a `String` field.
- The `pattern` rule, behind the `regex` feature: a string matching a regular expression. Written in `#[validate(...)]`, the expression is a string literal checked at compile time and compiled once, into a `static` at the field.
