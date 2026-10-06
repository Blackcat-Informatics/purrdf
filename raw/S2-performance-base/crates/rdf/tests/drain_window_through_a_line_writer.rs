// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The drain window survives the destination it most often meets.
//!
//! `DRAIN_BUFFER_BYTES` is the workspace's one egress constant, and it was chosen to
//! match the ingress buffer rather than measured against anything. The case left
//! unexamined was the commonest destination of all: Rust's `stdout()` is a
//! `LineWriter`, which forwards only up to the LAST NEWLINE in whatever it is handed
//! and buffers the remainder.
//!
//! That interaction is worth checking rather than assuming, because it is the one that
//! could make the window pointless. A line-oriented document — N-Triples, N-Quads —
//! is nothing but newlines, so a `LineWriter` could plausibly have turned each window
//! back into one forward per LINE, which is the syscall pattern the window exists to
//! avoid. Staging 64 KiB only to hand it downstream a line at a time would be the
//! buffer doing no work at all.
//!
//! It does not. `LineWriter` batches to its last newline, so a window arrives as one
//! forward regardless of how many lines it contains. This pins that.

use purrdf_core::sink::DRAIN_BUFFER_BYTES;
use purrdf_core::{RdfDatasetBuilder, RdfLiteral, RdfTerm};
use purrdf_rdf::{
    NativeRdfFormat, SerializeGraph, SerializeOptions, StatementLayer,
    serialize_dataset_to_writer_with,
};
use std::io::{LineWriter, Write};

/// Counts the writes that reach it — a stand-in for the syscall a real stdout makes.
#[derive(Default)]
struct Forwards {
    writes: usize,
    bytes: usize,
}

impl Write for Forwards {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.writes += 1;
        self.bytes += buf.len();
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn a_line_writer_forwards_per_window_not_per_line() {
    let rows = 4_000;
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri("https://example.org/predicate");
    for i in 0..rows {
        let subject =
            builder.intern_owned_term(&RdfTerm::iri(format!("https://example.org/subject/{i}")));
        let object = builder.intern_literal(RdfLiteral::simple(format!(
            "value {i}, padded out so the document spans many drain windows"
        )));
        builder.push_quad(subject, predicate, object, None);
    }
    let dataset = builder.freeze().expect("dataset freezes");

    let mut forwards = Forwards::default();
    {
        // The real shape of `stdout()`: a `LineWriter` in front of the destination.
        let mut line_writer = LineWriter::new(&mut forwards);
        let report = serialize_dataset_to_writer_with(
            &*dataset,
            NativeRdfFormat::NTriples,
            None,
            &SerializeOptions {
                selection: SerializeGraph::Dataset,
                statement_layer: StatementLayer::PerFormatCapability,
                jsonld_options: None,
            },
            &mut line_writer,
        )
        .expect("N-Triples serializes to a line writer");
        line_writer.flush().expect("the line writer flushes");
        assert!(
            report.bytes_written > 8 * DRAIN_BUFFER_BYTES as u64,
            "the fixture must span many windows; wrote {} bytes",
            report.bytes_written
        );
    }

    let windows = forwards.bytes.div_ceil(DRAIN_BUFFER_BYTES);
    assert!(
        windows >= 8,
        "the fixture must span many windows; got {windows}"
    );

    // The bound is derived, not fitted. `LineWriter` splits each window at its LAST
    // newline: the batch up to that point goes downstream, and the tail is held and
    // joins the next window's batch. So a window costs at most two forwards — its own
    // batch and the one carrying its predecessor's tail — plus one for the final
    // flush.
    //
    // Measured on this fixture: 537,780 bytes, 9 windows, 17 forwards, 4,000 rows.
    // That is 2 per window as predicted, and 235x fewer than one per line.
    assert!(
        forwards.writes <= windows * 2 + 1,
        "a {} -byte document in {windows} windows reached the destination in {} \
         forwards. Anything near the {rows}-row count means the line writer is \
         undoing the staging, and the window is buying nothing on the destination it \
         most often meets.",
        forwards.bytes,
        forwards.writes
    );
    assert!(
        forwards.writes * 10 < rows,
        "{} forwards for {rows} rows is within an order of magnitude of per-line, \
         which is the pattern the window exists to avoid",
        forwards.writes
    );
}
