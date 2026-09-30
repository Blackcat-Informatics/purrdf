// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
#![allow(missing_docs)]

//! Profiling harness for the analyzer's Unicode layer: case folding,
//! normalization, word segmentation and the whole analysis form, each over four
//! classes of text with different costs.
//!
//! * **ASCII** — the eight-bytes-at-a-time bypass carries it, so the pipeline
//!   runs for no character;
//! * **Latin** — mostly ASCII with accented letters, so the bypass is broken
//!   often and short runs go through the pipeline;
//! * **mixed** — Greek, Cyrillic, Arabic, Devanagari, combining marks, emoji
//!   and fullwidth forms, all through the pipeline;
//! * **CJK** — Han, Kana and Hangul, where the syllables decompose by
//!   arithmetic and recompose.
//!
//! Report-only: nothing here asserts a timing; behaviour is fixed by the test
//! suite.

use purrdf_testkit::bench::{Bench, Throughput, bench_group, bench_main, black_box};
use purrdf_text::unicode;

/// Repeat `unit` until the text is at least `bytes` long.
fn corpus(unit: &str, bytes: usize) -> String {
    let mut out = String::with_capacity(bytes + unit.len());
    while out.len() < bytes {
        out.push_str(unit);
    }
    out
}

fn classes() -> [(&'static str, String); 4] {
    const BYTES: usize = 64 * 1024;
    [
        (
            "ascii",
            corpus(
                "The Quick Brown Fox jumps over the lazy dog; 3.14 and 2,718 don't. ",
                BYTES,
            ),
        ),
        (
            "latin",
            corpus(
                "Straße Café naïve résumé ÉCOLE Ångström façade crème brûlée Øresund. ",
                BYTES,
            ),
        ),
        (
            "mixed",
            corpus(
                "σοφός ΣΟΦΟΣ Москва عربي नमस्ते e\u{301}\u{323} ｒｕｓｔ ﬁle 👩\u{200d}💻 🇨🇦 Ⅻ ",
                BYTES,
            ),
        ),
        (
            "cjk",
            corpus(
                "中文全文検索 私はサンドイッチを食べます 한국어 전문 검색 ",
                BYTES,
            ),
        ),
    ]
}

fn unicode_layer(criterion: &mut Bench) {
    let mut group = criterion.benchmark_group("purrdf_text_unicode");
    let mut scratch = String::new();
    for (class, text) in classes() {
        group.throughput(Throughput::Bytes(text.len() as u64));
        group.bench_function(format!("case_fold/{class}"), |b| {
            b.iter(|| black_box(unicode::case_fold(black_box(&text))));
        });
        group.bench_function(format!("nfc/{class}"), |b| {
            b.iter(|| black_box(unicode::nfc(black_box(&text))));
        });
        group.bench_function(format!("nfkd/{class}"), |b| {
            b.iter(|| black_box(unicode::nfkd(black_box(&text))));
        });
        group.bench_function(format!("analysis_form/{class}"), |b| {
            b.iter(|| {
                scratch.clear();
                unicode::analysis_form(black_box(&text), &mut scratch);
                black_box(scratch.len())
            });
        });
        group.bench_function(format!("is_in_analysis_form/{class}"), |b| {
            b.iter(|| {
                let mut compare = unicode::Compare::new(black_box(&text));
                unicode::analysis_form(black_box(&text), &mut compare);
                black_box(compare.finish())
            });
        });
        group.bench_function(format!("word_indices/{class}"), |b| {
            b.iter(|| black_box(unicode::word_indices(black_box(&text)).count()));
        });
    }
    group.finish();
}

bench_group!(benches, unicode_layer);
bench_main!(benches);
