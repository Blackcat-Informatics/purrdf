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

use purrdf_bench::{CLASS_MIX_PER_MILLE, CORPUS_PROFILE_ID, CorpusSpec, ROW_MIX_PER_MILLE};

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

    // `iris` is the entity INDEX SPACE, a target — never an achieved distinct-entity count.
    // Here 55 is the space and the shard emits 155 lines, so a reader who mistook the field for
    // a distinct count would be reading a number that is neither the space nor the achievement.
    // Nothing in the manifest claims the achieved count, by design: establishing it means
    // enumerating the corpus, which is what streaming at full scale exists to avoid.
    assert_eq!(
        value["emitted_lines"],
        serde_json::json!(end - start),
        "emitted_lines must be exactly end - start from shard_range()"
    );

    // The two mixes are INDEPENDENT AXES and the manifest must report both: the class mix is
    // the shape of an entity IRI over the entity INDEX SPACE, the row mix is the shape of a row
    // over the emitted rows. A capture that recorded only the class mix would not name the row
    // shapes its bytes actually contain — and an unqualified `class_mix_per_mille` invited a
    // capture to record the entity-space shares as though they were row-level ones, which under
    // the skew they are not.
    assert!(
        value.get("class_mix_per_mille").is_none(),
        "the unqualified class-mix key must not be emitted: its basis was ambiguous"
    );
    let class_mix = value["entity_class_mix_per_mille"]
        .as_object()
        .expect("entity_class_mix_per_mille must be a JSON object");
    assert_eq!(
        class_mix.len(),
        CLASS_MIX_PER_MILLE.len(),
        "every class name must be present"
    );
    let mut class_total = 0u64;
    for (name, share) in CLASS_MIX_PER_MILLE {
        assert_eq!(
            class_mix.get(name).and_then(serde_json::Value::as_u64),
            Some(u64::from(share)),
            "class {name} must carry its declared per-mille share"
        );
        class_total += u64::from(share);
    }
    assert_eq!(class_total, 1_000, "the class mix must sum to 1000");

    let row_mix = value["row_mix_per_mille"]
        .as_object()
        .expect("row_mix_per_mille must be a JSON object");
    assert_eq!(
        row_mix.len(),
        ROW_MIX_PER_MILLE.len(),
        "every row kind must be present"
    );
    let mut row_total = 0u64;
    for (name, share) in ROW_MIX_PER_MILLE {
        assert_eq!(
            row_mix.get(name).and_then(serde_json::Value::as_u64),
            Some(u64::from(share)),
            "row kind {name} must carry its declared per-mille share"
        );
        row_total += u64::from(share);
    }
    assert_eq!(row_total, 1_000, "the row mix must sum to 1000");

    // The manifest names every field it carries and nothing else: a field added without a
    // matching assertion above would go unpinned.
    let top_level: Vec<&str> = value
        .as_object()
        .expect("the manifest must be a JSON object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        top_level,
        vec![
            "emitted_lines",
            "entity_class_mix_per_mille",
            "iris",
            "profile",
            "quads",
            "row_mix_per_mille",
            "seed",
            "shard",
            "shard_rows",
            "shards",
        ],
        "every manifest field must be covered by an assertion in this test"
    );
}

#[test]
fn manifest_emitted_lines_equals_the_lines_the_same_invocation_produces() {
    // `emitted_lines` is arithmetic on `shard_range()`, so it costs nothing at any scale — but
    // an arithmetic claim about output is worth only as much as the one check that it matches
    // the output. Drive the SAME arguments twice, once for the manifest and once for the rows.
    for (shard, shards) in [(0u64, 1u64), (0, 7), (3, 7), (6, 7)] {
        let common: Vec<String> = ["--quads", "5000", "--iris", "300", "--seed", "99"]
            .iter()
            .map(ToString::to_string)
            .chain([
                "--shard".to_string(),
                shard.to_string(),
                "--shards".to_string(),
                shards.to_string(),
            ])
            .collect();
        let mut manifest_args: Vec<&str> = common.iter().map(String::as_str).collect();
        manifest_args.push("--manifest");
        let (code, out, err) = run(&manifest_args);
        assert_eq!(code, 0, "manifest run must succeed; stderr:\n{err}");
        let value: serde_json::Value =
            serde_json::from_slice(&out).expect("manifest must be valid JSON");
        let claimed = value["emitted_lines"]
            .as_u64()
            .expect("emitted_lines must be an unsigned integer");

        let row_args: Vec<&str> = common.iter().map(String::as_str).collect();
        let (code, out, err) = run(&row_args);
        assert_eq!(code, 0, "row run must succeed; stderr:\n{err}");
        let produced = u64::try_from(
            String::from_utf8(out)
                .expect("utf-8 stdout")
                .lines()
                .count(),
        )
        .expect("line count fits u64");
        assert_eq!(
            claimed, produced,
            "shard {shard} of {shards}: the manifest claimed {claimed} emitted lines but the \
             same invocation produced {produced}"
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
    // Neighbour of "--iris 0", and the over-refusal counter-check for the skew map's 2^32
    // correction: the entity space is a u64, so every positive value must be accepted and must
    // generate cleanly — on BOTH sides of the old 32-bit overflow point, at the smallest legal
    // value, and at the very top of the range. `--iris 10000000000` in particular used to
    // panic under `[profile.dev]`'s `overflow-checks = true`.
    for iris in [
        "1",
        "4294967295",
        "4294967296",
        "4294967297",
        "10000000000",
        "18446744073709551615",
    ] {
        let (code, out, err) = run(&["--quads", "2000", "--iris", iris]);
        assert_eq!(code, 0, "iris={iris} must be accepted; stderr:\n{err}");
        let text = String::from_utf8(out).expect("utf-8 stdout");
        assert_eq!(
            text.lines().count(),
            2_000,
            "iris={iris} must emit exactly one line per requested quad"
        );
        purrdf_rdf::parse_dataset(text.as_bytes(), "application/n-quads", None)
            .unwrap_or_else(|error| panic!("iris={iris} must generate strict N-Quads: {error}"));
    }
}

#[test]
fn ten_billion_iris_mints_only_in_range_entity_indexes() {
    // The 2^32 defect had two halves, and this pins the RANGE half end to end through the
    // shipped binary: every `…/e/N` the plain class mints must name an index below `--iris`.
    // (The DISTINCT-COUNT half cannot be seen from the range and is pinned in the library's
    // `skew_map_is_not_funnelled_through_a_thirty_two_bit_intermediate`.)
    let iris = 10_000_000_000u64;
    let (code, out, err) = run(&["--quads", "20000", "--iris", &iris.to_string()]);
    assert_eq!(code, 0, "iris={iris} must be accepted; stderr:\n{err}");
    let text = String::from_utf8(out).expect("utf-8 stdout");

    let mut checked = 0u64;
    for segment in text.split("https://example.org/e/").skip(1) {
        let digits: String = segment.chars().take_while(char::is_ascii_digit).collect();
        let index: u64 = digits.parse().expect("plain-class entity index is a u64");
        assert!(
            index < iris,
            "minted entity index {index} must be below --iris {iris}"
        );
        checked += 1;
    }
    assert!(checked > 0, "the plain class must be exercised");
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

    // The dataset's row count is not a free-floating inequality — but it is no longer a single
    // equality against `quad_count()` either, because the corpus emits RDF 1.2 reifier rows and
    // a reifier row is NOT a base quad: the reader folds it into the reifier table and leaves
    // `quad_count()` untouched. The exact relationship is a three-way partition of the DISTINCT
    // emitted lines:
    //
    //     quad_count() + reifiers().count() + annotations().count() == distinct emitted lines
    //
    // with `annotations().count() == 0` by construction: an annotation is a row whose subject is
    // a reifier declared in the same graph, and reifier IRIs live under `example.org/r/`, a path
    // segment no entity-IRI shape can produce. Measured at `--quads 4000 --iris 50 --seed
    // 31337`: 4_000 emitted lines, 3_818 distinct, 230 of them reifier rows, so the reader must
    // report quad_count() == 3_588, reifiers().count() == 230, annotations().count() == 0 and
    // rdf_row_count() == 3_818. Each term is pinned separately below, so a silent drop cannot
    // hide by moving rows from one table to another.
    let lines: Vec<&str> = text.lines().collect();
    let distinct_lines = lines
        .iter()
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let reified: std::collections::BTreeSet<&&str> =
        lines.iter().filter(|row| row.contains("<<(")).collect();
    let distinct_reified = reified.len();

    assert!(
        distinct_reified > 0,
        "the partition below is vacuous unless reifier rows are actually present"
    );
    assert_eq!(
        dataset.annotations().count(),
        0,
        "no emitted row may annotate a reifier: reifier IRIs are disjoint from entity IRIs"
    );
    assert_eq!(
        dataset.reifiers().count(),
        distinct_reified,
        "every distinct reifier row must land in the reifier table ({distinct_reified} distinct \
         reifier rows emitted)"
    );
    assert_eq!(
        dataset.quad_count(),
        distinct_lines - distinct_reified,
        "every distinct NON-reifier line must land in the quad table ({distinct_lines} distinct \
         lines less {distinct_reified} distinct reifier rows)"
    );
    assert_eq!(
        dataset.rdf_row_count(),
        distinct_lines,
        "quads + reifiers + annotations must account for exactly the DISTINCT emitted lines \
         ({distinct_lines} of {line_count} lines are distinct): identical rows minted at \
         different slots collapse, and nothing else may be lost — any other shortfall is a \
         silent drop"
    );

    // Wholesale collapse would also satisfy a bare "<= line_count" check, which is why that
    // inequality alone proves nothing. Measured at these parameters, 3_818 of the 4_000 emitted
    // lines are distinct (95.450%): the skewed 50-entity subject space collides often enough to
    // shave some rows, but nowhere near half of them. Guard against a generator or parser
    // regression that quietly collapses far more than that by requiring at least 90% of emitted
    // lines to survive as distinct.
    assert!(
        distinct_lines * 10 >= line_count as usize * 9,
        "distinct emitted lines ({distinct_lines} of {line_count}) fell below 90% of the \
         emitted line count; measured at these parameters it is ~95.5% distinct, so a drop \
         this large signals a generator regression, not ordinary skewed-entity collision"
    );
}

// ---------------------------------------------------------------------------------------------
// 8. Term-kind coverage through the shipped binary: the corpus the CLI actually emits must
//    contain every RDF 1.2 and term-kind class the profile promises, NON-VACUOUSLY.
// ---------------------------------------------------------------------------------------------

#[test]
fn generated_corpus_contains_every_promised_term_kind() {
    let (code, out, err) = run(&["--quads", "20000", "--iris", "2000", "--seed", "4242"]);
    assert_eq!(code, 0, "generation must succeed; stderr:\n{err}");
    let text = String::from_utf8(out).expect("utf-8 stdout");

    // A structurally-present but EMPTY class is the failure mode here: the shipped corpus once
    // contained zero triple terms, zero reifiers, zero typed literals and zero blank nodes with
    // every test green.
    let triple_terms = text.matches("<<(").count();
    let reifier_rows = text
        .matches("<http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies>")
        .count();
    let typed_literals = text.matches("^^<http://www.w3.org/2001/XMLSchema#").count();
    let blank_nodes = text.matches("_:").count();

    assert!(triple_terms > 0, "corpus must contain RDF 1.2 triple terms");
    assert!(reifier_rows > 0, "corpus must contain RDF 1.2 reifier rows");
    assert_eq!(
        triple_terms, reifier_rows,
        "every triple term must be bound by exactly one reifier row"
    );
    assert!(typed_literals > 0, "corpus must contain typed literals");
    assert!(blank_nodes > 0, "corpus must contain blank nodes");
    assert!(
        text.contains("_:bs"),
        "blank nodes must appear in SUBJECT position"
    );
    assert!(
        text.contains("_:bo"),
        "blank nodes must appear in OBJECT position"
    );
    for datatype in ["integer", "decimal", "date", "boolean"] {
        assert!(
            text.contains(&format!("^^<http://www.w3.org/2001/XMLSchema#{datatype}>")),
            "typed literals must cycle over xsd:{datatype}"
        );
    }

    // Every row kind's measured share must match ROW_MIX_PER_MILLE. The rarest kind
    // (blank-node, 40 per mille) draws ~800 of these 20_000 rows, whose binomial standard
    // deviation is 1.4 per mille; the 10 per mille tolerance is therefore ~7 sigma — wide
    // enough never to flake, far too narrow to admit a drifted table.
    let rows = u64::try_from(text.lines().count()).expect("row count fits u64");
    for (name, share) in ROW_MIX_PER_MILLE {
        let count = u64::try_from(
            text.lines()
                .filter(|row| row_kind_name_of(row) == name)
                .count(),
        )
        .expect("count fits u64");
        let measured = (count * 2_000 + rows) / (2 * rows);
        assert!(
            measured.abs_diff(u64::from(share)) <= 10,
            "row kind {name}: measured {measured} per mille of {rows} rows, pinned {share}"
        );
    }
}

/// Classifies an emitted row by its TEXT alone, returning the matching `ROW_MIX_PER_MILLE` name.
/// Subject and predicate are always the first two space-separated tokens (IRIs and blank-node
/// labels never contain a space), so the object is everything after them, less the trailing
/// ` .` and any named graph.
fn row_kind_name_of(row: &str) -> &'static str {
    let body = row
        .strip_suffix(" .")
        .unwrap_or_else(|| panic!("every row must end with ' .': {row}"));
    let mut fields = body.splitn(3, ' ');
    let _subject = fields.next().expect("subject token");
    let _predicate = fields.next().expect("predicate token");
    let rest = fields.next().expect("object token");
    let object = match rest.rfind(" <https://example.org/g/") {
        Some(cut) if rest.ends_with('>') => &rest[..cut],
        _ => rest,
    };

    if object.contains("<<(") {
        "reified"
    } else if row.starts_with("_:") || object.starts_with("_:") {
        "blank-node"
    } else if object.ends_with("\"@zh") {
        "zh-literal"
    } else if object.contains("^^<http://www.w3.org/2001/XMLSchema#") {
        "typed-literal"
    } else if object.starts_with("\"value ") {
        "plain-literal"
    } else if object.starts_with("\"text ") {
        "long-text-literal"
    } else if object.starts_with('<') && object.ends_with('>') {
        "entity-edge"
    } else {
        panic!("row text matches no known row kind: {row}")
    }
}
