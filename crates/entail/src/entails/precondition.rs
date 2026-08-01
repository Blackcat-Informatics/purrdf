// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! What a FAILED match is allowed to mean.
//!
//! # The asymmetry this module exists for
//!
//! A found homomorphism is a proof of entailment and needs no precondition: every rule of
//! every lane this service runs is sound, so every closure triple is entailed and a
//! conclusion mapped into the closure is entailed too.
//!
//! A missing homomorphism is a proof of NON-entailment only if the procedure is also
//! COMPLETE for the premise at hand. That is not a property of the search; it is a property
//! of the input, and every regime states its own condition for it. This module is where
//! those conditions live, one arm per regime, each naming the theorem it is the hypothesis
//! of. When a condition fails the outcome is [`EntailmentOutcome::Undecided`], never
//! [`EntailmentOutcome::NotEntailed`]: answering "not entailed" from an incomplete
//! procedure is an overclaim, and it is the specific overclaim that turns a reasoner's
//! silence into a false statement about a caller's data.
//!
//! [`EntailmentOutcome::Undecided`]: super::EntailmentOutcome::Undecided
//! [`EntailmentOutcome::NotEntailed`]: super::EntailmentOutcome::NotEntailed
//!
//! # The conditions, and the theorem each one comes from
//!
//! * **`Simple`** — none. RDF 1.2 Semantics' interpolation lemma says a graph `G` simply
//!   entails `E` exactly when `E` has an instance that is a subgraph of `G`, which is the
//!   homomorphism this service computes. The procedure is the definition, so there is
//!   nothing left to be incomplete about.
//! * **`RDF` / `RDFS`** — the closure rules are complete for RDF/RDFS entailment, with two
//!   qualifications this crate MEASURES rather than assumes. The first is the four rules
//!   (`rdfD1`, `rdfD1a`, `rdfs14`, `rdfs14a`) that conclude about a FRESH blank node: their
//!   conclusions are withheld, because a surrogate is not a term of the scoping graph and
//!   therefore not an answer a SPARQL entailment regime admits, so a run that withheld any
//!   is a run whose closure is smaller than the regime's. The second is the AXIOMATIC
//!   triples, which are an infinite schema over the container-membership properties
//!   `rdf:_1`, `rdf:_2`, …: a finite closure holds finitely many of them, so a question that
//!   mentions one cannot be refuted by its absence.
//! * **`OWL-RL`** — OWL 2 Profiles §4.3 Theorem PR1. The rule set is sound for arbitrary
//!   RDF graphs, and it is complete for a premise that is inside the OWL 2 RL SYNTAX. This
//!   crate already has an executable form of that hypothesis in [`profile()`], which is
//!   purely syntactic and runs no reasoner, so the precondition is checked rather than
//!   assumed. A premise outside RL is exactly the case where a rule the profile does not
//!   state would have been needed, so its non-match says nothing.
//! * **`D`** — no condition makes it decide. This crate realizes D-entailment as Simple
//!   entailment plus the five `dt-*` rules of OWL 2 Profiles §4.3 Table 8, and states no
//!   theorem that those five are complete for D-entailment: the rest of it quantifies over
//!   value spaces that are infinite, which is what the run's own
//!   [`Construct::DatatypeValueSpace`](crate::Construct::DatatypeValueSpace) boundary
//!   reports. So the `D` lane can PROVE an entailment and can never refute one, and it says
//!   so instead of letting an unreachable rule read as a refutation.
//!
//! # The conditions are checked against the run, not recited
//!
//! Two of the four read the run's own [`ReasoningReport`] — the withheld-surrogate count is
//! a measurement of THIS run, not a standing disclaimer — and one reads the question. That
//! is deliberate: a condition stated in prose beside a code path is a condition that stops
//! being true without anything noticing.

use purrdf_core::RdfDataset;

use crate::Regime;
use crate::entails::pattern::{Pat, PatTriple};
use crate::entails::warrant::EntailmentMechanism;
use crate::reasoner::{OwlProfile, ProfileViolation, profile};
use crate::report::ReasoningReport;

/// The `rdf:` namespace prefix a container-membership property is built on.
const RDF_NS: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";

/// Why a failed match does not refute.
///
/// Every variant is produced by the check that names it — there is no arm here waiting for
/// a mechanism to exist. A caller that reads one of these has been told, in data, which
/// hypothesis of which theorem its input broke.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UndecidedReason {
    /// `OWL-RL`: the premise is outside the OWL 2 RL syntax, so Theorem PR1's completeness
    /// half does not apply. Carries the syntactic violations, so the caller learns which
    /// axiom to change rather than that "something" was wrong.
    PremiseOutsideRl(Vec<ProfileViolation>),
    /// `RDF` / `RDFS`: the run withheld this many conclusions drawn about a SURROGATE blank
    /// node, so its closure is smaller than the regime's.
    WithheldSurrogate(u64),
    /// `RDF` / `RDFS`: the question mentions these container-membership properties, whose
    /// axiomatic triples are an infinite schema no finite closure holds.
    AxiomaticSchema(Vec<String>),
    /// `D`: the five `dt-*` rules are the part of datatype entailment a forward chase can
    /// produce, and this crate states no theorem that they are all of it.
    DatatypeValueSpace,
    /// `OWL-RL`: the conclusion states this many NEGATIVE FACTS, which is more chase
    /// re-runs than [`REFUTATION_BUDGET`](super::REFUTATION_BUDGET) allows, so the
    /// refutation lane did not finish.
    ///
    /// Distinct from every other variant here in what it is about: the others say the
    /// PROCEDURE is not complete for this input, and this one says the procedure was not
    /// RUN to completion. Both license exactly the same thing — nothing — and collapsing
    /// this into `NotEntailed` would turn "I stopped" into "there is nothing to find",
    /// which is the overclaim this whole enum exists to prevent.
    RefutationBudget(u64),
    /// `OWL-RL`: the conclusion states schema axioms abbreviating this many Horn
    /// implications, which is more frozen chases than
    /// [`FREEZE_BUDGET`](super::FREEZE_BUDGET) allows, so the freeze-and-chase lane did not
    /// finish.
    ///
    /// A sibling of [`Self::RefutationBudget`] and read the same way: the procedure was not
    /// RUN to completion, which licenses exactly nothing.
    FreezeBudget(u64),
    /// `OWL-RL`: the conclusion states these `rdfs:range` axioms whose containment the
    /// datatype decision procedure does not decide — an `xsd:pattern` facet, an unmodelled
    /// datatype, a range that is not an atomic datatype at all.
    ///
    /// The reason this is a variant rather than a `NotEntailed` is the whole of
    /// [`datarange`](super::datarange)'s discipline: the containment question is
    /// three-valued, and the `bool`-shaped idiom it would otherwise be answered with reads
    /// "cannot say" as "not entailed".
    DataRangeContainment(Vec<String>),
}

impl UndecidedReason {
    /// WHICH mechanism this reason came out of.
    ///
    /// Three of the seven name a lane that ran and stopped early; the other four are
    /// PRECONDITIONS on the rule table's own completeness — a premise outside RL, a withheld
    /// surrogate, an axiomatic schema, an infinite value space — and belong to
    /// [`EntailmentMechanism::StrictTable`], because that is the mechanism whose refutation
    /// they withhold. Written as a total match with no wildcard so an eighth reason has to
    /// say which lane produced it rather than defaulting into the table's.
    #[must_use]
    pub const fn mechanism(&self) -> EntailmentMechanism {
        match self {
            Self::PremiseOutsideRl(_)
            | Self::WithheldSurrogate(_)
            | Self::AxiomaticSchema(_)
            | Self::DatatypeValueSpace => EntailmentMechanism::StrictTable,
            Self::RefutationBudget(_) => EntailmentMechanism::Refutation,
            Self::FreezeBudget(_) => EntailmentMechanism::Freeze,
            Self::DataRangeContainment(_) => EntailmentMechanism::DataRange,
        }
    }

    /// Whether the lane stopped because a BUDGET ran out, rather than because the procedure
    /// is not complete for this input.
    ///
    /// The distinction [`Self::RefutationBudget`] and [`Self::FreezeBudget`] are documented
    /// on: the other five say the PROCEDURE is not complete here, and these two say it was
    /// not RUN to completion. Both license exactly nothing, so they share an outcome — but a
    /// caller deciding whether to retry with a larger question needs to tell them apart, and
    /// [`EntailmentCertificate::is_budget_exhausted`](super::EntailmentCertificate::is_budget_exhausted)
    /// is where that reaches a report.
    #[must_use]
    pub const fn is_budget_exhausted(&self) -> bool {
        matches!(self, Self::RefutationBudget(_) | Self::FreezeBudget(_))
    }
}

impl std::fmt::Display for UndecidedReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PremiseOutsideRl(violations) => write!(
                f,
                "the premise is outside the OWL 2 RL syntax ({} violation{}, first: {}), so OWL 2 \
                 Profiles Theorem PR1 does not apply and the absence of a match refutes nothing",
                violations.len(),
                if violations.len() == 1 { "" } else { "s" },
                violations
                    .first()
                    .map_or("none", |violation| violation.reason()),
            ),
            Self::WithheldSurrogate(count) => write!(
                f,
                "the run withheld {count} conclusion{} about a surrogate blank node, so its \
                 closure is smaller than the regime's",
                if *count == 1 { "" } else { "s" }
            ),
            Self::AxiomaticSchema(terms) => write!(
                f,
                "the question mentions the container-membership propert{} {}, whose axiomatic \
                 triples are an infinite schema",
                if terms.len() == 1 { "y" } else { "ies" },
                terms.join(", ")
            ),
            Self::DatatypeValueSpace => f.write_str(
                "datatype entailment quantifies over infinite value spaces, of which the five \
                 dt-* rules are the part a forward chase can produce",
            ),
            Self::RefutationBudget(needed) => write!(
                f,
                "the conclusion states {needed} negative fact{}, each of which needs its own \
                 re-chase, and the refutation budget of {} runs does not reach that far",
                if *needed == 1 { "" } else { "s" },
                super::REFUTATION_BUDGET,
            ),
            Self::FreezeBudget(needed) => write!(
                f,
                "the conclusion states schema axioms abbreviating {needed} Horn \
                 implication{}, each of which needs its own frozen chase, and the freeze \
                 budget of {} runs does not reach that far",
                if *needed == 1 { "" } else { "s" },
                super::FREEZE_BUDGET,
            ),
            Self::DataRangeContainment(axioms) => write!(
                f,
                "the datatype decision procedure does not decide the containment {} state{}: \
                 {}",
                if axioms.len() == 1 {
                    "the range axiom"
                } else {
                    "the range axioms"
                },
                if axioms.len() == 1 { "s" } else { "" },
                axioms.join(", ")
            ),
        }
    }
}

/// Whether `iri` is a container-membership property (`rdf:_1`, `rdf:_2`, …).
fn is_container_membership(iri: &str) -> bool {
    let Some(rest) = iri.strip_prefix(RDF_NS) else {
        return false;
    };
    let Some(digits) = rest.strip_prefix('_') else {
        return false;
    };
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

/// Every container-membership property the question mentions in a GROUND position, sorted
/// and deduplicated.
///
/// A variable position is not a mention: a `?p` ranges over the closure's own predicates and
/// binds to whichever of them exist, so it neither asks for nor misses an axiomatic triple.
fn axiomatic_terms(pats: &[PatTriple]) -> Vec<String> {
    fn walk(pat: &Pat, found: &mut Vec<String>) {
        match pat {
            Pat::Ground(purrdf_core::TermValue::Iri(iri)) if is_container_membership(iri) => {
                found.push(iri.clone());
            }
            Pat::Triple(inner) => {
                for position in inner.iter() {
                    walk(position, found);
                }
            }
            Pat::Ground(_) | Pat::Var(_) => {}
        }
    }
    let mut found = Vec::new();
    for triple in pats {
        for position in triple {
            walk(position, &mut found);
        }
    }
    found.sort_unstable();
    found.dedup();
    found
}

/// Every reason a failed match under `regime` does not refute, in check order.
///
/// An EMPTY list is the whole claim: the procedure is complete for this premise and this
/// question, so a failed match is a proof of non-entailment. The list is returned rather
/// than folded into a boolean because a caller acting on an undecided answer needs to know
/// which hypothesis to repair, and because two of them can hold at once.
pub(crate) fn limits(
    regime: Regime,
    premise: &RdfDataset,
    report: &ReasoningReport,
    pats: &[PatTriple],
) -> Vec<UndecidedReason> {
    let mut limits = Vec::new();
    match regime {
        // The interpolation lemma: the procedure IS the definition.
        Regime::Simple => {}
        Regime::Rdf | Regime::Rdfs => {
            let withheld = report.withheld_surrogates();
            if withheld > 0 {
                limits.push(UndecidedReason::WithheldSurrogate(withheld));
            }
            let axiomatic = axiomatic_terms(pats);
            if !axiomatic.is_empty() {
                limits.push(UndecidedReason::AxiomaticSchema(axiomatic));
            }
        }
        Regime::OwlRl => {
            let certificate = profile(premise);
            if !certificate.certifies(OwlProfile::Rl) {
                limits.push(UndecidedReason::PremiseOutsideRl(
                    certificate
                        .violations_of(OwlProfile::Rl)
                        .into_iter()
                        .cloned()
                        .collect(),
                ));
            }
        }
        Regime::D => limits.push(UndecidedReason::DatatypeValueSpace),
        // Unreachable: `super::plan` refuses these two before any closure exists, because
        // each is defined by an input this service's signature does not carry. Written out
        // rather than defaulted so an eighth regime has to decide which side it is on.
        Regime::OwlDirect | Regime::Rif => {}
    }
    limits
}

#[cfg(test)]
mod tests {
    use super::{RDF_NS, UndecidedReason, is_container_membership};

    /// The namespace this module tests against is the one the rest of the crate reads.
    #[test]
    fn the_rdf_namespace_does_not_drift() {
        assert!(crate::vocab::RDF_TYPE.starts_with(RDF_NS));
    }

    #[test]
    fn container_membership_properties_are_recognized_by_shape() {
        assert!(is_container_membership(
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#_1"
        ));
        assert!(is_container_membership(
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#_1024"
        ));
        // `rdf:_` with no ordinal is not one, and neither is anything else in the namespace.
        assert!(!is_container_membership(
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#_"
        ));
        assert!(!is_container_membership(
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#type"
        ));
        // …nor a look-alike in another namespace.
        assert!(!is_container_membership("http://example.org/_1"));
    }

    /// Every reason renders something a human can act on — no `Debug` leaking into a log.
    #[test]
    fn every_reason_renders() {
        for reason in [
            UndecidedReason::PremiseOutsideRl(Vec::new()),
            UndecidedReason::WithheldSurrogate(3),
            UndecidedReason::AxiomaticSchema(vec!["rdf:_1".to_owned()]),
            UndecidedReason::DatatypeValueSpace,
            UndecidedReason::RefutationBudget(9_000),
            UndecidedReason::FreezeBudget(9_000),
            UndecidedReason::DataRangeContainment(vec!["<p> rdfs:range <D>".to_owned()]),
        ] {
            assert!(!reason.to_string().is_empty(), "{reason:?}");
        }
    }
}
