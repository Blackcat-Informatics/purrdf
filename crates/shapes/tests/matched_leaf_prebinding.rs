// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Matched leaves receive SHACL focus bindings before OPTIONAL and nested scopes run.

#[path = "support/relation.rs"]
mod relation;

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::sync::Arc;

use purrdf_core::{
    BlankScope, RdfDataset, RdfDatasetBuilder, SparqlRequest, SparqlResult, TermId, TermValue,
};
use purrdf_shapes::engine::{parse_shapes, validate_dataset};
use purrdf_sparql_eval::{InternedOutcome, NativeSparqlEngine, QueryOptions, ShaclPrebinding};

const EX: &str = "http://example.org/";

fn check(dataset: &RdfDataset, target: &str, body: &str, expected: &BTreeSet<TermId>) {
    let shapes = parse_shapes(
        &format!(
            "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
         <{EX}Shape> a sh:NodeShape ; {target} ;\n\
         sh:sparql [ sh:select \"\"\"SELECT $this WHERE {{ {body} }}\"\"\" ] ."
        ),
        None,
    )
    .expect("shapes");
    let report = validate_dataset(dataset, &shapes)
        .unwrap_or_else(|error| panic!("{body}: validation: {error}"));
    let actual: BTreeSet<TermId> = report
        .results
        .iter()
        .map(|result| {
            relation::id_in(
                dataset,
                &result.focus_node.to_term_value(),
                "reported focus",
            )
        })
        .collect();
    assert_eq!(&actual, expected, "{body}: {report:?}");
    assert_eq!(
        report.results.len(),
        expected.len(),
        "{body}: no duplicate results"
    );
    assert_eq!(report.conforms, expected.is_empty(), "{body}");
}

#[test]
fn missing_property_optional_and_controls_report_only_the_two_missing_nodes() {
    let dataset = relation::ntriples(&format!(
        "<{EX}a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <{EX}C> .\n\
         <{EX}a> <{EX}p> <{EX}v> .\n\
         <{EX}b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <{EX}C> .\n\
         <{EX}c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <{EX}C> ."
    ));
    let expected = ["b", "c"]
        .into_iter()
        .map(|name| {
            dataset
                .term_id_by_iri(&format!("{EX}{name}"))
                .expect("focus")
        })
        .collect();
    for body in [
        format!("OPTIONAL {{ $this <{EX}p> ?v }} FILTER(!BOUND(?v))"),
        format!("FILTER NOT EXISTS {{ $this <{EX}p> ?v }}"),
        format!("$this a <{EX}C> OPTIONAL {{ $this <{EX}p> ?v }} FILTER(!BOUND(?v))"),
        format!("OPTIONAL {{ {{ OPTIONAL {{ $this <{EX}p> ?v }} }} }} FILTER(!BOUND(?v))"),
        format!("FILTER NOT EXISTS {{ OPTIONAL {{ $this <{EX}p> ?v }} FILTER(BOUND(?v)) }}"),
    ] {
        check(
            &dataset,
            &format!("sh:targetClass <{EX}C>"),
            &body,
            &expected,
        );
    }
}

fn all_kinds() -> (Arc<RdfDataset>, Vec<TermId>) {
    let pairs = [
        (format!("<{EX}iri-bad>"), format!("<{EX}iri-good>")),
        ("_:bad".to_owned(), "_:good".to_owned()),
        ("\"bad literal\"".to_owned(), "\"good literal\"".to_owned()),
        (
            "\"bad direction\"@en--rtl".to_owned(),
            "\"good direction\"@en--rtl".to_owned(),
        ),
        (
            format!("<<( <{EX}a> <{EX}r> <{EX}bad> )>>"),
            format!("<<( <{EX}a> <{EX}r> <{EX}good> )>>"),
        ),
        (
            format!("<<( <{EX}a> <{EX}r> _:qbad )>>"),
            format!("<<( <{EX}a> <{EX}r> _:qgood )>>"),
        ),
        (
            format!("<<( <{EX}a> <{EX}r> <<( _:deepbad <{EX}r> <{EX}x> )>> )>>"),
            format!("<<( <{EX}a> <{EX}r> <<( _:deepgood <{EX}r> <{EX}x> )>> )>>"),
        ),
    ];
    let mut text = String::new();
    for (index, (bad, good)) in pairs.iter().enumerate() {
        for (state, item) in [("bad", bad), ("good", good)] {
            writeln!(text,
                "<{EX}root> <{EX}focus> {item} .\n\
                 <{EX}{state}{index}> <{EX}about> {item} .\n\
                 <{EX}{state}{index}> <{EX}quoted> <<( <{EX}anchor> <{EX}r> <<( <{EX}anchor> <{EX}r> {item} )>> )>> ."
            ).expect("write");
        }
        writeln!(text, "<{EX}good{index}> <{EX}p> <{EX}present> .").expect("write");
    }
    let dataset = relation::ntriples(&text);
    let about = dataset
        .term_id_by_iri(&format!("{EX}about"))
        .expect("about");
    let items: Vec<TermId> = (0..pairs.len())
        .flat_map(|index| ["bad", "good"].into_iter().map(move |state| (index, state)))
        .map(|(index, state)| {
            let subject = dataset
                .term_id_by_iri(&format!("{EX}{state}{index}"))
                .expect("record");
            dataset
                .quads()
                .find(|quad| quad.s == subject && quad.p == about)
                .expect("about record")
                .o
        })
        .collect();
    assert_eq!(
        items.iter().copied().collect::<BTreeSet<_>>().len(),
        items.len(),
        "every node is distinct"
    );
    (dataset, items)
}

#[test]
fn every_focus_kind_keeps_its_identity_in_nested_matched_positions() {
    let (dataset, items) = all_kinds();
    let expected: BTreeSet<TermId> = items.into_iter().step_by(2).collect();
    for body in [
        format!("OPTIONAL {{ ?rec <{EX}about> $this ; <{EX}p> ?v }} FILTER(!BOUND(?v))"),
        format!("OPTIONAL {{ $this ^<{EX}about>/<{EX}p> ?v }} FILTER(!BOUND(?v))"),
        format!(
            "OPTIONAL {{ {{ SELECT $this ?v WHERE {{ ?rec <{EX}about> $this ; <{EX}p> ?v }} LIMIT 1 }} }} FILTER(!BOUND(?v))"
        ),
        format!(
            "OPTIONAL {{ ?rec <{EX}quoted> <<( <{EX}anchor> <{EX}r> <<( <{EX}anchor> <{EX}r> $this )>> )>> ; <{EX}p> ?v }} FILTER(!BOUND(?v))"
        ),
        format!(
            "FILTER NOT EXISTS {{ OPTIONAL {{ ?rec <{EX}about> $this ; <{EX}p> ?v }} FILTER(BOUND(?v)) }}"
        ),
    ] {
        check(
            &dataset,
            &format!("sh:targetObjectsOf <{EX}focus>"),
            &body,
            &expected,
        );
    }
}

#[test]
fn minus_remains_forbidden_at_the_shacl_loader() {
    let shapes = format!(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
         <{EX}Shape> a sh:NodeShape ; sh:targetNode <{EX}focus> ;\n\
         sh:sparql [ sh:select \"SELECT $this WHERE {{ MINUS {{ $this <{EX}p> ?v }} }}\" ] ."
    );
    assert!(
        parse_shapes(&shapes, None).is_err(),
        "MINUS is forbidden in SHACL-SPARQL"
    );
}

#[test]
fn cached_rebinding_uses_each_exact_focus_in_optional_and_nested_modifiers() {
    let (dataset, items) = all_kinds();
    let engine = NativeSparqlEngine::new();
    let options = QueryOptions::EMPTY.with_prebinding(ShaclPrebinding::Applied);
    for body in [
        format!("SELECT $this ?v WHERE {{ OPTIONAL {{ ?rec <{EX}about> $this ; <{EX}p> ?v }} }}"),
        format!(
            "SELECT $this ?v WHERE {{ OPTIONAL {{ {{ SELECT $this ?v WHERE {{ ?rec <{EX}about> $this ; <{EX}p> ?v }} LIMIT 1 }} }} }}"
        ),
        format!(
            "SELECT $this ?v WHERE {{ OPTIONAL {{ {{ SELECT $this (SAMPLE(?value) AS ?v) WHERE {{ ?rec <{EX}about> $this ; <{EX}p> ?value }} GROUP BY $this }} }} }}"
        ),
    ] {
        let mut prepared = engine
            .prepare_execution(&body, None, &["this"], options)
            .expect("prepare");
        // Consecutive nodes of the same kind test memo replay; two passes also cross
        // IRI, blank, literal, directional and nested quoted value shapes repeatedly.
        for &item in items.iter().chain(&items) {
            let focus = dataset.term_value(item);
            prepared.bind(0, focus.clone()).expect("bind");
            let rows = engine
                .execute(&mut prepared, &*dataset, options, |outcome| {
                    let InternedOutcome::Solutions(solutions) = outcome else {
                        panic!("SELECT");
                    };
                    solutions
                        .rows()
                        .iter()
                        .map(|row| (solutions.cell(row, 0), solutions.cell(row, 1)))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_else(|error| panic!("{body}: {focus:?}: {error}"));
            let index = items
                .iter()
                .position(|candidate| *candidate == item)
                .expect("item");
            let value = (index % 2 == 1).then(|| TermValue::iri(format!("{EX}present")));
            assert_eq!(rows, vec![(Some(focus), value)], "{body}");
        }
    }
}

#[test]
fn predicates_and_nested_predicates_rebind_and_non_iris_match_nothing() {
    let (dataset, items) = all_kinds();
    let engine = NativeSparqlEngine::new();
    let options = QueryOptions::EMPTY.with_prebinding(ShaclPrebinding::Applied);
    for (query, matching) in [
        ("SELECT ?rec WHERE { ?rec $this ?o }".to_owned(), "about"),
        (
            format!(
                "SELECT ?rec WHERE {{ OPTIONAL {{ ?rec <{EX}quoted> <<( <{EX}anchor> $this <<( <{EX}anchor> <{EX}r> ?x )>> )>> }} FILTER(BOUND(?rec)) }}"
            ),
            "r",
        ),
        (
            format!(
                "SELECT ?rec WHERE {{ OPTIONAL {{ ?rec <{EX}quoted> <<( <{EX}anchor> <{EX}r> <<( <{EX}anchor> $this ?x )>> )>> }} FILTER(BOUND(?rec)) }}"
            ),
            "r",
        ),
    ] {
        let mut prepared = engine
            .prepare_execution(&query, None, &["this"], options)
            .expect("prepare");
        let bindings = [
            TermValue::iri(format!("{EX}{matching}")),
            TermValue::iri(format!("{EX}absent")),
        ];
        for focus in bindings
            .iter()
            .chain(
                items
                    .iter()
                    .map(|item| dataset.term_value(*item))
                    .collect::<Vec<_>>()
                    .iter(),
            )
            .chain(&bindings)
        {
            prepared.bind(0, focus.clone()).expect("bind");
            let count = engine
                .execute(&mut prepared, &*dataset, options, |outcome| {
                    let InternedOutcome::Solutions(solutions) = outcome else {
                        panic!("SELECT");
                    };
                    solutions.len()
                })
                .unwrap_or_else(|error| panic!("{query}: {focus:?}: {error}"));
            let expected = if focus == &bindings[0] {
                items.len()
            } else {
                0
            };
            assert_eq!(count, expected, "{query}: {focus:?}");
        }
    }
}

#[test]
fn shacl_optional_and_minus_keep_ordinary_binding_semantics_distinct() {
    let (dataset, _) = all_kinds();
    let engine = NativeSparqlEngine::new();
    let substitutions = [("this".to_owned(), TermValue::iri(format!("{EX}iri-good")))];
    for (body, ordinary_count, shacl_count) in [
        (
            format!("?rec <{EX}about> ?x MINUS {{ ?rec <{EX}about> $this }}"),
            0,
            13,
        ),
        (
            format!("?rec <{EX}about> ?x OPTIONAL {{ ?rec <{EX}about> $this }}"),
            1,
            14,
        ),
    ] {
        let query = format!("SELECT ?rec WHERE {{ {body} }}");
        for (lane, expected) in [
            (ShaclPrebinding::None, ordinary_count),
            (ShaclPrebinding::Applied, shacl_count),
        ] {
            let result = engine
                .query_with_options_view(
                    &*dataset,
                    SparqlRequest {
                        query: &query,
                        base_iri: None,
                        substitutions: &substitutions,
                    },
                    QueryOptions::EMPTY.with_prebinding(lane),
                )
                .expect("query");
            let SparqlResult::Solutions { rows, .. } = result else {
                panic!("SELECT");
            };
            assert_eq!(rows.len(), expected, "{query}: {lane:?}");
        }
    }
}

#[test]
fn graph_name_only_bindings_are_local_in_optional_and_exists_scopes() {
    let (focus_data, items) = all_kinds();
    let mut builder = RdfDatasetBuilder::new();
    let graph = builder.intern_iri(&format!("{EX}graph"));
    let subject = builder.intern_iri(&format!("{EX}s"));
    let predicate = builder.intern_iri(&format!("{EX}p"));
    let value = builder.intern_iri(&format!("{EX}present"));
    builder.push_quad(subject, predicate, value, Some(graph));
    let blank_good = builder.intern_blank("graph-good", BlankScope::DEFAULT);
    let blank_bad = builder.intern_blank("graph-bad", BlankScope::DEFAULT);
    builder.push_quad(subject, predicate, value, Some(blank_good));
    builder.declare_named_graph(blank_bad);
    let dataset = builder.freeze().expect("named graph");
    let engine = NativeSparqlEngine::new();
    let options = QueryOptions::EMPTY.with_prebinding(ShaclPrebinding::Applied);
    let mut focuses = vec![
        TermValue::iri(format!("{EX}graph")),
        TermValue::iri(format!("{EX}absent")),
    ];
    focuses.extend([
        dataset.term_value(blank_good),
        dataset.term_value(blank_bad),
    ]);
    focuses.extend(items.iter().map(|item| focus_data.term_value(*item)));
    for query in [
        "SELECT $this WHERE { OPTIONAL { GRAPH $this { ?s ?p ?v } } FILTER(!BOUND(?v)) }",
        "SELECT $this WHERE { FILTER NOT EXISTS { GRAPH $this { ?s ?p ?v } } }",
        "SELECT $this WHERE { FILTER NOT EXISTS { OPTIONAL { GRAPH $this { ?s ?p ?v } } FILTER(BOUND(?v)) } }",
    ] {
        let mut prepared = engine
            .prepare_execution(query, None, &["this"], options)
            .expect("prepare");
        for focus in focuses.iter().chain(&focuses) {
            prepared.bind(0, focus.clone()).expect("bind");
            let rows = engine
                .execute(&mut prepared, &dataset, options, |outcome| {
                    let InternedOutcome::Solutions(solutions) = outcome else {
                        panic!("SELECT");
                    };
                    solutions
                        .rows()
                        .iter()
                        .map(|row| solutions.cell(row, 0))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_else(|error| panic!("{query}: {focus:?}: {error}"));
            let expected = if focus == &focuses[0] || focus == &dataset.term_value(blank_good) {
                Vec::new()
            } else {
                vec![Some(focus.clone())]
            };
            assert_eq!(rows, expected, "{query}: {focus:?}");
        }
    }
}
