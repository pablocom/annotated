//! `from_deserializer` reads a value from the root, and answers it, the format's error or every issue in it.

#![cfg(feature = "serde")]

use std::error::Error;
use std::{fmt, io};

use annotated::{DeserializeError, Issue, Location, Path, Problem, Report};
use serde::Deserializer;

#[derive(Debug, PartialEq)]
struct Handle(String);

fn not_a_handle() -> Problem {
    Problem::custom("handle", "must start with @")
}

fn too_short() -> Problem {
    Problem::custom("too_short", "must be 3 characters or more")
}

impl<'de> annotated::Deserialize<'de> for Handle {
    fn deserialize_at<D: Deserializer<'de>>(
        deserializer: D,
        at: &Location<'_>,
        found: &mut Report,
    ) -> Result<Option<Self>, D::Error> {
        let Some(handle) = <Option<String> as serde::Deserialize>::deserialize(deserializer)? else {
            found.push(at, Problem::Missing);
            return Ok(None);
        };
        if !handle.starts_with('@') {
            found.push(at, not_a_handle());
        }
        if handle.chars().count() < 3 {
            found.push(at, too_short());
        }
        Ok(Some(Self(handle)))
    }
}

fn handle_from(json: &str) -> Result<Handle, DeserializeError<serde_json::Error>> {
    annotated::from_deserializer(&mut serde_json::Deserializer::from_str(json))
}

#[test]
fn a_value_breaking_no_rule_is_answered() {
    assert_eq!(handle_from(r#""@lucia""#).unwrap(), Handle("@lucia".to_owned()));
}

#[test]
fn every_issue_in_a_value_is_answered_in_the_order_it_was_found() {
    let error = handle_from(r#""l""#).unwrap_err();

    assert_eq!(
        error.issues(),
        [
            Issue::new(Path::root(), not_a_handle()),
            Issue::new(Path::root(), too_short())
        ]
    );
}

#[test]
fn a_value_that_could_not_be_built_answers_the_issues_saying_why() {
    let error = handle_from("null").unwrap_err();

    assert_eq!(error.issues(), [Issue::new(Path::root(), Problem::Missing)]);
}

#[test]
fn issues_read_as_one_issue_per_line() {
    let error = handle_from(r#""l""#).unwrap_err();

    assert_eq!(error.to_string(), "must start with @\nmust be 3 characters or more");
}

#[test]
fn input_that_is_not_well_formed_is_the_format_s_error_with_no_issues() {
    let error = handle_from(r#""@lucia"#).unwrap_err();

    assert!(matches!(error, DeserializeError::Format(_)));
    assert_eq!(error.issues(), []);
}

#[test]
fn the_format_s_error_reads_as_the_format_wrote_it() {
    let error = handle_from(r#""@lucia"#).unwrap_err();

    assert_eq!(error.to_string(), "EOF while parsing a string at line 1 column 7");
}

#[derive(Debug)]
struct ConnectionLost(io::Error);

impl fmt::Display for ConnectionLost {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("the connection was lost")
    }
}

impl Error for ConnectionLost {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

#[test]
fn the_source_of_a_format_error_is_the_format_error_s_own_source() {
    let error = DeserializeError::Format(ConnectionLost(io::Error::other("disconnected")));

    assert_eq!(error.source().map(ToString::to_string), Some("disconnected".to_owned()));
}

#[test]
fn failing_fast_answers_only_the_first_issue() {
    let mut deserializer = serde_json::Deserializer::from_str(r#""l""#);

    let error = annotated::from_deserializer_fail_fast::<Handle, _>(&mut deserializer).unwrap_err();

    assert_eq!(error.issues(), [Issue::new(Path::root(), not_a_handle())]);
}

#[test]
fn failing_fast_answers_a_value_breaking_no_rule() {
    let mut deserializer = serde_json::Deserializer::from_str(r#""@lucia""#);

    let handle = annotated::from_deserializer_fail_fast::<Handle, _>(&mut deserializer).unwrap();

    assert_eq!(handle, Handle("@lucia".to_owned()));
}

#[derive(Debug)]
struct Unbuildable;

impl<'de> annotated::Deserialize<'de> for Unbuildable {
    fn deserialize_at<D: Deserializer<'de>>(
        deserializer: D,
        _: &Location<'_>,
        _: &mut Report,
    ) -> Result<Option<Self>, D::Error> {
        <serde::de::IgnoredAny as serde::Deserialize>::deserialize(deserializer)?;
        Ok(None)
    }
}

#[test]
fn a_value_that_could_not_be_built_with_no_issue_saying_why_is_the_format_s_error() {
    let mut deserializer = serde_json::Deserializer::from_str("{}");

    let error = annotated::from_deserializer::<Unbuildable, _>(&mut deserializer).unwrap_err();

    assert_eq!(error.to_string(), "the value couldn't be built, and no issue says why");
}
