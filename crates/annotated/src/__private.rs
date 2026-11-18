#[cfg(feature = "regex")]
pub use regex::Regex;
#[cfg(feature = "serde")]
pub use serde;

#[cfg(feature = "serde")]
use std::collections::{BTreeSet, HashSet};
#[cfg(feature = "serde")]
use std::hash::{BuildHasher, Hash};
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
    Buffered, InternallyTaggedUnitVisitor, TagName, TagOrContent, TagOrContentKey, TagValue, Tagged,
    UnitVariantVisitor, read_internally_tagged,
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

/// A set that `#[derive(Deserialize)]` reads as the list of its elements, in the order of the input, so that `each`
/// checks every element at its index in the input. The list becomes the set once it is checked.
#[cfg(feature = "serde")]
pub trait ReadAsAList: Sized {
    /// The set's elements, in the order they were read.
    type List;

    /// The set of the `list`'s elements, inserted in their order, as serde inserts them, so that of two equal elements
    /// the first is kept.
    fn from_list(list: Self::List) -> Self;
}

#[cfg(feature = "serde")]
impl<T: Eq + Hash, S: BuildHasher + Default> ReadAsAList for HashSet<T, S> {
    type List = Vec<T>;

    fn from_list(list: Vec<T>) -> Self {
        let mut set = Self::with_capacity_and_hasher(list.len(), S::default());
        for element in list {
            set.insert(element);
        }
        set
    }
}

#[cfg(feature = "serde")]
impl<T: Ord> ReadAsAList for BTreeSet<T> {
    type List = Vec<T>;

    fn from_list(list: Vec<T>) -> Self {
        let mut set = Self::new();
        for element in list {
            set.insert(element);
        }
        set
    }
}

/// Whether `T` is a `HashSet` or a `BTreeSet`. `#[derive(Deserialize)]` reads a set as the list of its elements only
/// when the field's type names it, so it asks this of the type of a field checked with `each(...)` whose name it doesn't
/// know, in a constant that fails to compile when the type is a set.
///
/// The constant is inherent for the two sets, and takes the place of the one [`NotASet`] gives every other type, as
/// with [`OwnRules`].
#[cfg(feature = "serde")]
pub struct Collection<T: ?Sized>(PhantomData<T>);

/// What [`Collection`] answers for a type that is neither a `HashSet` nor a `BTreeSet`. Its constant is named through
/// `Collection<T>`, with the trait in scope.
#[cfg(feature = "serde")]
pub trait NotASet {
    /// Whether the type is a `HashSet` or a `BTreeSet`.
    const IS_A_SET: bool = false;
}

#[cfg(feature = "serde")]
impl<T: ?Sized> NotASet for Collection<T> {}

#[cfg(feature = "serde")]
impl<T, S> Collection<HashSet<T, S>> {
    /// A `HashSet` is a set.
    pub const IS_A_SET: bool = true;
}

#[cfg(feature = "serde")]
impl<T> Collection<BTreeSet<T>> {
    /// A `BTreeSet` is a set.
    pub const IS_A_SET: bool = true;
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

    /// Like [`ElementIssues::new`], but leaves `found_in_elements` empty, with its limit, where `new` takes it. A set's
    /// elements are checked where the set is built, and the field's checks still name the report afterwards, to append
    /// it when the set couldn't be built.
    #[must_use]
    pub fn taken_from(found_in_elements: &mut Report) -> Self {
        let left_behind = found_in_elements.new_alike();
        Self::new(std::mem::replace(found_in_elements, left_behind))
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
