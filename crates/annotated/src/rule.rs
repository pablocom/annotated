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
}
