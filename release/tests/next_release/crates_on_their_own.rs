//! Every crate in `crates/` other than annotated's two has a version of its own, and is released on its own.

use crate::support::Repository;

#[test]
fn a_commit_to_a_crate_released_on_its_own_releases_only_that_crate() {
    let repository = Repository::released("0.2.0");
    repository.release_on_its_own("annotated-utoipa", "0.1.0");
    repository.commit_to_the_crate("annotated-utoipa", "fix(utoipa): find the schema of a generic type");

    let release = repository.write_next_release();

    assert_eq!(
        release,
        Ok("annotated-utoipa-v0.1.1 crates/annotated-utoipa/CHANGELOG.md annotated-utoipa".to_owned())
    );
}

#[test]
fn a_commit_to_annotated_leaves_the_crates_released_on_their_own_alone() {
    let repository = Repository::released("0.2.0");
    repository.release_on_its_own("annotated-utoipa", "0.1.0");
    repository.commit_to_the_crates("fix(core): accept an address with a quoted local part");

    let release = repository.write_next_release();

    assert_eq!(release, Ok("v0.2.1 CHANGELOG.md annotated annotated-derive".to_owned()));
}

#[test]
fn each_crate_with_commits_since_its_last_release_is_released() {
    let repository = Repository::released("0.2.0");
    repository.release_on_its_own("annotated-axum", "0.4.2");
    repository.release_on_its_own("annotated-utoipa", "0.1.0");
    repository.commit_to_the_crates("fix(core): accept an address with a quoted local part");
    repository.commit_to_the_crate("annotated-axum", "feat(axum): read a form");
    repository.commit_to_the_crate("annotated-utoipa", "fix(utoipa): find the schema of a generic type");

    let release = repository.write_next_release();

    assert_eq!(
        release,
        Ok("\
v0.2.1 CHANGELOG.md annotated annotated-derive
annotated-axum-v0.4.3 crates/annotated-axum/CHANGELOG.md annotated-axum
annotated-utoipa-v0.1.1 crates/annotated-utoipa/CHANGELOG.md annotated-utoipa"
            .to_owned())
    );
}

#[test]
fn a_breaking_release_of_annotated_is_a_breaking_release_of_every_crate_released_on_its_own() {
    let repository = Repository::released("0.2.0");
    repository.release_on_its_own("annotated-utoipa", "0.1.0");
    repository.commit_to_the_crates("refactor(core)!: call the issues found so far a report");
    repository.note_unreleased_changes(
        "crates/annotated-utoipa/CHANGELOG.md",
        "### Changed\n\n- Requires annotated 0.3.",
    );
    repository.commit_changing(
        "crates/annotated-utoipa/src/lib.rs",
        "docs(utoipa): say that annotated 0.3 is required",
    );

    let release = repository.write_next_release();

    assert_eq!(
        release,
        Ok("\
v0.3.0 CHANGELOG.md annotated annotated-derive
annotated-utoipa-v0.2.0 crates/annotated-utoipa/CHANGELOG.md annotated-utoipa"
            .to_owned())
    );
}

#[test]
fn a_breaking_release_of_annotated_needs_a_line_in_the_changelog_of_each_crate_released_on_its_own() {
    let repository = Repository::released("0.2.0");
    repository.release_on_its_own("annotated-utoipa", "0.1.0");
    repository.commit_to_the_crates("refactor(core)!: call the issues found so far a report");

    let release = repository.write_next_release();

    assert_eq!(
        release,
        Err("error: crates/annotated-utoipa/CHANGELOG.md lists nothing under Unreleased, and annotated's breaking release changes what annotated-utoipa requires".to_owned())
    );
}

#[test]
fn a_crate_released_on_its_own_requires_the_version_of_annotated_released_with_it() {
    let repository = Repository::released("0.2.0");
    repository.release_on_its_own("annotated-utoipa", "0.1.0");
    repository.commit_to_the_crates("feat(core): take exclusive bounds in range");
    repository.commit_to_the_crate("annotated-utoipa", "feat(utoipa): write exclusive bounds");

    repository.write_next_release().expect("the release is written");

    assert_eq!(
        repository.read("crates/annotated-utoipa/Cargo.toml"),
        "[package]\nname = \"annotated-utoipa\"\nversion = \"0.1.1\"\n\n[dependencies]\nannotated = { version = \"0.2.1\", path = \"../annotated\" }\n"
    );
}

#[test]
fn the_manifest_of_a_crate_released_on_its_own_takes_its_version() {
    let repository = Repository::released("0.2.0");
    repository.release_on_its_own("annotated-utoipa", "0.2.0");
    repository.commit_to_the_crate("annotated-utoipa", "fix(utoipa): find the schema of a generic type");

    repository.write_next_release().expect("the release is written");

    assert_eq!(
        repository.read("crates/annotated-utoipa/Cargo.toml"),
        "[package]\nname = \"annotated-utoipa\"\nversion = \"0.2.1\"\n\n[dependencies]\nannotated = { version = \"0.2.0\", path = \"../annotated\" }\n"
    );
}

#[test]
fn the_lock_file_takes_the_version_of_a_crate_released_on_its_own() {
    let repository = Repository::released("0.2.0");
    repository.release_on_its_own("annotated-utoipa", "0.1.0");
    repository.commit_to_the_crate("annotated-utoipa", "fix(utoipa): find the schema of a generic type");

    repository.write_next_release().expect("the release is written");

    assert_eq!(
        repository.read("Cargo.lock"),
        "\
[[package]]
name = \"annotated\"
version = \"0.2.0\"

[[package]]
name = \"annotated-derive\"
version = \"0.2.0\"

[[package]]
name = \"annotated-utoipa\"
version = \"0.1.1\"
"
    );
}

#[test]
fn the_changelog_of_a_crate_released_on_its_own_links_its_own_tags() {
    let repository = Repository::released("0.2.0");
    repository.release_on_its_own("annotated-utoipa", "0.1.0");
    repository.note_unreleased_changes(
        "crates/annotated-utoipa/CHANGELOG.md",
        "### Fixed\n\n- The schema of a generic type is found.",
    );
    repository.commit_changing(
        "crates/annotated-utoipa/src/lib.rs",
        "fix(utoipa): find the schema of a generic type",
    );

    repository
        .write_next_release_on("2026-11-16")
        .expect("the release is written");

    assert_eq!(
        repository.read("crates/annotated-utoipa/CHANGELOG.md"),
        "\
# Changelog

## [Unreleased]

## [0.1.1] - 2026-11-16

### Fixed

- The schema of a generic type is found.

## [0.1.0] - 2026-11-12

[Unreleased]: https://github.com/pablocom/annotated/compare/annotated-utoipa-v0.1.1...HEAD
[0.1.1]: https://github.com/pablocom/annotated/compare/annotated-utoipa-v0.1.0...annotated-utoipa-v0.1.1
[0.1.0]: https://github.com/pablocom/annotated/releases/tag/annotated-utoipa-v0.1.0
"
    );
}

#[test]
fn a_crate_never_released_is_left_alone() {
    let repository = Repository::released("0.2.0");
    repository.add_on_its_own("annotated-utoipa", "0.1.0");
    repository.commit_to_the_crate(
        "annotated-utoipa",
        "feat(utoipa): write the rules of a type into its schema",
    );

    let release = repository.write_next_release();

    assert_eq!(release, Ok(String::new()));
}

#[test]
fn a_crate_whose_version_has_no_tag_and_no_section_in_its_changelog_is_not_released() {
    let repository = Repository::released("0.2.0");
    repository.release_on_its_own("annotated-utoipa", "0.1.0");
    repository.write(
        "crates/annotated-utoipa/Cargo.toml",
        "[package]\nname = \"annotated-utoipa\"\nversion = \"0.2.0\"\n\n[dependencies]\nannotated = { version = \"0.2.0\", path = \"../annotated\" }\n",
    );
    repository.commit_to_the_crate("annotated-utoipa", "fix(utoipa): find the schema of a generic type");

    let release = repository.write_next_release();

    assert_eq!(
        release,
        Err("error: annotated-utoipa 0.2.0 has no tag `annotated-utoipa-v0.2.0`, and crates/annotated-utoipa/CHANGELOG.md has no section for it".to_owned())
    );
}

#[test]
fn every_requirement_of_a_crate_released_on_its_own_on_annotated_takes_its_version() {
    let repository = Repository::released("0.2.0");
    repository.release_on_its_own("annotated-utoipa", "0.1.0");
    repository.write(
        "crates/annotated-utoipa/Cargo.toml",
        "[package]\nname = \"annotated-utoipa\"\nversion = \"0.1.0\"\n\n[dependencies]\nannotated = { version = \"0.2.0\", path = \"../annotated\" }\n\n[dev-dependencies]\nannotated = { version = \"0.2.0\", path = \"../annotated\", features = [\"regex\"] }\n",
    );
    repository.commit_to_the_crates("fix(core): accept an address with a quoted local part");
    repository.commit_to_the_crate("annotated-utoipa", "fix(utoipa): find the schema of a generic type");

    repository.write_next_release().expect("the release is written");

    assert_eq!(
        repository.read("crates/annotated-utoipa/Cargo.toml"),
        "[package]\nname = \"annotated-utoipa\"\nversion = \"0.1.1\"\n\n[dependencies]\nannotated = { version = \"0.2.1\", path = \"../annotated\" }\n\n[dev-dependencies]\nannotated = { version = \"0.2.1\", path = \"../annotated\", features = [\"regex\"] }\n"
    );
}

#[test]
fn a_crate_released_on_its_own_that_names_no_version_of_annotated_is_not_released() {
    let repository = Repository::released("0.2.0");
    repository.release_on_its_own("annotated-utoipa", "0.1.0");
    repository.write(
        "crates/annotated-utoipa/Cargo.toml",
        "[package]\nname = \"annotated-utoipa\"\nversion = \"0.1.0\"\n\n[dependencies]\nannotated = { path = \"../annotated\" }\n",
    );
    repository.commit_to_the_crate("annotated-utoipa", "fix(utoipa): find the schema of a generic type");

    let release = repository.write_next_release();

    assert_eq!(
        release,
        Err("error: crates/annotated-utoipa/Cargo.toml: expected a line `annotated = { version = \"…\"`".to_owned())
    );
}
