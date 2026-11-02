// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The five files of a seeded dataset, pinned byte for byte by digest.
//!
//! These native digests pin adaptive physical encodings, exact/conservative
//! statistics and column/offset indexes. Their addition deliberately changes
//! the file bytes while preserving the RDF model and schema version.

#[cfg(not(target_arch = "wasm32"))]
use purrdf_columnar::read;
use purrdf_columnar::{Compression, Table, write};
use purrdf_core::{
    BlankScope, ContentDigest, ContentStore, RdfDataset, RdfDatasetBuilder, RdfLiteral,
    RdfTextDirection,
};
use purrdf_testkit::rng::splitmix64_next as mix;

/// Every term kind in a seeded mix, so each nullable `INT64` column of the
/// terms table (datatype, direction, scope, the triple components) holds runs
/// of nulls and values of many lengths across several 64-row words, and the
/// graph columns of the quads, reifiers and annotations are sometimes null.
fn fixture() -> (std::sync::Arc<RdfDataset>, ContentStore) {
    let mut state = 0x6012_u64;
    let mut builder = RdfDatasetBuilder::new();
    let predicates: Vec<_> = (0..5)
        .map(|i| {
            builder
                .intern_iri(&format!("https://example.org/p{i}"))
                .unwrap()
        })
        .collect();
    let graphs: Vec<_> = (0..3)
        .map(|i| {
            builder
                .intern_iri(&format!("https://example.org/g{i}"))
                .unwrap()
        })
        .collect();
    let empty = builder.intern_iri("https://example.org/empty").unwrap();
    builder.declare_named_graph(empty).unwrap();
    for row in 0..400_u32 {
        let subject = match mix(&mut state) % 3 {
            0 => builder
                .intern_blank(&format!("b{row}"), BlankScope(row % 5))
                .unwrap(),
            _ => builder
                .intern_iri(&format!("https://example.org/s{row}"))
                .unwrap(),
        };
        let object = match mix(&mut state) % 6 {
            0 => builder
                .intern_literal(RdfLiteral::simple(format!("plain {row}")))
                .unwrap(),
            1 => builder
                .intern_literal(RdfLiteral::typed(
                    row.to_string(),
                    "https://example.org/integer",
                ))
                .unwrap(),
            2 => builder
                .intern_literal(RdfLiteral::language_tagged(format!("mot {row}"), "fr"))
                .unwrap(),
            3 => builder
                .intern_literal(RdfLiteral {
                    lexical_form: format!("كلمة {row}"),
                    datatype: None,
                    language: Some("ar".to_owned()),
                    direction: Some(if row % 2 == 0 {
                        RdfTextDirection::Rtl
                    } else {
                        RdfTextDirection::Ltr
                    }),
                })
                .unwrap(),
            4 => builder
                .intern_blank(&format!("o{row}"), BlankScope(row % 3))
                .unwrap(),
            _ => builder
                .intern_iri(&format!("https://example.org/o{row}"))
                .unwrap(),
        };
        let predicate = predicates[(mix(&mut state) % 5) as usize];
        let graph = match mix(&mut state) % 4 {
            0 => None,
            k => Some(graphs[(k - 1) as usize]),
        };
        builder
            .push_quad(subject, predicate, object, graph)
            .unwrap();
        if mix(&mut state).is_multiple_of(4) {
            let triple = builder.intern_triple(subject, predicate, object).unwrap();
            let reifier = builder
                .intern_blank(&format!("r{row}"), BlankScope(row % 7))
                .unwrap();
            let graph = mix(&mut state).is_multiple_of(2).then_some(graphs[0]);
            builder
                .push_reifier_in_graph(reifier, triple, graph)
                .unwrap();
            let graph = mix(&mut state).is_multiple_of(3).then_some(graphs[1]);
            builder
                .push_annotation_in_graph(reifier, predicates[0], object, graph)
                .unwrap();
        }
    }
    let mut blobs = ContentStore::new();
    for row in 0..9 {
        blobs.insert(format!("payload {row}").into_bytes());
    }
    (builder.freeze().expect("the fixture is valid RDF"), blobs)
}

fn digests(compression: Compression) -> Vec<(Table, String)> {
    let (dataset, blobs) = fixture();
    let written = write(&*dataset, &blobs, compression).expect("the fixture encodes");
    #[cfg(not(target_arch = "wasm32"))]
    {
        let read_back = read(&written.files).expect("the fixture decodes");
        let rewritten = write(&*read_back.dataset, &read_back.blobs, compression)
            .expect("the read-back encodes");
        assert_eq!(
            rewritten.files, written.files,
            "a read-back rewrites the same bytes"
        );
    }
    written
        .files
        .iter()
        .map(|(table, bytes)| (table, ContentDigest::of(bytes).to_hex()))
        .collect()
}

const PINNED: [(Compression, Table, &str); 10] = [
    (
        Compression::Uncompressed,
        Table::Terms,
        "5667849c1df292bd4de0bb4171d63e5e72c2cf20c3fa886661b7d46f9efb271a",
    ),
    (
        Compression::Uncompressed,
        Table::Quads,
        "690c92c367bf75f0c6fe27946338eb687234bcca2cb668c6a99b085ccffe6095",
    ),
    (
        Compression::Uncompressed,
        Table::Reifiers,
        "0d7269e294c13b12d09eb0c808bc6f6b224618198a3dde7212146ec180d27195",
    ),
    (
        Compression::Uncompressed,
        Table::Annotations,
        "4858d6fdd419114cf532ca5b11b2f5c55d97a53f4553298ae9e865138094443a",
    ),
    (
        Compression::Uncompressed,
        Table::Blobs,
        "8dedcd85027c02020f3b2f8d1c7a8508d7bd63a815be0b4ee78a173b8f5331a7",
    ),
    (
        Compression::Zstd,
        Table::Terms,
        "9fe1fb68c3600c25e2373d7df50389c3e91636b7f6a263b2fa27ab929b8170b4",
    ),
    (
        Compression::Zstd,
        Table::Quads,
        "818949c71b957b24213c90f7452c49e9b73e9b4f5c2831693c523e7f2bb0d789",
    ),
    (
        Compression::Zstd,
        Table::Reifiers,
        "08e966eb4b412b2a386b9b912063edf4d06fa9c7b254115f29336c19725905e9",
    ),
    (
        Compression::Zstd,
        Table::Annotations,
        "d8ebc62e39d0f03f86ed28324105e24a636304e00ed4dbe52c793aed97a1668d",
    ),
    (
        Compression::Zstd,
        Table::Blobs,
        "9dc6dd2c8d963d1f07bdeb790f36e4d58c9a96350119c9fb2dadaf5fb864c19c",
    ),
];

/// Verify all ten frozen file digests across both compression modes.
fn seeded_files_are_the_pinned_bytes() {
    let mut observed = Vec::new();
    for compression in [Compression::Uncompressed, Compression::Zstd] {
        for (table, digest) in digests(compression) {
            observed.push((compression, table, digest));
        }
    }
    let pinned: Vec<_> = PINNED
        .iter()
        .map(|&(compression, table, digest)| (compression, table, digest.to_owned()))
        .collect();
    assert_eq!(observed, pinned);
}

purrdf_testkit::harness_main!(seeded_files_are_the_pinned_bytes);
