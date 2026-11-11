use std::cell::Cell;

use crate::{Problem, Rule};

/// The rule that a string is an absolute URL, valid by the [WHATWG URL Standard](https://url.spec.whatwg.org/),
/// whose scheme is `http` or `https`, or one of those [`Url::schemes`] sets.
///
/// It checks anything that is `AsRef<str>`, exactly as written: surrounding whitespace breaks it.
///
/// ```
/// use annotated::{Rule, rules};
///
/// assert_eq!(rules::url().check("https://example.com/projects/1"), Ok(()));
/// assert_eq!(rules::url().schemes(&["ftp"]).check("ftp://example.com/report.pdf"), Ok(()));
/// ```
#[must_use]
pub const fn url() -> Url {
    Url {
        schemes: &["http", "https"],
    }
}

/// Checks that a string is a URL of an accepted scheme; made by [`url()`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Url {
    schemes: &'static [&'static str],
}

impl Url {
    /// The rule accepting URLs of these `schemes` instead of `http` and `https`: `schemes = &["ftp"]`.
    #[must_use]
    pub const fn schemes(self, schemes: &'static [&'static str]) -> Self {
        Self { schemes }
    }
}

#[diagnostic::do_not_recommend]
impl<T: AsRef<str> + ?Sized> Rule<T> for Url {
    fn check(&self, value: &T) -> Result<(), Problem> {
        let has_accepted_scheme = valid_url(value.as_ref()).is_some_and(|url| self.schemes.contains(&url.scheme()));
        if has_accepted_scheme {
            return Ok(());
        }
        Err(Problem::Url { schemes: self.schemes })
    }
}

fn valid_url(text: &str) -> Option<url::Url> {
    let has_syntax_violation = Cell::new(false);
    let url = url::Url::options()
        .syntax_violation_callback(Some(&|_| has_syntax_violation.set(true)))
        .parse(text)
        .ok()?;
    (!has_syntax_violation.get()).then_some(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn not_a_url_of(schemes: &'static [&'static str]) -> Result<(), Problem> {
        Err(Problem::Url { schemes })
    }

    #[test]
    fn an_https_url_follows_the_rule() {
        assert_eq!(url().check("https://example.com/projects/1?tab=members#top"), Ok(()));
    }

    #[test]
    fn an_http_url_follows_the_rule() {
        assert_eq!(url().check("http://example.com"), Ok(()));
    }

    #[test]
    fn a_url_of_another_scheme_breaks_the_rule() {
        assert_eq!(url().check("ftp://example.com"), not_a_url_of(&["http", "https"]));
    }

    #[test]
    fn a_url_of_a_scheme_set_follows_the_rule() {
        assert_eq!(url().schemes(&["ftp", "sftp"]).check("sftp://example.com"), Ok(()));
    }

    #[test]
    fn schemes_set_replace_http_and_https() {
        assert_eq!(
            url().schemes(&["ftp"]).check("https://example.com"),
            not_a_url_of(&["ftp"])
        );
    }

    #[test]
    fn a_relative_url_breaks_the_rule() {
        assert_eq!(url().check("/projects/1"), not_a_url_of(&["http", "https"]));
    }

    #[test]
    fn a_url_with_a_syntax_error_by_the_url_standard_breaks_the_rule() {
        assert_eq!(url().check("https:example.com"), not_a_url_of(&["http", "https"]));
    }

    #[test]
    fn a_url_with_a_space_breaks_the_rule() {
        assert_eq!(url().check("https://example.com/a b"), not_a_url_of(&["http", "https"]));
    }

    #[test]
    fn a_url_with_credentials_breaks_the_rule() {
        assert_eq!(
            url().check("https://alex:secret@example.com"),
            not_a_url_of(&["http", "https"])
        );
    }

    #[test]
    fn surrounding_whitespace_breaks_the_rule() {
        assert_eq!(url().check(" https://example.com"), not_a_url_of(&["http", "https"]));
    }
}
