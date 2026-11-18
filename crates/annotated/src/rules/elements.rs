use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::fmt::Display;

use crate::Location;

/// A collection whose elements `#[validate(each(...))]` can check: the elements of a sequence or a set, each at its
/// index, or the values of a map, each at its key.
///
/// It is implemented for `Vec`, `VecDeque`, arrays, slices, `HashSet`, `BTreeSet`, `HashMap` and `BTreeMap`, and for
/// any of these behind a reference or a `Box`.
///
/// A set's elements are at their index in its iteration order. A `HashSet`'s order is arbitrary, so there the index
/// names the element only in that same set. `#[derive(Deserialize)]` checks the elements of a set it reads from the
/// input as it reads them, each at its index in the input, when the field's type names the `HashSet` or the
/// `BTreeSet`, alone or in an `Option`.
///
/// Implement it for a collection of your own to let `each` go through it:
///
/// ```
/// use annotated::{Location, rules::HasElements};
///
/// struct Pair<T> {
///     first: T,
///     second: T,
/// }
///
/// impl<T> HasElements for Pair<T> {
///     type Element = T;
///
///     fn for_each_element(&self, at: &Location<'_>, mut check: impl FnMut(&T, &Location<'_>) -> bool) {
///         if check(&self.first, &at.index(0)) {
///             check(&self.second, &at.index(1));
///         }
///     }
/// }
/// ```
#[diagnostic::on_unimplemented(
    message = "`each` can't go through the elements of `{Self}`",
    label = "`{Self}` doesn't implement `annotated::rules::HasElements`",
    note = "`each` goes through the elements of a list, an array, a slice or a set, and the values of a map; for a \
            collection of your own, implement `annotated::rules::HasElements` to say what its elements are"
)]
pub trait HasElements {
    /// An element of the collection, or a value of the map.
    type Element;

    /// Calls `check` with each element and its location under `at`, in the collection's order, for as long as `check`
    /// returns `true`. Once it returns `false`, the remaining elements are skipped.
    fn for_each_element(&self, at: &Location<'_>, check: impl FnMut(&Self::Element, &Location<'_>) -> bool);
}

impl<T> HasElements for [T] {
    type Element = T;

    fn for_each_element(&self, at: &Location<'_>, mut check: impl FnMut(&T, &Location<'_>) -> bool) {
        for (index, element) in self.iter().enumerate() {
            if !check(element, &at.index(index)) {
                return;
            }
        }
    }
}

impl<T, const N: usize> HasElements for [T; N] {
    type Element = T;

    fn for_each_element(&self, at: &Location<'_>, check: impl FnMut(&T, &Location<'_>) -> bool) {
        self.as_slice().for_each_element(at, check);
    }
}

impl<T> HasElements for Vec<T> {
    type Element = T;

    fn for_each_element(&self, at: &Location<'_>, check: impl FnMut(&T, &Location<'_>) -> bool) {
        self.as_slice().for_each_element(at, check);
    }
}

impl<T> HasElements for VecDeque<T> {
    type Element = T;

    fn for_each_element(&self, at: &Location<'_>, mut check: impl FnMut(&T, &Location<'_>) -> bool) {
        for (index, element) in self.iter().enumerate() {
            if !check(element, &at.index(index)) {
                return;
            }
        }
    }
}

impl<C: HasElements + ?Sized> HasElements for &C {
    type Element = C::Element;

    fn for_each_element(&self, at: &Location<'_>, check: impl FnMut(&C::Element, &Location<'_>) -> bool) {
        C::for_each_element(self, at, check);
    }
}

impl<C: HasElements + ?Sized> HasElements for Box<C> {
    type Element = C::Element;

    fn for_each_element(&self, at: &Location<'_>, check: impl FnMut(&C::Element, &Location<'_>) -> bool) {
        C::for_each_element(self, at, check);
    }
}

/// An element is at its index in the set's iteration order, which is arbitrary, so the index names the element only in
/// this same set.
impl<T, S> HasElements for HashSet<T, S> {
    type Element = T;

    fn for_each_element(&self, at: &Location<'_>, check: impl FnMut(&T, &Location<'_>) -> bool) {
        for_each_element_at_its_index(self, at, check);
    }
}

/// An element is at its index in the set's order, which is the elements' own.
impl<T> HasElements for BTreeSet<T> {
    type Element = T;

    fn for_each_element(&self, at: &Location<'_>, check: impl FnMut(&T, &Location<'_>) -> bool) {
        for_each_element_at_its_index(self, at, check);
    }
}

fn for_each_element_at_its_index<'a, T: 'a>(
    elements: impl IntoIterator<Item = &'a T>,
    at: &Location<'_>,
    mut check: impl FnMut(&T, &Location<'_>) -> bool,
) {
    for (index, element) in elements.into_iter().enumerate() {
        if !check(element, &at.index(index)) {
            return;
        }
    }
}

impl<K: Display, V, S> HasElements for HashMap<K, V, S> {
    type Element = V;

    fn for_each_element(&self, at: &Location<'_>, mut check: impl FnMut(&V, &Location<'_>) -> bool) {
        for (key, value) in self {
            if !check(value, &at.key(key)) {
                return;
            }
        }
    }
}

impl<K: Display, V> HasElements for BTreeMap<K, V> {
    type Element = V;

    fn for_each_element(&self, at: &Location<'_>, mut check: impl FnMut(&V, &Location<'_>) -> bool) {
        for (key, value) in self {
            if !check(value, &at.key(key)) {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn elements_checked_until_a_check_answers_false(collection: &impl HasElements) -> usize {
        let mut checked = 0;
        collection.for_each_element(&Location::root(), |_, _| {
            checked += 1;
            false
        });
        checked
    }

    #[test]
    fn going_through_a_slice_stops_once_a_check_answers_false() {
        let slice: &[&str] = &["tin", "jars"];

        assert_eq!(elements_checked_until_a_check_answers_false(&slice), 1);
    }

    #[test]
    fn going_through_a_queue_stops_once_a_check_answers_false() {
        let queue = VecDeque::from(["tin", "jars"]);

        assert_eq!(elements_checked_until_a_check_answers_false(&queue), 1);
    }

    #[test]
    fn going_through_a_hash_set_stops_once_a_check_answers_false() {
        let set = HashSet::from(["tin", "jars"]);

        assert_eq!(elements_checked_until_a_check_answers_false(&set), 1);
    }

    #[test]
    fn going_through_a_b_tree_set_stops_once_a_check_answers_false() {
        let set = BTreeSet::from(["tin", "jars"]);

        assert_eq!(elements_checked_until_a_check_answers_false(&set), 1);
    }

    #[test]
    fn going_through_a_hash_map_stops_once_a_check_answers_false() {
        let map = HashMap::from([("top", "tin"), ("bottom", "jars")]);

        assert_eq!(elements_checked_until_a_check_answers_false(&map), 1);
    }

    #[test]
    fn going_through_a_b_tree_map_stops_once_a_check_answers_false() {
        let map = BTreeMap::from([("top", "tin"), ("bottom", "jars")]);

        assert_eq!(elements_checked_until_a_check_answers_false(&map), 1);
    }
}
