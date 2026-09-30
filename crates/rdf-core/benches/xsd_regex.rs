// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
#![allow(missing_docs)]

//! COLD cost of the shared XSD/XPath `regExp` translator,
//! [`purrdf_core::xsd_regex::compile`] — translation **plus** the underlying
//! `regex` build, with no cache in front of it.
//!
//! A translation step sits in front of every regex compilation used by
//! SHACL `sh:pattern`, SPARQL `REGEX`/`REPLACE` and ShEx `PATTERN`. Those
//! call sites each layer their own cache over `compile`, so
//! the question this target answers is the one those caches cannot: what does
//! one cold translation cost, and does the ordinary ASCII pattern pay for the
//! dialect machinery the exotic ones need?
//!
//! The cases are chosen by *structural* cost, not by popularity:
//!
//! * `plain_ascii` — `^[a-z0-9]+$`, the common real-world case. This is the
//!   row that must not have regressed: the translator should merely copy it.
//! * `unicode_category` — `^\p{L}+$`, a general-category escape passed
//!   through to `regex-syntax` after an allowlist membership test.
//! * `xml_name_escape` — `^\i\c*$`, the expansion-heavy construct: `\i`/`\c`
//!   splice a large enumerated XML-name set in place of two characters.
//! * `xml_name_escape_i` — the same pattern under the `i` flag, which takes
//!   the pre-folded `(?-i:…)` path (the process-wide
//!   folded set is built on first use; the harness's warm-up absorbs that
//!   one-time construction).
//! * `block_first` / `block_last` — `\p{IsBasicLatin}` and the LAST block in
//!   the generated table, `\p{IsSupplementaryPrivateUseArea-B}`. The two
//!   bracket the binary search that replaced a linear scan over 338 rows.
//! * `class_subtraction` — `^[a-z-[aeiou]]+$`, rewritten to the `regex`
//!   crate's `--` set difference.
//! * `x_flag` / `q_flag` — the two flags whose handling is a source rewrite
//!   (`x` strips whitespace before translation; `q` escapes a literal).
//!
//! A second group, `xsd_regex_match`, measures MATCHING: the `regex` crate is
//! built without its literal prefilter, and [`CompiledPattern::is_match`]
//! (`purrdf_core::xsd_regex::CompiledPattern`) runs the required-literal
//! prefilter in front of the engine instead. Each case is timed twice, through
//! `is_match` (`prefiltered/…`) and through the engine alone
//! (`as_regex().is_match`, `engine/…`), so the prefilter's effect is read off
//! one run:
//!
//! * `long_*` — one 64 KiB haystack of lowercase filler with the match in its
//!   last bytes, the case a prefilter exists for: `needle` (a literal),
//!   `foo.*bar` (a literal prefix, then unbounded), `\d{4}-\d{2}` (a required
//!   `-` a bounded distance into the match), `[a-z]+@example\.org` (a
//!   required literal after an unbounded prefix), `NEEDLE` under `i` (the
//!   literal case-folded), and `[a-z]{25}` (no literal at all: the control,
//!   and a pattern the filler never matches, so the engine walks every byte).
//! * `short_*` — 4,096 IRIs of about 70 bytes, half under
//!   `http://example.org/`: `^http://example\.org/` (start-anchored, which the
//!   prefilter leaves to the engine), `example\.org` unanchored, and
//!   `[0-9]{3}$` (the control).
//!
//! Report-only, `cargo bench -p purrdf-core --bench xsd_regex` (the
//! `make bench` lane) — excluded from `make check`. No timing is asserted.

use purrdf_core::xsd_regex::compile;
use purrdf_testkit::bench::{Bench, BenchmarkId, Throughput, bench_group, bench_main, black_box};
use purrdf_testkit::rng::SplitMix64;
use purrdf_testkit::text::lowercase_filler as filler;

/// `(case name, pattern source, flag string)` for every measured shape.
const CASES: &[(&str, &str, &str)] = &[
    ("plain_ascii", r"^[a-z0-9]+$", ""),
    ("unicode_category", r"^\p{L}+$", ""),
    ("xml_name_escape", r"^\i\c*$", ""),
    ("xml_name_escape_i", r"^\i\c*$", "i"),
    ("block_first", r"\p{IsBasicLatin}", ""),
    ("block_last", r"\p{IsSupplementaryPrivateUseArea-B}", ""),
    ("class_subtraction", r"^[a-z-[aeiou]]+$", ""),
    ("x_flag", "a b", "x"),
    ("q_flag", "a.c", "q"),
];

fn bench_xsd_regex_compile(c: &mut Bench) {
    // Untimed sanity pass: every case must compile, or its timed closure would
    // measure the (much cheaper) error path and a correctness regression would
    // read as a speed win. This also initializes the `i`-flag folded sets, so
    // the timed iterations never pay a one-time construction.
    for &(label, pattern, flags) in CASES {
        compile(pattern, flags)
            .unwrap_or_else(|error| panic!("benchmark case {label} must compile: {error}"));
    }

    let mut group = c.benchmark_group("xsd_regex_compile");
    for &(label, pattern, flags) in CASES {
        group.bench_function(label, |bencher| {
            bencher.iter(|| {
                // `black_box` the inputs so the compiler cannot const-fold a
                // translation of a literal it can see; `black_box` the output
                // so it cannot discard the whole compile as dead.
                let compiled = compile(black_box(pattern), black_box(flags))
                    .expect("benchmark pattern compiles");
                black_box(compiled);
            });
        });
    }
    group.finish();
}

/// The bytes every `long_*` pattern but the control matches in.
const MATCH_TAIL: &str = " foo needle bar 2026-09 user@example.org";

/// `(case name, pattern source, flag string, matches)` over the 64 KiB haystack.
const LONG_CASES: &[(&str, &str, &str, bool)] = &[
    ("long_literal", "needle", "", true),
    ("long_prefix_unbounded", "foo.*bar", "", true),
    ("long_inner_bounded", r"\d{4}-\d{2}", "", true),
    ("long_inner_unbounded", r"[a-z]+@example\.org", "", true),
    ("long_literal_i", "NEEDLE", "i", true),
    ("long_control_no_literal", "[a-z]{25}", "", false),
];

/// `(case name, pattern source)` over the 4,096 short IRIs.
const SHORT_CASES: &[(&str, &str)] = &[
    ("short_anchored_prefix", r"^http://example\.org/"),
    ("short_unanchored_literal", r"example\.org"),
    ("short_control_no_literal", r"[0-9]{3}$"),
];

fn bench_xsd_regex_match(c: &mut Bench) {
    let mut rng = SplitMix64::new(0x0047_2600_BE4C_0001);
    let mut long = filler(&mut rng, 64 * 1024);
    long.push_str(MATCH_TAIL);
    let iris: Vec<String> = (0..4096)
        .map(|i| {
            let host = if i % 2 == 0 {
                "example.org"
            } else {
                "exemplar.net"
            };
            format!(
                "http://{host}/{}/{i}",
                filler(&mut rng, 40).replace(' ', "/")
            )
        })
        .collect();

    let mut group = c.benchmark_group("xsd_regex_match");
    group.throughput(Throughput::Bytes(long.len() as u64));
    for &(label, pattern, flags, matches) in LONG_CASES {
        let compiled = compile(pattern, flags)
            .unwrap_or_else(|error| panic!("benchmark case {label} must compile: {error}"));
        // Untimed sanity: both paths give the case's answer, or a timed row
        // would measure a different question.
        assert_eq!(compiled.is_match(&long), matches, "{label}");
        assert_eq!(compiled.as_regex().is_match(&long), matches, "{label}");
        group.bench_function(BenchmarkId::new("prefiltered", label), |bencher| {
            bencher.iter(|| black_box(compiled.is_match(black_box(&long))));
        });
        group.bench_function(BenchmarkId::new("engine", label), |bencher| {
            bencher.iter(|| black_box(compiled.as_regex().is_match(black_box(&long))));
        });
    }
    let short_bytes: usize = iris.iter().map(String::len).sum();
    group.throughput(Throughput::Bytes(short_bytes as u64));
    for &(label, pattern) in SHORT_CASES {
        let compiled = compile(pattern, "")
            .unwrap_or_else(|error| panic!("benchmark case {label} must compile: {error}"));
        let expected = iris
            .iter()
            .filter(|iri| compiled.as_regex().is_match(iri))
            .count();
        assert_eq!(
            iris.iter().filter(|iri| compiled.is_match(iri)).count(),
            expected,
            "{label}"
        );
        group.bench_function(BenchmarkId::new("prefiltered", label), |bencher| {
            bencher.iter(|| {
                black_box(
                    iris.iter()
                        .filter(|iri| compiled.is_match(black_box(iri)))
                        .count(),
                )
            });
        });
        group.bench_function(BenchmarkId::new("engine", label), |bencher| {
            bencher.iter(|| {
                black_box(
                    iris.iter()
                        .filter(|iri| compiled.as_regex().is_match(black_box(iri)))
                        .count(),
                )
            });
        });
    }
    group.finish();
}

bench_group!(benches, bench_xsd_regex_compile, bench_xsd_regex_match);
bench_main!(benches);
