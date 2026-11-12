//! Every case finds the violations its scenario is built to have, so a benchmark never times a library
//! rejecting the input it should accept, or doing less work than the others.

use annotated_benchmarks::all_cases;

fn violations_found_in(scenario: &str) -> Vec<(String, usize)> {
    all_cases()
        .into_iter()
        .filter(|case| case.scenario == scenario)
        .map(|case| (case.label(), case.violations()))
        .collect()
}

fn expected(violations: &[(&str, usize)]) -> Vec<(String, usize)> {
    violations
        .iter()
        .map(|(label, count)| ((*label).to_owned(), *count))
        .collect()
}

#[test]
fn a_valid_flat_body_has_no_violations_in_any_library() {
    assert_eq!(
        violations_found_in("flat_valid"),
        expected(&[
            ("annotated", 0),
            ("annotated (report_all)", 0),
            ("validator", 0),
            ("garde", 0),
            ("wary", 0),
        ])
    );
}

#[test]
fn a_flat_body_with_every_field_invalid_has_nine_violations_but_one_per_field_in_annotated_by_default() {
    assert_eq!(
        violations_found_in("flat_every_field_invalid"),
        expected(&[
            ("annotated", 8),
            ("annotated (report_all)", 9),
            ("validator", 9),
            ("garde", 9),
            ("wary", 9),
        ])
    );
}

#[test]
fn a_flat_body_with_one_field_invalid_has_one_violation_in_every_library() {
    assert_eq!(
        violations_found_in("flat_one_field_invalid"),
        expected(&[
            ("annotated", 1),
            ("annotated (report_all)", 1),
            ("validator", 1),
            ("garde", 1),
            ("wary", 1),
        ])
    );
}

#[test]
fn a_valid_order_has_no_violations_in_any_library() {
    assert_eq!(
        violations_found_in("nested_valid"),
        expected(&[
            ("annotated", 0),
            ("validator", 0),
            ("garde", 0),
            ("wary", 0),
            ("serde_valid (graphemes)", 0),
        ])
    );
}

#[test]
fn an_order_with_five_invalid_lines_has_five_violations_in_every_library() {
    assert_eq!(
        violations_found_in("nested_5pct_invalid"),
        expected(&[
            ("annotated", 5),
            ("validator", 5),
            ("garde", 5),
            ("wary", 5),
            ("serde_valid (graphemes)", 5),
        ])
    );
}

#[test]
fn a_valid_stock_map_has_no_violations_in_any_library_that_goes_through_maps() {
    assert_eq!(
        violations_found_in("map_nested_valid"),
        expected(&[
            ("annotated", 0),
            ("validator", 0),
            ("garde", 0),
            ("serde_valid (graphemes)", 0),
        ])
    );
}

#[test]
fn a_stock_map_with_five_invalid_entries_has_five_violations_in_every_library_that_goes_through_maps() {
    assert_eq!(
        violations_found_in("map_nested_5pct_invalid"),
        expected(&[
            ("annotated", 5),
            ("validator", 5),
            ("garde", 5),
            ("serde_valid (graphemes)", 5),
        ])
    );
}

#[test]
fn valid_label_values_have_no_violations() {
    assert_eq!(
        violations_found_in("map_string_values_valid"),
        expected(&[("annotated", 0)])
    );
}

#[test]
fn a_1_kb_text_within_its_length_has_no_violations_however_it_is_counted() {
    assert_eq!(
        violations_found_in("long_string_1kb"),
        expected(&[
            ("annotated (chars)", 0),
            ("annotated (bytes)", 0),
            ("annotated (graphemes)", 0),
            ("validator (chars)", 0),
            ("garde (chars)", 0),
            ("garde (bytes)", 0),
            ("garde (graphemes)", 0),
            ("wary (chars)", 0),
            ("wary (bytes)", 0),
            ("wary (graphemes)", 0),
            ("serde_valid (graphemes)", 0),
        ])
    );
}

#[test]
fn a_64_kb_text_within_its_length_has_no_violations_however_it_is_counted() {
    assert_eq!(
        violations_found_in("long_string_64kb"),
        expected(&[
            ("annotated (chars)", 0),
            ("annotated (bytes)", 0),
            ("annotated (graphemes)", 0),
            ("validator (chars)", 0),
            ("garde (chars)", 0),
            ("garde (bytes)", 0),
            ("garde (graphemes)", 0),
            ("wary (chars)", 0),
            ("wary (bytes)", 0),
            ("wary (graphemes)", 0),
            ("serde_valid (graphemes)", 0),
        ])
    );
}

#[test]
fn a_valid_body_received_has_no_violations_in_any_library_however_annotated_reads_it() {
    assert_eq!(
        violations_found_in("end_to_end_valid"),
        expected(&[
            ("serde_json alone", 0),
            ("annotated (json::from_str)", 0),
            ("annotated (serde_json, then validate)", 0),
            ("annotated (json::from_str, report_all)", 0),
            ("annotated (serde_json, then validate, report_all)", 0),
            ("annotated (json::from_str, collect_type_errors)", 0),
            ("validator", 0),
            ("garde", 0),
            ("wary", 0),
        ])
    );
}

#[test]
fn a_body_received_with_every_field_invalid_has_the_violations_of_its_flat_scenario_but_one_when_failing_fast() {
    assert_eq!(
        violations_found_in("end_to_end_every_field_invalid"),
        expected(&[
            ("serde_json alone", 0),
            ("annotated (json::from_str)", 8),
            ("annotated (serde_json, then validate)", 8),
            ("annotated (json::from_str, report_all)", 9),
            ("annotated (serde_json, then validate, report_all)", 9),
            ("annotated (json::from_str_fail_fast)", 1),
            ("annotated (serde_json, then validate_fail_fast)", 1),
            ("validator", 9),
            ("garde", 9),
            ("wary", 9),
        ])
    );
}

#[test]
fn a_body_received_with_one_field_invalid_has_one_violation_in_every_library() {
    assert_eq!(
        violations_found_in("end_to_end_one_field_invalid"),
        expected(&[
            ("serde_json alone", 0),
            ("annotated (json::from_str)", 1),
            ("annotated (serde_json, then validate)", 1),
            ("annotated (json::from_str, report_all)", 1),
            ("annotated (serde_json, then validate, report_all)", 1),
            ("annotated (json::from_str_fail_fast)", 1),
            ("annotated (serde_json, then validate_fail_fast)", 1),
            ("validator", 1),
            ("garde", 1),
            ("wary", 1),
        ])
    );
}

#[test]
fn a_valid_order_received_has_no_violations_in_any_library() {
    assert_eq!(
        violations_found_in("end_to_end_nested_valid"),
        expected(&[
            ("serde_json alone", 0),
            ("annotated (json::from_str)", 0),
            ("annotated (serde_json, then validate)", 0),
            ("validator", 0),
            ("garde", 0),
            ("wary", 0),
            ("serde_valid (graphemes)", 0),
        ])
    );
}

#[test]
fn an_order_received_with_five_invalid_lines_has_five_violations_in_every_library() {
    assert_eq!(
        violations_found_in("end_to_end_nested_5pct_invalid"),
        expected(&[
            ("serde_json alone", 0),
            ("annotated (json::from_str)", 5),
            ("annotated (serde_json, then validate)", 5),
            ("validator", 5),
            ("garde", 5),
            ("wary", 5),
            ("serde_valid (graphemes)", 5),
        ])
    );
}

#[test]
fn a_valid_stock_map_received_has_no_violations_in_any_library_that_goes_through_maps() {
    assert_eq!(
        violations_found_in("end_to_end_map_nested_valid"),
        expected(&[
            ("serde_json alone", 0),
            ("annotated (json::from_str)", 0),
            ("annotated (serde_json, then validate)", 0),
            ("validator", 0),
            ("garde", 0),
            ("serde_valid (graphemes)", 0),
        ])
    );
}

#[test]
fn a_stock_map_received_with_five_invalid_entries_has_five_violations_in_every_library_that_goes_through_maps() {
    assert_eq!(
        violations_found_in("end_to_end_map_nested_5pct_invalid"),
        expected(&[
            ("serde_json alone", 0),
            ("annotated (json::from_str)", 5),
            ("annotated (serde_json, then validate)", 5),
            ("validator", 5),
            ("garde", 5),
            ("serde_valid (graphemes)", 5),
        ])
    );
}

#[test]
fn a_body_received_without_a_field_reports_it_beside_the_broken_ones_in_annotated_but_alone_through_serde() {
    assert_eq!(
        violations_found_in("end_to_end_missing_field"),
        expected(&[
            ("serde_json alone", 1),
            ("annotated (json::from_str)", 3),
            ("annotated (serde_json, then validate)", 1),
            ("validator", 1),
            ("garde", 1),
            ("wary", 1),
        ])
    );
}

#[test]
fn checking_inline_finds_the_same_violations_as_the_derive_on_an_order() {
    assert_eq!(
        violations_found_in("experiment_inline_checks_order"),
        expected(&[
            ("annotated (derived)", 0),
            ("hand-written (inline checks, violations recorded out of line)", 0),
        ])
    );
}

#[test]
fn checking_inline_finds_the_same_violations_as_the_derive_on_a_valid_flat_body() {
    assert_eq!(
        violations_found_in("experiment_inline_checks_flat"),
        expected(&[
            ("annotated (derived)", 0),
            ("hand-written (inline checks, violations recorded out of line)", 0),
        ])
    );
}

#[test]
fn checking_inline_finds_the_same_violations_as_the_derive_on_a_flat_body_with_every_field_invalid() {
    assert_eq!(
        violations_found_in("experiment_inline_checks_flat_every_field_invalid"),
        expected(&[
            ("annotated (derived)", 8),
            ("hand-written (inline checks, violations recorded out of line)", 8),
        ])
    );
}

#[test]
fn a_website_is_a_url_to_both_annotated_and_the_url_crate() {
    assert_eq!(
        violations_found_in("experiment_url"),
        expected(&[
            ("annotated (rules::url())", 0),
            ("hand-written (url::Url::parse alone, as garde and validator check)", 0),
        ])
    );
}

#[test]
fn settling_a_length_by_bytes_first_accepts_what_counting_chars_accepts() {
    assert_eq!(
        [
            violations_found_in("experiment_length_short_text"),
            violations_found_in("experiment_length_64kb")
        ]
        .concat(),
        expected(&[
            ("annotated (rules::length(), chars counted)", 0),
            ("hand-written (settled by the byte length first)", 0),
            ("annotated (rules::length(), chars counted)", 0),
            ("hand-written (settled by the byte length first)", 0),
        ])
    );
}
