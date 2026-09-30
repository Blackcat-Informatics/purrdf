// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The bench harness, checked from outside.
//!
//! * The statistics against hand-computed values: type 7 quantiles, the
//!   median, the MAD, Tukey's fences at and beyond their boundaries, and
//!   bootstrap intervals whose distribution is known exactly; a seeded
//!   bootstrap gives one interval.
//! * Every command-line option, and every refusal beside a valid neighbour.
//! * The estimates file: the exact text written, a bit-exact round trip
//!   through the reader, and each refused file beside the accepted one.
//! * Whole runs in-process: `--test` runs each routine once, `--list` runs
//!   none, a measured run writes and compares records natively, and on wasm32
//!   says it writes nothing.
//!
//! The suite is `harness = false` on the testkit runner, so `make wasm-test`
//! runs it on wasm32; the cases that need a file system, or a panic that
//! unwinds, are native only.

use std::cell::Cell;
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use purrdf_testkit::bench::Comparison;
use purrdf_testkit::bench::stats::{self, Interval, Outliers};
use purrdf_testkit::bench::store::path_component;
use purrdf_testkit::bench::{
    Arguments, BatchSize, Bench, BenchmarkId, Estimates, Mode, Throughput, Verdict, black_box,
    format_rate, format_time,
};

// ---- statistics -----------------------------------------------------------

fn type_seven_quantiles_interpolate_between_order_statistics() {
    let sorted = [1.0, 2.0, 3.0, 4.0];
    assert_eq!(stats::quantile(&sorted, 0.0), 1.0);
    assert_eq!(stats::quantile(&sorted, 0.25), 1.75);
    assert_eq!(stats::quantile(&sorted, 0.5), 2.5);
    assert_eq!(stats::quantile(&sorted, 0.75), 3.25);
    assert_eq!(stats::quantile(&sorted, 1.0), 4.0);
    assert_eq!(stats::quantile(&[7.0], 0.9), 7.0);
}

fn the_median_is_the_middle_value_or_the_mean_of_the_middle_two() {
    assert_eq!(stats::median(&[5.0, 1.0, 3.0]), 3.0);
    assert_eq!(stats::median(&[4.0, 1.0, 3.0, 2.0]), 2.5);
    assert_eq!(stats::median(&[9.0]), 9.0);
}

fn the_mad_is_the_median_absolute_deviation_from_the_median() {
    // Deviations from 3 are 2, 1, 0, 1, 97; their median is 1.
    let values = [1.0, 2.0, 3.0, 4.0, 100.0];
    let median = stats::median(&values);
    assert_eq!(median, 3.0);
    assert_eq!(stats::median_absolute_deviation(&values, median), 1.0);
    // Deviations from 2.5 are 1.5, 0.5, 0.5, 1.5; their median is 1.
    assert_eq!(
        stats::median_absolute_deviation(&[1.0, 2.0, 3.0, 4.0], 2.5),
        1.0
    );
}

fn tukey_fences_classify_by_side_and_severity() {
    // Q1 = 2.75 and Q3 = 8.25 (type 7), so IQR = 5.5: the mild fences are
    // -5.5 and 16.5, the severe fences -13.75 and 24.75.
    let values = [
        -20.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 20.0, 40.0,
    ];
    assert_eq!(
        stats::classify_outliers(&values),
        Outliers {
            low_severe: 1,
            low_mild: 0,
            high_mild: 1,
            high_severe: 1,
        }
    );
    // A value exactly on a fence is inside it: 16.5 and 24.75 are not beyond.
    let on_fences = [
        -20.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 16.5, 40.0,
    ];
    assert_eq!(
        stats::classify_outliers(&on_fences),
        Outliers {
            low_severe: 1,
            low_mild: 0,
            high_mild: 0,
            high_severe: 1,
        }
    );
    assert_eq!(stats::classify_outliers(&[3.0; 10]).total(), 0);
}

fn a_bootstrap_of_identical_samples_is_a_point() {
    let interval = stats::bootstrap_median(&[5.0; 10], 1000, 0.95, 7);
    assert_eq!(
        interval,
        Interval {
            low: 5.0,
            high: 5.0
        }
    );
}

fn a_bootstrap_of_two_values_spans_both() {
    // A resample of [1, 2] has median 1 with probability 1/4, 1.5 with 1/2
    // and 2 with 1/4, so the 2.5% and 97.5% points of 10 000 resamples are 1
    // and 2.
    let interval = stats::bootstrap_median(&[1.0, 2.0], stats::RESAMPLES, 0.95, 11);
    assert_eq!(
        interval,
        Interval {
            low: 1.0,
            high: 2.0
        }
    );
}

fn a_seeded_bootstrap_is_reproducible() {
    let samples: Vec<f64> = (0..50_u32)
        .map(|index| f64::from((index * 37) % 23) + 100.0)
        .collect();
    let first = stats::bootstrap_median(&samples, stats::RESAMPLES, 0.95, 42);
    let second = stats::bootstrap_median(&samples, stats::RESAMPLES, 0.95, 42);
    assert_eq!(first, second);
    let median = stats::median(&samples);
    assert!(
        first.low <= median && median <= first.high,
        "{first:?} misses {median}"
    );
    assert!(
        first.low >= 100.0 && first.high <= 122.0,
        "{first:?} leaves the data's range"
    );
}

fn a_change_between_constant_samples_is_exact() {
    let (point, interval) = stats::bootstrap_change(&[4.0; 12], &[2.0; 12], 500, 0.95, 3);
    assert_eq!(point, 1.0);
    assert_eq!(
        interval,
        Interval {
            low: 1.0,
            high: 1.0
        }
    );
    let (point, interval) = stats::bootstrap_change(&[2.0; 12], &[2.0; 12], 500, 0.95, 3);
    assert_eq!(point, 0.0);
    assert_eq!(
        interval,
        Interval {
            low: 0.0,
            high: 0.0
        }
    );
}

fn verdicts_read_the_interval_against_the_noise_band() {
    assert_eq!(Verdict::of(0.02, 0.05), Verdict::Regressed);
    assert_eq!(Verdict::of(-0.05, -0.02), Verdict::Improved);
    assert_eq!(Verdict::of(0.005, 0.03), Verdict::WithinNoise);
    assert_eq!(Verdict::of(-0.03, -0.005), Verdict::WithinNoise);
    assert_eq!(Verdict::of(-0.02, 0.02), Verdict::NoChange);
    // The band's edge itself is noise: a regression is strictly above it.
    assert_eq!(Verdict::of(0.01, 0.05), Verdict::WithinNoise);
    assert_eq!(Verdict::Regressed.sentence(), "Performance has regressed.");
}

fn times_and_rates_print_with_four_significant_digits() {
    assert_eq!(format_time(0.5), "500.0 ps");
    assert_eq!(format_time(52.3), "52.30 ns");
    assert_eq!(format_time(1_234.5), "1.234 µs");
    assert_eq!(format_time(2_500_000.0), "2.500 ms");
    assert_eq!(format_time(3e9), "3.000 s");
    assert_eq!(format_rate(Throughput::Bytes(1), 2048.0), "2.000 KiB/s");
    assert_eq!(
        format_rate(Throughput::Elements(1), 1_500_000.0),
        "1.500 Melem/s"
    );
}

// ---- the command line -----------------------------------------------------

fn parse(args: &[&str]) -> Arguments {
    Arguments::parse(args.iter().copied()).unwrap_or_else(|error| panic!("{args:?}: {error}"))
}

fn refused(args: &[&str]) -> String {
    match Arguments::parse(args.iter().copied()) {
        Ok(parsed) => panic!("{args:?} was accepted as {parsed:?}"),
        Err(error) => error.to_string(),
    }
}

fn the_mode_follows_bench_test_list_and_help() {
    assert_eq!(parse(&["--bench"]).mode, Mode::Benchmark);
    assert_eq!(parse(&[]).mode, Mode::Test);
    assert_eq!(parse(&["--test"]).mode, Mode::Test);
    assert_eq!(parse(&["--bench", "--test"]).mode, Mode::Test);
    assert_eq!(parse(&["--bench", "--list"]).mode, Mode::List);
    assert_eq!(parse(&["--bench", "--help"]).mode, Mode::Help);
    assert_eq!(parse(&["-h"]).mode, Mode::Help);
}

fn filters_exact_quick_and_noplot_parse() {
    let parsed = parse(&[
        "--bench", "blake3", "--exact", "--quick", "--noplot", "--", "--odd",
    ]);
    assert_eq!(parsed.filters, ["blake3", "--odd"]);
    assert!(parsed.exact && parsed.quick);
    assert!(parsed.selects("blake3") && !parsed.selects("blake3/one-shot/64"));
    let substring = parse(&["--bench", "one-shot"]);
    assert!(substring.selects("blake3/one-shot/64") && !substring.selects("hex/32B"));
    assert!(parse(&["--bench"]).selects("anything"));
}

#[cfg(not(target_arch = "wasm32"))]
fn baseline_options_take_a_name_in_either_spelling() {
    assert_eq!(
        parse(&["--bench", "--save-baseline", "base"])
            .save_baseline
            .as_deref(),
        Some("base")
    );
    assert_eq!(
        parse(&["--bench", "--save-baseline=a.b-c_1"])
            .save_baseline
            .as_deref(),
        Some("a.b-c_1")
    );
    assert_eq!(
        parse(&["--bench", "--baseline", "base"]).baseline,
        Some(Comparison::Strict("base".to_owned()))
    );
    assert_eq!(
        parse(&["--bench", "--baseline-lenient=base"]).baseline,
        Some(Comparison::Lenient("base".to_owned()))
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn a_malformed_or_reserved_baseline_name_is_refused() {
    assert!(refused(&["--save-baseline", "../up"]).contains("baseline name `../up`"));
    assert!(refused(&["--save-baseline", ".hidden"]).contains("not starting with `.`"));
    assert!(refused(&["--baseline", "new"]).contains("latest run's record"));
    assert!(refused(&["--baseline"]).contains("requires a baseline name"));
    // The neighbours: a dotted name that does not start with a dot, and a
    // name that merely contains `new`.
    assert_eq!(
        parse(&["--save-baseline", "v1.2"]).save_baseline.as_deref(),
        Some("v1.2")
    );
    assert_eq!(
        parse(&["--save-baseline", "newer"])
            .save_baseline
            .as_deref(),
        Some("newer")
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn saving_and_comparing_in_one_run_is_refused() {
    let message = refused(&["--save-baseline", "a", "--baseline", "b"]);
    assert!(message.contains("cannot be combined"), "{message}");
    assert!(refused(&["--baseline", "a", "--baseline-lenient", "b"]).contains("more than once"));
    assert!(refused(&["--save-baseline", "a", "--save-baseline", "b"]).contains("given twice"));
    // Each alone is accepted.
    parse(&["--save-baseline", "a"]);
    parse(&["--baseline", "b"]);
    parse(&["--baseline-lenient", "b"]);
}

#[cfg(target_arch = "wasm32")]
fn the_store_options_are_refused_on_wasm32() {
    for option in ["--save-baseline", "--baseline", "--baseline-lenient"] {
        let message = refused(&["--bench", option, "base"]);
        assert!(
            message.contains(option) && message.contains("wasm32-unknown-unknown has none"),
            "{message}"
        );
    }
    // The neighbour: a measured run with no store option.
    assert_eq!(parse(&["--bench", "--quick"]).mode, Mode::Benchmark);
}

fn an_unknown_option_is_refused_by_name() {
    assert_eq!(
        refused(&["--frobnicate"]),
        "unrecognized option `--frobnicate`"
    );
    assert_eq!(refused(&["-s", "base"]), "unrecognized option `-s`");
    assert_eq!(
        refused(&["--bench=yes"]),
        "option `--bench` does not take a value"
    );
    assert!(refused(&["--list", "--test"]).contains("cannot be combined"));
    // The neighbours, spelled as they are known.
    assert_eq!(parse(&["--bench"]).mode, Mode::Benchmark);
    assert_eq!(parse(&["--list"]).mode, Mode::List);
}

// ---- the estimates file ---------------------------------------------------

fn sample_estimates(throughput: Option<Throughput>) -> Estimates {
    Estimates {
        median: 52.3,
        mad: 0.1,
        ci_low: 51.999_999_999_999_99,
        ci_high: 1e-7,
        confidence: 0.95,
        resamples: 10_000,
        iterations_per_sample: 1_000_000,
        samples: vec![52.3, 0.1, 123_456.789, 1e21],
        outliers: Outliers {
            low_severe: 0,
            low_mild: 1,
            high_mild: 2,
            high_severe: 3,
        },
        throughput,
    }
}

const SAMPLE_JSON: &str = r#"{
  "schema": 1,
  "unit": "ns",
  "median": 52.3,
  "mad": 0.1,
  "ci_low": 51.99999999999999,
  "ci_high": 0.0000001,
  "confidence": 0.95,
  "resamples": 10000,
  "iterations_per_sample": 1000000,
  "samples": [52.3, 0.1, 123456.789, 1000000000000000000000],
  "outliers": {"low_severe": 0, "low_mild": 1, "high_mild": 2, "high_severe": 3},
  "throughput": {"kind": "bytes", "per_iteration": 4096}
}
"#;

fn the_estimates_file_has_its_documented_text() {
    let written = sample_estimates(Some(Throughput::Bytes(4096)))
        .to_json()
        .expect("finite estimates");
    assert_eq!(written, SAMPLE_JSON);
}

fn the_estimates_file_round_trips_bit_for_bit() {
    for throughput in [
        None,
        Some(Throughput::Bytes(4096)),
        Some(Throughput::Elements(7)),
    ] {
        let estimates = sample_estimates(throughput);
        let text = estimates.to_json().expect("finite estimates");
        let read = Estimates::from_json(&text).expect("its own text");
        assert_eq!(read, estimates);
        for (read, written) in read.samples.iter().zip(&estimates.samples) {
            assert_eq!(read.to_bits(), written.to_bits());
        }
    }
    // Members in another order, with other whitespace, read the same.
    let reordered = SAMPLE_JSON
        .replace("\"schema\": 1,\n", "")
        .replace("{\n", "{\"schema\":1,");
    assert_eq!(
        Estimates::from_json(&reordered).expect("reordered"),
        sample_estimates(Some(Throughput::Bytes(4096)))
    );
}

fn a_malformed_estimates_file_is_refused_by_what_is_wrong() {
    let cases = [
        (
            SAMPLE_JSON.replace("\"mad\"", "\"spread\""),
            "unknown member `spread`",
        ),
        (
            SAMPLE_JSON.replace("  \"mad\": 0.1,\n", ""),
            "member `mad` is missing",
        ),
        (
            SAMPLE_JSON.replace("\"mad\": 0.1,", "\"mad\": 0.1, \"mad\": 0.1,"),
            "member `mad` given twice",
        ),
        (
            SAMPLE_JSON.replace("\"schema\": 1", "\"schema\": 2"),
            "schema 2",
        ),
        (SAMPLE_JSON.replace("\"ns\"", "\"ms\""), "unit `ms`"),
        (SAMPLE_JSON.replace("\"ns\"", "\"n\\s\""), "escape"),
        (SAMPLE_JSON.replace("52.3,", "052.3,"), "malformed number"),
        (
            SAMPLE_JSON.replace("\"bytes\"", "\"bits\""),
            "throughput kind `bits`",
        ),
        (
            format!("{SAMPLE_JSON}{{}}"),
            "text after the estimates object",
        ),
        (
            SAMPLE_JSON.replace("[52.3, 0.1, 123456.789, 1000000000000000000000]", "[]"),
            "`samples` is empty",
        ),
        (
            SAMPLE_JSON.replace("1000000,", "0,"),
            "`iterations_per_sample` is zero",
        ),
    ];
    for (text, expected) in cases {
        let message = Estimates::from_json(&text).expect_err(expected).to_string();
        assert!(message.contains(expected), "{message} lacks {expected}");
    }
    // The neighbour every case was edited from.
    Estimates::from_json(SAMPLE_JSON).expect("the documented text");
}

fn a_non_finite_estimate_is_refused_and_a_finite_one_written() {
    let mut estimates = sample_estimates(None);
    estimates.ci_high = f64::INFINITY;
    let message = estimates.to_json().expect_err("infinity").to_string();
    assert!(message.contains("`ci_high` is inf"), "{message}");
    estimates.ci_high = f64::MAX;
    let text = estimates.to_json().expect("the largest finite value");
    assert_eq!(
        Estimates::from_json(&text).expect("read back").ci_high,
        f64::MAX
    );
}

fn id_components_become_single_path_components() {
    assert_eq!(path_component("64B"), "64B");
    assert_eq!(path_component("stream/fresh/8"), "stream%2Ffresh%2F8");
    assert_eq!(path_component(".."), "%2E.");
    assert_eq!(path_component("a.b"), "a.b");
    assert_eq!(path_component("a b%"), "a%20b%25");
    assert_eq!(path_component("µ"), "%C2%B5");
}

// ---- whole runs -----------------------------------------------------------

fn tiny(arguments: &[&str]) -> Bench {
    Bench::with_arguments(parse(arguments))
        .sample_size(10)
        .warm_up_time(Duration::from_millis(2))
        .measurement_time(Duration::from_millis(20))
}

fn test_mode_runs_each_routine_exactly_once() {
    let calls = Cell::new(0_u32);
    let setups = Cell::new(0_u32);
    let mut bench = tiny(&["--test"]);
    bench.bench_function("single", |b| b.iter(|| calls.set(calls.get() + 1)));
    let mut group = bench.benchmark_group("group");
    group.bench_with_input(BenchmarkId::new("input", 3), &3_u32, |b, &n| {
        b.iter(|| calls.set(calls.get() + n));
    });
    group.bench_function(BenchmarkId::from_parameter("batched"), |b| {
        b.iter_batched(
            || setups.set(setups.get() + 1),
            |()| calls.set(calls.get() + 10),
            BatchSize::SmallInput,
        );
    });
    group.bench_function("batched_ref", |b| {
        b.iter_batched_ref(
            || {
                setups.set(setups.get() + 1);
                100_u32
            },
            |n| calls.set(calls.get() + *n),
            BatchSize::LargeInput,
        );
    });
    group.bench_function("large_drop", |b| {
        b.iter_with_large_drop(|| {
            calls.set(calls.get() + 1000);
            vec![0_u8; 16]
        });
    });
    group.finish();
    assert_eq!(calls.get(), 1 + 3 + 10 + 100 + 1000);
    assert_eq!(setups.get(), 2);
    assert_eq!(
        bench.console(),
        "Testing single\nSuccess\nTesting group/input/3\nSuccess\n\
         Testing group/batched\nSuccess\nTesting group/batched_ref\nSuccess\n\
         Testing group/large_drop\nSuccess\n"
    );
    assert!(
        bench
            .outcomes()
            .iter()
            .all(|outcome| outcome.failure.is_none())
    );
}

fn list_mode_names_the_selected_benchmarks_and_runs_none() {
    let calls = Cell::new(0_u32);
    let mut bench = tiny(&["--list", "keep"]);
    let mut group = bench.benchmark_group("g");
    group.bench_function("keep", |b| b.iter(|| calls.set(calls.get() + 1)));
    group.bench_function("drop", |b| b.iter(|| calls.set(calls.get() + 1)));
    group.finish();
    assert_eq!(calls.get(), 0);
    assert_eq!(bench.console(), "g/keep: benchmark\n");
}

fn a_measured_run_reports_its_estimates() {
    let setups = Cell::new(0_u64);
    let routines = Cell::new(0_u64);
    let bench = tiny(&["--bench"]);
    #[cfg(not(target_arch = "wasm32"))]
    let store = purrdf_testkit::temp_dir!().expect("store");
    #[cfg(not(target_arch = "wasm32"))]
    let bench = bench.with_store(store.path());
    let mut bench = bench;
    let mut group = bench.benchmark_group("measured");
    group.throughput(Throughput::Elements(64));
    group.bench_function("sum", |b| {
        b.iter(|| (0..black_box(64_u64)).sum::<u64>());
    });
    group.bench_function("batched", |b| {
        b.iter_batched(
            || setups.set(setups.get() + 1),
            |()| routines.set(routines.get() + 1),
            BatchSize::SmallInput,
        );
    });
    group.finish();
    let outcomes = bench.outcomes();
    assert_eq!(outcomes.len(), 2);
    for outcome in &outcomes {
        assert_eq!(outcome.failure, None, "{}", bench.console());
        let estimates = outcome.estimates.as_ref().expect("measured");
        assert_eq!(estimates.samples.len(), 10);
        assert!(estimates.ci_low <= estimates.median && estimates.median <= estimates.ci_high);
        assert!(
            outcome.change.is_none(),
            "a first run has nothing to compare against"
        );
    }
    assert_eq!(
        outcomes[0].estimates.as_ref().expect("measured").throughput,
        Some(Throughput::Elements(64))
    );
    // Every timed iteration had exactly one input set up outside the clock.
    assert_eq!(setups.get(), routines.get());
    let console = bench.console();
    assert!(
        console.contains("measured/sum") && console.contains("time:   ["),
        "{console}"
    );
    assert!(
        console.contains("thrpt:  [") && console.contains("elem/s"),
        "{console}"
    );
    #[cfg(not(target_arch = "wasm32"))]
    {
        let record = store.path().join("measured/sum/new/estimates.json");
        let text = std::fs::read_to_string(&record).expect("the latest record");
        assert_eq!(
            Estimates::from_json(&text).expect("a valid record"),
            *outcomes[0].estimates.as_ref().expect("measured")
        );
    }
    #[cfg(target_arch = "wasm32")]
    assert_eq!(
        console
            .matches("note: wasm32-unknown-unknown has no file system: estimates are printed, not written")
            .count(),
        1,
        "{console}"
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn run_once(store: &std::path::Path, arguments: &[&str]) -> Bench {
    let mut bench = tiny(arguments).with_store(store);
    bench.bench_function("compared", |b| {
        b.iter(|| black_box(3_u64).pow(black_box(7)));
    });
    bench
}

#[cfg(not(target_arch = "wasm32"))]
fn baselines_are_saved_and_compared() {
    let store = purrdf_testkit::temp_dir!().expect("store");
    // A strict comparison with no saved baseline fails, and measures nothing.
    let missing = run_once(store.path(), &["--bench", "--baseline", "base"]);
    let failure = missing.outcomes()[0].failure.clone().expect("no baseline");
    assert!(failure.contains("no baseline `base`"), "{failure}");
    assert!(missing.outcomes()[0].estimates.is_none());
    // The lenient form measures and records without a comparison.
    let lenient = run_once(store.path(), &["--bench", "--baseline-lenient", "base"]);
    assert_eq!(lenient.outcomes()[0].failure, None);
    assert!(lenient.outcomes()[0].change.is_none());
    // Saved, then compared strictly: the change carries its interval.
    let saved = run_once(store.path(), &["--bench", "--save-baseline", "base"]);
    assert_eq!(saved.outcomes()[0].failure, None);
    assert!(store.path().join("compared/base/estimates.json").is_file());
    let compared = run_once(store.path(), &["--bench", "--baseline", "base"]);
    let outcome = &compared.outcomes()[0];
    assert_eq!(outcome.failure, None, "{}", compared.console());
    let change = outcome.change.as_ref().expect("compared against base");
    assert_eq!(change.record, "base");
    assert!(change.ci_low <= change.point && change.point <= change.ci_high);
    assert!(
        compared.console().contains("change: ["),
        "{}",
        compared.console()
    );
    // A default run compares against the previous latest record.
    let again = run_once(store.path(), &["--bench"]);
    assert_eq!(
        again.outcomes()[0]
            .change
            .as_ref()
            .map(|change| change.record.as_str()),
        Some("new")
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn a_corrupt_record_fails_the_benchmark_rather_than_vanishing() {
    let store = purrdf_testkit::temp_dir!().expect("store");
    let record = store.path().join("compared/base/estimates.json");
    std::fs::create_dir_all(record.parent().expect("parent")).expect("record directory");
    std::fs::write(&record, "{}").expect("corrupt record");
    for option in ["--baseline", "--baseline-lenient"] {
        let bench = run_once(store.path(), &["--bench", option, "base"]);
        let failure = bench.outcomes()[0].failure.clone().expect("corrupt");
        assert!(failure.contains("is missing"), "{option}: {failure}");
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn a_failing_routine_fails_its_benchmark_and_the_rest_still_run() {
    let mut bench = tiny(&["--test"]);
    bench.bench_function("panics", |b| b.iter(|| panic!("routine broke")));
    bench.bench_function("never_times", |_| {});
    bench.bench_function("fine", |b| b.iter(|| 1));
    let outcomes = bench.outcomes();
    assert_eq!(outcomes[0].failure.as_deref(), Some("routine broke"));
    assert!(
        outcomes[1]
            .failure
            .as_deref()
            .is_some_and(|message| message.contains("must call one of Bencher's `iter` methods"))
    );
    assert_eq!(outcomes[2].failure, None);
}

#[cfg(not(target_arch = "wasm32"))]
fn a_duplicate_id_is_refused_and_distinct_ids_are_not() {
    let mut bench = tiny(&["--test"]);
    let mut group = bench.benchmark_group("g");
    group.bench_function(BenchmarkId::new("f", 1), |b| b.iter(|| 1));
    group.bench_function(BenchmarkId::new("f", 2), |b| b.iter(|| 2));
    let duplicate = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        group.bench_function(BenchmarkId::new("f", 1), |b| b.iter(|| 3));
    }));
    assert!(duplicate.is_err(), "a second `g/f/1` was accepted");
}

#[cfg(not(target_arch = "wasm32"))]
fn a_sample_size_below_ten_is_refused_and_ten_is_not() {
    let refused = std::panic::catch_unwind(|| Bench::default().sample_size(9));
    assert!(refused.is_err());
    let _ = Bench::default().sample_size(10);
    let zero = std::panic::catch_unwind(|| Bench::default().measurement_time(Duration::ZERO));
    assert!(zero.is_err());
    let _ = Bench::default().measurement_time(Duration::from_nanos(1));
}

purrdf_testkit::harness_main!(
    type_seven_quantiles_interpolate_between_order_statistics,
    the_median_is_the_middle_value_or_the_mean_of_the_middle_two,
    the_mad_is_the_median_absolute_deviation_from_the_median,
    tukey_fences_classify_by_side_and_severity,
    a_bootstrap_of_identical_samples_is_a_point,
    a_bootstrap_of_two_values_spans_both,
    a_seeded_bootstrap_is_reproducible,
    a_change_between_constant_samples_is_exact,
    verdicts_read_the_interval_against_the_noise_band,
    times_and_rates_print_with_four_significant_digits,
    the_mode_follows_bench_test_list_and_help,
    filters_exact_quick_and_noplot_parse,
    #[cfg(not(target_arch = "wasm32"))]
    baseline_options_take_a_name_in_either_spelling,
    #[cfg(not(target_arch = "wasm32"))]
    a_malformed_or_reserved_baseline_name_is_refused,
    #[cfg(not(target_arch = "wasm32"))]
    saving_and_comparing_in_one_run_is_refused,
    #[cfg(target_arch = "wasm32")]
    the_store_options_are_refused_on_wasm32,
    an_unknown_option_is_refused_by_name,
    the_estimates_file_has_its_documented_text,
    the_estimates_file_round_trips_bit_for_bit,
    a_malformed_estimates_file_is_refused_by_what_is_wrong,
    a_non_finite_estimate_is_refused_and_a_finite_one_written,
    id_components_become_single_path_components,
    test_mode_runs_each_routine_exactly_once,
    list_mode_names_the_selected_benchmarks_and_runs_none,
    a_measured_run_reports_its_estimates,
    #[cfg(not(target_arch = "wasm32"))]
    baselines_are_saved_and_compared,
    #[cfg(not(target_arch = "wasm32"))]
    a_corrupt_record_fails_the_benchmark_rather_than_vanishing,
    #[cfg(not(target_arch = "wasm32"))]
    a_failing_routine_fails_its_benchmark_and_the_rest_still_run,
    #[cfg(not(target_arch = "wasm32"))]
    a_duplicate_id_is_refused_and_distinct_ids_are_not,
    #[cfg(not(target_arch = "wasm32"))]
    a_sample_size_below_ten_is_refused_and_ten_is_not,
);
