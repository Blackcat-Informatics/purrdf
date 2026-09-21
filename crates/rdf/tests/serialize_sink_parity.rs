// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The streamed serialization equals the eager one, byte for byte, for every format.
//!
//! This is acceptance for the incremental path, and it is asserted on the public
//! surface rather than on an internal writer: `serialize_dataset_with` against
//! `serialize_dataset_to_writer_with`, over the same dataset and the same options.
//!
//! The equality is structural — both spellings are the same emitter pointed at a
//! different destination — so this suite exists to catch a future change that
//! reintroduces a second path, not to establish the property in the first place.
//!
//! The format list is a `match` with NO wildcard arm. Adding a variant to
//! `NativeRdfFormat` fails to compile here until it is covered, so a new format
//! cannot quietly arrive without a parity case.

use purrdf_core::{RdfDatasetBuilder, RdfLiteral, RdfTerm, RdfTextDirection};
use purrdf_rdf::{
    JsonLdSerializeOptions, NativeRdfFormat, SerializeGraph, SerializeOptions, StatementLayer,
    serialize_dataset_to_writer_with, serialize_dataset_with,
};
use std::sync::Arc;

/// Every format, enumerated exhaustively so the registry cannot grow past this file.
const fn all_formats() -> [NativeRdfFormat; 9] {
    // A wildcard-free match over a value of the type: if a variant is added, this
    // fails to compile rather than silently going untested.
    const fn covered(format: NativeRdfFormat) -> bool {
        match format {
            NativeRdfFormat::Turtle
            | NativeRdfFormat::TriG
            | NativeRdfFormat::NTriples
            | NativeRdfFormat::NQuads
            | NativeRdfFormat::RdfXml
            | NativeRdfFormat::TriX
            | NativeRdfFormat::HexTuples
            | NativeRdfFormat::JsonLd
            | NativeRdfFormat::YamlLd => true,
        }
    }
    assert!(covered(NativeRdfFormat::Turtle));
    [
        NativeRdfFormat::Turtle,
        NativeRdfFormat::TriG,
        NativeRdfFormat::NTriples,
        NativeRdfFormat::NQuads,
        NativeRdfFormat::RdfXml,
        NativeRdfFormat::TriX,
        NativeRdfFormat::HexTuples,
        NativeRdfFormat::JsonLd,
        NativeRdfFormat::YamlLd,
    ]
}

/// A plain dataset every format can carry.
fn simple() -> Arc<purrdf_core::RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    for i in 0..64 {
        let s = b.intern_owned_term(&RdfTerm::iri(format!("https://example.org/s{i}")));
        let p = b.intern_iri("https://example.org/p");
        let o = b.intern_literal(RdfLiteral::simple(format!("value {i}")));
        b.push_quad(s, p, o, None);
    }
    b.freeze().expect("dataset freezes")
}

/// Large enough that the streamed path crosses its drain window many times.
fn large() -> Arc<purrdf_core::RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("https://example.org/predicate-with-a-long-name-for-bulk");
    for i in 0..4_000 {
        let s = b.intern_owned_term(&RdfTerm::iri(format!(
            "https://example.org/subject/{i}/with/a/deliberately/long/path"
        )));
        let o = b.intern_literal(RdfLiteral::simple(format!(
            "a deliberately long literal value for row {i}, padded so the document \
             crosses the sink's staging window many times over"
        )));
        b.push_quad(s, p, o, None);
    }
    b.freeze().expect("dataset freezes")
}

/// Empty: the case where a header-bearing syntax must emit nothing at all.
fn empty() -> Arc<purrdf_core::RdfDataset> {
    RdfDatasetBuilder::new().freeze().expect("dataset freezes")
}

/// A directional literal, which several formats cannot express and must count.
fn directional() -> Arc<purrdf_core::RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_owned_term(&RdfTerm::iri("https://example.org/s"));
    let p = b.intern_iri("https://example.org/p");
    let o = b.intern_literal(RdfLiteral {
        direction: Some(RdfTextDirection::Rtl),
        ..RdfLiteral::language_tagged("مرحبا", "ar")
    });
    b.push_quad(s, p, o, None);
    b.freeze().expect("dataset freezes")
}

fn fixtures() -> Vec<(&'static str, Arc<purrdf_core::RdfDataset>)> {
    vec![
        ("empty", empty()),
        ("simple", simple()),
        ("large", large()),
        ("directional", directional()),
    ]
}

/// The `jsonld_options` axis of the cross-product.
///
/// This is the ONE dispatch arm that does not pass through
/// `codec_for(format).serialize_into`. A configured request routes straight to the
/// JSON-LD / YAML-LD writers instead, so it is the one arm where a second emitter
/// could reappear without this suite noticing — which is the whole reason the suite
/// exists. Leaving the axis pinned at `None`, as it was, left exactly the path that
/// bypasses the seam as the path nothing compared.
///
/// Every other format REFUSES a configured request. That is covered here too, and
/// deliberately: the two spellings must refuse identically, because a refusal that
/// fires on one path and not the other is the same divergence as diverging bytes,
/// and it is the divergence that looks like correctness from either side alone.
fn jsonld_option_modes() -> Vec<(&'static str, Option<JsonLdSerializeOptions>)> {
    vec![
        ("no-options", None),
        ("expanded", Some(JsonLdSerializeOptions::expanded())),
        ("derived", Some(JsonLdSerializeOptions::derived())),
    ]
}

/// The eager and streamed spellings agree on bytes AND on every drop count, across
/// the full cross-product of format, graph selection, statement layer and base.
#[test]
fn streamed_serialization_is_byte_identical_to_eager_for_every_format() {
    for (fixture, dataset) in fixtures() {
        for format in all_formats() {
            for selection in [SerializeGraph::DefaultGraph, SerializeGraph::Dataset] {
                for statement_layer in [
                    StatementLayer::Emit,
                    StatementLayer::Project,
                    StatementLayer::PerFormatCapability,
                ] {
                    for base in [None, Some("https://example.org/base/")] {
                        for (mode, jsonld) in &jsonld_option_modes() {
                            let options = SerializeOptions {
                                selection,
                                statement_layer,
                                jsonld_options: jsonld.as_ref(),
                            };

                            let eager = serialize_dataset_with(&*dataset, format, base, &options);
                            let mut streamed_bytes: Vec<u8> = Vec::new();
                            let streamed = serialize_dataset_to_writer_with(
                                &*dataset,
                                format,
                                base,
                                &options,
                                &mut streamed_bytes,
                            );

                            let label = format!(
                                "{fixture} / {format:?} / {selection:?} / {statement_layer:?} / base={} / {mode}",
                                base.is_some()
                            );

                            match (eager, streamed) {
                                (Ok(eager), Ok(report)) => {
                                    assert_eq!(
                                        eager.bytes, streamed_bytes,
                                        "bytes diverged for {label}"
                                    );
                                    assert_eq!(
                                        report.bytes_written as usize,
                                        streamed_bytes.len(),
                                        "reported byte count disagrees with what was written for {label}"
                                    );
                                    assert_eq!(
                                        (
                                            eager.statement_rows_dropped,
                                            eager.directional_literals_dropped,
                                            eager.named_graph_rows_dropped
                                        ),
                                        (
                                            report.statement_rows_dropped,
                                            report.directional_literals_dropped,
                                            report.named_graph_rows_dropped
                                        ),
                                        "drop counts diverged for {label}"
                                    );
                                }
                                (Err(eager), Err(streamed)) => {
                                    assert_eq!(
                                        eager.to_string(),
                                        streamed.to_string(),
                                        "the two spellings refused differently for {label}"
                                    );
                                }
                                (eager, streamed) => {
                                    panic!(
                                        "one spelling succeeded and the other did not for {label}: \
                                     eager={:?} streamed={:?}",
                                        eager.map(|o| o.bytes.len()),
                                        streamed.map(|r| r.bytes_written)
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The streamed path delivers through a writer that accepts one byte per call,
/// which pins the `write_all` retry loop rather than assuming whole-window writes.
#[test]
fn streaming_survives_a_one_byte_at_a_time_writer() {
    /// Accepts exactly one byte per `write`, the legal minimum.
    struct OneByteAtATime(Vec<u8>);
    impl std::io::Write for OneByteAtATime {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            if let Some(first) = buf.first() {
                self.0.push(*first);
                return Ok(1);
            }
            Ok(0)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let dataset = simple();
    let options = SerializeOptions {
        selection: SerializeGraph::Dataset,
        statement_layer: StatementLayer::PerFormatCapability,
        jsonld_options: None,
    };
    for format in all_formats() {
        let Ok(eager) = serialize_dataset_with(&*dataset, format, None, &options) else {
            continue;
        };
        let mut dribble = OneByteAtATime(Vec::new());
        let report =
            serialize_dataset_to_writer_with(&*dataset, format, None, &options, &mut dribble)
                .unwrap_or_else(|e| panic!("{format:?} streams to a one-byte writer: {e}"));
        assert_eq!(
            report.bytes_written as usize,
            dribble.0.len(),
            "{format:?} miscounted bytes across single-byte writes"
        );
        assert_eq!(
            eager.bytes, dribble.0,
            "{format:?} lost bytes to a short writer"
        );
    }
}

/// The `jsonld_options` axis is not decorative: it reaches a different code path
/// and that path produces a different document.
///
/// The cross-product above compares the eager and streamed spellings cell by cell.
/// Every cell of the new axis would agree even if a configured request were silently
/// ignored — both spellings would ignore it identically, and the suite would report
/// coverage it does not have. This asserts the axis bites, so a regression that
/// dropped the options on the floor fails here rather than passing everywhere.
///
/// The control is the other half: a format that does NOT define these options must
/// refuse them, on BOTH spellings and with the same words. A change that made the
/// configured path universal would satisfy the first assertion and fail this one.
#[test]
fn the_jsonld_options_axis_is_not_decorative() {
    let dataset = simple();

    for format in [NativeRdfFormat::JsonLd, NativeRdfFormat::YamlLd] {
        let rendered: Vec<Vec<u8>> = jsonld_option_modes()
            .iter()
            .map(|(mode, jsonld)| {
                let options = SerializeOptions {
                    selection: SerializeGraph::Dataset,
                    statement_layer: StatementLayer::PerFormatCapability,
                    jsonld_options: jsonld.as_ref(),
                };
                serialize_dataset_with(&*dataset, format, None, &options)
                    .unwrap_or_else(|e| panic!("{format:?} / {mode} serializes: {e}"))
                    .bytes
            })
            .collect();
        assert!(
            rendered.iter().any(|bytes| bytes != &rendered[0]),
            "{format:?} produced identical bytes for every jsonld_options mode, so the \
             axis in the parity cross-product proves nothing about the configured path"
        );
    }

    // The control. Turtle does not define these options and must say so, identically
    // from either spelling — a refusal reaching a caller from only one of them is the
    // same divergence as diverging bytes.
    let configured = JsonLdSerializeOptions::expanded();
    let options = SerializeOptions {
        selection: SerializeGraph::Dataset,
        statement_layer: StatementLayer::PerFormatCapability,
        jsonld_options: Some(&configured),
    };
    let eager = serialize_dataset_with(&*dataset, NativeRdfFormat::Turtle, None, &options)
        .expect_err("Turtle does not define JSON-LD options");
    let mut sink: Vec<u8> = Vec::new();
    let streamed = serialize_dataset_to_writer_with(
        &*dataset,
        NativeRdfFormat::Turtle,
        None,
        &options,
        &mut sink,
    )
    .expect_err("Turtle does not define JSON-LD options");
    assert_eq!(
        eager.to_string(),
        streamed.to_string(),
        "the two spellings refused a configured Turtle request differently"
    );
    assert!(
        sink.is_empty(),
        "the refusal must precede the first emitted byte; {} bytes reached the sink",
        sink.len()
    );
}
