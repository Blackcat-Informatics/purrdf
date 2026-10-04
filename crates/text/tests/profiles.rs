// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Stage conformance and index/query identity invariants.

use purrdf_core::{RdfDatasetBuilder, RdfLiteral, TermValue};
use purrdf_sparql_eval::PropertyFunctionRegistry;
use purrdf_text::{
    AccentFold, Analyzer, AnalyzerProfile, GraphSelector, InputMode, Segmentation, Stemming,
    SubstringLimits, SurfaceIndex, TermOccurrenceRelation, TextIndex, TextIndexConfig,
    TextSearchRelation, segment::Dictionary,
};
use std::sync::Arc;

#[path = "support/sparql.rs"]
mod sparql;

fn analyzer(bound: usize, accent: AccentFold, stemming: Stemming) -> Analyzer {
    Analyzer::with_profile(
        AnalyzerProfile::new(bound)
            .unwrap()
            .with_segmentation(Segmentation::EmptyLexicon)
            .with_accent_fold(accent)
            .with_stemming(stemming),
    )
    .unwrap()
}

#[test]
fn folded_surface_stemming_and_scalar_bounds_are_separate_projections() {
    let exact = Analyzer::empty_lexicon();
    let stemmed = analyzer(256, AccentFold::Preserve, Stemming::English);
    assert_eq!(
        stemmed
            .terms("running relational")
            .expect("valid text analysis"),
        ["run", "relat"]
    );
    assert_eq!(
        stemmed
            .surface_terms("running relational")
            .expect("valid text analysis"),
        ["running", "relational"]
    );
    assert_eq!(
        stemmed
            .terms("中文running run中文")
            .expect("valid text analysis"),
        ["中", "文", "run", "run", "中", "文"]
    );
    for spelling in ["Straße", "STRASSE", "strasse"] {
        assert_eq!(
            exact.terms(spelling).expect("valid text analysis"),
            ["strasse"]
        );
        assert_eq!(
            stemmed.terms(spelling).expect("valid text analysis"),
            ["strass"]
        );
    }
    let limit = purrdf_text::unicode::MAX_EMOJI_SCALARS;
    let bounded = analyzer(limit, AccentFold::Preserve, Stemming::None);
    assert_eq!(
        bounded.terms(&format!("aß{}", "c".repeat(limit))).unwrap(),
        [format!("ass{}", "c".repeat(limit - 3))]
    );
    assert_eq!(
        bounded.terms(&"é".repeat(limit + 1)).unwrap(),
        ["é".repeat(limit)]
    );
    assert_eq!(
        bounded.substring_terms(&"𠀀".repeat(limit + 1)).unwrap(),
        ["𠀀".repeat(limit)]
    );
    let run = "x".repeat(100_000);
    assert_eq!(bounded.terms(&run).unwrap(), ["x".repeat(limit)]);
    assert_eq!(bounded.surface_terms(&run).unwrap(), ["x".repeat(limit)]);
    assert_eq!(bounded.substring_terms(&run).unwrap(), ["x".repeat(limit)]);
}

#[test]
fn accent_fold_respects_script_scope_and_mark_category() {
    let fold = analyzer(256, AccentFold::LatinGreekCyrillic, Stemming::None);
    assert_eq!(
        fold.terms("café cafe cafe\u{301}")
            .expect("valid text analysis"),
        ["cafe", "cafe", "cafe"]
    );
    // U+0902 DEVANAGARI SIGN ANUSVARA is Mn with canonical combining class zero.
    assert!(purrdf_text::unicode::is_nonspacing_mark('\u{0902}'));
    assert_eq!(purrdf_lex::unicode::ccc('\u{0902}'), 0);
    assert_eq!(fold.analysis_form("अं").expect("valid text analysis"), "अं");
    // Mc is not Mn: the spacing vowel sign remains.
    assert_eq!(fold.analysis_form("का").expect("valid text analysis"), "का");
    assert_eq!(
        Analyzer::empty_lexicon()
            .terms("café cafe")
            .expect("valid text analysis"),
        ["cafe", "cafe"]
    );
}

#[test]
fn controls_follow_each_projection_and_preserve_emoji() {
    let analyzer = Analyzer::empty_lexicon();
    assert_eq!(
        analyzer.terms("a\u{200b}b").expect("valid text analysis"),
        ["a", "b"]
    );
    for control in [
        '\u{200d}', '\u{200e}', '\u{200f}', '\u{202a}', '\u{202e}', '\u{2066}', '\u{2069}',
        '\u{061c}', '\u{feff}', '\u{00ad}',
    ] {
        let input = format!("foo{control}bar");
        assert_eq!(
            analyzer.terms(&input).expect("valid text analysis"),
            ["foobar"],
            "{control:?}"
        );
        let expected = if control == '\u{00ad}' {
            input.clone()
        } else {
            "foobar".to_owned()
        };
        assert_eq!(
            analyzer
                .substring_terms(&input)
                .expect("valid text analysis"),
            [expected]
        );
    }
    assert_eq!(
        analyzer
            .analysis_form("👩\u{fe0f}\u{200d}💻")
            .expect("valid text analysis"),
        "👩\u{fe0f}\u{200d}💻"
    );
    let input = "alpha 👩\u{fe0f}\u{200d}💻 beta";
    assert_eq!(
        analyzer.terms(input).expect("valid text analysis"),
        ["alpha", "👩\u{fe0f}\u{200d}💻", "beta"]
    );
    for term in analyzer
        .substring_terms(input)
        .expect("valid text analysis")
    {
        assert!(
            !term.chars().any(purrdf_text::unicode::is_removed_control)
                || purrdf_text::unicode::is_emoji_grapheme(&term)
        );
    }
}

#[test]
fn every_profile_dimension_changes_the_analysis_identity() {
    let base = AnalyzerProfile::new(256)
        .unwrap()
        .with_segmentation(Segmentation::EmptyLexicon);
    let dictionaries = [
        vec![("知识".to_owned(), 1)],
        vec![("知识".to_owned(), 2)],
        vec![("知识图谱".to_owned(), 1)],
    ];
    let mut profiles = vec![
        base.clone(),
        AnalyzerProfile::new(255)
            .unwrap()
            .with_segmentation(Segmentation::EmptyLexicon),
        base.clone().with_accent_fold(AccentFold::Preserve),
        base.clone().with_stemming(Stemming::English),
        base.clone().with_phonetics(5, 2).unwrap(),
        base.clone().with_phonetics(4, 3).unwrap(),
        base.clone().with_input_mode(InputMode::HtmlText),
        base.clone().with_input_mode(InputMode::HtmlAttribute),
        base.clone().with_substring_limits(SubstringLimits {
            posting_operations: 0,
            ..SubstringLimits::STANDARD
        }),
        base.clone().with_substring_limits(SubstringLimits {
            candidate_spans: 0,
            ..SubstringLimits::STANDARD
        }),
        base.clone().with_substring_limits(SubstringLimits {
            verification_bytes: 0,
            ..SubstringLimits::STANDARD
        }),
    ];
    profiles.extend(dictionaries.map(|words| {
        base.clone()
            .with_segmentation(Segmentation::Dictionary(Arc::new(
                Dictionary::with_costs(words).unwrap(),
            )))
    }));
    let identities: std::collections::BTreeSet<_> = profiles
        .into_iter()
        .map(|profile| Analyzer::with_profile(profile).unwrap().fingerprint())
        .collect();
    assert_eq!(identities.len(), 14);
    assert!(AnalyzerProfile::new(0).is_err());
    assert!(AnalyzerProfile::new(1025).is_err());
    assert!(base.clone().with_phonetics(0, 2).is_err());
    assert!(base.with_phonetics(4, 65).is_err());
}

#[test]
fn substring_candidates_are_verified_and_short_fragments_have_postings() {
    let index = SurfaceIndex::from_texts(
        Analyzer::empty_lexicon(),
        [
            (7, "adamantine 12:30 abcxxbcd aaaa"),
            (3, "ADAMANTINE 13:31"),
        ],
    )
    .unwrap();
    let matches = index.substring("damant").unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].term.text(), "adamantine");
    assert_eq!(matches[0].term.documents(), [3, 7]);
    assert_eq!(matches[0].ranges.as_slice(), std::slice::from_ref(&(1..7)));
    assert_eq!(index.substring(":30").unwrap()[0].term.text(), "12:30");
    assert_eq!(index.substring(":3").unwrap().len(), 2);
    assert_eq!(index.substring(":").unwrap().len(), 2);
    assert_eq!(index.substring("abcd").unwrap().len(), 0);
    assert_eq!(index.substring("aaa").unwrap()[0].ranges, [0..3, 1..4]);
    assert!(index.substring("").is_err());
    assert!(index.substring("two words").is_err());
    let limit = purrdf_text::unicode::MAX_EMOJI_SCALARS;
    let long_word = "a".repeat(limit + 3);
    let input = format!("12:34{} {long_word}", "5".repeat(limit));
    let bounded = SurfaceIndex::from_texts(
        analyzer(limit, AccentFold::Preserve, Stemming::None),
        [(0, input.as_str())],
    )
    .unwrap();
    assert_eq!(
        bounded.substring(":34").unwrap()[0]
            .term
            .text()
            .chars()
            .count(),
        limit
    );
    assert_eq!(bounded.substring("678").unwrap().len(), 0);
    assert_eq!(
        bounded.substring(&long_word).unwrap()[0].term.text(),
        "a".repeat(limit)
    );
}

#[test]
fn phonetics_read_surface_words_and_expose_refinement_evidence() {
    let analyzer = analyzer(256, AccentFold::Preserve, Stemming::English);
    let index =
        SurfaceIndex::from_texts(analyzer, [(1, "Smith Schmidt running"), (2, "Smyth 中文")])
            .unwrap();
    let matches = index.phonetic("Smith").unwrap();
    assert_eq!(
        matches
            .iter()
            .map(|m| (m.term.text(), m.distance))
            .collect::<Vec<_>>(),
        [("smith", 0), ("smyth", 1)]
    );
    assert!(matches.iter().all(|m| !m.shared_codes.is_empty()));
    assert!(index.words().iter().any(|term| term.text() == "running"));
    assert_eq!(index.phonetic("running").unwrap()[0].term.text(), "running");
    assert!(index.phonetic("中文").is_err());
    assert!(index.phonetic("foo bar").is_err());
}

#[test]
fn search_and_occurrence_relations_use_the_stored_analyzer() {
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri("https://example.org/doc");
    let predicate = builder.intern_iri("https://example.org/text");
    let literal = builder.intern_literal(RdfLiteral::simple("CAFÉS running alpha\u{200b}beta"));
    builder.push_quad(subject, predicate, literal, None);
    let dataset = builder.freeze().unwrap();
    let configuration = TextIndexConfig::new(
        vec![TermValue::iri("https://example.org/text")],
        GraphSelector::Any,
        Analyzer::empty_lexicon(),
    )
    .unwrap()
    .with_analyzer(analyzer(
        256,
        AccentFold::LatinGreekCyrillic,
        Stemming::English,
    ));
    let index = Arc::new(TextIndex::from_dataset(&*dataset, &configuration).unwrap());
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        "https://example.org/search".to_owned(),
        Arc::new(TextSearchRelation::new(Arc::clone(&index))),
    );
    registry.register(
        "https://example.org/occurs".to_owned(),
        Arc::new(TermOccurrenceRelation::new(Arc::clone(&index))),
    );
    assert_eq!(
        sparql::answer(
            &dataset,
            &registry,
            "SELECT ?doc WHERE { ?doc <https://example.org/search> (\"cafe runs\" ?score ?rank ?lang ?matched) }"
        ),
        [vec!["<https://example.org/doc>".to_owned()]]
    );
    assert_eq!(
        sparql::answer(
            &dataset,
            &registry,
            "SELECT ?doc WHERE { ?doc <https://example.org/occurs> (\"runs\" ?lang ?position) }"
        ),
        [vec!["<https://example.org/doc>".to_owned()]]
    );
    assert_eq!(
        index.surface_index().substring("afé").unwrap()[0]
            .term
            .text(),
        "cafes"
    );
    assert_eq!(
        index.analyzer_fingerprint(),
        configuration.analyzer().fingerprint()
    );
    assert_eq!(
        index.surface_index().analyzer().fingerprint(),
        index.analyzer_fingerprint()
    );
}

#[test]
fn empty_indexes_still_bind_the_entire_analysis_law() {
    let dataset = RdfDatasetBuilder::new().freeze().unwrap();
    let config = TextIndexConfig::new(
        vec![TermValue::iri("https://example.org/text")],
        GraphSelector::Any,
        Analyzer::empty_lexicon(),
    )
    .unwrap();
    let exact = TextIndex::from_dataset(&*dataset, &config).unwrap();
    let stemmed = TextIndex::from_dataset(
        &*dataset,
        &config.with_analyzer(analyzer(256, AccentFold::Preserve, Stemming::English)),
    )
    .unwrap();
    assert_eq!(exact.source_fingerprint(), stemmed.source_fingerprint());
    assert_ne!(exact.analyzer_fingerprint(), stemmed.analyzer_fingerprint());
    assert_ne!(exact.fingerprint(), stemmed.fingerprint());
}
