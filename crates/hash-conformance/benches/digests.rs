// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Digest throughput at 64 B, 1 KiB and 1 MiB, per algorithm and per path the
//! host can run; base16 at 8 B to 1 MiB per encoding path
//! and through every public entry point (the length switch included), and the
//! 64-digit content-address readers. Report-only; not a gate.

#![allow(missing_docs)] // a bench target is not public API

use std::hint::black_box;

use purrdf_hash::Backend as _;
use purrdf_hash::backend::{Crc32Backend, HexBackend, Sha1Backend};
use purrdf_hash::hex::{
    Lower, decode, decode_32, decode_32_canonical, decode_canonical, encode, encode_into,
    encode_to_slice, encode_upper_to_slice,
};
use purrdf_hash::md5::Md5;
use purrdf_hash::sha3::{Sha3_224, Sha3_256, Sha3_384, Sha3_512};
use purrdf_testkit::bench::{Bench, BenchmarkId, Throughput, bench_group, bench_main};
use purrdf_testkit::rng::xorshift64_next;

const SIZES: [(usize, &str); 3] = [(64, "64B"), (1024, "1KiB"), (1 << 20, "1MiB")];

/// Deterministic, non-trivial input bytes.
fn input(len: usize) -> Vec<u8> {
    let mut state = purrdf_hash::mix::GOLDEN_GAMMA;
    (0..len)
        .map(|_| xorshift64_next(&mut state) as u8)
        .collect()
}

fn bench_one(c: &mut Bench, group: &str, path: &str, digest: &dyn Fn(&[u8]) -> u8) {
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

fn digests(c: &mut Bench) {
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

/// The length classes base16 is selected over: 8 bytes, a 16-, 20- and
/// 32-byte digest (32 is the length switch), one past it, 64 bytes, a
/// 4 KiB blob and a 1 MiB payload.
const HEX_SIZES: [usize; 8] = [8, 16, 20, 32, 33, 64, 4096, 1 << 20];

fn hex(c: &mut Bench) {
    let mut group = c.benchmark_group("hex");
    for len in HEX_SIZES {
        let data = input(len);
        let mut out = vec![0u8; 2 * len];
        group.throughput(Throughput::Bytes(len as u64));
        // Each path on its own, at every length class.
        for backend in HexBackend::all_available() {
            group.bench_with_input(
                BenchmarkId::new(format!("path-{}", backend.name()), len),
                &data,
                |b, data| b.iter(|| backend.encode(black_box(data), black_box(&mut out))),
            );
        }
        // The public entry points, through the length switch.
        group.bench_with_input(
            BenchmarkId::new("encode_to_slice", len),
            &data,
            |b, data| {
                b.iter(|| encode_to_slice(black_box(data), black_box(&mut out)).map(str::len));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("encode_upper_to_slice", len),
            &data,
            |b, data| {
                b.iter(|| {
                    encode_upper_to_slice(black_box(data), black_box(&mut out)).map(str::len)
                });
            },
        );
        group.bench_with_input(BenchmarkId::new("encode", len), &data, |b, data| {
            b.iter(|| encode(black_box(data)));
        });
        let mut text = String::with_capacity(2 * len);
        group.bench_with_input(BenchmarkId::new("encode_into", len), &data, |b, data| {
            b.iter(|| {
                text.clear();
                encode_into(black_box(data), &mut text);
                text.len()
            });
        });
        group.bench_with_input(BenchmarkId::new("lower-display", len), &data, |b, data| {
            b.iter(|| {
                use std::fmt::Write as _;
                text.clear();
                let _ = write!(text, "{}", Lower(black_box(data)));
                text.len()
            });
        });
        let digits = encode(&data);
        group.bench_with_input(BenchmarkId::new("decode", len), &digits, |b, digits| {
            b.iter(|| decode(black_box(digits)).map(|bytes| bytes.len()));
        });
        group.bench_with_input(
            BenchmarkId::new("decode_canonical", len),
            &digits,
            |b, digits| {
                b.iter(|| decode_canonical(black_box(digits)).map(|bytes| bytes.len()));
            },
        );
    }
    // The content-address readers: exactly 64 digits.
    let digits = encode(&input(32));
    let upper = digits.to_ascii_uppercase();
    group.throughput(Throughput::Bytes(32));
    group.bench_function("decode_32_canonical/32", |b| {
        b.iter(|| decode_32_canonical(black_box(&digits)));
    });
    group.bench_function("decode_32/32", |b| b.iter(|| decode_32(black_box(&upper))));
    group.finish();
}

fn blake3(c: &mut Bench) {
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
    group: &mut purrdf_testkit::bench::BenchmarkGroup<'_>,
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

fn blake3_backends(c: &mut Bench) {
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

bench_group!(benches, digests, hex, blake3, blake3_backends);
bench_main!(benches);
