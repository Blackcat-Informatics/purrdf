// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The evidence for a `yes`, and the check that re-decides it.
//!
//! # A verdict a caller has to believe is not evidence
//!
//! This crate's discipline is that every claim comes with something the reader can
//! re-decide rather than re-read: a [`ChaseProof`](crate::ChaseProof) re-derives its head
//! from its premises, a [`Justification`](crate::Justification) re-decides both its
//! sufficiency and its minimality. An entailment verdict owes the same, and
//! [`EntailmentWarrant`] is what it owes.
//!
//! [`verify`] then re-decides it. It runs no reasoner and re-derives nothing —
//! deliberately, because that is a different claim with a different checker. The two
//! decompose:
//!
//! * **"the closure follows from the premise"** is the chase's claim, and
//!   [`explain_conclusion`](crate::explain_conclusion) answers it with a `ChaseProof` whose
//!   `check` re-derives each step against the clause program.
//! * **"the conclusion follows from the closure"** is this claim, and it is finite and
//!   purely combinatorial — a graph homomorphism, or a set of lookups against a refutation's
//!   own closure — checkable in one pass over the conclusion.
//!
//! Folding them into one checker would mean `verify` had to re-run the chase, which costs
//! what the original call cost and gives a caller no independent check at all.
//!
//! # ONE ARM PER MECHANISM, and each arrives with its producer
//!
//! [`EntailmentWarrant`] is an enum with exactly as many arms as this crate has mechanisms
//! for reaching a `yes`, and it has never had one more than that. There are five:
//!
//! * [`Homomorphism`](EntailmentWarrant::Homomorphism) — the chase-and-graph-match procedure
//!   OWL 2 Profiles §4.3 states the RL entailment relation in terms of, minted by
//!   [`super::homomorphism`]. Its evidence is the blank-node MAPPING that made the
//!   conclusion true, together with the closure it maps into.
//! * [`Refutation`](EntailmentWarrant::Refutation) — assert the conclusion's negation,
//!   re-chase, and read the profile's own inconsistency calculus as the proof, minted by
//!   [`super::refutation`]. Its evidence is a [`Refutation`](super::Refutation) per negative
//!   fact — the clash, the minimal premise subset that produces it, and the closure its rule
//!   body instances lie in — beside the ordinary mapping for whatever part of the conclusion
//!   was not negative.
//! * [`Freeze`](EntailmentWarrant::Freeze) — instantiate a schema axiom's universally
//!   quantified body over constants the premise does not mention, re-chase, and read the
//!   derived head as the proof, minted by [`super::freeze`]. Its evidence is a
//!   [`Generalization`](super::Generalization) per axiom — the closure triples establishing
//!   the axiom's MEMBERSHIP half, and one frozen instance per implication carrying the
//!   constants, the body, the head and the closure they lie in.
//! * [`Comprehension`](EntailmentWarrant::Comprehension) — mint the anonymous class
//!   expressions the conclusion names, under the typing side conditions the RDF-Based
//!   comprehension conditions impose, minted by [`super::comprehension`]. Its evidence is the
//!   triples MINTED, the closure triples that LICENSE them, and the witness map that says
//!   which fresh node stood in for which of the conclusion's own.
//! * [`Reflexivity`](EntailmentWarrant::Reflexivity) — read the conclusion's own self-loops
//!   `x p x` off the premise's `owl:ReflexiveProperty` typings, minted by
//!   [`super::reflexivity`]. Its evidence is the self-loops MINTED and the typing that
//!   licenses each.
//!
//! An arm nothing constructs would be a state [`verify`] could not check and no caller could
//! ever be handed, which is the same unrepresentable-contradiction discipline that keeps
//! [`DlCertificate`](crate::DlCertificate) from storing a completeness beside the boundary
//! list that contradicts it. A sixth mechanism brings its own arm and its own producer,
//! together, or it does not arrive.
//!
//! # What `verify` actually checks, and why it needs the premise
//!
//! For the homomorphism arm, three things, all of them necessary and none of them a
//! reasoner:
//!
//! 1. Every triple of the conclusion, with the warrant's mapping applied, is a triple of the
//!    warrant's closure. A mapping that leaves a conclusion blank node unbound produces no
//!    triple to look for and is rejected rather than treated as satisfied.
//! 2. Every default-graph triple of the premise is a triple of the warrant's closure. This
//!    is what BINDS the warrant to the premise it was issued for: a closure is `premise +
//!    conclusions about it`, so a warrant minted for some other premise fails here instead
//!    of being replayed against this one.
//! 3. The conclusion is read from the caller's dataset on the spot, so a warrant cannot be
//!    replayed against a different conclusion either.
//!
//! It is `O(|conclusion| · log|closure|)` for the mapping plus `O(|premise| · log|closure|)`
//! for the binding — one lookup per triple, no search, because the search already happened
//! and its result is what the warrant carries.
//!
//! For the refutation arm it is the same three over the conclusion's RESIDUAL triples, plus
//! [`Refutation::check`](super::Refutation::check) per negative fact — and the conclusion is
//! LOWERED again on the spot, so a warrant cannot be replayed against a conclusion whose
//! negative facts are different ones.
//!
//! For the freeze arm it is again the same three over the residual triples, plus, per
//! generalisation: the axiom's typings are triples of the premise closure; the constants are
//! distinct blank nodes occurring in NEITHER the premise nor the conclusion — the hypothesis
//! of the theorem on constants, re-decided against the caller's own documents rather than
//! taken on the generator's word; the body and head are that shape's implication instantiated
//! over exactly those constants; and each frozen closure holds the premise, the body and
//! either the head or a `false`-concluding rule's satisfied body.
//!
//! For the comprehension arm the conclusion is READ again and the mint RECOMPUTED from the
//! warrant's witness map, then compared: a warrant whose minted list is not what this
//! conclusion licenses under those witnesses fails. Beside that, every witness is re-checked
//! to be a blank node naming nothing in either document (and distinct from the other
//! witnesses), every licence is re-looked-up in the premise closure, and the binding is
//! replayed against `closure ∪ minted`.
//!
//! The reflexivity arm is the same shape with no witnesses to check: the conclusion is read
//! again, the self-loops it licenses under the warrant's closure are recomputed and compared,
//! each licensing typing is re-looked-up, and the binding is replayed.
//!
//! # What `verify` does NOT check
//!
//! It does not re-derive any closure, and it therefore cannot detect a closure that never
//! followed from the premise in the first place. That is the chase's claim and the chase's
//! checker; saying so here rather than implying otherwise is the point of splitting them.

use std::collections::BTreeSet;

use purrdf_core::{RdfDataset, TermValue};

use crate::Regime;
use crate::entails::comprehension::{ComprehensionWarrant, verify_comprehension};
use crate::entails::freeze::{FreezeWarrant, verify_freeze};
use crate::entails::graph::{Triple, default_graph_triples};
use crate::entails::homomorphism::{Binding, Closure, substitute};
use crate::entails::pattern::conclusion_patterns;
use crate::entails::reflexivity::{ReflexivityWarrant, verify_reflexivity};
use crate::entails::refutation::{RefutationWarrant, verify_refutation};

/// The evidence that a premise entails a conclusion, tagged by the mechanism that produced
/// it.
///
/// See the [module docs](self) for why there are exactly five arms and what an arm owes.
#[derive(Debug, Clone)]
pub enum EntailmentWarrant {
    /// The conclusion MAPPED into the closure of the premise.
    Homomorphism(HomomorphismWarrant),
    /// The conclusion's negative facts were REFUTED and the rest mapped.
    Refutation(RefutationWarrant),
    /// The conclusion's schema axioms were FROZEN over fresh constants and chased, and the
    /// rest mapped.
    Freeze(FreezeWarrant),
    /// The conclusion's anonymous class expressions were COMPREHENDED under their typing side
    /// conditions, and the whole conclusion mapped into the extended closure.
    Comprehension(ComprehensionWarrant),
    /// The conclusion's self-loops were read off the premise's REFLEXIVE property typings,
    /// and the whole conclusion mapped into the extended closure.
    Reflexivity(ReflexivityWarrant),
}

impl EntailmentWarrant {
    /// The regime whose closure this warrant is against — the identity of the claim, not
    /// part of its check.
    #[must_use]
    pub const fn regime(&self) -> Regime {
        match self {
            Self::Homomorphism(w) => w.regime(),
            Self::Refutation(w) => w.regime(),
            Self::Freeze(w) => w.regime(),
            Self::Comprehension(w) => w.regime(),
            Self::Reflexivity(w) => w.regime(),
        }
    }

    /// The mapping: what each of the conclusion's existentials was bound to.
    ///
    /// For the refutation and freeze arms this is the mapping of the conclusion's RESIDUAL
    /// triples alone; a negative fact has no mapping, which is why it had to be refuted, and
    /// a schema axiom has none either, which is why it had to be frozen. For the
    /// comprehension arm it is the mapping of the WHOLE conclusion, into the closure extended
    /// by what was minted — nothing there is discharged by having been recognized.
    #[must_use]
    pub const fn binding(&self) -> &Binding {
        match self {
            Self::Homomorphism(w) => w.binding(),
            Self::Refutation(w) => w.binding(),
            Self::Freeze(w) => w.binding(),
            Self::Comprehension(w) => w.binding(),
            Self::Reflexivity(w) => w.binding(),
        }
    }

    /// How many distinct triples the premise closure this warrant is against holds.
    ///
    /// The closure itself is not exposed: a warrant is evidence for one conclusion, not an
    /// alternative way to read a closure out of a call that already returns one.
    #[must_use]
    pub fn closure_size(&self) -> usize {
        match self {
            Self::Homomorphism(w) => w.closure_size(),
            Self::Refutation(w) => w.closure_size(),
            Self::Freeze(w) => w.closure_size(),
            Self::Comprehension(w) => w.closure_size(),
            Self::Reflexivity(w) => w.closure_size(),
        }
    }
}

/// The evidence of a HOMOMORPHISM: the mapping that made the conclusion true, and the
/// closure it maps into.
#[derive(Debug, Clone)]
pub struct HomomorphismWarrant {
    /// The regime the closure was computed under.
    regime: Regime,
    /// What each existential of the conclusion was bound to.
    binding: Binding,
    /// The closure the mapping lands in.
    closure: Closure,
}

impl HomomorphismWarrant {
    /// Mint a warrant. Crate-internal: the only producer is the homomorphism search.
    pub(crate) const fn new(regime: Regime, binding: Binding, closure: Closure) -> Self {
        Self {
            regime,
            binding,
            closure,
        }
    }

    /// The regime whose closure this warrant is against.
    #[must_use]
    pub const fn regime(&self) -> Regime {
        self.regime
    }

    /// The mapping: what each of the conclusion's existentials was bound to.
    #[must_use]
    pub const fn binding(&self) -> &Binding {
        &self.binding
    }

    /// How many distinct triples the closure this warrant is against holds.
    #[must_use]
    pub fn closure_size(&self) -> usize {
        self.closure.len()
    }

    /// The image of `conclusion` under this warrant's mapping — the closure triples that
    /// witness it — or `None` if the mapping leaves a position of the conclusion unbound.
    ///
    /// This is what a caller renders when it wants to SHOW why an entailment holds, and it
    /// is computed rather than stored so it cannot disagree with the mapping.
    #[must_use]
    pub fn witnesses(&self, conclusion: &RdfDataset) -> Option<Vec<[TermValue; 3]>> {
        conclusion_patterns(conclusion)
            .iter()
            .map(|pat| {
                Some([
                    substitute(&pat[0], &self.binding)?,
                    substitute(&pat[1], &self.binding)?,
                    substitute(&pat[2], &self.binding)?,
                ])
            })
            .collect()
    }

    /// Re-decide this warrant against `premise` and `conclusion`.
    fn check(&self, premise: &RdfDataset, conclusion: &RdfDataset) -> bool {
        let Some(image) = self.witnesses(conclusion) else {
            return false;
        };
        if !image.iter().all(|triple| self.closure.contains(triple)) {
            return false;
        }
        // The closure of a premise holds the premise: every lane copies the input through
        // and adds conclusions ABOUT it. A warrant whose closure does not is a warrant about
        // some other premise.
        let held: BTreeSet<Triple> = default_graph_triples(premise).into_iter().collect();
        held.iter().all(|triple| self.closure.contains(triple))
    }
}

/// Re-decide a warrant, without running a reasoner.
///
/// Returns whether `w` really is evidence that `premise` entails `conclusion` under the
/// mechanism that minted it. See the [module docs](self) for what each arm checks, what this
/// does not check, and which checker owns that.
///
/// ```
/// use purrdf_core::RdfDatasetBuilder;
/// use purrdf_entail::{EntailmentOutcome, ImportMap, Regime, entails, verify};
///
/// let mut b = RdfDatasetBuilder::new();
/// let cat = b.intern_iri("http://example.org/Cat");
/// let animal = b.intern_iri("http://example.org/Animal");
/// let tom = b.intern_iri("http://example.org/tom");
/// let sub = b.intern_iri("http://www.w3.org/2000/01/rdf-schema#subClassOf");
/// let ty = b.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
/// b.push_quad(cat, sub, animal, None);
/// b.push_quad(tom, ty, cat, None);
/// let premise = b.freeze().expect("freeze");
///
/// // `tom a Animal` is not asserted; `cax-sco` derives it.
/// let mut c = RdfDatasetBuilder::new();
/// let tom = c.intern_iri("http://example.org/tom");
/// let ty = c.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
/// let animal = c.intern_iri("http://example.org/Animal");
/// c.push_quad(tom, ty, animal, None);
/// let conclusion = c.freeze().expect("freeze");
///
/// let EntailmentOutcome::Entailed(warrant) =
///     entails(&premise, &conclusion, Regime::OwlRl, &ImportMap::new()).expect("a consistent run")
/// else {
///     panic!("cax-sco derives it");
/// };
/// assert!(verify(&warrant, &premise, &conclusion));
/// // …and it says WHICH closure triple witnesses it.
/// let purrdf_entail::EntailmentWarrant::Homomorphism(mapped) = &warrant else {
///     panic!("an ordinary conclusion is reached by matching");
/// };
/// assert_eq!(mapped.witnesses(&conclusion).expect("covered").len(), 1);
/// ```
#[must_use]
pub fn verify(w: &EntailmentWarrant, premise: &RdfDataset, conclusion: &RdfDataset) -> bool {
    match w {
        EntailmentWarrant::Homomorphism(mapped) => mapped.check(premise, conclusion),
        EntailmentWarrant::Refutation(refuted) => verify_refutation(refuted, premise, conclusion),
        EntailmentWarrant::Freeze(frozen) => verify_freeze(frozen, premise, conclusion),
        EntailmentWarrant::Comprehension(comprehended) => {
            verify_comprehension(comprehended, premise, conclusion)
        }
        EntailmentWarrant::Reflexivity(reflexive) => {
            verify_reflexivity(reflexive, premise, conclusion)
        }
    }
}
