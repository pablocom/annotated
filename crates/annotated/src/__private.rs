#[cfg(feature = "regex")]
pub use regex::Regex;

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
