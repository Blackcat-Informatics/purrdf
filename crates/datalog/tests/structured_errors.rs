// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original term and proof refusal data through the portable public API.

use purrdf_datalog::term::{RetainedTerm, TermDag};
use purrdf_hash::error::ErrorValue;
use purrdf_testkit::error_capture::Capture;

fn reachable_snapshot_preserves_sharing_binders_and_deep_source_death() {
    let mut dag = TermDag::default();
    let op = dag.intern_leaf("operator").unwrap();
    let body = dag.intern_bound(u32::MAX, u16::MAX).unwrap();
    let binder = dag.intern_binder(op, vec![op], body).unwrap();
    let unreachable = dag.intern_leaf("unreachable").unwrap();
    let mut root = binder;
    for _ in 0..10_000 {
        root = dag.intern_app(op, vec![root, binder]).unwrap();
    }
    let snapshot = RetainedTerm::capture(&dag, root).unwrap();
    drop(dag);
    let clone = snapshot.clone();
    drop(snapshot);
    let captured = Capture::read(&clone);
    assert!(
        matches!(captured.field("root"), ErrorValue::Unsigned(value) if value == u128::from(root.index()))
    );
    let nodes = captured.records("nodes");
    assert_eq!(nodes.len(), 10_003);
    assert!(!nodes.iter().any(|node| matches!(node.field("id"), ErrorValue::Unsigned(value) if value == u128::from(unreachable.index()))));
    assert!(
        !nodes
            .iter()
            .any(|node| matches!(node.field("symbol"), ErrorValue::Text("unreachable")))
    );
    let bound = nodes
        .iter()
        .map(|node| node.child("data"))
        .find(|node| matches!(node.field("kind"), ErrorValue::Code("bound")))
        .unwrap();
    assert!(
        matches!(bound.field("debruijn"), ErrorValue::Unsigned(value) if value == u128::from(u32::MAX))
    );
    assert!(
        matches!(bound.field("slot"), ErrorValue::Unsigned(value) if value == u128::from(u16::MAX))
    );
}

fn actual_proof_decoder_preserves_rejected_bytes_and_native_lengths() {
    use purrdf_datalog::proof::{ProofArena, ProofMalformed};

    let error = ProofArena::<4>::decode(&[]).unwrap_err();
    let fields = Capture::read(&error).child("detail");
    assert_eq!(fields.code, "proof-truncated");
    assert!(matches!(fields.field("required"), ErrorValue::ByteCount(8)));
    assert!(matches!(
        fields.field("remaining"),
        ErrorValue::ByteCount(0)
    ));
    let error = ProofArena::<4>::decode(&u64::MAX.to_le_bytes()).unwrap_err();
    let fields = Capture::read(&error).child("detail");
    if usize::BITS < 64 {
        assert_eq!(fields.code, "proof-length");
        assert!(
            matches!(fields.field("value"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
        );
        assert!(
            matches!(fields.field("maximum"), ErrorValue::Unsigned(value) if value == usize::MAX as u128)
        );
    } else {
        assert_eq!(fields.code, "proof-truncated");
        assert!(
            matches!(fields.field("required"), ErrorValue::ByteCount(value) if value == u128::from(u64::MAX))
        );
        assert!(matches!(
            fields.field("remaining"),
            ErrorValue::ByteCount(0)
        ));
    }
    let mut input = Vec::new();
    purrdf_hash::frame::frame_le(&mut input, &[0xff, 0, 0xab]);
    let error = ProofArena::<4>::decode(&input).unwrap_err();
    drop(input);
    let copy = error.clone();
    drop(error);
    let fields = Capture::read(&copy).child("detail");
    assert_eq!(fields.code, "proof-encoding-tag");
    assert!(matches!(fields.field("actual"), ErrorValue::Bytes(bytes) if bytes == [0xff, 0, 0xab]));
    assert!(
        std::error::Error::source(&copy)
            .unwrap()
            .is::<ProofMalformed>()
    );
}

purrdf_testkit::harness_main!(
    reachable_snapshot_preserves_sharing_binders_and_deep_source_death,
    actual_proof_decoder_preserves_rejected_bytes_and_native_lengths,
);
