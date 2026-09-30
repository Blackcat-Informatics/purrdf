// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The source rows a write-only view locates its losses at, and their stable
//! identifiers.
//!
//! The curated views (CSVW terms, OKF terms, OBO Graphs, SKOS) each name the
//! source quad, reifier or annotation a loss concerns by a
//! [`stable_identifier`] over the row's canonical JSON. The row shapes and that
//! JSON live here once, so every view names the same row with the same bytes.

use purrdf_lex::json::{Object, Value};

use super::json_codec::{ToJson, to_vec};
use super::{ProjectionError, ProjectionTerm, stable_identifier};

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

/// The stable identifier of a source row: `prefix`, then the SHA-256 of the
/// row's compact JSON (members in declaration order, an absent graph `null`).
pub(crate) fn source_identifier(
    prefix: &str,
    value: &impl ToJson,
) -> Result<String, ProjectionError> {
    stable_identifier(prefix, &to_vec(value))
}

impl ToJson for SourceQuad {
    fn to_json(&self) -> Value {
        Value::Object(
            Object::new()
                .with("subject", self.subject.to_json())
                .with("predicate", self.predicate.as_str())
                .with("object", self.object.to_json())
                .with("graph", self.graph.to_json()),
        )
    }
}

impl ToJson for SourceReifier {
    fn to_json(&self) -> Value {
        Value::Object(
            Object::new()
                .with("reifier", self.reifier.to_json())
                .with("statement", self.statement.to_json())
                .with("graph", self.graph.to_json()),
        )
    }
}

impl ToJson for SourceAnnotation {
    fn to_json(&self) -> Value {
        Value::Object(
            Object::new()
                .with("reifier", self.reifier.to_json())
                .with("predicate", self.predicate.as_str())
                .with("object", self.object.to_json())
                .with("graph", self.graph.to_json()),
        )
    }
}
