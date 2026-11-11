use crate::{FoundViolations, Location, Violations};

/// A value that can be checked against its rules.
///
/// `#[derive(Validate)]` implements it from the rules on a struct's fields. It can also be implemented by
/// hand: [`Validate::validate_into`] pushes every violation it finds, at the location of the value it
/// found it in, and [`Validate::validate`] answers them all.
///
/// ```
/// use annotated::{FoundViolations, Location, Problem, Validate};
///
/// struct NewPassword {
///     password: String,
///     repeated_password: String,
/// }
///
/// impl Validate for NewPassword {
///     fn validate_into(&self, at: &Location<'_>, found: &mut FoundViolations) {
///         if self.repeated_password != self.password {
///             let passwords_differ = Problem::custom("passwords_differ", "must match the password");
///             found.push(&at.field("repeatedPassword"), passwords_differ);
///         }
///     }
/// }
///
/// let new_password = NewPassword { password: "correct horse battery staple".into(), repeated_password: "correct horse".into() };
///
/// let violations = new_password.validate().unwrap_err();
/// assert_eq!(violations.to_string(), "repeatedPassword: must match the password");
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` doesn't implement `Validate`",
    note = "derive it with `#[derive(Validate)]`, or implement it by hand"
)]
pub trait Validate {
    /// Checks this value, found at the location `at`, and pushes every violation into `found`.
    fn validate_into(&self, at: &Location<'_>, found: &mut FoundViolations);

    /// Checks this value from the root.
    ///
    /// # Errors
    ///
    /// Every violation found, in the order it was found.
    fn validate(&self) -> Result<(), Violations> {
        let mut found = FoundViolations::new();
        self.validate_into(&Location::root(), &mut found);
        found.into_result()
    }
}
