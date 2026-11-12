# annotated benchmarks

How fast [annotated](https://github.com/pablocom/annotated) validates, and how much it allocates, against the other Rust validation crates with a derive. The aim is for annotated to be the fastest; this project is how we show it, and how we find where it isn't yet.

The results, the environment they were measured on, and what annotated could change are in [RESULTS.md](RESULTS.md). [results.html](results.html) shows the same run as a page to open in a browser, with charts and a switch between the two build profiles. It's a snapshot of that run, its data embedded.

## What is measured

Every **case** is one library validating one prepared input of one **scenario**. For each case:

- **Time:** criterion's median time of one validation, with its 95% confidence interval. The input is built before timing; only `validate` and dropping its outcome are timed, behind `std::hint::black_box`.
- **Allocations and bytes:** how many allocations one validation makes and how many bytes it asks for, counted by a global allocator in a binary of its own (`allocations`), so counting never slows the timed runs. A reallocation counts as one allocation of its whole new size. Each case is warmed up first, so lazily compiled regular expressions and their caches aren't charged to the validations counted.
- **Violations:** how many violations the validation finds. `tests/cases.rs` pins them for every case, so no library is timed rejecting an input it should accept, or doing less work than the others without it showing.

| Scenario | What it is |
|---|---|
| `flat_valid` | A sign-up body of 8 fields mixing `String` and `Option`, with length in chars, integer and float ranges, email, URL and two patterns. All valid: the hot path of an API. |
| `flat_every_field_invalid` | The same body with every field broken, and one field breaking two rules: building the whole report. |
| `flat_one_field_invalid` | The same body with only the email broken, the most common failure. |
| `nested_valid`, `nested_5pct_invalid` | An order with a nested customer and a `Vec` of 100 nested lines; then with 5 of them invalid. |
| `map_nested_valid`, `map_nested_5pct_invalid` | A `BTreeMap` of 100 nested stock entries; then with 5 invalid. |
| `map_string_values_valid` | A `BTreeMap` of 100 strings, each value checked with `each(length(...))`. |
| `long_string_1kb`, `long_string_64kb` | `length` on 1 KB and 64 KB of Spanish prose, counted in chars, bytes or graphemes. |
| `end_to_end_valid`, `end_to_end_every_field_invalid`, `end_to_end_one_field_invalid` | The three flat bodies from JSON text: `serde_json::from_str` then validation, beside annotated's collecting deserialization, with `serde_json` alone as the floor. annotated also fails fast on the invalid bodies, and opts in to collecting type errors on the valid one. |
| `end_to_end_nested_*`, `end_to_end_map_nested_*` | The order and the warehouse from JSON text, valid and with 5 invalid elements, read the same ways. |
| `end_to_end_missing_field` | A sign-up body without one field and with two broken: annotated's collecting deserialization reports all three, while serde stops at the missing field, so the others answer its error alone. The cases do different amounts of work. |
| `experiment_*` | Not libraries: annotated beside code written by hand through its public API, the way a change to annotated would make it run, to measure what that change would gain. |

Each scenario module (`src/scenarios/`) starts with a table of its rules and holds every library's version of its types side by side, so you can check at a glance that they say the same thing.

## Libraries

| Library | Version | Why |
|---|---|---|
| [annotated](https://github.com/pablocom/annotated) | git `f881d35` | The library under test, features `regex`, `unicode` and `json`. |
| [validator](https://crates.io/crates/validator) | 0.21.0 | The most used validation crate. |
| [garde](https://crates.io/crates/garde) | 0.23.0 | The most used of the newer ones. |
| [serde_valid](https://crates.io/crates/serde_valid) | 3.2.0 | Widely used, JSON-Schema-style rules. It has no email or URL rule and counts string lengths only in graphemes, so it is in the nested, map and long-string scenarios only, and reads the order and the warehouse end to end. |
| [wary](https://crates.io/crates/wary) | 0.3.2 | Little used, but maintained and the one competitor built to allocate nothing, so the one most likely to beat annotated on allocations. It can't go through maps. |

Left out: [validify](https://crates.io/crates/validify) (no release since February 2025), [validators](https://crates.io/crates/validators) and [nutype](https://crates.io/crates/nutype) (they validate a newtype as it is built, not a struct's fields), and [valitron](https://crates.io/crates/valitron), [validatron](https://crates.io/crates/validatron) and [validy](https://crates.io/crates/validy) (no release in the last year, or almost no users).

Every library shares one build of `regex`, `url` and `unicode-segmentation`, with the same features, so none gets a faster engine than another. annotated recognizes URLs itself and doesn't use `url`.

## How to run

Use the current stable toolchain; the commands say `+1.99.1`.

```sh
# Every case finds the violations its scenario expects
cargo +1.99.1 test --release

# Time every case, pinned to one performance core (see "Measuring well")
taskset -c 4 cargo +1.99.1 bench --bench validation

# ...or only some, and quickly while you work on them
taskset -c 4 cargo +1.99.1 bench --bench validation -- --quick flat_

# Count allocations and bytes per validation; writes target/allocations.json
cargo +1.99.1 run --release --bin allocations
cargo +1.99.1 run --release --bin allocations -- nested_   # only cases whose scenario/label contains it

# Join both into one Markdown table per scenario
cargo +1.99.1 run --release --bin report > tables.md
```

The `release` and `bench` profiles build with `lto = "fat"` and `codegen-units = 1`, so every library gets the same cross-crate inlining and the same code each run. To see what a service built with cargo's release defaults gets (no LTO, 16 codegen units):

```sh
CRITERION_HOME=target/criterion-without-lto taskset -c 4 cargo +1.99.1 bench --profile release-without-lto --bench validation
CRITERION_HOME=target/criterion-without-lto cargo +1.99.1 run --release --bin report
```

### Measuring well

- Build first (`cargo bench --no-run`), then run with nothing else compiling or busy on the machine.
- Pin the run to one performance core with `taskset`. On a hybrid Intel CPU the efficiency cores are much slower: `cat /sys/devices/cpu_core/cpus` lists the performance ones.
- The CPU governor and turbo boost move the clock as the chip heats. Compare numbers from the same run; across runs, look at the ratios more than the nanoseconds.
- Each case warms up for 0.5 s and is measured for 2 s, set in `benches/validation.rs`; the medians stay within a run's own spread of criterion's 3 s and 5 s. A run to publish can still ask for those: `-- --warm-up-time 3 --measurement-time 5`.
- `--quick` is for development only: it takes few samples.

## Measuring a local annotated checkout

annotated is a git dependency pinned to a revision of the public repository, so every result names what it measured. To measure your working tree instead, while you optimise it, add this to `Cargo.toml`:

```toml
[patch."https://github.com/pablocom/annotated"]
annotated = { path = "../annotated/crates/annotated" }
```

`annotated-derive` comes along, as annotated's own path dependency. Tables still label annotated with the pinned revision, so note in your results that they measured a patched tree. Remove the patch, and move `rev` and `Library::version` in `src/lib.rs` to the new revision once it is pushed.

## Adding a library

1. Add it to `[dependencies]` at its latest version (`cargo search`, `cargo info`), with the features the rules need.
2. Add a `Library` variant in `src/lib.rs`, with its name and version.
3. In each scenario that it can express, add a `with_<library>` module beside the others with the same rules in its syntax, and a case in that scenario's `cases`. Where its meaning differs, say so in the module's doc. Where it can't express a scenario, leave it out and say why.
4. Add a function counting its violations to `scenarios::count`.
5. Add its expected violations to `tests/cases.rs`, and run the tests.

## Adding a scenario

1. Add a module to `src/scenarios/`: a doc table of its rules, a function building its JSON body, a `with_<library>` module per library, and `cases()`. Bodies are JSON so every library's input comes from the same text, and so the end-to-end scenario can reuse them.
2. List it in `scenarios::all`.
3. Pin its violations per library in `tests/cases.rs`.

## Collecting deserialization

annotated also reads a body and validates it in one pass, with `annotated::json::from_str`, reporting missing fields beside broken rules. Its types derive `annotated::Deserialize` in place of serde's derive, and that derive implements serde's `Deserialize` by validating, so they can't be the types a scenario parses before validating. Each scenario module keeps them apart, in `with_annotated_deserialize`, beside `with_annotated`. The end-to-end scenarios read each body both ways, labelled `json::from_str` and `serde_json, then validate`, and `count::annotated_deserialized` counts what the first finds.
