// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `purrdf_lex::unicode` against the frozen normalization vectors and the
//! Unicode Consortium's own conformance data.
//!
//! * `tests/vectors/unicode_normalization_vectors.txt`: the four forms, the
//!   combining class and the NFC verdict of every scalar value, of every
//!   `NormalizationTest.txt` source string, and of 20,000 mixed strings.
//! * `tests/vectors/normalization_differential_vectors.txt`: the four forms of
//!   every scalar value, as an independent implementation answered them.
//! * `NormalizationTest.txt` (`UAX #15` conformance clause UAX15-C3), read from
//!   the vendored database under `crates/iri/unicode/17.0.0/`: every line,
//!   every column, every invariant its header states for part 1, and the part 2
//!   invariant over every assigned code point part 1 does not list.
//! * `DerivedNormalizationProps.txt`: NFC recomposes a canonical decomposition
//!   exactly when its source is not `Full_Composition_Exclusion`.
//!
//! Each vector file holds answers recorded once and never edited: a
//! disagreement is a defect in the pipeline, never a reason to touch a vector.

use purrdf_testkit::ucd::{code_point as hex, scalar, sequence, unicode_data};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use purrdf_lex::unicode::{UNICODE_VERSION, ccc, is_nfc, nfc, nfd, nfkc, nfkd};
use purrdf_testkit::vectors::{VectorFile, decode_str, encode_str};

fn ucd(name: &str) -> String {
    let (major, minor, patch) = UNICODE_VERSION;
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../iri/unicode/{major}.{minor}.{patch}"))
        .join(name);
    fs::read_to_string(&path).unwrap_or_else(|err| panic!("reading {}: {err}", path.display()))
}

fn show(text: &str) -> String {
    text.chars()
        .map(|c| format!("{:04X}", c as u32))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The answers the frozen vectors record for `input`: NFD, NFC, NFKD, NFKC and
/// the NFC verdict.
fn forms(input: &str) -> [String; 5] {
    [
        encode_str(&nfd(input)),
        encode_str(&nfc(input)),
        encode_str(&nfkd(input)),
        encode_str(&nfkc(input)),
        if is_nfc(input) { "1" } else { "0" }.to_owned(),
    ]
}

/// Every record of the frozen file reproduces, and every scalar value the file
/// does not list is a starter that is its own form under all four.
#[test]
fn every_frozen_normalization_answer_is_reproduced() {
    let file = VectorFile::load(include_str!("vectors/unicode_normalization_vectors.txt"));
    let mut listed: BTreeSet<u32> = BTreeSet::new();
    let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
    for record in file.records() {
        let (input, class) = match record.fields[0] {
            "scalar" => {
                let point = hex(record.fields[1]);
                listed.insert(point);
                let c = scalar(point);
                (c.to_string(), ccc(c).to_string())
            }
            "sequence" | "mixed" => (
                decode_str(record.fields[1]).expect("an encoded input"),
                "-".to_owned(),
            ),
            other => panic!("line {}: unknown kind {other}", record.line),
        };
        *kinds.entry(record.fields[0]).or_default() += 1;
        assert_eq!(class, record.fields[2], "line {}: class", record.line);
        let answers = forms(&input);
        for (column, answer) in answers.iter().enumerate() {
            assert_eq!(
                answer,
                record.fields[3 + column],
                "line {}: column {} of {}",
                record.line,
                4 + column,
                show(&input)
            );
        }
    }
    assert_eq!(
        kinds,
        BTreeMap::from([("mixed", 20_000), ("scalar", 18_050), ("sequence", 20_034)])
    );
    for point in 0..=0x10_FFFF_u32 {
        let Some(c) = char::from_u32(point) else {
            continue;
        };
        if listed.contains(&point) {
            continue;
        }
        let x = c.to_string();
        assert_eq!(ccc(c), 0, "U+{point:04X} is unlisted, so it is a starter");
        assert!(
            nfd(&x) == x && nfc(&x) == x && nfkd(&x) == x && nfkc(&x) == x && is_nfc(&x),
            "U+{point:04X} is unlisted, so it must be its own form under all four"
        );
    }
}

#[test]
fn the_normalization_forms_replay_the_independent_answers_exactly() {
    let file = VectorFile::load(include_str!(
        "vectors/normalization_differential_vectors.txt"
    ));
    let replayed = file
        .replay(1, |fields| {
            let x = scalar(hex(fields[0])).to_string();
            [nfd(&x), nfc(&x), nfkd(&x), nfkc(&x)]
                .iter()
                .map(|form| encode_str(form))
                .collect()
        })
        .unwrap_or_else(|mismatch| panic!("{mismatch}"));
    assert_eq!(replayed, 17_086);
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
            ("NFC", c2, &[c1, c2, c3], nfc),
            ("NFD", c3, &[c1, c2, c3], nfd),
            ("NFKC", c4, &[c1, c2, c3, c4, c5], nfkc),
            ("NFKD", c5, &[c1, c2, c3, c4, c5], nfkd),
        ];
        let tail_checks: [Check<'_>; 2] =
            [("NFC", c4, &[c4, c5], nfc), ("NFD", c5, &[c4, c5], nfd)];
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
        // The NFC verdict: a string is NFC exactly when it is its NFC column
        // (c2 for c1..c3, and c4 for c4 and c5).
        for (source, its_nfc) in [(c1, c2), (c2, c2), (c3, c2), (c4, c4), (c5, c4)] {
            if is_nfc(source) != (source == its_nfc) {
                failures.push(format!("line {}: is_nfc({})", number + 1, show(source)));
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
    for &point in unicode_data(&ucd("UnicodeData.txt")).keys() {
        if part1.contains(&point) {
            continue;
        }
        let Some(c) = char::from_u32(point) else {
            continue; // surrogates are assigned but are not scalar values
        };
        let x = c.to_string();
        for (form, apply) in [
            ("NFC", nfc as fn(&str) -> String),
            ("NFD", nfd),
            ("NFKC", nfkc),
            ("NFKD", nfkd),
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

/// UAX 15 §5: a character with a canonical decomposition is recomposed by
/// NFC exactly when it is not `Full_Composition_Exclusion`. The generator
/// derives the exclusions from `CompositionExclusions.txt` plus singletons
/// plus non-starter decompositions; `DerivedNormalizationProps.txt` publishes
/// the derived property, and the two must agree.
#[test]
fn composition_exclusions_agree_with_the_derived_property() {
    let excluded: BTreeSet<u32> = ucd("DerivedNormalizationProps.txt")
        .lines()
        .filter_map(|line| {
            let data = line.split('#').next().unwrap_or("").trim();
            let (points, value) = data.split_once(';')?;
            (value.trim() == "Full_Composition_Exclusion").then(|| {
                match points.trim().split_once("..") {
                    Some((low, high)) => hex(low)..=hex(high),
                    None => hex(points)..=hex(points),
                }
            })
        })
        .flatten()
        .collect();
    assert!(!excluded.is_empty());
    let mut decomposable = 0usize;
    for (&point, fields) in &unicode_data(&ucd("UnicodeData.txt")) {
        let mapping = fields[5].trim();
        if mapping.is_empty() || mapping.starts_with('<') {
            continue;
        }
        decomposable += 1;
        let c = scalar(point).to_string();
        assert_eq!(
            nfc(&c) == c,
            !excluded.contains(&point),
            "U+{point:04X}: NFC recomposition disagrees with Full_Composition_Exclusion"
        );
        assert_eq!(is_nfc(&c), nfc(&c) == c, "U+{point:04X}: is_nfc");
    }
    assert!(decomposable > 2000);
}
