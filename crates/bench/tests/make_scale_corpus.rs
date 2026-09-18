// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Drives the DOCUMENTED `make scale-corpus` entry point (`docs/BENCHMARKS.md`), never
//! `scripts/scale-corpus.sh` directly.
//!
//! `crates/bench/tests/corpus_cli.rs` already pins the built `bench-corpus` binary and every
//! test in the rest of this crate drives either the library or the script directly — and that
//! gap is exactly what let two `Makefile`-only defects ship past 61 passing tests:
//!
//! * the `scale-corpus` recipe had no leading `@`, so `make` echoed its own recipe line onto the
//!   SAME stdout stream `SCALE_MODE=pipe` uses for corpus bytes, corrupting the first bytes of
//!   the exact `make scale-corpus ... SCALE_MODE=pipe | your-loader` invocation
//!   `docs/BENCHMARKS.md` prescribes verbatim;
//! * `SCALE_SINK` / `SCALE_MANIFEST` / `SCALE_OUT` were interpolated UNQUOTED into the recipe,
//!   so `make` word-split any value containing a space — dead for every real loader invocation,
//!   which always carries arguments (`wc -c`, `sha256sum -b`, ...).
//!
//! Neither defect is visible from the script: `bash scripts/scale-corpus.sh` reads its
//! parameters from already-parsed shell variables, never from a `make` recipe line. Only
//! `Command::new("make")` can see a `make`-level regression, so that is what this file drives.
//!
//! `SCALE_BIN` is set to the already-built `CARGO_BIN_EXE_bench-corpus` so `make` never spawns
//! its own `cargo build --release` — the shards become pure execs of a binary this crate's own
//! test harness already built, keeping this file fast.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

/// The path to the built `bench-corpus` binary, reused as `SCALE_BIN` so `make scale-corpus`
/// never triggers its own release build.
const BENCH: &str = env!("CARGO_BIN_EXE_bench-corpus");

/// A monotonically increasing counter, so every call to [`unique_tag`] in this process is
/// distinct even across parallel test threads.
static UNIQUE: AtomicU64 = AtomicU64::new(0);

/// A filesystem-unique fragment: this process's id plus a monotonic counter.
fn unique_tag() -> String {
    format!(
        "{}-{}",
        std::process::id(),
        UNIQUE.fetch_add(1, Ordering::Relaxed)
    )
}

/// The repository root, resolved from `CARGO_MANIFEST_DIR` (`crates/bench`) rather than the
/// process's current directory, so this test is independent of how `cargo test` was invoked.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/bench has two ancestors: crates/ and the repository root")
        .to_path_buf()
}

/// Runs `make <args>` from the repository root with `SCALE_BIN` pinned to the already-built
/// binary, returning (exit code, stdout bytes, stderr text).
fn run_make(args: &[&str]) -> (i32, Vec<u8>, String) {
    // This test itself runs under `cargo test --workspace` inside `make check`, so it is
    // ALREADY a sub-make (`make[1]` or deeper) by the time it spawns this nested `make`. GNU
    // make tracks recursion depth via `MAKELEVEL` in the environment, and it turns on `-w`
    // ("print directory") AUTOMATICALLY whenever `MAKELEVEL` shows the invocation is not the
    // top-level one — independently of whatever `MAKEFLAGS`/`MFLAGS` say. Clearing only
    // `MAKEFLAGS`/`MFLAGS` is therefore NOT sufficient: with `MAKELEVEL` still inherited, the
    // nested `make` still decides for itself that it is a sub-make and still emits `make[1]:
    // Entering directory '...'` / `Leaving directory '...'` banners onto the SAME stdout stream
    // `SCALE_MODE=pipe` uses for corpus bytes, corrupting the comparison this file exists to
    // make — even though the recipe itself does nothing wrong. (Confirmed by experiment: with
    // `MAKEFLAGS` removed but `MAKELEVEL=1` still set, the banner reappears; only removing both
    // — or removing `MAKELEVEL` and passing `--no-print-directory` — yields clean output.) That
    // is an artifact of running `make` inside `make`, not of the `scale-corpus` lane, so the
    // documented invocation (a shell with no pending recursion state at all) is reproduced by
    // scrubbing all three variables from the child.
    let output = Command::new("make")
        .current_dir(repo_root())
        .env_remove("MAKEFLAGS")
        .env_remove("MFLAGS")
        .env_remove("MAKELEVEL")
        .env("SCALE_BIN", BENCH)
        .args(args)
        .output()
        .expect("spawn make");
    (
        output.status.code().expect("make exited normally"),
        output.stdout,
        String::from_utf8(output.stderr).expect("utf-8 stderr"),
    )
}

/// Runs the built `bench-corpus` binary directly (not through `make`) with the given whole-run
/// (unsharded) parameters, returning its stdout bytes.
fn whole(quads: u64, iris: u64, seed: u64) -> Vec<u8> {
    let output = Command::new(BENCH)
        .args([
            "--seed",
            &seed.to_string(),
            "--quads",
            &quads.to_string(),
            "--iris",
            &iris.to_string(),
        ])
        .output()
        .expect("spawn bench-corpus directly");
    assert!(
        output.status.success(),
        "the direct whole run must succeed; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

// ---------------------------------------------------------------------------------------------
// F1 — `make scale-corpus ... SCALE_MODE=pipe` must emit ONLY corpus bytes on stdout.
// ---------------------------------------------------------------------------------------------

#[test]
fn make_scale_corpus_pipe_mode_matches_a_direct_whole_run_byte_for_byte() {
    let (quads, iris, seed, shards) = (600u64, 70u64, 42u64, 3u64);
    let expected = whole(quads, iris, seed);

    let (code, stdout, stderr) = run_make(&[
        "scale-corpus",
        &format!("SCALE_QUADS={quads}"),
        &format!("SCALE_IRIS={iris}"),
        &format!("SCALE_SEED={seed}"),
        &format!("SCALE_SHARDS={shards}"),
        "SCALE_MODE=pipe",
    ]);
    assert_eq!(
        code, 0,
        "make scale-corpus ... SCALE_MODE=pipe must exit 0; stderr:\n{stderr}"
    );
    assert_eq!(
        stdout, expected,
        "the documented `make scale-corpus ... SCALE_MODE=pipe | your-loader` invocation must \
         emit bytes byte-identical to a whole (unsharded) run. A recipe-level echo onto stdout \
         (a `make`-entry-point defect the script itself cannot exhibit) would corrupt exactly \
         this comparison, e.g. by prepending the recipe's own command line to the payload"
    );
    assert_eq!(
        str::from_utf8(&stdout)
            .expect("piped scale-corpus output must be valid UTF-8 N-Quads text")
            .lines()
            .count() as u64,
        quads,
        "the piped whole run must contain exactly one line per requested quad, with no extra \
         leading line from an echoed recipe"
    );
}

// ---------------------------------------------------------------------------------------------
// F2 — `SCALE_SINK`, `SCALE_MANIFEST` and `SCALE_OUT` must survive `make`'s word-splitting.
// ---------------------------------------------------------------------------------------------

#[test]
fn make_scale_corpus_sink_argument_survives_the_make_entry_point() {
    // A single-word sink already worked before the fix; the regression is specifically a sink
    // command carrying an ARGUMENT, which every real loader invocation does.
    for sink in ["wc -c", "sha256sum -b"] {
        let (code, stdout, stderr) = run_make(&[
            "scale-corpus",
            "SCALE_QUADS=200",
            "SCALE_IRIS=20",
            "SCALE_SHARDS=2",
            "SCALE_MODE=stream",
            &format!("SCALE_SINK={sink}"),
        ]);
        assert_eq!(
            code,
            0,
            "SCALE_SINK={sink:?} (a multi-word command) must survive `make`'s recipe \
             interpolation; stderr:\n{stderr}\nstdout:\n{}",
            String::from_utf8_lossy(&stdout)
        );
        assert!(
            !stderr.contains("command not found"),
            "SCALE_SINK={sink:?}: the sink's own argument must not be treated as a separate \
             command by an unquoted `make` interpolation; stderr:\n{stderr}"
        );
    }
}

#[test]
fn make_scale_corpus_manifest_path_with_a_space_survives_the_make_entry_point() {
    let arena = std::env::temp_dir().join(format!("purrdf-bench scale audit {}", unique_tag()));
    std::fs::create_dir_all(&arena).expect("create a space-containing arena directory");
    let manifest_path = arena.join("m.json");

    let (code, _stdout, stderr) = run_make(&[
        "scale-corpus",
        "SCALE_QUADS=100",
        "SCALE_IRIS=10",
        "SCALE_SHARDS=2",
        "SCALE_MODE=stream",
        &format!("SCALE_MANIFEST={}", manifest_path.display()),
    ]);
    assert_eq!(
        code, 0,
        "a SCALE_MANIFEST path containing a space must survive the `make` entry point; \
         stderr:\n{stderr}"
    );
    assert!(
        manifest_path.exists(),
        "the manifest must have been written to the space-containing path {}, not split at the \
         space by an unquoted `make` interpolation",
        manifest_path.display()
    );
    let contents = std::fs::read(&manifest_path).expect("read manifest");
    let _: serde_json::Value =
        serde_json::from_slice(&contents).expect("manifest must be valid JSON");

    std::fs::remove_dir_all(&arena).expect("cleanup arena");
}

#[test]
fn make_scale_corpus_out_path_with_a_space_survives_the_make_entry_point() {
    let out_dir = std::env::temp_dir().join(format!("purrdf-bench scale out {}", unique_tag()));
    std::fs::create_dir_all(&out_dir).expect("create a space-containing out directory");

    let (code, _stdout, stderr) = run_make(&[
        "scale-corpus",
        "SCALE_QUADS=100",
        "SCALE_IRIS=10",
        "SCALE_SHARDS=2",
        "SCALE_MODE=files",
        &format!("SCALE_OUT={}", out_dir.display()),
    ]);
    assert_eq!(
        code, 0,
        "a SCALE_OUT path containing a space must survive the `make` entry point; stderr:\n{stderr}"
    );
    let written: Vec<_> = std::fs::read_dir(&out_dir)
        .expect("read the space-containing out directory")
        .filter_map(Result::ok)
        .collect();
    assert!(
        !written.is_empty(),
        "SCALE_MODE=files must have written shard files under the space-containing SCALE_OUT \
         {}, not split it at the space by an unquoted `make` interpolation",
        out_dir.display()
    );

    std::fs::remove_dir_all(&out_dir).expect("cleanup out directory");
}
