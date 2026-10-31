// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Adversarial checks of the actual new proof term, not a producer cache boolean.

use super::*;
use crate::reasoner::schema_tests::{Fixture, shared_schema};
use crate::reasoner::{Reasoner, Verdict};
use purrdf_alloc_probe::CurrentThreadWindow;
use purrdf_iri::vocab::rdfs;

/// Build a derived cardinality clash together with its owning prepared context.
fn derived() -> (DlProof, DlProofContext<'static>) {
    let mut fixture = Fixture::default();
    let class = fixture.name("A");
    let filler = fixture.name("F");
    let role = fixture.name("p");
    fixture.bounds(class, role, filler, 2, true, false);
    fixture.bounds(class, role, filler, 1, false, false);
    let edge = fixture.name("edge");
    let subject = fixture.name("subject");
    let object = fixture.name("i");
    fixture.edge(edge, rdfs::RANGE, class);
    fixture
        .builder
        .push_quad(subject, edge, object, None)
        .unwrap();
    let dataset = fixture.freeze();
    let reasoner = Reasoner::with_proofs(
        &dataset,
        &mut purrdf_core::CanonicalizationControl::new(purrdf_core::RDFC_V2_CORPUS_POLICY, None),
    )
    .unwrap();
    let answer = reasoner.consistency();
    assert_eq!(*answer.answer(), Verdict::False);
    let proof = answer
        .proof()
        .unwrap()
        .runs()
        .last()
        .unwrap()
        .proof()
        .unwrap()
        .clone();
    let context = reasoner.proof_context().unwrap();
    assert_eq!(
        proof
            .replay_refutation(&context)
            .unwrap()
            .checks()
            .unattested(),
        0
    );
    (proof, context)
}

fn evidence(proof: &mut DlProof) -> &mut SchemaClashEvidence {
    proof
        .clashes
        .iter_mut()
        .find_map(|step| step.schema.as_mut())
        .expect("real recorded schema clash")
}

#[test]
fn legacy_v4_prepared_prefixes_use_the_same_current_support_checker() {
    let (proof, context) = derived();
    let bytes = proof.encode();
    let mut reader = Reader::new(&bytes);
    assert_eq!(
        reader.frame().unwrap(),
        CURRENT_PROOF_ENCODING_TAG.as_bytes()
    );
    reader.digest().unwrap();
    reader.digest().unwrap();
    reader.byte().unwrap();
    assert!(!reader.flag().unwrap());
    assert_eq!(reader.byte().unwrap(), 0, "unrefused fixture");
    for _ in 0..2 {
        for _ in 0..reader.length().unwrap() {
            reader.length().unwrap();
        }
    }
    let header_end = bytes.len() - reader.bytes.len();
    let mut legacy = bytes[..header_end].to_vec();
    legacy[8 + PROOF_ENCODING_TAG.len() - 1] = b'4';
    for _ in 0..reader.length().unwrap() {
        if reader.flag().unwrap() {
            reader.length().unwrap();
        }
        schema::decode_application(&mut reader).unwrap();
    }
    let count = reader.length().unwrap();
    legacy.extend_from_slice(&(count as u64).to_le_bytes());
    for step in &proof.clashes {
        let start = bytes.len() - reader.bytes.len();
        reader.length().unwrap();
        reader.node().unwrap();
        for _ in 0..reader.length().unwrap() {
            reader.node().unwrap();
        }
        for _ in 0..reader.length().unwrap() {
            reader.fact().unwrap();
        }
        assert!(reader.flag().unwrap(), "prepared bound law");
        decode_schema_clash(&mut reader).unwrap();
        legacy.extend_from_slice(&bytes[start..bytes.len() - reader.bytes.len()]);
        assert!(reader.flag().unwrap(), "current support");
        if reader.flag().unwrap() {
            reader.length().unwrap();
        }
        schema::decode_frame(&mut reader).unwrap();
        let current = step.current.as_ref().unwrap();
        schema::encode_frame(&mut legacy, &current.frame);
        let applications =
            crate::owl_dl::support::prefix(&proof.support, current.latest, None).unwrap();
        legacy.extend_from_slice(&(applications.len() as u64).to_le_bytes());
        for application in applications {
            schema::encode_application(&mut legacy, application);
        }
    }
    legacy.extend_from_slice(reader.bytes);
    let decoded = DlProof::decode(&legacy).unwrap();
    assert_eq!(
        decoded
            .replay_refutation(&context)
            .unwrap()
            .checks()
            .unattested(),
        0
    );
    assert_eq!(
        decoded.encode(),
        bytes,
        "v4 normalizes into the shared v5 arena"
    );
    let mut forged = decoded;
    forged.support[0].application.head.clear();
    assert!(forged.replay_refutation(&context).is_err());
}

#[test]
fn schema_layout_without_its_evidence_is_not_a_second_encoding_of_a_legacy_proof() {
    let dataset = shared_schema(1, 0, false);
    let reasoner = Reasoner::with_proofs(
        &dataset,
        &mut purrdf_core::CanonicalizationControl::new(purrdf_core::RDFC_V2_CORPUS_POLICY, None),
    )
    .unwrap();
    let answer = reasoner.consistency();
    let proof = answer
        .proof()
        .unwrap()
        .runs()
        .last()
        .unwrap()
        .proof()
        .unwrap();
    assert_eq!(proof.clashes(), []);
    let mut bytes = proof.encode();
    let tag_end = 8 + PROOF_ENCODING_TAG.len();
    assert_eq!(bytes[tag_end - 1], b'3');
    bytes[tag_end - 1] = b'4';
    bytes.insert(tag_end + 32 + 32 + 2, 0);
    assert!(matches!(
        DlProof::decode(&bytes),
        Err(DlProofError::Malformed { .. })
    ));
}

#[test]
fn newer_layout_without_its_evidence_is_not_a_second_encoding_of_a_current_proof() {
    let (proof, _) = derived();
    assert_eq!(proof.merges, []);
    assert_eq!(proof.branches, []);
    let bytes = proof.encode();
    let tag_end = 8 + PROOF_ENCODING_TAG.len();
    assert_eq!(bytes[tag_end - 1], b'5');
    assert_eq!(DlProof::decode(&bytes).unwrap().encode(), bytes);
    for version in *b"67" {
        let mut forged = bytes.clone();
        forged[tag_end - 1] = version;
        assert!(
            matches!(
                DlProof::decode(&forged),
                Err(DlProofError::Malformed { .. })
            ),
            "v{} must require its own native evidence",
            char::from(version)
        );
    }
}

#[test]
fn refusing_native_support_keeps_search_parity_and_a_typed_incomplete_proof() {
    let mut fixture = Fixture::default();
    let class = fixture.name("A");
    let filler = fixture.name("F");
    let role = fixture.name("p");
    fixture.bounds(class, role, filler, 2, true, false);
    fixture.bounds(class, role, filler, 1, false, false);
    let edge = fixture.name("edge");
    let subject = fixture.name("s");
    let object = fixture.name("i");
    fixture.edge(edge, rdfs::RANGE, class);
    fixture
        .builder
        .push_quad(subject, edge, object, None)
        .unwrap();
    let dataset = fixture.freeze();
    let plain = Reasoner::new(&dataset).unwrap().consistency();
    let mut refused = Reasoner::with_proofs(
        &dataset,
        &mut purrdf_core::CanonicalizationControl::new(purrdf_core::RDFC_V2_CORPUS_POLICY, None),
    )
    .unwrap();
    refused.set_schema_recording_budget(crate::reasoner::SchemaPreparationBudget {
        bytes: Some(0),
        work: None,
    });
    let answer = refused.consistency();
    assert_eq!(answer.answer(), plain.answer());
    assert_eq!(
        answer.certificate(),
        plain.certificate(),
        "recording is never a search or budget lever"
    );
    let context = refused.proof_context().unwrap();
    let trace = answer
        .proof()
        .unwrap()
        .runs()
        .last()
        .unwrap()
        .proof()
        .unwrap();
    let obstruction = trace
        .schema_recording_obstruction()
        .expect("actual metadata admission failed");
    assert!(matches!(
        obstruction,
        crate::reasoner::SchemaObstruction::Storage { limit: 0, .. }
    ));
    assert!(trace.truncated());
    let decoded = DlProof::decode(&trace.encode()).unwrap();
    assert_eq!(decoded.schema_recording_obstruction(), Some(obstruction));
    assert_eq!(decoded.encode(), trace.encode());
    assert_eq!(
        decoded.replay_refutation(&context).unwrap_err(),
        DlProofError::SchemaRecording { obstruction }
    );
}

#[test]
fn derived_membership_cannot_be_laundered_as_an_asserted_fact() {
    let (proof, context) = derived();
    assert_ne!(proof.support, []);
    let mut forged = proof.clone();
    forged.clashes.iter_mut().for_each(|step| {
        if let Some(current) = &mut step.current {
            current.latest = None;
        }
    });
    assert!(
        forged.replay_refutation(&context).is_err(),
        "a valid structural bound does not prove current membership"
    );
    let mut forged = proof.clone();
    forged
        .clashes
        .iter_mut()
        .find_map(|step| step.current.as_mut())
        .unwrap()
        .frame[0] = usize::MAX;
    assert!(forged.replay_refutation(&context).is_err());
    let mut forged = proof;
    forged.support[0].application.frame[0] = usize::MAX;
    assert!(forged.replay_refutation(&context).is_err());
}

/// A prepared owning source preserves caller cancellation as an operational refusal.
#[test]
fn the_consumers_stop_signal_preserves_operational_refusal_instead_of_a_bad_proof_verdict() {
    #[derive(Debug)]
    struct Stop(std::sync::atomic::AtomicBool);
    impl purrdf_datalog::StopSignal for Stop {
        /// Read the fixture's live cancellation flag during native replay.
        fn stopped(&self) -> bool {
            self.0.load(std::sync::atomic::Ordering::Relaxed)
        }
    }
    let (proof, mut context) = derived();
    let stop = std::sync::Arc::new(Stop(std::sync::atomic::AtomicBool::new(true)));
    let OwnedOrBorrowed::Owned(kb) = &mut context.kb else {
        panic!("the independently prepared fixture owns its knowledge base");
    };
    kb.stop = Some(stop.clone());
    assert_eq!(
        proof.replay_refutation(&context).unwrap_err(),
        DlProofError::Stopped
    );
    stop.0.store(false, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(
        proof
            .replay_refutation(&context)
            .unwrap()
            .checks()
            .unattested(),
        0
    );
}

#[test]
fn source_restrictions_qualifiers_and_each_actual_support_head_are_checked() {
    let (proof, context) = derived();
    let mut changed = proof.clone();
    evidence(&mut changed).bounds.lower = u32::MAX;
    assert!(changed.replay_refutation(&context).is_err());
    let mut changed = proof.clone();
    evidence(&mut changed).bounds.class_steps.clear();
    assert!(changed.replay_refutation(&context).is_err());
    let mut changed = proof.clone();
    let first = &mut changed.support.first_mut().unwrap().application;
    first.head = vec![Ground::Concept(first.frame[0], u32::MAX)];
    assert!(
        changed.replay_refutation(&context).is_err(),
        "malformed head indices refuse before graph access"
    );
    let mut changed = proof;
    evidence(&mut changed).bounds.qualifier = crate::owl_dl::bounds::QualifierProof::Data;
    assert!(
        changed.replay_refutation(&context).is_err(),
        "object membership is not datatype containment"
    );
}

#[test]
fn retained_schema_proof_clones_release_their_original_allocations_after_the_context_dies() {
    let dataset = shared_schema(2, 1, true);
    let window = CurrentThreadWindow::open();
    {
        let reasoner = Reasoner::with_proofs(
            &dataset,
            &mut purrdf_core::CanonicalizationControl::new(
                purrdf_core::RDFC_V2_CORPUS_POLICY,
                None,
            ),
        )
        .unwrap();
        let answer = reasoner.consistency();
        let proof = answer.proof().unwrap().clone();
        let clone = proof.clone();
        let context = reasoner.proof_context().unwrap();
        proof
            .verify(
                &dataset,
                proof.question(),
                Some(answer.certificate()),
                &context,
                &mut purrdf_core::CanonicalizationControl::new(
                    purrdf_core::RDFC_V2_CORPUS_POLICY,
                    None,
                ),
            )
            .unwrap();
        drop(context);
        drop(answer);
        drop(proof);
        // Extraction remains an owned canonical term after its producer/checker owners die.
        assert_eq!(
            crate::reasoner::ServiceProof::decode(&clone.encode())
                .unwrap()
                .encode(),
            clone.encode()
        );
        assert!(window.sample().retained_bytes > 0);
    }
    assert_eq!(window.sample().retained_bytes, 0);
    assert_eq!(std::sync::Arc::strong_count(&dataset), 1);
}
