// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use purrdf_core::{RdfDatasetBuilder, RdfLiteral, TermValue};
use purrdf_text::{Analyzer, GraphSelector, HanCharacterIndex, TextIndexConfig};

pub(crate) fn prepend_scalars_do_not_hide_han_bases() {
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri("https://example.org/text");
    for (subject, text) in [
        ("a", "\u{600}中"),
        ("b", "中\u{600}文"),
        ("c", "\u{600}中\u{fe00}\u{200c}文"),
        ("d", "中\u{16ff0}文"),
        ("e", "中\u{16ff1}文"),
    ] {
        let subject = builder.intern_iri(&format!("https://example.org/{subject}"));
        let text = builder.intern_literal(RdfLiteral::simple(text));
        builder.push_quad(subject, predicate, text, None);
    }
    let dataset = builder.freeze().expect("dataset");
    let config = TextIndexConfig::new(
        vec![TermValue::iri("https://example.org/text")],
        GraphSelector::Default,
        Analyzer::empty_lexicon(),
    )
    .expect("configuration");
    let han = HanCharacterIndex::from_dataset(&*dataset, &config).expect("Han index");
    assert_eq!(han.index().query_terms("\u{600}中").unwrap(), ["中"]);
    assert_eq!(
        han.index().query_terms("中\u{600}文").unwrap(),
        ["中", "文"]
    );
    assert_eq!(han.index().term_frequency(0, "中"), 1);
    assert_eq!(han.index().term_frequency(1, "中"), 1);
    assert_eq!(han.index().term_frequency(1, "文"), 1);
    assert_eq!(han.index().term_frequency(1, "中文"), 0);
    assert_eq!(han.index().term_frequency(2, "中文"), 1);
    for (document, mark) in [(3, '\u{16ff0}'), (4, '\u{16ff1}')] {
        assert_eq!(han.index().term_frequency(document, "中文"), 1);
        assert_eq!(han.index().term_frequency(document, &mark.to_string()), 0);
        assert_eq!(han.index().document_length(document), Some(3));
        assert_eq!(
            han.index().query_terms(&format!("中{mark}文")).unwrap(),
            ["中文"]
        );
        let evidence = han.evidence(document, "中");
        assert_eq!(
            evidence[0].projection.sources,
            std::slice::from_ref(&(0..3))
        );
        assert_eq!(evidence[0].projection.highlight, 0..7);
    }
    for (document, key, bounds, highlight) in [
        (0, "中", vec![(2, 5)], 0..5),
        (1, "文", vec![(5, 8)], 3..8),
        (2, "中文", vec![(2, 5), (11, 14)], 0..14),
    ] {
        let evidence = han.evidence(document, key);
        assert_eq!(evidence.len(), 1);
        let sources: Vec<_> = bounds.into_iter().map(|(start, end)| start..end).collect();
        assert_eq!(evidence[0].projection.sources, sources);
        assert_eq!(evidence[0].projection.highlight, highlight);
    }
}
