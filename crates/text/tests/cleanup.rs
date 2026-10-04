// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Malformed emoji controls cannot acquire protection from a neighboring atom.

use purrdf_core::{RdfDatasetBuilder, RdfLiteral, TermValue};
use purrdf_text::{
    Analyzer, AnalyzerProfile, AnalyzerScratch, GraphSelector, Stemming, SurfaceIndex, TextIndex,
    TextIndexConfig,
};

#[test]
fn every_standardized_emoji_presentation_keeps_its_control() {
    let analyzer = Analyzer::empty_lexicon();
    let data = include_str!("../../iri/unicode/17.0.0/emoji-variation-sequences.txt");
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
        assert_eq!(
            analyzer.terms(&atom).unwrap(),
            std::slice::from_ref(&atom),
            "{line}"
        );
        assert_eq!(analyzer.analysis_form(&atom).unwrap(), atom, "{line}");
        rows += 1;
    }
    assert_eq!(rows, 742);
}

#[test]
fn joins_require_complete_elements_in_the_admitted_grapheme() {
    let analyzer = Analyzer::empty_lexicon();
    for (input, normalized, terms) in [
        ("🇦\u{200d}👩", "🇦👩", ["🇦", "👩"]),
        ("🏿\u{200d}👩", "🏿👩", ["🏿", "👩"]),
        ("☕\u{fe0e}\u{200d}👩", "☕\u{fe0e}👩", ["☕\u{fe0e}", "👩"]),
    ] {
        assert_eq!(
            analyzer.analysis_form(input).unwrap(),
            normalized,
            "{input:?}"
        );
        assert_eq!(analyzer.terms(input).unwrap(), terms, "{input:?}");
    }
}

#[test]
fn pictographic_boundary_reservations_do_not_protect_controls() {
    // U+1F02C is Extended_Pictographic, but has no Emoji property in the
    // pinned emoji-data.txt. Its reserved EGC boundary is not a control base.
    let analyzer = Analyzer::empty_lexicon();
    for (input, normalized, terms) in [
        ("\u{1f02c}\u{200d}👩", "\u{1f02c}👩", ["\u{1f02c}", "👩"]),
        ("👩\u{200d}\u{1f02c}", "👩\u{1f02c}", ["👩", "\u{1f02c}"]),
    ] {
        assert_eq!(
            analyzer.analysis_form(input).unwrap(),
            normalized,
            "{input:?}"
        );
        assert_eq!(analyzer.terms(input).unwrap(), terms, "{input:?}");
    }
    assert_eq!(
        analyzer
            .analysis_form("\u{1f02c}\u{e0020}\u{e007f}")
            .unwrap(),
        "\u{1f02c}"
    );
    for variation in ['\u{fe0e}', '\u{fe0f}'] {
        let input = format!("\u{1f02c}{variation}");
        assert_eq!(analyzer.terms(&input).unwrap(), ["\u{1f02c}"]);
        assert_eq!(analyzer.surface_terms(&input).unwrap(), [input]);
    }
}

#[test]
fn emoji_controls_require_complete_elements_in_every_projection() {
    let analyzer = Analyzer::empty_lexicon();
    for (input, normalized, lexical) in [
        ("👩\u{200d}", "👩", "👩"),
        ("👩\u{e0020}", "👩", "👩"),
        ("👩\u{e007f}", "👩", "👩"),
        ("👩\u{e0020}\u{e0021}", "👩", "👩"),
        ("☕\u{200c}", "☕\u{200c}", "☕"),
        ("☕\u{fe00}", "☕\u{fe00}", "☕"),
        ("☕\u{fe0f}\u{200c}", "☕\u{fe0f}\u{200c}", "☕\u{fe0f}"),
        ("🧠\u{200d}👩", "🧠\u{200d}👩", "🧠\u{200d}👩"),
        ("👩\u{200d}🏿", "👩\u{200d}🏿", "👩\u{200d}🏿"),
        ("🧠🏿\u{200d}👩", "🧠🏿\u{200d}👩", "🧠🏿\u{200d}👩"),
        (
            "🇦\u{e0020}\u{e007f}",
            "🇦\u{e0020}\u{e007f}",
            "🇦\u{e0020}\u{e007f}",
        ),
        (
            "👩\u{200d}🏿\u{e0020}\u{e007f}",
            "👩\u{200d}🏿\u{e0020}\u{e007f}",
            "👩\u{200d}🏿\u{e0020}\u{e007f}",
        ),
        ("🏿\u{e0020}\u{e007f}", "🏿", "🏿"),
        ("1\u{e0020}\u{e007f}", "1", "1"),
        ("🧠\u{200d}👩\u{200d}", "🧠\u{200d}👩", "🧠\u{200d}👩"),
        (
            "🏴\u{e0067}\u{e0062}\u{e0065}\u{e006e}\u{e0067}\u{e007f}",
            "🏴\u{e0067}\u{e0062}\u{e0065}\u{e006e}\u{e0067}\u{e007f}",
            "🏴\u{e0067}\u{e0062}\u{e0065}\u{e006e}\u{e0067}\u{e007f}",
        ),
        (
            "👩\u{e0020}\u{e007f}",
            "👩\u{e0020}\u{e007f}",
            "👩\u{e0020}\u{e007f}",
        ),
        ("☕\u{fe0e}", "☕\u{fe0e}", "☕\u{fe0e}"),
        (
            "1\u{fe0f}\u{20e3}",
            "1\u{fe0f}\u{20e3}",
            "1\u{fe0f}\u{20e3}",
        ),
    ] {
        assert_eq!(
            analyzer.analysis_form(input).unwrap(),
            normalized,
            "{input:?}"
        );
        assert_eq!(analyzer.terms(input).unwrap(), [lexical], "{input:?}");
        assert_eq!(
            analyzer.surface_terms(input).unwrap(),
            [normalized],
            "{input:?}"
        );
        assert_eq!(
            analyzer.substring_terms(input).unwrap(),
            [normalized],
            "{input:?}"
        );
    }
    assert_eq!(analyzer.terms("👩\u{200d}a").unwrap(), ["👩", "a"]);
    assert_eq!(analyzer.analysis_form("👩\u{200d}a").unwrap(), "👩a");
}

#[test]
fn removed_and_lexical_controls_have_distinct_source_evidence() {
    let analyzer = Analyzer::empty_lexicon();
    let removed = analyzer.projections("👩\u{e0020}\u{200d}").unwrap();
    assert_eq!(removed.normalized.text, "👩");
    for projection in removed
        .lexical
        .iter()
        .chain(&removed.surface)
        .chain(&removed.spans)
    {
        assert_eq!(projection.text, "👩");
        assert_eq!(projection.sources, std::slice::from_ref(&(0..4)));
        assert_eq!(projection.highlight, 0..4);
        assert_eq!(projection.range, 0..4);
    }
    let lexical = analyzer.projections("☕\u{200c}").unwrap();
    assert_eq!(lexical.normalized.text, "☕\u{200c}");
    assert_eq!(lexical.lexical[0].sources, std::slice::from_ref(&(0..3)));
    assert_eq!(lexical.lexical[0].highlight, 0..3);
    assert_eq!(lexical.lexical[0].range, 0..3);
    for projection in lexical.surface.iter().chain(&lexical.spans) {
        assert_eq!(projection.sources, std::slice::from_ref(&(0..6)));
        assert_eq!(projection.highlight, 0..6);
        assert_eq!(projection.range, 0..6);
    }
}

#[test]
fn reusable_streaming_storage_obeys_the_same_control_law() {
    for stemming in [Stemming::None, Stemming::English] {
        let analyzer =
            Analyzer::with_profile(AnalyzerProfile::empty_lexicon().with_stemming(stemming))
                .unwrap();
        let mut scratch = AnalyzerScratch::default();
        let mut text_scratch = String::new();
        for input in [
            "👩\u{200d}",
            "☕\u{200c}",
            "🧠\u{200d}👩",
            "👩\u{e0020}",
            "RUNNING",
            "☕\u{fe0f}\u{200c}",
            "",
            "👩\u{200d}a",
        ] {
            let expected = analyzer
                .projections(input)
                .unwrap()
                .lexical
                .into_iter()
                .map(|projection| projection.text)
                .collect::<Vec<_>>();
            let mut streamed = Vec::new();
            analyzer
                .analyze_each_with_scratch(input, &mut scratch, |token| {
                    assert_eq!(token.position as usize, streamed.len());
                    streamed.push(token.text.into_owned());
                })
                .unwrap();
            assert_eq!(streamed, expected, "{input:?}");
            streamed.clear();
            analyzer
                .analyze_each(input, &mut text_scratch, |token| {
                    streamed.push(token.text.into_owned());
                })
                .unwrap();
            assert_eq!(streamed, expected, "{input:?}");
            let mut owned = Vec::new();
            analyzer.analyze(input, &mut owned).unwrap();
            assert_eq!(
                owned
                    .iter()
                    .map(|token| token.text.as_ref())
                    .collect::<Vec<_>>(),
                expected
            );
        }
    }
}

#[test]
fn index_queries_and_substring_evidence_share_corrected_cleanup() {
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri("https://example.org/text");
    for (number, input) in ["👩\u{200d}", "👩\u{e0020}", "☕\u{200c}", "🧠\u{200d}👩"]
        .into_iter()
        .enumerate()
    {
        let subject = builder.intern_iri(&format!("https://example.org/{number}"));
        let literal = builder.intern_literal(RdfLiteral::simple(input));
        builder.push_quad(subject, predicate, literal, None);
    }
    let dataset = builder.freeze().unwrap();
    let config = TextIndexConfig::new(
        vec![TermValue::iri("https://example.org/text")],
        GraphSelector::Default,
        Analyzer::empty_lexicon(),
    )
    .unwrap();
    let index = TextIndex::from_dataset(&*dataset, &config).unwrap();
    for (document, query, expected) in [
        (0, "👩\u{200d}", "👩"),
        (1, "👩\u{e0020}", "👩"),
        (2, "☕\u{200c}", "☕"),
        (3, "🧠\u{200d}👩", "🧠\u{200d}👩"),
    ] {
        assert_eq!(index.query_terms(query).unwrap(), [expected]);
        assert_eq!(index.term_frequency(document, expected), 1);
        assert_eq!(index.document_length(document), Some(1));
    }
    let surface = SurfaceIndex::from_texts(
        Analyzer::empty_lexicon(),
        [(0, "👩\u{200d}"), (1, "👩\u{e0020}")],
    )
    .unwrap();
    let matches = surface.substring_exhaustive("👩").unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].term.documents(), [0, 1]);
    assert_eq!(matches[0].evidence.len(), 2);
    for evidence in &matches[0].evidence {
        assert_eq!(evidence.sources, std::slice::from_ref(&(0..4)));
        assert_eq!(evidence.highlight, 0..4);
        assert_eq!(evidence.projected_range, 0..4);
        assert_eq!(evidence.analyzer, surface.analyzer().fingerprint());
        assert_eq!(evidence.generation, surface.generation());
    }
}
