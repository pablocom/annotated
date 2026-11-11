# Changelog

Every change users of the crates can notice is listed here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crates follow [Semantic Versioning](https://semver.org/).

## Unreleased

### Added

- The `Validate` trait, with `validate()` answering every violation found, and the error model it reports with: `Violations`, `Violation`, `Problem` with its stable `code()` and `params()`, `Path` with `json_pointer()`, and `Location` and `FoundViolations` for implementing `Validate` by hand.
