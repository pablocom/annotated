//! The same validation scenarios, written in each library's own syntax, and run the same way.
//!
//! A [`Case`] is one library validating one prepared input of one scenario. The criterion benchmark times
//! every case, the `allocations` binary counts what every case allocates, and the tests check that every
//! case finds the issues its scenario expects.

pub mod experiments;
pub mod scenarios;

use std::fmt::{self, Display};
use std::hint::black_box;
use std::rc::Rc;

/// A validation library under measurement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Library {
    Annotated,
    Validator,
    Garde,
    Wary,
    SerdeValid,
    /// `serde_json` alone, without validation: the floor of the end-to-end scenario.
    SerdeJsonAlone,
    /// Code written by hand through annotated's public API, the way a change to annotated would make it run,
    /// to measure what that change would gain.
    HandWritten,
}

impl Library {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Annotated => "annotated",
            Self::Validator => "validator",
            Self::Garde => "garde",
            Self::Wary => "wary",
            Self::SerdeValid => "serde_valid",
            Self::SerdeJsonAlone => "serde_json alone",
            Self::HandWritten => "hand-written",
        }
    }

    /// The version measured, as `Cargo.lock` resolves it.
    #[must_use]
    pub const fn version(self) -> &'static str {
        match self {
            Self::Annotated => "this checkout",
            Self::Validator => "0.21.0",
            Self::Garde => "0.23.0",
            Self::Wary => "0.3.2",
            Self::SerdeValid => "3.2.0",
            Self::SerdeJsonAlone => "1.0.154",
            Self::HandWritten => "on this checkout",
        }
    }
}

/// One library validating one prepared input of one scenario.
pub struct Case {
    pub scenario: &'static str,
    pub library: Library,
    /// How this library's way differs from its plain default, like `report_all` or counting bytes.
    pub variant: Option<&'static str>,
    /// The bytes of text one validation goes through, for scenarios measured in throughput.
    pub throughput_bytes: Option<u64>,
    validate_once: Box<dyn Fn()>,
    count_issues: Box<dyn Fn() -> usize>,
}

impl Case {
    /// The case of `library` checking `input` with `validate`, whose outcome `count` turns into a number of
    /// issues.
    pub fn new<Input, Outcome>(
        scenario: &'static str,
        library: Library,
        input: Input,
        validate: impl Fn(&Input) -> Outcome + 'static,
        count: impl Fn(&Outcome) -> usize + 'static,
    ) -> Self
    where
        Input: 'static,
    {
        let input = Rc::new(input);
        let validate = Rc::new(validate);
        let timed_input = Rc::clone(&input);
        let timed_validate = Rc::clone(&validate);
        Self {
            scenario,
            library,
            variant: None,
            throughput_bytes: None,
            validate_once: Box::new(move || {
                black_box(timed_validate(black_box(&*timed_input)));
            }),
            count_issues: Box::new(move || count(&validate(&input))),
        }
    }

    #[must_use]
    pub fn variant(self, variant: &'static str) -> Self {
        Self {
            variant: Some(variant),
            ..self
        }
    }

    #[must_use]
    pub fn throughput_bytes(self, bytes: usize) -> Self {
        Self {
            throughput_bytes: Some(bytes as u64),
            ..self
        }
    }

    /// Validates the prepared input once and drops the outcome, the work a benchmark iteration measures.
    pub fn validate_once(&self) {
        (self.validate_once)();
    }

    /// How many issues validating the prepared input finds.
    #[must_use]
    pub fn issues(&self) -> usize {
        (self.count_issues)()
    }

    /// The name of this case within its scenario: the library, and its variant if any.
    #[must_use]
    pub fn label(&self) -> String {
        match self.variant {
            Some(variant) => format!("{} ({variant})", self.library.name()),
            None => self.library.name().to_owned(),
        }
    }
}

impl Display for Case {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}/{}", self.scenario, self.label())
    }
}

/// Every case of every scenario, then every experiment, in the order they are reported.
#[must_use]
pub fn all_cases() -> Vec<Case> {
    scenarios::all().into_iter().chain(experiments::cases()).collect()
}
