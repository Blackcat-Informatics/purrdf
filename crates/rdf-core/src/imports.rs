// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `owl:imports`: the ONE rule for when a graph's imports closure is in hand, and the one
//! merge that folds a resolved closure into a single dataset.
//!
//! # Why the rule lives in the kernel
//!
//! `owl:imports` is not an entailment construct and not a SHACL construct. It is an
//! RDF-level directive every consumer of a document has to honour: OWL 2 defines the imports
//! closure of an ontology to BE the ontology, and SHACL reads a shapes graph's `owl:imports`
//! the same way. Two engines in this workspace read it — entailment (`purrdf-entail`) and
//! SHACL (`purrdf-shapes`) — and neither depends on the other. A rule that lived in one of
//! them would have to be copied into the other or not applied there, and "one module, two
//! verdicts" is exactly the defect that arrangement produced. The kernel owns
//! [`RdfDataset`], which is the only thing the rule reads, so the rule lives beside it and
//! both engines take their verdict from here.
//!
//! # PurRDF fetches nothing
//!
//! PurRDF has no notion of what an ontology IRI dereferences to, and inventing one would make
//! an answer depend on what a URL served today — and would break the wasm32 build, which has
//! no network. So the documents an import names are **caller-supplied configuration**
//! ([`ImportMap`]), exactly like every other vocabulary this library reads, and an import no
//! document resolves is reported by name ([`ImportClosure::unresolved`]) for the engine to
//! refuse.
//!
//! # The rule
//!
//! An `owl:imports <X>` is RESOLVED when the document or ontology `X` names is already in
//! hand:
//!
//! * the caller's [`ImportMap`] supplies a document for `X`;
//! * `X` names a document the graph was READ from — its retrieval IRI or the base it was
//!   parsed under ([`ImportMap::declare_loaded`]). SHACL collects `sh:declare` prefixes along
//!   `sh:prefixes/owl:imports*`, and a document routinely points that path at its OWN IRI
//!   from a node that is not an `owl:Ontology` at all;
//! * the closure holds `X rdf:type owl:Ontology` — the ontology header OWL 2's RDF mapping
//!   reads an ontology's IRI from; or
//! * the closure holds some `O owl:versionIRI X` — an ontology whose VERSION IRI is `X`.
//!   OWL 2 §3.2 makes a version IRI a name the ontology may be imported by, and
//!   `owl:versionIRI`'s domain is `owl:Ontology`, so its subject is an ontology whether or
//!   not it is also typed as one.
//!
//! Everything else is unresolved.
//!
//! Presence counts because merging a vocabulary INTO the graph that imports it is how an
//! `owl:imports` is resolved by hand, and it is what a caller who read the W3C SHACL 1.2
//! vocabularies together has done: `shnex.ttl` imports `sh:`, and `shacl.ttl` is the document
//! whose header declares it. Refusing that graph as incomplete would refuse the very closure
//! the import asked for.
//!
//! # The closure is transitive, because the specification's is
//!
//! A supplied document may import further documents, and OWL 2's imports closure is the
//! transitive one. [`ImportMap::closure`] is therefore a work-list to a fixpoint over the
//! import graph, visiting each document once — which also makes a cyclic import (`A`
//! imports `B` imports `A`, which OWL 2 §3.4 explicitly permits) terminate rather than loop.
//! Whether an unsupplied import is declared in the closure is decided only once the walk is
//! complete, because a document reached LATER may be the one that declares it.
//!
//! # A supplied document the closure never reaches is reported too
//!
//! [`ImportClosure::unreached`] names every map entry the walk never visited. An engine that
//! treats the map as "the documents this graph is made of" refuses those: a caller who
//! supplied a document believes its content is part of the answer, and an entry nothing
//! imports — a misspelled graph IRI, an import the author forgot to write — would otherwise
//! be read and silently never used.
//!
//! # Blank nodes are standardized apart, and the importing graph's are not moved
//!
//! Merging RDF documents is an RDF MERGE: `_:b` in one and `_:b` in another are different
//! nodes. [`ImportClosure::merge`] copies each supplied document under blank-node scopes of
//! its own, allocated strictly above every scope the importing graph uses, and copies the
//! importing graph with its ORIGINAL scopes. Keeping those unmoved is what lets a caller go
//! on naming the importing graph's blank nodes by label after the merge — a SHACL node
//! expression rooted at `_:e`, an entailment warrant re-checked against the premise the
//! caller passed.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

use crate::RdfDiagnostic;
use crate::ir::{BlankScope, RdfDataset, RdfDatasetBuilder, TermId, TermValue};
use crate::model::RdfLiteral;

/// `owl:imports`.
const OWL_IMPORTS: &str = "http://www.w3.org/2002/07/owl#imports";
/// `owl:Ontology`.
const OWL_ONTOLOGY: &str = "http://www.w3.org/2002/07/owl#Ontology";
/// `owl:versionIRI`.
const OWL_VERSIONIRI: &str = "http://www.w3.org/2002/07/owl#versionIRI";
/// `rdf:type`.
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

/// The documents an `owl:imports` resolves to, and the IRIs the importing graph was read
/// from.
///
/// See the [module documentation](self) for the rule this configures. An empty map is the
/// right value for the overwhelmingly common graph that imports nothing, and the wrong one
/// for a graph that imports something — which is why the difference is reported rather than
/// defaulted.
///
/// ```
/// use purrdf_core::RdfDatasetBuilder;
/// use purrdf_core::imports::ImportMap;
///
/// let mut b = RdfDatasetBuilder::new();
/// let ontology = b.intern_iri("http://example.org/o");
/// let imports = b.intern_iri("http://www.w3.org/2002/07/owl#imports");
/// let other = b.intern_iri("http://example.org/other");
/// b.push_quad(ontology, imports, other, None);
/// let graph = b.freeze().expect("freeze");
///
/// // An import nobody supplied is named.
/// let closure = ImportMap::new().closure(&graph);
/// assert_eq!(closure.unresolved(), ["http://example.org/other".to_owned()]);
/// ```
#[derive(Debug, Clone, Default)]
pub struct ImportMap {
    /// Ontology IRI → the document it names.
    documents: BTreeMap<String, Arc<RdfDataset>>,
    /// The IRIs of the documents the importing graph itself was read from — its retrieval
    /// IRI or base. An import of one of these names a document already in hand.
    loaded: BTreeSet<String>,
}

impl ImportMap {
    /// An import map that resolves nothing and declares no loaded document.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Declare that `iri` names `document`, returning whatever it named before.
    pub fn insert(
        &mut self,
        iri: impl Into<String>,
        document: Arc<RdfDataset>,
    ) -> Option<Arc<RdfDataset>> {
        self.documents.insert(iri.into(), document)
    }

    /// The document `iri` names, if this map has one.
    #[must_use]
    pub fn get(&self, iri: &str) -> Option<&Arc<RdfDataset>> {
        self.documents.get(iri)
    }

    /// How many documents this map resolves.
    #[must_use]
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    /// Whether this map resolves no document at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    /// Every ontology IRI this map supplies a document for, in IRI order.
    pub fn iris(&self) -> impl Iterator<Item = &str> + '_ {
        self.documents.keys().map(String::as_str)
    }

    /// Declare that the importing graph was itself read from the document at `iri` — its
    /// retrieval IRI, or the base it was parsed under — returning whether the IRI was new.
    ///
    /// An `owl:imports <iri>` then names a document that is already loaded, so it is
    /// resolved without a merge. A caller that does not know where the graph came from
    /// declares nothing, and an import of that document then stays unresolved.
    pub fn declare_loaded(&mut self, iri: impl Into<String>) -> bool {
        self.loaded.insert(iri.into())
    }

    /// Whether `iri` was declared with [`declare_loaded`](Self::declare_loaded).
    #[must_use]
    pub fn is_loaded(&self, iri: &str) -> bool {
        self.loaded.contains(iri)
    }

    /// Walk `graph`'s transitive `owl:imports` closure against this map, to a fixpoint.
    ///
    /// Breadth-first, each IRI visited once, so a cycle terminates. A map-supplied document
    /// wins over an in-graph declaration of the same ontology: the caller named that
    /// document, and merging it is what the caller asked for.
    #[must_use]
    pub fn closure(&self, graph: &RdfDataset) -> ImportClosure {
        let mut queue: VecDeque<String> = imported_iris(graph).into_iter().collect();
        if queue.is_empty() {
            // The common case — a graph that imports nothing — costs one term lookup and
            // no survey of the graph's ontology declarations.
            return ImportClosure {
                documents: Vec::new(),
                unresolved: Vec::new(),
                unreached: self.documents.keys().cloned().collect(),
            };
        }
        let mut declared: BTreeSet<String> = self.loaded.clone();
        declared_ontologies(graph, &mut declared);
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut documents: Vec<(String, Arc<RdfDataset>)> = Vec::new();
        let mut unsupplied: Vec<String> = Vec::new();
        while let Some(iri) = queue.pop_front() {
            if !seen.insert(iri.clone()) {
                continue;
            }
            let Some(document) = self.get(&iri) else {
                unsupplied.push(iri);
                continue;
            };
            declared_ontologies(document, &mut declared);
            queue.extend(imported_iris(document));
            documents.push((iri, Arc::clone(document)));
        }
        unsupplied.retain(|iri| !declared.contains(iri));
        let unreached = self
            .documents
            .keys()
            .filter(|iri| !seen.contains(*iri))
            .cloned()
            .collect();
        ImportClosure {
            documents,
            unresolved: unsupplied,
            unreached,
        }
    }

    /// Every ontology IRI in `graph`'s transitive `owl:imports` closure that neither this
    /// map nor the closure itself resolves, deduplicated, in the order the closure walk
    /// first meets them — [`ImportClosure::unresolved`] of [`closure`](Self::closure).
    ///
    /// ```
    /// use purrdf_core::RdfDatasetBuilder;
    /// use purrdf_core::imports::ImportMap;
    ///
    /// let mut b = RdfDatasetBuilder::new();
    /// let ontology = b.intern_iri("http://example.org/o");
    /// let imports = b.intern_iri("http://www.w3.org/2002/07/owl#imports");
    /// let other = b.intern_iri("http://example.org/other");
    /// b.push_quad(ontology, imports, other, None);
    /// let graph = b.freeze().expect("freeze");
    ///
    /// assert_eq!(
    ///     ImportMap::new().unresolved_imports(&graph),
    ///     vec!["http://example.org/other".to_owned()]
    /// );
    /// ```
    #[must_use]
    pub fn unresolved_imports(&self, graph: &RdfDataset) -> Vec<String> {
        self.closure(graph).unresolved
    }
}

/// One walk of an import closure: what the map supplied, what nothing resolved, and what
/// the map supplied that the walk never reached. See [`ImportMap::closure`].
#[derive(Debug, Clone, Default)]
pub struct ImportClosure {
    /// Each document the map supplied, once, in the order the walk reached it.
    documents: Vec<(String, Arc<RdfDataset>)>,
    /// Each import neither the map nor any graph of the closure resolves, once, in the
    /// order the walk first met it.
    unresolved: Vec<String>,
    /// Each map entry the walk never visited, in IRI order.
    unreached: Vec<String>,
}

impl ImportClosure {
    /// The supplied documents the closure reached, `(ontology IRI, document)`, in walk order.
    #[must_use]
    pub fn documents(&self) -> &[(String, Arc<RdfDataset>)] {
        &self.documents
    }

    /// Every import nothing in hand resolves, in the order the walk first met it. Empty
    /// exactly when the closure is complete.
    #[must_use]
    pub fn unresolved(&self) -> &[String] {
        &self.unresolved
    }

    /// Every map entry no import in the closure names, in IRI order.
    #[must_use]
    pub fn unreached(&self) -> &[String] {
        &self.unreached
    }

    /// `graph` together with every document this closure reached, as one dataset — or
    /// `None` when the closure reached no document, so there is no copy to pay for and the
    /// caller goes on with the dataset it already has.
    ///
    /// The merge does not look at [`unresolved`](Self::unresolved): whether an incomplete
    /// closure may be used at all is the calling engine's decision, and every engine in
    /// this workspace refuses it before it gets here.
    ///
    /// `graph` keeps its own blank-node scopes; each document is copied under fresh scopes
    /// allocated above every scope `graph` uses (see the [module documentation](self)).
    /// Reifier bindings, annotations and declared named graphs travel with their document.
    ///
    /// # Errors
    ///
    /// The merged dataset fails to freeze, which a merge of frozen datasets cannot cause.
    pub fn merge(&self, graph: &RdfDataset) -> Result<Option<Arc<RdfDataset>>, RdfDiagnostic> {
        if self.documents.is_empty() {
            return Ok(None);
        }
        let mut b = RdfDatasetBuilder::new();
        copy_scoped(&mut b, graph, &mut ScopeMap::Identity);
        let mut next = max_scope(graph)
            .checked_add(1)
            .expect("blank-node scope counter exceeded u32::MAX");
        for (_, document) in &self.documents {
            let mut rescope = ScopeMap::Fresh {
                assigned: BTreeMap::new(),
                next: &mut next,
            };
            copy_scoped(&mut b, document, &mut rescope);
        }
        b.freeze().map(Some)
    }
}

/// Every ontology IRI in `graph`'s `owl:imports` closure that `graph` does not itself
/// resolve, deduplicated, in the order the closure walk first meets them.
///
/// `loaded` is the IRIs of the documents `graph` was read from: each one's retrieval IRI or
/// the base it was parsed under. Pass an empty slice when that is unknown. See the
/// [module documentation](self) for the rule; this is [`ImportMap::unresolved_imports`] of a
/// map that supplies no document.
#[must_use]
pub fn unresolved_imports(graph: &RdfDataset, loaded: &[&str]) -> Vec<String> {
    let mut map = ImportMap::new();
    for iri in loaded {
        map.declare_loaded(*iri);
    }
    map.unresolved_imports(graph)
}

/// Every ontology IRI `graph` imports, in the dataset's own frozen quad order.
///
/// Only IRI objects: `owl:imports` is defined to relate an ontology to an ontology IRI, and
/// a blank node or literal object is not one — such a triple names no document and cannot
/// make one missing. Whether an import still NEEDS a document is a different question,
/// answered by [`ImportMap::closure`].
#[must_use]
pub fn imported_iris(graph: &RdfDataset) -> Vec<String> {
    let Some(imports) = graph.term_id_by_iri(OWL_IMPORTS) else {
        return Vec::new();
    };
    graph
        .quads()
        .filter(|quad| quad.p == imports)
        .filter_map(|quad| match graph.term_value(quad.o) {
            TermValue::Iri(iri) => Some(iri),
            _ => None,
        })
        .collect()
}

/// Add every ontology IRI `graph` declares to `into`: each IRI typed `owl:Ontology`, and
/// each IRI some ontology names as its `owl:versionIRI`.
fn declared_ontologies(graph: &RdfDataset, into: &mut BTreeSet<String>) {
    let rdf_type = graph.term_id_by_iri(RDF_TYPE);
    let ontology = graph.term_id_by_iri(OWL_ONTOLOGY);
    let version_iri = graph.term_id_by_iri(OWL_VERSIONIRI);
    if version_iri.is_none() && (rdf_type.is_none() || ontology.is_none()) {
        return;
    }
    for quad in graph.quads() {
        let named = if Some(quad.p) == rdf_type && Some(quad.o) == ontology {
            quad.s
        } else if Some(quad.p) == version_iri {
            quad.o
        } else {
            continue;
        };
        if let TermValue::Iri(iri) = graph.term_value(named) {
            into.insert(iri);
        }
    }
}

/// Every term id `graph` holds, in every position [`copy_scoped`] writes: the quads, the
/// reifier bindings, the annotations and the declared named graphs.
///
/// A blank node may occur only as a reifier, only inside an annotation, or only as a declared
/// named graph; a survey blind to those three would report a maximum scope BELOW one the
/// graph actually uses, and the first merged document would be rescoped on top of it.
fn term_positions(graph: &RdfDataset) -> impl Iterator<Item = TermId> + '_ {
    let quads = graph.quads().flat_map(|quad| {
        [Some(quad.s), Some(quad.p), Some(quad.o), quad.g]
            .into_iter()
            .flatten()
    });
    let reifiers = graph
        .reifiers_with_graph()
        .flat_map(|(reifier, triple, g)| [Some(reifier), Some(triple), g].into_iter().flatten());
    let annotations = graph
        .annotations_with_graph()
        .flat_map(|(reifier, predicate, object, g)| {
            [Some(reifier), Some(predicate), Some(object), g]
                .into_iter()
                .flatten()
        });
    quads
        .chain(reifiers)
        .chain(annotations)
        .chain(graph.named_graphs())
}

/// The highest blank-node scope `graph` uses, so merged documents can be placed above it.
fn max_scope(graph: &RdfDataset) -> u32 {
    term_positions(graph)
        .map(|id| scope_of(&graph.term_value(id)))
        .max()
        .unwrap_or(0)
}

/// The highest blank-node scope a term mentions, recursing into triple terms.
fn scope_of(term: &TermValue) -> u32 {
    match term {
        TermValue::Blank { scope, .. } => scope.ordinal(),
        TermValue::Triple { s, p, o } => scope_of(s).max(scope_of(p)).max(scope_of(o)),
        TermValue::Iri(_) | TermValue::Literal { .. } => 0,
    }
}

/// How one document's blank-node scopes are written into a merge.
enum ScopeMap<'a> {
    /// Unchanged: the importing graph keeps the scopes the caller gave it.
    Identity,
    /// An injective renumbering into fresh scopes, shared across every document of one
    /// merge. Injective rather than "add a constant", because a document may already carry
    /// several scopes of its own and collapsing two of them would merge nodes the document
    /// keeps apart.
    Fresh {
        /// The document's own scope → the scope it was given here.
        assigned: BTreeMap<u32, BlankScope>,
        /// The next unused scope.
        next: &'a mut u32,
    },
}

impl ScopeMap<'_> {
    /// The scope `scope` is written under.
    fn map(&mut self, scope: BlankScope) -> BlankScope {
        match self {
            Self::Identity => scope,
            Self::Fresh { assigned, next } => {
                if let Some(&given) = assigned.get(&scope.ordinal()) {
                    return given;
                }
                let given = BlankScope(**next);
                **next = next
                    .checked_add(1)
                    .expect("blank-node scope counter exceeded u32::MAX");
                assigned.insert(scope.ordinal(), given);
                given
            }
        }
    }
}

/// Copy every table of `graph` into `b`, rewriting blank-node scopes through `rescope`.
fn copy_scoped(b: &mut RdfDatasetBuilder, graph: &RdfDataset, rescope: &mut ScopeMap<'_>) {
    let term = |b: &mut RdfDatasetBuilder, id: TermId, rescope: &mut ScopeMap<'_>| {
        intern_scoped(b, &graph.term_value(id), rescope)
    };
    for quad in graph.quads() {
        let s = term(b, quad.s, rescope);
        let p = term(b, quad.p, rescope);
        let o = term(b, quad.o, rescope);
        let g = quad.g.map(|g| term(b, g, rescope));
        b.push_quad(s, p, o, g);
    }
    for (reifier, triple, graph_name) in graph.reifiers_with_graph() {
        let reifier = term(b, reifier, rescope);
        let triple = term(b, triple, rescope);
        let graph_name = graph_name.map(|g| term(b, g, rescope));
        b.push_reifier_in_graph(reifier, triple, graph_name);
    }
    for (reifier, predicate, object, graph_name) in graph.annotations_with_graph() {
        let reifier = term(b, reifier, rescope);
        let predicate = term(b, predicate, rescope);
        let object = term(b, object, rescope);
        let graph_name = graph_name.map(|g| term(b, g, rescope));
        b.push_annotation_in_graph(reifier, predicate, object, graph_name);
    }
    for graph_name in graph.named_graphs() {
        let g = term(b, graph_name, rescope);
        b.declare_named_graph(g);
    }
}

/// Intern `value` into `b`, rewriting every blank-node scope through `rescope`.
fn intern_scoped(
    b: &mut RdfDatasetBuilder,
    value: &TermValue,
    rescope: &mut ScopeMap<'_>,
) -> TermId {
    match value {
        TermValue::Iri(iri) => b.intern_iri(iri),
        TermValue::Blank { label, scope } => b.intern_blank(label, rescope.map(*scope)),
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => b.intern_literal(RdfLiteral {
            lexical_form: lexical_form.clone(),
            datatype: Some(datatype.clone()),
            language: language.clone(),
            direction: *direction,
        }),
        TermValue::Triple { s, p, o } => {
            let s = intern_scoped(b, s, rescope);
            let p = intern_scoped(b, p, rescope);
            let o = intern_scoped(b, o, rescope);
            b.intern_triple(s, p, o)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{
        ImportMap, OWL_IMPORTS, OWL_ONTOLOGY, OWL_VERSIONIRI, RDF_TYPE, imported_iris,
        unresolved_imports,
    };
    use crate::ir::{BlankScope, RdfDataset, RdfDatasetBuilder, TermValue};

    /// The empty answer, spelled once for `assert_eq!`.
    const NONE: [String; 0] = [];

    /// A dataset of the given IRI triples.
    fn triples(rows: &[(&str, &str, &str)]) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        for (s, p, o) in rows {
            let s = b.intern_iri(s);
            let p = b.intern_iri(p);
            let o = b.intern_iri(o);
            b.push_quad(s, p, o, None);
        }
        b.freeze().expect("freeze")
    }

    /// A one-triple document `_:label ex:p <object>`, plus `owl:imports` of each target.
    fn document(label: &str, object: &str, imports: &[&str]) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_blank(label, BlankScope::DEFAULT);
        let p = b.intern_iri("http://example.org/p");
        let o = b.intern_iri(object);
        b.push_quad(s, p, o, None);
        for target in imports {
            let ontology = b.intern_iri("http://example.org/self");
            let predicate = b.intern_iri(OWL_IMPORTS);
            let target = b.intern_iri(target);
            b.push_quad(ontology, predicate, target, None);
        }
        b.freeze().expect("freeze")
    }

    /// Every IRI object of `ds`'s quads.
    fn iri_objects(ds: &RdfDataset) -> Vec<String> {
        ds.quads()
            .filter_map(|quad| match ds.term_value(quad.o) {
                TermValue::Iri(iri) => Some(iri),
                _ => None,
            })
            .collect()
    }

    /// The four ways an import is in hand, each beside the neighbour that differs only in
    /// the one fact the rule reads, so every "resolved" answer is observed against a
    /// "missing" one.
    #[test]
    fn each_resolution_route_resolves_and_its_neighbour_does_not() {
        const LIB: &str = "http://example.org/lib";
        const OTHER: &str = "http://example.org/other";
        let importer = |extra: &[(&str, &str, &str)]| {
            let mut rows = vec![("http://example.org/shapes", OWL_IMPORTS, LIB)];
            rows.extend_from_slice(extra);
            triples(&rows)
        };

        // Declared as an ontology in the graph.
        assert_eq!(
            unresolved_imports(&importer(&[(LIB, RDF_TYPE, OWL_ONTOLOGY)]), &[]),
            NONE
        );
        assert_eq!(
            unresolved_imports(&importer(&[(OTHER, RDF_TYPE, OWL_ONTOLOGY)]), &[]),
            vec![LIB.to_owned()]
        );

        // Named as some ontology's version IRI.
        assert_eq!(
            unresolved_imports(&importer(&[(OTHER, OWL_VERSIONIRI, LIB)]), &[]),
            NONE
        );
        assert_eq!(
            unresolved_imports(
                &importer(&[(OTHER, OWL_VERSIONIRI, "http://example.org/lib/2")]),
                &[]
            ),
            vec![LIB.to_owned()]
        );

        // A document the graph was read from.
        assert_eq!(unresolved_imports(&importer(&[]), &[LIB]), NONE);
        assert_eq!(
            unresolved_imports(&importer(&[]), &[OTHER]),
            vec![LIB.to_owned()]
        );

        // A document the map supplies.
        let mut map = ImportMap::new();
        map.insert(LIB, triples(&[]));
        assert_eq!(map.closure(&importer(&[])).unresolved(), NONE);
        let mut other = ImportMap::new();
        other.insert(OTHER, triples(&[]));
        assert_eq!(other.closure(&importer(&[])).unresolved(), [LIB.to_owned()]);
    }

    /// A supplied document no import reaches is reported; the neighbour whose graph DOES
    /// import it reports nothing unreached.
    #[test]
    fn an_entry_nothing_imports_is_unreached() {
        const LIB: &str = "http://example.org/lib";
        let mut map = ImportMap::new();
        map.insert(LIB, triples(&[]));

        let imports_nothing = triples(&[("http://example.org/s", RDF_TYPE, OWL_ONTOLOGY)]);
        assert_eq!(map.closure(&imports_nothing).unreached(), [LIB.to_owned()]);

        let imports_lib = triples(&[("http://example.org/s", OWL_IMPORTS, LIB)]);
        assert_eq!(map.closure(&imports_lib).unreached(), NONE);
    }

    /// The merge carries every reached document, transitively and through a cycle, and the
    /// importing graph's blank node keeps its scope while each document's is moved apart.
    #[test]
    fn the_merge_is_transitive_cycle_safe_and_standardized_apart() {
        let graph = document("b", "http://example.org/o", &["http://example.org/a"]);
        let mut map = ImportMap::new();
        map.insert(
            "http://example.org/a",
            document("b", "http://example.org/a-said", &["http://example.org/c"]),
        );
        map.insert(
            "http://example.org/c",
            document("b", "http://example.org/c-said", &["http://example.org/a"]),
        );
        let closure = map.closure(&graph);
        assert_eq!(closure.unresolved(), NONE);
        let merged = closure
            .merge(&graph)
            .expect("freeze")
            .expect("two documents were reached");
        let objects = iri_objects(&merged);
        for said in [
            "http://example.org/o",
            "http://example.org/a-said",
            "http://example.org/c-said",
        ] {
            assert!(objects.iter().any(|o| o == said), "{said} is missing");
        }
        let mut scopes: Vec<u32> = merged
            .quads()
            .filter_map(|quad| match merged.term_value(quad.s) {
                TermValue::Blank { label, scope } if label == "b" => Some(scope.ordinal()),
                _ => None,
            })
            .collect();
        scopes.sort_unstable();
        scopes.dedup();
        assert_eq!(
            scopes.len(),
            3,
            "three documents, three `_:b` nodes: {scopes:?}"
        );
        assert!(scopes.contains(&BlankScope::DEFAULT.ordinal()));
    }

    /// No reached document, no copy.
    #[test]
    fn a_closure_that_reached_nothing_is_not_copied() {
        let graph = triples(&[
            (
                "http://example.org/s",
                OWL_IMPORTS,
                "http://example.org/lib",
            ),
            ("http://example.org/lib", RDF_TYPE, OWL_ONTOLOGY),
        ]);
        let closure = ImportMap::new().closure(&graph);
        assert_eq!(closure.unresolved(), NONE);
        assert!(closure.merge(&graph).expect("freeze").is_none());
        assert_eq!(
            imported_iris(&graph),
            vec!["http://example.org/lib".to_owned()]
        );
    }
}
