// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Carrier populations: exact independent arithmetic and validation boundaries.

use purrdf_testkit::exact::Natural;
use purrdf_text::{
    B, DOCUMENTS_MAX, FIELD_LENGTH_MAX, FIELD_WEIGHT_MAX, FieldInput, Fixed, PreparedCorpus,
    RankingField, RankingProfile,
};

const SCALE: u128 = 1_000_000_000_000;

/// Schoolbook unbounded arithmetic from testkit, independent of Fixed/wide.
fn product_quotient(left: u128, right: u128, divisor: u128) -> u128 {
    Natural::from_u128(left)
        .mul(&Natural::from_u128(right))
        .div_rem(&Natural::from_u128(divisor))
        .0
        .to_u128()
        .expect("the specified rounded value fits u128")
}

/// The published integer series, without calling the production logarithm.
fn reference_log(mut raw: u128) -> u128 {
    let mut exponent = 0;
    while raw >= (2 * SCALE) << exponent {
        // Reduce without rounding until after the exponent is known.
        exponent += 1;
    }
    raw = product_quotient(raw, 1_000_000, 1 << exponent);
    let z = product_quotient(
        raw - 1_000_000_000_000_000_000,
        1_000_000_000_000_000_000,
        raw + 1_000_000_000_000_000_000,
    );
    let square = product_quotient(z, z, 1_000_000_000_000_000_000);
    let mut power = z;
    let mut series = 0;
    for denominator in (1..40).step_by(2) {
        series += power / denominator;
        power = product_quotient(power, square, 1_000_000_000_000_000_000);
    }
    (exponent * 693_147_180_559_945_309 + 2 * series) / 1_000_000
}

fn reference_score(
    documents: u64,
    frequency: u64,
    populations: &[u64],
    totals: &[u128],
    profile: &RankingProfile,
    inputs: &[FieldInput],
) -> i128 {
    let idf = reference_log(
        SCALE
            + product_quotient(
                u128::from(2 * (documents - frequency) + 1),
                SCALE,
                u128::from(2 * frequency + 1),
            ),
    );
    let mut pseudo = 0;
    for (((input, field), &population), &total) in inputs
        .iter()
        .zip(profile.fields())
        .zip(populations)
        .zip(totals)
    {
        if input.term_frequency != 0 {
            let relative = product_quotient(
                u128::from(input.length) * u128::from(population),
                SCALE,
                total,
            );
            let b = field.b().into_raw().unsigned_abs();
            let normalization = SCALE - b + product_quotient(b, relative, SCALE);
            let normalized = product_quotient(
                u128::from(input.term_frequency) * SCALE,
                SCALE,
                normalization,
            );
            pseudo += product_quotient(normalized, field.weight().into_raw().unsigned_abs(), SCALE);
        }
    }
    let k1 = 12 * SCALE / 10;
    let saturated = product_quotient(
        product_quotient(pseudo, k1 + SCALE, SCALE),
        SCALE,
        pseudo + k1,
    );
    i128::try_from(product_quotient(idf, saturated, SCALE)).expect("bounded score")
}

fn profile() -> RankingProfile {
    RankingProfile::new(
        vec![
            RankingField::new("title", Fixed::from_raw(333_333_333_333), B).expect("field"),
            RankingField::new("body", FIELD_WEIGHT_MAX, Fixed::ONE).expect("field"),
            RankingField::new("unused", Fixed::ZERO, B).expect("field"),
        ],
        Vec::new(),
        Some(0),
    )
    .expect("profile")
}

#[test]
fn dense_populations_preserve_every_raw_score_and_legacy_identity() {
    let dense = profile();
    let sparse = dense.clone().with_field_populations();
    assert!(!dense.uses_field_populations());
    assert!(sparse.uses_field_populations());
    assert_eq!(sparse.clone().with_field_populations(), sparse);
    assert_ne!(dense.fingerprint(), sparse.fingerprint());
    let totals = [19, 100, 0];
    let corpus = PreparedCorpus::new(&dense, 8, &totals).expect("dense");
    let carriers =
        PreparedCorpus::with_field_populations(&sparse, 8, &totals, &[8; 3]).expect("carriers");
    let query = corpus.prepare_query(&[("cat", 3)]).expect("query");
    let sparse_query = carriers.prepare_query(&[("cat", 3)]).expect("query");
    for length in 1..=19 {
        let inputs = [
            FieldInput {
                term_frequency: 1,
                length,
            },
            FieldInput {
                term_frequency: 2,
                length: 4,
            },
            FieldInput::default(),
        ];
        assert_eq!(
            query.contribution(0, &inputs).expect("score"),
            sparse_query.contribution(0, &inputs).expect("score")
        );
    }
    assert!(PreparedCorpus::new(&sparse, 8, &totals).is_err());
    assert!(PreparedCorpus::with_field_populations(&dense, 8, &totals, &[8; 3]).is_err());
}

#[test]
fn sparse_field_means_match_the_unbounded_reference() {
    let profile = profile().with_field_populations();
    for documents in [3, 7, 23, DOCUMENTS_MAX] {
        for population in [1, 2, documents] {
            let populations = [population, documents, 0];
            let totals = [
                u128::from(population) * 7,
                u128::from(documents) * u128::from(FIELD_LENGTH_MAX),
                0,
            ];
            let corpus =
                PreparedCorpus::with_field_populations(&profile, documents, &totals, &populations)
                    .expect("sparse corpus");
            let query = corpus.prepare_query(&[("cat", 1)]).expect("query");
            for frequency in [0, 1, 7] {
                let inputs = [
                    FieldInput {
                        term_frequency: frequency,
                        length: 7,
                    },
                    FieldInput {
                        term_frequency: FIELD_LENGTH_MAX,
                        length: FIELD_LENGTH_MAX,
                    },
                    FieldInput::default(),
                ];
                assert_eq!(
                    query.contribution(0, &inputs).expect("score").into_raw(),
                    reference_score(documents, 1, &populations, &totals, &profile, &inputs)
                );
            }
        }
    }
}

#[test]
fn populations_and_totals_are_checked_before_scoring() {
    let profile = profile().with_field_populations();
    for (documents, totals, populations) in [
        (DOCUMENTS_MAX + 1, vec![0; 3], vec![0; 3]),
        (2, vec![1, 0, 0], vec![0; 3]),
        (2, vec![0; 3], vec![3, 0, 0]),
        (
            2,
            vec![u128::from(FIELD_LENGTH_MAX) + 1, 0, 0],
            vec![1, 0, 0],
        ),
        (0, vec![1, 0, 0], vec![0; 3]),
        (2, vec![0; 3], vec![0; 2]),
        (2, vec![0; 2], vec![0; 3]),
    ] {
        assert!(
            PreparedCorpus::with_field_populations(&profile, documents, &totals, &populations)
                .is_err()
        );
    }
    let empty_fields = PreparedCorpus::with_field_populations(&profile, 2, &[0; 3], &[2, 0, 0])
        .expect("zero totals");
    assert_eq!(
        empty_fields
            .prepare_query(&[])
            .expect("empty query")
            .score(&[])
            .expect("zero score"),
        Fixed::ZERO
    );
    let corpus = PreparedCorpus::with_field_populations(
        &profile,
        2,
        &[u128::from(FIELD_LENGTH_MAX), 0, 0],
        &[1, 0, 0],
    )
    .expect("one carrier");
    let query = corpus.prepare_query(&[("cat", 1)]).expect("query");
    // A document carrying this field leaves no other carrier to hold the total.
    assert!(
        query
            .contribution(
                0,
                &[
                    FieldInput {
                        term_frequency: 0,
                        length: 1
                    },
                    FieldInput::default(),
                    FieldInput::default()
                ]
            )
            .is_err()
    );
    // A document missing the field does not consume its one declared carrier.
    assert_eq!(
        query
            .contribution(0, &[FieldInput::default(); 3])
            .expect("missing field"),
        Fixed::ZERO
    );
    let all_carriers = PreparedCorpus::with_field_populations(
        &profile,
        2,
        &[2 * u128::from(FIELD_LENGTH_MAX), 0, 0],
        &[2, 0, 0],
    )
    .expect("every document carries the first field");
    // A zero-length row cannot fit when both carriers must have maximum length.
    assert!(
        all_carriers
            .prepare_query(&[("cat", 1)])
            .expect("query")
            .contribution(0, &[FieldInput::default(); 3])
            .is_err()
    );
}

#[test]
fn sparse_shortest_normalization_and_bounds_match_the_reference() {
    let profile = RankingProfile::new(
        vec![RankingField::new("text", FIELD_WEIGHT_MAX, Fixed::ONE).expect("field")],
        Vec::new(),
        Some(0),
    )
    .expect("profile")
    .with_field_populations();
    let population = DOCUMENTS_MAX - 1;
    let total =
        u128::from(population) * u128::from(FIELD_LENGTH_MAX) - u128::from(FIELD_LENGTH_MAX - 1);
    let corpus =
        PreparedCorpus::with_field_populations(&profile, DOCUMENTS_MAX, &[total], &[population])
            .expect("bounded");
    let query = corpus.prepare_query(&[("cat", 1)]).expect("query");
    let input = [FieldInput {
        term_frequency: 1,
        length: 1,
    }];
    assert_eq!(
        query.contribution(0, &input).expect("score").into_raw(),
        reference_score(DOCUMENTS_MAX, 1, &[population], &[total], &profile, &input)
    );
    assert!(
        query
            .contribution(
                0,
                &[FieldInput {
                    term_frequency: 1,
                    length: 0
                }]
            )
            .is_err()
    );
    assert!(
        query
            .contribution(
                0,
                &[FieldInput {
                    term_frequency: 0,
                    length: FIELD_LENGTH_MAX + 1
                }]
            )
            .is_err()
    );
    assert!(corpus.prepare_query(&[("cat", DOCUMENTS_MAX + 1)]).is_err());
    let empty =
        PreparedCorpus::with_field_populations(&profile, 0, &[0], &[0]).expect("empty corpus");
    assert!(empty.prepare_query(&[]).expect("query").score(&[]).is_err());
}

#[test]
fn index_carriers_follow_partition_and_remapping_facts() {
    use purrdf_core::{RdfDatasetBuilder, RdfLiteral, TermValue};
    use purrdf_text::{
        GraphSelector, PartitionKey, TextIndex, TextIndexConfig, explain, rank_partition,
    };

    let mut builder = RdfDatasetBuilder::new();
    let title = builder.intern_iri("https://example.org/title");
    let body = builder.intern_iri("https://example.org/body");
    let graph = builder.intern_iri("https://example.org/graph");
    for (subject, predicate, text, named) in [
        ("a", title, "cat", false),
        ("b", body, "cat cat dog dog", false),
        ("c", body, "dog dog", false),
        ("d", title, "cat", true),
        ("d", body, "dog", true),
    ] {
        let subject = builder.intern_iri(&format!("https://example.org/{subject}"));
        let literal = builder.intern_literal(RdfLiteral::simple(text));
        builder.push_quad(subject, predicate, literal, named.then_some(graph));
    }
    let dataset = builder.freeze().expect("dataset");
    let config = TextIndexConfig::new(
        vec![
            TermValue::iri("https://example.org/title"),
            TermValue::iri("https://example.org/body"),
        ],
        GraphSelector::Any,
        purrdf_text::Analyzer::empty_lexicon(),
    )
    .expect("config");
    let dense_profile = RankingProfile::new(
        vec![
            RankingField::new("title", Fixed::ONE, B).expect("field"),
            RankingField::new("body", Fixed::ONE, B).expect("field"),
        ],
        vec![
            (TermValue::iri("https://example.org/title"), 0),
            (TermValue::iri("https://example.org/body"), 1),
        ],
        None,
    )
    .expect("profile");
    let dense = TextIndex::from_dataset_with_ranking(&*dataset, &config, dense_profile.clone())
        .expect("index");
    let profile = dense_profile.with_field_populations();
    let sparse = dense
        .clone()
        .with_ranking_profile(profile.clone())
        .expect("rerank");
    let direct =
        TextIndex::from_dataset_with_ranking(&*dataset, &config, profile.clone()).expect("direct");
    assert_eq!(direct.fingerprint(), sparse.fingerprint());
    assert_eq!(dense.source_fingerprint(), sparse.source_fingerprint());
    assert_eq!(dense.analyzer_fingerprint(), sparse.analyzer_fingerprint());
    assert_ne!(dense.fingerprint(), sparse.fingerprint());
    let partition = PartitionKey::new(None, None);
    assert_eq!(
        sparse.field_populations(&partition),
        Some([1, 2].as_slice())
    );
    assert_eq!(sparse.field_totals(&partition), Some([1, 6].as_slice()));
    let named = PartitionKey::new(Some(TermValue::iri("https://example.org/graph")), None);
    assert_eq!(sparse.field_populations(&named), Some([1, 1].as_slice()));
    let needle = ["cat".to_owned()];
    let rows = rank_partition(&sparse, &partition, &needle, None).expect("rank");
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rank_partition(&sparse, &partition, &needle, Some(1)).expect("ceiling"),
        rows[..1]
    );
    let mut changed_score = false;
    for row in &rows {
        let lengths = sparse.field_lengths(row.document).expect("lengths");
        let inputs = if lengths[0] != 0 {
            [
                FieldInput {
                    term_frequency: 1,
                    length: 1,
                },
                FieldInput::default(),
            ]
        } else {
            [
                FieldInput::default(),
                FieldInput {
                    term_frequency: 2,
                    length: 4,
                },
            ]
        };
        assert_eq!(
            row.score.into_raw(),
            reference_score(3, 2, &[1, 2], &[1, 6], &profile, &inputs)
        );
        assert_eq!(
            explain(&sparse, row.document, &needle).expect("explanation")[0].contribution,
            row.score
        );
        changed_score |=
            explain(&dense, row.document, &needle).expect("dense")[0].contribution != row.score;
    }
    assert!(changed_score, "sparse means must alter sparse scores");
    let merged = RankingProfile::single_field().with_field_populations();
    let merged = sparse.with_ranking_profile(merged).expect("remap");
    assert_eq!(merged.field_populations(&partition), Some([3].as_slice()));
    assert_eq!(merged.field_totals(&partition), Some([7].as_slice()));
}
