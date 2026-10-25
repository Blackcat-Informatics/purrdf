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

#[global_allocator]
static GLOBAL: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

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

    let root_events = Vec::with_capacity(4);
    let cause_events = Vec::with_capacity(8);
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let error = VectorMatrix::new(usize::MAX, 2, Vec::new()).unwrap_err();
    let root = Capture::read_with_buffer(&error, root_events);
    let ErrorValue::Cause(cause) = root.field("cause") else {
        panic!("arithmetic refusal must retain its native cause");
    };
    let fields = Capture::read_with_buffer(cause, cause_events);
    let measured = window.close();
    assert_eq!(measured.allocations, 0);
    assert_eq!(measured.requested_bytes, 0);
    assert_eq!(root.code, "hnsw-arithmetic-overflow");
    assert_eq!(fields.code, "hnsw-arithmetic-matrix-elements");
    assert!(matches!(
        fields.field("operation"),
        ErrorValue::Code("multiply")
    ));
    assert!(
        matches!(fields.field("rows"), ErrorValue::Unsigned(value) if value == usize::MAX as u128)
    );
    assert!(matches!(
        fields.field("dimensions"),
        ErrorValue::Unsigned(2)
    ));
    assert!(
        matches!(fields.field("maximum"), ErrorValue::Unsigned(value) if value == usize::MAX as u128)
    );
    assert!(
        std::error::Error::source(&error)
            .unwrap()
            .is::<purrdf_hnsw::error::ArithmeticRefusal>()
    );
    assert_eq!(error.to_string(), "index arithmetic overflowed");

    let params = Params::new(2, 2, 2, 1).unwrap();
    let error = params.validate_against(usize::MAX, 2).unwrap_err();
    assert_eq!(
        Capture::read(&error).child("cause").code,
        "hnsw-arithmetic-matrix-elements"
    );
    let error = params.validate_against(usize::MAX, 1).unwrap_err();
    let fields = Capture::read(&error).child("cause");
    assert_eq!(fields.code, "hnsw-arithmetic-matrix-bytes");
    assert!(
        matches!(fields.field("elements"), ErrorValue::Unsigned(value) if value == usize::MAX as u128)
    );
    assert!(matches!(
        fields.field("element_bytes"),
        ErrorValue::ByteCount(8)
    ));
    assert!(
        matches!(fields.field("maximum"), ErrorValue::ByteCount(value) if value == usize::MAX as u128)
    );
    VectorMatrix::new(2, 3, vec![1.0; 6]).unwrap();
    params.validate_against(2, 3).unwrap();
    assert_eq!(params.footprint_bytes_32(1, 1).unwrap(), 72);

    #[cfg(target_pointer_width = "64")]
    {
        let error = params.footprint_bytes_32(usize::MAX, 2).unwrap_err();
        let fields = Capture::read(&error).child("cause");
        assert_eq!(fields.code, "hnsw-arithmetic-vector-elements");
        assert!(
            matches!(fields.field("rows"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
        );
        assert!(matches!(
            fields.field("dimensions"),
            ErrorValue::Unsigned(2)
        ));
        let error = params.footprint_bytes_32(usize::MAX, 1).unwrap_err();
        let fields = Capture::read(&error).child("cause");
        assert_eq!(fields.code, "hnsw-arithmetic-vector-bytes");
        assert!(
            matches!(fields.field("elements"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
        );
        assert!(matches!(
            fields.field("element_bytes"),
            ErrorValue::ByteCount(8)
        ));

        // For powers of two the complete population sum is geometric. Every
        // component fits; adding the adjacency to the prior bytes is the refusal.
        let error = params.footprint_bytes_32(1_usize << 57, 5).unwrap_err();
        let fields = Capture::read(&error).child("cause");
        assert_eq!(fields.code, "hnsw-arithmetic-edge-sum");
        assert!(matches!(fields.field("operation"), ErrorValue::Code("add")));
        assert!(
            matches!(fields.field("accumulated"), ErrorValue::ByteCount(value) if value == 9_u128 << 60)
        );
        assert!(
            matches!(fields.field("edges"), ErrorValue::ByteCount(value) if value == (1_u128 << 63) - 32)
        );
        assert!(
            matches!(fields.field("maximum"), ErrorValue::ByteCount(value) if value == u128::from(u64::MAX))
        );
    }
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
