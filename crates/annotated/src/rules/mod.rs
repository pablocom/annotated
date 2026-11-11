//! The built-in rules.
//!
//! Each rule is made by a function named after it, like [`length`], and set up with builder methods, like
//! [`Length::max`]. `#[validate(length(max = 50))]` becomes `annotated::rules::length().max(50)`, so a rule
//! can be used by hand just the same, through [`Rule::check`](crate::Rule::check).
//!
//! Rules are generic over small capability traits, like [`Required`] and [`HasLength`], so they check your
//! own types once those implement them.

mod custom;
mod email;
mod ip;
mod length;
#[cfg(feature = "regex")]
mod pattern;
mod range;
mod required;
mod text;
#[cfg(feature = "url")]
mod url;

pub use custom::{Custom, custom};
pub use email::{Email, email};
pub use ip::{Ip, ip};
pub use length::{HasLength, Length, StringLength, length};
#[cfg(feature = "regex")]
pub use pattern::{Pattern, pattern};
pub use range::{Range, range};
pub use required::{Presence, Required, required};
pub use text::{Contains, Prefix, Suffix, contains, prefix, suffix};
#[cfg(feature = "url")]
pub use url::{Url, url};
