//! Validation for structs: write rules on their fields, and get every problem back at once.
//!
//! A value is checked with [`Validate::validate`], which answers `Ok(())` or every [`Violation`] found,
//! each a [`Problem`] at the [`Path`] where it happened, or with [`Validate::validate_fail_fast`], which
//! answers only the first violation: the first that `validate` answers. Fields are reported by the names
//! serde reads them by.

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod rules;

#[doc(hidden)]
pub mod __private;

#[cfg(feature = "serde")]
mod deserialize;
#[cfg(feature = "serde")]
mod deserialize_error;
mod found_violations;
mod location;
mod param_value;
mod path;
mod problem;
mod rule;
mod validate;
mod violation;
mod violations;

#[cfg(feature = "serde")]
pub use deserialize::{Deserialize, from_deserializer, from_deserializer_fail_fast};
#[cfg(feature = "serde")]
pub use deserialize_error::DeserializeError;
pub use found_violations::FoundViolations;
pub use location::Location;
pub use param_value::ParamValue;
pub use path::{Path, PathSegment};
pub use problem::{Bounds, LengthUnit, Problem};
pub use rule::Rule;
pub use validate::Validate;

/// Implements [`Validate`] from the `#[validate(...)]` rules on a struct's fields.
///
/// ```
/// use annotated::Validate;
///
/// #[derive(Validate)]
/// #[serde(rename_all = "camelCase")]
/// struct NewMember {
///     #[validate(required, length(max = 50))]
///     full_name: String,
/// }
///
/// let member = NewMember { full_name: "  ".to_owned() };
///
/// assert_eq!(member.validate().unwrap_err().to_string(), "fullName: is required");
/// ```
///
/// Each entry in `#[validate(...)]` is a rule, checked in the order written. A single name is a built-in rule
/// from [`rules`]: `required` becomes `annotated::rules::required()`, and `length(max = 50)` calls the
/// builder method `max` with `50`: `annotated::rules::length().max(50)`. `message = "..."` in a rule's
/// arguments replaces its problem's message, with `{param}` placeholders filled from the problem's
/// parameters. `length` takes `unit = "chars"`, `"bytes"` or, with the `unicode` feature, `"graphemes"`, and
/// refuses a unit on a collection. `pattern`, with the `regex` feature, takes its regular expression as a
/// string literal, refuses one that doesn't parse, and compiles it once.
///
/// - Every field is checked, and the first rule a field breaks stops that field's other rules.
///   `#[validate(report_all)]` on the struct reports every rule a field breaks instead.
/// - [`Validate::validate_fail_fast`] checks the same rules in the same order, nested values and elements
///   included, and answers only the first violation, even under `report_all`. It stops right after it,
///   except that a `nested` value or element finding it doesn't stop the struct around it at once: that
///   struct records nothing more and stops at its own next violation or its end.
/// - On an `Option` field, `required` checks the `Option` itself; every other rule checks the value inside,
///   and is skipped when there is none.
/// - `nested` validates a field whose type implements [`Validate`], under the field's name.
/// - `each(...)` applies its rules to every element of a collection, at its index, or every value of a map, at
///   its key: anything implementing [`rules::HasElements`]. `each(nested)` validates every element.
/// - Fields are reported by the name serde reads them by, from `#[serde(...)]` attributes such as
///   `rename_all` and `rename`. The struct doesn't need to derive serde's traits.
///
/// `#[validate(custom(check))]` on the struct checks the whole struct with any `Fn(&Self) -> Result<(),
/// Problem>`, and reports at the struct's own location: the root, or the field it is `nested` under.
/// `custom(check, field = name)` reports at that field instead, by its serde name, and `message = "..."`
/// replaces the problem's message. Struct-level rules are checked after every field, in the order written,
/// beside `report_all` in the same or another `#[validate(...)]`. One at a field counts as that field's last
/// rule: it is skipped once the field has a violation, and the first one there that fails stops the later
/// ones, unless the struct has `report_all`. One at the struct itself always runs.
///
/// ```
/// use annotated::{Problem, Validate};
///
/// #[derive(Validate)]
/// #[serde(rename_all = "camelCase")]
/// #[validate(custom(passwords_match, field = repeated_password))]
/// struct NewPassword {
///     #[validate(length(min = 8))]
///     password: String,
///     repeated_password: String,
/// }
///
/// fn passwords_match(new_password: &NewPassword) -> Result<(), Problem> {
///     if new_password.repeated_password != new_password.password {
///         return Err(Problem::custom("passwords_differ", "must match the password"));
///     }
///     Ok(())
/// }
///
/// let new_password = NewPassword {
///     password: "correct horse battery staple".to_owned(),
///     repeated_password: "correct horse".to_owned(),
/// };
///
/// assert_eq!(new_password.validate().unwrap_err().to_string(), "repeatedPassword: must match the password");
/// ```
pub use annotated_derive::Validate;
pub use violation::Violation;
pub use violations::Violations;
