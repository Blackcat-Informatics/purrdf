// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Heap, section-aligned, and deliberately misaligned PURREMB parity.
//!
//! The reader contract is over borrowed bytes: a caller may hand it a file
//! mapping, a heap buffer, or a slice of a larger buffer. What the reader
//! observes of that backing is its contents and its address alignment, so these
//! tests read the golden file into owned buffers and control the alignment
//! directly: one aligned the way a page-aligned file mapping is, one shifted off
//! every alignment the format promises.

#![cfg(not(target_arch = "wasm32"))]

use purrdf_core::{
    ArtifactRoot, EmbeddingError, EmbeddingIntegrity, EmbeddingView, TargetId, VectorSpaceId,
    reopen_prevalidated, verify_embedding,
};

#[derive(Debug, PartialEq)]
struct Snapshot {
    root: ArtifactRoot,
    targets: Vec<TargetId>,
    spaces: Vec<VectorSpaceId>,
    rows: Vec<Vec<u32>>,
}

fn snapshot(bytes: &[u8]) -> Snapshot {
    let mut view = EmbeddingView::from_bytes(bytes).expect("structural PURREMB view");
    verify_embedding(&mut view).expect("fully verified PURREMB view");
    let matrix = view.matrices().next().expect("fixture matrix");
    let rows = (0..matrix.row_count())
        .map(|row| {
            matrix
                .f32_row(row)
                .expect("f32 row")
                .map(|value| value.expect("finite scalar").to_bits())
                .collect()
        })
        .collect();
    Snapshot {
        root: view.artifact_root(),
        targets: view.targets().map(purrdf_core::TargetView::id).collect(),
        spaces: view
            .families()
            .flat_map(purrdf_core::FamilyView::spaces)
            .map(purrdf_core::EffectiveSpaceView::id)
            .collect(),
        rows,
    }
}

/// The alignment a file mapping's first byte carries at the least (a page is
/// larger), and the largest section alignment the PURREMB layout uses.
const SECTION_ALIGN: usize = 64;

/// The golden PURREMB file's bytes.
fn golden() -> Vec<u8> {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/goldens/purremb_v1.bin");
    std::fs::read(path).expect("golden bytes")
}

/// `bytes` copied into `storage` at an offset aligned to [`SECTION_ALIGN`],
/// returned as the aligned slice: the alignment a page-aligned mapping gives.
fn aligned_copy<'a>(storage: &'a mut Vec<u8>, bytes: &[u8]) -> &'a [u8] {
    storage.clear();
    storage.resize(bytes.len() + SECTION_ALIGN, 0);
    let start = storage.as_ptr().align_offset(SECTION_ALIGN);
    storage[start..start + bytes.len()].copy_from_slice(bytes);
    let aligned = &storage[start..start + bytes.len()];
    assert_eq!(aligned.as_ptr() as usize % SECTION_ALIGN, 0);
    aligned
}

#[test]
fn heap_aligned_and_misaligned_borrows_are_logically_identical() {
    let heap = golden();
    let mut storage = Vec::new();
    let aligned = aligned_copy(&mut storage, &heap);
    let mut shifted = Vec::with_capacity(heap.len() + 1);
    shifted.push(0xa5);
    shifted.extend_from_slice(&heap);
    let misaligned = &shifted[1..];

    let expected = snapshot(&heap);
    assert_eq!(snapshot(aligned), expected);
    assert_eq!(snapshot(misaligned), expected);

    let mut aligned_view = EmbeddingView::from_bytes(aligned).expect("aligned view");
    verify_embedding(&mut aligned_view).expect("verified aligned view");
    assert!(
        aligned_view
            .matrices()
            .next()
            .expect("matrix")
            .native_f32_row(0)
            .is_some(),
        "section-aligned backing bytes permit native f32"
    );

    let mut shifted_view = EmbeddingView::from_bytes(misaligned).expect("shifted view");
    verify_embedding(&mut shifted_view).expect("verified shifted view");
    assert!(
        shifted_view
            .matrices()
            .next()
            .expect("matrix")
            .native_f32_row(0)
            .is_none(),
        "misaligned backing bytes must use portable scalar decoding"
    );
}

#[test]
fn resident_certificate_reopens_only_the_certified_byte_range() {
    // Two owned copies of the same bytes at different addresses: the certificate
    // binds the byte range it verified, not merely equal contents.
    let heap = golden();
    let resident = golden();

    let certificate = {
        let mut view = EmbeddingView::from_bytes(&resident).expect("resident view");
        verify_embedding(&mut view)
            .expect("verified resident view")
            .into_certificate()
    };
    let reopened = reopen_prevalidated(&resident, &certificate).expect("same resident range");
    assert_eq!(reopened.integrity(), EmbeddingIntegrity::FullyVerified);

    assert!(matches!(
        reopen_prevalidated(&heap, &certificate),
        Err(EmbeddingError::CertificateMismatch)
    ));

    let mut duplicated = Vec::with_capacity(heap.len() * 2);
    duplicated.extend_from_slice(&heap);
    duplicated.extend_from_slice(&heap);
    let (first, second) = duplicated.split_at(heap.len());
    let range_certificate = {
        let mut view = EmbeddingView::from_bytes(first).expect("first embedded range");
        verify_embedding(&mut view)
            .expect("verified first embedded range")
            .into_certificate()
    };
    assert!(reopen_prevalidated(first, &range_certificate).is_ok());
    assert!(matches!(
        reopen_prevalidated(second, &range_certificate),
        Err(EmbeddingError::CertificateMismatch)
    ));
}
