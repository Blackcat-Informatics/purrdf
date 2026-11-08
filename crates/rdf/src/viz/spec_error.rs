// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original vocabulary and specification operands at projection refusal.

use std::fmt;

use purrdf_hash::error::StructuredError;

use super::VizTableField;

/// A rejected vocabulary mapping and its original conflicting values.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum VizVocabularyError {
    /// A mapping has an empty prefix or namespace.
    EmptyMapping {
        /// Mapping position in the caller's vocabulary.
        index: usize,
        /// Original prefix, including an empty one.
        prefix: String,
        /// Original namespace, including an empty one.
        namespace: String,
    },
    /// One prefix maps to two different namespaces.
    PrefixConflict {
        /// Rejected mapping position.
        index: usize,
        /// Position of the most recent preceding mapping for this prefix.
        previous_index: usize,
        /// Original shared prefix.
        prefix: String,
        /// Namespace from the preceding mapping.
        existing_namespace: String,
        /// Namespace from the rejected mapping.
        incoming_namespace: String,
    },
    /// One namespace maps to two different prefixes.
    NamespaceConflict {
        /// Rejected mapping position.
        index: usize,
        /// Position of the most recent preceding mapping for this namespace.
        previous_index: usize,
        /// Original shared namespace.
        namespace: String,
        /// Prefix from the preceding mapping.
        existing_prefix: String,
        /// Prefix from the rejected mapping.
        incoming_prefix: String,
    },
}

impl fmt::Display for VizVocabularyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyMapping { .. } => f.write_str(
                "visualization vocabulary mappings require non-empty prefix and namespace",
            ),
            Self::PrefixConflict { prefix, .. } => {
                write!(
                    f,
                    "visualization prefix {prefix:?} maps to multiple namespaces"
                )
            }
            Self::NamespaceConflict { namespace, .. } => {
                write!(
                    f,
                    "visualization namespace {namespace:?} maps to multiple prefixes"
                )
            }
        }
    }
}

impl std::error::Error for VizVocabularyError {}

impl StructuredError for VizVocabularyError {
    purrdf_hash::error_visit! {
        EmptyMapping { index, prefix, namespace } => "rdf-viz-empty-vocabulary-mapping" {
            "index" => Unsigned(*index as u128),
            "prefix" => Text(prefix), "namespace" => Text(namespace),
        },
        PrefixConflict { index, previous_index, prefix, existing_namespace, incoming_namespace }
            => "rdf-viz-prefix-conflict" {
            "index" => Unsigned(*index as u128),
            "previous_index" => Unsigned(*previous_index as u128),
            "prefix" => Text(prefix),
            "existing_namespace" => Text(existing_namespace),
            "incoming_namespace" => Text(incoming_namespace),
        },
        NamespaceConflict { index, previous_index, namespace, existing_prefix, incoming_prefix }
            => "rdf-viz-namespace-conflict" {
            "index" => Unsigned(*index as u128),
            "previous_index" => Unsigned(*previous_index as u128),
            "namespace" => Text(namespace),
            "existing_prefix" => Text(existing_prefix),
            "incoming_prefix" => Text(incoming_prefix),
        },
    }
}

/// A graph-selector or table-field specification refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum VizSpecError {
    /// The first empty graph selector.
    EmptyGraphSelector {
        /// Position in the caller's selector list.
        index: usize,
    },
    /// The first repeated table field, with both occurrence positions.
    DuplicateTableField {
        /// Original requested field.
        field: VizTableField,
        /// Position of its first occurrence.
        first_index: usize,
        /// Position of its first repetition.
        index: usize,
        /// Total number of requested fields.
        actual: usize,
        /// Number of distinct requested fields.
        unique: usize,
    },
}

impl fmt::Display for VizSpecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::EmptyGraphSelector { .. } => "visualization graph selectors must not be empty",
            Self::DuplicateTableField { .. } => "visualization table fields must be unique",
        })
    }
}

impl std::error::Error for VizSpecError {}

impl StructuredError for VizSpecError {
    purrdf_hash::error_visit! {
        EmptyGraphSelector { index } => "rdf-viz-empty-graph-selector" {
            "index" => Unsigned(*index as u128), "selector" => Text(""),
        },
        DuplicateTableField { field, first_index, index, actual, unique }
            => "rdf-viz-duplicate-table-field" {
            "field" => Text(field.label()),
            "first_index" => Unsigned(*first_index as u128),
            "index" => Unsigned(*index as u128),
            "actual" => Unsigned(*actual as u128), "unique" => Unsigned(*unique as u128),
        },
    }
}
