// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The library entry points every host surface (CLI, Python, WASM, C ABI) reaches for
//! the three shapes-graph tools beside validation: the cold-certify report
//! ([`purrdf_shapes::lint`]), standalone node-expression evaluation
//! ([`purrdf_shapes::free_expression`]) and the rendered inference graph and proof
//! ([`purrdf_shapes::Inference`]).
//!
//! Every shapes graph here carries the W3C SHACL 1.2 declaration of
//! `sh:SPARQLExprExpression` verbatim — the declaration that used to be refused as a
//! bodiless custom function — beside shapes that call `sh:sparqlExpr` with
//! `sh:prefixes`. Every refusal is paired with a valid neighbour that differs from it
//! in exactly the refused respect.

use std::sync::Arc;

use purrdf::RdfDataset;
use purrdf_shapes::data::ShaclData;
use purrdf_shapes::free_expression::{FreeExpression, evaluate, parse_term};
use purrdf_shapes::function_resolution::FunctionBinding;
use purrdf_shapes::lint::{LintReport, lint};
use purrdf_shapes::term::Term;
use purrdf_shapes::text_ingest::{TurtleDocument, parse_turtle_document, parse_turtle_to_dataset};
use purrdf_shapes::{RuleOptions, engine, infer};

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

/// The prefix block every fixture opens with.
const PREFIXES: &str = r"
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix shnex: <http://www.w3.org/ns/shacl-node-expr#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix ex: <http://example.org/ns#> .
";

/// Shapes beside the snippet: a `sh:sparqlExpr` expression node naming `ex:yes` through
/// `sh:prefixes` (`ex:Tag`), a labelled `shnex:var` expression (`_:suffix`), a rule
/// tagging every `ex:Item` through the same expression, and a counter rule that steps
/// `ex:n` to 5 — four term-generating rounds.
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

/// The data graph: one `ex:Item` whose counter starts at 1.
const DATA: &str = "@prefix ex: <http://example.org/ns#> .\nex:a a ex:Item ; ex:n 1 .\n";

const SPARQL_EXPR_EXPRESSION: &str = "http://www.w3.org/ns/shacl#SPARQLExprExpression";

fn document(body: &str) -> TurtleDocument {
    parse_turtle_document(&format!("{PREFIXES}{SPARQL_EXPR_DECLARATION}{body}"), None)
        .expect("the fixture parses")
}

fn linted(body: &str) -> LintReport {
    let doc = document(body);
    lint(&doc.dataset, &doc.prefixes, None, None).expect("shacl-shacl.ttl loads and validates")
}

fn data() -> Arc<RdfDataset> {
    parse_turtle_to_dataset(DATA, None).expect("data parses")
}

// ── lint ──────────────────────────────────────────────────────────────────────

#[test]
fn lint_certifies_the_sparql_expr_declaration_clean_and_binds_it_natively() {
    let report = linted(TOOLS);
    assert_eq!(report.load_error(), None);
    assert!(report.shacl_shacl().is_empty(), "{}", report.render());
    assert!(report.is_clean());
    let functions = report
        .function_resolution()
        .expect("an accepted graph binds");
    assert_eq!(
        functions.bindings_of(SPARQL_EXPR_EXPRESSION),
        std::iter::once(FunctionBinding::Native).collect()
    );
    let text = report.render();
    assert!(
        text.contains(&format!(
            "call native <{SPARQL_EXPR_EXPRESSION}> in sh:rule on <http://example.org/ns#Tagger>\n"
        )),
        "{text}"
    );
    assert!(text.starts_with("load accepted\nshacl-shacl 0\n"), "{text}");
    assert!(text.ends_with("findings 0\nclean true\n"), "{text}");
    assert_eq!(
        text,
        linted(TOOLS).render(),
        "the rendering is deterministic"
    );
}

/// The treatment row's `sh:minCount` is a string; the control row's is an integer. The
/// loader refuses the first and `shacl-shacl.ttl` flags it; the second is clean.
#[test]
fn lint_reports_a_malformed_graph_and_not_its_well_formed_neighbour() {
    let malformed =
        linted("ex:S a sh:NodeShape ; sh:property [ sh:path ex:p ; sh:minCount \"one\" ] .");
    assert!(malformed.load_error().is_some(), "{}", malformed.render());
    assert!(
        malformed.shacl_shacl().iter().any(|result| result.path
            == Some(Term::NamedNode(
                "http://www.w3.org/ns/shacl#minCount".into()
            ))
            && result.superseded.is_none()),
        "{}",
        malformed.render()
    );
    assert!(malformed.findings() >= 2, "{}", malformed.render());
    assert!(!malformed.is_clean());
    assert!(malformed.function_resolution().is_none());
    let text = malformed.render();
    assert!(text.starts_with("load refused\n  error "), "{text}");
    assert!(text.contains("functions unavailable\n"), "{text}");
    assert!(text.ends_with("clean false\n"), "{text}");

    let well_formed =
        linted("ex:S a sh:NodeShape ; sh:property [ sh:path ex:p ; sh:minCount 1 ] .");
    assert!(well_formed.is_clean(), "{}", well_formed.render());
}

/// `sh:closed sh:ByTypes` is well-formed SHACL 1.2 Core that the vendored
/// `shacl-shacl.ttl` still flags: reported, marked superseded, not a finding. Beside a
/// refusal the same result counts, because the supersession's premise — the loader
/// agrees with the specification — does not hold for a graph it refused.
#[test]
fn lint_supersedes_what_shacl_1_2_core_makes_well_formed() {
    let by_types = linted("ex:S a sh:NodeShape ; sh:closed sh:ByTypes .");
    let [result] = by_types.shacl_shacl() else {
        panic!("one shacl-shacl result: {}", by_types.render());
    };
    assert_eq!(
        result.superseded.map(|rule| rule.name),
        Some("closed-by-types")
    );
    assert!(by_types.is_clean(), "{}", by_types.render());
    assert!(by_types.render().contains(" superseded closed-by-types\n"));

    let refused = linted(
        "ex:S a sh:NodeShape ; sh:closed sh:ByTypes ;
           sh:property [ sh:path ex:p ; sh:minCount \"one\" ] .",
    );
    assert!(refused.load_error().is_some());
    assert!(
        refused
            .shacl_shacl()
            .iter()
            .all(|result| result.superseded.is_none()),
        "{}",
        refused.render()
    );
}

// ── node expressions ───────────────────────────────────────────────────────────

fn eval(root: &str, focus: &str, scope: &[(&str, &str)]) -> Result<Vec<String>, String> {
    let doc = document(TOOLS);
    let data = data();
    let root = parse_term(root)?;
    let focus = parse_term(focus)?;
    let scope: Vec<(String, Term)> = scope
        .iter()
        .map(|(name, term)| Ok(((*name).to_owned(), parse_term(term)?)))
        .collect::<Result<_, String>>()?;
    evaluate(&FreeExpression {
        shapes: &doc.dataset,
        prefixes: &doc.prefixes,
        root: &root,
        data: data.as_ref(),
        focus: &focus,
        scope: &scope,
    })
    .map(|terms| terms.iter().map(ToString::to_string).collect())
}

#[test]
fn a_sparql_expr_node_evaluates_natively_with_its_prefixes() {
    assert_eq!(
        eval("http://example.org/ns#Tag", "http://example.org/ns#a", &[]),
        Ok(vec!["<http://example.org/ns#yes>".to_owned()])
    );
}

#[test]
fn a_labelled_blank_node_reads_the_scope() {
    assert_eq!(
        eval(
            "_:suffix",
            "\"-3\"^^<http://www.w3.org/2001/XMLSchema#integer>",
            &[("suffix", "\"!\"")]
        ),
        Ok(vec!["\"!\"".to_owned()])
    );
    // The label is the shapes document's: a label it never wrote is refused rather than
    // evaluated as the empty expression.
    let error = eval("_:nosuch", "http://example.org/ns#a", &[]).expect_err("unknown label");
    assert!(error.contains("mentions no blank node _:nosuch"), "{error}");
}

#[test]
fn a_scope_binding_that_could_never_be_read_is_refused() {
    let error = eval(
        "_:suffix",
        "http://example.org/ns#a",
        &[("focusNode", "\"!\"")],
    )
    .expect_err("focusNode is resolved before the scope");
    assert!(error.contains("\"focusNode\" can never be read"), "{error}");
    let error = eval(
        "_:suffix",
        "http://example.org/ns#a",
        &[("suffix", "\"!\""), ("suffix", "\"?\"")],
    )
    .expect_err("a name bound twice");
    assert!(error.contains("bound twice"), "{error}");
    // The neighbour: two distinct names, neither `focusNode`.
    assert_eq!(
        eval(
            "_:suffix",
            "http://example.org/ns#a",
            &[("other", "\"?\""), ("suffix", "\"!\"")]
        ),
        Ok(vec!["\"!\"".to_owned()])
    );
}

#[test]
fn a_term_argument_is_an_n_triples_term_or_an_absolute_iri() {
    let error = parse_term("ns#a").expect_err("relative");
    assert!(error.contains("relative IRI reference"), "{error}");
    assert_eq!(
        parse_term("http://example.org/ns#a"),
        Ok(Term::NamedNode("http://example.org/ns#a".into()))
    );
    assert_eq!(
        parse_term("<http://example.org/ns#a>"),
        parse_term("http://example.org/ns#a")
    );
    assert!(parse_term("\"x\" . <http://e.org/s> <http://e.org/p> <http://e.org/o>").is_err());
    assert_eq!(parse_term("_:b"), Ok(Term::blank("b")));
}

// ── rules: the inference graph, its proof, and the round limit ─────────────────

fn run_rules(options: &RuleOptions) -> Result<purrdf_shapes::Inference, String> {
    let shapes =
        engine::parse_shapes(&format!("{PREFIXES}{SPARQL_EXPR_DECLARATION}{TOOLS}"), None)?;
    let projected = engine::project_dataset(data().as_ref())?;
    let holder = ShaclData::new(Arc::clone(&projected), projected, None);
    infer(&holder, &shapes, options)
}

#[test]
fn the_inference_graph_and_its_proof_render_deterministically() {
    let inference = run_rules(&RuleOptions::default()).expect("rules run");
    let integer = |n: u8| format!("\"{n}\"^^<http://www.w3.org/2001/XMLSchema#integer>");
    let mut expected = String::new();
    for n in 2..=5 {
        expected.push_str("<http://example.org/ns#a> <http://example.org/ns#n> ");
        expected.push_str(&integer(n));
        expected.push_str(" .\n");
    }
    expected.push_str(
        "<http://example.org/ns#a> <http://example.org/ns#tagged> <http://example.org/ns#yes> .\n",
    );
    assert_eq!(
        inference.inferred_ntriples(),
        expected,
        "the inference graph alone"
    );
    let proof = inference.proof_text();
    assert_eq!(proof.matches("derived ").count(), 5, "{proof}");
    assert_eq!(proof.matches("\n  rule ").count(), 5, "{proof}");
    assert!(
        proof.starts_with(&format!(
            "derived <http://example.org/ns#a> <http://example.org/ns#n> {} .\n  rule _:",
            integer(2)
        )),
        "{proof}"
    );
    assert_eq!(
        proof,
        run_rules(&RuleOptions::default())
            .expect("again")
            .proof_text()
    );
    assert_eq!(
        inference.inferred_dataset().expect("dataset").quad_count(),
        5,
        "the base graph is not in the inference graph"
    );
}

/// The counter needs exactly four term-generating rounds: a limit of 4 completes, 3
/// refuses naming the limit.
#[test]
fn the_term_generating_round_limit_is_exact() {
    let error = run_rules(&RuleOptions::default().with_max_term_generating_rounds(3))
        .expect_err("three rounds are too few");
    assert!(error.contains("past the limit of 3"), "{error}");
    let inference = run_rules(&RuleOptions::default().with_max_term_generating_rounds(4))
        .expect("four rounds suffice");
    assert_eq!(inference.inferred().len(), 5);
}
