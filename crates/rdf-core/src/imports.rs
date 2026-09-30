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
//! # Which `owl:imports` triple is an import
//!
//! Not every `owl:imports` triple is one. OWL 2 reads an import only off the importing
//! document's ONTOLOGY HEADER. *OWL 2 Web Ontology Language Mapping to RDF Graphs* §3.1.1:
//!
//! > If G contains a pair of triples of the form `x rdf:type owl:Ontology .` `x owl:imports
//! > *:y .` ... the document accessible from the IRI *:y is retrieved
//!
//! and §3.1.2:
//!
//! > the ontology header is extracted from G by matching patterns from Table 4 ... The set
//! > Imp(G) of the IRIs of ontology documents that are directly imported into G contains
//! > exactly all *:z1, ..., *:zk that are matched in the pattern.
//!
//! Table 4's patterns are `*:x rdf:type owl:Ontology . [*:x owl:versionIRI *:y .] *:x
//! owl:imports *:z1 ...` and, for an anonymous ontology, `_:x rdf:type owl:Ontology . _:x
//! owl:imports *:z1 ...`. SHACL 1.2 Core §6.1 reads a shapes graph's imports from the shapes
//! graph's own IRI:
//!
//! > As a pre-validation step, SHACL processors should extend the originally provided shapes
//! > graph by transitively following and importing all referenced shapes graphs through the
//! > owl:imports predicate. When resolving an imported IRI, if the retrieved graph contains a
//! > triple with the imported IRI as the object of owl:versionIRI, the processor should treat
//! > the subject of that triple as the shapes graph IRI of the imported graph for the purpose
//! > of following further owl:imports statements. Formally, processors should use the
//! > property path ^owl:versionIRI?/owl:imports iteratively
//!
//! and names the class a document declares a shapes graph with: "The sh:ShapesGraph class MAY
//! be used as an rdf:type of the IRI of a graph that typically acts in the role of a shapes
//! graph." PurRDF treats every SHOULD as a MUST, and OWL 2's Table 4 as the floor of the rule,
//! not its limit. Together they give the rule: an `owl:imports` triple is an import exactly
//! when its subject is an ANCHOR of the document it occurs in, and the anchors of a document
//! are
//!
//! * each IRI the document was loaded under — the IRI a caller read it from or parsed it
//!   under ([`ImportMap::declare_loaded`]) for the importing graph, and for an imported
//!   document the IRI it was imported by;
//! * each subject, IRI or blank node, that is a SHACL instance of `owl:Ontology` — its OWL 2
//!   header;
//! * EVERY subject that is a SHACL instance of `sh:ShapesGraph` — `sh:RulesGraph` and any
//!   class the document declares `rdfs:subClassOf*` `sh:ShapesGraph` included. A document may
//!   declare several shapes graphs, and each one's imports are imports;
//! * each subject `s` of a triple `s owl:versionIRI a` whose object `a` is one of the
//!   anchors above — the `^owl:versionIRI?` step.
//!
//! The second and third are the kernel's ONE graph-role classifier,
//! [`crate::graph_roles`], which SHACL-SPARQL's implicit prefix collection selects from too.
//!
//! A node whose only graph role is `sh:DataGraph` is NOT an anchor. SHACL 1.2 Core §6.2:
//!
//! > owl:imports in the data graph is not enacted, in order to avoid uncontrolled increase of
//! > validation work.
//!
//! and its next note: "when using statements of the form X a sh:DataGraph ., X a owl:Ontology
//! . should also be included" — the `owl:Ontology` type is what makes such a triple a
//! directive. (A document a caller LOADS — as a shapes graph, or as an entailment premise — is
//! in that role whatever it types itself, so its loaded IRI stays an anchor.) The W3C
//! `shacl-shacl.ttl` checks the same thing from the other side: its `shsh:DataGraphImportsShape`
//! reports a `sh:DataGraph` that uses `owl:imports` without the type `owl:Ontology`.
//!
//! Any other `owl:imports` triple — one whose subject is some node of the document that is
//! none of these — imports nothing. It is DATA: it stays in the graph exactly as written, a
//! SHACL shape may constrain it and a query may match it, but no document is looked for and
//! none can be missing. [`unanchored_import_triples`] and [`ImportMap::unanchored_imports`]
//! list them, so a tool can show an author which of their `owl:imports` are not imports. The
//! W3C SHACL suite relies on exactly that: `sparql/component/validator-001` writes
//! `owl:imports <http://datashapes.org/dash>` on a node that is neither the test document's
//! IRI nor a declared graph, and its expected report is computed without DASH. Likewise
//! SHACL-SPARQL's `sh:prefixes/owl:imports*/sh:declare` path walks `owl:imports` edges between
//! prefix-declaring nodes of the shapes graph; that is prefix collection over the graph as it
//! stands, not a document import, and it lives with the SPARQL prefix code.
//!
//! A DATA graph's imports are never read by this module at all: SHACL validation resolves the
//! shapes graph's closure and hands the data graph to the validator as given.
//!
//! [`imported_iris`] is the rule, and every consumer in this workspace takes its imports from
//! it.
//!
//! # When an import is in hand
//!
//! An import `X` is RESOLVED when the document or ontology `X` names is already in hand:
//!
//! * the caller's [`ImportMap`] supplies a document for `X`;
//! * `X` names a document the graph was READ from — its retrieval IRI or the base it was
//!   parsed under ([`ImportMap::declare_loaded`]) — so a document importing itself names the
//!   document being read;
//! * `X` is an anchor the closure itself declares — an ontology header (`X rdf:type
//!   owl:Ontology`, the header OWL 2's RDF mapping reads an ontology's IRI from) or a shapes
//!   graph (`X rdf:type sh:ShapesGraph`), each through `rdfs:subClassOf*`; or
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
//! A supplied document's own imports are read by the same rule, from its own anchors: the
//! IRI it was imported by, its own ontology headers and shapes graphs, and the subjects that
//! name one of those as their `owl:versionIRI`. Whether an unsupplied import is declared in
//! the closure is decided only once the walk is complete, because a document reached LATER
//! may be the one that declares it.
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
use std::convert::Infallible;
use std::ops::ControlFlow;
use std::sync::Arc;

use crate::RdfDiagnostic;
use crate::dataset_view::{DatasetView, GraphMatch};
use crate::graph_roles::GraphRoleIndex;
use crate::ir::{
    BlankScope, Nested, RdfDataset, RdfDatasetBuilder, TermId, TermRef, TermValue, try_fold_nested,
};
use crate::model::RdfLiteral;

/// `owl:imports`.
use purrdf_iri::vocab::owl::IMPORTS as OWL_IMPORTS;
/// `owl:versionIRI`.
use purrdf_iri::vocab::owl::VERSION_IRI as OWL_VERSIONIRI;

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
/// let rdf_type = b.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
/// let owl_ontology = b.intern_iri("http://www.w3.org/2002/07/owl#Ontology");
/// let imports = b.intern_iri("http://www.w3.org/2002/07/owl#imports");
/// let other = b.intern_iri("http://example.org/other");
/// b.push_quad(ontology, rdf_type, owl_ontology, None);
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
    /// IRI or base. Each is an anchor of the importing graph, and an import of one of these
    /// names a document already in hand.
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
    ///
    /// The IRI is also an ANCHOR of the importing graph: an `owl:imports` whose subject is
    /// `iri` is one of the graph's imports (see the [module documentation](self)).
    pub fn declare_loaded(&mut self, iri: impl Into<String>) -> bool {
        self.loaded.insert(iri.into())
    }

    /// Whether `iri` was declared with [`declare_loaded`](Self::declare_loaded).
    #[must_use]
    pub fn is_loaded(&self, iri: &str) -> bool {
        self.loaded.contains(iri)
    }

    /// Every ontology IRI `graph` imports, read from the anchors this map declares loaded
    /// and the graph's own ontology headers and shapes graphs — [`imported_iris`] with this
    /// map's [`declare_loaded`](Self::declare_loaded) IRIs as `loaded`.
    #[must_use]
    pub fn imported_iris(&self, graph: &RdfDataset) -> Vec<String> {
        imported_iris(graph, &self.loaded_iris())
    }

    /// Every IRI declared with [`declare_loaded`](Self::declare_loaded), in IRI order.
    pub fn loaded(&self) -> impl Iterator<Item = &str> + '_ {
        self.loaded.iter().map(String::as_str)
    }

    /// The [`declare_loaded`](Self::declare_loaded) IRIs, in IRI order.
    fn loaded_iris(&self) -> Vec<&str> {
        self.loaded.iter().map(String::as_str).collect()
    }

    /// Every `owl:imports` triple of `graph`'s closure that is NOT an import, because its
    /// subject is no anchor of the document it occurs in — `graph` read under this map's
    /// loaded IRIs, and each supplied document the closure reaches read under the IRI it was
    /// imported by (see the [module documentation](self)).
    ///
    /// The importing graph's triples come first, then each reached document's, in walk
    /// order; within a document, in frozen quad order, each `(subject, object)` once. This
    /// is a COLD survey for tools that show an author which `owl:imports` are data: it walks
    /// the closure again, and the hot resolution path never calls it.
    ///
    /// ```
    /// use purrdf_core::{RdfDatasetBuilder, TermValue};
    /// use purrdf_core::imports::ImportMap;
    ///
    /// let mut b = RdfDatasetBuilder::new();
    /// let node = b.intern_iri("http://example.org/node");
    /// let imports = b.intern_iri("http://www.w3.org/2002/07/owl#imports");
    /// let lib = b.intern_iri("http://example.org/lib");
    /// b.push_quad(node, imports, lib, None);
    /// let graph = b.freeze().expect("freeze");
    ///
    /// let listed = ImportMap::new().unanchored_imports(&graph);
    /// assert_eq!(listed.len(), 1);
    /// assert_eq!(listed[0].document, None);
    /// assert_eq!(listed[0].subject, TermValue::iri("http://example.org/node"));
    ///
    /// // Read under the node's IRI, the same triple is an import, and nothing is listed.
    /// let mut map = ImportMap::new();
    /// map.declare_loaded("http://example.org/node");
    /// assert!(map.unanchored_imports(&graph).is_empty());
    /// ```
    #[must_use]
    pub fn unanchored_imports(&self, graph: &RdfDataset) -> Vec<UnanchoredImport> {
        let mut out: Vec<UnanchoredImport> = Vec::new();
        let mut survey = |document: Option<&str>, dataset: &RdfDataset, loaded: &[&str]| {
            for (subject, object) in unanchored_import_triples(dataset, loaded) {
                out.push(UnanchoredImport {
                    document: document.map(ToOwned::to_owned),
                    subject: dataset.term_value(subject),
                    object: dataset.term_value(object),
                });
            }
        };
        survey(None, graph, &self.loaded_iris());
        for (iri, document) in self.closure(graph).documents() {
            survey(Some(iri), document, &[iri.as_str()]);
        }
        out
    }

    /// Walk `graph`'s transitive `owl:imports` closure against this map, to a fixpoint.
    ///
    /// Breadth-first, each IRI visited once, so a cycle terminates. A map-supplied document
    /// wins over an in-graph declaration of the same ontology: the caller named that
    /// document, and merging it is what the caller asked for.
    ///
    /// `graph`'s imports are read from its anchors — the IRIs this map
    /// [declares loaded](Self::declare_loaded) and its own ontology headers — and each
    /// supplied document's from its own: the IRI it was imported by and its own headers
    /// (see the [module documentation](self)).
    #[must_use]
    pub fn closure(&self, graph: &RdfDataset) -> ImportClosure {
        self.closure_with_links(graph, &[])
    }

    /// [`closure`](Self::closure), with `links` as further roots of the walk: IRIs that name
    /// graphs to fold in beside `graph`'s own imports, each resolved exactly as an import of
    /// `graph` is.
    ///
    /// SHACL 1.2 Core §6.4 is the caller: a data graph's `sh:shapesGraph` values name graphs
    /// that "SHOULD be included into the shapes graph used to validate the data graph", and
    /// "the same strategy of resolving a shapes graph IRI from a version IRI, described for
    /// Shapes Graphs, applies here". So a link is resolved by the same table, by the same
    /// in-place declarations (a loaded IRI, an anchor the closure declares, an
    /// `owl:versionIRI`), and a supplied linked document's own imports are followed from its
    /// own anchors — the IRI it was linked by, its headers and shapes graphs, and the
    /// `^owl:versionIRI` step from them. A link is visited, so a map entry only a link names
    /// is not [`unreached`](ImportClosure::unreached); an unresolved link is reported in
    /// [`unresolved`](ImportClosure::unresolved) like any import, and the caller tells the two
    /// apart by its own `links`.
    #[must_use]
    pub fn closure_with_links(&self, graph: &RdfDataset, links: &[String]) -> ImportClosure {
        let mut queue: VecDeque<String> = links.iter().cloned().collect();
        queue.extend(self.imported_iris(graph));
        if queue.is_empty() {
            // The common case — a graph that imports nothing — costs one term lookup and
            // no survey of the graph's ontology declarations.
            return ImportClosure {
                documents: Vec::new(),
                unresolved: Vec::new(),
                unreached: self.documents.keys().cloned().collect(),
                conflicts: Vec::new(),
            };
        }
        let mut declared: BTreeSet<String> = self.loaded.clone();
        Self::declared_targets(graph, &mut declared);
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
            Self::declared_targets(document, &mut declared);
            queue.extend(imported_iris(document.as_ref(), &[iri.as_str()]));
            documents.push((iri, Arc::clone(document)));
        }
        unsupplied.retain(|iri| !declared.contains(iri));
        let unreached = self
            .documents
            .keys()
            .filter(|iri| !seen.contains(*iri))
            .cloned()
            .collect();
        let conflicts = version_conflicts(graph, &self.loaded_iris(), &documents);
        ImportClosure {
            documents,
            unresolved: unsupplied,
            unreached,
            conflicts,
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
    /// // Read under `ex:o`, the graph imports `ex:other`, and nothing supplies it.
    /// let mut map = ImportMap::new();
    /// map.declare_loaded("http://example.org/o");
    /// assert_eq!(
    ///     map.unresolved_imports(&graph),
    ///     vec!["http://example.org/other".to_owned()]
    /// );
    /// // Read under no IRI, `ex:o` is no anchor and the triple imports nothing.
    /// assert!(ImportMap::new().unresolved_imports(&graph).is_empty());
    /// ```
    #[must_use]
    pub fn unresolved_imports(&self, graph: &RdfDataset) -> Vec<String> {
        self.closure(graph).unresolved
    }

    /// Add every import target `graph` resolves in place to `into`: each IRI the graph
    /// declares an import anchor — an ontology header or a shapes graph, by the one
    /// classifier ([`crate::graph_roles`]) — and each IRI some ontology names as its
    /// `owl:versionIRI`.
    ///
    /// Indexed lookups only: the classifier reads the `rdf:type` quads of the graph classes,
    /// and the version step reads the `owl:versionIRI` quads.
    fn declared_targets(graph: &RdfDataset, into: &mut BTreeSet<String>) {
        let mut named: Vec<TermId> = GraphRoleIndex::classify(graph).import_anchors().collect();
        if let Some(version_iri) = graph.term_id_by_iri(OWL_VERSIONIRI) {
            named.extend(
                graph
                    .quads_for_pattern(None, Some(version_iri), None, GraphMatch::Any)
                    .map(|quad| quad.o),
            );
        }
        for id in named {
            if let TermValue::Iri(iri) = graph.term_value(id) {
                into.insert(iri);
            }
        }
    }
}

/// `owl:incompatibleWith`.
use purrdf_iri::vocab::owl::INCOMPATIBLE_WITH as OWL_INCOMPATIBLE_WITH;

/// Two graphs of one import closure that the closure should not hold together.
///
/// *OWL 2 Web Ontology Language Structural Specification* §3.4: "The import closure of O
/// SHOULD NOT contain ontologies O1 and O2 such that O1 and O2 are different ontology
/// versions from the same ontology series, or O1 contains an ontology annotation
/// owl:incompatibleWith with the value equal to either the ontology IRI or the version IRI of
/// O2." SHACL 1.2 Core §1.3 makes the same two conditions part of a well-formed shapes graph
/// ("Its import closure ... does not contain two shapes graphs where: they are different
/// versions of the same series (i.e., they share the same shapes graph IRI but have different
/// owl:versionIRI values), or one contains an owl:incompatibleWith annotation whose value is
/// equal to either the shapes graph IRI or the owl:versionIRI of the other"), and its §6.1
/// note repeats it: the import closure "SHOULD NOT contain two graphs that are different
/// versions of the same series". PurRDF reads SHOULD NOT as MUST NOT, so every engine refuses
/// a closure with a conflict.
///
/// A graph of the closure is the importing graph or one supplied document. Its names are the
/// IRIs of its anchors (the IRI it was loaded or imported under, its ontology headers and its
/// shapes graphs) and the `owl:versionIRI` of each; its versions are the `owl:versionIRI`
/// values of its anchors; and what it declares incompatible is the `owl:incompatibleWith`
/// values of its anchors. Only two DIFFERENT graphs conflict: a graph that names itself in an
/// `owl:incompatibleWith`, or a document supplied under two IRIs, is not a pair of versions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionConflict {
    /// The first graph: `None` for the importing graph, or the IRI a supplied document was
    /// imported under.
    pub first: Option<String>,
    /// The second graph, spelled as [`Self::first`] is.
    pub second: Option<String>,
    /// What makes the two conflict.
    pub kind: VersionConflictKind,
}

/// Why two graphs of one import closure conflict. See [`VersionConflict`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionConflictKind {
    /// Both graphs declare the ontology or shapes graph `series`, with different
    /// `owl:versionIRI` values.
    SameSeries {
        /// The series IRI both graphs declare.
        series: String,
        /// A version IRI the first graph gives the series and the second does not.
        first_version: String,
        /// A version IRI the second graph gives the series and the first does not.
        second_version: String,
    },
    /// The first graph declares `owl:incompatibleWith <iri>`, and `iri` is a name of the
    /// second graph — its ontology or shapes graph IRI, or its `owl:versionIRI`.
    IncompatibleWith {
        /// The `owl:incompatibleWith` value.
        iri: String,
    },
}

impl std::fmt::Display for VersionConflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let graph = |g: &Option<String>| {
            g.as_ref().map_or_else(
                || "the importing graph".to_owned(),
                |iri| format!("<{iri}>"),
            )
        };
        match &self.kind {
            VersionConflictKind::SameSeries {
                series,
                first_version,
                second_version,
            } => write!(
                f,
                "{} and {} are different versions of the series <{series}> (owl:versionIRI \
                 <{first_version}> and <{second_version}>)",
                graph(&self.first),
                graph(&self.second),
            ),
            VersionConflictKind::IncompatibleWith { iri } => write!(
                f,
                "{} declares owl:incompatibleWith <{iri}>, which names {}",
                graph(&self.first),
                graph(&self.second),
            ),
        }
    }
}

/// One graph of a closure as the version constraints read it. See [`VersionConflict`].
struct ClosureGraphIdentity {
    /// The graph: `None` for the importing graph, or the IRI it was imported under.
    graph: Option<String>,
    /// Its anchors' IRIs and their version IRIs.
    names: BTreeSet<String>,
    /// Series IRI → the version IRIs this graph gives it.
    versions: BTreeMap<String, BTreeSet<String>>,
    /// The `owl:incompatibleWith` IRI values of its anchors.
    incompatible: BTreeSet<String>,
}

impl ClosureGraphIdentity {
    /// Read `dataset`'s identity, loaded under `loaded`.
    fn read(graph: Option<String>, dataset: &RdfDataset, loaded: &[&str]) -> Self {
        let mut names: BTreeSet<String> = loaded.iter().map(|iri| (*iri).to_owned()).collect();
        let mut versions: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut incompatible: BTreeSet<String> = BTreeSet::new();
        let version_iri = dataset.term_id_by_iri(OWL_VERSIONIRI);
        let incompatible_with = dataset.term_id_by_iri(OWL_INCOMPATIBLE_WITH);
        let objects = |subject: TermId, predicate: Option<TermId>| -> Vec<String> {
            predicate.map_or_else(Vec::new, |predicate| {
                dataset
                    .objects(subject, predicate, GraphMatch::Any)
                    .into_iter()
                    .filter_map(|object| match dataset.term_value(object) {
                        TermValue::Iri(iri) => Some(iri),
                        _ => None,
                    })
                    .collect()
            })
        };
        for anchor in anchors(dataset, loaded) {
            incompatible.extend(objects(anchor, incompatible_with));
            // A series is named by an IRI; an anonymous ontology has no version.
            let TermValue::Iri(series) = dataset.term_value(anchor) else {
                continue;
            };
            let declared = objects(anchor, version_iri);
            names.insert(series.clone());
            names.extend(declared.iter().cloned());
            if !declared.is_empty() {
                versions.entry(series).or_default().extend(declared);
            }
        }
        Self {
            graph,
            names,
            versions,
            incompatible,
        }
    }

    /// Every way `self` and `other` conflict, `self` first.
    fn conflicts_with(&self, other: &Self, out: &mut Vec<VersionConflict>) {
        let pair = |kind| VersionConflict {
            first: self.graph.clone(),
            second: other.graph.clone(),
            kind,
        };
        for (series, mine) in &self.versions {
            let Some(theirs) = other.versions.get(series) else {
                continue;
            };
            if mine == theirs {
                continue;
            }
            let first_version = mine
                .difference(theirs)
                .next()
                .unwrap_or_else(|| mine.iter().next().expect("a declared series has a version"));
            let second_version = theirs.difference(mine).next().unwrap_or_else(|| {
                theirs
                    .iter()
                    .next()
                    .expect("a declared series has a version")
            });
            out.push(pair(VersionConflictKind::SameSeries {
                series: series.clone(),
                first_version: first_version.clone(),
                second_version: second_version.clone(),
            }));
        }
        for (declarer, target) in [(self, other), (other, self)] {
            for iri in &declarer.incompatible {
                if target.names.contains(iri) && !declarer.names.contains(iri) {
                    out.push(VersionConflict {
                        first: declarer.graph.clone(),
                        second: target.graph.clone(),
                        kind: VersionConflictKind::IncompatibleWith { iri: iri.clone() },
                    });
                }
            }
        }
    }
}

/// Every [`VersionConflict`] between two graphs of a closure: `graph` read under `loaded`,
/// and each supplied `documents` entry read under the IRI it was imported by.
///
/// A closure that reached no document is one graph, which cannot conflict with itself, so
/// the common case costs nothing.
fn version_conflicts(
    graph: &RdfDataset,
    loaded: &[&str],
    documents: &[(String, Arc<RdfDataset>)],
) -> Vec<VersionConflict> {
    if documents.is_empty() {
        return Vec::new();
    }
    let mut graphs = vec![ClosureGraphIdentity::read(None, graph, loaded)];
    for (iri, document) in documents {
        graphs.push(ClosureGraphIdentity::read(
            Some(iri.clone()),
            document,
            &[iri.as_str()],
        ));
    }
    let mut conflicts = Vec::new();
    for (index, first) in graphs.iter().enumerate() {
        for second in &graphs[index + 1..] {
            first.conflicts_with(second, &mut conflicts);
        }
    }
    conflicts
}

/// Every IRI `graph` resolves an import of IN PLACE, with no document supplied: each IRI the
/// graph declares an import anchor — a SHACL instance of `owl:Ontology` or `sh:ShapesGraph`
/// ([`crate::graph_roles`]) — and each IRI some node of the graph names as its
/// `owl:versionIRI`. The same set [`ImportMap::closure`] consults before it calls an import
/// unresolved (see the [module documentation](self), "When an import is in hand").
///
/// Indexed lookups only; a graph that interns neither `rdf:type` nor `owl:versionIRI` costs
/// two term lookups.
///
/// ```
/// use purrdf_core::RdfDatasetBuilder;
/// use purrdf_core::imports::declared_import_targets;
///
/// let mut b = RdfDatasetBuilder::new();
/// let rdf_type = b.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
/// let shapes_graph = b.intern_iri("http://www.w3.org/ns/shacl#ShapesGraph");
/// let version_iri = b.intern_iri("http://www.w3.org/2002/07/owl#versionIRI");
/// let g = b.intern_iri("http://example.org/g");
/// let v1 = b.intern_iri("http://example.org/g/1");
/// b.push_quad(g, rdf_type, shapes_graph, None);
/// b.push_quad(g, version_iri, v1, None);
/// let graph = b.freeze().expect("freeze");
///
/// let targets = declared_import_targets(&graph);
/// assert!(targets.contains("http://example.org/g"));
/// assert!(targets.contains("http://example.org/g/1"));
/// assert_eq!(targets.len(), 2);
/// ```
#[must_use]
pub fn declared_import_targets(graph: &RdfDataset) -> BTreeSet<String> {
    let mut declared = BTreeSet::new();
    ImportMap::declared_targets(graph, &mut declared);
    declared
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
    /// Each pair of graphs of the closure that OWL 2 §3.4 says it should not hold together.
    conflicts: Vec<VersionConflict>,
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

    /// Every pair of graphs this closure holds that are different versions of one series, or
    /// of which one declares `owl:incompatibleWith` the other — in walk order of the first
    /// graph, then of the second. Empty exactly when the closure meets the import-closure
    /// constraints of OWL 2 §3.4 (see [`VersionConflict`]).
    #[must_use]
    pub fn conflicts(&self) -> &[VersionConflict] {
        &self.conflicts
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

/// Every ontology IRI `graph` imports, in the dataset's own frozen quad order: the IRI
/// object of each `owl:imports` triple whose subject is an ANCHOR of `graph`.
///
/// `loaded` is the IRIs `graph` was loaded under — the IRI a caller read it from or parsed it
/// under, or, for an imported document, the IRI it was imported by. Those, every SHACL
/// instance of `owl:Ontology` or of `sh:ShapesGraph` in `graph` (IRI or blank node; see
/// [`crate::graph_roles`]), and every subject naming one of them as its `owl:versionIRI` are
/// the anchors; an `owl:imports` on any other subject — a node whose only graph role is
/// `sh:DataGraph` included — is data and imports nothing. See the
/// [module documentation](self) for the specification text this implements.
///
/// Only IRI objects: `owl:imports` is defined to relate an ontology to an ontology IRI, and
/// a blank node or literal object is not one — such a triple names no document and cannot
/// make one missing. Whether an import still NEEDS a document is a different question,
/// answered by [`ImportMap::closure`].
///
/// ```
/// use purrdf_core::RdfDatasetBuilder;
/// use purrdf_core::imports::imported_iris;
///
/// let mut b = RdfDatasetBuilder::new();
/// let doc = b.intern_iri("http://example.org/doc");
/// let other = b.intern_iri("http://example.org/other");
/// let imports = b.intern_iri("http://www.w3.org/2002/07/owl#imports");
/// let lib = b.intern_iri("http://example.org/lib");
/// let data = b.intern_iri("http://example.org/data");
/// b.push_quad(doc, imports, lib, None);
/// b.push_quad(other, imports, data, None);
/// let graph = b.freeze().expect("freeze");
///
/// // Read under `doc`, the document imports `lib`; `other`'s triple is data.
/// assert_eq!(imported_iris(&graph, &["http://example.org/doc"]), ["http://example.org/lib"]);
/// // Read under no IRI, and with no ontology header, it imports nothing.
/// assert!(imported_iris(&graph, &[]).is_empty());
/// ```
#[must_use]
pub fn imported_iris<D: DatasetView>(graph: &D, loaded: &[&str]) -> Vec<String> {
    // The common case — a document that imports nothing — costs this one term lookup, and
    // the graph-role classifier never runs.
    let Some(imports) = graph.term_id_by_value(&TermValue::iri(OWL_IMPORTS)) else {
        return Vec::new();
    };
    let anchors = anchors(graph, loaded);
    if anchors.is_empty() {
        return Vec::new();
    }
    graph
        .quads()
        .filter(|quad| quad.p == imports && anchors.binary_search(&quad.s).is_ok())
        .filter_map(|quad| match graph.resolve(quad.o) {
            TermRef::Iri(iri) => Some(iri.to_owned()),
            _ => None,
        })
        .collect()
}

/// Every `owl:imports` triple of `graph` that is NOT an import — its subject is no anchor of
/// `graph` read under `loaded` — as `(subject, object)` ids, in frozen quad order, each pair
/// once. The complement of [`imported_iris`] over the same anchors; see the
/// [module documentation](self).
///
/// ```
/// use purrdf_core::RdfDatasetBuilder;
/// use purrdf_core::imports::unanchored_import_triples;
///
/// let mut b = RdfDatasetBuilder::new();
/// let rdf_type = b.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
/// let data_graph = b.intern_iri("http://www.w3.org/ns/shacl#DataGraph");
/// let imports = b.intern_iri("http://www.w3.org/2002/07/owl#imports");
/// let data = b.intern_iri("http://example.org/data");
/// let lib = b.intern_iri("http://example.org/lib");
/// b.push_quad(data, rdf_type, data_graph, None);
/// b.push_quad(data, imports, lib, None);
/// let graph = b.freeze().expect("freeze");
///
/// // A node whose only graph role is sh:DataGraph anchors no import.
/// assert_eq!(unanchored_import_triples(graph.as_ref(), &[]), [(data, lib)]);
/// ```
#[must_use]
pub fn unanchored_import_triples<D: DatasetView>(
    graph: &D,
    loaded: &[&str],
) -> Vec<(D::Id, D::Id)> {
    let Some(imports) = graph.term_id_by_value(&TermValue::iri(OWL_IMPORTS)) else {
        return Vec::new();
    };
    let anchors = anchors(graph, loaded);
    let mut seen: BTreeSet<(D::Id, D::Id)> = BTreeSet::new();
    graph
        .quads()
        .filter(|quad| quad.p == imports && anchors.binary_search(&quad.s).is_err())
        .map(|quad| (quad.s, quad.o))
        .filter(|pair| seen.insert(*pair))
        .collect()
}

/// One `owl:imports` triple that is not an import: its subject is no anchor of the document
/// it occurs in. See [`ImportMap::unanchored_imports`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnanchoredImport {
    /// The document the triple occurs in: `None` for the importing graph, or the IRI a
    /// supplied document was imported under.
    pub document: Option<String>,
    /// The triple's subject.
    pub subject: TermValue,
    /// The triple's object.
    pub object: TermValue,
}

/// The anchors of `graph` read under `loaded`, sorted and deduplicated: each loaded IRI the
/// graph interns, each import anchor of the one graph-role classifier (a SHACL instance of
/// `owl:Ontology` or of `sh:ShapesGraph`), and each subject naming one of those as its
/// `owl:versionIRI` (the one `^owl:versionIRI?` step).
fn anchors<D: DatasetView>(graph: &D, loaded: &[&str]) -> Vec<D::Id> {
    let term = |iri: &str| graph.term_id_by_value(&TermValue::iri(iri));
    let mut anchors: Vec<D::Id> = loaded.iter().filter_map(|iri| term(iri)).collect();
    anchors.extend(GraphRoleIndex::classify(graph).import_anchors());
    anchors.sort_unstable();
    anchors.dedup();
    if let Some(version_iri) = term(OWL_VERSIONIRI) {
        let versioned: Vec<D::Id> = graph
            .quads_for_pattern(None, Some(version_iri), None, GraphMatch::Any)
            .filter(|quad| anchors.binary_search(&quad.o).is_ok())
            .map(|quad| quad.s)
            .collect();
        anchors.extend(versioned);
        anchors.sort_unstable();
        anchors.dedup();
    }
    anchors
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

/// The highest blank-node scope a term mentions, through triple terms, found over
/// [`TermValue::visit_terms`]'s work list.
fn scope_of(term: &TermValue) -> u32 {
    let mut highest = 0;
    let ControlFlow::Continue(()) = term.visit_terms(|term| -> ControlFlow<Infallible> {
        if let TermValue::Blank { scope, .. } = term {
            highest = highest.max(scope.ordinal());
        }
        ControlFlow::Continue(())
    });
    highest
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
///
/// A triple term is interned over [`try_fold_nested`]'s work list: its subject, predicate
/// and object, each fully before the next, then the triple itself.
fn intern_scoped(
    b: &mut RdfDatasetBuilder,
    value: &TermValue,
    rescope: &mut ScopeMap<'_>,
) -> TermId {
    let interned = try_fold_nested(
        value,
        &mut (b, rescope),
        |(b, rescope), value| {
            Ok::<_, Infallible>(match value {
                TermValue::Triple { s, p, o } => Nested::Triple(&**s, &**p, &**o),
                leaf => Nested::Leaf(intern_leaf(b, leaf, rescope)),
            })
        },
        |(b, _), _, s, p, o| Ok(b.intern_triple(s, p, o)),
    );
    match interned {
        Ok(id) => id,
    }
}

/// Intern a term that is not a triple term, rewriting a blank node's scope through
/// `rescope`.
fn intern_leaf(b: &mut RdfDatasetBuilder, value: &TermValue, rescope: &mut ScopeMap<'_>) -> TermId {
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
        TermValue::Triple { .. } => unreachable!("a triple term is folded, not interned whole"),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{
        ImportMap, OWL_IMPORTS, OWL_VERSIONIRI, UnanchoredImport, imported_iris, unresolved_imports,
    };
    use crate::graph_roles::{
        OWL_ONTOLOGY, RDF_TYPE, RDFS_SUB_CLASS_OF, SH_DATA_GRAPH, SH_RULES_GRAPH, SH_SHAPES_GRAPH,
    };
    use crate::ir::{BlankScope, RdfDataset, RdfDatasetBuilder, TermValue};

    /// The empty answer, spelled once for `assert_eq!`.
    const NONE: [String; 0] = [];
    /// No unanchored `owl:imports`, spelled once for `assert_eq!`.
    const NO_UNANCHORED: [UnanchoredImport; 0] = [];

    /// One case of a shapes-graph class: the class, and the axioms that make it one.
    type ClassCase<'a> = (&'a str, &'a [(&'a str, &'a str, &'a str)]);

    /// The IRI the importing graph in these tests is read under or declares as its header.
    const SHAPES: &str = "http://example.org/shapes";
    /// A node of the importing graph that is neither loaded nor an ontology header.
    const OTHER_NODE: &str = "http://example.org/other-node";
    /// An imported ontology.
    const LIB: &str = "http://example.org/lib";

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

    /// A one-triple document `_:label ex:p <object>`, plus an ontology header
    /// `ex:self a owl:Ontology` importing each target.
    fn document(label: &str, object: &str, imports: &[&str]) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_blank(label, BlankScope::DEFAULT);
        let p = b.intern_iri("http://example.org/p");
        let o = b.intern_iri(object);
        b.push_quad(s, p, o, None);
        let header = b.intern_iri("http://example.org/self");
        let rdf_type = b.intern_iri(RDF_TYPE);
        let ontology = b.intern_iri(OWL_ONTOLOGY);
        b.push_quad(header, rdf_type, ontology, None);
        for target in imports {
            let predicate = b.intern_iri(OWL_IMPORTS);
            let target = b.intern_iri(target);
            b.push_quad(header, predicate, target, None);
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

    /// Whether `ds` holds the all-IRI triple `(s, p, o)`.
    fn holds(ds: &RdfDataset, s: &str, p: &str, o: &str) -> bool {
        ds.quads().any(|quad| {
            ds.term_value(quad.s) == TermValue::iri(s)
                && ds.term_value(quad.p) == TermValue::iri(p)
                && ds.term_value(quad.o) == TermValue::iri(o)
        })
    }

    /// The four ways an import is in hand, each beside the neighbour that differs only in
    /// the one fact the rule reads, so every "resolved" answer is observed against a
    /// "missing" one. The importer is an ontology header, so its `owl:imports` is an import.
    #[test]
    fn each_resolution_route_resolves_and_its_neighbour_does_not() {
        const OTHER: &str = "http://example.org/other";
        let importer = |extra: &[(&str, &str, &str)]| {
            let mut rows = vec![(SHAPES, RDF_TYPE, OWL_ONTOLOGY), (SHAPES, OWL_IMPORTS, LIB)];
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

    /// An `owl:imports` on an IRI the graph was LOADED under is an import: unsupplied, it is
    /// refused by name. The neighbour writes the same triple on a node that is neither loaded
    /// nor an ontology header: it imports nothing, nothing is refused, and the triple is
    /// still in the graph as data.
    #[test]
    fn an_import_on_a_loaded_iri_counts_and_one_on_another_node_is_data() {
        let loaded = triples(&[(SHAPES, OWL_IMPORTS, LIB)]);
        let mut map = ImportMap::new();
        map.declare_loaded(SHAPES);
        assert_eq!(imported_iris(loaded.as_ref(), &[SHAPES]), [LIB]);
        assert_eq!(map.closure(&loaded).unresolved(), [LIB.to_owned()]);

        let data = triples(&[(OTHER_NODE, OWL_IMPORTS, LIB)]);
        assert_eq!(imported_iris(data.as_ref(), &[SHAPES]), NONE);
        let closure = map.closure(&data);
        assert_eq!(closure.unresolved(), NONE);
        assert!(closure.merge(&data).expect("freeze").is_none());
        assert!(
            holds(&data, OTHER_NODE, OWL_IMPORTS, LIB),
            "the triple stays in the graph as data"
        );

        // A graph read under no IRI at all: the loaded anchor is absent, so the very triple
        // that was an import above is data here.
        assert_eq!(ImportMap::new().closure(&loaded).unresolved(), NONE);
    }

    /// An `owl:Ontology` header's import counts: unsupplied it is refused, supplied it is
    /// merged — the supplied document's own triple is observed in the result, and a
    /// non-anchor `owl:imports` beside the header travels as data without being looked for.
    #[test]
    fn an_ontology_header_imports_and_a_supplied_document_is_merged() {
        const DATA_TARGET: &str = "http://example.org/not-a-document";
        let graph = triples(&[
            (SHAPES, RDF_TYPE, OWL_ONTOLOGY),
            (SHAPES, OWL_IMPORTS, LIB),
            (OTHER_NODE, OWL_IMPORTS, DATA_TARGET),
        ]);
        assert_eq!(imported_iris(graph.as_ref(), &[]), [LIB]);
        assert_eq!(
            ImportMap::new().closure(&graph).unresolved(),
            [LIB.to_owned()],
            "only the header's import is named; the other triple is data"
        );

        let mut map = ImportMap::new();
        map.insert(
            LIB,
            triples(&[(LIB, "http://example.org/said", "http://example.org/v")]),
        );
        let closure = map.closure(&graph);
        assert_eq!(closure.unresolved(), NONE);
        let merged = closure
            .merge(&graph)
            .expect("freeze")
            .expect("one document was reached");
        assert!(holds(
            &merged,
            LIB,
            "http://example.org/said",
            "http://example.org/v"
        ));
        assert!(holds(&merged, OTHER_NODE, OWL_IMPORTS, DATA_TARGET));
    }

    /// An anonymous ontology — OWL 2's `_:x rdf:type owl:Ontology . _:x owl:imports *:z` —
    /// imports too. The neighbour's blank node is not typed `owl:Ontology` and imports
    /// nothing.
    #[test]
    fn a_blank_node_header_imports_and_an_untyped_blank_node_does_not() {
        let with_header = |typed: bool| {
            let mut b = RdfDatasetBuilder::new();
            let header = b.intern_blank("h", BlankScope::DEFAULT);
            let imports = b.intern_iri(OWL_IMPORTS);
            let lib = b.intern_iri(LIB);
            b.push_quad(header, imports, lib, None);
            let rdf_type = b.intern_iri(RDF_TYPE);
            let class = b.intern_iri(if typed {
                OWL_ONTOLOGY
            } else {
                "http://example.org/Thing"
            });
            b.push_quad(header, rdf_type, class, None);
            b.freeze().expect("freeze")
        };
        assert_eq!(
            ImportMap::new().closure(&with_header(true)).unresolved(),
            [LIB.to_owned()]
        );
        assert_eq!(
            ImportMap::new().closure(&with_header(false)).unresolved(),
            NONE
        );
    }

    /// The `^owl:versionIRI?` step: a subject naming the loaded IRI as its version IRI is an
    /// anchor, so its import counts. The neighbour names a different version IRI and its
    /// `owl:imports` is data.
    #[test]
    fn a_subject_versioning_an_anchor_imports_and_one_versioning_another_iri_does_not() {
        const SERIES: &str = "http://example.org/series";
        let graph = |version: &str| {
            triples(&[
                (SERIES, OWL_VERSIONIRI, version),
                (SERIES, OWL_IMPORTS, LIB),
            ])
        };
        let mut map = ImportMap::new();
        map.declare_loaded(SHAPES);
        assert_eq!(map.closure(&graph(SHAPES)).unresolved(), [LIB.to_owned()]);
        assert_eq!(
            map.closure(&graph("http://example.org/elsewhere"))
                .unresolved(),
            NONE
        );
    }

    /// A supplied document's own imports are read from ITS anchors: the IRI it was imported
    /// by, its own header, and a subject versioning its import IRI. The neighbour document
    /// writes the same `owl:imports` on a node that is none of those, and the walk stops.
    #[test]
    fn an_imported_documents_imports_are_read_from_its_own_anchors() {
        const DEEP: &str = "http://example.org/deep";
        let root = triples(&[(SHAPES, RDF_TYPE, OWL_ONTOLOGY), (SHAPES, OWL_IMPORTS, LIB)]);
        let walk = |lib: Arc<RdfDataset>| {
            let mut map = ImportMap::new();
            map.insert(LIB, lib);
            map.closure(&root).unresolved().to_vec()
        };
        // By the IRI it was imported by.
        assert_eq!(
            walk(triples(&[(LIB, OWL_IMPORTS, DEEP)])),
            [DEEP.to_owned()]
        );
        // By its own ontology header.
        assert_eq!(
            walk(triples(&[
                ("http://example.org/lib-header", RDF_TYPE, OWL_ONTOLOGY),
                ("http://example.org/lib-header", OWL_IMPORTS, DEEP),
            ])),
            [DEEP.to_owned()]
        );
        // By a subject whose version IRI is the import IRI.
        assert_eq!(
            walk(triples(&[
                ("http://example.org/lib-series", OWL_VERSIONIRI, LIB),
                ("http://example.org/lib-series", OWL_IMPORTS, DEEP),
            ])),
            [DEEP.to_owned()]
        );
        // The neighbour: a node that is none of them. The importing graph's loaded IRI is
        // not an anchor of the imported document either.
        assert_eq!(walk(triples(&[(OTHER_NODE, OWL_IMPORTS, DEEP)])), NONE);
        assert_eq!(walk(triples(&[(SHAPES, OWL_IMPORTS, DEEP)])), NONE);
    }

    /// A supplied document no import reaches is reported; the neighbour whose graph DOES
    /// import it reports nothing unreached. A non-anchor `owl:imports` of it does not reach
    /// it.
    #[test]
    fn an_entry_nothing_imports_is_unreached() {
        let mut map = ImportMap::new();
        map.insert(LIB, triples(&[]));

        let imports_nothing = triples(&[(SHAPES, RDF_TYPE, OWL_ONTOLOGY)]);
        assert_eq!(map.closure(&imports_nothing).unreached(), [LIB.to_owned()]);

        let data_only = triples(&[(OTHER_NODE, OWL_IMPORTS, LIB)]);
        assert_eq!(map.closure(&data_only).unreached(), [LIB.to_owned()]);

        let imports_lib = triples(&[(SHAPES, RDF_TYPE, OWL_ONTOLOGY), (SHAPES, OWL_IMPORTS, LIB)]);
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

    /// A SHACL instance of `sh:ShapesGraph` is an anchor however it gets there: typed
    /// directly, typed `sh:RulesGraph` with no subclass axiom in the document, or typed a
    /// user class the document declares `rdfs:subClassOf*` `sh:ShapesGraph`. Each import is
    /// refused unsupplied and merged supplied; the control row — the same triple on a node
    /// typed an unrelated class — is data in every case.
    #[test]
    fn every_shapes_graph_instance_imports_and_an_unrelated_type_does_not() {
        const NODE: &str = "http://example.org/g";
        const MODULE: &str = "http://example.org/Module";
        const THING: &str = "http://example.org/Thing";
        let cases: [ClassCase<'_>; 4] = [
            (SH_SHAPES_GRAPH, &[]),
            (SH_RULES_GRAPH, &[]),
            (MODULE, &[(MODULE, RDFS_SUB_CLASS_OF, SH_SHAPES_GRAPH)]),
            (
                "http://example.org/SubModule",
                &[
                    (MODULE, RDFS_SUB_CLASS_OF, SH_RULES_GRAPH),
                    ("http://example.org/SubModule", RDFS_SUB_CLASS_OF, MODULE),
                ],
            ),
        ];
        for (class, axioms) in cases {
            let mut rows = vec![(NODE, RDF_TYPE, class), (NODE, OWL_IMPORTS, LIB)];
            rows.extend_from_slice(axioms);
            let graph = triples(&rows);
            assert_eq!(imported_iris(graph.as_ref(), &[]), [LIB], "{class}");
            assert_eq!(
                ImportMap::new().closure(&graph).unresolved(),
                [LIB.to_owned()],
                "{class}: refused unsupplied"
            );
            let mut map = ImportMap::new();
            map.insert(
                LIB,
                triples(&[(LIB, "http://example.org/said", "http://example.org/v")]),
            );
            let closure = map.closure(&graph);
            assert_eq!(closure.unresolved(), NONE, "{class}");
            let merged = closure
                .merge(&graph)
                .expect("freeze")
                .expect("the import was reached");
            assert!(
                holds(
                    &merged,
                    LIB,
                    "http://example.org/said",
                    "http://example.org/v"
                ),
                "{class}: merged supplied"
            );
            assert_eq!(ImportMap::new().unanchored_imports(&graph), NO_UNANCHORED);
        }
        let control = triples(&[(NODE, RDF_TYPE, THING), (NODE, OWL_IMPORTS, LIB)]);
        assert_eq!(imported_iris(control.as_ref(), &[]), NONE);
        assert_eq!(ImportMap::new().closure(&control).unresolved(), NONE);
        let listed = ImportMap::new().unanchored_imports(&control);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].subject, TermValue::iri(NODE));
        assert_eq!(listed[0].object, TermValue::iri(LIB));
    }

    /// Two shapes graphs declared in one document are both anchors: both imports are named,
    /// in quad order. Anchoring only the first would silently drop the second's library.
    #[test]
    fn two_shapes_graphs_in_one_document_are_both_anchored() {
        const DEEP: &str = "http://example.org/deep";
        let graph = triples(&[
            ("http://example.org/a", RDF_TYPE, SH_SHAPES_GRAPH),
            ("http://example.org/a", OWL_IMPORTS, LIB),
            ("http://example.org/b", RDF_TYPE, SH_SHAPES_GRAPH),
            ("http://example.org/b", OWL_IMPORTS, DEEP),
        ]);
        assert_eq!(imported_iris(graph.as_ref(), &[]), [LIB, DEEP]);
        assert_eq!(
            ImportMap::new().closure(&graph).unresolved(),
            [LIB.to_owned(), DEEP.to_owned()]
        );
    }

    /// A node whose only graph role is `sh:DataGraph` anchors nothing (SHACL 1.2 Core §6.2,
    /// "owl:imports in the data graph is not enacted"): its import is data, listed as
    /// unanchored, and never refused. The neighbour adds `owl:Ontology` — the type §6.2's note
    /// asks for — and the same triple is an import.
    #[test]
    fn a_data_graph_alone_anchors_nothing_and_a_data_graph_ontology_does() {
        const NODE: &str = "http://example.org/data";
        let data_only = triples(&[(NODE, RDF_TYPE, SH_DATA_GRAPH), (NODE, OWL_IMPORTS, LIB)]);
        assert_eq!(imported_iris(data_only.as_ref(), &[]), NONE);
        assert_eq!(ImportMap::new().closure(&data_only).unresolved(), NONE);
        let listed = ImportMap::new().unanchored_imports(&data_only);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].subject, TermValue::iri(NODE));

        let with_header = triples(&[
            (NODE, RDF_TYPE, SH_DATA_GRAPH),
            (NODE, RDF_TYPE, OWL_ONTOLOGY),
            (NODE, OWL_IMPORTS, LIB),
        ]);
        assert_eq!(
            ImportMap::new().closure(&with_header).unresolved(),
            [LIB.to_owned()]
        );
        assert_eq!(
            ImportMap::new().unanchored_imports(&with_header),
            NO_UNANCHORED
        );

        // A document LOADED under the node's IRI is in the loading role whatever it types
        // itself: the loaded IRI stays an anchor.
        let mut map = ImportMap::new();
        map.declare_loaded(NODE);
        assert_eq!(map.closure(&data_only).unresolved(), [LIB.to_owned()]);
    }

    /// A shapes graph the closure itself declares is in hand, like an ontology header; a
    /// node declared only a data graph is not, so an import of it is still refused.
    #[test]
    fn a_declared_shapes_graph_resolves_in_place_and_a_declared_data_graph_does_not() {
        let importer = |class: &str| {
            triples(&[
                (SHAPES, RDF_TYPE, OWL_ONTOLOGY),
                (SHAPES, OWL_IMPORTS, LIB),
                (LIB, RDF_TYPE, class),
            ])
        };
        assert_eq!(
            ImportMap::new()
                .closure(&importer(SH_SHAPES_GRAPH))
                .unresolved(),
            NONE
        );
        assert_eq!(
            ImportMap::new()
                .closure(&importer(SH_DATA_GRAPH))
                .unresolved(),
            [LIB.to_owned()]
        );
    }

    /// A supplied document's unanchored `owl:imports` are listed under the IRI it was
    /// imported by, after the importing graph's own; its anchored ones are not listed.
    #[test]
    fn unanchored_imports_are_listed_per_document() {
        let root = triples(&[
            (SHAPES, RDF_TYPE, SH_SHAPES_GRAPH),
            (SHAPES, OWL_IMPORTS, LIB),
            (OTHER_NODE, OWL_IMPORTS, "http://example.org/root-data"),
        ]);
        let mut map = ImportMap::new();
        map.insert(
            LIB,
            triples(&[
                (LIB, OWL_IMPORTS, SHAPES),
                (OTHER_NODE, OWL_IMPORTS, "http://example.org/lib-data"),
            ]),
        );
        let listed: Vec<(Option<String>, TermValue)> = map
            .unanchored_imports(&root)
            .into_iter()
            .map(|entry| (entry.document, entry.object))
            .collect();
        assert_eq!(
            listed,
            [
                (None, TermValue::iri("http://example.org/root-data")),
                (
                    Some(LIB.to_owned()),
                    TermValue::iri("http://example.org/lib-data")
                ),
            ]
        );
    }

    /// No reached document, no copy.
    #[test]
    fn a_closure_that_reached_nothing_is_not_copied() {
        let graph = triples(&[
            (SHAPES, RDF_TYPE, OWL_ONTOLOGY),
            (SHAPES, OWL_IMPORTS, LIB),
            (LIB, RDF_TYPE, OWL_ONTOLOGY),
        ]);
        let closure = ImportMap::new().closure(&graph);
        assert_eq!(closure.unresolved(), NONE);
        assert!(closure.merge(&graph).expect("freeze").is_none());
        assert_eq!(imported_iris(graph.as_ref(), &[]), vec![LIB.to_owned()]);
    }

    /// The version constraints of OWL 2 §3.4 over a closure of three graphs: the importing
    /// graph imports `lib/1` and a second document. Each treatment row is beside the
    /// neighbour that differs only in the one fact the rule reads, and the neighbour's
    /// closure is complete and conflict-free.
    #[test]
    fn two_versions_of_one_series_conflict_and_two_series_do_not() {
        use super::{VersionConflict, VersionConflictKind};
        const LIB_1: &str = "http://example.org/lib/1";
        const LIB_2: &str = "http://example.org/lib/2";
        const OLD: &str = "http://example.org/old";
        use purrdf_iri::vocab::owl::INCOMPATIBLE_WITH as OWL_INCOMPATIBLE_WITH;
        let importer = |second: &str| {
            triples(&[
                (SHAPES, RDF_TYPE, OWL_ONTOLOGY),
                (SHAPES, OWL_IMPORTS, LIB_1),
                (SHAPES, OWL_IMPORTS, second),
            ])
        };
        let first = triples(&[(LIB, OWL_VERSIONIRI, LIB_1)]);
        let conflicts = |second: &str, rows: &[(&str, &str, &str)]| {
            let mut map = ImportMap::new();
            map.insert(LIB_1, Arc::clone(&first));
            map.insert(second, triples(rows));
            let closure = map.closure(&importer(second));
            assert_eq!(closure.unresolved(), NONE, "the closure is complete");
            assert_eq!(closure.documents().len(), 2, "both documents are reached");
            closure.conflicts().to_vec()
        };

        // Same series, another version: a conflict naming both documents.
        assert_eq!(
            conflicts(LIB_2, &[(LIB, OWL_VERSIONIRI, LIB_2)]),
            [VersionConflict {
                first: Some(LIB_1.to_owned()),
                second: Some(LIB_2.to_owned()),
                kind: VersionConflictKind::SameSeries {
                    series: LIB.to_owned(),
                    first_version: LIB_1.to_owned(),
                    second_version: LIB_2.to_owned(),
                },
            }]
        );
        // Neighbour: another series with its own version.
        assert_eq!(
            conflicts(
                LIB_2,
                &[("http://example.org/lib-two", OWL_VERSIONIRI, LIB_2)]
            ),
            []
        );

        // One declares owl:incompatibleWith the other's version IRI, or its series IRI.
        for named in [LIB_1, LIB] {
            assert_eq!(
                conflicts(
                    OLD,
                    &[
                        (OLD, RDF_TYPE, OWL_ONTOLOGY),
                        (OLD, OWL_INCOMPATIBLE_WITH, named)
                    ]
                ),
                [VersionConflict {
                    first: Some(OLD.to_owned()),
                    second: Some(LIB_1.to_owned()),
                    kind: VersionConflictKind::IncompatibleWith {
                        iri: named.to_owned()
                    },
                }],
                "{named}"
            );
        }
        // Neighbour: incompatible with a graph the closure does not hold.
        assert_eq!(
            conflicts(
                OLD,
                &[
                    (OLD, RDF_TYPE, OWL_ONTOLOGY),
                    (OLD, OWL_INCOMPATIBLE_WITH, "http://example.org/elsewhere")
                ]
            ),
            []
        );
        // Neighbour: the declaration sits on a node that is no anchor, so it is data.
        assert_eq!(
            conflicts(
                OLD,
                &[
                    (OLD, RDF_TYPE, OWL_ONTOLOGY),
                    (OTHER_NODE, OWL_INCOMPATIBLE_WITH, LIB_1)
                ]
            ),
            []
        );
    }

    /// The importing graph is a graph of its closure too, named by the IRI it was loaded
    /// under, and a closure that reached no document holds one graph and no conflict.
    #[test]
    fn the_importing_graph_conflicts_with_a_version_it_imports() {
        use super::{VersionConflict, VersionConflictKind};
        const V1: &str = "http://example.org/shapes/1";
        const V2: &str = "http://example.org/shapes/2";
        let graph = triples(&[
            (SHAPES, RDF_TYPE, OWL_ONTOLOGY),
            (SHAPES, OWL_VERSIONIRI, V2),
            (SHAPES, OWL_IMPORTS, V1),
        ]);
        let mut map = ImportMap::new();
        map.insert(V1, triples(&[(SHAPES, OWL_VERSIONIRI, V1)]));
        let closure = map.closure(&graph);
        assert_eq!(
            closure.conflicts(),
            [VersionConflict {
                first: None,
                second: Some(V1.to_owned()),
                kind: VersionConflictKind::SameSeries {
                    series: SHAPES.to_owned(),
                    first_version: V2.to_owned(),
                    second_version: V1.to_owned(),
                },
            }]
        );
        assert_eq!(
            closure.conflicts()[0].to_string(),
            "the importing graph and <http://example.org/shapes/1> are different versions of \
             the series <http://example.org/shapes> (owl:versionIRI \
             <http://example.org/shapes/2> and <http://example.org/shapes/1>)"
        );
        // Neighbour: the imported document is the same version.
        let mut same = ImportMap::new();
        same.insert(V1, triples(&[(SHAPES, OWL_VERSIONIRI, V2)]));
        assert_eq!(same.closure(&graph).conflicts(), []);
    }
}

#[cfg(test)]
mod term_walk_tests {
    //! The scope survey and the rescoping re-intern against their recursive references,
    //! and at a hundred thousand levels on a 128 KiB thread.

    use std::collections::BTreeMap;

    use super::{ScopeMap, intern_scoped, scope_of};
    use crate::ir::{BlankScope, RdfDatasetBuilder, TermBox, TermId, TermValue};
    use crate::model::RdfLiteral;

    fn reference_scope(term: &TermValue) -> u32 {
        match term {
            TermValue::Blank { scope, .. } => scope.ordinal(),
            TermValue::Triple { s, p, o } => reference_scope(s)
                .max(reference_scope(p))
                .max(reference_scope(o)),
            TermValue::Iri(_) | TermValue::Literal { .. } => 0,
        }
    }

    fn reference_intern(
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
                let s = reference_intern(b, s, rescope);
                let p = reference_intern(b, p, rescope);
                let o = reference_intern(b, o, rescope);
                b.intern_triple(s, p, o)
            }
        }
    }

    /// The fresh-scope assignments a map made, for comparison.
    fn assigned(map: &ScopeMap<'_>) -> Option<BTreeMap<u32, BlankScope>> {
        match map {
            ScopeMap::Identity => None,
            ScopeMap::Fresh { assigned, .. } => Some(assigned.clone()),
        }
    }

    /// Both walks answer every generated term as their recursive references do — the
    /// same highest scope, the same interned id, the same scopes assigned — under both
    /// the identity and a fresh renumbering.
    #[test]
    fn the_walks_agree_with_their_recursive_references_on_generated_terms() {
        for seed in 0..400_u64 {
            let mut state = seed;
            let mut budget = 8;
            let value = crate::term_fixture::term_value(
                &mut state,
                purrdf_testkit::rng::splitmix64_next,
                &mut budget,
                crate::term_fixture::TermShape::Any,
            );
            assert_eq!(scope_of(&value), reference_scope(&value), "seed {seed}");
            let (mut found, mut expected) = (RdfDatasetBuilder::new(), RdfDatasetBuilder::new());
            assert_eq!(
                intern_scoped(&mut found, &value, &mut ScopeMap::Identity),
                reference_intern(&mut expected, &value, &mut ScopeMap::Identity),
                "seed {seed}"
            );
            let (mut next, mut expected_next) = (7, 7);
            let mut map = ScopeMap::Fresh {
                assigned: BTreeMap::new(),
                next: &mut next,
            };
            let mut expected_map = ScopeMap::Fresh {
                assigned: BTreeMap::new(),
                next: &mut expected_next,
            };
            let (mut found, mut expected) = (RdfDatasetBuilder::new(), RdfDatasetBuilder::new());
            assert_eq!(
                intern_scoped(&mut found, &value, &mut map),
                reference_intern(&mut expected, &value, &mut expected_map),
                "seed {seed}"
            );
            assert_eq!(assigned(&map), assigned(&expected_map), "seed {seed}");
        }
    }

    /// A triple term a hundred thousand levels deep, its innermost object a scoped blank
    /// node, is surveyed and re-interned on a thread whose whole stack is 128 KiB.
    #[test]
    fn a_hundred_thousand_level_term_is_rescoped_on_a_128_kib_thread() {
        const LEVELS: usize = 100_000;
        purrdf_stack::on_stack(128 * 1024, || {
            let mut value = TermValue::Blank {
                label: "b".to_owned(),
                scope: BlankScope(3),
            };
            for _ in 0..LEVELS {
                value = TermValue::Triple {
                    s: TermBox::new(TermValue::iri("http://example.org/s")),
                    p: TermBox::new(TermValue::iri("http://example.org/p")),
                    o: TermBox::new(value),
                };
            }
            assert_eq!(scope_of(&value), 3);
            let mut next = 10;
            let mut map = ScopeMap::Fresh {
                assigned: BTreeMap::new(),
                next: &mut next,
            };
            let mut builder = RdfDatasetBuilder::new();
            assert_eq!(
                intern_scoped(&mut builder, &value, &mut map).index(),
                LEVELS + 2
            );
            assert_eq!(assigned(&map).map(|assigned| assigned.len()), Some(1));
        })
        .expect("the thread starts");
    }
}
