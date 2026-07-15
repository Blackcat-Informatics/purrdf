// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

#![allow(dead_code, missing_docs)]

#[path = "../benches/support/pack_index.rs"]
mod pack_index;

use pack_index::{
    Adjacency, FoqIndexes, WaveletIndexes, WaveletMatrix, reference_space, reference_triples,
};

#[test]
fn wavelet_access_rank_and_select_match_plain_sequences() {
    let sequences = [
        Vec::new(),
        vec![0],
        vec![3, 0, 5, 3, 7, 0, 3, 1, 5, 2, 3],
        (0..257).map(|i| (i * 37) % 19).collect(),
    ];

    for sequence in sequences {
        let matrix = WaveletMatrix::build(&sequence);
        for (position, &expected) in sequence.iter().enumerate() {
            assert_eq!(matrix.access(position), Some(expected));
        }
        assert_eq!(matrix.access(sequence.len()), None);

        for value in 0..=sequence.iter().copied().max().unwrap_or(0) + 1 {
            for end in 0..=sequence.len() {
                let expected = sequence[..end]
                    .iter()
                    .filter(|&&candidate| candidate == value)
                    .count();
                assert_eq!(matrix.rank(value, end), expected);
            }
            let positions: Vec<_> = sequence
                .iter()
                .enumerate()
                .filter_map(|(position, &candidate)| (candidate == value).then_some(position))
                .collect();
            for (occurrence, &expected) in positions.iter().enumerate() {
                assert_eq!(matrix.select(value, occurrence), Some(expected));
            }
            assert_eq!(matrix.select(value, positions.len()), None);
        }
    }
}

#[test]
fn wavelet_and_foq_queries_match_reference_triples() {
    let triples = reference_triples(2_048);
    let adjacency = Adjacency::from_triples(&triples);
    let foq = FoqIndexes::build(&adjacency);
    let wavelet = WaveletIndexes::build(&adjacency);

    for predicate in [0, 1, 31, 32, 777, 1_055] {
        let local = adjacency
            .predicate_local(predicate)
            .expect("reference predicate is present");
        let expected = triples
            .iter()
            .filter(|&&(_, candidate, _)| candidate == predicate)
            .count();
        assert_eq!(foq.predicate_count(&adjacency, local), expected);
        assert_eq!(wavelet.predicate_count(&adjacency, local), expected);
    }

    for object in [0, 1, 257, 258, 4_354, 5_000, 6_401] {
        let local = adjacency
            .object_local(object)
            .expect("reference object is present");
        let expected = triples
            .iter()
            .filter(|&&(_, _, candidate)| candidate == object)
            .count();
        assert_eq!(foq.object_count(local), expected);
        assert_eq!(wavelet.object_count(local), expected);
    }

    for (predicate, object) in [(0, 0), (0, 1), (1, 258), (777, 6_401)] {
        let predicate_local = adjacency
            .predicate_local(predicate)
            .expect("reference predicate is present");
        let object_local = adjacency
            .object_local(object)
            .expect("reference object is present");
        let expected = triples
            .iter()
            .filter(|&&(_, candidate_p, candidate_o)| {
                candidate_p == predicate && candidate_o == object
            })
            .count();
        assert_eq!(
            foq.predicate_object_count(predicate_local, object_local),
            expected
        );
        assert_eq!(
            wavelet.predicate_object_count(&adjacency, predicate_local, object_local),
            expected
        );
    }
}

#[test]
fn experiment_builds_are_byte_deterministic() {
    let adjacency = Adjacency::from_triples(&reference_triples(1_024));
    let foq_a = FoqIndexes::build(&adjacency);
    let foq_b = FoqIndexes::build(&adjacency);
    let wavelet_a = WaveletIndexes::build(&adjacency);
    let wavelet_b = WaveletIndexes::build(&adjacency);

    assert_eq!(foq_a.to_bytes(), foq_b.to_bytes());
    assert_eq!(wavelet_a.to_bytes(), wavelet_b.to_bytes());
    assert_eq!(foq_a.to_bytes().len(), foq_a.index_serialized_len());
    assert_eq!(wavelet_a.to_bytes().len(), wavelet_a.index_serialized_len());
}

#[test]
fn reference_space_model_matches_materialized_encodings() {
    for rows in [1, 31, 257, 1_024, 4_096, 16_384] {
        let adjacency = Adjacency::from_triples(&reference_triples(rows));
        let foq = FoqIndexes::build(&adjacency);
        let wavelet = WaveletIndexes::build(&adjacency);
        let model = reference_space(rows as u64);

        assert_eq!(model.triples, adjacency.so_len() as u64);
        assert_eq!(model.shared, adjacency.shared_serialized_len() as u64);
        assert_eq!(model.foq_index, foq.index_serialized_len() as u64);
        assert_eq!(model.wavelet_index, wavelet.index_serialized_len() as u64);
        assert_eq!(model.foq_total(), foq.serialized_len(&adjacency) as u64);
        assert_eq!(
            model.wavelet_total(),
            wavelet.serialized_len(&adjacency) as u64
        );
    }
}
