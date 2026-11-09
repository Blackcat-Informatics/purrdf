// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicit positive-world retention and consumer replay authority.

use purrdf_core::{DatasetStateError, DrainFailure};

/// Caller-selected aggregate ceilings. No producer can raise consumer limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PositiveWorldLimits {
    /// Source and syntactic preparation steps, including empty preparations.
    pub history_steps: usize,
    /// Distinct source bindings, including promoted query symbols.
    pub source_terms: usize,
    /// All statement families and named declarations together.
    pub source_rows: usize,
    /// Total encoded prior questions.
    pub question_bytes: usize,
    /// Live retained alias and preparation metadata bytes.
    pub metadata_bytes: usize,
    /// Metadata bytes, source validation rows/bindings and preparation dispatches.
    /// Native canonicalization, schema and key work retain their original
    /// controls; this is not a total native reasoning-work ceiling.
    pub metadata_work: u64,
}

/// A source, preparation, verification or explicit retention refusal.
#[derive(Debug)]
#[non_exhaustive]
pub enum PositiveWorldError<E, V> {
    /// Original native reasoning cause.
    Entail(crate::EntailError<E>),
    /// Exact complete-state cause and operational evidence.
    State(DatasetStateError<E, V>),
    /// Exact completed-read checkpoint refusal and evidence.
    Drain(DrainFailure<E, V>),
    /// Original native proof refusal.
    Proof(crate::DlProofError<E>),
    /// Caller-selected aggregate ceiling refused an operation.
    Limit {
        /// The resource whose ceiling was exceeded.
        resource: &'static str,
        /// Requested aggregate value.
        attempted: u128,
        /// Caller-selected ceiling.
        limit: u128,
    },
}

purrdf_lex::variant_from!(impl<E, V> PositiveWorldError<E, V> { Entail(crate::EntailError<E>) });

impl<E: std::fmt::Display, V> std::fmt::Display for PositiveWorldError<E, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Entail(error) => error.fmt(f),
            Self::State(error) => error.fmt(f),
            Self::Drain(error) => write!(
                f,
                "positive-world {:?} checkpoint failed: {}",
                error.checkpoint, error.error
            ),
            Self::Proof(error) => error.fmt(f),
            Self::Limit {
                resource,
                attempted,
                limit,
            } => write!(
                f,
                "positive-world {resource} requires {attempted}; caller limit is {limit}"
            ),
        }
    }
}

impl<E: std::error::Error + 'static, V: std::fmt::Debug + 'static> std::error::Error
    for PositiveWorldError<E, V>
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Entail(error) => Some(error),
            Self::State(error) => Some(error),
            Self::Drain(failure) => Some(&failure.error),
            Self::Proof(error) => Some(error),
            Self::Limit { .. } => None,
        }
    }
}

impl<E: purrdf_hash::error::StructuredError, V: purrdf_hash::error::StructuredError>
    purrdf_hash::error::StructuredError for PositiveWorldError<E, V>
{
    purrdf_hash::error_visit! {
        Entail(error) => "positive-world-entailment" { "cause" => Cause(error) },
        State(error) => "positive-world-state" { "cause" => Cause(error) },
        Drain(failure) => "positive-world-drain" { "cause" => Cause(failure) },
        Proof(error) => "positive-world-proof" { "cause" => Cause(error) },
        Limit { resource, attempted, limit } => "positive-world-limit" {
            "resource" => Code(resource), "attempted" => Unsigned(*attempted), "limit" => Unsigned(*limit),
        },
    }
}
