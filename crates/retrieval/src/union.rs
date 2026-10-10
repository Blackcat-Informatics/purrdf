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
use purrdf_sparql_eval::{Completeness, DuplicatePolicy, OrderFidelity, PfAttestation};
use std::collections::{BTreeMap, BTreeSet};

/// A subject in a set, carrying every original rank at which each producer named it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnionCandidate {
    /// Original ranks, in producer emission order. Allowed duplicates retain all
    /// their ranks; they do not produce duplicate subjects in the candidate set.
    pub ranks: BTreeMap<Iri, Vec<u64>>,
    /// The first declared block placement, if a producer provided one.
    pub block: Option<(Iri, crate::DomainTag)>,
}

/// One producer's original work bound, observed read and evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnionProducer {
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

/// A candidate set with no relevance score, top-k truncation or ranking.
///
/// BTreeMap order is its canonical set representation, not a relevance order.
/// Every producer's independent prefix and original evidence remains inspectable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateUnion {
    /// The explicit candidate-plan identity.
    pub plan_id: PlanId,
    /// Distinct subjects and all their original producer ranks.
    pub candidates: BTreeMap<Term, UnionCandidate>,
    /// Every selected producer, including one that failed before opening.
    pub producers: BTreeMap<Iri, UnionProducer>,
    /// True only after every selected producer certifies its own requested prefix.
    pub completed_prefix: bool,
    /// Every request term/placement that no producer could serve.
    pub unserved_terms: Vec<UnservedTerm>,
    /// Original term-to-producer binding records.
    pub producer_bindings: Vec<ProducerBinding>,
    /// Original selection/rejection decisions.
    pub producer_decisions: Vec<ProducerDecision>,
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
        /// The compiled caller depth.
        depth: u32,
    },
    /// The shared native ranked-stream protocol refused.
    Protocol(ProtocolError),
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
                depth,
            } => write!(
                f,
                "candidate stratum {stratum} emitted rank {rank} past requested depth {depth}"
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
pub struct UnionFailure {
    /// The first whole-stage refusal; cleanup never replaces it.
    pub cause: UnionError,
    /// The explicitly incomplete observed set, with all settlement evidence retained.
    pub partial: CandidateUnion,
}
impl fmt::Display for UnionFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.cause.fmt(f)
    }
}
impl std::error::Error for UnionFailure {
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
pub async fn union(
    mut execution: CandidateExecutionResult<'_>,
    depths: &CandidateDepths,
) -> Result<CandidateUnion, Box<UnionFailure>> {
    let mut result = CandidateUnion {
        plan_id: execution.plan_id,
        candidates: BTreeMap::new(),
        producers: BTreeMap::new(),
        completed_prefix: false,
        unserved_terms: std::mem::take(&mut execution.unserved_terms),
        producer_bindings: std::mem::take(&mut execution.producer_bindings),
        producer_decisions: std::mem::take(&mut execution.producer_decisions),
    };
    for (stratum, depth) in execution.depths.sorted() {
        result.producers.insert(
            stratum.clone(),
            UnionProducer::new(
                depth,
                execution.contracts.get(stratum).cloned(),
                execution.statuses.get(stratum).cloned(),
            ),
        );
    }
    for read in &execution.streams {
        result
            .producers
            .entry(read.stratum.clone())
            .or_insert_with(|| {
                UnionProducer::new(read.requested_depth, Some(read.contract.clone()), None)
            })
            .announced = Some(read.attestation.clone());
    }
    let mut primary = check_execution(&execution, depths)
        .err()
        .map(|error| *error);
    if primary.is_none() {
        'reads: for read in &mut execution.streams {
            loop {
                let pulled = read.stream.next().await;
                match pulled {
                    Ok(Some((rank, subject, block))) => {
                        let producer = result
                            .producers
                            .get_mut(&read.stratum)
                            .expect("the actual stream's producer was recorded");
                        let expected = producer.rows_pulled + 1;
                        producer.rows_pulled += 1;
                        let checked = validate_rank(expected, rank)
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
                        if rank > u64::from(producer.requested_depth) {
                            primary = Some(UnionError::RowsPastDepth {
                                stratum: read.stratum.clone(),
                                rank,
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
                        let candidate =
                            result
                                .candidates
                                .entry(subject)
                                .or_insert_with(|| UnionCandidate {
                                    ranks: BTreeMap::new(),
                                    block: None,
                                });
                        let ranks = candidate.ranks.entry(read.stratum.clone()).or_default();
                        ranks.push(rank);
                        if candidate.block.is_none()
                            && let Some(tag) = block.tag()
                        {
                            candidate.block = Some((read.stratum.clone(), tag.clone()));
                        }
                    }
                    Ok(None) => {
                        let producer = result
                            .producers
                            .get_mut(&read.stratum)
                            .expect("the actual stream's producer was recorded");
                        match read.stream.receipt().await.and_then(|receipt| {
                            validate_receipt(&receipt, producer.rows_pulled)?;
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
                            .get_mut(&read.stratum)
                            .expect("the actual stream's producer was recorded");
                        producer.read_error = Some(error.clone());
                        if let ProtocolError::ReadFailed { reason, .. } = &error
                            && !read.stream.run_invalidated()
                        {
                            // A failure is a status, never a forged terminal receipt.
                            producer.status = Some(ProducerStatus::ExecutionFailed {
                                reason: reason.clone(),
                            });
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
    for producer in result.producers.values_mut() {
        producer.certify_prefix();
    }
    result.completed_prefix = primary.is_none()
        && result
            .producers
            .values()
            .all(|producer| producer.completed_prefix);
    match primary {
        Some(cause) => Err(Box::new(UnionFailure {
            cause,
            partial: result,
        })),
        None => Ok(result),
    }
}

fn check_execution(
    execution: &CandidateExecutionResult<'_>,
    depths: &CandidateDepths,
) -> Result<(), Box<UnionError>> {
    for stratum in execution.depths.0.keys().chain(depths.0.keys()) {
        if execution.depths.get(stratum) != depths.get(stratum) {
            return Err(Box::new(UnionError::DepthMismatch {
                stratum: stratum.clone(),
                executed: execution.depths.get(stratum),
                requested: depths.get(stratum),
            }));
        }
    }
    let mut seen = BTreeSet::new();
    for read in &execution.streams {
        if !seen.insert(read.stratum.clone()) {
            return Err(Box::new(UnionError::InconsistentExecution {
                reason: format!("candidate stratum {} appears twice", read.stratum),
            }));
        }
        if read.plan_id != execution.plan_id
            || execution.depths.get(&read.stratum) != Some(read.requested_depth)
            || execution.contracts.get(&read.stratum) != Some(&read.contract)
        {
            return Err(Box::new(UnionError::InconsistentExecution {
                reason: format!(
                    "candidate stratum {} differs from its compiled attribution",
                    read.stratum
                ),
            }));
        }
    }
    for stratum in execution.depths.0.keys() {
        if !execution.contracts.contains_key(stratum)
            || !seen.contains(stratum) && !execution.statuses.contains_key(stratum)
        {
            return Err(Box::new(UnionError::InconsistentExecution {
                reason: format!(
                    "candidate stratum {stratum} has no admitted contract or read/status"
                ),
            }));
        }
    }
    for stratum in execution.statuses.keys().chain(execution.contracts.keys()) {
        if execution.depths.get(stratum).is_none() {
            return Err(Box::new(UnionError::InconsistentExecution {
                reason: format!("candidate evidence names unselected stratum {stratum}"),
            }));
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
            .get_mut(&read.stratum)
            .expect("every actual stream was recorded before validation");
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
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut writer = Writer::new();
        writer.u16(1);
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
            writer.option_u64(producer.rows_materialised);
            writer.option_u64(producer.rows_emitted);
            writer.u8(u8::from(producer.receipt_verified));
            writer.u8(u8::from(producer.completed_prefix));
            write_status(&mut writer, producer.status.as_ref());
            write_contract(&mut writer, producer.contract.as_ref());
            write_attestation(&mut writer, stratum, producer.announced.as_ref());
            write_attestation(&mut writer, stratum, producer.settled.as_ref());
            writer.option_string(
                producer
                    .read_error
                    .as_ref()
                    .map(ToString::to_string)
                    .as_deref(),
            );
            writer.option_string(
                producer
                    .settlement_error
                    .as_ref()
                    .map(ToString::to_string)
                    .as_deref(),
            );
        }
        crate::plan::write_bindings(&mut writer, &self.producer_bindings);
        crate::plan::write_decisions(&mut writer, &self.producer_decisions);
        crate::plan::write_unserved_terms(&mut writer, &self.unserved_terms);
        writer.into_bytes()
    }
}

fn write_status(writer: &mut Writer, status: Option<&ProducerStatus>) {
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
            writer.i128(bound.into_raw());
        }
        Some(ProducerStatus::ExecutionFailed { reason }) => {
            writer.u8(6);
            writer.string(reason);
        }
        Some(ProducerStatus::TermsRejected) => writer.u8(7),
    }
}

fn write_contract(writer: &mut Writer, contract: Option<&StreamContract>) {
    let Some(contract) = contract else {
        writer.u8(0);
        return;
    };
    writer.u8(1);
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

fn write_attestation(writer: &mut Writer, stratum: &Iri, attestation: Option<&PfAttestation>) {
    match attestation {
        None => writer.u8(0),
        Some(attestation) => {
            writer.u8(1);
            writer.bytes(&crate::fusion_stream::evidence_canonical_bytes(
                &BTreeMap::from([(stratum.clone(), attestation.clone())]),
                &BTreeMap::new(),
            ));
        }
    }
}
