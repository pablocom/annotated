use std::fs;
use std::process::Command;

use tempfile::TempDir;

const CODE_OF_THE_CRATES: &str = "crates/annotated/src/lib.rs";
const DAY_OF_THE_RELEASE: &str = "2026-11-13";

pub struct Repository {
    directory: TempDir,
}

impl Repository {
    pub fn released(version: &str) -> Self {
        let repository = Self {
            directory: TempDir::new().expect("a temporary directory is created"),
        };
        repository.git(&["init", "--quiet", "--initial-branch=main"]);
        repository.write("Cargo.toml", &format!("[workspace.package]\nversion = \"{version}\"\n"));
        repository.write(
            "crates/annotated/Cargo.toml",
            &format!(
                "[dependencies]\nannotated-derive = {{ version = \"={version}\", path = \"../annotated-derive\" }}\n"
            ),
        );
        repository.write(CODE_OF_THE_CRATES, "");
        repository.write("Cargo.lock", &lock_file(version));
        repository.write("benchmarks/Cargo.lock", &lock_file(version));
        repository.write("CHANGELOG.md", &changelog(version));
        repository.commit(&format!("chore(repo): release {version}"));
        repository.git(&["tag", &format!("v{version}")]);
        repository
    }

    /// A commit to the crates' code that says what users can notice under Unreleased, as every such commit has to.
    pub fn commit_to_the_crates(&self, message: &str) {
        self.commit_noting("### Changed\n\n- A change that users can notice.", message);
    }

    pub fn commit_noting(&self, unreleased_changes: &str, message: &str) {
        let changelog = self.read("CHANGELOG.md");
        let noted = changelog.replacen(
            "## [Unreleased]\n",
            &format!("## [Unreleased]\n\n{unreleased_changes}\n"),
            1,
        );
        self.write("CHANGELOG.md", &noted);
        self.commit_changing(CODE_OF_THE_CRATES, message);
    }

    pub fn commit_changing(&self, path: &str, message: &str) {
        let grown = format!(
            "{}\n",
            fs::read_to_string(self.directory.path().join(path)).unwrap_or_default()
        );
        self.write(path, &grown);
        self.commit(message);
    }

    pub fn write_next_release(&self) -> Result<String, String> {
        self.run_annotated_release(&[DAY_OF_THE_RELEASE])
    }

    pub fn write_next_release_on(&self, day: &str) -> Result<String, String> {
        self.run_annotated_release(&[day])
    }

    pub fn write_next_release_without_its_day(&self) -> Result<String, String> {
        self.run_annotated_release(&[])
    }

    pub fn read(&self, path: &str) -> String {
        fs::read_to_string(self.directory.path().join(path)).expect("the file is read")
    }

    pub fn write(&self, path: &str, contents: &str) {
        let file = self.directory.path().join(path);
        fs::create_dir_all(file.parent().expect("the file is in a directory")).expect("its directory is created");
        fs::write(file, contents).expect("the file is written");
    }

    fn commit(&self, message: &str) {
        self.git(&["add", "--all"]);
        self.git(&["commit", "--quiet", "--message", message]);
    }

    fn git(&self, arguments: &[&str]) {
        let output = Command::new("git")
            .args([
                "-c",
                "user.name=A maintainer",
                "-c",
                "user.email=maintainer@example.com",
            ])
            .args(arguments)
            .current_dir(self.directory.path())
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .output()
            .expect("git runs");
        assert!(
            output.status.success(),
            "git {arguments:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn run_annotated_release(&self, arguments: &[&str]) -> Result<String, String> {
        let output = Command::new(env!("CARGO_BIN_EXE_annotated-release"))
            .args(arguments)
            .current_dir(self.directory.path())
            .output()
            .expect("annotated-release runs");
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
        }
    }
}

fn lock_file(version: &str) -> String {
    format!(
        "\
[[package]]
name = \"annotated\"
version = \"{version}\"

[[package]]
name = \"annotated-derive\"
version = \"{version}\"
"
    )
}

fn changelog(version: &str) -> String {
    format!(
        "\
# Changelog

## [Unreleased]

## [{version}] - 2026-11-12

[Unreleased]: https://github.com/pablocom/annotated/compare/v{version}...HEAD
[{version}]: https://github.com/pablocom/annotated/releases/tag/v{version}
"
    )
}
