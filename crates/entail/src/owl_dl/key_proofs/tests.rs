// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Actual recorded constructors retain all membership branches before key publication.

use super::{Evidence, Records, Verification};
use crate::{
    owl_dl::{Kb, graph::changes::Origin},
    reasoner::{Reasoner, SchemaPreparationBudget, schema_tests::Fixture},
};
use purrdf_iri::vocab::{owl, rdfs};

fn input(branching: bool) -> std::sync::Arc<purrdf_core::RdfDataset> {
    let mut f = Fixture::default();
    let keyed = f.name("Keyed");
    let property = f.name("key");
    let properties = f.list(&[property]);
    f.edge(keyed, owl::HAS_KEY, properties);
    let role = f.name("required");
    let filler = f.name("Filler");
    f.bounds(keyed, role, filler, 1, true, false);
    let class = if branching {
        let class = f.name("B");
        let left = f.name("C");
        let right = f.name("D");
        let union = f.blank();
        let members = f.list(&[left, right]);
        f.edge(union, owl::UNION_OF, members);
        f.edge(class, rdfs::SUB_CLASS_OF, union);
        f.edge(left, rdfs::SUB_CLASS_OF, keyed);
        f.edge(right, rdfs::SUB_CLASS_OF, keyed);
        class
    } else {
        keyed
    };
    let a = f.instance("a", class);
    let b = f.instance("b", class);
    let value = f.name("shared");
    f.builder.push_quad(a, property, value, None).unwrap();
    f.builder.push_quad(b, property, value, None).unwrap();
    let unrelated = f.name("Other");
    f.instance("unrelated", unrelated);
    f.freeze()
}

#[test]
fn recorded_key_memberships_are_checked_in_the_original_world_with_all_branches() {
    for branching in [false, true] {
        let input = input(branching);
        let ordinary = Kb::from_dataset(&input).unwrap();
        assert!(matches!(ordinary.key_proofs, Records::Disabled));
        let reasoner = Reasoner::with_proofs(&input).unwrap();
        let context = reasoner.borrowed_proof_context().unwrap();
        assert_eq!(ordinary.same_as, context.kb.same_as);
        let Records::Ready(evidence) = &context.kb.key_proofs else {
            panic!("recorded key preparation must complete");
        };
        assert_eq!(evidence.input, context.input());
        assert_eq!(evidence.phase.equalities, 0);
        assert_eq!(context.kb.same_as.len(), 1);
        for name in ["https://example.org/a", "https://example.org/b"] {
            let individual = context.kb.interner.id_of_iri(name).unwrap();
            let member = evidence.member(0, individual).unwrap();
            assert_eq!(member.individual, individual);
            assert_eq!(member.class, context.kb.keys[0].0);
            assert_eq!(member.proof.input(), context.input());
            let Verification::Closed(report, capture) = &member.verification else {
                panic!(
                    "expected complete native membership support: {:?}",
                    member.verification
                );
            };
            assert!(report.is_closed());
            assert_eq!(capture.clashes.len(), report.clashes());
            assert_ne!(capture.clashes.len(), 0);
            if branching {
                assert!(report.branches() > 0);
                assert!(capture.clashes.len() >= 2);
            }
            for clash in &capture.clashes {
                assert!(clash.index < member.proof.clashes().len());
                let step = &member.proof.clashes()[clash.index];
                assert_eq!(clash.schema, step.schema_evidence().map(|_| step.clause()));
                assert!(
                    clash
                        .journal
                        .events()
                        .iter()
                        .all(|event| event.cause.origin != Origin::PreparedEquality),
                    "a derived key equality cannot license its own membership"
                );
            }
        }
        let unrelated = context
            .kb
            .interner
            .id_of_iri("https://example.org/unrelated")
            .unwrap();
        assert!(evidence.member(0, unrelated).is_none());
    }
}

#[test]
fn deferred_key_recording_retains_the_original_request_and_rejects_a_later_world() {
    let input = input(false);
    let mut reasoner = Reasoner::with_proofs_and_preparation_budget(
        &input,
        SchemaPreparationBudget {
            work: Some(0),
            bytes: None,
        },
    )
    .unwrap();
    {
        let context = reasoner.borrowed_proof_context().unwrap();
        assert!(reasoner.schema_preparation().obstruction.is_some());
        assert!(matches!(context.kb.key_proofs, Records::Pending(_)));
        assert_eq!(context.kb.same_as, []);
    }
    reasoner
        .retry_schema_preparation(SchemaPreparationBudget::default())
        .unwrap();
    let context = reasoner.borrowed_proof_context().unwrap();
    let Records::Ready(recorded) = &context.kb.key_proofs else {
        panic!("retry must retain recording");
    };
    assert_eq!(recorded.phase.equalities, 0);
    let mut wrong_phase = Evidence::for_phase(&context).unwrap();
    wrong_phase.phase.equalities = 0;
    let individual = context
        .kb
        .interner
        .id_of_iri("https://example.org/a")
        .unwrap();
    assert!(
        wrong_phase
            .entails(0, individual, context.kb.keys[0].0, &context)
            .is_err()
    );
}
