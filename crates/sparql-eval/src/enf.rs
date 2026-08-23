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
//! ## Side conditions: a law may erase only an EFFECT-FREE portion
//!
//! Every law above is proved emptiness-equivalent over ROW SETS — it says what the
//! erased portion's presence or absence does to the row COUNT, never what evaluating
//! it might otherwise have done. A hard [`EvalError`](crate::EvalError) (an
//! unresolved custom function, an unconfigured `heldIn`, a malformed `rdf:List`, an
//! unresolved property-function relation or a mis-invoked one) or an observable
//! remote effect (a `SERVICE` call, `SILENT` or not) raised while evaluating the
//! erased portion would propagate outside the `EXISTS` if that portion sat OUTSIDE
//! one — so erasing it unconditionally would make the same query shape silently
//! swallow the error/effect merely for having been written inside an `EXISTS`. Each
//! law below is therefore additionally gated on
//! [`crate::governor::soundness::NodeAnalysis::can_hard_error`] being `false` for
//! the portion it erases; a law whose gate does not hold simply does not fire, and
//! the un-erased pattern falls through to the definition/probe machinery, which
//! evaluates it and lets the error/effect propagate exactly as it would outside an
//! `EXISTS`. A law that erases nothing evaluable (3, 4a) needs no gate at all.
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
//! **Side condition**: fires only when BOTH `B` and `c` are effect-free (see "Side
//! conditions" above) — erasing either could otherwise erase a hard error or a
//! `SERVICE` effect it would have raised.
//!
//! ## Law 2 — `OrderBy(P) → P` on the spine
//!
//! Sorting is a permutation, never a filter: `OrderBy(P)` and `P` have the same
//! multiset of rows, so they are empty under exactly the same condition. The sort
//! keys are never evaluated at all once emptiness is the only question asked.
//!
//! **Side condition**: fires only when every sort key expression is effect-free —
//! "never evaluated at all" is exactly the erasure this module's laws must not
//! perform on a key that could otherwise have hard-failed.
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
//! **Side condition**: none — unconditional. Only the dedup wrapper is erased; `P`
//! itself is still evaluated by whatever [`normalize`] returns, so nothing
//! evaluable is ever discarded.
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
//!   the slice is empty exactly when `P` is. **Side condition**: none — unconditional,
//!   for the same reason as law 3: only the `Slice` wrapper is erased, and `P` is
//!   still evaluated by whatever [`normalize`] returns.
//! * **4b**: `Slice(_, Some(0))(P) → ⊥` (the whole `EXISTS` folds to constant
//!   `false`). A zero-length slice is empty FOR EVERY `P` and EVERY μ — it needs no
//!   evaluation at all, so the fold is represented directly: [`normalize`] returns
//!   [`Enf::FoldedEmpty`] rather than a pattern to evaluate, and
//!   [`crate::expr::exists`] answers `false` (making `NOT EXISTS`, `Not(Exists(..))`
//!   in this algebra, answer `true`) without touching the dataset. **Side
//!   condition**: fires only when the WHOLE of `P` is effect-free — the fold
//!   suppresses every evaluation `P` would otherwise have performed, so an effectful
//!   `P` must fall through and actually be evaluated instead of being folded away.
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

use purrdf_sparql_algebra::{GraphPattern, OrderExpression};

use crate::expr::{SubstitutionSource, SubstitutionSourceMap};
use crate::governor::soundness;

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

/// Law 1's gate: `LeftJoin{left, right, expression}` erases to `left` only when
/// everything it would ERASE (`right`, the join condition) is effect-free. Shared
/// verbatim by [`normalize`] and [`ledger_source_map`]'s spine walk — see that
/// function's doc for why the two must never diverge.
fn left_join_erasable(
    right: &GraphPattern,
    expression: Option<&purrdf_sparql_algebra::Expression>,
) -> bool {
    let right_clean = !soundness::pattern_can_hard_error(right);
    let condition_clean = expression.is_none_or(|e| !soundness::expr_can_hard_error(e));
    right_clean && condition_clean
}

/// Law 2's gate: `OrderBy` erases entirely only when its sort keys — the ERASED
/// portion — are effect-free. Shared verbatim by [`normalize`] and
/// [`ledger_source_map`]'s spine walk.
fn order_by_erasable(expression: &[OrderExpression]) -> bool {
    !expression.iter().any(|oe| {
        let e = match oe {
            OrderExpression::Asc(e) | OrderExpression::Desc(e) => e,
        };
        soundness::expr_can_hard_error(e)
    })
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
        // Law 1: THE F2 FIX BY LAW — gated on the ERASED portion (`right`, the
        // join condition) being effect-free; see the module doc's "Side
        // conditions" section and `crate::governor::soundness::NodeAnalysis::can_hard_error`.
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => {
            if left_join_erasable(right, expression.as_ref()) {
                normalize(left)
            } else {
                Enf::Pattern(pattern.clone())
            }
        }
        // Law 2 — gated on the ERASED portion (the sort keys) being effect-free.
        GraphPattern::OrderBy { inner, expression } => {
            if order_by_erasable(expression) {
                normalize(inner)
            } else {
                Enf::Pattern(pattern.clone())
            }
        }
        // Law 3 (the "no Slice(start>0) above" qualifier holds automatically — see
        // the module doc). Unconditional: nothing evaluable is erased, only the
        // dedup wrapper — `inner` is still evaluated by whatever this recursion
        // returns.
        GraphPattern::Distinct { inner } | GraphPattern::Reduced { inner } => normalize(inner),
        // Law 4a/4b.
        GraphPattern::Slice {
            inner,
            start,
            length,
        } => match (*start, *length) {
            // 4b is gated on the WHOLE inner being effect-free: the fold answers
            // `EXISTS` without evaluating anything, so an inner that could have
            // hard-failed or reached a federation endpoint must not be erased.
            (_, Some(0)) => {
                if soundness::pattern_can_hard_error(inner) {
                    Enf::Pattern(pattern.clone())
                } else {
                    Enf::FoldedEmpty
                }
            }
            // 4a is unconditional: an offset-zero, room-for-at-least-one-row
            // slice erases nothing evaluable — `inner` is still evaluated by
            // whatever this recursion returns.
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

/// Map every node address inside `normalized` — an already-computed `Enf::Pattern` result
/// from calling [`normalize`] on `original` — back to the ORIGINAL, un-normalized
/// `original` tree's node address.
///
/// # Why this is needed at all
///
/// `ChargeLedger::for_plan`'s walk ([`crate::governor::ledger`]) assigns a ledger
/// ordinal to every node [`crate::governor::soundness::walk_spine`] visits —
/// including an `EXISTS`/`NOT EXISTS` inner pattern reached through an expression
/// (`crate::governor::soundness::visit_exists_patterns`), so `original`'s own nodes
/// already have ledger identity. But `normalize`'s `other => Enf::Pattern(other.clone())`
/// arm (and its two erasure-fallback arms) allocate a FRESH clone: `normalized`'s own
/// addresses never equal `original`'s, so a charge made while evaluating `normalized`
/// can never resolve to the ledger ordinal `original` already has. This walk is the
/// bridge: [`crate::binop::eval_correlated`]'s `EXISTS` caller pushes the returned map
/// onto [`crate::eval::EvalCtx::correlated_node_maps`] before evaluating `normalized`
/// (or the `witness_wrapped` form built from it), so `EvalCtx::resolve_ledger_ordinal`'s
/// existing one-hop-per-window chase resolves straight through to `original`'s ordinal —
/// no new resolution machinery, the same one `LATERAL`'s per-row substitution already
/// uses.
///
/// # What this walk covers, and what it deliberately does not
///
/// Mirrors [`normalize`]'s own recursion for the two erasure-only wrappers (a
/// spine-erasing `LeftJoin`/`OrderBy`/`Distinct`/`Reduced`/`Slice(0, _)`: no address of
/// its own, pure delegation to whatever its child resolves to) and the common terminal
/// case (every other variant, which `normalize` clones wholesale — `normalized` is then
/// a 1:1 structural copy of `original`, walked in lockstep via
/// [`crate::governor::soundness::visit_classified_children`], the SAME child order
/// [`crate::governor::soundness::walk_spine`] used to assign `original`'s own
/// ordinals). This also walks into any `EXISTS` pattern nested inside an expression the
/// clone carries, so a doubly-nested `EXISTS` reached this way gets its own entry too —
/// which is what lets [`crate::eval::EvalCtx::prepared_exists`] resolve a nested site's
/// cache key back to a stable address instead of rebuilding every outer row.
///
/// Declines to track — returning the map unchanged, with no entry for that node — for
/// `normalize`'s two SYNTHESIZING cases, `Project` and `Union`: their own root is a
/// freshly built combination, not a clone of one single node, and reconstructing that
/// correspondence needs the same `counts_rows` arbitration
/// [`crate::expr::substitute_pattern_tracked`] already does for `LATERAL`'s Values-Insertion
/// wrappers. Left as a documented, deliberate limitation rather than attempted here: a
/// query whose `EXISTS` inner is directly `{ SELECT/DISTINCT/UNION ... }`-shaped at its
/// OWN top level still charges correctly — its charges simply keep rolling into the
/// enclosing `FILTER`/`BIND`, exactly the behavior before this function existed, never
/// a regression.
pub(crate) fn ledger_source_map(
    original: &GraphPattern,
    normalized: &GraphPattern,
) -> SubstitutionSourceMap {
    let mut map = SubstitutionSourceMap::default();
    map_spine(original, normalized, &mut map);
    map
}

/// [`ledger_source_map`]'s recursive spine walk. See that function's doc.
fn map_spine(original: &GraphPattern, normalized: &GraphPattern, map: &mut SubstitutionSourceMap) {
    match original {
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } if left_join_erasable(right, expression.as_ref()) => {
            map_spine(left, normalized, map);
        }
        GraphPattern::OrderBy { inner, expression } if order_by_erasable(expression) => {
            map_spine(inner, normalized, map);
        }
        GraphPattern::Distinct { inner } | GraphPattern::Reduced { inner } => {
            map_spine(inner, normalized, map);
        }
        // Mirrors law 4a: an offset-zero slice with room for at least one row erases.
        // A `Slice(_, Some(0))` reached here is, by construction, the can-hard-error
        // terminal-clone case (see `normalize`'s law 4b): if it had folded empty
        // instead, EVERY enclosing step that transparently forwarded it here would
        // have folded empty too, and `ledger_source_map`'s caller would never have had
        // an `Enf::Pattern` — and therefore a `normalized` tree — to walk in the first
        // place. It falls through to the terminal arm below, exactly like `normalize`'s
        // own `(_, Some(0)) if can_hard_error` branch.
        GraphPattern::Slice {
            inner,
            start: 0,
            length,
        } if *length != Some(0) => {
            map_spine(inner, normalized, map);
        }
        // `Project`/`Union`: synthesizing cases this walk declines to track — see
        // [`ledger_source_map`]'s doc.
        GraphPattern::Project { .. } | GraphPattern::Union { .. } => {}
        // Every other shape, including a non-erasing `LeftJoin`/`OrderBy`/`Slice`, is
        // the terminal shape `normalize` clones wholesale.
        _ => map_clone_1to1(original, normalized, map),
    }
}

/// Record `normalized` (a 1:1 structural clone of `original` — see [`map_spine`]) and
/// every one of its descendants, INCLUDING any `EXISTS` pattern reached through a
/// nested expression, against the corresponding node of `original`. `counts_rows` is
/// unconditionally `true` throughout: unlike `LATERAL`'s Values-Insertion machinery,
/// this walk never adds a node `original` did not already have, so there is no
/// wrapper/wrapped ambiguity to arbitrate — every node here really is the one true
/// output of the real node it corresponds to.
fn map_clone_1to1(
    original: &GraphPattern,
    normalized: &GraphPattern,
    map: &mut SubstitutionSourceMap,
) {
    map.insert(
        std::ptr::from_ref(normalized) as usize,
        SubstitutionSource {
            source: std::ptr::from_ref(original) as usize,
            counts_rows: true,
        },
    );
    let mut original_children: smallvec::SmallVec<[&GraphPattern; 4]> = smallvec::SmallVec::new();
    soundness::visit_classified_children(original, &mut |child, _edge| {
        original_children.push(child);
        false
    });
    let mut normalized_children: smallvec::SmallVec<[&GraphPattern; 4]> = smallvec::SmallVec::new();
    soundness::visit_classified_children(normalized, &mut |child, _edge| {
        normalized_children.push(child);
        false
    });
    debug_assert_eq!(
        original_children.len(),
        normalized_children.len(),
        "normalized is a structural clone of original at this point in the walk, so their \
         classified children must pair up 1:1"
    );
    for (o, n) in original_children.into_iter().zip(normalized_children) {
        map_clone_1to1(o, n, map);
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

    /// Positive control for the `can_hard_error` side conditions added to gate
    /// laws 1, 2, and 4b (see the module doc's "Side conditions" section): every
    /// law still fires on an ordinary, EFFECT-FREE shape — the gate must not
    /// regress the common case, only refuse the effectful one. Each arm below is
    /// the same clean shape [`enf_left_join_erases_on_the_spine`],
    /// [`enf_order_by_erases`], and [`enf_limit_zero_folds_false`] already cover;
    /// see `enf::effect_free_gate_tests` for the evaluation-level negative twins
    /// (an effectful `B`/condition/sort-key/inner that must NOT be erased).
    #[test]
    fn enf_laws_still_fire_on_pure_shapes() {
        // Law 1: a clean `B` AND a clean join condition still erase to `A`.
        let left = GraphPattern::Bgp { patterns: vec![] };
        let right = bgp("x", "q", "y");
        let inner = GraphPattern::LeftJoin {
            left: Box::new(left.clone()),
            right: Box::new(right),
            expression: Some(Expression::Literal(
                purrdf_sparql_algebra::Literal::new_typed(
                    "true",
                    NamedNode::new_unchecked("http://www.w3.org/2001/XMLSchema#boolean"),
                ),
            )),
        };
        assert_eq!(
            assert_pattern(normalize(&inner)),
            left,
            "a clean B and a clean join condition must still erase to A"
        );

        // Law 2: clean sort keys still erase entirely.
        let p = bgp("x", "q", "y");
        let order_by = GraphPattern::OrderBy {
            inner: Box::new(p.clone()),
            expression: vec![OrderExpression::Asc(Expression::Variable(var("y")))],
        };
        assert_eq!(
            assert_pattern(normalize(&order_by)),
            p,
            "clean sort keys must still erase entirely"
        );

        // Law 4b: a clean inner still folds to constant false.
        let slice = GraphPattern::Slice {
            inner: Box::new(bgp("x", "q", "y")),
            start: 0,
            length: Some(0),
        };
        assert!(
            matches!(normalize(&slice), Enf::FoldedEmpty),
            "a clean inner must still fold Slice(_, Some(0)) to constant false"
        );
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

/// Evaluation-level negative controls for the `can_hard_error` side conditions (see the
/// module doc's "Side conditions" section): for each gated law, an EFFECTFUL erased
/// portion (an unresolved `Function::Custom` call, or a non-`SILENT` `SERVICE`) must
/// make the law refuse to fire, so the definition/probe machinery evaluates the
/// un-erased pattern and the hard error/effect propagates from INSIDE an `EXISTS`
/// exactly as it does OUTSIDE one, closing the error-swallowing hole these laws
/// otherwise open. [`enf::tests::enf_laws_still_fire_on_pure_shapes`] is the
/// positive control: the same laws still fire when the erased portion is clean.
///
/// Every "inside EXISTS" assertion here has an "outside EXISTS" twin: the same erased
/// pattern, evaluated directly (`eval`, not `Expression::Exists`), must hard-error
/// identically — proving the fix makes the two agree rather than merely making the
/// inside case error somehow.
#[cfg(test)]
mod effect_free_gate_tests {
    use std::sync::Arc;

    use purrdf_core::{RdfDataset, RdfDatasetBuilder};
    use purrdf_sparql_algebra::{
        Expression, Function, GraphPattern, NamedNode, NamedNodePattern, OrderExpression,
        TermPattern, TriplePattern, Variable,
    };

    use super::{Enf, normalize};
    use crate::error::{EvalError, UnsupportedKind};
    use crate::eval::{EvalCtx, eval};

    const EX: &str = "http://example.org/";

    fn var(name: &str) -> Variable {
        Variable::new(name)
    }

    fn tvar(name: &str) -> TermPattern {
        TermPattern::Variable(var(name))
    }

    fn nn(iri: &str) -> NamedNode {
        NamedNode::new_unchecked(iri)
    }

    fn triple(s: TermPattern, iri: &str, o: TermPattern) -> TriplePattern {
        TriplePattern {
            subject: s,
            predicate: NamedNodePattern::NamedNode(nn(iri)),
            object: o,
        }
    }

    fn bgp1(s: TermPattern, iri: &str, o: TermPattern) -> GraphPattern {
        GraphPattern::Bgp {
            patterns: vec![triple(s, iri, o)],
        }
    }

    fn bx(p: GraphPattern) -> Box<GraphPattern> {
        Box::new(p)
    }

    /// An unresolved `Function::Custom` call over `?w`, which hard-errors
    /// (`UnsupportedKind::CustomFunction`) wherever it is actually evaluated,
    /// `EXISTS` or not.
    fn undefined_fn_call() -> Expression {
        Expression::FunctionCall(
            Function::Custom(nn(&format!("{EX}undefined-fn"))),
            vec![Expression::Variable(var("w"))],
        )
    }

    fn assert_custom_function_error(err: &EvalError) {
        assert!(
            matches!(
                err,
                EvalError::Unsupported {
                    kind: Some(UnsupportedKind::CustomFunction),
                    ..
                }
            ),
            "expected UnsupportedKind::CustomFunction, got {err:?}"
        );
    }

    fn assert_remote_error(err: &EvalError) {
        assert!(
            matches!(err, EvalError::Remote(_)),
            "expected EvalError::Remote, got {err:?}"
        );
    }

    /// The fixture every test below shares: one outer row (`:s1 :outer :dummy`) to
    /// drive `EXISTS` over, and one row `:o1 :q :w1` for the erased portion's own
    /// `Bgp` to match — so the effectful expression it carries is actually reached
    /// and evaluated, not skipped for want of a candidate row.
    fn ds() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let outer_p = b.intern_iri(&format!("{EX}outer"));
        let s1 = b.intern_iri(&format!("{EX}s1"));
        let dummy = b.intern_iri(&format!("{EX}dummy"));
        let q = b.intern_iri(&format!("{EX}q"));
        let o1 = b.intern_iri(&format!("{EX}o1"));
        let w1 = b.intern_iri(&format!("{EX}w1"));
        b.push_quad(s1, outer_p, dummy, None);
        b.push_quad(o1, q, w1, None);
        b.freeze().expect("freeze")
    }

    fn outer() -> GraphPattern {
        bgp1(tvar("s"), &format!("{EX}outer"), tvar("d"))
    }

    /// Evaluate `Expression::Exists(inner)` once per row of `outer()`'s result
    /// against `ds`. Returns one `Result` per outer row, in row order.
    fn exists_results(ds: &Arc<RdfDataset>, inner: &GraphPattern) -> Vec<Result<bool, EvalError>> {
        let mut ctx = EvalCtx::new(ds);
        let outer_pattern = outer();
        let seq = eval(&outer_pattern, &mut ctx).expect("outer pattern evaluates");
        assert_eq!(
            seq.rows.len(),
            1,
            "the shared fixture drives exactly one outer row"
        );
        let exists_expr = Expression::Exists(Box::new(inner.clone()));
        seq.rows
            .iter()
            .map(|row| {
                crate::expr::eval_ebv(&exists_expr, row, &seq.schema, &mut ctx).map(|ebv| {
                    ebv.expect("EXISTS always yields a defined boolean when it succeeds")
                })
            })
            .collect()
    }

    /// The same `inner` pattern, evaluated directly (no `EXISTS`) — the "outside
    /// EXISTS" twin every assertion below compares against.
    fn outside_result(ds: &Arc<RdfDataset>, inner: &GraphPattern) -> Result<(), EvalError> {
        let mut ctx = EvalCtx::new(ds);
        eval(inner, &mut ctx).map(|_| ())
    }

    /// Law 1, `B` (the `OPTIONAL` right operand) carries an unresolved custom
    /// function call: `LeftJoin(A, B, None)` must NOT erase to `A`, because doing so
    /// would delete a hard error `B`'s own evaluation would otherwise raise.
    #[test]
    fn enf_left_join_law_requires_effect_free_right() {
        let right = GraphPattern::Filter {
            expr: undefined_fn_call(),
            inner: bx(bgp1(tvar("o"), &format!("{EX}q"), tvar("w"))),
        };
        let inner = GraphPattern::LeftJoin {
            left: bx(GraphPattern::Bgp { patterns: vec![] }),
            right: bx(right),
            expression: None,
        };

        let ds = ds();
        assert_custom_function_error(
            &outside_result(&ds, &inner).expect_err("B must hard-error outside EXISTS too"),
        );

        let results = exists_results(&ds, &inner);
        assert_eq!(results.len(), 1);
        assert_custom_function_error(
            results[0]
                .as_ref()
                .expect_err("Law 1 must not erase a B that can hard-error"),
        );
    }

    /// Law 1, the join CONDITION carries an unresolved custom function call (`B`
    /// itself is clean): `LeftJoin(A, B, c)` must NOT erase to `A`.
    #[test]
    fn enf_left_join_law_requires_effect_free_condition() {
        let inner = GraphPattern::LeftJoin {
            left: bx(GraphPattern::Bgp { patterns: vec![] }),
            right: bx(bgp1(tvar("o"), &format!("{EX}q"), tvar("w"))),
            expression: Some(undefined_fn_call()),
        };

        let ds = ds();
        assert_custom_function_error(
            &outside_result(&ds, &inner)
                .expect_err("the join condition must hard-error outside EXISTS too"),
        );

        let results = exists_results(&ds, &inner);
        assert_eq!(results.len(), 1);
        assert_custom_function_error(
            results[0]
                .as_ref()
                .expect_err("Law 1 must not erase when the join condition can hard-error"),
        );
    }

    /// Law 2, a sort key carries an unresolved custom function call: `OrderBy(P)`
    /// must NOT erase to `P` — the sort keys would never be evaluated at all.
    #[test]
    fn enf_order_by_law_requires_effect_free_keys() {
        let inner = GraphPattern::OrderBy {
            inner: bx(bgp1(tvar("o"), &format!("{EX}q"), tvar("w"))),
            expression: vec![OrderExpression::Asc(undefined_fn_call())],
        };

        let ds = ds();
        assert_custom_function_error(
            &outside_result(&ds, &inner)
                .expect_err("the sort key must hard-error outside EXISTS too"),
        );

        let results = exists_results(&ds, &inner);
        assert_eq!(results.len(), 1);
        assert_custom_function_error(
            results[0]
                .as_ref()
                .expect_err("Law 2 must not erase when a sort key can hard-error"),
        );
    }

    /// Law 4b, the WHOLE inner `P` under a `Slice(_, Some(0))` carries an unresolved
    /// custom function call: the `EXISTS` must NOT fold to constant `false` — the
    /// fold suppresses every evaluation `P` would otherwise have performed.
    #[test]
    fn enf_limit_zero_fold_requires_effect_free_inner() {
        let inner = GraphPattern::Slice {
            inner: bx(GraphPattern::Filter {
                expr: undefined_fn_call(),
                inner: bx(bgp1(tvar("o"), &format!("{EX}q"), tvar("w"))),
            }),
            start: 0,
            length: Some(0),
        };

        // `normalize` itself must not fold this to `Enf::FoldedEmpty`.
        assert!(
            !matches!(normalize(&inner), Enf::FoldedEmpty),
            "an effectful inner must not be folded to constant false"
        );

        let ds = ds();
        assert_custom_function_error(&outside_result(&ds, &inner).expect_err(
            "the inner must hard-error outside EXISTS too (Slice always \
                             evaluates its inner in full before truncating — see \
                             `crate::modifier::eval_slice`)",
        ));

        let results = exists_results(&ds, &inner);
        assert_eq!(results.len(), 1);
        assert_custom_function_error(
            results[0]
                .as_ref()
                .expect_err("Law 4b must not fold to false when the inner can hard-error"),
        );
    }

    /// Law 1, `B` carries a non-`SILENT` `SERVICE` call: `LeftJoin(A, B, None)` must
    /// NOT erase to `A` — deleting `B` would delete the federation effect/error it
    /// would otherwise raise, exactly the inconsistency the module doc's "one-
    /// definition theorem" section calls out (`probe_admissible` already refuses
    /// `Service` for the identical reason).
    #[test]
    fn enf_service_in_erased_operand_is_not_deleted() {
        let right = GraphPattern::Join {
            left: bx(bgp1(tvar("o"), &format!("{EX}q"), tvar("w"))),
            right: bx(GraphPattern::Service {
                name: NamedNodePattern::NamedNode(nn(&format!("{EX}ep"))),
                inner: bx(bgp1(tvar("w"), &format!("{EX}name"), tvar("n"))),
                silent: false,
            }),
        };
        let inner = GraphPattern::LeftJoin {
            left: bx(GraphPattern::Bgp { patterns: vec![] }),
            right: bx(right),
            expression: None,
        };

        let ds = ds();
        // No remote source is configured on this `EvalCtx`, so a non-SILENT
        // `SERVICE` hard-errors — the same failure mode a top-level (non-EXISTS)
        // query using this SERVICE call would raise.
        assert_remote_error(
            &outside_result(&ds, &inner)
                .expect_err("SERVICE must federation-error outside EXISTS too"),
        );

        let results = exists_results(&ds, &inner);
        assert_eq!(results.len(), 1);
        assert_remote_error(
            results[0]
                .as_ref()
                .expect_err("Law 1 must not delete a B carrying a non-SILENT SERVICE call"),
        );
    }
}
