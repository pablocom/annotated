#[cfg(feature = "regex")]
pub use regex::Regex;
#[cfg(feature = "serde")]
pub use serde;

use std::marker::PhantomData;

use crate::{Issue, Location, Problem, Report, Validate};

#[cfg(feature = "serde")]
pub use crate::borrowed::{borrow_cow_bytes, borrow_cow_str};
#[cfg(feature = "json-schema")]
pub use crate::checked_type::{
    Checked, ElementOfACollection, ElementOfAnotherValue, Said, SaidByARule, SaidByNoRule, Unnamed,
};
#[cfg(feature = "serde")]
pub use crate::collecting_type_errors::{
    CollectingTypeErrorsAt, CollectingTypeErrorsOfAFunctionAt, NestedCollectingTypeErrorsAt,
    deserialize_adjacently_tagged_collecting_type_errors, deserialize_enum_collecting_type_errors,
};
#[cfg(feature = "serde")]
pub use crate::deserialize::DeserializeAt;
#[cfg(feature = "serde")]
pub use crate::flattened::{FlatMapDeserializer, Key, Leftovers};
#[cfg(feature = "serde")]
pub use crate::tagged::{
    Buffered, InternallyTaggedUnitVisitor, TagOrContent, TagOrContentKey, TagValue, Tagged, UnitVariantVisitor,
    read_internally_tagged,
};

// Out of line and cold, so a rule that passes costs one branch: the benchmarks' order of 100 lines validates in
// 357 ns instead of 429 ns with LTO, and in 581 ns instead of 1.03 µs without.
#[cold]
#[inline(never)]
pub fn record_issue(found: &mut Report, at: &Location<'_>, problem: Problem, message: Option<&'static str>) {
    match message {
        Some(message) => found.push_with_message(at, problem, message),
        None => found.push(at, problem),
    }
}

/// Whether `T` has rules of its own, which a field holding it joins to its own with `nested`. The derives ask it of
/// the types written in each field that has neither `nested` nor `not_nested`, in a constant that fails to compile
/// when one has rules.
///
/// A derive can't ask whether a type implements a trait, but a constant can: an inherent associated constant exists
/// only for the types its bound holds for, and then takes the place of the one [`NoOwnRules`] gives every type.
pub struct OwnRules<T: ?Sized>(PhantomData<T>);

/// What [`OwnRules`] answers for a type without rules of its own. Its constants are named through `OwnRules<T>`, with
/// the trait in scope.
pub trait NoOwnRules {
    /// Whether the type implements [`Validate`].
    const EXIST_FOR_VALIDATE: bool = false;

    /// Whether the type implements annotated's [`Deserialize`](crate::Deserialize).
    #[cfg(feature = "serde")]
    const EXIST_FOR_DESERIALIZE: bool = false;
}

impl<T: ?Sized> NoOwnRules for OwnRules<T> {}

impl<T: Validate + ?Sized> OwnRules<T> {
    /// `T` implements [`Validate`].
    pub const EXIST_FOR_VALIDATE: bool = true;
}

#[cfg(feature = "serde")]
impl<T: crate::Deserialize<'static>> OwnRules<T> {
    /// `T` implements annotated's [`Deserialize`](crate::Deserialize).
    pub const EXIST_FOR_DESERIALIZE: bool = true;
}

/// The problem of a tag that names none of the `variants` of its enum.
#[cold]
#[must_use]
pub const fn unknown_variant(variants: &'static [&'static str]) -> Problem {
    Problem::UnknownVariant { expected: variants }
}

#[cfg(feature = "serde")]
pub fn deserialize_for_serde<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: crate::Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    use serde::de::Error as _;

    crate::from_deserializer(deserializer).map_err(|error| match error {
        crate::DeserializeError::Issues(issues) => D::Error::custom(issues),
        crate::DeserializeError::Format(format_error) => format_error,
    })
}

/// The issues that a collection's elements reported while they were read. They wait here for each element's
/// `nested` rule, in the order `HasElements` goes through the elements.
///
/// When the elements' report was finished, issues after the last one waiting may have been left out. Those of an
/// element whose `nested` rule doesn't run are dropped, so the report the others join may not reach its limit. Once
/// none is waiting, it keeps no more, so that no issue found later is listed before one left out.
pub struct ElementIssues {
    waiting: std::vec::IntoIter<Issue>,
    may_some_be_left_out: bool,
}

impl ElementIssues {
    #[must_use]
    pub fn new(found_in_elements: Report) -> Self {
        Self {
            may_some_be_left_out: found_in_elements.is_finished(),
            waiting: found_in_elements.into_issues().into_iter(),
        }
    }

    /// Moves the next waiting issues that are at `at` or under it into `found`, and returns whether there were any.
    pub fn join_those_at(&mut self, at: &Location<'_>, found: &mut Report) -> bool {
        let mut has_joined_any = false;
        while let Some(issue) = self.next_at(at) {
            found.push_issue(issue);
            has_joined_any = true;
        }
        if self.may_some_be_left_out && self.waiting.as_slice().is_empty() {
            found.keep_no_more();
        }
        has_joined_any
    }

    /// Drops the next waiting issues that are at `at` or under it. They belong to an element whose `nested` rule
    /// didn't run.
    pub fn skip_those_at(&mut self, at: &Location<'_>) {
        while self.next_at(at).is_some() {}
    }

    fn next_at(&mut self, at: &Location<'_>) -> Option<Issue> {
        let first_waiting = self.waiting.as_slice().first()?;
        if !at.leads_to(first_waiting.path()) {
            return None;
        }
        self.waiting.next()
    }
}
