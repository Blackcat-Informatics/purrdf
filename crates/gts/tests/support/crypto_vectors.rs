// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Decode frozen byte fields and report the first complete-answer mismatch.

pub(super) fn bytes(field: &str) -> Vec<u8> {
    let text = purrdf_testkit::vectors::decode_str(field).unwrap();
    purrdf_hash::hex::decode(&text).unwrap()
}

pub(super) fn equal_bytes(actual: &[u8], expected: &[u8], identity: &str) {
    assert_eq!(actual.len(), expected.len(), "{identity}: byte length");
    let difference = actual.iter().zip(expected).position(|(a, b)| a != b);
    assert_eq!(difference, None, "{identity}: first differing byte");
}
