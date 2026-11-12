//! The instructions benchmark counts every one of annotated's cases, so a case added to a scenario can't go
//! uncounted.

use annotated_benchmarks::annotated_cases;

#[test]
fn every_case_of_annotated_has_its_instructions_counted() {
    let benchmark = include_str!("../benches/instructions.rs");

    let uncounted: Vec<String> = annotated_cases()
        .map(|case| case.to_string())
        .filter(|name| !benchmark.contains(&format!("\"{name}\"")))
        .collect();

    assert_eq!(uncounted, Vec::<String>::new());
}
