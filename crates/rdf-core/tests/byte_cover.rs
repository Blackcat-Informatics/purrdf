// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The format-neutral byte law through production emit, decode and reference APIs.

use purrdf_core::{
    ContentDigest,
    cover::{
        ByteCover, ByteSpan, CoverBuilder, DelimitedCover, EmitError, ReconstructError,
        reconstruct_bytes,
    },
};

fn opaque_identity_covers_every_octet_and_empty_content() {
    let all: Vec<u8> = (0..=u8::MAX).collect();
    for source in [
        all.as_slice(),
        b"".as_slice(),
        b"\xff\0\xc0\xaf\x80".as_slice(),
    ] {
        let cover = ByteCover::identity(source);
        assert_eq!(cover.spans().len(), 1);
        assert_eq!(cover.spans()[0].bytes.as_ptr(), source.as_ptr());
        assert_eq!(cover.reconstruct().unwrap(), source);
        assert_eq!(cover.source_digest(), ContentDigest::of(source));
    }
}

fn emitted_byte_continuations_obey_original_refusal_neighbors() {
    let source = b"\xff\x80abcd";
    let mut lawful = CoverBuilder::new(source);
    lawful.emit(0..4, None).unwrap();
    lawful.emit(2..6, Some(0..4)).unwrap();
    assert_eq!(lawful.finish().unwrap().reconstruct().unwrap(), source);

    let mut undeclared = CoverBuilder::new(source);
    undeclared.emit(0..4, None).unwrap();
    undeclared.emit(2..6, None).unwrap();
    assert!(matches!(
        undeclared.finish(),
        Err(EmitError::Cover(ReconstructError::UndeclaredOverlap { .. }))
    ));

    let mut gap = CoverBuilder::new(source);
    gap.emit(0..2, None).unwrap();
    gap.emit(3..6, None).unwrap();
    assert!(matches!(
        gap.finish(),
        Err(EmitError::Cover(ReconstructError::UncoveredRange {
            byte_start: 2,
            byte_end: 3
        }))
    ));

    let mut self_reference = CoverBuilder::new(source);
    self_reference.emit(0..6, None).unwrap();
    self_reference.emit(0..6, Some(0..6)).unwrap();
    assert!(matches!(
        self_reference.finish(),
        Err(EmitError::Cover(ReconstructError::UndeclaredOverlap { .. }))
    ));
}

fn span_references_prove_content_before_returning_any_window() {
    let source = b"\xff\0bytes\x80";
    let cover = ByteCover::identity(source);
    let reference = cover.reference(1..7).unwrap();
    assert_eq!(reference.resolve(&cover).unwrap(), &source[1..7]);
    assert_eq!(reference.resolve_source(source).unwrap(), &source[1..7]);
    assert!(matches!(
        reference.resolve_source(b"\xff\0other\x80"),
        Err(EmitError::Cover(ReconstructError::DigestMismatch { .. }))
    ));
    assert!(cover.reference(0..source.len() as u64 + 1).is_err());
    let empty = cover
        .reference(source.len() as u64..source.len() as u64)
        .unwrap();
    assert_eq!(empty.resolve(&cover).unwrap(), b"");
}

fn delimited_records_preserve_separator_bytes_and_empty_multiplicity() {
    let source = b"\xff|same||same|";
    let model = DelimitedCover::analyze(source, b'|').unwrap();
    assert_eq!(model.cover.reconstruct().unwrap(), source);
    let records: Vec<_> = model
        .records
        .iter()
        .map(|range| &source[range.clone()])
        .collect();
    assert_eq!(records, [b"\xff".as_slice(), b"same", b"", b"same", b""]);
    assert_eq!(model.separators.len(), 4);
    let empty = DelimitedCover::analyze(b"", b'|').unwrap();
    assert_eq!(empty.records.len(), 1);
    assert_eq!(empty.records[0], 0..0);
}

fn byte_law_has_frozen_identity_and_order_independent_reconstruction() {
    let source = b"abc";
    let digest = ContentDigest::of(source);
    assert_eq!(
        digest.to_hex(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let spans = [
        ByteSpan {
            byte_start: 1,
            byte_end: 3,
            bytes: b"bc",
            continues: None,
        },
        ByteSpan {
            byte_start: 0,
            byte_end: 1,
            bytes: b"a",
            continues: None,
        },
    ];
    assert_eq!(reconstruct_bytes(3, &digest, &spans).unwrap(), source);
    let wrong = ContentDigest::of(b"abd");
    assert!(matches!(
        reconstruct_bytes(3, &wrong, &spans),
        Err(ReconstructError::DigestMismatch { .. })
    ));
}

purrdf_testkit::harness_main!(
    opaque_identity_covers_every_octet_and_empty_content,
    emitted_byte_continuations_obey_original_refusal_neighbors,
    span_references_prove_content_before_returning_any_window,
    delimited_records_preserve_separator_bytes_and_empty_multiplicity,
    byte_law_has_frozen_identity_and_order_independent_reconstruction,
);
