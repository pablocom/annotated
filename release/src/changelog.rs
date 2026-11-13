use crate::day::Day;
use crate::version::Version;

const UNRELEASED_HEADING: &str = "## [Unreleased]\n";
const NO_CHANGE_TO_NOTICE: &str = "\nNo change that users can notice.\n\n";

pub struct Changelog<'a> {
    introduction: &'a str,
    unreleased_changes: &'a str,
    released_versions: &'a str,
    released: Version,
}

impl<'a> Changelog<'a> {
    pub fn parse(text: &'a str, released: Version) -> Result<Self, String> {
        let (introduction, sections) = text
            .split_once(UNRELEASED_HEADING)
            .ok_or("expected a `## [Unreleased]` section")?;
        let heading_of_the_last_release = format!("## [{released}]");
        let end_of_unreleased = sections
            .find(&format!("\n{heading_of_the_last_release}"))
            .ok_or_else(|| format!("expected the section `{heading_of_the_last_release}` below Unreleased"))?
            + 1;
        let (unreleased_changes, released_versions) = sections.split_at(end_of_unreleased);
        Ok(Self {
            introduction,
            unreleased_changes,
            released_versions,
            released,
        })
    }

    pub fn lists_unreleased_changes(&self) -> bool {
        !self.unreleased_changes.trim().is_empty()
    }

    pub fn with_unreleased_changes_released(&self, version: Version, day: &Day) -> Result<String, String> {
        let introduction = self.introduction;
        let changes = if self.lists_unreleased_changes() {
            self.unreleased_changes
        } else {
            NO_CHANGE_TO_NOTICE
        };
        let released_versions = self.released_versions_linking(version)?;
        Ok(format!(
            "{introduction}{UNRELEASED_HEADING}\n## [{version}] - {day}\n{changes}{released_versions}"
        ))
    }

    fn released_versions_linking(&self, version: Version) -> Result<String, String> {
        let released = self.released;
        let compared_with_head = format!("/compare/v{released}...HEAD");
        let repository = self
            .released_versions
            .lines()
            .find_map(|line| line.strip_prefix("[Unreleased]: ")?.strip_suffix(&compared_with_head))
            .ok_or_else(|| format!("expected a link `[Unreleased]: …{compared_with_head}`"))?;
        let unreleased_link = format!("[Unreleased]: {repository}{compared_with_head}");
        let links = format!(
            "[Unreleased]: {repository}/compare/v{version}...HEAD\n[{version}]: {repository}/compare/v{released}...v{version}"
        );
        Ok(self.released_versions.replacen(&unreleased_link, &links, 1))
    }
}
