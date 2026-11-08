// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The realizer: the entailed types of an ontology's named individuals.
//!
//! Realization is instance retrieval run the other way round — for each named individual,
//! which of the named classes is it entailed to belong to — and it is decided the same
//! way, by refuting `a : ¬C`. What makes it a service rather than a loop is the second
//! list: the MOST SPECIFIC entailed types, which need the class hierarchy to compute and
//! are the only part of a realization a human reads. So this module consumes the
//! subsumption matrix the classifier already built rather than re-deciding
//! subsumption, which is why the two are separate values and not one function.
//!
//! # Determinism
//!
//! Individuals are visited in ascending interned-term-id order, classes in the classifier's
//! order, and both emitted sequences are sorted by a total, dataset-independent term key.

use purrdf_core::TermValue;

use super::certificate::{Certified, DirectnessRefusal, Session, Verdict};
use super::classify::Subsumptions;
use super::proof::{Claim, ClaimSubject, refutation_claim};
use crate::owl_dl::graph::Assumptions;

/// Whether `KB ⊨ individual : concept`, by refuting `individual : ¬concept`.
///
/// No fresh symbol is needed: the individual is already named, and the negated concept is
/// an ordinary assertion about it.
pub(crate) fn is_instance(session: &mut Session<'_>, individual: u32, concept: u32) -> Verdict {
    let neg = session.kb().table.negate(concept);
    session.refutes(&Assumptions {
        types: &[(individual, neg)],
        ..Assumptions::of_kb()
    })
}

/// The realized types of an ontology's named individuals.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct Realization {
    /// Every established `a : C`, sorted.
    types: Vec<(TermValue, TermValue)>,
    /// The subset of [`Self::types`] that is most specific, sorted.
    pub(super) direct: Vec<(TermValue, TermValue)>,
}

purrdf_hash::debug_non_exhaustive! {
    /// Unproved direct candidates remain private, including in debug output.
    Realization { types }
}

impl Realization {
    /// Every established type assertion `a : C` for a named individual `a` and a named
    /// class `C`, sorted.
    ///
    /// `owl:Thing` is a type of every individual, and appears as one: it is entailed, and
    /// omitting an entailed answer because it is obvious is how an answer set stops being
    /// an answer set.
    #[must_use]
    pub fn types(&self) -> &[(TermValue, TermValue)] {
        &self.types
    }

    /// Realize `individuals` against `classes`, reusing the classifier's matrix for the
    /// most-specific pass.
    ///
    /// The claim list is EMPTY unless the session is recording a proof term. It is one claim
    /// per (individual, class) pair, each cloning two terms, so a realization that built one
    /// for a proof nobody asked for would pay more for the answer binding than for the answer
    /// — which is why the two side matrices it is derived from are not allocated either.
    pub(crate) fn decide(
        session: &mut Session<'_>,
        individuals: &[u32],
        classes: &[(u32, u32)],
        m: &Subsumptions,
    ) -> (Self, Vec<Claim>) {
        Self::decide_until(session, individuals, classes, m, || true)
    }

    /// Retain answers and their run bindings before polling the next decision.
    pub(super) fn decide_until(
        session: &mut Session<'_>,
        individuals: &[u32],
        classes: &[(u32, u32)],
        m: &Subsumptions,
        mut poll: impl FnMut() -> bool,
    ) -> (Self, Vec<Claim>) {
        let n = classes.len();
        let mut entailed = vec![false; individuals.len() * n];
        let mut types = Vec::new();
        let mut direct = Vec::new();
        let mut claims = Vec::new();
        'decisions: for (i, &individual) in individuals.iter().enumerate() {
            for (c, &(class, concept)) in classes.iter().enumerate() {
                if !poll() {
                    break 'decisions;
                }
                let verdict = is_instance(session, individual, concept);
                entailed[i * n + c] = verdict.is_true();
                let kb = session.kb();
                if session.records() {
                    claims.push(refutation_claim(
                        ClaimSubject::Type {
                            individual: kb.interner.value(individual).clone(),
                            class: kb.interner.value(class).clone(),
                        },
                        verdict,
                        &[session.last_run()],
                    ));
                }
                if verdict.is_true() {
                    types.push((
                        kb.interner.value(individual).clone(),
                        kb.interner.value(class).clone(),
                    ));
                }
            }
        }
        let kb = session.kb();
        'direct: for (i, &individual) in individuals.iter().enumerate() {
            for (c, &(class, _)) in classes.iter().enumerate() {
                if !poll() {
                    break 'direct;
                }
                if !entailed[i * n + c] {
                    continue;
                }
                let mut specialized = false;
                for d in 0..n {
                    if !poll() {
                        break 'direct;
                    }
                    if entailed[i * n + d] && m.holds(d, c) && !m.holds(c, d) {
                        specialized = true;
                        break;
                    }
                }
                if !specialized {
                    direct.push((
                        kb.interner.value(individual).clone(),
                        kb.interner.value(class).clone(),
                    ));
                }
            }
        }

        types.sort();
        direct.sort();
        (Self { types, direct }, claims)
    }
}

impl Certified<Realization> {
    /// Exact most-specific types, sorted, including every equivalent class that
    /// is a type. No strictly more specific entailed type exists.
    ///
    /// # Errors
    ///
    /// Returns the original certificate in [`DirectnessRefusal`] unless the
    /// complete supplied ontology was decided. Positive types remain available
    /// through [`Self::answer`].
    pub fn direct_types(&self) -> Result<&[(TermValue, TermValue)], DirectnessRefusal<'_>> {
        Ok(&self.direct_answer()?.direct)
    }
}
