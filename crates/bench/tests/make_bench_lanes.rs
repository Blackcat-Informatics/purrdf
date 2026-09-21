// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Drives the DOCUMENTED `make lubm` and `make watdiv` entry points, for the laws the three
//! comparison lanes SHARE.
//!
//! `crates/bench/tests/make_scale_corpus.rs` pins the `scale-corpus` lane and nothing pinned the
//! other two, which is exactly how a repair reached `scale-corpus.sh` and `watdiv-lane.sh` and
//! left `lubm-lane.sh` carrying the defect. A law that holds in one lane and not its siblings is
//! not a law, so every test here runs against BOTH of the other lanes and names them in one
//! table.
//!
//! WHY THESE TESTS COST NOTHING, AND WHAT THAT BUYS
//! ================================================
//!
//! Neither lane is a continuous-integration gate: LUBM needs a JRE and a network fetch of a
//! GPL-2.0 generator, WatDiv needs a 58 MB download that expands past a gigabyte. A test that
//! required either would put a network dependency and a JRE dependency inside `make check`, which
//! is not a trade this repository makes for a report-only lane.
//!
//! So the lanes validate `LUBM_BIN` / `WATDIV_BIN` BEFORE step 1 — before a byte is fetched,
//! before the JRE is looked for, before eight megabytes of RDF/XML are generated on the binary's
//! behalf. That ordering is worth having on its own (a knob error should not cost a download),
//! and it is what lets every test in this file run OFFLINE, in milliseconds, on a machine with no
//! `java` and no cache at all.
//!
//! NO TEST HERE SKIPS, AND THAT IS A DESIGN CONSTRAINT RATHER THAN A PREFERENCE
//! ===========================================================================
//!
//! Two tests in this file used to `return` early when `java` was absent or when
//! `target/release/purrdf` had not been built — and one of them was the direct regression test for
//! the digest-over-a-nonexistent-corpus defect. No continuous-integration job installs a JRE or
//! builds the CLI before `cargo test --workspace`, so both were unconditional no-ops there. They
//! announced it with `eprintln!`, which libtest CAPTURES for a passing test: the announcement was
//! invisible and the suite reported two more passes than it had run.
//!
//! A test that cannot run in an environment is restructured so it does not NEED that environment.
//! Both of the laws in question are about the BINARY, and the binary is validated before step 1,
//! so both are reachable with a stand-in and an arena that cannot be created — no JRE, no network,
//! no build. The stand-ins are shell scripts that behave the way the lane requires: they answer
//! `--version`, and they either perform the lane's conversion probe or deliberately fail it. That
//! is what a wrapper an operator writes does too, which is why accepting them is the over-refusal
//! counter-check rather than a weakening.
//!
//! The REAL `purrdf` binary is proved acceptable by `make lubm` and `make watdiv` themselves,
//! which is where a real binary belongs; this file pins the laws, not the build.

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

// WHAT THE NEGATIVE ASSERTIONS BELOW CAN AND CANNOT CATCH.
//
// Every test in this file stops the lane before step 1 — at the binary probe or at a
// knob validator; six of the fourteen also point the arena somewhere uncreatable, and
// the rest do not need to because the probe comes first — so a
// `!contains("<a step-5 message>")` assertion is TRIVIALLY TRUE
// for the run it is made about. Nine such assertions live here. They are not controls
// for the behaviour their surrounding test is named for, and an earlier audit of this
// file wrongly reported that there was one.
//
// They are kept, with a narrower purpose stated: each guards against its message
// MOVING EARLIER. If a future edit blamed the corpus, published a digest, or wrote a
// stamp before the binary was proved to work, that assertion fires. That is a real
// regression guard and a small one; it is not evidence about step 5, and no test in
// this file can be.
//
// One assertion was worse than trivially true: it named a string
// (`could not load the WatDiv dataset`) that appears nowhere in `scripts/` and is
// never assembled at runtime, so it could not fail for any input at all. It now names
// the message the lane really emits. The pack-magic refusal reads similarly and is
// NOT inert: `lane_require_magic` assembles it from its `format` argument at runtime.
//
// The step-5 laws themselves are proved in `lane_common_laws.rs`, which calls the
// shared helpers directly instead of driving a lane.

/// The two lanes this file can drive: the `make` target, the knob that names the binary, and
/// the arena knob, so each test runs in a private arena.
///
/// `scale-corpus` shares the same laws through the same helpers and is deliberately NOT
/// here: it probes a `bench-corpus` binary that must print a whole-run manifest, so the
/// purrdf-shaped stand-ins below cannot satisfy its contract — adding it makes four tests
/// fail on the stand-in rather than on the lane. Its own laws are covered in
/// `make_scale_corpus.rs` (directory-as-binary, printed-NOTHING, the write guard, the
/// arena knob), and the probe law that file did not cover is now proved once for every
/// lane in `lane_common_laws.rs`.
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

/// A path no process can create an arena at, INCLUDING one running as uid 0.
///
/// `/nonexistent-<tag>/deeper` is uncreatable for an ordinary user and creatable
/// for root, and test containers commonly run as root. There, the lane would pass
/// arena creation and fall through into the artifacts step -- downloading tens of
/// megabytes, wanting a JDK for the LUBM lane, and contradicting the no-network
/// property these tests are built on -- while also leaving a directory under `/`.
///
/// A REGULAR FILE as the parent makes `mkdir` fail with `ENOTDIR` for every uid,
/// because the kernel's check is about the parent's type rather than about
/// permission. It is also confined to the scratch directory.
fn uncreatable_arena(label: &str) -> (String, PathBuf) {
    let root = scratch(label);
    let blocker = root.join("not-a-directory");
    std::fs::write(&blocker, b"").expect("write the file that blocks arena creation");
    (format!("{}/deeper", blocker.display()), root)
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
// `[[ -x ]]` IS NOT A CHECK FOR AN EXECUTABLE, BECAUSE A DIRECTORY CARRIES THE EXECUTE BIT.
//
// `LUBM_BIN=/tmp` passed `lubm-lane.sh`'s only test and the lane went on to generate 8.2 MB of
// RDF/XML on that "binary"'s behalf. `scale-corpus.sh` and `watdiv-lane.sh` already refused it;
// `lubm-lane.sh` did not. All three share one implementation of the check, in
// `scripts/lane-common.sh`.
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
            !combined.contains("loading the WatDiv dataset into a pack"),
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
            !combined.contains(EMPTY_STRING_SHA256),
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
// EACH LANE CERTIFIED AN ARTIFACT ITS BINARY NEVER PRODUCED.
//
// With a `LUBM_BIN` that exits 0 and writes nothing, `scripts/lubm-lane.sh` printed
// `data:     0 rows, 0 bytes` as a SUCCESS line, published
// `sha256(lubm-data.nq) = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` —
// THE SHA-256 OF THE EMPTY STRING — as the dataset's provenance digest, passed its own
// `converted == owl_count` guard 15 of 15, ran on through step 6, and EXITED 0 with a full
// report of fourteen BAD-RESULTS rows.
//
// `scripts/watdiv-lane.sh` had the same shape one artifact over: a CLI that wrote eight bytes of
// something that was not a pack passed the lane's `-s` check, was STAMPED as this dataset's pack,
// and every later run then printed "reusing the one already stamped with this dataset digest and
// this binary" and queried those eight bytes.
//
// Both are now refused BEFORE step 1, because each lane's binary check is a ROUND TRIP rather than
// a `--version` call: the binary is handed one triple and must hand back the artifact the lane is
// about. That is what makes this test reachable with no JRE, no network and no build — the fault
// is in the binary, and the binary is checked before anything is fetched.
// ---------------------------------------------------------------------------------------------

/// WRITE ONLY WHERE A LANE ASKED — the shared preamble both stand-ins use before writing.
///
/// A stand-in wrote to the last argument it was given, whatever it was. Pointed at a lane whose
/// final argument is a FLAG, it deposited a 71-byte file named `--manifest` at the repository
/// root — tracked, and breaking every `*` glob there until someone noticed.
///
/// The first repair refused a flag-shaped last argument and was claimed to close the hole. It did
/// not: BOTH lanes pass the query TEXT as the final argument of `query`, so
/// `standin query --data … --results-format json 'SELECT ?s WHERE { ?s ?p ?o } LIMIT 1'` still
/// wrote 71 bytes into the CWD under that name — the identical artifact, and a name the
/// tracked-path gate accepts. Meanwhile the sibling `convert_stand_in` had no guard at all. The
/// law was stated in a comment over a two-shape blocklist, in a file whose opening paragraph warns
/// about laws that hold in one place and not its sibling.
///
/// So the guard is derived from what a real destination IS, not from what a wrong one looks like,
/// and it lives in one place both stand-ins interpolate. Every property below is true of every
/// destination either lane passes (`convert --from … --to … IN OUT`, with the probe's pair inside
/// `LANE_TMP` and the corpus pair inside the arena):
///
/// 1. only `convert` writes a file — `query` writes to standard output, and its last argument is
///    the query text, which is how the 71 bytes got out;
/// 2. the destination is neither empty nor flag-shaped;
/// 3. it is ABSOLUTE — a lane never passes a relative destination, and a relative one is what
///    lands in whatever directory the harness happened to be in;
/// 4. its parent directory already exists — a lane creates its arena before converting into it.
///
/// A SPARQL query fails 1, 3 and 4; `--manifest` fails 1, 2 and 3.
const STAND_IN_GUARD: &str = r#"
if [ "$1" = "--version" ]; then echo 'purrdf 9.9.9 (stand-in)'; exit 0; fi
if [ "$1" != "convert" ]; then
  echo "stand-in: '$1' writes no file; only convert does" >&2
  exit 0
fi
for a in "$@"; do last="$a"; done
case "$last" in
  ""|-*) echo "stand-in refusing to write to non-path argument: $last" >&2; exit 3 ;;
  /*) ;;
  *) echo "stand-in refusing to write to a relative destination: $last" >&2; exit 3 ;;
esac
if [ ! -d "$(dirname "$last")" ]; then
  echo "stand-in refusing to write where no directory exists: $last" >&2
  exit 3
fi
"#;

/// A stand-in `purrdf` that answers `--version` and performs `convert` by writing `payload` to
/// the destination. `payload` is a `printf` format string run through `/bin/sh`, so `""` is the
/// silent-drop shape and `PURRPCK1...` is a credible pack.
fn convert_stand_in(path: PathBuf, payload: &str) -> PathBuf {
    write_executable(
        path,
        &format!(
            r#"#!/bin/sh
{STAND_IN_GUARD}
printf '%s' '{payload}' >"$last"
exit 0
"#
        ),
    )
}

/// A stand-in that converts CREDIBLY for both lanes: one N-Quads row for `lubm`, a pack whose
/// magic is the real one for `watdiv`. Chosen by the destination's extension, exactly as the
/// lane's own probe names it.
fn credible_stand_in(path: PathBuf) -> PathBuf {
    write_executable(
        path,
        &format!(
            r#"#!/bin/sh
{STAND_IN_GUARD}
case "$last" in
  *.pack) printf '{PACK_MAGIC}and then some payload bytes' >"$last" ;;
  *) printf '%s\n' '<http://example.org/s> <http://example.org/p> <http://example.org/o> .' >"$last" ;;
esac
exit 0
"#
        ),
    )
}

/// The 8-byte magic every native pack begins with, as `scripts/watdiv-lane.sh` spells it.
const PACK_MAGIC: &str = "PURRPCK1";

/// The SHA-256 of the empty string. If this ever appears in a lane's output as a digest, the lane
/// has certified nothing at all.
const EMPTY_STRING_SHA256: &str =
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

#[test]
fn every_lane_refuses_a_binary_that_converts_everything_to_nothing_and_publishes_no_digest() {
    // Credible at `--version`, and every conversion produces an EMPTY file. This is exactly what a
    // CLI that silently drops its input looks like from outside, and it is the shape that reached
    // step 5 of the LUBM lane and published a digest for a corpus that was never written.
    //
    // It needs NO JRE, NO network and NO built binary: the lane's own conversion probe runs before
    // step 1, so the refusal happens on the knob rather than six steps later.
    let dir = scratch("silent-drop");
    let stand_in = convert_stand_in(dir.join("silent-drop-purrdf"), "");

    for (lane, bin_knob, out_knob) in LANES {
        let (code, combined) =
            run_lane_with_bin(lane, bin_knob, out_knob, &stand_in.display().to_string());

        assert_ne!(
            code, 0,
            "make {lane}: a CLI that converts everything to nothing must FAIL the lane. Before \
             the fix the LUBM lane exited 0 with a full report. output:\n{combined}"
        );
        assert!(
            combined.contains(&format!("{bin_knob}='{}'", stand_in.display())),
            "make {lane}: the refusal must name the knob and quote the bytes it used; \
             output:\n{combined}"
        );
        // Refused BEFORE step 1: nothing fetched, no JRE looked for, no corpus generated on the
        // strength of a binary that produces nothing.
        assert!(
            !combined.contains("1/7 artifacts"),
            "make {lane}: a binary that cannot convert must be refused before a byte is fetched; \
             output:\n{combined}"
        );
        // THE LOAD-BEARING ASSERTIONS: no success line, and above all no digest.
        assert!(
            !combined.contains("0 rows, 0 bytes"),
            "make {lane}: `data: 0 rows, 0 bytes, converted in N ms` must never be printed as a \
             SUCCESS line; output:\n{combined}"
        );
        assert!(
            !combined.contains("sha256(lubm-data.nq)"),
            "make {lane}: NO DIGEST may be published for a dataset that does not exist — a digest \
             is a certificate; output:\n{combined}"
        );
        assert!(
            !combined.contains(EMPTY_STRING_SHA256),
            "make {lane}: the SHA-256 of the empty string must never appear as an artifact's \
             provenance; output:\n{combined}"
        );
        assert!(
            !combined.contains("SUMMARY"),
            "make {lane}: no report may be printed at all for a run with no corpus; \
             output:\n{combined}"
        );
        // And the death must be the lane's, not a bare `cat: '': No such file or directory` six
        // steps later that names neither the lane, the knob, nor purrdf.
        assert!(
            combined.contains(&format!("{lane}-lane:")),
            "make {lane}: the failure must carry the lane's voice; output:\n{combined}"
        );
    }

    std::fs::remove_dir_all(&dir).expect("cleanup scratch directory");
}

#[test]
fn the_watdiv_lane_refuses_a_pack_that_is_not_a_pack_and_stamps_nothing() {
    // NON-EMPTY IS NOT "IS A PACK". Eight bytes of something else passed the lane's emptiness
    // check, got a `.pack-stamp`, survived the failure that followed, and were reused by every
    // later run under "reusing the one already stamped with this dataset digest and this binary".
    let dir = scratch("not-a-pack");
    let stand_in = convert_stand_in(dir.join("eight-byte-purrdf"), "NOTAPACK");
    let arena = scratch("not-a-pack-arena");

    let (code, stdout, stderr) = run_make(&[
        "watdiv",
        &format!("WATDIV_BIN={}", stand_in.display()),
        &format!("WATDIV_OUT={}", arena.display()),
    ]);
    let combined = format!("{stdout}\n{stderr}");

    assert_ne!(
        code, 0,
        "eight bytes that are not a pack must FAIL the lane; output:\n{combined}"
    );
    assert!(
        combined.contains("is not a purrdf pack"),
        "the refusal must say the artifact is not what it claims to be, not merely that it is \
         small; output:\n{combined}"
    );
    assert!(
        !combined.contains("1/7 artifacts"),
        "a binary that cannot produce a pack must be refused before the 58 MB download; \
         output:\n{combined}"
    );
    assert!(
        !arena.join(".pack-stamp").exists(),
        "NO STAMP may be written for an artifact that is not a pack — a stamp is what a LATER run \
         consults instead of loading"
    );

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
// refuses whatever it is given passes every test above and is useless. So a binary that BEHAVES
// like the CLI must still be accepted, including when it is reached by an awkward but perfectly
// legal path — a relative, space-containing symlink is the shape an operator actually produces.
//
// Acceptance is observed as "the lane got PAST the binary check and on to the NEXT knob". Both
// lanes validate the binary and then create the arena, both before step 1, so pointing the arena
// knob at an uncreatable path stops the run between the two: a failure that names the ARENA is
// proof the BINARY was accepted. Nothing is fetched, so this costs no network and no artifacts —
// which matters, because letting the run reach step 1 would put a 58 MB download inside
// `cargo test --workspace`.
//
// The REAL `purrdf` binary is exercised by `make lubm` and `make watdiv` themselves, which is
// where a real binary belongs. These tests pin the LAW.
// ---------------------------------------------------------------------------------------------

#[test]
fn every_lane_accepts_a_credible_binary_at_an_awkward_but_legal_path() {
    let dir = scratch("awkward-path");
    let real = credible_stand_in(dir.join("purrdf-stand-in"));

    for (lane, bin_knob, out_knob) in LANES {
        // One path that is a SYMLINK, is RELATIVE to the repository root (the lane's own working
        // directory), and contains a SPACE — all three properties at once, so a single lane run
        // covers all three counter-checks.
        let arena = repo_root().join(format!("target/lane over refusal {}", unique_tag()));
        std::fs::create_dir_all(&arena).expect("create the space-containing arena");
        let link = arena.join("purrdf link");
        std::os::unix::fs::symlink(&real, &link).expect("symlink the stand-in");
        let relative = link
            .strip_prefix(repo_root())
            .expect("the arena is under the repository root")
            .to_path_buf();

        // The arena is pointed somewhere uncreatable so the run stops on the NEXT knob rather
        // than entering step 1 and its download.
        let (unusable, unusable_root) = uncreatable_arena("arena-blocked");
        let (code, stdout, stderr) = run_make(&[
            lane,
            &format!("{bin_knob}={}", relative.display()),
            &format!("{out_knob}={unusable}"),
        ]);
        let combined = format!("{stdout}\n{stderr}");
        let _ = std::fs::remove_dir_all(&unusable_root);

        assert!(
            combined.contains(&format!("{out_knob}='{unusable}'")),
            "make {lane}: a binary that answers --version and performs the lane's conversion, \
             reached through a relative, space-containing SYMLINK, is a legal way to name it; the \
             run must get past it and fail on the ARENA instead. Refusing the path would be the \
             mirror of accepting a directory. exit {code}, output:\n{combined}"
        );
        assert!(
            !combined.contains(&format!("{bin_knob}=")),
            "make {lane}: no knob diagnostic may be emitted for a credible binary at a legal \
             path; output:\n{combined}"
        );
        assert!(
            !combined.contains("is not a working purrdf binary"),
            "make {lane}: a credible binary reached by symlink must be recognised as one; \
             output:\n{combined}"
        );
        assert!(
            !combined.contains("printed NOTHING"),
            "make {lane}: this stand-in DID print a version; output:\n{combined}"
        );
        assert!(
            !combined.contains("NO N-QUADS AT ALL") && !combined.contains("is not a purrdf pack"),
            "make {lane}: this stand-in DID produce the artifact the probe asked for; \
             output:\n{combined}"
        );
        assert!(
            !combined.contains("1/7 artifacts"),
            "make {lane}: the arena knob is unusable, so the run must stop before step 1 — this \
             test must never reach a download; output:\n{combined}"
        );

        std::fs::remove_dir_all(&arena).expect("cleanup arena");
    }

    std::fs::remove_dir_all(&dir).expect("cleanup scratch directory");
}

#[test]
fn every_lane_names_its_arena_knob_when_the_arena_cannot_be_created() {
    // The arena is a knob too, and a knob error is not worth a download: both lanes create it
    // BEFORE step 1 now, for the same reason they validate the binary there. So this runs
    // everywhere, with no network and no artifacts, and it doubles as the over-refusal check that
    // a wrapper an operator writes around the CLI is accepted — the run must fail on the ARENA,
    // never on the binary.
    let dir = scratch("arena-unusable");
    let wrapper = credible_stand_in(dir.join("purrdf wrapper"));
    let (unusable, unusable_root) = uncreatable_arena("arena-blocked");

    for (lane, bin_knob, out_knob) in LANES {
        let (code, stdout, stderr) = run_make(&[
            lane,
            &format!("{bin_knob}={}", wrapper.display()),
            &format!("{out_knob}={unusable}"),
        ]);
        let combined = format!("{stdout}\n{stderr}");

        assert!(
            !combined.contains(&format!("{bin_knob}='")),
            "make {lane}: a wrapper that answers --version and performs the conversion is a legal \
             binary and must pass validation — an operator wrapping the CLI in a profiler or a \
             flag-pinning script is the ordinary case, and refusing it would be the mirror of \
             accepting a directory; output:\n{combined}"
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
        assert!(
            !combined.contains("1/7 artifacts"),
            "make {lane}: an unusable arena must be refused BEFORE a byte is fetched, exactly as \
             an unusable binary is; output:\n{combined}"
        );
    }

    // Removed after BOTH lanes have run: the blocker file IS the uncreatable
    // arena, so deleting it inside the loop would let the second lane create one
    // and fall through into step 1 and its download.
    let _ = std::fs::remove_dir_all(&unusable_root);
    std::fs::remove_dir_all(&dir).expect("cleanup scratch directory");
}

// ---------------------------------------------------------------------------------------------
// A LANE'S OWN KNOBS ARE LAWS TOO, AND THEY WERE THE UNTESTED HALF.
//
// The tests above cover the laws that live in `scripts/lane-common.sh`, so they cover every lane
// at once. Each lane ALSO validates its own knobs, before step 1 and before any download — and
// none of that had a test on either side. `docs/BENCHMARKS.md` states the WatDiv scale rule as a
// guarantee ("only `10M` is pinned; any other value is refused by name"), and a guarantee with no
// test is a sentence.
//
// Both halves are checked here, because a refusal is a claim in two directions. The valid
// neighbour is the one that actually bites: a knob check written slightly too eagerly refuses
// every run, and a test that only ever feeds it bad input passes exactly as happily.
//
// These cost nothing to run. Every one of them fires before the artifacts step, so there is no
// network, no JRE and no 58 MB download anywhere in this section — the same property that lets
// the binary-knob tests above run in the gate.
// ---------------------------------------------------------------------------------------------

/// Runs a lane with extra knob assignments in a private arena, returning the exit code and output.
fn run_lane_with_knobs(lane: &str, out_knob: &str, knobs: &[String]) -> (i32, String) {
    let arena = scratch(&format!("{lane}-knobs"));
    let mut args: Vec<String> = vec![lane.to_string()];
    args.extend(knobs.iter().cloned());
    args.push(format!("{out_knob}={}", arena.display()));
    let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
    let (code, stdout, stderr) = run_make(&borrowed);
    let _ = std::fs::remove_dir_all(&arena);
    (code, format!("{stdout}\n{stderr}"))
}

#[test]
fn the_watdiv_lane_refuses_an_unpinned_scale_by_name() {
    let (code, combined) =
        run_lane_with_knobs("watdiv", "WATDIV_OUT", &["WATDIV_SCALE=100M".to_string()]);

    assert_ne!(
        code, 0,
        "make watdiv: an unpinned scale must be refused, never silently run as 10M — reporting a \
         number for the wrong corpus under the right name is the failure this guard exists for; \
         output:\n{combined}"
    );
    assert!(
        combined.contains("WATDIV_SCALE='100M' is not pinned"),
        "make watdiv: the refusal must quote the value back, so an operator sees WHICH scale was \
         rejected rather than a bare usage error; output:\n{combined}"
    );
    assert!(
        !combined.contains("1/7 artifacts"),
        "make watdiv: a knob this lane cannot honour must be refused BEFORE the artifacts step — \
         a rejected run must not first download tens of megabytes; output:\n{combined}"
    );
}

#[test]
fn the_watdiv_lane_refuses_a_seed_that_is_not_a_number() {
    let (code, combined) =
        run_lane_with_knobs("watdiv", "WATDIV_OUT", &["WATDIV_SEED=abc".to_string()]);

    assert_ne!(
        code, 0,
        "make watdiv: a non-numeric seed must be refused; output:\n{combined}"
    );
    assert!(
        combined.contains("WATDIV_SEED") && combined.contains("abc"),
        "make watdiv: the refusal must name the knob and quote the value; output:\n{combined}"
    );
    assert!(
        !combined.contains("1/7 artifacts"),
        "make watdiv: a bad seed must be caught before the artifacts step; output:\n{combined}"
    );
}

#[test]
fn the_watdiv_lane_accepts_the_pinned_scale_and_an_ordinary_seed() {
    // THE OVER-REFUSAL HALF. `WATDIV_SCALE=10M` is the pinned value and `WATDIV_SEED=7` is an
    // ordinary seed, so neither may be refused. The run is still made to fail — on an uncreatable
    // arena — because that is what proves execution got PAST the knob checks rather than merely
    // that it exited non-zero for some reason of its own.
    let (unusable, unusable_root) = uncreatable_arena("arena-blocked");
    let (code, stdout, stderr) = run_make(&[
        "watdiv",
        "WATDIV_SCALE=10M",
        "WATDIV_SEED=7",
        &format!("WATDIV_OUT={unusable}"),
    ]);
    let combined = format!("{stdout}\n{stderr}");
    let _ = std::fs::remove_dir_all(&unusable_root);

    assert!(
        !combined.contains("is not pinned"),
        "make watdiv: 10M IS the pinned scale and must not be refused; output:\n{combined}"
    );
    assert!(
        !combined.contains("WATDIV_SEED must be"),
        "make watdiv: 7 is an ordinary seed and must not be refused — a seed check that rejects \
         valid seeds is the mirror of one that accepts junk; output:\n{combined}"
    );
    assert_ne!(
        code, 0,
        "make watdiv: the arena is uncreatable, so this run must still fail — the point is that it \
         fails on the ARENA, having accepted both knobs; output:\n{combined}"
    );
    assert!(
        combined.contains("WATDIV_OUT="),
        "make watdiv: the failure must name the arena knob, which is what shows the knob checks \
         passed and execution reached arena creation; output:\n{combined}"
    );
}

#[test]
fn the_lubm_lane_refuses_a_zero_university_count_and_accepts_one() {
    // `LUBM_UNIVERSITIES=0` generates no corpus at all, so it is refused by name. The valid
    // neighbour is 1 — the default and the count the published LUBM answers are quoted for — which
    // must reach arena creation instead.
    let (code, combined) =
        run_lane_with_knobs("lubm", "LUBM_OUT", &["LUBM_UNIVERSITIES=0".to_string()]);
    assert_ne!(
        code, 0,
        "make lubm: zero universities is an empty corpus, which converts cleanly and answers every \
         query 0 — it must be refused, not measured; output:\n{combined}"
    );
    assert!(
        combined.contains("LUBM_UNIVERSITIES"),
        "make lubm: the refusal must name the knob; output:\n{combined}"
    );

    let (unusable, unusable_root) = uncreatable_arena("arena-blocked");
    let (code, stdout, stderr) = run_make(&[
        "lubm",
        "LUBM_UNIVERSITIES=1",
        "LUBM_SEED=0",
        &format!("LUBM_OUT={unusable}"),
    ]);
    let combined = format!("{stdout}\n{stderr}");
    let _ = std::fs::remove_dir_all(&unusable_root);
    assert!(
        !combined.contains("LUBM_UNIVERSITIES must be"),
        "make lubm: one university is the DEFAULT and the count LUBM's published answers are \
         quoted for; refusing it would break every ordinary run; output:\n{combined}"
    );
    assert_ne!(
        code, 0,
        "make lubm: the arena is uncreatable, so the run must fail on the ARENA; output:\n{combined}"
    );
    assert!(
        combined.contains("LUBM_OUT="),
        "make lubm: the failure must name the arena knob, showing the knob checks passed; \
         output:\n{combined}"
    );
}

#[test]
fn the_lubm_lane_refuses_a_document_base_that_is_not_an_absolute_iri() {
    // An empty relative IRI in every generated file resolves against this base, so a base that is
    // not an absolute IRI silently puts the scratch directory's `file://` path into the corpus —
    // and into the digest published as that corpus's provenance.
    let (code, combined) = run_lane_with_knobs(
        "lubm",
        "LUBM_OUT",
        &["LUBM_DOC_BASE=not-an-iri".to_string()],
    );
    assert_ne!(
        code, 0,
        "make lubm: a relative document base reintroduces the `file://` leak the `--base` flag \
         exists to close; output:\n{combined}"
    );
    assert!(
        combined.contains("LUBM_DOC_BASE"),
        "make lubm: the refusal must name the knob; output:\n{combined}"
    );

    // THE VALID NEIGHBOUR. An absolute IRI is what the knob is for, and it must get
    // PAST this check — a guard that rejected every base would satisfy the half
    // above just as happily. As elsewhere, the neighbour is made to fail on the
    // ARENA, which is what shows execution reached it.
    let (unusable, unusable_root) = uncreatable_arena("doc-base-ok");
    let (code, stdout, stderr) = run_make(&[
        "lubm",
        "LUBM_DOC_BASE=http://example.org/lubm/",
        &format!("LUBM_OUT={unusable}"),
    ]);
    let combined = format!("{stdout}\n{stderr}");
    let _ = std::fs::remove_dir_all(&unusable_root);
    assert!(
        !combined.contains("LUBM_DOC_BASE must"),
        "make lubm: an absolute IRI is exactly what LUBM_DOC_BASE takes, and it is the \
         default this lane ships; refusing it would break every ordinary run; output:\n{combined}"
    );
    assert_ne!(
        code, 0,
        "make lubm: the arena is uncreatable, so the run must still fail — on the ARENA, having \
         accepted the base; output:\n{combined}"
    );
    assert!(
        combined.contains("LUBM_OUT="),
        "make lubm: the failure must name the arena knob, showing the base check passed; \
         output:\n{combined}"
    );
}

#[test]
fn the_lubm_lane_refuses_an_ontology_iri_that_is_not_absolute() {
    // `LUBM_ONTO` is stamped into every document the generator writes, so it is an
    // input to the corpus digest as surely as the seed is, and it had been left out of
    // the guard that decides whether to assert that digest against its pin.
    //
    // WHAT THIS TEST CAN AND CANNOT SEE, stated because the first version of it could
    // see nothing. The knob VALIDATION runs before step 1, so it is testable here. The
    // PIN GUARD lives at step 5, past generation and conversion, so no test in this
    // file can reach it: every test here points the arena somewhere uncreatable
    // precisely so the run stops before the network and the JDK. Asserting
    // `!contains("does not match its recorded pin")` after an arena failure is
    // trivially true and proves nothing — a non-control, which is what an earlier
    // draft of this test was. The pin guard's two directions are demonstrated by
    // running the lane instead, and `docs/design/purrdf-bench-lane-laws.md` records
    // what that demonstration showed.
    let (code, combined) =
        run_lane_with_knobs("lubm", "LUBM_OUT", &["LUBM_ONTO=not-an-iri".to_string()]);
    assert_ne!(
        code, 0,
        "make lubm: a relative ontology IRI must be refused — it is stamped into every \
         generated document, so a relative one would put an unresolvable IRI in the \
         corpus; output:\n{combined}"
    );
    assert!(
        combined.contains("LUBM_ONTO"),
        "make lubm: the refusal must name the knob; output:\n{combined}"
    );
    assert!(
        !combined.contains("1/7 artifacts"),
        "make lubm: a knob this lane cannot honour must be refused before the artifacts \
         step; output:\n{combined}"
    );

    // The valid neighbour for the VALIDATOR: an absolute, non-default ontology is a
    // legitimate request for a different corpus and must get past the knob check.
    let (unusable, unusable_root) = uncreatable_arena("onto-ok");
    let (code, stdout, stderr) = run_make(&[
        "lubm",
        "LUBM_ONTO=http://example.org/onto/univ-bench.owl",
        &format!("LUBM_OUT={unusable}"),
    ]);
    let combined = format!("{stdout}\n{stderr}");
    let _ = std::fs::remove_dir_all(&unusable_root);
    assert!(
        !combined.contains("LUBM_ONTO must"),
        "make lubm: an absolute IRI is what this knob takes; output:\n{combined}"
    );
    assert_ne!(
        code, 0,
        "make lubm: the arena is uncreatable; output:\n{combined}"
    );
    assert!(
        combined.contains("LUBM_OUT="),
        "make lubm: the failure must name the arena knob, showing the ontology was \
         accepted; output:\n{combined}"
    );
}

// ---------------------------------------------------------------------------------------------
// THE STAND-INS THEMSELVES ARE UNDER TEST, because one of them committed an artifact.
//
// The `--manifest` incident was this file's stand-in writing to the last argument it was given.
// The repair refused a flag-shaped last argument, and the claim that both holes were closed was
// false: both lanes pass the query TEXT last, so the same 71 bytes still landed in the working
// directory under the same name — and the sibling `convert_stand_in` had no guard at all.
//
// That survived because the refusal had NO OBSERVING TEST. `grep` for its message found only the
// fixture that emits it. A refusal nothing executes is a comment.
// ---------------------------------------------------------------------------------------------

/// Runs a stand-in with `args` from inside `cwd`, returning `(status, stderr, files created)`.
///
/// The created-file list is what makes this an observation rather than a status check: the defect
/// was never a wrong exit code, it was a file appearing somewhere nobody looked.
fn run_stand_in(binary: &Path, cwd: &Path, args: &[&str]) -> (i32, String, Vec<String>) {
    let before: std::collections::BTreeSet<String> = std::fs::read_dir(cwd)
        .expect("enumerate the working directory")
        .map(|e| e.expect("read an entry").file_name().to_string_lossy().into_owned())
        .collect();
    let output = Command::new(binary)
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("run the stand-in");
    let after: std::collections::BTreeSet<String> = std::fs::read_dir(cwd)
        .expect("enumerate the working directory")
        .map(|e| e.expect("read an entry").file_name().to_string_lossy().into_owned())
        .collect();
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        after.difference(&before).cloned().collect(),
    )
}

#[test]
fn a_stand_in_writes_nowhere_but_the_destination_a_lane_actually_names() {
    let root = scratch("stand-in-destinations");
    let workdir = root.join("cwd");
    std::fs::create_dir_all(&workdir).expect("create the working directory");

    // BOTH stand-ins, because the defect was present in one and the guard in the other, and that
    // asymmetry is the failure shape this whole file opens by warning about.
    for (label, binary) in [
        ("credible", credible_stand_in(root.join("credible"))),
        ("silent-drop", convert_stand_in(root.join("dropper"), "")),
    ] {
        // 1. The exact invocation that produced the committed `--manifest`: a lane's `query`,
        //    whose final argument is the query text. 71 bytes, in the CWD, under a name the
        //    tracked-path gate ACCEPTS — which is why the gate alone did not close this.
        let query_text = "SELECT ?s WHERE { ?s ?p ?o } LIMIT 1";
        let (code, err, created) = run_stand_in(
            &binary,
            &workdir,
            &["query", "--data", "/nonexistent.pack", "--results-format", "json", query_text],
        );
        assert!(
            created.is_empty(),
            "{label}: `query` must create no file; it created {created:?} (this is the \
             71-byte artifact that was committed). stderr:\n{err}"
        );
        assert_eq!(
            code, 0,
            "{label}: refusing to write is not an error for `query` — it writes to standard \
             output. stderr:\n{err}"
        );

        // 2. A flag-shaped destination, which is the shape the name itself had.
        let (code, err, created) =
            run_stand_in(&binary, &workdir, &["convert", "--from", "ntriples", "--manifest"]);
        assert_eq!(code, 3, "{label}: a flag-shaped destination must be refused; stderr:\n{err}");
        assert!(created.is_empty(), "{label}: created {created:?}");

        // 3. A relative destination. A lane never passes one, and a relative path is precisely
        //    what lands in whatever directory the harness happened to be standing in.
        let (code, err, created) =
            run_stand_in(&binary, &workdir, &["convert", "--from", "ntriples", "in.nt", "out.nq"]);
        assert_eq!(code, 3, "{label}: a relative destination must be refused; stderr:\n{err}");
        assert!(created.is_empty(), "{label}: created {created:?}");

        // 4. An absolute destination whose directory does not exist. A lane creates its arena
        //    before converting into it, so this is a harness mistake, not a lane instruction.
        let nowhere = root.join("no-such-dir").join("out.nq");
        let (code, err, created) = run_stand_in(
            &binary,
            &workdir,
            &["convert", "--from", "ntriples", "/in.nt", nowhere.to_str().expect("utf-8 path")],
        );
        assert_eq!(code, 3, "{label}: stderr:\n{err}");
        assert!(created.is_empty(), "{label}: created {created:?}");

        // THE NEIGHBOUR, and it is the half that decides whether any of this is usable: the
        // destination a lane really does pass must still be written. Guard the four shapes above
        // by refusing everything and every lane test in this file goes green for the wrong reason.
        let wanted = root.join("wanted.nq");
        let (code, err, created) = run_stand_in(
            &binary,
            &workdir,
            &["convert", "--from", "ntriples", "/in.nt", wanted.to_str().expect("utf-8 path")],
        );
        assert_eq!(
            code, 0,
            "{label}: an absolute destination in an existing directory is what every lane passes \
             and must be honoured; stderr:\n{err}"
        );
        assert!(
            wanted.is_file(),
            "{label}: the destination must actually be written, or these tests prove nothing \
             about the lanes"
        );
        assert!(
            created.is_empty(),
            "{label}: and nothing may appear in the working directory even on the accepted \
             path; created {created:?}"
        );
        std::fs::remove_file(&wanted).expect("remove the accepted destination");
    }

    let _ = std::fs::remove_dir_all(&root);
}
