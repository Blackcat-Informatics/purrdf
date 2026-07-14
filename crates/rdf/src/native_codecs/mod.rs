// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native RDF text codecs (S3).
//!
//! The codec-only backend that parses and serializes Turtle / TriG / N-Triples /
//! N-Quads / RDF/XML, emitting through the `purrdf-events` seam into the frozen
//! [`RdfDataset`] IR. The line/Turtle family and RDF/XML are now FIRST-PARTY (EPIC
//! `text_parse` for the line/Turtle family, `rdfxml` for RDF/XML); they no
//! longer route through the external `purrdf-gts` text/RDF-XML codecs. It implements
//! the narrow
//! [`RdfParserBackend`]/[`RdfSerializer`] traits and is **codec-only** — it never
//! touches the oxigraph Store, so it compiles under `--no-default-features --features
//! gts` (no oxigraph). That is the  end-state: the text path needs no Store.
//!
//! [`GtsCodecBackend`] is the always-on native replacement for `OxigraphBackend`'s
//! codec role; the workspace-wide sweep (Tasks 2–5) routes every
//! `oxigraph::io` text parse/serialize call site through it.

mod media_type;
// First-party serialization model + the Turtle / TriG / N-Triples / N-Quads text
// serializers that walk it, replacing the external purrdf-gts text serializers.
// `pub(crate)` so the container bridge (`crate::gts::gts_to_ser`) can construct a
// `SerGraph` from a real purrdf-gts model graph read out of a bundle.
pub(crate) mod ser_model;
// `pub(crate)` so the legacy `dataset_io` oxigraph path can reuse the SHARED
// `fold_statement_layer` (one fold, no drift) — Task 1.
pub(crate) mod parse;
mod serialize;
// First-party JSON-LD-star / YAML-LD-star codec: serializes the frozen IR to the PurRDF
// JSON-LD-star / YAML-LD-star surface and parses it back, walking the
// same first-party `SerGraph` the RDF text serializers use. The lowest crate the rdf /
// validate / pipeline consumers share, so all three call it here (the codec previously
// lived in `purrdf-pipeline::stages::yaml_ld`, above `rdf` and `validate`).
pub mod jsonld;
// First-party N-Triples / N-Quads / Turtle / TriG text parser: lowers
// directly to the in-memory GtsGraph the statement-layer fold consumes, replacing the
// purrdf-gts text codecs for the line/Turtle family.
mod text_parse;
// First-party RDF/XML codec: implements the W3C RDF/XML grammar in-repo on
// a pure-Rust XML DOM (`roxmltree`), parsing straight into the frozen IR through the
// shared statement-layer fold and serializing from the first-party `SerGraph` —
// replacing the external purrdf-gts `rdf_codecs::{from_rdf_xml, to_rdf_xml}` codec
// entry points (the first-party mandate). It is fully purrdf-gts free.
mod rdfxml;
// First-party TriX codec ("Triples in XML"): a quads/named-graph XML serialization
// parsed on the same pure-Rust XML DOM (`roxmltree`) as `rdfxml` and serialized by
// hand-rolled deterministic XML emission from the first-party `SerGraph`.
mod trix;
// First-party HexTuples codec: a line-oriented NDJSON quads serialization, encoded and
// decoded through `serde_json` (already a dep) into/from the first-party `SerGraph`.
mod hextuples;
// Opt-in triple → source-position side table (SARIF source tracing). A runtime option,
// NOT a Cargo feature; the default `NoSpans` collector monomorphizes the recording out
// so the pre-existing parse path is byte-identical.
mod span;
// Per-format codec dispatch: the single `NativeRdfFormat` → behavior chokepoint. Holds
// the `RdfCodec` trait, the shared `LineCodec` for the line/Turtle family, and the
// `codec_for` resolver every parse/serialize call site routes through (replacing the
// scattered match-on-format arms). `pub(super)`, never part of the public API.
mod codec;

pub use media_type::{NativeRdfFormat, classify};
pub use parse::{parse_dataset, parse_dataset_with};
pub use span::{ParseOptions, SpanTable};
// Bench/test-only sequential baseline for the chunk-parallel N-Triples/N-Quads path;
// hidden, unstable, not public API.
#[doc(hidden)]
pub use parse::parse_dataset_forced_sequential;
pub use serialize::{
    SerializeOutcome, serialize_dataset, serialize_dataset_base_only, serialize_dataset_to_format,
};

use std::io::Write;

use purrdf_events::{RdfEventSink, RdfEventSource};

use crate::ir::FrozenDatasetSource;
use crate::{
    RdfDataset, RdfDiagnostic, RdfParseRequest, RdfParserBackend, RdfSerializeRequest,
    RdfSerializer,
};

/// The native codec backend: a codec-only [`RdfParserBackend`] + [`RdfSerializer`] over
/// the `purrdf-gts` text codecs. Holds no state and references no oxigraph Store.
#[derive(Debug, Clone, Copy, Default)]
pub struct GtsCodecBackend;

impl RdfParserBackend for GtsCodecBackend {
    fn parse_into<S: RdfEventSink + ?Sized>(
        &self,
        request: RdfParseRequest<'_>,
        sink: &mut S,
    ) -> Result<(), RdfDiagnostic> {
        // Parse to a frozen dataset, then replay it into the caller's sink through the
        // in-repo frozen-IR source (which declares-before-reference and finishes the
        // sink). Driving the source IS the `finish` call, so the sink is left finished.
        let dataset = parse_dataset(request.bytes, request.media_type, request.base_iri)?;
        FrozenDatasetSource::new(&dataset)
            .drive(sink)
            .map_err(|e| RdfDiagnostic::error("native-codec-replay", e.to_string()))
    }
}

impl RdfSerializer for GtsCodecBackend {
    fn serialize<W: Write>(
        &self,
        dataset: &RdfDataset,
        request: RdfSerializeRequest<'_>,
        output: W,
    ) -> Result<(), RdfDiagnostic> {
        serialize::serialize_into(dataset, request.media_type, request.graph, output)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::ir::compare::datasets_isomorphic;
    use crate::{
        BlankScope, DatasetSink, RdfDatasetBuilder, RdfLiteral, RdfTextDirection, SerializeGraph,
        TermValue,
    };

    /// Round-trip a dataset through serialize → parse and assert isomorphism.
    fn round_trip(dataset: &RdfDataset, media_type: &str) -> Arc<RdfDataset> {
        let bytes =
            serialize_dataset(dataset, media_type, SerializeGraph::Dataset).expect("serialize");
        parse_dataset(&bytes, media_type, None).expect("re-parse")
    }

    fn assert_round_trips(dataset: &RdfDataset, media_type: &str) {
        let reparsed = round_trip(dataset, media_type);
        assert!(
            datasets_isomorphic(dataset, &reparsed),
            "round-trip via {media_type} must be isomorphic"
        );
    }

    #[test]
    fn basic_graph_round_trips() {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("https://e/s");
        let p = b.intern_iri("https://e/p");
        let o = b.intern_iri("https://e/o");
        b.push_quad(s, p, o, None);
        let ds = b.freeze().expect("freeze");
        assert_round_trips(&ds, "text/turtle");
        assert_round_trips(&ds, "application/n-triples");
        assert_round_trips(&ds, "application/trig");
        assert_round_trips(&ds, "application/n-quads");
        assert_round_trips(&ds, "application/rdf+xml");
    }

    #[test]
    fn reifier_and_annotation_round_trip() {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("https://e/s");
        let p = b.intern_iri("https://e/p");
        let o = b.intern_iri("https://e/o");
        let triple = b.intern_triple(s, p, o);
        let r = b.intern_iri("https://e/r");
        let conf = b.intern_iri("https://e/confidence");
        let val = b.intern_literal(RdfLiteral::typed(
            "0.9",
            "http://www.w3.org/2001/XMLSchema#decimal",
        ));
        b.push_reifier(r, triple);
        b.push_annotation(r, conf, val);
        let ds = b.freeze().expect("freeze");
        assert_round_trips(&ds, "application/n-triples");
        assert_round_trips(&ds, "text/turtle");
        // The reifier/@annotation form also round-trips through JSON-LD/YAML-LD.
        assert_round_trips(&ds, "application/ld+json");
        assert_round_trips(&ds, "application/ld+yaml");
    }

    #[test]
    fn object_position_triple_term_round_trips_jsonld() {
        // A quad whose object is an RDF-1.2 triple term — the exact construct N-Quads
        // round-trips (`quoted_triple_term_round_trips`). The distinguishable `@triple`
        // encoding makes it round-trip losslessly through JSON-LD and YAML-LD too, so
        // `carries_star == true` is honest for these formats.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("https://e/s");
        let p = b.intern_iri("https://e/p");
        let o = b.intern_iri("https://e/o");
        let inner = b.intern_triple(s, p, o);
        let asserts = b.intern_iri("https://e/asserts");
        b.push_quad(s, asserts, inner, None);
        let ds = b.freeze().expect("freeze");
        assert_round_trips(&ds, "application/ld+json");
        assert_round_trips(&ds, "application/ld+yaml");
    }

    #[test]
    fn triple_valued_annotation_round_trips_jsonld() {
        // An annotation whose VALUE is itself a triple term exercises the
        // annotation-value `@triple` path (`simple_term_value`'s `Triple` arm).
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("https://e/s");
        let p = b.intern_iri("https://e/p");
        let o = b.intern_iri("https://e/o");
        let base = b.intern_triple(s, p, o);
        let r = b.intern_iri("https://e/r");
        b.push_reifier(r, base);
        let derived = b.intern_iri("https://e/derivedFrom");
        let x = b.intern_iri("https://e/x");
        let y = b.intern_iri("https://e/y");
        let z = b.intern_iri("https://e/z");
        let ann_triple = b.intern_triple(x, y, z);
        b.push_annotation(r, derived, ann_triple);
        let ds = b.freeze().expect("freeze");
        assert_round_trips(&ds, "application/ld+json");
        assert_round_trips(&ds, "application/ld+yaml");
    }

    #[test]
    fn rtl_literal_round_trips() {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("https://e/s");
        let p = b.intern_iri("https://e/p");
        let lit = b.intern_literal(RdfLiteral {
            lexical_form: "\u{0645}\u{0631}\u{062d}\u{0628}\u{0627}".to_owned(),
            datatype: None,
            language: Some("ar".to_owned()),
            direction: Some(RdfTextDirection::Rtl),
        });
        b.push_quad(s, p, lit, None);
        let ds = b.freeze().expect("freeze");
        assert_round_trips(&ds, "application/n-triples");
    }

    #[test]
    fn quoted_triple_term_round_trips() {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("https://e/s");
        let p = b.intern_iri("https://e/p");
        let o = b.intern_iri("https://e/o");
        let inner = b.intern_triple(s, p, o);
        let asserts = b.intern_iri("https://e/asserts");
        b.push_quad(s, asserts, inner, None);
        let ds = b.freeze().expect("freeze");
        // FINDING: purrdf-gts 0.9.5's `to_turtle`/`to_ntriples`/`to_trig`/
        // `to_rdf_xml` event-source serializers CANNOT emit a quoted-triple term that
        // appears as a quad object — they hit a self-reifier "cycle while declaring
        // term N" (reproducible directly against purrdf-gts's own `from_nquads` output).
        // Only the `to_nquads` path handles the self-reifier sentinel. So a
        // quad-object triple term round-trips ONLY through N-Quads here. The IR + parse
        // path are correct (the GTS graph is byte-identical to purrdf-gts's native
        // `from_nquads` representation); this is a purrdf-gts serializer gap.
        assert_round_trips(&ds, "application/n-quads");
    }

    #[test]
    fn two_scope_blank_nodes_round_trip() {
        // Two same-label blanks in different scopes are distinct; the qualified label
        // (`x` vs `x.s1`) survives the round trip as two distinct blank nodes.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("https://e/s");
        let p = b.intern_iri("https://e/p");
        let b0 = b.intern_blank("x", BlankScope(0));
        let b1 = b.intern_blank("x", BlankScope(1));
        b.push_quad(s, p, b0, None);
        b.push_quad(s, p, b1, None);
        let ds = b.freeze().expect("freeze");
        // Two distinct blanks → two quads survive (a collapse would drop one).
        let reparsed = round_trip(&ds, "application/n-triples");
        assert_eq!(
            reparsed.quad_count(),
            2,
            "two distinct blanks stay distinct"
        );
    }

    #[test]
    fn lexical_form_is_preserved_verbatim() {
        // B2 fidelity: the native path must NOT canonicalize typed literals the way the
        // oxigraph Store does. "0.70", a "+00:00" dateTime, and "1.0E0" survive
        // parse → serialize → re-parse with their lexical form UNCHANGED.
        let cases = [
            ("0.70", "http://www.w3.org/2001/XMLSchema#decimal"),
            (
                "2024-01-01T00:00:00+00:00",
                "http://www.w3.org/2001/XMLSchema#dateTime",
            ),
            ("1.0E0", "http://www.w3.org/2001/XMLSchema#double"),
        ];
        for (lexical, datatype) in cases {
            let mut b = RdfDatasetBuilder::new();
            let s = b.intern_iri("https://e/s");
            let p = b.intern_iri("https://e/p");
            let lit = b.intern_literal(RdfLiteral::typed(lexical, datatype));
            b.push_quad(s, p, lit, None);
            let ds = b.freeze().expect("freeze");
            let reparsed = round_trip(&ds, "application/n-triples");
            assert!(
                reparsed
                    .term_id_by_value(&TermValue::Literal {
                        lexical_form: lexical.to_owned(),
                        datatype: datatype.to_owned(),
                        language: None,
                        direction: None,
                    })
                    .is_some(),
                "lexical form `{lexical}^^{datatype}` must survive verbatim"
            );
        }
    }

    #[test]
    fn backend_parses_into_event_sink() {
        let mut sink = DatasetSink::new();
        GtsCodecBackend
            .parse_into(
                RdfParseRequest {
                    bytes: b"<rel> <https://e/p> <https://e/o> .",
                    media_type: "text/turtle",
                    base_iri: Some("https://example.org/"),
                    source_name: Some("inline.ttl"),
                },
                &mut sink,
            )
            .expect("parse into sink");
        let dataset = sink.into_dataset().expect("sink finished");
        assert_eq!(dataset.quad_count(), 1);
        assert!(
            dataset
                .term_id_by_value(&TermValue::Iri("https://example.org/rel".to_owned()))
                .is_some()
        );
    }

    #[test]
    fn backend_serializes_default_graph_without_named_rows() {
        let mut b = RdfDatasetBuilder::new();
        let default_s = b.intern_iri("https://e/default-s");
        let named_s = b.intern_iri("https://e/named-s");
        let p = b.intern_iri("https://e/p");
        let o = b.intern_iri("https://e/o");
        let g = b.intern_iri("https://e/g");
        b.push_quad(default_s, p, o, None);
        b.push_quad(named_s, p, o, Some(g));
        let ds = b.freeze().expect("freeze");

        let bytes = serialize_dataset(&ds, "application/n-triples", SerializeGraph::DefaultGraph)
            .expect("serialize");
        let text = String::from_utf8(bytes).expect("utf8");
        assert!(text.contains("https://e/default-s"));
        assert!(!text.contains("https://e/named-s"));
        assert!(!text.contains("https://e/g"));
    }
}
