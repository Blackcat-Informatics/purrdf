// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Resumable native RDFS materialization over one frozen default graph.

use std::fmt;
use std::sync::{Arc, LazyLock};

use purrdf_core::{ContentIdentified, RdfDataset, TermValue};
use purrdf_datalog::chase::{
    AnalysisLimits, ChaseAdvance, ChaseError, ChaseFiring, ChaseProofError, ChaseSession,
    certify_with,
};
use purrdf_datalog::clause::DlClause;
use purrdf_datalog::seminaive::{BudgetReport, EvalOptions};
use purrdf_datalog::stop::StopSignal;
use purrdf_hash::hex::Digest32;
use purrdf_lex::allocation::{Memory, Resident};

use super::{
    ProjectedDerivation, Terms, emit_default_run, project_chase_run, project_derivation, seed,
};
use crate::calculus::{ChaseRule, program_with_attribution};
use crate::report::{ReasoningReport, RunStats};
use crate::{EntailError, Regime, RuleId};

// Only the immutable authored table is shared. Each session checks admission
// and owns its source, evaluation and work accounting independently.
static RDFS: LazyLock<(Vec<DlClause>, Vec<ChaseRule>)> =
    LazyLock::new(|| program_with_attribution(Regime::Rdfs));

/// Why a default-graph RDFS session could not start, complete, or check proof.
#[derive(Debug)]
#[non_exhaustive]
pub enum RdfsSessionError {
    /// Named graphs, including empty declarations, are outside this surface.
    NamedGraphs,
    /// The requested frozen snapshot differs from the source's identity.
    SnapshotMismatch {
        /// The caller's requested identity.
        expected: Digest32,
        /// The source's actual identity.
        actual: Digest32,
    },
    /// An operational failure with its original cause.
    Engine(EntailError),
    /// The recorded proof could not be independently checked.
    Proof(ChaseProofError),
    /// The restricted chase returned an outcome this RDFS surface cannot use.
    UnsupportedProgress,
}

purrdf_lex::variant_from!(RdfsSessionError { Engine(EntailError), Proof(ChaseProofError) });

impl fmt::Display for RdfsSessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NamedGraphs => {
                f.write_str("resumable RDFS requires a default-graph-only dataset")
            }
            Self::SnapshotMismatch { expected, actual } => {
                write!(
                    f,
                    "RDFS snapshot mismatch: expected {expected}, got {actual}"
                )
            }
            Self::Engine(error) => error.fmt(f),
            Self::Proof(error) => error.fmt(f),
            Self::UnsupportedProgress => f.write_str("unsupported native RDFS chase progress"),
        }
    }
}

impl std::error::Error for RdfsSessionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Engine(error) => Some(error),
            Self::Proof(error) => Some(error),
            _ => None,
        }
    }
}

/// In-process RDFS closure bound to its original immutable native snapshot.
pub struct RdfsSession {
    source: Arc<RdfDataset>,
    identity: Digest32,
    terms: Terms,
    chase: ChaseSession<'static>,
}

purrdf_hash::debug_non_exhaustive!(RdfsSession { identity, chase });

/// A complete materialization or a resumable refusal with recorded evidence.
#[derive(Debug)]
#[non_exhaustive]
pub enum RdfsProgress {
    /// The same closure and complete reasoning report as uninterrupted RDFS.
    Complete {
        /// Source quads and admitted RDF conclusions.
        closure: Arc<RdfDataset>,
        /// Complete reasoning evidence under the final policy.
        report: Box<ReasoningReport>,
    },
    /// A caller stop or logical ceiling retained the original execution.
    Paused(Box<RdfsPause>),
}

/// A committed, RDF-admissible conclusion and its original firing evidence.
#[derive(Debug)]
#[non_exhaustive]
pub struct RdfsConclusion<'a> {
    /// Subject of this default-graph conclusion.
    pub subject: &'a TermValue,
    /// Predicate of this default-graph conclusion.
    pub predicate: &'a TermValue,
    /// Object of this default-graph conclusion.
    pub object: &'a TermValue,
    /// Authored RDFS rule identifier.
    pub rule: RuleId,
    /// Authored clause index in the native RDFS calculus.
    pub clause: usize,
    /// Ground premises and authored head coordinate.
    pub firing: &'a ChaseFiring,
}

/// A partial, proof-carrying result. This is not a closure or a reasoning report.
#[derive(Debug)]
pub struct RdfsPause {
    reason: ChaseError,
    session: RdfsSession,
}

impl RdfsSession {
    /// Prepare a session without running a chase round.
    ///
    /// # Errors
    /// Refuses a different source identity, a named graph declaration, or a
    /// source, storage or program admission failure with the original cause.
    pub fn start(
        source: Arc<RdfDataset>,
        expected_identity: Digest32,
        options: &EvalOptions,
    ) -> Result<Self, RdfsSessionError> {
        let identity = source.content_identity().map_err(EntailError::Storage)?;
        if identity != expected_identity {
            return Err(RdfsSessionError::SnapshotMismatch {
                expected: expected_identity,
                actual: identity,
            });
        }
        if source.named_graphs().next().is_some() {
            return Err(RdfsSessionError::NamedGraphs);
        }
        let program = &RDFS.0;
        let (edb, terms) = seed(source.as_ref(), Regime::Rdfs, program, None)?;
        let limits = AnalysisLimits::default();
        let prepared = certify_with(program, Resident, limits, None)
            .and_then(|certificate| certificate.prepare(program, limits, None))
            .map_err(ChaseError::from)
            .map_err(super::chase_error)?;
        let chase = prepared.start_recorded_session(edb, options);
        prepared
            .close()
            .map_err(ChaseError::from)
            .map_err(super::chase_error)?;
        Ok(Self {
            source,
            identity,
            terms,
            chase: chase.map_err(super::chase_error)?,
        })
    }

    /// Advance with caller-supplied cumulative ceilings and a fresh stop signal.
    ///
    /// # Errors
    /// Returns hard execution, source or construction failures without a partial result.
    pub fn advance(
        self,
        options: &EvalOptions,
        stop: Option<&dyn StopSignal>,
    ) -> Result<RdfsProgress, RdfsSessionError> {
        let Self {
            source,
            identity,
            terms,
            chase,
        } = self;
        match chase
            .advance_with(options, stop)
            .map_err(super::chase_error)?
        {
            ChaseAdvance::Paused {
                reason,
                session: chase,
            } => Memory::new(&mut Resident)
                .boxed(RdfsPause {
                    reason,
                    session: Self {
                        source,
                        identity,
                        terms,
                        chase,
                    },
                })
                .map(RdfsProgress::Paused)
                .map_err(EntailError::Storage)
                .map_err(Into::into),
            ChaseAdvance::Complete(outcome) => {
                let run = project_chase_run(&outcome, &RDFS.1, &terms);
                let mut stats = RunStats::none(*options);
                let builder =
                    emit_default_run(source.as_ref(), Regime::Rdfs, run, &mut stats, |_| {})?;
                let closure = builder.freeze().map_err(EntailError::from)?;
                let report = ReasoningReport::of_run(source.as_ref(), Regime::Rdfs, &stats)?;
                let report = Memory::new(&mut Resident)
                    .boxed(report)
                    .map_err(EntailError::Storage)?;
                Ok(RdfsProgress::Complete { closure, report })
            }
            _ => Err(RdfsSessionError::UnsupportedProgress),
        }
    }
}

impl RdfsPause {
    /// The original stop or logical budget refusal.
    pub fn reason(&self) -> &ChaseError {
        &self.reason
    }

    /// The identity of the immutable source retained for this continuation.
    pub fn source_identity(&self) -> Digest32 {
        self.session.identity
    }

    /// Work from the original seed through this pause.
    pub fn budget(&self) -> BudgetReport {
        self.session.chase.budget()
    }

    /// Committed RDF conclusions only, with their original firing evidence.
    pub fn conclusions(&self) -> impl Iterator<Item = RdfsConclusion<'_>> {
        let (derived, proofs) = self
            .session
            .chase
            .recorded_prefix()
            .expect("resumable RDFS always records firing evidence");
        derived
            .iter()
            .zip(proofs)
            .filter_map(|(derivation, firing)| {
                let clause = derivation.clause();
                let ProjectedDerivation::Conclusion(conclusion) = project_derivation(
                    derivation.fact(),
                    RDFS.1[clause],
                    &self.session.terms,
                    |surface| self.session.chase.is_invented_term(surface),
                ) else {
                    return None;
                };
                Some(RdfsConclusion {
                    subject: conclusion.subject,
                    predicate: conclusion.predicate,
                    object: conclusion.object,
                    rule: conclusion.rule.rule_id(false),
                    clause,
                    firing,
                })
            })
    }

    /// Independently verify the recorded prefix from the original frozen seed.
    ///
    /// # Errors
    /// Returns native source or proof verification failure.
    pub fn verify(&self) -> Result<(), RdfsSessionError> {
        let (edb, _) = seed(self.session.source.as_ref(), Regime::Rdfs, &RDFS.0, None)?;
        Ok(self.session.chase.verify_recorded_prefix(edb)?)
    }

    /// Take the original execution state and resume under a caller-selected policy.
    pub fn into_session(self) -> RdfsSession {
        self.session
    }
}
