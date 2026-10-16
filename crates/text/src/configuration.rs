// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original values refused by native text configuration admission.

use purrdf_core::{IriError, TermValue};

#[derive(Debug)]
pub(crate) enum ConfigurationRefusal {
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
            Self::RankingIri { cause, .. } => Some(cause),
            _ => None,
        }
    }
}
