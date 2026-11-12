//! Turns the last benchmark run into Markdown tables, one per scenario: criterion's median time from
//! `target/criterion`, and the counts the `allocations` binary wrote to `target/allocations.json`.
//!
//! `cargo run --release --bin report > tables.md`
//!
//! It reads criterion's results from `CRITERION_HOME` when set, as criterion writes them.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use annotated_benchmarks::{Case, Library, all_cases};
use serde::Deserialize;

const TARGET: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/target");

#[derive(Deserialize)]
struct BenchmarkName {
    full_id: String,
}

#[derive(Deserialize)]
struct Estimates {
    median: Estimate,
}

#[derive(Deserialize)]
struct Estimate {
    point_estimate: f64,
    confidence_interval: ConfidenceInterval,
}

#[derive(Deserialize)]
struct ConfidenceInterval {
    lower_bound: f64,
    upper_bound: f64,
}

#[derive(Deserialize)]
struct AllocationsOfCase {
    scenario: String,
    label: String,
    allocations_per_validation: f64,
    bytes_per_validation: f64,
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Option<T> {
    let json = fs::read_to_string(path).ok()?;
    serde_json::from_str(&json).ok()
}

/// Criterion's median of every benchmark it has run, by its full id: `scenario/label`.
fn medians() -> HashMap<String, Estimate> {
    let mut medians = HashMap::new();
    let criterion_home =
        std::env::var_os("CRITERION_HOME").map_or_else(|| Path::new(TARGET).join("criterion"), Into::into);
    let Ok(groups) = fs::read_dir(criterion_home) else {
        return medians;
    };
    for benchmark in groups
        .flatten()
        .filter_map(|group| fs::read_dir(group.path()).ok())
        .flatten()
        .flatten()
    {
        let latest = benchmark.path().join("new");
        let name = read_json::<BenchmarkName>(&latest.join("benchmark.json"));
        let estimates = read_json::<Estimates>(&latest.join("estimates.json"));
        if let (Some(name), Some(estimates)) = (name, estimates) {
            medians.insert(name.full_id, estimates.median);
        }
    }
    medians
}

fn allocations() -> HashMap<String, AllocationsOfCase> {
    let counted: Vec<AllocationsOfCase> = read_json(&Path::new(TARGET).join("allocations.json")).unwrap_or_default();
    counted
        .into_iter()
        .map(|case| (format!("{}/{}", case.scenario, case.label), case))
        .collect()
}

fn duration(nanoseconds: f64) -> String {
    if nanoseconds < 1_000.0 {
        format!("{nanoseconds:.1} ns")
    } else if nanoseconds < 1_000_000.0 {
        format!("{:.2} µs", nanoseconds / 1_000.0)
    } else {
        format!("{:.2} ms", nanoseconds / 1_000_000.0)
    }
}

#[expect(clippy::cast_precision_loss, reason = "a text's bytes stay far below 2^52")]
fn throughput(bytes: u64, nanoseconds: f64) -> String {
    let gibibytes_per_second = bytes as f64 / nanoseconds * 1e9 / f64::from(1u32 << 30);
    format!("{gibibytes_per_second:.2} GiB/s")
}

fn is_a_floor(case: &Case) -> bool {
    case.library == Library::SerdeJsonAlone
}

fn print_scenario(
    cases: &[&Case],
    medians: &HashMap<String, Estimate>,
    allocations: &HashMap<String, AllocationsOfCase>,
) {
    let has_throughput = cases.iter().any(|case| case.throughput_bytes.is_some());
    let fastest = cases
        .iter()
        .filter(|case| !is_a_floor(case))
        .filter_map(|case| medians.get(&case.to_string()))
        .map(|median| median.point_estimate)
        .fold(f64::INFINITY, f64::min);

    println!("### `{}`\n", cases[0].scenario);
    let throughput_header = if has_throughput { " Throughput |" } else { "" };
    let throughput_alignment = if has_throughput { "--:|" } else { "" };
    println!(
        "| Library | Version | Median | 95% CI of the median | vs fastest |{throughput_header} Allocations | Bytes | Issues |"
    );
    println!("|---|---|--:|---|--:|{throughput_alignment}--:|--:|--:|");
    for case in cases {
        let id = case.to_string();
        let (median, interval, versus_fastest, case_throughput) = match medians.get(&id) {
            Some(median) => (
                duration(median.point_estimate),
                format!(
                    "{} – {}",
                    duration(median.confidence_interval.lower_bound),
                    duration(median.confidence_interval.upper_bound)
                ),
                if is_a_floor(case) {
                    "floor".to_owned()
                } else {
                    format!("{:.2}×", median.point_estimate / fastest)
                },
                case.throughput_bytes
                    .map_or_default(|bytes| throughput(bytes, median.point_estimate)),
            ),
            None => ("not run".to_owned(), String::new(), String::new(), String::new()),
        };
        let (allocation_count, bytes) = allocations.get(&id).map_or_else(
            || ("not counted".to_owned(), String::new()),
            |counted| {
                (
                    format!("{:.2}", counted.allocations_per_validation),
                    format!("{:.0}", counted.bytes_per_validation),
                )
            },
        );
        let throughput_cell = if has_throughput {
            format!(" {case_throughput} |")
        } else {
            String::new()
        };
        println!(
            "| {} | {} | {median} | {interval} | {versus_fastest} |{throughput_cell} {allocation_count} | {bytes} | {} |",
            case.label(),
            case.library.version(),
            case.issues(),
        );
    }
    println!();
}

fn main() {
    let medians = medians();
    let allocations = allocations();
    let cases = all_cases();
    let case_references: Vec<&Case> = cases.iter().collect();
    let mut is_among_experiments = false;
    for cases_of_scenario in case_references.chunk_by(|case, next| case.scenario == next.scenario) {
        let is_an_experiment = cases_of_scenario[0].scenario.starts_with("experiment_");
        if is_an_experiment && !is_among_experiments {
            println!("## Experiments\n");
            is_among_experiments = true;
        }
        print_scenario(cases_of_scenario, &medians, &allocations);
    }
}
