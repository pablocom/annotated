//! A commit is released when it changes the crates' code: `crates/` or the workspace manifest, unless it is
//! documentation.

use crate::support::Repository;

#[test]
fn a_commit_that_leaves_the_crates_alone_releases_nothing() {
    let repository = Repository::released("0.2.0");
    repository.commit_changing(
        "benchmarks/src/lib.rs",
        "feat(benchmarks): measure a struct with a hundred fields",
    );

    let release = repository.write_next_release();

    assert_eq!(release, Ok(String::new()));
}

#[test]
fn a_documentation_commit_to_the_crates_releases_nothing() {
    let repository = Repository::released("0.2.0");
    repository.commit_changing(
        "crates/annotated/src/lib.rs",
        "docs(core): say what a rule promises in one sentence",
    );

    let release = repository.write_next_release();

    assert_eq!(release, Ok(String::new()));
}

#[test]
fn a_commit_that_changes_the_workspace_manifest_is_released() {
    let repository = Repository::released("0.2.0");
    repository.commit_changing("Cargo.toml", "build(repo): raise the minimum supported Rust to 1.99");

    let release = repository.write_next_release();

    assert_eq!(release, Ok("0.2.1".to_owned()));
}

#[test]
fn documentation_is_released_with_the_next_change_to_the_code() {
    let repository = Repository::released("0.2.0");
    repository.commit_changing(
        "crates/annotated/src/lib.rs",
        "docs(core): say what a rule promises in one sentence",
    );
    repository.commit_changing(
        "crates/annotated/src/lib.rs",
        "refactor(core): read the bounds in one place",
    );

    let release = repository.write_next_release();

    assert_eq!(release, Ok("0.2.1".to_owned()));
}
