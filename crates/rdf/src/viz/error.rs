// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrowed visualization fields and original codec refusal owners.

use std::fmt;

use purrdf_core::xml_escape::InvalidXmlChar;
use purrdf_hash::error::{ErrorValue, StructuredError};
use purrdf_lex::json::record::DecodeError;

use super::{
    VizPredicateError, VizPresentationError, VizPresentationIssue, VizSpecError, VizStatementId,
    VizTermId, VizVocabularyError,
};

/// The original sorted candidate identities for an ambiguous focus selector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VizFocusAmbiguity {
    /// Original caller-selected focus spelling.
    pub focus: String,
    /// Matching term identities, in the producer's sorted/deduplicated order.
    pub term_matches: Vec<VizTermId>,
    /// Matching statement identities, in the producer's sorted/deduplicated order.
    pub statement_matches: Vec<VizStatementId>,
}

impl fmt::Display for VizFocusAmbiguity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "visualization focus {:?} is ambiguous", self.focus)
    }
}

impl std::error::Error for VizFocusAmbiguity {}

impl StructuredError for VizFocusAmbiguity {
    purrdf_hash::error_visit! {
        @record self { focus, term_matches, statement_matches } => "rdf-viz-focus-ambiguity" {
            "focus" => Text(&self.focus),
            "term_matches" => List(self.term_matches.iter().map(|id| ErrorValue::Text(&id.0))),
            "statement_matches" => List(self.statement_matches.iter().map(|id| ErrorValue::Text(&id.0))),
        }
    }
}

/// Original comparison operands at an identity registration conflict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VizIdentityCollision {
    /// The already computed visualization identity.
    pub identity: String,
    /// Original namespace prefix supplied to the identity producer.
    pub prefix: String,
    /// Original raw key already registered for the identity.
    pub existing_key: String,
    /// Original raw key refused by this registration attempt.
    pub incoming_key: String,
}

impl fmt::Display for VizIdentityCollision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "visualization structural identity collision for {}",
            self.identity
        )
    }
}

impl std::error::Error for VizIdentityCollision {}

impl StructuredError for VizIdentityCollision {
    purrdf_hash::error_visit! {
        @record self { identity, prefix, existing_key, incoming_key } => "rdf-viz-identity-conflict" {
            "identity" => Text(&self.identity), "prefix" => Text(&self.prefix),
            "existing_key" => Text(&self.existing_key), "incoming_key" => Text(&self.incoming_key),
        }
    }
}

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
    AmbiguousFocus(VizFocusAmbiguity),
    /// A deterministic identity hash collided with a different structural key.
    IdCollision(VizIdentityCollision),
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
            Self::AmbiguousFocus(cause) => cause.fmt(f),
            Self::IdCollision(cause) => cause.fmt(f),
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
    AmbiguousFocus,
    IdCollision,
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
        AmbiguousFocus(cause) => "rdf-viz-ambiguous-focus" { "cause" => Cause(cause) },
        IdCollision(cause) => "rdf-viz-identity-collision" { "cause" => Cause(cause) },
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
