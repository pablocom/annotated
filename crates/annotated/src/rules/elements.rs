use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fmt::Display;

use crate::Location;

/// A collection whose elements `#[validate(each(...))]` can check: the elements of a sequence, each at its index, or
/// the values of a map, each at its key.
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
    note = "implement `annotated::rules::HasElements` for `{Self}` to say what its elements are"
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
