# annotated

Directives for working in this repository.

@.claude/directives/coding-style.md
@.claude/directives/testing.md
@.claude/directives/commits.md

Name things with the terms in the [README](README.md#terms), in code, docs and conversation.

Before each commit, run what CI runs:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
```
