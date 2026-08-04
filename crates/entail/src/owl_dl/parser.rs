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
//!
//! # How deep an expression may nest, and why there is a limit at all
//!
//! A class expression's nesting depth is a property of the DOCUMENT, not of the vocabulary:
//! `_:c0 owl:complementOf _:c1 . _:c1 owl:complementOf _:c2 . …` nests once per triple, and
//! a data range does the same through `owl:datatypeComplementOf`. [`CeExtractor`] decodes
//! that structure recursively, and — this is the part a depth-free reading misses — what it
//! decodes it INTO is a [`Concept`] (or [`DataRange`]) tree of the same depth, whose `Drop`,
//! `Clone`, [`Concept::nnf`] and interning all walk it recursively in turn. An unbounded
//! nesting is therefore not one recursion to make iterative but a family of them, one of
//! which is the destructor.
//!
//! So the depth is bounded where the tree is BUILT, by [`MAX_EXPRESSION_DEPTH`]: no
//! over-deep tree is ever constructed, and every consumer of one is protected by the same
//! single check. Exceeding it is an [`EntailError::Parse`] — a refusal every host already
//! propagates — rather than a stack overflow, which is not a refusal at all: the process
//! aborts, nothing unwinds, and a host embedding this library dies with it.

use std::collections::{BTreeMap, BTreeSet};

use purrdf_core::{RdfDataset, TermValue};
use purrdf_datalog::StopSignal;
use purrdf_xsd::XsdDatatype;
use purrdf_xsd::range::{DataRange, Facet};

use crate::EntailError;
use crate::interner::Interner;
use crate::owl_dl::Kb;
use crate::owl_dl::concept::{Concept, ConceptTable, Decomp, Role};
use crate::owl_dl::constructs::{Support, support_of};
use crate::owl_dl::data::{self, DataRangeTable, LiteralValue};
use crate::report::Construct;
use crate::vocab::{
    OWL_ALLDIFFERENT, OWL_ALLDISJOINTCLASSES, OWL_ALLDISJOINTPROPERTIES, OWL_ALLVALUESFROM,
    OWL_ANNOTATIONPROPERTY, OWL_ASSERTIONPROPERTY, OWL_ASYMMETRICPROPERTY, OWL_AXIOM,
    OWL_CARDINALITY, OWL_CLASS, OWL_COMPLEMENTOF, OWL_DATARANGE, OWL_DATATYPECOMPLEMENTOF,
    OWL_DATATYPEPROPERTY, OWL_DIFFERENTFROM, OWL_DISJOINTUNIONOF, OWL_DISJOINTWITH,
    OWL_DISTINCTMEMBERS, OWL_EQUIVALENTCLASS, OWL_EQUIVALENTPROPERTY, OWL_FUNCTIONALPROPERTY,
    OWL_HASKEY, OWL_HASSELF, OWL_HASVALUE, OWL_INTERSECTIONOF, OWL_INVERSEFUNCTIONALPROPERTY,
    OWL_INVERSEOF, OWL_IRREFLEXIVEPROPERTY, OWL_MAXCARDINALITY, OWL_MAXQUALIFIEDCARDINALITY,
    OWL_MEMBERS, OWL_MINCARDINALITY, OWL_MINQUALIFIEDCARDINALITY, OWL_NAMEDINDIVIDUAL,
    OWL_NEGATIVEPROPERTYASSERTION, OWL_NOTHING, OWL_OBJECTPROPERTY, OWL_ONCLASS, OWL_ONDATARANGE,
    OWL_ONDATATYPE, OWL_ONEOF, OWL_ONPROPERTIES, OWL_ONPROPERTY, OWL_ONTOLOGY,
    OWL_ONTOLOGYPROPERTY, OWL_PROPERTYDISJOINTWITH, OWL_QUALIFIEDCARDINALITY, OWL_RATIONAL,
    OWL_REAL, OWL_REFLEXIVEPROPERTY, OWL_RESTRICTION, OWL_SAMEAS, OWL_SOMEVALUESFROM,
    OWL_SOURCEINDIVIDUAL, OWL_SYMMETRICPROPERTY, OWL_TARGETINDIVIDUAL, OWL_TARGETVALUE, OWL_THING,
    OWL_TRANSITIVEPROPERTY, OWL_UNIONOF, OWL_WITHRESTRICTIONS, RDF_FIRST, RDF_LANGRANGE, RDF_NIL,
    RDF_PROPERTY, RDF_REST, RDF_TYPE, RDFS_CLASS, RDFS_DATATYPE, RDFS_DOMAIN, RDFS_LITERAL,
    RDFS_RANGE, RDFS_SUBCLASSOF, RDFS_SUBPROPERTYOF, XSD_LENGTH, XSD_MAXEXCLUSIVE,
    XSD_MAXINCLUSIVE, XSD_MAXLENGTH, XSD_MINEXCLUSIVE, XSD_MININCLUSIVE, XSD_MINLENGTH, XSD_NS,
    XSD_PATTERN,
};

/// How deeply a class expression or a data range may nest before the parser refuses.
///
/// MEASURED rather than guessed, and measured on the SMALLEST stack this library runs on —
/// `wasm32`'s 1 MiB, an eighth of a native thread's default. Driving the whole OWL-Direct
/// pipeline (parse, negation-normalize, intern, decide, drop) over an `owl:complementOf`
/// chain under a 1 MiB rlimit completed at 1600 levels and aborted at 2400, in both a debug
/// and a release build. This ceiling is a factor of six below the shallower of those two
/// figures, so the refusal arrives with the stack still nearly untouched.
///
/// It costs no expressible ontology: OWL 2's own RDF mapping nests a class expression once
/// per operator, and the deepest published ontologies nest tens of levels, not hundreds.
/// This crate's sibling bound on RDF 1.2 triple-term nesting is 16.
pub(crate) const MAX_EXPRESSION_DEPTH: usize = 256;

/// Which constraining facet a predicate of an `owl:withRestrictions` list cell states.
///
/// The facet IRIs sit in the XML Schema namespace, which the OWL-2-RDF mapping does not
/// reserve, so they are recognized by this table rather than by the reserved-namespace split.
/// [`FacetSlot::Undecided`] is the honest arm: a facet whose constraint this layer decides
/// nothing about makes the whole range opaque, because SILENTLY DROPPING a facet is unsound —
/// under a complement, a dropped constraint SHRINKS the range and can invent an emptiness the
/// ontology does not state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FacetSlot {
    /// `xsd:minInclusive`.
    MinInclusive,
    /// `xsd:maxInclusive`.
    MaxInclusive,
    /// `xsd:minExclusive`.
    MinExclusive,
    /// `xsd:maxExclusive`.
    MaxExclusive,
    /// `xsd:length`.
    Length,
    /// `xsd:minLength`.
    MinLength,
    /// `xsd:maxLength`.
    MaxLength,
    /// A facet this layer models nothing about — `xsd:pattern`, whose emptiness question is a
    /// regular-language product construction, and `rdf:langRange`, whose value space is
    /// `rdf:langString`'s rather than an XSD one.
    Undecided,
}

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
    /// `rdfs:Datatype` — the typing that marks a node as a DATA RANGE rather than a class.
    pub(crate) datatype: u32,
    /// `owl:DataRange` — OWL 1's spelling of the same typing.
    pub(crate) data_range_class: u32,
    /// `rdfs:Literal` — the whole data domain.
    pub(crate) literal: u32,
    /// `owl:real`, `owl:rational` — datatype names outside the modelled XSD value space.
    pub(crate) real: u32,
    /// See [`Vocab::real`].
    pub(crate) rational: u32,
    /// Facet predicate term ids paired with the constraining facet each states.
    ///
    /// A `Vec` rather than a map: there are nine of them, so a linear scan is both smaller
    /// and faster than a tree, and the fixed reading order keeps every lookup deterministic.
    pub(crate) facets: Vec<(u32, FacetSlot)>,
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
        let facets = [
            (XSD_MININCLUSIVE, FacetSlot::MinInclusive),
            (XSD_MAXINCLUSIVE, FacetSlot::MaxInclusive),
            (XSD_MINEXCLUSIVE, FacetSlot::MinExclusive),
            (XSD_MAXEXCLUSIVE, FacetSlot::MaxExclusive),
            (XSD_LENGTH, FacetSlot::Length),
            (XSD_MINLENGTH, FacetSlot::MinLength),
            (XSD_MAXLENGTH, FacetSlot::MaxLength),
            (XSD_PATTERN, FacetSlot::Undecided),
            (RDF_LANGRANGE, FacetSlot::Undecided),
        ]
        .map(|(iri, slot)| (i.intern_iri(iri), slot))
        .to_vec();

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
            datatype: i.intern_iri(RDFS_DATATYPE),
            data_range_class: i.intern_iri(OWL_DATARANGE),
            literal: i.intern_iri(RDFS_LITERAL),
            real: i.intern_iri(OWL_REAL),
            rational: i.intern_iri(OWL_RATIONAL),
            facets,
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
    /// The knowledge base's data-range table, which a decoded data range is recorded in.
    /// Borrowed rather than owned so a class expression written in a QUERY lands its data
    /// ranges in the same table — and therefore under the same concept ids — as one written
    /// in the data.
    ranges: &'a mut DataRangeTable,
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
    /// The caller-owned cancellation signal for governed reverse mapping. Query-only
    /// extraction leaves this absent.
    stop: Option<&'a dyn StopSignal>,
}

impl<'a> CeExtractor<'a> {
    /// Build an extractor over `index`, resolving terms through `interner` and keying
    /// on `v`.
    pub(crate) fn new(
        index: &'a TripleIndex,
        interner: &'a Interner,
        v: &'a Vocab,
        ranges: &'a mut DataRangeTable,
    ) -> Self {
        Self::new_until(index, interner, v, ranges, None)
    }

    /// Build an extractor that polls `stop` while traversing class expressions and RDF
    /// collections.
    fn new_until(
        index: &'a TripleIndex,
        interner: &'a Interner,
        v: &'a Vocab,
        ranges: &'a mut DataRangeTable,
        stop: Option<&'a dyn StopSignal>,
    ) -> Self {
        Self {
            index,
            interner,
            v,
            ranges,
            expr_cache: BTreeMap::new(),
            in_progress: BTreeSet::new(),
            boundaries: BTreeSet::new(),
            counted_roles: BTreeSet::new(),
            stop,
        }
    }

    /// Refuse promptly when the governed caller has cancelled reverse mapping.
    fn poll(&self) -> Result<(), EntailError> {
        poll(self.stop)
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
    /// [`EntailError::Parse`] on a malformed class-expression graph, on a cyclic one, and on
    /// one nesting past [`MAX_EXPRESSION_DEPTH`].
    pub(crate) fn expr(&mut self, node: u32) -> Result<Concept, EntailError> {
        self.poll()?;
        if let Some(c) = self.expr_cache.get(&node) {
            return Ok(c.clone());
        }
        // `in_progress` holds exactly the nodes on the path from the outermost expression to
        // this one — inserted on entry, removed on the way out — so its size IS the current
        // nesting depth. See this module's *How deep an expression may nest*.
        if self.in_progress.len() >= MAX_EXPRESSION_DEPTH {
            return Err(EntailError::Parse(format!(
                "OWL class expression nests deeper than {MAX_EXPRESSION_DEPTH}"
            )));
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
        // A DATA RANGE is a CONCRETE-domain expression: a subset of the data domain rather
        // than of `owl:Thing`. It is decoded first, because most of the vocabulary below
        // (`owl:intersectionOf`, `owl:unionOf`, `owl:oneOf`) is spelled the same way in both
        // domains and only the datatype typing tells them apart.
        if self.is_data_range(node)? {
            let range = self.data_range(node, &mut Vec::new())?;
            let id = self.ranges.intern(range);
            return Ok(Concept::Data(id));
        }
        // A RESERVED term used in a class position that this layer does not model — a
        // built-in role, a newer-than-this-release OWL class — is read opaquely under its own
        // boundary rather than as an ordinary named class, which is what it would otherwise
        // silently become.
        if let TermValue::Iri(iri) = self.interner.value(node)
            && let Some(Support::Bounded(construct)) = support_of(iri)
        {
            return Ok(self.opaque(node, construct));
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
        // An `owl:oneOf` reached HERE enumerates INDIVIDUALS: one over literals is a
        // `DataOneOf`, which [`CeExtractor::is_data_range`] recognized above.
        if let Some(head) = self.get(node, self.v.one_of) {
            return Ok(one_of(self.node_list(head)?));
        }
        if self.get(node, self.v.on_property).is_some() || self.is_typed(node, self.v.restriction) {
            return self.restriction(node);
        }
        // An atomic named (or otherwise opaque) class.
        Ok(Concept::Named(node))
    }

    /// Whether `node` denotes a DATA RANGE — a subset of the data domain — rather than a
    /// class expression.
    ///
    /// The question is decided by the node's own syntax, never by guessing at the property a
    /// restriction happens to mention: a datatype NAME, a `rdfs:Datatype` (or OWL 1
    /// `owl:DataRange`) typing, one of the data-range-defining predicates, or an `owl:oneOf`
    /// whose members are literals. Every IRI in the XML Schema namespace is a datatype name,
    /// which is what keeps `xsd:anyURI` from being read as an ordinary named class.
    ///
    /// # Errors
    ///
    /// [`EntailError::Parse`] on a malformed `owl:oneOf` collection.
    fn is_data_range(&self, node: u32) -> Result<bool, EntailError> {
        if let TermValue::Iri(iri) = self.interner.value(node)
            && (iri.starts_with(XSD_NS) || node == self.v.literal)
        {
            return Ok(true);
        }
        if node == self.v.real || node == self.v.rational {
            return Ok(true);
        }
        if self.is_typed(node, self.v.datatype) || self.is_typed(node, self.v.data_range_class) {
            return Ok(true);
        }
        for defining in [
            self.v.on_datatype,
            self.v.datatype_complement,
            self.v.with_restrictions,
            self.v.on_properties,
        ] {
            if self.get(node, defining).is_some() {
                return Ok(true);
            }
        }
        if let Some(head) = self.get(node, self.v.one_of) {
            let members = self.node_list(head)?;
            if members
                .iter()
                .any(|&id| matches!(self.interner.value(id), TermValue::Literal { .. }))
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Decode the data range rooted at `node`.
    ///
    /// Everything this layer cannot decide EXACTLY becomes [`DataRange::Opaque`], which is
    /// undecidable by construction rather than by omission: `purrdf_xsd::range` answers
    /// `Undecided` for it, the tableau refuses to clash on it, and
    /// [`DataRangeTable::exactly_decided`](crate::owl_dl::data::DataRangeTable::exactly_decided)
    /// reports the boundary. Reading it as anything narrower would let a dropped constraint
    /// invent an emptiness under a complement.
    ///
    /// `path` is the chain of data-range nodes already open, outermost first — the same
    /// role [`CeExtractor::expr`]'s `in_progress` plays, but carried as an argument because
    /// data ranges are decoded through `&self`. It is what makes `_:a owl:datatypeComplementOf
    /// _:b . _:b owl:datatypeComplementOf _:a .` — legal RDF, and four lines long — a refusal
    /// rather than an unterminated descent.
    ///
    /// # Errors
    ///
    /// [`EntailError::Parse`] on a malformed collection in a data-range position, on a cyclic
    /// data range, and on one nesting past [`MAX_EXPRESSION_DEPTH`].
    fn data_range(&self, node: u32, path: &mut Vec<u32>) -> Result<DataRange, EntailError> {
        self.poll()?;
        if path.contains(&node) {
            return Err(EntailError::Parse("cyclic OWL data range".to_owned()));
        }
        if path.len() >= MAX_EXPRESSION_DEPTH {
            return Err(EntailError::Parse(format!(
                "OWL data range nests deeper than {MAX_EXPRESSION_DEPTH}"
            )));
        }
        path.push(node);
        let decoded = self.data_range_inner(node, path);
        path.pop();
        decoded
    }

    /// [`data_range`](Self::data_range)'s body, with `node` already on `path`.
    ///
    /// Split out so that the guard above has exactly one place to push and pop, rather than
    /// one per early return of a function that has eight.
    fn data_range_inner(&self, node: u32, path: &mut Vec<u32>) -> Result<DataRange, EntailError> {
        if node == self.v.literal {
            return Ok(DataRange::Any);
        }
        if let TermValue::Iri(iri) = self.interner.value(node) {
            if let Some(datatype) = XsdDatatype::from_iri(iri) {
                return Ok(DataRange::Datatype(datatype));
            }
            // A datatype NAME whose value space this layer does not model: `owl:real`,
            // `owl:rational`, `xsd:anyURI`, a caller's own `rdfs:Datatype`.
            return Ok(DataRange::Opaque);
        }
        if let Some(base) = self.get(node, self.v.on_datatype) {
            return self.datatype_restriction(node, base);
        }
        if let Some(inner) = self.get(node, self.v.datatype_complement) {
            let inner = self.data_range(inner, path)?;
            return Ok(DataRange::Not(Box::new(inner)));
        }
        if let Some(head) = self.get(node, self.v.intersection) {
            return Ok(DataRange::And(self.data_range_list(head, path)?));
        }
        if let Some(head) = self.get(node, self.v.union) {
            return Ok(DataRange::Or(self.data_range_list(head, path)?));
        }
        if let Some(head) = self.get(node, self.v.one_of) {
            return self.data_one_of(head);
        }
        // An n-ary data range (`owl:onProperties` with `owl:onDataRange`) — and any datatype
        // node carrying nothing this layer reads. OWL 2 defines no n-ary datatype at all, so
        // there is no datatype map entry to decide such a range against.
        Ok(DataRange::Opaque)
    }

    /// `owl:onDatatype` + `owl:withRestrictions`: a base datatype narrowed by facets.
    fn datatype_restriction(&self, node: u32, base: u32) -> Result<DataRange, EntailError> {
        let Some(base) = (match self.interner.value(base) {
            TermValue::Iri(iri) => XsdDatatype::from_iri(iri),
            _ => None,
        }) else {
            return Ok(DataRange::Opaque);
        };
        let Some(head) = self.get(node, self.v.with_restrictions) else {
            // A restriction with no facet list is the base datatype itself.
            return Ok(DataRange::Datatype(base));
        };
        match self.facet_list(head)? {
            Some(facets) => Ok(DataRange::Restriction { base, facets }),
            None => Ok(DataRange::Opaque),
        }
    }

    /// Walk an RDF list of data ranges.
    fn data_range_list(
        &self,
        head: u32,
        path: &mut Vec<u32>,
    ) -> Result<Vec<DataRange>, EntailError> {
        let members = self.node_list(head)?;
        members
            .into_iter()
            .map(|member| self.data_range(member, path))
            .collect()
    }

    /// `DataOneOf`: an `owl:oneOf` collection of LITERALS.
    ///
    /// A member that is not a literal, or whose value this layer cannot examine, makes the
    /// whole enumeration opaque — an enumeration with a member missing is a SMALLER set, and
    /// a smaller set under a complement is a larger one, so neither direction is sound.
    fn data_one_of(&self, head: u32) -> Result<DataRange, EntailError> {
        let members = self.node_list(head)?;
        let mut values = Vec::with_capacity(members.len());
        for member in members {
            self.poll()?;
            match data::literal_value(self.interner.value(member)) {
                Some(LiteralValue::Value(value)) => values.push(value),
                // An ill-typed member denotes nothing, so it contributes nothing: the
                // enumeration is exactly the remaining values.
                Some(LiteralValue::IllTyped) => {}
                _ => return Ok(DataRange::Opaque),
            }
        }
        Ok(DataRange::OneOf(values))
    }

    /// Decode an `owl:withRestrictions` collection, or `None` when a cell carries a facet this
    /// layer decides nothing about (or no facet at all).
    fn facet_list(&self, head: u32) -> Result<Option<Vec<Facet>>, EntailError> {
        let index = self.index;
        let cells = self.node_list(head)?;
        let mut out = Vec::with_capacity(cells.len());
        for cell in cells {
            self.poll()?;
            let Some(predicates) = index.get(&cell) else {
                return Ok(None);
            };
            let mut stated = false;
            for (predicate, objects) in predicates {
                self.poll()?;
                let Some(slot) = self.facet_slot(*predicate) else {
                    continue;
                };
                stated = true;
                let Some(&value) = objects.first() else {
                    return Ok(None);
                };
                match self.facet(slot, value) {
                    Some(facet) => out.push(facet),
                    None => return Ok(None),
                }
            }
            if !stated {
                return Ok(None);
            }
        }
        Ok(Some(out))
    }

    /// Which constraining facet `predicate` states, if any.
    fn facet_slot(&self, predicate: u32) -> Option<FacetSlot> {
        self.v
            .facets
            .iter()
            .find(|&&(facet, _)| facet == predicate)
            .map(|&(_, slot)| slot)
    }

    /// One facet, from its slot and its literal value; `None` when it cannot be read exactly.
    fn facet(&self, slot: FacetSlot, value: u32) -> Option<Facet> {
        let TermValue::Literal {
            lexical_form,
            datatype,
            language: None,
            ..
        } = self.interner.value(value)
        else {
            return None;
        };
        match slot {
            FacetSlot::Length | FacetSlot::MinLength | FacetSlot::MaxLength => {
                let length = lexical_form.trim().parse::<u64>().ok()?;
                Some(match slot {
                    FacetSlot::Length => Facet::Length(length),
                    FacetSlot::MinLength => Facet::MinLength(length),
                    _ => Facet::MaxLength(length),
                })
            }
            FacetSlot::MinInclusive
            | FacetSlot::MaxInclusive
            | FacetSlot::MinExclusive
            | FacetSlot::MaxExclusive => {
                let bound = purrdf_xsd::parse_by_iri(lexical_form, datatype).ok()??;
                Some(match slot {
                    FacetSlot::MinInclusive => Facet::MinInclusive(bound),
                    FacetSlot::MaxInclusive => Facet::MaxInclusive(bound),
                    FacetSlot::MinExclusive => Facet::MinExclusive(bound),
                    _ => Facet::MaxExclusive(bound),
                })
            }
            FacetSlot::Undecided => None,
        }
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
        if let Some(lit) = self.get(node, self.v.min_qcard) {
            let n = self.card(lit)?;
            let c = self.qualified_filler(node)?;
            self.counted_roles.insert(role);
            return Ok(Concept::Min(n, role, Box::new(c)));
        }
        if let Some(lit) = self.get(node, self.v.max_qcard) {
            let n = self.card(lit)?;
            let c = self.qualified_filler(node)?;
            self.counted_roles.insert(role);
            return Ok(Concept::Max(n, role, Box::new(c)));
        }
        if let Some(lit) = self.get(node, self.v.qcard) {
            let n = self.card(lit)?;
            let c = self.qualified_filler(node)?;
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

    /// The filler of a qualified cardinality restriction: `owl:onClass` for an object
    /// property, `owl:onDataRange` for a data property.
    ///
    /// The two are the SAME position in two domains, so they resolve through one function and
    /// both reach [`CeExtractor::expr`] — which is what makes `≥n p.DR` over a data range an
    /// ordinary counting restriction whose filler happens to be a concrete-domain leaf.
    fn qualified_filler(&mut self, node: u32) -> Result<Concept, EntailError> {
        let filler = self
            .get(node, self.v.on_class)
            .or_else(|| self.get(node, self.v.on_data_range))
            .ok_or_else(|| {
                EntailError::Parse(
                    "qualified cardinality without owl:onClass or owl:onDataRange".to_owned(),
                )
            })?;
        self.expr(filler)
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
    ///
    /// `u32::MAX` itself is REFUSED, not represented: both calculi need `n + 1` (the
    /// NNF of `¬(≤n r.C)` is `≥(n+1) r.C`, and the schematic counting clause fires on
    /// `n + 1` successors), so a bound that cannot be incremented in this representation
    /// would wrap in release builds — turning `owl:maxCardinality u32::MAX` over an
    /// individual with NO successors into a derived `false` under a certificate reading
    /// `decided` — and panic in debug builds. A legal-but-unrepresentable input is a
    /// named hard error, never a wrong verdict.
    fn card(&self, lit: u32) -> Result<u32, EntailError> {
        match self.interner.value(lit) {
            TermValue::Literal { lexical_form, .. } => {
                let n = lexical_form.trim().parse::<u32>().map_err(|_| {
                    EntailError::Parse(format!("non-integer cardinality literal: {lexical_form:?}"))
                })?;
                if n == u32::MAX {
                    return Err(EntailError::Parse(format!(
                        "cardinality {n} exceeds this reasoner's representable bound \
                         ({} is the largest supported cardinality): refusing rather \
                         than deciding wrongly",
                        u32::MAX - 1
                    )));
                }
                Ok(n)
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
            self.poll()?;
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

/// Poll the caller's latching cancellation signal at a reverse-mapping work boundary.
fn poll(stop: Option<&dyn StopSignal>) -> Result<(), EntailError> {
    if stop.is_some_and(StopSignal::stopped) {
        Err(EntailError::Stopped)
    } else {
        Ok(())
    }
}

/// Parse `ds`'s default graph into a knowledge base, polling `stop` throughout the
/// dataset-sized reverse-mapping passes.
///
/// # Errors
///
/// [`EntailError::Parse`] on a malformed class-expression graph (a restriction with no
/// `owl:onProperty`, a non-integer cardinality literal, a broken RDF list, …).
pub(crate) fn build_until(
    ds: &RdfDataset,
    stop: Option<&dyn StopSignal>,
) -> Result<Kb, EntailError> {
    poll(stop)?;
    let mut interner = Interner::default();
    let v = Vocab::intern(&mut interner);
    let mut table = ConceptTable::default();
    let top = table.top();
    let bottom = table.bottom();

    // Intern every default-graph triple and build the subject index.
    let mut index: TripleIndex = BTreeMap::new();
    let mut triples: Vec<(u32, u32, u32)> = Vec::new();
    for q in ds.quads() {
        poll(stop)?;
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
    let mut ranges = DataRangeTable::default();
    {
        let mut ce = CeExtractor::new_until(&index, &interner, &v, &mut ranges, stop);
        // The n-ary axioms are stated as a node carrying a list or a reification, so they
        // are read from the node rather than from any one of its triples.
        for (&node, preds) in &index {
            ce.poll()?;
            for &axiom_class in preds.get(&v.ty).map_or(&[][..], Vec::as_slice) {
                ce.poll()?;
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
            ce.poll()?;
            axiom(&mut ce, &mut table, &mut acc, &v, &interner, spo)?;
        }
        acc.boundaries.extend(ce.boundaries().iter().copied());
        // OWL 2 DL forbids a number restriction over a NON-SIMPLE role. The transitivity
        // axioms are only all known now, so the condition is checked here rather than
        // while the restriction was being decoded.
        for &role in ce.counted_roles() {
            ce.poll()?;
            if is_non_simple(role, &acc, stop)? {
                acc.boundaries.insert(Construct::NonSimpleRole);
                break;
            }
        }
    }

    // Every literal that reaches the knowledge base carries its VALUE into the completion
    // graph, and the literals' value classes decide which of them are one element of the data
    // domain and which are provably different ones.
    let literal_class = register_literals(&interner, &mut table, &mut ranges, &mut acc, stop)?;
    // A data range this layer cannot decide EXACTLY is a reported boundary rather than a
    // silent weakening. The predicate is `purrdf-xsd`'s own, so the boundary and the decision
    // procedure cannot drift apart.
    if !ranges.exactly_decided() {
        acc.boundaries.insert(Construct::DataRange);
    }

    table.finalize_until(|| poll(stop))?;
    poll(stop)?;
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
        data_ranges: ranges,
        literal_class,
        boundaries: acc.boundaries,
        // The reverse mapping is not the place a caller's stop signal is named:
        // `Kb::with_stop` installs it on the knowledge base the caller then reasons over.
        stop: None,
    })
}

/// Give every literal that reaches the knowledge base its VALUE, and return the value-class
/// partition of those literals.
///
/// A literal is a term of the completion graph — the object of a data-property assertion, a
/// member of an `owl:hasValue` or `owl:oneOf` nominal — and until its value is known it is an
/// opaque abstract symbol, which is the reading that makes `"5"^^xsd:integer` and
/// `"5.0"^^xsd:decimal` two things and lets a functional data property hold both. Two facts
/// are recorded here, and each is separately load-bearing:
///
/// 1. the singleton data range `{value}`, asserted on the literal's own node, so that a
///    `∀p.DR` or `∃p.DR` reaching the literal is DECIDED against the value the ontology
///    actually stated rather than only against the range's own emptiness;
/// 2. the literal's VALUE CLASS, which is what makes two literals one element of the data
///    domain or two — see [`Kb::literal_class`](crate::owl_dl::Kb::literal_class).
///
/// A literal whose lexical form is not in its datatype's lexical space is given the EMPTY
/// range, which is how OWL 2's rule that such an ontology is inconsistent reaches the tableau
/// through the same clash as every other empty range instead of a special case. A literal
/// whose datatype is outside the modelled value space is given nothing at all — no range and
/// no class — and raises the data-range boundary, because whether it is even well-typed, let
/// alone which other literals it agrees with, is not decided here.
///
/// The literals considered are exactly the ones an axiom reaches: the objects of ingested role
/// assertions, and the members of every interned nominal. A literal that only ever appears in
/// an annotation is not one of them, and OWL 2's Direct Semantics agrees — an annotation
/// constrains no interpretation, so its literal need not even denote.
fn register_literals(
    interner: &Interner,
    table: &mut ConceptTable,
    ranges: &mut DataRangeTable,
    acc: &mut Accums,
    stop: Option<&dyn StopSignal>,
) -> Result<BTreeMap<u32, u32>, EntailError> {
    let mut terms = BTreeSet::new();
    for &(_, _, object) in &acc.abox_roles {
        poll(stop)?;
        terms.insert(object);
    }
    for id in 0..table.len() {
        poll(stop)?;
        if let Decomp::Nominal(members) | Decomp::NegNominal(members) =
            table.decomp(u32::try_from(id).expect("concept id fits u32"))
        {
            for &member in members {
                poll(stop)?;
                terms.insert(member);
            }
        }
    }
    let mut described = Vec::new();
    for term in terms {
        poll(stop)?;
        if let Some(value) = data::literal_value(interner.value(term)) {
            described.push((term, value));
        }
    }
    if described.is_empty() {
        return Ok(BTreeMap::new());
    }
    let classes = data::literal_classes_until(&described, || poll(stop))?;
    for (term, value) in &described {
        poll(stop)?;
        let range = match value {
            LiteralValue::Value(value) => DataRange::OneOf(vec![value.clone()]),
            LiteralValue::IllTyped => DataRange::OneOf(Vec::new()),
            LiteralValue::TermIdentified | LiteralValue::Unmodelled => continue,
        };
        let id = ranges.intern(range);
        let concept = table.intern(Concept::Data(id));
        acc.abox_types.push((*term, concept));
    }
    if classes.any_unmodelled {
        acc.boundaries.insert(Construct::DataRange);
    }
    Ok(classes.class_of)
}

/// Whether `role` is NON-SIMPLE — transitive, or the super-role of a transitive role.
///
/// A role's inverse is simple exactly when the role is, so `Role::Inv(p)` is decided on
/// `p`. The sub-role closure walks [`Accums::role_sub`], which is the same hierarchy the
/// tableau's `achievers` walks, so the two agree on what a role's extension contains.
fn is_non_simple(
    role: Role,
    acc: &Accums,
    stop: Option<&dyn StopSignal>,
) -> Result<bool, EntailError> {
    let (Role::Named(head) | Role::Inv(head)) = role;
    let mut seen: BTreeSet<u32> = BTreeSet::new();
    let mut stack = vec![head];
    while let Some(current) = stack.pop() {
        poll(stop)?;
        if !seen.insert(current) {
            continue;
        }
        if acc.transitive.contains(&current) {
            return Ok(true);
        }
        if let Some(subs) = acc.role_sub.get(&current) {
            stack.extend(subs.iter().copied());
        }
    }
    Ok(false)
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
    ce.poll()?;
    if axiom_class == v.all_different {
        // Both spellings, and both are read: OWL 2 writes `owl:members`, OWL 1 wrote
        // `owl:distinctMembers`, and an ontology may carry either.
        for list_property in [v.members, v.distinct_members] {
            ce.poll()?;
            let Some(head) = first_object(ce, node, list_property) else {
                continue;
            };
            let members = ce.node_list(head)?;
            for (index, &left) in members.iter().enumerate() {
                for &right in &members[index + 1..] {
                    ce.poll()?;
                    acc.different_from.push((left, right));
                }
            }
            for member in members {
                ce.poll()?;
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
                    ce.poll()?;
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
                    ce.poll()?;
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
    ce.poll()?;
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
    ce.poll()?;
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
        role_or_boundary(acc, interner, v, s, p, o);
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
            ce.poll()?;
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
/// * a CONSTRAINING FACET is a component of the datatype restriction that carries it, read by
///   the data-range decoder from the restriction node rather than one triple at a time. The
///   facet IRIs sit in the XML Schema namespace, which the OWL-2-RDF mapping does not reserve,
///   so without naming them here a facet would fall through to the last arm and put an axiom's
///   own scaffolding — an `owl:withRestrictions` list cell — in the ABox as an individual;
/// * a term outside the reserved namespaces is the caller's own vocabulary and becomes a
///   role assertion.
fn role_or_boundary(acc: &mut Accums, interner: &Interner, v: &Vocab, s: u32, p: u32, o: u32) {
    if v.facets.iter().any(|&(facet, _)| facet == p) {
        return;
    }
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
