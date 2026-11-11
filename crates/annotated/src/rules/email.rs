use crate::{Problem, Rule};

/// The rule that a string is an email address, as HTML's `<input type="email">` accepts one: a local part of
/// letters, digits, dots and the symbols `` !#$%&'*+/=?^_`{|}~- ``, an `@`, and a domain of labels made of
/// letters, digits and hyphens. The address is at most 254 bytes and its local part at most 64.
///
/// It checks anything that is `AsRef<str>`, exactly as written: surrounding whitespace breaks it.
///
/// ```
/// use annotated::{Rule, rules};
///
/// assert_eq!(rules::email().check("alex@example.com"), Ok(()));
/// assert_eq!(rules::email().check("alex.example.com").unwrap_err().to_string(), "must be an email address");
/// ```
#[must_use]
pub const fn email() -> Email {
    Email
}

/// Checks that a string is an email address; made by [`email`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Email;

const ADDRESS_MAXIMUM_BYTES: usize = 254;
const LOCAL_PART_MAXIMUM_BYTES: usize = 64;
const LABEL_MAXIMUM_CHARACTERS: usize = 63;
const LOCAL_PART_SYMBOLS: &str = ".!#$%&'*+-/=?^_`{|}~";

#[diagnostic::do_not_recommend]
impl<T: AsRef<str> + ?Sized> Rule<T> for Email {
    fn check(&self, value: &T) -> Result<(), Problem> {
        if is_email_address(value.as_ref()) {
            return Ok(());
        }
        Err(Problem::Email)
    }
}

fn is_email_address(address: &str) -> bool {
    let Some((local_part, domain)) = address.split_once('@') else {
        return false;
    };
    address.len() <= ADDRESS_MAXIMUM_BYTES && is_local_part(local_part) && domain.split('.').all(is_label)
}

fn is_local_part(local_part: &str) -> bool {
    !local_part.is_empty()
        && local_part.len() <= LOCAL_PART_MAXIMUM_BYTES
        && local_part
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || LOCAL_PART_SYMBOLS.contains(character))
}

fn is_label(label: &str) -> bool {
    label.len() <= LABEL_MAXIMUM_CHARACTERS
        && label.starts_with(|character: char| character.is_ascii_alphanumeric())
        && label.ends_with(|character: char| character.is_ascii_alphanumeric())
        && label
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_email_address_follows_the_rule() {
        assert_eq!(email().check("alex@example.com"), Ok(()));
    }

    #[test]
    fn an_email_address_needs_an_at_sign() {
        assert_eq!(email().check("alex.example.com"), Err(Problem::Email));
    }

    #[test]
    fn an_empty_string_is_not_an_email_address() {
        assert_eq!(email().check(""), Err(Problem::Email));
    }

    #[test]
    fn surrounding_whitespace_breaks_the_rule() {
        assert_eq!(email().check(" alex@example.com "), Err(Problem::Email));
    }

    #[test]
    fn the_local_part_takes_letters_digits_dots_and_the_symbols_html_allows() {
        assert_eq!(email().check("alex.morgan+news!#$%&'*/=?^_`{|}~-@example.com"), Ok(()));
    }

    #[test]
    fn the_local_part_takes_no_space() {
        assert_eq!(email().check("alex morgan@example.com"), Err(Problem::Email));
    }

    #[test]
    fn the_local_part_takes_no_letter_outside_ascii() {
        assert_eq!(email().check("zoë@example.com"), Err(Problem::Email));
    }

    #[test]
    fn the_local_part_is_not_empty() {
        assert_eq!(email().check("@example.com"), Err(Problem::Email));
    }

    #[test]
    fn the_local_part_has_at_most_64_bytes() {
        let longest_local_part = "a".repeat(64);
        let too_long_local_part = "a".repeat(65);

        assert_eq!(
            (
                email().check(&format!("{longest_local_part}@example.com")),
                email().check(&format!("{too_long_local_part}@example.com"))
            ),
            (Ok(()), Err(Problem::Email))
        );
    }

    #[test]
    fn the_domain_is_made_of_labels_of_letters_digits_and_hyphens() {
        assert_eq!(email().check("alex@mail-1.example.com"), Ok(()));
    }

    #[test]
    fn a_label_takes_no_other_character() {
        assert_eq!(email().check("alex@mail_1.example.com"), Err(Problem::Email));
    }

    #[test]
    fn a_label_is_not_empty() {
        assert_eq!(email().check("alex@example..com"), Err(Problem::Email));
    }

    #[test]
    fn the_domain_is_not_empty() {
        assert_eq!(email().check("alex@"), Err(Problem::Email));
    }

    #[test]
    fn a_label_does_not_start_with_a_hyphen() {
        assert_eq!(email().check("alex@-example.com"), Err(Problem::Email));
    }

    #[test]
    fn a_label_does_not_end_with_a_hyphen() {
        assert_eq!(email().check("alex@example-.com"), Err(Problem::Email));
    }

    #[test]
    fn a_label_has_at_most_63_characters() {
        let longest_label = "b".repeat(63);
        let too_long_label = "b".repeat(64);

        assert_eq!(
            (
                email().check(&format!("alex@{longest_label}.com")),
                email().check(&format!("alex@{too_long_label}.com"))
            ),
            (Ok(()), Err(Problem::Email))
        );
    }

    #[test]
    fn a_domain_of_a_single_label_is_accepted() {
        assert_eq!(email().check("alex@localhost"), Ok(()));
    }

    #[test]
    fn an_email_address_has_at_most_254_bytes() {
        let longest_address = format!(
            "{}@{}.{}.{}",
            "a".repeat(64),
            "b".repeat(63),
            "c".repeat(63),
            "d".repeat(61)
        );
        let too_long_address = format!(
            "{}@{}.{}.{}",
            "a".repeat(64),
            "b".repeat(63),
            "c".repeat(63),
            "d".repeat(62)
        );

        assert_eq!(
            (email().check(&longest_address), email().check(&too_long_address)),
            (Ok(()), Err(Problem::Email))
        );
    }
}
