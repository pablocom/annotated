//! Annotate structs with validation rules and get every problem at once, missing fields included.
//!
//! A value is checked with [`Validate::validate`], which answers `Ok(())` or every [`Violation`] found,
//! each a [`Problem`] at the [`Path`] where it happened.

mod found_violations;
mod location;
mod path;
mod problem;
mod validate;
mod violation;
mod violations;

pub use found_violations::FoundViolations;
pub use location::Location;
pub use path::{Path, PathSegment};
pub use problem::{ParamValue, Problem};
pub use validate::Validate;
pub use violation::Violation;
pub use violations::Violations;
