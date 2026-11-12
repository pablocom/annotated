//! `#[derive(Deserialize)]` reads a field with `#[serde(deserialize_with = "...")]` or `#[serde(with = "...")]` through
//! the function given, and checks the value it returns by the field's rules like any other.

#![cfg(feature = "serde")]

use std::fmt::Display;
use std::str::FromStr;

use annotated::{Deserialize, DeserializeError, Issue, Validate};
use serde::de::{Error as _, Unexpected};
use serde::{Deserialize as _, Deserializer};

fn deserialize<'de, T: Deserialize<'de>>(json: &'de str) -> Result<T, DeserializeError<serde_json::Error>> {
    annotated::from_deserializer(&mut serde_json::Deserializer::from_str(json))
}

fn issues_found_reading<'de, T: Deserialize<'de>>(json: &'de str) -> Vec<Issue> {
    match deserialize::<T>(json) {
        Ok(_) => Vec::new(),
        Err(DeserializeError::Issues(issues)) => issues.into_iter().collect(),
        Err(error) => panic!("expected issues, got the format's error: {error}"),
    }
}

fn issues_reading<'de, T: Deserialize<'de>>(json: &'de str) -> Vec<String> {
    issues_found_reading::<T>(json).iter().map(path_and_code).collect()
}

fn path_and_code(issue: &Issue) -> String {
    let code = issue.problem().code();
    if issue.path().is_root() {
        return code.to_owned();
    }
    format!("{}: {code}", issue.path())
}

fn format_error_reading<'de, T: Deserialize<'de>>(json: &'de str) -> String {
    match deserialize::<T>(json) {
        Err(error @ DeserializeError::Format(_)) => error.to_string(),
        Ok(_) => panic!("expected the format's error, got a value"),
        Err(error) => panic!("expected the format's error, got {error:?}"),
    }
}

fn issues_validating_after_serde<T: serde::de::DeserializeOwned + Validate>(json: &str) -> Vec<Issue> {
    let value = serde_json::from_str::<T>(json).unwrap();
    value
        .validate()
        .map_or_else(|issues| issues.into_iter().collect(), |()| Vec::new())
}

fn described_issues_reading<'de, T: Deserialize<'de>>(json: &'de str) -> Vec<String> {
    issues_found_reading::<T>(json).iter().map(described).collect()
}

fn described(issue: &Issue) -> String {
    let params = issue.problem().params();
    let params = params
        .iter()
        .map(|(name, value)| format!("{name}: {value}"))
        .collect::<Vec<_>>();
    let code_and_params = if params.is_empty() {
        issue.problem().code().to_owned()
    } else {
        format!("{}({})", issue.problem().code(), params.join(", "))
    };
    format!("{}: {code_and_params}", issue.path())
}

fn comma_separated<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    let text = String::deserialize(deserializer)?;
    if text.starts_with(',') {
        return Err(D::Error::custom(format!("{text} starts with a comma")));
    }
    Ok(text.split(',').map(str::to_owned).collect())
}

mod cents {
    use serde::Deserializer;
    use serde::de::{Error as _, Unexpected};

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u32, D::Error> {
        let euros = <i32 as serde::Deserialize>::deserialize(deserializer)?;
        let euros = u32::try_from(euros)
            .map_err(|_| D::Error::invalid_value(Unexpected::Signed(euros.into()), &"an amount of zero or more"))?;
        Ok(euros.saturating_mul(100))
    }
}

#[derive(Deserialize, Debug, PartialEq)]
struct Product {
    #[serde(deserialize_with = "comma_separated")]
    #[validate(length(max = 3))]
    tags: Vec<String>,
    #[serde(with = "cents")]
    #[validate(range(min = 1))]
    price_in_cents: u32,
}

#[test]
fn a_field_is_read_by_its_deserialize_with_function() {
    let product = deserialize::<Product>(r#"{ "tags": "red,wool", "price_in_cents": 12 }"#).unwrap();

    assert_eq!(
        product,
        Product {
            tags: vec!["red".to_owned(), "wool".to_owned()],
            price_in_cents: 1200,
        }
    );
}

#[test]
fn a_field_is_read_by_the_deserialize_function_of_its_with_module() {
    let product = deserialize::<Product>(r#"{ "tags": "red", "price_in_cents": 1 }"#).unwrap();

    assert_eq!(
        product,
        Product {
            tags: vec!["red".to_owned()],
            price_in_cents: 100,
        }
    );
}

#[test]
fn the_value_a_function_returns_is_checked_by_the_field_s_rules() {
    let body = r#"{ "tags": "red,wool,winter,sale", "price_in_cents": 0 }"#;

    assert_eq!(
        issues_reading::<Product>(body),
        ["tags: length", "price_in_cents: range"]
    );
}

#[test]
fn a_struct_read_from_a_sequence_reads_each_field_with_its_function() {
    let product = deserialize::<Product>(r#"["red,wool", 3]"#).unwrap();

    assert_eq!(
        product,
        Product {
            tags: vec!["red".to_owned(), "wool".to_owned()],
            price_in_cents: 300,
        }
    );
}

#[test]
fn the_function_s_error_ends_deserialization() {
    let body = r#"{ "tags": ",red", "price_in_cents": 3 }"#;

    assert_eq!(
        format_error_reading::<Product>(body),
        ",red starts with a comma at line 1 column 16"
    );
}

#[test]
fn an_absent_field_read_by_a_function_is_missing() {
    assert_eq!(
        issues_reading::<Product>(r#"{ "tags": "red" }"#),
        ["price_in_cents: missing"]
    );
}

fn optional_comma_separated<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Vec<String>>, D::Error> {
    comma_separated(deserializer).map(Some)
}

#[derive(Deserialize, Debug, PartialEq)]
struct Filters {
    #[serde(deserialize_with = "optional_comma_separated")]
    tags: Option<Vec<String>>,
    #[serde(deserialize_with = "optional_comma_separated", default)]
    colors: Option<Vec<String>>,
}

#[test]
fn an_absent_optional_field_read_by_a_function_is_missing_as_with_serde() {
    assert_eq!(issues_reading::<Filters>(r#"{ "colors": "red" }"#), ["tags: missing"]);
}

#[test]
fn an_absent_field_read_by_a_function_takes_its_default() {
    let filters = deserialize::<Filters>(r#"{ "tags": "wool" }"#).unwrap();

    assert_eq!(
        filters,
        Filters {
            tags: Some(vec!["wool".to_owned()]),
            colors: None,
        }
    );
}

#[derive(Validate, Debug, PartialEq)]
struct Dimensions {
    #[validate(range(min = 1))]
    width: u32,
    #[validate(range(min = 1))]
    height: u32,
}

fn width_by_height<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Dimensions, D::Error> {
    let text = String::deserialize(deserializer)?;
    let (width, height) = text
        .split_once('x')
        .ok_or_else(|| D::Error::invalid_value(Unexpected::Other("text without an x"), &"width x height"))?;
    let measure = |side: &str| {
        side.parse()
            .map_err(|_| D::Error::invalid_value(Unexpected::Other("text"), &"a whole number"))
    };
    Ok(Dimensions {
        width: measure(width)?,
        height: measure(height)?,
    })
}

fn every_width_by_height<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<Dimensions>, D::Error> {
    let texts = Vec::<String>::deserialize(deserializer)?;
    let dimensions = texts
        .into_iter()
        .map(|text| width_by_height(serde::de::value::StringDeserializer::<D::Error>::new(text)));
    dimensions.collect()
}

#[derive(Deserialize, Debug)]
struct Frame {
    #[serde(deserialize_with = "width_by_height")]
    #[validate(nested)]
    #[allow(dead_code)]
    size: Dimensions,
    #[serde(deserialize_with = "every_width_by_height")]
    #[validate(each(nested))]
    #[allow(dead_code)]
    mats: Vec<Dimensions>,
}

#[test]
fn a_nested_value_read_by_a_function_is_validated_at_its_name() {
    assert_eq!(
        issues_reading::<Frame>(r#"{ "size": "0x20", "mats": ["1x1", "2x0"] }"#),
        ["size.width: range", "mats[1].height: range"]
    );
}

#[derive(Deserialize, Debug)]
#[annotated(collect_type_errors)]
struct FrameCollectingTypeErrors {
    #[serde(deserialize_with = "width_by_height")]
    #[validate(nested)]
    #[allow(dead_code)]
    size: Dimensions,
    #[validate(required)]
    #[allow(dead_code)]
    title: String,
}

#[test]
fn under_collect_type_errors_a_nested_value_read_by_a_function_is_validated_at_its_name() {
    assert_eq!(
        issues_reading::<FrameCollectingTypeErrors>(r#"{ "size": "0x20", "title": "" }"#),
        ["size.width: range", "title: required"]
    );
}

#[derive(Deserialize, Debug, PartialEq)]
#[annotated(collect_type_errors)]
struct ProductCollectingTypeErrors {
    #[serde(deserialize_with = "comma_separated")]
    tags: Vec<String>,
    #[serde(with = "cents")]
    price_in_cents: u32,
    #[validate(required)]
    name: String,
}

#[test]
fn under_collect_type_errors_a_value_of_the_wrong_type_for_the_function_is_an_issue_at_the_field() {
    let body = r#"{ "tags": 7, "price_in_cents": 3, "name": "" }"#;

    assert_eq!(
        described_issues_reading::<ProductCollectingTypeErrors>(body),
        [
            "tags: invalid_type(expected: a string, found: integer)",
            "name: required"
        ]
    );
}

#[test]
fn under_collect_type_errors_a_value_the_function_finds_out_of_range_is_an_issue_at_the_field() {
    let body = r#"{ "tags": "red", "price_in_cents": -3, "name": "Scarf" }"#;

    assert_eq!(
        described_issues_reading::<ProductCollectingTypeErrors>(body),
        ["price_in_cents: invalid_type(expected: an amount of zero or more, found: integer)"]
    );
}

#[test]
fn under_collect_type_errors_the_function_s_own_error_ends_deserialization_without_its_message_as_an_issue() {
    let body = r#"{ "tags": ",red", "price_in_cents": 3, "name": "" }"#;

    assert_eq!(
        format_error_reading::<ProductCollectingTypeErrors>(body),
        ",red starts with a comma at line 1 column 16"
    );
}

#[test]
fn under_collect_type_errors_a_value_the_function_reads_is_deserialized() {
    let body = r#"{ "tags": "red", "price_in_cents": 3, "name": "Scarf" }"#;

    let product = deserialize::<ProductCollectingTypeErrors>(body).unwrap();

    assert_eq!(
        product,
        ProductCollectingTypeErrors {
            tags: vec!["red".to_owned()],
            price_in_cents: 300,
            name: "Scarf".to_owned(),
        }
    );
}

fn width_and_height<'de, D: Deserializer<'de>>(deserializer: D) -> Result<(u32, u32), D::Error> {
    <(u32, u32)>::deserialize(deserializer)
}

#[derive(Deserialize, Debug)]
#[annotated(collect_type_errors)]
struct Poster {
    #[serde(deserialize_with = "width_and_height")]
    #[allow(dead_code)]
    size: (u32, u32),
}

#[test]
fn under_collect_type_errors_a_sequence_of_the_wrong_length_for_the_function_is_an_issue_at_the_field() {
    assert_eq!(
        described_issues_reading::<Poster>(r#"{ "size": [60, 90, 120] }"#),
        ["size: invalid_type(expected: 2 elements in sequence, found: sequence)"]
    );
}

#[derive(Debug, PartialEq)]
struct Celsius(i32);

impl FromStr for Celsius {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, String> {
        let degrees = text.strip_suffix("°C").ok_or_else(|| format!("{text} isn't in °C"))?;
        degrees
            .parse()
            .map(Self)
            .map_err(|_| format!("{degrees} isn't a number"))
    }
}

fn from_text<'de, D: Deserializer<'de>, T: FromStr<Err: Display>>(deserializer: D) -> Result<T, D::Error> {
    String::deserialize(deserializer)?.parse().map_err(D::Error::custom)
}

#[derive(Deserialize, Debug, PartialEq)]
struct Reading<T: FromStr<Err: Display>> {
    #[serde(deserialize_with = "from_text")]
    value: T,
}

#[test]
fn a_field_whose_type_is_a_type_parameter_is_read_by_its_function_without_a_deserialize_bound() {
    let reading = deserialize::<Reading<Celsius>>(r#"{ "value": "21°C" }"#).unwrap();

    assert_eq!(reading, Reading { value: Celsius(21) });
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct Gallery {
    #[validate(required)]
    title: String,
    #[serde(deserialize_with = "width_by_height")]
    #[validate(nested)]
    poster: Dimensions,
    #[serde(deserialize_with = "comma_separated")]
    #[validate(length(max = 1), each(length(min = 3)))]
    tags: Vec<String>,
}

#[derive(serde::Deserialize, Validate)]
struct GalleryThenValidated {
    #[validate(required)]
    title: String,
    #[serde(deserialize_with = "width_by_height")]
    #[validate(nested)]
    poster: Dimensions,
    #[serde(deserialize_with = "comma_separated")]
    #[validate(length(max = 1), each(length(min = 3)))]
    tags: Vec<String>,
}

#[test]
fn a_body_with_fields_read_by_functions_is_reported_as_serde_then_validate_report_it() {
    let body = r#"{ "title": "", "poster": "0x0", "tags": "a,bc" }"#;

    assert_eq!(
        issues_found_reading::<Gallery>(body),
        issues_validating_after_serde::<GalleryThenValidated>(body)
    );
}
