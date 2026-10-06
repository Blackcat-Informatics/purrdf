// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The independently executed Commons Codec oracle and a full-matrix scalar
//! distance oracle test the native rules and the bounded band independently.

use purrdf_hash::{blake3, hex};
use purrdf_text::TextError;
use purrdf_text::phonetic::{
    DoubleMetaphone, MAX_CODE_LENGTH, MAX_EDIT_DISTANCE, MAX_INPUT_SCALARS, PhoneticRefusal,
    PreparedDistance, PreparedMyers, accepts, bounded_edit_distance, canonicalize,
};

pub(crate) fn every_independent_reference_vector_matches_at_every_declared_code_bound() {
    for (data, digest) in [
        (
            include_str!("../phonetic_vectors/commons-1.18.0.tsv"),
            "1b7821cc3c6d6e6135dcac6e6e31bf1c823ba048567806a052a9949720cb0ba4",
        ),
        (
            include_str!("../phonetic_vectors/context-oracle.tsv"),
            "221126f7bafbcbc455eb60ca291022fb084028b84703d5fbae80579bd48b5b0b",
        ),
    ] {
        assert_eq!(
            hex::Lower(blake3::hash(data.as_bytes()).as_bytes()).to_string(),
            digest
        );
    }
    let bounds = [1, 2, 3, 4, 6, 8, 16, MAX_CODE_LENGTH];
    let mut count = 0;
    let mut checked = 0;
    let mut mismatches = Vec::new();
    for line in include_str!("../phonetic_vectors/commons-1.18.0.tsv")
        .lines()
        .chain(include_str!("../phonetic_vectors/context-oracle.tsv").lines())
    {
        if line.starts_with('#') {
            continue;
        }
        let mut fields = line.split('\t');
        let word = fields.next().expect("input field");
        let primary = fields.next().expect("primary field");
        let alternate = fields.next().expect("alternate field");
        assert!(fields.next().is_none(), "vector has exactly three fields");
        if !word.bytes().all(|letter| letter.is_ascii_alphabetic()) {
            // This historical corpus also contains spaces, punctuation and
            // accented raw inputs. Their raw reference keys do not specify our
            // separately tested canonical spelling law.
            count += 1;
            continue;
        }
        checked += 1;
        let encodable = !primary.is_empty() || !alternate.is_empty();
        assert_eq!(accepts(word), encodable, "domain selection: {word:?}");
        for bound in bounds {
            let encoder = DoubleMetaphone::new(bound).expect("valid bound");
            let codes = encoder.encode(word);
            if !encodable {
                assert!(matches!(codes, Err(PhoneticRefusal::EmptyCode)), "{word:?}");
                continue;
            }
            let codes = codes.expect("reference word emits a code");
            let expected_primary = &primary[..primary.len().min(bound)];
            let expected_alternate = &alternate[..alternate.len().min(bound)];
            if (codes.primary.as_str(), codes.alternate.as_str())
                != (expected_primary, expected_alternate)
            {
                mismatches.push(format!("{word:?}, bound {bound}: {:?}/{:?} != {expected_primary:?}/{expected_alternate:?}", codes.primary, codes.alternate));
            }
            assert_eq!(
                encoder
                    .encode(&word.to_ascii_lowercase())
                    .expect("same ASCII domain"),
                codes,
                "case: {word:?}"
            );
        }
        count += 1;
    }
    assert_eq!(
        count, 29_280,
        "every historical oracle row is accounted for"
    );
    assert_eq!(checked, 29_119, "all canonical ASCII inputs are replayed");
    assert!(
        mismatches.is_empty(),
        "{} mismatches: {:#?}",
        mismatches.len(),
        &mismatches[..mismatches.len().min(50)]
    );
}

pub(crate) fn phonetic_domain_and_every_resource_boundary_are_explicit() {
    assert_eq!(
        canonicalize("中"),
        Err(PhoneticRefusal::UnsupportedScalar { scalar: '中' })
    );
    assert_eq!(canonicalize("'a"), Err(PhoneticRefusal::InvalidApostrophe));
    assert_eq!(canonicalize(""), Err(PhoneticRefusal::EmptySpelling));
    for invalid in [0, MAX_CODE_LENGTH + 1, usize::MAX] {
        assert!(matches!(
            DoubleMetaphone::new(invalid),
            Err(TextError::Config(_))
        ));
    }
    let encoder = DoubleMetaphone::new(MAX_CODE_LENGTH).expect("maximum bound");
    assert_eq!(encoder.encode("H"), Err(PhoneticRefusal::EmptyCode));
    assert_eq!(
        encoder.encode_canonical("a"),
        Err(PhoneticRefusal::NonCanonicalSpelling { scalar: 'a' })
    );
    assert_eq!(encoder.max_code_len(), MAX_CODE_LENGTH);
    for invalid in [
        "",
        " ",
        "-'-",
        "H",
        "W",
        "東京",
        "ไทย",
        "abc123",
        "abc\t",
        "abc\n",
        "abc\u{200b}",
        "\u{301}A",
        "😀",
        "'abc",
        "abc’",
        "a''b",
        "a-b",
        "a b",
        "ß中",
        "aα",
        "ʒ",
    ] {
        assert!(!accepts(invalid), "{invalid:?}");
        assert!(encoder.encode(invalid).is_err(), "{invalid:?}");
    }
    for (input, primary, alternate) in [
        ("Smith", "SM0", "XMT"),
        ("Schmidt", "XMT", "SMT"),
        ("ç", "K", "K"),
        ("ñ", "N", "N"),
        ("aj", "AJ", "A "),
    ] {
        assert!(accepts(input));
        let codes = encoder.encode(input).expect("declared domain");
        assert_eq!(
            (codes.primary.as_str(), codes.alternate.as_str()),
            (primary, alternate)
        );
    }
    let longest = "b".repeat(MAX_INPUT_SCALARS);
    assert!(accepts(&longest));
    assert_eq!(
        encoder
            .encode(&longest)
            .expect("maximum input")
            .primary
            .len(),
        MAX_CODE_LENGTH
    );
    assert!(!accepts(&format!("{longest}b")));
    assert!(!accepts(&format!(" {longest}")));
    assert!(
        DoubleMetaphone::new(1)
            .expect("one code scalar")
            .encode("bad東京")
            .is_err()
    );
    assert!(canonicalize(&"ß".repeat(MAX_INPUT_SCALARS / 2)).is_ok());
    assert!(canonicalize(&"ß".repeat(MAX_INPUT_SCALARS / 2 + 1)).is_err());
    assert_eq!(
        canonicalize(&"b".repeat(MAX_INPUT_SCALARS + 1)),
        Err(PhoneticRefusal::InputTooLong {
            limit: MAX_INPUT_SCALARS,
            observed: MAX_INPUT_SCALARS + 1
        })
    );
}

pub(crate) fn canonical_spelling_is_shared_by_code_and_distance() {
    let encoder = DoubleMetaphone::new(4).expect("standard bound");
    for (raw, expected) in [
        ("café", "CAFE"),
        ("cafe\u{301}", "CAFE"),
        ("Straße", "STRASSE"),
        ("ÆŒÞØŁĐÐĦıŊ", "AEOETHOLDDHIN"),
        ("D’Angelo", "DANGELO"),
        ("O'Neill", "ONEILL"),
        ("a\u{200c}b\u{ad}c\u{fe0f}", "ABC"),
        ("a\u{202e}b\u{2060}c\u{feff}", "ABC"),
        ("a\0b", "AB"),
        ("a\u{301}’e\u{301}", "AE"),
    ] {
        let canonical = canonicalize(raw).expect("Latin widening domain");
        assert_eq!(canonical, expected, "{raw:?}");
        assert_eq!(canonicalize(&canonical).expect("idempotent"), canonical);
        assert_eq!(
            encoder.encode(raw).expect("raw codes"),
            encoder
                .encode_canonical(&canonical)
                .expect("canonical codes")
        );
        assert_eq!(
            bounded_edit_distance(&canonical, expected, 0).expect("exact"),
            Some(0)
        );
    }
    let cluster = format!("a{}", "\u{301}".repeat(4096));
    assert_eq!(
        canonicalize(&cluster).expect("expanded spelling governs bound"),
        "A"
    );
    for invalid in [
        "a\u{200b}b",
        "a\u{a0}b",
        "a\u{2028}b",
        "\u{301}a",
        "a\u{903}",
        "a\u{902}",
        "a\u{5b0}",
        "a\u{64b}",
        "a\u{20dd}",
        "a\u{301}’",
        "\u{200c}",
    ] {
        assert!(canonicalize(invalid).is_err(), "{invalid:?}");
    }
}

pub(crate) fn scalar_distance_examples_cover_thresholds_and_unicode() {
    for (a, b, distance) in [
        ("", "", 0),
        ("", "abc", 3),
        ("kitten", "sitting", 3),
        ("flaw", "lawn", 2),
        ("ab", "ba", 2),
        ("café", "cafe", 1),
        ("é", "e\u{301}", 2),
        ("a😀b", "a中b", 1),
        ("Straße", "STRASSE", 6),
        ("東京大学", "東京工科大学", 2),
    ] {
        for threshold in 0..=distance + 1 {
            let expected = (distance <= threshold).then_some(distance);
            assert_eq!(
                bounded_edit_distance(a, b, threshold).expect("bounded inputs"),
                expected,
                "{a:?}, {b:?}, {threshold}"
            );
            assert_eq!(
                bounded_edit_distance(b, a, threshold).expect("bounded inputs"),
                expected,
                "symmetry: {a:?}, {b:?}, {threshold}"
            );
        }
    }
}

/// The complete matrix supplies an independent reference for band boundaries.
fn matrix_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut matrix = vec![vec![0; b.len() + 1]; a.len() + 1];
    for (row, cells) in matrix.iter_mut().enumerate() {
        cells[0] = row;
    }
    for (column, cell) in matrix[0].iter_mut().enumerate() {
        *cell = column;
    }
    for row in 1..=a.len() {
        for column in 1..=b.len() {
            matrix[row][column] = if a[row - 1] == b[column - 1] {
                matrix[row - 1][column - 1]
            } else {
                1 + matrix[row - 1][column]
                    .min(matrix[row][column - 1])
                    .min(matrix[row - 1][column - 1])
            };
        }
    }
    matrix[a.len()][b.len()]
}

pub(crate) fn banded_distance_matches_full_matrix_for_all_short_mixed_scalar_strings() {
    let mut words = vec![String::new()];
    let mut layer = words.clone();
    for _ in 0..4 {
        layer = layer
            .iter()
            .flat_map(|word| ['a', 'é', '中'].map(|letter| format!("{word}{letter}")))
            .collect();
        words.extend(layer.iter().cloned());
    }
    assert_eq!(words.len(), 121);
    for a in &words {
        for b in &words {
            let expected = matrix_distance(a, b);
            for threshold in 0..=5 {
                let mut myers = PreparedMyers::new(a, threshold).expect("prepared bit vectors");
                assert_eq!(
                    myers.distance(b).expect("small input"),
                    (expected <= threshold).then_some(expected),
                    "Myers {a:?}, {b:?}, {threshold}"
                );
                assert_eq!(
                    bounded_edit_distance(a, b, threshold).expect("small bounded inputs"),
                    (expected <= threshold).then_some(expected),
                    "{a:?}, {b:?}, {threshold}"
                );
            }
        }
    }
}

pub(crate) fn scalar_distance_refuses_invalid_bounds_and_handles_maximum_inputs() {
    for invalid in [MAX_EDIT_DISTANCE + 1, usize::MAX] {
        assert!(matches!(
            bounded_edit_distance("a", "a", invalid),
            Err(TextError::Config(_))
        ));
    }
    let a = "中".repeat(MAX_INPUT_SCALARS);
    let b = "文".repeat(MAX_INPUT_SCALARS);
    assert_eq!(
        bounded_edit_distance(&a, &a, 0).expect("max input, zero band"),
        Some(0)
    );
    assert_eq!(
        bounded_edit_distance(&a, &b, MAX_EDIT_DISTANCE).expect("max inputs and band"),
        None
    );
    assert_eq!(
        bounded_edit_distance(&a, "", 0).expect("length refusal"),
        None
    );
    for (a, b) in [(format!("{a}中"), a.clone()), (a, format!("{b}文"))] {
        assert!(matches!(
            bounded_edit_distance(&a, &b, MAX_EDIT_DISTANCE),
            Err(TextError::Data(_))
        ));
    }
}

pub(crate) fn both_engines_cross_every_block_boundary_and_reuse_query_scratch() {
    for length in [0, 1, 2, 63, 64, 65, 127, 128, 129, 255, 256, 257, 2048] {
        let a: String = ['a', 'é', '中', '😀']
            .into_iter()
            .cycle()
            .take(length)
            .collect();
        let mut candidates = vec![a.clone(), "z".repeat(length), "中".repeat(length)];
        if length > 0 {
            candidates.push(a.chars().skip(1).collect());
            let mut last = a.clone();
            last.pop();
            candidates.push(last);
            let mut middle: Vec<char> = a.chars().collect();
            middle[length / 2] = '𠮷';
            candidates.push(middle.into_iter().collect());
        }
        if length < MAX_INPUT_SCALARS {
            candidates.push(format!("x{a}"));
            candidates.push(format!("{a}x"));
        }
        let candidates: Vec<_> = candidates
            .into_iter()
            .map(|b| {
                let distance = matrix_distance(&a, &b);
                (b, distance)
            })
            .collect();
        for threshold in [0, 1, 2, 4, 63, 64] {
            let mut band = PreparedDistance::new(&a, threshold).expect("prepared band");
            let mut bits = PreparedMyers::new(&a, threshold).expect("prepared bits");
            for (b, distance) in &candidates {
                let expected = (*distance <= threshold).then_some(*distance);
                assert_eq!(
                    band.distance(b).expect("band result"),
                    expected,
                    "band length {length}, threshold {threshold}"
                );
                assert_eq!(
                    bits.distance(b).expect("bits result"),
                    expected,
                    "bits length {length}, threshold {threshold}"
                );
            }
        }
    }
}

pub(crate) fn long_graphemes_remain_scalar_sequences_for_refinement() {
    let a = format!("a{}", "\u{301}".repeat(128));
    let b = format!("a{}", "\u{301}".repeat(126));
    assert_eq!(
        bounded_edit_distance(&a, &b, 2).expect("oversized cluster"),
        Some(2)
    );
    assert_eq!(
        PreparedMyers::new(&a, 2)
            .expect("query")
            .distance(&b)
            .expect("candidate"),
        Some(2)
    );
    assert_eq!(
        bounded_edit_distance("👩‍👩‍👧‍👦", "👩‍👩‍👧", 2).expect("scalar metric"),
        Some(2)
    );
}

pub(crate) fn arbitrary_unicode_edits_match_the_independent_global_matrix() {
    let alphabet = ['a', 'é', '\u{301}', '中', '𠮷', '😀', '\u{200d}', '\0'];
    let mut random = purrdf_testkit::rng::SplitMix64::new(407);
    for _ in 0..512 {
        let length = random.below_usize(193);
        let a: String = (0..length)
            .map(|_| alphabet[random.below_usize(alphabet.len())])
            .collect();
        let mut b: Vec<char> = a.chars().collect();
        for _ in 0..random.below_usize(12) {
            let position = random.below_usize(b.len() + 1);
            match random.below_usize(3) {
                0 => b.insert(position, alphabet[random.below_usize(alphabet.len())]),
                1 if position < b.len() => {
                    b.remove(position);
                }
                2 if position < b.len() => {
                    b[position] = alphabet[random.below_usize(alphabet.len())];
                }
                _ => {}
            }
        }
        let b: String = b.into_iter().collect();
        let distance = matrix_distance(&a, &b);
        for threshold in [0, 2, distance.saturating_sub(1), distance, distance + 1, 64] {
            let expected = (distance <= threshold).then_some(distance);
            assert_eq!(
                PreparedDistance::new(&a, threshold)
                    .expect("query")
                    .distance(&b)
                    .expect("candidate"),
                expected
            );
            assert_eq!(
                PreparedMyers::new(&a, threshold)
                    .expect("query")
                    .distance(&b)
                    .expect("candidate"),
                expected
            );
        }
    }
}
