#[cfg(feature = "regex")]
pub use regex::Regex;
#[cfg(feature = "serde")]
pub use serde;

use crate::{Issue, Location, Problem, Report};

#[cfg(feature = "serde")]
pub use crate::borrowed::{borrow_cow_bytes, borrow_cow_str};
#[cfg(feature = "serde")]
pub use crate::collecting_type_errors::{
    CollectingTypeErrorsAt, CollectingTypeErrorsOfAFunctionAt, NestedCollectingTypeErrorsAt,
};
#[cfg(feature = "serde")]
pub use crate::deserialize::DeserializeAt;

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
pub struct ElementIssues {
    waiting: std::vec::IntoIter<Issue>,
}

impl ElementIssues {
    #[must_use]
    pub fn new(found_in_elements: Report) -> Self {
        Self {
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
