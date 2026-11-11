use std::error::Error;
use std::fmt::{self, Display};
use std::slice;

use crate::Violation;

/// Every violation found in one validation: the error type. It is never empty.
///
/// It iterates over its violations in the order they were found, and reads as one violation per line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violations {
    violations: Vec<Violation>,
}

impl Violations {
    /// The violations of `found`, or `None` when there are none.
    pub(crate) fn of(found: Vec<Violation>) -> Option<Self> {
        (!found.is_empty()).then_some(Self { violations: found })
    }

    /// The violations, in the order they were found.
    pub fn iter(&self) -> slice::Iter<'_, Violation> {
        self.violations.iter()
    }

    /// The violations as a slice, in the order they were found.
    #[must_use]
    pub fn as_slice(&self) -> &[Violation] {
        &self.violations
    }
}

impl Display for Violations {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (first, others) = self.violations.split_first().expect("violations are never empty");
        write!(formatter, "{first}")?;
        for violation in others {
            write!(formatter, "\n{violation}")?;
        }
        Ok(())
    }
}

impl Error for Violations {}

impl IntoIterator for Violations {
    type Item = Violation;
    type IntoIter = std::vec::IntoIter<Violation>;

    fn into_iter(self) -> Self::IntoIter {
        self.violations.into_iter()
    }
}

impl<'a> IntoIterator for &'a Violations {
    type Item = &'a Violation;
    type IntoIter = slice::Iter<'a, Violation>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
