# Changelog

This file lists every change that users of annotated-axum can notice. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate follows [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- `ValidJson<T>`, an axum 0.8 extractor that reads a JSON body with `annotated::json::from_slice`. It answers every refusal with RFC 9457 problem details: the issues in a body with 422, and a request it can't read with the status axum's `Json` answers it with: 415, 413, 400, or 422 for a value of the wrong type in a struct that doesn't collect type errors. The `detail` of a body it can't read says what is wrong and where, and never repeats the client's value. `Option<ValidJson<T>>` is `None` for a request without a `Content-Type`, as `Option<Json<T>>` is.
- `ValidJsonRejection`, which holds the issues as data, so an app can answer them with a response of its own. It is an error, as axum's rejections are: `Display` says why the request was refused, `source()` is the error it came from, and `status()` and `body_text()` say how it is answered. It converts into the `ProblemDetails` it is answered with, for an app to set their type.
- `ProblemDetails`, RFC 9457 problem details as a response. It converts from `annotated::Issues`, whose `errors` member lists each issue's `pointer`, `code`, `params` and `detail`, as RFC 9457's example of validation errors does. `ProblemDetails::new(status)` answers any other problem, and `with_type`, `with_title`, `with_detail` and `with_instance` set its members to the app's own.
