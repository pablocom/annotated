use std::process::Command;

use crate::change::Change;
use crate::version::Version;

const CODE_OF_THE_CRATES: [&str; 2] = ["crates", "Cargo.toml"];

pub fn changes_to_the_crates_since(released: Version) -> Result<Vec<Change>, String> {
    let since_the_release = format!("v{released}..HEAD");
    let log = Command::new("git")
        .args(["log", "--format=%B%x00", &since_the_release, "--"])
        .args(CODE_OF_THE_CRATES)
        .output()
        .map_err(|error| format!("git can't run: {error}"))?;
    if !log.status.success() {
        let reason = String::from_utf8_lossy(&log.stderr);
        return Err(format!(
            "git can't list the commits since v{released}: {}",
            reason.trim()
        ));
    }

    let messages = String::from_utf8_lossy(&log.stdout);
    let changes = messages
        .split('\0')
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .filter_map(Change::of_commit);
    Ok(changes.collect())
}
