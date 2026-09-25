// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Decoder hostility: every malformed canonical image is a loud, typed
//! [`HnswError`], never a panic and never a process abort.
//!
//! The canonical image is the payload a PURREMB guard commits and the bytes a
//! determinism digest folds, so decoding it is a trust boundary: the bytes must
//! never be assumed to come from this encoder. This suite feeds the decoder
//! truncated, overlong, mis-versioned, mis-parameterized and structurally corrupt
//! images and asserts three things about each:
//!
//! 1. it is **rejected** (accepted corruption is the worst outcome);
//! 2. the rejection is a **typed error**, not a panic — every case runs inside
//!    [`std::panic::catch_unwind`], so a panic fails the test rather than merely
//!    unwinding through it;
//! 3. no length field can steer an allocation, because a count that the remaining
//!    bytes cannot hold is refused *before* the allocator sees it. A hostile count
//!    large enough to abort the process would not be caught by `catch_unwind`.
//!
//! The canonical image carries the graph and its identity but **not** the vector
//! dimension: the vectors are the source data the graph was derived from, so the
//! decoder checks that the matrix has one row per node, while binding the full
//! matrix coordinate (including `prefix_dimension`) is the PURREMB guard's job.
//! This suite therefore covers the decoder's own trust boundary; the guard's is
//! covered by the PURREMB round-trip suite that ships with the guard adapter.

use purrdf_core::DistanceMetric;
use purrdf_hnsw::level::splitmix64;
use purrdf_hnsw::{HnswError, HnswIndex, Params, VectorMatrix};

/// A deterministic fixture, generated from a splitmix64 integer stream.
fn fixture(rows: usize, dims: usize) -> VectorMatrix {
    let mut state = 0x0bad_c0de_0bad_c0de_u64;
    let mut data = Vec::with_capacity(rows * dims);
    for _ in 0..rows * dims {
        state = splitmix64(state);
        let unit = (state >> 11) as f64 / (1_u64 << 53) as f64;
        let value = unit.mul_add(2.0, -1.0);
        data.push(if value == 0.0 { 0.25 } else { value });
    }
    VectorMatrix::new(rows, dims, data).expect("the fixture shape is valid")
}

/// A small index and its canonical image, the baseline every mutation starts from.
fn baseline() -> (VectorMatrix, Vec<u8>) {
    let matrix = fixture(64, 8);
    let index = HnswIndex::build(
        matrix.clone(),
        &DistanceMetric::SquaredEuclidean,
        Params::new(4, 8, 16, 8).expect("valid"),
    )
    .expect("the baseline builds");
    let image = index.canonical_image();
    (matrix, image)
}

/// Assert that `bytes` decode against `matrix` to a typed error and do not panic.
fn assert_rejected(matrix: VectorMatrix, bytes: &[u8]) -> HnswError {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        HnswIndex::decode(matrix, bytes)
    })) {
        Ok(Ok(_)) => panic!("a hostile payload decoded successfully; it must be rejected"),
        Ok(Err(error)) => error,
        Err(cause) => {
            drop(cause);
            panic!("the decoder panicked on a hostile payload; it must return a typed error")
        }
    }
}

/// Whether a rejection is the structural [`HnswError::InvalidPayload`] variant.
fn is_invalid_payload(error: &HnswError) -> bool {
    matches!(error, HnswError::InvalidPayload { .. })
}

fn read_u32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().expect("four bytes"))
}

fn read_u64(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(bytes[at..at + 8].try_into().expect("eight bytes"))
}

fn write_u32(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u64(bytes: &mut [u8], at: usize, value: u64) {
    bytes[at..at + 8].copy_from_slice(&value.to_le_bytes());
}

/// The offset of the first adjacency list holding at least two neighbours, and its
/// count. The fixture is dense enough that one always exists.
fn first_dense_neighbour_list(image: &[u8]) -> (usize, usize) {
    let node_count = read_u64(image, 48) as usize;
    let mut at = 72;
    for _ in 0..node_count {
        at += 8; // row
        let level = read_u32(image, at);
        at += 8; // level + reserved
        for _ in 0..=level {
            at += 8; // layer + reserved
            let count = read_u64(image, at) as usize;
            at += 8;
            if count >= 2 {
                return (at, count);
            }
            at += count * 16;
        }
    }
    panic!("the baseline has no adjacency list with two neighbours");
}

/// The entry point the decoder recomputes: the minimum row at the maximum level.
fn recomputed_entry(image: &[u8]) -> usize {
    let node_count = read_u64(image, 48) as usize;
    let mut at = 72;
    let mut levels = Vec::with_capacity(node_count);
    for _ in 0..node_count {
        at += 8; // row
        let level = read_u32(image, at);
        at += 8; // level + reserved
        levels.push(level);
        for _ in 0..=level {
            at += 8; // layer + reserved
            let count = read_u64(image, at) as usize;
            at += 8 + count * 16;
        }
    }
    let max_level = levels.iter().copied().max().unwrap_or(0);
    levels
        .iter()
        .enumerate()
        .filter(|(_, level)| **level == max_level)
        .map(|(row, _)| row)
        .min()
        .expect("a non-empty graph has an entry")
}

#[test]
fn truncated_overlong_and_mis_versioned_payloads_are_typed_errors() {
    let (matrix, image) = baseline();

    assert!(is_invalid_payload(&assert_rejected(matrix.clone(), &[])));
    assert!(is_invalid_payload(&assert_rejected(
        matrix.clone(),
        &image[..image.len() - 1]
    )));

    let mut trailing = image.clone();
    trailing.push(0);
    assert!(is_invalid_payload(&assert_rejected(
        matrix.clone(),
        &trailing
    )));

    let mut bad_magic = image.clone();
    bad_magic[0] ^= 0xff;
    assert!(is_invalid_payload(&assert_rejected(
        matrix.clone(),
        &bad_magic
    )));

    // A version-1 image (its header's arithmetic field was reserved and zero) and a
    // version from the future are both refused by name.
    let mut version_one = image.clone();
    write_u32(&mut version_one, 8, 1);
    write_u32(&mut version_one, 60, 0);
    assert_eq!(
        assert_rejected(matrix.clone(), &version_one),
        HnswError::VersionMismatch {
            expected: 2,
            actual: 1
        }
    );
    let mut bad_version = image;
    write_u32(&mut bad_version, 8, 3);
    assert_eq!(
        assert_rejected(matrix, &bad_version),
        HnswError::VersionMismatch {
            expected: 2,
            actual: 3
        }
    );
}

#[test]
fn a_header_naming_another_arithmetic_is_refused_by_name() {
    let (matrix, image) = baseline();
    // The field version 1 reserved as zero records the arithmetic now: 1 is the exact one.
    assert_eq!(
        read_u32(&image, 60),
        1,
        "the baseline records the exact arithmetic"
    );
    for code in [0_u32, 2, u32::MAX] {
        let mut other = image.clone();
        write_u32(&mut other, 60, code);
        assert_eq!(
            assert_rejected(matrix.clone(), &other),
            HnswError::ArithmeticMismatch {
                arithmetic: "binary64-lane16-tree-v1",
                actual: code
            },
            "arithmetic code {code}"
        );
    }
    // The valid neighbour: the baseline itself, carrying code 1, decodes.
    assert!(HnswIndex::decode(matrix, &image).is_ok());
}

#[test]
fn non_zero_reserved_fields_are_refused() {
    let (matrix, image) = baseline();

    let mut node_reserved = image.clone();
    write_u32(&mut node_reserved, 84, 1);
    assert!(is_invalid_payload(&assert_rejected(
        matrix.clone(),
        &node_reserved
    )));

    let mut layer_reserved = image;
    write_u32(&mut layer_reserved, 92, 1);
    assert!(is_invalid_payload(&assert_rejected(
        matrix,
        &layer_reserved
    )));
}

#[test]
fn a_hostile_node_count_is_refused_before_any_allocation() {
    let (matrix, mut image) = baseline();
    write_u64(&mut image, 48, u64::MAX);
    assert!(is_invalid_payload(&assert_rejected(matrix, &image)));
}

#[test]
fn a_hostile_neighbour_count_is_refused_before_any_allocation() {
    let (matrix, mut image) = baseline();
    let (neighbours_at, _count) = first_dense_neighbour_list(&image);
    write_u64(&mut image, neighbours_at - 8, u64::MAX);
    assert!(is_invalid_payload(&assert_rejected(matrix, &image)));
}

#[test]
fn unsorted_and_duplicate_adjacency_are_refused() {
    let (matrix, image) = baseline();
    let (neighbours_at, _count) = first_dense_neighbour_list(&image);
    let first = read_u64(&image, neighbours_at);
    let second = read_u64(&image, neighbours_at + 16);

    let mut unsorted = image.clone();
    write_u64(&mut unsorted, neighbours_at, second);
    write_u64(&mut unsorted, neighbours_at + 16, first);
    assert!(is_invalid_payload(&assert_rejected(
        matrix.clone(),
        &unsorted
    )));

    let mut duplicate = image;
    write_u64(&mut duplicate, neighbours_at + 16, first);
    assert!(is_invalid_payload(&assert_rejected(matrix, &duplicate)));
}

#[test]
fn non_finite_distances_are_refused() {
    let (matrix, mut image) = baseline();
    let (neighbours_at, _count) = first_dense_neighbour_list(&image);
    write_u64(&mut image, neighbours_at + 8, f64::NAN.to_bits());
    assert!(is_invalid_payload(&assert_rejected(matrix, &image)));
}

#[test]
fn a_wrong_entry_point_is_refused() {
    let (matrix, mut image) = baseline();
    let correct = recomputed_entry(&image);
    let node_count = read_u64(&image, 48) as usize;
    let wrong = (correct + 1) % node_count;
    write_u64(&mut image, 64, wrong as u64);
    assert!(is_invalid_payload(&assert_rejected(matrix, &image)));
}

#[test]
fn bad_parameters_are_refused() {
    let (matrix, image) = baseline();

    let mut bad_m = image.clone();
    write_u64(&mut bad_m, 16, 1);
    assert!(matches!(
        assert_rejected(matrix.clone(), &bad_m),
        HnswError::InvalidParameter { name: "M", .. }
    ));

    let mut m0_below_m = image.clone();
    write_u64(&mut m0_below_m, 16, 4);
    write_u64(&mut m0_below_m, 24, 2);
    assert!(matches!(
        assert_rejected(matrix.clone(), &m0_below_m),
        HnswError::InvalidParameter { name: "M0", .. }
    ));

    // A degree bound smaller than an adjacency list the image already holds: every
    // structural field is well formed, yet the graph violates its own identity.
    let mut tight = image;
    write_u64(&mut tight, 16, 2);
    write_u64(&mut tight, 24, 2);
    assert!(is_invalid_payload(&assert_rejected(matrix, &tight)));
}

#[test]
fn a_matrix_of_the_wrong_shape_is_refused() {
    let (matrix, image) = baseline();
    let wider = fixture(matrix.rows() + 1, matrix.dims());
    assert!(is_invalid_payload(&assert_rejected(wider, &image)));
}
