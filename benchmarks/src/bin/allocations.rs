//! Counts what every case allocates per validation, through a global allocator that counts, in a binary of
//! its own so the counting never slows the timed benchmarks.
//!
//! `cargo run --release --bin allocations [-- <filter>]` prints a table and writes
//! `target/allocations.json`, which the `report` binary reads. A filter keeps the cases whose
//! `scenario/label` contains it.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};

use annotated_benchmarks::{Case, all_cases};
use serde::Serialize;

const WARM_UP_VALIDATIONS: u64 = 100;
const COUNTED_VALIDATIONS: u64 = 10_000;

static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static BYTES_ALLOCATED: AtomicU64 = AtomicU64::new(0);

/// The system allocator, counting every allocation and reallocation and the bytes each asks for: a
/// reallocation counts as one allocation of its whole new size.
struct CountingAllocator;

fn count_allocation_of(bytes: usize) {
    ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
    BYTES_ALLOCATED.fetch_add(bytes as u64, Ordering::Relaxed);
}

#[allow(
    unsafe_code,
    reason = "a global allocator is an unsafe trait; this one forwards to the system's"
)]
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count_allocation_of(layout.size());
        // SAFETY: the caller upholds `GlobalAlloc::alloc`'s contract, which `System` shares.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count_allocation_of(layout.size());
        // SAFETY: as for `alloc`.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count_allocation_of(new_size);
        // SAFETY: `pointer` was allocated by this allocator, which is `System`, with `layout`.
        unsafe { System.realloc(pointer, layout, new_size) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: `pointer` was allocated by this allocator, which is `System`, with `layout`.
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[global_allocator]
static GLOBAL_ALLOCATOR: CountingAllocator = CountingAllocator;

#[derive(Serialize)]
struct AllocationsOfCase {
    scenario: &'static str,
    label: String,
    allocations_per_validation: f64,
    bytes_per_validation: f64,
}

fn allocations_of(case: &Case) -> AllocationsOfCase {
    for _ in 0..WARM_UP_VALIDATIONS {
        case.validate_once();
    }
    let allocations_before = ALLOCATIONS.load(Ordering::Relaxed);
    let bytes_before = BYTES_ALLOCATED.load(Ordering::Relaxed);
    for _ in 0..COUNTED_VALIDATIONS {
        case.validate_once();
    }
    let allocations = ALLOCATIONS.load(Ordering::Relaxed) - allocations_before;
    let bytes = BYTES_ALLOCATED.load(Ordering::Relaxed) - bytes_before;
    AllocationsOfCase {
        scenario: case.scenario,
        label: case.label(),
        allocations_per_validation: per_validation(allocations),
        bytes_per_validation: per_validation(bytes),
    }
}

#[expect(clippy::cast_precision_loss, reason = "counts stay far below 2^52")]
fn per_validation(total: u64) -> f64 {
    total as f64 / COUNTED_VALIDATIONS as f64
}

fn main() {
    let filter = std::env::args().nth(1).unwrap_or_default();
    let cases = all_cases();
    let counted: Vec<AllocationsOfCase> = cases
        .iter()
        .filter(|case| case.to_string().contains(&filter))
        .map(allocations_of)
        .collect();

    println!("| Scenario | Library | Allocations per validation | Bytes per validation |");
    println!("|---|---|--:|--:|");
    for case in &counted {
        println!(
            "| {} | {} | {:.2} | {:.1} |",
            case.scenario, case.label, case.allocations_per_validation, case.bytes_per_validation
        );
    }

    let output = concat!(env!("CARGO_MANIFEST_DIR"), "/target/allocations.json");
    let json = serde_json::to_string_pretty(&counted).expect("the counts serialize to JSON");
    std::fs::write(output, json).expect("target/ is writable");
    eprintln!("wrote {output}");
}
