// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The shared lane laws, exercised directly rather than through a lane.
//!
//! `make_bench_lanes.rs` drives `make lubm` and `make watdiv`, which is the right
//! way to test what a LANE does — but every one of those tests stops before step 1,
//! because past that point a lane wants the network and a JRE. So the helpers in
//! `scripts/lane-common.sh` that run INSIDE a lane's steps had no test that could
//! reach them, and the proofs offered for them were run in a shell and discarded.
//! A proof nobody can re-run is a claim.
//!
//! These source `lane-common.sh` and call its functions, so they are offline, need
//! no binary, no artifact and no `make`, and run wherever `cargo test` runs.
//!
//! EVERY REFUSAL IS CHECKED IN BOTH DIRECTIONS. A guard written slightly too
//! eagerly refuses everything, which is indistinguishable from correct strictness
//! unless the valid neighbour is executed too — and two defects in this file's own
//! subject matter were caught exactly that way rather than by reading.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Runs `body` with `scripts/lane-common.sh` sourced, returning (exit code, stdout+stderr).
///
/// `LANE`/`LANE_BINARY` are set because the file refuses to load without them —
/// that refusal is itself one of its laws.
fn in_lane_common(body: &str) -> (i32, String) {
    let script = format!(
        "set -uo pipefail\nLANE=probe\nLANE_BINARY='probe binary'\nsource '{}'\n{body}\n",
        repo_root().join("scripts/lane-common.sh").display()
    );
    let output = Command::new("bash")
        .arg("-c")
        .arg(script)
        .current_dir(repo_root())
        .output()
        .expect("run bash with lane-common.sh sourced");
    (
        output.status.code().unwrap_or(-1),
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
    )
}

fn repo_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop();
    dir.pop();
    dir
}

fn scratch(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "purrdf-lane-common-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after the epoch")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create the scratch directory");
    dir
}

fn write(path: &Path, contents: &[u8]) {
    std::fs::write(path, contents).expect("write a fixture file");
}

// ---------------------------------------------------------------------------------------------
// A DIRECTORY DIGEST IS A MANIFEST OF PER-FILE DIGESTS, NEVER A CONCATENATION.
//
// Streaming `name || NUL || content` is ambiguous: nothing delimits the end of content from the
// start of the next name. This was shipped, and the collision below is the one that proved it.
// ---------------------------------------------------------------------------------------------

#[test]
fn the_query_set_digest_distinguishes_sets_a_concatenation_would_conflate() {
    let root = scratch("digest-collide");
    let a = root.join("A");
    let b = root.join("B");
    std::fs::create_dir_all(&a).expect("create A");
    std::fs::create_dir_all(&b).expect("create B");

    // A holds one file `a` whose content is `b\0c`. B holds an empty `a` and a `b`
    // containing `c`. Concatenating name || NUL || content yields `a\0b\0c` for both.
    write(&a.join("a"), b"b\0c");
    write(&b.join("a"), b"");
    write(&b.join("b"), b"c");

    let (code, out) = in_lane_common(&format!(
        "lane_query_set_digest '{}'\nlane_query_set_digest '{}'",
        a.display(),
        b.display()
    ));
    assert_eq!(code, 0, "both digests must be computable; output:\n{out}");
    let digests: Vec<&str> = out.split_whitespace().collect();
    assert_eq!(digests.len(), 2, "expected two digests; output:\n{out}");
    assert_ne!(
        digests[0], digests[1],
        "two DIFFERENT query sets must not share a digest. A concatenation of names and bytes \
         conflates them, so a substituted query set would keep the recorded digest and walk \
         straight past `lane_verify_query_set` — the tripwire that exists to catch exactly that. \
         output:\n{out}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_query_set_digest_is_identical_for_identical_content_elsewhere() {
    // THE VALID NEIGHBOUR. A digest that distinguished everything — including two
    // copies of one query set — would be useless as a reuse and reproducibility
    // handle, and would fail every verification it exists to perform.
    let root = scratch("digest-same");
    let one = root.join("one");
    let two = root.join("two");
    std::fs::create_dir_all(&one).expect("create one");
    std::fs::create_dir_all(&two).expect("create two");
    for dir in [&one, &two] {
        write(&dir.join("q1.rq"), b"SELECT * WHERE { ?s ?p ?o }\n");
        write(&dir.join("queries.tsv"), b"id\tfile\nQ1\tq1.rq\n");
    }

    let (code, out) = in_lane_common(&format!(
        "lane_query_set_digest '{}'\nlane_query_set_digest '{}'",
        one.display(),
        two.display()
    ));
    assert_eq!(code, 0, "output:\n{out}");
    let digests: Vec<&str> = out.split_whitespace().collect();
    assert_eq!(digests.len(), 2, "output:\n{out}");
    assert_eq!(
        digests[0], digests[1],
        "the same content in a different directory must digest the same, or the digest cannot \
         serve as a reproducibility handle at all; output:\n{out}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_query_set_digest_refuses_an_empty_directory_rather_than_certifying_nothing() {
    let root = scratch("digest-empty");
    let empty = root.join("empty");
    std::fs::create_dir_all(&empty).expect("create empty");

    let (code, out) = in_lane_common(&format!("lane_query_set_digest '{}'", empty.display()));
    assert_ne!(
        code, 0,
        "an empty directory must be refused. Its manifest digest is a perfectly valid-looking \
         64 hex characters, and publishing it under 'the reproducibility check' would certify a \
         workload of no queries; output:\n{out}"
    );
    assert!(
        out.contains("holds no files"),
        "the refusal must say what is wrong; output:\n{out}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_query_set_digest_refuses_a_name_a_manifest_record_cannot_represent() {
    let root = scratch("digest-newline");
    let dir = root.join("weird");
    std::fs::create_dir_all(&dir).expect("create weird");
    write(&dir.join("ordinary.rq"), b"SELECT 1\n");
    // A newline in a name would split one manifest record into two, which is the
    // same ambiguity in a different costume.
    write(&dir.join("we\nird.rq"), b"SELECT 2\n");

    let (code, out) = in_lane_common(&format!("lane_query_set_digest '{}'", dir.display()));
    assert_ne!(
        code, 0,
        "a name carrying a newline must be refused; output:\n{out}"
    );
    assert!(
        out.contains("newline") || out.contains("NUL"),
        "the refusal must name the reason; output:\n{out}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

// ---------------------------------------------------------------------------------------------
// A QUERY FILE MUST BE PRESENT, READABLE AND NON-EMPTY — each refused by name.
//
// All three failures otherwise converge on one symptom: `cat` yields the empty string, the engine
// is handed an empty query, and its usage complaint is printed as this lane's diagnosis, blaming
// the thing under test for the harness's own artifact.
// ---------------------------------------------------------------------------------------------

#[test]
fn a_query_file_is_accepted_when_it_is_present_readable_and_non_empty() {
    // THE VALID NEIGHBOUR, first: the three refusals below are worthless against a
    // guard that rejects everything.
    let root = scratch("qfile-ok");
    let path = root.join("q1.rq");
    write(&path, b"SELECT * WHERE { ?s ?p ?o }\n");

    let (code, out) = in_lane_common(&format!(
        "lane_require_query_file '{}' Q1 \"PROBE_OUT='x'\" && echo ACCEPTED",
        path.display()
    ));
    assert_eq!(
        code, 0,
        "an ordinary query file must be accepted; output:\n{out}"
    );
    assert!(out.contains("ACCEPTED"), "output:\n{out}");

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_missing_query_file_is_refused_naming_the_file_and_the_arena_knob() {
    let root = scratch("qfile-missing");
    let (code, out) = in_lane_common(&format!(
        "lane_require_query_file '{}' Q1 \"PROBE_OUT='x'\"",
        root.join("absent.rq").display()
    ));
    assert_ne!(code, 0, "output:\n{out}");
    assert!(
        out.contains("absent.rq"),
        "the refusal must name the file; output:\n{out}"
    );
    assert!(
        out.contains("PROBE_OUT"),
        "the refusal must name the knob that supplied the arena, so an operator knows which one \
         to look at; output:\n{out}"
    );
    assert!(
        out.contains("engine") || out.contains("not at fault"),
        "the refusal must say the engine has not been asked, which is the whole point of \
         checking here; output:\n{out}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_empty_query_file_is_refused_rather_than_sent_to_the_engine() {
    let root = scratch("qfile-empty");
    let path = root.join("empty.rq");
    write(&path, b"");

    let (code, out) = in_lane_common(&format!(
        "lane_require_query_file '{}' Q9 \"PROBE_OUT='x'\"",
        path.display()
    ));
    assert_ne!(code, 0, "output:\n{out}");
    assert!(
        out.contains("EMPTY"),
        "the refusal must say it is empty; output:\n{out}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_unreadable_query_file_is_refused_when_the_process_genuinely_cannot_read_it() {
    use std::os::unix::fs::PermissionsExt;

    let root = scratch("qfile-unreadable");
    let path = root.join("noread.rq");
    write(&path, b"SELECT 1\n");
    let mut permissions = std::fs::metadata(&path)
        .expect("stat the fixture")
        .permissions();
    permissions.set_mode(0o000);
    std::fs::set_permissions(&path, permissions).expect("drop every permission bit");

    // uid 0 bypasses file permission bits entirely, so for root the file IS readable
    // and `cat` WILL succeed — there is no misdiagnosis to prevent and nothing to
    // assert. Skipping is honest here; claiming a pass would not be. The `cat`-result
    // check inside each lane's `run_query` is the backstop that holds for every uid,
    // because it observes the read rather than predicting it.
    let readable_anyway = std::fs::read(&path).is_ok();
    if readable_anyway {
        eprintln!(
            "skipped: this process can read a mode-000 file (running as uid 0), so the \
             condition under test cannot occur here"
        );
        let _ = std::fs::remove_dir_all(&root);
        return;
    }

    let (code, out) = in_lane_common(&format!(
        "lane_require_query_file '{}' Q9 \"PROBE_OUT='x'\"",
        path.display()
    ));
    assert_ne!(
        code, 0,
        "a present, non-empty file this process cannot read must be refused: reading it yields \
         the empty string, and the engine's complaint about an empty query would be reported as \
         though the corpus or the binary were at fault; output:\n{out}"
    );
    assert!(
        out.contains("cannot be READ") || out.contains("cannot be read"),
        "the refusal must distinguish unreadable from missing and from empty; output:\n{out}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

// ---------------------------------------------------------------------------------------------
// AN UNSIGNED-INTEGER KNOB IS VALIDATED AND NORMALISED TOGETHER.
//
// Validation alone shipped, and the gap was that `007` passed it: a lane then printed `seed=007`
// beside a digest it advertised as reproducible from that seed, while the tool consuming the
// value read 7 and recorded 7 — one run, two published labels, byte-identical output.
// ---------------------------------------------------------------------------------------------

#[test]
fn a_uint_knob_is_normalised_so_one_run_cannot_publish_two_labels_for_one_number() {
    let (code, out) = in_lane_common(
        "SEED=007\nlane_require_uint PROBE_SEED SEED\necho \"normalised=${SEED}\"\n\
         PLAIN=7\nlane_require_uint PROBE_SEED PLAIN\necho \"plain=${PLAIN}\"\n\
         ZERO=0\nlane_require_uint PROBE_SEED ZERO\necho \"zero=${ZERO}\"",
    );
    assert_eq!(
        code, 0,
        "007, 7 and 0 are all valid unsigned integers; output:\n{out}"
    );
    assert!(
        out.contains("normalised=7"),
        "`007` must normalise to `7`, so the label a lane prints is the value its tools read; \
         output:\n{out}"
    );
    assert!(out.contains("plain=7"), "output:\n{out}");
    assert!(
        out.contains("zero=0"),
        "zero is a legal seed and must survive normalisation; output:\n{out}"
    );
}

#[test]
fn a_uint_knob_refuses_what_is_not_an_unsigned_integer_and_quotes_it_back() {
    for bad in ["abc", "-1", "1.5", "", "0x10"] {
        let (code, out) = in_lane_common(&format!("BAD='{bad}'\nlane_require_uint PROBE_SEED BAD"));
        assert_ne!(
            code, 0,
            "PROBE_SEED='{bad}' must be refused; output:\n{out}"
        );
        assert!(
            out.contains("PROBE_SEED"),
            "the refusal must name the knob; output:\n{out}"
        );
    }
}

#[test]
fn a_positive_knob_refuses_zero_and_accepts_one() {
    let (code, out) = in_lane_common("N=0\nlane_require_positive PROBE_COUNT N");
    assert_ne!(
        code, 0,
        "zero universities is an empty corpus; output:\n{out}"
    );
    assert!(out.contains("positive"), "output:\n{out}");

    // The valid neighbour: 1 is the default for the knob this guards.
    let (code, out) =
        in_lane_common("N=1\nlane_require_positive PROBE_COUNT N\necho \"accepted=${N}\"");
    assert_eq!(code, 0, "one must be accepted; output:\n{out}");
    assert!(out.contains("accepted=1"), "output:\n{out}");

    // And a normalising positive: `01` is one.
    let (code, out) =
        in_lane_common("N=01\nlane_require_positive PROBE_COUNT N\necho \"accepted=${N}\"");
    assert_eq!(code, 0, "output:\n{out}");
    assert!(
        out.contains("accepted=1"),
        "`01` must normalise to `1`; output:\n{out}"
    );
}

// ---------------------------------------------------------------------------------------------
// A DIAGNOSTIC MUST SURVIVE THE TAB-SEPARATED RECORD IT TRAVELS IN.
// ---------------------------------------------------------------------------------------------

#[test]
fn a_multi_line_diagnostic_survives_flattening_instead_of_being_truncated() {
    let (code, out) = in_lane_common(
        "msg=$'purrdf: evaluation exceeded the fixed ceiling\\n  observed 4096\\n  permitted 1024'\n\
         lane_flatten_detail \"${msg}\"",
    );
    assert_eq!(code, 0, "output:\n{out}");
    assert!(
        out.contains("observed 4096") && out.contains("permitted 1024"),
        "the counts are on later lines, and they are the part an operator needs. `head -1` \
         dropped both while the report called the detail verbatim; output:\n{out}"
    );
    assert!(
        !out.trim_end().contains('\n'),
        "the result must be one line, or it cannot travel in a tab-separated record; \
         output:\n{out:?}"
    );

    // A literal tab inside the message must not create a phantom column either.
    let (code, out) = in_lane_common("lane_flatten_detail \"$(printf 'a\\tb\\nc')\"");
    assert_eq!(code, 0, "output:\n{out}");
    assert!(
        !out.contains('\t'),
        "an embedded tab must be neutralised; output:\n{out:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// COLLATION IS PINNED BY THE LANE, NOT BY THE CALLER.
// ---------------------------------------------------------------------------------------------

#[test]
fn sourcing_the_shared_laws_pins_collation_whatever_the_caller_had() {
    let script = format!(
        "set -uo pipefail\nLANE=probe\nLANE_BINARY=b\nsource '{}'\necho \"LC_ALL=${{LC_ALL}}\"",
        repo_root().join("scripts/lane-common.sh").display()
    );
    let output = Command::new("bash")
        .arg("-c")
        .arg(&script)
        .env("LC_ALL", "en_US.UTF-8")
        .current_dir(repo_root())
        .output()
        .expect("run bash with a UTF-8 collation in the environment");
    let combined = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(
        combined.contains("LC_ALL=C"),
        "a caller carrying a UTF-8 collation must end up with a bytewise one, because `sort` \
         order is an input to every digest a lane publishes; output:\n{combined}"
    );
}
