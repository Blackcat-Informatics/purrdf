// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! XSD lexical hot-path benchmark: the per-literal operations every codec and
//! the SPARQL evaluator run for each typed literal they touch.
//!
//! Report-only, `cargo bench -p purrdf-xsd --bench lexical`. Nothing here asserts
//! a threshold; the machine running it is not assumed quiet. Each case isolates
//! one of the lexical primitives whose allocation profile the release sweep
//! changed, so CI's report-only lane can measure them on their own:
//!
//! - `datatype_from_iri` — [`XsdDatatype::from_iri`] over a mixed corpus of XSD
//!   and non-XSD IRIs (one namespace-prefix compare, then a local-name match).
//! - `whitespace_replace` / `whitespace_collapse` — the `whiteSpace` facets over
//!   mixed ASCII/multibyte text (byte-run copies rather than per-char pushes).
//! - `decimal_parse_canonical` — [`parse_decimal`] then `canonical_lexical` over
//!   a grid of scales and magnitudes (one exact-fit output buffer).
//! - `double_canonical` — [`canonical_double`] over a spread of magnitudes (the
//!   `mantissa E exponent` assembly in one buffer).

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use purrdf_xsd::{
    XSD_NS, XsdDatatype, normalize_whitespace_collapse, normalize_whitespace_replace,
    numeric::{canonical_double, parse_decimal},
};

/// IRIs a literal-heavy document resolves per typed literal: the common XSD
/// types, plus non-XSD datatypes that must fall out on the prefix compare.
const DATATYPE_CORPUS: &[&str] = &[
    "http://www.w3.org/2001/XMLSchema#string",
    "http://www.w3.org/2001/XMLSchema#integer",
    "http://www.w3.org/2001/XMLSchema#decimal",
    "http://www.w3.org/2001/XMLSchema#double",
    "http://www.w3.org/2001/XMLSchema#boolean",
    "http://www.w3.org/2001/XMLSchema#dateTime",
    "http://www.w3.org/2001/XMLSchema#nonNegativeInteger",
    "http://www.w3.org/2001/XMLSchema#base64Binary",
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString",
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#JSON",
    "http://www.opengis.net/ont/geosparql#wktLiteral",
    "https://example.org/datatype/custom",
];

/// Whitespace-facet inputs: clean ASCII, tab/newline-laden ASCII, and text with
/// multibyte characters around the whitespace so run boundaries are exercised.
const WHITESPACE_CORPUS: &[&str] = &[
    "a plain token with no facet triggers at all",
    "  leading\tand\ttrailing\n\n whitespace  \r\n  runs   here  ",
    "ünïcödé\t日本語\n한국어\r  العربية   \u{1F600}  end",
    "\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t",
    "one",
];

/// Decimal lexicals across scale (0 to 12 fraction digits) and magnitude.
fn decimal_corpus() -> Vec<String> {
    let mut out = Vec::new();
    for mantissa in [
        0i64,
        1,
        -1,
        42,
        -42,
        1_000,
        123_456_789,
        -987_654_321,
        i64::MAX / 7,
    ] {
        for scale in [0usize, 1, 2, 5, 12] {
            let digits = mantissa.unsigned_abs().to_string();
            let sign = if mantissa < 0 { "-" } else { "" };
            let text = if scale == 0 {
                format!("{sign}{digits}")
            } else if digits.len() > scale {
                let split = digits.len() - scale;
                format!("{sign}{}.{}", &digits[..split], &digits[split..])
            } else {
                format!("{sign}0.{}{digits}", "0".repeat(scale - digits.len()))
            };
            out.push(text);
        }
        // Trailing zeros: the canonical form trims them.
        out.push(format!("{mantissa}.500000"));
    }
    out
}

/// Doubles spanning the exponent range, including the exact-integer and
/// sub-unity shapes that take different branches of the canonical mapping.
const DOUBLE_CORPUS: &[f64] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    1.5,
    42.0,
    0.001,
    123_456.789,
    1e21,
    -2.5e-7,
    f64::MAX,
    f64::MIN_POSITIVE,
    f64::INFINITY,
    f64::NEG_INFINITY,
    f64::NAN,
];

fn bench_datatype_from_iri(c: &mut Criterion) {
    let mut group = c.benchmark_group("xsd_datatype_from_iri");
    group.throughput(Throughput::Elements(DATATYPE_CORPUS.len() as u64));
    group.bench_function("mixed_corpus", |b| {
        b.iter(|| {
            let mut hits = 0usize;
            for iri in DATATYPE_CORPUS {
                if XsdDatatype::from_iri(black_box(iri)).is_some() {
                    hits += 1;
                }
            }
            black_box(hits)
        });
    });
    // A namespace-only miss: the IRI shares the whole XSD prefix but names no
    // built-in, so the local-name match runs to its fall-through arm.
    let near_miss = format!("{XSD_NS}notARealDatatype");
    group.bench_function("namespace_near_miss", |b| {
        b.iter(|| black_box(XsdDatatype::from_iri(black_box(&near_miss))));
    });
    group.finish();
}

fn bench_whitespace_facets(c: &mut Criterion) {
    let total_bytes: usize = WHITESPACE_CORPUS.iter().map(|s| s.len()).sum();
    let mut group = c.benchmark_group("xsd_whitespace_facets");
    group.throughput(Throughput::Bytes(total_bytes as u64));
    group.bench_function("replace", |b| {
        b.iter(|| {
            let mut len = 0usize;
            for s in WHITESPACE_CORPUS {
                len += normalize_whitespace_replace(black_box(s)).len();
            }
            black_box(len)
        });
    });
    group.bench_function("collapse", |b| {
        b.iter(|| {
            let mut len = 0usize;
            for s in WHITESPACE_CORPUS {
                len += normalize_whitespace_collapse(black_box(s)).len();
            }
            black_box(len)
        });
    });
    group.finish();
}

fn bench_decimal(c: &mut Criterion) {
    let corpus = decimal_corpus();
    let mut group = c.benchmark_group("xsd_decimal");
    group.throughput(Throughput::Elements(corpus.len() as u64));
    group.bench_function("parse_canonical", |b| {
        b.iter(|| {
            let mut len = 0usize;
            for text in &corpus {
                let value = parse_decimal(black_box(text)).expect("bench corpus is valid");
                len += value.canonical_lexical().len();
            }
            black_box(len)
        });
    });
    group.finish();
}

fn bench_double_canonical(c: &mut Criterion) {
    let mut group = c.benchmark_group("xsd_double");
    group.throughput(Throughput::Elements(DOUBLE_CORPUS.len() as u64));
    group.bench_function("canonical", |b| {
        b.iter(|| {
            let mut len = 0usize;
            for &d in DOUBLE_CORPUS {
                len += canonical_double(black_box(d)).len();
            }
            black_box(len)
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_datatype_from_iri,
    bench_whitespace_facets,
    bench_decimal,
    bench_double_canonical
);
criterion_main!(benches);
