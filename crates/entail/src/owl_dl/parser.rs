// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The OWL-2-RDF reverse-mapping parser: an [`RdfDataset`] default graph → a DL
//! knowledge base ([`Kb`]).
//!
//! The mapping follows the OWL 2 "Mapping to RDF Graphs" specification, read in
//! reverse: `owl:Restriction` blank nodes become qualified restrictions, the RDF-list
//! collection vocabulary (`rdf:first`/`rdf:rest`/`rdf:nil`) is walked to recover
//! `owl:intersectionOf`/`unionOf`/`oneOf` operands, and the axiom vocabulary
//! (`rdfs:subClassOf`, `owl:equivalentClass`, `owl:disjointWith`, `rdfs:domain`/`range`,
//! `owl:inverseOf`, `owl:FunctionalProperty`, …) becomes TBox/RBox axioms. Class
//! expressions are interned to [`Concept`]s and memoized by their RDF node id.
//!
//! The class-expression extraction is factored into [`CeExtractor`] — a reusable view
//! over an interned `subject → predicate → objects` index plus the shared [`Vocab`] and
//! [`Interner`]. The knowledge-base build uses it over the dataset's own triples; the
//! query-answering layer ([`crate::owl_dl::query`]) reuses the very same extractor over
//! a query's ground class-expression sub-graph, so there is one class-expression parser.
//!
//! Every extraction is deterministic (all indices are `BTreeMap`/insertion-ordered
//! `Vec`s) and any malformed class-expression graph is a hard [`EntailError::Parse`],
//! never a silent skip.

use std::collections::{BTreeMap, BTreeSet};

use purrdf_core::{RdfDataset, TermValue};

use crate::interner::Interner;
use crate::owl_dl::concept::{Concept, ConceptTable, Role};
use crate::owl_dl::Kb;
use crate::vocab::{
    OWL_ALLVALUESFROM, OWL_CARDINALITY, OWL_CLASS, OWL_COMPLEMENTOF, OWL_DATATYPEPROPERTY,
    OWL_DISJOINTWITH, OWL_EQUIVALENTCLASS, OWL_EQUIVALENTPROPERTY, OWL_FUNCTIONALPROPERTY,
    OWL_HASVALUE, OWL_INTERSECTIONOF, OWL_INVERSEOF, OWL_MAXCARDINALITY,
    OWL_MAXQUALIFIEDCARDINALITY, OWL_MINCARDINALITY, OWL_MINQUALIFIEDCARDINALITY,
    OWL_NAMEDINDIVIDUAL, OWL_NOTHING, OWL_OBJECTPROPERTY, OWL_ONCLASS, OWL_ONEOF, OWL_ONPROPERTY,
    OWL_ONTOLOGY, OWL_QUALIFIEDCARDINALITY, OWL_RESTRICTION, OWL_SAMEAS, OWL_SOMEVALUESFROM,
    OWL_SYMMETRICPROPERTY, OWL_THING, OWL_TRANSITIVEPROPERTY, OWL_UNIONOF, RDFS_CLASS, RDFS_DOMAIN,
    RDFS_RANGE, RDFS_SUBCLASSOF, RDFS_SUBPROPERTYOF, RDF_FIRST, RDF_NIL, RDF_PROPERTY, RDF_REST,
    RDF_TYPE,
};
use crate::EntailError;

/// The interned vocabulary term ids the reverse mapping keys on. Fields are
/// `pub(crate)` so the query-answering layer can build the same class-expression
/// view and recognize query class expressions using the identical ids.
pub(crate) struct Vocab {
    pub(crate) ty: u32,
    pub(crate) thing: u32,
    pub(crate) nothing: u32,
    pub(crate) class: u32,
    pub(crate) restriction: u32,
    pub(crate) on_property: u32,
    pub(crate) some_values: u32,
    pub(crate) all_values: u32,
    pub(crate) has_value: u32,
    pub(crate) intersection: u32,
    pub(crate) union: u32,
    pub(crate) complement: u32,
    pub(crate) one_of: u32,
    pub(crate) min_card: u32,
    pub(crate) max_card: u32,
    pub(crate) card: u32,
    pub(crate) min_qcard: u32,
    pub(crate) max_qcard: u32,
    pub(crate) qcard: u32,
    pub(crate) on_class: u32,
    pub(crate) sub_class: u32,
    pub(crate) equiv_class: u32,
    pub(crate) disjoint: u32,
    pub(crate) domain: u32,
    pub(crate) range: u32,
    pub(crate) inverse_of: u32,
    pub(crate) equiv_prop: u32,
    pub(crate) sub_prop: u32,
    pub(crate) functional: u32,
    pub(crate) same_as: u32,
    pub(crate) first: u32,
    pub(crate) rest: u32,
    pub(crate) nil: u32,
    pub(crate) named_individual: u32,
    /// Class/property-typing objects that mark structure, not an instance assertion.
    pub(crate) structural_types: BTreeSet<u32>,
    /// Predicates consumed by class-expression / list / axiom extraction.
    pub(crate) structural_preds: BTreeSet<u32>,
}

impl Vocab {
    /// Intern (idempotently) the vocabulary IRIs into `i`, returning their ids.
    pub(crate) fn intern(i: &mut Interner) -> Self {
        let ty = i.intern_iri(RDF_TYPE);
        let restriction = i.intern_iri(OWL_RESTRICTION);
        let class = i.intern_iri(OWL_CLASS);
        let object_prop = i.intern_iri(OWL_OBJECTPROPERTY);
        let datatype_prop = i.intern_iri(OWL_DATATYPEPROPERTY);
        let functional = i.intern_iri(OWL_FUNCTIONALPROPERTY);
        let ontology = i.intern_iri(OWL_ONTOLOGY);
        let named_individual = i.intern_iri(OWL_NAMEDINDIVIDUAL);
        let symmetric = i.intern_iri(OWL_SYMMETRICPROPERTY);
        let transitive = i.intern_iri(OWL_TRANSITIVEPROPERTY);
        let rdf_property = i.intern_iri(RDF_PROPERTY);
        let rdfs_class = i.intern_iri(RDFS_CLASS);

        let on_property = i.intern_iri(OWL_ONPROPERTY);
        let some_values = i.intern_iri(OWL_SOMEVALUESFROM);
        let all_values = i.intern_iri(OWL_ALLVALUESFROM);
        let has_value = i.intern_iri(OWL_HASVALUE);
        let intersection = i.intern_iri(OWL_INTERSECTIONOF);
        let union = i.intern_iri(OWL_UNIONOF);
        let complement = i.intern_iri(OWL_COMPLEMENTOF);
        let one_of = i.intern_iri(OWL_ONEOF);
        let min_card = i.intern_iri(OWL_MINCARDINALITY);
        let max_card = i.intern_iri(OWL_MAXCARDINALITY);
        let card = i.intern_iri(OWL_CARDINALITY);
        let min_qcard = i.intern_iri(OWL_MINQUALIFIEDCARDINALITY);
        let max_qcard = i.intern_iri(OWL_MAXQUALIFIEDCARDINALITY);
        let qcard = i.intern_iri(OWL_QUALIFIEDCARDINALITY);
        let on_class = i.intern_iri(OWL_ONCLASS);
        let first = i.intern_iri(RDF_FIRST);
        let rest = i.intern_iri(RDF_REST);

        let sub_class = i.intern_iri(RDFS_SUBCLASSOF);
        let equiv_class = i.intern_iri(OWL_EQUIVALENTCLASS);
        let disjoint = i.intern_iri(OWL_DISJOINTWITH);
        let domain = i.intern_iri(RDFS_DOMAIN);
        let range = i.intern_iri(RDFS_RANGE);
        let inverse_of = i.intern_iri(OWL_INVERSEOF);
        let equiv_prop = i.intern_iri(OWL_EQUIVALENTPROPERTY);
        let sub_prop = i.intern_iri(RDFS_SUBPROPERTYOF);
        let same_as = i.intern_iri(OWL_SAMEAS);

        let mut structural_types = BTreeSet::new();
        for t in [
            class,
            restriction,
            object_prop,
            datatype_prop,
            functional,
            ontology,
            named_individual,
            symmetric,
            transitive,
            rdf_property,
            rdfs_class,
        ] {
            structural_types.insert(t);
        }
        let mut structural_preds = BTreeSet::new();
        for p in [
            on_property,
            some_values,
            all_values,
            has_value,
            intersection,
            union,
            complement,
            one_of,
            min_card,
            max_card,
            card,
            min_qcard,
            max_qcard,
            qcard,
            on_class,
            first,
            rest,
        ] {
            structural_preds.insert(p);
        }

        Self {
            ty,
            thing: i.intern_iri(OWL_THING),
            nothing: i.intern_iri(OWL_NOTHING),
            class,
            restriction,
            on_property,
            some_values,
            all_values,
            has_value,
            intersection,
            union,
            complement,
            one_of,
            min_card,
            max_card,
            card,
            min_qcard,
            max_qcard,
            qcard,
            on_class,
            sub_class,
            equiv_class,
            disjoint,
            domain,
            range,
            inverse_of,
            equiv_prop,
            sub_prop,
            functional,
            same_as,
            first,
            rest,
            nil: i.intern_iri(RDF_NIL),
            named_individual,
            structural_types,
            structural_preds,
        }
    }
}

/// A `subject → predicate → objects` index over interned term ids (insertion-ordered
/// objects; deterministic lookups). Shared by the knowledge-base build and the query
/// class-expression view.
pub(crate) type TripleIndex = BTreeMap<u32, BTreeMap<u32, Vec<u32>>>;

/// Insert `(s, p, o)` into `index`.
pub(crate) fn index_insert(index: &mut TripleIndex, s: u32, p: u32, o: u32) {
    index.entry(s).or_default().entry(p).or_default().push(o);
}

/// A reusable class-expression extractor: it decodes the OWL-2-RDF class-expression
/// vocabulary rooted at an RDF node into a [`Concept`], memoizing per node id.
///
/// It borrows an interned [`TripleIndex`], the [`Interner`] (to distinguish blank
/// inverse-role nodes and parse cardinality literals), and the shared [`Vocab`]; the
/// concept-interning [`ConceptTable`] is *not* needed here (extraction returns a
/// [`Concept`] tree; interning is the caller's concern).
pub(crate) struct CeExtractor<'a> {
    index: &'a TripleIndex,
    interner: &'a Interner,
    v: &'a Vocab,
    /// Node id → its class expression (memoized).
    expr_cache: BTreeMap<u32, Concept>,
    /// Nodes on the current recursion stack (cycle guard).
    in_progress: BTreeSet<u32>,
}

impl<'a> CeExtractor<'a> {
    /// Build an extractor over `index`, resolving terms through `interner` and keying
    /// on `v`.
    pub(crate) fn new(index: &'a TripleIndex, interner: &'a Interner, v: &'a Vocab) -> Self {
        Self {
            index,
            interner,
            v,
            expr_cache: BTreeMap::new(),
            in_progress: BTreeSet::new(),
        }
    }

    /// Whether `node` denotes a (compound / anonymous) class expression — i.e. it
    /// carries one of the class-expression-defining predicates or is typed
    /// `owl:Restriction`. A plain named class returns `false`.
    pub(crate) fn is_class_expression(&self, node: u32) -> bool {
        for p in [
            self.v.intersection,
            self.v.union,
            self.v.complement,
            self.v.one_of,
            self.v.on_property,
        ] {
            if self.get(node, p).is_some() {
                return true;
            }
        }
        self.is_typed(node, self.v.restriction)
    }

    /// The class expression denoted by RDF node `node` (memoized).
    ///
    /// # Errors
    ///
    /// [`EntailError::Parse`] on a malformed class-expression graph.
    pub(crate) fn expr(&mut self, node: u32) -> Result<Concept, EntailError> {
        if let Some(c) = self.expr_cache.get(&node) {
            return Ok(c.clone());
        }
        if !self.in_progress.insert(node) {
            return Err(EntailError::Parse("cyclic OWL class expression".to_owned()));
        }
        let c = self.build_expr(node)?;
        self.in_progress.remove(&node);
        self.expr_cache.insert(node, c.clone());
        Ok(c)
    }

    /// Structurally decode `node` into a [`Concept`].
    fn build_expr(&mut self, node: u32) -> Result<Concept, EntailError> {
        if node == self.v.thing {
            return Ok(Concept::Top);
        }
        if node == self.v.nothing {
            return Ok(Concept::Bottom);
        }
        if let Some(head) = self.get(node, self.v.intersection) {
            let items = self.expr_list(head)?;
            return Ok(Concept::And(items));
        }
        if let Some(head) = self.get(node, self.v.union) {
            let items = self.expr_list(head)?;
            return Ok(Concept::Or(items));
        }
        if let Some(inner) = self.get(node, self.v.complement) {
            return Ok(Concept::Not(Box::new(self.expr(inner)?)));
        }
        if let Some(head) = self.get(node, self.v.one_of) {
            let ids = self.node_list(head)?;
            return Ok(one_of(ids));
        }
        if self.get(node, self.v.on_property).is_some() || self.is_typed(node, self.v.restriction) {
            return self.restriction(node);
        }
        // An atomic named (or otherwise opaque) class.
        Ok(Concept::Named(node))
    }

    /// Decode an `owl:Restriction` node.
    fn restriction(&mut self, node: u32) -> Result<Concept, EntailError> {
        let r = self.get(node, self.v.on_property).ok_or_else(|| {
            EntailError::Parse("owl:Restriction without owl:onProperty".to_owned())
        })?;
        let role = self.role_of(r);
        if let Some(c) = self.get(node, self.v.some_values) {
            return Ok(Concept::Some(role, Box::new(self.expr(c)?)));
        }
        if let Some(c) = self.get(node, self.v.all_values) {
            return Ok(Concept::All(role, Box::new(self.expr(c)?)));
        }
        if let Some(a) = self.get(node, self.v.has_value) {
            return Ok(Concept::Some(role, Box::new(Concept::Nominal(vec![a]))));
        }
        if let Some(lit) = self.get(node, self.v.min_qcard) {
            let n = self.card(lit)?;
            let c = self.qualified_class(node)?;
            return Ok(Concept::Min(n, role, Box::new(c)));
        }
        if let Some(lit) = self.get(node, self.v.max_qcard) {
            let n = self.card(lit)?;
            let c = self.qualified_class(node)?;
            return Ok(Concept::Max(n, role, Box::new(c)));
        }
        if let Some(lit) = self.get(node, self.v.qcard) {
            let n = self.card(lit)?;
            let c = self.qualified_class(node)?;
            return Ok(Concept::And(vec![
                Concept::Min(n, role, Box::new(c.clone())),
                Concept::Max(n, role, Box::new(c)),
            ]));
        }
        if let Some(lit) = self.get(node, self.v.min_card) {
            let n = self.card(lit)?;
            return Ok(Concept::Min(n, role, Box::new(Concept::Top)));
        }
        if let Some(lit) = self.get(node, self.v.max_card) {
            let n = self.card(lit)?;
            return Ok(Concept::Max(n, role, Box::new(Concept::Top)));
        }
        if let Some(lit) = self.get(node, self.v.card) {
            let n = self.card(lit)?;
            return Ok(Concept::And(vec![
                Concept::Min(n, role, Box::new(Concept::Top)),
                Concept::Max(n, role, Box::new(Concept::Top)),
            ]));
        }
        Err(EntailError::Parse(
            "owl:Restriction with no recognized constraint".to_owned(),
        ))
    }

    /// The `owl:onClass` filler of a qualified cardinality restriction.
    fn qualified_class(&mut self, node: u32) -> Result<Concept, EntailError> {
        let on_class = self.v.on_class;
        let c = self.get(node, on_class).ok_or_else(|| {
            EntailError::Parse("qualified cardinality without owl:onClass".to_owned())
        })?;
        self.expr(c)
    }

    /// The role denoted by property node `r` (`Inv` for an anonymous inverse).
    fn role_of(&self, r: u32) -> Role {
        if matches!(self.interner.value(r), TermValue::Blank { .. }) {
            if let Some(inv) = self.get(r, self.v.inverse_of) {
                return Role::Inv(inv);
            }
        }
        Role::Named(r)
    }

    /// Parse a cardinality literal (an `xsd:nonNegativeInteger`/`integer`) as `u32`.
    fn card(&self, lit: u32) -> Result<u32, EntailError> {
        match self.interner.value(lit) {
            TermValue::Literal { lexical_form, .. } => {
                lexical_form.trim().parse::<u32>().map_err(|_| {
                    EntailError::Parse(format!("non-integer cardinality literal: {lexical_form:?}"))
                })
            }
            other => Err(EntailError::Parse(format!(
                "cardinality value is not a literal: {other:?}"
            ))),
        }
    }

    /// Walk an RDF list to its member node ids.
    fn node_list(&self, head: u32) -> Result<Vec<u32>, EntailError> {
        let mut out = Vec::new();
        let mut seen = BTreeSet::new();
        let mut cur = head;
        while cur != self.v.nil {
            if !seen.insert(cur) {
                return Err(EntailError::Parse("cyclic RDF list".to_owned()));
            }
            let first = self
                .get(cur, self.v.first)
                .ok_or_else(|| EntailError::Parse("RDF list cell without rdf:first".to_owned()))?;
            out.push(first);
            cur = self
                .get(cur, self.v.rest)
                .ok_or_else(|| EntailError::Parse("RDF list cell without rdf:rest".to_owned()))?;
        }
        Ok(out)
    }

    /// Walk an RDF list of class expressions.
    fn expr_list(&mut self, head: u32) -> Result<Vec<Concept>, EntailError> {
        let ids = self.node_list(head)?;
        ids.into_iter().map(|n| self.expr(n)).collect()
    }

    /// The first object of `(s, p, ·)`, if any.
    fn get(&self, s: u32, p: u32) -> Option<u32> {
        self.index.get(&s)?.get(&p)?.first().copied()
    }

    /// Whether `s rdf:type o` is asserted.
    fn is_typed(&self, s: u32, o: u32) -> bool {
        self.index
            .get(&s)
            .and_then(|m| m.get(&self.v.ty))
            .is_some_and(|os| os.contains(&o))
    }
}

/// Parse `ds`'s default graph into a knowledge base.
///
/// # Errors
///
/// [`EntailError::Parse`] on a malformed class-expression graph (a restriction with no
/// `owl:onProperty`, a non-integer cardinality literal, a broken RDF list, …).
pub(crate) fn build(ds: &RdfDataset) -> Result<Kb, EntailError> {
    let mut interner = Interner::default();
    let v = Vocab::intern(&mut interner);
    let mut table = ConceptTable::default();
    let top = table.top();
    let bottom = table.bottom();

    // Intern every default-graph triple and build the subject index.
    let mut index: TripleIndex = BTreeMap::new();
    let mut triples: Vec<(u32, u32, u32)> = Vec::new();
    for q in ds.quads() {
        if q.g.is_some() {
            continue;
        }
        let s = interner.intern(ds.term_value(q.s));
        let p = interner.intern(ds.term_value(q.p));
        let o = interner.intern(ds.term_value(q.o));
        triples.push((s, p, o));
        index_insert(&mut index, s, p, o);
    }

    let mut acc = Accums::default();
    {
        let mut ce = CeExtractor::new(&index, &interner, &v);
        for &spo in &triples {
            axiom(&mut ce, &mut table, &mut acc, &v, &interner, spo)?;
        }
    }

    table.finalize();
    Ok(Kb {
        interner,
        table,
        top,
        bottom,
        tbox: acc.tbox,
        meta: acc.meta,
        unfold: acc.unfold,
        inverses: acc.inverses,
        role_sub: acc.role_sub,
        abox_types: acc.abox_types,
        abox_roles: acc.abox_roles,
        same_as: acc.same_as,
        individuals: acc.individuals,
    })
}

/// The knowledge-base accumulators filled while scanning axioms.
#[derive(Default)]
struct Accums {
    tbox: Vec<(u32, u32)>,
    meta: Vec<u32>,
    unfold: BTreeMap<u32, Vec<u32>>,
    inverses: BTreeMap<u32, BTreeSet<u32>>,
    role_sub: BTreeMap<u32, BTreeSet<u32>>,
    abox_types: Vec<(u32, u32)>,
    abox_roles: Vec<(u32, u32, u32)>,
    same_as: Vec<(u32, u32)>,
    individuals: BTreeSet<u32>,
}

/// Interpret one `(s, p, o)` triple as an axiom / ABox fact.
fn axiom(
    ce: &mut CeExtractor<'_>,
    table: &mut ConceptTable,
    acc: &mut Accums,
    v: &Vocab,
    interner: &Interner,
    (s, p, o): (u32, u32, u32),
) -> Result<(), EntailError> {
    if p == v.sub_class {
        let sub = ce.expr(s)?;
        let sup = ce.expr(o)?;
        gci(table, acc, sub, sup);
    } else if p == v.equiv_class {
        let a = ce.expr(s)?;
        let b = ce.expr(o)?;
        gci(table, acc, a.clone(), b.clone());
        gci(table, acc, b, a);
    } else if p == v.disjoint {
        let a = ce.expr(s)?;
        let b = ce.expr(o)?;
        gci(table, acc, Concept::And(vec![a, b]), Concept::Bottom);
    } else if p == v.domain {
        let d = ce.expr(o)?;
        gci(
            table,
            acc,
            Concept::Some(Role::Named(s), Box::new(Concept::Top)),
            d,
        );
    } else if p == v.range {
        let d = ce.expr(o)?;
        gci(
            table,
            acc,
            Concept::Top,
            Concept::All(Role::Named(s), Box::new(d)),
        );
    } else if p == v.inverse_of {
        acc.inverses.entry(s).or_default().insert(o);
        acc.inverses.entry(o).or_default().insert(s);
    } else if p == v.equiv_prop {
        acc.role_sub.entry(s).or_default().insert(o);
        acc.role_sub.entry(o).or_default().insert(s);
    } else if p == v.sub_prop {
        // s ⊑ o : `o` has sub-role `s`.
        acc.role_sub.entry(o).or_default().insert(s);
    } else if p == v.same_as {
        acc.same_as.push((s, o));
        acc.individuals.insert(s);
        acc.individuals.insert(o);
    } else if p == v.ty {
        type_assertion(ce, table, acc, v, s, o)?;
    } else if !v.structural_preds.contains(&p) && interner.is_subject(o) {
        // Any remaining user predicate is an object-property (role) assertion.
        acc.abox_roles.push((s, p, o));
        acc.individuals.insert(s);
        acc.individuals.insert(o);
    }
    Ok(())
}

/// Handle `s rdf:type o`.
fn type_assertion(
    ce: &mut CeExtractor<'_>,
    table: &mut ConceptTable,
    acc: &mut Accums,
    v: &Vocab,
    s: u32,
    o: u32,
) -> Result<(), EntailError> {
    if o == v.functional {
        // Global functionality: ⊤ ⊑ ≤1 s.⊤.
        gci(
            table,
            acc,
            Concept::Top,
            Concept::Max(1, Role::Named(s), Box::new(Concept::Top)),
        );
        return Ok(());
    }
    if o == v.named_individual {
        acc.individuals.insert(s);
        return Ok(());
    }
    if v.structural_types.contains(&o) {
        return Ok(());
    }
    // An instance-typing assertion `s : C` for a (possibly anonymous) class C.
    let c = ce.expr(o)?;
    let cid = table.intern(c);
    acc.abox_types.push((s, cid));
    acc.individuals.insert(s);
    Ok(())
}

/// Record a GCI `sub ⊑ sup`, absorbing it into the lazy-unfolding index when its left
/// side is a single named class, else internalizing it as `nnf(¬sub ⊔ sup)`.
fn gci(table: &mut ConceptTable, acc: &mut Accums, sub: Concept, sup: Concept) {
    let sub_id = table.intern(sub.clone());
    let sup_id = table.intern(sup.clone());
    acc.tbox.push((sub_id, sup_id));
    if matches!(sub, Concept::Named(_)) {
        acc.unfold.entry(sub_id).or_default().push(sup_id);
    } else {
        let meta = Concept::Or(vec![Concept::Not(Box::new(sub)), sup]);
        acc.meta.push(table.intern(meta));
    }
}

/// Build a nominal from `owl:oneOf` ids: a singleton stays `{a}`; a larger set is the
/// disjunction of singletons (so the tableau's nominal rule only ever sees `{a}`).
fn one_of(ids: Vec<u32>) -> Concept {
    if ids.len() == 1 {
        return Concept::Nominal(vec![ids[0]]);
    }
    Concept::Or(ids.into_iter().map(|a| Concept::Nominal(vec![a])).collect())
}
