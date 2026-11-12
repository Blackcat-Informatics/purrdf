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

use std::sync::atomic::{AtomicUsize, Ordering};

use purrdf_alloc_probe::{CountingAllocator, Measurement, WholeProcessWindow};
use purrdf_core::{CanonPhase, CanonicalizationControl, RDFC_V2_CORPUS_POLICY};
use purrdf_rdf::{
    CanonHash, ContentDigest, RdfDataset, StopSignal, try_canonical_sha256_view,
    try_canonicalize_flat_view, try_canonicalize_sha256_view, try_flat_digest_view,
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
    scale: usize,
    lines: usize,
    leaves: usize,
    doc_bytes: usize,
    mean_line: f64,
    sha256: String,
    blake3: String,
}

fn line(entry: &str, f: &Fixture<'_>, m: Measurement, pre_output_peak: Option<i64>) {
    let pre = pre_output_peak.map_or_else(|| "-".to_owned(), |peak| peak.to_string());
    println!(
        "[flat_view_canon_alloc] entry={entry} shape={} scale={} lines={} leaves={} doc_bytes={} mean_line={:.2} allocations={} requested_bytes={} retained_bytes={} peak_working_bytes={} pre_output_peak={pre} sha256={} blake3={}",
        f.shape,
        f.scale,
        f.lines,
        f.leaves,
        f.doc_bytes,
        f.mean_line,
        m.allocations,
        m.requested_bytes,
        m.retained_bytes,
        m.peak_working_bytes,
        f.sha256,
        f.blake3,
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
fn pre_output_peak(run: &dyn Fn(&mut CanonicalizationControl<'_>) -> bool) -> i64 {
    let stop = StopAfter(AtomicUsize::new(usize::MAX));
    assert!(run(&mut control(Some(&stop))), "the quiet run completes");
    let total = usize::MAX - stop.0.load(Ordering::Relaxed);
    // Phases only advance with a later stop, so "past the search phase or
    // complete" is monotone in n.
    let at_output = |n: usize| {
        stopped_phase(run, n) != Some(CanonPhase::Preparation)
            && stopped_phase(run, n) != Some(CanonPhase::Search)
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
    window.close().peak_working_bytes
}

fn probe(shape: &str, scale: usize, lines: usize, with_pre_output: bool) {
    let loaded = match shape {
        "blank_free" => load::blank_free(lines, scale),
        _ => load::reifier(lines, scale),
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
    let doc = try_canonicalize_flat_view(ds, HASH, &mut control(None)).expect("flat document");
    let doc_measure = window.close();
    let f = Fixture {
        shape,
        scale,
        lines,
        leaves: loaded.leaves,
        doc_bytes: doc.nquads().len(),
        mean_line: load::mean_line_len(doc.nquads()),
        sha256: ContentDigest::of(doc.nquads().as_bytes()).to_hex(),
        blake3: purrdf_hash::hex::encode(
            purrdf_hash::blake3::hash(doc.nquads().as_bytes()).as_bytes(),
        ),
    };
    drop(doc);
    if scale == 1 {
        println!(
            "[flat_view_canon_alloc] mean_line shape={shape} lines={lines} mean_line={:.2}",
            f.mean_line
        );
        assert!(
            (MEAN_LINE_RANGE.0..=MEAN_LINE_RANGE.1).contains(&f.mean_line),
            "mean line length {} outside {MEAN_LINE_RANGE:?}",
            f.mean_line
        );
    }
    let pre_doc = with_pre_output.then(|| pre_output_peak(&doc_run));
    line("doc", &f, doc_measure, pre_doc);

    let window = WholeProcessWindow::open();
    let digest = try_flat_digest_view(ds, HASH, &mut control(None)).expect("flat digest");
    let m = window.close();
    assert_eq!(
        digest.to_hex(),
        f.sha256,
        "flat digest equals the document digest"
    );
    let pre = with_pre_output.then(|| pre_output_peak(&digest_run));
    line("flat_sha256", &f, m, pre);

    let window = WholeProcessWindow::open();
    let doc = try_canonicalize_flat_view(ds, HASH, &mut control(None)).expect("flat document");
    let composed = purrdf_hash::blake3::hash(doc.nquads().as_bytes());
    let m = window.close();
    drop(doc);
    assert_eq!(
        purrdf_hash::hex::encode(composed.as_bytes()),
        f.blake3,
        "composed BLAKE3 equals the document BLAKE3"
    );
    line("flat_blake3_composed", &f, m, None);

    let window = WholeProcessWindow::open();
    let overlay = try_canonical_sha256_view(ds, HASH, &mut control(None)).expect("overlay digest");
    let m = window.close();
    std::hint::black_box(overlay);
    line("overlay_sha256", &f, m, None);

    let window = WholeProcessWindow::open();
    let both =
        try_canonicalize_sha256_view(ds, HASH, &mut control(None)).expect("overlay document");
    let m = window.close();
    drop(both);
    line("overlay_doc_sha256", &f, m, None);
}

fn main() {
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
    for (lines, pre) in sizes {
        for shape in ["blank_free", "reifier"] {
            for scale in [1, 2] {
                probe(shape, scale, lines, pre);
            }
        }
    }
}
