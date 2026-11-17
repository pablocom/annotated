# annotated-utoipa

[![crates.io](https://img.shields.io/crates/v/annotated-utoipa)](https://crates.io/crates/annotated-utoipa)
[![docs.rs](https://img.shields.io/docsrs/annotated-utoipa)](https://docs.rs/annotated-utoipa)

Writes the rules of [annotated](https://crates.io/crates/annotated)'s types into the OpenAPI document that [utoipa](https://crates.io/crates/utoipa) generates. utoipa documents a type's fields and knows nothing of its `#[validate(...)]` rules, so a field written like this:

```rust
#[validate(required, length(max = 50))]
full_name: String,
```

is documented like this, where utoipa alone says `{ "type": "string" }`:

```json
"fullName": { "type": "string", "minLength": 1, "maxLength": 50 }
```

## Getting started

```sh
cargo add annotated annotated-utoipa utoipa
```

Derive `annotated::Deserialize` and `utoipa::ToSchema` on a type, and write its rules into the document where the document is built:

```rust
use annotated::Deserialize;
use utoipa::openapi::OpenApi as Document;
use utoipa::{Modify, OpenApi, ToSchema};

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct NewMember {
    #[validate(required, length(max = 50))]
    full_name: String,
    #[validate(range(min = 18, max = 120))]
    age: u8,
}

struct RulesInTheSchemas;

impl Modify for RulesInTheSchemas {
    fn modify(&self, document: &mut Document) {
        let unwritten = annotated_utoipa::write_rules_of::<NewMember>(document);
        if let Some(rule) = unwritten.first() {
            panic!("the document misses a rule of NewMember: {rule}");
        }
    }
}

#[derive(OpenApi)]
#[openapi(components(schemas(NewMember)), modifiers(&RulesInTheSchemas))]
struct MembersApi;
```

annotated-utoipa turns on annotated's `json-schema` feature, with which the derives describe each type's rules. [annotated's documentation](https://docs.rs/annotated/latest/annotated/json_schema/) lists what each rule writes. The schema never rejects a value the rules accept: where no keyword says exactly what a rule requires, a looser one is written, or none.

## Where to call it

Call `write_rules_of` once for each type, where the document is built:

- **With `#[derive(OpenApi)]`**, from a `Modify` in its `modifiers`, as above.
- **With utoipa-axum's `OpenApiRouter`**, on the document that `split_for_parts()` returns. A `Modify` runs before the routes add their schemas, so it would find none of them.

A type `nested` in another has rules of its own, so it takes a call of its own.

## What the document still doesn't say

Each call returns what the document still doesn't say about the type's rules, and leaves the choice to you. An app that wants every rule in its document panics on any, as above, so a rule missing from the document is noticed when the app starts. An app that can do without some lets those through. Each is an `Unwritten`:

- **`Rule`**: a rule annotated couldn't write, with the JSON Pointer where its keyword belongs and why. `ip` is one, since no keyword says "an IPv4 or IPv6 address". So is a rule on a field whose schema is a `$ref`.
- **`NoSchema`**: the document has no schema by the type's name, so it says none of its rules.
- **`LeftOut`**: a keyword annotated wrote that utoipa's schema can't hold, with its JSON Pointer.

## Other types

- **A generic type** is named after its arguments in the document, `Page_Address` for `Page<Address>`, and utoipa only gives its type's name, `Page`. Name its schema with `write_rules`:

  ```rust
  let unwritten = annotated_utoipa::write_rules(&mut document, "Page_Address", &<Page<Address> as Deserialize>::rules());
  ```

- **A type that derives `annotated::Validate`** and is read by serde's own `Deserialize` has its rules in `Validate::rules`. Pass them to `write_rules` the same way.

## How the schema is written

utoipa's schema becomes JSON, annotated writes the rules into it, and utoipa reads it back. utoipa's schema of a string has room for one `pattern` and no `allOf`, so a second pattern, as `prefix` and `suffix` on one field write, is required through an `allOf` of its own: the field's keywords are its first member, and the second pattern its second.

```json
"sku": {
  "allOf": [
    { "type": "string", "maxLength": 20, "pattern": "^SKU-" },
    { "type": "string", "pattern": "-X$" }
  ]
}
```

## Versions

annotated-utoipa works with utoipa 6. A new major version of utoipa comes with a new version of annotated-utoipa, so upgrading utoipa means upgrading both.

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/pablocom/annotated/blob/main/LICENSE-APACHE) or [MIT license](https://github.com/pablocom/annotated/blob/main/LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
