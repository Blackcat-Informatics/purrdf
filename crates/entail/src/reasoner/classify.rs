// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The classifier: the entailed subsumption relation over an ontology's named classes.
//!
//! Two pieces, deliberately separable, because they have different reasons to change and
//! very different costs:
//!
//! * the subsumption MATRIX is the reasoning half. It is a matrix of [`Verdict`]s, so an
//!   undecided pair stays visibly undecided instead of collapsing into "not subsumed".
//! * [`ClassHierarchy`] is the cheap half — a pure derivation from that matrix, with no
//!   reasoner in sight: equivalence classes, the unsatisfiable classes, and the transitive
//!   reduction a reader actually wants to look at. It is also what the realizer consumes
//!   to decide which of an individual's entailed types are its MOST SPECIFIC ones, which
//!   is why the matrix is a value rather than a private detail of one function.
//!
//! # The matrix is DERIVED, and only the residue is refuted
//!
//! The matrix used to be filled by one tableau refutation per ordered pair — `n²` complete
//! completions to build one taxonomy. It is filled instead by
//! the crate-internal `owl_dl::saturate` calculus: ONE consequence-based fixpoint over
//! the whole clause set derives every entailed subsumption at once, and every rule of that
//! calculus is sound, so a derived pair needs no refutation at all.
//!
//! What the saturation cannot do is REFUTE. A pair it did not derive is genuinely
//! not-subsumed only when the ontology lies in the fragment the calculus is complete for
//! (`owl_dl::saturate`'s module documentation states the exact condition). So:
//!
//! * inside the fragment, an underivable pair is [`Verdict::False`] and no tableau runs —
//!   classification costs the ONE consistency decision the session already made;
//! * outside it, exactly the underivable pairs go to the tableau, which decides them the way
//!   it always did. [`DlCertificate::decisions`](super::DlCertificate::decisions) reports how
//!   many that was, so the saving is a measurement rather than a claim.
//!
//! Neither path can report a subsumption the tableau would refute: a derivation is a proof,
//! and a refutation is the tableau's own answer.
//!
//! # Subsumption is decided against the whole knowledge base
//!
//! `KB ⊨ C ⊑ D` exactly when `KB ∪ {x : C ⊓ ¬D}` is inconsistent for a fresh `x`, and the
//! `KB` there includes the ABox. That is not pedantry: with nominals an ASSERTION changes
//! the class hierarchy — `C ≡ {a}` together with `a : D` entails `C ⊑ D`, and a TBox-only
//! test cannot see it. So every refutation here runs with the ABox loaded, and
//! `a_nominal_class_is_subsumed_through_an_assertion` is the fixture that would fail if
//! anyone narrowed it back. A nominal is also outside the saturation's fragment for exactly
//! that reason, so such an ontology takes the residual-tableau path and the fixture keeps
//! passing for the same reason it always did.
//!
//! # Determinism
//!
//! Classes are visited in ascending interned-term-id order (which is parse order), the
//! matrix is a dense `Vec<Verdict>` indexed by position, and every emitted sequence is
//! sorted by a total, dataset-independent term key. Nothing is read out of a hash map.

use purrdf_core::TermValue;

use super::certificate::{Session, Verdict};
use super::term_key;
use crate::owl_dl::Kb;
use crate::owl_dl::graph::Assumptions;
use crate::owl_dl::saturate::saturate;

/// The entailed subsumption relation over a fixed, ordered list of named classes.
///
/// Row-major and dense: `verdict(sub, sup)` is the answer for the `sub`-th class being
/// subsumed by the `sup`-th, both indices into the `classes` slice the matrix was built
/// over. Undecided pairs are [`Verdict::Unknown`] rather than absent, so a consumer cannot
/// mistake "the budget ran out" for "no".
pub(crate) struct Subsumptions {
    /// The number of classes; the matrix is `n × n`.
    n: usize,
    /// `n × n` verdicts, row-major, indexed `sub * n + sup`.
    verdicts: Vec<Verdict>,
}

impl Subsumptions {
    /// Decide every ordered pair of `classes` (a slice of `(term id, concept id)`).
    ///
    /// One saturation derives the entailed pairs; the tableau is asked only about the pairs
    /// it did not derive, and only when the saturation was not complete for this ontology.
    /// Reflexive pairs are never sent to the tableau: `C ⊑ C` holds in every interpretation,
    /// so asking would spend a decision to learn an axiom of the logic.
    pub(crate) fn decide(session: &mut Session<'_>, classes: &[(u32, u32)]) -> Self {
        let kb = session.kb();
        let seeds: Vec<u32> = classes.iter().map(|&(_, concept)| concept).collect();
        let taxonomy = saturate(kb, &seeds);
        let complete = taxonomy.is_complete();
        let n = classes.len();
        let mut verdicts = vec![Verdict::False; n * n];
        for (i, &(_, sub)) in classes.iter().enumerate() {
            for (j, &(_, sup)) in classes.iter().enumerate() {
                verdicts[i * n + j] = if i == j || taxonomy.derives(sub, sup) {
                    Verdict::True
                } else if complete {
                    Verdict::False
                } else {
                    subsumes(session, sub, sup)
                };
            }
        }
        Self { n, verdicts }
    }

    /// Decide every ordered pair by REFUTATION alone — one tableau run per pair.
    ///
    /// The pre-saturation classifier, kept reachable so
    /// `the_saturation_agrees_with_the_tableau_pair_by_pair` can hold two implementations of
    /// one contract against each other. A differential test needs both sides to exist; a
    /// deleted reference implementation is a test that can only assert the new code agrees
    /// with itself.
    #[cfg(test)]
    pub(crate) fn decide_by_tableau(session: &mut Session<'_>, classes: &[(u32, u32)]) -> Self {
        let n = classes.len();
        let mut verdicts = vec![Verdict::False; n * n];
        for (i, &(_, sub)) in classes.iter().enumerate() {
            for (j, &(_, sup)) in classes.iter().enumerate() {
                verdicts[i * n + j] = if i == j {
                    Verdict::True
                } else {
                    subsumes(session, sub, sup)
                };
            }
        }
        Self { n, verdicts }
    }

    /// The verdict for `classes[sub] ⊑ classes[sup]`.
    pub(crate) fn verdict(&self, sub: usize, sup: usize) -> Verdict {
        self.verdicts[sub * self.n + sup]
    }

    /// Whether `classes[sub] ⊑ classes[sup]` was ESTABLISHED — an undecided pair is not.
    pub(crate) fn holds(&self, sub: usize, sup: usize) -> bool {
        self.verdict(sub, sup).is_true()
    }

    /// Whether the two classes were established equivalent.
    pub(crate) fn equivalent(&self, a: usize, b: usize) -> bool {
        self.holds(a, b) && self.holds(b, a)
    }

    /// The index of the canonical member of `i`'s equivalence class — the lowest index it
    /// was established equivalent to, so a whole equivalence class collapses to one
    /// representative deterministically.
    pub(crate) fn representative(&self, i: usize) -> usize {
        (0..=i).find(|&j| self.equivalent(i, j)).unwrap_or(i)
    }

    /// Whether `classes[sub]` is strictly below `classes[sup]` — subsumed and not
    /// equivalent.
    fn strictly_below(&self, sub: usize, sup: usize) -> bool {
        self.holds(sub, sup) && !self.holds(sup, sub)
    }
}

/// Whether `KB ⊨ sub ⊑ sup`, by refuting `x : sub ⊓ ¬sup` over a fresh anonymous witness.
///
/// The witness is a node of the completion graph with no name at all — not a minted IRI,
/// not even a blank node, because a subsumption question needs an ARBITRARY element rather
/// than a nameable one. See [`super::axiom`] for the case that does need nameable fresh
/// symbols, and for why they are blank nodes.
pub(crate) fn subsumes(session: &mut Session<'_>, sub: u32, sup: u32) -> Verdict {
    let neg_sup = session.kb().table.negate(sup);
    session.refutes(&Assumptions {
        fresh_types: &[sub, neg_sup],
        ..Assumptions::of_kb()
    })
}

/// The classified hierarchy of an ontology's named classes.
///
/// `owl:Thing` and `owl:Nothing` participate: they are read as `⊤` and `⊥` rather than as
/// opaque atomic classes, so `⊥ ⊑ C ⊑ ⊤` appears for every named `C`, and a class the
/// ontology forces empty shows up equivalent to `owl:Nothing` rather than in a separate
/// list nobody joins against.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClassHierarchy {
    /// Every ESTABLISHED subsumption `C ⊑ D` with `C` and `D` distinct terms.
    subsumptions: Vec<(TermValue, TermValue)>,
    /// Every established equivalence, each unordered pair once.
    equivalences: Vec<(TermValue, TermValue)>,
    /// Named classes established equivalent to `owl:Nothing`.
    unsatisfiable: Vec<TermValue>,
    /// The transitive reduction of [`Self::subsumptions`].
    direct: Vec<(TermValue, TermValue)>,
}

impl ClassHierarchy {
    /// Every established subsumption `C ⊑ D` between two DISTINCT named class terms,
    /// sorted.
    ///
    /// The full relation, transitively closed — not the reduction. Reflexive pairs are
    /// omitted: `C ⊑ C` is a theorem of the logic rather than a fact about this ontology,
    /// and listing it once per class would bury the ones that are.
    #[must_use]
    pub fn subsumptions(&self) -> &[(TermValue, TermValue)] {
        &self.subsumptions
    }

    /// Every established equivalence `C ≡ D`, each unordered pair listed once with the
    /// lexicographically smaller term first, sorted.
    #[must_use]
    pub fn equivalences(&self) -> &[(TermValue, TermValue)] {
        &self.equivalences
    }

    /// The named classes the ontology forces empty — those established equivalent to
    /// `owl:Nothing`, sorted.
    ///
    /// `owl:Nothing` itself is in the list, because it IS empty; a caller looking for the
    /// ontology's own modelling errors filters it out, and one asking "which of these
    /// classes are empty" gets a correct answer without a special case.
    #[must_use]
    pub fn unsatisfiable(&self) -> &[TermValue] {
        &self.unsatisfiable
    }

    /// The transitive reduction of [`Self::subsumptions`]: `(C, D)` where `D` is a DIRECT
    /// subsumer of `C`, sorted.
    ///
    /// Computed over equivalence-class representatives, so a cycle of mutually subsuming
    /// classes contributes one node rather than an unreadable clique, and only the
    /// representative appears here — the other members are in [`Self::equivalences`].
    ///
    /// # What a `BudgetExhausted` certificate does to this list
    ///
    /// The reduction is derived from the subsumptions that were ESTABLISHED. If the
    /// certificate reports [`DlCompleteness::BudgetExhausted`](super::DlCompleteness), a
    /// pair listed here may have an intermediate class the search did not get to, so
    /// "direct" means "direct as far as this run decided". Every pair listed is still a
    /// genuine subsumption; it is the DIRECTNESS that weakens, and the certificate is
    /// where a caller finds that out.
    #[must_use]
    pub fn direct_subsumptions(&self) -> &[(TermValue, TermValue)] {
        &self.direct
    }

    /// Derive the hierarchy from a decided subsumption matrix.
    pub(crate) fn derive(kb: &Kb, classes: &[(u32, u32)], m: &Subsumptions) -> Self {
        let name = |i: usize| kb.interner.value(classes[i].0).clone();
        let n = classes.len();

        let mut subsumptions = Vec::new();
        let mut equivalences = Vec::new();
        for i in 0..n {
            for j in 0..n {
                if i == j || !m.holds(i, j) {
                    continue;
                }
                subsumptions.push((name(i), name(j)));
                if i < j && m.holds(j, i) {
                    equivalences.push((name(i), name(j)));
                }
            }
        }

        // A class is unsatisfiable exactly when it is subsumed by `owl:Nothing`'s concept,
        // which the signature always carries; if the signature somehow lacks it there is
        // nothing to compare against and the list is empty rather than guessed.
        let bottom = classes.iter().position(|&(_, cid)| cid == kb.bottom);
        let unsatisfiable: Vec<TermValue> = bottom.map_or_else(Vec::new, |b| {
            (0..n).filter(|&i| m.holds(i, b)).map(name).collect()
        });

        // The transitive reduction over equivalence-class representatives.
        //
        // The representatives are computed ONCE into a dense positional vector rather than
        // re-derived at each of the reduction's three loop levels:
        // `Subsumptions::representative` scans an equivalence class to find its lowest
        // member, so calling it from the
        // innermost `any` would put a linear scan under a cubic loop. Memoizing changes no
        // answer — it is the same function of the same matrix — and it is what keeps
        // classifying a large signature bounded by the reduction rather than by bookkeeping.
        let representatives: Vec<usize> = (0..n).map(|i| m.representative(i)).collect();
        let canonical: Vec<usize> = (0..n).filter(|&i| representatives[i] == i).collect();
        let mut direct = Vec::new();
        for &i in &canonical {
            for &j in &canonical {
                if !m.strictly_below(i, j) {
                    continue;
                }
                let interposed = canonical
                    .iter()
                    .any(|&k| m.strictly_below(i, k) && m.strictly_below(k, j));
                if !interposed {
                    direct.push((name(i), name(j)));
                }
            }
        }

        let mut out = Self {
            subsumptions,
            equivalences,
            unsatisfiable,
            direct,
        };
        out.sort();
        out
    }

    /// Put every sequence into the crate's canonical term order.
    fn sort(&mut self) {
        let pair = |(a, b): &(TermValue, TermValue)| (term_key(a), term_key(b));
        self.subsumptions.sort_by_key(pair);
        self.equivalences.sort_by_key(pair);
        self.unsatisfiable.sort_by_key(term_key);
        self.direct.sort_by_key(pair);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::sync::Arc;

    use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder, TermId, TermValue};

    use super::{ClassHierarchy, Subsumptions};
    use crate::owl_dl::saturate::saturate;
    use crate::reasoner::Reasoner;
    use crate::vocab::{
        OWL_ALLVALUESFROM, OWL_CLASS, OWL_COMPLEMENTOF, OWL_DISJOINTWITH, OWL_EQUIVALENTCLASS,
        OWL_INTERSECTIONOF, OWL_INVERSEOF, OWL_MAXCARDINALITY, OWL_OBJECTPROPERTY, OWL_ONEOF,
        OWL_ONPROPERTY, OWL_RESTRICTION, OWL_SOMEVALUESFROM, OWL_THING, OWL_TRANSITIVEPROPERTY,
        OWL_UNIONOF, RDF_FIRST, RDF_NIL, RDF_REST, RDF_TYPE, RDFS_DOMAIN, RDFS_RANGE,
        RDFS_SUBCLASSOF, RDFS_SUBPROPERTYOF, XSD_NONNEGATIVEINTEGER,
    };

    /// The fixture namespace: `example.org`, per the project rule that a test mints no
    /// vocabulary of its own.
    const EX: &str = "http://example.org/";

    /// A tiny fixture builder: `iri` and `blank` name terms, `push` states a triple.
    struct Fixture {
        /// The dataset under construction.
        builder: RdfDatasetBuilder,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                builder: RdfDatasetBuilder::new(),
            }
        }

        /// `example.org/{local}`.
        fn iri(&mut self, local: &str) -> TermId {
            self.builder.intern_iri(&format!("{EX}{local}"))
        }

        /// A reserved-vocabulary IRI, verbatim.
        fn vocab(&mut self, iri: &str) -> TermId {
            self.builder.intern_iri(iri)
        }

        /// A default-scope blank node.
        fn blank(&mut self, label: &str) -> TermId {
            self.builder.intern_blank(label, BlankScope::DEFAULT)
        }

        /// A non-negative-integer literal, for a cardinality restriction.
        fn count(&mut self, lexical: &str) -> TermId {
            crate::interner::intern_into(
                &mut self.builder,
                &TermValue::typed_literal(lexical, XSD_NONNEGATIVEINTEGER),
            )
        }

        fn push(&mut self, s: TermId, p: TermId, o: TermId) {
            self.builder.push_quad(s, p, o, None);
        }

        /// Declare `local` an `owl:Class`.
        fn class(&mut self, local: &str) -> TermId {
            let c = self.iri(local);
            let ty = self.vocab(RDF_TYPE);
            let class = self.vocab(OWL_CLASS);
            self.push(c, ty, class);
            c
        }

        /// Declare `local` an `owl:ObjectProperty`.
        fn property(&mut self, local: &str) -> TermId {
            let p = self.iri(local);
            let ty = self.vocab(RDF_TYPE);
            let object_property = self.vocab(OWL_OBJECTPROPERTY);
            self.push(p, ty, object_property);
            p
        }

        /// An RDF list of `items`, returning its head.
        fn list(&mut self, label: &str, items: &[TermId]) -> TermId {
            let first = self.vocab(RDF_FIRST);
            let rest = self.vocab(RDF_REST);
            let nil = self.vocab(RDF_NIL);
            let mut tail = nil;
            for (index, &item) in items.iter().enumerate().rev() {
                let cell = self.blank(&format!("{label}{index}"));
                self.push(cell, first, item);
                self.push(cell, rest, tail);
                tail = cell;
            }
            tail
        }

        fn freeze(self) -> Arc<RdfDataset> {
            self.builder.freeze().expect("freeze")
        }
    }

    /// A subclass chain `C0 ⊑ C1 ⊑ C2 ⊑ C3` with one instance — the shape a taxonomy is.
    fn chain() -> Arc<RdfDataset> {
        let mut f = Fixture::new();
        let sub_class = f.vocab(RDFS_SUBCLASSOF);
        let ty = f.vocab(RDF_TYPE);
        let classes: Vec<TermId> = (0..4).map(|i| f.class(&format!("C{i}"))).collect();
        for window in classes.windows(2) {
            f.push(window[0], sub_class, window[1]);
        }
        let x = f.iri("x");
        f.push(x, ty, classes[0]);
        f.freeze()
    }

    /// `A ≡ B`, plus an `Empty` forced into `⊥` by a complement — the equivalence and
    /// unsatisfiability shapes, with `¬A` in a superclass position.
    fn equivalence_and_complement() -> Arc<RdfDataset> {
        let mut f = Fixture::new();
        let a = f.class("A");
        let b = f.class("B");
        let empty = f.class("Empty");
        let equivalent = f.vocab(OWL_EQUIVALENTCLASS);
        let sub_class = f.vocab(RDFS_SUBCLASSOF);
        let complement = f.vocab(OWL_COMPLEMENTOF);
        f.push(a, equivalent, b);
        let not_a = f.blank("notA");
        f.push(not_a, complement, a);
        f.push(empty, sub_class, a);
        f.push(empty, sub_class, not_a);
        f.freeze()
    }

    /// `A owl:disjointWith B`, with a class under both.
    fn disjointness() -> Arc<RdfDataset> {
        let mut f = Fixture::new();
        let a = f.class("A");
        let b = f.class("B");
        let both = f.class("Both");
        let disjoint = f.vocab(OWL_DISJOINTWITH);
        let sub_class = f.vocab(RDFS_SUBCLASSOF);
        f.push(a, disjoint, b);
        f.push(both, sub_class, a);
        f.push(both, sub_class, b);
        f.freeze()
    }

    /// `Father ≡ Male ⊓ Parent` — the conjunction shape, in both polarities.
    fn intersection() -> Arc<RdfDataset> {
        let mut f = Fixture::new();
        let male = f.class("Male");
        let parent = f.class("Parent");
        let father = f.class("Father");
        let person = f.class("Person");
        let equivalent = f.vocab(OWL_EQUIVALENTCLASS);
        let intersection_of = f.vocab(OWL_INTERSECTIONOF);
        let sub_class = f.vocab(RDFS_SUBCLASSOF);
        let conjunction = f.blank("maleParent");
        let items = f.list("mp", &[male, parent]);
        f.push(conjunction, intersection_of, items);
        f.push(father, equivalent, conjunction);
        f.push(male, sub_class, person);
        f.push(parent, sub_class, person);
        f.freeze()
    }

    /// `Parent ≡ ∃hasChild.Person` with a sub-property and a transitive super-property —
    /// the existential, role-hierarchy and role-composition shapes at once.
    fn existential_with_roles() -> Arc<RdfDataset> {
        let mut f = Fixture::new();
        let person = f.class("Person");
        let parent = f.class("Parent");
        let ancestor_of = f.property("ancestorOf");
        let has_child = f.property("hasChild");
        let ty = f.vocab(RDF_TYPE);
        let transitive = f.vocab(OWL_TRANSITIVEPROPERTY);
        let sub_property = f.vocab(RDFS_SUBPROPERTYOF);
        let equivalent = f.vocab(OWL_EQUIVALENTCLASS);
        let restriction = f.vocab(OWL_RESTRICTION);
        let on_property = f.vocab(OWL_ONPROPERTY);
        let some_values = f.vocab(OWL_SOMEVALUESFROM);
        f.push(ancestor_of, ty, transitive);
        f.push(has_child, sub_property, ancestor_of);
        let some = f.blank("someChild");
        f.push(some, ty, restriction);
        f.push(some, on_property, has_child);
        f.push(some, some_values, person);
        f.push(parent, equivalent, some);
        f.freeze()
    }

    /// `owns rdfs:domain Agent` with `Owner ≡ ∃owns.Thing` — a domain axiom, which the
    /// reverse mapping reads as `∃owns.⊤ ⊑ Agent` and which is therefore ordinary `EL`.
    fn domain() -> Arc<RdfDataset> {
        let mut f = Fixture::new();
        let agent = f.class("Agent");
        let owner = f.class("Owner");
        let owns = f.property("owns");
        let ty = f.vocab(RDF_TYPE);
        let domain = f.vocab(RDFS_DOMAIN);
        let equivalent = f.vocab(OWL_EQUIVALENTCLASS);
        let restriction = f.vocab(OWL_RESTRICTION);
        let on_property = f.vocab(OWL_ONPROPERTY);
        let some_values = f.vocab(OWL_SOMEVALUESFROM);
        let thing = f.vocab(OWL_THING);
        f.push(owns, domain, agent);
        let some = f.blank("someOwns");
        f.push(some, ty, restriction);
        f.push(some, on_property, owns);
        f.push(some, some_values, thing);
        f.push(owner, equivalent, some);
        f.freeze()
    }

    /// `owns rdfs:range Asset` — a range axiom, which the reverse mapping reads as
    /// `⊤ ⊑ ∀owns.Asset` and which is therefore a universal restriction.
    fn range() -> Arc<RdfDataset> {
        let mut f = Fixture::new();
        let asset = f.class("Asset");
        let owns = f.property("owns");
        let range = f.vocab(RDFS_RANGE);
        f.push(owns, range, asset);
        f.freeze()
    }

    /// `Either ≡ A ⊔ B` — a disjunction, which the calculus derives only half of.
    fn union() -> Arc<RdfDataset> {
        let mut f = Fixture::new();
        let a = f.class("A");
        let b = f.class("B");
        let either = f.class("Either");
        let equivalent = f.vocab(OWL_EQUIVALENTCLASS);
        let union_of = f.vocab(OWL_UNIONOF);
        let disjunction = f.blank("aOrB");
        let items = f.list("ab", &[a, b]);
        f.push(disjunction, union_of, items);
        f.push(either, equivalent, disjunction);
        f.freeze()
    }

    /// `A ⊑ ∀p.B` — a universal restriction, which no one-context-per-concept saturation
    /// may propagate.
    fn universal() -> Arc<RdfDataset> {
        let mut f = Fixture::new();
        let a = f.class("A");
        let b = f.class("B");
        let p = f.property("p");
        let ty = f.vocab(RDF_TYPE);
        let restriction = f.vocab(OWL_RESTRICTION);
        let on_property = f.vocab(OWL_ONPROPERTY);
        let all_values = f.vocab(OWL_ALLVALUESFROM);
        let sub_class = f.vocab(RDFS_SUBCLASSOF);
        let all = f.blank("allPB");
        f.push(all, ty, restriction);
        f.push(all, on_property, p);
        f.push(all, all_values, b);
        f.push(a, sub_class, all);
        f.freeze()
    }

    /// `Only ≡ {alice}` with `alice : Female` — the ABox-driven subsumption a TBox-only
    /// saturation cannot see, and the reason a nominal leaves the fragment.
    fn nominal() -> Arc<RdfDataset> {
        let mut f = Fixture::new();
        let female = f.class("Female");
        let only = f.class("Only");
        let alice = f.iri("alice");
        let ty = f.vocab(RDF_TYPE);
        let equivalent = f.vocab(OWL_EQUIVALENTCLASS);
        let one_of = f.vocab(OWL_ONEOF);
        let just_alice = f.blank("justAlice");
        let items = f.list("ja", &[alice]);
        f.push(just_alice, one_of, items);
        f.push(only, equivalent, just_alice);
        f.push(alice, ty, female);
        f.freeze()
    }

    /// `A ⊑ ≤1 p.⊤` — a cardinality restriction.
    fn max_cardinality() -> Arc<RdfDataset> {
        let mut f = Fixture::new();
        let a = f.class("A");
        let p = f.property("p");
        let ty = f.vocab(RDF_TYPE);
        let restriction = f.vocab(OWL_RESTRICTION);
        let on_property = f.vocab(OWL_ONPROPERTY);
        let max_cardinality = f.vocab(OWL_MAXCARDINALITY);
        let sub_class = f.vocab(RDFS_SUBCLASSOF);
        let one = f.count("1");
        let at_most = f.blank("atMostOne");
        f.push(at_most, ty, restriction);
        f.push(at_most, on_property, p);
        f.push(at_most, max_cardinality, one);
        f.push(a, sub_class, at_most);
        f.freeze()
    }

    /// `p owl:inverseOf q` with an existential over each — an inverse role.
    fn inverse_role() -> Arc<RdfDataset> {
        let mut f = Fixture::new();
        let a = f.class("A");
        let b = f.class("B");
        let p = f.property("p");
        let q = f.property("q");
        let ty = f.vocab(RDF_TYPE);
        let inverse_of = f.vocab(OWL_INVERSEOF);
        let restriction = f.vocab(OWL_RESTRICTION);
        let on_property = f.vocab(OWL_ONPROPERTY);
        let some_values = f.vocab(OWL_SOMEVALUESFROM);
        let sub_class = f.vocab(RDFS_SUBCLASSOF);
        f.push(p, inverse_of, q);
        let some = f.blank("somePB");
        f.push(some, ty, restriction);
        f.push(some, on_property, p);
        f.push(some, some_values, b);
        f.push(a, sub_class, some);
        f.freeze()
    }

    /// The differential corpus: `(name, dataset, expected fragment membership)`.
    ///
    /// The expectation is asserted, not merely recorded, because the fragment predicate is
    /// the thing that licenses reporting `Verdict::False` without a refutation — a silent
    /// drift to "always outside" would make the classifier correct and pointless, and a
    /// drift to "always inside" would make it wrong.
    fn corpus() -> Vec<(&'static str, Arc<RdfDataset>, bool)> {
        vec![
            ("chain", chain(), true),
            (
                "equivalence_and_complement",
                equivalence_and_complement(),
                true,
            ),
            ("disjointness", disjointness(), true),
            ("intersection", intersection(), true),
            ("existential_with_roles", existential_with_roles(), true),
            ("domain", domain(), true),
            ("range", range(), false),
            ("union", union(), false),
            ("universal", universal(), false),
            ("nominal", nominal(), false),
            ("max_cardinality", max_cardinality(), false),
            ("inverse_role", inverse_role(), false),
        ]
    }

    /// TWO IMPLEMENTATIONS OF ONE CONTRACT, HELD AGAINST EACH OTHER.
    ///
    /// The saturation and the per-pair tableau decide the same relation by completely
    /// different means — a least fixpoint over a rule table versus one refutation per ordered
    /// pair — so agreeing on every pair of every fixture is evidence neither a soundness bug
    /// (the saturation deriving what the tableau refutes) nor a completeness bug (the
    /// saturation reporting `False` where the tableau finds a subsumption) is present.
    ///
    /// This is the reason [`Subsumptions::decide_by_tableau`] is kept alive after ceasing to
    /// be the production path.
    #[test]
    fn the_saturation_agrees_with_the_tableau_pair_by_pair() {
        for (name, dataset, _) in corpus() {
            let reasoner = Reasoner::new(&dataset).expect("reverse-map");
            let (mut derived_session, usable) = reasoner.open().expect("consistent");
            assert!(usable, "{name}: the fixture must be decidable in budget");
            let derived = Subsumptions::decide(&mut derived_session, &reasoner.classes);
            let (mut refuted_session, _) = reasoner.open().expect("consistent");
            let refuted = Subsumptions::decide_by_tableau(&mut refuted_session, &reasoner.classes);
            assert_eq!(
                derived.verdicts, refuted.verdicts,
                "{name}: the saturation and the tableau disagree about the subsumption matrix"
            );
        }
    }

    /// THE TWO DECISION CORES AGREE ON EVERY PAIR OF EVERY REVERSE-MAPPED FIXTURE.
    ///
    /// The hypertableau (`owl_dl::hyper`) is the production core; the concept-tree tableau
    /// (`owl_dl::tableau`) is kept as its reference. `owl_dl::oracle` compares them over 9,200
    /// GENERATED knowledge bases, but those are assembled axiom-by-axiom in memory; this
    /// corpus reaches the core through the OWL-2-RDF reverse mapping instead, so it is where a
    /// clause derived from a parsed class expression — a `owl:unionOf`, an
    /// `owl:allValuesFrom`, an `owl:oneOf`, an `owl:maxCardinality`, an `owl:inverseOf` — is
    /// held against the calculus that reads that expression's structure directly. Six of the
    /// twelve fixtures are outside the classifying saturation's fragment, which is exactly the
    /// non-Horn residue the two calculi handle differently.
    ///
    /// ZERO divergence is the contract. A disagreement is a soundness or completeness bug in
    /// one of the two, not a difference to record.
    #[test]
    fn the_two_decision_cores_agree_on_every_subsumption_of_every_fixture() {
        let mut compared = 0usize;
        for (name, dataset, _) in corpus() {
            let reasoner = Reasoner::new(&dataset).expect("reverse-map");
            assert_eq!(
                reasoner.kb.is_consistent().expect("decided"),
                reasoner
                    .kb
                    .is_consistent_by_concept_tree()
                    .expect("decided"),
                "{name}: the two decision cores disagree about consistency"
            );
            for &(_, sub) in &reasoner.classes {
                for &(_, sup) in &reasoner.classes {
                    let hyper = reasoner.kb.entails_subclass(sub, sup).expect("decided");
                    let concept_tree = reasoner
                        .kb
                        .entails_subclass_by_concept_tree(sub, sup)
                        .expect("decided");
                    assert_eq!(
                        hyper, concept_tree,
                        "{name}: the two decision cores disagree about {sub} ⊑ {sup}: \
                         hypertableau {hyper}, concept-tree tableau {concept_tree}"
                    );
                    compared += 1;
                }
            }
        }
        // The population, printed rather than asserted at a magic number: it is a function of
        // the corpus, and the assertion that matters is the zero divergence above.
        eprintln!("{compared} ordered class pairs decided by BOTH cores, zero divergence");
        assert!(compared > 100, "the differential compared almost nothing");
    }

    /// …and the same agreement holds at the seam the query-directed augmentation reads,
    /// which consults the saturation directly rather than through [`Subsumptions`].
    #[test]
    fn the_injected_relation_agrees_with_the_tableau() {
        for (name, dataset, _) in corpus() {
            let reasoner = Reasoner::new(&dataset).expect("reverse-map");
            let seeds: Vec<u32> = reasoner.classes.iter().map(|&(_, c)| c).collect();
            let taxonomy = saturate(&reasoner.kb, &seeds);
            let complete = taxonomy.is_complete();
            for &(_, sub) in &reasoner.classes {
                for &(_, sup) in &reasoner.classes {
                    let entailed = reasoner
                        .kb
                        .entails_subclass(sub, sup)
                        .expect("the fixture is satisfiable");
                    let derived = taxonomy.derives(sub, sup);
                    assert!(
                        !derived || entailed,
                        "{name}: derived {sub} ⊑ {sup}, which the tableau refutes"
                    );
                    if complete {
                        assert_eq!(
                            derived, entailed,
                            "{name}: inside the fragment the derivation IS the relation"
                        );
                    }
                }
            }
        }
    }

    /// The fragment predicate answers what the corpus says it should.
    #[test]
    fn the_fragment_predicate_separates_the_corpus() {
        for (name, dataset, expected) in corpus() {
            let reasoner = Reasoner::new(&dataset).expect("reverse-map");
            let seeds: Vec<u32> = reasoner.classes.iter().map(|&(_, c)| c).collect();
            assert_eq!(
                saturate(&reasoner.kb, &seeds).is_complete(),
                expected,
                "{name}: fragment membership"
            );
        }
    }

    /// THE DECISION COUNT IS THE MEASUREMENT, AND IT DROPS.
    ///
    /// The baseline is not a formula — it is [`Subsumptions::decide_by_tableau`] run over the
    /// same fixture in its own session, so the "before" number is measured by the same
    /// [`DlCertificate::decisions`](crate::reasoner::DlCertificate::decisions) counter as the
    /// "after". Inside the fragment classification spends exactly the one consistency decision
    /// the session opens with and makes no refutation at all; outside it, only the underived
    /// pairs are refuted, which is strictly fewer because the derived ones are not asked.
    #[test]
    fn the_derived_taxonomy_costs_strictly_fewer_decisions_than_the_refuted_one() {
        let none = BTreeSet::new();
        for (name, dataset, in_fragment) in corpus() {
            let reasoner = Reasoner::new(&dataset).expect("reverse-map");
            let (mut baseline_session, _) = reasoner.open().expect("consistent");
            let _ = Subsumptions::decide_by_tableau(&mut baseline_session, &reasoner.classes);
            let before = baseline_session.certificate(&none).decisions();
            let after = reasoner
                .classify()
                .expect("consistent")
                .certificate()
                .decisions();
            eprintln!(
                "{name}: {} classes, decisions before {before}, after {after}",
                reasoner.classes.len()
            );
            if in_fragment {
                assert_eq!(
                    after, 1,
                    "{name}: a derived taxonomy costs the consistency check and nothing else"
                );
            }
            assert!(
                after < before,
                "{name}: {after} refutations is not fewer than the {before} the per-pair \
                 classifier made"
            );
        }
    }

    /// A CLASSIFICATION IS BYTE-IDENTICAL ACROSS RUNS.
    ///
    /// A hundred independent reverse-mappings and classifications of one dataset, rendered
    /// and compared as bytes. A saturation is a fixpoint over hash-free ordered indexes, so
    /// the answer cannot depend on iteration order — and this is what would fail if a
    /// `HashMap` ever reached the path.
    #[test]
    fn classification_is_byte_identical_across_a_hundred_runs() {
        for (name, dataset, _) in corpus() {
            let render = || {
                let reasoner = Reasoner::new(&dataset).expect("reverse-map");
                let answer = reasoner.classify().expect("consistent");
                format!("{:?}{:?}", answer.answer(), answer.certificate())
            };
            let first = render();
            for run in 1..100 {
                assert_eq!(render(), first, "{name}: run {run} differs from run 0");
            }
        }
    }

    /// NO TAXONOMY EDGE IN THE REDUCTION IS IMPLIED BY ANOTHER.
    ///
    /// The transitive-reduction property, asserted rather than claimed: for every direct
    /// edge `A ⊑ B` there is no third class strictly between them. "Strictly" is what makes
    /// the check right in the presence of equivalences — a class equivalent to either
    /// endpoint sits at the same level and interposes nothing.
    #[test]
    fn no_direct_taxonomy_edge_is_implied_by_another() {
        for (name, dataset, _) in corpus() {
            let reasoner = Reasoner::new(&dataset).expect("reverse-map");
            let hierarchy: ClassHierarchy = reasoner.classify().expect("consistent").into_answer();
            let key = |term: &TermValue| super::term_key(term);
            let closure: BTreeSet<((u8, String), (u8, String))> = hierarchy
                .subsumptions()
                .iter()
                .map(|(sub, sup)| (key(sub), key(sup)))
                .collect();
            let below = |sub: &(u8, String), sup: &(u8, String)| {
                closure.contains(&(sub.clone(), sup.clone()))
            };
            let strictly_below =
                |sub: &(u8, String), sup: &(u8, String)| below(sub, sup) && !below(sup, sub);
            let signature: Vec<(u8, String)> = reasoner.signature().iter().map(key).collect();
            for (sub, sup) in hierarchy.direct_subsumptions() {
                let (sub, sup) = (key(sub), key(sup));
                assert!(
                    strictly_below(&sub, &sup),
                    "{name}: a direct edge must be a strict subsumption"
                );
                let interposed = signature
                    .iter()
                    .find(|middle| strictly_below(&sub, middle) && strictly_below(middle, &sup));
                assert!(
                    interposed.is_none(),
                    "{name}: {sub:?} ⊑ {sup:?} is implied through {:?}",
                    interposed.expect("checked above")
                );
            }
        }
    }
}
