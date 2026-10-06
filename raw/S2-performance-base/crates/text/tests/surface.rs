// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Positional substring admission, exact source evidence and phonetic reuse.

use purrdf_text::{
    AccentFold, Analyzer, AnalyzerProfile, InputMode, MatchProjection, SubstringLimits,
    SubstringRefusal, SubstringRefusalReason, SubstringWork, SurfaceIndex, TextError,
};

fn index(texts: &[&str]) -> SurfaceIndex {
    SurfaceIndex::from_texts(
        Analyzer::empty_lexicon(),
        texts
            .iter()
            .enumerate()
            .map(|(id, text)| (id as u32, *text)),
    )
    .unwrap()
}
fn limited(texts: &[&str], limits: SubstringLimits) -> SurfaceIndex {
    let analyzer =
        Analyzer::with_profile(AnalyzerProfile::empty_lexicon().with_substring_limits(limits))
            .unwrap();
    SurfaceIndex::from_texts(
        analyzer,
        texts
            .iter()
            .enumerate()
            .map(|(id, text)| (id as u32, *text)),
    )
    .unwrap()
}
fn refusal(error: TextError) -> SubstringRefusal {
    let TextError::Substring(refusal) = error else {
        panic!("expected typed substring refusal: {error}")
    };
    refusal
}

fn positional_alignment_narrows_universal_gram_membership() {
    let index = index(&["abcXbcd", "abcbcd", "abcd"]);
    let report = index.substring_report("abcd").unwrap();
    assert_eq!(report.matches.len(), 1);
    assert_eq!(report.matches[0].term.text(), "abcd");
    assert_eq!(report.matches[0].term.documents(), [2]);
    assert_eq!(report.counters.posting_operations, 6);
    assert_eq!(report.counters.candidate_spans, 1);
    assert_eq!(report.counters.verification_bytes, 4);
}

fn repeated_grams_require_distinct_relative_positions_and_keep_overlap() {
    let index = index(&["aaaaa", "aaaxaaa"]);
    let report = index.substring_report("aaaa").unwrap();
    assert_eq!(report.matches.len(), 1);
    assert_eq!(report.matches[0].ranges, [0..4, 1..5]);
    assert_eq!(report.counters.posting_operations, 10);
    assert_eq!(report.counters.candidate_spans, 1);
}

fn singletons_and_universal_candidates_require_explicit_exhaustion() {
    for texts in [&["adamantine"][..], &["adamantine", "adamantines"][..]] {
        let index = index(texts);
        let denied = refusal(index.substring("damant").unwrap_err());
        assert_eq!(denied.reason, SubstringRefusalReason::NonSelective);
        assert_eq!(denied.counters.candidate_spans, texts.len() as u64);
        assert_eq!(denied.counters.verification_bytes, 0);
        assert_eq!(denied.distinct_spans, texts.len() as u64);
        assert_eq!(denied.generation, index.generation());
        let explicit = index.substring_exhaustive_report("damant").unwrap();
        assert_eq!(explicit.matches.len(), texts.len());
        assert_eq!(explicit.counters.posting_operations, 0);
    }
}

fn missing_gram_and_empty_index_are_complete_empty_without_posting_work() {
    for texts in [&[][..], &["abcd", "abcxbcd"][..]] {
        let index = limited(
            texts,
            SubstringLimits {
                posting_operations: 0,
                candidate_spans: 0,
                verification_bytes: 0,
            },
        );
        let report = index.substring_report("abcdzz").unwrap();
        assert_eq!(report.matches.len(), 0);
        assert_eq!(report.counters, SubstringWork::default());
    }
}

fn each_budget_refuses_atomically_at_its_first_excess_charge() {
    let defaults = SubstringLimits::STANDARD;
    for (limits, reason, operations, candidates, bytes) in [
        (
            SubstringLimits {
                posting_operations: 3,
                ..defaults
            },
            SubstringRefusalReason::PostingOperations,
            4,
            1,
            0,
        ),
        (
            SubstringLimits {
                candidate_spans: 0,
                ..defaults
            },
            SubstringRefusalReason::CandidateSpans,
            2,
            1,
            0,
        ),
        (
            SubstringLimits {
                verification_bytes: 3,
                ..defaults
            },
            SubstringRefusalReason::VerificationBytes,
            4,
            1,
            4,
        ),
    ] {
        let index = limited(&["abcd", "abcxbcd"], limits);
        let denied = refusal(index.substring("abcd").unwrap_err());
        assert_eq!(denied.reason, reason);
        assert_eq!(denied.limits, limits);
        assert_eq!(denied.counters.posting_operations, operations);
        assert_eq!(denied.counters.candidate_spans, candidates);
        assert_eq!(denied.counters.verification_bytes, bytes);
        assert_eq!(denied.generation, index.generation());
    }
    let boundary = limited(
        &["abcd", "abcxbcd"],
        SubstringLimits {
            posting_operations: 4,
            candidate_spans: 1,
            verification_bytes: 4,
        },
    );
    assert_eq!(boundary.substring("abcd").unwrap().len(), 1);
    let index = limited(
        &["abcd", "other"],
        SubstringLimits {
            candidate_spans: 1,
            ..defaults
        },
    );
    assert_eq!(
        refusal(index.substring_exhaustive("abcd").unwrap_err()).reason,
        SubstringRefusalReason::CandidateSpans
    );
}

fn timestamp_punctuation_short_fragments_and_interior_word_match() {
    let index = index(&["adamantine", "2026-10-04T12:30:00Z", "a:b", "other"]);
    assert_eq!(
        index.substring("damant").unwrap()[0].ranges,
        std::slice::from_ref(&(1..7))
    );
    assert_eq!(
        index.substring(":30").unwrap()[0].ranges,
        std::slice::from_ref(&(13..16))
    );
    let colon = index.substring(":").unwrap();
    assert_eq!(colon.len(), 2);
    assert_eq!(colon[0].ranges, [13..14, 16..17]);
    assert_eq!(index.substring(":b").unwrap()[0].term.text(), "a:b");
    assert!(index.substring("two words").is_err());
    assert!(index.substring("").is_err());
}

fn emoji_endpoints_are_filtered_before_selectivity() {
    let singleton = index(&["👩‍💻"]);
    let partial = singleton.substring_report("👩").unwrap();
    assert_eq!(partial.matches.len(), 0);
    assert_eq!(partial.counters.candidate_spans, 0);
    assert_eq!(partial.counters.verification_bytes, 0);
    assert_eq!(
        refusal(singleton.substring("👩‍💻").unwrap_err()).reason,
        SubstringRefusalReason::NonSelective
    );
    let index = index(&["👩‍💻", "👩", "🇺🇸", "👍🏽", "1️⃣", "other"]);
    assert_eq!(index.substring("👩").unwrap()[0].term.documents(), [1]);
    for probe in ["💻", "🇺", "🇸", "👍", "🏽", "1"] {
        assert_eq!(index.substring(probe).unwrap().len(), 0, "{probe}");
    }
    assert_eq!(index.substring("👍🏽").unwrap()[0].term.documents(), [3]);
}

fn retained_controls_remain_searchable_and_do_not_form_false_adjacencies() {
    let index = index(&["a\u{ad}b", "a\u{200c}b", "a\u{fe0f}b", "other"]);
    assert_eq!(index.substring("ab").unwrap().len(), 0);
    for (document, probe) in ["\u{ad}", "\u{200c}", "\u{fe0f}"].into_iter().enumerate() {
        let matches = index.substring(probe).unwrap();
        assert_eq!(matches.len(), 1, "{probe:?}");
        assert_eq!(matches[0].term.documents(), [document as u32]);
    }
}

fn html_composition_and_multiscalar_entities_retain_original_contributors() {
    let analyzer = Analyzer::with_profile(
        AnalyzerProfile::empty_lexicon()
            .with_input_mode(InputMode::HtmlText)
            .with_accent_fold(AccentFold::Preserve),
    )
    .unwrap();
    let index = SurfaceIndex::from_texts(
        analyzer,
        [(7, "e&#x301; &amp;lt; &NotEqualTilde;"), (8, "other")],
    )
    .unwrap();
    let accent = index.substring("é").unwrap();
    assert_eq!(accent[0].evidence[0].sources, std::slice::from_ref(&(0..8)));
    assert_eq!(accent[0].evidence[0].highlight, 0..8);
    assert_eq!(accent[0].evidence[0].projected_range, 0..2);
    let once = index.substring("&amp;lt;").unwrap();
    assert_eq!(once[0].term.text(), "&lt;");
    assert_eq!(once[0].evidence[0].sources, std::slice::from_ref(&(9..17)));
    let entity = index.substring("≂").unwrap();
    assert_eq!(
        entity[0].evidence[0].sources,
        std::slice::from_ref(&(18..33))
    );
    assert_eq!(entity[0].evidence[0].document, 7);
    assert_eq!(
        entity[0].evidence[0].projection,
        MatchProjection::SubstringSpan
    );
    assert_eq!(
        entity[0].evidence[0].analyzer,
        index.analyzer().fingerprint()
    );
    assert_eq!(entity[0].evidence[0].generation, index.generation());
    assert!(!entity[0].evidence[0].coarse);
    assert!(matches!(
        SurfaceIndex::from_texts(index.analyzer().clone(), [(0, "safe"), (1, "&copy")]),
        Err(TextError::Html(_))
    ));
}

fn source_offsets_never_masquerade_as_normalized_offsets() {
    let index = index(&["é Straße", "a\u{2066}b", "other"]);
    let matched = index.substring("ss").unwrap();
    let evidence = &matched[0].evidence[0];
    assert_eq!(evidence.projected_range, 6..8);
    assert_eq!(evidence.sources, std::slice::from_ref(&(7..9)));
    assert_eq!(evidence.highlight, 7..9);
    let across_removed_bidi = index.substring("ab").unwrap();
    let evidence = &across_removed_bidi[0].evidence[0];
    assert_eq!(evidence.sources, [0..1, 4..5]);
    assert_eq!(evidence.highlight, 0..5);
    assert_eq!(evidence.projected_range, 0..2);
}

fn repeated_literals_and_input_order_have_canonical_identity_and_evidence() {
    let rows = [(0, "adamantine"), (0, "prefix adamantine"), (1, "other")];
    let left = SurfaceIndex::from_texts(Analyzer::empty_lexicon(), rows).unwrap();
    let right = SurfaceIndex::from_texts(
        Analyzer::empty_lexicon(),
        rows.into_iter().rev().chain([(0, "adamantine")]),
    )
    .unwrap();
    assert_eq!(left.generation(), right.generation());
    assert_eq!(
        left.substring_report("damant").unwrap(),
        right.substring_report("damant").unwrap()
    );
    let result = left.substring("damant").unwrap();
    assert_eq!(result[0].evidence.len(), 2);
    assert_ne!(result[0].evidence[0].literal, result[0].evidence[1].literal);
}

fn phonetics_refine_the_same_canonical_widened_spelling_they_encode() {
    let index = index(&[
        "Œuvre",
        "oeuvre",
        "Ŋoma",
        "noma",
        "O’Connor",
        "O'Connor",
        "中文",
    ]);
    for (query, expected) in [("oeuvre", 2), ("noma", 2), ("oconnor", 2)] {
        let matches = index.phonetic(query).unwrap();
        assert_eq!(matches.len(), expected, "{query}");
        for matched in matches {
            assert_eq!(matched.distance, 0);
            assert_eq!(matched.evidence.len(), 1);
            assert_eq!(matched.evidence[0].projection, MatchProjection::SurfaceWord);
        }
    }
    assert!(matches!(index.phonetic("中"), Err(TextError::Phonetic(_))));
    for (query, expected) in [("", 0), ("!", 0), (" \t", 0), ("two words", 2)] {
        assert!(matches!(
            index.phonetic(query),
            Err(TextError::Phonetic(
                purrdf_text::phonetic::PhoneticRefusal::SurfaceWordCount { observed }
            )) if observed == expected
        ));
    }
}

fn oversized_first_grapheme_has_full_positions_and_source_evidence() {
    let grapheme = format!("क{}", "ा".repeat(128));
    let original = format!("{grapheme}tail");
    let index = index(&[original.as_str(), "other"]);
    let matches = index.substring(&grapheme).unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].term.text().chars().count(), 129);
    assert_eq!(matches[0].evidence[0].highlight, 0..grapheme.len());
    let marks = index.substring("ा").unwrap();
    assert_eq!(marks[0].ranges.len(), 128);
    assert_eq!(marks[0].ranges[127], 384..387);
}

fn every_small_scalar_query_matches_an_independent_exhaustive_oracle() {
    let mut universe = Vec::new();
    for length in 1..=4 {
        for mut pattern in 0..3_usize.pow(length) {
            let text: String = (0..length)
                .map(|_| {
                    let c = ['a', '𰀀', ':'][pattern % 3];
                    pattern /= 3;
                    c
                })
                .collect();
            universe.push(text);
        }
    }
    let index = SurfaceIndex::from_texts(
        Analyzer::empty_lexicon(),
        universe
            .iter()
            .enumerate()
            .map(|(id, text)| (id as u32, text.as_str())),
    )
    .unwrap();
    for query in &universe {
        let expected: Vec<_> = index
            .spans()
            .iter()
            .filter_map(|term| {
                let ranges: Vec<_> = term
                    .text()
                    .char_indices()
                    .filter_map(|(at, _)| {
                        term.text()[at..]
                            .starts_with(query)
                            .then_some(at..at + query.len())
                    })
                    .collect();
                (!ranges.is_empty()).then_some((term.text(), ranges))
            })
            .collect();
        let indexed = index.substring(query).unwrap();
        let actual: Vec<_> = indexed
            .iter()
            .map(|matched| (matched.term.text(), matched.ranges.clone()))
            .collect();
        assert_eq!(actual, expected, "{query}");
    }
}

purrdf_testkit::harness_main!(
    positional_alignment_narrows_universal_gram_membership,
    repeated_grams_require_distinct_relative_positions_and_keep_overlap,
    singletons_and_universal_candidates_require_explicit_exhaustion,
    missing_gram_and_empty_index_are_complete_empty_without_posting_work,
    each_budget_refuses_atomically_at_its_first_excess_charge,
    timestamp_punctuation_short_fragments_and_interior_word_match,
    emoji_endpoints_are_filtered_before_selectivity,
    retained_controls_remain_searchable_and_do_not_form_false_adjacencies,
    html_composition_and_multiscalar_entities_retain_original_contributors,
    source_offsets_never_masquerade_as_normalized_offsets,
    repeated_literals_and_input_order_have_canonical_identity_and_evidence,
    phonetics_refine_the_same_canonical_widened_spelling_they_encode,
    oversized_first_grapheme_has_full_positions_and_source_evidence,
    every_small_scalar_query_matches_an_independent_exhaustive_oracle,
);
