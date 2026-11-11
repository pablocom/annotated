#[cfg(feature = "regex")]
pub use regex::Regex;
#[cfg(feature = "serde")]
pub use serde;

use crate::{FoundViolations, Location, Problem};

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
