// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Searching from a vector the index does not hold.
//!
//! This is the question an embedding index exists to answer: free text becomes a vector
//! through some model, and that vector is a *query*, never a stored target. Every other
//! suite in this crate searches from a stored row, which is the easy case — a row is its
//! own nearest neighbour at distance zero, so the graph only has to arrive at a node it
//! already sits on. A held-out query is where descent actually has to navigate, and a
//! recall figure taken only over members overstates what the index can do.
//!
//! The queries here are built the way a real workload builds them: half are perturbations
//! of stored rows, which have a true near neighbour to find, and half are drawn fresh from
//! the corpus distribution, which may not.

use purrdf_core::DistanceMetric;
#[path = "support/corpus.rs"]
mod corpus;

use corpus::CorpusShape;
use purrdf_hnsw::level::splitmix64;
use purrdf_hnsw::{HnswIndex, Params, Ranked, VectorMatrix};
use purrdf_sparql_eval::knn::{Kernel, best, norm};

const METRIC: DistanceMetric = DistanceMetric::SquaredEuclidean;
const KERNEL: Kernel = Kernel::SquaredEuclidean;

fn params() -> Params {
    Params::new(16, 32, 64, 16).expect("valid")
}

/// A structured corpus: the only kind on which a recall figure means anything.
fn corpus(rows: usize, dims: usize) -> VectorMatrix {
    corpus::embedding_like(CorpusShape::embedding_like(rows, dims), 0x0BAD_CAFE).expect("generates")
}

/// The exact top-`k` rows for `query`, by the shared kernel — this suite's oracle.
fn exact_top_k(matrix: &VectorMatrix, query: &[f64], k: usize) -> Vec<usize> {
    let query_norm = norm(query);
    let scored: Vec<Ranked> = (0..matrix.rows())
        .map(|row| Ranked {
            distance: KERNEL
                .distance(query, query_norm, matrix.row(row), norm(matrix.row(row)))
                .expect("the fixture is finite"),
            row,
        })
        .collect();
    best(k, scored).into_iter().map(|hit| hit.row).collect()
}

/// A query set built the way a workload builds one: half perturbed members, half fresh.
fn held_out(matrix: &VectorMatrix, count: usize, seed: u64) -> Vec<Vec<f64>> {
    let dims = matrix.dims();
    let mut state = seed;
    let mut next = || {
        state = splitmix64(state);
        ((state >> 11) as f64 / (1_u64 << 53) as f64).mul_add(2.0, -1.0)
    };
    (0..count)
        .map(|index| {
            if index % 2 == 0 {
                // A perturbation of a stored row: near something, but not equal to it.
                let source = matrix.row((index * 7 + 3) % matrix.rows()).to_vec();
                source
                    .iter()
                    .map(|value| next().mul_add(0.05, *value))
                    .collect()
            } else {
                // Drawn from the same generator, so it is in-distribution but held out.
                let fresh = corpus::embedding_like(
                    CorpusShape::embedding_like(1, dims),
                    seed ^ (index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15),
                )
                .expect("generates");
                fresh.row(0).to_vec()
            }
        })
        .collect()
}

#[test]
fn a_stored_row_searched_as_a_vector_gives_the_same_answer_as_searching_by_row() {
    // The two entry points must be one code path, not two that agree today. If they ever
    // diverge, an index would answer the same question differently depending on how the
    // caller phrased it.
    let matrix = corpus(256, 32);
    let index = HnswIndex::build(matrix.clone(), &METRIC, params()).expect("builds");
    for seed in [0_usize, 1, 17, 128, 255] {
        let by_row = index.search_rows(seed, 8).expect("searches");
        let by_vector = index.search_vector(matrix.row(seed), 8).expect("searches");
        assert_eq!(
            by_row, by_vector,
            "row {seed}: the same vector must give the same answer whichever way it is named"
        );
    }
}

#[test]
fn a_query_the_index_does_not_hold_is_answered() {
    // The capability itself: a vector that is not a row, and never was.
    let matrix = corpus(256, 32);
    let index = HnswIndex::build(matrix.clone(), &METRIC, params()).expect("builds");
    let queries = held_out(&matrix, 8, 0xFEED);
    for query in &queries {
        let offered = index.search_vector(query, 5).expect("searches");
        assert_eq!(offered.len(), 5, "a held-out query must still be answered");
        assert!(
            offered.windows(2).all(|pair| pair[0] <= pair[1]),
            "and answered in rank order"
        );
        assert!(
            offered.iter().all(|hit| hit.row < matrix.rows()),
            "naming only rows the index holds"
        );
    }
}

#[test]
fn a_query_of_the_wrong_width_is_refused_and_the_right_width_is_not() {
    // Both directions, per this project's rule that a refusal is a claim needing proof.
    let matrix = corpus(64, 16);
    let index = HnswIndex::build(matrix, &METRIC, params()).expect("builds");
    assert!(
        index.search_vector(&[0.5_f64; 15], 3).is_err(),
        "a short query must be refused rather than padded"
    );
    assert!(
        index.search_vector(&[0.5_f64; 17], 3).is_err(),
        "and a long one rather than truncated"
    );
    assert_eq!(
        index
            .search_vector(&[0.5_f64; 16], 3)
            .expect("the neighbouring valid width must still be admitted")
            .len(),
        3
    );
}

#[test]
fn the_work_reported_is_the_candidates_examined() {
    let matrix = corpus(256, 32);
    let index = HnswIndex::build(matrix.clone(), &METRIC, params()).expect("builds");
    let query = &held_out(&matrix, 1, 0xB0B)[0];
    let (offered, work) = index.search_vector_work(query, 5).expect("searches");
    assert_eq!(offered.len(), 5);
    assert!(
        work >= offered.len() as u64,
        "a search cannot return more rows than it examined; got {work}"
    );
    assert!(
        work <= matrix.rows() as u64,
        "and cannot examine more than the graph holds; got {work}"
    );
}

#[test]
fn held_out_recall_is_measured_rather_than_assumed() {
    // The number this crate previously had no way to produce. Every recall figure elsewhere
    // uses a stored row as its query, which is the case where the answer sits at distance
    // zero; this is the case a user actually meets.
    //
    // Pinned as an exact count, not a floor: recall is a pure function of the corpus, the
    // parameters and the algorithm, so an exact equality catches an improvement as well as a
    // regression, and this repository asserts no thresholds anywhere.
    //
    // The REGIME is chosen so the number carries information. An earlier version of this test
    // pinned 640/640 -- recall 1.000 -- at 512 rows with `ef_search = 64` over a graph with 32
    // layer-0 edges per node, where the beam holds an eighth of the corpus and the search is
    // effectively exhaustive. Perfect recall there is forced by the parameters rather than
    // earned by the graph, so it demonstrated that the index is EXACT on a toy, not that it is
    // a good APPROXIMATION. At 4,096 rows with `ef_search = 16` the beam holds well under one
    // percent and recall is genuinely below one, so a graph that got worse would move it.
    let matrix = corpus(4096, 64);
    let index = HnswIndex::build(matrix.clone(), &METRIC, params()).expect("builds");
    let queries = held_out(&matrix, 64, 0x5EED_5EED);
    let k = 10;

    let mut hits = 0_usize;
    let mut offered_total = 0_usize;
    for query in &queries {
        let truth = exact_top_k(&matrix, query, k);
        let offered = index.search_vector(query, k).expect("searches");
        offered_total += offered.len();
        hits += offered
            .iter()
            .filter(|hit| truth.contains(&hit.row))
            .count();
    }

    assert_eq!(
        (hits, offered_total),
        (604, 640),
        "held-out recall over {} queries at k={k} moved; recall is deterministic, so this \
         is a real behaviour change rather than a tolerance to widen",
        queries.len()
    );
}

/// **A smaller `k` is the prefix of a larger one, out of the same beam.**
///
/// The beam is the artifact's declared `ef_search` and is never narrowed or widened to
/// fit a request; `k` only truncates the beam's sorted result. So a search at any `k`
/// walks the same graph and evaluates the same distances, and its rows are exactly the
/// first `k` of a search at a larger `k`. That is what lets a consumer open a search
/// once at a planned depth and read only as far as it needs: a smaller `k` would have
/// saved no distance evaluation, and a second search at a larger `k` would repeat the
/// whole beam to return rows the first already held.
///
/// Asked from stored rows and from held-out vectors, at every `k` from one past the
/// beam down to one — the widest `k` is answered short, at the beam's width, and every
/// other is its prefix.
#[test]
fn a_smaller_k_is_a_prefix_of_a_larger_one_out_of_the_same_beam() {
    let matrix = corpus(400, 16);
    let index = HnswIndex::build(matrix.clone(), &METRIC, params()).expect("builds");
    let beam = params().ef_search();
    let widest = beam + 1;
    for query_row in [0, 17, 211, 399] {
        let (full, full_work) = index
            .search_rows_work(query_row, widest)
            .expect("the row is held");
        assert_eq!(
            full.len(),
            beam,
            "a k past the beam is answered at the beam's width"
        );
        for k in 1..=widest {
            let (read, work) = index
                .search_rows_work(query_row, k)
                .expect("the row is held");
            assert_eq!(
                read,
                full[..k.min(beam)],
                "row {query_row}: k = {k} is the prefix of k = {widest}"
            );
            assert_eq!(
                work, full_work,
                "row {query_row}: k = {k} evaluated the distances k = {widest} did"
            );
        }
    }
    for (at, query) in held_out(&matrix, 4, 0x5EED).iter().enumerate() {
        let (full, full_work) = index
            .search_vector_work(query, widest)
            .expect("the query has the index's dimension");
        for k in 1..=widest {
            let (read, work) = index
                .search_vector_work(query, k)
                .expect("the query has the index's dimension");
            assert_eq!(read, full[..k.min(full.len())], "held-out {at}: k = {k}");
            assert_eq!(work, full_work, "held-out {at}: k = {k}");
        }
    }
}
