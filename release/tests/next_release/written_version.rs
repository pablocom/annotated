//! A release writes its version everywhere the repository names the last one.

use crate::support::Repository;

#[test]
fn the_workspace_manifest_takes_the_version() {
    let repository = Repository::released("0.2.0");
    repository.commit_to_the_crates("fix(core): accept an address with a quoted local part");

    repository.write_next_release().expect("the release is written");

    assert_eq!(
        repository.read("Cargo.toml"),
        "[workspace.package]\nversion = \"0.2.1\"\n"
    );
}

#[test]
fn the_requirement_on_the_derive_crate_takes_the_version() {
    let repository = Repository::released("0.2.0");
    repository.commit_to_the_crates("fix(core): accept an address with a quoted local part");

    repository.write_next_release().expect("the release is written");

    assert_eq!(
        repository.read("crates/annotated/Cargo.toml"),
        "[dependencies]\nannotated-derive = { version = \"=0.2.1\", path = \"../annotated-derive\" }\n"
    );
}

#[test]
fn the_lock_file_takes_the_version_of_both_crates() {
    let repository = Repository::released("0.2.0");
    repository.commit_to_the_crates("fix(core): accept an address with a quoted local part");

    repository.write_next_release().expect("the release is written");

    assert_eq!(
        repository.read("Cargo.lock"),
        "\
[[package]]
name = \"annotated\"
version = \"0.2.1\"

[[package]]
name = \"annotated-derive\"
version = \"0.2.1\"
"
    );
}

#[test]
fn the_lock_file_of_the_benchmarks_takes_the_version_of_both_crates() {
    let repository = Repository::released("0.2.0");
    repository.commit_to_the_crates("fix(core): accept an address with a quoted local part");

    repository.write_next_release().expect("the release is written");

    assert_eq!(
        repository.read("benchmarks/Cargo.lock"),
        "\
[[package]]
name = \"annotated\"
version = \"0.2.1\"

[[package]]
name = \"annotated-derive\"
version = \"0.2.1\"
"
    );
}

#[test]
fn a_requirement_on_another_version_of_the_derive_crate_is_not_released() {
    let repository = Repository::released("0.2.0");
    repository.write(
        "crates/annotated/Cargo.toml",
        "[dependencies]\nannotated-derive = { version = \"=0.1.0\", path = \"../annotated-derive\" }\n",
    );
    repository.commit_to_the_crates("fix(core): accept an address with a quoted local part");

    let release = repository.write_next_release();

    assert_eq!(
        release,
        Err("error: crates/annotated/Cargo.toml: expected `annotated-derive = { version = \"=0.2.0\"` once".to_owned())
    );
}

#[test]
fn a_workspace_manifest_without_a_version_is_not_released() {
    let repository = Repository::released("0.2.0");
    repository.write("Cargo.toml", "[workspace]\n");
    repository.commit_to_the_crates("fix(core): accept an address with a quoted local part");

    let release = repository.write_next_release();

    assert_eq!(
        release,
        Err("error: Cargo.toml: expected a line `version = \"…\"`".to_owned())
    );
}

#[test]
fn a_version_that_is_not_three_numbers_is_not_released() {
    let repository = Repository::released("1.0.0-rc.1");
    repository.commit_to_the_crates("fix(core): accept an address with a quoted local part");

    let release = repository.write_next_release();

    assert_eq!(
        release,
        Err("error: Cargo.toml: `1.0.0-rc.1` isn't a version written like 1.2.3".to_owned())
    );
}

#[test]
fn a_workspace_manifest_naming_the_version_twice_is_not_released() {
    let repository = Repository::released("0.2.0");
    repository.write(
        "Cargo.toml",
        "[workspace.package]\nversion = \"0.2.0\"\n\n[workspace.dependencies]\nserde = { version = \"0.2.0\" }\n",
    );
    repository.commit_to_the_crates("fix(core): accept an address with a quoted local part");

    let release = repository.write_next_release();

    assert_eq!(
        release,
        Err("error: Cargo.toml: expected `version = \"0.2.0\"` once".to_owned())
    );
}
