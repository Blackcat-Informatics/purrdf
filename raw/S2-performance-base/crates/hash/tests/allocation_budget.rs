// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Hashing owns bounded state and must never allocate for any message size.

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_hash::{Digest, blake3, sha3::Sha3_256};
use std::hint::black_box;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

#[test]
fn creation_streaming_finalization_and_reset_allocate_nothing() {
    let bytes = vec![0x5a; 1024 * 1024 + 1];
    for length in [0, 1, 64, 1024, 16384, bytes.len()] {
        let input = black_box(&bytes[..length]);
        let window = CurrentThreadWindow::open();
        black_box(blake3::hash(input));
        black_box(Sha3_256::digest(input));
        let mut bulk = blake3::Hasher::new();
        let mut record = blake3::RecordHasher::new();
        let mut sha3 = Sha3_256::new();
        for part in input.chunks(65) {
            bulk.update(part);
            record.update(part);
            sha3.update(part);
        }
        black_box(bulk.finalize());
        black_box(record.finalize());
        black_box(sha3.clone().finalize());
        bulk.reset();
        record.reset();
        sha3.reset();
        black_box((bulk, record, sha3));
        let measured = window.close();
        assert_eq!(measured.allocations, 0, "length={length}: {measured:?}");
        assert_eq!(measured.requested_bytes, 0, "length={length}: {measured:?}");
    }
}
