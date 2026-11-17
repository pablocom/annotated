# annotated-axum

[![crates.io](https://img.shields.io/crates/v/annotated-axum)](https://crates.io/crates/annotated-axum)
[![docs.rs](https://img.shields.io/docsrs/annotated-axum)](https://docs.rs/annotated-axum)

An [axum](https://crates.io/crates/axum) extractor, `ValidJson<T>`, that reads a JSON body with [annotated](https://crates.io/crates/annotated)'s collecting deserialization, and answers every issue in it at once: missing fields and broken rules together, each at its JSON Pointer.

## Getting started

```sh
cargo add annotated annotated-axum
```

Derive `annotated::Deserialize` on the body, and take it as `ValidJson`:

```rust
use annotated::Deserialize;
use annotated_axum::ValidJson;
use axum::Router;
use axum::http::StatusCode;
use axum::routing::post;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NewMember {
    #[validate(required, length(max = 50))]
    full_name: String,
    #[validate(email)]
    email: String,
    #[validate(range(min = 18, max = 120))]
    age: u8,
}

async fn add_member(ValidJson(new_member): ValidJson<NewMember>) -> StatusCode {
    StatusCode::CREATED
}

let app: Router = Router::new().route("/members", post(add_member));
```

A body that breaks the rules is answered with 422 and [RFC 9457](https://www.rfc-editor.org/rfc/rfc9457) problem details, whose `errors` member lists every issue's JSON Pointer, code, params and message:

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

That is the answer annotated-axum recommends: a standard format, with the codes and params for a client that translates them, and an English message for people. A rule's own `message` replaces the English one.

## What it refuses

Every refusal is problem details, with the status axum's own `Json` answers the same request with:

| The request | The status | The problem details |
|---|---|---|
| Has issues: missing fields, broken rules, and values of the wrong type in a struct with `#[annotated(collect_type_errors)]` | 422 | `errors` lists every issue |
| Isn't sent as JSON: its `Content-Type` is neither `application/json` nor a type built on JSON, such as `application/vnd.api+json` | 415 | `detail` says so |
| Has a body larger than the app's limit | 413 | `detail` says so |
| Has a body that isn't JSON | 400 | `detail` says where: `the body is not JSON: syntax error at line 1 column 2` |
| Has a value of the wrong type, in a struct that doesn't collect type errors | 422 | `detail` says where: `a value at line 1 column 73 can't be read into the type expected` |

A `detail` never repeats what the client sent, so a password in the wrong place isn't echoed back. To answer values of the wrong type as issues, each at its JSON Pointer and all at once with the rest, put `#[annotated(collect_type_errors)]` on the struct.

## A response of your own

The rejection, `ValidJsonRejection`, holds the issues as data. To answer them with a body of your own, convert the rejection into an error type of yours, and read the body through `ValidJson` with axum's `FromRequest` derive, from axum's `macros` feature:

```rust
use annotated::{DeserializeError, Issues};
use annotated_axum::{ValidJson, ValidJsonRejection};
use axum::extract::FromRequest;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(FromRequest)]
#[from_request(via(ValidJson), rejection(ApiError))]
struct AppJson<T>(T);

enum ApiError {
    Invalid(Issues),
    Unreadable(ValidJsonRejection),
}

impl From<ValidJsonRejection> for ApiError {
    fn from(rejection: ValidJsonRejection) -> Self {
        match rejection {
            ValidJsonRejection::Deserialize(DeserializeError::Issues(issues)) => Self::Invalid(issues),
            rejection => Self::Unreadable(rejection),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            Self::Invalid(issues) => (StatusCode::BAD_REQUEST, issues.to_string()).into_response(),
            Self::Unreadable(rejection) => rejection.into_response(),
        }
    }
}
```

Handlers then take `AppJson<T>`. A rejection the app doesn't answer itself is answered as `ValidJson` answers it.

## Values that already exist

`ProblemDetails` converts from `annotated::Issues`, so a handler that returns `Result<_, ProblemDetails>` answers the issues `Validate::validate` finds with `?`:

```rust
async fn change_password(Json(new_password): Json<NewPassword>) -> Result<StatusCode, ProblemDetails> {
    new_password.validate()?;
    Ok(StatusCode::NO_CONTENT)
}
```

## Versions

annotated-axum works with axum 0.8. A new version of axum that breaks its API comes with a new version of annotated-axum, so upgrading axum means upgrading both.

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/pablocom/annotated/blob/main/LICENSE-APACHE) or [MIT license](https://github.com/pablocom/annotated/blob/main/LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
