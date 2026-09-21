// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! No single write carries a results document, for any format or any result kind.
//!
//! `serialize_into` is the results half of this crate's streaming surface. Byte parity
//! against the eager spelling cannot establish the property that matters here: both
//! spellings produce identical correct documents whether or not either one streams. The
//! property is that the bytes LEAVE incrementally, and it is only observable by watching
//! the writes.
//!
//! The CONSTRUCT (`Graph`) kind is the one that made this worth testing rather than
//! assuming. It is the only SPARQL answer that is dataset-sized, and the SRJ writer used
//! to render the whole N-Quads document into a `String` and JSON-escape it afterwards —
//! so the sink's window bounded the tail of the pipeline while the middle of it held the
//! graph twice. Every other kind was bounded; that one was decorative, and no assertion
//! anywhere distinguished them.

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_core::sink::DRAIN_BUFFER_BYTES;
use purrdf_core::{RdfDatasetBuilder, RdfLiteral, RdfTerm, SparqlResult, TermValue};
use purrdf_sparql_results::{
    ProvenanceNamespace, ResultProvenance, SparqlResultsFormat, serialize, serialize_into,
};
use std::io::Write;

/// Registered here, not by the crate: `#[global_allocator]` is one per binary.
#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// A writer that remembers how its bytes arrived, not just that they did.
struct Chunks {
    max_write: usize,
    writes: usize,
    bytes: Vec<u8>,
}

impl Write for Chunks {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.max_write = self.max_write.max(buf.len());
        self.writes += 1;
        self.bytes.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Chunks {
    fn new() -> Self {
        Self {
            max_write: 0,
            writes: 0,
            bytes: Vec::new(),
        }
    }
}

/// A CONSTRUCT answer far larger than one drain window.
fn graph_result(rows: usize) -> SparqlResult {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("https://example.org/predicate-with-a-deliberately-long-name");
    for i in 0..rows {
        let s = b.intern_owned_term(&RdfTerm::iri(format!(
            "https://example.org/subject/{i}/with/a/deliberately/long/path/segment"
        )));
        // Quotes and newlines so the JSON escaper is doing real work on every line,
        // not passing a clean run straight through.
        let o = b.intern_literal(RdfLiteral::simple(format!(
            "row {i} carrying \"quoted\" text\nand a newline, padded so the rendered \
             document crosses the staging window many times over"
        )));
        b.push_quad(s, p, o, None);
    }
    SparqlResult::Graph(b.freeze().expect("dataset freezes"))
}

/// A SELECT answer far larger than one drain window.
fn solutions_result(rows: usize) -> SparqlResult {
    SparqlResult::Solutions {
        variables: vec!["subject".to_owned(), "label".to_owned()],
        rows: (0..rows)
            .map(|i| {
                vec![
                    Some(TermValue::Iri(format!(
                        "https://example.org/subject/{i}/with/a/deliberately/long/path"
                    ))),
                    Some(TermValue::Literal {
                        lexical_form: format!(
                            "row {i} with \"quoted\" text, padded out so the document is \
                             far larger than one staging window"
                        ),
                        datatype: "http://www.w3.org/2001/XMLSchema#string".to_owned(),
                        language: None,
                        direction: None,
                    }),
                ]
            })
            .collect(),
        aux: RdfDatasetBuilder::new()
            .freeze()
            .expect("empty aux dataset"),
    }
}

const ALL_FORMATS: [SparqlResultsFormat; 4] = [
    SparqlResultsFormat::Json,
    SparqlResultsFormat::Xml,
    SparqlResultsFormat::Csv,
    SparqlResultsFormat::Tsv,
];

fn stream(result: &SparqlResult, format: SparqlResultsFormat) -> Chunks {
    let provenance = ResultProvenance::default();
    let mut chunks = Chunks::new();
    let outcome = serialize_into(result, format, &provenance, None, &mut chunks)
        .unwrap_or_else(|e| panic!("{format:?} streams: {e}"));
    assert!(
        !outcome.provenance_dropped,
        "{format:?}: nothing was asked to be carried, so nothing may be reported dropped"
    );
    chunks
}

/// Every format, both result kinds: no write exceeds the drain window.
#[test]
fn no_single_write_carries_a_results_document() {
    let cases: [(&str, SparqlResult); 2] = [
        ("solutions", solutions_result(4_000)),
        ("graph", graph_result(4_000)),
    ];

    for (kind, result) in &cases {
        for format in ALL_FORMATS {
            // CSV/TSV carry solutions only; a graph has no tabular spelling and is
            // refused. A refusal is not a streaming failure, and skipping it here is
            // not a gap: `refusals_precede_emission` already proves those refuse
            // before a byte, from both spellings.
            let provenance = ResultProvenance::default();
            if serialize(result, format, &provenance, None).is_err() {
                continue;
            }
            let chunks = stream(result, format);
            assert!(
                chunks.bytes.len() > DRAIN_BUFFER_BYTES,
                "{kind} / {format:?}: the fixture must exceed one drain window or this \
                 proves nothing; produced {} bytes",
                chunks.bytes.len()
            );
            assert!(
                chunks.max_write <= DRAIN_BUFFER_BYTES,
                "{kind} / {format:?}: a single write carried {} bytes, above the \
                 {DRAIN_BUFFER_BYTES}-byte drain window — the document was assembled \
                 before it was written",
                chunks.max_write
            );
            assert!(
                chunks.writes > 1,
                "{kind} / {format:?}: the whole document arrived in one write"
            );
        }
    }
}

/// Streamed bytes equal eager bytes, for both kinds and every format.
///
/// The bound above is satisfiable by a writer that emits garbage in small pieces.
/// This is the other half, and the two together are the claim: the same document,
/// delivered incrementally.
#[test]
fn streamed_results_are_byte_identical_to_eager_results() {
    for (kind, result) in [
        ("solutions", solutions_result(500)),
        ("graph", graph_result(500)),
    ] {
        for format in ALL_FORMATS {
            let provenance = ResultProvenance::default();
            let eager = serialize(&result, format, &provenance, None);
            let mut chunks = Chunks::new();
            let streamed = serialize_into(&result, format, &provenance, None, &mut chunks);
            match (eager, streamed) {
                (Ok(eager), Ok(report)) => {
                    assert_eq!(
                        eager.bytes, chunks.bytes,
                        "{kind} / {format:?}: streamed bytes diverged from eager"
                    );
                    assert_eq!(
                        report.bytes_written as usize,
                        chunks.bytes.len(),
                        "{kind} / {format:?}: reported count disagrees with what was written"
                    );
                }
                (Err(eager), Err(streamed)) => {
                    assert_eq!(
                        eager.to_string(),
                        streamed.to_string(),
                        "{kind} / {format:?}: the two spellings refused differently"
                    );
                }
                (eager, streamed) => panic!(
                    "{kind} / {format:?}: one spelling succeeded and the other did not: \
                     eager={:?} streamed={:?}",
                    eager.map(|o| o.bytes.len()),
                    streamed.map(|r| r.bytes_written)
                ),
            }
        }
    }
}

/// The provenance extension does not reintroduce a whole-document hold.
///
/// It is written after the base body, which is the position a rewind used to occupy
/// (the root `}` was popped and re-emitted). A splice that buffered to find its
/// insertion point would show up here as one large write.
#[test]
fn the_provenance_extension_stays_bounded_too() {
    let namespace = ProvenanceNamespace::new("ex", "https://example.org/provenance#")
        .expect("a well-formed provenance namespace");
    let provenance = ResultProvenance {
        query_hash: Some("a-content-hash-of-the-source-query".to_owned()),
        engine: Some("purrdf".to_owned()),
        solutions: Vec::new(),
    };

    for format in [SparqlResultsFormat::Json, SparqlResultsFormat::Xml] {
        let result = solutions_result(4_000);
        let mut chunks = Chunks::new();
        let outcome = serialize_into(&result, format, &provenance, Some(&namespace), &mut chunks)
            .unwrap_or_else(|e| panic!("{format:?} streams with provenance: {e}"));
        assert!(
            !outcome.provenance_dropped,
            "{format:?} defines the extension and was given a namespace, so nothing \
             may be dropped"
        );
        assert!(
            chunks.max_write <= DRAIN_BUFFER_BYTES,
            "{format:?} with provenance: a single write carried {} bytes",
            chunks.max_write
        );
    }
}

/// The CONSTRUCT arm's RESIDENCY is bounded, which watching writes cannot show.
///
/// This test exists because the bound above does not discriminate for the graph kind,
/// and was confirmed not to: with the previous whole-document spelling restored —
/// render the N-Quads into a `String`, JSON-escape it afterwards — every assertion in
/// `no_single_write_carries_a_results_document` still passed. The sink drains an
/// intermediate `String` through its window in exactly the same bounded pieces as a
/// streamed document, so a writer watching write SIZES is blind to whether anything
/// upstream of the sink is holding the whole answer.
///
/// Residency is the property, so an allocator is the oracle. The window measures the
/// peak live bytes across the serialization; the writer discards, so what is measured
/// is the serializer's own working set and not the test's accumulation.
///
/// `CurrentThreadWindow` is the correct window in a test binary — the harness runs
/// tests concurrently over one global allocator, and a whole-process window here would
/// absorb a sibling's traffic and pass or fail by scheduling.
#[test]
fn the_construct_arm_does_not_hold_the_document_it_is_writing() {
    let result = graph_result(4_000);
    let provenance = ResultProvenance::default();

    // How big the answer actually is, measured outside the window.
    let document = serialize(&result, SparqlResultsFormat::Json, &provenance, None)
        .expect("SRJ carries a CONSTRUCT graph")
        .bytes
        .len();
    assert!(
        document > 8 * DRAIN_BUFFER_BYTES,
        "the fixture must be many windows long or this proves nothing; {document} bytes"
    );

    let window = CurrentThreadWindow::open();
    let outcome = serialize_into(
        &result,
        SparqlResultsFormat::Json,
        &provenance,
        None,
        &mut Discard,
    )
    .expect("SRJ streams a CONSTRUCT graph");
    let measured = window.close();

    assert_eq!(
        outcome.bytes_written as usize, document,
        "the streamed and eager spellings must agree on length"
    );
    // Generous by design: the claim is that peak residency does not TRACK the
    // document, not that it hits a particular number. Holding the rendering — the
    // previous behaviour — puts the peak at or above `document`, so a quarter of it
    // separates the two regimes by a wide margin and does not turn an allocator
    // detail into a failing test.
    assert!(
        measured.peak_working_bytes < (document / 4) as i64,
        "the CONSTRUCT arm peaked at {} live bytes writing a {document}-byte document: \
         residency is tracking the answer, so something upstream of the sink is \
         holding it whole",
        measured.peak_working_bytes
    );
}

/// A writer that keeps nothing, so a measured window sees only the serializer.
struct Discard;

impl Write for Discard {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
