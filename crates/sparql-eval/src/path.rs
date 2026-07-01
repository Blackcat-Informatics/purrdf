// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SPARQL property-path evaluation (S8 #914) — the wasm-safe in-engine runtime.
//!
//! A property path constrains two endpoints by a *relation* between them rather
//! than a single triple. This module evaluates the `Path` graph pattern entirely in
//! interned [`TermId`](purrdf_core::TermId) space over the same indexed
//! [`DatasetView::quads_for_pattern`] surface the BGP hot path uses, returning a
//! [`SolutionSeq`] over the path's variable endpoint(s) that composes through the
//! existing join machinery unchanged.
//!
//! ## The reachability primitive
//!
//! Evaluation follows the SPARQL 1.1 §18.1.7 ALP (arbitrary-length-path) shape: a
//! single direction-parameterised primitive
//! [`reach`]`(path, node, forward)` returns the set of nodes `y` such that
//! `(node, y)` is in the path relation (forward), or `(y, node)` is (backward).
//! Every operator is structural recursion over the path expression:
//!
//! - `^p` (`Reverse`) flips the direction flag.
//! - `p/q` (`Sequence`) chains: `reach(q, ·)` over each `reach(p, node)` (and the
//!   order swaps under backward evaluation so predecessors compose correctly).
//! - `p|q` (`Alternative`) unions both sub-relations.
//! - `p?` (`ZeroOrOne`) adds the zero-length identity `{node}`.
//! - `p*`/`p+` (`ZeroOrMore`/`OneOrMore`) take the transitive closure with a
//!   **visited-set guard on the endpoint frontier**, so cyclic graphs terminate.
//! - `p{n,m}` (`Range`, a PURRDF extension) is **k-fold composition unioned over
//!   `[n, m]`**, re-entrant per `k` (NOT one global visited set across `k`) so a
//!   node reachable at several repetition counts is reported for each — a single
//!   visited-guarded level-BFS would be *wrong* on cyclic graphs.
//! - `!(…)` (`NegatedPropertySet`) and `<any>`/`<any:ns>` (`Wildcard`, a PURRDF
//!   extension) scan any-predicate edges, filtering by the excluded set or the
//!   namespace prefix respectively.
//!
//! Determinism: every intermediate is a `BTreeSet<TermId>`, so the materialised
//! solution order is the dataset's `TermId` order over the frozen dataset — the
//! same canonical discipline the rest of the evaluator follows.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::rc::Rc;

use purrdf_core::{RdfDataset, TermId, TermRef};
use purrdf_sparql_algebra::{NamedNode, PropertyPathExpression, TermPattern, Variable};

use crate::convert::{ground_term_pattern_to_value, named_node_to_value};
use crate::dataset_spec::GraphScope;
use crate::error::EvalError;
use crate::eval::EvalCtx;
use crate::scratch::SolutionTerm;
use crate::solution::{Solution, SolutionSeq, VarSchema};
use crate::DetHashMap;

/// Excluded-predicate sets for every `NegatedPropertySet` in the path, resolved to
/// dataset ids ONCE per `eval_path` call and keyed by the excluded slice's data
/// pointer (stable for the immutable path AST).
type NegatedCache = BTreeMap<usize, BTreeSet<TermId>>;
type ReachKey = (usize, TermId, bool);
type ReachCache = RefCell<DetHashMap<ReachKey, Rc<BTreeSet<TermId>>>>;

/// The immutable, traversal-wide context shared by every `reach` recursion: the
/// frozen dataset, the active dataset graph scope (§13: a single graph, or a
/// `FROM`/`USING`-merged default graph), the once-resolved negated-set cache, and
/// a per-evaluation reachability memo. Bundling these keeps the recursive
/// path-evaluation signatures small.
struct PathCtx<'a> {
    dataset: &'a RdfDataset,
    scope: GraphScope,
    cache: NegatedCache,
    reach_cache: ReachCache,
}

/// Build a `NegatedCache` by walking `path` once and pre-resolving every
/// `NegatedPropertySet`'s excluded predicates to `TermId`s. The result is
/// threaded through all `reach`/`closure`/`step_negated` calls so that IRI
/// resolution is not repeated on every traversal step.
fn build_negated_cache(path: &PropertyPathExpression, dataset: &RdfDataset) -> NegatedCache {
    let mut cache = NegatedCache::new();
    collect_negated(path, dataset, &mut cache);
    cache
}

fn collect_negated(path: &PropertyPathExpression, dataset: &RdfDataset, cache: &mut NegatedCache) {
    use PropertyPathExpression as P;
    match path {
        P::NegatedPropertySet(ps) => {
            let key = ps.as_ptr() as usize;
            cache.entry(key).or_insert_with(|| {
                ps.iter()
                    .filter_map(|p| dataset.term_id_by_value(&named_node_to_value(p)))
                    .collect()
            });
        }
        P::Reverse(i) | P::ZeroOrOne(i) | P::ZeroOrMore(i) | P::OneOrMore(i) => {
            collect_negated(i, dataset, cache);
        }
        P::Range { inner, .. } => collect_negated(inner, dataset, cache),
        P::Sequence(a, b) | P::Alternative(a, b) => {
            collect_negated(a, dataset, cache);
            collect_negated(b, dataset, cache);
        }
        P::NamedNode(_) | P::Wildcard { .. } => {}
    }
}

/// Evaluate a property-path constraint `subject path object` to a multiset of
/// solutions over its variable endpoint(s).
///
/// The result schema is the variable endpoints in subject-then-object order
/// (deduplicated, so `?x p+ ?x` is a single column). A blank-node endpoint is an
/// anonymous variable that is projected away (like BGP, SPARQL §4.1.4); a ground
/// endpoint absent from the dataset makes the whole path empty.
pub(crate) fn eval_path(
    subject: &TermPattern,
    path: &PropertyPathExpression,
    object: &TermPattern,
    ctx: &mut EvalCtx<'_>,
) -> Result<SolutionSeq, EvalError> {
    let dataset = ctx.dataset;
    let scope = ctx.active_dataset.scope_for(ctx.active_graph);

    // The output schema is fixed by which endpoints are *visible* variables, and is
    // independent of whether a ground endpoint happens to be absent — so an empty
    // result still carries the right columns for downstream joins.
    let schema = path_schema(subject, object);
    let width = schema.len();
    let s_col = visible_var(subject).and_then(|v| schema.index_of(&v));
    let o_col = visible_var(object).and_then(|v| schema.index_of(&v));

    let s_end = resolve_end(subject, dataset)?;
    let o_end = resolve_end(object, dataset)?;

    // An absent ground endpoint cannot match anything.
    let (Some(s_end), Some(o_end)) = (s_end, o_end) else {
        return Ok(SolutionSeq::empty(Rc::new(schema)));
    };

    // Pre-resolve all NegatedPropertySet excluded predicates once for this eval call.
    let pctx = PathCtx {
        dataset,
        scope,
        cache: build_negated_cache(path, dataset),
        reach_cache: RefCell::new(DetHashMap::default()),
    };

    let mut rows: Vec<Solution> = Vec::new();
    let push_pair = |rows: &mut Vec<Solution>, s_id: Option<TermId>, o_id: Option<TermId>| {
        let mut row = vec![None; width];
        if let (Some(c), Some(id)) = (s_col, s_id) {
            row[c] = Some(SolutionTerm::Existing(id));
        }
        if let (Some(c), Some(id)) = (o_col, o_id) {
            row[c] = Some(SolutionTerm::Existing(id));
        }
        rows.push(row);
    };

    match (s_end, o_end) {
        // Both ground: an ASK-shaped membership test. The schema is empty, so a hit
        // is the unit solution (one row binding nothing) and a miss is no rows.
        (Endpoint::Bound(sid), Endpoint::Bound(oid)) => {
            if reach(path, sid, true, &pctx).contains(&oid) {
                rows.push(vec![None; width]);
            }
        }
        // Subject ground, object variable: walk forward from the subject.
        (Endpoint::Bound(sid), Endpoint::Free { .. }) => {
            for y in reach_cached(path, sid, true, &pctx).iter().copied() {
                push_pair(&mut rows, Some(sid), Some(y));
            }
        }
        // Object ground, subject variable: walk backward from the object.
        (Endpoint::Free { .. }, Endpoint::Bound(oid)) => {
            for x in reach_cached(path, oid, false, &pctx).iter().copied() {
                push_pair(&mut rows, Some(x), Some(oid));
            }
        }
        // Both variable: enumerate the node universe (so zero-length `*`/`?`/`{0,…}`
        // pairs isolated nodes with themselves) and walk forward from each. When the
        // two endpoints are the *same* variable, keep only the reflexive pairs.
        (Endpoint::Free { var: sv }, Endpoint::Free { var: ov }) => {
            let same = sv == ov;
            if same {
                // Reflexive paths (p*, p?, p{0,m}) admit the zero-length identity, so
                // every node trivially reaches itself — skip the reach call entirely.
                // Non-reflexive paths (p, p+, p{n,…} with n>0, etc.) require an actual
                // traversal to discover whether x cycles back to itself.
                let reflexive = path_is_reflexive(path);
                for x in node_universe(dataset, &pctx.scope) {
                    if reflexive || reach_cached(path, x, true, &pctx).contains(&x) {
                        push_pair(&mut rows, Some(x), Some(x));
                    }
                }
            } else {
                // PINNED: spec-mandated distinct-var enumeration — enumerate every node
                // in the universe and materialise all forward reachability. DO NOT alter.
                for x in node_universe(dataset, &pctx.scope) {
                    for y in reach_cached(path, x, true, &pctx).iter().copied() {
                        push_pair(&mut rows, Some(x), Some(y));
                    }
                }
            }
        }
    }

    Ok(SolutionSeq {
        schema: Rc::new(schema),
        rows,
    })
}

/// A resolved path endpoint: a ground dataset id, or a free (variable / blank)
/// position. A ground term absent from the dataset resolves to `None`.
enum Endpoint {
    /// A ground constant resolved to its dataset id.
    Bound(TermId),
    /// A free position — a real variable, or a blank node treated as an anonymous
    /// (projected-away) variable. The variable identity is carried so two free
    /// endpoints sharing a name evaluate the reflexive `?x p ?x` case.
    Free { var: Variable },
}

/// Resolve an endpoint term. `Ok(None)` = a ground constant absent from the
/// dataset (so the whole path is empty).
fn resolve_end(term: &TermPattern, dataset: &RdfDataset) -> Result<Option<Endpoint>, EvalError> {
    match term {
        TermPattern::Variable(v) => Ok(Some(Endpoint::Free { var: v.clone() })),
        // A blank node in a path endpoint is an anonymous variable (SPARQL §4.1.4):
        // give it a NUL-prefixed synthetic name (the grammar can never produce one),
        // so two distinct blank labels are distinct vars and a repeated label
        // co-refers, exactly as in a BGP.
        TermPattern::BlankNode(b) => Ok(Some(Endpoint::Free {
            var: Variable::new(format!("\u{0}bnode:{}", b.as_str())),
        })),
        other => {
            let value = ground_term_pattern_to_value(other)?;
            Ok(dataset.term_id_by_value(&value).map(Endpoint::Bound))
        }
    }
}

/// The output schema: the visible variable endpoints in subject-then-object order,
/// deduplicated (a repeated variable is one column).
fn path_schema(subject: &TermPattern, object: &TermPattern) -> VarSchema {
    let mut schema = VarSchema::new();
    if let Some(v) = visible_var(subject) {
        schema.push(v);
    }
    if let Some(v) = visible_var(object) {
        schema.push(v);
    }
    schema
}

/// The projectable variable an endpoint exposes, if any. Blank nodes (anonymous
/// variables) and ground terms expose none.
fn visible_var(term: &TermPattern) -> Option<Variable> {
    match term {
        TermPattern::Variable(v) => Some(v.clone()),
        _ => None,
    }
}

/// All terms that appear as a subject or object of a quad in the active-dataset scope
/// — the node universe for a both-endpoints-variable path (SPARQL §18.1.7). The
/// `BTreeSet` de-dupes endpoints, so a `FROM`-merged scope needs no extra triple dedup.
fn node_universe(dataset: &RdfDataset, scope: &GraphScope) -> BTreeSet<TermId> {
    let mut out = BTreeSet::new();
    scope.for_each_quad(dataset, None, None, None, |q| {
        out.insert(q.s);
        out.insert(q.o);
    });
    out
}

/// The nodes `y` such that `(node, y)` is in `path`'s relation when `forward`, or
/// `(y, node)` when `!forward`. The ALP primitive — structural recursion over the
/// path expression; cycle-safe and deterministic.
fn reach(
    path: &PropertyPathExpression,
    node: TermId,
    forward: bool,
    ctx: &PathCtx<'_>,
) -> BTreeSet<TermId> {
    reach_cached(path, node, forward, ctx).as_ref().clone()
}

fn reach_cached(
    path: &PropertyPathExpression,
    node: TermId,
    forward: bool,
    ctx: &PathCtx<'_>,
) -> Rc<BTreeSet<TermId>> {
    let key = (
        path as *const PropertyPathExpression as usize,
        node,
        forward,
    );
    if let Some(cached) = ctx.reach_cache.borrow().get(&key) {
        return cached.clone();
    }

    let result = Rc::new(reach_uncached(path, node, forward, ctx));
    ctx.reach_cache.borrow_mut().insert(key, result.clone());
    result
}

fn reach_uncached(
    path: &PropertyPathExpression,
    node: TermId,
    forward: bool,
    ctx: &PathCtx<'_>,
) -> BTreeSet<TermId> {
    use PropertyPathExpression as P;
    match path {
        P::NamedNode(p) => step_predicate(p, node, forward, ctx),
        P::Reverse(inner) => reach_cached(inner, node, !forward, ctx).as_ref().clone(),
        P::Sequence(a, b) => {
            // Forward: step `a` then `b`. Backward (predecessors): step `b` then `a`,
            // each backward — so the composition order swaps with the direction.
            let (first, second): (&P, &P) = if forward { (a, b) } else { (b, a) };
            let mut out = BTreeSet::new();
            let first_reach = reach_cached(first, node, forward, ctx);
            for mid in first_reach.iter().copied() {
                out.extend(reach_cached(second, mid, forward, ctx).iter().copied());
            }
            out
        }
        P::Alternative(a, b) => {
            let mut out = reach_cached(a, node, forward, ctx).as_ref().clone();
            out.extend(reach_cached(b, node, forward, ctx).iter().copied());
            out
        }
        P::ZeroOrOne(inner) => {
            let mut out = reach_cached(inner, node, forward, ctx).as_ref().clone();
            out.insert(node); // the zero-length step is the identity
            out
        }
        P::ZeroOrMore(inner) => {
            let mut out = closure(inner, node, forward, ctx);
            out.insert(node); // zero-length: every node reaches itself
            out
        }
        P::OneOrMore(inner) => closure(inner, node, forward, ctx),
        P::Range { inner, min, max } => range_reach(inner, node, forward, *min, *max, ctx),
        P::NegatedPropertySet(ps) => step_negated(ps, node, forward, ctx),
        P::Wildcard { namespace } => step_wildcard(namespace.as_ref(), node, forward, ctx),
    }
}

/// Whether `path` admits the zero-length identity, i.e. `reach(path, n, …)` always
/// contains `n` itself regardless of the graph. Mirrors the identity-insertion in
/// [`reach`] exactly:
///
/// - `ZeroOrMore` / `ZeroOrOne` — both unconditionally insert `node` (reflexive).
/// - `Range { min, .. }` — `range_reach` starts `current = {node}` at k=0 and emits
///   `current` into `out` as soon as `k >= min`; so `node` enters `out` iff `min == 0`.
/// - `Reverse(inner)` — only flips the direction flag; reflexivity is preserved.
/// - `Sequence(a, b)` — the zero-length identity passes through both sides, so both
///   must individually admit the identity.
/// - `Alternative(a, b)` — either sub-path suffices.
/// - Everything else (`NamedNode`, `OneOrMore`, `NegatedPropertySet`, `Wildcard`) is
///   non-reflexive: `OneOrMore` returns `closure` only (node is included iff it cycles
///   back to itself, which is not a static guarantee).
fn path_is_reflexive(path: &PropertyPathExpression) -> bool {
    use PropertyPathExpression as P;
    match path {
        P::ZeroOrMore(_) | P::ZeroOrOne(_) => true,
        P::Range { min, .. } => *min == 0,
        P::Reverse(inner) => path_is_reflexive(inner),
        P::Sequence(a, b) => path_is_reflexive(a) && path_is_reflexive(b),
        P::Alternative(a, b) => path_is_reflexive(a) || path_is_reflexive(b),
        P::NamedNode(_) | P::OneOrMore(_) | P::NegatedPropertySet(_) | P::Wildcard { .. } => false,
    }
}

/// One predicate hop. Forward: objects of `(node, p, ?)`; backward: subjects of
/// `(?, p, node)`. A predicate absent from the dataset yields nothing.
fn step_predicate(
    p: &NamedNode,
    node: TermId,
    forward: bool,
    ctx: &PathCtx<'_>,
) -> BTreeSet<TermId> {
    let Some(pid) = ctx.dataset.term_id_by_value(&named_node_to_value(p)) else {
        return BTreeSet::new();
    };
    let mut out = BTreeSet::new();
    if forward {
        ctx.scope
            .for_each_quad(ctx.dataset, Some(node), Some(pid), None, |q| {
                out.insert(q.o);
            });
    } else {
        ctx.scope
            .for_each_quad(ctx.dataset, None, Some(pid), Some(node), |q| {
                out.insert(q.s);
            });
    }
    out
}

/// `!(p1|…|pn)`: one hop along any predicate NOT in the excluded set.
/// Uses the pre-resolved `cache` to avoid re-resolving excluded IRIs on every call.
fn step_negated(
    excluded: &[NamedNode],
    node: TermId,
    forward: bool,
    ctx: &PathCtx<'_>,
) -> BTreeSet<TermId> {
    let excluded = &ctx.cache[&(excluded.as_ptr() as usize)];
    let mut out = BTreeSet::new();
    if forward {
        ctx.scope
            .for_each_quad(ctx.dataset, Some(node), None, None, |q| {
                if !excluded.contains(&q.p) {
                    out.insert(q.o);
                }
            });
    } else {
        ctx.scope
            .for_each_quad(ctx.dataset, None, None, Some(node), |q| {
                if !excluded.contains(&q.p) {
                    out.insert(q.s);
                }
            });
    }
    out
}

/// `<any>` / `<any:ns>`: one hop along any predicate, optionally restricted to
/// predicates whose IRI begins with the namespace prefix.
fn step_wildcard(
    namespace: Option<&NamedNode>,
    node: TermId,
    forward: bool,
    ctx: &PathCtx<'_>,
) -> BTreeSet<TermId> {
    let prefix = namespace.map(NamedNode::as_str);
    let pred_ok = |pid: TermId| -> bool {
        match prefix {
            None => true,
            Some(pfx) => {
                matches!(ctx.dataset.resolve(pid), TermRef::Iri(iri) if iri.starts_with(pfx))
            }
        }
    };
    let mut out = BTreeSet::new();
    if forward {
        ctx.scope
            .for_each_quad(ctx.dataset, Some(node), None, None, |q| {
                if pred_ok(q.p) {
                    out.insert(q.o);
                }
            });
    } else {
        ctx.scope
            .for_each_quad(ctx.dataset, None, None, Some(node), |q| {
                if pred_ok(q.p) {
                    out.insert(q.s);
                }
            });
    }
    out
}

/// The one-or-more transitive closure of `inner` from `node`: every node reachable
/// by applying `inner` at least once. The visited-set guards the endpoint frontier
/// so cyclic graphs terminate; `node` itself appears iff it is reachable from
/// itself via a cycle (the correct SPARQL `+` behaviour).
fn closure(
    inner: &PropertyPathExpression,
    node: TermId,
    forward: bool,
    ctx: &PathCtx<'_>,
) -> BTreeSet<TermId> {
    let mut result = BTreeSet::new();
    let mut visited = BTreeSet::new();
    let mut frontier: Vec<TermId> = reach_cached(inner, node, forward, ctx)
        .iter()
        .copied()
        .collect();
    while let Some(n) = frontier.pop() {
        if !visited.insert(n) {
            continue;
        }
        result.insert(n);
        for next in reach_cached(inner, n, forward, ctx).iter().copied() {
            if !visited.contains(&next) {
                frontier.push(next);
            }
        }
    }
    result
}

/// The one-or-more transitive closure of `inner` from the WHOLE `seeds` set in a
/// single joint traversal: every node reachable by applying `inner` at least once
/// from any seed. Equivalent to unioning `closure` over each seed, but visits each
/// node at most once (O(V+E), not O(|seeds|·(V+E))).
fn closure_multi(
    inner: &PropertyPathExpression,
    seeds: &BTreeSet<TermId>,
    forward: bool,
    ctx: &PathCtx<'_>,
) -> BTreeSet<TermId> {
    let mut result = BTreeSet::new();
    let mut visited = BTreeSet::new();
    let mut frontier: Vec<TermId> = Vec::new();
    for &s in seeds {
        frontier.extend(reach_cached(inner, s, forward, ctx).iter().copied());
    }
    while let Some(n) = frontier.pop() {
        if !visited.insert(n) {
            continue;
        }
        result.insert(n);
        for next in reach_cached(inner, n, forward, ctx).iter().copied() {
            if !visited.contains(&next) {
                frontier.push(next);
            }
        }
    }
    result
}

/// `inner{min,max}` — the union over `k ∈ [min, max]` of the nodes reachable in
/// **exactly** `k` applications of `inner`. The per-level frontier is a fresh set
/// (re-entrant per `k`), so a node reachable at multiple repetition counts is
/// reported. `max == None` (`{n,}`) applies `inner` exactly `min` times then takes
/// the `*`-closure of that frontier.
fn range_reach(
    inner: &PropertyPathExpression,
    node: TermId,
    forward: bool,
    min: u32,
    max: Option<u32>,
    ctx: &PathCtx<'_>,
) -> BTreeSet<TermId> {
    let mut out = BTreeSet::new();
    // `current` = nodes reachable in exactly `k` applications; k starts at 0.
    let mut current: BTreeSet<TermId> = BTreeSet::from([node]);
    for k in 0u32.. {
        if k >= min {
            out.extend(current.iter().copied());
        }
        match max {
            Some(m) if k >= m => break,
            None if k >= min => {
                // Unbounded tail: `*`-close from the exactly-`min` frontier in a
                // single joint traversal (avoids redundant per-seed re-traversal).
                out.extend(closure_multi(inner, &current, forward, ctx));
                break;
            }
            _ => {}
        }
        if current.is_empty() {
            break; // nothing further reachable; no higher level can add nodes
        }
        let mut next = BTreeSet::new();
        for n in &current {
            next.extend(reach_cached(inner, *n, forward, ctx).iter().copied());
        }
        current = next;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use purrdf_core::RdfDatasetBuilder;
    use purrdf_sparql_algebra::NamedNode;
    use std::sync::Arc;

    const EX: &str = "http://ex/";

    fn iri(local: &str) -> String {
        format!("{EX}{local}")
    }

    /// Build a directed graph over predicate `local`-named edges. Each edge is a
    /// `(subject_local, predicate_local, object_local)` triple.
    fn graph_of(edges: &[(&str, &str, &str)]) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        for (s, p, o) in edges {
            let s = b.intern_iri(iri(s));
            let p = b.intern_iri(iri(p));
            let o = b.intern_iri(iri(o));
            b.push_quad(s, p, o, None);
        }
        b.freeze().expect("freeze")
    }

    fn nn(local: &str) -> NamedNode {
        NamedNode::new_unchecked(iri(local))
    }

    fn named(local: &str) -> PropertyPathExpression {
        PropertyPathExpression::NamedNode(nn(local))
    }

    fn var(name: &str) -> TermPattern {
        TermPattern::Variable(Variable::new(name))
    }

    fn ground(local: &str) -> TermPattern {
        TermPattern::NamedNode(nn(local))
    }

    /// Resolve a dataset id to its IRI local name (tests use only IRIs).
    fn local_of(ds: &RdfDataset, id: TermId) -> String {
        match ds.resolve(id) {
            TermRef::Iri(s) => s.strip_prefix(EX).unwrap_or(s).to_owned(),
            other => format!("{other:?}"),
        }
    }

    /// Evaluate a path and materialise the named columns as local-name rows, sorted
    /// for order-insensitive multiset comparison.
    fn run(
        ds: &RdfDataset,
        subject: TermPattern,
        path: PropertyPathExpression,
        object: TermPattern,
        vars: &[&str],
    ) -> Vec<Vec<Option<String>>> {
        let mut ctx = EvalCtx::new(ds);
        let seq = eval_path(&subject, &path, &object, &mut ctx).expect("path eval");
        let cols: Vec<usize> = vars
            .iter()
            .map(|v| {
                seq.schema
                    .index_of(&Variable::new(*v))
                    .expect("var present")
            })
            .collect();
        let mut out: Vec<Vec<Option<String>>> = seq
            .rows
            .iter()
            .map(|row| {
                cols.iter()
                    .map(|&c| match row[c] {
                        Some(SolutionTerm::Existing(id)) => Some(local_of(ds, id)),
                        Some(SolutionTerm::Computed(_)) => {
                            panic!("path never mints computed terms")
                        }
                        None => None,
                    })
                    .collect()
            })
            .collect();
        out.sort_by_key(|row| format!("{row:?}"));
        out
    }

    /// The local-name set reachable from `start` along `path` (forward).
    fn reach_locals(
        ds: &RdfDataset,
        path: &PropertyPathExpression,
        start: &str,
        forward: bool,
    ) -> Vec<String> {
        let sid = ds
            .term_id_by_value(&named_node_to_value(&nn(start)))
            .expect("start present");
        let pctx = PathCtx {
            dataset: ds,
            scope: GraphScope::One(purrdf_core::GraphMatch::Default),
            cache: build_negated_cache(path, ds),
            reach_cache: RefCell::new(DetHashMap::default()),
        };
        let mut v: Vec<String> = reach(path, sid, forward, &pctx)
            .into_iter()
            .map(|id| local_of(ds, id))
            .collect();
        v.sort();
        v
    }

    fn col1(vals: &[&str]) -> Vec<Vec<Option<String>>> {
        let mut rows: Vec<Vec<Option<String>>> =
            vals.iter().map(|v| vec![Some((*v).to_owned())]).collect();
        rows.sort_by_key(|row| format!("{row:?}"));
        rows
    }

    // ---- single predicate, sequence, alternative, reverse ------------------

    #[test]
    fn named_predicate_forward_and_reverse() {
        let ds = graph_of(&[("a", "p", "b"), ("a", "p", "c")]);
        // { :a :p ?o }
        let rows = run(&ds, ground("a"), named("p"), var("o"), &["o"]);
        assert_eq!(rows, col1(&["b", "c"]));
        // { :b ^:p ?s }  → inverse: ?s is anything that points to :b via :p, i.e. :a
        // (`:b ^:p ?s` ⟺ `?s :p :b`).
        let rev = PropertyPathExpression::Reverse(Box::new(named("p")));
        let rows = run(&ds, ground("b"), rev, var("s"), &["s"]);
        assert_eq!(rows, col1(&["a"]));
    }

    #[test]
    fn sequence_chains_two_predicates() {
        let ds = graph_of(&[("a", "p", "x"), ("x", "q", "b"), ("x", "q", "c")]);
        // :a :p/:q ?o → b, c
        let seq = PropertyPathExpression::Sequence(Box::new(named("p")), Box::new(named("q")));
        let rows = run(&ds, ground("a"), seq, var("o"), &["o"]);
        assert_eq!(rows, col1(&["b", "c"]));
    }

    #[test]
    fn sequence_backward_from_object() {
        let ds = graph_of(&[("a", "p", "x"), ("x", "q", "b")]);
        // ?s :p/:q :b  → a
        let seq = PropertyPathExpression::Sequence(Box::new(named("p")), Box::new(named("q")));
        let rows = run(&ds, var("s"), seq, ground("b"), &["s"]);
        assert_eq!(rows, col1(&["a"]));
    }

    #[test]
    fn alternative_unions_both() {
        let ds = graph_of(&[("a", "p", "b"), ("a", "q", "c")]);
        let alt = PropertyPathExpression::Alternative(Box::new(named("p")), Box::new(named("q")));
        let rows = run(&ds, ground("a"), alt, var("o"), &["o"]);
        assert_eq!(rows, col1(&["b", "c"]));
    }

    // ---- repetition: *, +, ? -----------------------------------------------

    #[test]
    fn zero_or_more_includes_self_and_transitive() {
        // a -> b -> c -> d (chain)
        let ds = graph_of(&[("a", "p", "b"), ("b", "p", "c"), ("c", "p", "d")]);
        let star = PropertyPathExpression::ZeroOrMore(Box::new(named("p")));
        assert_eq!(
            reach_locals(&ds, &star, "a", true),
            vec!["a", "b", "c", "d"]
        );
        let plus = PropertyPathExpression::OneOrMore(Box::new(named("p")));
        assert_eq!(reach_locals(&ds, &plus, "a", true), vec!["b", "c", "d"]);
        let opt = PropertyPathExpression::ZeroOrOne(Box::new(named("p")));
        assert_eq!(reach_locals(&ds, &opt, "a", true), vec!["a", "b"]);
    }

    #[test]
    fn one_or_more_includes_start_only_via_cycle() {
        // Cyclic a -> b -> c -> a: every node is reachable from itself.
        let cyclic = graph_of(&[("a", "p", "b"), ("b", "p", "c"), ("c", "p", "a")]);
        let plus = PropertyPathExpression::OneOrMore(Box::new(named("p")));
        assert_eq!(
            reach_locals(&cyclic, &plus, "a", true),
            vec!["a", "b", "c"],
            "in a cycle, a is reachable from itself via p+"
        );
        // Acyclic chain: a is NOT reachable from itself.
        let acyclic = graph_of(&[("a", "p", "b"), ("b", "p", "c")]);
        assert_eq!(
            reach_locals(&acyclic, &plus, "a", true),
            vec!["b", "c"],
            "acyclic: a is not in a+"
        );
    }

    #[test]
    fn star_terminates_on_a_cycle() {
        let cyclic = graph_of(&[("a", "p", "b"), ("b", "p", "c"), ("c", "p", "a")]);
        let star = PropertyPathExpression::ZeroOrMore(Box::new(named("p")));
        assert_eq!(reach_locals(&cyclic, &star, "a", true), vec!["a", "b", "c"]);
    }

    #[test]
    fn composite_step_cycle_terminates_and_reports() {
        // Cycle closed by a composite step: a -p-> x -q-> a. (p/q)+ from a must
        // terminate and report a (a reaches itself in one (p/q) application).
        let ds = graph_of(&[("a", "p", "x"), ("x", "q", "a")]);
        let seq = PropertyPathExpression::Sequence(Box::new(named("p")), Box::new(named("q")));
        let plus = PropertyPathExpression::OneOrMore(Box::new(seq.clone()));
        assert_eq!(reach_locals(&ds, &plus, "a", true), vec!["a"]);
        let star = PropertyPathExpression::ZeroOrMore(Box::new(seq));
        assert_eq!(reach_locals(&ds, &star, "a", true), vec!["a"]);
    }

    // ---- Range {n,m} (PURRDF extension), including on cycles -----------------

    #[test]
    fn range_exact_and_bounded_on_chain() {
        // a -> b -> c -> d -> e
        let ds = graph_of(&[
            ("a", "p", "b"),
            ("b", "p", "c"),
            ("c", "p", "d"),
            ("d", "p", "e"),
        ]);
        let rng = |min, max| PropertyPathExpression::Range {
            inner: Box::new(named("p")),
            min,
            max,
        };
        // {0,2}: self + up to 2 hops.
        assert_eq!(
            reach_locals(&ds, &rng(0, Some(2)), "a", true),
            vec!["a", "b", "c"]
        );
        // {2}: exactly two hops.
        assert_eq!(reach_locals(&ds, &rng(2, Some(2)), "a", true), vec!["c"]);
        // {2,}: two or more hops (unbounded tail).
        assert_eq!(
            reach_locals(&ds, &rng(2, None), "a", true),
            vec!["c", "d", "e"]
        );
    }

    #[test]
    fn range_on_cycle_reports_nodes_at_multiple_counts() {
        // 2-cycle a <-> b: from a, k applications land on a (even k) or b (odd k).
        // p{2,4} reaches a (at 2, 4) and b (at 3) — the case a single global
        // visited-set BFS would get wrong.
        let ds = graph_of(&[("a", "p", "b"), ("b", "p", "a")]);
        let rng = PropertyPathExpression::Range {
            inner: Box::new(named("p")),
            min: 2,
            max: Some(4),
        };
        assert_eq!(reach_locals(&ds, &rng, "a", true), vec!["a", "b"]);
    }

    // ---- negated property set & wildcard -----------------------------------

    #[test]
    fn negated_property_set_excludes_named() {
        let ds = graph_of(&[("a", "p", "b"), ("a", "q", "c"), ("a", "r", "d")]);
        // !(:p|:q) → only the :r edge.
        let neg = PropertyPathExpression::NegatedPropertySet(vec![nn("p"), nn("q")]);
        let rows = run(&ds, ground("a"), neg, var("o"), &["o"]);
        assert_eq!(rows, col1(&["d"]));
    }

    #[test]
    fn wildcard_any_and_namespace_scoped() {
        // Two predicate namespaces: http://ex/ and http://other/.
        let mut b = RdfDatasetBuilder::new();
        let a = b.intern_iri(iri("a"));
        let p = b.intern_iri(iri("p"));
        let other_p = b.intern_iri("http://other/p".to_owned());
        let x = b.intern_iri(iri("x"));
        let y = b.intern_iri(iri("y"));
        b.push_quad(a, p, x, None);
        b.push_quad(a, other_p, y, None);
        let ds = b.freeze().expect("freeze");

        // <any> → both objects.
        let any = PropertyPathExpression::Wildcard { namespace: None };
        let rows = run(&ds, ground("a"), any, var("o"), &["o"]);
        assert_eq!(rows, col1(&["x", "y"]));

        // <any:http://ex/> → only the ex-namespaced edge.
        let scoped = PropertyPathExpression::Wildcard {
            namespace: Some(NamedNode::new_unchecked(EX)),
        };
        let rows = run(&ds, ground("a"), scoped, var("o"), &["o"]);
        assert_eq!(rows, col1(&["x"]));
    }

    // ---- endpoint binding modes --------------------------------------------

    #[test]
    fn both_ground_is_ask_shaped() {
        let ds = graph_of(&[("a", "p", "b"), ("b", "p", "c")]);
        let plus = PropertyPathExpression::OneOrMore(Box::new(named("p")));
        // :a :p+ :c  → true (one unit solution).
        let mut ctx = EvalCtx::new(&ds);
        let hit = eval_path(&ground("a"), &plus, &ground("c"), &mut ctx).expect("eval");
        assert_eq!(hit.len(), 1);
        assert!(hit.schema.is_empty());
        // :a :p+ :a  → false (no solutions; acyclic).
        let mut ctx = EvalCtx::new(&ds);
        let miss = eval_path(&ground("a"), &plus, &ground("a"), &mut ctx).expect("eval");
        assert!(miss.is_empty());
    }

    #[test]
    fn both_variable_enumerates_pairs_with_zero_length_self_pairs() {
        // a -> b, plus an isolated edge c -> (nothing further). Node universe = {a,b,c}.
        let ds = graph_of(&[("a", "p", "b"), ("c", "q", "a")]);
        let star = PropertyPathExpression::ZeroOrMore(Box::new(named("p")));
        // ?s :p* ?o : every node pairs with itself (zero-length) + a→b transitive.
        let rows = run(&ds, var("s"), star, var("o"), &["s", "o"]);
        let mut expected = vec![
            vec![Some("a".to_owned()), Some("a".to_owned())],
            vec![Some("a".to_owned()), Some("b".to_owned())],
            vec![Some("b".to_owned()), Some("b".to_owned())],
            vec![Some("c".to_owned()), Some("c".to_owned())],
        ];
        expected.sort_by_key(|row| format!("{row:?}"));
        assert_eq!(rows, expected);
    }

    #[test]
    fn same_variable_keeps_only_reflexive_pairs() {
        // Cycle a -> b -> a: with p+, both a and b reach themselves.
        let ds = graph_of(&[("a", "p", "b"), ("b", "p", "a")]);
        let plus = PropertyPathExpression::OneOrMore(Box::new(named("p")));
        // ?x :p+ ?x  → a, b (each reaches itself via the cycle).
        let rows = run(&ds, var("x"), plus, var("x"), &["x"]);
        assert_eq!(rows, col1(&["a", "b"]));
    }

    // ---- same-variable reflexive short-circuit (Gap D) ---------------------

    #[test]
    fn same_var_reflexive_star() {
        // Graph a -> b -> c. Node universe = {a, b, c}.
        // ?x :p* ?x — p* is reflexive, so every node is a solution via zero-length identity.
        let ds = graph_of(&[("a", "p", "b"), ("b", "p", "c")]);
        let star = PropertyPathExpression::ZeroOrMore(Box::new(named("p")));
        let rows = run(&ds, var("x"), star, var("x"), &["x"]);
        assert_eq!(rows, col1(&["a", "b", "c"]));
    }

    #[test]
    fn same_var_reflexive_optional() {
        // Graph a -> b -> c. Node universe = {a, b, c}.
        // ?x :p? ?x — p? is reflexive, so every node is a solution via zero-length identity.
        let ds = graph_of(&[("a", "p", "b"), ("b", "p", "c")]);
        let opt = PropertyPathExpression::ZeroOrOne(Box::new(named("p")));
        let rows = run(&ds, var("x"), opt, var("x"), &["x"]);
        assert_eq!(rows, col1(&["a", "b", "c"]));
    }

    #[test]
    fn same_var_reflexive_range_zero_min() {
        // ?x :p{0,2} ?x — min=0 makes it reflexive; every node is a solution.
        let ds = graph_of(&[("a", "p", "b"), ("b", "p", "c")]);
        let rng = PropertyPathExpression::Range {
            inner: Box::new(named("p")),
            min: 0,
            max: Some(2),
        };
        let rows = run(&ds, var("x"), rng, var("x"), &["x"]);
        assert_eq!(rows, col1(&["a", "b", "c"]));
    }

    #[test]
    fn same_var_nonreflexive_no_cycle_is_empty() {
        // Acyclic a -> b -> c. ?x :p+ ?x — p+ is non-reflexive; no node cycles back.
        let ds = graph_of(&[("a", "p", "b"), ("b", "p", "c")]);
        let plus = PropertyPathExpression::OneOrMore(Box::new(named("p")));
        let rows = run(&ds, var("x"), plus, var("x"), &["x"]);
        assert_eq!(rows, col1(&[]));
    }

    #[test]
    fn absent_ground_endpoint_is_empty() {
        let ds = graph_of(&[("a", "p", "b")]);
        let plus = PropertyPathExpression::OneOrMore(Box::new(named("p")));
        // :nobody is not in the graph → empty, but the schema still carries ?o.
        let mut ctx = EvalCtx::new(&ds);
        let seq = eval_path(&ground("nobody"), &plus, &var("o"), &mut ctx).expect("eval");
        assert!(seq.is_empty());
        assert_eq!(seq.schema.vars(), &[Variable::new("o")]);
    }

    // ---- nested composition (corpus-shaped) --------------------------------

    #[test]
    fn nested_alternative_inverse_plus() {
        // Temporal-shaped: (:before | ^:after)+ — before-edges and reversed
        // after-edges, transitively. e1 before e2; e3 after e2 (so e2 ^after e3).
        let ds = graph_of(&[("e1", "before", "e2"), ("e3", "after", "e2")]);
        let alt = PropertyPathExpression::Alternative(
            Box::new(named("before")),
            Box::new(PropertyPathExpression::Reverse(Box::new(named("after")))),
        );
        let plus = PropertyPathExpression::OneOrMore(Box::new(alt));
        // From e1: e1 -before-> e2 -^after-> e3.
        assert_eq!(reach_locals(&ds, &plus, "e1", true), vec!["e2", "e3"]);
    }

    #[test]
    fn list_walk_members_rest_star_first() {
        // owl:members/rdf:rest*/rdf:first over a 3-element RDF list.
        let ds = graph_of(&[
            ("axiom", "members", "l0"),
            ("l0", "first", "A"),
            ("l0", "rest", "l1"),
            ("l1", "first", "B"),
            ("l1", "rest", "l2"),
            ("l2", "first", "C"),
            ("l2", "rest", "nil"),
        ]);
        // :axiom :members/:rest*/:first ?x → A, B, C
        let rest_star = PropertyPathExpression::ZeroOrMore(Box::new(named("rest")));
        let path = PropertyPathExpression::Sequence(
            Box::new(named("members")),
            Box::new(PropertyPathExpression::Sequence(
                Box::new(rest_star),
                Box::new(named("first")),
            )),
        );
        let rows = run(&ds, ground("axiom"), path, var("x"), &["x"]);
        assert_eq!(rows, col1(&["A", "B", "C"]));
    }

    #[test]
    fn determinism_rows_are_termid_ordered() {
        let ds = graph_of(&[("a", "p", "b"), ("b", "p", "c"), ("c", "p", "d")]);
        let star = PropertyPathExpression::ZeroOrMore(Box::new(named("p")));
        let mut ctx = EvalCtx::new(&ds);
        let first = eval_path(&ground("a"), &star, &var("o"), &mut ctx).expect("eval");
        let mut ctx = EvalCtx::new(&ds);
        let second = eval_path(&ground("a"), &star, &var("o"), &mut ctx).expect("eval");
        // Identical row order run-to-run (BTreeSet over TermId).
        let ids = |seq: &SolutionSeq| -> Vec<Option<SolutionTerm>> {
            seq.rows.iter().map(|r| r[0]).collect()
        };
        assert_eq!(ids(&first), ids(&second));
    }

    // ---- negated property set under transitive closure (Gap F) -------------

    #[test]
    fn negated_under_one_or_more() {
        // Graph: a -r-> b -r-> c, a -p-> x.
        // !(:p)+ from a: the negated step excludes :p so from a it follows :r to b,
        // then from b it follows :r to c. The :p edge is never followed.
        // Expected: {b, c}.
        let ds = graph_of(&[("a", "r", "b"), ("b", "r", "c"), ("a", "p", "x")]);
        let neg = PropertyPathExpression::NegatedPropertySet(vec![nn("p")]);
        let plus = PropertyPathExpression::OneOrMore(Box::new(neg));
        assert_eq!(reach_locals(&ds, &plus, "a", true), vec!["b", "c"]);
    }
}
