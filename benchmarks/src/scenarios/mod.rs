//! The scenarios, each in its own module, with every library's version of its types side by side so the rules
//! can be compared at a glance.

pub mod end_to_end;
pub mod end_to_end_missing_field;
pub mod end_to_end_nested;
pub mod flat;
pub mod long_strings;
pub mod nested;

use crate::Case;

/// Every case of every scenario, in the order they are reported.
#[must_use]
pub fn all() -> Vec<Case> {
    [
        flat::cases(),
        nested::cases(),
        long_strings::cases(),
        end_to_end::cases(),
        end_to_end_nested::cases(),
        end_to_end_missing_field::cases(),
    ]
    .into_iter()
    .flatten()
    .collect()
}

pub(crate) mod count {
    //! How many violations each library's outcome holds.

    use validator::ValidationErrorsKind;

    pub fn annotated(outcome: &Result<(), annotated::Violations>) -> usize {
        outcome
            .as_ref()
            .err()
            .map_or(0, |violations| violations.as_slice().len())
    }

    /// The violations annotated's collecting deserialization found, or `usize::MAX` for the format's error, so a
    /// body that couldn't be read can't pass for one without violations.
    pub fn annotated_deserialized<T>(outcome: &Result<T, annotated::DeserializeError<serde_json::Error>>) -> usize {
        match outcome {
            Ok(_) => 0,
            Err(annotated::DeserializeError::Violations(violations)) => violations.as_slice().len(),
            Err(_) => usize::MAX,
        }
    }

    pub fn validator(outcome: &Result<(), validator::ValidationErrors>) -> usize {
        outcome.as_ref().err().map_or(0, validator_errors)
    }

    fn validator_errors(errors: &validator::ValidationErrors) -> usize {
        errors
            .errors()
            .values()
            .map(|kind| match kind {
                ValidationErrorsKind::Field(field_errors) => field_errors.len(),
                ValidationErrorsKind::Struct(nested_errors) => validator_errors(nested_errors),
                ValidationErrorsKind::List(element_errors) => {
                    element_errors.values().map(|errors| validator_errors(errors)).sum()
                }
            })
            .sum()
    }

    pub fn garde(outcome: &Result<(), garde::Report>) -> usize {
        outcome.as_ref().err().map_or(0, |report| report.iter().count())
    }

    pub fn wary(outcome: &Result<(), wary::Report>) -> usize {
        outcome.as_ref().err().map_or(0, wary::Report::len)
    }

    pub fn serde_valid(outcome: &Result<(), serde_valid::validation::Errors>) -> usize {
        outcome.as_ref().err().map_or(0, serde_valid_errors)
    }

    fn serde_valid_errors(errors: &serde_valid::validation::Errors) -> usize {
        let as_json = serde_json::to_value(errors).expect("serde_valid errors serialize to JSON");
        messages_in(&as_json)
    }

    fn messages_in(errors: &serde_json::Value) -> usize {
        match errors {
            serde_json::Value::String(_) => 1,
            serde_json::Value::Array(values) => values.iter().map(messages_in).sum(),
            serde_json::Value::Object(entries) => entries.values().map(messages_in).sum(),
            _ => 0,
        }
    }
}
