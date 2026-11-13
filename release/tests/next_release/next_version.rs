//! The commits since the last release decide the next version, as Semantic Versioning asks.

use crate::support::Repository;

#[test]
fn before_1_0_a_fix_releases_the_next_patch_version() {
    let repository = Repository::released("0.2.0");
    repository.commit_to_the_crates("fix(core): accept an address with a quoted local part");

    let release = repository.write_next_release();

    assert_eq!(release, Ok("0.2.1".to_owned()));
}

#[test]
fn before_1_0_a_feature_releases_the_next_patch_version() {
    let repository = Repository::released("0.2.0");
    repository.commit_to_the_crates("feat(core): take exclusive bounds in range");

    let release = repository.write_next_release();

    assert_eq!(release, Ok("0.2.1".to_owned()));
}

#[test]
fn before_1_0_a_breaking_change_releases_the_next_minor_version() {
    let repository = Repository::released("0.2.4");
    repository.commit_to_the_crates("refactor(core)!: call the issues found so far a report");

    let release = repository.write_next_release();

    assert_eq!(release, Ok("0.3.0".to_owned()));
}

#[test]
fn from_1_0_a_fix_releases_the_next_patch_version() {
    let repository = Repository::released("1.4.2");
    repository.commit_to_the_crates("fix(core): accept an address with a quoted local part");

    let release = repository.write_next_release();

    assert_eq!(release, Ok("1.4.3".to_owned()));
}

#[test]
fn from_1_0_a_feature_releases_the_next_minor_version() {
    let repository = Repository::released("1.4.2");
    repository.commit_to_the_crates("feat(core): take exclusive bounds in range");

    let release = repository.write_next_release();

    assert_eq!(release, Ok("1.5.0".to_owned()));
}

#[test]
fn from_1_0_a_breaking_change_releases_the_next_major_version() {
    let repository = Repository::released("1.4.2");
    repository.commit_to_the_crates("refactor(core)!: call the issues found so far a report");

    let release = repository.write_next_release();

    assert_eq!(release, Ok("2.0.0".to_owned()));
}

#[test]
fn a_breaking_change_footer_makes_its_commit_a_breaking_change() {
    let repository = Repository::released("1.4.2");
    repository.commit_to_the_crates(
        "fix(derive): report a flattened field where its keys are\n\nBREAKING CHANGE: its issues moved to the struct.",
    );

    let release = repository.write_next_release();

    assert_eq!(release, Ok("2.0.0".to_owned()));
}

#[test]
fn the_largest_change_since_the_last_release_decides_the_version() {
    let repository = Repository::released("1.4.2");
    repository.commit_to_the_crates("fix(core): accept an address with a quoted local part");
    repository.commit_to_the_crates("feat(core): take exclusive bounds in range");
    repository.commit_to_the_crates("fix(derive): name the field in the error");

    let release = repository.write_next_release();

    assert_eq!(release, Ok("1.5.0".to_owned()));
}

#[test]
fn a_commit_of_any_other_type_releases_the_next_patch_version() {
    let repository = Repository::released("1.4.2");
    repository.commit_to_the_crates("refactor(core): read the bounds in one place");

    let release = repository.write_next_release();

    assert_eq!(release, Ok("1.4.3".to_owned()));
}
