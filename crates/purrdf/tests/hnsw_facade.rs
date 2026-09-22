// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Approximate nearest-neighbour retrieval reached through the umbrella alone.
//!
//! The point of this file is the `use` block. A consumer who depends on `purrdf` and
//! nothing else must be able to name the index, its parameters, its space and its relation,
//! and register that relation on the evaluator's property-function seam — the same shape
//! `purrdf-geo` and `purrdf-text` are re-exported for. A crate the umbrella does not carry
//! is a crate no host of this facade can reach, whatever its own tests prove.
//!
//! The artifact-backed path, the ordering law and the work accounting are pinned inside
//! `purrdf-hnsw`. What cannot be shown from in there is that the re-export exists and that
//! a query driven entirely through this crate answers.
//!
//! `https://example.org/pf#nearest` is a fixture IRI supplied by this test as the host.
//! PurRDF mints no vocabulary.

use std::sync::Arc;

use purrdf::hnsw::relation::HnswSpace;
use purrdf::hnsw::{HnswIndex, Params, VectorMatrix};
use purrdf::sparql::{KnnGuard, NativeSparqlEngine, PropertyFunctionRegistry, QueryOptions};
use purrdf::{DistanceMetric, RdfDatasetBuilder, SparqlRequest, SparqlResult, TermValue};

/// The caller-supplied predicate this host calls approximate retrieval by.
const NEAREST: &str = "https://example.org/pf#nearest";

/// A small deterministic space, built and registered using only umbrella names.
fn registered_space(rows: usize) -> (PropertyFunctionRegistry, Vec<TermValue>) {
    let dims = 4;
    let mut state = 0x51DE_0000_1234_ABCD_u64;
    let mut data = Vec::with_capacity(rows * dims);
    for _ in 0..rows * dims {
        // splitmix64, spelled here so the fixture depends on no private helper.
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        let value = ((z >> 11) as f64 / (1_u64 << 53) as f64).mul_add(2.0, -1.0);
        data.push(if value == 0.0 { 0.125 } else { value });
    }
    let matrix = VectorMatrix::new(rows, dims, data).expect("a valid matrix");
    let params = Params::new(4, 8, 16, 8).expect("valid parameters");
    let index =
        HnswIndex::build(matrix, &DistanceMetric::SquaredEuclidean, params).expect("it builds");

    let terms: Vec<TermValue> = (0..rows)
        .map(|row| TermValue::iri(format!("https://example.org/row/{row}")))
        .collect();
    let guard = KnnGuard::new(rows as u64, rows as u64).expect("a valid guard");
    let space =
        Arc::new(HnswSpace::from_index(index, terms.clone(), guard).expect("a valid space"));

    let mut registry = PropertyFunctionRegistry::new();
    registry.register(NEAREST.to_owned(), Arc::new(space.relation()));
    (registry, terms)
}

fn answer(registry: &PropertyFunctionRegistry, query: &str) -> usize {
    let data = RdfDatasetBuilder::new().freeze().expect("an empty dataset");
    let result = NativeSparqlEngine::new()
        .query_with_options_view(
            &data,
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions {
                env: &purrdf_sparql_eval::ExtensionEnv::over_relations(registry.clone())
                    .expect("the fixture declarations read cleanly"),
                ..QueryOptions::EMPTY
            },
        )
        .unwrap_or_else(|error| panic!("the query must evaluate: {error}"));
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("a SELECT answers with solutions, got {result:?}");
    };
    rows.len()
}

#[test]
fn the_umbrella_carries_approximate_retrieval_to_a_sparql_query() {
    let (registry, terms) = registered_space(64);
    let TermValue::Iri(seed) = &terms[0] else {
        panic!("the fixture binds IRIs");
    };
    let query = format!("SELECT ?neighbour WHERE {{ ?neighbour <{NEAREST}> ( <{seed}> 5 ?d ) }}");

    assert_eq!(
        answer(&registry, &query),
        5,
        "a host depending on `purrdf` alone must be able to run this"
    );
    assert_eq!(
        answer(&PropertyFunctionRegistry::new(), &query),
        0,
        "and the rows must come from the registration, not from the dataset"
    );
}
