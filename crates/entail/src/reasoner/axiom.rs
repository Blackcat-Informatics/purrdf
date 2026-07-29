// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Axiom entailment by refutation, and the fresh symbols it needs.
//!
//! `KB ⊨ α` exactly when `KB ∪ {¬α}` has no model, so every service here negates the axiom
//! it was handed, hands the negation to the tableau as an ASSUMPTION, and reads a closed
//! search as "entailed". The session's `refutes` helper is the
//! single place that conversion happens, so an exhausted search can never be read as a
//! counter-model.
//!
//! # Where the fresh symbols come from — and why they are blank nodes
//!
//! Two of the eight refutations cannot be written over the ontology's own vocabulary.
//!
//! * A subsumption or disjointness question is about an ARBITRARY element, so its witness
//!   is an unnamed node of the completion graph — no term at all. See
//!   [`super::classify`].
//! * A role-inclusion question — `KB ⊨ p ⊑ q` — is about an arbitrary PAIR, and the pair
//!   has to be joined by a `p`-edge, which means both endpoints need term identity. This
//!   is the one case that needs nameable fresh symbols.
//!
//! **PurRDF mints no vocabulary IRIs.** Not for a reasoner's internal scaffolding either:
//! an IRI in a reserved-looking namespace is a term some consumer will eventually resolve,
//! index, or publish, and inventing one would make this crate an ontology. So a refutation
//! symbol is a fresh **blank node** — the generator picks a label prefix no blank node
//! already interned begins with, which makes freshness a checked property of the actual
//! knowledge base rather than a hope about a namespace nobody else uses. A blank node is
//! also semantically the right term: it denotes an unnamed element, which is precisely
//! what "let `x` be an arbitrary individual" means.
//!
//! No caller-supplied namespace parameter is offered, because none is needed: every
//! refutation this module performs is expressible with blank nodes, and a parameter with
//! no use would only invite a default.
//!
//! The fresh symbols never leave: they are interned into the knowledge base's own term
//! space so the tableau can address them, and no service returns one. A caller sees the
//! [`Verdict`], not the scaffolding.

use purrdf_core::TermValue;

use super::certificate::{Session, Verdict};
use crate::interner::Interner;
use crate::owl_dl::concept::{Concept, Role};
use crate::owl_dl::tableau::Assumptions;

/// An axiom whose entailment the reasoner decides by refutation.
///
/// Every variant names its terms by [`TermValue`], so an axiom is written in the caller's
/// vocabulary rather than in the reasoner's interned ids. A term the ontology never
/// mentions is not an error: it is an atomic name no axiom constrains, which is exactly
/// what the Direct Semantics says it is, and the answer for it is a real answer rather
/// than a rejection.
///
/// `owl:Thing` and `owl:Nothing` are read as `⊤` and `⊥` wherever a class is expected, so
/// `SubClassOf { sub: C, sup: owl:Thing }` is entailed for every `C` and
/// `DisjointClasses` against `owl:Nothing` always holds — the answers the semantics gives,
/// not the answers an opaque-atomic-class reading would give.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DlAxiom {
    /// `C ⊑ D` between two named classes.
    SubClassOf {
        /// The subsumed class.
        sub: TermValue,
        /// The subsuming class.
        sup: TermValue,
    },
    /// `C ≡ D` — subsumption in both directions.
    EquivalentClasses {
        /// One class.
        left: TermValue,
        /// The other.
        right: TermValue,
    },
    /// `C ⊓ D ⊑ ⊥` — the two classes share no instance in any model.
    DisjointClasses {
        /// One class.
        left: TermValue,
        /// The other.
        right: TermValue,
    },
    /// `a : C` — a class assertion.
    ClassAssertion {
        /// The individual.
        individual: TermValue,
        /// The class it is asserted to belong to.
        class: TermValue,
    },
    /// `p(a, b)` — an object-property assertion.
    ObjectPropertyAssertion {
        /// The subject individual.
        subject: TermValue,
        /// The property.
        property: TermValue,
        /// The object individual.
        object: TermValue,
    },
    /// `a = b` — the two names denote the same element in every model.
    SameIndividual {
        /// One individual.
        left: TermValue,
        /// The other.
        right: TermValue,
    },
    /// `a ≠ b` — the two names denote different elements in every model.
    ///
    /// OWL 2 makes no unique name assumption, so this is a real question with a real
    /// answer rather than a syntactic comparison: it holds only when something in the
    /// ontology forces the two apart.
    DifferentIndividuals {
        /// One individual.
        left: TermValue,
        /// The other.
        right: TermValue,
    },
    /// `p ⊑ q` — a simple role inclusion.
    ///
    /// The one refutation that needs nameable fresh symbols; see the [module docs](self).
    SubObjectPropertyOf {
        /// The sub-property.
        sub: TermValue,
        /// The super-property.
        sup: TermValue,
    },
}

/// A generator of fresh blank-node symbols for entailment by refutation.
///
/// Freshness is CHECKED, not assumed: [`FreshSymbols::for_interner`] lengthens its prefix
/// until no blank node already interned begins with it, which terminates because labels
/// are finite and each round adds a character. A colliding label would alias a refutation
/// witness with a blank node the data already constrains, and that is an unsoundness
/// rather than an inconvenience — so it is decided against the actual term table.
pub(crate) struct FreshSymbols {
    /// A label prefix no interned blank node begins with.
    prefix: String,
    /// The next ordinal, so successive symbols are distinct from each other too.
    next: u64,
}

/// The label prefix a refutation symbol starts from before collision-avoidance lengthens
/// it. Not an IRI and not a namespace: a blank-node label is local to the graph it occurs
/// in, and none of these ever reaches an answer.
const FRESH_PREFIX: &str = "purrdfDlRefutation";

/// The character appended to the prefix on a collision.
const LENGTHEN: char = 'x';

impl FreshSymbols {
    /// A generator whose labels are guaranteed absent from `interner`.
    pub(crate) fn for_interner(interner: &Interner) -> Self {
        let mut prefix = FRESH_PREFIX.to_owned();
        while interner
            .blank_labels()
            .any(|label| label.starts_with(&prefix))
        {
            prefix.push(LENGTHEN);
        }
        Self { prefix, next: 0 }
    }

    /// Mint the next fresh symbol into `interner`, returning its term id.
    pub(crate) fn mint(&mut self, interner: &mut Interner) -> u32 {
        let label = format!("{}{}", self.prefix, self.next);
        self.next += 1;
        interner.intern(TermValue::blank(&label))
    }
}

/// The three-valued conjunction of two verdicts.
///
/// `False` dominates: one direction of an equivalence demonstrably failing settles the
/// equivalence whatever the other direction did. `Unknown` dominates `True`, because a
/// half-decided conjunction is not decided.
pub(crate) const fn both(a: Verdict, b: Verdict) -> Verdict {
    match (a, b) {
        (Verdict::False, _) | (_, Verdict::False) => Verdict::False,
        (Verdict::Unknown, _) | (_, Verdict::Unknown) => Verdict::Unknown,
        (Verdict::True, Verdict::True) => Verdict::True,
    }
}

/// Whether `KB ⊨ left ⊓ right ⊑ ⊥`, by refuting a witness that is in both.
pub(crate) fn disjoint(session: &mut Session<'_>, left: u32, right: u32) -> Verdict {
    session.refutes(&Assumptions {
        fresh_types: &[left, right],
        ..Assumptions::of_kb()
    })
}

/// Whether `KB ⊨ subject property object`, by refuting `subject : ¬∃property.{object}`.
pub(crate) fn holds_role(session: &mut Session<'_>, subject: u32, negated_reach: u32) -> Verdict {
    session.refutes(&Assumptions {
        types: &[(subject, negated_reach)],
        ..Assumptions::of_kb()
    })
}

/// Whether `KB ⊨ sub ⊑ sup` for two ROLES, over a fresh pair of individuals.
///
/// `x` and `y` are the fresh blank-node symbols; `negated_reach` is the interned
/// `¬∃sup.{y}` the caller built for them.
pub(crate) fn holds_role_inclusion(
    session: &mut Session<'_>,
    x: u32,
    sub: u32,
    y: u32,
    negated_reach: u32,
) -> Verdict {
    session.refutes(&Assumptions {
        types: &[(x, negated_reach)],
        roles: &[(x, sub, y)],
        ..Assumptions::of_kb()
    })
}

/// The concept `∃role.{filler}` — "has a `role`-edge to the individual `filler`".
///
/// The building block of every role refutation: negating it yields `∀role.¬{filler}`,
/// which is the DL way to say "this individual is NOT `role`-related to that one".
pub(crate) fn reaches(role: u32, filler: u32) -> Concept {
    Concept::Some(Role::Named(role), Box::new(Concept::nominal(vec![filler])))
}
