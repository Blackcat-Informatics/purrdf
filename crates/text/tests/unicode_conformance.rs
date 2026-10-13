// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `purrdf_text::unicode` against the Unicode Consortium's own conformance data
//! and property files, read from the vendored database under
//! `crates/iri/unicode/17.0.0/`.
//!
//! * `WordBreakTest.txt` and `SentenceBreakTest.txt` (`UAX #29`): every line,
//!   every boundary position, through the public borrowed iterators.
//! * `CaseFolding.txt`, `DerivedCoreProperties.txt` and `UnicodeData.txt`:
//!   the fold and the alphanumeric predicate checked for every scalar value
//!   against the files that define them.
//!
//! The normalization forms are checked against `NormalizationTest.txt` where
//! they are implemented, in `purrdf-lex`.

mod support {
    pub(crate) mod sentence_cases;
}

use purrdf_testkit::ucd::{code_point as hex, scalar, sequence, unicode_data};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use purrdf_text::unicode;

fn ucd(name: &str) -> String {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../iri/unicode/17.0.0")
        .join(name);
    fs::read_to_string(&path).unwrap_or_else(|err| panic!("reading {}: {err}", path.display()))
}

/// The data lines of a UCD property file: `(low, high, value)`.
fn property_ranges(text: &str) -> Vec<(u32, u32, String)> {
    text.lines()
        .filter_map(|line| {
            let data = line.split('#').next().unwrap_or("").trim();
            let (points, value) = data.split_once(';')?;
            let (low, high) = match points.trim().split_once("..") {
                Some((low, high)) => (hex(low), hex(high)),
                None => (hex(points), hex(points)),
            };
            Some((low, high, value.trim().to_owned()))
        })
        .collect()
}

/// The shared official break-file decoder, preserving UTF-8 tiling and every
/// boundary opportunity. Expected positions come only from the frozen corpus.
fn check_break_file(
    name: &str,
    text: &str,
    expected_count: usize,
    segments: impl Fn(&str, &mut dyn FnMut(&str)),
) {
    let mut lines = 0usize;
    let mut failures: Vec<String> = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let data = line.split('#').next().unwrap_or("").trim();
        if data.is_empty() {
            continue;
        }
        lines += 1;
        let mut input = String::new();
        let mut expected: Vec<usize> = Vec::new();
        for token in data.split_whitespace() {
            match token {
                "÷" => expected.push(input.len()),
                "×" => {}
                point => input.push(scalar(hex(point))),
            }
        }
        let mut actual = vec![0usize];
        let mut at = 0usize;
        segments(&input, &mut |segment| {
            assert_eq!(
                &input[at..at + segment.len()],
                segment,
                "segments must tile the input"
            );
            assert!(std::ptr::eq(segment.as_ptr(), input[at..].as_ptr()));
            at += segment.len();
            actual.push(at);
        });
        assert_eq!(
            at,
            input.len(),
            "{name} line {} must retain every byte",
            number + 1
        );
        if actual != expected {
            failures.push(format!(
                "line {}: {data}: boundaries {actual:?}, expected {expected:?}",
                number + 1
            ));
        }
    }
    assert_eq!(lines, expected_count, "{name} line count");
    assert!(
        failures.is_empty(),
        "{} of {lines} {name} lines fail; the first: {:#?}",
        failures.len(),
        &failures[..failures.len().min(20)]
    );
}

#[test]
fn word_break_test_every_line() {
    check_break_file(
        "WordBreakTest.txt",
        &ucd("WordBreakTest.txt"),
        1944,
        |input, emit| {
            unicode::word_bounds(input).for_each(emit);
        },
    );
}

#[test]
fn sentence_break_test_every_line() {
    let properties = ucd("SentenceBreakProperty.txt");
    let text = ucd("SentenceBreakTest.txt");
    assert_eq!(
        purrdf_testkit::vectors::sha256_hex(properties.as_bytes()),
        "871c0c985ad95125e25b302414065a10839d068970bceb383ecec138f22a0a18",
        "the property input is the frozen Unicode 17 release"
    );
    assert_eq!(
        purrdf_testkit::vectors::sha256_hex(text.as_bytes()),
        "12cb47d028ded0c1cb8a28558f95479cbcd24559c46977015c82f3b50a1cc6e4",
        "the complete corpus is the frozen Unicode 17 release"
    );
    assert_eq!(text.lines().next(), Some("# SentenceBreakTest-17.0.0.txt"));
    check_break_file("SentenceBreakTest.txt", &text, 512, |input, emit| {
        unicode::sentence_bounds(input).for_each(emit);
    });
    support::sentence_cases::the_sentence_boundaries_are_reproduced_on_this_target();
}

#[test]
fn the_fold_is_case_folding_txt_c_and_f_for_every_scalar() {
    let mut mapping: BTreeMap<u32, String> = BTreeMap::new();
    for line in ucd("CaseFolding.txt").lines() {
        let data = line.split('#').next().unwrap_or("").trim();
        let fields: Vec<&str> = data.split(';').map(str::trim).collect();
        if fields.len() >= 3 && matches!(fields[1], "C" | "F") {
            mapping.insert(hex(fields[0]), sequence(fields[2]));
        }
    }
    assert!(mapping.len() > 1500, "only {} C/F entries", mapping.len());
    for point in 0..=0x10_FFFF_u32 {
        let Some(c) = char::from_u32(point) else {
            continue;
        };
        let expected = mapping
            .get(&point)
            .cloned()
            .unwrap_or_else(|| c.to_string());
        assert_eq!(
            unicode::case_fold(&c.to_string()),
            expected,
            "fold of U+{point:04X}"
        );
    }
    assert_eq!(unicode::case_fold("STRASSE Straße"), "strasse strasse");
}

#[test]
fn is_alphanumeric_is_alphabetic_or_a_number_for_every_scalar() {
    let mut expected: BTreeSet<u32> = property_ranges(&ucd("DerivedCoreProperties.txt"))
        .into_iter()
        .filter(|(_, _, value)| value == "Alphabetic")
        .flat_map(|(low, high, _)| low..=high)
        .collect();
    for (&point, fields) in &unicode_data(&ucd("UnicodeData.txt")) {
        if matches!(fields[2].as_str(), "Nd" | "Nl" | "No") {
            expected.insert(point);
        }
    }
    for point in 0..=0x10_FFFF_u32 {
        let Some(c) = char::from_u32(point) else {
            continue;
        };
        assert_eq!(
            unicode::is_alphanumeric(c),
            expected.contains(&point),
            "is_alphanumeric(U+{point:04X})"
        );
    }
}

/// The streaming comparison agrees with building the form and comparing.
#[test]
fn compare_agrees_with_the_built_form() {
    for input in [
        "",
        "a",
        "A",
        "abc",
        "abcdefghijklmnop",
        "ß",
        "strasse",
        "e\u{301}",
        "\u{e9}",
        "ｒｕｓｔ",
        "rust",
        "abcdefgh\u{301}",
        "\u{301}abc",
        "中文",
    ] {
        let mut form = String::new();
        unicode::analysis_form(input, &mut form);
        for candidate in [input, form.as_str(), "", "x"] {
            let mut compare = unicode::Compare::new(candidate);
            unicode::analysis_form(input, &mut compare);
            assert_eq!(
                compare.finish(),
                form == candidate,
                "{input:?} against {candidate:?}"
            );
        }
    }
}

/// The analysis form is the composition of the public operations, in the
/// order the core specification §3.13 (D146) gives, then NFC.
#[test]
fn the_analysis_form_composes_the_public_operations() {
    for input in [
        "Hello World",
        "e\u{301}cole E\u{301}COLE",
        "ABCDEFGHIJKLMNOPQRSTUVWXYZ\u{301}\u{327}xyz",
        "caf\u{e9} CAF\u{c9}",
        "\u{212a}elvin \u{fb01}le",
        "a\u{308}\u{301}\u{323}b",
        "\u{345}\u{391}\u{345}",
        "\u{1e9e}\u{130}\u{fdfa}",
    ] {
        let mut whole = String::new();
        unicode::analysis_form(input, &mut whole);
        let composed = unicode::nfc(&unicode::nfkd(&unicode::case_fold(&unicode::nfkd(
            &unicode::case_fold(&unicode::nfd(input)),
        ))));
        assert_eq!(whole, composed, "{input:?}");
    }
}
