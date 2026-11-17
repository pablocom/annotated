//! Reading JSON with collecting deserialization: every missing field and broken rule in one error.
//!
//! These functions read a whole JSON document, as `serde_json::from_str` and `from_slice` do. So characters after the
//! value are the format's error, and that error is returned before any issue.
//!
//! [`from_str`] and [`from_slice`] keep up to [`Report::DEFAULT_LIMIT`] issues, the `_limited_to` functions as many
//! as they are given, and the `_fail_fast` functions only the first.
//!
//! ```
//! use annotated::Deserialize;
//!
//! #[derive(Deserialize, Debug)]
//! struct LogIn {
//!     #[validate(email)]
//!     email: String,
//!     #[validate(required)]
//!     password: String,
//! }
//!
//! let error = annotated::json::from_str::<LogIn>(r#"{ "email": "alex" }"#).unwrap_err();
//!
//! assert_eq!(error.to_string(), "email: must be an email address\npassword: is missing");
//! ```

use serde_json::de::{Read, SliceRead, StrRead};

use crate::deserialize::deserialized_or_issues;
use crate::{Deserialize, DeserializeError, Location, Report};

/// Reads a `T` from JSON text, collecting its issues: every one, up to [`Report::DEFAULT_LIMIT`].
///
/// For a value with more issues than that, it returns as many, as [`from_str_limited_to`] does.
///
/// # Errors
///
/// Returns [`DeserializeError::Issues`] with the issues found, in the order found, or
/// [`DeserializeError::Format`] when the text isn't a JSON document that `T` can be read from.
pub fn from_str<'de, T: Deserialize<'de>>(json: &'de str) -> Result<T, DeserializeError<serde_json::Error>> {
    read_whole_document(StrRead::new(json), Report::new())
}

/// Reads a `T` from JSON bytes, collecting its issues: every one, up to [`Report::DEFAULT_LIMIT`].
///
/// For a value with more issues than that, it returns as many, as [`from_slice_limited_to`] does.
///
/// # Errors
///
/// Returns [`DeserializeError::Issues`] with the issues found, in the order found, or
/// [`DeserializeError::Format`] when the bytes aren't a JSON document that `T` can be read from.
pub fn from_slice<'de, T: Deserialize<'de>>(json: &'de [u8]) -> Result<T, DeserializeError<serde_json::Error>> {
    read_whole_document(SliceRead::new(json), Report::new())
}

/// Reads a `T` from JSON text, and returns only its first issue: the one [`from_str`] would list first.
///
/// # Errors
///
/// Returns [`DeserializeError::Issues`] with the first issue found, alone, or [`DeserializeError::Format`] when
/// the text isn't a JSON document that `T` can be read from.
pub fn from_str_fail_fast<'de, T: Deserialize<'de>>(json: &'de str) -> Result<T, DeserializeError<serde_json::Error>> {
    read_whole_document(StrRead::new(json), Report::failing_fast())
}

/// Reads a `T` from JSON bytes, and returns only its first issue: the one [`from_slice`] would list first.
///
/// # Errors
///
/// Returns [`DeserializeError::Issues`] with the first issue found, alone, or [`DeserializeError::Format`] when
/// the bytes aren't a JSON document that `T` can be read from.
pub fn from_slice_fail_fast<'de, T: Deserialize<'de>>(
    json: &'de [u8],
) -> Result<T, DeserializeError<serde_json::Error>> {
    read_whole_document(SliceRead::new(json), Report::failing_fast())
}

/// Reads a `T` from JSON text, collecting at most `most_issues` of its issues, as a
/// [report limited to them](Report::limited_to) keeps them.
///
/// The issues returned are listed in the order [`from_str`] would list them, and are its first ones unless a map
/// holds them. A limit of 0 is taken as 1. When the value has as many issues as the limit or more,
/// [`Issues::is_complete`](crate::Issues::is_complete) is `false`.
///
/// ```
/// use annotated::Deserialize;
///
/// #[derive(Deserialize, Debug)]
/// struct LogIn {
///     #[validate(email)]
///     email: String,
///     #[validate(required)]
///     password: String,
/// }
///
/// let error = annotated::json::from_str_limited_to::<LogIn>(r#"{ "email": "alex" }"#, 1).unwrap_err();
///
/// assert_eq!(error.to_string(), "email: must be an email address");
/// ```
///
/// # Errors
///
/// Returns [`DeserializeError::Issues`] with the issues found, at most `most_issues`, or
/// [`DeserializeError::Format`] when the text isn't a JSON document that `T` can be read from.
pub fn from_str_limited_to<'de, T: Deserialize<'de>>(
    json: &'de str,
    most_issues: usize,
) -> Result<T, DeserializeError<serde_json::Error>> {
    read_whole_document(StrRead::new(json), Report::limited_to(most_issues))
}

/// Reads a `T` from JSON bytes, collecting at most `most_issues` of its issues, as [`from_str_limited_to`] does.
///
/// # Errors
///
/// Returns [`DeserializeError::Issues`] with the issues found, at most `most_issues`, or
/// [`DeserializeError::Format`] when the bytes aren't a JSON document that `T` can be read from.
pub fn from_slice_limited_to<'de, T: Deserialize<'de>>(
    json: &'de [u8],
    most_issues: usize,
) -> Result<T, DeserializeError<serde_json::Error>> {
    read_whole_document(SliceRead::new(json), Report::limited_to(most_issues))
}

fn read_whole_document<'de, R: Read<'de>, T: Deserialize<'de>>(
    json: R,
    mut found: Report,
) -> Result<T, DeserializeError<serde_json::Error>> {
    let mut deserializer = serde_json::Deserializer::new(json);
    // The value stays inside `read` across `end()` on purpose: moved out first, it was copied twice more, and
    // reading a valid 8-field body without LTO took 5% longer.
    let read = T::deserialize_at(&mut deserializer, &Location::root(), &mut found);
    if read.is_ok()
        && let Err(format_error) = deserializer.end()
    {
        return Err(DeserializeError::Format(format_error));
    }
    match read {
        Ok(value) => deserialized_or_issues(value, found),
        Err(format_error) => Err(DeserializeError::Format(format_error)),
    }
}
