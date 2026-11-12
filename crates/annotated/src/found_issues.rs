use std::borrow::Cow;
use std::mem;

use crate::{Issue, Issues, Location, Problem};

/// The issues found so far in one validation. Validation adds to it as it goes through a value.
///
/// It allocates when it records its first issue, not before, so a valid value never pays for it.
///
/// A report started with [`FoundIssues::failing_fast`] keeps only its first issue. Once it has that issue,
/// it [is finished](FoundIssues::is_finished) and records nothing more.
#[derive(Debug, Default)]
pub struct FoundIssues {
    issues: Vec<Issue>,
    is_failing_fast: bool,
}

impl FoundIssues {
    /// An empty report that keeps every issue found.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            issues: Vec::new(),
            is_failing_fast: false,
        }
    }

    /// An empty report that keeps only the first issue found.
    #[must_use]
    pub const fn failing_fast() -> Self {
        Self {
            issues: Vec::new(),
            is_failing_fast: true,
        }
    }

    /// An empty report that gathers as this one does: every issue, or only the first when this report is failing
    /// fast.
    ///
    /// Use it to keep the issues found in one part of a value apart, until [`append`](FoundIssues::append)
    /// moves them into this report.
    #[must_use]
    pub const fn new_alike(&self) -> Self {
        Self {
            issues: Vec::new(),
            is_failing_fast: self.is_failing_fast,
        }
    }

    /// Whether validation should stop, because the report is failing fast and already has its issue. A report that
    /// keeps every issue is never finished.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.is_failing_fast && !self.issues.is_empty()
    }

    /// Records `problem` at the location `at`, unless the report is finished.
    pub fn push(&mut self, at: &Location<'_>, problem: Problem) {
        if self.is_finished() {
            return;
        }
        self.issues.push(Issue::new(at.to_path(), problem));
    }

    /// Records `problem` at the location `at`, with `message` in place of the problem's own message, unless the report
    /// is finished.
    ///
    /// `{name}` in the message is replaced by the problem's parameter `name`. Write `{{` and `}}` for literal braces.
    pub fn push_with_message(&mut self, at: &Location<'_>, problem: Problem, message: impl Into<Cow<'static, str>>) {
        if self.is_finished() {
            return;
        }
        self.issues
            .push(Issue::new(at.to_path(), problem).with_message(message));
    }

    /// Moves the issues in `other` to the end of this report, keeping their order, and leaves `other` empty.
    ///
    /// A finished report takes none of them, and a report that is failing fast takes only the first. An empty report
    /// takes them without copying them.
    pub fn append(&mut self, other: &mut FoundIssues) {
        if self.is_finished() {
            other.issues.clear();
            return;
        }
        if self.is_failing_fast {
            other.issues.truncate(1);
        }
        if self.issues.is_empty() {
            mem::swap(&mut self.issues, &mut other.issues);
            return;
        }
        self.issues.append(&mut other.issues);
    }

    pub(crate) fn push_issue(&mut self, issue: Issue) {
        if self.is_finished() {
            return;
        }
        self.issues.push(issue);
    }

    /// Moves every issue after the first `kept` into a new report that gathers as this one does, and leaves the
    /// first `kept` here.
    pub(crate) fn take_those_after(&mut self, kept: usize) -> FoundIssues {
        let taken = if kept == 0 {
            mem::take(&mut self.issues)
        } else {
            self.issues.split_off(kept)
        };
        Self {
            issues: taken,
            is_failing_fast: self.is_failing_fast,
        }
    }

    pub(crate) fn into_issues(self) -> Vec<Issue> {
        self.issues
    }

    /// How many issues were found so far.
    #[must_use]
    pub fn len(&self) -> usize {
        self.issues.len()
    }

    /// Whether no issue was found so far.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.issues.is_empty()
    }

    /// Turns the report into a result: `Ok(())` when no issue was found.
    ///
    /// # Errors
    ///
    /// Returns the [`Issues`] found, in the order they were found, when there is at least one.
    pub fn into_result(self) -> Result<(), Issues> {
        Issues::of(self.issues).map_or(Ok(()), Err)
    }
}

#[cfg(test)]
mod tests {
    use crate::Path;

    use super::*;

    fn reserved_name() -> Problem {
        Problem::custom("reserved_name", "is reserved")
    }

    fn passwords_differ() -> Problem {
        Problem::custom("passwords_differ", "must match the password")
    }

    #[test]
    fn finding_no_issue_is_no_error() {
        let found = FoundIssues::new();

        assert_eq!(found.into_result(), Ok(()));
    }

    #[test]
    fn an_issue_is_found_at_the_path_of_its_location() {
        let root = Location::root();
        let members = root.field("members");
        let mut found = FoundIssues::new();

        found.push(&members.index(1).field("name"), reserved_name());

        let issues = found.into_result().unwrap_err();
        assert_eq!(
            issues.as_slice(),
            [Issue::new(
                Path::root().field("members").index(1).field("name"),
                reserved_name()
            )]
        );
    }

    #[test]
    fn issues_are_answered_in_the_order_they_were_found() {
        let root = Location::root();
        let mut found = FoundIssues::new();

        found.push(&root.field("repeatedPassword"), passwords_differ());
        found.push(&root.field("name"), reserved_name());

        let issues = found.into_result().unwrap_err();
        assert_eq!(
            issues.into_iter().collect::<Vec<_>>(),
            [
                Issue::new(Path::root().field("repeatedPassword"), passwords_differ()),
                Issue::new(Path::root().field("name"), reserved_name()),
            ]
        );
    }

    #[test]
    fn issues_are_iterated_by_reference_in_the_order_they_were_found() {
        let root = Location::root();
        let mut found = FoundIssues::new();
        found.push(&root.field("repeatedPassword"), passwords_differ());
        found.push(&root.field("name"), reserved_name());
        let issues = found.into_result().unwrap_err();

        let iterated = (&issues).into_iter().collect::<Vec<_>>();

        assert_eq!(
            iterated,
            [
                &Issue::new(Path::root().field("repeatedPassword"), passwords_differ()),
                &Issue::new(Path::root().field("name"), reserved_name()),
            ]
        );
    }

    #[test]
    fn an_issue_found_with_a_message_keeps_it() {
        let root = Location::root();
        let mut found = FoundIssues::new();

        found.push_with_message(&root.field("name"), reserved_name(), "is taken");

        let issues = found.into_result().unwrap_err();
        assert_eq!(issues.to_string(), "name: is taken");
    }

    #[test]
    fn issues_read_as_one_issue_per_line() {
        let root = Location::root();
        let mut found = FoundIssues::new();
        found.push(&root.field("name"), reserved_name());
        found.push(&root, passwords_differ());

        let issues = found.into_result().unwrap_err();

        assert_eq!(issues.to_string(), "name: is reserved\nmust match the password");
    }

    #[test]
    fn a_report_failing_fast_is_finished_once_it_has_an_issue() {
        let root = Location::root();
        let mut found = FoundIssues::failing_fast();

        found.push(&root.field("name"), reserved_name());

        assert!(found.is_finished());
    }

    #[test]
    fn a_report_failing_fast_is_not_finished_before_its_first_issue() {
        let found = FoundIssues::failing_fast();

        assert!(!found.is_finished());
    }

    #[test]
    fn a_report_of_every_issue_is_never_finished() {
        let root = Location::root();
        let mut found = FoundIssues::new();

        found.push(&root.field("name"), reserved_name());

        assert!(!found.is_finished());
    }

    #[test]
    fn a_finished_report_records_no_later_issue() {
        let root = Location::root();
        let mut found = FoundIssues::failing_fast();
        found.push(&root.field("name"), reserved_name());

        found.push(&root.field("repeatedPassword"), passwords_differ());
        found.push_with_message(&root, passwords_differ(), "passwords must match");

        let issues = found.into_result().unwrap_err();
        assert_eq!(
            issues.as_slice(),
            [Issue::new(Path::root().field("name"), reserved_name())]
        );
    }

    #[test]
    fn a_report_started_alike_a_report_failing_fast_fails_fast() {
        let root = Location::root();
        let mut found = FoundIssues::failing_fast().new_alike();

        found.push(&root.field("name"), reserved_name());

        assert!(found.is_finished());
    }

    #[test]
    fn a_report_started_alike_a_report_of_every_issue_keeps_every_issue() {
        let root = Location::root();
        let mut found = FoundIssues::new().new_alike();

        found.push(&root.field("name"), reserved_name());
        found.push(&root, passwords_differ());

        assert_eq!(found.len(), 2);
    }

    #[test]
    fn appended_issues_come_after_those_found_before_in_their_order() {
        let root = Location::root();
        let mut found = FoundIssues::new();
        found.push(&root.field("name"), reserved_name());
        let mut found_in_a_part = FoundIssues::new();
        found_in_a_part.push(&root.field("repeatedPassword"), passwords_differ());
        found_in_a_part.push(&root, passwords_differ());

        found.append(&mut found_in_a_part);

        let issues = found.into_result().unwrap_err();
        assert_eq!(
            issues.as_slice(),
            [
                Issue::new(Path::root().field("name"), reserved_name()),
                Issue::new(Path::root().field("repeatedPassword"), passwords_differ()),
                Issue::new(Path::root(), passwords_differ()),
            ]
        );
    }

    #[test]
    fn appended_issues_are_kept_by_an_empty_report() {
        let root = Location::root();
        let mut found = FoundIssues::new();
        let mut found_in_a_part = FoundIssues::new();
        found_in_a_part.push(&root.field("name"), reserved_name());

        found.append(&mut found_in_a_part);

        let issues = found.into_result().unwrap_err();
        assert_eq!(
            issues.as_slice(),
            [Issue::new(Path::root().field("name"), reserved_name())]
        );
    }

    #[test]
    fn appending_issues_leaves_the_report_they_came_from_empty() {
        let root = Location::root();
        let mut found = FoundIssues::new();
        let mut found_in_a_part = FoundIssues::new();
        found_in_a_part.push(&root.field("name"), reserved_name());

        found.append(&mut found_in_a_part);

        assert!(found_in_a_part.is_empty());
    }

    #[test]
    fn a_finished_report_takes_no_appended_issue() {
        let root = Location::root();
        let mut found = FoundIssues::failing_fast();
        found.push(&root.field("name"), reserved_name());
        let mut found_in_a_part = FoundIssues::failing_fast();
        found_in_a_part.push(&root.field("repeatedPassword"), passwords_differ());

        found.append(&mut found_in_a_part);

        let issues = found.into_result().unwrap_err();
        assert_eq!(
            issues.as_slice(),
            [Issue::new(Path::root().field("name"), reserved_name())]
        );
    }

    #[test]
    fn a_report_failing_fast_takes_only_the_first_appended_issue() {
        let root = Location::root();
        let mut found = FoundIssues::failing_fast();
        let mut found_in_a_part = FoundIssues::new();
        found_in_a_part.push(&root.field("repeatedPassword"), passwords_differ());
        found_in_a_part.push(&root, passwords_differ());

        found.append(&mut found_in_a_part);

        let issues = found.into_result().unwrap_err();
        assert_eq!(
            issues.as_slice(),
            [Issue::new(Path::root().field("repeatedPassword"), passwords_differ())]
        );
    }

    #[test]
    fn found_issues_count_what_was_found_so_far() {
        let root = Location::root();
        let mut found = FoundIssues::new();

        found.push(&root.field("name"), reserved_name());

        assert_eq!((found.len(), found.is_empty()), (1, false));
    }
}
