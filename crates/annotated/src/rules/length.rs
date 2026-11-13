use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::ops::RangeInclusive;
use std::rc::Rc;
use std::sync::Arc;

use crate::{Bounds, LengthUnit, Problem, Rule};

/// The rule that a value's length is within bounds: `length().min(1).max(50)`.
///
/// It checks anything that implements [`HasLength`]. A string is counted in `char`s (Unicode scalar values), and a
/// collection in elements. To count a string in another unit, use [`Length::in_bytes`], [`Length::in_chars`] or, with
/// the `unicode` feature, `Length::in_graphemes`. A collection can't be counted in another unit. The bounds are
/// included, and a length with no bounds accepts any value.
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

/// Checks the length of a string, in `char`s, or of a collection, in elements. Made by [`length`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Length {
    bounds: LengthBounds,
}

/// Checks the length of a string in a chosen unit: `char`s, UTF-8 bytes or grapheme clusters. Made by
/// [`Length::in_chars`], [`Length::in_bytes`] or `Length::in_graphemes`. It checks only strings, so using it on a
/// collection doesn't compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StringLength {
    bounds: LengthBounds,
    unit: StringUnit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StringUnit {
    Chars,
    Bytes,
    #[cfg(feature = "unicode")]
    Graphemes,
}

impl StringUnit {
    fn possible_counts(self, text: &str) -> RangeInclusive<usize> {
        let bytes = text.len();
        match self {
            Self::Chars => bytes.div_ceil(char::MAX_LEN_UTF8)..=bytes,
            Self::Bytes => bytes..=bytes,
            #[cfg(feature = "unicode")]
            Self::Graphemes => usize::from(!text.is_empty())..=bytes,
        }
    }

    fn count(self, text: &str) -> usize {
        match self {
            Self::Chars => text.chars().count(),
            Self::Bytes => text.len(),
            #[cfg(feature = "unicode")]
            Self::Graphemes => unicode_segmentation::UnicodeSegmentation::graphemes(text, true).count(),
        }
    }

    fn length_unit(self) -> LengthUnit {
        match self {
            Self::Chars => LengthUnit::Chars,
            Self::Bytes => LengthUnit::Bytes,
            #[cfg(feature = "unicode")]
            Self::Graphemes => LengthUnit::Graphemes,
        }
    }
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

    fn check(
        self,
        possible_lengths: RangeInclusive<usize>,
        length: impl FnOnce() -> usize,
        unit: LengthUnit,
    ) -> Result<(), Problem> {
        let is_within = self
            .shared_verdict(possible_lengths)
            .unwrap_or_else(|| self.accepts(length()));
        if is_within {
            return Ok(());
        }
        let bounds = Bounds::of(self.min, self.max).expect("a length out of bounds has bounds");
        Err(Problem::Length { bounds, unit })
    }

    fn shared_verdict(self, possible_lengths: RangeInclusive<usize>) -> Option<bool> {
        let (fewest, most) = possible_lengths.into_inner();
        if self.accepts(fewest) && self.accepts(most) {
            return Some(true);
        }
        if self.is_too_short(most) || self.is_too_long(fewest) {
            return Some(false);
        }
        None
    }

    fn accepts(self, length: usize) -> bool {
        !self.is_too_short(length) && !self.is_too_long(length)
    }

    fn is_too_short(self, length: usize) -> bool {
        self.min.is_some_and(|min| length < min)
    }

    fn is_too_long(self, length: usize) -> bool {
        self.max.is_some_and(|max| length > max)
    }

    fn problem(self, unit: LengthUnit) -> Option<Problem> {
        let bounds = Bounds::of(self.min, self.max)?;
        Some(Problem::Length { bounds, unit })
    }
}

impl Length {
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
        self.min(equal).max(equal)
    }

    /// The rule counting a string's `char`s. It then checks only strings, not collections.
    #[must_use]
    pub const fn in_chars(self) -> StringLength {
        self.counting(StringUnit::Chars)
    }

    /// The rule counting a string's UTF-8 bytes. It then checks only strings, not collections.
    #[must_use]
    pub const fn in_bytes(self) -> StringLength {
        self.counting(StringUnit::Bytes)
    }

    /// The rule counting a string's grapheme clusters, which are what a person sees as one character. It then checks
    /// only strings, not collections.
    #[cfg(feature = "unicode")]
    #[must_use]
    pub const fn in_graphemes(self) -> StringLength {
        self.counting(StringUnit::Graphemes)
    }

    const fn counting(self, unit: StringUnit) -> StringLength {
        StringLength {
            bounds: self.bounds,
            unit,
        }
    }
}

impl StringLength {
    /// The rule with `min` as the shortest length it accepts.
    #[must_use]
    pub const fn min(self, min: usize) -> Self {
        Self {
            bounds: self.bounds.with_min(min),
            ..self
        }
    }

    /// The rule with `max` as the longest length it accepts.
    #[must_use]
    pub const fn max(self, max: usize) -> Self {
        Self {
            bounds: self.bounds.with_max(max),
            ..self
        }
    }

    /// The rule accepting only the length `equal`.
    #[must_use]
    pub const fn equal(self, equal: usize) -> Self {
        self.min(equal).max(equal)
    }
}

impl<T: HasLength + ?Sized> Rule<T> for Length {
    fn check(&self, value: &T) -> Result<(), Problem> {
        self.bounds.check(value.possible_lengths(), || value.length(), T::UNIT)
    }

    fn problem(&self) -> Option<Problem> {
        self.bounds.problem(T::UNIT)
    }
}

#[diagnostic::do_not_recommend]
impl<T: AsRef<str> + ?Sized> Rule<T> for StringLength {
    // Inlined where the rule is written, so its unit is known there and picks the counting at compile time: a
    // length in bytes is checked in 2.2 ns instead of 4.3 ns.
    #[inline]
    fn check(&self, value: &T) -> Result<(), Problem> {
        let text = value.as_ref();
        self.bounds.check(
            self.unit.possible_counts(text),
            || self.unit.count(text),
            self.unit.length_unit(),
        )
    }

    fn problem(&self) -> Option<Problem> {
        self.bounds.problem(self.unit.length_unit())
    }
}

/// A value with a length that [`length`] can check.
///
/// Implement it for a type of your own to say how long one of its values is, and in what unit.
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

    /// The lengths the value could have, known without counting it: by default, its length.
    #[doc(hidden)]
    fn possible_lengths(&self) -> RangeInclusive<usize> {
        let length = self.length();
        length..=length
    }
}

impl HasLength for str {
    const UNIT: LengthUnit = LengthUnit::Chars;

    fn length(&self) -> usize {
        StringUnit::Chars.count(self)
    }

    fn possible_lengths(&self) -> RangeInclusive<usize> {
        StringUnit::Chars.possible_counts(self)
    }
}

impl HasLength for String {
    const UNIT: LengthUnit = LengthUnit::Chars;

    fn length(&self) -> usize {
        self.as_str().length()
    }

    fn possible_lengths(&self) -> RangeInclusive<usize> {
        self.as_str().possible_lengths()
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

                fn possible_lengths(&self) -> RangeInclusive<usize> {
                    T::possible_lengths(self)
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

    fn possible_lengths(&self) -> RangeInclusive<usize> {
        T::possible_lengths(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_says_its_bounds_and_that_a_string_is_counted_in_chars() {
        assert_eq!(
            Rule::<str>::problem(&length().min(1).max(50)),
            Some(Problem::Length {
                bounds: Bounds::Between { min: 1, max: 50 },
                unit: LengthUnit::Chars
            })
        );
    }

    #[test]
    fn length_says_that_a_collection_is_counted_in_elements() {
        assert_eq!(
            Rule::<Vec<u8>>::problem(&length().max(3)),
            Some(Problem::Length {
                bounds: Bounds::AtMost(3),
                unit: LengthUnit::Elements
            })
        );
    }

    #[test]
    fn length_in_a_chosen_unit_says_that_unit() {
        assert_eq!(
            Rule::<str>::problem(&length().min(2).in_bytes()),
            Some(Problem::Length {
                bounds: Bounds::AtLeast(2),
                unit: LengthUnit::Bytes
            })
        );
    }

    #[test]
    fn length_without_bounds_says_nothing() {
        assert_eq!(
            (
                Rule::<str>::problem(&length()),
                Rule::<str>::problem(&length().in_bytes())
            ),
            (None, None)
        );
    }

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
            length().in_bytes().min(7).max(9).check("Renée"),
            Err(Problem::Length {
                bounds: Bounds::Between { min: 7, max: 9 },
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

    fn at_least(min: usize, unit: LengthUnit) -> Result<(), Problem> {
        Err(Problem::Length {
            bounds: Bounds::AtLeast(min),
            unit,
        })
    }

    #[test]
    fn a_string_of_two_byte_chars_at_its_min_follows_the_rule() {
        assert_eq!(length().min(2).check("ñu"), Ok(()));
    }

    #[test]
    fn a_string_of_two_byte_chars_one_short_of_its_min_breaks_the_rule() {
        assert_eq!(length().min(2).check("ñ"), at_least(2, LengthUnit::Chars));
    }

    #[test]
    fn a_string_of_four_byte_chars_at_its_min_follows_the_rule() {
        let two_clefs = "𝄞𝄞";

        assert_eq!(length().min(2).check(two_clefs), Ok(()));
    }

    #[test]
    fn a_string_of_four_byte_chars_one_short_of_its_min_breaks_the_rule() {
        let two_clefs = "𝄞𝄞";

        assert_eq!(length().min(3).check(two_clefs), at_least(3, LengthUnit::Chars));
    }

    #[test]
    fn a_string_of_two_byte_chars_one_under_its_max_follows_the_rule() {
        assert_eq!(length().max(6).check("Renéé"), Ok(()));
    }

    #[test]
    fn a_string_of_two_byte_chars_one_over_its_max_breaks_the_rule() {
        assert_eq!(length().max(6).check("Renée M"), at_most(6, LengthUnit::Chars));
    }

    #[test]
    fn a_string_of_four_byte_chars_at_its_max_follows_the_rule() {
        let three_clefs = "𝄞𝄞𝄞";

        assert_eq!(length().max(3).check(three_clefs), Ok(()));
    }

    #[test]
    fn a_string_of_four_byte_chars_one_over_its_max_breaks_the_rule() {
        let three_clefs = "𝄞𝄞𝄞";

        assert_eq!(length().max(2).check(three_clefs), at_most(2, LengthUnit::Chars));
    }

    #[test]
    fn an_ascii_string_one_char_over_its_max_breaks_the_rule() {
        assert_eq!(length().max(2).check("Sam"), at_most(2, LengthUnit::Chars));
    }

    #[test]
    fn a_string_counted_in_chars_explicitly_at_its_max_follows_the_rule() {
        let three_clefs = "𝄞𝄞𝄞";

        assert_eq!(length().max(3).in_chars().check(three_clefs), Ok(()));
    }

    #[cfg(feature = "unicode")]
    #[test]
    fn a_string_of_one_grapheme_follows_a_min_of_one() {
        let flag_of_spain_of_two_chars = "🇪🇸";

        assert_eq!(length().min(1).in_graphemes().check(flag_of_spain_of_two_chars), Ok(()));
    }

    #[cfg(feature = "unicode")]
    #[test]
    fn an_empty_string_breaks_a_min_of_one_grapheme() {
        assert_eq!(
            length().min(1).in_graphemes().check(""),
            at_least(1, LengthUnit::Graphemes)
        );
    }

    #[cfg(feature = "unicode")]
    #[test]
    fn a_string_one_grapheme_short_of_its_min_breaks_the_rule() {
        let flag_of_spain_of_two_chars = "🇪🇸";

        assert_eq!(
            length().min(2).in_graphemes().check(flag_of_spain_of_two_chars),
            at_least(2, LengthUnit::Graphemes)
        );
    }

    #[cfg(feature = "unicode")]
    #[test]
    fn a_string_of_many_bytes_exactly_as_long_as_its_bounds_in_graphemes_follows_the_rule() {
        let flags_of_spain_and_portugal = "🇪🇸🇵🇹";

        assert_eq!(
            length().equal(2).in_graphemes().check(flags_of_spain_and_portugal),
            Ok(())
        );
    }

    #[cfg(feature = "unicode")]
    #[test]
    fn a_string_of_one_grapheme_breaks_a_max_of_none() {
        assert_eq!(
            length().max(0).in_graphemes().check("a"),
            at_most(0, LengthUnit::Graphemes)
        );
    }

    #[cfg(feature = "unicode")]
    #[test]
    fn an_empty_string_follows_a_max_of_none_in_graphemes() {
        assert_eq!(length().max(0).in_graphemes().check(""), Ok(()));
    }

    #[cfg(feature = "unicode")]
    #[test]
    fn a_string_with_as_many_bytes_as_its_max_in_graphemes_follows_the_rule() {
        assert_eq!(length().max(3).in_graphemes().check("Sam"), Ok(()));
    }
}
