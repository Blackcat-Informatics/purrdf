// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Query-local immutable evidence carried with the original retained rows.

use std::sync::Mutex;

use purrdf_core::evidence::{
    EvidenceCombination, EvidenceDictionaryError, EvidenceId, EvidenceRecord, EvidenceRecordError,
    EvidenceWorkspace, EvidenceWorkspaceError, ReasonDescription, ReasonError, ReasonId,
    ReasonTable, ReasonTableError, RetainedReason, SearchCompleteness,
};
use purrdf_lex::allocation::{Admission, Memory, StorageError};

use crate::error::{NativeDiagnostic, NativeDiagnosticKind};
use crate::workspace::{LexicalFrame, SharedWorkspace};
use crate::{EvalError, WorkspaceCapability, WorkspaceTerm};

/// Caller-selected RDF representation of one original solution, in answer order.
/// PurRDF cannot invent a vocabulary or a triple-term encoding for arbitrary
/// projected bindings. These terms remain borrowed throughout ledger reads.
#[derive(Clone, Copy, Debug)]
pub struct LedgerRowTerms<'a> {
    /// Unique caller-minted row reifier.
    pub node: &'a purrdf_core::TermValue,
    /// Caller-selected triple term representing the actual solution.
    pub solution: &'a purrdf_core::TermValue,
}

/// Explicit acceptance and RDF declarations for the original answer. Row terms
/// cover the full original order; rendering selects established and accepted
/// rows without re-ranking them. Every accepted reason remains visible.
#[derive(Debug)]
pub struct AcceptedLedgerRead<'a, N> {
    /// Sorted, unique reasons explicitly accepted by the caller.
    pub accepted: &'a [ReasonId],
    /// RDF declarations for every original answer row in order.
    pub terms: &'a [LedgerRowTerms<'a>],
    /// Caller-selected content-identified dictionary nodes.
    pub nodes: &'a N,
    /// Explicit caller vocabulary, with no ontology fallback.
    pub vocabulary: &'a purrdf_core::evidence::LedgerVocabulary<'a>,
}

/// Original query partitions merged by the core acceptance law. Iteration
/// borrows native rows; it never re-runs a producer or recomputes a rank.
#[derive(Debug)]
pub struct AcceptedAnswerRows<'a, I: purrdf_core::ViewTermId> {
    inner: AcceptedPayloadRows<'a, crate::solution::RetainedRow<I>>,
}

impl<'a, I: purrdf_core::ViewTermId> Iterator for AcceptedAnswerRows<'a, I> {
    type Item = Result<
        purrdf_core::evidence::AcceptedRow<'a, &'a crate::solution::RetainedRow<I>>,
        purrdf_core::evidence::AcceptedMergeError,
    >;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

struct LedgerRead<'a, N> {
    terms: &'a [LedgerRowTerms<'a>],
    nodes: &'a N,
    vocabulary: &'a purrdf_core::evidence::LedgerVocabulary<'a>,
    accepted: Option<&'a [ReasonId]>,
}

/// Lazy RDF annotations borrowing the original query evidence dictionaries.
/// Constructed only on an explicit ledger read; no RDF graph is copied.
#[derive(Debug)]
pub struct EvidenceLedger<'a, N: purrdf_core::evidence::LedgerNodes> {
    source: purrdf_core::evidence::AnswerLedger<'a, N, EvidenceAdmission>,
}

impl<'a, N: purrdf_core::evidence::LedgerNodes> purrdf_core::evidence::LedgerSource<'a>
    for EvidenceLedger<'a, N>
{
    fn statements(
        &self,
    ) -> impl Iterator<
        Item = Result<
            purrdf_core::evidence::LedgerStatement<'a>,
            purrdf_core::evidence::LedgerError,
        >,
    > {
        self.source.statements()
    }
}

/// Caller-declared whole-search scope, independent of an individual claim's polarity.
#[derive(Clone, Copy, Debug)]
pub struct EvidenceScope {
    /// Whether the declared input has been searched completely.
    pub completeness: SearchCompleteness,
    /// Whether absence in this scope is a declared closed-world conclusion.
    pub closure_declared: bool,
    /// Original reason explaining missing closure or incomplete search.
    pub missing_reason: Option<ReasonId>,
}

/// A declaration for one original SERVICE response row, before blank-node remapping.
#[derive(Clone, Copy, Debug)]
pub struct ServiceRowEvidence<'a> {
    /// The caller's immutable signed evidence declaration.
    pub record: &'a EvidenceRecord,
    /// Whether membership itself remains undecided, independently of polarity.
    pub undecided: bool,
}

/// Evidence for the original frozen view's matched claims and search scopes.
/// Operational errors stay on the evaluator's hard error channel.
pub trait EvidenceSource<I: purrdf_core::ViewTermId>: Sync {
    /// Borrow the declared sharpening order for scoped claim witnesses.
    /// A declared standpoint claim requires this order. Ordinary and unscoped
    /// claims do not implicitly enter an order when it is absent.
    fn standpoint_order(
        &self,
    ) -> Option<&purrdf_core::evidence::StandpointOrder<purrdf_hash::hex::Digest32>> {
        None
    }

    /// Borrow a matched claim's immutable declaration; None denotes ordinary evidence.
    ///
    /// # Errors
    /// Returns the original source or declaration failure.
    fn claim(&self, quad: &purrdf_core::QuadIds<I>) -> Result<Option<&EvidenceRecord>, EvalError>;

    /// Read an explicit scope declaration before drawing absence conclusions.
    ///
    /// # Errors
    /// Returns the original source or scope declaration failure.
    fn scope(&self, graph: purrdf_core::GraphMatch<I>) -> Result<EvidenceScope, EvalError>;

    /// Borrow an original canonical reason description.
    ///
    /// # Errors
    /// Returns the original source failure; an unknown identity is None.
    fn reason(&self, id: ReasonId) -> Result<Option<ReasonDescription<'_>>, EvalError>;

    /// Declare the scope of a successfully resolved SERVICE invocation, including
    /// an empty response. The default uses the caller's whole-input declaration.
    /// A transport error never becomes a scope declaration.
    ///
    /// # Errors
    /// Returns the original declaration failure.
    fn service_scope(&self, _endpoint: &str) -> Result<EvidenceScope, EvalError> {
        self.scope(purrdf_core::GraphMatch::Any)
    }

    /// Borrow a successful response row's declaration in its original response
    /// order and vocabulary. No row, term or declaration is reconstructed.
    /// None denotes the ordinary supported witness.
    ///
    /// # Errors
    /// Returns the original declaration failure, never a semantic unknown.
    fn service_row(
        &self,
        _endpoint: &str,
        _position: usize,
        _variables: &[purrdf_sparql_algebra::Variable],
        _binding: &[Option<purrdf_core::TermValue>],
    ) -> Result<Option<ServiceRowEvidence<'_>>, EvalError> {
        Ok(None)
    }

    /// Map a ranked producer's original loss declaration to a caller-owned cause.
    /// The producer's evidence is borrowed verbatim, never interpreted here.
    /// Returning None supplies no vocabulary and is refused when loss is reached.
    ///
    /// # Errors
    /// Returns the original caller declaration failure.
    fn lossy_producer_reason(
        &self,
        _producer: &str,
        _evidence: &str,
    ) -> Result<Option<ReasonId>, EvalError> {
        Ok(None)
    }

    /// Declare a genuinely missing input to this expression for this original
    /// row. Computed cells remain query-local ids; this callback must describe
    /// input facts it owns, rather than infer gaps from an evaluation error.
    /// None means the expression is decidable under the caller's declarations.
    ///
    /// # Errors
    /// Returns the original declaration failure, never a semantic unknown.
    fn expression_gap(
        &self,
        _expression: &purrdf_sparql_algebra::Expression,
        _variables: &[purrdf_sparql_algebra::Variable],
        _row: &[Option<crate::SolutionTerm<I>>],
        _graph: purrdf_core::GraphMatch<I>,
    ) -> Result<Option<ReasonId>, EvalError> {
        Ok(None)
    }

    /// Whether this SUM expression's whole declared domain, including missing
    /// members, is established nonnegative. Without such a declaration a partial
    /// SUM has no directional bound; observed positive rows alone prove nothing
    /// about the unseen domain.
    ///
    /// # Errors
    /// Returns an original declaration/read failure.
    fn sum_nonnegative(
        &self,
        _aggregate: &purrdf_sparql_algebra::AggregateExpression,
        _graph: purrdf_core::GraphMatch<I>,
    ) -> Result<bool, EvalError> {
        Ok(false)
    }
}

/// One original query dictionary admission, shared before worker contexts fork.
/// Native producers and explicit source declarations use the same adoption home.
#[derive(Clone)]
pub(crate) struct EvidenceOwner {
    state: SharedWorkspace<Mutex<Option<QueryEvidence>>>,
}

impl EvidenceOwner {
    pub(crate) fn new(
        owner: Option<QueryEvidence>,
        workspace: &WorkspaceCapability,
    ) -> Result<Self, EvalError> {
        Ok(Self {
            state: SharedWorkspace::new_admitted(Mutex::new(owner), workspace)?,
        })
    }

    pub(crate) fn query_owner(
        &self,
        workspace: &WorkspaceCapability,
    ) -> Result<QueryEvidence, EvalError> {
        let mut owner = self.state.lock().map_err(|_| {
            NativeDiagnostic::error_cause(
                NativeDiagnosticKind::Internal,
                crate::error::EvaluatorInvariant::EvidenceInputPoisoned,
                workspace,
            )
        })?;
        if owner.is_none() {
            *owner = Some(QueryEvidence::new(workspace)?);
        }
        Ok(owner
            .as_ref()
            .expect("native query owner constructed above")
            .clone())
    }

    /// Bind original native declarations to the answer's admitted dictionary.
    /// An input with no declarations adopts the producer's actual owner; an
    /// already admitted input imports through the canonical evidence home.
    pub(crate) fn bind_native_evidence(
        &self,
        row: &RowEvidence,
        workspace: &WorkspaceCapability,
    ) -> Result<RowEvidence, EvalError> {
        if row.is_virtual() {
            return Ok(row.clone());
        }
        let query = {
            let mut owner = self.state.lock().map_err(|_| {
                NativeDiagnostic::error_cause(
                    NativeDiagnosticKind::Internal,
                    crate::error::EvaluatorInvariant::EvidenceInputPoisoned,
                    workspace,
                )
            })?;
            if owner.is_none() {
                (*owner).clone_from(&row.owner);
            }
            owner.clone()
        };
        match query {
            Some(query) => query.import(row),
            None => Ok(row.clone()),
        }
    }

    pub(crate) fn retained_owner(
        &self,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<QueryEvidence>, EvalError> {
        self.state.lock().map(|owner| owner.clone()).map_err(|_| {
            NativeDiagnostic::error_cause(
                NativeDiagnosticKind::Internal,
                crate::error::EvaluatorInvariant::EvidenceInputPoisoned,
                workspace,
            )
        })
    }
}

#[derive(Clone)]
pub(crate) struct EvidenceInput<'d, I: purrdf_core::ViewTermId> {
    source: &'d dyn EvidenceSource<I>,
    pub(crate) owner: EvidenceOwner,
}

impl<'d, I: purrdf_core::ViewTermId> EvidenceInput<'d, I> {
    pub(crate) fn new(
        source: &'d dyn EvidenceSource<I>,
        workspace: &WorkspaceCapability,
        owner: Option<QueryEvidence>,
    ) -> Result<Self, EvalError> {
        Ok(Self {
            source,
            owner: EvidenceOwner::new(owner, workspace)?,
        })
    }

    pub(crate) fn claim(
        &self,
        quad: &purrdf_core::QuadIds<I>,
        workspace: &WorkspaceCapability,
    ) -> Result<RowEvidence, EvalError> {
        let Some(record) = self.source.claim(quad)? else {
            return Ok(RowEvidence::default());
        };
        self.retain_declaration(record, workspace)
    }

    fn retain_declaration(
        &self,
        record: &EvidenceRecord,
        workspace: &WorkspaceCapability,
    ) -> Result<RowEvidence, EvalError> {
        if record.is_default() {
            return Ok(RowEvidence::default());
        }
        let scoped = record
            .information()
            .entries()
            .iter()
            .any(|entry| matches!(entry.0, purrdf_core::evidence::StandpointKey::Declared(_)));
        let order = if scoped {
            Some(self.source.standpoint_order().ok_or_else(|| {
                NativeDiagnostic::error(
                    NativeDiagnosticKind::Config,
                    "declared standpoint evidence requires its original sharpening order",
                    workspace,
                )
            })?)
        } else {
            None
        };
        let query = self.owner.query_owner(workspace)?;
        for &id in record.reasons() {
            query.retain_reason(id, || self.source.reason(id))?;
        }
        query.retain_in_order(record, order)
    }

    pub(crate) fn service_row(
        &self,
        endpoint: &str,
        position: usize,
        variables: &[purrdf_sparql_algebra::Variable],
        binding: &[Option<purrdf_core::TermValue>],
        workspace: &WorkspaceCapability,
    ) -> Result<RowEvidence, EvalError> {
        let Some(declaration) = self
            .source
            .service_row(endpoint, position, variables, binding)?
        else {
            return Ok(RowEvidence::default());
        };
        if declaration.undecided
            && declaration.record.completeness() == SearchCompleteness::Complete
        {
            return Err(NativeDiagnostic::error(
                NativeDiagnosticKind::Config,
                "undecided SERVICE membership requires declared incomplete evidence",
                workspace,
            ));
        }
        self.retain_declaration(declaration.record, workspace)
            .map(|row| row.with_undecided(declaration.undecided))
    }

    pub(crate) fn service_scope(
        &self,
        endpoint: &str,
        workspace: &WorkspaceCapability,
    ) -> Result<RowEvidence, EvalError> {
        let scope = self.source.service_scope(endpoint)?;
        self.retain_scope(&RowEvidence::default(), scope, workspace)
            .map(|row| row.with_unclosed_search(!scope.closure_declared))
    }

    /// A matched claim or path derivation establishes its witness independently
    /// of missing edges elsewhere. Keep polarity, reasons and frame unchanged.
    pub(crate) fn found_witness(
        &self,
        row: &RowEvidence,
        workspace: &WorkspaceCapability,
    ) -> Result<RowEvidence, EvalError> {
        if row.with_record(EvidenceRecord::completeness)? == SearchCompleteness::Complete {
            return Ok(row.clone());
        }
        self.owner.query_owner(workspace)?.with_completeness(
            row,
            SearchCompleteness::Complete,
            None,
        )
    }

    pub(crate) fn scope_evidence(
        &self,
        row: &RowEvidence,
        graph: purrdf_core::GraphMatch<I>,
        workspace: &WorkspaceCapability,
    ) -> Result<RowEvidence, EvalError> {
        self.retain_scope(row, self.source.scope(graph)?, workspace)
    }

    fn retain_scope(
        &self,
        row: &RowEvidence,
        scope: EvidenceScope,
        workspace: &WorkspaceCapability,
    ) -> Result<RowEvidence, EvalError> {
        if scope.completeness == SearchCompleteness::Complete && scope.missing_reason.is_none() {
            return Ok(row.clone());
        }
        let query = self.owner.query_owner(workspace)?;
        if let Some(id) = scope.missing_reason {
            query.retain_reason(id, || self.source.reason(id))?;
        }
        let completeness =
            row.with_record(|record| record.completeness().worse(scope.completeness))?;
        query.with_completeness(row, completeness, scope.missing_reason)
    }

    /// Translate the actual registered search attestation, independently of
    /// operational cursor exhaustion and independently of candidate polarity.
    pub(crate) fn lossy_producer_evidence(
        &self,
        producer: &str,
        evidence: &str,
        workspace: &WorkspaceCapability,
    ) -> Result<RowEvidence, EvalError> {
        let reason = self
            .source
            .lossy_producer_reason(producer, evidence)?
            .ok_or_else(|| {
                NativeDiagnostic::error(
                    NativeDiagnosticKind::Config,
                    format_args!(
                        "lossy producer <{producer}> requires its caller-owned cause declaration"
                    ),
                    workspace,
                )
            })?;
        let query = self.owner.query_owner(workspace)?;
        query.retain_reason(reason, || self.source.reason(reason))?;
        query.with_completeness(
            &RowEvidence::default(),
            SearchCompleteness::Undetermined,
            Some(reason),
        )
    }

    pub(crate) fn aggregate_evidence(
        &self,
        row: &RowEvidence,
        aggregates: &[(
            purrdf_sparql_algebra::Variable,
            purrdf_sparql_algebra::AggregateExpression,
        )],
        graph: purrdf_core::GraphMatch<I>,
        workspace: &WorkspaceCapability,
    ) -> Result<RowEvidence, EvalError> {
        let row = self.scope_evidence(row, graph, workspace)?;
        if aggregates.is_empty() {
            return Ok(row);
        }
        self.owner.query_owner(workspace)?.aggregate_evidence(
            &row,
            aggregates,
            workspace,
            |aggregate| self.source.sum_nonnegative(aggregate, graph),
        )
    }

    pub(crate) fn expression_evidence(
        &self,
        expression: &purrdf_sparql_algebra::Expression,
        variables: &[purrdf_sparql_algebra::Variable],
        row: &crate::solution::RetainedRow<I>,
        graph: purrdf_core::GraphMatch<I>,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<RowEvidence>, EvalError> {
        let Some(reason) = self
            .source
            .expression_gap(expression, variables, row, graph)?
        else {
            return Ok(None);
        };
        let query = self.owner.query_owner(workspace)?;
        query.retain_reason(reason, || self.source.reason(reason))?;
        let completeness = row.evidence().with_record(|record| {
            record
                .completeness()
                .worse(SearchCompleteness::Undetermined)
        })?;
        query
            .with_completeness(row.evidence(), completeness, Some(reason))
            .map(|evidence| Some(evidence.with_undecided(true)))
    }

    pub(crate) fn absence_evidence(
        &self,
        row: &RowEvidence,
        graph: purrdf_core::GraphMatch<I>,
        workspace: &WorkspaceCapability,
    ) -> Result<RowEvidence, EvalError> {
        let scope = self.source.scope(graph)?;
        if scope.completeness.admits_absence(scope.closure_declared)
            && !row.unclosed_search
            && row.with_record(EvidenceRecord::completeness)? == SearchCompleteness::Complete
        {
            return Ok(row.clone());
        }
        let query = self.owner.query_owner(workspace)?;
        if let Some(reason) = scope.missing_reason {
            query.retain_reason(reason, || self.source.reason(reason))?;
        }
        let completeness = row.with_record(|record| {
            record
                .completeness()
                .worse(scope.completeness)
                .worse(SearchCompleteness::Undetermined)
        })?;
        query
            .with_completeness(row, completeness, scope.missing_reason)
            .map(|evidence| evidence.with_undecided(true))
    }

    /// Negate an original witness only after both its search and the declared
    /// scope establish closure. Missing closure is a membership gap, never an
    /// inverted claim or an operational-error fallback.
    pub(crate) fn negative_evidence(
        &self,
        witness: &RowEvidence,
        graph: purrdf_core::GraphMatch<I>,
        workspace: &WorkspaceCapability,
    ) -> Result<RowEvidence, EvalError> {
        let guarded = self.absence_evidence(witness, graph, workspace)?;
        let complete = guarded.with_record(EvidenceRecord::completeness)?;
        if guarded.is_undecided() || complete != SearchCompleteness::Complete {
            return Ok(guarded.with_undecided(true));
        }
        self.owner
            .query_owner(workspace)?
            .transform(&guarded, EvidenceRecord::negated)
    }

    /// Retain an existential witness bag using the original query dictionary.
    /// Only a completed, closed empty search opposes the existential predicate.
    pub(crate) fn existential_evidence<'r>(
        &self,
        witnesses: impl IntoIterator<Item = &'r RowEvidence>,
        search: &RowEvidence,
        graph: purrdf_core::GraphMatch<I>,
        workspace: &WorkspaceCapability,
    ) -> Result<RowEvidence, EvalError> {
        if let Some(combined) = RowEvidence::existential_witnesses(witnesses, search)? {
            return Ok(combined);
        }
        let search = self.absence_evidence(search, graph, workspace)?;
        self.owner
            .query_owner(workspace)?
            .empty_existential(&search)
    }

    /// Admit the caller's question against the source's explicit whole scope.
    pub(crate) fn answer_request(
        &self,
        mut request: purrdf_core::evidence::AnswerRequest,
    ) -> Result<purrdf_core::evidence::AnswerRequest, EvalError> {
        let scope = self.source.scope(purrdf_core::GraphMatch::Any)?;
        request.input_completeness = request.input_completeness.worse(scope.completeness);
        request.closure_declared &= scope.closure_declared;
        Ok(request)
    }

    pub(crate) fn with_answer_status<R>(
        &self,
        rows: &crate::solution::SolutionSeq<I>,
        mut request: purrdf_core::evidence::AnswerRequest,
        workspace: &WorkspaceCapability,
        exhausted: bool,
        visit: impl FnOnce(purrdf_core::evidence::AnswerStatus<'_>) -> R,
    ) -> Result<R, EvalError> {
        request = self.answer_request(request)?;
        request.input_completeness = request.input_completeness.worse(
            rows.rows
                .search_evidence()
                .with_record(EvidenceRecord::completeness)?,
        );
        request.closure_declared &= !rows.rows.search_evidence().unclosed_search;
        with_native_answer(request, workspace, |answer| {
            rows.rows
                .search_evidence()
                .with_record(|record| {
                    answer.observe_search(record, !rows.rows.search_evidence().unclosed_search)
                })
                .map_err(StatusFailure::Evaluation)?
                .map_err(StatusFailure::Answer)?;
            observe_answer_rows(answer, rows)?;
            answer.finish(exhausted).map_err(StatusFailure::Answer)?;
            Ok(visit(answer.status().map_err(StatusFailure::Answer)?))
        })
    }

    pub(crate) fn with_ledger<N: purrdf_core::evidence::LedgerNodes, R>(
        &self,
        rows: &crate::solution::SolutionSeq<I>,
        terms: &[LedgerRowTerms<'_>],
        nodes: &N,
        vocabulary: &purrdf_core::evidence::LedgerVocabulary<'_>,
        workspace: &WorkspaceCapability,
        visit: impl FnOnce(&EvidenceLedger<'_, N>) -> R,
    ) -> Result<R, EvalError> {
        self.with_ledger_selection(
            rows,
            &LedgerRead {
                terms,
                nodes,
                vocabulary,
                accepted: None,
            },
            workspace,
            visit,
        )
    }

    pub(crate) fn with_accepted_ledger<N: purrdf_core::evidence::LedgerNodes, R>(
        &self,
        rows: &crate::solution::SolutionSeq<I>,
        read: &AcceptedLedgerRead<'_, N>,
        workspace: &WorkspaceCapability,
        visit: impl FnOnce(&EvidenceLedger<'_, N>) -> R,
    ) -> Result<R, EvalError> {
        self.with_ledger_selection(
            rows,
            &LedgerRead {
                terms: read.terms,
                nodes: read.nodes,
                vocabulary: read.vocabulary,
                accepted: Some(read.accepted),
            },
            workspace,
            visit,
        )
    }

    fn with_ledger_selection<N: purrdf_core::evidence::LedgerNodes, R>(
        &self,
        rows: &crate::solution::SolutionSeq<I>,
        read: &LedgerRead<'_, N>,
        workspace: &WorkspaceCapability,
        visit: impl FnOnce(&EvidenceLedger<'_, N>) -> R,
    ) -> Result<R, EvalError> {
        if read.terms.len() != rows.rows.len() {
            return Err(NativeDiagnostic::error(
                NativeDiagnosticKind::Config,
                "ledger declarations must cover the original answer in order",
                workspace,
            ));
        }
        let owner = self.owner.query_owner(workspace)?;
        let records = owner.state.records.lock().map_err(|_| {
            owner.invariant(crate::error::EvaluatorInvariant::LedgerEvidenceDictionaryPoisoned)
        })?;
        let reasons = owner.state.reasons.lock().map_err(|_| {
            owner.invariant(crate::error::EvaluatorInvariant::LedgerReasonDictionaryPoisoned)
        })?;
        let admission = EvidenceAdmission(SharedWorkspace::new_admitted(
            Mutex::new(LexicalFrame::new(workspace)),
            workspace,
        )?);
        let result = (|| {
            let mut row_acceptances = crate::AdmittedVec::new(workspace);
            if let Some(accepted) = read.accepted {
                owner.accepted_payloads_locked(
                    &rows.rows,
                    crate::solution::RetainedRow::evidence,
                    &records,
                    accepted,
                )?;
                for row in &rows.rows {
                    let mut selected = crate::AdmittedVec::new(workspace);
                    if row.evidence().is_undecided() {
                        let record = records
                            .get(row.evidence().id)
                            .map_err(|error| owner.failure(error))?;
                        for reason in record.reasons() {
                            if accepted.binary_search(reason).is_ok() {
                                selected.push(*reason)?;
                            }
                        }
                    }
                    row_acceptances.push(selected)?;
                }
            }
            let mut declarations = crate::AdmittedVec::new(workspace);
            for (position, (row, terms)) in rows.rows.iter().zip(read.terms).enumerate() {
                if !owner.owns(row.evidence()) {
                    return Err(
                        owner.invariant(crate::error::EvaluatorInvariant::LedgerRowForeignQuery)
                    );
                }
                let record = records
                    .get(row.evidence().id)
                    .map_err(|error| owner.failure(error))?;
                let accepted = row_acceptances
                    .get(position)
                    .map_or(&[][..], |selected| &**selected);
                if read.accepted.is_some() && row.evidence().is_undecided() && accepted.is_empty() {
                    continue;
                }
                declarations.push(purrdf_core::evidence::LedgerRow {
                    node: terms.node,
                    solution: terms.solution,
                    record,
                    accepted,
                })?;
            }
            let source = purrdf_core::evidence::AnswerLedger::new(
                declarations.iter().copied(),
                &reasons,
                read.nodes,
                *read.vocabulary,
                admission.clone(),
            )
            .map_err(|error| match error {
                purrdf_core::evidence::LedgerBuildError::Storage(error) => {
                    admission.0.lock().map_or_else(
                        |_| {
                            owner.invariant(
                                crate::error::EvaluatorInvariant::LedgerAdmissionPoisoned,
                            )
                        },
                        |mut frame| frame.storage_error(error, "native ledger cursors"),
                    )
                }
                error => owner.invariant(error),
            })?;
            Ok(visit(&EvidenceLedger { source }))
        })();
        let original = admission
            .0
            .lock()
            .map_err(|_| {
                owner.invariant(crate::error::EvaluatorInvariant::LedgerAdmissionPoisoned)
            })?
            .take_failure();
        drop(reasons);
        drop(records);
        if let Some(original) = original {
            return Err(original);
        }
        result
    }

    pub(crate) fn with_accepted_rows<R>(
        &self,
        rows: &crate::solution::SolutionSeq<I>,
        accepted: &[ReasonId],
        workspace: &WorkspaceCapability,
        visit: impl FnOnce(AcceptedAnswerRows<'_, I>) -> R,
    ) -> Result<R, EvalError> {
        let owner = self.owner.query_owner(workspace)?;
        owner.with_accepted_payloads(
            &rows.rows,
            crate::solution::RetainedRow::evidence,
            accepted,
            |inner| visit(AcceptedAnswerRows { inner }),
        )
    }
}

impl QueryEvidence {
    /// Import an immutable row from another original query owner under this
    /// answer's native admission. Reason identities and RDF descriptions,
    /// frames, completeness and membership remain unchanged.
    ///
    /// # Errors
    /// Preserves original source and destination native refusals.
    pub fn import(&self, row: &RowEvidence) -> Result<RowEvidence, EvalError> {
        if self.owns(row) {
            return Ok(row.clone());
        }
        row.with_record(|record| {
            for &id in record.reasons() {
                row.with_reason(id, |description| {
                    self.retain_reason(id, || Ok(Some(description)))
                })??;
            }
            self.retain(record).map(|imported| {
                let mut imported = imported;
                if row.owner.is_some() {
                    imported.owner = Some(self.clone());
                }
                imported
                    .with_undecided(row.undecided)
                    .with_unclosed_search(row.unclosed_search)
                    .with_rank_of(row)
            })
        })?
    }

    /// Borrow an original answer's payloads through the shared ordered
    /// acceptance law. The callback cannot outlive the native dictionary guard.
    ///
    /// # Errors
    /// Refuses cross-owner evidence, noncanonical acceptance and reasons absent
    /// from the actual retained answer, including intermediate-only reasons.
    pub fn with_accepted_payloads<T, R>(
        &self,
        rows: &[T],
        evidence: fn(&T) -> &RowEvidence,
        accepted: &[ReasonId],
        visit: impl FnOnce(AcceptedPayloadRows<'_, T>) -> R,
    ) -> Result<R, EvalError> {
        let records = self.state.records.lock().map_err(|_| {
            self.invariant(crate::error::EvaluatorInvariant::AcceptanceEvidencePoisoned)
        })?;
        let rows = self.accepted_payloads_locked(rows, evidence, &records, accepted)?;
        let result = visit(rows);
        drop(records);
        Ok(result)
    }

    fn accepted_payloads_locked<'a, T>(
        &self,
        rows: &'a [T],
        evidence: fn(&T) -> &RowEvidence,
        records: &'a EvidenceWorkspace<EvidenceAdmission>,
        accepted: &'a [ReasonId],
    ) -> Result<AcceptedPayloadRows<'a, T>, EvalError> {
        for row in rows {
            if !self.owns(evidence(row)) {
                return Err(
                    self.invariant(crate::error::EvaluatorInvariant::AcceptedPayloadForeignQuery)
                );
            }
            records
                .get(evidence(row).id)
                .map_err(|error| self.failure(error))?;
        }
        let partition = |undecided| PayloadPartition {
            rows: rows.iter().enumerate(),
            evidence,
            undecided,
        };
        let inner = purrdf_core::evidence::AcceptedMerge::new(
            partition(false),
            partition(true),
            records,
            accepted,
        )
        .map_err(|error| {
            NativeDiagnostic::error(NativeDiagnosticKind::Config, error, &self.state.workspace)
        })?;
        for reason in accepted {
            let mut present = false;
            for row in rows {
                let record = records
                    .get(evidence(row).id)
                    .map_err(|error| self.failure(error))?;
                present |= record.reasons().binary_search(reason).is_ok();
            }
            if !present {
                return Err(NativeDiagnostic::error(
                    NativeDiagnosticKind::Config,
                    purrdf_core::evidence::AcceptedMergeError::UnknownReason(*reason),
                    &self.state.workspace,
                ));
            }
        }
        Ok(AcceptedPayloadRows { inner })
    }
}

/// A borrowed original payload stream with explicit acceptance annotations.
#[derive(Debug)]
pub struct AcceptedPayloadRows<'a, T> {
    inner: purrdf_core::evidence::AcceptedMerge<
        'a,
        &'a T,
        PayloadPartition<'a, T>,
        PayloadPartition<'a, T>,
        EvidenceAdmission,
    >,
}

#[derive(Debug)]
struct PayloadPartition<'a, T> {
    rows: core::iter::Enumerate<core::slice::Iter<'a, T>>,
    evidence: fn(&T) -> &RowEvidence,
    undecided: bool,
}

impl<'a, T> Iterator for PayloadPartition<'a, T> {
    type Item = purrdf_core::evidence::OrderedEvidenceRow<&'a T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.rows
            .find(|(_, row)| (self.evidence)(row).is_undecided() == self.undecided)
            .map(
                |(position, row)| purrdf_core::evidence::OrderedEvidenceRow {
                    position: position as u64,
                    row,
                    evidence: (self.evidence)(row).id,
                },
            )
    }
}

impl<'a, T> Iterator for AcceptedPayloadRows<'a, T> {
    type Item = Result<
        purrdf_core::evidence::AcceptedRow<'a, &'a T>,
        purrdf_core::evidence::AcceptedMergeError,
    >;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

pub(crate) enum StatusFailure {
    Answer(purrdf_core::evidence::AnswerError),
    Evaluation(EvalError),
}

/// The one admission/error boundary used by both streaming and final answers.
pub(crate) fn with_native_answer<R>(
    request: purrdf_core::evidence::AnswerRequest,
    workspace: &WorkspaceCapability,
    run: impl FnOnce(
        &mut purrdf_core::evidence::AnswerAccumulator<&mut LexicalFrame>,
    ) -> Result<R, StatusFailure>,
) -> Result<R, EvalError> {
    let mut frame = LexicalFrame::new(workspace);
    let result = {
        let mut answer =
            purrdf_core::evidence::AnswerAccumulator::with_admission(request, &mut frame);
        run(&mut answer)
    };
    if let Some(original) = frame.take_failure() {
        return Err(original);
    }
    result.map_err(|error| answer_failure(error, workspace))
}

/// Preserve the original physical count failure on both answer consumers.
pub(crate) fn answer_failure(error: StatusFailure, workspace: &WorkspaceCapability) -> EvalError {
    match error {
        StatusFailure::Evaluation(error) => error,
        StatusFailure::Answer(purrdf_core::evidence::AnswerError::Storage(error)) => {
            LexicalFrame::new(workspace).storage_error(error, "native answer reason counts")
        }
        StatusFailure::Answer(error) => {
            NativeDiagnostic::error_cause(NativeDiagnosticKind::Internal, error, workspace)
        }
    }
}

/// Observe the actual delivered partitions without copying cells or records.
pub(crate) fn observe_answer_rows<I: purrdf_core::ViewTermId, S: Admission>(
    answer: &mut purrdf_core::evidence::AnswerAccumulator<S>,
    rows: &crate::solution::SolutionSeq<I>,
) -> Result<(), StatusFailure> {
    use purrdf_core::evidence::EvidencePartition;
    for row in &rows.rows {
        let partition = if row.evidence().is_undecided() {
            EvidencePartition::Undecided
        } else {
            EvidencePartition::Established
        };
        row.evidence()
            .with_record(|record| answer.observe(record, partition))
            .map_err(StatusFailure::Evaluation)?
            .map_err(StatusFailure::Answer)?;
        if answer.witness_stop().is_some() {
            break;
        }
    }
    Ok(())
}

/// Original table admission and its independently retained native failure.
/// The record owner holds this handle until its last payload is destroyed.
#[derive(Clone, Debug)]
pub(crate) struct EvidenceAdmission(SharedWorkspace<Mutex<LexicalFrame>>);

impl EvidenceAdmission {
    pub(crate) fn new(workspace: &WorkspaceCapability) -> Result<Self, EvalError> {
        SharedWorkspace::new_admitted(Mutex::new(LexicalFrame::new(workspace)), workspace).map(Self)
    }

    pub(crate) fn storage_error(
        &self,
        storage: StorageError,
        context: &'static str,
        workspace: &WorkspaceCapability,
    ) -> EvalError {
        match self.0.lock() {
            Ok(mut frame) => frame
                .take_failure()
                .unwrap_or_else(|| frame.storage_error(storage, context)),
            Err(_) => NativeDiagnostic::error_cause(
                NativeDiagnosticKind::Internal,
                crate::error::EvaluatorInvariant::EvidenceAdmissionPoisoned,
                workspace,
            ),
        }
    }
}

impl Admission for EvidenceAdmission {
    fn resize(&mut self, live: usize) -> Result<(), StorageError> {
        self.0
            .lock()
            .map_err(|_| StorageError::AdmissionFailed)?
            .resize(live)
    }
}

/// One query's original immutable evidence dictionary and native grant.
/// Cloning shares ownership; it never clones records or adopts resident storage.
#[derive(Clone, Debug)]
pub struct QueryEvidence {
    state: SharedWorkspace<QueryEvidenceState>,
}

#[derive(Debug)]
struct QueryEvidenceState {
    records: Mutex<EvidenceWorkspace<EvidenceAdmission>>,
    reasons: Mutex<ReasonTable<EvidenceAdmission, WorkspaceTerm>>,
    admission: EvidenceAdmission,
    reason_admission: EvidenceAdmission,
    workspace: WorkspaceCapability,
}

impl QueryEvidence {
    /// Create a native evidence owner before constructing any table payload.
    ///
    /// # Errors
    /// Returns the original workspace or control-allocation refusal.
    pub fn new(workspace: &WorkspaceCapability) -> Result<Self, EvalError> {
        let admission = EvidenceAdmission::new(workspace)?;
        let reason_admission = EvidenceAdmission::new(workspace)?;
        let state = SharedWorkspace::new_admitted(
            QueryEvidenceState {
                records: Mutex::new(EvidenceWorkspace::with_admission(admission.clone())),
                reasons: Mutex::new(ReasonTable::with_admission(reason_admission.clone())),
                admission,
                reason_admission,
                workspace: workspace.clone(),
            },
            workspace,
        )?;
        Ok(Self { state })
    }

    fn invariant<E: purrdf_hash::error::NativeError + 'static>(&self, cause: E) -> EvalError {
        NativeDiagnostic::error_cause(NativeDiagnosticKind::Internal, cause, &self.state.workspace)
    }

    fn failure(&self, error: EvidenceWorkspaceError) -> EvalError {
        let Ok(mut admission) = self.state.admission.0.lock() else {
            return self.invariant(crate::error::EvaluatorInvariant::EvidenceAdmissionPoisoned);
        };
        if let Some(original) = admission.take_failure() {
            return original;
        }
        let storage = match error {
            EvidenceWorkspaceError::Record(EvidenceRecordError::Storage(storage))
            | EvidenceWorkspaceError::Dictionary(EvidenceDictionaryError::Storage(storage)) => {
                Some(storage)
            }
            _ => None,
        };
        if let Some(storage) = storage {
            admission.storage_error(storage, "query evidence payload")
        } else if matches!(
            &error,
            EvidenceWorkspaceError::Record(EvidenceRecordError::Standpoint(_))
        ) {
            NativeDiagnostic::error_cause(
                NativeDiagnosticKind::Config,
                error,
                &self.state.workspace,
            )
        } else {
            self.invariant(error)
        }
    }

    fn row(&self, id: EvidenceId, retain_owner: bool) -> RowEvidence {
        if id.is_default() && !retain_owner {
            RowEvidence::default()
        } else {
            RowEvidence {
                id,
                owner: Some(self.clone()),
                undecided: false,
                unclosed_search: false,
                original_rank: None,
            }
        }
    }

    /// Retain a caller's record through original native construction.
    /// The caller retains its borrowed declaration; the query owns its new payload.
    ///
    /// # Errors
    /// Returns native storage/refusal or a broken evidence invariant.
    pub fn retain(&self, record: &EvidenceRecord) -> Result<RowEvidence, EvalError> {
        self.retain_in_order(record, None)
    }

    /// Retain an original candidate whose declared membership is undecided.
    ///
    /// Membership is a separate fact from the record's information polarity.
    /// The original incomplete record and its previously admitted reasons remain
    /// in this same query owner; no complete absence licence is invented.
    ///
    /// # Errors
    /// Refuses a complete declaration, unknown reasons or native storage failure.
    pub fn retain_undecided(&self, record: &EvidenceRecord) -> Result<RowEvidence, EvalError> {
        if record.completeness() == SearchCompleteness::Complete {
            return Err(NativeDiagnostic::error(
                NativeDiagnosticKind::Config,
                "undecided candidate membership requires declared incomplete evidence",
                &self.state.workspace,
            ));
        }
        self.retain(record).map(|row| row.with_undecided(true))
    }

    pub(crate) fn undecided_reasons(
        &self,
        reasons: impl Clone + Iterator<Item = ReasonId>,
    ) -> Result<RowEvidence, EvalError> {
        self.require_reasons(reasons.clone())?;
        let mut table = self.state.records.lock().map_err(|_| {
            self.invariant(crate::error::EvaluatorInvariant::EvidenceDictionaryPoisoned)
        })?;
        table
            .intern(|memory| {
                memory.try_scope(|memory| {
                    let reasons = memory.collect(reasons)?;
                    let record = EvidenceRecord::from_parts(
                        &purrdf_core::evidence::InformationMap::default(),
                        SearchCompleteness::Undetermined,
                        &purrdf_core::evidence::Frame::default(),
                        &reasons,
                        &[],
                        memory,
                    );
                    memory.release_vec(reasons)?;
                    record
                })
            })
            .map(|id| self.row(id, false).with_undecided(true))
            .map_err(|error| self.failure(error))
    }

    fn retain_in_order(
        &self,
        record: &EvidenceRecord,
        order: Option<&purrdf_core::evidence::StandpointOrder<purrdf_hash::hex::Digest32>>,
    ) -> Result<RowEvidence, EvalError> {
        self.require_reasons(record.reasons().iter().copied())?;
        let mut table = self.state.records.lock().map_err(|_| {
            self.invariant(crate::error::EvaluatorInvariant::EvidenceDictionaryPoisoned)
        })?;
        if record.is_default() {
            table
                .get(EvidenceId::DEFAULT)
                .map_err(|error| self.failure(error))?;
            return Ok(RowEvidence::default());
        }
        table
            .intern(|memory| {
                if let Some(order) = order {
                    return record.inherited(order, memory);
                }
                record
                    .combine(
                        &EvidenceRecord::default(),
                        EvidenceCombination::Conjunction,
                        memory,
                    )
                    .map_err(EvidenceRecordError::Storage)
            })
            .map(|id| self.row(id, false))
            .map_err(|error| self.failure(error))
    }

    fn require_reasons(&self, ids: impl IntoIterator<Item = ReasonId>) -> Result<(), EvalError> {
        let table = self.state.reasons.lock().map_err(|_| {
            self.invariant(crate::error::EvaluatorInvariant::ReasonDictionaryPoisoned)
        })?;
        for id in ids {
            table.get(id).map_err(|error| self.reason_failure(error))?;
        }
        drop(table);
        Ok(())
    }

    fn retain_reason<'a>(
        &self,
        id: ReasonId,
        source: impl FnOnce() -> Result<Option<ReasonDescription<'a>>, EvalError>,
    ) -> Result<(), EvalError> {
        let table = self.state.reasons.lock().map_err(|_| {
            self.invariant(crate::error::EvaluatorInvariant::ReasonDictionaryPoisoned)
        })?;
        match table.get(id) {
            Ok(_) => return Ok(()),
            Err(ReasonTableError::UnknownReason(_)) => {}
            Err(error) => return Err(self.reason_failure(error)),
        }
        drop(table);
        let description = source()?
            .ok_or_else(|| self.invariant(crate::error::EvaluatorInvariant::UnknownClaimReason))?;
        self.publish_reason(description, id).map(|_| ())
    }

    /// Admit a candidate constraint's original reason into this query's ledger.
    ///
    /// Identity is computed from the supplied RDF description under the native
    /// query owner, independently of candidate keys, scores and insertion order.
    /// Repeated descriptions share one retained dictionary entry.
    ///
    /// # Errors
    /// Returns the original payload, identity-scratch or dictionary refusal.
    pub fn declare_reason(
        &self,
        description: ReasonDescription<'_>,
    ) -> Result<ReasonId, EvalError> {
        let mut frame = LexicalFrame::new(&self.state.workspace);
        let id = description
            .identity(&mut Memory::new(&mut frame))
            .map_err(|error| self.reason_identity_failure(&mut frame, error))?;
        self.retain_reason(id, || Ok(Some(description)))?;
        Ok(id)
    }

    fn reason_identity_failure(&self, frame: &mut LexicalFrame, error: ReasonError) -> EvalError {
        match error {
            ReasonError::Storage(storage) => {
                frame.storage_error(storage, "native reason identity scratch")
            }
            error => self.invariant(error),
        }
    }

    fn publish_reason(
        &self,
        description: ReasonDescription<'_>,
        expected: ReasonId,
    ) -> Result<ReasonId, EvalError> {
        let workspace = &self.state.workspace;
        let cause = workspace.clone_term(description.cause)?;
        let concern = workspace.clone_term(description.concern)?;
        let constraint = description
            .constraint
            .map(|term| workspace.clone_term(term))
            .transpose()?;
        let vault = description
            .vault
            .map(|term| workspace.clone_term(term))
            .transpose()?;
        let watermark = description
            .watermark
            .map(|term| workspace.clone_term(term))
            .transpose()?;
        let mut frame = LexicalFrame::new(workspace);
        let reason = RetainedReason::from_parts(
            cause,
            concern,
            constraint,
            vault,
            watermark,
            &mut Memory::new(&mut frame),
        )
        .map_err(|error| self.reason_identity_failure(&mut frame, error))?;
        let id = reason.identity();
        if expected != id {
            return Err(self.invariant(crate::error::EvaluatorInvariant::ReasonIdentityMismatch));
        }
        let mut table = self.state.reasons.lock().map_err(|_| {
            self.invariant(crate::error::EvaluatorInvariant::ReasonDictionaryPoisoned)
        })?;
        table
            .intern(reason)
            .map_err(|error| self.reason_failure(error))?;
        drop(table);
        Ok(id)
    }

    fn reason_failure(&self, error: ReasonTableError) -> EvalError {
        let Ok(mut admission) = self.state.reason_admission.0.lock() else {
            return self.invariant(crate::error::EvaluatorInvariant::ReasonAdmissionPoisoned);
        };
        if let Some(original) = admission.take_failure() {
            return original;
        }
        match error {
            ReasonTableError::Storage(storage) => {
                admission.storage_error(storage, "native reason dictionary")
            }
            error => self.invariant(error),
        }
    }

    fn owns(&self, row: &RowEvidence) -> bool {
        row.owner.as_ref().is_none_or(|owner| {
            std::ptr::eq(
                std::ptr::from_ref(&*self.state),
                std::ptr::from_ref(&*owner.state),
            )
        })
    }

    /// Apply the operator law to ids belonging to this original query.
    ///
    /// # Errors
    /// Refuses detached/cross-query ids and all native publication failures.
    pub fn combine(
        &self,
        left: &RowEvidence,
        right: &RowEvidence,
        law: EvidenceCombination,
    ) -> Result<RowEvidence, EvalError> {
        if !self.owns(left) || !self.owns(right) {
            return Err(self.invariant(crate::error::EvaluatorInvariant::EvidenceRowsForeignOwners));
        }
        let mut table = self.state.records.lock().map_err(|_| {
            self.invariant(crate::error::EvaluatorInvariant::EvidenceDictionaryPoisoned)
        })?;
        table
            .combine(left.id, right.id, law)
            .map(|id| {
                self.row(id, left.owner.is_some() || right.owner.is_some())
                    .with_membership_of(left, right, law)
            })
            .map_err(|error| self.failure(error))
    }

    /// Change search completeness without manufacturing a polarity witness.
    ///
    /// # Errors
    /// Returns native refusal or an invalid query-local owner.
    pub fn with_completeness(
        &self,
        row: &RowEvidence,
        completeness: SearchCompleteness,
        reason: Option<ReasonId>,
    ) -> Result<RowEvidence, EvalError> {
        if !self.owns(row) {
            return Err(self.invariant(crate::error::EvaluatorInvariant::EvidenceRowForeignOwner));
        }
        self.require_reasons(reason)?;
        self.transform(row, |record, memory| {
            record.with_completeness(completeness, reason, memory)
        })
    }

    fn empty_existential(&self, search: &RowEvidence) -> Result<RowEvidence, EvalError> {
        // This is the predicate over an actually empty invocation, not a bag
        // summary. Its completed closed search proves EXISTS false; an open or
        // incomplete search supplies no polarity witness.
        let closed = !search.is_undecided()
            && search.permits_closed_absence()
            && search.with_record(EvidenceRecord::completeness)? == SearchCompleteness::Complete;
        let information = if closed {
            purrdf_core::evidence::Information::Opposed
        } else {
            purrdf_core::evidence::Information::Neither
        };
        let empty = self.transform(&RowEvidence::default(), |record, memory| {
            EvidenceRecord::from_parts(
                &purrdf_core::evidence::InformationMap::uniform(information),
                record.completeness(),
                record.frame(),
                record.reasons(),
                record.aggregate_bounds(),
                memory,
            )
        })?;
        empty
            .combine(search, EvidenceCombination::Conjunction)
            .map(|row| row.with_undecided(!closed))
    }

    fn aggregate_evidence(
        &self,
        row: &RowEvidence,
        aggregates: &[(
            purrdf_sparql_algebra::Variable,
            purrdf_sparql_algebra::AggregateExpression,
        )],
        workspace: &WorkspaceCapability,
        mut sum_nonnegative: impl FnMut(
            &purrdf_sparql_algebra::AggregateExpression,
        ) -> Result<bool, EvalError>,
    ) -> Result<RowEvidence, EvalError> {
        use purrdf_core::evidence::{AggregateLaw, aggregate_column_identity};
        use purrdf_sparql_algebra::AggregateFunction;
        let completeness =
            row.with_record(EvidenceRecord::completeness)?
                .worse(if row.permits_closed_absence() {
                    SearchCompleteness::Complete
                } else {
                    SearchCompleteness::Undetermined
                });
        let mut bounds = crate::AdmittedVec::with_capacity(aggregates.len(), workspace)?;
        for (column, aggregate) in aggregates {
            let law = match aggregate.function() {
                AggregateFunction::Count | AggregateFunction::Max => AggregateLaw::Increasing,
                AggregateFunction::Min => AggregateLaw::Decreasing,
                AggregateFunction::Sum if sum_nonnegative(aggregate)? => AggregateLaw::Increasing,
                _ => AggregateLaw::NonMonotone,
            };
            bounds.push((
                aggregate_column_identity(column.as_str()),
                law.bound(completeness),
            ))?;
        }
        self.with_aggregate_bounds(row, &bounds)
    }

    fn with_aggregate_bounds(
        &self,
        row: &RowEvidence,
        bounds: &[(
            purrdf_hash::hex::Digest32,
            purrdf_core::evidence::AggregateBound,
        )],
    ) -> Result<RowEvidence, EvalError> {
        self.transform(row, |record, memory| {
            record.with_aggregate_bounds(bounds, memory)
        })
    }

    fn transform(
        &self,
        row: &RowEvidence,
        update: impl FnOnce(
            &EvidenceRecord,
            &mut Memory<'_, EvidenceAdmission>,
        ) -> Result<EvidenceRecord, EvidenceRecordError>,
    ) -> Result<RowEvidence, EvalError> {
        if !self.owns(row) {
            return Err(self.invariant(crate::error::EvaluatorInvariant::EvidenceRowForeignOwner));
        }
        let mut table = self.state.records.lock().map_err(|_| {
            self.invariant(crate::error::EvaluatorInvariant::EvidenceDictionaryPoisoned)
        })?;
        table
            .transform(row.id, update)
            .map(|id| {
                self.row(id, row.owner.is_some())
                    .with_undecided(row.undecided)
                    .with_unclosed_search(row.unclosed_search)
                    .with_rank_of(row)
            })
            .map_err(|error| self.failure(error))
    }
}

/// Immutable per-row metadata retaining its original query dictionary.
/// Ordinary rows use the virtual default id and allocate no metadata payload.
#[derive(Clone, Debug, Default)]
pub struct RowEvidence {
    id: EvidenceId,
    owner: Option<QueryEvidence>,
    undecided: bool,
    // An absence licence is independent of FOUR, frame and completeness. This
    // flag also travels on an empty invocation's original search carrier.
    unclosed_search: bool,
    // Original possible-candidate rank stamped before membership filtering.
    // It is a producer observation, not a reason id or composition position.
    original_rank: Option<core::num::NonZeroU64>,
}

pub(crate) static VIRTUAL_ROW_EVIDENCE: RowEvidence = RowEvidence::VIRTUAL;

impl RowEvidence {
    /// Native witnesses carry their own owner and completeness. A completed,
    /// closed empty invocation opposes EXISTS; other empty searches retain their
    /// original uncertainty without inventing an external scope or reason.
    pub(crate) fn native_existential<'r>(
        witnesses: impl IntoIterator<Item = &'r Self>,
        search: &Self,
        fallback_owner: impl FnOnce() -> Result<QueryEvidence, EvalError>,
    ) -> Result<Self, EvalError> {
        if let Some(record) = Self::existential_witnesses(witnesses, search)? {
            return Ok(record);
        }
        let fallback;
        let owner = match &search.owner {
            Some(owner) => owner,
            None => {
                fallback = fallback_owner()?;
                &fallback
            }
        };
        owner.empty_existential(search)
    }

    /// Negation consumes only the original native closure and completeness.
    /// An incomplete declaration remains a membership gap, with its owner intact.
    pub(crate) fn native_negative(
        &self,
        fallback_owner: impl FnOnce() -> Result<QueryEvidence, EvalError>,
    ) -> Result<Self, EvalError> {
        if self.is_undecided()
            || !self.permits_closed_absence()
            || self.with_record(EvidenceRecord::completeness)? != SearchCompleteness::Complete
        {
            return Ok(self.clone().with_undecided(true));
        }
        let fallback;
        let owner = match &self.owner {
            Some(owner) => owner,
            None => {
                fallback = fallback_owner()?;
                &fallback
            }
        };
        owner.transform(self, EvidenceRecord::negated)
    }

    pub(crate) fn native_absence(&self) -> Result<Self, EvalError> {
        Ok(self.clone().with_undecided(
            self.is_undecided()
                || !self.permits_closed_absence()
                || self.with_record(EvidenceRecord::completeness)? != SearchCompleteness::Complete,
        ))
    }

    pub(crate) fn native_aggregate(
        &self,
        aggregates: &[(
            purrdf_sparql_algebra::Variable,
            purrdf_sparql_algebra::AggregateExpression,
        )],
        workspace: &WorkspaceCapability,
        fallback_owner: impl FnOnce() -> Result<QueryEvidence, EvalError>,
    ) -> Result<Self, EvalError> {
        if self.is_virtual() || aggregates.is_empty() {
            return Ok(self.clone());
        }
        let fallback;
        let owner = match &self.owner {
            Some(owner) => owner,
            None => {
                fallback = fallback_owner()?;
                &fallback
            }
        };
        owner.aggregate_evidence(self, aggregates, workspace, |_| Ok(false))
    }

    /// Coalesce actual existential witnesses without weakening a found witness.
    /// Retain original search ownership and the closure needed by negation;
    /// incomplete search affects absence and totals, not a witnessed positive.
    /// An empty bag carries no polarity witness; its absence licence belongs to
    /// the caller's declared scope rather than this combination kernel.
    pub(crate) fn existential_witnesses<'r>(
        witnesses: impl IntoIterator<Item = &'r Self>,
        search: &Self,
    ) -> Result<Option<Self>, EvalError> {
        let mut combined: Option<Self> = None;
        for witness in witnesses {
            combined = Some(match combined {
                None => witness.clone(),
                Some(previous) => previous.combine(witness, EvidenceCombination::Alternatives)?,
            });
        }
        let Some(mut record) = combined else {
            return Ok(None);
        };
        if !search.is_virtual() {
            if let Some(owner) = &record.owner {
                if !owner.owns(search) {
                    return Err(owner.invariant(
                        crate::error::EvaluatorInvariant::ExistentialSearchForeignOwner,
                    ));
                }
            } else if let Some(owner) = &search.owner {
                if !owner.owns(&record) {
                    return Err(owner.invariant(
                        crate::error::EvaluatorInvariant::ExistentialWitnessForeignOwner,
                    ));
                }
                record.owner = Some(owner.clone());
            }
            record.unclosed_search |= search.unclosed_search
                || search.with_record(EvidenceRecord::completeness)?
                    != SearchCompleteness::Complete;
        }
        Ok(Some(record))
    }

    pub(crate) const VIRTUAL: Self = Self {
        id: EvidenceId::DEFAULT,
        owner: None,
        undecided: false,
        unclosed_search: false,
        original_rank: None,
    };

    /// Whether no original query owner, rank or non-default declaration needs
    /// transport. This physical law is stronger than semantic [`Self::is_default`].
    #[must_use]
    pub fn is_virtual(&self) -> bool {
        self.is_default() && self.owner.is_none() && self.original_rank.is_none()
    }

    pub(crate) fn same_attachment(&self, other: &Self) -> bool {
        self.id == other.id
            && self.undecided == other.undecided
            && self.unclosed_search == other.unclosed_search
            && self.original_rank == other.original_rank
            && match (&self.owner, &other.owner) {
                (Some(left), Some(right)) => std::ptr::eq(
                    std::ptr::from_ref(&*left.state),
                    std::ptr::from_ref(&*right.state),
                ),
                (None, None) => true,
                _ => false,
            }
    }

    fn with_undecided(mut self, undecided: bool) -> Self {
        self.undecided = undecided;
        self
    }

    /// Carry the original producer's closure declaration on its search handle.
    /// An unclosed search cannot establish closed absence or an exact aggregate;
    /// the original query owner, record and ranks remain unchanged.
    #[must_use]
    pub fn with_unclosed_search(mut self, unclosed: bool) -> Self {
        self.unclosed_search = unclosed;
        self
    }

    fn with_rank_of(mut self, row: &Self) -> Self {
        self.original_rank = row.original_rank;
        self
    }

    /// Stamp the original scored position before scope rejection or depth.
    /// This never renumbers established stream ranks or changes a score.
    #[must_use]
    pub const fn with_original_rank(mut self, rank: core::num::NonZeroU64) -> Self {
        self.original_rank = Some(rank);
        self
    }

    /// The producer's original possible-candidate rank, when retained.
    #[must_use]
    pub const fn original_rank(&self) -> Option<core::num::NonZeroU64> {
        self.original_rank
    }

    fn with_membership_of(mut self, left: &Self, right: &Self, law: EvidenceCombination) -> Self {
        self.original_rank = match (left.original_rank, right.original_rank) {
            (Some(left), Some(right)) if left != right => None,
            (left, right) => left.or(right),
        };
        (self.undecided, self.unclosed_search) = match law {
            EvidenceCombination::Conjunction => (
                left.undecided || right.undecided,
                left.unclosed_search || right.unclosed_search,
            ),
            EvidenceCombination::Alternatives => (
                left.undecided && right.undecided,
                left.unclosed_search && right.unclosed_search,
            ),
        };
        self
    }

    /// Whether the original search declares the closure required for absence.
    /// Search completeness and information remain separate record declarations.
    #[must_use]
    pub const fn permits_closed_absence(&self) -> bool {
        !self.unclosed_search
    }

    /// Whether a declared expression gap prevents establishing row membership.
    /// This is independent of information polarity and search completeness.
    #[must_use]
    pub const fn is_undecided(&self) -> bool {
        self.undecided
    }
    pub(crate) fn alias_column(&self, source: &str, target: &str) -> Result<Self, EvalError> {
        use purrdf_core::evidence::aggregate_column_identity;
        let Some(owner) = &self.owner else {
            return Ok(self.clone());
        };
        let source = aggregate_column_identity(source);
        if !self.with_record(|record| {
            record
                .aggregate_bounds()
                .iter()
                .any(|(column, _)| *column == source)
        })? {
            return Ok(self.clone());
        }
        owner.transform(self, |record, memory| {
            record.with_column_alias(source, aggregate_column_identity(target), memory)
        })
    }

    pub(crate) fn project_columns(
        &self,
        columns: &[purrdf_hash::hex::Digest32],
    ) -> Result<Self, EvalError> {
        let Some(owner) = &self.owner else {
            return Ok(self.clone());
        };
        if self.with_record(|record| {
            record
                .aggregate_bounds()
                .iter()
                .all(|(column, _)| columns.binary_search(column).is_ok())
        })? {
            return Ok(self.clone());
        }
        owner.transform(self, |record, memory| {
            record.with_selected_bounds(|column| columns.binary_search(&column).is_ok(), memory)
        })
    }
    /// Whether the row uses the ordinary supported/complete/actual default.
    #[must_use]
    pub const fn is_default(&self) -> bool {
        self.id.is_default() && !self.undecided && !self.unclosed_search
    }

    /// Borrow the original immutable record while keeping its native owner live.
    /// References cannot escape the read callback or dictionary guard.
    ///
    /// # Errors
    /// Refuses a poisoned/aborted table or invalid query-local id.
    pub fn with_record<R>(&self, read: impl FnOnce(&EvidenceRecord) -> R) -> Result<R, EvalError> {
        let Some(owner) = &self.owner else {
            return Ok(read(&EvidenceRecord::default()));
        };
        let table = owner.state.records.lock().map_err(|_| {
            owner.invariant(crate::error::EvaluatorInvariant::EvidenceDictionaryPoisoned)
        })?;
        let record = table.get(self.id).map_err(|error| owner.failure(error))?;
        let result = read(record);
        drop(table);
        Ok(result)
    }

    /// Borrow a canonical reason through this row's original native owner.
    /// No owned RDF terms or rendered ledger are constructed for inspection.
    ///
    /// # Errors
    /// Refuses unknown identities, absent owners and poisoned reason tables.
    pub fn with_reason<R>(
        &self,
        id: ReasonId,
        read: impl FnOnce(ReasonDescription<'_>) -> R,
    ) -> Result<R, EvalError> {
        let Some(owner) = &self.owner else {
            return Err(NativeDiagnostic::error_cause(
                NativeDiagnosticKind::Internal,
                crate::error::EvaluatorInvariant::ReasonOwnerMissing,
                &WorkspaceCapability::default(),
            ));
        };
        let table = owner.state.reasons.lock().map_err(|_| {
            owner.invariant(crate::error::EvaluatorInvariant::ReasonDictionaryPoisoned)
        })?;
        let reason = table.get(id).map_err(|error| owner.reason_failure(error))?;
        let result = read(reason.description());
        drop(table);
        Ok(result)
    }

    /// Combine dependencies while preserving the original table owner.
    ///
    /// # Errors
    /// Returns cross-query, original storage or native operational failures.
    pub fn combine(&self, other: &Self, law: EvidenceCombination) -> Result<Self, EvalError> {
        match self.owner.as_ref().or(other.owner.as_ref()) {
            Some(owner) => owner.combine(self, other, law),
            None => Ok(Self::default().with_membership_of(self, other, law)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_core::evidence::{Frame, InformationMap};
    use purrdf_lex::allocation::{Memory, Resident};

    fn incomplete_record() -> EvidenceRecord {
        EvidenceRecord::from_parts(
            &InformationMap::default(),
            SearchCompleteness::Undetermined,
            &Frame::default(),
            &[],
            &[],
            &mut Memory::new(&mut Resident),
        )
        .expect("native declaration")
    }

    #[test]
    fn native_empty_witness_preserves_search_owner_and_aggregate_closure() {
        use purrdf_core::evidence::{AggregateBound, Information, aggregate_column_identity};
        use purrdf_sparql_algebra::{AggregateExpression, AggregateFunction, Variable};
        let workspace = WorkspaceCapability::default();
        let query = QueryEvidence::new(&workspace).unwrap();
        let aggregates = [(
            Variable::new("count"),
            AggregateExpression::new(
                AggregateFunction::Count,
                Vec::new(),
                Vec::new(),
                Vec::new(),
                false,
            )
            .unwrap(),
        )];
        let virtual_search = query.retain(&EvidenceRecord::default()).unwrap();
        assert!(virtual_search.is_virtual());
        assert!(
            virtual_search
                .native_aggregate(&aggregates, &workspace, || Ok(query.clone()))
                .unwrap()
                .is_virtual()
        );
        for (incomplete, unclosed) in [(false, false), (true, false), (false, true)] {
            let record = if incomplete {
                incomplete_record()
            } else {
                EvidenceRecord::from_parts(
                    &InformationMap::default(),
                    SearchCompleteness::Complete,
                    &Frame::default(),
                    &[],
                    &[(aggregate_column_identity("source"), AggregateBound::Exact)],
                    &mut Memory::new(&mut Resident),
                )
                .unwrap()
            };
            let search = query
                .retain(&record)
                .unwrap()
                .with_unclosed_search(unclosed);
            assert!(!search.is_virtual());
            let witness =
                RowEvidence::native_existential(std::iter::empty(), &search, || Ok(query.clone()))
                    .unwrap();
            assert!(query.owns(&witness));
            assert_eq!(witness.permits_closed_absence(), !unclosed);
            assert_eq!(witness.is_undecided(), incomplete || unclosed);
            witness
                .with_record(|retained| {
                    assert_eq!(
                        retained.information().fallback(),
                        if incomplete || unclosed {
                            Information::Neither
                        } else {
                            Information::Opposed
                        }
                    );
                    assert_eq!(retained.completeness(), record.completeness());
                })
                .unwrap();
            let found =
                RowEvidence::native_existential([&virtual_search], &search, || Ok(query.clone()))
                    .unwrap();
            assert!(query.owns(&found));
            found
                .with_record(|retained| {
                    assert_eq!(retained.information().fallback(), Information::Supported);
                    assert_eq!(retained.completeness(), SearchCompleteness::Complete);
                })
                .unwrap();
            assert_eq!(found.permits_closed_absence(), !(incomplete || unclosed));
            assert_eq!(
                found
                    .native_negative(|| Ok(query.clone()))
                    .unwrap()
                    .is_undecided(),
                incomplete || unclosed,
            );
            let bounds = search
                .native_aggregate(&aggregates, &workspace, || Ok(query.clone()))
                .unwrap();
            bounds
                .with_record(|retained| {
                    assert_eq!(
                        retained.aggregate_bounds(),
                        &[(
                            aggregate_column_identity("count"),
                            if incomplete || unclosed {
                                AggregateBound::Lower
                            } else {
                                AggregateBound::Exact
                            }
                        )]
                    );
                })
                .unwrap();
        }
        let ownerless = RowEvidence::default().with_unclosed_search(true);
        let bounds = ownerless
            .native_aggregate(&aggregates, &workspace, || Ok(query.clone()))
            .unwrap();
        assert!(!bounds.permits_closed_absence());
        bounds
            .with_record(|record| {
                assert_eq!(
                    record.aggregate_bounds(),
                    &[(aggregate_column_identity("count"), AggregateBound::Lower)]
                );
            })
            .unwrap();
    }

    #[test]
    fn candidate_reason_and_undecided_membership_keep_the_original_query_owner() {
        let query = QueryEvidence::new(&WorkspaceCapability::default()).unwrap();
        let cause = purrdf_core::TermValue::iri("https://example.org/missing-time");
        let concern = purrdf_core::TermValue::Triple {
            s: purrdf_core::TermBox::new(purrdf_core::TermValue::iri(
                "https://example.org/candidate",
            )),
            p: purrdf_core::TermBox::new(purrdf_core::TermValue::iri(
                "https://example.org/required-time",
            )),
            o: purrdf_core::TermBox::new(purrdf_core::TermValue::simple_literal("absent")),
        };
        let description = ReasonDescription {
            cause: &cause,
            concern: &concern,
            constraint: None,
            vault: None,
            watermark: None,
        };
        let reason = query.declare_reason(description).unwrap();
        assert_eq!(query.declare_reason(description).unwrap(), reason);
        let record = EvidenceRecord::from_parts(
            &InformationMap::default(),
            SearchCompleteness::Undetermined,
            &Frame::default(),
            &[reason],
            &[],
            &mut Memory::new(&mut Resident),
        )
        .unwrap();
        let row = query.retain_undecided(&record).unwrap();
        assert!(row.is_undecided());
        assert!(query.retain_undecided(&EvidenceRecord::default()).is_err());
        drop(query);
        row.with_record(|retained| assert_eq!(retained, &record))
            .unwrap();
        row.with_reason(reason, |retained| {
            assert_eq!(retained.cause, &cause);
            assert_eq!(retained.concern, &concern);
        })
        .unwrap();
    }

    #[test]
    fn retained_row_clone_keeps_the_original_dictionary() {
        let workspace = WorkspaceCapability::default();
        let query = QueryEvidence::new(&workspace).expect("query owner");
        let source = incomplete_record();
        let evidence = query.retain(&source).expect("retained evidence");
        let row = crate::solution::RetainedRow::<purrdf_core::TermId>::from_resident(
            crate::solution::Solution::new(),
        )
        .with_evidence(evidence, &workspace)
        .expect("retained metadata admission");
        let clone = row.clone_admitted(&workspace).expect("original row owner");
        drop(row);
        drop(query);
        clone
            .evidence()
            .with_record(|record| {
                assert_eq!(record.completeness(), SearchCompleteness::Undetermined);
                assert_eq!(record, &source);
            })
            .expect("retained record");
        let (_, evidence) = clone
            .into_resident_parts()
            .expect("resident transport moves both owners");
        evidence
            .with_record(|record| assert_eq!(record, &source))
            .expect("cell transport preserves its original evidence owner");

        let owner = QueryEvidence::new(&workspace).expect("default record's explicit owner");
        let declared = owner
            .with_completeness(
                &owner.retain(&incomplete_record()).unwrap(),
                SearchCompleteness::Complete,
                None,
            )
            .unwrap();
        assert!(declared.is_default());
        assert!(!declared.is_virtual());
        assert!(owner.owns(&declared));
        let combined = declared
            .combine(&RowEvidence::default(), EvidenceCombination::Conjunction)
            .unwrap();
        assert!(owner.owns(&combined));
        assert!(!combined.is_virtual());
        let destination = QueryEvidence::new(&workspace).unwrap();
        let imported = destination.import(&declared).unwrap();
        assert!(imported.is_default());
        assert!(!imported.is_virtual());
        assert!(destination.owns(&imported));
        assert!(!owner.owns(&imported));
        for attachment in [
            declared.clone(),
            RowEvidence::default().with_original_rank(core::num::NonZeroU64::new(1).unwrap()),
        ] {
            let raw = crate::AdmittedPfRow::new(Vec::new(), workspace.charge(0).unwrap())
                .with_evidence(attachment);
            assert!(matches!(
                raw.try_into_resident(),
                Err(EvalError::WorkspaceUnpriced(
                    "raw property-function cells cannot transport original row evidence"
                ))
            ));
        }
        assert_eq!(
            crate::AdmittedPfRow::new(Vec::new(), workspace.charge(0).unwrap())
                .try_into_resident()
                .unwrap(),
            []
        );
        let declared = declared.with_original_rank(core::num::NonZeroU64::new(1).unwrap());
        let row = crate::solution::RetainedRow::<purrdf_core::TermId>::from_resident(
            crate::solution::Solution::new(),
        )
        .with_evidence(declared.clone(), &workspace)
        .expect("default-supported rank carrier");
        let clone = row
            .clone_admitted(&workspace)
            .expect("shared cell transport");
        drop(row);
        let updated = clone
            .with_evidence(
                declared.with_original_rank(core::num::NonZeroU64::new(2).unwrap()),
                &workspace,
            )
            .expect("rank update must not reuse semantic-default metadata");
        assert_eq!(updated.evidence().original_rank().unwrap().get(), 2);
        assert!(owner.owns(updated.evidence()));
        assert!(updated.evidence().owner.is_some());
        let copied = crate::solution::RetainedRow::<purrdf_core::TermId>::from_resident(
            crate::solution::Solution::new(),
        )
        .with_evidence_from(&updated, &workspace)
        .expect("copy preserves a physical default-supported owner and rank");
        drop(updated);
        assert!(owner.owns(copied.evidence()));
        assert!(copied.evidence().owner.is_some());
        let updated = copied;
        let rows = [updated];
        owner
            .with_accepted_payloads(
                &rows,
                crate::solution::RetainedRow::evidence,
                &[],
                |merged| {
                    let accepted: Vec<_> = merged
                        .map(|row| row.expect("original default record merge"))
                        .collect();
                    assert_eq!(accepted.len(), 1);
                    assert!(std::ptr::eq(accepted[0].row, &raw const rows[0]));
                    assert!(!accepted[0].is_accepted());
                    assert_eq!(accepted[0].row.evidence().original_rank().unwrap().get(), 2);
                    assert!(accepted[0].evidence.is_default());
                },
            )
            .expect("original acceptance keeps default-supported ranks and owner");
    }

    #[test]
    fn native_table_rejects_foreign_query_ids() {
        let workspace = WorkspaceCapability::default();
        let left = QueryEvidence::new(&workspace).expect("left query");
        let right = QueryEvidence::new(&workspace).expect("right query");
        let record = incomplete_record();
        let left_row = left.retain(&record).expect("left declaration");
        let right_row = right.retain(&record).expect("right declaration");
        assert!(
            left_row
                .combine(&right_row, EvidenceCombination::Conjunction)
                .is_err()
        );
    }

    #[test]
    fn default_rows_do_not_materialize_a_table() {
        let ordinary = RowEvidence::default();
        let combined = ordinary
            .combine(&ordinary, EvidenceCombination::Alternatives)
            .expect("virtual default");
        assert!(combined.owner.is_none());
        assert!(combined.is_default());
        combined
            .with_record(|record| assert!(record.is_default()))
            .expect("virtual record");
    }
}
