// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The graph-pattern evaluation recursion and its [`EvalCtx`].
//!
//! [`eval`] maps a [`GraphPattern`] to a [`SolutionSeq`] over the dataset in
//! [`EvalCtx`]. Every algebra variant is evaluated here, the host-injected
//! property-function call included: a
//! [`GraphPattern::PropertyFunction`]
//! is dispatched per driving row against the registry
//! [`EvalCtx::with_property_functions`] injected (`crate::property_fn_eval`), and a
//! call the host's table cannot answer is a typed error rather than a short row
//! stream. Anything a variant cannot evaluate hard-errors
//! ([`EvalError::Unsupported`], or [`EvalError::Function`] for a call into host code)
//! rather than returning a partial bag (the `no-optionality` doctrine).
//!
//! Evaluation pins the **concrete** [`RdfDataset`] rather than a generic
//! `DatasetView`: the value→id bridge [`RdfDataset::term_id_by_value`] (P4),
//! which BGP constant-resolution needs, is an inherent method on the frozen dataset
//! and is not part of the `DatasetView` trait. The dataset still exposes its
//! indexed read surface through `DatasetView` (the inherent `quads_for_pattern`
//! override, P4b).

use std::sync::Arc;

use purrdf_core::{
    DatasetView, GraphMatch, RdfDataset, TermFactory, TermId, TermValue, TrippedGovernor,
    ViewTermId,
};
use purrdf_sparql_algebra::{
    GraphPattern, NamedNodePattern, Query, SparqlVersion, TermPattern, TriplePattern, Update,
    Variable,
};

use crate::DetHashMap;
use crate::dataset_spec::ActiveDataset;
use crate::error::EvalError;
use crate::governor::GovernorState;
use crate::governor::ledger::ChargeLedger;
use crate::governor::lift::{Evaluated, ExpressionBarrier, Truncation};
use crate::governor::soundness::CapPushdown;
use crate::scratch::{ScratchInterner, SolutionTerm};
use crate::solution::{SolutionSeq, VarSchema};

/// Tunable evaluation behavior. Every flag defaults to the production-optimal
/// value; the criterion benches and differential tests flip individual flags to
/// measure their effect (the flags are a measurement seam, never a degraded
/// production mode).
#[derive(Debug, Clone, Copy)]
pub struct EvalOptions {
    /// Memoize each `EXISTS`/`NOT EXISTS` inner-pattern evaluation. The inner
    /// pattern is evaluated unconstrained and then joined with the outer row's
    /// seed, so its result is **independent of the outer row**: a `FILTER` over N
    /// rows can evaluate it once instead of N times. Always `true` in production.
    pub exists_memo: bool,
    /// Evaluate BGPs in the retired structural (most-constrained-first) order
    /// instead of the cost-based order. Used only by the differential planner-
    /// correctness corpus test to prove that reordering does not change the
    /// result multiset. Always `false` in production.
    pub force_structural_bgp_order: bool,
    /// Keep evaluator fork sites on their sequential implementation.
    ///
    /// This is a measurement seam for Criterion comparisons against the ordered
    /// parallel fold, and for differential tests. Production leaves it `false`.
    pub force_sequential: bool,
}

impl Default for EvalOptions {
    fn default() -> Self {
        Self {
            exists_memo: true,
            force_structural_bgp_order: false,
            force_sequential: false,
        }
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
/// Callers supply their own vocabulary, e.g. a deployment namespace such as
/// `http://example.org/ns/gmeow/accordingTo` / `…/sharpens`, via
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

/// The caller-supplied **loss-declaration vocabulary** used by loss-aware
/// `CONSTRUCT`.
///
/// When a `CONSTRUCT` template drops an RDF-1.2 reifier that was bound in the
/// `WHERE`, the engine emits in-band loss declarations. The IRIs for the loss
/// node type (`projection_loss`), the code predicate (`loss_code`), and the
/// dropped-reifier pointer (`lost_reifies`) are caller-supplied configuration:
/// PurRDF mints no vocabulary IRIs. If no vocabulary is configured, loss
/// declarations stay inactive and the query behaves like a plain `CONSTRUCT`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LossVocabulary {
    /// `rdf:type` of an in-band loss node (e.g. `…/ProjectionLoss`).
    pub projection_loss: String,
    /// Predicate carrying the machine loss code (e.g. `…/lossCode`).
    pub loss_code: String,
    /// Predicate pointing from the loss node to the dropped triple term
    /// (e.g. `…/lostReifies`).
    pub lost_reifies: String,
}

impl LossVocabulary {
    /// A vocabulary from the caller's three predicate IRIs.
    pub fn new(
        projection_loss: impl Into<String>,
        loss_code: impl Into<String>,
        lost_reifies: impl Into<String>,
    ) -> Self {
        Self {
            projection_loss: projection_loss.into(),
            loss_code: loss_code.into(),
            lost_reifies: lost_reifies.into(),
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
pub(crate) type ExistsCacheKey<I> = (usize, (u8, Option<I>), u64);

/// A memoized `EXISTS`/`NOT EXISTS` inner pattern together with the probe index built
/// over it. The inner pattern is evaluated unconstrained **once** per [`ExistsCacheKey`];
/// the `(shared, keyed, wild)` index is built once and reused to existence-probe every
/// outer row (see [`crate::binop::probe_has_match`]). This is what turns a `FILTER (NOT)
/// EXISTS` anti-join from N per-row index rebuilds into N O(1)/scan probes.
pub(crate) struct ExistsInner<I: ViewTermId = TermId> {
    /// The inner pattern's unconstrained result (outer-row-independent).
    pub inner: Arc<SolutionSeq<I>>,
    /// Shared columns between the outer schema and `inner.schema`, as
    /// `(outer_ordinal, inner_ordinal)` pairs (the probe's join key).
    pub shared: Vec<(usize, usize)>,
    /// Inner rows fully bound on the shared columns, grouped by their key.
    pub keyed: DetHashMap<crate::binop::JoinKey<I>, Vec<usize>>,
    /// Inner rows with an unbound shared column (compatible with any probe value).
    pub wild: Vec<usize>,
}

/// A cheap FNV-1a fingerprint of an outer schema's variables (names in column order),
/// for [`ExistsCacheKey`]. Two schemas with the same ordered variable list hash equal,
/// so the cached probe index is only reused against a matching outer-row layout.
pub(crate) fn schema_fingerprint(schema: &VarSchema) -> u64 {
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

/// Spell one minted blank-node label: `stem` followed by the decimal counter value
/// `n`, with `prefix` spliced in front when the evaluation has a deterministic
/// [`EvalCtx::bnode_mint_prefix`] installed.
///
/// The single formatting rule every mint site in this crate uses — CONSTRUCT
/// template blanks (`stem = "c"`, [`crate::template::mint_blank`]), `BNODE()`
/// (`stem = "bnode"`, [`crate::expr`]), and the PurRDF list constructors
/// (`stem = "lc"`, [`crate::list_fn::materialize_list`]) all call this rather than
/// re-deriving the `Some(prefix) => format!(...), None => format!(...)` match, so a
/// fourth mint site added later cannot omit the prefix branch and reopen the
/// cross-focus label collision [`EvalCtx::with_bnode_mint_prefix`] exists to
/// prevent. With `prefix: None` the result is exactly `{stem}{n}`, byte-identical
/// to every pre-prefix caller.
pub(crate) fn minted_label(prefix: Option<&str>, stem: &str, n: u64) -> String {
    match prefix {
        Some(prefix) => format!("{prefix}{stem}{n}"),
        None => format!("{stem}{n}"),
    }
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
///
/// Generic over the read view `D` (the storage-backend seam): the evaluator drives
/// the dataset entirely through the [`DatasetView`] trait, so any backend whose id
/// type is the production [`TermId`] (`D::Id = TermId`) plugs in unchanged. The
/// binding/solution id space is still concrete `TermId` in this layer; a backend with
/// its own id space bridges to `TermId` before this boundary. `D` defaults to
/// [`RdfDataset`], so the bare spelling `EvalCtx<'d>` names the production
/// instantiation everywhere it did before.
pub struct EvalCtx<'d, D: DatasetView + Sync = RdfDataset> {
    /// The frozen read view being queried, driven through the [`DatasetView`] trait.
    pub dataset: &'d D,
    /// The per-query interner for terms computed during evaluation (BIND, VALUES,
    /// aggregate output, arithmetic/string-function results).
    pub scratch: ScratchInterner,
    /// The graph currently in scope (set by `GRAPH`; the default graph at the root).
    /// At the root this is `GraphMatch::Default`, which `active_dataset` resolves to
    /// either the store default graph or a `FROM`/`USING`-merged default graph.
    pub active_graph: GraphMatch<D::Id>,
    /// The SPARQL active dataset (§13): how `active_graph == Default` is sourced and
    /// which named graphs `GRAPH` may address. Set from a query's `FROM` clause (the
    /// query path) or an UPDATE op's `USING` / `WITH` (the update path).
    pub(crate) active_dataset: ActiveDataset<D::Id>,
    /// A monotonic counter for minting fresh blank nodes (`BNODE()` and CONSTRUCT
    /// template blanks).
    pub bnode_counter: u64,
    /// An optional deterministic prefix for every blank-node label this evaluation
    /// mints (CONSTRUCT template blanks, `BNODE()`, and the PurRDF list
    /// constructors): a label the mint would spell `c{n}` becomes `{prefix}c{n}`.
    /// `None` (the default) leaves minted labels byte-identical to an unprefixed
    /// evaluation. The prefix is caller-supplied data — never derived from time,
    /// RNG, or iteration order — so a prefixed evaluation is exactly as
    /// deterministic as an unprefixed one. The SHACL rules engine supplies a
    /// per-focus-node prefix so distinct focus nodes mint distinct blanks at mint
    /// time (see [`Self::with_bnode_mint_prefix`]).
    pub bnode_mint_prefix: Option<Arc<str>>,
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
    pub(crate) bnode_memo: DetHashMap<(u64, String), SolutionTerm<D::Id>>,
    /// The evaluation-time value of NOW() — an xsd:dateTime, captured once at
    /// context construction from the host platform's real wall clock so all NOW()
    /// calls in a query return the same instant (SPARQL 1.1 §17.4.5.1).
    pub now: purrdf_xsd::XsdValue,
    /// Splitmix64 PRNG state for RAND()/UUID()/STRUUID(), seeded once at context
    /// construction from real OS/platform entropy.
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
    /// The caller-supplied loss-declaration vocabulary (see [`LossVocabulary`])
    /// used by loss-aware `CONSTRUCT`. `None` (the default) means loss
    /// declarations are inactive: a dropped reifier is projected silently, like
    /// a plain `CONSTRUCT`.
    pub loss_vocabulary: Option<LossVocabulary>,
    /// Memoized `EXISTS`/`NOT EXISTS` inner patterns **and their probe index**
    /// ([`ExistsInner`]), keyed by [`ExistsCacheKey`]. The inner eval and the index
    /// over it are outer-row-independent, so this turns `expr::exists`'s per-row
    /// re-evaluation *and* per-row index rebuild into a single build per site.
    /// Naturally per-query: a fresh [`EvalCtx`] is built for each `query()` call.
    pub(crate) exists_inner_cache: DetHashMap<ExistsCacheKey<D::Id>, Arc<ExistsInner<D::Id>>>,
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
    pub(crate) cached_bool_terms: [Option<SolutionTerm<D::Id>>; 2],
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
    pub(crate) const_atom_cache: DetHashMap<usize, SolutionTerm<D::Id>>,
    /// Per-query memo of the parsed XSD value of a dataset literal, keyed by its
    /// `TermId`. `FILTER`/comparison hot paths (`compare`/`equal`/`ebv_term`) parse
    /// the same `Existing(TermId)` literal's lexical form via `parse_by_iri` on
    /// every row; a 30k-row `?age > 40` re-parses ~60 distinct ages 30k times. The
    /// lexical form and datatype are immutable for a fixed id, so the parse is a
    /// pure function of the id — memoizing it (including the `None` "not an XSD
    /// value" outcome) collapses per-row re-parsing to one parse per distinct id.
    /// Naturally per-query. Only dataset (`Existing`) ids are cached; computed
    /// scratch values are ephemeral and stay on the borrowed-view path.
    pub(crate) xsd_parse_cache: DetHashMap<D::Id, Option<purrdf_xsd::XsdValue>>,
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
    /// The caller-injected SHACL-AF function table (`sh:SPARQLFunction`).
    /// [`crate::user_fn::UserFunctionRegistry::EMPTY`] (the default) means no user
    /// functions are declared: a call-position IRI unknown to the closed `PurrdfFn`
    /// set then falls through to the XSD-cast / unsupported path exactly as before.
    /// Borrowed for the dataset lifetime (like
    /// [`Self::remote`]/[`Self::bgp_order_cache`]), so carrying it is a `Copy`
    /// pointer, never a clone.
    pub(crate) user_functions: &'d crate::user_fn::UserFunctionRegistry,
    /// The caller-injected property-function table.
    /// [`crate::property_fn::PropertyFunctionRegistry::EMPTY`] (the default) means
    /// no relation is registered: a predicate IRI only reaches this table when the
    /// parser already lowered it to a
    /// [`purrdf_sparql_algebra::GraphPattern::PropertyFunction`]
    /// under a caller-configured namespace, and an unresolved call is the same
    /// failure regardless of whether the empty table was ever explicitly attached.
    /// Borrowed for the dataset lifetime like [`Self::user_functions`], so carrying
    /// it is a `Copy` pointer, never a clone.
    pub(crate) property_functions: &'d crate::property_fn::PropertyFunctionRegistry,
    /// The caller-injected custom-aggregate table.
    /// [`crate::agg_fn::AggregateRegistry::EMPTY`] (the default) means no aggregate
    /// is registered: an `AggregateFunction::Custom(iri)` call only reaches this
    /// table at evaluation time after `crate::property_fn_plan::plan_query`'s
    /// prepare-time walk has already admitted it against the SAME registry (see
    /// `crate::engine::check_plan_matches_relations`'s aggregate-identity check),
    /// so an unresolved call here is a defense-in-depth repeat of that refusal,
    /// never the normal path. Borrowed for the dataset lifetime like
    /// [`Self::property_functions`], so carrying it is a `Copy` pointer, never a
    /// clone.
    pub(crate) aggregates: &'d crate::agg_fn::AggregateRegistry,
    /// The current SHACL-AF function call depth, incremented by
    /// [`Self::child_for_user_fn`] and bounded by [`MAX_UDF_DEPTH`] so
    /// mutually-recursive functions fail closed rather than overflow the stack.
    pub(crate) udf_depth: u32,
    /// The caller-supplied execution governors in force, if any. `None` — the default —
    /// is an **ungoverned** execution: no ceiling can be exceeded and no stop signal can
    /// fire, so [`Self::stop_check`] short-circuits on one null test and evaluation does
    /// exactly the work it did before governors existed.
    ///
    /// Held behind an [`Arc`] rather than by value so a forked worker shares the ONE
    /// live accounting state instead of a copy: a per-worker copy would multiply the
    /// budget by the thread count, invisibly.
    pub(crate) governors: Option<Arc<GovernorState>>,
    /// The one-shot cell through which a governor trip inside an expression-embedded
    /// `EXISTS` reaches the operator that owns the expression (see
    /// [`ExpressionBarrier`]). Shared by [`Arc`] with every forked worker context, so a
    /// worker's observation is not lost when the worker's context is dropped.
    pub(crate) expression_barrier: ExpressionBarrier,
    /// The answer-cap / `LIMIT` pushdown for the plan being evaluated: the row ceiling
    /// past which each node's work cannot affect the query's answer.
    ///
    /// `None` — the default — is "no ceiling anywhere", which is every query with neither
    /// a `LIMIT` on the active subtree nor a caller answer cap. A root answer cap installs
    /// one plan for the execution; semantic slices install theirs lazily and remove it
    /// when their subtree returns. Either form is shared by [`Arc`] with every forked
    /// worker, because a worker evaluating part of a node's rows is under the same ceiling
    /// the node is.
    pub(crate) cap_pushdown: Option<Arc<CapPushdown>>,
    /// The address of the algebra node currently being evaluated, set by
    /// [`eval_evaluated`] and restored on the way out.
    ///
    /// The key both the pushdown and the ledger are looked up by. It is a plain scalar so
    /// that a fork copies it: a worker charges the node its parent was charging, which is
    /// what makes the ledger's per-node totals independent of how the work was split.
    pub(crate) current_node: usize,
    /// The per-node charge ledger, when one is installed. `None` on every ordinary query;
    /// the EXPLAIN path installs one.
    pub(crate) ledger: Option<Arc<ChargeLedger>>,
    /// The ledger ordinal of the nearest enclosing **plan** node.
    ///
    /// Distinct from [`Self::current_node`] because not every pattern the evaluator
    /// enters is in the plan: a correlated `EXISTS` builds a substituted temporary tree
    /// per outer row, and a SHACL-AF function body is a separate query entirely. Those
    /// have no ordinal, so this cursor does not move for them and their charges accrue to
    /// the operator that owns the expression — which is what makes the ledger's fuel
    /// column sum to the evidence's fuel total exactly.
    pub(crate) ledger_node: usize,
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

impl<D: DatasetView + Sync> core::fmt::Debug for EvalCtx<'_, D> {
    /// Summarized: the injected `SERVICE` source (`remote`) is a plain `dyn`
    /// trait object and the per-query caches are noise, so only the scalar
    /// evaluation state is shown.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EvalCtx")
            .field("active_graph", &self.active_graph)
            .field("bnode_counter", &self.bnode_counter)
            .field("bnode_mint_prefix", &self.bnode_mint_prefix)
            .field("now", &self.now)
            .field("rng_state", &self.rng_state)
            .field("options", &self.options)
            .field("standpoint_predicates", &self.standpoint_predicates)
            .field("loss_vocabulary", &self.loss_vocabulary)
            .finish_non_exhaustive()
    }
}

impl<'d, D: DatasetView + Sync> EvalCtx<'d, D> {
    /// A fresh context over `dataset`, scoped to the default graph.
    pub fn new(dataset: &'d D) -> Self {
        let now_val = purrdf_xsd::XsdValue::DateTime(crate::clock::wall_clock_now());
        let rng_seed: u64 = crate::clock::entropy_seed();

        // `static`, not a bare `&Registry::EMPTY` temporary: the returned `Self` must
        // outlive this function body, and a `HashMap`-backed registry's drop glue
        // blocks Rust's rvalue static promotion for a reference that has to live that
        // long (see `crate::agg_fn::AggregateRegistry::EMPTY`'s docs for why sharing
        // this one instance is the correct, not merely convenient, choice).
        static EMPTY_FUNCTIONS: crate::user_fn::UserFunctionRegistry =
            crate::user_fn::UserFunctionRegistry::EMPTY;
        static EMPTY_RELATIONS: crate::property_fn::PropertyFunctionRegistry =
            crate::property_fn::PropertyFunctionRegistry::EMPTY;
        static EMPTY_AGGREGATES: crate::agg_fn::AggregateRegistry =
            crate::agg_fn::AggregateRegistry::EMPTY;

        Self {
            dataset,
            scratch: ScratchInterner::new(),
            active_graph: GraphMatch::Default,
            active_dataset: ActiveDataset::store_default(),
            bnode_counter: 0,
            bnode_mint_prefix: None,
            current_row: 0,
            bnode_memo: DetHashMap::default(),
            now: now_val,
            rng_state: rng_seed,
            options: EvalOptions::default(),
            standpoint_predicates: None,
            loss_vocabulary: None,
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
            user_functions: &EMPTY_FUNCTIONS,
            property_functions: &EMPTY_RELATIONS,
            aggregates: &EMPTY_AGGREGATES,
            udf_depth: 0,
            governors: None,
            expression_barrier: ExpressionBarrier::default(),
            cap_pushdown: None,
            current_node: 0,
            ledger: None,
            ledger_node: ChargeLedger::root_ordinal(),
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

    /// Supply a deterministic blank-mint prefix (see [`Self::bnode_mint_prefix`]):
    /// every blank-node label this evaluation mints is spelled `{prefix}{label}`,
    /// where `{label}` is exactly the label an unprefixed evaluation would mint.
    /// The prefix must be caller-supplied, deterministic data — the SHACL rules
    /// engine passes a per-focus-node identity tag so distinct focus nodes mint
    /// distinct blanks.
    ///
    /// # Prefix validity contract
    ///
    /// `prefix` must satisfy
    /// [`purrdf_core::blank_label::is_valid_blank_node_label_prefix`]: every
    /// label this evaluation mints is `{prefix}{stem}{n}` for one of the fixed
    /// mint stems (`c`, `bnode`, `lc`) followed by a decimal counter, and that
    /// helper is exactly the check that every such concatenation stays a legal
    /// `BLANK_NODE_LABEL`. Per the fail-fast doctrine this is enforced HERE, at
    /// the setter, rather than left to surface later as a silently rewritten
    /// label at serialization egress — an out-of-alphabet prefix would
    /// otherwise mint fine and only visibly diverge from the caller's intent
    /// once [`purrdf_core::blank_label::escape_label`] rewrites it on the way
    /// out.
    ///
    /// # Errors
    ///
    /// Returns [`EvalError::Config`] if `prefix` is not a legal
    /// `BLANK_NODE_LABEL` prefix.
    pub fn with_bnode_mint_prefix(mut self, prefix: &str) -> Result<Self, EvalError> {
        if !purrdf_core::blank_label::is_valid_blank_node_label_prefix(prefix) {
            return Err(EvalError::config(format!(
                "blank-node mint prefix {prefix:?} is not a legal BLANK_NODE_LABEL prefix"
            )));
        }
        self.bnode_mint_prefix = Some(Arc::from(prefix));
        Ok(self)
    }

    /// Supply the caller's standpoint predicate table (see
    /// [`StandpointPredicates`]) for `heldIn` and loss-aware `CONSTRUCT`.
    /// Without it, `heldIn` is a hard evaluation error.
    #[must_use]
    pub fn with_standpoint_predicates(mut self, predicates: StandpointPredicates) -> Self {
        self.standpoint_predicates = Some(predicates);
        self
    }

    /// Supply the caller's loss-declaration vocabulary (see [`LossVocabulary`])
    /// so loss-aware `CONSTRUCT` can emit in-band `ProjectionLoss` declarations
    /// when a reifier is dropped by the template. Without it, loss declarations
    /// stay inactive.
    #[must_use]
    pub fn with_loss_vocabulary(mut self, vocab: LossVocabulary) -> Self {
        self.loss_vocabulary = Some(vocab);
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

    /// Attach caller-supplied execution governors to this evaluation.
    ///
    /// Build the state fresh for every execution: consumption is cumulative, so a state
    /// reused across queries would drain one query's budget into the next.
    ///
    /// # Which entry point this context may then be evaluated through
    ///
    /// A context built by hand is evaluated through [`eval`]/[`evaluate_query`], which
    /// return a complete result or a failure and have no third channel to report a trip
    /// on. So the configuration that belongs here is the **measuring** one —
    /// [`QueryGovernors::METERED`](crate::governor::QueryGovernors::METERED) engages every
    /// counter at a ceiling nothing can reach, and the cost is read off
    /// [`GovernorState::evidence`] afterwards. A configuration that can actually trip goes
    /// through
    /// [`NativeSparqlEngine::query_governed`](crate::NativeSparqlEngine::query_governed),
    /// which is where a trip is an outcome carrying the certified partial answers rather
    /// than a refusal.
    #[must_use]
    pub fn with_governors(mut self, governors: Arc<GovernorState>) -> Self {
        self.governors = Some(governors);
        self
    }

    /// Install the per-node charge ledger for this evaluation, and start its cursor at the
    /// plan root.
    #[must_use]
    pub(crate) fn with_charge_ledger(mut self, ledger: Arc<ChargeLedger>) -> Self {
        self.ledger = Some(ledger);
        self.ledger_node = ChargeLedger::root_ordinal();
        self
    }

    /// The number of output rows past which the node currently being evaluated cannot
    /// affect the query's answer, if the plan licensed one there.
    ///
    /// A leaf that can stop producing rows consults this; every other operator ignores it.
    /// Saturated into `usize` because it is compared against a materialized row count: on
    /// a 32-bit or wasm32 target a ceiling above `usize::MAX` is one no execution can
    /// reach, so clamping it is exactly "no cut".
    pub(crate) fn row_ceiling(&self) -> Option<usize> {
        let ceiling = self.cap_pushdown.as_ref()?.ceiling_at(self.current_node)?;
        Some(usize::try_from(ceiling).unwrap_or(usize::MAX))
    }

    /// The largest number of `columns`-wide rows one materialized operator bag may hold
    /// under the intermediate-cell ceiling.
    ///
    /// `None` is the zero-cost lane: the dimension is not engaged, the schema is empty
    /// (zero cells however many unit rows it carries), or the ceiling cannot be reached by
    /// a `usize`-addressable allocation on this target. Callers branch on this once before
    /// entering their row loop, so an ungoverned execution keeps its existing allocation
    /// and parallel path byte-for-byte.
    ///
    /// This only computes the inclusive bound; it does not record a trip. A producer must
    /// continue until it encounters the first qualifying row past this count, call
    /// [`Self::observe_cells`] for that attempted size, and decline to store the row. That
    /// distinction is what tells an exactly-full (complete) bag from an overflowing one
    /// without ever allocating row `limit + 1`.
    pub(crate) fn cell_row_ceiling(&self, columns: usize) -> Option<usize> {
        if columns == 0 {
            return None;
        }
        let state = self.governors.as_ref()?;
        let dimension = purrdf_core::ResourceDimension::IntermediateCells;
        if !state.is_engaged_in(dimension) {
            return None;
        }
        let columns = u64::try_from(columns).unwrap_or(u64::MAX);
        let rows = state.limits().get(dimension) / columns;
        usize::try_from(rows).ok()
    }

    /// Enter `pattern` as the node being evaluated, returning the cursors to restore.
    ///
    /// Both cursors move together, except that the ledger's only moves when `pattern` is a
    /// node of the plan the ledger was built for — see [`Self::ledger_node`].
    pub(crate) fn enter_node(&mut self, pattern: &GraphPattern) -> (usize, usize) {
        let restore = (self.current_node, self.ledger_node);
        self.current_node = std::ptr::from_ref(pattern) as usize;
        if let Some(ledger) = self.ledger.as_ref()
            && let Some(ordinal) = ledger.ordinal_of(self.current_node)
        {
            self.ledger_node = ordinal;
        }
        restore
    }

    /// Restore the cursors [`Self::enter_node`] returned.
    pub(crate) const fn leave_node(&mut self, restore: (usize, usize)) {
        self.current_node = restore.0;
        self.ledger_node = restore.1;
    }

    /// The per-node charge ledger this execution records into, if one is installed.
    ///
    /// Handed to seams that charge below the operator boundary and therefore cannot use
    /// [`Self::note_fuel`] — the property-path traversal is the one that exists today.
    pub(crate) const fn charge_ledger(&self) -> Option<&Arc<ChargeLedger>> {
        self.ledger.as_ref()
    }

    /// Record `units` of fuel spent at `point` against the current node, when a ledger is
    /// installed.
    ///
    /// Separate from the charge itself because the two answer different questions: the
    /// charge decides whether the work is allowed, and this decides where the cost is
    /// reported. Only work that was actually charged is recorded, so the ledger's fuel
    /// column always sums to the evidence's fuel total.
    #[inline]
    pub(crate) fn note_fuel(&self, point: crate::governor::ChargePoint, units: u64) {
        if let Some(ledger) = self.ledger.as_ref() {
            ledger.record_fuel(self.ledger_node, point, units);
        }
    }

    /// The governor that has already stopped this execution, if one has.
    ///
    /// **This is not a charge point.** It spends no budget and cannot itself exhaust one:
    /// it reads the latched trip and polls the host's stop signal, both of which are pure
    /// observations. That is what makes it safe to call at every operator boundary, which
    /// in turn is what gives a deadline and a cancellation the row/operator granularity
    /// they are useless without — a signal that could only be observed at a charge point
    /// would go unnoticed for as long as the evaluator happened not to charge.
    pub(crate) fn stop_check(&self) -> Option<TrippedGovernor> {
        let state = self.governors.as_ref()?;
        if let Some(tripped) = state.tripped() {
            return Some(tripped);
        }
        state
            .poll_stop()
            .map(|cause| TrippedGovernor::Stopped { cause })
    }

    /// The stop signal this execution runs under, if the caller supplied one.
    ///
    /// Handed to the `SERVICE` federation seam so a deadline or a cancellation can
    /// **prevent** a request rather than only be noticed once one has returned. A signal
    /// only the evaluator can poll cannot fire while the evaluator is blocked inside a
    /// host call, which is the one place an unbounded wait is most likely.
    pub(crate) fn stop_signal(&self) -> Option<&Arc<dyn crate::governor::StopSignal>> {
        self.governors.as_ref()?.stop_signal()
    }

    /// Latch a governor trip that a seam outside the evaluator observed, so the evidence
    /// reports the same trip the result does.
    ///
    /// Falls back to the candidate itself on an ungoverned execution, which cannot happen
    /// through the federation seam — a source only reports a governor trip when it was
    /// handed a signal, and only a governed execution has one — but is the honest answer
    /// if it ever did.
    pub(crate) fn record_trip(&self, candidate: TrippedGovernor) -> TrippedGovernor {
        match self.governors.as_ref() {
            None => candidate,
            Some(state) => state.record_trip(candidate),
        }
    }

    /// The caller-injected tables the fork-join safety walk consults — the one place
    /// this context's three registries are paired, so a call site cannot pass one and
    /// forget the others.
    pub(crate) fn safety_registries(&self) -> crate::parallel::SafetyRegistries<'d> {
        crate::parallel::SafetyRegistries {
            functions: self.user_functions,
            relations: self.property_functions,
            aggregates: self.aggregates,
        }
    }

    /// Whether a per-row loop evaluating `expr` may be forked across parallel workers.
    ///
    /// Two independent conditions, and both are about what a forked child does **not**
    /// share with its parent:
    ///
    /// 1. [`crate::parallel::is_parallel_safe`] — the expression must not reach a builtin
    ///    that mints from per-query counter/RNG state, which a fork deliberately does not
    ///    share.
    /// 2. [`crate::parallel::expression_re_enters_evaluation`] — under **engaged**
    ///    governors, the expression must not be able to call back into whole-pattern
    ///    evaluation. A fork *does* share the `Arc<GovernorState>`, and this lane has no
    ///    ordered per-item ledger, so a charge raised from inside a worker lands in shared
    ///    atomics whose total depends on the chunk geometry — i.e. on the machine's thread
    ///    count. See that function for the measurement and the full argument.
    ///
    /// Asked here rather than at the four fork sites (`FILTER`, `BIND`, `OPTIONAL`'s
    /// inline condition, and the per-group aggregate compute) so the rule is stated once
    /// and a new fork site cannot inherit half of it.
    pub(crate) fn may_fork_row_loop(&self, expr: &purrdf_sparql_algebra::Expression) -> bool {
        if !crate::parallel::is_parallel_safe(expr, self.safety_registries()) {
            return false;
        }
        !self.governors_are_engaged() || !crate::parallel::expression_re_enters_evaluation(expr)
    }

    /// Whether one `GROUP BY` group's aggregate compute may be forked across
    /// parallel workers (`modifier::eval_group`'s per-group fork gate).
    ///
    /// Two conditions, both about a forked worker computing this ONE aggregate for
    /// this ONE group in isolation from every other group:
    ///
    /// 1. Every argument expression must pass [`Self::may_fork_row_loop`], exactly
    ///    as before this aggregate seam existed.
    /// 2. If `agg` is [`purrdf_sparql_algebra::AggregateFunction::Custom`], the
    ///    IRI must resolve against [`Self::aggregates`] to an aggregate declaring
    ///    [`crate::user_fn::Volatility::Stable`] — see
    ///    [`crate::parallel::aggregate_is_unsafe`] for why an UNRESOLVED custom
    ///    aggregate is conservatively UNSAFE here (the property-function
    ///    treatment, not the scalar-function one: an unresolved relation's
    ///    volatility is unknown, where an unresolved scalar function IRI has a
    ///    defined deterministic meaning).
    ///
    /// This governs WHICH GROUP runs on which worker, gating [`crate::modifier::eval_group`]'s
    /// across-groups fork. It does NOT gate whether the fold WITHIN one group
    /// chunks: `eval_aggregate`/`eval_custom_aggregate` decide that separately —
    /// a built-in's within-group chunked fold never touches `EvalCtx` (see
    /// `crate::parallel::par_chunk_reduce_init`'s docs) so it needs no
    /// volatility check at all, and a custom aggregate's within-group chunking
    /// re-reads [`crate::agg_fn::CustomAggregate::volatility`] itself at that
    /// finer grain rather than reusing this method's answer, because a query
    /// with SEVERAL aggregates in one `GROUP BY` list — one `Custom` and
    /// `Volatile`, one built-in — must let the built-in chunk within its own
    /// group even on a query where [`Self::may_fork_aggregate`] returns `false`
    /// for the `Volatile` one (which only blocks that aggregate's own
    /// ACROSS-groups fork, not the built-in's within-group one).
    pub(crate) fn may_fork_aggregate(
        &self,
        agg: &purrdf_sparql_algebra::AggregateExpression,
    ) -> bool {
        if !agg.args().iter().all(|e| self.may_fork_row_loop(e)) {
            return false;
        }
        if let purrdf_sparql_algebra::AggregateFunction::Custom(iri) = &agg.function {
            return !crate::parallel::aggregate_is_unsafe(
                iri.as_str(),
                self.safety_registries().aggregates,
            );
        }
        true
    }

    /// Whether this execution carries a governor that actually enforces something.
    ///
    /// `false` for every ungoverned query and for one governed by
    /// [`QueryGovernors::UNBOUNDED`](crate::QueryGovernors::UNBOUNDED), which declines
    /// both the ceilings and the accounting. It is the gate on every decision that must
    /// not fork work capable of charging.
    pub(crate) fn governors_are_engaged(&self) -> bool {
        self.governors
            .as_ref()
            .is_some_and(|state| state.is_engaged())
    }

    /// Whether two **sibling patterns** may be evaluated concurrently.
    ///
    /// `UNION` is the one operator that starts both of its arms at once
    /// (`rayon::join`), and it is therefore the one place where two whole-pattern
    /// evaluations charge the same `Arc<GovernorState>` from two threads. Unlike a forked
    /// row loop, there is no expression to inspect: a sub-pattern always charges, so a
    /// governed `UNION` always races.
    ///
    /// It raced in fact, not in theory. `{ ?s ex:p ?o } UNION { ?s ex:q ?o }` under nine
    /// fuel produced **seven distinct outcomes** across sixty runs of one process — the
    /// certified answer was either five rows or none, and the reported consumption was
    /// 10, 12, 13 or 14 — because whichever arm rayon happened to schedule first drained
    /// the shared counter. That is the exact opposite of what a governor is for, and no
    /// certificate can repair it: the two arms disagree about how much budget there was.
    ///
    /// So a governed `UNION` evaluates its arms in source order on one thread, which is
    /// also the order the certificate's branch rule already assumes. An ungoverned
    /// `UNION` — which has no counter to race — keeps the fork untouched.
    pub(crate) fn may_fork_sibling_patterns(&self) -> bool {
        !self.governors_are_engaged()
    }

    /// The live governor accounting state, if this execution is governed at all.
    ///
    /// `None` is the ungoverned execution every non-governor entry point takes: every
    /// charge helper below short-circuits on this one null test, so an ungoverned query
    /// performs no atomic operation, no allocation, and no counter update anywhere on
    /// the hot path.
    pub(crate) const fn governor_state(&self) -> Option<&Arc<GovernorState>> {
        self.governors.as_ref()
    }

    /// Charge one occurrence of `point` against this execution's fuel.
    ///
    /// Two short-circuits, in this order: the ungoverned null test above, then the
    /// per-dimension engagement predicate
    /// ([`GovernorState::is_engaged_in`]) — so a caller who set only a deadline, or only
    /// an answer cap, never pays for a fuel counter it did not ask for.
    ///
    /// # Errors
    ///
    /// The governor that stopped this execution, once one has.
    ///
    /// # What the ledger sees
    ///
    /// A successful charge is recorded against the current node; the one charge that
    /// *crosses* a ceiling is not. That is a deliberate, stated one-unit difference rather
    /// than an approximation: it keeps the recording free of any read-modify-write race
    /// with a concurrent trip, and it is exactly zero on the path the ledger is actually
    /// installed for — EXPLAIN runs under
    /// [`QueryGovernors::METERED`](crate::governor::QueryGovernors::METERED), where no
    /// ceiling is reachable and therefore no charge is ever refused.
    #[inline]
    pub(crate) fn charge(
        &self,
        point: crate::governor::ChargePoint,
    ) -> Result<(), TrippedGovernor> {
        match self.governors.as_ref() {
            None => Ok(()),
            Some(state) => {
                // A stop signal is independent of fuel engagement. Every logical charge
                // point is also a bounded work checkpoint even when the caller selected
                // only a deadline or cancellation and deliberately left fuel unbounded.
                if !state.is_engaged_in(purrdf_core::ResourceDimension::Fuel)
                    && let Some(tripped) = self.stop_check()
                {
                    return Err(tripped);
                }
                let result = state.charge_point_if_engaged(point);
                if result.is_ok()
                    && self.ledger.is_some()
                    && state.is_engaged_in(purrdf_core::ResourceDimension::Fuel)
                {
                    self.note_fuel(point, point.cost());
                }
                result
            }
        }
    }

    /// Charge `amount` against `dimension`. See [`Self::charge`] for the short-circuits.
    ///
    /// # Errors
    ///
    /// The governor that stopped this execution, once one has.
    #[inline]
    pub(crate) fn charge_amount(
        &self,
        dimension: purrdf_core::ResourceDimension,
        amount: u64,
    ) -> Result<(), TrippedGovernor> {
        match self.governors.as_ref() {
            None => Ok(()),
            Some(state) => state.charge_if_engaged(dimension, amount),
        }
    }

    /// Record one operator instance's materialized bag as an observation of the
    /// intermediate-cell ceiling, **cell-denominated**: `rows * columns`.
    ///
    /// Cells, not rows: a solution row is a `SmallVec` that spills past four columns, so
    /// a two-column and a forty-column bag of the same row count are a twentyfold
    /// different allocation and a row-denominated ceiling would treat them alike.
    ///
    /// Compared inclusively against the **maximum** of any single operator instance —
    /// never a sum and never a running total. Summing would make a long, cheap query
    /// indistinguishable from one catastrophic cross product, which is the only failure
    /// this ceiling exists to stop.
    ///
    /// # Errors
    ///
    /// The governor that stopped this execution, once one has.
    #[inline]
    pub(crate) fn observe_cells(&self, rows: usize, columns: usize) -> Result<(), TrippedGovernor> {
        let cells = (rows as u64).saturating_mul(columns as u64);
        self.observe_cell_count(cells)
    }

    /// Record an already cell-denominated intermediate observation. Federation uses this
    /// when a bounded source reports the exact first response size it refused without
    /// materializing; ordinary in-engine producers use [`Self::observe_cells`].
    #[inline]
    pub(crate) fn observe_cell_count(&self, cells: u64) -> Result<(), TrippedGovernor> {
        let Some(state) = self.governors.as_ref() else {
            return Ok(());
        };
        if !state.is_engaged_in(purrdf_core::ResourceDimension::IntermediateCells) {
            return Ok(());
        }
        if let Some(ledger) = self.ledger.as_ref() {
            ledger.record_cells(self.ledger_node, cells);
        }
        state.observe_peak(purrdf_core::ResourceDimension::IntermediateCells, cells)
    }

    /// Charge the scratch arena's growth since it was last charged.
    ///
    /// The arena's minted-byte total is monotone and the charged total is exactly the
    /// consumption recorded on [`ResourceDimension::ScratchBytes`](purrdf_core::ResourceDimension::ScratchBytes),
    /// so the difference is the uncharged growth and calling this repeatedly cannot
    /// double-charge. That is what lets it run at every operator boundary — which is
    /// what makes the ceiling act *promptly*, rather than after the query has already
    /// minted its way out of memory.
    ///
    /// This dimension exists because arena growth is independent of every other meter: a
    /// `CONCAT`/`GROUP_CONCAT`/`REPLACE`/list-constructor query can exhaust memory with a
    /// perfectly satisfied row count and cell count.
    ///
    /// # Errors
    ///
    /// The governor that stopped this execution, once one has.
    #[inline]
    pub(crate) fn charge_scratch_growth(&self) -> Result<(), TrippedGovernor> {
        let Some(state) = self.governors.as_ref() else {
            return Ok(());
        };
        if !state.is_engaged_in(purrdf_core::ResourceDimension::ScratchBytes) {
            return Ok(());
        }
        let minted = self.scratch.minted_bytes();
        let charged = state.consumed_in(purrdf_core::ResourceDimension::ScratchBytes);
        if minted <= charged {
            return Ok(());
        }
        state.charge(
            purrdf_core::ResourceDimension::ScratchBytes,
            minted - charged,
        )
    }

    /// Charge one occurrence of `point` per row of `rows`, **in order**, truncating
    /// `rows` to the prefix the budget admits.
    ///
    /// The truncated bag is a positional prefix of what the operator computed, which is
    /// what the partial-lift channel needs in order to certify it. Charging in order and
    /// on the main thread is also what makes this identical under forced-parallel and
    /// forced-sequential evaluation: the rows have already been reduced into source
    /// order by the time this runs.
    pub(crate) fn admit_rows<I: ViewTermId>(
        &self,
        rows: &mut Vec<crate::solution::Solution<I>>,
        point: crate::governor::ChargePoint,
    ) -> Option<TrippedGovernor> {
        let state = self.governors.as_ref()?;
        if !state.is_engaged_in(purrdf_core::ResourceDimension::Fuel)
            && state.stop_signal().is_none()
        {
            return None;
        }
        for admitted in 0..rows.len() {
            if let Err(tripped) = self.charge(point) {
                rows.truncate(admitted);
                return Some(tripped);
            }
        }
        None
    }

    /// Charge `count` rows against fuel **without discarding any of them**, reporting the
    /// governor that stopped the charge if one did.
    ///
    /// # Why this one does not cut, when [`Self::admit_rows`] does
    ///
    /// [`Self::admit_rows`] is charged *before* per-row work that has not happened yet —
    /// a `FILTER` predicate, a `BIND` expression — so refusing a row there refuses the
    /// work, and the cut is the bound. This one is charged *after* an operator has
    /// finished: `commit_node_output` runs on a materialized bag, so truncating it saves
    /// no work at all. It would only throw away rows the evaluator has already computed
    /// and already paid for further down.
    ///
    /// That distinction is not a nicety, it is what makes a budget **monotone**. The lift
    /// is exempt from charging, so a node under a truncation passes its whole bag upward
    /// for free — while a node whose child *completed* charged for that same bag row by
    /// row and cut it. The two disagreed at exactly the budget where a child stops being
    /// truncated and starts being complete, and the disagreement ran the wrong way:
    /// `SELECT * WHERE { ?s ex:p ?o }` over three edges returned two certified rows at 7
    /// fuel and **zero** at 8, because at 7 the scan truncated and the projection lifted
    /// its rows free, while at 8 the scan completed and the projection was charged for
    /// them with nothing left. A caller paging by raising the ceiling would have watched
    /// answers disappear, which is precisely what
    /// [`PartialSparqlResult::is_positional_prefix`](crate::PartialSparqlResult::is_positional_prefix)
    /// promises cannot happen.
    ///
    /// Charging without cutting makes the two paths agree: a node reports every row it
    /// computed, whether the trip landed below it or on its own output, and the answer
    /// grows with the budget. Fuel keeps its meaning — it meters *work*, and every charge
    /// point that bounds work still cuts — while the two dimensions that bound an answer's
    /// *size*, [`ResourceDimension::AnswerRows`](purrdf_core::ResourceDimension::AnswerRows)
    /// and
    /// [`ResourceDimension::IntermediateCells`](purrdf_core::ResourceDimension::IntermediateCells),
    /// are unaffected: the cap still cuts the final answer and the cell ceiling is
    /// observed on this same bag one step earlier.
    ///
    /// Neither the schedule nor the trip point moves, so this is not a
    /// [`GOVERNOR_PROFILE_VERSION`](crate::GOVERNOR_PROFILE_VERSION) change: the charge
    /// sequence and its costs are identical, the first trip latches at the identical
    /// point, and consumption up to that point is identical. Only the rows a stopped
    /// execution is allowed to report change, and they only ever grow.
    pub(crate) fn charge_committed_rows(&self, count: usize) -> Option<TrippedGovernor> {
        let state = self.governors.as_ref()?;
        if !state.is_engaged_in(purrdf_core::ResourceDimension::Fuel) {
            return None;
        }
        let point = crate::governor::ChargePoint::CommittedOutputRow;
        for charged in 0..count {
            if let Err(tripped) = state.charge_point_if_engaged(point) {
                // Stop at the first refusal rather than charging the remaining rows into
                // a ceiling that is already crossed: the trip is latched and write-once,
                // so further charges could only inflate the reported consumption past the
                // point the execution actually stopped.
                self.note_rows(charged, point);
                return Some(tripped);
            }
        }
        self.note_rows(count, point);
        None
    }

    /// Record `count` committed output rows against the current node's ledger line, both
    /// as a row count and as the fuel those rows cost.
    #[inline]
    fn note_rows(&self, count: usize, point: crate::governor::ChargePoint) {
        if let Some(ledger) = self.ledger.as_ref() {
            ledger.record_rows(self.ledger_node, count as u64);
            ledger.record_fuel(
                self.ledger_node,
                point,
                point.cost().saturating_mul(count as u64),
            );
        }
    }

    /// Replace the evaluation options for this context. Used by the engine to thread
    /// its configured options into each per-query context, and by tests that need to
    /// flip a measurement seam.
    #[must_use]
    pub fn with_eval_options(mut self, options: EvalOptions) -> Self {
        self.options = options;
        self
    }

    /// Fork a `Send` child context for a parallel worker, sharing this context's
    /// immutable/read-only state and starting its mutable evaluation state fresh.
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
    /// Called by `expr::eval_filter` and `binop::left_outer_join_filtered` to give
    /// each FILTER-predicate worker its own child context.
    #[must_use]
    pub(crate) fn fork_for_worker(&self) -> Self {
        Self {
            dataset: self.dataset,
            scratch: self.scratch.clone(),
            active_graph: self.active_graph,
            active_dataset: self.active_dataset.clone(),
            bnode_counter: self.bnode_counter,
            // Carried exactly like `bnode_counter`: the prefix is part of the mint
            // state, so a worker that (hypothetically) minted would spell the same
            // labels the parent would. A cheap `Arc` pointer clone.
            bnode_mint_prefix: self.bnode_mint_prefix.clone(),
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
            loss_vocabulary: self.loss_vocabulary.clone(),
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
            // Read-only shared registry (a `Copy` pointer), for the same reason: a
            // worker that evaluates a property-function call must resolve the
            // predicate IRI against the same table its parent would.
            property_functions: self.property_functions,
            // Read-only shared registry (a `Copy` pointer), for the same reason: a
            // worker computing one group's fold through a registered custom
            // aggregate must resolve its IRI against the same table its parent
            // would.
            aggregates: self.aggregates,
            udf_depth: self.udf_depth,
            // SHARED, not fresh: one live accounting state across every worker, so the
            // budget is not multiplied by the thread count.
            governors: self.governors.clone(),
            // SHARED, not fresh: a worker that observes a truncation inside an
            // expression-embedded `EXISTS` must be able to tell the parent, and the
            // worker's own context is dropped the instant its closure returns.
            expression_barrier: self.expression_barrier.clone(),
            // SHARED: the plan, and therefore its row ceilings, is the same one.
            cap_pushdown: self.cap_pushdown.clone(),
            // COPIED: a worker is evaluating part of its parent's node, so it charges the
            // parent's node. Resetting either cursor would scatter one node's charges
            // across the ledger by worker count.
            current_node: self.current_node,
            ledger: self.ledger.clone(),
            ledger_node: self.ledger_node,
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
        self.user_functions = registry;
        self
    }

    /// Attach a caller-injected property-function registry for this evaluation, so a
    /// predicate IRI the parser lowered to a
    /// [`purrdf_sparql_algebra::GraphPattern::PropertyFunction`]
    /// resolves to a registered relation. The borrow shares the dataset lifetime `'d`.
    ///
    /// Attaching an EMPTY registry is exactly equivalent to attaching none: the
    /// resolution path asks the same question of both and gets the same answer, so no
    /// query's result can distinguish the two configurations.
    #[must_use]
    pub fn with_property_functions(
        mut self,
        registry: &'d crate::property_fn::PropertyFunctionRegistry,
    ) -> Self {
        self.property_functions = registry;
        self
    }

    /// Attach a caller-injected custom-aggregate registry for this evaluation, so
    /// an `AggregateFunction::Custom(iri)` call resolves to a registered
    /// aggregate. The borrow shares the dataset lifetime `'d`.
    ///
    /// Attaching an EMPTY registry is exactly equivalent to attaching none — the
    /// resolution path asks the same question of both and gets the same answer,
    /// the same pin [`Self::with_property_functions`] states for its registry.
    #[must_use]
    pub fn with_aggregates(mut self, registry: &'d crate::agg_fn::AggregateRegistry) -> Self {
        self.aggregates = registry;
        self
    }

    /// Build a child context for evaluating a SHACL-AF function body: it shares the
    /// dataset, clock/entropy, order cache, standpoint table, loss vocabulary,
    /// remote source and function registry, but starts fresh mutable evaluation state
    /// (the body is an independent query) and increments the call depth.
    ///
    /// `Ok(None)` means a governed execution reached its fixed UDF-depth ceiling. The
    /// trip is recorded on the shared expression barrier, so the operator owning the call
    /// returns typed exhaustion rather than an ordinary function error.
    ///
    /// # Errors
    ///
    /// [`EvalError::Function`] if an **ungoverned** call would exceed
    /// [`MAX_UDF_DEPTH`] — mutually-recursive functions still fail closed rather than
    /// overflow the stack when there is no governed outcome channel.
    pub(crate) fn child_for_user_fn(&self) -> Result<Option<Self>, EvalError> {
        let next_depth = self.udf_depth + 1;
        // The recursion guard is a governed dimension whose ceiling is a build constant.
        // `QueryGovernors::UNBOUNDED` already carries `MAX_UDF_DEPTH` on
        // `ResourceDimension::UdfDepth` and there is no builder that writes that slot, so
        // observing the depth here reports the consumption through the same evidence as
        // every other dimension **without** making the bound relaxable: the ceiling the
        // governor compares against and the constant below are the same number, and a
        // caller has no way to move either.
        //
        // The observation's own result is deliberately not the decision. A state that has
        // already latched some other governor answers `Err` for every dimension, and
        // reading that as "recursion too deep" would report the wrong cause; and a
        // caller-relaxable stack-recursion bound is not a bound, so the constant stays
        // the authority on both the governed and the ungoverned path.
        if let Some(state) = self.governors.as_ref()
            && let Err(tripped) = state.observe_peak(
                purrdf_core::ResourceDimension::UdfDepth,
                u64::from(next_depth),
            )
        {
            self.expression_barrier.record(tripped);
            return Ok(None);
        }
        if next_depth > MAX_UDF_DEPTH {
            return Err(EvalError::function(format!(
                "SHACL-AF function recursion exceeded the depth bound of {MAX_UDF_DEPTH}"
            )));
        }
        Ok(Some(Self {
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
            // Inherited with the counter: a function body minting under the same
            // evaluation must spell labels in the same (possibly prefixed) space.
            bnode_mint_prefix: self.bnode_mint_prefix.clone(),
            current_row: 0,
            bnode_memo: DetHashMap::default(),
            now: self.now.clone(),
            rng_state: self.rng_state,
            options: self.options,
            standpoint_predicates: self.standpoint_predicates.clone(),
            loss_vocabulary: self.loss_vocabulary.clone(),
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
            // Inherited with the function table: a function body is SPARQL like any
            // other, so a property-function call inside it resolves against the same
            // relations the calling query sees.
            property_functions: self.property_functions,
            // Inherited for the same reason: a function body's own `GROUP BY` (it is
            // SPARQL like any other) resolves a `Custom` aggregate against the same
            // registry the calling query sees.
            aggregates: self.aggregates,
            udf_depth: next_depth,
            // SHARED: a function body's evaluation spends the caller's budget, not a
            // fresh one — otherwise a query could evade its ceiling by calling a
            // function.
            governors: self.governors.clone(),
            // SHARED: a function call sits in an expression position, so a truncation
            // inside the body makes the call's value unknowable — exactly the opaque
            // edge an `EXISTS` is, and reported through the same cell so the operator
            // owning the calling expression withholds its rows.
            expression_barrier: self.expression_barrier.clone(),
            // NONE: the body is a different query with its own plan and its own modifiers,
            // so the calling query's row ceilings say nothing about it. A body that wants
            // a ceiling gets one from its own `LIMIT`.
            cap_pushdown: None,
            current_node: 0,
            // SHARED, and the cursor deliberately does NOT move: the body's nodes are not
            // in the calling plan, so its cost is reported against the call site — the one
            // node a reader of the ledger can act on.
            ledger: self.ledger.clone(),
            ledger_node: self.ledger_node,
        }))
    }

    /// A compact hashable encoding of the active graph, for [`ExistsCacheKey`].
    /// The named-graph id rides as `Option<D::Id>` (not a truncated integer) so the
    /// key is collision-free for any id width (e.g. a `u64` `GlobalTermId`).
    pub(crate) fn graph_key(&self) -> (u8, Option<D::Id>) {
        match self.active_graph {
            GraphMatch::Any => (0, None),
            GraphMatch::Default => (1, None),
            GraphMatch::Named(id) => (2, Some(id)),
        }
    }
}

/// Evaluate a graph pattern to a multiset of solutions, **trip-aware**.
///
/// The return is [`Evaluated`]: a complete bag, or a partial one carrying the certificate
/// that says what it bounds. A governor trip is therefore neither an error nor a silently
/// short result — it is a third outcome with a proof obligation attached, and every arm
/// below discharges that obligation through [`Lift`], which reads each child's transfer
/// function from the one algebra visitor rather than restating it.
///
/// Every operator receives the algebra node itself alongside its destructured children:
/// the node is what names the barrier when a bound is lost, and what identifies the
/// child edges in [`crate::governor::soundness::child_edges`]. Property paths are
/// evaluated in-engine (the `path` module); an unimplemented builtin is a typed
/// [`EvalError`], never a partial bag.
pub(crate) fn eval_evaluated<D: DatasetView + Sync>(
    pattern: &GraphPattern,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    // A semantic LIMIT is local to its Slice. Install its producer ceiling only while
    // that subtree is active, so a LIMIT-free ordinary query does not walk the plan at
    // all and two independent subquery slices do not overwrite one another.
    let local_cap = install_local_slice_pushdown(pattern, ctx);
    let evaluated = eval_evaluated_inner(pattern, ctx);
    if local_cap {
        ctx.cap_pushdown = None;
    }
    evaluated
}

/// The evaluator recursion after any local semantic-Slice ceiling has been installed.
fn eval_evaluated_inner<D: DatasetView + Sync>(
    pattern: &GraphPattern,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    // The ordinary hot path has no stop source, counters, certificate ledger, or producer
    // ceiling. Dispatch directly: no node-address cursor, atomic probe, output re-wrap, or
    // terminal checkpoint is useful when nothing can truncate. `UNBOUNDED` takes this
    // path too; its fixed UDF recursion guard remains enforced inside `child_for_user_fn`.
    if !ctx.governors_are_engaged() && ctx.ledger.is_none() && ctx.cap_pushdown.is_none() {
        return eval_node(pattern, ctx);
    }

    // Operator-granularity stop observation, before any of this node's work begins.
    // Nothing is committed yet, so the truncation originates with an empty bag — which
    // is a sound lower bound, and the ancestors that have already committed rows keep
    // theirs as the lift carries this upwards.
    if let Some(tripped) = ctx.stop_check() {
        return Ok(Evaluated::Truncated(Truncation::origin(
            SolutionSeq::empty(syntactic_schema(pattern)),
            tripped,
        )));
    }
    // The `algebra-node-entry` charge point: one unit per node visited, charged before
    // the node's own work. A trip here has committed nothing, so the truncation
    // originates with an empty bag — a sound lower bound — and the ancestors that have
    // already committed rows keep theirs as the lift carries this upwards.
    // This node becomes the cursor for the row ceiling the plan pushed to it and for the
    // ledger line its charges land on, for the whole of its own evaluation — including
    // the charges its operator makes after its children have returned, which is why the
    // cursor is restored around the recursion rather than only set before it.
    let restore = ctx.enter_node(pattern);
    if let Err(tripped) = ctx.charge(crate::governor::ChargePoint::AlgebraNodeEntry) {
        ctx.leave_node(restore);
        return Ok(Evaluated::Truncated(Truncation::origin(
            SolutionSeq::empty(syntactic_schema(pattern)),
            tripped,
        )));
    }
    let evaluated = match eval_node(pattern, ctx) {
        Ok(evaluated) => evaluated,
        Err(error) => {
            ctx.leave_node(restore);
            return Err(error);
        }
    };
    // Back at this node: a child's recursion moved the cursor and restored it, so this
    // re-entry is what makes the output charge land on the parent rather than on whichever
    // child happened to be evaluated last.
    ctx.enter_node(pattern);
    let committed = commit_node_output(evaluated, ctx);
    // The post-operator checkpoint closes the terminal-node hole: a signal that fires
    // while the final operator is running must not be returned as `Complete` merely
    // because there is no next node whose entry poll could observe it.
    let committed = match (committed, ctx.stop_check()) {
        (Evaluated::Complete(rows), Some(tripped)) => {
            Evaluated::Truncated(Truncation::origin(rows, tripped))
        }
        (evaluated, _) => evaluated,
    };
    ctx.leave_node(restore);
    Ok(committed)
}

/// Lazily install the early-producer ceiling for a semantic `Slice` subtree.
///
/// The answer-cap planner starts at the query root because its ceiling is operationally
/// outside every modifier. A SPARQL `LIMIT`/`OFFSET` already has its own algebra node, so
/// it can plan at that node when evaluation reaches it. This is both cheaper for the
/// overwhelmingly common LIMIT-free query (zero discovery walk) and more local: a slice
/// inside a subquery receives its own plan without making unrelated siblings carry it.
fn install_local_slice_pushdown<D: DatasetView + Sync>(
    pattern: &GraphPattern,
    ctx: &mut EvalCtx<'_, D>,
) -> bool {
    if ctx.cap_pushdown.is_some() {
        return false;
    }
    let GraphPattern::Slice { start, length, .. } = pattern else {
        return false;
    };
    if *start == 0 && length.is_none() {
        return false;
    }

    let pushdown = crate::governor::soundness::plan_cap_pushdown(pattern, Some(u64::MAX));
    if pushdown.is_empty() {
        false
    } else {
        ctx.cap_pushdown = Some(Arc::new(pushdown));
        true
    }
}

/// Charge one node's output and re-certify it: the `committed-output-row` charge point,
/// the intermediate-cell peak observation, and the scratch-arena growth charge, applied
/// uniformly to **every** operator at the one place every operator's result passes
/// through.
///
/// Charging here rather than inside each operator is what makes the schedule honest: a
/// new algebra variant is charged for its output the moment it is dispatched, instead of
/// costing nothing until someone remembers to add a call to it. It is also what makes the
/// charge deterministic under parallel evaluation — by the time a result reaches this
/// function its rows have already been reduced into source order by
/// [`crate::parallel`]'s index-ordered reduce, so the per-row charge below runs on one
/// thread over one fixed sequence regardless of how many workers produced it.
///
/// The order of the three charges is the precedence order of the governors they can
/// trip: the allocation ceiling first (it defends the failure mode from which there is no
/// recovery), then fuel, then the arena. Where two are true at the same point,
/// [`crate::governor::resolve_precedence`] settles it inside the state, so this order is
/// an efficiency, not a second precedence rule.
fn commit_node_output<D: DatasetView + Sync>(
    evaluated: Evaluated<D::Id>,
    ctx: &EvalCtx<'_, D>,
) -> Evaluated<D::Id> {
    // The one null test that makes an ungoverned execution free.
    let Some(state) = ctx.governors.as_ref() else {
        return evaluated;
    };

    let (rows, certificate) = match evaluated {
        Evaluated::Complete(seq) => (seq, None),
        Evaluated::Truncated(truncation) => {
            let (rows, certificate) = truncation.split();
            (rows, Some(certificate))
        }
    };

    // Already stopped, at this node or below it. The rows in hand are the prefix the
    // budget already paid for, so charging them a second time here would refuse rows the
    // certificate has already been written for. A leaf that stopped mid-scan reports its
    // trip only through the latched state, so this is also where a truncated `Bgp`,
    // `Path`, or `Values` acquires its certificate.
    if let Some(tripped) = state.tripped() {
        return Evaluated::Truncated(match certificate {
            None => Truncation::origin(rows, tripped),
            Some(certificate) => Truncation::new(rows, certificate),
        });
    }

    let width = rows.schema.len();
    let tripped = ctx
        .observe_cells(rows.rows.len(), width)
        .err()
        // Charged, never cut: this bag is already materialized, so refusing rows here
        // would discard computed answers without saving any work — and would make the
        // budget non-monotone against the free lift one row of budget away. See
        // [`EvalCtx::charge_committed_rows`].
        .or_else(|| ctx.charge_committed_rows(rows.rows.len()))
        .or_else(|| ctx.charge_scratch_growth().err());

    match (certificate, tripped) {
        // Nothing tripped and nothing had: the ordinary, complete result.
        (None, None) => Evaluated::Complete(rows),
        // The node's own output crossed a ceiling. The rows in hand are this node's whole
        // computed output — a tight positional prefix of itself — which is exactly what
        // `Truncation::origin` certifies, and the query as a whole is still stopped.
        (None, Some(tripped)) => Evaluated::Truncated(Truncation::origin(rows, tripped)),
        // A child had already truncated. The child's certificate is the one that
        // describes these rows — it names the governor that fired first and the path the
        // truncation travelled — so it is preserved whether or not this node's own charge
        // also crossed a ceiling.
        (Some(certificate), _) => Evaluated::Truncated(Truncation::new(rows, certificate)),
    }
}

/// Charge the answer cap against a query's **final** answer sequence, truncating it to
/// the prefix the cap admits.
///
/// The cap is deliberately not `LIMIT`. `LIMIT` is query semantics and has already been
/// applied by the time this runs — the pattern handed here is the fully-modified result —
/// so the cap is tested against what the caller would actually receive, which is what
/// makes it an operational governor rather than a second slice. Inclusive, like every
/// other ceiling: a result whose size equals the cap is complete.
fn commit_answer_rows<D: DatasetView + Sync>(
    evaluated: Evaluated<D::Id>,
    ctx: &EvalCtx<'_, D>,
) -> Evaluated<D::Id> {
    let Some(state) = ctx.governors.as_ref() else {
        return evaluated;
    };
    if !state.is_engaged_in(purrdf_core::ResourceDimension::AnswerRows) {
        return evaluated;
    }

    let (mut rows, certificate) = match evaluated {
        Evaluated::Complete(seq) => (seq, None),
        Evaluated::Truncated(truncation) => {
            let (rows, certificate) = truncation.split();
            (rows, Some(certificate))
        }
    };

    let mut tripped = state.tripped();
    let mut cap_cut = false;
    for admitted in 0..rows.rows.len() {
        if let Err(cap) = state.charge_final_output(purrdf_core::ResourceDimension::AnswerRows, 1) {
            rows.rows.truncate(admitted);
            tripped.get_or_insert(cap);
            cap_cut = true;
            break;
        }
    }

    match (certificate, tripped, cap_cut) {
        (None, None, _) => Evaluated::Complete(rows),
        (None, Some(tripped), _) => Evaluated::Truncated(Truncation::origin(rows, tripped)),
        (Some(certificate), _, true) => {
            Evaluated::Truncated(Truncation::after_answer_cap(rows, certificate))
        }
        (Some(certificate), _, false) => Evaluated::Truncated(Truncation::new(rows, certificate)),
    }
}

/// Dispatch one algebra node to its operator. Split out of [`eval_evaluated`] so that the
/// charge points bracketing every node are written once rather than once per variant.
fn eval_node<D: DatasetView + Sync>(
    pattern: &GraphPattern,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    match pattern {
        // Leaves: a truncation cannot happen "below" them, so they have no lift.
        GraphPattern::Bgp { patterns } => {
            Ok(Evaluated::Complete(crate::bgp::eval_bgp(patterns, ctx)?))
        }
        GraphPattern::Path {
            subject,
            path,
            object,
        } => Ok(Evaluated::Complete(crate::path::eval_path(
            subject, path, object, ctx,
        )?)),
        GraphPattern::Values {
            variables,
            bindings,
        } => Ok(Evaluated::Complete(crate::modifier::eval_values(
            variables, bindings, ctx,
        )?)),

        GraphPattern::Join { left, right } => crate::binop::eval_join(pattern, left, right, ctx),
        GraphPattern::Union { left, right } => crate::binop::eval_union(pattern, left, right, ctx),
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => crate::binop::eval_left_join(pattern, left, right, expression.as_ref(), ctx),
        GraphPattern::Minus { left, right } => crate::binop::eval_minus(pattern, left, right, ctx),
        GraphPattern::Filter { expr, inner } => crate::expr::eval_filter(pattern, expr, inner, ctx),
        GraphPattern::Extend {
            inner,
            variable,
            expression,
        } => crate::expr::eval_extend(pattern, inner, variable, expression, ctx),
        GraphPattern::Project { inner, variables } => {
            crate::modifier::eval_project(pattern, inner, variables, ctx)
        }
        GraphPattern::Distinct { inner } => crate::modifier::eval_distinct(pattern, inner, ctx),
        GraphPattern::Reduced { inner } => crate::modifier::eval_reduced(pattern, inner, ctx),
        GraphPattern::Slice {
            inner,
            start,
            length,
        } => crate::modifier::eval_slice(pattern, inner, *start, *length, ctx),
        GraphPattern::OrderBy { inner, expression } => {
            crate::modifier::eval_order_by(pattern, inner, expression, ctx)
        }
        GraphPattern::Graph { name, inner } => {
            crate::modifier::eval_graph(pattern, name, inner, ctx)
        }
        GraphPattern::Group {
            inner,
            variables,
            aggregates,
        } => crate::modifier::eval_group(pattern, inner, variables, aggregates, ctx),
        GraphPattern::Service {
            name,
            inner,
            silent,
        } => crate::remote::eval_service(pattern, name, inner, *silent, ctx),
        GraphPattern::Lateral { left, right } => {
            crate::binop::eval_lateral(pattern, left, right, ctx)
        }
        // A call reached without an enclosing `Lateral` — nothing was written before it
        // in its group — so it is driven over the identity table. The correlated shape
        // (the parser's usual one) is intercepted by `binop::eval_lateral`, which hands
        // the dispatch its left rows directly instead of substituting into this node.
        GraphPattern::PropertyFunction(call) => {
            crate::property_fn_eval::eval_property_function(call, ctx)
        }
    }
}

/// Derive the columns an algebra node exposes without evaluating it.
///
/// This is used only on early-stop and known-empty paths, where executing a child merely
/// to discover its schema would spend a budget that has already tripped or trigger side
/// effects in a branch known not to match. The match is exhaustive so a new algebra node
/// cannot silently inherit an empty schema.
pub(crate) fn syntactic_schema(pattern: &GraphPattern) -> Arc<VarSchema> {
    fn push_term(term: &TermPattern, schema: &mut VarSchema) {
        match term {
            TermPattern::Variable(variable) => {
                schema.push(variable.clone());
            }
            TermPattern::Triple(triple) => push_triple(triple, schema),
            TermPattern::NamedNode(_) | TermPattern::BlankNode(_) | TermPattern::Literal(_) => {}
        }
    }

    fn push_triple(triple: &TriplePattern, schema: &mut VarSchema) {
        push_term(&triple.subject, schema);
        if let NamedNodePattern::Variable(variable) = &triple.predicate {
            schema.push(variable.clone());
        }
        push_term(&triple.object, schema);
    }

    fn derive(pattern: &GraphPattern) -> VarSchema {
        match pattern {
            GraphPattern::Bgp { patterns } => {
                let mut schema = VarSchema::new();
                for pattern in patterns {
                    push_triple(pattern, &mut schema);
                }
                schema
            }
            GraphPattern::Path {
                subject,
                path: _,
                object,
            } => {
                let mut schema = VarSchema::new();
                push_term(subject, &mut schema);
                push_term(object, &mut schema);
                schema
            }
            GraphPattern::Join { left, right }
            | GraphPattern::LeftJoin {
                left,
                right,
                expression: _,
            }
            | GraphPattern::Lateral { left, right }
            | GraphPattern::Union { left, right } => derive(left).union(&derive(right)),
            GraphPattern::Minus { left, right: _ } => derive(left),
            GraphPattern::Filter { expr: _, inner }
            | GraphPattern::OrderBy {
                inner,
                expression: _,
            }
            | GraphPattern::Distinct { inner }
            | GraphPattern::Reduced { inner }
            | GraphPattern::Slice {
                inner,
                start: _,
                length: _,
            } => derive(inner),
            GraphPattern::Graph { name, inner } => {
                let mut schema = derive(inner);
                if let NamedNodePattern::Variable(variable) = name {
                    schema.push(variable.clone());
                }
                schema
            }
            GraphPattern::Service {
                name: _,
                inner,
                silent: _,
            } => derive(inner),
            GraphPattern::Extend {
                inner,
                variable,
                expression: _,
            } => {
                let mut schema = derive(inner);
                schema.push(variable.clone());
                schema
            }
            GraphPattern::Values {
                variables,
                bindings: _,
            }
            | GraphPattern::Project {
                inner: _,
                variables,
            } => VarSchema::from_vars(variables.iter().cloned()),
            GraphPattern::Group {
                inner: _,
                variables,
                aggregates,
            } => {
                let mut schema = VarSchema::from_vars(variables.iter().cloned());
                for (variable, _) in aggregates {
                    schema.push(variable.clone());
                }
                schema
            }
            // Every argument variable of a property function is in scope in the
            // enclosing group (the arguments are simultaneously the call's inputs and
            // its bindings), so the node's columns are exactly those variables — in
            // flattened first-seen order, subject side then object side, which is the
            // order the dispatch fills them in.
            GraphPattern::PropertyFunction(call) => {
                let mut schema = VarSchema::new();
                for term in call.subject_args.iter().chain(&call.object_args) {
                    push_term(term, &mut schema);
                }
                schema
            }
        }
    }

    Arc::new(derive(pattern))
}

/// Evaluate a graph pattern to a multiset of solutions, requiring completion.
///
/// The **completion-only** entry point, and the one every caller that holds no governor
/// certificate uses. This signature has room for a complete bag and for a failure, and for
/// nothing else, so a truncation reaching it cannot be reported as what it is: there is
/// nobody here to certify a partial bag to. It is therefore refused rather than quietly
/// returned as a short answer.
///
/// A context **may** carry governors here, and that is the supported way to *measure* an
/// execution: [`QueryGovernors::METERED`](crate::governor::QueryGovernors::METERED)
/// engages every counter at a ceiling nothing can reach, so nothing trips and the caller
/// reads the cost off [`GovernorState::evidence`]. A configuration that can actually trip
/// — any real ceiling, or a stop signal — belongs on the governed query path
/// ([`NativeSparqlEngine::query_governed`](crate::NativeSparqlEngine::query_governed)),
/// which hands back the certified partial answers instead of refusing them. Reaching a
/// trip through this function is therefore a caller-side mismatch between the ceiling set
/// and the entry point used, and it is reported as one.
///
/// # Errors
///
/// Propagates [`EvalError`] from evaluation.
pub fn eval<D: DatasetView + Sync>(
    pattern: &GraphPattern,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<SolutionSeq<D::Id>, EvalError> {
    crate::governor::soundness::validate_graph_pattern_depth(pattern)?;
    eval_evaluated(pattern, ctx)?
        .into_complete()
        .map_err(|truncation| {
            EvalError::internal(format!(
                "a governor tripped on an evaluation entry point that can only return a \
                 complete bag; run a bounding governor configuration through \
                 `NativeSparqlEngine::query_governed`, which returns the certified partial \
                 answers: {}",
                truncation.describe()
            ))
        })
}

/// The result of evaluating a top-level query form — the internal counterpart of
/// the `SparqlResult` egress model, which the engine materializes it into.
#[derive(Debug)]
pub enum Outcome<I: ViewTermId = TermId> {
    /// `SELECT` solutions (a multiset over the projected schema).
    Solutions(SolutionSeq<I>),
    /// `CONSTRUCT`/`DESCRIBE` graph result.
    Graph(Arc<RdfDataset>),
    /// `ASK` boolean.
    Boolean(bool),
}

impl<I: ViewTermId> Outcome<I> {
    /// The query form this outcome came from, for diagnostics.
    pub(crate) const fn form_label(&self) -> &'static str {
        match self {
            Self::Solutions(_) => "solution",
            Self::Graph(_) => "graph",
            Self::Boolean(_) => "boolean",
        }
    }
}

/// A top-level query form's result, trip-aware.
///
/// The pattern-level [`Evaluated`] channel carries rows; a query form's result may be a
/// boolean or a graph instead, so the certificate is carried beside the outcome rather
/// than inside it. The certificate's rows are the *pattern's* partial rows, which is what
/// a caller needs in order to know what the outcome was computed from.
#[derive(Debug)]
pub(crate) enum EvaluatedOutcome<I: ViewTermId = TermId> {
    /// The query form's full result.
    Complete(Outcome<I>),
    /// A governor tripped while evaluating the query's pattern. The outcome is what the
    /// form computed from the partial rows, and the certificate says what those rows
    /// bound.
    Truncated {
        /// The form's result over the partial rows.
        outcome: Outcome<I>,
        /// What the rows the outcome was computed from bound.
        certificate: Truncation<I>,
    },
}

/// The graph pattern a query form evaluates.
///
/// Wildcard-free, so a new query form is a compile error rather than a plan that silently
/// loses its pushdown and its ledger.
pub(crate) const fn query_pattern(query: &Query) -> &GraphPattern {
    match query {
        Query::Select { pattern, .. }
        | Query::Ask { pattern, .. }
        | Query::Construct { pattern, .. }
        | Query::Describe { pattern, .. } => pattern,
    }
}

/// Compute this query's operational answer-cap pushdown and install it on `ctx`.
///
/// # Where the root ceiling comes from
///
/// A caller's answer cap is an *operational* ceiling on the sequence that survives every
/// modifier, so it enters at the root — below nothing and above everything. A semantic
/// `LIMIT` lives at a `Slice` node and is installed lazily by
/// [`install_local_slice_pushdown`] when no root cap already supplies the composed plan.
///
/// # Why the cap's ceiling is the cap **plus one**
///
/// The cap is inclusive: a result whose size equals it is complete. A leaf stopped at
/// exactly the cap would hand the root a full sequence that the cap then charges without
/// crossing — and the caller would be told a truncated answer is the whole answer. One
/// extra row is precisely enough to tell "exactly full" from "overflowed", and it is the
/// only row the pushdown computes that the answer never uses.
///
/// # Only `SELECT` has a row-denominated root cap
///
/// For a graph-producing form the cap denominates output *triples*
/// ([`crate::construct::commit_answer_triples`]), and one solution row can instantiate a
/// whole template — so a row ceiling derived from a triple cap would be an arithmetic
/// non-sequitur. Their semantic slices are still planned locally.
fn install_answer_cap_pushdown<D: DatasetView + Sync>(query: &Query, ctx: &mut EvalCtx<'_, D>) {
    ctx.cap_pushdown = None;
    let cap = match (query, ctx.governors.as_ref()) {
        (Query::Select { .. }, Some(state))
            if state.is_engaged_in(purrdf_core::ResourceDimension::AnswerRows) =>
        {
            state
                .limits()
                .get(purrdf_core::ResourceDimension::AnswerRows)
                .saturating_add(1)
        }
        // No root cap: semantic slices install their local pushdown only if reached.
        _ => return,
    };
    let pushdown = crate::governor::soundness::plan_cap_pushdown(query_pattern(query), Some(cap));
    if !pushdown.is_empty() {
        ctx.cap_pushdown = Some(Arc::new(pushdown));
    }
}

/// What is being admitted at the `VERSION` boundary: a full query or an update
/// request. Both [`Query`] and [`Update`] carry an optional [`SparqlVersion`]
/// declared by their prologue (last-wins across repeated declarations); this is the
/// one type [`admit_version`] reads, so there is one admission function rather than
/// a query-shaped copy and an update-shaped copy of the same check.
///
/// Carrying the parsed request itself — not merely its declared version string — is
/// deliberate: a future SPARQL 1.2 Basic-profile admission check must inspect the
/// *algebra* a `VERSION "1.2-basic"` request declared over (a query pattern or an
/// update's operations), and this seam already hands `admit_version` that algebra
/// for both request shapes, so the profile check can land inside `admit_version`
/// once and apply to queries and updates alike instead of being wired twice.
#[derive(Clone, Copy)]
pub(crate) enum AdmittedRequest<'a> {
    /// A parsed query, admitted before [`evaluate_query_evaluated`] walks it.
    Query(&'a Query),
    /// A parsed update request, admitted before `crate::update::eval_update`
    /// applies it.
    Update(&'a Update),
}

impl AdmittedRequest<'_> {
    /// The request's `VERSION` declaration, if its prologue declared one.
    pub(crate) fn version(&self) -> Option<&SparqlVersion> {
        match self {
            Self::Query(query) => query.version(),
            Self::Update(update) => update.version(),
        }
    }
}

/// Admit a parsed request's `VERSION` declaration (SPARQL 1.2 Query specification
/// §4.4) — the SOLE enforcement site for both the query evaluator
/// ([`evaluate_query_evaluated`]) and the update evaluator
/// (`crate::update::eval_update`), so a request that names an unrecognized version
/// is refused identically regardless of which evaluator would otherwise run it.
///
/// Parsing is syntax-only for `VERSION` (see [`SparqlVersion`]): any string parses.
/// Evaluation is the admission boundary — a version this evaluator does not
/// recognize names a spec it does not know how to honor, so admitting it would
/// silently evaluate (or, for an update, silently *mutate*) under the wrong (or
/// unknown) semantics. [`SparqlVersion::V12`] falls through and evaluates
/// normally on the full engine. [`SparqlVersion::V12Basic`] ALSO falls through
/// here (it is a recognized version), but is then walked by
/// [`crate::basic_profile::admit`], which refuses any construct outside the
/// Basic profile the SPARQL 1.2 Query specification §4.3.1 defines (see that
/// module's docs for the spec citation and the gated construct set) — so a
/// `1.2-basic` request that stays inside the profile evaluates exactly as a
/// `1.2` one would, and one that does not is refused here, before any work is
/// spent.
pub(crate) fn admit_version(request: AdmittedRequest<'_>) -> Result<(), EvalError> {
    match request.version() {
        Some(SparqlVersion::Other(raw)) => {
            return Err(EvalError::unsupported(format!(
                "VERSION \"{raw}\" is not a recognized SPARQL version (recognized: \"1.2\", \"1.2-basic\")"
            )));
        }
        Some(SparqlVersion::V12Basic) => crate::basic_profile::admit(request)?,
        Some(SparqlVersion::V12) | None => {}
    }
    Ok(())
}

/// Evaluate a top-level [`Query`] form over `ctx`'s dataset, trip-aware.
///
/// `SELECT`/`ASK` walk the modifier-wrapped pattern; `CONSTRUCT` and `DESCRIBE` emit
/// the IR dataset directly (`DESCRIBE` via the canonical Symmetric CBD).
///
/// # `ASK` under a trip settles its value without claiming completion
///
/// `ASK` asks whether *any* solution exists, so a certified **lower** bound settles its
/// semantic value: a single row that is certainly an answer proves `true`, whichever
/// governor stopped the search. The execution still stopped operationally, so the result
/// stays [`EvaluatedOutcome::Truncated`] and carries that boolean as its certified partial.
/// Reporting [`EvaluatedOutcome::Complete`] beside a latched trip would contradict the
/// evidence. An empty lower bound leaves the boolean `false` and the question open.
pub(crate) fn evaluate_query_evaluated<D: DatasetView + Sync>(
    query: &Query,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<EvaluatedOutcome<D::Id>, EvalError> {
    admit_version(AdmittedRequest::Query(query))?;
    crate::governor::soundness::validate_graph_pattern_depth(query_pattern(query))?;
    // Criterion and differential tests can hold the operation on the sequential branch;
    // production keeps the ordered parallel fold. The guard is operation-scoped so every
    // recursive fork gate sees the same decision.
    let _sequential = ctx
        .options
        .force_sequential
        .then(crate::parallel::force_sequential_operation);
    // Install the query's FROM / FROM NAMED active dataset (§13) before evaluating.
    ctx.active_dataset = ActiveDataset::from_query_dataset(query.dataset(), ctx.dataset);
    // Install the query's effective base IRI so IRI()/URI() can resolve a relative
    // string argument against it (SPARQL 1.1 §17.4.2.6).
    ctx.base_iri = query.base_iri().map(|nn| nn.as_str().to_owned());
    install_answer_cap_pushdown(query, ctx);
    match query {
        Query::Select { pattern, .. } => {
            match commit_answer_rows(eval_evaluated(pattern, ctx)?, ctx) {
                Evaluated::Complete(seq) => Ok(EvaluatedOutcome::Complete(Outcome::Solutions(seq))),
                Evaluated::Truncated(certificate) => Ok(EvaluatedOutcome::Truncated {
                    outcome: Outcome::Solutions(certificate.rows().clone()),
                    certificate,
                }),
            }
        }
        Query::Ask { pattern, .. } => match eval_evaluated(pattern, ctx)? {
            Evaluated::Complete(seq) => Ok(EvaluatedOutcome::Complete(Outcome::Boolean(
                !seq.is_empty(),
            ))),
            Evaluated::Truncated(certificate) => Ok(EvaluatedOutcome::Truncated {
                // A non-empty certain lower bound settles the semantic value `true`, but
                // the execution still stopped operationally. Keep the typed exhaustion
                // and carry that settled boolean as its certified partial instead of
                // contradicting the evidence with `Complete + tripped`.
                outcome: Outcome::Boolean(
                    certificate
                        .certain_rows()
                        .is_some_and(|rows| !rows.is_empty()),
                ),
                certificate,
            }),
        },
        Query::Construct {
            template, pattern, ..
        } => {
            let (graph, certificate) = crate::construct::eval_construct(template, pattern, ctx)?;
            Ok(match certificate {
                None => EvaluatedOutcome::Complete(Outcome::Graph(graph)),
                Some(certificate) => EvaluatedOutcome::Truncated {
                    outcome: Outcome::Graph(graph),
                    certificate,
                },
            })
        }
        Query::Describe {
            pattern, targets, ..
        } => {
            let (graph, certificate) = crate::describe_query::eval_describe(pattern, targets, ctx)?;
            Ok(match certificate {
                None => EvaluatedOutcome::Complete(Outcome::Graph(graph)),
                Some(certificate) => EvaluatedOutcome::Truncated {
                    outcome: Outcome::Graph(graph),
                    certificate,
                },
            })
        }
    }
}

/// Evaluate a top-level [`Query`] form over `ctx`'s dataset, requiring completion.
///
/// The **completion-only** entry point; see [`eval`] for why a trip reaching it is
/// refused rather than answered, and for the measurement configuration that is welcome
/// here.
///
/// # Errors
///
/// Propagates [`EvalError`] from evaluation.
pub fn evaluate_query<D: DatasetView + Sync>(
    query: &Query,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Outcome<D::Id>, EvalError> {
    match evaluate_query_evaluated(query, ctx)? {
        EvaluatedOutcome::Complete(outcome) => Ok(outcome),
        EvaluatedOutcome::Truncated {
            outcome,
            certificate,
        } => Err(EvalError::internal(format!(
            "a governor tripped on a query entry point that can only return a complete {} \
             result; run a bounding governor configuration through \
             `NativeSparqlEngine::query_governed`, which returns the certified partial \
             answers: {}",
            outcome.form_label(),
            certificate.describe()
        ))),
    }
}

/// Materialize a [`SolutionSeq`] into dataset-independent egress form: the
/// projected variable names plus the owned [`TermValue`] rows (a `None` cell is
/// an unbound binding). The interned-`TermId` space ends here.
///
/// Shared by the engine's `SparqlResult` materializer and the SERVICE result
/// path, both of which turn an interned solution sequence into owned
/// term values via the per-query [`ScratchInterner`](crate::scratch::ScratchInterner).
pub(crate) fn materialize_solutions<D: DatasetView + Sync>(
    seq: &SolutionSeq<D::Id>,
    ctx: &EvalCtx<'_, D>,
) -> (Vec<String>, Vec<Vec<Option<TermValue>>>) {
    let variables = seq
        .schema
        .vars()
        .iter()
        .map(|v| v.as_str().to_owned())
        .collect();
    // Literal datatype IRIs repeat massively across a result (a handful of XSD
    // types over tens of thousands of cells), so each datatype id is resolved
    // once per call and cloned from a small memo instead of re-resolved per cell.
    let mut datatype_memo: DetHashMap<D::Id, String> = DetHashMap::default();
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
fn memoized_value_of<D: DatasetView + Sync>(
    ctx: &EvalCtx<'_, D>,
    term: SolutionTerm<D::Id>,
    datatype_memo: &mut DetHashMap<D::Id, String>,
) -> TermValue {
    match term {
        SolutionTerm::Existing(id) => memoized_term_value(ctx.dataset, id, datatype_memo),
        SolutionTerm::Computed(_) => ctx.scratch.value_of(ctx.dataset, term),
    }
}

/// `scratch::term_id_to_value`, with the literal datatype id → IRI string
/// resolution memoized across cells (recursing through RDF-1.2 triple terms).
fn memoized_term_value<D: DatasetView>(
    dataset: &D,
    id: D::Id,
    datatype_memo: &mut DetHashMap<D::Id, String>,
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
    fn bnode_mint_prefix_rejects_an_illegal_prefix() {
        let ds = RdfDatasetBuilder::new().freeze().expect("freeze empty");
        for illegal in ["a b", "-lead", "<urn:x>", "a\tb", "a/b"] {
            let err = EvalCtx::new(&ds)
                .with_bnode_mint_prefix(illegal)
                .expect_err(&format!("{illegal:?} must be rejected"));
            assert!(
                matches!(err, EvalError::Config(_)),
                "{illegal:?} -> {err:?}"
            );
            assert!(
                err.to_string().contains(illegal) || err.to_string().contains("prefix"),
                "error message should name the problem: {err}"
            );
        }
    }

    #[test]
    fn bnode_mint_prefix_accepts_a_shapes_style_prefix() {
        // The exact shape `focus_tag` (crates/shapes/src/rules.rs) mints:
        // leading `f`, alphanumeric/`-`-hex-escaped body, trailing `_`.
        let ds = RdfDatasetBuilder::new().freeze().expect("freeze empty");
        let ctx = EvalCtx::new(&ds)
            .with_bnode_mint_prefix("f2d616263-2d_")
            .expect("shapes-style prefix must be accepted");
        assert_eq!(ctx.bnode_mint_prefix.as_deref(), Some("f2d616263-2d_"));
    }

    #[test]
    fn unrecognized_version_is_refused_at_evaluation_admission() {
        use purrdf_sparql_algebra::SparqlParser;

        let ds = RdfDatasetBuilder::new().freeze().expect("freeze empty");
        let query = SparqlParser::new()
            .parse_query("VERSION \"0.9\"\nSELECT * WHERE { ?s ?p ?o }")
            .expect("VERSION is syntax-only; any string parses");
        let mut ctx = EvalCtx::new(&ds);
        let err = evaluate_query(&query, &mut ctx).expect_err("unrecognized VERSION is refused");
        assert!(matches!(err, EvalError::Unsupported(_)), "got {err:?}");
        assert!(
            err.to_string().contains("0.9"),
            "error should name the declared version: {err}"
        );
    }

    #[test]
    fn recognized_versions_evaluate_normally() {
        use purrdf_sparql_algebra::SparqlParser;

        let ds = RdfDatasetBuilder::new().freeze().expect("freeze empty");
        for version in ["1.2", "1.2-basic"] {
            let query = SparqlParser::new()
                .parse_query(&format!(
                    "VERSION \"{version}\"\nSELECT * WHERE {{ ?s ?p ?o }}"
                ))
                .expect("parse");
            let mut ctx = EvalCtx::new(&ds);
            let outcome = evaluate_query(&query, &mut ctx)
                .unwrap_or_else(|e| panic!("VERSION {version:?} must evaluate: {e}"));
            assert!(
                matches!(outcome, Outcome::Solutions(seq) if seq.is_empty()),
                "empty dataset yields no solutions"
            );
        }
    }

    #[test]
    fn basic_profile_refuses_a_triple_term_and_names_it() {
        use purrdf_sparql_algebra::SparqlParser;

        let ds = RdfDatasetBuilder::new().freeze().expect("freeze empty");
        let query = SparqlParser::new()
            .parse_query(
                "VERSION \"1.2-basic\"\n\
                 PREFIX : <http://example.org/>\n\
                 SELECT * WHERE { ?r :reifies <<( ?s ?p ?o )>> }",
            )
            .expect("triple terms parse under any VERSION (syntax-only)");
        let mut ctx = EvalCtx::new(&ds);
        let err = evaluate_query(&query, &mut ctx)
            .expect_err("a triple term outside the Basic profile is refused");
        assert!(matches!(err, EvalError::Unsupported(_)), "got {err:?}");
        assert!(
            err.to_string().contains("triple term"),
            "error should name the offending construct: {err}"
        );
    }

    #[test]
    fn basic_profile_answers_a_within_profile_query() {
        use purrdf_sparql_algebra::SparqlParser;

        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("http://example.org/s");
        let p = b.intern_iri("http://example.org/p");
        let o = b.intern_iri("http://example.org/o");
        b.push_quad(s, p, o, None);
        let ds = b.freeze().expect("freeze");

        let query = SparqlParser::new()
            .parse_query(
                "VERSION \"1.2-basic\"\n\
                 SELECT * WHERE { ?s ?p ?o . FILTER(BOUND(?s)) }",
            )
            .expect("parse");
        let mut ctx = EvalCtx::new(&ds);
        let outcome = evaluate_query(&query, &mut ctx)
            .expect("a query that stays inside the Basic profile still answers");
        assert!(
            matches!(outcome, Outcome::Solutions(seq) if seq.len() == 1),
            "expected the one matching solution"
        );
    }

    #[test]
    fn full_profile_version_is_unaffected_by_the_basic_gate() {
        use purrdf_sparql_algebra::SparqlParser;

        let ds = RdfDatasetBuilder::new().freeze().expect("freeze empty");
        let query = SparqlParser::new()
            .parse_query(
                "VERSION \"1.2\"\n\
                 PREFIX : <http://example.org/>\n\
                 SELECT * WHERE { ?r :reifies <<( ?s ?p ?o )>> }",
            )
            .expect("parse");
        let mut ctx = EvalCtx::new(&ds);
        let outcome = evaluate_query(&query, &mut ctx)
            .expect("VERSION \"1.2\" admits triple terms; the Basic gate never runs");
        assert!(
            matches!(outcome, Outcome::Solutions(seq) if seq.is_empty()),
            "empty dataset yields no solutions"
        );
    }

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
        // so `expr::eval_filter` routes it through
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
        let s = |names: &[&str]| VarSchema::from_vars(names.iter().map(|n| Variable::new(*n)));
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

    /// Determinism smoke test: a query exercising BGP, JOIN, a
    /// non-filtered OPTIONAL, and MINUS evaluated once with the parallel path
    /// FORCED (via [`crate::parallel::force_parallel_for_test`]) and once with
    /// the sequential path FORCED must produce byte-identical `Vec<Solution>`
    /// rows (schema and row order both). This is a narrower, faster-running
    /// tripwire than the full [`crate::parallel_determinism_gate`] sweep — it
    /// catches an ordering regression in any of those four read-only nodes
    /// immediately, something the conformance suite's multiset comparisons
    /// would not.
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

    /// Determinism smoke test: `FILTER(REGEX(...) && ?a > k)` — the
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

    /// Determinism smoke test: `FILTER EXISTS { ... }` evaluated once with
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

    /// Determinism smoke test: `OPTIONAL { ... FILTER ... }` (the inline
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

    // -----------------------------------------------------------------------
    // The partial-lift channel, end to end
    // -----------------------------------------------------------------------

    /// A [`crate::StopSignal`] that fires on its `n`-th poll and latches thereafter.
    ///
    /// A latched-from-the-start signal only ever truncates the first node, which proves
    /// nothing about the lift; firing part-way through is what puts committed rows in the
    /// evaluator's hands at the moment it stops.
    #[derive(Debug)]
    struct StopOnPoll {
        /// Polls remaining before the signal fires.
        remaining: std::sync::atomic::AtomicU64,
    }

    impl crate::StopSignal for StopOnPoll {
        fn poll(&self) -> Option<purrdf_core::StopCause> {
            let previous = self
                .remaining
                .fetch_update(
                    std::sync::atomic::Ordering::Relaxed,
                    std::sync::atomic::Ordering::Relaxed,
                    |left| Some(left.saturating_sub(1)),
                )
                .unwrap_or(0);
            // Latching: once the countdown reaches zero it stays there, so every later
            // poll answers the same cause.
            (previous == 0).then_some(purrdf_core::StopCause::Cancelled)
        }
    }

    /// The fixture for the channel tests: a two-hop `knows` chain with `likes` edges, so
    /// a LATERAL over it emits one block per left row and a truncation lands between
    /// blocks.
    fn chain_dataset() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let knows = b.intern_iri("https://example.org/knows");
        let likes = b.intern_iri("https://example.org/likes");
        let a = b.intern_iri("https://example.org/a");
        let bb = b.intern_iri("https://example.org/b");
        let c = b.intern_iri("https://example.org/c");
        let tea = b.intern_iri("https://example.org/tea");
        let cake = b.intern_iri("https://example.org/cake");
        b.push_quad(a, knows, bb, None);
        b.push_quad(bb, knows, c, None);
        b.push_quad(a, likes, tea, None);
        b.push_quad(bb, likes, cake, None);
        b.push_quad(c, likes, tea, None);
        b.freeze().expect("freeze")
    }

    /// `LATERAL { ?x :knows ?y } { ?y :likes ?z }`: the right side is evaluated once per
    /// left row, which is the commit boundary the channel is tested at.
    fn lateral_plan() -> GraphPattern {
        use purrdf_sparql_algebra::{NamedNode, NamedNodePattern, TermPattern, TriplePattern};
        let vp = |n: &str| TermPattern::Variable(Variable::new(n));
        let pred = |iri: &str| NamedNodePattern::NamedNode(NamedNode::new_unchecked(iri));
        let bgp = |s, p, o| GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: s,
                predicate: p,
                object: o,
            }],
        };
        GraphPattern::Lateral {
            left: Box::new(bgp(vp("x"), pred("https://example.org/knows"), vp("y"))),
            right: Box::new(bgp(vp("y"), pred("https://example.org/likes"), vp("z"))),
        }
    }

    #[test]
    fn a_certified_partial_is_always_a_prefix_of_the_ungoverned_answer() {
        // The certificate's whole purpose, exercised through production code: for every
        // point at which the execution can be stopped, whatever crosses as a certified
        // lower bound must be an initial segment of what the ungoverned run returns.
        // A fabricated row — one the true answer does not contain — fails this at the
        // first budget that produces it.
        let ds = chain_dataset();
        let plan = lateral_plan();

        let mut ctx = EvalCtx::new(&ds);
        let full = eval(&plan, &mut ctx).expect("ungoverned");
        assert!(!full.rows.is_empty(), "the fixture must produce rows");

        let mut saw_partial_with_rows = false;
        let mut saw_complete = false;
        for budget in 1..=30_u64 {
            let signal = Arc::new(StopOnPoll {
                remaining: std::sync::atomic::AtomicU64::new(budget),
            });
            let governors = crate::QueryGovernors::UNBOUNDED.with_stop_signal(signal);
            let state = Arc::new(GovernorState::new(&governors));
            let mut ctx = EvalCtx::new(&ds).with_governors(state);

            match eval_evaluated(&plan, &mut ctx).expect("governed eval") {
                Evaluated::Complete(seq) => {
                    saw_complete = true;
                    assert_eq!(
                        seq.rows, full.rows,
                        "a completed governed run must be byte-identical to the \
                         ungoverned one"
                    );
                }
                Evaluated::Truncated(truncation) => {
                    assert_eq!(
                        truncation.bound(),
                        crate::governor::soundness::SpineClass::Certain,
                        "every node on this plan's spine is prefix-monotone"
                    );
                    let rows = truncation
                        .certain_rows()
                        .expect("a lower bound carries its rows");
                    assert!(
                        rows.rows.len() <= full.rows.len()
                            && full.rows[..rows.rows.len()] == rows.rows[..],
                        "budget {budget}: the certified rows must be an initial segment \
                         of the true answer, not merely a subset of it"
                    );
                    saw_partial_with_rows |= !rows.rows.is_empty();
                }
            }
        }

        assert!(
            saw_complete,
            "the largest budgets must let the query finish, or the test proves nothing \
             about the complete path"
        );
        assert!(
            saw_partial_with_rows,
            "some budget must stop the execution with rows already committed, or the \
             lift is never exercised"
        );
    }

    #[test]
    fn an_already_latched_stop_signal_truncates_before_any_work() {
        let ds = chain_dataset();
        let plan = lateral_plan();
        let flag = crate::CancellationFlag::new();
        flag.cancel();
        let governors = crate::QueryGovernors::UNBOUNDED.with_stop_signal(Arc::new(flag));
        let state = Arc::new(GovernorState::new(&governors));
        let mut ctx = EvalCtx::new(&ds).with_governors(state);

        let Evaluated::Truncated(truncation) =
            eval_evaluated(&plan, &mut ctx).expect("governed eval")
        else {
            panic!("a latched stop signal must truncate the first node entered");
        };
        assert_eq!(
            truncation.tripped(),
            TrippedGovernor::Stopped {
                cause: purrdf_core::StopCause::Cancelled,
            }
        );
        assert!(truncation.rows().is_empty());
        assert_eq!(
            truncation
                .rows()
                .schema
                .vars()
                .iter()
                .map(Variable::as_str)
                .collect::<Vec<_>>(),
            ["x", "y", "z"],
            "an early stop preserves the algebra's projected columns"
        );
        assert_eq!(
            truncation.bound(),
            crate::governor::soundness::SpineClass::Certain,
            "the empty bag is a sound lower bound"
        );

        // The completion-only entry point refuses to hand a partial back as an answer.
        let flag = crate::CancellationFlag::new();
        flag.cancel();
        let governors = crate::QueryGovernors::UNBOUNDED.with_stop_signal(Arc::new(flag));
        let mut governed =
            EvalCtx::new(&ds).with_governors(Arc::new(GovernorState::new(&governors)));
        assert!(
            eval(&plan, &mut governed).is_err(),
            "a truncation reaching the completion-only entry point must be refused"
        );
    }
}
