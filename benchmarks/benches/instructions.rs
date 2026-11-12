//! Counts the instructions one validation runs, and what it allocates, in each of annotated's cases, under
//! Valgrind. Unlike time, the counts are the same on any machine, so two commits measured with the same
//! toolchain can be compared case by case: `compare.sh` does that.
//!
//! Each case has a benchmark id of its own, rather than one per position in a list, so that a case added to
//! a scenario is never compared with the one that was in its place.

use annotated_benchmarks::{Case, annotated_cases};
use gungraun::prelude::*;
use gungraun::{Dhat, EntryPoint};

/// The case named `name`, validated once already, so that compiling a lazy regular expression isn't counted.
fn warmed_up(name: &str) -> Case {
    let case = annotated_cases()
        .find(|case| case.to_string() == name)
        .expect("the name is one of annotated's cases");
    case.validate_once();
    case
}

// Returns the case so that dropping it isn't counted.
#[library_benchmark(setup = warmed_up)]
#[bench::flat_valid("flat_valid/annotated")]
#[bench::flat_valid_report_all("flat_valid/annotated (report_all)")]
#[bench::flat_every_field_invalid("flat_every_field_invalid/annotated")]
#[bench::flat_every_field_invalid_report_all("flat_every_field_invalid/annotated (report_all)")]
#[bench::flat_one_field_invalid("flat_one_field_invalid/annotated")]
#[bench::flat_one_field_invalid_report_all("flat_one_field_invalid/annotated (report_all)")]
#[bench::nested_valid("nested_valid/annotated")]
#[bench::nested_5pct_invalid("nested_5pct_invalid/annotated")]
#[bench::map_nested_valid("map_nested_valid/annotated")]
#[bench::map_nested_5pct_invalid("map_nested_5pct_invalid/annotated")]
#[bench::map_string_values_valid("map_string_values_valid/annotated")]
#[bench::long_string_1kb_chars("long_string_1kb/annotated (chars)")]
#[bench::long_string_1kb_bytes("long_string_1kb/annotated (bytes)")]
#[bench::long_string_1kb_graphemes("long_string_1kb/annotated (graphemes)")]
#[bench::long_string_64kb_chars("long_string_64kb/annotated (chars)")]
#[bench::long_string_64kb_bytes("long_string_64kb/annotated (bytes)")]
#[bench::long_string_64kb_graphemes("long_string_64kb/annotated (graphemes)")]
#[bench::end_to_end_valid_from_str("end_to_end_valid/annotated (json::from_str)")]
#[bench::end_to_end_valid_then_validate("end_to_end_valid/annotated (serde_json, then validate)")]
#[bench::end_to_end_valid_from_str_report_all("end_to_end_valid/annotated (json::from_str, report_all)")]
#[bench::end_to_end_valid_then_validate_report_all(
    "end_to_end_valid/annotated (serde_json, then validate, report_all)"
)]
#[bench::end_to_end_valid_from_str_collect_type_errors(
    "end_to_end_valid/annotated (json::from_str, collect_type_errors)"
)]
#[bench::end_to_end_every_field_invalid_from_str("end_to_end_every_field_invalid/annotated (json::from_str)")]
#[bench::end_to_end_every_field_invalid_then_validate(
    "end_to_end_every_field_invalid/annotated (serde_json, then validate)"
)]
#[bench::end_to_end_every_field_invalid_from_str_report_all(
    "end_to_end_every_field_invalid/annotated (json::from_str, report_all)"
)]
#[bench::end_to_end_every_field_invalid_then_validate_report_all(
    "end_to_end_every_field_invalid/annotated (serde_json, then validate, report_all)"
)]
#[bench::end_to_end_every_field_invalid_from_str_fail_fast(
    "end_to_end_every_field_invalid/annotated (json::from_str_fail_fast)"
)]
#[bench::end_to_end_every_field_invalid_then_validate_fail_fast(
    "end_to_end_every_field_invalid/annotated (serde_json, then validate_fail_fast)"
)]
#[bench::end_to_end_one_field_invalid_from_str("end_to_end_one_field_invalid/annotated (json::from_str)")]
#[bench::end_to_end_one_field_invalid_then_validate(
    "end_to_end_one_field_invalid/annotated (serde_json, then validate)"
)]
#[bench::end_to_end_one_field_invalid_from_str_report_all(
    "end_to_end_one_field_invalid/annotated (json::from_str, report_all)"
)]
#[bench::end_to_end_one_field_invalid_then_validate_report_all(
    "end_to_end_one_field_invalid/annotated (serde_json, then validate, report_all)"
)]
#[bench::end_to_end_one_field_invalid_from_str_fail_fast(
    "end_to_end_one_field_invalid/annotated (json::from_str_fail_fast)"
)]
#[bench::end_to_end_one_field_invalid_then_validate_fail_fast(
    "end_to_end_one_field_invalid/annotated (serde_json, then validate_fail_fast)"
)]
#[bench::end_to_end_nested_valid_from_str("end_to_end_nested_valid/annotated (json::from_str)")]
#[bench::end_to_end_nested_valid_then_validate("end_to_end_nested_valid/annotated (serde_json, then validate)")]
#[bench::end_to_end_nested_5pct_invalid_from_str("end_to_end_nested_5pct_invalid/annotated (json::from_str)")]
#[bench::end_to_end_nested_5pct_invalid_then_validate(
    "end_to_end_nested_5pct_invalid/annotated (serde_json, then validate)"
)]
#[bench::end_to_end_map_nested_valid_from_str("end_to_end_map_nested_valid/annotated (json::from_str)")]
#[bench::end_to_end_map_nested_valid_then_validate("end_to_end_map_nested_valid/annotated (serde_json, then validate)")]
#[bench::end_to_end_map_nested_5pct_invalid_from_str("end_to_end_map_nested_5pct_invalid/annotated (json::from_str)")]
#[bench::end_to_end_map_nested_5pct_invalid_then_validate(
    "end_to_end_map_nested_5pct_invalid/annotated (serde_json, then validate)"
)]
#[bench::end_to_end_missing_field_from_str("end_to_end_missing_field/annotated (json::from_str)")]
#[bench::end_to_end_missing_field_then_validate("end_to_end_missing_field/annotated (serde_json, then validate)")]
fn validate(case: Case) -> Case {
    case.validate_once();
    case
}

library_benchmark_group!(name = annotated, benchmarks = validate);

main!(
    config = LibraryBenchmarkConfig::default().tool(Dhat::default().entry_point(EntryPoint::Default)),
    library_benchmark_groups = annotated
);
