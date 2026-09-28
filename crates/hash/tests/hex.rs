// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Base16 rendering: the RFC 4648 §10 test vectors, every byte value, the
//! formatter options, and every encoding path this host can run against the
//! portable one at every length 0..=256 and every input and output alignment
//! 0..16.
//!
//! The target is `harness = false` on `purrdf_testkit`'s runner, so the same
//! cases run natively under `cargo test` and on `wasm32-unknown-unknown` in
//! Node (`make wasm-test`), baseline and `+simd128`.

use purrdf_hash::backend::HexBackend;
use purrdf_hash::hex::Lower;
use purrdf_testkit::rng::Xoshiro256;

/// RFC 4648 §10's BASE16 vectors. The RFC prints them in upper case; §8
/// calls base16 case-insensitive, and `Lower` renders the lower-case form.
fn rfc4648_base16_vectors() {
    let vectors: [(&str, &str); 7] = [
        ("", ""),
        ("f", "66"),
        ("fo", "666F"),
        ("foo", "666F6F"),
        ("foob", "666F6F62"),
        ("fooba", "666F6F6261"),
        ("foobar", "666F6F626172"),
    ];
    for (input, expected) in vectors {
        let expected = expected.to_ascii_lowercase();
        assert_eq!(Lower(input.as_bytes()).to_string(), expected, "{input:?}");
        for backend in available() {
            let mut out = vec![0u8; 2 * input.len()];
            backend
                .encode(input.as_bytes(), &mut out)
                .expect("an available path");
            assert_eq!(out, expected.as_bytes(), "{input:?} on {}", backend.name());
        }
    }
}

/// Every byte value renders as its two-digit, zero-padded `{:02x}`, and a
/// sequence renders as the concatenation.
fn every_byte_value_matches_lower_hex() {
    let all: Vec<u8> = (0..=255).collect();
    let mut expected = String::new();
    for byte in &all {
        let single = format!("{byte:02x}");
        assert_eq!(Lower(&[*byte]).to_string(), single);
        expected.push_str(&single);
    }
    assert_eq!(Lower(&all).to_string(), expected);
    // Array and vector references coerce into the field.
    let digest: [u8; 4] = [0xde, 0xad, 0xbe, 0xef];
    assert_eq!(Lower(&digest).to_string(), "deadbeef");
    assert_eq!(Lower(&all[..2]).to_string(), "0001");
}

/// Longer than one 128-byte chunk: chunk seams must not lose or repeat a
/// character.
fn multi_chunk_inputs_match_portable() {
    for len in [127, 128, 129, 255, 256, 257, 1000, 4096 + 7] {
        let data = input(len, len as u64);
        assert_eq!(Lower(&data).to_string(), portable(&data), "length {len}");
    }
}

/// Width, fill, alignment and precision act on the characters as they do on
/// the same text as a `str`, including across chunk seams.
fn formatter_options_match_str() {
    for len in [0, 1, 3, 32, 200] {
        let data = input(len, 7);
        let text = portable(&data);
        let rendered = Lower(&data);
        assert_eq!(format!("{rendered:>70}"), format!("{text:>70}"));
        assert_eq!(format!("{rendered:<70}"), format!("{text:<70}"));
        assert_eq!(format!("{rendered:*^71}"), format!("{text:*^71}"));
        assert_eq!(format!("{rendered:70}"), format!("{text:70}"));
        assert_eq!(format!("{rendered:.5}"), format!("{text:.5}"));
        assert_eq!(format!("{rendered:.300}"), format!("{text:.300}"));
        assert_eq!(format!("{rendered:-^9.3}"), format!("{text:-^9.3}"));
        assert_eq!(format!("{rendered:.0}"), "");
    }
}

/// `encode` refuses an output of the wrong length, writing nothing, beside
/// the exactly-sized neighbour it accepts.
fn encode_refuses_a_missized_output() {
    let data = input(40, 3);
    for backend in available() {
        let mut exact = vec![0u8; 80];
        assert_eq!(backend.encode(&data, &mut exact), Some(()));
        assert_eq!(exact, portable(&data).as_bytes());
        for wrong in [79, 81, 0] {
            let mut out = vec![0xAAu8; wrong];
            assert_eq!(backend.encode(&data, &mut out), None, "{wrong} bytes");
            assert!(out.iter().all(|&b| b == 0xAA), "nothing written");
        }
    }
}

/// Every available path against the portable one, at every length 0..=256
/// and every input and output offset 0..16 into their buffers.
fn every_path_matches_portable() {
    let source = input(256 + 16, 0x5eed);
    let mut out_buffer = vec![0u8; 2 * 256 + 16];
    let mut ran = Vec::new();
    for backend in available() {
        for len in 0..=256 {
            for in_offset in 0..16 {
                let data = &source[in_offset..in_offset + len];
                let expected = portable(data);
                for out_offset in [in_offset, (in_offset + 5) % 16] {
                    out_buffer.fill(0);
                    let out = &mut out_buffer[out_offset..out_offset + 2 * len];
                    backend.encode(data, out).expect("an available path");
                    assert_eq!(
                        out,
                        expected.as_bytes(),
                        "{} at length {len}, input offset {in_offset}, output offset {out_offset}",
                        backend.name()
                    );
                    // Nothing written outside the output window.
                    assert!(out_buffer[..out_offset].iter().all(|&b| b == 0));
                    assert!(out_buffer[out_offset + 2 * len..].iter().all(|&b| b == 0));
                }
            }
        }
        ran.push(backend.name());
    }
    purrdf_testkit::harness::print_line(&format!(
        "hex: {} checked; {} selected",
        ran.join(", "),
        HexBackend::selected().name()
    ));
    assert!(ran.contains(&"portable"));
    // A path the build or processor provides must be the one selected.
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    assert_eq!(HexBackend::selected(), HexBackend::Wasm32Simd128);
    #[cfg(all(target_arch = "wasm32", not(target_feature = "simd128")))]
    assert_eq!(HexBackend::selected(), HexBackend::Portable);
    #[cfg(target_arch = "aarch64")]
    assert_eq!(HexBackend::selected(), HexBackend::Aarch64Neon);
}

fn available() -> impl Iterator<Item = HexBackend> {
    HexBackend::ALL
        .into_iter()
        .filter(|backend| backend.is_available())
}

/// The reference rendering, independent of the crate's encoder.
fn portable(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

fn input(length: usize, seed: u64) -> Vec<u8> {
    let mut rng = Xoshiro256::from_seed(seed);
    let mut bytes = Vec::with_capacity(length + 8);
    while bytes.len() < length {
        bytes.extend_from_slice(&rng.next_u64().to_le_bytes());
    }
    bytes.truncate(length);
    bytes
}

purrdf_testkit::harness_main!(
    rfc4648_base16_vectors,
    every_byte_value_matches_lower_hex,
    multi_chunk_inputs_match_portable,
    formatter_options_match_str,
    encode_refuses_a_missized_output,
    every_path_matches_portable,
);
