// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! RFC 3339 `modified` stamps: parse and format.
//!
//! Report-only, `cargo bench -p purrdf-gts --bench rfc3339` (the `make bench`
//! lane). The files and tar profiles make one call per archive entry, so this
//! is a regression watch, not a hot-path claim. The module is crate-private,
//! so the bench compiles the same source file directly.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};

// `cargo clippy --all-targets` builds bench targets with `cfg(test)`, which
// pulls the module's own unit tests in here as well; they run from the library,
// so here they are unused by construction.
#[allow(dead_code, unused_imports)]
#[path = "../src/rfc3339.rs"]
mod rfc3339;

fn parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("rfc3339_parse");
    for (name, text) in [
        ("utc_whole_second", "2023-11-14T22:13:20Z"),
        ("offset_fraction", "1985-04-12T23:20:50.52-08:00"),
        ("long_fraction", "1985-04-12T23:20:50.123456789123456789Z"),
        ("leap_second", "1990-12-31T15:59:60-08:00"),
        ("refused_day", "2023-02-29T00:00:00Z"),
    ] {
        group.bench_with_input(BenchmarkId::from_parameter(name), text, |b, text| {
            b.iter(|| black_box(rfc3339::parse(black_box(text)).is_ok()));
        });
    }
    group.finish();
}

fn format(c: &mut Criterion) {
    let mut group = c.benchmark_group("rfc3339_format");
    for (name, seconds, nanos) in [
        ("whole_second", 1_700_000_000_i64, 0_u32),
        ("nanoseconds", 1_700_000_000, 123_456_789),
        ("pre_epoch", -1_041_337_173, 870_000_000),
    ] {
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &(seconds, nanos),
            |b, &(s, n)| {
                b.iter(|| black_box(rfc3339::format(black_box(s), black_box(n))));
            },
        );
    }
    group.finish();
}

criterion_group!(benches, parse, format);
criterion_main!(benches);
