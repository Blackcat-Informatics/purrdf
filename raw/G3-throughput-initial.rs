// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
use std::{hint::black_box,time::Instant};
use purrdf_hash::sha3::{Sha3_256,Shake256,keccak_f1600};
use purrdf_gts::mldsa65::SigningKey;
fn measure(name:&str, iterations:usize, mut f:impl FnMut()) {
    for _ in 0..3 { f(); }
    let mut samples=Vec::new();
    for _ in 0..9 {
        let start=Instant::now();
        for _ in 0..iterations { f(); }
        samples.push(start.elapsed().as_nanos() as f64 / iterations as f64);
    }
    println!("{name}: iterations={iterations} ns_per_operation_samples={samples:?}");
}
fn main() {
    println!("variant={} assertions={}",std::env::args().nth(1).unwrap(),cfg!(debug_assertions));
    let input=vec![0xa3;65536];
    measure("sha3_256_64KiB",30,|| {black_box(Sha3_256::digest(black_box(&input)));});
    let secret=[0xa5;128];
    measure("shake256_128in_640out",1000,|| {let mut out=[0;640];Shake256::digest(black_box(&secret),&mut out);black_box(out);});
    measure("keccak_f1600",3000,|| {let mut state=[0xa5;25];keccak_f1600(black_box(&mut state));black_box(state);});
    let seed=[7;32]; let key=SigningKey::from_seed(&seed).unwrap(); let public=key.verifying_key();
    let signature=key.sign_deterministic(b"owned-state-throughput",b"").unwrap();
    public.verify(b"owned-state-throughput",b"",&signature).unwrap();
    println!("signature_identity={}",purrdf_hash::hex::encode(&purrdf_hash::blake3::hash(signature.as_bytes())));
    measure("mldsa65_keygen",30,|| {black_box(SigningKey::from_seed(black_box(&seed)).unwrap());});
    measure("mldsa65_sign",20,|| {black_box(key.sign_deterministic(black_box(b"owned-state-throughput"),b"").unwrap());});
    measure("mldsa65_verify",30,|| {public.verify(black_box(b"owned-state-throughput"),b"",black_box(&signature)).unwrap();});
}
