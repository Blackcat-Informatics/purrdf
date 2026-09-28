// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `purrdf_text::unicode` against the Unicode Consortium's own conformance data
//! and property files, read from the vendored database under
//! `crates/iri/unicode/17.0.0/`.
//!
//! * `NormalizationTest.txt` (`UAX #15` conformance clause UAX15-C3): every
//!   line, every column, every invariant its header states for part 1, and the
//!   part 2 invariant over every assigned code point part 1 does not list.
//! * `WordBreakTest.txt` (`UAX #29`): every line, every boundary position.
//! * `CaseFolding.txt`, `DerivedNormalizationProps.txt`,
//!   `DerivedCoreProperties.txt` and `UnicodeData.txt`: the fold, the
//!   composition exclusions and the alphanumeric predicate checked for every
//!   scalar value against the files that define them.

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

fn hex(text: &str) -> u32 {
    u32::from_str_radix(text.trim(), 16).unwrap_or_else(|err| panic!("bad hex {text:?}: {err}"))
}

fn scalar(point: u32) -> char {
    char::from_u32(point).unwrap_or_else(|| panic!("{point:04X} is not a scalar value"))
}

/// A space-separated code point sequence as a string.
fn sequence(field: &str) -> String {
    field.split_whitespace().map(|p| scalar(hex(p))).collect()
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

/// Every assigned code point and its `UnicodeData.txt` fields, with the
/// `First`/`Last` ranges expanded.
fn unicode_data() -> BTreeMap<u32, Vec<String>> {
    let text = ucd("UnicodeData.txt");
    let mut out = BTreeMap::new();
    let mut first: Option<u32> = None;
    for line in text.lines() {
        let fields: Vec<String> = line.split(';').map(str::to_owned).collect();
        let point = hex(&fields[0]);
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

fn show(text: &str) -> String {
    text.chars()
        .map(|c| format!("{:04X}", c as u32))
        .collect::<Vec<_>>()
        .join(" ")
}

/// One invariant of the `NormalizationTest.txt` header: a form's name, the
/// column it must produce, the columns it must produce it from, and the form.
type Check<'a> = (
    &'static str,
    &'a String,
    &'a [&'a String],
    fn(&str) -> String,
);

#[test]
fn normalization_test_every_line_every_column() {
    let text = ucd("NormalizationTest.txt");
    let mut part1: BTreeSet<u32> = BTreeSet::new();
    let mut part = "";
    let mut lines = 0usize;
    let mut failures: Vec<String> = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let data = line.split('#').next().unwrap_or("").trim();
        if data.is_empty() {
            continue;
        }
        if let Some(name) = data.strip_prefix('@') {
            part = if name.starts_with("Part1") { "1" } else { "" };
            continue;
        }
        let columns: Vec<String> = data.split(';').take(5).map(sequence).collect();
        let [c1, c2, c3, c4, c5] = &columns[..] else {
            panic!("line {}: fewer than five columns", number + 1);
        };
        lines += 1;
        if part == "1" {
            let mut chars = c1.chars();
            if let (Some(only), None) = (chars.next(), chars.next()) {
                part1.insert(only as u32);
            }
        }
        // The header's invariants, column by column.
        let checks: [Check<'_>; 4] = [
            ("NFC", c2, &[c1, c2, c3], unicode::nfc),
            ("NFD", c3, &[c1, c2, c3], unicode::nfd),
            ("NFKC", c4, &[c1, c2, c3, c4, c5], unicode::nfkc),
            ("NFKD", c5, &[c1, c2, c3, c4, c5], unicode::nfkd),
        ];
        let tail_checks: [Check<'_>; 2] = [
            ("NFC", c4, &[c4, c5], unicode::nfc),
            ("NFD", c5, &[c4, c5], unicode::nfd),
        ];
        for (form, expected, sources, apply) in checks.iter().chain(tail_checks.iter()) {
            for source in *sources {
                let actual = apply(source);
                if &actual != *expected {
                    failures.push(format!(
                        "line {}: {form}({}) = {}, expected {}",
                        number + 1,
                        show(source),
                        show(&actual),
                        show(expected)
                    ));
                }
            }
        }
    }
    assert!(
        lines > 19_000,
        "NormalizationTest.txt yielded only {lines} lines"
    );
    assert!(
        failures.is_empty(),
        "{} NormalizationTest.txt failures of {lines} lines; the first: {:#?}",
        failures.len(),
        &failures[..failures.len().min(20)]
    );

    // Part 2: every assigned code point part 1 does not list is its own form
    // under all four.
    let mut checked = 0usize;
    for &point in unicode_data().keys() {
        if part1.contains(&point) {
            continue;
        }
        let Some(c) = char::from_u32(point) else {
            continue; // surrogates are assigned but are not scalar values
        };
        let x = c.to_string();
        for (form, apply) in [
            ("NFC", unicode::nfc as fn(&str) -> String),
            ("NFD", unicode::nfd),
            ("NFKC", unicode::nfkc),
            ("NFKD", unicode::nfkd),
        ] {
            assert_eq!(apply(&x), x, "part 2: {form}(U+{point:04X}) is not itself");
        }
        checked += 1;
    }
    assert!(
        checked > 250_000,
        "part 2 checked only {checked} code points"
    );
}

#[test]
fn word_break_test_every_line() {
    let text = ucd("WordBreakTest.txt");
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
        for segment in unicode::word_bounds(&input) {
            assert_eq!(
                &input[at..at + segment.len()],
                segment,
                "segments must tile the input"
            );
            at += segment.len();
            actual.push(at);
        }
        if actual != expected {
            failures.push(format!(
                "line {}: {data}: boundaries {actual:?}, expected {expected:?}",
                number + 1
            ));
        }
    }
    assert_eq!(lines, 1944, "WordBreakTest.txt line count");
    assert!(
        failures.is_empty(),
        "{} of {lines} WordBreakTest.txt lines fail; the first: {:#?}",
        failures.len(),
        &failures[..failures.len().min(20)]
    );
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

/// UAX 15 §5: a character with a canonical decomposition is recomposed by
/// NFC exactly when it is not `Full_Composition_Exclusion`. The generator
/// derives the exclusions from `CompositionExclusions.txt` plus singletons
/// plus non-starter decompositions; `DerivedNormalizationProps.txt` publishes
/// the derived property, and the two must agree.
#[test]
fn composition_exclusions_agree_with_the_derived_property() {
    let excluded: BTreeSet<u32> = property_ranges(&ucd("DerivedNormalizationProps.txt"))
        .into_iter()
        .filter(|(_, _, value)| value == "Full_Composition_Exclusion")
        .flat_map(|(low, high, _)| low..=high)
        .collect();
    assert!(!excluded.is_empty());
    let mut decomposable = 0usize;
    for (&point, fields) in &unicode_data() {
        let mapping = fields[5].trim();
        if mapping.is_empty() || mapping.starts_with('<') {
            continue;
        }
        decomposable += 1;
        let c = scalar(point).to_string();
        assert_eq!(
            unicode::nfc(&c) == c,
            !excluded.contains(&point),
            "U+{point:04X}: NFC recomposition disagrees with Full_Composition_Exclusion"
        );
    }
    assert!(decomposable > 2000);
}

#[test]
fn is_alphanumeric_is_alphabetic_or_a_number_for_every_scalar() {
    let mut expected: BTreeSet<u32> = property_ranges(&ucd("DerivedCoreProperties.txt"))
        .into_iter()
        .filter(|(_, _, value)| value == "Alphabetic")
        .flat_map(|(low, high, _)| low..=high)
        .collect();
    for (&point, fields) in &unicode_data() {
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
