use std::error::Error;
use std::fmt::{self, Display};
use std::slice;

use crate::Issue;

/// The issues found in one validation, at most the limit of the [`Report`](crate::Report) they were found in. This
/// is the error type, and it is never empty.
///
/// Iterating over it yields the issues in the order they were found, and its [`Display`] prints one issue per
/// line. [`Issues::is_complete`] says whether the value may have more.
///
/// With the `serde` feature, on by default, it serializes as the sequence of its issues, each as
/// [`Issue`] serializes it. Whether they are complete isn't serialized: send it apart when the client needs it.
///
/// ```
/// use annotated::Validate;
///
/// #[derive(Validate)]
/// struct NewMember {
///     #[validate(range(min = 18))]
///     age: u8,
/// }
///
/// let issues = NewMember { age: 12 }.validate().unwrap_err();
///
/// assert_eq!(
///     serde_json::to_string(&issues).unwrap(),
///     r#"[{"path":["age"],"code":"range","params":{"min":18},"message":"must be at least 18"}]"#
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Issues {
    issues: Vec<Issue>,
    is_complete: bool,
}

impl Issues {
    /// The issues of `found`, every issue in the value, or `None` when there are none.
    pub(crate) fn of(found: Vec<Issue>) -> Option<Self> {
        (!found.is_empty()).then_some(Self {
            issues: found,
            is_complete: true,
        })
    }

    /// These issues, cut off at the limit of their report: the value may have more.
    pub(crate) fn cut_off(self) -> Self {
        Self {
            is_complete: false,
            ..self
        }
    }

    /// The issues, in the order they were found.
    pub fn iter(&self) -> slice::Iter<'_, Issue> {
        self.issues.iter()
    }

    /// Whether these are every issue in the value.
    ///
    /// It is `false` when the report they were found in reached its limit, so the value may have more issues than
    /// are listed, even if it has none. So it is always `false` for the issue returned when failing fast. It is
    /// `false` too when the report stopped below its limit because issues may be missing among those it holds, as
    /// [`Report`](crate::Report) describes.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.is_complete
    }

    /// The issues as a slice, in the order they were found.
    #[must_use]
    pub fn as_slice(&self) -> &[Issue] {
        &self.issues
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Issues {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.iter())
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
