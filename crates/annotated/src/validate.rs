use std::borrow::Cow;
use std::rc::Rc;
use std::sync::Arc;

use crate::{Issues, Location, Report};

/// A value that can be checked against its rules.
///
/// `#[derive(Validate)]` implements it from the rules on a struct's fields. It can also be implemented by hand:
///
/// - [`Validate::validate_into`] pushes every issue it finds into the report, each at the location of the value it
///   was found in.
/// - [`Validate::validate`] returns them, up to [`Report::DEFAULT_LIMIT`]. [`Validate::validate_limited_to`] returns
///   as many as it is given, and [`Validate::validate_fail_fast`] only the first.
/// - A report is [finished](Report::is_finished) once it holds as many issues as its limit, after its first when
///   failing fast, and keeps nothing more. So once a push leaves the report finished, `validate_into` should return.
///
/// The crate implements it for what holds a value that implements it, so that `nested` checks the value inside:
///
/// - `Box<T>`, `Rc<T>`, `Arc<T>`, `&T`, `&mut T` and `Cow<'_, T>` check the value they point to, at their own
///   location. With the `json-schema` feature, their `rules()` are the value's.
/// - A tuple of up to 16 elements checks each element at its index, as a list's elements are: `leg[1].street`.
///
/// A collection's elements are checked with `each(nested)`, through [`HasElements`](crate::rules::HasElements).
///
/// ```
/// use annotated::{Report, Location, Problem, Validate};
///
/// struct NewPassword {
///     password: String,
///     repeated_password: String,
/// }
///
/// impl Validate for NewPassword {
///     fn validate_into(&self, at: &Location<'_>, found: &mut Report) {
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
    fn validate_into(&self, at: &Location<'_>, found: &mut Report);

    /// Checks this value, starting at the root, and returns its issues: every one, up to [`Report::DEFAULT_LIMIT`].
    ///
    /// For a value with more issues than that, it returns as many, as [`Validate::validate_limited_to`] does.
    ///
    /// # Errors
    ///
    /// Returns the issues found, in the order they were found.
    fn validate(&self) -> Result<(), Issues> {
        validated_reporting_to(self, Report::new())
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
        validated_reporting_to(self, Report::failing_fast())
    }

    /// Checks this value, starting at the root, and returns at most `most_issues` of its issues: the first ones
    /// [`Validate::validate`] would list, in its order.
    ///
    /// A limit of 0 is taken as 1. Checking stops once the limit is reached, with the exception that
    /// [`Validate::validate_fail_fast`] describes. When the value has as many issues as the limit or more,
    /// [`Issues::is_complete`] is `false`.
    ///
    /// # Errors
    ///
    /// Returns the issues found, at most `most_issues`.
    fn validate_limited_to(&self, most_issues: usize) -> Result<(), Issues> {
        validated_reporting_to(self, Report::limited_to(most_issues))
    }

    /// This type's rules, as data: what `#[validate(...)]` requires of each of its fields.
    ///
    /// `#[derive(Validate)]` implements it for a struct. A type that implements `Validate` by hand has no rules as
    /// data, unless it implements this too.
    #[cfg(feature = "json-schema")]
    #[must_use]
    fn rules() -> crate::Rules
    where
        Self: Sized,
    {
        crate::Rules::default()
    }
}

macro_rules! validates_the_value_it_points_to {
    ($($pointer:ty),*) => {
        $(
            impl<T: Validate> Validate for $pointer {
                fn validate_into(&self, at: &Location<'_>, found: &mut Report) {
                    T::validate_into(self, at, found);
                }

                #[cfg(feature = "json-schema")]
                fn rules() -> crate::Rules {
                    T::rules()
                }
            }
        )*
    };
}

validates_the_value_it_points_to!(Box<T>, Rc<T>, Arc<T>, &T, &mut T);

impl<T: Validate + ToOwned> Validate for Cow<'_, T> {
    fn validate_into(&self, at: &Location<'_>, found: &mut Report) {
        T::validate_into(self, at, found);
    }

    #[cfg(feature = "json-schema")]
    fn rules() -> crate::Rules {
        T::rules()
    }
}

macro_rules! validates_each_element_at_its_index {
    ($(($($index:tt $element:ident),+))+) => {
        $(
            #[diagnostic::do_not_recommend]
            impl<$($element: Validate),+> Validate for ($($element,)+) {
                fn validate_into(&self, at: &Location<'_>, found: &mut Report) {
                    $(
                        self.$index.validate_into(&at.index($index), found);
                    )+
                }
            }
        )+
    };
}

validates_each_element_at_its_index! {
    (0 T0)
    (0 T0, 1 T1)
    (0 T0, 1 T1, 2 T2)
    (0 T0, 1 T1, 2 T2, 3 T3)
    (0 T0, 1 T1, 2 T2, 3 T3, 4 T4)
    (0 T0, 1 T1, 2 T2, 3 T3, 4 T4, 5 T5)
    (0 T0, 1 T1, 2 T2, 3 T3, 4 T4, 5 T5, 6 T6)
    (0 T0, 1 T1, 2 T2, 3 T3, 4 T4, 5 T5, 6 T6, 7 T7)
    (0 T0, 1 T1, 2 T2, 3 T3, 4 T4, 5 T5, 6 T6, 7 T7, 8 T8)
    (0 T0, 1 T1, 2 T2, 3 T3, 4 T4, 5 T5, 6 T6, 7 T7, 8 T8, 9 T9)
    (0 T0, 1 T1, 2 T2, 3 T3, 4 T4, 5 T5, 6 T6, 7 T7, 8 T8, 9 T9, 10 T10)
    (0 T0, 1 T1, 2 T2, 3 T3, 4 T4, 5 T5, 6 T6, 7 T7, 8 T8, 9 T9, 10 T10, 11 T11)
    (0 T0, 1 T1, 2 T2, 3 T3, 4 T4, 5 T5, 6 T6, 7 T7, 8 T8, 9 T9, 10 T10, 11 T11, 12 T12)
    (0 T0, 1 T1, 2 T2, 3 T3, 4 T4, 5 T5, 6 T6, 7 T7, 8 T8, 9 T9, 10 T10, 11 T11, 12 T12, 13 T13)
    (0 T0, 1 T1, 2 T2, 3 T3, 4 T4, 5 T5, 6 T6, 7 T7, 8 T8, 9 T9, 10 T10, 11 T11, 12 T12, 13 T13, 14 T14)
    (0 T0, 1 T1, 2 T2, 3 T3, 4 T4, 5 T5, 6 T6, 7 T7, 8 T8, 9 T9, 10 T10, 11 T11, 12 T12, 13 T13, 14 T14, 15 T15)
}

fn validated_reporting_to<V: Validate + ?Sized>(value: &V, mut found: Report) -> Result<(), Issues> {
    value.validate_into(&Location::root(), &mut found);
    found.into_result()
}
