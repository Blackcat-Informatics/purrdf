// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Prepared execution across native mutation snapshots and their compacted form.

use purrdf_core::{
    DatasetMut, MutableDataset, QuadValues, RdfDatasetBuilder, SparqlResult, TermValue,
};
use purrdf_sparql_eval::{NativeSparqlEngine, QueryOptions};

fn iri(value: &str) -> TermValue {
    TermValue::Iri(format!("https://example.org/{value}"))
}

#[test]
fn prepared_joins_and_computed_terms_share_values_across_snapshot_layers() {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("https://example.org/p");
    let a = b.intern_iri("https://example.org/a");
    let shared = b.intern_iri("https://example.org/shared");
    let empty = b.intern_iri("https://example.org/empty");
    b.declare_named_graph(empty);
    b.push_quad(a, p, shared, None);
    let mut mutable = MutableDataset::new(b.freeze().unwrap());
    for (s, o, g) in [
        ("shared", "new", None),
        ("shared", "graph-value", Some("graph")),
    ] {
        mutable
            .insert(QuadValues {
                s: iri(s),
                p: iri("p"),
                o: iri(o),
                g: g.map(iri),
            })
            .unwrap();
    }
    let engine = NativeSparqlEngine::new();
    let queries = [
        "SELECT ?s ?v ?computed WHERE { ?s ex:p ?shared . ?shared ex:p ?v . BIND(CONCAT(STR(?v), '/computed') AS ?computed) } ORDER BY ?s ?v",
        "SELECT ?v WHERE { ex:a ex:p/ex:p ?v } ORDER BY ?v",
        "SELECT ?g ?v WHERE { GRAPH ?g { ?s ex:p ?v } } ORDER BY ?g ?v",
        "SELECT ?g WHERE { GRAPH ?g {} } ORDER BY ?g",
    ];
    let plans: Vec<_> = queries
        .iter()
        .map(|q| {
            engine
                .prepare_query(&format!("PREFIX ex: <https://example.org/> {q}"), None)
                .unwrap()
        })
        .collect();
    for _ in 0..2 {
        let view = mutable.snapshot_view().unwrap();
        let compacted = mutable.freeze().unwrap();
        for plan in &plans {
            let actual = engine
                .query_prepared_view(&view, plan, &[], QueryOptions::EMPTY)
                .unwrap();
            let expected = engine
                .query_prepared(&compacted, plan, &[], QueryOptions::EMPTY)
                .unwrap();
            let (
                SparqlResult::Solutions {
                    variables: av,
                    rows: ar,
                    aux: aa,
                },
                SparqlResult::Solutions {
                    variables: ev,
                    rows: er,
                    aux: ea,
                },
            ) = (actual, expected)
            else {
                panic!("SELECT queries return solution sequences")
            };
            assert_eq!(av, ev);
            assert_eq!(ar, er);
            assert_eq!(aa.quad_count(), 0);
            assert_eq!(ea.quad_count(), 0);
        }
        mutable
            .insert(QuadValues {
                s: iri("shared"),
                p: iri("p"),
                o: iri("later"),
                g: None,
            })
            .unwrap();
    }
}
