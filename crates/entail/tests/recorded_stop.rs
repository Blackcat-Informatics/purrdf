// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One stop signal covers recorded identity, preparation and decision search.

use purrdf_core::{
    BlankScope, CanonError, CanonHash, RdfDataset, RdfDatasetBuilder, RdfLiteral, StopSignal,
};
use purrdf_entail::reasoner::SchemaPreparationBudget;
use purrdf_entail::{EntailError, Question, Reasoner, StopCause, Verdict};
use purrdf_iri::vocab::{owl, rdf, rdfs};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[derive(Debug)]
struct Signal {
    calls: AtomicUsize,
    cancel_at: usize,
    cancelled: AtomicBool,
}

impl Signal {
    fn after(cancel_at: usize) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            cancel_at,
            cancelled: AtomicBool::new(false),
        }
    }
}

impl StopSignal for Signal {
    fn stopped(&self) -> bool {
        if self.calls.fetch_add(1, Ordering::Relaxed) >= self.cancel_at {
            self.cancelled.store(true, Ordering::Relaxed);
        }
        self.cancelled.load(Ordering::Relaxed)
    }
}

fn taxonomy(inconsistent: bool) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let class = builder.intern_iri("https://example.org/Cat").unwrap();
    let parent = builder.intern_iri("https://example.org/Animal").unwrap();
    let individual = builder.intern_iri("https://example.org/tom").unwrap();
    let ty = builder.intern_iri(rdf::TYPE).unwrap();
    let sub = builder.intern_iri(rdfs::SUB_CLASS_OF).unwrap();
    builder.push_quad(class, sub, parent, None).unwrap();
    builder.push_quad(individual, ty, class, None).unwrap();
    if inconsistent {
        let disjoint = builder.intern_iri(owl::DISJOINT_WITH).unwrap();
        builder.push_quad(class, disjoint, parent, None).unwrap();
    }
    builder.freeze().unwrap()
}

fn bounded_schema() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let class = builder.intern_iri("https://example.org/Cat").unwrap();
    let filler = builder.intern_iri("https://example.org/Animal").unwrap();
    let role = builder.intern_iri("https://example.org/parent").unwrap();
    let restriction = builder.intern_blank("bound", BlankScope::DEFAULT).unwrap();
    let number = builder
        .intern_literal(RdfLiteral {
            lexical_form: "1".to_owned(),
            datatype: Some(purrdf_xsd::datatype::XSD_NON_NEGATIVE_INTEGER.to_owned()),
            language: None,
            direction: None,
        })
        .unwrap();
    for (subject, predicate, object) in [
        (class, rdfs::SUB_CLASS_OF, restriction),
        (restriction, owl::ON_PROPERTY, role),
        (restriction, owl::ON_CLASS, filler),
        (restriction, owl::MIN_QUALIFIED_CARDINALITY, number),
    ] {
        let predicate = builder.intern_iri(predicate).unwrap();
        builder.push_quad(subject, predicate, object, None).unwrap();
    }
    let individual = builder.intern_iri("https://example.org/tom").unwrap();
    let ty = builder.intern_iri(rdf::TYPE).unwrap();
    builder.push_quad(individual, ty, class, None).unwrap();
    builder.freeze().unwrap()
}

/// A non-stopping signal changes neither verdicts, counters nor replayed proofs.
fn recorded_stop_preserves_complete_decisions_and_proof_identity() {
    for inconsistent in [false, true] {
        let dataset = taxonomy(inconsistent);
        let plain = Reasoner::with_proofs(
            &dataset,
            &mut purrdf_core::CanonicalizationControl::new(
                purrdf_core::RDFC_V2_CORPUS_POLICY,
                None,
            ),
        )
        .unwrap()
        .consistency();
        let signal = Arc::new(Signal::after(usize::MAX));
        let mut reasoner =
            Reasoner::with_proofs_and_stop(&dataset, SchemaPreparationBudget::default(), signal)
                .unwrap();
        assert!(reasoner.records_proofs());
        reasoner.prepare(&Question::Consistency).unwrap();
        let result = reasoner.consistency();
        assert_eq!(
            *result.answer(),
            if inconsistent {
                Verdict::False
            } else {
                Verdict::True
            }
        );
        assert_eq!(result.certificate(), plain.certificate());
        let context = reasoner.proof_context().unwrap();
        result
            .proof()
            .unwrap()
            .verify(
                &dataset,
                &Question::Consistency,
                Some(result.certificate()),
                &context,
            )
            .unwrap();
        assert_eq!(
            purrdf_entail::try_ontology_identity_with_stop(&dataset, &Signal::after(usize::MAX))
                .unwrap(),
            purrdf_entail::try_ontology_identity(
                &dataset,
                &mut purrdf_core::CanonicalizationControl::new(
                    purrdf_core::RDFC_V2_CORPUS_POLICY,
                    None
                )
            )
            .unwrap(),
        );
    }
}

/// Pre-cancelled recording is an execution stop, not malformed input or false.
fn recorded_stop_refuses_before_identity_preparation() {
    assert!(matches!(
        Reasoner::with_proofs_and_stop(
            &taxonomy(false),
            SchemaPreparationBudget::default(),
            Arc::new(Signal::after(0)),
        ),
        Err(EntailError::Stopped)
    ));
}

/// The same signal remains active when identity hands over to ontology parsing.
fn recorded_stop_retains_the_signal_after_identity() {
    let dataset = taxonomy(false);
    let counter = Signal::after(usize::MAX);
    purrdf_entail::try_ontology_identity_with_stop(&dataset, &counter).unwrap();
    let polls = counter.calls.load(Ordering::Relaxed);
    assert!(matches!(
        Reasoner::with_proofs_and_stop(
            &dataset,
            SchemaPreparationBudget::default(),
            Arc::new(Signal::after(polls)),
        ),
        Err(EntailError::Stopped)
    ));
}

/// Canonical bytes being ready does not permit publishing a partial input digest.
fn recorded_stop_covers_identity_digest_streaming() {
    let dataset = taxonomy(false);
    let counter = Signal::after(usize::MAX);
    purrdf_core::try_canonicalize_view(
        &*dataset,
        CanonHash::Sha256,
        &mut purrdf_core::CanonicalizationControl::new(
            purrdf_core::RDFC_V2_CORPUS_POLICY,
            Some(&counter),
        ),
    )
    .unwrap();
    let canonical_polls = counter.calls.load(Ordering::Relaxed);
    let signal = Signal::after(canonical_polls + 1);
    assert!(matches!(
        purrdf_entail::try_ontology_identity_with_stop(&dataset, &signal),
        Err(CanonError::Stopped)
    ));
    assert!(signal.calls.load(Ordering::Relaxed) > canonical_polls + 1);
}

/// A later cancelled decision retains an honest unknown certificate and receipt.
fn recorded_stop_reports_cancelled_search_without_a_semantic_verdict() {
    let dataset = taxonomy(false);
    let signal = Arc::new(Signal::after(usize::MAX));
    let mut reasoner = Reasoner::with_proofs_and_stop(
        &dataset,
        SchemaPreparationBudget::default(),
        signal.clone(),
    )
    .unwrap();
    reasoner.prepare(&Question::Consistency).unwrap();
    signal.cancelled.store(true, Ordering::Relaxed);
    let result = reasoner.consistency();
    assert_eq!(*result.answer(), Verdict::Unknown);
    assert!(!result.certificate().completeness().is_decided());
    let proof = result.proof().unwrap();
    assert_eq!(proof.receipt().unwrap().cause(), StopCause::CallerStop);
    proof
        .verify(
            &dataset,
            &Question::Consistency,
            Some(result.certificate()),
            &reasoner.proof_context().unwrap(),
        )
        .unwrap();
}

/// Selecting recording and cancellation does not discard preparation ceilings.
fn recorded_stop_preserves_preparation_budget_selection() {
    let dataset = bounded_schema();
    let budget = SchemaPreparationBudget {
        work: Some(0),
        bytes: None,
    };
    let plain = Reasoner::with_proofs_and_preparation_budget(
        &dataset,
        budget,
        &mut purrdf_core::CanonicalizationControl::new(purrdf_core::RDFC_V2_CORPUS_POLICY, None),
    )
    .unwrap();
    let controlled =
        Reasoner::with_proofs_and_stop(&dataset, budget, Arc::new(Signal::after(usize::MAX)))
            .unwrap();
    assert_eq!(controlled.schema_preparation(), plain.schema_preparation());
    assert!(controlled.schema_preparation().obstruction.is_some());
    assert_eq!(*controlled.consistency().answer(), Verdict::Unknown);
    assert_eq!(
        controlled.consistency().certificate(),
        plain.consistency().certificate()
    );
}

/// Every observed preparation boundary refuses without publishing a reasoner.
fn recorded_stop_covers_every_preparation_poll() {
    let dataset = bounded_schema();
    let counter = Arc::new(Signal::after(usize::MAX));
    Reasoner::with_proofs_and_stop(
        &dataset,
        SchemaPreparationBudget::default(),
        counter.clone(),
    )
    .unwrap();
    let polls = counter.calls.load(Ordering::Relaxed);
    for cancel_at in 0..polls {
        assert!(
            matches!(
                Reasoner::with_proofs_and_stop(
                    &dataset,
                    SchemaPreparationBudget::default(),
                    Arc::new(Signal::after(cancel_at)),
                ),
                Err(EntailError::Stopped)
            ),
            "preparation published after cancellation at poll {cancel_at}/{polls}"
        );
    }
}

purrdf_testkit::harness_main!(
    recorded_stop_preserves_complete_decisions_and_proof_identity,
    recorded_stop_refuses_before_identity_preparation,
    recorded_stop_retains_the_signal_after_identity,
    recorded_stop_covers_identity_digest_streaming,
    recorded_stop_reports_cancelled_search_without_a_semantic_verdict,
    recorded_stop_preserves_preparation_budget_selection,
    recorded_stop_covers_every_preparation_poll,
);
