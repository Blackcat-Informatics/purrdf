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
//!
//! # UPDATE has its own outcome, and it has no partial arm
//!
//! [`GovernedUpdateOutcome`] is deliberately *not* [`GovernedOutcome`]. A query's partial
//! answer is a useful, certifiable thing; a partial *mutation* is not a thing at all. See
//! that type for the argument.

use std::sync::Arc;

use purrdf_core::{
    GovernorEvidence, RdfDataset, RdfDatasetBuilder, RdfTerm, SparqlResult, TermValue,
    TrippedGovernor,
};

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

/// The result of one governed SPARQL **UPDATE** request.
///
/// # Why this is not [`GovernedOutcome`]
///
/// [`GovernedOutcome`] exists because a truncated *query* has something to hand back: the
/// rows already reached, plus a machine-checked statement of what they bound. That
/// reasoning does not transfer. A request either applied or it did not — there is no
/// certifiable "partial mutation", because the thing a partial answer certifies (a bound
/// on a set of rows) has no counterpart in a store that a caller will go on to read as if
/// it were whole. An `INSERT`/`DELETE` that landed halfway and was reported as "budget
/// exhausted" is not an incomplete result; it is a corrupt store, and the corruption is
/// silent — every later query answers confidently from it.
///
/// So the trip arm below carries the governor and the evidence and **structurally nothing
/// else**: there is no field a caller could read partial mutations out of, because the
/// engine guarantees there are none to read. A tripped request leaves the caller's dataset
/// handle exactly as it found it — the same `Arc`, not merely an equal one.
///
/// # The vocabulary is shared with the query path
///
/// [`TrippedGovernor`] and [`GovernorEvidence`] are the same kernel types
/// [`GovernedOutcome`] reports, so a caller writes one governor renderer and one
/// budget-sizing routine for both paths. Only the *shape* of the outcome differs, because
/// only the shape genuinely differs.
///
/// Deliberately not `#[non_exhaustive]`, for the reason [`GovernedOutcome`] gives: a third
/// arm would be a change to what a governor means.
#[derive(Debug, Clone)]
pub enum GovernedUpdateOutcome {
    /// Every operation of the request applied, and the store now reflects all of them.
    ///
    /// The evidence rides along here for the same reason it does on the query path:
    /// "applied, cost N fuel, peak M cells" is how a caller sizes the next request's
    /// budget (see [`QueryGovernors::METERED`](crate::governor::QueryGovernors::METERED)).
    Applied {
        /// This request's consumption, ceilings, and (here, always absent) trip.
        evidence: GovernorEvidence,
    },
    /// A governor stopped the request, and **no operation of it was applied**.
    ///
    /// Not "some of it applied". The store is byte-identical to what it was before the
    /// request was submitted, whichever operation the governor stopped and however much
    /// work the earlier operations of the same request had already done.
    BudgetExhausted {
        /// The governor that stopped the request.
        tripped: TrippedGovernor,
        /// This request's consumption, ceilings, and trip.
        evidence: GovernorEvidence,
    },
}

impl GovernedUpdateOutcome {
    /// This request's consumption and ceilings, whichever outcome it reached.
    #[must_use]
    pub const fn evidence(&self) -> &GovernorEvidence {
        match self {
            Self::Applied { evidence } | Self::BudgetExhausted { evidence, .. } => evidence,
        }
    }

    /// The governor that stopped this request, or `None` if it applied.
    #[must_use]
    pub const fn tripped(&self) -> Option<TrippedGovernor> {
        match self {
            Self::Applied { .. } => None,
            Self::BudgetExhausted { tripped, .. } => Some(*tripped),
        }
    }

    /// Whether the request applied.
    ///
    /// `false` means **nothing** applied, never "not all of it applied".
    #[must_use]
    pub const fn is_applied(&self) -> bool {
        matches!(self, Self::Applied { .. })
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

    /// Withhold every solution row or graph item that mentions a blank node selected by
    /// `withhold`, when there are rows in hand.
    ///
    /// The callback receives only an immutable blank-node label. The method itself performs
    /// the removal, so a caller cannot add, reorder, or rewrite a certified row through this
    /// API. A lower bound remains a lower bound after removal. Removing anything from an
    /// upper bound is different: the removed item might have been a true answer, so the
    /// upper-bound certificate is discarded and [`Self::Unknown`] names the
    /// `blank-node-filter` boundary. An upper bound is retained only when the filter was a
    /// no-op. Removing anything from a lower bound also clears its positional-prefix claim,
    /// because the retained rows now have holes even though every one remains certain.
    ///
    /// Blank nodes nested inside RDF 1.2 triple terms are visited recursively. For a graph
    /// result, ordinary quads, reifier bindings, annotations, and named-graph declarations
    /// are all filtered. [`Self::Unknown`] passes through untouched because it carries no
    /// rows to inspect.
    #[must_use]
    pub fn withholding_blank_nodes(self, mut withhold: impl FnMut(&str) -> bool) -> Self {
        match self {
            Self::Certain(partial) => {
                let (partial, _) = partial.withholding_blank_nodes(&mut withhold);
                Self::Certain(partial)
            }
            Self::AtMost(partial) => {
                let (partial, removed) = partial.withholding_blank_nodes(&mut withhold);
                if removed {
                    Self::Unknown(NonMonotoneBarrier::named("blank-node-filter"))
                } else {
                    Self::AtMost(partial)
                }
            }
            Self::Unknown(barrier) => Self::Unknown(barrier),
        }
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
    /// This is a relation to the complete output, not by itself a cross-run timing promise.
    /// For deterministic ceilings, re-running the same query and snapshot under a larger
    /// ceiling returns these rows first, so a caller can page by raising that ceiling. A
    /// wall deadline is not deterministic: a later run can stop sooner even with a longer
    /// duration, so it must be treated as a fresh run. When this bit is false, the rows are
    /// only a sound sub-bag (or super-bag) whose positions mean nothing — sorting, `UNION`,
    /// and a truncated join input all cost the positional relation while keeping the
    /// multiset one.
    #[must_use]
    pub const fn is_positional_prefix(&self) -> bool {
        self.positional_prefix
    }

    /// Apply the structurally-removal-only blank-node filter behind
    /// [`PartialAnswers::withholding_blank_nodes`].
    fn withholding_blank_nodes(mut self, withhold: &mut impl FnMut(&str) -> bool) -> (Self, bool) {
        let removed = withhold_blank_nodes_from_result(&mut self.result, withhold);
        if removed {
            self.positional_prefix = false;
        }
        (self, removed)
    }
}

/// Remove every output item containing a selected blank node.
///
/// The predicate never receives mutable result data. All reconstruction happens here, so
/// the only possible transformation is a deterministic, order-preserving subset.
fn withhold_blank_nodes_from_result(
    result: &mut SparqlResult,
    withhold: &mut impl FnMut(&str) -> bool,
) -> bool {
    match result {
        SparqlResult::Solutions { rows, aux, .. } => {
            let before = rows.len();
            rows.retain(|row| {
                !row.iter()
                    .flatten()
                    .any(|term| term_value_mentions_withheld_blank(term, withhold))
            });
            let removed_rows = rows.len() != before;
            let removed_aux =
                if let Some(filtered) = dataset_without_withheld_blank_nodes(aux, withhold) {
                    *aux = filtered;
                    true
                } else {
                    false
                };
            removed_rows || removed_aux
        }
        SparqlResult::Graph(graph) => {
            if let Some(filtered) = dataset_without_withheld_blank_nodes(graph, withhold) {
                *graph = filtered;
                true
            } else {
                false
            }
        }
        SparqlResult::Boolean(_) => false,
    }
}

fn term_value_mentions_withheld_blank(
    term: &TermValue,
    withhold: &mut impl FnMut(&str) -> bool,
) -> bool {
    match term {
        TermValue::Blank { label, .. } => withhold(label),
        TermValue::Triple { s, p, o } => {
            term_value_mentions_withheld_blank(s, withhold)
                || term_value_mentions_withheld_blank(p, withhold)
                || term_value_mentions_withheld_blank(o, withhold)
        }
        TermValue::Iri(_) | TermValue::Literal { .. } => false,
    }
}

fn rdf_term_mentions_withheld_blank(
    term: &RdfTerm,
    withhold: &mut impl FnMut(&str) -> bool,
) -> bool {
    match term {
        RdfTerm::BlankNode(label) => withhold(label),
        RdfTerm::Triple(triple) => {
            rdf_term_mentions_withheld_blank(&triple.subject, withhold)
                || rdf_term_mentions_withheld_blank(&triple.object, withhold)
        }
        RdfTerm::Iri(_) | RdfTerm::Literal(_) => false,
    }
}

/// Rebuild `dataset` without selected blank-bearing items, returning `None` for a no-op.
///
/// The one-pass rebuild is intentional: it invokes a stateful caller predicate exactly once
/// per visited occurrence, so the reported `removed` fact is the transformation that was
/// actually applied rather than the result of a separate preflight scan.
fn dataset_without_withheld_blank_nodes(
    dataset: &Arc<RdfDataset>,
    withhold: &mut impl FnMut(&str) -> bool,
) -> Option<Arc<RdfDataset>> {
    let mut builder = RdfDatasetBuilder::new();
    let mut removed = false;

    for quad in dataset.owned_quads() {
        let should_withhold = rdf_term_mentions_withheld_blank(&quad.subject, withhold)
            || rdf_term_mentions_withheld_blank(&quad.object, withhold)
            || quad
                .graph_name
                .as_ref()
                .is_some_and(|graph| rdf_term_mentions_withheld_blank(graph, withhold));
        if should_withhold {
            removed = true;
        } else {
            builder.push_owned_quad(&quad);
        }
    }
    for reifier in dataset.owned_reifiers() {
        let should_withhold = rdf_term_mentions_withheld_blank(&reifier.reifier, withhold)
            || rdf_term_mentions_withheld_blank(&reifier.statement.subject, withhold)
            || rdf_term_mentions_withheld_blank(&reifier.statement.object, withhold)
            || reifier
                .graph
                .as_ref()
                .is_some_and(|graph| rdf_term_mentions_withheld_blank(graph, withhold));
        if should_withhold {
            removed = true;
        } else {
            builder.push_owned_reifier(&reifier);
        }
    }
    for annotation in dataset.owned_annotations() {
        let should_withhold = rdf_term_mentions_withheld_blank(&annotation.reifier, withhold)
            || rdf_term_mentions_withheld_blank(&annotation.object, withhold)
            || annotation
                .graph
                .as_ref()
                .is_some_and(|graph| rdf_term_mentions_withheld_blank(graph, withhold));
        if should_withhold {
            removed = true;
        } else {
            builder.push_owned_annotation(&annotation);
        }
    }
    for name in dataset.owned_named_graphs() {
        if rdf_term_mentions_withheld_blank(&name, withhold) {
            removed = true;
        } else {
            let id = builder.intern_owned_term(&name);
            builder.declare_named_graph(id);
        }
    }

    removed.then(|| {
        builder
            .freeze()
            .expect("a subset of a frozen result dataset is itself a valid dataset")
    })
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
