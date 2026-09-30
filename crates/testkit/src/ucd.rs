// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Readers for the Unicode Character Database's text files, for the conformance
//! tests that replay them.
//!
//! Every UCD file spells code points as hexadecimal fields; the normalization and
//! text-analysis suites read the same fields the same way, so the readers live here
//! once. A malformed field is a broken fixture, so each reader panics naming it.

use std::collections::BTreeMap;

/// The code point a hexadecimal UCD field spells (surrounding whitespace ignored).
///
/// # Panics
///
/// If `field` is not hexadecimal or does not fit a `u32`.
#[must_use]
pub fn code_point(field: &str) -> u32 {
    purrdf_hash::hex::parse_u32(field.trim().as_bytes())
        .unwrap_or_else(|| panic!("bad hex code point {field:?}"))
}

/// The scalar value `point` names.
///
/// # Panics
///
/// If `point` is a surrogate or beyond U+10FFFF.
#[must_use]
pub fn scalar(point: u32) -> char {
    char::from_u32(point).unwrap_or_else(|| panic!("{point:04X} is not a scalar value"))
}

/// A space-separated code point sequence (a `NormalizationTest.txt` column) as a
/// string.
///
/// # Panics
///
/// If a code point is malformed or not a scalar value.
#[must_use]
pub fn sequence(field: &str) -> String {
    field
        .split_whitespace()
        .map(|point| scalar(code_point(point)))
        .collect()
}

/// `point` as a generated table spells it: a separated eight-digit hex literal,
/// `0x0001_F600`, which the `unreadable_literal` lint accepts at every width.
#[must_use]
pub fn code_point_literal(point: u32) -> String {
    format!("0x{:04X}_{:04X}", point >> 16, point & 0xFFFF)
}

/// Every assigned code point of a `UnicodeData.txt` text and its `;`-separated
/// fields, with the `<…, First>`/`<…, Last>` ranges expanded.
///
/// # Panics
///
/// If a range's `Last` line has no `First` before it, or a code point is malformed.
#[must_use]
pub fn unicode_data(text: &str) -> BTreeMap<u32, Vec<String>> {
    let mut out = BTreeMap::new();
    let mut first: Option<u32> = None;
    for line in text.lines() {
        let fields: Vec<String> = line.split(';').map(str::to_owned).collect();
        let point = code_point(&fields[0]);
        if fields[1].ends_with(", First>") {
            first = Some(point);
            continue;
        }
        let low = if fields[1].ends_with(", Last>") {
            first
                .take()
                .unwrap_or_else(|| panic!("range end {point:04X} without a start"))
        } else {
            point
        };
        for p in low..=point {
            out.insert(p, fields.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{code_point, sequence, unicode_data};

    #[test]
    fn fields_and_ranges_read_as_the_database_spells_them() {
        assert_eq!(code_point(" 00E9 "), 0xE9);
        assert_eq!(sequence("0065 0301"), "e\u{301}");
        let data = unicode_data(
            "0041;LATIN CAPITAL LETTER A;Lu\nAC00;<Hangul Syllable, First>;Lo\nAC02;<Hangul Syllable, Last>;Lo\n",
        );
        assert_eq!(
            data.keys().copied().collect::<Vec<_>>(),
            [0x41, 0xAC00, 0xAC01, 0xAC02]
        );
        assert_eq!(data[&0xAC01][1], "<Hangul Syllable, Last>");
    }
}
