//! Annotate structs with validation rules and get every problem at once, missing fields included.
//!
//! A value is checked with [`Validate::validate`], which answers `Ok(())` or every [`Violation`] found,
//! each a [`Problem`] at the [`Path`] where it happened.

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod rules;

#[doc(hidden)]
pub mod __private;

mod found_violations;
mod location;
mod path;
mod problem;
mod rule;
mod validate;
mod violation;
mod violations;

pub use found_violations::FoundViolations;
pub use location::Location;
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
/// refuses a unit on a collection.
///
/// - Every field is checked, and the first rule a field breaks stops that field's other rules.
///   `#[validate(report_all)]` on the struct reports every rule a field breaks instead.
/// - On an `Option` field, `required` checks the `Option` itself; every other rule checks the value inside,
///   and is skipped when there is none.
/// - Fields are reported by the name serde reads them by, from `#[serde(...)]` attributes such as
///   `rename_all` and `rename`. The struct doesn't need to derive serde's traits.
pub use annotated_derive::Validate;
pub use violation::Violation;
pub use violations::Violations;
