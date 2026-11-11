//! Annotate structs with validation rules and get every problem at once, missing fields included.
//!
//! A value is checked with [`Validate::validate`], which answers `Ok(())` or every [`Violation`] found,
//! each a [`Problem`] at the [`Path`] where it happened.

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod rules;

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
pub use violation::Violation;
pub use violations::Violations;
