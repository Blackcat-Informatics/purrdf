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
//! # NOTHING IS DROPPED IN SILENCE
//!
//! This parser has no catch-all. Every triple it reads falls into exactly one of four
//! outcomes, and [`crate::owl_dl::constructs`] is the table that decides which:
//!
//! 1. the predicate (or, for `rdf:type`, the object) is a construct this layer HANDLES, and
//!    the axiom reaches the knowledge base;
//! 2. it is a construct that is semantically INERT under OWL 2's Direct Semantics — an
//!    annotation, an ontology-header property — so reading it would constrain no model;
//! 3. it is a construct this layer does not fully handle, and a [`Boundary`](crate::Boundary)
//!    naming it is recorded on the knowledge base;
//! 4. it is outside the reserved `owl:`/`rdf:`/`rdfs:` namespaces, so it is a caller's own
//!    vocabulary and becomes an ABox role assertion.
//!
//! A RESERVED term the table does not name is outcome 3 under
//! [`Construct::UnrecognizedTerm`] — never outcome 4, because ingesting reserved vocabulary
//! as user data is a WRONG reading rather than an incomplete one. That is not a
//! hypothetical: the previous catch-all did exactly that to `owl:propertyChainAxiom`,
//! interning the axiom's RDF list head as an individual and asserting a role edge to it.
//!
//! A class EXPRESSION this layer cannot decode is read as an OPAQUE atomic class and
//! raises its boundary, rather than refusing the whole run. That is sound in the only
//! direction that matters: an unknown class constrains nothing beyond its own name, so the
//! reading admits MORE models than the specification's and can therefore miss a clash but
//! never invent one. A MALFORMED graph — a restriction with no `owl:onProperty`, a
//! non-integer cardinality, a broken or cyclic RDF list — is still a hard
//! [`EntailError::Parse`], because there is no sound reading of it at all.
//!
//! The class-expression extraction is factored into [`CeExtractor`] — a reusable view
//! over an interned `subject → predicate → objects` index plus the shared [`Vocab`] and
//! [`Interner`]. The knowledge-base build uses it over the dataset's own triples; the
//! query-answering layer ([`crate::owl_dl::query`]) reuses the very same extractor over
//! a query's ground class-expression sub-graph, so there is one class-expression parser.
//!
//! Every extraction is deterministic (all indices are `BTreeMap`/insertion-ordered
//! `Vec`s).

use std::collections::{BTreeMap, BTreeSet};

use purrdf_core::{RdfDataset, TermValue};

use crate::EntailError;
use crate::interner::Interner;
use crate::owl_dl::Kb;
use crate::owl_dl::concept::{Concept, ConceptTable, Role};
use crate::owl_dl::constructs::{Support, support_of};
use crate::report::Construct;
use crate::vocab::{
    OWL_ALLDIFFERENT, OWL_ALLDISJOINTCLASSES, OWL_ALLDISJOINTPROPERTIES, OWL_ALLVALUESFROM,
    OWL_ANNOTATIONPROPERTY, OWL_ASSERTIONPROPERTY, OWL_ASYMMETRICPROPERTY, OWL_AXIOM,
    OWL_CARDINALITY, OWL_CLASS, OWL_COMPLEMENTOF, OWL_DATATYPECOMPLEMENTOF, OWL_DATATYPEPROPERTY,
    OWL_DIFFERENTFROM, OWL_DISJOINTUNIONOF, OWL_DISJOINTWITH, OWL_DISTINCTMEMBERS,
    OWL_EQUIVALENTCLASS, OWL_EQUIVALENTPROPERTY, OWL_FUNCTIONALPROPERTY, OWL_HASKEY, OWL_HASSELF,
    OWL_HASVALUE, OWL_INTERSECTIONOF, OWL_INVERSEFUNCTIONALPROPERTY, OWL_INVERSEOF,
    OWL_IRREFLEXIVEPROPERTY, OWL_MAXCARDINALITY, OWL_MAXQUALIFIEDCARDINALITY, OWL_MEMBERS,
    OWL_MINCARDINALITY, OWL_MINQUALIFIEDCARDINALITY, OWL_NAMEDINDIVIDUAL,
    OWL_NEGATIVEPROPERTYASSERTION, OWL_NOTHING, OWL_OBJECTPROPERTY, OWL_ONCLASS, OWL_ONDATARANGE,
    OWL_ONDATATYPE, OWL_ONEOF, OWL_ONPROPERTIES, OWL_ONPROPERTY, OWL_ONTOLOGY,
    OWL_ONTOLOGYPROPERTY, OWL_PROPERTYDISJOINTWITH, OWL_QUALIFIEDCARDINALITY,
    OWL_REFLEXIVEPROPERTY, OWL_RESTRICTION, OWL_SAMEAS, OWL_SOMEVALUESFROM, OWL_SOURCEINDIVIDUAL,
    OWL_SYMMETRICPROPERTY, OWL_TARGETINDIVIDUAL, OWL_TARGETVALUE, OWL_THING,
    OWL_TRANSITIVEPROPERTY, OWL_UNIONOF, OWL_WITHRESTRICTIONS, RDF_FIRST, RDF_NIL, RDF_PROPERTY,
    RDF_REST, RDF_TYPE, RDFS_CLASS, RDFS_DATATYPE, RDFS_DOMAIN, RDFS_RANGE, RDFS_SUBCLASSOF,
    RDFS_SUBPROPERTYOF,
};

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
    pub(crate) has_self: u32,
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
    pub(crate) on_data_range: u32,
    pub(crate) on_properties: u32,
    pub(crate) on_datatype: u32,
    pub(crate) datatype_complement: u32,
    pub(crate) with_restrictions: u32,
    pub(crate) sub_class: u32,
    pub(crate) equiv_class: u32,
    pub(crate) disjoint: u32,
    pub(crate) disjoint_union: u32,
    pub(crate) domain: u32,
    pub(crate) range: u32,
    pub(crate) inverse_of: u32,
    pub(crate) equiv_prop: u32,
    pub(crate) sub_prop: u32,
    pub(crate) property_disjoint_with: u32,
    pub(crate) functional: u32,
    pub(crate) inverse_functional: u32,
    pub(crate) symmetric: u32,
    pub(crate) asymmetric: u32,
    pub(crate) transitive: u32,
    pub(crate) reflexive: u32,
    pub(crate) irreflexive: u32,
    pub(crate) same_as: u32,
    pub(crate) different_from: u32,
    pub(crate) all_different: u32,
    pub(crate) all_disjoint_classes: u32,
    pub(crate) all_disjoint_properties: u32,
    pub(crate) negative_property_assertion: u32,
    pub(crate) source_individual: u32,
    pub(crate) assertion_property: u32,
    pub(crate) target_individual: u32,
    pub(crate) target_value: u32,
    pub(crate) members: u32,
    pub(crate) distinct_members: u32,
    pub(crate) has_key: u32,
    pub(crate) first: u32,
    pub(crate) rest: u32,
    pub(crate) nil: u32,
    pub(crate) named_individual: u32,
    /// Class/property-typing objects that mark structure, not an instance assertion.
    pub(crate) structural_types: BTreeSet<u32>,
}

impl Vocab {
    /// Intern (idempotently) the vocabulary IRIs into `i`, returning their ids.
    pub(crate) fn intern(i: &mut Interner) -> Self {
        let class = i.intern_iri(OWL_CLASS);
        let restriction = i.intern_iri(OWL_RESTRICTION);
        let mut structural_types = BTreeSet::new();
        for iri in [
            OWL_CLASS,
            OWL_RESTRICTION,
            OWL_OBJECTPROPERTY,
            OWL_DATATYPEPROPERTY,
            OWL_ANNOTATIONPROPERTY,
            OWL_ONTOLOGYPROPERTY,
            OWL_FUNCTIONALPROPERTY,
            OWL_INVERSEFUNCTIONALPROPERTY,
            OWL_SYMMETRICPROPERTY,
            OWL_ASYMMETRICPROPERTY,
            OWL_TRANSITIVEPROPERTY,
            OWL_REFLEXIVEPROPERTY,
            OWL_IRREFLEXIVEPROPERTY,
            OWL_ONTOLOGY,
            OWL_NAMEDINDIVIDUAL,
            OWL_ALLDIFFERENT,
            OWL_ALLDISJOINTCLASSES,
            OWL_ALLDISJOINTPROPERTIES,
            OWL_NEGATIVEPROPERTYASSERTION,
            OWL_AXIOM,
            RDF_PROPERTY,
            RDFS_CLASS,
            RDFS_DATATYPE,
        ] {
            structural_types.insert(i.intern_iri(iri));
        }

        Self {
            ty: i.intern_iri(RDF_TYPE),
            thing: i.intern_iri(OWL_THING),
            nothing: i.intern_iri(OWL_NOTHING),
            class,
            restriction,
            on_property: i.intern_iri(OWL_ONPROPERTY),
            some_values: i.intern_iri(OWL_SOMEVALUESFROM),
            all_values: i.intern_iri(OWL_ALLVALUESFROM),
            has_value: i.intern_iri(OWL_HASVALUE),
            has_self: i.intern_iri(OWL_HASSELF),
            intersection: i.intern_iri(OWL_INTERSECTIONOF),
            union: i.intern_iri(OWL_UNIONOF),
            complement: i.intern_iri(OWL_COMPLEMENTOF),
            one_of: i.intern_iri(OWL_ONEOF),
            min_card: i.intern_iri(OWL_MINCARDINALITY),
            max_card: i.intern_iri(OWL_MAXCARDINALITY),
            card: i.intern_iri(OWL_CARDINALITY),
            min_qcard: i.intern_iri(OWL_MINQUALIFIEDCARDINALITY),
            max_qcard: i.intern_iri(OWL_MAXQUALIFIEDCARDINALITY),
            qcard: i.intern_iri(OWL_QUALIFIEDCARDINALITY),
            on_class: i.intern_iri(OWL_ONCLASS),
            on_data_range: i.intern_iri(OWL_ONDATARANGE),
            on_properties: i.intern_iri(OWL_ONPROPERTIES),
            on_datatype: i.intern_iri(OWL_ONDATATYPE),
            datatype_complement: i.intern_iri(OWL_DATATYPECOMPLEMENTOF),
            with_restrictions: i.intern_iri(OWL_WITHRESTRICTIONS),
            sub_class: i.intern_iri(RDFS_SUBCLASSOF),
            equiv_class: i.intern_iri(OWL_EQUIVALENTCLASS),
            disjoint: i.intern_iri(OWL_DISJOINTWITH),
            disjoint_union: i.intern_iri(OWL_DISJOINTUNIONOF),
            domain: i.intern_iri(RDFS_DOMAIN),
            range: i.intern_iri(RDFS_RANGE),
            inverse_of: i.intern_iri(OWL_INVERSEOF),
            equiv_prop: i.intern_iri(OWL_EQUIVALENTPROPERTY),
            sub_prop: i.intern_iri(RDFS_SUBPROPERTYOF),
            property_disjoint_with: i.intern_iri(OWL_PROPERTYDISJOINTWITH),
            functional: i.intern_iri(OWL_FUNCTIONALPROPERTY),
            inverse_functional: i.intern_iri(OWL_INVERSEFUNCTIONALPROPERTY),
            symmetric: i.intern_iri(OWL_SYMMETRICPROPERTY),
            asymmetric: i.intern_iri(OWL_ASYMMETRICPROPERTY),
            transitive: i.intern_iri(OWL_TRANSITIVEPROPERTY),
            reflexive: i.intern_iri(OWL_REFLEXIVEPROPERTY),
            irreflexive: i.intern_iri(OWL_IRREFLEXIVEPROPERTY),
            same_as: i.intern_iri(OWL_SAMEAS),
            different_from: i.intern_iri(OWL_DIFFERENTFROM),
            all_different: i.intern_iri(OWL_ALLDIFFERENT),
            all_disjoint_classes: i.intern_iri(OWL_ALLDISJOINTCLASSES),
            all_disjoint_properties: i.intern_iri(OWL_ALLDISJOINTPROPERTIES),
            negative_property_assertion: i.intern_iri(OWL_NEGATIVEPROPERTYASSERTION),
            source_individual: i.intern_iri(OWL_SOURCEINDIVIDUAL),
            assertion_property: i.intern_iri(OWL_ASSERTIONPROPERTY),
            target_individual: i.intern_iri(OWL_TARGETINDIVIDUAL),
            target_value: i.intern_iri(OWL_TARGETVALUE),
            members: i.intern_iri(OWL_MEMBERS),
            distinct_members: i.intern_iri(OWL_DISTINCTMEMBERS),
            has_key: i.intern_iri(OWL_HASKEY),
            first: i.intern_iri(RDF_FIRST),
            rest: i.intern_iri(RDF_REST),
            nil: i.intern_iri(RDF_NIL),
            named_individual: i.intern_iri(OWL_NAMEDINDIVIDUAL),
            structural_types,
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
    /// Boundaries raised while decoding — a class expression read as an opaque atomic
    /// class rather than as what it says.
    boundaries: BTreeSet<Construct>,
    /// Every role a NUMBER restriction counts over, so the caller can apply OWL 2 DL's
    /// simple-role condition once the transitivity axioms are known.
    counted_roles: BTreeSet<Role>,
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
            boundaries: BTreeSet::new(),
            counted_roles: BTreeSet::new(),
        }
    }

    /// The boundaries raised so far.
    pub(crate) fn boundaries(&self) -> &BTreeSet<Construct> {
        &self.boundaries
    }

    /// The roles a number restriction counted over, for the simple-role check.
    pub(crate) fn counted_roles(&self) -> &BTreeSet<Role> {
        &self.counted_roles
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

    /// Record `construct` as a boundary and read `node` as an OPAQUE atomic class.
    ///
    /// Sound in the only direction that matters: an unknown class constrains nothing
    /// beyond its own name, so the reading admits more models than the specification's and
    /// can miss a clash but never invent one.
    fn opaque(&mut self, node: u32, construct: Construct) -> Concept {
        self.boundaries.insert(construct);
        Concept::Named(node)
    }

    /// Structurally decode `node` into a [`Concept`].
    fn build_expr(&mut self, node: u32) -> Result<Concept, EntailError> {
        if node == self.v.thing {
            return Ok(Concept::Top);
        }
        if node == self.v.nothing {
            return Ok(Concept::Bottom);
        }
        // A RESERVED term used in a class position that this layer does not model —
        // `owl:real`, `owl:rational`, a newer-than-this-release OWL class — is read
        // opaquely under its own boundary rather than as an ordinary named class, which is
        // what it would otherwise silently become.
        if let TermValue::Iri(iri) = self.interner.value(node)
            && let Some(Support::Bounded(construct)) = support_of(iri)
        {
            return Ok(self.opaque(node, construct));
        }
        // A DATA range is a concrete-domain expression, not a class expression; reading it
        // as one would be a wrong answer rather than an incomplete one.
        for data_range in [
            self.v.on_datatype,
            self.v.datatype_complement,
            self.v.with_restrictions,
        ] {
            if self.get(node, data_range).is_some() {
                return Ok(self.opaque(node, Construct::DataRange));
            }
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
            // An `owl:oneOf` over LITERALS is `DataOneOf`, a data range.
            if ids
                .iter()
                .any(|&id| matches!(self.interner.value(id), TermValue::Literal { .. }))
            {
                return Ok(self.opaque(node, Construct::DataRange));
            }
            return Ok(one_of(ids));
        }
        if self.get(node, self.v.on_property).is_some() || self.is_typed(node, self.v.restriction) {
            return self.restriction(node);
        }
        if self.get(node, self.v.on_properties).is_some() {
            // An n-ary data restriction: several data properties over one data range.
            return Ok(self.opaque(node, Construct::DataRange));
        }
        // An atomic named (or otherwise opaque) class.
        Ok(Concept::Named(node))
    }

    /// Decode an `owl:Restriction` node.
    fn restriction(&mut self, node: u32) -> Result<Concept, EntailError> {
        let r = self.get(node, self.v.on_property).ok_or_else(|| {
            EntailError::Parse("owl:Restriction without owl:onProperty".to_owned())
        })?;
        // A restriction over a BUILT-IN role (`owl:topObjectProperty`,
        // `owl:bottomObjectProperty`, …) reads it as an ordinary named role, which is
        // exactly what the built-in-role boundary is about. It is raised HERE as well as at
        // the role-assertion site, because a built-in role reaches the knowledge base
        // through either position.
        if let TermValue::Iri(iri) = self.interner.value(r)
            && let Some(Support::Bounded(construct)) = support_of(iri)
        {
            self.boundaries.insert(construct);
        }
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
        if let Some(lit) = self.get(node, self.v.has_self) {
            return Ok(self.self_restriction(role, lit));
        }
        // A qualified cardinality over a DATA range counts into the concrete domain.
        if self.get(node, self.v.on_data_range).is_some() {
            return Ok(self.opaque(node, Construct::DataRange));
        }
        if let Some(lit) = self.get(node, self.v.min_qcard) {
            let n = self.card(lit)?;
            let c = self.qualified_class(node)?;
            self.counted_roles.insert(role);
            return Ok(Concept::Min(n, role, Box::new(c)));
        }
        if let Some(lit) = self.get(node, self.v.max_qcard) {
            let n = self.card(lit)?;
            let c = self.qualified_class(node)?;
            self.counted_roles.insert(role);
            return Ok(Concept::Max(n, role, Box::new(c)));
        }
        if let Some(lit) = self.get(node, self.v.qcard) {
            let n = self.card(lit)?;
            let c = self.qualified_class(node)?;
            self.counted_roles.insert(role);
            return Ok(Concept::And(vec![
                Concept::Min(n, role, Box::new(c.clone())),
                Concept::Max(n, role, Box::new(c)),
            ]));
        }
        if let Some(lit) = self.get(node, self.v.min_card) {
            let n = self.card(lit)?;
            self.counted_roles.insert(role);
            return Ok(Concept::Min(n, role, Box::new(Concept::Top)));
        }
        if let Some(lit) = self.get(node, self.v.max_card) {
            let n = self.card(lit)?;
            self.counted_roles.insert(role);
            return Ok(Concept::Max(n, role, Box::new(Concept::Top)));
        }
        if let Some(lit) = self.get(node, self.v.card) {
            let n = self.card(lit)?;
            self.counted_roles.insert(role);
            return Ok(Concept::And(vec![
                Concept::Min(n, role, Box::new(Concept::Top)),
                Concept::Max(n, role, Box::new(Concept::Top)),
            ]));
        }
        // Well formed — it carries `owl:onProperty` — but constrained by nothing this
        // layer decodes. A named boundary, not a refusal and not silence.
        Ok(self.opaque(node, Construct::UnrecognizedTerm))
    }

    /// `owl:hasSelf` with the boolean `lit`: `∃r.Self` for `true`, `¬∃r.Self` for `false`.
    ///
    /// A value that is neither is not a boolean literal, so the restriction states nothing
    /// this layer can read: it becomes the opaque reading under its own boundary rather
    /// than being guessed either way.
    fn self_restriction(&mut self, role: Role, lit: u32) -> Concept {
        let truth = match self.interner.value(lit) {
            TermValue::Literal { lexical_form, .. } => match lexical_form.trim() {
                "true" | "1" => Some(true),
                "false" | "0" => Some(false),
                _ => None,
            },
            _ => None,
        };
        match truth {
            Some(true) => Concept::SelfRestriction(role),
            Some(false) => Concept::Not(Box::new(Concept::SelfRestriction(role))),
            None => {
                self.boundaries.insert(Construct::UnrecognizedTerm);
                Concept::Top
            }
        }
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
        if matches!(self.interner.value(r), TermValue::Blank { .. })
            && let Some(inv) = self.get(r, self.v.inverse_of)
        {
            return Role::Inv(inv);
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
    pub(crate) fn node_list(&self, head: u32) -> Result<Vec<u32>, EntailError> {
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
        // The n-ary axioms are stated as a node carrying a list or a reification, so they
        // are read from the node rather than from any one of its triples.
        for (&node, preds) in &index {
            for &axiom_class in preds.get(&v.ty).map_or(&[][..], Vec::as_slice) {
                axiom_node(&mut ce, &mut table, &mut acc, &v, node, axiom_class)?;
            }
            // A negative property assertion is identified by its VOCABULARY, not by its
            // typing: OWL 2's RDF-Based Semantics states the axiom over any node carrying
            // `owl:sourceIndividual`, and the W3C corpus contains such a node with no
            // `rdf:type owl:NegativePropertyAssertion` triple at all. Keying on the typing
            // alone would read that ontology as saying nothing.
            if preds.contains_key(&v.source_individual)
                && !preds
                    .get(&v.ty)
                    .is_some_and(|types| types.contains(&v.negative_property_assertion))
            {
                negative_assertion(&ce, &mut table, &mut acc, &v, node)?;
            }
        }
        for &spo in &triples {
            axiom(&mut ce, &mut table, &mut acc, &v, &interner, spo)?;
        }
        acc.boundaries.extend(ce.boundaries().iter().copied());
        // OWL 2 DL forbids a number restriction over a NON-SIMPLE role. The transitivity
        // axioms are only all known now, so the condition is checked here rather than
        // while the restriction was being decoded.
        if ce
            .counted_roles()
            .iter()
            .any(|role| is_non_simple(*role, &acc))
        {
            acc.boundaries.insert(Construct::NonSimpleRole);
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
        different_from: acc.different_from,
        individuals: acc.individuals,
        transitive: acc.transitive,
        asymmetric: acc.asymmetric,
        disjoint_roles: acc.disjoint_roles,
        keys: acc.keys,
        boundaries: acc.boundaries,
    })
}

/// Whether `role` is NON-SIMPLE — transitive, or the super-role of a transitive role.
///
/// A role's inverse is simple exactly when the role is, so `Role::Inv(p)` is decided on
/// `p`. The sub-role closure walks [`Accums::role_sub`], which is the same hierarchy the
/// tableau's `achievers` walks, so the two agree on what a role's extension contains.
fn is_non_simple(role: Role, acc: &Accums) -> bool {
    let (Role::Named(head) | Role::Inv(head)) = role;
    let mut seen: BTreeSet<u32> = BTreeSet::new();
    let mut stack = vec![head];
    while let Some(current) = stack.pop() {
        if !seen.insert(current) {
            continue;
        }
        if acc.transitive.contains(&current) {
            return true;
        }
        if let Some(subs) = acc.role_sub.get(&current) {
            stack.extend(subs.iter().copied());
        }
    }
    false
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
    different_from: Vec<(u32, u32)>,
    individuals: BTreeSet<u32>,
    transitive: BTreeSet<u32>,
    asymmetric: BTreeSet<u32>,
    disjoint_roles: BTreeSet<(u32, u32)>,
    keys: Vec<(u32, Vec<u32>)>,
    boundaries: BTreeSet<Construct>,
}

impl Accums {
    /// Record `individual` as a NAMED individual — a tableau root and a realization
    /// candidate. A literal is deliberately not one: `i rdf:type C` with a literal `i` is
    /// a generalized-RDF triple the dataset IR cannot hold.
    fn name(&mut self, interner: &Interner, individual: u32) {
        if interner.is_subject(individual) {
            self.individuals.insert(individual);
        }
    }
}

/// Interpret an n-ary axiom stated as `node rdf:type axiom_class`.
///
/// `owl:AllDifferent`, `owl:AllDisjointClasses`, `owl:AllDisjointProperties` and
/// `owl:NegativePropertyAssertion` are all written as a typed node whose OTHER triples
/// carry the axiom's operands, so none of them can be read one triple at a time. They are
/// read here, from the node, before the per-triple pass; the per-triple pass then sees the
/// operand predicates (`owl:members`, `owl:sourceIndividual`, …) as constructs the table
/// marks handled and leaves them alone.
fn axiom_node(
    ce: &mut CeExtractor<'_>,
    table: &mut ConceptTable,
    acc: &mut Accums,
    v: &Vocab,
    node: u32,
    axiom_class: u32,
) -> Result<(), EntailError> {
    if axiom_class == v.all_different {
        // Both spellings, and both are read: OWL 2 writes `owl:members`, OWL 1 wrote
        // `owl:distinctMembers`, and an ontology may carry either.
        for list_property in [v.members, v.distinct_members] {
            let Some(head) = first_object(ce, node, list_property) else {
                continue;
            };
            let members = ce.node_list(head)?;
            for (index, &left) in members.iter().enumerate() {
                for &right in &members[index + 1..] {
                    acc.different_from.push((left, right));
                }
            }
            for member in members {
                acc.individuals.insert(member);
            }
        }
    } else if axiom_class == v.all_disjoint_classes {
        if let Some(head) = first_object(ce, node, v.members) {
            let members = ce.node_list(head)?;
            let concepts: Vec<Concept> = members
                .iter()
                .map(|&m| ce.expr(m))
                .collect::<Result<_, _>>()?;
            for (index, left) in concepts.iter().enumerate() {
                for right in &concepts[index + 1..] {
                    gci(
                        table,
                        acc,
                        Concept::And(vec![left.clone(), right.clone()]),
                        Concept::Bottom,
                    );
                }
            }
        }
    } else if axiom_class == v.all_disjoint_properties {
        if let Some(head) = first_object(ce, node, v.members) {
            let members = ce.node_list(head)?;
            for (index, &left) in members.iter().enumerate() {
                for &right in &members[index + 1..] {
                    acc.disjoint_roles.insert((left, right));
                    acc.disjoint_roles.insert((right, left));
                }
            }
        }
    } else if axiom_class == v.negative_property_assertion {
        negative_assertion(&*ce, table, acc, v, node)?;
    }
    Ok(())
}

/// `¬p(s, o)`, read from the reified `owl:NegativePropertyAssertion` node `node`.
///
/// The DL reading is the concept assertion `s : ∀p.¬{o}` — "every `p`-successor of `s` is
/// something other than `o`" — which is exactly the negation of `p(s, o)` and needs no new
/// tableau machinery: the `∀`-rule pushes `¬{o}` onto the successor, and the clash trigger
/// for a negated nominal naming the node's own individual is what closes the branch.
///
/// A node missing any of the three components states no assertion, so it is left alone
/// rather than half-read.
fn negative_assertion(
    ce: &CeExtractor<'_>,
    table: &mut ConceptTable,
    acc: &mut Accums,
    v: &Vocab,
    node: u32,
) -> Result<(), EntailError> {
    let (Some(source), Some(property)) = (
        first_object(ce, node, v.source_individual),
        first_object(ce, node, v.assertion_property),
    ) else {
        return Ok(());
    };
    let Some(target) = first_object(ce, node, v.target_individual)
        .or_else(|| first_object(ce, node, v.target_value))
    else {
        return Ok(());
    };
    let concept = Concept::All(
        Role::Named(property),
        Box::new(Concept::Not(Box::new(Concept::Nominal(vec![target])))),
    );
    let cid = table.intern(concept);
    acc.abox_types.push((source, cid));
    acc.individuals.insert(source);
    Ok(())
}

/// The first object of `(subject, predicate, ·)` in the extractor's own index.
fn first_object(ce: &CeExtractor<'_>, subject: u32, predicate: u32) -> Option<u32> {
    ce.index.get(&subject)?.get(&predicate)?.first().copied()
}

/// Interpret one `(s, p, o)` triple as an axiom / ABox fact.
///
/// Every branch is a construct [`crate::owl_dl::constructs`] marks handled; the final
/// `else` is where the table itself decides, and it has no silent arm.
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
    } else if p == v.disjoint_union {
        disjoint_union(ce, table, acc, s, o)?;
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
    } else if p == v.property_disjoint_with {
        acc.disjoint_roles.insert((s, o));
        acc.disjoint_roles.insert((o, s));
    } else if p == v.has_key {
        let class = ce.expr(s)?;
        let cid = table.intern(class);
        acc.keys.push((cid, ce.node_list(o)?));
    } else if p == v.same_as {
        acc.same_as.push((s, o));
        acc.name(interner, s);
        acc.name(interner, o);
    } else if p == v.different_from {
        acc.different_from.push((s, o));
        acc.name(interner, s);
        acc.name(interner, o);
    } else if p == v.ty {
        type_assertion(ce, table, acc, v, interner, s, o)?;
    } else {
        role_or_boundary(acc, interner, s, p, o);
    }
    Ok(())
}

/// `C owl:disjointUnionOf (C₁ … Cₙ)`: `C ≡ C₁ ⊔ … ⊔ Cₙ` with the `Cᵢ` pairwise disjoint.
fn disjoint_union(
    ce: &mut CeExtractor<'_>,
    table: &mut ConceptTable,
    acc: &mut Accums,
    class: u32,
    list: u32,
) -> Result<(), EntailError> {
    let members = ce.node_list(list)?;
    let parts: Vec<Concept> = members
        .iter()
        .map(|&m| ce.expr(m))
        .collect::<Result<_, _>>()?;
    let whole = ce.expr(class)?;
    let union = Concept::Or(parts.clone());
    gci(table, acc, whole.clone(), union.clone());
    gci(table, acc, union, whole);
    for (index, left) in parts.iter().enumerate() {
        for right in &parts[index + 1..] {
            gci(
                table,
                acc,
                Concept::And(vec![left.clone(), right.clone()]),
                Concept::Bottom,
            );
        }
    }
    Ok(())
}

/// The triple's predicate is not one this layer dispatches on: read it through the
/// construct table, which has no silent arm.
///
/// * a HANDLED construct is one another pass consumes (an operand predicate of an n-ary
///   axiom, a class-expression predicate the extractor reads when the node is referenced),
///   so there is nothing to do here and nothing lost;
/// * an INERT construct constrains no model, so ignoring it loses nothing either;
/// * a BOUNDED construct records its boundary;
/// * a term outside the reserved namespaces is the caller's own vocabulary and becomes a
///   role assertion.
fn role_or_boundary(acc: &mut Accums, interner: &Interner, s: u32, p: u32, o: u32) {
    let iri = match interner.value(p) {
        TermValue::Iri(iri) => iri.clone(),
        // A non-IRI predicate is not RDF 1.2 and cannot reach here from a frozen dataset,
        // but reading it as user vocabulary would be the wrong answer if it ever did.
        _ => {
            acc.boundaries.insert(Construct::UnrecognizedTerm);
            return;
        }
    };
    match support_of(&iri) {
        Some(Support::Handled(_) | Support::Operand(_) | Support::Inert(_)) => {}
        Some(Support::Bounded(construct)) => {
            acc.boundaries.insert(construct);
        }
        None => {
            acc.abox_roles.push((s, p, o));
            acc.name(interner, s);
            acc.name(interner, o);
        }
    }
}

/// Handle `s rdf:type o`.
fn type_assertion(
    ce: &mut CeExtractor<'_>,
    table: &mut ConceptTable,
    acc: &mut Accums,
    v: &Vocab,
    interner: &Interner,
    s: u32,
    o: u32,
) -> Result<(), EntailError> {
    // --- The role characteristics, each a global axiom or a graph-level constraint. ---
    if o == v.functional {
        // ⊤ ⊑ ≤1 s.⊤.
        gci(
            table,
            acc,
            Concept::Top,
            Concept::Max(1, Role::Named(s), Box::new(Concept::Top)),
        );
        return Ok(());
    }
    if o == v.inverse_functional {
        // ⊤ ⊑ ≤1 s⁻.⊤.
        gci(
            table,
            acc,
            Concept::Top,
            Concept::Max(1, Role::Inv(s), Box::new(Concept::Top)),
        );
        return Ok(());
    }
    if o == v.symmetric {
        // r ≡ r⁻: the role is its own inverse, so the role closure reaches both directions.
        acc.inverses.entry(s).or_default().insert(s);
        return Ok(());
    }
    if o == v.transitive {
        acc.transitive.insert(s);
        return Ok(());
    }
    if o == v.asymmetric {
        acc.asymmetric.insert(s);
        return Ok(());
    }
    if o == v.reflexive {
        // ⊤ ⊑ ∃r.Self.
        gci(
            table,
            acc,
            Concept::Top,
            Concept::SelfRestriction(Role::Named(s)),
        );
        return Ok(());
    }
    if o == v.irreflexive {
        // ⊤ ⊑ ¬∃r.Self.
        gci(
            table,
            acc,
            Concept::Top,
            Concept::Not(Box::new(Concept::SelfRestriction(Role::Named(s)))),
        );
        return Ok(());
    }
    if o == v.named_individual {
        acc.individuals.insert(s);
        return Ok(());
    }
    if v.structural_types.contains(&o) {
        // A declaration or an n-ary axiom node; the latter was read by `axiom_node`.
        return Ok(());
    }
    // A reserved class this layer does not model is a boundary, not an instance assertion:
    // typing an individual into `owl:topObjectProperty` says something about the reserved
    // vocabulary rather than about the individual.
    if let TermValue::Iri(iri) = interner.value(o)
        && let Some(support) = support_of(iri)
    {
        match support {
            Support::Bounded(construct) => {
                acc.boundaries.insert(construct);
                return Ok(());
            }
            Support::Inert(_) | Support::Operand(_) => return Ok(()),
            Support::Handled(_) => {}
        }
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
