# Changelog

This file lists every change that users of annotated-axum can notice. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate follows [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- `ValidJson<T>`, an axum 0.8 extractor that reads a JSON body with `annotated::json::from_slice`. It answers every refusal with RFC 9457 problem details: the issues in a body with 422, and a request it can't read with the status axum's `Json` answers it with: 415, 413, 400, or 422 for a value of the wrong type in a struct that doesn't collect type errors. The `detail` of a body it can't read says what is wrong and where, and never repeats the client's value.
- `ValidJsonRejection`, which holds the issues as data, so an app can answer them with a response of its own.
- `ProblemDetails`, the problem details of a refusal, which converts from `annotated::Issues` and is a response.
