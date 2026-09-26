// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! End-to-end coverage of the three shapes-graph tools beside `validate` — `rules`,
//! `node-expr` and `shapes lint` — driving the BUILT `purrdf` binary.
//!
//! Every shapes graph carries the W3C SHACL 1.2 declaration of `sh:SPARQLExprExpression`
//! verbatim — the declaration that used to be refused as a bodiless custom function —
//! beside shapes that call `sh:sparqlExpr` with `sh:prefixes`, so each command is
//! exercised over exactly the graph that was once unloadable.

use std::path::Path;
use std::process::{Command, Output};

/// The W3C SHACL 1.2 declaration of `sh:SPARQLExprExpression`, verbatim.
const SPARQL_EXPR_DECLARATION: &str = r#"
sh:SPARQLExprExpression a sh:NamedParameterExpressionFunction ;
  rdfs:label "SPARQL expr expression"@en ;
  rdfs:comment "The class of node expressions based on SPARQL expressions (sh:sparqlExpr)."@en ;
  rdfs:isDefinedBy sh: ;
  rdfs:subClassOf sh:NamedParameterExpression,
  sh:SPARQLExecutable ;
  sh:parameter sh:SPARQLExprExpression-prefixes,
  sh:SPARQLExprExpression-sparqlExpr .

sh:SPARQLExprExpression-prefixes a sh:Parameter ;
  rdfs:isDefinedBy sh: ;
  sh:description "The prefixes that shall be applied before parsing the SPARQL query that gets derived from the sh:sparqlExpr expression. The object should define those prefixes using sh:declare."@en ;
  sh:name "prefixes"@en ;
  sh:nodeKind sh:BlankNodeOrIRI ;
  sh:path sh:prefixes .

sh:SPARQLExprExpression-sparqlExpr a sh:Parameter ;
  rdfs:isDefinedBy sh: ;
  sh:datatype xsd:string ;
  sh:description "The SPARQL expression that is executed during evaluation of this node expression."@en ;
  sh:keyParameter true ;
  sh:name "SPARQL expr"@en ;
  sh:path sh:sparqlExpr .
"#;

const PREFIXES: &str = r"
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix shnex: <http://www.w3.org/ns/shacl-node-expr#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix ex: <http://example.org/ns#> .
";

/// A `sh:sparqlExpr` expression node naming `ex:yes` through `sh:prefixes` (`ex:Tag`), a
/// labelled `shnex:var` expression (`_:suffix`), a rule tagging every `ex:Item` through
/// the same expression, and a counter rule that steps `ex:n` to 5 — exactly four
/// term-generating rounds.
const TOOLS: &str = r#"
ex:Prefixes sh:declare [ sh:prefix "ex" ; sh:namespace "http://example.org/ns#"^^xsd:anyURI ] .
ex:Tag sh:sparqlExpr "ex:yes" ; sh:prefixes ex:Prefixes .
_:suffix shnex:var "suffix" .

ex:Tagger a sh:NodeShape ;
  sh:targetClass ex:Item ;
  sh:rule [ a sh:TripleRule ; sh:subject sh:this ; sh:predicate ex:tagged ;
            sh:object [ sh:sparqlExpr "ex:yes" ; sh:prefixes ex:Prefixes ] ] .

ex:Counter a sh:NodeShape ;
  sh:targetSubjectsOf ex:n ;
  sh:rule [ a sh:SPARQLRule ; sh:construct """PREFIX ex: <http://example.org/ns#>
CONSTRUCT { $this ex:n ?m } WHERE { $this ex:n ?k . FILTER(?k < 5) BIND(?k + 1 AS ?m) }""" ] .
"#;

const DATA: &str = "@prefix ex: <http://example.org/ns#> .\nex:a a ex:Item ; ex:n 1 .\n";

const INTEGER: &str = "<http://www.w3.org/2001/XMLSchema#integer>";

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_purrdf"))
        .args(args)
        .output()
        .expect("spawn the built purrdf binary")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn code(out: &Output) -> i32 {
    out.status.code().expect("the process exited normally")
}

fn write_file(dir: &Path, name: &str, contents: &str) -> String {
    let path = dir.join(name);
    std::fs::write(&path, contents).expect("write fixture file");
    path.to_str().expect("utf-8 temp path").to_owned()
}

/// The shapes graph: prefixes, the `sh:SPARQLExprExpression` declaration, then `body`.
fn shapes_file(dir: &Path, body: &str) -> String {
    write_file(
        dir,
        "shapes.ttl",
        &format!("{PREFIXES}{SPARQL_EXPR_DECLARATION}{body}"),
    )
}

/// The inference graph the fixture's rules produce, in canonical order.
fn expected_inference() -> String {
    let mut out = String::new();
    for n in 2..=5 {
        out.push_str("<http://example.org/ns#a> <http://example.org/ns#n> \"");
        out.push_str(&n.to_string());
        out.push_str("\"^^");
        out.push_str(INTEGER);
        out.push_str(" .\n");
    }
    out.push_str(
        "<http://example.org/ns#a> <http://example.org/ns#tagged> <http://example.org/ns#yes> .\n",
    );
    out
}

/// `rules` writes the inference graph — the base triples excluded — deterministically,
/// with the proof under `--explain` (bare to stderr, `=PATH` to a file); the round limit
/// refuses at 3 and completes at 4 and at the default; and a SPARQL 1.2 RL rule set runs
/// through the same command, its imports resolved from `--import`.
#[test]
fn cli_rules() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = shapes_file(dir.path(), TOOLS);
    let data = write_file(dir.path(), "data.ttl", DATA);

    let out = run(&["rules", "--shapes", &shapes, "--to", "ntriples", &data]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert_eq!(stdout(&out), expected_inference());
    assert!(
        stderr(&out).contains("rules inferred 5\n"),
        "{}",
        stderr(&out)
    );
    let again = run(&["rules", "--shapes", &shapes, "--to", "ntriples", &data]);
    assert_eq!(stdout(&again), stdout(&out), "byte-stable across runs");

    let explained = run(&[
        "rules",
        "--shapes",
        &shapes,
        "--explain",
        "--to",
        "ntriples",
        &data,
    ]);
    assert_eq!(code(&explained), 0, "{}", stderr(&explained));
    assert_eq!(
        stdout(&explained),
        expected_inference(),
        "the graph is unchanged"
    );
    let proof = stderr(&explained);
    assert!(
        proof.contains(
            "derived <http://example.org/ns#a> <http://example.org/ns#tagged> \
             <http://example.org/ns#yes> .\n  rule _:"
        ),
        "{proof}"
    );
    assert_eq!(proof.matches("derived ").count(), 5, "{proof}");
    let proof_path = dir.path().join("proof.txt");
    let to_file = run(&[
        "rules",
        "--shapes",
        &shapes,
        &format!("--explain={}", proof_path.display()),
        "--to",
        "ntriples",
        &data,
    ]);
    assert_eq!(code(&to_file), 0, "{}", stderr(&to_file));
    let written = std::fs::read_to_string(&proof_path).expect("proof written");
    assert!(
        written.starts_with("derived <http://example.org/ns#a> <http://example.org/ns#n> \"2\""),
        "{written}"
    );
    assert!(
        !stderr(&to_file).contains("derived "),
        "the proof went to the file only"
    );

    // The round limit: four term-generating rounds are needed.
    let refused = run(&[
        "rules",
        "--shapes",
        &shapes,
        "--max-term-generating-rounds",
        "3",
        "--to",
        "ntriples",
        &data,
    ]);
    assert_eq!(code(&refused), 1, "{}", stderr(&refused));
    assert!(
        stderr(&refused).contains("past the limit of 3"),
        "{}",
        stderr(&refused)
    );
    assert!(stdout(&refused).is_empty(), "a refused run writes no graph");
    let enough = run(&[
        "rules",
        "--shapes",
        &shapes,
        "--max-term-generating-rounds",
        "4",
        "--to",
        "ntriples",
        &data,
    ]);
    assert_eq!(code(&enough), 0, "{}", stderr(&enough));
    assert_eq!(stdout(&enough), expected_inference());

    // SPARQL 1.2 RL text, importing a second rule set through `--import`.
    let imported = write_file(
        dir.path(),
        "more.srl",
        "PREFIX ex: <http://example.org/ns#>\nRULE { ?x ex:counted true } WHERE { ?x ex:q ?y }\n",
    );
    let srl = write_file(
        dir.path(),
        "rules.srl",
        "PREFIX ex: <http://example.org/ns#>\nIMPORTS <http://example.org/more>\n\
         RULE { ?x ex:q ?y } WHERE { ?x ex:n ?y }\nDATA { ex:d ex:q 2 }\n",
    );
    let pair = format!("http://example.org/more={imported}");
    let srl_run = run(&[
        "rules",
        "--srl",
        &srl,
        "--import",
        &pair,
        "--explain",
        "--to",
        "ntriples",
        &data,
    ]);
    assert_eq!(code(&srl_run), 0, "{}", stderr(&srl_run));
    let graph = stdout(&srl_run);
    for line in [
        format!("<http://example.org/ns#a> <http://example.org/ns#q> \"1\"^^{INTEGER} .\n"),
        format!("<http://example.org/ns#d> <http://example.org/ns#q> \"2\"^^{INTEGER} .\n"),
        "<http://example.org/ns#a> <http://example.org/ns#counted> \"true\"^^<http://www.w3.org/2001/XMLSchema#boolean> .\n".to_owned(),
    ] {
        assert!(graph.contains(&line), "{line} missing from:\n{graph}");
    }
    let proof = stderr(&srl_run);
    assert!(proof.contains("  data-block\n"), "{proof}");
    assert!(
        proof.contains(&format!(
            "  premise <http://example.org/ns#a> <http://example.org/ns#n> \"1\"^^{INTEGER} .\n"
        )),
        "{proof}"
    );
    // The same rule set without the pair names the missing import; a pair nothing
    // imports is refused as unused.
    let missing = run(&["rules", "--srl", &srl, "--to", "ntriples", &data]);
    assert_eq!(code(&missing), 1, "{}", stderr(&missing));
    assert!(
        stderr(&missing).contains("--import http://example.org/more=FILE"),
        "{}",
        stderr(&missing)
    );
    let lone = write_file(
        dir.path(),
        "lone.srl",
        "PREFIX ex: <http://example.org/ns#>\nRULE { ?x ex:q ?y } WHERE { ?x ex:n ?y }\n",
    );
    let unused = run(&[
        "rules", "--srl", &lone, "--import", &pair, "--to", "ntriples", &data,
    ]);
    assert_eq!(code(&unused), 2, "{}", stderr(&unused));

    // The knob reaches the SPARQL 1.2 RL route too: a rule nesting a triple term every
    // round never stops generating terms, and a limit of 3 stops it by name.
    let nesting = write_file(
        dir.path(),
        "nesting.srl",
        "PREFIX ex: <http://example.org/ns#>\nRULE { ?x ex:n <<( ?x ex:n ?y )>> } WHERE { ?x ex:n ?y }\n",
    );
    let bounded = run(&[
        "rules",
        "--srl",
        &nesting,
        "--max-term-generating-rounds",
        "3",
        "--to",
        "ntriples",
        &data,
    ]);
    assert_eq!(code(&bounded), 1, "{}", stderr(&bounded));
    assert!(
        stderr(&bounded).contains("past the limit of 3"),
        "{}",
        stderr(&bounded)
    );

    // Exactly one rule source.
    let both = run(&[
        "rules", "--shapes", &shapes, "--srl", &lone, "--to", "ntriples", &data,
    ]);
    assert_eq!(code(&both), 2, "{}", stderr(&both));
    let neither = run(&["rules", "--to", "ntriples", &data]);
    assert_eq!(code(&neither), 2, "{}", stderr(&neither));
}

/// `node-expr` evaluates one expression node of the shapes graph: a `sh:sparqlExpr` node
/// natively, with its `sh:prefixes`; a labelled blank node reading `--scope`; a literal
/// focus. A label the document never wrote, and a scope binding that could never be read,
/// are refused beside the valid neighbour.
#[test]
fn cli_node_expr() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = shapes_file(dir.path(), TOOLS);
    let data = write_file(dir.path(), "data.ttl", DATA);

    let tag = run(&[
        "node-expr",
        "--shapes",
        &shapes,
        "--expr",
        "http://example.org/ns#Tag",
        "--focus",
        "http://example.org/ns#a",
        &data,
    ]);
    assert_eq!(code(&tag), 0, "{}", stderr(&tag));
    assert_eq!(stdout(&tag), "<http://example.org/ns#yes>\n");
    assert!(stderr(&tag).contains("node-expr outputs 1\n"));

    let scoped = run(&[
        "node-expr",
        "--shapes",
        &shapes,
        "--expr",
        "_:suffix",
        "--focus",
        &format!("\"-3\"^^{INTEGER}"),
        "--scope",
        "suffix=\"!\"@en",
        &data,
    ]);
    assert_eq!(code(&scoped), 0, "{}", stderr(&scoped));
    assert_eq!(stdout(&scoped), "\"!\"@en\n");

    let unknown = run(&[
        "node-expr",
        "--shapes",
        &shapes,
        "--expr",
        "_:nosuch",
        "--focus",
        "http://example.org/ns#a",
        &data,
    ]);
    assert_eq!(code(&unknown), 1, "{}", stderr(&unknown));
    assert!(stderr(&unknown).contains("mentions no blank node _:nosuch"));

    let unreadable = run(&[
        "node-expr",
        "--shapes",
        &shapes,
        "--expr",
        "_:suffix",
        "--focus",
        "http://example.org/ns#a",
        "--scope",
        "focusNode=\"!\"",
        &data,
    ]);
    assert_eq!(code(&unreadable), 1, "{}", stderr(&unreadable));
    assert!(stderr(&unreadable).contains("can never be read"));

    let relative = run(&[
        "node-expr",
        "--shapes",
        &shapes,
        "--expr",
        "Tag",
        "--focus",
        "http://example.org/ns#a",
        &data,
    ]);
    assert_eq!(
        code(&relative),
        2,
        "an argv term is decided before any document"
    );
    let no_equals = run(&[
        "node-expr",
        "--shapes",
        &shapes,
        "--expr",
        "_:suffix",
        "--focus",
        "http://example.org/ns#a",
        "--scope",
        "suffix",
        &data,
    ]);
    assert_eq!(code(&no_equals), 2, "{}", stderr(&no_equals));
}

/// `node-expr` names an ANONYMOUS expression two ways: by a walk from a named node (the
/// rule's `sh:object` expression, reached from `ex:Tagger`), and inline as Turtle (whose
/// `sh:prefixes ex:Prefixes` resolves in the shapes graph). Each refusal sits beside a
/// valid neighbour: a walk step reaching two values beside one reaching one, an inline
/// document with two roots beside one with one, and the selector flags' usage errors
/// beside the spelling that runs.
#[test]
fn cli_node_expr_selectors() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = shapes_file(dir.path(), TOOLS);
    let data = write_file(dir.path(), "data.ttl", DATA);
    let node_expr = |selector: &[&str]| {
        let mut args = vec!["node-expr", "--shapes", &shapes];
        args.extend_from_slice(selector);
        args.extend_from_slice(&["--focus", "http://example.org/ns#a", &data]);
        run(&args)
    };
    const SH: &str = "http://www.w3.org/ns/shacl#";

    let walked = node_expr(&[
        "--expr-at",
        "http://example.org/ns#Tagger",
        "--expr-via",
        &format!("{SH}rule"),
        "--expr-via",
        &format!("{SH}object"),
    ]);
    assert_eq!(code(&walked), 0, "{}", stderr(&walked));
    assert_eq!(stdout(&walked), "<http://example.org/ns#yes>\n");

    let one = node_expr(&[
        "--expr-at",
        &format!("{SH}SPARQLExprExpression"),
        "--expr-via",
        "http://www.w3.org/2000/01/rdf-schema#isDefinedBy",
    ]);
    assert_eq!(code(&one), 0, "{}", stderr(&one));
    assert_eq!(stdout(&one), format!("<{SH}>\n"));
    let two = node_expr(&[
        "--expr-at",
        &format!("{SH}SPARQLExprExpression"),
        "--expr-via",
        &format!("{SH}parameter"),
    ]);
    assert_eq!(code(&two), 1, "{}", stderr(&two));
    assert!(
        stderr(&two).contains("reaches 2 values"),
        "{}",
        stderr(&two)
    );

    let inline = node_expr(&[
        "--expr-turtle",
        "[ sh:sparqlExpr \"ex:yes\" ; sh:prefixes ex:Prefixes ] .",
    ]);
    assert_eq!(code(&inline), 0, "{}", stderr(&inline));
    assert_eq!(stdout(&inline), "<http://example.org/ns#yes>\n");
    let file = write_file(
        dir.path(),
        "expr.ttl",
        "[ sh:sparqlExpr \"ex:yes\" ; sh:prefixes ex:Prefixes ] .\n",
    );
    let from_file = node_expr(&["--expr-turtle-file", &file]);
    assert_eq!(code(&from_file), 0, "{}", stderr(&from_file));
    assert_eq!(stdout(&from_file), "<http://example.org/ns#yes>\n");
    let roots = node_expr(&[
        "--expr-turtle",
        "[ shnex:var \"a\" ] . [ shnex:var \"b\" ] .",
    ]);
    assert_eq!(code(&roots), 1, "{}", stderr(&roots));
    assert!(
        stderr(&roots).contains("has 2 root blank nodes"),
        "{}",
        stderr(&roots)
    );

    // Usage: two selectors, none, a walk with no predicate, a predicate with no walk,
    // a relative walk start, and the inline file on stdin.
    for selector in [
        &[
            "--expr",
            "http://example.org/ns#Tag",
            "--expr-turtle",
            "[ shnex:var \"a\" ] .",
        ][..],
        &[][..],
        &["--expr-at", "http://example.org/ns#Tagger"][..],
        &["--expr-via", &format!("{SH}rule")][..],
        &["--expr-at", "Tagger", "--expr-via", &format!("{SH}rule")][..],
        &["--expr-turtle-file", "-"][..],
    ] {
        let out = node_expr(selector);
        assert_eq!(code(&out), 2, "{selector:?}: {}", stderr(&out));
    }
}

/// `shapes lint` certifies the declaration-bearing graph clean (exit 0), naming `sh:sparqlExpr`'s
/// function as bound natively; a malformed neighbour is reported with findings (exit 1,
/// the report still written); a graph `shacl-shacl.ttl` flags but SHACL 1.2 Core makes
/// well-formed stays clean.
#[test]
fn cli_shapes_lint() {
    let dir = tempfile::tempdir().expect("tempdir");
    let clean = shapes_file(dir.path(), TOOLS);
    let out = run(&["shapes", "lint", &clean]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let report = stdout(&out);
    assert!(
        report.starts_with("load accepted\nshacl-shacl 0\n"),
        "{report}"
    );
    assert!(
        report.contains(
            "call native <http://www.w3.org/ns/shacl#SPARQLExprExpression> in sh:rule on <http://example.org/ns#Tagger>\n"
        ),
        "{report}"
    );
    assert!(
        report.ends_with("validators 0\nfindings 0\nclean true\n"),
        "{report}"
    );
    assert!(stderr(&out).contains("shapes lint clean true\n"));
    assert_eq!(
        stdout(&run(&["shapes", "lint", &clean])),
        report,
        "deterministic"
    );

    let malformed = write_file(
        dir.path(),
        "malformed.ttl",
        &format!(
            "{PREFIXES}{SPARQL_EXPR_DECLARATION}ex:S a sh:NodeShape ; sh:property [ sh:path ex:p ; sh:minCount \"one\" ] .\n"
        ),
    );
    let bad = run(&["shapes", "lint", &malformed]);
    assert_eq!(code(&bad), 1, "{}", stderr(&bad));
    let report = stdout(&bad);
    assert!(report.starts_with("load refused\n  error "), "{report}");
    assert!(
        report.contains("path <http://www.w3.org/ns/shacl#minCount>"),
        "{report}"
    );
    assert!(report.contains("functions unavailable\n"), "{report}");
    assert!(report.contains("validators unavailable\n"), "{report}");
    assert!(report.ends_with("clean false\n"), "{report}");
    assert!(stderr(&bad).contains("shapes lint clean false\n"));

    let by_types = write_file(
        dir.path(),
        "by-types.ttl",
        &format!(
            "{PREFIXES}{SPARQL_EXPR_DECLARATION}ex:S a sh:NodeShape ; sh:closed sh:ByTypes .\n"
        ),
    );
    let superseded = run(&["shapes", "lint", &by_types]);
    assert_eq!(code(&superseded), 0, "{}", stderr(&superseded));
    assert!(stdout(&superseded).contains(" superseded closed-by-types\n"));

    let ledger = run(&["--loss-ledger", "shapes", "lint", &clean]);
    assert_eq!(code(&ledger), 2, "a text report has no loss ledger");
}

// ── One shapes graph, one owl:imports verdict, on every lane ────────────────────

/// The importing shapes graph: an ontology header and its import, and no shape of its own.
const IMPORTER: &str = "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
    <http://example.org/shapes> a owl:Ontology ;\n\
      owl:imports <http://example.org/lib> .\n";

/// The imported document: a shape needing `ex:name`, a rule tagging every `ex:Person`
/// `ex:checked ex:yes`, and a node expression `ex:Who` reading the scope variable `who`.
const IMPORTED: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
    @prefix shnex: <http://www.w3.org/ns/shacl-node-expr#> .\n\
    @prefix ex: <http://example.org/> .\n\
    ex:NameShape a sh:NodeShape ;\n\
      sh:targetClass ex:Person ;\n\
      sh:property [ sh:path ex:name ; sh:minCount 1 ] ;\n\
      sh:rule [ a sh:TripleRule ; sh:subject sh:this ; sh:predicate ex:checked ; \
                sh:object ex:yes ] .\n\
    ex:Who shnex:var \"who\" .\n";

/// One `ex:Person` with no `ex:name`.
const PERSON: &str = "<http://example.org/alice> \
    <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .\n";

/// Every shapes-graph lane of the command line refuses the importing shapes graph with the
/// same `unresolved-import` refusal when no `--import` names the imported document, and —
/// with the pair — applies the imported document: the shape reports, the rule infers, the
/// expression reads its scope, the lint certifies, and the product carries the shape.
#[test]
fn every_shapes_lane_gives_the_same_owl_imports_verdict() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "importer.ttl", IMPORTER);
    let lib = write_file(dir.path(), "lib.ttl", IMPORTED);
    let data = write_file(dir.path(), "data.nt", PERSON);
    let pair = format!("http://example.org/lib={lib}");
    let product = dir.path().join("closure.purrshp");
    let product = product.to_str().expect("utf-8 path").to_owned();

    let validate = |import: Option<&str>| {
        let mut args = vec!["validate", "--shapes", &shapes];
        args.extend(import.map(|pair| ["--import", pair]).into_iter().flatten());
        args.push(&data);
        run(&args)
    };
    let rules = |import: Option<&str>| {
        let mut args = vec!["rules", "--shapes", &shapes, "--to", "ntriples"];
        args.extend(import.map(|pair| ["--import", pair]).into_iter().flatten());
        args.push(&data);
        run(&args)
    };
    let node_expr = |import: Option<&str>| {
        let mut args = vec![
            "node-expr",
            "--shapes",
            &shapes,
            "--expr",
            "http://example.org/Who",
            "--focus",
            "http://example.org/alice",
            "--scope",
            "who=http://example.org/bob",
        ];
        args.extend(import.map(|pair| ["--import", pair]).into_iter().flatten());
        args.push(&data);
        run(&args)
    };
    let lint = |import: Option<&str>| {
        let mut args = vec!["shapes", "lint"];
        args.extend(import.map(|pair| ["--import", pair]).into_iter().flatten());
        args.push(&shapes);
        run(&args)
    };
    let pack = |import: Option<&str>| {
        let mut args = vec!["shacl", "pack", "--shapes", &shapes, "--out", &product];
        args.extend(import.map(|pair| ["--import", pair]).into_iter().flatten());
        run(&args)
    };

    for (lane, out) in [
        ("validate", validate(None)),
        ("rules", rules(None)),
        ("node-expr", node_expr(None)),
        ("shapes lint", lint(None)),
        ("shacl pack", pack(None)),
    ] {
        let err = stderr(&out);
        assert_eq!(code(&out), 1, "{lane}: a runtime refusal: {err}");
        assert!(
            err.contains("unresolved-import")
                && err.contains("<http://example.org/lib>")
                && err.contains("--import http://example.org/lib=FILE"),
            "{lane}: the one refusal, naming the import and its pair: {err}"
        );
        assert!(stdout(&out).is_empty(), "{lane}: nothing is written");
    }

    let validated = validate(Some(&pair));
    assert_eq!(code(&validated), 0, "{}", stderr(&validated));
    assert!(
        stderr(&validated).contains("shacl conforms false\n")
            && stdout(&validated).contains("MinCountConstraintComponent"),
        "the imported shape reports: {}",
        stdout(&validated)
    );

    let inferred = rules(Some(&pair));
    assert_eq!(code(&inferred), 0, "{}", stderr(&inferred));
    assert_eq!(
        stdout(&inferred),
        "<http://example.org/alice> <http://example.org/checked> <http://example.org/yes> .\n",
        "the imported rule infers"
    );

    let evaluated = node_expr(Some(&pair));
    assert_eq!(code(&evaluated), 0, "{}", stderr(&evaluated));
    assert_eq!(
        stdout(&evaluated),
        "<http://example.org/bob>\n",
        "the imported expression reads its scope"
    );

    let linted = lint(Some(&pair));
    assert_eq!(code(&linted), 0, "{}", stderr(&linted));
    assert!(
        stdout(&linted).ends_with("clean true\n"),
        "{}",
        stdout(&linted)
    );

    let packed = pack(Some(&pair));
    assert_eq!(code(&packed), 0, "{}", stderr(&packed));
    let restored = run(&["validate", "--shapes-product", &product, &data]);
    assert_eq!(code(&restored), 0, "{}", stderr(&restored));
    assert!(
        stdout(&restored).contains("MinCountConstraintComponent"),
        "the product carries the imported shape: {}",
        stdout(&restored)
    );
}

/// SHACL's prefix idiom (the W3C `sparql/node/prefixes-001` shape on `example.org`): the
/// `owl:imports` target is a node the shapes graph describes with `sh:declare`, so the
/// command line validates it with no `--import`, and the prefixes declared along
/// `sh:prefixes/owl:imports*/sh:declare` — `imp:` on the target, `test:` on the importing
/// node, neither a Turtle `@prefix` — reach the query, which reports `ex:Invalid`. The
/// neighbour describes the target only with `rdfs:label` and is refused by name.
#[test]
fn the_shacl_prefix_idiom_validates_without_an_import_and_a_labelled_target_is_refused() {
    let idiom = |description: &str| {
        format!(
            "@prefix ex: <http://example.org/ns#> .\n\
             @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
             @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
             @prefix sh: <http://www.w3.org/ns/shacl#> .\n\
             @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\
             <http://example.org/ns#> {description} .\n\
             ex:TestPrefixes owl:imports <http://example.org/ns#> ;\n\
               sh:declare [ sh:prefix \"test\" ; \
                            sh:namespace \"http://example.org/test#\"^^xsd:anyURI ] .\n\
             ex:TestSPARQL sh:prefixes ex:TestPrefixes ;\n\
               sh:select \"SELECT $this ?value WHERE {{ $this imp:property ?value . \
                            FILTER (?value = test:Value) }}\" .\n\
             ex:TestShape a sh:NodeShape ; sh:sparql ex:TestSPARQL ;\n\
               sh:targetNode ex:Invalid , ex:Valid .\n"
        )
    };
    let dir = tempfile::tempdir().expect("tempdir");
    let declared = write_file(
        dir.path(),
        "declared.ttl",
        &idiom(
            "sh:declare [ sh:prefix \"imp\" ; \
             sh:namespace \"http://example.org/ns#\"^^xsd:anyURI ]",
        ),
    );
    let labelled = write_file(
        dir.path(),
        "labelled.ttl",
        &idiom("rdfs:label \"a namespace\""),
    );
    let data = write_file(
        dir.path(),
        "data.nt",
        "<http://example.org/ns#Invalid> <http://example.org/ns#property> \
         <http://example.org/test#Value> .\n\
         <http://example.org/ns#Valid> <http://example.org/ns#property> \
         <http://example.org/test#Other> .\n",
    );

    let out = run(&["validate", "--shapes", &declared, &data]);
    let err = stderr(&out);
    assert_eq!(code(&out), 0, "the idiom is in hand: {err}");
    assert!(
        err.contains("shacl conforms false\n") && err.contains("shacl results 1\n"),
        "one result: {err}"
    );
    let report = stdout(&out);
    assert!(
        report.contains("<http://www.w3.org/ns/shacl#focusNode> <http://example.org/ns#Invalid>")
            && report
                .contains("<http://www.w3.org/ns/shacl#value> <http://example.org/test#Value>"),
        "the query ran with both declared prefixes: {report}"
    );

    let out = run(&["validate", "--shapes", &labelled, &data]);
    let err = stderr(&out);
    assert_eq!(code(&out), 1, "a runtime refusal: {err}");
    assert!(
        err.contains("unresolved-import") && err.contains("<http://example.org/ns#>"),
        "the refusal names the labelled target: {err}"
    );
    assert!(stdout(&out).is_empty(), "no report is written");
}

/// `shapes lint` names each validator a shapes graph declares for a built-in component
/// — here an ASK validator on `sh:MinCountConstraintComponent` that would pass
/// nothing — as an alternative the native implementation supersedes, and counts it as
/// no finding: exit 0, clean, the same findings as the graph without it. `validate`
/// over the same graph reports the NATIVE verdict, so the alternative did not run.
#[test]
fn cli_shapes_lint_reports_superseded_builtin_validators() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(
        dir.path(),
        "alternatives.ttl",
        &format!(
            r#"{PREFIXES}
sh:MinCountConstraintComponent a sh:ConstraintComponent ;
  sh:validator ex:neverValid .
ex:neverValid a sh:SPARQLAskValidator ; sh:ask "ASK {{ FILTER (false) }}" .
ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:property [ sh:path ex:n ; sh:minCount 1 ] .
"#
        ),
    );
    let out = run(&["shapes", "lint", &shapes]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let report = stdout(&out);
    assert!(
        report.contains(
            "validators 1\nalternative <http://www.w3.org/ns/shacl#MinCountConstraintComponent> \
             <http://www.w3.org/ns/shacl#validator> <http://example.org/ns#neverValid> \
             sparql-ask superseded-by-native\n"
        ),
        "{report}"
    );
    assert!(report.ends_with("findings 0\nclean true\n"), "{report}");

    let data = write_file(dir.path(), "data.ttl", DATA);
    let validated = run(&["validate", "--shapes", &shapes, &data]);
    assert_eq!(code(&validated), 0, "{}", stderr(&validated));
}
