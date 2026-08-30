// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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

use super::certificate::{Session, Verdict};
use super::classify::Subsumptions;
use super::proof::{Claim, ClaimSubject, refutation_claim};
use super::term_key;
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
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Realization {
    /// Every established `a : C`, sorted.
    types: Vec<(TermValue, TermValue)>,
    /// The subset of [`Self::types`] that is most specific, sorted.
    direct: Vec<(TermValue, TermValue)>,
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

    /// The MOST SPECIFIC entailed types: those `a : C` for which no entailed `a : D` has
    /// `D` strictly subsumed by `C`. Sorted.
    ///
    /// Two equivalent classes that are both types are BOTH listed. Collapsing them to a
    /// representative would answer a question about the hierarchy inside an answer about
    /// an individual, and a caller who wants the collapse has
    /// [`ClassHierarchy::equivalences`](super::ClassHierarchy::equivalences) to do it with.
    #[must_use]
    pub fn direct_types(&self) -> &[(TermValue, TermValue)] {
        &self.direct
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
        let n = classes.len();
        let cells = individuals.len() * n;
        let records = session.records();
        // `entailed[i][c]` — whether individual `i` was established an instance of the
        // `c`-th class. Dense and positional, so no map order reaches the answer.
        let mut entailed = vec![false; cells];
        // WHICH run decided each pair, in the same dense layout: a realization's answer
        // binding has to name one run per (individual, class) question, and reconstructing
        // that afterwards from a count would be a guess about the order they were asked in.
        // Both of these exist for the binding alone, and so only when there is one.
        let mut runs = if records {
            vec![0_usize; cells]
        } else {
            Vec::new()
        };
        let mut verdicts = if records {
            vec![Verdict::Unknown; cells]
        } else {
            Vec::new()
        };
        for (i, &individual) in individuals.iter().enumerate() {
            for (c, &(_, concept)) in classes.iter().enumerate() {
                let verdict = is_instance(session, individual, concept);
                entailed[i * n + c] = verdict.is_true();
                if records {
                    verdicts[i * n + c] = verdict;
                    runs[i * n + c] = session.last_run();
                }
            }
        }

        let kb = session.kb();
        let individual_name = |i: usize| kb.interner.value(individuals[i]).clone();
        let class_name = |c: usize| kb.interner.value(classes[c].0).clone();

        let mut types = Vec::new();
        let mut direct = Vec::new();
        let mut claims = Vec::new();
        for i in 0..individuals.len() {
            for c in 0..n {
                // Every pair the realizer ASKED about gets a claim, established or not: an
                // answer binding that listed only the positive ones would leave a reader
                // unable to tell a type the search ruled out from one it never reached.
                if records {
                    claims.push(refutation_claim(
                        ClaimSubject::Type {
                            individual: individual_name(i),
                            class: class_name(c),
                        },
                        verdicts[i * n + c],
                        &[runs[i * n + c]],
                    ));
                }
                if !entailed[i * n + c] {
                    continue;
                }
                types.push((individual_name(i), class_name(c)));
                let specialized =
                    (0..n).any(|d| entailed[i * n + d] && m.holds(d, c) && !m.holds(c, d));
                if !specialized {
                    direct.push((individual_name(i), class_name(c)));
                }
            }
        }

        let pair = |(a, b): &(TermValue, TermValue)| (term_key(a), term_key(b));
        types.sort_by_key(pair);
        direct.sort_by_key(pair);
        (Self { types, direct }, claims)
    }
}
