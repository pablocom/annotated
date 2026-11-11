# Changelog

Every change users of the crates can notice is listed here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crates follow [Semantic Versioning](https://semver.org/).

## Unreleased

### Added

- The `Validate` trait, with `validate()` answering every violation found, and the error model it reports with: `Violations`, `Violation`, `Problem` with its stable `code()` and `params()`, `Path` with `json_pointer()`, and `Location` and `FoundViolations` for implementing `Validate` by hand.
- The `Rule` trait and the first built-in rules in `annotated::rules`: `required`, which checks anything implementing the `Required` capability trait, and `length`, which counts the `char`s of a string, or its bytes with `in_bytes()` and its grapheme clusters with `in_graphemes()` behind the `unicode` feature, and the elements of anything implementing `HasLength`.
- `#[derive(Validate)]`, re-exported by `annotated`. Rules are written in `#[validate(...)]` on fields: a built-in rule by name, your own by path, given a constructor value (`name = value` or `name(value)`) or named builder arguments (`name(key = value)`), and a `message` override with `{param}` placeholders. Every field is checked, stopping at a field's first broken rule unless the struct has `#[validate(report_all)]`. On an `Option` field, `required` checks the `Option` and every other rule the value inside. Fields are reported by the name serde reads them by, under `rename_all` and `rename`.
