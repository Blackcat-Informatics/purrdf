// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Allocation scaling of many protected emoji after a nonidentity transformation.
use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow, Measurement};
use purrdf_core::{RdfDatasetBuilder, RdfLiteral, TermValue};
use purrdf_sparql_eval::PropertyFunctionRegistry;
use purrdf_text::{
    Analyzer, GraphSelector, HanCharacterIndex, RankingProfile, TextIndex, TextIndexConfig,
    TextSearchRelation, rank_partition,
};
use std::sync::Arc;

#[path = "support/sparql.rs"]
mod sparql_fixture;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

#[test]
fn bm25f_only_omits_surface_storage_without_changing_ranked_answers() {
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri("https://example.org/text");
    for at in 0..256 {
        let subject = builder.intern_iri(&format!("https://example.org/document/{at}"));
        let literal = builder.intern_literal(RdfLiteral::simple(format!(
            "common 北京大学 document{at} substring{at} punctuation-bearing-{at} {}",
            "searchable words with different surfaces ".repeat(4)
        )));
        builder.push_quad(subject, predicate, literal, None);
    }
    let quiet = builder.intern_iri("https://example.org/quiet");
    let punctuation = builder.intern_literal(RdfLiteral::simple("--- ... !!!"));
    builder.push_quad(quiet, predicate, punctuation, None);
    let dataset = builder.freeze().expect("corpus");
    let config = TextIndexConfig::new(
        vec![TermValue::iri("https://example.org/text")],
        GraphSelector::Default,
        Analyzer::empty_lexicon(),
    )
    .expect("explicit configuration");
    let mut built = Vec::new();
    for enabled in [true, false] {
        let window = CurrentThreadWindow::open();
        let index = TextIndex::from_dataset(&*dataset, &config.clone().with_surface_index(enabled))
            .expect("index");
        let measurement = window.close();
        eprintln!("surface={enabled}: {measurement:?}");
        assert_eq!(index.surface_index().is_some(), enabled);
        built.push((index, measurement));
    }
    let [(full, full_allocations), (ranked, ranked_allocations)] = built.as_slice() else {
        unreachable!("both configurations built")
    };
    assert!(ranked_allocations.requested_bytes < full_allocations.requested_bytes);
    assert!(ranked_allocations.peak_working_bytes < full_allocations.peak_working_bytes);
    assert_eq!(full.source_fingerprint(), ranked.source_fingerprint());
    assert_ne!(full.fingerprint(), ranked.fingerprint());
    assert!(full.documents().eq(ranked.documents()));
    assert!(full.partitions().eq(ranked.partitions()));
    let mut answers = Vec::new();
    for index in [full, ranked] {
        let mut relations = PropertyFunctionRegistry::new();
        relations.register(
            "https://example.org/search".to_owned(),
            Arc::new(TextSearchRelation::new(Arc::new(index.clone()))),
        );
        answers.push(sparql_fixture::answer(
            &dataset,
            &relations,
            "SELECT ?doc ?score ?rank WHERE { ?doc <https://example.org/search> (\"common searchable\" ?score ?rank ?lang ?matched) } ORDER BY ?rank",
        ));
    }
    assert_eq!(answers[0], answers[1]);
    assert_eq!(answers[0].len(), 256);
    for (partition, _) in full.partitions() {
        for query in ["common searchable", "document42", "missing", "!!!"] {
            let needle = full.query_terms(query).expect("query analysis");
            assert_eq!(
                rank_partition(full, partition, &needle, None).expect("full scores"),
                rank_partition(ranked, partition, &needle, None).expect("ranked scores")
            );
        }
    }
    for index in [full, ranked] {
        let reranked = index
            .clone()
            .with_ranking_profile(RankingProfile::single_field().with_field_populations())
            .expect("new ranking profile");
        assert_eq!(
            reranked.surface_index().is_some(),
            index.surface_index().is_some()
        );
        assert_ne!(reranked.fingerprint(), index.fingerprint());
        if let Some(surface) = reranked.surface_index() {
            assert_eq!(
                surface.substring("substring42").expect("substring")[0].evidence[0].generation,
                reranked.fingerprint()
            );
        }
    }
    let full_han = HanCharacterIndex::from_dataset(&*dataset, &config).expect("Han");
    let ranked_han =
        HanCharacterIndex::from_dataset(&*dataset, &config.clone().with_surface_index(false))
            .expect("Han without surface");
    assert!(ranked_han.index().surface_index().is_none());
    for (partition, _) in full_han.index().partitions() {
        let needle = full_han.index().query_terms("北京大学").expect("Han query");
        assert_eq!(
            rank_partition(full_han.index(), partition, &needle, None).expect("full Han"),
            rank_partition(ranked_han.index(), partition, &needle, None).expect("ranked Han")
        );
    }
    assert_eq!(ranked_han.evidence(0, "北京").len(), 1);
    let empty = RdfDatasetBuilder::new().freeze().expect("empty corpus");
    let enabled = TextIndex::from_dataset(&*empty, &config).expect("empty full index");
    let absent = TextIndex::from_dataset(&*empty, &config.with_surface_index(false))
        .expect("empty ranked index");
    assert!(enabled.surface_index().is_some());
    assert!(absent.surface_index().is_none());
    assert_ne!(enabled.fingerprint(), absent.fingerprint());
}

fn emoji_measurement(count: usize) -> Measurement {
    let input = format!("A{}", "😀".repeat(count));
    let window = CurrentThreadWindow::open();
    let analysis = Analyzer::empty_lexicon().projections(&input).unwrap();
    let measurement = window.close();
    assert_eq!(analysis.surface.len(), count + 1);
    assert_eq!(analysis.normalized.text, input.to_lowercase());
    measurement
}

#[test]
fn repeated_emoji_metadata_grows_linearly() {
    let mut previous = emoji_measurement(256);
    for count in [512, 1_024, 2_048, 4_096] {
        let current = emoji_measurement(count);
        // Doubling a linear input must remain well below the fourfold traffic
        // of per-atom exact reallocations; include every persisted projection.
        assert!(
            current.requested_bytes <= previous.requested_bytes * 3,
            "emoji count {count}: {previous:?} -> {current:?}"
        );
        previous = current;
    }
}

#[test]
fn incomplete_tag_runs_and_lexical_cleanup_use_linear_storage() {
    for pattern in ["☕\u{200c} ", "👩\u{e0020} "] {
        let mut previous: Option<Measurement> = None;
        for count in [256, 512, 1_024, 2_048, 4_096] {
            let input = pattern.repeat(count);
            let window = CurrentThreadWindow::open();
            let analysis = Analyzer::empty_lexicon().projections(&input).unwrap();
            let current = window.close();
            assert_eq!(analysis.lexical.len(), count);
            assert_eq!(analysis.surface.len(), count);
            assert_eq!(analysis.spans.len(), count);
            assert!(
                analysis
                    .lexical
                    .iter()
                    .all(|projection| projection.text.chars().count() == 1)
            );
            if let Some(previous) = previous {
                assert!(
                    current.requested_bytes <= previous.requested_bytes * 3,
                    "{pattern:?}, count {count}: {previous:?} -> {current:?}"
                );
            }
            previous = Some(current);
        }
    }
    // A single unfinished tag suffix must not allocate one protection record
    // per tag, or repeatedly copy the remaining suffix at each scalar.
    for count in [1_024, 16_384, 262_144] {
        let input = "👩".to_owned() + &"\u{e0020}".repeat(count);
        let window = CurrentThreadWindow::open();
        let analysis = Analyzer::empty_lexicon().projections(&input).unwrap();
        let measurement = window.close();
        assert_eq!(analysis.normalized.text, "👩");
        assert!(
            measurement.requested_bytes < 16_384,
            "{count}: {measurement:?}"
        );
    }
}
