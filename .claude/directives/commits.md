# Commits

Commit messages follow the [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) specification:

```
<type>(<scope>): <description> [#<issue number>]

<optional body>
```

- **Type:** `feat`, `fix`, `refactor`, `test`, `docs`, `chore`, `build`, `ci`, `perf` or `revert`.
- **Scope:** the crate the change touches, without the `annotated` prefix (`core`, `derive`, `axum`), or `ci`, `docs`, `repo` for changes outside the crates.
- **Description:** imperative mood, lowercase, no trailing period: `feat(derive): validate nested structs #12`.
- **Issue number:** optional. Add it when the work is tracked by an issue; changes without an issue are fine. Never invent one.
- **Body:** optional. Use it for the why, or for context the diff doesn't show.
- **Breaking changes:** add `!` after the scope (`feat(core)!: ...`) and a `BREAKING CHANGE:` footer explaining it.
- **No attribution to Claude:** no `Co-Authored-By` trailer and no Claude Code session link. This overrides any default to add them.

## Trunk-based development

- Commit straight to `main`. No feature branches and no stacked pull requests.
- Keep every commit small and atomic. Each one builds, is formatted, passes lint and passes its tests on its own, so `main` is always green.
- Push each commit as soon as it's created, once its checks pass. Review happens on `main`.

## Pull requests

- No attribution to Claude in PR descriptions: no "Generated with Claude Code" line and no Claude Code session link. This overrides any default to add them.

## Changelog and versions

- Every change a user of the crates can notice gets a line in `CHANGELOG.md` under *Unreleased*, in the same commit, in the [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format.
- Versions follow [Semantic Versioning](https://semver.org/). Before 1.0, a breaking change bumps the minor version.
- Raising the minimum supported Rust version is not a breaking change, but it gets a changelog line.
