use crate::{FoundViolations, Location, Violations};

/// A value that can be checked against its rules.
///
/// `#[derive(Validate)]` implements it from the rules on a struct's fields. It can also be implemented by
/// hand: [`Validate::validate_into`] pushes every violation it finds, at the location of the value it
/// found it in, and [`Validate::validate`] answers them all. [`Validate::validate_fail_fast`] answers only
/// the first, and once a push leaves the report [finished](FoundViolations::is_finished), `validate_into`
/// should return, since nothing more would be kept.
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
///         if self.password.chars().count() < 8 {
///             found.push(&at.field("password"), Problem::custom("too_short", "must be 8 characters or more"));
///             if found.is_finished() {
///                 return;
///             }
///         }
///         if self.repeated_password != self.password {
///             let passwords_differ = Problem::custom("passwords_differ", "must match the password");
///             found.push(&at.field("repeatedPassword"), passwords_differ);
///         }
///     }
/// }
///
/// let new_password = NewPassword { password: "horse".into(), repeated_password: "battery".into() };
///
/// assert_eq!(
///     new_password.validate().unwrap_err().to_string(),
///     "password: must be 8 characters or more\nrepeatedPassword: must match the password"
/// );
/// assert_eq!(new_password.validate_fail_fast().unwrap_err().to_string(), "password: must be 8 characters or more");
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` doesn't implement `Validate`",
    note = "derive it with `#[derive(Validate)]`, or implement it by hand"
)]
pub trait Validate {
    /// Checks this value, found at the location `at`, and pushes every violation into `found`, returning
    /// once `found` is finished.
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

    /// Checks this value from the root, and answers only its first violation.
    ///
    /// The value is checked by the same rules, in the same order, as by [`Validate::validate`], so the
    /// violation answered is the first that `validate` answers. Checking stops right after it, but a nested
    /// value that found it doesn't stop its parent at once: the parent records nothing more and stops at its
    /// own next violation or its end.
    ///
    /// # Errors
    ///
    /// The first violation found, alone.
    fn validate_fail_fast(&self) -> Result<(), Violations> {
        let mut found = FoundViolations::failing_fast();
        self.validate_into(&Location::root(), &mut found);
        found.into_result()
    }
}
