// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The executor: each admitted unit runs independently through the evaluator.
//!
//! [`execute`] is the only stage that runs a query. It runs one unit per stratum
//! through `purrdf-sparql-eval` against **the caller's dataset**, with the
//! caller's registry injected, and the query text is exactly the admitted
//! emission. A unit that fails to parse or evaluate becomes that stratum's
//! [`ProducerStatus::ExecutionFailed`] while every other stratum streams on — the
//! isolation requirement a single monolithic query could not meet.
//!
//! # The dataset is the caller's, and it is read
//!
//! A unit is ordinary SPARQL: its ranked relations are reached from predicate
//! position, but every other pattern in it is matched against the stored data
//! like any other query. The dataset is therefore a parameter, taken by reference
//! and never built here — a retrieval layer that searched a graph of its own
//! choosing would answer about data nobody asked about. `execute` is generic over
//! [`DatasetView`] rather than taking a trait object because the view has
//! associated types and is not object-safe; the evaluator entry point it calls is
//! generic for the same reason.
//!
//! # Ranks are the evaluator's emission order
//!
//! Each successful unit's solution rows become candidates in emission order, with
//! **1-based** ranks. The evaluator preserves a relation's declared emission
//! order, so the candidate set and per-stratum ranks are a pure function of the
//! plan, the registry and the dataset — a pinned plan replays identically against
//! the same data.
//!
//! # A raw stream, not yet a fusion stream
//!
//! [`RankedStreamImpl`] carries `(rank, candidate)` only. A fusion contribution
//! depends on the fusion profile's weights and smoothing constant, which are
//! deliberately not a plan input, so the contribution is attached at `fuse` time.
//! Keeping the executor profile-free is what lets the unfused rung be consumed
//! without bound.

use std::collections::{HashMap, VecDeque};

use purrdf_core::{DatasetView, SparqlRequest, SparqlResult, TermValue};
use purrdf_sparql_eval::{NativeSparqlEngine, PropertyFunctionRegistry, QueryOptions, RegistryId};

use crate::compile::{CANDIDATE_NAME, CompiledRetrieval};
use crate::fusion_stream::ProducerStatus;
use crate::id::PlanId;
use crate::iri::{Iri, Term};
use crate::ranked_stream::{ProducerReceipt, ProtocolError};
use crate::render::candidate_lexical;

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

/// Run every compiled unit independently through `purrdf-sparql-eval` against
/// `dataset`.
///
/// Each unit is evaluated over the caller's `dataset` with `registry` injected as
/// the property-function registry. A unit that fails is recorded as that
/// stratum's [`ProducerStatus::ExecutionFailed`] and contributes no stream, while
/// every other unit runs to completion.
///
/// # Errors
///
/// [`ExecutionError::RegistryMismatch`] when `compiled` was built against a
/// different live registry instance than `registry`. Per-stratum failures are not
/// errors; they are reported in [`ExecutionResult::statuses`].
// The executor drives a synchronous evaluator and returns materialized streams;
// the `async` shape is its stage contract, not a pending future. A caller composes
// it with the asynchronous fusion stage.
//
// `future_not_send`: the dataset is a caller-chosen type parameter, so the
// returned future's `Send`-ness is the caller's to establish. Requiring it here
// would force every caller's view — and its statistics and environment, through
// `search` — to be `Send` for a future that is awaited in one task and never
// crosses a thread boundary. `search` carries the same reasoning.
#[allow(
    clippy::unused_async,
    clippy::unused_async_trait_impl,
    clippy::future_not_send
)]
pub async fn execute<D: DatasetView + Sync>(
    compiled: &CompiledRetrieval,
    registry: &PropertyFunctionRegistry,
    dataset: &D,
) -> Result<ExecutionResult, ExecutionError> {
    if compiled.registry_id != registry.instance_id() {
        return Err(ExecutionError::RegistryMismatch {
            expected: compiled.registry_id,
            got: registry.instance_id(),
        });
    }

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
            dataset,
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
            Ok(SparqlResult::Solutions {
                variables, rows, ..
            }) => match rank_candidates(&variables, &rows) {
                Ok(ranked) => {
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
                Err(reason) => {
                    statuses.insert(
                        unit.stratum.clone(),
                        ProducerStatus::ExecutionFailed {
                            reason: format!("stratum {}: {reason}", unit.stratum),
                        },
                    );
                }
            },
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

/// Read a unit's projected candidate column into ranked `(rank, candidate)` rows.
///
/// # The column is found by name
///
/// A unit projects exactly one variable, `?candidate`, so the column is located
/// by that name. Reading a row's first *bound* cell instead would make the
/// candidate depend on projection order, and a unit that projected anything
/// alongside the candidate would silently rank the wrong term.
///
/// # No row is ever dropped
///
/// A rank is a position in the stratum's answer, so discarding a row renumbers
/// every row after it: the stratum would report a shorter, differently-ranked
/// list that still looked complete. An unbound cell and an unrenderable value are
/// therefore both refusals of the whole unit, returned as the reason the caller
/// records as this stratum's [`ProducerStatus::ExecutionFailed`].
fn rank_candidates(
    variables: &[String],
    rows: &[Vec<Option<TermValue>>],
) -> Result<Vec<(u64, Term)>, String> {
    let column = variables
        .iter()
        .position(|name| name == CANDIDATE_NAME)
        .ok_or_else(|| {
            format!("the unit's solutions project no ?{CANDIDATE_NAME} column: {variables:?}")
        })?;
    let mut ranked = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        let rank = u64::try_from(index + 1).unwrap_or(u64::MAX);
        let value = row.get(column).and_then(Option::as_ref).ok_or_else(|| {
            format!("the projected ?{CANDIDATE_NAME} column is unbound in row {rank}")
        })?;
        ranked.push((rank, term_candidate(value)));
    }
    Ok(ranked)
}

/// A candidate's canonical term lexical — exactly the spelling a caller uses for
/// a seed.
///
/// [`candidate_lexical`] writes `<http://example.org/doc>`, `"lex"@en` or the RDF 1.2
/// triple term `<<( s p o )>>`: deterministic, injective, target-independent, and
/// invertible by the decoder that lives beside it. A candidate is therefore
/// spelled exactly as [`RequestTerm::EntitySeed`](crate::RequestTerm::EntitySeed)
/// spells a seed, so a row this layer returns can be fed straight back in as the
/// seed of a follow-up request.
///
/// A blank node is named `_:label` rather than refused. Its label is
/// dataset-local, so it cannot seed a *later* request — but that is refused at
/// placement, where a caller actually tries it, and the decoder reads `_:label`
/// back either way. Refusing here would discard every other row in the stratum
/// over one answer the layer merely declined to write down.
fn term_candidate(value: &TermValue) -> Term {
    Term::new(candidate_lexical(value))
}

#[cfg(test)]
mod tests {
    use purrdf_core::TermValue;

    use super::{rank_candidates, term_candidate};
    use crate::compile::CANDIDATE_NAME;
    use crate::render::decode_term;

    fn variables() -> Vec<String> {
        vec![CANDIDATE_NAME.to_owned()]
    }

    #[test]
    fn a_candidate_is_the_canonical_lexical_and_decodes_back_to_its_value() {
        for value in [
            TermValue::iri("http://example.org/doc"),
            TermValue::simple_literal("quick brown fox"),
            TermValue::typed_literal("3", "http://www.w3.org/2001/XMLSchema#integer"),
            TermValue::Triple {
                s: Box::new(TermValue::iri("http://example.org/s")),
                p: Box::new(TermValue::iri("http://example.org/p")),
                o: Box::new(TermValue::simple_literal("o")),
            },
        ] {
            let candidate = term_candidate(&value);
            assert_eq!(
                decode_term(candidate.as_str()),
                Ok(value),
                "the candidate {} reads back as the value it names",
                candidate.as_str()
            );
        }
        assert_eq!(
            term_candidate(&TermValue::iri("http://example.org/doc")).as_str(),
            "<http://example.org/doc>",
            "an IRI candidate is legible, not an encoding of one"
        );
    }

    /// A blank node is a perfectly ordinary answer. It cannot seed a *later*
    /// request, because its label is dataset-local — but that is refused at
    /// placement, where a caller actually tries it. Refusing it here would throw
    /// away every other row in the stratum over one answer this layer merely
    /// declined to write down, which is the silent-drop bug wearing strictness.
    #[test]
    fn a_blank_candidate_is_named_and_does_not_take_its_stratum_down() {
        let ranked = rank_candidates(
            &variables(),
            &[
                vec![Some(TermValue::iri("http://example.org/doc"))],
                vec![Some(TermValue::blank("b0"))],
                vec![Some(TermValue::iri("http://example.org/other"))],
            ],
        )
        .expect("a blank node among the answers is still an answer");

        assert_eq!(
            ranked
                .iter()
                .map(|(_, term)| term.as_str())
                .collect::<Vec<_>>(),
            vec![
                "<http://example.org/doc>",
                "_:b0",
                "<http://example.org/other>"
            ],
            "the blank node is named in place, and its neighbours keep their ranks"
        );
        assert_eq!(
            ranked.iter().map(|(rank, _)| *rank).collect::<Vec<_>>(),
            vec![1, 2, 3],
            "ranks stay 1-based and contiguous, so no row was dropped"
        );
        assert_eq!(
            decode_term("_:b0"),
            Ok(TermValue::blank("b0")),
            "and the decoder reads the label back, so nothing is lost"
        );
    }

    #[test]
    fn the_candidate_column_is_read_by_name_not_by_first_binding() {
        // A row whose earlier column is bound and whose candidate column is not
        // is a refusal, never the earlier column's term promoted into the rank.
        let variables = vec!["other".to_owned(), CANDIDATE_NAME.to_owned()];
        let rows = vec![vec![Some(TermValue::iri("http://example.org/other")), None]];
        let reason = rank_candidates(&variables, &rows)
            .expect_err("an unbound candidate column is a refusal");
        assert!(reason.contains("unbound"), "{reason}");

        // Bound in the candidate column, it is that column that ranks.
        let rows = vec![vec![
            Some(TermValue::iri("http://example.org/other")),
            Some(TermValue::iri("http://example.org/doc")),
        ]];
        assert_eq!(
            rank_candidates(&variables, &rows).expect("the candidate column ranks"),
            vec![(
                1,
                crate::iri::Term::new("<http://example.org/doc>".to_owned())
            )]
        );
    }

    #[test]
    fn a_unit_that_projects_no_candidate_column_is_refused() {
        let reason = rank_candidates(&["other".to_owned()], &[vec![None]])
            .expect_err("a unit with no candidate column cannot be ranked");
        assert!(reason.contains(CANDIDATE_NAME), "{reason}");
    }
}
