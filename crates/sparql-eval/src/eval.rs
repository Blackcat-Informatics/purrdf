// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The graph-pattern evaluation recursion and its [`EvalCtx`].
//!
//! [`eval`] maps a [`GraphPattern`] to a [`SolutionSeq`] over the dataset in
//! [`EvalCtx`]. The recursion is filled in across the S6 build tasks (#912); each
//! not-yet-implemented variant hard-errors ([`EvalError::Unsupported`]) rather than
//! returning a partial bag (the `no-optionality` doctrine).
//!
//! Evaluation pins the **concrete** [`RdfDataset`] rather than a generic
//! `DatasetView`: the value→id bridge [`RdfDataset::term_id_by_value`] (P4 #838),
//! which BGP constant-resolution needs, is an inherent method on the frozen dataset
//! and is not part of the `DatasetView` trait. The dataset still exposes its
//! indexed read surface through `DatasetView` (the inherent `quads_for_pattern`
//! override, P4b #891).

use std::rc::Rc;
use std::sync::Arc;

use purrdf_core::{GraphMatch, RdfDataset, TermFactory, TermValue};
use purrdf_sparql_algebra::{GraphPattern, Query, Variable};

use crate::dataset_spec::ActiveDataset;
use crate::error::EvalError;
use crate::scratch::{ScratchInterner, SolutionTerm};
use crate::solution::SolutionSeq;
use crate::DetHashMap;

/// Tunable evaluation behavior. Every flag defaults to the production-optimal
/// value; the criterion benches flip individual flags to measure their effect
/// (the flags are a measurement seam, never a degraded production mode).
#[derive(Debug, Clone, Copy)]
pub struct EvalOptions {
    /// Memoize each `EXISTS`/`NOT EXISTS` inner-pattern evaluation. The inner
    /// pattern is evaluated unconstrained and then joined with the outer row's
    /// seed, so its result is **independent of the outer row**: a `FILTER` over N
    /// rows can evaluate it once instead of N times. Always `true` in production.
    pub exists_memo: bool,
}

impl Default for EvalOptions {
    fn default() -> Self {
        Self { exists_memo: true }
    }
}

/// The caller-supplied **standpoint predicate table** read by the `heldIn`
/// extension function and by loss-aware `CONSTRUCT`.
///
/// `heldIn(reifier, standpoint)` interprets *domain* predicates that live in the
/// caller's ontology and data — the annotation predicate binding a reifier to its
/// vantage standpoint (`according_to`) and the materialized poset edge
/// (`sharpens`). Those are NOT part of the engine: there is **no built-in
/// default**, and evaluating `heldIn` without a configured table is a hard
/// [`crate::EvalError`] (never a silently-wrong answer against fabricated IRIs).
///
/// Callers supply their own vocabulary, e.g. the gmeow ontology's
/// (`https://blackcatinformatics.ca/gmeow/accordingTo` / `…/sharpens`), via
/// [`crate::NativeSparqlEngine::with_standpoint_predicates`] (engine-level) or
/// [`EvalCtx::with_standpoint_predicates`] (a directly-built context).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandpointPredicates {
    /// The annotation predicate whose objects are a reifier's vantage
    /// standpoint(s) (e.g. `…/accordingTo`).
    pub according_to: String,
    /// The direct (already-materialized) "is more specific than" poset edge
    /// between standpoints (e.g. `…/sharpens`).
    pub sharpens: String,
}

impl StandpointPredicates {
    /// A table from the caller's two predicate IRIs.
    pub fn new(according_to: impl Into<String>, sharpens: impl Into<String>) -> Self {
        Self {
            according_to: according_to.into(),
            sharpens: sharpens.into(),
        }
    }
}

/// A hashable key for an `EXISTS` inner-cache entry: the inner pattern's address
/// (stable for the immutable AST during a query), a compact encoding of the active
/// graph, and a fingerprint of the **outer schema**. The schema fingerprint is part
/// of the key because the cached probe index ([`ExistsInner`]) — its `shared` column
/// pairing and the keyed/wild split derived from it — depends on the outer schema, not
/// just the inner pattern and graph. Keying on it makes a cached index correct *by
/// construction* even if the same `EXISTS` AST node is reached under two outer schemas.
pub(crate) type ExistsCacheKey = (usize, (u8, u32), u64);

/// A memoized `EXISTS`/`NOT EXISTS` inner pattern together with the probe index built
/// over it. The inner pattern is evaluated unconstrained **once** per [`ExistsCacheKey`];
/// the `(shared, keyed, wild)` index is built once and reused to existence-probe every
/// outer row (see [`crate::binop::probe_has_match`]). This is what turns a `FILTER (NOT)
/// EXISTS` anti-join from N per-row index rebuilds into N O(1)/scan probes.
pub(crate) struct ExistsInner {
    /// The inner pattern's unconstrained result (outer-row-independent).
    pub inner: Rc<SolutionSeq>,
    /// Shared columns between the outer schema and `inner.schema`, as
    /// `(outer_ordinal, inner_ordinal)` pairs (the probe's join key).
    pub shared: Vec<(usize, usize)>,
    /// Inner rows fully bound on the shared columns, grouped by their key.
    pub keyed: DetHashMap<crate::binop::JoinKey, Vec<usize>>,
    /// Inner rows with an unbound shared column (compatible with any probe value).
    pub wild: Vec<usize>,
}

/// A cheap FNV-1a fingerprint of an outer schema's variables (names in column order),
/// for [`ExistsCacheKey`]. Two schemas with the same ordered variable list hash equal,
/// so the cached probe index is only reused against a matching outer-row layout.
pub(crate) fn schema_fingerprint(schema: &crate::solution::VarSchema) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for v in schema.vars() {
        for b in v.as_str().as_bytes() {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        // Separator so ["ab","c"] and ["a","bc"] do not collide.
        h ^= 0xff;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// The shared, dataset-aware BGP join-order cache: maps `(dataset stats fingerprint,
/// BGP shape key)` to a cached evaluation order. It lives on the engine and is threaded
/// into evaluation by reference, so it persists across queries — the static query
/// corpus re-plans each BGP once per dataset. In-memory engine state only; never
/// materialised as triples (Principle 12). A stale or colliding key is at worst a
/// suboptimal order (the reorder is a permutation of a commutative join), never an
/// incorrect result, so the fingerprint can be cheap.
pub type BgpOrderCache = std::cell::RefCell<DetHashMap<(u64, u64), Arc<[usize]>>>;

/// The mutable evaluation context threaded through [`eval`].
pub struct EvalCtx<'d> {
    /// The frozen dataset being queried (the concrete IR — see the module docs for
    /// why this is not a generic `DatasetView`).
    pub dataset: &'d RdfDataset,
    /// The per-query interner for terms computed during evaluation (BIND, VALUES,
    /// aggregate output, arithmetic/string-function results).
    pub scratch: ScratchInterner,
    /// The graph currently in scope (set by `GRAPH`; the default graph at the root).
    /// At the root this is `GraphMatch::Default`, which `active_dataset` resolves to
    /// either the store default graph or a `FROM`/`USING`-merged default graph.
    pub active_graph: GraphMatch,
    /// The SPARQL active dataset (§13): how `active_graph == Default` is sourced and
    /// which named graphs `GRAPH` may address. Set from a query's `FROM` clause (the
    /// query path) or an UPDATE op's `USING` / `WITH` (the update path).
    pub(crate) active_dataset: ActiveDataset,
    /// A monotonic counter for minting fresh blank nodes (`BNODE()` and CONSTRUCT
    /// template blanks).
    pub bnode_counter: u64,
    /// The evaluation-time value of NOW() — an xsd:dateTime, captured once at
    /// context construction (from the host platform's real wall clock, see
    /// [`crate::clock::wall_clock_now`]) so all NOW() calls in a query return the
    /// same instant (SPARQL 1.1 §17.4.5.1).
    pub now: purrdf_xsd::XsdValue,
    /// Splitmix64 PRNG state for RAND()/UUID()/STRUUID(), seeded once at context
    /// construction from real OS/platform entropy (see [`crate::clock::entropy_seed`]).
    pub rng_state: u64,
    /// Tunable evaluation behavior (see [`EvalOptions`]). Production default.
    pub options: EvalOptions,
    /// The caller-supplied standpoint predicate table (see
    /// [`StandpointPredicates`]) read by `heldIn` and loss-aware
    /// `CONSTRUCT`. `None` (the default) means no table is configured:
    /// `heldIn` then hard-errors and `CONSTRUCT` cannot attribute a dropped
    /// annotation to a standpoint scope — deliberately, since these are domain
    /// predicates from the caller's ontology, never engine defaults.
    pub standpoint_predicates: Option<StandpointPredicates>,
    /// Memoized `EXISTS`/`NOT EXISTS` inner patterns **and their probe index**
    /// ([`ExistsInner`]), keyed by [`ExistsCacheKey`]. The inner eval and the index
    /// over it are outer-row-independent, so this turns `expr::exists`'s per-row
    /// re-evaluation *and* per-row index rebuild into a single build per site.
    /// Naturally per-query: a fresh [`EvalCtx`] is built for each `query()` call.
    pub(crate) exists_inner_cache: DetHashMap<ExistsCacheKey, Rc<ExistsInner>>,
    /// Per-query syntactic variable cache for expression positions inside an
    /// `EXISTS` inner pattern, keyed by the immutable inner-pattern AST address.
    /// Correlation detection runs for every outer row; caching this pure walk keeps
    /// the row loop focused on the cheap membership test against currently-bound
    /// outer variables.
    pub(crate) exists_expr_vars_cache: DetHashMap<usize, Rc<crate::DetHashSet<Variable>>>,
    /// Per-query cache for SPARQL `REGEX`/`REPLACE` pattern+flag compilations,
    /// keyed pattern-then-flags so a hit probes with **borrowed** strings (no
    /// per-row key allocation). The compiled regex is behind an `Rc`, so a hit
    /// hands out a cheap pointer clone that **shares** the regex's lazy-DFA cache
    /// pool instead of minting a fresh one per row. Dynamic pattern expressions
    /// still compile per distinct value, but a filter over many rows no longer
    /// rebuilds the same automata (or their DFA caches) for every row.
    pub(crate) regex_cache: DetHashMap<String, DetHashMap<String, Option<Rc<regex::Regex>>>>,
    /// Lazily-resolved solution terms for the `xsd:boolean` literals `"false"` /
    /// `"true"` (indexed by `usize::from(bool)`), so per-row boolean expression
    /// results skip the value-hash intern probe. Interning is deterministic per
    /// `(dataset, scratch)` — the dataset is pinned for the context's lifetime and
    /// the scratch interner dedups by value — so the cached term is bit-identical
    /// to what a fresh intern would return.
    pub(crate) cached_bool_terms: [Option<SolutionTerm>; 2],
    /// Per-query memo of interned constant expression atoms (`NamedNode` /
    /// `Literal`), keyed by the atom node's immutable AST address. A constant atom
    /// inside a `FILTER`/`BIND` is otherwise re-`to_owned()`'d into an owned
    /// `TermValue` and re-interned (a dataset reverse-index probe) once per row;
    /// this collapses that to a single intern per distinct atom node. Like
    /// [`Self::cached_bool_terms`], interning is deterministic for the pinned
    /// `(dataset, scratch)` pair, so a cached hit is the same `SolutionTerm` a
    /// fresh intern would produce. Naturally per-query — **but only for the
    /// static query algebra**: the address is a sound cache key precisely because
    /// those nodes are allocated once and outlive the whole `query()` call.
    /// Per-outer-row correlated-`EXISTS` substitution (`expr::exists`) is the
    /// exception: it heap-allocates a fresh substituted pattern tree per row and
    /// drops it at the end of that row, so a later row's differently-substituted
    /// node can be allocated at the SAME address (an ABA hazard) and would
    /// otherwise return a stale, wrong-row value from this cache.
    /// [`Self::in_substituted_exists`] flags exactly that window so `const_atom`
    /// bypasses this cache while it is set.
    pub(crate) const_atom_cache: DetHashMap<usize, SolutionTerm>,
    /// Per-query memo of the parsed XSD value of a dataset literal, keyed by its
    /// `TermId`. `FILTER`/comparison hot paths (`compare`/`equal`/`ebv_term`) parse
    /// the same `Existing(TermId)` literal's lexical form via `parse_by_iri` on
    /// every row; a 30k-row `?age > 40` re-parses ~60 distinct ages 30k times. The
    /// lexical form and datatype are immutable for a fixed id, so the parse is a
    /// pure function of the id — memoizing it (including the `None` "not an XSD
    /// value" outcome) collapses per-row re-parsing to one parse per distinct id.
    /// Naturally per-query. Only dataset (`Existing`) ids are cached; computed
    /// scratch values are ephemeral and stay on the borrowed-view path.
    pub(crate) xsd_parse_cache: DetHashMap<purrdf_core::TermId, Option<purrdf_xsd::XsdValue>>,
    /// The `SERVICE` federation source, if one is injected. `None` in
    /// the default engine path: a non-silent `SERVICE` then hard-fails. Tests and
    /// the conformance harness inject an in-memory source via [`EvalCtx::with_remote`].
    pub(crate) remote: Option<&'d dyn crate::remote::RemoteQuerySource>,
    /// The shared, dataset-aware BGP join-order cache, if one is injected. `None` for
    /// a directly-built context (e.g. a unit test): planning then runs every BGP, which
    /// is semantically identical — just not memoised. The engine injects its own cache
    /// via [`EvalCtx::with_order_cache`] so the static query corpus re-plans once per
    /// dataset. The order itself is computed, never materialised as triples
    /// (Principle 12).
    pub(crate) bgp_order_cache: Option<&'d BgpOrderCache>,
    /// Quads invented during evaluation by value-constructing builtins
    /// (`listSlice`/`listConcat` mint fresh `rdf:List` cells). A SPARQL
    /// expression returns one term, so the new cells are buffered here and surface at
    /// the result boundary — but only the cells **reachable from the surviving result
    /// rows** ([`Self::reachable_constructed`]): a list minted on a row later pruned by
    /// `FILTER`/`DISTINCT`/`LIMIT`/etc. must not leak orphaned cells.
    /// [`crate::construct::eval_construct`] folds the reachable set into the CONSTRUCT
    /// output, and the native `query` egress into `SparqlResult::Solutions::aux`. Empty
    /// whenever no constructing builtin ran.
    pub(crate) constructed: Vec<(TermValue, TermValue, TermValue)>,
    /// `true` while evaluating a per-outer-row correlated-`EXISTS` substituted
    /// temporary pattern (see `expr::exists`'s correlated branch). That
    /// temporary's `Expression`/`GraphPattern` nodes are heap-allocated fresh for
    /// the current outer row and dropped at the end of it — they do NOT outlive
    /// this context's `query()` call — so address-keyed memoization
    /// ([`Self::const_atom_cache`], [`Self::exists_expr_vars_cache`],
    /// [`Self::exists_inner_cache`]) is unsound over them (a later row's
    /// allocation can reuse a dropped node's address) and must be bypassed
    /// entirely while this flag is set.
    pub(crate) in_substituted_exists: bool,
}

impl core::fmt::Debug for EvalCtx<'_> {
    /// Summarized: the injected `SERVICE` source (`remote`) is a plain `dyn`
    /// trait object and the per-query caches are noise, so only the scalar
    /// evaluation state is shown.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EvalCtx")
            .field("active_graph", &self.active_graph)
            .field("bnode_counter", &self.bnode_counter)
            .field("now", &self.now)
            .field("rng_state", &self.rng_state)
            .field("options", &self.options)
            .field("standpoint_predicates", &self.standpoint_predicates)
            .finish_non_exhaustive()
    }
}

impl<'d> EvalCtx<'d> {
    /// A fresh context over `dataset`, scoped to the default graph.
    pub fn new(dataset: &'d RdfDataset) -> Self {
        let now_val = purrdf_xsd::XsdValue::DateTime(crate::clock::wall_clock_now());
        let rng_seed: u64 = crate::clock::entropy_seed();

        Self {
            dataset,
            scratch: ScratchInterner::new(),
            active_graph: GraphMatch::Default,
            active_dataset: ActiveDataset::store_default(),
            bnode_counter: 0,
            now: now_val,
            rng_state: rng_seed,
            options: EvalOptions::default(),
            standpoint_predicates: None,
            exists_inner_cache: DetHashMap::default(),
            exists_expr_vars_cache: DetHashMap::default(),
            regex_cache: DetHashMap::default(),
            cached_bool_terms: [None, None],
            const_atom_cache: DetHashMap::default(),
            xsd_parse_cache: DetHashMap::default(),
            remote: None,
            bgp_order_cache: None,
            constructed: Vec::new(),
            in_substituted_exists: false,
        }
    }

    /// Set the evaluation-time value of NOW(). Test-only: production callers get a
    /// correct wall-clock value for free from [`Self::new`].
    #[cfg(test)]
    #[must_use]
    pub(crate) fn with_now(mut self, now: purrdf_xsd::XsdValue) -> Self {
        self.now = now;
        self
    }

    /// Set the SplitMix64 seed used by RAND()/UUID()/STRUUID(). Test-only:
    /// production callers get a correct entropy seed for free from [`Self::new`].
    #[cfg(test)]
    #[must_use]
    pub(crate) fn with_rng_seed(mut self, seed: u64) -> Self {
        self.rng_state = seed;
        self
    }

    /// Supply the caller's standpoint predicate table (see
    /// [`StandpointPredicates`]) for `heldIn` and loss-aware `CONSTRUCT`.
    /// Without it, `heldIn` is a hard evaluation error.
    #[must_use]
    pub fn with_standpoint_predicates(mut self, predicates: StandpointPredicates) -> Self {
        self.standpoint_predicates = Some(predicates);
        self
    }

    /// Freeze the invented quads reachable from the surviving result `rows` (see
    /// [`Self::reachable_constructed`]) into a standalone dataset — the auxiliary graph
    /// surfaced alongside a SELECT/ASK result. The common empty-buffer case yields an
    /// empty (but valid) dataset.
    pub(crate) fn constructed_dataset(&self, rows: &[Vec<Option<TermValue>>]) -> Arc<RdfDataset> {
        let mut builder = purrdf_core::RdfDatasetBuilder::new();
        for (s, p, o) in self.reachable_constructed(rows) {
            let s = builder.intern_value(&s);
            let p = builder.intern_value(&p);
            let o = builder.intern_value(&o);
            builder.push_quad(s, p, o, None);
        }
        builder
            .freeze()
            .expect("constructed list cells are positionally valid by construction")
    }

    /// The constructed cells (see [`Self::constructed`]) reachable, via
    /// `rdf:first`/`rdf:rest`, from a term bound in a surviving result `row` — so a
    /// list minted on a row later removed by `FILTER`/`HAVING`/`DISTINCT`/`LIMIT` (or a
    /// failed join) contributes no orphaned cells to the egress.
    ///
    /// `TermValue` is not `Hash`, so the forest walk uses linear scans; the buffer
    /// holds only THIS query's freshly-minted cells, so it is small, and the common
    /// empty case is a fast no-op.
    pub(crate) fn reachable_constructed(
        &self,
        rows: &[Vec<Option<TermValue>>],
    ) -> Vec<(TermValue, TermValue, TermValue)> {
        if self.constructed.is_empty() {
            return Vec::new();
        }
        // Seed the walk with every term bound in a surviving row.
        let mut worklist: Vec<TermValue> = rows.iter().flatten().filter_map(Clone::clone).collect();
        let mut visited: Vec<TermValue> = Vec::new();
        let mut out: Vec<(TermValue, TermValue, TermValue)> = Vec::new();
        while let Some(node) = worklist.pop() {
            if visited.contains(&node) {
                continue;
            }
            visited.push(node.clone());
            for (s, p, o) in &self.constructed {
                if *s == node {
                    out.push((s.clone(), p.clone(), o.clone()));
                    // Follow the rest chain and any nested-list member head.
                    worklist.push(o.clone());
                }
            }
        }
        out
    }

    /// Attach a `SERVICE` federation source for this evaluation. The borrow shares
    /// the dataset lifetime `'d`; the engine's default path leaves it `None`.
    #[must_use]
    pub fn with_remote(mut self, source: &'d dyn crate::remote::RemoteQuerySource) -> Self {
        self.remote = Some(source);
        self
    }

    /// Attach the engine's shared BGP join-order cache for this evaluation. The borrow
    /// shares the dataset lifetime `'d`; a directly-built context leaves it `None` and
    /// re-plans each BGP (identical result, just not memoised).
    #[must_use]
    pub fn with_order_cache(mut self, cache: &'d BgpOrderCache) -> Self {
        self.bgp_order_cache = Some(cache);
        self
    }

    /// A compact hashable encoding of the active graph, for [`ExistsCacheKey`].
    pub(crate) fn graph_key(&self) -> (u8, u32) {
        match self.active_graph {
            GraphMatch::Any => (0, 0),
            GraphMatch::Default => (1, 0),
            GraphMatch::Named(id) => (2, id.index() as u32),
        }
    }
}

/// Evaluate a graph pattern to a multiset of solutions.
///
/// Implemented incrementally over the S6 build tasks; an unimplemented variant
/// returns [`EvalError::Unsupported`] naming the construct. Property paths are
/// evaluated in-engine (S8 #914, the `path` module); the remaining out-of-scope
/// nodes (`Service`, `Lateral`) stay permanent hard errors (SERVICE is S6b #928).
pub fn eval(pattern: &GraphPattern, ctx: &mut EvalCtx<'_>) -> Result<SolutionSeq, EvalError> {
    match pattern {
        GraphPattern::Bgp { patterns } => crate::bgp::eval_bgp(patterns, ctx),
        GraphPattern::Path {
            subject,
            path,
            object,
        } => crate::path::eval_path(subject, path, object, ctx),
        GraphPattern::Join { left, right } => crate::binop::eval_join(left, right, ctx),
        GraphPattern::Union { left, right } => crate::binop::eval_union(left, right, ctx),
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => crate::binop::eval_left_join(left, right, expression.as_ref(), ctx),
        GraphPattern::Minus { left, right } => crate::binop::eval_minus(left, right, ctx),
        GraphPattern::Filter { expr, inner } => crate::expr::eval_filter(expr, inner, ctx),
        GraphPattern::Extend {
            inner,
            variable,
            expression,
        } => crate::expr::eval_extend(inner, variable, expression, ctx),
        GraphPattern::Values {
            variables,
            bindings,
        } => crate::modifier::eval_values(variables, bindings, ctx),
        GraphPattern::Project { inner, variables } => {
            crate::modifier::eval_project(inner, variables, ctx)
        }
        GraphPattern::Distinct { inner } => crate::modifier::eval_distinct(inner, ctx),
        GraphPattern::Reduced { inner } => crate::modifier::eval_reduced(inner, ctx),
        GraphPattern::Slice {
            inner,
            start,
            length,
        } => crate::modifier::eval_slice(inner, *start, *length, ctx),
        GraphPattern::OrderBy { inner, expression } => {
            crate::modifier::eval_order_by(inner, expression, ctx)
        }
        GraphPattern::Graph { name, inner } => crate::modifier::eval_graph(name, inner, ctx),
        GraphPattern::Group {
            inner,
            variables,
            aggregates,
        } => crate::modifier::eval_group(inner, variables, aggregates, ctx),
        GraphPattern::Service {
            name,
            inner,
            silent,
        } => crate::remote::eval_service(name, inner, *silent, ctx),
        // Implemented incrementally over the remaining S6 build tasks; until then
        // (and permanently, for out-of-scope nodes) a hard error names the construct.
        other @ GraphPattern::Lateral { .. } => Err(EvalError::Unsupported(format!(
            "graph pattern `{}` is not yet implemented in sparql-eval",
            pattern_kind(other)
        ))),
    }
}

/// The result of evaluating a top-level query form — the internal counterpart of
/// the `SparqlResult` egress model (materialized by the engine, S6 Task 9).
#[derive(Debug)]
pub enum Outcome {
    /// `SELECT` solutions (a multiset over the projected schema).
    Solutions(SolutionSeq),
    /// `CONSTRUCT`/`DESCRIBE` graph result.
    Graph(Arc<RdfDataset>),
    /// `ASK` boolean.
    Boolean(bool),
}

/// Evaluate a top-level [`Query`] form over `ctx`'s dataset.
///
/// `SELECT`/`ASK` walk the modifier-wrapped pattern; `CONSTRUCT` and `DESCRIBE` emit
/// the IR dataset directly (`DESCRIBE` via the canonical Symmetric CBD).
pub fn evaluate_query(query: &Query, ctx: &mut EvalCtx<'_>) -> Result<Outcome, EvalError> {
    // Install the query's FROM / FROM NAMED active dataset (§13) before evaluating.
    ctx.active_dataset = ActiveDataset::from_query_dataset(query.dataset(), ctx.dataset);
    match query {
        Query::Select { pattern, .. } => Ok(Outcome::Solutions(eval(pattern, ctx)?)),
        Query::Ask { pattern, .. } => Ok(Outcome::Boolean(!eval(pattern, ctx)?.is_empty())),
        Query::Construct {
            template, pattern, ..
        } => Ok(Outcome::Graph(crate::construct::eval_construct(
            template, pattern, ctx,
        )?)),
        Query::Describe {
            pattern, targets, ..
        } => Ok(Outcome::Graph(crate::describe_query::eval_describe(
            pattern, targets, ctx,
        )?)),
    }
}

/// Materialize a [`SolutionSeq`] into dataset-independent egress form: the
/// projected variable names plus the owned [`TermValue`] rows (a `None` cell is
/// an unbound binding). The interned-`TermId` space ends here.
///
/// Shared by the engine's `SparqlResult` materializer and the SERVICE result
/// path, both of which turn an interned solution sequence into owned
/// term values via the per-query [`ScratchInterner`](crate::scratch::ScratchInterner).
pub(crate) fn materialize_solutions(
    seq: &SolutionSeq,
    ctx: &EvalCtx<'_>,
) -> (Vec<String>, Vec<Vec<Option<TermValue>>>) {
    let variables = seq
        .schema
        .vars()
        .iter()
        .map(|v| v.as_str().to_owned())
        .collect();
    // Literal datatype IRIs repeat massively across a result (a handful of XSD
    // types over tens of thousands of cells), so each datatype TermId is resolved
    // once per call and cloned from a small memo instead of re-resolved per cell.
    let mut datatype_memo: DetHashMap<purrdf_core::TermId, String> = DetHashMap::default();
    let mut rows = Vec::with_capacity(seq.rows.len());
    for row in &seq.rows {
        let mut out = Vec::with_capacity(row.len());
        for cell in row {
            out.push(cell.map(|t| memoized_value_of(ctx, t, &mut datatype_memo)));
        }
        rows.push(out);
    }
    (variables, rows)
}

/// [`ScratchInterner::value_of`], with repeated literal datatype-IRI resolutions
/// served from `datatype_memo` (egress-only; identical output values).
fn memoized_value_of(
    ctx: &EvalCtx<'_>,
    term: SolutionTerm,
    datatype_memo: &mut DetHashMap<purrdf_core::TermId, String>,
) -> TermValue {
    match term {
        SolutionTerm::Existing(id) => memoized_term_value(ctx.dataset, id, datatype_memo),
        SolutionTerm::Computed(_) => ctx.scratch.value_of(ctx.dataset, term),
    }
}

/// `scratch::term_id_to_value`, with the literal datatype id → IRI string
/// resolution memoized across cells (recursing through RDF-1.2 triple terms).
fn memoized_term_value(
    dataset: &RdfDataset,
    id: purrdf_core::TermId,
    datatype_memo: &mut DetHashMap<purrdf_core::TermId, String>,
) -> TermValue {
    match dataset.resolve(id) {
        purrdf_core::TermRef::Iri(iri) => TermValue::Iri(iri.to_owned()),
        purrdf_core::TermRef::Blank { label, scope } => TermValue::Blank {
            label: label.to_owned(),
            scope,
        },
        purrdf_core::TermRef::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => {
            let datatype = datatype_memo
                .entry(datatype)
                .or_insert_with(|| match dataset.resolve(datatype) {
                    purrdf_core::TermRef::Iri(iri) => iri.to_owned(),
                    // A literal's datatype is always an interned IRI (C0.1).
                    other => unreachable!("literal datatype must be an IRI, got {other:?}"),
                })
                .clone();
            TermValue::Literal {
                lexical_form: lexical.to_owned(),
                datatype,
                language: language.map(str::to_owned),
                direction,
            }
        }
        purrdf_core::TermRef::Triple { s, p, o } => TermValue::Triple {
            s: Box::new(memoized_term_value(dataset, s, datatype_memo)),
            p: Box::new(memoized_term_value(dataset, p, datatype_memo)),
            o: Box::new(memoized_term_value(dataset, o, datatype_memo)),
        },
    }
}

/// A short, stable name for a [`GraphPattern`] variant, for diagnostics.
pub(crate) fn pattern_kind(pattern: &GraphPattern) -> &'static str {
    match pattern {
        GraphPattern::Bgp { .. } => "BGP",
        GraphPattern::Path { .. } => "property path",
        GraphPattern::Join { .. } => "Join",
        GraphPattern::LeftJoin { .. } => "OPTIONAL (LeftJoin)",
        GraphPattern::Lateral { .. } => "LATERAL",
        GraphPattern::Filter { .. } => "FILTER",
        GraphPattern::Union { .. } => "UNION",
        GraphPattern::Graph { .. } => "GRAPH",
        GraphPattern::Extend { .. } => "BIND (Extend)",
        GraphPattern::Minus { .. } => "MINUS",
        GraphPattern::Service { .. } => "SERVICE",
        GraphPattern::Values { .. } => "VALUES",
        GraphPattern::OrderBy { .. } => "ORDER BY",
        GraphPattern::Project { .. } => "Project",
        GraphPattern::Distinct { .. } => "DISTINCT",
        GraphPattern::Reduced { .. } => "REDUCED",
        GraphPattern::Slice { .. } => "LIMIT/OFFSET (Slice)",
        GraphPattern::Group { .. } => "GROUP BY",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_core::RdfDatasetBuilder;

    #[test]
    fn empty_bgp_is_the_unit_sequence() {
        let ds = RdfDatasetBuilder::new().freeze().expect("freeze empty");
        let mut ctx = EvalCtx::new(&ds);
        let seq = eval(&GraphPattern::Bgp { patterns: vec![] }, &mut ctx).expect("empty BGP");
        // The identity table Z: exactly one solution that binds nothing.
        assert_eq!(seq.len(), 1);
        assert!(seq.schema.is_empty());
    }

    #[test]
    fn unimplemented_variant_hard_errors_with_its_name() {
        let ds = RdfDatasetBuilder::new().freeze().expect("freeze empty");
        let mut ctx = EvalCtx::new(&ds);
        // LATERAL remains permanently out of scope (SERVICE is now evaluated via
        // the remote seam); a still-unsupported node names itself.
        let pattern = GraphPattern::Lateral {
            left: Box::new(GraphPattern::Bgp { patterns: vec![] }),
            right: Box::new(GraphPattern::Bgp { patterns: vec![] }),
        };
        let err = eval(&pattern, &mut ctx).unwrap_err();
        assert!(matches!(err, EvalError::Unsupported(_)));
        assert!(err.to_string().contains("LATERAL"));
    }

    #[test]
    fn filter_exists_builds_inner_index_once_across_outer_rows() {
        use purrdf_sparql_algebra::{
            Expression, NamedNode, NamedNodePattern, TermPattern, TriplePattern, Variable,
        };

        // Three typed subjects; two carry a :stereo, one does not — the class-without-stereotype
        // anti-join shape: the outer var `?class` appears in the inner ONLY in a BGP
        // triple position (no expression correlation), so the uncorrelated fast path
        // is taken and the inner index must be reused across the three outer rows.
        let mut b = RdfDatasetBuilder::new();
        let ty = b.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
        let cls = b.intern_iri("http://ex/Class");
        let stereo = b.intern_iri("http://ex/stereo");
        let a = b.intern_iri("http://ex/a");
        let bb = b.intern_iri("http://ex/b");
        let c = b.intern_iri("http://ex/c");
        let s = b.intern_iri("http://ex/S");
        b.push_quad(a, ty, cls, None);
        b.push_quad(bb, ty, cls, None);
        b.push_quad(c, ty, cls, None);
        b.push_quad(a, stereo, s, None);
        b.push_quad(bb, stereo, s, None);
        let ds = b.freeze().expect("freeze");

        let vp = |n: &str| TermPattern::Variable(Variable::new(n));
        let pred = |iri: &str| NamedNodePattern::NamedNode(NamedNode::new_unchecked(iri));
        let bgp = |s, p, o| GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: s,
                predicate: p,
                object: o,
            }],
        };

        // outer: ?class a ?ctype (3 rows). inner: ?class :stereo ?st.
        let outer = bgp(
            vp("class"),
            pred("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"),
            vp("ctype"),
        );
        let inner = bgp(vp("class"), pred("http://ex/stereo"), vp("st"));
        let filter = GraphPattern::Filter {
            expr: Expression::Exists(Box::new(inner)),
            inner: Box::new(outer),
        };

        let mut ctx = EvalCtx::new(&ds);
        let seq = eval(&filter, &mut ctx).expect("filter exists");
        // EXISTS keeps the two subjects with a :stereo (a, b); drops c.
        assert_eq!(seq.len(), 2);
        // The inner pattern AND its probe index were built exactly once despite three
        // outer rows — the per-row index rebuild is gone.
        assert_eq!(ctx.exists_inner_cache.len(), 1);
    }

    #[test]
    fn schema_fingerprint_distinguishes_variable_lists() {
        use purrdf_sparql_algebra::Variable;
        let s = |names: &[&str]| {
            crate::solution::VarSchema::from_vars(names.iter().map(|n| Variable::new(*n)))
        };
        // Order matters, separator prevents boundary collisions, equal lists match.
        assert_ne!(
            schema_fingerprint(&s(&["a", "b"])),
            schema_fingerprint(&s(&["b", "a"]))
        );
        assert_ne!(
            schema_fingerprint(&s(&["ab", "c"])),
            schema_fingerprint(&s(&["a", "bc"]))
        );
        assert_eq!(
            schema_fingerprint(&s(&["x", "y"])),
            schema_fingerprint(&s(&["x", "y"]))
        );
    }
}
