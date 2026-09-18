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
///
/// The child's recursion state is scrubbed so this reproduces the DOCUMENTED invocation — a
/// plain shell with no pending `make` recursion. [`run_make_at_level_one`] covers the other
/// half, a child that believes it is a sub-make.
fn run_make(args: &[&str]) -> (i32, Vec<u8>, String) {
    // This test itself runs under `cargo test --workspace` inside `make check`, so it is
    // ALREADY a sub-make (`make[1]` or deeper) by the time it spawns this nested `make`. GNU
    // make tracks recursion depth via `MAKELEVEL` in the environment, and it turns on `-w`
    // ("print directory") AUTOMATICALLY whenever `MAKELEVEL` shows the invocation is not the
    // top-level one — independently of whatever `MAKEFLAGS`/`MFLAGS` say. Clearing only
    // `MAKEFLAGS`/`MFLAGS` is therefore NOT sufficient: with `MAKELEVEL` still inherited, the
    // nested `make` still decides for itself that it is a sub-make and would emit `make[1]:
    // Entering directory '...'` / `Leaving directory '...'` banners onto the SAME stdout stream
    // `SCALE_MODE=pipe` uses for corpus bytes.
    //
    // The repository `Makefile` now sets `MAKEFLAGS += --no-print-directory` at its top, which
    // suppresses those banners at every recursion depth — that is what
    // `make_scale_corpus_pipe_mode_is_clean_even_when_the_child_thinks_it_is_a_sub_make`
    // pins. This helper still scrubs all three variables because its job is to reproduce the
    // documented invocation exactly, not to lean on the global setting.
    run_make_with(args, true)
}

/// Runs `make <args>` exactly like [`run_make`], but leaves `MAKELEVEL=1` in the child so GNU
/// make decides for itself that it is a sub-make — the state a wrapper `Makefile` puts it in.
fn run_make_at_level_one(args: &[&str]) -> (i32, Vec<u8>, String) {
    run_make_with(args, false)
}

fn run_make_with(args: &[&str], scrub_recursion_state: bool) -> (i32, Vec<u8>, String) {
    let mut command = Command::new("make");
    command.current_dir(repo_root()).env("SCALE_BIN", BENCH);
    if scrub_recursion_state {
        command
            .env_remove("MAKEFLAGS")
            .env_remove("MFLAGS")
            .env_remove("MAKELEVEL");
    } else {
        command
            .env_remove("MAKEFLAGS")
            .env_remove("MFLAGS")
            .env("MAKELEVEL", "1");
    }
    let output = command.args(args).output().expect("spawn make");
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

// ---------------------------------------------------------------------------------------------
// F3 — a path knob is DATA. Nothing in it may be executed, and nothing in it may be expanded
// away: writing to a different path than the operator asked for and reporting success is the
// swallowed-error shape `.goals` forbids.
//
// The earlier fix QUOTED the recipe's interpolations, which stops word-splitting but leaves the
// value inside a double-quoted `/bin/sh` string that `sh` still evaluates. Demonstrated before
// this was fixed: a backtick in `SCALE_MANIFEST` RAN, and the manifest then landed on a
// different path with exit 0; `m$x.json` silently became `m.json`; a `"` produced a raw
// `/bin/sh: unexpected EOF` instead of a lane diagnostic. The recipe no longer builds a shell
// assignment prefix at all — `make` `export`s the value, and `override X := $(value X)` freezes
// the bytes before `make`'s own `$` expansion can touch them.
// ---------------------------------------------------------------------------------------------

#[test]
fn make_scale_corpus_never_lets_a_manifest_path_reach_a_shell() {
    // The arena deliberately has NO SPACE in it, unlike the other tests here. The sentinel path
    // is the payload of a command substitution, so a space inside it would be split into extra
    // arguments by the very shell this test is proving never runs — `touch /tmp/a b/EXECUTED`
    // would then create `a` and `b` in the process's working directory and leave the sentinel
    // itself absent, making the "was it executed?" assertion pass for the wrong reason. A
    // single-word path makes that assertion load-bearing. (Observed for real: an earlier
    // spelling of this test left two stray files in the repository root when run against the
    // pre-fix tree, and the sentinel check still saw nothing.)
    let arena = std::env::temp_dir().join(format!("purrdf-bench-scale-inject-{}", unique_tag()));
    std::fs::create_dir_all(&arena).expect("create the arena directory");
    let sentinel = arena.join("EXECUTED");

    // A backtick whose body would create a file. If any shell evaluates this value the sentinel
    // appears; the literal filename contains `/` characters, so a lane that takes the path
    // literally cannot open it and must say so rather than write somewhere else.
    let injected = arena.join(format!("`touch {}`m.json", sentinel.display()));
    let (code, _stdout, stderr) = run_make(&[
        "scale-corpus",
        "SCALE_QUADS=100",
        "SCALE_IRIS=10",
        "SCALE_SHARDS=2",
        "SCALE_MODE=stream",
        &format!("SCALE_MANIFEST={}", injected.display()),
    ]);
    assert!(
        !sentinel.exists(),
        "a backtick inside SCALE_MANIFEST must NEVER be executed; the sentinel {} was created, \
         which means the value still reaches a shell",
        sentinel.display()
    );
    assert_ne!(
        code, 0,
        "an unopenable SCALE_MANIFEST must HARD-FAIL, never exit 0 having written elsewhere; \
         stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("scale-corpus: cannot write the manifest to SCALE_MANIFEST="),
        "the failure must be a LANE diagnostic naming the knob, not a bare shell error; \
         stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("`touch "),
        "the diagnostic must quote back the bytes the lane actually used, so an operator can \
         see the path was taken literally; stderr:\n{stderr}"
    );

    std::fs::remove_dir_all(&arena).expect("cleanup arena");
}

#[test]
fn make_scale_corpus_takes_shell_and_make_metacharacters_in_a_path_literally() {
    // The over-refusal counter-check to the test above: these characters are perfectly legal in
    // a filename, so each one must produce a file with exactly that name.
    //
    // `m$x.json` is the `make`-level half of the defect — `$` in a command-line variable value
    // is `make`'s OWN variable syntax, so without `$(value ...)` this silently became `m.json`.
    // `` x`id`y `` is the shell half, with a command substitution whose output would be
    // unmistakable. Neither may be evaluated by anyone.
    for name in ["m$x.json", "a\"b.json", "x`id`y.json", "with space.json"] {
        let arena =
            std::env::temp_dir().join(format!("purrdf-bench scale literal {}", unique_tag()));
        std::fs::create_dir_all(&arena).expect("create the arena directory");
        let manifest_path = arena.join(name);

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
            "SCALE_MANIFEST={name:?} is a legal filename and must still work — refusing it \
             would be the mirror of the silent-misdirection bug; stderr:\n{stderr}"
        );
        assert!(
            manifest_path.exists(),
            "the manifest must be at the literal path {:?}; the directory instead holds {:?}",
            manifest_path,
            std::fs::read_dir(&arena)
                .expect("read arena")
                .filter_map(Result::ok)
                .map(|entry| entry.file_name())
                .collect::<Vec<_>>()
        );
        let contents = std::fs::read(&manifest_path).expect("read manifest");
        let _: serde_json::Value =
            serde_json::from_slice(&contents).expect("manifest must be valid JSON");

        std::fs::remove_dir_all(&arena).expect("cleanup arena");
    }
}

#[test]
fn make_scale_corpus_rejects_an_unrunnable_sink_with_a_lane_diagnostic() {
    // `SCALE_SINK` is the DELIBERATE exception: it is documented as a command, so it is executed
    // on purpose. Being the exception is not licence to fail badly — a value that cannot even be
    // parsed must be a lane diagnostic and a non-zero exit, never a bare `bash: -c: unexpected
    // EOF` from inside a backgrounded shard and never an exit-0 run that consumed nothing.
    let (code, _stdout, stderr) = run_make(&[
        "scale-corpus",
        "SCALE_QUADS=100",
        "SCALE_IRIS=10",
        "SCALE_SHARDS=2",
        "SCALE_MODE=stream",
        "SCALE_SINK=wc -c \"",
    ]);
    assert_ne!(
        code, 0,
        "an unparseable SCALE_SINK must fail the lane; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("scale-corpus: SCALE_SINK is not a runnable shell command"),
        "the failure must name SCALE_SINK in the lane's own voice; stderr:\n{stderr}"
    );

    // The counter-check: a sink that legitimately uses `$` and a pipeline must still run. The
    // sink is handed to `bash` verbatim, so ITS `$` is its own business.
    let (code, stdout, stderr) = run_make(&[
        "scale-corpus",
        "SCALE_QUADS=100",
        "SCALE_IRIS=10",
        "SCALE_SHARDS=2",
        "SCALE_MODE=stream",
        "SCALE_SINK=cnt=$(wc -c); echo bytes=$cnt",
    ]);
    assert_eq!(
        code, 0,
        "a sink using command substitution is a legal sink and must still run; stderr:\n{stderr}"
    );
    assert!(
        String::from_utf8_lossy(&stdout).contains("bytes="),
        "the sink's own output must reach the report; stdout:\n{}",
        String::from_utf8_lossy(&stdout)
    );
}

// ---------------------------------------------------------------------------------------------
// F4 — the pipe payload survives a caller that believes it is a sub-make, with NO cooperation.
//
// GNU make turns `-w` on by itself whenever an inherited `MAKELEVEL` is nonzero — NOT from
// `MAKEFLAGS`, which a real nested `$(MAKE)` leaves EMPTY. The banner then lands on the same
// stdout the corpus bytes use. `MAKEFLAGS += --no-print-directory` at the top of the repository
// `Makefile` is what makes the payload immune; requiring every wrapper to remember a flag would
// be optionality pushed onto the consumer.
// ---------------------------------------------------------------------------------------------

#[test]
fn make_scale_corpus_pipe_mode_is_clean_even_when_the_child_thinks_it_is_a_sub_make() {
    let (quads, iris, seed, shards) = (600u64, 70u64, 42u64, 3u64);
    let expected = whole(quads, iris, seed);

    let (code, stdout, stderr) = run_make_at_level_one(&[
        "scale-corpus",
        &format!("SCALE_QUADS={quads}"),
        &format!("SCALE_IRIS={iris}"),
        &format!("SCALE_SEED={seed}"),
        &format!("SCALE_SHARDS={shards}"),
        "SCALE_MODE=pipe",
    ]);
    assert_eq!(
        code, 0,
        "the lane must succeed at MAKELEVEL=1; stderr:\n{stderr}"
    );
    assert!(
        !String::from_utf8_lossy(&stdout).contains("Entering directory"),
        "no directory banner may share the payload's stdout"
    );
    assert_eq!(
        stdout, expected,
        "at MAKELEVEL=1 — the state a wrapper `Makefile` puts this lane in — the piped payload \
         must STILL be byte-identical to a whole run, without the caller passing \
         `--no-print-directory`"
    );
}
