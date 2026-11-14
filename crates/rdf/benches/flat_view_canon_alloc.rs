// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

#![allow(missing_docs)]

//! One-shot allocation probes for flat canonical output and the digest entries.
//!
//! `cargo bench -p purrdf-rdf --bench flat_view_canon_alloc`. Separate from the
//! timed benches so allocator atomics do not perturb timing; no time is measured.
//! Each entry runs inside its own whole-process window over a fixture built in
//! memory before any window opens. The only asserted facts are the digest
//! equalities and the fixture's mean line length.
//!
//! Environment:
//! * `PURRDF_CANON_ALLOC_LINES=<n>` adds one size; above 4,096,000 lines it
//!   needs `PURRDF_CANON_ALLOC_ALLOW_LARGE=1`.
//! * `PURRDF_CANON_ALLOC_PRE_OUTPUT_LARGE=1` also measures the pre-output peak
//!   at that added size (routine sizes always measure it).
//! * `PURRDF_CANON_ALLOC_CALLS=<shape>:<n>` skips the rows and runs only `n`
//!   calls over a sorted fixture of `PURRDF_CANON_ALLOC_CALL_LINES` lines (default
//!   32,768; the `collision` fixture ignores it), so an external instruction
//!   counter can compare `n` calls with none. `<shape>` is `blank_free`, `reifier`
//!   or `collision`, optionally followed by `_digest` (flat SHA-256 digest),
//!   `_prepared` (flat document against a state prepared once, before the calls)
//!   or `_prepared_digest`; with no suffix each call is a flat document run.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use purrdf_alloc_probe::{CountingAllocator, Measurement, WholeProcessWindow};
use purrdf_core::{CanonPhase, CanonicalizationControl, RDFC_V2_CORPUS_POLICY};
use purrdf_rdf::{
    CanonHash, CanonPresentation, CanonRequest, ContentDigest, PreparedCanon, RdfDataset,
    StopSignal, prepare_canon_view, try_blake3_prepared_view, try_canonical_sha256_view,
    try_canonicalize_flat_view, try_canonicalize_prepared_view, try_canonicalize_sha256_view,
    try_digest_prepared_view, try_flat_blake3_view, try_flat_digest_view,
};

#[path = "support/canon_load.rs"]
mod load;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

const ROUTINE_LINES: [usize; 2] = [32_768, 131_072];
const LARGE_REFUSAL_LINES: usize = 4_096_000;
const MEAN_LINE_RANGE: (f64, f64) = (300.0, 332.0);
const HASH: CanonHash = CanonHash::Sha256;

/// Stops at the poll after `n` polls have passed; counts the polls it sees.
#[derive(Debug)]
struct StopAfter(AtomicUsize);

impl StopSignal for StopAfter {
    fn stopped(&self) -> bool {
        self.0
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
                Some(n.saturating_sub(1))
            })
            .expect("the update always supplies a value")
            == 0
    }
}

fn control(stop: Option<&StopAfter>) -> CanonicalizationControl<'_> {
    CanonicalizationControl::new(
        RDFC_V2_CORPUS_POLICY,
        stop.map(|stop| stop as &dyn StopSignal),
    )
}

struct Fixture<'a> {
    shape: &'a str,
    order: load::Order,
    scale: usize,
    lines: usize,
    leaves: usize,
}

/// The identity of one canonical document: its length, line length and digests.
struct Document {
    doc_bytes: usize,
    mean_line: f64,
    sha256: String,
    blake3: String,
}

impl Document {
    fn of(nquads: &str) -> Self {
        Self {
            doc_bytes: nquads.len(),
            mean_line: load::mean_line_len(nquads),
            sha256: ContentDigest::of(nquads.as_bytes()).to_hex(),
            blake3: purrdf_hash::hex::encode(
                purrdf_hash::blake3::hash(nquads.as_bytes()).as_bytes(),
            ),
        }
    }
}

/// Where a pre-output peak was sampled: the stop index and the run's poll total.
struct Sample {
    peak: i64,
    stop_poll: usize,
    total_polls: usize,
}

fn line(entry: &str, f: &Fixture<'_>, d: &Document, m: Measurement, pre: Option<&Sample>) {
    let (peak, poll, total) = pre.map_or_else(
        || ("-".to_owned(), "-".to_owned(), "-".to_owned()),
        |p| {
            (
                p.peak.to_string(),
                p.stop_poll.to_string(),
                p.total_polls.to_string(),
            )
        },
    );
    // A sorted row is the original format byte for byte; an unsorted row names its order.
    let order = match f.order {
        load::Order::Sorted => String::new(),
        load::Order::Unsorted => format!(" order={}", f.order.label()),
    };
    println!(
        "[flat_view_canon_alloc] entry={entry} shape={}{order} scale={} lines={} leaves={} doc_bytes={} mean_line={:.2} allocations={} requested_bytes={} retained_bytes={} peak_working_bytes={} pre_output_peak={peak} pre_output_poll={poll} total_polls={total} sha256={} blake3={}",
        f.shape,
        f.scale,
        f.lines,
        f.leaves,
        d.doc_bytes,
        d.mean_line,
        m.allocations,
        m.requested_bytes,
        m.retained_bytes,
        m.peak_working_bytes,
        d.sha256,
        d.blake3,
    );
}

/// Run `run` under a stop after `n` polls; `Some(phase)` of the first receipt on a
/// refusal, `None` on completion.
fn stopped_phase(
    run: &dyn Fn(&mut CanonicalizationControl<'_>) -> bool,
    n: usize,
) -> Option<CanonPhase> {
    let stop = StopAfter(AtomicUsize::new(n));
    let mut control = control(Some(&stop));
    if run(&mut control) {
        None
    } else {
        Some(control.receipts()[0].phase)
    }
}

/// Peak working bytes of a run stopped at its first `Output`-phase poll.
fn pre_output_peak(run: &dyn Fn(&mut CanonicalizationControl<'_>) -> bool) -> Sample {
    let stop = StopAfter(AtomicUsize::new(usize::MAX));
    assert!(run(&mut control(Some(&stop))), "the quiet run completes");
    let total = usize::MAX - stop.0.load(Ordering::Relaxed);
    // Phases only advance with a later stop, so "past the search phase or
    // complete" is monotone in n.
    let at_output = |n: usize| {
        !matches!(
            stopped_phase(run, n),
            Some(CanonPhase::Preparation | CanonPhase::Search)
        )
    };
    let (mut lo, mut hi) = (0, total);
    assert!(
        at_output(hi),
        "a stop at the total poll count reaches output"
    );
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if at_output(mid) {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    assert_eq!(
        stopped_phase(run, lo),
        Some(CanonPhase::Output),
        "the smallest qualifying stop lands in the Output phase"
    );
    let window = WholeProcessWindow::open();
    let _ = stopped_phase(run, lo);
    Sample {
        peak: window.close().peak_working_bytes,
        stop_poll: lo,
        total_polls: total,
    }
}

fn probe(shape: &str, order: load::Order, scale: usize, lines: usize, with_pre_output: bool) {
    let loaded = match shape {
        "blank_free" => load::blank_free(lines, scale, order),
        _ => load::reifier(lines, scale, order),
    };
    let ds: &RdfDataset = &loaded.dataset;
    let lines = if shape == "reifier" {
        lines / 4 * 4
    } else {
        lines
    };

    let doc_run =
        |c: &mut CanonicalizationControl<'_>| try_canonicalize_flat_view(ds, HASH, c).is_ok();
    let digest_run =
        |c: &mut CanonicalizationControl<'_>| try_flat_digest_view(ds, HASH, c).is_ok();

    let window = WholeProcessWindow::open();
    let mut quiet = control(None);
    let doc = try_canonicalize_flat_view(ds, HASH, &mut quiet).expect("flat document");
    let search_work = quiet.receipts()[0].work.total();
    drop(quiet);
    let doc_measure = window.close();
    // The fixtures give every blank a distinct first-degree identity, so no n-degree
    // search runs; a padding change that collides two blanks would add search work and
    // make this baseline measure something else.
    assert_eq!(
        search_work, 0,
        "{shape}: the fixture must not trigger an n-degree search"
    );
    let flat = Document::of(doc.nquads());
    // The order the walk presents lines to the output stage, observed over the
    // first 4,096 rows before anything else is measured.
    let (descents, pairs) = load::arrival_descents(shape, ds, doc.nquads(), 4096);
    drop(doc);
    println!(
        "[flat_view_canon_alloc] arrival shape={shape} order={} scale={scale} lines={lines} descents={descents} pairs={pairs}",
        order.label()
    );
    if order == load::Order::Unsorted {
        assert!(
            descents * 4 >= pairs,
            "{shape}: an unsorted walk has at least a quarter of its pairs out of order"
        );
    }
    let f = Fixture {
        shape,
        order,
        scale,
        lines,
        leaves: loaded.leaves,
    };
    if scale == 1 {
        println!(
            "[flat_view_canon_alloc] mean_line shape={shape} order={} lines={lines} mean_line={:.2}",
            order.label(),
            flat.mean_line
        );
        assert!(
            (MEAN_LINE_RANGE.0..=MEAN_LINE_RANGE.1).contains(&flat.mean_line),
            "mean line length {} outside {MEAN_LINE_RANGE:?}",
            flat.mean_line
        );
    }
    let pre_doc = with_pre_output.then(|| pre_output_peak(&doc_run));
    line("doc", &f, &flat, doc_measure, pre_doc.as_ref());

    let window = WholeProcessWindow::open();
    let digest = try_flat_digest_view(ds, HASH, &mut control(None)).expect("flat digest");
    let m = window.close();
    assert_eq!(
        digest.to_hex(),
        flat.sha256,
        "flat digest equals the document digest"
    );
    let pre = with_pre_output.then(|| pre_output_peak(&digest_run));
    line("flat_sha256", &f, &flat, m, pre.as_ref());

    let window = WholeProcessWindow::open();
    let doc = try_canonicalize_flat_view(ds, HASH, &mut control(None)).expect("flat document");
    let composed = purrdf_hash::blake3::hash(doc.nquads().as_bytes());
    // The document is released before the window closes, so `retained_bytes` is what
    // the route keeps beyond its digest (comparable with a digest-only route).
    drop(doc);
    let m = window.close();
    assert_eq!(
        purrdf_hash::hex::encode(composed.as_bytes()),
        flat.blake3,
        "composed BLAKE3 equals the document BLAKE3"
    );
    line("flat_blake3_composed", &f, &flat, m, None);

    // The streamed route: the same digest, with no document to build.
    let window = WholeProcessWindow::open();
    let streamed = try_flat_blake3_view(ds, HASH, &mut control(None)).expect("flat BLAKE3");
    let m = window.close();
    assert_eq!(
        purrdf_hash::hex::encode(&streamed),
        flat.blake3,
        "streamed BLAKE3 equals the document BLAKE3"
    );
    let blake3_run =
        |c: &mut CanonicalizationControl<'_>| try_flat_blake3_view(ds, HASH, c).is_ok();
    let pre = with_pre_output.then(|| pre_output_peak(&blake3_run));
    line("flat_blake3_streamed", &f, &flat, m, pre.as_ref());

    // The overlay presentation is a different document on the reifier shape, so its
    // length and digests come from its own result.
    let window = WholeProcessWindow::open();
    let (both, both_digest) =
        try_canonicalize_sha256_view(ds, HASH, &mut control(None)).expect("overlay document");
    let doc_m = window.close();
    let overlay = Document::of(both.nquads());
    assert_eq!(
        both_digest.to_hex(),
        overlay.sha256,
        "the overlay digest equals the digest of the overlay document"
    );
    drop(both);

    let window = WholeProcessWindow::open();
    let overlay_digest =
        try_canonical_sha256_view(ds, HASH, &mut control(None)).expect("overlay digest");
    let m = window.close();
    assert_eq!(
        overlay_digest.to_hex(),
        overlay.sha256,
        "the digest-only overlay entry equals the overlay document's digest"
    );
    line("overlay_sha256", &f, &overlay, m, None);
    line("overlay_doc_sha256", &f, &overlay, doc_m, None);

    prepared_rows(ds, &f, &flat);
}

/// The flat request every prepared row uses: the presentation and label hash of
/// the flat rows above.
const FLAT_REQUEST: CanonRequest<'static> =
    CanonRequest::new(HASH, CanonPresentation::FlatAssertion);

/// The cost of preparing once, and of a second run of each flat output against
/// that product. Each reuse row must give the flat row's bytes and digests.
fn prepared_rows(ds: &RdfDataset, f: &Fixture<'_>, flat: &Document) {
    let window = WholeProcessWindow::open();
    let prepared = prepare_canon_view(ds, FLAT_REQUEST, &mut control(None)).expect("prepare");
    let m = window.close();
    line("prepare", f, flat, m, None);
    println!(
        "[flat_view_canon_alloc] product shape={} order={} scale={} lines={} product_bytes={} product_lines={} product_blanks={} product_bytes_per_line={:.2}",
        f.shape,
        f.order.label(),
        f.scale,
        f.lines,
        prepared.retained_bytes(),
        prepared.line_count(),
        prepared.blank_count(),
        prepared.retained_bytes() as f64 / prepared.line_count().max(1) as f64,
    );

    let window = WholeProcessWindow::open();
    let doc = try_canonicalize_prepared_view(ds, FLAT_REQUEST, &prepared, &mut control(None))
        .expect("prepared document");
    let m = window.close();
    assert_eq!(
        Document::of(doc.nquads()).sha256,
        flat.sha256,
        "the prepared document equals the flat document"
    );
    drop(doc);
    line("prepared_doc", f, flat, m, None);

    let window = WholeProcessWindow::open();
    let digest = try_digest_prepared_view(ds, FLAT_REQUEST, &prepared, &mut control(None))
        .expect("prepared digest");
    let m = window.close();
    assert_eq!(
        digest.to_hex(),
        flat.sha256,
        "the prepared digest equals the flat digest"
    );
    line("prepared_sha256", f, flat, m, None);

    let window = WholeProcessWindow::open();
    let streamed = try_blake3_prepared_view(ds, FLAT_REQUEST, &prepared, &mut control(None))
        .expect("prepared BLAKE3");
    let m = window.close();
    assert_eq!(
        purrdf_hash::hex::encode(&streamed),
        flat.blake3,
        "the prepared BLAKE3 equals the flat BLAKE3"
    );
    line("prepared_blake3", f, flat, m, None);
}

/// Rings of identical blank nodes: every blank collides at first degree, so the
/// n-degree search runs for each of them. Many small rings and one long one.
fn collision() -> RdfDataset {
    use purrdf_rdf::{BlankScope, RdfDatasetBuilder};
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("https://example.org/p").expect("predicate");
    let mut ring = |tag: &str, size: usize| {
        let nodes: Vec<_> = (0..size)
            .map(|i| {
                b.intern_blank(&format!("{tag}_{i}"), BlankScope::DEFAULT)
                    .expect("blank interns")
            })
            .collect();
        for i in 0..size {
            b.push_quad(nodes[i], p, nodes[(i + 1) % size], None)
                .expect("quad pushes");
        }
    };
    for r in 0..500 {
        ring(&format!("s{r}"), 4);
    }
    ring("long", 40);
    let frozen = b.freeze().expect("collision fixture freezes");
    Arc::try_unwrap(frozen).unwrap_or_else(|_| panic!("the fixture is uniquely owned"))
}

/// One row for the collision fixture: allocations and peak of the flat document
/// entry, and the poll total (search work is asserted nonzero).
fn probe_collision() {
    let ds = collision();
    let window = WholeProcessWindow::open();
    let mut quiet = control(None);
    let doc = try_canonicalize_flat_view(&ds, HASH, &mut quiet).expect("flat document");
    let work = quiet.receipts()[0].work.total();
    drop(quiet);
    let m = window.close();
    assert!(
        work > 0,
        "the collision fixture must run an n-degree search"
    );
    let d = Document::of(doc.nquads());
    println!(
        "[flat_view_canon_alloc] entry=doc shape=collision lines={} search_work={work} doc_bytes={} allocations={} requested_bytes={} retained_bytes={} peak_working_bytes={} sha256={} blake3={}",
        doc.nquads().lines().count(),
        d.doc_bytes,
        m.allocations,
        m.requested_bytes,
        m.retained_bytes,
        m.peak_working_bytes,
        d.sha256,
        d.blake3,
    );
    let prepared = prepare_canon_view(&ds, FLAT_REQUEST, &mut control(None)).expect("prepare");
    let window = WholeProcessWindow::open();
    let doc = try_canonicalize_prepared_view(&ds, FLAT_REQUEST, &prepared, &mut control(None))
        .expect("prepared document");
    let m = window.close();
    let again = Document::of(doc.nquads());
    assert_eq!(
        again.sha256, d.sha256,
        "the prepared document equals the flat document"
    );
    println!(
        "[flat_view_canon_alloc] entry=prepared_doc shape=collision lines={} product_bytes={} product_blanks={} allocations={} requested_bytes={} retained_bytes={} peak_working_bytes={} sha256={}",
        prepared.line_count(),
        prepared.retained_bytes(),
        prepared.blank_count(),
        m.allocations,
        m.requested_bytes,
        m.retained_bytes,
        m.peak_working_bytes,
        again.sha256,
    );
}

/// `n` calls and nothing else, for an external instruction counter.
fn calls(spec: &str) {
    let (shape, n) = spec.split_once(':').expect("<shape>:<n>");
    let n: usize = n.parse().expect("n is an integer");
    let lines: usize = std::env::var("PURRDF_CANON_ALLOC_CALL_LINES")
        .map_or(32_768, |raw| raw.parse().expect("a line count"));
    let (fixture, mode) = ["_prepared_digest", "_prepared", "_digest"]
        .into_iter()
        .find_map(|suffix| shape.strip_suffix(suffix).map(|name| (name, suffix)))
        .unwrap_or((shape, ""));
    let ds: Arc<RdfDataset> = match fixture {
        "reifier" => load::reifier(lines, 1, load::Order::Sorted).dataset,
        "blank_free" => load::blank_free(lines, 1, load::Order::Sorted).dataset,
        "collision" => Arc::new(collision()),
        other => panic!("unknown shape {other}"),
    };
    // Prepared once, before the calls, so that `n` calls less none is one run.
    let prepared: Option<PreparedCanon> = mode
        .starts_with("_prepared")
        .then(|| prepare_canon_view(&*ds, FLAT_REQUEST, &mut control(None)).expect("prepare"));
    for _ in 0..n {
        match (mode, &prepared) {
            ("_prepared", Some(state)) => {
                let doc =
                    try_canonicalize_prepared_view(&*ds, FLAT_REQUEST, state, &mut control(None))
                        .expect("prepared document");
                std::hint::black_box(doc);
            }
            ("_prepared_digest", Some(state)) => {
                let digest =
                    try_digest_prepared_view(&*ds, FLAT_REQUEST, state, &mut control(None))
                        .expect("prepared digest");
                std::hint::black_box(digest);
            }
            ("_digest", _) => {
                let digest = try_flat_digest_view(&*ds, HASH, &mut control(None)).expect("digest");
                std::hint::black_box(digest);
            }
            _ => {
                let doc =
                    try_canonicalize_flat_view(&*ds, HASH, &mut control(None)).expect("document");
                std::hint::black_box(doc);
            }
        }
    }
}

fn main() {
    if let Ok(spec) = std::env::var("PURRDF_CANON_ALLOC_CALLS") {
        calls(&spec);
        return;
    }
    let extra = std::env::var("PURRDF_CANON_ALLOC_LINES").ok().map(|raw| {
        let lines: usize = raw.parse().expect("PURRDF_CANON_ALLOC_LINES is an integer");
        assert!(
            lines <= LARGE_REFUSAL_LINES
                || std::env::var("PURRDF_CANON_ALLOC_ALLOW_LARGE").as_deref() == Ok("1"),
            "{lines} lines exceeds {LARGE_REFUSAL_LINES}; set PURRDF_CANON_ALLOC_ALLOW_LARGE=1"
        );
        lines
    });
    let large_pre = std::env::var("PURRDF_CANON_ALLOC_PRE_OUTPUT_LARGE").as_deref() == Ok("1");
    let sizes = ROUTINE_LINES
        .into_iter()
        .map(|n| (n, true))
        .chain(extra.map(|n| (n, large_pre)));
    probe_collision();
    for (lines, pre) in sizes {
        for shape in ["blank_free", "reifier"] {
            for scale in [1, 2] {
                for order in [load::Order::Sorted, load::Order::Unsorted] {
                    probe(shape, order, scale, lines, pre);
                }
            }
        }
    }
}
