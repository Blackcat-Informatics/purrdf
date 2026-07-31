// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Robinson-style structural unification, with an occurs-check, over a
//! [`term::TermDag`](crate::term::TermDag)'s hash-consed compound terms.
//!
//! # A normal answer, not a Rust error
//!
//! [`Unified::Clash`] and [`Unified::Occurs`] are ordinary, expected outcomes — "no
//! unifier exists" and "the only unifier would be infinite" are properties of the
//! two terms, not failures of this module. This mirrors [`crate::proof`]'s and
//! [`crate::chase`]'s own doctrine: a rejection the caller can inspect and act on
//! is a different thing from an engine defect, and [`unify`]/[`unify_sorted`]
//! return `Result`-shaped information through an ordinary enum rather than
//! `Result<(), Error>`, so a caller cannot accidentally `?`-propagate a normal
//! "these don't unify" outcome as if it were a bug.
//!
//! # Transactional substitutions
//!
//! [`unify`] and [`unify_sorted`] are the only two public entrances, and both are
//! transactional: on any outcome other than [`Unified::Ok`], the [`Subst`] passed
//! in is left EXACTLY as it was found, even if several metavariables were bound
//! while descending into a multi-argument application before a deeper clash was
//! found. A caller never has to reason about how far a failed unification got.
//!
//! # `BTreeMap`/`BTreeSet`, not `HashMap`/`HashSet`
//!
//! This module (and [`crate::resolve_fol`], which is built on it) is a port of an
//! algorithm whose upstream implementation used ordinary hash-based maps and sets
//! for the order-sorted machinery below. This crate's own determinism doctrine —
//! see [`crate::lib`](crate)'s module docs — forbids any map whose iteration order
//! could reach an output path, so every such collection here is a `BTreeMap` or
//! `BTreeSet` instead. This is a deliberate strengthening over the source
//! algorithm, not an oversight: the two are drop-in equivalent for correctness, and
//! only the iteration-order guarantee differs.
//!
//! # Locally-nameless unification underneath binders
//!
//! A metavariable's binding is recorded once, at its own "home" depth (ambient
//! depth 0 relative to where it was minted), never once per depth it is later
//! observed at. `whnf` is what makes this work: unfolding a bound meta while
//! recursing under `depth` binders LIFTS the stored solution by `depth` via
//! [`shift`], and `bind_meta` does the inverse — LOWERS a candidate solution
//! found `depth` binders down to the metavariable's home depth via `shift_down`,
//! which fails (a sound [`Unified::Clash`], not a panic) exactly when the
//! candidate mentions a bound variable that cannot be expressed outside the
//! binders being stripped away.

use std::collections::{BTreeMap, BTreeSet};

use crate::id::{MetaId, NodeId};
use crate::term::{NodeData, TermDag};

// ── Substitutions ────────────────────────────────────────────────────────────────

/// A triangular (union-find-like) substitution from metavariables to nodes, plus
/// each metavariable's declared sort (for order-sorted unification).
///
/// Indexed densely by [`MetaId::index`], so binding metavariable `k` never
/// requires that metavariables `0..k` be bound first — `Self::ensure` grows both
/// backing vectors on demand.
#[derive(Debug, Default, Clone)]
pub struct Subst {
    /// `bindings[m.index()]` is `Some(node)` once `m` has been bound.
    bindings: Vec<Option<NodeId>>,
    /// `meta_sort[m.index()]` is `Some(sort)` once `m`'s sort has been declared.
    meta_sort: Vec<Option<NodeId>>,
}

impl Subst {
    /// An empty substitution: nothing bound, nothing sorted.
    pub fn new() -> Self {
        Self::default()
    }

    /// Grow both backing vectors to hold index `idx`.
    fn ensure(&mut self, idx: usize) {
        if self.bindings.len() <= idx {
            self.bindings.resize(idx + 1, None);
        }
        if self.meta_sort.len() <= idx {
            self.meta_sort.resize(idx + 1, None);
        }
    }

    /// Bind `m` to `node`.
    fn bind(&mut self, m: MetaId, node: NodeId) {
        self.ensure(m.index());
        self.bindings[m.index()] = Some(node);
    }

    /// `m`'s current binding, if any.
    fn get(&self, m: MetaId) -> Option<NodeId> {
        self.bindings.get(m.index()).copied().flatten()
    }

    /// Record that `m` now stands for `node`.
    ///
    /// Identical to the private `Self::bind`, exposed for a clause-variable-
    /// freshening caller (see [`crate::resolve_fol`]'s clause instantiation): "old
    /// metavariable `m`, as authored in the clause text, now stands for the fresh
    /// node minted for this firing" is exactly a binding, and giving it its own
    /// name at the call site documents that intent without a second mechanism.
    pub fn bind_renaming(&mut self, m: MetaId, node: NodeId) {
        self.bind(m, node);
    }

    /// Declare `m`'s sort, for order-sorted unification via [`unify_sorted`].
    pub fn declare_meta_sort(&mut self, m: MetaId, sort: NodeId) {
        self.set_meta_sort(m, Some(sort));
    }

    /// Set (or clear) `m`'s declared sort.
    fn set_meta_sort(&mut self, m: MetaId, sort: Option<NodeId>) {
        self.ensure(m.index());
        self.meta_sort[m.index()] = sort;
    }

    /// `m`'s declared sort, if any was recorded.
    pub fn meta_sort(&self, m: MetaId) -> Option<NodeId> {
        self.meta_sort.get(m.index()).copied().flatten()
    }

    /// Whether `m` currently has a binding.
    fn is_bound(&self, m: MetaId) -> bool {
        self.get(m).is_some()
    }

    /// The number of currently-bound metavariables — a test-only introspection
    /// hook used to assert the transactional rollback property.
    #[cfg(test)]
    pub fn bound_count(&self) -> usize {
        self.bindings.iter().filter(|slot| slot.is_some()).count()
    }

    /// Follow `node` through `Meta` bindings until a non-`Meta` node or an unbound
    /// metavariable is reached.
    ///
    /// This resolves only the TOP-LEVEL meta chain; it does not recurse into
    /// structure (an `App` or `Binder` reached this way is returned as-is, still
    /// possibly containing further unresolved metas inside it). See `whnf` for
    /// the depth-aware wrapper this crate's unification actually calls.
    pub fn resolve(&self, dag: &TermDag, node: NodeId) -> NodeId {
        let mut current = node;
        loop {
            match dag.data(current) {
                NodeData::Meta(m) => match self.get(*m) {
                    Some(next) => current = next,
                    None => return current,
                },
                _ => return current,
            }
        }
    }
}

// ── Outcome ─────────────────────────────────────────────────────────────────────

/// The outcome of a unification attempt.
///
/// See the [module docs](self): every variant other than [`Self::Ok`] is a NORMAL
/// answer, never a Rust error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unified {
    /// A unifier was found (and, for [`unify`]/[`unify_sorted`], recorded into the
    /// substitution).
    Ok,
    /// No unifier exists: `left` and `right` are structurally incompatible (a
    /// symbol/arity mismatch, or a sort violation under [`unify_sorted`]).
    Clash {
        /// The left side of the incompatible pair, after resolving through the
        /// substitution.
        left: NodeId,
        /// The right side of the incompatible pair, after resolving through the
        /// substitution.
        right: NodeId,
    },
    /// The only unifier would be infinite: binding `meta` to `in_node` would make
    /// `meta` occur in its own solution.
    Occurs {
        /// The metavariable that would occur in its own binding.
        meta: MetaId,
        /// The node the occurs-check was run against.
        in_node: NodeId,
    },
}

// ── Order-sorted support ─────────────────────────────────────────────────────────

/// A subsort order over [`NodeId`]s standing for sort names.
///
/// Built once, from covering `(sub, super)` edges, as the reflexive-transitive
/// closure of "immediate supersort". [`Self::leq`] and [`Self::meet`] then answer
/// against the closed order in O(1) amortised lookups rather than walking edges
/// per query.
#[derive(Debug, Default, Clone)]
pub struct SortOrder {
    /// `up[a]` is the set of `b` with `a <: b`, reflexive-transitively closed.
    up: BTreeMap<NodeId, BTreeSet<NodeId>>,
    /// Every sort name mentioned by any edge.
    universe: BTreeSet<NodeId>,
}

impl SortOrder {
    /// Build the reflexive-transitive closure of `up` from covering edges `(a, b)`
    /// meaning `a <: b`.
    pub fn from_subclass_edges(edges: &[(NodeId, NodeId)]) -> Self {
        let mut up: BTreeMap<NodeId, BTreeSet<NodeId>> = BTreeMap::new();
        let mut universe: BTreeSet<NodeId> = BTreeSet::new();
        for &(a, b) in edges {
            universe.insert(a);
            universe.insert(b);
            up.entry(a).or_default().insert(b);
        }
        // Reflexivity: every sort is its own subsort.
        for &node in &universe {
            up.entry(node).or_default().insert(node);
        }
        // Transitive closure: repeat until a full pass adds nothing.
        loop {
            let mut changed = false;
            let snapshot: Vec<(NodeId, BTreeSet<NodeId>)> = up
                .iter()
                .map(|(&node, supers)| (node, supers.clone()))
                .collect();
            for (node, supers) in &snapshot {
                let additions: BTreeSet<NodeId> = supers
                    .iter()
                    .flat_map(|&mid| up.get(&mid).cloned().unwrap_or_default())
                    .collect();
                let entry = up.get_mut(node).expect("node was just snapshotted from up");
                let before = entry.len();
                entry.extend(additions);
                if entry.len() != before {
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        Self { up, universe }
    }

    /// Whether `a <: b` in the closed order. Reflexive: `leq(a, a)` is always true.
    pub fn leq(&self, a: NodeId, b: NodeId) -> bool {
        a == b || self.up.get(&a).is_some_and(|supers| supers.contains(&b))
    }

    /// The greatest lower bound of `a` and `b`, if a UNIQUE one exists among the
    /// declared universe; `None` if the bound is ambiguous (two incomparable
    /// maximal candidates) or nonexistent.
    pub fn meet(&self, a: NodeId, b: NodeId) -> Option<NodeId> {
        let candidates: Vec<NodeId> = self
            .universe
            .iter()
            .copied()
            .filter(|&x| self.leq(x, a) && self.leq(x, b))
            .collect();
        let maximal: Vec<NodeId> = candidates
            .iter()
            .copied()
            .filter(|&c| candidates.iter().all(|&x| self.leq(x, c)))
            .collect();
        match maximal.as_slice() {
            [single] => Some(*single),
            _ => None,
        }
    }
}

/// Sort information consulted by [`unify_sorted`]: the subsort order, each
/// leaf/free term's own sort, and each function symbol's result sort.
#[derive(Debug, Default, Clone)]
pub struct SortContext {
    /// The subsort order.
    order: SortOrder,
    /// A resolved `Leaf`/`Free` node's declared sort.
    term_sorts: BTreeMap<NodeId, NodeId>,
    /// An `App` node's operator's declared result sort.
    op_result_sort: BTreeMap<NodeId, NodeId>,
}

impl SortContext {
    /// A sort context from its three components.
    pub fn new(
        order: SortOrder,
        term_sorts: BTreeMap<NodeId, NodeId>,
        op_result_sort: BTreeMap<NodeId, NodeId>,
    ) -> Self {
        Self {
            order,
            term_sorts,
            op_result_sort,
        }
    }

    /// The subsort order this context consults.
    pub fn order(&self) -> &SortOrder {
        &self.order
    }

    /// `node`'s sort, if this context can determine one.
    ///
    /// `node` is resolved through `s` first. A resolved `Leaf`/`Free` looks up
    /// `term_sorts`; a resolved `App` looks up `op_result_sort` by its operator; a
    /// resolved (still-unbound) `Meta` looks up its own declared sort in `s`;
    /// anything else (`Bound`, `Binder`) has no sort here, so this returns `None`
    /// — which is also exactly what [`SortContext::default`] does for every node,
    /// making `unify_sorted` under a default context behave identically to plain
    /// [`unify`].
    pub fn sort_of(&self, dag: &TermDag, node: NodeId, s: &Subst) -> Option<NodeId> {
        let resolved = s.resolve(dag, node);
        match dag.data(resolved) {
            NodeData::Leaf(_) | NodeData::Free(_) => self.term_sorts.get(&resolved).copied(),
            NodeData::App { op, .. } => self.op_result_sort.get(op).copied(),
            NodeData::Meta(m) => s.meta_sort(*m),
            NodeData::Bound { .. } | NodeData::Binder { .. } => None,
        }
    }
}

// ── The unification core ─────────────────────────────────────────────────────────

/// Unify `a` and `b`, extending `s` on success.
///
/// Transactional: on any outcome other than [`Unified::Ok`], `s` is left exactly
/// as it was found. See the [module docs](self).
pub fn unify(dag: &mut TermDag, a: NodeId, b: NodeId, s: &mut Subst) -> Unified {
    let checkpoint = s.clone();
    let result = unify_at(dag, a, b, s, 0, None);
    if !matches!(result, Unified::Ok) {
        *s = checkpoint;
    }
    result
}

/// Unify `a` and `b` under an order-sorted discipline, extending `s` on success.
///
/// Identical to [`unify`] except that binding a metavariable additionally checks
/// `ctx` for a sort violation. Transactional, exactly like [`unify`].
pub fn unify_sorted(
    dag: &mut TermDag,
    a: NodeId,
    b: NodeId,
    s: &mut Subst,
    ctx: &SortContext,
) -> Unified {
    let checkpoint = s.clone();
    let result = unify_at(dag, a, b, s, 0, Some(ctx));
    if !matches!(result, Unified::Ok) {
        *s = checkpoint;
    }
    result
}

/// The shared recursive core behind [`unify`] and [`unify_sorted`].
///
/// `depth` counts binders crossed since the top-level call, which is what lets
/// `whnf` and `bind_meta` correctly lift/lower a metavariable's stored
/// solution across them. `ctx` is `None` for plain unification and `Some` for
/// order-sorted unification; passing it through explicitly (rather than
/// duplicating this whole function) keeps the two entry points from drifting
/// apart.
fn unify_at(
    dag: &mut TermDag,
    a: NodeId,
    b: NodeId,
    s: &mut Subst,
    depth: u32,
    ctx: Option<&SortContext>,
) -> Unified {
    let a = whnf(dag, s, a, depth);
    let b = whnf(dag, s, b, depth);
    if a == b {
        // Hash-consing gives this short-circuit for free, including alpha-
        // equivalent binders: the locally-nameless encoding makes them literally
        // the same node.
        return Unified::Ok;
    }
    if let NodeData::Meta(m) = *dag.data(a) {
        return bind_meta(dag, s, m, a, b, depth, ctx);
    }
    if let NodeData::Meta(m) = *dag.data(b) {
        return bind_meta(dag, s, m, b, a, depth, ctx);
    }
    match (dag.data(a).clone(), dag.data(b).clone()) {
        (
            NodeData::App {
                op: op_a,
                args: args_a,
            },
            NodeData::App {
                op: op_b,
                args: args_b,
            },
        ) => {
            if op_a != op_b || args_a.len() != args_b.len() {
                return Unified::Clash { left: a, right: b };
            }
            let op_result = unify_at(dag, op_a, op_b, s, depth, ctx);
            if !matches!(op_result, Unified::Ok) {
                return op_result;
            }
            for (&x, &y) in args_a.iter().zip(args_b.iter()) {
                let result = unify_at(dag, x, y, s, depth, ctx);
                if !matches!(result, Unified::Ok) {
                    return result;
                }
            }
            Unified::Ok
        }
        (
            NodeData::Binder {
                op: op_a,
                sorts: sorts_a,
                body: body_a,
            },
            NodeData::Binder {
                op: op_b,
                sorts: sorts_b,
                body: body_b,
            },
        ) => {
            if op_a != op_b || sorts_a.len() != sorts_b.len() {
                return Unified::Clash { left: a, right: b };
            }
            let op_result = unify_at(dag, op_a, op_b, s, depth, ctx);
            if !matches!(op_result, Unified::Ok) {
                return op_result;
            }
            for (&x, &y) in sorts_a.iter().zip(sorts_b.iter()) {
                let result = unify_at(dag, x, y, s, depth, ctx);
                if !matches!(result, Unified::Ok) {
                    return result;
                }
            }
            unify_at(dag, body_a, body_b, s, depth + 1, ctx)
        }
        _ => Unified::Clash { left: a, right: b },
    }
}

/// Resolve `node` through `s` at ambient `depth`, unfolding at most the outermost
/// metavariable chain.
///
/// If `node` resolves to an unbound metavariable, that meta node is returned
/// unchanged. If it resolves through one or more bindings to a non-meta solution,
/// that solution was recorded at the metavariable's OWN home depth (ambient depth
/// 0 relative to where it was minted — see `bind_meta`), so it is LIFTED by
/// `depth` via [`shift`] before being returned: this is what makes unification
/// correct underneath binders, a metavariable bound once outside a binder can
/// still be compared against structure found several binders deep. If `node` was
/// never a metavariable to begin with, [`Subst::resolve`] is the identity and no
/// lift is applied.
fn whnf(dag: &mut TermDag, s: &Subst, node: NodeId, depth: u32) -> NodeId {
    let was_meta = matches!(dag.data(node), NodeData::Meta(_));
    let resolved = s.resolve(dag, node);
    if !was_meta || matches!(dag.data(resolved), NodeData::Meta(_)) {
        return resolved;
    }
    shift(dag, resolved, depth)
}

/// Bind metavariable `m` (denoted by the already-whnf'd `meta_node`) to `t`
/// (already whnf'd at the same `depth`), or report why no binding is possible.
fn bind_meta(
    dag: &mut TermDag,
    s: &mut Subst,
    m: MetaId,
    meta_node: NodeId,
    t: NodeId,
    depth: u32,
    ctx: Option<&SortContext>,
) -> Unified {
    // Binding a meta to itself is vacuous.
    if let NodeData::Meta(m2) = *dag.data(t)
        && m2 == m
    {
        return Unified::Ok;
    }

    if occurs_through(s, dag, m, t) {
        return Unified::Occurs {
            meta: m,
            in_node: t,
        };
    }

    // `t` lives at ambient `depth`; `m`'s home is depth 0, so the candidate
    // solution must be lowered. Failure is a genuine scope escape: a sound
    // clash, not a bug.
    let Some(lowered) = shift_down(dag, t, depth) else {
        return Unified::Clash {
            left: meta_node,
            right: t,
        };
    };

    if let Some(ctx) = ctx {
        if let NodeData::Meta(m2) = *dag.data(lowered) {
            if let (Some(mine), Some(theirs)) = (s.meta_sort(m), s.meta_sort(m2))
                && ctx.order().meet(mine, theirs).is_none()
            {
                return Unified::Clash {
                    left: meta_node,
                    right: t,
                };
            }
        } else if let (Some(mine), Some(theirs)) = (s.meta_sort(m), ctx.sort_of(dag, lowered, s))
            && !ctx.order().leq(theirs, mine)
        {
            return Unified::Clash {
                left: meta_node,
                right: t,
            };
        }
    }

    s.bind(m, lowered);
    Unified::Ok
}

/// Whether `m` occurs anywhere in `node`, following the current bindings of `s`.
///
/// The free-meta cache is a purely STRUCTURAL/syntactic property of `node` as
/// built — it says nothing about what any of those metavariables are currently
/// bound to. So the cache is a trustworthy fast path only when every meta it
/// lists is currently UNBOUND: in that case the syntactic structure IS the fully
/// resolved structure, and a single binary search decides membership in either
/// direction (`true` if `m` is listed, `false` otherwise). The moment some cached
/// meta IS bound, the cache can no longer be trusted for a `false` verdict
/// either — that meta's binding might reach `m` through structure the cache
/// never recorded (exactly the scenario a chain of bindings like `a := f(b)`
/// then `b := g(a)` produces) — so this falls back to a full structural walk
/// that follows every bound meta's current value.
fn occurs_through(s: &Subst, dag: &TermDag, m: MetaId, node: NodeId) -> bool {
    let cached = dag.free_meta(node);
    if cached.iter().all(|&fm| !s.is_bound(fm)) {
        return cached.binary_search(&m).is_ok();
    }
    match dag.data(node) {
        NodeData::Meta(m2) => {
            *m2 == m
                || s.get(*m2)
                    .is_some_and(|bound| occurs_through(s, dag, m, bound))
        }
        NodeData::App { op, args } => {
            occurs_through(s, dag, m, *op) || args.iter().any(|&arg| occurs_through(s, dag, m, arg))
        }
        NodeData::Binder { op, sorts, body } => {
            occurs_through(s, dag, m, *op)
                || sorts.iter().any(|&sort| occurs_through(s, dag, m, sort))
                || occurs_through(s, dag, m, *body)
        }
        NodeData::Leaf(_) | NodeData::Free(_) | NodeData::Bound { .. } => false,
    }
}

// ── Substitution materialization ─────────────────────────────────────────────────

/// Re-intern `node` with every currently-bound metavariable replaced by its
/// (recursively applied) solution.
///
/// Memoized by `(NodeId, depth)` to stay linear in the DAG. Splicing a bound
/// meta's solution in at a depth that has crossed some binders since the
/// top-level call requires [`shift`]-ing the solution up first, for the same
/// capture-avoidance reason `whnf` does.
pub fn apply(dag: &mut TermDag, s: &Subst, node: NodeId) -> NodeId {
    let mut memo = BTreeMap::new();
    apply_at(dag, s, node, 0, &mut memo)
}

/// The recursive core of [`apply`].
fn apply_at(
    dag: &mut TermDag,
    s: &Subst,
    node: NodeId,
    depth: u32,
    memo: &mut BTreeMap<(NodeId, u32), NodeId>,
) -> NodeId {
    if let Some(&cached) = memo.get(&(node, depth)) {
        return cached;
    }
    let result = match dag.data(node).clone() {
        NodeData::Leaf(_) | NodeData::Free(_) | NodeData::Bound { .. } => node,
        NodeData::Meta(_) => {
            let resolved = s.resolve(dag, node);
            if resolved == node {
                node
            } else {
                let shifted = shift(dag, resolved, depth);
                apply_at(dag, s, shifted, depth, memo)
            }
        }
        NodeData::App { op, args } => {
            let new_op = apply_at(dag, s, op, depth, memo);
            let new_args: Vec<NodeId> = args
                .iter()
                .map(|&arg| apply_at(dag, s, arg, depth, memo))
                .collect();
            dag.intern_app(new_op, new_args)
        }
        NodeData::Binder { op, sorts, body } => {
            let new_op = apply_at(dag, s, op, depth, memo);
            let new_sorts: Vec<NodeId> = sorts
                .iter()
                .map(|&sort| apply_at(dag, s, sort, depth, memo))
                .collect();
            let new_body = apply_at(dag, s, body, depth + 1, memo);
            dag.intern_binder(new_op, new_sorts, new_body)
        }
    };
    memo.insert((node, depth), result);
    result
}

/// Lift every occurrence of a `Bound` variable free at cutoff 0 (i.e. every
/// `debruijn >= cutoff`, where `cutoff` starts at 0 and increases by one per
/// `Binder` crossed during the descent) by `by`.
///
/// `Leaf`, `Free` and `Meta` nodes are unaffected: metavariables are not scoped by
/// depth at all (they live at the ambient top level by convention — see
/// `bind_meta`), which is exactly why `whnf` must shift a meta's stored
/// solution by the CURRENT depth every time it unfolds one, rather than storing a
/// separate copy per depth. Memoized by `(NodeId, cutoff)`.
pub fn shift(dag: &mut TermDag, node: NodeId, by: u32) -> NodeId {
    if by == 0 {
        return node;
    }
    let mut memo = BTreeMap::new();
    shift_at(dag, node, by, 0, &mut memo)
}

/// The recursive core of [`shift`].
fn shift_at(
    dag: &mut TermDag,
    node: NodeId,
    by: u32,
    cutoff: u32,
    memo: &mut BTreeMap<(NodeId, u32), NodeId>,
) -> NodeId {
    if let Some(&cached) = memo.get(&(node, cutoff)) {
        return cached;
    }
    let result = match dag.data(node).clone() {
        NodeData::Leaf(_) | NodeData::Free(_) | NodeData::Meta(_) => node,
        NodeData::Bound { debruijn, slot } => {
            if debruijn >= cutoff {
                dag.intern_bound(debruijn + by, slot)
            } else {
                node
            }
        }
        NodeData::App { op, args } => {
            let new_op = shift_at(dag, op, by, cutoff, memo);
            let new_args: Vec<NodeId> = args
                .iter()
                .map(|&arg| shift_at(dag, arg, by, cutoff, memo))
                .collect();
            dag.intern_app(new_op, new_args)
        }
        NodeData::Binder { op, sorts, body } => {
            let new_op = shift_at(dag, op, by, cutoff, memo);
            let new_sorts: Vec<NodeId> = sorts
                .iter()
                .map(|&sort| shift_at(dag, sort, by, cutoff, memo))
                .collect();
            let new_body = shift_at(dag, body, by, cutoff + 1, memo);
            dag.intern_binder(new_op, new_sorts, new_body)
        }
    };
    memo.insert((node, cutoff), result);
    result
}

/// The inverse of [`shift`]: lower every `Bound` occurrence free at cutoff 0 by
/// `by`, or report `None` if some such occurrence refers to one of the `by`
/// binders being stripped away (a genuine scope escape — the bound variable
/// cannot be expressed at the outer depth).
fn shift_down(dag: &mut TermDag, node: NodeId, by: u32) -> Option<NodeId> {
    if by == 0 {
        return Some(node);
    }
    let mut memo = BTreeMap::new();
    shift_down_at(dag, node, by, 0, &mut memo)
}

/// The recursive core of [`shift_down`].
fn shift_down_at(
    dag: &mut TermDag,
    node: NodeId,
    by: u32,
    cutoff: u32,
    memo: &mut BTreeMap<(NodeId, u32), Option<NodeId>>,
) -> Option<NodeId> {
    if let Some(&cached) = memo.get(&(node, cutoff)) {
        return cached;
    }
    let result = match dag.data(node).clone() {
        NodeData::Leaf(_) | NodeData::Free(_) | NodeData::Meta(_) => Some(node),
        NodeData::Bound { debruijn, slot } => {
            if debruijn < cutoff {
                Some(node)
            } else if debruijn < cutoff + by {
                None
            } else {
                Some(dag.intern_bound(debruijn - by, slot))
            }
        }
        NodeData::App { op, args } => {
            let new_op = shift_down_at(dag, op, by, cutoff, memo)?;
            let mut new_args = Vec::with_capacity(args.len());
            for &arg in &args {
                new_args.push(shift_down_at(dag, arg, by, cutoff, memo)?);
            }
            Some(dag.intern_app(new_op, new_args))
        }
        NodeData::Binder { op, sorts, body } => {
            let new_op = shift_down_at(dag, op, by, cutoff, memo)?;
            let mut new_sorts = Vec::with_capacity(sorts.len());
            for &sort in &sorts {
                new_sorts.push(shift_down_at(dag, sort, by, cutoff, memo)?);
            }
            let new_body = shift_down_at(dag, body, by, cutoff + 1, memo)?;
            Some(dag.intern_binder(new_op, new_sorts, new_body))
        }
    };
    memo.insert((node, cutoff), result);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 2-arg application `f(x, y)`.
    fn app2(dag: &mut TermDag, f: &str, x: NodeId, y: NodeId) -> NodeId {
        let op = dag.intern_leaf(f);
        dag.intern_app(op, vec![x, y])
    }

    /// A 1-arg application `f(x)`.
    fn app1(dag: &mut TermDag, f: &str, x: NodeId) -> NodeId {
        let op = dag.intern_leaf(f);
        dag.intern_app(op, vec![x])
    }

    // ── 1/2: occurs-check ────────────────────────────────────────────────────

    /// Binding a fresh meta to an application containing itself is `Occurs`;
    /// binding it to an application that does NOT contain it is `Ok`.
    #[test]
    fn occurs_check_rejects_cyclic_and_accepts_well_founded() {
        let mut dag = TermDag::new();
        let (_, meta) = dag.fresh_meta();
        let a = dag.intern_leaf("a");
        let cyclic = app1(&mut dag, "f", meta);
        let mut s = Subst::new();
        assert!(matches!(
            unify(&mut dag, meta, cyclic, &mut s),
            Unified::Occurs { .. }
        ));
        assert_eq!(s.bound_count(), 0);

        let mut s2 = Subst::new();
        let acyclic = app1(&mut dag, "f", a);
        assert_eq!(unify(&mut dag, meta, acyclic, &mut s2), Unified::Ok);
    }

    /// Meta `a` bound to something containing meta `b`; unifying `b` with
    /// something containing `a` must be rejected transitively, through the
    /// existing binding rather than by direct syntactic containment alone.
    #[test]
    fn occurs_check_is_sound_through_substitution() {
        let mut dag = TermDag::new();
        let (_, meta_a) = dag.fresh_meta();
        let (meta_b_id, meta_b) = dag.fresh_meta();
        let mut s = Subst::new();
        // a := f(b)
        let f_b = app1(&mut dag, "f", meta_b);
        assert_eq!(unify(&mut dag, meta_a, f_b, &mut s), Unified::Ok);
        // now unify b against g(a) — a's binding contains b, so this is cyclic.
        let g_a = app1(&mut dag, "g", meta_a);
        assert_eq!(
            unify(&mut dag, meta_b, g_a, &mut s),
            Unified::Occurs {
                meta: meta_b_id,
                in_node: g_a
            }
        );
    }

    // ── 3/4: transactional rollback ──────────────────────────────────────────

    /// A shallow clash after one successful arg-unification in a 2-arg
    /// application leaves NO bindings behind.
    #[test]
    fn failed_unification_after_partial_bind_leaves_no_bindings() {
        let mut dag = TermDag::new();
        let (_, meta) = dag.fresh_meta();
        let a = dag.intern_leaf("a");
        let b = dag.intern_leaf("b");
        let c = dag.intern_leaf("c");
        let left = app2(&mut dag, "f", meta, b);
        let right = app2(&mut dag, "f", a, c);
        let mut s = Subst::new();
        assert!(matches!(
            unify(&mut dag, left, right, &mut s),
            Unified::Clash { .. }
        ));
        assert_eq!(
            s.bound_count(),
            0,
            "the meta bound to `a` must be rolled back"
        );
    }

    /// Same property, three levels deep: `f(g(h(X)), b)` vs `f(g(h(a)), c)`
    /// succeeds three nested unifications before the outer clash.
    #[test]
    fn deep_nested_clash_after_partial_bind_leaves_no_bindings() {
        let mut dag = TermDag::new();
        let (_, meta) = dag.fresh_meta();
        let h_x = app1(&mut dag, "h", meta);
        let g_h_x = app1(&mut dag, "g", h_x);
        let b = dag.intern_leaf("b");
        let left = app2(&mut dag, "f", g_h_x, b);

        let a = dag.intern_leaf("a");
        let h_a = app1(&mut dag, "h", a);
        let g_h_a = app1(&mut dag, "g", h_a);
        let c = dag.intern_leaf("c");
        let right = app2(&mut dag, "f", g_h_a, c);

        let mut s = Subst::new();
        assert!(matches!(
            unify(&mut dag, left, right, &mut s),
            Unified::Clash { .. }
        ));
        assert_eq!(s.bound_count(), 0);
    }

    // ── 5: apply on ground terms ─────────────────────────────────────────────

    /// A term with no metavariables is unchanged by `apply`.
    #[test]
    fn apply_is_identity_on_ground_terms() {
        let mut dag = TermDag::new();
        let a = dag.intern_leaf("a");
        let b = dag.intern_leaf("b");
        let ground = app2(&mut dag, "f", a, b);
        let s = Subst::new();
        assert_eq!(apply(&mut dag, &s, ground), ground);
    }

    // ── 6/7: shift ────────────────────────────────────────────────────────────

    /// A `Binder`'s body containing `Bound{0,_}` (bound BY that binder) must not
    /// shift; a `Bound{1,_}` referring PAST the binder must shift.
    #[test]
    fn shift_lifts_only_free_bound_occurrences() {
        let mut dag = TermDag::new();
        let sort = dag.intern_leaf("s");
        let bound0 = dag.intern_bound(0, 0); // bound by the binder itself
        let bound1 = dag.intern_bound(1, 0); // refers past the binder
        let inner = app2(&mut dag, "pair", bound0, bound1);
        let op = dag.intern_leaf("forall");
        let binder = dag.intern_binder(op, vec![sort], inner);

        let shifted = shift(&mut dag, binder, 5);
        let NodeData::Binder { body, .. } = dag.data(shifted).clone() else {
            panic!("shift must preserve the Binder shape");
        };
        let NodeData::App { args, .. } = dag.data(body).clone() else {
            panic!("shift must preserve the App shape");
        };
        assert_eq!(
            dag.data(args[0]),
            &NodeData::Bound {
                debruijn: 0,
                slot: 0
            }
        );
        assert_eq!(
            dag.data(args[1]),
            &NodeData::Bound {
                debruijn: 6,
                slot: 0
            }
        );
    }

    /// A meta bound to a term containing `Bound{0,_}`, spliced under an EXTRA
    /// binder in the target, must have its bound index correctly shifted so it
    /// does not accidentally refer to the new binder.
    #[test]
    fn apply_under_binders_avoids_capture() {
        let mut dag = TermDag::new();
        let a = dag.intern_leaf("a");
        let (m, meta) = dag.fresh_meta();
        let mut s = Subst::new();
        s.bind_renaming(m, a); // meta := a (a ground constant, at home depth 0)

        // Build `binder[sort]. pair(Bound{0,0}, meta)`: the meta occurs one
        // binder deep, so applying the substitution must not touch the `Bound{0}`
        // occurrence and must correctly splice in `a` for the meta.
        let sort = dag.intern_leaf("s");
        let bound0 = dag.intern_bound(0, 0);
        let body = app2(&mut dag, "pair", bound0, meta);
        let op = dag.intern_leaf("forall");
        let binder = dag.intern_binder(op, vec![sort], body);

        let result = apply(&mut dag, &s, binder);
        let NodeData::Binder { body, .. } = dag.data(result).clone() else {
            panic!("apply must preserve the Binder shape");
        };
        let NodeData::App { args, .. } = dag.data(body).clone() else {
            panic!("apply must preserve the App shape");
        };
        assert_eq!(
            dag.data(args[0]),
            &NodeData::Bound {
                debruijn: 0,
                slot: 0
            },
            "the binder's own bound variable must be untouched"
        );
        assert_eq!(
            dag.data(args[1]),
            &NodeData::Leaf(*match dag.data(a) {
                NodeData::Leaf(sym) => sym,
                _ => unreachable!(),
            })
        );
    }

    // ── 8/9/10: binders in unification ──────────────────────────────────────

    /// Two separately-built, structurally identical (Bound-based) binder shapes
    /// hash-cons to the SAME `NodeId` and unify trivially.
    #[test]
    fn alpha_equivalent_binders_unify_trivially() {
        let mut dag = TermDag::new();
        let op = dag.intern_leaf("forall");
        let sort = dag.intern_leaf("s");
        let body1 = dag.intern_bound(0, 0);
        let first = dag.intern_binder(op, vec![sort], body1);

        let body2 = dag.intern_bound(0, 0);
        let second = dag.intern_binder(op, vec![sort], body2);

        assert_eq!(
            first, second,
            "alpha-equivalent binders hash-cons to one node"
        );
        let mut s = Subst::new();
        assert_eq!(unify(&mut dag, first, second, &mut s), Unified::Ok);
        assert_eq!(s.bound_count(), 0, "no metas involved, nothing to bind");
    }

    /// A binder whose body is a fresh meta unifies against a binder whose body is
    /// a ground leaf, binding the meta (after whnf/shift handling).
    #[test]
    fn binder_with_metavar_body_unifies_by_binding() {
        let mut dag = TermDag::new();
        let op = dag.intern_leaf("forall");
        let sort = dag.intern_leaf("s");
        let (m, meta) = dag.fresh_meta();
        let left = dag.intern_binder(op, vec![sort], meta);

        let leaf = dag.intern_leaf("c");
        let right = dag.intern_binder(op, vec![sort], leaf);

        let mut s = Subst::new();
        assert_eq!(unify(&mut dag, left, right, &mut s), Unified::Ok);
        assert_eq!(s.get(m), Some(leaf));
    }

    /// Attempting to bind a metavariable that lives OUTSIDE a binder to a node
    /// which, once inside the binder, would need to reference a `Bound`
    /// occurrence that cannot be expressed outside it, is a `Clash`
    /// (scope-escape), via `bind_meta`'s `shift_down` failure path.
    #[test]
    fn binder_body_capturing_bound_var_clashes() {
        let mut dag = TermDag::new();
        let op = dag.intern_leaf("forall");
        let sort = dag.intern_leaf("s");
        let (_, meta) = dag.fresh_meta();
        // left: binder[s]. meta   (meta lives at depth 0, home outside the binder)
        let left = dag.intern_binder(op, vec![sort], meta);
        // right: binder[s]. Bound{0,0}   (a variable genuinely local to the binder)
        let bound = dag.intern_bound(0, 0);
        let right = dag.intern_binder(op, vec![sort], bound);

        let mut s = Subst::new();
        assert!(matches!(
            unify(&mut dag, left, right, &mut s),
            Unified::Clash { .. }
        ));
        assert_eq!(s.bound_count(), 0);
    }

    // ── 11/12/13: order-sorted unification ──────────────────────────────────

    /// `Cat <: Animal <: Thing`; a meta declared `Animal` binds a `Cat`-sorted
    /// leaf, but not a leaf of an unrelated (incomparable) sort.
    #[test]
    fn subsort_metavar_binds_a_narrower_term_but_not_a_wider_one() {
        let mut dag = TermDag::new();
        let cat = dag.intern_leaf("Cat");
        let animal = dag.intern_leaf("Animal");
        let thing = dag.intern_leaf("Thing");
        let rock = dag.intern_leaf("Rock"); // an unrelated, incomparable sort
        let order = SortOrder::from_subclass_edges(&[(cat, animal), (animal, thing)]);

        let felix = dag.intern_leaf("felix");
        let pebble = dag.intern_leaf("pebble");
        let mut term_sorts = BTreeMap::new();
        term_sorts.insert(felix, cat);
        term_sorts.insert(pebble, rock);
        let ctx = SortContext::new(order, term_sorts, BTreeMap::new());

        let (m, meta) = dag.fresh_meta();
        let mut s = Subst::new();
        s.declare_meta_sort(m, animal);
        assert_eq!(
            unify_sorted(&mut dag, meta, felix, &mut s, &ctx),
            Unified::Ok
        );

        let mut s2 = Subst::new();
        s2.declare_meta_sort(m, animal);
        assert!(matches!(
            unify_sorted(&mut dag, meta, pebble, &mut s2, &ctx),
            Unified::Clash { .. }
        ));
    }

    /// A closed 3-level order computes `leq` correctly in both directions and a
    /// unique `meet`; two sorts with no common declared subsort have no meet.
    #[test]
    fn sort_order_closure_and_meet() {
        let mut dag = TermDag::new();
        let cat = dag.intern_leaf("Cat");
        let dog = dag.intern_leaf("Dog");
        let animal = dag.intern_leaf("Animal");
        let thing = dag.intern_leaf("Thing");
        let order =
            SortOrder::from_subclass_edges(&[(cat, animal), (dog, animal), (animal, thing)]);
        assert!(order.leq(cat, thing), "transitive closure: Cat <: Thing");
        assert!(order.leq(cat, cat), "reflexive");
        assert!(!order.leq(thing, cat));
        assert_eq!(order.meet(cat, animal), Some(cat));
        assert_eq!(
            order.meet(cat, dog),
            None,
            "Cat and Dog share no declared subsort"
        );
    }

    /// Two metavariables with declared, INCOMPARABLE sorts (no common subsort)
    /// clash rather than silently binding.
    #[test]
    fn incomparable_sorts_clash() {
        let mut dag = TermDag::new();
        let cat = dag.intern_leaf("Cat");
        let rock = dag.intern_leaf("Rock");
        let order = SortOrder::from_subclass_edges(&[]); // no relation at all
        let ctx = SortContext::new(order, BTreeMap::new(), BTreeMap::new());

        let (m1, meta1) = dag.fresh_meta();
        let (m2, meta2) = dag.fresh_meta();
        let mut s = Subst::new();
        s.declare_meta_sort(m1, cat);
        s.declare_meta_sort(m2, rock);
        assert!(matches!(
            unify_sorted(&mut dag, meta1, meta2, &mut s, &ctx),
            Unified::Clash { .. }
        ));
    }

    /// `unify_sorted` with `SortContext::default()` behaves identically to plain
    /// `unify` across a representative ok/clash/occurs case each.
    #[test]
    fn empty_sort_context_matches_the_unsorted_path() {
        let mut dag = TermDag::new();
        let ctx = SortContext::default();
        let a = dag.intern_leaf("a");
        let b = dag.intern_leaf("b");

        // ok
        let (_, meta) = dag.fresh_meta();
        let mut plain = Subst::new();
        let mut sorted = Subst::new();
        assert_eq!(
            unify(&mut dag, meta, a, &mut plain),
            unify_sorted(&mut dag, meta, a, &mut sorted, &ctx)
        );

        // clash
        let mut plain2 = Subst::new();
        let mut sorted2 = Subst::new();
        let r1 = unify(&mut dag, a, b, &mut plain2);
        let r2 = unify_sorted(&mut dag, a, b, &mut sorted2, &ctx);
        assert_eq!(r1, r2);

        // occurs
        let (_, meta2) = dag.fresh_meta();
        let cyclic = app1(&mut dag, "f", meta2);
        let mut plain3 = Subst::new();
        let mut sorted3 = Subst::new();
        let r3 = unify(&mut dag, meta2, cyclic, &mut plain3);
        let r4 = unify_sorted(&mut dag, meta2, cyclic, &mut sorted3, &ctx);
        assert_eq!(r3, r4);
    }
}
