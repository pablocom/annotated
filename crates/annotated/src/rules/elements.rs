use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fmt::Display;

use crate::Location;

/// A collection whose elements `#[validate(each(...))]` checks: a sequence's elements at their index, or a
/// map's values at their key.
///
/// Implement it for a collection of your own to let `each` go through it.
#[diagnostic::on_unimplemented(
    message = "`each` can't go through the elements of `{Self}`",
    label = "`{Self}` doesn't implement `annotated::rules::HasElements`",
    note = "implement `annotated::rules::HasElements` for `{Self}` to say what its elements are"
)]
pub trait HasElements {
    /// An element of the collection, or a value of the map.
    type Element;

    /// Calls `check` with every element and its location under `at`, in the collection's order.
    fn for_each_element(&self, at: &Location<'_>, check: impl FnMut(&Self::Element, &Location<'_>));
}

impl<T> HasElements for [T] {
    type Element = T;

    fn for_each_element(&self, at: &Location<'_>, mut check: impl FnMut(&T, &Location<'_>)) {
        for (index, element) in self.iter().enumerate() {
            check(element, &at.index(index));
        }
    }
}

impl<T, const N: usize> HasElements for [T; N] {
    type Element = T;

    fn for_each_element(&self, at: &Location<'_>, check: impl FnMut(&T, &Location<'_>)) {
        self.as_slice().for_each_element(at, check);
    }
}

impl<T> HasElements for Vec<T> {
    type Element = T;

    fn for_each_element(&self, at: &Location<'_>, check: impl FnMut(&T, &Location<'_>)) {
        self.as_slice().for_each_element(at, check);
    }
}

impl<T> HasElements for VecDeque<T> {
    type Element = T;

    fn for_each_element(&self, at: &Location<'_>, mut check: impl FnMut(&T, &Location<'_>)) {
        for (index, element) in self.iter().enumerate() {
            check(element, &at.index(index));
        }
    }
}

impl<C: HasElements + ?Sized> HasElements for &C {
    type Element = C::Element;

    fn for_each_element(&self, at: &Location<'_>, check: impl FnMut(&C::Element, &Location<'_>)) {
        C::for_each_element(self, at, check);
    }
}

impl<C: HasElements + ?Sized> HasElements for Box<C> {
    type Element = C::Element;

    fn for_each_element(&self, at: &Location<'_>, check: impl FnMut(&C::Element, &Location<'_>)) {
        C::for_each_element(self, at, check);
    }
}

impl<K: Display, V, S> HasElements for HashMap<K, V, S> {
    type Element = V;

    fn for_each_element(&self, at: &Location<'_>, mut check: impl FnMut(&V, &Location<'_>)) {
        for (key, value) in self {
            check(value, &at.key(key));
        }
    }
}

impl<K: Display, V> HasElements for BTreeMap<K, V> {
    type Element = V;

    fn for_each_element(&self, at: &Location<'_>, mut check: impl FnMut(&V, &Location<'_>)) {
        for (key, value) in self {
            check(value, &at.key(key));
        }
    }
}
