// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! Report-only experiment comparing the shipped pack codec's FoQ posting-list
//! indexes with a non-shipped pointerless bitmap wavelet matrix over the same
//! deterministic Sp/So adjacency sequences. The comparison deliberately keeps
//! the shared adjacency fixed and does not alter the pack wire format.

#[path = "support/pack_index.rs"]
mod pack_index;

use std::sync::Arc;

use criterion::{Criterion, criterion_group, criterion_main};
use pack_index::{Adjacency, FoqIndexes, WaveletIndexes, reference_space, reference_triples};
use purrdf_core::{
    DatasetView, GraphMatch, PackBuilder, PackId, PackView, RdfDataset, RdfDatasetBuilder,
    TermValue,
};

const QUERY_ROWS: usize = 65_536;

fn numeric_iri(role: char, value: u64) -> String {
    format!("http://example.org/{role}/{value:020}")
}

fn build_dataset(triples: &[(u64, u64, u64)]) -> Arc<RdfDataset> {
    let mut subjects: Vec<_> = triples.iter().map(|&(s, _, _)| s).collect();
    subjects.sort_unstable();
    subjects.dedup();
    let mut predicates: Vec<_> = triples.iter().map(|&(_, p, _)| p).collect();
    predicates.sort_unstable();
    predicates.dedup();
    let mut objects: Vec<_> = triples.iter().map(|&(_, _, o)| o).collect();
    objects.sort_unstable();
    objects.dedup();

    let mut builder = RdfDatasetBuilder::new();
    let subject_ids: Vec<_> = subjects
        .iter()
        .map(|&value| builder.intern_iri(&numeric_iri('s', value)))
        .collect();
    let predicate_ids: Vec<_> = predicates
        .iter()
        .map(|&value| builder.intern_iri(&numeric_iri('p', value)))
        .collect();
    let object_ids: Vec<_> = objects
        .iter()
        .map(|&value| builder.intern_iri(&numeric_iri('o', value)))
        .collect();

    for &(subject, predicate, object) in triples {
        let subject = subject_ids[subjects.binary_search(&subject).expect("subject present")];
        let predicate = predicate_ids[predicates
            .binary_search(&predicate)
            .expect("predicate present")];
        let object = object_ids[objects.binary_search(&object).expect("object present")];
        builder.push_quad(subject, predicate, object, None);
    }
    builder.freeze().expect("reference dataset is valid")
}

fn pack_term(pack: &PackView<'_>, role: char, value: u64) -> PackId {
    pack.term_id_by_value(&TermValue::iri(numeric_iri(role, value)))
        .expect("reference term is packed")
}

fn report_space_curve() {
    eprintln!(
        "pack-index-space rows triples shared-bytes foq-index-bytes wavelet-index-bytes foq-total-bytes wavelet-total-bytes"
    );
    for rows in [
        1_024u64,
        16_384,
        28_414,
        28_415,
        65_536,
        262_144,
        25_000_000,
        250_000_000,
    ] {
        let space = reference_space(rows);
        eprintln!(
            "pack-index-space {rows} {} {} {} {} {} {}",
            space.triples,
            space.shared,
            space.foq_index,
            space.wavelet_index,
            space.foq_total(),
            space.wavelet_total(),
        );
    }

    let mut previous = reference_space(1)
        .wavelet_index
        .cmp(&reference_space(1).foq_index);
    let mut flip_count = 0u64;
    let mut last_flip = None;
    for rows in 2..=1_048_576 {
        let space = reference_space(rows);
        let ordering = space.wavelet_index.cmp(&space.foq_index);
        if ordering != previous {
            flip_count += 1;
            last_flip = Some((rows, previous, ordering));
            previous = ordering;
        }
    }
    let (last_rows, last_previous, last_current) = last_flip.expect("the ordering changes");
    eprintln!(
        "pack-index-ordering-scan max-rows=1048576 flips={flip_count} last-rows={last_rows} last-previous={last_previous:?} last-current={last_current:?}"
    );
}

fn bench_pack_index_hypothesis(c: &mut Criterion) {
    report_space_curve();

    let triples = reference_triples(QUERY_ROWS);
    let adjacency = Adjacency::from_triples(&triples);
    let foq = FoqIndexes::build(&adjacency);
    let wavelet = WaveletIndexes::build(&adjacency);
    let dataset = build_dataset(&triples);
    let pack_bytes = PackBuilder::build_bytes(&dataset).expect("reference dataset packs");
    let pack = PackView::from_bytes(&pack_bytes).expect("reference pack opens");

    let dense_predicate = 0;
    let dense_object = 0;
    let sparse_predicate = 32 + (QUERY_ROWS as u64 - 1) % 1_024;
    let sparse_object = 4_354 + QUERY_ROWS as u64 - 1;

    let dense_predicate_local = adjacency
        .predicate_local(dense_predicate)
        .expect("dense predicate present");
    let dense_object_local = adjacency
        .object_local(dense_object)
        .expect("dense object present");
    let sparse_predicate_local = adjacency
        .predicate_local(sparse_predicate)
        .expect("sparse predicate present");
    let sparse_object_local = adjacency
        .object_local(sparse_object)
        .expect("sparse object present");

    let dense_predicate_term = pack_term(&pack, 'p', dense_predicate);
    let dense_object_term = pack_term(&pack, 'o', dense_object);
    let sparse_predicate_term = pack_term(&pack, 'p', sparse_predicate);
    let sparse_object_term = pack_term(&pack, 'o', sparse_object);

    let expected_dense_predicate = pack
        .quads_for_pattern(None, Some(dense_predicate_term), None, GraphMatch::Any)
        .count();
    let expected_dense_object = pack
        .quads_for_pattern(None, None, Some(dense_object_term), GraphMatch::Any)
        .count();
    let expected_dense_pair = pack
        .quads_for_pattern(
            None,
            Some(dense_predicate_term),
            Some(dense_object_term),
            GraphMatch::Any,
        )
        .count();
    let expected_sparse_predicate = pack
        .quads_for_pattern(None, Some(sparse_predicate_term), None, GraphMatch::Any)
        .count();
    let expected_sparse_object = pack
        .quads_for_pattern(None, None, Some(sparse_object_term), GraphMatch::Any)
        .count();
    let expected_sparse_pair = pack
        .quads_for_pattern(
            None,
            Some(sparse_predicate_term),
            Some(sparse_object_term),
            GraphMatch::Any,
        )
        .count();

    assert_eq!(
        foq.predicate_count(&adjacency, dense_predicate_local),
        expected_dense_predicate
    );
    assert_eq!(
        wavelet.predicate_count(&adjacency, dense_predicate_local),
        expected_dense_predicate
    );
    assert_eq!(foq.object_count(dense_object_local), expected_dense_object);
    assert_eq!(
        wavelet.object_count(dense_object_local),
        expected_dense_object
    );
    assert_eq!(
        foq.predicate_object_count(dense_predicate_local, dense_object_local),
        expected_dense_pair
    );
    assert_eq!(
        wavelet.predicate_object_count(&adjacency, dense_predicate_local, dense_object_local),
        expected_dense_pair
    );
    assert_eq!(
        foq.predicate_count(&adjacency, sparse_predicate_local),
        expected_sparse_predicate
    );
    assert_eq!(
        wavelet.predicate_count(&adjacency, sparse_predicate_local),
        expected_sparse_predicate
    );
    assert_eq!(
        foq.object_count(sparse_object_local),
        expected_sparse_object
    );
    assert_eq!(
        wavelet.object_count(sparse_object_local),
        expected_sparse_object
    );
    assert_eq!(
        foq.predicate_object_count(sparse_predicate_local, sparse_object_local),
        expected_sparse_pair
    );
    assert_eq!(
        wavelet.predicate_object_count(&adjacency, sparse_predicate_local, sparse_object_local),
        expected_sparse_pair
    );

    eprintln!(
        "pack-index-context rows={} triples={} pack-bytes={} foq-index-bytes={} wavelet-index-bytes={}",
        QUERY_ROWS,
        triples.len(),
        pack_bytes.len(),
        foq.index_serialized_len(),
        wavelet.index_serialized_len(),
    );

    {
        let mut build = c.benchmark_group("pack_index_build");
        build.bench_function("production_pack", |b| {
            b.iter(|| {
                std::hint::black_box(
                    PackBuilder::build_bytes(&dataset).expect("reference dataset packs"),
                )
            });
        });
        build.bench_function("isolated_foq", |b| {
            b.iter(|| std::hint::black_box(FoqIndexes::build(&adjacency)));
        });
        build.bench_function("isolated_wavelet", |b| {
            b.iter(|| std::hint::black_box(WaveletIndexes::build(&adjacency)));
        });
        build.finish();
    }

    let mut query = c.benchmark_group("pack_index_query");
    query.bench_function("production_foq/predicate_dense", |b| {
        b.iter(|| {
            std::hint::black_box(
                pack.quads_for_pattern(None, Some(dense_predicate_term), None, GraphMatch::Any)
                    .count(),
            )
        });
    });
    query.bench_function("isolated_foq/predicate_dense", |b| {
        b.iter(|| std::hint::black_box(foq.predicate_count(&adjacency, dense_predicate_local)));
    });
    query.bench_function("isolated_wavelet/predicate_dense", |b| {
        b.iter(|| std::hint::black_box(wavelet.predicate_count(&adjacency, dense_predicate_local)));
    });
    query.bench_function("production_foq/object_dense", |b| {
        b.iter(|| {
            std::hint::black_box(
                pack.quads_for_pattern(None, None, Some(dense_object_term), GraphMatch::Any)
                    .count(),
            )
        });
    });
    query.bench_function("isolated_foq/object_dense", |b| {
        b.iter(|| std::hint::black_box(foq.object_count(dense_object_local)));
    });
    query.bench_function("isolated_wavelet/object_dense", |b| {
        b.iter(|| std::hint::black_box(wavelet.object_count(dense_object_local)));
    });
    query.bench_function("production_foq/pair_dense", |b| {
        b.iter(|| {
            std::hint::black_box(
                pack.quads_for_pattern(
                    None,
                    Some(dense_predicate_term),
                    Some(dense_object_term),
                    GraphMatch::Any,
                )
                .count(),
            )
        });
    });
    query.bench_function("isolated_foq/pair_dense", |b| {
        b.iter(|| {
            std::hint::black_box(
                foq.predicate_object_count(dense_predicate_local, dense_object_local),
            )
        });
    });
    query.bench_function("isolated_wavelet/pair_dense", |b| {
        b.iter(|| {
            std::hint::black_box(wavelet.predicate_object_count(
                &adjacency,
                dense_predicate_local,
                dense_object_local,
            ))
        });
    });
    query.bench_function("production_foq/predicate_sparse", |b| {
        b.iter(|| {
            std::hint::black_box(
                pack.quads_for_pattern(None, Some(sparse_predicate_term), None, GraphMatch::Any)
                    .count(),
            )
        });
    });
    query.bench_function("isolated_foq/predicate_sparse", |b| {
        b.iter(|| std::hint::black_box(foq.predicate_count(&adjacency, sparse_predicate_local)));
    });
    query.bench_function("isolated_wavelet/predicate_sparse", |b| {
        b.iter(|| {
            std::hint::black_box(wavelet.predicate_count(&adjacency, sparse_predicate_local))
        });
    });
    query.bench_function("production_foq/object_sparse", |b| {
        b.iter(|| {
            std::hint::black_box(
                pack.quads_for_pattern(None, None, Some(sparse_object_term), GraphMatch::Any)
                    .count(),
            )
        });
    });
    query.bench_function("isolated_foq/object_sparse", |b| {
        b.iter(|| std::hint::black_box(foq.object_count(sparse_object_local)));
    });
    query.bench_function("isolated_wavelet/object_sparse", |b| {
        b.iter(|| std::hint::black_box(wavelet.object_count(sparse_object_local)));
    });
    query.bench_function("production_foq/pair_sparse", |b| {
        b.iter(|| {
            std::hint::black_box(
                pack.quads_for_pattern(
                    None,
                    Some(sparse_predicate_term),
                    Some(sparse_object_term),
                    GraphMatch::Any,
                )
                .count(),
            )
        });
    });
    query.bench_function("isolated_foq/pair_sparse", |b| {
        b.iter(|| {
            std::hint::black_box(
                foq.predicate_object_count(sparse_predicate_local, sparse_object_local),
            )
        });
    });
    query.bench_function("isolated_wavelet/pair_sparse", |b| {
        b.iter(|| {
            std::hint::black_box(wavelet.predicate_object_count(
                &adjacency,
                sparse_predicate_local,
                sparse_object_local,
            ))
        });
    });
    query.finish();
}

criterion_group!(benches, bench_pack_index_hypothesis);
criterion_main!(benches);
