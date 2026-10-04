// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Independent integration checks across fast paths, controls and source evidence.

use purrdf_text::{AccentFold, Analyzer, AnalyzerProfile, AnalyzerScratch, InputMode, Stemming};

fn configured(accent: AccentFold, stemming: Stemming, mode: InputMode) -> Analyzer {
    Analyzer::with_profile(
        AnalyzerProfile::empty_lexicon()
            .with_accent_fold(accent)
            .with_stemming(stemming)
            .with_input_mode(mode),
    )
    .unwrap()
}

pub(crate) fn every_lexical_entry_point_agrees_across_reused_scratch() {
    let pieces = [
        "ABC",
        "中文",
        "𠮷",
        "café",
        "a\u{301}",
        "a\u{0902}",
        "क्‍क",
        "ب‍ب",
        "👩‍💻",
        "1️⃣",
        "a\u{ad}b",
        "中\u{e0100}文",
        "a\u{200b}b",
        "a\0b",
        "ground.logic.ttl",
        "rdf:type",
        "3.14",
        "don't",
        "running",
        "\u{301}",
        "Ａß",
        "㍿",
        "!",
        "",
    ];
    for accent in [
        AccentFold::Preserve,
        AccentFold::LatinGreekCyrillic,
        AccentFold::LatinGreekCyrillicArabicHebrew,
    ] {
        for stemming in [Stemming::None, Stemming::English] {
            let analyzer = configured(accent, stemming, InputMode::Plain);
            let mut scratch = AnalyzerScratch::default();
            let mut text_scratch = String::new();
            for left in pieces {
                for right in pieces {
                    let input = format!("{left}{right}");
                    let projected = analyzer.projections(&input).unwrap();
                    let expected: Vec<_> =
                        projected.lexical.iter().map(|p| p.text.clone()).collect();
                    assert_eq!(analyzer.terms(&input).unwrap(), expected, "{input:?}");
                    let mut streamed = Vec::new();
                    analyzer
                        .analyze_each_with_scratch(&input, &mut scratch, |token| {
                            assert_eq!(token.position as usize, streamed.len());
                            streamed.push(token.text.into_owned());
                        })
                        .unwrap();
                    assert_eq!(streamed, expected, "reusable storage: {input:?}");
                    streamed.clear();
                    analyzer
                        .analyze_each(&input, &mut text_scratch, |token| {
                            streamed.push(token.text.into_owned());
                        })
                        .unwrap();
                    assert_eq!(streamed, expected, "text scratch: {input:?}");
                    let mut collected = Vec::new();
                    analyzer.analyze(&input, &mut collected).unwrap();
                    assert_eq!(
                        collected
                            .iter()
                            .map(|token| token.text.as_ref())
                            .collect::<Vec<_>>(),
                        expected,
                        "owned token API: {input:?}"
                    );
                }
            }
        }
    }
}

pub(crate) fn fast_and_expanding_normalization_keep_original_utf8_contributors() {
    let analyzer = Analyzer::empty_lexicon();
    let ascii = analyzer.projections("ABC").unwrap();
    assert_eq!(ascii.normalized.text, "abc");
    assert_eq!(
        ascii.normalized.contributors(1..2),
        std::slice::from_ref(&(1..2))
    );
    let chinese = analyzer.projections("知识😀图谱").unwrap();
    assert_eq!(chinese.normalized.text, "知识😀图谱");
    assert_eq!(
        chinese.normalized.contributors(3..6),
        std::slice::from_ref(&(3..6))
    );
    let widened = analyzer.projections("Ａß\u{061c}中").unwrap();
    assert_eq!(widened.normalized.text, "ass中");
    assert_eq!(
        widened.normalized.contributors(0..1),
        std::slice::from_ref(&(0..3))
    );
    assert_eq!(
        widened.normalized.contributors(1..3),
        std::slice::from_ref(&(3..5))
    );
    assert_eq!(
        widened.normalized.contributors(3..6),
        std::slice::from_ref(&(7..10))
    );
    let compatibility = analyzer.projections("㍿").unwrap();
    assert_eq!(compatibility.normalized.text, "株式会社");
    for lexical in compatibility.lexical {
        assert_eq!(lexical.sources, std::slice::from_ref(&(0..3)));
        assert_eq!(lexical.highlight, 0..3);
        assert!(!lexical.coarse);
    }
    let preserving = configured(AccentFold::Preserve, Stemming::None, InputMode::Plain);
    let reordered = preserving.projections("a\u{315}\u{300}").unwrap();
    assert_eq!(reordered.normalized.text, "à\u{315}");
    assert_eq!(reordered.normalized.contributors(0..2), vec![0..1, 3..5]);
    assert_eq!(
        reordered.normalized.contributors(2..4),
        std::slice::from_ref(&(1..3))
    );
}

pub(crate) fn refusals_never_call_a_sink_and_scratch_remains_reusable() {
    let analyzer = configured(AccentFold::Preserve, Stemming::None, InputMode::HtmlText);
    let mut scratch = AnalyzerScratch::default();
    let mut calls = 0;
    assert!(
        analyzer
            .analyze_each_with_scratch("valid &bogus;", &mut scratch, |_| {
                calls += 1;
            })
            .is_err()
    );
    assert_eq!(calls, 0);
    let mut terms = Vec::new();
    analyzer
        .analyze_each_with_scratch("e&#x301;", &mut scratch, |token| {
            terms.push(token.text.into_owned());
        })
        .unwrap();
    assert_eq!(terms, ["é"]);
    let source = "&NotEqualTilde;";
    let entity = analyzer.projections(source).unwrap();
    assert_eq!(entity.normalized.text, "≂\u{338}");
    assert_eq!(
        entity.normalized.contributors(0..3),
        std::slice::from_ref(&(0..source.len()))
    );
    assert_eq!(
        entity.normalized.contributors(3..5),
        std::slice::from_ref(&(0..source.len()))
    );
}

pub(crate) fn removed_controls_do_not_destroy_meaningful_orthographic_joiners() {
    let analyzer = Analyzer::empty_lexicon();
    for (input, expected) in [
        ("ب\u{200d}\u{061c}ب", "ب\u{200d}ب"),
        ("ب\u{061c}\u{200d}ب", "ب\u{200d}ب"),
        ("क्\u{200d}\u{061c}क", "क्\u{200d}क"),
    ] {
        assert_eq!(
            analyzer.analysis_form(input).unwrap(),
            expected,
            "{input:?}"
        );
        assert_eq!(analyzer.surface_terms(input).unwrap(), [expected]);
    }
}

pub(crate) fn html_streaming_agrees_after_decode_cleanup_and_joining() {
    for accent in [
        AccentFold::Preserve,
        AccentFold::LatinGreekCyrillic,
        AccentFold::LatinGreekCyrillicArabicHebrew,
    ] {
        for stemming in [Stemming::None, Stemming::English] {
            for mode in [
                InputMode::Plain,
                InputMode::HtmlText,
                InputMode::HtmlAttribute,
            ] {
                let analyzer = configured(accent, stemming, mode);
                let mut scratch = AnalyzerScratch::default();
                for input in [
                    "&amp;lt;",
                    "e&#x301;",
                    "&#x1F469;&#x200D;&#x1F4BB;",
                    "ب&#x200D;&#x061C;ب",
                    "&NotEqualTilde;",
                    "A&#0;",
                    "x&copy",
                    "x&bogus;",
                    "x&#x110000;",
                    "Stra&szlig;e",
                    "&#x0301;a &#x200b; b",
                ] {
                    let projected = analyzer.projections(input);
                    let expected = projected.as_ref().map(|value| {
                        value
                            .lexical
                            .iter()
                            .map(|word| word.text.as_str())
                            .collect::<Vec<_>>()
                    });
                    let mut actual = Vec::new();
                    let streamed =
                        analyzer.analyze_each_with_scratch(input, &mut scratch, |token| {
                            actual.push(token.text.into_owned());
                        });
                    match expected {
                        Ok(expected) => {
                            streamed.unwrap();
                            assert_eq!(actual, expected, "{mode:?}: {input}");
                            assert_eq!(
                                analyzer.analysis_form(input).unwrap(),
                                projected.unwrap().normalized.text
                            );
                        }
                        Err(expected) => {
                            assert_eq!(streamed.unwrap_err().to_string(), expected.to_string());
                            assert_eq!(actual.len(), 0);
                        }
                    }
                }
            }
        }
    }
}

pub(crate) fn long_control_runs_preserve_joining_context_without_rescanning() {
    let analyzer = Analyzer::empty_lexicon();
    let controls = "\u{200d}\u{061c}\u{200c}\u{ad}".repeat(8192);
    let retained = "\u{200d}\u{200c}\u{ad}".repeat(8192);
    let mut scratch = AnalyzerScratch::default();
    for (left, right, lexical) in [("ب", "ب", "بب"), ("क्", "क", "क्क")] {
        let input = format!("{left}{controls}{right}");
        let expected = format!("{left}{retained}{right}");
        assert_eq!(analyzer.analysis_form(&input).unwrap(), expected);
        let analysis = analyzer.projections(&input).unwrap();
        assert_eq!(analysis.normalized.text, expected);
        assert_eq!(
            analysis
                .lexical
                .iter()
                .map(|word| word.text.as_str())
                .collect::<Vec<_>>(),
            [lexical]
        );
        let mut streamed = Vec::new();
        analyzer
            .analyze_each_with_scratch(&input, &mut scratch, |token| {
                streamed.push(token.text.into_owned());
            })
            .unwrap();
        assert_eq!(streamed, [lexical]);
    }
}
