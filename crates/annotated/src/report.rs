use std::borrow::Cow;
use std::mem;

use crate::{Issue, Issues, Location, Problem};

/// The issues found so far in one validation. Validation adds to it as it goes through a value.
///
/// It allocates when it records its first issue, not before, so a valid value never pays for it.
///
/// A report keeps at most its limit of issues, however many the value has:
///
/// - [`Report::new`] keeps up to [`Report::DEFAULT_LIMIT`], 1,000.
/// - [`Report::failing_fast`] keeps only the first issue.
/// - [`Report::limited_to`] keeps as many as it is given, and [`Report::unlimited`] keeps every issue.
///
/// Once it holds as many issues as its limit, it [is finished](Report::is_finished) and records nothing more. When
/// deserializing, the input is still read to its end.
///
/// The issues it keeps are listed in the order the full report, one without a limit, lists them:
///
/// - For a struct, an enum or a list, they are the first ones of the full report.
/// - A map reports its values' issues in its own order, which may not be the order it reads them in. Once its values'
///   issues reach the limit, which of them it keeps is unspecified. Failing fast still keeps exactly the first.
/// - When issues may be missing among those it holds, it keeps none after them, even below its limit. That happens
///   when a list's elements reached the limit, and the issues of an element whose `nested` rule doesn't run, because
///   a rule before it broke, are dropped: those of the elements after it may have been left out.
#[derive(Debug)]
pub struct Report {
    issues: Vec<Issue>,
    limit: usize,
}

impl Report {
    /// The most issues a report started with [`Report::new`] keeps: 1,000. Enough for a client to fix everything a
    /// request gets wrong, and few enough that a request with millions of problems can't make the server hold them
    /// all.
    pub const DEFAULT_LIMIT: usize = 1_000;

    /// An empty report that keeps at most [`Report::DEFAULT_LIMIT`] issues.
    #[must_use]
    pub const fn new() -> Self {
        Self::keeping_at_most(Self::DEFAULT_LIMIT)
    }

    /// An empty report that keeps only the first issue found.
    #[must_use]
    pub const fn failing_fast() -> Self {
        Self::keeping_at_most(1)
    }

    /// An empty report that keeps at most `most_issues` issues.
    ///
    /// A limit of 0 is taken as 1, the least that still tells a value with issues apart from a valid one.
    #[must_use]
    pub const fn limited_to(most_issues: usize) -> Self {
        if most_issues == 0 {
            return Self::failing_fast();
        }
        Self::keeping_at_most(most_issues)
    }

    /// An empty report that keeps every issue found.
    ///
    /// It holds as many issues as the value has, and a value read from input can have as many as the input makes
    /// it: a list of a million empty objects is a million missing fields for each field the elements require. Use
    /// it only for values you trust.
    #[must_use]
    pub const fn unlimited() -> Self {
        Self::keeping_at_most(usize::MAX)
    }

    /// An empty report that keeps at most `limit` issues, none when `limit` is 0. Only the report of a part of a value
    /// keeps none, when the report it joins is already finished: as the report of a whole value, it would let a value
    /// with issues pass.
    pub(crate) const fn keeping_at_most(limit: usize) -> Self {
        Self {
            issues: Vec::new(),
            limit,
        }
    }

    /// An empty report with the limit of this one.
    ///
    /// Use it to keep the issues found in one part of a value apart, until [`append`](Report::append)
    /// moves them into this report.
    #[must_use]
    pub const fn new_alike(&self) -> Self {
        Self::keeping_at_most(self.limit)
    }

    /// Whether validation should stop, because the report holds as many issues as its limit and records nothing
    /// more.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.issues.len() >= self.limit
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

    /// Records `issue` as it is, at its own path and with its own message, unless the report is finished.
    ///
    /// It moves an issue an app already has into the report: one found by another validation, or by a check of the
    /// app's own. [`Issue::new`] makes one from a [`Path`](crate::Path) and a [`Problem`], and
    /// [`Issue::with_message`] gives it a message of its own. A built-in problem is made by its rule, as
    /// [`Problem`] shows.
    ///
    /// ```
    /// use annotated::{Issue, Location, Path, Problem, Report, Validate};
    ///
    /// #[derive(Validate)]
    /// struct NewMember {
    ///     #[validate(email)]
    ///     email: String,
    /// }
    ///
    /// let new_member = NewMember { email: "alex@example.com".to_owned() };
    /// let mut found = Report::new();
    /// new_member.validate_into(&Location::root(), &mut found);
    ///
    /// let email_taken = Problem::custom("email_taken", "belongs to another member");
    /// found.push_issue(Issue::new(Path::root().field("email"), email_taken));
    ///
    /// assert_eq!(found.into_result().unwrap_err().to_string(), "email: belongs to another member");
    /// ```
    pub fn push_issue(&mut self, issue: Issue) {
        if self.is_finished() {
            return;
        }
        self.issues.push(issue);
    }

    /// Moves the issues in `other` to the end of this report, keeping their order, and leaves `other` empty.
    ///
    /// The report takes the first of them, as many as its limit has room for, so a finished report takes none. An
    /// empty report takes them without copying them.
    ///
    /// When `other` is finished, the part of the value it was found in may have more issues than it kept. So if
    /// this report takes every issue of `other` and still has room, it keeps no more: an issue found later would be
    /// listed before those `other` left out, out of the order of the full report.
    pub fn append(&mut self, other: &mut Report) {
        let may_other_have_left_issues_out = other.is_finished();
        self.take_issues_of(other);
        if may_other_have_left_issues_out {
            self.keep_no_more();
        }
    }

    /// Moves the first issues in `other`, as many as this report has room for, to its end, and leaves `other`
    /// empty.
    pub(crate) fn take_issues_of(&mut self, other: &mut Report) {
        if other.issues.is_empty() {
            return;
        }
        other.issues.truncate(self.room());
        if self.issues.is_empty() {
            mem::swap(&mut self.issues, &mut other.issues);
            return;
        }
        self.issues.append(&mut other.issues);
    }

    /// How many more issues the report keeps before it is finished.
    pub(crate) fn room(&self) -> usize {
        self.limit.saturating_sub(self.issues.len())
    }

    /// Lowers the limit to the issues the report holds, so that it is finished. Call it once issues may be missing
    /// after those it holds: none found later can then be listed before them. A report that holds none still keeps
    /// the next issue, so that a value with issues can't pass, unless it keeps none already.
    pub(crate) fn keep_no_more(&mut self) {
        let least_limit_that_fails_the_value = self.issues.len().max(1);
        self.limit = self.limit.min(least_limit_that_fails_the_value);
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
    /// Returns the [`Issues`] found, in the order they were found, when there is at least one. They are
    /// [complete](Issues::is_complete) unless the report reached its limit.
    pub fn into_result(self) -> Result<(), Issues> {
        let has_reached_its_limit = self.is_finished();
        match Issues::of(self.issues) {
            None => Ok(()),
            Some(issues) if has_reached_its_limit => Err(issues.cut_off()),
            Some(issues) => Err(issues),
        }
    }
}

impl Default for Report {
    /// An empty report that keeps at most [`Report::DEFAULT_LIMIT`] issues, as [`Report::new`].
    fn default() -> Self {
        Self::new()
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
        let found = Report::new();

        assert_eq!(found.into_result(), Ok(()));
    }

    #[test]
    fn an_issue_is_found_at_the_path_of_its_location() {
        let root = Location::root();
        let members = root.field("members");
        let mut found = Report::new();

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
        let mut found = Report::new();

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
        let mut found = Report::new();
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
        let mut found = Report::new();

        found.push_with_message(&root.field("name"), reserved_name(), "is taken");

        let issues = found.into_result().unwrap_err();
        assert_eq!(issues.to_string(), "name: is taken");
    }

    #[test]
    fn issues_read_as_one_issue_per_line() {
        let root = Location::root();
        let mut found = Report::new();
        found.push(&root.field("name"), reserved_name());
        found.push(&root, passwords_differ());

        let issues = found.into_result().unwrap_err();

        assert_eq!(issues.to_string(), "name: is reserved\nmust match the password");
    }

    #[test]
    fn a_report_failing_fast_is_finished_once_it_has_an_issue() {
        let root = Location::root();
        let mut found = Report::failing_fast();

        found.push(&root.field("name"), reserved_name());

        assert!(found.is_finished());
    }

    #[test]
    fn a_report_failing_fast_is_not_finished_before_its_first_issue() {
        let found = Report::failing_fast();

        assert!(!found.is_finished());
    }

    #[test]
    fn a_report_holding_fewer_issues_than_its_limit_is_not_finished() {
        let root = Location::root();
        let mut found = Report::limited_to(2);

        found.push(&root.field("name"), reserved_name());

        assert!(!found.is_finished());
    }

    #[test]
    fn a_report_is_finished_once_it_holds_as_many_issues_as_its_limit() {
        let root = Location::root();
        let mut found = Report::limited_to(2);

        found.push(&root.field("name"), reserved_name());
        found.push(&root, passwords_differ());

        assert!(found.is_finished());
    }

    #[test]
    fn a_report_limited_to_no_issue_keeps_one() {
        let root = Location::root();
        let mut found = Report::limited_to(0);

        found.push(&root.field("name"), reserved_name());
        found.push(&root, passwords_differ());

        let issues = found.into_result().unwrap_err();
        assert_eq!(
            issues.as_slice(),
            [Issue::new(Path::root().field("name"), reserved_name())]
        );
    }

    #[test]
    fn a_default_report_keeps_as_many_issues_as_a_new_one() {
        let found = Report::default();

        assert_eq!(found.limit, Report::DEFAULT_LIMIT);
    }

    #[test]
    fn issues_of_a_report_below_its_limit_are_complete() {
        let root = Location::root();
        let mut found = Report::limited_to(2);
        found.push(&root.field("name"), reserved_name());

        let issues = found.into_result().unwrap_err();

        assert!(issues.is_complete());
    }

    #[test]
    fn issues_of_a_report_that_reached_its_limit_are_not_complete() {
        let root = Location::root();
        let mut found = Report::limited_to(1);
        found.push(&root.field("name"), reserved_name());

        let issues = found.into_result().unwrap_err();

        assert!(!issues.is_complete());
    }

    #[test]
    fn a_finished_report_records_no_later_issue() {
        let root = Location::root();
        let mut found = Report::failing_fast();
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
        let mut found = Report::failing_fast().new_alike();

        found.push(&root.field("name"), reserved_name());

        assert!(found.is_finished());
    }

    #[test]
    fn a_report_started_alike_another_keeps_as_many_issues_as_it() {
        let root = Location::root();
        let mut found = Report::limited_to(2).new_alike();

        found.push(&root.field("name"), reserved_name());
        found.push(&root, passwords_differ());
        found.push(&root.field("repeatedPassword"), passwords_differ());

        assert_eq!(found.len(), 2);
    }

    #[test]
    fn appended_issues_come_after_those_found_before_in_their_order() {
        let root = Location::root();
        let mut found = Report::new();
        found.push(&root.field("name"), reserved_name());
        let mut found_in_a_part = Report::new();
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
        let mut found = Report::new();
        let mut found_in_a_part = Report::new();
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
        let mut found = Report::new();
        let mut found_in_a_part = Report::new();
        found_in_a_part.push(&root.field("name"), reserved_name());

        found.append(&mut found_in_a_part);

        assert!(found_in_a_part.is_empty());
    }

    #[test]
    fn a_finished_report_takes_no_appended_issue() {
        let root = Location::root();
        let mut found = Report::failing_fast();
        found.push(&root.field("name"), reserved_name());
        let mut found_in_a_part = Report::failing_fast();
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
        let mut found = Report::failing_fast();
        let mut found_in_a_part = Report::new();
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
    fn a_report_takes_the_first_appended_issues_its_limit_has_room_for() {
        let root = Location::root();
        let mut found = Report::limited_to(2);
        found.push(&root.field("name"), reserved_name());
        let mut found_in_a_part = Report::new();
        found_in_a_part.push(&root.field("repeatedPassword"), passwords_differ());
        found_in_a_part.push(&root, passwords_differ());

        found.append(&mut found_in_a_part);

        let issues = found.into_result().unwrap_err();
        assert_eq!(
            issues.as_slice(),
            [
                Issue::new(Path::root().field("name"), reserved_name()),
                Issue::new(Path::root().field("repeatedPassword"), passwords_differ()),
            ]
        );
    }

    #[test]
    fn a_report_that_takes_every_issue_of_a_finished_one_keeps_no_more() {
        let root = Location::root();
        let mut found = Report::limited_to(3);
        let mut found_in_a_part = Report::failing_fast();
        found_in_a_part.push(&root.field("repeatedPassword"), passwords_differ());
        found.append(&mut found_in_a_part);

        found.push(&root.field("name"), reserved_name());

        let issues = found.into_result().unwrap_err();
        assert_eq!(
            (issues.as_slice(), issues.is_complete()),
            (
                [Issue::new(Path::root().field("repeatedPassword"), passwords_differ())].as_slice(),
                false
            )
        );
    }

    #[test]
    fn a_report_that_keeps_no_more_before_any_issue_still_keeps_one() {
        let root = Location::root();
        let mut found = Report::new();
        found.keep_no_more();

        found.push(&root.field("name"), reserved_name());
        found.push(&root, passwords_differ());

        let issues = found.into_result().unwrap_err();
        assert_eq!(
            issues.as_slice(),
            [Issue::new(Path::root().field("name"), reserved_name())]
        );
    }

    #[test]
    fn a_report_that_keeps_none_keeps_none_after_keeping_no_more() {
        let root = Location::root();
        let mut found = Report::keeping_at_most(0);
        found.keep_no_more();

        found.push(&root.field("name"), reserved_name());

        assert!(found.is_empty());
    }

    #[test]
    fn a_report_counts_what_was_found_so_far() {
        let root = Location::root();
        let mut found = Report::new();

        found.push(&root.field("name"), reserved_name());

        assert_eq!((found.len(), found.is_empty()), (1, false));
    }
}
