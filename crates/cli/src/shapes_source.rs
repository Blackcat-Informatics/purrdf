// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Reading a SHACL shapes graph and its `--import IRI=FILE` documents — the ONE seam every
//! lane that starts from a shapes DOCUMENT reads through.
//!
//! `validate --shapes`, `shacl pack`, `rules --shapes`, `node-expr` and `shapes lint` all
//! turn a shapes document (plus whatever `--import IRI=FILE` names) into the shapes graph
//! the engine parses. They read the document with [`read_shapes_document`] and the pairs
//! with [`shapes_imports`], and hand both to the engine, which resolves the `owl:imports`
//! closure itself — through `purrdf_shapes::imports::resolve_shapes_imports`, the one helper
//! every PurRDF host resolves through. This module does not walk the closure: a second walk
//! here is how the command line once refused shapes graphs the library accepted, so the
//! command line now asks the same question every other host asks and renders the one
//! answer ([`shapes_error`]) in flags.
//!
//! # Why this is not a fetch
//!
//! Jena resolves `owl:imports` by dereferencing the IRI over HTTP. PurRDF cannot and will
//! not: it ships no HTTP client, every release crate must build for `wasm32-unknown-unknown`,
//! and a validation verdict that depends on what a URL served today is not reproducible. So
//! the closure is caller-supplied configuration, the same answer `entails --import` and
//! `shex --import` give — see `purrdf_core::imports`, whose doctrine paragraph is the one
//! this follows.
//!
//! # An unresolved import is a refusal; an import already in the graph is not unresolved
//!
//! An `owl:imports` whose ontology is not in hand is REFUSED, naming every such IRI and the
//! `--import IRI=FILE` pair that resolves it. The alternative — validating against the
//! shapes graph alone — is a verdict about a different, smaller shapes graph than the one the
//! operator named: a shapes graph whose shapes all live in an imported document would
//! "conform" against no shapes at all. A warning beside that verdict does not undo it, so
//! there is no warn-and-continue path.
//!
//! What keeps this from refusing valid input is the resolution rule, `purrdf_core::imports`:
//! an import is resolved when it names a document already LOADED — the shapes document's own
//! retrieval IRI or base (including an in-document `@base`), or an `--import` document's — or
//! when the closure already HOLDS the ontology it names (`<X> a owl:Ontology`, or an ontology
//! whose `owl:versionIRI` is `<X>`) — or, for a shapes graph, describes `<X>` with
//! `sh:declare`. That last case is SHACL's own idiom: `sh:prefixes` collects `sh:declare`s
//! along `sh:prefixes/owl:imports*/sh:declare` within the shapes graph, so the import names a
//! node the shapes graph declares prefixes on, not a document to fetch — the W3C
//! `prefixes-001` vector validates as written. A shapes document that merges the W3C
//! SHACL 1.2 vocabularies — `shnex.ttl` importing `sh:`, beside the `shacl.ttl` that declares
//! it — is therefore complete as written and needs no `--import`.
//!
//! Naming a pair also makes it MANDATORY that the pair is used: a pair the closure never
//! reaches is refused, because it would be read and never used — the engine's
//! `unreached-import`, rendered here as a usage error.
//!
//! # `--shapes-graph` lives here too, for the same reason
//!
//! [`resolve_shapes_graph`] is the second thing `validate --shapes` and `shacl pack` must
//! agree on: both read a `--shapes-graph IRI` argument and both have to resolve it against
//! the SAME base their shapes document parses under, or a product packed with one answer and
//! a document validated with the other would expose `$shapesGraph` under two different IRIs
//! for what is supposed to be one shapes graph. Living beside [`shapes_imports`] is what
//! keeps that agreement from being re-derived per caller.

use std::sync::Arc;

use purrdf::shapes::{ShapesError, ShapesImportError, ShapesImports};
use purrdf_core::RdfDataset;
use purrdf_core::imports::imported_iris;
use purrdf_iri::{BaseIri, BaseOrigin, BaseScope};
use purrdf_rdf::{NativeRdfFormat, SourceFormat};

use crate::error::CliError;
use crate::{format, source};

/// A shapes document READ but not yet parsed into `Shapes`: the frozen graph plus the
/// `@prefix`/`PREFIX` map recovered from its source text.
///
/// The two travel together because they are only jointly meaningful. SHACL-AF `sh:select`
/// bodies may use prefixed names, the frozen IR does not retain a document's prefix map, and
/// an IMPORTED document's queries resolve against ITS OWN declarations — so folding a closure
/// has to carry every document's prefixes forward, not just the root's.
pub(crate) struct ShapesDocument {
    /// The document's quads.
    pub(crate) dataset: Arc<RdfDataset>,
    /// Its own prefix declarations, empty for any non-Turtle syntax (the recovery is a scan
    /// of Turtle source text, which no other syntax offers).
    pub(crate) prefixes: Vec<(String, String)>,
    /// The IRIs this document was read FROM: the base it was parsed under (its `file://`
    /// retrieval IRI, `--base`, or the ontology IRI an `--import` pair named) and, for
    /// Turtle, the base an in-document `@base` established. An `owl:imports` of one of
    /// these names a document already loaded, so it is resolved in place.
    pub(crate) loaded: Vec<String>,
}

/// Read one shapes document, by the same two routes `purrdf_shapes::engine::parse_shapes`
/// takes, stopping one step short of parsing it into `Shapes`.
///
/// Stopping short is what makes an import closure possible at all: the imports have to be
/// read off the GRAPH, and the documents merged as graphs, before anything is asked to be a
/// shape. The composition is deliberately the identical one `parse_shapes` performs —
/// `parse_turtle_document` (the dataset and the codec's own prefix map, from one parse),
/// then `from_dataset_with_config(…, None)` —
/// so a single document with no imports parses to exactly the `Shapes` it did before this
/// seam existed. `what` names the flag for the diagnostic, since this reads `--shapes` and
/// `--import` alike.
pub(crate) fn read_shapes_document(
    path: &str,
    format: SourceFormat,
    base: Option<&str>,
    what: &str,
) -> Result<ShapesDocument, CliError> {
    if format == SourceFormat::Native(NativeRdfFormat::Turtle) {
        let bytes = source::read_bytes(path)?;
        let text = String::from_utf8(bytes).map_err(|error| {
            CliError::Runtime(format!("{what} {path}: not UTF-8 text: {error}"))
        })?;
        let purrdf::shapes::text_ingest::TurtleDocument {
            dataset,
            base: document_base,
            prefixes,
        } = purrdf::shapes::text_ingest::parse_turtle_document(&text, base)
            .map_err(|errors| CliError::Runtime(format!("{what} {path}: {}", errors.join("\n"))))?;
        let mut loaded: Vec<String> = base.into_iter().map(str::to_owned).collect();
        if let Some(document_base) = document_base
            && !loaded.contains(&document_base)
        {
            loaded.push(document_base);
        }
        return Ok(ShapesDocument {
            dataset,
            prefixes,
            loaded,
        });
    }

    Ok(ShapesDocument {
        dataset: source::load_dataset(path, format, base)?,
        prefixes: Vec::new(),
        loaded: base.into_iter().map(str::to_owned).collect(),
    })
}

/// Build the shapes graph's `owl:imports` table from the `--import IRI=FILE` pairs.
///
/// `imports` is the raw `--import IRI=FILE` table exactly as the operator wrote it — this
/// function is agnostic to which command line it came from, which is what lets
/// `validate --shapes`, `shacl pack`, `shacl rules`, `shacl node-expr` and `shapes lint`
/// share it. Every pair is decided before any file is opened, then each document is read
/// under its ontology IRI as its base (the per-document base an `owl:imports` names), with
/// its own prefix map. The IRIs the shapes document itself was read under (`root.loaded`)
/// and those an imported document's `@base` declares are declared loaded.
///
/// This builds the TABLE only. Whether the closure is complete — and whether every pair is
/// reached — is decided by the engine, by the one helper every host resolves through
/// (`purrdf_shapes::imports::resolve_shapes_imports`), when the shapes graph is parsed;
/// [`shapes_error`] renders its refusal in command-line terms.
pub(crate) fn shapes_imports(
    root: &ShapesDocument,
    imports: &[String],
) -> Result<ShapesImports, CliError> {
    let pairs = resolve_shapes_import_pairs(imports)?;
    let mut table = ShapesImports::new();
    for iri in &root.loaded {
        table.declare_loaded(iri.clone());
    }
    for pair in &pairs {
        let document = read_shapes_document(
            pair.path,
            pair.format,
            Some(pair.iri),
            &format!("--import {}", pair.iri),
        )?;
        for loaded in &document.loaded {
            if loaded != pair.iri {
                table.declare_loaded(loaded.clone());
            }
        }
        table
            .insert(pair.iri, document.dataset, document.prefixes)
            .map_err(|error| CliError::Usage(format!("--import {}: {error}", pair.spec)))?;
    }
    Ok(table)
}

/// Render a shapes-graph refusal for the command line.
///
/// The typed [`ShapesImportError`] is the SAME refusal every host raises; only its remedy is
/// spelled here in flags. An unresolved import is a runtime refusal (exit 1) naming every
/// missing IRI, the `--import IRI=FILE` pair that resolves each, and — for a document that
/// imports its own IRI — `base_flag`, the flag that sets the shapes document's base on the
/// calling command (`--shapes-base` for `validate`, `--base` for `shacl pack`). A pair the
/// closure never reaches is a USAGE error (exit 2): the fault is in the command line, and
/// only its discovery needed the documents. Any other refusal is `context: message`.
pub(crate) fn shapes_error(
    error: ShapesError,
    context: &str,
    root: &ShapesDocument,
    base_flag: &str,
) -> CliError {
    let error = match error {
        ShapesError::Imports(error) => error,
        ShapesError::Invalid(message) => return CliError::Runtime(format!("{context}: {message}")),
        ShapesError::ShaclJs(refusal) => {
            return CliError::Runtime(format!("{context}: {refusal}"));
        }
    };
    match error {
        ShapesImportError::Unresolved { iris } => {
            let named: Vec<String> = iris.iter().map(|iri| format!("<{iri}>")).collect();
            let remedy: Vec<String> = iris
                .iter()
                .map(|iri| format!("`--import {iri}=FILE`"))
                .collect();
            CliError::Runtime(format!(
                "unresolved-import: the shapes graph's owl:imports closure names {named}, which \
                 {verb} not in the shapes graph and no --import pair resolves. PurRDF fetches \
                 nothing the operator did not name, and going on without an imported document \
                 would use a different, smaller shapes graph than the one named. Pass {remedy} \
                 to fold {it} in, or merge the imported ontology into the shapes document. If \
                 the shapes document IS {one_of}, read it under that IRI with {self_remedy}",
                named = named.join(", "),
                verb = if iris.len() == 1 { "is" } else { "are" },
                remedy = remedy.join(" "),
                it = if iris.len() == 1 { "it" } else { "them" },
                one_of = if iris.len() == 1 {
                    named.join("")
                } else {
                    format!("one of {}", named.join(", "))
                },
                self_remedy = if iris.len() == 1 {
                    format!("`{base_flag} {}`", iris[0])
                } else {
                    format!("`{base_flag} IRI`")
                },
            ))
        }
        ShapesImportError::Unreached { iris } => {
            let named: Vec<String> = iris.iter().map(|iri| format!("<{iri}>")).collect();
            if imported_iris(&root.dataset).is_empty() {
                return CliError::Usage(format!(
                    "unreached-import: --import {named}: the shapes graph has no owl:imports at \
                     all, so {these} would be read and never used. Remove the pair, or import \
                     the IRI from the shapes graph",
                    named = named.join(", "),
                    these = if iris.len() == 1 {
                        "this document"
                    } else {
                        "these documents"
                    },
                ));
            }
            CliError::Usage(format!(
                "unreached-import: --import {named}: the shapes graph's import closure never \
                 reaches {it}, so {these} would be read and never used. Remove the pair, or \
                 import the IRI from the shapes graph",
                named = named.join(", "),
                it = if iris.len() == 1 { "it" } else { "them" },
                these = if iris.len() == 1 {
                    "this document"
                } else {
                    "these documents"
                },
            ))
        }
        error @ ShapesImportError::InvalidEntry { .. } => {
            CliError::Usage(format!("--import {error}"))
        }
    }
}

/// One `--import IRI=FILE` argument for the SHACL lane, fully DECIDED but not yet read.
struct ShapesImportPair<'a> {
    /// The pair exactly as the operator wrote it, so a diagnostic can quote it back.
    spec: &'a str,
    /// The ontology-IRI half, checked absolute.
    iri: &'a str,
    /// The document-path half.
    path: &'a str,
    /// The syntax that path's own extension classifies it as.
    format: SourceFormat,
}

/// Decide every `--import IRI=FILE` ARGUMENT, with no I/O at all.
///
/// Nothing here touches the filesystem, so the FIRST malformed pair is reported before the
/// FIRST file is opened — a malformed pair is a usage error naming the argument, never a
/// skipped import, because a shapes graph folded without a document the operator supplied is
/// a different shapes graph. This mirrors `shex`'s `resolve_import_pairs`, including why the
/// IRI half must be ABSOLUTE: it is matched against the shapes graph's `owl:imports` objects,
/// which are absolute by the time the parser is done with them, and it is the base the
/// imported document parses under.
fn resolve_shapes_import_pairs(specs: &[String]) -> Result<Vec<ShapesImportPair<'_>>, CliError> {
    let mut pairs: Vec<ShapesImportPair<'_>> = Vec::with_capacity(specs.len());
    for spec in specs {
        let Some((iri, path)) = spec.split_once('=') else {
            return Err(CliError::Usage(format!(
                "--import {spec}: an import pair is `IRI=FILE` — the ontology IRI the shapes \
                 graph imports, then the local document that resolves it — and this one has \
                 no `=`"
            )));
        };
        if iri.is_empty() || path.is_empty() {
            return Err(CliError::Usage(format!(
                "--import {spec}: both halves of `IRI=FILE` are required — the IRI names what \
                 the shapes graph imports, and the path names the document that is it"
            )));
        }
        if path == "-" {
            return Err(CliError::Usage(format!(
                "--import {spec}: an imported document's syntax is inferred from its own path \
                 extension, and `-` has none. Write the document to a file, or name it with a \
                 recognized RDF extension"
            )));
        }
        if let Err(error) = BaseIri::parse(iri) {
            return Err(CliError::Usage(format!(
                "--import {spec}: the IRI half `{iri}` is not an absolute IRI ({code}): it is \
                 matched against the shapes graph's owl:imports objects, which are absolute, \
                 and it is the base the imported document parses under. {error}",
                code = error.diagnostic_code()
            )));
        }
        if pairs.iter().any(|seen| seen.iri == iri) {
            return Err(CliError::Usage(format!(
                "--import {iri}=…: the IRI is named twice, and one IRI resolves to one \
                 document; the second pair would be read and never used"
            )));
        }
        pairs.push(ShapesImportPair {
            spec,
            iri,
            path,
            format: format::resolve(None, path)?,
        });
    }
    Ok(pairs)
}

/// Resolve a `--shapes-graph` value against the shapes document's base.
///
/// `--shapes-graph` names the graph the shapes document is exposed under to SHACL-SPARQL
/// paths, overriding a `sh:shapesGraph` that document declares. That declaration is an IRI
/// *inside* the shapes document, so it resolves against the shapes document's base — and the
/// flag that overrides it has to resolve against the SAME base, or the two would disagree
/// about what a relative reference names. `validate --shapes` and `shacl pack` share this one
/// function for exactly that reason: each derives its own document base independently (a
/// validation run and a pack run read the document on separate occasions), but both spend it
/// on `--shapes-graph` through this single derivation, which is what keeps the flag and the
/// document agreeing about what a relative IRI denotes no matter which command resolved it.
///
/// An ABSOLUTE value is carried lexical-verbatim — [`BaseScope::resolve`]'s own contract — so
/// an already-absolute invocation is byte-for-byte what it always was. A RELATIVE one resolves
/// against `base`, so `--shapes-graph sg` names exactly what `sh:shapesGraph <sg>` written in
/// the shapes document names. A relative one with nothing in scope is refused: see
/// [`shapes_graph_refusal`].
pub(crate) fn resolve_shapes_graph(
    raw: Option<&str>,
    base: Option<&str>,
) -> Result<Option<String>, CliError> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let scope = match base {
        // A derived retrieval IRI is produced by its own parse, so a failure here is not
        // reachable from the command line; it is still reported rather than unwrapped,
        // because an unreachable panic in a CLI is a crash report.
        Some(base) => BaseScope::rooted(
            BaseIri::parse(base).map_err(|error| {
                CliError::Usage(format!(
                    "the shapes graph's base `{base}` is not a usable base IRI: {error}"
                ))
            })?,
            BaseOrigin::Caller,
        ),
        None => BaseScope::empty(),
    };
    scope
        .resolve(raw)
        .map(|iri| Some(iri.as_str().to_owned()))
        .map_err(|error| shapes_graph_refusal(raw, &error))
}

/// The refusal for a `--shapes-graph` that names no graph.
///
/// It carries the shared [`purrdf_iri::IriError::diagnostic_code`] so it groups with every
/// other IRI failure in this toolkit, and it does NOT carry the library's own remedy for a
/// missing base: that one names `@base` and `xml:base`, which are DOCUMENT directives, and a
/// `--shapes-graph` value is argv text that no document can reach. Naming a fix the operator
/// cannot apply is worse than naming none — this is the same refusal shape `describe --iri`
/// carries, for the same reason.
fn shapes_graph_refusal(raw: &str, error: &purrdf_iri::IriError) -> CliError {
    let code = error.diagnostic_code();
    if code == "iri-relative-no-base" {
        return CliError::Usage(format!(
            "--shapes-graph `{raw}`: {code}: a relative IRI reference has no base in scope, so \
             it names no graph to expose the shapes under. This is a command-line value, so no \
             `@base` you write in a document resolves it: give --shapes a PATH, whose `file://` \
             retrieval IRI this flag resolves against exactly as a `sh:shapesGraph` inside that \
             document would, or write the graph name in absolute form"
        ));
    }
    CliError::Usage(format!("--shapes-graph `{raw}`: {code}: {error}"))
}
