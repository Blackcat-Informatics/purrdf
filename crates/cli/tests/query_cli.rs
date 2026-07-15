// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! End-to-end `query` coverage that drives the BUILT `purrdf` binary
//! (`env!("CARGO_BIN_EXE_purrdf")`) — never the library — so every assertion pins the
//! shipped executable's behavior. All fixtures use `example.org`.
//!
//! ## The result-shape × format-kind dispatch this exercises
//!
//! `--results-format` is a superset of the four SPARQL-results serializations and the
//! nine RDF syntaxes; the result SHAPE selects which half is legal:
//!
//! * SELECT solutions / ASK boolean → a SPARQL-results format (json/xml/csv/tsv);
//! * a CONSTRUCT/DESCRIBE graph → an RDF syntax, serialized through the SAME universal
//!   sink `convert` uses (so a star-incapable target projects the RDF-1.2 statement
//!   layer and the loss ledger records the drop);
//! * a shape/format-kind mismatch (solutions/boolean + an RDF syntax, or a graph + a
//!   SPARQL-results format) is a hard error (exit non-zero).
//!
//! ## A note on CONSTRUCT reifiers and the universal-sink invariant
//!
//! A CONSTRUCT whose template uses the RDF-1.2 annotation syntax (`{| ... |}`) mints a
//! reifier that lives in the dataset's STATEMENT-LAYER overlay — so serializing the
//! result to a `carries_star = false` target (RDF/XML) PROJECTS the layer to base
//! quads and records the dropped rows, exactly as `convert` does. (A plain
//! `rdf:reifies <<( … )>>` triple that merely flows through a variable binding is a
//! nested triple TERM, not an overlay row, and RDF/XML would emit it via
//! `parseType="Triple"` instead — so the reifier test deliberately uses the annotation
//! syntax to drive the overlay/projection path.)

use std::process::{Command, Output, Stdio};

/// The path to the built `purrdf` binary this integration test target links against.
const PURRDF: &str = env!("CARGO_BIN_EXE_purrdf");

/// A default-graph fixture with rich term shapes (an IRI object and a plain literal),
/// enough to drive SELECT / ASK / CONSTRUCT / DESCRIBE over `example.org`.
const DATA_TTL: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "ex:alice ex:knows ex:bob .\n",
    "ex:alice ex:name \"Alice\" .\n",
);

/// A `Command` for the built `purrdf` binary.
fn purrdf() -> Command {
    Command::new(PURRDF)
}

/// Run `purrdf` with `args`, returning the captured [`Output`].
fn run(args: &[&str]) -> Output {
    purrdf().args(args).output().expect("spawn purrdf")
}

/// stdout of an [`Output`] as a `String`.
fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).expect("utf-8 stdout")
}

/// stderr of an [`Output`] as a `String`, for diagnostics + ledger assertions.
fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// Write `contents` to `dir/name` and return the path as an owned string.
fn write_file(dir: &std::path::Path, name: &str, contents: &str) -> String {
    let p = dir.join(name);
    std::fs::write(&p, contents).expect("write fixture");
    p.to_str().expect("utf-8 path").to_owned()
}

/// The SAME SELECT over (a) a Turtle file and (b) an mmap'd `.purrpck` pack built from
/// identical data yields byte-identical, non-vacuous results — file/pack query parity.
#[test]
fn select_file_and_pack_are_byte_identical_and_non_vacuous() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "data.ttl", DATA_TTL);
    let query = "SELECT ?o WHERE { ?s <http://example.org/knows> ?o }";

    // (a) Query the Turtle file.
    let file_out = run(&["query", "--data", &ttl, "--results-format", "json", query]);
    assert!(
        file_out.status.success(),
        "query over the turtle file must exit 0; stderr:\n{}",
        stderr(&file_out)
    );
    let file_json = stdout(&file_out);
    assert!(
        file_json.contains("http://example.org/bob"),
        "the SELECT must bind at least one row (ex:bob); got:\n{file_json}"
    );

    // (b) Build a pack from the same data, then query it (mmap'd, zero-copy).
    let pack = write_file(dir, "data.purrpck", "");
    let build = run(&["convert", "--from", "turtle", "--to", "pack", &ttl, &pack]);
    assert!(
        build.status.success(),
        "building the pack must exit 0; stderr:\n{}",
        stderr(&build)
    );
    let pack_out = run(&["query", "--data", &pack, "--results-format", "json", query]);
    assert!(
        pack_out.status.success(),
        "query over the pack must exit 0; stderr:\n{}",
        stderr(&pack_out)
    );

    assert_eq!(
        file_json,
        stdout(&pack_out),
        "the SELECT result must be byte-identical whether queried over the turtle file or its pack"
    );
}

/// Each of the four SPARQL-results formats (json / xml / csv / tsv) serializes a SELECT
/// non-vacuously and DETERMINISTICALLY (two runs are byte-identical). TSV and XML are
/// explicit acceptance criteria and are covered here alongside JSON and CSV.
#[test]
fn select_all_four_result_formats_are_non_vacuous_and_deterministic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "data.ttl", DATA_TTL);
    let query = "SELECT ?o WHERE { ?s <http://example.org/knows> ?o }";

    for fmt in ["json", "xml", "csv", "tsv"] {
        let first = run(&["query", "--data", &ttl, "--results-format", fmt, query]);
        assert!(
            first.status.success(),
            "SELECT --results-format {fmt} must exit 0; stderr:\n{}",
            stderr(&first)
        );
        let body = stdout(&first);
        assert!(
            body.contains("http://example.org/bob"),
            "SELECT --results-format {fmt} must be non-vacuous (bind ex:bob); got:\n{body}"
        );

        // Deterministic: a second identical run yields byte-identical bytes.
        let second = run(&["query", "--data", &ttl, "--results-format", fmt, query]);
        assert!(second.status.success(), "second {fmt} run must exit 0");
        assert_eq!(
            first.stdout, second.stdout,
            "SELECT --results-format {fmt} must be byte-deterministic across runs"
        );
    }
}

/// An ASK with `--results-format json` returns a JSON boolean result (the W3C
/// `{"head":{},"boolean":true}` shape).
#[test]
fn ask_json_returns_a_boolean_shape() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "data.ttl", DATA_TTL);

    let out = run(&[
        "query",
        "--data",
        &ttl,
        "--results-format",
        "json",
        "ASK { ?s <http://example.org/knows> ?o }",
    ]);
    assert!(
        out.status.success(),
        "ASK json must exit 0; stderr:\n{}",
        stderr(&out)
    );
    let body = stdout(&out);
    assert!(
        body.contains("\"boolean\"") && body.contains("true"),
        "the ASK JSON must carry a boolean result (true); got:\n{body}"
    );
}

/// A CONSTRUCT to Turtle AND a DESCRIBE to Turtle both surface their triples in the RDF
/// output (the graph → RDF-syntax half of the dispatch).
#[test]
fn construct_and_describe_to_turtle_surface_their_triples() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "data.ttl", DATA_TTL);

    // CONSTRUCT a fresh `ex:friend` edge from the `ex:knows` edge.
    let construct = run(&[
        "query",
        "--data",
        &ttl,
        "--results-format",
        "turtle",
        "CONSTRUCT { ?s <http://example.org/friend> ?o } WHERE { ?s <http://example.org/knows> ?o }",
    ]);
    assert!(
        construct.status.success(),
        "CONSTRUCT -> turtle must exit 0; stderr:\n{}",
        stderr(&construct)
    );
    let construct_body = stdout(&construct);
    assert!(
        construct_body.contains("http://example.org/friend")
            && construct_body.contains("http://example.org/bob"),
        "the CONSTRUCTed triple must appear in the Turtle output; got:\n{construct_body}"
    );

    // DESCRIBE alice: her two triples must appear.
    let describe = run(&[
        "query",
        "--data",
        &ttl,
        "--results-format",
        "turtle",
        "DESCRIBE <http://example.org/alice>",
    ]);
    assert!(
        describe.status.success(),
        "DESCRIBE -> turtle must exit 0; stderr:\n{}",
        stderr(&describe)
    );
    let describe_body = stdout(&describe);
    assert!(
        describe_body.contains("http://example.org/knows") && describe_body.contains("Alice"),
        "the DESCRIBEd triples must appear in the Turtle output; got:\n{describe_body}"
    );
}

/// A CONSTRUCT whose result carries an RDF-1.2 reifier (minted via the annotation
/// syntax `{| … |}`) serialized to a star-INcapable RDF format (RDF/XML) PROJECTS the
/// statement layer, and under `--loss-ledger` the ledger records the drop — the
/// universal-sink invariant, identical to `convert`'s behavior.
#[test]
fn construct_reifier_to_rdfxml_records_the_loss_ledger_drop() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(
        dir,
        "data.ttl",
        "@prefix ex: <http://example.org/> .\nex:s ex:p ex:o .\n",
    );

    let out = run(&[
        "--loss-ledger",
        "query",
        "--data",
        &ttl,
        "--results-format",
        "rdfxml",
        "CONSTRUCT { ?s ?p ?o {| <http://example.org/certainty> \"0.9\" |} } WHERE { ?s ?p ?o }",
    ]);
    assert!(
        out.status.success(),
        "CONSTRUCT reifier -> rdfxml must PROJECT (exit 0), not fail-close; stderr:\n{}",
        stderr(&out)
    );
    // The projected RDF/XML keeps the base triple but not the reifies binding.
    let body = stdout(&out);
    assert!(
        body.contains("http://example.org/s"),
        "the base triple must survive the projection; got:\n{body}"
    );
    assert!(
        !body.contains("reifies"),
        "the reifier binding must be projected away for a star-incapable target; got:\n{body}"
    );
    // The bare `--loss-ledger` renders the ledger to stderr, recording the dropped rows.
    let ledger = stderr(&out);
    assert!(
        ledger.contains("statement-rows-dropped"),
        "the loss ledger must record the dropped statement rows (universal-sink invariant); \
         got:\n{ledger}"
    );
}

/// A shape/format-kind mismatch is a hard error (exit non-zero, diagnostic on stderr):
/// `csv` on a CONSTRUCT graph, `turtle` on SELECT solutions, and `turtle` on an ASK
/// boolean.
#[test]
fn shape_format_mismatches_are_hard_errors() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "data.ttl", DATA_TTL);

    // A graph result with a SPARQL-results format.
    let graph_with_results = run(&[
        "query",
        "--data",
        &ttl,
        "--results-format",
        "csv",
        "CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }",
    ]);
    assert!(
        !graph_with_results.status.success(),
        "a CONSTRUCT graph with --results-format csv must fail"
    );
    assert!(
        !stderr(&graph_with_results).is_empty(),
        "the graph/results-format mismatch must print a diagnostic to stderr"
    );

    // Solutions with an RDF syntax.
    let solutions_with_rdf = run(&[
        "query",
        "--data",
        &ttl,
        "--results-format",
        "turtle",
        "SELECT ?o WHERE { ?s <http://example.org/knows> ?o }",
    ]);
    assert!(
        !solutions_with_rdf.status.success(),
        "a SELECT with --results-format turtle must fail"
    );
    assert!(
        !stderr(&solutions_with_rdf).is_empty(),
        "the solutions/RDF-syntax mismatch must print a diagnostic to stderr"
    );

    // A boolean with an RDF syntax.
    let boolean_with_rdf = run(&[
        "query",
        "--data",
        &ttl,
        "--results-format",
        "turtle",
        "ASK { ?s ?p ?o }",
    ]);
    assert!(
        !boolean_with_rdf.status.success(),
        "an ASK with --results-format turtle must fail"
    );
    assert!(
        !stderr(&boolean_with_rdf).is_empty(),
        "the boolean/RDF-syntax mismatch must print a diagnostic to stderr"
    );
}

/// `--entailment rdfs` materializes the RDFS closure IN MEMORY before querying: a SELECT
/// whose match requires `rdfs:subClassOf` entailment binds its row WITH the flag and
/// binds NOTHING without it.
#[test]
fn entailment_rdfs_reveals_a_binding_absent_without_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(
        dir,
        "sub.ttl",
        concat!(
            "@prefix ex: <http://example.org/> .\n",
            "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n",
            "ex:Dog rdfs:subClassOf ex:Animal .\n",
            "ex:rex a ex:Dog .\n",
        ),
    );
    // `ex:rex a ex:Animal` holds ONLY under the RDFS subClassOf closure.
    let query = "SELECT ?x WHERE { ?x a <http://example.org/Animal> }";

    // Without --entailment: no row.
    let plain = run(&["query", "--data", &ttl, "--results-format", "tsv", query]);
    assert!(
        plain.status.success(),
        "the plain query must exit 0; stderr:\n{}",
        stderr(&plain)
    );
    assert!(
        !stdout(&plain).contains("http://example.org/rex"),
        "ex:rex must NOT bind without --entailment; got:\n{}",
        stdout(&plain)
    );

    // With --entailment rdfs: the inferred binding appears.
    let entailed = run(&[
        "query",
        "--data",
        &ttl,
        "--entailment",
        "rdfs",
        "--results-format",
        "tsv",
        query,
    ]);
    assert!(
        entailed.status.success(),
        "--entailment rdfs query must exit 0; stderr:\n{}",
        stderr(&entailed)
    );
    assert!(
        stdout(&entailed).contains("http://example.org/rex"),
        "ex:rex must bind under --entailment rdfs (subClassOf closure); got:\n{}",
        stdout(&entailed)
    );
}

/// `--base` resolves relative IRIs in the DATA while parsing, so a query naming the
/// resolved absolute IRI matches.
#[test]
fn base_resolves_relative_iris_in_the_data() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    // A relative-IRI subject resolved against `--base`.
    let ttl = write_file(dir, "rel.ttl", "<thing> <http://example.org/p> \"hit\" .\n");

    let out = run(&[
        "query",
        "--data",
        &ttl,
        "--base",
        "http://example.org/base/",
        "--results-format",
        "tsv",
        "SELECT ?o WHERE { <http://example.org/base/thing> <http://example.org/p> ?o }",
    ]);
    assert!(
        out.status.success(),
        "--base query must exit 0; stderr:\n{}",
        stderr(&out)
    );
    assert!(
        stdout(&out).contains("hit"),
        "the relative subject must resolve against --base so the query matches; got:\n{}",
        stdout(&out)
    );
}

/// A truncated/garbage `.purrpck` passed to `--data` fails closed (exit non-zero): the
/// pack integrity verifier rejects it before any view is opened.
#[test]
fn corrupt_pack_fails_closed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let bad = write_file(dir, "bad.purrpck", "not a pack file at all — pure garbage");

    let out = run(&[
        "query",
        "--data",
        &bad,
        "--results-format",
        "json",
        "SELECT ?s WHERE { ?s ?p ?o }",
    ]);
    assert!(
        !out.status.success(),
        "a corrupt pack must fail closed (exit non-zero); stdout:\n{}",
        stdout(&out)
    );
    assert!(
        !stderr(&out).is_empty(),
        "the pack-integrity failure must print a diagnostic to stderr"
    );
}

/// A SELECT piped through stdout is stable enough to run as a process smoke (the binary
/// spawns, reads the file, and writes results) — a belt-and-suspenders check that the
/// query path does not deadlock on a captured stdout pipe.
#[test]
fn select_writes_results_to_a_captured_stdout_pipe() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "data.ttl", DATA_TTL);

    let child = purrdf()
        .args([
            "query",
            "--data",
            &ttl,
            "--results-format",
            "tsv",
            "SELECT ?o WHERE { ?s <http://example.org/knows> ?o }",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn purrdf query");
    let out = child.wait_with_output().expect("await query child");
    assert!(
        out.status.success(),
        "piped SELECT must exit 0; stderr:\n{}",
        stderr(&out)
    );
    assert!(
        stdout(&out).contains("http://example.org/bob"),
        "the piped TSV must carry the bound object; got:\n{}",
        stdout(&out)
    );
}
