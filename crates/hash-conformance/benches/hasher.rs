// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Fixed-hasher latency per key class, for every function this build can
//! run: the portable function always, the AES function on a build whose
//! target enables AES (`RUSTFLAGS='-C target-feature=+aes'`, or a
//! `target-cpu` that has it). Each measurement hashes a whole key set;
//! streaming cases use one fresh hasher and one `finish` per key, while the
//! interner case calls the terminal key function. The reported time per
//! element is one table-lookup hash. Report-only; not a gate.

#![allow(missing_docs)] // criterion_main! generates an undocumented `main`

use core::hash::{Hash, Hasher};
use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
#[cfg(all(
    any(target_arch = "x86_64", target_arch = "aarch64"),
    target_endian = "little",
    target_feature = "aes"
))]
use purrdf_hash::backend::AesFixedHasher;
use purrdf_hash::backend::PortableFixedHasher;
use purrdf_hash_conformance::iri_corpus;
use purrdf_testkit::rng::Xoshiro256;

/// `count` byte keys whose lengths cycle through `lengths`.
fn byte_keys(lengths: core::ops::RangeInclusive<usize>, count: usize, seed: u64) -> Vec<Vec<u8>> {
    let mut rng = Xoshiro256::from_seed(seed);
    let lengths: Vec<usize> = lengths.collect();
    (0..count)
        .map(|index| {
            let len = lengths[index % lengths.len()];
            (0..len).map(|_| rng.next_u64() as u8).collect()
        })
        .collect()
}

/// Times one function over every class.
fn bench_path<H: Hasher + Default>(c: &mut Criterion, path: &str, terminal: fn(u8, &[u8]) -> u64) {
    let mut rng = Xoshiro256::from_seed(0x6265_6e63_6831);
    let words: Vec<u64> = (0..4096).map(|_| rng.next_u64()).collect();
    let short = byte_keys(1..=16, 4096, 1);
    let short_long = byte_keys(17..=32, 4096, 2);
    let medium = byte_keys(33..=128, 4096, 4);
    let long = byte_keys(129..=1024, 512, 3);
    // `http://example.org/i/` is 21 bytes; five digits make a 26-byte IRI.
    let short_iris: Vec<String> = (0..4096)
        .map(|i| format!("http://example.org/i/{i:05}"))
        .collect();
    assert!(short_iris.iter().all(|iri| iri.len() == 26));
    let iris = iri_corpus();

    let mut group = c.benchmark_group("fixed-hasher");

    group.throughput(Throughput::Elements(words.len() as u64));
    group.bench_with_input(BenchmarkId::new("u32", path), &words, |b, words| {
        b.iter(|| {
            words.iter().fold(0u64, |sum, &word| {
                let mut hasher = H::default();
                hasher.write_u32(black_box(word as u32));
                sum ^ hasher.finish()
            })
        });
    });
    group.bench_with_input(BenchmarkId::new("u64", path), &words, |b, words| {
        b.iter(|| {
            words.iter().fold(0u64, |sum, &word| {
                let mut hasher = H::default();
                hasher.write_u64(black_box(word));
                sum ^ hasher.finish()
            })
        });
    });

    for (class, keys) in [
        ("1-16B", &short),
        ("17-32B", &short_long),
        ("33-128B", &medium),
        ("129-1024B", &long),
    ] {
        group.throughput(Throughput::Elements(keys.len() as u64));
        group.bench_with_input(BenchmarkId::new(class, path), keys, |b, keys| {
            b.iter(|| {
                keys.iter().fold(0u64, |sum, key| {
                    let mut hasher = H::default();
                    hasher.write(black_box(key));
                    sum ^ hasher.finish()
                })
            });
        });
    }

    // The hot 26-byte IRI class through `str`'s `Hash`, including its terminator.
    group.throughput(Throughput::Elements(short_iris.len() as u64));
    group.bench_with_input(BenchmarkId::new("iri-26B", path), &short_iris, |b, iris| {
        b.iter(|| {
            iris.iter().fold(0u64, |sum, iri| {
                let mut hasher = H::default();
                black_box(iri.as_str()).hash(&mut hasher);
                sum ^ hasher.finish()
            })
        });
    });

    // The primary IR interner's terminal IRI key: one compressed operation.
    group.bench_with_input(
        BenchmarkId::new("iri-26B-interner", path),
        &short_iris,
        |b, iris| {
            b.iter(|| {
                iris.iter().fold(0u64, |sum, iri| {
                    sum ^ terminal(0, black_box(iri.as_bytes()))
                })
            });
        },
    );

    // The IRI corpus through `str`'s `Hash`, as a `HashMap<String, _>` would.
    group.throughput(Throughput::Elements(iris.len() as u64));
    group.bench_with_input(BenchmarkId::new("iri-corpus", path), &iris, |b, iris| {
        b.iter(|| {
            iris.iter().fold(0u64, |sum, iri| {
                let mut hasher = H::default();
                black_box(iri.as_str()).hash(&mut hasher);
                sum ^ hasher.finish()
            })
        });
    });
    group.finish();
}

fn hashers(c: &mut Criterion) {
    bench_path::<PortableFixedHasher>(c, "portable", PortableFixedHasher::hash_terminal);
    #[cfg(all(
        any(target_arch = "x86_64", target_arch = "aarch64"),
        target_endian = "little",
        target_feature = "aes"
    ))]
    bench_path::<AesFixedHasher>(c, "aes", AesFixedHasher::hash_terminal);
}

criterion_group!(benches, hashers);
criterion_main!(benches);
