use regex::Regex;

use crate::{Problem, Rule};

/// The rule that a string matches a regular expression. A match anywhere in the string passes, unless the expression is
/// anchored with `^` and `$`.
///
/// `#[validate(pattern = r"^\d{5}$")]` compiles the expression once, into a `static` at the field, and an expression
/// that doesn't parse is a compile error. When you use the rule by hand, keep the [`Regex`] in a `static` in the same
/// way.
///
/// ```
/// use std::sync::LazyLock;
///
/// use annotated::{Rule, rules};
/// use regex::Regex;
///
/// static POSTAL_CODE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\d{5}$").unwrap());
///
/// assert_eq!(rules::pattern(&POSTAL_CODE).check("28013"), Ok(()));
/// ```
#[must_use]
pub const fn pattern(regex: &'static Regex) -> Pattern {
    Pattern { regex }
}

/// Checks that a string matches a regular expression. Made by [`pattern()`].
#[derive(Debug, Clone, Copy)]
pub struct Pattern {
    regex: &'static Regex,
}

#[diagnostic::do_not_recommend]
impl<T: AsRef<str> + ?Sized> Rule<T> for Pattern {
    fn check(&self, value: &T) -> Result<(), Problem> {
        if self.regex.is_match(value.as_ref()) {
            return Ok(());
        }
        Err(Problem::Pattern {
            pattern: self.regex.as_str(),
        })
    }

    fn problem(&self) -> Option<Problem> {
        Some(Problem::Pattern {
            pattern: self.regex.as_str(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::LazyLock;

    use super::*;

    static POSTAL_CODE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\d{5}$").unwrap());
    static DIGIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\d").unwrap());

    #[test]
    fn pattern_says_the_expression_a_string_must_match() {
        assert_eq!(
            Rule::<str>::problem(&pattern(&POSTAL_CODE)),
            Some(Problem::Pattern { pattern: r"^\d{5}$" })
        );
    }

    #[test]
    fn a_string_matching_the_expression_follows_the_rule() {
        assert_eq!(pattern(&POSTAL_CODE).check("28013"), Ok(()));
    }

    #[test]
    fn a_string_not_matching_the_expression_breaks_the_rule() {
        assert_eq!(
            pattern(&POSTAL_CODE).check("2801"),
            Err(Problem::Pattern { pattern: r"^\d{5}$" })
        );
    }

    #[test]
    fn an_expression_without_anchors_matches_anywhere_in_the_string() {
        assert_eq!(pattern(&DIGIT).check("room 4"), Ok(()));
    }
}
