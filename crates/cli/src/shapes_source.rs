// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Reading a SHACL shapes graph and folding its `owl:imports` closure — the ONE seam every
//! lane that starts from a shapes DOCUMENT reads through.
//!
//! `validate --shapes` and `shacl pack` are two commands with one job in common: turn a
//! shapes document (plus whatever `--import IRI=FILE` resolves) into the dataset a shapes
//! graph is parsed from. Before this module existed they did that job twice, at two
//! different seams, and the two copies quietly diverged — `shacl pack` read raw Turtle text
//! straight into [`purrdf_shapes::engine::parse_shapes`], which never sees an `--import`
//! table and cannot fold a closure or even report one as unresolved, so a product packed
//! from a shapes graph with an `owl:imports` silently carried FEWER shapes than
//! `validate --shapes` validated against, with nothing printed to say so. Extracting the
//! read-then-fold sequence here, for both lanes to call, is what makes that divergence
//! impossible to reintroduce: there is exactly one implementation of "what does this shapes
//! graph mean", and every command that needs it calls the same one.
//!
//! # Why this is not a fetch
//!
//! Jena resolves `owl:imports` by dereferencing the IRI over HTTP. PurRDF cannot and will
//! not: it ships no HTTP client, every release crate must build for `wasm32-unknown-unknown`,
//! and a validation verdict that depends on what a URL served today is not reproducible. So
//! the closure is caller-supplied configuration, the same answer `entails --import` and
//! `shex --import` give — see `purrdf_entail::entails::imports`, whose doctrine paragraph is
//! the one this follows.
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
//! What keeps this from refusing valid input is the resolution rule, which is
//! `purrdf_entail::entails::imports::unresolved_imports` and not a copy of it: an import is
//! resolved when it names a document already LOADED — the shapes document's own retrieval
//! IRI or base (including an in-document `@base`), or an `--import` document's — or when the
//! closure already HOLDS the ontology it names (`<X> a owl:Ontology`, or an ontology whose
//! `owl:versionIRI` is `<X>`). The first case is SHACL's own idiom: `sh:prefixes` collects
//! `sh:declare`s along `owl:imports*`, and a document routinely points that path at its own
//! IRI from a node that is no `owl:Ontology`. A shapes document that merges the W3C SHACL 1.2
//! vocabularies — `shnex.ttl` importing `sh:`, beside the `shacl.ttl` that declares it — is
//! therefore complete as written and needs no `--import`. The prepared-product packer the
//! WebAssembly and C-ABI hosts call takes its verdict from the same rule, so every host
//! agrees on which shapes graphs are complete.
//!
//! Naming a pair also makes it MANDATORY that the pair is used: a pair the closure never
//! reaches is refused, because it would be read and never used.
//!
//! # `--shapes-graph` lives here too, for the same reason
//!
//! [`resolve_shapes_graph`] is the second thing `validate --shapes` and `shacl pack` must
//! agree on: both read a `--shapes-graph IRI` argument and both have to resolve it against
//! the SAME base their shapes document parses under, or a product packed with one answer and
//! a document validated with the other would expose `$shapesGraph` under two different IRIs
//! for what is supposed to be one shapes graph. Living beside [`fold_shapes_imports`] is what
//! keeps that agreement from being re-derived per caller the way the import fold used to be.

use std::collections::{BTreeSet, VecDeque};
use std::sync::Arc;

use purrdf_core::RdfDataset;
use purrdf_entail::ImportMap;
use purrdf_entail::entails::imports::imported_iris;
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

/// Fold the shapes graph's transitive `owl:imports` closure in from the `--import IRI=FILE`
/// table.
///
/// `imports` is the raw `--import IRI=FILE` table exactly as the operator wrote it — this
/// function is agnostic to which command line it came from, which is what lets
/// `validate --shapes` and `shacl pack` share it rather than each open-coding their own
/// walk. An import neither a pair nor the closure itself resolves is refused (exit 1), named
/// together with the `--import IRI=FILE` remedy and, for a document that imports its own
/// IRI, `base_flag` — the flag that sets the shapes document's base on the calling command
/// (`--shapes-base` for `validate`, `--base` for `shacl pack`); see the [module documentation](self) for the
/// rule and why there is no warn-and-continue path.
pub(crate) fn fold_shapes_imports(
    root: ShapesDocument,
    imports: &[String],
    base_flag: &str,
) -> Result<ShapesDocument, CliError> {
    let pairs = resolve_shapes_import_pairs(imports)?;
    let direct = imported_iris(&root.dataset);
    if direct.is_empty() {
        if let Some(pair) = pairs.first() {
            return Err(CliError::Usage(format!(
                "--import {spec}: the shapes graph has no owl:imports at all, so this \
                 document would be read and never used. Remove the pair, or import <{iri}> \
                 from the shapes graph",
                spec = pair.spec,
                iri = pair.iri
            )));
        }
        return Ok(root);
    }

    // Breadth-first to a FIXPOINT over the import graph, each named document read once. Two
    // properties are inherited from `purrdf_entail::entails::imports`, and both matter: an
    // imported document's OWN imports are followed, and a CYCLE terminates rather than
    // looping — OWL 2 §3.4 defines the closure as the transitive one and explicitly permits
    // `A` to import `B` to import `A`, so refusing a cycle would refuse an ontology the
    // specification allows. An import no pair names is not refused HERE: whether it is
    // missing depends on the whole closure, since the graph may already hold the ontology it
    // names, and that is decided below by the one rule every host shares.
    let mut queue: VecDeque<String> = direct.into_iter().collect();
    let mut requested: BTreeSet<String> = BTreeSet::new();
    let mut documents: Vec<ShapesDocument> = Vec::new();
    let mut map = ImportMap::new();
    for iri in &root.loaded {
        map.declare_loaded(iri.clone());
    }
    while let Some(iri) = queue.pop_front() {
        if !requested.insert(iri.clone()) {
            continue;
        }
        let Some(pair) = pairs.iter().find(|pair| pair.iri == iri) else {
            continue;
        };
        // The imported document parses under the ONTOLOGY IRI as its base, which is the
        // per-document base an `owl:imports` names — not the root's base and not `--base`.
        let document = read_shapes_document(
            pair.path,
            pair.format,
            Some(pair.iri),
            &format!("--import {iri}"),
        )?;
        queue.extend(imported_iris(&document.dataset));
        for loaded in &document.loaded {
            map.declare_loaded(loaded.clone());
        }
        map.insert(iri, Arc::clone(&document.dataset));
        documents.push(document);
    }

    // The refusal, naming EVERY missing import at once so one re-run can fix them all. An
    // import is missing when no pair names it, it names no document already loaded (the
    // shapes document's own base or `@base`, or an imported document's), and the closure
    // does not already hold the ontology it names (`<X> a owl:Ontology`, or an ontology
    // whose `owl:versionIRI` is `<X>`) — `purrdf_entail::entails::imports::unresolved_imports` is that rule, and the
    // prepared-product packer and `entails` take their verdicts from it too.
    let unresolved = map.unresolved_imports(&root.dataset);
    if !unresolved.is_empty() {
        let named: Vec<String> = unresolved.iter().map(|iri| format!("<{iri}>")).collect();
        let remedy: Vec<String> = unresolved
            .iter()
            .map(|iri| format!("`--import {iri}=FILE`"))
            .collect();
        return Err(CliError::Runtime(format!(
            "the shapes graph's owl:imports closure names {named}, which {verb} not in the \
             shapes graph and no --import pair resolves. PurRDF fetches nothing the operator \
             did not name, and going on without an imported document would use a different, \
             smaller shapes graph than the one named. Pass {remedy} to fold {it} in, or merge \
             the imported ontology into the shapes document. If the shapes document IS \
             {one_of}, read it under that IRI with {self_remedy}",
            named = named.join(", "),
            verb = if unresolved.len() == 1 { "is" } else { "are" },
            remedy = remedy.join(" "),
            it = if unresolved.len() == 1 { "it" } else { "them" },
            one_of = if unresolved.len() == 1 {
                named.join("")
            } else {
                format!("one of {}", named.join(", "))
            },
            self_remedy = if unresolved.len() == 1 {
                format!("`{base_flag} {}`", unresolved[0])
            } else {
                format!("`{base_flag} IRI`")
            },
        )));
    }
    if documents.is_empty() && pairs.is_empty() {
        // Every import is already in the graph: there is nothing to fold.
        return Ok(root);
    }

    // A pair the closure never reached, quoted back exactly as the operator wrote it. A
    // USAGE error (exit 2) that nevertheless needs the closure walked to detect: the fault
    // is in the command line, and only its DISCOVERY needed the documents.
    if let Some(pair) = pairs.iter().find(|pair| !requested.contains(pair.iri)) {
        return Err(CliError::Usage(format!(
            "--import {spec}: the shapes graph's import closure never reaches <{iri}>, so \
             this document would be read and never used. Remove the pair, or import the IRI \
             from the shapes graph",
            spec = pair.spec,
            iri = pair.iri
        )));
    }

    // `union` standardizes blank nodes apart per source document and dedupes, which is what
    // keeps two documents' independently-labelled property shapes from colliding.
    let merged = {
        let graphs: Vec<&RdfDataset> = std::iter::once(root.dataset.as_ref())
            .chain(documents.iter().map(|doc| doc.dataset.as_ref()))
            .collect();
        Arc::new(RdfDataset::union(&graphs))
    };
    // The root's prefixes come FIRST so its declarations win a collision: it is the document
    // the operator named, and `from_dataset_with_prefixes` takes the first match.
    let mut prefixes = root.prefixes;
    let mut loaded = root.loaded;
    for document in documents {
        prefixes.extend(document.prefixes);
        for iri in document.loaded {
            if !loaded.contains(&iri) {
                loaded.push(iri);
            }
        }
    }
    Ok(ShapesDocument {
        dataset: merged,
        prefixes,
        loaded,
    })
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
