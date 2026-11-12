use std::error::Error;
use std::fmt::{self, Display};
use std::slice;

use crate::Issue;

/// Every issue found in one validation. This is the error type, and it is never empty.
///
/// Iterating over it yields the issues in the order they were found, and its [`Display`] prints one issue per
/// line.
#[derive(Debug, Clone, PartialEq)]
pub struct Issues {
    issues: Vec<Issue>,
}

impl Issues {
    /// The issues of `found`, or `None` when there are none.
    pub(crate) fn of(found: Vec<Issue>) -> Option<Self> {
        (!found.is_empty()).then_some(Self { issues: found })
    }

    /// The issues, in the order they were found.
    pub fn iter(&self) -> slice::Iter<'_, Issue> {
        self.issues.iter()
    }

    /// The issues as a slice, in the order they were found.
    #[must_use]
    pub fn as_slice(&self) -> &[Issue] {
        &self.issues
    }
}

impl Display for Issues {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (first, others) = self.issues.split_first().expect("issues are never empty");
        write!(formatter, "{first}")?;
        for issue in others {
            write!(formatter, "\n{issue}")?;
        }
        Ok(())
    }
}

impl Error for Issues {}

impl IntoIterator for Issues {
    type Item = Issue;
    type IntoIter = std::vec::IntoIter<Issue>;

    fn into_iter(self) -> Self::IntoIter {
        self.issues.into_iter()
    }
}

impl<'a> IntoIterator for &'a Issues {
    type Item = &'a Issue;
    type IntoIter = slice::Iter<'a, Issue>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
