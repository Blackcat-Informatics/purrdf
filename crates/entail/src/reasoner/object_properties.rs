// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Certified semantic classification of the source's named object properties.
//!
//! The universe follows the reasoner's default-graph reverse mapping: declarations,
//! accepted assertions, restrictions, characteristics, property links, keys and
//! chains contribute their named operands. Anonymous inverse expressions contribute
//! the underlying name. Query-only names never enlarge this universe.
//!
//! Source declarations and typed uses constrain object, data or annotation kind.
//! Subproperty links propagate any of those kinds; equivalence and disjointness
//! propagate only object/data kinds. Key members remain independent. A domain,
//! functional declaration or unqualified cardinality alone leaves the kind open;
//! this service refuses unresolved or contradictory evidence instead of guessing.
//! The ordinary reasoner services retain their existing admission behavior.
//!
//! Every semantic inclusion is decided against the whole native knowledge base,
//! including the ABox. Empty properties therefore have their vacuous inclusions,
//! and equivalent properties collapse only for direct-edge reporting. Reflexive
//! pairs are omitted; all distinct equivalent names remain in the full relation.
//! Top and bottom participate without modifying unrelated reasoner construction.
//! Data-property classification and configured SPARQL relations are separate work.

use purrdf_core::TermValue;
use purrdf_iri::vocab::owl::{BOTTOM_OBJECT_PROPERTY, TOP_OBJECT_PROPERTY};

use super::axiom::{holds_role_inclusion, reaches};
use super::certificate::{Certified, DirectnessRefusal, Verdict};
use super::hierarchy::Relations;
use super::proof::{ClaimSubject, Question, refutation_claim};
use super::{DlAxiom, Reasoner};
use crate::EntailError;
use crate::owl_dl::parser;

/// Established semantic inclusions and equivalences among source object properties.
/// Top and bottom participate even when absent from the source. Direct edges are
/// strict edges between equivalence representatives, ordered by canonical term order.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct ObjectPropertyHierarchy {
    properties: Vec<TermValue>,
    relations: Relations,
}

purrdf_hash::debug_non_exhaustive!(ObjectPropertyHierarchy { properties });

impl ObjectPropertyHierarchy {
    /// The complete source-fixed named object-property universe, including built-ins.
    #[must_use]
    pub fn properties(&self) -> &[TermValue] {
        &self.properties
    }

    /// Established nonreflexive inclusions, sorted; incomplete results are positive evidence only.
    #[must_use]
    pub fn subsumptions(&self) -> &[(TermValue, TermValue)] {
        &self.relations.subsumptions
    }

    /// Established equivalences, one pair per two distinct names, sorted.
    #[must_use]
    pub fn equivalences(&self) -> &[(TermValue, TermValue)] {
        &self.relations.equivalences
    }
}

impl Certified<ObjectPropertyHierarchy> {
    /// Exact strict direct inclusions between the least named equivalence representatives.
    ///
    /// # Errors
    /// Refuses with the original certificate if any ontology boundary or undecided
    /// pair could conceal an intermediate property.
    pub fn direct_subsumptions(&self) -> Result<&[(TermValue, TermValue)], DirectnessRefusal<'_>> {
        Ok(&self.direct_answer()?.relations.direct)
    }
}

pub(super) struct Prepared {
    properties: Vec<u32>,
    x: u32,
    y: u32,
    negative: Vec<u32>,
}

impl Reasoner {
    /// The source's object-property classification question, independent of prior queries.
    ///
    /// # Errors
    /// Refuses ambiguous/conflicting source property kinds or cancellation during preparation.
    pub fn object_property_classification_question(&self) -> Result<Question, EntailError> {
        let mut properties: Vec<TermValue> = self
            .object_property_ids()?
            .iter()
            .map(|&id| self.kb.interner.value(id).clone())
            .collect();
        for iri in [TOP_OBJECT_PROPERTY, BOTTOM_OBJECT_PROPERTY] {
            properties.push(TermValue::Iri(iri.to_owned()));
        }
        properties.sort();
        properties.dedup();
        Ok(Question::ObjectPropertyClassification { properties })
    }

    fn object_property_ids(&self) -> Result<Vec<u32>, EntailError> {
        self.kb.property_kinds.objects(&self.kb.interner, || {
            if purrdf_core::governor::is_stopped(self.kb.stop.as_deref()) {
                Err(EntailError::Stopped)
            } else {
                Ok(())
            }
        })
    }

    /// The service and its proof consumer use exactly the same witness preparation.
    pub(super) fn prepare_object_properties(&mut self) -> Result<Prepared, EntailError> {
        let mut properties = self.object_property_ids()?;
        properties.extend(
            [TOP_OBJECT_PROPERTY, BOTTOM_OBJECT_PROPERTY]
                .map(|iri| self.kb.interner.intern_iri(iri)),
        );
        properties.sort_by(|&a, &b| self.kb.interner.value(a).cmp(self.kb.interner.value(b)));
        properties.dedup();
        parser::constrain_bottom_roles(&mut self.kb, properties.iter().copied());
        let x = self.fresh.mint(&mut self.kb.interner);
        let y = self.fresh.mint(&mut self.kb.interner);
        let signal = self.kb.stop.clone();
        let mut poll = || {
            if purrdf_core::governor::is_stopped(signal.as_deref()) {
                Err(EntailError::Stopped)
            } else {
                Ok(())
            }
        };
        let mut negative = Vec::new();
        for &property in &properties {
            poll()?;
            negative.push(self.kb.table.intern(reaches(property, y)));
        }
        self.kb.finalize_until(&mut poll)?;
        for concept in &mut negative {
            poll()?;
            *concept = self.kb.table.negate(*concept);
        }
        Ok(Prepared {
            properties,
            x,
            y,
            negative,
        })
    }

    /// Classify all source-named object properties by native semantic entailment.
    ///
    /// Preparation shares one fresh witness pair and the super-property concepts;
    /// one session performs consistency and every inclusion decision. Existing step
    /// and work caps apply per decision, and the certificate reports their totals.
    /// These are not aggregate query or whole-operation allocation limits.
    ///
    /// # Errors
    /// Refuses unresolved/conflicting property kinds, an inconsistent ontology,
    /// source/storage failures, or cancellation during preparation. Undecided
    /// reasoning returns its partial answer and original incomplete certificate.
    pub fn classify_object_properties(
        &mut self,
    ) -> Result<Certified<ObjectPropertyHierarchy>, EntailError> {
        let prepared = self.prepare_object_properties()?;
        let properties: Vec<_> = prepared
            .properties
            .iter()
            .map(|&id| self.kb.interner.value(id).clone())
            .collect();
        let question = Question::ObjectPropertyClassification {
            properties: properties.clone(),
        };
        let (mut session, usable) = self.open()?;
        let n = properties.len();
        let cells = n
            .checked_mul(n)
            .ok_or(purrdf_lex::allocation::StorageError::SizeOverflow)?;
        let mut verdicts = vec![Verdict::Unknown; cells];
        let mut claims = Vec::new();
        let mut relations = Relations::default();
        if usable {
            for i in 0..n {
                verdicts[i * n + i] = Verdict::True;
            }
            'pairs: for i in 0..n {
                for j in i + 1..n {
                    for (sub, sup) in [(i, j), (j, i)] {
                        let verdict = holds_role_inclusion(
                            &mut session,
                            prepared.x,
                            prepared.properties[sub],
                            prepared.y,
                            prepared.negative[sup],
                        );
                        verdicts[sub * n + sup] = verdict;
                        if session.records() {
                            claims.push(refutation_claim(
                                ClaimSubject::Axiom {
                                    axiom: Box::new(DlAxiom::SubObjectPropertyOf {
                                        sub: properties[sub].clone(),
                                        sup: properties[sup].clone(),
                                    }),
                                },
                                verdict,
                                &[session.last_run()],
                            ));
                        }
                    }
                    // Retain positive answers together with their claims before any
                    // cancellation can interrupt quotient/direct-edge reduction.
                    relations.record_pair(
                        properties[i].clone(),
                        properties[j].clone(),
                        verdicts[i * n + j].is_true(),
                        verdicts[j * n + i].is_true(),
                    );
                    if purrdf_core::governor::is_stopped(self.kb.stop.as_deref()) {
                        break 'pairs;
                    }
                }
            }
        }
        relations.reduce(
            n,
            &|i| properties[i].clone(),
            &|i, j| verdicts[i * n + j].is_true(),
            &mut || !purrdf_core::governor::is_stopped(self.kb.stop.as_deref()),
        );
        relations.sort();
        if purrdf_core::governor::is_stopped(self.kb.stop.as_deref()) {
            // A stop during reduction is observed by the original decision owner.
            // Bind that actual stopped run as an undecided reflexive question;
            // undecided claims assert no extra positive answer.
            let verdict = holds_role_inclusion(
                &mut session,
                prepared.x,
                prepared.properties[0],
                prepared.y,
                prepared.negative[0],
            );
            if session.records() {
                claims.push(refutation_claim(
                    ClaimSubject::Axiom {
                        axiom: Box::new(DlAxiom::SubObjectPropertyOf {
                            sub: properties[0].clone(),
                            sup: properties[0].clone(),
                        }),
                    },
                    verdict,
                    &[session.last_run()],
                ));
            }
        }
        session.check_storage()?;
        Ok(self.seal(
            session,
            ObjectPropertyHierarchy {
                properties,
                relations,
            },
            || (question, claims),
        ))
    }
}
