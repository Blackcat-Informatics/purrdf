// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The source rows a write-only view locates its losses at, and their stable
//! identifiers.
//!
//! The curated views (CSVW terms, OKF terms, OBO Graphs, SKOS) each name the
//! source quad, reifier or annotation a loss concerns by a
//! [`stable_identifier`] over the row's canonical JSON. The row shapes and that
//! JSON live here once, so every view names the same row with the same bytes.

use super::{ProjectionError, ProjectionTerm, stable_identifier};
use purrdf_lex::json::record::{ToJson, to_vec};

/// One source quad: its terms and, outside the default graph, its graph name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SourceQuad {
    pub(crate) subject: ProjectionTerm,
    pub(crate) predicate: String,
    pub(crate) object: ProjectionTerm,
    pub(crate) graph: Option<ProjectionTerm>,
}

/// One source reifier binding: the reifier, the triple term it reifies, and
/// its graph.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SourceReifier {
    pub(crate) reifier: ProjectionTerm,
    pub(crate) statement: ProjectionTerm,
    pub(crate) graph: Option<ProjectionTerm>,
}

/// One source annotation on a reifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SourceAnnotation {
    pub(crate) reifier: ProjectionTerm,
    pub(crate) predicate: String,
    pub(crate) object: ProjectionTerm,
    pub(crate) graph: Option<ProjectionTerm>,
}

impl SourceQuad {
    /// The node terms: subject, object and, outside the default graph, the
    /// graph name.
    pub(crate) fn node_terms(&self) -> impl Iterator<Item = &ProjectionTerm> {
        [&self.subject, &self.object].into_iter().chain(&self.graph)
    }
}

impl SourceReifier {
    /// The node terms: reifier, reified triple term and graph name.
    pub(crate) fn node_terms(&self) -> impl Iterator<Item = &ProjectionTerm> {
        [&self.reifier, &self.statement]
            .into_iter()
            .chain(&self.graph)
    }
}

impl SourceAnnotation {
    /// The node terms: reifier, object and graph name.
    pub(crate) fn node_terms(&self) -> impl Iterator<Item = &ProjectionTerm> {
        [&self.reifier, &self.object].into_iter().chain(&self.graph)
    }
}

/// The stable identifier of a source row: `prefix`, then the SHA-256 of the
/// row's compact JSON (members in declaration order, an absent graph `null`).
pub(crate) fn source_identifier(
    prefix: &str,
    value: &impl ToJson,
) -> Result<String, ProjectionError> {
    stable_identifier(prefix, &to_vec(value))
}

purrdf_lex::json_record!(impl ToJson for SourceQuad {
    "subject" => subject,
    "predicate" => predicate,
    "object" => object,
    "graph" => graph,
});

purrdf_lex::json_record!(impl ToJson for SourceReifier {
    "reifier" => reifier,
    "statement" => statement,
    "graph" => graph,
});

purrdf_lex::json_record!(impl ToJson for SourceAnnotation {
    "reifier" => reifier,
    "predicate" => predicate,
    "object" => object,
    "graph" => graph,
});
