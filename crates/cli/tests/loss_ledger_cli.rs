// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Dedicated end-to-end coverage of the global `--loss-ledger` flag, driving the
//! BUILT `purrdf` binary (`env!("CARGO_BIN_EXE_purrdf")`) — never the library — so
//! every assertion pins the shipped executable's surfacing contract. All fixtures
//! use `example.org`.
//!
//! Where `convert_cli.rs` / `query_cli.rs` assert *that* a particular conversion
//! records a particular loss code, this suite pins the flag's SURFACING semantics
//! independently:
//!
//! * **the tri-state** — absent → silent (nothing on stderr, clean conversion on
//!   stdout); bare `--loss-ledger` → the JSON goes to STDERR while STDOUT stays the
//!   clean conversion; `--loss-ledger=PATH` → the JSON is written to PATH
//!   byte-identically to what bare mode writes to stderr, with stderr empty;
//! * **the empty-vs-non-empty contract** — a lossy conversion yields a non-empty
//!   `losses` array carrying the realized/contract codes; a lossless one yields an
//!   empty `losses` array (the versioned envelope is still present, but no `"code"`
//!   field appears);
//! * **determinism** — the same lossy conversion's ledger JSON is byte-identical
//!   across runs;
//! * **the universal-sink invariant** — `query`-CONSTRUCT and `reason`, not just
//!   `convert`, feed the same sink, so both surface the ledger under the flag;
//! * **flag position** — `--loss-ledger` is `global = true`, so it is accepted both
//!   BEFORE the subcommand (`purrdf --loss-ledger convert …`) and AFTER it
//!   (`purrdf convert … --loss-ledger`), and both positions produce the same ledger.

use std::process::{Command, Output};

/// The path to the built `purrdf` binary this integration target links against.
const PURRDF: &str = env!("CARGO_BIN_EXE_purrdf");

/// SEED C — a star-free base quad + one reifier (`rdf:reifies` a quoted triple) +
/// one annotation on that reifier. Serializing it to a star-INcapable target
/// (RDF/XML) drops the RDF-1.2 statement layer, so the ledger is non-empty.
const SEED_C: &str = concat!(
    "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n",
    "<http://example.org/r> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> ",
    "<<( <http://example.org/s> <http://example.org/p> <http://example.org/o> )>> .\n",
    "<http://example.org/r> <http://example.org/certainty> \"0.9\" .\n",
);

/// A TriG document with one named graph plus a default-graph triple. Converting it
/// to a single-graph target (Turtle) drops the named graph, so the ledger records a
/// `named-graph-dropped` loss.
const NAMED_GRAPH_TRIG: &str = concat!(
    "<http://example.org/g1> {\n",
    "  <http://example.org/gs> <http://example.org/gp> <http://example.org/go> .\n",
    "}\n",
    "<http://example.org/d> <http://example.org/p> <http://example.org/o> .\n",
);

/// A plain, star-free, default-graph Turtle triple. Every syntax carries it
/// losslessly, so a conversion to N-Triples leaves the ledger empty.
const PLAIN_TTL: &str = "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n";

/// A `Command` for the built `purrdf` binary.
fn purrdf() -> Command {
    Command::new(PURRDF)
}

/// Run `purrdf` with `args`, returning the captured [`Output`].
fn run(args: &[&str]) -> Output {
    purrdf()
        .args(args)
        .output()
        .expect("spawn the built purrdf binary")
}

/// stdout of an [`Output`] as a `String`.
fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).expect("utf-8 stdout")
}

/// stderr of an [`Output`] as a `String`.
fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// Join a name onto `dir`, returning it as an owned `String`.
fn path(dir: &std::path::Path, name: &str) -> String {
    dir.join(name)
        .to_str()
        .expect("temp path is valid UTF-8")
        .to_owned()
}

/// Write `contents` to `dir/name`, returning the path.
fn write_file(dir: &std::path::Path, name: &str, contents: &str) -> String {
    let p = path(dir, name);
    std::fs::write(&p, contents).expect("write fixture file");
    p
}

/// A ledger JSON string is "non-empty" iff it carries at least one `"code"` field
/// (the versioned envelope always carries `schema_version` + a `losses` array, so
/// the envelope alone is not enough).
fn ledger_is_empty(json: &str) -> bool {
    assert!(
        json.contains("\"schema_version\""),
        "any surfaced ledger must carry the versioned envelope; got:\n{json}"
    );
    assert!(
        json.contains("\"losses\""),
        "any surfaced ledger must carry a `losses` array; got:\n{json}"
    );
    !json.contains("\"code\"")
}

/// A lossy `nquads -> rdfxml` conversion of SEED C (a reifier + annotation) records
/// the star-drop into a NON-EMPTY ledger under bare `--loss-ledger` (on stderr).
/// The binary emits the realized `statement-rows-dropped` code AND the contract
/// `rdf12-star-unrepresentable` code — both are asserted against the real output.
#[test]
fn lossy_nq_to_rdfxml_reifier_records_the_star_drop() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed = write_file(dir, "seedC.nq", SEED_C);
    let out = path(dir, "out.rdf");

    let o = run(&[
        "--loss-ledger",
        "convert",
        "--from",
        "nquads",
        "--to",
        "rdfxml",
        &seed,
        &out,
    ]);
    assert!(
        o.status.success(),
        "nq -> rdfxml must exit 0; stderr:\n{}",
        stderr(&o)
    );

    let ledger = stderr(&o);
    assert!(
        !ledger_is_empty(&ledger),
        "a lossy nq -> rdfxml must produce a non-empty ledger; got:\n{ledger}"
    );
    // The realized dropped-row code the serializer emits when it strips the star layer.
    assert!(
        ledger.contains("statement-rows-dropped"),
        "the ledger must record the realized statement-layer drop; got:\n{ledger}"
    );
    // The contract star-drop code for the (nquads -> rdfxml) pair.
    assert!(
        ledger.contains("rdf12-star-unrepresentable"),
        "the ledger must record the contract star-unrepresentable loss; got:\n{ledger}"
    );
}

/// A lossy `trig -> turtle` conversion dropping a named graph records the
/// `named-graph-dropped` code into a non-empty ledger under bare `--loss-ledger`.
#[test]
fn lossy_trig_to_turtle_records_the_named_graph_drop() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed = write_file(dir, "named.trig", NAMED_GRAPH_TRIG);
    let out = path(dir, "out.ttl");

    let o = run(&[
        "--loss-ledger",
        "convert",
        "--from",
        "trig",
        "--to",
        "turtle",
        &seed,
        &out,
    ]);
    assert!(
        o.status.success(),
        "trig -> turtle must exit 0; stderr:\n{}",
        stderr(&o)
    );

    let ledger = stderr(&o);
    assert!(
        !ledger_is_empty(&ledger),
        "a lossy trig -> turtle must produce a non-empty ledger; got:\n{ledger}"
    );
    assert!(
        ledger.contains("named-graph-dropped"),
        "the ledger must record the dropped named graph; got:\n{ledger}"
    );
}

/// A lossless `nquads -> trig` conversion (both dataset- AND star-capable) leaves the
/// `losses` array EMPTY: the versioned envelope is present but no `"code"` appears.
/// (SEED C's statement layer round-trips, so nothing is dropped.)
#[test]
fn lossless_nq_to_trig_has_empty_losses() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed = write_file(dir, "seedC.nq", SEED_C);
    let out = path(dir, "out.trig");
    let ledger_path = path(dir, "ledger.json");

    let o = run(&[
        &format!("--loss-ledger={ledger_path}"),
        "convert",
        "--from",
        "nquads",
        "--to",
        "trig",
        &seed,
        &out,
    ]);
    assert!(
        o.status.success(),
        "nq -> trig must exit 0; stderr:\n{}",
        stderr(&o)
    );

    let ledger = std::fs::read_to_string(&ledger_path).expect("read ledger json");
    assert!(
        ledger_is_empty(&ledger),
        "nq -> trig is lossless — the losses array must be empty (no \"code\"); got:\n{ledger}"
    );
}

/// A lossless star-free `turtle -> ntriples` conversion (default graph only) leaves
/// the `losses` array EMPTY.
#[test]
fn lossless_ttl_to_nt_has_empty_losses() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed = write_file(dir, "plain.ttl", PLAIN_TTL);
    let out = path(dir, "out.nt");
    let ledger_path = path(dir, "ledger.json");

    let o = run(&[
        &format!("--loss-ledger={ledger_path}"),
        "convert",
        "--from",
        "turtle",
        "--to",
        "ntriples",
        &seed,
        &out,
    ]);
    assert!(
        o.status.success(),
        "ttl -> nt must exit 0; stderr:\n{}",
        stderr(&o)
    );

    let ledger = std::fs::read_to_string(&ledger_path).expect("read ledger json");
    assert!(
        ledger_is_empty(&ledger),
        "ttl -> nt is lossless — the losses array must be empty (no \"code\"); got:\n{ledger}"
    );
}

/// ABSENT flag: a LOSSY conversion with NO `--loss-ledger` emits NOTHING on stderr,
/// and stdout carries ONLY the clean conversion output — the ledger never leaks into
/// the converted document. (trig -> turtle to stdout `-` drops a named graph.)
#[test]
fn absent_flag_is_silent_and_stdout_is_clean() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed = write_file(dir, "named.trig", NAMED_GRAPH_TRIG);

    let o = run(&["convert", "--from", "trig", "--to", "turtle", &seed, "-"]);
    assert!(
        o.status.success(),
        "trig -> turtle must exit 0; stderr:\n{}",
        stderr(&o)
    );

    // Nothing leaks to stderr.
    assert!(
        o.stderr.is_empty(),
        "an absent --loss-ledger must leave stderr EMPTY; got:\n{}",
        stderr(&o)
    );
    // stdout is the converted Turtle — the surviving default-graph triple — and NOT
    // any part of the ledger JSON.
    let body = stdout(&o);
    assert!(
        body.contains("http://example.org/d"),
        "stdout must carry the converted Turtle document; got:\n{body}"
    );
    assert!(
        !body.contains("schema_version")
            && !body.contains("\"losses\"")
            && !body.contains("named-graph-dropped"),
        "the ledger must NOT leak into the conversion on stdout; got:\n{body}"
    );
}

/// BARE `--loss-ledger`: the JSON goes to STDERR and the clean conversion goes to
/// STDOUT — the two streams are cleanly separated.
#[test]
fn bare_flag_separates_ledger_on_stderr_from_conversion_on_stdout() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed = write_file(dir, "named.trig", NAMED_GRAPH_TRIG);

    let o = run(&[
        "--loss-ledger",
        "convert",
        "--from",
        "trig",
        "--to",
        "turtle",
        &seed,
        "-",
    ]);
    assert!(
        o.status.success(),
        "trig -> turtle must exit 0; stderr:\n{}",
        stderr(&o)
    );

    // stdout: the clean converted Turtle, with no ledger contamination.
    let body = stdout(&o);
    assert!(
        body.contains("http://example.org/d"),
        "stdout must carry the converted Turtle; got:\n{body}"
    );
    assert!(
        !body.contains("schema_version") && !body.contains("named-graph-dropped"),
        "the ledger must NOT appear on stdout under bare --loss-ledger; got:\n{body}"
    );

    // stderr: the ledger JSON, non-empty, naming the dropped named graph.
    let ledger = stderr(&o);
    assert!(
        ledger.contains("\"schema_version\"") && ledger.contains("named-graph-dropped"),
        "the ledger JSON must be on stderr under bare --loss-ledger; got:\n{ledger}"
    );
}

/// `--loss-ledger=PATH`: the JSON is written to PATH byte-identically to what bare
/// mode writes to stderr for the SAME conversion; stderr is empty; stdout is the
/// clean conversion output.
#[test]
fn path_flag_matches_bare_stderr_byte_for_byte_and_leaves_streams_clean() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed = write_file(dir, "seedC.nq", SEED_C);

    // Bare mode: capture the ledger from stderr (output to a discardable file).
    let out_bare = path(dir, "bare.rdf");
    let bare = run(&[
        "--loss-ledger",
        "convert",
        "--from",
        "nquads",
        "--to",
        "rdfxml",
        &seed,
        &out_bare,
    ]);
    assert!(
        bare.status.success(),
        "bare nq -> rdfxml must exit 0; stderr:\n{}",
        stderr(&bare)
    );
    let bare_ledger_bytes = bare.stderr;

    // Path mode: the SAME conversion, ledger to a file, conversion to stdout `-`.
    let ledger_path = path(dir, "ledger.json");
    let path_run = run(&[
        &format!("--loss-ledger={ledger_path}"),
        "convert",
        "--from",
        "nquads",
        "--to",
        "rdfxml",
        &seed,
        "-",
    ]);
    assert!(
        path_run.status.success(),
        "path nq -> rdfxml must exit 0; stderr:\n{}",
        stderr(&path_run)
    );

    // stderr is empty when the ledger is redirected to a file.
    assert!(
        path_run.stderr.is_empty(),
        "--loss-ledger=PATH must leave stderr EMPTY; got:\n{}",
        stderr(&path_run)
    );
    // stdout carries the clean RDF/XML conversion (the base triple survives).
    let body = stdout(&path_run);
    assert!(
        body.contains("http://example.org/s") && !body.contains("schema_version"),
        "stdout must carry the clean RDF/XML conversion (no ledger); got:\n{body}"
    );

    // The file written by =PATH is byte-identical to bare mode's stderr.
    let file_ledger_bytes = std::fs::read(&ledger_path).expect("read ledger file");
    assert_eq!(
        file_ledger_bytes, bare_ledger_bytes,
        "--loss-ledger=PATH must write byte-identically to bare mode's stderr"
    );
}

/// The ledger JSON is DETERMINISTIC: the same lossy conversion produces byte-identical
/// ledger files across two runs.
#[test]
fn ledger_json_is_deterministic_across_runs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed = write_file(dir, "seedC.nq", SEED_C);
    let out = path(dir, "out.rdf");

    let ledger_a = path(dir, "a.json");
    let ledger_b = path(dir, "b.json");
    for ledger in [&ledger_a, &ledger_b] {
        let o = run(&[
            &format!("--loss-ledger={ledger}"),
            "convert",
            "--from",
            "nquads",
            "--to",
            "rdfxml",
            &seed,
            &out,
        ]);
        assert!(
            o.status.success(),
            "nq -> rdfxml must exit 0; stderr:\n{}",
            stderr(&o)
        );
    }

    assert_eq!(
        std::fs::read(&ledger_a).expect("read a"),
        std::fs::read(&ledger_b).expect("read b"),
        "the same lossy conversion's ledger JSON must be byte-identical across runs"
    );
}

/// `--loss-ledger` is a GLOBAL flag: it is accepted BOTH before the subcommand
/// (`purrdf --loss-ledger convert …`) and AFTER it (`purrdf convert … --loss-ledger`),
/// and both positions surface the SAME ledger.
#[test]
fn global_flag_works_before_and_after_the_subcommand() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed = write_file(dir, "named.trig", NAMED_GRAPH_TRIG);
    let out = path(dir, "out.ttl");

    // Documented position: BEFORE the subcommand.
    let before = run(&[
        "--loss-ledger",
        "convert",
        "--from",
        "trig",
        "--to",
        "turtle",
        &seed,
        &out,
    ]);
    assert!(
        before.status.success(),
        "flag before the subcommand must exit 0; stderr:\n{}",
        stderr(&before)
    );
    assert!(
        stderr(&before).contains("named-graph-dropped"),
        "the flag before the subcommand must surface the ledger; got:\n{}",
        stderr(&before)
    );

    // `global = true` also accepts it AFTER the subcommand (trailing).
    let after = run(&[
        "convert",
        "--from",
        "trig",
        "--to",
        "turtle",
        &seed,
        &out,
        "--loss-ledger",
    ]);
    assert!(
        after.status.success(),
        "flag after the subcommand must exit 0; stderr:\n{}",
        stderr(&after)
    );
    assert!(
        stderr(&after).contains("named-graph-dropped"),
        "the flag after the subcommand must surface the ledger; got:\n{}",
        stderr(&after)
    );

    // Both positions produce the identical ledger bytes.
    assert_eq!(
        before.stderr, after.stderr,
        "the ledger must be identical whether --loss-ledger precedes or follows the subcommand"
    );
}

/// UNIVERSAL-SINK INVARIANT (query-CONSTRUCT): a CONSTRUCT whose result carries an
/// RDF-1.2 reifier (minted via the annotation syntax `{| … |}`) serialized to a
/// star-INcapable RDF format (RDF/XML) PROJECTS the statement layer, and — surfaced
/// here via `--loss-ledger=PATH` (a variant distinct from `query_cli.rs`, which pins
/// the bare-stderr path) — the file ledger is non-empty. The graph has no source
/// codec, so the drop is recorded as the realized `statement-rows-dropped`.
#[test]
fn query_construct_reifier_to_rdfxml_via_path_records_the_universal_drop() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(
        dir,
        "data.ttl",
        "@prefix ex: <http://example.org/> .\nex:s ex:p ex:o .\n",
    );
    let ledger_path = path(dir, "query.json");

    let o = run(&[
        &format!("--loss-ledger={ledger_path}"),
        "query",
        "--data",
        &ttl,
        "--results-format",
        "rdfxml",
        "CONSTRUCT { ?s ?p ?o {| <http://example.org/certainty> \"0.9\" |} } WHERE { ?s ?p ?o }",
    ]);
    assert!(
        o.status.success(),
        "CONSTRUCT reifier -> rdfxml must PROJECT (exit 0); stderr:\n{}",
        stderr(&o)
    );
    // stderr is empty (the ledger is redirected to the file); stdout carries the RDF/XML.
    assert!(
        o.stderr.is_empty(),
        "--loss-ledger=PATH must leave stderr EMPTY; got:\n{}",
        stderr(&o)
    );
    assert!(
        stdout(&o).contains("http://example.org/s"),
        "the base triple must survive the projection on stdout; got:\n{}",
        stdout(&o)
    );

    let ledger = std::fs::read_to_string(&ledger_path).expect("read query ledger");
    assert!(
        !ledger_is_empty(&ledger),
        "the query-CONSTRUCT universal-sink drop must produce a non-empty ledger; got:\n{ledger}"
    );
    assert!(
        ledger.contains("statement-rows-dropped"),
        "the query ledger must record the dropped statement rows; got:\n{ledger}"
    );
}

/// `reason` also feeds the universal sink: a `reason --regime simple` closure over
/// SEED C (a reifier + annotation) serialized to a star-INcapable target (RDF/XML)
/// drops the statement layer, and `--loss-ledger=PATH` surfaces the non-empty ledger —
/// proving the `reason` lane records losses exactly like `convert`/`query`.
#[test]
fn reason_closure_to_star_incapable_target_surfaces_the_ledger() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed = write_file(dir, "seedC.nq", SEED_C);
    let out = path(dir, "closure.rdf");
    let ledger_path = path(dir, "reason.json");

    let o = run(&[
        &format!("--loss-ledger={ledger_path}"),
        "reason",
        "--regime",
        "simple",
        &seed,
        &out,
    ]);
    assert!(
        o.status.success(),
        "reason simple -> rdfxml must exit 0; stderr:\n{}",
        stderr(&o)
    );
    // The closure keeps the base triple but drops the star layer under RDF/XML.
    let body = std::fs::read_to_string(&out).expect("read closure");
    assert!(
        body.contains("http://example.org/s") && !body.contains("reifies"),
        "the reason closure must project the statement layer for a star-incapable target; got:\n{body}"
    );

    let ledger = std::fs::read_to_string(&ledger_path).expect("read reason ledger");
    assert!(
        !ledger_is_empty(&ledger),
        "the reason universal-sink drop must produce a non-empty ledger; got:\n{ledger}"
    );
    assert!(
        ledger.contains("statement-rows-dropped"),
        "the reason ledger must record the dropped statement rows; got:\n{ledger}"
    );
}
