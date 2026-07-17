// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Fail-closed PURREMB framing, integrity, and deep-corruption coverage.

use purrdf_core::{
    EmbeddingError, EmbeddingView, PURREMB_HEADER_LENGTH, SECTION_MATRICES, SECTION_MATRIX_DATA,
    SECTION_RELATIONS, SECTION_TARGET_SETS, SECTION_TARGETS, SECTION_TOKEN_SPANS,
    derive_artifact_root, verify_embedding,
};
use sha2::{Digest as _, Sha256};

const DIRECTORY_ENTRY_LENGTH: usize = 64;

fn golden() -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/goldens/purremb_v1.bin"),
    )
    .expect("checked-in PURREMB golden")
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("u32 field"))
}

fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().expect("u64 field"))
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn directory_entry(bytes: &[u8], kind: u32, instance: u32) -> usize {
    let count = usize::try_from(read_u32(bytes, 20)).expect("section count");
    (0..count)
        .map(|index| PURREMB_HEADER_LENGTH as usize + index * DIRECTORY_ENTRY_LENGTH)
        .find(|offset| read_u32(bytes, *offset) == kind && read_u32(bytes, *offset + 8) == instance)
        .expect("section directory entry")
}

fn section_span(bytes: &[u8], kind: u32, instance: u32) -> (usize, usize) {
    let entry = directory_entry(bytes, kind, instance);
    let offset = usize::try_from(read_u64(bytes, entry + 16)).expect("section offset");
    let length = usize::try_from(read_u64(bytes, entry + 24)).expect("section length");
    (offset, length)
}

fn reseal(bytes: &mut [u8], sections: &[(u32, u32)]) {
    for &(kind, instance) in sections {
        let entry = directory_entry(bytes, kind, instance);
        let (offset, length) = section_span(bytes, kind, instance);
        let digest: [u8; 32] = Sha256::digest(&bytes[offset..offset + length]).into();
        bytes[entry + 32..entry + 64].copy_from_slice(&digest);
    }
    let count = usize::try_from(read_u32(bytes, 20)).expect("section count");
    let directory_end = PURREMB_HEADER_LENGTH as usize + count * DIRECTORY_ENTRY_LENGTH;
    let mut header = [0u8; PURREMB_HEADER_LENGTH as usize];
    header.copy_from_slice(&bytes[..PURREMB_HEADER_LENGTH as usize]);
    header[64..96].fill(0);
    let root = derive_artifact_root(
        &header,
        &bytes[PURREMB_HEADER_LENGTH as usize..directory_end],
    );
    bytes[64..96].copy_from_slice(root.as_bytes());
    let trailer = usize::try_from(read_u64(bytes, 48)).expect("trailer offset");
    bytes[trailer + 24..trailer + 56].copy_from_slice(root.as_bytes());
}

fn assert_structural_error(bytes: &[u8]) {
    assert!(
        EmbeddingView::from_bytes(bytes).is_err(),
        "corruption unexpectedly passed structural open"
    );
}

#[test]
fn framing_corruptions_fail_closed() {
    let original = golden();

    let mut cases = Vec::new();

    let mut unsupported_version = original.clone();
    put_u32(&mut unsupported_version, 8, 2);
    cases.push(unsupported_version);

    let mut reserved_header = original.clone();
    put_u32(&mut reserved_header, 16, 1);
    cases.push(reserved_header);

    let mut reserved_directory = original.clone();
    let first = PURREMB_HEADER_LENGTH as usize;
    put_u32(&mut reserved_directory, first + 12, 1);
    cases.push(reserved_directory);

    let mut misaligned = original.clone();
    let second = PURREMB_HEADER_LENGTH as usize + DIRECTORY_ENTRY_LENGTH;
    let second_offset = read_u64(&misaligned, second + 16);
    put_u64(&mut misaligned, second + 16, second_offset + 1);
    cases.push(misaligned);

    let mut overlapping = original.clone();
    let first_offset = read_u64(&overlapping, first + 16);
    put_u64(&mut overlapping, second + 16, first_offset);
    cases.push(overlapping);

    let mut overflowing = original.clone();
    put_u64(&mut overflowing, first + 24, u64::MAX);
    cases.push(overflowing);

    let mut bad_trailer = original.clone();
    let trailer = usize::try_from(read_u64(&bad_trailer, 48)).expect("trailer offset");
    bad_trailer[trailer] ^= 0xff;
    cases.push(bad_trailer);

    let mut root_disagreement = original.clone();
    root_disagreement[64] ^= 1;
    cases.push(root_disagreement);

    let mut critical_unknown = original.clone();
    let matrix_entry = directory_entry(&critical_unknown, SECTION_MATRIX_DATA, 1);
    put_u32(&mut critical_unknown, matrix_entry, 0x8000_0001);
    cases.push(critical_unknown);

    let mut nonzero_padding = original.clone();
    let count = usize::try_from(read_u32(&nonzero_padding, 20)).expect("section count");
    let mut changed_padding = false;
    for index in 0..count {
        let entry = PURREMB_HEADER_LENGTH as usize + index * DIRECTORY_ENTRY_LENGTH;
        let offset = usize::try_from(read_u64(&nonzero_padding, entry + 16)).expect("offset");
        let length = usize::try_from(read_u64(&nonzero_padding, entry + 24)).expect("length");
        let next = if index + 1 == count {
            usize::try_from(read_u64(&nonzero_padding, 48)).expect("trailer")
        } else {
            usize::try_from(read_u64(
                &nonzero_padding,
                entry + DIRECTORY_ENTRY_LENGTH + 16,
            ))
            .expect("next offset")
        };
        if offset + length < next {
            nonzero_padding[offset + length] = 1;
            changed_padding = true;
            break;
        }
    }
    assert!(changed_padding, "golden has at least one alignment gap");
    cases.push(nonzero_padding);

    for bytes in &cases {
        assert_structural_error(bytes);
    }
    for length in [0, 1, 7, 8, 64, 127, original.len() - 1] {
        assert_structural_error(&original[..length]);
    }
}

#[test]
fn outer_and_internal_integrity_are_independently_verified() {
    let mut forged_root = golden();
    forged_root[64] ^= 1;
    let trailer = usize::try_from(read_u64(&forged_root, 48)).expect("trailer");
    forged_root[trailer + 24] ^= 1;
    let mut view = EmbeddingView::from_bytes(&forged_root).expect("matching stored roots");
    assert!(matches!(
        verify_embedding(&mut view),
        Err(EmbeddingError::DigestMismatch { .. })
    ));

    let mut stale_section_digest = golden();
    let (matrix_offset, _) = section_span(&stale_section_digest, SECTION_MATRIX_DATA, 1);
    stale_section_digest[matrix_offset] ^= 1;
    let mut view = EmbeddingView::from_bytes(&stale_section_digest).expect("structural matrix");
    assert!(matches!(
        verify_embedding(&mut view),
        Err(EmbeddingError::DigestMismatch { .. })
    ));

    let mut stale_matrix_identity = golden();
    let (matrix_offset, _) = section_span(&stale_matrix_identity, SECTION_MATRIX_DATA, 1);
    stale_matrix_identity[matrix_offset] ^= 1;
    reseal(&mut stale_matrix_identity, &[(SECTION_MATRIX_DATA, 1)]);
    let mut view = EmbeddingView::from_bytes(&stale_matrix_identity).expect("resealed outer file");
    assert!(matches!(
        verify_embedding(&mut view),
        Err(EmbeddingError::DigestMismatch { .. })
    ));
}

#[test]
fn deep_catalog_and_projection_corruptions_are_rejected() {
    let mut target_set = golden();
    let (offset, _) = section_span(&target_set, SECTION_TARGET_SETS, 0);
    let row_pool = usize::try_from(read_u64(&target_set, offset + 40)).expect("row pool");
    target_set[offset + row_pool] ^= 0x80;
    reseal(&mut target_set, &[(SECTION_TARGET_SETS, 0)]);
    assert_structural_error(&target_set);

    let mut relation = golden();
    let (offset, _) = section_span(&relation, SECTION_RELATIONS, 0);
    relation[offset + 64 + 32] ^= 0x80;
    reseal(&mut relation, &[(SECTION_RELATIONS, 0)]);
    assert_structural_error(&relation);

    let mut token_span = golden();
    let (offset, _) = section_span(&token_span, SECTION_TOKEN_SPANS, 0);
    token_span[offset + 64 + 32] ^= 0x80;
    reseal(&mut token_span, &[(SECTION_TOKEN_SPANS, 0)]);
    assert_structural_error(&token_span);

    let mut prefix = golden();
    let (offset, _) = section_span(&prefix, SECTION_MATRICES, 0);
    let projections = usize::try_from(read_u64(&prefix, offset + 48)).expect("projections");
    put_u32(&mut prefix, offset + projections + 128, 3);
    reseal(&mut prefix, &[(SECTION_MATRICES, 0)]);
    assert_structural_error(&prefix);
}

#[test]
fn nonfinite_and_zero_norm_vectors_fail_after_outer_resealing() {
    let mut nonfinite = golden();
    let (offset, _) = section_span(&nonfinite, SECTION_MATRIX_DATA, 1);
    nonfinite[offset..offset + 4].copy_from_slice(&f32::NAN.to_le_bytes());
    reseal(&mut nonfinite, &[(SECTION_MATRIX_DATA, 1)]);
    let mut view = EmbeddingView::from_bytes(&nonfinite).expect("structural nonfinite matrix");
    assert!(matches!(
        verify_embedding(&mut view),
        Err(EmbeddingError::NonFiniteScalar { .. })
    ));

    let mut zero_norm = golden();
    let (offset, _) = section_span(&zero_norm, SECTION_MATRIX_DATA, 1);
    zero_norm[offset..offset + 16].fill(0);
    reseal(&mut zero_norm, &[(SECTION_MATRIX_DATA, 1)]);
    let mut view = EmbeddingView::from_bytes(&zero_norm).expect("structural zero row");
    assert!(matches!(
        verify_embedding(&mut view),
        Err(EmbeddingError::DigestMismatch { .. } | EmbeddingError::ZeroNorm { .. })
    ));
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut state = 0x9e37_79b9_7f4a_7c15u64;
    for length in (0..=4096).step_by(17) {
        let mut bytes = vec![0u8; length];
        for byte in &mut bytes {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            *byte = state as u8;
        }
        let outcome = std::panic::catch_unwind(|| EmbeddingView::from_bytes(&bytes));
        assert!(
            outcome.is_ok(),
            "parser panicked for {length} arbitrary bytes"
        );
    }
}

#[test]
fn hostile_counts_are_rejected_without_count_controlled_allocation() {
    let original = golden();

    let mut directory_bomb = original.clone();
    put_u32(&mut directory_bomb, 20, u32::MAX);
    let outcome = std::panic::catch_unwind(|| EmbeddingView::from_bytes(&directory_bomb));
    assert!(outcome.is_ok());
    assert!(outcome.expect("no parser panic").is_err());

    let mut target_bomb = original;
    let (target_offset, _) = section_span(&target_bomb, SECTION_TARGETS, 0);
    put_u64(&mut target_bomb, target_offset + 8, u64::MAX);
    let outcome = std::panic::catch_unwind(|| EmbeddingView::from_bytes(&target_bomb));
    assert!(outcome.is_ok());
    assert!(outcome.expect("no parser panic").is_err());
}
