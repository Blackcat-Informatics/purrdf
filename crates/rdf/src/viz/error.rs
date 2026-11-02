// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrowed visualization fields and original codec refusal owners.

use std::fmt;

use purrdf_core::xml_escape::InvalidXmlChar;
use purrdf_hash::error::StructuredError;
use purrdf_lex::json::record::DecodeError;

use super::{
    VizPredicateError, VizPresentationError, VizPresentationIssue, VizSpecError, VizVocabularyError,
};

/// Visualization projection errors.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum VizError {
    /// The requested projection exceeds explicit size limits.
    TooLarge {
        /// Limit name.
        limit: &'static str,
        /// Actual count.
        actual: usize,
        /// Allowed count.
        allowed: usize,
    },
    /// A spec references an unknown role predicate.
    UnknownRolePredicate(String),
    /// A spec contains an invalid vocabulary mapping.
    InvalidVocabulary(VizVocabularyError),
    /// A visualization specification is internally inconsistent.
    InvalidSpec(VizSpecError),
    /// A graph-like input contains an invalid predicate.
    InvalidPredicate(VizPredicateError),
    /// A focus selector does not match any projected term or statement.
    UnknownFocus(String),
    /// A focus selector matches more than one projected entity.
    AmbiguousFocus(String),
    /// A deterministic identity hash collided with a different structural key.
    IdCollision(String),
    /// A renderer-neutral scene is structurally invalid.
    Scene(String),
    /// Deterministic layout could not satisfy its structural invariants.
    Layout(String),
    /// Original XML character refusal from SVG serialization.
    Serialize(InvalidXmlChar),
    /// Original native JSON record refusal and private diagnostic fields.
    Decode(DecodeError),
    /// A caller-owned presentation instruction is invalid or unsupported.
    Presentation(VizPresentationError),
}

impl fmt::Display for VizError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge {
                limit,
                actual,
                allowed,
            } => write!(
                f,
                "visualization {limit} limit exceeded: {actual} > {allowed}"
            ),
            Self::UnknownRolePredicate(iri) => {
                write!(f, "visualization role predicate {iri:?} is not present")
            }
            Self::InvalidVocabulary(cause) => cause.fmt(f),
            Self::InvalidSpec(cause) => cause.fmt(f),
            Self::Scene(message) | Self::Layout(message) => f.write_str(message),
            Self::InvalidPredicate(cause) => cause.fmt(f),
            Self::UnknownFocus(focus) => {
                write!(
                    f,
                    "visualization focus {focus:?} does not match the projection"
                )
            }
            Self::AmbiguousFocus(focus) => write!(f, "visualization focus {focus:?} is ambiguous"),
            Self::IdCollision(id) => {
                write!(f, "visualization structural identity collision for {id}")
            }
            Self::Serialize(cause) => cause.fmt(f),
            Self::Decode(cause) => write!(f, "visualization JSON: {cause}"),
            Self::Presentation(cause) => cause.fmt(f),
        }
    }
}

purrdf_hash::error_sources!(
    VizError,
    InvalidVocabulary,
    InvalidSpec,
    InvalidPredicate,
    Serialize,
    Decode,
    Presentation
);

impl StructuredError for VizError {
    purrdf_hash::error_visit! {
        TooLarge { limit, actual, allowed } => "rdf-viz-too-large" {
            "limit" => Code(limit), "actual" => Unsigned(*actual as u128),
            "allowed" => Unsigned(*allowed as u128),
        },
        UnknownRolePredicate(predicate) => "rdf-viz-unknown-role-predicate" { "predicate" => Text(predicate) },
        InvalidVocabulary(cause) => "rdf-viz-invalid-vocabulary" { "cause" => Cause(cause) },
        InvalidSpec(cause) => "rdf-viz-invalid-specification" { "cause" => Cause(cause) },
        InvalidPredicate(cause) => "rdf-viz-invalid-predicate" { "cause" => Cause(cause) },
        UnknownFocus(focus) => "rdf-viz-unknown-focus" { "focus" => Text(focus) },
        AmbiguousFocus(focus) => "rdf-viz-ambiguous-focus" { "focus" => Text(focus) },
        IdCollision(identity) => "rdf-viz-identity-collision" { "identity" => Text(identity) },
        Scene(message) => "rdf-viz-scene" { "message" => Message(message) },
        Layout(message) => "rdf-viz-layout" { "message" => Message(message) },
        Serialize(cause) => "rdf-viz-xml" { "cause" => Cause(cause) },
        Decode(cause) => "rdf-viz-json-decode" { "cause" => Cause(cause) },
        Presentation(cause) => "rdf-viz-presentation" { "cause" => Cause(cause) },
    }
}

impl fmt::Display for VizPresentationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "visualization presentation {:?} at {}: {}",
            self.issue, self.instruction, self.message
        )
    }
}

impl std::error::Error for VizPresentationError {}

impl StructuredError for VizPresentationError {
    purrdf_hash::error_visit! {
        @record self { issue, instruction, message } => "rdf-viz-presentation-refusal" {
            "issue" => Code(match self.issue {
                VizPresentationIssue::Binding => "binding",
                VizPresentationIssue::Containment => "containment",
                VizPresentationIssue::Scenario => "scenario",
                VizPresentationIssue::Navigation => "navigation",
                VizPresentationIssue::Unsupported => "unsupported",
                VizPresentationIssue::Export => "export",
            }),
            "instruction" => Text(&self.instruction), "message" => Message(&self.message),
        }
    }
}
