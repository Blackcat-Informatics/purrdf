// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
//!    in this round reads. The entry point is computed from the levels *before* any
//!    linking and is excluded from every batch, so the snapshot is never empty and no
//!    round needs a bootstrap special case.
//! 2. **Propose (parallel).** Each node in the batch runs the standard insertion search
//!    against the snapshot only: greedy descent above its level when the snapshot has
//!    layers there, then an `ef_construction` beam at each layer from its top down to
//!    layer 0. Candidates are the shared `Ranked` order `(distance, row)`, and the
//!    proposal links are emitted in **both directions**.
//! 3. **Merge (canonical).** Every touched node's new neighbourhood is
//!    `frozen neighbours ∪ inbound proposals`, sorted by `(distance, row)` and truncated
//!    to the per-layer degree bound. A set union followed by a total sort is order-free,
//!    so the merge cannot see which worker produced which proposal.
//! 4. **Commit.** The merged adjacency replaces the frozen one. The entry point does not
//!    move: it was fixed before the first round and is a function of the levels alone.
//!
//! A final serial pass repairs connectivity — see [`repair_connectivity`] — because the
//! degree bound in step 3 can drop a node's last inbound edge, and a node nothing points at
//! is a node no search can reach.
//!
//! The loop walks a finite partition of the rows, so termination is structural. There is
//! no `while pending` and no level-based re-queue; a node is proposed exactly once, in its
//! own batch.
//!
//! # Why the batch schedule doubles from one, and why it is capped
//!
//! Nodes within a round cannot link to each other — they all read the same frozen
//! snapshot. A round is therefore a window of mutual invisibility, and its size is the
//! only knob controlling how much of the corpus is invisible to how much else.
//!
//! The schedule starts at a single row and doubles, so the early graph is dense in links
//! rather than sparse: the first proposer links into the entry point, the next two link
//! into those, and so on. A flat schedule of `isqrt(n)` rows would instead leave the whole
//! first batch reading an empty or near-empty snapshot.
//!
//! The doubling is capped at [`MAX_ROUND`] because an uncapped schedule makes the final
//! round half the corpus, and half the corpus mutually invisible is a graph whose recall
//! collapses at scale. The cap trades more rounds for a bounded invisibility window.

use std::collections::BTreeSet;

use rayon::prelude::*;

use purrdf_core::distance::{Arithmetic, Resolved};
use purrdf_sparql_eval::knn::{Kernel, Ranked};

use crate::error::{HnswError, Result};
use crate::graph::{Edge, Graph, VectorMatrix};
use crate::level::{level_cap, level_from_index};
use crate::params::Params;
use crate::search::{DistanceCache, Query, Visited, greedy_descend, norm_of, search_layer};
use crate::select::select_neighbors;
use crate::{Compiled, HnswIndex, resolve_recorded};

/// One node's proposal: per layer, the selected neighbours in rank order.
type NodeProposal = Vec<(u32, Vec<Ranked>)>;

/// The largest number of rows proposed against a single frozen snapshot.
///
/// Rows inside one round cannot link to each other, so the round size is exactly the width
/// of a mutual-invisibility window. Uncapped doubling would make the last round half the
/// corpus; the cap bounds the window at the price of more rounds. It is a structural
/// constant, not a tuning knob: changing it changes the graph, and therefore the artifact.
pub(crate) const MAX_ROUND: usize = 2_048;

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
}

/// Build an index over `matrix` under `kernel` and `params`, with a fixed round size, or
/// the capped doubling schedule when `batch` is `None`.
///
/// Every distance runs under arithmetic `A`, resolved on the calling thread; the path it
/// resolves is the one the image records. The graph is built by `compiled`'s build walk,
/// the copy this crate compiled for `A`, which is also the copy every later rebuild
/// verification runs.
///
/// `batch = Some(1)` is a plain serial insertion: each node proposes against the graph
/// that already holds every row below it. It exists so the determinism suite can observe
/// that the round structure is load-bearing — within-round isolation produces a different
/// graph — rather than asserting the property against a second identical build.
///
/// # Errors
///
/// [`HnswError::InvalidParameter`] / [`HnswError::ParameterValidation`] for a parameter or
/// matrix problem caught before any work; [`HnswError::ZeroNorm`] when `kernel` divides by
/// a norm and a row's is zero; [`HnswError::NonFiniteDistance`] if a kernel result leaves
/// the finite range; [`HnswError::FloatEnvironment`] if the calling thread, or a worker
/// thread the build runs on, is not in the IEEE environment the arithmetic defines.
pub(crate) fn build_with_batch<A: Arithmetic>(
    matrix: VectorMatrix,
    kernel: Kernel,
    params: Params,
    batch: Option<usize>,
    compiled: Compiled<A>,
) -> Result<HnswIndex<A>> {
    let arithmetic = A::resolve()?;
    let (graph, norms) = (compiled.build_graph)(&matrix, arithmetic, kernel, params, batch)?;
    Ok(HnswIndex::new(
        matrix, kernel, params, graph, norms, arithmetic, compiled,
    ))
}

/// The graph and per-row norms for `matrix`, without taking ownership of it.
///
/// The build reads the matrix through a shared reference throughout; ownership is needed
/// only to hand it to the finished index. Separating the two lets a caller that already
/// holds the vectors -- `verify_rebuild`, which rebuilds in order to compare -- avoid
/// copying them. At a million rows of 4,096 `f64` that copy is over thirty gigabytes, and
/// the guard's verification path was paying it twice.
///
/// Every distance runs under `arithmetic`, resolved on the calling thread; each worker of
/// the parallel proposal phase resolves again on its own thread, because the float
/// environment is a per-thread property, and must resolve the same dispatch path, because
/// the image records one.
pub(crate) fn build_graph<A: Arithmetic>(
    matrix: &VectorMatrix,
    arithmetic: Resolved<A>,
    kernel: Kernel,
    params: Params,
    batch: Option<usize>,
) -> Result<(Graph, Vec<f64>)> {
    params.validate_against(matrix.rows(), matrix.dims())?;

    let n = matrix.rows();
    let cap = level_cap(n, params.m());
    let levels: Vec<u32> = (0..n)
        .map(|row| level_from_index(row as u64, params.m(), cap))
        .collect();
    let norms = compute_norms(matrix, kernel)?;

    let mut graph = Graph::with_levels(levels.clone());

    // The entry point is a pure function of the levels, so it is known before any link
    // exists. Fixing it here is what removes the bootstrap case: every proposal, including
    // the very first, searches a snapshot that already holds a reachable node, and
    // `search_layer` seeds its beam with the entry points themselves. The rule is the
    // minimum row at the maximum level, so the tie-break comparator reverses the row order.
    let entry = (0..n)
        .max_by(|a, b| levels[*a].cmp(&levels[*b]).then_with(|| b.cmp(a)))
        .ok_or_else(|| HnswError::ParameterValidation {
            description: "a matrix with no rows has no entry point".to_owned(),
        })?;
    graph.promote_entry(entry, levels[entry]);

    let mut start = 0;
    while start < n {
        // An explicit span is the serial-insert comparison; otherwise the schedule doubles
        // from one and is capped, per the module docs.
        let span = match batch {
            Some(fixed) => fixed.max(1),
            None if start == 0 => 1,
            None => start.min(n - start).min(MAX_ROUND),
        };
        let end = (start + span).min(n);
        // The entry point is never proposed: it is the node every other row links into.
        let batch_rows: Vec<usize> = (start..end).filter(|row| *row != entry).collect();
        if !batch_rows.is_empty() {
            let round = Round {
                frozen: &graph,
                matrix,
                kernel,
                norms: &norms,
                params: &params,
            };
            let edges = propose_round(&round, arithmetic, &batch_rows, &levels)?;
            graph.commit(edges, &params);
        }
        start = end;
    }

    repair_connectivity(
        &mut graph, matrix, arithmetic, kernel, &norms, &params, entry,
    )?;

    Ok((graph, norms))
}

/// Close the reachability gap the degree bound can open.
///
/// Every link is proposed in both directions, but a merge already at its degree bound keeps
/// only the nearest entries, so a node whose every chosen neighbour is saturated can finish
/// the build holding out-edges and no inbound edge at all. Out-edges do not make a row
/// findable: the beam arrives only by following an inbound edge. Such a row is unreachable
/// from the entry point and can never be returned — not by a neighbour query, and not even
/// as its own nearest neighbour at distance zero.
///
/// The rate is a function of sparsity rather than of corpus size: measured on a uniform
/// fixture it is flat near 3% of rows at `M0 = 8` from 250 rows to 5,000, and total at
/// `M0 = 2`, where greedy nearest selection cannot find a spanning structure even though
/// the degree budget admits one.
///
/// # The repair
///
/// Each pass walks the unreachable rows in ascending order and gives each one an inbound
/// edge from the nearest node that is *already reachable* — preferring one the orphan
/// itself points at, since adjacency is held in rank order and the first reachable entry is
/// therefore the nearest. That edge is then **protected**: no later eviction may remove it.
///
/// Termination: a protected edge is never removed, every pass converts at least one row
/// from unreachable to reachable, and total protected capacity (`n * bound`) exceeds the
/// `n - 1` edges a spanning structure needs, so a host with room always exists. The loop
/// therefore runs at most `n` passes and cannot oscillate.
///
/// Determinism: the orphan order, the host preference, and the eviction choice are all
/// total functions of the graph, so the repaired graph is a pure function of the build.
fn repair_connectivity<A: Arithmetic>(
    graph: &mut Graph,
    matrix: &VectorMatrix,
    arithmetic: Resolved<A>,
    kernel: Kernel,
    norms: &[f64],
    params: &Params,
    entry: usize,
) -> Result<()> {
    let n = graph.node_count();
    let bound = params.degree_bound(0);
    let mut protected: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); n];

    for _ in 0..n {
        let mut reachable = graph.reachable_from(entry, 0);
        let orphans: Vec<usize> = (0..n).filter(|row| !reachable[*row]).collect();
        if orphans.is_empty() {
            return Ok(());
        }
        for orphan in orphans {
            // Attaching an orphan makes it, and everything it already points at, a legal
            // host for the orphans still to come in this pass. Holding the reachable set
            // frozen for the whole pass would exhaust the hosts a sparse graph can offer.
            if reachable[orphan] {
                continue;
            }
            let host = choose_host(graph, &reachable, &protected, bound, entry, orphan)?;
            let distance = matrix
                .distance_with(
                    arithmetic,
                    kernel,
                    host,
                    norm_of(norms, host),
                    orphan,
                    norm_of(norms, orphan),
                )
                .ok_or(HnswError::NonFiniteDistance { row: orphan })?;
            let linked = graph.link_protected(
                host,
                0,
                Ranked {
                    distance,
                    row: orphan,
                },
                bound,
                &protected[host],
            );
            debug_assert!(
                linked,
                "choose_host only returns a host with unprotected room"
            );
            protected[host].insert(orphan);
            mark_reachable(graph, &mut reachable, orphan);
        }
    }
    Err(HnswError::ParameterValidation {
        description: "connectivity repair did not converge; the degree bound admits no \
                      spanning structure over this corpus"
            .to_owned(),
    })
}

/// Mark `from` reachable, and with it everything its out-edges lead to.
fn mark_reachable(graph: &Graph, reachable: &mut [bool], from: usize) {
    if reachable[from] {
        return;
    }
    reachable[from] = true;
    let mut frontier = vec![from];
    while let Some(row) = frontier.pop() {
        for neighbor in graph.neighbors(row, 0) {
            if !reachable[neighbor.row] {
                reachable[neighbor.row] = true;
                frontier.push(neighbor.row);
            }
        }
    }
}

/// The reachable node that will adopt `orphan`, preferring the nearest one it points at.
fn choose_host(
    graph: &Graph,
    reachable: &[bool],
    protected: &[BTreeSet<usize>],
    bound: usize,
    entry: usize,
    orphan: usize,
) -> Result<usize> {
    let has_room = |row: usize| protected[row].len() < bound;
    // Adjacency is kept in rank order, so the first reachable entry is the nearest one.
    let nearest = graph
        .neighbors(orphan, 0)
        .iter()
        .map(|candidate| candidate.row)
        .find(|row| reachable[*row] && has_room(*row));
    if let Some(row) = nearest {
        return Ok(row);
    }
    if reachable[entry] && has_room(entry) {
        return Ok(entry);
    }
    (0..graph.node_count())
        .find(|row| reachable[*row] && has_room(*row))
        .ok_or_else(|| HnswError::ParameterValidation {
            description: format!(
                "row {orphan} cannot be reattached: every reachable node has exhausted its \
                 degree bound of {bound}"
            ),
        })
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
        let value = matrix.norm_of_row(row);
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
fn propose_round<A: Arithmetic>(
    round: &Round<'_>,
    arithmetic: Resolved<A>,
    batch: &[usize],
    levels: &[u32],
) -> Result<Vec<Edge>> {
    let node_count = round.frozen.node_count();
    let recorded = arithmetic.image_code();
    let results: Vec<Result<NodeProposal>> = batch
        .par_iter()
        .map_init(
            // Resolved per worker: the float environment belongs to the thread that runs
            // the proposal, not to the one that started the build. The worker must land on
            // the path the build resolved, since the image records that one.
            || (Visited::new(node_count), resolve_recorded::<A>(recorded)),
            |(visited, arithmetic), &node| {
                let arithmetic = arithmetic.clone()?;
                propose_node(round, arithmetic, visited, node, levels[node])
            },
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
fn propose_node<A: Arithmetic>(
    round: &Round<'_>,
    arithmetic: Resolved<A>,
    visited: &mut Visited,
    node: usize,
    node_level: u32,
) -> Result<NodeProposal> {
    let frozen = round.frozen;
    let Some(entry) = frozen.entry() else {
        return Ok(Vec::new());
    };
    // One memo per proposal. It is worth having -- a node's beam re-scores rows it has
    // already seen at the layer above -- and worth bounding: two nodes in a batch almost
    // never need the same pair, so a shared cache buys little and costs a lock in the
    // innermost loop of the parallel phase.
    let cache = DistanceCache::new();
    let query = Query::new(
        round.matrix,
        round.kernel,
        arithmetic,
        round.norms,
        &cache,
        node,
    );
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
        let selected = select_neighbors(
            &beam,
            round.params.degree_bound(layer),
            round.matrix,
            arithmetic,
            round.kernel,
            round.norms,
        )?;
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
    use purrdf_core::distance::Exact;

    fn build(matrix: VectorMatrix, kernel: Kernel, params: Params) -> Result<HnswIndex> {
        build_with_batch(matrix, kernel, params, None, Compiled::<Exact>::here())
    }

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
