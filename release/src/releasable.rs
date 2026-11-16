use std::borrow::Cow;
use std::{fmt, fs};

use crate::change::Change;
use crate::changelog::{self, Changelog};
use crate::day::Day;
use crate::git;
use crate::version::Version;

const CRATES: &str = "crates";
const ANNOTATED_CRATES: [&str; 2] = ["annotated", "annotated-derive"];
const REQUIREMENT_ON_ANNOTATED: &str = "annotated = { version = \"";

/// Crates released together: under one version, with one tag and one changelog.
pub struct Releasable {
    name: String,
    tag_prefix: String,
    manifest: String,
    changelog: String,
    crates: Vec<String>,
    code: Vec<String>,
    texts_naming_the_version: Vec<(String, String)>,
}

/// annotated as this run leaves it, which each crate released on its own requires.
pub struct Annotated {
    pub version: Version,
    pub has_a_breaking_release: bool,
}

/// A version to publish: its tag, the changelog holding its notes, and its crates.
pub struct Release {
    tag: String,
    changelog: String,
    crates: Vec<String>,
    is_breaking: bool,
}

impl Releasable {
    /// annotated's two crates, which share the version in the workspace manifest.
    pub fn annotated() -> Self {
        let texts_naming_the_version = [
            ("Cargo.toml", "version = \"{version}\""),
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
        Self {
            name: "annotated".to_owned(),
            tag_prefix: "v".to_owned(),
            manifest: "Cargo.toml".to_owned(),
            changelog: "CHANGELOG.md".to_owned(),
            crates: ANNOTATED_CRATES.map(str::to_owned).to_vec(),
            code: ["Cargo.toml", "crates/annotated", "crates/annotated-derive"]
                .map(str::to_owned)
                .to_vec(),
            texts_naming_the_version: texts_naming_the_version
                .map(|(path, text)| (path.to_owned(), text.to_owned()))
                .to_vec(),
        }
    }

    /// Every other crate in `crates/` that has been released. Its first version is published by hand, since crates.io's
    /// Trusted Publishing serves only a crate it already has.
    pub fn released_on_their_own() -> Result<Vec<Self>, String> {
        let mut released = Vec::new();
        for name in crates_other_than_annotated()? {
            let releasable = Self::on_its_own(&name);
            if git::has_a_tag_starting_with(&releasable.tag_prefix)? {
                released.push(releasable);
            }
        }
        Ok(released)
    }

    fn on_its_own(name: &str) -> Self {
        let directory = format!("{CRATES}/{name}");
        let manifest = format!("{directory}/Cargo.toml");
        Self {
            name: name.to_owned(),
            tag_prefix: format!("{name}-v"),
            changelog: format!("{directory}/CHANGELOG.md"),
            crates: vec![name.to_owned()],
            texts_naming_the_version: vec![
                // Its own line, since its requirement on annotated may name the same version.
                (manifest.clone(), "\nversion = \"{version}\"\n".to_owned()),
                (
                    "Cargo.lock".to_owned(),
                    format!("name = \"{name}\"\nversion = \"{{version}}\""),
                ),
            ],
            manifest,
            code: vec![directory],
        }
    }

    pub fn version(&self) -> Result<Version, String> {
        let manifest = read(&self.manifest)?;
        let version = manifest
            .lines()
            .find_map(|line| line.strip_prefix("version = \"")?.strip_suffix('"'))
            .ok_or_else(|| format!("{}: expected a line `version = \"…\"`", self.manifest))?;
        version.parse().map_err(|reason| format!("{}: {reason}", self.manifest))
    }

    /// Writes the next release, when commits since the last one change the crates' code, or when `annotated`, which
    /// the crates require, has a breaking release. A version written earlier and never tagged is released again, and
    /// nothing more is written.
    pub fn next_release(&self, day: &Day, annotated: Option<&Annotated>) -> Result<Option<Release>, String> {
        let released = self.version()?;
        let released_tag = self.tag(released);
        if !git::has_tag(&released_tag)? {
            return self.unpublished(released).map(Some);
        }
        let changes = ChangesSinceTheRelease {
            largest_commit: git::changes_since(&released_tag, &self.code)?.into_iter().max(),
            is_broken_by_annotated: annotated.is_some_and(|annotated| annotated.has_a_breaking_release),
            released_tag,
        };
        let Some(largest_change) = changes.largest() else {
            return Ok(None);
        };
        let next = released.after(largest_change);

        self.release_the_unreleased_changes(released, next, day, changes.why_users_notice(&self.name))?;
        self.write_the_version(released, next)?;
        if let Some(annotated) = annotated {
            self.require_annotated(annotated.version)?;
        }
        Ok(Some(self.release(next, largest_change == Change::Breaking)))
    }

    fn unpublished(&self, version: Version) -> Result<Release, String> {
        if !changelog::has_section_for(&read(&self.changelog)?, version) {
            return Err(format!(
                "{} {version} has no tag `{}`, and {} has no section for it",
                self.name,
                self.tag(version),
                self.changelog
            ));
        }
        Ok(self.release(version, false))
    }

    fn release_the_unreleased_changes(
        &self,
        released: Version,
        next: Version,
        day: &Day,
        why_users_notice: Option<String>,
    ) -> Result<(), String> {
        let text = read(&self.changelog)?;
        let changelog = Changelog::parse(&text, released, &self.tag_prefix)
            .map_err(|reason| format!("{}: {reason}", self.changelog))?;
        if let Some(why) = why_users_notice
            && !changelog.lists_unreleased_changes()
        {
            return Err(format!("{} lists nothing under Unreleased, and {why}", self.changelog));
        }
        let released_changes = changelog
            .with_unreleased_changes_released(next, day)
            .map_err(|reason| format!("{}: {reason}", self.changelog))?;
        write(&self.changelog, &released_changes)
    }

    fn write_the_version(&self, released: Version, next: Version) -> Result<(), String> {
        for (path, text) in &self.texts_naming_the_version {
            let naming = |version: Version| text.replace("{version}", &version.to_string());
            let contents = replacing_the_only(&read(path)?, &naming(released), &naming(next))
                .map_err(|reason| format!("{path}: {reason}"))?;
            write(path, &contents)?;
        }
        Ok(())
    }

    /// Makes each of the crate's requirements on annotated, as a dependency and as a dev-dependency, ask for
    /// `version` at least.
    fn require_annotated(&self, version: Version) -> Result<(), String> {
        let manifest = read(&self.manifest)?;
        if !manifest
            .lines()
            .any(|line| requiring_annotated(line, version).is_some())
        {
            return Err(format!(
                "{}: expected a line `{REQUIREMENT_ON_ANNOTATED}…\"`",
                self.manifest
            ));
        }
        let required: String = manifest
            .split_inclusive('\n')
            .map(|line| requiring_annotated(line, version).map_or(Cow::Borrowed(line), Cow::Owned))
            .collect();
        write(&self.manifest, &required)
    }

    fn tag(&self, version: Version) -> String {
        format!("{}{version}", self.tag_prefix)
    }

    fn release(&self, version: Version, is_breaking: bool) -> Release {
        Release {
            tag: self.tag(version),
            changelog: self.changelog.clone(),
            crates: self.crates.clone(),
            is_breaking,
        }
    }
}

/// What changed for crates since their last release: their own commits, and annotated's release.
struct ChangesSinceTheRelease {
    released_tag: String,
    largest_commit: Option<Change>,
    is_broken_by_annotated: bool,
}

impl ChangesSinceTheRelease {
    fn largest(&self) -> Option<Change> {
        let required_by_annotated = self.is_broken_by_annotated.then_some(Change::Breaking);
        self.largest_commit.max(required_by_annotated)
    }

    /// Why users of the crates named `name` can notice their next release, which their changelog then has to say.
    fn why_users_notice(&self, name: &str) -> Option<String> {
        if self.largest_commit.is_some_and(Change::is_noticed_by_users) {
            return Some(format!("users can notice a commit since {}", self.released_tag));
        }
        if self.is_broken_by_annotated {
            return Some(format!("annotated's breaking release changes what {name} requires"));
        }
        None
    }
}

impl Release {
    pub fn is_breaking(&self) -> bool {
        self.is_breaking
    }
}

/// The line CI reads to publish the release: its tag, its changelog and its crates, separated by spaces.
impl fmt::Display for Release {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} {} {}", self.tag, self.changelog, self.crates.join(" "))
    }
}

/// `line` asking for `version` of annotated at least, when it is a requirement on annotated.
fn requiring_annotated(line: &str, version: Version) -> Option<String> {
    let (_, after_the_version) = line.strip_prefix(REQUIREMENT_ON_ANNOTATED)?.split_once('"')?;
    Some(format!("{REQUIREMENT_ON_ANNOTATED}{version}\"{after_the_version}"))
}

fn crates_other_than_annotated() -> Result<Vec<String>, String> {
    let entries = fs::read_dir(CRATES).map_err(|error| format!("{CRATES} can't be read: {error}"))?;
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("{CRATES} can't be read: {error}"))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.path().is_dir() && !ANNOTATED_CRATES.contains(&name.as_str()) {
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
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
