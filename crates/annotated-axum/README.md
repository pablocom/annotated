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

A body that breaks the rules is answered with 422 and [RFC 9457](https://www.rfc-editor.org/rfc/rfc9457) problem details, whose `errors` member lists every issue, as RFC 9457's own example of validation errors does: its JSON Pointer, its code and params, and its message as its `detail`:

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
    { "pointer": "/fullName", "code": "required", "params": {}, "detail": "is required" },
    { "pointer": "/email", "code": "missing", "params": {}, "detail": "is missing" },
    { "pointer": "/age", "code": "range", "params": { "min": 18, "max": 120 }, "detail": "must be between 18 and 120" }
  ]
}
```

That is the answer annotated-axum recommends: a standard format, with the codes and params for a client that translates them, and an English message for people. A rule's own `message` replaces the English one. A `pointer` is an [RFC 6901](https://www.rfc-editor.org/rfc/rfc6901) JSON Pointer in its string form, `/age`, not the fragment of a URI, `#/age`.

RFC 9457 says the type `about:blank` means no more than the status. An app that documents its problems, the `errors` member among them, sets their type to the URI of that documentation, as [Problem details of your own](#problem-details-of-your-own) shows.

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

## Problem details of your own

`ProblemDetails` answers an app's own problems too. Each member it leaves unset is left out of the JSON:

```rust
use annotated_axum::ProblemDetails;
use axum::http::StatusCode;

async fn add_member() -> Result<StatusCode, ProblemDetails> {
    Err(ProblemDetails::new(StatusCode::CONFLICT)
        .with_type("https://example.com/problems/email-taken")
        .with_detail("the email belongs to another member"))
}
```

```json
{
  "type": "https://example.com/problems/email-taken",
  "title": "Conflict",
  "status": 409,
  "detail": "the email belongs to another member"
}
```

`with_title` and `with_instance` set the other two members RFC 9457 defines. The title is the status's reason phrase unless the app sets its own.

The rejection, `ValidJsonRejection`, converts into the `ProblemDetails` it is answered with. To set their type, or any other member, convert the rejection into a rejection of yours, and read the body through `ValidJson` with axum's `FromRequest` derive, from axum's `macros` feature:

```rust
use annotated_axum::{ProblemDetails, ValidJson, ValidJsonRejection};
use axum::extract::FromRequest;
use axum::response::{IntoResponse, Response};

#[derive(FromRequest)]
#[from_request(via(ValidJson), rejection(InvalidBody))]
struct AppJson<T>(T);

struct InvalidBody(ProblemDetails);

impl From<ValidJsonRejection> for InvalidBody {
    fn from(rejection: ValidJsonRejection) -> Self {
        Self(ProblemDetails::from(rejection).with_type("https://example.com/problems/invalid-body"))
    }
}

impl IntoResponse for InvalidBody {
    fn into_response(self) -> Response {
        self.0.into_response()
    }
}
```

Handlers then take `AppJson<T>`.

## A response of your own

The rejection is an error, as axum's own rejections are: its `Display` says why the request was refused, `status()` says the status it is answered with, and `source()` is the error it came from, axum's rejection, `serde_json`'s error or the issues. So your rejection can log it, or build a response of its own from it.

It holds the issues as data. To answer them with a body of your own, convert the rejection into an error type of yours:

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

A rejection the app doesn't answer itself is answered as `ValidJson` answers it.

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
