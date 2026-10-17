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

purrdf_testkit::harness_main!(
    current_clash_preserves_named_and_scoped_blank_source_identity,
    current_clash_preserves_public_service_certificate_and_verification,
    current_clash_replays_generated_existential_witnesses,
    current_clash_checks_fresh_refutation_assumptions_without_a_source_name,
    current_clash_replays_equality_before_observing_role_premises,
);
