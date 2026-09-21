// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The `W: Write` serialization seam streams, and is measured doing it.
//!
//! `RdfSerializer::serialize` is the workspace's writer-shaped egress seam — the
//! spelling an abstraction-layer caller reaches instead of naming a format function,
//! and the one a bounded-memory caller picks precisely BECAUSE it takes a writer. It
//! used to build the whole document and `write_all` it, so that caller got the entire
//! document resident anyway. A `W: Write` parameter that buffers is worse than an
//! honest `-> Vec<u8>`: the signature tells the caller the opposite of the truth, and
//! nothing about the call site looks wrong.
//!
//! The property is therefore not "the bytes are right" — the parity suite covers that,
//! and it covered it while the seam still buffered. The property is that **no single
//! write carries the document**, and it is only observable by watching the writes
//! themselves. That is what `Chunks` below does.
//!
//! This is an integration-level check on the public seam, not a unit test of the sink.
//! A sink test pushing `"x".repeat(n)` into a recorder proves the sink staged
//! correctly; it cannot prove a codec reached the sink incrementally rather than
//! handing it one finished document. Only driving a real codec over a real dataset
//! distinguishes those, and they are exactly the two things this change moved between.

use purrdf_core::sink::DRAIN_BUFFER_BYTES;
use purrdf_core::{
    RdfDatasetBuilder, RdfLiteral, RdfSerializeRequest, RdfSerializer, RdfTerm, SerializeGraph,
};
use purrdf_rdf::GtsCodecBackend;
use std::io::Write;
use std::sync::Arc;

/// A writer that remembers how its bytes arrived, not just that they did.
struct Chunks {
    /// The largest single `write` call, which is the number that falls when a
    /// serializer streams and stays document-sized when it does not.
    max_write: usize,
    /// How many `write` calls arrived, so a one-call document is visible as such.
    writes: usize,
    total: usize,
}

impl Write for Chunks {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.max_write = self.max_write.max(buf.len());
        self.writes += 1;
        self.total += buf.len();
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// `rows` quads whose rendering is far larger than one drain window.
fn dataset(rows: usize) -> Arc<purrdf_core::RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("https://example.org/predicate-with-a-deliberately-long-name");
    for i in 0..rows {
        let s = b.intern_owned_term(&RdfTerm::iri(format!(
            "https://example.org/subject/{i}/with/a/deliberately/long/path/segment/run"
        )));
        let o = b.intern_literal(RdfLiteral::simple(format!(
            "a deliberately long literal value for row {i}, padded out so that the \
             rendered document crosses the sink's staging window many times over"
        )));
        b.push_quad(s, p, o, None);
    }
    b.freeze().expect("dataset freezes")
}

fn emit(rows: usize, media_type: &str) -> Chunks {
    let dataset = dataset(rows);
    let mut chunks = Chunks {
        max_write: 0,
        writes: 0,
        total: 0,
    };
    GtsCodecBackend
        .serialize(
            &dataset,
            RdfSerializeRequest {
                media_type,
                graph: SerializeGraph::Dataset,
                base_iri: None,
            },
            &mut chunks,
        )
        .unwrap_or_else(|e| panic!("{media_type} serializes through the backend seam: {e}"));
    chunks
}

/// No single write carries the document, at any size, for every text media type.
#[test]
fn the_backend_seam_never_hands_a_writer_the_whole_document() {
    for media_type in [
        "application/n-triples",
        "application/n-quads",
        "text/turtle",
        "application/trig",
        "application/rdf+xml",
        "application/trix",
        "application/hex+x-ndjson",
        "application/ld+json",
    ] {
        let chunks = emit(4_000, media_type);
        assert!(
            chunks.total > DRAIN_BUFFER_BYTES,
            "{media_type}: the fixture must exceed one drain window or this proves \
             nothing; produced {} bytes against a {DRAIN_BUFFER_BYTES}-byte window",
            chunks.total
        );
        assert!(
            chunks.max_write <= DRAIN_BUFFER_BYTES,
            "{media_type}: a single write carried {} bytes, above the \
             {DRAIN_BUFFER_BYTES}-byte drain window — the seam buffered the document \
             instead of streaming it",
            chunks.max_write
        );
        assert!(
            chunks.writes > 1,
            "{media_type}: the whole document arrived in one write"
        );
    }
}

/// The bound holds as the document grows: writes scale, the largest one does not.
///
/// The assertion above could be satisfied by a seam that buffers everything and then
/// dribbles it out in window-sized pieces — bounded writes, unbounded residency. This
/// is the discriminating half: across a 100x growth in document size the write COUNT
/// has to track the document while `max_write` stays pinned to the window. A buffering
/// implementation cannot make the first quantity grow without the second.
#[test]
fn growing_the_document_grows_the_write_count_and_not_the_write_size() {
    let small = emit(100, "application/n-quads");
    let large = emit(10_000, "application/n-quads");

    assert!(
        large.total > small.total * 50,
        "the fixture must actually grow: {} against {}",
        large.total,
        small.total
    );
    assert!(
        large.writes > small.writes * 10,
        "writes must track the document: {} against {} for a 100x larger document",
        large.writes,
        small.writes
    );
    assert!(
        large.max_write <= DRAIN_BUFFER_BYTES && small.max_write <= DRAIN_BUFFER_BYTES,
        "the largest write must not track the document: {} against {}",
        large.max_write,
        small.max_write
    );
}
