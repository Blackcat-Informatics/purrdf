// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Independent Han statistics, dictionary-boundary recall and source evidence.

use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral, TermValue};
use purrdf_sparql_eval::{CandidateDomains, RankFidelity};
use purrdf_text::segment::Dictionary;
use purrdf_text::{
    Analyzer, AnalyzerProfile, GraphSelector, HanCharacterIndex, Segmentation, TextIndex,
    TextIndexConfig, TextSearchRelation, rank_partition,
};

fn data(rows: &[(&str, &str)]) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri("https://example.org/text");
    for &(subject, text) in rows {
        let subject = builder.intern_iri(&format!("https://example.org/{subject}"));
        let literal = builder.intern_literal(RdfLiteral::simple(text));
        builder.push_quad(subject, predicate, literal, None);
    }
    builder.freeze().expect("fixture dataset")
}
fn config(analyzer: Analyzer) -> TextIndexConfig {
    TextIndexConfig::new(
        vec![TermValue::iri("https://example.org/text")],
        GraphSelector::Default,
        analyzer,
    )
    .expect("explicit configuration")
}
#[path = "support/index.rs"]
mod index_fixture;

#[test]
fn character_recall_is_independent_of_dictionary_boundaries_and_lexical_statistics() {
    let dictionary = Dictionary::new(["北京大学", "北京"].map(str::to_owned)).expect("dictionary");
    let analyzer = Analyzer::with_profile(
        AnalyzerProfile::standard()
            .with_segmentation(Segmentation::Dictionary(Arc::new(dictionary))),
    )
    .expect("caller dictionary");
    let config = config(analyzer);
    let data = data(&[("a", "北京大学"), ("b", "夏"), ("c", "plain")]);
    let lexical = TextIndex::from_dataset(&*data, &config).expect("lexical index");
    let character = HanCharacterIndex::from_dataset(&*data, &config).expect("character index");
    assert_eq!(lexical.term_frequency(0, "北京大学"), 1);
    assert_eq!(lexical.term_frequency(0, "北京"), 0);
    assert_eq!(lexical.document_length(0), Some(1));
    assert_eq!(character.index().document_length(0), Some(7));
    assert_eq!(character.index().term_frequency(0, "北京"), 1);
    assert_eq!(character.index().term_frequency(0, "北"), 1);
    assert_eq!(
        character.index().query_terms("北京大学").expect("query"),
        ["北京", "京大", "大学"]
    );
    assert_eq!(character.index().query_terms("北").expect("query"), ["北"]);
    assert_eq!(
        character.index().query_terms("北 京").expect("query"),
        ["北", "京"]
    );
    assert_eq!(
        character
            .index()
            .partition_stats(&index_fixture::plain())
            .expect("partition")
            .document_count(),
        2
    );
    assert_eq!(
        lexical
            .partition_stats(&index_fixture::plain())
            .expect("partition")
            .document_count(),
        3
    );
    assert_ne!(lexical.fingerprint(), character.index().fingerprint());
    let relation = character.relation();
    assert_eq!(
        relation.index().fingerprint(),
        character.index().fingerprint()
    );
}

#[test]
fn repeated_grams_and_boundaries_preserve_counts_without_crossing_literals() {
    let data = data(&[
        ("a", "人人人"),
        ("b", "中"),
        ("b", "文"),
        ("c", "中。文"),
        ("d", "中👩‍💻文"),
    ]);
    let index = HanCharacterIndex::from_dataset(&*data, &config(Analyzer::empty_lexicon()))
        .expect("character index");
    assert_eq!(index.index().term_frequency(0, "人"), 3);
    assert_eq!(index.index().term_frequency(0, "人人"), 2);
    assert_eq!(index.index().document_length(0), Some(5));
    for document in 1..=3 {
        assert_eq!(index.index().term_frequency(document, "中文"), 0);
    }
    assert_eq!(index.evidence(0, "人人").len(), 2);
}

#[test]
fn supplementary_han_and_controls_have_exact_contributors_and_whole_cluster_highlight() {
    let text = "𠀀\u{e0100}\u{200c}字";
    let data = data(&[("a", text)]);
    let index = HanCharacterIndex::from_dataset(&*data, &config(Analyzer::empty_lexicon()))
        .expect("character index");
    assert_eq!(index.index().query_terms(text).expect("query"), ["𠀀字"]);
    let evidence = index.evidence(0, "𠀀字");
    assert_eq!(evidence.len(), 1);
    let evidence = &evidence[0];
    assert_eq!(evidence.projection.text, "𠀀字");
    assert_eq!(evidence.projection.sources, [0..4, 11..14]);
    assert_eq!(evidence.projection.highlight, 0..14);
    assert_eq!(evidence.generation, index.index().fingerprint());
    assert_eq!(evidence.analyzer, index.index().analyzer_fingerprint());
    assert_ne!(evidence.literal, [0; 32]);
}

#[test]
fn auxiliary_only_documents_remain_addressable_without_changing_lexical_scores() {
    let config = config(Analyzer::empty_lexicon());
    let baseline = data(&[("a", "cat")]);
    let extra = data(&[("a", "cat"), ("b", "---")]);
    let baseline = TextIndex::from_dataset(&*baseline, &config).expect("baseline");
    let extra = TextIndex::from_dataset(&*extra, &config).expect("extra");
    assert_eq!(extra.document_count(), 2);
    assert_eq!(extra.document_length(1), Some(0));
    assert_eq!(
        extra.partition_stats(&index_fixture::plain()),
        baseline.partition_stats(&index_fixture::plain())
    );
    assert_eq!(
        rank_partition(&extra, &index_fixture::plain(), &["cat".to_owned()], None).expect("rank"),
        rank_partition(
            &baseline,
            &index_fixture::plain(),
            &["cat".to_owned()],
            None
        )
        .expect("rank")
    );
    assert!(
        extra
            .surface_index()
            .spans()
            .iter()
            .any(|term| term.text() == "---")
    );
}

#[test]
fn evidence_generation_distinguishes_original_forms_with_identical_character_keys() {
    let config = config(Analyzer::empty_lexicon());
    let plain = data(&[("a", "漢字")]);
    let selector = data(&[("a", "漢\u{fe00}字")]);
    let plain = HanCharacterIndex::from_dataset(&*plain, &config).expect("plain");
    let selector = HanCharacterIndex::from_dataset(&*selector, &config).expect("selector");
    assert_eq!(
        plain.index().terms().collect::<Vec<_>>(),
        selector.index().terms().collect::<Vec<_>>()
    );
    assert_ne!(plain.index().fingerprint(), selector.index().fingerprint());
    assert_ne!(
        plain.evidence(0, "漢字")[0].literal,
        selector.evidence(0, "漢字")[0].literal
    );
}

#[test]
fn html_references_keep_original_contributors_in_character_evidence() {
    let analyzer = Analyzer::with_profile(
        AnalyzerProfile::empty_lexicon().with_input_mode(purrdf_text::InputMode::HtmlText),
    )
    .expect("HTML profile");
    let data = data(&[("a", "&#x4E2D;&#25991;")]);
    let index = HanCharacterIndex::from_dataset(&*data, &config(analyzer)).expect("index");
    assert_eq!(index.index().query_terms("中文").expect("query"), ["中文"]);
    for (term, range) in [("中", 0..8), ("文", 8..16), ("中文", 0..16)] {
        let evidence = index.evidence(0, term);
        assert_eq!(evidence.len(), 1);
        assert_eq!(evidence[0].projection.sources.len(), 1);
        assert_eq!(evidence[0].projection.sources[0], range);
    }
}

#[test]
fn long_han_runs_have_bounded_keys_without_losing_later_characters() {
    let input = "中".repeat(200) + "𠮷";
    let data = data(&[("a", &input)]);
    let index = HanCharacterIndex::from_dataset(&*data, &config(Analyzer::empty_lexicon()))
        .expect("long character run");
    assert_eq!(index.index().document_length(0), Some(401));
    assert_eq!(index.index().term_frequency(0, "𠮷"), 1);
    assert_eq!(index.index().term_frequency(0, "中𠮷"), 1);
    assert!(index.index().terms().all(|term| term.chars().count() <= 2));
}

#[test]
fn auxiliary_only_graph_and_language_partitions_allow_ranked_declaration() {
    for lexical in [false, true] {
        let mut builder = RdfDatasetBuilder::new();
        let predicate = builder.intern_iri("https://example.org/text");
        if lexical {
            let subject = builder.intern_iri("https://example.org/lexical");
            let text = builder.intern_literal(RdfLiteral::simple("cat"));
            builder.push_quad(subject, predicate, text, None);
        }
        for (name, language, graph) in [
            ("aux-default", "fr", None),
            ("aux-named", "zh", Some("https://example.org/graph")),
        ] {
            let subject = builder.intern_iri(&format!("https://example.org/{name}"));
            let text = builder.intern_literal(RdfLiteral::language_tagged("---", language));
            let graph = graph.map(|graph| builder.intern_iri(graph));
            builder.push_quad(subject, predicate, text, graph);
        }
        let dataset = builder.freeze().expect("auxiliary partitions");
        let config = TextIndexConfig::new(
            vec![TermValue::iri("https://example.org/text")],
            GraphSelector::Any,
            Analyzer::empty_lexicon(),
        )
        .expect("configuration");
        let index = TextIndex::from_dataset(&*dataset, &config).expect("index");
        assert_eq!(index.partition_count(), 2 + u64::from(lexical));
        assert_eq!(
            index
                .partitions()
                .filter(|(_, stats)| stats.document_count() != 0)
                .count(),
            usize::from(lexical)
        );
        let relation = TextSearchRelation::new(Arc::new(index));
        relation
            .ranked_declaration(
                purrdf_core::parse_iri("https://example.org/stratum").expect("IRI"),
                None,
                RankFidelity::EXACT,
                CandidateDomains::Unrestricted,
            )
            .expect("only active scoring partitions constrain the rank declaration");
    }
}

#[path = "support/character_cases.rs"]
mod character_cases;

#[test]
fn prepend_scalars_do_not_hide_han_bases() {
    character_cases::prepend_scalars_do_not_hide_han_bases();
}
