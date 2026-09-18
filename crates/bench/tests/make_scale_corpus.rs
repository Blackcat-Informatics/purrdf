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

use std::os::unix::fs::PermissionsExt;
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

/// Runs `make` like [`run_make`], retrying while the child reports `ETXTBSY`.
///
/// NOT a retry on failure in general — only on this one kernel condition, and the assertions
/// afterwards are unchanged. A test that copies a binary and then executes it races the rest of
/// this file: `std::fs::copy` holds the destination open for write, and any OTHER test thread
/// that `fork`s in that window hands the child an inherited write descriptor to the same file,
/// so the `exec` fails with `ETXTBSY` ("Text file busy") until that child exits. Nothing about
/// the lane is wrong when that happens, and the condition clears on its own.
fn run_make_retrying_etxtbsy(args: &[&str]) -> (i32, Vec<u8>, String) {
    for _ in 0..8 {
        let result = run_make(args);
        if !result.2.contains("Text file busy") {
            return result;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    run_make(args)
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

/// Writes `contents` to `path` and makes it executable, returning `path`.
fn write_executable(path: PathBuf, contents: &str) -> PathBuf {
    std::fs::write(&path, contents).expect("write the executable script");
    let mut permissions = std::fs::metadata(&path)
        .expect("stat the freshly written script")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&path, permissions).expect("make the script executable");
    path
}

/// Builds a stand-in for `bench-corpus` that delegates every invocation to the real binary
/// EXCEPT a single shard of a sharded run, which it refuses.
///
/// A sharded lane has no other way to break exactly one shard in `stream` and `pipe` mode: those
/// modes write nothing, so there is no filesystem obstacle to place in one shard's way (the
/// `files`-mode test does exactly that instead, because there the ORDER of the failing command
/// inside the shard body is the whole defect).
///
/// The whole-run manifest is `--shard 0 --shards 1`, so `shards != 1` keeps this wrapper from
/// refusing the manifest and turning the test into an up-front-validation test instead.
fn failing_shard_binary(dir: &Path, refuse_shard: u64) -> PathBuf {
    write_executable(
        dir.join("bench-corpus-failing-shard"),
        &format!(
            r#"#!/bin/sh
shard=""
shards=""
prev=""
for arg in "$@"; do
  case "${{prev}}" in
    --shard) shard="${{arg}}" ;;
    --shards) shards="${{arg}}" ;;
  esac
  prev="${{arg}}"
done
if [ "${{shards}}" != "1" ] && [ "${{shard}}" = "{refuse_shard}" ]; then
  echo "failing-shard stand-in: refusing shard ${{shard}}" >&2
  exit 1
fi
exec "{BENCH}" "$@"
"#
        ),
    )
}

/// A scratch directory unique to this process, created and returned.
fn scratch(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("purrdf-bench-scale-{label}-{}", unique_tag()));
    std::fs::create_dir_all(&dir).expect("create the scratch directory");
    dir
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

// ---------------------------------------------------------------------------------------------
// F5 — A SHARD THAT FAILS MUST FAIL THE RUN, IN EVERY MODE.
//
// The script's own header promises it ("Any shard that fails fails the whole run, loudly ... a
// driver that let either pass would report a corpus that was never generated") and
// `docs/BENCHMARKS.md` repeats it. Nothing tested it, and in `files` mode it was FALSE.
//
// The mechanism: shard bodies are backgrounded subshells started from
// `run_all_shards <body> || die`, and putting that call on the LEFT of `||` suppresses `errexit`
// for its entire dynamic extent — the subshells included. `file_shard` ran the corpus write
// FIRST and the manifest write LAST, so a failing corpus write left a succeeding last command,
// the subshell exited 0, `wait` saw success, and `make scale-corpus SCALE_MODE=files` exited 0
// after printing `bytes=0` for the missing shard, writing a manifest CERTIFYING it, and then
// printing its byte-for-byte guarantee — which the sorted `cat` disproved. `stream_shard` was
// correct only by accident, its failing command happening to be last.
//
// So these tests break a shard in each mode, and the `files` one breaks it in the ORDER that
// defeated `errexit`. The byte-identity test is the other half: the old files-mode test asserted
// only that the output directory was non-empty, which is exactly why it passed on the bug.
// ---------------------------------------------------------------------------------------------

#[test]
fn make_scale_corpus_files_mode_fails_loudly_when_a_shard_cannot_be_written() {
    let out_dir = scratch("failshard-files");
    let (quads, iris, seed, shards) = (400u64, 40u64, 1_592_642_302u64, 4u64);
    let prefix = format!("purrdf-scale-mixed-v1.seed{seed}.quads{quads}.iris{iris}");
    let shard_one = format!("{prefix}.shard-00001-of-00004.nq");

    // A leftover DIRECTORY where shard 1's file belongs. This makes the corpus write — the FIRST
    // of the shard body's two commands — fail while the `--manifest` write after it succeeds,
    // which is precisely the ordering that used to be swallowed.
    std::fs::create_dir(out_dir.join(&shard_one)).expect("occupy the shard file's path");

    // A manifest left by an earlier, successful run of this same shard. A manifest is a
    // CERTIFICATE, so it must not survive a shard that did not produce its corpus file.
    let stale_manifest = out_dir.join(format!("{shard_one}.manifest.json"));
    std::fs::write(&stale_manifest, b"{\"stale\": true}\n").expect("seed a stale shard manifest");

    let (code, stdout, stderr) = run_make(&[
        "scale-corpus",
        &format!("SCALE_QUADS={quads}"),
        &format!("SCALE_IRIS={iris}"),
        &format!("SCALE_SEED={seed}"),
        &format!("SCALE_SHARDS={shards}"),
        "SCALE_MODE=files",
        &format!("SCALE_OUT={}", out_dir.display()),
    ]);
    let stdout = String::from_utf8_lossy(&stdout).into_owned();

    assert_ne!(
        code, 0,
        "a shard that could not be written must fail the whole run; stdout:\n{stdout}\n\
         stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("scale-corpus: FAILED shards: 1"),
        "the run must name the shard that failed in its own diagnostic; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("reproduces a whole run byte for byte"),
        "the byte-for-byte guarantee must NOT be printed for a run whose shards are incomplete; \
         stderr:\n{stderr}"
    );
    assert!(
        !stdout.contains("bytes="),
        "a failed run must not report shard sizes at all — `bytes=0` for a shard that was never \
         written is the corpus-certification bug itself; stdout:\n{stdout}"
    );
    assert!(
        !stale_manifest.exists(),
        "no manifest may be left certifying shard 1, whose corpus file does not exist; {} is \
         still present",
        stale_manifest.display()
    );

    std::fs::remove_dir_all(&out_dir).expect("cleanup out directory");
}

#[test]
fn make_scale_corpus_stream_mode_fails_when_a_shard_fails() {
    let dir = scratch("failshard-stream");
    let stand_in = failing_shard_binary(&dir, 1);

    let (code, stdout, stderr) = run_make(&[
        "scale-corpus",
        "SCALE_QUADS=400",
        "SCALE_IRIS=40",
        "SCALE_SHARDS=4",
        "SCALE_MODE=stream",
        &format!("SCALE_BIN={}", stand_in.display()),
    ]);
    assert_ne!(
        code,
        0,
        "a failing shard must fail a stream run; stdout:\n{}\nstderr:\n{stderr}",
        String::from_utf8_lossy(&stdout)
    );
    assert!(
        stderr.contains("scale-corpus: FAILED shards: 1"),
        "the stream run must name the shard that failed; stderr:\n{stderr}"
    );

    // The over-refusal counter-check: the same lane, same shard count, with a stand-in that
    // refuses NOTHING must still succeed. A run that fails whatever it is given proves nothing.
    let healthy = failing_shard_binary(&dir, u64::MAX);
    let (code, _stdout, stderr) = run_make(&[
        "scale-corpus",
        "SCALE_QUADS=400",
        "SCALE_IRIS=40",
        "SCALE_SHARDS=4",
        "SCALE_MODE=stream",
        &format!("SCALE_BIN={}", healthy.display()),
    ]);
    assert_eq!(
        code, 0,
        "a stream run whose shards all succeed must still exit 0; stderr:\n{stderr}"
    );

    std::fs::remove_dir_all(&dir).expect("cleanup scratch directory");
}

#[test]
fn make_scale_corpus_pipe_mode_fails_when_a_shard_fails() {
    let dir = scratch("failshard-pipe");
    let stand_in = failing_shard_binary(&dir, 1);

    let (code, _stdout, stderr) = run_make(&[
        "scale-corpus",
        "SCALE_QUADS=400",
        "SCALE_IRIS=40",
        "SCALE_SHARDS=4",
        "SCALE_MODE=pipe",
        &format!("SCALE_BIN={}", stand_in.display()),
    ]);
    assert_ne!(
        code, 0,
        "a failing shard must fail a pipe run — a truncated stream silently loaded is the worst \
         outcome this lane has; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("the stream is INCOMPLETE"),
        "the pipe run must say the stream is incomplete; stderr:\n{stderr}"
    );

    std::fs::remove_dir_all(&dir).expect("cleanup scratch directory");
}

#[test]
fn make_scale_corpus_files_mode_sorted_cat_matches_a_direct_whole_run_byte_for_byte() {
    // The guarantee `SCALE_MODE=files` prints on its last line, asserted in BYTES. The
    // pre-existing files-mode test asserted only that the output directory was non-empty, so it
    // passed on a run with an entirely missing shard.
    let (quads, iris, seed, shards) = (600u64, 70u64, 42u64, 3u64);
    let expected = whole(quads, iris, seed);
    let out_dir = scratch("files-identity");

    let (code, _stdout, stderr) = run_make(&[
        "scale-corpus",
        &format!("SCALE_QUADS={quads}"),
        &format!("SCALE_IRIS={iris}"),
        &format!("SCALE_SEED={seed}"),
        &format!("SCALE_SHARDS={shards}"),
        "SCALE_MODE=files",
        &format!("SCALE_OUT={}", out_dir.display()),
    ]);
    assert_eq!(
        code, 0,
        "the files-mode run must succeed; stderr:\n{stderr}"
    );

    // Lexicographic order over the zero-padded names IS shard order; that is what the zero
    // padding is for, and what `cat <out>/<prefix>.shard-*.nq` relies on.
    let mut shard_files: Vec<PathBuf> = std::fs::read_dir(&out_dir)
        .expect("read the out directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        // Exactly what `cat <out>/<prefix>.shard-*.nq` selects: the corpus files and not the
        // `.nq.manifest.json` certificates beside them, whose extension is `json`.
        .filter(|path| path.extension().is_some_and(|extension| extension == "nq"))
        .collect();
    shard_files.sort();
    assert_eq!(
        shard_files.len() as u64,
        shards,
        "every shard must have produced a file; found {shard_files:?}"
    );

    let mut concatenated = Vec::new();
    for path in &shard_files {
        concatenated.extend_from_slice(&std::fs::read(path).expect("read a shard file"));
    }
    // Reported as a length plus the first differing offset rather than as two byte vectors: a
    // failing `assert_eq!` over ~90 kB of N-Quads prints both sides in full and buries the one
    // fact that matters.
    let divergence = concatenated
        .iter()
        .zip(expected.iter())
        .position(|(left, right)| left != right);
    assert!(
        concatenated == expected,
        "the sorted `cat` of the shard files must be byte-identical to a direct whole run — the \
         exact guarantee the lane prints on its last line. Concatenation is {} bytes, the whole \
         run is {} bytes, first differing offset: {divergence:?}",
        concatenated.len(),
        expected.len()
    );

    std::fs::remove_dir_all(&out_dir).expect("cleanup out directory");
}

// ---------------------------------------------------------------------------------------------
// F6 — `SCALE_BIN` is a path knob like every other, and it names the executable that CERTIFIES
// THE BYTES.
//
// It was the one knob the script read that the `Makefile` never passed through `lane-env`, so it
// still went through `make`'s own expansion: `make scale-corpus 'SCALE_BIN=/tmp/bin/de$xcoy'`
// had `$x` expanded away and the lane EXECUTED `/tmp/bin/decoy` — a different binary than the
// operator named — and exited 0.
// ---------------------------------------------------------------------------------------------

#[test]
fn make_scale_corpus_scale_bin_is_taken_literally_and_never_resolves_to_another_binary() {
    let dir = scratch("bin-literal");
    let sentinel = dir.join("DECOY_RAN");

    // The binary `make`'s expansion used to arrive at. It is a real, executable file, so a lane
    // that lost the `$x` runs it happily.
    write_executable(
        dir.join("decoy"),
        &format!("#!/bin/sh\n: >\"{}\"\nexit 0\n", sentinel.display()),
    );

    let requested = dir.join("de$xcoy");
    assert!(
        !requested.exists(),
        "the requested path must NOT exist — that is the whole point of the test"
    );

    let (code, _stdout, stderr) = run_make(&[
        "scale-corpus",
        "SCALE_QUADS=100",
        "SCALE_IRIS=10",
        "SCALE_SHARDS=2",
        "SCALE_MODE=stream",
        &format!("SCALE_BIN={}", requested.display()),
    ]);
    assert!(
        !sentinel.exists(),
        "`make` must not expand `$x` away and run a DIFFERENT executable; the decoy at {} ran",
        dir.join("decoy").display()
    );
    assert_ne!(
        code, 0,
        "a SCALE_BIN that does not exist must hard-fail; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("scale-corpus: SCALE_BIN="),
        "the failure must be a LANE diagnostic naming the knob, not a bare shell or `make` \
         error; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("de$xcoy"),
        "the diagnostic must quote back the bytes the lane actually used; stderr:\n{stderr}"
    );

    std::fs::remove_dir_all(&dir).expect("cleanup scratch directory");
}

#[test]
fn make_scale_corpus_accepts_a_scale_bin_reached_by_symlink_or_a_relative_path() {
    // The other two shapes an operator actually uses, and the two that a `-e`/`-f`/`-x` test
    // could plausibly have broken. `-f` follows symlinks, which is why a symlink to a real
    // binary must still pass — and a test that only ever passed absolute paths would not notice
    // if it stopped.
    let dir = scratch("bin-symlink");
    let link = dir.join("purrdf-via-symlink");
    std::os::unix::fs::symlink(BENCH, &link).expect("symlink the real binary");

    for (label, value) in [
        ("an absolute symlink", link.display().to_string()),
        (
            "a relative path",
            // Relative to the repository root, which is where `make` runs the lane.
            pathdiff_to_repo_root(&link),
        ),
    ] {
        let (code, stdout, stderr) = run_make(&[
            "scale-corpus",
            "SCALE_QUADS=100",
            "SCALE_IRIS=10",
            "SCALE_SHARDS=2",
            "SCALE_MODE=stream",
            &format!("SCALE_BIN={value}"),
        ]);
        assert_eq!(
            code, 0,
            "SCALE_BIN={value:?} ({label}) names a real executable and must run — refusing it \
             would be the mirror of accepting a directory; stderr:\n{stderr}"
        );
        assert!(
            String::from_utf8_lossy(&stdout).contains("total rows=100"),
            "{label}: the run must actually have produced the corpus; stdout:\n{}",
            String::from_utf8_lossy(&stdout)
        );
    }

    std::fs::remove_dir_all(&dir).expect("cleanup scratch directory");
}

/// Expresses `path` relative to the repository root when it is under it, and otherwise returns it
/// unchanged. `/tmp` is not under the repository root on most machines, so the "relative path"
/// case is exercised by way of a second symlink placed inside `target/`.
fn pathdiff_to_repo_root(path: &Path) -> String {
    if let Ok(relative) = path.strip_prefix(repo_root()) {
        return relative.display().to_string();
    }
    // Place a symlink under `target/` (build output, ignored) so a genuinely RELATIVE value can
    // be handed to the lane.
    let anchor = repo_root().join("target");
    std::fs::create_dir_all(&anchor).expect("target/ exists");
    let relative_link = anchor.join(format!("purrdf-bench-relative-{}", unique_tag()));
    let _ = std::fs::remove_file(&relative_link);
    std::os::unix::fs::symlink(path, &relative_link).expect("symlink under target/");
    relative_link
        .strip_prefix(repo_root())
        .expect("the anchor is under the repository root")
        .display()
        .to_string()
}

#[test]
fn make_scale_corpus_accepts_empty_and_absolute_path_knobs() {
    // The knobs that are ALLOWED to be empty must stay allowed: `SCALE_OUT`, `SCALE_SINK` and
    // `SCALE_MANIFEST` all default to unset, and the whole `stream` lane runs that way. Tightening
    // validation is exactly where a "required" creeps in by accident.
    let (code, stdout, stderr) = run_make(&[
        "scale-corpus",
        "SCALE_QUADS=100",
        "SCALE_IRIS=10",
        "SCALE_SHARDS=2",
        "SCALE_MODE=stream",
        "SCALE_OUT=",
        "SCALE_SINK=",
        "SCALE_MANIFEST=",
        &format!("SCALE_BIN={BENCH}"),
    ]);
    assert_eq!(
        code, 0,
        "explicitly empty SCALE_OUT/SCALE_SINK/SCALE_MANIFEST is the DEFAULT configuration and \
         must run; stderr:\n{stderr}"
    );
    assert!(
        String::from_utf8_lossy(&stdout).contains("total rows=100"),
        "stdout:\n{}",
        String::from_utf8_lossy(&stdout)
    );

    // And an ABSOLUTE output path must be honoured verbatim, in files mode.
    let absolute = scratch("absolute-out");
    let (code, _stdout, stderr) = run_make(&[
        "scale-corpus",
        "SCALE_QUADS=100",
        "SCALE_IRIS=10",
        "SCALE_SHARDS=2",
        "SCALE_MODE=files",
        &format!("SCALE_OUT={}", absolute.display()),
        &format!("SCALE_BIN={BENCH}"),
    ]);
    assert_eq!(
        code, 0,
        "an absolute SCALE_OUT must be honoured as written; stderr:\n{stderr}"
    );
    assert!(
        std::fs::read_dir(&absolute)
            .expect("read the absolute out directory")
            .filter_map(Result::ok)
            .any(|entry| entry.path().extension().is_some_and(|e| e == "nq")),
        "the shards must be where the operator asked, at {}",
        absolute.display()
    );

    std::fs::remove_dir_all(&absolute).expect("cleanup out directory");
}

// ---------------------------------------------------------------------------------------------
// F7 — A DIGEST IS A CERTIFICATE. It must never be emitted for output that is empty or that
// failed to be produced.
//
// `SCALE_BIN=/bin/true` exits 0 and writes nothing. The lane checked the manifest command's exit
// STATUS and not its OUTPUT, so `WHOLE_RUN_MANIFEST` was the empty string, the specification was
// emitted as a BLANK LINE, four shards generated nothing, and each was reported as
// `rows=0 bytes=0 sha256=e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` —
// THE SHA-256 OF THE EMPTY STRING — followed by `total rows=0` and exit 0.
//
// This is the same class as the LUBM lane's `sha256(lubm-data.nq) = e3b0c442...` on a corpus that
// did not exist. It is pinned here in every mode, because a fix that lands in one mode and not
// its siblings is the failure this whole audit is about.
// ---------------------------------------------------------------------------------------------

/// The SHA-256 of the empty string. If this ever appears in a lane's output as a digest, the lane
/// has certified nothing at all.
const EMPTY_STRING_SHA256: &str =
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

#[test]
fn make_scale_corpus_never_certifies_a_corpus_a_binary_did_not_produce() {
    let out_dir = scratch("empty-certificate");
    for mode in ["stream", "pipe", "files"] {
        let mut args = vec![
            "scale-corpus".to_string(),
            "SCALE_QUADS=2000".to_string(),
            "SCALE_IRIS=200".to_string(),
            "SCALE_SHARDS=4".to_string(),
            format!("SCALE_MODE={mode}"),
            // A binary that exits 0 and writes nothing — the empty-certificate shape.
            "SCALE_BIN=/bin/true".to_string(),
        ];
        if mode == "files" {
            args.push(format!("SCALE_OUT={}", out_dir.display()));
        }
        let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
        let (code, stdout, stderr) = run_make(&borrowed);
        let stdout = String::from_utf8_lossy(&stdout).into_owned();
        let combined = format!("{stdout}\n{stderr}");

        assert_ne!(
            code, 0,
            "SCALE_MODE={mode}: a binary that exits 0 and produces nothing must FAIL the lane. \
             Exiting 0 is not producing a corpus. output:\n{combined}"
        );
        assert!(
            !combined.contains(EMPTY_STRING_SHA256),
            "SCALE_MODE={mode}: the SHA-256 of the EMPTY STRING must never be published as a \
             digest — it certifies nothing at all. output:\n{combined}"
        );
        assert!(
            !stdout.contains("sha256="),
            "SCALE_MODE={mode}: no per-shard digest may be printed for a run that generated \
             nothing; stdout:\n{stdout}"
        );
        assert!(
            !stdout.contains("total rows=0"),
            "SCALE_MODE={mode}: `total rows=0` is a FAILED run, never a summary line; \
             stdout:\n{stdout}"
        );
        assert!(
            combined.contains("did not produce a usable whole-run manifest"),
            "SCALE_MODE={mode}: the failure must be a LANE diagnostic saying the binary produced \
             no manifest, not a bare blank line where the specification belongs; \
             output:\n{combined}"
        );
        assert!(
            !out_dir
                .join("purrdf-scale-mixed-v1.seed1592642302.quads2000.iris200.manifest.json")
                .exists(),
            "SCALE_MODE={mode}: no whole-run manifest may exist for a corpus that was never \
             produced"
        );
    }
    std::fs::remove_dir_all(&out_dir).expect("cleanup out directory");
}

#[test]
fn make_scale_corpus_still_runs_a_legitimately_tiny_corpus() {
    // THE OVER-REFUSAL COUNTER-CHECK to the test above, and the reason the non-empty guard is at
    // RUN level rather than shard level.
    //
    // With `--quads 1 --shards 8` the generator gives shard 7 the single row and shards 0..6 an
    // EMPTY RANGE, so seven of the eight shards legitimately produce zero bytes and digest to the
    // SHA-256 of the empty string. A shard-level guard would refuse this real run; the run-level
    // guard must not. `SCALE_QUADS` is validated positive, so an empty RUN is never legitimate.
    let out_dir = scratch("tiny-but-real");
    let expected = whole(1, 10, 1_592_642_302);
    assert!(
        !expected.is_empty(),
        "a one-quad whole run really does produce a row"
    );

    let (code, stdout, stderr) = run_make(&[
        "scale-corpus",
        "SCALE_QUADS=1",
        "SCALE_IRIS=10",
        "SCALE_SHARDS=8",
        "SCALE_MODE=stream",
        &format!("SCALE_BIN={BENCH}"),
    ]);
    let stdout_text = String::from_utf8_lossy(&stdout).into_owned();
    assert_eq!(
        code, 0,
        "a one-quad run across eight shards is small but REAL and must succeed; stderr:\n{stderr}"
    );
    assert!(
        stdout_text.contains("total rows=1"),
        "the tiny run must report its single row; stdout:\n{stdout_text}"
    );
    assert!(
        stdout_text.contains(EMPTY_STRING_SHA256),
        "the seven legitimately-empty shards must still be REPORTED, digest and all — the guard \
         is about the run, not about a shard whose range is empty by arithmetic; \
         stdout:\n{stdout_text}"
    );

    // The same run in `files` mode: empty shard FILES are legitimate, an empty run is not.
    let (code, _stdout, stderr) = run_make(&[
        "scale-corpus",
        "SCALE_QUADS=1",
        "SCALE_IRIS=10",
        "SCALE_SHARDS=8",
        "SCALE_MODE=files",
        &format!("SCALE_OUT={}", out_dir.display()),
        &format!("SCALE_BIN={BENCH}"),
    ]);
    assert_eq!(
        code, 0,
        "a one-quad files run must succeed even though seven shard files are empty; \
         stderr:\n{stderr}"
    );
    let mut shard_files: Vec<PathBuf> = std::fs::read_dir(&out_dir)
        .expect("read the out directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "nq"))
        .collect();
    shard_files.sort();
    let mut concatenated = Vec::new();
    for path in &shard_files {
        concatenated.extend_from_slice(&std::fs::read(path).expect("read a shard file"));
    }
    assert_eq!(
        concatenated, expected,
        "the sorted `cat` of a one-quad sharded run must still be byte-identical to a whole run"
    );

    // The `pipe` half, in bytes.
    let (code, stdout, stderr) = run_make(&[
        "scale-corpus",
        "SCALE_QUADS=1",
        "SCALE_IRIS=10",
        "SCALE_SHARDS=8",
        "SCALE_MODE=pipe",
        &format!("SCALE_BIN={BENCH}"),
    ]);
    assert_eq!(
        code, 0,
        "a one-quad pipe run must succeed; stderr:\n{stderr}"
    );
    assert_eq!(
        stdout, expected,
        "the piped one-quad run must be byte-identical to a direct whole run"
    );

    std::fs::remove_dir_all(&out_dir).expect("cleanup out directory");
}

// ---------------------------------------------------------------------------------------------
// F8 — ONE LAW, ONE IMPLEMENTATION: every manifest write is status-checked.
//
// The `SCALE_MANIFEST` branch of `emit_manifest` emitted a three-line lane diagnostic for an
// unwritable destination. The default-destination branch thirteen lines below it had a bare
// `whole_run_manifest >"$1"` — so the IDENTICAL fault produced a named knob through one branch
// and `scripts/scale-corpus.sh: line 390: ...: Is a directory` through the other, with no lane
// voice and no knob named.
// ---------------------------------------------------------------------------------------------

#[test]
fn make_scale_corpus_names_the_knob_when_the_whole_run_manifest_cannot_be_written() {
    let out_dir = scratch("manifest-unwritable");
    let (quads, iris, seed) = (2000u64, 200u64, 1_592_642_302u64);
    let prefix = format!("purrdf-scale-mixed-v1.seed{seed}.quads{quads}.iris{iris}");

    // A DIRECTORY exactly where the whole-run manifest belongs, so opening it for write fails.
    std::fs::create_dir(out_dir.join(format!("{prefix}.manifest.json")))
        .expect("occupy the manifest's path with a directory");

    let (code, _stdout, stderr) = run_make(&[
        "scale-corpus",
        &format!("SCALE_QUADS={quads}"),
        &format!("SCALE_IRIS={iris}"),
        &format!("SCALE_SEED={seed}"),
        "SCALE_SHARDS=4",
        "SCALE_MODE=files",
        &format!("SCALE_OUT={}", out_dir.display()),
        &format!("SCALE_BIN={BENCH}"),
    ]);
    assert_ne!(
        code, 0,
        "an unwritable whole-run manifest must fail the lane; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("scale-corpus: cannot write the whole-run manifest"),
        "the failure must be a LANE diagnostic, not a bare `scripts/scale-corpus.sh: line N: \
         ...: Is a directory`; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("SCALE_OUT="),
        "the diagnostic must name the KNOB that supplied the path — the whole point of the \
         sibling branch's message; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("The path is used exactly as given, byte for byte"),
        "both branches must emit the SAME three-line shape; stderr:\n{stderr}"
    );

    std::fs::remove_dir_all(&out_dir).expect("cleanup out directory");
}

// ---------------------------------------------------------------------------------------------
// F9 — A MANIFEST IS A CERTIFICATE, SO ONE MUST NEVER OUTLIVE THE RUN IT CERTIFIES.
//
// The law was stated and applied at SHARD level and not at RUN level. After a shard failure the
// arena held three of four shard corpus files beside a whole-run manifest certifying
// `"quads": 2000, "emitted_lines": 2000`, with no marker of failure anywhere — the exact shape
// the shard-level fix exists to stop, one level up.
// ---------------------------------------------------------------------------------------------

#[test]
fn make_scale_corpus_leaves_no_whole_run_manifest_behind_a_failed_run() {
    let out_dir = scratch("run-manifest-outlives");
    let (quads, iris, seed, shards) = (2000u64, 200u64, 1_592_642_302u64, 4u64);
    let prefix = format!("purrdf-scale-mixed-v1.seed{seed}.quads{quads}.iris{iris}");
    let run_manifest = out_dir.join(format!("{prefix}.manifest.json"));

    // A leftover directory where shard 1's corpus file belongs: the shard cannot be written, so
    // the RUN did not produce the corpus its whole-run manifest describes.
    std::fs::create_dir(out_dir.join(format!("{prefix}.shard-00001-of-00004.nq")))
        .expect("occupy the shard file's path");

    let (code, _stdout, stderr) = run_make(&[
        "scale-corpus",
        &format!("SCALE_QUADS={quads}"),
        &format!("SCALE_IRIS={iris}"),
        &format!("SCALE_SEED={seed}"),
        &format!("SCALE_SHARDS={shards}"),
        "SCALE_MODE=files",
        &format!("SCALE_OUT={}", out_dir.display()),
        &format!("SCALE_BIN={BENCH}"),
    ]);
    assert_ne!(code, 0, "the run must fail; stderr:\n{stderr}");
    assert!(
        !run_manifest.exists(),
        "the WHOLE-RUN manifest {} must not outlive the run it certifies: the arena holds only \
         three of four shards, and this file claims emitted_lines={quads}. Directory now holds \
         {:?}",
        run_manifest.display(),
        std::fs::read_dir(&out_dir)
            .expect("read arena")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name())
            .collect::<Vec<_>>()
    );
    assert!(
        stderr.contains("removed the whole-run manifest"),
        "the removal must be ANNOUNCED — a certificate silently vanishing is its own puzzle; \
         stderr:\n{stderr}"
    );

    std::fs::remove_dir_all(&out_dir).expect("cleanup out directory");
}

#[test]
fn make_scale_corpus_keeps_the_whole_run_manifest_of_a_run_that_succeeded() {
    // The over-refusal counter-check to the test above: removing the certificate of a run that
    // DID produce its corpus would destroy the very provenance the lane exists to record.
    let out_dir = scratch("run-manifest-kept");
    let (quads, iris, seed, shards) = (600u64, 70u64, 42u64, 3u64);
    let prefix = format!("purrdf-scale-mixed-v1.seed{seed}.quads{quads}.iris{iris}");

    let (code, _stdout, stderr) = run_make(&[
        "scale-corpus",
        &format!("SCALE_QUADS={quads}"),
        &format!("SCALE_IRIS={iris}"),
        &format!("SCALE_SEED={seed}"),
        &format!("SCALE_SHARDS={shards}"),
        "SCALE_MODE=files",
        &format!("SCALE_OUT={}", out_dir.display()),
        &format!("SCALE_BIN={BENCH}"),
    ]);
    assert_eq!(code, 0, "the run must succeed; stderr:\n{stderr}");
    let run_manifest = out_dir.join(format!("{prefix}.manifest.json"));
    assert!(
        run_manifest.exists(),
        "a run that produced its corpus MUST keep its whole-run manifest; the certificate is \
         half the evidence a capture records"
    );
    let contents = std::fs::read(&run_manifest).expect("read the whole-run manifest");
    let record: serde_json::Value =
        serde_json::from_slice(&contents).expect("the manifest must be valid JSON");
    assert_eq!(
        record["emitted_lines"], quads,
        "the surviving manifest must describe the corpus that was actually produced"
    );

    std::fs::remove_dir_all(&out_dir).expect("cleanup out directory");
}

#[test]
fn make_scale_corpus_refuses_a_directory_as_scale_bin() {
    // The `[[ -x ]]`-accepts-a-directory class, pinned for this lane too: `/tmp` is a directory
    // that certainly carries the execute bit.
    let (code, _stdout, stderr) = run_make(&[
        "scale-corpus",
        "SCALE_QUADS=100",
        "SCALE_IRIS=10",
        "SCALE_SHARDS=2",
        "SCALE_MODE=stream",
        "SCALE_BIN=/tmp",
    ]);
    assert_ne!(
        code, 0,
        "a directory is not an executable, whatever its execute bit says; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("SCALE_BIN='/tmp' is not a regular file"),
        "the refusal must name the knob and the fault; stderr:\n{stderr}"
    );
}

#[test]
fn make_scale_corpus_accepts_a_scale_bin_whose_path_is_unusual_but_real() {
    // The over-refusal counter-check to the test above. `$` and a space are legal in a filename,
    // and a real binary at such a path must RUN — refusing it would be the mirror of executing
    // the wrong one. This also pins that a command-line `SCALE_BIN` beats the environment's,
    // since `run_make` always puts the plain binary in the child's environment.
    let dir = scratch("bin literal ok");
    for name in ["be$nch-corpus", "bench corpus", "bench-corpus"] {
        let copy = dir.join(name);
        std::fs::copy(BENCH, &copy).expect("copy the real binary to an unusual path");
        let mut permissions = std::fs::metadata(&copy)
            .expect("stat the copied binary")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&copy, permissions).expect("keep the copy executable");

        let (code, stdout, stderr) = run_make_retrying_etxtbsy(&[
            "scale-corpus",
            "SCALE_QUADS=100",
            "SCALE_IRIS=10",
            "SCALE_SHARDS=2",
            "SCALE_MODE=stream",
            &format!("SCALE_BIN={}", copy.display()),
        ]);
        assert_eq!(
            code, 0,
            "SCALE_BIN={name:?} names a real executable and must run; stderr:\n{stderr}"
        );
        assert!(
            String::from_utf8_lossy(&stdout).contains("total rows=100"),
            "the run must actually have produced the corpus; stdout:\n{}",
            String::from_utf8_lossy(&stdout)
        );
    }

    std::fs::remove_dir_all(&dir).expect("cleanup scratch directory");
}
