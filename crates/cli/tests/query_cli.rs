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
/// Before the engine had an aggregate-registry-aware explain entry, this exact
/// combination was refused: `--explain` would have described a run in which the
/// query's own `Custom` aggregate call was refused as unregistered.
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

// ── `--provenance-namespace`: reachable AND readable back ─────────────────────────

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

/// `--provenance-namespace` beside `--results-format csv`/`tsv` is refused rather than
/// silently accepted and trimmed: CSV/TSV are pure-W3C value-only formats with no
/// extension point at all, unlike JSON/XML.
#[test]
fn provenance_namespace_with_csv_or_tsv_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "numbers.ttl", NUMBERS_TTL);

    for fmt in ["csv", "tsv"] {
        let out = run(&[
            "query",
            "--data",
            &ttl,
            "--provenance-namespace",
            "prov=https://example.org/ns/prov#",
            "--results-format",
            fmt,
            "SELECT ?v WHERE { ?s <http://example.org/value> ?v }",
        ]);
        assert!(
            !out.status.success(),
            "--provenance-namespace with --results-format {fmt} must be refused"
        );
        assert_eq!(out.status.code(), Some(2), "usage errors exit 2");
        assert!(
            stderr(&out).contains("--provenance-namespace"),
            "the error must name the flag for {fmt}: {}",
            stderr(&out)
        );
    }
}

/// `--explain` never reaches [`emit_result`](crate::query::emit_result): it prints the
/// plan as plain text and returns before any serializer runs. A named
/// `--results-format` used to be accepted and silently ignored there (identical
/// output for every choice); it is refused by name instead.
#[test]
fn results_format_with_explain_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "numbers.ttl", NUMBERS_TTL);

    for fmt in ["json", "xml", "csv", "tsv", "turtle"] {
        let out = run(&[
            "query",
            "--data",
            &ttl,
            "--explain",
            "--results-format",
            fmt,
            "SELECT ?v WHERE { ?s <http://example.org/value> ?v }",
        ]);
        assert!(
            !out.status.success(),
            "--explain with --results-format {fmt} must be refused"
        );
        assert_eq!(out.status.code(), Some(2), "usage errors exit 2");
        assert!(
            stderr(&out).contains("--results-format"),
            "the error must name the flag for {fmt}: {}",
            stderr(&out)
        );
    }
}

/// `--explain` never runs the serializer that produces a loss ledger: a bare
/// `--loss-ledger` used to be accepted, exit 0, and write nothing. It is refused by
/// name instead, and `--loss-ledger=PATH` leaves no file behind either.
#[test]
fn loss_ledger_with_explain_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "numbers.ttl", NUMBERS_TTL);
    let ledger_path = dir.join("ledger.json");

    let out = run(&[
        "query",
        "--data",
        &ttl,
        "--explain",
        "--loss-ledger",
        "SELECT ?v WHERE { ?s <http://example.org/value> ?v }",
    ]);
    assert!(!out.status.success(), "the combination must be refused");
    assert_eq!(out.status.code(), Some(2), "usage errors exit 2");
    assert!(
        stderr(&out).contains("--loss-ledger"),
        "the error must name the flag: {}",
        stderr(&out)
    );

    let out = run(&[
        &format!("--loss-ledger={}", ledger_path.display()),
        "query",
        "--data",
        &ttl,
        "--explain",
        "SELECT ?v WHERE { ?s <http://example.org/value> ?v }",
    ]);
    assert!(
        !out.status.success(),
        "the combination must be refused with a PATH value too"
    );
    assert!(
        !ledger_path.exists(),
        "a refused run must not write the ledger file"
    );
}

/// `--explain` never reaches the results serializer, so a configured
/// `--jsonld-options` document never reaches a serializer either: a bare
/// `--explain --jsonld-options FILE` used to be accepted, exit 0, and render the plan
/// as if the flag had never been named. It is refused by name instead.
#[test]
fn jsonld_options_with_explain_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "numbers.ttl", NUMBERS_TTL);
    let options = write_file(
        dir,
        "jsonld-options.json",
        r#"{"version":1,"mode":"expanded"}"#,
    );

    let out = run(&[
        "--jsonld-options",
        &options,
        "query",
        "--data",
        &ttl,
        "--explain",
        "SELECT ?v WHERE { ?s <http://example.org/value> ?v }",
    ]);
    assert!(!out.status.success(), "the combination must be refused");
    assert_eq!(out.status.code(), Some(2), "usage errors exit 2");
    assert!(
        stderr(&out).contains("--jsonld-options"),
        "the error must name the flag: {}",
        stderr(&out)
    );
}

/// `--rules FILE` names the rule document `--entailment rif` runs; without
/// `--entailment` at all it would otherwise be accepted by clap and silently do
/// nothing (`options.rules` is read only inside the `--entailment` lane) — refused by
/// name instead.
#[test]
fn rules_without_entailment_is_refused_by_name() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "numbers.ttl", NUMBERS_TTL);
    let rules = write_file(dir, "unused.rif", "<rdf:RDF xmlns:rdf=\"x\"></rdf:RDF>");

    let out = run(&[
        "query",
        "--data",
        &ttl,
        "--rules",
        &rules,
        "SELECT ?v WHERE { ?s <http://example.org/value> ?v }",
    ]);
    assert!(
        !out.status.success(),
        "--rules with no --entailment must be refused"
    );
    assert_eq!(out.status.code(), Some(2), "usage errors exit 2");
    assert!(
        stderr(&out).contains("--rules"),
        "the refusal must name --rules: {}",
        stderr(&out)
    );
}

// ── --path-relation: the CLI's property-function registration surface ─────────────
//
// `--path-relation` registers a PATH-WITNESS relation under a caller-supplied IRI,
// callable from predicate position as
// `?start <IRI> ( ?end ?pathId ?len ?step ?node ?edge )`. Unlike a property path — which
// answers only with the endpoint pair — it binds the DERIVATION: one row per hop,
// carrying the traversed statement as a first-class RDF 1.2 term, so `GROUP BY ?pathId`
// with `ORDER BY ?step` reassembles the whole walk inside the query language.
//
// Every assertion below drives the BUILT binary, because the point of this surface is
// that it is reachable from the shipped executable and not only from a Rust host.

/// A three-edge chain `a → b → c → d` over one predicate.
const CHAIN_TTL: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "ex:a ex:p ex:b .\n",
    "ex:b ex:p ex:c .\n",
    "ex:c ex:p ex:d .\n",
);

/// A diamond `a → {b, c} → d`: two distinct derivations reach `d` in two hops, which is
/// exactly where `mode=walk` and `mode=shortest` must disagree.
const DIAMOND_TTL: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "ex:a ex:p ex:b .\n",
    "ex:a ex:p ex:c .\n",
    "ex:b ex:p ex:d .\n",
    "ex:c ex:p ex:d .\n",
);

/// The relation IRI the tests register. `example.org`, caller-supplied: PurRDF mints no
/// vocabulary IRIs and there is no default namespace to fall back on.
const WALK_IRI: &str = "http://example.org/pf#walk";

/// A `--path-relation` value over `ex:p` in `mode`, with a stated envelope on every key.
fn walk_spec(mode: &str) -> String {
    format!(
        "iri={WALK_IRI};forward=http://example.org/p;min-hops=1;max-hops=4;\
         max-paths-per-seed=64;max-expansions=4096;mode={mode}"
    )
}

/// A call seeded at `ex:a`, projecting the four columns whose values are exactly pinnable
/// (`?pathId` is a content digest and is asserted structurally instead).
fn walk_query(order: &str) -> String {
    format!(
        "SELECT ?end ?len ?step ?node WHERE {{ <http://example.org/a> <{WALK_IRI}> \
         ( ?end ?pathId ?len ?step ?node ?edge ) }} ORDER BY {order}"
    )
}

/// **The demonstration.** A multi-hop chain, through the shipped binary, one row per hop
/// with `?step` an `xsd:integer` and `?node` the node that hop arrived at — the exact rows,
/// not a count.
///
/// Three walks leave `ex:a` (`→b`, `→b→c`, `→b→c→d`) and they emit `1 + 2 + 3 = 6` rows.
/// CSV rather than JSON because a CSV body is a byte-exact table; the JSON test below
/// covers the term-level shape CSV cannot show.
#[test]
fn path_relation_binds_every_hop_of_a_multi_hop_chain() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "chain.ttl", CHAIN_TTL);

    let out = run(&[
        "query",
        "--data",
        &ttl,
        "--results-format",
        "csv",
        "--path-relation",
        &walk_spec("walk"),
        &walk_query("?len ?step"),
    ]);

    assert!(
        out.status.success(),
        "the --path-relation query must exit 0; stderr:\n{}",
        stderr(&out)
    );
    assert_eq!(
        stdout(&out),
        concat!(
            "end,len,step,node\r\n",
            "http://example.org/b,1,1,http://example.org/b\r\n",
            "http://example.org/c,2,1,http://example.org/b\r\n",
            "http://example.org/c,2,2,http://example.org/c\r\n",
            "http://example.org/d,3,1,http://example.org/b\r\n",
            "http://example.org/d,3,2,http://example.org/c\r\n",
            "http://example.org/d,3,3,http://example.org/d\r\n",
        ),
        "one row per hop of every simple-prefix walk out of ex:a, in (len, step) order"
    );
}

/// `?pathId` groups the hops of ONE walk: `GROUP BY ?pathId` with the concatenation in
/// `?step` order reconstructs each route, which is the recipe the relation's own
/// documentation gives for recovering a whole walk without a list term.
#[test]
fn path_relation_path_id_groups_the_hops_of_one_walk() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "chain.ttl", CHAIN_TTL);

    let out = run(&[
        "query",
        "--data",
        &ttl,
        "--results-format",
        "csv",
        "--path-relation",
        &walk_spec("walk"),
        &format!(
            "SELECT ?end ?len (GROUP_CONCAT(?node; separator=\"->\") AS ?route) WHERE {{ \
             <http://example.org/a> <{WALK_IRI}> ( ?end ?pathId ?len ?step ?node ?edge ) \
             }} GROUP BY ?pathId ?end ?len ORDER BY ?len"
        ),
    ]);

    assert!(
        out.status.success(),
        "the GROUP BY ?pathId query must exit 0; stderr:\n{}",
        stderr(&out)
    );
    assert_eq!(
        stdout(&out),
        concat!(
            "end,len,route\r\n",
            "http://example.org/b,1,http://example.org/b\r\n",
            "http://example.org/c,2,http://example.org/b->http://example.org/c\r\n",
            "http://example.org/d,3,http://example.org/b->http://example.org/c->\
             http://example.org/d\r\n",
        ),
        "each ?pathId group is exactly one walk, concatenated in ?step order"
    );
}

/// `?edge` is an RDF 1.2 STATEMENT TERM, not a reified stand-in: SPARQL-results JSON
/// renders it as `"type":"triple"` with the asserted subject/predicate/object.
#[test]
fn path_relation_binds_each_hop_to_an_rdf_12_statement_term() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "chain.ttl", CHAIN_TTL);

    let out = run(&[
        "query",
        "--data",
        &ttl,
        "--results-format",
        "json",
        "--path-relation",
        &walk_spec("walk"),
        &format!(
            "SELECT ?edge WHERE {{ <http://example.org/a> <{WALK_IRI}> \
             ( ?end ?pathId ?len ?step ?node ?edge ) FILTER(?len = 1) }}"
        ),
    ]);

    assert!(
        out.status.success(),
        "the ?edge query must exit 0; stderr:\n{}",
        stderr(&out)
    );
    let body = stdout(&out);
    assert!(
        body.contains("\"type\":\"triple\""),
        "?edge must be a first-class RDF 1.2 statement term; got:\n{body}"
    );
    assert!(
        body.contains("\"subject\":{\"type\":\"uri\",\"value\":\"http://example.org/a\"}"),
        "the statement is the ASSERTED triple the hop traversed; got:\n{body}"
    );
}

/// `mode=shortest` yields ONE shortest witness per reachable pair; `mode=walk` yields
/// every simple-prefix witness. On a diamond the two answers differ by exactly the second
/// two-hop derivation of `ex:d`, which is the whole reason they are two registrations
/// rather than one relation with a runtime flag.
#[test]
fn path_relation_shortest_mode_yields_one_witness_per_pair_on_a_diamond() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "diamond.ttl", DIAMOND_TTL);
    let query = walk_query("?end ?len ?step");

    let shortest = run(&[
        "query",
        "--data",
        &ttl,
        "--results-format",
        "csv",
        "--path-relation",
        &walk_spec("shortest"),
        &query,
    ]);
    assert!(
        shortest.status.success(),
        "mode=shortest must exit 0; stderr:\n{}",
        stderr(&shortest)
    );
    assert_eq!(
        stdout(&shortest),
        concat!(
            "end,len,step,node\r\n",
            "http://example.org/b,1,1,http://example.org/b\r\n",
            "http://example.org/c,1,1,http://example.org/c\r\n",
            "http://example.org/d,2,1,http://example.org/b\r\n",
            "http://example.org/d,2,2,http://example.org/d\r\n",
        ),
        "one shortest witness per reachable (seed, end) pair"
    );

    let walk = run(&[
        "query",
        "--data",
        &ttl,
        "--results-format",
        "csv",
        "--path-relation",
        &walk_spec("walk"),
        &query,
    ]);
    assert!(
        walk.status.success(),
        "mode=walk must exit 0; stderr:\n{}",
        stderr(&walk)
    );
    assert_eq!(
        stdout(&walk),
        concat!(
            "end,len,step,node\r\n",
            "http://example.org/b,1,1,http://example.org/b\r\n",
            "http://example.org/c,1,1,http://example.org/c\r\n",
            "http://example.org/d,2,1,http://example.org/b\r\n",
            "http://example.org/d,2,1,http://example.org/c\r\n",
            "http://example.org/d,2,2,http://example.org/d\r\n",
            "http://example.org/d,2,2,http://example.org/d\r\n",
        ),
        "mode=walk keeps BOTH two-hop derivations of ex:d; mode=shortest keeps one"
    );
}

/// Without the flag the SAME query text is an ordinary triple pattern — the object-side
/// parentheses are an `rdf:List`, which this data does not contain — so it answers
/// nothing. Pinned rather than assumed: it is the difference the flag makes.
#[test]
fn without_the_flag_the_same_query_text_is_an_ordinary_triple_pattern() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "chain.ttl", CHAIN_TTL);

    let out = run(&[
        "query",
        "--data",
        &ttl,
        "--results-format",
        "csv",
        &walk_query("?len ?step"),
    ]);

    assert!(
        out.status.success(),
        "the unregistered call is a triple pattern, not an error; stderr:\n{}",
        stderr(&out)
    );
    assert_eq!(
        stdout(&out),
        "end,len,step,node\r\n",
        "with no --path-relation the predicate is data, and this data holds no such triple"
    );
}

/// Recognition is derived from the registry ONE-TO-ONE, so the parser's exact-IRI set and
/// the registered relations cannot disagree: a REGISTERED IRI is a call, and any OTHER
/// IRI stays an ordinary triple pattern that reads the data.
///
/// That one-to-one derivation is also why this binary cannot produce the third case —
/// an IRI the parser recognizes but no relation answers, which the engine hard-fails at
/// plan time with `no property function is registered for <…>`. There is no flag here
/// that adds to the recognition set without also registering, and that is the property
/// being pinned: the CLI has no spelling that degrades a recognized call into a silently
/// empty scan.
#[test]
fn only_the_registered_iri_becomes_a_call() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(
        dir,
        "chain-plus.ttl",
        concat!(
            "@prefix ex: <http://example.org/> .\n",
            "ex:a ex:p ex:b .\n",
            "ex:b ex:p ex:c .\n",
            "ex:c ex:p ex:d .\n",
            "ex:a <http://example.org/pf#other> ex:z .\n",
        ),
    );

    let out = run(&[
        "query",
        "--data",
        &ttl,
        "--results-format",
        "csv",
        "--path-relation",
        &walk_spec("walk"),
        "SELECT ?o WHERE { <http://example.org/a> <http://example.org/pf#other> ?o }",
    ]);

    assert!(
        out.status.success(),
        "an unregistered IRI must stay a triple pattern; stderr:\n{}",
        stderr(&out)
    );
    assert_eq!(
        stdout(&out),
        "o\r\nhttp://example.org/z\r\n",
        "registering <pf#walk> must not reclassify the merely-same-prefixed <pf#other>"
    );
}

/// Every malformed spec is refused by name, never coerced and never defaulted. Each row
/// is one rule of the grammar and the substring the diagnostic must carry.
#[test]
fn a_malformed_path_relation_spec_is_refused_by_name() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "chain.ttl", CHAIN_TTL);
    let query = walk_query("?len ?step");

    let cases: [(&str, &str); 6] = [
        // An unknown key.
        (
            "iri=http://example.org/pf#walk;forward=http://example.org/p;min-hops=1;\
             max-hops=4;max-paths-per-seed=64;max-expansions=99;mode=walk;depth=2",
            "`depth`",
        ),
        // A missing mandatory key, named.
        (
            "iri=http://example.org/pf#walk;forward=http://example.org/p;min-hops=1;\
             max-hops=4;max-paths-per-seed=64;mode=walk",
            "`max-expansions`",
        ),
        // No `forward=`/`inverse=` predicate at all.
        (
            "iri=http://example.org/pf#walk;min-hops=1;max-hops=4;max-paths-per-seed=64;\
             max-expansions=99;mode=walk",
            "names no predicate",
        ),
        // A mode that is neither of the two registrations.
        (
            "iri=http://example.org/pf#walk;forward=http://example.org/p;min-hops=1;\
             max-hops=4;max-paths-per-seed=64;max-expansions=99;mode=cheapest",
            "`mode=cheapest`",
        ),
        // A relative IRI in an IRI position.
        (
            "iri=pf#walk;forward=http://example.org/p;min-hops=1;max-hops=4;\
             max-paths-per-seed=64;max-expansions=99;mode=walk",
            "relative IRI reference",
        ),
        // A zero `min-hops`: the kernel's own envelope diagnostic, carried verbatim.
        (
            "iri=http://example.org/pf#walk;forward=http://example.org/p;min-hops=0;\
             max-hops=4;max-paths-per-seed=64;max-expansions=99;mode=walk",
            "min_hops must be at least 1",
        ),
    ];

    for (spec, expected) in cases {
        let out = run(&["query", "--data", &ttl, "--path-relation", spec, &query]);
        assert!(
            !out.status.success(),
            "`{spec}` must be refused; stdout:\n{}",
            stdout(&out)
        );
        assert!(
            stderr(&out).contains(expected),
            "the refusal of `{spec}` must contain {expected:?}; got:\n{}",
            stderr(&out)
        );
    }
}

/// One IRI declared twice across repeated flags is a usage error (exit 2), not a
/// silently shadowed relation — and not the abort `PropertyFunctionRegistry::register`
/// would otherwise be.
#[test]
fn one_relation_iri_declared_twice_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "chain.ttl", CHAIN_TTL);
    let spec = walk_spec("walk");

    let out = run(&[
        "query",
        "--data",
        &ttl,
        "--path-relation",
        &spec,
        "--path-relation",
        &spec,
        &walk_query("?len ?step"),
    ]);

    assert!(!out.status.success(), "a duplicate IRI must be refused");
    assert_eq!(out.status.code(), Some(2), "usage errors exit 2");
    assert!(
        stderr(&out).contains("twice"),
        "the refusal must say the IRI is declared twice: {}",
        stderr(&out)
    );
}

/// An alternative the data has no edges for contributes zero edges rather than failing:
/// an operator with a FIXED step vocabulary running the same command line across many
/// datasets supplies valid configuration every time, and a dataset that carries none of
/// those edges has a correct answer — the empty one. The relation is still a CALL, so
/// the empty answer comes from the traversal rather than from a triple pattern.
#[test]
fn a_path_relation_over_an_edgeless_predicate_answers_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "chain.ttl", CHAIN_TTL);

    let out = run(&[
        "query",
        "--data",
        &ttl,
        "--results-format",
        "csv",
        "--path-relation",
        &format!(
            "iri={WALK_IRI};forward=http://example.org/noSuchPredicate;min-hops=1;\
             max-hops=4;max-paths-per-seed=64;max-expansions=99;mode=walk"
        ),
        &walk_query("?len ?step"),
    ]);

    assert!(
        out.status.success(),
        "an edgeless alternative is valid configuration; stderr:\n{}",
        stderr(&out)
    );
    assert_eq!(
        stdout(&out),
        "end,len,step,node\r\n",
        "no edges, no walks — and the header proves the call was planned, not refused"
    );
}

/// An envelope the ENGINE refuses is refused whichever lane it reaches, and the message
/// names both the relation and the kernel's own diagnostic.
#[test]
fn a_path_relation_with_an_unbuildable_envelope_names_the_relation() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "chain.ttl", CHAIN_TTL);

    let out = run(&[
        "query",
        "--data",
        &ttl,
        "--path-relation",
        &format!(
            "iri={WALK_IRI};forward=http://example.org/p;min-hops=1;max-hops=99999;\
             max-paths-per-seed=64;max-expansions=99;mode=walk"
        ),
        &walk_query("?len ?step"),
    ]);

    assert!(!out.status.success(), "a max-hops past the cap is refused");
    let message = stderr(&out);
    assert!(
        message.contains(WALK_IRI),
        "must name the relation: {message}"
    );
    assert!(
        message.contains("exceeds the hard cap"),
        "must carry the kernel's own diagnostic: {message}"
    );
}

/// The GOVERNED lane carries the registry too: a relation's rows are charged like every
/// other row source, and an answer cap below the walk's row count trips (exit 3) rather
/// than reporting a short answer as complete.
#[test]
fn a_governed_query_over_a_path_relation_trips_on_an_answer_cap() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "chain.ttl", CHAIN_TTL);

    let complete = run(&[
        "query",
        "--data",
        &ttl,
        "--results-format",
        "csv",
        "--max-answers",
        "6",
        "--path-relation",
        &walk_spec("walk"),
        &walk_query("?len ?step"),
    ]);
    assert!(
        complete.status.success(),
        "a cap at the exact row count completes; stderr:\n{}",
        stderr(&complete)
    );
    assert_eq!(
        stdout(&complete).lines().count(),
        7,
        "a header plus six hop rows"
    );

    let tripped = run(&[
        "query",
        "--data",
        &ttl,
        "--results-format",
        "csv",
        "--max-answers",
        "2",
        "--path-relation",
        &walk_spec("walk"),
        &walk_query("?len ?step"),
    ]);
    assert_eq!(
        tripped.status.code(),
        Some(3),
        "a tripped governor exits 3; stderr:\n{}",
        stderr(&tripped)
    );
}

/// `--explain` reaches the same registry, so the rendered `relations` block NAMES the
/// registered relation instead of being empty. (Before this flag existed the CLI had no
/// registration surface at all, and the block was empty on every call.)
#[test]
fn explain_lists_the_registered_path_relation() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "chain.ttl", CHAIN_TTL);

    let out = run(&[
        "query",
        "--data",
        &ttl,
        "--explain",
        "--path-relation",
        &walk_spec("walk"),
        &walk_query("?len ?step"),
    ]);

    assert!(
        out.status.success(),
        "--explain with --path-relation must exit 0; stderr:\n{}",
        stderr(&out)
    );
    let body = stdout(&out);
    assert!(
        body.contains(WALK_IRI),
        "the explanation's relations block must name the registered relation; got:\n{body}"
    );
}

/// `purrdf update --path-relation` reaches an `INSERT … WHERE`, which is a triple-pattern
/// context exactly as a query's is, and the relation is read from the PRE-update state.
#[test]
fn update_registers_a_path_relation_for_its_where_clause() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "chain.ttl", CHAIN_TTL);
    let out_path = write_file(dir, "out.nt", "");

    let updated = run(&[
        "update",
        "--data",
        &ttl,
        "--output",
        &out_path,
        "--to",
        "ntriples",
        "--path-relation",
        &walk_spec("shortest"),
        &format!(
            "INSERT {{ <http://example.org/a> <http://example.org/reaches> ?end }} WHERE {{ \
             <http://example.org/a> <{WALK_IRI}> ( ?end ?pathId ?len ?step ?node ?edge ) }}"
        ),
    ]);
    assert!(
        updated.status.success(),
        "the update must exit 0; stderr:\n{}",
        stderr(&updated)
    );

    let body = std::fs::read_to_string(&out_path).expect("read the updated dataset");
    for local in ["b", "c", "d"] {
        assert!(
            body.contains(&format!(
                "<http://example.org/a> <http://example.org/reaches> \
                 <http://example.org/{local}> ."
            )),
            "the WHERE clause must have reached ex:{local} through the relation; got:\n{body}"
        );
    }
}

// ── `CONSTRUCT GRAPH` × the nine RDF `--results-format` syntaxes ────────────────────

/// The six `--results-format` RDF syntaxes that carry named graphs.
const QUAD_CAPABLE: [&str; 6] = ["trig", "nquads", "trix", "hextuples", "jsonld", "yamlld"];

/// The three `--results-format` RDF syntaxes that do NOT: single-graph syntaxes with
/// no named-graph construct at all.
const SINGLE_GRAPH: [&str; 3] = ["turtle", "ntriples", "rdfxml"];

/// A whole-template `CONSTRUCT GRAPH ex:out { … }` over [`DATA_TTL`].
const CONSTRUCT_ONE_GRAPH: &str = "PREFIX ex: <http://example.org/> \
     CONSTRUCT GRAPH ex:out { ?s ex:rel ?o } WHERE { ?s ex:knows ?o }";

/// A per-statement quad template writing into TWO named graphs, declared in the
/// template in `g2`-then-`g1` order so the refusal's ordering cannot be an accident of
/// template order.
const CONSTRUCT_TWO_GRAPHS: &str = "PREFIX ex: <http://example.org/> \
     CONSTRUCT { GRAPH ex:g2 { ?s ex:rel ?o } GRAPH ex:g1 { ?s ex:other ?o } } \
     WHERE { ?s ex:knows ?o }";

/// A per-statement quad template MIXING a default-graph triple with a named-graph
/// quad: the shape that would otherwise emit the default-graph half and drop the rest,
/// reporting a partial answer as a complete one.
const CONSTRUCT_MIXED: &str = "PREFIX ex: <http://example.org/> \
     CONSTRUCT { ?s ex:plain ?o GRAPH ex:named { ?s ex:rel ?o } } WHERE { ?s ex:knows ?o }";

/// Run `CONSTRUCT GRAPH` into one `--results-format`, with the bare `--loss-ledger` on
/// so a silent drop would have to show up as an empty ledger beside empty stdout.
fn construct_graph_into(format: &str, query: &str, ttl: &str) -> Output {
    run(&[
        "--loss-ledger",
        "query",
        "--data",
        ttl,
        "--results-format",
        format,
        query,
    ])
}

/// Every quad-capable `--results-format` EMITS the named graph a `CONSTRUCT GRAPH`
/// result carries: the graph IRI and the constructed statement both appear, and the
/// run exits 0.
#[test]
fn construct_graph_into_quad_capable_formats_emits_the_graph() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "data.ttl", DATA_TTL);

    for format in QUAD_CAPABLE {
        let out = construct_graph_into(format, CONSTRUCT_ONE_GRAPH, &ttl);
        assert_eq!(
            out.status.code(),
            Some(0),
            "CONSTRUCT GRAPH -> {format} must exit 0; stderr:\n{}",
            stderr(&out)
        );
        let body = stdout(&out);
        assert!(
            body.contains("http://example.org/out"),
            "{format} is dataset-capable and must name the graph; got:\n{body}"
        );
        assert!(
            body.contains("http://example.org/rel") && body.contains("http://example.org/bob"),
            "{format} must carry the constructed statement; got:\n{body}"
        );
    }
}

/// Every single-graph `--results-format` REFUSES a `CONSTRUCT GRAPH` result rather
/// than serializing it: exit 2 (a usage error — the caller named a graph in the query
/// and a format that cannot carry it), nothing on stdout, and a stderr message naming
/// BOTH the graph and the format and pointing at the quad-capable alternatives.
///
/// This is the regression net for the shipped behaviour it replaces: one statement in,
/// ZERO bytes out, an EMPTY loss ledger, and exit 0.
#[test]
fn construct_graph_into_single_graph_formats_is_refused_by_name() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "data.ttl", DATA_TTL);

    for format in SINGLE_GRAPH {
        let out = construct_graph_into(format, CONSTRUCT_ONE_GRAPH, &ttl);
        assert_eq!(
            out.status.code(),
            Some(2),
            "CONSTRUCT GRAPH -> {format} must be a usage refusal (exit 2); stderr:\n{}",
            stderr(&out)
        );
        assert!(
            stdout(&out).is_empty(),
            "a refused emission must write nothing to stdout; got:\n{}",
            stdout(&out)
        );
        let message = stderr(&out);
        assert!(
            message.contains("<http://example.org/out>"),
            "{format}'s refusal must NAME the offending graph; got:\n{message}"
        );
        assert!(
            message.contains(&format!("`{format}`")),
            "{format}'s refusal must name the format; got:\n{message}"
        );
        assert!(
            message.contains("trig/nquads/trix/hextuples/jsonld/yamlld"),
            "{format}'s refusal must point at a quad-capable alternative; got:\n{message}"
        );
        assert!(
            message.contains("DROPPED"),
            "{format}'s refusal must say the statements would be dropped; got:\n{message}"
        );
    }
}

/// A MULTI-graph template refused against a single-graph format names EVERY distinct
/// graph, in lexicographic order — not template order, and not a hash-map order that
/// could differ between runs.
#[test]
fn a_multi_graph_template_refusal_names_every_graph_in_order() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "data.ttl", DATA_TTL);

    let out = construct_graph_into("turtle", CONSTRUCT_TWO_GRAPHS, &ttl);
    assert_eq!(
        out.status.code(),
        Some(2),
        "a two-graph template -> turtle must be refused; stderr:\n{}",
        stderr(&out)
    );
    let message = stderr(&out);
    assert!(
        message.contains("2 named graphs"),
        "the refusal must count both graphs; got:\n{message}"
    );
    assert!(
        message.contains("(<http://example.org/g1>, <http://example.org/g2>)"),
        "the refusal must name both graphs, sorted (g1 before g2 despite the template \
         declaring g2 first); got:\n{message}"
    );
    // Deterministic: the identical run produces byte-identical stderr.
    let again = construct_graph_into("turtle", CONSTRUCT_TWO_GRAPHS, &ttl);
    assert_eq!(
        again.stderr, out.stderr,
        "the refusal message must be byte-deterministic across runs"
    );
}

/// A template MIXING default-graph triples with a named-graph quad is refused too: it
/// still carries a named graph, and serializing the default-graph half alone would be
/// a partial answer reported as a complete one.
#[test]
fn a_mixed_default_and_named_template_is_still_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "data.ttl", DATA_TTL);

    for format in SINGLE_GRAPH {
        let out = construct_graph_into(format, CONSTRUCT_MIXED, &ttl);
        assert_eq!(
            out.status.code(),
            Some(2),
            "a mixed default+named template -> {format} must be refused; stderr:\n{}",
            stderr(&out)
        );
        assert!(
            stdout(&out).is_empty(),
            "{format} must not emit the default-graph half of a refused result; got:\n{}",
            stdout(&out)
        );
        assert!(
            stderr(&out).contains("<http://example.org/named>"),
            "{format}'s refusal must name the graph; got:\n{}",
            stderr(&out)
        );
    }
    // The SAME template into a quad-capable format keeps both halves.
    let trig = construct_graph_into("trig", CONSTRUCT_MIXED, &ttl);
    assert_eq!(trig.status.code(), Some(0), "mixed -> trig must exit 0");
    let body = stdout(&trig);
    assert!(
        body.contains("http://example.org/plain") && body.contains("http://example.org/named"),
        "trig must carry both the default-graph triple and the named graph; got:\n{body}"
    );
}

/// A plain (default-graph) `CONSTRUCT` still serializes to every single-graph syntax,
/// byte for byte, with an EMPTY loss ledger. The refusal above triggers on a
/// non-default graph and on nothing else.
///
/// The expected bytes are pinned literally rather than probed, so a change to the
/// serializer's output shows up here as a diff rather than as a passing test.
#[test]
fn a_plain_construct_still_serializes_to_every_single_graph_syntax() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "data.ttl", DATA_TTL);
    let query = "CONSTRUCT { ?s <http://example.org/friend> ?o } \
                 WHERE { ?s <http://example.org/knows> ?o }";

    let expected: [(&str, &str); 3] = [
        (
            "turtle",
            concat!(
                "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n",
                "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n",
                "\n",
                "<http://example.org/alice> <http://example.org/friend> \
                 <http://example.org/bob> .\n",
            ),
        ),
        (
            "ntriples",
            "<http://example.org/alice> <http://example.org/friend> \
             <http://example.org/bob> .\n",
        ),
        (
            "rdfxml",
            concat!(
                "<?xml version=\"1.0\"?>\n",
                "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\" ",
                "xmlns:xsd=\"http://www.w3.org/2001/XMLSchema#\" ",
                "xmlns:ns0=\"http://example.org/\" rdf:version=\"1.2\">\n",
                "  <rdf:Description rdf:about=\"http://example.org/alice\">\n",
                "    <ns0:friend rdf:resource=\"http://example.org/bob\"/>\n",
                "  </rdf:Description>\n",
                "</rdf:RDF>\n",
            ),
        ),
    ];

    for (format, bytes) in expected {
        let out = construct_graph_into(format, query, &ttl);
        assert_eq!(
            out.status.code(),
            Some(0),
            "a default-graph CONSTRUCT -> {format} must exit 0; stderr:\n{}",
            stderr(&out)
        );
        assert_eq!(
            stdout(&out),
            bytes,
            "a default-graph CONSTRUCT -> {format} must emit the same bytes it always has"
        );
        let ledger = stderr(&out);
        assert!(
            ledger.contains("\"losses\": [\n  ]"),
            "a default-graph CONSTRUCT -> {format} loses nothing; got:\n{ledger}"
        );
    }
}

/// `DESCRIBE` shares the graph-result lane, so the same refusal covers it — and a
/// `DESCRIBE` over default-graph data carries no named graph, so it is untouched.
#[test]
fn describe_over_default_graph_data_is_never_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "data.ttl", DATA_TTL);

    for format in SINGLE_GRAPH {
        let out = construct_graph_into(format, "DESCRIBE <http://example.org/alice>", &ttl);
        assert_eq!(
            out.status.code(),
            Some(0),
            "DESCRIBE -> {format} over default-graph data must exit 0; stderr:\n{}",
            stderr(&out)
        );
        // `knows` is spelled `<http://example.org/knows>` in the line/Turtle family and
        // `ns0:knows` in RDF/XML, so the assertion pins the local name plus the object.
        let body = stdout(&out);
        assert!(
            body.contains("knows") && body.contains("http://example.org/bob"),
            "DESCRIBE -> {format} must still emit the described triples; got:\n{body}"
        );
    }
}

/// A template writing into MORE graphs than the message spells out reports a bounded,
/// deterministically-ordered sample plus an exact count of the tail — never a silent
/// truncation, and never an unbounded message.
#[test]
fn a_refusal_over_many_graphs_samples_deterministically_and_counts_the_tail() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "data.ttl", DATA_TTL);
    // Ten graphs, declared in reverse alphabetical order so neither the sample nor its
    // ordering can be an artifact of the template's own order.
    let mut template = String::from("PREFIX ex: <http://example.org/> CONSTRUCT {");
    for name in ["j", "i", "h", "g", "f", "e", "d", "c", "b", "a"] {
        use std::fmt::Write as _;
        write!(template, " GRAPH ex:{name} {{ ?s ex:r ?o }}").expect("write to a String");
    }
    template.push_str(" } WHERE { ?s ex:knows ?o }");

    let out = construct_graph_into("turtle", &template, &ttl);
    assert_eq!(
        out.status.code(),
        Some(2),
        "a ten-graph template -> turtle must be refused; stderr:\n{}",
        stderr(&out)
    );
    let message = stderr(&out);
    assert!(
        message.contains("10 named graphs"),
        "the count must be exact even though the list is sampled; got:\n{message}"
    );
    assert!(
        message.contains(
            "(<http://example.org/a>, <http://example.org/b>, <http://example.org/c>, \
             <http://example.org/d>, <http://example.org/e>, <http://example.org/f>, \
             <http://example.org/g>, <http://example.org/h>, and 2 more)"
        ),
        "the first eight graphs must be named in sorted order and the tail counted; \
         got:\n{message}"
    );
}

// ── SEP-0008 SHA-3 through the built binary ─────────────────────────────────────
//
// The evaluator's own tests pin the digests against NIST FIPS 202. They cannot pin
// what a HOST receives: the CLI parses the query text, evaluates it, and writes a
// SPARQL-results document, and every one of those three steps is a place a
// hyphen-bearing keyword or a fresh built-in can be lost. This block drives the
// shipped executable end to end, from query text to the JSON on stdout.

/// A fixture whose object is the NIST FIPS 202 example message `"abc"` — the message
/// every published SHA-3 known-answer table is written against, so the expected
/// digests below are citable values rather than recorded output.
const SHA3_DATA_TTL: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "ex:s ex:message \"abc\" .\n",
);

/// `(function name, SELECT alias, published FIPS 202 digest of "abc")`.
///
/// Provenance: NIST FIPS 202 publishes `"abc"` as a worked example for all four
/// SHA-3 sizes. Each value below was taken from that table and independently
/// cross-checked against two implementations that are not the code under test —
/// OpenSSL (`printf 'abc' | openssl dgst -sha3-256`) and CPython's `hashlib`
/// (`hashlib.new("sha3_256", b"abc").hexdigest()`), which agree with each other and
/// with these strings.
const SHA3_ABC_VECTORS: [(&str, &str, &str); 4] = [
    (
        "SHA3-224",
        "h224",
        "e642824c3f8cf24ad09234ee7d3c766fc9a3a5168d0c94ad73b46fdf",
    ),
    (
        "SHA3-256",
        "h256",
        "3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532",
    ),
    (
        "SHA3-384",
        "h384",
        "ec01498288516fc926459f58e2c6ad8df9b473cb0fc08c2596da7cf0e49be4b298d88cea927ac7f5\
         39f1edf228376d25",
    ),
    (
        "SHA3-512",
        "h512",
        "b751850b1a57168a5693cd924b6b096e08f621827444f70d884f5d0240d2712e10e116e9192af3c9\
         1a7ec57647e3934057340b4cf408d5a56592f8274eec53f0",
    ),
];

/// One `SELECT` projecting all four SHA-3 digests of `?m`, spelled with `spelling`
/// applied to each function name.
fn sha3_select(spelling: impl Fn(&str) -> String) -> String {
    let mut q = String::from("PREFIX ex: <http://example.org/> SELECT");
    for (name, alias, _) in SHA3_ABC_VECTORS {
        use std::fmt::Write as _;
        write!(q, " ({}(?m) AS ?{alias})", spelling(name)).expect("write to a String");
    }
    q.push_str(" WHERE { ?s ex:message ?m }");
    q
}

/// The one solution row of a `--results-format json` run, as `alias -> value`.
fn sha3_row(out: &Output) -> serde_json::Map<String, serde_json::Value> {
    assert!(
        out.status.success(),
        "a SHA-3 SELECT must exit 0; stderr:\n{}",
        stderr(out)
    );
    let body = stdout(out);
    let doc: serde_json::Value = serde_json::from_str(&body)
        .unwrap_or_else(|e| panic!("--results-format json must emit JSON ({e}); got:\n{body}"));
    let bindings = doc["results"]["bindings"]
        .as_array()
        .unwrap_or_else(|| panic!("no results.bindings array in:\n{body}"));
    assert_eq!(bindings.len(), 1, "the fixture binds one row; got:\n{body}");
    bindings[0]
        .as_object()
        .unwrap_or_else(|| panic!("the binding row is not an object in:\n{body}"))
        .clone()
}

/// The four SEP-0008 built-ins, evaluated by the SHIPPED binary, reproduce their
/// published FIPS 202 `"abc"` digests in the SPARQL-results JSON it writes.
#[test]
fn sha3_builtins_reach_their_published_digests_through_the_cli() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "sha3.ttl", SHA3_DATA_TTL);

    let out = run(&[
        "query",
        "--data",
        &ttl,
        "--results-format",
        "json",
        &sha3_select(str::to_owned),
    ]);
    let row = sha3_row(&out);
    for (name, alias, want) in SHA3_ABC_VECTORS {
        let got = row[alias]["value"]
            .as_str()
            .unwrap_or_else(|| panic!("{alias} is not a literal value in {row:?}"));
        assert_eq!(
            got, want,
            "{name} through the CLI does not match its published FIPS 202 vector"
        );
    }
    // The four sizes must not collide: a dispatch table sending SHA3-384 to the
    // 256-bit arm would fail above, but this also pins the digest LENGTHS.
    let lengths: Vec<usize> = SHA3_ABC_VECTORS
        .iter()
        .map(|(_, alias, _)| row[*alias]["value"].as_str().expect("a literal").len())
        .collect();
    assert_eq!(lengths, vec![56, 64, 96, 128]);
}

/// SEP-0008 spells its four functions with an UNDERSCORE, so a query copied out of
/// the proposal must reach the same digests through the same shipped binary.
#[test]
fn sha3_underscored_sep_spelling_reaches_the_same_digests_through_the_cli() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "sha3.ttl", SHA3_DATA_TTL);

    let out = run(&[
        "query",
        "--data",
        &ttl,
        "--results-format",
        "json",
        &sha3_select(|name| name.replace('-', "_")),
    ]);
    let row = sha3_row(&out);
    for (name, alias, want) in SHA3_ABC_VECTORS {
        assert_eq!(
            row[alias]["value"].as_str(),
            Some(want),
            "the underscored spelling of {name} must reach the same digest"
        );
    }
}

/// The hyphen/spacing rule, at the surface a user types into a shell: `SHA3-256(?m)`
/// is one built-in call, a SPACED `SHA3 - 256` is not a function at all (a hard
/// parse failure, never a silently different answer), and an ordinary subtraction
/// beside a SHA-3 call still subtracts.
#[test]
fn the_cli_reads_the_sha3_hyphen_as_part_of_the_name() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "sha3.ttl", SHA3_DATA_TTL);

    let spaced = run(&[
        "query",
        "--data",
        &ttl,
        "--results-format",
        "json",
        "PREFIX ex: <http://example.org/> SELECT (SHA3 - 256 AS ?h) WHERE { ?s ex:message ?m }",
    ]);
    assert!(
        !spaced.status.success(),
        "`SHA3 - 256` must not resolve to the built-in; stdout:\n{}",
        stdout(&spaced)
    );

    // `STRLEN(SHA3-256(?m)) - 4` is subtraction: 64 hex chars minus 4.
    let arith = run(&[
        "query",
        "--data",
        &ttl,
        "--results-format",
        "json",
        "PREFIX ex: <http://example.org/> \
         SELECT (STRLEN(SHA3-256(?m)) - 4 AS ?n) WHERE { ?s ex:message ?m }",
    ]);
    assert!(
        arith.status.success(),
        "a subtraction beside a SHA-3 call must exit 0; stderr:\n{}",
        stderr(&arith)
    );
    let body = stdout(&arith);
    let doc: serde_json::Value = serde_json::from_str(&body).expect("JSON results");
    assert_eq!(
        doc["results"]["bindings"][0]["n"]["value"].as_str(),
        Some("60"),
        "STRLEN(SHA3-256(?m)) - 4 must be 64 - 4; got:\n{body}"
    );
}
