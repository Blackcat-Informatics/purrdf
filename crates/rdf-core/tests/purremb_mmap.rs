// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Heap, immutable-mmap, and deliberately misaligned PURREMB parity.

#![cfg(not(target_arch = "wasm32"))]

use std::fs::File;

use memmap2::MmapOptions;
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

#[test]
fn heap_mmap_and_misaligned_borrows_are_logically_identical() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/goldens/purremb_v1.bin");
    let heap = std::fs::read(&path).expect("golden bytes");
    let file = File::open(&path).expect("golden file");
    // SAFETY: the checked-in fixture is opened read-only, the mapping is not
    // mutated or truncated during this test, and every view is dropped first.
    let mmap = unsafe { MmapOptions::new().map(&file).expect("immutable mmap") };
    let mut shifted = Vec::with_capacity(heap.len() + 1);
    shifted.push(0xa5);
    shifted.extend_from_slice(&heap);
    let misaligned = &shifted[1..];

    let expected = snapshot(&heap);
    assert_eq!(snapshot(&mmap), expected);
    assert_eq!(snapshot(misaligned), expected);

    let mut mmap_view = EmbeddingView::from_bytes(&mmap).expect("mmap view");
    verify_embedding(&mut mmap_view).expect("verified mmap view");
    assert!(
        mmap_view
            .matrices()
            .next()
            .expect("matrix")
            .native_f32_row(0)
            .is_some(),
        "page-aligned mmap plus 64-byte section alignment permits native f32"
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
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/goldens/purremb_v1.bin");
    let heap = std::fs::read(&path).expect("golden bytes");
    let file = File::open(&path).expect("golden file");
    // SAFETY: the checked-in fixture is opened read-only, the mapping is not
    // mutated or truncated during this test, and every view is dropped first.
    let mmap = unsafe { MmapOptions::new().map(&file).expect("immutable mmap") };

    let certificate = {
        let mut view = EmbeddingView::from_bytes(&mmap).expect("mmap view");
        verify_embedding(&mut view)
            .expect("verified mmap view")
            .into_certificate()
    };
    let reopened = reopen_prevalidated(&mmap, &certificate).expect("same mmap range");
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
