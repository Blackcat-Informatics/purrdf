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
/// Native in-memory Open Knowledge Format Markdown-bundle codec.
pub mod okf;
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
    serialize_dataset_to_format_with_jsonld_options, serialize_dataset_with_jsonld_options,
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
        // Every bidirectional format round-trips a basic graph isomorphically — JSON-LD /
        // YAML-LD alongside the FULL declared peer set (including TriX / HexTuples).
        for media_type in [
            "text/turtle",
            "application/n-triples",
            "application/trig",
            "application/n-quads",
            "application/rdf+xml",
            "application/trix",
            "application/x-hextuples",
            "application/ld+json",
            "application/ld+yaml",
        ] {
            assert_round_trips(&ds, media_type);
        }
    }

    #[test]
    fn jsonld_and_yamlld_emit_empty_context_and_absolute_iris() {
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_iri("https://example.org/alice");
        let predicate = builder.intern_iri("https://schema.org/name");
        let object = builder.intern_literal(RdfLiteral::simple("Alice"));
        builder.push_quad(subject, predicate, object, None);
        let dataset = builder.freeze().expect("freeze");

        let json = jsonld::serialize_dataset_to_jsonld(&dataset).expect("JSON-LD");
        assert_eq!(
            json,
            "{\n  \"@context\": {},\n  \"@graph\": [\n    {\n      \"@id\": \"https://example.org/alice\",\n      \"https://schema.org/name\": {\n        \"@value\": \"Alice\"\n      }\n    }\n  ]\n}"
        );
        assert_round_trips(&dataset, "application/ld+json");

        let yaml = jsonld::serialize_dataset_to_yamlld(&dataset, None).expect("YAML-LD");
        let yaml_as_json = jsonld::yamlld_to_jsonld(yaml.as_bytes()).expect("YAML to JSON");
        let expected: serde_json::Value = serde_json::from_str(&json).expect("expected JSON");
        let actual: serde_json::Value = serde_json::from_str(&yaml_as_json).expect("actual JSON");
        assert_eq!(actual, expected);
        assert!(!yaml.contains("schema:"));
        assert!(yaml.contains("https://schema.org/name"));
        assert_round_trips(&dataset, "application/ld+yaml");
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
    fn nested_triple_term_with_inner_reifier_round_trips_jsonld() {
        // A depth-2 nested triple term: the OUTER triple term's subject is itself a
        // triple term (the INNER one), and the inner triple carries a reifier +
        // annotation. Every other star round-trip test in this module nests only one
        // level deep (a triple term appearing in a quad's object position); this is
        // the first to nest a triple term INSIDE another triple term's component,
        // which is the only path that exercises the `encode_triple_component` /
        // `parse_triple_term` recursion (jsonld.rs) — previously zero coverage.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("https://e/s");
        let p = b.intern_iri("https://e/p");
        let o = b.intern_iri("https://e/o");
        let inner = b.intern_triple(s, p, o); // depth-1 triple term: <<( s p o )>>
        let p2 = b.intern_iri("https://e/p2");
        let o2 = b.intern_iri("https://e/o2");
        let outer = b.intern_triple(inner, p2, o2); // depth-2: subject is itself a triple term
        let subj = b.intern_iri("https://e/subj");
        let asserts = b.intern_iri("https://e/asserts");
        b.push_quad(subj, asserts, outer, None); // assert the nested term in object position

        // Reifier + annotation live on the INNER triple, not the outer one.
        let r = b.intern_iri("https://e/r");
        let conf = b.intern_iri("https://e/confidence");
        let val = b.intern_literal(RdfLiteral::typed(
            "0.9",
            "http://www.w3.org/2001/XMLSchema#decimal",
        ));
        b.push_reifier(r, inner);
        b.push_annotation(r, conf, val);

        let ds = b.freeze().expect("freeze");
        assert_round_trips(&ds, "application/ld+json");
        assert_round_trips(&ds, "application/ld+yaml");
    }

    #[test]
    fn annotation_inside_triple_component_is_rejected() {
        // The RDF 1.2 abstract syntax gives a triple *term* no annotation of its own —
        // annotation/reification is always a SEPARATE statement about a reifier
        // (`?r rdf:reifies <<( s p o )>>` plus annotation triples on `?r`). purrdf's own
        // encoder never emits `@annotation` nested inside a `@triple` component (see
        // `nested_triple_term_with_inner_reifier_round_trips_jsonld` above, where the
        // inner triple's annotation rides a separate reifier quad instead). So a
        // hand-authored document with `@annotation` under a `@triple` component's
        // `@subject`/`@object` is not a well-formed RDF-1.2 term shape and must hard-fail
        // rather than silently drop the annotation.
        let json = r#"{
            "@id": "https://e/s",
            "https://e/asserts": {
                "@triple": {
                    "@subject": {
                        "@id": "https://e/s",
                        "@annotation": {
                            "@id": "https://e/r",
                            "https://e/confidence": { "@value": "0.9" }
                        }
                    },
                    "@predicate": "https://e/p",
                    "@object": { "@id": "https://e/o" }
                }
            }
        }"#;
        let err = parse_dataset(json.as_bytes(), "application/ld+json", None)
            .expect_err("@annotation inside a @triple component must be rejected");
        assert_eq!(err.code, "native-jsonld-decode");
        assert!(
            err.message
                .contains("not permitted inside a @triple component"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn annotation_directly_inside_triple_object_is_rejected() {
        // An `@annotation` placed DIRECTLY inside the `@triple` object itself (a
        // SIBLING of `@subject`/`@predicate`/`@object`, not nested under a component)
        // is equally not well-formed RDF-1.2: a triple *term* carries no annotation of
        // its own. Previously this key was read past and silently ignored rather than
        // rejected.
        let json = r#"{
            "@id": "https://e/s",
            "https://e/asserts": {
                "@triple": {
                    "@subject": { "@id": "https://e/s" },
                    "@predicate": "https://e/p",
                    "@object": { "@id": "https://e/o" },
                    "@annotation": {
                        "@id": "https://e/r",
                        "https://e/confidence": { "@value": "0.9" }
                    }
                }
            }
        }"#;
        let err = parse_dataset(json.as_bytes(), "application/ld+json", None)
            .expect_err("@annotation directly inside a @triple object must be rejected");
        assert_eq!(err.code, "native-jsonld-decode");
        assert!(
            err.message
                .contains("not permitted inside a @triple object"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn inline_triple_term_and_real_reifier_share_base_triple_round_trips_jsonld() {
        // The SAME base triple (s,p,o) is BOTH asserted as an object-position triple
        // term (which pushes a self-reifier sentinel keyed by the Triple-kind term
        // itself into `graph.reifiers`) AND carries a genuine IRI reifier with an
        // annotation. Building the JSON-LD `reifier_of` index used to sort ALL
        // `graph.reifiers` rows for a given (s,p,o) by their `@id`, including the
        // sentinel — but a `Triple`-kind term has no `@id`, so `term_id` returned
        // `Err` and the comparator's `.expect(...)` panicked. `reifier_of` must skip
        // Triple-kind sentinel rows so this collision no longer panics.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("https://e/s");
        let p = b.intern_iri("https://e/p");
        let o = b.intern_iri("https://e/o");
        let inner = b.intern_triple(s, p, o);
        let asserts = b.intern_iri("https://e/asserts");
        let subj = b.intern_iri("https://e/subj");
        b.push_quad(subj, asserts, inner, None); // object-position triple term for (s,p,o)
        let r = b.intern_iri("https://e/r");
        let conf = b.intern_iri("https://e/confidence");
        let val = b.intern_literal(RdfLiteral::typed(
            "0.9",
            "http://www.w3.org/2001/XMLSchema#decimal",
        ));
        b.push_reifier(r, inner); // real reifier on the SAME (s,p,o)
        b.push_annotation(r, conf, val);
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
    fn reifier_on_named_graph_triple_round_trips_jsonld() {
        // A reifier binding AND its annotation live on a triple INSIDE a named graph.
        // JSON-LD must thread the reifier/annotation graph slot, not force it to the
        // default graph — otherwise the round-trip is not isomorphic.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("https://e/s");
        let p = b.intern_iri("https://e/p");
        let o = b.intern_iri("https://e/o");
        let g = b.intern_iri("https://e/g");
        let triple = b.intern_triple(s, p, o);
        let r = b.intern_iri("https://e/r");
        let conf = b.intern_iri("https://e/confidence");
        let val = b.intern_literal(RdfLiteral::typed(
            "0.9",
            "http://www.w3.org/2001/XMLSchema#decimal",
        ));
        b.push_quad(s, p, o, Some(g));
        b.push_reifier_in_graph(r, triple, Some(g));
        b.push_annotation_in_graph(r, conf, val, Some(g));
        let ds = b.freeze().expect("freeze");
        assert_round_trips(&ds, "application/ld+json");
        assert_round_trips(&ds, "application/ld+yaml");
    }

    #[test]
    fn same_base_triple_reified_differently_in_two_named_graphs_round_trips_jsonld() {
        // The SAME base triple (s,p,o) is asserted in TWO distinct named graphs, each
        // reified by a DIFFERENT reifier with a DIFFERENT annotation value. The
        // reifier/annotation lookup keys previously discarded the graph slot, so every
        // reifier for (s,p,o) got attached to the value object in EVERY graph carrying
        // that triple — fabricating reifier bindings the author never placed. Six quads
        // go in (2 asserted triples + 2 reifier bindings + 2 annotations); the round-trip
        // must come back isomorphic, not cross-contaminated to ~10 quads.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("https://e/s");
        let p = b.intern_iri("https://e/p");
        let o = b.intern_iri("https://e/o");
        let g1 = b.intern_iri("https://e/g1");
        let g2 = b.intern_iri("https://e/g2");
        let triple = b.intern_triple(s, p, o);
        let conf = b.intern_iri("https://e/confidence");

        let r1 = b.intern_iri("https://e/r1");
        let val1 = b.intern_literal(RdfLiteral::typed(
            "0.1",
            "http://www.w3.org/2001/XMLSchema#decimal",
        ));
        b.push_quad(s, p, o, Some(g1));
        b.push_reifier_in_graph(r1, triple, Some(g1));
        b.push_annotation_in_graph(r1, conf, val1, Some(g1));

        let r2 = b.intern_iri("https://e/r2");
        let val2 = b.intern_literal(RdfLiteral::typed(
            "0.9",
            "http://www.w3.org/2001/XMLSchema#decimal",
        ));
        b.push_quad(s, p, o, Some(g2));
        b.push_reifier_in_graph(r2, triple, Some(g2));
        b.push_annotation_in_graph(r2, conf, val2, Some(g2));

        let ds = b.freeze().expect("freeze");
        assert_round_trips(&ds, "application/ld+json");
        assert_round_trips(&ds, "application/ld+yaml");
    }

    #[test]
    fn gts_codec_backend_reaches_jsonld_and_yamlld_no_side_door() {
        // The GENERIC backend (RdfParserBackend + RdfSerializer) reaches JSON-LD/YAML-LD
        // through classify + codec_for — proving no `native_codecs::jsonld::` side-door is
        // needed. A named-graph + reifier dataset exercises the star + dataset paths.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("https://e/s");
        let p = b.intern_iri("https://e/p");
        let o = b.intern_iri("https://e/o");
        let g = b.intern_iri("https://e/g");
        let triple = b.intern_triple(s, p, o);
        let r = b.intern_iri("https://e/r");
        let conf = b.intern_iri("https://e/confidence");
        let val = b.intern_literal(RdfLiteral::simple("high"));
        b.push_quad(s, p, o, Some(g));
        b.push_reifier_in_graph(r, triple, Some(g));
        b.push_annotation_in_graph(r, conf, val, Some(g));
        let ds = b.freeze().expect("freeze");

        let backend = GtsCodecBackend;
        for media_type in ["application/ld+json", "application/ld+yaml"] {
            // Serialize through the generic RdfSerializer.
            let mut bytes = Vec::new();
            backend
                .serialize(
                    &ds,
                    RdfSerializeRequest {
                        media_type,
                        graph: SerializeGraph::Dataset,
                        base_iri: None,
                    },
                    &mut bytes,
                )
                .expect("generic serialize");
            // Parse back through the generic RdfParserBackend into a frozen dataset.
            let mut sink = DatasetSink::new();
            backend
                .parse_into(
                    RdfParseRequest {
                        bytes: &bytes,
                        media_type,
                        base_iri: None,
                        source_name: None,
                    },
                    &mut sink,
                )
                .expect("generic parse");
            let reparsed = sink.into_dataset().expect("sink finished");
            assert!(
                datasets_isomorphic(&ds, &reparsed),
                "GtsCodecBackend round-trip via {media_type} must be isomorphic"
            );
        }
    }

    #[test]
    fn jsonld_yamlld_are_star_capable_no_rows_dropped() {
        // JSON-LD / YAML-LD are star-capable, so `serialize_dataset_to_format` reports
        // ZERO dropped statement rows even when the dataset carries reifiers/annotations.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("https://e/s");
        let p = b.intern_iri("https://e/p");
        let o = b.intern_iri("https://e/o");
        let triple = b.intern_triple(s, p, o);
        let r = b.intern_iri("https://e/r");
        let conf = b.intern_iri("https://e/confidence");
        let val = b.intern_literal(RdfLiteral::simple("high"));
        b.push_quad(s, p, o, None);
        b.push_reifier(r, triple);
        b.push_annotation(r, conf, val);
        let ds = b.freeze().expect("freeze");
        for format in [NativeRdfFormat::JsonLd, NativeRdfFormat::YamlLd] {
            let outcome = serialize_dataset_to_format(&ds, format, None).expect("serialize");
            assert_eq!(
                outcome.statement_rows_dropped, 0,
                "{format:?} is star-capable, so no statement rows drop"
            );
        }
    }

    #[test]
    fn registry_capabilities_stay_consistent_with_the_loss_ledger() {
        use crate::loss::{supports_quads, supports_stars};

        // The registry (FORMATS) and rdf-core::loss are two hand-maintained capability
        // tables that MUST agree. For every loss-named format, the descriptor bool equals
        // the independent loss.rs predicate — so neither can silently drift.
        let mut unnamed: Vec<NativeRdfFormat> = Vec::new();
        for descriptor in media_type::FORMATS {
            let format = descriptor.format;
            match format.loss_codec_name() {
                Some(name) => {
                    assert_eq!(
                        format.carries_star(),
                        supports_stars(name),
                        "{format:?} carries_star vs loss::supports_stars({name})"
                    );
                    assert_eq!(
                        format.supports_datasets(),
                        supports_quads(name),
                        "{format:?} supports_datasets vs loss::supports_quads({name})"
                    );
                }
                None => unnamed.push(format),
            }
        }
        // Exactly TriX and HexTuples carry no loss codec name, so no format silently
        // escapes the consistency check.
        assert_eq!(
            unnamed,
            vec![NativeRdfFormat::TriX, NativeRdfFormat::HexTuples],
            "only TriX / HexTuples may lack a loss codec name"
        );
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
    fn yamlld_adversarial_scalar_round_trips() {
        // The "Norway problem": YAML's implicit-typing resolver reinterprets bare
        // scalars like `true`, `no`, `~`, `1.0`, `0755`, or `2020-01-01` as
        // bool/null/number/timestamp on re-parse unless the emitter quotes them.
        // Every lexical form below is exactly one of those adversarial tokens; the
        // round-trip bar is LOSSLESS (`assert_round_trips` requires isomorphism), so
        // the JSON->YAML bridge must force-quote them rather than let serde_yaml's
        // default resolver re-coerce the type.
        let adversarial = [
            "true",
            "false",
            "null",
            "~",
            "no",
            "yes",
            "on",
            "off",
            "1.0",
            "0755",
            "0x1A",
            "1_000",
            "",
            "2020-01-01",
        ];
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("https://e/s");
        for (i, scalar) in adversarial.iter().enumerate() {
            // The scalar as a plain literal — implied xsd:string (no datatype IRI).
            let p_plain = b.intern_iri(&format!("https://e/plain{i}"));
            let plain = b.intern_literal(RdfLiteral::simple(*scalar));
            b.push_quad(s, p_plain, plain, None);

            // The same scalar as an EXPLICIT (non-numeric, non-boolean) xsd:string
            // typed literal — the lexical form must survive verbatim either way.
            let p_typed = b.intern_iri(&format!("https://e/typed{i}"));
            let typed = b.intern_literal(RdfLiteral::typed(
                *scalar,
                "http://www.w3.org/2001/XMLSchema#string",
            ));
            b.push_quad(s, p_typed, typed, None);
        }
        let ds = b.freeze().expect("freeze");
        // JSON-LD as a control: JSON has no ambiguous bare-scalar resolver, so this
        // must always pass and isolates any failure to the YAML bridge.
        assert_round_trips(&ds, "application/ld+json");
        assert_round_trips(&ds, "application/ld+yaml");
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
