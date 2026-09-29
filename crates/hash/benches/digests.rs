// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Digest throughput at 64 B, 1 KiB and 1 MiB, per algorithm and per path the
//! host can run, and base16 encoding throughput per path, plus `hex::Lower`
//! rendering a 32-byte digest. Report-only; not a gate.

#![allow(missing_docs)] // criterion_main! generates an undocumented `main`

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use purrdf_hash::Backend as _;
use purrdf_hash::backend::{Crc32Backend, HexBackend, Sha1Backend};
use purrdf_hash::hex::Lower;
use purrdf_hash::md5::Md5;
use purrdf_hash::sha3::{Sha3_224, Sha3_256, Sha3_384, Sha3_512};

const SIZES: [(usize, &str); 3] = [(64, "64B"), (1024, "1KiB"), (1 << 20, "1MiB")];

/// Deterministic, non-trivial input bytes.
fn input(len: usize) -> Vec<u8> {
    let mut state = 0x9E37_79B9_7F4A_7C15u64;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect()
}

fn bench_one(c: &mut Criterion, group: &str, path: &str, digest: &dyn Fn(&[u8]) -> u8) {
    let mut group = c.benchmark_group(group);
    for (len, label) in SIZES {
        let data = input(len);
        group.throughput(Throughput::Bytes(len as u64));
        group.bench_with_input(BenchmarkId::new(path, label), &data, |b, data| {
            b.iter(|| digest(black_box(data)));
        });
    }
    group.finish();
}

fn digests(c: &mut Criterion) {
    bench_one(c, "md5", "portable", &|data| Md5::digest(data)[0]);
    for backend in Sha1Backend::all_available() {
        bench_one(c, "sha1", backend.name(), &|data| {
            backend.digest(data).map_or(0, |digest| digest[0])
        });
    }
    bench_one(c, "sha3-224", "portable", &|data| Sha3_224::digest(data)[0]);
    bench_one(c, "sha3-256", "portable", &|data| Sha3_256::digest(data)[0]);
    bench_one(c, "sha3-384", "portable", &|data| Sha3_384::digest(data)[0]);
    bench_one(c, "sha3-512", "portable", &|data| Sha3_512::digest(data)[0]);
    for backend in Crc32Backend::all_available() {
        bench_one(c, "crc32", backend.name(), &|data| {
            backend.checksum(data).map_or(0, |crc| crc as u8)
        });
    }
}

fn hex(c: &mut Criterion) {
    let mut group = c.benchmark_group("hex");
    for backend in HexBackend::all_available() {
        for (len, label) in [(32, "32B"), (1024, "1KiB"), (1 << 20, "1MiB")] {
            let data = input(len);
            let mut out = vec![0u8; 2 * len];
            group.throughput(Throughput::Bytes(len as u64));
            group.bench_with_input(BenchmarkId::new(backend.name(), label), &data, |b, data| {
                b.iter(|| backend.encode(black_box(data), black_box(&mut out)));
            });
        }
    }
    // The call-site shape: a digest written into a reserved `String`.
    let digest = input(32);
    let mut text = String::with_capacity(128);
    group.throughput(Throughput::Bytes(32));
    group.bench_function("lower-display/32B", |b| {
        b.iter(|| {
            use std::fmt::Write as _;
            text.clear();
            let _ = write!(text, "{}", Lower(black_box(&digest)));
            text.len()
        });
    });
    group.finish();
}

fn blake3(c: &mut Criterion) {
    let mut group = c.benchmark_group("blake3");
    for len in [0, 64, 1024, 4096, 16384, 65536, 1 << 20] {
        let data = input(len);
        group.throughput(Throughput::Bytes(len as u64));
        group.bench_with_input(BenchmarkId::new("one-shot", len), &data, |b, data| {
            b.iter(|| purrdf_hash::blake3::hash(black_box(data)));
        });
        for width in [8, 64, 1024, 16384, 65536] {
            if width > len {
                continue;
            }
            bench_stream::<16384>(&mut group, &data, width, "stream");
            bench_stream::<1024>(&mut group, &data, width, "record");
        }
    }
    group.finish();
}

fn bench_stream<const BUFFER: usize>(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    data: &[u8],
    width: usize,
    name: &str,
) {
    use purrdf_hash::blake3::Streaming;
    group.bench_function(
        BenchmarkId::new(format!("{name}/fresh/{width}"), data.len()),
        |b| {
            b.iter(|| {
                let mut state = Streaming::<BUFFER>::new();
                for part in black_box(data).chunks(width) {
                    state.update(part);
                }
                state.finalize()
            });
        },
    );
    let mut state = Streaming::<BUFFER>::new();
    group.bench_function(
        BenchmarkId::new(format!("{name}/reused/{width}"), data.len()),
        |b| {
            b.iter(|| {
                state.reset();
                for part in black_box(data).chunks(width) {
                    state.update(part);
                }
                state.finalize()
            });
        },
    );
}

fn blake3_backends(c: &mut Criterion) {
    use purrdf_hash::backend::Blake3Backend;
    let mut group = c.benchmark_group("blake3-backends");
    for backend in Blake3Backend::all_available() {
        for len in [0, 64, 1024, 4096, 16384, 65536, 1 << 20] {
            let data = input(len);
            group.throughput(Throughput::Bytes(len as u64));
            group.bench_function(
                BenchmarkId::new(format!("{backend:?}/one-shot"), len),
                |b| {
                    b.iter(|| backend.hash(black_box(&data)).expect("available backend"));
                },
            );
            for width in [64, 16384] {
                group.bench_function(
                    BenchmarkId::new(format!("{backend:?}/stream/{width}"), len),
                    |b| {
                        b.iter(|| {
                            let mut state = backend.hasher().expect("available backend");
                            for part in black_box(&data).chunks(width) {
                                state.update(part);
                            }
                            state.finalize()
                        });
                    },
                );
            }
        }
    }
    group.finish();
}

criterion_group!(benches, digests, hex, blake3, blake3_backends);
criterion_main!(benches);
