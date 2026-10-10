// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Run with `cargo run -p purrdf-sparql-eval --example paged_stack`.
//! Pinned readers survive head publication; consumers own durable publication.

#[path = "support/paged_stack.rs"]
mod fixture;

use fixture::{carrier_pages, eager, seal};
use purrdf_core::term_fixture::iri;
use purrdf_core::{
    PagedQueryLimits, PagedStack, QuadValues, SparqlRequest, TermValue, canonical_paged_seal,
};
use purrdf_sparql_eval::{NativeSparqlEngine, QueryOptions};
use std::collections::BTreeSet;
use std::num::NonZeroUsize;

fn main() {
    let page_rows = NonZeroUsize::new(2).expect("positive partition bound");
    let edge = QuadValues::triple(iri("alice"), iri("knows"), iri("bob"));
    let name = QuadValues::triple(iri("bob"), iri("name"), TermValue::simple_literal("Bob"));
    let mut expected = BTreeSet::from([edge.clone(), name.clone()]);
    let mut stack = PagedStack::new(vec![
        seal(eager([edge.clone()], &[])),
        seal(eager([name.clone()], &[])),
    ])
    .expect("independent generations");
    let old = stack.snapshot().expect("retain reader before edits");
    assert!(stack.remove(&name).expect("remove by RDF value"));
    assert!(expected.remove(&name));
    let replacement =
        QuadValues::triple(iri("bob"), iri("name"), TermValue::simple_literal("Robert"));
    assert!(
        stack
            .insert(replacement.clone())
            .expect("new value in resident head")
    );
    assert!(expected.insert(replacement));
    stack
        .seal_head(page_rows)
        .expect("seal only the head batch");
    assert!(stack.remove(&edge).expect("remove older edge"));
    assert!(
        stack
            .insert(edge)
            .expect("later insertion survives its tombstone")
    );
    let current = stack.snapshot().expect("current reader with unsealed head");
    let engine = NativeSparqlEngine::new();
    let text = "SELECT ?name WHERE { <http://example.org/alice> <http://example.org/knows> ?person . ?person <http://example.org/name> ?name }";
    let prepared = engine
        .prepare_query(text, None)
        .expect("prepare join across dictionaries");
    for (snapshot, wanted) in [(&old, "Bob"), (&current, "Robert")] {
        let view = snapshot.query_view(PagedQueryLimits::UNBOUNDED);
        let complete = engine
            .query_fallible_view(
                &view,
                SparqlRequest {
                    query: text,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::EMPTY,
            )
            .expect("complete layered query");
        let solutions = complete.result.into_solutions().expect("SELECT shape");
        let (variables, rows) = solutions.as_parts();
        assert_eq!(variables, vec!["name"]);
        assert_eq!(rows, vec![vec![Some(TermValue::simple_literal(wanted))]]);
        let repeated = engine
            .query_prepared_fallible_view(&view, &prepared, &[], QueryOptions::EMPTY)
            .expect("same cached reader through prepared API");
        let repeated_solutions = repeated.result.into_solutions().expect("SELECT shape");
        assert_eq!(repeated_solutions.as_parts().1, rows);
        assert_eq!(
            *repeated.evidence, *complete.evidence,
            "no repeated admission charge"
        );
        println!(
            "name={wanted}; layers={}; pages={}; bytes={}; origins={:?}",
            snapshot.sealed_depth(),
            complete.evidence.pages.consumed_pages,
            complete.evidence.pages.consumed_bytes,
            complete.evidence.requested_origins
        );
    }
    let oracle = eager(expected, &[]);
    let canonical = canonical_paged_seal(oracle.as_ref(), page_rows)
        .expect("independent eager canonical input");
    let folded = current
        .compact(PagedQueryLimits::UNBOUNDED, page_rows)
        .expect("guarded fold");
    assert_eq!(
        carrier_pages(&folded),
        carrier_pages(&canonical),
        "actual ordered carrier bytes"
    );
    println!(
        "compacted pages={}; ordered carrier identity verified",
        folded.page_count()
    );
}
