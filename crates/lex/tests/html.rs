// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Reference-submachine conformance: the complete pinned named data, every
//! numeric code point, prose-derived context/error cases and exact alignment.

use purrdf_lex::html::{self, ErrorKind, Mode, resolve, resolve_strict};
use purrdf_lex::json::{self, Value};
use std::borrow::Cow;

const DATA: &str = include_str!("../data/html/entities.json");

fn names() -> Vec<(String, String)> {
    let Value::Object(ref object) = json::read(DATA).expect("pinned data") else {
        panic!("named table object");
    };
    object
        .iter()
        .map(|(name, value)| {
            let Value::Object(value) = value else {
                panic!("named value object");
            };
            (
                name.clone(),
                value
                    .get("characters")
                    .and_then(Value::as_str)
                    .expect("characters")
                    .to_owned(),
            )
        })
        .collect()
}

fn kinds(input: &str, mode: Mode) -> Vec<ErrorKind> {
    resolve(input, mode)
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.kind)
        .collect()
}

#[test]
fn every_pinned_name_and_legacy_context_is_reproduced() {
    let names = names();
    assert_eq!(names.len(), html::NAMED_REFERENCE_COUNT);
    assert_eq!(names.len(), 2_231);
    let mut legacy = 0;
    for (name, characters) in names {
        for mode in [Mode::Text, Mode::Attribute] {
            let decoded = resolve(&name, mode);
            assert_eq!(decoded.decoded.text, characters, "{name} {mode:?}");
            assert_eq!(
                decoded.decoded.sources,
                vec![0..name.len(); characters.chars().count()]
            );
            if name.ends_with(';') {
                assert!(decoded.diagnostics.is_empty(), "{name}");
            } else {
                assert_eq!(
                    kinds(&name, mode),
                    [ErrorKind::MissingSemicolonAfterCharacterReference],
                    "{name}"
                );
            }
        }
        if !name.ends_with(';') {
            legacy += 1;
            for tail in ['=', '0', 'A', 'z'] {
                let input = format!("{name}{tail}");
                let attribute = resolve(&input, Mode::Attribute);
                assert_eq!(attribute.decoded.text, input);
                assert_eq!(attribute.decoded.sources, [] as [std::ops::Range<usize>; 0]);
                assert_eq!(attribute.diagnostics, [] as [html::Diagnostic; 0]);
                let text = resolve(&input, Mode::Text);
                // A tail without a semicolon cannot complete any different
                // legacy name in this pinned table; the corpus proves it.
                assert_eq!(text.decoded.text, format!("{characters}{tail}"));
                assert_eq!(text.diagnostics.len(), 1);
            }
        }
    }
    assert_eq!(legacy, 106);
}

#[test]
fn longest_matching_is_independent_of_failed_longer_prefixes() {
    let names = names();
    for (name, _) in &names {
        // A linear longest-key oracle, independent of production's bounded
        // binary prefix narrowing, checks every truncated spelling.
        for end in 2..=name.len() {
            let input = format!("{}!", &name[..end]);
            let expected = names
                .iter()
                .filter(|(candidate, _)| input.starts_with(candidate))
                .max_by_key(|(candidate, _)| candidate.len())
                .map_or_else(
                    || input.clone(),
                    |(candidate, value)| format!("{value}{}", &input[candidate.len()..]),
                );
            assert_eq!(
                resolve(&input, Mode::Text).decoded.text,
                expected,
                "{input}"
            );
        }
    }
}

#[test]
fn text_and_attribute_ambiguities_and_case_are_exact() {
    for (input, text, attribute) in [
        ("&notit;", "¬it;", "&notit;"),
        ("&amp=1", "&=1", "&amp=1"),
        ("&notin;", "∉", "∉"),
        ("&notin", "¬in", "&notin"),
        ("&AMP;&amp;&Amp;", "&&&Amp;", "&&&Amp;"),
        ("&copy!", "©!", "©!"),
        ("&copyé", "©é", "©é"),
        ("&copy_", "©_", "©_"),
    ] {
        assert_eq!(resolve(input, Mode::Text).decoded.text, text, "{input}");
        assert_eq!(resolve(input, Mode::Attribute).decoded.text, attribute);
    }
    for input in ["&notit;", "&amp=1", "&notin"] {
        assert!(resolve_strict(input, Mode::Attribute).is_ok());
        assert!(resolve_strict(input, Mode::Text).is_err());
    }
}

#[test]
fn reference_errors_have_exact_codes_and_original_spans() {
    for (input, kind, range) in [
        (
            "&NoSuchName;",
            ErrorKind::UnknownNamedCharacterReference,
            0..12,
        ),
        (
            "&#;",
            ErrorKind::AbsenceOfDigitsInNumericCharacterReference,
            0..2,
        ),
        (
            "&#x;",
            ErrorKind::AbsenceOfDigitsInNumericCharacterReference,
            0..3,
        ),
        (
            "&copy",
            ErrorKind::MissingSemicolonAfterCharacterReference,
            0..5,
        ),
        ("&#0;", ErrorKind::NullCharacterReference, 0..4),
        (
            "&#1114112;",
            ErrorKind::CharacterReferenceOutsideUnicodeRange,
            0..10,
        ),
        ("&#xD800;", ErrorKind::SurrogateCharacterReference, 0..8),
        ("&#xFFFF;", ErrorKind::NoncharacterCharacterReference, 0..8),
        ("&#13;", ErrorKind::ControlCharacterReference, 0..5),
    ] {
        let diagnostics = resolve_strict(input, Mode::Text).expect_err(input);
        assert_eq!(diagnostics.len(), 1, "{input}");
        assert_eq!(diagnostics[0].kind, kind, "{input}");
        assert_eq!(diagnostics[0].source, range, "{input}");
        assert!(diagnostics[0].to_string().starts_with(kind.code()));
    }
    let result = resolve("é&#0 &#x110000", Mode::Text);
    assert_eq!(result.decoded.text, "é� �");
    assert_eq!(result.diagnostics.len(), 4);
    assert_eq!(result.diagnostics[0].source, 2..5);
    assert_eq!(result.diagnostics[1].source, 2..5);
    assert_eq!(result.diagnostics[2].source, 6..15);
    assert_eq!(result.diagnostics[3].source, 6..15);
}

#[test]
fn numeric_recovery_and_missing_digit_reconsumption() {
    for (input, expected) in [
        ("&#", "&#"),
        ("&#x", "&#x"),
        ("&#X", "&#X"),
        ("&#xg;", "&#xg;"),
        ("&#-1;", "&#-1;"),
        ("&#x&copy;", "&#x©"),
        ("&#65A", "AA"),
        ("&#x41g;", "Ag;"),
        ("&#X1F469;", "👩"),
        ("&#0;", "�"),
        ("&#xDFFF;", "�"),
        ("&#x110000;", "�"),
        ("&#xFDD0;", "\u{fdd0}"),
        ("&#x10FFFF;", "\u{10ffff}"),
        ("&#128;&#129;&#159;", "€\u{81}Ÿ"),
        ("&#9;&#10;&#12;&#32;", "\t\n\u{c} "),
        ("&#13;", "\r"),
    ] {
        assert_eq!(resolve(input, Mode::Text).decoded.text, expected, "{input}");
    }
    assert!(resolve_strict("&#9;&#10;&#12;&#32;", Mode::Text).is_ok());
    assert!(resolve_strict("&#11;", Mode::Text).is_err());
}

#[test]
fn every_control_and_noncharacter_reports_the_exact_error_class() {
    for point in (0_u32..=0x20).chain(0x7F..=0x9F) {
        let input = format!("&#{point};");
        let expected = match point {
            0 => vec![ErrorKind::NullCharacterReference],
            9 | 10 | 12 | 32 => vec![],
            _ => vec![ErrorKind::ControlCharacterReference],
        };
        assert_eq!(kinds(&input, Mode::Text), expected, "{input}");
    }
    let noncharacters = (0xFDD0..=0xFDEF)
        .chain((0..=16).flat_map(|plane| [plane * 65_536 + 65_534, plane * 65_536 + 65_535]));
    let mut count = 0;
    for point in noncharacters {
        count += 1;
        let input = format!("&#{point};");
        assert_eq!(
            kinds(&input, Mode::Text),
            [ErrorKind::NoncharacterCharacterReference],
            "{input}"
        );
    }
    assert_eq!(count, 66);
    for input in ["&#xD7FF;", "&#xE000;", "&#xFDCF;", "&#xFDF0;", "&#x10FFFD;"] {
        assert!(resolve_strict(input, Mode::Text).is_ok(), "{input}");
    }
}

#[test]
fn long_unknown_names_are_preserved_and_diagnosed_once() {
    let input = format!("&{};&amp;", "Z".repeat(100_000));
    let result = resolve(&input, Mode::Text);
    assert_eq!(result.decoded.text, format!("&{};&", "Z".repeat(100_000)));
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(
        result.diagnostics[0].kind,
        ErrorKind::UnknownNamedCharacterReference
    );
    assert_eq!(result.diagnostics[0].source, 0..100_002);
}

#[test]
fn every_numeric_value_has_the_specified_scalar_recovery() {
    let numeric = json::read(include_str!("../data/html/numeric-recovery.json")).unwrap();
    let Value::Array(c1) = &numeric else {
        panic!("C1 table");
    };
    // Exhaustively covers every plane's noncharacters, both surrogate edges,
    // control ranges and decimal digit lengths. Hex is checked independently
    // at every boundary and on every C1 entry below.
    for point in 0..=0x11_0000 {
        let expected = match point {
            0 | 0xD800..=0xDFFF | 0x11_0000 => '\u{fffd}',
            0x80..=0x9F => {
                char::from_u32(u32::try_from(c1[point as usize - 0x80].as_u64().unwrap()).unwrap())
                    .unwrap()
            }
            _ => char::from_u32(point).unwrap(),
        };
        let input = format!("&#{point};");
        let result = resolve(&input, Mode::Text);
        assert_eq!(
            result.decoded.text.chars().next(),
            Some(expected),
            "{input}"
        );
        assert_eq!(result.decoded.text.chars().count(), 1);
        assert_eq!(result.decoded.sources.len(), 1);
        assert_eq!(result.decoded.sources[0], 0..input.len());
    }
    for point in (0..=0xA0).chain([
        0xD7FF, 0xD800, 0xDFFF, 0xE000, 0xFDCF, 0xFDD0, 0xFDEF, 0xFDF0, 0xFFFD, 0xFFFE, 0xFFFF,
        0x10_FFFD, 0x10_FFFE, 0x10_FFFF, 0x11_0000,
    ]) {
        let decimal = format!("&#{point};");
        let hex = format!("&#x{point:X};");
        assert_eq!(
            resolve(&decimal, Mode::Text).decoded.text,
            resolve(&hex, Mode::Text).decoded.text
        );
        assert_eq!(kinds(&decimal, Mode::Text), kinds(&hex, Mode::Text));
    }
}

#[test]
fn arbitrarily_long_digits_saturate_without_truncating_the_reference() {
    for (prefix, digit) in [("&#", '9'), ("&#x", 'f')] {
        let input = format!("{prefix}{};tail", digit.to_string().repeat(100_000));
        let result = resolve(&input, Mode::Text);
        assert_eq!(result.decoded.text, "�tail");
        assert_eq!(result.decoded.sources[0], 0..input.len() - 4);
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(
            result.diagnostics[0].kind,
            ErrorKind::CharacterReferenceOutsideUnicodeRange
        );
    }
    let input = format!("&#{}65;", "0".repeat(100_000));
    assert_eq!(resolve_strict(&input, Mode::Text).unwrap().text, "A");
}

#[test]
fn decoding_is_once_and_alignment_is_per_scalar() {
    let result = resolve_strict("é&amp;lt;&NotEqualTilde;Z", Mode::Text).unwrap();
    assert_eq!(result.text, "é&lt;≂\u{338}Z");
    assert_eq!(
        result.sources,
        [0..2, 2..7, 7..8, 8..9, 9..10, 10..25, 10..25, 25..26]
    );
    let acute = resolve_strict("e&#x301;", Mode::Text).unwrap();
    assert_eq!(acute.text, "e\u{301}");
    assert_eq!(acute.sources, [0..1, 1..8]);
    let emoji = resolve_strict("&#x1F469;&#x200D;&#x1F4BB;", Mode::Text).unwrap();
    assert_eq!(emoji.text, "👩\u{200d}💻");
    assert_eq!(emoji.sources, [0..9, 9..17, 17..26]);
}

#[test]
fn only_reference_errors_are_reported_and_other_content_is_verbatim() {
    for input in [
        "",
        "plain ASCII",
        "é𠀀👩‍💻",
        "<broken x='\r\n\0",
        "&",
        "&&",
        "&;",
        "&unknown",
        "&é;",
        "&=",
        "&\r\n",
        "&#amp;",
    ] {
        let plain = resolve(input, Mode::Plain);
        assert!(matches!(plain.decoded.text, Cow::Borrowed(_)));
        assert_eq!(plain.decoded.text, input);
        assert_eq!(plain.decoded.sources, [] as [std::ops::Range<usize>; 0]);
        assert_eq!(plain.diagnostics, [] as [html::Diagnostic; 0]);
        let result = resolve(input, Mode::Text);
        assert!(matches!(result.decoded.text, Cow::Borrowed(_)));
        assert_eq!(result.decoded.text, input);
        assert_eq!(result.decoded.sources, [] as [std::ops::Range<usize>; 0]);
        if input != "&#amp;" {
            assert!(result.diagnostics.is_empty(), "{input:?}");
        }
    }
    let input = "<x y='&amp;'>\r\n\0</x>";
    assert_eq!(
        resolve_strict(input, Mode::Text).unwrap().text,
        "<x y='&'>\r\n\0</x>"
    );
}

#[test]
fn data_identities_match_the_exact_frozen_bytes() {
    assert_eq!(
        html::NAMED_TABLE_BLAKE3,
        *purrdf_hash::blake3::hash(DATA.as_bytes()).as_bytes()
    );
    assert_eq!(
        html::NUMERIC_TABLE_BLAKE3,
        *purrdf_hash::blake3::hash(include_bytes!("../data/html/numeric-recovery.json")).as_bytes()
    );
}
