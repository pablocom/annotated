//! The built-in rules.
//!
//! A function named after each rule makes it, like [`length`], and builder methods set it up, like [`Length::max`].
//! `#[validate(length(max = 50))]` becomes `annotated::rules::length().max(50)`, so you can use a rule by hand in the
//! same way, through [`Rule::check`](crate::Rule::check).
//!
//! Rules are generic over small capability traits, like [`Required`] and [`HasLength`]. Implement one for a type of
//! your own, and the rules that use it check that type too.

mod custom;
mod elements;
mod email;
mod ip;
mod length;
#[cfg(feature = "regex")]
mod pattern;
mod range;
mod required;
mod text;
mod url;

pub use custom::{Custom, custom};
pub use elements::HasElements;
pub use email::{Email, email};
pub use ip::{Ip, ip, ipv4, ipv6};
pub use length::{HasLength, Length, StringLength, length};
#[cfg(feature = "regex")]
pub use pattern::{Pattern, pattern};
pub use range::{Range, range};
pub use required::{Presence, Required, required};
pub use text::{Contains, Prefix, Suffix, contains, prefix, suffix};
pub use url::{Url, url};
