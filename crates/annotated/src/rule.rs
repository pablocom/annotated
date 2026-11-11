use crate::Problem;

/// A check a value of type `T` must pass, written in `#[validate(...)]`.
///
/// Built-in rules are made by the functions in [`rules`](crate::rules); a rule of your own is a type that
/// implements this trait, made by a function of the same name.
///
/// ```
/// use annotated::{Problem, Rule, rules};
///
/// assert_eq!(rules::required().check("Lucía"), Ok(()));
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
    /// The [`Problem`] with `value`, when it breaks this rule.
    fn check(&self, value: &T) -> Result<(), Problem>;
}
