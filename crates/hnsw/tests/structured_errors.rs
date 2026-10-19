// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original HNSW refusal fields across the native and WASM public boundaries.

#[path = "support/purremb.rs"]
mod purremb;

use purrdf_core::distance::{BuildShape, Exact};
use purrdf_core::{DistanceMetric, IndexId};
use purrdf_hash::error::ErrorValue;
use purrdf_hnsw::error::GuardPayloadRefusal;
use purrdf_hnsw::relation::HnswSpace;
use purrdf_hnsw::{HnswIndex, Params, VectorMatrix};
use purrdf_sparql_eval::KnnGuard;
use purrdf_testkit::error_capture::Capture;

fn matrix_and_parameter_producers_keep_original_operands() {
    let error = VectorMatrix::new(2, 3, vec![1.0]).unwrap_err();
    let fields = Capture::read(&error).child("cause");
    assert_eq!(fields.code, "hnsw-matrix-value-count");
    for (name, count) in [
        ("rows", 2),
        ("dimensions", 3),
        ("expected", 6),
        ("actual", 1),
    ] {
        assert!(matches!(fields.field(name), ErrorValue::Unsigned(value) if value == count));
    }
    let error = Params::new(1, usize::MAX, usize::MAX, 1).unwrap_err();
    let fields = Capture::read(&error);
    assert!(matches!(fields.field("name"), ErrorValue::Code("M")));
    assert!(matches!(fields.field("value"), ErrorValue::Unsigned(1)));
    assert!(matches!(fields.field("minimum"), ErrorValue::Unsigned(2)));
}

fn payload_producer_keeps_reserved_bits_after_input_disposal() {
    let matrix = VectorMatrix::new(4, 1, vec![1.0, 2.0, 3.0, 4.0]).unwrap();
    let index = HnswIndex::build(
        matrix.clone(),
        &DistanceMetric::SquaredEuclidean,
        Params::new(2, 3, 4, 4).unwrap(),
    )
    .unwrap();
    let mut image = index.canonical_image();
    purrdf_core::purremb_fixture::put_u32(&mut image, 84, u32::MAX);
    let error = HnswIndex::<Exact>::decode(matrix, &image).unwrap_err();
    drop(image);
    drop(index);
    let copy = error.clone();
    drop(error);
    let fields = Capture::read(&copy).child("reason");
    assert_eq!(fields.code, "hnsw-payload-reserved-node");
    assert!(matches!(fields.field("row"), ErrorValue::Unsigned(0)));
    assert!(
        matches!(fields.field("value"), ErrorValue::Unsigned(value) if value == u128::from(u32::MAX))
    );
}

fn guard_and_build_identities_keep_full_width_values() {
    let refusal = purrdf_hnsw::profile::parse_parameters(&[]).unwrap_err();
    let cause = Capture::read(&refusal).child("cause");
    assert_eq!(cause.code, "hnsw-profile-missing-parameter");
    assert!(matches!(cause.field("name"), ErrorValue::Code("M")));
    assert!(
        std::error::Error::source(&refusal)
            .unwrap()
            .is::<purrdf_hnsw::error::ProfileRefusal>()
    );
    let guard = IndexId::from_raw([0xa5; 32]);
    let error = GuardPayloadRefusal::Length {
        guard,
        expected: u64::MAX,
        actual: 7,
    };
    let fields = Capture::read(&error);
    assert!(
        matches!(fields.child("guard").field("bytes"), ErrorValue::Bytes(bytes) if bytes == [0xa5; 32])
    );
    assert!(
        matches!(fields.field("expected"), ErrorValue::ByteCount(value) if value == u128::from(u64::MAX))
    );
    assert!(matches!(fields.field("actual"), ErrorValue::ByteCount(7)));
    let shape = BuildShape::here();
    let fields = Capture::read(&shape);
    assert!(
        matches!(fields.field("bits"), ErrorValue::Unsigned(value) if value == u128::from(shape.bits()))
    );
    assert!(
        matches!(fields.child("identity").field("digest"), ErrorValue::Unsigned(value) if value == u128::from(shape.identity().digest()))
    );
}

fn real_artifact_setup_retains_its_native_cause_after_owners_drop() {
    let fixture = purremb::Fixture::new(4, 1, Params::new(2, 3, 4, 4).unwrap());
    let error = HnswSpace::<Exact>::from_artifact(
        &fixture.tampered_payload(),
        fixture.target_set,
        fixture.vector_space,
        fixture.bindings(),
        KnnGuard::new(4, 4).unwrap(),
    )
    .unwrap_err();
    drop(fixture);
    let copy = error.clone();
    drop(error);
    let cause = Capture::read(&copy).child("cause").child("cause");
    assert_eq!(cause.code, "hnsw-setup-embedding");
    let digest = cause.child("cause");
    assert_eq!(digest.code, "purremb-digest-mismatch");
    let mut source = std::error::Error::source(&copy);
    let original = loop {
        let current = source.expect("the original container cause remains reachable");
        if let Some(original) = current.downcast_ref::<purrdf_core::EmbeddingError>() {
            break original;
        }
        source = current.source();
    };
    let purrdf_core::EmbeddingError::DigestMismatch {
        expected, actual, ..
    } = original
    else {
        panic!("the tampered payload must retain its digest mismatch");
    };
    assert_ne!(expected, actual);
    assert!(matches!(digest.field("expected"), ErrorValue::Bytes(bytes) if bytes == expected));
    assert!(matches!(digest.field("actual"), ErrorValue::Bytes(bytes) if bytes == actual));
}

purrdf_testkit::harness_main!(
    matrix_and_parameter_producers_keep_original_operands,
    payload_producer_keeps_reserved_bits_after_input_disposal,
    guard_and_build_identities_keep_full_width_values,
    real_artifact_setup_retains_its_native_cause_after_owners_drop,
);
