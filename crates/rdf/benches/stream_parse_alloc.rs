// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// This bench is a plain `main`, but the workspace `missing_docs` lint applies to its
// items just the same; the reporting helpers below are internal probes, not API.
#![allow(missing_docs)]

//! Peak-allocation evidence for `parse_dataset_from_reader` against `parse_dataset`.
//!
//! Run with `cargo bench -p purrdf-rdf --bench stream_parse_alloc`. It is a plain
//! `main` rather than a Criterion harness because the quantity of interest is a MEMORY
//! high-water mark, not a duration, and a tracking allocator's atomics would perturb
//! any timing measured beside it.
//!
//! # What is being claimed, and what is not
//!
//! Streaming the line-oriented codecs removes the SOURCE-SIDE buffers from peak
//! residency. It does not — and this bench is arranged so it cannot appear to — make
//! the parse constant-memory:
//!
//! * The frozen `RdfDataset` is the OUTPUT. It is proportional to the document's
//!   content on both paths, and it is reported here (`retained_bytes`) precisely so the
//!   peak numbers are read against it rather than mistaken for a total.
//! * The RDF 1.2 statement-layer fold is genuinely two-pass — whether `<r> <p> <v>` is
//!   a base quad or an annotation depends on whether some possibly-later line binds `r`
//!   with `rdf:reifies` — so the row table is resident until the document ends. That is
//!   a property of the format.
//!
//! What streaming removes is (a) the document bytes and (b) the intermediate
//! `Vec<Statement>` the buffered line pipeline materializes before it lowers. Both are
//! proportional to the source, and together they are the difference this bench prints.
//!
//! # Why the streamed arm reads from a generator
//!
//! Reading a `Cursor` over a resident `String` would keep the source alive for the whole
//! parse and measure nothing: the buffer this lane removes would still be there, just
//! owned by the bench. The streamed arm therefore pulls from a reader that SYNTHESIZES
//! the identical document one row at a time, which is what a file or a pipe is from the
//! parser's point of view. The two arms are checked to produce the identical dataset
//! before any number is printed, so the comparison is between two paths to the same
//! result.
//!
//! Report-only. Nothing here is a gate and nothing asserts a bound; the numbers are
//! whatever this machine produced when it ran.

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::io::Read;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use purrdf_rdf::{
    RdfDataset, SerializeGraph, parse_dataset, parse_dataset_from_reader, serialize_dataset,
};

static LIVE_BYTES: AtomicI64 = AtomicI64::new(0);
static PEAK_BYTES: AtomicI64 = AtomicI64::new(0);

fn to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn record_allocation(size: usize) {
    let size = to_i64(size);
    let live = LIVE_BYTES
        .fetch_add(size, Ordering::Relaxed)
        .saturating_add(size);
    let mut peak = PEAK_BYTES.load(Ordering::Relaxed);
    while live > peak {
        match PEAK_BYTES.compare_exchange_weak(peak, live, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(observed) => peak = observed,
        }
    }
}

fn record_deallocation(size: usize) {
    LIVE_BYTES.fetch_sub(to_i64(size), Ordering::Relaxed);
}

struct CountingAllocator;

// SAFETY: every operation delegates to `System` with the exact incoming pointer and
// layout; the atomic accounting does not affect allocator ownership.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: delegated with the caller's exact layout.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            record_allocation(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        record_deallocation(layout.size());
        // SAFETY: delegated with the caller's exact pointer/layout.
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: delegated with the caller's exact pointer/layout/size.
        let fresh = unsafe { System.realloc(pointer, layout, new_size) };
        if !fresh.is_null() {
            record_deallocation(layout.size());
            record_allocation(new_size);
        }
        fresh
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

/// Rows in the fixture. ~120 k rows is ~12 MiB of N-Quads: large enough that the
/// source-sized buffers dominate the noise floor of an allocator counter.
const ROWS: usize = 120_000;

/// One row of the fixture, written into `out`.
///
/// The single definition of the document, shared by the resident-`String` arm and the
/// generator-reader arm, so the two arms cannot be comparing different documents. The
/// shape is deliberately statement-layer-heavy: a reifier is bound in one row and
/// annotated many rows later, so the fold's forward references cross every read
/// boundary.
fn write_row(out: &mut String, index: usize) {
    use std::fmt::Write as _;
    let _ = match index % 4 {
        0 => writeln!(
            out,
            "<https://example.org/s{}> <https://example.org/p{}> \
             <https://example.org/o{}> <https://example.org/g{}> .",
            index % 4_099,
            index % 17,
            index % 4_093,
            index % 11
        ),
        1 => writeln!(
            out,
            "<https://example.org/s{}> <https://example.org/label> \"row {index}\"@en .",
            index % 4_099
        ),
        2 => {
            let reifier = index % 1_021;
            writeln!(
                out,
                "_:r{reifier} <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> \
                 <<( <https://example.org/a{reifier}> <https://example.org/p{}> \
                 <https://example.org/c{reifier}> )>> .",
                reifier % 17
            )
        }
        _ => writeln!(
            out,
            "_:r{} <https://example.org/confidence> \"0.{}\" .",
            (index + 500) % 1_021,
            index % 100
        ),
    };
}

/// The whole fixture as one resident `String`.
fn fixture() -> String {
    let mut out = String::with_capacity(ROWS * 140);
    for index in 0..ROWS {
        write_row(&mut out, index);
    }
    out
}

/// A `Read` that SYNTHESIZES the fixture one row at a time, holding at most one row.
///
/// This is what a file or a pipe looks like to the parser: the bytes exist only as they
/// are pulled. Using it rather than a `Cursor` over a resident buffer is the whole
/// reason this bench measures anything — a `Cursor` would keep the very buffer under
/// test alive for the duration.
struct GeneratedFixtureReader {
    /// The next row index to synthesize, or `ROWS` once the document is exhausted.
    next_row: usize,
    /// The current row's bytes, and how much of it has been handed out.
    row: String,
    consumed: usize,
}

impl GeneratedFixtureReader {
    fn new() -> Self {
        Self {
            next_row: 0,
            row: String::new(),
            consumed: 0,
        }
    }
}

impl Read for GeneratedFixtureReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.consumed == self.row.len() {
            if self.next_row == ROWS {
                return Ok(0);
            }
            self.row.clear();
            write_row(&mut self.row, self.next_row);
            self.next_row += 1;
            self.consumed = 0;
        }
        let remaining = &self.row.as_bytes()[self.consumed..];
        let take = remaining.len().min(buf.len());
        buf[..take].copy_from_slice(&remaining[..take]);
        self.consumed += take;
        Ok(take)
    }
}

/// One arm's measurement.
struct Arm {
    /// The high-water mark of live allocated bytes across the whole parse.
    peak: i64,
    /// The bytes the frozen dataset itself holds, measured by dropping the ONLY
    /// reference to it and reading the counter back. Reported so `peak` is read
    /// against the output rather than mistaken for pure overhead.
    retained: i64,
}

/// Run one parse with the allocator counters zeroed, reporting its peak and the bytes
/// its result retains.
///
/// `body` must return the SOLE reference to the dataset, because the retained figure is
/// obtained by dropping it: a surviving `Arc` clone elsewhere would free nothing and
/// report zero.
fn measure(body: impl FnOnce() -> Arc<RdfDataset>) -> Arm {
    LIVE_BYTES.store(0, Ordering::Relaxed);
    PEAK_BYTES.store(0, Ordering::Relaxed);
    let dataset = body();
    let peak = PEAK_BYTES.load(Ordering::Relaxed);
    let before_drop = LIVE_BYTES.load(Ordering::Relaxed);
    drop(dataset);
    Arm {
        peak,
        retained: before_drop - LIVE_BYTES.load(Ordering::Relaxed),
    }
}

fn main() {
    // Equivalence first: a residency comparison between two paths that disagree about
    // the answer would be meaningless, so prove they agree before printing anything.
    let text = fixture();
    let buffered_check =
        parse_dataset(text.as_bytes(), "application/n-quads", None).expect("buffered parse");
    let streamed_check =
        parse_dataset_from_reader(GeneratedFixtureReader::new(), "application/n-quads", None)
            .expect("streamed parse");
    assert_eq!(
        serialize_dataset(
            &buffered_check,
            "application/n-quads",
            SerializeGraph::Dataset
        )
        .expect("serialize buffered"),
        serialize_dataset(
            &streamed_check,
            "application/n-quads",
            SerializeGraph::Dataset
        )
        .expect("serialize streamed"),
        "the streamed and buffered parses must produce the identical dataset"
    );
    let source_bytes = text.len();
    let quads = buffered_check.quad_count();
    let terms = buffered_check.term_count();
    drop(buffered_check);
    drop(streamed_check);
    drop(text);

    // BUFFERED (the default `Auto` path, which is chunk-parallel above 1 MiB): the
    // caller holds the source for the duration, which is what `read_to_end` then
    // `parse_dataset` does.
    let buffered = measure(|| {
        let text = fixture();
        let dataset =
            parse_dataset(black_box(text.as_bytes()), "application/n-quads", None).expect("parse");
        drop(text);
        dataset
    });

    // BUFFERED, forced sequential. Reported separately so the streamed/buffered
    // difference is not confounded with the chunk-parallel path's concurrent per-chunk
    // statement staging: this arm is the same pipeline the streamed arm runs, differing
    // ONLY in whether the source and the statement list are materialized.
    let sequential = measure(|| {
        let text = fixture();
        let dataset = purrdf_rdf::native_codecs::parse_dataset_forced_sequential(
            black_box(text.as_bytes()),
            "application/n-quads",
            None,
        )
        .expect("parse");
        drop(text);
        dataset
    });

    // STREAMED: the source never exists as a buffer at all.
    let streamed = measure(|| {
        parse_dataset_from_reader(
            black_box(GeneratedFixtureReader::new()),
            "application/n-quads",
            None,
        )
        .expect("parse")
    });

    println!("stream_parse_alloc (report-only; this machine, this run)");
    println!("  fixture: {ROWS} rows, {source_bytes} source bytes → {quads} quads, {terms} terms");
    for (label, arm) in [
        ("buffered parse_dataset (auto: chunk-parallel)", &buffered),
        ("buffered parse_dataset (forced sequential)", &sequential),
        ("streamed parse_dataset_from_reader", &streamed),
    ] {
        println!("  {label}:");
        println!("    peak_allocated_bytes   = {}", arm.peak);
        println!("    retained_dataset_bytes = {}", arm.retained);
    }
    #[allow(clippy::cast_precision_loss)]
    let ratio = |a: i64| a as f64 / source_bytes as f64;
    println!(
        "  peak removed vs sequential-buffered = {} bytes ({:.2}x the source)",
        sequential.peak - streamed.peak,
        ratio(sequential.peak - streamed.peak)
    );
    println!(
        "  NOTE: the dataset is retained on EVERY path and is not bounded by streaming; \
         what streaming removes is the source text and the intermediate statement list. \
         This is not end-to-end constant memory and must not be reported as such."
    );
}
