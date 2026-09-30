// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The small fixtures the HNSW integration tests and benches share: the modest build
//! parameters, a seeded matrix, a batch answer as bits, one kNN invocation and the exact
//! scoring of every row against a query.

// Included by `#[path]` into several test and bench binaries, each of which uses a different
// subset. Matches the sibling `support/purremb.rs`.
#![allow(dead_code, unreachable_pub, unused_imports)]

use purrdf_core::distance::{Arithmetic, Exact};
use purrdf_hnsw::{Kernel, Ranked, VectorMatrix};
use purrdf_sparql_eval::{EmbeddingKnnRelation, EvalError, PfArgs, PfRow, PropertyFunction};

pub use purrdf_hnsw::fixture::params;

/// A deterministic `rows x dims` matrix of SplitMix64 draws in `[-1, 1)` from `seed`.
/// With `zero = Some(substitute)` an exact zero draw is replaced by `substitute`; with
/// `None` the draws are kept as they are. Nothing here reads a clock or an RNG.
pub fn seeded_matrix(rows: usize, dims: usize, seed: u64, zero: Option<f64>) -> VectorMatrix {
    let mut state = seed;
    let data = (0..rows * dims)
        .map(|_| match zero {
            Some(substitute) => {
                purrdf_testkit::rng::signed_unit_step_nonzero(&mut state, substitute)
            }
            None => purrdf_testkit::rng::signed_unit_step(&mut state),
        })
        .collect();
    VectorMatrix::new(rows, dims, data).expect("the fixture matrix is valid")
}

/// A batch answer as its rows and distance bits, so equality is bit-identity.
pub fn bits(batch: &[Vec<Ranked>]) -> Vec<Vec<(usize, u64)>> {
    batch
        .iter()
        .map(|ranked| {
            ranked
                .iter()
                .map(|scored| (scored.row, scored.distance.to_bits()))
                .collect()
        })
        .collect()
}

/// Every row of one kNN invocation seeded at `seed`: the ranked read of depth 3 when
/// `neighbour` is free, the membership lookup of `neighbour` when it is bound.
pub fn knn_invoke<A: Arithmetic>(
    relation: &EmbeddingKnnRelation<A>,
    seed: &purrdf_core::TermValue,
    neighbour: Option<&purrdf_core::TermValue>,
) -> Result<Vec<PfRow>, EvalError> {
    let count =
        purrdf_core::TermValue::typed_literal("3", "http://www.w3.org/2001/XMLSchema#integer");
    let subject = [neighbour];
    let object = [Some(seed), neighbour.is_none().then_some(&count), None];
    let args = PfArgs::new(&subject, &object);
    let mut cursor = relation.open(&args, None)?;
    let mut rows = Vec::new();
    while let Some(row) = cursor.next()? {
        rows.push(row);
    }
    Ok(rows)
}

/// Every row of `matrix` scored against row `query` under `kernel`, in the exact path's
/// order. `norms` are the rows' exact norms.
pub fn exact_scored(
    kernel: Kernel,
    matrix: &VectorMatrix,
    norms: &[f64],
    query: usize,
) -> Vec<Ranked> {
    let exact = Exact::resolve().expect("the default float environment is the IEEE one");
    let vector = matrix.row(query);
    (0..matrix.rows())
        .map(|row| Ranked {
            distance: kernel
                .distance(exact, vector, norms[query], matrix.row(row), norms[row])
                .expect("the fixture is finite and the kernel keeps it so"),
            row,
        })
        .collect()
}
