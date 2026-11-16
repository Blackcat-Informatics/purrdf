// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! No query hashes the dataset it reads. A caller that wants an execution identity
//! asks the view for its content identity and binds the run to it.

use std::sync::{Arc, Mutex};

use purrdf_core::{ContentIdentified, RdfDataset, RdfDatasetBuilder, SparqlRequest};
use purrdf_sparql_eval::{
    EvalError, ExecutionCoordinateObserver, ExecutionSources, GovernedOutcome, NativeSparqlEngine,
    QueryCoordinates, QueryGovernors, QueryOptions, ReceiptError, SuppliedQueryCoordinates,
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

fn request() -> SparqlRequest<'static> {
    SparqlRequest {
        query: SELECT,
        base_iri: None,
        substitutions: &[],
    }
}

fn started(coordinates: &QueryCoordinates) -> &SuppliedQueryCoordinates {
    let QueryCoordinates::Started(started) = coordinates else {
        panic!("a started context")
    };
    started
}

/// No receipt and no observer: nobody reads the identity, so a frozen dataset is not hashed.
#[test]
fn a_select_with_no_receipt_requested_hashes_nothing() {
    let dataset = dataset();
    assert!(!identity_hashed(&dataset), "a fresh freeze is unsealed");
    NativeSparqlEngine::new()
        .query_with_options_view(&dataset, request(), &QueryOptions::EMPTY)
        .unwrap();
    assert!(!identity_hashed(&dataset));
}

/// The governed prepared lane returns the run's coordinates with its result; it
/// hashes nothing, and the execution identity is computed only when the caller
/// supplies the dataset's content identity.
#[test]
fn a_governed_prepared_select_returns_coordinates_without_hashing_the_dataset() {
    let dataset = dataset();
    let engine = NativeSparqlEngine::new();
    let prepared = engine.prepare_query(SELECT, None).unwrap();
    let outcome = engine
        .query_prepared_governed_view(
            &dataset,
            &prepared,
            &[],
            &QueryOptions::EMPTY,
            &QueryGovernors::UNBOUNDED,
        )
        .unwrap();
    let GovernedOutcome::Complete { coordinates, .. } = outcome else {
        panic!("an unbounded run completes")
    };
    assert!(!identity_hashed(&dataset));
    let run = *started(&coordinates).run_identity().unwrap();
    let content = dataset
        .content_identity()
        .unwrap_or_else(|never| match never {});
    let execution = coordinates
        .execution_identity(&ExecutionSources::new(content))
        .unwrap();
    assert!(identity_hashed(&dataset));
    assert_ne!(execution, run);
}

/// An observer receives the run identity; it hashes nothing either.
#[test]
fn a_select_with_an_observer_publishes_the_run_identity_without_hashing() {
    let dataset = dataset();
    let observer = Observer::default();
    NativeSparqlEngine::new()
        .query_with_options_view(
            &dataset,
            request(),
            &QueryOptions::new().with_coordinate_observer(&observer),
        )
        .unwrap();
    assert!(!identity_hashed(&dataset));
    let seen = observer.0.lock().unwrap().clone();
    let [coordinates] = seen.as_slice() else {
        panic!("one context")
    };
    assert!(started(coordinates).run_identity().is_ok());
}

/// A run that read a focus graph needs that graph's identity too, and a run that
/// read none refuses one; each refusal has a neighbouring request that succeeds.
/// A run that never started has no execution identity.
#[test]
fn execution_identity_refuses_sources_that_do_not_match_the_run() {
    let focus = dataset();
    let dataset = dataset();
    let engine = NativeSparqlEngine::new();
    let run = |options: &QueryOptions<'_, '_>| {
        let observer = Observer::default();
        engine
            .query_with_options_view(
                &dataset,
                request(),
                &(*options).with_coordinate_observer(&observer),
            )
            .unwrap();
        observer.0.lock().unwrap().remove(0)
    };
    let content = dataset
        .content_identity()
        .unwrap_or_else(|never| match never {});
    let focus_content = focus
        .content_identity()
        .unwrap_or_else(|never| match never {});
    let plain = ExecutionSources::new(content);
    let with_focus = plain.with_focus_graph(focus_content);

    let unfocused = run(&QueryOptions::EMPTY);
    assert!(unfocused.execution_identity(&plain).is_ok());
    assert_eq!(
        unfocused.execution_identity(&with_focus),
        Err(ReceiptError::FocusGraphIdentityUnused)
    );

    let focused = run(&QueryOptions::new().with_focus_graph(Some(&focus)));
    assert!(started(&focused).reads_focus_graph());
    assert_eq!(
        focused.execution_identity(&plain),
        Err(ReceiptError::FocusGraphIdentityMissing)
    );
    assert!(focused.execution_identity(&with_focus).is_ok());

    assert_eq!(
        QueryCoordinates::NotStarted.execution_identity(&plain),
        Err(ReceiptError::NotStarted)
    );
}
