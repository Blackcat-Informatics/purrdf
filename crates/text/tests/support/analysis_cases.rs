// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use purrdf_text::{
    AccentFold, Analyzer, AnalyzerProfile, InputMode, Segmentation, Stemming, unicode,
};

pub(crate) fn official_grapheme_and_emoji_cases() {
    let data = include_str!("../../../iri/unicode/17.0.0/GraphemeBreakTest.txt");
    let mut count = 0;
    for line in data.lines() {
        let row = line.split('#').next().unwrap().trim();
        if row.is_empty() {
            continue;
        }
        let mut text = String::new();
        let mut expected = Vec::new();
        for field in row.split_whitespace() {
            match field {
                "÷" => expected.push(text.len()),
                "×" => {}
                hex => text.push(char::from_u32(u32::from_str_radix(hex, 16).unwrap()).unwrap()),
            }
        }
        let actual = unicode::grapheme_bounds(&text)
            .map(|(at, _)| at)
            .chain(std::iter::once(text.len()))
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "{line}");
        count += 1;
    }
    assert_eq!(count, 766);
    let emoji = include_str!("../../../iri/unicode/17.0.0/emoji-test.txt");
    let mut count = 0;
    let analyzer = Analyzer::empty_lexicon();
    for line in emoji.lines() {
        let row = line.split('#').next().unwrap().trim();
        if row.is_empty() {
            continue;
        }
        let (hex, _) = row.split_once(';').unwrap();
        let text = hex
            .split_whitespace()
            .map(|hex| char::from_u32(u32::from_str_radix(hex, 16).unwrap()).unwrap())
            .collect::<String>();
        assert_eq!(unicode::grapheme_bounds(&text).count(), 1, "{line}");
        assert!(unicode::emoji_status(&text).is_some(), "{line}");
        assert_eq!(
            analyzer.terms(&text).unwrap(),
            std::slice::from_ref(&text),
            "{line}"
        );
        assert_eq!(
            analyzer.surface_terms(&text).unwrap(),
            std::slice::from_ref(&text)
        );
        assert_eq!(analyzer.substring_terms(&text).unwrap(), [text]);
        count += 1;
    }
    assert_eq!(count, 5225);
}
pub(crate) fn ordered_analysis_and_alignment_cases() {
    let profile = AnalyzerProfile::empty_lexicon()
        .with_accent_fold(AccentFold::Preserve)
        .with_input_mode(InputMode::HtmlText);
    let analyzer = Analyzer::with_profile(profile).unwrap();
    let analysis = analyzer.projections("e&#x301;").unwrap();
    assert_eq!(analysis.normalized.text, "é");
    assert_eq!(
        analysis.normalized.contributors(0..2),
        std::slice::from_ref(&(0..8))
    );
    assert_eq!(analysis.lexical[0].highlight, 0..8);
    let once = analyzer.analysis_form("&amp;lt;").unwrap();
    assert_eq!(once, "&lt;");
    assert!(analyzer.analysis_form("&copy").is_err());
    let joined = analyzer.projections("&#x1F469;&#x200D;&#x1F4BB;").unwrap();
    assert_eq!(joined.lexical[0].text, "👩‍💻");
    assert_eq!(joined.lexical[0].sources, std::slice::from_ref(&(0..26)));
    let plain = Analyzer::empty_lexicon();
    assert_eq!(
        plain.terms("alpha.beta gamma:delta 3.14 don't").unwrap(),
        ["alpha", "beta", "gamma", "delta", "3.14", "don't"]
    );
    assert_eq!(
        plain.surface_terms("alpha.beta gamma:delta").unwrap(),
        ["alpha.beta", "gamma:delta"]
    );
    assert_eq!(
        plain.terms("a\u{200c}b c\u{ad}d e\u{fe0f}f").unwrap(),
        ["ab", "cd", "ef"]
    );
    assert_eq!(
        plain
            .surface_terms("a\u{200c}b c\u{ad}d e\u{fe0f}f")
            .unwrap(),
        ["a\u{200c}b", "c\u{ad}d", "e\u{fe0f}f"]
    );
    assert_eq!(plain.terms("a\u{200b}b").unwrap(), ["a", "b"]);
    assert_eq!(
        plain.analysis_form("अं café Ελληνικά Москва").unwrap(),
        "अं cafe ελληνικα москва"
    );
    let preserving = Analyzer::with_profile(
        AnalyzerProfile::empty_lexicon().with_accent_fold(AccentFold::Preserve),
    )
    .unwrap();
    let composed = preserving.projections("e\u{61c}\u{301}").unwrap();
    assert_eq!(composed.normalized.text, "é");
    assert_eq!(composed.normalized.contributors(0..2), [0..1, 3..5]);
    assert_eq!(composed.lexical[0].highlight, 0..5);
    let stemmed =
        Analyzer::with_profile(AnalyzerProfile::empty_lexicon().with_stemming(Stemming::English))
            .unwrap();
    let stem = stemmed.projections("running").unwrap();
    assert_eq!(stem.lexical[0].text, "run");
    assert!(stem.lexical[0].coarse);
    assert_eq!(stem.lexical[0].sources, std::slice::from_ref(&(0..7)));
    let minimum = unicode::MAX_EMOJI_SCALARS;
    let bounded = Analyzer::with_profile(
        AnalyzerProfile::new(minimum)
            .unwrap()
            .with_segmentation(Segmentation::EmptyLexicon)
            .with_accent_fold(AccentFold::Preserve),
    )
    .unwrap();
    let oversized = format!("a{}z", "\u{301}".repeat(minimum + 20));
    let result = bounded.substring_terms(&oversized).unwrap();
    assert_eq!(result[0], format!("á{}", "\u{301}".repeat(minimum + 19)));
    let second = format!("x a{}z", "\u{301}".repeat(minimum + 20));
    assert_eq!(bounded.substring_terms(&second).unwrap()[1], result[0]);
    let later = format!("xa{}z", "\u{301}".repeat(minimum + 20));
    assert_eq!(bounded.substring_terms(&later).unwrap(), ["x"]);
}
