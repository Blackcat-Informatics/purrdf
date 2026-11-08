// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Retained expression selection failures shared by native and host callers.

use crate::term::{NamedNode, Term};
use std::fmt;

/// Why a selector names no single expression.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExprSelectorError {
    /// The native expression graph could not admit its original storage.
    Storage(purrdf_lex::allocation::StorageError),
    /// The merged expression graph failed native construction.
    Construction(purrdf_core::RdfDiagnostic),
    /// A term the selector spells is not one: `role` names it (`expr`, `expr-at`,
    /// `expr-via 2`), the native cause says why.
    Term {
        /// Which of the selector's terms.
        role: String,
        /// Why it is not a term.
        cause: crate::free_expression::TermParseError,
    },
    /// The walk's start node is a literal or a triple term, which is the subject of no
    /// triple, so no predicate can reach anything from it.
    StartNotANode {
        /// The start node, as N-Triples.
        node: Term,
    },
    /// A walk with no predicate, which would select the start node itself — name that
    /// by selecting the node directly.
    EmptyPath,
    /// A walk predicate that is not an IRI.
    PredicateNotIri {
        /// The step, counted from 1.
        step: usize,
        /// The original non-IRI term.
        predicate: Term,
    },
    /// A step reached no value or several.
    NotOneValue {
        /// The step, counted from 1.
        step: usize,
        /// The node the step started from, as N-Triples.
        subject: Term,
        /// The step's predicate IRI.
        predicate: NamedNode,
        /// How many values the step reached.
        count: usize,
    },
    /// The inline expression is not a Turtle document.
    Turtle {
        /// Original caller fragment, before inherited prefix replay.
        input: String,
        /// Exact synthetic document supplied to native Turtle ingress.
        carrier: String,
        /// Byte offset at which the caller's fragment begins in the carrier.
        fragment_offset: usize,
        /// The Turtle codec's diagnostics, one per malformed statement.
        errors: crate::text_ingest::TextErrors,
    },
    /// The inline expression has no root blank node, or several.
    NotOneRoot {
        /// How many roots the fragment has.
        count: usize,
    },
    /// A host named no selector, or several: the selector constructor takes exactly
    /// one of the node, the walk and the inline expression.
    NotOneSelector {
        /// How many of the three were given.
        count: usize,
    },
    /// Walk predicates given with no walk start.
    ViaWithoutAt,
}

impl fmt::Display for ExprSelectorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(error) => error.fmt(f),
            Self::Term { role, cause } => write!(f, "{role}: {cause}"),
            Self::Construction(cause) => write!(
                f,
                "expression selector: the inline expression is not a Turtle document: {cause}"
            ),
            Self::StartNotANode { node } => write!(
                f,
                "expression selector: the walk starts from {node}, which is not an IRI or a \
                 blank node and is the subject of no triple; start from the node that \
                 carries the expression"
            ),
            Self::EmptyPath => f.write_str(
                "expression selector: a walk from a node names no predicate to follow; name \
                 the node itself as the expression instead",
            ),
            Self::PredicateNotIri { step, predicate } => write!(
                f,
                "expression selector: step {step}'s predicate {predicate} is not an IRI"
            ),
            Self::NotOneValue {
                step,
                subject,
                predicate,
                count,
            } => write!(
                f,
                "expression selector: step {step}, {subject} {predicate}, reaches {count} \
                 values in the shapes graph; each step must reach exactly one"
            ),
            Self::Turtle { errors, .. } => write!(
                f,
                "expression selector: the inline expression is not a Turtle document: {}",
                errors
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
            Self::NotOneRoot { count } => write!(
                f,
                "expression selector: the inline expression has {count} root blank nodes \
                 (blank nodes that are the subject of a triple and the object of none); it \
                 must have exactly one, the expression. Name an IRI expression, or a \
                 constant, as the expression node itself"
            ),
            Self::NotOneSelector { count } => write!(
                f,
                "expression selector: {count} of the expression node, the walk start and the \
                 inline expression were given; name the expression exactly one way"
            ),
            Self::ViaWithoutAt => f.write_str(
                "expression selector: walk predicates were given with no walk start; name \
                 the node the walk starts from",
            ),
        }
    }
}

impl std::error::Error for ExprSelectorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Storage(cause) => Some(cause),
            Self::Construction(cause) => Some(cause),
            Self::Term { cause, .. } => Some(cause),
            Self::Turtle { errors, .. } => Some(errors),
            _ => None,
        }
    }
}

impl ExprSelectorError {
    /// Locate an inline diagnostic in the original caller fragment.
    /// Returns no position for another refusal or a diagnostic outside the fragment.
    /// The native cause and its carrier-relative location remain unchanged.
    ///
    /// # Errors
    /// Returns the original native position refusal if its column cannot fit.
    pub fn fragment_position(
        &self,
        diagnostic: usize,
    ) -> Result<Option<purrdf_iri::Position>, purrdf_iri::PositionError> {
        let Self::Turtle {
            input,
            fragment_offset,
            errors,
            ..
        } = self
        else {
            return Ok(None);
        };
        let Some(diagnostic) = errors.get(diagnostic) else {
            return Ok(None);
        };
        let Some(position) = diagnostic.source_position()? else {
            return Ok(None);
        };
        let Some(offset) = position.byte_offset.checked_sub(*fragment_offset) else {
            return Ok(None);
        };
        if offset > input.len() {
            return Ok(None);
        }
        let mut cursor = purrdf_iri::PositionCursor::new(input);
        match cursor.advance(offset, &mut purrdf_lex::control::Unmetered) {
            Ok(position) => Ok(Some(position)),
            Err(purrdf_lex::control::ControlledError::Native(cause)) => Err(cause),
            Err(purrdf_lex::control::ControlledError::Control(cause)) => match cause {},
        }
    }
}

purrdf_lex::variant_from!(ExprSelectorError { Storage(purrdf_lex::allocation::StorageError) });

impl purrdf_hash::error::StructuredError for ExprSelectorError {
    purrdf_hash::error_visit! {
        Storage(cause) => "expression-selector-storage" { "cause" => Cause(cause) },
        Construction(cause) => "expression-selector-construction" { "cause" => Cause(cause) },
        Term { role, cause } => "expression-selector-term" { "role" => Text(role), "cause" => Cause(cause) },
        StartNotANode { node } => "expression-selector-start-not-node" { "node" => Term(node) },
        EmptyPath => "expression-selector-empty-path" {},
        PredicateNotIri { step, predicate } => "expression-selector-predicate-not-iri" {
            "step" => Unsigned(*step as u128), "predicate" => Term(predicate),
        },
        NotOneValue { step, subject, predicate, count } => "expression-selector-not-one-value" {
            "step" => Unsigned(*step as u128), "subject" => Term(subject),
            "predicate" => Iri(predicate.as_str()), "count" => Unsigned(*count as u128),
        },
        Turtle { input, carrier, fragment_offset, errors } => "expression-selector-turtle" {
            "input" => Text(input), "carrier" => Text(carrier),
            "fragment_offset" => Unsigned(*fragment_offset as u128), "cause" => Cause(errors),
        },
        NotOneRoot { count } => "expression-selector-not-one-root" { "count" => Unsigned(*count as u128) },
        NotOneSelector { count } => "expression-selector-not-one-selector" { "count" => Unsigned(*count as u128) },
        ViaWithoutAt => "expression-selector-via-without-at" {},
    }
}
