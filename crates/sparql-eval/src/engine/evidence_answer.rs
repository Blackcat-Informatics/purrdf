// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Fallible publication for the governed borrowed evidence answer.

use std::ops::ControlFlow;

use purrdf_core::evidence::{AnswerRequest, AnswerTier};
use purrdf_core::{DatasetView, FallibleDatasetView, SparqlResult, TrippedGovernor};

use crate::evidence_exec::EvidenceSource;
use crate::governor::lift::Truncation;
use crate::workspace::{QueryWorkspace, SharedWorkspace};
use crate::{
    EvalError, EvidenceBudgetExhausted, GovernedEvidenceAnswer, GovernedEvidenceView,
    PartialAnswers,
};

use super::{
    BudgetExhausted, EvalCtx, EvaluationFailure, FallibleScopedResult, FallibleSparqlError,
    GovernedEvidence, GovernedResolution, GovernorState, InternedRequest, InternedSolutions,
    NativeSparqlEngine, NonMonotoneBarrier, Outcome, PlanSurveyContext, Prebindings, Query,
    QueryCoordinates, QueryGovernors, QueryOptions, Sequencing, admit_governor_state,
    borrow_outcome, bounded_workspace, certain_partial, coordinates,
    evaluate_governed_with_substitutions, fallible_admission_failure, finish_fallible_read,
    materialize, prepared_fallible, relation_identity, reserve_fallible_execution,
    reserve_governed_reporting, resolve_governed_with_prefix, retain_partial_answers,
};

enum AnswerSettlement<R> {
    Complete(R),
    Exhausted {
        tripped: TrippedGovernor,
        partial: PartialAnswers,
        prefix: Option<R>,
    },
}

impl<R> AnswerSettlement<R> {
    fn exhausted(exhausted: BudgetExhausted, prefix: Option<R>) -> Self {
        let partial = match exhausted.partial {
            // An upper bound on candidate rows is not an upper bound of false
            // on existential truth. No certain witness means ASK is unsettled.
            PartialAnswers::AtMost(bound)
                if matches!(bound.result(), SparqlResult::Boolean(false)) =>
            {
                PartialAnswers::Unknown(NonMonotoneBarrier::named("ask-unsettled"))
            }
            partial => partial,
        };
        Self::Exhausted {
            tripped: exhausted.tripped,
            partial,
            prefix,
        }
    }
}

impl NativeSparqlEngine {
    /// Visit a governed SELECT or ASK evidence answer inside a fallible evaluation.
    ///
    /// The governors are per call and can share a stop signal across evaluations.
    /// Stop, deadline and admission refusal do not run the visitor. Only a
    /// certain resource prefix whose row evidence cannot be merged by DISTINCT
    /// or REDUCED can run the prefix visitor. An ASK without a certain witness
    /// never reports `false` as a partial answer.
    ///
    /// # Errors
    /// Refuses other query forms and propagates the original operational and
    /// evaluation failures. Visitor output is publishable only after this call
    /// returns `Ok` with its final view and governor receipt.
    #[allow(
        clippy::result_large_err,
        clippy::too_many_arguments,
        reason = "the scoped evidence ingress carries the same caller inputs and typed receipts as its sibling entries"
    )]
    pub fn query_evidence_answer_governed_fallible_view<'d, D, R>(
        &'d self,
        dataset: &'d D,
        request: InternedRequest<'_>,
        options: &QueryOptions<'d, 'd>,
        governors: &QueryGovernors,
        source: &'d dyn EvidenceSource<D::Id>,
        answer: AnswerRequest,
        visit: impl FnOnce(GovernedEvidenceView<'_, '_, D>) -> R,
    ) -> FallibleScopedResult<GovernedEvidenceAnswer<R>, D::Error, GovernedEvidence<D::Evidence>>
    where
        D: FallibleDatasetView + Sync,
    {
        let reporting = reserve_governed_reporting(dataset, governors)?;
        let state = admit_governor_state(dataset, governors, &reporting.execution())?;
        prepared_fallible::preflight_governed_fallible_view(dataset, &state)?;
        let execution =
            reserve_fallible_execution(dataset, reporting.execution()).map_err(|error| {
                error.map_evidence(|view| GovernedEvidence::new(view, state.evidence()))
            })?;
        let workspace = &*execution;
        let publication = crate::user_fn::RefusalPublication::default();
        let options = publication
            .options(dataset, *options)
            .map_err(|diagnostic| {
                fallible_admission_failure(
                    dataset,
                    bounded_workspace::AdmissionError::Query(diagnostic),
                )
                .map_evidence(|view| GovernedEvidence::new(view, state.evidence()))
            })?;
        let mut capture = QueryCoordinates::NotStarted;
        let evaluation = self.governed_evidence_answer_admitted(
            dataset,
            request,
            &options,
            &state,
            source,
            answer,
            workspace,
            &mut capture,
            visit,
        );
        let receipt =
            |view| GovernedEvidence::new(view, state.evidence()).with_coordinates(capture.clone());
        let checked = (|| {
            let (settlement, view) = finish_fallible_read(dataset, evaluation, Some(workspace))
                .map_err(|error| error.map_evidence(receipt))?;
            match settlement {
                AnswerSettlement::Complete(value) => {
                    Ok((GovernedEvidenceAnswer::Complete(value), receipt(view)))
                }
                AnswerSettlement::Exhausted {
                    tripped,
                    partial,
                    prefix,
                } => {
                    let partial = retain_partial_answers(dataset, partial, Some(workspace))
                        .map_err(|error| error.map_evidence(receipt))?;
                    let (partial, view) =
                        finish_fallible_read(dataset, Ok(partial), Some(workspace))
                            .map_err(|error| error.map_evidence(receipt))?;
                    let trip = EvidenceBudgetExhausted {
                        tripped,
                        partial,
                        prefix,
                    };
                    Ok((GovernedEvidenceAnswer::BudgetExhausted(trip), receipt(view)))
                }
            }
        })();
        publication.finish(checked, |error| {
            matches!(error, FallibleSparqlError::Query { .. })
        })
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the original governor, evidence policy, reporting coordinate and execution owner cross one admission boundary"
    )]
    fn governed_evidence_answer_admitted<'d, D: DatasetView + Sync, R>(
        &'d self,
        dataset: &'d D,
        request: InternedRequest<'_>,
        options: &QueryOptions<'d, 'd>,
        state: &SharedWorkspace<GovernorState>,
        source: &'d dyn EvidenceSource<D::Id>,
        answer: AnswerRequest,
        workspace: &QueryWorkspace<D::ReadError>,
        capture: &mut QueryCoordinates,
        visit: impl FnOnce(GovernedEvidenceView<'_, '_, D>) -> R,
    ) -> Result<AnswerSettlement<R>, EvaluationFailure> {
        let capability = workspace.capability();
        let (prepared, parameters) =
            self.prepare_interned_request_admitted(request, options, &capability)?;
        if !matches!(prepared.query, Query::Select { .. } | Query::Ask { .. }) {
            return Err(
                EvalError::unsupported("governed evidence answers require SELECT or ASK").into(),
            );
        }
        parameters.check_admitted(&prepared, options, &capability)?;
        let mut identity = relation_identity(&prepared, options.property_functions(), &capability)?;
        let survey =
            PlanSurveyContext::new(options, !request.substitutions.is_empty(), true).retained();
        if let Some(refusal) = self.admit_refusal(
            dataset,
            &prepared.query,
            survey,
            state,
            &mut identity.value,
            workspace,
        ) {
            return refusal.map(|exhausted| AnswerSettlement::exhausted(exhausted, None));
        }
        let mut ctx = self.governed_ctx(dataset, state.clone(), options, workspace)?;
        // This evidence-only egress does not publish a relation witness. Keep
        // the ungoverned evidence entry's refusal for unwitnessed incomplete relations.
        ctx.witnessing = false;
        ctx.options.force_sequential |= Sequencing::for_view::<D>() == Sequencing::Sequential;
        let bindings = Prebindings::Borrowed(request.substitutions);
        *capture = coordinates::capture_query(
            &ctx,
            &prepared,
            bindings,
            options,
            coordinates::Consumer::Retained,
        )?;
        let mut ctx = attach_query_evidence(ctx, Some(source), Some(answer))?;
        let evaluated = evaluate_governed_with_substitutions(
            &prepared,
            bindings,
            survey.distinct_planning::<D>(&prepared.query),
            &mut ctx,
        )?;
        // A host callback in the last operator can signal a stop without
        // another charge or operator boundary. Latch it before resolution.
        let _ = ctx.stop_check();
        let mut visit = Some(visit);
        let (resolution, prefix) = resolve_governed_with_prefix(
            evaluated,
            &mut ctx,
            state,
            identity.value,
            capture.clone(),
            |tripped, outcome, certificate, ctx| {
                visit_prefix(tripped, outcome, certificate, ctx, answer, &mut visit)
                    .map_err(EvaluationFailure::Evaluation)
            },
        )?;
        match resolution {
            GovernedResolution::Complete { outcome, .. } => {
                let visit = visit.ok_or_else(|| {
                    EvalError::internal("a prefix already consumed the completed evidence visitor")
                })?;
                let value = Self::visit_evidence_answer(
                    &outcome,
                    &ctx,
                    answer,
                    |outcome, rows, status| {
                        // Status construction calls the host's evidence source.
                        // Observe a stop before giving the caller a view.
                        if let Some(tripped) = ctx.stop_check() {
                            return ControlFlow::Break(tripped);
                        }
                        ControlFlow::Continue(visit(GovernedEvidenceView::Complete {
                            outcome,
                            rows,
                            status,
                        }))
                    },
                )?;
                Ok(match value {
                    ControlFlow::Continue(value) => AnswerSettlement::Complete(value),
                    ControlFlow::Break(tripped) => AnswerSettlement::Exhausted {
                        tripped,
                        partial: certain_partial(materialize(&outcome, &ctx)?, true),
                        prefix: None,
                    },
                })
            }
            GovernedResolution::Exhausted(exhausted) => {
                Ok(AnswerSettlement::exhausted(exhausted, prefix))
            }
        }
    }
}

pub(super) fn attach_query_evidence<'d, D: DatasetView + Sync>(
    mut ctx: EvalCtx<'d, D>,
    source: Option<&'d dyn EvidenceSource<D::Id>>,
    answer: Option<AnswerRequest>,
) -> Result<EvalCtx<'d, D>, EvalError> {
    if let Some(source) = source {
        ctx = ctx.with_evidence_source(source)?;
    }
    ctx.evidence_verdict =
        answer.filter(|request| request.tier == AnswerTier::Verdict && request.question.is_some());
    Ok(ctx)
}

fn visit_prefix<D: DatasetView + Sync, R>(
    tripped: TrippedGovernor,
    outcome: &Outcome<D::Id>,
    certificate: Option<&Truncation<D::Id>>,
    ctx: &EvalCtx<'_, D>,
    answer: AnswerRequest,
    visit: &mut Option<impl FnOnce(GovernedEvidenceView<'_, '_, D>) -> R>,
) -> Result<Option<R>, EvalError> {
    let plan = ctx
        .plan
        .as_ref()
        .ok_or_else(|| EvalError::internal("an evidence prefix lost its numbered query plan"))?;
    if plan.shape().coalesces_row_evidence() {
        return Ok(None);
    }
    let rows = match outcome {
        Outcome::Solutions(rows) => Some(rows),
        Outcome::Boolean(true) => certificate.map_or(ctx.evidence_answer.as_ref(), |certificate| {
            Some(certificate.rows())
        }),
        _ => None,
    }
    .ok_or_else(|| EvalError::internal("a certified evidence prefix lost its original rows"))?;
    let input = ctx
        .evidence_input
        .as_ref()
        .ok_or_else(|| EvalError::internal("an evidence prefix lost its source"))?;
    let visit = visit
        .take()
        .ok_or_else(|| EvalError::internal("the evidence prefix visitor was already used"))?;
    input.with_prefix_answer_status(rows, answer, &ctx.growth, |status| {
        // The first resource trip stays latched, but a subsequent host stop
        // still withholds the prefix visitor.
        if ctx
            .governors
            .as_ref()
            .is_some_and(|state| state.poll_stop().is_some())
        {
            return None;
        }
        Some(visit(GovernedEvidenceView::Prefix {
            tripped,
            outcome: borrow_outcome(outcome, ctx),
            rows: InternedSolutions::new(rows, ctx),
            status,
        }))
    })
}
