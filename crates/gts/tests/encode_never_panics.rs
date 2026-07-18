// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Standing "never panics" guard for the GTS codec (§8) transform chain.
//!
//! Task 1 (commit fd1e5f2) made the zstd encode path total against an
//! internal huff0-encoder panic (`guarded_encode` in `crates/gts/src/codec.rs`).
//! This file is the permanent regression guard that huff0 panic motivated: it
//! sweeps a broad, deterministic spread of generated inputs through
//! `encode_chain`, and a broad spread of arbitrary/truncated/garbage bytes
//! through `decode_chain` and `frame_block_kinds`, and asserts none of them
//! ever unwinds — encode and decode are both TOTAL over their input domains
//! (`Result`, never a panic).
//!
//! All inputs below are built with a fixed-seed LCG (never `rand`/time), so a
//! failing case reproduces byte-for-byte on every run.

use std::panic::{AssertUnwindSafe, catch_unwind};

use purrdf_gts::codec::{Codec, decode_chain, encode_chain, frame_block_kinds};

/// A tiny deterministic linear congruential generator (Numerical Recipes
/// constants) — no `rand`, no time, fully reproducible across runs.
struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 32) as u32
    }

    fn bytes(&mut self, len: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(len);
        while out.len() < len {
            out.extend_from_slice(&self.next_u32().to_le_bytes());
        }
        out.truncate(len);
        out
    }
}

/// A broad, deterministic spread of byte inputs covering: empty, tiny,
/// medium, and large (> 64 KiB, crossing the rsyncable block boundary and
/// forcing multiple blocks); all-same-byte, small-alphabet, high-entropy
/// LCG, real-ish repeated text, and skewed-histogram distributions.
fn generated_inputs() -> Vec<Vec<u8>> {
    let mut inputs = vec![
        Vec::new(),   // empty
        vec![0u8],    // single byte, minimum
        vec![0xFFu8], // single byte, maximum
    ];

    // Tiny inputs (1..16 bytes) over a couple of deterministic seeds.
    for seed in [1u64, 7u64] {
        let mut lcg = Lcg::new(seed);
        for len in 1..16usize {
            inputs.push(lcg.bytes(len));
        }
    }

    // All-same-byte inputs at a few sizes.
    for &byte in &[0x00u8, 0xFFu8, 0x41u8] {
        for &len in &[16usize, 4096, 70_000] {
            inputs.push(vec![byte; len]);
        }
    }

    // Small-alphabet inputs (2-symbol and 4-symbol) at medium size.
    {
        let mut lcg = Lcg::new(11);
        let mut two_symbol = Vec::with_capacity(8192);
        for _ in 0..8192 {
            two_symbol.push(if lcg.next_u32() & 1 == 0 { b'a' } else { b'b' });
        }
        inputs.push(two_symbol);

        let mut lcg = Lcg::new(13);
        let alphabet = [b'w', b'x', b'y', b'z'];
        let mut four_symbol = Vec::with_capacity(8192);
        for _ in 0..8192 {
            four_symbol.push(alphabet[(lcg.next_u32() % 4) as usize]);
        }
        inputs.push(four_symbol);
    }

    // High-entropy LCG bytes, medium and large (crosses the 64 KiB rsyncable
    // block boundary and forces multiple blocks).
    inputs.push(Lcg::new(101).bytes(4096));
    inputs.push(Lcg::new(103).bytes(70_000));
    inputs.push(Lcg::new(107).bytes(200_000));

    // Real-ish repeated text, a size that does not evenly divide the
    // rsyncable block size, to exercise a ragged final block.
    let text = b"<https://example.org/s> <https://example.org/p> \"claim about cats\" .\n";
    inputs.push(text.repeat(1200));

    // Skewed histogram: mostly one byte, with rare other bytes scattered in
    // (a realistic "mostly-zero padding with sparse structure" shape).
    {
        let mut lcg = Lcg::new(211);
        let mut skewed = vec![0u8; 20_000];
        for _ in 0..200 {
            let idx = (lcg.next_u32() as usize) % skewed.len();
            skewed[idx] = (lcg.next_u32() % 256) as u8;
        }
        inputs.push(skewed);
    }

    inputs
}

/// Run `f` under `catch_unwind`, asserting it never unwinds. Returns the
/// value on success (never returns on panic — the test fails first).
fn assert_no_panic<T>(label: &str, f: impl FnOnce() -> T + std::panic::UnwindSafe) -> T {
    catch_unwind(f).unwrap_or_else(|_| panic!("{label} panicked instead of returning a Result"))
}

#[test]
fn encode_chain_never_panics_over_generated_inputs() {
    let inputs = generated_inputs();
    assert!(inputs.len() >= 20, "expected a broad generated spread");

    for (i, input) in inputs.iter().enumerate() {
        for chain_name in ["zstd", "zstd-rsyncable"] {
            let chain = vec![chain_name.to_string()];
            let label = format!(
                "encode_chain([{chain_name}]) on input #{i} (len {})",
                input.len()
            );
            let input_ref = input.as_slice();
            let result =
                assert_no_panic(&label, AssertUnwindSafe(|| encode_chain(&chain, input_ref)));
            let encoded = result.unwrap_or_else(|e| panic!("{label} must succeed, got {e}"));

            // Round-trip a sampling back through decode_chain to prove the
            // output is a genuinely valid, decodable frame — not just "some
            // bytes came out".
            let codec = Codec::new(chain_name, "compress");
            let decoded = decode_chain(std::slice::from_ref(&codec), &encoded)
                .unwrap_or_else(|e| panic!("{label}: round-trip decode must succeed, got {e}"));
            assert_eq!(
                &decoded, input,
                "{label}: round-trip must reproduce the original bytes exactly"
            );
        }
    }
}

/// Arbitrary/truncated/garbage byte inputs that need NOT be valid zstd
/// frames — the decoder must stay total (`Ok`/`Err`, never a panic) over
/// this untrusted-input surface.
fn arbitrary_decode_inputs() -> Vec<Vec<u8>> {
    let mut inputs = vec![
        Vec::new(),
        vec![0x00u8],
        vec![0xFFu8],
        vec![0x28u8, 0xB5u8], // partial zstd magic
    ];

    // All-0x00 and all-0xFF buffers at a few sizes.
    for &len in &[1usize, 8, 64, 4096] {
        inputs.push(vec![0x00u8; len]);
        inputs.push(vec![0xFFu8; len]);
    }

    // Random-ish (LCG) bytes at a few sizes.
    for (seed, len) in [(31u64, 8usize), (37, 64), (41, 4096)] {
        inputs.push(Lcg::new(seed).bytes(len));
    }

    // Truncated valid zstd frames: compress something real, then cut it at
    // several lengths, including mid-header and mid-block-body cuts.
    let payload =
        b"real payload with enough structure to build a multi-block zstd frame".repeat(4000);
    let full_frame = encode_chain(&["zstd".to_string()], &payload).expect("zstd encodes");
    for &cut in &[0usize, 1, 2, 3, 4, 5, 8, 16, 32, 64] {
        if cut <= full_frame.len() {
            inputs.push(full_frame[..cut].to_vec());
        }
    }
    // A handful of cuts spread across the rest of the frame, including one
    // that lands just short of the end (truncated checksum/trailer).
    let frame_len = full_frame.len();
    for frac in [4usize, 3, 2] {
        let cut = frame_len / frac;
        inputs.push(full_frame[..cut].to_vec());
    }
    if frame_len > 1 {
        inputs.push(full_frame[..frame_len - 1].to_vec());
    }

    // Same treatment for a rsyncable (multi-frame) encode.
    let rsync_payload = vec![b'q'; 200_000];
    let full_rsync = encode_chain(&["zstd-rsyncable".to_string()], &rsync_payload)
        .expect("zstd-rsyncable encodes");
    let rsync_len = full_rsync.len();
    for frac in [8usize, 4, 2] {
        let cut = rsync_len / frac;
        inputs.push(full_rsync[..cut].to_vec());
    }

    inputs
}

#[test]
fn decode_chain_never_panics_over_arbitrary_bytes() {
    let inputs = arbitrary_decode_inputs();
    assert!(inputs.len() >= 15, "expected a broad arbitrary-byte spread");

    let chain = [Codec::new("zstd", "compress")];
    for (i, input) in inputs.iter().enumerate() {
        let label = format!("decode_chain on arbitrary input #{i} (len {})", input.len());
        let input_ref = input.as_slice();
        let chain_ref = &chain;
        // The point of this assertion is totality, not success: garbage
        // bytes are expected to yield `Err`, never an unwind.
        let _ = assert_no_panic(
            &label,
            AssertUnwindSafe(|| decode_chain(chain_ref, input_ref)),
        );
    }
}

#[test]
fn frame_block_kinds_never_panics_over_arbitrary_bytes() {
    let inputs = arbitrary_decode_inputs();
    assert!(inputs.len() >= 15, "expected a broad arbitrary-byte spread");

    for (i, input) in inputs.iter().enumerate() {
        let label = format!(
            "frame_block_kinds on arbitrary input #{i} (len {})",
            input.len()
        );
        let input_ref = input.as_slice();
        let _ = assert_no_panic(&label, AssertUnwindSafe(|| frame_block_kinds(input_ref)));
    }
}
