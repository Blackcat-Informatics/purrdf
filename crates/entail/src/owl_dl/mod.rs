// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The native OWL-Direct (Description-Logic) reasoner core.
//!
//! Three layers compose here: [`concept`] is the DL syntax and its structural
//! interner; [`parser`] reverse-maps an [`RdfDataset`] into a [`Kb`] (TBox, RBox,
//! ABox, plus anonymous class expressions); [`tableau`] is the `ALCOIQ` completion
//! procedure that decides consistency. [`Kb`] ties them together and exposes the
//! internal reasoning seams — [`Kb::is_consistent`], [`Kb::entails_instance`],
//! [`Kb::entails_subclass`], and [`Kb::instances_of`] — used by the query-answering
//! layer wired up in a subsequent task. There is no public `materialize` seam yet.
//!
//! Every derived answer is deterministic: concept ids are assigned in parse order,
//! all working sets are `BTreeSet`/`BTreeMap` or insertion-ordered `Vec`s, and the
//! tableau branches in a fixed order — nothing is ever read out of a `HashMap`.
//!
//! The reasoning entry points are exercised by the module's own tests and by the
//! query-answering layer ([`crate::owl_dl::query`]), which wires them into the public
//! [`crate::materialize_dl`] seam.

use std::collections::{BTreeMap, BTreeSet};

use purrdf_core::RdfDataset;

use crate::EntailError;
use crate::interner::Interner;
#[cfg(test)]
use crate::owl_dl::concept::Concept;
use crate::owl_dl::concept::ConceptTable;

pub(crate) mod concept;
pub(crate) mod parser;
pub(crate) mod query;
pub(crate) mod tableau;

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
    /// All named individual term ids.
    pub(crate) individuals: BTreeSet<u32>,
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
            individuals: BTreeSet::new(),
        }
    }

    /// Reverse-map an [`RdfDataset`]'s default graph into a knowledge base.
    ///
    /// # Errors
    ///
    /// [`EntailError::Parse`] on a malformed OWL class-expression graph.
    pub(crate) fn from_dataset(ds: &RdfDataset) -> Result<Self, EntailError> {
        parser::build(ds)
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
        tableau::consistent(self, true, &[], &[])
    }

    /// Whether `individual : concept_id` is entailed — i.e. the knowledge base with
    /// `individual : ¬concept` added is inconsistent.
    ///
    /// # Errors
    ///
    /// [`EntailError::Inconsistent`] if the base knowledge base is already
    /// inconsistent; [`EntailError::Build`] on step-cap exhaustion.
    pub(crate) fn entails_instance(
        &self,
        individual: u32,
        concept_id: u32,
    ) -> Result<bool, EntailError> {
        if !self.is_consistent()? {
            return Err(EntailError::Inconsistent);
        }
        let neg = self.table.negate(concept_id);
        let consistent = tableau::consistent(self, true, &[(individual, neg)], &[])?;
        Ok(!consistent)
    }

    /// Whether `sub_id ⊑ sup_id` holds w.r.t. the TBox — i.e. `sub ⊓ ¬sup` is
    /// unsatisfiable. Yields `⊥ ⊑ X` and reflexive `X ⊑ X`.
    ///
    /// # Errors
    ///
    /// [`EntailError::Inconsistent`] if the base knowledge base is already
    /// inconsistent; [`EntailError::Build`] on step-cap exhaustion.
    pub(crate) fn entails_subclass(&self, sub_id: u32, sup_id: u32) -> Result<bool, EntailError> {
        if !self.is_consistent()? {
            return Err(EntailError::Inconsistent);
        }
        let neg_sup = self.table.negate(sup_id);
        let consistent = tableau::consistent(self, false, &[], &[sub_id, neg_sup])?;
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
