// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Adversarial checks of the actual new proof term, not a producer cache boolean.

use super::*;
use crate::reasoner::schema_tests::{Fixture, shared_schema};
use crate::reasoner::{Reasoner, Verdict};
use purrdf_alloc_probe::CurrentThreadWindow;
use purrdf_iri::vocab::rdfs;

fn derived() -> (DlProof, DlProofContext) {
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
    fixture.builder.push_quad(subject, edge, object, None);
    let dataset = fixture.freeze();
    let reasoner = Reasoner::with_proofs(&dataset).unwrap();
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
fn schema_layout_without_its_evidence_is_not_a_second_encoding_of_a_legacy_proof() {
    let dataset = shared_schema(1, 0, false);
    let reasoner = Reasoner::with_proofs(&dataset).unwrap();
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
    fixture.builder.push_quad(subject, edge, object, None);
    let dataset = fixture.freeze();
    let plain = Reasoner::new(&dataset).unwrap().consistency();
    let mut refused = Reasoner::with_proofs(&dataset).unwrap();
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
    assert!(proof.clashes.iter().any(|step| {
        step.schema
            .as_ref()
            .is_some_and(|evidence| !evidence.support.is_empty())
    }));
    let mut forged = proof.clone();
    evidence(&mut forged).support.clear();
    assert!(
        forged.replay_refutation(&context).is_err(),
        "a valid structural bound does not prove current membership"
    );
    let mut forged = proof.clone();
    evidence(&mut forged).frame[0] = usize::MAX;
    assert!(forged.replay_refutation(&context).is_err());
    let mut forged = proof;
    evidence(&mut forged).support[0].frame[0] = usize::MAX;
    assert!(forged.replay_refutation(&context).is_err());
}

#[test]
fn the_consumers_stop_signal_preserves_operational_refusal_instead_of_a_bad_proof_verdict() {
    #[derive(Debug)]
    struct Stop;
    impl purrdf_datalog::StopSignal for Stop {
        fn stopped(&self) -> bool {
            true
        }
    }
    let (proof, mut context) = derived();
    context.kb.stop = Some(std::sync::Arc::new(Stop));
    assert_eq!(
        proof.replay_refutation(&context).unwrap_err(),
        DlProofError::Stopped
    );
    context.kb.stop = None;
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
    let support = &mut evidence(&mut changed).support;
    let first = support.first_mut().unwrap();
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
        let reasoner = Reasoner::with_proofs(&dataset).unwrap();
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
