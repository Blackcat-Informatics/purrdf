// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Existential Normal Form (ENF): the rewrite laws applied to every `EXISTS`/`NOT
//! EXISTS` inner pattern before either evaluation strategy — the memoized probe or
//! the per-row definition — runs over it.
//!
//! # The one-definition theorem
//!
//! `exists(X, μ)` has exactly one semantics:
//!
//! ```text
//! exists(X, μ) ⟺ eval(D(G), Replace(PrjMap(X), μ)) is non-empty
//! ```
//!
//! where `Replace` is Values Insertion at a `Bgp`/`Path`/`Graph(Var, ·)` site (join
//! the row in as a one-row `VALUES` table rather than a syntactic constant rewrite —
//! total over every RDF 1.2 term kind, including blank nodes and quoted triples,
//! because it reaches the dataset through the same evaluation path real `VALUES`
//! data uses) and `PrjMap` is realized by `Project`-boundary narrowing (μ is
//! restricted to a `Project` node's own variable list before it is injected below
//! that node — the one scope boundary the surface language has; see
//! [`crate::expr::SubstitutionRow::narrow_to`]).
//!
//! This engine carries exactly two IMPLEMENTATIONS of that one definition:
//!
//! 1. **The definition itself** — per-row substitution via
//!    [`crate::binop::eval_correlated`], which builds `Replace(PrjMap(X), μ)`
//!    explicitly for the current row and evaluates it. Always correct, for any `X`.
//! 2. **The memoized probe** — evaluate `X` exactly ONCE, unconstrained, build an
//!    index over the columns it shares with the outer schema, and existence-probe
//!    each row's μ against that index (`crate::binop::probe_has_match`). Correct
//!    ONLY where [`crate::governor::soundness::probe_admissible`] proves it
//!    equivalent to the definition for every μ the site can see.
//!
//! Both `crate::expr::eval_correlated` and `crate::governor::soundness`'s
//! probe-admissibility predicate cite this same theorem rather than restating it —
//! see their doc comments.
//!
//! # Records
//!
//! * **`toMultiSet` structural mootness.** SEP-0007 states `Replace` in terms of
//!   `toMultiSet(μ)`; this engine's algebra has no pattern/query type split for that
//!   conversion to bridge — a solution row already IS the multiset unit `Replace`
//!   joins in (a one-row `VALUES` table), so `Replace` lands directly below whatever
//!   modifiers wrap the leaf it targets, with nothing extra to construct.
//! * **The `HAVING` position.** A current-row variable appearing in a `HAVING`
//!   clause of an `EXISTS`'s inner sub-`SELECT` is OUTSIDE SEP-0007's stated
//!   coverage (SEP-0007 only specifies `Replace` for `Bgp`/`Path`/`Graph`
//!   Values-Insertion sites and ordinary expression positions). PurRDF follows the
//!   literal `PrjMap` reading: a sub-`SELECT`'s `HAVING` filters that sub-`SELECT`'s
//!   OWN scope only, exactly like every other expression inside it — no special
//!   correlation channel is invented for it beyond what `Replace`/`PrjMap` already
//!   provide.
//! * **Governed truncation.** A governor-truncated inner bag is never a sound
//!   answer to `exists(X, μ)` in either direction (see
//!   [`crate::governor::soundness`]'s module doc on why `EXISTS` is classified
//!   [`crate::governor::soundness::ChildRole::Opaque`]): both evaluation strategies
//!   refuse to memoize or answer from a truncated inner, recording the trip on the
//!   expression barrier instead — see [`crate::expr::exists`]'s doc.
//! * **Truncated-at-one-witness is complete for emptiness.** The definition path's
//!   first-witness wrap (below) truncates the inner's row COUNT but never its
//!   EMPTINESS: "does this bag have a first row" is exactly what `Slice{0, Some(1)}`
//!   answers completely, even though the bag's full contents are not computed. This
//!   is unrelated to — and not weakened by — the governed-truncation refusal above,
//!   which is about a governor cutting evaluation short of even that first witness
//!   under an exhausted budget, a distinct (and distinctly refused) condition.
//! * **The SHACL pre-binding fork.** `crate::substitute::apply_shacl_prebinding`
//!   (`substitute_pattern_impl` in that module — not this crate's
//!   `crate::expr::substitute_pattern_impl`, a different, unrelated walk with a
//!   coincidentally similar name) deliberately diverges from THIS module's
//!   `Replace`/Values-Insertion walk in three ways, none of them an oversight:
//!   1. **No `Project`-boundary narrowing.** A SHACL `$this` (or another
//!      pre-bound variable) must reach an UNPROJECTED scope inside a nested
//!      sub-`SELECT` — the shapes language's focus-node binding is not subject to
//!      SPARQL's own scope rule the way an `EXISTS` correlation is, so
//!      `apply_shacl_prebinding` rewrites `Expression::Variable`/`Expression::Bound`
//!      unconditionally through every nested pattern, `Project` included.
//!   2. **Single query-level injection, not per-row.** A pre-binding substitutes
//!      ONE caller-supplied value into the whole query ONCE, before any row is
//!      evaluated — there is no "current row" to restrict against, unlike
//!      `Replace`, which runs once per outer row inside a live evaluation.
//!   3. **Literal property-function-argument substitution.** SHACL pre-binding
//!      rewrites a `PropertyFunction` argument's `TermPattern` directly (an
//!      IRI/literal constant swap), where `Replace`'s Values-Insertion walk never
//!      touches `PropertyFunction` argument vectors at all — a relation's argument
//!      is an invocation input the evaluator reads from the row, not a join key a
//!      `VALUES` table can supply (see `crate::expr::substitute_term_pattern`'s
//!      doc).
//!
//! # Existential Normal Form itself (Part A)
//!
//! Exactly one bit per row is observed under [`Expression::Exists`][exists]: whether
//! the inner pattern's evaluation is empty. Four rewrite laws exploit that: each
//! replaces a node with something PROVABLY EMPTINESS-EQUIVALENT but cheaper (or, for
//! law 4b, decides the answer outright without evaluating anything). They apply
//! along the **emptiness-observed spine** — the chain from the inner's own root
//! through every [`GraphPattern::OrderBy`]/[`GraphPattern::Distinct`]/
//! [`GraphPattern::Reduced`]/an identity-or-length-preserving
//! [`GraphPattern::Slice`]/[`GraphPattern::Project`] wrapper, and into BOTH branches
//! of a [`GraphPattern::Union`] (a union is empty iff both branches are) — and
//! **never** through a [`GraphPattern::Join`]/[`GraphPattern::Filter`]/
//! [`GraphPattern::Extend`]/[`GraphPattern::Graph`]/[`GraphPattern::Minus`]/
//! [`GraphPattern::LeftJoin`] operand: those consume a ROW SET (their own semantics
//! read more than "is it empty" from their child), so rewriting past them would
//! change what THEY compute, not merely how emptiness is decided.
//!
//! [exists]: purrdf_sparql_algebra::Expression::Exists
//!
//! ## Law 1 — `LeftJoin(A, B, c) → A` on the spine
//!
//! **THE F2 FIX BY LAW.** An `OPTIONAL` (`LeftJoin`) emits AT LEAST ONE row per left
//! row, for every left row, unconditionally: either the padded left row alone (no
//! compatible right row, or the join condition rejected every candidate) or one row
//! per compatible match. It never REMOVES a left row. So `LeftJoin(A, B, c)` is empty
//! **iff** `A` is empty, regardless of `B` or `c` — the whole right side and its join
//! condition are irrelevant to the one bit `EXISTS` reads. Replacing the node with
//! `A` alone preserves emptiness for every μ and drops the correlation `B`/`c` may
//! carry — the shape a bare `OPTIONAL` at the top of an `EXISTS` inner has always
//! (wrongly) been treated as correlated by, before this law existed.
//!
//! ## Law 2 — `OrderBy(P) → P` on the spine
//!
//! Sorting is a permutation, never a filter: `OrderBy(P)` and `P` have the same
//! multiset of rows, so they are empty under exactly the same condition. The sort
//! keys are never evaluated at all once emptiness is the only question asked.
//!
//! ## Law 3 — `Distinct(P) → P`, `Reduced(P) → P` on the spine (no `Slice(start>0)` above)
//!
//! De-duplication (`DISTINCT`) and its permissive cousin (`REDUCED`) can only ever
//! REMOVE rows relative to `P`, never add one — and they remove a row only when an
//! EARLIER-appearing row already carries the same value, so `P` empty implies
//! `Distinct(P)`/`Reduced(P)` empty and conversely `P` non-empty always leaves at
//! least the first row's value present after dedup. The two are therefore
//! empty under exactly the same condition as `P`.
//!
//! The "no `Slice(start>0)` above" qualifier in the source rule is automatically
//! satisfied by this law's own scope: a restricting-offset `Slice` is not one of the
//! spine's transparent wrappers (law 4 handles only `start == 0`), so the top-down
//! walk below never descends PAST one to reach a `Distinct`/`Reduced` in the first
//! place — there is no separate flag to thread.
//!
//! ## Law 4 — `Slice` on the spine
//!
//! * **4a**: `Slice(0, len ≥ 1)(P) → P`. An offset-zero slice with room for at least
//!   one row drops rows only from the END of `P`'s bag (past the first `len`), never
//!   the front, so `P` non-empty always leaves row zero inside the slice's window —
//!   the slice is empty exactly when `P` is.
//! * **4b**: `Slice(_, Some(0))(P) → ⊥` (the whole `EXISTS` folds to constant
//!   `false`). A zero-length slice is empty FOR EVERY `P` and EVERY μ — it needs no
//!   evaluation at all, so the fold is represented directly: [`normalize`] returns
//!   [`Enf::FoldedEmpty`] rather than a pattern to evaluate, and
//!   [`crate::expr::exists`] answers `false` (making `NOT EXISTS`, `Not(Exists(..))`
//!   in this algebra, answer `true`) without touching the dataset.
//!
//! A `Slice(start > 0, _)` is NOT one of the transparent wrappers: an offset can
//! discard every row `P` produces (when `P`'s bag is shorter than `start`) even
//! though `P` itself is non-empty, so slicing past an offset is not
//! emptiness-equivalent to `P` and the walk stops there, keeping the `Slice` node
//! as-is.
//!
//! ## Fixpoint, determinism, idempotence
//!
//! [`normalize`] recurses on every law that erases a wrapper, so a chain of several
//! spine wrappers (e.g. `OrderBy(Distinct(LeftJoin(A, B, c)))`) collapses in one call
//! to `A` (further normalized itself, in case `A` is ALSO spine-shaped). The walk is
//! a pure function of the input tree — no clock, no RNG, no iteration-order
//! dependence — so it is deterministic and, applied to its own output, a no-op
//! (idempotent): a normalized tree exposes no further spine wrapper for the walk to
//! erase.
//!
//! # Prepare-seam choice
//!
//! [`normalize`] is invoked lazily, the first time each distinct `EXISTS`/`NOT
//! EXISTS` AST node is reached during an evaluation, and its result is cached for
//! the remainder of that evaluation (`crate::eval::EvalCtx`'s per-query cache
//! discipline, exactly like `exists_inner_cache`'s lifecycle) — NOT at
//! [`crate::engine::PlanCache`] construction, even though that IS a genuine
//! prepare-time seam for the common case (parsed query text reaching evaluation
//! through [`crate::engine::NativeSparqlEngine::query`] and its siblings). The
//! reason: `PlanCache` is only ONE of several ways a [`GraphPattern`] reaches
//! evaluation in this crate. SHACL-SPARQL pre-binding, the entailment chase's
//! rewritten plans, `LATERAL`'s and `EXISTS`'s own per-row substituted temporaries,
//! and the majority of this crate's unit tests all construct or receive a
//! [`purrdf_sparql_algebra::Query`]/[`GraphPattern`] directly, bypassing
//! `PlanCache` entirely. Gating normalization at `PlanCache` construction would
//! silently leave every one of those paths evaluating un-normalized algebra,
//! reintroducing exactly the two-tier inconsistency (some `EXISTS` sites
//! defensible, some not) this task exists to eliminate. Hooking at first-encounter
//! with a per-query cache guarantees uniform coverage on every path while staying
//! "prepare-time" in the sense that matters operationally: computed once per
//! distinct site per evaluation, before any row of that site is tested, never
//! recomputed per row.

use purrdf_sparql_algebra::GraphPattern;

/// The outcome of normalizing one `EXISTS`/`NOT EXISTS` inner pattern to Existential
/// Normal Form (see the [module docs](self)).
#[derive(Debug, Clone)]
pub(crate) enum Enf {
    /// The normalized pattern: `exists(pattern, μ)` and `exists(original, μ)` agree
    /// for every μ (this module's laws are emptiness-preserving), so evaluating THIS
    /// pattern's non-emptiness answers the original `EXISTS` exactly.
    Pattern(GraphPattern),
    /// Law 4b decided the answer without evaluating anything: a `Slice(_, Some(0))`
    /// on the spine makes the inner empty for every `P` beneath it and every μ, so
    /// `EXISTS` is always `false` (and `NOT EXISTS`, `Not(Exists(..))` in this
    /// algebra, always `true`).
    FoldedEmpty,
}

/// Normalize `pattern` — an `EXISTS`/`NOT EXISTS` inner — to Existential Normal Form
/// (see the [module docs](self) for the laws and their proof sketches).
///
/// Applied to fixpoint: every law that erases a wrapper recurses into what it
/// exposed, so a chain of several spine wrappers collapses in one call. Pure,
/// deterministic, and idempotent — see the module doc's "Fixpoint, determinism,
/// idempotence" section.
pub(crate) fn normalize(pattern: &GraphPattern) -> Enf {
    match pattern {
        // Law 1: THE F2 FIX BY LAW.
        GraphPattern::LeftJoin { left, .. } => normalize(left),
        // Law 2.
        GraphPattern::OrderBy { inner, .. } => normalize(inner),
        // Law 3 (the "no Slice(start>0) above" qualifier holds automatically — see
        // the module doc).
        GraphPattern::Distinct { inner } | GraphPattern::Reduced { inner } => normalize(inner),
        // Law 4a/4b.
        GraphPattern::Slice {
            inner,
            start,
            length,
        } => match (*start, *length) {
            (_, Some(0)) => Enf::FoldedEmpty,
            (0, _) => normalize(inner),
            // start > 0: not a transparent wrapper; stop here, unmodified.
            (_, _) => Enf::Pattern(pattern.clone()),
        },
        // Project is transparent to the spine, but it is also a real node in the
        // output (the `PrjMap` boundary substitution narrows against) — rebuild it
        // over whatever its own child normalized to, rather than erasing it outright.
        GraphPattern::Project { inner, variables } => match normalize(inner) {
            Enf::FoldedEmpty => Enf::FoldedEmpty,
            Enf::Pattern(p) => Enf::Pattern(GraphPattern::Project {
                inner: Box::new(p),
                variables: variables.clone(),
            }),
        },
        // Union: both branches are on the spine (empty iff BOTH are), so both get
        // the same treatment; a branch that folds to empty drops out of the
        // reconstructed Union entirely (Union(∅, R) ≡ R for emptiness purposes).
        GraphPattern::Union { left, right } => match (normalize(left), normalize(right)) {
            (Enf::FoldedEmpty, Enf::FoldedEmpty) => Enf::FoldedEmpty,
            (Enf::FoldedEmpty, Enf::Pattern(r)) => Enf::Pattern(r),
            (Enf::Pattern(l), Enf::FoldedEmpty) => Enf::Pattern(l),
            (Enf::Pattern(l), Enf::Pattern(r)) => Enf::Pattern(GraphPattern::Union {
                left: Box::new(l),
                right: Box::new(r),
            }),
        },
        // Every other variant consumes a row SET, not merely emptiness (Join/Filter/
        // Extend/Graph/Minus/LeftJoin already handled above/Bgp/Path/Values/
        // PropertyFunction/Service/Group/Lateral) — the spine stops here, unmodified.
        other => Enf::Pattern(other.clone()),
    }
}

#[cfg(test)]
mod tests {
    use purrdf_sparql_algebra::{
        Expression, NamedNode, NamedNodePattern, OrderExpression, TermPattern, TriplePattern,
        Variable,
    };

    use super::*;

    fn var(name: &str) -> Variable {
        Variable::new(name)
    }

    fn tp(s: &str, p: &str, o: &str) -> TriplePattern {
        TriplePattern {
            subject: TermPattern::Variable(var(s)),
            predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked(format!(
                "http://ex/{p}"
            ))),
            object: TermPattern::Variable(var(o)),
        }
    }

    fn bgp(s: &str, p: &str, o: &str) -> GraphPattern {
        GraphPattern::Bgp {
            patterns: vec![tp(s, p, o)],
        }
    }

    fn assert_pattern(enf: Enf) -> GraphPattern {
        match enf {
            Enf::Pattern(p) => p,
            Enf::FoldedEmpty => panic!("expected Enf::Pattern, got FoldedEmpty"),
        }
    }

    /// Law 1: a bare `OPTIONAL` at the top of an `EXISTS` inner erases to its left
    /// (required) operand — the F2 shape, `EXISTS { OPTIONAL { ?x :q ?y } }`, whose
    /// left operand is the identity pattern (an empty `Bgp`).
    #[test]
    fn enf_left_join_erases_on_the_spine() {
        let left = GraphPattern::Bgp { patterns: vec![] };
        let right = bgp("x", "q", "y");
        let inner = GraphPattern::LeftJoin {
            left: Box::new(left.clone()),
            right: Box::new(right),
            expression: None,
        };
        let normalized = assert_pattern(normalize(&inner));
        assert_eq!(normalized, left, "LeftJoin must erase to its left operand");
    }

    /// Law 2: `ORDER BY` at the top of an `EXISTS` inner erases entirely — sorting
    /// never changes whether a bag is empty.
    #[test]
    fn enf_order_by_erases() {
        let p = bgp("x", "q", "y");
        let inner = GraphPattern::OrderBy {
            inner: Box::new(p.clone()),
            expression: vec![OrderExpression::Asc(Expression::Variable(var("y")))],
        };
        let normalized = assert_pattern(normalize(&inner));
        assert_eq!(normalized, p, "OrderBy must erase entirely");
    }

    /// Law 3: `DISTINCT`/`REDUCED` at the top of an `EXISTS` inner erase, when no
    /// restricting-offset `Slice` sits above them on the spine (here: nothing does).
    #[test]
    fn enf_distinct_erases_without_offset_above() {
        let p = bgp("x", "q", "y");
        let distinct = GraphPattern::Distinct {
            inner: Box::new(p.clone()),
        };
        assert_eq!(assert_pattern(normalize(&distinct)), p);

        let reduced = GraphPattern::Reduced {
            inner: Box::new(p.clone()),
        };
        assert_eq!(assert_pattern(normalize(&reduced)), p);
    }

    /// Law 4a: `LIMIT` with room for at least one row (`Slice(0, Some(n >= 1))`)
    /// erases entirely — it can only drop rows from the END of the bag.
    #[test]
    fn enf_limit_one_erases() {
        let p = bgp("x", "q", "y");
        let inner = GraphPattern::Slice {
            inner: Box::new(p.clone()),
            start: 0,
            length: Some(1),
        };
        assert_eq!(assert_pattern(normalize(&inner)), p);

        // Also true for length None (no LIMIT at all — an identity slice).
        let identity = GraphPattern::Slice {
            inner: Box::new(p.clone()),
            start: 0,
            length: None,
        };
        assert_eq!(assert_pattern(normalize(&identity)), p);
    }

    /// Law 4b: `Slice(_, Some(0))` folds the WHOLE `EXISTS` to constant `false`,
    /// represented as [`Enf::FoldedEmpty`] rather than a pattern to evaluate.
    #[test]
    fn enf_limit_zero_folds_false() {
        let p = bgp("x", "q", "y");
        let inner = GraphPattern::Slice {
            inner: Box::new(p),
            start: 0,
            length: Some(0),
        };
        assert!(matches!(normalize(&inner), Enf::FoldedEmpty));

        // Also true with a nonzero offset alongside the zero length.
        let inner_offset = GraphPattern::Slice {
            inner: Box::new(bgp("x", "q", "y")),
            start: 3,
            length: Some(0),
        };
        assert!(matches!(normalize(&inner_offset), Enf::FoldedEmpty));
    }

    /// The laws never fire through a `Join`/`Filter`/`Extend`/`Graph`/`Minus`
    /// operand: a `LeftJoin` buried under a `Join` stays exactly as written, because
    /// the `Join` consumes a row SET, not merely emptiness, and erasing the
    /// `LeftJoin` underneath it would change what the `Join` computes.
    #[test]
    fn enf_laws_do_not_fire_off_spine() {
        let left_join = GraphPattern::LeftJoin {
            left: Box::new(bgp("a", "p", "b")),
            right: Box::new(bgp("x", "q", "y")),
            expression: None,
        };
        let inner = GraphPattern::Join {
            left: Box::new(bgp("s", "r", "t")),
            right: Box::new(left_join.clone()),
        };
        let normalized = assert_pattern(normalize(&inner));
        assert_eq!(
            normalized, inner,
            "a Join operand must not be rewritten by the spine laws"
        );

        // Same for Filter, Extend, Graph, and Minus operands.
        let under_filter = GraphPattern::Filter {
            expr: Expression::Literal(purrdf_sparql_algebra::Literal::new_typed(
                "true",
                NamedNode::new_unchecked("http://www.w3.org/2001/XMLSchema#boolean"),
            )),
            inner: Box::new(left_join.clone()),
        };
        assert_eq!(assert_pattern(normalize(&under_filter)), under_filter);

        let under_minus = GraphPattern::Minus {
            left: Box::new(bgp("s", "r", "t")),
            right: Box::new(left_join),
        };
        assert_eq!(assert_pattern(normalize(&under_minus)), under_minus);
    }

    /// Idempotence: normalizing an already-normalized tree is a no-op.
    #[test]
    fn enf_normalize_is_idempotent() {
        let inner = GraphPattern::OrderBy {
            inner: Box::new(GraphPattern::Distinct {
                inner: Box::new(GraphPattern::LeftJoin {
                    left: Box::new(bgp("a", "p", "b")),
                    right: Box::new(bgp("x", "q", "y")),
                    expression: None,
                }),
            }),
            expression: vec![],
        };
        let once = assert_pattern(normalize(&inner));
        let twice = assert_pattern(normalize(&once));
        assert_eq!(once, twice);
    }
}
