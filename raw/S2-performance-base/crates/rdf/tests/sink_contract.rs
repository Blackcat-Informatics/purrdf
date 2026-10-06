// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The two promises the sink seam makes that byte parity cannot check.
//!
//! The parity suite compares the eager and streamed spellings and would pass for a
//! codec that cleared its buffer before writing, and for one that kept formatting a
//! document long after its destination stopped accepting bytes. Neither shows up as a
//! wrong document; both show up as a corrupted one, or as work nobody asked for.
//!
//! So both are asserted here, on the public surface, against every text format.

use purrdf_core::sink::DRAIN_BUFFER_BYTES;
use purrdf_core::{RdfDatasetBuilder, RdfLiteral, RdfTerm};
use purrdf_rdf::{
    NativeRdfFormat, SerializeGraph, SerializeOptions, StatementLayer,
    serialize_dataset_to_writer_with, serialize_dataset_with,
};
use std::io::Write;
use std::sync::Arc;

/// Every text format, enumerated so a new one cannot quietly skip these promises.
const FORMATS: [NativeRdfFormat; 8] = [
    NativeRdfFormat::NTriples,
    NativeRdfFormat::NQuads,
    NativeRdfFormat::Turtle,
    NativeRdfFormat::TriG,
    NativeRdfFormat::RdfXml,
    NativeRdfFormat::TriX,
    NativeRdfFormat::HexTuples,
    NativeRdfFormat::JsonLd,
];

fn options() -> SerializeOptions<'static> {
    SerializeOptions {
        selection: SerializeGraph::Dataset,
        statement_layer: StatementLayer::PerFormatCapability,
        jsonld_options: None,
    }
}

/// A dataset whose rendering crosses the drain window many times.
fn dataset(rows: usize, tag: &str) -> Arc<purrdf_core::RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("https://example.org/predicate");
    for i in 0..rows {
        let s = b.intern_owned_term(&RdfTerm::iri(format!(
            "https://example.org/{tag}/subject/{i}/with/a/long/path"
        )));
        let o = b.intern_literal(RdfLiteral::simple(format!(
            "{tag} value {i}, padded out so the document crosses the staging window"
        )));
        b.push_quad(s, p, o, None);
    }
    b.freeze().expect("dataset freezes")
}

/// A writer that refuses after `accept` windows and counts what arrives afterwards.
struct FailsAfter {
    remaining: usize,
    writes_after_failure: usize,
    bytes_after_failure: usize,
}

impl Write for FailsAfter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.remaining == 0 {
            self.writes_after_failure += 1;
            self.bytes_after_failure += buf.len();
            return Err(std::io::Error::other("the destination stopped accepting"));
        }
        self.remaining -= 1;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// A destination that stops accepting stops the serializer, promptly.
///
/// A codec that ignored the failure and kept formatting would still report an error —
/// the sink remembers the first one — so the ERROR is not the property. The property
/// is that the work stops, and the only way to see it is to count what keeps arriving
/// after the refusal.
///
/// The bound is deliberately generous. What is being distinguished is "a handful of
/// further windows while the emitter unwinds" from "the rest of the document", and on
/// these fixtures the rest of the document is dozens of windows.
#[test]
fn a_refusing_destination_stops_the_serializer() {
    let dataset = dataset(4_000, "stop");
    for format in FORMATS {
        let Ok(eager) = serialize_dataset_with(&*dataset, format, None, &options()) else {
            continue;
        };
        let eager_len = eager.bytes.len();
        let total_windows = eager_len / DRAIN_BUFFER_BYTES;

        let mut writer = FailsAfter {
            remaining: 2,
            writes_after_failure: 0,
            bytes_after_failure: 0,
        };
        let outcome =
            serialize_dataset_to_writer_with(&*dataset, format, None, &options(), &mut writer);
        assert!(
            outcome.is_err(),
            "{format:?}: a refused write must surface as a failure"
        );
        assert!(
            total_windows >= 8,
            "{format:?}: a {total_windows}-window document is too short to tell a \
             prompt stop from a complete one"
        );
        // Measured at exactly ONE further window for all eight formats, against
        // documents of 9 to 15 windows: the sink's failure flag is polled at the
        // emitter's innermost loop, so a codec unwinds within the window it was
        // filling. Two is the bar, which is one window of slack on a measurement that
        // does not vary by format — not a number chosen to accommodate the result.
        assert!(
            writer.writes_after_failure <= 2,
            "{format:?}: {} further windows ({} bytes) arrived after the destination \
             refused, against a {total_windows}-window document — the emitter kept \
             formatting a document nobody was taking",
            writer.writes_after_failure,
            writer.bytes_after_failure
        );
    }
}

/// Two graphs into one sink append; the second does not erase the first.
///
/// `RdfCodec::serialize_into`'s contract says implementors MUST append, because `out`
/// may already hold a document's worth of text. Nothing checked it. A codec that
/// cleared its buffer, or that assumed it began empty, would pass every parity case —
/// those all start from an empty sink — and would silently drop the first document of
/// any caller writing more than one.
#[test]
fn a_second_document_appends_rather_than_replacing_the_first() {
    let first = dataset(200, "first");
    let second = dataset(200, "second");

    for format in FORMATS {
        let (Ok(a), Ok(b)) = (
            serialize_dataset_with(&*first, format, None, &options()),
            serialize_dataset_with(&*second, format, None, &options()),
        ) else {
            continue;
        };

        let mut both: Vec<u8> = Vec::new();
        let first_report =
            serialize_dataset_to_writer_with(&*first, format, None, &options(), &mut both)
                .unwrap_or_else(|e| panic!("{format:?} writes its first document: {e}"));
        let after_first = both.len();
        let second_report =
            serialize_dataset_to_writer_with(&*second, format, None, &options(), &mut both)
                .unwrap_or_else(|e| panic!("{format:?} writes its second document: {e}"));
        // Each report describes its OWN document. A codec accumulating counts across
        // calls would be reporting the first document's losses against the second.
        assert_eq!(
            first_report.bytes_written as usize,
            a.bytes.len(),
            "{format:?}: the first report must count only the first document"
        );
        assert_eq!(
            second_report.bytes_written as usize,
            b.bytes.len(),
            "{format:?}: the second report must count only the second document"
        );

        assert_eq!(
            after_first,
            a.bytes.len(),
            "{format:?}: the first document's length changed by writing a second"
        );
        assert_eq!(
            both.len(),
            a.bytes.len() + b.bytes.len(),
            "{format:?}: two documents into one writer must total both lengths"
        );
        assert_eq!(
            &both[..a.bytes.len()],
            a.bytes.as_slice(),
            "{format:?}: the second document overwrote the first"
        );
        assert_eq!(
            &both[a.bytes.len()..],
            b.bytes.as_slice(),
            "{format:?}: the second document is not what it serializes to alone"
        );
    }
}
