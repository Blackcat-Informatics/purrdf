// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The one exhaustive algebra visitor, and the two answers it computes: whether a
//! truncated bag at a node still certifies a bound on the root answer, and whether the
//! operational answer cap may be pushed down to that node.
//!
//! # Why one visitor
//!
//! Three analyses need the same structural walk over [`GraphPattern`] — the fork-join
//! parallel-safety gate ([`crate::parallel`]), the answer-completeness certificate, and
//! the answer-cap pushdown licence — and all three need it to descend into
//! [`Expression`]s as well, because `FILTER`/`BIND`/`ORDER BY`/aggregates all carry
//! patterns inside expressions (`EXISTS`) and builtins inside expressions. Written three
//! times, a new algebra variant would need three independent edits and only some of them
//! would be found. The walk is therefore defined exactly once, here, in
//! [`visit_pattern_parts`] and [`visit_expression_parts`]; [`crate::parallel`]'s
//! unsafe-builtin search is expressed in terms of it.
//!
//! # The formalism: prefix-monotonicity, not subset-monotonicity
//!
//! Evaluation is fully materialized and its parallel reduce is order-stable, so a
//! truncated bag is a **prefix of the true bag in evaluation order**, not merely a subset
//! of it. That distinction is load-bearing rather than pedantic:
//!
//! ```text
//! true bag   = [a, b]
//! partial    = [b]        -- a subset, but NOT a prefix
//! LIMIT 1    -> partial gives [b], true gives [a]
//! ```
//!
//! `Slice(0, 1)` is not subset-monotone, so a subset certificate would license emitting
//! `b` for a query whose only answer is `a`. It **is** prefix-preserving, so the prefix
//! certificate licenses nothing false. Every classification below is stated in terms of
//! prefixes for exactly this reason.
//!
//! # Two axes, because two different things can be lost
//!
//! A truncation can cost the *positional* relation while keeping the *multiset* relation.
//! `ORDER BY` is the obvious case — sorting a prefix of the input yields a sub-bag of the
//! sorted output, in a different order — but it is not the only one. Under a hash join
//! whose output is left-major, truncating the **right** input removes rows from the
//! middle of the output, not from its end; under `UNION`, whose output is the left rows
//! followed by the right rows, truncating the **left** input does the same. Those
//! operators still deliver a sound multiset lower bound, and they still must not be
//! trusted positionally.
//!
//! So each child edge carries both a [`ChildRole`] (which side of the answer interval the
//! child's truncation bounds) and a [`PrefixFidelity`] (whether the positional prefix
//! relation survives it), and [`SpineContext`] tracks [`SpineClass`] and
//! [`OrderCertainty`] separately.
//!
//! # The interaction that is easiest to get wrong
//!
//! A node that selects its output **by position** — a restricting `Slice` — needs its
//! input to be a genuine prefix. Given only a sub-bag it can select rows the true query
//! never returns:
//!
//! ```text
//! ORDER BY ?x LIMIT 1  over  true bag [a, b]
//! partial sub-bag [b] -> LIMIT 1 gives [b]; the true answer is [a]
//! ```
//!
//! So [`SpineContext`] carries a bit recording that a restricting `Slice` lies above on
//! this spine, and any edge below it that is only [`PrefixFidelity::BagOnly`] collapses to
//! [`SpineClass::Unknown`]. This is what makes `ORDER BY` + `LIMIT` a top-*k* problem with
//! no certified lower bound, and it applies identically to a `MINUS` right arm under a
//! `LIMIT` (an upper bound is not a prefix either, so slicing it selects the wrong rows).
//!
//! # `EXISTS` is opaque, deliberately
//!
//! An `EXISTS` pattern reached through an expression is classified
//! [`ChildRole::Opaque`], and so is a `NOT EXISTS` one — `NOT EXISTS` is
//! `Not(Exists(..))` in this algebra, so the two are not even distinguishable without
//! tracking negation polarity through `!`, `IF`, `COALESCE`, `IN`, and user-function
//! bodies.
//!
//! The reasoning for refusing both: truncating an `EXISTS` inner bag can only turn its
//! boolean from true to false, so a `FILTER EXISTS` drops rows the true query keeps —
//! from the **middle** of its output, which is a sub-bag and not a prefix. Truncating a
//! `NOT EXISTS` inner bag turns its boolean from false to true, which **fabricates** rows
//! the true query never returns; that is not a bound in either direction. A single
//! classification must cover both, and only [`ChildRole::Opaque`] is sound for both.
//! Withholding rows costs utility; admitting a fabricated row costs correctness, and the
//! certificate exists to make that trade in exactly one direction.
//!
//! Note the scope: it is truncation **inside** the `EXISTS` subtree that is opaque. A
//! `FILTER`'s own data child stays [`ChildRole::PrefixMonotone`], because the filter
//! predicate is then evaluated in full over a prefix and yields a prefix.
//!
//! # Enforcement
//!
//! **Every `match` over [`GraphPattern`], [`Expression`], [`OrderExpression`], and
//! [`AggregateExpression`] in this module is wildcard-free — no `_ =>` arm, and every
//! field of every variant is named.** A new algebra variant, or a new field on an
//! existing one, must therefore be a compile error (E0004) here rather than silently
//! inheriting the most permissive classification, which is
//! [`SpineClass::Certain`] — i.e. rather than silently licensing partial rows to cross as
//! answers. This mirrors the enforcement idiom `purrdf_sparql_conformance`'s OWL-RL
//! scoreboard uses for the same reason: an unclassified case counted as a neighbour
//! prints a figure that is not what it says it is.

// The certificate is a pure function of the algebra and holds no evaluator types, which
// is what lets it be exercised against hand-built plans (see this module's tests) rather
// than only through whole-query evaluation — a certificate that can only be observed
// through the thing it licenses is a certificate nobody can falsify. The price is that
// the compiler sees the classification surface as unreferenced from inside this crate
// until the evaluator's descent and the planner's cap pushdown read it; the parallel
// gate already reads the visitor half. The allow is module-scoped rather than per-item
// because it applies to the whole classification surface for one reason, not to a
// handful of items for several.
#![allow(dead_code)]

use purrdf_sparql_algebra::{
    AggregateExpression, Expression, Function, GraphPattern, OrderExpression,
};

// ---------------------------------------------------------------------------
// Per-child transfer functions
// ---------------------------------------------------------------------------

/// How a child's truncation propagates into this node's output — which side of the
/// answer interval it bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ChildRole {
    /// A prefix of the child's output yields a prefix of this node's output, so the
    /// bound the child carries survives unchanged.
    PrefixMonotone,
    /// Truncating this child bounds this node's output from the **other** side: fewer
    /// rows in the child mean more rows out of the node, so a lower bound below becomes
    /// an upper bound here and vice versa.
    Antitone,
    /// Neither bound survives. Absorbing: nothing below an opaque edge is certifiable.
    Opaque,
}

/// Whether the *positional* prefix relation survives a child's truncation into this
/// node's output, or only the multiset relation does.
///
/// This is a separate question from [`ChildRole`]. An operator can deliver a perfectly
/// sound multiset bound while reordering or interleaving, and the difference only becomes
/// observable under a positional selection (a restricting `Slice`) further up the spine —
/// which is precisely where it becomes an unsoundness rather than an inconvenience.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum PrefixFidelity {
    /// A prefix of the child's output yields a prefix of this node's output.
    Positional,
    /// A prefix of the child's output yields only a sub-bag (or super-bag, under
    /// [`ChildRole::Antitone`]) of this node's output: rows are removed from, or added
    /// to, the middle rather than the end.
    BagOnly,
}

/// One edge from a node to one of its children: the transfer function that edge applies
/// to a truncation below it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ChildEdge {
    /// Which side of the answer interval a truncation below this edge bounds.
    pub(crate) role: ChildRole,
    /// Whether the positional prefix relation survives this edge.
    pub(crate) fidelity: PrefixFidelity,
    /// Whether the parent selects its output from this child **by position**, so that
    /// everything below this edge needs a genuine prefix rather than merely a sub-bag.
    ///
    /// True exactly for the child of a `Slice` that actually restricts. An identity
    /// slice (`OFFSET 0` with no `LIMIT`) cannot select a different row set, so it does
    /// not impose the requirement and does not set this.
    pub(crate) selects_by_position: bool,
}

impl ChildEdge {
    /// The edge of an operator that both preserves prefixes and imposes nothing:
    /// truncation below it keeps whatever bound it had, positionally.
    const MONOTONE: Self = Self {
        role: ChildRole::PrefixMonotone,
        fidelity: PrefixFidelity::Positional,
        selects_by_position: false,
    };

    /// [`Self::MONOTONE`], but the parent reorders or interleaves this child's rows into
    /// its output, so only the multiset relation survives.
    const MONOTONE_BAG: Self = Self {
        role: ChildRole::PrefixMonotone,
        fidelity: PrefixFidelity::BagOnly,
        selects_by_position: false,
    };

    /// A subtracted / negated position. Antitone edges are always
    /// [`PrefixFidelity::BagOnly`]: a super-bag of the true output has extra rows
    /// wherever the removed ones were, never only at the end.
    const ANTITONE: Self = Self {
        role: ChildRole::Antitone,
        fidelity: PrefixFidelity::BagOnly,
        selects_by_position: false,
    };

    /// A position from which no bound propagates at all.
    const OPAQUE: Self = Self {
        role: ChildRole::Opaque,
        fidelity: PrefixFidelity::BagOnly,
        selects_by_position: false,
    };

    /// The child of a restricting `Slice`.
    ///
    /// # Proof sketch that `Slice` is prefix-monotone in both `start` and `length`
    ///
    /// Let `t` be the true input sequence and `p` a prefix of it, and write
    /// `slice(x) = x[start..][..length]` (clamped at the end of `x`). Because `p` is a
    /// prefix, `p[i] == t[i]` for every `i < p.len()`, so `slice(p)` and `slice(t)` agree
    /// element-for-element at every index both define, and `slice(p)` simply runs out
    /// first. Hence `slice(p)` is a prefix of `slice(t)` — for any `start`, any `length`,
    /// and including the degenerate cases where `p` is shorter than `start` (both empty
    /// or `slice(p)` empty).
    ///
    /// It is emphatically **not** subset-monotone; see this module's header for the
    /// counterexample. That is why the edge sets [`ChildEdge::selects_by_position`]: the
    /// proof consumed the hypothesis that the input is a prefix, so anything below that
    /// only delivers a sub-bag invalidates it.
    const SLICED: Self = Self {
        role: ChildRole::PrefixMonotone,
        fidelity: PrefixFidelity::Positional,
        selects_by_position: true,
    };
}

// ---------------------------------------------------------------------------
// Spine composition
// ---------------------------------------------------------------------------

/// Which bound on the **root** answer a truncated bag at this node still certifies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum SpineClass {
    /// Rows here are certifiable as a lower bound on the root answer: every row that
    /// reaches the root from this position is an answer.
    Certain,
    /// Rows here bound the root answer from above: the true answer is contained in what
    /// reaches the root, so a row absent from it is definitively not an answer.
    Possible,
    /// Neither bound survives to the root. Absorbing.
    Unknown,
}

impl SpineClass {
    /// Compose this class with the transfer function of the edge being descended.
    ///
    /// Written as nine explicit cases rather than with a catch-all so that the interval
    /// algebra is legible at the point it is applied:
    ///
    /// - `Certain ∘ PrefixMonotone = Certain`, `Possible ∘ PrefixMonotone = Possible` —
    ///   a monotone edge transports whichever bound it is given.
    /// - `Certain ∘ Antitone = Possible`, and **`Possible ∘ Antitone = Certain`** — the
    ///   classical interval rule: antitone composed with antitone is monotone. This is
    ///   not a curiosity; `MINUS(A, MINUS(B, C))` truncated at `C` really does certify a
    ///   lower bound at the root, and collapsing it to [`SpineClass::Unknown`] would
    ///   discard true information the engine holds.
    /// - Everything touching [`SpineClass::Unknown`] or [`ChildRole::Opaque`] is
    ///   [`SpineClass::Unknown`]: both are absorbing, because no later operator can
    ///   restore a bound that was never established.
    const fn compose(self, role: ChildRole) -> Self {
        match (self, role) {
            (Self::Certain, ChildRole::PrefixMonotone) => Self::Certain,
            (Self::Certain, ChildRole::Antitone) => Self::Possible,
            (Self::Possible, ChildRole::PrefixMonotone) => Self::Possible,
            (Self::Possible, ChildRole::Antitone) => Self::Certain,
            (Self::Unknown, ChildRole::PrefixMonotone | ChildRole::Antitone)
            | (Self::Certain | Self::Possible | Self::Unknown, ChildRole::Opaque) => Self::Unknown,
        }
    }
}

/// Whether rows certified at this node also certify their **order** at the root, or only
/// their membership.
///
/// A bare `ORDER BY` does not change the row multiset, only its order, so a trip beneath
/// one still certifies a sound bound as a bag. Saying so requires a second axis: the
/// alternative — calling `ORDER BY` non-monotone — would void the certificate for every
/// sorted query for no safety gain at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum OrderCertainty {
    /// The certified rows appear at the root in the true answer's order, and form a
    /// genuine prefix of it.
    Ordered,
    /// The certified rows are a sound bound as a multiset; their positions at the root
    /// are not the true answer's positions.
    Unordered,
}

/// The composed certificate for one node, carried down the plan during evaluation.
///
/// This is a "descend and compose" value rather than a side map keyed by node address:
/// the evaluator already walks the plan, so it threads one of these and updates it per
/// child. That keeps the analysis O(1) per node with no allocation, and — unlike an
/// address-keyed map — it cannot go stale against a substituted or rewritten subtree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct SpineContext {
    /// Which bound a truncation at this node certifies at the root.
    class: SpineClass,
    /// Whether that bound is positional as well as multiset.
    order: OrderCertainty,
    /// Whether a restricting `Slice` lies above this node on the spine, so that a
    /// later loss of positional fidelity is fatal rather than merely limiting.
    under_positional_selection: bool,
}

impl SpineContext {
    /// The context at the root of a plan: everything is certifiable, in order, and no
    /// positional selection is in force yet.
    pub(crate) const ROOT: Self = Self {
        class: SpineClass::Certain,
        order: OrderCertainty::Ordered,
        under_positional_selection: false,
    };

    /// Which bound a truncation at this node certifies at the root.
    pub(crate) const fn class(self) -> SpineClass {
        self.class
    }

    /// Whether the certified bound is positional as well as multiset.
    pub(crate) const fn order(self) -> OrderCertainty {
        self.order
    }

    /// Whether a restricting `Slice` lies above this node.
    pub(crate) const fn under_positional_selection(self) -> bool {
        self.under_positional_selection
    }

    /// Whether the operational answer cap may be pushed down to this node.
    ///
    /// The licence is exactly "this node's rows are a certified lower bound on the root
    /// answer **and** they are the root answer's first rows in order" — because that, and
    /// only that, is what makes stopping the scan here produce the same first *n* answers
    /// the full scan would have produced. It is therefore the **same computation** as the
    /// soundness certificate, read for a different purpose, not a second analysis: a cap
    /// pushed to a node that merely bounds the answer as a bag would return *some n*
    /// answers rather than *the first n*, and pushed to an antitone or opaque position it
    /// would change the answer outright.
    pub(crate) const fn admits_cap_pushdown(self) -> bool {
        matches!(
            (self.class, self.order),
            (SpineClass::Certain, OrderCertainty::Ordered)
        )
    }

    /// The context for a child reached through `edge`.
    ///
    /// Three things happen, in this order:
    ///
    /// 1. If a restricting `Slice` is already above and this edge only preserves the
    ///    multiset relation, the result is [`SpineClass::Unknown`]: the slice would be
    ///    selecting by position from something that is not a prefix, so it can pick rows
    ///    the true query never returns. This is checked against the state **above** the
    ///    edge, so the slice's own child — which is positional — is not caught by it.
    /// 2. Otherwise the class composes through [`SpineClass::compose`].
    /// 3. Order certainty is monotone: once lost it is never regained, because no
    ///    operator can restore positions that a reordering below it already destroyed.
    pub(crate) const fn descend(self, edge: ChildEdge) -> Self {
        let bag_only = matches!(edge.fidelity, PrefixFidelity::BagOnly);
        let class = if self.under_positional_selection && bag_only {
            SpineClass::Unknown
        } else {
            self.class.compose(edge.role)
        };
        let order = if bag_only {
            OrderCertainty::Unordered
        } else {
            self.order
        };
        Self {
            class,
            order,
            under_positional_selection: self.under_positional_selection || edge.selects_by_position,
        }
    }

    /// Build a context directly, for tests that need to enumerate the licence's domain
    /// rather than reach each point through a plan.
    #[cfg(test)]
    const fn from_parts(
        class: SpineClass,
        order: OrderCertainty,
        under_positional_selection: bool,
    ) -> Self {
        Self {
            class,
            order,
            under_positional_selection,
        }
    }
}

// ---------------------------------------------------------------------------
// The visitor
// ---------------------------------------------------------------------------

/// One structural part of a single algebra node: a child pattern with the transfer
/// function of the edge reaching it, or an expression attached to the node.
#[derive(Debug)]
pub(crate) enum PatternPart<'a> {
    /// A directly-nested sub-pattern, and how a truncation there propagates.
    Child(&'a GraphPattern, ChildEdge),
    /// An expression this node evaluates: a `FILTER` predicate, a `BIND` expression, an
    /// `OPTIONAL` join condition, an `ORDER BY` sort key, or an aggregate's argument.
    Expression(&'a Expression),
}

/// One structural part of a single expression node.
#[derive(Debug)]
pub(crate) enum ExpressionPart<'a> {
    /// A directly-nested sub-expression.
    Sub(&'a Expression),
    /// The function a call names. Yielded before the call's arguments.
    Call(&'a Function),
    /// The pattern inside an `EXISTS` (or, as `Not(Exists(..))`, a `NOT EXISTS`).
    Exists(&'a GraphPattern),
}

/// Visit every structural part of `pattern` — its direct children, then its attached
/// expressions — stopping as soon as `visit` returns `true`. Returns whether it stopped.
///
/// **Shallow by design**: this yields one node's parts and does not recurse. Recursion
/// belongs to the consumer, because the three consumers recurse differently — the
/// parallel-safety search short-circuits on the first unsafe builtin, the certificate
/// composes a context on the way down, and the cap-pushdown licence reads that same
/// context. Sharing the *decomposition* is what makes a new algebra variant one compile
/// error instead of three silent omissions; sharing the traversal strategy as well would
/// force all three into the least useful of the three shapes.
///
/// The match below is wildcard-free and names every field of every variant. See this
/// module's header for why that is a requirement and not a style choice.
pub(crate) fn visit_pattern_parts<'a, F>(pattern: &'a GraphPattern, visit: &mut F) -> bool
where
    F: FnMut(PatternPart<'a>) -> bool,
{
    match pattern {
        // Leaves. A truncation cannot happen "below" them; they are where truncation
        // originates.
        GraphPattern::Bgp { patterns: _ } => false,
        GraphPattern::Path {
            subject: _,
            path: _,
            object: _,
        } => false,
        GraphPattern::Values {
            variables: _,
            bindings: _,
        } => false,

        // A hash join's output is left-major (each left row's matches, in left order), so
        // a prefix of the left input yields a prefix of the output, while a prefix of the
        // right input removes rows from the middle of every left row's block — a sound
        // sub-bag, not a prefix.
        GraphPattern::Join { left, right } => {
            visit(PatternPart::Child(left, ChildEdge::MONOTONE))
                || visit(PatternPart::Child(right, ChildEdge::MONOTONE_BAG))
        }
        // `LATERAL` evaluates its right side once per left row and emits left-major, so
        // it has exactly the join's shape.
        GraphPattern::Lateral { left, right } => {
            visit(PatternPart::Child(left, ChildEdge::MONOTONE))
                || visit(PatternPart::Child(right, ChildEdge::MONOTONE_BAG))
        }
        // `UNION` is a multiset concatenation, left rows then right rows: truncating the
        // RIGHT side removes rows from the end, so that side is positional, while
        // truncating the LEFT side removes rows from the middle of the concatenation.
        GraphPattern::Union { left, right } => {
            visit(PatternPart::Child(left, ChildEdge::MONOTONE_BAG))
                || visit(PatternPart::Child(right, ChildEdge::MONOTONE))
        }
        // Truncating the optional side can only cause MORE left rows to find no match and
        // be padded with unbound, so it bounds the output from above.
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => {
            visit(PatternPart::Child(left, ChildEdge::MONOTONE))
                || visit(PatternPart::Child(right, ChildEdge::ANTITONE))
                || expression
                    .as_ref()
                    .is_some_and(|e| visit(PatternPart::Expression(e)))
        }
        // Truncating the subtracted side removes rows from the set being subtracted, so
        // fewer left rows are eliminated: an upper bound.
        GraphPattern::Minus { left, right } => {
            visit(PatternPart::Child(left, ChildEdge::MONOTONE))
                || visit(PatternPart::Child(right, ChildEdge::ANTITONE))
        }

        // Filtering a prefix yields a prefix of the filtered output: the predicate is
        // evaluated in full over each row, and no surviving row moves.
        GraphPattern::Filter { expr, inner } => {
            visit(PatternPart::Child(inner, ChildEdge::MONOTONE))
                || visit(PatternPart::Expression(expr))
        }
        // `BIND` adds a column; it neither drops nor reorders rows.
        GraphPattern::Extend {
            inner,
            variable: _,
            expression,
        } => {
            visit(PatternPart::Child(inner, ChildEdge::MONOTONE))
                || visit(PatternPart::Expression(expression))
        }
        GraphPattern::Graph { name: _, inner } => {
            visit(PatternPart::Child(inner, ChildEdge::MONOTONE))
        }
        GraphPattern::Project {
            inner,
            variables: _,
        } => visit(PatternPart::Child(inner, ChildEdge::MONOTONE)),
        // `S ⊑ T ⇒ dedup S ⊑ dedup T`: de-duplication keeps each value's first
        // occurrence, and a prefix's first occurrences are the true bag's first
        // occurrences. Classifying `DISTINCT` non-monotone would void every
        // `SELECT DISTINCT` for no safety gain.
        GraphPattern::Distinct { inner } => visit(PatternPart::Child(inner, ChildEdge::MONOTONE)),
        // `REDUCED` may or may not drop a duplicate, but the decision is taken from the
        // rows already seen, so it too depends only on the prefix.
        GraphPattern::Reduced { inner } => visit(PatternPart::Child(inner, ChildEdge::MONOTONE)),
        // A remote sub-query is opaque as a *value*, but structurally its rows flow
        // straight through.
        GraphPattern::Service {
            name: _,
            inner,
            silent: _,
        } => visit(PatternPart::Child(inner, ChildEdge::MONOTONE)),

        // See [`ChildEdge::SLICED`] for the proof sketch, and this module's header for
        // why the positional-selection bit it sets is load-bearing.
        GraphPattern::Slice {
            inner,
            start,
            length,
        } => {
            let edge = if *start == 0 && length.is_none() {
                // An identity slice selects nothing; it cannot pick a different row set,
                // so it imposes no positional requirement on what lies below.
                ChildEdge::MONOTONE
            } else {
                ChildEdge::SLICED
            };
            visit(PatternPart::Child(inner, edge))
        }
        // Sorting a prefix yields a sub-bag of the sorted output, in a different order:
        // sound as a bag, worthless as a position.
        GraphPattern::OrderBy { inner, expression } => {
            if visit(PatternPart::Child(inner, ChildEdge::MONOTONE_BAG)) {
                return true;
            }
            expression.iter().any(|key| match key {
                OrderExpression::Asc(e) | OrderExpression::Desc(e) => {
                    visit(PatternPart::Expression(e))
                }
            })
        }
        // An aggregate computed over a truncated input is a *different number*, not a
        // subset of the true one: `COUNT` under-counts, `SUM` under-sums, `AVG` is wrong
        // in an unsigned direction. No row-level bound survives, so the edge is opaque.
        GraphPattern::Group {
            inner,
            variables: _,
            aggregates,
        } => {
            if visit(PatternPart::Child(inner, ChildEdge::OPAQUE)) {
                return true;
            }
            aggregates.iter().any(|(_, aggregate)| match aggregate {
                AggregateExpression::CountStar { distinct: _ } => false,
                AggregateExpression::FunctionCall {
                    function: _,
                    expression,
                    distinct: _,
                } => visit(PatternPart::Expression(expression)),
            })
        }
    }
}

/// Visit every structural part of `expr`, stopping as soon as `visit` returns `true`.
/// Returns whether it stopped. Shallow, for the same reason [`visit_pattern_parts`] is.
///
/// A [`ExpressionPart::Call`] is yielded before that call's arguments, so a consumer that
/// short-circuits on the function itself never walks arguments it does not need.
///
/// The match below is wildcard-free. See this module's header.
pub(crate) fn visit_expression_parts<'a, F>(expr: &'a Expression, visit: &mut F) -> bool
where
    F: FnMut(ExpressionPart<'a>) -> bool,
{
    match expr {
        Expression::NamedNode(_)
        | Expression::Literal(_)
        | Expression::Variable(_)
        | Expression::Bound(_) => false,
        Expression::Or(a, b)
        | Expression::And(a, b)
        | Expression::Equal(a, b)
        | Expression::SameTerm(a, b)
        | Expression::Greater(a, b)
        | Expression::GreaterOrEqual(a, b)
        | Expression::Less(a, b)
        | Expression::LessOrEqual(a, b)
        | Expression::Add(a, b)
        | Expression::Subtract(a, b)
        | Expression::Multiply(a, b)
        | Expression::Divide(a, b) => {
            visit(ExpressionPart::Sub(a)) || visit(ExpressionPart::Sub(b))
        }
        Expression::UnaryPlus(a) | Expression::UnaryMinus(a) | Expression::Not(a) => {
            visit(ExpressionPart::Sub(a))
        }
        Expression::In(head, list) => {
            visit(ExpressionPart::Sub(head)) || list.iter().any(|e| visit(ExpressionPart::Sub(e)))
        }
        Expression::If(a, b, c) => {
            visit(ExpressionPart::Sub(a))
                || visit(ExpressionPart::Sub(b))
                || visit(ExpressionPart::Sub(c))
        }
        Expression::Coalesce(list) => list.iter().any(|e| visit(ExpressionPart::Sub(e))),
        Expression::FunctionCall(function, arguments) => {
            visit(ExpressionPart::Call(function))
                || arguments.iter().any(|e| visit(ExpressionPart::Sub(e)))
        }
        Expression::Exists(pattern) => visit(ExpressionPart::Exists(pattern)),
    }
}

/// Visit every `EXISTS` pattern reachable from `expr` without leaving the expression —
/// i.e. without descending into a pattern. Stops as soon as `visit` returns `true`.
fn visit_exists_patterns<'a, F>(expr: &'a Expression, visit: &mut F) -> bool
where
    F: FnMut(&'a GraphPattern) -> bool,
{
    visit_expression_parts(expr, &mut |part| match part {
        ExpressionPart::Sub(sub) => visit_exists_patterns(sub, visit),
        ExpressionPart::Call(_) => false,
        ExpressionPart::Exists(pattern) => visit(pattern),
    })
}

/// Visit every pattern that is a child of `pattern` for classification purposes: its
/// direct algebraic children first, then every `EXISTS` pattern reachable through an
/// expression attached to it. Stops as soon as `visit` returns `true`; returns whether
/// it stopped.
///
/// This is the certificate's view of the tree, and it is built from the same two
/// primitives everything else uses — nothing here re-matches on an algebra type.
pub(crate) fn visit_classified_children<'a, F>(pattern: &'a GraphPattern, visit: &mut F) -> bool
where
    F: FnMut(&'a GraphPattern, ChildEdge) -> bool,
{
    visit_pattern_parts(pattern, &mut |part| match part {
        PatternPart::Child(child, edge) => visit(child, edge),
        PatternPart::Expression(expr) => {
            visit_exists_patterns(expr, &mut |inner| visit(inner, ChildEdge::OPAQUE))
        }
    })
}

/// Walk the whole plan rooted at `root`, invoking `visit` with every node and the
/// [`SpineContext`] that holds at it.
///
/// The entry point for asking either question — "what does a truncation here certify?"
/// and "may the answer cap be pushed here?" — about any node of a plan. The evaluator
/// does not need this: it carries a [`SpineContext`] and calls
/// [`SpineContext::descend`] per child as it goes, which is the same composition with no
/// second traversal. This function exists for callers that hold a plan but are not
/// evaluating it, such as planner-side pushdown and the tests below.
pub(crate) fn walk_spine<F>(root: &GraphPattern, visit: &mut F)
where
    F: FnMut(&GraphPattern, SpineContext),
{
    fn descend<F>(node: &GraphPattern, context: SpineContext, visit: &mut F)
    where
        F: FnMut(&GraphPattern, SpineContext),
    {
        visit(node, context);
        visit_classified_children(node, &mut |child, edge| {
            descend(child, context.descend(edge), visit);
            false
        });
    }

    descend(root, SpineContext::ROOT, visit);
}

/// The index of `pattern`'s variant in [`PATTERN_LABELS`].
///
/// The match is wildcard-free, so a new [`GraphPattern`] variant is a compile error here.
/// A variant added without extending [`PATTERN_LABELS`] then panics on the out-of-range
/// index the moment [`pattern_label`] is called for it, which the coverage test below
/// does for every variant — so the label table cannot silently fall behind the algebra
/// either.
const fn pattern_label_index(pattern: &GraphPattern) -> usize {
    match pattern {
        GraphPattern::Bgp { patterns: _ } => 0,
        GraphPattern::Path {
            subject: _,
            path: _,
            object: _,
        } => 1,
        GraphPattern::Join { left: _, right: _ } => 2,
        GraphPattern::LeftJoin {
            left: _,
            right: _,
            expression: _,
        } => 3,
        GraphPattern::Lateral { left: _, right: _ } => 4,
        GraphPattern::Filter { expr: _, inner: _ } => 5,
        GraphPattern::Union { left: _, right: _ } => 6,
        GraphPattern::Graph { name: _, inner: _ } => 7,
        GraphPattern::Extend {
            inner: _,
            variable: _,
            expression: _,
        } => 8,
        GraphPattern::Minus { left: _, right: _ } => 9,
        GraphPattern::Service {
            name: _,
            inner: _,
            silent: _,
        } => 10,
        GraphPattern::Values {
            variables: _,
            bindings: _,
        } => 11,
        GraphPattern::OrderBy {
            inner: _,
            expression: _,
        } => 12,
        GraphPattern::Project {
            inner: _,
            variables: _,
        } => 13,
        GraphPattern::Distinct { inner: _ } => 14,
        GraphPattern::Reduced { inner: _ } => 15,
        GraphPattern::Slice {
            inner: _,
            start: _,
            length: _,
        } => 16,
        GraphPattern::Group {
            inner: _,
            variables: _,
            aggregates: _,
        } => 17,
    }
}

/// Every [`GraphPattern`] variant's stable label, indexed by [`pattern_label_index`].
pub(crate) const PATTERN_LABELS: [&str; 18] = [
    "Bgp", "Path", "Join", "LeftJoin", "Lateral", "Filter", "Union", "Graph", "Extend", "Minus",
    "Service", "Values", "OrderBy", "Project", "Distinct", "Reduced", "Slice", "Group",
];

/// `pattern`'s variant label, for diagnostics and for the coverage test.
pub(crate) fn pattern_label(pattern: &GraphPattern) -> &'static str {
    PATTERN_LABELS[pattern_label_index(pattern)]
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use pretty_assertions::assert_eq;
    use purrdf_sparql_algebra::{
        AggregateFunction, GroundTerm, Literal, NamedNode, NamedNodePattern,
        PropertyPathExpression, TermPattern, TriplePattern, Variable,
    };

    use super::*;

    // ---- fixtures ---------------------------------------------------------

    /// A one-triple BGP over `example.org`, the standard leaf for these plans.
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

    /// A distinguishable second leaf, so a test can tell two arms apart by shape.
    fn other_bgp() -> GraphPattern {
        GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: TermPattern::Variable(Variable::new("s")),
                predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked(
                    "https://example.org/q",
                )),
                object: TermPattern::Variable(Variable::new("o")),
            }],
        }
    }

    fn boxed(pattern: GraphPattern) -> Box<GraphPattern> {
        Box::new(pattern)
    }

    /// The context at the node reached by following `path` — a list of child ordinals in
    /// [`visit_classified_children`] order — down from the root of `root`.
    fn context_at(root: &GraphPattern, path: &[usize]) -> SpineContext {
        let mut node = root;
        let mut context = SpineContext::ROOT;
        for &step in path {
            let mut children: Vec<(&GraphPattern, ChildEdge)> = Vec::new();
            visit_classified_children(node, &mut |child, edge| {
                children.push((child, edge));
                false
            });
            let (child, edge) = children[step];
            node = child;
            context = context.descend(edge);
        }
        context
    }

    // ---- prefix-monotone operators certify --------------------------------

    #[test]
    fn slice_limit_is_prefix_monotone_and_certifies() {
        // `LIMIT 1` over a pattern. The certificate is prefix-monotonicity, and this is
        // the operator that proves subset-monotonicity would have been the WRONG
        // property to certify:
        //
        //     true bag = [a, b]
        //     a partial that is a SUBSET but not a PREFIX: [b]
        //     LIMIT 1 over the partial -> [b]
        //     LIMIT 1 over the true bag -> [a]
        //     [b] is not a subset of [a]
        //
        // So a subset certificate would have licensed emitting `b` for a query whose
        // only answer is `a`. A PREFIX partial ([a]) slices to [a], which is exactly the
        // true answer — which is why `Slice` is classified prefix-monotone and why the
        // positional-selection bit exists to keep the prefix hypothesis true below it.
        let plan = GraphPattern::Slice {
            inner: boxed(bgp()),
            start: 0,
            length: Some(1),
        };
        let inner = context_at(&plan, &[0]);
        assert_eq!(inner.class(), SpineClass::Certain);
        assert_eq!(inner.order(), OrderCertainty::Ordered);
        assert!(inner.under_positional_selection());
        assert!(inner.admits_cap_pushdown());
    }

    #[test]
    fn slice_offset_is_prefix_monotone_and_certifies() {
        // `OFFSET 5`: a prefix `p` of the true input agrees with it element-for-element
        // at every index both define, so `p[5..]` is a prefix of `true[5..]` — including
        // when `p` is shorter than 5 and the result is empty.
        let plan = GraphPattern::Slice {
            inner: boxed(bgp()),
            start: 5,
            length: None,
        };
        let inner = context_at(&plan, &[0]);
        assert_eq!(inner.class(), SpineClass::Certain);
        assert_eq!(inner.order(), OrderCertainty::Ordered);
        assert!(inner.under_positional_selection());
    }

    #[test]
    fn distinct_is_prefix_monotone_and_certifies() {
        let plan = GraphPattern::Distinct {
            inner: boxed(bgp()),
        };
        let inner = context_at(&plan, &[0]);
        assert_eq!(inner.class(), SpineClass::Certain);
        assert_eq!(inner.order(), OrderCertainty::Ordered);
        assert!(
            inner.admits_cap_pushdown(),
            "classifying DISTINCT non-monotone would void every SELECT DISTINCT for no \
             safety gain"
        );

        // `REDUCED` is the same shape.
        let reduced = GraphPattern::Reduced {
            inner: boxed(bgp()),
        };
        assert_eq!(context_at(&reduced, &[0]).class(), SpineClass::Certain);
    }

    // ---- the order axis ---------------------------------------------------

    #[test]
    fn bare_order_by_certifies_as_a_bag_but_not_as_an_order() {
        let plan = GraphPattern::OrderBy {
            inner: boxed(bgp()),
            expression: vec![OrderExpression::Asc(Expression::Variable(Variable::new(
                "o",
            )))],
        };
        let inner = context_at(&plan, &[0]);
        assert_eq!(
            inner.class(),
            SpineClass::Certain,
            "a bare ORDER BY does not change the row multiset, only its order, so a \
             truncation beneath it is still a sound lower bound as a bag"
        );
        assert_eq!(inner.order(), OrderCertainty::Unordered);
        assert!(
            !inner.admits_cap_pushdown(),
            "a cap pushed beneath a sort would return some n rows, not the first n"
        );
    }

    #[test]
    fn order_by_with_slice_certifies_no_lower_bound() {
        // ORDER BY + LIMIT is a top-k problem. A sub-bag of the input can sort to rows
        // the true top-k never contains, so nothing beneath is certifiable at all.
        let plan = GraphPattern::Slice {
            inner: boxed(GraphPattern::OrderBy {
                inner: boxed(bgp()),
                expression: vec![OrderExpression::Asc(Expression::Variable(Variable::new(
                    "o",
                )))],
            }),
            start: 0,
            length: Some(10),
        };

        let order_by = context_at(&plan, &[0]);
        assert_eq!(order_by.class(), SpineClass::Certain);
        assert!(order_by.under_positional_selection());

        let beneath = context_at(&plan, &[0, 0]);
        assert_eq!(
            beneath.class(),
            SpineClass::Unknown,
            "a Slice selecting by position from something that is only a sub-bag can \
             pick rows the true query never returns"
        );
        assert!(!beneath.admits_cap_pushdown());

        // Without the slice the same sort certifies a bag bound, so the Unknown above is
        // the interaction and not the sort alone.
        let unsliced = GraphPattern::OrderBy {
            inner: boxed(bgp()),
            expression: vec![OrderExpression::Asc(Expression::Variable(Variable::new(
                "o",
            )))],
        };
        assert_eq!(context_at(&unsliced, &[0]).class(), SpineClass::Certain);

        // And an identity slice imposes nothing, because it cannot select a different
        // row set in the first place.
        let identity_sliced = GraphPattern::Slice {
            inner: boxed(GraphPattern::OrderBy {
                inner: boxed(bgp()),
                expression: vec![OrderExpression::Asc(Expression::Variable(Variable::new(
                    "o",
                )))],
            }),
            start: 0,
            length: None,
        };
        assert_eq!(
            context_at(&identity_sliced, &[0, 0]).class(),
            SpineClass::Certain
        );
    }

    // ---- antitone positions ----------------------------------------------

    #[test]
    fn minus_right_arm_truncation_yields_an_upper_bound_not_unknown() {
        let plan = GraphPattern::Minus {
            left: boxed(bgp()),
            right: boxed(other_bgp()),
        };

        let left = context_at(&plan, &[0]);
        assert_eq!(left.class(), SpineClass::Certain);
        assert_eq!(left.order(), OrderCertainty::Ordered);

        let right = context_at(&plan, &[1]);
        assert_eq!(
            right.class(),
            SpineClass::Possible,
            "truncating the subtracted side eliminates fewer left rows, so the output \
             contains the true answer: an upper bound, not a black hole"
        );
        assert_eq!(right.order(), OrderCertainty::Unordered);
        assert!(!right.admits_cap_pushdown());
    }

    #[test]
    fn left_join_right_arm_truncation_yields_an_upper_bound_not_unknown() {
        let plan = GraphPattern::LeftJoin {
            left: boxed(bgp()),
            right: boxed(other_bgp()),
            expression: None,
        };

        assert_eq!(context_at(&plan, &[0]).class(), SpineClass::Certain);

        let right = context_at(&plan, &[1]);
        assert_eq!(
            right.class(),
            SpineClass::Possible,
            "truncating the optional side can only pad MORE left rows with unbound, so \
             it bounds the output from above"
        );
        assert_eq!(right.order(), OrderCertainty::Unordered);
    }

    #[test]
    fn double_antitone_composes_back_to_certain() {
        // MINUS(A, MINUS(B, C)) truncated at C. Truncating C subtracts less from B, so
        // MINUS(B, C) grows; a larger subtrahend removes more from A, so the root output
        // shrinks — a sound LOWER bound. Antitone composed with antitone is monotone,
        // and collapsing this to Unknown would discard real information.
        let plan = GraphPattern::Minus {
            left: boxed(bgp()),
            right: boxed(GraphPattern::Minus {
                left: boxed(other_bgp()),
                right: boxed(bgp()),
            }),
        };

        let inner_minus = context_at(&plan, &[1]);
        assert_eq!(inner_minus.class(), SpineClass::Possible);

        let c = context_at(&plan, &[1, 1]);
        assert_eq!(c.class(), SpineClass::Certain);
        assert_eq!(
            c.order(),
            OrderCertainty::Unordered,
            "an antitone edge yields a bag bound, never a positional one"
        );
        assert!(
            !c.admits_cap_pushdown(),
            "a bag bound is not the first n answers, so it licenses no cap pushdown"
        );

        // B, reached through one antitone edge then one monotone one, stays an upper
        // bound: the interval algebra does not collapse on either side.
        assert_eq!(context_at(&plan, &[1, 0]).class(), SpineClass::Possible);
    }

    // ---- opaque positions -------------------------------------------------

    #[test]
    fn group_is_opaque_and_absorbs() {
        let plan = GraphPattern::Group {
            inner: boxed(bgp()),
            variables: vec![Variable::new("s")],
            aggregates: vec![(
                Variable::new("n"),
                AggregateExpression::CountStar { distinct: false },
            )],
        };
        let inner = context_at(&plan, &[0]);
        assert_eq!(
            inner.class(),
            SpineClass::Unknown,
            "an aggregate over a truncated input is a different number, not a subset of \
             the true one"
        );
        assert_eq!(inner.order(), OrderCertainty::Unordered);
        assert!(!inner.admits_cap_pushdown());

        // Absorbing: composing anything further with Unknown stays Unknown.
        for role in [
            ChildRole::PrefixMonotone,
            ChildRole::Antitone,
            ChildRole::Opaque,
        ] {
            assert_eq!(SpineClass::Unknown.compose(role), SpineClass::Unknown);
        }
        // And so is Opaque, from every starting class.
        for class in [
            SpineClass::Certain,
            SpineClass::Possible,
            SpineClass::Unknown,
        ] {
            assert_eq!(class.compose(ChildRole::Opaque), SpineClass::Unknown);
        }
    }

    #[test]
    fn monotone_operator_under_a_non_monotone_parent_is_still_barred() {
        // FILTER, DISTINCT and Slice are each individually prefix-monotone, but sitting
        // under a GROUP they certify nothing: the barrier is a property of the SPINE,
        // not of the node.
        let plan = GraphPattern::Group {
            inner: boxed(GraphPattern::Filter {
                expr: Expression::Bound(Variable::new("o")),
                inner: boxed(GraphPattern::Distinct {
                    inner: boxed(GraphPattern::Slice {
                        inner: boxed(bgp()),
                        start: 0,
                        length: Some(3),
                    }),
                }),
            }),
            variables: vec![Variable::new("s")],
            aggregates: Vec::new(),
        };

        for path in [&[0][..], &[0, 0][..], &[0, 0, 0][..], &[0, 0, 0, 0][..]] {
            let context = context_at(&plan, path);
            assert_eq!(
                context.class(),
                SpineClass::Unknown,
                "path {path:?} is beneath an opaque edge"
            );
            assert!(!context.admits_cap_pushdown());
        }
    }

    // ---- expressions ------------------------------------------------------

    #[test]
    fn exists_inside_a_filter_expression_is_found_by_the_walk() {
        // The EXISTS pattern is buried under NOT, IF and a function call, so only a walk
        // that descends into expression structure finds it at all.
        let exists = Expression::Exists(boxed(other_bgp()));
        let buried = Expression::Not(Box::new(Expression::If(
            Box::new(Expression::Bound(Variable::new("o"))),
            Box::new(exists),
            Box::new(Expression::Literal(Literal::new_simple("no"))),
        )));
        let plan = GraphPattern::Filter {
            expr: buried,
            inner: boxed(bgp()),
        };

        let mut children: Vec<(&'static str, ChildEdge)> = Vec::new();
        visit_classified_children(&plan, &mut |child, edge| {
            children.push((pattern_label(child), edge));
            false
        });
        assert_eq!(children.len(), 2, "the data child and the EXISTS pattern");
        assert_eq!(children[0].0, "Bgp");
        assert_eq!(children[0].1.role, ChildRole::PrefixMonotone);
        assert_eq!(children[1].0, "Bgp");
        assert_eq!(
            children[1].1.role,
            ChildRole::Opaque,
            "a truncated EXISTS inner drops rows from the middle of a FILTER's output, \
             and a truncated NOT EXISTS inner fabricates rows outright; NOT EXISTS is \
             Not(Exists(..)) here, so one classification must cover both and only Opaque \
             is sound for both"
        );

        // The FILTER's own data child is unaffected: the predicate is still evaluated in
        // full over every row that reaches it.
        assert_eq!(context_at(&plan, &[0]).class(), SpineClass::Certain);
        assert_eq!(context_at(&plan, &[1]).class(), SpineClass::Unknown);

        // The same holds for an EXISTS in a BIND, an OPTIONAL join condition, an
        // ORDER BY key and an aggregate argument — every expression-bearing position.
        let bind = GraphPattern::Extend {
            inner: boxed(bgp()),
            variable: Variable::new("b"),
            expression: Expression::Exists(boxed(other_bgp())),
        };
        assert_eq!(context_at(&bind, &[1]).class(), SpineClass::Unknown);

        let optional = GraphPattern::LeftJoin {
            left: boxed(bgp()),
            right: boxed(other_bgp()),
            expression: Some(Expression::Exists(boxed(bgp()))),
        };
        assert_eq!(context_at(&optional, &[2]).class(), SpineClass::Unknown);

        let sorted = GraphPattern::OrderBy {
            inner: boxed(bgp()),
            expression: vec![OrderExpression::Desc(Expression::Exists(
                boxed(other_bgp()),
            ))],
        };
        assert_eq!(context_at(&sorted, &[1]).class(), SpineClass::Unknown);

        let grouped = GraphPattern::Group {
            inner: boxed(bgp()),
            variables: Vec::new(),
            aggregates: vec![(
                Variable::new("n"),
                AggregateExpression::FunctionCall {
                    function: AggregateFunction::Count,
                    expression: Box::new(Expression::Exists(boxed(other_bgp()))),
                    distinct: false,
                },
            )],
        };
        assert_eq!(context_at(&grouped, &[1]).class(), SpineClass::Unknown);
    }

    // ---- the pushdown licence --------------------------------------------

    #[test]
    fn cap_pushdown_is_licensed_exactly_when_certain_and_ordered() {
        for class in [
            SpineClass::Certain,
            SpineClass::Possible,
            SpineClass::Unknown,
        ] {
            for order in [OrderCertainty::Ordered, OrderCertainty::Unordered] {
                for sliced in [false, true] {
                    let context = SpineContext::from_parts(class, order, sliced);
                    let expected = class == SpineClass::Certain && order == OrderCertainty::Ordered;
                    assert_eq!(
                        context.admits_cap_pushdown(),
                        expected,
                        "class={class:?} order={order:?} under_slice={sliced}"
                    );
                }
            }
        }

        // The motivating plan: `SELECT ?s WHERE { ?s ?p ?o }`. Charged only at the root
        // the cap protects nothing, so the licence must reach the leaf.
        let scan = GraphPattern::Project {
            inner: boxed(GraphPattern::Distinct {
                inner: boxed(bgp()),
            }),
            variables: vec![Variable::new("s")],
        };
        let mut licensed = 0_usize;
        walk_spine(&scan, &mut |_node, context| {
            assert!(context.admits_cap_pushdown());
            licensed += 1;
        });
        assert_eq!(licensed, 3, "Project, Distinct and the Bgp leaf");

        // The licence is the certificate, read for a different purpose: every node it
        // admits is Certain and Ordered, everywhere, on an arbitrary plan.
        let mixed = all_variants_plan();
        walk_spine(&mixed, &mut |node, context| {
            assert_eq!(
                context.admits_cap_pushdown(),
                context.class() == SpineClass::Certain
                    && context.order() == OrderCertainty::Ordered,
                "{} disagreed with its own certificate",
                pattern_label(node)
            );
        });
    }

    // ---- exhaustiveness ---------------------------------------------------

    /// A plan containing every [`GraphPattern`] variant at least once.
    fn all_variants_plan() -> GraphPattern {
        let path = GraphPattern::Path {
            subject: TermPattern::Variable(Variable::new("s")),
            path: PropertyPathExpression::NamedNode(NamedNode::new_unchecked(
                "https://example.org/p",
            )),
            object: TermPattern::Variable(Variable::new("o")),
        };
        let values = GraphPattern::Values {
            variables: vec![Variable::new("v")],
            bindings: vec![vec![Some(GroundTerm::NamedNode(NamedNode::new_unchecked(
                "https://example.org/v",
            )))]],
        };
        let service = GraphPattern::Service {
            name: NamedNodePattern::NamedNode(NamedNode::new_unchecked(
                "https://example.org/sparql",
            )),
            inner: boxed(bgp()),
            silent: false,
        };
        let graph = GraphPattern::Graph {
            name: NamedNodePattern::NamedNode(NamedNode::new_unchecked("https://example.org/g")),
            inner: boxed(path),
        };
        let joined = GraphPattern::Join {
            left: boxed(graph),
            right: boxed(values),
        };
        let lateral = GraphPattern::Lateral {
            left: boxed(joined),
            right: boxed(service),
        };
        let optional = GraphPattern::LeftJoin {
            left: boxed(lateral),
            right: boxed(other_bgp()),
            expression: Some(Expression::Bound(Variable::new("o"))),
        };
        let united = GraphPattern::Union {
            left: boxed(optional),
            right: boxed(bgp()),
        };
        let minus = GraphPattern::Minus {
            left: boxed(united),
            right: boxed(other_bgp()),
        };
        let filtered = GraphPattern::Filter {
            expr: Expression::Bound(Variable::new("s")),
            inner: boxed(minus),
        };
        let extended = GraphPattern::Extend {
            inner: boxed(filtered),
            variable: Variable::new("b"),
            expression: Expression::Literal(Literal::new_simple("x")),
        };
        let grouped = GraphPattern::Group {
            inner: boxed(extended),
            variables: vec![Variable::new("s")],
            aggregates: vec![(
                Variable::new("n"),
                AggregateExpression::CountStar { distinct: false },
            )],
        };
        let ordered = GraphPattern::OrderBy {
            inner: boxed(grouped),
            expression: vec![OrderExpression::Asc(Expression::Variable(Variable::new(
                "n",
            )))],
        };
        let reduced = GraphPattern::Reduced {
            inner: boxed(ordered),
        };
        let distinct = GraphPattern::Distinct {
            inner: boxed(reduced),
        };
        let sliced = GraphPattern::Slice {
            inner: boxed(distinct),
            start: 1,
            length: Some(2),
        };
        GraphPattern::Project {
            inner: boxed(sliced),
            variables: vec![Variable::new("s")],
        }
    }

    #[test]
    fn every_graph_pattern_variant_is_reached_and_classified() {
        let plan = all_variants_plan();

        let mut seen: BTreeSet<&'static str> = BTreeSet::new();
        walk_spine(&plan, &mut |node, context| {
            // Reaching a node at all means the walk descended into it, and every context
            // is one of the three classes — there is no fourth, unclassified state.
            assert!(matches!(
                context.class(),
                SpineClass::Certain | SpineClass::Possible | SpineClass::Unknown
            ));
            seen.insert(pattern_label(node));
        });

        let expected: BTreeSet<&'static str> = PATTERN_LABELS.iter().copied().collect();
        assert_eq!(
            expected.len(),
            PATTERN_LABELS.len(),
            "the label table must not contain duplicates"
        );
        assert_eq!(
            seen, expected,
            "every algebra variant must be reachable by the one visitor and carry a \
             classification; an unclassified variant would inherit the most permissive \
             answer, which is 'these partial rows are answers'"
        );
    }

    #[test]
    fn every_variant_has_a_child_classification_and_leaves_have_none() {
        let plan = all_variants_plan();
        let mut with_children: BTreeSet<&'static str> = BTreeSet::new();
        let mut leaves: BTreeSet<&'static str> = BTreeSet::new();

        walk_spine(&plan, &mut |node, _context| {
            let mut count = 0_usize;
            visit_classified_children(node, &mut |_child, _edge| {
                count += 1;
                false
            });
            if count == 0 {
                leaves.insert(pattern_label(node));
            } else {
                with_children.insert(pattern_label(node));
            }
        });

        assert_eq!(
            leaves,
            ["Bgp", "Path", "Values"].into_iter().collect(),
            "only BGP, property paths and inline VALUES are leaves; anything else \
             reporting no children means its subtree escaped classification"
        );
        assert_eq!(with_children.len(), PATTERN_LABELS.len() - leaves.len());
    }

    #[test]
    fn the_walk_stops_when_the_visitor_says_stop() {
        // The short-circuit is what makes the parallel-safety consumer's behaviour
        // identical to the hand-written `||` chain it replaces.
        let plan = GraphPattern::Join {
            left: boxed(bgp()),
            right: boxed(other_bgp()),
        };
        let mut visited = 0_usize;
        let stopped = visit_pattern_parts(&plan, &mut |_part| {
            visited += 1;
            true
        });
        assert!(stopped);
        assert_eq!(visited, 1, "the second child must not be visited");

        let expr = Expression::Coalesce(vec![
            Expression::Bound(Variable::new("a")),
            Expression::Bound(Variable::new("b")),
            Expression::Bound(Variable::new("c")),
        ]);
        let mut parts = 0_usize;
        assert!(visit_expression_parts(&expr, &mut |_part| {
            parts += 1;
            parts == 2
        }));
        assert_eq!(parts, 2);
    }

    #[test]
    fn union_and_join_lose_positional_fidelity_on_opposite_arms() {
        // UNION emits left rows then right rows, so truncating the RIGHT arm removes
        // rows from the END (a genuine prefix) while truncating the LEFT arm removes
        // them from the middle of the concatenation.
        let union = GraphPattern::Union {
            left: boxed(bgp()),
            right: boxed(other_bgp()),
        };
        assert_eq!(context_at(&union, &[0]).order(), OrderCertainty::Unordered);
        assert_eq!(context_at(&union, &[1]).order(), OrderCertainty::Ordered);

        // A hash join emits left-major, so it is the other way round.
        let join = GraphPattern::Join {
            left: boxed(bgp()),
            right: boxed(other_bgp()),
        };
        assert_eq!(context_at(&join, &[0]).order(), OrderCertainty::Ordered);
        assert_eq!(context_at(&join, &[1]).order(), OrderCertainty::Unordered);

        // Both arms still certify a lower bound as a bag.
        assert_eq!(context_at(&union, &[0]).class(), SpineClass::Certain);
        assert_eq!(context_at(&join, &[1]).class(), SpineClass::Certain);

        // Under a restricting LIMIT, the bag-only arms lose even that: the slice would
        // be selecting by position from something that is not a prefix.
        let limited = GraphPattern::Slice {
            inner: boxed(join),
            start: 0,
            length: Some(5),
        };
        assert_eq!(context_at(&limited, &[0, 0]).class(), SpineClass::Certain);
        assert_eq!(context_at(&limited, &[0, 1]).class(), SpineClass::Unknown);
    }
}
