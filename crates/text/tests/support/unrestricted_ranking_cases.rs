// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Complete oversized ranking behaviors shared by native and portable callers.

use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral, TermValue};
use purrdf_text::{
    Analyzer, B, FieldInput, Fixed, GraphSelector, PartitionKey, PreparedCorpus, RankingField,
    RankingProfile, TextIndex, TextIndexConfig, explain, rank_partition,
};
use std::sync::Arc;

#[path = "ranking_oracle.rs"]
pub(super) mod ranking_oracle;

pub(super) const NOTE: &str = "https://example.org/note";

fn names() -> Vec<String> {
    (0..5_000).map(|at| format!("q{at:04}")).collect()
}

pub(super) fn configuration(predicates: Vec<TermValue>) -> TextIndexConfig {
    TextIndexConfig::new(predicates, GraphSelector::Any, Analyzer::empty_lexicon())
        .expect("configuration")
}

pub(super) fn input(frequency: u64, length: u64) -> FieldInput {
    FieldInput {
        term_frequency: frequency,
        length,
    }
}

/// The full needle reaches the actual index, ranking, heap and explanation laws.
/// Return the same original dataset/index for the public property-function test.
pub(super) fn five_thousand_term_index() -> (Arc<RdfDataset>, Arc<TextIndex>, String, Vec<i128>) {
    let names = names();
    let needle = names.join(" ");
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri(NOTE);
    for (subject, text) in [("a", needle.clone()), ("b", names[..2_500].join(" "))] {
        let subject = builder.intern_iri(&format!("https://example.org/{subject}"));
        let object = builder.intern_literal(RdfLiteral::simple(&text));
        builder.push_quad(subject, predicate, object, None);
    }
    let dataset = builder.freeze().expect("dataset");
    let index = Arc::new(
        TextIndex::from_dataset(&*dataset, &configuration(vec![TermValue::iri(NOTE)]))
            .expect("index"),
    );
    let terms = index.analyzer().terms(&needle).expect("analyzed needle");
    assert_eq!(terms.len(), 5_000);
    let parameters = [(Fixed::ONE.into_raw(), B.into_raw())];
    let contribution = |frequency, length| {
        ranking_oracle::contribution(
            2,
            frequency,
            &[2],
            &[7_500],
            &parameters,
            &[input(1, length)],
        )
    };
    let expected = vec![
        2_500 * (contribution(2, 5_000) + contribution(1, 5_000)),
        2_500 * contribution(2, 2_500),
    ];
    // Frozen by the unchanged Python reference's unbounded score function.
    assert_eq!(expected, [1_926_031_222_170_000, 527_772_927_557_500]);
    let partition = PartitionKey::new(None, None);
    let rows = rank_partition(&index, &partition, &terms, None).expect("complete needle ranks");
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows.iter()
            .map(|row| (row.document, row.score.into_raw(), row.matched))
            .collect::<Vec<_>>(),
        vec![(0, expected[0], 5_000), (1, expected[1], 2_500)]
    );
    for row in &rows {
        assert_eq!(
            row.score_bound.profile_fingerprint(),
            index.ranking_profile().fingerprint()
        );
        assert!(row.score <= row.score_bound.maximum());
    }
    assert_eq!(
        rank_partition(&index, &partition, &terms, Some(1)).expect("heap"),
        rows[..1]
    );
    let explained = explain(&index, 0, &terms).expect("all terms explained");
    assert_eq!(explained.len(), 5_000);
    assert_eq!(
        explained
            .iter()
            .map(|term| term.contribution.into_raw())
            .sum::<i128>(),
        expected[0]
    );
    (dataset, index, needle, expected)
}

/// The actual query's score and certificate both exceed the former global cap.
pub(super) fn declared_large_corpora_are_exact() {
    let profile = RankingProfile::single_field();
    let names = names();
    let terms: Vec<_> = names.iter().map(|name| (name.as_str(), 1)).collect();
    for documents in [1_u64 << 41, u64::MAX] {
        let total = u128::from(documents) * 5_000;
        let corpus =
            PreparedCorpus::new(&profile, documents, &[total]).expect("full u64 population");
        let query = corpus.prepare_query(&terms).expect("all prepared terms");
        let inputs = vec![vec![input(1, 5_000)]; names.len()];
        let expected = 5_000
            * ranking_oracle::contribution(
                documents,
                1,
                &[documents],
                &[total],
                &[(Fixed::ONE.into_raw(), B.into_raw())],
                &[input(1, 5_000)],
            );
        let result = query.score(&inputs).expect("large declared corpus scores");
        assert_eq!(result.value.into_raw(), expected);
        assert_eq!(
            expected,
            if documents == 1 << 41 {
                140_067_846_474_250_000
            } else {
                219_779_772_238_640_000
            }
        );
        assert_eq!(result.bound, query.score_bound());
        assert!(result.value.into_raw() > 65_536 * Fixed::ONE.into_raw());
        assert!(result.bound.bits() > 56);
        assert!(result.value <= result.bound.maximum());
    }
}

/// Unrestricted fields and weights exercise the existing heap-backed Integer,
/// while exact corpus totals summed across fields also exceed u128.
pub(super) fn promoted_field_arithmetic_is_exact() {
    let fields = (0..32)
        .map(|at| {
            RankingField::new(
                format!("field-{at}"),
                Fixed::from_raw(i128::MAX),
                Fixed::ONE,
            )
            .expect("nonnegative representable weight")
        })
        .collect();
    let profile = RankingProfile::new(fields, Vec::new(), Some(0)).expect("unrestricted fields");
    let documents = u64::MAX;
    let total = u128::from(documents) * u128::from(u64::MAX);
    let totals = vec![total; profile.fields().len()];
    let corpus = PreparedCorpus::new(&profile, documents, &totals)
        .expect("exact totals past aggregate u128");
    let query = corpus.prepare_query(&[("needle", 1)]).expect("query");
    let inputs = vec![input(u64::MAX, u64::MAX); profile.fields().len()];
    let expected = ranking_oracle::contribution(
        documents,
        1,
        &vec![documents; totals.len()],
        &totals,
        &vec![(i128::MAX, Fixed::ONE.into_raw()); totals.len()],
        &inputs,
    );
    let result = query
        .score(&[inputs])
        .expect("promoted products and saturation");
    assert_eq!(result.value.into_raw(), expected);
    assert_eq!(expected, 96_703_099_784_957, "unchanged Python reference");
    assert!(result.value <= result.bound.maximum());

    // A genuinely undefined zero normalization is still a typed domain error:
    // one tiny carrier plus one maximum-length carrier is consistent data.
    let profile = RankingProfile::new(
        vec![RankingField::new("zero-normalization", Fixed::ONE, Fixed::ONE).expect("field")],
        Vec::new(),
        Some(0),
    )
    .expect("profile");
    let corpus =
        PreparedCorpus::new(&profile, 2, &[u128::from(u64::MAX) + 1]).expect("consistent corpus");
    let query = corpus.prepare_query(&[("needle", 1)]).expect("query");
    assert!(matches!(
        query.contribution(0, &[input(1, 1)]),
        Err(purrdf_text::TextError::Domain(_))
    ));

    // The first term fits the field; the second makes the distinct-term sum
    // impossible. It must fail as Data rather than wrap a u64 frequency.
    let profile = RankingProfile::single_field();
    let corpus = PreparedCorpus::new(&profile, 2, &[2 * u128::from(u64::MAX)]).expect("corpus");
    let query = corpus
        .prepare_query(&[("a", 1), ("b", 1)])
        .expect("distinct terms");
    assert!(matches!(
        query.score(&[
            vec![input(u64::MAX, u64::MAX)],
            vec![input(u64::MAX, u64::MAX)]
        ]),
        Err(purrdf_text::TextError::Data(_))
    ));
}

/// Routing and retained field buffers have no sixteen-field ceiling.
pub(super) fn thirty_two_index_fields_are_exact() {
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri("https://example.org/document");
    let object = builder.intern_literal(RdfLiteral::simple("needle needle"));
    let mut predicates = Vec::new();
    let mut fields = Vec::new();
    for at in 0..32 {
        let iri = format!("https://example.org/field-{at:02}");
        let predicate = builder.intern_iri(&iri);
        builder.push_quad(subject, predicate, object, None);
        predicates.push(TermValue::iri(&iri));
        fields.push(
            RankingField::new(
                format!("field-{at}"),
                Fixed::from_raw(i128::from(at + 1) * Fixed::ONE.into_raw()),
                B,
            )
            .expect("field"),
        );
    }
    let mappings = predicates
        .iter()
        .cloned()
        .enumerate()
        .map(|(at, predicate)| (predicate, at))
        .collect();
    let profile = RankingProfile::new(fields, mappings, None).expect("all routed fields");
    let dataset = builder.freeze().expect("dataset");
    let index =
        TextIndex::from_dataset_with_ranking(&*dataset, &configuration(predicates), profile)
            .expect("wide field index");
    let partition = PartitionKey::new(None, None);
    assert_eq!(index.field_lengths(0), Some([2; 32].as_slice()));
    let parameters: Vec<_> = index
        .ranking_profile()
        .fields()
        .iter()
        .map(|field| (field.weight().into_raw(), field.b().into_raw()))
        .collect();
    let expected =
        ranking_oracle::contribution(1, 1, &[1; 32], &[2; 32], &parameters, &[input(2, 2); 32]);
    let rows = rank_partition(&index, &partition, &["needle".to_owned()], None).expect("rank");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].score.into_raw(), expected);
    assert_eq!(
        explain(&index, 0, &["needle".to_owned()]).expect("explain")[0].contribution,
        rows[0].score
    );
}
