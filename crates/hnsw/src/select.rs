// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Neighbour selection: which of a beam's candidates become edges.
//!
//! # Why not simply the nearest `M`
//!
//! Keeping a node's `M` nearest candidates produces a graph that is locally excellent and
//! globally unnavigable. In any corpus with cluster structure the `M` nearest neighbours of
//! a point are usually near each other too, so every edge leaves the node in roughly the
//! same direction and the graph acquires no long-range links at all. Greedy descent then
//! stalls in whichever cluster it entered, and the only way to recover a good answer is to
//! widen `ef` until the beam is a substantial fraction of the corpus — at which point the
//! index is doing more work than the exact scan it exists to avoid.
//!
//! # The relative-neighbourhood condition
//!
//! This is SELECT-NEIGHBORS-HEURISTIC (Malkov & Yashunin, Algorithm 4). Walking the beam in
//! `(distance, row)` order, a candidate `c` is admitted unless some already-chosen `p` is
//! closer to `c` than `c` is to the query:
//!
//! ```text
//! admit c  <=>  for every chosen p:  d(c, p) >= d(c, query)
//! ```
//!
//! Read geometrically, `c` is rejected when it lies in the lune between the query and some
//! `p` already linked — the region where travelling via `p` is a better route to `c` than a
//! direct edge would be. What survives approximates the *relative neighbourhood graph*, a
//! subgraph of the Delaunay graph that keeps greedy routing intact while spending its
//! degree budget on directions rather than on near-duplicates.
//!
//! # Connectivity beats purity
//!
//! The condition can reject enough candidates to leave a node under its degree bound. Rather
//! than ship a sparser graph, the remaining budget is filled with the nearest unchosen
//! candidates. A redundant edge costs a little search work; a missing edge can cost a row
//! its reachability.
//!
//! # Determinism
//!
//! The beam arrives in the shared `(distance, row)` total order, admission is a pure
//! function of the candidates already chosen, and the top-up walks the same order. No
//! comparison depends on iteration order of a hash container and none on thread identity,
//! so the selected set is a function of the beam alone.

use purrdf_sparql_eval::knn::{Bound, Bounded, Kernel, Ranked};

use crate::error::{HnswError, Result};
use crate::graph::VectorMatrix;
use crate::search::norm_of;

/// The subset of `beam` that becomes `node`'s edges, at most `cap` of them.
///
/// `beam` must be in `(distance, row)` order, which is what [`crate::search::search_layer`]
/// returns. The result is returned in that same order.
///
/// No distance memo is taken: within one call each candidate is asked about exactly once,
/// so a cache here only ever misses and inserts.
///
/// # Errors
///
/// [`crate::error::HnswError::NonFiniteDistance`] if a candidate-to-candidate distance left
/// the finite range.
pub(crate) fn select_neighbors(
    beam: &[Ranked],
    cap: usize,
    matrix: &VectorMatrix,
    kernel: Kernel,
    norms: &[f64],
) -> Result<Vec<Ranked>> {
    if cap == 0 || beam.is_empty() {
        return Ok(Vec::new());
    }
    if beam.len() <= cap {
        // Every candidate fits, so the heuristic could only discard edges the degree bound
        // was willing to pay for.
        return Ok(beam.to_vec());
    }

    let mut chosen: Vec<Ranked> = Vec::with_capacity(cap);
    for candidate in beam {
        if chosen.len() == cap {
            break;
        }
        let mut dominated = false;
        for picked in &chosen {
            // The value is never kept -- only whether it falls under the candidate's own
            // distance -- so the kernel may stop as soon as it cannot. Equality already
            // falsifies the test, hence `AtOrAbove`.
            match kernel.distance_bounded(
                matrix.row(candidate.row),
                norm_of(norms, candidate.row),
                matrix.row(picked.row),
                norm_of(norms, picked.row),
                Bound::AtOrAbove(candidate.distance),
            ) {
                Bounded::Below(_) => {
                    dominated = true;
                    break;
                }
                Bounded::Beyond => {}
                Bounded::NonFinite => {
                    return Err(HnswError::NonFiniteDistance { row: picked.row });
                }
            }
        }
        if !dominated {
            chosen.push(*candidate);
        }
    }

    if chosen.len() < cap {
        for candidate in beam {
            if chosen.len() == cap {
                break;
            }
            if !chosen.iter().any(|picked| picked.row == candidate.row) {
                chosen.push(*candidate);
            }
        }
        // The top-up appends in beam order but interleaves with earlier choices, so the
        // rank order has to be restored before the result is stored as adjacency.
        chosen.sort_unstable();
    }

    Ok(chosen)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Explicit points, so a candidate can be far from the query and still add a direction.
    fn plane(points: &[(f64, f64)]) -> VectorMatrix {
        let rows: Vec<Vec<f64>> = points.iter().map(|(x, y)| vec![*x, *y]).collect();
        VectorMatrix::from_rows(&rows).expect("valid")
    }

    /// Points on a line: row `r` sits at `r`. Collinear points are the degenerate case for
    /// the relative-neighbourhood condition — every point is behind its nearer neighbour —
    /// so this fixture exercises the top-up path.
    fn line(rows: usize) -> VectorMatrix {
        let values: Vec<Vec<f64>> = (0..rows).map(|r| vec![r as f64, 0.0]).collect();
        VectorMatrix::from_rows(&values).expect("valid")
    }

    /// The beam a search over `matrix` would hand to selection, in rank order.
    fn beam_over(matrix: &VectorMatrix, query: usize, rows: &[usize]) -> Vec<Ranked> {
        let mut beam: Vec<Ranked> = rows
            .iter()
            .map(|&row| Ranked {
                distance: Kernel::SquaredEuclidean
                    .distance(matrix.row(query), 0.0, matrix.row(row), 0.0)
                    .expect("finite"),
                row,
            })
            .collect();
        beam.sort_unstable();
        beam
    }

    /// Row 0 is the query. Rows 1 and 2 sit almost on top of each other along one axis;
    /// row 3 is three times farther away but in an entirely different direction.
    fn duplicate_and_direction() -> VectorMatrix {
        plane(&[(0.0, 0.0), (1.0, 0.0), (1.1, 0.0), (0.0, 3.0)])
    }

    #[test]
    fn a_beam_within_the_budget_is_kept_whole() {
        let matrix = line(8);
        let beam = beam_over(&matrix, 0, &[1, 2, 3]);
        let chosen =
            select_neighbors(&beam, 4, &matrix, Kernel::SquaredEuclidean, &[]).expect("selects");
        assert_eq!(chosen, beam, "no candidate is discarded under the bound");
    }

    #[test]
    fn a_near_duplicate_loses_its_place_to_a_new_direction() {
        // Row 1 is admitted. Row 2 is then rejected: d(2, 1) = 0.01 is far less than
        // d(2, query) = 1.21, so reaching 2 through 1 is the better route and a direct edge
        // would buy nothing. Row 3 survives — d(3, 1) = 10 exceeds d(3, query) = 9, so no
        // chosen neighbour is a shortcut to it.
        let matrix = duplicate_and_direction();
        let beam = beam_over(&matrix, 0, &[1, 2, 3]);
        let chosen =
            select_neighbors(&beam, 2, &matrix, Kernel::SquaredEuclidean, &[]).expect("selects");
        let rows: Vec<usize> = chosen.iter().map(|pick| pick.row).collect();
        assert_eq!(
            rows,
            vec![1, 3],
            "the far, direction-adding candidate beats the near duplicate"
        );
    }

    #[test]
    fn the_nearest_m_would_have_spent_both_edges_on_one_direction() {
        // The same beam under plain truncation — what this heuristic replaces.
        let matrix = duplicate_and_direction();
        let beam = beam_over(&matrix, 0, &[1, 2, 3]);
        let truncated: Vec<usize> = beam.iter().take(2).map(|pick| pick.row).collect();
        assert_eq!(
            truncated,
            vec![1, 2],
            "both edges leave along the same axis, and the graph gains no reach"
        );
    }

    #[test]
    fn an_over_strict_pass_is_topped_up_to_the_budget() {
        // Collinear candidates: each is behind its nearer neighbour, so the condition alone
        // admits exactly one. Connectivity beats purity, so the budget is filled and the
        // result is restored to rank order.
        let matrix = line(64);
        let beam = beam_over(&matrix, 0, &[30, 31, 32, 33]);
        let chosen =
            select_neighbors(&beam, 3, &matrix, Kernel::SquaredEuclidean, &[]).expect("selects");
        assert_eq!(chosen.len(), 3, "the degree budget is spent, not abandoned");
        let mut sorted = chosen.clone();
        sorted.sort_unstable();
        assert_eq!(chosen, sorted, "the result is in rank order");
    }

    #[test]
    fn selection_is_a_pure_function_of_the_beam() {
        let matrix = line(64);
        let beam = beam_over(&matrix, 5, &[1, 2, 3, 20, 21, 40]);
        let first =
            select_neighbors(&beam, 3, &matrix, Kernel::SquaredEuclidean, &[]).expect("selects");
        let second =
            select_neighbors(&beam, 3, &matrix, Kernel::SquaredEuclidean, &[]).expect("selects");
        assert_eq!(first, second, "selection is a function of the beam alone");
    }
}
