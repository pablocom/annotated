use std::process::Command;

use crate::change::Change;

/// The change of each commit since `tag` that touches `code`, which holds paths as git reads them.
pub fn changes_since(tag: &str, code: &[String]) -> Result<Vec<Change>, String> {
    let since_the_tag = format!("{tag}..HEAD");
    let messages = git(&["log", "--format=%B%x00", &since_the_tag, "--"], code)
        .map_err(|reason| format!("git can't list the commits since {tag}: {reason}"))?;
    let changes = messages
        .split('\0')
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .filter_map(Change::of_commit);
    Ok(changes.collect())
}

pub fn has_tag(tag: &str) -> Result<bool, String> {
    let tags = git(&["tag", "--list", tag], &[]).map_err(|reason| format!("git can't list the tags: {reason}"))?;
    Ok(!tags.trim().is_empty())
}

pub fn has_a_tag_starting_with(prefix: &str) -> Result<bool, String> {
    has_tag(&format!("{prefix}*"))
}

fn git(arguments: &[&str], paths: &[String]) -> Result<String, String> {
    let output = Command::new("git")
        .args(arguments)
        .args(paths)
        .output()
        .map_err(|error| format!("git can't run: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
