// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Oxigraph-free RDF query surface for the slice emitters and linters.
//!
//! The slice crate used to parse Turtle/N-Triples into an `oxigraph::store::Store`
//! and pattern-match it. Every store/term type is now native: parsing folds the
//! RDF 1.2 statement layer into the frozen [`purrdf::RdfDataset`] IR via the
//! native codecs ([`purrdf::parse_dataset`]), pattern queries route through the
//! IR's [`purrdf::DatasetView::quads_for_pattern`] (an indexed lookup), and the
//! object terms surface as the native [`Object`] value model.
//!
//! Multi-file accumulation (the merged DSL / ontology stores) uses
//! [`purrdf::RdfDatasetBuilder::push_dataset`], which standardizes blank nodes
//! apart per source dataset (a fresh [`purrdf::BlankScope`] per file, C0.2) — the
//! native equivalent of the old per-source blank-prefix scoping. Quads dedup at
//! freeze (C0.5), matching the old `Store::insert` set semantics.
//!
//! All committed byte output here is keyed on IRI subjects/predicates/objects and
//! literal values (blank-insensitive), and the one blank-bearing consumer
//! (`compute_semantic_digest`) canonicalizes via RDFC-1.0, so the blank label
//! *spelling* never reaches a committed artifact.

use std::path::Path;

use purrdf::{
    DatasetView, GraphMatch, NativeRdfFormat, RdfDataset, RdfDatasetBuilder, RdfQuad, RdfTerm,
    RdfTextDirection, TermId, TermRef, TermValue, parse_dataset,
};

use crate::error::SliceError;

/// The `rdf:reifies` predicate IRI — re-materialized when flattening the RDF 1.2
/// statement overlay back to plain quads for canonicalization.
const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

// ── Native NamedNode ───────────────────────────────────────────────────────────

/// An absolute IRI in term position — the oxigraph-free replacement for
/// `oxigraph::model::NamedNode` across the slice crate.
///
/// `new` validates the IRI through the native `purrdf-iri` parser (via the
/// `purrdf-sparql-algebra` validator, the same RFC-3987 check oxigraph applied), so
/// the `Ok`/`Err` discrimination at the slice's IRI-construction sites is preserved.
/// `Ord`/`Hash` are lexical on the IRI string, matching oxigraph's `NamedNode`
/// ordering (it orders by the IRI string), so every `BTreeMap`/`BTreeSet` keyed on a
/// `NamedNode` keeps the same iteration order.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NamedNode {
    iri: String,
}

impl NamedNode {
    /// Validate and wrap an absolute IRI, returning `Err` on a malformed or relative
    /// IRI (term-position IRIs must be absolute) — mirrors `oxigraph::model::NamedNode::new`.
    pub fn new(iri: impl Into<String>) -> Result<Self, SliceError> {
        let iri = iri.into();
        purrdf_sparql_algebra::NamedNode::new(iri.clone())
            .map_err(|e| SliceError::Parse(format!("invalid IRI {iri}: {e}")))?;
        Ok(Self { iri })
    }

    /// Wrap an IRI without validation (a static/already-validated IRI).
    pub fn new_unchecked(iri: impl Into<String>) -> Self {
        Self { iri: iri.into() }
    }

    /// The IRI lexical form.
    pub fn as_str(&self) -> &str {
        &self.iri
    }
}

impl core::fmt::Debug for NamedNode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "<{}>", self.iri)
    }
}

// ── Native object value model ───────────────────────────────────────────────────

/// The kinds of RDF subject a quad can carry (named node, blank node, OR a quoted
/// triple term in subject position, RDF 1.2), surfaced from the native IR. Mirrors
/// the `oxigraph::model::NamedOrBlankNode` discrimination the slice linters relied
/// on, extended with the RDF 1.2 triple-term arm.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Subject {
    /// An IRI subject.
    Named(String),
    /// A blank-node subject, by its (scope-qualified) label.
    Blank(String),
    /// A quoted triple term (RDF 1.2) in subject position, resolved to its interior.
    Triple(Box<TripleTerm>),
}

/// An RDF object term, surfaced from the native IR as an owned value — the
/// oxigraph-free replacement for `oxigraph::model::Term` in object position.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Object {
    /// An IRI object.
    Named(String),
    /// A blank-node object, by its (scope-qualified) label.
    Blank(String),
    /// A literal, carried at full fidelity: its lexical form, its datatype IRI, an
    /// optional language tag, and an optional RDF 1.2 base direction.
    Literal {
        /// The lexical form (the `.value()` of the old oxigraph literal).
        value: String,
        /// The datatype IRI (e.g. `…#string`, `…#integer`).
        datatype: String,
        /// The BCP-47 language tag, if this is a language-tagged literal.
        language: Option<String>,
        /// The RDF 1.2 base direction, if this is a directional language-tagged
        /// literal.
        direction: Option<RdfTextDirection>,
    },
    /// A quoted triple term (RDF 1.2), resolved to its interior.
    Triple(Box<TripleTerm>),
}

/// The interior of a quoted triple term (RDF 1.2): its subject, predicate IRI, and
/// object, each fully resolved through the same helpers as top-level terms (so a
/// triple term nested in the subject or object resolves recursively).
///
/// Acyclicity is guaranteed by frozen-dataset validation — a triple term can never
/// contain itself, transitively — so this recursive resolution always terminates
/// and needs no depth guard.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct TripleTerm {
    /// The subject of the quoted triple.
    pub subject: Subject,
    /// The predicate IRI of the quoted triple (always an IRI in well-formed RDF).
    pub predicate: String,
    /// The object of the quoted triple.
    pub object: Object,
}

impl Object {
    /// The IRI, if this object is a named node (`oxigraph` `Term::NamedNode` arm).
    pub fn as_named(&self) -> Option<&str> {
        match self {
            Self::Named(iri) => Some(iri.as_str()),
            _ => None,
        }
    }

    /// The quoted triple interior, if this object is a triple term (RDF 1.2).
    pub fn as_triple(&self) -> Option<&TripleTerm> {
        match self {
            Self::Triple(t) => Some(t),
            _ => None,
        }
    }
}

// ── Resolution helpers ──────────────────────────────────────────────────────────

fn subject_of(ds: &RdfDataset, id: TermId) -> Option<Subject> {
    match ds.resolve(id) {
        TermRef::Iri(iri) => Some(Subject::Named(iri.to_owned())),
        TermRef::Blank { label, scope } => {
            Some(Subject::Blank(scope.qualify_label(label).into_owned()))
        }
        TermRef::Triple { s, p, o } => Some(Subject::Triple(Box::new(triple_term_of(ds, s, p, o)))),
        // A literal cannot stand in subject position in well-formed RDF; the slice
        // never queries on one, so treat it as "no subject term".
        TermRef::Literal { .. } => None,
    }
}

fn object_of(ds: &RdfDataset, id: TermId) -> Object {
    match ds.resolve(id) {
        TermRef::Iri(iri) => Object::Named(iri.to_owned()),
        TermRef::Blank { label, scope } => Object::Blank(scope.qualify_label(label).into_owned()),
        TermRef::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => Object::Literal {
            value: lexical.to_owned(),
            datatype: iri_text_of(ds, datatype),
            language: language.map(str::to_owned),
            direction,
        },
        TermRef::Triple { s, p, o } => Object::Triple(Box::new(triple_term_of(ds, s, p, o))),
    }
}

/// Resolve a triple term's `(s, p, o)` component ids to a [`TripleTerm`], recursing
/// through [`subject_of`]/[`object_of`] for nested triple terms. A subject that
/// resolves to a literal (malformed) falls back to its lexical text as a named
/// subject rather than being dropped.
fn triple_term_of(ds: &RdfDataset, s: TermId, p: TermId, o: TermId) -> TripleTerm {
    TripleTerm {
        subject: subject_of(ds, s).unwrap_or_else(|| Subject::Named(iri_text_of(ds, s))),
        predicate: iri_text_of(ds, p),
        object: object_of(ds, o),
    }
}

/// The IRI text of a term, resolved defensively. A triple-term predicate or a
/// literal datatype is always an IRI in well-formed RDF; a non-IRI term (malformed)
/// falls back to its best available string form rather than panicking.
fn iri_text_of(ds: &RdfDataset, id: TermId) -> String {
    match ds.resolve(id) {
        TermRef::Iri(iri) => iri.to_owned(),
        TermRef::Blank { label, scope } => scope.qualify_label(label).into_owned(),
        TermRef::Literal { lexical, .. } => lexical.to_owned(),
        TermRef::Triple { .. } => String::new(),
    }
}

/// Resolve an IRI query term to its dataset-local id, without minting.
fn iri_id(ds: &RdfDataset, iri: &str) -> Option<TermId> {
    ds.term_id_by_value(&TermValue::iri(iri))
}

/// Resolve a [`Subject`] to its dataset-local id, without minting. A triple-term
/// subject has no interned value handle to reconstruct here, so it yields `None`
/// (its callers treat that as an empty match).
fn subject_term_id(ds: &RdfDataset, subject: &Subject) -> Option<TermId> {
    match subject {
        Subject::Named(iri) => ds.term_id_by_value(&TermValue::iri(iri.clone())),
        Subject::Blank(label) => ds.term_id_by_value(&TermValue::blank(label.clone())),
        Subject::Triple(_) => None,
    }
}

// ── Dataset wrapper ─────────────────────────────────────────────────────────────

/// A frozen RDF dataset plus the IRI-pattern query surface the slice emitters and
/// linters use. Wraps [`purrdf::RdfDataset`]; the query helpers resolve a query
/// IRI to a dataset-local term id via [`purrdf::RdfDataset::term_id_by_value`]
/// and pattern-scan via the indexed [`purrdf::DatasetView::quads_for_pattern`].
#[derive(Debug)]
pub struct Dataset {
    ds: RdfDataset,
}

impl Dataset {
    /// Wrap a frozen [`RdfDataset`].
    pub fn from_dataset(ds: RdfDataset) -> Self {
        Self { ds }
    }

    /// Wrap a freshly-frozen `Arc<RdfDataset>` (e.g. straight off a builder),
    /// unwrapping the unique allocation into an owned dataset.
    pub fn from_frozen(ds: std::sync::Arc<RdfDataset>) -> Self {
        Self {
            ds: std::sync::Arc::try_unwrap(ds).unwrap_or_else(|arc| owned_clone_of(&arc)),
        }
    }

    /// The underlying frozen dataset.
    pub fn inner(&self) -> &RdfDataset {
        &self.ds
    }

    /// Consume the wrapper, yielding the underlying frozen dataset.
    pub fn into_inner(self) -> RdfDataset {
        self.ds
    }

    /// The number of quads in the dataset.
    pub fn quad_count(&self) -> usize {
        self.ds.quad_count()
    }

    /// A cursor over one graph selection ([`GraphSel`]) of this dataset. The whole
    /// graph-parameterized nav surface lives on the returned [`GraphView`]; the
    /// `Dataset` nav methods are thin wrappers over `graph(GraphSel::Default)`.
    ///
    /// A [`GraphSel::Named`] whose IRI is interned nowhere in this dataset yields a
    /// view whose every query is empty (never an error) — absence is an empty match.
    pub fn graph(&self, sel: GraphSel<'_>) -> GraphView<'_> {
        let graph = match sel {
            GraphSel::Default => Some(GraphMatch::Default),
            GraphSel::Any => Some(GraphMatch::Any),
            GraphSel::Named(iri) => iri_id(&self.ds, iri).map(GraphMatch::Named),
        };
        GraphView {
            ds: &self.ds,
            graph,
        }
    }

    /// All `(predicate-IRI, Object)` pairs of `<subject> ?p ?o` in the default graph,
    /// in dataset order. Used to scan a single subject's outgoing edges.
    pub fn predicate_objects_of(
        &self,
        subject_iri: &str,
    ) -> Result<Vec<(String, Object)>, SliceError> {
        self.graph(GraphSel::Default)
            .predicate_objects_of(subject_iri)
    }

    /// Parse RDF text `bytes` of `media_type` into a fresh dataset. `context` labels
    /// parse errors. Lenient on PurRDF's `@x-purrdf-*` language tags (the native codecs
    /// are fully lenient).
    pub fn parse(bytes: &[u8], media_type: &str, context: &str) -> Result<Self, SliceError> {
        let ds = parse_dataset(bytes, media_type, None)
            .map_err(|e| SliceError::Parse(format!("syntax error in {context}: {e}")))?;
        // `parse_dataset` returns an `Arc<RdfDataset>`; unwrap the fresh, uniquely-owned
        // allocation into an owned dataset (no clone — it is the sole reference).
        let ds = std::sync::Arc::try_unwrap(ds).unwrap_or_else(|arc| owned_clone_of(&arc));
        Ok(Self { ds })
    }

    /// Parse Turtle text into a fresh dataset.
    pub fn parse_turtle(bytes: &[u8], context: &str) -> Result<Self, SliceError> {
        Self::parse(bytes, NativeRdfFormat::Turtle.media_type(), context)
    }

    /// The IRI subjects of `?s a <type_iri>` in the default graph, in dataset order.
    pub fn subjects_of_type(&self, type_iri: &str) -> Result<Vec<String>, SliceError> {
        self.graph(GraphSel::Default).subjects_of_type(type_iri)
    }

    /// Every subject (named OR blank) of `?s a <type_iri>` in the default graph.
    pub fn subject_terms_of_type(&self, type_iri: &str) -> Result<Vec<Subject>, SliceError> {
        self.graph(GraphSel::Default)
            .subject_terms_of_type(type_iri)
    }

    /// All object terms of `<subject> <pred> ?o` in the default graph, where the
    /// subject is named.
    pub fn objects(&self, subject_iri: &str, pred: &str) -> Result<Vec<Object>, SliceError> {
        self.graph(GraphSel::Default).objects(subject_iri, pred)
    }

    /// All object terms of `<subject> <pred> ?o` in the default graph (subject may be
    /// a blank node).
    pub fn objects_of_subject(
        &self,
        subject: &Subject,
        pred: &str,
    ) -> Result<Vec<Object>, SliceError> {
        self.graph(GraphSel::Default)
            .objects_of_subject(subject, pred)
    }

    /// The first object of `<subject> <pred> ?o` in the default graph, or `None`.
    pub fn first_object(
        &self,
        subject_iri: &str,
        pred: &str,
    ) -> Result<Option<Object>, SliceError> {
        self.graph(GraphSel::Default)
            .first_object(subject_iri, pred)
    }

    /// The first IRI object of `<subject> <pred> ?o`, or `None` when the first object
    /// is not an IRI (mirrors `graph.value(...)` restricted to a URIRef).
    pub fn first_object_iri(
        &self,
        subject_iri: &str,
        pred: &str,
    ) -> Result<Option<String>, SliceError> {
        self.graph(GraphSel::Default)
            .first_object_iri(subject_iri, pred)
    }

    /// The first literal-object lexical value of `<subject> <pred> ?o`, or `None`.
    pub fn object_literal(
        &self,
        subject_iri: &str,
        pred: &str,
    ) -> Result<Option<String>, SliceError> {
        self.graph(GraphSel::Default)
            .object_literal(subject_iri, pred)
    }

    /// Every IRI object of `<subject> <pred> ?o`, in dataset order.
    pub fn object_iris(&self, subject_iri: &str, pred: &str) -> Result<Vec<String>, SliceError> {
        self.graph(GraphSel::Default).object_iris(subject_iri, pred)
    }

    /// Every IRI subject of `?s <pred> <object>` in the default graph.
    pub fn subjects_with_object(
        &self,
        pred: &str,
        object_iri: &str,
    ) -> Result<Vec<String>, SliceError> {
        self.graph(GraphSel::Default)
            .subjects_with_object(pred, object_iri)
    }

    /// Every `(subject-IRI, object-IRI)` pair of `?s <pred> ?o` in the default graph
    /// where both are named nodes.
    pub fn subject_object_iri_pairs(
        &self,
        pred: &str,
    ) -> Result<Vec<(String, String)>, SliceError> {
        self.graph(GraphSel::Default).subject_object_iri_pairs(pred)
    }

    /// Whether `<subject> a <type_iri>` holds in the default graph.
    pub fn has_type(&self, subject_iri: &str, type_iri: &str) -> Result<bool, SliceError> {
        self.graph(GraphSel::Default)
            .has_type(subject_iri, type_iri)
    }

    /// Iterate every quad in the default graph as `(Subject, predicate-IRI, Object,
    /// graph-IRI-or-None)`. Used by the few consumers that scan the whole dataset.
    pub fn for_each_quad(&self, f: impl FnMut(Subject, &str, Object, Option<&str>)) {
        self.graph(GraphSel::Default).for_each_quad(f);
    }

    /// The members of the RDF Collection whose head is `head`, in the default graph.
    /// See [`GraphView::rdf_list`].
    pub fn rdf_list(&self, head: &Subject) -> Result<Vec<Object>, SliceError> {
        self.graph(GraphSel::Default).rdf_list(head)
    }

    /// The members of the RDF Collection OR Container whose head is `head`, in the
    /// default graph. See [`GraphView::members`].
    pub fn members(&self, head: &Subject) -> Result<Vec<Object>, SliceError> {
        self.graph(GraphSel::Default).members(head)
    }

    /// The IRIs of every named graph in this dataset (quad-bearing or explicitly
    /// declared empty), in the dataset's sorted, deduplicated named-graph order.
    /// Non-IRI graph names are skipped defensively.
    pub fn named_graph_iris(&self) -> Vec<String> {
        self.ds
            .named_graphs()
            .filter_map(|gid| match self.ds.resolve(gid) {
                TermRef::Iri(iri) => Some(iri.to_owned()),
                _ => None,
            })
            .collect()
    }

    /// The canonical N-Quads document (full W3C RDFC-1.0) of this dataset's quads,
    /// **flattened** — the RDF 1.2 statement overlay (reifier bindings + annotations)
    /// is re-materialized back into plain `rdf:reifies` / annotation triples BEFORE
    /// canonicalizing, with no overlay re-fold. This is byte-identical to the prior
    /// `purrdf::canonical_nquads` over a flat oxigraph quad set: both canonicalize
    /// the same flat triple set, so the semantic digest is preserved (the native
    /// folded `canonicalize` would instead emit reserved overlay sentinels).
    pub fn canonical_nquads_flat(&self) -> Result<String, SliceError> {
        let mut builder = RdfDatasetBuilder::new();
        for quad in self.flat_quads() {
            builder.push_owned_quad(&quad);
        }
        let frozen = builder
            .freeze()
            .map_err(|e| SliceError::Parse(format!("flatten for canonicalization: {e}")))?;
        Ok(purrdf::canonicalize(&frozen).nquads)
    }

    /// Flatten the dataset to the source-faithful plain-quad stream: base quads, then
    /// the re-materialized `rdf:reifies` reifier rows, then the annotation rows. The
    /// oxigraph-free twin of `purrdf::oxigraph::flat_rdf_quads_from_dataset`.
    fn flat_quads(&self) -> Vec<RdfQuad> {
        let mut quads: Vec<RdfQuad> = self.ds.owned_quads().collect();
        for reifier in self.ds.owned_reifiers() {
            let statement = RdfTerm::triple(reifier.statement.clone());
            quads.push(RdfQuad::new(
                reifier.reifier.clone(),
                RDF_REIFIES,
                statement,
            ));
        }
        for annotation in self.ds.owned_annotations() {
            quads.push(RdfQuad::new(
                annotation.reifier.clone(),
                annotation.predicate.clone(),
                annotation.object.clone(),
            ));
        }
        quads
    }

    /// Build a frozen [`RdfDataset`] from a flat owned-quad set (`push_owned_quad`,
    /// no overlay fold) — for callers needing an owned dataset reconstructed from
    /// arbitrary quads.
    pub fn from_owned_quads(quads: &[RdfQuad]) -> Result<Self, SliceError> {
        let mut builder = RdfDatasetBuilder::new();
        for quad in quads {
            builder.push_owned_quad(quad);
        }
        let frozen = builder
            .freeze()
            .map_err(|e| SliceError::Parse(format!("dataset freeze failed: {e}")))?;
        Ok(Self::from_frozen(frozen))
    }
}

// ── Graph selection + cursor ────────────────────────────────────────────────────

/// Which graph of a [`Dataset`] a [`GraphView`] scans.
#[derive(Clone, Copy, Debug)]
pub enum GraphSel<'a> {
    /// The default (unnamed) graph.
    Default,
    /// The named graph with this IRI. An IRI interned nowhere in the dataset yields
    /// an always-empty view (never an error).
    Named(&'a str),
    /// Every graph — the default graph and all named graphs.
    Any,
}

/// A cursor over one [`GraphSel`] of a [`Dataset`], carrying the whole
/// graph-parameterized IRI-pattern nav surface. Obtained from [`Dataset::graph`].
///
/// `graph` is `None` exactly when a [`GraphSel::Named`] names an IRI that is not
/// interned in the dataset: no quad can match it, so every query short-circuits to
/// an empty result. Otherwise it is the [`GraphMatch`] every `quads_for_pattern`
/// call scopes to.
#[derive(Debug)]
pub struct GraphView<'a> {
    ds: &'a RdfDataset,
    graph: Option<GraphMatch>,
}

impl GraphView<'_> {
    /// All `(predicate-IRI, Object)` pairs of `<subject> ?p ?o` in this graph, in
    /// dataset order. Used to scan a single subject's outgoing edges.
    pub fn predicate_objects_of(
        &self,
        subject_iri: &str,
    ) -> Result<Vec<(String, Object)>, SliceError> {
        let (Some(graph), Some(s)) = (self.graph, iri_id(self.ds, subject_iri)) else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for q in self.ds.quads_for_pattern(Some(s), None, None, graph) {
            let TermRef::Iri(p) = self.ds.resolve(q.p) else {
                continue;
            };
            out.push((p.to_owned(), object_of(self.ds, q.o)));
        }
        Ok(out)
    }

    /// The IRI subjects of `?s a <type_iri>` in this graph, in dataset order.
    pub fn subjects_of_type(&self, type_iri: &str) -> Result<Vec<String>, SliceError> {
        let mut out = Vec::new();
        for subject in self.subject_terms_of_type(type_iri)? {
            if let Subject::Named(iri) = subject {
                out.push(iri);
            }
        }
        Ok(out)
    }

    /// Every subject (named OR blank) of `?s a <type_iri>` in this graph.
    pub fn subject_terms_of_type(&self, type_iri: &str) -> Result<Vec<Subject>, SliceError> {
        let (Some(graph), Some(p), Some(o)) = (
            self.graph,
            iri_id(self.ds, RDF_TYPE),
            iri_id(self.ds, type_iri),
        ) else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for q in self.ds.quads_for_pattern(None, Some(p), Some(o), graph) {
            if let Some(s) = subject_of(self.ds, q.s) {
                out.push(s);
            }
        }
        Ok(out)
    }

    /// All object terms of `<subject> <pred> ?o` in this graph, where the subject is
    /// named.
    pub fn objects(&self, subject_iri: &str, pred: &str) -> Result<Vec<Object>, SliceError> {
        self.objects_of_subject(&Subject::Named(subject_iri.to_owned()), pred)
    }

    /// All object terms of `<subject> <pred> ?o` in this graph (subject may be a
    /// blank node).
    pub fn objects_of_subject(
        &self,
        subject: &Subject,
        pred: &str,
    ) -> Result<Vec<Object>, SliceError> {
        let (Some(graph), Some(s), Some(p)) = (
            self.graph,
            subject_term_id(self.ds, subject),
            iri_id(self.ds, pred),
        ) else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for q in self.ds.quads_for_pattern(Some(s), Some(p), None, graph) {
            out.push(object_of(self.ds, q.o));
        }
        Ok(out)
    }

    /// The first object of `<subject> <pred> ?o` in this graph, or `None`.
    pub fn first_object(
        &self,
        subject_iri: &str,
        pred: &str,
    ) -> Result<Option<Object>, SliceError> {
        Ok(self.objects(subject_iri, pred)?.into_iter().next())
    }

    /// The first IRI object of `<subject> <pred> ?o`, or `None` when the first object
    /// is not an IRI (mirrors `graph.value(...)` restricted to a URIRef).
    pub fn first_object_iri(
        &self,
        subject_iri: &str,
        pred: &str,
    ) -> Result<Option<String>, SliceError> {
        match self.first_object(subject_iri, pred)? {
            Some(Object::Named(iri)) => Ok(Some(iri)),
            _ => Ok(None),
        }
    }

    /// The first literal-object lexical value of `<subject> <pred> ?o`, or `None`.
    pub fn object_literal(
        &self,
        subject_iri: &str,
        pred: &str,
    ) -> Result<Option<String>, SliceError> {
        match self.first_object(subject_iri, pred)? {
            Some(Object::Literal { value, .. }) => Ok(Some(value)),
            _ => Ok(None),
        }
    }

    /// Every IRI object of `<subject> <pred> ?o`, in dataset order.
    pub fn object_iris(&self, subject_iri: &str, pred: &str) -> Result<Vec<String>, SliceError> {
        Ok(self
            .objects(subject_iri, pred)?
            .into_iter()
            .filter_map(|o| match o {
                Object::Named(iri) => Some(iri),
                _ => None,
            })
            .collect())
    }

    /// Every IRI subject of `?s <pred> <object>` in this graph.
    pub fn subjects_with_object(
        &self,
        pred: &str,
        object_iri: &str,
    ) -> Result<Vec<String>, SliceError> {
        let (Some(graph), Some(p), Some(o)) = (
            self.graph,
            iri_id(self.ds, pred),
            iri_id(self.ds, object_iri),
        ) else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for q in self.ds.quads_for_pattern(None, Some(p), Some(o), graph) {
            if let Some(Subject::Named(iri)) = subject_of(self.ds, q.s) {
                out.push(iri);
            }
        }
        Ok(out)
    }

    /// Every `(subject-IRI, object-IRI)` pair of `?s <pred> ?o` in this graph where
    /// both are named nodes.
    pub fn subject_object_iri_pairs(
        &self,
        pred: &str,
    ) -> Result<Vec<(String, String)>, SliceError> {
        let (Some(graph), Some(p)) = (self.graph, iri_id(self.ds, pred)) else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for q in self.ds.quads_for_pattern(None, Some(p), None, graph) {
            if let (Some(Subject::Named(s)), Object::Named(o)) =
                (subject_of(self.ds, q.s), object_of(self.ds, q.o))
            {
                out.push((s, o));
            }
        }
        Ok(out)
    }

    /// Whether `<subject> a <type_iri>` holds in this graph.
    pub fn has_type(&self, subject_iri: &str, type_iri: &str) -> Result<bool, SliceError> {
        let (Some(graph), Some(s), Some(p), Some(o)) = (
            self.graph,
            iri_id(self.ds, subject_iri),
            iri_id(self.ds, RDF_TYPE),
            iri_id(self.ds, type_iri),
        ) else {
            return Ok(false);
        };
        Ok(self
            .ds
            .quads_for_pattern(Some(s), Some(p), Some(o), graph)
            .next()
            .is_some())
    }

    /// Iterate every quad in this graph as `(Subject, predicate-IRI, Object,
    /// graph-IRI-or-None)`.
    pub fn for_each_quad(&self, mut f: impl FnMut(Subject, &str, Object, Option<&str>)) {
        let Some(graph) = self.graph else {
            return;
        };
        for q in self.ds.quads_for_pattern(None, None, None, graph) {
            let Some(s) = subject_of(self.ds, q.s) else {
                continue;
            };
            let TermRef::Iri(p) = self.ds.resolve(q.p) else {
                continue;
            };
            let o = object_of(self.ds, q.o);
            let g = q.g.and_then(|gid| match self.ds.resolve(gid) {
                TermRef::Iri(iri) => Some(iri),
                _ => None,
            });
            f(s, p, o, g);
        }
    }

    /// The members of the RDF Collection (`rdf:first`/`rdf:rest`/`rdf:nil`) whose
    /// head is `head`, in this graph, in list order. A head that does not resolve —
    /// or that is not a list — yields an empty `Vec`. A structurally malformed
    /// Collection is a [`SliceError::RdfList`].
    pub fn rdf_list(&self, head: &Subject) -> Result<Vec<Object>, SliceError> {
        let (Some(graph), Some(head_id)) = (self.graph, subject_term_id(self.ds, head)) else {
            return Ok(Vec::new());
        };
        let members = self
            .ds
            .rdf_list(head_id, graph)
            .map_err(SliceError::RdfList)?;
        Ok(members
            .into_iter()
            .map(|id| object_of(self.ds, id))
            .collect())
    }

    /// The members of the RDF Collection OR Container whose head is `head`, in this
    /// graph (shape-dispatched: an `rdf:first` head is walked as a Collection, an
    /// `rdf:Seq`/`rdf:Bag`/`rdf:Alt` or `rdf:_n` head as a Container). A head that
    /// does not resolve — or that is neither — yields an empty `Vec`.
    pub fn members(&self, head: &Subject) -> Result<Vec<Object>, SliceError> {
        let (Some(graph), Some(head_id)) = (self.graph, subject_term_id(self.ds, head)) else {
            return Ok(Vec::new());
        };
        let members = self
            .ds
            .members(head_id, graph)
            .map_err(SliceError::RdfList)?;
        Ok(members
            .into_iter()
            .map(|id| object_of(self.ds, id))
            .collect())
    }
}

/// Re-freeze a shared dataset into an owned one (only used on the rare path where the
/// parser's `Arc` is somehow shared; in practice `parse_dataset` returns a fresh,
/// uniquely-owned `Arc` so `try_unwrap` succeeds and this never runs).
fn owned_clone_of(ds: &RdfDataset) -> RdfDataset {
    let mut builder = RdfDatasetBuilder::new();
    builder.push_dataset(ds);
    std::sync::Arc::try_unwrap(builder.freeze().expect("re-freeze owned dataset"))
        .unwrap_or_else(|arc| owned_clone_of(&arc))
}

// ── Multi-file scoped accumulator ───────────────────────────────────────────────

/// A builder that accumulates several parsed RDF documents into ONE frozen dataset,
/// standardizing each document's blank nodes apart (a fresh [`purrdf::BlankScope`]
/// per `add`, C0.2) — the native equivalent of the old per-source blank-prefix
/// scoping. Quads dedup at [`freeze`](Self::freeze) (C0.5), matching the old
/// `Store::insert` set semantics.
#[derive(Debug)]
pub struct DatasetAccumulator {
    builder: RdfDatasetBuilder,
}

impl DatasetAccumulator {
    /// A fresh, empty accumulator.
    pub fn new() -> Self {
        Self {
            builder: RdfDatasetBuilder::new(),
        }
    }

    /// Parse `bytes` of `media_type` and merge them in under a fresh blank scope.
    /// `context` labels parse errors.
    pub fn add(&mut self, bytes: &[u8], media_type: &str, context: &str) -> Result<(), SliceError> {
        let parsed = parse_dataset(bytes, media_type, None)
            .map_err(|e| SliceError::Parse(format!("syntax error in {context}: {e}")))?;
        self.builder.push_dataset(&parsed);
        Ok(())
    }

    /// Parse Turtle `bytes` and merge them in under a fresh blank scope.
    pub fn add_turtle(&mut self, bytes: &[u8], context: &str) -> Result<(), SliceError> {
        self.add(bytes, NativeRdfFormat::Turtle.media_type(), context)
    }

    /// Merge an already-parsed [`Dataset`] in under a fresh blank scope.
    pub fn add_dataset(&mut self, dataset: &Dataset) {
        self.builder.push_dataset(dataset.inner());
    }

    /// Freeze the accumulated quads into a queryable [`Dataset`].
    pub fn freeze(self) -> Result<Dataset, SliceError> {
        let ds = self
            .builder
            .freeze()
            .map_err(|e| SliceError::Parse(format!("dataset freeze failed: {e}")))?;
        let ds = std::sync::Arc::try_unwrap(ds).unwrap_or_else(|arc| owned_clone_of(&arc));
        Ok(Dataset::from_dataset(ds))
    }
}

impl Default for DatasetAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

// ── Media-type routing ──────────────────────────────────────────────────────────

const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

/// Map a file extension to the native RDF media type, defaulting to Turtle.
///
/// Mirrors the historical extension routing (`.nt` → N-Triples, `.nq` → N-Quads,
/// `.trig` → TriG, everything else Turtle).
pub(crate) fn media_type_for_path(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("nt") => NativeRdfFormat::NTriples.media_type(),
        Some("nq") => NativeRdfFormat::NQuads.media_type(),
        Some("trig") => NativeRdfFormat::TriG.media_type(),
        _ => NativeRdfFormat::Turtle.media_type(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_node_validates_and_orders_lexically() {
        assert!(NamedNode::new("https://example.org/x").is_ok());
        assert!(NamedNode::new("not an iri").is_err());
        let a = NamedNode::new("https://example.org/a").unwrap();
        let b = NamedNode::new("https://example.org/b").unwrap();
        assert!(a < b);
        assert_eq!(a.as_str(), "https://example.org/a");
    }

    #[test]
    fn subjects_of_type_finds_named_subjects() {
        let ttl = "@prefix ex: <https://example.org/> .\n\
                   ex:a a ex:Thing .\n\
                   ex:b a ex:Thing .\n";
        let ds = Dataset::parse_turtle(ttl.as_bytes(), "test").unwrap();
        let mut subjects = ds.subjects_of_type("https://example.org/Thing").unwrap();
        subjects.sort();
        assert_eq!(
            subjects,
            vec![
                "https://example.org/a".to_owned(),
                "https://example.org/b".to_owned()
            ]
        );
    }

    #[test]
    fn object_literal_reads_the_lexical_value() {
        let ttl = "@prefix ex: <https://example.org/> .\n\
                   ex:a ex:label \"hello\" .\n";
        let ds = Dataset::parse_turtle(ttl.as_bytes(), "test").unwrap();
        assert_eq!(
            ds.object_literal("https://example.org/a", "https://example.org/label")
                .unwrap(),
            Some("hello".to_owned())
        );
    }

    #[test]
    fn named_graph_nav_scopes_to_the_selected_graph() {
        // A default-graph triple plus an RDF-1.2 annotation living in a named graph.
        let trig = "@prefix ex: <https://example.org/> .\n\
                    @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
                    ex:s ex:p ex:o .\n\
                    ex:g { ex:ax owl:annotatedSource ex:s . }\n";
        let ds =
            Dataset::parse(trig.as_bytes(), NativeRdfFormat::TriG.media_type(), "test").unwrap();

        // The named-graph edge is visible only under GraphSel::Named / Any.
        let named = ds.graph(GraphSel::Named("https://example.org/g"));
        assert_eq!(
            named
                .objects(
                    "https://example.org/ax",
                    "http://www.w3.org/2002/07/owl#annotatedSource",
                )
                .unwrap(),
            vec![Object::Named("https://example.org/s".to_owned())]
        );

        // The default graph does NOT see the named-graph annotation.
        let default = ds.graph(GraphSel::Default);
        assert!(
            default
                .objects(
                    "https://example.org/ax",
                    "http://www.w3.org/2002/07/owl#annotatedSource",
                )
                .unwrap()
                .is_empty()
        );
        // …but it does see the default-graph triple.
        assert_eq!(
            default
                .objects("https://example.org/s", "https://example.org/p")
                .unwrap(),
            vec![Object::Named("https://example.org/o".to_owned())]
        );

        // for_each_quad is graph-scoped: Default sees only the base triple, Named
        // only the annotation, Any sees both.
        let mut default_preds = Vec::new();
        ds.graph(GraphSel::Default)
            .for_each_quad(|_, p, _, _| default_preds.push(p.to_owned()));
        assert_eq!(default_preds, vec!["https://example.org/p".to_owned()]);

        let mut named_preds = Vec::new();
        ds.graph(GraphSel::Named("https://example.org/g"))
            .for_each_quad(|_, p, _, g| {
                named_preds.push((p.to_owned(), g.map(str::to_owned)));
            });
        assert_eq!(
            named_preds,
            vec![(
                "http://www.w3.org/2002/07/owl#annotatedSource".to_owned(),
                Some("https://example.org/g".to_owned())
            )]
        );

        let mut any_count = 0usize;
        ds.graph(GraphSel::Any)
            .for_each_quad(|_, _, _, _| any_count += 1);
        assert_eq!(any_count, 2);

        // The dataset reports its named graph IRI.
        assert_eq!(
            ds.named_graph_iris(),
            vec!["https://example.org/g".to_owned()]
        );
    }

    #[test]
    fn unresolved_named_graph_is_empty_not_error() {
        let trig = "@prefix ex: <https://example.org/> .\n\
                    ex:s ex:p ex:o .\n";
        let ds =
            Dataset::parse(trig.as_bytes(), NativeRdfFormat::TriG.media_type(), "test").unwrap();
        let missing = ds.graph(GraphSel::Named("https://example.org/does-not-exist"));
        assert!(
            missing
                .objects("https://example.org/s", "https://example.org/p")
                .unwrap()
                .is_empty()
        );
        let mut hit = false;
        missing.for_each_quad(|_, _, _, _| hit = true);
        assert!(!hit, "an unresolved named graph enumerates nothing");
    }

    #[test]
    fn triple_term_object_resolves_without_pattern_scan() {
        // A quoted triple term in object position (RDF 1.2 `<<( … )>>`).
        let ttl = "@prefix ex: <https://example.org/> .\n\
                   ex:s ex:p <<( ex:a ex:b ex:c )>> .\n";
        let ds = Dataset::parse_turtle(ttl.as_bytes(), "test").unwrap();
        let objects = ds
            .objects("https://example.org/s", "https://example.org/p")
            .unwrap();
        assert_eq!(objects.len(), 1);
        let tt = objects[0].as_triple().expect("object is a triple term");
        assert_eq!(
            tt.subject,
            Subject::Named("https://example.org/a".to_owned())
        );
        assert_eq!(tt.predicate, "https://example.org/b");
        assert_eq!(tt.object, Object::Named("https://example.org/c".to_owned()));
    }

    #[test]
    fn deeply_nested_triple_term_resolves_both_levels() {
        // A triple term whose object is itself a triple term.
        let ttl = "@prefix ex: <https://example.org/> .\n\
                   ex:s ex:p <<( ex:a ex:b <<( ex:d ex:e ex:f )>> )>> .\n";
        let ds = Dataset::parse_turtle(ttl.as_bytes(), "test").unwrap();
        let objects = ds
            .objects("https://example.org/s", "https://example.org/p")
            .unwrap();
        let outer = objects[0].as_triple().expect("outer triple term");
        assert_eq!(
            outer.subject,
            Subject::Named("https://example.org/a".to_owned())
        );
        let inner = outer.object.as_triple().expect("inner triple term");
        assert_eq!(
            inner.subject,
            Subject::Named("https://example.org/d".to_owned())
        );
        assert_eq!(inner.predicate, "https://example.org/e");
        assert_eq!(
            inner.object,
            Object::Named("https://example.org/f".to_owned())
        );
    }

    #[test]
    fn literal_fidelity_carries_datatype_and_language() {
        let ttl = "@prefix ex: <https://example.org/> .\n\
                   @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\
                   ex:s ex:count \"5\"^^xsd:integer ;\n\
                        ex:greeting \"bonjour\"@fr .\n";
        let ds = Dataset::parse_turtle(ttl.as_bytes(), "test").unwrap();

        let count = ds
            .first_object("https://example.org/s", "https://example.org/count")
            .unwrap()
            .unwrap();
        match count {
            Object::Literal {
                value, datatype, ..
            } => {
                assert_eq!(value, "5");
                assert!(
                    datatype.ends_with("#integer"),
                    "datatype was {datatype}, expected an xsd:integer IRI"
                );
            }
            other => panic!("expected a literal, got {other:?}"),
        }

        let greeting = ds
            .first_object("https://example.org/s", "https://example.org/greeting")
            .unwrap()
            .unwrap();
        match greeting {
            Object::Literal {
                value, language, ..
            } => {
                assert_eq!(value, "bonjour");
                assert_eq!(language, Some("fr".to_owned()));
            }
            other => panic!("expected a language-tagged literal, got {other:?}"),
        }
    }

    #[test]
    fn rdf_list_walks_a_collection_in_order() {
        let ttl = "@prefix ex: <https://example.org/> .\n\
                   ex:s ex:items ( ex:a ex:b ex:c ) .\n";
        let ds = Dataset::parse_turtle(ttl.as_bytes(), "test").unwrap();
        let head = ds
            .first_object("https://example.org/s", "https://example.org/items")
            .unwrap()
            .unwrap();
        // The list head is a blank node; turn it back into a Subject to walk it.
        let head_subject = match head {
            Object::Blank(label) => Subject::Blank(label),
            Object::Named(iri) => Subject::Named(iri),
            other => panic!("unexpected list head {other:?}"),
        };
        assert_eq!(
            ds.rdf_list(&head_subject).unwrap(),
            vec![
                Object::Named("https://example.org/a".to_owned()),
                Object::Named("https://example.org/b".to_owned()),
                Object::Named("https://example.org/c".to_owned()),
            ]
        );
        // `members` dispatches to the same Collection walk.
        assert_eq!(
            ds.members(&head_subject).unwrap(),
            ds.rdf_list(&head_subject).unwrap()
        );
    }

    /// Turn a resolved list-head [`Object`] back into the [`Subject`] that the
    /// `rdf_list`/`members` walkers accept (named or blank; a literal/triple head is
    /// never a valid list head in these fixtures).
    fn head_subject(head: Object) -> Subject {
        match head {
            Object::Blank(label) => Subject::Blank(label),
            Object::Named(iri) => Subject::Named(iri),
            other => panic!("unexpected list head {other:?}"),
        }
    }

    #[test]
    fn rdf_list_ordered_collection_via_head_object() {
        // acceptance 1: parse a `( … )` collection, feed its blank head back in.
        let ttl = "@prefix ex: <https://example.org/> .\n\
                   ex:s ex:list ( ex:a ex:b ex:c ) .\n";
        let ds = Dataset::parse_turtle(ttl.as_bytes(), "test").unwrap();
        let head = head_subject(
            ds.first_object("https://example.org/s", "https://example.org/list")
                .unwrap()
                .unwrap(),
        );
        assert!(
            matches!(head, Subject::Blank(_)),
            "list head is a blank node"
        );
        assert_eq!(
            ds.rdf_list(&head).unwrap(),
            vec![
                Object::Named("https://example.org/a".to_owned()),
                Object::Named("https://example.org/b".to_owned()),
                Object::Named("https://example.org/c".to_owned()),
            ]
        );
    }

    #[test]
    fn owl_shapes_materialize_members_in_order() {
        // acceptance 2: the list-encoded owl: constructs each materialize in order.
        let ttl = "@prefix ex: <https://example.org/> .\n\
                   @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
                   ex:U owl:unionOf ( ex:A ex:B ) .\n\
                   ex:I owl:intersectionOf ( ex:A ex:B ex:C ) .\n\
                   ex:M owl:members ( ex:x ex:y ) .\n\
                   ex:P owl:propertyChainAxiom ( ex:p1 ex:p2 ) .\n";
        let ds = Dataset::parse_turtle(ttl.as_bytes(), "test").unwrap();

        let named = |n: &str| Object::Named(format!("https://example.org/{n}"));
        let members_of = |subject: &str, pred: &str| {
            let head = head_subject(
                ds.first_object(
                    &format!("https://example.org/{subject}"),
                    &format!("http://www.w3.org/2002/07/owl#{pred}"),
                )
                .unwrap()
                .unwrap(),
            );
            ds.rdf_list(&head).unwrap()
        };

        assert_eq!(members_of("U", "unionOf"), vec![named("A"), named("B")]);
        assert_eq!(
            members_of("I", "intersectionOf"),
            vec![named("A"), named("B"), named("C")]
        );
        assert_eq!(members_of("M", "members"), vec![named("x"), named("y")]);
        assert_eq!(
            members_of("P", "propertyChainAxiom"),
            vec![named("p1"), named("p2")]
        );
    }

    #[test]
    fn rdf_list_nested_and_blank_members() {
        // acceptance 3: a list whose members include a blank node and a nested
        // list. Order is preserved; both blank members surface as Object::Blank.
        let ttl = "@prefix ex: <https://example.org/> .\n\
                   ex:s ex:list ( [] ( ex:x ex:y ) ex:z ) .\n";
        let ds = Dataset::parse_turtle(ttl.as_bytes(), "test").unwrap();
        let head = head_subject(
            ds.first_object("https://example.org/s", "https://example.org/list")
                .unwrap()
                .unwrap(),
        );
        let members = ds.rdf_list(&head).unwrap();
        assert_eq!(members.len(), 3);
        // Member 0 is the bare blank node, member 1 is the nested list's blank head.
        assert!(matches!(members[0], Object::Blank(_)), "bare blank member");
        assert!(
            matches!(members[1], Object::Blank(_)),
            "nested-list head is a blank member"
        );
        assert_eq!(
            members[2],
            Object::Named("https://example.org/z".to_owned())
        );
        // The nested list head itself walks to its own members, in order.
        let nested = head_subject(members[1].clone());
        assert_eq!(
            ds.rdf_list(&nested).unwrap(),
            vec![
                Object::Named("https://example.org/x".to_owned()),
                Object::Named("https://example.org/y".to_owned()),
            ]
        );
    }

    #[test]
    fn rdf_list_cyclic_rest_terminates() {
        // acceptance 4: an explicit rdf:first/rdf:rest cons chain whose tail loops
        // back to the head. The walk is cycle-guarded — it returns Ok and terminates.
        let ttl = "@prefix ex: <https://example.org/> .\n\
                   @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n\
                   ex:s ex:list _:c0 .\n\
                   _:c0 rdf:first ex:a ; rdf:rest _:c1 .\n\
                   _:c1 rdf:first ex:b ; rdf:rest _:c0 .\n";
        let ds = Dataset::parse_turtle(ttl.as_bytes(), "test").unwrap();
        let head = head_subject(
            ds.first_object("https://example.org/s", "https://example.org/list")
                .unwrap()
                .unwrap(),
        );
        // Finite Vec, truncated at the revisited cell — no hang.
        assert_eq!(
            ds.rdf_list(&head).unwrap(),
            vec![
                Object::Named("https://example.org/a".to_owned()),
                Object::Named("https://example.org/b".to_owned()),
            ]
        );
    }

    #[test]
    fn rdf_list_malformed_multiple_first_is_error() {
        // acceptance 5: a cons cell with two rdf:first objects is a hard error.
        let ttl = "@prefix ex: <https://example.org/> .\n\
                   @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n\
                   ex:s ex:list _:c0 .\n\
                   _:c0 rdf:first ex:a ; rdf:first ex:b ; rdf:rest rdf:nil .\n";
        let ds = Dataset::parse_turtle(ttl.as_bytes(), "test").unwrap();
        let head = head_subject(
            ds.first_object("https://example.org/s", "https://example.org/list")
                .unwrap()
                .unwrap(),
        );
        assert!(matches!(
            ds.rdf_list(&head),
            Err(SliceError::RdfList(purrdf::RdfListError::MultipleFirst))
        ));
    }

    #[test]
    fn rdf_list_malformed_dangling_rest_is_error() {
        // acceptance 5 (variant): an rdf:rest to a non-nil non-cell IRI is an error.
        let ttl = "@prefix ex: <https://example.org/> .\n\
                   @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n\
                   ex:s ex:list _:c0 .\n\
                   _:c0 rdf:first ex:a ; rdf:rest ex:dangling .\n";
        let ds = Dataset::parse_turtle(ttl.as_bytes(), "test").unwrap();
        let head = head_subject(
            ds.first_object("https://example.org/s", "https://example.org/list")
                .unwrap()
                .unwrap(),
        );
        assert!(matches!(
            ds.rdf_list(&head),
            Err(SliceError::RdfList(purrdf::RdfListError::DanglingRest))
        ));
    }

    #[test]
    fn rdf_list_terminator_taxonomy() {
        // acceptance 6: rdf:nil head → empty; a plain non-list IRI head → empty.
        // The fixture carries a real collection so the rdf: list vocabulary interns.
        let ttl = "@prefix ex: <https://example.org/> .\n\
                   ex:s ex:list ( ex:a ) .\n\
                   ex:plain a ex:Thing .\n";
        let ds = Dataset::parse_turtle(ttl.as_bytes(), "test").unwrap();

        let nil = Subject::Named("http://www.w3.org/1999/02/22-rdf-syntax-ns#nil".to_owned());
        assert!(
            ds.rdf_list(&nil).unwrap().is_empty(),
            "rdf:nil head is empty"
        );

        let plain = Subject::Named("https://example.org/plain".to_owned());
        assert!(
            ds.rdf_list(&plain).unwrap().is_empty(),
            "a plain IRI with no list structure is empty"
        );
    }

    #[test]
    fn members_container_numeric_order_and_collection_dispatch() {
        // acceptance 7: an rdf:Seq container materializes in numeric ordinal order
        // (with a gap absorbed), and members() on an rdf:first head walks the Collection.
        let ttl = "@prefix ex: <https://example.org/> .\n\
                   @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n\
                   ex:seq a rdf:Seq ;\n\
                          rdf:_2 ex:y ;\n\
                          rdf:_1 ex:x ;\n\
                          rdf:_3 ex:z .\n\
                   ex:s ex:list ( ex:a ex:b ) .\n";
        let ds = Dataset::parse_turtle(ttl.as_bytes(), "test").unwrap();

        let seq = Subject::Named("https://example.org/seq".to_owned());
        assert_eq!(
            ds.members(&seq).unwrap(),
            vec![
                Object::Named("https://example.org/x".to_owned()),
                Object::Named("https://example.org/y".to_owned()),
                Object::Named("https://example.org/z".to_owned()),
            ]
        );

        // members() on a Collection head dispatches to the rdf:first/rest walk.
        let list_head = head_subject(
            ds.first_object("https://example.org/s", "https://example.org/list")
                .unwrap()
                .unwrap(),
        );
        assert_eq!(
            ds.members(&list_head).unwrap(),
            vec![
                Object::Named("https://example.org/a".to_owned()),
                Object::Named("https://example.org/b".to_owned()),
            ]
        );
    }

    #[test]
    fn rdf_list_graph_scoped_to_named_graph() {
        // acceptance 8: a list living in a NAMED graph is materialized only under
        // GraphSel::Named — the default graph does not see its cons cells.
        let trig = "@prefix ex: <https://example.org/> .\n\
                    ex:g { ex:s ex:list ( ex:a ex:b ) . }\n";
        let ds =
            Dataset::parse(trig.as_bytes(), NativeRdfFormat::TriG.media_type(), "test").unwrap();

        let named = ds.graph(GraphSel::Named("https://example.org/g"));
        let head = head_subject(
            named
                .first_object("https://example.org/s", "https://example.org/list")
                .unwrap()
                .unwrap(),
        );
        assert_eq!(
            named.rdf_list(&head).unwrap(),
            vec![
                Object::Named("https://example.org/a".to_owned()),
                Object::Named("https://example.org/b".to_owned()),
            ]
        );

        // The default graph sees neither the head edge nor the list itself.
        let default = ds.graph(GraphSel::Default);
        assert!(
            default
                .first_object("https://example.org/s", "https://example.org/list")
                .unwrap()
                .is_none()
        );
        assert!(
            default.rdf_list(&head).unwrap().is_empty(),
            "the named-graph list is invisible from the default graph"
        );
    }

    #[test]
    fn accumulator_merges_and_dedups() {
        let mut acc = DatasetAccumulator::new();
        acc.add_turtle(
            b"@prefix ex: <https://example.org/> .\nex:a a ex:Thing .\n",
            "f1",
        )
        .unwrap();
        // Same triple again from a different file dedups; a new triple is added.
        acc.add_turtle(
            b"@prefix ex: <https://example.org/> .\nex:a a ex:Thing .\nex:b a ex:Thing .\n",
            "f2",
        )
        .unwrap();
        let ds = acc.freeze().unwrap();
        let subjects = ds.subjects_of_type("https://example.org/Thing").unwrap();
        assert_eq!(subjects.len(), 2, "the duplicate ex:a collapses");
    }
}
