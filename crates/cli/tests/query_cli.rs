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
/// A normative RIF-in-XML rule document: `?x a ex:Cat` ⟹ `?x a ex:Animal`.
const RIF_RULES: &str = "<Document xmlns=\"http://www.w3.org/2007/rif#\"><payload><Group><sentence><Forall><declare><Var>x</Var></declare><formula><Implies><if><Frame><object><Var>x</Var></object><slot><Const type=\"http://www.w3.org/2007/rif#iri\">http://www.w3.org/1999/02/22-rdf-syntax-ns#type</Const><Const type=\"http://www.w3.org/2007/rif#iri\">http://example.org/Cat</Const></slot></Frame></if><then><Frame><object><Var>x</Var></object><slot><Const type=\"http://www.w3.org/2007/rif#iri\">http://www.w3.org/1999/02/22-rdf-syntax-ns#type</Const><Const type=\"http://www.w3.org/2007/rif#iri\">http://example.org/Animal</Const></slot></Frame></then></Implies></formula></Forall></sentence></Group></payload></Document>";

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

/// `query --entailment` answers under EVERY regime, sharing `reason`'s resolution
/// (`crates/cli/src/reason.rs::EntailmentPlan`, wired into `query` via
/// `crates/cli/src/query.rs`).
///
/// Falsifiable against the old behavior: `owl-direct` and `rif` exited code 3 here with
/// "cannot be materialized". `rif` now names its rule document with `--rules`; the other
/// six take none, and passing one is a usage error rather than a discarded argument.
#[test]
fn every_entailment_regime_answers() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "data.ttl", DATA_TTL);
    let rules = write_file(dir, "data.rif", RIF_RULES);
    let query = "SELECT ?o WHERE { ?s <http://example.org/knows> ?o }";

    let ask = |extra: &[&str]| {
        let mut args = vec!["query", "--data", &ttl];
        args.extend_from_slice(extra);
        args.extend_from_slice(&["--results-format", "json", query]);
        run(&args)
    };
    for regime in ["simple", "rdf", "rdfs", "owl-rl", "owl-direct", "d"] {
        let out = ask(&["--entailment", regime]);
        assert!(
            out.status.success(),
            "query --entailment {regime} must answer; stderr:\n{}",
            stderr(&out)
        );
    }
    let out = ask(&["--entailment", "rif", "--rules", &rules]);
    assert!(
        out.status.success(),
        "query --entailment rif must answer under --rules; stderr:\n{}",
        stderr(&out)
    );
    let out = ask(&["--entailment", "rdfs", "--rules", &rules]);
    assert_eq!(
        out.status.code(),
        Some(2),
        "--rules for rdfs must be refused, not ignored; stderr:\n{}",
        stderr(&out)
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

#[test]
fn configured_jsonld_options_reach_graph_results_and_reject_select_results() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "data.ttl", DATA_TTL);
    let options = write_file(
        dir,
        "jsonld-options.json",
        r#"{"version":1,"mode":"context","prefixes":{"ex":"http://example.org/"}}"#,
    );
    let graph = run(&[
        "--jsonld-options",
        &options,
        "query",
        "--data",
        &ttl,
        "--results-format",
        "jsonld",
        "CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }",
    ]);
    assert!(graph.status.success(), "graph: {}", stderr(&graph));
    assert!(stdout(&graph).contains("ex:alice"));
    assert!(stdout(&graph).contains("ex:knows"));

    let select = run(&[
        "--jsonld-options",
        &options,
        "query",
        "--data",
        &ttl,
        "--results-format",
        "json",
        "SELECT ?s WHERE { ?s ?p ?o }",
    ]);
    assert_eq!(select.status.code(), Some(2));
    assert!(stderr(&select).contains("requires an RDF graph result"));
}

/// The `A ⊑ ∃r.B`, `a : A` ontology: the shape whose certain answer NO query-independent
/// augmentation over named terms can find, because no NAMED individual need be `r`-related to
/// anything — the axiom only entails that SOME element is.
const SOME_VALUES_FROM_TTL: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n",
    "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n",
    "ex:A a owl:Class .\n",
    "ex:B a owl:Class .\n",
    "ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:r ; owl:someValuesFrom ex:B ] .\n",
    "ex:a a ex:A .\n",
);

/// An ontology OUTSIDE the combined approach's Horn fragment: `owl:equivalentClass` is a class
/// axiom the TBox lowering does not express.
const EQUIVALENT_CLASS_TTL: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n",
    "ex:A a owl:Class .\n",
    "ex:B a owl:Class .\n",
    "ex:A owl:equivalentClass ex:B .\n",
    "ex:a a ex:A .\n",
);

/// THE COMBINED APPROACH IS REACHABLE FROM THE COMMAND LINE.
///
/// `SELECT ?x WHERE { ?x ex:r ?y . ?y a ex:B }` has the certain answer `ex:a`: every model of
/// the ontology has SOME `r`-successor of `ex:a` typed `ex:B`, so `ex:a` satisfies the pattern
/// under OWL Direct-Semantics entailment even though no triple — asserted or in the
/// whole-vocabulary augmentation — ever says any named individual is `r`-related to anything.
///
/// Falsifiable against the old behavior: this exact command returned an EMPTY binding list.
/// `query --entailment` open-coded "materialize the closure, then evaluate", and
/// `Materialization::OwlDirect(&[])` is the query-INDEPENDENT augmentation — so the binary
/// could not reach the query-directed combined approach the library implements, and a
/// capability with exactly one caller in the whole repository (its own test) was dark from
/// every host. The lane routes through `purrdf::query_with_entailment` now.
#[test]
fn entailment_owl_direct_answers_through_the_combined_approach() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "onto.ttl", SOME_VALUES_FROM_TTL);
    let out = run(&[
        "query",
        "--data",
        &ttl,
        "--entailment",
        "owl-direct",
        "--results-format",
        "json",
        "SELECT ?x WHERE { ?x <http://example.org/r> ?y . ?y a <http://example.org/B> }",
    ]);
    assert!(
        out.status.success(),
        "the query must answer; stderr:\n{}",
        stderr(&out)
    );
    let body = stdout(&out);
    assert!(
        body.contains("\"value\":\"http://example.org/a\""),
        "ex:a is a certain answer and must be returned:\n{body}"
    );
    // And the internal chase witness is not in the answer.
    assert!(
        !body.contains("purrdfCombinedWitness"),
        "a chase witness leaked into the CLI's answer:\n{body}"
    );

    // The same pattern with `?y` PROJECTED has no certain answer, because the witness is not a
    // term of the scoping graph — so the rows are empty rather than carrying the witness.
    let out = run(&[
        "query",
        "--data",
        &ttl,
        "--entailment",
        "owl-direct",
        "--results-format",
        "json",
        "SELECT ?y WHERE { ?x <http://example.org/r> ?y . ?y a <http://example.org/B> }",
    ]);
    let body = stdout(&out);
    assert!(
        body.contains("\"bindings\":[]"),
        "a witness must not bind a projected variable:\n{body}"
    );
}

/// The OPTIONAL whose only match is a witness leaves the row STANDING with `?y` unbound, and a
/// `CONSTRUCT` template never emits the witness label. Both over the shipped binary.
#[test]
fn entailment_owl_direct_keeps_the_optional_row_and_constructs_no_witness() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "onto.ttl", SOME_VALUES_FROM_TTL);
    let ask = |format: &str, query: &str| {
        run(&[
            "query",
            "--data",
            &ttl,
            "--entailment",
            "owl-direct",
            "--results-format",
            format,
            query,
        ])
    };

    // `ex:a a ex:A` is ASSERTED, so the row is an answer under any reading. A filter that
    // dropped the whole row because the OPTIONAL matched a witness returned nothing here.
    let out = ask(
        "json",
        "SELECT ?x ?y WHERE { ?x a <http://example.org/A> . \
         OPTIONAL { ?x <http://example.org/r> ?y . ?y a <http://example.org/B> } }",
    );
    let body = stdout(&out);
    assert!(
        body.contains("\"value\":\"http://example.org/a\""),
        "the OPTIONAL's row must survive with ?x bound:\n{body}"
    );
    // `?y` is in the header (it is projected) and absent from the binding object, which is
    // exactly how the W3C results serialization spells an UNBOUND cell.
    assert!(
        body.contains("\"vars\":[\"x\",\"y\"]"),
        "?y stays a projected variable:\n{body}"
    );
    assert!(
        !body.contains("\"y\":{"),
        "?y must be UNBOUND in the returned row, not a witness:\n{body}"
    );

    // A CONSTRUCT template variable is observable, so no solution supplies it and the emitted
    // graph is empty — rather than carrying a triple whose object is the internal label.
    let out = ask(
        "ntriples",
        "CONSTRUCT { ?x <http://example.org/saw> ?y } \
         WHERE { ?x <http://example.org/r> ?y . ?y a <http://example.org/B> }",
    );
    let body = stdout(&out);
    assert!(
        !body.contains("purrdfCombinedWitness"),
        "a witness label reached CONSTRUCT output:\n{body}"
    );
    assert!(body.trim().is_empty(), "expected no triples:\n{body}");

    // A `DESCRIBE` reaches dataset triples no variable names, so its graph is scrubbed: the
    // asserted type survives and the chase's witness-bearing role assertion does not.
    let out = ask("ntriples", "DESCRIBE <http://example.org/a>");
    let body = stdout(&out);
    assert!(
        !body.contains("purrdfCombinedWitness"),
        "a witness label reached DESCRIBE output:\n{body}"
    );
    assert!(
        body.contains("<http://example.org/A>"),
        "the description must not be empty:\n{body}"
    );
    assert!(
        !body.contains("<http://example.org/r>"),
        "the witness-bearing role assertion must be scrubbed:\n{body}"
    );

    // An aggregate is computed INSIDE the engine, so it sees the restricted sequence.
    let out = ask(
        "json",
        "SELECT (COUNT(?y) AS ?n) WHERE { ?x <http://example.org/r> ?y . ?y a <http://example.org/B> }",
    );
    let body = stdout(&out);
    assert!(
        body.contains("\"value\":\"0\""),
        "COUNT must not count chase witnesses:\n{body}"
    );
}

/// AN ONTOLOGY OUTSIDE THE HORN FRAGMENT SAYS SO, on the `--report` the operator reads.
///
/// Falsifiable against the old behavior: `Construct::NonHornTBox` had no producer anywhere, so
/// every fallback run reported an empty boundary list and `completeness exact` while three
/// prose sites promised the disclosure. The fallback still ANSWERS — the boundary is a
/// disclosure of which lane answered, not a refusal.
#[test]
fn entailment_owl_direct_reports_the_non_horn_tbox_boundary_on_the_fallback() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let equivalent = write_file(dir, "equivalent.ttl", EQUIVALENT_CLASS_TTL);
    let out = run(&[
        "query",
        "--data",
        &equivalent,
        "--entailment",
        "owl-direct",
        "--results-format",
        "json",
        "--report",
        "ASK { <http://example.org/a> a <http://example.org/B> }",
    ]);
    assert!(
        out.status.success(),
        "the fallback must still answer; stderr:\n{}",
        stderr(&out)
    );
    assert!(
        stdout(&out).contains("\"boolean\":true"),
        "the augmentation reads owl:equivalentClass and must answer true:\n{}",
        stdout(&out)
    );
    let report = stderr(&out);
    assert!(
        report.contains("\nboundary non-horn-tbox "),
        "the fallback must disclose the boundary:\n{report}"
    );
    assert!(
        report.contains("\ncompleteness exact-within-boundaries\n"),
        "naming a boundary narrows completeness:\n{report}"
    );

    // A run that STAYS in the fragment names it nowhere.
    let in_fragment = write_file(dir, "onto.ttl", SOME_VALUES_FROM_TTL);
    let out = run(&[
        "query",
        "--data",
        &in_fragment,
        "--entailment",
        "owl-direct",
        "--results-format",
        "json",
        "--report",
        "ASK { <http://example.org/a> a <http://example.org/A> }",
    ]);
    let report = stderr(&out);
    assert!(
        !report.contains("non-horn-tbox"),
        "the combined approach applied, so nothing fell back:\n{report}"
    );
}

/// `rdfs:subPropertyOf` — the axiom the applicability check used to IGNORE rather than lower or
/// refuse — disqualifies the combined approach, and the certain answer it licenses arrives
/// through the fallback's augmentation. Both halves on the shipped binary.
#[test]
fn a_sub_property_axiom_falls_back_and_still_answers() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(
        dir,
        "subprop.ttl",
        concat!(
            "@prefix ex: <http://example.org/> .\n",
            "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n",
            "ex:r rdfs:subPropertyOf ex:q .\n",
            "ex:a ex:r ex:b .\n",
        ),
    );
    let out = run(&[
        "query",
        "--data",
        &ttl,
        "--entailment",
        "owl-direct",
        "--results-format",
        "json",
        "--report",
        "SELECT ?x WHERE { ?x <http://example.org/q> ?y }",
    ]);
    assert!(
        out.status.success(),
        "the query must answer; stderr:\n{}",
        stderr(&out)
    );
    assert!(
        stdout(&out).contains("\"value\":\"http://example.org/a\""),
        "ex:a is a certain answer through ex:q:\n{}",
        stdout(&out)
    );
    assert!(
        stderr(&out).contains("\nboundary non-horn-tbox "),
        "a property axiom the lowering does not express must disqualify it:\n{}",
        stderr(&out)
    );
}

/// Numeric fixture for the statistical-aggregate end-to-end tests: three values
/// (1, 2, 3) on distinct subjects — `MEDIAN` folds this to `2`.
const NUMBERS_TTL: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "ex:s1 ex:value 1 .\n",
    "ex:s2 ex:value 2 .\n",
    "ex:s3 ex:value 3 .\n",
);

/// End-to-end: `--aggregate-namespace` actually COMPUTES `MEDIAN` through the built
/// CLI binary — not merely that the flag parses. This is the reachability gap the
/// flag closes: before it existed, `AggregateRegistry::register_statistical_aggregates`
/// was reachable only by embedding the Rust engine directly.
#[test]
fn aggregate_namespace_computes_median_through_the_cli() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "numbers.ttl", NUMBERS_TTL);
    let query = "SELECT (AGG(<https://example.org/agg#MEDIAN>, ?v) AS ?m) \
                 WHERE { ?s <http://example.org/value> ?v }";

    let out = run(&[
        "query",
        "--data",
        &ttl,
        "--aggregate-namespace",
        "https://example.org/agg#",
        "--results-format",
        "json",
        query,
    ]);
    assert!(
        out.status.success(),
        "the aggregate-namespace query must exit 0; stderr:\n{}",
        stderr(&out)
    );
    let json = stdout(&out);
    assert!(
        json.contains("\"value\":\"2\""),
        "MEDIAN of {{1, 2, 3}} is 2:\n{json}"
    );
}

/// Regression: the namespace stays caller-supplied with no fabricated default —
/// omitting `--aggregate-namespace` leaves the ten statistical names unregistered,
/// and the SAME typed error an ordinary unregistered custom-aggregate IRI already
/// produces surfaces here, unchanged (existing behaviour with the parameter absent
/// is unchanged).
#[test]
fn omitted_aggregate_namespace_leaves_the_statistical_set_unregistered() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "numbers.ttl", NUMBERS_TTL);
    let query = "SELECT (AGG(<https://example.org/agg#MEDIAN>, ?v) AS ?m) \
                 WHERE { ?s <http://example.org/value> ?v }";

    let out = run(&["query", "--data", &ttl, "--results-format", "json", query]);
    assert!(
        !out.status.success(),
        "an unregistered custom aggregate must be refused"
    );
    let err = stderr(&out);
    assert!(
        err.contains("aggregate") || err.contains("regist"),
        "the refusal must name the unregistered aggregate:\n{err}"
    );
}

/// End-to-end: `purrdf update`'s `--aggregate-namespace` reaches `MEDIAN` from a
/// `DELETE`/`INSERT … WHERE` clause through a nested `SELECT … GROUP BY` — the only
/// place SPARQL UPDATE's grammar admits an aggregate.
#[test]
fn aggregate_namespace_computes_median_through_a_cli_update() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "numbers.ttl", NUMBERS_TTL);
    let update = "PREFIX ex: <http://example.org/> \
                  INSERT { ex:summary ex:median ?m } \
                  WHERE { SELECT (AGG(<https://example.org/agg#MEDIAN>, ?v) AS ?m) \
                          WHERE { ?s ex:value ?v } }";

    let out = run(&[
        "update",
        "--data",
        &ttl,
        "--to",
        "ntriples",
        "--aggregate-namespace",
        "https://example.org/agg#",
        update,
    ]);
    assert!(
        out.status.success(),
        "the aggregate-namespace update must exit 0; stderr:\n{}",
        stderr(&out)
    );
    let ntriples = stdout(&out);
    assert!(
        ntriples.contains("<http://example.org/summary> <http://example.org/median> \"2\""),
        "MEDIAN of {{1, 2, 3}} inserted as 2:\n{ntriples}"
    );
}

/// End-to-end: `--aggregate-namespace` combined with `--entailment` COMPUTES `MEDIAN`
/// over the ENTAILED CLOSURE, not merely over the raw asserted data. The RDFS closure
/// entails `ex:s1`/`ex:s2`/`ex:s3` are `ex:Thing` (their only common ancestor beyond
/// `ex:value`'s domain assertion), so a query that could only match through the closure
/// pins that the closure — not the raw view — is what the aggregate folds over.
///
/// Before `query_with_entailment_governed` took a `QueryOptions` parameter, this exact
/// combination was refused BY NAME rather than silently doing nothing; it now runs.
#[test]
fn aggregate_namespace_computes_median_under_entailment_through_the_cli() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    const VALUES_WITH_DOMAIN_TTL: &str = concat!(
        "@prefix ex: <http://example.org/> .\n",
        "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n",
        "ex:value rdfs:domain ex:Thing .\n",
        "ex:s1 ex:value 1 .\n",
        "ex:s2 ex:value 2 .\n",
        "ex:s3 ex:value 3 .\n",
    );
    let ttl = write_file(dir, "numbers.ttl", VALUES_WITH_DOMAIN_TTL);
    let query = "SELECT (AGG(<https://example.org/agg#MEDIAN>, ?v) AS ?m) \
                 WHERE { ?s a <http://example.org/Thing> . ?s <http://example.org/value> ?v }";

    // Without --entailment: `rdfs:domain` is not applied, so no ?s is typed `ex:Thing` and
    // the aggregate folds an empty group (unbound, per this build's `MEDIAN` over no rows).
    let without = run(&[
        "query",
        "--data",
        &ttl,
        "--aggregate-namespace",
        "https://example.org/agg#",
        "--results-format",
        "json",
        query,
    ]);
    assert!(without.status.success(), "stderr:\n{}", stderr(&without));
    assert!(
        !stdout(&without).contains("\"value\":\"2\""),
        "without --entailment no ?s is typed ex:Thing, so MEDIAN must not see the rows:\n{}",
        stdout(&without)
    );

    let with = run(&[
        "query",
        "--data",
        &ttl,
        "--entailment",
        "rdfs",
        "--aggregate-namespace",
        "https://example.org/agg#",
        "--results-format",
        "json",
        query,
    ]);
    assert!(
        with.status.success(),
        "the aggregate-namespace + entailment query must exit 0; stderr:\n{}",
        stderr(&with)
    );
    let json = stdout(&with);
    assert!(
        json.contains("\"value\":\"2\""),
        "MEDIAN of {{1, 2, 3}} over the RDFS-entailed ex:Thing group is 2:\n{json}"
    );
}

/// End-to-end: `--explain` combined with `--aggregate-namespace` renders the plan WITH the
/// registered custom aggregate named in the receipt's `aggregates` block, rather than
/// refusing the combination by name.
///
/// Before `explain_query_with_aggregates`/`_view` existed, this exact combination was
/// refused: the engine had no aggregate-registry-aware explain entry, so `--explain` would
/// have described a run in which the query's own `Custom` aggregate call was refused as
/// unregistered.
#[test]
fn explain_with_aggregate_namespace_renders_the_aggregate_in_the_receipt() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "numbers.ttl", NUMBERS_TTL);
    let query = "SELECT (AGG(<https://example.org/agg#MEDIAN>, ?v) AS ?m) \
                 WHERE { ?s <http://example.org/value> ?v }";

    let out = run(&[
        "query",
        "--data",
        &ttl,
        "--aggregate-namespace",
        "https://example.org/agg#",
        "--explain",
        query,
    ]);
    assert!(
        out.status.success(),
        "--explain with --aggregate-namespace must exit 0; stderr:\n{}",
        stderr(&out)
    );
    let body = stdout(&out);
    assert!(
        body.contains("\naggregates\n"),
        "the explanation must carry the `aggregates` section: {body}"
    );
    assert!(
        body.contains("https://example.org/agg#MEDIAN"),
        "the registered aggregate must be named in the receipt: {body}"
    );
    // The answers are replaced, not accompanied: an EXPLAIN returns a plan, not rows.
    assert!(
        !body.contains("\"value\":\"2\""),
        "--explain prints the explanation INSTEAD of the answers: {body}"
    );
}

// ── `--provenance-namespace` (F6): reachable AND readable back ────────────────────

/// End-to-end: `--provenance-namespace PREFIX=IRI` actually anchors a populated
/// `purrdf` provenance extension in the emitted SPARQL-results JSON, and what this
/// binary writes, `purrdf_sparql_results::provenance_from_json` reads back — closing
/// the "writer emits something nothing can read back" gap.
#[test]
fn provenance_namespace_populates_and_round_trips_through_json() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "numbers.ttl", NUMBERS_TTL);
    let query = "SELECT ?v WHERE { ?s <http://example.org/value> ?v } ORDER BY ?v";

    let out = run(&[
        "query",
        "--data",
        &ttl,
        "--provenance-namespace",
        "prov=https://example.org/ns/prov#",
        "--results-format",
        "json",
        query,
    ]);
    assert!(
        out.status.success(),
        "the provenance-namespace query must exit 0; stderr:\n{}",
        stderr(&out)
    );
    let json = stdout(&out);
    assert!(
        json.contains("\"prov\":{"),
        "the additive prov member must appear: {json}"
    );
    assert!(
        json.contains("\"queryForm\":\"select\""),
        "queryForm must name the result kind: {json}"
    );
    assert!(
        json.contains("\"engine\":\"purrdf-sparql-eval\""),
        "engine must be populated: {json}"
    );
    assert!(
        json.contains("\"queryHash\":\"sha256:"),
        "queryHash must be a sha256 content hash of the query text: {json}"
    );

    let namespace =
        purrdf_sparql_results::ProvenanceNamespace::new("prov", "https://example.org/ns/prov#")
            .expect("valid namespace");
    let decoded = purrdf_sparql_results::provenance_from_json(json.as_bytes(), &namespace)
        .expect("the CLI's own output decodes back");
    assert_eq!(decoded.engine.as_deref(), Some("purrdf-sparql-eval"));
    assert!(
        decoded
            .query_hash
            .as_deref()
            .is_some_and(|h| h.starts_with("sha256:")),
        "decoded query_hash: {:?}",
        decoded.query_hash
    );
}

/// The same round trip through XML.
#[test]
fn provenance_namespace_populates_and_round_trips_through_xml() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "numbers.ttl", NUMBERS_TTL);
    let query = "SELECT ?v WHERE { ?s <http://example.org/value> ?v } ORDER BY ?v";

    let out = run(&[
        "query",
        "--data",
        &ttl,
        "--provenance-namespace",
        "prov=https://example.org/ns/prov#",
        "--results-format",
        "xml",
        query,
    ]);
    assert!(
        out.status.success(),
        "the provenance-namespace XML query must exit 0; stderr:\n{}",
        stderr(&out)
    );
    let xml = stdout(&out);
    assert!(
        xml.contains("<prov:provenance"),
        "the additive prov:provenance element must appear: {xml}"
    );

    let namespace =
        purrdf_sparql_results::ProvenanceNamespace::new("prov", "https://example.org/ns/prov#")
            .expect("valid namespace");
    let decoded = purrdf_sparql_results::provenance_from_xml(xml.as_bytes(), &namespace)
        .expect("the CLI's own XML output decodes back");
    assert_eq!(decoded.engine.as_deref(), Some("purrdf-sparql-eval"));
}

/// Regression: the namespace stays caller-supplied with no fabricated default —
/// omitting `--provenance-namespace` emits pure-W3C output, byte-unchanged from
/// before the flag existed.
#[test]
fn omitting_provenance_namespace_emits_pure_w3c_output() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "numbers.ttl", NUMBERS_TTL);
    let query = "SELECT ?v WHERE { ?s <http://example.org/value> ?v } ORDER BY ?v";

    let out = run(&["query", "--data", &ttl, "--results-format", "json", query]);
    assert!(out.status.success(), "stderr:\n{}", stderr(&out));
    let json = stdout(&out);
    assert!(
        !json.contains("\"prov\""),
        "no namespace was supplied: no additive member may appear: {json}"
    );
}

/// A malformed `--provenance-namespace` value (no `=`) is a usage error at parse
/// time, before the query even runs.
#[test]
fn malformed_provenance_namespace_is_a_usage_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "numbers.ttl", NUMBERS_TTL);

    let out = run(&[
        "query",
        "--data",
        &ttl,
        "--provenance-namespace",
        "not-a-prefix-iri-pair",
        "SELECT ?v WHERE { ?s <http://example.org/value> ?v }",
    ]);
    assert!(!out.status.success(), "a missing `=` must be refused");
    assert!(
        stderr(&out).contains("--provenance-namespace"),
        "the error must name the flag: {}",
        stderr(&out)
    );
}

/// `--provenance-namespace` beside `--explain` is refused rather than silently
/// dropped: `--explain` prints the plan INSTEAD of the answers, so the extension it
/// would anchor is never emitted.
#[test]
fn provenance_namespace_with_explain_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "numbers.ttl", NUMBERS_TTL);

    let out = run(&[
        "query",
        "--data",
        &ttl,
        "--provenance-namespace",
        "prov=https://example.org/ns/prov#",
        "--explain",
        "SELECT ?v WHERE { ?s <http://example.org/value> ?v }",
    ]);
    assert!(!out.status.success(), "the combination must be refused");
    assert!(
        stderr(&out).contains("--provenance-namespace"),
        "the error must name the flag: {}",
        stderr(&out)
    );
}

/// `--provenance-namespace` beside a CONSTRUCT query is refused: a graph result has
/// no SPARQL-results provenance extension to carry it.
#[test]
fn provenance_namespace_with_a_construct_result_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "data.ttl", DATA_TTL);

    let out = run(&[
        "query",
        "--data",
        &ttl,
        "--provenance-namespace",
        "prov=https://example.org/ns/prov#",
        "--results-format",
        "turtle",
        "CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }",
    ]);
    assert!(!out.status.success(), "the combination must be refused");
    assert!(
        stderr(&out).contains("--provenance-namespace"),
        "the error must name the flag: {}",
        stderr(&out)
    );
}
