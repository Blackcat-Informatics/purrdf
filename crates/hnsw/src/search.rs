// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Beam search with `ef`, and the scratch state that keeps it allocation-free.
//!
//! The traversal is the standard HNSW two-heap search: a **candidate** min-heap ordered
//! by [`Ranked`] (nearest first) and a **result** max-heap bounded at `ef` (the worst
//! retained entry on top). It stops when the nearest unexplored candidate is farther than
//! the worst result, which is the point at which no unexpanded path can improve the beam.
//!
//! Three pieces of scratch state make the search deterministic and cheap:
//!
//! * [`Visited`] is a generation-stamped `Vec<u32>`: a node is "visited" when its stamp
//!   equals the current generation, so starting a query is one increment and clearing it
//!   is not an `O(n)` fill. The generation is per search, and a per-thread instance is
//!   reused across the queries a rayon worker runs.
//! * [`DistanceCache`] memoizes `d(a, b)` for the round. The graph's answer cannot depend
//!   on it — a cache hit and a recomputation return the same bits — so it is
//!   digest-neutral by construction; it exists only to stop the build evaluating a pair
//!   it already knows.
//! * [`Query`] binds the query row, its norm, and the cache into the one closure every
//!   distance in a search goes through, so no call site can accidentally rank by a
//!   differently-computed number.

use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::collections::BinaryHeap;
use std::sync::Mutex;

use purrdf_sparql_eval::knn::{Kernel, Ranked};

use crate::error::{HnswError, Result};
use crate::graph::{Graph, VectorMatrix};

/// Generation-stamped visited tracking: `O(1)` clear, no per-query allocation when reused.
#[derive(Debug)]
pub(crate) struct Visited {
    stamps: Vec<u32>,
    generation: u32,
}

impl Visited {
    /// Scratch sized for a graph of `nodes` rows.
    pub(crate) fn new(nodes: usize) -> Self {
        Self {
            stamps: vec![0; nodes],
            generation: 0,
        }
    }

    /// Start a fresh search, invalidating every prior stamp.
    ///
    /// The generation wraps after `u32::MAX` searches; at the wrap the stamps are
    /// cleared before the new generation is installed, so a stale zero stamp cannot be
    /// mistaken for the current generation.
    fn begin(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        if self.generation == 0 {
            self.stamps.fill(0);
            self.generation = 1;
        }
    }

    /// Mark `row` visited, returning `true` the first time it is seen this generation.
    fn visit(&mut self, row: usize) -> bool {
        if self.stamps[row] == self.generation {
            false
        } else {
            self.stamps[row] = self.generation;
            true
        }
    }
}

/// The memoized state of one [`DistanceCache`]: the stored distances and how many
/// were actually evaluated.
#[derive(Debug, Default)]
struct CacheState {
    /// `(min(a, b), max(a, b)) -> distance`.
    entries: BTreeMap<(usize, usize), f64>,
    /// Kernel evaluations that actually ran, as opposed to cache hits.
    ///
    /// Incremented where the kernel is invoked and nowhere else, so it is exactly the
    /// quantity a work-accounting caller wants: `d(a, b)` computed once counts once,
    /// and a cache hit counts zero. A concurrent duplicate (two workers racing the same
    /// pair) is two real evaluations and is counted twice, which is the honest number.
    evaluations: u64,
}

/// A memoized `(row_a, row_b) -> distance` map shared by one build round.
///
/// A `BTreeMap` under a mutex rather than a hash map: the key is an integer pair, the map
/// is consulted in a hot loop, and a `BTreeMap` needs no hasher choice at all, which keeps
/// the determinism argument free of "which `BuildHasher`" entirely. The lock is released
/// around the distance computation, so a concurrent duplicate is possible and harmless —
/// both computations produce the same bits.
#[derive(Debug, Default)]
pub(crate) struct DistanceCache {
    state: Mutex<CacheState>,
}

impl DistanceCache {
    /// An empty cache.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// A stored distance, keyed by the unordered pair.
    fn get(&self, a: usize, b: usize) -> Option<f64> {
        self.state
            .lock()
            .expect("distance cache lock is never poisoned")
            .entries
            .get(&ordered(a, b))
            .copied()
    }

    /// Record a distance and charge it as one evaluation.
    fn insert(&self, a: usize, b: usize, distance: f64) {
        let mut state = self
            .state
            .lock()
            .expect("distance cache lock is never poisoned");
        state.entries.insert(ordered(a, b), distance);
        state.evaluations = state.evaluations.saturating_add(1);
    }

    /// The number of kernel evaluations performed through this cache.
    ///
    /// A cache hit does not increment it: the value was not evaluated again, merely
    /// recalled. Callers that want a delta (a reused cache across queries) subtract two
    /// reads; callers that drained a fresh cache read it directly.
    pub(crate) fn evaluations(&self) -> u64 {
        self.state
            .lock()
            .expect("distance cache lock is never poisoned")
            .evaluations
    }
}

/// Order a pair so `d(a, b)` and `d(b, a)` share one key.
const fn ordered(a: usize, b: usize) -> (usize, usize) {
    if a <= b { (a, b) } else { (b, a) }
}

/// A search's query vector bound to the matrix, kernel, and cache it reads.
pub(crate) struct Query<'a> {
    matrix: &'a VectorMatrix,
    kernel: Kernel,
    norms: &'a [f64],
    cache: &'a DistanceCache,
    row: usize,
}

impl<'a> Query<'a> {
    /// Bind `query_row` as the search's seed.
    pub(crate) fn new(
        matrix: &'a VectorMatrix,
        kernel: Kernel,
        norms: &'a [f64],
        cache: &'a DistanceCache,
        query_row: usize,
    ) -> Self {
        Self {
            matrix,
            kernel,
            norms,
            cache,
            row: query_row,
        }
    }

    /// The distance from the query to `row`, memoized.
    ///
    /// # Errors
    ///
    /// [`HnswError::NonFiniteDistance`] if the kernel's result left the finite range. A
    /// distance that overflowed would still sort — last, confidently — so it is refused
    /// rather than ranked.
    pub(crate) fn of(&self, row: usize) -> Result<f64> {
        pair_distance(
            self.matrix,
            self.kernel,
            self.norms,
            self.cache,
            self.row,
            row,
        )
    }

    /// The cached distance from the query, or `None` if it has not been computed.
    #[cfg(test)]
    pub(crate) fn cached(&self, row: usize) -> Option<f64> {
        self.cache.get(self.row, row)
    }
}

/// The L2 norm of `row`, or `0.0` for a kernel that does not divide by one.
pub(crate) fn norm_of(norms: &[f64], row: usize) -> f64 {
    norms.get(row).copied().unwrap_or(0.0)
}

/// The memoized distance between any two rows.
///
/// A search ranks everything against one query row, but neighbour selection also asks how
/// far two *candidates* are from each other, so the pair form is the general one and
/// [`Query::of`] is the case where one endpoint is fixed. Both go through the same cache
/// and the same kernel, so a distance cannot be computed two ways.
///
/// # Errors
///
/// [`HnswError::NonFiniteDistance`] if the kernel's result left the finite range. A distance
/// that overflowed would still sort — last, confidently — so it is refused rather than
/// ranked.
pub(crate) fn pair_distance(
    matrix: &VectorMatrix,
    kernel: Kernel,
    norms: &[f64],
    cache: &DistanceCache,
    a: usize,
    b: usize,
) -> Result<f64> {
    if let Some(distance) = cache.get(a, b) {
        return Ok(distance);
    }
    let distance = kernel
        .distance(
            matrix.row(a),
            norm_of(norms, a),
            matrix.row(b),
            norm_of(norms, b),
        )
        .ok_or(HnswError::NonFiniteDistance { row: b })?;
    cache.insert(a, b, distance);
    Ok(distance)
}

/// Greedy descent through layers `from_layer..=to_layer`, nearest-neighbour at each.
///
/// Returns the node reached and its distance. Each layer is hill-climbed until no
/// neighbour improves, which is the `ef = 1` special case of the beam. `from_layer` must
/// not be below `to_layer`; if it is, the entry is returned unchanged.
pub(crate) fn greedy_descend(
    graph: &Graph,
    query: &Query<'_>,
    entry: usize,
    from_layer: u32,
    to_layer: u32,
) -> Result<(usize, f64)> {
    let mut current = entry;
    let mut current_distance = query.of(current)?;
    if from_layer < to_layer {
        return Ok((current, current_distance));
    }
    for layer in (to_layer..=from_layer).rev() {
        loop {
            let mut improved = false;
            for neighbor in graph.neighbors(current, layer) {
                let distance = query.of(neighbor.row)?;
                let candidate = Ranked {
                    distance,
                    row: neighbor.row,
                };
                if candidate
                    < (Ranked {
                        distance: current_distance,
                        row: current,
                    })
                {
                    current = neighbor.row;
                    current_distance = distance;
                    improved = true;
                }
            }
            if !improved {
                break;
            }
        }
    }
    Ok((current, current_distance))
}

/// The `ef` nearest nodes to `query` at `layer`, in rank order.
///
/// `entry_points` must all exist at `layer`. The returned vector is strictly sorted by
/// [`Ranked`] (`(distance, row)`), so its first `n` entries are the `n` nearest of the
/// explored set for every `n`.
pub(crate) fn search_layer(
    graph: &Graph,
    query: &Query<'_>,
    visited: &mut Visited,
    entry_points: &[usize],
    layer: u32,
    ef: usize,
) -> Result<Vec<Ranked>> {
    visited.begin();
    let mut candidates: BinaryHeap<Reverse<Ranked>> = BinaryHeap::new();
    let mut results: BinaryHeap<Ranked> = BinaryHeap::new();

    for &entry in entry_points {
        if !visited.visit(entry) {
            continue;
        }
        let score = Ranked {
            distance: query.of(entry)?,
            row: entry,
        };
        candidates.push(Reverse(score));
        results.push(score);
        if results.len() > ef {
            results.pop();
        }
    }

    while let Some(Reverse(current)) = candidates.pop() {
        if results.len() >= ef {
            // The nearest unexplored candidate is already worse than the worst result, so
            // no expansion can improve the beam.
            if results.peek().is_some_and(|worst| current > *worst) {
                break;
            }
        }
        for neighbor in graph.neighbors(current.row, layer) {
            if !visited.visit(neighbor.row) {
                continue;
            }
            let score = Ranked {
                distance: query.of(neighbor.row)?,
                row: neighbor.row,
            };
            let admitted = results.len() < ef || results.peek().is_some_and(|worst| score < *worst);
            if admitted {
                candidates.push(Reverse(score));
                results.push(score);
                if results.len() > ef {
                    results.pop();
                }
            }
        }
    }

    Ok(results.into_sorted_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::Params;

    /// A small hand-built graph on a line: row `r` sits at `r` on the number line.
    fn line_graph(degrees: &[usize]) -> (VectorMatrix, Graph) {
        let matrix = {
            let rows: Vec<Vec<f64>> = (0..degrees.len()).map(|r| vec![r as f64]).collect();
            VectorMatrix::from_rows(&rows).expect("valid matrix")
        };
        let mut graph = Graph::with_levels(vec![0; degrees.len()]);
        let mut edges = Vec::new();
        for (row, &degree) in degrees.iter().enumerate() {
            for step in 1..=degree {
                if row + step < degrees.len() {
                    edges.push(crate::graph::Edge {
                        node: row,
                        layer: 0,
                        neighbor: Ranked {
                            distance: step as f64,
                            row: row + step,
                        },
                    });
                }
            }
        }
        graph.commit(edges, &Params::new(2, 2, 2, 1).expect("valid"));
        graph.promote_entry(0, 0);
        (matrix, graph)
    }

    #[test]
    fn visited_stamps_clear_on_a_new_generation() {
        let mut visited = Visited::new(4);
        visited.begin();
        assert!(visited.visit(2));
        assert!(!visited.visit(2));
        visited.begin();
        assert!(visited.visit(2), "a new generation clears the stamp");
    }

    #[test]
    fn a_greedy_descent_walks_toward_the_query() {
        let (matrix, graph) = line_graph(&[2, 2, 2, 2]);
        let cache = DistanceCache::new();
        let query = Query::new(&matrix, Kernel::SquaredEuclidean, &[], &cache, 3);
        let (node, distance) = greedy_descend(&graph, &query, 0, 0, 0).expect("descends");
        assert_eq!(node, 3, "it should reach the query's own row");
        assert_eq!(distance, 0.0);
    }

    #[test]
    fn the_beam_returns_the_nearest_first() {
        let (matrix, graph) = line_graph(&[3, 3, 3, 3, 3, 3]);
        let cache = DistanceCache::new();
        let query = Query::new(&matrix, Kernel::SquaredEuclidean, &[], &cache, 0);
        let mut visited = Visited::new(6);
        let result = search_layer(&graph, &query, &mut visited, &[0], 0, 3).expect("searches");
        let rows: Vec<usize> = result.iter().map(|ranked| ranked.row).collect();
        assert_eq!(rows, vec![0, 1, 2], "nearest-first, by distance then row");
    }

    #[test]
    fn the_distance_cache_is_value_neutral() {
        let (matrix, _graph) = line_graph(&[2, 2, 2]);
        let cache = DistanceCache::new();
        let query = Query::new(&matrix, Kernel::SquaredEuclidean, &[], &cache, 0);
        let first = query.of(2).expect("finite");
        assert_eq!(query.cached(2), Some(first));
        let second = query.of(2).expect("finite");
        assert_eq!(
            first.to_bits(),
            second.to_bits(),
            "a hit returns the same bits"
        );
        assert_eq!(query.of(1).expect("finite"), query.of(1).expect("finite"));
    }
}
