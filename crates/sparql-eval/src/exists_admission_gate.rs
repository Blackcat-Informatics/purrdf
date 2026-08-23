// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The differential machinery that proves the `EXISTS` admission boundary
//! (`crate::governor::soundness::probe_admissible`) is non-vacuous — every
//! ADMISSIBLE shape genuinely agrees with the per-row definition, and every
//! INADMISSIBLE one genuinely, observably diverges from it when forced through
//! the probe anyway.
//!
//! # The seam
//!
//! [`crate::expr::force_exists_strategy_for_test`] is a `#[cfg(test)]` thread-local
//! override (the same idiom [`crate::parallel::force_parallel_for_test`] already
//! uses for its own measurement seam) that pins [`crate::expr::exists`]'s
//! probe-vs-definition decision to a specific [`crate::expr::ForcedExistsStrategy`],
//! bypassing `probe_admissible` entirely in EITHER direction:
//!
//! * `Definition` on a shape the gate would ADMIT — always sound (the definition is
//!   correct for every shape), used below to prove the two strategies AGREE on every
//!   admissible arm.
//! * `Probe` on a shape the gate would REFUSE — unsound by construction where the
//!   arm's exclusion is load-bearing, used below to observe (and pin) the SPECIFIC
//!   wrong answer the gate exists to prevent.
//!
//! The seam is entirely `#[cfg(test)]`-gated (the thread-local, the enum, the guard,
//! and the one `if let` read at the decision site) — see `crate::expr`'s definitions.
//! It is never constructible outside this crate's own test binary, and does not exist
//! at all in a non-test build (verified by `make wasm`/hygiene: a `cfg(test)`-gated
//! item contributes nothing to a release artifact).
//!
//! # Organization
//!
//! * **Admissible-arm equivalence** (`exists_paths_agree_on_*`) — one test per arm
//!   `probe_admissible` admits, each a correlated shape whose family IS that arm, on a
//!   fixture where the natural (memoized-probe) answer and the forced-definition
//!   answer are compared row-for-row, with the fixture chosen so the answer is
//!   non-trivial (some outer rows true, some false).
//! * **Divergence witnesses** (`probe_would_*`) — one test per arm `probe_admissible`
//!   REFUSES, each forcing the probe onto that exact shape and asserting the SPECIFIC
//!   wrong answer against the correct (definition-path) one.
//! * **Parity cases** — error discipline and `SERVICE SILENT` swallow-point symmetry
//!   across strategies, plus the stateful-builtin classification pin.
//! * **The bounded-exhaustive generator** — enumerates `EXISTS` inner shapes from a
//!   small algebra grammar to a bounded depth (modeled on
//!   `crate::parallel_determinism_gate`'s corpus-gate architecture, but generated
//!   rather than hand-written) and checks memo-on ≡ memo-off everywhere, and
//!   probe ≡ forced-definition wherever the gate admits.
//! * **Governed × strategy** — the truncated-inner-never-memoized discipline, pinned
//!   against the current (post-redesign) decision site.

use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral};
use purrdf_sparql_algebra::{
    AggregateExpression, AggregateFunction, Expression, Function, GraphPattern, GroundTerm,
    Literal, NamedNode, NamedNodePattern, OrderExpression, PropertyPathExpression, TermPattern,
    TriplePattern, Variable,
};

use crate::error::EvalError;
use crate::eval::{EvalCtx, eval};
use crate::expr::{ForcedExistsStrategy, force_exists_strategy_for_test};

const EX: &str = "http://example.org/";

// ---------------------------------------------------------------------------
// Algebra-building helpers
// ---------------------------------------------------------------------------

fn var(name: &str) -> Variable {
    Variable::new(name)
}

fn tvar(name: &str) -> TermPattern {
    TermPattern::Variable(var(name))
}

fn nn(iri: &str) -> NamedNode {
    NamedNode::new_unchecked(iri)
}

fn pred(iri: &str) -> NamedNodePattern {
    NamedNodePattern::NamedNode(nn(iri))
}

fn pred_var(name: &str) -> NamedNodePattern {
    NamedNodePattern::Variable(var(name))
}

/// One triple pattern `s <iri> o`.
fn triple(s: TermPattern, iri: &str, o: TermPattern) -> TriplePattern {
    TriplePattern {
        subject: s,
        predicate: pred(iri),
        object: o,
    }
}

fn bgp(triples: Vec<TriplePattern>) -> GraphPattern {
    GraphPattern::Bgp { patterns: triples }
}

fn bgp1(s: TermPattern, iri: &str, o: TermPattern) -> GraphPattern {
    bgp(vec![triple(s, iri, o)])
}

fn bx(p: GraphPattern) -> Box<GraphPattern> {
    Box::new(p)
}

// ---------------------------------------------------------------------------
// Evaluation helpers
// ---------------------------------------------------------------------------

/// Evaluate `Expression::Exists(inner)` once per row of `outer`'s result against
/// `ds` (with `registry`, if given, injected as the property-function table),
/// under whatever strategy is (or is not) pinned via
/// [`force_exists_strategy_for_test`] at the call site. Returns one `Result` per
/// outer row, in row order — never unwrapped here, so a caller that expects (and
/// wants to assert) a hard error on some/every row can.
fn exists_results(
    ds: &Arc<RdfDataset>,
    outer: &GraphPattern,
    inner: &GraphPattern,
    registry: Option<&crate::property_fn::PropertyFunctionRegistry>,
) -> Vec<Result<bool, EvalError>> {
    let mut ctx = EvalCtx::new(ds);
    if let Some(registry) = registry {
        ctx = ctx.with_property_functions(registry);
    }
    let seq = eval(outer, &mut ctx).expect("outer pattern evaluates");
    let exists_expr = Expression::Exists(Box::new(inner.clone()));
    seq.rows
        .iter()
        .map(|row| {
            crate::expr::eval_ebv(&exists_expr, row, &seq.schema, &mut ctx)
                .map(|ebv| ebv.expect("EXISTS always yields a defined boolean when it succeeds"))
        })
        .collect()
}

/// Evaluate `Expression::Exists(inner)` once per row of `outer`'s result against
/// `ds`, under whatever strategy is (or is not) pinned via
/// [`force_exists_strategy_for_test`] at the call site. Returns the boolean answer
/// per outer row, in row order. Panics if any row hard-errors — use
/// [`exists_results`] directly when a hard error is expected/under test.
fn exists_answers(ds: &Arc<RdfDataset>, outer: &GraphPattern, inner: &GraphPattern) -> Vec<bool> {
    exists_results(ds, outer, inner, None)
        .into_iter()
        .map(|r| r.expect("EXISTS does not hard-error on this fixture"))
        .collect()
}

/// [`exists_answers`] under the natural (unforced) gate decision.
fn natural_answers(ds: &Arc<RdfDataset>, outer: &GraphPattern, inner: &GraphPattern) -> Vec<bool> {
    exists_answers(ds, outer, inner)
}

/// [`exists_answers`] with the decision site forced to `strategy` for the whole call.
fn forced_answers(
    ds: &Arc<RdfDataset>,
    outer: &GraphPattern,
    inner: &GraphPattern,
    strategy: ForcedExistsStrategy,
) -> Vec<bool> {
    let _guard = force_exists_strategy_for_test(strategy);
    exists_answers(ds, outer, inner)
}

/// [`exists_results`] with the decision site forced to `strategy` for the whole call,
/// and a property-function `registry` injected.
fn forced_results_with_registry(
    ds: &Arc<RdfDataset>,
    outer: &GraphPattern,
    inner: &GraphPattern,
    strategy: ForcedExistsStrategy,
    registry: &crate::property_fn::PropertyFunctionRegistry,
) -> Vec<Result<bool, EvalError>> {
    let _guard = force_exists_strategy_for_test(strategy);
    exists_results(ds, outer, inner, Some(registry))
}

/// [`exists_results`] under the natural (unforced) gate decision, with a
/// property-function `registry` injected.
fn natural_results_with_registry(
    ds: &Arc<RdfDataset>,
    outer: &GraphPattern,
    inner: &GraphPattern,
    registry: &crate::property_fn::PropertyFunctionRegistry,
) -> Vec<Result<bool, EvalError>> {
    exists_results(ds, outer, inner, Some(registry))
}

/// Whether `crate::governor::soundness::probe_admissible` classifies `inner` as
/// admissible against `outer`'s schema — the exact prepare-time-plus-decision-site
/// computation `crate::eval::PreparedExists::build`/`crate::expr::exists` perform
/// together (ENF-normalize, run the fourth structural analysis over the result, then
/// consult the caller's actual outer schema), reproduced here so a test can ask the
/// same question the real decision site asks. See [`classify`] for the shared
/// computation and its panic condition.
fn is_classified_admissible(outer: &GraphPattern, inner: &GraphPattern) -> bool {
    classify(outer, inner).0
}

/// Whether `inner`'s root [`crate::governor::soundness::NodeAnalysis::has_stateful_builtin`]
/// is `true` — a stateful builtin (directly, or through a nested `EXISTS`) reachable
/// anywhere within it. Used by the generator to tell a stateful shape apart from a
/// deterministic one: a stateful shape's raw per-evaluation answer is not itself
/// asserted (it can legitimately differ between two SEPARATE evaluations — see
/// [`crate::expr::exists`]'s doc), only its CLASSIFICATION (never probe-admitted).
/// Takes `outer` only for symmetry with [`is_classified_admissible`] (`classify`'s
/// shared computation needs it); statefulness itself never depends on the outer
/// schema.
fn is_classified_stateful(outer: &GraphPattern, inner: &GraphPattern) -> bool {
    classify(outer, inner).1
}

/// Shared computation for [`is_classified_admissible`]/[`is_classified_stateful`]: the
/// exact prepare-time-plus-decision-site sequence `crate::eval::PreparedExists::build`
/// and `crate::expr::exists` perform together (ENF-normalize, run the fourth structural
/// analysis over the result, then classify against `outer`'s own syntactic schema —
/// `crate::eval::syntactic_schema`, the same structural-schema derivation the real
/// evaluator falls back to on an early-stop/known-empty path, standing in here for the
/// real `schema` a live evaluation of `outer` would produce), run ONCE per call so a
/// caller that wants both classifications never normalizes/analyzes `inner` twice.
/// Panics if ENF folds `inner` to constant `false` (a degenerate fixture no
/// admissibility test wants).
///
/// `normalized` is moved into its own `Box` FIRST, and `analyze_pattern` walks it
/// THROUGH that `Box` — never through the pre-move local — for the identical reason
/// [`crate::eval::PreparedExists::build`]'s own doc comment gives: a table built
/// against the pre-move address would key every entry to memory this function's own
/// later `probe_admissible` call can never match, silently falling back to
/// `node_analysis`'s conservative-EMPTY-free-vars default and reporting every shape
/// admissible regardless of its actual `Values`/`Extend` collisions.
fn classify(outer: &GraphPattern, inner: &GraphPattern) -> (bool, bool) {
    let normalized = match crate::enf::normalize(inner) {
        crate::enf::Enf::Pattern(p) => Box::new(p),
        crate::enf::Enf::FoldedEmpty => {
            panic!("fixture's ENF folded to constant false — construct a non-degenerate inner")
        }
    };
    let mut table = crate::governor::soundness::NodeAnalysisTable::default();
    let root = crate::governor::soundness::analyze_pattern(&normalized, &mut table);
    let outer_schema = crate::eval::syntactic_schema(outer);
    let admissible =
        crate::governor::soundness::probe_admissible(&normalized, &table, &outer_schema);
    (admissible, root.has_stateful_builtin)
}

/// Assert that `crate::governor::soundness::probe_admissible` classifies `inner` as
/// ADMISSIBLE against `outer`'s schema — the precondition every
/// `exists_paths_agree_on_*` test needs for its "natural == forced-definition"
/// comparison to actually exercise the probe path naturally, rather than passing
/// vacuously because the gate already refused it.
fn assert_classified_admissible(outer: &GraphPattern, inner: &GraphPattern) {
    assert!(
        is_classified_admissible(outer, inner),
        "expected this shape to be classified ADMISSIBLE by probe_admissible"
    );
}

/// Assert that `crate::governor::soundness::probe_admissible` classifies `inner` as
/// INADMISSIBLE against `outer`'s schema — the "committed negative control" every
/// `probe_would_*` divergence witness needs: proof the exclusion is real (the gate
/// really does refuse this shape), before forcing the probe onto it anyway to observe
/// the wrong answer.
fn assert_classified_inadmissible(outer: &GraphPattern, inner: &GraphPattern) {
    assert!(
        !is_classified_admissible(outer, inner),
        "expected this shape to be classified INADMISSIBLE by probe_admissible — if it is \
         now admitted, this is no longer a valid divergence witness for the arm it names"
    );
}

/// Assert that `inner`, evaluated over `outer`'s rows against `ds`, gives the SAME
/// per-row answers whether reached NATURALLY (the real gate decides — expected to
/// choose the memoized probe for an admissible `inner`, verified via
/// [`assert_classified_admissible`]) or FORCED onto the per-row definition path, AND
/// that the fixture is non-trivial (both `true` and `false` occur). This is
/// `exists_paths_agree_on_*`'s shared body.
fn assert_probe_and_definition_agree(
    ds: &Arc<RdfDataset>,
    outer: &GraphPattern,
    inner: &GraphPattern,
) {
    assert_classified_admissible(outer, inner);
    let natural = natural_answers(ds, outer, inner);
    let forced_definition = forced_answers(ds, outer, inner, ForcedExistsStrategy::Definition);
    assert_eq!(
        natural, forced_definition,
        "the memoized probe and the forced per-row definition must agree row-for-row"
    );
    assert!(
        natural.contains(&true) && natural.contains(&false),
        "fixture is trivial (all {natural:?}) — it proves nothing about a shape whose \
         answer genuinely depends on the outer row"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- shared fixture: `EX:s{n} EX:knows EX:{target}` outer, correlated on `?s` ----
    //
    // `?s` ranges over `s1` (no further property) and `s2` (which the per-arm inner
    // additionally relates to something true only for `s2`), so `EXISTS` is `false`
    // for the `s1` row and `true` for the `s2` row — non-trivial by construction, and
    // shared across every admissible-arm test below so each test only has to vary the
    // INNER shape under study.
    /// The named graph the `graph_*` arm tests read from.
    const ARM_GRAPH: &str = "http://example.org/g1";

    fn arm_ds() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let knows = b.intern_iri(&format!("{EX}knows"));
        let member = b.intern_iri(&format!("{EX}member"));
        let club = b.intern_iri(&format!("{EX}club"));
        let s1 = b.intern_iri(&format!("{EX}s1"));
        let s2 = b.intern_iri(&format!("{EX}s2"));
        let o1 = b.intern_iri(&format!("{EX}o1"));
        let g1 = b.intern_iri(ARM_GRAPH);
        b.push_quad(s1, knows, o1, None);
        b.push_quad(s2, knows, o1, None);
        b.push_quad(s2, member, club, None);
        // The same `s2`-only fact again, but inside a NAMED graph — the fixture the
        // `graph_*` arm tests read from (default-graph triples above are untouched).
        b.push_quad(s2, member, club, Some(g1));
        b.freeze().expect("freeze")
    }

    /// The shared outer pattern: `?s :knows ?o` (rows `s=s1` and `s=s2`).
    fn arm_outer() -> GraphPattern {
        bgp1(tvar("s"), &format!("{EX}knows"), tvar("o"))
    }

    #[test]
    fn exists_paths_agree_on_bgp() {
        let ds = arm_ds();
        // { ?s :member ?m } — true only for s2.
        let inner = bgp1(tvar("s"), &format!("{EX}member"), tvar("m"));
        assert_probe_and_definition_agree(&ds, &arm_outer(), &inner);
    }

    #[test]
    fn exists_paths_agree_on_path() {
        let ds = arm_ds();
        // { ?s :member/^:member ?s } via a Path node rather than a Bgp leaf —
        // structurally the same "always admissible" family, exercised through the
        // OTHER leaf variant `probe_admissible` names explicitly.
        let inner = GraphPattern::Path {
            subject: tvar("s"),
            path: PropertyPathExpression::NamedNode(nn(&format!("{EX}member"))),
            object: tvar("m"),
        };
        assert_probe_and_definition_agree(&ds, &arm_outer(), &inner);
    }

    #[test]
    fn exists_paths_agree_on_values() {
        // `probe_admissible`'s `Values` arm refuses a VALUES block whenever one of its
        // OWN columns is in `current_row_vars` — `pattern`'s root free-variable set
        // INTERSECTED with the caller's outer schema (`probe_admissible`'s doc). Here
        // `?dummy` is narrowed out TWICE over: it is projected away by the outer
        // `Project { variables: [s] }` (so it never reaches the root's own
        // free-variable set at all) AND, independently, `?dummy` is not a column of
        // `arm_outer`'s schema (`{s, o}`) either — either narrowing alone would already
        // make this shape admissible after the outer-schema fix (see
        // `fresh_values_column_inner_probe_admits` for the same shape with the
        // `Project` wrapper removed, relying on the outer-schema intersection alone).
        // The correlation channel is the ordinary Bgp leaf on `?s`; the sibling VALUES
        // block enumerates two `?dummy` rows (doubling multiplicity, never emptiness).
        let ds = arm_ds();
        let inner = GraphPattern::Project {
            inner: bx(GraphPattern::Join {
                left: bx(bgp1(tvar("s"), &format!("{EX}member"), tvar("m"))),
                right: bx(GraphPattern::Values {
                    variables: vec![var("dummy")],
                    bindings: vec![
                        vec![Some(GroundTerm::Literal(Literal::new_simple("d1")))],
                        vec![Some(GroundTerm::Literal(Literal::new_simple("d2")))],
                    ],
                }),
            }),
            variables: vec![var("s")],
        };
        assert_probe_and_definition_agree(&ds, &arm_outer(), &inner);
    }

    #[test]
    fn exists_paths_agree_on_graph_iri_name() {
        let ds = arm_ds();
        // `GRAPH <ARM_GRAPH> { ?s :member ?m }` — a FIXED graph name; `admissible_rec`'s
        // `Graph` arm ignores the name entirely and defers to the inner.
        let inner = GraphPattern::Graph {
            name: NamedNodePattern::NamedNode(nn(ARM_GRAPH)),
            inner: bx(bgp1(tvar("s"), &format!("{EX}member"), tvar("m"))),
        };
        assert_probe_and_definition_agree(&ds, &arm_outer(), &inner);
    }

    #[test]
    fn exists_paths_agree_on_graph_variable_name() {
        let ds = arm_ds();
        // `GRAPH ?g { ?s :member ?m }` — a VARIABLE graph name; still admissible (the
        // `Graph` arm is name-blind), and `?g` is free but never collides with anything.
        let inner = GraphPattern::Graph {
            name: pred_var("g"),
            inner: bx(bgp1(tvar("s"), &format!("{EX}member"), tvar("m"))),
        };
        assert_probe_and_definition_agree(&ds, &arm_outer(), &inner);
    }

    #[test]
    fn exists_paths_agree_on_join() {
        let ds = arm_ds();
        // `{ ?s :member ?m . ?k :knows ?anyone }` — the second (Join right) operand is
        // an unrelated, always-nonempty Bgp, so the join's own admissibility genuinely
        // exercises "both operands admissible", not merely a single-leaf shape.
        let inner = GraphPattern::Join {
            left: bx(bgp1(tvar("s"), &format!("{EX}member"), tvar("m"))),
            right: bx(bgp1(tvar("k"), &format!("{EX}knows"), tvar("anyone"))),
        };
        assert_probe_and_definition_agree(&ds, &arm_outer(), &inner);
    }

    #[test]
    fn exists_paths_agree_on_union() {
        let ds = arm_ds();
        // `{ ?s :member ?m } UNION { ?s :nosuchpred ?never }` — the right branch never
        // matches anything, so the union's truth value tracks the left branch alone,
        // through `admissible_rec`'s dedicated `Union` arm (both branches admissible).
        let inner = GraphPattern::Union {
            left: bx(bgp1(tvar("s"), &format!("{EX}member"), tvar("m"))),
            right: bx(bgp1(tvar("s"), &format!("{EX}nosuchpred"), tvar("never"))),
        };
        assert_probe_and_definition_agree(&ds, &arm_outer(), &inner);
    }

    #[test]
    fn exists_paths_agree_on_filter_certainly_bound() {
        let ds = arm_ds();
        // `{ ?s :member ?m FILTER(?m = :club) }` — `?m` is read by the filter
        // expression, but it is CERTAINLY bound by the Bgp leaf that produces it
        // (`crate::governor::soundness::analyze_pattern`'s Bgp rule: free = certainly
        // bound), so `expr_probe_admissible` accepts it despite `?m` sitting in the
        // root's (conservative) `current_row_vars` set.
        let inner = GraphPattern::Filter {
            expr: Expression::Equal(
                Box::new(Expression::Variable(var("m"))),
                Box::new(Expression::NamedNode(nn(&format!("{EX}club")))),
            ),
            inner: bx(bgp1(tvar("s"), &format!("{EX}member"), tvar("m"))),
        };
        assert_probe_and_definition_agree(&ds, &arm_outer(), &inner);
    }

    #[test]
    fn exists_paths_agree_on_extend_fresh_target() {
        let ds = arm_ds();
        // `SELECT ?s WHERE { ?s :member ?m BIND("x" AS ?fresh) }` — `admissible_rec`'s
        // own `Extend` arm inserts `fresh` into the root's own free-variable set
        // (`crate::governor::soundness::analyze_pattern`'s `Extend` rule), but
        // `probe_admissible`'s `current_row_vars` intersects that with the outer
        // schema (`{s, o}`, from `arm_outer`), and `?fresh` is not one of its columns
        // either way — so this shape is admissible both because the outer `Project {
        // variables: [s] }` narrows `?fresh` back out of the root's own free-variable
        // set, AND independently because of the outer-schema intersection (see
        // `fresh_extend_target_inner_probe_admits` for the same shape with the
        // `Project` wrapper removed).
        let inner = GraphPattern::Project {
            inner: bx(GraphPattern::Extend {
                inner: bx(bgp1(tvar("s"), &format!("{EX}member"), tvar("m"))),
                variable: var("fresh"),
                expression: Expression::Literal(Literal::new_simple("x")),
            }),
            variables: vec![var("s")],
        };
        assert_probe_and_definition_agree(&ds, &arm_outer(), &inner);
    }

    // =========================================================================
    // `probe_admissible` receives the caller's ACTUAL outer schema (`outer_schema ∩
    // pattern`'s own free vars — `probe_admissible`'s doc), rather than treating
    // `pattern`'s own root free-variable set as a stand-in for it. The three tests below are the SAME
    // families as `exists_paths_agree_on_values`/`exists_paths_agree_on_extend_fresh_target`
    // above and the nested-`EXISTS` arm of `expr_probe_admissible`, but with the
    // `Project`-narrowing trick those two needed REMOVED — before this fix, EVERY
    // one of these three shapes was refused unconditionally (no `Project` narrows
    // them here), regardless of whether the outer schema (`{s, o}`, from
    // `arm_outer`) could ever actually have supplied a colliding binding.
    // =========================================================================

    #[test]
    fn fresh_extend_target_inner_probe_admits() {
        // `EXISTS { ?s :member ?m BIND("x" AS ?fresh) }` — no enclosing `Project`
        // narrows `?fresh` out of the tree's own free-variable set (unlike
        // `exists_paths_agree_on_extend_fresh_target`), so this shape is admissible
        // ONLY through the outer-schema intersection: `?fresh` is not a column of
        // `arm_outer`'s schema (`{s, o}`), so it can never collide with anything the
        // outer row could bind, and `admissible_rec`'s `Extend` arm admits it.
        let ds = arm_ds();
        let inner = GraphPattern::Extend {
            inner: bx(bgp1(tvar("s"), &format!("{EX}member"), tvar("m"))),
            variable: var("fresh"),
            expression: Expression::Literal(Literal::new_simple("x")),
        };
        assert_classified_admissible(&arm_outer(), &inner);
        assert_probe_and_definition_agree(&ds, &arm_outer(), &inner);

        // Ledger counters: through the full governed engine, this site must be
        // answered by the memoized probe (`exists-probe-answered`), never the
        // per-row definition — the direct observable the CLI's `--explain` also
        // reports (see this gap's acceptance demonstration).
        let query = format!(
            "PREFIX ex: <{EX}> \
             SELECT ?s WHERE {{ \
               ?s ex:knows ?o \
               FILTER EXISTS {{ ?s ex:member ?m BIND(\"x\" AS ?fresh) }} \
             }}"
        );
        let engine = crate::engine::NativeSparqlEngine::new();
        let explanation = engine.explain_query(&ds, &query, None).expect("explain");
        let definition_answered: u64 = explanation
            .ledger()
            .iter()
            .map(|node| node.fuel_at(crate::governor::ChargePoint::ExistsDefinitionAnswered))
            .sum();
        let probe_answered: u64 = explanation
            .ledger()
            .iter()
            .map(|node| node.fuel_at(crate::governor::ChargePoint::ExistsProbeAnswered))
            .sum();
        assert_eq!(
            definition_answered, 0,
            "a fresh BIND target must never fall back to the per-row definition path"
        );
        assert_eq!(
            probe_answered, 1,
            "the memoized probe must answer this site exactly once (cached across both \
             outer rows)"
        );
    }

    #[test]
    fn fresh_values_column_inner_probe_admits() {
        // `EXISTS { ?s :member ?m . VALUES ?fresh { "x" } }` — the `VALUES` twin of
        // `fresh_extend_target_inner_probe_admits`: no enclosing `Project` narrows
        // `?fresh` out of the tree's own free-variable set, so this shape is
        // admissible only through the outer-schema intersection (`?fresh` is not a
        // column of `arm_outer`'s `{s, o}` schema).
        let ds = arm_ds();
        let inner = GraphPattern::Join {
            left: bx(bgp1(tvar("s"), &format!("{EX}member"), tvar("m"))),
            right: bx(GraphPattern::Values {
                variables: vec![var("fresh")],
                bindings: vec![vec![Some(GroundTerm::Literal(Literal::new_simple("x")))]],
            }),
        };
        assert_classified_admissible(&arm_outer(), &inner);
        assert_probe_and_definition_agree(&ds, &arm_outer(), &inner);

        // Ledger counters, same discipline as `fresh_extend_target_inner_probe_admits`.
        let query = format!(
            "PREFIX ex: <{EX}> \
             SELECT ?s WHERE {{ \
               ?s ex:knows ?o \
               FILTER EXISTS {{ ?s ex:member ?m . VALUES ?fresh {{ \"x\" }} }} \
             }}"
        );
        let engine = crate::engine::NativeSparqlEngine::new();
        let explanation = engine.explain_query(&ds, &query, None).expect("explain");
        let definition_answered: u64 = explanation
            .ledger()
            .iter()
            .map(|node| node.fuel_at(crate::governor::ChargePoint::ExistsDefinitionAnswered))
            .sum();
        let probe_answered: u64 = explanation
            .ledger()
            .iter()
            .map(|node| node.fuel_at(crate::governor::ChargePoint::ExistsProbeAnswered))
            .sum();
        assert_eq!(
            definition_answered, 0,
            "a fresh VALUES column must never fall back to the per-row definition path"
        );
        assert_eq!(
            probe_answered, 1,
            "the memoized probe must answer this site exactly once (cached across both \
             outer rows)"
        );
    }

    #[test]
    fn nested_exists_with_disjoint_free_vars_probe_admits() {
        // `EXISTS { ?s :member ?m FILTER EXISTS { ?nx :club ?ny } }` — the NESTED
        // `EXISTS`'s own free variables (`?nx`, `?ny`) are disjoint from the outer
        // schema (`{s, o}`). Before this fix, `expr_probe_admissible`'s `Exists` arm
        // tested the nested inner's free vars against the WHOLE enclosing tree's own
        // free-variable set — which, per `NodeAnalysis::free_vars`'s doc, already
        // includes a nested `EXISTS`'s own free variables as part of the enclosing
        // `Filter` expression's free vars — so `?nx`/`?ny` were (wrongly) treated as
        // potential outer-row collisions and the whole tree was refused, regardless
        // of whether `arm_outer`'s schema could ever have bound them. After the fix,
        // `current_row_vars` is intersected with the real outer schema first, so
        // `?nx`/`?ny` — never a column of it — no longer trip the refusal.
        let mut b = RdfDatasetBuilder::new();
        let knows = b.intern_iri(&format!("{EX}knows"));
        let member = b.intern_iri(&format!("{EX}member"));
        let club = b.intern_iri(&format!("{EX}club"));
        let s1 = b.intern_iri(&format!("{EX}s1"));
        let s2 = b.intern_iri(&format!("{EX}s2"));
        let o1 = b.intern_iri(&format!("{EX}o1"));
        let witness = b.intern_iri(&format!("{EX}witness"));
        let anyone = b.intern_iri(&format!("{EX}anyone"));
        b.push_quad(s1, knows, o1, None);
        b.push_quad(s2, knows, o1, None);
        // Only `s2` has the `:member` fact — the correlation channel, matching
        // `arm_ds`'s own `s1`/`s2` split.
        b.push_quad(s2, member, club, None);
        // A witness fact totally unrelated to `?s`/`?o`, making the nested EXISTS
        // unconditionally true — its own variables (`?nx`, `?ny`) never appear
        // anywhere else in this shape.
        b.push_quad(witness, club, anyone, None);
        let ds = b.freeze().expect("freeze");

        let outer = bgp1(tvar("s"), &format!("{EX}knows"), tvar("o"));
        let inner = GraphPattern::Filter {
            expr: Expression::Exists(bx(bgp1(tvar("nx"), &format!("{EX}club"), tvar("ny")))),
            inner: bx(bgp1(tvar("s"), &format!("{EX}member"), tvar("m"))),
        };

        assert_classified_admissible(&outer, &inner);
        assert_probe_and_definition_agree(&ds, &outer, &inner);

        // Ledger counters: BOTH the outer `FILTER EXISTS` site and the nested one
        // must be answered by the memoized probe.
        let query = format!(
            "PREFIX ex: <{EX}> \
             SELECT ?s WHERE {{ \
               ?s ex:knows ?o \
               FILTER EXISTS {{ ?s ex:member ?m FILTER EXISTS {{ ?nx ex:club ?ny }} }} \
             }}"
        );
        let engine = crate::engine::NativeSparqlEngine::new();
        let explanation = engine.explain_query(&ds, &query, None).expect("explain");
        let definition_answered: u64 = explanation
            .ledger()
            .iter()
            .map(|node| node.fuel_at(crate::governor::ChargePoint::ExistsDefinitionAnswered))
            .sum();
        let probe_answered: u64 = explanation
            .ledger()
            .iter()
            .map(|node| node.fuel_at(crate::governor::ChargePoint::ExistsProbeAnswered))
            .sum();
        assert_eq!(
            definition_answered, 0,
            "neither the outer nor the nested EXISTS site may fall back to the per-row \
             definition path — both are disjoint from the outer schema"
        );
        assert_eq!(
            probe_answered, 2,
            "one memoized-probe answer per site (outer + nested), each cached once"
        );
    }

    #[test]
    fn exists_paths_agree_on_order_by() {
        let ds = arm_ds();
        // `ORDER BY` sits at the TOP of a plain EXISTS inner is erased outright by
        // `crate::enf`'s Law 2 (sorting cannot change emptiness) before
        // `probe_admissible` ever sees it — so to actually exercise
        // `admissible_rec`'s OWN `OrderBy` arm (rather than redundantly re-testing
        // `Bgp` after ENF erasure), the `ORDER BY` is placed OFF the emptiness-observed
        // spine, as the right operand of a `Join` (a construct ENF's `normalize` never
        // recurses into — see `crate::enf`'s module doc and its
        // `enf_laws_do_not_fire_off_spine` test).
        let inner = GraphPattern::Join {
            left: bx(bgp1(tvar("k"), &format!("{EX}knows"), tvar("anyone"))),
            right: bx(GraphPattern::OrderBy {
                inner: bx(bgp1(tvar("s"), &format!("{EX}member"), tvar("m"))),
                expression: vec![OrderExpression::Asc(Expression::Variable(var("m")))],
            }),
        };
        assert_probe_and_definition_agree(&ds, &arm_outer(), &inner);
    }

    #[test]
    fn exists_paths_agree_on_project() {
        let ds = arm_ds();
        // `Project` is never erased by ENF (it is rebuilt over its normalized child —
        // `crate::enf`'s module doc), so a plain top-level `SELECT ?s WHERE { ?s
        // :member ?m }` already exercises `admissible_rec`'s dedicated `Project` arm
        // directly, with no off-spine trick needed.
        let inner = GraphPattern::Project {
            inner: bx(bgp1(tvar("s"), &format!("{EX}member"), tvar("m"))),
            variables: vec![var("s")],
        };
        assert_probe_and_definition_agree(&ds, &arm_outer(), &inner);
    }

    #[test]
    fn exists_paths_agree_on_distinct() {
        let ds = arm_ds();
        // Off-spine for the same reason as `order_by`: `DISTINCT` directly on the spine
        // is erased by ENF Law 3 before `probe_admissible` runs.
        let inner = GraphPattern::Join {
            left: bx(bgp1(tvar("k"), &format!("{EX}knows"), tvar("anyone"))),
            right: bx(GraphPattern::Distinct {
                inner: bx(bgp1(tvar("s"), &format!("{EX}member"), tvar("m"))),
            }),
        };
        assert_probe_and_definition_agree(&ds, &arm_outer(), &inner);
    }

    #[test]
    fn exists_paths_agree_on_reduced() {
        let ds = arm_ds();
        // Off-spine for the same reason as `order_by`/`distinct`: `REDUCED` directly on
        // the spine is erased by ENF Law 3 before `probe_admissible` runs.
        let inner = GraphPattern::Join {
            left: bx(bgp1(tvar("k"), &format!("{EX}knows"), tvar("anyone"))),
            right: bx(GraphPattern::Reduced {
                inner: bx(bgp1(tvar("s"), &format!("{EX}member"), tvar("m"))),
            }),
        };
        assert_probe_and_definition_agree(&ds, &arm_outer(), &inner);
    }

    // =========================================================================
    // Divergence witnesses: INADMISSIBLE arms, forced onto the probe anyway.
    // =========================================================================

    #[test]
    fn probe_would_diverge_on_minus() {
        // `EXISTS { ?z :base "1" MINUS { ?z :excluded ?s } }` — the correlated `?s`
        // occurs ONLY in the SUBTRACTED (right) operand, which contributes nothing to
        // `Minus`'s own output schema (`analyze_pattern`'s rule: a `Minus`'s columns are
        // exactly its LEFT side's), so `?s` is not even a shared column between the
        // outer schema and the inner's — the probe degenerates to "is the unconstrained
        // bag nonempty at all", ignoring which outer value is being tested.
        //
        // Dataset: `z1 :excluded cA` is the ONLY `:excluded` fact. Correct (per-row)
        // answer: substituting `?s = cA` makes the right operand match `z1`, so the left
        // row is subtracted — EXISTS is FALSE. Substituting any OTHER value (`other`)
        // makes the right operand match nothing, so the left row survives — EXISTS is
        // TRUE. But the UNCONSTRAINED right operand (evaluated once, `?s` free) DOES
        // find `z1 :excluded cA`, compatible with the left row's `?z = z1` — so the
        // ONE-SHOT probe subtracts the left row regardless of outer value, leaving an
        // EMPTY unconstrained bag and answering FALSE for every outer row, including
        // `other`, where the correct answer is TRUE.
        let mut b = RdfDatasetBuilder::new();
        let tag = b.intern_iri(&format!("{EX}tag"));
        let base = b.intern_iri(&format!("{EX}base"));
        let excluded = b.intern_iri(&format!("{EX}excluded"));
        let one = b.intern_literal(RdfLiteral::simple("1"));
        let anything = b.intern_literal(RdfLiteral::simple("x"));
        let z1 = b.intern_iri(&format!("{EX}z1"));
        let ca = b.intern_iri(&format!("{EX}cA"));
        let other = b.intern_iri(&format!("{EX}other"));
        b.push_quad(ca, tag, anything, None);
        b.push_quad(other, tag, anything, None);
        b.push_quad(z1, base, one, None);
        b.push_quad(z1, excluded, ca, None);
        let ds = b.freeze().expect("freeze");

        let outer = bgp1(tvar("s"), &format!("{EX}tag"), tvar("w"));
        let inner = GraphPattern::Minus {
            left: bx(GraphPattern::Bgp {
                patterns: vec![TriplePattern {
                    subject: tvar("z"),
                    predicate: pred(&format!("{EX}base")),
                    object: TermPattern::Literal(Literal::new_simple("1")),
                }],
            }),
            right: bx(bgp1(tvar("z"), &format!("{EX}excluded"), tvar("s"))),
        };

        assert_classified_inadmissible(&outer, &inner);

        let correct = natural_answers(&ds, &outer, &inner);
        let probed = forced_answers(&ds, &outer, &inner, ForcedExistsStrategy::Probe);

        assert_eq!(
            correct,
            vec![false, true],
            "correct per-row answer: FALSE for `cA` (subtracted), TRUE for `other` (not \
             subtracted) — outer rows are `[cA, other]` in `?s :tag ?w`'s solution order"
        );
        assert_eq!(
            probed,
            vec![false, false],
            "the forced probe wrongly answers FALSE for `other` too: the unconstrained \
             single evaluation already subtracted the left row via `cA`, leaving nothing \
             for ANY outer value to find"
        );
        assert_ne!(
            correct, probed,
            "the forced probe must diverge from the correct answer"
        );
    }

    #[test]
    fn probe_would_diverge_on_slice() {
        // `EXISTS { ?z :item ?s } OFFSET 1` — a restricting-offset `Slice` (never erased
        // by ENF regardless of position: "`start > 0` is NOT one of the transparent
        // wrappers" — `crate::enf`'s module doc). Values Insertion restricts the leaf's
        // own match set to exactly the substituted `?s` value BEFORE the offset is
        // applied (`crate::expr::substitute_pattern`'s doc), so the per-row CORRECT
        // answer depends on how MANY `:item` triples that specific value has: `cA` has
        // two (offset 1 leaves one — non-empty), `cB` has one (offset 1 empties it).
        //
        // The forced probe evaluates the WHOLE `Slice` UNCONSTRAINED, once: the offset
        // drops exactly one row from the raw (unrestricted) 3-row scan, in whatever
        // order the engine enumerates it — a row count and identity no single outer
        // value's substitution reproduces, since the offset there is computed against
        // the FULL (both variables) unconstrained bag, not against the up-to-3-row
        // per-value slice the correct definition computes.
        let mut b = RdfDatasetBuilder::new();
        let tag = b.intern_iri(&format!("{EX}tag"));
        let item = b.intern_iri(&format!("{EX}item"));
        let anything = b.intern_literal(RdfLiteral::simple("x"));
        let za = b.intern_iri(&format!("{EX}zA"));
        let zb = b.intern_iri(&format!("{EX}zB"));
        let zc = b.intern_iri(&format!("{EX}zC"));
        let ca = b.intern_iri(&format!("{EX}cA"));
        let cb = b.intern_iri(&format!("{EX}cB"));
        b.push_quad(ca, tag, anything, None);
        b.push_quad(cb, tag, anything, None);
        b.push_quad(za, item, ca, None);
        b.push_quad(zb, item, ca, None);
        b.push_quad(zc, item, cb, None);
        let ds = b.freeze().expect("freeze");

        let outer = bgp1(tvar("s"), &format!("{EX}tag"), tvar("w"));
        let inner = GraphPattern::Slice {
            inner: bx(bgp1(tvar("z"), &format!("{EX}item"), tvar("s"))),
            start: 1,
            length: None,
        };

        assert_classified_inadmissible(&outer, &inner);

        let correct = natural_answers(&ds, &outer, &inner);
        let probed = forced_answers(&ds, &outer, &inner, ForcedExistsStrategy::Probe);

        assert_eq!(
            correct,
            vec![true, false],
            "correct per-row answer: TRUE for `cA` (two `:item` facts, offset 1 leaves \
             one), FALSE for `cB` (one `:item` fact, offset 1 empties it) — outer rows \
             are `[cA, cB]` in `?s :tag ?w`'s solution order"
        );
        assert_eq!(
            probed,
            vec![true, true],
            "the forced probe wrongly answers TRUE for `cB` too: the unconstrained \
             OFFSET-1 pass drops one row from the combined 3-row raw scan and leaves two \
             behind — one for each distinct `?s` value — so BOTH values probe as \
             non-empty, even though `cB`'s own (single-match) per-value slice is empty"
        );
        assert_ne!(
            correct, probed,
            "the forced probe must diverge from the correct per-value answer"
        );
    }

    #[test]
    fn probe_would_diverge_on_group() {
        // `EXISTS { SELECT (COUNT(*) AS ?c) WHERE { ?z :item ?x FILTER(?x = ?s) } } FILTER(?c > 1)`
        // — `?s` is read only by the Filter INSIDE the Group's own input, never by a
        // grouping key or an aggregate output variable, so it is not a column of the
        // Group's own (runtime) schema at all: `shared` between the outer schema and
        // the inner's ends up empty, just as in the `minus`/`slice` witnesses above.
        //
        // Dataset: `cA` has TWO `:item` facts, `cB` has ONE. Correct per-row answer:
        // substituting `?s = cA` filters the group's input to 2 rows (`COUNT(*) = 2`,
        // `2 > 1` — TRUE); `?s = cB` filters it to 1 row (`COUNT(*) = 1`, `1 > 1` —
        // FALSE).
        //
        // The forced probe evaluates the WHOLE thing UNCONSTRAINED, once: with `?s`
        // free, `?x = ?s` is indeterminate on every candidate row (an unbound operand),
        // so the Filter drops every row, the implicit single group aggregates over an
        // EMPTY input (`COUNT(*) = 0` — SPARQL's "an aggregate always yields a value,
        // even over an empty group"), and the outer `?c > 1` then discards that one row
        // too — leaving an EMPTY unconstrained bag that answers FALSE for every outer
        // row, including `cA`, where the correct answer is TRUE.
        let mut b = RdfDatasetBuilder::new();
        let tag = b.intern_iri(&format!("{EX}tag"));
        let item = b.intern_iri(&format!("{EX}item"));
        let anything = b.intern_literal(RdfLiteral::simple("x"));
        let za = b.intern_iri(&format!("{EX}zA"));
        let zb = b.intern_iri(&format!("{EX}zB"));
        let zc = b.intern_iri(&format!("{EX}zC"));
        let ca = b.intern_iri(&format!("{EX}cA"));
        let cb = b.intern_iri(&format!("{EX}cB"));
        b.push_quad(ca, tag, anything, None);
        b.push_quad(cb, tag, anything, None);
        b.push_quad(za, item, ca, None);
        b.push_quad(zb, item, ca, None);
        b.push_quad(zc, item, cb, None);
        let ds = b.freeze().expect("freeze");

        let outer = bgp1(tvar("s"), &format!("{EX}tag"), tvar("w"));
        let group = GraphPattern::Group {
            inner: bx(GraphPattern::Filter {
                expr: Expression::Equal(
                    Box::new(Expression::Variable(var("x"))),
                    Box::new(Expression::Variable(var("s"))),
                ),
                inner: bx(bgp1(tvar("z"), &format!("{EX}item"), tvar("x"))),
            }),
            variables: Vec::new(),
            aggregates: vec![(
                var("c"),
                AggregateExpression::new(AggregateFunction::Count, Vec::new(), Vec::new(), false)
                    .expect("fixture: valid AggregateExpression"),
            )],
        };
        let inner = GraphPattern::Filter {
            expr: Expression::Greater(
                Box::new(Expression::Variable(var("c"))),
                Box::new(Expression::Literal(Literal::new_typed(
                    "1",
                    nn("http://www.w3.org/2001/XMLSchema#integer"),
                ))),
            ),
            inner: bx(group),
        };

        assert_classified_inadmissible(&outer, &inner);

        let correct = natural_answers(&ds, &outer, &inner);
        let probed = forced_answers(&ds, &outer, &inner, ForcedExistsStrategy::Probe);

        assert_eq!(
            correct,
            vec![true, false],
            "correct per-row answer: TRUE for `cA` (COUNT = 2), FALSE for `cB` \
             (COUNT = 1) — outer rows are `[cA, cB]` in `?s :tag ?w`'s solution order"
        );
        assert_eq!(
            probed,
            vec![false, false],
            "the forced probe wrongly answers FALSE for `cA` too: the unconstrained \
             pass filters every candidate row out (an unbound `?s` never equals a bound \
             `?x`), aggregates COUNT(*) = 0 over the resulting empty group, and the \
             outer `?c > 1` drops that lone zero-count row"
        );
        assert_ne!(
            correct, probed,
            "the forced probe must diverge from the correct per-value answer"
        );
    }

    #[test]
    fn probe_would_diverge_on_lateral() {
        // `EXISTS { ?z :base "1" . LATERAL { ?zw :item ?x FILTER(?x = ?s) } }` — same
        // `shared = []` construction as `minus`/`group` above: `?s` is read only inside
        // the right operand's Filter expression, never exposed as a schema column of
        // the `Lateral` node itself. (The right operand's OWN driving variable is named
        // `?zw`, deliberately disjoint from the OUTER pattern's `?otag` — the analogue
        // of SEP-0007's parser-enforced no-rebinding rule, which this hand-built algebra
        // must observe by construction since no parser checks it here.)
        //
        // Dataset: `cA` has one matching `:item` fact; `cZ` has none. Correct: `?s = cA`
        // leaves the right operand non-empty (LATERAL non-empty, since the left is
        // always non-empty too) — TRUE. `?s = cZ` empties the right operand entirely —
        // FALSE.
        //
        // The forced probe evaluates the WHOLE thing UNCONSTRAINED, once: with `?s`
        // free, `?x = ?s` is indeterminate on every candidate row, so the right operand
        // is ALWAYS empty regardless of outer value — the unconstrained bag is empty,
        // and the probe answers FALSE for every outer row, including `cA`.
        let mut b = RdfDatasetBuilder::new();
        let tag = b.intern_iri(&format!("{EX}tag"));
        let item = b.intern_iri(&format!("{EX}item"));
        let base = b.intern_iri(&format!("{EX}base"));
        let anything = b.intern_literal(RdfLiteral::simple("x"));
        let one = b.intern_literal(RdfLiteral::simple("1"));
        let za = b.intern_iri(&format!("{EX}zA"));
        let z1 = b.intern_iri(&format!("{EX}z1"));
        let ca = b.intern_iri(&format!("{EX}cA"));
        let cz = b.intern_iri(&format!("{EX}cZ"));
        b.push_quad(ca, tag, anything, None);
        b.push_quad(cz, tag, anything, None);
        b.push_quad(za, item, ca, None);
        b.push_quad(z1, base, one, None);
        let ds = b.freeze().expect("freeze");

        let outer = bgp1(tvar("s"), &format!("{EX}tag"), tvar("otag"));
        let inner = GraphPattern::Lateral {
            left: bx(GraphPattern::Bgp {
                patterns: vec![TriplePattern {
                    subject: tvar("z"),
                    predicate: pred(&format!("{EX}base")),
                    object: TermPattern::Literal(Literal::new_simple("1")),
                }],
            }),
            right: bx(GraphPattern::Filter {
                expr: Expression::Equal(
                    Box::new(Expression::Variable(var("x"))),
                    Box::new(Expression::Variable(var("s"))),
                ),
                inner: bx(bgp1(tvar("zw"), &format!("{EX}item"), tvar("x"))),
            }),
        };

        assert_classified_inadmissible(&outer, &inner);

        let correct = natural_answers(&ds, &outer, &inner);
        let probed = forced_answers(&ds, &outer, &inner, ForcedExistsStrategy::Probe);

        assert_eq!(
            correct,
            vec![true, false],
            "correct per-row answer: TRUE for `cA` (one matching `:item` fact), FALSE \
             for `cZ` (none) — outer rows are `[cA, cZ]` in `?s :tag ?w`'s solution order"
        );
        assert_eq!(
            probed,
            vec![false, false],
            "the forced probe wrongly answers FALSE for `cA` too: the unconstrained pass \
             leaves `?s` free, so `?x = ?s` never holds and the right operand is always \
             empty regardless of outer value"
        );
        assert_ne!(
            correct, probed,
            "the forced probe must diverge from the correct per-value answer"
        );
    }

    /// Assert that EVERY row of `exists_results(ds, outer, inner, None)` is
    /// `Err(EvalError::ExistsScopeCollision { variable, intro })` matching
    /// `expected_variable`/`expected_intro` — and that forcing EITHER strategy
    /// (`ForcedExistsStrategy::Probe`/`Definition`) still hard-errors the same
    /// way, since the collision check (`crate::expr::exists`'s call to
    /// `crate::governor::soundness::exists_row_collision`) runs BEFORE the
    /// probe-vs-definition decision — this is the shared body of
    /// `exists_hard_errors_on_values_collision_regardless_of_strategy` and
    /// `exists_hard_errors_on_extend_collision_regardless_of_strategy` (the
    /// SEP-0007 Part 3 twin of the parser-side collision tests
    /// (`purrdf_sparql_algebra::parser::tests`), at the eval boundary that
    /// runs when no parser ever saw this algebra).
    fn assert_hard_errors_on_every_strategy(
        ds: &Arc<RdfDataset>,
        outer: &GraphPattern,
        inner: &GraphPattern,
        expected_variable: &str,
        expected_intro: &str,
    ) {
        let strategies: [Option<ForcedExistsStrategy>; 3] = [
            None,
            Some(ForcedExistsStrategy::Probe),
            Some(ForcedExistsStrategy::Definition),
        ];
        for strategy in strategies {
            let _guard = strategy.map(force_exists_strategy_for_test);
            for result in exists_results(ds, outer, inner, None) {
                let err = result.expect_err(
                    "a BIND/VALUES collision with the outer row must hard-error under every \
                     strategy (natural, forced-probe, forced-definition)",
                );
                match err {
                    EvalError::ExistsScopeCollision { variable, intro } => {
                        assert_eq!(variable, expected_variable, "unexpected colliding variable");
                        assert_eq!(intro, expected_intro, "unexpected collision-intro wording");
                    }
                    other => panic!("expected EvalError::ExistsScopeCollision, got {other:?}"),
                }
            }
        }
    }

    #[test]
    fn exists_hard_errors_on_values_collision_regardless_of_strategy() {
        // `EXISTS { VALUES ?s { <cA> } }` — `?s` is the SAME variable name the outer row
        // binds. Before this collision check existed, the (un-forced) definition path
        // used to evaluate this UNCHANGED — `crate::expr::substitute_pattern_impl`'s `Values`
        // arm never rewrites a `Values` node at all (cloned verbatim: VALUES is not
        // itself a Values-Insertion SITE) — fabricating a CONSTANT `true` answer for
        // every outer row regardless of its own value, while the forced probe
        // fabricated a DIFFERENT, per-row-dependent wrong answer. Now
        // `crate::governor::soundness::exists_row_collision` catches the collision
        // before either strategy runs, so both give way to a single hard error.
        let mut b = RdfDatasetBuilder::new();
        let tag = b.intern_iri(&format!("{EX}tag"));
        let anything = b.intern_literal(RdfLiteral::simple("x"));
        let ca = b.intern_iri(&format!("{EX}cA"));
        let other = b.intern_iri(&format!("{EX}other"));
        b.push_quad(ca, tag, anything, None);
        b.push_quad(other, tag, anything, None);
        let ds = b.freeze().expect("freeze");

        let outer = bgp1(tvar("s"), &format!("{EX}tag"), tvar("w"));
        let inner = GraphPattern::Values {
            variables: vec![var("s")],
            bindings: vec![vec![Some(GroundTerm::NamedNode(nn(&format!("{EX}cA"))))]],
        };

        assert_classified_inadmissible(&outer, &inner);
        assert_hard_errors_on_every_strategy(&ds, &outer, &inner, "s", "VALUES variable");
    }

    #[test]
    fn exists_hard_errors_on_extend_collision_regardless_of_strategy() {
        // `EXISTS { BIND("fixed" AS ?s) }` — `?s` (the `BIND` TARGET) is the SAME
        // variable name the outer row binds. Before this collision check existed, the
        // (un-forced) definition path used to evaluate this UNCHANGED —
        // `crate::expr::substitute_pattern_impl`'s `Extend` arm never touches
        // `variable` (only `expression`/`inner` are rewritten) — fabricating a
        // CONSTANT `true` answer for every outer row (a `BIND` always yields a row),
        // while the forced probe fabricated a DIFFERENT, per-row-dependent wrong
        // answer. Now `crate::governor::soundness::exists_row_collision` catches the
        // collision before either strategy runs, so both give way to a single hard
        // error.
        let mut b = RdfDatasetBuilder::new();
        let tag = b.intern_iri(&format!("{EX}tag"));
        let holder_a = b.intern_iri(&format!("{EX}holderA"));
        let holder_b = b.intern_iri(&format!("{EX}holderB"));
        let fixed = b.intern_literal(RdfLiteral::simple("fixed"));
        let other = b.intern_literal(RdfLiteral::simple("other"));
        b.push_quad(holder_a, tag, fixed, None);
        b.push_quad(holder_b, tag, other, None);
        let ds = b.freeze().expect("freeze");

        // `?s` is the OBJECT here (not the subject, as in the other fixtures above):
        // the `BIND` target it collides with is a STRING literal, and a literal cannot
        // occupy a triple's subject position.
        let outer = bgp1(tvar("w"), &format!("{EX}tag"), tvar("s"));
        let inner = GraphPattern::Extend {
            inner: bx(GraphPattern::Bgp {
                patterns: Vec::new(),
            }),
            variable: var("s"),
            expression: Expression::Literal(Literal::new_simple("fixed")),
        };

        assert_classified_inadmissible(&outer, &inner);
        assert_hard_errors_on_every_strategy(&ds, &outer, &inner, "s", "BIND target");
    }

    /// Build `SELECT ?x (EXISTS { inner } AS ?z) WHERE { ?x :tag ?w }` as raw
    /// [`purrdf_sparql_algebra`] algebra — no `SparqlParser` involved anywhere —
    /// wrap it in a [`PreparedQuery`] via [`PreparedQuery::rewritten`] (the exact
    /// mechanism a rewriting caller — the entailment lane's chase rewrite, a
    /// caller-restricted plan — uses to hand hand-built algebra to a governed
    /// entry), and run it through [`NativeSparqlEngine::query_prepared`], the
    /// PUBLIC evaluation entry point. Panics if the outer `?x :tag ?w` BGP itself
    /// fails to build (it never should — it is a plain triple pattern).
    fn run_hand_built_select(
        ds: &Arc<RdfDataset>,
        exists_inner: GraphPattern,
    ) -> Result<purrdf_core::SparqlResult, purrdf_core::RdfDiagnostic> {
        let outer_pattern = GraphPattern::Extend {
            inner: bx(bgp1(tvar("x"), &format!("{EX}tag"), tvar("w"))),
            variable: var("z"),
            expression: Expression::Exists(bx(exists_inner)),
        };
        let query = purrdf_sparql_algebra::Query::Select {
            pattern: GraphPattern::Project {
                inner: bx(outer_pattern),
                variables: vec![var("x"), var("z")],
            },
            dataset: purrdf_sparql_algebra::QueryDataset::default(),
            base_iri: None,
            version: None,
        };
        let prepared = crate::PreparedQuery::rewritten(query, crate::QueryOptions::EMPTY)
            .expect("no property-function/aggregate registry to fingerprint");
        let engine = crate::NativeSparqlEngine::new();
        engine.query_prepared(ds, &prepared, &[], crate::QueryOptions::EMPTY)
    }

    #[test]
    fn exists_collision_shape_hard_errors_without_the_parser() {
        // The `EXISTS`-side counterpart of the parser-side
        // `select_expression_exists_scope_collision_is_rejected`
        // (`purrdf_sparql_algebra::parser::tests`), but reached with NO parser at
        // all — this is exactly what a SHACL-AF pre-binding, an entailment-chase
        // rewrite, or any other direct caller of this crate's public algebra API
        // can build. `EXISTS { BIND(:fixed AS ?x) }` collides with the outer row's
        // OWN `?x` — before this collision check existed, this construct silently
        // reached the per-row definition path and answered a fabricated `true`
        // for every row (see `exists_hard_errors_on_extend_collision_regardless_of_strategy`'s
        // doc). Through the public engine it must now hard-error instead.
        let mut b = RdfDatasetBuilder::new();
        let tag = b.intern_iri(&format!("{EX}tag"));
        let anything = b.intern_literal(RdfLiteral::simple("x"));
        let holder = b.intern_iri(&format!("{EX}npHolder"));
        b.push_quad(holder, tag, anything, None);
        let ds = b.freeze().expect("freeze");

        let inner = GraphPattern::Extend {
            inner: bx(GraphPattern::Bgp {
                patterns: Vec::new(),
            }),
            variable: var("x"),
            expression: Expression::NamedNode(nn(&format!("{EX}fixed"))),
        };

        let diagnostic =
            run_hand_built_select(&ds, inner).expect_err("a BIND-target/outer-row collision inside EXISTS must hard-error through the public engine, not silently answer");
        assert!(
            diagnostic.message.contains("BIND target ?x inside EXISTS")
                && diagnostic
                    .message
                    .contains("already in scope on the row being filtered"),
            "unexpected diagnostic message: {}",
            diagnostic.message
        );
    }

    #[test]
    fn exists_collision_shape_hard_errors_without_the_parser_values_twin() {
        // The `VALUES`-column twin of `exists_collision_shape_hard_errors_without_the_parser`:
        // `EXISTS { VALUES ?x { :fixed } }` collides with the outer row's own `?x`
        // the same way a `BIND` target does.
        let mut b = RdfDatasetBuilder::new();
        let tag = b.intern_iri(&format!("{EX}tag"));
        let anything = b.intern_literal(RdfLiteral::simple("x"));
        let holder = b.intern_iri(&format!("{EX}npHolder"));
        b.push_quad(holder, tag, anything, None);
        let ds = b.freeze().expect("freeze");

        let inner = GraphPattern::Values {
            variables: vec![var("x")],
            bindings: vec![vec![Some(GroundTerm::NamedNode(nn(&format!("{EX}fixed"))))]],
        };

        let diagnostic =
            run_hand_built_select(&ds, inner).expect_err("a VALUES-column/outer-row collision inside EXISTS must hard-error through the public engine, not silently answer");
        assert!(
            diagnostic
                .message
                .contains("VALUES variable ?x inside EXISTS")
                && diagnostic
                    .message
                    .contains("already in scope on the row being filtered"),
            "unexpected diagnostic message: {}",
            diagnostic.message
        );
    }

    #[test]
    fn exists_collision_shape_hard_errors_without_the_parser_fresh_target_control() {
        // The control: `EXISTS { BIND(:fixed AS ?fresh) }` — `?fresh` is NOT the
        // outer row's `?x`, so this is a genuinely fresh target, not a collision.
        // Wiring the gap-R2 hard-fail must not touch this shape at all — it still
        // evaluates through the SAME public engine entry point used by the two
        // collision tests above.
        let mut b = RdfDatasetBuilder::new();
        let tag = b.intern_iri(&format!("{EX}tag"));
        let anything = b.intern_literal(RdfLiteral::simple("x"));
        let holder = b.intern_iri(&format!("{EX}npHolder"));
        b.push_quad(holder, tag, anything, None);
        let ds = b.freeze().expect("freeze");

        let inner = GraphPattern::Extend {
            inner: bx(GraphPattern::Bgp {
                patterns: Vec::new(),
            }),
            variable: var("fresh"),
            expression: Expression::NamedNode(nn(&format!("{EX}fixed"))),
        };

        let result = run_hand_built_select(&ds, inner)
            .expect("a genuinely fresh BIND target inside EXISTS must still evaluate");
        let purrdf_core::SparqlResult::Solutions { rows, .. } = result else {
            panic!("a SELECT query must materialize as SparqlResult::Solutions, got {result:?}");
        };
        assert_eq!(
            rows.len(),
            1,
            "one outer row (`?x = :npHolder`), EXISTS true (BIND always yields a row)"
        );
    }

    #[test]
    fn exists_definition_path_injects_a_doubly_nested_exists_without_a_false_collision() {
        // The W3C `exists04`/`exists05` shape (`suite/w3c-sparql11/exists/exists04.rq`,
        // `exists05.rq`): `EXISTS { ?s ?p :o1 FILTER EXISTS { ?s ?p :o2 } }` — an
        // off-spine, INADMISSIBLE-for-the-probe `Filter{expr:Exists(nested), inner}`,
        // correlated on BOTH `?s` and `?p`, so the OUTER `exists()` call takes the
        // per-row DEFINITION path (`crate::binop::eval_correlated`). That path's own
        // "Values Insertion" substitution rewrites the correlated leaf `?s ?p :o1` —
        // and, since `substitute_expr`'s `Exists` arm recurses into the nested
        // `EXISTS`'s own inner too, the nested leaf `?s ?p :o2` as well — into
        // `Join(leaf, Values{[s,p]: [[row's values]]})`: a SYNTHETIC `Values` node
        // that is the substitution mechanism's OWN injection vehicle, never a
        // user-written rebinding.
        //
        // `eval_filter` forks a worker per chunk of the OUTER Filter's candidate rows
        // via `EvalCtx::fork_for_worker` (`ctx.may_fork_row_loop`), and the NESTED
        // `EXISTS`'s own `exists()` call runs from INSIDE that forked worker while
        // still logically "inside" the outer substitution window. Before
        // `EvalCtx::fork_for_worker` was fixed to inherit `in_substituted_exists`
        // (it previously hardcoded `false` for every forked worker), the
        // collision check (`crate::governor::soundness::exists_row_collision`,
        // gated on `!ctx.in_substituted_exists`) misread the worker's view of that
        // synthetic `Values` as a genuine SEP-0007 Part 3 violation and hard-errored
        // on legitimate, W3C-vendored SPARQL — this pins the fix.
        let mut b = RdfDatasetBuilder::new();
        let p = nn(&format!("{EX}dblP"));
        let o = nn(&format!("{EX}dblO"));
        let o1 = nn(&format!("{EX}dblO1"));
        let o2 = nn(&format!("{EX}dblO2"));
        let p_id = b.intern_iri(p.as_str());
        let o_id = b.intern_iri(o.as_str());
        let o1_id = b.intern_iri(o1.as_str());
        let o2_id = b.intern_iri(o2.as_str());
        let s1 = b.intern_iri(&format!("{EX}dblS1"));
        let s2 = b.intern_iri(&format!("{EX}dblS2"));
        // `s1` has BOTH `:dblO1` and `:dblO2` — the nested EXISTS is true, so the
        // outer FILTER keeps `?s ?p :dblO1`, and the outer EXISTS is true.
        b.push_quad(s1, p_id, o_id, None);
        b.push_quad(s1, p_id, o1_id, None);
        b.push_quad(s1, p_id, o2_id, None);
        // `s2` has `:dblO1` but NOT `:dblO2` — the nested EXISTS is false, the outer
        // FILTER drops every candidate, and the outer EXISTS is false.
        b.push_quad(s2, p_id, o_id, None);
        b.push_quad(s2, p_id, o1_id, None);
        let ds = b.freeze().expect("freeze");

        let outer = bgp(vec![TriplePattern {
            subject: tvar("s"),
            predicate: pred_var("p"),
            object: TermPattern::NamedNode(o.clone()),
        }]);
        let nested = bgp(vec![TriplePattern {
            subject: tvar("s"),
            predicate: pred_var("p"),
            object: TermPattern::NamedNode(o2.clone()),
        }]);
        let inner = GraphPattern::Filter {
            expr: Expression::Exists(bx(nested)),
            inner: bx(bgp(vec![TriplePattern {
                subject: tvar("s"),
                predicate: pred_var("p"),
                object: TermPattern::NamedNode(o1.clone()),
            }])),
        };

        assert_classified_inadmissible(&outer, &inner);

        let answers = natural_answers(&ds, &outer, &inner);
        assert_eq!(
            answers,
            vec![true, false],
            "s1 (both :dblO1 and :dblO2) is true; s2 (only :dblO1) is false — outer \
             rows are `[s1, s2]` in `?s ?p :dblO`'s solution order"
        );
    }

    #[test]
    fn probe_would_fabricate_on_left_join() {
        // `EXISTS { ?z :base "1" OPTIONAL { ?z :cand ?c FILTER(?c = ?s) } FILTER(!BOUND(?c)) }`
        // — an off-spine `LeftJoin` (a bare `OPTIONAL` at the TOP of an `EXISTS` inner
        // erases outright under ENF Law 1 — `crate::enf`'s module doc, "THE F2 FIX BY
        // LAW" — so this shape sits under a `Filter`, which the spine laws never
        // recurse through, keeping the `LeftJoin` intact). `?s` is read ONLY inside the
        // `LeftJoin`'s join-condition expression, never by a leaf/schema-producing
        // position, so it is not a column of the tree's own schema at all: `shared`
        // between the outer schema and the inner's is empty, exactly like the
        // `minus`/`group`/`lateral` witnesses above.
        //
        // The `FILTER(!BOUND(?c))` on top turns the `OPTIONAL` into an anti-join idiom:
        // it keeps a row only when the RIGHT side did NOT match (the `LeftJoin` padded
        // `?c` unbound). Dataset: `z1 :cand c1`, `z1 :cand c2`. Correct per-row answer:
        // `?s = c1` makes the join condition match the `c1` candidate — `?c` comes back
        // BOUND, so `!BOUND(?c)` drops the row — FALSE. `?s = "other"` (matching no
        // candidate) makes the condition fail for every candidate — `LeftJoin` pads
        // (`?c` unbound), `!BOUND(?c)` keeps it — TRUE.
        //
        // The forced probe evaluates the WHOLE thing UNCONSTRAINED, once: with `?s`
        // free, the join condition `?c = ?s` is indeterminate for EVERY real candidate,
        // so the `LeftJoin` ALWAYS pads, `!BOUND(?c)` ALWAYS keeps the one padded row,
        // and the resulting one-row unconstrained bag — combined with the empty
        // `shared` set — makes the probe answer TRUE for literally every outer value,
        // FABRICATING existence for `?s = c1`, where the correct answer is FALSE.
        let mut b = RdfDatasetBuilder::new();
        let tag = b.intern_iri(&format!("{EX}tag"));
        let base = b.intern_iri(&format!("{EX}base"));
        let cand = b.intern_iri(&format!("{EX}cand"));
        let anything = b.intern_literal(RdfLiteral::simple("x"));
        let one = b.intern_literal(RdfLiteral::simple("1"));
        let z1 = b.intern_iri(&format!("{EX}z1"));
        let c1 = b.intern_iri(&format!("{EX}c1"));
        let c2 = b.intern_iri(&format!("{EX}c2"));
        let other = b.intern_iri(&format!("{EX}other"));
        b.push_quad(c1, tag, anything, None);
        b.push_quad(other, tag, anything, None);
        b.push_quad(z1, base, one, None);
        b.push_quad(z1, cand, c1, None);
        b.push_quad(z1, cand, c2, None);
        let ds = b.freeze().expect("freeze");

        let outer = bgp1(tvar("s"), &format!("{EX}tag"), tvar("w"));
        let left_join = GraphPattern::LeftJoin {
            left: bx(GraphPattern::Bgp {
                patterns: vec![TriplePattern {
                    subject: tvar("z"),
                    predicate: pred(&format!("{EX}base")),
                    object: TermPattern::Literal(Literal::new_simple("1")),
                }],
            }),
            right: bx(bgp1(tvar("z"), &format!("{EX}cand"), tvar("c"))),
            expression: Some(Expression::Equal(
                Box::new(Expression::Variable(var("c"))),
                Box::new(Expression::Variable(var("s"))),
            )),
        };
        let inner = GraphPattern::Filter {
            expr: Expression::Not(Box::new(Expression::Bound(var("c")))),
            inner: bx(left_join),
        };

        assert_classified_inadmissible(&outer, &inner);

        let correct = natural_answers(&ds, &outer, &inner);
        let probed = forced_answers(&ds, &outer, &inner, ForcedExistsStrategy::Probe);

        assert_eq!(
            correct,
            vec![false, true],
            "correct per-row answer: FALSE for `c1` (the condition matches, `?c` comes \
             back bound), TRUE for `other` (no candidate matches, `?c` stays unbound) — \
             outer rows are `[c1, other]` in `?s :tag ?w`'s solution order"
        );
        assert_eq!(
            probed,
            vec![true, true],
            "the forced probe FABRICATES a match for `c1` too: the unconstrained pass \
             leaves `?s` free, so the join condition never matches any real candidate, \
             the `LeftJoin` always pads, and the resulting non-empty one-row bag answers \
             TRUE for every outer value regardless of whether it ever really matched"
        );
        assert_ne!(
            correct, probed,
            "the forced probe must diverge from the correct per-value answer"
        );
    }

    // ---- `probe_would_diverge_on_property_function`'s test-only relation ----

    /// A single-mode (SUBJECT-BOUND ONLY) property function: `(?s) <pf> (?out)`
    /// invoked with `?s` bound to `<cA>` emits one row (`?out = "matched"`); bound to
    /// anything else emits zero rows; the relation declares NO all-free mode at all,
    /// so an invocation reaching it with `?s` UNBOUND is refused outright — the exact
    /// "invocation input the evaluator reads from the current row, not a join key a
    /// post-hoc probe can supply" hazard `probe_admissible`'s `PropertyFunction` arm
    /// doc names.
    #[derive(Debug)]
    struct MembershipRelation {
        modes: [purrdf_core::binding_pattern::BindingPattern; 1],
    }

    impl MembershipRelation {
        fn new() -> Self {
            Self {
                modes: [purrdf_core::binding_pattern::BindingPattern::from_bound_positions(2, [0])],
            }
        }
    }

    impl crate::property_fn::PropertyFunction for MembershipRelation {
        fn volatility(&self) -> crate::user_fn::Volatility {
            crate::user_fn::Volatility::Stable
        }

        fn arity(&self) -> crate::property_fn::PfArity {
            crate::property_fn::PfArity::new(1, 1)
        }

        fn modes(&self) -> &[purrdf_core::binding_pattern::BindingPattern] {
            &self.modes
        }

        fn rows_per_invocation(&self, _mode: purrdf_core::binding_pattern::BindingPattern) -> u64 {
            1
        }

        fn open(
            &self,
            args: &crate::property_fn::PfArgs<'_>,
            _ceiling: Option<u64>,
        ) -> Result<Box<dyn crate::property_fn::PfCursor>, EvalError> {
            let rows = match args.get(0) {
                Some(purrdf_core::TermValue::Iri(subject)) if subject == &format!("{EX}cA") => {
                    vec![vec![
                        purrdf_core::TermValue::Iri(subject.clone()),
                        purrdf_core::TermValue::typed_literal(
                            "matched",
                            "http://www.w3.org/2001/XMLSchema#string",
                        ),
                    ]]
                }
                _ => Vec::new(),
            };
            Ok(Box::new(MembershipCursor { rows, next: 0 }))
        }
    }

    struct MembershipCursor {
        rows: Vec<crate::property_fn::PfRow>,
        next: usize,
    }

    impl crate::property_fn::PfCursor for MembershipCursor {
        fn next(&mut self) -> Result<Option<crate::property_fn::PfRow>, EvalError> {
            let row = self.rows.get(self.next).cloned();
            self.next += 1;
            Ok(row)
        }
    }

    #[test]
    fn probe_would_diverge_on_property_function() {
        // `EXISTS { (?s) <pf> (?out) }` — a bare `PropertyFunction` leaf,
        // unconditionally inadmissible (`admissible_rec`'s `PropertyFunction` arm:
        // `false`, no case analysis at all). `MembershipRelation` above declares
        // EXACTLY ONE access pattern — subject bound — so the definition path (which
        // substitutes the outer `?s` into the call's subject argument BEFORE dispatch:
        // `substitute_term_pattern`'s IRI-only value substitution — see
        // `crate::expr::substitute_pattern`'s doc, "Property-function arguments") always
        // invokes it correctly, while the forced probe evaluates the call UNCONSTRAINED
        // — `?s` still a free VARIABLE, not a bound value — which no declared mode
        // admits at all.
        //
        // This is the case the task notes as possibly "unobservable": the PF fusion
        // contract does not let the probe silently compute a WRONG boolean here — the
        // access-pattern check refuses the call outright, so a mode mismatch is a hard
        // [`crate::error::EvalError::Function`], not a wrong answer. The exclusion is
        // still load-bearing (and observably so): the natural/definition path succeeds
        // with the correct boolean on EVERY outer row, while the forced probe turns the
        // SAME query into a hard failure — the specific "wrong answer" a bare-boolean
        // probe-vs-definition comparison cannot phrase for this arm, but a
        // success-vs-Err comparison can and does.
        let registry = {
            let mut registry = crate::property_fn::PropertyFunctionRegistry::new();
            registry.register("http://example.org/pf", Arc::new(MembershipRelation::new()));
            registry
        };

        let mut b = RdfDatasetBuilder::new();
        let tag = b.intern_iri(&format!("{EX}tag"));
        let anything = b.intern_literal(RdfLiteral::simple("x"));
        let ca = b.intern_iri(&format!("{EX}cA"));
        let other = b.intern_iri(&format!("{EX}other"));
        b.push_quad(ca, tag, anything, None);
        b.push_quad(other, tag, anything, None);
        let ds = b.freeze().expect("freeze");

        let outer = bgp1(tvar("s"), &format!("{EX}tag"), tvar("w"));
        let inner = GraphPattern::PropertyFunction(purrdf_sparql_algebra::PropertyFunctionCall {
            iri: "http://example.org/pf".to_owned(),
            subject_args: vec![tvar("s")],
            object_args: vec![tvar("out")],
        });

        assert_classified_inadmissible(&outer, &inner);

        let correct = natural_results_with_registry(&ds, &outer, &inner, &registry);
        assert_eq!(
            correct.iter().map(Result::is_ok).collect::<Vec<_>>(),
            vec![true, true],
            "the natural (definition) path must answer BOTH outer rows without error: \
             {correct:?}"
        );
        assert_eq!(
            correct.into_iter().map(Result::unwrap).collect::<Vec<_>>(),
            vec![true, false],
            "correct per-row answer: TRUE for `cA` (the only subject the relation \
             matches), FALSE for `other`"
        );

        let probed = forced_results_with_registry(
            &ds,
            &outer,
            &inner,
            ForcedExistsStrategy::Probe,
            &registry,
        );
        assert!(
            probed.iter().all(Result::is_err),
            "the forced probe must hard-error on EVERY outer row: the unconstrained \
             evaluation dispatches the call with `?s` still free, which the relation's \
             sole (subject-bound) mode does not admit — got {probed:?}"
        );
    }

    // =========================================================================
    // Parity cases: error discipline, the SERVICE SILENT swallow point, and the
    // stateful-builtin/variable-SERVICE classification pins.
    // =========================================================================

    #[test]
    fn exists_error_parity_across_strategies() {
        // `EXISTS { ?z :val ?m FILTER(IF(?s = :trigger, <unresolved custom fn>(), true)) }`
        // — a hard `EvalError` (an unresolved `Function::Custom` IRI —
        // `crate::error::UnsupportedKind::CustomFunction`) reachable ONLY when the
        // OUTER `?s` equals `:trigger`. `Function::Custom` inside an `If`'s branch makes
        // the WHOLE `Filter` inadmissible unconditionally
        // (`expr_probe_admissible`'s `FunctionCall` arm, `!matches!(function,
        // Function::Custom(_))`, checked on EVERY `If` branch regardless of which one
        // actually runs), so the NATURAL (unforced) path is always the per-row
        // definition — which correctly hard-errors on the `:trigger` outer row (its own
        // substitution replaces `?s` with the constant `:trigger` before the `Filter`
        // runs, so `IF`'s condition is decidably TRUE and the custom call is reached)
        // and answers cleanly (no error) on every other outer row.
        //
        // This is the ONE place in the suite where forcing the probe does NOT reproduce
        // even a WRONG boolean — it reproduces a SILENT, WRONG SUCCESS. Forced onto this
        // exact shape, the probe evaluates the WHOLE inner UNCONSTRAINED, ONCE, with `?s`
        // still a genuinely free variable (never substituted): `IF`'s condition
        // `?s = :trigger` is therefore INDETERMINATE (an unbound-variable comparison —
        // SPARQL's type-error, `Ok(None)`), and `Expression::If`'s `None` arm evaluates
        // NEITHER branch at all (`crate::expr::eval_expr`'s `If` arm: `Some(true) =>
        // eval T, Some(false) => eval E, None => Ok(None)`) — so the custom call the
        // correct path hard-errors on is never even reached, and the probe would answer
        // `false` for the `:trigger` row too, cleanly, no error. This is exactly why
        // `Function::Custom` makes the containing construct UNCONDITIONALLY
        // inadmissible rather than merely "inadmissible if the erroring branch is
        // provably reached": a conservative static check cannot know which branch a
        // conditional gated on the outer row will take, and the unconstrained probe
        // pass, with the outer row genuinely absent, systematically takes the WRONG one
        // (whichever an unbound comparison resolves to — always "neither").
        let mut b = RdfDatasetBuilder::new();
        let tag = b.intern_iri(&format!("{EX}tag"));
        let val = b.intern_iri(&format!("{EX}val"));
        let anything = b.intern_literal(RdfLiteral::simple("x"));
        let trigger = b.intern_iri(&format!("{EX}trigger"));
        let safe = b.intern_iri(&format!("{EX}safe"));
        let z1 = b.intern_iri(&format!("{EX}z1"));
        b.push_quad(trigger, tag, anything, None);
        b.push_quad(safe, tag, anything, None);
        b.push_quad(z1, val, anything, None);
        let ds = b.freeze().expect("freeze");

        let outer = bgp1(tvar("s"), &format!("{EX}tag"), tvar("w"));
        let inner = GraphPattern::Filter {
            expr: Expression::If(
                Box::new(Expression::Equal(
                    Box::new(Expression::Variable(var("s"))),
                    Box::new(Expression::NamedNode(nn(&format!("{EX}trigger")))),
                )),
                Box::new(Expression::FunctionCall(
                    Function::Custom(nn("http://example.org/undefined-fn")),
                    Vec::new(),
                )),
                Box::new(Expression::Literal(Literal::new_typed(
                    "true",
                    nn("http://www.w3.org/2001/XMLSchema#boolean"),
                ))),
            ),
            inner: bx(bgp1(tvar("z"), &format!("{EX}val"), tvar("m"))),
        };

        assert_classified_inadmissible(&outer, &inner);

        // Correct (definition, natural — the gate always refuses this shape): hard
        // errors on the `trigger` row, succeeds cleanly on `safe`.
        let correct = natural_results_with_registry(
            &ds,
            &outer,
            &inner,
            &crate::property_fn::PropertyFunctionRegistry::new(),
        );
        assert_eq!(correct.len(), 2, "two outer rows: `trigger` and `safe`");
        assert!(
            matches!(
                &correct[0],
                Err(EvalError::Unsupported {
                    kind: Some(crate::error::UnsupportedKind::CustomFunction),
                    ..
                })
            ),
            "the `trigger` row must hard-error on the unresolved custom function: {:?}",
            correct[0]
        );
        assert_eq!(
            correct[1],
            Ok(true),
            "the `safe` row never reaches the custom call: `IF`'s condition is FALSE \
             (`safe != :trigger`), so the filter predicate is the literal `true` and the \
             one driving row `?z :val ?m` survives — the inner bag is non-empty, so \
             EXISTS is TRUE"
        );

        let probed = forced_results_with_registry(
            &ds,
            &outer,
            &inner,
            ForcedExistsStrategy::Probe,
            &crate::property_fn::PropertyFunctionRegistry::new(),
        );
        assert_eq!(
            probed,
            vec![Ok(false), Ok(false)],
            "the forced probe hard-errors on NEITHER row: with `?s` free during the one \
             unconstrained evaluation, `IF`'s condition is indeterminate, so the custom \
             call the correct path hard-errors on is never reached at all — the probe \
             silently reports a clean (and, for `trigger`, WRONG) answer instead of the \
             hard failure the correct evaluation raises"
        );
        assert_ne!(
            correct[0].is_err(),
            probed[0].is_err(),
            "the forced probe's error discipline diverges from the correct one on the \
             `trigger` row: correct hard-errors, the forced probe does not"
        );
    }

    #[test]
    fn exists_silent_service_swallow_point_parity() {
        // `EXISTS { ?s :tag ?w2 . SERVICE SILENT <http://example.org/remote> { ?x :p ?y } }`
        // — no `ctx.remote` source is configured, so `crate::remote::eval_service`'s
        // `silent_or_err` swallows the missing source into the JOIN IDENTITY (a single
        // empty-binding row — `crate::remote::identity_seq`'s doc, "`Join(left,
        // identity) == left`, so a swallowed `SERVICE SILENT` leaves the surrounding
        // query unchanged"), uniformly, REGARDLESS of any row's bindings (the endpoint is
        // a FIXED IRI here, not a variable — that per-row-resolution case is
        // `service_variable_inner_classifies_inadmissible`, pinned separately since it
        // has no constructible divergence at all). Both strategies therefore compute the
        // SAME always-identity right operand, so the correlated Join always reduces to
        // its LEFT operand alone for every outer row — a genuine PARITY (not a
        // divergence): the exclusion is still load-bearing in general (per
        // `probe_admissible`'s doc, "a SILENT call can swallow a per-row failure that an
        // evaluate-once pass would never see"), but THIS fixture's failure is uniform,
        // not per-row, so no divergence is observable here — which is exactly what this
        // test pins.
        let mut b = RdfDatasetBuilder::new();
        let tag = b.intern_iri(&format!("{EX}tag"));
        let anything = b.intern_literal(RdfLiteral::simple("x"));
        let sa = b.intern_iri(&format!("{EX}sA"));
        let sb = b.intern_iri(&format!("{EX}sB"));
        b.push_quad(sa, tag, anything, None);
        b.push_quad(sb, tag, anything, None);
        let ds = b.freeze().expect("freeze");

        let outer = bgp1(tvar("s"), &format!("{EX}tag"), tvar("w"));
        let inner = GraphPattern::Join {
            left: bx(bgp1(tvar("s"), &format!("{EX}tag"), tvar("w2"))),
            right: bx(GraphPattern::Service {
                name: NamedNodePattern::NamedNode(nn("http://example.org/remote")),
                inner: bx(bgp1(tvar("x"), &format!("{EX}p"), tvar("y"))),
                silent: true,
            }),
        };

        assert_classified_inadmissible(&outer, &inner);

        let natural = natural_answers(&ds, &outer, &inner);
        let probed = forced_answers(&ds, &outer, &inner, ForcedExistsStrategy::Probe);

        assert_eq!(
            natural,
            vec![true, true],
            "with no remote source configured, SERVICE SILENT swallows to the join \
             identity for every outer row, so the Join reduces to its LEFT operand alone \
             (`?s :tag ?w2`, which both outer rows satisfy)"
        );
        assert_eq!(
            natural, probed,
            "both strategies must agree — the SILENT swallow is uniform, not per-row, in \
             this fixture"
        );
    }

    #[test]
    fn service_variable_inner_classifies_inadmissible() {
        // `SERVICE ?g { ?s :p ?o }` — a VARIABLE endpoint. `admissible_rec`'s `Service`
        // arm is a blanket `false` regardless of whether the name is a variable or a
        // fixed IRI, but the variable-endpoint case is the one with NO constructible
        // divergence WITNESS: forcing the probe on it would need this test to actually
        // dispatch a remote call with the endpoint IRI still unbound, which
        // `crate::remote::eval_service` refuses outright before ever reaching a source
        // (`NamedNodePattern::Variable(_) => silent_or_err(silent, ...)`, "SERVICE with
        // a variable endpoint is not supported (needs lateral evaluation)") — an
        // UNCONDITIONAL refusal, independent of probe/definition strategy, of any
        // `ctx.remote` configuration, and of `silent`'s value (`silent_or_err` still
        // requires SOME outcome; without SILENT it hard-errors either way). There is no
        // configuration under which forcing the probe reaches different code from the
        // natural (definition) path at all — both refuse identically, before any
        // dispatch a "wrong answer" could be observed in. So this test pins the
        // CLASSIFICATION only (the exclusion is real and load-bearing: a variable
        // endpoint's resolution is definitionally per-row, per `probe_admissible`'s doc,
        // "a variable endpoint needs per-row resolution to a concrete IRI"), rather than
        // asserting a divergence no configuration of this engine can produce.
        let inner = GraphPattern::Service {
            name: pred_var("g"),
            inner: bx(bgp1(tvar("s"), &format!("{EX}p"), tvar("o"))),
            silent: false,
        };
        // `Service` is a blanket, outer-schema-independent refusal (`admissible_rec`'s
        // `Service` arm), so an empty outer schema exercises the exact same arm a
        // non-empty one would.
        let outer = bgp(Vec::new());
        assert_classified_inadmissible(&outer, &inner);
    }

    #[test]
    fn stateful_builtin_inner_classifies_inadmissible() {
        // `EXISTS { ?z :base ?b BIND(RAND() AS ?r) FILTER(?r >= 0) FILTER(?s = ?z) }` —
        // `expr_probe_admissible`'s `FunctionCall` arm refuses `RAND()`
        // unconditionally (`function_is_builtin_stateful`), so the whole `Extend` (and
        // therefore the whole tree) is inadmissible regardless of the SEPARATE
        // correlated `?s = ?z` filter — the definition path runs on BOTH `exists_memo`
        // settings (the memo is a pure cache toggle over which evaluation runs; it
        // cannot itself change which strategy is chosen — `EvalOptions::exists_memo`'s
        // doc).
        let ds = {
            let mut b = RdfDatasetBuilder::new();
            let base = b.intern_iri(&format!("{EX}base"));
            let tag = b.intern_iri(&format!("{EX}tag"));
            let anything = b.intern_literal(RdfLiteral::simple("x"));
            let z1 = b.intern_iri(&format!("{EX}z1"));
            let z2 = b.intern_iri(&format!("{EX}z2"));
            b.push_quad(z1, base, anything, None);
            b.push_quad(z2, base, anything, None);
            b.push_quad(z1, tag, anything, None);
            b.push_quad(z2, tag, anything, None);
            b.freeze().expect("freeze")
        };
        let outer = bgp1(tvar("s"), &format!("{EX}tag"), tvar("w"));
        let inner = GraphPattern::Filter {
            expr: Expression::Equal(
                Box::new(Expression::Variable(var("s"))),
                Box::new(Expression::Variable(var("z"))),
            ),
            inner: bx(GraphPattern::Filter {
                expr: Expression::GreaterOrEqual(
                    Box::new(Expression::Variable(var("r"))),
                    Box::new(Expression::Literal(Literal::new_typed(
                        "0",
                        nn("http://www.w3.org/2001/XMLSchema#integer"),
                    ))),
                ),
                inner: bx(GraphPattern::Extend {
                    inner: bx(bgp1(tvar("z"), &format!("{EX}base"), tvar("b"))),
                    variable: var("r"),
                    expression: Expression::FunctionCall(Function::Rand, Vec::new()),
                }),
            }),
        };

        assert_classified_inadmissible(&outer, &inner);

        for memo in [true, false] {
            let mut ctx = EvalCtx::new(&ds);
            ctx.options.exists_memo = memo;
            let seq = eval(&outer, &mut ctx).expect("outer");
            let exists_expr = Expression::Exists(Box::new(inner.clone()));
            for row in &seq.rows {
                crate::expr::eval_ebv(&exists_expr, row, &seq.schema, &mut ctx)
                    .expect("no hard error");
            }
            assert!(
                ctx.exists_inner_cache.is_empty(),
                "memo={memo}: a stateful-builtin inner must never populate the memoized \
                 probe's cache — it always takes the per-row definition path"
            );
        }
    }

    #[test]
    fn stateful_nested_exists_classifies_inadmissible() {
        // `FILTER EXISTS { ?s :q ?z FILTER(EXISTS { :e :q
        // :y1 . FILTER(RAND() < 0.5) }) }` — the OUTER `EXISTS`'s own inner is
        // correlated on `?s` (`?s :q ?z`), and its `Filter`'s expression is a NESTED
        // `EXISTS` whose own inner mentions NO variable at all (`:e :q :y1` are every
        // one a constant), but reaches a stateful `RAND()`. Before the fix,
        // `expr_probe_admissible`'s `Exists` arm asked ONLY whether the nested inner's
        // free variables intersect the current row — trivially "no" here, since it has
        // none — so the WHOLE outer body was classified ADMISSIBLE and the memoized
        // probe evaluated the nested `RAND()` exactly ONCE, shared across every outer
        // row, instead of once per row. `NodeAnalysis::has_stateful_builtin` was
        // already computed correctly through the nested `EXISTS` (see
        // `analyze_expr`'s `Exists` arm) but read by nothing at the admission site —
        // this test pins that it is now consulted.
        let mut b = RdfDatasetBuilder::new();
        let tag = b.intern_iri(&format!("{EX}tag"));
        let q = b.intern_iri(&format!("{EX}q"));
        let anything = b.intern_literal(RdfLiteral::simple("x"));
        let z1 = b.intern_literal(RdfLiteral::simple("z1"));
        let e = b.intern_iri(&format!("{EX}e"));
        let y1 = b.intern_iri(&format!("{EX}y1"));
        let sa = b.intern_iri(&format!("{EX}sA"));
        let sb = b.intern_iri(&format!("{EX}sB"));
        b.push_quad(sa, tag, anything, None);
        b.push_quad(sb, tag, anything, None);
        // Only `sA` has a `?s :q ?z` witness — `sB`'s row never reaches the nested
        // EXISTS at all (its own `Bgp` is already empty), so the ledger assertion
        // below (`ExistsDefinitionAnswered == 2`) also proves the per-row definition
        // is charged even for a row whose inner never touches the stateful builtin.
        b.push_quad(sa, q, z1, None);
        b.push_quad(e, q, y1, None);
        let ds = b.freeze().expect("freeze");

        let nested = GraphPattern::Filter {
            expr: Expression::Less(
                Box::new(Expression::FunctionCall(Function::Rand, Vec::new())),
                Box::new(Expression::Literal(Literal::new_typed(
                    "0.5",
                    nn("http://www.w3.org/2001/XMLSchema#double"),
                ))),
            ),
            inner: bx(bgp1(
                TermPattern::NamedNode(nn(&format!("{EX}e"))),
                &format!("{EX}q"),
                TermPattern::NamedNode(nn(&format!("{EX}y1"))),
            )),
        };
        let inner = GraphPattern::Filter {
            expr: Expression::Exists(bx(nested)),
            inner: bx(bgp1(tvar("s"), &format!("{EX}q"), tvar("z"))),
        };

        // The outer pattern the SPARQL text below lowers to: `?s :tag ?w`.
        let outer = bgp1(tvar("s"), &format!("{EX}tag"), tvar("w"));
        assert_classified_inadmissible(&outer, &inner);

        // `--explain`-level ledger counters, through the full governed engine, over
        // the same shape as SPARQL text.
        let query = format!(
            "PREFIX : <{EX}> \
             SELECT ?s WHERE {{ \
               ?s :tag ?w \
               FILTER EXISTS {{ ?s :q ?z FILTER(EXISTS {{ :e :q :y1 . FILTER(RAND() < 0.5) }}) }} \
             }}"
        );
        let engine = crate::engine::NativeSparqlEngine::new();
        let explanation = engine.explain_query(&ds, &query, None).expect("explain");
        let definition_answered: u64 = explanation
            .ledger()
            .iter()
            .map(|node| node.fuel_at(crate::governor::ChargePoint::ExistsDefinitionAnswered))
            .sum();
        let probe_answered: u64 = explanation
            .ledger()
            .iter()
            .map(|node| node.fuel_at(crate::governor::ChargePoint::ExistsProbeAnswered))
            .sum();
        assert_eq!(
            definition_answered, 2,
            "both outer rows (`sA`, `sB`) must now take the per-row DEFINITION path for \
             the OUTER FILTER EXISTS: {definition_answered} `exists-definition-answered` \
             charges, expected 2 (one per outer row) — before the fix this counter reads \
             0, because the outer body was (wrongly) classified admissible and the \
             memoized probe path ran instead"
        );
        assert_eq!(
            probe_answered, 1,
            "the ONE surviving `exists-probe-answered` charge is the NESTED EXISTS's \
             own — it is genuinely uncorrelated (no free variables at all), so it \
             legitimately takes the probe path and is cached once; the OUTER site no \
             longer contributes to this counter — before the fix this counter reads 2 \
             (one for the outer site, one for the nested one)"
        );
    }

    // =========================================================================
    // The bounded-exhaustive generator (modeled on
    // `crate::parallel_determinism_gate`'s corpus-gate architecture: a fixed dataset
    // and a broad, deterministic shape corpus, each checked for one invariant — but
    // GENERATED from a small algebra grammar rather than hand-written, since the
    // property under test here is "holds for every shape a small grammar reaches",
    // not "holds for this specific hand-picked query mix").
    // =========================================================================

    /// The three leaf shapes every generated `EXISTS` inner is built from — all
    /// correlated on `?s` (the generator's outer variable) as their subject, so `?s`
    /// remains reachable (and, through every operator in [`BINARY_COUNT`]/
    /// [`UNARY_COUNT`]'s alphabet, certainly-bound) no matter how the grammar composes
    /// them. The third leaf is a `RAND() < 0.5`-style stateful-builtin `Filter` — the
    /// generator's ONLY source of
    /// `NodeAnalysis::has_stateful_builtin == true` shapes, which every unary/binary
    /// composition then propagates upward, exercising `expr_probe_admissible`'s
    /// `Exists` arm (directly, once composed under [`UNARY_COUNT`]'s nested-`EXISTS`
    /// operator) at every depth the grammar reaches.
    fn generator_leaves() -> Vec<GraphPattern> {
        vec![
            bgp1(tvar("s"), &format!("{EX}p1"), tvar("x")),
            bgp1(tvar("s"), &format!("{EX}p2"), tvar("y")),
            GraphPattern::Filter {
                expr: Expression::Less(
                    Box::new(Expression::FunctionCall(Function::Rand, Vec::new())),
                    Box::new(Expression::Literal(Literal::new_typed(
                        "0.5",
                        nn("http://www.w3.org/2001/XMLSchema#double"),
                    ))),
                ),
                inner: bx(bgp1(tvar("s"), &format!("{EX}p1"), tvar("x"))),
            },
        ]
    }

    /// The generator's fixed dataset: three subjects, each with a DIFFERENT one of
    /// `:p1`/`:p2`/neither, so the generated shapes' answers vary across outer rows
    /// without needing per-shape fixture tuning. Also carries the same `:p1`/`:p2`
    /// facts inside a named graph (`ARM_GRAPH`-style, but the generator's OWN IRI —
    /// see [`GRAPH_IRI`]) for the widened alphabet's `Graph` unary operator.
    fn generator_ds() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let tag = b.intern_iri(&format!("{EX}outerTag"));
        let p1 = b.intern_iri(&format!("{EX}p1"));
        let p2 = b.intern_iri(&format!("{EX}p2"));
        let anything = b.intern_literal(RdfLiteral::simple("x"));
        let sa = b.intern_iri(&format!("{EX}sA"));
        let sb = b.intern_iri(&format!("{EX}sB"));
        let sc = b.intern_iri(&format!("{EX}sC"));
        let g1 = b.intern_iri(GRAPH_IRI);
        b.push_quad(sa, tag, anything, None);
        b.push_quad(sb, tag, anything, None);
        b.push_quad(sc, tag, anything, None);
        b.push_quad(sa, p1, anything, None);
        b.push_quad(sb, p2, anything, None);
        // `sc` has neither `:p1` nor `:p2` — the "matches nothing" outer row.
        b.push_quad(sa, p1, anything, Some(g1));
        b.push_quad(sb, p2, anything, Some(g1));
        b.freeze().expect("freeze")
    }

    /// The named graph the generator's `Graph` unary operator reads from — mirrors
    /// [`generator_ds`]'s default-graph `:p1`/`:p2` facts so a `GRAPH`-wrapped shape's
    /// answer is non-trivial rather than uniformly empty.
    const GRAPH_IRI: &str = "http://example.org/genGraph";

    /// The generator's outer pattern: `?s :outerTag ?w`, three rows (`sA`, `sB`, `sC`).
    fn generator_outer() -> GraphPattern {
        bgp1(tvar("s"), &format!("{EX}outerTag"), tvar("w"))
    }

    /// Wrap `p` in the `idx`-th unary operator of the generator's alphabet —
    /// `Filter`/`Distinct`/`Slice`/`Project` (the original grammar), plus `Extend`
    /// with a fresh target, `Values` (via a `Join` with a
    /// fresh-column `VALUES` block — the algebra shape a surface `{ p . VALUES ?v {
    /// … } }` lowers to), `Graph` with a fixed named-graph IRI, and a nested `EXISTS`
    /// (`p` becomes the nested inner, correlated on `?s` exactly like every other leaf
    /// — the shape [`crate::governor::soundness::expr_probe_admissible`]'s `Exists`
    /// arm gates). The four new targets/columns (`fresh1`/`fresh2`) are deliberately
    /// NOT `s`/`w` (the generator's own outer-row variables) — a collision there would
    /// hard-error under SEP-0007's rebinding rule ([`crate::governor::soundness::exists_row_collision`]),
    /// which is not what this generator is testing.
    const UNARY_COUNT: usize = 8;
    fn unary_wrap(p: &GraphPattern, idx: usize) -> GraphPattern {
        match idx {
            0 => GraphPattern::Filter {
                expr: Expression::Bound(var("s")),
                inner: bx(p.clone()),
            },
            1 => GraphPattern::Distinct {
                inner: bx(p.clone()),
            },
            2 => GraphPattern::Slice {
                inner: bx(p.clone()),
                start: 0,
                length: Some(5),
            },
            3 => GraphPattern::Project {
                inner: bx(p.clone()),
                variables: vec![var("s")],
            },
            4 => GraphPattern::Extend {
                inner: bx(p.clone()),
                variable: var("fresh1"),
                expression: Expression::Literal(Literal::new_typed(
                    "1",
                    nn("http://www.w3.org/2001/XMLSchema#integer"),
                )),
            },
            5 => GraphPattern::Join {
                left: bx(p.clone()),
                right: bx(GraphPattern::Values {
                    variables: vec![var("fresh2")],
                    bindings: vec![vec![Some(GroundTerm::NamedNode(nn(&format!("{EX}v1"))))]],
                }),
            },
            6 => GraphPattern::Graph {
                name: NamedNodePattern::NamedNode(nn(GRAPH_IRI)),
                inner: bx(p.clone()),
            },
            7 => GraphPattern::Filter {
                expr: Expression::Exists(bx(p.clone())),
                inner: bx(generator_leaves()[0].clone()),
            },
            _ => unreachable!("UNARY_COUNT out of sync with this match"),
        }
    }

    /// Combine `l`/`r` with the `idx`-th binary operator of the generator's alphabet —
    /// `Join`/`Union`/`LeftJoin`/`Minus`, per the task's stated grammar.
    const BINARY_COUNT: usize = 4;
    fn binary_wrap(l: &GraphPattern, r: &GraphPattern, idx: usize) -> GraphPattern {
        match idx {
            0 => GraphPattern::Join {
                left: bx(l.clone()),
                right: bx(r.clone()),
            },
            1 => GraphPattern::Union {
                left: bx(l.clone()),
                right: bx(r.clone()),
            },
            2 => GraphPattern::LeftJoin {
                left: bx(l.clone()),
                right: bx(r.clone()),
                expression: None,
            },
            3 => GraphPattern::Minus {
                left: bx(l.clone()),
                right: bx(r.clone()),
            },
            _ => unreachable!("BINARY_COUNT out of sync with this match"),
        }
    }

    /// Deterministic, seed-free enumeration of every `EXISTS` inner shape reachable
    /// from `leaves` within `max_depth` rounds of composition: each round applies
    /// every unary operator to every shape the PREVIOUS round produced, and every
    /// binary operator (both operand orders) to every (previous-round shape, leaf)
    /// pair — bounded to the previous round × the leaf set, rather than the previous
    /// round squared, so growth stays polynomial in `max_depth` instead of doubly
    /// exponential. A pure function of `leaves`/[`UNARY_COUNT`]/[`BINARY_COUNT`]/
    /// `max_depth` — no clock, no RNG, no iteration-order dependence on anything but
    /// these arguments — so the exact shape count (and the corpus itself) is stable
    /// across runs.
    fn generate_shapes(leaves: &[GraphPattern], max_depth: usize) -> Vec<GraphPattern> {
        let mut levels: Vec<Vec<GraphPattern>> = vec![leaves.to_vec()];
        for _ in 0..max_depth {
            let previous = levels.last().expect("levels always has at least one entry");
            let mut next = Vec::new();
            for p in previous {
                for idx in 0..UNARY_COUNT {
                    next.push(unary_wrap(p, idx));
                }
            }
            for p in previous {
                for leaf in leaves {
                    for idx in 0..BINARY_COUNT {
                        next.push(binary_wrap(p, leaf, idx));
                        next.push(binary_wrap(leaf, p, idx));
                    }
                }
            }
            levels.push(next);
        }
        levels.into_iter().flatten().collect()
    }

    /// [`exists_answers`] with a specific `exists_memo` toggle (rather than under a
    /// forced strategy) — the generator's memo-on-vs-off half of its per-shape check.
    fn exists_answers_with_memo(
        ds: &Arc<RdfDataset>,
        outer: &GraphPattern,
        inner: &GraphPattern,
        memo: bool,
    ) -> Vec<bool> {
        let mut ctx = EvalCtx::new(ds);
        ctx.options.exists_memo = memo;
        let seq = eval(outer, &mut ctx).expect("outer pattern evaluates");
        let exists_expr = Expression::Exists(Box::new(inner.clone()));
        seq.rows
            .iter()
            .map(|row| {
                crate::expr::eval_ebv(&exists_expr, row, &seq.schema, &mut ctx)
                    .expect("the generator's grammar never reaches a hard-erroring construct")
                    .expect("EXISTS always yields a defined boolean")
            })
            .collect()
    }

    /// The generator's shape-count floor: `crate::parallel_determinism_gate`'s CORPUS
    /// is small and hand-picked (dozens of cases); this generator's job is BREADTH, so
    /// the floor is set to the count [`generate_shapes`] actually reaches at
    /// [`GENERATOR_DEPTH`] with the alphabet above (3 leaves — including the
    /// stateful-builtin leaf — 8 unary ops, 4 binary ops), rounded down generously so
    /// a small future alphabet/depth tweak does not make this test flaky.
    const GENERATOR_DEPTH: usize = 2;
    const GENERATOR_FLOOR: usize = 2500;

    #[test]
    fn bounded_exhaustive_exists_shape_generator() {
        let ds = generator_ds();
        let outer = generator_outer();
        let leaves = generator_leaves();
        let shapes = generate_shapes(&leaves, GENERATOR_DEPTH);

        println!(
            "exists_admission_gate::bounded_exhaustive_exists_shape_generator: {} shapes \
             generated at depth {GENERATOR_DEPTH}",
            shapes.len()
        );
        assert!(
            shapes.len() >= GENERATOR_FLOOR,
            "the generator produced {} shapes, below the stated floor of {GENERATOR_FLOOR} — \
             either the alphabet/depth shrank or `generate_shapes` regressed",
            shapes.len()
        );

        let mut admissible_checked = 0_usize;
        let mut stateful_checked = 0_usize;
        for inner in &shapes {
            if is_classified_stateful(&outer, inner) {
                // A stateful inner (reachable `RAND()`, directly or through the
                // generator's nested-`EXISTS` operator) draws a FRESH value on every
                // evaluation, so the raw per-row booleans from two SEPARATE
                // evaluations (the memo-on run and the memo-off run below) may
                // legitimately differ even when BOTH take the per-row definition path
                // — comparing them for exact equality would make this sweep
                // nondeterministic. What must hold, and IS deterministic, is the
                // CLASSIFICATION: `expr_probe_admissible`'s `Exists` arm must never
                // admit a stateful inner. Still exercise both toggles once each
                // (unasserted against one another) so a stateful shape is proven not
                // to hard-error either way.
                assert!(
                    !is_classified_admissible(&outer, inner),
                    "a shape with a reachable stateful builtin was classified \
                     ADMISSIBLE for the memoized probe — has_stateful_builtin must \
                     refuse it: {inner:?}"
                );
                let _ = exists_answers_with_memo(&ds, &outer, inner, true);
                let _ = exists_answers_with_memo(&ds, &outer, inner, false);
                stateful_checked += 1;
                continue;
            }

            let memo_on = exists_answers_with_memo(&ds, &outer, inner, true);
            let memo_off = exists_answers_with_memo(&ds, &outer, inner, false);
            assert_eq!(
                memo_on, memo_off,
                "the exists_memo toggle changed the EXISTS answer for a generated shape: \
                 {inner:?}"
            );

            if is_classified_admissible(&outer, inner) {
                let natural = natural_answers(&ds, &outer, inner);
                let forced_definition =
                    forced_answers(&ds, &outer, inner, ForcedExistsStrategy::Definition);
                assert_eq!(
                    natural, forced_definition,
                    "the memoized probe and the forced per-row definition disagreed for a \
                     generated ADMISSIBLE shape: {inner:?}"
                );
                admissible_checked += 1;
            }
        }

        assert!(
            stateful_checked > 0,
            "no generated shape was ever classified stateful — the widened alphabet's \
             stateful-builtin leaf/nested-EXISTS operator never reached the corpus"
        );
        println!(
            "exists_admission_gate::bounded_exhaustive_exists_shape_generator: \
             {stateful_checked} of {} shapes were classified stateful (never \
             probe-admitted, memo-on/off equality skipped)",
            shapes.len()
        );
        assert!(
            admissible_checked > 0,
            "no generated shape was ever classified admissible — the probe-vs-definition \
             half of this test ran vacuously over the whole corpus"
        );
        println!(
            "exists_admission_gate::bounded_exhaustive_exists_shape_generator: {admissible_checked} \
             of {} shapes were classified admissible and probe-vs-definition-checked",
            shapes.len()
        );
    }

    // =========================================================================
    // Governed × strategy: the truncated-inner-never-memoized discipline pinned
    // against the CURRENT decision site, and a governed-certificate check for a
    // truncated DEFINITION-path (correlated, inadmissible) inner.
    // =========================================================================

    #[test]
    fn truncated_correlated_inner_withholds_per_certificate() {
        // Reuses `probe_would_diverge_on_minus`'s shape: `MINUS` is unconditionally
        // inadmissible (`admissible_rec`'s blanket `false`), so a correlated
        // `FILTER EXISTS` over it runs the per-row DEFINITION path NATURALLY — no force
        // seam needed — through the FULL governed engine
        // (`crate::engine::NativeSparqlEngine::query_governed`), which is what makes
        // this test distinct from `a_truncated_exists_inner_is_never_memoized` (that one
        // drives `eval_ebv` directly, over an UNCORRELATED/probe-admissible shape, and
        // checks the memo rather than the certificate).
        //
        // `crate::governor::soundness`'s module doc, "EXISTS is opaque, deliberately":
        // truncating an `EXISTS` inner bag can only turn a `FILTER EXISTS`'s boolean
        // from true to false (dropping a row from the middle, never a prefix), so the
        // `Filter → Exists` edge is `ChildEdge::OPAQUE` and collapses the certificate to
        // `SpineClass::Unknown` — `PartialAnswers::Unknown`, never `Certain`/`AtMost`,
        // whenever the trip lands there. A fuel sweep (rather than one tuned number)
        // proves this holds at whichever fuel level actually trips INSIDE the `Filter`'s
        // `EXISTS` child, since the exact charge schedule is not this test's concern.
        let mut b = RdfDatasetBuilder::new();
        let tag = b.intern_iri(&format!("{EX}tag"));
        let base = b.intern_iri(&format!("{EX}base"));
        let anything = b.intern_literal(RdfLiteral::simple("x"));
        let one = b.intern_literal(RdfLiteral::simple("1"));
        let z1 = b.intern_iri(&format!("{EX}z1"));
        let ca = b.intern_iri(&format!("{EX}cA"));
        b.push_quad(ca, tag, anything, None);
        b.push_quad(z1, base, one, None);
        let ds = b.freeze().expect("freeze");

        let query = format!(
            "PREFIX ex: <{EX}> \
             SELECT ?s WHERE {{ \
               ?s ex:tag ?w \
               FILTER EXISTS {{ ?z ex:base \"1\" MINUS {{ ?z ex:excluded ?s }} }} \
             }}"
        );
        let engine = crate::engine::NativeSparqlEngine::new();

        let mut saw_filter_barrier = false;
        for fuel in 1..=64_u64 {
            let governors = crate::QueryGovernors::UNBOUNDED.with_fuel(fuel);
            let outcome = engine
                .query_governed(
                    &ds,
                    purrdf_core::SparqlRequest {
                        query: &query,
                        base_iri: None,
                        substitutions: &[],
                    },
                    crate::engine::QueryOptions::EMPTY,
                    &governors,
                )
                .expect("the query parses and evaluates under every fuel level");

            if let crate::GovernedOutcome::BudgetExhausted(exhausted) = outcome
                && let crate::PartialAnswers::Unknown(barrier) = &exhausted.partial
                && barrier.operator() == "Filter"
            {
                saw_filter_barrier = true;
            }
        }
        assert!(
            saw_filter_barrier,
            "no fuel level in the sweep tripped inside the FILTER's EXISTS child, so the \
             test proves nothing about that certificate"
        );
    }
}
