// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The native OWL-Direct (Description-Logic) reasoner core.
//!
//! Eight layers compose here: [`concept`] is the DL syntax and its structural
//! interner; [`data`] is the CONCRETE domain — the data ranges and literal values a
//! datatype map fixes rather than the ontology; [`parser`] reverse-maps an [`RdfDataset`]
//! into a [`Kb`] (TBox, RBox, ABox, plus anonymous class expressions); [`absorb`] decides,
//! per general concept inclusion, whether it becomes a GUARDED CLAUSE or is internalized into
//! every node's label; [`clause`] compiles the concept table and that decision into
//! DL-clauses; [`graph`] is the completion graph and the two-domain
//! semantics of a node; [`hyper`] is the `SHOIQ(D)` HYPERTABLEAU that decides consistency
//! over those clauses; and [`saturate`] is the consequence-based
//! calculus that derives the WHOLE named-class subsumption relation in one fixpoint,
//! so classification is not a loop over the decision procedure. [`Kb`] ties them together and
//! exposes the internal reasoning seams — [`Kb::is_consistent`], [`Kb::entails_instance`],
//! [`Kb::entails_subclass`], and [`Kb::instances_of`] — which the query-answering layer
//! ([`crate::owl_dl::query`]) drives. Those seams are internal: the public one is
//! [`crate::materialize_dl_reported`], which is where an answer acquires the
//! [`ReasoningReport`](crate::ReasoningReport) naming the constructs this layer could not
//! fully handle.
//!
//! Every derived answer is deterministic: concept ids are assigned in parse order, the
//! clause set is derived in that order, all working sets are `BTreeSet`/`BTreeMap` or
//! insertion-ordered `Vec`s, and the hypertableau branches in a fixed order — nothing is ever
//! read out of a `HashMap`.
//!
//! The reasoning entry points are exercised by the module's own tests and by the
//! query-answering layer ([`crate::owl_dl::query`]), which wires them into the public
//! [`crate::materialize_dl_reported`] seam.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use purrdf_core::RdfDataset;
use purrdf_datalog::StopSignal;

use crate::EntailError;
use crate::interner::Interner;
use crate::owl_dl::absorb::{Encoding, GuardedClause};
use crate::owl_dl::concept::ConceptTable;
use crate::owl_dl::concept::{Concept, Decomp, Role};
use crate::owl_dl::graph::Assumptions;
use crate::report::Construct;

pub(crate) mod absorb;
pub(crate) mod clause;
pub(crate) mod concept;
pub(crate) mod constructs;
pub(crate) mod data;
pub(crate) mod graph;
pub(crate) mod hyper;
/// The differential test of [`hyper`] against a naive model-enumeration oracle AND against
/// the concept-tree [`tableau`] it replaced.
#[cfg(test)]
mod oracle;
pub(crate) mod parser;
pub(crate) mod query;
pub(crate) mod saturate;
/// The concept-tree tableau [`hyper`] replaced, kept as its differential reference.
///
/// Compiled only under `cfg(test)`: it decides no question this crate asks, so shipping it
/// would put a second decision procedure in every artifact for the benefit of a test.
#[cfg(test)]
pub(crate) mod tableau;

/// The concept a named class IRI denotes: `⊤` for `owl:Thing`, `⊥` for `owl:Nothing`, else
/// the atomic named class.
///
/// Shared by every layer that turns a class NAME into something the tableau can reason
/// over — the query-directed materialization and each reasoner service — because reading
/// `owl:Thing` as an opaque atomic class instead of `⊤` would make `C ⊑ owl:Thing`
/// undecidable-looking and `owl:Nothing`'s emptiness a fact nobody stated.
pub(crate) fn class_concept(v: &parser::Vocab, class: u32) -> Concept {
    if class == v.thing {
        Concept::Top
    } else if class == v.nothing {
        Concept::Bottom
    } else {
        Concept::Named(class)
    }
}

/// A Description-Logic knowledge base: the interned TBox/RBox/ABox plus the concept
/// table needed to reason over it.
pub(crate) struct Kb {
    /// The RDF-term interner (class/property/individual IRIs → dense ids).
    pub(crate) interner: Interner,
    /// The structural concept interner.
    pub(crate) table: ConceptTable,
    /// `⊤` concept id.
    pub(crate) top: u32,
    /// `⊥` concept id.
    pub(crate) bottom: u32,
    /// General concept inclusions `sub ⊑ sup`, as concept-id pairs.
    pub(crate) tbox: Vec<(u32, u32)>,
    /// The internalized TBox: meta-concept ids `nnf(¬sub ⊔ sup)`, one per inclusion whose
    /// antecedent [`absorb`] could not guard.
    ///
    /// Seeded into every ABSTRACT node's label, where the `⊔`-clause of each branches. That
    /// is the encoding a general concept inclusion gets when nothing better applies, and
    /// [`Kb::absorbed`] is what "better" means.
    pub(crate) meta: Vec<u32>,
    /// The **absorbed** TBox: one guarded clause `⋀ body → head` per inclusion whose
    /// antecedent is FAITHFUL — see [`absorb`] for the criterion, the per-shape dispositions
    /// and why a guarded clause is not merely an optimization of the internalized form.
    ///
    /// It subsumes the lazy-unfolding index this field replaced: `A ⊑ D` is the degenerate
    /// one-atom guard `A(x) → D(x)`, and `∃r.C ⊑ D`, `A ⊓ B ⊑ D`, `{a} ⊑ D`, `rdfs:domain`
    /// and `rdfs:range` are the cases that used to branch on every node instead.
    pub(crate) absorbed: Vec<GuardedClause>,
    /// Concept id → whether holding it FORCES an at-least head, over the absorbed table's own
    /// closure — see [`absorb::generating`], which computes it.
    ///
    /// Read only to ORDER the alternatives of a case split, never to decide one: a generating
    /// alternative opens a subtree of minted witnesses and a non-generating one does not, so
    /// trying the cheap alternative first is the difference between a search that answers from
    /// one label and a search that builds a graph to find out. Both calculi read it through
    /// [`Kb::order_disjuncts`], so the two branch in the same order.
    pub(crate) generating: Vec<bool>,
    /// `owl:inverseOf` partners (symmetric), property term id → its inverses.
    pub(crate) inverses: BTreeMap<u32, BTreeSet<u32>>,
    /// Role hierarchy: super-property term id → its sub-property term ids.
    pub(crate) role_sub: BTreeMap<u32, BTreeSet<u32>>,
    /// Concept assertions `a : C` — `(individual term id, concept id)`.
    pub(crate) abox_types: Vec<(u32, u32)>,
    /// Role assertions `a r b` — `(subject, property, object)` term ids.
    pub(crate) abox_roles: Vec<(u32, u32, u32)>,
    /// Equality assertions `a owl:sameAs b` — term id pairs.
    pub(crate) same_as: Vec<(u32, u32)>,
    /// Inequality assertions `a owl:differentFrom b` (and every pair of an
    /// `owl:AllDifferent` list) — term id pairs, recorded as `≠` on the completion graph.
    ///
    /// Without them a `≤n r.C` restriction can never be violated: the clash rule counts
    /// PAIRWISE-DISTINCT neighbours, and OWL 2 makes no unique name assumption, so two
    /// names are distinct only when something says so.
    pub(crate) different_from: Vec<(u32, u32)>,
    /// All named individual term ids.
    ///
    /// A LITERAL is deliberately absent even when it is the object of a data-property
    /// assertion: it is a term of the completion graph (so `∀p.C` and `≤n p.C` see it) but
    /// not a realization candidate, because `i rdf:type C` with a literal `i` is a
    /// generalized-RDF triple the dataset IR cannot hold.
    pub(crate) individuals: BTreeSet<u32>,
    /// Property term ids declared `owl:TransitiveProperty`.
    pub(crate) transitive: BTreeSet<u32>,
    /// Property term ids declared `owl:AsymmetricProperty`.
    pub(crate) asymmetric: BTreeSet<u32>,
    /// Disjoint role pairs, held in BOTH orders so a lookup needs no normalization.
    pub(crate) disjoint_roles: BTreeSet<(u32, u32)>,
    /// `owl:hasKey` axioms: the keyed class's concept id and its key property term ids.
    pub(crate) keys: Vec<(u32, Vec<u32>)>,
    /// The CONCRETE domain: every data range the ontology states, decided once.
    ///
    /// A [`Concept::Data`] leaf indexes this table. It is empty for an ontology that states
    /// no data range and no literal, and the tableau's concrete-domain rules are skipped
    /// wholesale in that case.
    pub(crate) data_ranges: data::DataRangeTable,
    /// Literal term id → its VALUE class, for the literals that reach the knowledge base.
    ///
    /// The data domain admits no unique-name freedom: two literals denote one element exactly
    /// when they denote one value. Sharing a class is therefore identity and differing in
    /// class is distinctness — neither is a name comparison, and a literal whose value cannot
    /// be examined is simply absent here rather than guessed either way.
    pub(crate) literal_class: BTreeMap<u32, u32>,
    /// The constructs this knowledge base could not fully handle, in `Construct` order.
    pub(crate) boundaries: BTreeSet<Construct>,
    /// Whether [`Kb::absorbed`]/[`Kb::meta`]/[`Kb::generating`] already reflect [`Kb::tbox`].
    ///
    /// [`Kb::encode_until`] runs three passes, and only the middle one — deriving the
    /// absorbed table from the WHOLE inclusion list — is a pure function of [`Kb::tbox`]
    /// alone; the two negation-cache passes around it also cover concepts a caller interned
    /// since the last call (a named class, a query concept, a refutation witness), so they
    /// stay unconditional. This flag gates only the middle pass: [`Kb::from_dataset`] already
    /// runs it once inside [`parser::build_until`], and every caller downstream that finalizes
    /// again after interning MORE concepts — [`Reasoner::new`](crate::reasoner::Reasoner::new)
    /// chief among them — would otherwise re-derive an identical absorbed table from an
    /// unchanged inclusion list. [`Kb::push_gci`] is the only way [`Kb::tbox`] grows after
    /// construction, and it clears this, so the flag cannot go stale.
    pub(crate) encoded: bool,
    /// How many times [`Kb::encode_until`] actually ran its absorption pass.
    ///
    /// Compiled only under `cfg(test)`: the one thing that reads it is the regression proving
    /// [`Kb::encoded`] does what its doc claims — that `Reasoner::new`'s `finalize()` call
    /// after `Kb::from_dataset` already encoded costs no second absorption pass.
    #[cfg(test)]
    pub(crate) absorb_calls: u32,
    /// The caller's latching stop signal, polled at every search and saturation round.
    ///
    /// Held on the knowledge base rather than passed down through
    /// [`hyper::decide`]/[`saturate::saturate`] and their private drivers because every one
    /// of those already borrows a `&Kb`: putting it here reaches all of them without adding
    /// a parameter to a dozen private functions, and it cannot be forgotten at one call
    /// site and honoured at another.
    ///
    /// `None` — the state [`Kb::from_dataset`] builds — is a run nothing can stop, which is
    /// exactly the behaviour this lane had before the field existed. The stop-aware
    /// constructor installs this before key inference, so a key-bearing ontology cannot
    /// enter an uninterruptible tableau before the public governed path attaches its signal.
    /// It is NOT a budget: see [`purrdf_datalog::stop`] for the distinction, which is what
    /// admits it into a crate whose ceilings are deliberately constants.
    pub(crate) stop: Option<Arc<dyn StopSignal>>,
    /// Whether [`Kb::finalize`] must INTERNALIZE every general concept inclusion instead of
    /// absorbing the ones [`absorb`] can guard.
    ///
    /// Compiled only under `cfg(test)`, because the one thing that reads it is the ENCODING
    /// DIFFERENTIAL in [`crate::owl_dl::oracle`]: every generated knowledge base is built
    /// twice — absorbed and all-meta — and decided by the SAME calculus, which must reach the
    /// same verdict. Absorption is a claim about two encodings of one terminology, and a
    /// claim about two encodings can only be checked by having both. It is deliberately not
    /// a mode a caller can select: there is one encoding this crate decides under, and the
    /// other exists to check it.
    #[cfg(test)]
    pub(crate) internalize_only: bool,
    /// Whether the hypertableau's blocking condition must compare LABELS ALONE, dropping the
    /// predecessor-label and incoming-edge halves of the pairwise signature.
    ///
    /// Compiled only under `cfg(test)`, and for the same reason [`Kb::internalize_only`] is:
    /// the module docs of [`hyper`] state an EMPIRICAL claim — that no knowledge base in this
    /// crate's corpora separates pairwise blocking from label-only blocking — and a claim of
    /// that shape is worth nothing unless the mutation it names can be run. The BLOCKING
    /// DIFFERENTIAL in [`crate::owl_dl::oracle`] sets this on every generated knowledge base
    /// and requires the verdict to be the one the shipped condition reached.
    ///
    /// It is deliberately not a mode a caller can select. Pairwise blocking is the published
    /// calculus's condition and the one this crate decides under; label-only blocking exists
    /// to be compared against, and a verdict difference under it would be a discovery about
    /// the calculus rather than a setting somebody wanted.
    #[cfg(test)]
    pub(crate) label_only_blocking: bool,
}

impl Kb {
    /// An empty knowledge base (with `⊤`/`⊥` pre-interned). Used by the tableau's own
    /// unit tests, which assemble a knowledge base axiom-by-axiom.
    #[cfg(test)]
    pub(crate) fn empty() -> Self {
        let mut table = ConceptTable::default();
        let top = table.top();
        let bottom = table.bottom();
        Self {
            interner: Interner::default(),
            table,
            top,
            bottom,
            tbox: Vec::new(),
            meta: Vec::new(),
            absorbed: Vec::new(),
            generating: Vec::new(),
            inverses: BTreeMap::new(),
            role_sub: BTreeMap::new(),
            abox_types: Vec::new(),
            abox_roles: Vec::new(),
            same_as: Vec::new(),
            different_from: Vec::new(),
            individuals: BTreeSet::new(),
            transitive: BTreeSet::new(),
            asymmetric: BTreeSet::new(),
            disjoint_roles: BTreeSet::new(),
            keys: Vec::new(),
            data_ranges: data::DataRangeTable::default(),
            literal_class: BTreeMap::new(),
            boundaries: BTreeSet::new(),
            encoded: false,
            #[cfg(test)]
            absorb_calls: 0,
            stop: None,
            internalize_only: false,
            label_only_blocking: false,
        }
    }

    /// Whether the caller has asked this run to stop.
    pub(crate) fn stopped(&self) -> bool {
        self.stop.as_ref().is_some_and(|stop| stop.stopped())
    }

    /// Reverse-map an [`RdfDataset`]'s default graph into a knowledge base.
    ///
    /// # Errors
    ///
    /// [`EntailError::Parse`] on a malformed OWL class-expression graph;
    /// [`EntailError::Build`] if applying an `owl:hasKey` axiom exhausts the tableau's step
    /// cap; [`EntailError::Unsatisfiable`] if a key axiom is present over a knowledge base
    /// that is already unsatisfiable (every identification would then be entailed, so the
    /// key says nothing).
    pub(crate) fn from_dataset(ds: &RdfDataset) -> Result<Self, EntailError> {
        Self::from_dataset_until(ds, None)
    }

    /// Reverse-map `ds`, polling `stop` during parsing, and retain it for key inference.
    ///
    /// # Errors
    ///
    /// The same failures as [`Self::from_dataset`], plus [`EntailError::Stopped`] when the
    /// signal fires before or during reverse mapping or key inference.
    pub(crate) fn from_dataset_until(
        ds: &RdfDataset,
        stop: Option<Arc<dyn StopSignal>>,
    ) -> Result<Self, EntailError> {
        if stop.as_deref().is_some_and(StopSignal::stopped) {
            return Err(EntailError::Stopped);
        }
        let mut kb = parser::build_until(ds, stop.as_deref())?;
        kb.stop = stop;
        if kb.stopped() {
            return Err(EntailError::Stopped);
        }
        kb.apply_keys()?;
        Ok(kb)
    }

    /// Apply every `owl:hasKey` axiom, recording the identifications it forces.
    ///
    /// OWL 2's key semantics is DL-SAFE: `C owl:hasKey (p₁ … pₙ)` identifies two
    /// individuals only when both are NAMED, both are instances of `C`, and both have the
    /// same value for every `pᵢ`. The named-individual restriction is what keeps keys
    /// decidable, and it is the reason this is a pass over [`Kb::individuals`] rather than
    /// a completion rule the tableau could apply to an anonymous witness.
    ///
    /// "Instance of `C`" is decided by [`Kb::entails_instance`] rather than read off the
    /// asserted type triples, so a class membership the TBox entails — `a : Male ⊓ Parent`
    /// with `Father ≡ Male ⊓ Parent` — triggers the key exactly as an asserted
    /// `a rdf:type Father` would. The pass runs only when the ontology actually states a
    /// key, so an ontology without one pays nothing for it.
    ///
    /// The key values compared are the ASSERTED role assertions, which is what a key is
    /// about: `p₁` values that only an existential restriction entails are anonymous, and
    /// an anonymous value is outside the DL-safe fragment by the same argument that
    /// excludes an anonymous individual.
    ///
    /// # Errors
    ///
    /// Propagates [`Kb::entails_instance`]'s failures.
    fn apply_keys(&mut self) -> Result<(), EntailError> {
        if self.keys.is_empty() {
            return Ok(());
        }
        self.finalize();
        let named: Vec<u32> = self.individuals.iter().copied().collect();
        let mut forced: Vec<(u32, u32)> = Vec::new();
        for (class, properties) in &self.keys {
            // The individuals this key ranges over, in ascending id order.
            let mut members: Vec<u32> = Vec::new();
            for &individual in &named {
                if self.entails_instance(individual, *class)? {
                    members.push(individual);
                }
            }
            for (index, &left) in members.iter().enumerate() {
                for &right in &members[index + 1..] {
                    if properties
                        .iter()
                        .all(|&property| self.agrees_on(left, right, property))
                    {
                        forced.push((left, right));
                    }
                }
            }
        }
        self.same_as.extend(forced);
        Ok(())
    }

    /// Whether `left` and `right` share at least one asserted `property` value, and neither
    /// is without one.
    ///
    /// A key property with no value on one of the two individuals does not identify them —
    /// OWL 2 requires the key values to EXIST and to coincide — so an absent value answers
    /// `false` rather than vacuously `true`.
    fn agrees_on(&self, left: u32, right: u32, property: u32) -> bool {
        let values = |individual: u32| -> BTreeSet<u32> {
            self.abox_roles
                .iter()
                .filter(|&&(subject, predicate, _)| subject == individual && predicate == property)
                .map(|&(_, _, object)| object)
                .collect()
        };
        let left = values(left);
        !left.is_empty() && values(right).intersection(&left).next().is_some()
    }

    /// The constructs this knowledge base could not fully handle.
    pub(crate) fn boundaries(&self) -> &BTreeSet<Construct> {
        &self.boundaries
    }

    /// Record a general concept inclusion `sub ⊑ sup`. Used by the tableau unit tests and by
    /// the oracle's generators (the RDF build path records inclusions inline in [`parser`]).
    ///
    /// Recording is all it does: which ENCODING an inclusion gets — a guarded clause in
    /// [`Kb::absorbed`] or a meta-concept in [`Kb::meta`] — is decided once, over the whole
    /// TBox, by [`Kb::finalize`]. It has to be: absorption SPLITS `C ⊑ D ⊓ E` and
    /// `C ⊔ D ⊑ E`, and a streaming per-axiom decision cannot split what it has not yet seen
    /// the rest of.
    ///
    /// Clears [`Kb::encoded`]: this is the one way [`Kb::tbox`] grows after construction, and
    /// [`Kb::encode_until`]'s absorption pass must see the grown list on the next call rather
    /// than skip itself over an inclusion it has not yet absorbed.
    #[cfg(test)]
    pub(crate) fn push_gci(&mut self, sub: Concept, sup: Concept) {
        let sub_id = self.table.intern(sub);
        let sup_id = self.table.intern(sup);
        self.tbox.push((sub_id, sup_id));
        self.encoded = false;
    }

    /// Intern a query concept and refresh the negation cache so it can be negated by
    /// [`Kb::entails_instance`] / [`Kb::entails_subclass`]. Used by the module's unit
    /// tests; the query layer interns in bulk and calls [`Kb::finalize`] once.
    #[cfg(test)]
    pub(crate) fn intern_query(&mut self, c: Concept) -> u32 {
        let id = self.table.intern(c);
        self.table.finalize();
        id
    }

    /// Finalize the knowledge base for DECIDING: clausify the TBox, and disclose the
    /// completeness limit the finished terminology forces. Call once after all axioms and
    /// assertions are in place.
    pub(crate) fn finalize(&mut self) {
        match self.encode_until(|| Ok::<(), std::convert::Infallible>(())) {
            Ok(()) => {}
            Err(never) => match never {},
        }
        if self.counts_over_an_inverse() {
            self.boundaries.insert(Construct::CountingOnInverse);
        }
    }

    /// Clausify the TBox, polling a caller-supplied fallible work boundary.
    ///
    /// The ENCODING half of [`Self::finalize`], and the half the reverse mapping needs: a
    /// parsed knowledge base is not yet a question, while
    /// [`Construct::CountingOnInverse`] is a statement about the DECISION core's completeness
    /// rather than about the ontology's syntax. So the disclosure stays where a decision is
    /// prepared — the query layer's finalize, and the key-inference pass — which is exactly
    /// where it was made before the TBox needed a pass of its own.
    ///
    /// Three passes, in this order and for this reason: the negation cache first, because
    /// PARTIAL absorption negates the conjuncts it cannot guard; then [`absorb`], which
    /// derives [`Kb::absorbed`] and [`Kb::meta`] from [`Kb::tbox`] and interns whatever
    /// residual and internalized concepts those dispositions need; then the negation cache
    /// again, over exactly those new concepts.
    ///
    /// It recomputes both encodings from the authoritative inclusion list rather than
    /// appending to them, so calling it twice — which the key-inference pass and the query
    /// layer both do, each after interning more concepts — answers what calling it once
    /// would. An encoding that accumulated instead would give the second call a doubled
    /// clause table and a search that derived every absorbed head twice.
    ///
    /// # Why the middle pass alone is skippable, and the other two are not
    ///
    /// [`absorb::absorb`] is a pure function of [`Kb::tbox`] (and the encoding choice, fixed
    /// for a `Kb`'s lifetime outside `cfg(test)`), so re-running it over an UNCHANGED
    /// inclusion list re-derives the identical [`Kb::absorbed`]/[`Kb::meta`] at real cost and
    /// no benefit — [`parser::build_until`] already ran it once, and
    /// [`Reasoner::new`](crate::reasoner::Reasoner::new) used to pay for it again on every
    /// construction. [`Kb::encoded`] gates exactly that pass. The two [`ConceptTable::finalize_until`]
    /// calls around it stay unconditional: a caller between two `encode_until`s regularly
    /// interns MORE concepts — a named class, a query concept, a refutation witness — that
    /// need negation-cache entries of their own, and [`ConceptTable::finalize_until`] is
    /// already cheap for a concept it has already covered (a single `is_none` check per id).
    pub(crate) fn encode_until<E>(
        &mut self,
        mut poll: impl FnMut() -> Result<(), E>,
    ) -> Result<(), E> {
        self.table.finalize_until(&mut poll)?;
        if !self.encoded {
            let encoding = self.encoding();
            let absorption = absorb::absorb(
                &mut self.table,
                &self.tbox,
                self.top,
                self.bottom,
                encoding,
                &mut poll,
            )?;
            self.absorbed = absorption.clauses;
            self.meta = absorption.meta;
            self.encoded = true;
            #[cfg(test)]
            {
                self.absorb_calls += 1;
            }
        }
        self.table.finalize_until(&mut poll)?;
        // LAST, over the finished table: the closure reads the absorbed clauses just derived,
        // and the negation pass above interned the residual concepts a partial absorption or a
        // `≤n` restriction's decided filler puts into a case split. Cheap even when the middle
        // pass was skipped — [`absorb::generating`] walks [`Kb::absorbed`], not [`Kb::tbox`] —
        // so it stays unconditional rather than adding a second flag to keep in step with the
        // first.
        self.generating = absorb::generating(&self.table, &self.absorbed);
        Ok(())
    }

    /// Whether holding `concept` forces witnesses to be minted — see [`Kb::generating`].
    ///
    /// A concept interned AFTER the last [`Self::encode_until`] is answered `false` rather than
    /// panicking on a short table: the closure orders a case split and does not decide one, so
    /// a missing entry costs the ordering and never the verdict. Unreachable via every named
    /// path this crate ships, though: every caller that orders a case split has already
    /// finalized the concept it is ordering, since a disjunct reaching [`Kb::order_disjuncts`]
    /// was interned by absorption or by the reverse mapping, both of which run before the
    /// search that reads this.
    pub(crate) fn generates(&self, concept: u32) -> bool {
        self.generating
            .get(concept as usize)
            .copied()
            .unwrap_or(false)
    }

    /// `members` in the order a case split over them should be TRIED: the alternatives that
    /// mint no witnesses first, everything else after, and the members' own incoming order
    /// preserved inside each of the two ranks.
    ///
    /// # Why this is not the canonical member order, and must not become it
    ///
    /// [`Concept::or`](concept::Concept) sorts a disjunction's members under the concept tree's
    /// own total order, and that sort is about IDENTITY: it is what makes two spellings of one
    /// disjunction reach one interned id, and it is therefore a pure function of the syntax
    /// that must not depend on anything the terminology decides. This order is about SEARCH:
    /// it depends on the absorbed table, which depends on the whole TBox, and it changes when
    /// an unrelated axiom is added. The two are kept apart deliberately — an interner whose
    /// keys moved with the terminology would mint a fresh id for an unchanged concept, and a
    /// search that branched in interning order would try the expensive alternative first
    /// whenever the cheap one happened to sort later.
    ///
    /// # Why the tie-break is STABILITY and not the concept id
    ///
    /// Inside one rank this changes nothing at all, and that is the point. The members arrive
    /// in an order that is already a total, deterministic function of the concept — the
    /// canonical `⊔` order for a disjunction, `[filler, ¬filler]` for a `≤n` restriction's
    /// decided neighbour — and re-sorting them by concept id would replace it with a
    /// DIFFERENT total order for no reason connected to cost. That is not a neutral choice,
    /// and it is MEASURED rather than assumed: adding the concept id as a tie-break inside a
    /// rank — permuting only members that rank identically — costs the generated corpora of
    /// the DL oracle suite 1,900 rounds → 2,558 on the `wide` family and 1,179 → 1,524 on the
    /// `deep` one. Two `wide` knowledge bases that decide in 14 and 56 rounds stop deciding
    /// inside that suite's cap at all; with the cap lifted, the worst `deep` case goes from
    /// 175 rounds to 438. So this function does one thing and only that thing: it moves the
    /// generating alternatives last, and leaves every order it was not asked about exactly as
    /// it found it.
    pub(crate) fn order_disjuncts(&self, members: &[u32]) -> Vec<u32> {
        let mut out = members.to_vec();
        out.sort_by_key(|&member| u8::from(self.generates(member)));
        out
    }

    /// The encoding [`Self::encode_until`] clausifies the TBox under.
    #[cfg(test)]
    const fn encoding(&self) -> Encoding {
        if self.internalize_only {
            Encoding::Internalizing
        } else {
            Encoding::Absorbing
        }
    }

    /// The encoding [`Self::encode_until`] clausifies the TBox under — the only one a
    /// shipped build has.
    #[cfg(not(test))]
    const fn encoding(&self) -> Encoding {
        Encoding::Absorbing
    }

    /// Whether the hypertableau's blocking signature is LABELS ALONE — see
    /// [`Kb::label_only_blocking`], which the differential corpus sets.
    #[cfg(test)]
    pub(crate) const fn labels_alone_block(&self) -> bool {
        self.label_only_blocking
    }

    /// Whether the hypertableau's blocking signature is LABELS ALONE. A shipped build blocks
    /// PAIRWISE — labels, predecessor labels and the incoming edge — which is the published
    /// calculus's condition and the only one outside a test.
    #[cfg(not(test))]
    pub(crate) const fn labels_alone_block(&self) -> bool {
        false
    }

    /// Whether the ontology counts successors of a role that is SOMETHING's inverse — the
    /// NN/NI corner neither decision core is complete for.
    ///
    /// Keyed on LOGICAL CONTENT, not on spelling. `≤n r⁻.C` written directly is the obvious
    /// shape, but `q owl:inverseOf p` with `≤n q.C` denotes exactly the same thing, and an
    /// earlier revision of this check saw only the first: two logically equivalent knowledge
    /// bases disclosed the limit differently, which makes the disclosure a fact about syntax
    /// rather than about the answer. Both spellings are the corner, so both raise it.
    ///
    /// `owl:InverseFunctionalProperty` is the everyday case in the first spelling — it IS
    /// `⊤ ⊑ ≤1 p⁻.⊤` — and the second is how the same restriction reads when a caller names
    /// the inverse. A counted role with no inverse partner is outside the corner and raises
    /// nothing.
    fn counts_over_an_inverse(&self) -> bool {
        (0..self.table.len() as u32).any(|id| match self.table.decomp(id) {
            // Counted directly over an inverse role.
            Decomp::Max(_, Role::Inv(_), _) => true,
            // Counted over a NAMED role that some `owl:inverseOf` axiom makes an inverse.
            // The map is symmetric, so one membership test settles either direction.
            Decomp::Max(_, Role::Named(p), _) => self.inverses.contains_key(p),
            _ => false,
        })
    }

    /// Whether the knowledge base (TBox + ABox) is consistent.
    ///
    /// # Errors
    ///
    /// [`EntailError::Build`] if the hypertableau exceeds its step cap.
    pub(crate) fn is_consistent(&self) -> Result<bool, EntailError> {
        hyper::consistent(self, &Assumptions::of_kb())
    }

    /// The same question decided by the CONCEPT-TREE tableau — the differential reference.
    ///
    /// Kept reachable for exactly the reason
    /// [`Subsumptions::decide_by_tableau`](crate::reasoner::classify::Subsumptions::decide_by_tableau)
    /// is: a differential test needs both sides to exist, and a deleted reference
    /// implementation is a test that can only assert the new code agrees with itself. Every
    /// knowledge base this module's tests build is decided by both, and a divergence is a bug
    /// in one of them rather than a difference to be recorded.
    ///
    /// # Errors
    ///
    /// [`EntailError::Build`] if the tableau exceeds its step cap.
    #[cfg(test)]
    pub(crate) fn is_consistent_by_concept_tree(&self) -> Result<bool, EntailError> {
        tableau::consistent(self, &Assumptions::of_kb())
    }

    /// Whether `individual : concept_id` is entailed — i.e. the knowledge base with
    /// `individual : ¬concept` added is inconsistent.
    ///
    /// # Errors
    ///
    /// [`EntailError::Unsatisfiable`] if the base knowledge base is already
    /// unsatisfiable; [`EntailError::Build`] on step-cap exhaustion.
    pub(crate) fn entails_instance(
        &self,
        individual: u32,
        concept_id: u32,
    ) -> Result<bool, EntailError> {
        if !self.is_consistent()? {
            return Err(EntailError::Unsatisfiable);
        }
        let neg = self.table.negate(concept_id);
        let consistent = hyper::consistent(
            self,
            &Assumptions {
                types: &[(individual, neg)],
                ..Assumptions::of_kb()
            },
        )?;
        Ok(!consistent)
    }

    /// Whether `sub_id ⊑ sup_id` is entailed — i.e. a fresh witness in `sub ⊓ ¬sup` has no
    /// model. Yields `⊥ ⊑ X` and reflexive `X ⊑ X`.
    ///
    /// # The ABox is loaded, and that is not an optimization to undo
    ///
    /// Subsumption is decided against the WHOLE knowledge base, assertions included,
    /// because with nominals an assertion changes the class hierarchy: `Only ≡ {alice}`
    /// together with `alice : Female` entails `Only ⊑ Female`, and a TBox-only test — which
    /// this used to be — cannot see it. Reasoning over the TBox alone is cheaper and
    /// answers a different question, and the query-directed materialization that consumes
    /// this is claiming to hold every entailed ground atom over the query's vocabulary, so
    /// the different question is the wrong one.
    ///
    /// # Errors
    ///
    /// [`EntailError::Unsatisfiable`] if the base knowledge base is already
    /// unsatisfiable; [`EntailError::Build`] on step-cap exhaustion.
    pub(crate) fn entails_subclass(&self, sub_id: u32, sup_id: u32) -> Result<bool, EntailError> {
        if !self.is_consistent()? {
            return Err(EntailError::Unsatisfiable);
        }
        let neg_sup = self.table.negate(sup_id);
        let consistent = hyper::consistent(
            self,
            &Assumptions {
                fresh_types: &[sub_id, neg_sup],
                ..Assumptions::of_kb()
            },
        )?;
        Ok(!consistent)
    }

    /// Whether `sub_id ⊑ sup_id` is entailed, decided by the CONCEPT-TREE tableau.
    ///
    /// The subsumption sibling of [`Kb::is_consistent_by_concept_tree`], and the reference the
    /// differential over the reverse-mapped fixture corpus reads: those fixtures reach the
    /// decision core through the RDF parser rather than through a hand-assembled knowledge
    /// base, so they are where a clause derived from a REVERSE-MAPPED class expression is
    /// compared against the calculus that reads the expression's structure directly.
    ///
    /// # Errors
    ///
    /// [`EntailError::Unsatisfiable`] if the base knowledge base is already unsatisfiable;
    /// [`EntailError::Build`] on step-cap exhaustion.
    #[cfg(test)]
    pub(crate) fn entails_subclass_by_concept_tree(
        &self,
        sub_id: u32,
        sup_id: u32,
    ) -> Result<bool, EntailError> {
        if !self.is_consistent_by_concept_tree()? {
            return Err(EntailError::Unsatisfiable);
        }
        let neg_sup = self.table.negate(sup_id);
        let consistent = tableau::consistent(
            self,
            &Assumptions {
                fresh_types: &[sub_id, neg_sup],
                ..Assumptions::of_kb()
            },
        )?;
        Ok(!consistent)
    }

    /// Every named individual entailed to be an instance of `concept_id`, ascending.
    ///
    /// # Errors
    ///
    /// Propagates [`Kb::entails_instance`] failures.
    pub(crate) fn instances_of(&self, concept_id: u32) -> Result<Vec<u32>, EntailError> {
        let mut out = Vec::new();
        for &ind in &self.individuals {
            if self.entails_instance(ind, concept_id)? {
                out.push(ind);
            }
        }
        Ok(out)
    }

    /// The interned term id of an IRI, if it occurs in the knowledge base.
    #[cfg(test)]
    pub(crate) fn iri_id(&self, iri: &str) -> Option<u32> {
        self.interner.id_of_iri(iri)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::owl_dl::concept::{Concept, Role};
    use purrdf_core::{RdfDatasetBuilder, TermId};

    const NS: &str = "http://example.org/test#";

    fn iri(b: &mut RdfDatasetBuilder, local: &str) -> TermId {
        b.intern_iri(&format!("{NS}{local}"))
    }

    fn vocab(b: &mut RdfDatasetBuilder, full: &str) -> TermId {
        b.intern_iri(full)
    }

    const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    const OWL_CLASS: &str = "http://www.w3.org/2002/07/owl#Class";
    const OWL_OBJECTPROPERTY: &str = "http://www.w3.org/2002/07/owl#ObjectProperty";
    const OWL_FUNCTIONALPROPERTY: &str = "http://www.w3.org/2002/07/owl#FunctionalProperty";
    const OWL_HASKEY: &str = "http://www.w3.org/2002/07/owl#hasKey";
    const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
    const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
    const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";

    #[derive(Debug)]
    struct StopAtPoll {
        polls: std::sync::atomic::AtomicU64,
        fire_at: u64,
    }

    impl StopAtPoll {
        fn new(fire_at: u64) -> Self {
            Self {
                polls: std::sync::atomic::AtomicU64::new(0),
                fire_at,
            }
        }

        fn polls(&self) -> u64 {
            self.polls.load(std::sync::atomic::Ordering::Relaxed)
        }
    }

    impl StopSignal for StopAtPoll {
        fn stopped(&self) -> bool {
            self.polls
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                >= self.fire_at
        }
    }

    /// Build the `simple.ttl` fixture as a dataset (default graph).
    fn simple_dataset() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let ty = vocab(&mut b, RDF_TYPE);
        let class = vocab(&mut b, OWL_CLASS);
        let objp = vocab(&mut b, OWL_OBJECTPROPERTY);
        let funcp = vocab(&mut b, OWL_FUNCTIONALPROPERTY);
        // Class / property declarations.
        for c in ["A", "B", "C"] {
            let s = iri(&mut b, c);
            b.push_quad(s, ty, class, None);
        }
        let p = iri(&mut b, "p");
        b.push_quad(p, ty, objp, None);
        b.push_quad(p, ty, funcp, None);
        // Individuals.
        let a = iri(&mut b, "a");
        let bb = iri(&mut b, "b");
        let cc = iri(&mut b, "c");
        let dd = iri(&mut b, "d");
        let acls = iri(&mut b, "A");
        let bcls = iri(&mut b, "B");
        let ccls = iri(&mut b, "C");
        b.push_quad(a, ty, acls, None);
        b.push_quad(a, ty, bcls, None);
        b.push_quad(a, p, bb, None);
        b.push_quad(bb, ty, bcls, None);
        b.push_quad(bb, p, cc, None);
        b.push_quad(cc, ty, ccls, None);
        b.push_quad(cc, p, dd, None);
        b.push_quad(dd, ty, acls, None);
        b.push_quad(dd, ty, bcls, None);
        b.push_quad(dd, ty, ccls, None);
        b.freeze().expect("freeze")
    }

    #[test]
    fn simple_instance_retrieval() {
        let ds = simple_dataset();
        let mut kb = Kb::from_dataset(&ds).expect("parse");
        let a = kb.iri_id(&format!("{NS}a")).unwrap();
        let bb = kb.iri_id(&format!("{NS}b")).unwrap();
        let cc = kb.iri_id(&format!("{NS}c")).unwrap();
        let dd = kb.iri_id(&format!("{NS}d")).unwrap();
        let acls = kb.iri_id(&format!("{NS}A")).unwrap();
        let bcls = kb.iri_id(&format!("{NS}B")).unwrap();
        let ccls = kb.iri_id(&format!("{NS}C")).unwrap();
        let p = kb.iri_id(&format!("{NS}p")).unwrap();

        // A ⊓ B → {a, d}.
        let and_ab = kb.intern_query(Concept::And(vec![
            Concept::Named(acls),
            Concept::Named(bcls),
        ]));
        assert_eq!(kb.instances_of(and_ab).unwrap(), vec![a.min(dd), a.max(dd)]);

        // ∃p.B includes a (a p b, b : B).
        let some_pb = kb.intern_query(Concept::Some(
            Role::Named(p),
            Box::new(Concept::Named(bcls)),
        ));
        assert!(kb.entails_instance(a, some_pb).unwrap(), "a ∈ ∃p.B");

        // B ⊔ C → {a, b, c, d} (everyone).
        let or_bc = kb.intern_query(Concept::Or(vec![
            Concept::Named(bcls),
            Concept::Named(ccls),
        ]));
        let mut expected = [a, bb, cc, dd];
        expected.sort_unstable();
        assert_eq!(kb.instances_of(or_bc).unwrap(), expected.to_vec());
    }

    #[test]
    fn governed_construction_refuses_an_already_cancelled_request_before_parsing() {
        let stop = Arc::new(StopAtPoll::new(0));
        let signal: Arc<dyn StopSignal> = stop.clone();
        let Err(error) = Kb::from_dataset_until(&simple_dataset(), Some(signal)) else {
            panic!("an already-cancelled construction must refuse");
        };
        assert!(matches!(error, EntailError::Stopped));
        assert_eq!(
            stop.polls(),
            1,
            "the parser must not start after cancellation"
        );
    }

    #[test]
    fn governed_construction_stops_during_reverse_mapping() {
        // Poll 0 is the API preflight, poll 1 is the parser preflight, and the following
        // polls are dataset cells. Firing at poll 3 therefore proves the signal is observed
        // inside the quad scan rather than only after a complete knowledge base exists.
        let stop = Arc::new(StopAtPoll::new(3));
        let signal: Arc<dyn StopSignal> = stop.clone();
        let Err(error) = Kb::from_dataset_until(&simple_dataset(), Some(signal)) else {
            panic!("reverse mapping must observe cancellation during its quad scan");
        };
        assert!(matches!(error, EntailError::Stopped));
        assert_eq!(stop.polls(), 4);
    }

    #[test]
    fn governed_construction_installs_stop_before_key_inference() {
        let mut b = RdfDatasetBuilder::new();
        let ty = vocab(&mut b, RDF_TYPE);
        let has_key = vocab(&mut b, OWL_HASKEY);
        let first = vocab(&mut b, RDF_FIRST);
        let rest = vocab(&mut b, RDF_REST);
        let nil = vocab(&mut b, RDF_NIL);
        let class = iri(&mut b, "Keyed");
        let property = iri(&mut b, "key");
        let list = iri(&mut b, "key-list");
        let left = iri(&mut b, "left");
        let right = iri(&mut b, "right");
        let value = iri(&mut b, "value");
        b.push_quad(class, has_key, list, None);
        b.push_quad(list, first, property, None);
        b.push_quad(list, rest, nil, None);
        b.push_quad(left, ty, class, None);
        b.push_quad(right, ty, class, None);
        b.push_quad(left, property, value, None);
        b.push_quad(right, property, value, None);
        let dataset = b.freeze().expect("key fixture freezes");

        // Measure the parser's deterministic poll count, then let the API preflight, every
        // parser poll, and the post-build check pass. The next poll is inside the first
        // key-membership proof and must still carry the same signal.
        let parser_counter = StopAtPoll::new(u64::MAX);
        parser::build_until(&dataset, Some(&parser_counter)).expect("count parser polls");
        let stop = Arc::new(StopAtPoll::new(parser_counter.polls() + 2));
        let signal: Arc<dyn StopSignal> = stop.clone();
        let Err(error) = Kb::from_dataset_until(&dataset, Some(signal)) else {
            panic!("the first key-inference poll must stop construction");
        };
        assert!(matches!(error, EntailError::Stopped));
        assert!(stop.polls() > parser_counter.polls() + 2);
    }

    /// Build the `parent.ttl` knowledge base directly (Concepts + axioms).
    fn parent_kb() -> (Kb, BTreeMap<&'static str, u32>) {
        let mut kb = Kb::empty();
        let mut ids: BTreeMap<&'static str, u32> = BTreeMap::new();
        let mut id = |kb: &mut Kb, name: &'static str| -> u32 {
            *ids.entry(name)
                .or_insert_with(|| kb.interner.intern_iri(&format!("{NS}{name}")))
        };
        let male = id(&mut kb, "Male");
        let female = id(&mut kb, "Female");
        let parent = id(&mut kb, "Parent");
        let father = id(&mut kb, "Father");
        let mother = id(&mut kb, "Mother");
        let has_child = id(&mut kb, "hasChild");
        let alice = id(&mut kb, "Alice");
        let bob = id(&mut kb, "Bob");
        let charlie = id(&mut kb, "Charlie");
        let dudley = id(&mut kb, "Dudley");

        // Father ≡ Male ⊓ Parent
        kb.push_gci(
            Concept::Named(father),
            Concept::And(vec![Concept::Named(male), Concept::Named(parent)]),
        );
        kb.push_gci(
            Concept::And(vec![Concept::Named(male), Concept::Named(parent)]),
            Concept::Named(father),
        );
        // Mother ≡ Female ⊓ Parent
        kb.push_gci(
            Concept::Named(mother),
            Concept::And(vec![Concept::Named(female), Concept::Named(parent)]),
        );
        kb.push_gci(
            Concept::And(vec![Concept::Named(female), Concept::Named(parent)]),
            Concept::Named(mother),
        );
        // Parent ≡ ∃hasChild.⊤
        kb.push_gci(
            Concept::Named(parent),
            Concept::Some(Role::Named(has_child), Box::new(Concept::Top)),
        );
        kb.push_gci(
            Concept::Some(Role::Named(has_child), Box::new(Concept::Top)),
            Concept::Named(parent),
        );

        // Individuals.
        for a in [alice, bob, charlie, dudley] {
            kb.individuals.insert(a);
        }
        let female_id = kb.table.intern(Concept::Named(female));
        let parent_id = kb.table.intern(Concept::Named(parent));
        let male_id = kb.table.intern(Concept::Named(male));
        kb.abox_types.push((alice, female_id));
        kb.abox_types.push((alice, parent_id));
        kb.abox_types.push((bob, male_id));
        // Bob hasChild Charlie; Dudley hasChild Alice.
        kb.abox_roles.push((bob, has_child, charlie));
        kb.abox_roles.push((dudley, has_child, alice));
        // Dudley : ∀hasChild.{Alice}
        let dudley_all = kb.table.intern(Concept::All(
            Role::Named(has_child),
            Box::new(Concept::Nominal(vec![alice])),
        ));
        kb.abox_types.push((dudley, dudley_all));

        kb.finalize();
        (kb, ids)
    }

    #[test]
    fn parent_existential_instance_retrieval() {
        let (mut kb, ids) = parent_kb();
        let has_child = ids["hasChild"];
        let alice = ids["Alice"];
        let bob = ids["Bob"];
        let dudley = ids["Dudley"];

        // ∃hasChild.⊤ → {Alice, Bob, Dudley}.
        let some_child = kb.intern_query(Concept::Some(
            Role::Named(has_child),
            Box::new(Concept::Top),
        ));
        let mut expected = [alice, bob, dudley];
        expected.sort_unstable();
        assert_eq!(
            kb.instances_of(some_child).unwrap(),
            expected.to_vec(),
            "∃hasChild.⊤ = {{Alice, Bob, Dudley}}"
        );
        assert!(
            kb.entails_instance(alice, some_child).unwrap(),
            "Alice IS a parent (via Parent ≡ ∃hasChild.⊤)"
        );

        // ≥1 hasChild.⊤ equals the same set (unqualified min-1 = ∃).
        let min_child = kb.intern_query(Concept::Min(
            1,
            Role::Named(has_child),
            Box::new(Concept::Top),
        ));
        assert_eq!(
            kb.instances_of(min_child).unwrap(),
            expected.to_vec(),
            "≥1 hasChild.⊤ = ∃hasChild.⊤"
        );
    }

    #[test]
    fn subsumption_reflexive_and_bottom() {
        let (mut kb, ids) = parent_kb();
        let father = ids["Father"];
        let parent = ids["Parent"];
        let father_id = kb.intern_query(Concept::Named(father));
        let parent_id = kb.intern_query(Concept::Named(parent));
        let bottom = kb.bottom;
        // Reflexive: Father ⊑ Father.
        assert!(kb.entails_subclass(father_id, father_id).unwrap());
        // Father ⊑ Parent (Father ≡ Male ⊓ Parent).
        assert!(kb.entails_subclass(father_id, parent_id).unwrap());
        // ⊥ ⊑ everything.
        assert!(kb.entails_subclass(bottom, parent_id).unwrap());
        // Parent ⋢ Father (not every parent is male).
        assert!(!kb.entails_subclass(parent_id, father_id).unwrap());
    }

    /// `Kb::from_dataset` already runs [`Kb::encode_until`]'s absorption pass once (inside
    /// `parser::build_until`), so a caller that finalizes again after interning MORE
    /// concepts — the shape `Reasoner::new` is — must not pay for a second absorption over
    /// an unchanged [`Kb::tbox`].
    #[test]
    fn finalizing_again_after_interning_more_concepts_does_not_re_absorb() {
        let ds = simple_dataset();
        let mut kb = Kb::from_dataset(&ds).expect("parse");
        assert_eq!(
            kb.absorb_calls, 1,
            "the reverse mapping's own encode_until call is the first and only absorption"
        );

        // Mirror what `Reasoner::new` does before its own `finalize()`: intern a fresh
        // concept — a named class the reverse mapping never had reason to intern — then
        // finalize again.
        let acls = kb.iri_id(&format!("{NS}A")).unwrap();
        kb.table.intern(Concept::Named(acls));
        kb.finalize();
        assert_eq!(
            kb.absorb_calls, 1,
            "Kb::tbox did not change, so the second finalize() must not re-absorb it"
        );

        // Growing the TBox is the one thing that must force a re-absorption, proving the
        // skip is conditional on `Kb::tbox` rather than a blanket no-op after the first call.
        kb.push_gci(Concept::Named(acls), Concept::Top);
        kb.finalize();
        assert_eq!(
            kb.absorb_calls, 2,
            "a grown Kb::tbox must be re-absorbed on the next finalize()"
        );
    }
}

/// The ABSORBED terminology, driven through the real reverse mapping.
///
/// Everything here reaches the decision core the way an ontology does — RDF triples, the
/// parser, [`Kb::finalize`] — rather than through a hand-assembled knowledge base, because
/// what these cases pin is the WIRING: which clause a reverse-mapped axiom becomes, and which
/// nodes it is allowed to fire at. Every verdict is taken from BOTH calculi, so a case here
/// cannot pass by one of them being wrong in the same direction as the other.
#[cfg(test)]
mod absorption_tests {
    use super::{Kb, hyper, tableau};
    use crate::owl_dl::graph::{Assumptions, Budget};
    use crate::vocab::{
        OWL_NOTHING, OWL_ONPROPERTY, OWL_RESTRICTION, OWL_SOMEVALUESFROM, OWL_THING, RDF_TYPE,
        RDFS_RANGE, RDFS_SUBCLASSOF,
    };
    use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder, TermValue};

    /// The fixture property.
    const EX_R: &str = "http://example.org/r";
    /// A fixture class.
    const EX_A: &str = "http://example.org/A";
    /// A fixture individual.
    const EX_SUBJECT: &str = "http://example.org/a";
    /// A second fixture individual.
    const EX_OBJECT: &str = "http://example.org/b";

    /// Whether the dataset is consistent, DECIDED BY BOTH cores, which must agree.
    fn consistent_by_both(ds: &RdfDataset) -> bool {
        let kb = Kb::from_dataset(ds).expect("the fixture parses");
        let cap = Budget::for_kb(&kb);
        let hyper = hyper::decide(&kb, &Assumptions::of_kb(), cap);
        let reference = tableau::decide(&kb, &Assumptions::of_kb(), cap);
        assert!(
            !hyper.exhausted && !reference.exhausted,
            "a fixture this small must decide inside both caps"
        );
        assert_eq!(
            hyper.consistent, reference.consistent,
            "the hypertableau and the concept-tree tableau disagree about the absorbed \
             terminology"
        );
        hyper.consistent
    }

    /// `∃r.⊤ ⊑ A`, `A ⊑ ⊥`, `a r b` — INCONSISTENT, and the case that refutes reading an
    /// absorbed antecedent contrapositively.
    ///
    /// `a` has an `r`-successor, so `a ∈ (∃r.⊤)^I`, so `a ∈ A^I`, which `A ⊑ ⊥` forbids. The
    /// derivation has to run FORWARDS from the edge; a design that fired the axiom on nodes
    /// LABELLED with the antecedent's negation would see nothing here at all, because nothing
    /// ever labels `a` with `¬∃r.⊤` — the completion graph simply has the edge. That reading
    /// was refuted before it was written, and this is the knowledge base that refutes it.
    #[test]
    fn an_existential_antecedent_fires_from_the_edge_and_not_from_a_label() {
        let mut b = RdfDatasetBuilder::new();
        let ty = b.intern_iri(RDF_TYPE);
        let sub_class = b.intern_iri(RDFS_SUBCLASSOF);
        let restriction = b.intern_iri(OWL_RESTRICTION);
        let on_property = b.intern_iri(OWL_ONPROPERTY);
        let some_values = b.intern_iri(OWL_SOMEVALUESFROM);
        let thing = b.intern_iri(OWL_THING);
        let nothing = b.intern_iri(OWL_NOTHING);
        let r = b.intern_iri(EX_R);
        let a_class = b.intern_iri(EX_A);
        let subject = b.intern_iri(EX_SUBJECT);
        let object = b.intern_iri(EX_OBJECT);
        let node = b.intern_blank("restriction", BlankScope::DEFAULT);
        b.push_quad(node, ty, restriction, None);
        b.push_quad(node, on_property, r, None);
        b.push_quad(node, some_values, thing, None);
        b.push_quad(node, sub_class, a_class, None);
        b.push_quad(a_class, sub_class, nothing, None);
        b.push_quad(subject, r, object, None);
        let ds = b.freeze().expect("freeze");
        assert!(
            !consistent_by_both(&ds),
            "an r-edge out of `a` puts it in ∃r.⊤, and ∃r.⊤ ⊑ A ⊑ ⊥ closes every branch"
        );
    }

    /// A TBox clause fires on the OBJECT domain only.
    ///
    /// `rdfs:range` propagates the named class `A` onto whatever an `r`-edge reaches, and an
    /// `r`-edge can reach a LITERAL — a node of `Δ_D`. The inclusion `A ⊑ ⊥` quantifies over
    /// `owl:Thing`, so it says nothing about a literal value, and firing it there would refute
    /// a knowledge base on the strength of an axiom that never ranged over the element it
    /// closed. The internalized encoding never had this exposure (a concrete node is not
    /// seeded with the TBox); the absorbed encoding is a CLAUSE, so the scope has to be stated
    /// in the rule that fires it.
    ///
    /// The two halves are the observable: the literal object is consistent and the IRI object
    /// is not, over axioms that are otherwise identical. So the clause is still firing — it is
    /// firing on exactly the domain the axiom quantifies over.
    #[test]
    fn a_tbox_clause_does_not_fire_on_a_node_of_the_data_domain() {
        let build = |literal_object: bool| {
            let mut b = RdfDatasetBuilder::new();
            let sub_class = b.intern_iri(RDFS_SUBCLASSOF);
            let range = b.intern_iri(RDFS_RANGE);
            let nothing = b.intern_iri(OWL_NOTHING);
            let r = b.intern_iri(EX_R);
            let a_class = b.intern_iri(EX_A);
            let subject = b.intern_iri(EX_SUBJECT);
            let object = if literal_object {
                crate::interner::intern_into(&mut b, &TermValue::simple_literal("cat"))
            } else {
                b.intern_iri(EX_OBJECT)
            };
            b.push_quad(r, range, a_class, None);
            b.push_quad(a_class, sub_class, nothing, None);
            b.push_quad(subject, r, object, None);
            b.freeze().expect("freeze")
        };
        assert!(
            consistent_by_both(&build(true)),
            "a general concept inclusion does not quantify over literal VALUES, so it cannot \
             close a branch on one"
        );
        assert!(
            !consistent_by_both(&build(false)),
            "the same axioms over an IRI object ARE refuted, which is what makes the case \
             above a scope and not a dropped clause"
        );
    }
}

#[cfg(test)]
mod boundary_tests {
    use crate::owl_dl::Kb;
    use crate::report::Construct;
    use crate::vocab::{
        OWL_PROPERTYCHAINAXIOM, OWL_TRANSITIVEPROPERTY, RDF_FIRST, RDF_NIL, RDF_REST, RDF_TYPE,
        RDFS_SUBCLASSOF, XSD_NONNEGATIVEINTEGER,
    };
    use crate::{Completeness, QTriple, materialize_dl_reported};
    use purrdf_core::{BlankScope, RdfDatasetBuilder, TermId, TermValue};

    /// A fixture property that a chain axiom composes into.
    const EX_CHAINED: &str = "http://example.org/chained";
    /// The first property of the chain.
    const EX_Q: &str = "http://example.org/q";
    /// The second property of the chain.
    const EX_R: &str = "http://example.org/r";
    /// A fixture class.
    const EX_C: &str = "http://example.org/C";
    /// A fixture individual.
    const EX_A: &str = "http://example.org/a";

    /// `owl:propertyChainAxiom` is the construct that was not merely dropped but MIS-READ:
    /// the reverse mapping's catch-all ingested it as a role assertion whose object was the
    /// axiom's RDF list head, so the knowledge base held `chained(chained, _:cell0)` — a
    /// statement the ontology does not make, about an individual that does not exist.
    ///
    /// This asserts all three halves of the repair: the boundary is RAISED, the list head
    /// is NOT an individual, and no role assertion was invented.
    #[test]
    fn a_property_chain_is_bounded_rather_than_ingested_as_a_role_assertion() {
        let mut b = RdfDatasetBuilder::new();
        let chained = b.intern_iri(EX_CHAINED);
        let chain = b.intern_iri(OWL_PROPERTYCHAINAXIOM);
        let first = b.intern_iri(RDF_FIRST);
        let rest = b.intern_iri(RDF_REST);
        let nil = b.intern_iri(RDF_NIL);
        let q = b.intern_iri(EX_Q);
        let r = b.intern_iri(EX_R);
        let cell1 = b.intern_blank("cell1", BlankScope::DEFAULT);
        let cell0 = b.intern_blank("cell0", BlankScope::DEFAULT);
        b.push_quad(cell1, first, r, None);
        b.push_quad(cell1, rest, nil, None);
        b.push_quad(cell0, first, q, None);
        b.push_quad(cell0, rest, cell1, None);
        b.push_quad(chained, chain, cell0, None);
        let ds = b.freeze().expect("freeze");

        let kb = Kb::from_dataset(&ds).expect("the chain axiom parses");
        assert!(
            kb.boundaries().contains(&Construct::PropertyChain),
            "a chain axiom must raise its boundary: {:?}",
            kb.boundaries()
        );
        assert!(
            kb.abox_roles.is_empty(),
            "a chain axiom must not become a role assertion: {:?}",
            kb.abox_roles
        );
        assert!(
            kb.individuals.is_empty(),
            "the chain's RDF list head must not become an individual: {:?}",
            kb.individuals
        );

        // …and the boundary reaches the caller, with the completeness narrowed to match.
        let (_, report) = materialize_dl_reported(&ds, &[] as &[QTriple]).expect("consistent");
        assert_eq!(
            report.completeness(),
            Completeness::ExactWithinBoundaries,
            "a run that met a boundary is not exact"
        );
        let constructs: Vec<Construct> = report
            .boundaries()
            .iter()
            .map(|boundary| boundary.construct())
            .collect();
        assert_eq!(constructs, vec![Construct::PropertyChain]);
        assert!(
            report.boundaries()[0].reason().contains("REGULARITY"),
            "the reason must name the check that is missing"
        );
    }

    /// OWL 2 DL forbids a number restriction over a NON-SIMPLE role, and the condition is
    /// only decidable once every transitivity axiom has been read — so it is checked after
    /// the scan, not while the restriction is being decoded.
    #[test]
    fn a_number_restriction_over_a_transitive_role_is_bounded() {
        let build = |transitive: bool| {
            let mut b = RdfDatasetBuilder::new();
            let ty = b.intern_iri(RDF_TYPE);
            let sub_class = b.intern_iri(RDFS_SUBCLASSOF);
            let restriction = b.intern_iri(crate::vocab::OWL_RESTRICTION);
            let on_property = b.intern_iri(crate::vocab::OWL_ONPROPERTY);
            let max_card = b.intern_iri(crate::vocab::OWL_MAXCARDINALITY);
            let c = b.intern_iri(EX_C);
            let r = b.intern_iri(EX_R);
            let node = b.intern_blank("restriction", BlankScope::DEFAULT);
            let one: TermId = crate::interner::intern_into(
                &mut b,
                &TermValue::typed_literal("1", XSD_NONNEGATIVEINTEGER),
            );
            b.push_quad(c, sub_class, node, None);
            b.push_quad(node, ty, restriction, None);
            b.push_quad(node, on_property, r, None);
            b.push_quad(node, max_card, one, None);
            if transitive {
                let trans = b.intern_iri(OWL_TRANSITIVEPROPERTY);
                b.push_quad(r, ty, trans, None);
            }
            b.freeze().expect("freeze")
        };

        let simple = Kb::from_dataset(&build(false)).expect("parse");
        assert!(
            simple.boundaries().is_empty(),
            "a SIMPLE role counted by a number restriction is ordinary OWL 2 DL: {:?}",
            simple.boundaries()
        );
        let non_simple = Kb::from_dataset(&build(true)).expect("parse");
        assert!(
            non_simple.boundaries().contains(&Construct::NonSimpleRole),
            "counting a transitive role is outside OWL 2 DL: {:?}",
            non_simple.boundaries()
        );
    }

    /// The role characteristics REACH the knowledge base rather than being dropped, and
    /// each lands where its DL reading says it should.
    #[test]
    fn the_role_characteristics_reach_the_knowledge_base() {
        let mut b = RdfDatasetBuilder::new();
        let ty = b.intern_iri(RDF_TYPE);
        let r = b.intern_iri(EX_R);
        let q = b.intern_iri(EX_Q);
        let transitive = b.intern_iri(OWL_TRANSITIVEPROPERTY);
        let symmetric = b.intern_iri(crate::vocab::OWL_SYMMETRICPROPERTY);
        let asymmetric = b.intern_iri(crate::vocab::OWL_ASYMMETRICPROPERTY);
        b.push_quad(r, ty, transitive, None);
        b.push_quad(r, ty, symmetric, None);
        b.push_quad(q, ty, asymmetric, None);
        let ds = b.freeze().expect("freeze");
        let kb = Kb::from_dataset(&ds).expect("parse");
        assert!(kb.boundaries().is_empty(), "{:?}", kb.boundaries());
        let r_id = kb.iri_id(EX_R).expect("the property was interned");
        let q_id = kb.iri_id(EX_Q).expect("the property was interned");
        assert!(kb.transitive.contains(&r_id), "owl:TransitiveProperty");
        assert!(
            kb.inverses
                .get(&r_id)
                .is_some_and(|set| set.contains(&r_id)),
            "owl:SymmetricProperty is r ≡ r⁻"
        );
        assert!(kb.asymmetric.contains(&q_id), "owl:AsymmetricProperty");
    }

    /// A DATA-property assertion is ingested (its literal object is an opaque abstract
    /// term), and the literal is NOT a realization candidate — a type triple with a literal
    /// subject is a generalized-RDF triple the dataset IR cannot hold.
    #[test]
    fn a_data_property_assertion_is_ingested_without_naming_its_literal() {
        let mut b = RdfDatasetBuilder::new();
        let a = b.intern_iri(EX_A);
        let r = b.intern_iri(EX_R);
        let value = crate::interner::intern_into(&mut b, &TermValue::simple_literal("cat"));
        b.push_quad(a, r, value, None);
        let ds = b.freeze().expect("freeze");
        let kb = Kb::from_dataset(&ds).expect("parse");
        assert_eq!(kb.abox_roles.len(), 1, "the assertion is ingested");
        assert_eq!(
            kb.individuals.len(),
            1,
            "only the subject is a named individual: {:?}",
            kb.individuals
        );
        assert!(kb.boundaries().is_empty(), "{:?}", kb.boundaries());
    }

    /// `rdfs:range` is the general concept inclusion `⊤ ⊑ ∀r.C`, and it reaches the search as
    /// the EDGE CLAUSE `r(x,y) → C(y)` — nothing enters any node's label for it.
    ///
    /// Two costs are what this pins away. Internalized, the axiom is a concept seeded into
    /// EVERY node of every completion, so it widens every blocking signature and is re-read by
    /// the `∀`-rule at every node in every round; and before the concept table canonicalized
    /// its disjunctions it arrived as `⊔{⊥, ∀r.C}`, a guaranteed-failing case split paid on
    /// ontologies that state nothing but a range. As a clause it is one body atom that fails
    /// immediately at a node with no `r`-edge.
    #[test]
    fn a_range_axiom_becomes_an_edge_clause_rather_than_a_label() {
        use crate::owl_dl::clause::BodyAtom;
        use crate::owl_dl::concept::{Concept, Role};
        use crate::vocab::RDFS_RANGE;

        let mut b = RdfDatasetBuilder::new();
        let r = b.intern_iri(EX_R);
        let range = b.intern_iri(RDFS_RANGE);
        let c = b.intern_iri(EX_C);
        b.push_quad(r, range, c, None);
        let ds = b.freeze().expect("freeze");
        let kb = Kb::from_dataset(&ds).expect("parse");

        assert!(
            kb.meta.is_empty(),
            "a range axiom must seed nothing into a node label: {:?}",
            kb.meta
        );
        assert_eq!(kb.absorbed.len(), 1, "one range axiom, one clause");
        let clause = &kb.absorbed[0];
        let property = kb.iri_id(EX_R).expect("the property was interned");
        assert_eq!(
            clause.body,
            vec![BodyAtom::Role {
                from: 0,
                to: 1,
                role: Role::Named(property),
            }]
        );
        assert_eq!(clause.head_var, 1, "the filler lands on the SUCCESSOR");
        let class = kb.iri_id(EX_C).expect("the class was interned");
        assert_eq!(
            kb.table.concept(clause.head),
            &Concept::Named(class),
            "the head is the range class itself"
        );
    }
}
