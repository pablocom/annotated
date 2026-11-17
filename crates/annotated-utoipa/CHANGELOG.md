# Changelog

This file lists every change that users of annotated-utoipa can notice. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate follows [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- `write_rules_of::<T>`, which writes the rules of a type that derives `annotated::Deserialize` into the schema utoipa 6 generated for it, among the components of an OpenAPI document.
- `write_rules`, which writes rules into the schema of a name given with them: a generic type's schema, named after its arguments, and the rules of a type that derives `annotated::Validate`.
- Both set each keyword the rules add or tighten in utoipa's own schema, at its place: a property, the `items` of a list, the `additionalProperties` of a map, or a member of the `allOf` utoipa documents a struct with a flattened field as. Nothing else in the document changes, and writing the same rules a second time changes nothing.
- Both return what the document still doesn't say about the rules, each as an `Unwritten` that names the schema, the JSON Pointer of the place in it, and its `Reason`: a rule annotated couldn't write, a type the document has no schema for, or a keyword utoipa's schema can't hold, such as the `allOf` a second pattern on a field goes into. The property stays a string with its first pattern. An `Unwritten` reads as one sentence naming the schema.
