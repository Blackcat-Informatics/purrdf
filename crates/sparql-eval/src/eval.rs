// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The graph-pattern evaluation recursion and its [`EvalCtx`].
//!
//! [`eval`] maps a [`GraphPattern`] to a [`SolutionSeq`] over the dataset in
//! [`EvalCtx`]. The recursion is filled in across the S6 build tasks; each
//! not-yet-implemented variant hard-errors ([`EvalError::Unsupported`]) rather than
//! returning a partial bag (the `no-optionality` doctrine).
//!
//! Evaluation pins the **concrete** [`RdfDataset`] rather than a generic
//! `DatasetView`: the value→id bridge [`RdfDataset::term_id_by_value`] (P4),
//! which BGP constant-resolution needs, is an inherent method on the frozen dataset
//! and is not part of the `DatasetView` trait. The dataset still exposes its
//! indexed read surface through `DatasetView` (the inherent `quads_for_pattern`
//! override, P4b).

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
    pub inner: Arc<SolutionSeq>,
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
pub type BgpOrderCache = std::sync::RwLock<DetHashMap<(u64, u64), Arc<[usize]>>>;

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
    /// The row ordinal of the solution currently being extended, set by
    /// [`crate::expr::eval_extend`] right before it evaluates that row's
    /// expression. `BNODE(strExpr)` (SPARQL 1.1 §17.4.2.2) uses this to
    /// memoize per-solution: the row/argument pair identifies "the same query
    /// solution" across the chain of `Extend` nodes one `SELECT`'s
    /// `(expr AS ?v)` list (or a `WHERE`-clause `BIND`) lowers to, since each
    /// `Extend` maps the SAME ordered row sequence 1:1 with no row dropped,
    /// added, or reordered between them.
    pub(crate) current_row: u64,
    /// Per-solution memo for `BNODE(strExpr)`, keyed by `(current_row, argument
    /// string)`: two calls with an equal argument at the same row ordinal reuse
    /// the same minted blank (SPARQL 1.1 §17.4.2.2); the zero-argument `BNODE()`
    /// form bypasses this entirely and always mints fresh. Query-scoped like the
    /// other caches on this context — never cleared mid-query, since a later row
    /// never revisits an earlier row's ordinal within the same `Extend` chain.
    pub(crate) bnode_memo: DetHashMap<(u64, String), SolutionTerm>,
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
    pub(crate) exists_inner_cache: DetHashMap<ExistsCacheKey, Arc<ExistsInner>>,
    /// Per-query syntactic variable cache for expression positions inside an
    /// `EXISTS` inner pattern, keyed by the immutable inner-pattern AST address.
    /// Correlation detection runs for every outer row; caching this pure walk keeps
    /// the row loop focused on the cheap membership test against currently-bound
    /// outer variables.
    pub(crate) exists_expr_vars_cache: DetHashMap<usize, Arc<crate::DetHashSet<Variable>>>,
    /// Per-query cache for SPARQL `REGEX`/`REPLACE` pattern+flag compilations,
    /// keyed pattern-then-flags so a hit probes with **borrowed** strings (no
    /// per-row key allocation). The compiled regex is behind an `Arc`, so a hit
    /// hands out a cheap pointer clone that **shares** the regex's lazy-DFA cache
    /// pool instead of minting a fresh one per row. Dynamic pattern expressions
    /// still compile per distinct value, but a filter over many rows no longer
    /// rebuilds the same automata (or their DFA caches) for every row.
    pub(crate) regex_cache: DetHashMap<String, DetHashMap<String, Option<Arc<regex::Regex>>>>,
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
    pub(crate) remote: Option<&'d (dyn crate::remote::RemoteQuerySource + Sync)>,
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
    /// The query's effective base IRI (see [`purrdf_sparql_algebra::Query::base_iri`]),
    /// set once per `evaluate_query` call. `IRI()`/`URI()` resolves a relative-reference
    /// string argument against this (SPARQL 1.1 §17.4.2.6); `None` means no base was
    /// ever supplied (an explicit `BASE` decl nor a caller document base), so a
    /// relative argument cannot be resolved and the call is a type error.
    pub(crate) base_iri: Option<String>,
    /// The caller-injected SHACL-AF function table (`sh:SPARQLFunction`), if any.
    /// `None` (the default) means no user functions are declared: a call-position
    /// IRI unknown to the closed `PurrdfFn` set then falls through to the XSD-cast /
    /// unsupported path exactly as before. Borrowed for the dataset lifetime (like
    /// [`Self::remote`]/[`Self::bgp_order_cache`]), so carrying it is a `Copy`
    /// pointer, never a clone.
    pub(crate) user_functions: Option<&'d crate::user_fn::UserFunctionRegistry>,
    /// The current SHACL-AF function call depth, incremented by
    /// [`Self::child_for_user_fn`] and bounded by [`MAX_UDF_DEPTH`] so
    /// mutually-recursive functions fail closed rather than overflow the stack.
    pub(crate) udf_depth: u32,
}

/// The maximum SHACL-AF function call depth. A function body that calls another
/// function (directly or in a cycle) is bounded here and fails closed on overflow —
/// the evaluator's counterpart of the shapes engine's `MAX_RECURSION_DEPTH`. The two
/// counters are independent: this bounds function→function chains inside SPARQL
/// evaluation, while the shapes guard bounds shape re-entry.
pub(crate) const MAX_UDF_DEPTH: u32 = 32;

/// Compile-time proof that [`EvalCtx`] is `Send + Sync`, so a future parallel
/// worker can hold `&EvalCtx`/build its own from a shared `&'d RdfDataset`
/// across threads. Every field must stay `Send + Sync` for this to hold — the
/// `Rc`/`RefCell` fields that used to block it were switched to `Arc`/`RwLock`
/// and `remote`'s trait object was given an explicit `+ Sync` bound precisely
/// so this assertion compiles.
const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<EvalCtx<'static>>();
};

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
            current_row: 0,
            bnode_memo: DetHashMap::default(),
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
            base_iri: None,
            user_functions: None,
            udf_depth: 0,
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
    pub fn with_remote(
        mut self,
        source: &'d (dyn crate::remote::RemoteQuerySource + Sync),
    ) -> Self {
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

    /// Fork a `Send` child context for a parallel worker (Task 4-6's fork-join
    /// evaluation), sharing this context's immutable/read-only state and starting
    /// its mutable evaluation state fresh.
    ///
    /// The split is what makes fork-join deterministic under [`crate::parallel`]:
    ///
    /// - **Shared** (`dataset`/`remote`/`bgp_order_cache`/`options`, and the cheap
    ///   `Clone`s `active_graph`/`active_dataset`/`now`/`standpoint_predicates`):
    ///   read-only for the duration of evaluation, so sharing them across workers
    ///   cannot introduce a data race or a cross-worker ordering dependency.
    /// - **Cloned** (`exists_inner_cache`/`exists_expr_vars_cache`): cheap
    ///   `Arc`-valued maps, so a memo the parent already warmed (e.g. from
    ///   evaluating an earlier sibling sequentially) is inherited by the child
    ///   instead of rebuilt — a performance inheritance, not a correctness
    ///   requirement, since a cache miss just re-derives the same value.
    /// - **Cloned base, fresh appends** (`scratch`): input rows carry
    ///   [`crate::scratch::SolutionTerm::Computed`] ids that index into THIS
    ///   context's scratch value table, so a child given a fresh, empty scratch
    ///   could not resolve them (wrong value, or an out-of-bounds panic). The
    ///   whole [`crate::scratch::ScratchInterner`] (value table AND its
    ///   value→id dedup index) is cloned instead: the child resolves every
    ///   existing `Computed` id identically to the parent, and any NEW value it
    ///   mints is deduped against its own clone of the index exactly as the
    ///   parent would dedup it. A child's fresh mints are ephemeral — discarded
    ///   for a read-only FILTER predicate (the surviving rows are the original
    ///   rows, nothing new escapes) or, for a minting worker, captured by
    ///   [`crate::parallel::portable_row`] as a `Vec` of
    ///   [`crate::parallel::PortableTerm`] while the child's scratch is still
    ///   alive and re-interned against the parent by
    ///   [`crate::parallel::reintern_minted_row`] so each freshly-minted cell's
    ///   id is valid in the parent's space (a raw child `ScratchId` is never
    ///   reused in the parent — only the id space, not individual ids, is
    ///   shared by the clone).
    /// - **Fresh** (`regex_cache`, `cached_bool_terms`, `const_atom_cache`,
    ///   `xsd_parse_cache`, `constructed`, `in_substituted_exists`): per-worker
    ///   mutable state that must NOT be shared, so each worker mints its own
    ///   constructed-quad buffer without contending on a lock. The caller
    ///   classifies each worker row with [`crate::parallel::minted_row`] into a
    ///   [`crate::parallel::MintedRow`] (`Direct` — no post-fork mint, passed
    ///   through untouched — or `Portable`) and folds it back into the parent
    ///   via [`crate::parallel::reintern_minted_row`], invoked once per row in
    ///   source-index order across all workers, so the result is bit-identical
    ///   to sequential evaluation. A read-only FILTER-predicate worker never
    ///   reaches this path: only the boolean result and the original `Copy` row
    ///   escape, so its child scratch is discarded whole.
    /// - **Copied scalars** (`bnode_counter`, `rng_state`): their only stateful
    ///   builtins (`BNODE`, `RAND`/`UUID`/`STRUUID`, and the PurRDF list
    ///   constructors) are excluded from parallel evaluation by
    ///   [`crate::parallel::is_parallel_safe`], so the copied value is never
    ///   actually observed divergently across workers — copying it here is
    ///   harmless rather than load-bearing.
    ///
    /// Called by `expr::eval_filter` and `binop::left_outer_join_filtered` (Task 5)
    /// to give each FILTER-predicate worker its own child context.
    #[must_use]
    pub(crate) fn fork_for_worker(&self) -> Self {
        Self {
            dataset: self.dataset,
            scratch: self.scratch.clone(),
            active_graph: self.active_graph,
            active_dataset: self.active_dataset.clone(),
            bnode_counter: self.bnode_counter,
            // Per-row `BNODE(strExpr)` memo state. Like `bnode_counter`, only ever
            // observed by `Function::BNode`, which `is_parallel_safe` classifies
            // UNSAFE — so a worker never evaluates it and this state is never read
            // divergently. Each worker gets a fresh empty memo / copied scalar; both
            // are harmless rather than load-bearing (mirrors the `bnode_counter`
            // note above).
            current_row: self.current_row,
            bnode_memo: DetHashMap::default(),
            now: self.now.clone(),
            rng_state: self.rng_state,
            options: self.options,
            standpoint_predicates: self.standpoint_predicates.clone(),
            exists_inner_cache: self.exists_inner_cache.clone(),
            exists_expr_vars_cache: self.exists_expr_vars_cache.clone(),
            regex_cache: DetHashMap::default(),
            cached_bool_terms: [None, None],
            const_atom_cache: DetHashMap::default(),
            xsd_parse_cache: DetHashMap::default(),
            remote: self.remote,
            bgp_order_cache: self.bgp_order_cache,
            constructed: Vec::new(),
            in_substituted_exists: false,
            // The query's effective base IRI is a read-only per-query constant.
            // `IRI()`/`URI()` (parallel-safe, so reachable in a parallel `Extend`)
            // resolve relative references against it, so every worker must see it.
            base_iri: self.base_iri.clone(),
            // Read-only shared registry (a `Copy` pointer) and the current call
            // depth: a worker that evaluates a `Function::Custom` user-function call
            // must see the same table and depth bound as its parent.
            user_functions: self.user_functions,
            udf_depth: self.udf_depth,
        }
    }

    /// Attach a caller-injected SHACL-AF function registry (`sh:SPARQLFunction`) for
    /// this evaluation. The borrow shares the dataset lifetime `'d`; a context
    /// without one leaves it `None` and a call-position IRI unknown to the closed
    /// `PurrdfFn` set is an XSD cast or an unsupported-function error.
    #[must_use]
    pub fn with_user_functions(
        mut self,
        registry: &'d crate::user_fn::UserFunctionRegistry,
    ) -> Self {
        self.user_functions = Some(registry);
        self
    }

    /// Build a child context for evaluating a SHACL-AF function body: it shares the
    /// dataset, clock/entropy, order cache, standpoint table, remote source and
    /// function registry, but starts fresh mutable evaluation state (the body is an
    /// independent query) and increments the call depth.
    ///
    /// # Errors
    ///
    /// [`EvalError::Function`] if the call depth would exceed [`MAX_UDF_DEPTH`] —
    /// mutually-recursive functions fail closed rather than overflow the stack.
    pub(crate) fn child_for_user_fn(&self) -> Result<Self, EvalError> {
        let next_depth = self.udf_depth + 1;
        if next_depth > MAX_UDF_DEPTH {
            return Err(EvalError::function(format!(
                "SHACL-AF function recursion exceeded the depth bound of {MAX_UDF_DEPTH}"
            )));
        }
        Ok(Self {
            dataset: self.dataset,
            // Fresh: the body is an independent query that mints its own computed
            // terms; its parameter inputs ride in as ground substitutions, not
            // scratch ids, so no parent scratch state is needed.
            scratch: ScratchInterner::new(),
            // The body evaluates as a root query; `evaluate_query` re-installs the
            // body's own FROM/base, so seed the default graph here.
            active_graph: GraphMatch::Default,
            active_dataset: ActiveDataset::store_default(),
            // Inherit the parent counter so body-minted blanks continue the
            // parent's sequence; the advanced value is merged back after the call.
            bnode_counter: self.bnode_counter,
            current_row: 0,
            bnode_memo: DetHashMap::default(),
            now: self.now.clone(),
            rng_state: self.rng_state,
            options: self.options,
            standpoint_predicates: self.standpoint_predicates.clone(),
            exists_inner_cache: DetHashMap::default(),
            exists_expr_vars_cache: DetHashMap::default(),
            regex_cache: DetHashMap::default(),
            cached_bool_terms: [None, None],
            const_atom_cache: DetHashMap::default(),
            xsd_parse_cache: DetHashMap::default(),
            remote: self.remote,
            bgp_order_cache: self.bgp_order_cache,
            constructed: Vec::new(),
            in_substituted_exists: false,
            base_iri: None,
            user_functions: self.user_functions,
            udf_depth: next_depth,
        })
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
/// evaluated in-engine (S8, the `path` module); the remaining out-of-scope
/// nodes (`Service`, `Lateral`) stay permanent hard errors (SERVICE is S6b).
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
        GraphPattern::Lateral { left, right } => crate::binop::eval_lateral(left, right, ctx),
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
    // Install the query's effective base IRI so IRI()/URI() can resolve a relative
    // string argument against it (SPARQL 1.1 §17.4.2.6).
    ctx.base_iri = query.base_iri().map(|nn| nn.as_str().to_owned());
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
    fn lateral_of_units_is_the_unit_sequence() {
        let ds = RdfDatasetBuilder::new().freeze().expect("freeze empty");
        let mut ctx = EvalCtx::new(&ds);
        // LATERAL(Z, Z): the left unit table drives one substituted evaluation of
        // the right unit table, merging to a single binding-nothing solution.
        let pattern = GraphPattern::Lateral {
            left: Box::new(GraphPattern::Bgp { patterns: vec![] }),
            right: Box::new(GraphPattern::Bgp { patterns: vec![] }),
        };
        let seq = eval(&pattern, &mut ctx).expect("LATERAL of units");
        assert_eq!(seq.len(), 1);
        assert!(seq.schema.is_empty());
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
        let outer_for_low_level_check = outer.clone();
        let inner_for_low_level_check = inner.clone();
        let filter = GraphPattern::Filter {
            expr: Expression::Exists(Box::new(inner)),
            inner: Box::new(outer),
        };

        let mut ctx = EvalCtx::new(&ds);
        let seq = eval(&filter, &mut ctx).expect("filter exists");
        // EXISTS keeps the two subjects with a :stereo (a, b); drops c.
        assert_eq!(seq.len(), 2);

        // The `ctx.exists_inner_cache.len()` check this test used to make against
        // `ctx` directly no longer applies: this EXISTS reaches no unsafe builtin,
        // so (Task 5) `expr::eval_filter` routes it through
        // `crate::parallel::par_chunk_try_map_init`, which runs the per-row loop on
        // a FORKED child context (`EvalCtx::fork_for_worker`), not `ctx` itself —
        // even below the parallel threshold, exactly one child is forked and reused
        // across every outer row. So the cache still builds exactly once per query,
        // just on that (discarded-after-use) child rather than on `ctx`. Reproduce
        // the same shape here directly (drive `eval_ebv` for each outer row over one
        // shared ctx, exactly as the child's per-row loop does) to keep exercising
        // the "no per-row index rebuild" invariant.
        let mut child_ctx = EvalCtx::new(&ds);
        let outer_seq = eval(&outer_for_low_level_check, &mut child_ctx).expect("outer bgp");
        let exists_expr = Expression::Exists(Box::new(inner_for_low_level_check));
        let mut kept = 0;
        for row in &outer_seq.rows {
            if crate::expr::eval_ebv(&exists_expr, row, &outer_seq.schema, &mut child_ctx)
                .expect("ebv")
                == Some(true)
            {
                kept += 1;
            }
        }
        assert_eq!(kept, 2);
        assert_eq!(
            child_ctx.exists_inner_cache.len(),
            1,
            "the inner pattern AND its probe index were built exactly once despite \
             three outer rows — the per-row index rebuild is gone"
        );
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

    /// Determinism smoke test (Task 4): a query exercising BGP, JOIN, a
    /// non-filtered OPTIONAL, and MINUS evaluated once with the parallel path
    /// FORCED (via [`crate::parallel::force_parallel_for_test`]) and once with
    /// the sequential path FORCED must produce byte-identical `Vec<Solution>`
    /// rows (schema and row order both). This is a narrower, faster-running
    /// tripwire than the full Task 7 gate — it catches an ordering regression
    /// in any of the four read-only nodes wired in this task immediately,
    /// something the conformance suite's multiset comparisons would not.
    #[test]
    fn parallel_and_sequential_paths_agree_bit_for_bit() {
        use purrdf_sparql_algebra::{
            NamedNode, NamedNodePattern, TermPattern, TriplePattern, Variable,
        };

        // :a :knows :b . :b :knows :c .
        // :a :likes :cake . :b :likes :tea . :c :likes :juice .
        // :tea :extra :hot .
        // :a :bad :x .
        let mut b = RdfDatasetBuilder::new();
        let knows = b.intern_iri("http://ex/knows");
        let likes = b.intern_iri("http://ex/likes");
        let extra = b.intern_iri("http://ex/extra");
        let bad = b.intern_iri("http://ex/bad");
        let a = b.intern_iri("http://ex/a");
        let bb = b.intern_iri("http://ex/b");
        let c = b.intern_iri("http://ex/c");
        let cake = b.intern_iri("http://ex/cake");
        let tea = b.intern_iri("http://ex/tea");
        let juice = b.intern_iri("http://ex/juice");
        let hot = b.intern_iri("http://ex/hot");
        let x = b.intern_iri("http://ex/x");
        b.push_quad(a, knows, bb, None);
        b.push_quad(bb, knows, c, None);
        b.push_quad(a, likes, cake, None);
        b.push_quad(bb, likes, tea, None);
        b.push_quad(c, likes, juice, None);
        b.push_quad(tea, extra, hot, None);
        b.push_quad(a, bad, x, None);
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

        // { ?x :knows ?y } JOIN { ?y :likes ?z } OPTIONAL { ?z :extra ?w } MINUS { ?x :bad ?v }
        let knows_bgp = bgp(vp("x"), pred("http://ex/knows"), vp("y"));
        let likes_bgp = bgp(vp("y"), pred("http://ex/likes"), vp("z"));
        let join = GraphPattern::Join {
            left: Box::new(knows_bgp),
            right: Box::new(likes_bgp),
        };
        let extra_bgp = bgp(vp("z"), pred("http://ex/extra"), vp("w"));
        let optional = GraphPattern::LeftJoin {
            left: Box::new(join),
            right: Box::new(extra_bgp),
            expression: None,
        };
        let bad_bgp = bgp(vp("x"), pred("http://ex/bad"), vp("v"));
        let pattern = GraphPattern::Minus {
            left: Box::new(optional),
            right: Box::new(bad_bgp),
        };

        let run = |forced: bool| {
            let _guard = crate::parallel::force_parallel_for_test(forced);
            let mut ctx = EvalCtx::new(&ds);
            let seq = eval(&pattern, &mut ctx).expect("eval");
            (seq.schema.vars().to_vec(), seq.rows)
        };

        let (schema_par, rows_par) = run(true);
        let (schema_seq, rows_seq) = run(false);

        assert_eq!(
            schema_par, schema_seq,
            "schema must match regardless of path"
        );
        assert_eq!(
            rows_par, rows_seq,
            "parallel and sequential paths must produce byte-identical row order"
        );
        // Sanity: the MINUS removes the x=a row (it has a :bad edge); only the
        // x=b/y=c/z=juice row (with ?w unbound, no :extra match) survives.
        assert_eq!(rows_seq.len(), 1);
    }

    /// Determinism smoke test (Task 5): `FILTER(REGEX(...) && ?a > k)` — the
    /// `b_scan_filter` bench shape — evaluated once with the parallel path FORCED
    /// and once with the sequential path FORCED must produce byte-identical rows.
    #[test]
    fn filter_regex_and_numeric_forced_parallel_and_sequential_agree() {
        use purrdf_core::RdfLiteral;
        use purrdf_sparql_algebra::{
            Expression, Function, Literal, NamedNode, NamedNodePattern, TermPattern, TriplePattern,
            Variable,
        };

        const XINT: &str = "http://www.w3.org/2001/XMLSchema#integer";

        let mut b = RdfDatasetBuilder::new();
        let name = b.intern_iri("http://ex/name");
        let age = b.intern_iri("http://ex/age");
        let p1 = b.intern_iri("http://ex/p1");
        let p2 = b.intern_iri("http://ex/p2");
        let p3 = b.intern_iri("http://ex/p3");
        let name1 = b.intern_literal(RdfLiteral::simple("Name1002"));
        let name2 = b.intern_literal(RdfLiteral::simple("Name1003"));
        let name3 = b.intern_literal(RdfLiteral::simple("Name2002"));
        let typed_int = |v: &str| RdfLiteral {
            lexical_form: v.to_owned(),
            datatype: Some(XINT.to_owned()),
            language: None,
            direction: None,
        };
        let age1 = b.intern_literal(typed_int("45"));
        let age2 = b.intern_literal(typed_int("30"));
        let age3 = b.intern_literal(typed_int("50"));
        b.push_quad(p1, name, name1, None);
        b.push_quad(p1, age, age1, None);
        b.push_quad(p2, name, name2, None);
        b.push_quad(p2, age, age2, None);
        b.push_quad(p3, name, name3, None);
        b.push_quad(p3, age, age3, None);
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

        let name_bgp = bgp(vp("x"), pred("http://ex/name"), vp("n"));
        let age_bgp = bgp(vp("x"), pred("http://ex/age"), vp("a"));
        let join = GraphPattern::Join {
            left: Box::new(name_bgp),
            right: Box::new(age_bgp),
        };

        let regex = Expression::FunctionCall(
            Function::Regex,
            vec![
                Expression::Variable(Variable::new("n")),
                Expression::Literal(Literal::new_simple("^Name1[0-9][0-9]2$")),
            ],
        );
        let numeric = Expression::Greater(
            Box::new(Expression::Variable(Variable::new("a"))),
            Box::new(Expression::Literal(Literal::new_typed(
                "40",
                NamedNode::new_unchecked(XINT),
            ))),
        );
        let cond = Expression::And(Box::new(regex), Box::new(numeric));
        let pattern = GraphPattern::Filter {
            expr: cond,
            inner: Box::new(join),
        };

        let run = |forced: bool| {
            let _guard = crate::parallel::force_parallel_for_test(forced);
            let mut ctx = EvalCtx::new(&ds);
            let seq = eval(&pattern, &mut ctx).expect("eval");
            (seq.schema.vars().to_vec(), seq.rows)
        };

        let (schema_par, rows_par) = run(true);
        let (schema_seq, rows_seq) = run(false);

        assert_eq!(
            schema_par, schema_seq,
            "schema must match regardless of path"
        );
        assert_eq!(
            rows_par, rows_seq,
            "parallel and sequential FILTER paths must produce byte-identical row order"
        );
        // Only p1 (Name1002, age 45) satisfies both the regex and the numeric bound.
        assert_eq!(rows_seq.len(), 1);
    }

    /// Determinism smoke test (Task 5): `FILTER EXISTS { ... }` evaluated once with
    /// the parallel FILTER path FORCED and once with the sequential path FORCED
    /// must produce byte-identical rows. `EXISTS` reaches no stateful builtin, so
    /// [`crate::parallel::is_parallel_safe`] must accept it.
    #[test]
    fn filter_exists_forced_parallel_and_sequential_agree() {
        use purrdf_sparql_algebra::{
            Expression, NamedNode, NamedNodePattern, TermPattern, TriplePattern, Variable,
        };

        // :a, :b carry a :stereo; :c does not.
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

        let outer = bgp(
            vp("class"),
            pred("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"),
            vp("ctype"),
        );
        let inner = bgp(vp("class"), pred("http://ex/stereo"), vp("st"));
        let pattern = GraphPattern::Filter {
            expr: Expression::Exists(Box::new(inner)),
            inner: Box::new(outer),
        };

        let run = |forced: bool| {
            let _guard = crate::parallel::force_parallel_for_test(forced);
            let mut ctx = EvalCtx::new(&ds);
            let seq = eval(&pattern, &mut ctx).expect("eval");
            (seq.schema.vars().to_vec(), seq.rows)
        };

        let (schema_par, rows_par) = run(true);
        let (schema_seq, rows_seq) = run(false);

        assert_eq!(
            schema_par, schema_seq,
            "schema must match regardless of path"
        );
        assert_eq!(
            rows_par, rows_seq,
            "parallel and sequential FILTER EXISTS paths must produce byte-identical row order"
        );
        // EXISTS keeps the two subjects with a :stereo (a, b); drops c.
        assert_eq!(rows_seq.len(), 2);
    }

    /// Determinism smoke test (Task 5): `OPTIONAL { ... FILTER ... }` (the inline
    /// `LeftJoin` filter, [`crate::binop`]'s `left_outer_join_filtered`) evaluated
    /// once with the parallel path FORCED and once with the sequential path FORCED
    /// must produce byte-identical rows, including left-alone padded rows for a
    /// left solution whose only compatible right row fails the filter.
    #[test]
    fn optional_filter_forced_parallel_and_sequential_agree() {
        use purrdf_sparql_algebra::{
            Expression, NamedNode, NamedNodePattern, TermPattern, TriplePattern, Variable,
        };

        const XINT: &str = "http://www.w3.org/2001/XMLSchema#integer";

        // :a :knows :b (age 50) — passes the OPTIONAL filter (age > 40).
        // :a :knows :c (age 10) — right row exists but fails the filter ⇒ left-alone.
        let mut b = RdfDatasetBuilder::new();
        let knows = b.intern_iri("http://ex/knows");
        let age = b.intern_iri("http://ex/age");
        let a = b.intern_iri("http://ex/a");
        let bb = b.intern_iri("http://ex/b");
        let c = b.intern_iri("http://ex/c");
        let age50 = b.intern_literal(purrdf_core::RdfLiteral {
            lexical_form: "50".to_owned(),
            datatype: Some(XINT.to_owned()),
            language: None,
            direction: None,
        });
        let age10 = b.intern_literal(purrdf_core::RdfLiteral {
            lexical_form: "10".to_owned(),
            datatype: Some(XINT.to_owned()),
            language: None,
            direction: None,
        });
        b.push_quad(a, knows, bb, None);
        b.push_quad(a, knows, c, None);
        b.push_quad(bb, age, age50, None);
        b.push_quad(c, age, age10, None);
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

        // left = { ?x :knows ?y }: the two rows (x=a,y=b) and (x=a,y=c) exercise
        // both "filter passes" (b, age 50) and "compatible right row exists but
        // fails the filter ⇒ left-alone" (c, age 10) in one shape.
        let left = bgp(vp("x"), pred("http://ex/knows"), vp("y"));
        let right = bgp(vp("y"), pred("http://ex/age"), vp("a"));
        let cond = Expression::Greater(
            Box::new(Expression::Variable(Variable::new("a"))),
            Box::new(Expression::Literal(
                purrdf_sparql_algebra::Literal::new_typed("40", NamedNode::new_unchecked(XINT)),
            )),
        );
        let pattern = GraphPattern::LeftJoin {
            left: Box::new(left),
            right: Box::new(right),
            expression: Some(cond),
        };

        let run = |forced: bool| {
            let _guard = crate::parallel::force_parallel_for_test(forced);
            let mut ctx = EvalCtx::new(&ds);
            let seq = eval(&pattern, &mut ctx).expect("eval");
            (seq.schema.vars().to_vec(), seq.rows)
        };

        let (schema_par, rows_par) = run(true);
        let (schema_seq, rows_seq) = run(false);

        assert_eq!(
            schema_par, schema_seq,
            "schema must match regardless of path"
        );
        assert_eq!(
            rows_par, rows_seq,
            "parallel and sequential OPTIONAL-FILTER paths must produce byte-identical row order"
        );
        // x=a/y=b/age=50 passes the filter; x=a/y=c fails it and falls back to a
        // left-alone row (y/a unbound) — two rows total.
        assert_eq!(rows_seq.len(), 2);
    }
}
