# Changelog

This file lists every change that users of annotated-utoipa can notice. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate follows [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- `write_rules_of::<T>`, which writes the rules of a type that derives `annotated::Deserialize` into the schema utoipa 6 generated for it, among the components of an OpenAPI document. It returns what the document still doesn't say about them, as an `Unwritten`: a rule annotated couldn't write, a type the document has no schema for, or a keyword utoipa's schema can't hold.
- `write_rules`, which does the same for a schema named by the caller, such as a generic type's, and for rules from elsewhere, such as `Validate::rules`.
- A second pattern on a field is required through an `allOf` of its own, which utoipa's schema can hold.
