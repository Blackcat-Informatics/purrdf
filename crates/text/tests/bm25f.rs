// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact fielded arithmetic, refusal boundaries, and re-ranking retained facts.

use purrdf_core::{RdfDatasetBuilder, RdfLiteral, TermValue};
use purrdf_text::{
    B, DOCUMENTS_MAX, FIELD_LENGTH_MAX, FIELD_WEIGHT_MAX, FieldInput, Fixed, GraphSelector, K1,
    MAX_FIELDS, PartitionKey, PreparedCorpus, QUERY_TERMS_MAX, RankingField, RankingProfile,
    SCORE_BITS, SCORE_MAX, TextIndex, TextIndexConfig, explain, rank_partition,
};

fn field(name: &str, weight: Fixed, b: Fixed) -> RankingField {
    RankingField::new(name, weight, b).expect("valid field")
}

#[path = "support/bm25f_reference.rs"]
mod bm25f_reference;

#[test]
fn independent_integer_reference_corpus_matches_every_raw_unit() {
    bm25f_reference::verify_reference_corpus();
}

#[test]
fn score_bound_is_exact_and_width_is_derived() {
    let profile = RankingProfile::single_field();
    assert_eq!(SCORE_BITS, 56);
    assert_eq!(
        profile.validate_score(SCORE_MAX).expect("inclusive"),
        SCORE_MAX
    );
    let too_large = Fixed::from_raw(SCORE_MAX.into_raw() + 1);
    assert!(too_large.into_raw().unsigned_abs() < (1_u128 << SCORE_BITS));
    assert!(profile.validate_score(too_large).is_err());
    assert!(profile.validate_score(Fixed::from_raw(-1)).is_err());
    // For N <= 2^40, IDF's argument is <= 2N+2. Its integer ln is a
    // lower approximation, and ln(2^41+2) < 29. Saturation is <= 2.2,
    // including each truncation. This coarse bound has no rounding ambiguity.
    let conservative = Fixed::from_integer(QUERY_TERMS_MAX as i64)
        .expect("count")
        .checked_mul(Fixed::from_integer(29).expect("integer"))
        .expect("product")
        .checked_mul(K1.checked_add(Fixed::ONE).expect("sum"))
        .expect("product");
    assert!(conservative < SCORE_MAX);
    let actual_idf_ceiling = Fixed::from_integer(2 * DOCUMENTS_MAX as i64 + 2)
        .expect("argument")
        .ln()
        .expect("ln");
    assert!(actual_idf_ceiling < Fixed::from_integer(29).expect("integer"));
    let scale = 1_000_000_000_000_i128;
    let minimum_normalization = scale / i128::from(FIELD_LENGTH_MAX);
    let pseudo_ceiling = i128::from(FIELD_LENGTH_MAX) * scale * scale / minimum_normalization;
    let weighted_ceiling = Fixed::from_raw(pseudo_ceiling)
        .checked_mul(FIELD_WEIGHT_MAX)
        .expect("bounded field pseudo-frequency");
    let sum_ceiling = weighted_ceiling
        .checked_mul(Fixed::from_integer(MAX_FIELDS as i64).expect("count"))
        .expect("bounded field sum");
    let numerator_ceiling = sum_ceiling
        .checked_mul(K1.checked_add(Fixed::ONE).expect("sum"))
        .expect("bounded saturation numerator");
    assert!(numerator_ceiling.into_raw() < i128::MAX);
}

#[test]
fn profiles_refuse_bad_fields_and_incomplete_or_ambiguous_mapping() {
    assert!(RankingField::new("", Fixed::ONE, B).is_err());
    assert!(RankingField::new("x", Fixed::from_raw(-1), B).is_err());
    assert!(
        RankingField::new(
            "x",
            FIELD_WEIGHT_MAX
                .checked_add(Fixed::from_raw(1))
                .expect("sum"),
            B
        )
        .is_err()
    );
    assert!(RankingField::new("x", Fixed::ONE, Fixed::from_raw(-1)).is_err());
    assert!(
        RankingField::new(
            "x",
            Fixed::ONE,
            Fixed::ONE.checked_add(Fixed::from_raw(1)).expect("sum")
        )
        .is_err()
    );
    assert!(RankingProfile::new(Vec::new(), Vec::new(), None).is_err());
    assert!(
        RankingProfile::new(
            (0..=MAX_FIELDS)
                .map(|i| field(&i.to_string(), Fixed::ONE, B))
                .collect(),
            Vec::new(),
            None
        )
        .is_err()
    );
    assert!(
        RankingProfile::new(
            vec![field("x", Fixed::ONE, B), field("x", Fixed::ONE, B)],
            Vec::new(),
            None
        )
        .is_err()
    );
    let iri = TermValue::iri("https://example.org/text");
    assert!(
        RankingProfile::new(
            vec![field("x", Fixed::ONE, B)],
            vec![(iri.clone(), 0), (iri.clone(), 0)],
            None
        )
        .is_err()
    );
    assert!(
        RankingProfile::new(
            vec![field("x", Fixed::ONE, B)],
            vec![(iri.clone(), 1)],
            None
        )
        .is_err()
    );
    let profile = RankingProfile::new(vec![field("x", Fixed::ONE, B)], Vec::new(), None)
        .expect("closed routing");
    assert!(profile.field_for(&iri).is_err());
    assert!(
        RankingProfile::single_field()
            .field_for(&TermValue::simple_literal("not an IRI"))
            .is_err()
    );
}

#[test]
fn invalid_inputs_fail_before_zero_contribution_shortcuts() {
    let profile = RankingProfile::single_field();
    assert!(PreparedCorpus::new(&profile, DOCUMENTS_MAX + 1, &[0]).is_err());
    assert!(PreparedCorpus::new(&profile, 0, &[1]).is_err());
    assert!(PreparedCorpus::new(&profile, 1, &[]).is_err());
    assert!(PreparedCorpus::new(&profile, 1, &[u128::from(FIELD_LENGTH_MAX) + 1]).is_err());
    let corpus = PreparedCorpus::new(&profile, 2, &[10]).expect("corpus");
    assert!(corpus.prepare_query(&[("a", 3)]).is_err());
    assert!(
        corpus
            .prepare_query(&vec![("a", 0); QUERY_TERMS_MAX + 1])
            .is_err()
    );
    assert!(corpus.prepare_query(&[("a", 1), ("a", 1)]).is_err());
    assert!(corpus.prepare_query(&[("b", 1), ("a", 1)]).is_err());
    assert!(corpus.prepare_query(&[("", 1)]).is_err());
    let query = corpus.prepare_query(&[("a", 0)]).expect("absent term");
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
    assert!(
        query
            .contribution(
                0,
                &[FieldInput {
                    term_frequency: 1,
                    length: 1
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
                    length: 11
                }]
            )
            .is_err()
    );
    assert!(query.contribution(0, &[]).is_err());
    assert!(query.contribution(1, &[FieldInput::default()]).is_err());
    assert!(query.score(&[]).is_err());
    let empty = PreparedCorpus::new(&profile, 0, &[0]).expect("empty corpus");
    assert!(
        empty
            .prepare_query(&[])
            .expect("empty query")
            .score(&[])
            .is_err()
    );
    assert!(empty.prepare_query(&[("a", 1)]).is_err());
    let query = corpus
        .prepare_query(&[("a", 1), ("b", 1)])
        .expect("two terms");
    assert!(
        query
            .score(&[
                vec![FieldInput {
                    term_frequency: 1,
                    length: 2
                }],
                vec![FieldInput {
                    term_frequency: 1,
                    length: 3
                }]
            ])
            .is_err()
    );
    assert!(
        query
            .score(&[
                vec![FieldInput {
                    term_frequency: 2,
                    length: 3
                }],
                vec![FieldInput {
                    term_frequency: 2,
                    length: 3
                }]
            ])
            .is_err()
    );
}

fn fixture() -> TextIndex {
    let mut builder = RdfDatasetBuilder::new();
    let title = builder.intern_iri("https://example.org/title");
    let body = builder.intern_iri("https://example.org/body");
    for (subject, head, text) in [("a", "cat", "dog dog dog"), ("b", "dog", "cat cat cat")] {
        let subject = builder.intern_iri(&format!("https://example.org/{subject}"));
        let head = builder.intern_literal(RdfLiteral::simple(head));
        let text = builder.intern_literal(RdfLiteral::simple(text));
        builder.push_quad(subject, title, head, None);
        builder.push_quad(subject, body, text, None);
    }
    let dataset = builder.freeze().expect("dataset");
    TextIndex::from_dataset(
        &*dataset,
        &TextIndexConfig::new(
            vec![
                TermValue::iri("https://example.org/title"),
                TermValue::iri("https://example.org/body"),
            ],
            GraphSelector::Any,
        )
        .expect("config"),
    )
    .expect("index")
}

#[test]
fn reweight_and_remap_reuse_predicate_facts_but_change_ranking_identity() {
    let original = fixture();
    let partition = PartitionKey::new(None, None);
    let needle = vec!["cat".to_owned()];
    let old = rank_partition(&original, &partition, &needle, None).expect("ranking");
    assert_eq!(old[0].document, 1);
    let fields = vec![
        field("title", Fixed::from_integer(8).expect("weight"), B),
        field("body", Fixed::ONE, B),
    ];
    let mappings = vec![
        (TermValue::iri("https://example.org/title"), 0),
        (TermValue::iri("https://example.org/body"), 1),
    ];
    let profile = RankingProfile::new(fields.clone(), mappings.clone(), None).expect("profile");
    let mut reversed = mappings;
    reversed.reverse();
    assert_eq!(
        profile.fingerprint(),
        RankingProfile::new(fields, reversed, None)
            .expect("permuted")
            .fingerprint()
    );
    let reranked = original
        .clone()
        .with_ranking_profile(profile)
        .expect("reweight");
    assert!(original.documents().eq(reranked.documents()));
    for term in original.terms() {
        assert_eq!(
            original.postings(&partition, term).collect::<Vec<_>>(),
            reranked.postings(&partition, term).collect::<Vec<_>>()
        );
    }
    assert_eq!(
        original.analyzer_fingerprint(),
        reranked.analyzer_fingerprint()
    );
    assert_eq!(original.source_fingerprint(), reranked.source_fingerprint());
    assert_ne!(original.fingerprint(), reranked.fingerprint());
    for document in 0..2 {
        assert_eq!(
            original.predicate_lengths(document),
            reranked.predicate_lengths(document)
        );
        assert_eq!(
            original.term_frequency(document, "cat"),
            reranked.term_frequency(document, "cat")
        );
    }
    assert_eq!(
        reranked.field_totals(&partition),
        Some([2_u128, 6].as_slice())
    );
    let new = rank_partition(&reranked, &partition, &needle, None).expect("ranking");
    assert_eq!(new[0].document, 0);
    assert_eq!(
        rank_partition(&reranked, &partition, &needle, Some(1)).expect("bounded"),
        new[..1]
    );
    for row in &new {
        let contributions = explain(&reranked, row.document, &needle).expect("explain");
        assert_eq!(contributions[0].contribution, row.score);
    }
    let incomplete = RankingProfile::new(vec![field("closed", Fixed::ONE, B)], Vec::new(), None)
        .expect("profile");
    assert!(original.with_ranking_profile(incomplete).is_err());
}

#[test]
fn zero_weight_matching_rows_keep_the_canonical_tie_order() {
    let profile = RankingProfile::new(vec![field("muted", Fixed::ZERO, B)], Vec::new(), Some(0))
        .expect("profile");
    let index = fixture().with_ranking_profile(profile).expect("rerank");
    let rows = rank_partition(
        &index,
        &PartitionKey::new(None, None),
        &["cat".to_owned()],
        None,
    )
    .expect("ranking");
    assert_eq!(
        rows.iter()
            .map(|row| (row.document, row.score, row.matched))
            .collect::<Vec<_>>(),
        vec![(0, Fixed::ZERO, 1), (1, Fixed::ZERO, 1)]
    );
}

#[test]
fn canonical_ranking_identity_binds_the_index_corpus_construction_law() {
    use purrdf_text::{INDEX_CORPUS_PROFILE_ID, RANKING_PROFILE_ID};
    let bytes = RankingProfile::single_field().canonical_description();
    let first_end = 8 + RANKING_PROFILE_ID.len();
    let length_bytes: [u8; 8] = bytes[first_end..first_end + 8]
        .try_into()
        .expect("framed length");
    assert_eq!(
        u64::from_le_bytes(length_bytes),
        INDEX_CORPUS_PROFILE_ID.len() as u64
    );
    assert_eq!(
        &bytes[first_end + 8..first_end + 8 + INDEX_CORPUS_PROFILE_ID.len()],
        INDEX_CORPUS_PROFILE_ID.as_bytes()
    );
    assert_eq!(
        INDEX_CORPUS_PROFILE_ID,
        "purrdf-text-corpus-graph-language-v1"
    );
}
