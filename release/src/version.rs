use std::fmt;
use std::str::FromStr;

use crate::change::Change;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
    major: u64,
    minor: u64,
    patch: u64,
}

impl Version {
    pub fn after(self, largest_change: Change) -> Self {
        let promises_a_stable_api = self.major > 0;
        match largest_change {
            Change::Breaking if promises_a_stable_api => Self {
                major: self.major + 1,
                minor: 0,
                patch: 0,
            },
            Change::Breaking => Self {
                minor: self.minor + 1,
                patch: 0,
                ..self
            },
            Change::Feature if promises_a_stable_api => Self {
                minor: self.minor + 1,
                patch: 0,
                ..self
            },
            Change::Feature | Change::Fix | Change::Internal => Self {
                patch: self.patch + 1,
                ..self
            },
        }
    }
}

impl FromStr for Version {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let numbers: Result<Vec<u64>, _> = text.split('.').map(str::parse).collect();
        match numbers.as_deref() {
            Ok(&[major, minor, patch]) => Ok(Self { major, minor, patch }),
            _ => Err(format!("`{text}` isn't a version written like 1.2.3")),
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}
