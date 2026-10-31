// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Ownership and exact consumer binding of borrowed native checking contexts.

use super::schema_tests::{shared_schema, term};
use super::{Question, Reasoner, SchemaPreparationBudget, Verdict};
use crate::{DlProofError, EntailError};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Caller-owned cancellation shared across preparation and independent replay.
#[derive(Debug)]
struct Stop(AtomicBool);

impl purrdf_datalog::StopSignal for Stop {
    fn stopped(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// Direct shared and packed sources preserve decisions and independent proof replay.
#[test]
fn dataset_views_prepare_and_check_without_materializing_another_dataset() {
    for contradictory in [false, true] {
        let dataset = shared_schema(2, 3, contradictory);
        let bytes = purrdf_core::PackBuilder::build_bytes(
            &dataset,
            &mut purrdf_core::CanonicalizationControl::new(
                purrdf_core::RDFC_V2_CORPUS_POLICY,
                None,
            ),
        )
        .unwrap();
        let view = purrdf_core::PackView::from_bytes(&bytes).unwrap();
        let shared = Reasoner::with_proofs(
            &dataset,
            &mut purrdf_core::CanonicalizationControl::new(
                purrdf_core::RDFC_V2_CORPUS_POLICY,
                None,
            ),
        )
        .unwrap();
        let packed = Reasoner::with_proofs(
            &view,
            &mut purrdf_core::CanonicalizationControl::new(
                purrdf_core::RDFC_V2_CORPUS_POLICY,
                None,
            ),
        )
        .unwrap();
        let plain = Reasoner::new(&view).unwrap();
        let answer = packed.consistency();
        let expected = if contradictory {
            Verdict::False
        } else {
            Verdict::True
        };
        assert_eq!(*answer.answer(), expected);
        assert_eq!(answer.answer(), shared.consistency().answer());
        assert_eq!(answer.certificate(), plain.consistency().certificate());
        assert_eq!(plain.consistency().answer(), answer.answer());
        assert!(plain.consistency().proof().is_none());
        assert_eq!(packed.input, shared.input);
        assert_eq!(Arc::strong_count(&dataset), 1);

        // An independent consumer parses the same shared carrier directly; it
        // does not borrow any prepared state from the proof producer.
        let consumer = Reasoner::with_proofs(
            &view,
            &mut purrdf_core::CanonicalizationControl::new(
                purrdf_core::RDFC_V2_CORPUS_POLICY,
                None,
            ),
        )
        .unwrap();
        let context = consumer.borrowed_proof_context().unwrap();
        let replay = answer
            .proof()
            .unwrap()
            .verify_prepared(&Question::Consistency, Some(answer.certificate()), &context)
            .unwrap();
        assert_eq!(
            replay,
            answer
                .proof()
                .unwrap()
                .verify(
                    &view,
                    &Question::Consistency,
                    Some(answer.certificate()),
                    &context,
                )
                .unwrap()
        );
        assert_eq!(replay.checks().unattested(), 0);
        answer
            .proof()
            .unwrap()
            .covers_verdict(&super::proof::ClaimSubject::Consistent, *answer.answer())
            .unwrap();
    }
}

/// Shared sources retain the original caller stop signal in both recording modes.
#[test]
fn dataset_views_preserve_constructor_cancellation() {
    let dataset = shared_schema(1, 1, false);
    let bytes = purrdf_core::PackBuilder::build_bytes(
        &dataset,
        &mut purrdf_core::CanonicalizationControl::new(purrdf_core::RDFC_V2_CORPUS_POLICY, None),
    )
    .unwrap();
    let view = purrdf_core::PackView::from_bytes(&bytes).unwrap();
    let stopped = Arc::new(Stop(AtomicBool::new(true)));
    assert!(matches!(
        Reasoner::with_stop(&view, stopped.clone()),
        Err(EntailError::Stopped)
    ));
    assert!(matches!(
        Reasoner::with_proofs_and_stop(&view, SchemaPreparationBudget::default(), stopped),
        Err(EntailError::Stopped)
    ));
}

/// A borrowed context retains the actual prepared allocation, not a reconstructed copy.
#[test]
fn borrowed_context_shares_prepared_source_and_releases_the_reasoner() {
    let dataset = shared_schema(2, 3, false);
    let consumer = Reasoner::with_proofs(
        &dataset,
        &mut purrdf_core::CanonicalizationControl::new(purrdf_core::RDFC_V2_CORPUS_POLICY, None),
    )
    .unwrap();
    let before = consumer.schema_preparation();
    {
        let borrowed = consumer.borrowed_proof_context().unwrap();
        assert!(std::ptr::eq(
            &raw const *borrowed.kb,
            &raw const consumer.kb
        ));
        assert_eq!(consumer.schema_preparation(), before);
        assert_eq!(borrowed.input(), consumer.input.unwrap());
    }
    assert_eq!(*consumer.consistency().answer(), Verdict::True);
    let later = consumer.borrowed_proof_context().unwrap();
    assert!(std::ptr::eq(&raw const *later.kb, &raw const consumer.kb));
    assert_eq!(consumer.schema_preparation(), before);
}

/// Independent owning and borrowing consumers replay both complete consistency outcomes identically.
#[test]
fn borrowed_context_matches_owned_replay_without_producer_state() {
    for contradictory in [false, true] {
        let dataset = shared_schema(2, 3, contradictory);
        let producer = Reasoner::with_proofs(
            &dataset,
            &mut purrdf_core::CanonicalizationControl::new(
                purrdf_core::RDFC_V2_CORPUS_POLICY,
                None,
            ),
        )
        .unwrap();
        let answer = producer.consistency();
        let proof = answer.proof().unwrap();
        let owned = Reasoner::with_proofs(
            &dataset,
            &mut purrdf_core::CanonicalizationControl::new(
                purrdf_core::RDFC_V2_CORPUS_POLICY,
                None,
            ),
        )
        .unwrap()
        .proof_context()
        .unwrap();
        let consumer = Reasoner::with_proofs(
            &dataset,
            &mut purrdf_core::CanonicalizationControl::new(
                purrdf_core::RDFC_V2_CORPUS_POLICY,
                None,
            ),
        )
        .unwrap();
        let borrowed = consumer.borrowed_proof_context().unwrap();
        assert_eq!(borrowed.input(), owned.input());
        assert_eq!(borrowed.contract(), owned.contract());
        assert_eq!(borrowed.clause_count(), owned.clause_count());
        let check = |context: &crate::DlProofContext<'_>| {
            let sourced = proof
                .verify(
                    &dataset,
                    &Question::Consistency,
                    Some(answer.certificate()),
                    context,
                )
                .unwrap();
            assert_eq!(
                sourced,
                proof
                    .verify_prepared(&Question::Consistency, Some(answer.certificate()), context)
                    .unwrap()
            );
            sourced
        };
        assert_eq!(check(&borrowed), check(&owned));
        assert_eq!(check(&borrowed).checks().unattested(), 0);
    }
}

/// A prepared new question retains its exact contract and cannot check a different question or source.
#[test]
fn borrowed_context_preserves_prepared_question_and_source_refusals() {
    let dataset = shared_schema(1, 1, false);
    let class = term("PreviouslyUnmentioned");
    let question = Question::ClassSatisfiability {
        class: class.clone(),
    };
    let mut producer = Reasoner::with_proofs(
        &dataset,
        &mut purrdf_core::CanonicalizationControl::new(purrdf_core::RDFC_V2_CORPUS_POLICY, None),
    )
    .unwrap();
    let answer = producer.class_satisfiability(&class).unwrap();
    let proof = answer.proof().unwrap();
    let mut consumer = Reasoner::with_proofs(
        &dataset,
        &mut purrdf_core::CanonicalizationControl::new(purrdf_core::RDFC_V2_CORPUS_POLICY, None),
    )
    .unwrap();
    let unprepared = consumer.borrowed_proof_context().unwrap();
    assert!(
        proof
            .verify(&dataset, &question, Some(answer.certificate()), &unprepared)
            .is_err()
    );
    assert!(
        proof
            .verify_prepared(&question, Some(answer.certificate()), &unprepared)
            .is_err()
    );
    drop(unprepared);
    consumer.prepare(&question).unwrap();
    let borrowed = consumer.borrowed_proof_context().unwrap();
    let mut owner = Reasoner::with_proofs(
        &dataset,
        &mut purrdf_core::CanonicalizationControl::new(purrdf_core::RDFC_V2_CORPUS_POLICY, None),
    )
    .unwrap();
    owner.prepare(&question).unwrap();
    let owned = owner.proof_context().unwrap();
    assert_eq!(borrowed.contract(), owned.contract());
    let prepared = proof
        .verify_prepared(&question, Some(answer.certificate()), &borrowed)
        .unwrap();
    assert_eq!(
        prepared,
        proof
            .verify_prepared(&question, Some(answer.certificate()), &owned)
            .unwrap()
    );
    assert_eq!(
        prepared,
        proof
            .verify(&dataset, &question, Some(answer.certificate()), &borrowed)
            .unwrap()
    );
    assert_eq!(
        proof
            .verify(&dataset, &question, Some(answer.certificate()), &borrowed)
            .unwrap(),
        proof
            .verify(&dataset, &question, Some(answer.certificate()), &owned)
            .unwrap()
    );
    assert!(
        proof
            .verify(&dataset, &Question::Consistency, None, &borrowed)
            .is_err()
    );
    let changed = shared_schema(1, 2, false);
    assert!(
        proof
            .verify(&changed, &question, Some(answer.certificate()), &borrowed)
            .is_err()
    );
    let mut changed_consumer = Reasoner::with_proofs(
        &changed,
        &mut purrdf_core::CanonicalizationControl::new(purrdf_core::RDFC_V2_CORPUS_POLICY, None),
    )
    .unwrap();
    changed_consumer.prepare(&question).unwrap();
    let wrong = changed_consumer.borrowed_proof_context().unwrap();
    assert!(matches!(
        proof.verify_prepared(&question, Some(answer.certificate()), &wrong),
        Err(DlProofError::InputMismatch { .. })
    ));
    assert!(matches!(
        proof.verify_prepared(&Question::Consistency, None, &borrowed),
        Err(DlProofError::WrongQuestion { .. })
    ));
}

/// Borrowing cannot invent recording or detach the original caller's stop signal.
#[test]
fn borrowed_context_preserves_recording_refusal_and_caller_stop() {
    let dataset = shared_schema(1, 1, true);
    let plain = Reasoner::new(&dataset).unwrap();
    assert!(matches!(
        plain.borrowed_proof_context(),
        Err(EntailError::ProofsNotRecorded)
    ));
    let producer = Reasoner::with_proofs(
        &dataset,
        &mut purrdf_core::CanonicalizationControl::new(purrdf_core::RDFC_V2_CORPUS_POLICY, None),
    )
    .unwrap();
    let answer = producer.consistency();
    let stop = Arc::new(Stop(AtomicBool::new(false)));
    let consumer =
        Reasoner::with_proofs_and_stop(&dataset, SchemaPreparationBudget::default(), stop.clone())
            .unwrap();
    let borrowed = consumer.borrowed_proof_context().unwrap();
    stop.0.store(true, Ordering::Relaxed);
    assert!(matches!(
        answer.proof().unwrap().verify_prepared(
            &Question::Consistency,
            Some(answer.certificate()),
            &borrowed
        ),
        Err(DlProofError::Stopped)
    ));
    assert!(matches!(
        answer.proof().unwrap().verify(
            &dataset,
            &Question::Consistency,
            Some(answer.certificate()),
            &borrowed
        ),
        Err(DlProofError::Stopped)
    ));
}
