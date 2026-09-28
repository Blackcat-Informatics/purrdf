// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A shapes graph's `owl:imports` closure: the table a caller supplies documents through,
//! the one helper every shapes-graph entry point resolves it with, and the typed refusal.
//!
//! # Why a shapes graph's imports are resolved or refused
//!
//! SHACL reads a shapes graph that `owl:imports` another document as the union of the two:
//! the shapes the imported document declares constrain the data exactly as if they had been
//! written in the importing one. A validator that goes on without an imported document is
//! therefore not validating a slightly smaller shapes graph — it is validating a DIFFERENT
//! one, and a shapes graph whose shapes all live in an imported document "conforms" against
//! no shapes at all. A warning beside that verdict does not undo it, so there is no
//! warn-and-continue path: an import nothing in hand resolves is a
//! [`ShapesImportError::Unresolved`], naming every such IRI at once.
//!
//! # Which `owl:imports` is an import
//!
//! SHACL 1.2 Core, on the shapes graph:
//!
//! > As a pre-validation step, SHACL processors should extend the originally provided shapes
//! > graph by transitively following and importing all referenced shapes graphs through the
//! > owl:imports predicate. When resolving an imported IRI, if the retrieved graph contains a
//! > triple with the imported IRI as the object of owl:versionIRI, the processor should treat
//! > the subject of that triple as the shapes graph IRI of the imported graph for the purpose
//! > of following further owl:imports statements. Formally, processors should use the
//! > property path ^owl:versionIRI?/owl:imports iteratively
//!
//! and names the class that declares one: "The sh:ShapesGraph class MAY be used as an
//! rdf:type of the IRI of a graph that typically acts in the role of a shapes graph." OWL 2's
//! mapping to RDF (§3.1.1, §3.1.2, Table 4) reads the imports of a document off its ontology
//! header, `x rdf:type owl:Ontology . x owl:imports *:y`. PurRDF treats the SHOULD as a MUST
//! and OWL 2 as the floor. So an `owl:imports` triple of a shapes graph is an import only when
//! its subject is an ANCHOR of its document:
//!
//! * the shapes graph's own IRI — the IRI it was read or parsed under
//!   ([`ShapesImports::declare_loaded`], the `loaded` argument of [`resolve_shapes_imports`]),
//!   or for an imported document the IRI it was imported by;
//! * a SHACL instance of `owl:Ontology` (IRI or blank node);
//! * EVERY SHACL instance of `sh:ShapesGraph` — `sh:RulesGraph`, and a class the document
//!   declares `rdfs:subClassOf*` `sh:ShapesGraph`, included;
//! * a node naming one of those as its `owl:versionIRI`.
//!
//! A node whose only graph role is `sh:DataGraph` is not an anchor — SHACL 1.2 Core §6.2:
//! "owl:imports in the data graph is not enacted" — and the DATA graph's imports are never
//! read at all: validation resolves the shapes graph's closure and nothing else. Any other
//! `owl:imports` triple is data: it stays in the shapes graph as written, and no document is
//! looked for. The W3C test `sparql/component/validator-001` is that case — its
//! `owl:imports <http://datashapes.org/dash>` sits on a node that is none of these, and its
//! expected report is computed without DASH. `shapes lint` lists every such triple in its
//! `unanchored-imports` section ([`crate::lint`]).
//!
//! SHACL-SPARQL's prefix path `sh:prefixes/owl:imports*/sh:declare` is not an import either.
//! It walks `owl:imports` edges between prefix-declaring nodes WITHIN the shapes graph (the
//! W3C test `sparql/node/prefixes-001` writes `ex:TestPrefixes owl:imports
//! <http://example.com/ns#>` for exactly that), and it is answered by the SPARQL prefix
//! collection over the graph as it stands, never by fetching a document.
//!
//! # One rule, every host
//!
//! The rule, and when an import counts as in hand, is [`purrdf_core::imports`] — the same
//! rule entailment applies, stated once in the kernel both engines sit on — and its anchors
//! come from the kernel's one graph-role classifier, [`purrdf_core::graph_roles`], the same
//! answer SHACL-SPARQL's implicit prefixes select from. This module adds
//! only what a SHACL shapes graph needs beyond it: each supplied document's own `@prefix`
//! map (a SHACL-SPARQL query in an imported document resolves prefixed names against ITS
//! declarations), and the refusal of a supplied document nothing imports.
//!
//! [`resolve_shapes_imports`] is the one place the two meet. Every constructor of a
//! [`Shapes`](crate::shapes::Shapes) — and so every validation, rules run, node-expression
//! evaluation, lint and prepared product built from one — resolves its shapes graph through
//! it before a single shape is read, so a `Shapes` value is complete by construction, and the
//! command line, Python, WebAssembly and C hosts all give the same verdict about the same
//! shapes graph. A host supplies documents through its own spelling of a
//! [`ShapesImports`] table; none of them fetches anything.
//!
//! # A data graph's `sh:shapesGraph` links are folded in the same way
//!
//! SHACL 1.2 Core §6.4:
//!
//! > A data graph can include triples used to suggest one or more graphs to a SHACL processor
//! > with the predicate sh:shapesGraph. Every value of sh:shapesGraph is an IRI representing a
//! > graph that SHOULD be included into the shapes graph used to validate the data graph. The
//! > value of sh:shapesGraph may be a value of owl:versionIRI, so the same strategy of
//! > resolving a shapes graph IRI from a version IRI, described for Shapes Graphs, applies
//! > here.
//!
//! PurRDF treats the SHOULD as a MUST. A `sh:shapesGraph` triple is a LINK when its subject
//! is an anchor of the DATA graph: an IRI the data graph was loaded under (its retrieval IRI
//! or base, when the host knows one), or a SHACL instance of `sh:DataGraph` by the kernel's
//! one classifier ([`purrdf_core::graph_roles`]). A `sh:shapesGraph` triple on any other
//! node is data. [`data_graph_links`] reads the links; [`ShapesImports::link_data_graph`]
//! puts them in the table, and [`resolve_shapes_imports`] resolves each one exactly as it
//! resolves an import — the same table, the same in-place declarations, the
//! `^owl:versionIRI` step, and the linked graph's own `owl:imports` closure — and unions the
//! linked graphs into the shapes graph. A link nothing resolves is
//! [`ShapesImportError::UnresolvedLink`]; a link value that is not an IRI is
//! [`ShapesImportError::InvalidLink`]; a table entry only a link names is reached.
//!
//! A shapes graph built before its data graph was known — a [`PreparedShapes`], a prepared
//! product, a `Shapes` validated against many data graphs — cannot take a link any more, so
//! validation checks each link against it instead ([`check_data_graph_links`]): a link the
//! shapes graph already holds (its loaded IRI, a graph its closure or its links folded in,
//! an anchor or version IRI it declares) validates; any other is
//! [`ShapesImportError::UnheldLink`], never a verdict about a smaller shapes graph than the
//! data graph asked for.
//!
//! The data graph's own `owl:imports` stay unenacted (Core §6.2): a link names a graph for
//! the SHAPES graph, and nothing about it makes the data graph's imports directives.
//!
//! [`PreparedShapes`]: crate::engine::PreparedShapes
//!
//! # A supplied document nothing imports is refused too
//!
//! [`ShapesImportError::Unreached`]: a caller who hands over a document believes its shapes
//! take part in the verdict. If no import in the closure names it — a misspelled IRI, an
//! import the author forgot to write — those shapes would be read and silently never
//! applied, which is the same silent omission from the other side.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use ::purrdf::RdfDataset;
use purrdf_core::dataset_view::{DatasetView, GraphMatch};
use purrdf_core::graph_roles::{GraphRoleIndex, GraphRoles};
use purrdf_core::imports::{ImportMap, UnanchoredImport, declared_import_targets};
pub use purrdf_core::imports::{VersionConflict, VersionConflictKind};
use purrdf_core::ir::{TermRef, TermValue};

/// `sh:shapesGraph`: on a data-graph anchor, a link to a graph the shapes graph includes.
pub const SH_SHAPES_GRAPH_LINK: &str = "http://www.w3.org/ns/shacl#shapesGraph";

/// The documents a shapes graph's `owl:imports` resolve to, and the IRIs the shapes graph
/// was read from.
///
/// PurRDF fetches nothing: a caller that has the imported documents hands them over here,
/// keyed by the ontology IRI the shapes graph imports them under. An empty table is the
/// ordinary value for a shapes graph that imports nothing, and it still enforces the rule —
/// a shapes graph that imports a document the table does not supply is refused rather than
/// validated without it.
///
/// ```
/// use purrdf_shapes::imports::{ShapesImportError, ShapesImports};
/// use purrdf_shapes::{ShapesError, engine};
///
/// let shapes = "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
///     <http://example.org/shapes> a owl:Ontology ; owl:imports <http://example.org/lib> .\n";
///
/// // Nothing supplies the imported document: refused, by name.
/// let Err(ShapesError::Imports(ShapesImportError::Unresolved { iris })) =
///     engine::parse_shapes(shapes, None)
/// else {
///     panic!("an unresolved import is refused");
/// };
/// assert_eq!(iris, ["http://example.org/lib"]);
///
/// // Supplied: the imported document's shapes are part of the shapes graph.
/// let mut imports = ShapesImports::new();
/// imports
///     .insert_turtle(
///         "http://example.org/lib",
///         "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
///          <http://example.org/lib#S> a sh:NodeShape .\n",
///     )
///     .expect("the document parses");
/// let parsed = engine::parse_shapes_with_config(shapes, None, None, &imports)
///     .expect("every import resolves");
/// assert_eq!(parsed.node_shapes.len(), 1);
/// ```
#[derive(Debug, Clone, Default)]
pub struct ShapesImports {
    /// The kernel's table: ontology IRI → document, plus the loaded IRIs.
    map: ImportMap,
    /// Each supplied document's own `@prefix` map, by the ontology IRI it was supplied
    /// under.
    prefixes: BTreeMap<String, Vec<(String, String)>>,
    /// The data graph's `sh:shapesGraph` links, in the order the data graph states them,
    /// each once ([`Self::link_data_graph`]).
    links: Vec<String>,
}

impl ShapesImports {
    /// A table that supplies no document and declares no loaded IRI.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a table from `(ontology IRI, Turtle document)` pairs — the spelling every
    /// host binding passes across its boundary. Each document is parsed with its ontology
    /// IRI as its base (see [`insert_turtle`](Self::insert_turtle)).
    ///
    /// # Errors
    ///
    /// The first entry [`insert_turtle`](Self::insert_turtle) refuses.
    pub fn from_turtle(pairs: &[(&str, &str)]) -> Result<Self, ShapesImportError> {
        let mut table = Self::new();
        for (iri, turtle) in pairs {
            table.insert_turtle(iri, turtle)?;
        }
        Ok(table)
    }

    /// Supply `dataset` as the document `iri` names, with the document's own `@prefix`
    /// map (empty for a syntax that has none).
    ///
    /// # Errors
    ///
    /// [`ShapesImportError::InvalidEntry`] when `iri` is not an absolute IRI — an
    /// `owl:imports` object is absolute once parsed, so a relative or empty key could never
    /// match one and would be configuration that silently never applies — or when `iri`
    /// already names a document, since keeping either would be a choice made on the
    /// caller's behalf.
    pub fn insert(
        &mut self,
        iri: &str,
        dataset: Arc<RdfDataset>,
        prefixes: Vec<(String, String)>,
    ) -> Result<(), ShapesImportError> {
        check_import_iri(iri)?;
        if self.map.get(iri).is_some() {
            return Err(ShapesImportError::InvalidEntry {
                iri: iri.to_owned(),
                reason: "the import table names this IRI twice, and one IRI names one \
                         document; keeping either would be a choice made for the caller"
                    .to_owned(),
            });
        }
        self.map.insert(iri, dataset);
        self.prefixes.insert(iri.to_owned(), prefixes);
        Ok(())
    }

    /// Parse `turtle` as the document `iri` names and supply it.
    ///
    /// The document parses with `iri` as its base — the per-document base an `owl:imports`
    /// names, not the importing document's — so its relative references resolve where its
    /// author wrote them. A document that declares its own `@base` is also declared LOADED
    /// under that IRI: an import of it names this document.
    ///
    /// # Errors
    ///
    /// [`ShapesImportError::InvalidEntry`] for an unusable `iri` (see
    /// [`insert`](Self::insert)) or a document that is not Turtle.
    pub fn insert_turtle(&mut self, iri: &str, turtle: &str) -> Result<(), ShapesImportError> {
        check_import_iri(iri)?;
        let document =
            crate::text_ingest::parse_turtle_document(turtle, Some(iri)).map_err(|errors| {
                ShapesImportError::InvalidEntry {
                    iri: iri.to_owned(),
                    reason: format!(
                        "the document does not parse as Turtle: {}",
                        errors.join("; ")
                    ),
                }
            })?;
        self.insert(iri, document.dataset, document.prefixes)?;
        if let Some(base) = document.base
            && base != iri
        {
            self.map.declare_loaded(base);
        }
        Ok(())
    }

    /// Declare that the shapes graph was itself read from the document at `iri` — its
    /// retrieval IRI, or the base it was parsed under. `iri` is then the shapes graph's own
    /// IRI: an `owl:imports` whose subject is `iri` is one of its imports, and an
    /// `owl:imports <iri>` names a document already in hand.
    pub fn declare_loaded(&mut self, iri: impl Into<String>) {
        self.map.declare_loaded(iri);
    }

    /// Fold the `sh:shapesGraph` links of the data graph `data` into this table: each graph
    /// the data graph links (SHACL 1.2 Core §6.4, see the [module documentation](self)) is
    /// then resolved and unioned into the shapes graph by every constructor this table
    /// reaches, exactly as an import of the shapes graph is.
    ///
    /// `loaded` names the IRIs the DATA graph was loaded under — its retrieval IRI or base —
    /// when the host knows one; a `sh:shapesGraph` on one of those is a link as well as on a
    /// `sh:DataGraph`. Pass an empty slice when there is none (an N-Triples string, a pack).
    /// Links accumulate, each IRI once, so a host validating a data graph and a change to it
    /// can call this for both.
    ///
    /// ```
    /// use purrdf_shapes::imports::ShapesImports;
    /// use purrdf_shapes::text_ingest::parse_ntriples_to_dataset;
    ///
    /// let data = parse_ntriples_to_dataset(
    ///     "<http://example.org/d> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
    ///      <http://www.w3.org/ns/shacl#DataGraph> .\n\
    ///      <http://example.org/d> <http://www.w3.org/ns/shacl#shapesGraph> \
    ///      <http://example.org/shapes> .\n",
    /// )
    /// .expect("data");
    /// let mut imports = ShapesImports::new();
    /// imports.link_data_graph(data.as_ref(), &[]).expect("every link is an IRI");
    /// assert_eq!(imports.links(), ["http://example.org/shapes"]);
    /// ```
    ///
    /// # Errors
    ///
    /// [`ShapesImportError::InvalidLink`] when a link's value is not an IRI.
    pub fn link_data_graph<D: DatasetView>(
        &mut self,
        data: &D,
        loaded: &[&str],
    ) -> Result<(), ShapesImportError> {
        for link in data_graph_links(data, loaded)? {
            if !self.links.contains(&link) {
                self.links.push(link);
            }
        }
        Ok(())
    }

    /// The data graph's `sh:shapesGraph` links this table carries, in the order they were
    /// linked ([`Self::link_data_graph`]).
    #[must_use]
    pub fn links(&self) -> &[String] {
        &self.links
    }

    /// The kernel import table this wraps.
    #[must_use]
    pub const fn import_map(&self) -> &ImportMap {
        &self.map
    }

    /// Whether the table supplies no document.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// Refuse an import-table key no `owl:imports` object could ever equal.
fn check_import_iri(iri: &str) -> Result<(), ShapesImportError> {
    match purrdf_iri::is_absolute(iri) {
        Ok(true) => Ok(()),
        Ok(false) => Err(ShapesImportError::InvalidEntry {
            iri: iri.to_owned(),
            reason: "the key is not an absolute IRI, so no owl:imports object — absolute once \
                     parsed — can ever equal it, and the document would never be used"
                .to_owned(),
        }),
        Err(error) => Err(ShapesImportError::InvalidEntry {
            iri: iri.to_owned(),
            reason: format!("the key is not an IRI: {error}"),
        }),
    }
}

/// Why a shapes graph's `owl:imports` closure could not be used.
///
/// The typed half of every shapes-graph entry point's refusal: a host branches on the
/// variant (or on [`kind`](Self::kind), the label that crosses a language boundary), never
/// on the message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShapesImportError {
    /// The closure imports ontologies that nothing in hand resolves — no table entry, no
    /// loaded document, no `owl:Ontology` or `owl:versionIRI` declaration of the IRI in the
    /// closure.
    /// Every such IRI, in the order the closure walk first met it.
    Unresolved {
        /// The unresolved ontology IRIs.
        iris: Vec<String>,
    },
    /// The table supplies documents no import in the closure names, in IRI order.
    Unreached {
        /// The unreached table keys.
        iris: Vec<String>,
        /// The unreached keys an `owl:imports` triple of the closure DOES name, on a subject
        /// that is no anchor of the document the triple occurs in — so the triple is data, not
        /// an import (see [`ImportMap::unanchored_imports`]). A subset of `iris`, in its
        /// order; empty when no unanchored triple names any of them.
        unanchored: Vec<String>,
    },
    /// The data graph links shapes graphs (SHACL 1.2 Core §6.4, `sh:shapesGraph`) that
    /// nothing in hand resolves — no table entry, no loaded document, no `owl:Ontology`,
    /// `sh:ShapesGraph` or `owl:versionIRI` declaration of the IRI in the closure. Every such
    /// IRI, in the order the data graph states them.
    UnresolvedLink {
        /// The unresolved linked-graph IRIs.
        iris: Vec<String>,
    },
    /// The data graph links shapes graphs that a shapes graph built BEFORE the data graph
    /// was known does not hold — a prepared shapes graph or product, which can no longer
    /// take a graph in. Every such IRI, in the order the data graph states them.
    UnheldLink {
        /// The linked-graph IRIs the shapes graph does not hold.
        iris: Vec<String>,
    },
    /// A data-graph anchor's `sh:shapesGraph` value is not an IRI ("Every value of
    /// sh:shapesGraph is an IRI", SHACL 1.2 Core §6.4), so it names no graph to include.
    InvalidLink {
        /// Each offending value, rendered as an N-Triples-style term.
        values: Vec<String>,
    },
    /// The closure holds two graphs that are different versions of one series, or of which
    /// one declares `owl:incompatibleWith` the other. SHACL 1.2 Core §1.3 makes such a
    /// shapes graph ill-formed, and its §6.1 says the import closure "SHOULD NOT contain two
    /// graphs that are different versions of the same series, or where one declares
    /// owl:incompatibleWith the other". Every conflicting pair, each naming both graphs
    /// (see [`VersionConflict`]).
    IncompatibleVersions {
        /// The conflicting pairs, in closure walk order.
        conflicts: Vec<VersionConflict>,
    },
    /// A table entry that cannot be used: a key that is not an absolute IRI, a key named
    /// twice, or a document that does not parse.
    InvalidEntry {
        /// The entry's key, as the caller wrote it.
        iri: String,
        /// What is wrong with it.
        reason: String,
    },
}

impl ShapesImportError {
    /// The stable kebab-case label of the variant: `unresolved-import`,
    /// `unreached-import`, `incompatible-import-versions`, `invalid-import`,
    /// `unresolved-shapes-graph-link`, `unheld-shapes-graph-link` or
    /// `invalid-shapes-graph-link`. It is what the Python, JavaScript and C hosts carry in
    /// their own typed slot.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Unresolved { .. } => "unresolved-import",
            Self::Unreached { .. } => "unreached-import",
            Self::IncompatibleVersions { .. } => "incompatible-import-versions",
            Self::InvalidEntry { .. } => "invalid-import",
            Self::UnresolvedLink { .. } => "unresolved-shapes-graph-link",
            Self::UnheldLink { .. } => "unheld-shapes-graph-link",
            Self::InvalidLink { .. } => "invalid-shapes-graph-link",
        }
    }

    /// The IRIs the refusal names — for [`Self::InvalidLink`], the offending values; for
    /// [`Self::IncompatibleVersions`], each conflicting supplied document once, in the order
    /// the conflicts first name it (the shapes graph itself has no table IRI).
    #[must_use]
    pub fn iris(&self) -> Vec<&str> {
        match self {
            Self::IncompatibleVersions { conflicts } => {
                let mut named: Vec<&str> = Vec::new();
                for conflict in conflicts {
                    for graph in [&conflict.first, &conflict.second].into_iter().flatten() {
                        if !named.contains(&graph.as_str()) {
                            named.push(graph.as_str());
                        }
                    }
                }
                named
            }
            Self::Unresolved { iris }
            | Self::Unreached { iris, .. }
            | Self::UnresolvedLink { iris }
            | Self::UnheldLink { iris } => iris.iter().map(String::as_str).collect(),
            Self::InvalidLink { values } => values.iter().map(String::as_str).collect(),
            Self::InvalidEntry { iri, .. } => vec![iri.as_str()],
        }
    }
}

/// The sentence an `unreached-import` refusal adds when an `owl:imports` triple names the
/// entry from a subject that is no anchor: the triple is data, and the author most likely
/// meant it as an import. Every host renders it: `lint` is the host's spelling of the lint
/// entry point (`purrdf shapes lint` on the command line), and `None` names it by what it
/// is, the shapes lint, which every host exposes (Python `shapes.lint_shapes`, WebAssembly
/// `shaclLintShapes`, C `purrdf_shacl_lint_shapes`).
#[derive(Debug, Clone, Copy)]
pub struct UnanchoredNote<'a> {
    /// The unreached import IRIs an unanchored `owl:imports` triple names.
    pub iris: &'a [String],
    /// The host's spelling of the lint entry point, or `None` for "the shapes lint".
    pub lint: Option<&'a str>,
}

impl fmt::Display for UnanchoredNote<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let named = self
            .iris
            .iter()
            .map(|iri| format!("<{iri}>"))
            .collect::<Vec<_>>()
            .join(", ");
        let one = self.iris.len() == 1;
        let lint = self
            .lint
            .map_or_else(|| "The shapes lint".to_owned(), |lint| format!("`{lint}`"));
        write!(
            f,
            "The shapes graph does state owl:imports {named}, but on a subject that is not \
             anchored — not the IRI the document was read or imported under, not an \
             owl:Ontology header, not a sh:ShapesGraph, and not a node naming one of those as \
             its owl:versionIRI — so {that} data, not an import. {lint} lists \
             {each} in its unanchored-imports section. To import {it}, state the owl:imports on \
             the shapes graph's own IRI or on its ontology header",
            that = if one {
                "that triple is"
            } else {
                "those triples are"
            },
            each = if one { "it" } else { "each" },
            it = if one { "it" } else { "them" },
        )
    }
}

/// The IRIs of `iris` (in order) that an entry of `unanchored` names as its object.
fn unanchored_targets(unanchored: &[UnanchoredImport], iris: &[String]) -> Vec<String> {
    iris.iter()
        .filter(|iri| {
            unanchored
                .iter()
                .any(|entry| matches!(&entry.object, TermValue::Iri(object) if object == *iri))
        })
        .cloned()
        .collect()
}

impl fmt::Display for ShapesImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let named = |iris: &[String]| {
            iris.iter()
                .map(|iri| format!("<{iri}>"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        match self {
            Self::Unresolved { iris } => write!(
                f,
                "{kind}: the shapes graph's owl:imports closure names {named}, which {verb} not \
                 in the shapes graph and no import table entry supplies. PurRDF fetches \
                 nothing, and going on without an imported document would validate against a \
                 different, smaller shapes graph than the one named. Supply the document in \
                 the import table, merge the imported ontology into the shapes document, or — \
                 if the shapes document IS the imported one — read it under that IRI as its \
                 base",
                kind = self.kind(),
                named = named(iris),
                verb = if iris.len() == 1 { "is" } else { "are" },
            ),
            Self::Unreached { iris, unanchored } => {
                write!(
                    f,
                    "{kind}: the import table supplies {named}, which no owl:imports in the \
                     shapes graph's closure names, so {it} would be read and never used. Remove \
                     the {entry}, or import the IRI from the shapes graph",
                    kind = self.kind(),
                    named = named(iris),
                    it = if iris.len() == 1 { "it" } else { "they" },
                    entry = if iris.len() == 1 { "entry" } else { "entries" },
                )?;
                if !unanchored.is_empty() {
                    write!(
                        f,
                        ". {}",
                        UnanchoredNote {
                            iris: unanchored,
                            lint: None,
                        }
                    )?;
                }
                Ok(())
            }
            Self::IncompatibleVersions { conflicts } => {
                write!(
                    f,
                    "{kind}: the shapes graph's owl:imports closure holds {count} pair{s} of \
                     graphs that SHACL 1.2 Core says it SHOULD NOT hold together (\"different \
                     versions of the same series, or where one declares owl:incompatibleWith \
                     the other\"); such a shapes graph is ill-formed. Supply one version of \
                     each series, and remove a graph another declares incompatible:",
                    kind = self.kind(),
                    count = conflicts.len(),
                    s = if conflicts.len() == 1 { "" } else { "s" },
                )?;
                for conflict in conflicts {
                    let rendered = conflict.to_string();
                    let rendered = rendered.replace("the importing graph", "the shapes graph");
                    write!(f, "\n  {rendered}")?;
                }
                Ok(())
            }
            Self::InvalidEntry { iri, reason } => {
                write!(
                    f,
                    "{kind}: the import table entry <{iri}>: {reason}",
                    kind = self.kind()
                )
            }
            Self::UnresolvedLink { iris } => write!(
                f,
                "{kind}: the data graph links {named} with sh:shapesGraph (SHACL 1.2 Core \
                 section 6.4), which {verb} not in the shapes graph and no import table entry \
                 supplies. A linked graph is part of the shapes graph that validates this data \
                 graph, PurRDF fetches nothing, and going on without it would validate against \
                 a smaller shapes graph than the data graph names. Supply {it} in the import \
                 table, or merge {it} into the shapes document",
                kind = self.kind(),
                named = named(iris),
                verb = if iris.len() == 1 { "is" } else { "are" },
                it = if iris.len() == 1 {
                    "the graph"
                } else {
                    "the graphs"
                },
            ),
            Self::UnheldLink { iris } => write!(
                f,
                "{kind}: the data graph links {named} with sh:shapesGraph (SHACL 1.2 Core \
                 section 6.4), and the shapes graph this validation holds was built before \
                 the data graph was known and does not include {it}: it is not the shapes \
                 graph's loaded IRI, a graph its closure folded in, or an owl:Ontology, \
                 sh:ShapesGraph or owl:versionIRI it declares. Validate from the shapes \
                 document with the data graph given at parse time and {it} in the import \
                 table, or merge {it} into the shapes document",
                kind = self.kind(),
                named = named(iris),
                it = if iris.len() == 1 {
                    "that graph"
                } else {
                    "those graphs"
                },
            ),
            Self::InvalidLink { values } => write!(
                f,
                "{kind}: the data graph's sh:shapesGraph {values_word} {listed} {verb} not an \
                 IRI. Every value of sh:shapesGraph on a data graph is the IRI of a graph to \
                 include into the shapes graph (SHACL 1.2 Core section 6.4), so {it} names no \
                 graph at all. Replace {it} with the linked graph's IRI, or remove the triple",
                kind = self.kind(),
                values_word = if values.len() == 1 { "value" } else { "values" },
                listed = values.join(", "),
                verb = if values.len() == 1 { "is" } else { "are" },
                it = if values.len() == 1 { "it" } else { "they" },
            ),
        }
    }
}

impl std::error::Error for ShapesImportError {}

/// A shapes graph with its whole `owl:imports` closure folded in.
#[derive(Debug, Clone)]
pub struct ResolvedShapesGraph {
    /// The shapes graph and every document its closure reached, merged — or the shapes
    /// graph itself, the same `Arc`, when the closure reached no document.
    pub dataset: Arc<RdfDataset>,
    /// The shapes document's own prefix map first (its declarations win a collision), then
    /// each reached document's, in the order the closure reached it.
    pub prefixes: Vec<(String, String)>,
    /// The IRIs of every graph this shapes graph was assembled from by name, sorted: each
    /// IRI the import table declares it loaded under (an in-document `@base`, say — the
    /// `loaded` argument itself is the base and is recorded as such), each document its
    /// closure reached, and each data-graph link it resolved. [`check_data_graph_links`]
    /// reads a link as held when it is here or is the base.
    pub included: Vec<String>,
}

/// Resolve `dataset`'s `owl:imports` closure, and the data-graph links `imports` carries
/// ([`ShapesImports::link_data_graph`]), against `imports`, or refuse them.
///
/// `prefixes` is the shapes document's own `@prefix` map; `loaded` names the IRIs the
/// shapes document was read from (its base, and any `@base` it declared), in addition to
/// whatever `imports` declares. This is the ONE helper every shapes-graph entry point
/// resolves through — see the [module documentation](self).
///
/// # Errors
///
/// [`ShapesImportError::UnresolvedLink`] naming every data-graph link nothing in hand
/// resolves; then [`ShapesImportError::Unresolved`] naming every import nothing in hand
/// resolves; then [`ShapesImportError::Unreached`] naming every table entry neither an
/// import nor a link reaches; then [`ShapesImportError::IncompatibleVersions`] naming every
/// pair of graphs of the closure that are two versions of one series or of which one
/// declares `owl:incompatibleWith` the other.
pub fn resolve_shapes_imports(
    dataset: &Arc<RdfDataset>,
    prefixes: &[(String, String)],
    loaded: &[&str],
    imports: &ShapesImports,
) -> Result<ResolvedShapesGraph, ShapesImportError> {
    let widened;
    let map = if loaded.iter().all(|iri| imports.map.is_loaded(iri)) {
        &imports.map
    } else {
        let mut map = imports.map.clone();
        for iri in loaded {
            map.declare_loaded(*iri);
        }
        widened = map;
        &widened
    };
    let closure = map.closure_with_links(dataset, &imports.links);
    if !closure.unresolved().is_empty() {
        let (links, others): (Vec<String>, Vec<String>) = closure
            .unresolved()
            .iter()
            .cloned()
            .partition(|iri| imports.links.contains(iri));
        if !links.is_empty() {
            return Err(ShapesImportError::UnresolvedLink { iris: links });
        }
        return Err(ShapesImportError::Unresolved { iris: others });
    }
    // `loaded` — the base the caller parsed the shapes document under — is not repeated
    // here: it is the shapes graph's own base, recorded (and bound, and read as held)
    // as the base itself.
    let mut included: Vec<String> = map
        .loaded()
        .filter(|iri| !loaded.contains(iri))
        .map(ToOwned::to_owned)
        .chain(closure.documents().iter().map(|(iri, _)| iri.clone()))
        .chain(imports.links.iter().cloned())
        .collect();
    included.sort_unstable();
    included.dedup();
    if !closure.unreached().is_empty() {
        let iris = closure.unreached().to_vec();
        // COLD: only a refusal pays for the survey, and only to say why the entry is unused.
        let unanchored = unanchored_targets(&map.unanchored_imports(dataset), &iris);
        return Err(ShapesImportError::Unreached { iris, unanchored });
    }
    if !closure.conflicts().is_empty() {
        return Err(ShapesImportError::IncompatibleVersions {
            conflicts: closure.conflicts().to_vec(),
        });
    }
    let merged = closure
        .merge(dataset)
        .map_err(|error| ShapesImportError::InvalidEntry {
            iri: closure
                .documents()
                .first()
                .map_or_default(|(iri, _)| iri.clone()),
            reason: format!("the merged shapes graph does not freeze: {error}"),
        })?;
    let Some(merged) = merged else {
        return Ok(ResolvedShapesGraph {
            dataset: Arc::clone(dataset),
            prefixes: prefixes.to_vec(),
            included,
        });
    };
    let mut all_prefixes = prefixes.to_vec();
    for (iri, _) in closure.documents() {
        if let Some(document_prefixes) = imports.prefixes.get(iri) {
            all_prefixes.extend(document_prefixes.iter().cloned());
        }
    }
    Ok(ResolvedShapesGraph {
        dataset: merged,
        prefixes: all_prefixes,
        included,
    })
}

/// The data graph's `sh:shapesGraph` links: the IRI value of each `sh:shapesGraph` triple
/// whose subject is an anchor of the data graph — an IRI in `loaded` (the data graph's
/// retrieval IRI or base, when known) or a SHACL instance of `sh:DataGraph` — in frozen quad
/// order, each IRI once. See the [module documentation](self) for SHACL 1.2 Core §6.4.
///
/// A `sh:shapesGraph` triple on any other node is data and is not read. A data graph that
/// does not intern `sh:shapesGraph` costs one term lookup, and the graph-role classifier
/// never runs.
///
/// # Errors
///
/// [`ShapesImportError::InvalidLink`] naming every anchor's value that is not an IRI.
pub fn data_graph_links<D: DatasetView>(
    data: &D,
    loaded: &[&str],
) -> Result<Vec<String>, ShapesImportError> {
    data_graph_links_by(data, &TermValue::iri(SH_SHAPES_GRAPH_LINK), loaded)
}

/// [`data_graph_links`] with the `sh:shapesGraph` term already built, so a caller reading
/// several graphs spells it once.
pub(crate) fn data_graph_links_by<D: DatasetView>(
    data: &D,
    link: &TermValue,
    loaded: &[&str],
) -> Result<Vec<String>, ShapesImportError> {
    let Some(predicate) = data.term_id_by_value(link) else {
        return Ok(Vec::new());
    };
    let mut anchors: Vec<D::Id> = loaded
        .iter()
        .filter_map(|iri| data.term_id_by_value(&TermValue::iri(*iri)))
        .collect();
    anchors.extend(
        GraphRoleIndex::classify(data)
            .iter()
            .filter(|(_, roles)| roles.contains(GraphRoles::DATA_GRAPH))
            .map(|(node, _)| node),
    );
    anchors.sort_unstable();
    anchors.dedup();
    if anchors.is_empty() {
        return Ok(Vec::new());
    }
    let mut links: Vec<String> = Vec::new();
    let mut invalid: Vec<String> = Vec::new();
    for quad in data.quads_for_pattern(None, Some(predicate), None, GraphMatch::Any) {
        if anchors.binary_search(&quad.s).is_err() {
            continue;
        }
        match data.resolve(quad.o) {
            TermRef::Iri(iri) => {
                if !links.iter().any(|link| link == iri) {
                    links.push(iri.to_owned());
                }
            }
            TermRef::Blank { label, .. } => invalid.push(format!("_:{label}")),
            TermRef::Literal { lexical, .. } => invalid.push(format!("\"{lexical}\"")),
            TermRef::Triple { .. } => invalid.push("a triple term".to_owned()),
        }
    }
    if !invalid.is_empty() {
        invalid.sort_unstable();
        invalid.dedup();
        return Err(ShapesImportError::InvalidLink { values: invalid });
    }
    Ok(links)
}

/// Check that `shapes` — a shapes graph built without the data graph in hand — HOLDS every
/// graph the data graph `data` links (SHACL 1.2 Core §6.4; see the
/// [module documentation](self)).
///
/// A link is held when it is an IRI the shapes graph was assembled from by name (its loaded
/// IRI, a document its closure reached, a link it resolved — [`ResolvedShapesGraph::included`]),
/// its parse base, or an IRI the shapes graph resolves in place: an `owl:Ontology` or
/// `sh:ShapesGraph` it declares, or an `owl:versionIRI` value
/// ([`purrdf_core::imports::declared_import_targets`]). Every validation of a data graph runs
/// this with no `loaded` IRI; a host that knows the data graph's retrieval IRI runs it again
/// with that IRI.
///
/// A data graph with no `sh:shapesGraph` triple costs one term lookup.
///
/// # Errors
///
/// [`ShapesImportError::InvalidLink`] for a non-IRI link value;
/// [`ShapesImportError::UnheldLink`] naming every linked graph `shapes` does not hold.
pub fn check_data_graph_links<D: DatasetView>(
    data: &D,
    loaded: &[&str],
    shapes: &crate::shapes::Shapes,
) -> Result<(), ShapesImportError> {
    let links = data_graph_links(data, loaded)?;
    if links.is_empty() {
        return Ok(());
    }
    let provenance = shapes.provenance();
    let mut declared: Option<std::collections::BTreeSet<String>> = None;
    let unheld: Vec<String> = links
        .into_iter()
        .filter(|link| {
            if provenance.included_graphs().binary_search(link).is_ok()
                || provenance.base() == Some(link.as_str())
            {
                return false;
            }
            !declared
                .get_or_insert_with(|| declared_import_targets(shapes.shapes_dataset.as_ref()))
                .contains(link)
        })
        .collect();
    if unheld.is_empty() {
        Ok(())
    } else {
        Err(ShapesImportError::UnheldLink { iris: unheld })
    }
}

#[cfg(test)]
mod tests {
    use super::{ShapesImportError, ShapesImports, resolve_shapes_imports};
    use crate::ShapesError;
    use crate::engine::validate_graphs_with_config;

    const LIB: &str = "http://example.org/lib";
    /// The IRI the shapes documents below are read under.
    const SHAPES_IRI: &str = "http://example.org/shapes";

    fn graph(turtle: &str) -> std::sync::Arc<::purrdf::RdfDataset> {
        crate::text_ingest::parse_turtle_to_dataset(turtle, None).expect("turtle")
    }

    const IMPORTER: &str = "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
        <http://example.org/shapes> a owl:Ontology ; owl:imports <http://example.org/lib> .\n";

    const PREFIXES: &str = "@prefix ex: <http://example.org/ns#> .\n\
        @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
        @prefix sh: <http://www.w3.org/ns/shacl#> .\n";

    /// The data every validation below runs over: `ex:Focus`, which has no `ex:p`, and
    /// `ex:Other owl:imports ex:Target` as plain data.
    const DATA: &str = "<http://example.org/ns#Focus> <http://example.org/ns#q> \"x\" .\n\
        <http://example.org/ns#Other> <http://www.w3.org/2002/07/owl#imports> \
        <http://example.org/ns#Target> .\n";

    /// A shape that fires on [`DATA`]: `ex:Focus` needs an `ex:p`.
    const LIB_SHAPE: &str = "ex:LibShape a sh:NodeShape ; sh:targetNode ex:Focus ;\n\
        sh:property [ sh:path ex:p ; sh:minCount 1 ] .\n";

    /// The `(focus node, result path)` of every result of validating [`DATA`] against
    /// `shapes`, read under [`SHAPES_IRI`] with `imports`.
    fn results(
        shapes: &str,
        imports: &ShapesImports,
    ) -> Result<Vec<(String, String)>, ShapesError> {
        let report = validate_graphs_with_config(
            DATA,
            &format!("{PREFIXES}{shapes}"),
            Some(SHAPES_IRI),
            None,
            imports,
        )?;
        Ok(report
            .results
            .iter()
            .map(|r| {
                (
                    r.focus_node.to_string(),
                    r.result_path
                        .as_ref()
                        .map_or_else(String::new, ToString::to_string),
                )
            })
            .collect())
    }

    /// The one result [`LIB_SHAPE`] reports: `ex:Focus` has no `ex:p`.
    fn lib_shape_fired() -> Vec<(String, String)> {
        vec![(
            "<http://example.org/ns#Focus>".to_owned(),
            "<http://example.org/ns#p>".to_owned(),
        )]
    }

    /// An import table supplying `turtle` (after [`PREFIXES`]) for each IRI.
    fn table(documents: &[(&str, &str)]) -> ShapesImports {
        let mut imports = ShapesImports::new();
        for (iri, turtle) in documents {
            imports
                .insert_turtle(iri, &format!("{PREFIXES}{turtle}"))
                .expect("the document parses");
        }
        imports
    }

    fn unresolved(outcome: Result<Vec<(String, String)>, ShapesError>) -> Vec<String> {
        match outcome {
            Err(ShapesError::Imports(ShapesImportError::Unresolved { iris })) => iris,
            other => panic!("expected an unresolved-import refusal, got {other:?}"),
        }
    }

    #[test]
    fn an_unresolved_import_is_refused_and_a_supplied_one_is_merged() {
        let shapes = graph(IMPORTER);
        let Err(ShapesImportError::Unresolved { iris }) =
            resolve_shapes_imports(&shapes, &[], &[], &ShapesImports::new())
        else {
            panic!("unresolved");
        };
        assert_eq!(iris, [LIB]);

        let mut imports = ShapesImports::new();
        imports
            .insert_turtle(
                LIB,
                "@prefix ex: <http://example.org/ns#> .\nex:a ex:p ex:b .\n",
            )
            .expect("turtle");
        let resolved = resolve_shapes_imports(&shapes, &[], &[], &imports).expect("resolved");
        assert_eq!(
            resolved.dataset.quad_count(),
            3,
            "the import's triple is merged"
        );
        assert_eq!(
            resolved.prefixes,
            [("ex".to_owned(), "http://example.org/ns#".to_owned())],
            "the imported document's prefixes travel with it"
        );
    }

    #[test]
    fn a_loaded_iri_resolves_and_its_neighbour_does_not() {
        let shapes = graph(IMPORTER);
        resolve_shapes_imports(&shapes, &[], &[LIB], &ShapesImports::new())
            .expect("the shapes document is the imported one");
        assert!(matches!(
            resolve_shapes_imports(
                &shapes,
                &[],
                &["http://example.org/elsewhere"],
                &ShapesImports::new()
            ),
            Err(ShapesImportError::Unresolved { .. })
        ));
    }

    #[test]
    fn an_unreached_entry_is_refused_and_a_reached_one_is_not() {
        let mut imports = ShapesImports::new();
        imports.insert_turtle(LIB, "").expect("empty turtle");
        let Err(ShapesImportError::Unreached { iris, .. }) =
            resolve_shapes_imports(&graph(""), &[], &[], &imports)
        else {
            panic!("unreached");
        };
        assert_eq!(iris, [LIB]);
        resolve_shapes_imports(&graph(IMPORTER), &[], &[], &imports).expect("reached");
    }

    /// The vendored W3C SHACL 1.2 vocabularies, merged: `shnex.ttl` imports `sh:` and
    /// `shacl.ttl` beside it declares `sh:`, so the closure is in hand with no table. The
    /// neighbour: `shnex.ttl` alone names exactly `sh:` as unresolved.
    #[test]
    fn the_merged_w3c_vocabularies_resolve_and_shnex_alone_does_not() {
        const SHACL_TTL: &str = include_str!("../spec/shacl.ttl");
        const SHNEX_TTL: &str = include_str!("../spec/shnex.ttl");
        let merged = graph(&format!("{SHNEX_TTL}\n{SHACL_TTL}"));
        assert!(
            purrdf_core::imports::imported_iris(merged.as_ref(), &[])
                .iter()
                .any(|iri| iri == "http://www.w3.org/ns/shacl#"),
            "the merged graph does import `sh:`"
        );
        resolve_shapes_imports(&merged, &[], &[], &ShapesImports::new())
            .expect("the merged vocabularies are a complete closure");
        let Err(ShapesImportError::Unresolved { iris }) =
            resolve_shapes_imports(&graph(SHNEX_TTL), &[], &[], &ShapesImports::new())
        else {
            panic!("shnex.ttl alone imports an absent ontology");
        };
        assert_eq!(iris, ["http://www.w3.org/ns/shacl#"]);
    }

    /// An `owl:imports` on the shapes graph's own IRI is an import, and unsupplied it is
    /// refused. The neighbour writes it on `ex:Other`, which is neither the shapes graph's
    /// IRI nor an `owl:Ontology`: nothing is refused, validation runs, and the triple is
    /// data — it is still in the shapes graph, and a shape over `owl:imports` sees it.
    #[test]
    fn an_import_on_the_shapes_graph_iri_is_refused_and_one_on_another_node_is_data() {
        const MAX_ZERO: &str = "ex:NoImports a sh:NodeShape ; sh:targetNode ex:Other ;\n\
            sh:property [ sh:path owl:imports ; sh:maxCount 0 ] .\n";
        let refused = results(
            &format!("<{SHAPES_IRI}> owl:imports ex:Target .\n{MAX_ZERO}"),
            &ShapesImports::new(),
        );
        assert_eq!(unresolved(refused), ["http://example.org/ns#Target"]);

        let data = format!("ex:Other owl:imports ex:Target .\n{MAX_ZERO}");
        assert_eq!(
            results(&data, &ShapesImports::new()).expect("a non-anchor owl:imports is data"),
            [(
                "<http://example.org/ns#Other>".to_owned(),
                "<http://www.w3.org/2002/07/owl#imports>".to_owned()
            )],
            "validation ran, and the shape over owl:imports fired on the data graph's triple"
        );
        let shapes = crate::text_ingest::parse_turtle_to_dataset(
            &format!("{PREFIXES}{data}"),
            Some(SHAPES_IRI),
        )
        .expect("turtle");
        let resolved = resolve_shapes_imports(&shapes, &[], &[SHAPES_IRI], &ShapesImports::new())
            .expect("nothing to resolve");
        assert!(
            std::sync::Arc::ptr_eq(&resolved.dataset, &shapes),
            "nothing was merged"
        );
        let other = ::purrdf::TermValue::iri("http://example.org/ns#Other");
        let imports = ::purrdf::TermValue::iri("http://www.w3.org/2002/07/owl#imports");
        let target = ::purrdf::TermValue::iri("http://example.org/ns#Target");
        assert!(
            resolved.dataset.quads().any(|quad| {
                resolved.dataset.term_value(quad.s) == other
                    && resolved.dataset.term_value(quad.p) == imports
                    && resolved.dataset.term_value(quad.o) == target
            }),
            "the non-anchor owl:imports stays in the shapes graph as data"
        );
    }

    /// SHACL 1.2 Core §1.3 / §6.1: an import closure holding two versions of one series, or
    /// a graph another declares `owl:incompatibleWith`, is ill-formed and refused, naming
    /// both graphs. The neighbour — the same closure with one version of the series, the
    /// shape of Example 23 — validates, and its shape fires, so the refusal is observed
    /// against a closure that is honoured.
    #[test]
    fn two_versions_of_one_series_are_refused_and_one_version_validates() {
        const V1: &str = "http://example.org/lib/v1";
        const V2: &str = "http://example.org/lib/v2";
        let shapes = "<http://example.org/shapes> a owl:Ontology ;\n\
            owl:imports <http://example.org/lib/v2> , <http://example.org/app> .\n";
        let series = |version: &str| {
            format!("<http://example.org/lib> owl:versionIRI <{version}> .\n{LIB_SHAPE}")
        };
        let app = |imports: &str, extra: &str| {
            format!("<http://example.org/app> a owl:Ontology ; owl:imports <{imports}> .\n{extra}")
        };

        // Two versions of <http://example.org/lib>: v2 directly, v1 through app.
        let outcome = results(
            shapes,
            &table(&[
                (V2, &series(V2)),
                (V1, &series(V1)),
                ("http://example.org/app", &app(V1, "")),
            ]),
        );
        let Err(ShapesError::Imports(error @ ShapesImportError::IncompatibleVersions { .. })) =
            outcome
        else {
            panic!("expected an incompatible-versions refusal, got {outcome:?}");
        };
        assert_eq!(error.kind(), "incompatible-import-versions");
        assert_eq!(error.iris(), [V2, V1]);
        let message = error.to_string();
        assert!(
            message.contains(
                "<http://example.org/lib/v2> and <http://example.org/lib/v1> are different \
                 versions of the series <http://example.org/lib>"
            ),
            "{message}"
        );

        // A graph declaring owl:incompatibleWith a version the closure holds.
        let outcome = results(
            shapes,
            &table(&[
                (V2, &series(V2)),
                (
                    "http://example.org/app",
                    &app(
                        "http://example.org/other",
                        "<http://example.org/app> owl:incompatibleWith <http://example.org/lib/v2> .\n",
                    ),
                ),
                ("http://example.org/other", "ex:x ex:y ex:z .\n"),
            ]),
        );
        let Err(ShapesError::Imports(ShapesImportError::IncompatibleVersions { conflicts })) =
            outcome
        else {
            panic!("expected an incompatible-versions refusal, got {outcome:?}");
        };
        assert_eq!(conflicts.len(), 1);
        assert_eq!(
            conflicts[0].first.as_deref(),
            Some("http://example.org/app")
        );
        assert_eq!(conflicts[0].second.as_deref(), Some(V2));

        // Neighbour: app imports the same version, so the closure holds one version, and
        // the series' shape fires.
        assert_eq!(
            results(
                shapes,
                &table(&[(V2, &series(V2)), ("http://example.org/app", &app(V2, ""))]),
            )
            .expect("one version of the series"),
            lib_shape_fired()
        );
    }

    /// An `owl:Ontology` header's import counts: unsupplied it is refused; supplied, the
    /// imported document's shape fires. The control — the same shapes graph with no header
    /// and no table — reports nothing, so the result is the imported shape's.
    #[test]
    fn an_ontology_header_import_is_refused_unsupplied_and_merged_supplied() {
        let header = "ex:O a owl:Ontology ; owl:imports <http://example.org/lib> .\n";
        assert_eq!(unresolved(results(header, &ShapesImports::new())), [LIB]);
        assert_eq!(
            results(header, &table(&[(LIB, LIB_SHAPE)])).expect("supplied"),
            lib_shape_fired()
        );
        assert_eq!(
            results(
                "ex:O owl:imports <http://example.org/lib> .\n",
                &ShapesImports::new()
            )
            .expect("no header, no import"),
            []
        );
    }

    /// An anonymous ontology header imports too; an untyped blank node does not.
    #[test]
    fn a_blank_node_header_imports_and_an_untyped_blank_node_does_not() {
        assert_eq!(
            unresolved(results(
                "[] a owl:Ontology ; owl:imports <http://example.org/lib> .\n",
                &ShapesImports::new()
            )),
            [LIB]
        );
        assert_eq!(
            results(
                "[] a ex:Thing ; owl:imports <http://example.org/lib> .\n",
                &ShapesImports::new()
            )
            .expect("an untyped blank node imports nothing"),
            []
        );
        assert_eq!(
            results(
                "[] a owl:Ontology ; owl:imports <http://example.org/lib> .\n",
                &table(&[(LIB, LIB_SHAPE)])
            )
            .expect("supplied"),
            lib_shape_fired()
        );
    }

    /// `^owl:versionIRI?/owl:imports`: a node whose version IRI is the shapes graph's IRI
    /// stands for the shapes graph, so its import counts. The neighbour versions another IRI
    /// and its `owl:imports` is data.
    #[test]
    fn a_node_versioning_the_shapes_graph_iri_imports_and_one_versioning_another_does_not() {
        let versioned = |version: &str| {
            format!(
                "ex:Series owl:versionIRI <{version}> ; owl:imports <http://example.org/lib> .\n"
            )
        };
        assert_eq!(
            unresolved(results(&versioned(SHAPES_IRI), &ShapesImports::new())),
            [LIB]
        );
        assert_eq!(
            results(&versioned(SHAPES_IRI), &table(&[(LIB, LIB_SHAPE)])).expect("supplied"),
            lib_shape_fired()
        );
        assert_eq!(
            results(
                &versioned("http://example.org/elsewhere"),
                &ShapesImports::new()
            )
            .expect("not an anchor"),
            []
        );
    }

    /// The closure is followed through an imported document's OWN header: the shape of the
    /// document it imports fires. The neighbour's imported document writes the same
    /// `owl:imports` on a node that is no anchor of it, so the walk stops there and a table
    /// entry for the deeper document is refused as unreached.
    #[test]
    fn an_import_through_an_imported_documents_header_is_followed_and_a_non_anchor_one_is_not() {
        const DEEP: &str = "http://example.org/deep";
        let root = format!("<{SHAPES_IRI}> owl:imports <{LIB}> .\n");
        let lib_header = format!("ex:LibHeader a owl:Ontology ; owl:imports <{DEEP}> .\n");
        assert_eq!(
            unresolved(results(&root, &table(&[(LIB, &lib_header)]))),
            [DEEP]
        );
        assert_eq!(
            results(&root, &table(&[(LIB, &lib_header), (DEEP, LIB_SHAPE)])).expect("supplied"),
            lib_shape_fired()
        );

        let lib_data = format!("ex:LibNode owl:imports <{DEEP}> .\n");
        assert_eq!(
            results(&root, &table(&[(LIB, &lib_data)])).expect("the deeper triple is data"),
            []
        );
        let Err(ShapesError::Imports(ShapesImportError::Unreached { iris, .. })) =
            results(&root, &table(&[(LIB, &lib_data), (DEEP, LIB_SHAPE)]))
        else {
            panic!("a document only a data triple names is never reached");
        };
        assert_eq!(iris, [DEEP]);
    }

    /// An `owl:imports` on a SHACL instance of `sh:ShapesGraph` is an import, whichever way
    /// the node is one: typed `sh:ShapesGraph`, typed `sh:RulesGraph` (a subclass by the
    /// SPARQL Extensions' own text, stated here without the axiom), or typed a user class
    /// the shapes graph declares `rdfs:subClassOf sh:ShapesGraph`. Each is refused
    /// unsupplied and, supplied, the imported shape fires. The control row types the node an
    /// unrelated class: its triple is data, nothing is refused and no shape fires.
    #[test]
    fn an_import_on_every_shapes_graph_instance_is_followed_and_an_unrelated_type_is_data() {
        const NODE: &str = "ex:Module";
        let typed = |class: &str, axioms: &str| {
            format!("{axioms}{NODE} a {class} ; owl:imports <{LIB}> .\n")
        };
        for shapes in [
            typed("sh:ShapesGraph", ""),
            typed("sh:RulesGraph", ""),
            typed(
                "ex:ShapesModule",
                "ex:ShapesModule <http://www.w3.org/2000/01/rdf-schema#subClassOf> \
                 sh:ShapesGraph .\n",
            ),
        ] {
            assert_eq!(
                unresolved(results(&shapes, &ShapesImports::new())),
                [LIB],
                "{shapes}"
            );
            assert_eq!(
                results(&shapes, &table(&[(LIB, LIB_SHAPE)])).expect("supplied"),
                lib_shape_fired(),
                "{shapes}"
            );
        }
        let control = typed("ex:Unrelated", "");
        assert_eq!(
            results(&control, &ShapesImports::new()).expect("an unrelated type anchors nothing"),
            []
        );
    }

    /// Two shapes graphs declared in one document: BOTH imports are named when unsupplied,
    /// and both documents' shapes fire when supplied — anchoring only one would silently drop
    /// the other's.
    #[test]
    fn two_shapes_graphs_in_one_document_are_both_anchored() {
        const OTHER_LIB: &str = "http://example.org/other-lib";
        const OTHER_SHAPE: &str = "ex:OtherShape a sh:NodeShape ; sh:targetNode ex:Focus ;\n\
            sh:property [ sh:path ex:q ; sh:maxCount 0 ] .\n";
        let shapes = format!(
            "ex:A a sh:ShapesGraph ; owl:imports <{LIB}> .\n\
             ex:B a sh:ShapesGraph ; owl:imports <{OTHER_LIB}> .\n"
        );
        assert_eq!(
            unresolved(results(&shapes, &ShapesImports::new())),
            [LIB, OTHER_LIB]
        );
        let mut fired = results(
            &shapes,
            &table(&[(LIB, LIB_SHAPE), (OTHER_LIB, OTHER_SHAPE)]),
        )
        .expect("both supplied");
        fired.sort();
        assert_eq!(
            fired,
            [
                (
                    "<http://example.org/ns#Focus>".to_owned(),
                    "<http://example.org/ns#p>".to_owned()
                ),
                (
                    "<http://example.org/ns#Focus>".to_owned(),
                    "<http://example.org/ns#q>".to_owned()
                ),
            ]
        );
    }

    /// A node whose only graph role is `sh:DataGraph` anchors no import (SHACL 1.2 Core
    /// §6.2): nothing is refused, no document is looked for, and a supplied one is refused
    /// as unreached rather than silently used. The neighbour types the same node
    /// `owl:Ontology` as well — §6.2's note — and its import is followed.
    #[test]
    fn a_data_graph_only_import_is_not_followed_and_a_data_graph_ontology_is() {
        let data_only = format!("ex:D a sh:DataGraph ; owl:imports <{LIB}> .\n");
        assert_eq!(
            results(&data_only, &ShapesImports::new()).expect("a data-graph import is data"),
            []
        );
        let Err(ShapesError::Imports(refusal @ ShapesImportError::Unreached { .. })) =
            results(&data_only, &table(&[(LIB, LIB_SHAPE)]))
        else {
            panic!("a document only a data-graph triple names is never reached");
        };
        assert_eq!(
            refusal,
            ShapesImportError::Unreached {
                iris: vec![LIB.to_owned()],
                unanchored: vec![LIB.to_owned()],
            }
        );
        // The refusal says WHY the stated owl:imports is not one, and where the lint lists
        // it — the wording every non-CLI host (Python, WebAssembly, C) carries verbatim.
        let message = refusal.to_string();
        assert!(
            message.contains(&format!("does state owl:imports <{LIB}>"))
                && message.contains("not anchored")
                && message.contains("The shapes lint lists it in its unanchored-imports section"),
            "{message}"
        );

        // The observing neighbour: an entry nothing names at all is unreached with NO
        // unanchored note, so the note is about the triple, not about every refusal.
        let Err(ShapesError::Imports(bare)) = results("", &table(&[(LIB, LIB_SHAPE)])) else {
            panic!("an entry nothing names is never reached");
        };
        assert_eq!(
            bare,
            ShapesImportError::Unreached {
                iris: vec![LIB.to_owned()],
                unanchored: vec![],
            }
        );
        assert!(!bare.to_string().contains("not anchored"), "{bare}");

        let with_header = format!("ex:D a sh:DataGraph , owl:Ontology ; owl:imports <{LIB}> .\n");
        assert_eq!(
            unresolved(results(&with_header, &ShapesImports::new())),
            [LIB]
        );
        assert_eq!(
            results(&with_header, &table(&[(LIB, LIB_SHAPE)])).expect("supplied"),
            lib_shape_fired()
        );
    }

    #[test]
    fn a_relative_or_repeated_key_is_an_invalid_entry_and_an_absolute_one_is_not() {
        let mut imports = ShapesImports::new();
        assert!(matches!(
            imports.insert_turtle("lib", ""),
            Err(ShapesImportError::InvalidEntry { .. })
        ));
        imports.insert_turtle(LIB, "").expect("absolute");
        assert!(matches!(
            imports.insert_turtle(LIB, ""),
            Err(ShapesImportError::InvalidEntry { .. })
        ));
        assert!(matches!(
            imports.insert_turtle("http://example.org/bad", "<a"),
            Err(ShapesImportError::InvalidEntry { .. })
        ));
    }
}
