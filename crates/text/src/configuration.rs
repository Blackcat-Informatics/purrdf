// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original values refused by native text configuration admission.

use purrdf_core::{IriError, TermValue};

#[derive(Debug)]
pub(crate) enum ConfigurationRefusal {
    DictionaryEntries {
        entries: usize,
    },
    DictionarySize {
        current: usize,
        added: usize,
    },
    DictionaryArena {
        bytes: usize,
    },
    DictionaryCosts {
        word: String,
        first: u32,
        second: u32,
    },
    DictionaryInput {
        bytes: usize,
    },
    DictionaryEmoji {
        start: usize,
        end: usize,
    },
    DictionaryKey {
        key: String,
    },
    ArtifactSize {
        bytes: usize,
    },
    ArtifactPhysical {
        expected: [u8; 32],
        actual: [u8; 32],
    },
    ArtifactArity {
        items: u64,
    },
    ArtifactProfile {
        actual: String,
    },
    ArtifactUnicode {
        actual: Vec<u8>,
    },
    ArtifactLayout {
        remaining: usize,
        offsets: usize,
        costs: usize,
        first: Option<u32>,
        last: Option<u32>,
        arena_bytes: usize,
    },
    ArtifactOffset {
        start: u32,
        end: u32,
        arena_bytes: usize,
    },
    ArtifactKeyOrder {
        previous: String,
        word: String,
    },
    ArtifactKeyCanonical {
        word: String,
        canonical: String,
    },
    ArtifactSemantic {
        expected: Vec<u8>,
        computed: [u8; 32],
        canonical: Option<bool>,
    },
    ArtifactHead {
        major: u8,
        first: Option<u8>,
        before: usize,
        after: usize,
    },
    ArtifactHeadCanonical {
        major: u8,
        value: u64,
        bytes: usize,
        expected: usize,
    },
    ArtifactTextSize {
        length: u64,
    },
    ArtifactByteSize {
        length: u64,
    },
    ArtifactArraySize {
        length: u64,
    },
    ArtifactTextTruncated {
        requested: usize,
        remaining: usize,
    },
    ArtifactBytesTruncated {
        requested: usize,
        remaining: usize,
    },
    ArtifactUtf8 {
        cause: core::str::Utf8Error,
    },
    ArtifactArrayLimit {
        count: usize,
        maximum: usize,
        remaining: usize,
    },
    ArtifactInteger {
        value: u64,
    },
    EmptyLexiconArtifacts {
        count: usize,
    },
    CallerDictionaryArtifacts {
        count: usize,
    },
    UnexpectedBaseline {
        identity: [u8; 32],
    },
    DuplicateBaseline {
        identity: [u8; 32],
    },
    MissingBaselines {
        present: [bool; 5],
    },
    MultiplePartitions {
        partitions: usize,
    },
    TokenBound {
        requested: usize,
    },
    PhoneticCodeLength {
        requested: usize,
    },
    PhoneticProfileDistance {
        requested: usize,
    },
    EditThreshold {
        requested: usize,
    },
    NoPredicates,
    IndexedPredicate {
        predicate: TermValue,
    },
    NamedGraph {
        graph: TermValue,
    },
    DuplicatePredicate {
        predicate: TermValue,
    },
    ScopePredicate {
        predicate: TermValue,
    },
    ScopeToken {
        token: String,
    },
    GraphFreeSource,
    GraphFreeData {
        graph: TermValue,
    },
    FieldNameOrWeight {
        name: String,
        weight: i128,
    },
    FieldNormalization {
        b: i128,
    },
    NoFields,
    DuplicateField {
        name: String,
    },
    UnclassifiedField {
        field: usize,
        fields: usize,
    },
    MappingField {
        predicate: TermValue,
        field: usize,
        fields: usize,
    },
    DuplicateMapping {
        predicate: TermValue,
    },
    UnmappedPredicate {
        predicate: TermValue,
    },
    RankingPredicate {
        predicate: TermValue,
    },
    RankingIri {
        iri: String,
        cause: IriError,
    },
    RelativeRankingIri {
        iri: String,
    },
}

impl ConfigurationRefusal {
    pub(crate) fn into_error(self) -> crate::TextError {
        use purrdf_sparql_eval::{NativeDiagnostic, NativeDiagnosticKind, WorkspaceCapability};
        match crate::query_workspace::admitted(NativeDiagnostic::render_cause(
            NativeDiagnosticKind::Config,
            self,
            &WorkspaceCapability::resident(),
        )) {
            Ok(diagnostic) => crate::TextError::Config(diagnostic),
            Err(error) => error,
        }
    }
}

impl purrdf_hash::error::StructuredError for ConfigurationRefusal {
    purrdf_hash::error_visit! {
        DictionaryEntries { entries } => "text-dictionary-entry-limit" { "entries" => Unsigned(*entries as u128), "maximum" => Unsigned(crate::segment::MAX_ENTRIES as u128) },
        DictionarySize { current, added } => "text-dictionary-size-overflow" { "current" => Unsigned(*current as u128), "added" => Unsigned(*added as u128) },
        DictionaryArena { bytes } => "text-dictionary-arena-limit" { "bytes" => Unsigned(*bytes as u128), "maximum" => Unsigned(crate::segment::MAX_ARENA_BYTES as u128) },
        DictionaryCosts { word, first, second } => "text-dictionary-conflicting-costs" { "word" => Text(word), "first" => Unsigned(u128::from(*first)), "second" => Unsigned(u128::from(*second)) },
        DictionaryInput { bytes } => "text-dictionary-input-limit" { "bytes" => Unsigned(*bytes as u128), "maximum" => Unsigned((crate::segment::MAX_WORD_SCALARS * 4) as u128) },
        DictionaryEmoji { start, end } => "text-dictionary-protected-emoji" { "start" => Unsigned(*start as u128), "end" => Unsigned(*end as u128) },
        DictionaryKey { key } => "text-dictionary-invalid-key" { "key" => Text(key), "maximum_scalars" => Unsigned(crate::segment::MAX_WORD_SCALARS as u128) },
        ArtifactSize { bytes } => "text-dictionary-artifact-size" { "bytes" => Unsigned(*bytes as u128), "maximum" => Unsigned(crate::segment::MAX_ARTIFACT_BYTES as u128) },
        ArtifactPhysical { expected, actual } => "text-dictionary-artifact-identity" { "expected" => Bytes(expected), "actual" => Bytes(actual) },
        ArtifactArity { items } => "text-dictionary-artifact-arity" { "items" => Unsigned(u128::from(*items)), "expected" => Unsigned(6) },
        ArtifactProfile { actual } => "text-dictionary-artifact-profile" { "actual" => Text(actual), "expected" => Code(crate::segment::PROFILE_ID) },
        ArtifactUnicode { actual } => "text-dictionary-artifact-unicode" { "actual" => Bytes(actual), "expected_major" => Unsigned(u128::from(crate::unicode::UNICODE_VERSION.0)), "expected_minor" => Unsigned(u128::from(crate::unicode::UNICODE_VERSION.1)), "expected_patch" => Unsigned(u128::from(crate::unicode::UNICODE_VERSION.2)) },
        ArtifactLayout { remaining, offsets, costs, first, last, arena_bytes } => "text-dictionary-artifact-layout" {
            "remaining" => Unsigned(*remaining as u128), "offsets" => Unsigned(*offsets as u128), "costs" => Unsigned(*costs as u128), "arena_bytes" => Unsigned(*arena_bytes as u128),
            "first" => Value(first.map_or(purrdf_hash::error::ErrorValue::Absent, |value| purrdf_hash::error::ErrorValue::Unsigned(u128::from(value)))),
            "last" => Value(last.map_or(purrdf_hash::error::ErrorValue::Absent, |value| purrdf_hash::error::ErrorValue::Unsigned(u128::from(value)))),
        },
        ArtifactOffset { start, end, arena_bytes } => "text-dictionary-artifact-offset" { "start" => Unsigned(u128::from(*start)), "end" => Unsigned(u128::from(*end)), "arena_bytes" => Unsigned(*arena_bytes as u128) },
        ArtifactKeyOrder { previous, word } => "text-dictionary-artifact-key-order" { "previous" => Text(previous), "word" => Text(word) },
        ArtifactKeyCanonical { word, canonical } => "text-dictionary-artifact-key-canonical" { "word" => Text(word), "canonical" => Text(canonical) },
        ArtifactSemantic { expected, computed, canonical } => "text-dictionary-artifact-semantic" { "expected" => Bytes(expected), "computed" => Bytes(computed), "canonical" => Value(canonical.map_or(purrdf_hash::error::ErrorValue::Absent, purrdf_hash::error::ErrorValue::Boolean)) },
        ArtifactHead { major, first, before, after } => "text-dictionary-artifact-head" { "major" => Unsigned(u128::from(*major)), "before" => Unsigned(*before as u128), "after" => Unsigned(*after as u128), "first" => Value(first.map_or(purrdf_hash::error::ErrorValue::Absent, |value| purrdf_hash::error::ErrorValue::Unsigned(u128::from(value)))) },
        ArtifactHeadCanonical { major, value, bytes, expected } => "text-dictionary-artifact-head-canonical" { "major" => Unsigned(u128::from(*major)), "value" => Unsigned(u128::from(*value)), "bytes" => Unsigned(*bytes as u128), "expected" => Unsigned(*expected as u128) },
        ArtifactTextSize { length } => "text-dictionary-artifact-text-size" { "length" => Unsigned(u128::from(*length)) },
        ArtifactByteSize { length } => "text-dictionary-artifact-byte-size" { "length" => Unsigned(u128::from(*length)) },
        ArtifactArraySize { length } => "text-dictionary-artifact-array-size" { "length" => Unsigned(u128::from(*length)) },
        ArtifactTextTruncated { requested, remaining } => "text-dictionary-artifact-text-truncated" { "requested" => Unsigned(*requested as u128), "remaining" => Unsigned(*remaining as u128) },
        ArtifactBytesTruncated { requested, remaining } => "text-dictionary-artifact-bytes-truncated" { "requested" => Unsigned(*requested as u128), "remaining" => Unsigned(*remaining as u128) },
        ArtifactUtf8 { cause } => "text-dictionary-artifact-utf8" { "valid_up_to" => Unsigned(cause.valid_up_to() as u128), "error_len" => Value(cause.error_len().map_or(purrdf_hash::error::ErrorValue::Absent, |value| purrdf_hash::error::ErrorValue::Unsigned(value as u128))) },
        ArtifactArrayLimit { count, maximum, remaining } => "text-dictionary-artifact-array-limit" { "count" => Unsigned(*count as u128), "maximum" => Unsigned(*maximum as u128), "remaining" => Unsigned(*remaining as u128) },
        ArtifactInteger { value } => "text-dictionary-artifact-integer" { "value" => Unsigned(u128::from(*value)), "maximum" => Unsigned(u128::from(u32::MAX)) },
        EmptyLexiconArtifacts { count } => "text-config-empty-lexicon-artifacts" { "count" => Unsigned(*count as u128) },
        CallerDictionaryArtifacts { count } => "text-config-caller-dictionary-artifacts" { "count" => Unsigned(*count as u128) },
        UnexpectedBaseline { identity } => "text-config-unexpected-baseline" { "identity" => Bytes(identity) },
        DuplicateBaseline { identity } => "text-config-duplicate-baseline" { "identity" => Bytes(identity) },
        MissingBaselines { present } => "text-config-missing-baselines" {
            "present" => List(present.iter().map(|&value| purrdf_hash::error::ErrorValue::Boolean(value))),
            "expected_identifiers" => List(crate::profile::baseline_ids().into_iter().map(purrdf_hash::error::ErrorValue::Text)),
        },
        MultiplePartitions { partitions } => "text-config-multiple-partitions" { "partitions" => Unsigned(*partitions as u128) },
        TokenBound { requested } => "text-config-token-bound" {
            "requested" => Unsigned(*requested as u128),
            "minimum" => Unsigned(crate::unicode::MAX_EMOJI_SCALARS as u128),
            "maximum" => Unsigned(crate::profile::MAX_TOKEN_SCALARS as u128),
        },
        PhoneticCodeLength { requested } => "text-config-phonetic-code-length" {
            "requested" => Unsigned(*requested as u128), "minimum" => Unsigned(1),
            "maximum" => Unsigned(crate::phonetic::MAX_CODE_LENGTH as u128),
        },
        PhoneticProfileDistance { requested } => "text-config-phonetic-distance" {
            "requested" => Unsigned(*requested as u128), "maximum" => Unsigned(crate::phonetic::MAX_EDIT_DISTANCE as u128),
        },
        EditThreshold { requested } => "text-config-edit-threshold" {
            "requested" => Unsigned(*requested as u128), "maximum" => Unsigned(crate::phonetic::MAX_EDIT_DISTANCE as u128),
        },
        NoPredicates => "text-config-no-predicates" {},
        IndexedPredicate { predicate } => "text-config-indexed-predicate" { "predicate" => Term(predicate) },
        NamedGraph { graph } => "text-config-named-graph" { "graph" => Term(graph) },
        DuplicatePredicate { predicate } => "text-config-duplicate-predicate" { "predicate" => Term(predicate) },
        ScopePredicate { predicate } => "text-config-scope-predicate" { "predicate" => Term(predicate) },
        ScopeToken { token } => "text-config-scope-token" { "token" => Text(token) },
        GraphFreeSource => "text-config-graph-free-source" {},
        GraphFreeData { graph } => "text-config-graph-free-data" { "graph" => Term(graph) },
        FieldNameOrWeight { name, weight } => "text-config-field-name-or-weight" { "name" => Text(name), "weight_raw" => Signed(*weight) },
        FieldNormalization { b } => "text-config-field-normalization" { "b_raw" => Signed(*b) },
        NoFields => "text-config-no-fields" {},
        DuplicateField { name } => "text-config-duplicate-field" { "name" => Text(name) },
        UnclassifiedField { field, fields } => "text-config-unclassified-field" { "field" => Unsigned(*field as u128), "fields" => Unsigned(*fields as u128) },
        MappingField { predicate, field, fields } => "text-config-mapping-field" { "predicate" => Term(predicate), "field" => Unsigned(*field as u128), "fields" => Unsigned(*fields as u128) },
        DuplicateMapping { predicate } => "text-config-duplicate-mapping" { "predicate" => Term(predicate) },
        UnmappedPredicate { predicate } => "text-config-unmapped-predicate" { "predicate" => Term(predicate) },
        RankingPredicate { predicate } => "text-config-ranking-predicate" { "predicate" => Term(predicate) },
        RankingIri { iri, cause } => "text-config-ranking-iri" { "iri" => Iri(iri), "cause" => Cause(cause) },
        RelativeRankingIri { iri } => "text-config-relative-ranking-iri" { "iri" => Iri(iri) },
    }
}

impl core::fmt::Display for ConfigurationRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DictionaryEntries { .. } => f.write_str("dictionary entry count exceeds its bound"),
            Self::DictionarySize { .. } => f.write_str("dictionary size overflow"),
            Self::DictionaryArena { .. } => f.write_str("dictionary arena exceeds its byte bound"),
            Self::DictionaryCosts { .. } => f.write_str("canonical dictionary key has conflicting costs"),
            Self::DictionaryInput { .. } => f.write_str("dictionary input exceeds its byte bound"),
            Self::DictionaryEmoji { .. } => f.write_str("dictionary entries cannot contain protected emoji"),
            Self::DictionaryKey { .. } => f.write_str("dictionary key must be lexical, bounded and contain no separators or non-internal controls"),
            Self::ArtifactSize { .. } | Self::ArtifactPhysical { .. } => f.write_str("dictionary artifact size or physical identity mismatch"),
            Self::ArtifactArity { .. } | Self::ArtifactProfile { .. } | Self::ArtifactUnicode { .. } => f.write_str("dictionary artifact format or Unicode edition mismatch"),
            Self::ArtifactLayout { .. } => f.write_str("dictionary artifact structure mismatch"),
            Self::ArtifactOffset { .. } => f.write_str("dictionary offsets are not UTF-8 boundaries"),
            Self::ArtifactKeyOrder { .. } | Self::ArtifactKeyCanonical { .. } => f.write_str("dictionary entries are not sorted canonical unique keys"),
            Self::ArtifactSemantic { .. } => f.write_str("dictionary semantic identity or canonical encoding mismatch"),
            Self::ArtifactHead { .. } => f.write_str("invalid dictionary CBOR head"),
            Self::ArtifactHeadCanonical { .. } => f.write_str("noncanonical dictionary CBOR head"),
            Self::ArtifactTextSize { .. } => f.write_str("dictionary text size overflow"),
            Self::ArtifactByteSize { .. } => f.write_str("dictionary byte size overflow"),
            Self::ArtifactArraySize { .. } => f.write_str("dictionary array size overflow"),
            Self::ArtifactTextTruncated { .. } => f.write_str("truncated dictionary text"),
            Self::ArtifactBytesTruncated { .. } => f.write_str("truncated dictionary bytes"),
            Self::ArtifactUtf8 { .. } => f.write_str("dictionary text is not UTF-8"),
            Self::ArtifactArrayLimit { .. } => f.write_str("dictionary array exceeds its bound"),
            Self::ArtifactInteger { .. } => f.write_str("dictionary integer exceeds u32"),
            Self::EmptyLexiconArtifacts { .. } => f.write_str("empty lexicon profile refuses artifact bytes"),
            Self::CallerDictionaryArtifacts { .. } => f.write_str("caller dictionary profile refuses baseline artifacts"),
            Self::UnexpectedBaseline { .. } => f.write_str("unexpected baseline artifact identity"),
            Self::DuplicateBaseline { .. } => f.write_str("duplicate baseline artifact"),
            Self::MissingBaselines { .. } => f.write_str("standard analysis requires all five full baseline artifacts"),
            Self::MultiplePartitions { partitions } => write!(f,
                "this index holds {partitions} partitions, and a rank is computed within one \
                 partition: the rows of a multi-partition answer are emitted partition-major, so \
                 they open with {partitions} rows of rank 1 and one subject may appear in more \
                 than one of them. No ranked ordering can be declared over that, and declaring \
                 one anyway would hand a consumer positions it would fuse as though they were a \
                 ranking. Build the index over a single partition — GraphSelector::Named or \
                 GraphSelector::Default over a corpus in one language — or write the declaration \
                 by hand with the claim the host can actually stand behind."),
            Self::TokenBound { .. } => f.write_str("token bound must admit the longest recognized emoji and be at most 1024"),
            Self::PhoneticCodeLength { .. } => f.write_str("phonetic code length must be in 1..=64"),
            Self::PhoneticProfileDistance { .. } => f.write_str("phonetic edit distance exceeds 64"),
            Self::EditThreshold { .. } => f.write_str("edit-distance threshold must be in 0..=64"),
            Self::NoPredicates => f.write_str("no indexed predicates supplied; PurRDF mints no vocabulary, so there is no default predicate set to fall back on"),
            Self::IndexedPredicate { predicate } => write!(f, "indexed predicate {predicate:?} is not an IRI; only an IRI can occupy the predicate position of an RDF statement"),
            Self::NamedGraph { graph } => write!(f, "named graph selector {graph:?} is not an IRI"),
            Self::DuplicatePredicate { predicate } => write!(f, "indexed predicate {predicate:?} is listed more than once"),
            Self::ScopePredicate { .. } => f.write_str("field scope predicate is not selected by this text index"),
            Self::ScopeToken { .. } => f.write_str("field scope token is outside the analyzed query needle"),
            Self::GraphFreeSource => f.write_str("the graph-free text profile requires a graph-free source"),
            Self::GraphFreeData { .. } => f.write_str("named graph data cannot enter a graph-free text corpus"),
            Self::FieldNameOrWeight { .. } => f.write_str("a field needs a name and a nonnegative weight"),
            Self::FieldNormalization { .. } => f.write_str("a field's b must be in [0, 1]"),
            Self::NoFields => f.write_str("a ranking profile needs at least one field"),
            Self::DuplicateField { name } => write!(f, "repeated field name {name:?}"),
            Self::UnclassifiedField { .. } => f.write_str("the unclassified field is not in this profile"),
            Self::MappingField { .. } => f.write_str("a predicate mapping needs an IRI and an existing field"),
            Self::DuplicateMapping { predicate } => write!(f, "repeated mapped predicate {predicate:?}"),
            Self::UnmappedPredicate { predicate } => write!(f, "selected predicate {predicate:?} has no ranking field"),
            Self::RankingPredicate { .. } => f.write_str("a ranking predicate must be an IRI"),
            Self::RankingIri { iri, cause } => write!(f, "invalid ranking predicate {iri:?}: {cause}"),
            Self::RelativeRankingIri { iri } => write!(f, "ranking predicate {iri:?} must be absolute"),
        }
    }
}

impl std::error::Error for ConfigurationRefusal {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ArtifactUtf8 { cause } => Some(cause),
            Self::RankingIri { cause, .. } => Some(cause),
            _ => None,
        }
    }
}
