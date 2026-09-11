// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! Profiling harness for the structural slicer.
//!
//! Wall-clock samples from a shared development host are not acceptance
//! evidence, and nothing here asserts one. What fixes behaviour is the crate's
//! test suite and the byte-for-byte goldens beside it; this target exists to
//! collect allocation and profile data on a controlled host, and it measures
//! rather than concludes. No timing, ratio, or speedup is asserted anywhere in
//! this file.
//!
//! Three paths are covered, because the crate offers exactly three and they
//! have genuinely different shapes:
//!
//! * [`analyze`] alone — the dialect walk, the containment closure over the
//!   sections, the split law, one scalar-offset table over the whole text, a
//!   content digest per unit, and the match of every concordance row against
//!   the verses the document carries. This is where the structure is found.
//! * [`render`] over an **already analyzed** model — the projection: one node
//!   identity per section and per unit, a citation node per lift, and the
//!   sorted N-Triples lines of each claim. Benched separately because it is a
//!   pure function of the model and a caller that keeps the model pays for it
//!   only when it wants triples.
//! * [`slice_markdown`] — the two in a row, which is the whole surface a
//!   caller who only wants triples uses.
//!
//! # The document is generated, not sampled
//!
//! The corpus is one function of the constants below and of nothing else: no
//! file is read, no clock and no random-number source is consulted, and every
//! line is a pure function of its own index. The same bytes are generated on
//! every run, every host, and every target — a benchmark whose input drifted
//! between runs would be reporting the input rather than the code.
//!
//! Its shape is chosen to reach the parts of the law that a flat document
//! never would:
//!
//! * **Deep lineage.** Headings at three levels open on a fixed rhythm under
//!   one title, and a movement marker opens under the innermost of them, so
//!   every unit carries a five-deep heading stack.
//! * **Thousands of sections.** At the larger size the rhythm opens a section
//!   for roughly every fourth unit, which is what makes the containment
//!   closure — the parent and the end of each section — a measurable cost
//!   rather than a rounding error.
//! * **The split law.** Every fiftieth unit is written long enough to run past
//!   the profile's byte bound, so it is cut into pieces with an overlapping
//!   continuation, and the concordance still lifts onto the first piece.
//! * **Multi-byte scalars.** Every line carries two-, three-, and four-byte
//!   scalars, so the byte-to-scalar table, the boundary snaps of the split,
//!   and the escaping of every emitted literal all see real UTF-8 rather than
//!   ASCII.
//! * **A concordance with many rows**, one per eight units, some of them
//!   naming verses this document does not carry (a paragraph carries no verse
//!   number, and the last rows reach past the end), one naming every verse a
//!   `u64` can spell, and one too malformed to read. All three are data the
//!   model carries out, not refusals.

use std::fmt::Write as _;
use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use purrdf_markdown::{
    Document, Profile, SourceDocument, Vocabulary, analyze, render, slice_markdown,
};

/// The generated document's IRI. `example.org` is the repository's fixture
/// authority, and nothing here mints a vocabulary of its own.
const DOCUMENT_ID: &str = "https://example.org/doc/generated";
/// The base the profile's vocabulary and node IRIs are derived from.
const SLICE_BASE: &str = "https://example.org/slice/";
/// The declared profile's name.
const PROFILE_NAME: &str = "bench-slice-md-v1";

/// How many units each point of the grid writes.
const SIZES: [usize; 2] = [1_000, 10_000];
/// A level-2 heading opens every this many units.
const CHAPTER_EVERY: usize = 256;
/// A level-3 heading opens every this many units.
const PART_EVERY: usize = 64;
/// A level-4 heading opens every this many units.
const PASSAGE_EVERY: usize = 16;
/// A movement marker opens every this many units. It sits one level under the
/// nearest heading, so the lineage below it is five deep.
const MOVEMENT_EVERY: usize = 8;
/// Every this many units is a two-line paragraph rather than a numbered
/// verse, which is also what leaves gaps in the verse numbering for the
/// concordance to miss.
const PARAGRAPH_EVERY: usize = 7;
/// Every this many units runs past the profile's byte bound and is split.
const OVERSIZE_EVERY: usize = 50;
/// How many lines an oversize unit runs to. Each line is well over forty
/// bytes, so the unit is comfortably past the default two-kilobyte bound.
const OVERSIZE_LINES: usize = 48;
/// One concordance row per this many units.
const ROW_EVERY: usize = 8;
/// How many verses a concordance row's range covers.
const ROW_VERSES: u64 = 3;
/// How many rows name verses wholly past the document's end, lifting nothing.
const ROWS_PAST_THE_END: u64 = 4;

/// The words the generated prose is built from. Small and fixed, so a line is
/// reproducible by hand from its index.
const WORDS: [&str; 12] = [
    "tide", "lantern", "harbour", "ledger", "compass", "rope", "signal", "channel", "beacon",
    "current", "anchor", "chart",
];

/// The declared profile: the default bound and overlap, a vocabulary derived
/// from one base, and no canon base — so a concordance anchor stays a typed
/// literal and nothing is minted from it.
fn profile() -> Profile {
    Profile::new(
        PROFILE_NAME,
        1,
        Vocabulary::under(SLICE_BASE).expect("a vocabulary under an absolute base"),
    )
}

/// One deterministic line of prose: a pure function of its index, carrying a
/// two-byte, a three-byte, and a four-byte scalar.
fn line(index: usize) -> String {
    let a = WORDS[index % WORDS.len()];
    let b = WORDS[(index / 3 + 1) % WORDS.len()];
    let c = WORDS[(index / 5 + 2) % WORDS.len()];
    // U+2014 EM DASH (3 bytes), U+00E9 (2 bytes), U+4E2D (3 bytes),
    // U+1F5FA WORLD MAP (4 bytes).
    format!("the {a} of the {b} \u{2014} a rel\u{e9}ve of the {c}, \u{4e2d}\u{6587} \u{1f5fa}")
}

/// The generated document: `units` verses and paragraphs under a heading
/// rhythm, followed by a concordance table.
fn document(units: usize) -> String {
    let mut text = String::with_capacity(units * 160);
    text.push_str("# A Generated Document for Profiling\n\n");
    for index in 0..units {
        if index.is_multiple_of(CHAPTER_EVERY) {
            let n = index / CHAPTER_EVERY;
            put(&mut text, format_args!("## Chapter {n}\n\n"));
        }
        if index.is_multiple_of(PART_EVERY) {
            let n = index / PART_EVERY;
            put(&mut text, format_args!("### Part {n}\n\n"));
        }
        if index.is_multiple_of(PASSAGE_EVERY) {
            let n = index / PASSAGE_EVERY;
            put(&mut text, format_args!("#### Passage {n}\n\n"));
        }
        if index.is_multiple_of(MOVEMENT_EVERY) {
            let n = index / MOVEMENT_EVERY;
            put(&mut text, format_args!("\u{2042} *movement {n}*\n\n"));
        }
        unit(&mut text, index);
    }
    concordance(&mut text, units);
    text
}

/// One unit: a numbered verse, a two-line paragraph, or a verse long enough
/// that the split law cuts it.
fn unit(text: &mut String, index: usize) {
    if index.is_multiple_of(PARAGRAPH_EVERY) {
        put(
            text,
            format_args!("{}\n{}\n\n", line(index), line(index + 1)),
        );
        return;
    }
    let number = index + 1;
    if index.is_multiple_of(OVERSIZE_EVERY) {
        put(text, format_args!("{number}. {}\n", line(index)));
        for extra in 1..OVERSIZE_LINES {
            put(text, format_args!("{}\n", line(index + extra)));
        }
        text.push('\n');
        return;
    }
    put(text, format_args!("{number}. {}\n\n", line(index)));
}

/// The concordance table: one row per [`ROW_EVERY`] units, then rows naming
/// verses past the end, then one row too malformed to read at all.
fn concordance(text: &mut String, units: usize) {
    text.push_str("## Concordance\n\n| Verses | Canon source | Anchors |\n|---|---|---|\n");
    let rows = (units / ROW_EVERY) as u64;
    let past = units as u64 * 2;
    for row in 0..rows + ROWS_PAST_THE_END {
        let first = if row < rows {
            row * ROW_EVERY as u64 + 1
        } else {
            past + (row - rows) * ROW_VERSES
        };
        let last = first + ROW_VERSES - 1;
        put(
            text,
            format_args!(
                "| {first}\u{2013}{last} | `canon/volume-{row}.md`, `canon/index.md` | `anchor-{row}`, `anchor-{row}-alt` |\n"
            ),
        );
    }
    // A row naming every verse a `u64` can spell. It is lawful, it lifts
    // every verse the document carries, and answering it costs what the
    // document carries rather than what the row claims — so a harness
    // that has to walk the range would never finish this line, which is
    // the point of putting it here rather than only in the vectors.
    put(
        text,
        format_args!(
            "| 1\u{2013}{} | `canon/index.md` | `anchor-every` |\n",
            u64::MAX
        ),
    );
    text.push_str("| overview | `canon/index.md` | `anchor-overview` |\n");
}

/// Appends formatted text. Writing to a `String` is infallible — the only
/// error a `fmt::Write` implementation can raise is one its own `write_str`
/// returns, and `String`'s never does.
fn put(text: &mut String, args: std::fmt::Arguments<'_>) {
    text.write_fmt(args).expect("a String never fails to grow");
}

/// What the generated document turned into, reported so a reader of the
/// samples knows what was measured. Never a threshold.
fn report(units: usize, bytes: usize, model: &Document<'_>) {
    eprintln!(
        "markdown_slicer input units={units} bytes={bytes} sections={} pieces={} citations={} unlifted={} malformed={} deepest_lineage={}",
        model.sections().len(),
        model.units().len(),
        model.citations().len(),
        model.unmatched_citations().count(),
        model.malformed_rows().len(),
        model
            .units()
            .iter()
            .map(|u| u.lineage().len())
            .max()
            .unwrap_or(0),
    );
}

/// The generator's own claims about its output, checked before anything is
/// timed. These are structural, not temporal: a benchmark that quietly stopped
/// generating splits, deep lineages, or unmatched rows would keep reporting
/// numbers for a workload nobody meant to measure.
fn check(units: usize, model: &Document<'_>) {
    assert!(
        model.sections().len() > units / PASSAGE_EVERY,
        "the heading rhythm opens a section for every few units"
    );
    assert!(
        model.units().len() > units,
        "the oversize units split into more pieces than there are units"
    );
    assert!(
        model.units().iter().any(|u| u.continues().is_some()),
        "at least one unit is cut by the split law"
    );
    assert!(
        model.units().iter().any(|u| u.lineage().len() == 5),
        "a unit under a movement carries a five-deep heading stack"
    );
    assert!(
        model.citations().iter().any(|c| !c.lifted().is_empty()),
        "the concordance lifts onto the verses the document carries"
    );
    assert!(
        model.unmatched_citations().count() >= ROWS_PAST_THE_END as usize,
        "the rows past the end lift nothing, and that is data rather than a refusal"
    );
    assert_eq!(
        model.malformed_rows().len(),
        1,
        "the unreadable row is carried out, not dropped"
    );
}

fn benches(c: &mut Criterion) {
    let profile = profile();
    let mut group = c.benchmark_group("markdown_slicer");
    for units in SIZES {
        let text = document(units);
        let doc = SourceDocument {
            id: DOCUMENT_ID,
            bytes: text.as_bytes(),
        };
        let model = analyze(&doc, &profile).expect("the generated document slices");
        report(units, text.len(), &model);
        check(units, &model);

        group.throughput(Throughput::Bytes(text.len() as u64));
        group.bench_function(BenchmarkId::new("analyze", units), |b| {
            b.iter(|| black_box(analyze(black_box(&doc), &profile).expect("slices")));
        });
        group.bench_function(BenchmarkId::new("render", units), |b| {
            b.iter(|| black_box(render(black_box(&model))));
        });
        group.bench_function(BenchmarkId::new("slice_markdown", units), |b| {
            b.iter(|| black_box(slice_markdown(black_box(&doc), &profile).expect("slices")));
        });
    }
    group.finish();
}

criterion_group!(slicer, benches);
criterion_main!(slicer);
