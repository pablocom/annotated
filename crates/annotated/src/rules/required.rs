use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::rc::Rc;
use std::sync::Arc;

use crate::{Problem, Rule};

/// The rule that a value is present, like .NET's `[Required]`: an `Option` is `Some`, a string is neither
/// empty nor only whitespace, and a collection is not empty.
///
/// It checks anything that implements [`Required`]. On an `Option` field, `#[derive(Validate)]` checks the
/// `Option` itself with it, while every other rule checks the value inside.
#[must_use]
pub const fn required() -> Presence {
    Presence
}

/// Checks that a value is present; made by [`required`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Presence;

impl<T: Required + ?Sized> Rule<T> for Presence {
    fn check(&self, value: &T) -> Result<(), Problem> {
        if value.is_present() {
            return Ok(());
        }
        Err(Problem::Required)
    }
}

/// A value that can be present or not, so [`required`] can check it.
///
/// Implement it for a type of your own to say when a value of it counts as present.
#[diagnostic::on_unimplemented(
    message = "`required` can't check a value of type `{Self}`",
    label = "`{Self}` doesn't implement `annotated::rules::Required`",
    note = "implement `annotated::rules::Required` for `{Self}` to say when a value of it is present"
)]
pub trait Required {
    /// Whether the value is present.
    fn is_present(&self) -> bool;
}

impl Required for str {
    fn is_present(&self) -> bool {
        !self.trim().is_empty()
    }
}

impl Required for String {
    fn is_present(&self) -> bool {
        self.as_str().is_present()
    }
}

impl<T> Required for Option<T> {
    fn is_present(&self) -> bool {
        self.is_some()
    }
}

impl<T> Required for [T] {
    fn is_present(&self) -> bool {
        !self.is_empty()
    }
}

impl<T, const N: usize> Required for [T; N] {
    fn is_present(&self) -> bool {
        !self.is_empty()
    }
}

impl<T> Required for Vec<T> {
    fn is_present(&self) -> bool {
        !self.is_empty()
    }
}

impl<T> Required for VecDeque<T> {
    fn is_present(&self) -> bool {
        !self.is_empty()
    }
}

impl<T, S> Required for HashSet<T, S> {
    fn is_present(&self) -> bool {
        !self.is_empty()
    }
}

impl<T> Required for BTreeSet<T> {
    fn is_present(&self) -> bool {
        !self.is_empty()
    }
}

impl<K, V, S> Required for HashMap<K, V, S> {
    fn is_present(&self) -> bool {
        !self.is_empty()
    }
}

impl<K, V> Required for BTreeMap<K, V> {
    fn is_present(&self) -> bool {
        !self.is_empty()
    }
}

impl<T: Required + ?Sized> Required for &T {
    fn is_present(&self) -> bool {
        T::is_present(self)
    }
}

impl<T: Required + ?Sized> Required for Box<T> {
    fn is_present(&self) -> bool {
        T::is_present(self)
    }
}

impl<T: Required + ?Sized> Required for Rc<T> {
    fn is_present(&self) -> bool {
        T::is_present(self)
    }
}

impl<T: Required + ?Sized> Required for Arc<T> {
    fn is_present(&self) -> bool {
        T::is_present(self)
    }
}

impl<T: Required + ToOwned + ?Sized> Required for Cow<'_, T> {
    fn is_present(&self) -> bool {
        T::is_present(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_string_with_text_is_present() {
        assert_eq!(required().check("Alex"), Ok(()));
    }

    #[test]
    fn a_string_with_text_and_surrounding_whitespace_is_present() {
        assert_eq!(required().check("  Renée  "), Ok(()));
    }

    #[test]
    fn an_empty_string_is_missing_a_required_value() {
        assert_eq!(required().check(""), Err(Problem::Required));
    }

    #[test]
    fn a_string_of_only_whitespace_is_missing_a_required_value() {
        assert_eq!(required().check(&" \t\n".to_owned()), Err(Problem::Required));
    }

    #[test]
    fn a_shared_string_is_checked_like_a_string() {
        let name: Arc<str> = Arc::from("   ");

        assert_eq!(required().check(&name), Err(Problem::Required));
    }

    #[test]
    fn none_is_missing_a_required_value() {
        assert_eq!(required().check(&None::<String>), Err(Problem::Required));
    }

    #[test]
    fn some_is_present_whatever_it_holds() {
        assert_eq!(required().check(&Some(String::new())), Ok(()));
    }

    #[test]
    fn an_empty_collection_is_missing_a_required_value() {
        assert_eq!(required().check(&Vec::<String>::new()), Err(Problem::Required));
    }

    #[test]
    fn a_collection_with_an_element_is_present() {
        assert_eq!(required().check(&vec!["Alex"]), Ok(()));
    }

    #[test]
    fn an_empty_map_is_missing_a_required_value() {
        assert_eq!(required().check(&HashMap::<String, u8>::new()), Err(Problem::Required));
    }

    #[test]
    fn a_map_with_an_entry_is_present() {
        assert_eq!(required().check(&BTreeMap::from([("en", "Settings")])), Ok(()));
    }

    #[test]
    fn an_empty_slice_is_missing_a_required_value() {
        let names: &[&str] = &[];

        assert_eq!(required().check(names), Err(Problem::Required));
    }

    #[test]
    fn an_empty_array_is_missing_a_required_value() {
        assert_eq!(required().check(&[0_u8; 0]), Err(Problem::Required));
    }

    #[test]
    fn an_empty_queue_is_missing_a_required_value() {
        assert_eq!(required().check(&VecDeque::<u8>::new()), Err(Problem::Required));
    }

    #[test]
    fn an_empty_hash_set_is_missing_a_required_value() {
        assert_eq!(required().check(&HashSet::<u8>::new()), Err(Problem::Required));
    }

    #[test]
    fn an_empty_ordered_set_is_missing_a_required_value() {
        assert_eq!(required().check(&BTreeSet::<u8>::new()), Err(Problem::Required));
    }

    #[test]
    fn an_empty_ordered_map_is_missing_a_required_value() {
        assert_eq!(required().check(&BTreeMap::<u8, u8>::new()), Err(Problem::Required));
    }

    #[test]
    fn a_borrowed_string_is_checked_like_a_string() {
        assert_eq!(required().check(&"   "), Err(Problem::Required));
    }

    #[test]
    fn a_boxed_string_is_checked_like_a_string() {
        let name: Box<str> = Box::from("   ");

        assert_eq!(required().check(&name), Err(Problem::Required));
    }

    #[test]
    fn a_reference_counted_string_is_checked_like_a_string() {
        let name: Rc<str> = Rc::from("   ");

        assert_eq!(required().check(&name), Err(Problem::Required));
    }

    #[test]
    fn a_copy_on_write_string_is_checked_like_a_string() {
        let name: Cow<'_, str> = Cow::Borrowed("   ");

        assert_eq!(required().check(&name), Err(Problem::Required));
    }
}
