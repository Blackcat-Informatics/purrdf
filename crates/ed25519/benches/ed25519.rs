// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Ed25519 signing, verification and key work: `sign` and `verify_strict` over
//! messages from empty to 64 KiB (SHA-512 dominates the long ones), key
//! expansion from a seed, public-key decoding, and a GTS-shaped batch: 64
//! COSE_Sign1 `Sig_structure` sized messages (a few hundred bytes each) signed
//! by four keys, verified in a row. Report-only; not a gate.

#![allow(missing_docs)] // a bench target is not public API

use std::hint::black_box;

use purrdf_ed25519::{Signature, SigningKey, VerifyingKey};
use purrdf_testkit::bench::{Bench, BenchmarkId, Throughput, bench_group, bench_main};
use purrdf_testkit::rng::SplitMix64;

fn bytes(rng: &mut SplitMix64, len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len + 8);
    while out.len() < len {
        out.extend_from_slice(&rng.next_u64().to_le_bytes());
    }
    out.truncate(len);
    out
}

fn key(rng: &mut SplitMix64) -> SigningKey {
    let seed: [u8; 32] = bytes(rng, 32).try_into().expect("32 bytes");
    SigningKey::from_bytes(&seed)
}

const SIZES: [usize; 4] = [0, 128, 4 << 10, 64 << 10];

fn sign(c: &mut Bench) {
    let mut rng = SplitMix64::new(0x0ED2_5519);
    let signer = key(&mut rng);
    let mut group = c.benchmark_group("ed25519-sign");
    for len in SIZES {
        let message = bytes(&mut rng, len);
        group.throughput(Throughput::Bytes(len.max(1) as u64));
        group.bench_with_input(BenchmarkId::from_parameter(len), &message, |b, message| {
            b.iter(|| signer.sign(black_box(message)));
        });
    }
    group.finish();
}

fn verify(c: &mut Bench) {
    let mut rng = SplitMix64::new(0x0ED2_5519 + 1);
    let signer = key(&mut rng);
    let public = signer.verifying_key();
    let mut group = c.benchmark_group("ed25519-verify");
    for len in SIZES {
        let message = bytes(&mut rng, len);
        let signature = signer.sign(&message);
        group.throughput(Throughput::Bytes(len.max(1) as u64));
        group.bench_with_input(BenchmarkId::from_parameter(len), &message, |b, message| {
            b.iter(|| {
                public
                    .verify_strict(black_box(message), black_box(&signature))
                    .expect("valid");
            });
        });
    }
    group.finish();
}

fn keys(c: &mut Bench) {
    let mut rng = SplitMix64::new(0x0ED2_5519 + 2);
    let seed: [u8; 32] = bytes(&mut rng, 32).try_into().expect("32 bytes");
    let public = SigningKey::from_bytes(&seed).verifying_key().to_bytes();
    let mut group = c.benchmark_group("ed25519-keys");
    group.bench_function("expand-seed", |b| {
        b.iter(|| SigningKey::from_bytes(black_box(&seed)));
    });
    group.bench_function("decode-public-key", |b| {
        b.iter(|| {
            VerifyingKey::from_bytes(black_box(&public)).expect("valid");
        });
    });
    group.finish();
}

/// GTS-shaped: many COSE_Sign1 signatures over short structures, from a few
/// signers, verified back to back.
fn batch(c: &mut Bench) {
    const COUNT: usize = 64;
    let mut rng = SplitMix64::new(0x0ED2_5519 + 3);
    let signers: Vec<SigningKey> = (0..4).map(|_| key(&mut rng)).collect();
    let items: Vec<(VerifyingKey, Vec<u8>, Signature)> = (0..COUNT)
        .map(|i| {
            let signer = &signers[i % signers.len()];
            let message = bytes(&mut rng, 128 + (i % 8) * 64);
            let signature = signer.sign(&message);
            (signer.verifying_key(), message, signature)
        })
        .collect();
    let mut group = c.benchmark_group("ed25519-batch");
    group.throughput(Throughput::Elements(COUNT as u64));
    group.bench_function("verify-64", |b| {
        b.iter(|| {
            for (public, message, signature) in black_box(&items) {
                public.verify_strict(message, signature).expect("valid");
            }
        });
    });
    group.bench_function("sign-64", |b| {
        b.iter(|| {
            for (i, (_, message, _)) in black_box(&items).iter().enumerate() {
                black_box(signers[i % signers.len()].sign(message));
            }
        });
    });
    group.finish();
}

bench_group!(benches, sign, verify, keys, batch);
bench_main!(benches);
