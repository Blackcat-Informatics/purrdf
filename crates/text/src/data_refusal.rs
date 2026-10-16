// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native analysis, index and ranking refusals with original semantic fields.

use crate::{fixed::Scaled, query_workspace::QueryString};

#[derive(Debug)]
pub(crate) enum DataRefusal {
    TokenPositions {
        count: u128,
    },
    SourceSize {
        kind: &'static str,
        bytes: usize,
    },
    DocumentCount {
        count: usize,
    },
    DictionaryOrdinal {
        ordinal: usize,
    },
    DocumentPositions {
        count: u128,
    },
    EditInput {
        observed: usize,
    },
    SubstringTerms {
        count: usize,
    },
    ConflictingAnalysis {
        document: u32,
        identity: [u8; 32],
    },
    EmptySurfaceTerm {
        projection: &'static str,
        ordinal: usize,
    },
    SurfaceTerms {
        words: usize,
        spans: usize,
    },
    ProjectionRange {
        document: u32,
        source: usize,
        start: usize,
        end: usize,
        bytes: usize,
    },
    ProjectionSpelling {
        document: u32,
        source: usize,
        start: usize,
        end: usize,
        expected: String,
        actual: String,
        coarse: bool,
    },
    ExplainDocument {
        document: u32,
    },
    ScoreDocument {
        document: u32,
    },
    MissingPartition {
        profile: &'static str,
        graph: Option<purrdf_sparql_eval::WorkspaceTerm>,
        language: Option<QueryString>,
    },
    SourceDigest {
        expected: [u8; 32],
        actual: [u8; 32],
    },
    MissingPredicates {
        predicates: Vec<purrdf_core::TermValue>,
    },
    ForeignTermId,
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
        TokenPositions { count } => "text-data-token-positions" { "count" => Unsigned(*count), "limit" => Unsigned(u128::from(u32::MAX)) },
        SourceSize { kind, bytes } => "text-data-source-size" { "kind" => Code(kind), "bytes" => ByteCount(*bytes as u128), "limit" => ByteCount(u128::from(u32::MAX)) },
        DocumentCount { count } => "text-data-document-count" { "count" => Unsigned(*count as u128), "limit" => Unsigned(u128::from(u32::MAX)) },
        DictionaryOrdinal { ordinal } => "text-data-dictionary-ordinal" { "ordinal" => Unsigned(*ordinal as u128), "limit" => Unsigned(u128::from(u32::MAX)) },
        DocumentPositions { count } => "text-data-document-positions" { "count" => Unsigned(*count), "limit" => Unsigned(u128::from(u32::MAX)) },
        EditInput { observed } => "text-data-edit-input" { "observed" => Unsigned(*observed as u128), "limit" => Unsigned(crate::phonetic::MAX_INPUT_SCALARS as u128), "complete_count" => Boolean(false) },
        SubstringTerms { count } => "text-data-substring-terms" { "count" => Unsigned(*count as u128), "expected" => Unsigned(1) },
        ConflictingAnalysis { document, identity } => "text-data-conflicting-analysis" { "document" => Unsigned(u128::from(*document)), "identity" => Bytes(identity) },
        EmptySurfaceTerm { projection, ordinal } => "text-data-empty-surface-term" { "projection" => Code(projection), "ordinal" => Unsigned(*ordinal as u128) },
        SurfaceTerms { words, spans } => "text-data-surface-terms" { "words" => Unsigned(*words as u128), "spans" => Unsigned(*spans as u128), "limit" => Unsigned(u128::from(u32::MAX)) },
        ProjectionRange { document, source, start, end, bytes } => "text-data-projection-range" { "document" => Unsigned(u128::from(*document)), "source" => Unsigned(*source as u128), "start" => ByteCount(*start as u128), "end" => ByteCount(*end as u128), "normalized_bytes" => ByteCount(*bytes as u128) },
        ProjectionSpelling { document, source, start, end, expected, actual, coarse } => "text-data-projection-spelling" { "document" => Unsigned(u128::from(*document)), "source" => Unsigned(*source as u128), "start" => ByteCount(*start as u128), "end" => ByteCount(*end as u128), "expected" => Text(expected), "actual" => Text(actual), "coarse" => Boolean(*coarse) },
        ExplainDocument { document } => "text-data-explain-document" { "document" => Unsigned(u128::from(*document)) },
        ScoreDocument { document } => "text-data-score-document" { "document" => Unsigned(u128::from(*document)) },
        MissingPartition { profile, graph, language } => "text-data-missing-partition" { "profile" => Code(profile), "graph" => Value(graph.as_ref().map_or(purrdf_hash::error::ErrorValue::Absent, |term| purrdf_hash::error::ErrorValue::Term(&**term))), "language" => Value(language.as_ref().map_or(purrdf_hash::error::ErrorValue::Absent, |value| purrdf_hash::error::ErrorValue::Text(value.as_str()))) },
        SourceDigest { expected, actual } => "text-data-source-digest" { "expected" => Bytes(expected), "actual" => Bytes(actual) },
        MissingPredicates { predicates } => "text-data-missing-predicates" { "count" => Unsigned(predicates.len() as u128), "predicates" => List(predicates.iter().map(|term| purrdf_hash::error::ErrorValue::Term(term))) },
        ForeignTermId => "text-data-foreign-term-id" {},
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
            Self::TokenPositions { .. } => "token positions exceed u32",
            Self::SourceSize { kind, .. } => return write!(f, "{kind} exceeds source record space"),
            Self::DocumentCount { count } => return write!(f, "the dataset yields {count} documents, which exceeds the u32 document-id space this index addresses"),
            Self::DictionaryOrdinal { .. } => "term dictionary exceeds the u32 ordinal space",
            Self::DocumentPositions { .. } => "document exceeds the u32 token position space",
            Self::EditInput { .. } => "edit-distance input exceeds 2048 scalars",
            Self::SubstringTerms { .. } => "substring query must normalize to one nonempty span",
            Self::ConflictingAnalysis { .. } => "one source identity names conflicting analyses",
            Self::EmptySurfaceTerm { .. } => "surface indexes cannot contain empty terms",
            Self::SurfaceTerms { .. } => "surface index exceeds u32 distinct-term identifiers",
            Self::ProjectionRange { .. } => "surface projection has an invalid normalized range",
            Self::ProjectionSpelling { .. } => "surface projection spelling disagrees with its normalized range",
            Self::ExplainDocument { document } => return write!(f, "document {document} is not in this index, so there is nothing to explain"),
            Self::ScoreDocument { document } => return write!(f, "document {document} is not in this index, so there is nothing to score"),
            Self::MissingPartition { .. } => "ranking partition is absent",
            Self::SourceDigest { expected, actual } => return write!(f,
                "this index was built over a different dataset: its source digest is {expected:02x?} \
             and the supplied dataset digests to {actual:02x?}. Rebuild the index from the dataset \
             the query runs against, or pair the query with the dataset the index was built from — \
             an index joined to the wrong dataset returns no rows and reports nothing."),
            Self::MissingPredicates { predicates } => return write!(f,
                "this dataset holds statements but carries none under {} of this configuration's \
             predicates: {predicates:?}. Each contributes no text, so the corpus is short by their \
             share of it while every digest agrees. Check the spelling of those predicate IRIs and \
             of the configured graph — a named graph the dataset does not hold leaves every \
             configured predicate with nothing in scope. If the data is instead still arriving, \
             read TextIndex::source_coverage rather than asking for this verdict: the index \
             answers, and reports the corpus it has.", predicates.len()),
            Self::ForeignTermId => {
                return core::fmt::Display::fmt(
                    &purrdf_core::TermLookupError::<core::convert::Infallible>::ForeignId,
                    f,
                );
            }
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

#[cfg(test)]
mod tests {
    #[test]
    fn representational_limits_keep_full_width_values_and_original_diagnostic_shape() {
        use purrdf_hash::error::ErrorValue;
        use purrdf_testkit::error_capture::Capture;
        let error = crate::query_workspace::data_error(
            super::DataRefusal::TokenPositions { count: u128::MAX },
            None,
        );
        let cause = Capture::read(&error).child("reason").child("cause");
        assert_eq!(cause.code, "text-data-token-positions");
        assert!(matches!(
            cause.field("count"),
            ErrorValue::Unsigned(u128::MAX)
        ));
        assert!(
            matches!(cause.field("limit"), ErrorValue::Unsigned(value) if value == u128::from(u32::MAX))
        );
        let error = crate::query_workspace::arithmetic_error(
            crate::arithmetic::ArithmeticRefusal::PartitionRank { rank: usize::MAX },
            None,
        );
        assert!(matches!(error, crate::TextError::Diagnostic(_)));
        assert_eq!(
            error.to_string(),
            "host function error: fixed-point overflow: a partition holds more ranked rows than a u32 can number"
        );
        let cause = Capture::read(&error).child("cause").child("cause");
        assert_eq!(cause.code, "text-partition-rank-overflow");
        assert!(
            matches!(cause.field("rank"), ErrorValue::Unsigned(value) if value == usize::MAX as u128)
        );
    }
}
