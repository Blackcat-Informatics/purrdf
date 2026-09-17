// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The executor: each admitted unit runs independently through the evaluator.
//!
//! [`execute`] is the only stage that runs a query. It runs one unit per stratum
//! through `purrdf-sparql-eval` over an empty default graph, with the caller's
//! registry injected: the relations are row sources reached from predicate
//! position, so their rows do not depend on any stored data, and the query text is
//! exactly the admitted emission. A unit that fails to parse or evaluate becomes
//! that stratum's [`ProducerStatus::ExecutionFailed`] while every other stratum
//! streams on — the isolation requirement a single monolithic query could not
//! meet.
//!
//! # Ranks are the evaluator's emission order
//!
//! Each successful unit's solution rows become candidates in emission order, with
//! **1-based** ranks. The evaluator preserves a relation's declared emission
//! order, so the candidate set and per-stratum ranks are a pure function of the
//! plan and the registry — a pinned plan replays identically.
//!
//! # A raw stream, not yet a fusion stream
//!
//! [`RankedStreamImpl`] carries `(rank, candidate)` only. A fusion contribution
//! depends on the fusion profile's weights and smoothing constant, which are
//! deliberately not a plan input, so the contribution is attached at `fuse` time.
//! Keeping the executor profile-free is what lets the unfused rung be consumed
//! without bound.

use std::collections::{HashMap, VecDeque};

use purrdf_core::{RdfDatasetBuilder, SparqlRequest, SparqlResult, TermValue};
use purrdf_sparql_eval::{NativeSparqlEngine, PropertyFunctionRegistry, QueryOptions, RegistryId};

use crate::compile::CompiledRetrieval;
use crate::fusion_stream::ProducerStatus;
use crate::id::PlanId;
use crate::iri::{Iri, Term};
use crate::ranked_stream::{ProducerReceipt, ProtocolError};

/// One stratum's ranked rows, tagged with the pinned plan they descend from.
#[derive(Debug)]
pub struct StratumStream {
    /// The stratum the stream's ranks are within.
    pub stratum: Iri,
    /// The admitted plan the stream was compiled from.
    pub plan_id: PlanId,
    /// The evaluator's rows, in rank order.
    pub stream: RankedStreamImpl,
}

/// The result of running every compiled unit: the surviving streams and every
/// stratum's status.
///
/// A failed stratum appears in `statuses` as
/// [`ProducerStatus::ExecutionFailed`] and has no entry in `streams`; a surviving
/// stratum appears in both. Nothing reduces the statuses to one flag.
#[derive(Debug)]
pub struct ExecutionResult {
    /// One stream per stratum that ran, ordered as compiled.
    pub streams: Vec<StratumStream>,
    /// Every stratum's own terminal status.
    pub statuses: HashMap<Iri, ProducerStatus>,
}

/// A whole-execution failure, as distinct from a per-stratum one.
///
/// A per-stratum failure is *data* — a [`ProducerStatus::ExecutionFailed`] in
/// [`ExecutionResult::statuses`] — because the remaining strata must still run.
/// Only a defect that invalidates the run itself is an error here.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ExecutionError {
    /// A unit failed before it could be attributed to a stratum.
    #[error("evaluation failed: {0}")]
    EvalError(#[from] purrdf_sparql_eval::EvalError),

    /// A compiled unit is structurally invalid and cannot be run.
    #[error("stratum {stratum} carries an invalid compiled unit: {reason}")]
    InvalidUnit {
        /// The stratum the unit belongs to. Boxed because [`Iri`] carries five
        /// parsed spans; see [`AdmissionError`](crate::AdmissionError).
        stratum: Box<Iri>,
        /// Why the unit cannot be run.
        reason: String,
    },

    /// The compiled units were built against a different live registry instance.
    #[error("compiled units name registry instance {expected:?}, but execution holds {got:?}")]
    RegistryMismatch {
        /// The instance the compiled bundle records.
        expected: RegistryId,
        /// The instance execution was handed.
        got: RegistryId,
    },
}

/// A concrete ranked stream of `(rank, candidate)` rows.
///
/// The rows are materialized by the evaluator and drained in order; `next` never
/// pends, so the stream is usable under any executor. A caller that wants to fuse
/// the rows wraps them with the fusion profile at `fuse` time.
#[derive(Debug)]
pub struct RankedStreamImpl {
    rows: VecDeque<(u64, Term)>,
    pulled: u64,
    exhausted: bool,
}

impl RankedStreamImpl {
    /// Build a stream over pre-ranked `(rank, candidate)` rows.
    #[must_use]
    pub fn new(rows: Vec<(u64, Term)>) -> Self {
        Self {
            rows: rows.into(),
            pulled: 0,
            exhausted: false,
        }
    }

    /// The next row, or `None` when the stream is exhausted.
    ///
    /// # Errors
    ///
    /// A [`ProtocolError`] if the stream cannot describe its own rows; this
    /// materialized stream never raises one.
    // The body is synchronous because the evaluator materializes its rows before
    // returning; the `async` shape is the ranked-stream contract the fusion stage
    // consumes, and a caller may compose it with genuinely asynchronous streams.
    #[allow(clippy::unused_async, clippy::unused_async_trait_impl)]
    pub async fn next(&mut self) -> Result<Option<(u64, Term)>, ProtocolError> {
        match self.rows.pop_front() {
            Some(row) => {
                self.pulled += 1;
                Ok(Some(row))
            }
            None => {
                self.exhausted = true;
                Ok(None)
            }
        }
    }

    /// How the stream ended. Call only after [`Self::next`] returned `None`.
    ///
    /// # Errors
    ///
    /// [`ProtocolError::NeverEndingSource`] when called before the stream was
    /// drained, because a completeness claim from a partially read stream is
    /// exactly the falsifiable status the protocol forbids.
    #[allow(clippy::unused_async, clippy::unused_async_trait_impl)]
    pub async fn receipt(&mut self) -> Result<ProducerReceipt, ProtocolError> {
        if !self.exhausted {
            return Err(ProtocolError::NeverEndingSource);
        }
        Ok(ProducerReceipt::Exhausted {
            rows_emitted: self.pulled,
        })
    }

    /// Consume the stream and return its rows.
    #[must_use]
    pub fn into_rows(self) -> Vec<(u64, Term)> {
        self.rows.into_iter().collect()
    }
}

/// Run every compiled unit independently through `purrdf-sparql-eval`.
///
/// Each unit is evaluated over an empty default graph with `registry` injected as
/// the property-function registry; the relations are row sources, so no dataset is
/// needed. A unit that fails is recorded as that stratum's
/// [`ProducerStatus::ExecutionFailed`] and contributes no stream, while every
/// other unit runs to completion.
///
/// # Errors
///
/// [`ExecutionError::RegistryMismatch`] when `compiled` was built against a
/// different live registry instance than `registry`. Per-stratum failures are not
/// errors; they are reported in [`ExecutionResult::statuses`].
// The executor drives a synchronous evaluator and returns materialized streams;
// the `async` shape is its stage contract, not a pending future. A caller composes
// it with the asynchronous fusion stage.
#[allow(clippy::unused_async, clippy::unused_async_trait_impl)]
pub async fn execute(
    compiled: &CompiledRetrieval,
    registry: &PropertyFunctionRegistry,
) -> Result<ExecutionResult, ExecutionError> {
    if compiled.registry_id != registry.instance_id() {
        return Err(ExecutionError::RegistryMismatch {
            expected: compiled.registry_id,
            got: registry.instance_id(),
        });
    }

    // An empty default graph: relations are row sources, so the stored data is
    // irrelevant and the query text is exactly what the plan produced.
    let dataset = RdfDatasetBuilder::new()
        .freeze()
        .expect("an empty default graph is structurally valid");

    let engine = NativeSparqlEngine::new();
    let mut streams = Vec::with_capacity(compiled.units.len());
    let mut statuses = HashMap::with_capacity(compiled.units.len());

    for unit in &compiled.units {
        if unit.sparql.trim().is_empty() {
            statuses.insert(
                unit.stratum.clone(),
                ProducerStatus::ExecutionFailed {
                    reason: "compiled unit is empty".to_owned(),
                },
            );
            continue;
        }
        let outcome = engine.query_with_options_view(
            &*dataset,
            SparqlRequest {
                query: &unit.sparql,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions {
                property_functions: registry,
                ..QueryOptions::EMPTY
            },
        );
        match outcome {
            Ok(SparqlResult::Solutions { rows, .. }) => {
                let ranked: Vec<(u64, Term)> = rows
                    .iter()
                    .filter_map(|row| row.iter().find_map(Option::as_ref))
                    .enumerate()
                    .map(|(index, value)| {
                        (
                            u64::try_from(index + 1).unwrap_or(u64::MAX),
                            term_candidate(value),
                        )
                    })
                    .collect();
                let rows_emitted = u64::try_from(ranked.len()).unwrap_or(u64::MAX);
                streams.push(StratumStream {
                    stratum: unit.stratum.clone(),
                    plan_id: compiled.plan_id,
                    stream: RankedStreamImpl::new(ranked),
                });
                statuses.insert(
                    unit.stratum.clone(),
                    ProducerStatus::Exhausted { rows_emitted },
                );
            }
            Ok(_) => {
                statuses.insert(
                    unit.stratum.clone(),
                    ProducerStatus::ExecutionFailed {
                        reason: "compiled unit did not return solutions".to_owned(),
                    },
                );
            }
            Err(diagnostic) => {
                statuses.insert(
                    unit.stratum.clone(),
                    ProducerStatus::ExecutionFailed {
                        reason: diagnostic.to_string(),
                    },
                );
            }
        }
    }

    Ok(ExecutionResult { streams, statuses })
}

/// A candidate's canonical term text: the hex of the evaluator value's injective
/// canonical byte encoding.
///
/// The composition layer mints no vocabulary and parses no RDF, so it does not
/// re-render a term as Turtle or N-Triples. Hexing
/// [`TermValue::to_canonical_bytes`] yields a deterministic, injective, target-
/// independent name for exactly the value the evaluator produced, and distinct
/// values can never share one.
fn term_candidate(value: &TermValue) -> Term {
    use core::fmt::Write as _;

    let bytes = value.to_canonical_bytes();
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    Term::new(out)
}
