// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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

use std::os::unix::fs::PermissionsExt;
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

/// As [`in_lane_common`], with `TMPDIR` pointed at a directory the caller can enumerate.
///
/// `mktemp` obeys `TMPDIR`, so this is what makes a claim about what a helper leaves behind
/// observable at all: the default temporary directory is shared with every other process on the
/// host, and "is this file still there?" cannot be answered in it.
fn in_lane_common_with_tmpdir(body: &str, tmpdir: &Path) -> (i32, String) {
    let script = format!(
        "set -uo pipefail\nLANE=probe\nLANE_BINARY='probe binary'\nsource '{}'\n{body}\n",
        repo_root().join("scripts/lane-common.sh").display()
    );
    let output = Command::new("bash")
        .arg("-c")
        .arg(script)
        .env("TMPDIR", tmpdir)
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
        // A `return` after an `eprintln!` reports a PASS for a check that never ran,
        // and libtest hides that output for a passing test — the mechanism this file's
        // sibling bans as a design constraint. So the skip is ASSERTED instead: under
        // CI it is a failure, because a suite that silently stops proving something is
        // worse than one that says it cannot.
        let _ = std::fs::remove_dir_all(&root);
        assert!(
            std::env::var("CI").is_err(),
            "running as a uid that can read a mode-000 file (root): the unreadable-file \
             branch cannot be reached, so this test is not proving what it is named for. \
             Run the suite as a non-root uid."
        );
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
        // AND THE VALUE, which is what "quotes it back" means. Asserting only the knob
        // name left the quoted value unguarded — and `lane-common.sh` records that one
        // of the three per-lane copies of this check had already dropped it, so this is
        // a drift that has happened before. Mutation-checked: stripping `(got '…')`
        // from the message turns this red.
        assert!(
            out.contains(&format!("'{bad}'")),
            "the refusal must quote the offending value back, or an operator cannot see \
             what the knob actually carried; output:\n{out}"
        );
    }
}

// A DIGIT STRING BASH CANNOT CARRY IS REFUSED, NOT SILENTLY REPLACED.
//
// `$((10#${value}))` is base-10-explicit, which is the right answer to the octal reading of
// `007` and says nothing about the other way a digit string fails to survive arithmetic: bash
// integers are signed 64-bit and wrap without a word. `9223372036854775808` became
// `-9223372036854775808` — a NEGATIVE seed out of an all-digits knob — and
// `18446744073709551617` became `1`, so `SCALE_QUADS` at that value published a one-quad corpus
// as the run the operator asked for. Every one of those passes `^[0-9]+$`, so validation could
// not see it; the substitution happened in the normalisation that was supposed to make the
// published label and the consumed value the same number.
#[test]
fn a_uint_knob_refuses_a_value_bash_would_wrap_rather_than_substituting_a_different_number() {
    for (bad, wrapped_to) in [
        ("9223372036854775808", "-9223372036854775808"),
        ("18446744073709551617", "1"),
        ("99999999999999999999", "7766279631452241919"),
    ] {
        let (code, out) = in_lane_common(&format!(
            "BIG='{bad}'\nlane_require_uint PROBE_SEED BIG\necho \"published=${{BIG}}\""
        ));
        assert_ne!(
            code, 0,
            "PROBE_SEED='{bad}' wraps to {wrapped_to} in bash arithmetic and must be refused \
             rather than carried; output:\n{out}"
        );
        assert!(
            out.contains(&format!("'{bad}'")),
            "the refusal must quote the value the operator actually set; output:\n{out}"
        );
        assert!(
            !out.contains(&format!("published={wrapped_to}")),
            "the wrapped value {wrapped_to} must never reach a caller; output:\n{out}"
        );
    }
}

#[test]
fn a_uint_knob_accepts_the_largest_value_it_can_carry_intact() {
    // THE NEIGHBOUR, and it is the boundary itself rather than a comfortable distance from it:
    // a bound written one too tight refuses a legal value, which is the mirror of the bug above
    // and just as invisible — the refusal looks like correct strictness. 2^63-1 is representable,
    // so it must be accepted and must come back unchanged.
    let (code, out) = in_lane_common(
        "MAX=9223372036854775807\nlane_require_uint PROBE_SEED MAX\necho \"carried=${MAX}\"\n\
         NEAR=9223372036854775806\nlane_require_uint PROBE_SEED NEAR\necho \"near=${NEAR}\"\n\
         PADDED=000000000000000000009\nlane_require_uint PROBE_SEED PADDED\n\
         echo \"padded=${PADDED}\"",
    );
    assert_eq!(
        code, 0,
        "2^63-1 is representable and must not be refused; output:\n{out}"
    );
    assert!(
        out.contains("carried=9223372036854775807"),
        "the largest carriable value must come back byte-identical; output:\n{out}"
    );
    assert!(out.contains("near=9223372036854775806"), "output:\n{out}");
    // A padded value LONGER than the bound's own digit count is still small: the range check
    // must run on the stripped text, not on the string the operator typed. Written the other
    // way round, this 21-character `9` would be refused for its length.
    assert!(
        out.contains("padded=9"),
        "leading zeros are stripped before the range is judged; output:\n{out}"
    );
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

// ---------------------------------------------------------------------------------------------
// THE LAWS THAT LIVE PAST STEP 1, PROVED WHERE THEY LIVE.
//
// `make_bench_lanes.rs` stops every lane before step 1, so it cannot observe these at all — it
// carries nine negative assertions about step-5 messages that are trivially true for the runs
// they are made about. These call the helpers directly instead, which is the only harness that
// reaches them.
// ---------------------------------------------------------------------------------------------

#[test]
fn a_file_whose_magic_is_wrong_is_refused_and_a_correct_one_is_not() {
    let root = scratch("magic");
    let good = root.join("good.pack");
    let bad = root.join("bad.pack");
    write(&good, b"PURRPCK1and then some payload bytes");
    // Eight bytes of something else passed an emptiness test, got stamped as a pack,
    // and was queried by every later run. That is the defect this law exists for.
    write(&bad, b"NOTAPACKand then some payload bytes");

    let (code, out) = in_lane_common(&format!(
        "lane_require_magic '{}' 'the pack' 'PURRPCK1' 'a purrdf pack' && echo ACCEPTED",
        good.display()
    ));
    assert_eq!(
        code, 0,
        "a file with the right magic must be accepted; output:\n{out}"
    );
    assert!(out.contains("ACCEPTED"), "output:\n{out}");

    let (code, out) = in_lane_common(&format!(
        "lane_require_magic '{}' 'the pack' 'PURRPCK1' 'a purrdf pack'",
        bad.display()
    ));
    assert_ne!(code, 0, "wrong magic must be refused; output:\n{out}");
    assert!(
        out.contains("is not a purrdf pack"),
        "the refusal must name the format it is not — this message is assembled from the \
         helper's `format` argument, which is why it reads as one sentence; output:\n{out}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_certificate_does_not_outlive_the_run_that_wrote_it() {
    // THE ORDERING NOTHING ELSE OBSERVES. A stamp is what a LATER run consults instead
    // of redoing the work, so one left behind by a run that failed certifies output
    // that was never finished. `make_bench_lanes.rs` has an assertion named for this
    // and cannot reach it: the run it makes that assertion about dies 288 lines before
    // the stamp is written.
    let root = scratch("certify");
    let stamp = root.join(".pack-stamp");

    // A run that FAILS must leave no stamp, and must say that it removed it.
    let (code, out) = in_lane_common(&format!(
        "printf 'key\\n' > '{}'\nlane_certify '{}' 'the pack stamp'\ndie 'the load failed'",
        stamp.display(),
        stamp.display()
    ));
    assert_ne!(code, 0, "the probe deliberately fails; output:\n{out}");
    assert!(
        !stamp.exists(),
        "a certificate must not survive the run that wrote it failing — a later run \
         consults it INSTEAD of loading, so it would certify a pack nothing finished \
         certifying; output:\n{out}"
    );
    assert!(
        out.contains("removed the pack stamp"),
        "the removal must be announced: a certificate vanishing silently is its own \
         puzzle; output:\n{out}"
    );

    // THE VALID NEIGHBOUR: a run that SUCCEEDS must keep it. A trap that deleted
    // certificates unconditionally would satisfy the half above and destroy every
    // reuse path in every lane.
    let kept = root.join(".kept-stamp");
    let (code, out) = in_lane_common(&format!(
        "printf 'key\\n' > '{}'\nlane_certify '{}' 'the pack stamp'\nexit 0",
        kept.display(),
        kept.display()
    ));
    assert_eq!(code, 0, "output:\n{out}");
    assert!(
        kept.exists(),
        "a certificate written by a run that SUCCEEDED must survive it, or no lane could \
         ever reuse anything; output:\n{out}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_query_set_tripwire_fires_when_the_set_changes_under_it() {
    // The law `make_bench_lanes.rs` cannot reach, and the one whose absence from the
    // LUBM lane's most expensive phase went unnoticed: a concurrent run rewriting the
    // arena mid-flight.
    let root = scratch("tripwire");
    let queries = root.join("queries");
    std::fs::create_dir_all(&queries).expect("create the query directory");
    write(&queries.join("q1.rq"), b"SELECT * WHERE { ?s ?p ?o }\n");

    let (code, out) = in_lane_common(&format!(
        "sha=\"$(lane_query_set_digest '{q}')\"\n\
         lane_verify_query_set '{q}' \"${{sha}}\" 'unchanged' \"PROBE_OUT='x'\" && echo UNCHANGED\n\
         printf 'SELECT ?x WHERE {{ ?x ?p ?o }}\\n' > '{q}/q1.rq'\n\
         lane_verify_query_set '{q}' \"${{sha}}\" 'after a rewrite' \"PROBE_OUT='x'\"",
        q = queries.display()
    ));
    assert!(
        out.contains("UNCHANGED"),
        "an unchanged set must verify, or the tripwire would fire on every run; \
         output:\n{out}"
    );
    assert_ne!(code, 0, "a rewritten set must be refused; output:\n{out}");
    assert!(
        out.contains("CHANGED") && out.contains("PROBE_OUT"),
        "the refusal must say the set changed and name the arena knob, because a \
         concurrent run sharing the arena is the overwhelmingly likely cause; \
         output:\n{out}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_short_query_set_is_refused_and_a_complete_one_is_not() {
    let root = scratch("count");
    let queries = root.join("queries");
    std::fs::create_dir_all(&queries).expect("create the query directory");
    for n in 1..=3 {
        write(&queries.join(format!("q{n}.rq")), b"SELECT 1\n");
    }

    let (code, out) = in_lane_common(&format!(
        "lane_require_query_count '{}' 3 \"PROBE_OUT='x'\" && echo COMPLETE",
        queries.display()
    ));
    assert_eq!(
        code, 0,
        "the expected count must be accepted; output:\n{out}"
    );
    assert!(out.contains("COMPLETE"), "output:\n{out}");

    let (code, out) = in_lane_common(&format!(
        "lane_require_query_count '{}' 4 \"PROBE_OUT='x'\"",
        queries.display()
    ));
    assert_ne!(code, 0, "a short set must be refused; output:\n{out}");
    assert!(
        out.contains("not 4"),
        "the refusal must name the count the workload is defined to have — no digest is \
         published for a set that is not the whole set; output:\n{out}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_empty_artifact_is_refused_and_a_non_empty_one_is_not() {
    let root = scratch("nonempty");
    let empty = root.join("empty.nq");
    let full = root.join("full.nq");
    write(&empty, b"");
    write(
        &full,
        b"<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n",
    );

    let (code, out) = in_lane_common(&format!(
        "lane_require_nonempty_file '{}' 'the corpus' && echo ACCEPTED",
        full.display()
    ));
    assert_eq!(code, 0, "output:\n{out}");
    assert!(out.contains("ACCEPTED"), "output:\n{out}");

    let (code, out) = in_lane_common(&format!(
        "lane_require_nonempty_file '{}' 'the corpus'",
        empty.display()
    ));
    assert_ne!(code, 0, "an empty artifact must be refused; output:\n{out}");
    assert!(out.contains("EMPTY"), "output:\n{out}");
    assert!(
        !out.contains("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
        "the SHA-256 of the empty string must be DESCRIBED and never emitted — an \
         operator grepping a log for it would otherwise hit the message saying it was \
         refused; output:\n{out}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_file_that_is_not_n_quads_is_refused_and_a_real_one_is_not() {
    // NON-EMPTY IS NOT "IS WHAT IT CLAIMS TO BE". A CLI that exits 0 having written
    // something other than N-Quads passes an emptiness test, and a lane that then
    // digests it publishes provenance for bytes that are not the corpus.
    let root = scratch("nquads");
    let good = root.join("good.nq");
    let bad = root.join("bad.nq");
    write(
        &good,
        b"<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n",
    );
    // Plausible, non-empty, and not N-Quads: no terminating dot.
    write(&bad, b"this is not a quad\nnor is this\n");

    let (code, out) = in_lane_common(&format!(
        "lane_require_nquads '{}' 'the corpus' && echo ACCEPTED",
        good.display()
    ));
    assert_eq!(
        code, 0,
        "a real N-Quads row must be accepted; output:\n{out}"
    );
    assert!(out.contains("ACCEPTED"), "output:\n{out}");

    let (code, out) = in_lane_common(&format!(
        "lane_require_nquads '{}' 'the corpus'",
        bad.display()
    ));
    assert_ne!(code, 0, "output:\n{out}");
    assert!(
        out.contains("is not N-Quads"),
        "the refusal must name what the file is not; output:\n{out}"
    );
    assert!(
        out.contains("this is not a quad"),
        "and it must quote the offending first row back, so an operator can see what \
         arrived instead of guessing; output:\n{out}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn digesting_a_file_is_streamed_stable_and_refuses_what_it_cannot_read() {
    // Used at nine sites across the lanes and never proved directly. The property that
    // matters is agreement with the reference implementation, since this value ends up
    // in a published certificate.
    let root = scratch("sha");
    let path = root.join("corpus.nq");
    let body = b"<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n";
    write(&path, body);

    let (code, out) = in_lane_common(&format!("lane_sha256_file '{}'", path.display()));
    assert_eq!(code, 0, "output:\n{out}");
    let digest = out.trim().to_string();
    assert_eq!(
        digest.len(),
        64,
        "a SHA-256 is 64 hex characters; got {digest:?}"
    );

    // A one-shot digest of the same bytes, so what this proves is CHUNK-INVARIANCE:
    // that streaming in 4 MiB pieces equals hashing the whole file at once. Both sides
    // are CPython's hashlib, so it is not independent of the implementation — an
    // earlier version of this comment claimed `sha256sum` as a fallback and there was
    // no such branch. The algorithm itself is pinned elsewhere, by the reference
    // vector `splitmix64(0) == 0xe220a8397b1dcdaf` for the mixing function and by the
    // published artifact digests for SHA-256.
    let reference = Command::new("python3")
        .arg("-c")
        .arg("import hashlib,sys;print(hashlib.sha256(open(sys.argv[1],'rb').read()).hexdigest())")
        .arg(&path)
        .output()
        .expect("compute a reference digest");
    let expected = String::from_utf8_lossy(&reference.stdout)
        .trim()
        .to_string();
    assert_eq!(
        digest, expected,
        "the streamed digest must equal the one-shot digest of the same bytes, or the \
         chunking is wrong and every certificate built on it is wrong too"
    );

    // Two calls agree: a digest that varied per invocation would make every reuse
    // stamp a cache miss.
    let (_, again) = in_lane_common(&format!("lane_sha256_file '{}'", path.display()));
    assert_eq!(again.trim(), digest, "two digests of one file must agree");

    // A missing file is refused in the lane's voice, not as a Python traceback.
    let (code, out) = in_lane_common(&format!(
        "lane_sha256_file '{}'",
        root.join("absent.nq").display()
    ));
    assert_ne!(code, 0, "output:\n{out}");
    assert!(
        out.contains("does not exist") && out.contains("probe:"),
        "the refusal must carry the lane's name, not surface as a bare traceback; \
         output:\n{out}"
    );

    // AND THE UNREADABLE BRANCH, which had no test in either direction. Without it the
    // helper emits a bare PermissionError for what may be a multi-gigabyte corpus in an
    // arena an operator has touched. A diagnostic must name the right cause, and this
    // helper is one over from where that was last enforced.
    let unreadable = root.join("noread.nq");
    write(&unreadable, body);
    let mut permissions = std::fs::metadata(&unreadable)
        .expect("stat the fixture")
        .permissions();
    permissions.set_mode(0o000);
    std::fs::set_permissions(&unreadable, permissions).expect("drop every permission bit");
    if std::fs::read(&unreadable).is_err() {
        let (code, out) = in_lane_common(&format!("lane_sha256_file '{}'", unreadable.display()));
        assert_ne!(
            code, 0,
            "an unreadable file must be refused; output:\n{out}"
        );
        assert!(
            out.contains("cannot read it") && out.contains("probe:"),
            "the refusal must distinguish unreadable from missing AND carry the lane's \
             name, rather than surfacing as a PermissionError traceback; output:\n{out}"
        );
    } else {
        // uid 0 bypasses permission bits, so the condition cannot occur. Asserted
        // rather than skipped silently, so a root run fails visibly instead of
        // reporting a pass for a check it never made.
        assert!(
            std::env::var("CI").is_err(),
            "running as a uid that can read a mode-000 file (root): the unreadable branch \
             is unreachable here, so this suite is not proving it. Run as a non-root uid."
        );
    }

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_query_set_digest_refuses_a_symlink_so_the_certificate_stays_closed() {
    // `is_file()` follows symlinks, so the first version of this refusal accepted a link
    // whose target lived OUTSIDE the directory — and the published digest then moved
    // when that outside file changed, while `find -maxdepth 1 -type f` counted one
    // fewer entry than the manifest recorded. A certificate closed over a directory
    // cannot depend on bytes that are not in it.
    let root = scratch("symlink");
    let queries = root.join("queries");
    let outside = root.join("outside");
    std::fs::create_dir_all(&queries).expect("create the query directory");
    std::fs::create_dir_all(&outside).expect("create the outside directory");
    write(&queries.join("a.rq"), b"SELECT 1\n");

    // THE VALID NEIGHBOUR FIRST: a flat directory of regular files must be accepted, or
    // every lane loses its certificate.
    let (code, out) = in_lane_common(&format!("lane_query_set_digest '{}'", queries.display()));
    assert_eq!(
        code, 0,
        "a flat directory of regular files must be accepted; output:\n{out}"
    );
    let flat_digest = out.trim().to_string();

    let target = outside.join("target.rq");
    write(&target, b"ORIGINAL\n");
    std::os::unix::fs::symlink(&target, queries.join("link.rq")).expect("create the symlink");

    let (code, out) = in_lane_common(&format!("lane_query_set_digest '{}'", queries.display()));
    assert_ne!(
        code, 0,
        "a symlink in the query set must be refused: following it makes this digest \
         depend on bytes outside the directory it certifies, and makes the manifest \
         disagree with the `-maxdepth 1 -type f` count the query-count guard uses; \
         output:\n{out}"
    );
    assert!(
        out.contains("symbolic link"),
        "the refusal must say it is a symlink, not merely 'not a regular file' — the \
         two have different remedies; output:\n{out}"
    );

    // And removing it must restore exactly the earlier digest, so the refusal is about
    // the link and not about having touched the directory.
    std::fs::remove_file(queries.join("link.rq")).expect("remove the symlink");
    let (code, out) = in_lane_common(&format!("lane_query_set_digest '{}'", queries.display()));
    assert_eq!(code, 0, "output:\n{out}");
    assert_eq!(
        out.trim(),
        flat_digest,
        "removing the link must restore the original digest; output:\n{out}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_query_set_digest_names_a_vanished_directory_instead_of_raising() {
    // The state this digest exists to diagnose: a concurrent run deletes the arena at
    // the top of its own instantiation step. The first version raised
    // `FileNotFoundError`, so the carefully written "another run is using the same
    // arena" message never printed in precisely the case it was written for — and the
    // replacement had no observer either.
    let root = scratch("vanished");
    let (code, out) = in_lane_common(&format!(
        "lane_query_set_digest '{}'",
        root.join("gone").display()
    ));
    assert_ne!(code, 0, "output:\n{out}");
    assert!(
        out.contains("is not a directory") && out.contains("probe:"),
        "the refusal must name the lane and say the directory is gone, rather than \
         surfacing as a Python traceback; output:\n{out}"
    );
    assert!(
        !out.contains("Traceback"),
        "a traceback here would replace the diagnosis with a stack; output:\n{out}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_write_that_cannot_be_opened_names_the_knob_and_not_the_payload() {
    // Claimed once to "run before step 1" and to be asserted in the lane-driving tests.
    // Both were false: its earliest call site is step 5 in one lane and after step 3 in
    // the other, and its message was asserted in neither of this PR's test files — it is
    // covered for `scale-corpus` only. Proved here directly.
    let root = scratch("write-checked");
    // A regular file as the parent directory makes the open fail with ENOTDIR for every
    // uid, including root — the same trick the lane tests use for an uncreatable arena.
    let blocker = root.join("not-a-directory");
    write(&blocker, b"");
    let destination = blocker.join("manifest.txt");

    let (code, out) = in_lane_common(&format!(
        "lane_write_checked '{}' 'the manifest' \"PROBE_OUT='x'\" printf 'payload\\n'",
        destination.display()
    ));
    assert_ne!(
        code, 0,
        "an unopenable destination must be refused; output:\n{out}"
    );
    assert!(
        out.contains("cannot write the manifest") && out.contains("PROBE_OUT"),
        "the refusal must name WHAT was being written and WHICH knob supplied the path, \
         rather than surfacing as a bare shell redirection error; output:\n{out}"
    );
    assert!(
        !out.contains("payload"),
        "the payload must not be reported as the fault — the write failed, the command \
         that produced the bytes did not; output:\n{out}"
    );

    // THE VALID NEIGHBOUR: an openable destination writes the bytes and says nothing.
    let good = root.join("manifest.txt");
    let (code, out) = in_lane_common(&format!(
        "lane_write_checked '{}' 'the manifest' \"PROBE_OUT='x'\" printf 'payload\\n'",
        good.display()
    ));
    assert_eq!(
        code, 0,
        "an openable destination must succeed; output:\n{out}"
    );
    assert_eq!(
        std::fs::read_to_string(&good).expect("read the written manifest"),
        "payload\n",
        "the command's stdout must land in the file verbatim"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_binary_that_cannot_run_is_refused_for_every_lane_at_once() {
    // This law was asserted per-lane, and only for two of the three: `make_scale_corpus`
    // covers the directory-as-binary and printed-NOTHING laws but never the unrunnable
    // probe. Proving the shared helper covers all three lanes at once, which is the
    // point of the helper being shared.
    let root = scratch("probe");

    // A binary that exits non-zero must be refused, naming what it was asked and
    // saying the check ran before anything was fetched — so no downstream failure can
    // be blamed on the corpus.
    let (code, out) =
        in_lane_common("lane_run_probe \"the binary '/bin/false'\" 'for its version' /bin/false");
    assert_ne!(
        code, 0,
        "a binary that exits non-zero must be refused; output:\n{out}"
    );
    assert!(
        out.contains("is not a working probe binary") && out.contains("for its version"),
        "the refusal must name the KIND of binary and what it was asked, so the lane's \
         own noun appears rather than a generic failure; output:\n{out}"
    );
    assert!(
        out.contains("nothing has been fetched"),
        "and it must say the check ran before the lane's first step, which is what stops \
         a later failure being blamed on the corpus; output:\n{out}"
    );

    // A binary that exits 0 and says nothing is refused too — exiting 0 is not working.
    let silent = write_executable(root.join("silent"), "#!/bin/sh\nexit 0\n");
    let (code, out) = in_lane_common(&format!(
        "lane_run_probe \"the binary\" 'for its version' '{}'\n\
         lane_require_probe_said_something \"the binary\" 'for its version' \
         'A binary that says nothing is not the one under test.'",
        silent.display()
    ));
    assert_ne!(code, 0, "a silent binary must be refused; output:\n{out}");
    assert!(
        out.contains("printed NOTHING"),
        "the refusal must say it printed nothing rather than that it failed — it did \
         not fail, which is the whole trap; output:\n{out}"
    );

    // THE VALID NEIGHBOUR: a binary that answers is accepted, and its answer is
    // captured. An over-refusing probe would reject every wrapper an operator writes.
    let talker = write_executable(root.join("talker"), "#!/bin/sh\necho 'probe 9.9.9'\n");
    let (code, out) = in_lane_common(&format!(
        "lane_run_probe \"the binary\" 'for its version' '{}'\n\
         lane_require_probe_said_something \"the binary\" 'for its version' 'unused'\n\
         echo \"CAPTURED=${{LANE_PROBE_OUT}}\"",
        talker.display()
    ));
    assert_eq!(
        code, 0,
        "a binary that answers must be accepted; output:\n{out}"
    );
    assert!(
        out.contains("CAPTURED=probe 9.9.9"),
        "and what it said must be captured, because that string becomes the provenance \
         of every number the lane prints; output:\n{out}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

// THE BRANCH NO TEST COULD REACH SAID THE WRONG THING TWICE.
//
// The fallback diagnosis — used when the digest's interpreter fails and writes nothing — carried
// the `'"'"'` idiom for a literal quote, which is correct in a single-quoted context and was
// pasted into a double-quoted one where a quote is already literal, so it printed `'''/path'''`.
// Its advice ("check that python3 is on PATH") also named a case that cannot arrive there: a
// missing interpreter writes "command not found" INTO the capture, so the captured detail is used
// instead and the fallback never fires. Both survived because no test reached the branch — the
// existing scratch-independence test passes a nonexistent directory, which makes Python write a
// `FAIL:` line, so the captured detail is non-empty.
//
// Shadowing `python3` with a shell function reaches it: a function is found before a command on
// PATH, so this produces exactly the observed state the branch is written for — a non-zero status
// with an empty capture, which is what a killed interpreter looks like.
#[test]
fn the_digest_fallback_names_the_directory_once_and_quotes_the_status_it_has() {
    let (code, out) =
        in_lane_common("python3() { return 137; }\nlane_query_set_digest /some/arena");
    assert_ne!(code, 0, "a failing digest must refuse; output:\n{out}");
    assert!(
        out.contains("at '/some/arena'"),
        "the path must be quoted with ONE quote on each side; output:\n{out:?}"
    );
    assert!(
        !out.contains("'''"),
        "three quotes means the literal-quote idiom was pasted into a context that did not \
         need it; output:\n{out:?}"
    );
    assert!(
        out.contains("exited\n  137") || out.contains("exited 137"),
        "the status is the only evidence this branch has and must appear; output:\n{out:?}"
    );
    assert!(
        !out.contains("python3 is on PATH"),
        "advice naming a state that cannot reach this branch sends a reader to check the one \
         thing that is not wrong; output:\n{out:?}"
    );
}

// A HELPER THAT MOVED OUT OF `LANE_TMP` STILL OWES THE TRAP ITS PATH.
//
// The digest captures its interpreter's stderr with `mktemp` rather than into `LANE_TMP`, and
// that is deliberate — a certificate must not depend on a directory whose failure it is the thing
// reporting. What it cost was the cleanup guarantee `LANE_TMP` provided for free: an interrupt
// mid-digest left the capture behind in TMPDIR, because the trap only knew about `LANE_TMP`.
// Independence from the scratch directory must not be bought with a leak.
#[test]
fn a_capture_taken_outside_the_scratch_directory_is_still_swept_on_a_signal() {
    let root = scratch("stray-sweep");

    // The script interrupts ITSELF mid-digest, so the observation needs no external signaller
    // and no wall-clock race: `python3` is shadowed by a function that raises SIGINT and then
    // blocks, guaranteeing the digest is in flight when the signal lands.
    let (_code, out) = in_lane_common_with_tmpdir(
        "python3() { kill -INT $$; sleep 30; }\nlane_query_set_digest /some/arena",
        &root,
    );
    let leaked: Vec<String> = std::fs::read_dir(&root)
        .expect("enumerate the private TMPDIR")
        .map(|entry| {
            entry
                .expect("read a TMPDIR entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| name.starts_with("tmp."))
        .collect();
    assert!(
        leaked.is_empty(),
        "the capture file must not survive an interrupt: {leaked:?} left in TMPDIR; output:\n{out}"
    );

    // THE NEIGHBOUR: the sweep must not be achieved by never creating the file, or by the
    // trap removing something it should not. A successful digest still returns its value, and
    // it too leaves nothing behind.
    let queries = root.join("queries");
    std::fs::create_dir_all(&queries).expect("create the query directory");
    write(&queries.join("q1.rq"), b"SELECT * WHERE { ?s ?p ?o }\n");
    let (code, out) = in_lane_common_with_tmpdir(
        &format!("lane_query_set_digest '{}'", queries.display()),
        &root,
    );
    assert_eq!(code, 0, "a valid set must still certify; output:\n{out}");
    assert_eq!(
        out.trim().len(),
        64,
        "and it must be a digest, so the sweep is not hiding a helper that stopped working; \
         output:\n{out:?}"
    );
    let survivors: Vec<String> = std::fs::read_dir(&root)
        .expect("enumerate the private TMPDIR")
        .map(|entry| {
            entry
                .expect("read a TMPDIR entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| name.starts_with("tmp."))
        .collect();
    assert!(
        survivors.is_empty(),
        "a successful digest must leave nothing behind either: {survivors:?}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

// A SCRATCH FAILURE MUST NOT BE PUBLISHED AS A FAILURE OF THE BINARY UNDER TEST.
//
// Both lanes carried their own copy of the same two lines for capturing the binary's stderr, and
// both copies had the defect this file records one helper over: an unconditional `cat` of a
// `LANE_TMP` file. In a query loop the consequence is worse than in a digest, because the loop
// attributes blame per row. When the scratch directory is unusable the REDIRECT fails, so the
// binary never runs; `rc` is 1 with an empty capture, and every row reads "the binary exited 1
// without saying anything", ending in a summary reporting that not one query executed.
#[test]
fn an_unwritable_capture_path_is_a_scratch_failure_named_as_one() {
    let root = scratch("capture-path");
    // A regular file as the parent directory: every open beneath it is ENOTDIR for every uid,
    // so this is uncreatable for root as well and the test does not depend on who runs it.
    let blocker = root.join("not-a-directory");
    write(&blocker, b"x");
    let unwritable = blocker.join("query.err");

    let (code, out) = in_lane_common(&format!(
        "lane_reset_capture '{}'\necho REACHED-THE-QUERY",
        unwritable.display()
    ));
    assert_ne!(
        code, 0,
        "an uncreatable capture path must refuse; output:\n{out}"
    );
    assert!(
        !out.contains("REACHED-THE-QUERY"),
        "the refusal must precede the redirect that depends on the path; output:\n{out}"
    );
    assert!(
        out.contains("scratch directory"),
        "the diagnosis must name the scratch directory — the whole point is that it is NOT the \
         binary, the corpus or a knob; output:\n{out}"
    );
    for misblamed in ["the binary", "corpus", "knob you set"] {
        // Each of these appears in the message only as something explicitly EXCLUDED. What must
        // not happen is the old behaviour: silence here and `the binary exited 1` per row.
        assert!(
            out.contains(misblamed),
            "the message must say what is not at fault, because the failure it replaces blamed \
             exactly those things; output:\n{out}"
        );
    }

    // THE NEIGHBOUR: an ordinary path under the scratch directory is accepted and truncated.
    let (code, out) = in_lane_common(
        "lane_reset_capture \"${LANE_TMP}/query.err\"\n\
         printf 'stale bytes' >\"${LANE_TMP}/query.err\"\n\
         lane_reset_capture \"${LANE_TMP}/query.err\"\n\
         printf 'size=%s\\n' \"$(wc -c <\"${LANE_TMP}/query.err\")\"",
    );
    assert_eq!(
        code, 0,
        "a usable capture path must be accepted; output:\n{out}"
    );
    assert!(
        out.contains("size=0"),
        "and resetting it must truncate, so one query cannot inherit the previous query's \
         message; output:\n{out}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_capture_is_read_only_when_there_is_something_to_read() {
    let root = scratch("capture-read");
    let empty = root.join("empty.err");
    write(&empty, b"");
    let spoken = root.join("spoken.err");
    write(&spoken, b"error: line one\nerror: line two\n");
    let missing = root.join("never-created.err");

    // The ordinary case, and the one the unconditional `cat` got wrong: a binary that succeeded
    // said nothing. That must be the empty string and NOT a `cat:` line on the lane's stderr.
    for (path, label) in [
        (&empty, "an empty capture"),
        (&missing, "a capture never created"),
    ] {
        let (code, out) = in_lane_common(&format!(
            "said=\"$(lane_capture_stderr '{}')\"\nprintf 'said=[%s]\\n' \"${{said}}\"",
            path.display()
        ));
        assert_eq!(code, 0, "{label} is not an error; output:\n{out}");
        assert!(
            out.contains("said=[]"),
            "{label} must flatten to the empty string; output:\n{out}"
        );
        assert!(
            !out.contains("cat:"),
            "{label} must not put a stray `cat:` complaint on the lane's own stderr, which is \
             where the report goes; output:\n{out}"
        );
    }

    // And the case it exists for: a real message survives whole, on one line.
    let (code, out) = in_lane_common(&format!(
        "said=\"$(lane_capture_stderr '{}')\"\nprintf 'said=[%s]\\n' \"${{said}}\"",
        spoken.display()
    ));
    assert_eq!(code, 0, "output:\n{out}");
    assert!(
        out.contains("said=[error: line one | error: line two]"),
        "both lines must survive, joined — a multi-line message dropped after the first line is \
         the defect `lane_flatten_detail` exists for; output:\n{out}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// Writes `contents` to `path`, makes it executable, and returns `path`.
fn write_executable(path: PathBuf, contents: &str) -> PathBuf {
    std::fs::write(&path, contents).expect("write the executable script");
    let mut permissions = std::fs::metadata(&path)
        .expect("stat the freshly written script")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&path, permissions).expect("make the script executable");
    path
}

#[test]
fn a_certificate_does_not_depend_on_the_scratch_directory() {
    // A previous version of the digest captured Python's stderr into `LANE_TMP` and read
    // it back unconditionally. A scratch directory that could not be written therefore
    // made a VALID query set fail — and fail with an EMPTY message, because there was
    // nothing to read. That is two defects at once: a certificate acquiring an unnamed
    // precondition, and a helper refusing with nothing to say.
    let root = scratch("scratch-independence");
    let queries = root.join("queries");
    std::fs::create_dir_all(&queries).expect("create the query directory");
    write(&queries.join("q1.rq"), b"SELECT * WHERE { ?s ?p ?o }\n");

    let (code, out) = in_lane_common(&format!("lane_query_set_digest '{}'", queries.display()));
    assert_eq!(code, 0, "output:\n{out}");
    let with_scratch = out.trim().to_string();

    // Same call with the scratch directory destroyed first.
    let (code, out) = in_lane_common(&format!(
        "rm -rf \"${{LANE_TMP}}\"\nlane_query_set_digest '{}'",
        queries.display()
    ));
    assert_eq!(
        code, 0,
        "a valid query set must still certify when the scratch directory is unusable — \
         the digest is a certificate and must not acquire an unnamed precondition; \
         output:\n{out}"
    );
    assert_eq!(
        out.trim(),
        with_scratch,
        "and it must be the SAME digest, not merely a successful one; output:\n{out}"
    );

    // And an invalid directory must still produce a diagnosis carrying the lane's name,
    // rather than an empty refusal.
    let (code, out) = in_lane_common(&format!(
        "rm -rf \"${{LANE_TMP}}\"\nlane_query_set_digest '{}'",
        queries.join("nowhere").display()
    ));
    assert_ne!(code, 0, "output:\n{out}");
    assert!(
        out.contains("probe:") && out.trim() != "probe:",
        "the refusal must carry the lane's name AND a reason — an empty diagnosis is \
         worse than the bare traceback it replaced; output:\n{out:?}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

// EVERY STDERR CAPTURE SITE IS GUARDED, ENUMERATED RATHER THAN SPOT-CHECKED.
//
// `lane_reset_capture` was written for the two query loops and applied to them, and the THIRD
// site — `lane_run_probe`, twelve lines below the comment block stating the law — was left raw.
// It was the site that mattered most: it runs before step 1 and asserts most loudly that the
// binary is at fault, so an unwritable scratch directory produced verbatim the sentence the law
// forbids ("the pinned binary is not a working …: it exited 1 when asked for its version").
//
// A behaviour test of the helper cannot catch an unadopted site, and neither can reading. So the
// subject here is COVERAGE: every redirection of a command's stderr into `LANE_TMP` must be
// preceded by a reset of that same path.
#[test]
fn every_stderr_capture_into_the_scratch_directory_is_reset_first() {
    // THIS TEST WAS DEFEATED FIVE WAYS BEFORE IT HELD, and every one of them is ordinary
    // shell. `line.contains("2>\"")` missed `2> "${x}"` (a space after the operator),
    // `2>${x}` (unquoted) and `2>>"${x}"` (appending) — and missed them INVISIBLY, without
    // even incrementing the counter, so the floor could not fire either. And
    // `ends_with("() {")` missed `function f {`, missed a header with trailing whitespace,
    // and treated a capture with no enclosing function as guarded by whatever reset
    // happened to appear earlier in the file — in a different, already-closed function.
    //
    // A coverage test that can be evaded is worth less than no coverage test, because it
    // reports the absence of a problem it cannot see.
    let redirect = regex_lite_capture();
    // A HEADER IS A SHAPE, NOT A SUFFIX. Requiring the line to END with `{` missed a
    // header carrying a trailing comment and one with the brace on the next line — and
    // each miss widened the search window so a reset inside a DIFFERENT, already-closed
    // function satisfied the capture. It also produced the mirror fault: a correctly
    // guarded capture under such a header was reported UNGUARDED, because no enclosing
    // function was found at all. Same defect shape, third rewrite.
    let function_header = |line: &str| {
        if line.starts_with(char::is_whitespace) {
            return false;
        }
        let head = line.split('#').next().unwrap_or("").trim_end();
        let named = head.split_once('(').is_some_and(|(name, rest)| {
            !name.trim().is_empty()
                && name
                    .trim()
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
                && rest.trim_start().starts_with(')')
        });
        let keyword = head.starts_with("function ");
        // The brace may close the line or open the next one.
        (named || keyword) && (head.ends_with('{') || head.ends_with(')') || keyword)
    };

    let mut unguarded: Vec<String> = Vec::new();
    let mut found = 0usize;

    for script in [
        "scripts/lane-common.sh",
        "scripts/lubm-lane.sh",
        "scripts/watdiv-lane.sh",
        // The third lane sources `lane-common.sh` and calls `lane_run_probe`. It has no
        // file-based capture today, so including it is what keeps that true.
        "scripts/scale-corpus.sh",
    ] {
        let text = std::fs::read_to_string(repo_root().join(script)).expect("read the lane script");
        let lines: Vec<&str> = text.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            if line.trim_start().starts_with('#') {
                continue;
            }
            let Some(target) = redirect(line) else {
                continue;
            };
            found += 1;
            // NO ENCLOSING FUNCTION MEANS UNGUARDED, not "scan from the top of the file".
            // Scanning from line 0 let a reset inside an unrelated closed function satisfy
            // a top-level capture.
            let Some(function_start) = lines[..index].iter().rposition(|l| function_header(l))
            else {
                unguarded.push(format!(
                    "{script}:{}: {} (no enclosing function, so no reset can precede it)",
                    index + 1,
                    line.trim()
                ));
                continue;
            };
            let guarded = lines[function_start + 1..index]
                .iter()
                .any(|earlier| earlier.contains("lane_reset_capture") && earlier.contains(&target));
            if !guarded {
                unguarded.push(format!("{script}:{}: {}", index + 1, line.trim()));
            }
        }
    }

    assert!(
        found >= 4,
        "expected at least the four known capture sites; found {found}. A coverage test \
         that covers nothing must fail loudly rather than pass — this floor is what caught \
         the detector missing two of three sites on its first attempt."
    );
    assert!(
        unguarded.is_empty(),
        "these stderr captures are not preceded by `lane_reset_capture` naming the same \
         target, inside the same function, so an unwritable scratch directory is reported \
         as a failure of the binary under test:\n  {}",
        unguarded.join("\n  ")
    );
}

/// Returns a closure extracting the target of a stderr redirection, in every form the lanes
/// use: `2>"x"`, `2> "x"`, `2>x`, `2>>"x"`.
///
/// Hand-rolled rather than pulling in a regex dependency for a test: the grammar is small and
/// writing it out keeps the evaded forms visible in one place.
fn regex_lite_capture() -> impl Fn(&str) -> Option<String> {
    |line: &str| {
        // EVERY `2>` ON THE LINE, not the first. `2>/dev/null` earlier on a line made the
        // whole line return None, so a real capture after it was invisible — and
        // invisible without incrementing the counter, so the floor could not fire either.
        line.match_indices("2>")
            .filter_map(|(at, _)| capture_target(line, at))
            .next()
    }
}

/// The target of the stderr redirection at byte offset `at`, if it is a file capture.
fn capture_target(line: &str, at: usize) -> Option<String> {
    {
        // `12>` is not a stderr redirection, and neither is `$2>`.
        if at > 0 {
            let before = line.as_bytes()[at - 1];
            if before.is_ascii_digit() || before == b'$' || before == b'&' {
                return None;
            }
        }
        // `2>|` overrides noclobber and is still a redirection to a file; it was not
        // counted at all.
        let rest = line[at + 2..]
            .trim_start_matches('>')
            .trim_start_matches('|')
            .trim_start();
        if rest.starts_with('&') {
            // `2>&1` and `2>&${fd}` duplicate a descriptor; there is no file to reset.
            return None;
        }
        // A SINGLE-QUOTED TARGET IS THE SAME TARGET. `2>'\${errors}'` never matched the
        // reset naming `"\${errors}"`, so a correctly guarded capture was reported
        // unguarded — the mirror of an evasion, and just as wrong.
        let target: String = if let Some(stripped) = rest.strip_prefix('"') {
            stripped.chars().take_while(|c| *c != '"').collect()
        } else if let Some(stripped) = rest.strip_prefix('\'') {
            stripped.chars().take_while(|c| *c != '\'').collect()
        } else {
            // An UNQUOTED target ends at whitespace OR at shell syntax. Taking only
            // "not whitespace" produced `/dev/null)"` from `2>/dev/null)" || count=""`,
            // which then failed to match the `/dev/null` exclusion below and reported two
            // discard sites as unguarded captures.
            rest.chars()
                .take_while(|c| !c.is_whitespace() && !matches!(c, ')' | ';' | '|' | '&' | '"'))
                .collect()
        };
        if target.is_empty() {
            return None;
        }
        // DISCARDING IS NOT CAPTURING. `2>/dev/null` throws stderr away deliberately and
        // there is nothing to make writable first, so it is not a capture site. Stated
        // rather than left as an unexplained exception: the stricter detector found both
        // of these the moment it started seeing unquoted targets, which is the detector
        // working, and calling them unguarded would be the over-refusal that gets a
        // coverage test deleted.
        if target == "/dev/null" {
            return None;
        }
        Some(target)
    }
}
