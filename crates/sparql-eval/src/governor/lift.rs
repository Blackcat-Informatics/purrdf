// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The partial-lift channel: the third result an operator can produce, beside a complete
//! bag and a genuine failure.
//!
//! # Why a third channel exists at all
//!
//! Evaluation is fully materialized and every operator returns
//! `Result<SolutionSeq, EvalError>`. With only those two channels a governor trip deep in
//! the plan has exactly one way out — the error channel — and an error carries no rows,
//! so "the partial answers found so far" would be unimplementable for every governor
//! except a cap tested at the very end. [`Evaluated`] is that third channel:
//! [`Evaluated::Complete`] is the ordinary result, and [`Evaluated::Truncated`] carries
//! the rows the evaluator actually holds **together with a machine-checked statement of
//! what they bound**.
//!
//! # What a truncated bag is allowed to claim
//!
//! Never "these are the answers". A [`Truncation`] claims exactly one of three things,
//! and the claim is composed, not asserted:
//!
//! - [`SpineClass::Certain`] — a **lower** bound: every row here is an answer. Safe to
//!   admit.
//! - [`SpineClass::Possible`] — an **upper** bound: every answer is here, but some rows
//!   here may not be answers. Safe only for "definitely not an answer".
//! - [`SpineClass::Unknown`] — **neither**. No row crosses; a [`NonMonotoneBarrier`]
//!   crosses in their place, naming the operator that withheld them.
//!
//! The composition is the classical interval algebra over the two axes
//! [`crate::governor::soundness`] computes — which side of the interval an edge
//! contributes to ([`ChildRole`]) and whether the positional prefix relation survives it
//! ([`crate::governor::soundness::PrefixFidelity`]). A parent never invents a bound: it
//! pushes the edge it descended onto the truncation's path and the class is **recomputed
//! from that path by the same [`SpineContext::descend`] the plan-level analysis uses**.
//! One composition function, so the evaluator's certificate and the plan-level
//! certificate cannot disagree — an equality that would otherwise be a coincidence two
//! implementations have to keep agreeing on.
//!
//! # Commit granularity
//!
//! A row is *committed* only when the producing operator has finished **all** output for
//! the input row that generated it. A trip discards the in-flight operator instance's
//! uncommitted output entirely.
//!
//! This is the single most dangerous rule in the whole channel, and `OPTIONAL` is where
//! it bites. `LeftJoin` emits a left row padded with unbound **iff no compatible right
//! row exists**; that is a statement about the *whole* right bag. Emitting a padded left
//! row while holding only part of the right bag would state something the evaluator has
//! not established — a fabricated answer, which is categorically worse than a missing
//! one. So an operator whose per-input-row output depends on completing a scan
//! (`LeftJoin`, `Minus`, `Group`, `EXISTS`/`NOT EXISTS`, `Distinct`) commits per **input**
//! row, never per emitted row, and when it cannot complete the scan it emits nothing for
//! that input row at all.
//!
//! It bites in **both** directions at `LeftJoin`, which is the part that was originally
//! got wrong. A truncated *left* arm must not pad, for the reason above. A truncated
//! *right* arm must not pad either — the right bag is a prefix, so "no compatible right
//! row exists" is not a fact the operator holds about any left row — and the padded row
//! is not merely imprecise there, it is emitted **instead of** the true pairings the cut
//! hid, so it is neither a lower bound nor an upper one. With the padding suppressed the
//! operator computes an inner join over the prefix, whose every row is an answer; that is
//! why `LeftJoin`'s right edge is [`ChildEdge::MONOTONE_BAG`] and not
//! [`ChildEdge::ANTITONE`]. `Minus` keeps the antitone edge, because subtracting less
//! really does leave a superset.
//!
//! # One observable rule per operator, whichever path evaluated it
//!
//! A governor may not report a different partial result because a different scheduler
//! ran. Only `UNION` starts two sibling patterns at once (`rayon::join`), and under
//! engaged governors it no longer does: both branches are whole patterns, so both charge
//! the one shared `GovernorState`, and forking them let the two arms race the same
//! counter. That was not a theoretical hazard — `{ ?s ex:p ?o } UNION { ?s ex:q ?o }`
//! under nine fuel produced seven distinct outcomes across sixty runs of one process,
//! differing in both the certified rows and the reported consumption. A governed `UNION`
//! therefore evaluates its arms in source order on one thread
//! ([`EvalCtx::may_fork_sibling_patterns`](crate::eval::EvalCtx::may_fork_sibling_patterns)),
//! and `binop::union_branch_order` keeps the discard rule that the ungoverned fork still
//! needs — a computed right branch is dropped when the left one truncated, so the two
//! paths agree on rows as well as on budget. Audited and found not to have the shape at
//! all: `Join`, `Minus`, `LeftJoin`, and `Lateral` evaluate their arms one after the
//! other on one thread, so a truncated left arm stops the operator before the right arm
//! begins on either path; the chunked row loops in `FILTER`/`BIND`/`OPTIONAL`-with-filter
//! and the per-group aggregate fork parallelize *within* one operator over an input that
//! is already materialized, and a truncation reached from inside one of their expressions
//! withholds the operator's entire output on both paths, so both agree on the empty
//! result. The governor identity they report is the one write-once latched trip, so two
//! workers cannot disagree about which governor fired either.
//!
//! # The lift's own budget rule
//!
//! Completing an operator over a truncated child is real work performed **after** the
//! budget is spent. Two rules keep that from making the trip point unpredictable:
//!
//! 1. **The lift is exempt from charging.** It spends no fuel. If it charged, the point
//!    at which a query stops would stop being a pure function of `(query, data, budget)` —
//!    it would also depend on how much lifting happened to be needed above the trip, which
//!    is a property of the plan shape rather than of the budget.
//! 2. **The lift is bounded by the already-committed row count.** An operator that
//!    absorbs a truncated child finishes *its own* computation over the rows already in
//!    hand and **never begins evaluating a child it has not started** ([`Lift::absorb`]
//!    stops the operator through [`Lift::is_truncated`]). Without that second rule a fuel
//!    trip in the left arm of a join would still license a full scan of the right arm, and
//!    the governor would bound nothing.
//!
//! The first rule has a consequence that has to be paid for on the *other* side, or the
//! budget stops being monotone. A node under a truncation passes its whole bag upward for
//! free; a node whose child merely *completed* would, if its own committed-row charge cut
//! that bag, report fewer rows than the free lift reports one unit of budget lower. It
//! did: `SELECT * WHERE { ?s ex:p ?o }` over three edges returned two certified rows at 7
//! fuel and none at 8. So the committed-row charge does not cut — see
//! [`EvalCtx::charge_committed_rows`](crate::eval::EvalCtx::charge_committed_rows) for
//! why cutting an already-materialized bag saves no work and costs the resumption
//! contract. The charges that bound *work* (a `FILTER` predicate not yet run, a `BIND`
//! expression not yet evaluated) still cut, because there the refusal is the bound.

use std::sync::{Arc, OnceLock};

use purrdf_core::{TrippedGovernor, ViewTermId};
use purrdf_sparql_algebra::GraphPattern;

use crate::governor::soundness::{
    ChildEdge, ChildEdges, OrderCertainty, PrefixFidelity, SpineClass, SpineContext, child_edges,
    pattern_label,
};
use crate::solution::{SolutionSeq, VarSchema};

/// The operator that withheld a truncation's rows because no bound survived it.
///
/// Carried in place of the rows, never beside them: a caller that receives a barrier
/// receives no rows at all, so there is nothing it could mistake for an answer. What it
/// gets instead is the actionable half — *which* operator turned a partial result into no
/// result, which is what tells a caller whether raising the budget or rewriting the query
/// is the way forward.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NonMonotoneBarrier {
    /// The algebra variant label of the operator that withheld the rows.
    operator: &'static str,
}

impl NonMonotoneBarrier {
    /// The barrier at `node`, labelled by its algebra variant.
    pub(crate) fn at(node: &GraphPattern) -> Self {
        Self {
            operator: pattern_label(node),
        }
    }

    /// A barrier introduced at an egress boundary rather than by an algebra node.
    pub(crate) const fn named(operator: &'static str) -> Self {
        Self { operator }
    }

    /// The algebra variant label of the operator that withheld the rows.
    #[must_use]
    pub const fn operator(self) -> &'static str {
        self.operator
    }
}

impl std::fmt::Display for NonMonotoneBarrier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "no bound survives {}", self.operator)
    }
}

/// What a truncation certifies, and how it got there.
///
/// The class is **not** stored: it is recomputed from `path` on demand by folding
/// [`SpineContext::descend`] from [`SpineContext::ROOT`] in root-to-origin order. Storing
/// a composed class and a path that could disagree with it would be two sources of truth
/// for the one thing this type exists to state; recomputing costs a handful of branches
/// per edge and is paid only on a path that has already tripped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Certificate {
    /// The edges from the node the truncation originated at up to the node it currently
    /// sits at, **origin-first**. The fold walks it in reverse, because
    /// [`SpineContext::descend`] composes from the root downwards.
    path: Vec<ChildEdge>,
    /// The governor that stopped the execution, propagated unchanged to the root.
    tripped: TrippedGovernor,
    /// The operator that collapsed the class to [`SpineClass::Unknown`], if one has.
    barrier: Option<NonMonotoneBarrier>,
}

impl Certificate {
    /// A certificate for a trip that happened **at** the node holding it: no edge has
    /// been crossed yet, so it certifies a positional lower bound on that node's own
    /// output.
    pub(crate) const fn origin(tripped: TrippedGovernor) -> Self {
        Self {
            path: Vec::new(),
            tripped,
            barrier: None,
        }
    }

    /// A certificate for work that completed behind a blocking host seam after the stop
    /// signal fired and was therefore discarded.
    ///
    /// The surviving rows are still a lower bound, but they are no longer licensed as a
    /// positional prefix: the discarded host response may have belonged between rows the
    /// surrounding plan had already committed. Encoding that loss as the same
    /// [`ChildEdge::MONOTONE_BAG`] the ordinary lift uses keeps the interval algebra as the
    /// single source of truth.
    pub(crate) fn bag_only_origin(tripped: TrippedGovernor) -> Self {
        Self {
            path: vec![ChildEdge::MONOTONE_BAG],
            tripped,
            barrier: None,
        }
    }

    /// The composed classification: what the rows this certificate describes bound,
    /// relative to the true output of the node currently holding them.
    ///
    /// Folded with the plan-level [`SpineContext::descend`], not with a second copy of
    /// the interval algebra.
    pub(crate) fn spine(&self) -> SpineContext {
        self.path
            .iter()
            .rev()
            .fold(SpineContext::ROOT, |context, &edge| context.descend(edge))
    }

    /// Cross one edge upwards: the node `parent` reached this truncation through `edge`.
    ///
    /// Records a [`NonMonotoneBarrier`] at the first node whose edge collapses the class
    /// to [`SpineClass::Unknown`] — the *nearest-the-origin* such node, because once a
    /// bound is gone no later operator can restore it and blaming a node further up would
    /// point a caller at the wrong operator.
    fn ascend(&mut self, edge: ChildEdge, parent: &GraphPattern) {
        let before = self.spine().class();
        self.path.push(edge);
        if self.spine().class() == SpineClass::Unknown && before != SpineClass::Unknown {
            self.barrier = Some(NonMonotoneBarrier::at(parent));
        }
    }

    /// Collapse an upper bound whose rows had to be removed at the answer cap.
    ///
    /// Removing from a lower bound is sound; removing an arbitrary member of an upper
    /// bound is not, because the removed member may be a true answer. This root-level
    /// barrier makes that loss structural: [`Truncation::new`] empties the rows and the
    /// public outcome names `answer-cap` instead of exposing a forged upper bound.
    fn with_answer_cap_barrier(mut self) -> Self {
        if self.spine().class() == SpineClass::Possible {
            self.path.push(ChildEdge::OPAQUE);
            self.barrier = Some(NonMonotoneBarrier::named("answer-cap"));
        }
        self
    }
}

/// A partial result: the rows an operator holds, and what they bound.
///
/// Constructing one is the only way to attach rows to a [`Certificate`], and it enforces
/// the rule that makes the certificate safe to hand to a caller: **when the bound is
/// [`SpineClass::Unknown`], no rows cross**. Carrying rows that bound the answer on
/// neither side would offer a caller something it has no sound way to use, and the one
/// unsound use — reading them as answers — is the easiest to reach for.
#[derive(Debug, Clone)]
pub(crate) struct Truncation<I: ViewTermId> {
    /// The rows in hand at this node. Empty whenever [`Certificate::spine`] classifies
    /// them [`SpineClass::Unknown`].
    rows: SolutionSeq<I>,
    /// What `rows` bound relative to this node's true output.
    certificate: Certificate,
}

impl<I: ViewTermId> Truncation<I> {
    /// Attach `rows` to `certificate`, emptying them when no bound survives.
    pub(crate) fn new(rows: SolutionSeq<I>, certificate: Certificate) -> Self {
        let rows = if certificate.spine().class() == SpineClass::Unknown {
            // The schema is kept: it describes the columns this node WOULD have
            // produced, which a caller still needs to render an empty partial result.
            SolutionSeq::empty(rows.schema)
        } else {
            rows
        };
        Self { rows, certificate }
    }

    /// A truncation originating at the node that holds `rows`: the governor tripped here,
    /// and `rows` are the rows this node committed before it did.
    pub(crate) fn origin(rows: SolutionSeq<I>, tripped: TrippedGovernor) -> Self {
        Self::new(rows, Certificate::origin(tripped))
    }

    /// A truncation whose rows remain a lower bound but are not a positional prefix.
    pub(crate) fn bag_only_origin(rows: SolutionSeq<I>, tripped: TrippedGovernor) -> Self {
        Self::new(rows, Certificate::bag_only_origin(tripped))
    }

    /// A truncation originating at `node` from **inside one of its expressions** — an
    /// `EXISTS` subtree, which is [`ChildRole::Opaque`], so nothing crosses and `node` is
    /// the barrier.
    ///
    /// `schema` is the node's own output schema. No rows cross, but the columns still do:
    /// a caller diffing the column list of a partial result against a complete one must
    /// not see them differ, and at an expression barrier the operator has already
    /// computed its output schema, so reporting it costs nothing.
    pub(crate) fn barred_at(
        node: &GraphPattern,
        tripped: TrippedGovernor,
        schema: Arc<VarSchema>,
    ) -> Self {
        let mut certificate = Certificate::origin(tripped);
        certificate.ascend(ChildEdge::OPAQUE, node);
        Self::new(SolutionSeq::empty(schema), certificate)
    }

    /// Which side of the answer interval these rows bound.
    pub(crate) fn bound(&self) -> SpineClass {
        self.certificate.spine().class()
    }

    /// Whether these rows are a positional prefix of this node's true output, or only a
    /// sub-bag (or super-bag) of it.
    pub(crate) fn fidelity(&self) -> PrefixFidelity {
        match self.certificate.spine().order() {
            OrderCertainty::Ordered => PrefixFidelity::Positional,
            OrderCertainty::Unordered => PrefixFidelity::BagOnly,
        }
    }

    /// Whether these rows are the true output's **first** rows, in order.
    ///
    /// This is [`SpineContext::admits_cap_pushdown`] read for its other purpose: the
    /// predicate that licenses stopping a scan early is, word for word, the predicate
    /// that says a partial bag is a genuine positional prefix — which is what makes a
    /// re-run under a larger budget return these same rows first.
    pub(crate) fn is_positional_prefix(&self) -> bool {
        self.certificate.spine().admits_cap_pushdown()
    }

    /// The governor that stopped the execution.
    pub(crate) const fn tripped(&self) -> TrippedGovernor {
        self.certificate.tripped
    }

    /// The operator that withheld the rows, when no bound survived.
    pub(crate) const fn barrier(&self) -> Option<NonMonotoneBarrier> {
        self.certificate.barrier
    }

    /// The rows in hand, whatever they bound.
    pub(crate) const fn rows(&self) -> &SolutionSeq<I> {
        &self.rows
    }

    /// The rows, but only when they are a certified **lower** bound — i.e. only when
    /// every one of them is an answer.
    pub(crate) fn certain_rows(&self) -> Option<&SolutionSeq<I>> {
        (self.bound() == SpineClass::Certain).then_some(&self.rows)
    }

    /// A one-line description of what these rows bound and why, for a diagnostic that
    /// has to explain a partial result to a human.
    ///
    /// Reports all three facts a caller acts on, because they are genuinely independent:
    /// which side of the interval the rows bound, whether their *positions* mean
    /// anything, and whether re-running under a larger budget returns these same rows
    /// first (the resumption property, which needs both a lower bound and order).
    pub(crate) fn describe(&self) -> String {
        let bound = match self.bound() {
            SpineClass::Certain => "a certified lower bound",
            SpineClass::Possible => "an upper bound",
            SpineClass::Unknown => "bounded on neither side",
        };
        let fidelity = match self.fidelity() {
            PrefixFidelity::Positional => "positional",
            PrefixFidelity::BagOnly => "bag-only",
        };
        let stable = if self.is_positional_prefix() {
            "; a larger budget returns these rows first"
        } else {
            ""
        };
        let barrier = match self.barrier() {
            Some(barrier) => format!("; {barrier}"),
            None => String::new(),
        };
        // Whether a restricting `Slice` was crossed on the way up is worth saying out
        // loud: it is the one thing that turns a merely bag-only bound into no bound at
        // all, so a caller reading "bag-only" and "position-selected" together knows the
        // shape of the query is what cost them the rows, not the size of the budget.
        let selected = if self.certificate.spine().under_positional_selection() {
            ", position-selected"
        } else {
            ""
        };
        format!(
            "{} tripped, leaving {} of {} row(s), {fidelity}{selected}{stable}{barrier}",
            self.tripped(),
            bound,
            self.rows.len()
        )
    }

    /// Split into the rows and the certificate describing them.
    ///
    /// The two halves travel separately in exactly one place: a parallel `UNION` branch,
    /// whose rows must be materialized against the worker's own scratch interner before
    /// that worker's context is dropped, while the certificate is composed on the main
    /// thread afterwards.
    pub(crate) fn split(self) -> (SolutionSeq<I>, Certificate) {
        (self.rows, self.certificate)
    }

    /// Rebuild this truncation after an answer-cap cut, collapsing an upper bound when
    /// removal would make it unsound.
    pub(crate) fn after_answer_cap(rows: SolutionSeq<I>, certificate: Certificate) -> Self {
        Self::new(rows, certificate.with_answer_cap_barrier())
    }
}

/// The result of evaluating one algebra node: a complete bag, or a partial one with a
/// certificate.
///
/// [`Evaluated::Complete`] is the zero-overhead path — the same `SolutionSeq` the
/// evaluator produced before this channel existed, moved through one enum discriminant.
/// An ungoverned execution produces nothing else, so it does no new work and its results
/// are byte-identical.
#[derive(Debug, Clone)]
pub(crate) enum Evaluated<I: ViewTermId> {
    /// The node's full output.
    Complete(SolutionSeq<I>),
    /// A governor tripped at or below this node; see [`Truncation`].
    Truncated(Truncation<I>),
}

impl<I: ViewTermId> Evaluated<I> {
    /// The rows this result holds, complete or not.
    pub(crate) const fn rows(&self) -> &SolutionSeq<I> {
        match self {
            Self::Complete(seq) => seq,
            Self::Truncated(truncation) => truncation.rows(),
        }
    }

    /// The complete bag, or the truncation that prevented one.
    pub(crate) fn into_complete(self) -> Result<SolutionSeq<I>, Truncation<I>> {
        match self {
            Self::Complete(seq) => Ok(seq),
            Self::Truncated(truncation) => Err(truncation),
        }
    }

    /// Whether a governor tripped at or below this node.
    pub(crate) const fn is_truncated(&self) -> bool {
        matches!(self, Self::Truncated(_))
    }
}

/// The per-node lift: absorbs each child's [`Evaluated`], composes the certificate, and
/// re-wraps the operator's own output.
///
/// An operator uses exactly three calls — [`Lift::absorb`] per child, [`Lift::is_truncated`]
/// before starting the *next* child, and [`Lift::finish`] (or [`Lift::withheld`]) once —
/// so the D4 composition rules are applied in one place rather than restated per operator.
#[derive(Debug)]
pub(crate) struct Lift<'p> {
    /// The algebra node this lift belongs to; names the barrier and supplies the edges.
    node: &'p GraphPattern,
    /// The classified edges of `node`'s children, read from the one visitor.
    edges: ChildEdges,
    /// The composed certificate of the child truncation absorbed so far, if any.
    certificate: Option<Certificate>,
    /// The best schema known for this node's output, used when no rows can be computed.
    schema: Option<Arc<VarSchema>>,
}

impl<'p> Lift<'p> {
    /// A lift for `node`, reading its child classification from
    /// [`crate::governor::soundness::child_edges`].
    pub(crate) fn at(node: &'p GraphPattern) -> Self {
        Self {
            node,
            edges: child_edges(node),
            certificate: None,
            schema: None,
        }
    }

    /// Absorb the result of this node's `ordinal`-th classified child — the ordinal at
    /// which [`crate::governor::soundness::visit_classified_children`] yields it, which
    /// is the order the operator evaluates its children in.
    ///
    /// Returns the rows the operator may compute over, or `None` when no bound would
    /// survive the computation, in which case the operator must return
    /// [`Lift::withheld`] without computing anything: an aggregate over a partial input
    /// is a different number, not a subset of the true one, and a positional selection
    /// over a sub-bag picks rows the true query never returns.
    pub(crate) fn absorb<I: ViewTermId>(
        &mut self,
        ordinal: usize,
        evaluated: Evaluated<I>,
    ) -> Option<SolutionSeq<I>> {
        match evaluated {
            Evaluated::Complete(seq) => Some(seq),
            Evaluated::Truncated(truncation) => {
                let (rows, mut certificate) = truncation.split();
                certificate.ascend(self.edges.at(ordinal), self.node);
                let barred = certificate.spine().class() == SpineClass::Unknown;
                self.schema = Some(Arc::clone(&rows.schema));
                self.certificate = Some(certificate);
                if barred { None } else { Some(rows) }
            }
        }
    }

    /// Absorb a child truncation whose rows this operator has **already** consumed.
    ///
    /// The rows-carrying [`Lift::absorb`] is the normal path; this one exists for the
    /// parallel `UNION`, where each branch's rows have to be materialized against that
    /// branch's own worker context before it is dropped, leaving only the certificate to
    /// compose once the main thread reduces the branches in source order.
    ///
    /// Answers whether a bound survives — `false` means the operator must return
    /// [`Lift::withheld`].
    pub(crate) fn absorb_certificate(&mut self, ordinal: usize, certificate: Certificate) -> bool {
        let mut certificate = certificate;
        certificate.ascend(self.edges.at(ordinal), self.node);
        let survives = certificate.spine().class() != SpineClass::Unknown;
        self.certificate = Some(certificate);
        survives
    }

    /// Whether a child has truncated.
    ///
    /// An operator checks this **before evaluating its next child** and stops if it is
    /// true. That is the lift's work bound: finishing over rows already in hand is
    /// proportional to the committed row count, while starting a fresh subtree is
    /// proportional to the dataset — and a governor that still licenses a full scan after
    /// its budget is spent bounds nothing.
    pub(crate) const fn is_truncated(&self) -> bool {
        self.certificate.is_some()
    }

    /// Wrap this node's computed `rows`.
    pub(crate) fn finish<I: ViewTermId>(self, rows: SolutionSeq<I>) -> Evaluated<I> {
        match self.certificate {
            None => Evaluated::Complete(rows),
            Some(certificate) => Evaluated::Truncated(Truncation::new(rows, certificate)),
        }
    }

    /// Finish with no rows: this node's output cannot be computed from what it holds.
    ///
    /// Reached either because an edge is opaque, or because a child truncated before a
    /// sibling this node's output depends on was ever evaluated. The empty bag is a
    /// sound lower bound in the second case and carries no claim at all in the first.
    pub(crate) fn withheld<I: ViewTermId>(self) -> Evaluated<I> {
        let schema = self
            .schema
            .clone()
            .unwrap_or_else(|| Arc::new(VarSchema::new()));
        self.finish(SolutionSeq::empty(schema))
    }

    /// The schema of the last absorbed child, for an operator that needs to build an
    /// empty result of its own shape.
    pub(crate) fn absorbed_schema(&self) -> Option<Arc<VarSchema>> {
        self.schema.clone()
    }
}

/// The one-shot cell through which a truncation **inside an expression** reaches the
/// operator that owns the expression.
///
/// `EXISTS` is evaluated from inside expression evaluation, whose result type is a term
/// or a boolean — there is no room in it for a partial bag, and widening it would put a
/// third channel on the hottest path in the evaluator for a case that can only ever
/// withhold rows. So the `EXISTS` site records the trip here and the enclosing operator
/// (`FILTER`, `BIND`, `ORDER BY`, `GROUP BY`, an `OPTIONAL` join condition) reads it once
/// its row loop is done and withholds its whole output — which is exactly what
/// [`ChildRole::Opaque`] licenses and all it licenses.
///
/// Shared through an [`Arc`], so an observation made on a forked parallel worker's
/// context reaches the parent that forked it. A worker's own context is dropped the
/// instant its closure returns, so a non-shared cell would silently lose the barrier and
/// the operator would emit rows whose `EXISTS` booleans were computed over a truncated
/// bag.
#[derive(Debug, Clone, Default)]
pub(crate) struct ExpressionBarrier(Arc<OnceLock<TrippedGovernor>>);

impl ExpressionBarrier {
    /// Record that an expression-embedded pattern truncated. Write-once: the first trip
    /// is the reported trip, matching the governor state's own latching.
    pub(crate) fn record(&self, tripped: TrippedGovernor) {
        let _ = self.0.set(tripped);
    }

    /// The trip observed inside an expression, if one was.
    pub(crate) fn observed(&self) -> Option<TrippedGovernor> {
        self.0.get().copied()
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use purrdf_core::{ResourceDimension, TermId};
    use purrdf_sparql_algebra::{
        NamedNode, NamedNodePattern, TermPattern, TriplePattern, Variable,
    };

    use super::*;
    use crate::governor::soundness::{ChildRole, walk_spine};

    /// A fuel trip, the governor these tests use whenever the identity of the governor is
    /// not what is under test.
    const FUEL: TrippedGovernor = TrippedGovernor::Budget {
        dimension: ResourceDimension::Fuel,
        limit: 8,
        consumed: 9,
    };

    fn bgp() -> GraphPattern {
        GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: TermPattern::Variable(Variable::new("s")),
                predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked(
                    "https://example.org/p",
                )),
                object: TermPattern::Variable(Variable::new("o")),
            }],
        }
    }

    fn boxed(pattern: GraphPattern) -> Box<GraphPattern> {
        Box::new(pattern)
    }

    /// One row over a one-column schema, so a test can tell "rows survived" from "rows
    /// were withheld" without building a dataset.
    fn one_row() -> SolutionSeq<TermId> {
        SolutionSeq {
            schema: Arc::new(VarSchema::from_vars([Variable::new("s")])),
            rows: vec![smallvec::smallvec![Some(
                crate::scratch::SolutionTerm::Existing(TermId::from_index(0))
            )]],
        }
    }

    /// Ascend `truncation` through the `ordinal`-th child edge of `node`, as an operator
    /// would, keeping the rows.
    fn ascend(
        node: &GraphPattern,
        ordinal: usize,
        truncation: Truncation<TermId>,
    ) -> Truncation<TermId> {
        let mut lift = Lift::at(node);
        let rows = lift.absorb(ordinal, Evaluated::Truncated(truncation));
        match rows {
            Some(rows) => match lift.finish(rows) {
                Evaluated::Truncated(truncation) => truncation,
                Evaluated::Complete(_) => panic!("absorbing a truncation must stay truncated"),
            },
            None => match lift.withheld() {
                Evaluated::Truncated(truncation) => truncation,
                Evaluated::Complete(_) => panic!("absorbing a truncation must stay truncated"),
            },
        }
    }

    #[test]
    fn a_fresh_truncation_is_a_positional_lower_bound_on_its_own_node() {
        let truncation = Truncation::origin(one_row(), FUEL);
        assert_eq!(truncation.bound(), SpineClass::Certain);
        assert_eq!(truncation.fidelity(), PrefixFidelity::Positional);
        assert!(truncation.is_positional_prefix());
        assert_eq!(truncation.tripped(), FUEL);
        assert_eq!(truncation.barrier(), None);
        assert_eq!(truncation.rows().len(), 1);
        assert!(truncation.certain_rows().is_some());
    }

    #[test]
    fn the_composed_certificate_agrees_with_the_plan_level_analysis() {
        // The evaluator composes bottom-up as it returns; `walk_spine` composes top-down
        // over the whole plan. They are the same function applied to the same edges, and
        // this pins that: for every node of a mixed plan, ascending a truncation from
        // that node to the root must land on the class the plan-level walk assigns it.
        let plan = GraphPattern::Slice {
            inner: boxed(GraphPattern::Minus {
                left: boxed(GraphPattern::Union {
                    left: boxed(bgp()),
                    right: boxed(GraphPattern::Distinct {
                        inner: boxed(bgp()),
                    }),
                }),
                right: boxed(GraphPattern::OrderBy {
                    inner: boxed(bgp()),
                    expression: vec![purrdf_sparql_algebra::OrderExpression::Asc(
                        purrdf_sparql_algebra::Expression::Variable(Variable::new("o")),
                    )],
                }),
            }),
            start: 0,
            length: Some(3),
        };

        // Every (path from the root, class) pair the plan-level analysis assigns.
        let mut expected: Vec<(Vec<usize>, SpineClass)> = Vec::new();
        collect_paths(&plan, &mut Vec::new(), &mut expected);

        for (path, class) in expected {
            // Ascend a fresh truncation from the node at `path` back to the root,
            // crossing the same edges in reverse.
            let mut truncation = Truncation::origin(one_row(), FUEL);
            for depth in (0..path.len()).rev() {
                let parent = node_at(&plan, &path[..depth]);
                truncation = ascend(parent, path[depth], truncation);
            }
            assert_eq!(
                truncation.bound(),
                class,
                "path {path:?} disagreed with the plan-level analysis"
            );
        }
    }

    /// The node reached by following `path` (child ordinals) from `root`.
    fn node_at<'a>(root: &'a GraphPattern, path: &[usize]) -> &'a GraphPattern {
        let mut node = root;
        for &step in path {
            let mut children: Vec<&GraphPattern> = Vec::new();
            crate::governor::soundness::visit_classified_children(node, &mut |child, _edge| {
                children.push(child);
                false
            });
            node = children[step];
        }
        node
    }

    /// Record every node's path and the class the plan-level walk gives it.
    fn collect_paths(
        root: &GraphPattern,
        path: &mut Vec<usize>,
        out: &mut Vec<(Vec<usize>, SpineClass)>,
    ) {
        // `walk_spine` yields nodes without their paths, so the paths are enumerated here
        // and each one's class is read back from the walk by position.
        let mut classes: Vec<(usize, SpineClass)> = Vec::new();
        let mut index = 0_usize;
        walk_spine(root, &mut |_node, context, _depth| {
            classes.push((index, context.class()));
            index += 1;
        });
        // Re-walk in the same pre-order, recording paths.
        fn descend(
            node: &GraphPattern,
            path: &mut Vec<usize>,
            index: &mut usize,
            classes: &[(usize, SpineClass)],
            out: &mut Vec<(Vec<usize>, SpineClass)>,
        ) {
            out.push((path.clone(), classes[*index].1));
            *index += 1;
            let mut children: Vec<&GraphPattern> = Vec::new();
            crate::governor::soundness::visit_classified_children(node, &mut |child, _edge| {
                children.push(child);
                false
            });
            for (ordinal, child) in children.into_iter().enumerate() {
                path.push(ordinal);
                descend(child, path, index, classes, out);
                path.pop();
            }
        }
        let mut index = 0_usize;
        descend(root, path, &mut index, &classes, out);
    }

    #[test]
    fn truncated_child_under_a_monotone_parent_still_certifies_a_lower_bound() {
        let plan = GraphPattern::Distinct {
            inner: boxed(bgp()),
        };
        let lifted = ascend(&plan, 0, Truncation::origin(one_row(), FUEL));
        assert_eq!(lifted.bound(), SpineClass::Certain);
        assert_eq!(lifted.fidelity(), PrefixFidelity::Positional);
        assert_eq!(lifted.rows().len(), 1, "a lower bound carries its rows");
        assert_eq!(lifted.barrier(), None);
        assert_eq!(lifted.tripped(), FUEL, "the governor propagates unchanged");
    }

    #[test]
    fn truncated_right_arm_of_minus_yields_an_upper_bound() {
        let plan = GraphPattern::Minus {
            left: boxed(bgp()),
            right: boxed(bgp()),
        };
        let lifted = ascend(&plan, 1, Truncation::origin(one_row(), FUEL));
        assert_eq!(
            lifted.bound(),
            SpineClass::Possible,
            "subtracting less leaves a superset of the true answer"
        );
        assert_eq!(lifted.fidelity(), PrefixFidelity::BagOnly);
        assert!(!lifted.is_positional_prefix());
        assert!(
            lifted.certain_rows().is_none(),
            "an upper bound must never be readable as answers"
        );
        assert_eq!(lifted.rows().len(), 1, "but it is not a black hole either");
    }

    #[test]
    fn double_antitone_composes_back_to_a_lower_bound() {
        // MINUS(A, MINUS(B, C)) truncated at C: truncating C subtracts less from B, so
        // the inner MINUS grows; a larger subtrahend removes more from A, so the root
        // shrinks — a sound LOWER bound. Antitone composed with antitone is monotone.
        let inner = GraphPattern::Minus {
            left: boxed(bgp()),
            right: boxed(bgp()),
        };
        let outer = GraphPattern::Minus {
            left: boxed(bgp()),
            right: boxed(inner.clone()),
        };

        let at_c = ascend(&inner, 1, Truncation::origin(one_row(), FUEL));
        assert_eq!(at_c.bound(), SpineClass::Possible);

        let at_root = ascend(&outer, 1, at_c);
        assert_eq!(
            at_root.bound(),
            SpineClass::Certain,
            "collapsing antitone-over-antitone to Unknown would discard true information"
        );
        assert_eq!(
            at_root.fidelity(),
            PrefixFidelity::BagOnly,
            "an antitone edge yields a bag bound, never a positional one"
        );
        assert!(!at_root.is_positional_prefix());
        assert_eq!(at_root.rows().len(), 1);
    }

    #[test]
    fn opaque_barrier_carries_no_rows() {
        let plan = GraphPattern::Group {
            inner: boxed(bgp()),
            variables: vec![Variable::new("s")],
            aggregates: Vec::new(),
        };
        let lifted = ascend(&plan, 0, Truncation::origin(one_row(), FUEL));
        assert_eq!(lifted.bound(), SpineClass::Unknown);
        assert!(
            lifted.rows().is_empty(),
            "a bag bounded on neither side must not reach a caller"
        );
        assert_eq!(
            lifted.barrier().map(NonMonotoneBarrier::operator),
            Some("Group"),
            "the barrier names the operator that withheld the rows"
        );
        assert!(lifted.certain_rows().is_none());
        assert_eq!(lifted.tripped(), FUEL);

        // An expression-embedded EXISTS is the same shape, barred at its own node.
        let filter = GraphPattern::Filter {
            expr: purrdf_sparql_algebra::Expression::Exists(boxed(bgp())),
            inner: boxed(bgp()),
        };
        let barred = Truncation::<TermId>::barred_at(&filter, FUEL, Arc::new(VarSchema::new()));
        assert_eq!(barred.bound(), SpineClass::Unknown);
        assert!(barred.rows().is_empty());
        assert_eq!(
            barred.barrier().map(NonMonotoneBarrier::operator),
            Some("Filter")
        );
    }

    #[test]
    fn bag_only_truncation_below_a_restricting_slice_carries_no_rows() {
        // UNION emits left rows then right rows, so truncating the LEFT arm removes rows
        // from the middle of the concatenation: a sound sub-bag, but not a prefix. A
        // restricting LIMIT above selects BY POSITION, so it can pick rows the true query
        // never returns — and nothing may cross.
        let union = GraphPattern::Union {
            left: boxed(bgp()),
            right: boxed(bgp()),
        };
        let sliced = GraphPattern::Slice {
            inner: boxed(union.clone()),
            start: 0,
            length: Some(1),
        };

        let at_union = ascend(&union, 0, Truncation::origin(one_row(), FUEL));
        assert_eq!(at_union.bound(), SpineClass::Certain);
        assert_eq!(at_union.fidelity(), PrefixFidelity::BagOnly);
        assert_eq!(at_union.rows().len(), 1);

        let at_slice = ascend(&sliced, 0, at_union);
        assert_eq!(at_slice.bound(), SpineClass::Unknown);
        assert!(at_slice.rows().is_empty());
        assert_eq!(
            at_slice.barrier().map(NonMonotoneBarrier::operator),
            Some("Slice")
        );

        // An identity slice cannot select a different row set, so it imposes nothing.
        let identity = GraphPattern::Slice {
            inner: boxed(union.clone()),
            start: 0,
            length: None,
        };
        let unaffected = ascend(
            &identity,
            0,
            ascend(&union, 0, Truncation::origin(one_row(), FUEL)),
        );
        assert_eq!(unaffected.bound(), SpineClass::Certain);
        assert_eq!(unaffected.rows().len(), 1);
    }

    #[test]
    fn a_complete_child_leaves_the_lift_complete() {
        let plan = GraphPattern::Distinct {
            inner: boxed(bgp()),
        };
        let mut lift = Lift::at(&plan);
        let rows = lift
            .absorb(0, Evaluated::Complete(one_row()))
            .expect("a complete child always yields rows");
        assert!(!lift.is_truncated());
        assert!(matches!(lift.finish(rows), Evaluated::Complete(_)));
    }

    #[test]
    fn an_out_of_range_child_ordinal_is_classified_opaque() {
        // The conservative answer: a caller that asks about a child a node does not have
        // is handed the classification that withholds every row, never the one that
        // licenses them as answers.
        let plan = GraphPattern::Distinct {
            inner: boxed(bgp()),
        };
        let edges = child_edges(&plan);
        assert_eq!(edges.at(0).role, ChildRole::PrefixMonotone);
        assert_eq!(edges.at(7).role, ChildRole::Opaque);
    }

    #[test]
    fn the_expression_barrier_latches_and_is_shared_across_clones() {
        let barrier = ExpressionBarrier::default();
        assert_eq!(barrier.observed(), None);
        let forked = barrier.clone();
        forked.record(FUEL);
        assert_eq!(
            barrier.observed(),
            Some(FUEL),
            "a worker's observation must reach the context that forked it"
        );
        forked.record(TrippedGovernor::Stopped {
            cause: purrdf_core::StopCause::Cancelled,
        });
        assert_eq!(
            barrier.observed(),
            Some(FUEL),
            "write-once: the first trip is the reported trip"
        );
    }
}
