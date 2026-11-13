//! Writes the next release of annotated into the repository it runs in, and prints its version.
//!
//! CI runs it at the root of the repository once a push to `main` passes its checks, with the day of the release:
//!
//! ```sh
//! cargo run --package annotated-release -- 2026-11-13
//! ```
//!
//! The commits since the last release that change the crates' code decide the version, as Semantic Versioning
//! asks. When there are none, it writes nothing and prints nothing.

mod change;
mod changelog;
mod commits;
mod day;
mod version;

use std::process::ExitCode;
use std::{env, fs};

use change::Change;
use changelog::Changelog;
use day::Day;
use version::Version;

const WORKSPACE_MANIFEST: &str = "Cargo.toml";
const CHANGELOG: &str = "CHANGELOG.md";

const TEXTS_NAMING_THE_VERSION: [(&str, &str); 6] = [
    (WORKSPACE_MANIFEST, "version = \"{version}\""),
    (
        "crates/annotated/Cargo.toml",
        "annotated-derive = { version = \"={version}\"",
    ),
    ("Cargo.lock", "name = \"annotated\"\nversion = \"{version}\""),
    ("Cargo.lock", "name = \"annotated-derive\"\nversion = \"{version}\""),
    ("benchmarks/Cargo.lock", "name = \"annotated\"\nversion = \"{version}\""),
    (
        "benchmarks/Cargo.lock",
        "name = \"annotated-derive\"\nversion = \"{version}\"",
    ),
];

fn main() -> ExitCode {
    match write_next_release() {
        Ok(Some(version)) => println!("{version}"),
        Ok(None) => {}
        Err(reason) => {
            eprintln!("error: {reason}");
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

fn write_next_release() -> Result<Option<Version>, String> {
    let day = day_of_the_release()?;
    let released = released_version()?;
    let Some(largest_change) = commits::changes_to_the_crates_since(released)?.into_iter().max() else {
        return Ok(None);
    };
    let next = released.after(largest_change);

    release_the_unreleased_changes(next, &day, released, largest_change)?;
    write_the_version(next, released)?;
    Ok(Some(next))
}

fn day_of_the_release() -> Result<Day, String> {
    let day = env::args()
        .nth(1)
        .ok_or("expected the day of the release, such as `annotated-release 2026-11-13`")?;
    day.parse()
}

fn released_version() -> Result<Version, String> {
    let manifest = read(WORKSPACE_MANIFEST)?;
    let version = manifest
        .lines()
        .find_map(|line| line.strip_prefix("version = \"")?.strip_suffix('"'))
        .ok_or_else(|| format!("{WORKSPACE_MANIFEST}: expected a line `version = \"…\"`"))?;
    version
        .parse()
        .map_err(|reason| format!("{WORKSPACE_MANIFEST}: {reason}"))
}

fn release_the_unreleased_changes(
    version: Version,
    day: &Day,
    released: Version,
    largest_change: Change,
) -> Result<(), String> {
    let text = read(CHANGELOG)?;
    let changelog = Changelog::parse(&text, released).map_err(|reason| format!("{CHANGELOG}: {reason}"))?;
    if largest_change.is_noticed_by_users() && !changelog.lists_unreleased_changes() {
        return Err(format!(
            "{CHANGELOG} lists nothing under Unreleased, and users can notice a commit since v{released}"
        ));
    }
    let released_changes = changelog
        .with_unreleased_changes_released(version, day)
        .map_err(|reason| format!("{CHANGELOG}: {reason}"))?;
    write(CHANGELOG, &released_changes)
}

fn write_the_version(version: Version, released: Version) -> Result<(), String> {
    for (path, text) in TEXTS_NAMING_THE_VERSION {
        let naming = |version: Version| text.replace("{version}", &version.to_string());
        let contents = replacing_the_only(&read(path)?, &naming(released), &naming(version))
            .map_err(|reason| format!("{path}: {reason}"))?;
        write(path, &contents)?;
    }
    Ok(())
}

fn replacing_the_only(contents: &str, text: &str, replacement: &str) -> Result<String, String> {
    if contents.matches(text).count() != 1 {
        return Err(format!("expected `{text}` once"));
    }
    Ok(contents.replacen(text, replacement, 1))
}

fn read(path: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("{path} can't be read: {error}"))
}

fn write(path: &str, contents: &str) -> Result<(), String> {
    fs::write(path, contents).map_err(|error| format!("{path} can't be written: {error}"))
}
