// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Public recorded services and original source identities, natively and on WASM.

use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder, TermId, TermValue};
use purrdf_entail::{
    DlProof, DlProofContext, ProofAnswer, Question, Reasoner, Verdict, prove_consistency,
};
use purrdf_iri::vocab::{owl, rdf, rdfs};
use std::sync::Arc;

#[derive(Default)]
struct Fixture {
    builder: RdfDatasetBuilder,
}

impl Fixture {
    fn name(&mut self, name: &str) -> TermId {
        self.builder
            .intern_iri(&format!("https://example.org/{name}"))
            .unwrap()
    }

    fn edge(&mut self, subject: TermId, predicate: &str, object: TermId) {
        let predicate = self.builder.intern_iri(predicate).unwrap();
        self.builder
            .push_quad(subject, predicate, object, None)
            .unwrap();
    }

    fn freeze(self) -> Arc<RdfDataset> {
        self.builder.freeze().unwrap()
    }
}

fn term(name: &str) -> TermValue {
    TermValue::Iri(format!("https://example.org/{name}"))
}

fn derived(blank: bool) -> Arc<RdfDataset> {
    let mut fixture = Fixture::default();
    let subject = if blank {
        fixture
            .builder
            .intern_blank("subject", BlankScope(47))
            .unwrap()
    } else {
        fixture.name("subject")
    };
    let a = fixture.name("A");
    let b = fixture.name("B");
    let c = fixture.name("C");
    fixture.edge(a, rdfs::SUB_CLASS_OF, b);
    fixture.edge(b, owl::DISJOINT_WITH, c);
    fixture.edge(subject, rdf::TYPE, a);
    fixture.edge(subject, rdf::TYPE, c);
    let other = fixture.name("disconnected");
    let other_class = fixture.name("Other");
    fixture.edge(other, rdf::TYPE, other_class);
    fixture.freeze()
}

fn current_clash_preserves_named_and_scoped_blank_source_identity() {
    for blank in [false, true] {
        let dataset = derived(blank);
        let (answer, proof) = prove_consistency(&dataset).unwrap();
        assert_eq!(answer, ProofAnswer::Inconsistent);
        let decoded = DlProof::decode(&proof.encode()).unwrap();
        let context = DlProofContext::of_ontology(&dataset).unwrap();
        let expected = if blank {
            TermValue::Blank {
                label: "subject".to_owned(),
                scope: BlankScope(47),
            }
        } else {
            term("subject")
        };
        for index in 0..decoded.clashes().len() {
            let replay = decoded.replay_clash(index, &context).unwrap();
            assert!(replay.current_supported());
            assert_eq!(replay.unattested(), 0);
            assert_eq!(replay.source_node(), Some(&expected));
            assert_ne!(replay.source_node(), Some(&term("disconnected")));
        }
        assert!(decoded.replay_refutation(&context).unwrap().is_closed());
        assert_eq!(decoded.encode(), proof.encode());
    }
}

fn current_clash_preserves_public_service_certificate_and_verification() {
    let dataset = derived(false);
    let plain = Reasoner::new(&dataset).unwrap().consistency();
    let reasoner = Reasoner::with_proofs(&dataset).unwrap();
    let recorded = reasoner.consistency();
    assert_eq!(*recorded.answer(), Verdict::False);
    assert_eq!(plain.certificate(), recorded.certificate());
    let context = reasoner.proof_context().unwrap();
    recorded
        .proof()
        .unwrap()
        .verify(
            &dataset,
            &Question::Consistency,
            Some(recorded.certificate()),
            &context,
        )
        .unwrap();
    assert!(
        recorded
            .proof()
            .unwrap()
            .runs()
            .iter()
            .filter_map(|run| run.proof())
            .flat_map(DlProof::clashes)
            .all(|step| step.current_support().is_some())
    );
    let other = Fixture::default().freeze();
    let wrong = DlProofContext::of_ontology(&other).unwrap();
    let proof = recorded
        .proof()
        .unwrap()
        .runs()
        .iter()
        .find_map(|run| run.proof())
        .unwrap();
    assert!(proof.replay_clash(0, &wrong).is_err());
}

fn current_clash_replays_generated_existential_witnesses() {
    let mut fixture = Fixture::default();
    let subject = fixture.name("subject");
    let a = fixture.name("A");
    let b = fixture.name("B");
    let c = fixture.name("C");
    let role = fixture.name("role");
    let restriction = fixture
        .builder
        .intern_blank("restriction", BlankScope::DEFAULT)
        .unwrap();
    fixture.edge(a, rdfs::SUB_CLASS_OF, restriction);
    fixture.edge(restriction, owl::ON_PROPERTY, role);
    fixture.edge(restriction, owl::SOME_VALUES_FROM, b);
    fixture.edge(role, rdfs::RANGE, c);
    fixture.edge(b, owl::DISJOINT_WITH, c);
    fixture.edge(subject, rdf::TYPE, a);
    let dataset = fixture.freeze();
    let (answer, proof) = prove_consistency(&dataset).unwrap();
    assert_eq!(answer, ProofAnswer::Inconsistent);
    let proof = DlProof::decode(&proof.encode()).unwrap();
    let context = DlProofContext::of_ontology(&dataset).unwrap();
    assert_eq!(
        proof
            .replay_refutation(&context)
            .unwrap()
            .checks()
            .unattested(),
        0
    );
    assert_ne!(proof.clashes(), []);
    for index in 0..proof.clashes().len() {
        let replay = proof.replay_clash(index, &context).unwrap();
        assert!(replay.current_supported());
        assert!(
            replay.source_node().is_none(),
            "generated node is not the named predecessor"
        );
    }
}

fn current_clash_checks_fresh_refutation_assumptions_without_a_source_name() {
    let mut fixture = Fixture::default();
    let a = fixture.name("A");
    let b = fixture.name("B");
    let c = fixture.name("C");
    fixture.edge(a, rdfs::SUB_CLASS_OF, b);
    fixture.edge(a, rdfs::SUB_CLASS_OF, c);
    fixture.edge(b, owl::DISJOINT_WITH, c);
    let dataset = fixture.freeze();
    let mut reasoner = Reasoner::with_proofs(&dataset).unwrap();
    let result = reasoner.class_satisfiability(&term("A")).unwrap();
    assert_eq!(*result.answer(), Verdict::False);
    let context = reasoner.proof_context().unwrap();
    result
        .proof()
        .unwrap()
        .verify(
            &dataset,
            &Question::ClassSatisfiability { class: term("A") },
            Some(result.certificate()),
            &context,
        )
        .unwrap();
    let proofs: Vec<_> = result
        .proof()
        .unwrap()
        .runs()
        .iter()
        .filter_map(|run| run.proof())
        .filter(|proof| proof.answer() == ProofAnswer::Inconsistent)
        .collect();
    assert!(!proofs.is_empty(), "the class query records a refutation");
    assert!(proofs.iter().all(|proof| {
        proof
            .clashes()
            .iter()
            .all(|step| step.current_support().is_some())
    }));
    // A standalone consistency replay lacks the service's fresh assumption.
    assert!(
        proofs
            .iter()
            .all(|proof| proof.replay_clash(0, &context).is_err())
    );
}

fn current_clash_replays_equality_before_observing_role_premises() {
    let mut fixture = Fixture::default();
    let a = fixture.name("a");
    let b = fixture.name("b");
    let class = fixture.name("A");
    let role = fixture.name("p");
    let restriction = fixture
        .builder
        .intern_blank("restriction", BlankScope::DEFAULT)
        .unwrap();
    let nominal = fixture
        .builder
        .intern_blank("nominal", BlankScope::DEFAULT)
        .unwrap();
    let list = fixture
        .builder
        .intern_blank("list", BlankScope::DEFAULT)
        .unwrap();
    let nil = fixture.builder.intern_iri(rdf::NIL).unwrap();
    let asymmetric = fixture
        .builder
        .intern_iri(owl::ASYMMETRIC_PROPERTY)
        .unwrap();
    fixture.edge(class, rdfs::SUB_CLASS_OF, restriction);
    fixture.edge(restriction, owl::ON_PROPERTY, role);
    fixture.edge(restriction, owl::SOME_VALUES_FROM, nominal);
    fixture.edge(nominal, owl::ONE_OF, list);
    fixture.edge(list, rdf::FIRST, b);
    fixture.edge(list, rdf::REST, nil);
    fixture.edge(a, rdf::TYPE, class);
    fixture.edge(role, rdf::TYPE, asymmetric);
    fixture.builder.push_quad(b, role, a, None).unwrap();
    let dataset = fixture.freeze();
    let (answer, proof) = prove_consistency(&dataset).unwrap();
    assert_eq!(answer, ProofAnswer::Inconsistent);
    assert!(
        !proof.merges().is_empty(),
        "the existential nominal identifies its witness"
    );
    assert!(
        !proof.clashes().is_empty(),
        "the identified edge closes on the asymmetric clause"
    );
    let proof = DlProof::decode(&proof.encode()).unwrap();
    let context = DlProofContext::of_ontology(&dataset).unwrap();
    assert_eq!(
        proof
            .replay_refutation(&context)
            .unwrap()
            .checks()
            .unattested(),
        0
    );
    for index in 0..proof.clashes().len() {
        assert!(
            proof
                .replay_clash(index, &context)
                .unwrap()
                .current_supported()
        );
    }
}

/// Borrowing checks the independent consumer snapshot repeatedly and leaves it reusable.
fn current_clash_borrows_the_independent_prepared_consumer() {
    for blank in [false, true] {
        let dataset = derived(blank);
        let producer = Reasoner::with_proofs(&dataset).unwrap();
        let answer = producer.consistency();
        let proof = answer.proof().unwrap();
        let consumer = Reasoner::with_proofs(&dataset).unwrap();
        let owned = Reasoner::with_proofs(&dataset)
            .unwrap()
            .proof_context()
            .unwrap();
        {
            let borrowed = consumer.borrowed_proof_context().unwrap();
            assert_eq!(borrowed.input(), owned.input());
            assert_eq!(borrowed.contract(), owned.contract());
            let reference = proof
                .verify(
                    &dataset,
                    &Question::Consistency,
                    Some(answer.certificate()),
                    &owned,
                )
                .unwrap();
            for _ in 0..3 {
                assert_eq!(
                    proof
                        .verify_prepared(
                            &Question::Consistency,
                            Some(answer.certificate()),
                            &borrowed
                        )
                        .unwrap(),
                    reference
                );
            }
        }
        assert_eq!(consumer.consistency().certificate(), answer.certificate());
        let later = consumer.borrowed_proof_context().unwrap();
        let other = Fixture::default().freeze();
        assert!(
            proof
                .verify(
                    &other,
                    &Question::Consistency,
                    Some(answer.certificate()),
                    &later
                )
                .is_err()
        );
        let wrong_consumer = Reasoner::with_proofs(&other).unwrap();
        let wrong = wrong_consumer.borrowed_proof_context().unwrap();
        assert!(
            proof
                .verify_prepared(&Question::Consistency, Some(answer.certificate()), &wrong)
                .is_err()
        );
    }
}

/// The actual public operation exposes selected source support and original dependency edges.
fn public_causal_service_preserves_source_scope_and_excludes_disconnected_inputs() {
    use purrdf_entail::proof_source::{
        ResolvedTerm,
        causal::{CausalInputSource, CausalMutation, CausalObservation},
    };
    for blank in [false, true] {
        let dataset = derived(blank);
        let producer = Reasoner::with_proofs(&dataset).unwrap();
        let answer = producer.consistency();
        let proof = answer.proof().unwrap();
        let consumer = Reasoner::with_proofs(&dataset).unwrap();
        let ctx = consumer.borrowed_proof_context().unwrap();
        let ordinary = proof
            .verify_prepared(&Question::Consistency, Some(answer.certificate()), &ctx)
            .unwrap();
        let causal = proof
            .verify_prepared_causal(&Question::Consistency, Some(answer.certificate()), &ctx)
            .unwrap();
        assert_eq!(causal.report(), &ordinary);
        assert_eq!(causal.question(), &Question::Consistency);
        assert_ne!(causal.refutations().len(), 0);
        assert_ne!(
            causal
                .refutations()
                .flat_map(|(_, refutation)| refutation.clashes())
                .count(),
            0
        );
        let expected = if blank {
            TermValue::Blank {
                label: "subject".to_owned(),
                scope: BlankScope(47),
            }
        } else {
            term("subject")
        };
        for (_, refutation) in causal.refutations() {
            assert!(refutation.replay().is_closed());
            for clash in refutation.clashes() {
                assert!(matches!(
                    clash.root().observation(),
                    CausalObservation::Clash { .. }
                ));
                let ordinals = clash
                    .steps()
                    .map(|step| step.ordinal())
                    .collect::<std::collections::BTreeSet<_>>();
                let mut named = false;
                let mut body_read = false;
                for step in clash.steps() {
                    assert!(
                        step.parents()
                            .all(|parent| parent.ordinal() < step.ordinal()
                                && ordinals.contains(&parent.ordinal()))
                    );
                    if let CausalObservation::Effect {
                        mutation:
                            CausalMutation::Node {
                                node,
                                named: Some(ResolvedTerm::Source(value)),
                                ..
                            },
                        ..
                    } = step.observation()
                    {
                        assert_ne!(value, &term("disconnected"));
                        assert!(node.same_node(&node));
                        if value == &expected {
                            assert_eq!(node.term(), Some(ResolvedTerm::Source(&expected)));
                            named = true;
                        }
                    }
                    for read in step.reads() {
                        body_read |= matches!(
                            read,
                            purrdf_entail::proof_source::causal::CausalFact::Concept(..)
                        );
                    }
                }
                assert!(named);
                assert!(body_read);
                assert!(clash.inputs().all(|input| match input.source() {
                    CausalInputSource::Axiom(
                        purrdf_entail::proof_source::ResolvedAxiomSource::Triple {
                            subject: ResolvedTerm::Source(value),
                            ..
                        },
                    ) => value != &term("disconnected"),
                    _ => true,
                }));
                assert_ne!(clash.syntax_roots().len(), 0);
            }
        }
    }
}

/// A query's anonymous premise stays separate from source terms and cannot verify another question.
fn public_causal_service_binds_the_requested_question_and_query_assumptions() {
    use purrdf_entail::proof_source::causal::{CausalCauseKind, CausalMutation, CausalObservation};
    let mut fixture = Fixture::default();
    let a = fixture.name("A");
    let b = fixture.name("B");
    let c = fixture.name("C");
    fixture.edge(a, rdfs::SUB_CLASS_OF, b);
    fixture.edge(a, rdfs::SUB_CLASS_OF, c);
    fixture.edge(b, owl::DISJOINT_WITH, c);
    let dataset = fixture.freeze();
    let mut producer = Reasoner::with_proofs(&dataset).unwrap();
    let answer = producer.class_satisfiability(&term("A")).unwrap();
    assert_eq!(*answer.answer(), Verdict::False);
    let question = Question::ClassSatisfiability { class: term("A") };
    let mut consumer = Reasoner::with_proofs(&dataset).unwrap();
    consumer.prepare(&question).unwrap();
    let ctx = consumer.borrowed_proof_context().unwrap();
    let proof = answer.proof().unwrap();
    let causal = proof
        .verify_prepared_causal(&question, Some(answer.certificate()), &ctx)
        .unwrap();
    let mut assumption = false;
    for (_, refutation) in causal.refutations() {
        for clash in refutation.clashes() {
            for step in clash.steps() {
                if let CausalObservation::Effect { cause, mutation } = step.observation() {
                    assumption |= cause.kind() == CausalCauseKind::AssumedAnonymousType;
                    if let CausalMutation::Node {
                        node, named: None, ..
                    } = mutation
                    {
                        assert_eq!(node.term(), None);
                    }
                }
            }
        }
    }
    assert!(assumption);
    assert!(matches!(
        proof.verify_prepared_causal(&Question::Consistency, Some(answer.certificate()), &ctx),
        Err(purrdf_entail::DlProofError::WrongQuestion { .. })
    ));
    let wrong = Reasoner::with_proofs(&Fixture::default().freeze()).unwrap();
    let wrong_ctx = wrong.borrowed_proof_context().unwrap();
    assert!(matches!(
        proof.verify_prepared_causal(&question, Some(answer.certificate()), &wrong_ctx),
        Err(purrdf_entail::DlProofError::InputMismatch { .. })
    ));
}

fn public_causal_service_checks_nominal_merge_branches_after_wire_roundtrip() {
    use purrdf_core::RdfLiteral;
    use purrdf_core::collections::{ListVocab, build_rdf_list};
    use purrdf_entail::proof_source::causal::{CausalFact, CausalMutation, CausalObservation};
    let mut f = Fixture::default();
    let class = f.name("A");
    let filler = f.name("F");
    let property = f.name("p");
    let restriction = f.builder.intern_blank("count", BlankScope(52)).unwrap();
    let nominal = f.builder.intern_blank("nominal", BlankScope(52)).unwrap();
    let bound = f
        .builder
        .intern_literal(RdfLiteral {
            lexical_form: "3".to_owned(),
            datatype: Some(purrdf_xsd::datatype::XSD_NON_NEGATIVE_INTEGER.to_owned()),
            language: None,
            direction: None,
        })
        .unwrap();
    f.edge(class, rdfs::SUB_CLASS_OF, restriction);
    f.edge(restriction, owl::ON_PROPERTY, property);
    f.edge(restriction, owl::ON_CLASS, filler);
    f.edge(restriction, owl::MIN_QUALIFIED_CARDINALITY, bound);
    let members = [f.name("first"), f.name("second")];
    let cells = [
        f.builder.intern_blank("list0", BlankScope(52)).unwrap(),
        f.builder.intern_blank("list1", BlankScope(52)).unwrap(),
    ];
    let vocab = ListVocab {
        first: f.builder.intern_iri(rdf::FIRST).unwrap(),
        rest: f.builder.intern_iri(rdf::REST).unwrap(),
        nil: f.builder.intern_iri(rdf::NIL).unwrap(),
    };
    let head = build_rdf_list(
        members,
        &vocab,
        |index| cells[index],
        |subject, predicate, object| {
            f.builder
                .push_quad(subject, predicate, object, None)
                .unwrap();
        },
    );
    f.edge(nominal, owl::ONE_OF, head);
    f.edge(filler, rdfs::SUB_CLASS_OF, nominal);
    let subject = f.name("subject");
    f.edge(subject, rdf::TYPE, class);
    let source = f.freeze();
    let producer = Reasoner::with_proofs(&source).unwrap();
    let answer = producer.consistency();
    assert_eq!(*answer.answer(), Verdict::False);
    let original = answer.proof().unwrap();
    let proof = purrdf_entail::ServiceProof::decode(&original.encode()).unwrap();
    assert_eq!(proof.encode(), original.encode());
    let consumer = Reasoner::with_proofs(&source).unwrap();
    let ctx = consumer.borrowed_proof_context().unwrap();
    let checked = proof
        .verify_prepared_causal(&Question::Consistency, Some(answer.certificate()), &ctx)
        .unwrap();
    let (_, refutation) = checked.refutations().next().unwrap();
    assert!(refutation.replay().is_closed());
    assert!(refutation.replay().branches() > 0);
    assert!(refutation.replay().checked_merges() > 1);
    assert_eq!(
        refutation.clashes().len(),
        refutation.replay().checked_merges()
    );
    for leaf in refutation.clashes() {
        assert!(leaf.is_merge());
        assert!(matches!(
            leaf.root().observation(),
            CausalObservation::Merge { .. }
        ));
        assert!(
            leaf.root()
                .reads()
                .any(|fact| matches!(fact, CausalFact::Distinct(..)))
        );
        assert!(leaf.steps().any(|step| matches!(
            step.observation(),
            CausalObservation::Effect {
                mutation: CausalMutation::Distinct { .. },
                ..
            }
        )));
    }
}

purrdf_testkit::harness_main!(
    current_clash_preserves_named_and_scoped_blank_source_identity,
    current_clash_preserves_public_service_certificate_and_verification,
    current_clash_replays_generated_existential_witnesses,
    current_clash_checks_fresh_refutation_assumptions_without_a_source_name,
    current_clash_replays_equality_before_observing_role_premises,
    current_clash_borrows_the_independent_prepared_consumer,
    public_causal_service_preserves_source_scope_and_excludes_disconnected_inputs,
    public_causal_service_binds_the_requested_question_and_query_assumptions,
    public_causal_service_checks_nominal_merge_branches_after_wire_roundtrip,
);
