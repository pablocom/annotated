use std::borrow::Borrow;
use std::marker::PhantomData;

use crate::{Problem, Rule};

/// The rule that `check` accepts a value: any `Fn(&V) -> Result<(), Problem>`, reporting with
/// [`Problem::custom`].
///
/// It checks any value that borrows as a `V`, so a check of `&str` checks a `String` field too.
///
/// ```
/// use annotated::{Problem, Rule, rules};
///
/// fn not_reserved(name: &str) -> Result<(), Problem> {
///     if name == "admin" {
///         return Err(Problem::custom("reserved_name", "is reserved"));
///     }
///     Ok(())
/// }
///
/// assert_eq!(rules::custom(not_reserved).check(&"alex".to_owned()), Ok(()));
/// ```
#[must_use]
pub const fn custom<V: ?Sized, F: Fn(&V) -> Result<(), Problem>>(check: F) -> Custom<F, V> {
    Custom {
        check,
        checked: PhantomData,
    }
}

/// Checks a value with a function of your own; made by [`custom`].
pub struct Custom<F, V: ?Sized> {
    check: F,
    checked: PhantomData<fn(&V)>,
}

impl<T, V, F> Rule<T> for Custom<F, V>
where
    T: Borrow<V> + ?Sized,
    V: ?Sized,
    F: Fn(&V) -> Result<(), Problem>,
{
    fn check(&self, value: &T) -> Result<(), Problem> {
        (self.check)(value.borrow())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn not_reserved(name: &str) -> Result<(), Problem> {
        if name == "admin" {
            return Err(Problem::custom("reserved_name", "is reserved"));
        }
        Ok(())
    }

    #[test]
    fn a_value_your_check_accepts_follows_the_rule() {
        assert_eq!(custom(not_reserved).check("alex"), Ok(()));
    }

    #[test]
    fn a_value_your_check_refuses_breaks_the_rule_with_its_problem() {
        assert_eq!(
            custom(not_reserved).check("admin"),
            Err(Problem::custom("reserved_name", "is reserved"))
        );
    }

    #[test]
    fn a_check_of_a_borrowed_form_checks_the_owned_value() {
        assert_eq!(
            custom(not_reserved).check(&"admin".to_owned()),
            Err(Problem::custom("reserved_name", "is reserved"))
        );
    }
}
