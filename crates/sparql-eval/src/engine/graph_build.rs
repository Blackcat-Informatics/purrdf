// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Transactional publication of typed CONSTRUCT graphs into an existing builder.

use purrdf_core::{RdfDatasetBuilder, ResourceDimension, ValidatedRdfDatasetBuilder};

use super::{
    Arc, Cow, DatasetView, FallibleDatasetView, FallibleSparqlError, GovernedEvidence,
    GovernorState, NativeSparqlEngine, PreparedQuery, Query, QueryOptions, RdfDiagnostic,
    ShaclPrebinding, TermValue, ViewOperationStatus, apply_query_options, certain_partial,
    check_plan_matches_relations, empty_result_for, eval_diagnostic_code, query_pattern,
};

/// Work performed when a complete typed graph is appended to its destination.
/// These counters describe the operation, never the graph's content identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphBuildStats {
    /// Distinct statements staged, including reifier and annotation rows.
    pub statements: usize,
    /// Terms in the temporary validated builder.
    pub staged_terms: usize,
    /// Term payload bytes in the temporary builder.
    pub staged_payload_bytes: usize,
    /// New term payload bytes copied into the destination.
    pub copied_payload_bytes: usize,
    /// Frozen intermediate datasets created before the destination append.
    pub intermediate_freezes: usize,
    /// Successful shared-operation resource evidence, when a governor was supplied.
    pub governors: Option<crate::GovernorEvidence>,
}

/// Why a transactional graph build published no statements.
#[derive(Debug)]
pub enum GraphBuildError {
    /// Admission, substitution, evaluation or RDF validation failed.
    Query(RdfDiagnostic),
    /// A shared operation governor stopped before complete publication.
    BudgetExhausted {
        /// The ceiling or stop signal that ended evaluation.
        tripped: crate::TrippedGovernor,
        /// The operation's accumulated resource evidence.
        evidence: Box<crate::GovernorEvidence>,
    },
}

/// A complete typed graph receipt paired with a fallible carrier's operation evidence.
pub type FallibleGraphBuildResult<E, V> =
    Result<(GraphBuildStats, GovernedEvidence<V>), FallibleSparqlError<E, GovernedEvidence<V>>>;

impl std::fmt::Display for GraphBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Query(diagnostic) => std::fmt::Display::fmt(diagnostic, f),
            Self::BudgetExhausted { tripped, .. } => {
                write!(f, "graph publication stopped: {tripped:?}")
            }
        }
    }
}

impl std::error::Error for GraphBuildError {}

impl From<RdfDiagnostic> for GraphBuildError {
    fn from(diagnostic: RdfDiagnostic) -> Self {
        Self::Query(diagnostic)
    }
}

impl NativeSparqlEngine {
    /// Evaluate a prepared CONSTRUCT and append its complete typed graph directly.
    ///
    /// Uses the ordinary CONSTRUCT template, blank freshness and projection-loss
    /// machinery. The destination receives no rows until evaluation and structural
    /// validation succeed. No intermediate dataset is frozen or serialized. Blank
    /// scopes carried by bindings are preserved. Minted blanks use a deterministic
    /// namespace disjoint from destination blanks, including previous appends;
    /// a caller's `bnode_mint_prefix` remains the prefix of that namespace.
    ///
    /// # Errors
    /// Refuses non-CONSTRUCT forms, registry mismatches and evaluation failures.
    /// On every error the destination remains untouched.
    pub fn construct_prepared_into_view<'d, D: DatasetView + Sync>(
        &'d self,
        dataset: &'d D,
        prepared: &PreparedQuery,
        substitutions: &[(String, TermValue)],
        options: QueryOptions<'d>,
        destination: &mut RdfDatasetBuilder,
    ) -> Result<GraphBuildStats, GraphBuildError> {
        let prefix = destination_mint_prefix(destination, options.bnode_mint_prefix);
        let options = QueryOptions {
            bnode_mint_prefix: prefix.as_deref().or(options.bnode_mint_prefix),
            ..options
        };
        let staged = self.stage_construct(dataset, prepared, substitutions, options, None)?;
        Ok(publish(staged, destination, None))
    }

    /// Build a complete typed CONSTRUCT under an existing operation governor.
    ///
    /// Charges the same WHERE work and distinct output statement count as ordinary
    /// governed CONSTRUCT. Truncation and cancellation publish nothing, so callers
    /// can keep accumulating a graph transaction without exposing partial answers.
    ///
    /// # Errors
    /// Returns the typed governor trip or ordinary query failure. The destination
    /// is untouched on either path.
    #[allow(
        clippy::too_many_arguments,
        reason = "mirrors the prepared operation entry with an explicit publication destination"
    )]
    pub fn construct_prepared_in_operation_into_view<'d, D: DatasetView + Sync>(
        &'d self,
        dataset: &'d D,
        prepared: &PreparedQuery,
        substitutions: &[(String, TermValue)],
        options: QueryOptions<'d>,
        state: &Arc<GovernorState>,
        destination: &mut RdfDatasetBuilder,
    ) -> Result<GraphBuildStats, GraphBuildError> {
        let prefix = destination_mint_prefix(destination, options.bnode_mint_prefix);
        let options = QueryOptions {
            bnode_mint_prefix: prefix.as_deref().or(options.bnode_mint_prefix),
            ..options
        };
        let staged =
            self.stage_construct(dataset, prepared, substitutions, options, Some(state))?;
        Ok(publish(staged, destination, Some(state.evidence())))
    }

    /// Build from an operationally fallible view, checking readiness before append.
    ///
    /// Operational failure takes precedence over a query error or governor trip.
    /// Evaluation is sequential for deterministic lazy-view request order. The
    /// destination is modified only after the final ready checkpoint.
    ///
    /// # Errors
    /// Returns the existing typed fallible-query error with shared-governor and
    /// dataset evidence; no error publishes any part of the graph.
    #[allow(
        clippy::too_many_arguments,
        clippy::result_large_err,
        reason = "retains the existing fallible operation contract and adds its destination"
    )]
    pub fn construct_prepared_fallible_in_operation_into_view<'d, D>(
        &'d self,
        dataset: &'d D,
        prepared: &PreparedQuery,
        substitutions: &[(String, TermValue)],
        options: QueryOptions<'d>,
        state: &Arc<GovernorState>,
        destination: &mut RdfDatasetBuilder,
    ) -> FallibleGraphBuildResult<D::Error, D::Evidence>
    where
        D: FallibleDatasetView + Sync,
    {
        if let ViewOperationStatus::Failed { error, evidence } = dataset.operation_status() {
            return Err(FallibleSparqlError::Operational {
                error,
                evidence: GovernedEvidence::new(evidence, state.evidence()),
            });
        }
        let _sequential = crate::parallel::force_sequential_operation();
        let prefix = destination_mint_prefix(destination, options.bnode_mint_prefix);
        let options = QueryOptions {
            bnode_mint_prefix: prefix.as_deref().or(options.bnode_mint_prefix),
            ..options
        };
        let evaluation =
            self.stage_construct(dataset, prepared, substitutions, options, Some(state));
        match dataset.operation_status() {
            ViewOperationStatus::Failed { error, evidence } => {
                Err(FallibleSparqlError::Operational {
                    error,
                    evidence: GovernedEvidence::new(evidence, state.evidence()),
                })
            }
            ViewOperationStatus::Ready { evidence } => {
                let evidence = GovernedEvidence::new(evidence, state.evidence());
                match evaluation {
                    Ok(staged) => Ok((
                        publish(staged, destination, Some(state.evidence())),
                        evidence,
                    )),
                    Err(GraphBuildError::Query(diagnostic)) => Err(FallibleSparqlError::Query {
                        diagnostic,
                        evidence,
                    }),
                    Err(GraphBuildError::BudgetExhausted { tripped, .. }) => {
                        Err(FallibleSparqlError::BudgetExhausted {
                            tripped,
                            partial: certain_partial(empty_result_for(&prepared.query), true),
                            evidence,
                        })
                    }
                }
            }
        }
    }

    fn stage_construct<'d, D: DatasetView + Sync>(
        &'d self,
        dataset: &'d D,
        prepared: &PreparedQuery,
        substitutions: &[(String, TermValue)],
        options: QueryOptions<'d>,
        state: Option<&Arc<GovernorState>>,
    ) -> Result<ValidatedRdfDatasetBuilder, GraphBuildError> {
        self.admit_construct(dataset, prepared, options, state)?;
        let query = if substitutions.is_empty() {
            Cow::Borrowed(&prepared.query)
        } else {
            Cow::Owned(match options.prebinding {
                ShaclPrebinding::Applied => crate::substitute::apply_shacl_prebinding(
                    prepared.query.clone(),
                    substitutions,
                )?,
                ShaclPrebinding::None => {
                    crate::substitute::apply_substitutions(prepared.query.clone(), substitutions)?
                }
            })
        };
        let mut ctx = apply_query_options(self.eval_ctx(dataset), options)?;
        if let Some(state) = state {
            ctx = ctx.with_governors(Arc::clone(state));
        }
        let _sequential = ctx
            .options
            .force_sequential
            .then(crate::parallel::force_sequential_operation);
        let evaluated = (|| {
            crate::eval::prepare_query_context(&query, &mut ctx)?;
            let Query::Construct {
                template, pattern, ..
            } = query.as_ref()
            else {
                unreachable!("substitution preserves the admitted query form")
            };
            crate::construct::eval_construct_staged(template, pattern, &mut ctx)
        })();
        finish_staging(evaluated.map_err(|error| query_error(&error))?, state)
    }
    fn admit_construct<D: DatasetView + Sync>(
        &self,
        dataset: &D,
        prepared: &PreparedQuery,
        options: QueryOptions<'_>,
        state: Option<&Arc<GovernorState>>,
    ) -> Result<(), GraphBuildError> {
        check_plan_matches_relations(prepared, options)?;
        crate::governor::soundness::validate_graph_pattern_depth(query_pattern(&prepared.query))
            .map_err(|error| query_error(&error))?;
        if !matches!(prepared.query, Query::Construct { .. }) {
            return Err(RdfDiagnostic::error(
                "native-sparql-construct",
                "typed graph building requires a CONSTRUCT query",
            )
            .into());
        }
        if let Some(state) = state {
            let _ = state.poll_stop();
            if let Some(tripped) = state.tripped() {
                return Err(GraphBuildError::BudgetExhausted {
                    tripped,
                    evidence: Box::new(state.evidence()),
                });
            }
            let dimension = ResourceDimension::IntermediateCells;
            if state.is_engaged_in(dimension) {
                let estimate = self
                    .survey_plan(dataset, &prepared.query, options.property_functions)?
                    .peak_cells();
                let limit = state.limits().get(dimension);
                if estimate > limit {
                    let tripped = state.record_trip(crate::TrippedGovernor::Refused {
                        dimension,
                        limit,
                        estimate,
                    });
                    return Err(GraphBuildError::BudgetExhausted {
                        tripped,
                        evidence: Box::new(state.evidence()),
                    });
                }
            }
        }
        Ok(())
    }
}

fn query_error(error: &crate::EvalError) -> RdfDiagnostic {
    RdfDiagnostic::error(
        eval_diagnostic_code(error, "native-sparql-query-eval"),
        error.to_string(),
    )
}

fn finish_staging<I: purrdf_core::ViewTermId>(
    result: crate::construct::StagedConstruct<I>,
    state: Option<&Arc<GovernorState>>,
) -> Result<ValidatedRdfDatasetBuilder, GraphBuildError> {
    let crate::construct::StagedConstruct {
        builder: staged,
        certificate,
        ..
    } = result;
    if let Some(state) = state
        && let Some(tripped) = state.tripped()
    {
        return Err(GraphBuildError::BudgetExhausted {
            tripped,
            evidence: Box::new(state.evidence()),
        });
    }
    if let Some(certificate) = certificate {
        let state = state.expect("a truncation requires a governor");
        return Err(GraphBuildError::BudgetExhausted {
            tripped: certificate.tripped(),
            evidence: Box::new(state.evidence()),
        });
    }
    if let Some(state) = state.filter(|state| state.is_engaged_in(ResourceDimension::AnswerRows)) {
        for _ in 0..staged.statement_count() {
            if let Err(tripped) = state.charge_final_output(ResourceDimension::AnswerRows, 1) {
                return Err(GraphBuildError::BudgetExhausted {
                    tripped,
                    evidence: Box::new(state.evidence()),
                });
            }
        }
    }
    if let Some(state) = state {
        let _ = state.poll_stop();
        if let Some(tripped) = state.tripped() {
            return Err(GraphBuildError::BudgetExhausted {
                tripped,
                evidence: Box::new(state.evidence()),
            });
        }
    }
    Ok(staged)
}

fn publish(
    staged: ValidatedRdfDatasetBuilder,
    destination: &mut RdfDatasetBuilder,
    governors: Option<crate::GovernorEvidence>,
) -> GraphBuildStats {
    let statements = staged.statement_count();
    let staged_terms = staged.term_count();
    let staged_payload_bytes = staged.payload_bytes();
    let copied_payload_bytes = destination.append_validated(staged);
    GraphBuildStats {
        statements,
        staged_terms,
        staged_payload_bytes,
        copied_payload_bytes,
        intermediate_freezes: 0,
        governors,
    }
}

/// Choose a deterministic namespace disjoint from every existing destination
/// blank. Source-carried bindings are untouched; only evaluation's minted terms
/// consume this prefix. A fresh destination keeps the ordinary query spelling.
fn destination_mint_prefix(
    destination: &RdfDatasetBuilder,
    requested: Option<&str>,
) -> Option<String> {
    if !destination
        .blank_identities()
        .any(|(_, scope)| scope == purrdf_core::BlankScope::DEFAULT)
    {
        return None;
    }
    let requested = requested.unwrap_or("");
    for ordinal in 0_u64.. {
        let candidate = format!("{requested}append{ordinal}_");
        if !destination.blank_identities().any(|(label, scope)| {
            scope == purrdf_core::BlankScope::DEFAULT && label.starts_with(&candidate)
        }) {
            return Some(candidate);
        }
    }
    unreachable!("a finite builder cannot occupy every mint namespace")
}
