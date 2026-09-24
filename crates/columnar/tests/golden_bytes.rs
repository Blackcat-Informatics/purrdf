// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The five files of a seeded dataset, pinned byte for byte by digest.
//!
//! The digests were taken from the encoder that held each nullable `INT64`
//! column as `Vec<Option<i64>>`, before the column moved to a presence bitmap
//! and a dense value vector. They are not regenerated: a change that moves
//! them changes the written bytes, which the byte-determinism contract
//! forbids without a golden update and its reason.

use purrdf_columnar::{Compression, Table, read, write};
use purrdf_core::{
    BlankScope, ContentDigest, ContentStore, RdfDataset, RdfDatasetBuilder, RdfLiteral,
    RdfTextDirection,
};

/// A deterministic SplitMix64 step: the fixture's only source of variety.
fn mix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Every term kind in a seeded mix, so each nullable `INT64` column of the
/// terms table (datatype, direction, scope, the triple components) holds runs
/// of nulls and values of many lengths across several 64-row words, and the
/// graph columns of the quads, reifiers and annotations are sometimes null.
fn fixture() -> (std::sync::Arc<RdfDataset>, ContentStore) {
    let mut state = 0x6012_u64;
    let mut builder = RdfDatasetBuilder::new();
    let predicates: Vec<_> = (0..5)
        .map(|i| builder.intern_iri(&format!("https://example.org/p{i}")))
        .collect();
    let graphs: Vec<_> = (0..3)
        .map(|i| builder.intern_iri(&format!("https://example.org/g{i}")))
        .collect();
    let empty = builder.intern_iri("https://example.org/empty");
    builder.declare_named_graph(empty);
    for row in 0..400_u32 {
        let subject = match mix(&mut state) % 3 {
            0 => builder.intern_blank(&format!("b{row}"), BlankScope(row % 5)),
            _ => builder.intern_iri(&format!("https://example.org/s{row}")),
        };
        let object = match mix(&mut state) % 6 {
            0 => builder.intern_literal(RdfLiteral::simple(format!("plain {row}"))),
            1 => builder.intern_literal(RdfLiteral::typed(
                row.to_string(),
                "https://example.org/integer",
            )),
            2 => builder.intern_literal(RdfLiteral::language_tagged(format!("mot {row}"), "fr")),
            3 => builder.intern_literal(RdfLiteral {
                lexical_form: format!("كلمة {row}"),
                datatype: None,
                language: Some("ar".to_owned()),
                direction: Some(if row % 2 == 0 {
                    RdfTextDirection::Rtl
                } else {
                    RdfTextDirection::Ltr
                }),
            }),
            4 => builder.intern_blank(&format!("o{row}"), BlankScope(row % 3)),
            _ => builder.intern_iri(&format!("https://example.org/o{row}")),
        };
        let predicate = predicates[(mix(&mut state) % 5) as usize];
        let graph = match mix(&mut state) % 4 {
            0 => None,
            k => Some(graphs[(k - 1) as usize]),
        };
        builder.push_quad(subject, predicate, object, graph);
        if mix(&mut state).is_multiple_of(4) {
            let triple = builder.intern_triple(subject, predicate, object);
            let reifier = builder.intern_blank(&format!("r{row}"), BlankScope(row % 7));
            let graph = mix(&mut state).is_multiple_of(2).then_some(graphs[0]);
            builder.push_reifier_in_graph(reifier, triple, graph);
            let graph = mix(&mut state).is_multiple_of(3).then_some(graphs[1]);
            builder.push_annotation_in_graph(reifier, predicates[0], object, graph);
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
    let read_back = read(&written.files).expect("the fixture decodes");
    let rewritten =
        write(&*read_back.dataset, &read_back.blobs, compression).expect("the read-back encodes");
    assert_eq!(
        rewritten.files, written.files,
        "a read-back rewrites the same bytes"
    );
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
        "f55714a0def2b5690fdb13d5370cf2955636b1a4dd247d9b98b3a21be0640ad0",
    ),
    (
        Compression::Uncompressed,
        Table::Quads,
        "8989b60250dd18db1b5c0bf9e5f1fb97661800422a1a1dc37a9dea36ee367362",
    ),
    (
        Compression::Uncompressed,
        Table::Reifiers,
        "ca070d5511e4b5a20c221ac5c19045484368f56b6c43ae6236aaae47bd4ded54",
    ),
    (
        Compression::Uncompressed,
        Table::Annotations,
        "13f0dfb715f0dc2b8e31f299f284bed32e90333dcb852fa4ebd8ef37a73fea92",
    ),
    (
        Compression::Uncompressed,
        Table::Blobs,
        "a70af1999d21847594dde92c43cf04588974f9ca9fbe1f0787703331aff7456d",
    ),
    (
        Compression::Zstd,
        Table::Terms,
        "1c86c691ef70e890e79c8e10da305bfdb81ebcf86cf85a256404d6154e82c36e",
    ),
    (
        Compression::Zstd,
        Table::Quads,
        "83dbdca636a40ff6446f113439547e3cc91f5ba7b8cab7dd291d721fedc709d5",
    ),
    (
        Compression::Zstd,
        Table::Reifiers,
        "aa8ccfb58a9f9a009c721f46eb3c6c2aa0142b00f021d85b1de5524673ac3559",
    ),
    (
        Compression::Zstd,
        Table::Annotations,
        "64687fcf66cbf767f68a5ff9b5d53917c28584ddf906d70262fc2351d3d1c149",
    ),
    (
        Compression::Zstd,
        Table::Blobs,
        "e6eeff0d444c8243411bcd8bf2f93665b6ccfbe1c855629a0e4a22e1601ef282",
    ),
];

#[test]
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
