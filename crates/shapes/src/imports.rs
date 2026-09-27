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
use purrdf_core::imports::ImportMap;

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
    /// `unreached-import` or `invalid-import`. It is what the Python, JavaScript and C hosts
    /// carry in their own typed slot.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Unresolved { .. } => "unresolved-import",
            Self::Unreached { .. } => "unreached-import",
            Self::InvalidEntry { .. } => "invalid-import",
        }
    }

    /// The IRIs the refusal names.
    #[must_use]
    pub fn iris(&self) -> Vec<&str> {
        match self {
            Self::Unresolved { iris } | Self::Unreached { iris } => {
                iris.iter().map(String::as_str).collect()
            }
            Self::InvalidEntry { iri, .. } => vec![iri.as_str()],
        }
    }
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
            Self::Unreached { iris } => write!(
                f,
                "{kind}: the import table supplies {named}, which no owl:imports in the shapes \
                 graph's closure names, so {it} would be read and never used. Remove the \
                 {entry}, or import the IRI from the shapes graph",
                kind = self.kind(),
                named = named(iris),
                it = if iris.len() == 1 { "it" } else { "they" },
                entry = if iris.len() == 1 { "entry" } else { "entries" },
            ),
            Self::InvalidEntry { iri, reason } => {
                write!(
                    f,
                    "{kind}: the import table entry <{iri}>: {reason}",
                    kind = self.kind()
                )
            }
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
}

/// Resolve `dataset`'s `owl:imports` closure against `imports`, or refuse it.
///
/// `prefixes` is the shapes document's own `@prefix` map; `loaded` names the IRIs the
/// shapes document was read from (its base, and any `@base` it declared), in addition to
/// whatever `imports` declares. This is the ONE helper every shapes-graph entry point
/// resolves through — see the [module documentation](self).
///
/// # Errors
///
/// [`ShapesImportError::Unresolved`] naming every import nothing in hand resolves; then
/// [`ShapesImportError::Unreached`] naming every table entry no import reaches.
pub fn resolve_shapes_imports(
    dataset: &Arc<RdfDataset>,
    prefixes: &[(String, String)],
    loaded: &[&str],
    imports: &ShapesImports,
) -> Result<ResolvedShapesGraph, ShapesImportError> {
    let closure = if loaded.iter().all(|iri| imports.map.is_loaded(iri)) {
        imports.map.closure(dataset)
    } else {
        let mut map = imports.map.clone();
        for iri in loaded {
            map.declare_loaded(*iri);
        }
        map.closure(dataset)
    };
    if !closure.unresolved().is_empty() {
        return Err(ShapesImportError::Unresolved {
            iris: closure.unresolved().to_vec(),
        });
    }
    if !closure.unreached().is_empty() {
        return Err(ShapesImportError::Unreached {
            iris: closure.unreached().to_vec(),
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
    })
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
        let Err(ShapesImportError::Unreached { iris }) =
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
        let Err(ShapesError::Imports(ShapesImportError::Unreached { iris })) =
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
        let Err(ShapesError::Imports(ShapesImportError::Unreached { iris })) =
            results(&data_only, &table(&[(LIB, LIB_SHAPE)]))
        else {
            panic!("a document only a data-graph triple names is never reached");
        };
        assert_eq!(iris, [LIB]);

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
