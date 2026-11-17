# Changelog

This file lists every change that users of annotated-axum can notice. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate follows [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- `ValidJson<T>`, an axum 0.8 extractor that reads a JSON body with `annotated::json::from_slice`. It answers the issues in a body with 422 and RFC 9457 problem details, and refuses a request it can't read as axum's `Json` does: 415, 400, or 422 for a value of the wrong type in a struct that doesn't collect type errors.
- `ValidJsonRejection`, which holds the issues as data, so an app can answer them with a response of its own.
- `ProblemDetails`, the problem details of a set of issues, which converts from `annotated::Issues` and is a response.
