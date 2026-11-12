use crate::{FoundIssues, Issues, Location};

/// A value that can be checked against its rules.
///
/// `#[derive(Validate)]` implements it from the rules on a struct's fields. It can also be implemented by hand:
///
/// - [`Validate::validate_into`] pushes every issue it finds into the report, each at the location of the value it
///   was found in.
/// - [`Validate::validate`] returns all of them, and [`Validate::validate_fail_fast`] only the first.
/// - When failing fast, the report is [finished](FoundIssues::is_finished) after its first issue and keeps
///   nothing more. So once a push leaves the report finished, `validate_into` should return.
///
/// ```
/// use annotated::{FoundIssues, Location, Problem, Validate};
///
/// struct NewPassword {
///     password: String,
///     repeated_password: String,
/// }
///
/// impl Validate for NewPassword {
///     fn validate_into(&self, at: &Location<'_>, found: &mut FoundIssues) {
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
    /// Checks this value, which is at the location `at`, and pushes every issue into `found`. It should return as
    /// soon as `found` is finished.
    fn validate_into(&self, at: &Location<'_>, found: &mut FoundIssues);

    /// Checks this value, starting at the root.
    ///
    /// # Errors
    ///
    /// Returns every issue found, in the order they were found.
    fn validate(&self) -> Result<(), Issues> {
        let mut found = FoundIssues::new();
        self.validate_into(&Location::root(), &mut found);
        found.into_result()
    }

    /// Checks this value, starting at the root, and returns only its first issue.
    ///
    /// The value is checked by the same rules, in the same order, as by [`Validate::validate`]. So the issue
    /// returned is the one `validate` would list first. Checking stops right after it, with one exception: when a
    /// nested value finds it, the parent may check more before it stops. It records nothing more.
    ///
    /// # Errors
    ///
    /// Returns the first issue found, alone.
    fn validate_fail_fast(&self) -> Result<(), Issues> {
        let mut found = FoundIssues::failing_fast();
        self.validate_into(&Location::root(), &mut found);
        found.into_result()
    }
}
