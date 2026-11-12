//! What changes to annotated would gain, measured in the same binary as the scenarios so the compiler makes
//! the same choices for both. Each experiment puts annotated as it is beside code written by hand through
//! its public API, the way the change would make it run, with the same rules and the same issues.

use std::sync::LazyLock;

use annotated::{Location, Problem, Report, Rule, Validate, rules};
use regex::Regex;
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::scenarios::flat::{self, parsed};
use crate::scenarios::{count, long_strings, nested};
use crate::{Case, Library};

static USERNAME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[a-z0-9_]+$").unwrap());
static POSTAL_CODE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\d{5}$").unwrap());

/// Records an issue away from the checks, so a check that passes costs one branch.
#[cold]
#[inline(never)]
fn record(found: &mut Report, at: &Location<'_>, field: &'static str, problem: Problem) {
    found.push(&at.field(field), problem);
}

/// The order of scenario 4, checked with every rule inline and the issue recorded out of line.
#[derive(Deserialize)]
pub struct InlineChecksOrder {
    customer: InlineChecksCustomer,
    lines: Vec<InlineChecksOrderLine>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InlineChecksCustomer {
    name: String,
    postal_code: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InlineChecksOrderLine {
    sku: String,
    quantity: u32,
    unit_price: f64,
    note: Option<String>,
}

impl Validate for InlineChecksOrder {
    fn validate_into(&self, at: &Location<'_>, found: &mut Report) {
        self.customer.validate_into(&at.field("customer"), found);
        let lines_at = at.field("lines");
        for (index, line) in self.lines.iter().enumerate() {
            line.validate_into(&lines_at.index(index), found);
        }
    }
}

impl Validate for InlineChecksCustomer {
    fn validate_into(&self, at: &Location<'_>, found: &mut Report) {
        if let Err(problem) = rules::length().min(1).max(50).check(&self.name) {
            record(found, at, "name", problem);
        }
        if let Err(problem) = rules::pattern(&POSTAL_CODE).check(&self.postal_code) {
            record(found, at, "postalCode", problem);
        }
    }
}

impl Validate for InlineChecksOrderLine {
    fn validate_into(&self, at: &Location<'_>, found: &mut Report) {
        if let Err(problem) = rules::length().min(4).max(16).check(&self.sku) {
            record(found, at, "sku", problem);
        }
        if let Err(problem) = rules::range().min(1).max(99).check(&self.quantity) {
            record(found, at, "quantity", problem);
        }
        if let Err(problem) = rules::range().min(0.0).max(10_000.0).check(&self.unit_price) {
            record(found, at, "unitPrice", problem);
        }
        if let Some(note) = &self.note
            && let Err(problem) = rules::length().max(200).check(note)
        {
            record(found, at, "note", problem);
        }
    }
}

/// The sign-up body of scenarios 1 to 3, checked with every rule inline and the issue recorded out of
/// line, the first failure of a field stopping its other rules.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InlineChecksSignUp {
    username: String,
    full_name: Option<String>,
    email: String,
    website: Option<String>,
    age: u8,
    rating: f64,
    postal_code: String,
    bio: Option<String>,
}

impl Validate for InlineChecksSignUp {
    fn validate_into(&self, at: &Location<'_>, found: &mut Report) {
        if let Err(problem) = rules::length().min(3).max(30).check(&self.username) {
            record(found, at, "username", problem);
        } else if let Err(problem) = rules::pattern(&USERNAME).check(&self.username) {
            record(found, at, "username", problem);
        }
        match &self.full_name {
            None => record(found, at, "fullName", Problem::Required),
            Some(full_name) => {
                if let Err(problem) = rules::length().max(50).check(full_name) {
                    record(found, at, "fullName", problem);
                }
            }
        }
        if let Err(problem) = rules::email().check(&self.email) {
            record(found, at, "email", problem);
        }
        if let Some(website) = &self.website
            && let Err(problem) = rules::url().check(website)
        {
            record(found, at, "website", problem);
        }
        if let Err(problem) = rules::range().min(18).max(120).check(&self.age) {
            record(found, at, "age", problem);
        }
        if let Err(problem) = rules::range().min(0.0).max(5.0).check(&self.rating) {
            record(found, at, "rating", problem);
        }
        if let Err(problem) = rules::pattern(&POSTAL_CODE).check(&self.postal_code) {
            record(found, at, "postalCode", problem);
        }
        if let Some(bio) = &self.bio
            && let Err(problem) = rules::length().max(280).check(bio)
        {
            record(found, at, "bio", problem);
        }
    }
}

const WEBSITE: &str = "https://example.com/alex";

/// `length` in chars, settled by the byte length where it can be: a string has no more chars than bytes,
/// and no fewer than a quarter of its bytes, so only a byte length between the two needs the chars counted.
fn length_settled_by_bytes_first(text: &str, min: usize, max: usize) -> Result<(), Problem> {
    let bytes = text.len();
    let is_short_enough = bytes <= max;
    let is_long_enough = bytes.div_ceil(4) >= min;
    if is_short_enough && is_long_enough {
        return Ok(());
    }
    rules::length().min(min).max(max).check(text)
}

#[must_use]
pub fn cases() -> Vec<Case> {
    [inline_checks_cases(), url_cases(), length_cases()]
        .into_iter()
        .flatten()
        .collect()
}

fn inline_checks_cases() -> Vec<Case> {
    let order_body = nested::order_body(false);
    let every_field_invalid_body = flat::every_field_invalid_body();
    vec![
        derived::<nested::with_annotated::Order>("experiment_inline_checks_order", &order_body),
        checked_inline::<InlineChecksOrder>("experiment_inline_checks_order", &order_body),
        derived::<flat::with_annotated::SignUp>("experiment_inline_checks_flat", flat::VALID_BODY),
        checked_inline::<InlineChecksSignUp>("experiment_inline_checks_flat", flat::VALID_BODY),
        derived::<flat::with_annotated::SignUp>(
            "experiment_inline_checks_flat_every_field_invalid",
            &every_field_invalid_body,
        ),
        checked_inline::<InlineChecksSignUp>(
            "experiment_inline_checks_flat_every_field_invalid",
            &every_field_invalid_body,
        ),
    ]
}

fn derived<T: DeserializeOwned + Validate + 'static>(experiment: &'static str, body: &str) -> Case {
    Case::new(
        experiment,
        Library::Annotated,
        parsed::<T>(body),
        Validate::validate,
        count::annotated,
    )
    .variant("derived")
}

fn checked_inline<T: DeserializeOwned + Validate + 'static>(experiment: &'static str, body: &str) -> Case {
    Case::new(
        experiment,
        Library::HandWritten,
        parsed::<T>(body),
        Validate::validate,
        count::annotated,
    )
    .variant("inline checks, issues recorded out of line")
}

fn url_cases() -> Vec<Case> {
    vec![
        Case::new(
            "experiment_url",
            Library::Annotated,
            WEBSITE,
            |website| rules::url().check(*website),
            |outcome| usize::from(outcome.is_err()),
        )
        .variant("rules::url()"),
        Case::new(
            "experiment_url",
            Library::HandWritten,
            WEBSITE,
            |website| url::Url::parse(website),
            |outcome| usize::from(outcome.is_err()),
        )
        .variant("url::Url::parse alone, as garde and validator check"),
    ]
}

fn length_cases() -> Vec<Case> {
    let long_text = long_strings::prose_of(64 * 1024);
    let username = "alex_morgan".to_owned();
    vec![
        Case::new(
            "experiment_length_short_text",
            Library::Annotated,
            username.clone(),
            |username| rules::length().min(3).max(30).check(username),
            |outcome| usize::from(outcome.is_err()),
        )
        .variant("rules::length(), chars counted"),
        Case::new(
            "experiment_length_short_text",
            Library::HandWritten,
            username,
            |username| length_settled_by_bytes_first(username, 3, 30),
            |outcome| usize::from(outcome.is_err()),
        )
        .variant("settled by the byte length first"),
        Case::new(
            "experiment_length_64kb",
            Library::Annotated,
            long_text.clone(),
            |text| rules::length().max(100_000).check(text),
            |outcome| usize::from(outcome.is_err()),
        )
        .variant("rules::length(), chars counted"),
        Case::new(
            "experiment_length_64kb",
            Library::HandWritten,
            long_text,
            |text| length_settled_by_bytes_first(text, 0, 100_000),
            |outcome| usize::from(outcome.is_err()),
        )
        .variant("settled by the byte length first"),
    ]
}
