// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The public outcome of a **governed** query: a complete result, or an exhausted budget
//! carrying the partial answers the execution actually reached.
//!
//! # Two outcomes, one of which is not an error
//!
//! A governor trip is neither a complete result nor a failure. Conflating it with either
//! is the failure mode this module exists to make unrepresentable: reported as complete, a
//! truncated answer is silently wrong; reported as an error, the rows the budget already
//! paid for are thrown away and the caller is told the engine misbehaved. So
//! [`GovernedOutcome`] has exactly two shapes, and the second one carries both the
//! [`TrippedGovernor`] that stopped the execution and what the rows in hand bound.
//!
//! # Everything here is materialized and non-generic
//!
//! The evaluator's internal partial-result channel is generic over the dataset's id type
//! and carries interned solution terms — including terms minted into the per-query scratch
//! arena, which dies with the evaluation context. Those cannot cross a public boundary:
//! a scratch id outside its own execution is a dangling reference in all but name. So the
//! rows are materialized into the ordinary [`SparqlResult`] egress model — the same
//! model a complete query returns — **before** they reach any type in this module, exactly
//! as the engine already does for a complete result.
//!
//! # What a partial answer is allowed to claim
//!
//! Never "these are the answers". [`PartialAnswers`] states one of the three things the
//! evaluator's prefix-monotonicity certificate can prove, and no more: a lower bound (safe
//! to admit as answers), an upper bound (safe only for "definitely not an answer"), or
//! neither — in which case no row crosses at all and the caller receives the
//! [`NonMonotoneBarrier`] naming the operator that withheld them instead.

use purrdf_core::{GovernorEvidence, SparqlResult, TrippedGovernor};

use crate::governor::NonMonotoneBarrier;

/// The result of one governed query execution.
///
/// Deliberately not `#[non_exhaustive]`: "complete" and "budget exhausted" is the whole
/// taxonomy a governor can produce, and a caller that handles both has handled every
/// outcome. A third arm would be a change to what a governor *means*, which is a breaking
/// change whether or not the compiler is allowed to say so.
#[derive(Debug, Clone)]
pub enum GovernedOutcome {
    /// Every governor stayed intact and this is the query's complete answer.
    ///
    /// The evidence rides along on this path too: "completed, cost N fuel, peak M cells"
    /// is how a caller sizes the next query's budget in the first place (see
    /// [`QueryGovernors::METERED`](crate::governor::QueryGovernors::METERED)).
    Complete {
        /// The query's complete result, in the ordinary egress model.
        result: SparqlResult,
        /// This execution's consumption, ceilings, and (here, always absent) trip.
        evidence: GovernorEvidence,
    },
    /// A governor stopped the execution before it finished. See [`BudgetExhausted`].
    BudgetExhausted(BudgetExhausted),
}

impl GovernedOutcome {
    /// This execution's consumption and ceilings, whichever outcome it reached.
    #[must_use]
    pub const fn evidence(&self) -> &GovernorEvidence {
        match self {
            Self::Complete { evidence, .. } => evidence,
            Self::BudgetExhausted(exhausted) => &exhausted.evidence,
        }
    }

    /// The governor that stopped this execution, or `None` if it completed.
    #[must_use]
    pub const fn tripped(&self) -> Option<TrippedGovernor> {
        match self {
            Self::Complete { .. } => None,
            Self::BudgetExhausted(exhausted) => Some(exhausted.tripped),
        }
    }

    /// Whether every governor stayed intact.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        matches!(self, Self::Complete { .. })
    }

    /// The complete result, or the exhaustion that prevented one.
    ///
    /// The one-line way for a caller that has no use for a partial answer to reduce the
    /// two outcomes to a `Result` — **without** the partial rows being silently dropped
    /// on the way, because they are still there in the `Err`.
    ///
    /// # Errors
    ///
    /// The [`BudgetExhausted`] outcome, when a governor stopped this execution.
    #[allow(
        clippy::result_large_err,
        reason = "the Err side is a typed OUTCOME carrying the execution's receipt — two \
                  ResourceVectors of ceilings and consumption — not a failure path; boxing \
                  it would put an allocation on a value every governed caller reads in \
                  order to save one move per query"
    )]
    pub fn into_complete(self) -> Result<SparqlResult, BudgetExhausted> {
        match self {
            Self::Complete { result, .. } => Ok(result),
            Self::BudgetExhausted(exhausted) => Err(exhausted),
        }
    }

    /// The exhaustion, when a governor stopped this execution.
    #[must_use]
    pub const fn exhausted(&self) -> Option<&BudgetExhausted> {
        match self {
            Self::Complete { .. } => None,
            Self::BudgetExhausted(exhausted) => Some(exhausted),
        }
    }
}

/// A governed execution that ran out of budget: which governor stopped it, what it had
/// spent, and what the rows it reached bound.
///
/// All three travel together because a caller acts on all three: the governor says which
/// ceiling to raise, the evidence says by how much, and the partial answers say whether
/// anything already in hand is usable while that decision is made.
#[derive(Debug, Clone)]
pub struct BudgetExhausted {
    /// The governor that stopped the execution.
    pub tripped: TrippedGovernor,
    /// This execution's consumption, ceilings, and trip.
    pub evidence: GovernorEvidence,
    /// What the rows the execution reached bound, and what they are.
    pub partial: PartialAnswers,
}

/// What a truncated execution's rows bound relative to the query's true answer.
///
/// This is the evaluator's certificate, restated in the egress model. It is a three-way
/// interval, not a yes/no: the two bounds are genuinely different licences and collapsing
/// them would either forbid a sound use or permit an unsound one.
#[derive(Debug, Clone)]
pub enum PartialAnswers {
    /// A certified **lower** bound: every row here is an answer to the query. Safe to
    /// admit as answers; the query may have more.
    Certain(PartialSparqlResult),
    /// A certified **upper** bound: every answer is here, but some rows here may not be
    /// answers. Safe only for the negative reading — a row absent from this result is
    /// definitively not an answer.
    AtMost(PartialSparqlResult),
    /// Neither bound survived to the root, so **no row crosses**. The barrier names the
    /// operator that withheld them, which is what tells a caller whether a larger budget
    /// or a different query is the way forward.
    Unknown(NonMonotoneBarrier),
}

impl PartialAnswers {
    /// The rows in hand, when they bound the answer on either side.
    ///
    /// `None` is [`Self::Unknown`], where there is deliberately nothing to hand out: rows
    /// that bound the answer on neither side offer a caller no sound use, and the one
    /// unsound use — reading them as answers — is the easiest to reach for.
    #[must_use]
    pub const fn result(&self) -> Option<&PartialSparqlResult> {
        match self {
            Self::Certain(partial) | Self::AtMost(partial) => Some(partial),
            Self::Unknown(_) => None,
        }
    }

    /// Take the rows in hand, when they bound the answer on either side.
    #[must_use]
    pub fn into_result(self) -> Option<PartialSparqlResult> {
        match self {
            Self::Certain(partial) | Self::AtMost(partial) => Some(partial),
            Self::Unknown(_) => None,
        }
    }

    /// The operator that withheld the rows, when no bound survived.
    #[must_use]
    pub const fn barrier(&self) -> Option<NonMonotoneBarrier> {
        match self {
            Self::Certain(_) | Self::AtMost(_) => None,
            Self::Unknown(barrier) => Some(*barrier),
        }
    }

    /// Whether these rows are certified answers — i.e. whether a caller may admit them.
    #[must_use]
    pub const fn is_certain(&self) -> bool {
        matches!(self, Self::Certain(_))
    }
}

/// A materialized result computed from the rows a governor left in hand.
///
/// Distinct from a plain [`SparqlResult`] on purpose: this type is only ever reachable
/// from inside a [`PartialAnswers`] arm, so the result cannot be mistaken for a complete
/// one by a caller that stopped reading the outcome one level too early. It also carries
/// the one further fact the certificate proves and the rows themselves do not —
/// [`Self::is_positional_prefix`].
#[derive(Debug, Clone)]
pub struct PartialSparqlResult {
    /// What the rows in hand produced, in the ordinary egress model.
    result: SparqlResult,
    /// Whether those rows are the true output's first rows, in order.
    positional_prefix: bool,
}

impl PartialSparqlResult {
    /// Pair a materialized partial `result` with the certificate's positional verdict.
    pub(crate) const fn new(result: SparqlResult, positional_prefix: bool) -> Self {
        Self {
            result,
            positional_prefix,
        }
    }

    /// The rows in hand, in the ordinary egress model.
    #[must_use]
    pub const fn result(&self) -> &SparqlResult {
        &self.result
    }

    /// Take the rows in hand.
    #[must_use]
    pub fn into_result(self) -> SparqlResult {
        self.result
    }

    /// Whether these rows are the true answer's **first** rows, in order.
    ///
    /// The resumption property: when this holds, re-running the same query over the same
    /// data under a larger budget returns these same rows first, so a caller can page
    /// through a query by raising the ceiling. When it does not, the rows are a sound
    /// sub-bag (or super-bag) of the answer whose *positions* mean nothing — sorting,
    /// `UNION`, and a truncated join input all cost the positional relation while keeping
    /// the multiset one.
    #[must_use]
    pub const fn is_positional_prefix(&self) -> bool {
        self.positional_prefix
    }
}

/// The evidence a governed query over an operationally fallible view accumulates: the
/// view's own operational evidence **and** this execution's governor accounting.
///
/// The two are independent measurements of one execution — pages and bytes on one side,
/// fuel, rows, and cells on the other — and a caller sizing a budget needs both. Pairing
/// them here rather than widening
/// [`CompleteSparqlResult`](crate::CompleteSparqlResult) or
/// [`FallibleSparqlError`](crate::FallibleSparqlError) is what keeps those two types'
/// shapes unchanged: they are already generic over their evidence, so the governed lane
/// simply instantiates that parameter with this pair.
#[derive(Debug, Clone)]
pub struct GovernedEvidence<Evidence> {
    /// The view's deterministic operational evidence at the reporting checkpoint.
    pub view: Evidence,
    /// This execution's consumption, ceilings, and trip.
    pub governors: GovernorEvidence,
}

impl<Evidence> GovernedEvidence<Evidence> {
    /// Pair a view's operational `view` evidence with this execution's `governors`
    /// accounting.
    pub(crate) const fn new(view: Evidence, governors: GovernorEvidence) -> Self {
        Self { view, governors }
    }
}
