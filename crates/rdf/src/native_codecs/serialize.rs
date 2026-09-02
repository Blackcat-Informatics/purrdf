// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Frozen [`RdfDataset`](crate::RdfDataset) IR → native RDF text egress.
//!
//! Builds a first-party [`SerGraph`](super::ser_model::SerGraph) from the frozen IR —
//! interning every IR term into the graph's term table, materializing literal datatypes
//! and quoted-triple reifier bindings — then dispatches to the matching first-party
//! serializer: the Turtle / TriG / N-Triples / N-Quads serializers in
//! [`ser_model`](super::ser_model), and the in-repo [`rdfxml`](super::rdfxml) codec for
//! RDF/XML. The graph layout mirrors exactly what the parser produces, so parse and
//! serialize are inverses.
//!
//! The [`SerializeGraph`] filter matches `oxigraph/backend.rs:333-391` exactly:
//! `DefaultGraph` emits the default-graph quads plus ALL statement rows
//! (reifiers/annotations); `Named(g)` emits only that graph's quads as triples and NO
//! statement rows; `Dataset` keeps graph names for TriG/N-Quads but falls back to the
//! default graph for Turtle/N-Triples/RDF-XML.
//!
//! # One seam, four spellings
//!
//! [`serialize_dataset_with`] is the only function here that serializes anything. It
//! takes all four axes — target format, document base, graph selection, and
//! [`StatementLayer`] — and the other public entry points are one-expression delegations
//! that fix some of them:
//!
//! | spelling | format | base | selection | statement layer |
//! |---|---|---|---|---|
//! | [`serialize_dataset`] | media type | none | caller's | `Emit` |
//! | [`serialize_dataset_with_jsonld_options`] | media type | none | caller's | `Emit` |
//! | [`serialize_dataset_to_format`] | caller's | caller's | `Dataset` | `PerFormatCapability` |
//! | [`serialize_dataset_to_format_with_jsonld_options`] | caller's | caller's | `Dataset` | `PerFormatCapability` |
//!
//! Read down the two right-hand columns and the gap this family used to have is visible
//! as a hole in the table: no spelling combined a BASE with a caller-chosen selection or
//! statement layer, so a caller wanting a base on a store dump had to take
//! `SerializeGraph::Dataset` and the projection contract with it — silently trading its
//! RDF 1.2 reifier and annotation rows for a base declaration. `serialize_dataset_with`
//! is that missing row, and the four above are now expressed through it rather than
//! beside it, so they cannot drift from it.
//!
//! `serialize_dataset_base_only` used to be a fifth spelling. It was the `Project`
//! statement-layer axis wearing a function name, so it is gone: pass
//! [`StatementLayer::Project`] instead.

use std::collections::HashMap;
use std::io::Write;

use super::jsonld::JsonLdSerializeOptions;
use super::media_type::{NativeRdfFormat, classify};
use super::ser_model::{SerAnnotationRow, SerGraph, SerReifierRow, SerTerm, SerTermKind};
use crate::dataset_view::ViewTermId;
use crate::ir::TermRef;
use crate::{
    DatasetView, FastHasher, FastMap, RdfDiagnostic, RdfTextDirection, SerializeGraph, TermValue,
};
use purrdf_core::blank_label::{LabelAlphabet, encode_blank_label};
use purrdf_iri::BaseIri;

/// The blank-node label alphabet the TARGET format's codec can legally emit —
/// the egress contract applied at the [`SerGraph`] ingress so no codec can write
/// a label that is illegal in ITS syntax. Exactly one alphabet per target
/// syntax; a label outside it — or any label carrying a non-default scope — is
/// enveloped, never refused.
///
/// - The line/Turtle-family codecs write `_:{label}` tokens, so their labels
///   must satisfy the exact W3C Turtle/SPARQL `BLANK_NODE_LABEL` production.
/// - RDF/XML writes labels into `rdf:nodeID` attributes, whose value the RDF/XML
///   grammar constrains to an XML `NCName`.
/// - TriX writes the label as `<id>` element text, so the constraint is what XML
///   1.0 character data can carry unchanged: XML has NO representation for a C0
///   control (not even a character reference), and XML whitespace normalization
///   plus element-text trimming means whitespace cannot carry identity either.
/// - HexTuples, JSON-LD and YAML-LD type their blank identifiers as
///   `BLANK_NODE_LABEL`-shaped `_:` names in their own specifications, not as
///   free text, so they take the same alphabet as the Turtle family.
const fn blank_label_alphabet(format: NativeRdfFormat) -> LabelAlphabet {
    match format {
        NativeRdfFormat::Turtle
        | NativeRdfFormat::TriG
        | NativeRdfFormat::NTriples
        | NativeRdfFormat::NQuads
        | NativeRdfFormat::HexTuples
        | NativeRdfFormat::JsonLd
        | NativeRdfFormat::YamlLd => LabelAlphabet::BlankNodeLabel,
        NativeRdfFormat::RdfXml => LabelAlphabet::NcName,
        NativeRdfFormat::TriX => LabelAlphabet::XmlText,
    }
}

/// The `xsd:string` datatype IRI: a literal of this datatype with no language is a
/// plain literal and is emitted WITHOUT an explicit `^^<…>`, so it round-trips back to
/// the same plain form (matching the purrdf-gts native projection).
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

/// Serialize a frozen [`RdfDataset`](crate::RdfDataset) to RDF text of `media_type`, honoring the
/// [`SerializeGraph`] selection. Returns the serialized bytes.
///
/// The FIDELITY spelling of [`serialize_dataset_with`]: the full RDF 1.2 statement layer
/// (reifier bindings + annotations) is emitted, and a format with no surface for it fails
/// closed rather than dropping rows. No document base is applied — this entry point takes
/// none, so IRIs are absolute.
///
/// To project the statement layer away and receive the dropped-row count instead, or to
/// write under a document base, call [`serialize_dataset_with`] with the
/// [`StatementLayer`] and base you want. It is the same code path.
///
/// # Termination
///
/// `D: DatasetView` is a public trait. The `SerGraph` lowering this function
/// drives — `build_ser_graph`'s term interner — resolves a quoted-triple term's
/// components through `DatasetView::resolve` with no depth bound and no visited
/// set, so it terminates only if `dataset` does. That is [`DatasetView`]'s own
/// contract (see its `# Termination` doc), not something this function — or
/// `ensure_terms_terminate`, which polices a concrete GTS `Graph` and
/// structurally cannot police an arbitrary trait impl — can check on `dataset`'s
/// behalf. Every in-repo `DatasetView` satisfies it; a caller supplying a
/// third-party one owes it directly.
pub fn serialize_dataset<D: DatasetView>(
    dataset: &D,
    media_type: &str,
    selection: SerializeGraph<'_>,
) -> Result<Vec<u8>, RdfDiagnostic> {
    serialize_dataset_with(
        dataset,
        classify(media_type)?,
        None,
        &SerializeOptions {
            selection,
            statement_layer: StatementLayer::Emit,
            jsonld_options: None,
        },
    )
    .map(|outcome| outcome.bytes)
}

/// Which RDF 1.2 statement-layer rows (reifier bindings + annotation triples) the emitted
/// document carries.
///
/// This is a CALLER decision, not a derived one, because two legitimate answers exist for
/// the same target format. RDF/XML's emitter really can render a reifier binding — as
/// `rdf:parseType="Triple"` — so a fidelity-first caller (a language binding dumping a
/// store) wants those rows emitted, while the transcode loss contract deliberately
/// PROJECTS them away for the same format and records the count. Deriving this from
/// `carries_star()` would silently pick one of those two for everybody.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementLayer {
    /// Emit the statement layer whatever the target format is.
    ///
    /// A format with no surface for it FAILS CLOSED rather than dropping rows silently:
    /// TriX and HexTuples have no triple-term surface at all and refuse. RDF/XML renders
    /// it. This is the fidelity answer — nothing is lost, or the caller is told.
    Emit,
    /// Drop the statement layer and REPORT the dropped-row count in
    /// [`SerializeOutcome::statement_rows_dropped`].
    ///
    /// The projection answer: the loss is declared, counted, and the caller's to record
    /// on the loss ledger. Never a silent drop.
    Project,
    /// Let the format registry choose: [`Emit`](Self::Emit) for a
    /// [`carries_star`](NativeRdfFormat::carries_star) format,
    /// [`Project`](Self::Project) for the rest.
    ///
    /// This is the transcode contract every `*_to_format` spelling applies.
    PerFormatCapability,
}

/// Policy options for [`serialize_dataset_with`] — the egress mirror of
/// [`ParseOptions`](super::ParseOptions).
///
/// Every field is public and there is deliberately NO `Default`. A defaulted overload
/// beside a fully-specified original is exactly how this crate's serialize leg came to
/// accept a base and discard it: no call site had to mention the axis, so no call site
/// was ever seen to be missing it. Naming all three at every construction keeps each an
/// answer somebody gave.
///
/// The two axes every serialization has regardless — the target format and the document
/// base — stay POSITIONAL on [`serialize_dataset_with`], exactly as `media_type` and
/// `base_iri` stay positional on
/// [`parse_dataset_with`](super::parse_dataset_with).
#[derive(Debug, Clone, Copy)]
pub struct SerializeOptions<'a> {
    /// Which graph(s) to emit.
    pub selection: SerializeGraph<'a>,
    /// Whether the RDF 1.2 statement layer is emitted, projected away, or decided by the
    /// format registry.
    pub statement_layer: StatementLayer,
    /// JSON-LD / YAML-LD serialization configuration. `Some` for any other format is a
    /// hard failure rather than silently ignored caller policy.
    pub jsonld_options: Option<&'a JsonLdSerializeOptions>,
}

/// Serialize a frozen dataset under an explicit target format, document base, and policy —
/// **the one serialization seam in this crate**.
///
/// Every other public spelling on this module (`serialize_dataset`,
/// `serialize_dataset_with_jsonld_options`, `serialize_dataset_to_format`,
/// `serialize_dataset_to_format_with_jsonld_options`) is a one-expression delegation to
/// this function that fixes some of its axes. There is no second code path: a behaviour
/// that is true here is true through all of them, which is the property the split family
/// did not have. It is also why this is the only entry point that can express the
/// combination the others could not — a document base TOGETHER WITH a graph selection and
/// the RDF 1.2 statement layer.
///
/// # The four axes
///
/// * `format` — the target syntax. Positional, because there is no serialization without
///   one.
/// * `base_iri` — the document base the output is written under. Positional, and the
///   egress mirror of [`parse_dataset_with`](super::parse_dataset_with)'s. Validated for
///   EVERY format (a non-absolute base is a hard failure with the shared
///   [`IriError::diagnostic_code`](purrdf_iri::IriError::diagnostic_code)) and APPLIED
///   only where the registry's [`emits_base`](NativeRdfFormat::emits_base) column says
///   the syntax can express one, in one private decision point (`egress_base`) this module
///   keeps as that rule's only owner.
/// * `options.selection` — which graph(s) to emit.
/// * `options.statement_layer` — see [`StatementLayer`].
///
/// # Errors
///
/// Returns a diagnostic when the base is not absolute, when JSON-LD options are supplied
/// for a non-JSON-LD format, or when the target codec cannot represent what it was asked
/// to emit (a triple term for TriX/HexTuples, a named graph for a single-graph syntax).
pub fn serialize_dataset_with<D: DatasetView>(
    dataset: &D,
    format: NativeRdfFormat,
    base_iri: Option<&str>,
    options: &SerializeOptions<'_>,
) -> Result<SerializeOutcome, RdfDiagnostic> {
    if options.jsonld_options.is_some()
        && !matches!(format, NativeRdfFormat::JsonLd | NativeRdfFormat::YamlLd)
    {
        return Err(jsonld_options_unused(format));
    }

    let include_statement_layer = match options.statement_layer {
        StatementLayer::Emit => true,
        StatementLayer::Project => false,
        StatementLayer::PerFormatCapability => format.carries_star(),
    };

    // A base direction is dropped independently of the star layer: RDF/XML is
    // star-incapable yet carries direction, while TriX / HexTuples carry neither. Only
    // the two direction-less formats pay the scan; every other target skips it entirely.
    let directional_literals_dropped = if format.carries_direction() {
        0
    } else {
        count_directional_object_literals(dataset)
    };

    // Independent of BOTH the star layer and the direction surface: a caller who asked
    // for the WHOLE dataset and named a target with no named-graph construct gets a
    // flattening, and the flattening DROPS every graph-scoped row rather than folding it
    // into the default graph. `DefaultGraph` and `Named` are not flattenings — the
    // caller wrote the subset they wanted, and a row outside a selection the caller
    // spelled out is not a loss to report.
    let flattened =
        matches!(options.selection, SerializeGraph::Dataset) && !format.supports_datasets();
    let named_graph_rows_dropped = if flattened {
        count_named_graph_rows(dataset)
    } else {
        0
    };

    let graph = build_ser_graph(
        dataset,
        format,
        options.selection,
        include_statement_layer,
        egress_base(format, base_iri)?,
    )?;

    let text = match options.jsonld_options {
        // Dispatch to the format's codec (the single `codec_for` chokepoint): the
        // line/Turtle family walks the shared `ser_model` writers, and RDF/XML, TriX and
        // HexTuples walk the SAME `SerGraph` through their in-repo emitters.
        None => super::codec::codec_for(format).serialize(&graph)?,
        Some(configured) if format == NativeRdfFormat::JsonLd => {
            super::jsonld::serialize_ser_graph_with_options(&graph, configured)?
        }
        Some(configured) => {
            super::jsonld::serialize_ser_graph_to_yamlld_with_options(&graph, configured)?
        }
    };

    // A `Named` selection emits NO statement rows whatever the format can carry (the
    // filter in `build_ser_graph`), so rows the caller asked to emit still did not reach
    // the document and the count must say so. Reporting zero because the FORMAT could
    // have carried them would be the silent drop this accounting exists to prevent.
    //
    // Attributed by CAUSE, so the two counts partition the loss and no row is charged
    // twice: a graph-scoped statement row that a flattening discarded is already in
    // `named_graph_rows_dropped`, so only the DEFAULT-graph rows are charged to the
    // statement-layer policy here. Without a flattening every statement row that did not
    // reach the document is charged here, exactly as before.
    let reached_the_document =
        include_statement_layer && !matches!(options.selection, SerializeGraph::Named(_));
    let statement_rows_dropped = if reached_the_document {
        0
    } else if flattened {
        dataset.reifier_quads().filter(|q| q.g.is_none()).count()
            + dataset.annotation_quads().filter(|q| q.g.is_none()).count()
    } else {
        dataset.reifier_quads().count() + dataset.annotation_quads().count()
    };

    Ok(SerializeOutcome {
        bytes: text.into_bytes(),
        statement_rows_dropped,
        directional_literals_dropped,
        named_graph_rows_dropped,
    })
}

/// Resolve the base `format` will actually be EMITTED under — the single place the egress
/// base decision is taken, and the exact mirror of the ingress decision in
/// [`base_scope_for`](super::parse::base_scope_for).
///
/// Two steps, in this order and always both:
///
/// 1. **Validate.** A supplied base must be an absolute IRI whatever the target format
///    is. The condition already has a diagnostic code —
///    [`IriError::diagnostic_code`](purrdf_iri::IriError::diagnostic_code) — and it is
///    reused verbatim rather than respelled, so one identity covers the parse leg and the
///    serialize leg. A malformed base is a hard failure even for a format that would not
///    have applied it: the caller is told their base is wrong rather than having the
///    mistake absorbed.
/// 2. **Apply, if the registry says the syntax can express one.** `emits_base()` is the
///    egress mirror of `admits_relative_iri()`: a syntax that can write a base directive
///    gets the base; one that cannot gets `None` and emits absolute IRIs. That is not the
///    parameter being swallowed — it is the only answer those grammars admit, decided
///    once from the registry rather than per codec.
fn egress_base(
    format: NativeRdfFormat,
    base_iri: Option<&str>,
) -> Result<Option<BaseIri>, RdfDiagnostic> {
    let base = base_iri
        .map(|base| {
            BaseIri::parse(base).map_err(|error| {
                RdfDiagnostic::error(
                    error.diagnostic_code(),
                    format!("serialization base IRI `{base}` is unusable: {error}"),
                )
            })
        })
        .transpose()?;
    Ok(base.filter(|_| format.emits_base()))
}

/// Serialize JSON-LD or YAML-LD through the generic media-type surface under an
/// explicit configured mode.
///
/// Supplying JSON-LD options for another syntax is a hard error instead of silently
/// ignoring caller policy. Existing [`serialize_dataset`] calls retain their frozen
/// expanded compatibility behavior.
pub fn serialize_dataset_with_jsonld_options<D: DatasetView>(
    dataset: &D,
    media_type: &str,
    selection: SerializeGraph<'_>,
    options: &JsonLdSerializeOptions,
) -> Result<Vec<u8>, RdfDiagnostic> {
    serialize_dataset_with(
        dataset,
        classify(media_type)?,
        None,
        &SerializeOptions {
            selection,
            statement_layer: StatementLayer::Emit,
            jsonld_options: Some(options),
        },
    )
    .map(|outcome| outcome.bytes)
}

/// Serialize a frozen [`RdfDataset`](crate::RdfDataset) into the given writer.
///
/// `base_iri` is the egress base and is honored exactly as on every other seam: a format
/// whose registry row can express a base emits it and relativizes against it; one that
/// cannot emits absolute IRIs. A base that is not an absolute IRI is a hard failure here,
/// not a silent fall back to absolute output.
pub(crate) fn serialize_into<D: DatasetView, W: Write>(
    dataset: &D,
    media_type: &str,
    selection: SerializeGraph<'_>,
    base_iri: Option<&str>,
    mut output: W,
) -> Result<(), RdfDiagnostic> {
    let bytes = serialize_dataset_with(
        dataset,
        classify(media_type)?,
        base_iri,
        &SerializeOptions {
            selection,
            statement_layer: StatementLayer::Emit,
            jsonld_options: None,
        },
    )?
    .bytes;
    output
        .write_all(&bytes)
        .map_err(|e| RdfDiagnostic::error("native-codec-write", e.to_string()))
}

/// Outcome of serializing an [`RdfDataset`](crate::RdfDataset) to a concrete RDF format through the
/// native codecs (universal transcoder helper, ported onto the native path).
#[derive(Debug, Clone)]
pub struct SerializeOutcome {
    /// The serialized document bytes.
    pub bytes: Vec<u8>,
    /// The number of RDF-1.2 statement-layer rows (reifier bindings + annotation
    /// triples) dropped because the target format does not carry the star layer in
    /// the transcode contract. Zero for star-capable formats.
    ///
    /// Rows dropped for the OTHER reason — they were scoped to a named graph a
    /// single-graph target cannot carry — are counted by
    /// [`Self::named_graph_rows_dropped`] instead, never here and never twice: the
    /// two counts partition the dropped rows by CAUSE, so their sum is the total.
    pub statement_rows_dropped: usize,
    /// The number of base-quad object literals whose RDF-1.2 base direction was
    /// dropped because the target format has no direction surface (TriX / HexTuples
    /// keep the language tag but cannot carry `--ltr` / `--rtl`). Zero for every
    /// direction-capable format. Recorded as declared loss — never a silent drop.
    pub directional_literals_dropped: usize,
    /// The number of rows the single-graph flattening dropped because the target
    /// format has no named-graph construct: base quads asserted in a named graph,
    /// plus the RDF-1.2 statement-layer rows (reifier bindings + annotation
    /// triples) scoped to one. Zero for every dataset-capable format (TriG,
    /// N-Quads, TriX, HexTuples, JSON-LD, YAML-LD).
    ///
    /// The rows are DROPPED, not folded into the default graph — see
    /// `purrdf_core::loss`'s `named-graph-dropped` contract note. Without this
    /// count the flattening was a silent loss: `statement_rows_dropped` is about
    /// the STAR layer and says nothing about graph scoping, so a star-capable
    /// single-graph target (Turtle, N-Triples) reported zero loss while discarding
    /// every named graph it was handed.
    pub named_graph_rows_dropped: usize,
}

/// Serialize the frozen IR to a concrete [`NativeRdfFormat`], returning the bytes and
/// the count of RDF-1.2 statement-layer rows dropped because the target format does
/// not carry the star layer (the projection doctrine).
///
/// Star-capable formats (Turtle, N-Triples, N-Quads, TriG) emit the full RDF-1.2
/// statement layer and report `statement_rows_dropped = 0`. Star-incapable formats
/// (RDF/XML) emit only the base quads and report the dropped statement-row count —
/// the caller records this as declared loss against the loss ledger.
///
/// # `base_iri` is the EGRESS base, and the registry decides who applies it
///
/// `base_iri` is the document base the output is written under. Whether it is applied is
/// the [`emits_base`](NativeRdfFormat::emits_base) column's decision, made once for the
/// whole workspace, and it is the exact mirror of the ingress rule keyed on
/// [`admits_relative_iri`](NativeRdfFormat::admits_relative_iri):
///
/// * a syntax that CAN express a base (Turtle, TriG, RDF/XML, JSON-LD, YAML-LD) emits the
///   base directive and relativizes its IRIs against it;
/// * a syntax that CANNOT (N-Triples, N-Quads, TriX, HexTuples) never applies it and
///   emits absolute IRIs — exactly as, on ingress, those grammars never apply a base to a
///   relative reference.
///
/// The second case is NOT the parameter being swallowed. The base is still read, still
/// validated (a non-absolute one is a hard failure, code
/// [`IriError::diagnostic_code`](purrdf_iri::IriError::diagnostic_code)), and still
/// answered — with the only spelling those grammars admit. That is why `--base` paired
/// with `--to ntriples` succeeds and emits absolute IRIs rather than erroring: one flag
/// serves both legs, and the format's own capability decides what each leg does with it.
///
/// Graph selection follows [`SerializeGraph::Dataset`]: dataset-capable formats
/// (N-Quads, TriG) emit all named graphs; the single-graph syntaxes (Turtle,
/// N-Triples, RDF/XML) DROP every named graph and emit the default graph alone,
/// reporting what they discarded as
/// [`SerializeOutcome::named_graph_rows_dropped`].
///
/// This is the TRANSCODE spelling of [`serialize_dataset_with`], and it fixes two of that
/// function's axes: [`SerializeGraph::Dataset`] and
/// [`StatementLayer::PerFormatCapability`]. A caller that needs another graph selection,
/// or that wants the statement layer emitted for a star-incapable format that can
/// nonetheless render it (RDF/XML), calls [`serialize_dataset_with`] directly rather than
/// trading one for the other here.
pub fn serialize_dataset_to_format<D: DatasetView>(
    dataset: &D,
    format: NativeRdfFormat,
    base_iri: Option<&str>,
) -> Result<SerializeOutcome, RdfDiagnostic> {
    serialize_dataset_with(
        dataset,
        format,
        base_iri,
        &SerializeOptions {
            selection: SerializeGraph::Dataset,
            statement_layer: StatementLayer::PerFormatCapability,
            jsonld_options: None,
        },
    )
}

/// Serialize through the generic format surface with explicit JSON-LD/YAML-LD
/// configuration.
///
/// The function accepts only the two JSON-LD family formats and reports zero loss for
/// their RDF 1.2-capable carrier. Passing another format is a stable hard failure.
///
/// `base_iri` is the egress base, honored exactly as in [`serialize_dataset_to_format`].
/// Both formats' registry rows set `emits_base`, so the base reaches the emitted
/// `@context` as `@base` and document-position `@id`s are compacted against it — through
/// the JSON-LD 1.1 §4.1.4 candidate-selection layer the context compiler already owns. A
/// base already declared by the caller's own context WINS, matching the ingress
/// precedence where an in-document `@context.@base` overrides the caller's.
pub fn serialize_dataset_to_format_with_jsonld_options<D: DatasetView>(
    dataset: &D,
    format: NativeRdfFormat,
    base_iri: Option<&str>,
    options: &JsonLdSerializeOptions,
) -> Result<SerializeOutcome, RdfDiagnostic> {
    serialize_dataset_with(
        dataset,
        format,
        base_iri,
        &SerializeOptions {
            selection: SerializeGraph::Dataset,
            // Both JSON-LD-family rows are `carries_star`, so this resolves to `Emit` and
            // the reported drop counts stay zero — stated through the registry rather
            // than hardcoded, so a capability change cannot leave the count lying. Both
            // are dataset-capable too, so nothing is scoped away and
            // `named_graph_rows_dropped` is zero for the same structural reason.
            statement_layer: StatementLayer::PerFormatCapability,
            jsonld_options: Some(options),
        },
    )
}

fn jsonld_options_unused(format: NativeRdfFormat) -> RdfDiagnostic {
    RdfDiagnostic::error(
        "jsonld-options-unused",
        format!(
            "JSON-LD serialization options cannot be used with `{}`",
            format.media_type()
        ),
    )
}

/// Count the base-quad OBJECT literals whose resolved term carries an RDF-1.2 base
/// direction. Used to record declared loss when serializing to a format with no
/// direction surface (TriX / HexTuples) — the drop is the realized count the caller
/// attaches to the loss ledger, never a silent loss.
fn count_directional_object_literals<D: DatasetView>(dataset: &D) -> usize {
    dataset
        .quads()
        .filter(|q| {
            matches!(
                dataset.resolve(q.o),
                TermRef::Literal {
                    direction: Some(_),
                    ..
                }
            )
        })
        .count()
}

/// Count every row a single-graph target's flattening discards: base quads asserted in
/// a named graph, plus the RDF-1.2 statement-layer rows (reifier bindings + annotation
/// triples) scoped to one.
///
/// This is the realized twin of the `named-graph-dropped` transcode contract, and it is
/// counted for EVERY single-graph target — including the star-capable ones (Turtle,
/// N-Triples), whose graph-scoped statement rows [`build_ser_graph`]'s
/// `default_graph_only` pass also drops and which no other counter sees.
fn count_named_graph_rows<D: DatasetView>(dataset: &D) -> usize {
    dataset.quads().filter(|q| q.g.is_some()).count()
        + dataset.reifier_quads().filter(|q| q.g.is_some()).count()
        + dataset.annotation_quads().filter(|q| q.g.is_some()).count()
}

/// Build the first-party [`SerGraph`] from the frozen IR, applying the
/// [`SerializeGraph`] filter while populating the quad and statement-row tables.
///
/// `pub(crate)` so the JSON-LD / YAML-LD codec ([`super::jsonld`]) can build the same
/// first-party graph shape it walks (a dataset-capable `format` such as
/// [`NativeRdfFormat::NQuads`] preserves named graphs).
///
/// `base` is stored verbatim on the graph and is what every writer relativizes against.
/// It arrives already decided by [`egress_base`], which is where the registry's
/// `emits_base()` column is consulted — deliberately NOT here, because `format` on this
/// function is the graph-SHAPE selector (the JSON-LD codec passes
/// [`NativeRdfFormat::NQuads`] to keep named graphs) and is not always the format the
/// document is written as. Gating on it here would silently drop a JSON-LD base.
/// A format with `emits_base: false` therefore reaches its writer with `None` and emits
/// absolute IRIs, structurally rather than by a per-codec convention.
pub(crate) fn build_ser_graph<D: DatasetView>(
    dataset: &D,
    format: NativeRdfFormat,
    selection: SerializeGraph<'_>,
    include_statement_layer: bool,
    base: Option<BaseIri>,
) -> Result<SerGraph, RdfDiagnostic> {
    let mut interner =
        SerGraphInterner::with_capacity(dataset.term_count(), blank_label_alphabet(format));

    // Which quad rows to emit, and whether the statement layer (reifiers/annotations)
    // participates — matching the oxigraph backend's filter exactly.
    let mut graph = SerGraph {
        terms: Vec::new(),
        quads: Vec::with_capacity(dataset.len_hint().unwrap_or(0)),
        reifiers: Vec::new(),
        annotations: Vec::new(),
        base,
    };

    match selection {
        // TriG / N-Quads keep graph names; the single-graph syntaxes fall back to the
        // default graph (their `to_*` serializers reject named-graph quads).
        SerializeGraph::Dataset if format.supports_datasets() => {
            for quad in dataset.quads() {
                let s = interner.intern(dataset, quad.s)?;
                let p = interner.intern(dataset, quad.p)?;
                let o = interner.intern(dataset, quad.o)?;
                let g = match quad.g {
                    Some(g) => Some(interner.intern(dataset, g)?),
                    None => None,
                };
                graph.quads.push((s, p, o, g));
            }
            if include_statement_layer {
                push_statement_rows(&mut interner, dataset, &mut graph, false)?;
            }
        }
        SerializeGraph::Dataset | SerializeGraph::DefaultGraph => {
            for quad in dataset.quads() {
                if quad.g.is_some() {
                    continue;
                }
                let s = interner.intern(dataset, quad.s)?;
                let p = interner.intern(dataset, quad.p)?;
                let o = interner.intern(dataset, quad.o)?;
                graph.quads.push((s, p, o, None));
            }
            // A single-graph (flattened) projection drops named-graph QUADS above, so
            // it must likewise drop graph-scoped STATEMENT ROWS — otherwise the
            // single-graph serializers' `ensure_default_graph_projection` guard rejects
            // a graph-scoped reifier/annotation that has no home in the default graph.
            if include_statement_layer {
                push_statement_rows(&mut interner, dataset, &mut graph, true)?;
            }
        }
        SerializeGraph::Named(name) => {
            let target = dataset.term_id_by_value(name);
            for quad in dataset.quads() {
                if quad.g != target {
                    continue;
                }
                let s = interner.intern(dataset, quad.s)?;
                let p = interner.intern(dataset, quad.p)?;
                let o = interner.intern(dataset, quad.o)?;
                graph.quads.push((s, p, o, None));
            }
            // A named-graph selection emits NO statement rows (oxigraph parity).
        }
    }

    graph.terms = std::mem::take(&mut interner.terms);
    // The interner rows already carry the serialization row-array's graph slot (`None`
    // = default graph): a reifier/annotation declared inside a `GRAPH g { … }` block
    // keeps `g` so the emitted N-Quads/TriG round-trips it.
    graph.reifiers = std::mem::take(&mut interner.reifiers);
    // Annotations populated alongside the statement rows above.
    graph
        .annotations
        .extend(std::mem::take(&mut interner.annotations));
    // Impose a canonical, value-based row order so the emitted document is
    // byte-identical across `DatasetView` backends (whose term-table interning order —
    // and hence quad iteration order — differs) and independent of insertion order.
    graph.sort_canonical();
    Ok(graph)
}

/// Push the RDF 1.2 statement layer (reifier bindings + annotations) onto the graph,
/// interning their terms. The reifier bindings land in `interner.reifiers`; the
/// annotation triples in `interner.annotations`.
fn push_statement_rows<D: DatasetView>(
    interner: &mut SerGraphInterner<D::Id>,
    dataset: &D,
    _graph: &mut SerGraph,
    default_graph_only: bool,
) -> Result<(), RdfDiagnostic> {
    // `reifier_quads()` yields each side-table binding as a virtual quad
    // `(s = reifier, p = rdf:reifies, o = triple-term, g = graph)`. The `rdf:reifies`
    // predicate id (`q.p`) is the fixed virtual edge and is not materialized here — the
    // reifier row carries the resolved triple components directly.
    for q in dataset.reifier_quads() {
        // A flattened single-graph projection carries only the default graph, so a
        // graph-scoped binding is dropped exactly as its named-graph quads are.
        if default_graph_only && q.g.is_some() {
            continue;
        }
        let reifier_id = interner.intern(dataset, q.s)?;
        let (s, p, o) = interner.intern_triple_components(dataset, q.o)?;
        let g = q.g.map(|g| interner.intern(dataset, g)).transpose()?;
        interner.reifiers.push((reifier_id, (s, p, o), g));
    }
    // `annotation_quads()` yields each annotation as `(s = reifier, p = predicate,
    // o = object, g = graph)`.
    for q in dataset.annotation_quads() {
        if default_graph_only && q.g.is_some() {
            continue;
        }
        let r = interner.intern(dataset, q.s)?;
        let p = interner.intern(dataset, q.p)?;
        let o = interner.intern(dataset, q.o)?;
        let g = q.g.map(|g| interner.intern(dataset, g)).transpose()?;
        interner.annotations.push((r, p, o, g));
    }
    Ok(())
}

/// Builds the first-party term table from the frozen IR, deduplicating terms by value
/// and materializing literal datatypes + quoted-triple reifier bindings.
struct SerGraphInterner<I: ViewTermId> {
    terms: Vec<SerTerm>,
    /// Reifier-id → `(s, p, o)` bindings. Carries both the statement-layer reifiers
    /// (a resource reifying a statement) and the self-reifier sentinels of inline
    /// quoted-triple terms (skipped by the N-Quads serializer).
    reifiers: Vec<SerReifierRow>,
    annotations: Vec<SerAnnotationRow>,
    /// Value → term-id memo so equal terms collapse to one term, matching the fold the
    /// reader produces.
    memo: HashMap<TermValue, usize>,
    /// IR-id → term-id memo probed BEFORE the value memo. `resolve(id)` is a pure
    /// function of the (frozen, immutable) view for the whole build, so an id seen
    /// once maps to the same value — and thus the same term-id — every time. Without
    /// this, every OCCURRENCE of a term (a predicate repeated on 10k quads, say) built
    /// an owned `TermValue` just to probe `memo`. Fixed-key hasher, lookup-only, never
    /// iterated: it cannot influence emitted order.
    id_memo: FastMap<I, usize>,
    /// The TARGET codec's blank-node label alphabet ([`blank_label_alphabet`]).
    /// Every blank node's `(label, scope)` pair is encoded into it at intern
    /// time, so no downstream emitter can write a label illegal in its syntax
    /// and no codec has to repeat the check.
    alphabet: LabelAlphabet,
}

impl<I: ViewTermId> SerGraphInterner<I> {
    fn with_capacity(term_count: usize, alphabet: LabelAlphabet) -> Self {
        Self {
            terms: Vec::with_capacity(term_count),
            reifiers: Vec::new(),
            annotations: Vec::new(),
            memo: HashMap::with_capacity(term_count),
            id_memo: FastMap::with_capacity_and_hasher(term_count, FastHasher::default()),
            alphabet,
        }
    }

    /// Intern an IR term id into the first-party term table, returning its index.
    fn intern<D: DatasetView<Id = I>>(
        &mut self,
        dataset: &D,
        id: D::Id,
    ) -> Result<usize, RdfDiagnostic> {
        // Repeat occurrences of an id are answered here without materializing the
        // term's value (see `id_memo`).
        if let Some(&idx) = self.id_memo.get(&id) {
            return Ok(idx);
        }
        let value = term_value(dataset, id);
        if let Some(&idx) = self.memo.get(&value) {
            self.id_memo.insert(id, idx);
            return Ok(idx);
        }
        let idx = match dataset.resolve(id) {
            TermRef::Iri(iri) => self.push_term(SerTerm {
                kind: SerTermKind::Iri,
                value: Some(iri.to_owned()),
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
            }),
            TermRef::Blank { label, scope } => {
                // Encode the `(label, scope)` pair into the TARGET format's
                // alphabet in ONE step. An unscoped label already legal there
                // passes through byte-identically; anything else becomes the
                // deterministic, injective envelope, so serialization is total
                // and blank-node co-reference survives exactly.
                let emitted = encode_blank_label(label, scope, self.alphabet).into_owned();
                self.push_term(SerTerm {
                    kind: SerTermKind::Bnode,
                    value: Some(emitted),
                    datatype: None,
                    lang: None,
                    direction: None,
                    reifier: None,
                })
            }
            TermRef::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => {
                // Borrowed twin of `iri_of`: the comparison and the intern both read
                // the IRI, neither keeps it, so no owned copy is needed here.
                let datatype_iri = iri_str_of(dataset, datatype)?;
                // A plain literal (xsd:string, no language) and a language-tagged
                // literal carry no explicit datatype term — the serializer defaults
                // them, so emitting one would change the round-trip text.
                let datatype_slot = if language.is_some() || datatype_iri == XSD_STRING {
                    None
                } else {
                    Some(self.intern_iri_string(datatype_iri))
                };
                self.push_term(SerTerm {
                    kind: SerTermKind::Literal,
                    value: Some(lexical.to_owned()),
                    datatype: datatype_slot,
                    lang: language.map(str::to_owned),
                    direction: direction.map(direction_str),
                    reifier: None,
                })
            }
            TermRef::Triple { s, p, o } => {
                // A quoted-triple term is a `Triple` term whose `reifier` points at a
                // self-reifier binding holding `(s, p, o)`. This self-reifier sentinel
                // is what the N-Quads serializer skips.
                let s = self.intern(dataset, s)?;
                let p = self.intern(dataset, p)?;
                let o = self.intern(dataset, o)?;
                let triple_id = self.terms.len();
                self.terms.push(SerTerm {
                    kind: SerTermKind::Triple,
                    value: None,
                    datatype: None,
                    lang: None,
                    direction: None,
                    reifier: Some(triple_id),
                });
                // Self-reifier sentinel for an inline quoted-triple TERM — never a
                // graph-scoped statement-layer row, so its graph slot is `None`.
                self.reifiers.push((triple_id, (s, p, o), None));
                triple_id
            }
        };
        self.memo.insert(value, idx);
        self.id_memo.insert(id, idx);
        Ok(idx)
    }

    /// Intern an IRI by value, deduplicating through the memo. Used for literal
    /// datatype terms, which the IR does not surface as standalone term ids.
    fn intern_iri_string(&mut self, iri: &str) -> usize {
        let value = TermValue::Iri(iri.to_owned());
        if let Some(&idx) = self.memo.get(&value) {
            return idx;
        }
        let idx = self.push_term(SerTerm {
            kind: SerTermKind::Iri,
            value: Some(iri.to_owned()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        });
        self.memo.insert(value, idx);
        idx
    }

    /// Resolve a triple-term id to the `(s, p, o)` term indices of its components
    /// (interning each), for a statement-layer reifier binding.
    fn intern_triple_components<D: DatasetView<Id = I>>(
        &mut self,
        dataset: &D,
        triple: D::Id,
    ) -> Result<(usize, usize, usize), RdfDiagnostic> {
        match dataset.resolve(triple) {
            TermRef::Triple { s, p, o } => {
                let s = self.intern(dataset, s)?;
                let p = self.intern(dataset, p)?;
                let o = self.intern(dataset, o)?;
                Ok((s, p, o))
            }
            other => Err(RdfDiagnostic::error(
                "native-codec-reifier-not-triple",
                format!("a reifier must bind a triple term, got {other:?}"),
            )),
        }
    }

    fn push_term(&mut self, term: SerTerm) -> usize {
        let idx = self.terms.len();
        self.terms.push(term);
        idx
    }
}

/// The dataset-independent value of an IR term, for the interner memo.
fn term_value<D: DatasetView>(dataset: &D, id: D::Id) -> TermValue {
    match dataset.resolve(id) {
        TermRef::Iri(iri) => TermValue::Iri(iri.to_owned()),
        TermRef::Blank { label, scope } => TermValue::Blank {
            label: label.to_owned(),
            scope,
        },
        TermRef::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => TermValue::Literal {
            lexical_form: lexical.to_owned(),
            datatype: iri_of(dataset, datatype).unwrap_or_default(),
            language: language.map(str::to_owned),
            direction,
        },
        TermRef::Triple { s, p, o } => TermValue::Triple {
            s: Box::new(term_value(dataset, s)),
            p: Box::new(term_value(dataset, p)),
            o: Box::new(term_value(dataset, o)),
        },
    }
}

/// Resolve an IR term id known to be an IRI (a literal datatype) to its IRI string.
fn iri_of<D: DatasetView>(dataset: &D, id: D::Id) -> Result<String, RdfDiagnostic> {
    iri_str_of(dataset, id).map(str::to_owned)
}

/// Borrowing twin of [`iri_of`]: the IRI straight out of the view, for callers that
/// only compare or copy it into the term table (no intermediate `String`).
fn iri_str_of<D: DatasetView>(dataset: &D, id: D::Id) -> Result<&str, RdfDiagnostic> {
    match dataset.resolve(id) {
        TermRef::Iri(iri) => Ok(iri),
        other => Err(RdfDiagnostic::error(
            "native-codec-datatype-not-iri",
            format!("a literal datatype must be an IRI, got {other:?}"),
        )),
    }
}

fn direction_str(direction: RdfTextDirection) -> String {
    match direction {
        RdfTextDirection::Ltr => "ltr".to_owned(),
        RdfTextDirection::Rtl => "rtl".to_owned(),
    }
}

#[cfg(test)]
mod serialize_to_format_tests {
    //! Coverage for the universal-transcoder helper
    //! [`serialize_dataset_to_format`], ported onto the native codecs. JSON-LD and
    //! YAML-LD are now first-class [`NativeRdfFormat`] variants routed through this
    //! helper, so their star-drop accounting is exercised alongside the others (they are
    //! star-capable, so the count is 0).
    use super::*;
    use crate::{RdfDataset, RdfDatasetBuilder, TermFactory, parse_dataset};
    use std::sync::Arc;

    /// A star-free dataset: 1 default-graph quad + 1 named-graph quad.
    fn star_free_dataset() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri_value("https://example.org/s");
        let p = b.intern_iri_value("https://example.org/p");
        let o = b.intern_iri_value("https://example.org/o");
        let g = b.intern_iri_value("https://example.org/g");
        let s2 = b.intern_iri_value("https://example.org/s2");
        let o2 = b.intern_iri_value("https://example.org/o2");
        b.push_quad(s, p, o, None);
        b.push_quad(s2, p, o2, Some(g));
        b.freeze().expect("star_free_dataset freeze")
    }

    /// A dataset WITH one reifier (`rdf:reifies` binding) + one annotation.
    fn reifier_dataset() -> Arc<RdfDataset> {
        let nq = concat!(
            "<https://e/s> <https://e/p> <https://e/o> .\n",
            "<https://e/r> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> ",
            "<<( <https://e/s> <https://e/p> <https://e/o> )>> .\n",
            "<https://e/r> <https://e/confidence> \"0.9\" .\n",
        );
        parse_dataset(nq.as_bytes(), "application/n-triples", None).expect("reifier_dataset parse")
    }

    fn text_of(outcome: &SerializeOutcome) -> String {
        String::from_utf8(outcome.bytes.clone()).expect("valid utf-8")
    }

    // ── star-capable formats: full statement layer, zero drops ────────────────────

    #[test]
    fn star_free_nquads_preserves_named_graph_drops_zero() {
        let ds = star_free_dataset();
        let out = serialize_dataset_to_format(&ds, NativeRdfFormat::NQuads, None)
            .expect("serialize to NQuads");
        assert_eq!(out.statement_rows_dropped, 0);
        let text = text_of(&out);
        assert!(text.contains("https://example.org/s"), "default-graph quad");
        assert!(text.contains("https://example.org/s2"), "named-graph quad");
        assert!(
            text.contains("https://example.org/g"),
            "named graph IRI preserved in NQuads"
        );
    }

    #[test]
    fn star_free_turtle_flattens_named_graph_drops_zero() {
        let ds = star_free_dataset();
        let out = serialize_dataset_to_format(&ds, NativeRdfFormat::Turtle, None)
            .expect("serialize to Turtle");
        assert_eq!(out.statement_rows_dropped, 0);
        let text = text_of(&out);
        assert!(text.contains("https://example.org/s"), "default-graph quad");
        // Turtle is default-graph-only: the named graph IRI must NOT appear.
        assert!(
            !text.contains("https://example.org/g"),
            "Turtle must not emit the named graph IRI"
        );
    }

    #[test]
    fn star_free_trig_preserves_named_graph_drops_zero() {
        let ds = star_free_dataset();
        let out = serialize_dataset_to_format(&ds, NativeRdfFormat::TriG, None)
            .expect("serialize to TriG");
        assert_eq!(out.statement_rows_dropped, 0);
        let text = text_of(&out);
        assert!(text.contains("https://example.org/s2"));
        assert!(
            text.contains("https://example.org/g"),
            "named graph preserved in a dataset-capable format"
        );
    }

    #[test]
    fn reifier_nquads_lossless() {
        let ds = reifier_dataset();
        assert_eq!(ds.reifiers().count(), 1);
        assert_eq!(ds.annotations().count(), 1);
        let out = serialize_dataset_to_format(&ds, NativeRdfFormat::NQuads, None)
            .expect("serialize to NQuads");
        assert_eq!(
            out.statement_rows_dropped, 0,
            "NQuads is star-capable: no rows dropped"
        );
        let text = text_of(&out);
        assert!(text.contains("22-rdf-syntax-ns#reifies"), "rdf:reifies row");
        assert!(text.contains("https://e/confidence"), "annotation row");
        assert!(text.contains("https://e/s"), "base quad");
    }

    // ── star-incapable format (RDF/XML): base quads only, rows reported dropped ────

    #[test]
    fn reifier_rdfxml_drops_statement_rows() {
        let ds = reifier_dataset();
        let out = serialize_dataset_to_format(&ds, NativeRdfFormat::RdfXml, None)
            .expect("serialize to RDF/XML");
        // 1 reifier + 1 annotation = 2 statement rows declared dropped (the
        // loss contract treats classic reification as non-faithful star).
        assert_eq!(out.statement_rows_dropped, 2);
        let text = text_of(&out);
        assert!(text.contains("https://e/s"), "base quad present in RDF/XML");
        assert!(
            !text.contains("22-rdf-syntax-ns#reifies"),
            "rdf:reifies must not appear in base-only RDF/XML output"
        );
    }

    #[test]
    fn star_free_rdfxml_drops_zero_when_no_statement_layer() {
        let ds = star_free_dataset();
        let out = serialize_dataset_to_format(&ds, NativeRdfFormat::RdfXml, None)
            .expect("serialize to RDF/XML");
        // No reifiers/annotations in the dataset → nothing to drop.
        assert_eq!(out.statement_rows_dropped, 0);
        // RDF/XML carries a base direction: nothing dropped there either.
        assert_eq!(out.directional_literals_dropped, 0);
        assert!(text_of(&out).contains("https://example.org/s"));
    }

    // ── base-direction drop accounting (TriX / HexTuples only) ─────────────────────

    /// A dataset with one base-direction object literal (`"hello"@en--ltr`).
    fn directional_literal_dataset() -> Arc<RdfDataset> {
        let nt = "<https://example.org/s> <https://example.org/greeting> \"hello\"@en--ltr .\n";
        parse_dataset(nt.as_bytes(), "application/n-triples", None)
            .expect("directional_literal_dataset parse")
    }

    #[test]
    fn directional_literal_dropped_by_trix_and_hextuples() {
        let ds = directional_literal_dataset();
        for format in [NativeRdfFormat::TriX, NativeRdfFormat::HexTuples] {
            let out = serialize_dataset_to_format(&ds, format, None)
                .expect("serialize to a direction-incapable format");
            assert_eq!(
                out.directional_literals_dropped, 1,
                "{format:?} must record the dropped base direction"
            );
            // The bytes still emit the language tag (only the direction is lost).
            let text = text_of(&out);
            assert!(text.contains("en"), "{format:?} keeps the language tag");
        }
    }

    #[test]
    fn directional_literal_preserved_by_direction_capable_formats() {
        let ds = directional_literal_dataset();
        for format in [
            NativeRdfFormat::Turtle,
            NativeRdfFormat::TriG,
            NativeRdfFormat::NTriples,
            NativeRdfFormat::NQuads,
            NativeRdfFormat::RdfXml,
            NativeRdfFormat::JsonLd,
            NativeRdfFormat::YamlLd,
        ] {
            let out = serialize_dataset_to_format(&ds, format, None)
                .expect("serialize to a direction-capable format");
            assert_eq!(
                out.directional_literals_dropped, 0,
                "{format:?} carries the base direction — nothing dropped"
            );
        }
    }
}
