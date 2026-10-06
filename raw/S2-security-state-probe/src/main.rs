// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use purrdf_hash::sha3::{Shake256, ShakeReader};

fn main() {
    let absorber_drop = core::mem::needs_drop::<Shake256>();
    let reader_drop = core::mem::needs_drop::<ShakeReader<256>>();
    println!("Shake256 needs_drop={absorber_drop}; ShakeReader256 needs_drop={reader_drop}");
    // Controlled fixture bytes, not production secrets. This is the actual
    // public XOF path used by ML-DSA's private-seed and mask derivations.
    let private_seed = [0x79; 64];
    let mut absorber = Shake256::new();
    absorber.update(&private_seed);
    absorber.update(&0u16.to_le_bytes());
    let mut reader = absorber.finalize();
    let mut output = [0; 640];
    reader.squeeze(&mut output);
    assert!(output.iter().any(|&byte| byte != 0));
    assert!(!absorber_drop && !reader_drop,
        "probe records the current unguarded state boundary; correction must change this result or add explicit clearing on every secret path");
    println!("Actual private-seed-shaped SHAKE path executed; no destructor exists for either owned state type.");
    println!("No freed-memory read, allocator-reuse claim, timing proof or key recovery is performed.");
}
