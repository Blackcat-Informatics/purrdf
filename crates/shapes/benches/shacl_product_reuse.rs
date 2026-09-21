// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! Report-only phase separation for the SHACL prepared-shapes product.
//!
//! **Nothing here is acceptance evidence, and no comparison between two of these
//! ids is asserted anywhere.** Wall-clock samples taken on a shared development
//! host are an illustration of where the work sits, not a measurement anyone can
//! gate on; the product's correctness is established by the determinism, refusal
//! and restore-equivalence tests under `crates/shapes/tests/`, which do not look
//! at a clock. A prepared product is a structural change — a parse that happened
//! once at build time instead of once per process start — and a structural change
//! needs no invented speedup threshold to be worth making.
//!
//! What this target does provide is a decomposition, because "restoring is
//! cheaper than parsing" is an aggregate that hides the three questions a reader
//! actually has:
//!
//! * `cold/parse_and_prepare` is the whole cold path — Turtle text to a
//!   [`PreparedShapes`](purrdf_shapes::engine::PreparedShapes) — measured fresh on
//!   every iteration. It is the thing a restore stands in for.
//! * `encode/to_product` is the PRODUCER's cost, and it is deliberately not
//!   folded into the cold path. It re-packs the shapes dataset and canonicalizes
//!   it to certify the identity the product states, which makes it the real
//!   peak-memory event of the whole pipeline — and it is paid by a build tool,
//!   once, never by the process that restores.
//! * `restore/open` is the structural tier alone: envelope framing, the section
//!   directory, every section digest, the trailer. It admits nothing, so a caller
//!   deciding what to do with an unknown product pays only this.
//! * `restore/admit` is the full memo path to a `PreparedShapes`, with the open
//!   as untimed setup so the id names what it says it names.
//! * `restore/rebuild` is the memo-free path: ignore the AST section and
//!   re-derive the shapes from the dataset the product carries. Its number is the
//!   one that says whether the AST section earns its keep — a rebuild that cost
//!   about what an admit costs would mean the model section is carrying bytes for
//!   nothing.
//! * `prepare/reusable` is the class-catalog derivation alone, over an
//!   already-parsed shapes graph. Both restore paths re-derive that catalog
//!   rather than carrying it, and this is the id that says whether that is a
//!   cheap re-derivation or a cost a future format revision should consider
//!   memoizing.
//! * `bind/dataset` and `eval/validate` are per-dataset work, separated from all
//!   of the above because they are paid once per snapshot regardless of how the
//!   preparation was obtained. Reusable preparation and per-dataset binding are
//!   different lines in the budget and are reported as different lines here.
//!
//! Every phase is driven through the crate's public API only, so this exact file
//! compiles and runs against a revision that predates any change to the product
//! internals.
//!
//! The encoded artifact's byte length — the pipeline's "intermediate bytes" — is
//! reported by the companion `shacl_product_alloc` target rather than here, and
//! is additionally pinned as an asserted constant in
//! `crates/shapes/tests/product_determinism.rs`.
//!
//! Report-only, `cargo bench -p purrdf-shapes --bench shacl_product_reuse` (the
//! `make bench` lane) — excluded from `make check`. No timing is asserted.

use std::sync::Arc;

use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};
use purrdf_shapes::engine::{PreparedShapes, parse_shapes};
use purrdf_shapes::product::{HostBindings, ShapesProduct, ShapesProfile};

#[path = "support/product.rs"]
mod fixture;

fn bench_product_reuse(c: &mut Criterion) {
    let source = fixture::shapes_source();
    let prepared = fixture::prepared(&source);
    let product = fixture::encode(&prepared);
    let data = fixture::data();
    let host = HostBindings::empty();

    // An already-parsed shapes graph, so `prepare/reusable` times the catalog
    // derivation and not the parse that precedes it in the cold path.
    let parsed = Arc::new(parse_shapes(&source, None).expect("the bench shapes graph parses"));
    let validator = prepared
        .bind_shared_dataset(Arc::clone(&data))
        .expect("the bench data graph binds");
    // Once, outside every timed region: prove `eval/validate` below is measuring
    // evaluation that actually reaches the constraints.
    fixture::assert_non_vacuous(&validator.validate().expect("validation runs"));

    let mut group = c.benchmark_group("shacl_product_reuse");

    group.bench_function("cold/parse_and_prepare", |b| {
        b.iter(|| {
            let shapes =
                parse_shapes(black_box(source.as_str()), None).expect("the shapes graph parses");
            black_box(PreparedShapes::new(Arc::new(shapes)))
        });
    });

    group.bench_function("encode/to_product", |b| {
        b.iter(|| {
            black_box(
                black_box(&prepared)
                    .to_product(&ShapesProfile::CORE)
                    .expect("the preparation is representable"),
            )
        });
    });

    group.bench_function("restore/open", |b| {
        b.iter(|| black_box(ShapesProduct::open(black_box(product.as_slice())).expect("opens")));
    });

    // `admit` and `rebuild` consume the view, so the open is untimed setup: the
    // id would otherwise silently include the structural tier that `restore/open`
    // already reports on its own line.
    group.bench_function("restore/admit", |b| {
        b.iter_batched(
            || ShapesProduct::open(&product).expect("the product opens"),
            |view| {
                black_box(
                    view.admit(&ShapesProfile::CORE, &host)
                        .expect("the product admits"),
                )
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("restore/rebuild", |b| {
        b.iter_batched(
            || ShapesProduct::open(&product).expect("the product opens"),
            |view| {
                black_box(
                    view.rebuild(&ShapesProfile::CORE, &host)
                        .expect("the product rebuilds"),
                )
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("prepare/reusable", |b| {
        b.iter(|| black_box(PreparedShapes::new(Arc::clone(black_box(&parsed)))));
    });

    group.bench_function("bind/dataset", |b| {
        b.iter(|| {
            black_box(
                black_box(&prepared)
                    .bind_shared_dataset(Arc::clone(&data))
                    .expect("the data graph binds"),
            )
        });
    });

    group.bench_function("eval/validate", |b| {
        b.iter(|| black_box(black_box(&validator).validate().expect("validation runs")));
    });

    group.finish();
}

criterion_group!(benches, bench_product_reuse);
criterion_main!(benches);
