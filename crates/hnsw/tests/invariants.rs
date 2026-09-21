// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The graph-invariant suite.
//!
//! Every property the build promises is asserted here against real graphs — several
//! parameter sets and both a norm-free and a norm-dividing kernel — rather than against a
//! mock. The fixture vectors are generated deterministically at test time from a
//! splitmix64 stream, so the suite carries no committed binaries and produces the same
//! graphs on every target.

use purrdf_core::DistanceMetric;
#[path = "support/corpus.rs"]
mod corpus;

use corpus::CorpusShape;
use purrdf_hnsw::level::{level_cap, level_from_index};
use purrdf_hnsw::{HnswIndex, Params, Ranked, VectorMatrix, build};

/// A deterministic matrix of `rows x dims` values in `[-1, 1)`, never exactly zero.
fn fixture(rows: usize, dims: usize, seed: u64) -> VectorMatrix {
    let mut state = seed;
    let mut data = Vec::with_capacity(rows * dims);
    for _ in 0..rows * dims {
        state = purrdf_hnsw::level::splitmix64(state);
        let value = ((state >> 11) as f64 / (1_u64 << 53) as f64).mul_add(2.0, -1.0);
        data.push(if value == 0.0 { 0.125 } else { value });
    }
    VectorMatrix::new(rows, dims, data).expect("fixture is valid")
}

/// The parameter sets every invariant is checked under.
fn parameter_sets() -> Vec<Params> {
    vec![
        Params::new(2, 2, 2, 1).expect("minimum"),
        Params::new(4, 8, 16, 8).expect("modest"),
        Params::new(16, 32, 64, 16).expect("wide"),
    ]
}

fn kernels() -> [DistanceMetric; 3] {
    [
        DistanceMetric::SquaredEuclidean,
        DistanceMetric::NegativeDot,
        DistanceMetric::Cosine,
    ]
}

/// Walk every node and layer of `index`, handing `(row, layer, neighbors)` to `check`.
fn for_each_layer(index: &HnswIndex, mut check: impl FnMut(usize, u32, &[Ranked])) {
    for row in 0..index.rows() {
        for layer in 0..=index.level(row) {
            check(row, layer, index.neighbors(row, layer));
        }
    }
}

#[test]
fn no_node_links_to_itself_at_any_layer() {
    for params in parameter_sets() {
        for metric in kernels() {
            let index = build(fixture(96, 8, 0xA11CE), &metric, params).expect("builds");
            for_each_layer(&index, |row, layer, neighbors| {
                for neighbor in neighbors {
                    assert_ne!(
                        neighbor.row, row,
                        "{metric:?} {params:?}: row {row} self-links at layer {layer}"
                    );
                }
            });
        }
    }
}

#[test]
fn every_neighbor_is_a_valid_row_at_a_layer_that_row_occupies() {
    for params in parameter_sets() {
        let index = build(
            fixture(96, 8, 0xB0B),
            &DistanceMetric::SquaredEuclidean,
            params,
        )
        .expect("builds");
        for_each_layer(&index, |row, layer, neighbors| {
            for neighbor in neighbors {
                assert!(
                    neighbor.row < index.rows(),
                    "row {row} layer {layer} names row {} beyond {} rows",
                    neighbor.row,
                    index.rows()
                );
                assert!(
                    index.level(neighbor.row) >= layer,
                    "row {row} layer {layer} links to row {}, which only reaches level {}",
                    neighbor.row,
                    index.level(neighbor.row)
                );
            }
        });
    }
}

#[test]
fn adjacency_is_strictly_sorted_by_distance_then_row() {
    for params in parameter_sets() {
        for metric in kernels() {
            let index = build(fixture(96, 8, 0x00C0_FFEE), &metric, params).expect("builds");
            for_each_layer(&index, |row, layer, neighbors| {
                for pair in neighbors.windows(2) {
                    assert!(
                        pair[0] < pair[1],
                        "{metric:?} {params:?}: row {row} layer {layer} is unsorted: \
                         {:?} then {:?}",
                        pair[0],
                        pair[1]
                    );
                }
            });
        }
    }
}

#[test]
fn degree_bounds_hold_at_every_layer() {
    for params in parameter_sets() {
        let index = build(
            fixture(128, 6, 0xD00D),
            &DistanceMetric::SquaredEuclidean,
            params,
        )
        .expect("builds");
        for_each_layer(&index, |row, layer, neighbors| {
            let bound = params.degree_bound(layer);
            assert!(
                neighbors.len() <= bound,
                "row {row} layer {layer} holds {} neighbours, over the bound {bound}",
                neighbors.len()
            );
        });
    }
}

#[test]
fn the_entry_point_is_the_minimum_row_at_the_maximum_level() {
    for params in parameter_sets() {
        let index = build(
            fixture(128, 6, 0xE11E),
            &DistanceMetric::SquaredEuclidean,
            params,
        )
        .expect("builds");
        let max_level = index.max_level();
        let expected = (0..index.rows())
            .filter(|&row| index.level(row) == max_level)
            .min();
        assert!(
            expected.is_some(),
            "some node must attain the maximum level"
        );
        assert_eq!(
            index.entry(),
            expected,
            "the entry point is pinned to the minimum row at the maximum level"
        );
    }
}

#[test]
fn levels_are_exactly_the_fixed_formula() {
    for params in parameter_sets() {
        let index = build(
            fixture(256, 4, 0xF00D),
            &DistanceMetric::SquaredEuclidean,
            params,
        )
        .expect("builds");
        let cap = level_cap(index.rows(), params.m());
        for row in 0..index.rows() {
            assert_eq!(
                index.level(row),
                level_from_index(row as u64, params.m(), cap),
                "row {row} does not carry the fixed-formula level"
            );
        }
    }
}

#[test]
fn the_canonical_image_round_trips_through_decode_byte_for_byte() {
    for params in parameter_sets() {
        for metric in kernels() {
            let matrix = fixture(64, 8, 0x5EED);
            let index = build(matrix.clone(), &metric, params).expect("builds");
            let image = index.canonical_image();
            let decoded = HnswIndex::decode(matrix, &image).expect("decodes");
            assert_eq!(
                decoded.canonical_image(),
                image,
                "{metric:?} {params:?}: decode then encode changed the bytes"
            );
        }
    }
}

#[test]
fn an_index_is_a_pure_function_of_its_inputs() {
    // The determinism claim, at the level a graph invariant can state it: the whole build
    // is reproduced by a second call, including the entry point and every layer.
    for params in parameter_sets() {
        for metric in kernels() {
            let first = build(fixture(200, 8, 0x1DEA), &metric, params).expect("builds");
            let second = build(fixture(200, 8, 0x1DEA), &metric, params).expect("builds");
            assert_eq!(first.canonical_image(), second.canonical_image());
            assert!(first.verify_rebuild().expect("rebuilds"));
        }
    }
}

#[test]
fn a_single_row_index_is_the_degenerate_bootstrap() {
    let matrix = VectorMatrix::new(1, 2, vec![1.0, 2.0]).expect("valid");
    let index = HnswIndex::build(
        matrix,
        &DistanceMetric::SquaredEuclidean,
        Params::new(2, 2, 2, 1).expect("valid"),
    )
    .expect("builds");
    assert_eq!(index.rows(), 1);
    assert_eq!(index.level(0), level_from_index(0, 2, level_cap(1, 2)));
    assert_eq!(index.entry(), Some(0));
    assert_eq!(
        index.neighbors(0, 0),
        [] as [Ranked; 0],
        "the lone node links to nothing"
    );
    assert_eq!(index.search_rows(0, 5).expect("searches").len(), 1);
}

/// The rows a layer-0 walk from the entry point can actually arrive at.
///
/// Search only ever reaches a node by following an *inbound* edge from a node it has
/// already reached, so out-edges do not make a row findable. This is a plain BFS over the
/// same adjacency the beam walks.
fn reachable_at_layer_zero(index: &HnswIndex) -> Vec<bool> {
    let mut seen = vec![false; index.rows()];
    let Some(entry) = index.entry() else {
        return seen;
    };
    seen[entry] = true;
    let mut frontier = vec![entry];
    while let Some(row) = frontier.pop() {
        for neighbor in index.neighbors(row, 0) {
            if !seen[neighbor.row] {
                seen[neighbor.row] = true;
                frontier.push(neighbor.row);
            }
        }
    }
    seen
}

#[test]
fn every_row_is_reachable_from_the_entry_point_at_layer_zero() {
    // A row the entry cannot reach can never be returned -- not by a neighbour query, and
    // not even as its own nearest neighbour at distance zero. It is a silent drop, and it is
    // invisible to every determinism check because a deterministically wrong graph is still
    // deterministic.
    //
    // The corpus families matter as much as the sizes. Uniform i.i.d. vectors are the
    // EASIEST case for reachability: with no cluster structure, a node's nearest neighbours
    // point in every direction and the graph is naturally well connected. Stranding is a
    // clustering phenomenon -- nearest-M truncation inside a tight cluster produces edges
    // that never leave it -- so an invariant tested only on uniform data is testing where
    // the defect cannot appear. These run over the structured generator and over two
    // deliberately adversarial geometries as well.
    for params in parameter_sets() {
        for metric in kernels() {
            for rows in [2_usize, 3, 17, 96, 250] {
                assert_reachable(
                    &fixture(rows, 8, 0x00C0_FFEE),
                    &metric,
                    params,
                    &format!("uniform-{rows}x8"),
                );
            }
            // Cluster structure, which is where truncation strands rows.
            assert_reachable(
                &corpus::embedding_like(CorpusShape::embedding_like(250, 16), 0xC1_0057)
                    .expect("generates"),
                &metric,
                params,
                "embedding-like-250x16",
            );
            // Locally dense and globally disconnected: a greedy descent that enters the
            // wrong manifold has no downhill path to the right one.
            assert_reachable(
                &corpus::separated_manifolds(240, 24, 6, 0x5EA4_A7ED).expect("generates"),
                &metric,
                params,
                "separated-manifolds-240x24",
            );
            // Signed zeros and huge-but-finite magnitudes, which stress the comparator
            // rather than the geometry.
            assert_reachable(
                &corpus::extreme_but_finite(160, 12, 0xE547_3E4E).expect("generates"),
                &metric,
                params,
                "extreme-160x12",
            );
        }
    }
}

#[test]
fn reachability_holds_at_the_scale_the_crate_claims() {
    // The size the crate's own evidence quotes. Kept to one parameter set and one kernel so
    // the conformance lane stays affordable; the cross-product above covers the variety.
    let params = Params::new(16, 32, 64, 16).expect("wide");
    for rows in [1_000_usize, 5_000] {
        assert_reachable(
            &fixture(rows, 8, 0x00C0_FFEE),
            &DistanceMetric::SquaredEuclidean,
            params,
            &format!("uniform-{rows}x8"),
        );
    }
    assert_reachable(
        &corpus::embedding_like(CorpusShape::embedding_like(2_000, 16), 0x5CA1_AB1E)
            .expect("generates"),
        &DistanceMetric::SquaredEuclidean,
        params,
        "embedding-like-2000x16",
    );
}

/// Build `matrix` and assert every row is reachable from the entry point at layer 0.
fn assert_reachable(matrix: &VectorMatrix, metric: &DistanceMetric, params: Params, name: &str) {
    let index = build(matrix.clone(), metric, params).expect("builds");
    let seen = reachable_at_layer_zero(&index);
    let stranded: Vec<usize> = (0..index.rows()).filter(|row| !seen[*row]).collect();
    assert!(
        stranded.is_empty(),
        "{metric:?} {params:?} {name}: {} of {} rows are unreachable from entry {:?}: {:?}",
        stranded.len(),
        index.rows(),
        index.entry(),
        &stranded[..stranded.len().min(12)]
    );
}

#[test]
fn asymmetric_adjacency_is_explained_by_the_degree_bound() {
    // Links are proposed in both directions, so an edge `a -> b` without `b -> a` can only
    // come from `b`'s merge truncating at its degree bound. Anything else is a lost edge.
    //
    // The retained set is deliberately *not* asserted to be uniformly nearer than the
    // dropped edge. The connectivity repair may hold a far neighbour on purpose — that is
    // the whole point of a protected edge, and connectivity beats purity.
    for params in parameter_sets() {
        for metric in kernels() {
            let index = build(fixture(96, 8, 0xBEEF), &metric, params).expect("builds");
            for_each_layer(&index, |row, layer, neighbors| {
                for neighbor in neighbors {
                    let back = index.neighbors(neighbor.row, layer);
                    if back.iter().any(|entry| entry.row == row) {
                        continue;
                    }
                    let bound = if layer == 0 { params.m0() } else { params.m() };
                    assert_eq!(
                        back.len(),
                        bound,
                        "{metric:?} {params:?}: {row} -> {} at layer {layer} has no return \
                         edge, but {}'s adjacency is not full",
                        neighbor.row,
                        neighbor.row
                    );
                }
            });
        }
    }
}

#[test]
fn a_narrow_matrix_builds_the_identical_graph_to_its_widened_copy() {
    // The whole licence for keeping a binary32 corpus narrow. Widening an f32 is exact, so a
    // matrix stored at PURREMB's own width must produce the SAME GRAPH, byte for byte, as
    // the same data widened up front. If it did not, halving the resident size would quietly
    // be a different index -- and the guard commitment over the canonical image would move.
    //
    // Asserted on the canonical image rather than on a sample of answers: two graphs can
    // agree on every probe you happen to try and differ elsewhere.
    let (rows, dims) = (192_usize, 12_usize);
    let mut state = 0x1F32_D00D_u64;
    let narrow: Vec<f32> = (0..rows * dims)
        .map(|_| {
            state = purrdf_hnsw::level::splitmix64(state);
            let value = ((state >> 11) as f64 / (1_u64 << 53) as f64).mul_add(2.0, -1.0);
            (if value == 0.0 { 0.125 } else { value }) as f32
        })
        .collect();
    let widened: Vec<f64> = narrow.iter().copied().map(f64::from).collect();

    for params in parameter_sets() {
        for metric in kernels() {
            let from_f32 = build(
                VectorMatrix::from_f32(rows, dims, narrow.clone()).expect("valid"),
                &metric,
                params,
            )
            .expect("builds");
            let from_f64 = build(
                VectorMatrix::new(rows, dims, widened.clone()).expect("valid"),
                &metric,
                params,
            )
            .expect("builds");
            assert_eq!(
                from_f32.canonical_image(),
                from_f64.canonical_image(),
                "{metric:?} {params:?}: the stored width must not reach the graph"
            );
        }
    }
}
