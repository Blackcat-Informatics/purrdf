// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Per-format codec dispatch for the native RDF text codecs.
//!
//! [`RdfCodec`] is the single object-safe seam the [`NativeRdfFormat`] router resolves
//! to: [`codec_for`] maps each format to one `&'static dyn RdfCodec`, so the parse /
//! serialize / star-capability / span-capability decisions live in ONE place instead of
//! the parallel `match` arms that were scattered across [`parse`](super::parse) and
//! [`serialize`](super::serialize).
//!
//! The four line/Turtle-family formats (N-Triples / N-Quads / Turtle / TriG) share ONE
//! implementor ([`LineCodec`]) BY DESIGN — they share the [`text_parse`](super::text_parse)
//! front-end and the [`ser_model`](super::ser_model) serializers, so splitting them into
//! four codecs would shatter that fusion. RDF/XML, TriX and HexTuples are standalone
//! codecs whose implementors live beside their own codec functions
//! ([`super::rdfxml`], [`super::trix`], [`super::hextuples`]).
//!
//! This is a `pub(super)` INTERNAL organizing device, NOT a public seam — it never
//! leaves the `native_codecs` module. The crate's public contract stays the
//! `RdfParserBackend` / `RdfSerializer` traits in `purrdf-rdf-core`.

use std::sync::Arc;

use super::media_type::NativeRdfFormat;
use super::ser_model::{self, SerGraph};
use super::span::NoSpans;
use super::text_parse::LineParseMode;
use crate::{RdfDataset, RdfDiagnostic};

/// One RDF text format's parse + serialize behavior. Object-safe so [`codec_for`] can
/// hand back a `&'static dyn RdfCodec` — a COLD dispatch boundary (the media-type
/// router), never the tokenizer hot loop, so the one indirect call it adds is not on any
/// measured path. The DATA capability predicates (`carries_star` /
/// `tokenizer_carries_spans` / `supports_datasets`) live on [`NativeRdfFormat`] itself,
/// backed by the [`FORMATS`](super::media_type::FORMATS) table, not on this behavior seam.
pub(super) trait RdfCodec {
    /// Parse `text` into the frozen IR on the hot (span-free) path. `mode` is honored
    /// only by [`LineCodec`] (the N-Triples/N-Quads chunk-parallel toggle); the
    /// standalone codecs ignore it. Every implementor is panic-guarded, matching the
    /// per-format guards the call sites applied before.
    fn parse(
        &self,
        text: &str,
        base_iri: Option<&str>,
        mode: LineParseMode,
    ) -> Result<Arc<RdfDataset>, RdfDiagnostic>;

    /// Serialize a first-party [`SerGraph`] to this format's text.
    fn serialize(&self, graph: &SerGraph) -> Result<String, RdfDiagnostic>;
}

/// The shared implementor for the four line/Turtle-family formats, keyed by the wrapped
/// variant. They parse through one `text_parse` front-end and serialize through the
/// matching `ser_model` writer, so a SINGLE codec — not four — preserves that fusion.
pub(super) struct LineCodec(pub(super) NativeRdfFormat);

impl RdfCodec for LineCodec {
    fn parse(
        &self,
        text: &str,
        base_iri: Option<&str>,
        mode: LineParseMode,
    ) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
        // The hot path never records spans; the span-tracking path in
        // `parse_dataset_with` calls `text_parse_without_panicking` with a `SpanTable`
        // directly (a distinct monomorphization), so `NoSpans` stays zero-cost here.
        let graph =
            super::parse::text_parse_without_panicking(self.0, text, base_iri, mode, &mut NoSpans)?;
        super::parse::dataset_from_ser_graph(&graph)
    }

    fn serialize(&self, graph: &SerGraph) -> Result<String, RdfDiagnostic> {
        Ok(match self.0 {
            NativeRdfFormat::Turtle => ser_model::to_turtle(graph)?,
            NativeRdfFormat::TriG => ser_model::to_trig(graph),
            NativeRdfFormat::NTriples => ser_model::to_ntriples(graph)?,
            NativeRdfFormat::NQuads => ser_model::to_nquads(graph),
            NativeRdfFormat::RdfXml
            | NativeRdfFormat::TriX
            | NativeRdfFormat::HexTuples
            | NativeRdfFormat::JsonLd
            | NativeRdfFormat::YamlLd => {
                unreachable!("LineCodec only wraps line/Turtle-family formats")
            }
        })
    }
}

/// Resolve a [`NativeRdfFormat`] to its codec — the SINGLE format→behavior chokepoint.
///
/// The returned reference is `'static` via rvalue static promotion: every implementor is
/// a zero-sized (or `Copy`-enum-wrapping) value with no interior mutability and no
/// destructor, so `&Codec` promotes to a `&'static` borrow of a constant.
pub(super) fn codec_for(format: NativeRdfFormat) -> &'static dyn RdfCodec {
    match format {
        NativeRdfFormat::NTriples => &LineCodec(NativeRdfFormat::NTriples),
        NativeRdfFormat::NQuads => &LineCodec(NativeRdfFormat::NQuads),
        NativeRdfFormat::Turtle => &LineCodec(NativeRdfFormat::Turtle),
        NativeRdfFormat::TriG => &LineCodec(NativeRdfFormat::TriG),
        NativeRdfFormat::RdfXml => &super::rdfxml::RdfXmlCodec,
        NativeRdfFormat::TriX => &super::trix::TriXCodec,
        NativeRdfFormat::HexTuples => &super::hextuples::HexTuplesCodec,
        NativeRdfFormat::JsonLd => &super::jsonld::JsonLdCodec,
        NativeRdfFormat::YamlLd => &super::jsonld::YamlLdCodec,
    }
}
