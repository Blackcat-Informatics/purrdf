// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Oxigraph-free RDF query surface for the slice emitters and linters (EPIC #906).
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
    parse_dataset, DatasetView, GraphMatch, NativeRdfFormat, RdfDataset, RdfDatasetBuilder,
    RdfQuad, RdfTerm, TermId, TermRef, TermValue,
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

/// The kinds of RDF subject a quad can carry (named node OR blank node), surfaced
/// from the native IR. Mirrors the `oxigraph::model::NamedOrBlankNode` discrimination
/// the slice linters relied on.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Subject {
    /// An IRI subject.
    Named(String),
    /// A blank-node subject, by its (scope-qualified) label.
    Blank(String),
}

/// An RDF object term, surfaced from the native IR as an owned value — the
/// oxigraph-free replacement for `oxigraph::model::Term` in object position.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Object {
    /// An IRI object.
    Named(String),
    /// A blank-node object, by its (scope-qualified) label.
    Blank(String),
    /// A literal: its lexical form (the `.value()` of the old oxigraph literal).
    Literal { value: String },
    /// A quoted triple term (RDF 1.2). The slice linters never inspect inside one;
    /// it is surfaced only so it is not silently dropped.
    Triple,
}

impl Object {
    /// The IRI, if this object is a named node (`oxigraph` `Term::NamedNode` arm).
    pub fn as_named(&self) -> Option<&str> {
        match self {
            Self::Named(iri) => Some(iri.as_str()),
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
        // A literal/triple cannot stand in subject position in well-formed RDF; the
        // slice never queries on one, so treat it as "no IRI subject".
        _ => None,
    }
}

fn object_of(ds: &RdfDataset, id: TermId) -> Object {
    match ds.resolve(id) {
        TermRef::Iri(iri) => Object::Named(iri.to_owned()),
        TermRef::Blank { label, scope } => Object::Blank(scope.qualify_label(label).into_owned()),
        TermRef::Literal { lexical, .. } => Object::Literal {
            value: lexical.to_owned(),
        },
        TermRef::Triple { .. } => Object::Triple,
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

    /// All `(predicate-IRI, Object)` pairs of `<subject> ?p ?o` in the default graph,
    /// in dataset order. Used to scan a single subject's outgoing edges.
    pub fn predicate_objects_of(
        &self,
        subject_iri: &str,
    ) -> Result<Vec<(String, Object)>, SliceError> {
        let Some(s) = self.id(subject_iri) else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for q in self
            .ds
            .quads_for_pattern(Some(s), None, None, GraphMatch::Default)
        {
            let TermRef::Iri(p) = self.ds.resolve(q.p) else {
                continue;
            };
            out.push((p.to_owned(), object_of(&self.ds, q.o)));
        }
        Ok(out)
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
        let mut out = Vec::new();
        for subject in self.subject_terms_of_type(type_iri)? {
            if let Subject::Named(iri) = subject {
                out.push(iri);
            }
        }
        Ok(out)
    }

    /// Every subject (named OR blank) of `?s a <type_iri>` in the default graph.
    pub fn subject_terms_of_type(&self, type_iri: &str) -> Result<Vec<Subject>, SliceError> {
        let p = self.id(RDF_TYPE);
        let o = self.id(type_iri);
        let (Some(p), Some(o)) = (p, o) else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for q in self
            .ds
            .quads_for_pattern(None, Some(p), Some(o), GraphMatch::Default)
        {
            if let Some(s) = subject_of(&self.ds, q.s) {
                out.push(s);
            }
        }
        Ok(out)
    }

    /// All object terms of `<subject> <pred> ?o` in the default graph, where the
    /// subject is named.
    pub fn objects(&self, subject_iri: &str, pred: &str) -> Result<Vec<Object>, SliceError> {
        self.objects_of_subject(&Subject::Named(subject_iri.to_owned()), pred)
    }

    /// All object terms of `<subject> <pred> ?o` in the default graph (subject may be
    /// a blank node).
    pub fn objects_of_subject(
        &self,
        subject: &Subject,
        pred: &str,
    ) -> Result<Vec<Object>, SliceError> {
        let s = self.subject_id(subject);
        let p = self.id(pred);
        let (Some(s), Some(p)) = (s, p) else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for q in self
            .ds
            .quads_for_pattern(Some(s), Some(p), None, GraphMatch::Default)
        {
            out.push(object_of(&self.ds, q.o));
        }
        Ok(out)
    }

    /// The first object of `<subject> <pred> ?o` in the default graph, or `None`.
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
            Some(Object::Literal { value }) => Ok(Some(value)),
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

    /// Every IRI subject of `?s <pred> <object>` in the default graph.
    pub fn subjects_with_object(
        &self,
        pred: &str,
        object_iri: &str,
    ) -> Result<Vec<String>, SliceError> {
        let p = self.id(pred);
        let o = self.id(object_iri);
        let (Some(p), Some(o)) = (p, o) else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for q in self
            .ds
            .quads_for_pattern(None, Some(p), Some(o), GraphMatch::Default)
        {
            if let Some(Subject::Named(iri)) = subject_of(&self.ds, q.s) {
                out.push(iri);
            }
        }
        Ok(out)
    }

    /// Every `(subject-IRI, object-IRI)` pair of `?s <pred> ?o` in the default graph
    /// where both are named nodes.
    pub fn subject_object_iri_pairs(
        &self,
        pred: &str,
    ) -> Result<Vec<(String, String)>, SliceError> {
        let Some(p) = self.id(pred) else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for q in self
            .ds
            .quads_for_pattern(None, Some(p), None, GraphMatch::Default)
        {
            if let (Some(Subject::Named(s)), Object::Named(o)) =
                (subject_of(&self.ds, q.s), object_of(&self.ds, q.o))
            {
                out.push((s, o));
            }
        }
        Ok(out)
    }

    /// Whether `<subject> a <type_iri>` holds in the default graph.
    pub fn has_type(&self, subject_iri: &str, type_iri: &str) -> Result<bool, SliceError> {
        let s = self.id(subject_iri);
        let p = self.id(RDF_TYPE);
        let o = self.id(type_iri);
        let (Some(s), Some(p), Some(o)) = (s, p, o) else {
            return Ok(false);
        };
        Ok(self
            .ds
            .quads_for_pattern(Some(s), Some(p), Some(o), GraphMatch::Default)
            .next()
            .is_some())
    }

    /// Iterate every quad as `(Subject, predicate-IRI, Object, graph-IRI-or-None)`.
    /// Used by the few consumers that scan the whole dataset.
    pub fn for_each_quad(&self, mut f: impl FnMut(Subject, &str, Object, Option<&str>)) {
        for q in self.ds.quads() {
            let Some(s) = subject_of(&self.ds, q.s) else {
                continue;
            };
            let TermRef::Iri(p) = self.ds.resolve(q.p) else {
                continue;
            };
            let o = object_of(&self.ds, q.o);
            let g = q.g.and_then(|gid| match self.ds.resolve(gid) {
                TermRef::Iri(iri) => Some(iri),
                _ => None,
            });
            f(s, p, o, g);
        }
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

    fn id(&self, iri: &str) -> Option<TermId> {
        self.ds.term_id_by_value(&TermValue::iri(iri))
    }

    fn subject_id(&self, subject: &Subject) -> Option<TermId> {
        let value = match subject {
            Subject::Named(iri) => TermValue::iri(iri.clone()),
            Subject::Blank(label) => TermValue::blank(label.clone()),
        };
        self.ds.term_id_by_value(&value)
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
