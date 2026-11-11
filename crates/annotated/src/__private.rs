#[cfg(feature = "regex")]
pub use regex::Regex;

use crate::{FoundViolations, Location, Problem};

pub fn passes(
    outcome: Result<(), Problem>,
    at: &Location<'_>,
    found: &mut FoundViolations,
    message: Option<&'static str>,
) -> bool {
    let Err(problem) = outcome else {
        return true;
    };
    match message {
        Some(message) => found.push_with_message(at, problem, message),
        None => found.push(at, problem),
    }
    false
}
