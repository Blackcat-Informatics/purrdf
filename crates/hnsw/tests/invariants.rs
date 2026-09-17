// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The graph-invariant suite.
//!
//! Every property the build promises is asserted here against real graphs — several
//! parameter sets and both a norm-free and a norm-dividing kernel — rather than against a
//! mock. The fixture vectors are generated deterministically at test time from a
//! splitmix64 stream, so the suite carries no committed binaries and produces the same
//! graphs on every target.

use purrdf_core::DistanceMetric;
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
