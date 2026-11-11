# Answering violations from axum

[`src/lib.rs`](src/lib.rs) is an axum extractor, `ValidJson<T>`, to copy into your app. It reads a JSON body with `annotated::json::from_slice` and answers every violation in it at once, with 422 and [RFC 9457](https://www.rfc-editor.org/rfc/rfc9457) problem details listing each one's JSON Pointer, code, params and message; a request it can't read is refused as axum's own `Json` refuses it. [`src/main.rs`](src/main.rs) uses it to take new members, and runs with `cargo run -p annotated-example-axum`:

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
