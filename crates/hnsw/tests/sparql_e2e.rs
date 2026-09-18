// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Approximate retrieval driven the way a host drives it: SPARQL query **text**
//! through [`NativeSparqlEngine`], over a real PURREMB artifact.
//!
//! Every other test in this crate opens a cursor directly, or drives the relation through
//! `PropertyFunction::open`. Those pin what the engine emits once it has been asked; what
//! they cannot show is that the seam a host actually reaches through — parse, registry
//! resolution, evaluation, answers — lines up with it. A capability whose only callers
//! construct its internals is a capability no query can reach.
//!
//! The artifact here is the genuine one: a certified source, a typed embedding family, the
//! HNSW guard and payload in the real `INDEX_GUARDS` and `INDEX_PAYLOAD` sections, reopened
//! and verified before the space is built. The engine is a default `NativeSparqlEngine`
//! with no parser options, so the only reason the predicate resolves is the registration.
//!
//! # PurRDF mints no vocabulary
//!
//! `https://example.org/pf#nearest` is a **fixture** IRI supplied by this test in its role
//! as the host. It is not a default, and this crate contains no namespace of its own.

#[path = "support/purremb.rs"]
mod purremb;

use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, SparqlRequest, SparqlResult, TermValue};
use purrdf_hnsw::{Params, relation::HnswSpace};
use purrdf_sparql_eval::{KnnGuard, NativeSparqlEngine, PropertyFunctionRegistry, QueryOptions};

/// The caller-supplied predicate this host calls approximate retrieval by.
const NEAREST: &str = "https://example.org/pf#nearest";

fn params() -> Params {
    Params::new(4, 8, 16, 8).expect("valid")
}

/// A space over the real artifact, plus the row terms the fixture bound.
fn space(rows: usize) -> (Arc<HnswSpace>, Vec<TermValue>) {
    let fixture = purremb::Fixture::new(rows, 8, params());
    let guard = KnnGuard::new(rows as u64, rows as u64).expect("valid guard");
    let space = HnswSpace::from_artifact(
        &fixture.bytes,
        fixture.target_set,
        fixture.vector_space,
        fixture.bindings(),
        guard,
    )
    .expect("the artifact yields a space");
    (Arc::new(space), fixture.terms)
}

/// A registry with the relation registered under `NEAREST` — the whole of what a host does.
fn registry(space: &Arc<HnswSpace>) -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(NEAREST.to_owned(), Arc::new(space.relation()));
    registry
}

/// An empty dataset: the relation is self-contained, so nothing here comes from quads.
fn dataset() -> Arc<RdfDataset> {
    RdfDatasetBuilder::new().freeze().expect("empty dataset")
}

/// Evaluate `query` with `registry` in scope and return the solution rows as terms.
fn answer(registry: &PropertyFunctionRegistry, query: &str) -> Vec<Vec<Option<TermValue>>> {
    let data = dataset();
    let result = NativeSparqlEngine::new()
        .query_with_options_view(
            &data,
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions {
                property_functions: registry,
                ..QueryOptions::EMPTY
            },
        )
        .unwrap_or_else(|error| panic!("the query must evaluate: {error}"));
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("a SELECT answers with solutions, got {result:?}");
    };
    rows.iter().map(Clone::clone).collect()
}

#[test]
fn a_sparql_query_retrieves_neighbours_through_the_registered_predicate() {
    let (space, terms) = space(48);
    let seed = &terms[0];
    let rows = answer(
        &registry(&space),
        &format!(
            "SELECT ?neighbour ?distance WHERE {{ \
             ?neighbour <{NEAREST}> ( <{}> 5 ?distance ) }}",
            match seed {
                TermValue::Iri(iri) => iri.clone(),
                other => panic!("the fixture binds IRIs, got {other:?}"),
            }
        ),
    );

    assert_eq!(rows.len(), 5, "the query asked for five neighbours");
    assert!(
        rows.iter().all(|row| row.len() == 2),
        "each solution binds ?neighbour and ?distance"
    );
    assert_eq!(
        rows[0][0].as_ref(),
        Some(seed),
        "a seed is its own nearest neighbour at distance zero"
    );
    assert!(
        rows.iter().all(|row| row[0].is_some() && row[1].is_some()),
        "no solution leaves a projected variable unbound"
    );
}

#[test]
fn the_offer_is_ordered_nearest_first() {
    let (space, terms) = space(64);
    let TermValue::Iri(seed) = &terms[7] else {
        panic!("the fixture binds IRIs");
    };
    let rows = answer(
        &registry(&space),
        &format!(
            "SELECT ?neighbour ?distance WHERE {{ \
             ?neighbour <{NEAREST}> ( <{seed}> 6 ?distance ) }}"
        ),
    );

    let distances: Vec<f64> = rows
        .iter()
        .map(|row| match row[1].as_ref() {
            Some(TermValue::Literal { lexical_form, .. }) => {
                lexical_form.parse().expect("a numeric distance")
            }
            other => panic!("?distance must bind a literal, got {other:?}"),
        })
        .collect();

    assert_eq!(distances.len(), 6);
    assert!(
        distances.windows(2).all(|pair| pair[0] <= pair[1]),
        "the offer must be ordered nearest first, got {distances:?}"
    );
}

#[test]
fn a_limit_takes_a_prefix_of_the_same_offer() {
    // The engine may push a row ceiling down through `open`. That ceiling decides how many
    // rows are emitted and must never decide which graph is searched.
    let (space, terms) = space(64);
    let TermValue::Iri(seed) = &terms[3] else {
        panic!("the fixture binds IRIs");
    };
    let registry = registry(&space);
    let full = answer(
        &registry,
        &format!("SELECT ?neighbour WHERE {{ ?neighbour <{NEAREST}> ( <{seed}> 6 ?distance ) }}"),
    );
    let limited = answer(
        &registry,
        &format!(
            "SELECT ?neighbour WHERE {{ ?neighbour <{NEAREST}> ( <{seed}> 6 ?distance ) }} \
             LIMIT 2"
        ),
    );

    assert_eq!(limited.len(), 2);
    assert_eq!(
        limited,
        full[..2],
        "a LIMIT must yield a prefix of the unlimited offer"
    );
}

#[test]
fn the_rows_come_from_the_registration_and_from_nothing_else() {
    // The non-vacuity control. Without it, every assertion above would be equally satisfied
    // by an engine that matched the pattern some other way, and the tests could not
    // distinguish "retrieval ran" from "the pattern happened to match".
    //
    // The dataset is empty and the predicate is a fixture IRI no quad mentions, so with an
    // empty registry the same query text is an ordinary triple pattern and answers nothing.
    // The only difference between the two runs is the registration.
    let (space, terms) = space(48);
    let TermValue::Iri(seed) = &terms[0] else {
        panic!("the fixture binds IRIs");
    };
    let query = format!("SELECT ?neighbour WHERE {{ ?neighbour <{NEAREST}> ( <{seed}> 5 ?d ) }}");

    let unregistered = answer(&PropertyFunctionRegistry::new(), &query);
    assert!(
        unregistered.is_empty(),
        "an unregistered predicate is an ordinary pattern over an empty dataset, so it \
         cannot be the source of any row"
    );

    let registered = answer(&registry(&space), &query);
    assert_eq!(
        registered.len(),
        5,
        "registering the relation is what produces the rows"
    );
}
