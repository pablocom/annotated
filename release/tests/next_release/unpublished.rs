//! A release that was written but never published, so never tagged, is published before anything more is written
//! for its crates.

use crate::support::Repository;

#[test]
fn a_release_written_but_never_tagged_is_named_again() {
    let repository = Repository::released("0.2.0");
    repository.commit_to_the_crates("fix(core): accept an address with a quoted local part");
    repository.write_next_release().expect("the release is written");
    repository.commit_the_release_without_publishing_it();
    repository.commit_to_the_crates("feat(core): take exclusive bounds in range");

    let release = repository.write_next_release();

    assert_eq!(release, Ok("v0.2.1 CHANGELOG.md annotated annotated-derive".to_owned()));
}

#[test]
fn nothing_more_is_written_for_a_release_never_tagged() {
    let repository = Repository::released("0.2.0");
    repository.commit_to_the_crates("fix(core): accept an address with a quoted local part");
    repository.write_next_release().expect("the release is written");
    repository.commit_the_release_without_publishing_it();
    repository.commit_to_the_crates("feat(core): take exclusive bounds in range");

    repository.write_next_release().expect("the release is named again");

    assert_eq!(
        repository.read("Cargo.toml"),
        "[workspace.package]\nversion = \"0.2.1\"\n"
    );
}

#[test]
fn a_crate_released_on_its_own_is_released_while_annotated_waits_to_be_published() {
    let repository = Repository::released("0.2.0");
    repository.release_on_its_own("annotated-utoipa", "0.1.0");
    repository.commit_to_the_crates("fix(core): accept an address with a quoted local part");
    repository.write_next_release().expect("the release is written");
    repository.commit_the_release_without_publishing_it();
    repository.commit_to_the_crate("annotated-utoipa", "fix(utoipa): find the schema of a generic type");

    let release = repository.write_next_release();

    assert_eq!(
        release,
        Ok("\
v0.2.1 CHANGELOG.md annotated annotated-derive
annotated-utoipa-v0.1.1 crates/annotated-utoipa/CHANGELOG.md annotated-utoipa"
            .to_owned())
    );
}
