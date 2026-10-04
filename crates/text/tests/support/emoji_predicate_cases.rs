// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::collections::BTreeSet;

use purrdf_text::unicode;

/// Build the predicate's expected protected scalar set directly from the
/// official input files, independently of the generated runtime properties.
fn protected_scalars() -> Vec<bool> {
    let mut protected = vec![false; 0x11_0000];
    for (data, wanted) in [
        (
            include_str!("../../../iri/unicode/17.0.0/emoji-data.txt"),
            "Extended_Pictographic",
        ),
        (
            include_str!("../../../iri/unicode/17.0.0/GraphemeBreakProperty.txt"),
            "Regional_Indicator",
        ),
    ] {
        for line in data.lines() {
            let row = line.split('#').next().unwrap().trim();
            let Some((points, property)) = row.split_once(';') else {
                continue;
            };
            if property.trim() != wanted {
                continue;
            }
            let points = points.trim();
            let (low, high) = points.split_once("..").unwrap_or((points, points));
            let low = usize::from_str_radix(low, 16).unwrap();
            let high = usize::from_str_radix(high, 16).unwrap();
            protected[low..=high].fill(true);
        }
    }
    protected
}

pub(crate) fn every_emoji_spelling_and_protected_scalar_matches_the_input_data() {
    let protected = protected_scalars();
    let mut atoms = BTreeSet::new();
    for (data, expected_rows) in [
        (
            include_str!("../../../iri/unicode/17.0.0/emoji-test.txt"),
            5225,
        ),
        (
            include_str!("../../../iri/unicode/17.0.0/emoji-variation-sequences.txt"),
            742,
        ),
    ] {
        let mut rows = 0;
        for line in data.lines() {
            let row = line.split('#').next().unwrap().trim();
            let Some((points, _)) = row.split_once(';') else {
                continue;
            };
            let atom: String = points
                .split_whitespace()
                .map(|point| char::from_u32(u32::from_str_radix(point, 16).unwrap()).unwrap())
                .collect();
            assert!(unicode::emoji_status(&atom).is_some(), "{line}");
            assert!(unicode::is_emoji_grapheme(&atom), "{line}");
            assert_eq!(unicode::grapheme_bounds(&atom).count(), 1, "{line}");
            // Every finite atom without an independently protected scalar
            // must have a keycap/variation prefix or be a singleton component.
            if !atom.chars().any(|c| protected[c as usize]) {
                let mut scalars = atom.chars();
                assert!(matches!(
                    (scalars.next(), scalars.next()),
                    (
                        Some('#' | '*' | '0'..='9'),
                        Some('\u{20e3}' | '\u{fe0e}' | '\u{fe0f}')
                    ) | (Some('\u{1f3fb}'..='\u{1f3ff}'), None)
                ));
            }
            atoms.insert(atom);
            rows += 1;
        }
        assert_eq!(rows, expected_rows);
    }
    assert_eq!(atoms.len(), 5760);

    let expected = |text: &str| {
        !text.is_ascii() && (atoms.contains(text) || text.chars().any(|c| protected[c as usize]))
    };
    let mut utf8 = [0; 4];
    for point in 0..=0x10_ffff {
        if let Some(c) = char::from_u32(point) {
            let text = c.encode_utf8(&mut utf8);
            assert_eq!(unicode::is_emoji_grapheme(text), expected(text), "{c:?}");
        }
    }
    // A preceding scalar or trailing extension can remove finite recognition.
    // Existing EP/RI protection must still be exactly the input-data law,
    // including Prepend, unknown joins, selectors and standalone components.
    for atom in &atoms {
        for text in [
            format!("\u{600}{atom}"),
            format!("{atom}\u{301}"),
            format!("{atom}\u{200d}🧠"),
        ] {
            assert_eq!(
                unicode::is_emoji_grapheme(&text),
                expected(&text),
                "{text:?}"
            );
        }
    }
    for left in [
        "",
        "a",
        "中",
        "ที่",
        "7",
        "#",
        "*",
        "🏿",
        "🧠",
        "🇨",
        "\u{600}",
    ] {
        for right in [
            "", "\u{301}", "\u{fe0e}", "\u{fe0f}", "\u{20e3}", "\u{200d}", "🧠", "🇦",
        ] {
            let text = format!("{left}{right}");
            assert_eq!(
                unicode::is_emoji_grapheme(&text),
                expected(&text),
                "{text:?}"
            );
        }
    }
}
