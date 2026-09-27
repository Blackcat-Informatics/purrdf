// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
use crate::witness::RelationWitness;

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
        /// The identity of the property-function registry this execution ran under.
        /// See [`RelationIdentity`].
        relations: RelationIdentity,
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

    /// The identity of the property-function registry this execution ran under,
    /// whichever outcome it reached — and, on
    /// [`RelationIdentity::witness`], what the relations it actually invoked attested
    /// about the indexes behind them.
    ///
    /// Empty ([`RelationIdentity::is_empty`]) when no registry, or an empty one, was in
    /// scope — never absent, for the reason [`RelationIdentity`] gives.
    #[must_use]
    pub const fn relations(&self) -> &RelationIdentity {
        match self {
            Self::Complete { relations, .. } => relations,
            Self::BudgetExhausted(exhausted) => &exhausted.relations,
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
    /// The identity of the property-function registry this execution ran under. See
    /// [`RelationIdentity`].
    pub relations: RelationIdentity,
    /// What the rows the execution reached bound, and what they are.
    pub partial: PartialAnswers,
}

/// The identity of the property-function registry a governed execution ran under: the
/// registry's fingerprint (`crate::property_fn_plan::registry_fingerprint`, already
/// computed once at prepare time) paired with the IRIs it covers, sorted.
///
/// # Why a governed receipt needs this at all
///
/// A relation is host code, not data the queried dataset or the query text determine —
/// two runs of the same query, over the same snapshot, under two registries that
/// disagree about nothing but one relation's arity or declared volatility can produce
/// different rows AND a different parallel-safety classification for the same call. See
/// [`QueryExplanation::relations`](crate::governor::QueryExplanation::relations) for the
/// sibling receipt this pairs with on the explain path; this is the same fact carried on
/// the governed path, where an explain call was never made. Without it, a governed
/// receipt that looked identical to another could have answered a different query.
///
/// # Why the fingerprint rather than the full descriptor list
///
/// [`QueryExplanation`](crate::governor::QueryExplanation) already carries the full
/// per-relation descriptors when a caller asked to explain — that is the receipt for a
/// human or a diagnostic to read. [`GovernedOutcome`] is on the hot path of every
/// governed query, so it carries the fingerprint two registries are compared with
/// (already computed once, at prepare, and validated against the plan before evaluation
/// begins — see `check_plan_matches_relations`) rather than re-deriving or re-carrying
/// the full descriptor set on every call. A caller that needs the full description reads
/// it from an explain call over the same registry.
///
/// # What the declarations cannot say, and the witness can
///
/// The fingerprint covers everything DECLARED about a registry, which is exactly what
/// the planner reads — and a relation's index can be rebuilt underneath it without any
/// declaration changing. Two governed runs of one query over one dataset snapshot under
/// one registry could therefore carry byte-identical identities and different rows, with
/// nothing on the receipt to say why. [`Self::witness`] is the other half: what the
/// relations actually attested while they answered. The identity says which relations
/// could have been asked; the witness says what the ones that WERE asked reported about
/// themselves.
///
/// # Absence, not omission
///
/// Empty (the fingerprint, the IRI list, AND the witness) when no registry, or an empty
/// one, was in scope — the same "present but empty" convention every other absence on
/// this receipt uses (see [`QueryExplanation::relations`](crate::governor::QueryExplanation::relations)),
/// never a missing value that could be mistaken for "this build does not report it". The
/// three are independently empty and that is deliberate: a non-empty registry whose
/// relations this query never invoked carries a fingerprint, IRIs, and an EMPTY witness,
/// which is the true statement that relations were in scope and none of them ran. An
/// empty witness is never a claim that an index was whole — see
/// [`ServiceLevel`](crate::ServiceLevel).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RelationIdentity {
    /// The registry's fingerprint — everything about its contents that can change a
    /// plan, an answer, or a parallel-safety classification, taken IRI-sorted so it is a
    /// function of what was registered rather than of registration order. Empty when no
    /// registry was in scope.
    pub fingerprint: String,
    /// The registered IRIs the fingerprint was taken over, sorted.
    pub iris: Vec<String>,
    /// What each relation this execution actually invoked attested about the index
    /// behind it: which generation answered, and whether it declared that index NOT
    /// whole. See [`RelationWitness`].
    ///
    /// This is the per-run half of this receipt, riding beside the per-registry half,
    /// exactly as [`GovernorEvidence`] rides beside the ceilings that produced it.
    pub witness: RelationWitness,
}

impl RelationIdentity {
    /// The empty identity: no registry, or an empty one, was in scope, and nothing
    /// attested.
    pub const EMPTY: Self = Self {
        fingerprint: String::new(),
        iris: Vec::new(),
        // `const`-constructible for the same reason `String::new()` and `Vec::new()`
        // above are, so this constant survives the witness field without being
        // downgraded to a function. See [`RelationWitness::EMPTY`].
        witness: RelationWitness::EMPTY,
    };

    /// Whether no registry, or an empty one, was in scope.
    ///
    /// Reads the fingerprint ONLY, deliberately: the witness is per-run evidence, and a
    /// run under a real registry that happened to invoke no relation has an empty
    /// witness while the registry it ran under was anything but empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fingerprint.is_empty()
    }
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

/// Whether `term`, or any component of it at any depth, is a blank node `withhold`
/// selects.
///
/// The blank nodes are offered to `withhold` in the order they are written — a triple
/// term's subject, then its predicate, then its object, each fully before the next —
/// and the first one selected ends the walk. The walk keeps its own work list, so a
/// term of any nesting costs no more machine stack.
fn term_value_mentions_withheld_blank(
    term: &TermValue,
    withhold: &mut impl FnMut(&str) -> bool,
) -> bool {
    let mut pending: Vec<&TermValue> = vec![term];
    while let Some(term) = pending.pop() {
        match term {
            TermValue::Blank { label, .. } => {
                if withhold(label) {
                    return true;
                }
            }
            TermValue::Triple { s, p, o } => pending.extend([&**o, &**p, &**s]),
            TermValue::Iri(_) | TermValue::Literal { .. } => {}
        }
    }
    false
}

/// [`term_value_mentions_withheld_blank`] over an egress [`RdfTerm`], whose triple
/// term nests through its subject and object (its predicate is an IRI string).
fn rdf_term_mentions_withheld_blank(
    term: &RdfTerm,
    withhold: &mut impl FnMut(&str) -> bool,
) -> bool {
    let mut pending: Vec<&RdfTerm> = vec![term];
    while let Some(term) = pending.pop() {
        match term {
            RdfTerm::BlankNode(label) => {
                if withhold(label) {
                    return true;
                }
            }
            RdfTerm::Triple(triple) => pending.extend([&triple.object, &triple.subject]),
            RdfTerm::Iri(_) | RdfTerm::Literal(_) => {}
        }
    }
    false
}

/// Rebuild `dataset` without selected blank-bearing items, returning `None` for a no-op.
///
/// The one-pass rebuild is intentional: selection and copying happen in the same traversal,
/// so the reported `removed` fact describes the transformation actually applied rather than
/// the result of a separate preflight scan.
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

#[cfg(test)]
mod withheld_blank_walk_tests {
    //! The withheld-blank walks over egress terms, checked against a recursive
    //! reference: the same answer and the same labels offered to the predicate in the
    //! same order, over generated shapes; and a term a hundred thousand levels deep,
    //! walked on a thread with a 128 KiB stack.

    use super::{rdf_term_mentions_withheld_blank, term_value_mentions_withheld_blank};
    use purrdf_core::{BlankScope, RdfLiteral, RdfTerm, RdfTriple, TermBox, TermValue};

    const EX: &str = "http://example.org/";
    const LABELS: [&str; 4] = ["a", "b", "c", "d"];
    const DEPTH: usize = 100_000;
    const SMALL_STACK: usize = 128 * 1024;

    /// A deterministic choice sequence.
    struct Choices {
        state: u64,
    }

    impl Choices {
        const fn new(seed: u64) -> Self {
            Self { state: seed }
        }

        /// One choice below `n`.
        fn choose(&mut self, n: usize) -> usize {
            let bound = u64::try_from(n).expect("a choice count fits");
            usize::try_from(crate::test_rng::splitmix64_next(&mut self.state) % bound)
                .expect("a draw below the count fits")
        }
    }

    /// A generated term value: leaves of every kind, and triple terms while `budget`
    /// lasts.
    fn value(choices: &mut Choices, budget: &mut usize) -> TermValue {
        match choices.choose(if *budget > 0 { 4 } else { 3 }) {
            0 => TermValue::Iri(format!("{EX}i{}", choices.choose(3))),
            1 => TermValue::Blank {
                label: LABELS[choices.choose(LABELS.len())].to_owned(),
                scope: BlankScope::DEFAULT,
            },
            2 => TermValue::Literal {
                lexical_form: "x".to_owned(),
                datatype: format!("{EX}dt"),
                language: None,
                direction: None,
            },
            _ => {
                *budget -= 1;
                TermValue::Triple {
                    s: TermBox::new(value(choices, budget)),
                    p: TermBox::new(value(choices, budget)),
                    o: TermBox::new(value(choices, budget)),
                }
            }
        }
    }

    /// A generated egress term, shaped like [`value`].
    fn rdf_term(choices: &mut Choices, budget: &mut usize) -> RdfTerm {
        match choices.choose(if *budget > 0 { 4 } else { 3 }) {
            0 => RdfTerm::Iri(format!("{EX}i{}", choices.choose(3))),
            1 => RdfTerm::BlankNode(LABELS[choices.choose(LABELS.len())].to_owned()),
            2 => RdfTerm::Literal(RdfLiteral::simple("x")),
            _ => {
                *budget -= 1;
                let subject = rdf_term(choices, budget);
                let object = rdf_term(choices, budget);
                RdfTerm::Triple(Box::new(RdfTriple::new(subject, format!("{EX}p"), object)))
            }
        }
    }

    /// The recursive reference for [`term_value_mentions_withheld_blank`].
    fn reference_value(term: &TermValue, withhold: &mut impl FnMut(&str) -> bool) -> bool {
        match term {
            TermValue::Blank { label, .. } => withhold(label),
            TermValue::Triple { s, p, o } => {
                reference_value(s, withhold)
                    || reference_value(p, withhold)
                    || reference_value(o, withhold)
            }
            TermValue::Iri(_) | TermValue::Literal { .. } => false,
        }
    }

    /// The recursive reference for [`rdf_term_mentions_withheld_blank`].
    fn reference_rdf(term: &RdfTerm, withhold: &mut impl FnMut(&str) -> bool) -> bool {
        match term {
            RdfTerm::BlankNode(label) => withhold(label),
            RdfTerm::Triple(triple) => {
                reference_rdf(&triple.subject, withhold) || reference_rdf(&triple.object, withhold)
            }
            RdfTerm::Iri(_) | RdfTerm::Literal(_) => false,
        }
    }

    /// Run `body` on a fresh thread with [`SMALL_STACK`] of stack.
    fn on_small_stack<T: Send + 'static>(body: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(SMALL_STACK)
            .spawn(body)
            .expect("spawn")
            .join()
            .expect("the 128 KiB thread returned")
    }

    /// Release a deep egress term one level at a time; its derived drop would take one
    /// stack frame per level.
    fn dismantle(mut term: RdfTerm) {
        while let RdfTerm::Triple(triple) = term {
            term = triple.object;
        }
    }

    #[test]
    fn the_term_value_walk_offers_the_same_labels_as_the_recursive_reference() {
        for seed in 0..200_u64 {
            let mut choices = Choices::new(seed);
            let mut budget = 6;
            let term = value(&mut choices, &mut budget);
            for target in LABELS.iter().chain(std::iter::once(&"none")) {
                let mut seen_walk = Vec::new();
                let hit_walk = term_value_mentions_withheld_blank(&term, &mut |label| {
                    seen_walk.push(label.to_owned());
                    label == *target
                });
                let mut seen_ref = Vec::new();
                let hit_ref = reference_value(&term, &mut |label| {
                    seen_ref.push(label.to_owned());
                    label == *target
                });
                assert_eq!((hit_walk, seen_walk), (hit_ref, seen_ref), "seed {seed}");
            }
        }
    }

    #[test]
    fn the_egress_term_walk_offers_the_same_labels_as_the_recursive_reference() {
        for seed in 0..200_u64 {
            let mut choices = Choices::new(seed);
            let mut budget = 6;
            let term = rdf_term(&mut choices, &mut budget);
            for target in LABELS.iter().chain(std::iter::once(&"none")) {
                let mut seen_walk = Vec::new();
                let hit_walk = rdf_term_mentions_withheld_blank(&term, &mut |label| {
                    seen_walk.push(label.to_owned());
                    label == *target
                });
                let mut seen_ref = Vec::new();
                let hit_ref = reference_rdf(&term, &mut |label| {
                    seen_ref.push(label.to_owned());
                    label == *target
                });
                assert_eq!((hit_walk, seen_walk), (hit_ref, seen_ref), "seed {seed}");
            }
        }
    }

    /// A blank node at every level, and the selected one at the bottom: every level's
    /// label is offered before the walk answers.
    #[test]
    fn a_hundred_thousand_level_term_is_walked_on_a_128_kib_stack() {
        on_small_stack(|| {
            let mut term = TermValue::Blank {
                label: "deep".to_owned(),
                scope: BlankScope::DEFAULT,
            };
            for _ in 0..DEPTH {
                term = TermValue::Triple {
                    s: TermBox::new(TermValue::Blank {
                        label: "b".to_owned(),
                        scope: BlankScope::DEFAULT,
                    }),
                    p: TermBox::new(TermValue::Iri(format!("{EX}p"))),
                    o: TermBox::new(term),
                };
            }
            let mut offered = 0_usize;
            assert!(term_value_mentions_withheld_blank(&term, &mut |label| {
                offered += 1;
                label == "deep"
            }));
            assert_eq!(offered, DEPTH + 1);
            let mut offered = 0_usize;
            assert!(!term_value_mentions_withheld_blank(&term, &mut |label| {
                offered += 1;
                label == "none"
            }));
            assert_eq!(offered, DEPTH + 1);

            let mut egress = RdfTerm::BlankNode("deep".to_owned());
            for _ in 0..DEPTH {
                egress = RdfTerm::Triple(Box::new(RdfTriple::new(
                    RdfTerm::BlankNode("b".to_owned()),
                    format!("{EX}p"),
                    egress,
                )));
            }
            let mut offered = 0_usize;
            assert!(rdf_term_mentions_withheld_blank(&egress, &mut |label| {
                offered += 1;
                label == "deep"
            }));
            assert_eq!(offered, DEPTH + 1);
            dismantle(egress);
        });
    }
}
