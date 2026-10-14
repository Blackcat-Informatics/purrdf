// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Omitted public UPDATE options select remembered graph lifetime.

use std::sync::Arc;

use purrdf_core::{
    GraphExistenceMode, RdfDataset, RdfDatasetBuilder, SparqlEngine, SparqlRequest, SparqlResult,
};
use purrdf_sparql_eval::{
    GovernedUpdateOutcome, GraphResolveRequest, GraphResolver, LoadError, NativeSparqlEngine,
    QueryGovernors, QueryOptions,
};

fn request(query: &str) -> SparqlRequest<'_> {
    SparqlRequest {
        query,
        base_iri: None,
        substitutions: &[],
    }
}

fn empty() -> Arc<RdfDataset> {
    RdfDatasetBuilder::new().freeze().expect("empty dataset")
}

struct EmptyDocument;

impl GraphResolver for EmptyDocument {
    fn resolve(&self, request: GraphResolveRequest<'_>) -> Result<Arc<RdfDataset>, LoadError> {
        assert_eq!(request.iri, "http://example.org/document");
        Ok(empty())
    }
}

#[test]
fn default_options_empty_load_declares_destination_and_constant_graph_sees_it() {
    assert_eq!(
        QueryOptions::default().graph_existence,
        GraphExistenceMode::RememberEmpty
    );
    let engine = NativeSparqlEngine::new();
    for governed in [false, true] {
        let mut dataset = empty();
        let options = QueryOptions::default().with_load(Some(&EmptyDocument));
        let text = "LOAD <http://example.org/document> INTO GRAPH <http://example.org/g>";
        if governed {
            assert!(matches!(
                engine
                    .update_governed(
                        &mut dataset,
                        request(text),
                        options,
                        &QueryGovernors::UNBOUNDED
                    )
                    .expect("empty LOAD"),
                GovernedUpdateOutcome::Applied { .. }
            ));
        } else {
            engine
                .update_with_options(&mut dataset, request(text), options)
                .expect("empty LOAD");
        }
        assert_eq!(observed(&engine, &dataset), 1);
        assert!(matches!(
            engine
                .query(&dataset, request("ASK { GRAPH <http://example.org/g> {} }"))
                .expect("constant GRAPH"),
            SparqlResult::Boolean(true)
        ));
        let mut implicit = empty();
        engine
            .update_with_options(
                &mut implicit,
                request(text),
                options.with_graph_existence(GraphExistenceMode::Implicit),
            )
            .expect("implicit LOAD neighbor");
        assert_eq!(observed(&engine, &implicit), 0);
    }
}

fn observed(engine: &NativeSparqlEngine, dataset: &Arc<RdfDataset>) -> usize {
    let SparqlResult::Solutions { rows, .. } = engine
        .query(dataset, request("SELECT ?g WHERE { GRAPH ?g {} }"))
        .expect("public GRAPH query")
    else {
        panic!("solutions")
    };
    assert_eq!(rows.len(), dataset.named_graphs().count());
    assert_eq!(dataset.capabilities().named_graphs, !rows.is_empty());
    rows.len()
}

#[test]
fn omitted_update_options_retain_clear_and_empty_transfer_destinations() {
    let engine = NativeSparqlEngine::new();
    for governed in [false, true] {
        let mut dataset = empty();
        let text = "CREATE GRAPH <http://example.org/g>; COPY GRAPH <http://example.org/g> TO GRAPH <http://example.org/h>; CLEAR GRAPH <http://example.org/g>";
        if governed {
            assert!(matches!(
                engine
                    .update_governed(
                        &mut dataset,
                        request(text),
                        QueryOptions::EMPTY,
                        &QueryGovernors::UNBOUNDED
                    )
                    .expect("governed update"),
                GovernedUpdateOutcome::Applied { .. }
            ));
        } else {
            engine
                .update(&mut dataset, request(text))
                .expect("ordinary update");
        }
        assert_eq!(observed(&engine, &dataset), 2);
        engine
            .update(&mut dataset, request("DROP ALL"))
            .expect("withdraws slots");
        assert_eq!(observed(&engine, &dataset), 0);
    }
    let mut implicit = empty();
    engine
        .update_with_options(
            &mut implicit,
            request("CREATE GRAPH <http://example.org/g>"),
            QueryOptions::EMPTY.with_graph_existence(GraphExistenceMode::Implicit),
        )
        .expect("explicit implicit neighbor");
    assert_eq!(observed(&engine, &implicit), 0);
}

#[test]
fn omitted_governed_policy_refusal_publishes_no_declaration_prefix() {
    let engine = NativeSparqlEngine::new();
    let mut dataset = empty();
    let original = Arc::clone(&dataset);
    let text = "CREATE GRAPH <http://example.org/g>; INSERT DATA { GRAPH <http://example.org/g> { <http://example.org/s> <http://example.org/p> <http://example.org/o> } }";
    let outcome = engine
        .update_governed(
            &mut dataset,
            request(text),
            QueryOptions::EMPTY,
            &QueryGovernors::UNBOUNDED.with_fuel(0),
        )
        .expect("typed refusal");
    assert!(matches!(
        outcome,
        GovernedUpdateOutcome::BudgetExhausted { .. }
    ));
    assert!(Arc::ptr_eq(&dataset, &original));
    assert_eq!(observed(&engine, &dataset), 0);
    assert!(matches!(
        engine
            .update_governed(
                &mut dataset,
                request(text),
                QueryOptions::EMPTY,
                &QueryGovernors::UNBOUNDED.with_fuel(1)
            )
            .expect("valid budget neighbor"),
        GovernedUpdateOutcome::Applied { .. }
    ));
    assert_eq!(observed(&engine, &dataset), 1);
    engine
        .update(
            &mut dataset,
            request("DELETE WHERE { GRAPH <http://example.org/g> { ?s ?p ?o } }"),
        )
        .expect("last row removed");
    assert_eq!(observed(&engine, &dataset), 1);
}
