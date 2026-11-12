# Results

annotated at git `f881d35` against validator 0.21.0, garde 0.23.0, wary 0.3.2 and serde_valid 3.2.0, measured on 2026-11-12. The [README](README.md) says what each scenario is and how to run it again.

This run adds annotated's collecting deserialization, `annotated::json::from_str`, which reads a body and validates it in one pass: on the three flat bodies, beside `serde_json::from_str` then validation; on the order and the warehouse read from their JSON text; and on a body missing a field, where the cases don't do the same work. It measures exactly what the repository pins, with no `[patch]`. [History](#history) sets annotated beside the runs of `b58e76b` and `99972e3`.

## Environment

| | |
|---|---|
| CPU | Intel Core i7-14700K: 8 performance cores with two threads each (CPUs 0–15), 12 efficiency cores (16–27), up to 5.6 GHz |
| Pinned to | CPU 4, a performance core, with `taskset -c 4`. Its sibling thread, CPU 5, was not isolated. |
| Clock | `intel_pstate` active, governor `powersave` (hardware-managed), energy preference `balance_performance`, turbo on, SMT on |
| Kernel | Linux 7.3.3-1-cachyos-bore |
| Toolchain | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` |
| Profile | `release`/`bench`: `opt-level = 3`, `lto = "fat"`, `codegen-units = 1`, `debug = false`, `panic = "unwind"`. Also run with `release-without-lto`: cargo's release defaults, no LTO and 16 codegen units. |
| Harness | criterion 0.8.2 (no plots, no rayon): 100 samples, 0.5 s warm-up and 2 s of measurement per case, set in `benches/validation.rs`. Allocations counted over 10,000 validations after 100 warm-up ones. |
| Shared dependencies | `regex` 1.13.1, `url` 2.5.8, `unicode-segmentation` 1.13.3, `serde` 1.0.229, `serde_json` 1.0.154: one build of each for every library that uses it. annotated doesn't use `url`. |
| Load | A desktop session, load average about 1.0 to 1.5. No other cargo or rustc ran during the runs. |

## Headline

Ranked by median time among the libraries in each scenario (with LTO). End to end, annotated is ranked by `json::from_str`, with `serde_json` then `validate` beside it; `end_to_end_missing_field` isn't ranked, because its cases do different amounts of work (see [Collecting deserialization](#collecting-deserialization)).

annotated is first in every scenario with LTO, though reading the valid order end to end only by 0.4% over garde, within a run's noise. It allocates nothing on any valid input; end to end, its allocations are serde_json's own, the same as `serde_json` alone. Where something is invalid it makes the fewest allocations, but not the fewest bytes: a violation is 176 bytes, so its report's first buffer takes 704.

| Scenario | annotated's rank | annotated | Best other | annotated's allocations (bytes) | Fewest allocations of the others (bytes) |
|---|---|--:|---|--:|--:|
| `flat_valid` | 1st of 4 | 100.7 ns (report_all 102.2 ns) | garde 189.7 ns | 0 (0) | garde 1 (24) |
| `flat_every_field_invalid` | 1st of 4 | 154.7 ns (report_all 190.1 ns) | wary 281.4 ns | 2 (2,112) | wary 13 (1,943) |
| `flat_one_field_invalid` | 1st of 4 | 88.2 ns (report_all 89.4 ns) | wary 177.9 ns | 1 (704) | wary 3 (288) |
| `nested_valid` | 1st of 5 | 339.8 ns | validator 397.6 ns | 0 (0) | garde 0 (0) |
| `nested_5pct_invalid` | 1st of 5 | 464.8 ns | validator 923.2 ns | 2 (2,112) | garde 18 (1,206) |
| `map_nested_valid` | 1st of 4 | 144.8 ns | validator 461.0 ns | 0 (0) | garde 0 (0) |
| `map_nested_5pct_invalid` | 1st of 4 | 294.0 ns | validator 826.2 ns | 7 (2,177) | garde 18 (1,226) |
| `long_string_1kb`, chars | 1st of 4 | 1.60 ns | wary 28.8 ns | 0 (0) | 0 |
| `long_string_1kb`, bytes | 1st of 3 | 1.28 ns | wary 2.01 ns | 0 (0) | 0 |
| `long_string_1kb`, graphemes | 1st of 4 | 1.33 ns | wary 9.42 µs | 0 (0) | 0 |
| `long_string_64kb`, chars | 1st of 4 | 1.59 ns | wary 1.79 µs | 0 (0) | 0 |
| `long_string_64kb`, bytes | 1st of 3 | 1.28 ns | wary 2.00 ns | 0 (0) | 0 |
| `long_string_64kb`, graphemes | 1st of 4 | 1.35 ns | wary 603.23 µs | 0 (0) | 0 |
| `end_to_end_valid` | 1st of 4 | 299.0 ns (serde_json then validate 293.6 ns) | garde 406.3 ns | 6 (140) | garde 7 (164) |
| `end_to_end_every_field_invalid` | 1st of 4 | 363.1 ns (serde_json then validate 364.4 ns) | wary 488.0 ns | 7 (2,437) | wary 18 (2,268) |
| `end_to_end_one_field_invalid` | 1st of 4 | 277.6 ns (serde_json then validate 285.2 ns) | wary 402.0 ns | 7 (844) | wary 9 (428) |
| `end_to_end_nested_valid` | 1st of 5 | 7.85 µs (serde_json then validate 7.77 µs) | garde 7.88 µs | 118 (17,114) | garde 118 (17,114) |
| `end_to_end_nested_5pct_invalid` | 1st of 5 | 7.85 µs (serde_json then validate 7.93 µs) | garde 8.62 µs | 120 (19,226) | garde 136 (18,320) |
| `end_to_end_map_nested_valid` | 1st of 4 | 9.82 µs (serde_json then validate 9.62 µs) | garde 10.09 µs | 217 (13,122) | garde 217 (13,122) |
| `end_to_end_map_nested_5pct_invalid` | 1st of 4 | 10.43 µs (serde_json then validate 9.96 µs) | validator 10.70 µs | 229 (19,099) | garde 235 (14,348) |

Without LTO, cargo's default for `--release`, annotated is second in three cases: the trivial byte-length check, where wary's 2.7 ns beats its 3.4 ns, and, read end to end, the valid order (8.27 µs against validator's 8.22) and the warehouse with 5 invalid entries (11.88 µs against validator's 11.69):

| Scenario | annotated's rank | annotated | Best other | annotated's allocations (bytes) | Fewest allocations of the others (bytes) |
|---|---|--:|---|--:|--:|
| `flat_valid` | 1st of 4 | 102.2 ns (report_all 102.6 ns) | garde 209.3 ns | 0 (0) | garde 1 (24) |
| `flat_every_field_invalid` | 1st of 4 | 178.7 ns (report_all 237.9 ns) | wary 299.1 ns | 2 (2,112) | wary 13 (1,943) |
| `flat_one_field_invalid` | 1st of 4 | 96.4 ns (report_all 96.0 ns) | wary 201.0 ns | 1 (704) | wary 3 (288) |
| `nested_valid` | 1st of 5 | 519.0 ns | validator 749.9 ns | 0 (0) | garde 0 (0) |
| `nested_5pct_invalid` | 1st of 5 | 707.1 ns | validator 1.32 µs | 2 (2,112) | garde 18 (1,206) |
| `map_nested_valid` | 1st of 4 | 316.6 ns | garde 707.5 ns | 0 (0) | garde 0 (0) |
| `map_nested_5pct_invalid` | 1st of 4 | 545.4 ns | validator 1.24 µs | 7 (2,177) | garde 18 (1,226) |
| `long_string_1kb`, chars | 1st of 4 | 5.65 ns | wary 29.6 ns | 0 (0) | 0 |
| `long_string_1kb`, bytes | 2nd of 3 | 3.43 ns | wary 2.73 ns | 0 (0) | 0 |
| `long_string_1kb`, graphemes | 1st of 4 | 3.48 ns | serde_valid 9.46 µs | 0 (0) | 0 |
| `long_string_64kb`, chars | 1st of 4 | 5.65 ns | wary 1.79 µs | 0 (0) | 0 |
| `long_string_64kb`, bytes | 2nd of 3 | 3.44 ns | wary 2.73 ns | 0 (0) | 0 |
| `long_string_64kb`, graphemes | 1st of 4 | 3.47 ns | serde_valid 603.36 µs | 0 (0) | 0 |
| `end_to_end_valid` | 1st of 4 | 321.1 ns (serde_json then validate 318.5 ns) | garde 438.3 ns | 6 (140) | garde 7 (164) |
| `end_to_end_every_field_invalid` | 1st of 4 | 408.6 ns (serde_json then validate 412.8 ns) | wary 538.8 ns | 7 (2,437) | wary 18 (2,268) |
| `end_to_end_one_field_invalid` | 1st of 4 | 307.0 ns (serde_json then validate 307.8 ns) | wary 431.8 ns | 7 (844) | wary 9 (428) |
| `end_to_end_nested_valid` | 2nd of 5 | 8.27 µs (serde_json then validate 8.08 µs) | validator 8.22 µs | 118 (17,114) | garde 118 (17,114) |
| `end_to_end_nested_5pct_invalid` | 1st of 5 | 8.50 µs (serde_json then validate 8.33 µs) | validator 9.05 µs | 120 (19,226) | garde 136 (18,320) |
| `end_to_end_map_nested_valid` | 1st of 4 | 10.96 µs (serde_json then validate 10.41 µs) | validator 11.10 µs | 217 (13,122) | garde 217 (13,122) |
| `end_to_end_map_nested_5pct_invalid` | 2nd of 4 | 11.88 µs (serde_json then validate 11.06 µs) | validator 11.69 µs | 229 (19,099) | garde 235 (14,348) |

## Collecting deserialization

`annotated::json::from_str` reads a body and runs the same rules as `validate()`, in one pass. On a complete, well-typed body it finds exactly what `serde_json::from_str` then `validate()` finds, in the same order, so the two are compared like for like.

- **Flat bodies: on par.** 299.0 against 293.6 ns on the valid sign-up body with LTO, 363.1 against 364.4 ns with every field invalid, 277.6 against 285.2 ns with one; without LTO, 321.1 against 318.5, 408.6 against 412.8 and 307.0 against 307.8 ns. Either way annotated is 1.36× ahead of the next library on the valid body, garde at 406.3 ns.
- **`#[annotated(collect_type_errors)]` costs about a fifth on a valid body**: 360.5 against 299.0 ns with LTO (+21%), 393.2 against 321.1 ns without (+22%). It reads each field into a buffer before its type, borrowing the strings serde_json lends, so it makes the same 6 allocations of 140 bytes.
- **Failing fast** answers the first violation in 206.3 ns with every field invalid, against 219.6 ns for `serde_json` then `validate_fail_fast`, and 363.1 ns for every violation; with one field invalid, 217.9 against 223.8 ns. Without LTO, 224.4 against 234.8 ns and 239.8 against 243.9 ns. Both ways answer the same single violation.
- **Valid nested bodies: 1–4% behind with LTO, 2–5% without.** Reading the order of 100 lines, `json::from_str` took 7.85 µs against 7.77 µs for `serde_json` then `validate()` (+1.0%); the warehouse of 100 entries, 9.82 against 9.62 µs (+2.1%). Without LTO, 8.27 against 8.08 µs (+2.4%) and 10.96 against 10.41 µs (+5.3%). In repeated, alternating runs of the same code during the work below, the gap with LTO ranged from 1.4 to 3.0% on the order and from 1.7 to 4.4% on the warehouse. In instructions, measured apart through `perf_event_open`, the order takes 214,448 against 210,101 (+2.1%) and the warehouse 247,807 against 243,704 (+1.7%). On the valid order annotated, validator and garde are within a run's noise of each other end to end: 7.85, 8.81 and 7.88 µs with LTO; 8.27, 8.22 and 8.73 µs without.
- **Nested bodies with violations.** The order with 5 invalid lines reads as fast as `serde_json` then `validate()` (7.85 against 7.93 µs with LTO). The warehouse with 5 invalid entries trails it by 4.7% with LTO and 7.4% without, because a map keeps each value's violations apart until it has read them all, to report them in key order: 229 allocations and 19,099 bytes against 224 and 15,299.
- **What closed part of the gap**, measured in each commit: `29a23e2` lets a struct's serde visitor inline the range check, which `validate_into` already did, and `f881d35` reads a `BTreeMap`'s values straight into the map's report instead of one report per value. [Optimisation opportunities](#optimisation-opportunities-in-annotated) has the numbers and what's left.
- **A body missing a field isn't like for like.** `end_to_end_missing_field` sends the sign-up body without `postalCode`, its email and age broken. `json::from_str` reads the whole body, checks every field that came and answers three violations, in 277.3 ns with 6 allocations. serde's derive stops at the missing field once it has read the body, so with `serde_json::from_str` no library validates: every one, annotated's `Validate` included, answers serde's error alone, in 213.6 to 224.6 ns, as `serde_json` alone does in 214.2 ns. The client learns about the email and the age only on its next requests. The table counts serde's error as one problem. The timings say what each answer costs, not which library validates faster.

## History

annotated in the three runs: the first, at `b58e76b`, the second, at `99972e3`, and this one, at `f881d35`. The `b58e76b` run measured without LTO only where code generation mattered, so the long strings and the string map have no number there. Each run is a different binary: read the large moves, and see [How far to trust these numbers](#how-far-to-trust-these-numbers) for how far the same code moves between builds.

| Scenario | b58e76b | 99972e3 | f881d35 | b58e76b, no LTO | 99972e3, no LTO | f881d35, no LTO | Allocations (bytes), b58e76b → 99972e3 → f881d35 |
|---|--:|--:|--:|--:|--:|--:|---|
| `flat_valid` | 217.1 ns | 100.1 ns | 100.7 ns | 250.6 ns | 109.8 ns | 102.2 ns | 2 (32) → 0 (0) → 0 (0) |
| `flat_every_field_invalid` | 177.7 ns | 155.1 ns | 154.7 ns | 218.8 ns | 186.3 ns | 178.7 ns | 11 (1,743) → 2 (2,112) → 2 (2,112) |
| `flat_one_field_invalid` | 205.3 ns | 85.2 ns | 88.2 ns | 243.1 ns | 98.8 ns | 96.4 ns | 4 (568) → 1 (704) → 1 (704) |
| `nested_valid` | 426.7 ns | 345.0 ns | 339.8 ns | 1.02 µs | 509.2 ns | 519.0 ns | 0 (0) → 0 (0) → 0 (0) |
| `nested_5pct_invalid` | 568.8 ns | 476.6 ns | 464.8 ns | 1.21 µs | 700.2 ns | 707.1 ns | 7 (1,896) → 2 (2,112) → 2 (2,112) |
| `map_nested_valid` | 354.1 ns | 122.7 ns | 144.8 ns | 803.1 ns | 314.1 ns | 316.6 ns | 0 (0) → 0 (0) → 0 (0) |
| `map_nested_5pct_invalid` | 464.2 ns | 270.1 ns | 294.0 ns | 1.08 µs | 538.8 ns | 545.4 ns | 12 (1,961) → 7 (2,177) → 7 (2,177) |
| `map_string_values_valid` | 506.3 ns | 109.2 ns | 113.0 ns | — | 147.4 ns | 133.4 ns | 0 (0) → 0 (0) → 0 (0) |
| `long_string_1kb`, chars | 28.5 ns | 1.66 ns | 1.60 ns | — | 5.31 ns | 5.65 ns | 0 (0) → 0 (0) → 0 (0) |
| `long_string_1kb`, bytes | 2.73 ns | 1.48 ns | 1.28 ns | — | 3.17 ns | 3.43 ns | 0 (0) → 0 (0) → 0 (0) |
| `long_string_1kb`, graphemes | 9.38 µs | 1.33 ns | 1.33 ns | — | 3.50 ns | 3.48 ns | 0 (0) → 0 (0) → 0 (0) |
| `long_string_64kb`, chars | 1.78 µs | 1.65 ns | 1.59 ns | — | 5.32 ns | 5.65 ns | 0 (0) → 0 (0) → 0 (0) |
| `long_string_64kb`, bytes | 2.56 ns | 1.49 ns | 1.28 ns | — | 3.15 ns | 3.44 ns | 0 (0) → 0 (0) → 0 (0) |
| `long_string_64kb`, graphemes | 601.01 µs | 1.33 ns | 1.35 ns | — | 3.53 ns | 3.47 ns | 0 (0) → 0 (0) → 0 (0) |
| `end_to_end_valid` | 443.4 ns | 297.4 ns | 293.6 ns | 491.2 ns | 314.6 ns | 318.5 ns | 8 (172) → 6 (140) → 6 (140) |
| `end_to_end_every_field_invalid` | 391.4 ns | 365.3 ns | 364.4 ns | 443.0 ns | 406.6 ns | 412.8 ns | 16 (2,068) → 7 (2,437) → 7 (2,437) |
| `end_to_end_one_field_invalid` | 430.5 ns | 286.5 ns | 285.2 ns | 479.4 ns | 313.2 ns | 307.8 ns | 10 (708) → 7 (844) → 7 (844) |

Between `99972e3` and `f881d35` annotated gained collecting deserialization, `validate_fail_fast`, and the two changes that came out of measuring the new scenarios, `29a23e2` and `f881d35`. Validation itself barely changed: failing fast adds a check after each violation recorded, so only invalid paths run more code. The rises in `map_nested_valid` (122.7 → 144.8 ns), `map_nested_5pct_invalid` and `flat_valid` with LTO come from where the code landed in the binary, not from what it does: built so that identical code sits identically, the two revisions are within 2.4% of each other on every valid path over structs. `flat_one_field_invalid` is 2.5% slower even then, the cost of the check failing fast added after each violation.

What each earlier commit did, measured on its own against the commit before it:

- **`8790b68` perf(derive): check each rule inline and record violations out of line.** A passing rule costs one branch, and a broken one calls a cold function. Valid paths no longer depend on whether LLVM inlines a helper: `nested_valid` went from 1.03 µs to 581 ns without LTO.
- **`7ec9d3f` perf(core): settle a string's length by its byte length before counting.** A string has no more chars than bytes and no fewer than a quarter of them, so most lengths are settled without counting: the long strings went from microseconds to nanoseconds, and `map_string_values_valid` from 475 to 115 ns.
- **`cbd7a6a` perf(core): keep a path of up to three segments without allocating.** One allocation fewer per violation: `flat_every_field_invalid` from 11 to 3, `nested_5pct_invalid` from 7 to 2. A violation grew from 128 to 176 bytes.
- **`99972e3` feat(core)!: recognize URLs by the grammar of RFC 3986 and RFC 3987.** The `url` rule stopped parsing with the `url` crate: `experiment_url` went from 122 to 39 ns, and the flat body's last two allocations went with it.
- **`29a23e2` perf(core): inline the range check where the rule is written** and **`f881d35` perf(core): read a btree map's values into the map's own report**: items 8 and 9 of [Optimisation opportunities](#optimisation-opportunities-in-annotated).

## How far to trust these numbers

- **Where the code lands moves annotated's valid paths by up to 18%.** `map_nested_valid` reads 144.8 ns in this run against 122.7 ns at `99972e3`, yet its `validate_into` is byte for byte the code it was: the linker put it at another offset of its 64-byte cache line. A control run timed both revisions in one session, alternating, built as the README says and again with every function and jump target aligned to 64 bytes (`RUSTFLAGS="-C llvm-args=-align-all-functions=6 -C llvm-args=-align-all-nofallthru-blocks=6"`), so identical code sits identically. Medians of two runs each, with LTO / without:

  | Case | `99972e3`, as built | `f881d35`, as built | `99972e3`, aligned | `f881d35`, aligned |
  |---|--:|--:|--:|--:|
  | `flat_valid` | 98.3 ns / 106.2 ns | 101.2 ns / 102.3 ns | 91.7 ns / 96.8 ns | 92.5 ns / 94.5 ns |
  | `flat_every_field_invalid` | 151.8 ns / 186.8 ns | 153.2 ns / 182.8 ns | 153.2 ns / 183.7 ns | 151.8 ns / 186.5 ns |
  | `flat_one_field_invalid` | 85.1 ns / 96.0 ns | 89.0 ns / 96.5 ns | 79.6 ns / 89.6 ns | 81.6 ns / 87.9 ns |
  | `nested_valid` | 345.2 ns / 508.5 ns | 339.8 ns / 518.4 ns | 340.8 ns / 488.0 ns | 340.1 ns / 478.3 ns |
  | `nested_5pct_invalid` | 479.6 ns / 688.9 ns | 457.8 ns / 707.4 ns | 432.6 ns / 674.4 ns | 434.2 ns / 784.0 ns |
  | `map_nested_valid` | 122.3 ns / 313.8 ns | 144.7 ns / 320.6 ns | 121.1 ns / 307.9 ns | 122.8 ns / 303.4 ns |
  | `map_nested_5pct_invalid` | 267.6 ns / 532.2 ns | 292.3 ns / 544.4 ns | 260.6 ns / 522.9 ns | 265.7 ns / 526.1 ns |
  | `map_string_values_valid` | 106.2 ns / 146.4 ns | 112.0 ns / 133.6 ns | 102.6 ns / 129.1 ns | 102.3 ns / 129.0 ns |
  | `long_string_64kb`, chars | 1.65 ns / 5.44 ns | 1.59 ns / 5.65 ns | 1.51 ns / 5.34 ns | 1.48 ns / 5.65 ns |
  | `long_string_64kb`, bytes | 1.47 ns / 3.19 ns | 1.28 ns / 3.46 ns | 1.29 ns / 3.29 ns | 1.29 ns / 3.12 ns |
  | `long_string_64kb`, graphemes | 1.29 ns / 3.56 ns | 1.34 ns / 3.50 ns | 1.29 ns / 2.99 ns | 1.33 ns / 3.27 ns |

  Aligned, the valid paths over structs of the two revisions are within 2.4% of each other, and the long-string checks, which take 1.3 to 5.7 ns, within 0.3 ns either way; as built, they differ by up to 18%. `flat_one_field_invalid` is 2.5% slower aligned too, which fits the check failing fast added after each violation. `nested_5pct_invalid` without LTO jumps between about 690 and 785 ns from run to run of the same binary, so it says nothing either way. Compare libraries within a run, and read a move of a few percent in one case across builds as placement unless an aligned build or the instruction count says otherwise. In the experiments, neither side of `experiment_length_short_text` changed, yet it went from 1.46 / 1.46 ns to 1.8 / 1.3 ns. The earlier claim that valid paths no longer swing with the rest of the binary held for inlining, which `8790b68` settled, not for placement.
- **Invalid paths move more**, by up to about 15% for the same code. They allocate, and the allocator's state depends on what ran before them in the same process. Compare libraries within a run, not nanoseconds across runs.
- **Noise.** 126 of the 129 medians, with LTO and without, have a 95% confidence interval within 1% of them; all are within 3.7%. The widest are a few nanoseconds' worth on the long strings and the length experiments.
- **No hardware profiler.** `perf` and valgrind aren't installed. The instruction counts in this file were read through `perf_event_open`, in a probe binary kept out of the repository; the explanations come from them, the generated code, the machine code and the experiments.
- **Allocation counts are exact.** Every case allocates the same on every validation after warm-up.

## Scenarios (with LTO)

### `flat_valid`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| annotated | git f881d35 | 100.7 ns | 100.4 ns – 101.0 ns | 1.00× | 0.00 | 0 | 0 |
| annotated (report_all) | git f881d35 | 102.2 ns | 101.9 ns – 102.4 ns | 1.01× | 0.00 | 0 | 0 |
| validator | 0.21.0 | 194.7 ns | 194.6 ns – 194.9 ns | 1.93× | 2.00 | 88 | 0 |
| garde | 0.23.0 | 189.7 ns | 189.5 ns – 189.9 ns | 1.88× | 1.00 | 24 | 0 |
| wary | 0.3.2 | 205.8 ns | 205.6 ns – 206.2 ns | 2.04× | 2.00 | 47 | 0 |

### `flat_every_field_invalid`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| annotated | git f881d35 | 154.7 ns | 154.6 ns – 154.9 ns | 1.00× | 2.00 | 2112 | 8 |
| annotated (report_all) | git f881d35 | 190.1 ns | 189.9 ns – 190.3 ns | 1.23× | 3.00 | 4928 | 9 |
| validator | 0.21.0 | 747.7 ns | 747.4 ns – 748.1 ns | 4.83× | 27.00 | 7254 | 9 |
| garde | 0.23.0 | 765.9 ns | 761.3 ns – 768.2 ns | 4.95× | 27.00 | 2219 | 9 |
| wary | 0.3.2 | 281.4 ns | 281.3 ns – 281.4 ns | 1.82× | 13.00 | 1943 | 9 |

### `flat_one_field_invalid`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| annotated | git f881d35 | 88.2 ns | 88.0 ns – 88.4 ns | 1.00× | 1.00 | 704 | 1 |
| annotated (report_all) | git f881d35 | 89.4 ns | 89.2 ns – 89.6 ns | 1.01× | 1.00 | 704 | 1 |
| validator | 0.21.0 | 220.8 ns | 220.7 ns – 220.9 ns | 2.50× | 5.00 | 919 | 1 |
| garde | 0.23.0 | 265.3 ns | 265.0 ns – 265.6 ns | 3.01× | 6.00 | 393 | 1 |
| wary | 0.3.2 | 177.9 ns | 177.6 ns – 178.0 ns | 2.02× | 3.00 | 288 | 1 |

### `nested_valid`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| annotated | git f881d35 | 339.8 ns | 339.6 ns – 339.9 ns | 1.00× | 0.00 | 0 | 0 |
| validator | 0.21.0 | 397.6 ns | 397.4 ns – 397.8 ns | 1.17× | 1.00 | 244 | 0 |
| garde | 0.23.0 | 613.8 ns | 612.6 ns – 614.9 ns | 1.81× | 0.00 | 0 | 0 |
| wary | 0.3.2 | 3.01 µs | 3.01 µs – 3.02 µs | 8.87× | 102.00 | 4080 | 0 |
| serde_valid (graphemes) | 3.2.0 | 17.48 µs | 17.47 µs – 17.49 µs | 51.44× | 0.00 | 0 | 0 |

### `nested_5pct_invalid`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| annotated | git f881d35 | 464.8 ns | 464.6 ns – 465.2 ns | 1.00× | 2.00 | 2112 | 5 |
| validator | 0.21.0 | 923.2 ns | 922.3 ns – 923.9 ns | 1.99× | 23.00 | 5280 | 5 |
| garde | 0.23.0 | 1.25 µs | 1.24 µs – 1.25 µs | 2.68× | 18.00 | 1206 | 5 |
| wary | 0.3.2 | 3.19 µs | 3.19 µs – 3.19 µs | 6.87× | 109.00 | 4952 | 5 |
| serde_valid (graphemes) | 3.2.0 | 18.31 µs | 18.30 µs – 18.31 µs | 39.38× | 39.00 | 7036 | 5 |

### `map_nested_valid`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| annotated | git f881d35 | 144.8 ns | 144.7 ns – 144.9 ns | 1.00× | 0.00 | 0 | 0 |
| validator | 0.21.0 | 461.0 ns | 460.3 ns – 461.5 ns | 3.18× | 1.00 | 244 | 0 |
| garde | 0.23.0 | 520.3 ns | 518.3 ns – 523.3 ns | 3.59× | 0.00 | 0 | 0 |
| serde_valid (graphemes) | 3.2.0 | 15.53 µs | 15.52 µs – 15.55 µs | 107.25× | 0.00 | 0 | 0 |

### `map_nested_5pct_invalid`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| annotated | git f881d35 | 294.0 ns | 293.9 ns – 294.1 ns | 1.00× | 7.00 | 2177 | 5 |
| validator | 0.21.0 | 826.2 ns | 823.1 ns – 830.8 ns | 2.81× | 23.00 | 5280 | 5 |
| garde | 0.23.0 | 1.16 µs | 1.16 µs – 1.17 µs | 3.95× | 18.00 | 1226 | 5 |
| serde_valid (graphemes) | 3.2.0 | 16.33 µs | 16.32 µs – 16.33 µs | 55.54× | 44.00 | 7261 | 5 |

### `map_string_values_valid`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| annotated | git f881d35 | 113.0 ns | 112.6 ns – 113.6 ns | 1.00× | 0.00 | 0 | 0 |

### `long_string_1kb`

| Library | Version | Median | 95% CI of the median | vs fastest | Throughput | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|--:|
| annotated (chars) | git f881d35 | 1.6 ns | 1.6 ns – 1.6 ns | 1.25× | 595.71 GiB/s | 0.00 | 0 | 0 |
| annotated (bytes) | git f881d35 | 1.3 ns | 1.3 ns – 1.3 ns | 1.00× | 746.40 GiB/s | 0.00 | 0 | 0 |
| annotated (graphemes) | git f881d35 | 1.3 ns | 1.3 ns – 1.3 ns | 1.04× | 719.54 GiB/s | 0.00 | 0 | 0 |
| validator (chars) | 0.21.0 | 29.3 ns | 29.3 ns – 29.3 ns | 22.91× | 32.57 GiB/s | 0.00 | 0 | 0 |
| garde (chars) | 0.23.0 | 29.7 ns | 29.7 ns – 29.7 ns | 23.28× | 32.06 GiB/s | 0.00 | 0 | 0 |
| garde (bytes) | 0.23.0 | 3.9 ns | 3.9 ns – 3.9 ns | 3.06× | 244.11 GiB/s | 0.00 | 0 | 0 |
| garde (graphemes) | 0.23.0 | 9.48 µs | 9.48 µs – 9.50 µs | 7422.08× | 0.10 GiB/s | 0.00 | 0 | 0 |
| wary (chars) | 0.3.2 | 28.8 ns | 28.8 ns – 28.8 ns | 22.53× | 33.13 GiB/s | 0.00 | 0 | 0 |
| wary (bytes) | 0.3.2 | 2.0 ns | 2.0 ns – 2.0 ns | 1.57× | 475.22 GiB/s | 0.00 | 0 | 0 |
| wary (graphemes) | 0.3.2 | 9.42 µs | 9.41 µs – 9.42 µs | 7369.12× | 0.10 GiB/s | 0.00 | 0 | 0 |
| serde_valid (graphemes) | 3.2.0 | 9.54 µs | 9.53 µs – 9.54 µs | 7462.74× | 0.10 GiB/s | 0.00 | 0 | 0 |

### `long_string_64kb`

| Library | Version | Median | 95% CI of the median | vs fastest | Throughput | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|--:|
| annotated (chars) | git f881d35 | 1.6 ns | 1.6 ns – 1.6 ns | 1.25× | 38386.81 GiB/s | 0.00 | 0 | 0 |
| annotated (bytes) | git f881d35 | 1.3 ns | 1.3 ns – 1.3 ns | 1.00× | 47819.41 GiB/s | 0.00 | 0 | 0 |
| annotated (graphemes) | git f881d35 | 1.3 ns | 1.3 ns – 1.4 ns | 1.05× | 45373.83 GiB/s | 0.00 | 0 | 0 |
| validator (chars) | 0.21.0 | 1.79 µs | 1.79 µs – 1.79 µs | 1400.83× | 34.14 GiB/s | 0.00 | 0 | 0 |
| garde (chars) | 0.23.0 | 1.79 µs | 1.79 µs – 1.79 µs | 1400.48× | 34.14 GiB/s | 0.00 | 0 | 0 |
| garde (bytes) | 0.23.0 | 4.0 ns | 4.0 ns – 4.0 ns | 3.13× | 15300.98 GiB/s | 0.00 | 0 | 0 |
| garde (graphemes) | 0.23.0 | 606.92 µs | 606.53 µs – 607.16 µs | 475507.90× | 0.10 GiB/s | 0.00 | 0 | 0 |
| wary (chars) | 0.3.2 | 1.79 µs | 1.79 µs – 1.79 µs | 1400.38× | 34.15 GiB/s | 0.00 | 0 | 0 |
| wary (bytes) | 0.3.2 | 2.0 ns | 2.0 ns – 2.0 ns | 1.57× | 30461.70 GiB/s | 0.00 | 0 | 0 |
| wary (graphemes) | 0.3.2 | 603.23 µs | 602.89 µs – 603.77 µs | 472618.14× | 0.10 GiB/s | 0.00 | 0 | 0 |
| serde_valid (graphemes) | 3.2.0 | 609.53 µs | 609.26 µs – 609.80 µs | 477546.97× | 0.10 GiB/s | 0.00 | 0 | 0 |

### `end_to_end_valid`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| serde_json alone | 1.0.154 | 180.5 ns | 180.3 ns – 180.7 ns | floor | 6.00 | 140 | 0 |
| annotated (json::from_str) | git f881d35 | 299.0 ns | 298.8 ns – 299.3 ns | 1.02× | 6.00 | 140 | 0 |
| annotated (serde_json, then validate) | git f881d35 | 293.6 ns | 293.4 ns – 293.7 ns | 1.00× | 6.00 | 140 | 0 |
| annotated (json::from_str, report_all) | git f881d35 | 301.4 ns | 301.2 ns – 301.5 ns | 1.03× | 6.00 | 140 | 0 |
| annotated (serde_json, then validate, report_all) | git f881d35 | 299.5 ns | 299.3 ns – 299.8 ns | 1.02× | 6.00 | 140 | 0 |
| annotated (json::from_str, collect_type_errors) | git f881d35 | 360.5 ns | 360.3 ns – 360.7 ns | 1.23× | 6.00 | 140 | 0 |
| validator | 0.21.0 | 415.9 ns | 415.6 ns – 416.3 ns | 1.42× | 8.00 | 228 | 0 |
| garde | 0.23.0 | 406.3 ns | 405.9 ns – 406.6 ns | 1.38× | 7.00 | 164 | 0 |
| wary | 0.3.2 | 425.5 ns | 425.2 ns – 426.0 ns | 1.45× | 8.00 | 187 | 0 |

### `end_to_end_every_field_invalid`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| serde_json alone | 1.0.154 | 190.0 ns | 189.8 ns – 190.2 ns | floor | 5.00 | 325 | 0 |
| annotated (json::from_str) | git f881d35 | 363.1 ns | 361.7 ns – 363.7 ns | 1.76× | 7.00 | 2437 | 8 |
| annotated (serde_json, then validate) | git f881d35 | 364.4 ns | 364.0 ns – 364.7 ns | 1.77× | 7.00 | 2437 | 8 |
| annotated (json::from_str, report_all) | git f881d35 | 408.2 ns | 407.8 ns – 409.0 ns | 1.98× | 8.00 | 5253 | 9 |
| annotated (serde_json, then validate, report_all) | git f881d35 | 415.5 ns | 414.9 ns – 416.1 ns | 2.01× | 8.00 | 5253 | 9 |
| annotated (json::from_str_fail_fast) | git f881d35 | 206.3 ns | 206.1 ns – 206.6 ns | 1.00× | 6.00 | 1029 | 1 |
| annotated (serde_json, then validate_fail_fast) | git f881d35 | 219.6 ns | 219.5 ns – 219.8 ns | 1.06× | 6.00 | 1029 | 1 |
| validator | 0.21.0 | 998.4 ns | 997.6 ns – 1.00 µs | 4.84× | 32.00 | 7579 | 9 |
| garde | 0.23.0 | 994.6 ns | 992.0 ns – 997.5 ns | 4.82× | 32.00 | 2544 | 9 |
| wary | 0.3.2 | 488.0 ns | 487.5 ns – 488.5 ns | 2.37× | 18.00 | 2268 | 9 |

### `end_to_end_one_field_invalid`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| serde_json alone | 1.0.154 | 182.3 ns | 182.2 ns – 182.5 ns | floor | 6.00 | 140 | 0 |
| annotated (json::from_str) | git f881d35 | 277.6 ns | 277.3 ns – 278.0 ns | 1.27× | 7.00 | 844 | 1 |
| annotated (serde_json, then validate) | git f881d35 | 285.2 ns | 285.0 ns – 285.3 ns | 1.31× | 7.00 | 844 | 1 |
| annotated (json::from_str, report_all) | git f881d35 | 282.7 ns | 282.5 ns – 283.0 ns | 1.30× | 7.00 | 844 | 1 |
| annotated (serde_json, then validate, report_all) | git f881d35 | 283.7 ns | 283.3 ns – 283.9 ns | 1.30× | 7.00 | 844 | 1 |
| annotated (json::from_str_fail_fast) | git f881d35 | 217.9 ns | 217.6 ns – 218.2 ns | 1.00× | 7.00 | 844 | 1 |
| annotated (serde_json, then validate_fail_fast) | git f881d35 | 223.8 ns | 223.7 ns – 224.0 ns | 1.03× | 7.00 | 844 | 1 |
| validator | 0.21.0 | 439.7 ns | 439.4 ns – 439.9 ns | 2.02× | 11.00 | 1059 | 1 |
| garde | 0.23.0 | 490.5 ns | 490.3 ns – 490.8 ns | 2.25× | 12.00 | 533 | 1 |
| wary | 0.3.2 | 402.0 ns | 401.7 ns – 402.3 ns | 1.85× | 9.00 | 428 | 1 |

### `end_to_end_nested_valid`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| serde_json alone | 1.0.154 | 7.16 µs | 7.16 µs – 7.17 µs | floor | 118.00 | 17114 | 0 |
| annotated (json::from_str) | git f881d35 | 7.85 µs | 7.85 µs – 7.86 µs | 1.01× | 118.00 | 17114 | 0 |
| annotated (serde_json, then validate) | git f881d35 | 7.77 µs | 7.76 µs – 7.77 µs | 1.00× | 118.00 | 17114 | 0 |
| validator | 0.21.0 | 8.81 µs | 8.80 µs – 8.81 µs | 1.13× | 119.00 | 17358 | 0 |
| garde | 0.23.0 | 7.88 µs | 7.87 µs – 7.88 µs | 1.01× | 118.00 | 17114 | 0 |
| wary | 0.3.2 | 10.45 µs | 10.44 µs – 10.46 µs | 1.35× | 220.00 | 21194 | 0 |
| serde_valid (graphemes) | 3.2.0 | 24.99 µs | 24.98 µs – 25.00 µs | 3.22× | 118.00 | 17114 | 0 |

### `end_to_end_nested_5pct_invalid`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| serde_json alone | 1.0.154 | 7.19 µs | 7.19 µs – 7.20 µs | floor | 118.00 | 17114 | 0 |
| annotated (json::from_str) | git f881d35 | 7.85 µs | 7.85 µs – 7.86 µs | 1.00× | 120.00 | 19226 | 5 |
| annotated (serde_json, then validate) | git f881d35 | 7.93 µs | 7.93 µs – 7.94 µs | 1.01× | 120.00 | 19226 | 5 |
| validator | 0.21.0 | 9.45 µs | 9.44 µs – 9.45 µs | 1.20× | 141.00 | 22394 | 5 |
| garde | 0.23.0 | 8.62 µs | 8.62 µs – 8.63 µs | 1.10× | 136.00 | 18320 | 5 |
| wary | 0.3.2 | 10.61 µs | 10.61 µs – 10.62 µs | 1.35× | 227.00 | 22066 | 5 |
| serde_valid (graphemes) | 3.2.0 | 25.98 µs | 25.98 µs – 25.99 µs | 3.31× | 157.00 | 24150 | 5 |

### `end_to_end_map_nested_valid`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| serde_json alone | 1.0.154 | 9.46 µs | 9.46 µs – 9.47 µs | floor | 217.00 | 13122 | 0 |
| annotated (json::from_str) | git f881d35 | 9.82 µs | 9.82 µs – 9.82 µs | 1.02× | 217.00 | 13122 | 0 |
| annotated (serde_json, then validate) | git f881d35 | 9.62 µs | 9.62 µs – 9.63 µs | 1.00× | 217.00 | 13122 | 0 |
| validator | 0.21.0 | 10.09 µs | 10.08 µs – 10.09 µs | 1.05× | 218.00 | 13366 | 0 |
| garde | 0.23.0 | 10.09 µs | 10.08 µs – 10.09 µs | 1.05× | 217.00 | 13122 | 0 |
| serde_valid (graphemes) | 3.2.0 | 25.10 µs | 25.09 µs – 25.10 µs | 2.61× | 217.00 | 13122 | 0 |

### `end_to_end_map_nested_5pct_invalid`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| serde_json alone | 1.0.154 | 9.52 µs | 9.52 µs – 9.52 µs | floor | 217.00 | 13122 | 0 |
| annotated (json::from_str) | git f881d35 | 10.43 µs | 10.42 µs – 10.43 µs | 1.05× | 229.00 | 19099 | 5 |
| annotated (serde_json, then validate) | git f881d35 | 9.96 µs | 9.95 µs – 9.96 µs | 1.00× | 224.00 | 15299 | 5 |
| validator | 0.21.0 | 10.70 µs | 10.70 µs – 10.71 µs | 1.07× | 240.00 | 18402 | 5 |
| garde | 0.23.0 | 10.80 µs | 10.79 µs – 10.80 µs | 1.08× | 235.00 | 14348 | 5 |
| serde_valid (graphemes) | 3.2.0 | 26.14 µs | 26.12 µs – 26.15 µs | 2.62× | 261.00 | 20383 | 5 |

### `end_to_end_missing_field`

Not like for like: `json::from_str` reports all three problems; every other case answers serde's error alone, counted as one. See [Collecting deserialization](#collecting-deserialization).

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| serde_json alone | 1.0.154 | 214.2 ns | 214.0 ns – 214.3 ns | floor | 9.00 | 273 | 1 |
| annotated (json::from_str) | git f881d35 | 277.3 ns | 277.2 ns – 277.5 ns | 1.30× | 6.00 | 839 | 3 |
| annotated (serde_json, then validate) | git f881d35 | 215.9 ns | 215.8 ns – 216.0 ns | 1.01× | 9.00 | 273 | 1 |
| validator | 0.21.0 | 213.6 ns | 213.5 ns – 213.8 ns | 1.00× | 9.00 | 273 | 1 |
| garde | 0.23.0 | 217.4 ns | 217.1 ns – 217.6 ns | 1.02× | 9.00 | 273 | 1 |
| wary | 0.3.2 | 224.6 ns | 224.3 ns – 224.7 ns | 1.05× | 9.00 | 273 | 1 |

## Experiments (with LTO)

The experiments put annotated beside the same checks written by hand through its public API, the way each change would make it run. The changes they measured are now in annotated, so each pair should agree, and the inline checks still do, within 3%. `experiment_length_short_text` now reads 1.8 against 1.3 ns with LTO, though neither side's code changed since both read 1.46 ns: that is where the code landed, as [How far to trust these numbers](#how-far-to-trust-these-numbers) shows. Their labels still name the code as it was written for `b58e76b`: `rules::length(), chars counted` now settles by the byte length too. `experiment_url` is the one that still compares two different things: annotated's rule against plain `url::Url::parse`, which is what garde and validator call.

### `experiment_inline_checks_order`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| annotated (derived) | git f881d35 | 340.1 ns | 339.9 ns – 340.3 ns | 1.00× | 0.00 | 0 | 0 |
| hand-written (inline checks, violations recorded out of line) | on annotated git f881d35 | 348.9 ns | 348.8 ns – 349.2 ns | 1.03× | 0.00 | 0 | 0 |

### `experiment_inline_checks_flat`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| annotated (derived) | git f881d35 | 100.8 ns | 100.6 ns – 101.1 ns | 1.00× | 0.00 | 0 | 0 |
| hand-written (inline checks, violations recorded out of line) | on annotated git f881d35 | 102.3 ns | 102.1 ns – 102.5 ns | 1.01× | 0.00 | 0 | 0 |

### `experiment_inline_checks_flat_every_field_invalid`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| annotated (derived) | git f881d35 | 148.6 ns | 148.4 ns – 148.7 ns | 1.01× | 2.00 | 2112 | 8 |
| hand-written (inline checks, violations recorded out of line) | on annotated git f881d35 | 147.7 ns | 147.5 ns – 147.8 ns | 1.00× | 2.00 | 2112 | 8 |

### `experiment_url`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| annotated (rules::url()) | git f881d35 | 41.7 ns | 41.5 ns – 42.0 ns | 1.00× | 0.00 | 0 | 0 |
| hand-written (url::Url::parse alone, as garde and validator check) | on annotated git f881d35 | 96.5 ns | 96.3 ns – 96.6 ns | 2.31× | 1.00 | 24 | 0 |

### `experiment_length_short_text`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| annotated (rules::length(), chars counted) | git f881d35 | 1.8 ns | 1.8 ns – 1.8 ns | 1.43× | 0.00 | 0 | 0 |
| hand-written (settled by the byte length first) | on annotated git f881d35 | 1.3 ns | 1.3 ns – 1.3 ns | 1.00× | 0.00 | 0 | 0 |

### `experiment_length_64kb`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| annotated (rules::length(), chars counted) | git f881d35 | 1.8 ns | 1.8 ns – 1.8 ns | 1.09× | 0.00 | 0 | 0 |
| hand-written (settled by the byte length first) | on annotated git f881d35 | 1.7 ns | 1.7 ns – 1.7 ns | 1.00× | 0.00 | 0 | 0 |

## Scenarios without LTO

The same cases built with cargo's release defaults (`--profile release-without-lto`), every one of them. The allocation columns are the counts from the LTO build: inlining doesn't change what is allocated.

### `flat_valid`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| annotated | git f881d35 | 102.2 ns | 102.0 ns – 102.3 ns | 1.00× | 0.00 | 0 | 0 |
| annotated (report_all) | git f881d35 | 102.6 ns | 102.4 ns – 102.8 ns | 1.00× | 0.00 | 0 | 0 |
| validator | 0.21.0 | 224.0 ns | 223.7 ns – 224.1 ns | 2.19× | 2.00 | 88 | 0 |
| garde | 0.23.0 | 209.3 ns | 209.0 ns – 209.7 ns | 2.05× | 1.00 | 24 | 0 |
| wary | 0.3.2 | 230.7 ns | 230.5 ns – 230.9 ns | 2.26× | 2.00 | 47 | 0 |

### `flat_every_field_invalid`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| annotated | git f881d35 | 178.7 ns | 178.6 ns – 178.9 ns | 1.00× | 2.00 | 2112 | 8 |
| annotated (report_all) | git f881d35 | 238.0 ns | 237.7 ns – 238.2 ns | 1.33× | 3.00 | 4928 | 9 |
| validator | 0.21.0 | 868.3 ns | 867.6 ns – 868.8 ns | 4.86× | 27.00 | 7254 | 9 |
| garde | 0.23.0 | 789.8 ns | 787.9 ns – 790.9 ns | 4.42× | 27.00 | 2219 | 9 |
| wary | 0.3.2 | 299.1 ns | 299.0 ns – 299.1 ns | 1.67× | 13.00 | 1943 | 9 |

### `flat_one_field_invalid`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| annotated | git f881d35 | 96.4 ns | 96.2 ns – 96.6 ns | 1.00× | 1.00 | 704 | 1 |
| annotated (report_all) | git f881d35 | 96.0 ns | 95.8 ns – 96.4 ns | 1.00× | 1.00 | 704 | 1 |
| validator | 0.21.0 | 249.2 ns | 249.1 ns – 249.4 ns | 2.60× | 5.00 | 919 | 1 |
| garde | 0.23.0 | 297.8 ns | 297.5 ns – 298.0 ns | 3.10× | 6.00 | 393 | 1 |
| wary | 0.3.2 | 201.0 ns | 200.8 ns – 201.2 ns | 2.09× | 3.00 | 288 | 1 |

### `nested_valid`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| annotated | git f881d35 | 519.0 ns | 518.2 ns – 520.0 ns | 1.00× | 0.00 | 0 | 0 |
| validator | 0.21.0 | 749.9 ns | 749.7 ns – 750.1 ns | 1.44× | 1.00 | 244 | 0 |
| garde | 0.23.0 | 884.5 ns | 882.5 ns – 886.5 ns | 1.70× | 0.00 | 0 | 0 |
| wary | 0.3.2 | 3.27 µs | 3.27 µs – 3.27 µs | 6.29× | 102.00 | 4080 | 0 |
| serde_valid (graphemes) | 3.2.0 | 17.15 µs | 17.12 µs – 17.17 µs | 33.03× | 0.00 | 0 | 0 |

### `nested_5pct_invalid`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| annotated | git f881d35 | 707.1 ns | 705.4 ns – 710.5 ns | 1.00× | 2.00 | 2112 | 5 |
| validator | 0.21.0 | 1.32 µs | 1.32 µs – 1.32 µs | 1.87× | 23.00 | 5280 | 5 |
| garde | 0.23.0 | 1.55 µs | 1.55 µs – 1.55 µs | 2.20× | 18.00 | 1206 | 5 |
| wary | 0.3.2 | 3.37 µs | 3.37 µs – 3.37 µs | 4.76× | 109.00 | 4952 | 5 |
| serde_valid (graphemes) | 3.2.0 | 17.90 µs | 17.89 µs – 17.91 µs | 25.32× | 39.00 | 7036 | 5 |

### `map_nested_valid`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| annotated | git f881d35 | 316.6 ns | 315.7 ns – 317.5 ns | 1.00× | 0.00 | 0 | 0 |
| validator | 0.21.0 | 727.6 ns | 726.7 ns – 728.3 ns | 2.30× | 1.00 | 244 | 0 |
| garde | 0.23.0 | 707.5 ns | 707.2 ns – 708.0 ns | 2.23× | 0.00 | 0 | 0 |
| serde_valid (graphemes) | 3.2.0 | 15.34 µs | 15.32 µs – 15.36 µs | 48.45× | 0.00 | 0 | 0 |

### `map_nested_5pct_invalid`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| annotated | git f881d35 | 545.4 ns | 544.1 ns – 546.8 ns | 1.00× | 7.00 | 2177 | 5 |
| validator | 0.21.0 | 1.24 µs | 1.23 µs – 1.24 µs | 2.27× | 23.00 | 5280 | 5 |
| garde | 0.23.0 | 1.36 µs | 1.36 µs – 1.36 µs | 2.49× | 18.00 | 1226 | 5 |
| serde_valid (graphemes) | 3.2.0 | 16.28 µs | 16.25 µs – 16.32 µs | 29.85× | 44.00 | 7261 | 5 |

### `map_string_values_valid`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| annotated | git f881d35 | 133.4 ns | 133.2 ns – 133.7 ns | 1.00× | 0.00 | 0 | 0 |

### `long_string_1kb`

| Library | Version | Median | 95% CI of the median | vs fastest | Throughput | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|--:|
| annotated (chars) | git f881d35 | 5.7 ns | 5.7 ns – 5.7 ns | 2.07× | 168.73 GiB/s | 0.00 | 0 | 0 |
| annotated (bytes) | git f881d35 | 3.4 ns | 3.4 ns – 3.4 ns | 1.26× | 277.82 GiB/s | 0.00 | 0 | 0 |
| annotated (graphemes) | git f881d35 | 3.5 ns | 3.5 ns – 3.5 ns | 1.27× | 274.34 GiB/s | 0.00 | 0 | 0 |
| validator (chars) | 0.21.0 | 30.8 ns | 30.8 ns – 30.8 ns | 11.27× | 30.96 GiB/s | 0.00 | 0 | 0 |
| garde (chars) | 0.23.0 | 30.5 ns | 30.5 ns – 30.5 ns | 11.15× | 31.29 GiB/s | 0.00 | 0 | 0 |
| garde (bytes) | 0.23.0 | 5.5 ns | 5.5 ns – 5.6 ns | 2.02× | 172.55 GiB/s | 0.00 | 0 | 0 |
| garde (graphemes) | 0.23.0 | 9.68 µs | 9.67 µs – 9.69 µs | 3540.09× | 0.10 GiB/s | 0.00 | 0 | 0 |
| wary (chars) | 0.3.2 | 29.6 ns | 29.6 ns – 29.6 ns | 10.84× | 32.17 GiB/s | 0.00 | 0 | 0 |
| wary (bytes) | 0.3.2 | 2.7 ns | 2.7 ns – 2.7 ns | 1.00× | 348.89 GiB/s | 0.00 | 0 | 0 |
| wary (graphemes) | 0.3.2 | 9.46 µs | 9.46 µs – 9.46 µs | 3461.62× | 0.10 GiB/s | 0.00 | 0 | 0 |
| serde_valid (graphemes) | 3.2.0 | 9.46 µs | 9.45 µs – 9.46 µs | 3459.32× | 0.10 GiB/s | 0.00 | 0 | 0 |

### `long_string_64kb`

| Library | Version | Median | 95% CI of the median | vs fastest | Throughput | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|--:|
| annotated (chars) | git f881d35 | 5.7 ns | 5.7 ns – 5.7 ns | 2.07× | 10797.41 GiB/s | 0.00 | 0 | 0 |
| annotated (bytes) | git f881d35 | 3.4 ns | 3.4 ns – 3.5 ns | 1.26× | 17722.76 GiB/s | 0.00 | 0 | 0 |
| annotated (graphemes) | git f881d35 | 3.5 ns | 3.5 ns – 3.5 ns | 1.27× | 17614.20 GiB/s | 0.00 | 0 | 0 |
| validator (chars) | 0.21.0 | 1.79 µs | 1.79 µs – 1.79 µs | 654.97× | 34.08 GiB/s | 0.00 | 0 | 0 |
| garde (chars) | 0.23.0 | 1.79 µs | 1.79 µs – 1.79 µs | 654.66× | 34.10 GiB/s | 0.00 | 0 | 0 |
| garde (bytes) | 0.23.0 | 5.5 ns | 5.5 ns – 5.6 ns | 2.02× | 11029.13 GiB/s | 0.00 | 0 | 0 |
| garde (graphemes) | 0.23.0 | 616.82 µs | 616.62 µs – 617.12 µs | 225604.48× | 0.10 GiB/s | 0.00 | 0 | 0 |
| wary (chars) | 0.3.2 | 1.79 µs | 1.79 µs – 1.79 µs | 654.59× | 34.10 GiB/s | 0.00 | 0 | 0 |
| wary (bytes) | 0.3.2 | 2.7 ns | 2.7 ns – 2.7 ns | 1.00× | 22323.74 GiB/s | 0.00 | 0 | 0 |
| wary (graphemes) | 0.3.2 | 603.90 µs | 603.62 µs – 604.22 µs | 220876.19× | 0.10 GiB/s | 0.00 | 0 | 0 |
| serde_valid (graphemes) | 3.2.0 | 603.36 µs | 603.03 µs – 603.70 µs | 220680.50× | 0.10 GiB/s | 0.00 | 0 | 0 |

### `end_to_end_valid`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| serde_json alone | 1.0.154 | 199.2 ns | 198.9 ns – 199.6 ns | floor | 6.00 | 140 | 0 |
| annotated (json::from_str) | git f881d35 | 321.1 ns | 320.7 ns – 321.3 ns | 1.01× | 6.00 | 140 | 0 |
| annotated (serde_json, then validate) | git f881d35 | 318.5 ns | 318.1 ns – 318.7 ns | 1.00× | 6.00 | 140 | 0 |
| annotated (json::from_str, report_all) | git f881d35 | 317.1 ns | 316.8 ns – 317.4 ns | 1.00× | 6.00 | 140 | 0 |
| annotated (serde_json, then validate, report_all) | git f881d35 | 320.6 ns | 320.3 ns – 320.7 ns | 1.01× | 6.00 | 140 | 0 |
| annotated (json::from_str, collect_type_errors) | git f881d35 | 393.2 ns | 392.7 ns – 393.4 ns | 1.24× | 6.00 | 140 | 0 |
| validator | 0.21.0 | 471.3 ns | 470.8 ns – 471.6 ns | 1.49× | 8.00 | 228 | 0 |
| garde | 0.23.0 | 438.3 ns | 438.0 ns – 438.6 ns | 1.38× | 7.00 | 164 | 0 |
| wary | 0.3.2 | 464.5 ns | 464.2 ns – 465.0 ns | 1.46× | 8.00 | 187 | 0 |

### `end_to_end_every_field_invalid`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| serde_json alone | 1.0.154 | 206.0 ns | 205.8 ns – 206.3 ns | floor | 5.00 | 325 | 0 |
| annotated (json::from_str) | git f881d35 | 408.5 ns | 408.0 ns – 409.2 ns | 1.82× | 7.00 | 2437 | 8 |
| annotated (serde_json, then validate) | git f881d35 | 412.8 ns | 412.3 ns – 413.3 ns | 1.84× | 7.00 | 2437 | 8 |
| annotated (json::from_str, report_all) | git f881d35 | 441.9 ns | 441.0 ns – 442.7 ns | 1.97× | 8.00 | 5253 | 9 |
| annotated (serde_json, then validate, report_all) | git f881d35 | 448.3 ns | 447.7 ns – 449.3 ns | 2.00× | 8.00 | 5253 | 9 |
| annotated (json::from_str_fail_fast) | git f881d35 | 224.4 ns | 224.3 ns – 224.7 ns | 1.00× | 6.00 | 1029 | 1 |
| annotated (serde_json, then validate_fail_fast) | git f881d35 | 234.8 ns | 234.6 ns – 235.2 ns | 1.05× | 6.00 | 1029 | 1 |
| validator | 0.21.0 | 1.15 µs | 1.15 µs – 1.15 µs | 5.14× | 32.00 | 7579 | 9 |
| garde | 0.23.0 | 1.04 µs | 1.04 µs – 1.04 µs | 4.64× | 32.00 | 2544 | 9 |
| wary | 0.3.2 | 538.8 ns | 538.3 ns – 539.3 ns | 2.40× | 18.00 | 2268 | 9 |

### `end_to_end_one_field_invalid`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| serde_json alone | 1.0.154 | 199.0 ns | 198.6 ns – 199.1 ns | floor | 6.00 | 140 | 0 |
| annotated (json::from_str) | git f881d35 | 307.0 ns | 306.7 ns – 307.4 ns | 1.28× | 7.00 | 844 | 1 |
| annotated (serde_json, then validate) | git f881d35 | 307.8 ns | 307.4 ns – 308.0 ns | 1.28× | 7.00 | 844 | 1 |
| annotated (json::from_str, report_all) | git f881d35 | 303.7 ns | 303.5 ns – 303.9 ns | 1.27× | 7.00 | 844 | 1 |
| annotated (serde_json, then validate, report_all) | git f881d35 | 309.6 ns | 309.3 ns – 309.9 ns | 1.29× | 7.00 | 844 | 1 |
| annotated (json::from_str_fail_fast) | git f881d35 | 239.8 ns | 239.6 ns – 240.3 ns | 1.00× | 7.00 | 844 | 1 |
| annotated (serde_json, then validate_fail_fast) | git f881d35 | 243.9 ns | 243.7 ns – 244.1 ns | 1.02× | 7.00 | 844 | 1 |
| validator | 0.21.0 | 492.1 ns | 491.7 ns – 492.4 ns | 2.05× | 11.00 | 1059 | 1 |
| garde | 0.23.0 | 536.9 ns | 536.6 ns – 537.1 ns | 2.24× | 12.00 | 533 | 1 |
| wary | 0.3.2 | 431.8 ns | 431.4 ns – 432.2 ns | 1.80× | 9.00 | 428 | 1 |

### `end_to_end_nested_valid`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| serde_json alone | 1.0.154 | 7.45 µs | 7.44 µs – 7.45 µs | floor | 118.00 | 17114 | 0 |
| annotated (json::from_str) | git f881d35 | 8.27 µs | 8.26 µs – 8.27 µs | 1.02× | 118.00 | 17114 | 0 |
| annotated (serde_json, then validate) | git f881d35 | 8.08 µs | 8.07 µs – 8.08 µs | 1.00× | 118.00 | 17114 | 0 |
| validator | 0.21.0 | 8.22 µs | 8.21 µs – 8.22 µs | 1.02× | 119.00 | 17358 | 0 |
| garde | 0.23.0 | 8.73 µs | 8.72 µs – 8.73 µs | 1.08× | 118.00 | 17114 | 0 |
| wary | 0.3.2 | 10.78 µs | 10.78 µs – 10.79 µs | 1.34× | 220.00 | 21194 | 0 |
| serde_valid (graphemes) | 3.2.0 | 24.77 µs | 24.77 µs – 24.78 µs | 3.07× | 118.00 | 17114 | 0 |

### `end_to_end_nested_5pct_invalid`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| serde_json alone | 1.0.154 | 7.44 µs | 7.43 µs – 7.45 µs | floor | 118.00 | 17114 | 0 |
| annotated (json::from_str) | git f881d35 | 8.50 µs | 8.49 µs – 8.51 µs | 1.02× | 120.00 | 19226 | 5 |
| annotated (serde_json, then validate) | git f881d35 | 8.33 µs | 8.33 µs – 8.33 µs | 1.00× | 120.00 | 19226 | 5 |
| validator | 0.21.0 | 9.05 µs | 9.04 µs – 9.05 µs | 1.09× | 141.00 | 22394 | 5 |
| garde | 0.23.0 | 9.47 µs | 9.46 µs – 9.47 µs | 1.14× | 136.00 | 18320 | 5 |
| wary | 0.3.2 | 11.14 µs | 11.13 µs – 11.14 µs | 1.34× | 227.00 | 22066 | 5 |
| serde_valid (graphemes) | 3.2.0 | 25.89 µs | 25.89 µs – 25.90 µs | 3.11× | 157.00 | 24150 | 5 |

### `end_to_end_map_nested_valid`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| serde_json alone | 1.0.154 | 10.41 µs | 10.40 µs – 10.41 µs | floor | 217.00 | 13122 | 0 |
| annotated (json::from_str) | git f881d35 | 10.96 µs | 10.95 µs – 10.96 µs | 1.05× | 217.00 | 13122 | 0 |
| annotated (serde_json, then validate) | git f881d35 | 10.41 µs | 10.41 µs – 10.42 µs | 1.00× | 217.00 | 13122 | 0 |
| validator | 0.21.0 | 11.10 µs | 11.09 µs – 11.10 µs | 1.07× | 218.00 | 13366 | 0 |
| garde | 0.23.0 | 11.39 µs | 11.38 µs – 11.40 µs | 1.09× | 217.00 | 13122 | 0 |
| serde_valid (graphemes) | 3.2.0 | 25.61 µs | 25.60 µs – 25.62 µs | 2.46× | 217.00 | 13122 | 0 |

### `end_to_end_map_nested_5pct_invalid`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| serde_json alone | 1.0.154 | 10.46 µs | 10.46 µs – 10.47 µs | floor | 217.00 | 13122 | 0 |
| annotated (json::from_str) | git f881d35 | 11.88 µs | 11.87 µs – 11.90 µs | 1.07× | 229.00 | 19099 | 5 |
| annotated (serde_json, then validate) | git f881d35 | 11.06 µs | 11.05 µs – 11.06 µs | 1.00× | 224.00 | 15299 | 5 |
| validator | 0.21.0 | 11.68 µs | 11.68 µs – 11.69 µs | 1.06× | 240.00 | 18402 | 5 |
| garde | 0.23.0 | 12.10 µs | 12.10 µs – 12.10 µs | 1.09× | 235.00 | 14348 | 5 |
| serde_valid (graphemes) | 3.2.0 | 26.84 µs | 26.83 µs – 26.84 µs | 2.43× | 261.00 | 20383 | 5 |

### `end_to_end_missing_field`

Not like for like: `json::from_str` reports all three problems; every other case answers serde's error alone, counted as one. See [Collecting deserialization](#collecting-deserialization).

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| serde_json alone | 1.0.154 | 233.6 ns | 233.3 ns – 234.0 ns | floor | 9.00 | 273 | 1 |
| annotated (json::from_str) | git f881d35 | 305.2 ns | 304.8 ns – 305.5 ns | 1.32× | 6.00 | 839 | 3 |
| annotated (serde_json, then validate) | git f881d35 | 237.0 ns | 236.6 ns – 237.3 ns | 1.02× | 9.00 | 273 | 1 |
| validator | 0.21.0 | 231.6 ns | 231.4 ns – 231.8 ns | 1.00× | 9.00 | 273 | 1 |
| garde | 0.23.0 | 235.3 ns | 235.0 ns – 235.6 ns | 1.02× | 9.00 | 273 | 1 |
| wary | 0.3.2 | 240.7 ns | 240.5 ns – 241.0 ns | 1.04× | 9.00 | 273 | 1 |

## Experiments without LTO

### `experiment_inline_checks_order`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| annotated (derived) | git f881d35 | 517.9 ns | 517.6 ns – 518.6 ns | 1.00× | 0.00 | 0 | 0 |
| hand-written (inline checks, violations recorded out of line) | on annotated git f881d35 | 551.5 ns | 551.2 ns – 551.9 ns | 1.06× | 0.00 | 0 | 0 |

### `experiment_inline_checks_flat`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| annotated (derived) | git f881d35 | 101.6 ns | 101.4 ns – 101.8 ns | 1.00× | 0.00 | 0 | 0 |
| hand-written (inline checks, violations recorded out of line) | on annotated git f881d35 | 102.1 ns | 101.9 ns – 102.5 ns | 1.00× | 0.00 | 0 | 0 |

### `experiment_inline_checks_flat_every_field_invalid`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| annotated (derived) | git f881d35 | 190.0 ns | 189.9 ns – 190.4 ns | 1.00× | 2.00 | 2112 | 8 |
| hand-written (inline checks, violations recorded out of line) | on annotated git f881d35 | 192.4 ns | 192.2 ns – 192.6 ns | 1.01× | 2.00 | 2112 | 8 |

### `experiment_url`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| annotated (rules::url()) | git f881d35 | 43.7 ns | 43.6 ns – 43.8 ns | 1.00× | 0.00 | 0 | 0 |
| hand-written (url::Url::parse alone, as garde and validator check) | on annotated git f881d35 | 105.5 ns | 105.2 ns – 105.8 ns | 2.41× | 1.00 | 24 | 0 |

### `experiment_length_short_text`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| annotated (rules::length(), chars counted) | git f881d35 | 2.4 ns | 2.4 ns – 2.5 ns | 1.00× | 0.00 | 0 | 0 |
| hand-written (settled by the byte length first) | on annotated git f881d35 | 2.7 ns | 2.7 ns – 2.8 ns | 1.12× | 0.00 | 0 | 0 |

### `experiment_length_64kb`

| Library | Version | Median | 95% CI of the median | vs fastest | Allocations | Bytes | Violations |
|---|---|--:|---|--:|--:|--:|--:|
| annotated (rules::length(), chars counted) | git f881d35 | 2.4 ns | 2.4 ns – 2.4 ns | 1.00× | 0.00 | 0 | 0 |
| hand-written (settled by the byte length first) | on annotated git f881d35 | 2.6 ns | 2.6 ns – 2.6 ns | 1.08× | 0.00 | 0 | 0 |

## Where the libraries mean different things

The scenarios give every library the same rules, in its own syntax. Where the libraries still mean different things by them, here is how, and what it does to the numbers.

1. **One failure per field, or every failure.** annotated stops checking a field at the first rule it breaks; `#[validate(report_all)]` makes it check them all. validator, garde and wary check every rule of a field. Only `username` in `flat_every_field_invalid` breaks two rules, so annotated reports 8 violations there by default and 9 with `report_all`, like the others. Both are measured: `report_all` is the like-for-like comparison, and the default is what annotated users get.
2. **What a length counts.** annotated counts a string's chars by default, and bytes or graphemes when asked. garde counts bytes by default; the scenarios ask it for `chars`. validator counts only chars. wary is asked for `chars`. serde_valid counts only graphemes, so its rows count something slower than everyone else's: they're labelled `(graphemes)`, and in the long-string scenario they're comparable only with the other `(graphemes)` rows. annotated settles a length by the byte length when that decides it, and counts otherwise; the answer is the one counting gives, so it means the same as the others.
3. **What an email address is.** annotated checks the HTML form rule by hand: ASCII local part and domain labels, at most 254 bytes. validator applies the same rule with two regular expressions, retries a non-ASCII domain through IDNA, and accepts an IP literal as the domain. garde applies it with regular expressions, without IDNA unless built with `email-idna`. wary parses RFC 5322 with the `email_address` crate, which accepts more, like quoted local parts. Every library accepts `alex.morgan@example.com` and refuses `alex.morgan.example.com`.
4. **What a URL is.** annotated accepts an absolute URI with an authority by RFC 3986, with RFC 3987's international characters, a host that isn't empty and no userinfo, of the scheme `http`, `https` or `ftp` in any case. It recognizes that by hand, without parsing into a URL. validator, garde and wary accept anything `url::Url::parse` accepts, by the WHATWG URL Standard, of any scheme. The definitions differ at the edges: annotated accepts `http://256.0.0.1` (a registered name to the RFC, a broken IPv4 address to WHATWG) and ports above 65535, and refuses URIs without an authority, like `mailto:alex@example.com`, which `url` parses. Every library accepts `https://example.com/alex` and refuses `example dot com`.
5. **`required`.** Every library is given `required` only on an `Option`, where it means `Some` in all of them. annotated's `required` on a string, which also refuses blank text, isn't measured, because no other library has it.
6. **Patterns.** Every library compiles each regular expression once into a static and matches with the same `regex` build. annotated and garde make the static from the literal. validator is given one.
7. **What a violation holds.** The libraries build different reports, and that is part of what's measured. annotated records an owned path of serde names, kept in place for up to three segments, and a `Problem` with its parameters: 176 bytes a violation. validator builds a `HashMap` per struct, and stores each failing field's value in the error as a `serde_json::Value`. garde stores a path and a formatted message per violation. wary links its paths through `Arc`s.
8. **Maps.** wary goes through no map, so it isn't in the map scenarios. Only annotated checks a map's values with rules of their own (`each(length(...))`), so `map_string_values_valid` has annotated alone, as a reference for its own progress. The maps are `BTreeMap`s, so every run goes through them in the same order.
9. **A wary bug seen on the way.** wary 0.3.2 reports the missing `fullName` under the path `username`. It does the same work, so its numbers are still comparable.
10. **End to end, annotated reads two ways.** `serde_json, then validate` deserializes with serde's derive and validates afterwards, as every other library does. `json::from_str` is annotated's collecting deserialization: it checks each field as it reads it, and would report a missing field beside the broken rules. On a complete, well-typed body the two find the same violations in the same order, so they compare like for like with each other and with the other libraries. `json::from_str_fail_fast` and `validate_fail_fast` answer only the first violation: compare them with each other, not with the cases answering every one. `collect_type_errors` reads each field into a buffer before its type, to report a value of the wrong type as one more violation; on a body without one it finds the same.
11. **`end_to_end_missing_field` isn't like for like.** The body lacks `postalCode` and breaks the email and the age. `json::from_str` reads the whole body, checks every field that came and answers 3 violations. serde's derive stops at the missing field once it has read the body, so with `serde_json::from_str` no library gets to validate: each answers serde's error alone, counted as one problem, and the client learns about the email and the age only on later requests. The timings show what each answer costs, not which library validates faster.

## Where annotated still loses

- **Bytes on the invalid paths.** annotated makes the fewest allocations whenever something is invalid, but not the fewest bytes. One violation takes 704 bytes, the report's first buffer of four 176-byte violations, against wary's 288 for its 3 allocations. Five invalid lines take 2,112 bytes against garde's 1,206. The time it costs is small: annotated is still ahead by 2× there.
- **The byte-length check without LTO**, 3.4 ns against wary's 2.7 ns. That is `validate()`'s fixed cost: `FoundViolations::into_result` isn't inlined across the crate boundary. With LTO annotated is ahead, 1.3 against 2.0 ns.
- **Two end-to-end cases without LTO.** Read with `json::from_str`, the valid order takes 8.27 µs against validator's 8.22, and the warehouse with 5 invalid entries 11.88 µs against validator's 11.69. annotated's own `serde_json` then `validate()` is ahead of validator in both, 8.08 and 11.06 µs. With LTO `json::from_str` leads both scenarios, the order only by 0.4% over garde.
- **`json::from_str` behind annotated's own `serde_json` then `validate()` on nested bodies**: 1–4% on the valid order and warehouse with LTO, 2–5% without, and 4.7% / 7.4% on the warehouse with 5 invalid entries. [Collecting deserialization](#collecting-deserialization) has the numbers, and item 9's *Open* what's left.

## Optimisation opportunities in annotated

The seven changes the `b58e76b` run ranked, in its order, with what became of each, then the two that came out of measuring collecting deserialization in this run. Measurements are medians before → after, with LTO / without: from the commit bodies for the changes made, and from trial runs for those set aside.

### 1. Check every rule inline, and record violations in a cold function: done in `8790b68`

The derive matches on each rule's check where it is written, and a broken rule calls the cold `__private::record_violation`, which builds the field's location only then. `nested_valid` 429.0 → 357.1 ns / 1.03 µs → 580.8 ns; `flat_valid` 214.1 → 202.8 ns / 251.5 → 239.1 ns. The derived code now matches the hand-written experiment.

### 2. Recognise a URL without building a `url::Url`: done differently, in `99972e3`

Instead of a fast path in front of the `url` crate, the owner chose to define a URL by RFC 3986 and RFC 3987 and drop the crate. The recognizer is written by hand after the grammar's productions, allocates nothing, and a property test checks it against `iri-string`, a strict RFC validator. `experiment_url` 122.0 → 39.3 ns / 133.8 → 44.3 ns; `flat_valid` 199.4 → 97.7 ns / 212.7 → 107.1 ns, with no allocations. What the rule accepts changed at the edges; item 4 of [Where the libraries mean different things](#where-the-libraries-mean-different-things) says how.

### 3. Settle a string's length by its byte length before counting: done in `7ec9d3f`

`long_string_64kb` in chars 1.79 µs → 2.51 ns / 5.29 ns, in graphemes 598.5 µs → 2.06 ns / 3.28 ns; `map_string_values_valid` 475.0 → 114.9 ns. A text too long by its bytes alone is refused without counting too. `HasLength` gained a `#[doc(hidden)]` provided method for it, which defaults to the exact length.

### 4. Make a violation smaller: measured, shapes kept

Three candidates, measured in alternating runs of the same binary layout:

- **`ParamValue::Integer` as an `i64`** brings `Problem` from 80 to 64 bytes and a violation from 176 to 160, saving 64 bytes per report slot. It ran 4–7% slower on `flat_every_field_invalid` and 2% slower on `flat_one_field_invalid`, and integers beyond an `i64` would become text.
- **Boxing `Range`'s bounds** gives the same size, but costs an allocation per range violation: `nested_5pct_invalid` +26%, `flat_every_field_invalid` with `report_all` +17%.
- **Going below 64 bytes** needs `Custom` boxed too: an allocation per custom violation, and a different way to match it.

None made validation faster, so `Problem` and `Violation` keep their shapes for now.

### 5. Build a path without allocating: done in `cbd7a6a`

A `Path` holds up to three segments in place and moves to a `Vec` only from the fourth. `flat_every_field_invalid` 190.6 → 151.9 ns, 11 → 3 allocations; `nested_5pct_invalid` 491.3 → 433.3 ns, 7 → 2 allocations. A violation grew from 128 to 176 bytes.

### 6. Size the report once: dropped

Reserving room for nine violations at the first one made `report_all`'s every-field-invalid case 18% faster (198.0 → 162.7 ns), but `flat_one_field_invalid` 14% slower (184.1 → 209.3 ns) and `nested_5pct_invalid` 6% slower. A first buffer of 9 × 176 = 1,584 bytes is past what glibc's thread cache serves (1,032 bytes), so the most common failure, a single field, paid for the rare one.

### 7. Inline `validate()`'s fixed cost: dropped

`#[inline]` on `FoundViolations::into_result` measured no consistent gain: the byte-length check without LTO went from 3.84 to 3.62 ns, but `nested_5pct_invalid` went 10% the other way and the rest moved within noise.

### 8. Inline the range check where the rule is written: done in `29a23e2`

Collecting deserialization checks a struct's fields inside its serde visitor, a function far larger than `validate_into`, and there LLVM left `Range::check` a call answering its `Result<(), Problem>` through memory, where `validate_into` inlined it. With `#[inline]` the visitor inlines it too, as `StringLength::check` already was. Reading the order with `json::from_str`: 216,152 → 214,448 instructions with LTO, 222,623 → 221,723 without; medians 7.56 → 7.54 µs / 8.39 → 8.14 µs. `validate()` runs the same instructions with LTO, and an aligned build times it the same both ways.

### 9. Read a map's values into the map's own report: done in `f881d35`

A `BTreeMap` read each value into a report of its own, so a value without violations paid for starting one, dropping it out of line and looking its key up among the values set aside, and the visitor grew too large for LLVM to inline the map's `insert`. Each value is now read into the map's report, and only a value that adds violations there, or couldn't be built, has them taken out and set aside. The warehouse: 251,005 → 247,807 instructions valid, 268,425 → 261,176 with 5 entries invalid, with LTO; medians 10.29 → 9.59 µs and 11.33 → 10.32 µs with LTO, 10.77 → 10.82 µs and 11.95 → 11.84 µs without. Against `serde_json` then `validate()` in the same runs, the invalid warehouse went from 15% to 7.5% slower with LTO.

### Open

- **Valid nested bodies read with `json::from_str`**, 1–4% behind `serde_json` then `validate()` with LTO: about 43 instructions a line on the order. About 11 of them are the element's location, which paths need. The rest go to the visitor carrying its location and report and to the `Option`s a value that couldn't be built travels in; and since `serde_json`'s `deserialize_struct` inlines both `visit_map` and `visit_seq`, every check is emitted twice, which makes the reader large enough that LLVM leaves the counting path of a string's length out of line. Not tried: emitting the checks once and calling them out of line from `visit_seq`, which would cost formats that only read sequences, like binary ones. Tried and dropped: `#[inline]` on `HasLength` for strings (the call moved to `StringUnit::count`, and `validate()` ran 1,800 more instructions without LTO), a cold builder for `Range`'s problem (`validate()` ran 700 more with LTO), and the map's setting aside out of line, or its values joined in bulk (no gain, or a loss).
- **A map's values with violations** are still read into a report each and kept in a second map until the end: 5 allocations and 3,800 bytes more than `validate()` on the warehouse with 5 invalid entries. `HashMap`'s visitor could take item 9's change; no scenario measures it yet.
- **Fewer bytes per violation.** Of a violation's 176 bytes, 72 are the in-place path, 80 the `Problem`, 24 the message override. A smaller report needs a cheaper shape for one of them that doesn't cost an allocation or time; item 4's candidates didn't find one.
- **The byte-length check without LTO**, item 7's 0.5 ns, if a way to inline `validate()`'s tail measures as a gain.
