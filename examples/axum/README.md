# Returning violations from axum

[`src/lib.rs`](src/lib.rs) is an axum extractor, `ValidJson<T>`, to copy into your app.

- It reads a JSON body with `annotated::json::from_slice`.
- When the body has violations, it responds with 422 and [RFC 9457](https://www.rfc-editor.org/rfc/rfc9457) problem details, which list every violation's JSON Pointer, code, params and message.
- When it can't read the request at all, it rejects it the way axum's own `Json` does.

[`src/main.rs`](src/main.rs) uses the extractor to accept new members. Run it with `cargo run -p annotated-example-axum`, then send it a body that breaks the rules:

```sh
curl -i http://127.0.0.1:3000/members -H 'content-type: application/json' -d '{ "fullName": "  ", "age": 12 }'
```

```http
HTTP/1.1 422 Unprocessable Entity
content-type: application/problem+json

{
  "type": "about:blank",
  "title": "Unprocessable Content",
  "status": 422,
  "errors": [
    { "pointer": "/fullName", "code": "required", "params": {}, "message": "is required" },
    { "pointer": "/email", "code": "missing", "params": {}, "message": "is missing" },
    { "pointer": "/age", "code": "range", "params": { "min": 18, "max": 120 }, "message": "must be between 18 and 120" }
  ]
}
```
