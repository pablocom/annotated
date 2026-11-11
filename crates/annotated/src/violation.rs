use std::borrow::Cow;
use std::fmt::{self, Display};

use crate::{Path, Problem};

/// One broken rule, at the path where it happened.
///
/// ```
/// use annotated::{Path, Problem, Violation};
///
/// let reserved_name = Problem::custom("reserved_name", "is reserved").with_param("name", "admin");
/// let violation = Violation::new(Path::root().field("name"), reserved_name).with_message("{name} is taken");
///
/// assert_eq!(violation.to_string(), "name: admin is taken");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    path: Path,
    problem: Problem,
    message_override: Option<Cow<'static, str>>,
}

impl Violation {
    /// The violation of `problem` at `path`, with the problem's own message.
    #[must_use]
    pub fn new(path: Path, problem: Problem) -> Self {
        Self {
            path,
            problem,
            message_override: None,
        }
    }

    /// This violation, with `message` instead of the problem's own.
    ///
    /// `{name}` in the message is filled with the problem's parameter `name`; `{{` and `}}` are literal braces.
    #[must_use]
    pub fn with_message(mut self, message: impl Into<Cow<'static, str>>) -> Self {
        self.message_override = Some(message.into());
        self
    }

    /// Where in the value it happened.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// What kind of violation it is.
    #[must_use]
    pub fn problem(&self) -> &Problem {
        &self.problem
    }

    /// The message for people: the override, with its placeholders filled from the problem's parameters,
    /// or else the problem's English message.
    #[must_use]
    pub fn message(&self) -> String {
        match &self.message_override {
            Some(message_override) => fill_placeholders(message_override, &self.problem.params()),
            None => self.problem.to_string(),
        }
    }
}

fn fill_placeholders(message: &str, params: &[(&'static str, String)]) -> String {
    let mut filled = String::with_capacity(message.len());
    let mut rest = message;
    while let Some(next_character) = rest.chars().next() {
        rest = if let Some(after) = rest.strip_prefix("{{") {
            filled.push('{');
            after
        } else if let Some(after) = rest.strip_prefix("}}") {
            filled.push('}');
            after
        } else if let Some((value, after)) = param_placeholder_at_start_of(rest, params) {
            filled.push_str(value);
            after
        } else {
            filled.push(next_character);
            &rest[next_character.len_utf8()..]
        };
    }
    filled
}

fn param_placeholder_at_start_of<'text, 'params>(
    text: &'text str,
    params: &'params [(&'static str, String)],
) -> Option<(&'params str, &'text str)> {
    let (name, after) = text.strip_prefix('{')?.split_once('}')?;
    let (_, value) = params.iter().find(|(param_name, _)| *param_name == name)?;
    Some((value, after))
}

impl Display for Violation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.path.is_root() {
            return formatter.write_str(&self.message());
        }
        write!(formatter, "{}: {}", self.path, self.message())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reserved_name() -> Problem {
        Problem::custom("reserved_name", "is reserved").with_param("name", "admin")
    }

    #[test]
    fn a_violation_s_message_is_its_problem_s_message() {
        let violation = Violation::new(Path::root().field("name"), reserved_name());

        assert_eq!(violation.message(), "is reserved");
    }

    #[test]
    fn a_message_override_replaces_the_problem_s_message() {
        let violation = Violation::new(Path::root().field("name"), reserved_name()).with_message("is taken");

        assert_eq!(violation.message(), "is taken");
    }

    #[test]
    fn a_placeholder_in_a_message_override_is_filled_from_the_problem_s_params() {
        let violation = Violation::new(Path::root().field("name"), reserved_name()).with_message("{name} is taken");

        assert_eq!(violation.message(), "admin is taken");
    }

    #[test]
    fn a_placeholder_naming_no_param_is_left_as_written() {
        let violation = Violation::new(Path::root().field("name"), reserved_name()).with_message("{nickname} is taken");

        assert_eq!(violation.message(), "{nickname} is taken");
    }

    #[test]
    fn doubled_braces_in_a_message_override_are_literal_braces() {
        let violation = Violation::new(Path::root().field("name"), reserved_name()).with_message("{{name}} is {name}");

        assert_eq!(violation.message(), "{name} is admin");
    }

    #[test]
    fn an_unclosed_brace_in_a_message_override_is_left_as_written() {
        let violation = Violation::new(Path::root().field("name"), reserved_name()).with_message("{name is taken");

        assert_eq!(violation.message(), "{name is taken");
    }

    #[test]
    fn a_violation_reads_as_its_path_and_message() {
        let violation = Violation::new(Path::root().field("members").index(1).field("name"), reserved_name());

        assert_eq!(violation.to_string(), "members[1].name: is reserved");
    }

    #[test]
    fn a_violation_at_the_root_reads_as_its_message_alone() {
        let violation = Violation::new(Path::root(), reserved_name());

        assert_eq!(violation.to_string(), "is reserved");
    }
}
