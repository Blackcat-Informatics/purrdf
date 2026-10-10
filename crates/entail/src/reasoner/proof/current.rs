// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Bind new prepared-clash support to the consumer's actual service question.

use super::{
    ClaimBasis, ClaimSubject, DlAxiom, DlProofContext, DlProofError, Question, RunAssumptions,
    ServiceProof,
};
use crate::owl_dl::concept::{Concept, Role};
use crate::owl_dl::{Kb, class_concept_ids};
use purrdf_core::TermValue;

pub(super) fn check_run(
    proof: &ServiceProof,
    index: usize,
    ctx: &DlProofContext,
) -> Result<(), DlProofError> {
    let assumptions = &proof.runs[index].assumptions;
    if index == 0 && bare(assumptions) {
        return Ok(());
    }
    if !assumptions.include_abox {
        return Err(rejected());
    }
    let valid = proof.claims.iter().any(|claim| {
        let refers = match &claim.basis {
            ClaimBasis::ClosedRefutation { runs } => runs.contains(&index),
            ClaimBasis::ExhibitedModel { run }
            | ClaimBasis::CounterModel { run }
            | ClaimBasis::Undecided { run } => *run == index,
            _ => false,
        };
        refers
            && authorized(&proof.question, &claim.subject, &ctx.kb)
            && matches_subject(&ctx.kb, &claim.subject, assumptions)
            && match (&claim.subject, &claim.basis) {
                (ClaimSubject::Axiom { axiom }, ClaimBasis::ClosedRefutation { runs }) => {
                    match axiom.as_ref() {
                        DlAxiom::EquivalentClasses { left, right } => {
                            [(left, right), (right, left)].iter().all(|(sub, sup)| {
                                runs.iter().any(|&at| {
                                    proof.runs.get(at).is_some_and(|run| {
                                        subsumption(&ctx.kb, &run.assumptions, sub, sup)
                                    })
                                })
                            })
                        }
                        _ => true,
                    }
                }
                _ => true,
            }
    });
    if valid { Ok(()) } else { Err(rejected()) }
}

fn rejected() -> DlProofError {
    DlProofError::AnswerNotCovered {
        detail: "prepared-clash assumptions are not the requested claim's refutation premises"
            .to_owned(),
    }
}

fn bare(assumptions: &RunAssumptions) -> bool {
    assumptions.include_abox
        && assumptions.types.is_empty()
        && assumptions.roles.is_empty()
        && assumptions.fresh_types.is_empty()
}

fn authorized(question: &Question, subject: &ClaimSubject, kb: &Kb) -> bool {
    match (question, subject) {
        (Question::Consistency, ClaimSubject::Consistent) => true,
        (
            Question::ClassSatisfiability { class: requested },
            ClaimSubject::ClassSatisfiable { class },
        ) => requested == class,
        (Question::Classification { classes }, ClaimSubject::Subsumption { sub, sup }) => {
            classes.contains(sub) && classes.contains(sup)
        }
        (
            Question::Realization {
                individuals,
                classes,
            },
            ClaimSubject::Type { individual, class },
        ) => individuals.contains(individual) && classes.contains(class),
        (
            Question::InstanceRetrieval { class: requested },
            ClaimSubject::Type { individual, class },
        ) => {
            requested == class
                && kb
                    .interner
                    .id_of(individual)
                    .is_some_and(|id| kb.individuals.contains(&id))
        }
        (Question::AxiomEntailment { axiom: requested }, ClaimSubject::Axiom { axiom }) => {
            requested == axiom
        }
        _ => false,
    }
}

fn class_id(kb: &Kb, class: &TermValue) -> Option<u32> {
    let term = kb.interner.id_of(class)?;
    // Read the input-bound built-in ids without interning any checker-selected name.
    let thing = kb
        .interner
        .id_of(&TermValue::Iri(purrdf_iri::vocab::owl::THING.to_owned()))?;
    let nothing = kb
        .interner
        .id_of(&TermValue::Iri(purrdf_iri::vocab::owl::NOTHING.to_owned()))?;
    let concept = class_concept_ids(term, thing, nothing);
    kb.table.id_of(&concept)
}

fn fresh(assumptions: &RunAssumptions, concepts: &[u32]) -> bool {
    assumptions.types.is_empty()
        && assumptions.roles.is_empty()
        && assumptions.fresh_types == concepts
}

fn typed(assumptions: &RunAssumptions, individual: u32, concept: u32) -> bool {
    assumptions.roles.is_empty()
        && assumptions.fresh_types.is_empty()
        && assumptions.types == [(individual, concept)]
}

fn subsumption(kb: &Kb, assumptions: &RunAssumptions, sub: &TermValue, sup: &TermValue) -> bool {
    let Some(sub) = class_id(kb, sub) else {
        return false;
    };
    let Some(sup) = class_id(kb, sup) else {
        return false;
    };
    fresh(assumptions, &[sub, kb.table.negate(sup)])
}

fn membership(
    kb: &Kb,
    assumptions: &RunAssumptions,
    individual: &TermValue,
    class: &TermValue,
) -> bool {
    let Some(individual) = kb.interner.id_of(individual) else {
        return false;
    };
    let Some(class) = class_id(kb, class) else {
        return false;
    };
    typed(assumptions, individual, kb.table.negate(class))
}

fn matches_subject(kb: &Kb, subject: &ClaimSubject, assumptions: &RunAssumptions) -> bool {
    match subject {
        ClaimSubject::Consistent => bare(assumptions),
        ClaimSubject::ClassSatisfiable { class } => {
            class_id(kb, class).is_some_and(|id| fresh(assumptions, &[id]))
        }
        ClaimSubject::Subsumption { sub, sup } => subsumption(kb, assumptions, sub, sup),
        ClaimSubject::Type { individual, class } => membership(kb, assumptions, individual, class),
        ClaimSubject::Axiom { axiom } => matches_axiom(kb, axiom, assumptions),
        ClaimSubject::Module { .. } => false,
    }
}

fn matches_axiom(kb: &Kb, axiom: &DlAxiom, assumptions: &RunAssumptions) -> bool {
    match axiom {
        DlAxiom::SubClassOf { sub, sup } => subsumption(kb, assumptions, sub, sup),
        DlAxiom::EquivalentClasses { left, right } => {
            subsumption(kb, assumptions, left, right) || subsumption(kb, assumptions, right, left)
        }
        DlAxiom::DisjointClasses { left, right } => {
            let (Some(left), Some(right)) = (class_id(kb, left), class_id(kb, right)) else {
                return false;
            };
            fresh(assumptions, &[left, right])
        }
        DlAxiom::ClassAssertion { individual, class } => {
            membership(kb, assumptions, individual, class)
        }
        DlAxiom::SameIndividual { left, right } | DlAxiom::DifferentIndividuals { left, right } => {
            let (Some(left), Some(right)) = (kb.interner.id_of(left), kb.interner.id_of(right))
            else {
                return false;
            };
            let concept = Concept::nominal(vec![right]);
            let Some(id) = kb.table.id_of(&concept) else {
                return false;
            };
            let id = if matches!(axiom, DlAxiom::SameIndividual { .. }) {
                kb.table.negate(id)
            } else {
                id
            };
            typed(assumptions, left, id)
        }
        DlAxiom::ObjectPropertyAssertion {
            subject,
            property,
            object,
        } => {
            let (Some(subject), Some(property), Some(object)) = (
                kb.interner.id_of(subject),
                kb.interner.id_of(property),
                kb.interner.id_of(object),
            ) else {
                return false;
            };
            negative_reach(kb, assumptions, subject, property, object, false)
        }
        DlAxiom::SubObjectPropertyOf { sub, sup } => {
            let (Some(sub), Some(sup)) = (kb.interner.id_of(sub), kb.interner.id_of(sup)) else {
                return false;
            };
            let [(subject, property, object)] = assumptions.roles.as_slice() else {
                return false;
            };
            *property == sub
                && *subject != *object
                && *subject >= kb.source_terms
                && *object >= kb.source_terms
                && negative_reach(kb, assumptions, *subject, sup, *object, true)
        }
    }
}

fn negative_reach(
    kb: &Kb,
    assumptions: &RunAssumptions,
    subject: u32,
    property: u32,
    object: u32,
    with_role: bool,
) -> bool {
    let [(individual, concept)] = assumptions.types.as_slice() else {
        return false;
    };
    if *individual != subject
        || !assumptions.fresh_types.is_empty()
        || (!with_role && !assumptions.roles.is_empty())
    {
        return false;
    }
    if *concept as usize >= kb.table.len() {
        return false;
    }
    let held = kb.table.concept(*concept);
    match held {
        Concept::All(Role::Named(role), filler) if *role == property => {
            matches!(filler.as_ref(), Concept::Not(inner)
            if matches!(inner.as_ref(), Concept::Nominal(members) if members.as_slice() == [object]))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Claim, Service};
    use super::*;
    use crate::reasoner::schema_tests::{shared_schema, term};
    use crate::reasoner::{Reasoner, Verdict};

    #[test]
    fn a_fresh_class_refutation_cannot_be_relabelled_as_an_ontology_contradiction() {
        let dataset = shared_schema(1, 0, true);
        let mut producer = Reasoner::with_proofs(&dataset).unwrap();
        let answer = producer.class_satisfiability(&term("A")).unwrap();
        let mut forged = answer.proof().unwrap().clone();
        forged.service = Service::Consistency;
        forged.question = Question::Consistency;
        let index = forged
            .runs
            .iter()
            .position(|run| !run.assumptions.fresh_types.is_empty())
            .unwrap();
        forged.claims = vec![Claim::new(
            ClaimSubject::Consistent,
            ClaimBasis::ClosedRefutation { runs: vec![index] },
        )];
        let context = Reasoner::with_proofs(&dataset)
            .unwrap()
            .proof_context()
            .unwrap();
        assert!(
            forged
                .verify(&dataset, &Question::Consistency, None, &context)
                .is_err()
        );
    }

    #[test]
    fn equivalence_requires_both_refutation_directions_not_two_copies_of_one() {
        let dataset = shared_schema(1, 0, true);
        let axiom = DlAxiom::EquivalentClasses {
            left: term("A"),
            right: term("F"),
        };
        let mut producer = Reasoner::with_proofs(&dataset).unwrap();
        let answer = producer.entails(&axiom).unwrap();
        assert_eq!(*answer.answer(), Verdict::False);
        let mut forged = answer.proof().unwrap().clone();
        let question = Question::AxiomEntailment {
            axiom: Box::new(axiom.clone()),
        };
        let index = forged
            .runs
            .iter()
            .position(|run| {
                run.proof.as_ref().is_some_and(|proof| {
                    proof
                        .clashes()
                        .iter()
                        .any(|step| step.schema_evidence().is_some())
                })
            })
            .unwrap();
        forged.claims = vec![Claim::new(
            ClaimSubject::Axiom {
                axiom: Box::new(axiom),
            },
            ClaimBasis::ClosedRefutation {
                runs: vec![index, index],
            },
        )];
        let mut checker = Reasoner::with_proofs(&dataset).unwrap();
        checker.prepare(&question);
        assert!(
            forged
                .verify(&dataset, &question, None, &checker.proof_context().unwrap())
                .is_err()
        );
    }
}
