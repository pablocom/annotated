use crate::{Problem, Rule};

/// The rule that a string contains `text`: `contains("@")`.
///
/// ```
/// use annotated::{Rule, rules};
///
/// assert_eq!(rules::contains("@").check("alex@example.com"), Ok(()));
/// ```
#[must_use]
pub const fn contains(text: &'static str) -> Contains {
    Contains { text }
}

/// The rule that a string starts with `prefix`, matching letter case: `prefix("SKU-")`.
///
/// ```
/// use annotated::{Rule, rules};
///
/// assert_eq!(rules::prefix("SKU-").check("SKU-1042"), Ok(()));
/// ```
#[must_use]
pub const fn prefix(prefix: &'static str) -> Prefix {
    Prefix { prefix }
}

/// The rule that a string ends with `suffix`, matching letter case: `suffix(".pdf")`.
///
/// ```
/// use annotated::{Rule, rules};
///
/// assert_eq!(rules::suffix(".pdf").check("manual.pdf"), Ok(()));
/// ```
#[must_use]
pub const fn suffix(suffix: &'static str) -> Suffix {
    Suffix { suffix }
}

/// Checks that a string contains a text. Made by [`contains`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Contains {
    text: &'static str,
}

/// Checks that a string starts with a prefix. Made by [`prefix()`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Prefix {
    prefix: &'static str,
}

/// Checks that a string ends with a suffix. Made by [`suffix()`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Suffix {
    suffix: &'static str,
}

#[diagnostic::do_not_recommend]
impl<T: AsRef<str> + ?Sized> Rule<T> for Contains {
    fn check(&self, value: &T) -> Result<(), Problem> {
        if value.as_ref().contains(self.text) {
            return Ok(());
        }
        Err(Problem::Contains { text: self.text })
    }

    fn problem(&self) -> Option<Problem> {
        Some(Problem::Contains { text: self.text })
    }
}

#[diagnostic::do_not_recommend]
impl<T: AsRef<str> + ?Sized> Rule<T> for Prefix {
    fn check(&self, value: &T) -> Result<(), Problem> {
        if value.as_ref().starts_with(self.prefix) {
            return Ok(());
        }
        Err(Problem::Prefix { prefix: self.prefix })
    }

    fn problem(&self) -> Option<Problem> {
        Some(Problem::Prefix { prefix: self.prefix })
    }
}

#[diagnostic::do_not_recommend]
impl<T: AsRef<str> + ?Sized> Rule<T> for Suffix {
    fn check(&self, value: &T) -> Result<(), Problem> {
        if value.as_ref().ends_with(self.suffix) {
            return Ok(());
        }
        Err(Problem::Suffix { suffix: self.suffix })
    }

    fn problem(&self) -> Option<Problem> {
        Some(Problem::Suffix { suffix: self.suffix })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_says_the_text_a_string_must_contain() {
        assert_eq!(
            Rule::<str>::problem(&contains("@")),
            Some(Problem::Contains { text: "@" })
        );
    }

    #[test]
    fn prefix_says_the_text_a_string_must_start_with() {
        assert_eq!(
            Rule::<str>::problem(&prefix("https://")),
            Some(Problem::Prefix { prefix: "https://" })
        );
    }

    #[test]
    fn suffix_says_the_text_a_string_must_end_with() {
        assert_eq!(
            Rule::<str>::problem(&suffix(".png")),
            Some(Problem::Suffix { suffix: ".png" })
        );
    }

    #[test]
    fn a_string_containing_the_text_follows_the_rule() {
        assert_eq!(contains("@").check("alex@example.com"), Ok(()));
    }

    #[test]
    fn a_string_without_the_text_breaks_the_rule() {
        assert_eq!(
            contains("@").check("alex.example.com"),
            Err(Problem::Contains { text: "@" })
        );
    }

    #[test]
    fn a_string_starting_with_the_prefix_follows_the_rule() {
        assert_eq!(prefix("SKU-").check("SKU-1042"), Ok(()));
    }

    #[test]
    fn a_string_starting_with_the_prefix_in_another_case_breaks_the_rule() {
        assert_eq!(
            prefix("SKU-").check("sku-1042"),
            Err(Problem::Prefix { prefix: "SKU-" })
        );
    }

    #[test]
    fn a_string_ending_with_the_suffix_follows_the_rule() {
        assert_eq!(suffix(".pdf").check("manual.pdf"), Ok(()));
    }

    #[test]
    fn a_string_ending_with_the_suffix_in_another_case_breaks_the_rule() {
        assert_eq!(
            suffix(".pdf").check("manual.PDF"),
            Err(Problem::Suffix { suffix: ".pdf" })
        );
    }
}
