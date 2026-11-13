//! What `rules()`, as the derives generate it, asks each rule with.
//!
//! A rule that can't check a field's type is already a compile error where the derive checks the field, and so is
//! `each(...)` on a value without elements. So that `rules()` doesn't report the same mistake a second time, it asks
//! through traits picked by autoref: the first of each pair applies when the types fit, and the second answers
//! nothing when they don't.

use std::marker::PhantomData;

use crate::rules::HasElements;
use crate::{Rule, ValueRules};

/// The type of the value a rule checks.
pub struct Checked<T: ?Sized>(PhantomData<fn(&T)>);

/// Stands for the element of a value that has none. No rule checks it, so no rule says anything of it.
pub struct Unnamed;

impl<T: ?Sized> Checked<T> {
    /// The type `T`, as the type of a value that rules check.
    #[must_use]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T: ?Sized> Clone for Checked<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for Checked<T> {}

impl<T: ?Sized> Default for Checked<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Checked<Option<T>> {
    /// The type of the value the `Option` holds, which every rule but `required` checks.
    #[must_use]
    pub const fn inside(self) -> Checked<T> {
        Checked::new()
    }
}

/// The type of a collection's elements, which the rules in `each(...)` check.
pub trait ElementOfACollection {
    /// An element of the collection.
    type Element;

    /// The type of the collection's elements.
    fn element(&self) -> Checked<Self::Element> {
        Checked::new()
    }
}

impl<C: HasElements + ?Sized> ElementOfACollection for Checked<C> {
    type Element = C::Element;
}

/// Answers for a type that has no elements, though `each(...)` is written on it.
pub trait ElementOfAnotherValue {
    /// No type that a rule checks.
    fn element(&self) -> Checked<Unnamed> {
        Checked::new()
    }
}

impl<T: ?Sized> ElementOfAnotherValue for &Checked<T> {}

/// A rule, and the type of the value it checks.
pub struct Said<'a, R, T: ?Sized>(pub &'a R, pub Checked<T>);

/// What a rule says it requires, of a type it can check.
pub trait SaidByARule {
    /// `rules`, and the problem the rule says a value breaking it has.
    #[must_use]
    fn added_to(&self, rules: ValueRules) -> ValueRules;
}

impl<R: Rule<T>, T: ?Sized> SaidByARule for Said<'_, R, T> {
    fn added_to(&self, rules: ValueRules) -> ValueRules {
        match self.0.problem() {
            Some(problem) => rules.with_problem(problem),
            None => rules,
        }
    }
}

/// Answers for a rule that can't check the type.
pub trait SaidByNoRule {
    /// `rules`, with nothing added.
    #[must_use]
    fn added_to(&self, rules: ValueRules) -> ValueRules {
        rules
    }
}

impl<R, T: ?Sized> SaidByNoRule for &Said<'_, R, T> {}
