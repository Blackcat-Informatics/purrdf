// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Recognition latency for ordinary scripts, recognized atoms and protected
//! pictographic graphemes outside the finite inventory. Save a harness baseline
//! before changing the predicate to measure the same public path before/after.

use std::time::Duration;

use purrdf_testkit::bench::{Bench, Throughput, bench_group, bench_main, black_box};
use purrdf_text::unicode::is_emoji_grapheme;

fn recognition(bench: &mut Bench) {
    let mut group = bench.benchmark_group("purrdf_text_emoji");
    group
        .sample_size(60)
        .warm_up_time(Duration::from_millis(200))
        .measurement_time(Duration::from_millis(700))
        .throughput(Throughput::Elements(1));
    for (name, text) in [
        ("ascii_scalar", "x"),
        ("ascii_word", "knowledge"),
        ("han_scalar", "中"),
        ("han_word", "知识图谱查询"),
        ("han_supplementary", "𠀀"),
        ("thai_grapheme", "ที่"),
        ("thai_word", "ฐานข้อมูล"),
        ("latin_grapheme", "é"),
        ("latin_word", "mémoire"),
        ("pictograph", "🧠"),
        ("joined_profession", "👩🏽‍💻"),
        ("joined_family", "👨‍👩‍👧‍👦"),
        ("regional_flag", "🇨🇦"),
        ("keycap", "7️⃣"),
        ("text_variation", "©︎"),
        ("skin_component", "🏿"),
        ("unknown_join", "🧠‍🧠"),
        ("prepended_pictograph", "\u{600}🧠"),
        ("nonemoji_keycap_start", "7\u{301}"),
        ("nonemoji_leading_mark", "\u{301}\u{300}"),
    ] {
        group.bench_function(name, |b| {
            b.iter(|| black_box(is_emoji_grapheme(black_box(text))));
        });
    }
    group.finish();
}

bench_group!(benches, recognition);
bench_main!(benches);
