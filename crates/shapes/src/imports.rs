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
//! # One rule, every host
//!
//! When an import counts as in hand is [`purrdf_core::imports`] — the same rule entailment
//! applies, stated once in the kernel both engines sit on. This module adds only what a
//! SHACL shapes graph needs beyond it: each supplied document's own `@prefix` map (a
//! SHACL-SPARQL query in an imported document resolves prefixed names against ITS
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
///     <http://example.org/shapes> owl:imports <http://example.org/lib> .\n";
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
    /// retrieval IRI, or the base it was parsed under. An `owl:imports <iri>` then names
    /// a document already in hand. The SHACL idiom this serves is `sh:prefixes`
    /// collecting `sh:declare`s along `owl:imports*`, which routinely points at the shapes
    /// document's OWN IRI from a node that is no `owl:Ontology`.
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
    /// loaded document, no `owl:Ontology` or `owl:versionIRI` declaration in the closure.
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
                .map(|(iri, _)| iri.clone())
                .unwrap_or_default(),
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

    const LIB: &str = "http://example.org/lib";

    fn graph(turtle: &str) -> std::sync::Arc<::purrdf::RdfDataset> {
        crate::text_ingest::parse_turtle_to_dataset(turtle, None).expect("turtle")
    }

    const IMPORTER: &str = "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
        <http://example.org/shapes> owl:imports <http://example.org/lib> .\n";

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
            2,
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
