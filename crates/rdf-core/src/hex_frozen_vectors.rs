// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The frozen base16 tables, read for the differential over this crate's hex
//! encoders and decoders.

use purrdf_testkit::vectors::{VectorFile, decode_str};

const ENCODINGS: &str = include_str!("../../hash-conformance/tests/vectors/hex_vectors.txt");
const DIGITS: &str = include_str!("../../hash-conformance/tests/vectors/hex_digit_vectors.txt");

/// Every record: input bytes, lowercase rendering, uppercase rendering.
pub(crate) fn encodings() -> Vec<(Vec<u8>, String, String)> {
    let file = VectorFile::parse(ENCODINGS).expect("hex_vectors.txt");
    file.records()
        .iter()
        .map(|record| {
            let len: usize = record.fields[0].parse().expect("length");
            let first: u8 = record.fields[1].parse().expect("first");
            let input = (0..len).map(|i| first.wrapping_add(i as u8)).collect();
            (
                input,
                decode_str(record.fields[2]).expect("lower"),
                decode_str(record.fields[3]).expect("upper"),
            )
        })
        .collect()
}

/// Every byte: its any-case, lowercase-canonical and uppercase-canonical value.
pub(crate) fn digits() -> Vec<(u8, Option<u8>, Option<u8>, Option<u8>)> {
    let file = VectorFile::parse(DIGITS).expect("hex_digit_vectors.txt");
    let value = |field: &str| (field != "-").then(|| field.parse().expect("digit value"));
    file.records()
        .iter()
        .map(|record| {
            (
                record.fields[0].parse().expect("byte"),
                value(record.fields[1]),
                value(record.fields[2]),
                value(record.fields[3]),
            )
        })
        .collect()
}
