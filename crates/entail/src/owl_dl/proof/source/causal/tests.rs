// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Actual question-bound public verification, with explicit missing-evidence refusals.

/// A merge license alone must not attest the inequality that closed its branch.
#[test]
fn public_causal_counting_refuses_unchecked_merge_closure() {
    let mut f = Fixture::default();
    let a = f.name("A");
    let filler = f.name("F");
    let property = f.name("p");
    f.bounds(a, property, filler, 2, true, false);
    let nominal = f.blank();
    let target = f.name("target");
    let members = f.list(&[target]);
    f.edge(nominal, owl::ONE_OF, members);
    f.edge(filler, rdfs::SUB_CLASS_OF, nominal);
    let root = f.instance("root", a);
    let existing = f.instance("existing", filler);
    f.edge(root, "https://example.org/p", existing);
    let unrelated = f.name("unrelated");
    f.edge(root, "https://example.org/p", unrelated);
    let source = f.freeze();
    let producer = Reasoner::with_proofs(&source).unwrap();
    let answer = producer.consistency();
    assert_eq!(*answer.answer(), crate::reasoner::Verdict::False);
    let consumer = Reasoner::with_proofs(&source).unwrap();
    let ctx = consumer.borrowed_proof_context().unwrap();
    assert!(matches!(
        answer.proof().unwrap().verify_prepared_causal(
            &Question::Consistency,
            Some(answer.certificate()),
            &ctx
        ),
        Err(DlProofError::CausalSupport {
            kind: CausalSupportKind::Unclosed,
            ..
        })
    ));
}

/// The public graph retains the actual counted head for forward and inverse witnesses.
#[test]
fn public_causal_counting_retains_original_head_and_generated_births() {
    for inverse in [false, true] {
        let mut f = Fixture::default();
        let class = f.name("A");
        let filler = f.name("B");
        let forbidden = f.name("C");
        let property = f.name("p");
        let role = if inverse {
            let role = f.blank();
            f.edge(role, owl::INVERSE_OF, property);
            role
        } else {
            property
        };
        f.bounds(class, role, filler, 2, true, false);
        f.edge(
            property,
            if inverse { rdfs::DOMAIN } else { rdfs::RANGE },
            forbidden,
        );
        f.edge(filler, owl::DISJOINT_WITH, forbidden);
        f.instance("subject", class);
        let source = f.freeze();
        let producer = Reasoner::with_proofs(&source).unwrap();
        let answer = producer.consistency();
        assert_eq!(*answer.answer(), crate::reasoner::Verdict::False);
        let consumer = Reasoner::with_proofs(&source).unwrap();
        let ctx = consumer.borrowed_proof_context().unwrap();
        let checked = answer
            .proof()
            .unwrap()
            .verify_prepared_causal(&Question::Consistency, Some(answer.certificate()), &ctx)
            .unwrap();
        let mut counting = 0;
        let mut births = 0;
        for (_, refutation) in checked.refutations() {
            for clash in refutation.clashes() {
                for step in clash.steps() {
                    match step.observation() {
                        CausalObservation::Counting { .. } => {
                            counting += 1;
                            assert!(step.parents().any(|parent| matches!(
                                parent.observation(),
                                CausalObservation::Application { .. }
                            )));
                        }
                        CausalObservation::Effect {
                            mutation:
                                CausalMutation::Node {
                                    parent: Some(_), ..
                                },
                            ..
                        } => births += 1,
                        _ => {}
                    }
                }
            }
        }
        assert!(counting > 0);
        assert!(births > 0);
    }
}

use super::*;
use crate::reasoner::{Question, Reasoner, RunProof, ServiceProof, schema_tests::Fixture};
use purrdf_core::RdfDataset;
use purrdf_iri::vocab::{owl, rdf, rdfs};
use std::sync::Arc;

/// A real source contradiction with no complex gate or repository corpus producer.
fn contradiction() -> Arc<RdfDataset> {
    let mut f = Fixture::default();
    let a = f.name("A");
    let b = f.name("B");
    f.edge(a, owl::DISJOINT_WITH, b);
    let individual = f.instance("i", a);
    f.edge(individual, rdf::TYPE, b);
    f.freeze()
}

/// Preserve the actual service envelope while changing one native refutation payload.
fn rewrite(proof: &ServiceProof, edit: impl Fn(&mut DlProof)) -> ServiceProof {
    let runs = proof
        .runs()
        .iter()
        .map(|run| {
            let native = run.proof().cloned().map(|mut native| {
                edit(&mut native);
                native
            });
            RunProof::new(run.assumptions().clone(), run.answer(), native)
        })
        .collect();
    ServiceProof::new(
        proof.input(),
        proof.question().clone(),
        runs,
        proof.claims().to_vec(),
        proof.receipt().cloned(),
        proof.truncated(),
    )
}

/// Legacy, missing source ownership, wrong calculus and unclosed records never yield graphs.
#[test]
fn public_causal_refuses_missing_and_unclosed_support_with_original_native_errors() {
    let source = contradiction();
    let producer = Reasoner::with_proofs(&source).unwrap();
    let answer = producer.consistency();
    let proof = answer.proof().unwrap();
    let mut ctx = Reasoner::with_proofs(&source)
        .unwrap()
        .proof_context()
        .unwrap();
    let missing = ServiceProof::new(
        proof.input(),
        proof.question().clone(),
        proof
            .runs()
            .iter()
            .map(|run| RunProof::new(run.assumptions().clone(), run.answer(), None))
            .collect(),
        proof.claims().to_vec(),
        proof.receipt().cloned(),
        false,
    );
    assert!(matches!(
        missing.verify_prepared_causal(&Question::Consistency, Some(answer.certificate()), &ctx),
        Err(DlProofError::CausalSupport {
            kind: CausalSupportKind::Recording,
            ..
        })
    ));
    let truncated = ServiceProof::new(
        proof.input(),
        proof.question().clone(),
        proof.runs().to_vec(),
        proof.claims().to_vec(),
        proof.receipt().cloned(),
        true,
    );
    assert!(matches!(
        truncated.verify_prepared_causal(&Question::Consistency, Some(answer.certificate()), &ctx),
        Err(DlProofError::ReceiptMismatch { .. })
    ));
    let wrong_assumptions = ServiceProof::new(
        proof.input(),
        proof.question().clone(),
        proof
            .runs()
            .iter()
            .map(|run| {
                RunProof::new(
                    crate::reasoner::RunAssumptions::of(&crate::owl_dl::graph::Assumptions {
                        include_abox: false,
                        types: &[],
                        roles: &[],
                        fresh_types: &[],
                    }),
                    run.answer(),
                    run.proof().cloned(),
                )
            })
            .collect(),
        proof.claims().to_vec(),
        proof.receipt().cloned(),
        false,
    );
    assert!(matches!(
        wrong_assumptions.verify_prepared_causal(
            &Question::Consistency,
            Some(answer.certificate()),
            &ctx
        ),
        Err(DlProofError::AnswerNotCovered { .. })
    ));
    let legacy = rewrite(proof, |native| {
        for step in &mut native.clashes {
            step.current = None;
        }
    });
    assert!(matches!(
        legacy.verify_prepared_causal(&Question::Consistency, Some(answer.certificate()), &ctx),
        Err(DlProofError::CausalSupport {
            kind: CausalSupportKind::CurrentSupport,
            run: Some(0),
            ..
        })
    ));
    let unused = rewrite(proof, |native| {
        let mut unused = native.clashes[0].clone();
        unused.current = None;
        native.clashes.push(unused);
    });
    let selected = unused
        .verify_prepared_causal(&Question::Consistency, Some(answer.certificate()), &ctx)
        .unwrap();
    assert_eq!(
        selected
            .refutations()
            .flat_map(|(_, proof)| proof.clashes())
            .count(),
        1
    );
    let unclosed = rewrite(proof, |native| {
        assert_eq!(native.branches, []);
        native.root = crate::owl_dl::proof::BranchOutcome::Unrecorded;
    });
    assert!(matches!(
        unclosed.verify_prepared_causal(&Question::Consistency, Some(answer.certificate()), &ctx),
        Err(DlProofError::CausalSupport {
            kind: CausalSupportKind::Unclosed,
            ..
        })
    ));
    let wrong = rewrite(proof, |native| native.contract[0] ^= 1);
    assert!(matches!(
        wrong.verify_prepared_causal(&Question::Consistency, Some(answer.certificate()), &ctx),
        Err(DlProofError::ContractMismatch { .. })
    ));
    let refused = rewrite(proof, |native| {
        native.schema_recording_obstruction =
            Some(crate::owl_dl::bounds::SchemaObstruction::Allocation);
    });
    assert!(matches!(
        refused.verify_prepared_causal(&Question::Consistency, Some(answer.certificate()), &ctx),
        Err(DlProofError::SchemaRecording {
            obstruction: crate::owl_dl::bounds::SchemaObstruction::Allocation
        })
    ));
    let purrdf_core::ownership::OwnedOrBorrowed::Owned(kb) = &mut ctx.kb else {
        unreachable!()
    };
    kb.inputs = crate::owl_dl::input_source::Inputs::default();
    assert!(matches!(
        proof.verify_prepared_causal(&Question::Consistency, Some(answer.certificate()), &ctx),
        Err(DlProofError::CausalSupport {
            kind: CausalSupportKind::Input,
            ..
        })
    ));
}

/// The selected public operation retains the consumer's original latched stop signal.
#[test]
fn public_causal_cancellation_preserves_the_original_native_refusal() {
    #[derive(Debug, Default)]
    struct Stop(std::sync::atomic::AtomicBool);
    impl purrdf_datalog::StopSignal for Stop {
        fn stopped(&self) -> bool {
            self.0.load(std::sync::atomic::Ordering::Relaxed)
        }
    }
    let source = contradiction();
    let producer = Reasoner::with_proofs(&source).unwrap();
    let answer = producer.consistency();
    let proof = answer.proof().unwrap();
    let stop = Arc::new(Stop::default());
    let consumer = Reasoner::with_proofs_and_stop(
        &source,
        crate::owl_dl::bounds::SchemaPreparationBudget::default(),
        stop.clone(),
    )
    .unwrap();
    let ctx = consumer.borrowed_proof_context().unwrap();
    assert!(
        proof
            .verify_prepared_causal(&Question::Consistency, Some(answer.certificate()), &ctx)
            .is_ok()
    );
    stop.0.store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(matches!(
        proof.verify_prepared_causal(&Question::Consistency, Some(answer.certificate()), &ctx),
        Err(DlProofError::Stopped)
    ));
    assert!(matches!(
        support_error(
            0,
            0,
            &slice::Error::Storage(purrdf_lex::allocation::StorageError::SizeOverflow)
        ),
        DlProofError::Storage(purrdf_lex::allocation::StorageError::SizeOverflow)
    ));
}

/// Branch choices come from the original checked prefix; a forged alternative cannot publish.
#[test]
fn public_causal_branches_bind_original_alternatives_and_keep_node_scopes_separate() {
    let mut f = Fixture::default();
    let a = f.name("A");
    let b = f.name("B");
    let c = f.name("C");
    let d = f.name("D");
    let union = f.blank();
    let members = f.list(&[b, c]);
    f.edge(union, owl::UNION_OF, members);
    f.edge(a, rdfs::SUB_CLASS_OF, union);
    f.edge(b, owl::DISJOINT_WITH, d);
    f.edge(c, owl::DISJOINT_WITH, d);
    let individual = f.instance("i", a);
    f.edge(individual, rdf::TYPE, d);
    let source = f.freeze();
    let producer = Reasoner::with_proofs(&source).unwrap();
    let answer = producer.consistency();
    let proof = answer.proof().unwrap();
    let consumer = Reasoner::with_proofs(&source).unwrap();
    let ctx = consumer.borrowed_proof_context().unwrap();
    let causal = proof
        .verify_prepared_causal(&Question::Consistency, Some(answer.certificate()), &ctx)
        .unwrap();
    let (_, refutation) = causal.refutations().next().unwrap();
    let branch = refutation.branches().next().unwrap();
    assert_eq!(branch.alternatives().len(), 2);
    assert_eq!(branch.outcomes().len(), 2);
    assert!(branch.alternatives().all(|alternative| {
        alternative
            .head()
            .all(|atom| matches!(atom, CausalGround::Concept(..)))
    }));
    let mut clashes = refutation.clashes();
    let first = clashes.next().unwrap();
    let second = clashes.next().unwrap();
    let first_node = first
        .steps()
        .find_map(|step| match step.observation() {
            CausalObservation::Effect {
                mutation:
                    CausalMutation::Node {
                        node,
                        named: Some(_),
                        ..
                    },
                ..
            } => Some(node),
            _ => None,
        })
        .unwrap();
    let second_node = second
        .steps()
        .find_map(|step| match step.observation() {
            CausalObservation::Effect {
                mutation:
                    CausalMutation::Node {
                        node,
                        named: Some(_),
                        ..
                    },
                ..
            } => Some(node),
            _ => None,
        })
        .unwrap();
    assert_eq!(first_node.ordinal(), second_node.ordinal());
    assert_eq!(first_node.term(), second_node.term());
    assert!(!first_node.same_node(&second_node));
    let forged = rewrite(proof, |native| {
        let choice = native
            .support
            .iter_mut()
            .find_map(|link| link.application.branch.as_mut())
            .unwrap();
        choice.1 = usize::MAX;
    });
    assert!(matches!(
        forged.verify_prepared_causal(&Question::Consistency, Some(answer.certificate()), &ctx),
        Err(DlProofError::Malformed { .. })
    ));
}

/// Every original membership branch remains observable in the public key-equality support.
#[test]
fn public_causal_key_memberships_retain_original_branch_graphs_and_source_scope() {
    let mut f = Fixture::default();
    let a = f.name("A");
    let b = f.name("B");
    let keyed = f.name("Keyed");
    let first = f.name("First");
    let second = f.name("Second");
    let membership = f.name("Membership");
    let union = f.blank();
    let list = f.list(&[first, second]);
    f.edge(union, owl::UNION_OF, list);
    f.edge(membership, rdfs::SUB_CLASS_OF, union);
    f.edge(first, rdfs::SUB_CLASS_OF, keyed);
    f.edge(second, rdfs::SUB_CLASS_OF, keyed);
    let property = f.name("key");
    let properties = f.list(&[property]);
    f.edge(keyed, owl::HAS_KEY, properties);
    f.edge(a, owl::DISJOINT_WITH, b);
    let left = f.instance("left", a);
    let right = f.instance("right", b);
    f.edge(left, rdf::TYPE, membership);
    f.edge(right, rdf::TYPE, membership);
    let value = f.name("value");
    f.builder.push_quad(left, property, value, None).unwrap();
    f.builder.push_quad(right, property, value, None).unwrap();
    let source = f.freeze();
    let producer = Reasoner::with_proofs(&source).unwrap();
    let answer = producer.consistency();
    let consumer = Reasoner::with_proofs(&source).unwrap();
    let ctx = consumer.borrowed_proof_context().unwrap();
    let causal = answer
        .proof()
        .unwrap()
        .verify_prepared_causal(&Question::Consistency, Some(answer.certificate()), &ctx)
        .unwrap();
    let mut keys = 0;
    for (_, refutation) in causal.refutations() {
        for clash in refutation.clashes() {
            for input in clash.inputs() {
                if let Some(members) = input.key_memberships() {
                    keys += 1;
                    assert!(matches!(
                        input.key_definition().unwrap().source(),
                        CausalInputSource::Axiom(ResolvedAxiomSource::Triple { .. })
                    ));
                    let values = input.key_values().unwrap().collect::<Vec<_>>();
                    assert_eq!(values.len(), 1);
                    assert!(
                        matches!(values[0].property().property, ResolvedTerm::Source(purrdf_core::TermValue::Iri(iri)) if iri == "https://example.org/key")
                    );
                    for assertion in [values[0].left(), values[0].right()] {
                        let CausalInputSource::Axiom(ResolvedAxiomSource::Triple {
                            predicate,
                            object,
                            ..
                        }) = assertion.source()
                        else {
                            unreachable!()
                        };
                        assert_eq!(predicate, values[0].property().property);
                        assert_eq!(object, values[0].value());
                    }
                    for member in members {
                        let original = member.refutation();
                        assert!(original.replay().is_closed());
                        assert!(original.clashes().len() >= 2);
                        assert_ne!(original.proof().branches().len(), 0);
                        let mut choice = false;
                        for branch in original.branches() {
                            assert_eq!(
                                branch.outcomes().len(),
                                branch.alternatives().len() + branch.introduced().len()
                            );
                            assert!(branch.frame().next().is_some());
                            assert!(
                                branch
                                    .alternatives()
                                    .all(|alternative| alternative.head().next().is_some())
                            );
                        }
                        for selected in original.clashes() {
                            for step in selected.steps() {
                                if let CausalObservation::Application {
                                    branch: Some((at, alternative)),
                                    ..
                                } = step.observation()
                                {
                                    let branch = original
                                        .branches()
                                        .find(|branch| branch.ordinal() == at)
                                        .unwrap();
                                    assert!(alternative < branch.outcomes().len());
                                    choice = true;
                                }
                                if let CausalObservation::Effect { cause, .. } = step.observation()
                                    && cause.kind() == CausalCauseKind::Internalized
                                {
                                    assert!(cause.normalization().is_some());
                                }
                            }
                            assert!(selected.steps().all(|step| !matches!(step.observation(),
                                CausalObservation::Effect { cause, .. } if cause.kind() == CausalCauseKind::PreparedEquality)));
                        }
                        assert!(choice);
                    }
                }
            }
        }
    }
    assert_ne!(keys, 0);
}

/// Valid unused schema steps remain inspectable in the proof but are absent from causal premises.
#[test]
fn public_causal_schema_excludes_unused_checked_implications() {
    use crate::owl_dl::{
        bounds::{SchemaRule, SchemaStep},
        concept::Concept,
    };
    let mut f = Fixture::default();
    let class = f.name("A");
    let filler = f.name("F");
    let property = f.name("p");
    let unused = f.name("Unused");
    f.edge(class, rdfs::SUB_CLASS_OF, unused);
    f.bounds(class, property, filler, 2, true, false);
    f.bounds(class, property, filler, 1, false, false);
    f.instance("i", class);
    let source = f.freeze();
    let producer = Reasoner::with_proofs(&source).unwrap();
    let answer = producer.consistency();
    let consumer = Reasoner::with_proofs(&source).unwrap();
    let ctx = consumer.borrowed_proof_context().unwrap();
    let inclusion = ctx.kb.tbox.iter().position(|&(_, sup)| matches!(ctx.kb.table.concept(sup),
        Concept::Named(term) if matches!(ctx.kb.interner.value(*term), purrdf_core::TermValue::Iri(iri) if iri == "https://example.org/Unused"))).unwrap();
    let proof = rewrite(answer.proof().unwrap(), |native| {
        for clash in &mut native.clashes {
            if let Some(schema) = &mut clash.schema {
                let after = schema
                    .bounds
                    .class_steps
                    .iter()
                    .position(|step| step.rule == SchemaRule::Assume)
                    .unwrap()
                    + 1;
                schema.bounds.class_steps.insert(
                    after,
                    SchemaStep {
                        concept: ctx.kb.tbox[inclusion].1,
                        rule: SchemaRule::Inclusion(inclusion),
                    },
                );
            }
        }
    });
    let causal = proof
        .verify_prepared_causal(&Question::Consistency, Some(answer.certificate()), &ctx)
        .unwrap();
    let mut checked = false;
    for (_, refutation) in causal.refutations() {
        for clash in refutation.clashes() {
            if let Some(schema) = clash.schema() {
                checked = true;
                assert!(
                    schema.class_derivation().count() < schema.bounds().class_derivation().len()
                );
                assert!(schema.class_derivation().all(|step| !matches!(step.concept.kind(),
                    super::super::ConceptKind::Named(ResolvedTerm::Source(purrdf_core::TermValue::Iri(iri))) if iri == "https://example.org/Unused")));
                assert_eq!(schema.qualifier_derivation().count(), 0);
            }
        }
    }
    assert!(checked);
}
