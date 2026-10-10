// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact fielded arithmetic, refusal boundaries, and re-ranking retained facts.

use purrdf_core::{RdfDatasetBuilder, RdfLiteral, TermValue};
use purrdf_text::{
    B, FieldInput, Fixed, GraphSelector, PartitionKey, PreparedCorpus, RankingField,
    RankingProfile, TextIndex, TextIndexConfig, explain, rank_partition,
};

fn field(name: &str, weight: Fixed, b: Fixed) -> RankingField {
    RankingField::new(name, weight, b).expect("valid field")
}

#[path = "support/bm25f_reference.rs"]
mod bm25f_reference;

#[test]
fn independent_integer_reference_corpus_matches_every_raw_unit() {
    bm25f_reference::verify_reference_corpus();
    bm25f_reference::verify_reference_corpus_with_population_mode(true);
}

#[test]
fn score_bound_is_exact_and_width_is_derived() {
    let profile = RankingProfile::single_field();
    let corpus = PreparedCorpus::new(&profile, 1, &[2]).expect("corpus");
    let query = corpus.prepare_query(&[("a", 1), ("b", 1)]).expect("query");
    let bound = query.score_bound();
    // Independent integer reference: two IDFs ln(4/3), each saturated below
    // 2.2. The raw ceiling is 2 * floor(287682072451 * 2.2).
    assert_eq!(bound.maximum().into_raw(), 1_265_801_118_784);
    assert_eq!(bound.bits(), 41);
    assert_eq!(bound.profile_fingerprint(), profile.fingerprint());
    assert_eq!(
        bound.validate(bound.maximum()).expect("inclusive"),
        bound.maximum()
    );
    let too_large = Fixed::from_raw(bound.maximum().into_raw() + 1);
    assert!(too_large.into_raw().unsigned_abs() < (1_u128 << bound.bits()));
    assert!(bound.validate(too_large).is_err());
    assert!(bound.validate(Fixed::from_raw(-1)).is_err());
    let empty = corpus.prepare_query(&[]).expect("empty query");
    assert_eq!(empty.score_bound().maximum(), Fixed::ZERO);
    assert_eq!(empty.score_bound().bits(), 0);
    assert_eq!(
        empty.score(&[]).expect("empty sum").bound,
        empty.score_bound()
    );
}

#[test]
fn profiles_refuse_bad_fields_and_incomplete_or_ambiguous_mapping() {
    assert!(RankingField::new("", Fixed::ONE, B).is_err());
    assert!(RankingField::new("x", Fixed::from_raw(-1), B).is_err());
    assert!(RankingField::new("x", Fixed::from_raw(i128::MAX), B).is_ok());
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
            (0..32)
                .map(|i| field(&i.to_string(), Fixed::ONE, B))
                .collect(),
            Vec::new(),
            None
        )
        .is_ok()
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
    assert!(PreparedCorpus::new(&profile, u64::MAX, &[0]).is_ok());
    assert!(PreparedCorpus::new(&profile, 0, &[1]).is_err());
    assert!(PreparedCorpus::new(&profile, 1, &[]).is_err());
    assert!(PreparedCorpus::new(&profile, 1, &[u128::from(u64::MAX) + 1]).is_err());
    let corpus = PreparedCorpus::new(&profile, 2, &[10]).expect("corpus");
    assert!(corpus.prepare_query(&[("a", 3)]).is_err());
    assert!(corpus.prepare_query(&[("a", 0), ("a", 0)]).is_err());
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
                    length: u64::MAX
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
            purrdf_text::Analyzer::empty_lexicon(),
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

/// The ranking profile identity is a published fingerprint: its bytes are the
/// BLAKE3 digest of the profile's length-framed canonical description, and a
/// moved value is a different profile. Frozen over the single-field law and
/// over a two-field profile with predicate routing, so every framed field of
/// the description — names and routed IRIs — is covered.
#[test]
fn the_ranking_profile_fingerprint_is_frozen() {
    let hex = |profile: &RankingProfile| purrdf_hash::hex::encode(&profile.fingerprint());
    assert_eq!(
        hex(&RankingProfile::single_field()),
        "0640f4572eb75938bfdb202ce627ba796de5308581c9ab09ebbe536096cbf7a8"
    );
    let routed = RankingProfile::new(
        vec![field("title", Fixed::ONE, B), field("body", Fixed::ONE, B)],
        vec![
            (TermValue::iri("https://example.org/title"), 0),
            (TermValue::iri("https://example.org/body"), 1),
        ],
        Some(1),
    )
    .expect("a routed profile");
    assert_eq!(
        hex(&routed),
        "ade45c9a911230438d4cd179d2d7cbef09d67a88ec045611a8845b125f97c674"
    );
}
