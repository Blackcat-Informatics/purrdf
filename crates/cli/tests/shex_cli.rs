// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! End-to-end `shex` coverage that drives the BUILT `purrdf` binary
//! (`env!("CARGO_BIN_EXE_purrdf")`) — never the library — so every assertion pins the shipped
//! executable's ShEx 2.1 surface as an installed consumer meets it.
//!
//! ## What is pinned here
//!
//! * the artifact is the ShapeMap specification's **result shape map** on stdout, and a
//!   NONCONFORMANT node still exits **0** with the verdict on stderr;
//! * both schema syntaxes (ShExC and ShExJ) reach the same schema, by extension and by
//!   `--schema-from`;
//! * every native data syntax and the pack container reach the identical verdict;
//! * query shape-map selectors resolve deterministically, and RDF 1.2 **triple terms** are
//!   ordinary nodes to the validator (the statement layer is not, and that is stated rather
//!   than silently empty — see the `shex` module docs);
//! * every construct whose semantics this boundary cannot supply — an unresolved `IMPORT`, an
//!   `EXTERNAL` shape, a semantic action — is refused BY NAME instead of becoming a verdict;
//! * `--loss-ledger`/`--jsonld-options` are refused rather than silently ignored.

use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Output, Stdio};

/// A `Command` for the built `purrdf` binary.
fn purrdf() -> Command {
    Command::new(env!("CARGO_BIN_EXE_purrdf"))
}

/// Run `purrdf` with `args`, returning the captured [`Output`].
fn run(args: &[&str]) -> Output {
    purrdf()
        .args(args)
        .output()
        .expect("spawn the built purrdf binary")
}

/// Run `purrdf` with `args`, writing `stdin_bytes` to its standard input.
fn pipe(args: &[&str], stdin_bytes: &str) -> Output {
    let mut child = purrdf()
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn purrdf");
    child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(stdin_bytes.as_bytes())
        .expect("write stdin");
    child.wait_with_output().expect("wait for purrdf")
}

/// stdout of an [`Output`] as a `String`.
fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// stderr of an [`Output`] as a `String`.
fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// The exit code of an [`Output`].
fn code(out: &Output) -> i32 {
    out.status.code().expect("the process exited normally")
}

/// Write `contents` to `dir/name`, returning the path as a `String`.
fn write_file(dir: &Path, name: &str, contents: &str) -> String {
    let p = dir.join(name);
    std::fs::write(&p, contents).expect("write fixture file");
    p.to_str().expect("temp path is valid UTF-8").to_owned()
}

/// A schema whose `ex:age` must be an `xsd:integer` when present.
const SCHEMA: &str = concat!(
    "PREFIX ex: <http://example.org/>\n",
    "PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>\n",
    "ex:UserShape { ex:name LITERAL ? ; ex:age xsd:integer ? }\n",
);

/// Two people: `alice`'s age is a string (nonconformant), `bob`'s is an integer.
const DATA: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "ex:alice a ex:Person ; ex:age \"nope\" .\n",
    "ex:bob a ex:Person ; ex:age 42 .\n",
);

/// The `bob@UserShape` fixed association.
const BOB: &str = "<http://example.org/bob>@<http://example.org/UserShape>";

/// The `alice@UserShape` fixed association.
const ALICE: &str = "<http://example.org/alice>@<http://example.org/UserShape>";

/// A conformant association: the result shape map says so, and the run exits 0.
#[test]
fn a_conformant_node_reports_and_exits_zero() {
    let dir = tempfile::tempdir().expect("tempdir");
    let schema = write_file(dir.path(), "schema.shex", SCHEMA);
    let data = write_file(dir.path(), "data.ttl", DATA);

    let out = run(&["shex", "--schema", &schema, "--data", &data, BOB]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));

    let body = stdout(&out);
    assert!(
        body.contains("\"status\":\"conformant\""),
        "the result shape map states the verdict:\n{body}"
    );
    assert!(
        body.contains("\"node\":\"<http://example.org/bob>\""),
        "the node is written in the shape-map term syntax:\n{body}"
    );
    assert!(
        !body.contains("\"reason\""),
        "a conformant entry carries no reason:\n{body}"
    );
    assert!(
        body.ends_with("]\n"),
        "the artifact ends with a newline:\n{body}"
    );

    let verdict = stderr(&out);
    assert!(verdict.contains("shex conformant true\n"), "{verdict}");
    assert!(verdict.contains("shex entries 1\n"), "{verdict}");
    assert!(verdict.contains("shex nonconformant 0\n"), "{verdict}");
}

/// A NONCONFORMANT association is a decided verdict: the entry carries its reason, and the run
/// still exits 0 — the same position `validate`, `consistency` and a `false` ASK hold.
#[test]
fn a_nonconformant_node_reports_a_reason_and_still_exits_zero() {
    let dir = tempfile::tempdir().expect("tempdir");
    let schema = write_file(dir.path(), "schema.shex", SCHEMA);
    let data = write_file(dir.path(), "data.ttl", DATA);

    let out = run(&["shex", "--schema", &schema, "--data", &data, ALICE]);
    assert_eq!(code(&out), 0, "a decided verdict exits 0: {}", stderr(&out));

    let body = stdout(&out);
    assert!(body.contains("\"status\":\"nonconformant\""), "{body}");
    assert!(
        body.contains("\"reason\"") && body.contains("XMLSchema#integer"),
        "the entry says WHY it failed:\n{body}"
    );

    let verdict = stderr(&out);
    assert!(verdict.contains("shex conformant false\n"), "{verdict}");
    assert!(verdict.contains("shex nonconformant 1\n"), "{verdict}");
}

/// A query shape-map selector expands against the data, deterministically, and both nodes it
/// selects are reported in one map.
#[test]
fn a_query_selector_expands_against_the_data() {
    let dir = tempfile::tempdir().expect("tempdir");
    let schema = write_file(dir.path(), "schema.shex", SCHEMA);
    let data = write_file(dir.path(), "data.ttl", DATA);

    let map = "{FOCUS a <http://example.org/Person>}@<http://example.org/UserShape>";
    let out = run(&["shex", "--schema", &schema, "--data", &data, map]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));

    let body = stdout(&out);
    assert!(body.contains("<http://example.org/alice>"), "{body}");
    assert!(body.contains("<http://example.org/bob>"), "{body}");
    assert!(
        stderr(&out).contains("shex entries 2\n"),
        "{}",
        stderr(&out)
    );
    assert!(
        stderr(&out).contains("shex nonconformant 1\n"),
        "only alice fails: {}",
        stderr(&out)
    );

    // Deterministic: the same command, twice, is the same bytes.
    let again = run(&["shex", "--schema", &schema, "--data", &data, map]);
    assert_eq!(
        stdout(&again),
        body,
        "the result shape map is deterministic"
    );
}

/// EVERY native data syntax, plus the pack container, reaches the identical verdict.
#[test]
fn every_native_data_format_reaches_the_same_verdict() {
    let dir = tempfile::tempdir().expect("tempdir");
    let schema = write_file(dir.path(), "schema.shex", SCHEMA);
    let seed = write_file(dir.path(), "seed.ttl", DATA);

    for (token, extension) in [
        ("turtle", "ttl"),
        ("trig", "trig"),
        ("ntriples", "nt"),
        ("nquads", "nq"),
        ("rdfxml", "rdf"),
        ("trix", "trix"),
        ("hextuples", "hext"),
        ("jsonld", "jsonld"),
        ("yamlld", "yamlld"),
        ("pack", "purrpck"),
    ] {
        let target = dir
            .path()
            .join(format!("data.{extension}"))
            .to_str()
            .expect("temp path is valid UTF-8")
            .to_owned();
        let converted = run(&["convert", "--from", "turtle", "--to", token, &seed, &target]);
        assert!(
            converted.status.success(),
            "seeding the `{token}` source failed: {}",
            stderr(&converted)
        );

        let out = run(&["shex", "--schema", &schema, "--data", &target, ALICE]);
        assert_eq!(code(&out), 0, "`{token}`: {}", stderr(&out));
        assert!(
            stderr(&out).contains("shex nonconformant 1\n"),
            "`{token}` must reach the same verdict: {}",
            stderr(&out)
        );
    }
}

/// A ShExJ schema is the same schema: resolved by extension, and by `--schema-from` over a
/// path whose extension says nothing.
#[test]
fn shexj_schemas_reach_the_same_verdict() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);
    // The ShExJ spelling of `ex:UserShape { ex:age xsd:integer ? }`.
    let shexj = concat!(
        r#"{"type":"Schema","shapes":[{"type":"Shape","id":"http://example.org/UserShape","#,
        r#""expression":{"type":"TripleConstraint","predicate":"http://example.org/age","#,
        r#""valueExpr":{"type":"NodeConstraint","#,
        r#""datatype":"http://www.w3.org/2001/XMLSchema#integer"},"min":0,"max":1}}]}"#,
    );
    let by_extension = write_file(dir.path(), "schema.shexj", shexj);
    let opaque = write_file(dir.path(), "schema.bin", shexj);

    let inferred = run(&["shex", "--schema", &by_extension, "--data", &data, ALICE]);
    assert_eq!(code(&inferred), 0, "{}", stderr(&inferred));
    assert!(
        stderr(&inferred).contains("shex nonconformant 1\n"),
        "{}",
        stderr(&inferred)
    );

    let overridden = run(&[
        "shex",
        "--schema",
        &opaque,
        "--schema-from",
        "shexj",
        "--data",
        &data,
        ALICE,
    ]);
    assert_eq!(code(&overridden), 0, "{}", stderr(&overridden));
    assert!(
        stderr(&overridden).contains("shex nonconformant 1\n"),
        "{}",
        stderr(&overridden)
    );

    // And an unresolvable extension is a usage error naming the flag, never a guessed syntax.
    let unguessable = run(&["shex", "--schema", &opaque, "--data", &data, ALICE]);
    assert_eq!(code(&unguessable), 2, "{}", stderr(&unguessable));
    assert!(
        stderr(&unguessable).contains("--schema-from"),
        "the refusal names the flag that fixes it: {}",
        stderr(&unguessable)
    );
}

/// RDF 1.2 triple terms are ordinary nodes: they are matched as an arc's object, and a shape
/// map may name one as the focus node.
#[test]
fn rdf12_triple_terms_are_ordinary_nodes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let schema = write_file(
        dir.path(),
        "claim.shex",
        concat!(
            "PREFIX ex: <http://example.org/>\n",
            "ex:Claim { ex:states NONLITERAL }\n",
        ),
    );
    let data = write_file(
        dir.path(),
        "claim.ttl",
        concat!(
            "@prefix ex: <http://example.org/> .\n",
            "ex:claim ex:states <<( ex:alice ex:knows ex:bob )>> .\n",
        ),
    );

    // A triple term as the OBJECT of a constrained arc.
    let as_object = run(&[
        "shex",
        "--schema",
        &schema,
        "--data",
        &data,
        "<http://example.org/claim>@<http://example.org/Claim>",
    ]);
    assert_eq!(code(&as_object), 0, "{}", stderr(&as_object));
    assert!(
        stdout(&as_object).contains("\"status\":\"conformant\""),
        "a triple term satisfies NONLITERAL:\n{}",
        stdout(&as_object)
    );

    // The triple term itself selected as a FOCUS node, and written back in the `<< … >>` term
    // syntax the shape-map grammar uses.
    let as_focus = run(&[
        "shex",
        "--schema",
        &schema,
        "--data",
        &data,
        "{_ <http://example.org/states> FOCUS}@<http://example.org/Claim>",
    ]);
    assert_eq!(code(&as_focus), 0, "{}", stderr(&as_focus));
    let body = stdout(&as_focus);
    assert!(
        body.contains(
            "<< <http://example.org/alice> <http://example.org/knows> \
                       <http://example.org/bob> >>"
        ),
        "the triple term is the reported focus node:\n{body}"
    );
    assert!(
        stderr(&as_focus).contains("shex entries 1\n"),
        "{}",
        stderr(&as_focus)
    );
}

/// The RDF 1.2 STATEMENT layer IS reachable through the CLI's shape maps: a selector over
/// an annotation predicate selects the reifier that carries it.
///
/// ShEx 2.1's own data model predates RDF 1.2 and describes only arcs, so the annotation
/// would have been invisible. PurRDF extends it rather than inheriting the gap — the
/// reifier's neighbourhood is the union of its ordinary arcs, its `rdf:reifies` arc, and its
/// annotations, matching what `purrdf validate` (SHACL) and SPARQL already see over the same
/// document. Pinning it here keeps the three surfaces answering alike.
#[test]
fn the_statement_layer_is_an_arc_to_shex() {
    let dir = tempfile::tempdir().expect("tempdir");
    let schema = write_file(
        dir.path(),
        "anno.shex",
        concat!(
            "PREFIX ex: <http://example.org/>\n",
            "ex:Stmt { ex:certainty . }\n",
        ),
    );
    let data = write_file(
        dir.path(),
        "star.ttl",
        concat!(
            "@prefix ex: <http://example.org/> .\n",
            "ex:alice ex:knows ex:bob {| ex:certainty 0.9 |} .\n",
        ),
    );

    let out = run(&[
        "shex",
        "--schema",
        &schema,
        "--data",
        &data,
        "{FOCUS <http://example.org/certainty> _}@<http://example.org/Stmt>",
    ]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        stdout(&out).contains("\"status\":\"conformant\""),
        "the annotation predicate selects its reifier, which conforms:\n{}",
        stdout(&out)
    );
    assert!(
        stdout(&out).contains("\"shape\":\"<http://example.org/Stmt>\""),
        "{}",
        stdout(&out)
    );
    assert!(
        stderr(&out).contains("shex entries 1\n"),
        "exactly one reifier carries ex:certainty:\n{}",
        stderr(&out)
    );

    // The plain arcs of the SAME document are matched as usual, so the graph was read.
    let plain = run(&[
        "shex",
        "--schema",
        &write_file(
            dir.path(),
            "knows.shex",
            "PREFIX ex: <http://example.org/>\nex:Knower { ex:knows IRI }\n",
        ),
        "--data",
        &data,
        "<http://example.org/alice>@<http://example.org/Knower>",
    ]);
    assert_eq!(code(&plain), 0, "{}", stderr(&plain));
    assert!(
        stdout(&plain).contains("\"status\":\"conformant\""),
        "the star document's plain arcs validate normally:\n{}",
        stdout(&plain)
    );
}

/// The data graph may arrive on stdin, and `-` requires `--from`.
#[test]
fn stdin_data_requires_an_explicit_from_and_then_validates() {
    let dir = tempfile::tempdir().expect("tempdir");
    let schema = write_file(dir.path(), "schema.shex", SCHEMA);

    let bare = pipe(&["shex", "--schema", &schema, "--data", "-", ALICE], DATA);
    assert_eq!(code(&bare), 2, "a usage error: {}", stderr(&bare));
    assert!(stderr(&bare).contains("--from"), "{}", stderr(&bare));

    let explicit = pipe(
        &[
            "shex", "--schema", &schema, "--data", "-", "--from", "turtle", ALICE,
        ],
        DATA,
    );
    assert_eq!(code(&explicit), 0, "{}", stderr(&explicit));
    assert!(stderr(&explicit).contains("shex nonconformant 1\n"));
}

/// The SCHEMA may arrive on stdin instead, under `--schema-from`; both on stdin is refused.
#[test]
fn stdin_schemas_are_read_under_schema_from_and_two_stdins_are_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);

    let bare = pipe(&["shex", "--schema", "-", "--data", &data, ALICE], SCHEMA);
    assert_eq!(code(&bare), 2, "{}", stderr(&bare));
    assert!(stderr(&bare).contains("--schema-from"), "{}", stderr(&bare));

    let explicit = pipe(
        &[
            "shex",
            "--schema",
            "-",
            "--schema-from",
            "shexc",
            "--data",
            &data,
            ALICE,
        ],
        SCHEMA,
    );
    assert_eq!(code(&explicit), 0, "{}", stderr(&explicit));
    assert!(stderr(&explicit).contains("shex nonconformant 1\n"));

    let both = pipe(
        &[
            "shex",
            "--schema",
            "-",
            "--schema-from",
            "shexc",
            "--data",
            "-",
            "--from",
            "turtle",
            ALICE,
        ],
        SCHEMA,
    );
    assert_eq!(code(&both), 2, "a usage error");
    assert!(
        stderr(&both).contains("standard input"),
        "the refusal names the one-stdin invariant: {}",
        stderr(&both)
    );
}

/// A malformed ShExC schema, a malformed ShExJ schema and a malformed shape map are all
/// runtime failures (exit 1) that name the document, and none of them writes a verdict.
#[test]
fn malformed_inputs_are_runtime_failures() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);
    let good = write_file(dir.path(), "schema.shex", SCHEMA);

    let bad_shexc = write_file(dir.path(), "bad.shex", "ex:Broken { { { ");
    let out = run(&["shex", "--schema", &bad_shexc, "--data", &data, ALICE]);
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    assert!(stderr(&out).contains("--schema"), "{}", stderr(&out));
    assert!(stdout(&out).is_empty(), "a failed run writes no verdict");

    let bad_shexj = write_file(dir.path(), "bad.shexj", "{ not json");
    let out = run(&["shex", "--schema", &bad_shexj, "--data", &data, ALICE]);
    assert_eq!(code(&out), 1, "{}", stderr(&out));

    let out = run(&[
        "shex",
        "--schema",
        &good,
        "--data",
        &data,
        "not a shape map",
    ]);
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    assert!(
        stderr(&out).contains("MAP"),
        "the diagnostic names the argument that failed: {}",
        stderr(&out)
    );
}

/// A schema that violates the ShEx 2.1 §5.7 structural requirements is refused, naming the
/// section, rather than validated against a dangling reference.
#[test]
fn a_structurally_invalid_schema_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);
    let schema = write_file(
        dir.path(),
        "dangling.shex",
        concat!(
            "PREFIX ex: <http://example.org/>\n",
            "ex:UserShape { ex:age @ex:Nowhere }\n",
        ),
    );

    let out = run(&["shex", "--schema", &schema, "--data", &data, ALICE]);
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    assert!(
        stderr(&out).contains("5.7"),
        "the refusal cites the structural requirements: {}",
        stderr(&out)
    );
    assert!(stdout(&out).is_empty());
}

/// An `EXTERNAL` shape is refused by name rather than reported as `nonconformant`.
///
/// The library documents that, without a resolver, an `EXTERNAL` shape "fails every node" —
/// which as a printed verdict is a definite answer derived from semantics nobody supplied. The
/// resolver is a host callback and cannot cross a command line, so the schema is refused.
#[test]
fn an_external_shape_is_refused_rather_than_reported_nonconformant() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);

    for (name, source) in [
        (
            "top-level.shex",
            "PREFIX ex: <http://example.org/>\nex:UserShape EXTERNAL\n",
        ),
        (
            "nested.shex",
            "PREFIX ex: <http://example.org/>\nex:UserShape { ex:age EXTERNAL }\n",
        ),
    ] {
        let schema = write_file(dir.path(), name, source);
        let out = run(&["shex", "--schema", &schema, "--data", &data, ALICE]);
        assert_eq!(code(&out), 1, "`{name}`: {}", stderr(&out));
        assert!(
            stderr(&out).contains("EXTERNAL"),
            "`{name}`: the refusal names the construct: {}",
            stderr(&out)
        );
        assert!(
            stdout(&out).is_empty(),
            "`{name}`: no verdict is invented:\n{}",
            stdout(&out)
        );
    }
}

/// A semantic action is refused by name rather than treated as the inert success the empty
/// extension registry would make it — which would report a conformance the check never
/// granted.
#[test]
fn a_semantic_action_is_refused_rather_than_treated_as_an_inert_success() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);
    let schema = write_file(
        dir.path(),
        "semact.shex",
        concat!(
            "PREFIX ex: <http://example.org/>\n",
            "ex:UserShape { ex:age . } %<http://example.org/ext>{ nonsense %}\n",
        ),
    );

    let out = run(&["shex", "--schema", &schema, "--data", &data, ALICE]);
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    assert!(
        stderr(&out).contains("http://example.org/ext"),
        "the refusal names the extension it cannot dispatch: {}",
        stderr(&out)
    );
    assert!(
        stderr(&out).contains("INERT SUCCESS"),
        "and says why reporting it would be wrong: {}",
        stderr(&out)
    );
    assert!(stdout(&out).is_empty());
}

/// `--import IRI=FILE` folds an imported schema in; an import with no pair is refused by name;
/// a pair the closure never reaches is refused as unused.
#[test]
fn imports_are_caller_supplied_and_both_halves_are_enforced() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);
    let imported = write_file(
        dir.path(),
        "imported.shex",
        concat!(
            "PREFIX ex: <http://example.org/>\n",
            "PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>\n",
            "ex:AgeShape xsd:integer\n",
        ),
    );
    let importer = write_file(
        dir.path(),
        "importer.shex",
        concat!(
            "PREFIX ex: <http://example.org/>\n",
            "IMPORT <http://example.org/ages>\n",
            "ex:UserShape { ex:age @ex:AgeShape }\n",
        ),
    );

    // Unresolved: the import is refused by name, never folded in as an empty schema (which
    // would leave `@ex:AgeShape` dangling and change the verdict).
    let unresolved = run(&["shex", "--schema", &importer, "--data", &data, ALICE]);
    assert_eq!(code(&unresolved), 1, "{}", stderr(&unresolved));
    assert!(
        stderr(&unresolved).contains("http://example.org/ages"),
        "the refusal names the unresolved IRI: {}",
        stderr(&unresolved)
    );
    assert!(
        stderr(&unresolved).contains("--import"),
        "and the flag that resolves it: {}",
        stderr(&unresolved)
    );

    // Resolved: the imported shape is in scope and the verdict is decided against it.
    let resolved = run(&[
        "shex",
        "--schema",
        &importer,
        "--import",
        &format!("http://example.org/ages={imported}"),
        "--data",
        &data,
        ALICE,
    ]);
    assert_eq!(code(&resolved), 0, "{}", stderr(&resolved));
    assert!(
        stderr(&resolved).contains("shex nonconformant 1\n"),
        "alice's string age fails the imported integer shape: {}",
        stderr(&resolved)
    );

    // An unused pair is refused rather than read and ignored.
    let plain = write_file(dir.path(), "plain.shex", SCHEMA);
    let unused = run(&[
        "shex",
        "--schema",
        &plain,
        "--import",
        &format!("http://example.org/ages={imported}"),
        "--data",
        &data,
        ALICE,
    ]);
    assert_eq!(code(&unused), 2, "a usage error: {}", stderr(&unused));
    assert!(
        stderr(&unused).contains("never reaches"),
        "the refusal says the pair would go unused: {}",
        stderr(&unused)
    );
}

/// A malformed `--import` pair, a duplicate IRI, and a `-` import path are each usage errors
/// naming the argument — never a silently skipped import.
#[test]
fn malformed_import_pairs_are_usage_errors() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);
    let schema = write_file(dir.path(), "schema.shex", SCHEMA);
    let imported = write_file(
        dir.path(),
        "imported.shex",
        "PREFIX ex: <http://example.org/>\n",
    );

    for (spec, needle) in [
        ("http://example.org/ages", "has no `=`"),
        ("=/tmp/x.shex", "both halves"),
        ("http://example.org/ages=", "both halves"),
        ("http://example.org/ages=-", "has none"),
    ] {
        let out = run(&[
            "shex", "--schema", &schema, "--import", spec, "--data", &data, ALICE,
        ]);
        assert_eq!(code(&out), 2, "`{spec}`: {}", stderr(&out));
        assert!(
            stderr(&out).contains(needle),
            "`{spec}`: the refusal explains itself: {}",
            stderr(&out)
        );
    }

    let pair = format!("http://example.org/ages={imported}");
    let duplicate = run(&[
        "shex", "--schema", &schema, "--import", &pair, "--import", &pair, "--data", &data, ALICE,
    ]);
    assert_eq!(code(&duplicate), 2, "{}", stderr(&duplicate));
    assert!(
        stderr(&duplicate).contains("named twice"),
        "{}",
        stderr(&duplicate)
    );
}

/// `--base` resolves relative IRIs in the data graph, the ShExC schema and the shape map at
/// once, and the unbased run is the negative control that keeps the positive honest.
#[test]
fn base_resolves_relative_iris_everywhere_it_applies() {
    let dir = tempfile::tempdir().expect("tempdir");
    let schema = write_file(
        dir.path(),
        "relative.shex",
        "<UserShape> { <age> <http://www.w3.org/2001/XMLSchema#integer> }\n",
    );
    // Turtle rather than N-Triples: N-Triples has no relative-IRI syntax at all, so `--base`
    // would have nothing to resolve there.
    let relative_data = "<alice> <age> \"nope\" .\n";
    let relative_map = "<alice>@<UserShape>";

    let based = pipe(
        &[
            "shex",
            "--schema",
            &schema,
            "--data",
            "-",
            "--from",
            "turtle",
            "--base",
            "http://example.org/",
            relative_map,
        ],
        relative_data,
    );
    assert_eq!(code(&based), 0, "{}", stderr(&based));
    assert!(
        stdout(&based).contains("\"node\":\"<http://example.org/alice>\""),
        "every relative IRI resolved against the same base:\n{}",
        stdout(&based)
    );
    assert!(
        stderr(&based).contains("shex nonconformant 1\n"),
        "and the resolved shape actually decided the node: {}",
        stderr(&based)
    );

    // The negative control. Without `--base` nothing is resolved: the three documents still
    // agree with each other (all three keep the relative term `<alice>`, so the map still
    // matches the data), which is exactly why the assertion above has to be about the term
    // IDENTITY rather than about the verdict. What changed is what the node IS, and the
    // unbased run says so in its own output.
    let unbased = pipe(
        &[
            "shex",
            "--schema",
            &schema,
            "--data",
            "-",
            "--from",
            "turtle",
            relative_map,
        ],
        relative_data,
    );
    assert_eq!(code(&unbased), 0, "{}", stderr(&unbased));
    assert!(
        stdout(&unbased).contains("\"node\":\"<alice>\""),
        "unresolved, the node is the relative term the document wrote:\n{}",
        stdout(&unbased)
    );
    assert!(
        !stdout(&unbased).contains("http://example.org/alice"),
        "and it is NOT the absolute IRI --base produced:\n{}",
        stdout(&unbased)
    );
}

/// `--base` with a pack data source is refused by name (a pack stores fully-resolved terms).
#[test]
fn base_with_a_pack_data_source_is_refused_by_name() {
    let dir = tempfile::tempdir().expect("tempdir");
    let schema = write_file(dir.path(), "schema.shex", SCHEMA);
    let data = write_file(dir.path(), "data.ttl", DATA);
    let pack = dir
        .path()
        .join("data.purrpck")
        .to_str()
        .expect("temp path")
        .to_owned();
    let packed = run(&["convert", "--from", "turtle", "--to", "pack", &data, &pack]);
    assert!(packed.status.success(), "{}", stderr(&packed));

    let out = run(&[
        "shex",
        "--schema",
        &schema,
        "--data",
        &pack,
        "--base",
        "http://example.org/",
        ALICE,
    ]);
    assert_eq!(code(&out), 2, "a usage error");
    assert!(stderr(&out).contains("--base"), "{}", stderr(&out));
}

/// The two global document flags are refused rather than silently ignored: `shex` transcodes
/// nothing and runs no RDF serializer.
#[test]
fn the_global_document_flags_are_refused_by_name() {
    let dir = tempfile::tempdir().expect("tempdir");
    let schema = write_file(dir.path(), "schema.shex", SCHEMA);
    let data = write_file(dir.path(), "data.ttl", DATA);
    let options = write_file(
        dir.path(),
        "jsonld-options.json",
        r#"{"version":1,"mode":"context","prefixes":{"ex":"http://example.org/"}}"#,
    );

    let ledger = run(&[
        "--loss-ledger",
        "shex",
        "--schema",
        &schema,
        "--data",
        &data,
        ALICE,
    ]);
    assert_eq!(code(&ledger), 2, "{}", stderr(&ledger));
    assert!(
        stderr(&ledger).contains("--loss-ledger"),
        "{}",
        stderr(&ledger)
    );

    let jsonld = run(&[
        "--jsonld-options",
        &options,
        "shex",
        "--schema",
        &schema,
        "--data",
        &data,
        ALICE,
    ]);
    assert_eq!(code(&jsonld), 2, "{}", stderr(&jsonld));
    assert!(
        stderr(&jsonld).contains("--jsonld-options"),
        "{}",
        stderr(&jsonld)
    );
}

/// The result shape map can be written to a FILE, leaving stdout untouched.
#[test]
fn the_result_map_can_be_written_to_a_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let schema = write_file(dir.path(), "schema.shex", SCHEMA);
    let data = write_file(dir.path(), "data.ttl", DATA);
    let out_path = dir
        .path()
        .join("result.json")
        .to_str()
        .expect("temp path")
        .to_owned();

    let out = run(&[
        "shex", "--schema", &schema, "--data", &data, ALICE, &out_path,
    ]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(stdout(&out).is_empty(), "the map went to the file");
    assert!(stderr(&out).contains("shex conformant false\n"));

    let written = std::fs::read_to_string(&out_path).expect("result written");
    let parsed: serde_json::Value = serde_json::from_str(&written).expect("valid JSON");
    assert_eq!(parsed[0]["status"], "nonconformant", "{written}");
}

/// `--schema`, `--data` and the `MAP` positional are all required; none of them is invented.
#[test]
fn the_required_inputs_are_required() {
    let dir = tempfile::tempdir().expect("tempdir");
    let schema = write_file(dir.path(), "schema.shex", SCHEMA);
    let data = write_file(dir.path(), "data.ttl", DATA);

    for (args, needle) in [
        (vec!["shex", "--data", &data, ALICE], "--schema"),
        (vec!["shex", "--schema", &schema, ALICE], "--data"),
        (vec!["shex", "--schema", &schema, "--data", &data], "MAP"),
    ] {
        let out = run(&args);
        assert_eq!(code(&out), 2, "{args:?}: {}", stderr(&out));
        assert!(
            stderr(&out).contains(needle),
            "{args:?}: the refusal names `{needle}`: {}",
            stderr(&out)
        );
    }
}
