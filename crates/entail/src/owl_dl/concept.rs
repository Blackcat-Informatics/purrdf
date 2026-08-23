// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The Description-Logic concept representation for the OWL-Direct tableau.
//!
//! [`Concept`] is a structural syntax tree over interned term ids (class IRIs,
//! property IRIs, individual IRIs). [`Concept::nnf`] rewrites a concept into
//! negation-normal form — negation pushed to the atomic leaves, and every `⊓`/`⊔`
//! flattened, constant-folded, deduped and SORTED — which is what the tableau
//! completion rules assume. The boolean normalization is part of the normal form
//! rather than a pass over it, so a semantic shape reaching the structural interner by
//! any construction path reaches ONE id: `nnf` is the only door, and it is the door
//! [`ConceptTable::intern`] and the negation cache both go through.
//!
//! [`ConceptTable`] structurally interns every
//! (NNF) concept and each of its sub-concepts to a dense `u32` *concept id*, records
//! an id-indexed [`Decomp`]osition so the tableau reads structure by id without ever
//! touching the tree, and precomputes each concept's negation id for O(1) clash
//! detection.
//!
//! Ids are assigned in first-seen (insertion) order, driven by the deterministic
//! parse order, so the whole table is reproducible run to run. The store-once lookup
//! table contains only dense ids and no result is ever derived from hash iteration.

use std::hash::{Hash, Hasher};

use hashbrown::HashTable;

/// A DL role: a named object property, or its inverse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Role {
    /// A named object property, by its interned IRI id.
    Named(u32),
    /// The inverse `r⁻` of the named property with the given interned IRI id.
    Inv(u32),
}

impl Role {
    /// The inverse of this role: `r⁻` of a named property, and the named property of an
    /// inverse one.
    ///
    /// An involution, which is what lets a clause body be RE-ROOTED: reading the edge
    /// `r(x, y)` from `y`'s side is reading `r⁻(y, x)`, and a guard authored at the filler
    /// of `∃r.C` reaches the node the axiom constrains by exactly that step.
    pub(crate) const fn inverse(self) -> Self {
        match self {
            Self::Named(p) => Self::Inv(p),
            Self::Inv(p) => Self::Named(p),
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
    /// `C₁ ⊓ … ⊓ Cₙ` — in NNF, the canonical form [`Concept::and`] builds: at least two
    /// members, sorted and deduped, none of them `⊤`, `⊥`, or a nested `⊓`.
    And(Vec<Self>),
    /// `C₁ ⊔ … ⊔ Cₙ` — in NNF, the canonical form [`Concept::or`] builds: at least two
    /// members, sorted and deduped, none of them `⊤`, `⊥`, or a nested `⊔`.
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
    /// A DATA RANGE, by its id in the knowledge base's
    /// [`DataRangeTable`](crate::owl_dl::data::DataRangeTable) — the concrete-domain
    /// counterpart of a named class.
    ///
    /// It is an ATOMIC leaf, exactly like [`Concept::Named`]: it says the element is a
    /// literal VALUE lying in a subset of the data domain, which is a statement about that
    /// one element rather than a constraint that recurses into a sub-concept. What separates
    /// it from a named class is that its extension is fixed by the datatype map rather than
    /// by the ontology, so the tableau does not guess at it — it asks
    /// [`purrdf_xsd::range`] whether the conjunction of the ranges on a node is empty.
    Data(u32),
    /// `∃r.Self` — the local reflexivity restriction (`owl:hasSelf`).
    ///
    /// It is an ATOMIC leaf rather than a quantifier: `∃r.Self` says the node has an
    /// `r`-edge to ITSELF, which is a property of one node and its edges rather than a
    /// constraint that recurses into a filler concept. That is why [`Concept::neg`] wraps
    /// it in `Not` exactly as it does a named class, and why the two global role axioms
    /// `owl:ReflexiveProperty` (`⊤ ⊑ ∃r.Self`) and `owl:IrreflexiveProperty`
    /// (`⊤ ⊑ ¬∃r.Self`) are ordinary GCIs here rather than a second mechanism.
    SelfRestriction(Role),
}

impl Concept {
    /// A nominal over `ids`, normalized to sorted-deduped order (canonical form).
    pub(crate) fn nominal(mut ids: Vec<u32>) -> Self {
        ids.sort_unstable();
        ids.dedup();
        Self::Nominal(ids)
    }

    /// The CANONICAL `⊔` over `members`: the only way a [`Concept::Or`] is ever built.
    ///
    /// Nested disjunctions are flattened, `⊥` members are dropped (they can satisfy no
    /// disjunct), a `⊤` member collapses the whole disjunction to `⊤`, duplicate members are
    /// removed, and what survives is sorted under [`Concept`]'s derived total order.
    /// `⊔{}` is `⊥` and `⊔{C}` is `C`, so an interned `Or` always has at least two members,
    /// none of which is a constant.
    ///
    /// # Why a boolean simplifier belongs in the NORMAL FORM rather than in the search
    ///
    /// A disjunction is the one concept shape the tableau cannot decide locally: it is a
    /// BRANCH POINT, and every branch costs a step. `⊥` as a disjunct is a branch whose only
    /// outcome is a clash, so leaving it in the tree buys nothing but multiplies the search
    /// by the number of nodes the disjunction is seeded into — and the general concept
    /// inclusion `⊤ ⊑ C` (which is what `rdfs:range` and `rdfs:domain` internalize to) is
    /// exactly `⊔{⊥, C}` seeded into EVERY node. Simplifying here, in front of the interner,
    /// makes that axiom the deterministic `C` it always was.
    ///
    /// Sorting is what makes the simplification structural rather than syntactic: the
    /// interner is a STRUCTURAL table, so one semantic shape reaching one id requires
    /// commutativity to be normalized away. The order is [`Concept`]'s derived `Ord` — a
    /// total order over the tree itself, so it is a pure function of the concept and
    /// introduces no dependence on parse order, hashing, or interning sequence.
    fn or(members: Vec<Self>) -> Self {
        let mut flat: Vec<Self> = Vec::with_capacity(members.len());
        let mut pending = members;
        while let Some(member) = pending.pop() {
            match member {
                // ⊤ is the annihilator of ⊔.
                Self::Top => return Self::Top,
                // ⊥ is the unit of ⊔.
                Self::Bottom => {}
                Self::Or(inner) => pending.extend(inner),
                other => flat.push(other),
            }
        }
        flat.sort_unstable();
        flat.dedup();
        match flat.len() {
            0 => Self::Bottom,
            1 => flat.pop().expect("a one-member disjunction has a member"),
            _ => Self::Or(flat),
        }
    }

    /// The CANONICAL `⊓` over `members` — the exact dual of [`Concept::or`], and the only
    /// way a [`Concept::And`] is ever built.
    ///
    /// Nested conjunctions are flattened, `⊤` members are dropped, a `⊥` member collapses the
    /// whole conjunction to `⊥`, duplicates are removed, and the survivors are sorted under
    /// [`Concept`]'s derived total order. `⊓{}` is `⊤` and `⊓{C}` is `C`.
    ///
    /// The dual must be normalized in lockstep with [`Concept::or`], because
    /// [`Concept::neg`] maps one onto the other: were only the disjunction canonical, the
    /// negation cache [`ConceptTable::finalize`] builds would hand back a DIFFERENT id for a
    /// concept's double negation, and the complementary-pair clash — which is a comparison of
    /// two ids — would stop firing on a pair it should close.
    fn and(members: Vec<Self>) -> Self {
        let mut flat: Vec<Self> = Vec::with_capacity(members.len());
        let mut pending = members;
        while let Some(member) = pending.pop() {
            match member {
                // ⊤ is the unit of ⊓.
                Self::Top => {}
                // ⊥ is the annihilator of ⊓.
                Self::Bottom => return Self::Bottom,
                Self::And(inner) => pending.extend(inner),
                other => flat.push(other),
            }
        }
        flat.sort_unstable();
        flat.dedup();
        match flat.len() {
            0 => Self::Top,
            1 => flat.pop().expect("a one-member conjunction has a member"),
            _ => Self::And(flat),
        }
    }

    /// Rewrite into negation-normal form: every `¬` pushed to an atomic
    /// (`Named` / `Nominal`) leaf, and every `⊓`/`⊔` in the canonical form
    /// [`Concept::and`] / [`Concept::or`] define.
    ///
    /// Idempotent: the canonical form is hereditary (children are normalized before their
    /// parent is built), so re-normalizing a normalized concept is the identity.
    pub(crate) fn nnf(self) -> Self {
        match self {
            Self::Top
            | Self::Bottom
            | Self::Named(_)
            | Self::SelfRestriction(_)
            | Self::Data(_) => self,
            Self::Nominal(ids) => Self::nominal(ids),
            Self::And(cs) => Self::and(cs.into_iter().map(Self::nnf).collect()),
            Self::Or(cs) => Self::or(cs.into_iter().map(Self::nnf).collect()),
            Self::Some(r, c) => Self::Some(r, Box::new(c.nnf())),
            Self::All(r, c) => Self::All(r, Box::new(c.nnf())),
            // `≥0 r.C` is satisfied by every element, so it IS `⊤` — and it has to be spelled
            // `⊤` here, not merely treated as one downstream. `¬(≥0 r.C)` is `⊥`, whose own
            // negation is `⊤`; leaving `≥0 r.C` as a distinct concept would make double
            // negation land on a different id than it started from, and the negation cache
            // the complementary-pair clash reads is built by negating twice.
            Self::Min(0, _, _) => Self::Top,
            Self::Min(n, r, c) => Self::Min(n, r, Box::new(c.nnf())),
            Self::Max(n, r, c) => Self::Max(n, r, Box::new(c.nnf())),
            Self::Not(inner) => Self::neg(*inner),
        }
    }

    /// The NNF of `¬c` (the dual rewriting used by [`Concept::nnf`] under a `Not`).
    fn neg(c: Self) -> Self {
        match c {
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
            Self::Named(_) | Self::SelfRestriction(_) | Self::Data(_) => Self::Not(Box::new(c)),
            Self::Nominal(ids) => Self::Not(Box::new(Self::nominal(ids))),
            Self::Not(inner) => inner.nnf(),
            Self::And(cs) => Self::or(cs.into_iter().map(Self::neg).collect()),
            Self::Or(cs) => Self::and(cs.into_iter().map(Self::neg).collect()),
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
    /// A named class (atomic positive leaf). The class is identified by the
    /// concept id indexing this decomposition, so the tableau reads the leaf
    /// opaquely and needs no term id here.
    Named,
    /// `¬A` for an atomic class `A` (atomic negative leaf).
    NegNamed,
    /// `⊓` over child concept ids — at least two, no child being `⊤`, `⊥`, or another `⊓`
    /// (the canonical [`Concept::and`] form, read out by id).
    And(Vec<u32>),
    /// `⊔` over child concept ids — at least two, no child being `⊤`, `⊥`, or another `⊔`
    /// (the canonical [`Concept::or`] form, read out by id).
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
    /// `∃r.Self` — an atomic positive leaf about the node's own `r`-loop.
    SelfRestriction(Role),
    /// `¬∃r.Self` — an atomic negative leaf about the node's own `r`-loop.
    NegSelfRestriction(Role),
    /// A data range, by its id in the knowledge base's data-range table (atomic positive
    /// leaf). The id is carried here — unlike [`Decomp::Named`]'s class, which the indexing
    /// concept id already identifies — because the tableau must reach the RANGE to decide it.
    Data(u32),
    /// The complement of a data range (atomic negative leaf).
    NegData(u32),
}

/// A structural interning table mapping (NNF) concepts to dense concept ids.
#[derive(Default)]
pub(crate) struct ConceptTable {
    /// Store-once concept index: ids only; equality resolves through `concepts`.
    index: HashTable<u32>,
    /// id → the canonical NNF concept.
    concepts: Vec<Concept>,
    /// id → structural decomposition (child ids resolved).
    decomp: Vec<Decomp>,
    /// id → id of its NNF negation (filled by [`ConceptTable::finalize`]).
    neg: Vec<Option<u32>>,
}

fn hash_concept(concept: &Concept) -> u64 {
    let mut hasher = ahash::AHasher::default();
    concept.hash(&mut hasher);
    hasher.finish()
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
        let hash = hash_concept(c);
        if let Some(&id) = self
            .index
            .find(hash, |&id| self.concepts[id as usize] == *c)
        {
            return id;
        }
        let decomp = match c {
            Concept::Top => Decomp::Top,
            Concept::Bottom => Decomp::Bottom,
            Concept::Named(_) => Decomp::Named,
            Concept::Nominal(ids) => Decomp::Nominal(ids.clone()),
            Concept::SelfRestriction(role) => Decomp::SelfRestriction(*role),
            Concept::Data(range) => Decomp::Data(*range),
            Concept::Not(inner) => match inner.as_ref() {
                Concept::Named(_) => Decomp::NegNamed,
                Concept::Nominal(ids) => Decomp::NegNominal(ids.clone()),
                Concept::SelfRestriction(role) => Decomp::NegSelfRestriction(*role),
                Concept::Data(range) => Decomp::NegData(*range),
                // NNF guarantees `Not` wraps only an atomic leaf.
                other => unreachable!("non-atomic under Not in NNF: {other:?}"),
            },
            Concept::And(cs) => Decomp::And(cs.iter().map(|c| self.intern_nnf(c)).collect()),
            Concept::Or(cs) => Decomp::Or(cs.iter().map(|c| self.intern_nnf(c)).collect()),
            Concept::Some(r, c) => Decomp::Some(*r, self.intern_nnf(c)),
            Concept::All(r, c) => Decomp::All(*r, self.intern_nnf(c)),
            Concept::Min(n, r, c) => Decomp::Min(*n, *r, self.intern_nnf(c)),
            Concept::Max(n, r, c) => Decomp::Max(*n, *r, self.intern_nnf(c)),
        };
        let id = u32::try_from(self.concepts.len()).expect("concept count fits u32");
        self.concepts.push(c.clone());
        self.decomp.push(decomp);
        self.neg.push(None);
        self.index
            .insert_unique(hash, id, |&id| hash_concept(&self.concepts[id as usize]));
        id
    }

    /// The decomposed structure behind a concept id.
    pub(crate) fn decomp(&self, id: u32) -> &Decomp {
        &self.decomp[id as usize]
    }

    /// The canonical NNF concept behind a concept id.
    ///
    /// The one reader is [`crate::owl_dl::absorb`], which DERIVES concepts from interned ones
    /// — the residual `D ⊔ ¬C₂` of a partial absorption, the internalized `¬C ⊔ D` of an
    /// inclusion nothing absorbs. Building those needs the trees the ids stand for, and
    /// re-deriving them from [`Decomp`] would be a second, drifting spelling of the syntax
    /// this table already holds.
    pub(crate) fn concept(&self, id: u32) -> &Concept {
        &self.concepts[id as usize]
    }

    /// The id of the NNF of `¬c` where `c` is the concept with id `id`.
    ///
    /// Requires [`ConceptTable::finalize`] to have populated the negation cache.
    pub(crate) fn negate(&self, id: u32) -> u32 {
        self.neg[id as usize].expect("negation cache populated by finalize()")
    }

    /// The id of the NNF of `¬c`, or `None` when the negation cache has no entry yet.
    ///
    /// The total sibling of [`ConceptTable::negate`], for the one caller that walks the
    /// WHOLE table rather than a concept it just interned: a normalization pass over every
    /// id cannot assume [`ConceptTable::finalize`] has run, and a missing negation there
    /// means one fewer derived axiom rather than a panic.
    pub(crate) fn negation(&self, id: u32) -> Option<u32> {
        self.neg[id as usize]
    }

    /// How many concepts are interned — the exclusive upper bound of the valid ids.
    ///
    /// Read by the normalization pass that turns the whole table into a rule table: the
    /// concept ids are dense and assigned in first-seen order, so `0..len()` enumerates
    /// every concept exactly once, in a sequence that is a function of the parse order
    /// alone.
    pub(crate) fn len(&self) -> usize {
        self.concepts.len()
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
        match self.finalize_until(|| Ok::<(), std::convert::Infallible>(())) {
            Ok(()) => {}
            Err(never) => match never {},
        }
    }

    /// [`Self::finalize`], polling a caller-supplied fallible work-boundary hook before
    /// each concept is normalized.
    pub(crate) fn finalize_until<E>(
        &mut self,
        mut poll: impl FnMut() -> Result<(), E>,
    ) -> Result<(), E> {
        let mut i = 0usize;
        while i < self.concepts.len() {
            poll()?;
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
        Ok(())
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
        // ≥0 r.A = ⊤, and ¬(≥0 r.A) = ⊥ — a matched pair, so double negation returns.
        let zero = Concept::Min(0, r(), Box::new(a.clone()));
        assert_eq!(zero.clone().nnf(), Concept::Top);
        let lhs = Concept::Not(Box::new(zero)).nnf();
        assert_eq!(lhs, Concept::Bottom);
        // ¬(≤3 r.A) = ≥4 r.A
        let lhs = Concept::Not(Box::new(Concept::Max(3, r(), Box::new(a.clone())))).nnf();
        assert_eq!(lhs, Concept::Min(4, r(), Box::new(a)));
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
        assert!(matches!(t.decomp(na), Decomp::NegNamed));
    }

    #[test]
    fn a_disjunction_is_flattened_constant_folded_deduped_and_sorted() {
        let a = Concept::Named(1);
        let b = Concept::Named(2);
        // ⊔{⊥, B, ⊔{A, B}} = ⊔{A, B}, in sorted order.
        let folded = Concept::Or(vec![
            Concept::Bottom,
            b.clone(),
            Concept::Or(vec![a.clone(), b.clone()]),
        ])
        .nnf();
        assert_eq!(folded, Concept::Or(vec![a.clone(), b.clone()]));
        // Commutativity is normalized away: the reversed spelling is the SAME tree.
        assert_eq!(Concept::Or(vec![b, a.clone()]).nnf(), folded);
        // ⊤ annihilates, ⊥ is dropped to nothing, and a singleton is its member.
        assert_eq!(
            Concept::Or(vec![a.clone(), Concept::Top]).nnf(),
            Concept::Top
        );
        assert_eq!(
            Concept::Or(vec![Concept::Bottom, a.clone()]).nnf(),
            Concept::Named(1)
        );
        assert_eq!(Concept::Or(vec![]).nnf(), Concept::Bottom);
        assert_eq!(Concept::Or(vec![a]).nnf(), Concept::Named(1));
    }

    #[test]
    fn a_conjunction_is_flattened_constant_folded_deduped_and_sorted() {
        let a = Concept::Named(1);
        let b = Concept::Named(2);
        // ⊓{⊤, B, ⊓{A, B}} = ⊓{A, B}, in sorted order.
        let folded = Concept::And(vec![
            Concept::Top,
            b.clone(),
            Concept::And(vec![a.clone(), b.clone()]),
        ])
        .nnf();
        assert_eq!(folded, Concept::And(vec![a.clone(), b.clone()]));
        assert_eq!(Concept::And(vec![b, a.clone()]).nnf(), folded);
        assert_eq!(
            Concept::And(vec![a.clone(), Concept::Bottom]).nnf(),
            Concept::Bottom
        );
        assert_eq!(
            Concept::And(vec![Concept::Top, a.clone()]).nnf(),
            Concept::Named(1)
        );
        assert_eq!(Concept::And(vec![]).nnf(), Concept::Top);
        assert_eq!(Concept::And(vec![a]).nnf(), Concept::Named(1));
    }

    /// The shape a general concept inclusion `⊤ ⊑ C` internalizes to — which is what
    /// `rdfs:range` and `rdfs:domain` become — is the DETERMINISTIC `C`, not a disjunction
    /// with a guaranteed-clash branch. `nnf(¬⊤ ⊔ C)` is `⊔{⊥, C}` before folding, and a
    /// disjunction is a branch point seeded into every node.
    #[test]
    fn internalizing_a_top_subsumption_leaves_no_branch_point() {
        let range = Concept::All(r(), Box::new(Concept::Named(7)));
        let meta = Concept::Or(vec![Concept::Not(Box::new(Concept::Top)), range.clone()]).nnf();
        assert_eq!(meta, range, "⊤ ⊑ ∀r.S internalizes to the bare ∀r.S");
        let mut t = ConceptTable::default();
        let id = t.intern(meta);
        assert!(
            matches!(t.decomp(id), Decomp::All(..)),
            "the internalized axiom must decompose to the universal, not a disjunction"
        );
    }

    /// A deterministic pseudo-random concept generator: a fixed-seed SplitMix64 walk over the
    /// variants, so the corpus below is byte-identical on every run and on every platform.
    /// No clock, no thread-local entropy, no floating point.
    struct Gen(u64);

    impl Gen {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        }

        fn below(&mut self, bound: u64) -> u64 {
            self.next() % bound
        }

        /// A concept of at most `depth` nested constructors over a two-name, two-role,
        /// two-individual signature — small enough that duplicate and complementary members
        /// arise often, which is precisely what the canonicalization must survive. [`Gen::leaf`]
        /// is the whole signature: `below(6)` reserves one case each for `⊤` and `⊥`, one for a
        /// nominal over one of two individuals, one for a data range over one of two ranges,
        /// and the remaining two cases for [`Concept::Named`] — so exactly two named classes,
        /// not three. This generator is unrelated to the oracle module's own differential
        /// corpus generator (`owl_dl::oracle::CONCEPT_NAMES`, four class names for a
        /// separate, wider signature) — the two live in different test modules and share no
        /// code.
        fn concept(&mut self, depth: u32) -> Concept {
            let leaves = 6;
            if depth == 0 {
                return self.leaf();
            }
            match self.below(leaves + 8) {
                0..=5 => self.leaf(),
                6 => Concept::Not(Box::new(self.concept(depth - 1))),
                7 => {
                    let n = 1 + self.below(3);
                    Concept::And((0..n).map(|_| self.concept(depth - 1)).collect())
                }
                8 => {
                    let n = 1 + self.below(3);
                    Concept::Or((0..n).map(|_| self.concept(depth - 1)).collect())
                }
                9 => Concept::Some(self.role(), Box::new(self.concept(depth - 1))),
                10 => Concept::All(self.role(), Box::new(self.concept(depth - 1))),
                11 => {
                    let n = u32::try_from(self.below(3)).expect("a count below three fits u32");
                    Concept::Min(n, self.role(), Box::new(self.concept(depth - 1)))
                }
                12 => {
                    let n = u32::try_from(self.below(3)).expect("a count below three fits u32");
                    Concept::Max(n, self.role(), Box::new(self.concept(depth - 1)))
                }
                _ => Concept::SelfRestriction(self.role()),
            }
        }

        fn leaf(&mut self) -> Concept {
            match self.below(6) {
                0 => Concept::Top,
                1 => Concept::Bottom,
                2 => Concept::Nominal(vec![
                    u32::try_from(self.below(2)).expect("an index below two fits u32"),
                ]),
                3 => Concept::Data(u32::try_from(self.below(2)).expect("an index below two fits")),
                other => {
                    Concept::Named(u32::try_from(other).expect("a small index fits u32") - 4 + 10)
                }
            }
        }

        fn role(&mut self) -> Role {
            if self.below(2) == 0 {
                Role::Named(20)
            } else {
                Role::Inv(21)
            }
        }
    }

    /// The generated corpus: fixed seeds, fixed depths, fixed count.
    fn corpus() -> Vec<Concept> {
        let mut out = Vec::new();
        for seed in [1u64, 0x5EED, 0xC0FF_EE00, 0xDEAD_BEEF] {
            let mut g = Gen(seed);
            for _ in 0..250 {
                out.push(g.concept(4));
            }
        }
        out
    }

    /// NEGATION IS AN INVOLUTION on the canonical normal form: `nnf(¬¬C) = nnf(C)`, both as
    /// trees and — the property the tableau actually depends on — as interned IDS.
    ///
    /// The complementary-pair clash is the comparison of a concept id with the id in the
    /// negation cache. If double negation could land on a different id, a node carrying `C`
    /// and `¬C` would not close, and the calculus would report a model where there is none.
    #[test]
    fn negation_is_involutive_over_the_generated_corpus() {
        let mut t = ConceptTable::default();
        for c in corpus() {
            let normal = c.clone().nnf();
            let twice = Concept::neg(Concept::neg(normal.clone())).nnf();
            assert_eq!(
                twice, normal,
                "¬¬C is not C as a tree, for C = {c:?} (normal form {normal:?})"
            );
            let id = t.intern(c.clone());
            t.finalize();
            let neg = t.negate(id);
            assert_eq!(t.negate(neg), id, "¬¬C is not C as an id, for C = {c:?}");
        }
    }

    /// DE MORGAN IS AN IDENTITY ON IDS, not merely on trees: `¬(C₁ ⊔ … ⊔ Cₙ)` and
    /// `¬C₁ ⊓ … ⊓ ¬Cₙ` intern to the SAME id, and dually. This is the property the shared
    /// canonical form of [`Concept::and`] and [`Concept::or`] buys — without the sort, the two
    /// spellings differ by member order and the structural interner would hand out two ids for
    /// one concept.
    #[test]
    fn de_morgan_interns_to_one_id_over_the_generated_corpus() {
        let mut t = ConceptTable::default();
        let members = corpus();
        for group in members.as_chunks::<3>().0 {
            let parts: Vec<Concept> = group.to_vec();
            let negated: Vec<Concept> = parts.iter().cloned().map(Concept::neg).collect();

            let or = Concept::Or(parts.clone());
            let left = t.intern(Concept::Not(Box::new(or)));
            let right = t.intern(Concept::And(negated.clone()));
            assert_eq!(left, right, "¬⊔{parts:?} and ⊓¬{parts:?} interned apart");

            let and = Concept::And(parts.clone());
            let left = t.intern(Concept::Not(Box::new(and)));
            let right = t.intern(Concept::Or(negated));
            assert_eq!(left, right, "¬⊓{parts:?} and ⊔¬{parts:?} interned apart");
        }
    }

    /// The normal form is IDEMPOTENT and the interner is INSENSITIVE to member order: two
    /// spellings that differ only by permutation, duplication or a nested same-operator group
    /// reach one id.
    #[test]
    fn normalization_is_idempotent_and_order_insensitive() {
        let mut t = ConceptTable::default();
        for c in corpus() {
            let once = c.clone().nnf();
            assert_eq!(once.clone().nnf(), once, "nnf is not idempotent on {c:?}");
        }
        let a = Concept::Named(1);
        let b = Concept::Named(2);
        let c = Concept::Named(3);
        let straight = t.intern(Concept::Or(vec![a.clone(), b.clone(), c.clone()]));
        let permuted = t.intern(Concept::Or(vec![c.clone(), a.clone(), b.clone()]));
        let nested = t.intern(Concept::Or(vec![
            Concept::Or(vec![b, c]),
            Concept::Or(vec![a.clone(), a]),
        ]));
        assert_eq!(straight, permuted, "member order must not mint a new id");
        assert_eq!(straight, nested, "nesting must not mint a new id");
    }
}
