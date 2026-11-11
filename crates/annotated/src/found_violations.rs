use std::borrow::Cow;

use crate::{Location, Problem, Violation, Violations};

/// The violations found so far in one validation, gathered as validation goes through a value.
///
/// It allocates on its first violation, not before, so validating a valid value costs nothing.
#[derive(Debug, Default)]
pub struct FoundViolations {
    violations: Vec<Violation>,
}

impl FoundViolations {
    /// No violations yet.
    #[must_use]
    pub const fn new() -> Self {
        Self { violations: Vec::new() }
    }

    /// Records `problem` at the location `at`.
    pub fn push(&mut self, at: &Location<'_>, problem: Problem) {
        self.violations.push(Violation::new(at.to_path(), problem));
    }

    /// Records `problem` at the location `at`, with `message` instead of the problem's own.
    ///
    /// `{name}` in the message is filled with the problem's parameter `name`; `{{` and `}}` are literal braces.
    pub fn push_with_message(&mut self, at: &Location<'_>, problem: Problem, message: impl Into<Cow<'static, str>>) {
        self.violations
            .push(Violation::new(at.to_path(), problem).with_message(message));
    }

    /// How many violations were found so far.
    #[must_use]
    pub fn len(&self) -> usize {
        self.violations.len()
    }

    /// Whether no violation was found so far.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.violations.is_empty()
    }

    /// `Ok(())` when no violation was found, or every violation found, in the order they were found.
    ///
    /// # Errors
    ///
    /// The [`Violations`] found, when there is at least one.
    pub fn into_result(self) -> Result<(), Violations> {
        Violations::of(self.violations).map_or(Ok(()), Err)
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
    fn finding_no_violation_is_no_error() {
        let found = FoundViolations::new();

        assert_eq!(found.into_result(), Ok(()));
    }

    #[test]
    fn a_violation_is_found_at_the_path_of_its_location() {
        let root = Location::root();
        let members = root.field("members");
        let mut found = FoundViolations::new();

        found.push(&members.index(1).field("name"), reserved_name());

        let violations = found.into_result().unwrap_err();
        assert_eq!(
            violations.as_slice(),
            [Violation::new(
                Path::root().field("members").index(1).field("name"),
                reserved_name()
            )]
        );
    }

    #[test]
    fn violations_are_answered_in_the_order_they_were_found() {
        let root = Location::root();
        let mut found = FoundViolations::new();

        found.push(&root.field("repeatedPassword"), passwords_differ());
        found.push(&root.field("name"), reserved_name());

        let violations = found.into_result().unwrap_err();
        assert_eq!(
            violations.into_iter().collect::<Vec<_>>(),
            [
                Violation::new(Path::root().field("repeatedPassword"), passwords_differ()),
                Violation::new(Path::root().field("name"), reserved_name()),
            ]
        );
    }

    #[test]
    fn a_violation_found_with_a_message_keeps_it() {
        let root = Location::root();
        let mut found = FoundViolations::new();

        found.push_with_message(&root.field("name"), reserved_name(), "is taken");

        let violations = found.into_result().unwrap_err();
        assert_eq!(violations.to_string(), "name: is taken");
    }

    #[test]
    fn violations_read_as_one_violation_per_line() {
        let root = Location::root();
        let mut found = FoundViolations::new();
        found.push(&root.field("name"), reserved_name());
        found.push(&root, passwords_differ());

        let violations = found.into_result().unwrap_err();

        assert_eq!(violations.to_string(), "name: is reserved\nmust match the password");
    }

    #[test]
    fn found_violations_count_what_was_found_so_far() {
        let root = Location::root();
        let mut found = FoundViolations::new();

        found.push(&root.field("name"), reserved_name());

        assert_eq!((found.len(), found.is_empty()), (1, false));
    }
}
