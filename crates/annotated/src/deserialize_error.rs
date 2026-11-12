use std::error::Error;
use std::fmt::{self, Display};

use crate::{Issue, Issues};

/// Why a value couldn't be deserialized: the format's own error, or the issues found in it.
///
/// [`DeserializeError::issues`] returns the issues, and an empty slice for the format's error, so a caller can
/// read them without matching. Its `Display` is that of the error it holds: the format's message, or one issue per
/// line.
#[derive(Debug)]
#[non_exhaustive]
pub enum DeserializeError<E> {
    /// The format's own error, which ends deserialization as it would end serde's. It can be input that isn't well
    /// formed, an unknown field under `#[serde(deny_unknown_fields)]`, a field that appears twice, or a value of the
    /// wrong type. A struct with `#[annotated(collect_type_errors)]` reports values of the wrong type as issues
    /// instead.
    Format(E),
    /// Every issue found in the value, in the order found: missing fields, broken rules, and values of the wrong
    /// type when the struct collects them.
    Issues(Issues),
}

impl<E> DeserializeError<E> {
    /// The issues found, in the order found. The slice is empty for the format's error.
    #[must_use]
    pub fn issues(&self) -> &[Issue] {
        match self {
            Self::Format(_) => &[],
            Self::Issues(issues) => issues.as_slice(),
        }
    }
}

impl<E: Display> Display for DeserializeError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format(format_error) => format_error.fmt(formatter),
            Self::Issues(issues) => issues.fmt(formatter),
        }
    }
}

impl<E: Error + 'static> Error for DeserializeError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Format(format_error) => format_error.source(),
            Self::Issues(issues) => issues.source(),
        }
    }
}
