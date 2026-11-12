// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `XXH64` over short Bloom values, LDM windows and frame checksum blocks.

use core::hash::Hasher as _;
use std::hint::black_box;

use purrdf_hash::xxh64::{Xxh64, hash};
use purrdf_testkit::bench::{Bench, BenchmarkId, Throughput, bench_group, bench_main};
use purrdf_testkit::rng::splitmix64_bytes;

fn oneshot(c: &mut Bench, length: usize) {
    let mut group = c.benchmark_group("xxh64");
    let data = splitmix64_bytes(length, 0);
    group.throughput(Throughput::Bytes(length as u64));
    group.bench_with_input(BenchmarkId::new("native", length), &data, |b, data| {
        b.iter(|| hash(0, black_box(data)));
    });
    group.finish();
}

fn streaming(c: &mut Bench, chunk_size: usize) {
    let mut group = c.benchmark_group("xxh64-stream");
    let data = splitmix64_bytes(131_072, 0);
    group.throughput(Throughput::Bytes(data.len() as u64));
    group.bench_with_input(BenchmarkId::new("native", chunk_size), &data, |b, data| {
        b.iter(|| {
            let mut state = Xxh64::new();
            for chunk in black_box(data).chunks(chunk_size) {
                state.write(chunk);
            }
            state.finish()
        });
    });
    group.finish();
}

fn xxh64(c: &mut Bench) {
    for length in [8, 31, 64, 1024, 131_072] {
        oneshot(c, length);
    }
    for chunk_size in [17, 4096] {
        streaming(c, chunk_size);
    }
}

bench_group!(benches, xxh64);
bench_main!(benches);
