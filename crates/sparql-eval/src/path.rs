// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SPARQL property-path evaluation (S8) — the wasm-safe in-engine runtime.
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
//! single direction-parameterised primitive [`reach`], where
//! `reach(path, node, forward)` returns the set of nodes `y` such that
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
//! - `p{n,m}` (`Range`, a PurRDF extension) is **k-fold composition unioned over
//!   `[n, m]`**, re-entrant per `k` (NOT one global visited set across `k`) so a
//!   node reachable at several repetition counts is reported for each — a single
//!   visited-guarded level-BFS would be *wrong* on cyclic graphs.
//! - `!(…)` (`NegatedPropertySet`) and `<any>`/`<any:ns>` (`Wildcard`, a PurRDF
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
use std::sync::Arc;

use purrdf_core::{DatasetView, TermId, TermRef, TermValue, ViewTermId};
use purrdf_sparql_algebra::{NamedNode, PropertyPathExpression, TermPattern, Variable};

use crate::convert::{ground_term_pattern_to_value, named_node_to_value};
use crate::dataset_spec::GraphScope;
use crate::error::EvalError;
use crate::eval::EvalCtx;
use crate::scratch::SolutionTerm;
use crate::solution::{Solution, SolutionSeq, VarSchema};
use crate::{DetHashMap, DetHashSet};

/// The pre-resolved exclusion sets for one `NegatedPropertySet`, split by
/// element direction (SPARQL 1.1 §18.2/§18.3): `!(p1|^p2|...)` decomposes into
/// a forward-only negated step over the plain elements and a reverse-only
/// negated step over the `^`-elements, unioned. Each side is `None` when that
/// direction has NO elements at all — a direction with zero listed elements
/// contributes NOTHING to the union (it is omitted, not treated as "excludes
/// nothing so everything forward-matches"); `Some(empty set)` cannot occur
/// because an empty `TermId` set would only arise from a non-empty element
/// list whose IRIs are simply absent from the dataset, which still
/// legitimately participates (excluding nothing that occurs).
struct NegatedSets<I: ViewTermId = TermId> {
    /// Predicates excluded from a **forward** hop (the plain, non-`^` elements),
    /// or `None` if the set has no plain elements.
    forward: Option<BTreeSet<I>>,
    /// Predicates excluded from a **reverse** hop (the `^`-prefixed elements),
    /// or `None` if the set has no inverted elements.
    inverse: Option<BTreeSet<I>>,
}

/// Per-`NegatedPropertySet` exclusion sets, resolved to view ids ONCE per
/// `eval_path` call and keyed by the element slice's data pointer (stable for
/// the immutable path AST).
type NegatedCache<I = TermId> = BTreeMap<usize, NegatedSets<I>>;
type ReachKey<I = TermId> = (usize, I, bool);
type ReachCache<I = TermId> = RefCell<DetHashMap<ReachKey<I>, Rc<BTreeSet<I>>>>;

/// The immutable, traversal-wide context shared by every `reach` recursion: the
/// frozen dataset, the active dataset graph scope (§13: a single graph, or a
/// `FROM`/`USING`-merged default graph), the once-resolved negated-set cache, and
/// a per-evaluation reachability memo. Bundling these keeps the recursive
/// path-evaluation signatures small.
struct PathCtx<'a, D: DatasetView + Sync> {
    dataset: &'a D,
    scope: GraphScope<D::Id>,
    cache: NegatedCache<D::Id>,
    reach_cache: ReachCache<D::Id>,
    /// The live governor accounting of the execution this traversal belongs to, shared
    /// by [`Arc`] with the evaluation context rather than copied — a traversal that
    /// spent its own copy of the budget would let a query with `N` property paths spend
    /// `N` budgets.
    governors: Option<Arc<crate::governor::GovernorState>>,
    /// The per-node charge ledger and the ordinal of the path node being evaluated, when
    /// one is installed. A traversal's fuel is spent well below the operator boundary, so
    /// without this the single most graph-dependent cost in the evaluator would be the one
    /// cost an EXPLAIN could not attribute.
    ledger: Option<(Arc<crate::governor::ledger::ChargeLedger>, usize)>,
}

impl<D: DatasetView + Sync> PathCtx<'_, D> {
    /// The `path-frontier-expansion` charge point: one unit per frontier node expanded.
    ///
    /// A property path is the one place in the evaluator where the work is unbounded in
    /// the *graph* rather than in the query — a transitive closure over a cyclic graph
    /// re-enters the same nodes at every repetition count — so the frontier expansion,
    /// not the emitted row, is the quantity a budget has to bound. Returns `false` when
    /// the execution has stopped, which every traversal loop treats as "return what has
    /// been reached so far": a partially explored frontier reaches a subset of the truly
    /// reachable nodes, which is a sound lower bound.
    #[inline]
    fn charge_frontier(&self) -> bool {
        let Some(state) = self.governors.as_ref() else {
            return true;
        };
        let point = crate::governor::ChargePoint::PathFrontierExpansion;
        let charged = state.charge_point_if_engaged(point).is_ok();
        if charged && let Some((ledger, node)) = self.ledger.as_ref() {
            ledger.record_fuel(*node, point, point.cost());
        }
        charged
    }

    /// Whether a governor has already stopped this execution, so that anything computed
    /// from here on is a partial view of the graph rather than a finished one.
    ///
    /// One null test on an ungoverned traversal — which is every traversal that did not
    /// ask for a budget — and one write-once cell read on a governed one.
    #[inline]
    fn stopped(&self) -> bool {
        self.governors
            .as_ref()
            .is_some_and(|state| state.tripped().is_some())
    }
}

/// Build a `NegatedCache` by walking `path` once and pre-resolving every
/// `NegatedPropertySet`'s excluded predicates to `TermId`s. The result is
/// threaded through all `reach`/`closure`/`step_negated` calls so that IRI
/// resolution is not repeated on every traversal step.
fn build_negated_cache<D: DatasetView + Sync>(
    path: &PropertyPathExpression,
    dataset: &D,
) -> NegatedCache<D::Id> {
    let mut cache = NegatedCache::new();
    collect_negated(path, dataset, &mut cache);
    cache
}

fn collect_negated<D: DatasetView + Sync>(
    path: &PropertyPathExpression,
    dataset: &D,
    cache: &mut NegatedCache<D::Id>,
) {
    use PropertyPathExpression as P;
    match path {
        P::NegatedPropertySet(elems) => {
            let key = elems.as_ptr() as usize;
            cache.entry(key).or_insert_with(|| {
                let mut forward = None;
                let mut inverse = None;
                for e in elems {
                    let target = if e.inverse {
                        inverse.get_or_insert_with(BTreeSet::new)
                    } else {
                        forward.get_or_insert_with(BTreeSet::new)
                    };
                    if let Some(id) = dataset.term_id_by_value(&named_node_to_value(&e.predicate)) {
                        target.insert(id);
                    }
                }
                NegatedSets { forward, inverse }
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
/// anonymous variable that is projected away (like BGP, SPARQL §4.1.4).
///
/// A ground endpoint absent from the dataset (it is never the subject or object
/// of any quad — e.g. the whole graph is empty) is NOT automatically an empty
/// result: SPARQL 1.1's zero-length-path identity (`?`, `*`, `{0,…}`) matches a
/// node to itself regardless of whether that node happens to appear in any
/// triple, so `:o :p* :o` and `?s :p* :o` both still admit the trivial
/// self-pairing (W3C `property-path/zero_or_more_set_start` /
/// `zero_or_more_set_end`). A non-reflexive path cannot connect an absent node
/// to anything else (it has no edges to traverse), so it correctly stays empty.
/// # A leaf of the partial-lift channel
///
/// A property path has no sub-pattern, so there is no child truncation to compose: like a
/// basic graph pattern, it is where a truncation ORIGINATES rather than somewhere one
/// passes through, and the dispatch in [`crate::eval::eval`] wraps its result directly.
pub(crate) fn eval_path<D: DatasetView + Sync>(
    subject: &TermPattern,
    path: &PropertyPathExpression,
    object: &TermPattern,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<SolutionSeq<D::Id>, EvalError> {
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

    // Pre-resolve all NegatedPropertySet excluded predicates once for this eval call.
    let pctx = PathCtx {
        dataset,
        scope,
        cache: build_negated_cache(path, dataset),
        reach_cache: RefCell::new(DetHashMap::default()),
        governors: ctx.governor_state().map(Arc::clone),
        ledger: ctx
            .charge_ledger()
            .map(|ledger| (Arc::clone(ledger), ctx.ledger_node)),
    };

    // SPARQL 1.1 §18.3: a path with NO repetition operator (`*`/`+`/`?`/`{n,m}`)
    // anywhere in its tree evaluates as if unrolled into a BGP — each distinct
    // combination of matching triples is its own solution, so a node reachable
    // by several derivations (e.g. `pp11`'s `:p1/:p2` through two different
    // intermediates) surfaces as that many DUPLICATE result rows (a MULTISET).
    // A path containing repetition anywhere instead uses the ALP fixpoint
    // semantics (`reach`/`closure`), which is a SET of reachable nodes — no
    // duplicates, and required for termination on cyclic/infinite graphs. Both
    // shapes are unified behind `node_reach`, which returns a `Vec` either way
    // (with genuine duplicates in the bag case, and none in the set case).
    let bag = !path_has_repetition(path);
    let node_reach = |node: D::Id, forward: bool| -> Vec<D::Id> {
        if bag {
            simple_reach_multiset(path, node, forward, &pctx)
        } else {
            reach_cached(path, node, forward, &pctx)
                .iter()
                .copied()
                .collect()
        }
    };

    // The answer-cap / `LIMIT` pushdown's verdict for this path node: the number of output
    // rows past which nothing it produces can reach the query's answer. A path's emission
    // order is the traversal order below, and the loops that can be stopped are stopped at
    // the top of an iteration — so what is skipped is whole `node_reach` traversals, which
    // is where a path's cost lives, not merely the row that would have been pushed.
    let ceiling = ctx.row_ceiling().unwrap_or(usize::MAX);

    let mut rows: Vec<Solution<D::Id>> = Vec::new();
    let push_pair = |rows: &mut Vec<Solution<D::Id>>,
                     s_id: Option<SolutionTerm<D::Id>>,
                     o_id: Option<SolutionTerm<D::Id>>| {
        let mut row = smallvec::smallvec![None; width];
        if let (Some(c), Some(id)) = (s_col, s_id) {
            row[c] = Some(id);
        }
        if let (Some(c), Some(id)) = (o_col, o_id) {
            row[c] = Some(id);
        }
        rows.push(row);
    };

    match (s_end, o_end) {
        // Both ground: an ASK-shaped membership test. The schema is empty, so
        // each derivation reaching `oid` is its own unit solution (one empty
        // row) — for a SET path (`bag == false`) that count is always 0 or 1.
        (Endpoint::Bound(sid), Endpoint::Bound(oid)) => {
            let count = node_reach(sid, true).iter().filter(|&&y| y == oid).count();
            for _ in 0..count {
                rows.push(smallvec::smallvec![None; width]);
            }
        }
        // Both ground but absent from the dataset entirely: the only way they can
        // ever connect is the reflexive zero-length identity, when they are the
        // SAME term (an absent node has no edges to traverse for anything else).
        (Endpoint::BoundAbsent(sval), Endpoint::BoundAbsent(oval)) => {
            if sval == oval && path_is_reflexive(path) {
                rows.push(smallvec::smallvec![None; width]);
            }
        }
        // One side present in the dataset, the other absent: they cannot be the
        // same term (an equal value would have resolved to the same `TermId` on
        // both sides), and an absent node has no edges — so no path connects them.
        (Endpoint::Bound(_), Endpoint::BoundAbsent(_))
        | (Endpoint::BoundAbsent(_), Endpoint::Bound(_)) => {}
        // Subject ground, object variable: walk forward from the subject.
        (Endpoint::Bound(sid), Endpoint::Free { .. }) => {
            for y in node_reach(sid, true) {
                if rows.len() >= ceiling {
                    break;
                }
                push_pair(
                    &mut rows,
                    Some(SolutionTerm::Existing(sid)),
                    Some(SolutionTerm::Existing(y)),
                );
            }
        }
        // Subject ground but absent from the dataset, object variable: only the
        // zero-length reflexive pair (subject bound to itself) can ever match.
        (Endpoint::BoundAbsent(sval), Endpoint::Free { .. }) => {
            if path_is_reflexive(path) {
                let term = ctx.scratch.intern(dataset, sval);
                push_pair(&mut rows, Some(term), Some(term));
            }
        }
        // Object ground, subject variable: walk backward from the object.
        (Endpoint::Free { .. }, Endpoint::Bound(oid)) => {
            for x in node_reach(oid, false) {
                if rows.len() >= ceiling {
                    break;
                }
                push_pair(
                    &mut rows,
                    Some(SolutionTerm::Existing(x)),
                    Some(SolutionTerm::Existing(oid)),
                );
            }
        }
        // Object ground but absent from the dataset, subject variable: symmetric
        // to the subject-absent case above.
        (Endpoint::Free { .. }, Endpoint::BoundAbsent(oval)) => {
            if path_is_reflexive(path) {
                let term = ctx.scratch.intern(dataset, oval);
                push_pair(&mut rows, Some(term), Some(term));
            }
        }
        // Both variable: enumerate the node universe (so zero-length `*`/`?`/`{0,…}`
        // pairs isolated nodes with themselves) and walk forward from each. When the
        // two endpoints are the *same* variable, keep only the reflexive pairs.
        (Endpoint::Free { var: sv }, Endpoint::Free { var: ov }) => {
            let same = sv == ov;
            if same {
                // Reflexive paths (p*, p?, p{0,m}) admit the zero-length identity, so
                // every node trivially reaches itself — skip the reach call entirely
                // (and note `reflexive` is only ever true for a repetition path, i.e.
                // `bag == false`, so the multiset-count branch below never double-
                // counts a reflexive path's zero-length step). Non-reflexive paths
                // require an actual traversal to discover whether x cycles back to
                // itself — and for a bag path, count EACH derivation as its own row.
                let reflexive = path_is_reflexive(path);
                for x in node_universe(&pctx) {
                    if rows.len() >= ceiling {
                        break;
                    }
                    if reflexive {
                        push_pair(
                            &mut rows,
                            Some(SolutionTerm::Existing(x)),
                            Some(SolutionTerm::Existing(x)),
                        );
                    } else {
                        let count = node_reach(x, true).into_iter().filter(|&y| y == x).count();
                        for _ in 0..count {
                            push_pair(
                                &mut rows,
                                Some(SolutionTerm::Existing(x)),
                                Some(SolutionTerm::Existing(x)),
                            );
                        }
                    }
                }
            } else {
                // PINNED: spec-mandated distinct-var enumeration — enumerate every node
                // in the universe and materialise all forward reachability. DO NOT alter.
                for x in node_universe(&pctx) {
                    if rows.len() >= ceiling {
                        break;
                    }
                    for y in node_reach(x, true) {
                        if rows.len() >= ceiling {
                            break;
                        }
                        push_pair(
                            &mut rows,
                            Some(SolutionTerm::Existing(x)),
                            Some(SolutionTerm::Existing(y)),
                        );
                    }
                }
            }
        }
    }

    Ok(SolutionSeq {
        schema: Arc::new(schema),
        rows,
    })
}

/// A resolved path endpoint: a ground dataset id, a ground term absent from the
/// dataset, or a free (variable / blank) position.
enum Endpoint<I: ViewTermId = TermId> {
    /// A ground constant resolved to its dataset id.
    Bound(I),
    /// A ground constant that is not the subject or object of any quad in the
    /// dataset. Still a valid RDF term for the zero-length reflexive identity
    /// (see [`eval_path`]'s doc comment) — just not reachable by any real hop.
    BoundAbsent(TermValue),
    /// A free position — a real variable, or a blank node treated as an anonymous
    /// (projected-away) variable. The variable identity is carried so two free
    /// endpoints sharing a name evaluate the reflexive `?x p ?x` case.
    Free { var: Variable },
}

/// Resolve an endpoint term to a [`Endpoint`].
fn resolve_end<D: DatasetView + Sync>(
    term: &TermPattern,
    dataset: &D,
) -> Result<Endpoint<D::Id>, EvalError> {
    match term {
        TermPattern::Variable(v) => Ok(Endpoint::Free { var: v.clone() }),
        // A blank node in a path endpoint is an anonymous variable (SPARQL §4.1.4):
        // give it a NUL-prefixed synthetic name (the grammar can never produce one),
        // so two distinct blank labels are distinct vars and a repeated label
        // co-refers, exactly as in a BGP.
        TermPattern::BlankNode(b) => Ok(Endpoint::Free {
            var: Variable::new(format!("\u{0}bnode:{}", b.as_str())),
        }),
        other => {
            let value = ground_term_pattern_to_value(other)?;
            Ok(match dataset.term_id_by_value(&value) {
                Some(id) => Endpoint::Bound(id),
                None => Endpoint::BoundAbsent(value),
            })
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
fn node_universe<D: DatasetView + Sync>(ctx: &PathCtx<'_, D>) -> BTreeSet<D::Id> {
    let mut out = BTreeSet::new();
    // The node universe is a full scan of the active scope, and it seeds every
    // both-endpoints-variable path — so it is charged per candidate endpoint examined,
    // exactly like a frontier expansion. Stopping early yields a smaller universe, hence
    // a subset of the true solutions: a sound lower bound.
    let mut stopped = false;
    ctx.scope.for_each_quad(ctx.dataset, None, None, None, |q| {
        if stopped {
            return;
        }
        if !ctx.charge_frontier() {
            stopped = true;
            return;
        }
        out.insert(q.s);
        out.insert(q.o);
    });
    out
}

/// The reachable-set memo in front of [`reach_uncached`], keyed by the path node's
/// address, the start node, and the direction.
///
/// **A set the budget cut short is never memoized.** `closure`, `closure_multi` and
/// `range_reach` all return the frontier reached so far when a governor stops them, and
/// that partial set is a *lower* bound on the path relation, not the relation — writing it
/// under this key would let a later lookup read it as though the traversal had finished.
/// The write is therefore gated on the traversal having completed, which is the same rule
/// [`crate::expr::exists`] applies to its inner-pattern memo, and it makes the memo's
/// soundness local to this function rather than a consequence of trips being latched
/// somewhere else.
fn reach_cached<D: DatasetView + Sync>(
    path: &PropertyPathExpression,
    node: D::Id,
    forward: bool,
    ctx: &PathCtx<'_, D>,
) -> Rc<BTreeSet<D::Id>> {
    let key = (
        std::ptr::from_ref::<PropertyPathExpression>(path) as usize,
        node,
        forward,
    );
    if let Some(cached) = ctx.reach_cache.borrow().get(&key) {
        return cached.clone();
    }

    let result = Rc::new(reach_uncached(path, node, forward, ctx));
    if ctx.stopped() {
        return result;
    }
    ctx.reach_cache.borrow_mut().insert(key, result.clone());
    result
}

fn reach_uncached<D: DatasetView + Sync>(
    path: &PropertyPathExpression,
    node: D::Id,
    forward: bool,
    ctx: &PathCtx<'_, D>,
) -> BTreeSet<D::Id> {
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
        P::NegatedPropertySet(elems) => step_negated(elems, node, forward, ctx),
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

/// Whether `path` contains a repetition operator (`*`, `+`, `?`, `{n,m}`)
/// anywhere in its tree — the SPARQL 1.1 §18.3 dividing line between the two
/// evaluation strategies `eval_path` dispatches on:
///
/// - `false` (a "simple" path of only `/`, `|`, `^`, `!(…)`, a single
///   predicate, or `<any>`): evaluates as if unrolled into a BGP, so a node
///   pair reachable via several distinct triple combinations is a MULTISET —
///   one result row per derivation (`pp11`, `pp31`). See
///   [`simple_reach_multiset`].
/// - `true`: evaluates via the ALP fixpoint (`reach`/`closure`), a SET of
///   reachable nodes with no duplicates — required for termination on
///   cyclic/infinite graphs, and mandated even when the repetition is nested
///   under a combinator (e.g. `(:p/:q)+`).
fn path_has_repetition(path: &PropertyPathExpression) -> bool {
    use PropertyPathExpression as P;
    match path {
        P::ZeroOrMore(_) | P::OneOrMore(_) | P::ZeroOrOne(_) | P::Range { .. } => true,
        P::NamedNode(_) | P::NegatedPropertySet(_) | P::Wildcard { .. } => false,
        P::Reverse(inner) => path_has_repetition(inner),
        P::Sequence(a, b) | P::Alternative(a, b) => {
            path_has_repetition(a) || path_has_repetition(b)
        }
    }
}

/// The MULTISET of nodes `y` such that `(node,y)` (forward) or `(y,node)`
/// (backward) is in `path`'s relation, for a path with NO repetition operator
/// (see [`path_has_repetition`]) — one entry per distinct underlying triple
/// combination, so a `Sequence`/`Alternative` that can be satisfied several
/// ways yields that many entries. Deterministic: every leaf step iterates the
/// dataset's `TermId` order (via the same `step_*` primitives `reach` uses)
/// and `Sequence`/`Alternative` compose that order structurally, so row order
/// is stable run-to-run. Must never be called on a path containing repetition
/// (`path_has_repetition(path)` is checked once by the caller, `eval_path`).
fn simple_reach_multiset<D: DatasetView + Sync>(
    path: &PropertyPathExpression,
    node: D::Id,
    forward: bool,
    ctx: &PathCtx<'_, D>,
) -> Vec<D::Id> {
    use PropertyPathExpression as P;
    match path {
        P::NamedNode(p) => step_predicate(p, node, forward, ctx).into_iter().collect(),
        P::Reverse(inner) => simple_reach_multiset(inner, node, !forward, ctx),
        P::Sequence(a, b) => {
            // Forward: step `a` then `b`. Backward: step `b` then `a`, each backward
            // — same direction-swap `reach_uncached`'s `Sequence` arm applies.
            let (first, second): (&P, &P) = if forward { (a, b) } else { (b, a) };
            let mut out = Vec::new();
            for mid in simple_reach_multiset(first, node, forward, ctx) {
                out.extend(simple_reach_multiset(second, mid, forward, ctx));
            }
            out
        }
        P::Alternative(a, b) => {
            let mut out = simple_reach_multiset(a, node, forward, ctx);
            out.extend(simple_reach_multiset(b, node, forward, ctx));
            out
        }
        P::NegatedPropertySet(elems) => step_negated(elems, node, forward, ctx)
            .into_iter()
            .collect(),
        P::Wildcard { namespace } => step_wildcard(namespace.as_ref(), node, forward, ctx)
            .into_iter()
            .collect(),
        P::ZeroOrMore(_) | P::OneOrMore(_) | P::ZeroOrOne(_) | P::Range { .. } => {
            unreachable!(
                "simple_reach_multiset called on a path containing repetition; \
                 eval_path must check path_has_repetition first"
            )
        }
    }
}

/// One predicate hop. Forward: objects of `(node, p, ?)`; backward: subjects of
/// `(?, p, node)`. A predicate absent from the dataset yields nothing.
fn step_predicate<D: DatasetView + Sync>(
    p: &NamedNode,
    node: D::Id,
    forward: bool,
    ctx: &PathCtx<'_, D>,
) -> BTreeSet<D::Id> {
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

/// `!(p1|…|^q1|…)`: one hop along any predicate NOT in the excluded set, per
/// direction (SPARQL 1.1 §18.3). The plain elements exclude a **forward** hop;
/// the `^`-elements exclude a **reverse** hop; the two contributions are
/// unioned, and a direction with no listed elements is omitted entirely (see
/// [`NegatedSets`]). Uses the pre-resolved `cache` to avoid re-resolving
/// excluded IRIs on every call.
fn step_negated<D: DatasetView + Sync>(
    elems: &[purrdf_sparql_algebra::NegatedPathElement],
    node: D::Id,
    forward: bool,
    ctx: &PathCtx<'_, D>,
) -> BTreeSet<D::Id> {
    let sets = &ctx.cache[&(elems.as_ptr() as usize)];
    let mut out = BTreeSet::new();
    if let Some(excluded) = &sets.forward {
        out.extend(step_excluding(excluded, node, forward, ctx));
    }
    if let Some(excluded) = &sets.inverse {
        out.extend(step_excluding(excluded, node, !forward, ctx));
    }
    out
}

/// One hop along any predicate NOT in `excluded`, in the given direction —
/// the direction-parameterised primitive `step_negated` composes twice (once
/// per element kind) to get the full negated-set relation.
fn step_excluding<D: DatasetView + Sync>(
    excluded: &BTreeSet<D::Id>,
    node: D::Id,
    forward: bool,
    ctx: &PathCtx<'_, D>,
) -> BTreeSet<D::Id> {
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
fn step_wildcard<D: DatasetView + Sync>(
    namespace: Option<&NamedNode>,
    node: D::Id,
    forward: bool,
    ctx: &PathCtx<'_, D>,
) -> BTreeSet<D::Id> {
    let prefix = namespace.map(NamedNode::as_str);
    let pred_ok = |pid: D::Id| -> bool {
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
fn closure<D: DatasetView + Sync>(
    inner: &PropertyPathExpression,
    node: D::Id,
    forward: bool,
    ctx: &PathCtx<'_, D>,
) -> BTreeSet<D::Id> {
    // `result` stays an ordered `BTreeSet` — it is the returned/egress set, and its
    // iteration order determines solution-row order (byte-identity). `visited` is a
    // membership-only guard, never iterated into output, so it uses an O(1)
    // `DetHashSet` instead of the O(log n) `BTreeSet` on the closure's hot loop.
    let mut result = BTreeSet::new();
    let mut visited: DetHashSet<D::Id> = DetHashSet::default();
    let mut frontier: Vec<D::Id> = reach_cached(inner, node, forward, ctx)
        .iter()
        .copied()
        .collect();
    while let Some(n) = frontier.pop() {
        if !visited.insert(n) {
            continue;
        }
        // The `path-frontier-expansion` charge point. Returning the frontier reached so
        // far is sound: fewer expansions can only reach fewer nodes.
        if !ctx.charge_frontier() {
            break;
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
fn closure_multi<D: DatasetView + Sync>(
    inner: &PropertyPathExpression,
    seeds: &BTreeSet<D::Id>,
    forward: bool,
    ctx: &PathCtx<'_, D>,
) -> BTreeSet<D::Id> {
    // As in `closure`: ordered `result` for egress, O(1) `DetHashSet` for the
    // membership-only `visited` guard.
    let mut result = BTreeSet::new();
    let mut visited: DetHashSet<D::Id> = DetHashSet::default();
    let mut frontier: Vec<D::Id> = Vec::new();
    for &s in seeds {
        frontier.extend(reach_cached(inner, s, forward, ctx).iter().copied());
    }
    while let Some(n) = frontier.pop() {
        if !visited.insert(n) {
            continue;
        }
        // The `path-frontier-expansion` charge point; see `closure`.
        if !ctx.charge_frontier() {
            break;
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
///
/// # Why this cannot iterate `max` times
///
/// `min` and `max` are `u32`, so reading the definition literally makes `p{4000000000,}`
/// over a cyclic graph a four-billion-level breadth-first search: work bounded by the
/// *query text* rather than by the data, which no caller can wait out and no budget can
/// make correct — a budget turns a hang into a truncated answer, where the answer here is
/// small, exact, and cheap to compute. Two identities bound the iteration by the **graph**
/// instead. Write `S_k` for the set of nodes reachable in exactly `k` applications, so
/// `S_{k+1} = step(S_k)` with `step(A) = ⋃_{y ∈ A} reach(y)`. `step` distributes over
/// union (`step(A ∪ B) = step(A) ∪ step(B)`), which is what both arguments turn on.
///
/// 1. **At or above `min`, the first level that adds nothing is the last that could.**
///    Let `A_k = ⋃_{j=min..k} S_j` be the accumulated output. If `S_k ⊆ A_{k-1}` then
///    `S_{k+1} = step(S_k) ⊆ step(A_{k-1}) = ⋃_{j=min+1..k} S_j ⊆ A_k = A_{k-1}`, and the
///    same step applies again to `S_{k+2}`, so *every* later level is already inside
///    `A_{k-1}`. Each level that does not end the loop therefore contributes at least one
///    node, and a level set holds only nodes of the graph — which is the reachable-node
///    cap on the accumulating phase, derived rather than asserted.
/// 2. **Below `min`, the level sequence is eventually periodic.** `step` is a
///    deterministic function of the whole set, so a single repeat `S_a == S_b` (`a < b`)
///    forces `S_{a+t} == S_{b+t}` for every `t`: from `a` on, the sequence cycles with
///    period `b - a`. [`advance_to_min`] finds such a repeat with Brent's cycle detection
///    — one saved frontier and one set comparison per level, never a second walk of the
///    sequence — and then reaches `S_min` by advancing `(min - b) mod (b - a)` further
///    levels instead of `min - b` of them.
///
/// Neither identity truncates a result. `p{n,m}` keeps exact multiset semantics; the
/// levels not walked are proven to recompute sets the answer already holds. (`p*`/`p+`
/// are a different shape: [`closure`]/[`closure_multi`] carry a visited-set guard and
/// already terminate on a cyclic graph.)
fn range_reach<D: DatasetView + Sync>(
    inner: &PropertyPathExpression,
    node: D::Id,
    forward: bool,
    min: u32,
    max: Option<u32>,
    ctx: &PathCtx<'_, D>,
) -> BTreeSet<D::Id> {
    let mut out = BTreeSet::new();
    // An empty repetition window (`max < min`) admits no `k` at all.
    if max.is_some_and(|m| m < min) {
        return out;
    }

    // The prefix walk: reach level `min`. Levels below it contribute nothing to the
    // output, so a
    // frontier that dies on the way there (or a budget that stops the walk) leaves the
    // whole range empty — no level at or above `min` is reachable either.
    let mut current: BTreeSet<D::Id> = BTreeSet::from([node]);
    if !advance_to_min(inner, &mut current, forward, min, ctx) {
        return out;
    }

    // The accumulating walk: union the levels in `[min, max]`.
    let mut level = min;
    loop {
        let before = out.len();
        out.extend(current.iter().copied());
        let grew = out.len() != before;
        match max {
            // The window closes at this level.
            Some(m) if level >= m => break,
            // Unbounded tail: `*`-close from the exactly-`min` frontier in a single joint
            // traversal (avoids redundant per-seed re-traversal). Only ever reached at
            // `level == min`, since this arm always breaks.
            None => {
                out.extend(closure_multi(inner, &current, forward, ctx));
                break;
            }
            _ => {}
        }
        // Identity 1: this level contributed nothing, so no later level can either.
        if !grew {
            break;
        }
        let Some(next) = step_level(inner, &current, forward, ctx) else {
            return out;
        };
        current = next;
        level += 1;
    }
    out
}

/// Advance `current` from level 0 to level `min`, reporting whether a level-`min`
/// frontier survives to accumulate.
///
/// `false` means the frontier emptied out — or the execution's budget stopped the walk —
/// before level `min`, so no repetition count at or above `min` reaches anything.
///
/// Brent's cycle detection runs alongside the advance: `tortoise` is a frontier already
/// seen, `lam` levels behind the live one, and `power` is the next milestone at which the
/// tortoise jumps forward. It costs one saved frontier and one set comparison per level —
/// the live sequence is walked once, never twice — and finds a repeat in
/// `O(pre-period + period)` levels, which is a property of the graph rather than of
/// `min`. See [`range_reach`] for why a repeat licenses skipping whole levels exactly.
fn advance_to_min<D: DatasetView + Sync>(
    inner: &PropertyPathExpression,
    current: &mut BTreeSet<D::Id>,
    forward: bool,
    min: u32,
    ctx: &PathCtx<'_, D>,
) -> bool {
    let mut tortoise = current.clone();
    let mut power: u64 = 1;
    let mut lam: u64 = 0;
    let mut level: u32 = 0;

    while level < min {
        let Some(next) = step_level(inner, current, forward, ctx) else {
            return false;
        };
        *current = next;
        if current.is_empty() {
            return false;
        }
        level += 1;
        lam += 1;

        if *current == tortoise {
            // `S_{level - lam} == S_level`, so from level `level - lam` on the sequence
            // has period `lam` and every level congruent to `level` modulo `lam` holds
            // exactly this set. Walk out the residue only; the levels stepped over are
            // proven equal to levels already stepped.
            let skip = u64::from(min - level) % lam;
            for _ in 0..skip {
                let Some(next) = step_level(inner, current, forward, ctx) else {
                    return false;
                };
                *current = next;
            }
            return !current.is_empty();
        }
        if lam == power {
            tortoise.clone_from(current);
            power = power.saturating_mul(2);
            lam = 0;
        }
    }
    !current.is_empty()
}

/// One frontier advance: the nodes reachable in one further application of `inner` from
/// every node of `current`. `None` means the execution's budget stopped the walk
/// mid-level, which every caller treats as "return what has been reached so far" — a
/// partially expanded frontier reaches a subset of the truly reachable nodes.
fn step_level<D: DatasetView + Sync>(
    inner: &PropertyPathExpression,
    current: &BTreeSet<D::Id>,
    forward: bool,
    ctx: &PathCtx<'_, D>,
) -> Option<BTreeSet<D::Id>> {
    note_level_advance();
    let mut next = BTreeSet::new();
    for n in current {
        // The `path-frontier-expansion` charge point. A `{n,m}` range re-enters the same
        // nodes at every repetition count by design, so the frontier expansion — not the
        // level — is the quantity a budget bounds.
        if !ctx.charge_frontier() {
            return None;
        }
        next.extend(reach_cached(inner, *n, forward, ctx).iter().copied());
    }
    Some(next)
}

#[cfg(test)]
thread_local! {
    /// Test-only instrumentation: the number of range-path frontier advances performed on
    /// this thread since [`counting_level_advances`] last reset it.
    static LEVEL_ADVANCES: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Record one range-path frontier advance.
///
/// Compiled to nothing outside `cfg(test)`. The bound on [`range_reach`]'s level count is
/// a structural property of the algorithm, and pinning a structural property by test needs
/// a counter — a wall-clock timeout would only say "it finished on this machine today",
/// which is exactly the assertion a regression can pass by being slow instead of infinite.
#[inline]
fn note_level_advance() {
    #[cfg(test)]
    LEVEL_ADVANCES.with(|advances| advances.set(advances.get().saturating_add(1)));
}

/// Run `body`, returning its value with the number of range-path frontier advances it
/// performed on this thread.
#[cfg(test)]
fn counting_level_advances<T>(body: impl FnOnce() -> T) -> (T, u64) {
    LEVEL_ADVANCES.with(|advances| advances.set(0));
    let value = body();
    (value, LEVEL_ADVANCES.with(std::cell::Cell::get))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use purrdf_core::{RdfDataset, RdfDatasetBuilder};
    use purrdf_sparql_algebra::NamedNode;

    const EX: &str = "http://ex/";

    fn iri(local: &str) -> String {
        format!("{EX}{local}")
    }

    /// Build a directed graph over predicate `local`-named edges. Each edge is a
    /// `(subject_local, predicate_local, object_local)` triple.
    fn graph_of(edges: &[(&str, &str, &str)]) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        for (s, p, o) in edges {
            let s = b.intern_iri(&iri(s));
            let p = b.intern_iri(&iri(p));
            let o = b.intern_iri(&iri(o));
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

    /// A negated-property-set element: `npe("p", false)` is the plain `:p`,
    /// `npe("p", true)` is the inverted `^:p`.
    fn npe(local: &str, inverse: bool) -> purrdf_sparql_algebra::NegatedPathElement {
        purrdf_sparql_algebra::NegatedPathElement {
            predicate: nn(local),
            inverse,
        }
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
        subject: &TermPattern,
        path: &PropertyPathExpression,
        object: &TermPattern,
        vars: &[&str],
    ) -> Vec<Vec<Option<String>>> {
        let mut ctx = EvalCtx::new(ds);
        let seq = eval_path(subject, path, object, &mut ctx).expect("path eval");
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
                        // A reflexive zero-length pairing on a ground endpoint absent
                        // from the dataset mints a `Computed` term for that value
                        // (see `eval_path`'s `BoundAbsent` handling) — resolve it back
                        // through the scratch interner the same way the real
                        // evaluator's result egress does.
                        Some(term @ SolutionTerm::Computed(_)) => {
                            let value = ctx.scratch.value_of(ds, term);
                            Some(match value {
                                TermValue::Iri(s) => s.strip_prefix(EX).unwrap_or(&s).to_owned(),
                                other => format!("{other:?}"),
                            })
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
            governors: None,
            ledger: None,
        };
        let mut v: Vec<String> = reach_cached(path, sid, forward, &pctx)
            .iter()
            .copied()
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
        let rows = run(&ds, &ground("a"), &named("p"), &var("o"), &["o"]);
        assert_eq!(rows, col1(&["b", "c"]));
        // { :b ^:p ?s }  → inverse: ?s is anything that points to :b via :p, i.e. :a
        // (`:b ^:p ?s` ⟺ `?s :p :b`).
        let rev = PropertyPathExpression::Reverse(Box::new(named("p")));
        let rows = run(&ds, &ground("b"), &rev, &var("s"), &["s"]);
        assert_eq!(rows, col1(&["a"]));
    }

    #[test]
    fn sequence_chains_two_predicates() {
        let ds = graph_of(&[("a", "p", "x"), ("x", "q", "b"), ("x", "q", "c")]);
        // :a :p/:q ?o → b, c
        let seq = PropertyPathExpression::Sequence(Box::new(named("p")), Box::new(named("q")));
        let rows = run(&ds, &ground("a"), &seq, &var("o"), &["o"]);
        assert_eq!(rows, col1(&["b", "c"]));
    }

    #[test]
    fn sequence_backward_from_object() {
        let ds = graph_of(&[("a", "p", "x"), ("x", "q", "b")]);
        // ?s :p/:q :b  → a
        let seq = PropertyPathExpression::Sequence(Box::new(named("p")), Box::new(named("q")));
        let rows = run(&ds, &var("s"), &seq, &ground("b"), &["s"]);
        assert_eq!(rows, col1(&["a"]));
    }

    #[test]
    fn alternative_unions_both() {
        let ds = graph_of(&[("a", "p", "b"), ("a", "q", "c")]);
        let alt = PropertyPathExpression::Alternative(Box::new(named("p")), Box::new(named("q")));
        let rows = run(&ds, &ground("a"), &alt, &var("o"), &["o"]);
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

    // ---- Range {n,m} (PurRDF extension), including on cycles -----------------

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

    #[test]
    fn cyclic_graph_p_n_m_terminates_without_a_budget() {
        // `min`/`max` are `u32`, so the levels a literal reading of `p{n,m}` would walk
        // are bounded by the query text — billions of them over a graph with three nodes.
        // No governor is engaged here on purpose: an ungoverned caller is every caller
        // that has not opted in, and "the budget stops it" is not a fix for a hang.
        //
        // The bound is asserted by COUNTING frontier advances, never by a wall clock: a
        // regression that reintroduces the level-per-`k` walk fails this deterministically
        // on any machine, where a timeout would only report that today's machine was slow
        // enough or fast enough.
        let ds = graph_of(&[("a", "p", "b"), ("b", "p", "c"), ("c", "p", "a")]);
        let rng = |min, max| PropertyPathExpression::Range {
            inner: Box::new(named("p")),
            min,
            max,
        };
        let all = vec!["a".to_owned(), "b".to_owned(), "c".to_owned()];

        // A huge `max`: the accumulating phase stops at the first level that adds no node
        // the output already holds, so it walks at most one level per graph node.
        let (locals, advances) =
            counting_level_advances(|| reach_locals(&ds, &rng(0, Some(u32::MAX)), "a", true));
        assert_eq!(locals, all);
        assert!(
            advances <= 4,
            "a {{0,u32::MAX}} range over a 3-node cycle must stop within one level per \
             node, walked {advances}"
        );

        // A huge `min` with an open tail: the prefix is skipped by periodicity, not
        // walked.
        let (locals, advances) =
            counting_level_advances(|| reach_locals(&ds, &rng(4_000_000_000, None), "a", true));
        assert_eq!(locals, all);
        assert!(
            advances <= 16,
            "a {{4000000000,}} range over a 3-node cycle must detect the period instead \
             of walking to it, walked {advances}"
        );

        // A huge `min` AND a huge `max`: both phases bounded at once.
        let (locals, advances) = counting_level_advances(|| {
            reach_locals(&ds, &rng(4_000_000_000, Some(u32::MAX)), "a", true)
        });
        assert_eq!(locals, all);
        assert!(
            advances <= 16,
            "a {{4000000000,u32::MAX}} range over a 3-node cycle must bound both the \
             prefix and the accumulation, walked {advances}"
        );
    }

    #[test]
    fn range_path_answers_are_unchanged_by_the_early_exit() {
        // A governor may only change an outcome, never an answer — and the level bounds
        // in `range_reach` are not a governor at all: they remove levels that provably
        // recompute a set the answer already holds. The corpus below spans cyclic and
        // acyclic shapes, ranges with and without a reachable fixpoint, `{0,k}`, `{n,}`,
        // `{n,m}`, `n == m`, an empty window (`max < min`), and repetition counts far past
        // any level the walk actually performs.
        //
        // Every expectation is computed BY HAND from the exact-`k` definition
        // (`S_0 = {start}`, `S_{k+1} = step(S_k)`, answer = `⋃_{k ∈ [min,max]} S_k`), not
        // captured from this implementation: a golden captured from the code under test
        // agrees with it by construction, including when both are wrong.
        struct Case {
            edges: &'static [(&'static str, &'static str, &'static str)],
            start: &'static str,
            min: u32,
            max: Option<u32>,
            expected: &'static [&'static str],
            why: &'static str,
        }

        // a → b → c → a.  S_k = {a},{b},{c} cycling with period 3.
        const CYCLE3: &[(&str, &str, &str)] = &[("a", "p", "b"), ("b", "p", "c"), ("c", "p", "a")];
        // a → b → c → d.  S_0..S_3 = {a},{b},{c},{d}; S_4 and beyond empty.
        const CHAIN: &[(&str, &str, &str)] = &[("a", "p", "b"), ("b", "p", "c"), ("c", "p", "d")];
        // x → x, x → y.  S_0 = {x}; S_k = {x,y} for every k ≥ 1 (a level-set fixpoint).
        const SELF_LOOP: &[(&str, &str, &str)] = &[("x", "p", "x"), ("x", "p", "y")];
        // a → x1..x5 and xi → x(i+1).  S_k = {x_k … x5} for 1 ≤ k ≤ 5; S_6 empty. The
        // cumulative reachable set stops growing at level 1, but the LEVEL sets keep
        // shrinking for four more levels — the shape that makes "stop when the cumulative
        // union stops growing" an unsound rule and the accumulated-output rule a sound one.
        const FAN: &[(&str, &str, &str)] = &[
            ("a", "p", "x1"),
            ("a", "p", "x2"),
            ("a", "p", "x3"),
            ("a", "p", "x4"),
            ("a", "p", "x5"),
            ("x1", "p", "x2"),
            ("x2", "p", "x3"),
            ("x3", "p", "x4"),
            ("x4", "p", "x5"),
        ];
        // a ⇄ b, b → c.  S_0 = {a}; S_odd = {b}; S_even≥2 = {a,c} (period 2 from level 1).
        const CYCLE2_TAIL: &[(&str, &str, &str)] =
            &[("a", "p", "b"), ("b", "p", "a"), ("b", "p", "c")];

        let corpus = [
            Case {
                edges: CYCLE3,
                start: "a",
                min: 0,
                max: Some(0),
                expected: &["a"],
                why: "S_0 is the zero-length identity",
            },
            Case {
                edges: CYCLE3,
                start: "a",
                min: 1,
                max: Some(1),
                expected: &["b"],
                why: "S_1",
            },
            Case {
                edges: CYCLE3,
                start: "a",
                min: 7,
                max: Some(7),
                expected: &["b"],
                why: "7 mod 3 == 1, so S_7 == S_1",
            },
            Case {
                edges: CYCLE3,
                start: "a",
                min: 4_000_000_000,
                max: Some(4_000_000_000),
                expected: &["b"],
                why: "4000000000 mod 3 == 1, so S_4000000000 == S_1",
            },
            Case {
                edges: CYCLE3,
                start: "a",
                min: 2,
                max: Some(5),
                expected: &["a", "b", "c"],
                why: "S_2..S_5 covers every phase of the cycle",
            },
            Case {
                edges: CYCLE3,
                start: "a",
                min: 4,
                max: None,
                expected: &["a", "b", "c"],
                why: "the open tail of a cycle covers every phase",
            },
            Case {
                edges: CYCLE3,
                start: "a",
                min: 0,
                max: None,
                expected: &["a", "b", "c"],
                why: "{0,} is the reflexive-transitive closure",
            },
            Case {
                edges: CYCLE3,
                start: "a",
                min: 3,
                max: Some(1),
                expected: &[],
                why: "max < min admits no repetition count",
            },
            Case {
                edges: CHAIN,
                start: "a",
                min: 0,
                max: Some(2),
                expected: &["a", "b", "c"],
                why: "S_0 ∪ S_1 ∪ S_2",
            },
            Case {
                edges: CHAIN,
                start: "a",
                min: 2,
                max: None,
                expected: &["c", "d"],
                why: "S_2 ∪ S_3; S_4 onwards is empty",
            },
            Case {
                edges: CHAIN,
                start: "a",
                min: 5,
                max: None,
                expected: &[],
                why: "the frontier dies before level 5",
            },
            Case {
                edges: CHAIN,
                start: "a",
                min: 2,
                max: Some(10),
                expected: &["c", "d"],
                why: "a max past the end of the chain adds nothing",
            },
            Case {
                edges: CHAIN,
                start: "a",
                min: 3,
                max: Some(3),
                expected: &["d"],
                why: "S_3",
            },
            Case {
                edges: CHAIN,
                start: "a",
                min: 4,
                max: Some(4),
                expected: &[],
                why: "S_4 is empty",
            },
            Case {
                edges: SELF_LOOP,
                start: "x",
                min: 0,
                max: Some(0),
                expected: &["x"],
                why: "S_0",
            },
            Case {
                edges: SELF_LOOP,
                start: "x",
                min: 1,
                max: Some(1),
                expected: &["x", "y"],
                why: "S_1 = step({x})",
            },
            Case {
                edges: SELF_LOOP,
                start: "x",
                min: 2,
                max: Some(3),
                expected: &["x", "y"],
                why: "the level sets have reached a fixpoint by level 1",
            },
            Case {
                edges: SELF_LOOP,
                start: "x",
                min: 5,
                max: None,
                expected: &["x", "y"],
                why: "a fixpoint reached far below min",
            },
            Case {
                edges: FAN,
                start: "a",
                min: 1,
                max: Some(2),
                expected: &["x1", "x2", "x3", "x4", "x5"],
                why: "S_1 ∪ S_2",
            },
            Case {
                edges: FAN,
                start: "a",
                min: 2,
                max: Some(4),
                expected: &["x2", "x3", "x4", "x5"],
                why: "S_2 ∪ S_3 ∪ S_4 = S_2",
            },
            Case {
                edges: FAN,
                start: "a",
                min: 3,
                max: None,
                expected: &["x3", "x4", "x5"],
                why: "x1 and x2 are reachable in fewer than 3 steps only",
            },
            Case {
                edges: FAN,
                start: "a",
                min: 5,
                max: Some(5),
                expected: &["x5"],
                why: "S_5",
            },
            Case {
                edges: FAN,
                start: "a",
                min: 6,
                max: None,
                expected: &[],
                why: "S_6 is empty",
            },
            Case {
                edges: CYCLE2_TAIL,
                start: "a",
                min: 2,
                max: Some(2),
                expected: &["a", "c"],
                why: "S_2 = step({b})",
            },
            Case {
                edges: CYCLE2_TAIL,
                start: "a",
                min: 3,
                max: None,
                expected: &["a", "b", "c"],
                why: "the tail alternates {b} and {a,c}",
            },
            Case {
                edges: CYCLE2_TAIL,
                start: "a",
                min: 0,
                max: Some(3),
                expected: &["a", "b", "c"],
                why: "S_0 ∪ S_1 ∪ S_2 ∪ S_3",
            },
            Case {
                edges: CYCLE2_TAIL,
                start: "a",
                min: 1_000_000_001,
                max: Some(1_000_000_001),
                expected: &["b"],
                why: "an odd level past the pre-period is S_1",
            },
            Case {
                edges: CYCLE2_TAIL,
                start: "a",
                min: 1_000_000_000,
                max: Some(1_000_000_000),
                expected: &["a", "c"],
                why: "an even level past the pre-period is S_2",
            },
            Case {
                edges: CHAIN,
                start: "b",
                min: 0,
                max: None,
                expected: &["b", "c", "d"],
                why: "a start part-way along the chain",
            },
            Case {
                edges: CHAIN,
                start: "d",
                min: 1,
                max: None,
                expected: &[],
                why: "a sink node reaches nothing in one or more steps",
            },
        ];

        for case in &corpus {
            let ds = graph_of(case.edges);
            let rng = PropertyPathExpression::Range {
                inner: Box::new(named("p")),
                min: case.min,
                max: case.max,
            };
            let expected: Vec<String> = case.expected.iter().map(|s| (*s).to_owned()).collect();
            assert_eq!(
                reach_locals(&ds, &rng, case.start, true),
                expected,
                "p{{{},{:?}}} from {}: {}",
                case.min,
                case.max,
                case.start,
                case.why
            );
        }
    }

    // ---- negated property set & wildcard -----------------------------------

    #[test]
    fn negated_property_set_excludes_named() {
        let ds = graph_of(&[("a", "p", "b"), ("a", "q", "c"), ("a", "r", "d")]);
        // !(:p|:q) → only the :r edge.
        let neg =
            PropertyPathExpression::NegatedPropertySet(vec![npe("p", false), npe("q", false)]);
        let rows = run(&ds, &ground("a"), &neg, &var("o"), &["o"]);
        assert_eq!(rows, col1(&["d"]));
    }

    #[test]
    fn negated_property_set_pure_inverse() {
        // W3C nps_inverse: !^:pr — from :od, the only reverse-non-:pr edge is
        // :sd :pd :od (^:pd), i.e. the forward-listed part is EMPTY and must be
        // omitted from the union entirely (not "match every forward edge").
        let ds = graph_of(&[("sd", "pd", "od"), ("sr", "pr", "or")]);
        let neg = PropertyPathExpression::NegatedPropertySet(vec![npe("pr", true)]);
        let rows = run(&ds, &var("s"), &neg, &var("o"), &["s", "o"]);
        assert_eq!(
            rows,
            vec![vec![Some("od".to_owned()), Some("sd".to_owned())]]
        );
    }

    #[test]
    fn negated_property_set_direct_and_inverse() {
        // W3C nps_direct_and_inverse: !(:pd|^:pr) decomposes into
        // Alternative(NegatedPropertySet([:pd]), Reverse(NegatedPropertySet([:pr]))).
        let ds = graph_of(&[("sd", "pd", "od"), ("sr", "pr", "or")]);
        let neg =
            PropertyPathExpression::NegatedPropertySet(vec![npe("pd", false), npe("pr", true)]);
        let rows = run(&ds, &var("s"), &neg, &var("o"), &["s", "o"]);
        let mut expected = vec![
            vec![Some("sr".to_owned()), Some("or".to_owned())],
            vec![Some("od".to_owned()), Some("sd".to_owned())],
        ];
        expected.sort_by_key(|row| format!("{row:?}"));
        assert_eq!(rows, expected);
    }

    #[test]
    fn wildcard_any_and_namespace_scoped() {
        // Two predicate namespaces: http://ex/ and http://other/.
        let mut b = RdfDatasetBuilder::new();
        let a = b.intern_iri(&iri("a"));
        let p = b.intern_iri(&iri("p"));
        let other_p = b.intern_iri("http://other/p");
        let x = b.intern_iri(&iri("x"));
        let y = b.intern_iri(&iri("y"));
        b.push_quad(a, p, x, None);
        b.push_quad(a, other_p, y, None);
        let ds = b.freeze().expect("freeze");

        // <any> → both objects.
        let any = PropertyPathExpression::Wildcard { namespace: None };
        let rows = run(&ds, &ground("a"), &any, &var("o"), &["o"]);
        assert_eq!(rows, col1(&["x", "y"]));

        // <any:http://ex/> → only the ex-namespaced edge.
        let scoped = PropertyPathExpression::Wildcard {
            namespace: Some(NamedNode::new_unchecked(EX)),
        };
        let rows = run(&ds, &ground("a"), &scoped, &var("o"), &["o"]);
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
        let rows = run(&ds, &var("s"), &star, &var("o"), &["s", "o"]);
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
        let rows = run(&ds, &var("x"), &plus, &var("x"), &["x"]);
        assert_eq!(rows, col1(&["a", "b"]));
    }

    // ---- same-variable reflexive short-circuit (Gap D) ---------------------

    #[test]
    fn same_var_reflexive_star() {
        // Graph a -> b -> c. Node universe = {a, b, c}.
        // ?x :p* ?x — p* is reflexive, so every node is a solution via zero-length identity.
        let ds = graph_of(&[("a", "p", "b"), ("b", "p", "c")]);
        let star = PropertyPathExpression::ZeroOrMore(Box::new(named("p")));
        let rows = run(&ds, &var("x"), &star, &var("x"), &["x"]);
        assert_eq!(rows, col1(&["a", "b", "c"]));
    }

    #[test]
    fn same_var_reflexive_optional() {
        // Graph a -> b -> c. Node universe = {a, b, c}.
        // ?x :p? ?x — p? is reflexive, so every node is a solution via zero-length identity.
        let ds = graph_of(&[("a", "p", "b"), ("b", "p", "c")]);
        let opt = PropertyPathExpression::ZeroOrOne(Box::new(named("p")));
        let rows = run(&ds, &var("x"), &opt, &var("x"), &["x"]);
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
        let rows = run(&ds, &var("x"), &rng, &var("x"), &["x"]);
        assert_eq!(rows, col1(&["a", "b", "c"]));
    }

    #[test]
    fn same_var_nonreflexive_no_cycle_is_empty() {
        // Acyclic a -> b -> c. ?x :p+ ?x — p+ is non-reflexive; no node cycles back.
        let ds = graph_of(&[("a", "p", "b"), ("b", "p", "c")]);
        let plus = PropertyPathExpression::OneOrMore(Box::new(named("p")));
        let rows = run(&ds, &var("x"), &plus, &var("x"), &["x"]);
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
        let rows = run(&ds, &ground("axiom"), &path, &var("x"), &["x"]);
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
        let neg = PropertyPathExpression::NegatedPropertySet(vec![npe("p", false)]);
        let plus = PropertyPathExpression::OneOrMore(Box::new(neg));
        assert_eq!(reach_locals(&ds, &plus, "a", true), vec!["b", "c"]);
    }

    // ---- ground endpoint absent from the dataset (Gap: zero-length identity) --

    #[test]
    fn zero_or_more_reflexive_ground_endpoint_absent_from_empty_dataset() {
        // W3C zero_or_more_set_start / zero_or_more_set_end: `:p*` on a
        // completely empty dataset still admits the zero-length reflexive
        // pairing for a ground endpoint that never appears in any triple.
        let ds = graph_of(&[]);
        let star = PropertyPathExpression::ZeroOrMore(Box::new(named("p")));
        // ?s :p* :o → s = :o (the object, bound to itself)
        let rows = run(&ds, &var("s"), &star, &ground("o"), &["s"]);
        assert_eq!(rows, col1(&["o"]));
        // :s :p* ?o → o = :s (the subject, bound to itself)
        let rows = run(&ds, &ground("s"), &star, &var("o"), &["o"]);
        assert_eq!(rows, col1(&["s"]));
    }

    #[test]
    fn zero_or_one_reflexive_ground_endpoint_absent_from_empty_dataset() {
        // Same shape as above but for `?` (zero_or_one_set_start/_end).
        let ds = graph_of(&[]);
        let opt = PropertyPathExpression::ZeroOrOne(Box::new(named("p")));
        let rows = run(&ds, &var("s"), &opt, &ground("o"), &["s"]);
        assert_eq!(rows, col1(&["o"]));
        let rows = run(&ds, &ground("s"), &opt, &var("o"), &["o"]);
        assert_eq!(rows, col1(&["s"]));
    }

    #[test]
    fn non_reflexive_ground_endpoint_absent_from_dataset_is_empty() {
        // A non-reflexive path (no *, +, ? at the relevant position) cannot
        // connect an absent node to anything else — it has no edges.
        let ds = graph_of(&[("a", "p", "b")]);
        let rows = run(&ds, &var("s"), &named("p"), &ground("nobody"), &["s"]);
        assert_eq!(rows, col1(&[]));
    }
}
