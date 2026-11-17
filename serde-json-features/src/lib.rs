//! Tests of annotated with `serde_json`'s `arbitrary_precision` and `raw_value` features, in `tests/`.
//!
//! Cargo turns a crate's features on for every crate built with it, so these tests are a workspace of their own:
//! built with annotated's, they would change how `serde_json` reads numbers in every other test.
