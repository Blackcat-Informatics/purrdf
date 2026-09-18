// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Drives the DOCUMENTED `make lubm` and `make watdiv` entry points, for the laws the three
//! comparison lanes SHARE.
//!
//! `crates/bench/tests/make_scale_corpus.rs` pins the `scale-corpus` lane and nothing pinned the
//! other two, which is exactly how a fix landed in `scale-corpus.sh` and `watdiv-lane.sh` and not
//! in `lubm-lane.sh` while the commit message claimed all three were touched. A law that holds in
//! one lane and not its siblings is not a law, so every test here runs against BOTH of the other
//! lanes and names them in one table.
//!
//! WHY THESE TESTS COST NOTHING, AND WHAT THAT BUYS
//! ================================================
//!
//! Neither lane is a continuous-integration gate: LUBM needs a JRE and a network fetch of a
//! GPL-2.0 generator, WatDiv needs a 58 MB download that expands past a gigabyte. A test that
//! required either would put a network dependency and a JRE dependency inside `make check`, which
//! is not a trade this repository makes for a report-only lane.
//!
//! So the lanes now validate `LUBM_BIN` / `WATDIV_BIN` BEFORE step 1 — before a byte is fetched,
//! before the JRE is looked for, before eight megabytes of RDF/XML are generated on the binary's
//! behalf. That ordering is worth having on its own (a knob error should not cost a download),
//! and it is what lets every refusal below be exercised OFFLINE, in milliseconds, on a machine
//! with no `java` and no cache at all.
//!
//! The over-refusal counter-check is the other half and cannot be free: proving a REAL binary is
//! ACCEPTED means letting the lane run, which means the artifacts. That one test states its
//! precondition and says so out loud when it cannot run; the refusals above never skip.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

/// A monotonically increasing counter, so every call to [`unique_tag`] in this process is
/// distinct even across parallel test threads.
static UNIQUE: AtomicU64 = AtomicU64::new(0);

fn unique_tag() -> String {
    format!(
        "{}-{}",
        std::process::id(),
        UNIQUE.fetch_add(1, Ordering::Relaxed)
    )
}

/// The repository root, resolved from `CARGO_MANIFEST_DIR` (`crates/bench`) rather than the
/// process's current directory, so these tests are independent of how `cargo test` was invoked.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/bench has two ancestors: crates/ and the repository root")
        .to_path_buf()
}

/// A scratch directory unique to this process, created and returned.
fn scratch(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("purrdf-bench-lane-{label}-{}", unique_tag()));
    std::fs::create_dir_all(&dir).expect("create the scratch directory");
    dir
}

/// Runs `make <args>` from the repository root, returning (exit code, stdout, stderr).
///
/// The child's `make` recursion state is scrubbed so this reproduces the documented invocation:
/// a plain shell with no pending `make` recursion.
fn run_make(args: &[&str]) -> (i32, String, String) {
    let output = Command::new("make")
        .current_dir(repo_root())
        .env_remove("MAKEFLAGS")
        .env_remove("MFLAGS")
        .env_remove("MAKELEVEL")
        .args(args)
        .output()
        .expect("spawn make");
    (
        output.status.code().expect("make exited normally"),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// The two lanes that share these laws: the `make` target, the knob that names the binary, and
/// the arena knob, so each test runs in a private arena and never touches `target/lubm` or
/// `target/watdiv`.
const LANES: &[(&str, &str, &str)] = &[
    ("lubm", "LUBM_BIN", "LUBM_OUT"),
    ("watdiv", "WATDIV_BIN", "WATDIV_OUT"),
];

/// Runs one lane with `<bin knob>=<binary>` in a private arena.
fn run_lane_with_bin(lane: &str, bin_knob: &str, out_knob: &str, binary: &str) -> (i32, String) {
    let arena = scratch(&format!("{lane}-arena"));
    let (code, stdout, stderr) = run_make(&[
        lane,
        &format!("{bin_knob}={binary}"),
        &format!("{out_knob}={}", arena.display()),
    ]);
    let _ = std::fs::remove_dir_all(&arena);
    (code, format!("{stdout}\n{stderr}"))
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

// ---------------------------------------------------------------------------------------------
// F2 — `[[ -x ]]` IS NOT A CHECK FOR AN EXECUTABLE, BECAUSE A DIRECTORY CARRIES THE EXECUTE BIT.
//
// `LUBM_BIN=/tmp` passed `lubm-lane.sh`'s only test and the lane went on to generate 8.2 MB of
// RDF/XML on that "binary"'s behalf. `scale-corpus.sh` and `watdiv-lane.sh` had already been
// fixed; `lubm-lane.sh` had not, while the commit claimed all three lanes were touched.
// ---------------------------------------------------------------------------------------------

#[test]
fn every_lane_refuses_a_directory_as_its_binary_knob_and_names_it() {
    for (lane, bin_knob, out_knob) in LANES {
        // A directory that certainly exists and certainly carries the execute bit.
        let (code, combined) = run_lane_with_bin(lane, bin_knob, out_knob, "/tmp");
        assert_ne!(
            code, 0,
            "make {lane} {bin_knob}=/tmp must FAIL: a directory carries the execute bit, so an \
             executability test alone accepts one. output:\n{combined}"
        );
        assert!(
            combined.contains(&format!("{bin_knob}='/tmp' is not a regular file")),
            "make {lane} must refuse the directory BY NAME, saying which knob and which bytes; \
             output:\n{combined}"
        );
        // Refused before step 1, so nothing was fetched and nothing was generated on the
        // strength of a knob that was never a binary.
        assert!(
            !combined.contains("1/7 artifacts"),
            "make {lane} must refuse the knob BEFORE fetching artifacts — a knob error is not \
             worth a download; output:\n{combined}"
        );
    }
}

#[test]
fn every_lane_refuses_a_binary_that_cannot_run_without_blaming_the_corpus() {
    for (lane, bin_knob, out_knob) in LANES {
        // `/bin/false` is a real, regular, executable file that exits non-zero. The execute bit
        // is not proof that a binary RUNS, and this is the case that proves it.
        let (code, combined) = run_lane_with_bin(lane, bin_knob, out_knob, "/bin/false");
        assert_ne!(
            code, 0,
            "make {lane} {bin_knob}=/bin/false must FAIL; output:\n{combined}"
        );
        assert!(
            combined.contains("is not a working purrdf binary")
                && combined.contains("when asked for its version"),
            "make {lane} must say the BINARY did not run, and how it found that out; \
             output:\n{combined}"
        );
        // THE MISDIAGNOSIS ITSELF. Before the fix, `LUBM_BIN=/bin/false` generated 8.2 MB of
        // LUBM data and then died at 5/7 with `purrdf convert failed on ... -- the CLI could
        // not parse LUBM's RDF/XML`. Every clause of that was false: the CLI never ran, the
        // RDF/XML parses, and the fault was in the knob. An operator following it would go
        // hunting a parser bug that does not exist.
        assert!(
            !combined.contains("could not parse LUBM's RDF/XML"),
            "make {lane} must NOT assert a cause it has not established — blaming the corpus \
             for a knob error sends someone hunting a parser bug that does not exist; \
             output:\n{combined}"
        );
        assert!(
            !combined.contains("could not load the WatDiv dataset"),
            "make {lane} must NOT blame the dataset for a binary that never ran; \
             output:\n{combined}"
        );
    }
}

#[test]
fn every_lane_refuses_a_binary_that_produces_nothing_and_publishes_no_digest() {
    // Exits 0 and writes nothing — the empty-certificate shape. NOT `/bin/true`: GNU coreutils
    // `true --version` prints a real version banner, so it passes a version probe and is the
    // WRONG stand-in for "produces nothing" (it is instead a fine stand-in for "produces nothing
    // USEFUL", which `the_lubm_lane_refuses_an_empty_conversion_and_publishes_no_digest` covers).
    let dir = scratch("silent-binary");
    let silent = write_executable(dir.join("silent-purrdf"), "#!/bin/sh\nexit 0\n");
    let silent = silent.display().to_string();

    for (lane, bin_knob, out_knob) in LANES {
        let (code, combined) = run_lane_with_bin(lane, bin_knob, out_knob, &silent);
        assert_ne!(
            code, 0,
            "make {lane} {bin_knob}={silent} must FAIL: exiting 0 is not producing output; \
             output:\n{combined}"
        );
        assert!(
            combined.contains("printed NOTHING"),
            "make {lane} must say the binary produced nothing, in the lane's own voice; \
             output:\n{combined}"
        );
        // THE LOAD-BEARING ASSERTION. `e3b0c442...b855` is the SHA-256 OF THE EMPTY STRING, and
        // `lubm-lane.sh` published it as `sha256(lubm-data.nq) = ...` — the dataset's provenance
        // digest — on a SUCCESS line, for a corpus that did not exist. A digest is a
        // certificate: it must never be emitted for output that is empty or unproduced.
        assert!(
            !combined.contains("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
            "make {lane} must NEVER publish the SHA-256 of the empty string as a digest — that \
             is a certificate for a corpus that does not exist; output:\n{combined}"
        );
        assert!(
            !combined.contains("this digest is the determinism check"),
            "make {lane} must not present any digest as a determinism check for output it did \
             not produce; output:\n{combined}"
        );
        assert!(
            !combined.contains("0 rows, 0 bytes"),
            "make {lane} must never print a zero-row dataset as a SUCCESS line; \
             output:\n{combined}"
        );
    }

    std::fs::remove_dir_all(&dir).expect("cleanup scratch directory");
}

// ---------------------------------------------------------------------------------------------
// F1 (THE FINDING ITSELF) — THE LUBM LANE CERTIFIED A DIGEST FOR A CORPUS THAT DID NOT EXIST.
//
// With a `LUBM_BIN` that exits 0 and writes nothing, `scripts/lubm-lane.sh` printed
// `data:     0 rows, 0 bytes, converted in 56 ms` as a SUCCESS line, published
// `sha256(lubm-data.nq) = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` —
// THE SHA-256 OF THE EMPTY STRING — as the dataset's provenance digest, passed its own
// `converted == owl_count` guard 15 of 15, ran on through step 6, and EXITED 0 with a full
// report of fourteen BAD-RESULTS rows.
//
// This is the direct regression test for that. It needs a binary that is CREDIBLE — it answers
// `--version` like the real CLI — and silently drops every conversion, which is the only shape
// that reaches step 5. Reaching step 5 means the LUBM generator, hence a JRE and the pinned
// artifacts; the test says so rather than pretending to have checked when it cannot run.
// ---------------------------------------------------------------------------------------------

#[test]
fn the_lubm_lane_refuses_an_empty_conversion_and_publishes_no_digest() {
    let Some(real) = prebuilt_purrdf() else {
        eprintln!(
            "NOT RUN: target/release/purrdf is not built, so no credible --version can be \
             delegated. Run `cargo build --release -p purrdf-cli` and re-run."
        );
        return;
    };
    if Command::new("java").arg("-version").output().is_err() {
        eprintln!(
            "NOT RUN: no `java` on PATH, so the LUBM generator cannot produce the RDF/XML \
                   this law is about."
        );
        return;
    }

    let dir = scratch("silent-drop");
    // Credible at `--version`, and every conversion produces an EMPTY file. This is exactly what
    // a CLI that silently drops its input looks like from outside.
    let stand_in = write_executable(
        dir.join("silent-drop-purrdf"),
        &format!(
            "#!/bin/sh\n\
             if [ \"$1\" = \"--version\" ]; then exec \"{}\" --version; fi\n\
             for a in \"$@\"; do last=\"$a\"; done\n\
             case \"$last\" in *.nq) : >\"$last\" ;; esac\n\
             exit 0\n",
            real.display()
        ),
    );

    let arena = scratch("silent-drop-arena");
    let (code, stdout, stderr) = run_make(&[
        "lubm",
        &format!("LUBM_BIN={}", stand_in.display()),
        &format!("LUBM_OUT={}", arena.display()),
    ]);
    let combined = format!("{stdout}\n{stderr}");

    if combined.contains("artifact acquisition failed") {
        eprintln!(
            "NOT RUN past step 1: the pinned LUBM artifacts are neither cached nor reachable."
        );
    } else {
        assert_ne!(
            code, 0,
            "a CLI that converts everything to nothing must FAIL the lane. Before the fix this \
             exited 0 with a full report. output:\n{combined}"
        );
        assert!(
            combined.contains("is EMPTY"),
            "the lane must say the conversion was empty, in its own voice and naming the file; \
             output:\n{combined}"
        );
        // THE LOAD-BEARING ASSERTIONS: no success line, and above all no digest.
        assert!(
            !combined.contains("0 rows, 0 bytes"),
            "`data: 0 rows, 0 bytes, converted in N ms` must never be printed as a SUCCESS line; \
             output:\n{combined}"
        );
        assert!(
            !combined.contains("sha256(lubm-data.nq)"),
            "NO DIGEST may be published for a dataset that does not exist — a digest is a \
             certificate; output:\n{combined}"
        );
        assert!(
            !combined.contains("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
            "the SHA-256 of the empty string must never appear as a dataset's provenance; \
             output:\n{combined}"
        );
        assert!(
            !combined.contains("SUMMARY"),
            "no report may be printed at all for a run with no corpus; output:\n{combined}"
        );
        // And the death must be the lane's, not a bare `cat: '': No such file or directory` six
        // steps later that names neither the lane, the knob, nor purrdf.
        assert!(
            combined.contains("lubm-lane:"),
            "the failure must carry the lane's voice; output:\n{combined}"
        );
    }

    std::fs::remove_dir_all(&dir).expect("cleanup scratch directory");
    let _ = std::fs::remove_dir_all(&arena);
}

#[test]
fn every_lane_refuses_a_binary_knob_that_does_not_exist_and_quotes_the_bytes_back() {
    for (lane, bin_knob, out_knob) in LANES {
        let missing = format!("/nonexistent-{}/purrdf", unique_tag());
        let (code, combined) = run_lane_with_bin(lane, bin_knob, out_knob, &missing);
        assert_ne!(
            code, 0,
            "make {lane} {bin_knob}={missing} must FAIL; output:\n{combined}"
        );
        assert!(
            combined.contains(&format!("{bin_knob}='{missing}' does not exist")),
            "make {lane} must quote back the bytes it actually used, so a typo is visible; \
             output:\n{combined}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// OVER-REFUSAL — "a refusal is a claim too, prove it" (`CLAUDE.md`).
//
// Everything above tightens validation, which is exactly where over-refusal appears: a lane that
// refuses whatever it is given passes every test above and is useless. So a REAL `purrdf` must
// still be accepted when it is reached by an awkward but perfectly legal path.
//
// This is the one test here that cannot be free. Proving acceptance means letting the lane past
// validation, and past validation is the artifact fetch. It therefore states its precondition
// and reports loudly when it cannot run, rather than pretending to have checked.
// ---------------------------------------------------------------------------------------------

/// The `purrdf` CLI this repository builds, if it is already built. Not built on demand: a test
/// that triggers a release build of the CLI would dominate the suite's runtime.
fn prebuilt_purrdf() -> Option<PathBuf> {
    let candidate = repo_root().join("target/release/purrdf");
    candidate.is_file().then_some(candidate)
}

#[test]
fn the_lubm_lane_accepts_a_real_binary_reached_by_an_awkward_but_legal_path() {
    let Some(real) = prebuilt_purrdf() else {
        eprintln!(
            "NOT RUN: target/release/purrdf is not built, so there is no real binary to prove \
             acceptance with. Run `cargo build --release -p purrdf-cli` (or `make lubm` once) \
             and re-run. The refusal tests in this file do not skip and have run."
        );
        return;
    };

    // One path that is a SYMLINK, is RELATIVE to the repository root (the lane's own working
    // directory), and contains a SPACE — all three properties at once, so a single lane run
    // covers all three counter-checks.
    let arena = repo_root().join(format!("target/lane over refusal {}", unique_tag()));
    std::fs::create_dir_all(&arena).expect("create the space-containing arena");
    let link = arena.join("purrdf link");
    std::os::unix::fs::symlink(&real, &link).expect("symlink the real binary");
    let relative = link
        .strip_prefix(repo_root())
        .expect("the arena is under the repository root")
        .to_path_buf();

    let (code, stdout, stderr) = run_make(&[
        "lubm",
        &format!("LUBM_BIN={}", relative.display()),
        &format!("LUBM_OUT={}", arena.join("run").display()),
    ]);
    let combined = format!("{stdout}\n{stderr}");

    if combined.contains("artifact acquisition failed") {
        eprintln!(
            "NOT RUN past step 1: the pinned LUBM artifacts are neither cached nor reachable, \
             so the lane could not proceed to prove acceptance. Validation itself was passed — \
             no knob diagnostic was emitted."
        );
    } else {
        assert_eq!(
            code, 0,
            "a REAL purrdf reached through a relative, space-containing SYMLINK is a legal way \
             to name the binary and the lane must run; refusing it would be the mirror of \
             accepting a directory. output:\n{combined}"
        );
        assert!(
            stdout.contains("queries executed   14 of 14"),
            "the accepted binary must actually have answered the workload; stdout:\n{stdout}"
        );
    }

    // Whatever happened downstream, the BINARY must not have been the complaint.
    assert!(
        !combined.contains("LUBM_BIN="),
        "no knob diagnostic may be emitted for a real binary at a legal path; \
         output:\n{combined}"
    );
    assert!(
        !combined.contains("is not a working purrdf binary"),
        "a real purrdf reached by symlink must be recognised as one; output:\n{combined}"
    );

    std::fs::remove_dir_all(&arena).expect("cleanup arena");
}

#[test]
fn a_lane_binary_knob_accepts_a_wrapper_that_answers_for_its_version() {
    // The narrowest over-refusal counter-check, and it needs no artifacts at all: the validation
    // added above must accept anything that behaves like the CLI, not just the one file this
    // repository builds. A shell wrapper is the ordinary way an operator pins flags or a
    // profiler around the binary under test, and refusing it would break that outright.
    //
    // Acceptance is observed as "the lane got PAST validation": every refusal above is emitted
    // BEFORE step 1 by construction, so a run that reaches step 3 accepted the binary.
    //
    // The arena knob is then pointed at an UNCREATABLE directory so each lane stops at its
    // `mkdir` — which both lanes reach before they unzip or extract anything. That keeps this
    // test to a cache-hit artifact check instead of a gigabyte of bzip2 into the temp directory,
    // and it doubles as the check that an unusable arena is itself a lane diagnostic naming its
    // own knob rather than a bare `mkdir: cannot create directory ...`.
    let dir = scratch("wrapper-accepted");
    let wrapper = write_executable(
        dir.join("purrdf wrapper"),
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'purrdf 9.9.9'; exit 0; fi\nexit 0\n",
    );
    let unusable = format!("/nonexistent-arena-{}/deeper/arena", unique_tag());

    for (lane, bin_knob, out_knob) in LANES {
        let (code, stdout, stderr) = run_make(&[
            lane,
            &format!("{bin_knob}={}", wrapper.display()),
            &format!("{out_knob}={unusable}"),
        ]);
        let combined = format!("{stdout}\n{stderr}");
        if combined.contains("artifact acquisition failed") {
            eprintln!(
                "NOT RUN past step 1 for {lane}: the pinned artifacts are neither cached nor \
                 reachable. Validation itself was passed — no knob diagnostic was emitted."
            );
        }

        assert!(
            !combined.contains(&format!("{bin_knob}='")),
            "make {lane}: a wrapper that answers --version is a legal binary and must pass \
             validation — an operator wrapping the CLI in a profiler or a flag-pinning script is \
             the ordinary case, and refusing it would be the mirror of accepting a directory; \
             output:\n{combined}"
        );
        assert!(
            !combined.contains("is not a working purrdf binary"),
            "make {lane}: a wrapper that answers --version must not be called unworkable; \
             output:\n{combined}"
        );
        assert!(
            !combined.contains("printed NOTHING"),
            "make {lane}: this wrapper DID print a version; output:\n{combined}"
        );
        assert_ne!(
            code, 0,
            "make {lane}: the arena is uncreatable, so the run must still fail — on the ARENA, \
             not on the binary; output:\n{combined}"
        );
        assert!(
            combined.contains(&format!("{out_knob}='{unusable}'")),
            "make {lane}: an uncreatable arena must be a LANE diagnostic naming {out_knob} and \
             quoting the bytes back, not a bare `mkdir: cannot create directory ...`; \
             output:\n{combined}"
        );
    }

    std::fs::remove_dir_all(&dir).expect("cleanup scratch directory");
}
