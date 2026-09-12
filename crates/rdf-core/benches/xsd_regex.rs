// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
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
//!   folded set is built on first use; criterion's warm-up absorbs that
//!   one-time construction).
//! * `block_first` / `block_last` — `\p{IsBasicLatin}` and the LAST block in
//!   the generated table, `\p{IsSupplementaryPrivateUseArea-B}`. The two
//!   bracket the binary search that replaced a linear scan over 338 rows.
//! * `class_subtraction` — `^[a-z-[aeiou]]+$`, rewritten to the `regex`
//!   crate's `--` set difference.
//! * `x_flag` / `q_flag` — the two flags whose handling is a source rewrite
//!   (`x` strips whitespace before translation; `q` escapes a literal).
//!
//! Report-only, `cargo bench -p purrdf-core --bench xsd_regex` (the
//! `make bench` lane) — excluded from `make check`. No timing is asserted.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use purrdf_core::xsd_regex::compile;

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

fn bench_xsd_regex_compile(c: &mut Criterion) {
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

criterion_group!(benches, bench_xsd_regex_compile);
criterion_main!(benches);
