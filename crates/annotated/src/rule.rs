use crate::Problem;

/// A check that a value of type `T` must pass, written in `#[validate(...)]`.
///
/// The functions in [`rules`](crate::rules) make the built-in rules. A rule of your own is a type that implements this
/// trait, with a function that makes it: `#[validate(my_rules::nif)]` calls `my_rules::nif()`.
///
/// ```
/// use annotated::{Problem, Rule, rules};
///
/// assert_eq!(rules::required().check("Alex"), Ok(()));
/// assert_eq!(rules::required().check("   "), Err(Problem::Required));
/// ```
#[diagnostic::on_unimplemented(
    message = "the rule `{Self}` can't check a value of type `{T}`",
    label = "`{Self}` can't check a `{T}`"
)]
pub trait Rule<T: ?Sized> {
    /// Checks `value` against this rule.
    ///
    /// # Errors
    ///
    /// Returns the [`Problem`] with `value`, when it breaks this rule.
    fn check(&self, value: &T) -> Result<(), Problem>;

    /// The problem that every value breaking this rule has. It says what the rule requires, as data.
    ///
    /// `length().max(50)` answers, for a string, a [`Problem::Length`] of at most 50 `char`s. The problems of the
    /// built-in rules hold the rule's parameters and never the value checked, so each describes its rule.
    ///
    /// It answers `None` by default. That suits a rule that can't say what it requires, like `custom(...)`, and a
    /// rule whose problem depends on the value it checked.
    ///
    /// ```
    /// use annotated::{Problem, Rule, rules};
    ///
    /// assert_eq!(Rule::<str>::problem(&rules::email()), Some(Problem::Email));
    /// ```
    fn problem(&self) -> Option<Problem> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Even;

    impl Rule<u32> for Even {
        fn check(&self, value: &u32) -> Result<(), Problem> {
            if value.is_multiple_of(2) {
                return Ok(());
            }
            Err(Problem::custom("even", "must be even"))
        }
    }

    #[test]
    fn a_rule_that_does_not_say_what_it_requires_has_no_problem_of_its_own() {
        assert_eq!(Even.problem(), None);
    }
}
