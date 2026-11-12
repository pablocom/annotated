use std::marker::PhantomData;

use serde::Deserializer;
use serde::de::DeserializeSeed;

use crate::{DeserializeError, Location, Report};

/// A value that can be deserialized while collecting the issues in it: missing fields and broken rules together,
/// instead of stopping at the first.
///
/// [`#[derive(Deserialize)]`](macro@crate::Deserialize) implements it, and serde's own `Deserialize` through it, for a
/// struct with rules on its fields. It can also be implemented by hand. [`Deserialize::deserialize_at`] reads the value
/// at the location `at`, pushes every issue in it into `found`, and returns:
///
/// - `Ok(Some(value))` when the value could be built. Rules in it may still have been broken, and those issues are
///   in `found`.
/// - `Ok(None)` when the value couldn't be built, for example because a field inside it is missing. `found` has an
///   issue that says why.
/// - `Err` with the format's own error, such as a syntax error. It ends deserialization, as it would end serde's.
///
/// A value that could be built is returned even when `found` is already finished from
/// [failing fast](Report::failing_fast), because the value around it may still have to check it.
///
/// The crate implements the trait for the containers that hold a nested value, whenever what they hold implements it:
///
/// - `Option<T>` and `Box<T>`.
/// - `Box<[T]>`, `Vec<T>` and `VecDeque<T>`, whose elements are read at their index.
/// - `BTreeMap<K, V>` and `HashMap<K, V, S>`, whose values are read at their key.
///
/// A container is built only when everything in it could be built. A map reports its values' issues in the order it
/// iterates over them. When a key appears twice, the later value replaces the earlier one, along with its issues.
///
/// [`from_deserializer`] reads a value from the root, and returns it or every issue found.
///
/// ```
/// use annotated::{Report, Location, Problem};
/// use serde::Deserializer;
///
/// #[derive(Debug)]
/// struct Handle(String);
///
/// impl<'de> annotated::Deserialize<'de> for Handle {
///     fn deserialize_at<D: Deserializer<'de>>(
///         deserializer: D,
///         at: &Location<'_>,
///         found: &mut Report,
///     ) -> Result<Option<Self>, D::Error> {
///         let handle: String = serde::Deserialize::deserialize(deserializer)?;
///         if !handle.starts_with('@') {
///             found.push(at, Problem::custom("handle", "must start with @"));
///         }
///         Ok(Some(Self(handle)))
///     }
/// }
///
/// let mut deserializer = serde_json::Deserializer::from_str(r#""lucia""#);
/// let error = annotated::from_deserializer::<Handle, _>(&mut deserializer).unwrap_err();
///
/// assert_eq!(error.to_string(), "must start with @");
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` doesn't implement `annotated::Deserialize`",
    note = "derive it with `#[derive(annotated::Deserialize)]`, or implement it by hand"
)]
pub trait Deserialize<'de>: Sized {
    /// Reads this value, which is at the location `at`, from `deserializer`, and pushes every issue in it into
    /// `found`.
    ///
    /// # Errors
    ///
    /// Returns the format's own error, which ends deserialization.
    fn deserialize_at<D: Deserializer<'de>>(
        deserializer: D,
        at: &Location<'_>,
        found: &mut Report,
    ) -> Result<Option<Self>, D::Error>;
}

/// A `DeserializeSeed` that reads a `T` at a location and pushes its issues into a report. A collection reads its
/// elements with it, and a struct its nested fields.
pub struct DeserializeAt<'a, 'l, T> {
    at: &'a Location<'l>,
    found: &'a mut Report,
    read: PhantomData<fn() -> T>,
}

impl<'a, 'l, T> DeserializeAt<'a, 'l, T> {
    /// A seed that reads a `T` at `at` and pushes its issues into `found`.
    pub fn new<'de>(at: &'a Location<'l>, found: &'a mut Report) -> Self
    where
        T: Deserialize<'de>,
    {
        Self {
            at,
            found,
            read: PhantomData,
        }
    }
}

impl<'de, T: Deserialize<'de>> DeserializeSeed<'de> for DeserializeAt<'_, '_, T> {
    type Value = Option<T>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Option<T>, D::Error> {
        T::deserialize_at(deserializer, self.at, self.found)
    }
}

/// Reads a `T` from `deserializer`, collecting every issue in it.
///
/// # Errors
///
/// Returns [`DeserializeError::Issues`] with every issue found, in the order found, or
/// [`DeserializeError::Format`] with the format's own error, which ends deserialization at once.
pub fn from_deserializer<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<T, DeserializeError<D::Error>> {
    deserialize_reporting_to(deserializer, Report::new())
}

/// Reads a `T` from `deserializer`, and returns only its first issue.
///
/// The value is read and checked as by [`from_deserializer`], so the issue returned is the one `from_deserializer`
/// would list first. The input is still read to its end, because a field is known to be missing only then. Once it is
/// read, checking stops at the first issue. A nested value is checked while it is read, so its rules may still run.
///
/// # Errors
///
/// Returns [`DeserializeError::Issues`] with the first issue found, alone, or [`DeserializeError::Format`] with
/// the format's own error.
pub fn from_deserializer_fail_fast<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<T, DeserializeError<D::Error>> {
    deserialize_reporting_to(deserializer, Report::failing_fast())
}

fn deserialize_reporting_to<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
    deserializer: D,
    mut found: Report,
) -> Result<T, DeserializeError<D::Error>> {
    let value = T::deserialize_at(deserializer, &Location::root(), &mut found).map_err(DeserializeError::Format)?;
    deserialized_or_issues(value, found)
}

pub(crate) fn deserialized_or_issues<T, E: serde::de::Error>(
    value: Option<T>,
    found: Report,
) -> Result<T, DeserializeError<E>> {
    found.into_result().map_err(DeserializeError::Issues)?;
    value.ok_or_else(|| DeserializeError::Format(E::custom("the value couldn't be built, and no issue says why")))
}
