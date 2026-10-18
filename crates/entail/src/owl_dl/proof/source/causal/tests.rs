// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Actual question-bound public verification, with explicit missing-evidence refusals.

/// The original native prefix must establish the inequality closing a nominal merge.
#[test]
fn public_causal_counting_checks_reachable_merge_closure_and_generated_inequality() {
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
    let proof = answer.proof().unwrap();
    let checked = proof
        .verify_prepared_causal(&Question::Consistency, Some(answer.certificate()), &ctx)
        .unwrap();
    let (_, refutation) = checked.refutations().next().unwrap();
    assert!(refutation.replay().is_closed());
    assert_eq!(refutation.replay().checked_merges(), 1);
    let clash = refutation.clashes().next().unwrap();
    assert!(clash.is_merge());
    assert!(matches!(
        clash.root().observation(),
        CausalObservation::Merge { .. }
    ));
    assert!(
        clash
            .root()
            .reads()
            .any(|fact| matches!(fact, CausalFact::Distinct(..)))
    );
    assert!(clash.steps().any(|step| matches!(
        step.observation(),
        CausalObservation::Effect {
            mutation: CausalMutation::Distinct { .. },
            ..
        }
    )));
    assert!(
        clash
            .steps()
            .any(|step| matches!(step.observation(), CausalObservation::Counting { .. }))
    );
    for step in clash.steps() {
        for fact in step.reads() {
            if let CausalFact::Role { to, .. } = fact {
                assert_ne!(
                    to.term(),
                    Some(ResolvedTerm::Source(&source.term_value(unrelated)))
                );
            }
        }
    }
    let roundtrip = ServiceProof::decode(&proof.encode()).unwrap();
    assert_eq!(roundtrip.encode(), proof.encode());
    assert!(
        roundtrip
            .verify_prepared_causal(&Question::Consistency, Some(answer.certificate()), &ctx)
            .unwrap()
            .refutations()
            .next()
            .unwrap()
            .1
            .replay()
            .is_closed()
    );
    let legacy = rewrite(proof, |native| {
        for merge in &mut native.merges {
            merge.current = None;
        }
    });
    assert!(matches!(
        legacy.verify_prepared_causal(&Question::Consistency, Some(answer.certificate()), &ctx),
        Err(DlProofError::CausalSupport {
            kind: CausalSupportKind::Unclosed,
            ..
        })
    ));
    let missing = rewrite(proof, |native| {
        native
            .merges
            .iter_mut()
            .find(|merge| merge.current.is_some())
            .unwrap()
            .current = Some(usize::MAX);
    });
    assert!(ServiceProof::decode(&missing.encode()).is_err());
    assert!(
        missing
            .verify_prepared_causal(&Question::Consistency, Some(answer.certificate()), &ctx)
            .is_err()
    );
    let wrong_join = rewrite(proof, |native| {
        native
            .merges
            .iter_mut()
            .find(|merge| merge.current.is_some())
            .unwrap()
            .joined = crate::owl_dl::proof::NodeRef::Anonymous(u32::MAX);
    });
    assert!(matches!(
        wrong_join.verify_prepared_causal(&Question::Consistency, Some(answer.certificate()), &ctx),
        Err(DlProofError::Malformed { .. })
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

/// The native spy-point source with either signed or named-inverse counting.
fn reserved_source(named_partner: bool, blank: bool) -> (Arc<RdfDataset>, purrdf_core::TermValue) {
    let mut f = Fixture::default();
    let thing = f.builder.intern_iri(owl::THING).unwrap();
    let counted = f.name("p");
    let other = f.name("r");
    let spy_class = f.name("Spy");
    let demand = f.name("Demand");
    let spy = if blank {
        let spy = f
            .builder
            .intern_blank("spy", purrdf_core::BlankScope(91))
            .unwrap();
        f.edge(spy, rdf::TYPE, spy_class);
        spy
    } else {
        f.instance("spy", spy_class)
    };
    let nominal = f.blank();
    let members = f.list(&[spy]);
    f.edge(nominal, owl::ONE_OF, members);
    f.some(thing, counted, nominal);
    let inverse = if named_partner {
        f.name("inverse")
    } else {
        f.blank()
    };
    f.edge(inverse, owl::INVERSE_OF, counted);
    f.bounds(spy_class, inverse, thing, 2, false, false);
    f.bounds(demand, other, thing, 3, true, false);
    f.instance("subject", demand);
    let source = f.freeze();
    let spy = source.term_value(spy);
    (source, spy)
}

/// Native nominal introduction retains its actual generative trigger and typed reserved birth.
#[test]
fn public_causal_reserved_birth_retains_native_predecessor_and_original_descriptor() {
    for (named_partner, blank) in [(false, false), (true, false), (false, true)] {
        let (source, spy) = reserved_source(named_partner, blank);
        let producer = Reasoner::with_proofs(&source).unwrap();
        let answer = producer.consistency();
        assert_eq!(*answer.answer(), crate::reasoner::Verdict::False);
        let proof = ServiceProof::decode(&answer.proof().unwrap().encode()).unwrap();
        let consumer = Reasoner::with_proofs(&source).unwrap();
        let ctx = consumer.borrowed_proof_context().unwrap();
        let checked = proof
            .verify_prepared_causal(&Question::Consistency, Some(answer.certificate()), &ctx)
            .unwrap();
        let mut reserved = 0;
        let mut predecessors = 0;
        for (_, refutation) in checked.refutations() {
            assert!(refutation.replay().is_closed());
            for leaf in refutation.clashes() {
                for step in leaf.steps() {
                    for read in step.reads() {
                        if let CausalFact::Predecessor {
                            child,
                            parent,
                            node,
                            predecessor,
                            role,
                            letter,
                        } = read
                        {
                            predecessors += 1;
                            assert_ne!(child.ordinal(), parent.ordinal());
                            assert_ne!(node.ordinal(), predecessor.ordinal());
                            assert_eq!(role.inverse, !named_partner);
                            assert!(letter.inverse);
                        }
                    }
                    if let CausalObservation::Effect {
                        mutation:
                            CausalMutation::Node {
                                node,
                                reserved: true,
                                parent: None,
                                incoming: None,
                                ..
                            },
                        ..
                    } = step.observation()
                    {
                        reserved += 1;
                        assert!(node.term().is_none());
                        let identity = node.reserved_identity().unwrap();
                        assert_eq!(identity.role().inverse, !named_partner);
                        assert!(identity.index() < 2);
                        assert!(
                            matches!(identity.origin(), super::super::ResolvedNode::Individual(ResolvedTerm::Source(value)) if value == &spy)
                        );
                    }
                }
            }
        }
        assert!(reserved > 0);
        assert!(predecessors > 0);
    }
}

/// The recursive nominal/number-restriction corner, entirely from authored RDF.
fn nested_reserved_source() -> (Arc<RdfDataset>, purrdf_core::TermId) {
    let mut f = Fixture::default();
    let thing = f.builder.intern_iri(owl::THING).unwrap();
    let spy_class = f.name("Spy");
    let middle = f.name("Middle");
    let leaf = f.name("Leaf");
    let demand = f.name("Demand");
    let p = f.name("p");
    let r = f.name("r");
    let s = f.name("s");
    let spy = f.instance("spy", spy_class);
    let nominal = f.blank();
    let members = f.list(&[spy]);
    f.edge(nominal, owl::ONE_OF, members);
    f.some(leaf, p, nominal);
    f.some(middle, r, leaf);
    let inverse_p = f.blank();
    let inverse_r = f.blank();
    f.edge(inverse_p, owl::INVERSE_OF, p);
    f.edge(inverse_r, owl::INVERSE_OF, r);
    f.bounds(spy_class, inverse_p, thing, 1, false, false);
    f.bounds(leaf, inverse_r, thing, 1, false, false);
    f.bounds(demand, s, middle, 2, true, false);
    f.instance("subject", demand);
    (f.freeze(), spy)
}

/// A generated nominal can itself own a later reserved set without losing its parent scope.
#[test]
fn public_causal_reserved_nested_origin_retains_original_parent_descriptor() {
    let (source, spy) = nested_reserved_source();
    let producer = Reasoner::with_proofs(&source).unwrap();
    let answer = producer.consistency();
    // Two distinct Middle witnesses must reach one shared Leaf through r:
    // every Leaf points to spy, whose inverse-p bound is one. That Leaf's
    // inverse-r bound then identifies the two Middle witnesses: contradiction.
    assert_eq!(*answer.answer(), crate::reasoner::Verdict::False);
    let proof = ServiceProof::decode(&answer.proof().unwrap().encode()).unwrap();
    let consumer = Reasoner::with_proofs(&source).unwrap();
    let ctx = consumer.borrowed_proof_context().unwrap();
    let checked = proof
        .verify_prepared_causal(&Question::Consistency, Some(answer.certificate()), &ctx)
        .unwrap();
    let mut nested = 0;
    for (_, refutation) in checked.refutations() {
        assert!(refutation.replay().is_closed());
        for terminal in refutation.clashes() {
            for step in terminal.steps() {
                if let CausalObservation::Effect {
                    mutation:
                        CausalMutation::Node {
                            node,
                            reserved: true,
                            ..
                        },
                    ..
                } = step.observation()
                {
                    let identity = node.reserved_identity().unwrap();
                    assert_eq!(identity.index(), 0);
                    if let super::super::ResolvedNode::Reserved(parent) = identity.origin() {
                        assert_eq!(parent.index(), 0);
                        assert!(parent.role().inverse);
                        assert!(
                            matches!(parent.origin(), super::super::ResolvedNode::Individual(ResolvedTerm::Source(value)) if value == &source.term_value(spy))
                        );
                        nested += 1;
                    }
                }
            }
        }
    }
    assert!(nested > 0);
}

/// A finite graph can satisfy the ordinary counts while its blocked unravelling
/// violates them. The consumer must reject that graph before trusting unravelling.
#[test]
fn public_causal_reserved_rejects_unsaturated_blocked_completion() {
    use crate::owl_dl::proof::{ProofAnswer, node_ref, observe_completion};
    use crate::owl_dl::{
        concept::{Concept, Role},
        graph::{Assumptions, Graph},
    };
    let (source, _) = nested_reserved_source();
    let consumer = Reasoner::with_proofs(&source).unwrap();
    let ctx = consumer.borrowed_proof_context().unwrap();
    let kb = &ctx.kb;
    let term = |name: &str| {
        kb.interner
            .id_of_iri(&format!("https://example.org/{name}"))
            .unwrap()
    };
    let concept = |name: &str| kb.table.id_of(&Concept::Named(term(name))).unwrap();
    let graph = Graph::new(kb, u64::MAX);
    let mut state = graph.init_state(&Assumptions::of_kb());
    let subject = graph.root(&mut state, term("subject"));
    let spy = graph.root(&mut state, term("spy"));
    graph.ensure_at_least(
        &mut state,
        subject,
        2,
        Role::Named(term("s")),
        concept("Middle"),
    );
    let middle = graph.neighbors(&state, subject, Role::Named(term("s")));
    assert_eq!(middle.len(), 2);
    let leaf = graph.new_successor(
        &mut state,
        middle[0],
        Role::Named(term("r")),
        &[concept("Leaf")],
    );
    let nominal = kb
        .table
        .id_of(&Concept::Nominal(vec![term("spy")]))
        .unwrap();
    let target = graph.new_successor(&mut state, leaf, Role::Named(term("p")), &[nominal]);
    graph.merge_nodes(&mut state, spy, target);
    // Retain the restrictions licensed by each authored class inclusion.
    for node in [subject, spy, middle[0], middle[1], leaf] {
        for &(sub, sup) in &kb.tbox {
            if graph.has_concept(&state, node, sub) {
                graph.add_concept(&mut state, node, sup);
            }
        }
    }
    let blocks = [(node_ref(&state, middle[1]), node_ref(&state, middle[0]))];
    let completion = observe_completion(&state, &blocks);
    // Build the same public, source-bound proof envelope; replace only the model claim.
    let answer = consumer.consistency();
    let mut proof = answer.proof().unwrap().runs()[0].proof().unwrap().clone();
    proof.answer = ProofAnswer::Consistent;
    proof.completion = Some(completion);
    let replay = proof.replay_completion(&ctx);
    assert!(
        matches!(replay, Err(DlProofError::UnsaturatedIntroduction { .. })),
        "{replay:?}"
    );
}

/// Exhaustiveness, original origin and repeated native binding are checked independently.
#[test]
fn public_causal_reserved_rejects_malformed_introduction_inventory() {
    use crate::owl_dl::proof::{NodeRef, ProofGround};
    let (source, _) = reserved_source(false, false);
    let producer = Reasoner::with_proofs(&source).unwrap();
    let answer = producer.consistency();
    let ctx = producer.borrowed_proof_context().unwrap();
    let native = answer.proof().unwrap().runs()[0].proof().unwrap();
    let index = native
        .branches
        .iter()
        .position(|branch| branch.introduction_only)
        .unwrap();
    assert!(native.branches[index].introduced.len() >= 2);
    for attack in 0..7 {
        let mut forged = native.clone();
        let branch = &mut forged.branches[index];
        match attack {
            0 => branch.introduced.clear(),
            1 => {
                branch.introduced.pop();
            }
            2 => {
                branch.introduced[1] = branch.introduced[0].clone();
            }
            3 => {
                let ProofGround::EqualReserved { root, .. } = &mut branch.introduced[0].atoms[0]
                else {
                    unreachable!()
                };
                root.origin = NodeRef::Anonymous(u32::MAX);
            }
            4 => {
                *branch.frame.last_mut().unwrap() = branch.node.clone();
            }
            5 => {
                branch.introduction_only = false;
            }
            6 => {
                branch.introduced.swap(0, 1);
            }
            _ => unreachable!(),
        }
        // No forged inventory may acquire credibility merely by surviving its wire codec.
        if let Ok(decoded) = DlProof::decode(&forged.encode()) {
            assert!(
                decoded.replay_branch(index, &ctx).is_err(),
                "attack {attack}"
            );
        }
    }
}

/// Structural NI shape is insufficient: a named root is not a generative predecessor.
#[test]
fn public_causal_reserved_introduction_rejects_forged_nonblockable_target() {
    let (source, _) = reserved_source(false, false);
    let producer = Reasoner::with_proofs(&source).unwrap();
    let answer = producer.consistency();
    let consumer = Reasoner::with_proofs(&source).unwrap();
    let ctx = consumer.borrowed_proof_context().unwrap();
    let forged = rewrite(answer.proof().unwrap(), |native| {
        for branch in &mut native.branches {
            for alternative in &mut branch.introduced {
                for atom in &mut alternative.atoms {
                    if let crate::owl_dl::proof::ProofGround::EqualReserved { node, root } = atom {
                        *node = root.origin().clone();
                    }
                }
            }
            if branch.introduction_only {
                for node in &mut branch.frame[1..] {
                    *node = branch.node.clone();
                }
            }
        }
        for link in &mut native.support {
            if link
                .application
                .head
                .iter()
                .any(|atom| matches!(atom, crate::owl_dl::hyper::Ground::EqualReserved(..)))
            {
                let root = link.application.frame[0];
                link.application.frame[1..].fill(root);
            }
            for atom in &mut link.application.head {
                if let crate::owl_dl::hyper::Ground::EqualReserved(node, _) = atom {
                    *node = link.application.frame[0];
                }
            }
        }
        for merge in &mut native.merges {
            if let crate::owl_dl::proof::MergeLicence::Branch {
                branch,
                ordinal,
                atom,
            } = merge.licence
            {
                let choice = native.branches[branch]
                    .alternatives
                    .iter()
                    .chain(&native.branches[branch].introduced)
                    .nth(ordinal)
                    .unwrap();
                if let crate::owl_dl::proof::ProofGround::EqualReserved { node, .. } =
                    &choice.atoms[atom]
                {
                    merge.right = node.clone();
                    if merge.clashed {
                        merge.joined = node.clone();
                    }
                }
            }
        }
    });
    let native = forged.runs()[0].proof().unwrap();
    let mut structural = 0;
    for (index, branch) in native.branches().iter().enumerate() {
        if !branch.introduced().is_empty() {
            native.replay_branch(index, &ctx).unwrap();
            structural += 1;
        }
    }
    assert!(structural > 0);
    assert!(
        matches!(forged.verify_prepared_causal(&Question::Consistency, Some(answer.certificate()), &ctx), Err(DlProofError::Malformed { detail }) if detail.contains("generative predecessor"))
    );
}

/// Every original nominal alternative must reach its own independently checked merge.
#[test]
fn public_causal_merge_closes_every_nominal_branch_and_rejects_wrong_prefix() {
    let mut f = Fixture::default();
    let class = f.name("A");
    let filler = f.name("F");
    let property = f.name("p");
    f.bounds(class, property, filler, 3, true, false);
    let nominal = f.blank();
    let first = f.name("first");
    let second = f.name("second");
    let members = f.list(&[first, second]);
    f.edge(nominal, owl::ONE_OF, members);
    f.edge(filler, rdfs::SUB_CLASS_OF, nominal);
    f.instance("subject", class);
    let source = f.freeze();
    let producer = Reasoner::with_proofs(&source).unwrap();
    let answer = producer.consistency();
    assert_eq!(*answer.answer(), crate::reasoner::Verdict::False);
    let proof = ServiceProof::decode(&answer.proof().unwrap().encode()).unwrap();
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
    assert!(refutation.clashes().all(|clash| clash.is_merge()
        && matches!(clash.root().observation(), CausalObservation::Merge { .. })));
    let forged = rewrite(&proof, |native| {
        let first = native.merges.iter().find_map(|step| step.current).unwrap();
        for step in &mut native.merges {
            if step.current.is_some() {
                step.current = Some(first);
            }
        }
    });
    assert!(
        forged
            .verify_prepared_causal(&Question::Consistency, Some(answer.certificate()), &ctx)
            .is_err()
    );
}

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
