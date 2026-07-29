// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The native OWL-Direct (Description-Logic) reasoner core.
//!
//! Three layers compose here: [`concept`] is the DL syntax and its structural
//! interner; [`parser`] reverse-maps an [`RdfDataset`] into a [`Kb`] (TBox, RBox,
//! ABox, plus anonymous class expressions); [`tableau`] is the `ALCOIQ` completion
//! procedure that decides consistency. [`Kb`] ties them together and exposes the
//! internal reasoning seams — [`Kb::is_consistent`], [`Kb::entails_instance`],
//! [`Kb::entails_subclass`], and [`Kb::instances_of`] — which the query-answering layer
//! ([`crate::owl_dl::query`]) drives. Those seams are internal: the public one is
//! [`crate::materialize_dl_reported`], which is where an answer acquires the
//! [`ReasoningReport`](crate::ReasoningReport) naming the constructs this layer could not
//! fully handle.
//!
//! Every derived answer is deterministic: concept ids are assigned in parse order,
//! all working sets are `BTreeSet`/`BTreeMap` or insertion-ordered `Vec`s, and the
//! tableau branches in a fixed order — nothing is ever read out of a `HashMap`.
//!
//! The reasoning entry points are exercised by the module's own tests and by the
//! query-answering layer ([`crate::owl_dl::query`]), which wires them into the public
//! [`crate::materialize_dl_reported`] seam.

use std::collections::{BTreeMap, BTreeSet};

use purrdf_core::RdfDataset;

use crate::EntailError;
use crate::interner::Interner;
use crate::owl_dl::concept::Concept;
use crate::owl_dl::concept::ConceptTable;
use crate::report::Construct;

pub(crate) mod concept;
pub(crate) mod constructs;
pub(crate) mod parser;
pub(crate) mod query;
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
    /// The internalized TBox: meta-concept ids `nnf(¬sub ⊔ sup)`, one per
    /// non-absorbable GCI (a GCI whose left side is not a single named class).
    pub(crate) meta: Vec<u32>,
    /// The **absorbed** TBox: a named-class concept id `A` → the super-concept ids it
    /// entails (`A ⊑ D`). A lazy-unfolding rule adds each `D` to any node labelled `A`
    /// rather than branching a `¬A ⊔ D` disjunction on *every* node — the standard
    /// absorption optimization that keeps a many-axiom TBox from exploding.
    pub(crate) unfold: BTreeMap<u32, Vec<u32>>,
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
    /// The constructs this knowledge base could not fully handle, in `Construct` order.
    pub(crate) boundaries: BTreeSet<Construct>,
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
            unfold: BTreeMap::new(),
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
            boundaries: BTreeSet::new(),
        }
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
        let mut kb = parser::build(ds)?;
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

    /// Record a general concept inclusion `sub ⊑ sup`, absorbing it into the lazy
    /// [`Kb::unfold`] index when its left side is a single named class, else
    /// internalizing it as a meta-concept disjunction. Used by the tableau unit tests
    /// (the RDF build path records inclusions inline in [`parser`]).
    #[cfg(test)]
    pub(crate) fn push_gci(&mut self, sub: Concept, sup: Concept) {
        let sub_id = self.table.intern(sub.clone());
        let sup_id = self.table.intern(sup.clone());
        self.tbox.push((sub_id, sup_id));
        if matches!(sub, Concept::Named(_)) {
            self.unfold.entry(sub_id).or_default().push(sup_id);
        } else {
            let meta = Concept::Or(vec![Concept::Not(Box::new(sub)), sup]);
            let meta_id = self.table.intern(meta);
            self.meta.push(meta_id);
        }
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

    /// Finalize the concept table (populate the negation cache). Call once after all
    /// axioms and assertions are in place.
    pub(crate) fn finalize(&mut self) {
        self.table.finalize();
    }

    /// Whether the knowledge base (TBox + ABox) is consistent.
    ///
    /// # Errors
    ///
    /// [`EntailError::Build`] if the tableau exceeds its step cap.
    pub(crate) fn is_consistent(&self) -> Result<bool, EntailError> {
        tableau::consistent(self, &tableau::Assumptions::of_kb())
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
        let consistent = tableau::consistent(
            self,
            &tableau::Assumptions {
                types: &[(individual, neg)],
                ..tableau::Assumptions::of_kb()
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
        let consistent = tableau::consistent(
            self,
            &tableau::Assumptions {
                fresh_types: &[sub_id, neg_sup],
                ..tableau::Assumptions::of_kb()
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

    /// Build the `simple.ttl` fixture as a dataset (default graph).
    fn simple_dataset() -> std::sync::Arc<RdfDataset> {
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
        assert!(
            !report.overclaims(),
            "a boundary beside `exact` is an overclaim"
        );
        assert_eq!(
            report.completeness(),
            &Completeness::ExactWithinBoundaries,
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
}
