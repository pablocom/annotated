//! Times every case: one criterion group per scenario, one benchmark per library in it.

use std::time::Duration;

use annotated_benchmarks::all_cases;
use criterion::{Criterion, Throughput, criterion_group, criterion_main};

fn validation(criterion: &mut Criterion) {
    let cases = all_cases();
    for cases_of_scenario in cases.chunk_by(|case, next| case.scenario == next.scenario) {
        let mut group = criterion.benchmark_group(cases_of_scenario[0].scenario);
        for case in cases_of_scenario {
            if let Some(bytes) = case.throughput_bytes {
                group.throughput(Throughput::Bytes(bytes));
            }
            group.bench_function(case.label(), |bencher| bencher.iter(|| case.validate_once()));
        }
        group.finish();
    }
}

fn timings() -> Criterion {
    Criterion::default()
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2))
}

criterion_group! {
    name = benches;
    config = timings();
    targets = validation
}
criterion_main!(benches);
