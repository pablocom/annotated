# annotated

How we work in this repository. The [README](README.md) shows how to use the library. [ARCHITECTURE](ARCHITECTURE.md) holds its principles and terms, and says how it's built. Name things with its [terms](ARCHITECTURE.md#terms).

## Code

- Code explains itself: names say what things mean, not how they're built (`missing_fields`, not `filtered_fields`). Booleans read as questions (`is_optional`). No abbreviations beyond universal ones.
- Small functions at one level of abstraction; guard clauses over nesting; types over runtime checks.
- Keep it simple. Reach for a pattern only when the problem in front of you needs it. Trade clarity for speed only on a measured hot path, behind a clear name.
- Comments almost never: only for what the code can't say (an external constraint, a measured trade-off, a surprising invariant). No commented-out code, no change narration.
- Depend on the latest released versions of crates and GitHub Actions. Look them up (`cargo search`, crates.io, the action's releases) each time you add or touch one; never go by memory or the local cache.
- Every public item has a doc comment saying what it means and promises (`missing_docs` enforces it). A doc example is a test: the shortest real use.

## Tests

- Tests document behaviour. Name them as sentences: `a_blank_name_breaks_the_required_rule`. One behaviour each, arrange / act / assert separated by blank lines, no logic, literal expected values, whole values compared.
- Test through the public API: the derives from the outside, rules through `Rule::check`.
- Test first. Watch the test fail on its assertion, not on a compile error (stub first), then make it pass.
- For code written before its test, break the code on purpose and watch the test fail, then restore it.
- Every error the derives emit has a `trybuild` compile-fail test under `tests/ui/` pinning its message and span.
- When reporting work, list the tests you saw fail and why, one line each.

## Performance

- Never make annotated less performant without saying why: slower, or allocating more. A fix or a feature can be worth what it costs; tidier code rarely is.
- Before committing a change that could affect how annotated performs, [measure it](benchmarks/README.md#measuring-a-change-to-annotated) against the commit before; CI doesn't. When a case got slower or allocates more, look for a way that doesn't cost it; if there is none, the commit body says why the cost is worth paying.
- A claim that a change is more performant gives before → after medians from the benchmarks, timed in one sitting on one pinned core.
- The compilation flags and the hardware the benchmarks run on move their numbers, so measuring doesn't replace understanding: know why a change costs or saves what it does, and when a number disagrees with that, find out why before trusting either.

## Commits

- [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/): `<type>(<scope>): <description>`, scope `core`, `derive`, `utoipa`, `axum`, `benchmarks`, `ci`, `docs` or `repo`; imperative, lowercase, no trailing period. Breaking changes take `!` and a `BREAKING CHANGE:` footer. The body says why. The type and the `!` decide the version of the next [release](#releasing), so they have to be true.
- No attribution to Claude: no `Co-Authored-By` trailer and no session link.
- Trunk-based: small, atomic commits straight to `main`, each green on its own, pushed once its checks pass.
- Every change users can notice gets a line under *Unreleased* in the changelog of each crate it changes, in the same commit: `CHANGELOG.md` for annotated, and the one in its directory for a crate released on its own. The lines there become the notes of the next release.

Before each commit, run what CI runs:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --workspace --exclude annotated-axum --exclude annotated-utoipa
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
for feature in "" serde json regex unicode json-schema; do
  cargo clippy --package annotated --all-targets --no-default-features --features "$feature" -- -D warnings
done
```

The second `cargo test` leaves out the crates built on annotated, which turn its features on, so it tests annotated with its default features alone. CI runs both again on the minimum Rust version annotated supports: `cargo +1.88 test ... -- --skip compile_errors`.

CI also fails a push that breaks the public API of a crate on crates.io without a breaking commit since its last release. It writes the next versions with [`release`](release), then runs [cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks), which compares each crate with its latest version on crates.io under the release that version makes. To check a change to the public API first, do the same: `cargo run --package annotated-release -- 2026-01-01`, `cargo semver-checks --package annotated`, then discard what the release tool wrote with `git checkout -- .`.

Check the tests under `serde_json`'s `arbitrary_precision` and `raw_value` features too, a workspace of their own since those features change `serde_json` for every crate built with it:

```sh
cargo fmt --check --manifest-path serde-json-features/Cargo.toml
cargo clippy --manifest-path serde-json-features/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path serde-json-features/Cargo.toml
```

and check the benchmarks, which CI leaves out for now:

```sh
cargo fmt --check --manifest-path benchmarks/Cargo.toml
cargo clippy --manifest-path benchmarks/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path benchmarks/Cargo.toml
```

## Releasing

- annotated's two crates share a version, in the workspace `Cargo.toml`, a changelog, `CHANGELOG.md`, and tags, `v1.2.3`. Every other crate in `crates/` is released on its own: its version is in its manifest, its changelog is the `CHANGELOG.md` in its directory, and its tags are `<crate>-v1.2.3`.
- CI releases every push to `main` that changes a crate's code. Once the push's checks pass, CI commits the releases to `main` as `chore(repo): release` followed by their tags, publishes their crates to crates.io, and creates a GitHub release for each, which tags that commit. Only annotated's releases are marked as the repository's latest. So after a push, wait for its run and pull before changing anything else.
- A commit changes annotated's code when it touches `crates/annotated`, `crates/annotated-derive` or the workspace `Cargo.toml`, and another crate's code when it touches that crate's directory, unless its type is `docs`. A push without such a commit releases nothing, and what it changes goes out with the next release.
- Versions follow SemVer, and the commits since a crate's last release decide its next one. A breaking change bumps the minor version before 1.0 and the major version from 1.0 on. A `feat` bumps the minor version from 1.0 on. Anything else bumps the patch version.
- annotated's types are part of the API of every crate released on its own. So a breaking release of annotated is a breaking release of each of them, and each of their releases requires the version of annotated that is current when it is made.
- The release commit writes each version into its manifest and the lock files, and annotated's into the `=` requirement on `annotated-derive`. It writes annotated's version into every requirement on annotated of a crate it releases. It turns *Unreleased* in each changelog into the version's section, with its link, and that section is the release's notes. With nothing under *Unreleased*, the section says that users can notice no change. CI refuses to release a `feat`, a `fix`, a `perf` or a breaking change that way, and a crate that a breaking release of annotated changes.
- Write a line under *Unreleased* only on top of the last release commit. Git merges a line written before that commit into the released version's section, without a conflict.
- A new crate in `crates/` is left alone until its first version is published by hand, since crates.io's Trusted Publishing serves only a crate it already has. Until then it depends on annotated by `path` alone. To publish it, write its first version's section in its changelog and its requirement on annotated, run `cargo publish --package <crate>`, create its GitHub release with `gh release create <crate>-v<version> --latest=false`, and add this repository's `ci.yml` and `release` environment as its trusted publisher on crates.io. CI releases it from then on.
- [`release`](release) decides the versions and writes those files, and its tests say how. Never bump a version by hand.
- CI publishes a version that has its section in the changelog but no tag as it is. So when publishing fails after the release commit reached `main`, run CI on `main` again: `gh workflow run ci.yml`.
- CI publishes through crates.io's Trusted Publishing, so no token is stored anywhere. crates.io accepts a publish only from `ci.yml` in the `release` environment, which only `main` can use.
