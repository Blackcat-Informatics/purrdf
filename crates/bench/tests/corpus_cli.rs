// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration tests driving the BUILT `bench-corpus` binary
//! (`env!("CARGO_BIN_EXE_bench-corpus")`) — never the library.
//!
//! `crates/bench/src/lib.rs` already pins the pure generator (determinism and shard-stitch as
//! in-process function calls, the strict-N-Quads round trip, the golden digest). None of that
//! can observe a PROCESS boundary, an exit code, a file `--out` actually wrote, or the shipped
//! `--manifest` JSON contract — and it was exactly that gap that let three silent-success
//! `--out` bugs ship past authoring, review, and `make check`. This file drives the compiled
//! executable exactly the way an operator invokes it, so every assertion here pins the shipped
//! surface rather than a library call that merely resembles it.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use purrdf_bench::{CLASS_MIX_PER_MILLE, CORPUS_PROFILE_ID, CorpusSpec};

/// The path to the built `bench-corpus` binary.
const BENCH: &str = env!("CARGO_BIN_EXE_bench-corpus");

/// Runs `bench-corpus` with `args`, returning (exit code, stdout bytes, stderr text).
fn run(args: &[&str]) -> (i32, Vec<u8>, String) {
    let output = Command::new(BENCH)
        .args(args)
        .output()
        .expect("spawn bench-corpus");
    (
        output.status.code().expect("process exited normally"),
        output.stdout,
        String::from_utf8(output.stderr).expect("utf-8 stderr"),
    )
}

/// Runs `bench-corpus` with `args` from within `dir`, returning (exit code, stdout bytes,
/// stderr text). Used for the relative-path and leading-dash-filename cases, where the
/// argument's meaning depends on the current directory.
fn run_in(dir: &std::path::Path, args: &[&str]) -> (i32, Vec<u8>, String) {
    let output = Command::new(BENCH)
        .current_dir(dir)
        .args(args)
        .output()
        .expect("spawn bench-corpus");
    (
        output.status.code().expect("process exited normally"),
        output.stdout,
        String::from_utf8(output.stderr).expect("utf-8 stderr"),
    )
}

/// Runs a whole (unsharded) corpus and returns its stdout bytes.
fn whole(quads: u64, iris: u64, seed: u64) -> Vec<u8> {
    let (code, out, err) = run(&[
        "--quads",
        &quads.to_string(),
        "--iris",
        &iris.to_string(),
        "--seed",
        &seed.to_string(),
    ]);
    assert_eq!(code, 0, "the whole run must succeed; stderr:\n{err}");
    out
}

/// A monotonically increasing counter, so every call to [`unique_tag`] in this process is
/// distinct even across parallel test threads.
static UNIQUE: AtomicU64 = AtomicU64::new(0);

/// A filesystem-unique fragment: this process's id plus a monotonic counter. Parallel test
/// threads within one `cargo test` run (and separate runs, which get different process ids)
/// never collide on the same name.
fn unique_tag() -> String {
    format!(
        "{}-{}",
        std::process::id(),
        UNIQUE.fetch_add(1, Ordering::Relaxed)
    )
}

/// A path under the OS temp dir, unique per call and tagged with `test_name` so a leftover
/// file (if cleanup ever fails) is traceable to the test that made it.
fn unique_temp_path(test_name: &str, suffix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "purrdf-bench-corpus-{test_name}-{}.{suffix}",
        unique_tag()
    ))
}

// ---------------------------------------------------------------------------------------------
// 1. Cross-process determinism.
// ---------------------------------------------------------------------------------------------

#[test]
fn cross_process_stdout_is_byte_identical() {
    let args = ["--quads", "600", "--iris", "70", "--seed", "42"];
    let (code_a, out_a, err_a) = run(&args);
    let (code_b, out_b, err_b) = run(&args);
    assert_eq!(code_a, 0, "first invocation must succeed; stderr:\n{err_a}");
    assert_eq!(
        code_b, 0,
        "second invocation must succeed; stderr:\n{err_b}"
    );
    assert_eq!(
        out_a, out_b,
        "two SEPARATE process invocations with identical args must produce byte-identical stdout"
    );
}

// ---------------------------------------------------------------------------------------------
// 2. Shard stitching through the CLI.
// ---------------------------------------------------------------------------------------------

/// Runs every shard `0..shards` of `(quads, iris, seed)` through the CLI, asserting each
/// shard's emitted row count matches the library's `shard_range()` width for it EXACTLY (so
/// there is neither a gap nor an overlap at any boundary), and that concatenating all shards
/// reproduces the whole (unsharded) run byte for byte.
fn assert_shard_stitch_partitions_the_whole(quads: u64, iris: u64, seed: u64, shards: u64) {
    let whole_bytes = whole(quads, iris, seed);
    let mut stitched = Vec::new();
    let mut previous_end = 0u64;
    for shard in 0..shards {
        let spec = CorpusSpec::new(seed, quads, iris, shard, shards).expect("valid shard spec");
        let (start, end) = spec.shard_range();
        assert_eq!(
            start, previous_end,
            "shard {shard} of {shards} must start exactly where the previous shard ended (no gap)"
        );
        previous_end = end;

        let (code, out, err) = run(&[
            "--quads",
            &quads.to_string(),
            "--iris",
            &iris.to_string(),
            "--seed",
            &seed.to_string(),
            "--shard",
            &shard.to_string(),
            "--shards",
            &shards.to_string(),
        ]);
        assert_eq!(
            code, 0,
            "shard {shard} of {shards} must succeed; stderr:\n{err}"
        );
        let text = String::from_utf8(out).expect("utf-8 stdout");
        let row_count = u64::try_from(text.lines().count()).expect("row count fits u64");
        assert_eq!(
            row_count,
            end - start,
            "shard {shard} of {shards} must emit exactly its shard_range() width (no overlap)"
        );
        stitched.extend_from_slice(text.as_bytes());
    }
    assert_eq!(
        previous_end, quads,
        "the last shard boundary must land exactly at quads"
    );
    assert_eq!(
        stitched, whole_bytes,
        "concatenating every shard's output must equal one whole run, byte for byte"
    );
}

#[test]
fn shard_stitch_shards_one_partitions_the_whole() {
    assert_shard_stitch_partitions_the_whole(240, 40, 7, 1);
}

#[test]
fn shard_stitch_uneven_division_partitions_the_whole() {
    // 5000 / 7 does not divide evenly; the remainder must be absorbed entirely by the last
    // shard, and every other boundary must still land exactly on a multiple of the base size.
    assert_shard_stitch_partitions_the_whole(5_000, 300, 99, 7);
}

#[test]
fn shard_stitch_shards_outnumber_quads_partitions_the_whole() {
    assert_shard_stitch_partitions_the_whole(3, 10, 5, 8);
}

#[test]
fn shard_stitch_final_shard_equals_the_tail_of_the_whole() {
    let (quads, iris, seed, shards) = (5_000u64, 300u64, 99u64, 7u64);
    let whole_text = String::from_utf8(whole(quads, iris, seed)).expect("utf-8 stdout");
    // `split_inclusive` keeps each row's trailing '\n' attached, so slicing and concatenating
    // reproduces the original bytes exactly rather than reassembling newlines by hand.
    let lines: Vec<&str> = whole_text.split_inclusive('\n').collect();

    let spec = CorpusSpec::new(seed, quads, iris, shards - 1, shards).expect("valid spec");
    let (start, end) = spec.shard_range();
    let expected_tail: String = lines
        [usize::try_from(start).expect("fits usize")..usize::try_from(end).expect("fits usize")]
        .concat();

    let (code, out, err) = run(&[
        "--quads", "5000", "--iris", "300", "--seed", "99", "--shard", "6", "--shards", "7",
    ]);
    assert_eq!(code, 0, "the final shard must succeed; stderr:\n{err}");
    assert_eq!(
        String::from_utf8(out).expect("utf-8 stdout"),
        expected_tail,
        "the final shard's rows must equal exactly the tail slice of the whole run"
    );
}

// ---------------------------------------------------------------------------------------------
// 3. Manifest mode.
// ---------------------------------------------------------------------------------------------

#[test]
fn manifest_reports_every_field_correctly() {
    let (code, out, err) = run(&[
        "--quads",
        "777",
        "--iris",
        "55",
        "--seed",
        "123456789",
        "--shard",
        "2",
        "--shards",
        "5",
        "--manifest",
    ]);
    assert_eq!(code, 0, "manifest run must succeed; stderr:\n{err}");
    assert!(
        err.is_empty(),
        "manifest run must not write to stderr; got:\n{err}"
    );

    let value: serde_json::Value =
        serde_json::from_slice(&out).expect("manifest must be valid JSON");
    assert_eq!(
        value["profile"], CORPUS_PROFILE_ID,
        "profile must equal the profile id"
    );
    assert_eq!(value["seed"], 123_456_789, "seed must echo the argument");
    assert_eq!(value["quads"], 777, "quads must echo the argument");
    assert_eq!(value["iris"], 55, "iris must echo the argument");
    assert_eq!(value["shard"], 2, "shard must echo the argument");
    assert_eq!(value["shards"], 5, "shards must echo the argument");

    let spec = CorpusSpec::new(123_456_789, 777, 55, 2, 5).expect("valid spec");
    let (start, end) = spec.shard_range();
    assert_eq!(
        value["shard_rows"],
        serde_json::json!([start, end]),
        "shard_rows must equal the library's shard_range() for this spec"
    );

    let class_mix = value["class_mix_per_mille"]
        .as_object()
        .expect("class_mix_per_mille must be a JSON object");
    assert_eq!(class_mix.len(), 5, "all five class names must be present");
    for (name, share) in CLASS_MIX_PER_MILLE {
        assert_eq!(
            class_mix.get(name).and_then(serde_json::Value::as_u64),
            Some(u64::from(share)),
            "class {name} must carry its declared per-mille share"
        );
    }
}

#[test]
fn manifest_honours_out_writing_to_file_with_empty_stdout() {
    let path = unique_temp_path("manifest_honours_out", "json");
    let (code, out, err) = run(&[
        "--quads",
        "10",
        "--iris",
        "10",
        "--manifest",
        "--out",
        path.to_str().expect("utf-8 path"),
    ]);
    assert_eq!(code, 0, "manifest to file must succeed; stderr:\n{err}");
    assert!(
        out.is_empty(),
        "stdout must be empty when --manifest is captured by --out"
    );

    let contents = std::fs::read(&path).expect("read manifest file");
    let _: serde_json::Value =
        serde_json::from_slice(&contents).expect("file must hold valid JSON");
    std::fs::remove_file(&path).expect("cleanup manifest file");
}

// ---------------------------------------------------------------------------------------------
// 4. Hard-fail matrix.
// ---------------------------------------------------------------------------------------------

/// One row of the hard-fail matrix: the args, the expected exit code, and a substring stderr
/// MUST contain. Never assert merely "non-zero" or "is_err" — the diagnostic text is the
/// claim under test.
struct FailCase {
    name: &'static str,
    args: &'static [&'static str],
    expected_code: i32,
    stderr_needle: &'static str,
}

#[test]
fn hard_fail_matrix_reports_the_exact_exit_code_and_diagnostic() {
    let cases: &[FailCase] = &[
        FailCase {
            name: "--out as the final argument",
            args: &["--quads", "10", "--iris", "10", "--out"],
            expected_code: 2,
            stderr_needle: "--out requires a path",
        },
        FailCase {
            name: "--out --manifest",
            args: &["--quads", "10", "--iris", "10", "--out", "--manifest"],
            expected_code: 2,
            stderr_needle: "--out requires a path, not the recognized flag",
        },
        FailCase {
            name: "--out --seed",
            args: &["--quads", "10", "--iris", "10", "--out", "--seed"],
            expected_code: 2,
            stderr_needle: "--out requires a path, not the recognized flag",
        },
        FailCase {
            name: "duplicate flag",
            args: &["--quads", "10", "--quads", "20", "--iris", "10"],
            expected_code: 2,
            stderr_needle: "--quads may not be given more than once",
        },
        FailCase {
            name: "--shards 0",
            args: &["--quads", "10", "--iris", "10", "--shards", "0"],
            expected_code: 2,
            stderr_needle: "shards must be positive",
        },
        FailCase {
            name: "--shard equal to --shards",
            args: &[
                "--quads", "10", "--iris", "10", "--shard", "3", "--shards", "3",
            ],
            expected_code: 2,
            stderr_needle: "shard 3 must be less than shards 3",
        },
        FailCase {
            name: "--shard greater than --shards",
            args: &[
                "--quads", "10", "--iris", "10", "--shard", "5", "--shards", "3",
            ],
            expected_code: 2,
            stderr_needle: "shard 5 must be less than shards 3",
        },
        FailCase {
            name: "--quads 0",
            args: &["--quads", "0", "--iris", "10"],
            expected_code: 2,
            stderr_needle: "quads must be positive",
        },
        FailCase {
            name: "--iris 0",
            args: &["--quads", "10", "--iris", "0"],
            expected_code: 2,
            stderr_needle: "iris must be positive",
        },
        FailCase {
            name: "unknown flag",
            args: &["--bogus"],
            expected_code: 2,
            stderr_needle: "unknown argument \"--bogus\"",
        },
        FailCase {
            name: "no arguments at all",
            args: &[],
            expected_code: 2,
            stderr_needle: "quads must be positive",
        },
        FailCase {
            name: "missing numeric operand",
            args: &["--quads"],
            expected_code: 2,
            stderr_needle: "--quads requires an unsigned integer",
        },
        FailCase {
            name: "non-numeric operand",
            args: &["--quads", "abc", "--iris", "10"],
            expected_code: 2,
            stderr_needle: "--quads requires an unsigned integer",
        },
    ];

    for case in cases {
        let (code, stdout, stderr) = run(case.args);
        assert_eq!(
            code, case.expected_code,
            "case {:?}: unexpected exit code; stderr:\n{stderr}",
            case.name
        );
        assert!(
            stderr.contains(case.stderr_needle),
            "case {:?}: stderr missing {:?}; got:\n{stderr}",
            case.name,
            case.stderr_needle
        );
        assert!(
            stdout.is_empty(),
            "case {:?}: a failing parse must never write to stdout",
            case.name
        );
    }
}

// ---------------------------------------------------------------------------------------------
// 6. Over-refusal counter-checks: for every refusal above, the neighbouring VALID input must
//    still succeed. A suite that only pins the refusals would let a future over-tightening
//    pass silently.
// ---------------------------------------------------------------------------------------------

#[test]
fn out_accepts_a_leading_dash_filename_and_a_relative_path() {
    let dir = std::env::temp_dir();
    let dash_name = format!("-weird-{}.nq", unique_tag());
    let relative_name = format!("relative-{}.nq", unique_tag());

    // Neighbour of "--out --manifest" / "--out --seed": a path that merely STARTS with a
    // dash, but is not one of the exact recognized flag tokens, must be accepted.
    let (code, out, err) = run_in(&dir, &["--quads", "5", "--iris", "5", "--out", &dash_name]);
    assert_eq!(
        code, 0,
        "a leading-dash path that is not a recognized flag token must be accepted; stderr:\n{err}"
    );
    assert!(out.is_empty(), "output must go to the file, not stdout");
    let dash_path = dir.join(&dash_name);
    assert!(
        dash_path.exists(),
        "the leading-dash-named file must have been created"
    );
    std::fs::remove_file(&dash_path).expect("cleanup dash-named file");

    let (code, out, err) = run_in(
        &dir,
        &["--quads", "5", "--iris", "5", "--out", &relative_name],
    );
    assert_eq!(code, 0, "a relative path must be accepted; stderr:\n{err}");
    assert!(out.is_empty(), "output must go to the file, not stdout");
    let relative_path = dir.join(&relative_name);
    assert!(
        relative_path.exists(),
        "the relative-path file must have been created"
    );
    std::fs::remove_file(&relative_path).expect("cleanup relative-path file");
}

#[test]
fn quads_one_iris_one_is_accepted() {
    // Neighbour of "--quads 0" / "--iris 0": the smallest positive values must succeed.
    let (code, out, err) = run(&["--quads", "1", "--iris", "1"]);
    assert_eq!(code, 0, "quads=1, iris=1 must be accepted; stderr:\n{err}");
    let text = String::from_utf8(out).expect("utf-8 stdout");
    assert_eq!(text.lines().count(), 1, "exactly one row must be emitted");
}

#[test]
fn shards_one_is_accepted() {
    // Neighbour of "--shards 0": one shard (the whole corpus) must be accepted.
    let (code, _out, err) = run(&["--quads", "10", "--iris", "10", "--shards", "1"]);
    assert_eq!(code, 0, "shards=1 must be accepted; stderr:\n{err}");
}

#[test]
fn last_shard_index_is_accepted() {
    // Neighbour of "--shard equal/greater than --shards": the last legal shard index
    // (shard == shards - 1) must be accepted.
    let (code, _out, err) = run(&[
        "--quads", "10", "--iris", "10", "--shard", "6", "--shards", "7",
    ]);
    assert_eq!(
        code, 0,
        "shard 6 of 7 (the last legal shard) must be accepted; stderr:\n{err}"
    );
}

#[test]
fn very_large_iris_is_accepted() {
    // Neighbour of "--iris 0": a very large but still positive IRI target must be accepted.
    let (code, _out, err) = run(&["--quads", "1", "--iris", "4294967295"]);
    assert_eq!(code, 0, "iris=4294967295 must be accepted; stderr:\n{err}");
}

// ---------------------------------------------------------------------------------------------
// 5. --help and -h.
// ---------------------------------------------------------------------------------------------

#[test]
fn help_long_flag_prints_usage_to_stdout_with_empty_stderr() {
    let (code, out, err) = run(&["--help"]);
    assert_eq!(code, 0, "--help must exit 0; stderr:\n{err}");
    assert!(
        err.is_empty(),
        "--help must not write to stderr; got:\n{err}"
    );
    let text = String::from_utf8(out).expect("utf-8 stdout");
    assert!(
        text.starts_with("Usage: bench-corpus"),
        "usage text must be on stdout; got:\n{text}"
    );
}

#[test]
fn help_short_flag_prints_usage_to_stdout_with_empty_stderr() {
    let (code, out, err) = run(&["-h"]);
    assert_eq!(code, 0, "-h must exit 0; stderr:\n{err}");
    assert!(err.is_empty(), "-h must not write to stderr; got:\n{err}");
    let text = String::from_utf8(out).expect("utf-8 stdout");
    assert!(
        text.starts_with("Usage: bench-corpus"),
        "usage text must be on stdout; got:\n{text}"
    );
}

// ---------------------------------------------------------------------------------------------
// 7. Every generated row parses as strict N-Quads.
// ---------------------------------------------------------------------------------------------

#[test]
fn generated_rows_parse_as_strict_nquads_with_exact_line_count() {
    let quads = 4_000u64;
    let (code, out, err) = run(&[
        "--quads",
        &quads.to_string(),
        "--iris",
        "50",
        "--seed",
        "31337",
    ]);
    assert_eq!(code, 0, "generation must succeed; stderr:\n{err}");

    let text = String::from_utf8(out).expect("utf-8 stdout");
    let line_count = u64::try_from(text.lines().count()).expect("line count fits u64");
    assert_eq!(
        line_count, quads,
        "the CLI emits exactly one line per requested quad"
    );

    let dataset = purrdf_rdf::parse_dataset(text.as_bytes(), "application/n-quads", None)
        .expect("every generated row must satisfy the strict N-Quads reader");

    // The dataset's row count is not a free-floating inequality: it is exactly the number of
    // DISTINCT emitted lines. Identical (s, p, o, g) rows minted at different slots collapse
    // under the dataset's set semantics — nothing else may be lost. Any shortfall below the
    // distinct-line count would be a silent drop, not "extra deduplication".
    let distinct_lines = text
        .lines()
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    assert_eq!(
        dataset.rdf_row_count(),
        distinct_lines,
        "the dataset must hold exactly one row per DISTINCT emitted line ({distinct_lines} of \
         {line_count} lines are distinct): identical (s, p, o, g) rows minted at different \
         slots collapse, and nothing else may be lost — any other shortfall is a silent drop"
    );

    // Wholesale collapse would also satisfy a bare "<= line_count" check, which is why that
    // inequality alone proves nothing. Measured at `--quads 4000 --iris 50 --seed 31337`,
    // 3_791 of the 4_000 emitted lines are distinct (94.775%): the skewed 50-entity subject
    // space collides often enough to shave some rows, but nowhere near half of them. Guard
    // against a generator or parser regression that quietly collapses far more than that by
    // requiring at least 90% of emitted lines to survive as distinct.
    assert!(
        distinct_lines * 10 >= line_count as usize * 9,
        "distinct emitted lines ({distinct_lines} of {line_count}) fell below 90% of the \
         emitted line count; measured at these parameters it is ~94.8% distinct, so a drop \
         this large signals a generator regression, not ordinary skewed-entity collision"
    );
}
