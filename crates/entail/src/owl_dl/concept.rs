// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The Description-Logic concept representation for the OWL-Direct tableau.
//!
//! [`Concept`] is a structural syntax tree over interned term ids (class IRIs,
//! property IRIs, individual IRIs). [`Concept::nnf`] rewrites a concept into
//! negation-normal form — negation pushed to the atomic leaves — which is what the
//! tableau completion rules assume. [`ConceptTable`] structurally interns every
//! (NNF) concept and each of its sub-concepts to a dense `u32` *concept id*, records
//! an id-indexed [`Decomp`]osition so the tableau reads structure by id without ever
//! touching the tree, and precomputes each concept's negation id for O(1) clash
//! detection.
//!
//! Ids are assigned in first-seen (insertion) order, driven by the deterministic
//! parse order, so the whole table is reproducible run to run — the lookup map is a
//! [`BTreeMap`] and no result is ever derived from hash iteration.

use std::collections::BTreeMap;

/// A DL role: a named object property, or its inverse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Role {
    /// A named object property, by its interned IRI id.
    Named(u32),
    /// The inverse `r⁻` of the named property with the given interned IRI id.
    Inv(u32),
}

impl Role {
    /// The inverse of this role (`r ↦ r⁻`, `r⁻ ↦ r`).
    pub(crate) fn inverse(self) -> Self {
        match self {
            Self::Named(p) => Self::Inv(p),
            Self::Inv(p) => Self::Named(p),
        }
    }

    /// The underlying named property id, ignoring direction.
    pub(crate) fn property(self) -> u32 {
        match self {
            Self::Named(p) | Self::Inv(p) => p,
        }
    }
}

/// A Description-Logic concept (class expression) over interned term ids.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Concept {
    /// `⊤` (`owl:Thing`).
    Top,
    /// `⊥` (`owl:Nothing`).
    Bottom,
    /// A named class, by its interned IRI id.
    Named(u32),
    /// `¬C`.
    Not(Box<Self>),
    /// `C₁ ⊓ … ⊓ Cₙ`.
    And(Vec<Self>),
    /// `C₁ ⊔ … ⊔ Cₙ`.
    Or(Vec<Self>),
    /// `∃r.C`.
    Some(Role, Box<Self>),
    /// `∀r.C`.
    All(Role, Box<Self>),
    /// `≥n r.C` (qualified; the unqualified form uses [`Concept::Top`]).
    Min(u32, Role, Box<Self>),
    /// `≤n r.C`.
    Max(u32, Role, Box<Self>),
    /// `{a₁,…,aₙ}` — a nominal (`owl:oneOf`), interned individual ids (sorted, deduped).
    Nominal(Vec<u32>),
    /// `∃r.{a}` (`owl:hasValue`) — sugar that [`Concept::nnf`] rewrites to `Some`.
    HasValue(Role, u32),
}

impl Concept {
    /// A nominal over `ids`, normalized to sorted-deduped order (canonical form).
    pub(crate) fn nominal(mut ids: Vec<u32>) -> Self {
        ids.sort_unstable();
        ids.dedup();
        Self::Nominal(ids)
    }

    /// Rewrite into negation-normal form: every `¬` pushed to an atomic
    /// (`Named` / `Nominal`) leaf, `HasValue` desugared to `∃r.{a}`.
    pub(crate) fn nnf(self) -> Self {
        match self {
            Self::Top | Self::Bottom | Self::Named(_) => self,
            Self::Nominal(ids) => Self::nominal(ids),
            Self::And(cs) => Self::And(cs.into_iter().map(Self::nnf).collect()),
            Self::Or(cs) => Self::Or(cs.into_iter().map(Self::nnf).collect()),
            Self::Some(r, c) => Self::Some(r, Box::new(c.nnf())),
            Self::All(r, c) => Self::All(r, Box::new(c.nnf())),
            Self::Min(n, r, c) => Self::Min(n, r, Box::new(c.nnf())),
            Self::Max(n, r, c) => Self::Max(n, r, Box::new(c.nnf())),
            Self::HasValue(r, a) => Self::Some(r, Box::new(Self::Nominal(vec![a]))).nnf(),
            Self::Not(inner) => Self::neg(*inner),
        }
    }

    /// The NNF of `¬c` (the dual rewriting used by [`Concept::nnf`] under a `Not`).
    fn neg(c: Self) -> Self {
        match c {
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
            Self::Named(_) => Self::Not(Box::new(c)),
            Self::Nominal(ids) => Self::Not(Box::new(Self::nominal(ids))),
            Self::Not(inner) => inner.nnf(),
            Self::And(cs) => Self::Or(cs.into_iter().map(Self::neg).collect()),
            Self::Or(cs) => Self::And(cs.into_iter().map(Self::neg).collect()),
            Self::Some(r, c) => Self::All(r, Box::new(Self::neg(*c))),
            Self::All(r, c) => Self::Some(r, Box::new(Self::neg(*c))),
            // ¬(≥n r.C) = ≤(n-1) r.C, and ¬(≥0 r.C) = ⊥.
            Self::Min(n, r, c) => {
                if n == 0 {
                    Self::Bottom
                } else {
                    Self::Max(n - 1, r, Box::new(c.nnf()))
                }
            }
            // ¬(≤n r.C) = ≥(n+1) r.C.
            Self::Max(n, r, c) => Self::Min(n + 1, r, Box::new(c.nnf())),
            Self::HasValue(r, a) => Self::neg(Self::Some(r, Box::new(Self::Nominal(vec![a])))),
        }
    }
}

/// The id-indexed structural decomposition of a (NNF) concept.
///
/// Every child slot is a concept id (another entry in the [`ConceptTable`]) and the
/// `Nominal`/`NegNominal` variants carry interned individual ids. The tableau reads
/// only this form — never the [`Concept`] tree — so completion is a pure integer game.
#[derive(Debug, Clone)]
pub(crate) enum Decomp {
    /// `⊤`.
    Top,
    /// `⊥`.
    Bottom,
    /// A named class id.
    Named(u32),
    /// `¬A` for an atomic class id `A`.
    NegNamed(u32),
    /// `⊓` over child concept ids.
    And(Vec<u32>),
    /// `⊔` over child concept ids.
    Or(Vec<u32>),
    /// `∃r.C` (child concept id).
    Some(Role, u32),
    /// `∀r.C`.
    All(Role, u32),
    /// `≥n r.C`.
    Min(u32, Role, u32),
    /// `≤n r.C`.
    Max(u32, Role, u32),
    /// `{a₁,…,aₙ}` (interned individual ids).
    Nominal(Vec<u32>),
    /// `¬{a₁,…,aₙ}`.
    NegNominal(Vec<u32>),
}

/// A structural interning table mapping (NNF) concepts to dense concept ids.
#[derive(Default)]
pub(crate) struct ConceptTable {
    /// Concept → id (lookup only; never iterated for a result).
    map: BTreeMap<Concept, u32>,
    /// id → the canonical NNF concept.
    concepts: Vec<Concept>,
    /// id → structural decomposition (child ids resolved).
    decomp: Vec<Decomp>,
    /// id → id of its NNF negation (filled by [`ConceptTable::finalize`]).
    neg: Vec<Option<u32>>,
}

impl ConceptTable {
    /// Intern `c` (normalized to NNF), returning its stable concept id.
    ///
    /// Children are interned first so their ids are available in the parent's
    /// [`Decomp`]. Ids are assigned in first-seen order.
    pub(crate) fn intern(&mut self, c: Concept) -> u32 {
        let c = c.nnf();
        self.intern_nnf(&c)
    }

    /// Intern an already-NNF concept (children recursed first).
    fn intern_nnf(&mut self, c: &Concept) -> u32 {
        if let Some(&id) = self.map.get(c) {
            return id;
        }
        let decomp = match c {
            Concept::Top => Decomp::Top,
            Concept::Bottom => Decomp::Bottom,
            Concept::Named(a) => Decomp::Named(*a),
            Concept::Nominal(ids) => Decomp::Nominal(ids.clone()),
            Concept::Not(inner) => match inner.as_ref() {
                Concept::Named(a) => Decomp::NegNamed(*a),
                Concept::Nominal(ids) => Decomp::NegNominal(ids.clone()),
                // NNF guarantees `Not` wraps only an atomic leaf.
                other => unreachable!("non-atomic under Not in NNF: {other:?}"),
            },
            Concept::And(cs) => Decomp::And(cs.iter().map(|c| self.intern_nnf(c)).collect()),
            Concept::Or(cs) => Decomp::Or(cs.iter().map(|c| self.intern_nnf(c)).collect()),
            Concept::Some(r, c) => Decomp::Some(*r, self.intern_nnf(c)),
            Concept::All(r, c) => Decomp::All(*r, self.intern_nnf(c)),
            Concept::Min(n, r, c) => Decomp::Min(*n, *r, self.intern_nnf(c)),
            Concept::Max(n, r, c) => Decomp::Max(*n, *r, self.intern_nnf(c)),
            Concept::HasValue(..) => unreachable!("HasValue is desugared by nnf"),
        };
        let id = u32::try_from(self.concepts.len()).expect("concept count fits u32");
        self.concepts.push(c.clone());
        self.decomp.push(decomp);
        self.neg.push(None);
        self.map.insert(c.clone(), id);
        id
    }

    /// The decomposed structure behind a concept id.
    pub(crate) fn decomp(&self, id: u32) -> &Decomp {
        &self.decomp[id as usize]
    }

    /// The canonical NNF concept behind an id.
    #[cfg(test)]
    pub(crate) fn concept(&self, id: u32) -> &Concept {
        &self.concepts[id as usize]
    }

    /// The id of the NNF of `¬c` where `c` is the concept with id `id`.
    ///
    /// Requires [`ConceptTable::finalize`] to have populated the negation cache.
    pub(crate) fn negate(&self, id: u32) -> u32 {
        self.neg[id as usize].expect("negation cache populated by finalize()")
    }

    /// Convenience concept-id lookups for common atoms.
    pub(crate) fn top(&mut self) -> u32 {
        self.intern(Concept::Top)
    }

    /// The `⊥` concept id.
    pub(crate) fn bottom(&mut self) -> u32 {
        self.intern(Concept::Bottom)
    }

    /// Populate the negation id of every interned concept (a fixpoint, since
    /// negating one concept may intern a new one whose own negation is then filled).
    pub(crate) fn finalize(&mut self) {
        let mut i = 0usize;
        while i < self.concepts.len() {
            if self.neg[i].is_none() {
                let neg = Concept::neg(self.concepts[i].clone());
                let neg_id = self.intern_nnf(&neg.nnf());
                self.neg[i] = Some(neg_id);
                // The negation of the negation is the original (idempotent NNF).
                if self.neg[neg_id as usize].is_none() {
                    self.neg[neg_id as usize] = Some(u32::try_from(i).expect("id fits u32"));
                }
            }
            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r() -> Role {
        Role::Named(0)
    }

    #[test]
    fn nnf_double_negation_and_bottoms() {
        assert_eq!(Concept::Not(Box::new(Concept::Top)).nnf(), Concept::Bottom);
        assert_eq!(Concept::Not(Box::new(Concept::Bottom)).nnf(), Concept::Top);
        let a = Concept::Named(1);
        let nn = Concept::Not(Box::new(Concept::Not(Box::new(a.clone()))));
        assert_eq!(nn.nnf(), a);
    }

    #[test]
    fn nnf_demorgan_and_quantifier_duals() {
        let a = Concept::Named(1);
        let b = Concept::Named(2);
        // ¬(A ⊓ B) = ¬A ⊔ ¬B
        let lhs = Concept::Not(Box::new(Concept::And(vec![a.clone(), b.clone()]))).nnf();
        let rhs = Concept::Or(vec![
            Concept::Not(Box::new(a.clone())),
            Concept::Not(Box::new(b)),
        ]);
        assert_eq!(lhs, rhs);
        // ¬∃r.A = ∀r.¬A
        let lhs = Concept::Not(Box::new(Concept::Some(r(), Box::new(a.clone())))).nnf();
        assert_eq!(
            lhs,
            Concept::All(r(), Box::new(Concept::Not(Box::new(a.clone()))))
        );
        // ¬∀r.A = ∃r.¬A
        let lhs = Concept::Not(Box::new(Concept::All(r(), Box::new(a.clone())))).nnf();
        assert_eq!(lhs, Concept::Some(r(), Box::new(Concept::Not(Box::new(a)))));
    }

    #[test]
    fn nnf_cardinality_negation() {
        let a = Concept::Named(1);
        // ¬(≥2 r.A) = ≤1 r.A
        let lhs = Concept::Not(Box::new(Concept::Min(2, r(), Box::new(a.clone())))).nnf();
        assert_eq!(lhs, Concept::Max(1, r(), Box::new(a.clone())));
        // ¬(≥0 r.A) = ⊥
        let lhs = Concept::Not(Box::new(Concept::Min(0, r(), Box::new(a.clone())))).nnf();
        assert_eq!(lhs, Concept::Bottom);
        // ¬(≤3 r.A) = ≥4 r.A
        let lhs = Concept::Not(Box::new(Concept::Max(3, r(), Box::new(a.clone())))).nnf();
        assert_eq!(lhs, Concept::Min(4, r(), Box::new(a)));
    }

    #[test]
    fn has_value_desugars() {
        let hv = Concept::HasValue(r(), 7).nnf();
        assert_eq!(hv, Concept::Some(r(), Box::new(Concept::Nominal(vec![7]))));
    }

    #[test]
    fn interning_is_stable_and_negation_is_involutive() {
        let mut t = ConceptTable::default();
        let a = t.intern(Concept::Named(1));
        let a2 = t.intern(Concept::Named(1));
        assert_eq!(a, a2, "same concept interns to same id");
        let b = t.intern(Concept::Named(2));
        assert_ne!(a, b);
        t.finalize();
        let na = t.negate(a);
        assert_eq!(t.negate(na), a, "negation is involutive");
        assert!(matches!(t.decomp(na), Decomp::NegNamed(1)));
    }
}
