// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! gzip encode and decode throughput over text, random and repeated-token
//! inputs at 4 KiB, 1 MiB and 16 MiB, at the default level, and the kernels
//! per path the host can run. Report-only; not a gate.
//!
//! Each encode case prints its compressed size once (`gzip-size …` lines), so
//! a run records density beside throughput.

#![allow(missing_docs)] // criterion_main! generates an undocumented `main`

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use purrdf_deflate::backend::Backend;
use purrdf_deflate::{Deflater, GzipDecoder, Level, gzip};
use purrdf_hash::Backend as _;

const SIZES: [(usize, &str); 3] = [(4 << 10, "4KiB"), (1 << 20, "1MiB"), (16 << 20, "16MiB")];

/// A deterministic xorshift byte stream.
fn random(len: usize, mut state: u64) -> Vec<u8> {
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect()
}

/// Words from a small vocabulary, pseudo-randomly ordered.
fn text(len: usize) -> Vec<u8> {
    const WORDS: [&str; 16] = [
        "the",
        "subject",
        "predicate",
        "object",
        "graph",
        "literal",
        "of",
        "and",
        "a",
        "resource",
        "language",
        "datatype",
        "blank",
        "node",
        "triple",
        "quad",
    ];
    let picks = random(len / 3 + 16, 0x7E57);
    let mut out = Vec::with_capacity(len + 16);
    for pick in picks {
        if out.len() >= len {
            break;
        }
        out.extend_from_slice(WORDS[usize::from(pick & 15)].as_bytes());
        out.push(if pick >> 4 == 0 { b'\n' } else { b' ' });
    }
    out.truncate(len);
    out
}

/// N-Triples lines that differ only in a counter.
fn repeated_token(len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len + 128);
    let mut n = 0u64;
    while out.len() < len {
        out.extend_from_slice(
            format!(
                "<http://example.org/s{}> <http://example.org/p> \"v{}\" .\n",
                n % 997,
                n
            )
            .as_bytes(),
        );
        n += 1;
    }
    out.truncate(len);
    out
}

fn corpus() -> Vec<(&'static str, &'static str, Vec<u8>)> {
    let mut all = Vec::new();
    for (len, label) in SIZES {
        all.push(("text", label, text(len)));
        all.push(("random", label, random(len, 0xD1CE)));
        all.push(("repeated-token", label, repeated_token(len)));
    }
    all
}

fn encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("gzip-encode");
    for (kind, label, data) in corpus() {
        let size = gzip::compress(&data, Level::DEFAULT).len();
        println!("gzip-size {kind}/{label}: {} -> {size} bytes", data.len());
        group.throughput(Throughput::Bytes(data.len() as u64));
        group.sample_size(if data.len() > 1 << 22 { 10 } else { 30 });
        group.bench_with_input(BenchmarkId::new(kind, label), &data, |b, data| {
            b.iter(|| gzip::compress(black_box(data), Level::DEFAULT));
        });
    }
    group.finish();
}

fn decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("gzip-decode");
    for (kind, label, data) in corpus() {
        let framed = gzip::compress(&data, Level::DEFAULT);
        group.throughput(Throughput::Bytes(data.len() as u64));
        group.sample_size(if data.len() > 1 << 22 { 10 } else { 30 });
        group.bench_with_input(BenchmarkId::new(kind, label), &framed, |b, framed| {
            let mut out = vec![0u8; 256 * 1024];
            b.iter(|| {
                let mut decoder = GzipDecoder::new();
                let mut input = framed.as_slice();
                let mut total = 0usize;
                loop {
                    let progress = decoder.feed(black_box(input), &mut out).expect("valid");
                    total += progress.written;
                    input = &input[progress.consumed..];
                    if progress.status == purrdf_deflate::Status::NeedsInput {
                        break;
                    }
                }
                decoder.finish().expect("complete");
                total
            });
        });
    }
    group.finish();
}

/// Append successive gzip members into reusable output without staging copies.
fn decode_vec(c: &mut Criterion) {
    let mut group = c.benchmark_group("gzip-decode-vector");
    for (kind, label, data) in corpus() {
        let framed = gzip::compress(&data, Level::DEFAULT);
        group.throughput(Throughput::Bytes(data.len() as u64));
        group.sample_size(if data.len() > 1 << 22 { 10 } else { 30 });
        group.bench_with_input(BenchmarkId::new(kind, label), &framed, |b, framed| {
            let mut decoder = GzipDecoder::new();
            let mut out = Vec::with_capacity(data.len());
            b.iter(|| {
                out.clear();
                let progress = decoder
                    .feed_to_vec(black_box(framed), &mut out)
                    .expect("valid");
                assert_eq!(progress.consumed, framed.len());
                decoder.finish().expect("complete member");
                black_box(out.as_slice());
            });
        });
    }
    group.finish();
}

/// Encode and decode of the 1 MiB text case per kernel path.
fn paths(c: &mut Criterion) {
    let data = text(1 << 20);
    let mut group = c.benchmark_group("deflate-paths");
    group.throughput(Throughput::Bytes(data.len() as u64));
    for backend in Backend::all_available() {
        group.bench_function(BenchmarkId::new("encode", backend.name()), |b| {
            b.iter(|| {
                let mut deflater = Deflater::with_backend(Level::DEFAULT, backend).expect("path");
                let mut out = Vec::new();
                deflater.write(black_box(&data), &mut out);
                deflater.finish(&mut out);
                out
            });
        });
        let raw = purrdf_deflate::deflate::compress(&data, Level::DEFAULT);
        group.bench_function(BenchmarkId::new("decode", backend.name()), |b| {
            b.iter(|| {
                let mut inflater = purrdf_deflate::Inflater::with_backend(backend).expect("path");
                let mut out = Vec::with_capacity(data.len());
                inflater
                    .feed_to_vec(black_box(&raw), &mut out)
                    .expect("valid");
                out
            });
        });
    }
    group.finish();
}

criterion_group!(benches, encode, decode, decode_vec, paths);
criterion_main!(benches);
