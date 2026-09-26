// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! **One shapes graph, one verdict, on every Rust entry point.**
//!
//! A shapes graph that `owl:imports <http://example.org/lib>` is driven through every
//! shapes-graph entry point the Rust surface offers — the engine's own constructors and
//! text entry points, and the string boundary the Python, WebAssembly and C hosts call
//! (validation, the change path, SHACL-AF entailment, rules, node expressions, lint and
//! the prepared product). Each one:
//!
//! * REFUSES the graph with the identical typed [`ShapesImportError::Unresolved`] when no
//!   table supplies the import;
//! * ACCEPTS it when the table supplies the document, and the imported document is
//!   observably APPLIED — the imported shape produces a result, the imported rule infers a
//!   triple, the imported node expression evaluates to its binding, the imported shape is
//!   linted — none of which the importing document alone can produce.
//!
//! The in-graph neighbours hold the other half of the rule: the same shapes graph with the
//! ontology declared in place (`<lib> a owl:Ontology`) or named by a version IRI resolves
//! with no table, and a table entry nothing imports is refused as unreached.

use std::sync::Arc;

use purrdf_shapes::engine::{self, PreparedShapes};
use purrdf_shapes::free_expression::{FreeExpression, evaluate, parse_term};
use purrdf_shapes::lint::lint;
use purrdf_shapes::text_ingest::{parse_ntriples_to_dataset, parse_turtle_document};
use purrdf_shapes::{ShapesError, ShapesImportError, ShapesImports};
use purrdf_validate::{
    ExprSelector, NodeExprRequest, RulesRequest, SarifOptions, ShapesProductRefusal,
    ValidationOptions, apply_rules_to_ntriples, entail_to_ntriples_string, eval_node_expr_to_terms,
    lint_shapes_ttl, pack_shapes_product, validate_changes_to_sarif_string,
    validate_to_sarif_string, validate_with_shapes_product,
};

/// The imported ontology's IRI.
const LIB: &str = "http://example.org/lib";

/// The importing shapes graph: an ontology header and its import, and no shape of its own.
const IMPORTER: &str = "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
    <http://example.org/shapes> a owl:Ontology ;\n\
      owl:imports <http://example.org/lib> .\n";

/// The imported document: a shape (every `ex:Person` needs an `ex:name`) carrying a rule
/// (every `ex:Person` is `ex:checked ex:yes`), and a node expression `ex:Who` reading the
/// scope variable `who`.
const LIB_DOCUMENT: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
    @prefix shnex: <http://www.w3.org/ns/shacl-node-expr#> .\n\
    @prefix ex: <http://example.org/> .\n\
    ex:NameShape a sh:NodeShape ;\n\
      sh:targetClass ex:Person ;\n\
      sh:property [ sh:path ex:name ; sh:minCount 1 ] ;\n\
      sh:rule [ a sh:TripleRule ; sh:subject sh:this ; sh:predicate ex:checked ; \
                sh:object ex:yes ] .\n\
    ex:Who shnex:var \"who\" .\n";

/// One `ex:Person` with no `ex:name`: the imported shape's one violation.
const DATA: &str = "<http://example.org/alice> \
    <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .\n";

/// The triple the imported rule infers.
const INFERRED: &str =
    "<http://example.org/alice> <http://example.org/checked> <http://example.org/yes> .";

/// The constraint component only the imported shape can report.
const MIN_COUNT: &str = "http://www.w3.org/ns/shacl#MinCountConstraintComponent";

/// The table that supplies the imported document.
const TABLE: &[(&str, &str)] = &[(LIB, LIB_DOCUMENT)];

/// The typed refusal every entry point must return without the table.
fn unresolved() -> ShapesImportError {
    ShapesImportError::Unresolved {
        iris: vec![LIB.to_owned()],
    }
}

/// `error` is the unresolved-import refusal, as a VARIANT rather than a message.
#[track_caller]
fn assert_unresolved(error: &ShapesError, entry: &str) {
    assert_eq!(
        error.as_imports(),
        Some(&unresolved()),
        "{entry} must refuse with the typed unresolved-import error, got: {error}"
    );
}

fn table() -> ShapesImports {
    ShapesImports::from_turtle(TABLE).expect("the imported document parses")
}

// ── The engine ────────────────────────────────────────────────────────────────

#[test]
fn engine_constructors_and_text_entry_points_agree() {
    // Refused, identically, everywhere a shapes graph becomes `Shapes`.
    assert_unresolved(
        &engine::parse_shapes(IMPORTER, None).expect_err("refused"),
        "parse_shapes",
    );
    assert_unresolved(
        &engine::validate_graphs(DATA, IMPORTER, None).expect_err("refused"),
        "validate_graphs",
    );
    assert_unresolved(
        &engine::validate_graphs_with_options(
            DATA,
            IMPORTER,
            None,
            &ValidationOptions::default(),
            &ShapesImports::new(),
        )
        .expect_err("refused"),
        "validate_graphs_with_options",
    );
    let data = parse_ntriples_to_dataset(DATA).expect("data");
    assert_unresolved(
        &engine::validate_dataset_graphs(data.as_ref(), IMPORTER, None, &ShapesImports::new())
            .expect_err("refused"),
        "validate_dataset_graphs",
    );
    assert_unresolved(
        &engine::entail_graphs(DATA, IMPORTER, None, &ShapesImports::new()).expect_err("refused"),
        "entail_graphs",
    );
    let document = parse_turtle_document(IMPORTER, None).expect("turtle");
    assert_unresolved(
        &purrdf_shapes::shapes::from_dataset(&document.dataset).expect_err("refused"),
        "from_dataset",
    );

    // Supplied: the imported shape is a shape of the result, and `Shapes` built this way
    // validates through `validate_dataset_with_shapes_graph` with the imported result.
    let shapes =
        engine::parse_shapes_with_config(IMPORTER, None, None, &table()).expect("resolved");
    let report = engine::validate_dataset_with_shapes_graph(data.as_ref(), &shapes, None)
        .expect("validates");
    assert!(!report.conforms);
    assert_eq!(report.results.len(), 1);
    assert_eq!(
        report.results[0].source_constraint_component.as_str(),
        MIN_COUNT
    );
    let report = engine::validate_graphs_with_options(
        DATA,
        IMPORTER,
        None,
        &ValidationOptions::default(),
        &table(),
    )
    .expect("validates");
    assert_eq!(report.results.len(), 1, "the imported shape is applied");
    let entailed = engine::entail_graphs(DATA, IMPORTER, None, &table()).expect("entails");
    assert!(
        entailed.quads().count() > data.quads().count(),
        "the imported rule infers a triple"
    );
}

#[test]
fn free_expression_and_lint_agree() {
    let document = parse_turtle_document(IMPORTER, None).expect("turtle");
    let data = parse_ntriples_to_dataset(DATA).expect("data");
    let root = parse_term("http://example.org/Who").expect("term");
    let focus = parse_term("http://example.org/alice").expect("term");
    let scope = vec![(
        "who".to_owned(),
        parse_term("http://example.org/bob").expect("term"),
    )];
    let request = |imports: &ShapesImports| {
        evaluate(&FreeExpression {
            shapes: &document.dataset,
            prefixes: &document.prefixes,
            root: &root,
            data: data.as_ref(),
            focus: &focus,
            scope: &scope,
            imports,
        })
    };
    assert_unresolved(
        &request(&ShapesImports::new()).expect_err("refused"),
        "free_expression::evaluate",
    );
    let outputs = request(&table()).expect("evaluates");
    assert_eq!(
        outputs.iter().map(ToString::to_string).collect::<Vec<_>>(),
        ["<http://example.org/bob>"],
        "the imported expression reads the scope; unimported, `ex:Who` would be a constant"
    );

    assert_unresolved(
        &lint(
            &document.dataset,
            &document.prefixes,
            None,
            None,
            &ShapesImports::new(),
        )
        .expect_err("refused, never a report about the importing document alone"),
        "lint",
    );
    let report = lint(&document.dataset, &document.prefixes, None, None, &table())
        .expect("the closure is linted");
    assert!(report.is_clean(), "{}", report.render());

    // The lint certifies the IMPORTED document too: the same closure with a malformed
    // imported shape is not clean, although the importing document did not change.
    let malformed = LIB_DOCUMENT.replace("sh:minCount 1", "sh:minCount \"one\"");
    let malformed_table = ShapesImports::from_turtle(&[(LIB, malformed.as_str())])
        .expect("the malformed document still parses as Turtle");
    let report = lint(
        &document.dataset,
        &document.prefixes,
        None,
        None,
        &malformed_table,
    )
    .expect("the closure is linted");
    assert!(
        !report.is_clean() && report.load_error().is_some(),
        "the imported document's malformed shape is a finding: {}",
        report.render()
    );
}

// ── The string boundary every host calls ──────────────────────────────────────

#[test]
fn every_boundary_entry_point_refuses_without_the_table_and_applies_it_with() {
    let options = SarifOptions::default();

    let refused = validate_to_sarif_string(IMPORTER, None, DATA, &options, &[])
        .expect_err("validate refuses");
    assert_unresolved(&refused, "validate_to_sarif_string");
    let sarif = validate_to_sarif_string(IMPORTER, None, DATA, &options, TABLE)
        .expect("validate applies the import");
    assert!(sarif.contains("MinCountConstraintComponent"), "{sarif}");

    let refused =
        validate_changes_to_sarif_string(IMPORTER, None, "", Some(DATA), None, &options, &[])
            .expect_err("the change path refuses");
    assert_unresolved(&refused, "validate_changes_to_sarif_string");
    let (sarif, _) =
        validate_changes_to_sarif_string(IMPORTER, None, "", Some(DATA), None, &options, TABLE)
            .expect("the change path applies the import");
    assert!(sarif.contains("MinCountConstraintComponent"), "{sarif}");

    let refused =
        entail_to_ntriples_string(IMPORTER, None, DATA, &[]).expect_err("entailment refuses");
    assert_unresolved(&refused, "entail_to_ntriples_string");
    let entailed = entail_to_ntriples_string(IMPORTER, None, DATA, TABLE).expect("entails");
    assert!(entailed.contains(INFERRED), "{entailed}");

    let rules = |imports: &[(&str, &str)]| {
        apply_rules_to_ntriples(&RulesRequest {
            data_nt: DATA,
            shapes_ttl: Some(IMPORTER),
            shapes_imports: imports,
            ..RulesRequest::default()
        })
    };
    assert_unresolved(
        &rules(&[]).expect_err("rules refuse"),
        "apply_rules_to_ntriples",
    );
    let inferred = rules(TABLE)
        .expect("the imported rule runs")
        .inferred_ntriples;
    assert_eq!(
        inferred.trim(),
        INFERRED,
        "the imported rule's one inference"
    );

    let node_expr = |imports: &[(&str, &str)]| {
        eval_node_expr_to_terms(&NodeExprRequest {
            shapes_ttl: IMPORTER,
            shapes_base: None,
            data_nt: DATA,
            expr: ExprSelector::Node("http://example.org/Who"),
            focus: "http://example.org/alice",
            scope: &[("who", "http://example.org/bob")],
            imports,
        })
    };
    assert_unresolved(
        &node_expr(&[]).expect_err("node-expr refuses"),
        "eval_node_expr_to_terms",
    );
    assert_eq!(
        node_expr(TABLE).expect("evaluates"),
        ["<http://example.org/bob>"]
    );

    assert_unresolved(
        &lint_shapes_ttl(IMPORTER, None, &[]).expect_err("lint refuses"),
        "lint_shapes_ttl",
    );
    let report = lint_shapes_ttl(IMPORTER, None, TABLE).expect("lints the closure");
    assert!(report.load_error().is_none(), "{}", report.render());

    let refusal = pack_shapes_product(IMPORTER, None, &[]).expect_err("pack refuses");
    assert_eq!(refusal.import_error(), Some(&unresolved()));
    assert!(
        matches!(
            refusal,
            ShapesProductRefusal::Shapes(ShapesError::Imports(_))
        ),
        "the product boundary carries the same typed error: {refusal:?}"
    );
    let product = pack_shapes_product(IMPORTER, None, TABLE).expect("packs the closure");
    let sarif = validate_with_shapes_product(&product, DATA, &options).expect("restores");
    assert!(
        sarif.contains("MinCountConstraintComponent"),
        "the product carries the imported shape: {sarif}"
    );
}

// ── The in-graph neighbours ───────────────────────────────────────────────────

/// The imported document's content merged into the importing one by hand, with `extra`
/// declaring (or not) the ontology the import names.
fn merged_by_hand(extra: &str) -> String {
    format!("{IMPORTER}{LIB_DOCUMENT}{extra}")
}

#[test]
fn an_ontology_declared_in_place_resolves_and_its_neighbour_does_not() {
    let options = SarifOptions::default();
    for declaration in [
        // The ontology header.
        "<http://example.org/lib> a <http://www.w3.org/2002/07/owl#Ontology> .\n",
        // An ontology whose version IRI is the imported IRI.
        "<http://example.org/lib-series> \
         <http://www.w3.org/2002/07/owl#versionIRI> <http://example.org/lib> .\n",
    ] {
        let sarif =
            validate_to_sarif_string(&merged_by_hand(declaration), None, DATA, &options, &[])
                .unwrap_or_else(|error| panic!("{declaration}: resolved in place: {error}"));
        assert!(
            sarif.contains("MinCountConstraintComponent"),
            "{declaration}: the in-place shape is applied: {sarif}"
        );
    }
    // The neighbour: the same content, with no declaration of the imported ontology.
    let refused = validate_to_sarif_string(&merged_by_hand(""), None, DATA, &options, &[])
        .expect_err("an undeclared import is refused even when the content is present");
    assert_unresolved(&refused, "validate_to_sarif_string");
}

#[test]
fn the_shapes_documents_own_iri_resolves_a_self_import_and_another_does_not() {
    let options = SarifOptions::default();
    let importer = format!("{IMPORTER}{LIB_DOCUMENT}");
    let sarif = validate_to_sarif_string(&importer, Some(LIB), DATA, &options, &[])
        .expect("the shapes document IS the imported document");
    assert!(sarif.contains("MinCountConstraintComponent"), "{sarif}");
    let refused = validate_to_sarif_string(
        &importer,
        Some("http://example.org/elsewhere"),
        DATA,
        &options,
        &[],
    )
    .expect_err("a different base names a different document");
    assert_unresolved(&refused, "validate_to_sarif_string");
}

#[test]
fn a_table_entry_nothing_imports_is_refused_and_a_reached_one_is_not() {
    let options = SarifOptions::default();
    let plain = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
        <http://example.org/S> a sh:NodeShape .\n";
    let refused = validate_to_sarif_string(plain, None, DATA, &options, TABLE)
        .expect_err("an unreached entry would be read and never used");
    assert_eq!(
        refused.as_imports(),
        Some(&ShapesImportError::Unreached {
            iris: vec![LIB.to_owned()]
        })
    );
    validate_to_sarif_string(IMPORTER, None, DATA, &options, TABLE)
        .expect("the same entry, reached, is used");
}

#[test]
fn a_prepared_preparation_of_the_closure_matches_a_fresh_parse() {
    let shapes = Arc::new(
        engine::parse_shapes_with_config(IMPORTER, None, None, &table()).expect("resolved"),
    );
    let prepared = PreparedShapes::new(Arc::clone(&shapes));
    let data = parse_ntriples_to_dataset(DATA).expect("data");
    let direct = engine::validate_dataset_with_shapes_graph(data.as_ref(), &shapes, None)
        .expect("validates");
    let via_prepared =
        engine::validate_dataset_with_shapes_graph(data.as_ref(), prepared.shapes(), None)
            .expect("validates");
    assert_eq!(direct.to_ntriples(), via_prepared.to_ntriples());
}

// ── SHACL's prefix idiom ──────────────────────────────────────────────────────

/// The W3C SHACL `sparql/node/prefixes-001` idiom on `example.org`: the query's prefixes are
/// collected along `sh:prefixes/owl:imports*/sh:declare`, and the `owl:imports` target is a
/// node this shapes graph describes with `sh:declare`. `imp:` is declared ONLY on that
/// target and `test:` only on the importing node — neither is a Turtle `@prefix` — so a
/// result proves the import was followed to the described node and both declarations
/// reached the query. `description` is what the shapes graph says about the target.
fn prefix_idiom(description: &str) -> String {
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
}

/// The import target declares `imp:` — the idiom.
const PREFIX_DECLARING: &str = "sh:declare [ sh:prefix \"imp\" ; \
    sh:namespace \"http://example.org/ns#\"^^xsd:anyURI ]";

/// The neighbour: the import target is described, but only by a label.
const LABELLED_ONLY: &str = "rdfs:label \"a namespace\"";

/// `ex:Invalid` holds `test:Value`, the one the query reports; `ex:Valid` holds another.
const PREFIX_IDIOM_DATA: &str = "<http://example.org/ns#Invalid> \
    <http://example.org/ns#property> <http://example.org/test#Value> .\n\
    <http://example.org/ns#Valid> \
    <http://example.org/ns#property> <http://example.org/test#Other> .\n";

#[test]
fn the_shacl_prefix_idiom_resolves_with_no_table_and_a_labelled_target_does_not() {
    let shapes = prefix_idiom(PREFIX_DECLARING);
    let report = engine::validate_graphs(PREFIX_IDIOM_DATA, &shapes, None)
        .expect("a prefix-declaring import target is in hand");
    assert!(!report.conforms);
    let results: Vec<(String, Option<String>)> = report
        .results
        .iter()
        .map(|r| {
            (
                r.focus_node.to_string(),
                r.value.as_ref().map(ToString::to_string),
            )
        })
        .collect();
    assert_eq!(
        results,
        [(
            "<http://example.org/ns#Invalid>".to_owned(),
            Some("<http://example.org/test#Value>".to_owned())
        )],
        "both declared prefixes reached the query, and only `ex:Invalid` is reported"
    );
    let sarif = validate_to_sarif_string(
        &shapes,
        None,
        PREFIX_IDIOM_DATA,
        &SarifOptions::default(),
        &[],
    )
    .expect("the string boundary agrees");
    assert!(sarif.contains("SPARQLConstraintComponent"), "{sarif}");

    let neighbour = ShapesImportError::Unresolved {
        iris: vec!["http://example.org/ns#".to_owned()],
    };
    let refused = engine::validate_graphs(PREFIX_IDIOM_DATA, &prefix_idiom(LABELLED_ONLY), None)
        .expect_err("a target described only by a label is not in hand");
    assert_eq!(refused.as_imports(), Some(&neighbour), "{refused}");
    let refused = validate_to_sarif_string(
        &prefix_idiom(LABELLED_ONLY),
        None,
        PREFIX_IDIOM_DATA,
        &SarifOptions::default(),
        &[],
    )
    .expect_err("the string boundary refuses it too");
    assert_eq!(refused.as_imports(), Some(&neighbour), "{refused}");
}
