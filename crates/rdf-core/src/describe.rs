// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Per-subject subgraph extraction — the **Symmetric Concise Bounded Description**
//! (SCBD) of a resource.
//!
//! The documentation site exports each term's (and each slice's) RDF in every
//! serialization format. To do that it needs the subgraph that *describes* a subject.
//! A plain CBD (outgoing triples + forward blank closure) would under-represent the
//! very thing PurRDF exists to showcase: the **incoming** links — `skos:exactMatch`
//! targets, `rdfs:subPropertyOf`/`subClassOf` children, authority back-references. So
//! `describe` returns the **symmetric** CBD:
//!
//! 1. every triple where the subject is the **subject** (outgoing), and
//! 2. every triple where the subject is the **object** (incoming), and
//! 3. the transitive **blank-node** closure on both directions (a definition hung off
//!    a blank restriction surfaces in full), and
//! 4. the RDF-1.2 statement-layer **reifiers** whose reified triple's subject *or*
//!    object lies in the closure, together with their annotations.
//!
//! Named-node endpoints do **not** expand (that would pull in the whole graph); only
//! blank nodes do.
//!
//! # Graph scope: selection is graph-blind, emission is graph-faithful
//!
//! Every clause above **selects** by term membership alone and **emits** each statement
//! into the graph it was asserted in. Clauses 1-3 have always done that — `by_endpoint`
//! indexes a quad under its subject/object whatever `q.g` says, and the emission carries
//! `q.g` through. Clause 4 now does the same:
//!
//! * a reifier **declaration** is the row `(reifier, triple-term, graph)`. It is selected
//!   when the reified triple's subject *or* object lies in the closure — **graph
//!   membership takes no part in that test** — and is re-emitted into its own `graph`.
//!   So a declaration in `<g>` about a triple asserted in the default graph (or about a
//!   triple asserted nowhere at all — a reified triple need not be asserted, so there is
//!   no "graph of the reified triple" a selector could even match against) is kept, and
//!   kept in `<g>`.
//! * an **annotation** rides with the declaration it annotates, not with the reifier
//!   resource: rows are harvested per `(reifier, graph)` and re-emitted into that graph.
//!   The RDF 1.2 statement layer is keyed per graph, so an annotation belongs to the
//!   declaration made in the same graph; harvesting a reifier's rows across graphs would
//!   emit an annotation into a graph whose declaration the description never selected.
//!
//! This is deliberate, and it is the opposite of what this module did before: it used to
//! drop the graph slot and re-emit the whole statement layer into the default graph, on
//! the rationale that reification is standpoint-scoped and carries no graph dimension.
//! It does carry one — the reifier and annotation side-tables are keyed by
//! `(graph, term)` — so that collapse silently relocated statements out of the graph they
//! were asserted in, which is the one thing a description must never do.
//!
//! The extracted subgraph is a fresh, structurally valid [`RdfDataset`] that can be
//! handed straight to the `native_codecs` serializers (Turtle / N-Triples / N-Quads /
//! TriG / RDF-XML) and the JSON-LD serializer — the one serialization seam.

use crate::{
    DatasetView, FastMap, NativeBuildError, QuadIds, RdfDataset, RdfDatasetBuilder, RdfDiagnostic,
    TermGuard as _, TermId, TermRef, TermValue,
};
use core::hash::Hash;
use purrdf_lex::allocation::{Admission, Memory, Resident, StorageError};
use std::convert::Infallible;
use std::sync::Arc;

/// A source or physical storage failure during description construction.
#[derive(Debug)]
pub enum DescribeError<E> {
    /// The original backing view's read failure.
    Source(E),
    /// Refusal before native buffer growth.
    Storage(StorageError),
    /// Invalid RDF encountered while building the selected graph.
    Build(NativeBuildError),
    /// A malformed term returned by the view.
    Invalid(&'static str),
}
impl<E: std::fmt::Display> std::fmt::Display for DescribeError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(error) => write!(f, "description source read: {error}"),
            Self::Storage(error) => std::fmt::Display::fmt(error, f),
            Self::Build(error) => std::fmt::Display::fmt(error, f),
            Self::Invalid(reason) => f.write_str(reason),
        }
    }
}
impl<E: std::error::Error + 'static> std::error::Error for DescribeError<E> {}
purrdf_lex::variant_from!(impl<E> DescribeError<E> { Storage(StorageError) });

type ReifierDeclaration<I> = (I, I, Option<I>);
type AnnotationBinding<I> = (I, I, Option<I>);
type AnnotationRow<I> = (I, I, I, Option<I>);

fn read_diagnostic(error: impl std::fmt::Display) -> RdfDiagnostic {
    RdfDiagnostic::error("rdf-describe-source-read", error.to_string())
}

fn insert<K: Eq + Hash, V, S: Admission + ?Sized>(
    map: &mut FastMap<K, V>,
    key: K,
    value: V,
    memory: &mut Memory<'_, S>,
) -> Result<Option<V>, StorageError> {
    if let Some(existing) = map.get_mut(&key) {
        // HashMap::insert may grow a full table even when replacing an existing key.
        return Ok(Some(std::mem::replace(existing, value)));
    }
    crate::hash::reserve_map_with_memory(
        map,
        map.len().checked_add(1).ok_or(StorageError::SizeOverflow)?,
        memory,
    )?;
    Ok(map.insert(key, value))
}

fn adjacency<K: Copy + Eq + Hash, V, S: Admission + ?Sized>(
    map: &mut FastMap<K, Vec<V>>,
    key: K,
    value: V,
    memory: &mut Memory<'_, S>,
) -> Result<(), StorageError> {
    if !map.contains_key(&key) {
        let _ = insert(map, key, Vec::new(), memory)?;
    }
    memory.push(map.get_mut(&key).expect("adjacency key inserted"), value)
}

fn release_map<K, V, S: Admission + ?Sized>(
    map: FastMap<K, V>,
    memory: &mut Memory<'_, S>,
) -> Result<(), StorageError> {
    let bytes = crate::hash::hash_table_allocation_bound::<(K, V)>(map.capacity())
        .ok_or(StorageError::SizeOverflow)?;
    drop(map);
    memory.release_bytes(bytes)
}

/// Reusable endpoint and RDF 1.2 statement-layer adjacency for symmetric CBD.
/// Native construction and extraction use the same algorithm as resident APIs.
#[derive(Debug)]
pub struct Describer<'a, D: DatasetView = RdfDataset> {
    dataset: &'a D,
    by_endpoint: FastMap<D::Id, Vec<QuadIds<D::Id>>>,
    reifiers_by_endpoint: FastMap<D::Id, Vec<ReifierDeclaration<D::Id>>>,
    annotations_by_reifier: FastMap<D::Id, Vec<AnnotationBinding<D::Id>>>,
}

impl<'a, D: DatasetView<ReadError = Infallible>> Describer<'a, D> {
    /// Build reusable adjacency over a resident infallible dataset.
    #[must_use]
    pub fn new(dataset: &'a D) -> Self {
        match Self::try_new(dataset) {
            Ok(value) => value,
            Err(error) => match error {},
        }
    }
}

impl<'a, D: DatasetView> Describer<'a, D> {
    /// Build adjacency, returning the original source failure.
    ///
    /// # Errors
    /// Returns source read failure; resident allocator refusal is terminal.
    pub fn try_new(dataset: &'a D) -> Result<Self, D::ReadError> {
        let mut resident = Resident;
        match Self::try_new_with_memory(dataset, &mut Memory::new(&mut resident)) {
            Ok(value) => Ok(value),
            Err(DescribeError::Source(error)) => Err(error),
            Err(error) => panic!("resident description construction: {error}"),
        }
    }

    /// Admit each concrete adjacency table and row buffer before its growth.
    /// The original Memory owner must enclose this extractor until destruction.
    ///
    /// # Errors
    /// Returns typed source or native storage refusal without partial publication.
    pub fn try_new_with_memory<S: Admission + ?Sized>(
        dataset: &'a D,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, DescribeError<D::ReadError>> {
        dataset
            .checked_read(|_| Self::build(dataset, memory))
            .map_err(DescribeError::Source)?
    }

    fn build<S: Admission + ?Sized>(
        dataset: &'a D,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, DescribeError<D::ReadError>> {
        let mut result = Self {
            dataset,
            by_endpoint: FastMap::default(),
            reifiers_by_endpoint: FastMap::default(),
            annotations_by_reifier: FastMap::default(),
        };
        for quad in dataset.quads() {
            adjacency(&mut result.by_endpoint, quad.s, quad, memory)?;
            if quad.o != quad.s {
                adjacency(&mut result.by_endpoint, quad.o, quad, memory)?;
            }
        }
        for quad in dataset.reifier_quads() {
            let guard = dataset.resolve(quad.o).map_err(DescribeError::Source)?;
            if let TermRef::Triple { s, p: _, o } = guard.term() {
                let binding = (quad.s, quad.o, quad.g);
                adjacency(&mut result.reifiers_by_endpoint, s, binding, memory)?;
                if o != s {
                    adjacency(&mut result.reifiers_by_endpoint, o, binding, memory)?;
                }
            }
        }
        for quad in dataset.annotation_quads() {
            adjacency(
                &mut result.annotations_by_reifier,
                quad.s,
                (quad.p, quad.o, quad.g),
                memory,
            )?;
        }
        Ok(result)
    }

    /// Destroy adjacency buffers before releasing their original physical bytes.
    ///
    /// # Errors
    /// Propagates the original account's refusal to shrink conservatively.
    pub fn release_with_memory<S: Admission + ?Sized>(
        mut self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        for (_, rows) in self.by_endpoint.drain() {
            memory.release_vec(rows)?;
        }
        for (_, rows) in self.reifiers_by_endpoint.drain() {
            memory.release_vec(rows)?;
        }
        for (_, rows) in self.annotations_by_reifier.drain() {
            memory.release_vec(rows)?;
        }
        release_map(self.by_endpoint, memory)?;
        release_map(self.reifiers_by_endpoint, memory)?;
        release_map(self.annotations_by_reifier, memory)
    }

    /// Describe one resource, preserving its source graph and statement metadata.
    ///
    /// # Errors
    /// Returns source or RDF construction diagnostics.
    pub fn describe_iri(&self, subject: &str) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
        self.describe_iris([subject])
    }

    /// Describe the union of several resources using the same native walk.
    ///
    /// # Errors
    /// Returns source or RDF construction diagnostics.
    pub fn describe_iris<'s>(
        &self,
        subjects: impl IntoIterator<Item = &'s str>,
    ) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
        let mut resident = Resident;
        self.describe_iris_with_memory(subjects, &mut Memory::new(&mut resident))
            .map(Arc::new)
            .map_err(|error| match error {
                DescribeError::Build(NativeBuildError::Diagnostic(diagnostic)) => diagnostic,
                DescribeError::Invalid(reason) => {
                    RdfDiagnostic::error("rdf-describe-invalid-term", reason)
                }
                error => read_diagnostic(error),
            })
    }

    /// Build a frozen description by value under the original native admission.
    ///
    /// # Errors
    /// Returns source, storage, or RDF validation failure before publication.
    pub fn describe_iris_with_memory<'s, S: Admission + ?Sized>(
        &self,
        subjects: impl IntoIterator<Item = &'s str>,
        memory: &mut Memory<'_, S>,
    ) -> Result<RdfDataset, DescribeError<D::ReadError>> {
        let mut seeds = Vec::new();
        for subject in subjects {
            let term = TermValue::Iri(memory.string(subject)?);
            let result = self
                .dataset
                .term_id_by_value(&term)
                .map_err(DescribeError::Source);
            let TermValue::Iri(text) = term else {
                unreachable!("IRI lookup term")
            };
            memory.release_string(text)?;
            if let Some(id) = result? {
                memory.push(&mut seeds, id)?;
            }
        }
        self.dataset
            .checked_read(|_| self.describe_seeds_read(seeds, memory))
            .map_err(DescribeError::Source)?
    }

    fn describe_seeds_read<S: Admission + ?Sized>(
        &self,
        seeds: Vec<D::Id>,
        memory: &mut Memory<'_, S>,
    ) -> Result<RdfDataset, DescribeError<D::ReadError>> {
        let mut anchors = FastMap::default();
        let mut frontier = Vec::new();
        for &seed in &seeds {
            if insert(&mut anchors, seed, (), memory)?.is_none() {
                memory.push(&mut frontier, seed)?;
            }
        }
        memory.release_vec(seeds)?;
        let mut quads = FastMap::default();
        let mut reifiers = FastMap::default();
        let mut annotations: Vec<AnnotationRow<D::Id>> = Vec::new();
        let mut visited_reifiers = FastMap::default();
        macro_rules! expand_blank {
            ($endpoint:expr) => {{
                let endpoint = $endpoint;
                let guard = self
                    .dataset
                    .resolve(endpoint)
                    .map_err(DescribeError::Source)?;
                let blank = matches!(guard.term(), TermRef::Blank { .. });
                drop(guard);
                if blank && insert(&mut anchors, endpoint, (), memory)?.is_none() {
                    memory.push(&mut frontier, endpoint)?;
                }
            }};
        }
        while let Some(anchor) = frontier.pop() {
            if let Some(touching) = self.by_endpoint.get(&anchor) {
                for &quad in touching {
                    let _ = insert(&mut quads, quad, (), memory)?;
                    expand_blank!(quad.s);
                    expand_blank!(quad.o);
                }
            }
            if let Some(bindings) = self.reifiers_by_endpoint.get(&anchor) {
                for &(reifier, triple, graph) in bindings {
                    let _ = insert(&mut reifiers, (reifier, triple, graph), (), memory)?;
                    expand_blank!(reifier);
                    let guard = self
                        .dataset
                        .resolve(triple)
                        .map_err(DescribeError::Source)?;
                    if let TermRef::Triple { s, p: _, o } = guard.term() {
                        expand_blank!(s);
                        expand_blank!(o);
                    }
                    drop(guard);
                    if insert(&mut visited_reifiers, (reifier, graph), (), memory)?.is_none()
                        && let Some(bindings) = self.annotations_by_reifier.get(&reifier)
                    {
                        for &(predicate, object, _) in
                            bindings.iter().filter(|binding| binding.2 == graph)
                        {
                            memory.push(&mut annotations, (reifier, predicate, object, graph))?;
                            expand_blank!(predicate);
                            expand_blank!(object);
                        }
                    }
                }
            }
        }
        memory.release_vec(frontier)?;
        release_map(anchors, memory)?;
        release_map(visited_reifiers, memory)?;
        let mut ordered = memory.collect(quads.keys().copied())?;
        release_map(quads, memory)?;
        ordered.sort_unstable_by_key(|quad| (quad.g, quad.s, quad.p, quad.o));
        let mut ordered_reifiers = memory.collect(reifiers.keys().copied())?;
        release_map(reifiers, memory)?;
        ordered_reifiers.sort_unstable();
        let mut builder = RdfDatasetBuilder::new();
        let mut remap = FastMap::default();
        for quad in &ordered {
            let s = self.map_id(&mut builder, &mut remap, quad.s, memory)?;
            let p = self.map_id(&mut builder, &mut remap, quad.p, memory)?;
            let o = self.map_id(&mut builder, &mut remap, quad.o, memory)?;
            let g = quad
                .g
                .map(|id| self.map_id(&mut builder, &mut remap, id, memory))
                .transpose()?;
            builder.push_quad_with_memory(s, p, o, g, memory)?;
        }
        for &(reifier, triple, graph) in &ordered_reifiers {
            let r = self.map_id(&mut builder, &mut remap, reifier, memory)?;
            let t = self.map_id(&mut builder, &mut remap, triple, memory)?;
            let g = graph
                .map(|id| self.map_id(&mut builder, &mut remap, id, memory))
                .transpose()?;
            builder.push_reifier_in_graph_with_memory(r, t, g, memory)?;
        }
        for &(reifier, predicate, object, graph) in &annotations {
            let r = self.map_id(&mut builder, &mut remap, reifier, memory)?;
            let p = self.map_id(&mut builder, &mut remap, predicate, memory)?;
            let o = self.map_id(&mut builder, &mut remap, object, memory)?;
            let g = graph
                .map(|id| self.map_id(&mut builder, &mut remap, id, memory))
                .transpose()?;
            builder.push_annotation_in_graph_with_memory(r, p, o, g, memory)?;
        }
        memory.release_vec(ordered)?;
        memory.release_vec(ordered_reifiers)?;
        memory.release_vec(annotations)?;
        release_map(remap, memory)?;
        builder
            .freeze_with_memory(memory)
            .map_err(DescribeError::Build)
    }

    fn map_id<S: Admission + ?Sized>(
        &self,
        builder: &mut RdfDatasetBuilder,
        remap: &mut FastMap<D::Id, TermId>,
        old: D::Id,
        memory: &mut Memory<'_, S>,
    ) -> Result<TermId, DescribeError<D::ReadError>> {
        intern_view_term_with_memory(self.dataset, builder, remap, old, memory)
    }
}

/// Copy a source term into a builder using the original source guard and admitted storage.
///
/// The memo preserves shared nested components without constructing an owned intermediate.
///
/// # Errors
/// Returns the original source failure, physical storage refusal, or invalid datatype.
#[expect(
    clippy::implicit_hasher,
    reason = "native term memoization must retain the workspace's required fixed-key hasher"
)]
pub fn intern_view_term_with_memory<D: DatasetView, S: Admission + ?Sized>(
    dataset: &D,
    builder: &mut RdfDatasetBuilder,
    remap: &mut FastMap<D::Id, TermId>,
    old: D::Id,
    memory: &mut Memory<'_, S>,
) -> Result<TermId, DescribeError<D::ReadError>> {
    let mut pending = Vec::new();
    memory.push(&mut pending, (old, false))?;
    while let Some((id, assemble)) = pending.pop() {
        if remap.contains_key(&id) {
            continue;
        }
        let guard = dataset.resolve(id).map_err(DescribeError::Source)?;
        let new = match guard.term() {
            TermRef::Triple { s, p, o } if !assemble => {
                memory.push(&mut pending, (id, true))?;
                memory.push(&mut pending, (o, false))?;
                memory.push(&mut pending, (p, false))?;
                memory.push(&mut pending, (s, false))?;
                continue;
            }
            TermRef::Triple { s, p, o } => {
                builder.intern_triple_with_memory(remap[&s], remap[&p], remap[&o], memory)?
            }
            TermRef::Iri(iri) => builder.intern_iri_with_memory(iri, memory)?,
            TermRef::Blank { label, scope } => {
                builder.intern_blank_with_memory(label, scope, memory)?
            }
            TermRef::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => {
                let datatype = dataset.resolve(datatype).map_err(DescribeError::Source)?;
                let TermRef::Iri(iri) = datatype.term() else {
                    return Err(DescribeError::Invalid(
                        "literal datatype must resolve to an IRI",
                    ));
                };
                builder.intern_literal_parts_with_memory(
                    lexical,
                    Some(iri),
                    language,
                    direction,
                    memory,
                )?
            }
        };
        crate::hash::reserve_map_with_memory(remap, remap.len() + 1, memory)?;
        let _ = remap.insert(id, new);
    }
    memory.release_vec(pending)?;
    Ok(remap[&old])
}

/// One-shot symmetric CBD using the same reusable extractor.
///
/// # Errors
/// Returns source or RDF construction diagnostics.
pub fn describe(dataset: &RdfDataset, subject: &str) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    Describer::try_new(dataset)
        .map_err(read_diagnostic)?
        .describe_iri(subject)
}

#[cfg(test)]
fn owned_term<D: DatasetView>(dataset: &D, id: D::Id) -> Result<crate::RdfTerm, RdfDiagnostic> {
    let describer = Describer::try_new(dataset).map_err(read_diagnostic)?;
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let mut builder = RdfDatasetBuilder::new();
    let mut remap = FastMap::default();
    let mapped = describer
        .map_id(&mut builder, &mut remap, id, &mut memory)
        .map_err(read_diagnostic)?;
    let graph = builder
        .freeze_with_memory(&mut memory)
        .map_err(read_diagnostic)?;
    Ok(graph.to_owned_term(mapped))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RdfLiteral, RdfQuad, RdfTerm};

    const S: &str = "https://e/s";
    const OTHER: &str = "https://e/other";

    #[test]
    fn full_description_table_replacement_preserves_original_capacity_and_grant() {
        struct Boundary {
            limit: usize,
            live: usize,
        }
        impl Admission for Boundary {
            fn resize(&mut self, required: usize) -> Result<(), StorageError> {
                if required > self.limit {
                    return Err(StorageError::AdmissionFailed);
                }
                self.live = required;
                Ok(())
            }
        }
        let mut boundary = Boundary {
            limit: usize::MAX,
            live: 0,
        };
        let mut memory = Memory::new(&mut boundary);
        let mut map = FastMap::default();
        for key in 0..3_u32 {
            assert_eq!(insert(&mut map, key, key, &mut memory), Ok(None));
        }
        assert_eq!(map.len(), 3);
        assert_eq!(map.capacity(), 3);
        let original = memory.admitted_bytes();
        memory.admission_mut().limit = original;
        assert_eq!(insert(&mut map, 1, 9, &mut memory), Ok(Some(1)));
        assert_eq!(map[&1], 9);
        assert_eq!(map.capacity(), 3);
        assert_eq!(memory.admitted_bytes(), original);
        assert_eq!(
            insert(&mut map, 3, 3, &mut memory),
            Err(StorageError::AdmissionFailed)
        );
        assert_eq!(map.len(), 3);
        assert_eq!(map.capacity(), 3);
        release_map(map, &mut memory).unwrap();
        assert_eq!(memory.admitted_bytes(), 0);
        assert_eq!(memory.admission_mut().live, 0);
    }

    fn iri(v: &str) -> RdfTerm {
        RdfTerm::iri(v)
    }

    /// Build a dataset from owned quads (default graph) with an optional reifier +
    /// annotation on the first quad.
    fn dataset(quads: &[RdfQuad]) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        for q in quads {
            b.push_owned_quad(q);
        }
        b.freeze().expect("freeze test dataset")
    }

    fn triple(s: &str, p: &str, o: RdfTerm) -> RdfQuad {
        RdfQuad::new(iri(s), p.to_string(), o)
    }

    /// The set of `(s, p, o)` IRI/lexical strings in a described subgraph, for terse
    /// membership assertions (blank labels are scope-qualified, so compare by kind).
    fn objects_for(ds: &RdfDataset, subject: &str, predicate: &str) -> Vec<String> {
        let mut out = Vec::new();
        for q in ds.quad_refs() {
            let s = match q.s {
                TermRef::Iri(i) => i.to_string(),
                _ => continue,
            };
            let p = match q.p {
                TermRef::Iri(i) => i.to_string(),
                _ => continue,
            };
            if s == subject
                && p == predicate
                && let TermRef::Iri(o) = q.o
            {
                out.push(o.to_string());
            }
        }
        out.sort();
        out
    }

    #[test]
    fn describes_outgoing_triples() {
        let ds = dataset(&[
            triple(S, "https://e/p", iri("https://e/o1")),
            triple(S, "https://e/p", iri("https://e/o2")),
            triple(OTHER, "https://e/p", iri("https://e/x")),
        ]);
        let scbd = describe(&ds, S).unwrap();
        assert_eq!(
            objects_for(&scbd, S, "https://e/p"),
            vec!["https://e/o1".to_string(), "https://e/o2".to_string()]
        );
        // The unrelated OTHER subject's triple must NOT be pulled in.
        assert_eq!(objects_for(&scbd, OTHER, "https://e/p"), [] as [String; 0]);
    }

    #[test]
    fn describes_incoming_triples_symmetrically() {
        // A plain (forward-only) CBD would miss this: OTHER points AT S.
        let ds = dataset(&[triple(OTHER, "https://e/refersTo", iri(S))]);
        let scbd = describe(&ds, S).unwrap();
        assert_eq!(
            objects_for(&scbd, OTHER, "https://e/refersTo"),
            vec![S.to_string()],
            "the incoming link OTHER -> S must be present in the symmetric CBD"
        );
    }

    #[test]
    fn named_node_neighbours_do_not_expand() {
        // S -> N, and N -> deep. `deep` must NOT come along: named nodes don't expand.
        let ds = dataset(&[
            triple(S, "https://e/p", iri("https://e/n")),
            triple("https://e/n", "https://e/p", iri("https://e/deep")),
        ]);
        let scbd = describe(&ds, S).unwrap();
        // The N -> deep triple is neither outgoing-from nor incoming-to S, so absent.
        assert_eq!(
            objects_for(&scbd, "https://e/n", "https://e/p"),
            [] as [String; 0]
        );
        assert_eq!(
            objects_for(&scbd, S, "https://e/p"),
            vec!["https://e/n".to_string()]
        );
    }

    #[test]
    fn blank_nodes_expand_transitively() {
        // S -> _:b (restriction) -> onProperty target. The blank closure must bring the
        // blank's own triples along, both hops.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(S);
        let has = b.intern_iri("https://e/hasRestriction");
        let bnode = b.intern_blank("r1", crate::BlankScope::DEFAULT);
        let on = b.intern_iri("https://e/onProperty");
        let target = b.intern_iri("https://e/target");
        b.push_quad(s, has, bnode, None);
        b.push_quad(bnode, on, target, None);
        let ds = b.freeze().unwrap();

        let scbd = describe(&ds, S).unwrap();
        // Both quads survive: S -> _:b and _:b -> target.
        assert_eq!(
            scbd.quad_count(),
            2,
            "blank-node closure must keep both hops"
        );
        // The blank's onProperty edge is present (object is the named target).
        let has_target = scbd.as_ref().quad_refs().any(|q| {
            matches!(q.p, TermRef::Iri(i) if i == "https://e/onProperty")
                && matches!(q.o, TermRef::Iri(i) if i == "https://e/target")
        });
        assert!(has_target, "the blank node's own triple must be included");
    }

    #[test]
    fn absent_subject_yields_empty() {
        let ds = dataset(&[triple(S, "https://e/p", iri("https://e/o"))]);
        let scbd = describe(&ds, "https://e/nope").unwrap();
        assert_eq!(scbd.quad_count(), 0);
    }

    #[test]
    fn includes_reifiers_about_the_subject() {
        // S p o, with a reifier annotating that statement (a certainty note).
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(S);
        let p = b.intern_iri("https://e/p");
        let o = b.intern_iri("https://e/o");
        b.push_quad(s, p, o, None);
        let triple_term = b.intern_triple(s, p, o);
        let reifier = b.intern_blank("stmt1", crate::BlankScope::DEFAULT);
        b.push_reifier(reifier, triple_term);
        let certainty = b.intern_iri("https://e/certainty");
        let high = b.intern_literal(RdfLiteral::simple("high"));
        b.push_annotation(reifier, certainty, high);
        let ds = b.freeze().unwrap();

        let scbd = describe(&ds, S).unwrap();
        // The reifier binding (reifier rdf:reifies << s p o >>) and its annotation ride
        // along because the reified statement's subject is S.
        assert_eq!(
            scbd.reifiers().count(),
            1,
            "the reifier about S must be kept"
        );
        assert_eq!(scbd.annotations().count(), 1, "its annotation must be kept");
    }

    #[test]
    fn slice_scope_unions_subjects() {
        let ds = dataset(&[
            triple(S, "https://e/p", iri("https://e/o1")),
            triple(OTHER, "https://e/p", iri("https://e/o2")),
        ]);
        let d = Describer::new(&ds);
        let scbd = d.describe_iris([S, OTHER]).unwrap();
        assert_eq!(scbd.quad_count(), 2, "both subjects' triples in the union");
    }

    // The serializer round-trip (describe → every `native_codecs` format) lives in
    // `purrdf` (`crates/rdf/tests/describe_serialize.rs`) because the serializers
    // are in that higher crate; `purrdf-core` holds only the extraction itself.

    #[test]
    fn blank_reifier_own_triples_are_closed() {
        // A blank reifier that is ALSO the subject of a plain quad
        // (`_:stmt ex:author ex:alice`). That describing quad must ride along with the
        // reifier — a plain forward walk would drop it, leaving a dangling blank.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(S);
        let p = b.intern_iri("https://e/p");
        let o = b.intern_iri("https://e/o");
        b.push_quad(s, p, o, None);
        let triple_term = b.intern_triple(s, p, o);
        let reifier = b.intern_blank("stmt1", crate::BlankScope::DEFAULT);
        b.push_reifier(reifier, triple_term);
        // A plain triple describing the blank reifier itself.
        let author = b.intern_iri("https://e/author");
        let alice = b.intern_iri("https://e/alice");
        b.push_quad(reifier, author, alice, None);
        let ds = b.freeze().unwrap();

        let scbd = describe(&ds, S).unwrap();
        assert_eq!(
            scbd.reifiers().count(),
            1,
            "the reifier about S must be kept"
        );
        let has_author = scbd.as_ref().quad_refs().any(|q| {
            matches!(q.s, TermRef::Blank { .. })
                && matches!(q.p, TermRef::Iri(i) if i == "https://e/author")
                && matches!(q.o, TermRef::Iri(i) if i == "https://e/alice")
        });
        assert!(
            has_author,
            "the blank reifier's own describing triple must be included"
        );
    }

    #[test]
    fn blank_annotation_object_is_closed() {
        // S p o, reified; the reifier's `source` annotation points at a blank provenance
        // node that itself carries a triple (`_:prov ex:by ex:agent`). The annotation
        // side-table does not live in `by_endpoint`, so without folding the annotation
        // harvest into the walk that blank would dangle.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(S);
        let p = b.intern_iri("https://e/p");
        let o = b.intern_iri("https://e/o");
        b.push_quad(s, p, o, None);
        let triple_term = b.intern_triple(s, p, o);
        let reifier = b.intern_blank("stmt1", crate::BlankScope::DEFAULT);
        b.push_reifier(reifier, triple_term);
        let source = b.intern_iri("https://e/source");
        let prov = b.intern_blank("prov", crate::BlankScope::DEFAULT);
        b.push_annotation(reifier, source, prov);
        // The blank provenance node's own describing triple (a plain quad).
        let by = b.intern_iri("https://e/by");
        let agent = b.intern_iri("https://e/agent");
        b.push_quad(prov, by, agent, None);
        let ds = b.freeze().unwrap();

        let scbd = describe(&ds, S).unwrap();
        assert_eq!(
            scbd.annotations().count(),
            1,
            "the source annotation must be kept"
        );
        // The blank provenance node's `by agent` triple must survive (no dangling blank).
        let has_by = scbd.as_ref().quad_refs().any(|q| {
            matches!(q.s, TermRef::Blank { .. })
                && matches!(q.p, TermRef::Iri(i) if i == "https://e/by")
                && matches!(q.o, TermRef::Iri(i) if i == "https://e/agent")
        });
        assert!(
            has_by,
            "the blank annotation object's own triple must be included"
        );
    }

    /// The IRI a described dataset's single reifier declaration is scoped to, or
    /// `None` for the default graph. Panics unless there is exactly one declaration.
    fn only_reifier_graph(ds: &RdfDataset) -> Option<String> {
        let mut rows = ds.reifiers_with_graph();
        let (_, _, g) = rows.next().expect("exactly one reifier declaration");
        assert!(rows.next().is_none(), "exactly one reifier declaration");
        g.map(|g| match ds.resolve(g) {
            TermRef::Iri(i) => i.to_string(),
            other => panic!("a graph name must be an IRI, got {other:?}"),
        })
    }

    /// The graph IRIs (or `None`) the described dataset's annotation rows are scoped to.
    fn annotation_graphs(ds: &RdfDataset) -> Vec<Option<String>> {
        let mut out: Vec<Option<String>> = ds
            .annotations_with_graph()
            .map(|(_, _, _, g)| {
                g.map(|g| match ds.resolve(g) {
                    TermRef::Iri(i) => i.to_string(),
                    other => panic!("a graph name must be an IRI, got {other:?}"),
                })
            })
            .collect();
        out.sort();
        out
    }

    #[test]
    fn reifier_and_annotation_stay_in_their_source_graph() {
        // `GRAPH <g> { S p o ~:r {| certainty "high" |} }`. The base quad, the reifier
        // declaration and the annotation were all asserted in <g>, and the description
        // must keep all three there — collapsing the statement layer into the default
        // graph would relocate two of the three.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(S);
        let p = b.intern_iri("https://e/p");
        let o = b.intern_iri("https://e/o");
        let g = b.intern_iri("https://e/g");
        b.push_quad(s, p, o, Some(g));
        let triple_term = b.intern_triple(s, p, o);
        let reifier = b.intern_iri("https://e/r");
        b.push_reifier_in_graph(reifier, triple_term, Some(g));
        let certainty = b.intern_iri("https://e/certainty");
        let high = b.intern_literal(RdfLiteral::simple("high"));
        b.push_annotation_in_graph(reifier, certainty, high, Some(g));
        let ds = b.freeze().unwrap();

        let scbd = describe(&ds, S).unwrap();
        assert_eq!(
            only_reifier_graph(&scbd),
            Some("https://e/g".to_owned()),
            "the reifier declaration must stay in the graph that declared it"
        );
        assert_eq!(
            annotation_graphs(&scbd),
            vec![Some("https://e/g".to_owned())],
            "the annotation must stay in the graph that asserted it"
        );
        let base_graph = scbd
            .as_ref()
            .quad_refs()
            .map(|q| q.g.map(|g| format!("{g:?}")))
            .collect::<Vec<_>>();
        assert_eq!(base_graph.len(), 1, "one base quad");
        assert!(base_graph[0].is_some(), "the base quad keeps its graph too");
    }

    #[test]
    fn a_reifier_declared_in_another_graph_than_its_reified_triple_is_kept_there() {
        // The cross-graph case the graph-blind SELECTION rule admits: the base quad is
        // asserted in <g1>, and <g2> declares a reifier about it with an annotation.
        // Selection ignores graphs, so the declaration is reached; emission is faithful,
        // so it lands in <g2> — not in <g1>, and not in the default graph.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(S);
        let p = b.intern_iri("https://e/p");
        let o = b.intern_iri("https://e/o");
        let g1 = b.intern_iri("https://e/g1");
        let g2 = b.intern_iri("https://e/g2");
        b.push_quad(s, p, o, Some(g1));
        let triple_term = b.intern_triple(s, p, o);
        let reifier = b.intern_iri("https://e/r");
        b.push_reifier_in_graph(reifier, triple_term, Some(g2));
        let source = b.intern_iri("https://e/source");
        let ledger = b.intern_iri("https://e/ledger");
        b.push_annotation_in_graph(reifier, source, ledger, Some(g2));
        let ds = b.freeze().unwrap();

        let scbd = describe(&ds, S).unwrap();
        assert_eq!(
            only_reifier_graph(&scbd),
            Some("https://e/g2".to_owned()),
            "a declaration in <g2> about a triple asserted in <g1> stays in <g2>"
        );
        assert_eq!(
            annotation_graphs(&scbd),
            vec![Some("https://e/g2".to_owned())],
            "its annotation stays in <g2> as well"
        );
    }

    #[test]
    fn each_declaration_harvests_only_its_own_graphs_annotations() {
        // ONE reifier id, TWO graphs. <g1> declares it about a triple of S; <g2> declares
        // the SAME reifier id about an unrelated triple and annotates it there. The
        // annotation harvest is keyed by `(reifier, graph)`, so describing S keeps the
        // <g1> declaration and the <g1> annotation, and leaves the <g2> pair alone —
        // a reifier-only harvest would emit the <g2> annotation into a graph whose
        // declaration is not in the description at all.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(S);
        let p = b.intern_iri("https://e/p");
        let o = b.intern_iri("https://e/o");
        let other = b.intern_iri(OTHER);
        let g1 = b.intern_iri("https://e/g1");
        let g2 = b.intern_iri("https://e/g2");
        b.push_quad(s, p, o, Some(g1));
        b.push_quad(other, p, o, Some(g2));
        let about_s = b.intern_triple(s, p, o);
        let about_other = b.intern_triple(other, p, o);
        let reifier = b.intern_iri("https://e/r");
        b.push_reifier_in_graph(reifier, about_s, Some(g1));
        b.push_reifier_in_graph(reifier, about_other, Some(g2));
        let source = b.intern_iri("https://e/source");
        let here = b.intern_iri("https://e/here");
        let elsewhere = b.intern_iri("https://e/elsewhere");
        b.push_annotation_in_graph(reifier, source, here, Some(g1));
        b.push_annotation_in_graph(reifier, source, elsewhere, Some(g2));
        let ds = b.freeze().unwrap();

        let scbd = describe(&ds, S).unwrap();
        assert_eq!(
            only_reifier_graph(&scbd),
            Some("https://e/g1".to_owned()),
            "only the <g1> declaration reifies a triple of S"
        );
        assert_eq!(
            annotation_graphs(&scbd),
            vec![Some("https://e/g1".to_owned())],
            "the <g2> annotation belongs to the <g2> declaration, which was not selected"
        );
    }

    #[test]
    fn reifier_reifying_multiple_triples_closes_each_binding() {
        // One reifier reifies TWO triples about S. The second reified triple is NOT
        // asserted as a plain quad and its object is a blank with its own triple, so the
        // blank is reachable ONLY through that second binding. Deduplicating the whole
        // binding on the reifier id (rather than only the annotation harvest) would skip
        // closing the second triple's endpoints and drop the blank.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(S);
        let p1 = b.intern_iri("https://e/p1");
        let o1 = b.intern_iri("https://e/o1");
        let p2 = b.intern_iri("https://e/p2");
        let blank = b.intern_blank("bo", crate::BlankScope::DEFAULT);
        // Only the first triple is asserted as a plain quad; the second exists solely as
        // a reified statement.
        b.push_quad(s, p1, o1, None);
        let t1 = b.intern_triple(s, p1, o1);
        let t2 = b.intern_triple(s, p2, blank);
        let reifier = b.intern_blank("stmt", crate::BlankScope::DEFAULT);
        b.push_reifier(reifier, t1);
        b.push_reifier(reifier, t2);
        // The blank object of the second reified triple carries its own describing triple.
        let deep = b.intern_iri("https://e/deep");
        let val = b.intern_iri("https://e/val");
        b.push_quad(blank, deep, val, None);
        let ds = b.freeze().unwrap();

        let scbd = describe(&ds, S).unwrap();
        assert_eq!(
            scbd.reifiers().count(),
            2,
            "both reified triples must be kept"
        );
        let has_deep = scbd
            .as_ref()
            .quad_refs()
            .any(|q| matches!(q.p, TermRef::Iri(i) if i == "https://e/deep"));
        assert!(
            has_deep,
            "the blank endpoint of the second reified triple must be closed"
        );
    }
}

#[cfg(test)]
mod term_walk_tests {
    //! The owned-model resolution against its recursive reference.

    use super::owned_term;
    use crate::backend::TermFactory as _;
    use crate::term_fixture::TermShape;
    use crate::{RdfDataset, RdfDatasetBuilder, RdfTerm, RdfTriple, TermId, TermRef};

    fn reference(dataset: &RdfDataset, id: TermId) -> RdfTerm {
        match dataset.resolve(id) {
            TermRef::Triple { s, p, o } => {
                let TermRef::Iri(predicate) = dataset.resolve(p) else {
                    unreachable!("a stored predicate is an IRI")
                };
                RdfTerm::triple(RdfTriple::new(
                    reference(dataset, s),
                    predicate,
                    reference(dataset, o),
                ))
            }
            _ => owned_term(dataset, id).expect("a generated resident term resolves"),
        }
    }

    /// Every generated stored term resolves to the owned term the recursive reference
    /// builds.
    #[test]
    fn resolution_agrees_with_its_recursive_reference_on_generated_terms() {
        for seed in 0..300_u64 {
            let mut state = seed;
            let mut budget = 8;
            let value = crate::term_fixture::term_value(
                &mut state,
                purrdf_testkit::rng::splitmix64_next,
                &mut budget,
                TermShape::WellFormed,
            );
            let mut builder = RdfDatasetBuilder::new();
            let object = builder.intern_value(&value);
            let holder = builder.intern_iri("http://example.org/holder");
            builder.push_quad(holder, holder, object, None);
            let dataset = builder.freeze().expect("a generated term freezes");
            let object = dataset.quads().next().expect("one quad").o;
            assert_eq!(
                owned_term(&*dataset, object).expect("a generated resident term resolves"),
                reference(&dataset, object),
                "seed {seed}"
            );
        }
    }
}
