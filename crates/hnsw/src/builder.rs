// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The round-structured, terminating builder.
//!
//! # Why rounds instead of serial insertion
//!
//! A conventional HNSW build inserts node `q`, then the next node sees the links `q`
//! just made. That makes the graph a function of insertion order, and it makes a parallel
//! build impossible: two workers inserting at once would see different graphs. This
//! builder cuts the dependency by processing **finitely many batches**, each a pure
//! function of the graph that existed before the batch:
//!
//! 1. **Freeze.** The graph from the previous round is the sole snapshot every proposal
//!    in this round reads. Round 1's snapshot is empty, so its batch is the bootstrap
//!    seed and links to nothing.
//! 2. **Propose (parallel).** Each node in the batch runs the standard insertion search
//!    against the snapshot only: greedy descent above its level when the snapshot has
//!    layers there, then an `ef_construction` beam at each layer from its top down to
//!    layer 0. Candidates are the shared `Ranked` order `(distance, row)`, and the
//!    proposal links are emitted in **both directions**.
//! 3. **Merge (canonical).** Every touched node's new neighbourhood is
//!    `frozen neighbours ∪ inbound proposals`, sorted by `(distance, row)` and truncated
//!    to the per-layer degree bound. A set union followed by a total sort is order-free,
//!    so the merge cannot see which worker produced which proposal.
//! 4. **Commit.** The entry point becomes the minimum row index at the current maximum
//!    level.
//!
//! The loop is a `for` over a finite partition of the rows — `ceil(n / B)` rounds with
//! `B = max(1, isqrt(n))` — so termination is structural. There is no `while pending` and
//! no level-based re-queue; a node is proposed exactly once, in its own batch.

use rayon::prelude::*;

use purrdf_sparql_eval::knn::{Kernel, Ranked, norm};

use crate::HnswIndex;
use crate::error::{HnswError, Result};
use crate::graph::{Edge, Graph, VectorMatrix};
use crate::level::{level_cap, level_from_index};
use crate::params::Params;
use crate::search::{DistanceCache, Query, Visited, greedy_descend, search_layer};

/// One node's proposal: per layer, the selected neighbours in rank order.
type NodeProposal = Vec<(u32, Vec<Ranked>)>;

/// The round-invariant inputs every proposal in a batch reads.
struct Round<'a> {
    /// The frozen snapshot the proposals may look at and nothing else.
    frozen: &'a Graph,
    /// The vectors the query rows index.
    matrix: &'a VectorMatrix,
    /// The ranking kernel.
    kernel: Kernel,
    /// Per-row L2 norms, empty for a kernel that does not divide by one.
    norms: &'a [f64],
    /// The declared identity.
    params: &'a Params,
    /// The round's distance memo.
    cache: &'a DistanceCache,
}

/// Build an index over `matrix` under `kernel` and `params`.
///
/// # Errors
///
/// [`HnswError::InvalidParameter`] / [`HnswError::ParameterValidation`] for a parameter or
/// matrix problem caught before any work; [`HnswError::ZeroNorm`] when `kernel` divides by
/// a norm and a row's is zero; [`HnswError::NonFiniteDistance`] if a kernel result leaves
/// the finite range.
pub(crate) fn build(matrix: VectorMatrix, kernel: Kernel, params: Params) -> Result<HnswIndex> {
    params.validate_against(matrix.rows(), matrix.dims())?;

    let n = matrix.rows();
    let cap = level_cap(n, params.m());
    let levels: Vec<u32> = (0..n)
        .map(|row| level_from_index(row as u64, params.m(), cap))
        .collect();
    let norms = compute_norms(&matrix, kernel)?;

    let mut graph = Graph::with_levels(levels.clone());
    let batch = n.isqrt().max(1);
    let mut start = 0;
    while start < n {
        let end = (start + batch).min(n);
        let batch_rows: Vec<usize> = (start..end).collect();
        let edges = {
            let cache = DistanceCache::new();
            let round = Round {
                frozen: &graph,
                matrix: &matrix,
                kernel,
                norms: &norms,
                params: &params,
                cache: &cache,
            };
            propose_round(&round, &batch_rows, &levels)?
        };
        graph.commit(edges, &params);

        // Entry point: the minimum row at the current maximum level. Rows are processed in
        // ascending order, so the first row to attain a strictly higher level is the
        // minimum row at that level and no later row can displace it on a tie.
        for &node in &batch_rows {
            let level = levels[node];
            if graph.entry().is_none() || level > graph.max_level() {
                graph.promote_entry(node, level);
            }
        }
        start = end;
    }

    Ok(HnswIndex::new(matrix, kernel, params, graph, norms))
}

/// The per-row L2 norms, or an empty vector for a kernel that does not divide by one.
///
/// Computed once, before any graph work, so a space that cannot be searched under the
/// chosen metric fails at construction rather than mid-build. PURREMB v1: cosine distance
/// is undefined for a zero-norm operand.
pub(crate) fn compute_norms(matrix: &VectorMatrix, kernel: Kernel) -> Result<Vec<f64>> {
    if !kernel.needs_norms() {
        return Ok(Vec::new());
    }
    let mut norms = Vec::with_capacity(matrix.rows());
    for row in 0..matrix.rows() {
        let value = norm(matrix.row(row));
        if value <= 0.0 {
            return Err(HnswError::ZeroNorm { row });
        }
        norms.push(value);
    }
    Ok(norms)
}

/// One round of proposals against a frozen snapshot.
///
/// The batch is mapped in parallel with an indexed adaptor, so `collect` restores program
/// order and the flattening below is deterministic regardless of which worker ran which
/// row. The shared [`DistanceCache`] is the only cross-worker state and affects nothing
/// but how often a distance is recomputed.
fn propose_round(round: &Round<'_>, batch: &[usize], levels: &[u32]) -> Result<Vec<Edge>> {
    if round.frozen.entry().is_none() {
        // Round 1: the snapshot is empty, so the batch is the bootstrap seed and links to
        // nothing. Later rounds link *into* these nodes, so they are not stranded.
        return Ok(Vec::new());
    }
    let node_count = round.frozen.node_count();
    let results: Vec<Result<NodeProposal>> = batch
        .par_iter()
        .map_init(
            || Visited::new(node_count),
            |visited, &node| propose_node(round, visited, node, levels[node]),
        )
        .collect();

    let mut edges = Vec::new();
    for (index, result) in results.into_iter().enumerate() {
        let node = batch[index];
        for (layer, selected) in result? {
            for neighbor in selected {
                // Both directions: the proposing node gains the candidate, and the
                // candidate gains the proposer. The merge then reads only inbound
                // proposals, so a link is symmetric at proposal time.
                edges.push(Edge {
                    node,
                    layer,
                    neighbor,
                });
                edges.push(Edge {
                    node: neighbor.row,
                    layer,
                    neighbor: Ranked {
                        distance: neighbor.distance,
                        row: node,
                    },
                });
            }
        }
    }
    Ok(edges)
}

/// The proposed links for one node: per layer, the selected neighbours in rank order.
fn propose_node(
    round: &Round<'_>,
    visited: &mut Visited,
    node: usize,
    node_level: u32,
) -> Result<NodeProposal> {
    let frozen = round.frozen;
    let Some(entry) = frozen.entry() else {
        return Ok(Vec::new());
    };
    let query = Query::new(round.matrix, round.kernel, round.norms, round.cache, node);
    let frozen_top = frozen.max_level();

    // Greedy descent through the snapshot's layers above this node's own level.
    let start = if frozen_top > node_level {
        let (reached, _) = greedy_descend(frozen, &query, entry, frozen_top, node_level + 1)?;
        reached
    } else {
        entry
    };

    let top = node_level.min(frozen_top);
    let mut entry_points = vec![start];
    let mut proposals = Vec::new();
    for layer in (0..=top).rev() {
        let beam = search_layer(
            frozen,
            &query,
            visited,
            &entry_points,
            layer,
            round.params.ef_construction(),
        )?;
        let selected: Vec<Ranked> = beam
            .iter()
            .take(round.params.degree_bound(layer))
            .copied()
            .collect();
        // The next (lower) layer starts from the whole beam, as standard HNSW does; the
        // degree bound applies to what is *linked*, not to what seeds the next search.
        entry_points = beam.iter().map(|ranked| ranked.row).collect();
        proposals.push((layer, selected));
    }
    Ok(proposals)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(rows: usize, dims: usize) -> VectorMatrix {
        let mut state = 0x1234_5678_9abc_def0_u64;
        let mut data = Vec::with_capacity(rows * dims);
        for _ in 0..rows * dims {
            state = crate::level::splitmix64(state);
            // A value in [-1, 1) that is never exactly zero, so cosine norms are positive.
            let value = ((state >> 11) as f64 / (1_u64 << 53) as f64).mul_add(2.0, -1.0);
            data.push(if value == 0.0 { 0.5 } else { value });
        }
        VectorMatrix::new(rows, dims, data).expect("valid fixture")
    }

    #[test]
    fn the_same_input_builds_the_same_graph_twice() {
        let params = Params::new(8, 16, 32, 8).expect("valid");
        let first = build(fixture(64, 4), Kernel::SquaredEuclidean, params).expect("builds");
        let second = build(fixture(64, 4), Kernel::SquaredEuclidean, params).expect("builds");
        assert_eq!(
            first.canonical_image(),
            second.canonical_image(),
            "the build is a pure function of the input"
        );
    }

    #[test]
    fn a_zero_norm_row_is_refused_under_cosine() {
        let matrix = VectorMatrix::new(2, 2, vec![0.0, 0.0, 1.0, 1.0]).expect("valid");
        let params = Params::new(2, 2, 2, 1).expect("valid");
        let error = build(matrix, Kernel::Cosine, params).expect_err("zero norm");
        assert!(matches!(error, HnswError::ZeroNorm { row: 0 }));
    }
}
