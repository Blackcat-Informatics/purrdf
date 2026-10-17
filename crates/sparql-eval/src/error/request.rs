// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native query policy and forwarding refusals.

use super::{NativeDiagnosticKind, RetainedNativeFailure};
use crate::{EvalError, WorkspaceCapability};
use purrdf_lex::allocation::SharedText;

#[derive(Debug)]
pub(crate) enum RequestRefusal {
    DatasetRequired,
    ReducedRequired,
    GroupRequired,
    GroupOutputCollision { variable: SharedText, index: usize },
    UnknownVersion { label: SharedText },
    ForwardedPropertyFunction,
    ForwardedAggregate,
}

impl RequestRefusal {
    pub(crate) fn error(self, workspace: &WorkspaceCapability) -> EvalError {
        let kind = match &self {
            Self::DatasetRequired
            | Self::ReducedRequired
            | Self::GroupRequired
            | Self::GroupOutputCollision { .. } => NativeDiagnosticKind::Config,
            Self::UnknownVersion { .. }
            | Self::ForwardedPropertyFunction
            | Self::ForwardedAggregate => NativeDiagnosticKind::UnclassifiedUnsupported,
        };
        RetainedNativeFailure::error(kind, self, workspace)
    }
}

impl purrdf_hash::error::StructuredError for RequestRefusal {
    purrdf_hash::error_visit! {
        DatasetRequired => "eval-policy-dataset-required" {},
        ReducedRequired => "eval-policy-reduced-required" {},
        GroupRequired => "eval-policy-group-required" {},
        GroupOutputCollision { variable, index } => "eval-group-output-collision" {
            "variable" => Text(variable.as_str()), "index" => Unsigned(*index as u128),
        },
        UnknownVersion { label } => "sparql-version-unknown" { "label" => Text(label.as_str()) },
        ForwardedPropertyFunction => "service-property-function-forwarded" {},
        ForwardedAggregate => "service-custom-aggregate-forwarded" {},
    }
}

impl std::error::Error for RequestRefusal {}

impl core::fmt::Display for RequestRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DatasetRequired => f.write_str(
                "GRAPH requires a dataset, but this query is scoped to a single graph"),
            Self::ReducedRequired => f.write_str("adjacent reduction requires a Reduced operand"),
            Self::GroupRequired => f.write_str("group mapping domain requires a Group operand"),
            Self::GroupOutputCollision { .. } => f.write_str(
                "aggregate output collides with another group output"),
            Self::UnknownVersion { label } => write!(f,
                "VERSION \"{label}\" is not a recognized SPARQL version (recognized: \"1.2\", \"1.2-basic\")"),
            Self::ForwardedPropertyFunction => f.write_str(
                "a property-function call inside a SERVICE body: the call would be forwarded as an \
                 ordinary triple pattern and matched against the remote endpoint's data, so the \
                 relation would never be invoked and the answer would be silently wrong"),
            Self::ForwardedAggregate => f.write_str(
                "a custom-aggregate call inside a SERVICE body: `AGG(<iri>, …)` would be forwarded as \
                 text the remote endpoint has no registered meaning for, so this engine's aggregate \
                 registry would never resolve the call and the answer would be silently wrong"),
        }
    }
}
