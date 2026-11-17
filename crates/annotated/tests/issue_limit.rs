//! A report keeps at most its limit of issues: 1,000 unless the call asks for another. However many issues a body
//! has, annotated holds no more than that. The issues kept are the first ones the full report lists, and
//! `Issues::is_complete` says whether the value may have more.

#![cfg(feature = "json")]

use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;
use std::hash::{BuildHasherDefault, Hasher};

use annotated::{Deserialize, DeserializeError, Issue, Issues, Location, Path, Problem, Report, Validate};
use serde::Deserializer;

#[derive(Deserialize, Validate, Debug)]
struct Order {
    #[validate(each(nested))]
    lines: Vec<Line>,
}

#[derive(Deserialize, Validate, Debug, Clone)]
struct Line {
    #[validate(required)]
    product: String,
    #[validate(required)]
    unit: String,
    #[validate(range(min = 1))]
    quantity: u32,
    #[validate(required)]
    note: String,
}

fn order_of_lines_with_no_fields(lines: usize) -> String {
    format!(r#"{{ "lines": [{}] }}"#, vec!["{}"; lines].join(", "))
}

fn order_of_blank_lines(lines: usize) -> Order {
    let blank_line = Line {
        product: String::new(),
        unit: String::new(),
        quantity: 0,
        note: String::new(),
    };
    Order {
        lines: vec![blank_line; lines],
    }
}

fn lines_by_product_with_no_fields(products: usize) -> String {
    let lines: Vec<String> = (0..products)
        .map(|product| format!(r#""{product:04}": {{}}"#))
        .collect();
    format!("{{ {} }}", lines.join(", "))
}

fn issues_of<T: Debug>(outcome: Result<T, DeserializeError<serde_json::Error>>) -> Issues {
    match outcome {
        Err(DeserializeError::Issues(issues)) => issues,
        outcome => panic!("expected issues, got {outcome:?}"),
    }
}

fn paths_and_codes(issues: &Issues) -> Vec<String> {
    issues.iter().map(path_and_code).collect()
}

fn path_and_code(issue: &Issue) -> String {
    format!("{}: {}", issue.path(), issue.problem().code())
}

#[test]
fn reading_a_body_keeps_a_thousand_issues_by_default() {
    let body = order_of_lines_with_no_fields(5_000);

    let error = annotated::json::from_str::<Order>(&body).unwrap_err();

    assert_eq!(error.issues().len(), 1_000);
}

#[test]
fn the_issues_kept_are_the_first_ones_found() {
    let body = order_of_lines_with_no_fields(5_000);

    let error = annotated::json::from_str::<Order>(&body).unwrap_err();

    assert_eq!(
        error.issues().last(),
        Some(&Issue::new(
            Path::root().field("lines").index(249).field("note"),
            Problem::Missing
        ))
    );
}

#[test]
fn issues_cut_off_at_the_limit_are_not_complete() {
    let body = order_of_lines_with_no_fields(5_000);

    let issues = issues_of(annotated::json::from_str::<Order>(&body));

    assert!(!issues.is_complete());
}

#[test]
fn issues_fewer_than_the_limit_are_complete() {
    let body = order_of_lines_with_no_fields(3);

    let issues = issues_of(annotated::json::from_str::<Order>(&body));

    assert!(issues.is_complete());
}

#[test]
fn validating_a_value_keeps_a_thousand_issues_by_default() {
    let order = order_of_blank_lines(5_000);

    let issues = order.validate().unwrap_err();

    assert_eq!((issues.as_slice().len(), issues.is_complete()), (1_000, false));
}

#[test]
fn serde_s_own_deserialize_keeps_a_thousand_issues_by_default() {
    let body = order_of_lines_with_no_fields(5_000);

    let error = serde_json::from_str::<Order>(&body).unwrap_err();

    assert_eq!(error.to_string().lines().count(), 1_000);
}

#[test]
fn reading_text_limited_to_fifty_issues_keeps_fifty() {
    let body = order_of_lines_with_no_fields(5_000);

    let error = annotated::json::from_str_limited_to::<Order>(&body, 50).unwrap_err();

    assert_eq!(error.issues().len(), 50);
}

#[test]
fn reading_bytes_limited_to_fifty_issues_keeps_fifty() {
    let body = order_of_lines_with_no_fields(5_000);

    let error = annotated::json::from_slice_limited_to::<Order>(body.as_bytes(), 50).unwrap_err();

    assert_eq!(error.issues().len(), 50);
}

#[test]
fn reading_from_a_deserializer_limited_to_fifty_issues_keeps_fifty() {
    let body = order_of_lines_with_no_fields(5_000);
    let mut deserializer = serde_json::Deserializer::from_str(&body);

    let error = annotated::from_deserializer_limited_to::<Order, _>(&mut deserializer, 50).unwrap_err();

    assert_eq!(error.issues().len(), 50);
}

#[test]
fn validating_a_value_limited_to_fifty_issues_keeps_fifty() {
    let order = order_of_blank_lines(5_000);

    let issues = order.validate_limited_to(50).unwrap_err();

    assert_eq!(issues.as_slice().len(), 50);
}

#[test]
fn reading_a_body_with_a_limit_of_zero_keeps_one_issue() {
    let body = order_of_lines_with_no_fields(5_000);

    let error = annotated::json::from_str_limited_to::<Order>(&body, 0).unwrap_err();

    assert_eq!(error.issues().len(), 1);
}

#[test]
fn validating_a_value_with_a_limit_of_zero_keeps_one_issue() {
    let order = order_of_blank_lines(5_000);

    let issues = order.validate_limited_to(0).unwrap_err();

    assert_eq!(issues.as_slice().len(), 1);
}

#[test]
fn validating_into_an_unlimited_report_keeps_every_issue() {
    let order = order_of_blank_lines(5_000);
    let mut found = Report::unlimited();

    order.validate_into(&Location::root(), &mut found);

    assert_eq!(found.len(), 20_000);
}

#[test]
fn reading_into_an_unlimited_report_keeps_every_issue() {
    let body = order_of_lines_with_no_fields(5_000);
    let mut found = Report::unlimited();

    let _ = <Order as Deserialize>::deserialize_at(
        &mut serde_json::Deserializer::from_str(&body),
        &Location::root(),
        &mut found,
    );

    assert_eq!(found.len(), 20_000);
}

#[test]
fn issues_read_failing_fast_are_never_complete() {
    let body = order_of_lines_with_no_fields(1);

    let issues = issues_of(annotated::json::from_str_fail_fast::<Order>(&body));

    assert!(!issues.is_complete());
}

#[test]
fn issues_validated_failing_fast_are_never_complete() {
    let order = order_of_blank_lines(1);

    let issues = order.validate_fail_fast().unwrap_err();

    assert!(!issues.is_complete());
}

#[test]
fn a_map_of_values_with_issues_keeps_at_most_the_limit() {
    let body = lines_by_product_with_no_fields(100);

    let error = annotated::json::from_str_limited_to::<BTreeMap<String, Line>>(&body, 50).unwrap_err();

    assert_eq!(error.issues().len(), 50);
}

#[test]
fn a_hash_map_of_values_with_issues_keeps_at_most_the_limit() {
    let body = lines_by_product_with_no_fields(100);

    let error = annotated::json::from_str_limited_to::<HashMap<String, Line>>(&body, 50).unwrap_err();

    assert_eq!(error.issues().len(), 50);
}

/// A value with as many issues as the number given for it, which says how many of them its report kept.
#[derive(Debug, PartialEq)]
struct GivenIssues {
    kept: usize,
}

impl<'de> Deserialize<'de> for GivenIssues {
    fn deserialize_at<D: Deserializer<'de>>(
        deserializer: D,
        at: &Location<'_>,
        found: &mut Report,
    ) -> Result<Option<Self>, D::Error> {
        let given: u8 = serde::Deserialize::deserialize(deserializer)?;
        let issues_before = found.len();
        for _ in 0..given {
            found.push(at, Problem::custom("given", "was given"));
        }
        Ok(Some(Self {
            kept: found.len() - issues_before,
        }))
    }
}

fn read_into<'de, T: Deserialize<'de>>(json: &'de str, mut found: Report) -> Option<T> {
    let mut deserializer = serde_json::Deserializer::from_str(json);
    T::deserialize_at(&mut deserializer, &Location::root(), &mut found).unwrap()
}

fn issues_read_into<'de, T: Deserialize<'de>>(json: &'de str, mut found: Report) -> Issues {
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let _ = T::deserialize_at(&mut deserializer, &Location::root(), &mut found);
    found.into_result().unwrap_err()
}

#[test]
fn a_map_keeps_one_issue_of_each_value_read_once_those_it_keeps_reach_the_limit() {
    let body = r#"{ "a": 5, "b": 5, "c": 5, "d": 5 }"#;

    let read = read_into::<BTreeMap<String, GivenIssues>>(body, Report::limited_to(12));

    assert_eq!(
        read,
        Some(BTreeMap::from([
            ("a".to_owned(), GivenIssues { kept: 5 }),
            ("b".to_owned(), GivenIssues { kept: 5 }),
            ("c".to_owned(), GivenIssues { kept: 2 }),
            ("d".to_owned(), GivenIssues { kept: 1 }),
        ]))
    );
}

#[test]
fn a_hash_map_keeps_one_issue_of_each_value_read_once_those_it_keeps_reach_the_limit() {
    let body = r#"{ "a": 5, "b": 5, "c": 5, "d": 5 }"#;

    let read = read_into::<HashMap<String, GivenIssues>>(body, Report::limited_to(12));

    assert_eq!(
        read,
        Some(HashMap::from([
            ("a".to_owned(), GivenIssues { kept: 5 }),
            ("b".to_owned(), GivenIssues { kept: 5 }),
            ("c".to_owned(), GivenIssues { kept: 2 }),
            ("d".to_owned(), GivenIssues { kept: 1 }),
        ]))
    );
}

#[test]
fn the_issues_of_a_value_replaced_in_a_map_no_longer_count_towards_the_limit() {
    let body = r#"{ "a": 5, "a": 5, "a": 0, "b": 5, "c": 5 }"#;

    let read = read_into::<BTreeMap<String, GivenIssues>>(body, Report::limited_to(10));

    assert_eq!(
        read,
        Some(BTreeMap::from([
            ("a".to_owned(), GivenIssues { kept: 0 }),
            ("b".to_owned(), GivenIssues { kept: 5 }),
            ("c".to_owned(), GivenIssues { kept: 5 }),
        ]))
    );
}

#[test]
fn the_issues_of_a_value_replaced_in_a_hash_map_no_longer_count_towards_the_limit() {
    let body = r#"{ "a": 5, "a": 5, "a": 0, "b": 5, "c": 5 }"#;

    let read = read_into::<HashMap<String, GivenIssues>>(body, Report::limited_to(10));

    assert_eq!(
        read,
        Some(HashMap::from([
            ("a".to_owned(), GivenIssues { kept: 0 }),
            ("b".to_owned(), GivenIssues { kept: 5 }),
            ("c".to_owned(), GivenIssues { kept: 5 }),
        ]))
    );
}

#[derive(Deserialize, Debug)]
struct Route {
    #[validate(each(custom(open_to_traffic), nested))]
    #[allow(dead_code)]
    stops: Vec<Stop>,
}

#[derive(Deserialize, Debug)]
struct Stop {
    #[validate(required)]
    #[allow(dead_code)]
    street: String,
    #[validate(required)]
    #[allow(dead_code)]
    city: String,
    is_closed: bool,
}

fn open_to_traffic(stop: &Stop) -> Result<(), Problem> {
    if stop.is_closed {
        return Err(Problem::custom("closed", "is closed to traffic"));
    }
    Ok(())
}

#[test]
fn a_list_whose_elements_are_checked_before_nested_keeps_no_issue_after_one_that_may_be_missing() {
    let body = r#"{ "stops": [
        { "street": "", "city": "", "is_closed": true },
        { "street": "", "city": "", "is_closed": false }
    ] }"#;

    let issues = issues_of(annotated::json::from_str_limited_to::<Route>(body, 2));

    assert_eq!(
        (paths_and_codes(&issues), issues.is_complete()),
        (vec!["stops[0]: closed".to_owned()], false)
    );
}

#[derive(Deserialize, Debug)]
struct Trip {
    #[validate(nested)]
    #[allow(dead_code)]
    route: Route,
    #[validate(required)]
    #[allow(dead_code)]
    driver: String,
}

#[test]
fn a_value_holding_a_list_that_may_miss_issues_keeps_no_issue_after_them() {
    let body = r#"{
        "route": { "stops": [
            { "street": "", "city": "", "is_closed": true },
            { "street": "", "city": "", "is_closed": false }
        ] },
        "driver": ""
    }"#;

    let issues = issues_of(annotated::json::from_str_limited_to::<Trip>(body, 2));

    assert_eq!(
        (paths_and_codes(&issues), issues.is_complete()),
        (vec!["route.stops[0]: closed".to_owned()], false)
    );
}

#[test]
fn a_map_fills_its_report_up_to_the_limit_after_a_value_it_left_issues_of_out() {
    let body = r#"{ "b": 5, "a": 5 }"#;

    let issues = issues_read_into::<BTreeMap<String, GivenIssues>>(body, Report::limited_to(6));

    assert_eq!(issues.as_slice().len(), 6);
}

#[derive(Default)]
struct KeyAsItsOwnHash(u64);

impl Hasher for KeyAsItsOwnHash {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, _: &[u8]) {
        unreachable!("only u64 keys are hashed")
    }

    fn write_u64(&mut self, key: u64) {
        self.0 = key;
    }
}

type ByNumber<V> = HashMap<u64, V, BuildHasherDefault<KeyAsItsOwnHash>>;

#[test]
fn a_hash_map_fills_its_report_up_to_the_limit_after_a_value_it_left_issues_of_out() {
    let body = r#"{ "2": 5, "1": 5 }"#;

    let issues = issues_read_into::<ByNumber<GivenIssues>>(body, Report::limited_to(6));

    assert_eq!(issues.as_slice().len(), 6);
}

#[test]
fn a_map_that_left_out_issues_of_a_value_is_not_complete_even_below_its_limit() {
    let body = r#"{ "a": 10, "b": 5, "a": 0 }"#;

    let issues = issues_read_into::<BTreeMap<String, GivenIssues>>(body, Report::limited_to(10));

    assert_eq!(
        (paths_and_codes(&issues), issues.is_complete()),
        (vec!["[b]: given".to_owned()], false)
    );
}

#[test]
fn a_hash_map_that_left_out_issues_of_a_value_is_not_complete_even_below_its_limit() {
    let body = r#"{ "a": 10, "b": 5, "a": 0 }"#;

    let issues = issues_read_into::<HashMap<String, GivenIssues>>(body, Report::limited_to(10));

    assert_eq!(
        (paths_and_codes(&issues), issues.is_complete()),
        (vec!["[b]: given".to_owned()], false)
    );
}

#[test]
fn a_map_read_once_its_report_is_finished_keeps_no_issue_of_its_values() {
    let body = r#"[{ "a": 5 }, { "b": 5 }]"#;

    let read = read_into::<Vec<BTreeMap<String, GivenIssues>>>(body, Report::failing_fast());

    assert_eq!(
        read,
        Some(vec![
            BTreeMap::from([("a".to_owned(), GivenIssues { kept: 1 })]),
            BTreeMap::from([("b".to_owned(), GivenIssues { kept: 0 })]),
        ])
    );
}

#[test]
fn a_hash_map_read_once_its_report_is_finished_keeps_no_issue_of_its_values() {
    let body = r#"[{ "a": 5 }, { "b": 5 }]"#;

    let read = read_into::<Vec<HashMap<String, GivenIssues>>>(body, Report::failing_fast());

    assert_eq!(
        read,
        Some(vec![
            HashMap::from([("a".to_owned(), GivenIssues { kept: 1 })]),
            HashMap::from([("b".to_owned(), GivenIssues { kept: 0 })]),
        ])
    );
}
