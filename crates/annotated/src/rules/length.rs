use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::rc::Rc;
use std::sync::Arc;

use crate::{Bounds, LengthUnit, Problem, Rule};

/// The rule that a value's length is within bounds: `length().min(1).max(50)`.
///
/// It counts the `char`s of a string (Unicode scalar values) and the elements of a collection, anything that
/// implements [`HasLength`]. A string can be counted in another unit with [`Length::in_bytes`],
/// [`Length::in_chars`] or, with the `unicode` feature, `Length::in_graphemes`; a collection can't. Bounds
/// are inclusive, and a length with no bounds accepts any value.
///
/// ```
/// use annotated::{Rule, rules};
///
/// assert_eq!(rules::length().max(5).check("Renée"), Ok(()));
///
/// let too_long_in_bytes = rules::length().max(5).in_bytes().check("Renée").unwrap_err();
/// assert_eq!(too_long_in_bytes.to_string(), "must be at most 5 bytes long");
/// ```
#[must_use]
pub const fn length() -> Length {
    Length {
        bounds: LengthBounds { min: None, max: None },
    }
}

/// Checks the length of a string, in `char`s, or of a collection, in elements; made by [`length`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Length {
    bounds: LengthBounds,
}

/// Checks the length of a string in `char`s; made by [`Length::in_chars`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LengthInChars {
    bounds: LengthBounds,
}

/// Checks the length of a string in UTF-8 bytes; made by [`Length::in_bytes`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LengthInBytes {
    bounds: LengthBounds,
}

/// Checks the length of a string in grapheme clusters, what a person sees as one character; made by
/// [`Length::in_graphemes`].
#[cfg(feature = "unicode")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LengthInGraphemes {
    bounds: LengthBounds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LengthBounds {
    min: Option<usize>,
    max: Option<usize>,
}

impl LengthBounds {
    const fn with_min(self, min: usize) -> Self {
        Self { min: Some(min), ..self }
    }

    const fn with_max(self, max: usize) -> Self {
        Self { max: Some(max), ..self }
    }

    fn check(self, length: usize, unit: LengthUnit) -> Result<(), Problem> {
        let is_long_enough = self.min.is_none_or(|min| length >= min);
        let is_short_enough = self.max.is_none_or(|max| length <= max);
        if is_long_enough && is_short_enough {
            return Ok(());
        }
        let bounds = Bounds::of(self.min, self.max).expect("a length out of bounds has bounds");
        Err(Problem::Length { bounds, unit })
    }
}

macro_rules! length_bounds {
    ($rule:ident) => {
        impl $rule {
            /// The rule with `min` as the shortest length it accepts.
            #[must_use]
            pub const fn min(self, min: usize) -> Self {
                Self {
                    bounds: self.bounds.with_min(min),
                }
            }

            /// The rule with `max` as the longest length it accepts.
            #[must_use]
            pub const fn max(self, max: usize) -> Self {
                Self {
                    bounds: self.bounds.with_max(max),
                }
            }

            /// The rule accepting only the length `equal`.
            #[must_use]
            pub const fn equal(self, equal: usize) -> Self {
                Self {
                    bounds: self.bounds.with_min(equal).with_max(equal),
                }
            }
        }
    };
}

length_bounds!(Length);
length_bounds!(LengthInChars);
length_bounds!(LengthInBytes);
#[cfg(feature = "unicode")]
length_bounds!(LengthInGraphemes);

impl Length {
    /// The rule counting a string's `char`s, and refusing to check a collection.
    #[must_use]
    pub const fn in_chars(self) -> LengthInChars {
        LengthInChars { bounds: self.bounds }
    }

    /// The rule counting a string's UTF-8 bytes, and refusing to check a collection.
    #[must_use]
    pub const fn in_bytes(self) -> LengthInBytes {
        LengthInBytes { bounds: self.bounds }
    }

    /// The rule counting a string's grapheme clusters, what a person sees as one character, and refusing to
    /// check a collection.
    #[cfg(feature = "unicode")]
    #[must_use]
    pub const fn in_graphemes(self) -> LengthInGraphemes {
        LengthInGraphemes { bounds: self.bounds }
    }
}

impl<T: HasLength + ?Sized> Rule<T> for Length {
    fn check(&self, value: &T) -> Result<(), Problem> {
        self.bounds.check(value.length(), T::UNIT)
    }
}

impl<T: AsRef<str> + ?Sized> Rule<T> for LengthInChars {
    fn check(&self, value: &T) -> Result<(), Problem> {
        self.bounds.check(value.as_ref().chars().count(), LengthUnit::Chars)
    }
}

impl<T: AsRef<str> + ?Sized> Rule<T> for LengthInBytes {
    fn check(&self, value: &T) -> Result<(), Problem> {
        self.bounds.check(value.as_ref().len(), LengthUnit::Bytes)
    }
}

#[cfg(feature = "unicode")]
impl<T: AsRef<str> + ?Sized> Rule<T> for LengthInGraphemes {
    fn check(&self, value: &T) -> Result<(), Problem> {
        let graphemes = unicode_segmentation::UnicodeSegmentation::graphemes(value.as_ref(), true);
        self.bounds.check(graphemes.count(), LengthUnit::Graphemes)
    }
}

/// A value with a length that [`length`] can check.
///
/// Implement it for a type of your own to say how long a value of it is, and in what unit.
#[diagnostic::on_unimplemented(
    message = "`length` can't check a value of type `{Self}`",
    label = "`{Self}` doesn't implement `annotated::rules::HasLength`",
    note = "implement `annotated::rules::HasLength` for `{Self}` to say how long a value of it is"
)]
pub trait HasLength {
    /// The unit the length is counted in.
    const UNIT: LengthUnit;

    /// How long the value is, in [`HasLength::UNIT`]s.
    fn length(&self) -> usize;
}

impl HasLength for str {
    const UNIT: LengthUnit = LengthUnit::Chars;

    fn length(&self) -> usize {
        self.chars().count()
    }
}

impl HasLength for String {
    const UNIT: LengthUnit = LengthUnit::Chars;

    fn length(&self) -> usize {
        self.as_str().length()
    }
}

macro_rules! collection_lengths {
    ($(impl<$($generic:ident),*> for $collection:ty;)*) => {
        $(
            impl<$($generic),*> HasLength for $collection {
                const UNIT: LengthUnit = LengthUnit::Elements;

                fn length(&self) -> usize {
                    self.len()
                }
            }
        )*
    };
}

collection_lengths! {
    impl<T> for [T];
    impl<T> for Vec<T>;
    impl<T> for VecDeque<T>;
    impl<T, S> for HashSet<T, S>;
    impl<T> for BTreeSet<T>;
    impl<K, V, S> for HashMap<K, V, S>;
    impl<K, V> for BTreeMap<K, V>;
}

impl<T, const N: usize> HasLength for [T; N] {
    const UNIT: LengthUnit = LengthUnit::Elements;

    fn length(&self) -> usize {
        N
    }
}

macro_rules! forwarded_lengths {
    ($($pointer:ty),*) => {
        $(
            impl<T: HasLength + ?Sized> HasLength for $pointer {
                const UNIT: LengthUnit = T::UNIT;

                fn length(&self) -> usize {
                    T::length(self)
                }
            }
        )*
    };
}

forwarded_lengths!(&T, Box<T>, Rc<T>, Arc<T>);

impl<T: HasLength + ToOwned + ?Sized> HasLength for Cow<'_, T> {
    const UNIT: LengthUnit = T::UNIT;

    fn length(&self) -> usize {
        T::length(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_string_is_counted_in_chars() {
        assert_eq!(length().max(5).check("Renée"), Ok(()));
    }

    #[test]
    fn a_string_longer_than_the_max_is_too_long() {
        assert_eq!(
            length().max(5).check("Renée M"),
            Err(Problem::Length {
                bounds: Bounds::AtMost(5),
                unit: LengthUnit::Chars,
            })
        );
    }

    #[test]
    fn a_string_shorter_than_the_min_is_too_short() {
        assert_eq!(
            length().min(2).check("L"),
            Err(Problem::Length {
                bounds: Bounds::AtLeast(2),
                unit: LengthUnit::Chars,
            })
        );
    }

    #[test]
    fn a_length_breaking_a_min_and_max_reports_both() {
        assert_eq!(
            length().min(2).max(5).check("L"),
            Err(Problem::Length {
                bounds: Bounds::Between { min: 2, max: 5 },
                unit: LengthUnit::Chars,
            })
        );
    }

    #[test]
    fn a_max_set_before_a_min_bounds_the_length_the_same() {
        assert_eq!(
            length().max(5).min(2).check("L"),
            Err(Problem::Length {
                bounds: Bounds::Between { min: 2, max: 5 },
                unit: LengthUnit::Chars,
            })
        );
    }

    #[test]
    fn the_bounds_are_inclusive() {
        assert_eq!(
            (
                length().min(2).max(5).check("Lu"),
                length().min(2).max(5).check("Renée")
            ),
            (Ok(()), Ok(()))
        );
    }

    #[test]
    fn a_length_other_than_the_equal_one_is_wrong() {
        assert_eq!(
            length().equal(5).check("Luci"),
            Err(Problem::Length {
                bounds: Bounds::Between { min: 5, max: 5 },
                unit: LengthUnit::Chars,
            })
        );
    }

    #[test]
    fn a_length_without_bounds_accepts_any_value() {
        assert_eq!(length().check(""), Ok(()));
    }

    #[test]
    fn a_collection_is_counted_in_elements() {
        assert_eq!(
            length().max(2).check(&vec!["Renée", "Sam", "Kai"]),
            Err(Problem::Length {
                bounds: Bounds::AtMost(2),
                unit: LengthUnit::Elements,
            })
        );
    }

    #[test]
    fn a_map_is_counted_in_entries() {
        let labels = HashMap::from([("en", "Settings"), ("fr", "Paramètres")]);

        assert_eq!(
            length().max(1).check(&labels),
            Err(Problem::Length {
                bounds: Bounds::AtMost(1),
                unit: LengthUnit::Elements,
            })
        );
    }

    #[test]
    fn a_shared_string_is_counted_like_a_string() {
        let name: Arc<str> = Arc::from("Renée M");

        assert_eq!(
            length().max(5).check(&name),
            Err(Problem::Length {
                bounds: Bounds::AtMost(5),
                unit: LengthUnit::Chars,
            })
        );
    }

    #[test]
    fn a_string_can_be_counted_in_bytes() {
        assert_eq!(
            length().max(5).in_bytes().check("Renée"),
            Err(Problem::Length {
                bounds: Bounds::AtMost(5),
                unit: LengthUnit::Bytes,
            })
        );
    }

    #[test]
    fn a_string_can_be_counted_in_chars_explicitly() {
        assert_eq!(
            length().max(4).in_chars().check("Renée"),
            Err(Problem::Length {
                bounds: Bounds::AtMost(4),
                unit: LengthUnit::Chars,
            })
        );
    }

    #[test]
    fn bounds_set_after_the_unit_bound_the_length_the_same() {
        assert_eq!(
            length().in_bytes().max(5).check("Renée"),
            Err(Problem::Length {
                bounds: Bounds::AtMost(5),
                unit: LengthUnit::Bytes,
            })
        );
    }

    #[cfg(feature = "unicode")]
    #[test]
    fn a_string_can_be_counted_in_graphemes() {
        let flag_of_spain_of_two_chars = "🇪🇸";

        assert_eq!(length().max(1).in_graphemes().check(flag_of_spain_of_two_chars), Ok(()));
    }

    #[cfg(feature = "unicode")]
    #[test]
    fn a_string_longer_in_graphemes_than_the_max_is_too_long() {
        assert_eq!(
            length().max(1).in_graphemes().check("🇪🇸🇵🇹"),
            Err(Problem::Length {
                bounds: Bounds::AtMost(1),
                unit: LengthUnit::Graphemes,
            })
        );
    }

    fn at_most(max: usize, unit: LengthUnit) -> Result<(), Problem> {
        Err(Problem::Length {
            bounds: Bounds::AtMost(max),
            unit,
        })
    }

    #[test]
    fn an_owned_string_is_counted_in_chars() {
        assert_eq!(
            length().max(4).check(&"Renée".to_owned()),
            at_most(4, LengthUnit::Chars)
        );
    }

    #[test]
    fn a_slice_is_counted_in_elements() {
        let names: &[&str] = &["Renée", "Sam"];

        assert_eq!(length().max(1).check(names), at_most(1, LengthUnit::Elements));
    }

    #[test]
    fn an_array_is_counted_in_elements() {
        assert_eq!(
            length().max(1).check(&["Renée", "Sam"]),
            at_most(1, LengthUnit::Elements)
        );
    }

    #[test]
    fn a_queue_is_counted_in_elements() {
        let names = VecDeque::from(["Renée", "Sam"]);

        assert_eq!(length().max(1).check(&names), at_most(1, LengthUnit::Elements));
    }

    #[test]
    fn a_hash_set_is_counted_in_elements() {
        let names = HashSet::from(["Renée", "Sam"]);

        assert_eq!(length().max(1).check(&names), at_most(1, LengthUnit::Elements));
    }

    #[test]
    fn an_ordered_set_is_counted_in_elements() {
        let names = BTreeSet::from(["Renée", "Sam"]);

        assert_eq!(length().max(1).check(&names), at_most(1, LengthUnit::Elements));
    }

    #[test]
    fn an_ordered_map_is_counted_in_entries() {
        let labels = BTreeMap::from([("en", "Settings"), ("fr", "Paramètres")]);

        assert_eq!(length().max(1).check(&labels), at_most(1, LengthUnit::Elements));
    }

    #[test]
    fn a_borrowed_string_is_counted_like_a_string() {
        assert_eq!(length().max(4).check(&"Renée"), at_most(4, LengthUnit::Chars));
    }

    #[test]
    fn a_boxed_string_is_counted_like_a_string() {
        let name: Box<str> = Box::from("Renée");

        assert_eq!(length().max(4).check(&name), at_most(4, LengthUnit::Chars));
    }

    #[test]
    fn a_reference_counted_string_is_counted_like_a_string() {
        let name: Rc<str> = Rc::from("Renée");

        assert_eq!(length().max(4).check(&name), at_most(4, LengthUnit::Chars));
    }

    #[test]
    fn a_copy_on_write_string_is_counted_like_a_string() {
        let name: Cow<'_, str> = Cow::Borrowed("Renée");

        assert_eq!(length().max(4).check(&name), at_most(4, LengthUnit::Chars));
    }
}
