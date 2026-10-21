// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Snapshot-bound execution with typed certified prefixes and retained work.

use super::{
    BudgetReport, Derivation, DerivationMode, EvalError, EvalOptions, Evaluation, Executable,
    FixpointState, GuardEvaluator, JoinStrategy, Limits, RelationStore, StopSignal, StratumRun,
    TermDictionary, WorkReport, check_budget, is_stopped, stratum,
};
use crate::cache::{canonical_rule_hash, contract_hash_with};
use crate::store::{Arity, StoreShape};
use purrdf_hash::{Domain, hex::Digest32};
use std::fmt;

const PARTIAL_MODEL_DOMAIN: Domain = Domain::new(b"purrdf-datalog-partial-model-v1");

/// Canonical identities supplied by the owner of the original execution inputs.
///
/// These identify the source snapshot, guard semantics and evidence regime, rather
/// than runtime handles. There are no fabricated defaults. The session consumes its
/// store, borrows its immutable program and guard evaluator, and checks the original
/// dictionary's pinned-source validity at every public read and resume boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionContext {
    /// The caller's canonical source snapshot identity, including its coordinates.
    pub snapshot: Digest32,
    /// The caller's canonical identity of the borrowed guard evaluator's semantics.
    pub guards: Digest32,
    /// The caller's canonical identity of the selected evidence regime.
    pub evidence: Digest32,
}

/// The original source context and the library-computed authored program identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContinuationBinding {
    /// Canonical clause content; no runtime term or row identity is hashed.
    pub program: Digest32,
    /// The original caller-declared semantic context.
    pub context: SessionContext,
}

/// What is known about the extension produced by one stratum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StratumStatus {
    /// The stratum reached its fact fixpoint.
    Complete,
    /// All published facts are sound; further facts may still be derived.
    SoundLowerBound,
    /// Further execution is withheld until preceding strata complete. Seeded facts
    /// and sound heads already emitted by a lower-stratum conjunction remain facts.
    Withheld,
}

/// A completed execution or an original, typed refusal with its state retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResumeProgress<E = core::convert::Infallible> {
    /// A complete model can now be taken with [`EvaluationSession::take_complete`].
    Complete,
    /// Published facts remain available only through [`PartialModel`].
    Paused(EvalError<E>),
}

/// A continuation cannot be rebound, change proof recording, or hide a hard failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionError<E = core::convert::Infallible> {
    /// An attempted resume changed an original canonical input identity.
    BindingMismatch {
        /// The changed coordinate: program, snapshot, guards or evidence.
        component: &'static str,
        /// The identity retained at construction.
        expected: Digest32,
        /// The replacement supplied by the attempted resume.
        actual: Digest32,
    },
    /// Proof recording cannot be enabled after unrecorded derivations were published.
    EvidenceModeChanged {
        /// The original mode, retained by the session.
        expected: DerivationMode,
        /// The replacement requested by the attempted resume.
        actual: DerivationMode,
    },
    /// A complete model was requested before successful completion.
    Incomplete,
    /// The completed model has already been taken.
    Finished,
    /// An original source, guard or semantic failure; this session cannot resume.
    Failed(EvalError<E>),
}

impl<E: purrdf_hash::error::StructuredError> purrdf_hash::error::StructuredError
    for SessionError<E>
{
    purrdf_hash::error_visit! {
        BindingMismatch { component, expected, actual } => "datalog-continuation-binding" {
            "component" => Text(component), "expected" => Bytes(expected.as_bytes()),
            "actual" => Bytes(actual.as_bytes()),
        },
        EvidenceModeChanged { expected, actual } => "datalog-continuation-evidence-mode" {
            "expected_recording" => Boolean(*expected == DerivationMode::Record),
            "actual_recording" => Boolean(*actual == DerivationMode::Record),
        },
        Incomplete => "datalog-continuation-incomplete" {},
        Finished => "datalog-continuation-finished" {},
        Failed(cause) => "datalog-continuation-failed" { "cause" => Cause(cause) },
    }
}

impl<E: fmt::Display> fmt::Display for SessionError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BindingMismatch {
                component,
                expected,
                actual,
            } => write!(
                formatter,
                "continuation {component} changed: expected {expected}, received {actual}"
            ),
            Self::EvidenceModeChanged { .. } => {
                formatter.write_str("continuation derivation recording cannot change")
            }
            Self::Incomplete => formatter.write_str("continuation has not completed"),
            Self::Finished => {
                formatter.write_str("continuation's complete model was already taken")
            }
            Self::Failed(error) => error.fmt(formatter),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for SessionError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Failed(error) => Some(error),
            _ => None,
        }
    }
}

/// An in-process continuation owns its store and every entered execution phase.
///
/// Only cumulative governors and per-call scheduling may change. The original
/// program and guard evaluator remain borrowed, and no mutable store access is
/// exposed. Guard implementations must preserve their declared pure semantics.
/// Source and guard failures poison execution; they are never treated as a page.
pub struct EvaluationSession<'r, D: TermDictionary, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    exe: &'r Executable<N>,
    guards: &'r dyn GuardEvaluator<D, N>,
    binding: ContinuationBinding,
    options: EvalOptions,
    state: Option<FixpointState<D, N>>,
    stratum: usize,
    frame: Option<stratum::Frame<'r, D::Id, N>>,
    complete: bool,
    failure: Option<EvalError<D::Error>>,
}

impl<D: TermDictionary, const N: usize> fmt::Debug for EvaluationSession<'_, D, N>
where
    Arity<N>: StoreShape<N>,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EvaluationSession")
            .field("binding", &self.binding)
            .field("stratum", &self.stratum)
            .field("complete", &self.complete)
            .finish_non_exhaustive()
    }
}

impl<'r, D: TermDictionary, const N: usize> EvaluationSession<'r, D, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Retain the original input owners without running or discarding seeded facts.
    ///
    /// # Errors
    /// Preserves invalid-source and unsupported immutable-base scheduling failures.
    pub fn new(
        exe: &'r Executable<N>,
        edb: RelationStore<D, N>,
        guards: &'r dyn GuardEvaluator<D, N>,
        context: SessionContext,
        options: EvalOptions,
    ) -> Result<Self, EvalError<D::Error>> {
        if let Some(rule) = exe
            .rules()
            .iter()
            .position(crate::clause::DlClause::reads_base)
        {
            return Err(EvalError::BaseReadRequiresSchedule { rule });
        }
        edb.interner().check_read().map_err(EvalError::Source)?;
        Ok(Self {
            exe,
            guards,
            binding: ContinuationBinding {
                program: Digest32::new(canonical_rule_hash(exe.rules())),
                context,
            },
            options,
            state: Some(FixpointState::seeded(edb, options)),
            stratum: 0,
            frame: None,
            complete: false,
            failure: None,
        })
    }

    /// The exact original binding a subsequent resume must present unchanged.
    pub fn binding(&self) -> ContinuationBinding {
        self.binding
    }

    /// Continue original frames under cumulative ceilings, without replaying work.
    /// A fresh stop signal may replace a previously latched signal.
    ///
    /// # Errors
    /// Refuses changed bindings or recording mode before work. Original hard
    /// failures are preserved and poison further execution. Resource and stop
    /// refusals return [`ResumeProgress::Paused`] with the continuation intact.
    pub fn resume(
        &mut self,
        binding: ContinuationBinding,
        options: EvalOptions,
        stop: Option<&dyn StopSignal>,
    ) -> Result<ResumeProgress<D::Error>, SessionError<D::Error>> {
        for (component, expected, actual) in [
            ("program", self.binding.program, binding.program),
            (
                "snapshot",
                self.binding.context.snapshot,
                binding.context.snapshot,
            ),
            (
                "guards",
                self.binding.context.guards,
                binding.context.guards,
            ),
            (
                "evidence",
                self.binding.context.evidence,
                binding.context.evidence,
            ),
        ] {
            if expected != actual {
                return Err(SessionError::BindingMismatch {
                    component,
                    expected,
                    actual,
                });
            }
        }
        if let Some(error) = &self.failure {
            return Err(SessionError::Failed(error.clone()));
        }
        let state = self.state.as_mut().ok_or(SessionError::Finished)?;
        if state.derivation_mode != options.derivation_mode() {
            return Err(SessionError::EvidenceModeChanged {
                expected: state.derivation_mode,
                actual: options.derivation_mode(),
            });
        }
        self.options = options;
        state.limits = Limits::of(&options, state.input_terms);
        state.stated = BudgetReport::stated_limits(&options);
        state.execution = options.round_execution();
        self.complete = false;
        let result = (|| {
            check_budget(state)?;
            if is_stopped(stop) {
                return Err(EvalError::Stopped {
                    report: state.report(),
                });
            }
            while self.stratum < self.exe.stratum_count() {
                if self.exe.stratum_is_empty(self.stratum) {
                    self.stratum += 1;
                    continue;
                }
                let frame = self.frame.get_or_insert_with(|| {
                    stratum::Frame::new(self.exe, self.exe.runtimes(), self.stratum, state)
                });
                frame.advance(
                    state,
                    StratumRun {
                        stop,
                        guards: self.guards,
                        execution: options.round_execution(),
                        strategy: JoinStrategy::Planned,
                    },
                    &mut None,
                )?;
                debug_assert!(frame.is_complete());
                self.frame = None;
                self.stratum += 1;
            }
            state.rel.interner().check_read().map_err(EvalError::Source)
        })();
        match result {
            Ok(()) => {
                self.complete = true;
                Ok(ResumeProgress::Complete)
            }
            Err(
                error @ (EvalError::BudgetExhausted { .. }
                | EvalError::TermLimitExceeded { .. }
                | EvalError::Stopped { .. }
                | EvalError::Storage(_)),
            ) => Ok(ResumeProgress::Paused(error)),
            Err(error) => {
                self.failure = Some(error.clone());
                Err(SessionError::Failed(error))
            }
        }
    }

    /// Borrow certified published facts without converting them into a complete model.
    ///
    /// # Errors
    /// Refuses a taken model or an original hard failure, including a dead source.
    pub fn partial_model(&self) -> Result<PartialModel<'_, D, N>, SessionError<D::Error>> {
        if let Some(error) = &self.failure {
            return Err(SessionError::Failed(error.clone()));
        }
        let state = self.state.as_ref().ok_or(SessionError::Finished)?;
        state
            .rel
            .interner()
            .check_read()
            .map_err(|error| SessionError::Failed(EvalError::Source(error)))?;
        Ok(PartialModel {
            state,
            exe: self.exe,
            options: self.options,
            binding: self.binding,
            stratum: self.stratum,
        })
    }

    /// Move out a complete model once; an incomplete request leaves all owners intact.
    ///
    /// # Errors
    /// Refuses incompletion, source invalidation, hard failure or a second take.
    pub fn take_complete(&mut self) -> Result<Evaluation<D, N>, SessionError<D::Error>> {
        self.partial_model()?;
        if !self.complete {
            return Err(SessionError::Incomplete);
        }
        Ok(self
            .state
            .take()
            .expect("the checked complete state is retained")
            .finish())
    }
}

/// A borrowed certified prefix, deliberately distinct from [`Evaluation`].
///
/// Every returned derived fact was published from proved premises. Later strata
/// have never run, so negation cannot observe an incomplete lower extension.
pub struct PartialModel<'a, D: TermDictionary, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    state: &'a FixpointState<D, N>,
    exe: &'a Executable<N>,
    options: EvalOptions,
    binding: ContinuationBinding,
    stratum: usize,
}

impl<D: TermDictionary, const N: usize> fmt::Debug for PartialModel<'_, D, N>
where
    Arity<N>: StoreShape<N>,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PartialModel")
            .field("binding", &self.binding)
            .field("stratum", &self.stratum)
            .finish_non_exhaustive()
    }
}

impl<D: TermDictionary, const N: usize> PartialModel<'_, D, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Seeded and proved published rows; this is not a complete least model.
    pub fn facts(&self) -> &RelationStore<D, N> {
        &self.state.rel
    }

    /// Recorded canonical winners in publication order, or explicitly unrecorded.
    pub fn derivations(&self) -> Option<&[Derivation<N>]> {
        (self.state.derivation_mode == DerivationMode::Record)
            .then_some(self.state.derivations.as_slice())
    }

    /// Actual successful consumption; a refusal attempt is not repeated work.
    pub fn budget(&self) -> BudgetReport {
        self.state.report()
    }

    /// Number of program strata, including those whose conclusions are withheld.
    pub fn stratum_count(&self) -> usize {
        self.exe.stratum_count()
    }

    /// Certified status, or `None` for an index outside this program.
    pub fn stratum_status(&self, index: usize) -> Option<StratumStatus> {
        (index < self.exe.stratum_count()).then_some(match index.cmp(&self.stratum) {
            std::cmp::Ordering::Less => StratumStatus::Complete,
            std::cmp::Ordering::Equal => StratumStatus::SoundLowerBound,
            std::cmp::Ordering::Greater => StratumStatus::Withheld,
        })
    }

    /// Successful physical work and published semantic counts.
    pub fn work(&self) -> WorkReport {
        WorkReport {
            input_facts: self.state.input_facts,
            committed_facts: self.state.committed_facts,
            join_steps: self.state.join_steps,
        }
    }

    /// Content-bound partial identity, domain-separated from every complete contract.
    /// It includes canonical bindings, effective governors and actual consumption.
    pub fn identity(&self) -> Digest32 {
        let mut hasher = purrdf_hash::blake3::Hasher::new();
        hasher.update(PARTIAL_MODEL_DOMAIN.as_bytes());
        hasher.update(contract_hash_with(self.exe.rules(), &self.options).digest());
        for digest in [
            self.binding.program,
            self.binding.context.snapshot,
            self.binding.context.guards,
            self.binding.context.evidence,
        ] {
            hasher.update(digest.as_bytes());
        }
        let report = self.budget();
        for value in [
            self.stratum as u64,
            self.state.input_facts,
            self.state.committed_facts,
            report.join_steps(),
            report.observed_stored_facts() as u64,
            report.term_arena_bytes() as u64,
            report.term_generating_rounds(),
            report.generated_terms(),
        ] {
            hasher.update(&value.to_le_bytes());
        }
        hasher.update(&[u8::from(
            self.state.derivation_mode == DerivationMode::Record,
        )]);
        Digest32::new(*hasher.finalize().as_bytes())
    }
}
