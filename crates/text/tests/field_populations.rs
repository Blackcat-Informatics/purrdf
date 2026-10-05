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
}
