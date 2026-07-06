// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! Intern-time content-id overhead bench.
//!
//! Content addressing (`RdfDatasetBuilder::with_content_addressing`) adds a
//! recognition check to every `intern_iri` call: does this IRI carry the
//! configured scheme prefix and, if so, does the suffix decode as valid hex? The
//! cost model is that this check is near-free for IRIs that
//! never match the prefix (a `strip_prefix` miss) and pay a real but small cost
//! only for genuine content-id IRIs (decode + side-table insert).
//!
//! This is a **report-only** bench: it prints/records wall-clock time for three
//! scenarios so the cost can be *read*, not a gate that asserts a speedup or a
//! timing threshold (repo policy — benches never assert timing).
//!
//! 1. `intern_ordinary_scheme_inactive` — baseline: a plain builder interning N
//!    ordinary `http://example.org/r/<i>` IRIs. No content-addressing config at
//!    all, so no recognition check runs.
//! 2. `intern_ordinary_scheme_active` — the SAME N ordinary IRIs, but interned
//!    into a builder configured with `with_content_addressing`. Every one of
//!    these IRIs misses the `blake3:` prefix, so this measures the strip-prefix
//!    miss cost in isolation. Expectation (NOT asserted): this should track
//!    group 1 closely, since a prefix miss is a cheap string compare.
//! 3. `intern_content_ids_scheme_active` — N genuine `blake3:<64 lowercase
//!    hex>` IRIs interned into a content-addressing-active builder. Every IRI
//!    hits the prefix, decodes 64 hex chars to 32 bytes, and inserts a
//!    side-table entry. This measures the real decode + insert cost.
//!
//! IRIs are generated deterministically (no RNG, no time source) so the input
//! set is stable across runs: ordinary IRIs are `http://example.org/r/{i}`, and
//! content-ids are built by tiling a hex encoding of `i` out to exactly 64
//! lowercase hex characters.

use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};
use purrdf_core::{ContentIdScheme, RdfDatasetBuilder};

/// Number of IRIs interned per measured iteration.
const N: u32 = 10_000;

/// Build the ordinary (non-content-id) IRI set once: `http://example.org/r/{i}`
/// for `i` in `0..N`. Distinct per `i` so every intern is a genuine new-entry
/// insert, not a dedup hit.
fn ordinary_iris() -> Vec<String> {
    (0..N)
        .map(|i| format!("http://example.org/r/{i}"))
        .collect()
}

/// Build the genuine content-id IRI set once: `blake3:<64 lowercase hex>` for
/// `i` in `0..N`, each suffix deterministically derived from `i` (no RNG). The
/// hex digest is `i` formatted as 8 hex digits, tiled 8x to fill exactly 64
/// lowercase hex characters — distinct per `i`, always valid hex.
fn content_id_iris() -> Vec<String> {
    (0..N)
        .map(|i| {
            let octet = format!("{i:08x}");
            debug_assert_eq!(octet.len(), 8);
            let hex64 = octet.repeat(8);
            debug_assert_eq!(hex64.len(), 64);
            format!("blake3:{hex64}")
        })
        .collect()
}

/// Group 1: baseline — plain builder, ordinary IRIs, no content-addressing
/// config at all (no recognition check runs on this path).
fn bench_ordinary_scheme_inactive(c: &mut Criterion) {
    let iris = ordinary_iris();
    let mut group = c.benchmark_group("intern_content_id");
    group.bench_function("intern_ordinary_scheme_inactive", |b| {
        b.iter_batched(
            || iris.clone(),
            |iris| {
                let mut builder = RdfDatasetBuilder::new();
                for iri in &iris {
                    black_box(builder.intern_iri(black_box(iri)));
                }
                black_box(builder)
            },
            BatchSize::LargeInput,
        );
    });
    group.finish();
}

/// Group 2: the SAME ordinary IRIs, but the builder has content-addressing
/// active — every intern pays a `blake3:` prefix-strip MISS. Expectation (not
/// asserted): should read close to group 1's time.
fn bench_ordinary_scheme_active(c: &mut Criterion) {
    let iris = ordinary_iris();
    let mut group = c.benchmark_group("intern_content_id");
    group.bench_function("intern_ordinary_scheme_active", |b| {
        b.iter_batched(
            || iris.clone(),
            |iris| {
                let scheme = ContentIdScheme::new("blake3:").expect("valid scheme prefix");
                let mut builder = RdfDatasetBuilder::with_content_addressing(scheme, None);
                for iri in &iris {
                    black_box(builder.intern_iri(black_box(iri)));
                }
                black_box(builder)
            },
            BatchSize::LargeInput,
        );
    });
    group.finish();
}

/// Group 3: genuine `blake3:<64hex>` content-id IRIs interned into a
/// content-addressing-active builder — every intern pays the prefix HIT +
/// 64-hex decode + side-table insert.
fn bench_content_ids_scheme_active(c: &mut Criterion) {
    let iris = content_id_iris();
    let mut group = c.benchmark_group("intern_content_id");
    group.bench_function("intern_content_ids_scheme_active", |b| {
        b.iter_batched(
            || iris.clone(),
            |iris| {
                let scheme = ContentIdScheme::new("blake3:").expect("valid scheme prefix");
                let mut builder = RdfDatasetBuilder::with_content_addressing(scheme, None);
                for iri in &iris {
                    black_box(builder.intern_iri(black_box(iri)));
                }
                black_box(builder)
            },
            BatchSize::LargeInput,
        );
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_ordinary_scheme_inactive,
    bench_ordinary_scheme_active,
    bench_content_ids_scheme_active
);
criterion_main!(benches);
