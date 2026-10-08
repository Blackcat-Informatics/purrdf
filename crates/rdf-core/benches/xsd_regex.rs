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
//! * `literal` — `needle`, a pattern that is literal text: its match plan is
//!   the literal's, found without the engine, so the engine is not built until
//!   a caller asks for it.
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
//!   `http://example.org/`: `^http://example\.org/` (start-anchored: its
//!   literal prefix is compared with the IRI's first bytes and the engine never
//!   runs), `example\.org` unanchored (a haystack shorter than one sixty-four-
//!   byte block, where the search's per-call cost shows), and `[0-9]{3}$`
//!   (end-anchored: three digit positions compared with the last bytes).
//!
//! Report-only, `cargo bench -p purrdf-core --bench xsd_regex` (the
//! `make bench` lane) — excluded from `make check`. No timing is asserted.
//!
//! The `native_xpath_*` groups measure the explicitly dated native program:
//! cold compilation of the same grammar shapes, capture-producing matching,
//! and replacement. Backreferences, nullable repetitions and finite work
//! refusal have their own cases. All expectations are checked before timing;
//! the compatibility and native programs retain their distinct contracts.

use purrdf_core::xsd_regex::{compile, xpath};
use purrdf_testkit::bench::{Bench, BenchmarkId, Throughput, bench_group, bench_main, black_box};
use purrdf_testkit::rng::SplitMix64;
use purrdf_testkit::text::lowercase_filler as filler;

/// `(case name, pattern source, flag string)` for every measured shape.
const CASES: &[(&str, &str, &str)] = &[
    ("plain_ascii", r"^[a-z0-9]+$", ""),
    ("literal", "needle", ""),
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

fn bench_native_xpath_compile(c: &mut Bench) {
    let mut group = c.benchmark_group("native_xpath_compile");
    for (profile, name) in [
        (xpath::Profile::Xpath20, "xpath20"),
        (xpath::Profile::Xpath31, "xpath31"),
    ] {
        for &(label, pattern, flags) in CASES {
            if profile == xpath::Profile::Xpath20 && flags == "q" {
                continue;
            }
            xpath::compile(profile, pattern, flags, xpath::Limits::new())
                .expect("native benchmark pattern compiles");
            group.bench_function(BenchmarkId::new(name, label), |bencher| {
                bencher.iter(|| {
                    black_box(
                        xpath::compile(
                            black_box(profile),
                            black_box(pattern),
                            black_box(flags),
                            xpath::Limits::new(),
                        )
                        .expect("native benchmark pattern compiles"),
                    );
                });
            });
        }
    }
    group.finish();
}

fn bench_native_xpath_execute(c: &mut Bench) {
    let mut group = c.benchmark_group("native_xpath_execute");
    let limits = xpath::Limits::new();
    for (label, source, input, expected) in [
        ("literal", "needle", "hay needle stack", true),
        ("plain_ascii", "^[a-z0-9]+$", "abc123", true),
        ("unicode_category", r"^\p{L}+$", "é𐀀", true),
        ("class_subtraction", "^[a-z-[aeiou]]+$", "rhythm", true),
        ("backreference", r"^([a-z]+)-\1$", "repeated-repeated", true),
        (
            "backreference_negative",
            r"^([a-z]+)-\1$",
            "repeated-repeatea",
            false,
        ),
        (
            "capture_repetition",
            "^(a|b){1,16}$",
            "abababababababab",
            true,
        ),
        ("nullable", "^(a?)*b$", "aaaaab", true),
    ] {
        let program = xpath::compile(xpath::Profile::Xpath31, source, "", limits).unwrap();
        assert_eq!(
            program.is_match(input, limits).unwrap(),
            expected,
            "{label}"
        );
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_function(label, |bencher| {
            bencher.iter(|| black_box(program.find(black_box(input), limits).unwrap()));
        });
    }
    let source = xpath::compile(xpath::Profile::Xpath31, "(a|b)", "", limits).unwrap();
    let input = "abab".repeat(64);
    assert_eq!(
        source.replace_all(&input, "$1$1", limits).unwrap(),
        "aabbaabb".repeat(64)
    );
    group.throughput(Throughput::Bytes(input.len() as u64));
    group.bench_function("replacement", |bencher| {
        bencher.iter(|| {
            black_box(
                source
                    .replace_all(black_box(&input), black_box("$1$1"), limits)
                    .unwrap(),
            )
        });
    });
    let program = xpath::compile(xpath::Profile::Xpath31, "(a?){1000}", "", limits).unwrap();
    let withheld = limits.with(xpath::Resource::MatchSteps, 100);
    assert!(program.find("", withheld).unwrap_err().is_operational());
    group.throughput(Throughput::Elements(1));
    group.bench_function("work_refusal", |bencher| {
        bencher.iter(|| black_box(program.find(black_box(""), withheld).unwrap_err()));
    });
    group.finish();
}

/// Ordinary large inputs at the production default limits.
///
/// `run/*` and `general/*` match the same 32 KiB run of letters: `[a-z]+`
/// scans it once with one pending state for every shorter stop, while
/// `([a-z])+` repeats a capturing body and so keeps one pending state per
/// iteration. `run_128k` is a run longer than the default pending-state bound.
/// `search/*` look for an absent needle in 1 MiB of filler. `literal` and
/// `choice` skip every start whose character cannot begin a match, `anchored`
/// tries only the first start, and `leading_run` resumes after the run a
/// failed start already covered. `no_lead` hides those facts behind an empty
/// group, so the program runs from every start: the cost they avoid.
///
/// `adversary/*` are the shapes the backtracking machine alone refused far
/// below these sizes, each over 1 MiB at the production defaults: searches
/// with several unbounded runs that never match, repetitions of groups over
/// prose and over `ab` pairs, and a nested nullable repetition. Every one now
/// runs on the linear-time machines after a bounded backtracking attempt;
/// `adversary/nested_nullable` is the 41-byte input that was refused.
///
/// `counted/*` are the counted repetitions the thread machine refused at
/// these sizes, where a group repeated from every start kept one thread per
/// distinct count: each search finds no match, through `find` (a reverse scan
/// marking match starts) and through `is_match` (a forward scan), and the
/// set machine keeps every repetition's counts as one set. In
/// `counted/exact_100000` the set of live counts grows at every position of
/// the forward scan, so no state repeats and each position costs a closure.
/// `walk/*` follow one start as one thread: `ambiguous_1_1000` over a
/// 2,001-byte match whose iterations can end anywhere, `replace_runs` over
/// thousands of such matches in a mebibyte, and `empty_first_huge` below a
/// minimum beyond `u64` whose body prefers the empty string. `scale/*` are
/// sixty-four mebibytes of prose without a match and matched whole.
/// `refused/backreference_blowup` is the time the step bound allows the
/// exponential exploration a backreference can still reach.
fn bench_native_xpath_large(c: &mut Bench) {
    let mut group = c.benchmark_group("native_xpath_large");
    let limits = xpath::Limits::new();
    let compiled = |source: &str| {
        xpath::compile(xpath::Profile::Xpath31, source, "", limits)
            .unwrap_or_else(|error| panic!("{source}: {error}"))
    };
    let letters = "abcdefghijklmnopqrstuvwxyz".repeat(5042);
    let short = &letters[..32 * 1024];
    for (label, source, input) in [
        ("run/32k", "[a-z]+", short),
        ("general/32k", "([a-z])+", short),
        ("run_128k", "[a-z]+", letters.as_str()),
    ] {
        let program = compiled(source);
        let span = program.find(input, limits).unwrap().unwrap().get(0);
        assert_eq!(span, Some(0..input.len()), "{label}");
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_function(label, |bencher| {
            bencher.iter(|| black_box(program.find(black_box(input), limits).unwrap()));
        });
    }
    let mut rng = SplitMix64::new(0x0047_2600_BE4C_0002);
    let haystack = filler(&mut rng, 1024 * 1024);
    for (label, source) in [
        ("search/literal", "needle"),
        ("search/choice", "needle|haystack"),
        ("search/anchored", "^needle"),
        ("search/leading_run", ".*needle"),
        ("search/no_lead", "(?:)needle"),
    ] {
        let program = compiled(source);
        assert!(!program.is_match(&haystack, limits).unwrap(), "{label}");
        group.throughput(Throughput::Bytes(haystack.len() as u64));
        group.bench_function(label, |bencher| {
            bencher.iter(|| black_box(program.is_match(black_box(&haystack), limits).unwrap()));
        });
    }
    let text = purrdf_testkit::text::word_prose(1024 * 1024);
    let words = text.trim_end();
    let pairs = "ab".repeat(512 * 1024);
    let nested = format!("{}b", "a".repeat(40));
    for (label, source, input, matched) in [
        ("adversary/multi_run", "node.*graph.*zzz", words, false),
        ("adversary/two_run", "alpha.*zzz", words, false),
        ("adversary/word_group", "^([a-z]+ ?)+$", words, true),
        ("adversary/word_space_group", r"^(\w+\s)*\w+$", words, true),
        ("adversary/choice_group", "^(a|b)*$", pairs.as_str(), true),
        ("adversary/pair_group", "^(ab)*$", pairs.as_str(), true),
        (
            "adversary/pair_noncapturing",
            "^(?:ab)*$",
            pairs.as_str(),
            true,
        ),
        (
            "adversary/nested_nullable",
            "^(a|aa)*$|^(a*)*b$",
            nested.as_str(),
            true,
        ),
    ] {
        let program = compiled(source);
        let found = program.find(input, limits).unwrap();
        assert_eq!(found.is_some(), matched, "{label}");
        // The compatibility engine gives the same whole-match answer.
        let compatibility = compile(source, "").unwrap();
        assert_eq!(
            found.and_then(|captures| captures.get(0)),
            compatibility
                .as_regex()
                .find(input)
                .map(|span| span.range()),
            "{label}"
        );
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_function(label, |bencher| {
            bencher.iter(|| black_box(program.find(black_box(input), limits).unwrap()));
        });
    }
    let random_ab = |bytes: usize, seed: u64| -> String {
        purrdf_testkit::rng::splitmix64_bytes(bytes, seed)
            .iter()
            .map(|byte| if byte & 1 == 0 { 'a' } else { 'b' })
            .collect()
    };
    let pairs_128k = "ab".repeat(64 * 1024);
    let quads = "abcd".repeat(256 * 1024);
    let ab_400k = random_ab(400 * 1024, 0x0047_2600_BE4C_0003);
    let ab_800k = random_ab(800 * 1024, 0x0047_2600_BE4C_0004);
    let ab_4m = random_ab(4 << 20, 0x0047_2600_BE4C_0005);
    let prose_4m = purrdf_testkit::text::word_prose(4 << 20);
    let prose_8m = purrdf_testkit::text::word_prose(8 << 20);
    for (label, source, input) in [
        ("counted/pair_1_1000", "(ab){1,1000}c", pairs_128k.as_str()),
        ("counted/pair_2_50", "(ab){2,50}c", pairs.as_str()),
        ("counted/pair_1_100", "(ab){1,100}c", pairs.as_str()),
        ("counted/choice_pair_1_20", "(ab|cd){1,20}e", quads.as_str()),
        ("counted/nested_3_5_9", "((a|b){3}){5,9}c", ab_400k.as_str()),
        ("counted/nested_2_2_5", "((a|b){2}){2,5}c", ab_800k.as_str()),
        ("counted/choice_1_30", "(a|b){1,30}c", ab_4m.as_str()),
        ("counted/choice_3_9", "(a|b){3,9}c", ab_4m.as_str()),
        (
            "counted/word_space_3_5",
            r"(\w+\s){3,5}zzz",
            prose_4m.as_str(),
        ),
        (
            "counted/multi_run_8m",
            "node.*graph.*zzz",
            prose_8m.as_str(),
        ),
    ] {
        let program = compiled(source);
        assert!(program.find(input, limits).unwrap().is_none(), "{label}");
        assert!(!program.is_match(input, limits).unwrap(), "{label}");
        // The compatibility engine gives the same answer.
        assert!(!compile(source, "").unwrap().is_match(input), "{label}");
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_function(label, |bencher| {
            bencher.iter(|| black_box(program.find(black_box(input), limits).unwrap()));
        });
        group.bench_function(format!("{label}/is_match"), |bencher| {
            bencher.iter(|| black_box(program.is_match(black_box(input), limits).unwrap()));
        });
    }
    // An exact count above the input's pairs, whose set of live counts grows
    // at every position: no state repeats, and every position costs a closure.
    let exact = compiled("(a|b){100000}c");
    let ab_1m = &ab_4m[..1 << 20];
    assert!(!exact.is_match(ab_1m, limits).unwrap());
    assert!(exact.find(ab_1m, limits).unwrap().is_none());
    group.throughput(Throughput::Bytes(ab_1m.len() as u64));
    group.bench_function("counted/exact_100000", |bencher| {
        bencher.iter(|| black_box(exact.find(black_box(ab_1m), limits).unwrap()));
    });
    group.bench_function("counted/exact_100000/is_match", |bencher| {
        bencher.iter(|| black_box(exact.is_match(black_box(ab_1m), limits).unwrap()));
    });
    // One start's ambiguous counted body over a long match, a replacement of
    // thousands of its matches, and an empty-preferring body below a minimum
    // beyond u64: the walk keeps one thread whatever the counts.
    let run_b = format!("{}b", "a".repeat(1 << 20));
    let mut runs = "a".repeat(100_000);
    let mut index = 0_usize;
    while runs.len() < 1 << 20 {
        runs.push_str(&"a".repeat(1 + (index * 7 + index / 5) % 13));
        runs.push(if index.is_multiple_of(3) { 'c' } else { 'b' });
        index += 1;
    }
    let ambiguous = compiled("(a|aa){1,1000}b");
    let span = ambiguous.find(&run_b, limits).unwrap().unwrap().get(0);
    assert_eq!(span, Some(run_b.len() - 2001..run_b.len()));
    group.throughput(Throughput::Bytes(run_b.len() as u64));
    group.bench_function("walk/ambiguous_1_1000", |bencher| {
        bencher.iter(|| black_box(ambiguous.find(black_box(&run_b), limits).unwrap()));
    });
    let replaced = ambiguous.replace_all(&runs, "[$1]", limits).unwrap();
    assert_eq!(
        replaced,
        compile("(a|aa){1,1000}b", "")
            .unwrap()
            .replace_all(&runs, "[$1]")
            .unwrap()
    );
    group.throughput(Throughput::Bytes(runs.len() as u64));
    group.bench_function("walk/replace_runs", |bencher| {
        bencher.iter(|| {
            black_box(
                ambiguous
                    .replace_all(black_box(&runs), "[$1]", limits)
                    .unwrap(),
            )
        });
    });
    let short_run_b = &run_b[run_b.len() - 100_001..];
    let huge = compiled("(|a){18446744073709551616}b");
    let captures = huge.find(short_run_b, limits).unwrap().unwrap();
    assert_eq!(captures.get(0), Some(0..short_run_b.len()));
    assert_eq!(
        captures.get(1),
        Some(short_run_b.len() - 2..short_run_b.len() - 1)
    );
    group.throughput(Throughput::Bytes(short_run_b.len() as u64));
    group.bench_function("walk/empty_first_huge", |bencher| {
        bencher.iter(|| black_box(huge.find(black_box(short_run_b), limits).unwrap()));
    });
    // Sixty-four mebibytes: an abandoned attempt, a reverse scan and a walk.
    let prose_64m = purrdf_testkit::text::word_prose(64 << 20);
    for (label, source, matched) in [
        ("scale/multi_run_64m", "node.*graph.*zzz", false),
        ("scale/word_group_64m", "^([a-z]+ ?)+$", true),
    ] {
        let program = compiled(source);
        assert_eq!(
            program.find(&prose_64m, limits).unwrap().is_some(),
            matched,
            "{label}"
        );
        group.throughput(Throughput::Bytes(prose_64m.len() as u64));
        group.bench_function(label, |bencher| {
            bencher.iter(|| black_box(program.find(black_box(&prose_64m), limits).unwrap()));
        });
    }
    // A backreference keeps the backtracking machine, and its exponential
    // exploration is refused by the step bound: the time that bound allows.
    let blowup = compiled(r"^(a|aa)*c\1$");
    let forty = "a".repeat(40);
    let refused = blowup.is_match(&forty, limits).unwrap_err();
    assert!(
        matches!(&refused, xpath::Error::Resource(refusal) if refusal.resource == xpath::Resource::MatchSteps),
        "{refused}"
    );
    group.throughput(Throughput::Elements(1));
    group.bench_function("refused/backreference_blowup", |bencher| {
        bencher.iter(|| black_box(blowup.is_match(black_box(&forty), limits).unwrap_err()));
    });
    let literal: String = ('a'..='z').cycle().take(30_000).collect();
    compiled(&literal);
    group.throughput(Throughput::Bytes(literal.len() as u64));
    group.bench_function("compile_literal_30k", |bencher| {
        bencher.iter(|| black_box(compiled(black_box(&literal))));
    });
    group.finish();
}

bench_group!(
    benches,
    bench_xsd_regex_compile,
    bench_xsd_regex_match,
    bench_native_xpath_compile,
    bench_native_xpath_execute,
    bench_native_xpath_large
);
bench_main!(benches);
