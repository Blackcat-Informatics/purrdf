// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A query hashes the dataset's content for its identity only when something reads it.

use std::sync::{Arc, Mutex};

use purrdf_core::{DatasetView, RdfDataset, RdfDatasetBuilder, SparqlRequest};
use purrdf_sparql_eval::{
    EvalError, ExecutionCoordinateObserver, NativeSparqlEngine, QueryCoordinates, QueryOptions,
};

const SELECT: &str = "SELECT ?s WHERE { ?s <http://example.org/p> ?o . FILTER NOT EXISTS { ?o <http://example.org/p> ?s } }";

fn dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri("http://example.org/p").unwrap();
    for index in 0..8 {
        let s = builder
            .intern_iri(&format!("http://example.org/s{index}"))
            .unwrap();
        let o = builder
            .intern_iri(&format!("http://example.org/o{index}"))
            .unwrap();
        builder.push_quad(s, p, o, None).unwrap();
    }
    builder.freeze().unwrap()
}

/// The identity is hashed once per dataset, behind a once-lock, so "sealed" is
/// exactly "one hash was performed" and "unsealed" is "zero".
fn identity_hashed(dataset: &RdfDataset) -> bool {
    !format!("{dataset:?}").contains("snapshot_identity: OnceLock(<uninit>)")
}

#[derive(Default)]
struct Observer(Mutex<Vec<QueryCoordinates>>);

impl ExecutionCoordinateObserver for Observer {
    fn observe(&self, coordinates: &QueryCoordinates) -> Result<(), EvalError> {
        self.0.lock().unwrap().push(coordinates.clone());
        Ok(())
    }
}

fn run(dataset: &RdfDataset, options: &QueryOptions<'_, '_>) {
    let request = SparqlRequest {
        query: SELECT,
        base_iri: None,
        substitutions: &[],
    };
    NativeSparqlEngine::new()
        .query_with_options_view(dataset, request, options)
        .unwrap();
}

/// No receipt and no observer: nobody reads the identity, so a frozen dataset is not hashed.
#[test]
fn a_select_with_no_receipt_requested_hashes_nothing() {
    let dataset = dataset();
    assert!(!identity_hashed(&dataset), "a fresh freeze is unsealed");
    run(&dataset, &QueryOptions::EMPTY);
    assert!(!identity_hashed(&dataset));
}

/// An observer is a receipt request: the identity is hashed once and published unchanged.
#[test]
fn a_select_with_an_observer_hashes_once_and_publishes_the_dataset_identity() {
    let dataset = dataset();
    let observer = Observer::default();
    run(
        &dataset,
        &QueryOptions::new().with_coordinate_observer(&observer),
    );
    assert!(identity_hashed(&dataset));
    let seen = observer.0.lock().unwrap().clone();
    let [QueryCoordinates::Started(started)] = seen.as_slice() else {
        panic!("one started context")
    };
    let expected = DatasetView::snapshot_identity(&*dataset).unwrap();
    assert!(expected.is_some());
    assert_eq!(started.snapshot_identity(), expected.as_ref());
    assert!(started.execution_identity().is_some());
}
