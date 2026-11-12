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

- [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/): `<type>(<scope>): <description>`, scope `core`, `derive`, `examples`, `benchmarks`, `ci`, `docs` or `repo`; imperative, lowercase, no trailing period. Breaking changes take `!` and a `BREAKING CHANGE:` footer. The body says why.
- No attribution to Claude: no `Co-Authored-By` trailer and no session link.
- Trunk-based: small, atomic commits straight to `main`, each green on its own, pushed once its checks pass.
- Every change users can notice gets a line in `CHANGELOG.md` under *Unreleased*, in the same commit. Versions follow SemVer; before 1.0, a breaking change bumps the minor version.

Before each commit, run what CI runs:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
```

and check the benchmarks, which CI leaves out for now:

```sh
cargo fmt --check --manifest-path benchmarks/Cargo.toml
cargo clippy --manifest-path benchmarks/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path benchmarks/Cargo.toml
```

## Releasing

- A release is one commit to `main`, `chore(repo): release x.y.z`. It bumps `version` in the workspace `Cargo.toml` and the `=` requirement on `annotated-derive` to match, and turns *Unreleased* in `CHANGELOG.md` into the version's section, with its link.
- Once CI passes on a commit whose version has no `v{major}.{minor}.{patch}` tag yet, CI publishes both crates to crates.io and tags the commit. So bumping the version is what releases: never bump it in a commit that isn't the release.
- CI publishes with the `CARGO_REGISTRY_TOKEN` secret of the `release` environment, which only `main` can use.
