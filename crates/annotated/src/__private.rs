#[cfg(feature = "regex")]
pub use regex::Regex;
#[cfg(feature = "serde")]
pub use serde;

use crate::{FoundViolations, Location, Problem, Violation};

#[cfg(feature = "serde")]
pub use crate::collecting_type_errors::{CollectingTypeErrorsAt, NestedCollectingTypeErrorsAt};
#[cfg(feature = "serde")]
pub use crate::deserialize::DeserializeAt;

// Out of line and cold, so a rule that passes costs one branch: the benchmarks' order of 100 lines validates in
// 357 ns instead of 429 ns with LTO, and in 581 ns instead of 1.03 µs without.
#[cold]
#[inline(never)]
pub fn record_violation(
    found: &mut FoundViolations,
    at: &Location<'_>,
    problem: Problem,
    message: Option<&'static str>,
) {
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
        crate::DeserializeError::Violations(violations) => D::Error::custom(violations),
        crate::DeserializeError::Format(format_error) => format_error,
    })
}

/// The violations that a collection's elements reported while they were read. They wait here for each element's
/// `nested` rule, in the order `HasElements` goes through the elements.
pub struct ElementViolations {
    waiting: std::vec::IntoIter<Violation>,
}

impl ElementViolations {
    #[must_use]
    pub fn new(found_in_elements: FoundViolations) -> Self {
        Self {
            waiting: found_in_elements.into_violations().into_iter(),
        }
    }

    /// Moves the next waiting violations that are at `at` or under it into `found`, and returns whether there were any.
    pub fn join_those_at(&mut self, at: &Location<'_>, found: &mut FoundViolations) -> bool {
        let mut has_joined_any = false;
        while let Some(violation) = self.next_at(at) {
            found.push_violation(violation);
            has_joined_any = true;
        }
        has_joined_any
    }

    /// Drops the next waiting violations that are at `at` or under it. They belong to an element whose `nested` rule
    /// didn't run.
    pub fn skip_those_at(&mut self, at: &Location<'_>) {
        while self.next_at(at).is_some() {}
    }

    fn next_at(&mut self, at: &Location<'_>) -> Option<Violation> {
        let first_waiting = self.waiting.as_slice().first()?;
        if !at.leads_to(first_waiting.path()) {
            return None;
        }
        self.waiting.next()
    }
}
