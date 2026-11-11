use std::marker::PhantomData;

use serde::Deserializer;
use serde::de::DeserializeSeed;

use crate::{DeserializeError, FoundViolations, Location};

/// A value that is deserialized collecting the violations in it: missing fields and broken rules together,
/// instead of stopping at the first.
///
/// [`#[derive(Deserialize)]`](macro@crate::Deserialize) implements it, and serde's own `Deserialize` through it,
/// for a struct with rules on its fields. It can also be implemented by hand: [`Deserialize::deserialize_at`]
/// reads the value found at the location `at` and pushes every violation in it into `found`. It answers:
///
/// - `Ok(Some(value))` when the value could be built, though rules in it may have broken: those are in
///   `found`.
/// - `Ok(None)` when it couldn't, because a field inside is missing, say: `found` has a violation saying why. A
///   value that could be built is answered even once `found` is finished
///   [failing fast](FoundViolations::failing_fast), since the value around it may still have to check it.
/// - `Err` with the format's own error, such as a syntax error, which ends deserialization as it would end
///   serde's.
///
/// The crate implements it for the containers a nested value is held in, whenever what they hold implements it:
/// `Option<T>`, `Box<T>`, and `Box<[T]>`, `Vec<T>` and `VecDeque<T>`, whose elements are read at their index, and
/// `BTreeMap<K, V>` and `HashMap<K, V, S>`, whose values are read at their key. A container is built only when
/// everything in it is. A map reports its values' violations in the order it goes through them, and a value given
/// again for a key replaces the one before, with its violations.
///
/// [`from_deserializer`] reads a value from the root and answers it, or every violation found.
///
/// ```
/// use annotated::{FoundViolations, Location, Problem};
/// use serde::Deserializer;
///
/// #[derive(Debug)]
/// struct Handle(String);
///
/// impl<'de> annotated::Deserialize<'de> for Handle {
///     fn deserialize_at<D: Deserializer<'de>>(
///         deserializer: D,
///         at: &Location<'_>,
///         found: &mut FoundViolations,
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
    /// Reads this value, found at the location `at`, from `deserializer`, and pushes every violation in it
    /// into `found`.
    ///
    /// # Errors
    ///
    /// The format's own error, which ends deserialization.
    fn deserialize_at<D: Deserializer<'de>>(
        deserializer: D,
        at: &Location<'_>,
        found: &mut FoundViolations,
    ) -> Result<Option<Self>, D::Error>;
}

/// Reads a `T` found at a location, pushing its violations into a report: the `DeserializeSeed` a collection
/// reads its elements with, and a struct its nested fields.
pub struct DeserializeAt<'a, 'l, T> {
    at: &'a Location<'l>,
    found: &'a mut FoundViolations,
    read: PhantomData<fn() -> T>,
}

impl<'a, 'l, T> DeserializeAt<'a, 'l, T> {
    /// Reads a `T` found at `at`, pushing its violations into `found`.
    pub fn new<'de>(at: &'a Location<'l>, found: &'a mut FoundViolations) -> Self
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

/// Reads a `T` from `deserializer`, collecting every violation in it.
///
/// # Errors
///
/// [`DeserializeError::Violations`] with every violation found, in the order found, or
/// [`DeserializeError::Format`] with the format's own error, which ends deserialization at once.
pub fn from_deserializer<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<T, DeserializeError<D::Error>> {
    deserialize_reporting_to(deserializer, FoundViolations::new())
}

/// Reads a `T` from `deserializer`, and answers only its first violation.
///
/// The value is read and checked as by [`from_deserializer`], so the violation answered is the first that
/// `from_deserializer` answers. No rule runs after it, but the input is still read to its end, since a field
/// is known to be missing only then.
///
/// # Errors
///
/// [`DeserializeError::Violations`] with the first violation found, alone, or [`DeserializeError::Format`]
/// with the format's own error.
pub fn from_deserializer_fail_fast<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<T, DeserializeError<D::Error>> {
    deserialize_reporting_to(deserializer, FoundViolations::failing_fast())
}

fn deserialize_reporting_to<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
    deserializer: D,
    mut found: FoundViolations,
) -> Result<T, DeserializeError<D::Error>> {
    let value = T::deserialize_at(deserializer, &Location::root(), &mut found).map_err(DeserializeError::Format)?;
    deserialized_or_violations(value, found)
}

pub(crate) fn deserialized_or_violations<T, E: serde::de::Error>(
    value: Option<T>,
    found: FoundViolations,
) -> Result<T, DeserializeError<E>> {
    found.into_result().map_err(DeserializeError::Violations)?;
    value.ok_or_else(|| DeserializeError::Format(E::custom("the value couldn't be built, and no violation says why")))
}
