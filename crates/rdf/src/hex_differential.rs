// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The frozen base16 tables, replayed through this crate's hex renderers and
//! readers.

use purrdf_testkit::vectors::{VectorFile, decode_str};

const ENCODINGS: &str = include_str!("../../hash-conformance/tests/vectors/hex_vectors.txt");
const DIGITS: &str = include_str!("../../hash-conformance/tests/vectors/hex_digit_vectors.txt");

/// Every frozen record: input bytes, lowercase and uppercase renderings.
pub(crate) fn encodings() -> Vec<(Vec<u8>, String, String)> {
    let file = VectorFile::parse(ENCODINGS).expect("hex vectors");
    file.records()
        .iter()
        .map(|r| {
            let len: usize = r.fields[0].parse().unwrap();
            let first: u8 = r.fields[1].parse().unwrap();
            let input = (0..len).map(|i| first.wrapping_add(i as u8)).collect();
            (
                input,
                decode_str(r.fields[2]).unwrap(),
                decode_str(r.fields[3]).unwrap(),
            )
        })
        .collect()
}

/// Each byte's two uppercase digits, from the one-byte records.
pub(crate) fn upper_pairs() -> Vec<String> {
    let mut pairs = vec![String::new(); 256];
    for (input, _, upper) in encodings() {
        if input.len() == 1 {
            pairs[usize::from(input[0])] = upper;
        }
    }
    pairs
}

/// The digit table: (any case, canonical lowercase) values per byte.
pub(crate) fn digits() -> Vec<(Option<u8>, Option<u8>)> {
    let file = VectorFile::parse(DIGITS).expect("digit vectors");
    file.records()
        .iter()
        .map(|r| (r.fields[1].parse().ok(), r.fields[2].parse().ok()))
        .collect()
}

/// A percent-encoder escapes each byte it does not pass through as `%` and
/// the frozen uppercase pair, for every ASCII scalar and non-ASCII scalars
/// whose UTF-8 bytes cover every lead and continuation byte.
pub(crate) fn assert_percent_escapes(encode: impl Fn(&str) -> String) {
    let pairs = upper_pairs();
    let probes = (0u32..0x800)
        .chain((0x800..=0xFFFF).step_by(0x1000))
        .chain((0x10000..=0x10_FFFF).step_by(0x40000))
        .filter_map(char::from_u32);
    for probe in probes {
        let text = probe.to_string();
        let escaped: String = text
            .bytes()
            .map(|b| format!("%{}", pairs[usize::from(b)]))
            .collect();
        let out = encode(&text);
        if probe.is_ascii() {
            assert!(out == text || out == escaped, "{probe:?}: {out}");
        } else {
            assert_eq!(out, escaped, "{probe:?}");
        }
    }
}
