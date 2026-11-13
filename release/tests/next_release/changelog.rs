//! A release turns what `CHANGELOG.md` lists under Unreleased into the section of its version.

use crate::support::Repository;

#[test]
fn the_unreleased_changes_become_the_section_of_the_released_version() {
    let repository = Repository::released("0.2.0");
    repository.commit_noting(
        "### Added\n\n- `range` takes exclusive bounds.",
        "feat(core): take exclusive bounds in range",
    );

    repository
        .write_next_release_on("2026-11-13")
        .expect("the release is written");

    assert_eq!(
        repository.read("CHANGELOG.md"),
        "\
# Changelog

## [Unreleased]

## [0.2.1] - 2026-11-13

### Added

- `range` takes exclusive bounds.

## [0.2.0] - 2026-11-12

[Unreleased]: https://github.com/pablocom/annotated/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/pablocom/annotated/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/pablocom/annotated/releases/tag/v0.2.0
"
    );
}

#[test]
fn a_release_with_nothing_under_unreleased_says_that_users_can_notice_no_change() {
    let repository = Repository::released("0.2.0");
    repository.commit_changing(
        "crates/annotated/src/lib.rs",
        "refactor(core): read the bounds in one place",
    );

    repository
        .write_next_release_on("2026-11-13")
        .expect("the release is written");

    assert_eq!(
        repository.read("CHANGELOG.md"),
        "\
# Changelog

## [Unreleased]

## [0.2.1] - 2026-11-13

No change that users can notice.

## [0.2.0] - 2026-11-12

[Unreleased]: https://github.com/pablocom/annotated/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/pablocom/annotated/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/pablocom/annotated/releases/tag/v0.2.0
"
    );
}

#[test]
fn a_feature_with_nothing_under_unreleased_is_not_released() {
    let repository = Repository::released("0.2.0");
    repository.commit_changing(
        "crates/annotated/src/lib.rs",
        "feat(core): take exclusive bounds in range",
    );

    let release = repository.write_next_release();

    assert_eq!(
        release,
        Err(
            "error: CHANGELOG.md lists nothing under Unreleased, and users can notice a commit since v0.2.0".to_owned()
        )
    );
}

#[test]
fn a_fix_with_nothing_under_unreleased_is_not_released() {
    let repository = Repository::released("0.2.0");
    repository.commit_changing(
        "crates/annotated/src/lib.rs",
        "fix(core): accept an address with a quoted local part",
    );

    let release = repository.write_next_release();

    assert_eq!(
        release,
        Err(
            "error: CHANGELOG.md lists nothing under Unreleased, and users can notice a commit since v0.2.0".to_owned()
        )
    );
}

#[test]
fn a_faster_path_with_nothing_under_unreleased_is_not_released() {
    let repository = Repository::released("0.2.0");
    repository.commit_changing(
        "crates/annotated/src/lib.rs",
        "perf(core): count the chars of a string once",
    );

    let release = repository.write_next_release();

    assert_eq!(
        release,
        Err(
            "error: CHANGELOG.md lists nothing under Unreleased, and users can notice a commit since v0.2.0".to_owned()
        )
    );
}

#[test]
fn a_breaking_change_with_nothing_under_unreleased_is_not_released() {
    let repository = Repository::released("0.2.0");
    repository.commit_changing(
        "crates/annotated/src/lib.rs",
        "refactor(core)!: call the issues found so far a report",
    );

    let release = repository.write_next_release();

    assert_eq!(
        release,
        Err(
            "error: CHANGELOG.md lists nothing under Unreleased, and users can notice a commit since v0.2.0".to_owned()
        )
    );
}

#[test]
fn a_changelog_without_an_unreleased_section_is_not_released() {
    let repository = Repository::released("0.2.0");
    repository.write("CHANGELOG.md", "# Changelog\n\n## [0.2.0] - 2026-11-12\n");
    repository.commit_changing(
        "crates/annotated/src/lib.rs",
        "refactor(core): read the bounds in one place",
    );

    let release = repository.write_next_release();

    assert_eq!(
        release,
        Err("error: CHANGELOG.md: expected a `## [Unreleased]` section".to_owned())
    );
}

#[test]
fn a_changelog_whose_unreleased_link_compares_another_version_is_not_released() {
    let repository = Repository::released("0.2.0");
    repository.write(
        "CHANGELOG.md",
        "\
# Changelog

## [Unreleased]

## [0.2.0] - 2026-11-12

[Unreleased]: https://github.com/pablocom/annotated/compare/v0.1.0...HEAD
",
    );
    repository.commit_changing(
        "crates/annotated/src/lib.rs",
        "refactor(core): read the bounds in one place",
    );

    let release = repository.write_next_release();

    assert_eq!(
        release,
        Err("error: CHANGELOG.md: expected a link `[Unreleased]: …/compare/v0.2.0...HEAD`".to_owned())
    );
}

#[test]
fn a_release_needs_its_day() {
    let repository = Repository::released("0.2.0");
    repository.commit_to_the_crates("fix(core): accept an address with a quoted local part");

    let release = repository.write_next_release_without_its_day();

    assert_eq!(
        release,
        Err("error: expected the day of the release, such as `annotated-release 2026-11-13`".to_owned())
    );
}

#[test]
fn a_day_written_in_another_order_is_refused() {
    let repository = Repository::released("0.2.0");
    repository.commit_to_the_crates("fix(core): accept an address with a quoted local part");

    let release = repository.write_next_release_on("13-11-2026");

    assert_eq!(
        release,
        Err("error: `13-11-2026` isn't a day written like 2026-11-13".to_owned())
    );
}

#[test]
fn a_changelog_without_the_section_of_the_last_release_is_not_released() {
    let repository = Repository::released("0.2.0");
    repository.write("CHANGELOG.md", "# Changelog\n\n## [Unreleased]\n");
    repository.commit_changing(
        "crates/annotated/src/lib.rs",
        "refactor(core): read the bounds in one place",
    );

    let release = repository.write_next_release();

    assert_eq!(
        release,
        Err("error: CHANGELOG.md: expected the section `## [0.2.0]` below Unreleased".to_owned())
    );
}
