//! The built-in rules.
//!
//! Each rule is made by a function named after it, like [`length`], and set up with builder methods, like
//! [`Length::max`]. `#[validate(length(max = 50))]` becomes `annotated::rules::length().max(50)`, so a rule
//! can be used by hand just the same, through [`Rule::check`](crate::Rule::check).
//!
//! Rules are generic over small capability traits, like [`Required`] and [`HasLength`], so they check your
//! own types once those implement them.

mod length;
mod required;

#[cfg(feature = "unicode")]
pub use length::LengthInGraphemes;
pub use length::{HasLength, Length, LengthInBytes, LengthInChars, length};
pub use required::{Presence, Required, required};
