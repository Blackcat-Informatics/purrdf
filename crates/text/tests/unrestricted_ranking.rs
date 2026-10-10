// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Actual oversized inputs, independent arithmetic and public retrieval callers.

#[path = "support/unrestricted_ranking_cases.rs"]
mod cases;

#[cfg(not(target_arch = "wasm32"))]
use purrdf_core::{RdfDatasetBuilder, RdfLiteral};
use purrdf_core::{SparqlRequest, SparqlResult, TermValue};
use purrdf_sparql_eval::{
    ExtensionEnv, NativeSparqlEngine, PropertyFunctionRegistry, QueryOptions,
};
#[cfg(not(target_arch = "wasm32"))]
use purrdf_text::{B, PartitionKey, TextIndex, explain, rank_partition};
use purrdf_text::{Fixed, TextSearchRelation};
#[cfg(not(target_arch = "wasm32"))]
use std::fmt::Write as _;
use std::sync::Arc;

#[test]
fn a_complete_five_thousand_term_needle_reaches_the_public_relation() {
    let (dataset, index, needle, expected) = cases::five_thousand_term_index();
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        "https://example.org/search".to_owned(),
        Arc::new(TextSearchRelation::new(index)),
    );
    let environment = ExtensionEnv::over_relations(registry).expect("registry");
    let query = format!(
        "SELECT ?doc ?score ?rank ?matched WHERE {{ ?doc <https://example.org/search> (\"{needle}\" ?score ?rank ?lang ?matched) }} ORDER BY ?rank"
    );
    let result = NativeSparqlEngine::new()
        .query_with_options_view(
            &*dataset,
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::new().with_env(&environment),
        )
        .expect("actual public query");
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("SELECT result");
    };
    assert_eq!(rows.len(), 2);
    for (at, row) in rows.iter().enumerate() {
        assert_eq!(
            row[0],
            Some(TermValue::iri(format!(
                "https://example.org/{}",
                if at == 0 { "a" } else { "b" }
            )))
        );
        assert_eq!(
            row[1],
            Some(TermValue::typed_literal(
                Fixed::from_raw(expected[at]).to_decimal_lexical(),
                purrdf_core::datatype::XSD_DECIMAL
            ))
        );
        assert_eq!(row[2], Some(TermValue::integer((at + 1) as i128)));
        assert_eq!(
            row[3],
            Some(TermValue::integer(if at == 0 { 5_000 } else { 2_500 }))
        );
    }
}

#[test]
fn full_u64_populations_and_large_query_scores_match_independent_arithmetic() {
    cases::declared_large_corpora_are_exact();
}

#[test]
fn promoted_products_and_unrestricted_fields_keep_exact_rounding() {
    cases::promoted_field_arithmetic_is_exact();
    cases::thirty_two_index_fields_are_exact();
}

/// A physical native index over exactly 2^25 tokens in one merged field.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn actual_two_to_the_twenty_fifth_token_field_builds_and_scores() {
    let tokens_per_literal = 1_usize << 20;
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri("https://example.org/large-document");
    let predicate = builder.intern_iri(cases::NOTE);
    for at in 0..32 {
        // One unique final term makes all source literals distinct RDF rows.
        // The normal analyzer sees 2^20 terms and one punctuation-bearing span.
        let mut lexical = "a,".repeat(tokens_per_literal - 1);
        write!(lexical, "chunk{at:02}").expect("write to String");
        let object = builder.intern_literal(RdfLiteral::simple(&lexical));
        builder.push_quad(subject, predicate, object, None);
    }
    let dataset = builder.freeze().expect("actual dataset");
    let index = TextIndex::from_dataset(
        &*dataset,
        &cases::configuration(vec![TermValue::iri(cases::NOTE)]),
    )
    .expect("actual 2^25-token field builds");
    let length = 1_u64 << 25;
    let frequency = length - 32;
    assert_eq!(index.field_lengths(0), Some([length].as_slice()));
    assert_eq!(index.term_frequency(0, "a"), frequency);
    assert_eq!(index.document(0).expect("document").length(), length);
    let partition = PartitionKey::new(None, None);
    assert_eq!(
        index.field_totals(&partition),
        Some([u128::from(length)].as_slice())
    );
    let rows =
        rank_partition(&index, &partition, &["a".to_owned()], None).expect("large field ranks");
    assert_eq!(rows.len(), 1);
    let expected = cases::ranking_oracle::contribution(
        1,
        1,
        &[1],
        &[u128::from(length)],
        &[(Fixed::ONE.into_raw(), B.into_raw())],
        &[cases::input(frequency, length)],
    );
    assert_eq!(expected, 632_900_536_757, "unchanged Python reference");
    assert_eq!(rows[0].score.into_raw(), expected);
    let detail = explain(&index, 0, &["a".to_owned()]).expect("actual explanation");
    assert_eq!(
        (detail[0].term_frequency, detail[0].contribution),
        (frequency, rows[0].score)
    );
    assert!(
        index
            .surface_index()
            .words()
            .iter()
            .any(|term| term.text() == "a")
    );
    assert_eq!(
        index.surface_index().spans().len(),
        1,
        "the bounded span spelling is shared without suppressing the surface projection"
    );
}
