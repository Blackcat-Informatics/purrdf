// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Signature precedence, coverage and trust through the generation 2 reader.

use purrdf_ed25519::EncodedVerificationError;
use purrdf_gts::v2::{
    Authenticity, HeldKey, ReadCapacity, ReadGraph, SignatureInputs, SignatureInvalid,
    SignatureStatus, SignatureUnverified, read, read_with_signature_inputs,
};
use purrdf_hash::hex;
use purrdf_lex::{cbor, json};

macro_rules! fixture {
    ($name:literal, $status:expr) => {
        (
            include_bytes!(concat!("fixtures/v2/P6/", $name, ".bin")).as_slice(),
            include_str!(concat!("fixtures/v2/P6/", $name, ".json")),
            $status,
        )
    };
}

fn inputs(decoded_result: &cbor::Value) -> SignatureInputs<'_> {
    let record = decoded_result.as_array().unwrap()[5].as_array().unwrap();
    let trusted = record[0]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_bytes().unwrap().try_into().unwrap())
        .collect();
    let held = record[1]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| {
            let key = value.as_array().unwrap();
            HeldKey {
                key_id: key[0].as_bytes().unwrap().try_into().unwrap(),
                algorithm: key[1].as_integer().unwrap().try_into().unwrap(),
                public_key: key[2].as_bytes().unwrap(),
            }
        })
        .collect();
    SignatureInputs::new(trusted, held)
}

fn pinned_report(case: &json::Value) -> cbor::Value {
    let report = hex::decode(case["read_result"].as_str().unwrap()).unwrap();
    cbor::decode(&report, cbor::Limits::default()).unwrap()
}

#[test]
fn signature_outcomes_follow_the_pinned_reader_vectors() {
    use EncodedVerificationError::{NonCanonicalScalar, SmallOrderKey};
    use SignatureInvalid::{CompositeClassical, CompositePostQuantum, Ed25519};
    use SignatureStatus::{Invalid, Valid};

    for (bytes, manifest, status) in [
        fixture!("ed25519-honest", Valid),
        fixture!(
            "ed25519-order-scalar-key",
            Invalid(Ed25519(NonCanonicalScalar))
        ),
        fixture!("ed25519-order-key-nonce", Invalid(Ed25519(SmallOrderKey))),
        fixture!("composite-honest", Valid),
        fixture!("composite-both", Invalid(CompositeClassical)),
        fixture!("composite-post-quantum", Invalid(CompositePostQuantum)),
        fixture!("cov-healed", Valid),
        fixture!("auth-commit-bad-count", Valid),
    ] {
        let manifest = json::read(manifest).unwrap();
        let case = &manifest["cases"].as_array().unwrap()[0];
        let report = pinned_report(case);
        let inputs = inputs(&report);
        let ReadGraph::Second(graph) =
            read_with_signature_inputs(bytes, ReadCapacity::default(), &inputs).unwrap()
        else {
            panic!("the vector uses generation 2");
        };
        let [frame] = graph.signatures.as_slice() else {
            panic!(
                "expected one signed frame in {}",
                manifest["name"].as_str().unwrap()
            );
        };
        let [actual] = frame.checks.as_slice() else {
            panic!("expected one signature on the frame");
        };
        let expected = case["result"]["result"].as_array().unwrap();
        assert_eq!(
            actual.status,
            status,
            "{}",
            manifest["name"].as_str().unwrap()
        );
        assert_eq!(actual.algorithm, expected[2].as_u64().unwrap());
        assert_eq!(
            actual.key_id,
            hex::decode_32(expected[3].as_str().unwrap()).unwrap()
        );
        assert_eq!(actual.coverage_first, expected[4].as_u64().unwrap());
        assert_eq!(actual.coverage_end, expected[5].as_u64().unwrap());
        assert_eq!(actual.authenticated, expected[6].as_bool().unwrap());
        if manifest["name"].as_str() == Some("auth-commit-bad-count") {
            let [(offset, commit)] = graph.invalid_commits.as_slice() else {
                panic!("expected a checked commit with a false count");
            };
            assert_eq!(*offset, frame.offset);
            assert!(!commit.count_matches);
            assert!(commit.root_matches);
        }
        let signed_segment = &graph.segment_authentication[0];
        if actual.authenticated {
            assert_eq!(signed_segment.authenticity, Authenticity::Authenticated);
            assert_eq!(signed_segment.authenticated_frames, 2);
        } else {
            assert_eq!(signed_segment.authenticity, Authenticity::Unauthenticated);
            assert_eq!(signed_segment.authenticated_frames, 0);
        }
    }
}

#[test]
fn caller_trust_changes_authentication_without_changing_signature_status() {
    let bytes = include_bytes!("fixtures/v2/P6/ed25519-honest.bin");
    let manifest = json::read(include_str!("fixtures/v2/P6/ed25519-honest.json")).unwrap();
    let case = &manifest["cases"].as_array().unwrap()[0];
    let report = pinned_report(case);
    let held = inputs(&report);
    let untrusted = SignatureInputs::new(Vec::new(), held.held_keys().to_vec());
    let ReadGraph::Second(graph) =
        read_with_signature_inputs(bytes, ReadCapacity::default(), &untrusted).unwrap()
    else {
        panic!("the vector uses generation 2");
    };
    let verified = graph.signatures[0].checks[0];
    assert_eq!(verified.status, SignatureStatus::Valid);
    assert!(!verified.authenticated);
    assert_eq!(
        graph.segment_authentication[0].authenticity,
        Authenticity::Unauthenticated
    );
    let ReadGraph::Second(graph) = read(bytes, ReadCapacity::default()).unwrap() else {
        panic!("the vector uses generation 2");
    };
    let unverified = graph.signatures[0].checks[0];
    assert_eq!(
        unverified.status,
        SignatureStatus::Unverified(SignatureUnverified::KeyNotHeld)
    );
    assert_eq!(unverified.coverage_first, unverified.coverage_end);
    assert!(!unverified.authenticated);
}

#[test]
fn a_signature_after_detached_recovery_covers_only_its_new_run() {
    let bytes = include_bytes!("fixtures/v2/P6/sig-detached-frame.bin");
    let manifest = json::read(include_str!("fixtures/v2/P6/sig-detached-frame.json")).unwrap();
    let report = pinned_report(&manifest["cases"].as_array().unwrap()[0]);
    let inputs = inputs(&report);
    let ReadGraph::Second(graph) =
        read_with_signature_inputs(bytes, ReadCapacity::default(), &inputs).unwrap()
    else {
        panic!("the vector uses generation 2");
    };
    let [recovery] = graph.parity_recoveries.as_slice() else {
        panic!("expected the detached parity recovery");
    };
    assert!(recovery.detached);
    let [frame] = graph.signatures.as_slice() else {
        panic!("expected one signed frame after the recovery");
    };
    let [signature] = frame.checks.as_slice() else {
        panic!("expected one signature on the detached frame");
    };
    assert_eq!(signature.status, SignatureStatus::Valid);
    assert_eq!((signature.coverage_first, signature.coverage_end), (1, 2));
    assert!(signature.authenticated);
    let summary = &graph.segment_authentication[0];
    assert_eq!(summary.authenticated_frames, 1);
    assert_eq!(summary.authenticity, Authenticity::PartlyAuthenticated);
}

#[test]
fn a_trusted_commit_covers_linked_frames_across_a_break() {
    let bytes = include_bytes!("fixtures/v2/P6/auth-broken-middle.bin");
    let manifest = json::read(include_str!("fixtures/v2/P6/auth-broken-middle.json")).unwrap();
    let report = pinned_report(&manifest["cases"].as_array().unwrap()[0]);
    let inputs = inputs(&report);
    let ReadGraph::Second(graph) =
        read_with_signature_inputs(bytes, ReadCapacity::default(), &inputs).unwrap()
    else {
        panic!("the vector uses generation 2");
    };
    let summary = &graph.segment_authentication[0];
    assert_eq!(summary.authenticated_frames, 3);
    assert_eq!(summary.authenticity, Authenticity::PartlyAuthenticated);
    let [_, second_commit] = graph.signatures.as_slice() else {
        panic!("expected signatures on both commits");
    };
    let authenticating = second_commit
        .checks
        .iter()
        .find(|check| check.authenticated)
        .unwrap();
    assert_eq!(
        (authenticating.coverage_first, authenticating.coverage_end),
        (3, 4)
    );
}

#[test]
fn an_unsigned_index_leaves_a_fully_authenticated_segment_authenticated() {
    let bytes = include_bytes!("fixtures/v2/P6/auth-two-commits-two-signers.bin");
    let manifest = json::read(include_str!(
        "fixtures/v2/P6/auth-two-commits-two-signers.json"
    ))
    .unwrap();
    let report = pinned_report(&manifest["cases"].as_array().unwrap()[0]);
    let inputs = inputs(&report);
    let ReadGraph::Second(graph) =
        read_with_signature_inputs(bytes, ReadCapacity::default(), &inputs).unwrap()
    else {
        panic!("the vector uses generation 2");
    };
    let [index] = graph.index_checks.as_slice() else {
        panic!("expected the closing index");
    };
    let ReadGraph::Second(prefix) =
        read_with_signature_inputs(&bytes[..index.offset], ReadCapacity::default(), &inputs)
            .unwrap()
    else {
        panic!("the prefix uses generation 2");
    };
    assert_eq!(graph.segment_authentication, prefix.segment_authentication);
    let summary = &graph.segment_authentication[0];
    assert_eq!(summary.authenticity, Authenticity::Authenticated);
    assert_eq!(summary.authenticated_frames, 5);
}
