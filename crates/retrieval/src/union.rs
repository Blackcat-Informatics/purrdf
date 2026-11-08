// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The unscored candidate-prefix union over the native ranked read protocol.

use crate::candidate::CandidateDepths;
use crate::canonical::Writer;
use crate::execute::{CandidateExecutionResult, ReadStratum};
use crate::fusion_stream::ProducerStatus;
use crate::id::PlanId;
use crate::iri::{Iri, Term};
use crate::plan::{ProducerBinding, ProducerDecision, UnservedTerm};
use crate::ranked_stream::{
    ProtocolError, StreamContract, validate_candidate_block, validate_declared_block,
    validate_rank, validate_receipt,
};
use core::fmt;
use purrdf_sparql_eval::{AdmittedOrderedMap, AdmittedOrderedSet, AdmittedVec, SharedWorkspace};
use purrdf_sparql_eval::{Completeness, DuplicatePolicy, OrderFidelity, PfAttestation};

/// A subject in a set, carrying every original rank at which each producer named it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnionCandidate {
    /// Original ranks, in producer emission order. Allowed duplicates retain all
    /// their ranks; they do not produce duplicate subjects in the candidate set.
    pub ranks: SharedWorkspace<AdmittedOrderedMap<Iri, AdmittedVec<u64>>>,
    /// The first declared block placement, if a producer provided one.
    pub block: Option<(Iri, crate::DomainTag)>,
}

/// One producer's original work bound, observed read and evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnionProducer {
    /// Last valid producer rank observed; zero only before the first valid row.
    pub last_rank: u64,
    /// The caller's independent work depth, excluding the probe.
    pub requested_depth: u32,
    /// The actual terminal status, absent when an interrupted read has no receipt.
    pub status: Option<ProducerStatus>,
    /// Its admitted contract, including loss/order and domain evidence.
    pub contract: Option<StreamContract>,
    /// The original generation/service announcement, absent if no stream opened.
    pub announced: Option<PfAttestation>,
    /// The producer's final attestation, never substituted for its announcement.
    pub settled: Option<PfAttestation>,
    /// Rows pulled from the ranked stream, including a protocol-refused row.
    pub rows_pulled: u64,
    /// Actual rows produced by the native invocation, including its probe.
    /// Absent when the executor did not expose a readable invocation.
    pub rows_materialised: Option<u64>,
    /// Actual rows handed out by the native stream.
    pub rows_emitted: Option<u64>,
    /// Whether a terminal receipt was observed after None and count-verified.
    pub receipt_verified: bool,
    /// Whether this requested prefix was observed to its depth or verified exhaustion.
    /// This says nothing about corpus/index completeness or rank fidelity.
    pub completed_prefix: bool,
    /// The original read/protocol failure, with any partial ranks still in the set.
    pub read_error: Option<ProtocolError>,
    /// A settlement refusal, retained even when an earlier failure has precedence.
    pub settlement_error: Option<ProtocolError>,
}

impl UnionProducer {
    fn new(depth: u32, contract: Option<StreamContract>, status: Option<ProducerStatus>) -> Self {
        Self {
            requested_depth: depth,
            status,
            contract,
            announced: None,
            settled: None,
            rows_pulled: 0,
            last_rank: 0,
            rows_materialised: None,
            rows_emitted: None,
            receipt_verified: false,
            completed_prefix: false,
            read_error: None,
            settlement_error: None,
        }
    }

    fn certify_prefix(&mut self) {
        self.completed_prefix = self.read_error.is_none()
            && self.settlement_error.is_none()
            && self.receipt_verified
            && !matches!(
                self.status,
                Some(ProducerStatus::ExecutionFailed { .. } | ProducerStatus::TermsRejected)
            )
            && (self.rows_pulled == u64::from(self.requested_depth)
                || matches!(self.status, Some(ProducerStatus::Exhausted { .. })));
    }
}

impl purrdf_hash::error::StructuredError for UnionCandidate {
    fn visit<'error>(&'error self, visitor: &mut dyn purrdf_hash::error::ErrorVisitor<'error>) {
        use purrdf_hash::error::ErrorValue;
        let Self { ranks, block } = self;
        visitor.variant("retrieval-union-candidate");
        visitor.value(
            Some("block"),
            block
                .as_ref()
                .map_or(ErrorValue::Absent, |value| ErrorValue::Record(value)),
        );
        visitor.begin_list(Some("ranks"), ranks.len());
        for (producer, values) in ranks.iter() {
            visitor.begin_record(None, "retrieval-producer-ranks");
            visitor.value(Some("producer"), ErrorValue::Iri(producer.as_str()));
            purrdf_hash::error::visit_list(
                visitor,
                Some("ranks"),
                values
                    .iter()
                    .map(|rank| ErrorValue::Unsigned(u128::from(*rank))),
            );
            visitor.end_record();
        }
        visitor.end_list();
    }
}

impl purrdf_hash::error::StructuredError for UnionProducer {
    purrdf_hash::error_visit! {
        @record self { requested_depth, status, contract, announced, settled, rows_pulled, last_rank,
            rows_materialised, rows_emitted, receipt_verified, completed_prefix, read_error,
            settlement_error } => "retrieval-union-producer" {
            "requested_depth" => Unsigned(u128::from(self.requested_depth)),
            "status" => Value(self.status.as_ref().map_or(purrdf_hash::error::ErrorValue::Absent, |value| purrdf_hash::error::ErrorValue::Record(value))),
            "contract" => Value(self.contract.as_ref().map_or(purrdf_hash::error::ErrorValue::Absent, |value| purrdf_hash::error::ErrorValue::Record(value))),
            "announced" => Value(self.announced.as_ref().map_or(purrdf_hash::error::ErrorValue::Absent, |value| purrdf_hash::error::ErrorValue::Record(value))),
            "settled" => Value(self.settled.as_ref().map_or(purrdf_hash::error::ErrorValue::Absent, |value| purrdf_hash::error::ErrorValue::Record(value))),
            "rows_pulled" => Unsigned(u128::from(self.rows_pulled)),
            "last_rank" => Unsigned(u128::from(self.last_rank)),
            "rows_materialised" => Value(self.rows_materialised.map_or(purrdf_hash::error::ErrorValue::Absent, |value| purrdf_hash::error::ErrorValue::Unsigned(u128::from(value)))),
            "rows_emitted" => Value(self.rows_emitted.map_or(purrdf_hash::error::ErrorValue::Absent, |value| purrdf_hash::error::ErrorValue::Unsigned(u128::from(value)))),
            "receipt_verified" => Boolean(self.receipt_verified),
            "completed_prefix" => Boolean(self.completed_prefix),
            "read_error" => Value(self.read_error.as_ref().map_or(purrdf_hash::error::ErrorValue::Absent, |value| purrdf_hash::error::ErrorValue::Cause(value))),
            "settlement_error" => Value(self.settlement_error.as_ref().map_or(purrdf_hash::error::ErrorValue::Absent, |value| purrdf_hash::error::ErrorValue::Cause(value)))
        }
    }
}

/// A candidate set with no relevance score, top-k truncation or ranking.
///
/// Lexical key order is its canonical set representation, not a relevance order.
/// Every producer's independent prefix and original evidence remains inspectable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateUnion {
    /// The explicit candidate-plan identity.
    pub plan_id: PlanId,
    /// Distinct subjects and all their original producer ranks.
    pub candidates: SharedWorkspace<AdmittedOrderedMap<Term, UnionCandidate>>,
    /// Every selected producer, including one that failed before opening.
    pub producers: SharedWorkspace<AdmittedOrderedMap<Iri, UnionProducer>>,
    /// True only after every selected producer certifies its own requested prefix.
    pub completed_prefix: bool,
    /// Every request term/placement that no producer could serve.
    pub unserved_terms: SharedWorkspace<AdmittedVec<UnservedTerm>>,
    /// Original term-to-producer binding records.
    pub producer_bindings: SharedWorkspace<Vec<ProducerBinding>>,
    /// Original selection/rejection decisions.
    pub producer_decisions: SharedWorkspace<Vec<ProducerDecision>>,
}

impl purrdf_hash::error::StructuredError for CandidateUnion {
    fn visit<'error>(&'error self, visitor: &mut dyn purrdf_hash::error::ErrorVisitor<'error>) {
        use purrdf_hash::error::{ErrorValue, visit_list};
        let Self {
            plan_id,
            candidates,
            producers,
            completed_prefix,
            unserved_terms,
            producer_bindings,
            producer_decisions,
        } = self;
        visitor.variant("retrieval-candidate-union");
        visitor.value(Some("plan_id"), ErrorValue::Record(plan_id));
        visitor.value(
            Some("completed_prefix"),
            ErrorValue::Boolean(*completed_prefix),
        );
        visitor.begin_list(Some("candidates"), candidates.len());
        for (term, candidate) in candidates.iter() {
            visitor.begin_record(None, "retrieval-union-entry");
            visitor.value(Some("term"), ErrorValue::Term(term));
            visitor.value(Some("candidate"), ErrorValue::Record(candidate));
            visitor.end_record();
        }
        visitor.end_list();
        visitor.begin_list(Some("producers"), producers.len());
        for (iri, producer) in producers.iter() {
            visitor.begin_record(None, "retrieval-union-producer-entry");
            visitor.value(Some("producer"), ErrorValue::Iri(iri.as_str()));
            visitor.value(Some("observation"), ErrorValue::Record(producer));
            visitor.end_record();
        }
        visitor.end_list();
        visit_list(
            visitor,
            Some("unserved_terms"),
            unserved_terms.iter().map(|value| ErrorValue::Record(value)),
        );
        visit_list(
            visitor,
            Some("producer_bindings"),
            producer_bindings
                .iter()
                .map(|value| ErrorValue::Record(value)),
        );
        visit_list(
            visitor,
            Some("producer_decisions"),
            producer_decisions
                .iter()
                .map(|value| ErrorValue::Record(value)),
        );
    }
}

/// A refusal of an inconsistent candidate stage or the original ranked protocol.
#[derive(Debug)]
pub enum UnionError {
    /// Supplied work depths differ from the depths actually compiled/executed.
    DepthMismatch {
        /// The affected stratum.
        stratum: Iri,
        /// The actual execution depth, or absence.
        executed: Option<u32>,
        /// The requested union depth, or absence.
        requested: Option<u32>,
    },
    /// A stream, status or declaration does not belong to the actual selected set.
    InconsistentExecution {
        /// The exact inconsistent claim.
        reason: String,
    },
    /// A stream emitted a ranked row past the explicit prefix it was compiled for.
    RowsPastDepth {
        /// The producer.
        stratum: Iri,
        /// The actual row's rank.
        rank: u64,
        /// Physical rows pulled, which is what exceeds the requested row depth.
        rows: u64,
        /// The compiled caller depth.
        depth: u32,
    },
    /// The shared native ranked-stream protocol refused.
    Protocol(ProtocolError),
}

impl purrdf_hash::error::StructuredError for UnionError {
    purrdf_hash::error_visit! {
        DepthMismatch { stratum, executed, requested } => "retrieval-union-depth-mismatch" {
            "stratum" => Iri(stratum.as_str()),
            "executed" => Value(executed.map_or(purrdf_hash::error::ErrorValue::Absent, |value| purrdf_hash::error::ErrorValue::Unsigned(u128::from(value)))),
            "requested" => Value(requested.map_or(purrdf_hash::error::ErrorValue::Absent, |value| purrdf_hash::error::ErrorValue::Unsigned(u128::from(value)))),
        },
        InconsistentExecution { reason } => "retrieval-union-inconsistent-execution" { "reason" => Text(reason) },
        RowsPastDepth { stratum, rank, rows, depth } => "retrieval-union-rows-past-depth" {
            "stratum" => Iri(stratum.as_str()), "rank" => Unsigned(u128::from(*rank)), "rows" => Unsigned(u128::from(*rows)), "depth" => Unsigned(u128::from(*depth)),
        },
        Protocol(cause) => "retrieval-union-protocol" { "cause" => Cause(cause) },
    }
}

impl fmt::Display for UnionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DepthMismatch {
                stratum,
                executed,
                requested,
            } => write!(
                f,
                "candidate depth for {stratum} differs: executed {executed:?}, requested {requested:?}"
            ),
            Self::InconsistentExecution { reason } => f.write_str(reason),
            Self::RowsPastDepth {
                stratum,
                rank,
                rows,
                depth,
            } => write!(
                f,
                "candidate stratum {stratum} emitted row {rows} at rank {rank} past requested row depth {depth}"
            ),
            Self::Protocol(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for UnionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Protocol(error) => Some(error),
            _ => None,
        }
    }
}
purrdf_lex::variant_from!(UnionError { Protocol(ProtocolError) });

/// A whole-stage refusal with the original cause and all ranks/evidence observed so far.
#[derive(Debug)]
pub struct UnionFailure<'d> {
    /// The first whole-stage refusal; cleanup never replaces it.
    pub cause: UnionError,
    /// The incomplete observed set; absent only if admission failed before a read.
    pub partial: Option<CandidateUnion>,
    /// Original execution and settled reads retained through the refusal.
    /// This moves the already-admitted arena; no new evidence storage is
    /// allocated after refusal and every producer's settlement remains visible.
    pub execution: CandidateExecutionResult<'d>,
}

impl purrdf_hash::error::StructuredError for UnionFailure<'_> {
    purrdf_hash::error_visit! {
        @record self { cause, partial, execution } => "retrieval-union-failure" {
            "cause" => Cause(&self.cause),
            "partial" => Value(self.partial.as_ref().map_or(purrdf_hash::error::ErrorValue::Absent, |value| purrdf_hash::error::ErrorValue::Record(value))),
            "execution" => Record(&self.execution)
        }
    }
}
impl fmt::Display for UnionFailure<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.cause.fmt(f)
    }
}
impl std::error::Error for UnionFailure<'_> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.cause)
    }
}

/// Read each stratum to its explicit independent depth and union by subject.
///
/// The executor already bounds each actual native invocation and owns its probe;
/// this consumer reads its native ranks until None, then verifies its receipt.
/// It creates no fusion profile, contribution or score. A per-producer native
/// read failure preserves its partial ranks and lets healthy siblings answer.
/// A protocol breach or moving attestation returns the first cause plus partial
/// evidence after settling every invocation. No result can certify a prefix
/// while one stratum remains open, failed or stopped below its requested depth.
///
/// # Errors
/// A UnionFailure for mismatched depths, inconsistent execution metadata or an
/// original native protocol failure. Per-producer failures remain producer data.
#[expect(
    clippy::future_not_send,
    reason = "native on-demand reads retain Rc invocation state and are awaited in one task"
)]
#[expect(
    clippy::result_large_err,
    reason = "a native refusal must retain its partial evidence without allocating an error box after admission has stopped"
)]
pub async fn union<'d>(
    mut execution: CandidateExecutionResult<'d>,
    depths: &CandidateDepths,
) -> Result<CandidateUnion, UnionFailure<'d>> {
    let workspace = execution.streams.workspace().clone();
    let arenas = (|| {
        let candidates =
            SharedWorkspace::new_admitted(AdmittedOrderedMap::new(&workspace), &workspace)?;
        let producers =
            SharedWorkspace::new_admitted(AdmittedOrderedMap::new(&workspace), &workspace)?;
        Ok::<_, purrdf_sparql_eval::EvalError>((candidates, producers))
    })();
    let (candidates, producers) = match arenas {
        Ok(arenas) => arenas,
        Err(error) => {
            return Err(settle_admission_failure(execution, error).await);
        }
    };
    let mut result = CandidateUnion {
        plan_id: execution.plan_id,
        candidates,
        producers,
        completed_prefix: false,
        unserved_terms: execution.unserved_terms.clone(),
        producer_bindings: execution.producer_bindings.clone(),
        producer_decisions: execution.producer_decisions.clone(),
    };
    let mut primary = None;
    if primary.is_none() {
        let setup = (|| {
            let producers = result
                .producers
                .get_mut()
                .expect("unpublished producer arena is unique");
            for (stratum, depth) in execution.depths.iter() {
                producers.insert(
                    stratum.clone_admitted(&workspace)?,
                    UnionProducer::new(
                        depth,
                        execution.contracts.get(stratum).cloned(),
                        execution.statuses.get(stratum).cloned(),
                    ),
                )?;
            }
            for read in &execution.streams {
                if !producers.contains_key(&read.stratum) {
                    producers.insert(
                        read.stratum.clone_admitted(&workspace)?,
                        UnionProducer::new(read.requested_depth, Some(read.contract.clone()), None),
                    )?;
                }
                producers
                    .get_mut(&read.stratum)
                    .expect("actual selected producer")
                    .announced = Some(read.attestation.clone());
            }
            Ok::<(), purrdf_sparql_eval::EvalError>(())
        })();
        if let Err(error) = setup {
            primary = Some(ProtocolError::NativeFailure(error).into());
        }
    }
    if primary.is_none() {
        primary = check_execution(&execution, depths).err();
    }
    if primary.is_none() {
        'reads: for read in execution.streams.iter_mut() {
            loop {
                let pulled = read.stream.next().await;
                match pulled {
                    Ok(Some((rank, subject, block))) => {
                        let producer = result
                            .producers
                            .get_mut()
                            .expect("unpublished producer arena is unique")
                            .get_mut(&read.stratum)
                            .expect("the actual stream's producer was recorded");
                        let previous_rank = producer.last_rank;
                        producer.rows_pulled += 1;
                        let checked = validate_rank(read.contract.rank_law, previous_rank, rank)
                            .and_then(|()| {
                                validate_declared_block(
                                    &read.stratum,
                                    &read.contract.domains,
                                    &block,
                                    rank,
                                    || subject.clone(),
                                )
                            })
                            .and_then(|()| {
                                validate_candidate_block(
                                    &subject,
                                    &read.stratum,
                                    &block,
                                    result
                                        .candidates
                                        .get(&subject)
                                        .and_then(|candidate| candidate.block.as_ref())
                                        .map(|(stratum, block)| (stratum, block)),
                                )
                            });
                        if let Err(error) = checked {
                            producer.read_error = Some(error.clone());
                            primary = Some(error.into());
                            break 'reads;
                        }
                        producer.last_rank = rank;
                        if producer.rows_pulled > u64::from(producer.requested_depth) {
                            primary = Some(UnionError::RowsPastDepth {
                                stratum: read.stratum.clone(),
                                rank,
                                rows: producer.rows_pulled,
                                depth: producer.requested_depth,
                            });
                            break 'reads;
                        }
                        if read.contract.duplicates == DuplicatePolicy::Unique
                            && result.candidates.get(&subject).is_some_and(|candidate| {
                                candidate.ranks.contains_key(&read.stratum)
                            })
                        {
                            let error = ProtocolError::DuplicateItem {
                                item: subject.as_str().to_owned(),
                                stratum: read.stratum.as_str().to_owned(),
                            };
                            producer.read_error = Some(error.clone());
                            primary = Some(error.into());
                            break 'reads;
                        }
                        let recorded = (|| {
                            let candidates = result
                                .candidates
                                .get_mut()
                                .expect("unpublished candidate arena is unique");
                            if !candidates.contains_key(&subject) {
                                let ranks = SharedWorkspace::new_admitted(
                                    AdmittedOrderedMap::new(&workspace),
                                    &workspace,
                                )?;
                                candidates.insert(
                                    subject.clone_admitted(&workspace)?,
                                    UnionCandidate { ranks, block: None },
                                )?;
                            }
                            let candidate = candidates
                                .get_mut(&subject)
                                .expect("candidate installed before rank mutation");
                            let ranks = candidate
                                .ranks
                                .get_mut()
                                .expect("unpublished ranks arena is unique");
                            if !ranks.contains_key(&read.stratum) {
                                ranks.insert(
                                    read.stratum.clone_admitted(&workspace)?,
                                    AdmittedVec::new(&workspace),
                                )?;
                            }
                            ranks
                                .get_mut(&read.stratum)
                                .expect("rank array installed")
                                .push(rank)?;
                            if candidate.block.is_none()
                                && let Some(tag) = block.tag()
                            {
                                candidate.block = Some((
                                    read.stratum.clone_admitted(&workspace)?,
                                    tag.clone_admitted(&workspace)?,
                                ));
                            }
                            Ok::<(), purrdf_sparql_eval::EvalError>(())
                        })();
                        if let Err(error) = recorded {
                            let error = ProtocolError::NativeFailure(error);
                            producer.read_error = Some(error.clone());
                            primary = Some(error.into());
                            break 'reads;
                        }
                    }
                    Ok(None) => {
                        let producer = result
                            .producers
                            .get_mut()
                            .expect("unpublished producer arena is unique")
                            .get_mut(&read.stratum)
                            .expect("the actual stream's producer was recorded");
                        match read.stream.receipt().await.and_then(|receipt| {
                            validate_receipt(&receipt, producer.rows_pulled, producer.last_rank)?;
                            Ok(ProducerStatus::from(receipt))
                        }) {
                            Ok(status) => {
                                if producer
                                    .status
                                    .as_ref()
                                    .is_some_and(|known| known != &status)
                                {
                                    primary = Some(UnionError::InconsistentExecution {
                                        reason: format!(
                                            "stratum {} receipt disagrees with its executor status",
                                            read.stratum
                                        ),
                                    });
                                    break 'reads;
                                }
                                producer.status = Some(status);
                                producer.receipt_verified = true;
                            }
                            Err(error) => {
                                producer.read_error = Some(error.clone());
                                primary = Some(error.into());
                                break 'reads;
                            }
                        }
                        break;
                    }
                    Err(error) => {
                        let producer = result
                            .producers
                            .get_mut()
                            .expect("unpublished producer arena is unique")
                            .get_mut(&read.stratum)
                            .expect("the actual stream's producer was recorded");
                        producer.read_error = Some(error.clone());
                        if matches!(
                            error,
                            ProtocolError::ReadFailed { .. } | ProtocolError::NativeFailure(_)
                        ) && !read.stream.run_invalidated()
                        {
                            // A failure is a status, never a forged terminal receipt.
                            let workspace =
                                read.stream.workspace().cloned().unwrap_or_else(
                                    purrdf_sparql_eval::WorkspaceCapability::resident,
                                );
                            match ProducerStatus::failed_admitted(&error, &workspace) {
                                Ok(status) => producer.status = Some(status),
                                Err(error) => {
                                    let error = ProtocolError::NativeFailure(error);
                                    producer.read_error = Some(error.clone());
                                    primary = Some(error.into());
                                    break 'reads;
                                }
                            }
                            break;
                        }
                        primary = Some(error.into());
                        break 'reads;
                    }
                }
            }
        }
    }
    settle_reads(&mut execution.streams, &mut result, &mut primary).await;
    for producer in result
        .producers
        .get_mut()
        .expect("unpublished producer arena is unique")
        .values_mut()
    {
        producer.certify_prefix();
    }
    result.completed_prefix = primary.is_none()
        && result
            .producers
            .values()
            .all(|producer| producer.completed_prefix);
    match primary {
        Some(cause) => Err(UnionFailure {
            cause,
            partial: Some(result),
            execution,
        }),
        None => Ok(result),
    }
}

#[expect(
    clippy::future_not_send,
    reason = "settlement retains the same task-local native invocation owners"
)]
async fn settle_admission_failure(
    mut execution: CandidateExecutionResult<'_>,
    error: purrdf_sparql_eval::EvalError,
) -> UnionFailure<'_> {
    for read in execution.streams.iter_mut() {
        read.admission_failure_settlement = Some(read.stream.settle().await);
    }
    UnionFailure {
        cause: ProtocolError::NativeFailure(error).into(),
        partial: None,
        execution,
    }
}

#[expect(
    clippy::result_large_err,
    reason = "depth verification retains existing identifiers and native first refusal; failure cannot allocate a replacement owner"
)]
fn check_execution(
    execution: &CandidateExecutionResult<'_>,
    depths: &CandidateDepths,
) -> Result<(), UnionError> {
    for stratum in execution.depths.0.keys().chain(depths.0.keys()) {
        if execution.depths.get(stratum) != depths.get(stratum) {
            return Err(UnionError::DepthMismatch {
                stratum: stratum.clone(),
                executed: execution.depths.get(stratum),
                requested: depths.get(stratum),
            });
        }
    }
    let workspace = execution.streams.workspace();
    let mut seen = AdmittedOrderedSet::new(workspace);
    for read in &execution.streams {
        if !seen
            .insert(
                read.stratum
                    .clone_admitted(workspace)
                    .map_err(ProtocolError::NativeFailure)?,
            )
            .map_err(ProtocolError::NativeFailure)?
        {
            return Err(UnionError::InconsistentExecution {
                reason: format!("candidate stratum {} appears twice", read.stratum),
            });
        }
        if read.plan_id != execution.plan_id
            || execution.depths.get(&read.stratum) != Some(read.requested_depth)
            || execution.contracts.get(&read.stratum) != Some(&read.contract)
        {
            return Err(UnionError::InconsistentExecution {
                reason: format!(
                    "candidate stratum {} differs from its compiled attribution",
                    read.stratum
                ),
            });
        }
    }
    for stratum in execution.depths.0.keys() {
        if !execution.contracts.contains_key(stratum)
            || !seen.contains(stratum) && !execution.statuses.contains_key(stratum)
        {
            return Err(UnionError::InconsistentExecution {
                reason: format!(
                    "candidate stratum {stratum} has no admitted contract or read/status"
                ),
            });
        }
    }
    for stratum in execution.statuses.keys().chain(execution.contracts.keys()) {
        if execution.depths.get(stratum).is_none() {
            return Err(UnionError::InconsistentExecution {
                reason: format!("candidate evidence names unselected stratum {stratum}"),
            });
        }
    }
    Ok(())
}

#[expect(
    clippy::future_not_send,
    reason = "native invocation settlement shares Rc read state"
)]
async fn settle_reads(
    reads: &mut [ReadStratum<'_>],
    result: &mut CandidateUnion,
    primary: &mut Option<UnionError>,
) {
    for read in reads {
        let producer = result
            .producers
            .get_mut()
            .expect("unpublished producer arena is unique")
            .get_mut(&read.stratum);
        let Some(producer) = producer else {
            // Setup stopped at the first arena refusal. Its original error is
            // already primary; retain every remaining settlement in the existing
            // read slot instead of requiring another refused map insertion.
            assert!(
                primary.is_some(),
                "a missing producer follows setup refusal"
            );
            read.admission_failure_settlement = Some(read.stream.settle().await);
            continue;
        };
        producer.rows_materialised = Some(read.stream.rows_materialised());
        producer.rows_emitted = Some(read.stream.rows_emitted());
        let settlement = read.stream.settle().await.and_then(|settled| {
            if let Some(attestation) = &settled
                && *attestation != read.attestation
            {
                return Err(ProtocolError::AttestationMoved {
                    stratum: read.stratum.as_str().to_owned(),
                    reason: format!("announced {:?}, settled {attestation:?}", read.attestation),
                });
            }
            Ok(settled)
        });
        match settlement {
            Ok(settled) => producer.settled = settled,
            Err(error) => {
                producer.settlement_error = Some(error.clone());
                if primary.is_none() {
                    *primary = Some(error.into());
                }
            }
        }
    }
}

impl CandidateUnion {
    /// Canonical set bytes. Subject/stratum order, framing and integer widths are
    /// fixed across native and WASM; no score, relevance order or native usize is written.
    /// # Errors
    /// Returns the original native allocation refusal if export cannot be admitted.
    pub fn canonical_bytes(
        &self,
    ) -> Result<SharedWorkspace<AdmittedVec<u8>>, purrdf_sparql_eval::EvalError> {
        let workspace = self.candidates.workspace();
        let mut writer = Writer::admitted(workspace);
        // Version 3 retains separate row/rank progress and carries ceiling precision.
        writer.u16(3);
        writer.string("candidate-union");
        writer.bytes(self.plan_id.as_bytes());
        writer.u8(u8::from(self.completed_prefix));
        writer.u64(self.candidates.len() as u64);
        for (subject, candidate) in &self.candidates {
            writer.string(subject.as_str());
            writer.u64(candidate.ranks.len() as u64);
            for (stratum, ranks) in &candidate.ranks {
                writer.string(stratum.as_str());
                writer.u64(ranks.len() as u64);
                for rank in ranks {
                    writer.u64(*rank);
                }
            }
            match &candidate.block {
                None => writer.u8(0),
                Some((stratum, block)) => {
                    writer.u8(1);
                    writer.string(stratum.as_str());
                    writer.string(block.as_str());
                }
            }
        }
        writer.u64(self.producers.len() as u64);
        for (stratum, producer) in &self.producers {
            writer.string(stratum.as_str());
            writer.u32(producer.requested_depth);
            writer.u64(producer.rows_pulled);
            writer.u64(producer.last_rank);
            writer.option_u64(producer.rows_materialised);
            writer.option_u64(producer.rows_emitted);
            writer.u8(u8::from(producer.receipt_verified));
            writer.u8(u8::from(producer.completed_prefix));
            write_status(&mut writer, producer.status.as_ref());
            write_contract(&mut writer, producer.contract.as_ref());
            write_attestation(&mut writer, stratum, producer.announced.as_ref(), workspace)?;
            write_attestation(&mut writer, stratum, producer.settled.as_ref(), workspace)?;
            writer.ensure_live()?;
            let read_error = producer
                .read_error
                .as_ref()
                .map(|error| workspace.authored_text(error))
                .transpose()?;
            writer.option_string(read_error.as_deref());
            writer.ensure_live()?;
            let settlement_error = producer
                .settlement_error
                .as_ref()
                .map(|error| workspace.authored_text(error))
                .transpose()?;
            writer.option_string(settlement_error.as_deref());
        }
        crate::plan::write_bindings(&mut writer, &self.producer_bindings);
        crate::plan::write_decisions(&mut writer, &self.producer_decisions);
        crate::plan::write_unserved_terms(&mut writer, &self.unserved_terms);
        writer.into_admitted()
    }
}

fn write_status(
    writer: &mut Writer<impl crate::canonical::CanonicalSink>,
    status: Option<&ProducerStatus>,
) {
    match status {
        None => writer.u8(0),
        Some(ProducerStatus::Exhausted { rows_emitted }) => {
            writer.u8(1);
            writer.u64(*rows_emitted);
        }
        Some(ProducerStatus::DepthReached { rank }) => {
            writer.u8(2);
            writer.u64(*rank);
        }
        Some(ProducerStatus::RowBoundReached { rank }) => {
            writer.u8(3);
            writer.u64(*rank);
        }
        Some(ProducerStatus::SuppliedQueryEnded { rank }) => {
            writer.u8(4);
            writer.u64(*rank);
        }
        Some(ProducerStatus::CeilingReached { bound }) => {
            writer.u8(5);
            writer.u128(bound.into_raw());
            writer.u32(bound.precision().digits());
        }
        Some(ProducerStatus::ExecutionFailed { reason }) => {
            writer.u8(6);
            writer.string(reason);
        }
        Some(ProducerStatus::TermsRejected) => writer.u8(7),
    }
}

fn write_contract(
    writer: &mut Writer<impl crate::canonical::CanonicalSink>,
    contract: Option<&StreamContract>,
) {
    let Some(contract) = contract else {
        writer.u8(0);
        return;
    };
    writer.u8(1);
    writer.string(contract.rank_law.as_str());
    writer.u8(u8::from(contract.duplicates == DuplicatePolicy::Unique));
    match &contract.fidelity.completeness {
        Completeness::Complete => writer.u8(0),
        Completeness::Lossy { evidence } => {
            writer.u8(1);
            writer.string(evidence);
        }
    }
    match &contract.fidelity.order {
        OrderFidelity::Faithful => writer.u8(0),
        OrderFidelity::Perturbed { evidence } => {
            writer.u8(1);
            writer.string(evidence);
        }
    }
    match contract.domains.tags() {
        None => writer.u8(0),
        Some(tags) => {
            writer.u8(1);
            writer.u64(tags.len() as u64);
            for tag in tags {
                writer.string(tag.as_str());
            }
        }
    }
    writer.string(contract.exclusion.as_str());
}

fn write_attestation(
    writer: &mut Writer<crate::canonical::AdmittedBytes>,
    stratum: &Iri,
    attestation: Option<&PfAttestation>,
    workspace: &purrdf_sparql_eval::WorkspaceCapability,
) -> Result<(), purrdf_sparql_eval::EvalError> {
    match attestation {
        None => writer.u8(0),
        Some(attestation) => {
            writer.u8(1);
            writer.ensure_live()?;
            let mut evidence = Writer::admitted(workspace);
            crate::fusion_stream::write_evidence(
                &mut evidence,
                || core::iter::once((stratum, attestation)),
                core::iter::empty,
            );
            writer.bytes(&evidence.into_admitted()?);
        }
    }
    Ok(())
}

#[cfg(test)]
mod admission_failure_tests {
    use super::*;

    #[test]
    fn result_admission_refusal_settles_all_reads_and_moves_original_evidence() {
        let workspace = purrdf_sparql_eval::WorkspaceCapability::resident();
        let stratum = Iri::parse("http://example.org/producer").unwrap();
        let plan_id = PlanId::from_canonical(b"settlement-before-result-admission");
        let contract = StreamContract::new(
            purrdf_sparql_eval::RankLaw::Ordinal,
            DuplicatePolicy::Unique,
            crate::RankFidelity::EXACT,
            crate::CandidateDomains::Unrestricted,
            crate::ExclusionBasis::Unavailable,
        );
        let streams = AdmittedVec::from_iter_admitted(
            (0..2).map(|_| ReadStratum {
                stratum: stratum.clone(),
                plan_id,
                requested_depth: 3,
                contract: contract.clone(),
                attestation: PfAttestation::UNDECLARED,
                stream: crate::RankedStreamImpl::new(Vec::new(), crate::StreamEnding::Exhausted),
                admission_failure_settlement: None,
            }),
            &workspace,
        )
        .unwrap();
        let original_buffer = streams.as_slice().as_ptr();
        let execution = CandidateExecutionResult {
            streams,
            statuses: AdmittedOrderedMap::new(&workspace),
            plan_id,
            depths: CandidateDepths::new([(stratum, 3)]).unwrap(),
            contracts: SharedWorkspace::new_admitted(
                AdmittedOrderedMap::new(&workspace),
                &workspace,
            )
            .unwrap(),
            unserved_terms: SharedWorkspace::new_admitted(AdmittedVec::new(&workspace), &workspace)
                .unwrap(),
            producer_bindings: std::sync::Arc::new(Vec::new()).into(),
            producer_decisions: std::sync::Arc::new(Vec::new()).into(),
        };
        let failure = crate::block_on(settle_admission_failure(
            execution,
            purrdf_sparql_eval::EvalError::AllocationFailed {
                construct: "original result control",
            },
        ));
        assert!(matches!(
            failure.cause,
            UnionError::Protocol(ProtocolError::NativeFailure(
                purrdf_sparql_eval::EvalError::AllocationFailed {
                    construct: "original result control"
                }
            ))
        ));
        assert!(failure.partial.is_none());
        let fields = purrdf_testkit::error_capture::Capture::read(&failure);
        assert_eq!(fields.code, "retrieval-union-failure");
        assert!(matches!(
            fields.field("partial"),
            purrdf_hash::error::ErrorValue::Absent
        ));
        let execution_fields = fields.child("execution");
        let read_fields = execution_fields.records("streams");
        assert_eq!(read_fields.len(), 2);
        for read in read_fields {
            assert!(matches!(
                read.field("requested_depth"),
                purrdf_hash::error::ErrorValue::Unsigned(3)
            ));
            let settlement = read.child("admission_failure_settlement");
            assert!(matches!(
                settlement.field("kind"),
                purrdf_hash::error::ErrorValue::Code("settled")
            ));
            assert!(matches!(
                settlement.field("attestation"),
                purrdf_hash::error::ErrorValue::Absent
            ));
        }
        let mut retained = failure.execution;
        assert_eq!(retained.plan_id, plan_id);
        assert_eq!(retained.streams.as_slice().as_ptr(), original_buffer);
        assert_eq!(retained.streams.len(), 2);
        for read in &retained.streams {
            assert!(matches!(read.admission_failure_settlement, Some(Ok(None))));
            assert_eq!(read.stream.rows_emitted(), 0);
        }
        // A later setup refusal can leave the producer map without entries.
        // Exercise that separate settlement route with the same original arena.
        for read in retained.streams.iter_mut() {
            read.admission_failure_settlement = None;
        }
        let mut partial = CandidateUnion {
            plan_id,
            candidates: SharedWorkspace::new_admitted(
                AdmittedOrderedMap::new(&workspace),
                &workspace,
            )
            .unwrap(),
            producers: SharedWorkspace::new_admitted(
                AdmittedOrderedMap::new(&workspace),
                &workspace,
            )
            .unwrap(),
            completed_prefix: false,
            unserved_terms: retained.unserved_terms.clone(),
            producer_bindings: retained.producer_bindings.clone(),
            producer_decisions: retained.producer_decisions.clone(),
        };
        let mut primary = Some(UnionError::Protocol(ProtocolError::NativeFailure(
            purrdf_sparql_eval::EvalError::WorkspaceStopped,
        )));
        crate::block_on(settle_reads(
            &mut retained.streams,
            &mut partial,
            &mut primary,
        ));
        assert!(matches!(
            primary,
            Some(UnionError::Protocol(ProtocolError::NativeFailure(
                purrdf_sparql_eval::EvalError::WorkspaceStopped
            )))
        ));
        assert_eq!(retained.streams.as_slice().as_ptr(), original_buffer);
        assert!(
            retained
                .streams
                .iter()
                .all(|read| matches!(read.admission_failure_settlement, Some(Ok(None))))
        );
        let failure = UnionFailure {
            cause: primary.unwrap(),
            partial: Some(partial),
            execution: retained,
        };
        drop(workspace);
        let fields = purrdf_testkit::error_capture::Capture::read(&failure);
        let partial = fields.child("partial");
        assert!(matches!(
            partial.field("completed_prefix"),
            purrdf_hash::error::ErrorValue::Boolean(false)
        ));
        assert!(partial.records("candidates").is_empty());
        assert!(partial.records("producers").is_empty());
        assert_eq!(fields.child("execution").records("streams").len(), 2);
    }
}
