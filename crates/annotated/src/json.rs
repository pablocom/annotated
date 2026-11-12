//! Reading JSON with collecting deserialization: every missing field and broken rule in one error.
//!
//! These functions read a whole JSON document, as `serde_json::from_str` and `from_slice` do. So characters after the
//! value are the format's error, and that error is returned before any issue.
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
use crate::{Deserialize, DeserializeError, FoundIssues, Location};

/// Reads a `T` from JSON text, collecting every issue in it.
///
/// # Errors
///
/// Returns [`DeserializeError::Issues`] with every issue found, in the order found, or
/// [`DeserializeError::Format`] when the text isn't a JSON document that `T` can be read from.
pub fn from_str<'de, T: Deserialize<'de>>(json: &'de str) -> Result<T, DeserializeError<serde_json::Error>> {
    read_whole_document(StrRead::new(json), FoundIssues::new())
}

/// Reads a `T` from JSON bytes, collecting every issue in it.
///
/// # Errors
///
/// Returns [`DeserializeError::Issues`] with every issue found, in the order found, or
/// [`DeserializeError::Format`] when the bytes aren't a JSON document that `T` can be read from.
pub fn from_slice<'de, T: Deserialize<'de>>(json: &'de [u8]) -> Result<T, DeserializeError<serde_json::Error>> {
    read_whole_document(SliceRead::new(json), FoundIssues::new())
}

/// Reads a `T` from JSON text, and returns only its first issue: the one [`from_str`] would list first.
///
/// # Errors
///
/// Returns [`DeserializeError::Issues`] with the first issue found, alone, or [`DeserializeError::Format`] when
/// the text isn't a JSON document that `T` can be read from.
pub fn from_str_fail_fast<'de, T: Deserialize<'de>>(json: &'de str) -> Result<T, DeserializeError<serde_json::Error>> {
    read_whole_document(StrRead::new(json), FoundIssues::failing_fast())
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
    read_whole_document(SliceRead::new(json), FoundIssues::failing_fast())
}

fn read_whole_document<'de, R: Read<'de>, T: Deserialize<'de>>(
    json: R,
    mut found: FoundIssues,
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
