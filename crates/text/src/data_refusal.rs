// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native ranking facts refused before scoring.

use crate::{fixed::Scaled, query_workspace::QueryString};

#[derive(Debug)]
pub(crate) enum DataRefusal {
    ExplicitPopulations {
        fields: usize,
    },
    PopulationMode {
        enabled: bool,
        fields: usize,
        populations: usize,
    },
    CorpusFields {
        expected: usize,
        actual: usize,
    },
    CorpusBound {
        field: usize,
        total: u128,
        population: u64,
        documents: u64,
    },
    QueryOrder {
        term: QueryString,
        previous: Option<QueryString>,
    },
    QueryFrequencySum {
        sum: Scaled,
        total: Scaled,
    },
    EmptyCorpus,
    TermOrdinal {
        ordinal: usize,
        terms: usize,
    },
    DocumentFields {
        expected: usize,
        actual: usize,
    },
    DocumentTerms {
        expected: usize,
        actual: usize,
    },
    FieldLengths {
        term: usize,
        field: usize,
        expected: Option<u64>,
        actual: Option<u64>,
    },
    FrequencySum {
        term: usize,
        field: usize,
        current: u64,
        added: u64,
        length: u64,
    },
    FieldFacts {
        frequency: u64,
        length: u64,
        total: u128,
    },
    RemainingTotal {
        length: u64,
        total: u128,
        remaining: u64,
    },
    EmptyPopulation {
        frequency: u64,
        length: u64,
        documents: u64,
        df: u64,
    },
    DocumentFrequency {
        documents: u64,
        frequency: u64,
    },
    MissingDocument {
        document: u32,
    },
}

impl purrdf_hash::error::StructuredError for DataRefusal {
    purrdf_hash::error_visit! {
        ExplicitPopulations { fields } => "text-data-explicit-populations" { "fields" => Unsigned(*fields as u128) },
        PopulationMode { enabled, fields, populations } => "text-data-population-mode" {
            "enabled" => Boolean(*enabled), "fields" => Unsigned(*fields as u128), "populations" => Unsigned(*populations as u128),
        },
        CorpusFields { expected, actual } => "text-data-corpus-fields" { "expected" => Unsigned(*expected as u128), "actual" => Unsigned(*actual as u128) },
        CorpusBound { field, total, population, documents } => "text-data-corpus-bound" {
            "field" => Unsigned(*field as u128), "total" => Unsigned(*total), "population" => Unsigned(u128::from(*population)), "documents" => Unsigned(u128::from(*documents)),
        },
        QueryOrder { term, previous } => "text-data-query-order" {
            "term" => Text(term.as_str()), "previous" => Value(previous.as_ref().map_or(purrdf_hash::error::ErrorValue::Absent, |value| purrdf_hash::error::ErrorValue::Text(value.as_str()))),
        },
        QueryFrequencySum { sum, total } => "text-data-query-frequency-sum" { "sum" => Record(sum), "total" => Record(total), "unit" => Code("tokens") },
        EmptyCorpus => "text-data-empty-corpus" {},
        TermOrdinal { ordinal, terms } => "text-data-term-ordinal" { "ordinal" => Unsigned(*ordinal as u128), "terms" => Unsigned(*terms as u128) },
        DocumentFields { expected, actual } => "text-data-document-fields" { "expected" => Unsigned(*expected as u128), "actual" => Unsigned(*actual as u128) },
        DocumentTerms { expected, actual } => "text-data-document-terms" { "expected" => Unsigned(*expected as u128), "actual" => Unsigned(*actual as u128) },
        FieldLengths { term, field, expected, actual } => "text-data-field-lengths" {
            "term" => Unsigned(*term as u128), "field" => Unsigned(*field as u128),
            "expected" => Value(expected.map_or(purrdf_hash::error::ErrorValue::Absent, |value| purrdf_hash::error::ErrorValue::Unsigned(u128::from(value)))),
            "actual" => Value(actual.map_or(purrdf_hash::error::ErrorValue::Absent, |value| purrdf_hash::error::ErrorValue::Unsigned(u128::from(value)))),
        },
        FrequencySum { term, field, current, added, length } => "text-data-frequency-sum" {
            "term" => Unsigned(*term as u128), "field" => Unsigned(*field as u128), "current" => Unsigned(u128::from(*current)), "added" => Unsigned(u128::from(*added)), "length" => Unsigned(u128::from(*length)),
        },
        FieldFacts { frequency, length, total } => "text-data-field-facts" { "frequency" => Unsigned(u128::from(*frequency)), "length" => Unsigned(u128::from(*length)), "total" => Unsigned(*total) },
        RemainingTotal { length, total, remaining } => "text-data-remaining-total" { "length" => Unsigned(u128::from(*length)), "total" => Unsigned(*total), "remaining" => Unsigned(u128::from(*remaining)) },
        EmptyPopulation { frequency, length, documents, df } => "text-data-empty-population" { "frequency" => Unsigned(u128::from(*frequency)), "length" => Unsigned(u128::from(*length)), "documents" => Unsigned(u128::from(*documents)), "df" => Unsigned(u128::from(*df)) },
        DocumentFrequency { documents, frequency } => "text-data-document-frequency" { "documents" => Unsigned(u128::from(*documents)), "frequency" => Unsigned(u128::from(*frequency)) },
        MissingDocument { document } => "text-data-missing-document" { "document" => Unsigned(u128::from(*document)) },
    }
}

impl core::fmt::Display for DataRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::ExplicitPopulations { .. } => {
                "this ranking profile requires explicit field populations"
            }
            Self::PopulationMode { .. } => {
                "field populations require their ranking mode and one count per field"
            }
            Self::CorpusFields { .. } => {
                "corpus population or field count exceeds the ranking profile"
            }
            Self::CorpusBound { .. } => {
                "a field population or token total exceeds its corpus bound"
            }
            Self::QueryOrder { .. } => {
                "prepared query terms must be nonempty, distinct and strictly sorted"
            }
            Self::QueryFrequencySum { .. } => {
                "distinct query document frequencies exceed all tokens in their corpus"
            }
            Self::EmptyCorpus => "an empty corpus contains no document to score",
            Self::TermOrdinal { .. } => "query term ordinal is absent",
            Self::DocumentFields { .. } => {
                "document field count disagrees with its ranking profile"
            }
            Self::DocumentTerms { .. } => "document term count disagrees with its prepared query",
            Self::FieldLengths { .. } => {
                "one document has inconsistent field lengths across query terms"
            }
            Self::FrequencySum { .. } => {
                "distinct query term frequencies exceed the document field length"
            }
            Self::FieldFacts { .. } => "field frequency, length, and corpus total are inconsistent",
            Self::RemainingTotal { .. } => {
                "the remaining corpus cannot hold the declared field total"
            }
            Self::EmptyPopulation { .. } => {
                "a nonempty document or term cannot belong to an empty corpus or posting list"
            }
            Self::DocumentFrequency { .. } => "document frequency exceeds its corpus population",
            Self::MissingDocument { .. } => "field input names an absent document",
        })
    }
}

impl std::error::Error for DataRefusal {}
