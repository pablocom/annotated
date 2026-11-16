//! Writes the next release of annotated, and of each crate released on its own, into the repository it runs in.
//!
//! CI runs it at the root of the repository once a push to `main` passes its checks, with the day of the release:
//!
//! ```sh
//! cargo run --package annotated-release -- 2026-11-13
//! ```
//!
//! annotated's two crates share the version in the workspace manifest, and their releases are tagged `v1.2.3`. Every
//! other crate in `crates/` has a version of its own in its manifest, and its releases are tagged with its name:
//! `annotated-utoipa-v1.2.3`. For each of them, the commits since its last release that change its code decide the
//! next version, as Semantic Versioning asks.
//!
//! It prints a line for each release to publish: its tag, the changelog holding its notes, and its crates. When it
//! writes no release and none is left unpublished, it prints nothing.

mod change;
mod changelog;
mod day;
mod git;
mod releasable;
mod version;

use std::env;
use std::process::ExitCode;

use day::Day;
use releasable::{Annotated, Releasable, Release};

fn main() -> ExitCode {
    match write_next_releases() {
        Ok(releases) => {
            for release in releases {
                println!("{release}");
            }
        }
        Err(reason) => {
            eprintln!("error: {reason}");
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

fn write_next_releases() -> Result<Vec<Release>, String> {
    let day = day_of_the_release()?;
    let annotated = Releasable::annotated();
    let annotated_release = annotated.next_release(&day, None)?;
    let annotated_as_released = Annotated {
        version: annotated.version()?,
        has_a_breaking_release: annotated_release.as_ref().is_some_and(Release::is_breaking),
    };

    let mut releases = Vec::from_iter(annotated_release);
    for releasable in Releasable::released_on_their_own()? {
        releases.extend(releasable.next_release(&day, Some(&annotated_as_released))?);
    }
    Ok(releases)
}

fn day_of_the_release() -> Result<Day, String> {
    let day = env::args()
        .nth(1)
        .ok_or("expected the day of the release, such as `annotated-release 2026-11-13`")?;
    day.parse()
}
