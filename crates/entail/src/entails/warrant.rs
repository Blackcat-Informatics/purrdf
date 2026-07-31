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
//! [`EntailmentWarrant`] is what it owes: the blank-node MAPPING that made the conclusion
//! true, together with the closure it maps into.
//!
//! [`verify`] then re-applies the mapping and looks each resulting triple up. It runs no
//! reasoner and re-derives nothing — deliberately, because that is a different claim with a
//! different checker. The two decompose:
//!
//! * **"the closure follows from the premise"** is the chase's claim, and
//!   [`explain_conclusion`](crate::explain_conclusion) answers it with a `ChaseProof` whose
//!   `check` re-derives each step against the clause program.
//! * **"the conclusion follows from the closure"** is this claim, and it is a graph
//!   homomorphism: finite, purely combinatorial, and checkable in one pass over the
//!   conclusion.
//!
//! Folding them into one checker would mean `verify` had to re-run the chase, which costs
//! what the original call cost and gives a caller no independent check at all.
//!
//! # What `verify` actually checks, and why it needs the premise
//!
//! Three things, all of them necessary and none of them a reasoner:
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
//! # What `verify` does NOT check
//!
//! It does not re-derive the closure, and it therefore cannot detect a closure that never
//! followed from the premise in the first place. That is the chase's claim and the chase's
//! checker; saying so here rather than implying otherwise is the point of splitting them.

use std::collections::BTreeSet;

use purrdf_core::{RdfDataset, TermValue};

use crate::Regime;
use crate::entails::graph::{Triple, default_graph_triples};
use crate::entails::homomorphism::{Binding, Closure, substitute};
use crate::entails::pattern::conclusion_patterns;

/// The evidence that a premise entails a conclusion.
///
/// # There is one variant of evidence because there is one mechanism
///
/// A warrant is minted only by the homomorphism mechanism, so the homomorphism's mapping is
/// the only thing it can carry. A second field for a mechanism that does not exist yet
/// would be a state nothing constructs and [`verify`] could not check — the same
/// unrepresentable-contradiction discipline that keeps
/// [`DlCertificate`](crate::DlCertificate) from storing a completeness beside the boundary
/// list that contradicts it. When a second mechanism arrives it brings its own arm and its
/// own producer, together, or it does not arrive.
#[derive(Debug, Clone)]
pub struct EntailmentWarrant {
    /// The regime the closure was computed under — the identity of the claim, not part of
    /// its check.
    regime: Regime,
    /// What each existential of the conclusion was bound to.
    binding: Binding,
    /// The closure the mapping lands in.
    closure: Closure,
}

impl EntailmentWarrant {
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
    ///
    /// The closure itself is not exposed: a warrant is evidence for one conclusion, not an
    /// alternative way to read a closure out of a call that already returns one.
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
}

/// Re-decide a warrant, without running a reasoner.
///
/// Returns whether `w` really is evidence that `premise` entails `conclusion` under the
/// mechanism that minted it: the mapping covers the conclusion, its image lies in the
/// warrant's closure, and that closure extends `premise`. See the [module docs](self) for
/// what this does not check and which checker owns that.
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
/// assert_eq!(warrant.witnesses(&conclusion).expect("covered").len(), 1);
/// ```
#[must_use]
pub fn verify(w: &EntailmentWarrant, premise: &RdfDataset, conclusion: &RdfDataset) -> bool {
    let Some(image) = w.witnesses(conclusion) else {
        return false;
    };
    if !image.iter().all(|triple| w.closure.contains(triple)) {
        return false;
    }
    // The closure of a premise holds the premise: every lane copies the input through and
    // adds conclusions ABOUT it. A warrant whose closure does not is a warrant about some
    // other premise.
    let held: BTreeSet<Triple> = default_graph_triples(premise).into_iter().collect();
    held.iter().all(|triple| w.closure.contains(triple))
}
