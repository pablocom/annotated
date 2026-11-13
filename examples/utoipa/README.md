# Rules in an OpenAPI document

[utoipa](https://crates.io/crates/utoipa) generates a schema from a type's fields, and knows nothing of its `#[validate(...)]` rules. [`src/lib.rs`](src/lib.rs) is a function to copy into your app, `write_rules_of::<T>`, which writes the rules of `T` into the schema utoipa made for it.

- It asks `T` for its rules as data, which annotated's derives provide with the `json-schema` feature.
- It turns utoipa's schema into JSON, lets `annotated::json_schema::write_rules` add the keywords, and reads the schema back.
- It panics when the document has no schema for `T`, so a type missing from the document is noticed when the app starts.

[`src/main.rs`](src/main.rs) documents an API that takes new members. Run it with `cargo run -p annotated-example-utoipa`. A field written like this:

```rust
#[validate(required, length(max = 50))]
full_name: String,
```

is documented like this, where utoipa alone says `{ "type": "string" }`:

```json
"fullName": { "type": "string", "minLength": 1, "maxLength": 50 }
```

## Where to call it

Name each type once, where the document is built:

- **With `#[derive(OpenApi)]`**, call it from a `Modify`, as `src/main.rs` does.
- **With utoipa-axum's `OpenApiRouter`**, call it on the document that `split_for_parts()` returns. A `Modify` runs before the routes add their schemas, so it would find none of them.

A nested type has rules of its own, so it takes its own call: `Address` in `src/main.rs`.

For a type that derives `Validate` and is read by serde's own `Deserialize`, ask `Validate::rules` instead: change the bound on `T` and the call.
