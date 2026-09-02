// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Dynamic, host-injected user functions — of two kinds under one IRI namespace.
//!
//! A call-position IRI that is under no configured extension-function namespace
//! (so not the closed, parse-time-resolved `PurrdfFn` set) is lowered by the
//! parser to [`Function::Custom`](purrdf_sparql_algebra::Function::Custom) and
//! resolved at eval time against a caller-injected [`UserFunctionRegistry`] — the
//! open counterpart to `PurrdfFn` dispatch. The registry holds two independent
//! kinds keyed by IRI, hard-failing on a cross-kind collision so an IRI is
//! unambiguously one kind or the other:
//!
//! - **SHACL-AF SPARQL-bodied** ([`UserFunction`]) — declared by a shapes graph as
//!   an IRI typed `sh:SPARQLFunction` with ordered `sh:parameter`s, an optional
//!   `sh:returnType`, and a `sh:select`/`sh:ask` body. This kind is pure data
//!   (parsed body + parameter metadata); executing a call binds the arguments to
//!   the parameter variables as a pre-binding rewrite (the same `crate::substitute`
//!   path `$this` injection uses) and evaluates the body in a recursion-bounded
//!   child context, keeping SPARQL execution inside the evaluator and the registry
//!   free of engine coupling.
//!
//! - **Native (host-Rust closure)** ([`NativeFunction`], registered via
//!   [`UserFunctionRegistry::register_native`]) — a `Send + Sync` closure over the
//!   already-evaluated argument values (see [`NativeFnBody`]) for scoring
//!   predicates whose bodies cannot be expressed in SPARQL (e.g. an external-index
//!   read). It carries a declared [`Arity`] (fail-fast checked before the closure
//!   runs) and a [`Volatility`] the fork-join parallel-evaluation gate consults to
//!   decide whether the call may run across worker threads. It takes no
//!   [`EvalCtx`] and so cannot re-enter the evaluator.
//!
//! - **Dataset-aware (expression-bodied)** ([`ExprFunction`], registered via
//!   [`UserFunctionRegistry::register_expr`]) — a `Send + Sync` closure that, unlike
//!   the native kind, is handed the GRAPH the calling query is running over plus the
//!   current call depth, through [`ExprFnCall`]. It exists for the SHACL 1.2 SPARQL
//!   Extensions §7 "Declaring SPARQL Functions based on Node Expressions" seam, where
//!   the body is a node expression evaluated against a focus graph rather than a
//!   SPARQL query. See [`ExprFnCall::focus_graph`] for why the graph is a concrete
//!   [`Arc<RdfDataset>`] rather than the context's own `D: DatasetView` (which is not
//!   object-safe — it carries an associated `Id` type), and `eval_expr_function` for
//!   the re-entrancy bound.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use purrdf_core::{DatasetView, RdfDataset, TermValue};
use purrdf_sparql_algebra::Query;

use crate::DetHashMap;
use crate::error::EvalError;
use crate::eval::{
    EvalCtx, EvaluatedOutcome, Outcome, evaluate_query_evaluated, materialize_solutions,
};

/// The result form of a function body: a `sh:select` returns the first projected
/// value of the first solution; a `sh:ask` returns an `xsd:boolean`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserFnBody {
    /// A `sh:select` body: the return value is the first projected variable of the
    /// first solution row (empty result ⇒ no value).
    Select,
    /// A `sh:ask` body: the return value is the `xsd:boolean` of the ASK.
    Ask,
}

/// The `sh:nodeKind` of a parameter or return value, when constrained.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    /// `sh:IRI`.
    Iri,
    /// `sh:BlankNode`.
    BlankNode,
    /// `sh:Literal`.
    Literal,
    /// `sh:BlankNodeOrIRI`.
    BlankNodeOrIri,
    /// `sh:BlankNodeOrLiteral`.
    BlankNodeOrLiteral,
    /// `sh:IRIOrLiteral`.
    IriOrLiteral,
}

/// The optional `sh:datatype`/`sh:nodeKind` type constraint on a parameter or the
/// return value. An empty constraint (`None`/`None`) accepts any term.
#[derive(Debug, Clone, Default)]
pub struct TypeConstraint {
    /// The required literal datatype IRI (`sh:datatype`), if any.
    pub datatype: Option<String>,
    /// The required node kind (`sh:nodeKind`), if any.
    pub node_kind: Option<NodeKind>,
}

impl TypeConstraint {
    /// Whether this constraint imposes any requirement.
    fn is_any(&self) -> bool {
        self.datatype.is_none() && self.node_kind.is_none()
    }

    /// Validate `value` against this constraint. `role` names the position
    /// (`parameter ?var` / `return value`) for the error message.
    fn check(&self, iri: &str, role: &str, value: &TermValue) -> Result<(), EvalError> {
        if self.is_any() {
            return Ok(());
        }
        if let Some(nk) = self.node_kind
            && !matches_node_kind(value, nk)
        {
            return Err(EvalError::function(format!(
                "SHACL-AF function <{iri}> {role} violates its sh:nodeKind constraint"
            )));
        }
        if let Some(dt) = &self.datatype {
            let ok = matches!(value, TermValue::Literal { datatype, .. } if datatype == dt);
            if !ok {
                return Err(EvalError::function(format!(
                    "SHACL-AF function <{iri}> {role} is not a literal of datatype <{dt}>"
                )));
            }
        }
        Ok(())
    }
}

/// A parameter of a [`UserFunction`]: the pre-bound variable name plus its type
/// constraint. Parameters are stored in call order (ascending `sh:order`, IRI as a
/// deterministic tiebreak).
#[derive(Debug, Clone)]
pub struct UserFnParam {
    /// The pre-bound SPARQL variable name (the local name of the parameter's
    /// `sh:path`/`sh:predicate` predicate).
    pub var: String,
    /// The parameter's `sh:datatype`/`sh:nodeKind` constraint.
    pub constraint: TypeConstraint,
}

/// A declared SHACL-AF SPARQL-based function: its ordered parameters, the count of
/// leading required (non-`sh:optional`) parameters, the parsed body, and the
/// return-value constraint.
#[derive(Debug, Clone)]
pub struct UserFunction {
    /// The parameters in call order.
    pub params: Vec<UserFnParam>,
    /// The number of leading required parameters (arity is `[required, params.len()]`).
    pub required: usize,
    /// The parsed `sh:select`/`sh:ask` body.
    pub body: Arc<Query>,
    /// Whether the body is a SELECT or an ASK.
    pub kind: UserFnBody,
    /// The `sh:returnType` constraint on the produced value, if declared.
    pub return_constraint: TypeConstraint,
}

/// A native (host-Rust) user function body: a closure over the already-evaluated,
/// dataset-independent argument values.
///
/// The arguments arrive as a slice of **borrows** (`&[&TermValue]`): every value
/// already lives in the evaluator's per-call argument buffer for the duration of
/// the call, so the dispatch path lends the closure those borrows rather than deep
/// cloning each [`TermValue`] (which owns heap strings for IRIs/literals) on the
/// per-row scoring hot path this seam exists to serve. A closure that needs to
/// retain a value past the call clones it itself.
///
/// Unlike a [`UserFunction`]'s SPARQL body, this takes **no [`EvalCtx`]** — it
/// therefore cannot re-enter the evaluator, so there is no recursion/re-entrancy
/// boundary to bound (the SPARQL-bodied path's `MAX_UDF_DEPTH` guard does not
/// apply here). When a function is declared non-[`Volatile`](Volatility::Volatile)
/// it **must** be deterministic within a query: the fork-join parallel-evaluation
/// gate relies on that declaration alone to decide whether the call may run across
/// worker threads, so a closure that lies about its own volatility can silently
/// diverge under parallel evaluation.
pub type NativeFnBody = Arc<dyn Fn(&[&TermValue]) -> Result<TermValue, EvalError> + Send + Sync>;

/// Everything a dataset-aware (expression-bodied) user function is given when it is
/// called: the IRI it was called through, the already-evaluated arguments, the graph
/// the calling query is running over, and the current user-function call depth.
///
/// This is the whole of the difference between [`ExprFnBody`] and [`NativeFnBody`].
/// A native closure sees only values and therefore cannot read a graph or re-enter
/// an evaluator; an expression-bodied one is defined by a SHACL node expression,
/// whose evaluation is a function OF a graph (`evalExpr(expr, focusGraph, focusNode,
/// scope)`), so the graph has to travel with the call.
#[derive(Debug)]
#[non_exhaustive]
pub struct ExprFnCall<'a> {
    /// The call-position IRI the function was reached through.
    pub iri: &'a str,
    /// The already-evaluated argument values in call order; a `None` cell is an
    /// unbound argument.
    pub args: &'a [Option<TermValue>],
    /// The graph the calling query is running over — the specification's
    /// `focusGraph`.
    ///
    /// A concrete `Arc<RdfDataset>` rather than the evaluation context's own
    /// `D: DatasetView`: `DatasetView` carries an associated `Id` type, so `&dyn
    /// DatasetView` is not a legal type and the generic view cannot be erased behind
    /// a trait object. The frozen dataset IS the erased handle — every backend that
    /// can supply a focus graph supplies exactly this, and one that cannot supplies
    /// none, in which case the call never reaches a body at all (see
    /// `eval_expr_function`).
    ///
    /// It is supplied per QUERY, through
    /// [`QueryOptions::focus_graph`](crate::QueryOptions::focus_graph), never captured
    /// once at registration time. That is what makes a function called during round
    /// *n* of a fixpoint read round *n*'s graph: the caller passes the graph it is
    /// actually querying, so a rebuilt graph is a rebuilt argument rather than a stale
    /// capture.
    pub focus_graph: &'a Arc<RdfDataset>,
    /// The user-function call depth this invocation sits at (1 for a call from the
    /// top-level query). A callee that re-enters SPARQL evaluation propagates this
    /// through [`QueryOptions::call_depth`](crate::QueryOptions::call_depth) so a
    /// cycle that leaves and re-enters the evaluator is still bounded.
    pub depth: u32,
}

/// A dataset-aware (expression-bodied) user function body: a closure over the
/// evaluated argument values AND the calling query's focus graph (see
/// [`ExprFnCall`]).
///
/// `Ok(None)` is the SPARQL "expression error / no value" result, exactly as it is
/// for [`NativeFnBody`]; `Err` is a hard failure that aborts the query. A body that
/// cannot evaluate something MUST take one of those two exits — never a value it did
/// not compute.
pub type ExprFnBody =
    Arc<dyn Fn(&ExprFnCall<'_>) -> Result<Option<TermValue>, EvalError> + Send + Sync>;

/// A registered dataset-aware function: its closure body plus its declared arity.
///
/// There is deliberately no [`Volatility`] axis. An expression-bodied function reads
/// the focus graph and may re-enter a whole evaluator, so it is never a candidate for
/// the fork-join parallel gate (`crate::parallel` refuses it unconditionally) and a
/// declaration either way would be a distinction without a difference.
#[derive(Clone)]
pub struct ExprFunction {
    pub(crate) body: ExprFnBody,
    pub(crate) arity: Arity,
}

impl core::fmt::Debug for ExprFunction {
    /// The closure body has no `Debug` impl, so only the declared arity is shown.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ExprFunction")
            .field("arity", &self.arity)
            .finish_non_exhaustive()
    }
}

/// A native function's determinism class — the volatility axis of its descriptor
/// (after PostgreSQL's `provolatile`).
///
/// The fork-join parallel-evaluation gate is this enum's sole consumer:
/// [`Volatile`](Self::Volatile) pins the call to sequential evaluation;
/// [`Stable`](Self::Stable) (deterministic *within one query* — e.g. a frozen
/// external-index read, or pure math over its arguments) may run across workers.
/// There is deliberately no bool-typed "purity" flag: the three-way Postgres
/// vocabulary (`IMMUTABLE`/`STABLE`/`VOLATILE`) is honest about the difference
/// between "never changes" and "fixed for the lifetime of one query", and a
/// frozen-index read is the latter, not the former.
///
/// `#[non_exhaustive]`: a finer class (e.g. `Immutable`, for a future
/// const-folding pass) is addable without a breaking change — no dead variant is
/// carried now.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum Volatility {
    /// Deterministic for the lifetime of one query (a frozen external-index read,
    /// or pure math over its arguments). Safe to run across fork-join workers.
    Stable,
    /// May observe or mutate state that changes between calls within the same
    /// query (a mutable external resource, wall-clock time, RNG). Pinned to
    /// sequential evaluation.
    Volatile,
}

impl Volatility {
    /// A stable diagnostic label for this determinism class — shared by the
    /// property-function registry fingerprint (`crate::property_fn_plan::registry_fingerprint`)
    /// and every receipt that renders a [`PfDescriptor`](crate::property_fn::PfDescriptor),
    /// so both name the same class in the same words.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Volatile => "volatile",
        }
    }
}

/// A native function's declared argument arity, checked before the closure is
/// ever invoked (fail-fast: a wrong-count call never hands the host closure a
/// short or long slice).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Arity {
    /// Exactly `n` arguments.
    Exact(usize),
    /// Between `min` and `max` arguments, inclusive.
    Range {
        /// The minimum accepted argument count, inclusive.
        min: usize,
        /// The maximum accepted argument count, inclusive.
        max: usize,
    },
    /// At least `n` arguments (no upper bound).
    AtLeast(usize),
}

impl Arity {
    /// Whether `count` arguments satisfies this declared arity.
    ///
    /// `pub(crate)` rather than private: `crate::property_fn_plan`'s prepare-time
    /// walk checks a custom aggregate's supplied `AGG(<iri>, args…)` argument
    /// count against its registered [`CustomAggregate`](crate::agg_fn::CustomAggregate)'s
    /// declared [`Arity`] the same way this module checks a native function's.
    pub(crate) fn accepts(self, count: usize) -> bool {
        match self {
            Self::Exact(n) => count == n,
            Self::Range { min, max } => (min..=max).contains(&count),
            Self::AtLeast(n) => count >= n,
        }
    }

    /// A stable, machine-facing encoding of this arity —
    /// deliberately INDEPENDENT of [`Display for Arity`](#impl-Display-for-Arity)
    /// just below, which exists only for human-facing diagnostics and is free to
    /// change wording (pluralization, punctuation, phrasing) without that being a
    /// breaking change to anything.
    ///
    /// `crate::agg_fn::registry_fingerprint` folds this into a prepared plan's
    /// identity. A load-bearing `Display` impl would mean a purely cosmetic
    /// wording edit silently invalidates every previously-prepared plan's cache
    /// key (or worse, two DIFFERENT arities that happen to render the same
    /// diagnostic words would collide) — this encoding exists so that can never
    /// happen: it is a variant tag plus its numeric fields, with no wording to
    /// drift.
    pub(crate) fn stable_encoding(self) -> String {
        match self {
            Self::Exact(n) => format!("exact:{n}"),
            Self::Range { min, max } => format!("range:{min}:{max}"),
            Self::AtLeast(n) => format!("atleast:{n}"),
        }
    }
}

impl core::fmt::Display for Arity {
    /// Human-facing wording only — NOT part of any stable fingerprint or cache-key
    /// encoding. See `Arity::stable_encoding`, which
    /// `crate::agg_fn::registry_fingerprint` uses instead, for the encoding that
    /// IS load-bearing; this impl is free to change wording at any time.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Exact(n) => write!(f, "exactly {n}"),
            Self::Range { min, max } => write!(f, "{min}..={max}"),
            Self::AtLeast(n) => write!(f, "at least {n}"),
        }
    }
}

/// A registered native function: its closure body plus its declared arity and
/// volatility. See [`NativeFnBody`] for the determinism contract the closure
/// itself must uphold.
#[derive(Clone)]
pub struct NativeFunction {
    pub(crate) body: NativeFnBody,
    pub(crate) arity: Arity,
    pub(crate) volatility: Volatility,
}

impl core::fmt::Debug for NativeFunction {
    /// The closure body has no `Debug` impl, so only the declared arity and
    /// volatility are shown (the same two fields the fork-join parallel gate and
    /// the native-dispatch path `eval_native_function` consult).
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("NativeFunction")
            .field("arity", &self.arity)
            .field("volatility", &self.volatility)
            .finish_non_exhaustive()
    }
}

/// A caller-injected table of user functions, keyed by function IRI. Holds two
/// independent kinds under two separate tables — SHACL-AF SPARQL-bodied
/// ([`UserFunction`]) and native Rust-closure ([`NativeFunction`]) — sharing one
/// IRI namespace: [`Self::insert`]/[`Self::register_native`] hard-fail on a
/// cross-kind collision (see their docs) so an IRI is unambiguously one kind or
/// the other, never silently shadowed. Built once per shapes graph / host
/// configuration and borrowed into evaluation via
/// [`NativeSparqlEngine::query_with_options_view`](crate::NativeSparqlEngine::query_with_options_view)
/// (via `QueryOptions::functions`).
#[derive(Default, Clone)]
pub struct UserFunctionRegistry {
    fns: DetHashMap<String, UserFunction>,
    native: DetHashMap<String, NativeFunction>,
    exprs: DetHashMap<String, ExprFunction>,
}

impl core::fmt::Debug for UserFunctionRegistry {
    /// A [`NativeFunction`]/[`ExprFunction`] closure has no `Debug` impl, so this
    /// lists the three tables' key sets (sorted for deterministic output) rather
    /// than deriving.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut sparql_bodied: Vec<&str> = self.fns.keys().map(String::as_str).collect();
        sparql_bodied.sort_unstable();
        let mut native: Vec<&str> = self.native.keys().map(String::as_str).collect();
        native.sort_unstable();
        let mut exprs: Vec<&str> = self.exprs.keys().map(String::as_str).collect();
        exprs.sort_unstable();
        f.debug_struct("UserFunctionRegistry")
            .field("fns", &sparql_bodied)
            .field("native", &native)
            .field("exprs", &exprs)
            .finish()
    }
}

impl UserFunctionRegistry {
    /// An empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The canonical empty registry — the non-optional "no scalar functions
    /// registered" value every registry-carrying seam
    /// ([`crate::engine::QueryOptions::functions`],
    /// [`crate::eval::EvalCtx::user_functions`](crate::eval::EvalCtx),
    /// `crate::parallel::SafetyRegistries::functions`) now uses in place of the
    /// old `Option::None` spelling. Unlike
    /// [`crate::agg_fn::AggregateRegistry::EMPTY`]/[`crate::property_fn::PropertyFunctionRegistry::EMPTY`],
    /// this type carries no `RegistryId` (`crate::registry_id::RegistryId`) — a
    /// SHACL-AF/native function registry is never checked for plan identity (see
    /// `crate::engine`'s note on why `QueryOptions::functions` is deliberately
    /// excluded from `check_plan_matches_relations`) — so there is no identity
    /// question for this constant to answer either way.
    pub const EMPTY: Self = Self {
        fns: DetHashMap::with_hasher(crate::DetHasher::new()),
        native: DetHashMap::with_hasher(crate::DetHasher::new()),
        exprs: DetHashMap::with_hasher(crate::DetHasher::new()),
    };

    /// Register `func` under its `iri`. A later registration of the same IRI
    /// replaces the earlier one.
    ///
    /// # Panics
    ///
    /// Panics if `iri` is already registered as a [`NativeFunction`] or an
    /// [`ExprFunction`] — one IRI cannot be two kinds at once (a host
    /// misconfiguration, caught at registration time rather than silently shadowing
    /// one kind with another). Re-registering the same IRI with another
    /// SPARQL-bodied function is unaffected ("last write wins").
    pub fn insert(&mut self, iri: impl Into<String>, func: UserFunction) {
        let iri = iri.into();
        assert!(
            !self.native.contains_key(&iri),
            "IRI <{iri}> is already registered as a native function; cannot also register it as a SPARQL-bodied function"
        );
        assert!(
            !self.exprs.contains_key(&iri),
            "IRI <{iri}> is already registered as an expression-bodied function; cannot also register it as a SPARQL-bodied function"
        );
        self.fns.insert(iri, func);
    }

    /// Register a dataset-aware (expression-bodied) function under `iri`, with its
    /// declared calling `arity`. A later registration of the same IRI as another
    /// expression-bodied function replaces the earlier one ("last write wins").
    ///
    /// This is the registration seam SHACL 1.2 SPARQL Extensions §7.3 "Evaluation of
    /// Custom SPARQL Functions" asks an engine to use: "SPARQL engines SHOULD
    /// register a function for any SHACL instance of `sh:ListParameterExpressionFunction`
    /// from any provided shapes graph."
    ///
    /// # Panics
    ///
    /// Panics if `iri` is already registered as a SPARQL-bodied [`UserFunction`] or
    /// a [`NativeFunction`] — see [`Self::insert`]'s panic doc for the rationale
    /// (symmetric guard).
    pub fn register_expr(&mut self, iri: impl Into<String>, arity: Arity, body: ExprFnBody) {
        let iri = iri.into();
        assert!(
            !self.fns.contains_key(&iri),
            "IRI <{iri}> is already registered as a SPARQL-bodied function; cannot also register it as expression-bodied"
        );
        assert!(
            !self.native.contains_key(&iri),
            "IRI <{iri}> is already registered as a native function; cannot also register it as expression-bodied"
        );
        self.exprs.insert(iri, ExprFunction { body, arity });
    }

    /// Register a native (host-Rust closure) function under `iri`, with its
    /// declared calling `arity` and determinism `volatility`. A later
    /// registration of the same IRI as another native function replaces the
    /// earlier one ("last write wins").
    ///
    /// # Panics
    ///
    /// Panics if `iri` is already registered as a SPARQL-bodied [`UserFunction`] or
    /// an [`ExprFunction`] — see [`Self::insert`]'s panic doc for the rationale
    /// (symmetric guard).
    pub fn register_native(
        &mut self,
        iri: impl Into<String>,
        arity: Arity,
        volatility: Volatility,
        body: NativeFnBody,
    ) {
        let iri = iri.into();
        assert!(
            !self.fns.contains_key(&iri),
            "IRI <{iri}> is already registered as a SPARQL-bodied function; cannot also register it as native"
        );
        assert!(
            !self.exprs.contains_key(&iri),
            "IRI <{iri}> is already registered as an expression-bodied function; cannot also register it as native"
        );
        self.native.insert(
            iri,
            NativeFunction {
                body,
                arity,
                volatility,
            },
        );
    }

    /// Resolve a call-position IRI to its declared SPARQL-bodied function, if any.
    #[must_use]
    pub fn resolve(&self, iri: &str) -> Option<&UserFunction> {
        self.fns.get(iri)
    }

    /// Resolve a call-position IRI to its declared native function, if any.
    #[must_use]
    pub fn resolve_native(&self, iri: &str) -> Option<&NativeFunction> {
        self.native.get(iri)
    }

    /// Resolve a call-position IRI to its declared dataset-aware
    /// (expression-bodied) function, if any.
    #[must_use]
    pub fn resolve_expr(&self, iri: &str) -> Option<&ExprFunction> {
        self.exprs.get(iri)
    }

    /// Whether the registry holds no functions of any kind (the common case: no
    /// `sh:SPARQLFunction` declared and nothing registered, so evaluation carries
    /// no registry at all).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fns.is_empty() && self.native.is_empty() && self.exprs.is_empty()
    }

    /// The number of declared functions across all three kinds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.fns.len() + self.native.len() + self.exprs.len()
    }
}

/// Whether `value`'s node kind satisfies `nk`.
fn matches_node_kind(value: &TermValue, nk: NodeKind) -> bool {
    let (is_iri, is_blank, is_literal) = match value {
        TermValue::Iri(_) => (true, false, false),
        TermValue::Blank { .. } => (false, true, false),
        TermValue::Literal { .. } => (false, false, true),
        // A triple term is none of the three simple kinds.
        TermValue::Triple { .. } => (false, false, false),
    };
    match nk {
        NodeKind::Iri => is_iri,
        NodeKind::BlankNode => is_blank,
        NodeKind::Literal => is_literal,
        NodeKind::BlankNodeOrIri => is_blank || is_iri,
        NodeKind::BlankNodeOrLiteral => is_blank || is_literal,
        NodeKind::IriOrLiteral => is_iri || is_literal,
    }
}

/// Execute a resolved SHACL-AF function call: arity- and type-check the arguments,
/// bind them to the parameter variables, evaluate the body in a recursion-bounded
/// child context, and extract the single return value (`Ok(None)` = no value).
///
/// `args` are the already-evaluated argument values in call order (a `None` cell is
/// an unbound argument, which leaves that parameter variable unbound). The result is
/// a dataset-independent [`TermValue`]; the caller interns it into the parent
/// context.
///
/// # Errors
///
/// [`EvalError::Function`] on an arity or type-constraint violation, or on exceeding the
/// user-function recursion bound during an ungoverned execution; propagates body evaluation
/// errors. During a governed execution, fuel and depth exhaustion are recorded on the
/// expression truncation channel and return `Ok(None)` here so they cannot masquerade as a
/// function failure.
pub(crate) fn eval_user_function<D: DatasetView + Sync>(
    func: &UserFunction,
    iri: &str,
    args: &[Option<TermValue>],
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Option<TermValue>, EvalError> {
    if args.len() < func.required || args.len() > func.params.len() {
        return Err(EvalError::function(format!(
            "SHACL-AF function <{iri}> expects {}..={} argument(s), got {}",
            func.required,
            func.params.len(),
            args.len()
        )));
    }

    // Bind each supplied argument to its parameter variable, type-checking as we go.
    // A mandatory parameter with an unbound (`None`) argument yields no result node
    // (SHACL-AF §5.2/§9.5): the function is not evaluated at all. An unbound OPTIONAL
    // argument simply leaves that parameter variable unbound (pre-binding semantics).
    let mut substitutions: Vec<(String, TermValue)> = Vec::with_capacity(args.len());
    for (idx, (arg, param)) in args.iter().zip(&func.params).enumerate() {
        match arg {
            Some(value) => {
                param
                    .constraint
                    .check(iri, &format!("parameter ?{}", param.var), value)?;
                substitutions.push((param.var.clone(), value.clone()));
            }
            None if idx < func.required => return Ok(None),
            None => {}
        }
    }

    // The `user-function-invocation` charge point. Charged once per invocation that
    // actually reaches a body — an arity or type-constraint refusal above evaluated
    // nothing, and charging for work that never happened would make the schedule a
    // description of the query text rather than of the execution. A function body's own
    // evaluation then charges through the shared state, so a query cannot evade its
    // ceiling by moving work into a function.
    if let Err(tripped) = ctx.charge(crate::governor::ChargePoint::UserFunctionInvocation) {
        ctx.expression_barrier.record(tripped);
        return Ok(None);
    }

    // Recursion-bounded child context (guards mutually-recursive functions).
    let Some(mut child) = ctx.child_for_user_fn()? else {
        return Ok(None);
    };
    let substituted = crate::substitute::apply_substitutions((*func.body).clone(), &substitutions)
        .map_err(|d| EvalError::function(d.to_string()))?;
    let outcome = match evaluate_query_evaluated(&substituted, &mut child)? {
        EvaluatedOutcome::Complete(outcome) => outcome,
        EvaluatedOutcome::Truncated { certificate, .. } => {
            // A function call is an expression position, so a truncated body makes the
            // call's value unknowable rather than merely partial: an `ASK` body over a
            // partial answer can report `false` where the truth is `true`, and a `SELECT`
            // body can report a different first row. The trip is recorded on the shared
            // expression barrier, which makes the operator that owns the calling
            // expression withhold every row it produced — the same treatment an `EXISTS`
            // gets, for the same reason.
            child.expression_barrier.record(certificate.tripped());
            ctx.bnode_counter = child.bnode_counter;
            ctx.rng_state = child.rng_state;
            return Ok(None);
        }
    };

    let result: Option<TermValue> = match (func.kind, outcome) {
        (UserFnBody::Ask, Outcome::Boolean(value)) => Some(TermValue::typed_literal(
            if value { "true" } else { "false" },
            "http://www.w3.org/2001/XMLSchema#boolean",
        )),
        (UserFnBody::Select, Outcome::Solutions(seq)) => {
            let (variables, rows) = materialize_solutions(&seq, &child);
            // A SHACL-AF function SELECT body yields a single result variable; a
            // multi-projection body has no well-defined return value.
            if variables.len() != 1 {
                return Err(EvalError::function(format!(
                    "SHACL-AF function <{iri}> SELECT body must project exactly one variable, got {}",
                    variables.len()
                )));
            }
            // The single projected value of the first solution row; an empty
            // result set is "no value".
            rows.into_iter()
                .next()
                .and_then(|row| row.into_iter().next().flatten())
        }
        // The declaration parser pairs `kind` with the matching body form, so a
        // mismatch is an internal invariant violation, not user input.
        (kind, outcome) => {
            return Err(EvalError::internal(format!(
                "SHACL-AF function <{iri}> body kind {kind:?} produced {outcome:?}"
            )));
        }
    };

    // Merge the child's minted identity / entropy / constructed state back into
    // the parent so it survives the return boundary: body-minted blanks stay
    // globally unique across calls, RAND()/UUID()/STRUUID() advance the stream
    // rather than replay it, and rdf:List quads constructed by listSlice/
    // listConcat remain reachable in the enclosing query's results.
    ctx.bnode_counter = child.bnode_counter;
    ctx.rng_state = child.rng_state;
    ctx.constructed.append(&mut child.constructed);

    // `sh:returnType` is informational (SHACL-AF §5.3): it documents/casts the
    // return and MAY be a class IRI, not a literal datatype. Enforcing it as a
    // runtime datatype constraint would spuriously reject IRI/blank-node returns,
    // so it is retained on `UserFunction` for callers but NOT enforced here.
    Ok(result)
}

/// Execute a resolved native (host-Rust closure) function call: arity-check the
/// arguments, then invoke the closure with the bound values.
///
/// `args` are the already-evaluated argument values in call order (a `None` cell
/// is an unbound argument). A native function declares no per-parameter
/// optionality: unlike the SPARQL-bodied path's "leading required parameters"
/// split, **any** unbound argument yields no result node at all (`Ok(None)`),
/// since the closure has no way to observe which parameter was left unbound. The
/// result is a dataset-independent [`TermValue`]; the caller interns it into the
/// parent context.
///
/// # Errors
///
/// [`EvalError::Function`] on an arity violation, on a panic inside the closure
/// (converted to a fixed, payload-free error so the message is identical
/// regardless of which worker thread panicked — mirrors the `native-codec-panic`
/// guard in `purrdf_rdf::native_codecs::parse`), or propagated straight through
/// from the closure's own `Err`.
pub(crate) fn eval_native_function(
    native: &NativeFunction,
    iri: &str,
    args: &[Option<TermValue>],
) -> Result<Option<TermValue>, EvalError> {
    // Fail-fast: a wrong-count call never reaches the host closure with a short
    // or long slice.
    if !native.arity.accepts(args.len()) {
        return Err(EvalError::function(format!(
            "native function <{iri}> expects {} argument(s), got {}",
            native.arity,
            args.len()
        )));
    }

    // A native function declares no per-parameter optionality: any unbound
    // argument yields no result node rather than being handed to the closure.
    if args.iter().any(Option::is_none) {
        return Ok(None);
    }
    // Lend the closure borrows into the caller's argument buffer — no per-call
    // deep clone of the (heap-string-owning) TermValues on the scoring hot path.
    let values: Vec<&TermValue> = args
        .iter()
        .map(|arg| arg.as_ref().expect("checked all-Some above"))
        .collect();

    // Guard the host closure with catch_unwind: a panicking closure (dim
    // mismatch, unwrap, OOB index) must not abort a rayon worker or otherwise
    // surface nondeterministically. The error message is fixed and
    // payload-free so it is identical no matter which worker panicked. Mirrors
    // `purrdf_rdf::native_codecs::parse`'s `native-codec-panic` guard.
    match catch_unwind(AssertUnwindSafe(|| (native.body)(&values))) {
        Ok(inner_result) => inner_result.map(Some),
        Err(_) => Err(EvalError::function(format!(
            "native function <{iri}> panicked"
        ))),
    }
}

/// Execute a resolved dataset-aware (expression-bodied) function call: arity-check
/// the arguments, obtain the calling query's focus graph, bound the re-entrancy
/// depth, then invoke the closure.
///
/// # Why this cannot reuse the native path
///
/// A [`NativeFunction`] is a pure function of its argument values, so the evaluator
/// hands it nothing else. An expression-bodied function is a SHACL node expression,
/// and `evalExpr(expr, focusGraph, focusNode, scope)` is a function OF a graph — so
/// the graph must travel with the call, and a call that has no graph to evaluate
/// against cannot be answered at all.
///
/// # Fail-closed, twice
///
/// * **No focus graph.** When the calling query supplied none
///   ([`QueryOptions::focus_graph`](crate::QueryOptions::focus_graph) left `None`) the
///   call is a hard [`EvalError::Function`], never `Ok(None)`. `Ok(None)` is SPARQL's
///   "this expression had an error, treat the value as unbound", and using it here
///   would make a function that could not be evaluated indistinguishable from one
///   that evaluated to nothing — a silent wrong answer.
/// * **Re-entrancy.** A node-expression body can call back into query evaluation,
///   which can call this function again. The chain is bounded by the SAME
///   [`MAX_UDF_DEPTH`](crate::eval::MAX_UDF_DEPTH) ceiling the SPARQL-bodied path
///   uses, and the depth is handed to the callee so a callee that leaves and re-enters
///   the evaluator ([`QueryOptions::call_depth`](crate::QueryOptions::call_depth))
///   keeps counting rather than restarting at zero. Unbounded native recursion in Rust
///   ABORTS the process, which no caller can catch, so this bound is the difference
///   between an error and a crash.
///
/// # Errors
///
/// [`EvalError::Function`] on an arity violation, an absent focus graph, a depth-bound
/// breach, or a panic inside the closure (converted to a fixed, payload-free error, as
/// on the native path); propagates the closure's own `Err` unchanged.
pub(crate) fn eval_expr_function<D: DatasetView + Sync>(
    func: &ExprFunction,
    iri: &str,
    args: &[Option<TermValue>],
    ctx: &EvalCtx<'_, D>,
) -> Result<Option<TermValue>, EvalError> {
    if !func.arity.accepts(args.len()) {
        return Err(EvalError::function(format!(
            "expression-bodied function <{iri}> expects {} argument(s), got {}",
            func.arity,
            args.len()
        )));
    }
    let Some(focus_graph) = ctx.focus_graph else {
        return Err(EvalError::function(format!(
            "expression-bodied function <{iri}> needs the focus graph its body is evaluated \
             against, and this query supplied none (QueryOptions::focus_graph); the call is \
             refused rather than answered from a graph it never read"
        )));
    };
    let depth = ctx.udf_depth.saturating_add(1);
    if depth > crate::eval::MAX_UDF_DEPTH {
        return Err(EvalError::function(format!(
            "expression-bodied function <{iri}> recursion exceeded the depth bound of {}",
            crate::eval::MAX_UDF_DEPTH
        )));
    }
    let call = ExprFnCall {
        iri,
        args,
        focus_graph,
        depth,
    };
    // The same `catch_unwind` contract the native path documents: a panicking host
    // closure must not abort a worker or surface nondeterministically, and the
    // message is fixed and payload-free so it does not depend on which thread ran.
    match catch_unwind(AssertUnwindSafe(|| (func.body)(&call))) {
        Ok(inner_result) => inner_result,
        Err(_) => Err(EvalError::function(format!(
            "expression-bodied function <{iri}> panicked"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::sync::Arc;

    use purrdf_core::{
        RdfDataset, RdfDatasetBuilder, RdfLiteral, ResourceDimension, SparqlRequest, SparqlResult,
        TermValue, TrippedGovernor,
    };
    use purrdf_sparql_algebra::SparqlParser;

    use crate::{
        GovernedOutcome, GovernorState, NativeSparqlEngine, PartialAnswers, QueryGovernors,
        QueryOptions,
    };

    const EX_INC: &str = "http://example.org/ns#inc";
    const EX_EVEN: &str = "http://example.org/ns#isEven";
    const EX_LOOP: &str = "http://example.org/ns#loop";
    const EX_NATIVE_INC: &str = "http://example.org/ns#nativeInc";
    const EX_NATIVE_ERR: &str = "http://example.org/ns#nativeErr";
    const EX_NATIVE_PANIC: &str = "http://example.org/ns#nativePanic";
    const EX_NATIVE_ARITY: &str = "http://example.org/ns#nativeArity";
    const EX_NATIVE_UNBOUND: &str = "http://example.org/ns#nativeUnbound";
    const EX_NATIVE_COLLIDE: &str = "http://example.org/ns#nativeCollide";
    const EX_SPARQL_ONLY: &str = "http://example.org/ns#sparqlOnly";
    const EX_NATIVE_ONLY: &str = "http://example.org/ns#nativeOnly";
    const EX_SCORE: &str = "http://example.org/ns#score";
    const EX_SCORE_NAN: &str = "http://example.org/ns#scoreNan";
    const EX_VAL: &str = "http://example.org/ns#val";
    const EX_SUBJECT_PREFIX: &str = "http://example.org/ns#s";
    const EX_EXPR_COUNT: &str = "http://example.org/ns#exprCount";

    const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
    const XSD_DOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";

    /// A native closure that adds one to its sole integer-literal argument.
    fn inc_native_body() -> NativeFnBody {
        Arc::new(|args: &[&TermValue]| {
            let TermValue::Literal { lexical_form, .. } = args[0] else {
                return Err(EvalError::function("expected a literal argument"));
            };
            let n: i64 = lexical_form
                .parse()
                .map_err(|_| EvalError::function("argument is not an integer"))?;
            Ok(TermValue::typed_literal((n + 1).to_string(), XSD_INTEGER))
        })
    }

    fn empty_dataset() -> Arc<RdfDataset> {
        RdfDatasetBuilder::new().freeze().expect("freeze")
    }

    /// A two-triple dataset, so a dataset-aware closure that answers the graph's
    /// size has a size that could not have come from anywhere else.
    fn dataset_with_two_triples() -> Arc<RdfDataset> {
        let mut builder = RdfDatasetBuilder::new();
        let predicate = builder.intern_iri(EX_VAL);
        let object = builder.intern_literal(RdfLiteral::typed("1", XSD_INTEGER));
        for local in ["a", "b"] {
            let subject = builder.intern_iri(&format!("{EX_SUBJECT_PREFIX}{local}"));
            builder.push_quad(subject, predicate, object, None);
        }
        builder.freeze().expect("freeze")
    }

    /// A trivial SPARQL-bodied function, for the cross-kind collision guard.
    fn select_body_function() -> UserFunction {
        UserFunction {
            params: Vec::new(),
            required: 0,
            body: parse("SELECT (1 AS ?result) WHERE {}"),
            kind: UserFnBody::Select,
            return_constraint: TypeConstraint::default(),
        }
    }

    /// A native closure that parses its literal argument as `f64` and divides it
    /// by `divisor`, returning an `xsd:double`. Pure math over the argument
    /// value, so it is honestly `Volatility::Stable`.
    fn ratio_score_native_body(divisor: f64) -> NativeFnBody {
        Arc::new(move |args: &[&TermValue]| {
            let TermValue::Literal { lexical_form, .. } = args[0] else {
                return Err(EvalError::function("expected a literal argument"));
            };
            let n: f64 = lexical_form
                .parse()
                .map_err(|_| EvalError::function("argument is not numeric"))?;
            Ok(TermValue::typed_literal(
                (n / divisor).to_string(),
                XSD_DOUBLE,
            ))
        })
    }

    /// A native closure returning `xsd:double` `NaN` for a non-positive argument
    /// and `arg / 5.0` otherwise — used to exercise `ORDER BY`'s handling of
    /// `NaN` scores.
    fn nan_score_native_body() -> NativeFnBody {
        Arc::new(|args: &[&TermValue]| {
            let TermValue::Literal { lexical_form, .. } = args[0] else {
                return Err(EvalError::function("expected a literal argument"));
            };
            let n: f64 = lexical_form
                .parse()
                .map_err(|_| EvalError::function("argument is not numeric"))?;
            let score = if n <= 0.0 { f64::NAN } else { n / 5.0 };
            Ok(TermValue::typed_literal(score.to_string(), XSD_DOUBLE))
        })
    }

    /// A dataset of `n` subjects `ex:s1..ex:sN`, each with an integer `ex:val`
    /// property `1..=n`.
    fn scored_dataset(n: usize) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let val_pred = b.intern_iri(EX_VAL);
        for i in 1..=n {
            let s = b.intern_iri(&format!("{EX_SUBJECT_PREFIX}{i}"));
            let v = b.intern_literal(RdfLiteral::typed(i.to_string(), XSD_INTEGER.to_owned()));
            b.push_quad(s, val_pred, v, None);
        }
        b.freeze().expect("freeze")
    }

    /// Run `query` and collect the bound `?s` (sole projected column) IRIs of
    /// every solution row as an unordered set.
    fn run_subject_set(
        ds: &Arc<RdfDataset>,
        query: &str,
        registry: &UserFunctionRegistry,
    ) -> BTreeSet<String> {
        run_subject_rows(ds, query, registry).into_iter().collect()
    }

    /// Run `query` and collect the bound `?s` (sole projected column) IRIs of
    /// every solution row in result order.
    fn run_subject_rows(
        ds: &Arc<RdfDataset>,
        query: &str,
        registry: &UserFunctionRegistry,
    ) -> Vec<String> {
        let result = NativeSparqlEngine::new()
            .query_with_options_view(
                ds,
                SparqlRequest {
                    query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions {
                    functions: registry,
                    ..QueryOptions::EMPTY
                },
            )
            .expect("query");
        match result {
            SparqlResult::Solutions { rows, .. } => rows
                .into_iter()
                .map(|row| match row[0].as_ref().expect("bound subject") {
                    TermValue::Iri(iri) => iri.clone(),
                    other => panic!("expected an IRI subject, got {other:?}"),
                })
                .collect(),
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    fn parse(body: &str) -> Arc<Query> {
        Arc::new(
            SparqlParser::new()
                .parse_query(body)
                .expect("parse function body"),
        )
    }

    fn int_param(var: &str) -> UserFnParam {
        UserFnParam {
            var: var.to_owned(),
            constraint: TypeConstraint::default(),
        }
    }

    /// A SELECT-bodied function `inc(?n) = ?n + 1` returns the projected value.
    #[test]
    fn select_body_returns_projected_value() {
        let mut registry = UserFunctionRegistry::new();
        registry.insert(
            EX_INC,
            UserFunction {
                params: vec![int_param("n")],
                required: 1,
                body: parse("SELECT ((?n + 1) AS ?result) WHERE {}"),
                kind: UserFnBody::Select,
                return_constraint: TypeConstraint::default(),
            },
        );
        let ds = empty_dataset();
        let query = format!("SELECT ((<{EX_INC}>(41)) AS ?v) WHERE {{}}");
        let result = NativeSparqlEngine::new()
            .query_with_options_view(
                &ds,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions {
                    functions: &registry,
                    ..QueryOptions::EMPTY
                },
            )
            .expect("query");
        match result {
            SparqlResult::Solutions { rows, .. } => {
                let cell = rows[0][0].as_ref().expect("bound result");
                assert!(
                    matches!(cell, TermValue::Literal { lexical_form, .. } if lexical_form == "42"),
                    "expected 42, got {cell:?}"
                );
            }
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    /// An ASK-bodied function returns an `xsd:boolean`.
    #[test]
    fn ask_body_returns_boolean() {
        let mut registry = UserFunctionRegistry::new();
        registry.insert(
            EX_EVEN,
            UserFunction {
                params: vec![int_param("n")],
                required: 1,
                body: parse("ASK { FILTER(?n / 2 = FLOOR(?n / 2)) }"),
                kind: UserFnBody::Ask,
                return_constraint: TypeConstraint::default(),
            },
        );
        let ds = empty_dataset();
        let run = |arg: i32| -> String {
            let query = format!("SELECT ((<{EX_EVEN}>({arg})) AS ?v) WHERE {{}}");
            match NativeSparqlEngine::new()
                .query_with_options_view(
                    &ds,
                    SparqlRequest {
                        query: &query,
                        base_iri: None,
                        substitutions: &[],
                    },
                    QueryOptions {
                        functions: &registry,
                        ..QueryOptions::EMPTY
                    },
                )
                .expect("query")
            {
                SparqlResult::Solutions { rows, .. } => match rows[0][0].as_ref().expect("bound") {
                    TermValue::Literal { lexical_form, .. } => lexical_form.clone(),
                    other => panic!("expected literal, got {other:?}"),
                },
                other => panic!("expected solutions, got {other:?}"),
            }
        };
        assert_eq!(run(4), "true");
        assert_eq!(run(5), "false");
    }

    /// SHACL-AF §5.2/§9.5: a call missing a mandatory argument yields no result
    /// node. The body here ignores its parameter and always succeeds, so only the
    /// mandatory-argument guard (not an unbound `?n`) can suppress the value.
    #[test]
    fn unbound_mandatory_parameter_yields_no_value() {
        let mut registry = UserFunctionRegistry::new();
        registry.insert(
            EX_EVEN,
            UserFunction {
                params: vec![int_param("n")],
                required: 1,
                body: parse("ASK {}"),
                kind: UserFnBody::Ask,
                return_constraint: TypeConstraint::default(),
            },
        );
        let ds = empty_dataset();
        // `?missing` is never bound, so the sole mandatory argument is unbound.
        let query = format!("SELECT ((<{EX_EVEN}>(?missing)) AS ?v) WHERE {{}}");
        let result = NativeSparqlEngine::new()
            .query_with_options_view(
                &ds,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions {
                    functions: &registry,
                    ..QueryOptions::EMPTY
                },
            )
            .expect("query");
        match result {
            SparqlResult::Solutions { rows, .. } => {
                assert!(
                    rows[0][0].is_none(),
                    "unbound mandatory argument must yield no value, got {:?}",
                    rows[0][0]
                );
            }
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    /// A call with the wrong argument count is a hard [`EvalError::Function`].
    #[test]
    fn wrong_arity_is_a_hard_error() {
        let mut registry = UserFunctionRegistry::new();
        registry.insert(
            EX_INC,
            UserFunction {
                params: vec![int_param("n")],
                required: 1,
                body: parse("SELECT ((?n + 1) AS ?result) WHERE {}"),
                kind: UserFnBody::Select,
                return_constraint: TypeConstraint::default(),
            },
        );
        let ds = empty_dataset();
        // Two arguments to a one-parameter function.
        let query = format!("SELECT ((<{EX_INC}>(1, 2)) AS ?v) WHERE {{}}");
        let err = NativeSparqlEngine::new()
            .query_with_options_view(
                &ds,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions {
                    functions: &registry,
                    ..QueryOptions::EMPTY
                },
            )
            .expect_err("arity mismatch must fail");
        assert!(
            err.to_string().contains("expects"),
            "expected arity error, got {err}"
        );
    }

    /// State minted in a function body is merged back into the parent context. Two
    /// calls of a RAND()-bodied function within ONE expression share the same
    /// `&mut EvalCtx` sequentially, so the merged-back rng_state advances and the
    /// two results differ (`= ` is false); without the merge-back they would
    /// replay the identical value and compare equal.
    #[test]
    fn function_body_state_is_merged_back() {
        let mut registry = UserFunctionRegistry::new();
        registry.insert(
            EX_INC,
            UserFunction {
                params: vec![],
                required: 0,
                body: parse("SELECT (RAND() AS ?result) WHERE {}"),
                kind: UserFnBody::Select,
                return_constraint: TypeConstraint::default(),
            },
        );
        let ds = empty_dataset();
        let query = format!("SELECT ((<{EX_INC}>() = <{EX_INC}>()) AS ?eq) WHERE {{}}");
        let result = NativeSparqlEngine::new()
            .query_with_options_view(
                &ds,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions {
                    functions: &registry,
                    ..QueryOptions::EMPTY
                },
            )
            .expect("query");
        match result {
            SparqlResult::Solutions { rows, .. } => {
                let eq = rows[0][0].as_ref().expect("eq bound");
                assert!(
                    matches!(eq, TermValue::Literal { lexical_form, .. } if lexical_form == "false"),
                    "two RAND() calls sharing a merged context must differ (eq=false), got {eq:?}"
                );
            }
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    /// Calling a function IRI that is neither a registered `sh:SPARQLFunction` nor
    /// an XSD constructor is a hard error, not a silent unbound.
    #[test]
    fn undefined_function_call_is_a_hard_error() {
        let registry = UserFunctionRegistry::new();
        let ds = empty_dataset();
        let query = "SELECT ((<http://example.org/ns#nope>(1)) AS ?v) WHERE {}".to_owned();
        let err = NativeSparqlEngine::new()
            .query_with_options_view(
                &ds,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions {
                    functions: &registry,
                    ..QueryOptions::EMPTY
                },
            )
            .expect_err("undefined function must fail");
        assert!(
            err.to_string().contains("custom SPARQL function"),
            "expected undefined-function error, got {err}"
        );
    }

    /// An argument violating a parameter's `sh:datatype` is a hard error.
    #[test]
    fn parameter_datatype_violation_is_a_hard_error() {
        let mut registry = UserFunctionRegistry::new();
        registry.insert(
            EX_INC,
            UserFunction {
                params: vec![UserFnParam {
                    var: "n".to_owned(),
                    constraint: TypeConstraint {
                        datatype: Some("http://www.w3.org/2001/XMLSchema#integer".to_owned()),
                        node_kind: None,
                    },
                }],
                required: 1,
                body: parse("SELECT ((?n) AS ?result) WHERE {}"),
                kind: UserFnBody::Select,
                return_constraint: TypeConstraint::default(),
            },
        );
        let ds = empty_dataset();
        // The sole parameter requires xsd:integer; pass a string.
        let query = format!("SELECT ((<{EX_INC}>(\"hello\")) AS ?v) WHERE {{}}");
        let err = NativeSparqlEngine::new()
            .query_with_options_view(
                &ds,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions {
                    functions: &registry,
                    ..QueryOptions::EMPTY
                },
            )
            .expect_err("parameter datatype violation must fail");
        assert!(
            err.to_string().contains("datatype") || err.to_string().contains("parameter"),
            "expected parameter type error, got {err}"
        );
    }

    /// A SELECT body projecting more than one variable has no well-defined return
    /// value and is a hard error.
    #[test]
    fn multi_projection_select_body_is_a_hard_error() {
        let mut registry = UserFunctionRegistry::new();
        registry.insert(
            EX_INC,
            UserFunction {
                params: vec![int_param("n")],
                required: 1,
                body: parse("SELECT ((?n + 1) AS ?a) ((?n + 2) AS ?b) WHERE {}"),
                kind: UserFnBody::Select,
                return_constraint: TypeConstraint::default(),
            },
        );
        let ds = empty_dataset();
        let query = format!("SELECT ((<{EX_INC}>(1)) AS ?v) WHERE {{}}");
        let err = NativeSparqlEngine::new()
            .query_with_options_view(
                &ds,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions {
                    functions: &registry,
                    ..QueryOptions::EMPTY
                },
            )
            .expect_err("multi-projection body must fail");
        assert!(
            err.to_string().contains("exactly one variable"),
            "expected projection-arity error, got {err}"
        );
    }

    /// A self-recursive function fails closed at the depth bound rather than
    /// overflowing the stack.
    #[test]
    fn unbounded_recursion_fails_closed() {
        let mut registry = UserFunctionRegistry::new();
        // loop(?n) calls loop(?n) — a non-terminating self-recursion.
        registry.insert(
            EX_LOOP,
            UserFunction {
                params: vec![int_param("n")],
                required: 1,
                body: parse(&format!("SELECT ((<{EX_LOOP}>(?n)) AS ?result) WHERE {{}}")),
                kind: UserFnBody::Select,
                return_constraint: TypeConstraint::default(),
            },
        );
        let ds = empty_dataset();
        let query = format!("SELECT ((<{EX_LOOP}>(1)) AS ?v) WHERE {{}}");
        let err = NativeSparqlEngine::new()
            .query_with_options_view(
                &ds,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions {
                    functions: &registry,
                    ..QueryOptions::EMPTY
                },
            )
            .expect_err("runaway recursion must fail");
        assert!(
            err.to_string().contains("recursion"),
            "expected recursion-bound error, got {err}"
        );
    }

    #[test]
    fn fuel_exhaustion_at_the_invocation_boundary_is_an_expression_trip() {
        let function = UserFunction {
            params: Vec::new(),
            required: 0,
            body: parse("SELECT (1 AS ?result) WHERE {}"),
            kind: UserFnBody::Select,
            return_constraint: TypeConstraint::default(),
        };
        let dataset = empty_dataset();
        let governors = QueryGovernors::UNBOUNDED.with_fuel(0);
        let state = Arc::new(GovernorState::new(&governors));
        let mut ctx = EvalCtx::new(&*dataset).with_governors(Arc::clone(&state));

        let value = eval_user_function(&function, EX_INC, &[], &mut ctx)
            .expect("a governor trip is not a function error");
        assert_eq!(value, None, "the refused body produces no expression value");
        let expected = TrippedGovernor::Budget {
            dimension: ResourceDimension::Fuel,
            limit: 0,
            consumed: 1,
        };
        assert_eq!(state.evidence().tripped, Some(expected));
        assert_eq!(ctx.expression_barrier.observed(), Some(expected));
    }

    #[test]
    fn governed_recursion_depth_is_typed_exhaustion_not_a_function_error() {
        let mut registry = UserFunctionRegistry::new();
        registry.insert(
            EX_LOOP,
            UserFunction {
                params: vec![int_param("n")],
                required: 1,
                body: parse(&format!("SELECT ((<{EX_LOOP}>(?n)) AS ?result) WHERE {{}}")),
                kind: UserFnBody::Select,
                return_constraint: TypeConstraint::default(),
            },
        );
        let dataset = empty_dataset();
        let query = format!("SELECT ((<{EX_LOOP}>(1)) AS ?v) WHERE {{}}");
        let state = Arc::new(GovernorState::new(&QueryGovernors::METERED));
        let outcome = NativeSparqlEngine::new()
            .query_governed_in_operation(
                &*dataset,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions {
                    functions: &registry,
                    ..QueryOptions::EMPTY
                },
                &state,
            )
            .expect("a governed recursion ceiling is an outcome");

        let GovernedOutcome::BudgetExhausted(exhausted) = outcome else {
            panic!("the fixed recursion ceiling must stop this call");
        };
        assert_eq!(
            exhausted.tripped,
            TrippedGovernor::Budget {
                dimension: ResourceDimension::UdfDepth,
                limit: u64::from(crate::eval::MAX_UDF_DEPTH),
                consumed: u64::from(crate::eval::MAX_UDF_DEPTH) + 1,
            }
        );
        let PartialAnswers::Unknown(barrier) = exhausted.partial else {
            panic!("a function expression stopped before its value was knowable");
        };
        assert_eq!(barrier.operator(), "Extend");
    }

    /// `sh:returnType` is informational (SHACL-AF §5.3) and MAY be a class IRI, so
    /// it is NOT enforced at runtime: a function returning an IRI is accepted even
    /// when its declared return type is a class rather than a literal datatype.
    /// (The pre-fix code enforced it as a datatype and wrongly hard-failed this.)
    #[test]
    fn return_type_is_informational_not_enforced() {
        let mut registry = UserFunctionRegistry::new();
        registry.insert(
            EX_INC,
            UserFunction {
                params: vec![int_param("n")],
                required: 1,
                // Returns an IRI; declared return type is a class (rdfs:Resource).
                body: parse("SELECT (<http://example.org/ns#thing> AS ?result) WHERE {}"),
                kind: UserFnBody::Select,
                return_constraint: TypeConstraint {
                    datatype: Some("http://www.w3.org/2000/01/rdf-schema#Resource".to_owned()),
                    node_kind: None,
                },
            },
        );
        let ds = empty_dataset();
        let query = format!("SELECT ((<{EX_INC}>(1)) AS ?v) WHERE {{}}");
        let result = NativeSparqlEngine::new()
            .query_with_options_view(
                &ds,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions {
                    functions: &registry,
                    ..QueryOptions::EMPTY
                },
            )
            .expect("class-typed return must be accepted, not rejected");
        match result {
            SparqlResult::Solutions { rows, .. } => {
                assert!(
                    rows[0][0].is_some(),
                    "class-typed IRI return must be accepted and returned, got {:?}",
                    rows[0][0]
                );
            }
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    /// A registered native `Stable` closure is invoked from a SELECT projection
    /// and its return value is interned and reported like any other.
    #[test]
    fn native_function_returns_value() {
        let mut registry = UserFunctionRegistry::new();
        registry.register_native(
            EX_NATIVE_INC,
            Arity::Exact(1),
            Volatility::Stable,
            inc_native_body(),
        );
        let ds = empty_dataset();
        let query = format!("SELECT ((<{EX_NATIVE_INC}>(41)) AS ?v) WHERE {{}}");
        let result = NativeSparqlEngine::new()
            .query_with_options_view(
                &ds,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions {
                    functions: &registry,
                    ..QueryOptions::EMPTY
                },
            )
            .expect("query");
        match result {
            SparqlResult::Solutions { rows, .. } => {
                let cell = rows[0][0].as_ref().expect("bound result");
                assert!(
                    matches!(cell, TermValue::Literal { lexical_form, .. } if lexical_form == "42"),
                    "expected 42, got {cell:?}"
                );
            }
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    /// A dataset-aware function is handed the graph the query is running over, and
    /// can answer FROM it — the capability that distinguishes it from the native
    /// kind, which sees only argument values.
    #[test]
    fn expr_function_receives_the_query_s_focus_graph() {
        let mut registry = UserFunctionRegistry::new();
        registry.register_expr(
            EX_EXPR_COUNT,
            Arity::Exact(0),
            Arc::new(|call: &ExprFnCall<'_>| {
                // Answer the graph's own size, which is only knowable from the graph.
                let count = call.focus_graph.quads().count();
                Ok(Some(TermValue::typed_literal(
                    count.to_string(),
                    XSD_INTEGER,
                )))
            }),
        );
        let ds = dataset_with_two_triples();
        let query = format!("SELECT ((<{EX_EXPR_COUNT}>()) AS ?v) WHERE {{}}");
        let result = NativeSparqlEngine::new()
            .query_with_options_view(
                &ds,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions {
                    functions: &registry,
                    focus_graph: Some(&ds),
                    ..QueryOptions::EMPTY
                },
            )
            .expect("query");
        match result {
            SparqlResult::Solutions { rows, .. } => {
                let cell = rows[0][0].as_ref().expect("bound result");
                assert!(
                    matches!(cell, TermValue::Literal { lexical_form, .. } if lexical_form == "2"),
                    "the closure read the supplied focus graph, got {cell:?}"
                );
            }
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    /// Calling a dataset-aware function with no focus graph supplied is a hard
    /// error, never an unbound value.
    ///
    /// An unbound result is SPARQL's "this expression errored", and using it here
    /// would make "the function was never evaluated" indistinguishable from "the
    /// function looked and found nothing".
    #[test]
    fn expr_function_without_a_focus_graph_is_a_hard_error() {
        let mut registry = UserFunctionRegistry::new();
        registry.register_expr(
            EX_EXPR_COUNT,
            Arity::Exact(0),
            Arc::new(|_call: &ExprFnCall<'_>| {
                panic!("the body must never run without a focus graph")
            }),
        );
        let ds = empty_dataset();
        let query = format!("SELECT ((<{EX_EXPR_COUNT}>()) AS ?v) WHERE {{}}");
        let err = NativeSparqlEngine::new()
            .query_with_options_view(
                &ds,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions {
                    functions: &registry,
                    ..QueryOptions::EMPTY
                },
            )
            .expect_err("a call with no focus graph must be refused");
        assert!(err.to_string().contains("focus graph"), "got: {err}");
    }

    /// The call depth seeded on the query reaches the callee, and the ceiling is
    /// enforced before the body runs — so a callee that re-enters the evaluator with
    /// its own depth cannot recurse without bound.
    #[test]
    fn expr_function_depth_is_seeded_and_bounded() {
        let mut registry = UserFunctionRegistry::new();
        registry.register_expr(
            EX_EXPR_COUNT,
            Arity::Exact(0),
            Arc::new(|call: &ExprFnCall<'_>| {
                Ok(Some(TermValue::typed_literal(
                    call.depth.to_string(),
                    XSD_INTEGER,
                )))
            }),
        );
        let ds = empty_dataset();
        let query = format!("SELECT ((<{EX_EXPR_COUNT}>()) AS ?v) WHERE {{}}");
        let run = |call_depth: u32| {
            NativeSparqlEngine::new().query_with_options_view(
                &ds,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions {
                    functions: &registry,
                    focus_graph: Some(&ds),
                    call_depth,
                    ..QueryOptions::EMPTY
                },
            )
        };
        match run(7).expect("query") {
            SparqlResult::Solutions { rows, .. } => {
                let cell = rows[0][0].as_ref().expect("bound result");
                assert!(
                    matches!(cell, TermValue::Literal { lexical_form, .. } if lexical_form == "8"),
                    "the seeded depth travels into the call, got {cell:?}"
                );
            }
            other => panic!("expected solutions, got {other:?}"),
        }
        let err = run(crate::eval::MAX_UDF_DEPTH).expect_err("the ceiling must be enforced");
        assert!(err.to_string().contains("depth bound"), "got: {err}");
    }

    /// One IRI cannot be two function kinds at once: registering an
    /// expression-bodied function over a SPARQL-bodied one panics at registration
    /// rather than silently shadowing it.
    #[test]
    #[should_panic(expected = "already registered as a SPARQL-bodied function")]
    fn expr_function_collides_with_a_sparql_bodied_iri() {
        let mut registry = UserFunctionRegistry::new();
        registry.insert(EX_INC, select_body_function());
        registry.register_expr(
            EX_INC,
            Arity::Exact(0),
            Arc::new(|_call: &ExprFnCall<'_>| Ok(None)),
        );
    }

    /// A native closure's own `Err` return is a hard query failure, rendered
    /// through the generalized (no-longer-SHACL-AF-only) `Function` error text.
    #[test]
    fn native_function_error_is_a_hard_error() {
        let mut registry = UserFunctionRegistry::new();
        registry.register_native(
            EX_NATIVE_ERR,
            Arity::Exact(1),
            Volatility::Stable,
            Arc::new(|_args: &[&TermValue]| Err(EvalError::function("boom"))),
        );
        let ds = empty_dataset();
        let query = format!("SELECT ((<{EX_NATIVE_ERR}>(1)) AS ?v) WHERE {{}}");
        let err = NativeSparqlEngine::new()
            .query_with_options_view(
                &ds,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions {
                    functions: &registry,
                    ..QueryOptions::EMPTY
                },
            )
            .expect_err("closure error must fail the query");
        assert!(
            err.to_string().contains("host function error"),
            "expected generalized 'host function error' text, got {err}"
        );
        assert!(
            err.to_string().contains("boom"),
            "expected the closure's own message to propagate, got {err}"
        );
    }

    /// A panic inside a native closure is caught and converted to a clean,
    /// deterministic [`EvalError::Function`] — the query fails cleanly rather than
    /// aborting the test process.
    #[test]
    fn native_function_panic_is_a_clean_error() {
        // Suppress the default panic-hook stderr dump for this *expected*,
        // caught panic so test output stays clean (mirrors
        // crates/rdf/tests/rdfc_w3c.rs and crates/shapes/tests/w3c_conformance.rs).
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));

        let mut registry = UserFunctionRegistry::new();
        registry.register_native(
            EX_NATIVE_PANIC,
            Arity::Exact(1),
            Volatility::Stable,
            Arc::new(|_args: &[&TermValue]| panic!("native closure exploded")),
        );
        let ds = empty_dataset();
        let query = format!("SELECT ((<{EX_NATIVE_PANIC}>(1)) AS ?v) WHERE {{}}");
        let err = NativeSparqlEngine::new()
            .query_with_options_view(
                &ds,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions {
                    functions: &registry,
                    ..QueryOptions::EMPTY
                },
            )
            .expect_err("a panicking closure must fail the query, not abort it");

        std::panic::set_hook(default_hook);

        assert!(
            err.to_string().contains("panicked"),
            "expected a clean 'panicked' error, got {err}"
        );
        // The panic payload is deliberately NOT interpolated (deterministic across
        // rayon workers), so the closure's own message must not leak into the error.
        assert!(
            !err.to_string().contains("exploded"),
            "panic payload must not leak into the deterministic error message, got {err}"
        );
    }

    /// A call whose argument count does not match the declared [`Arity`] is a hard
    /// error, checked before the closure ever runs.
    #[test]
    fn native_function_wrong_arity_is_a_hard_error() {
        let mut registry = UserFunctionRegistry::new();
        registry.register_native(
            EX_NATIVE_ARITY,
            Arity::Exact(1),
            Volatility::Stable,
            inc_native_body(),
        );
        let ds = empty_dataset();
        // Two arguments to a one-argument native function.
        let query = format!("SELECT ((<{EX_NATIVE_ARITY}>(1, 2)) AS ?v) WHERE {{}}");
        let err = NativeSparqlEngine::new()
            .query_with_options_view(
                &ds,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions {
                    functions: &registry,
                    ..QueryOptions::EMPTY
                },
            )
            .expect_err("arity mismatch must fail");
        assert!(
            err.to_string().contains("expects"),
            "expected arity error, got {err}"
        );
    }

    /// An unbound argument yields no result node rather than reaching the closure
    /// (a native function declares no per-parameter optionality).
    #[test]
    fn native_function_unbound_argument_yields_no_value() {
        let mut registry = UserFunctionRegistry::new();
        registry.register_native(
            EX_NATIVE_UNBOUND,
            Arity::Exact(1),
            Volatility::Stable,
            Arc::new(|_args: &[&TermValue]| {
                panic!("must not be invoked when an argument is unbound")
            }),
        );
        let ds = empty_dataset();
        // `?missing` is never bound.
        let query = format!("SELECT ((<{EX_NATIVE_UNBOUND}>(?missing)) AS ?v) WHERE {{}}");
        let result = NativeSparqlEngine::new()
            .query_with_options_view(
                &ds,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions {
                    functions: &registry,
                    ..QueryOptions::EMPTY
                },
            )
            .expect("query");
        match result {
            SparqlResult::Solutions { rows, .. } => {
                assert!(
                    rows[0][0].is_none(),
                    "unbound argument must yield no value, got {:?}",
                    rows[0][0]
                );
            }
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    /// Registering a native function under an IRI already held by a SPARQL-bodied
    /// function is a hard panic (cross-kind collision guard).
    #[test]
    #[should_panic(expected = "already registered as a SPARQL-bodied function")]
    fn register_native_collision_with_sparql_bodied_panics() {
        let mut registry = UserFunctionRegistry::new();
        registry.insert(
            EX_NATIVE_COLLIDE,
            UserFunction {
                params: vec![int_param("n")],
                required: 1,
                body: parse("SELECT ((?n) AS ?result) WHERE {}"),
                kind: UserFnBody::Select,
                return_constraint: TypeConstraint::default(),
            },
        );
        registry.register_native(
            EX_NATIVE_COLLIDE,
            Arity::Exact(1),
            Volatility::Stable,
            inc_native_body(),
        );
    }

    /// The reverse collision: registering a SPARQL-bodied function under an IRI
    /// already held by a native function is likewise a hard panic.
    #[test]
    #[should_panic(expected = "already registered as a native function")]
    fn insert_collision_with_native_panics() {
        let mut registry = UserFunctionRegistry::new();
        registry.register_native(
            EX_NATIVE_COLLIDE,
            Arity::Exact(1),
            Volatility::Stable,
            inc_native_body(),
        );
        registry.insert(
            EX_NATIVE_COLLIDE,
            UserFunction {
                params: vec![int_param("n")],
                required: 1,
                body: parse("SELECT ((?n) AS ?result) WHERE {}"),
                kind: UserFnBody::Select,
                return_constraint: TypeConstraint::default(),
            },
        );
    }

    /// A SPARQL-bodied function and a native function registered under distinct
    /// IRIs coexist in one registry: each resolves through its own path and a
    /// single query using both gets the correct result from each.
    #[test]
    fn native_and_sparql_bodied_coexist() {
        let mut registry = UserFunctionRegistry::new();
        registry.insert(
            EX_SPARQL_ONLY,
            UserFunction {
                params: vec![int_param("n")],
                required: 1,
                body: parse("SELECT ((?n + 1) AS ?result) WHERE {}"),
                kind: UserFnBody::Select,
                return_constraint: TypeConstraint::default(),
            },
        );
        registry.register_native(
            EX_NATIVE_ONLY,
            Arity::Exact(1),
            Volatility::Stable,
            inc_native_body(),
        );
        assert_eq!(registry.len(), 2);

        let ds = empty_dataset();
        let query = format!(
            "SELECT ((<{EX_SPARQL_ONLY}>(1)) AS ?a) ((<{EX_NATIVE_ONLY}>(1)) AS ?b) WHERE {{}}"
        );
        let result = NativeSparqlEngine::new()
            .query_with_options_view(
                &ds,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions {
                    functions: &registry,
                    ..QueryOptions::EMPTY
                },
            )
            .expect("query");
        match result {
            SparqlResult::Solutions { rows, .. } => {
                let a = rows[0][0].as_ref().expect("sparql-bodied result bound");
                let b = rows[0][1].as_ref().expect("native result bound");
                assert!(
                    matches!(a, TermValue::Literal { lexical_form, .. } if lexical_form == "2"),
                    "expected SPARQL-bodied result 2, got {a:?}"
                );
                assert!(
                    matches!(b, TermValue::Literal { lexical_form, .. } if lexical_form == "2"),
                    "expected native result 2, got {b:?}"
                );
            }
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    // ── engine-level push-down determinism ──────────────────────────────────

    /// `FILTER(<score>(?v) > 0.5)` over a `Stable` native scorer returns exactly
    /// the subjects whose score exceeds the threshold — the native call is
    /// correctly pushed down into the FILTER predicate.
    #[test]
    fn native_score_in_filter_pushes_down() {
        const N: usize = 20;
        let mut registry = UserFunctionRegistry::new();
        registry.register_native(
            EX_SCORE,
            Arity::Exact(1),
            Volatility::Stable,
            ratio_score_native_body(N as f64),
        );
        let ds = scored_dataset(N);
        let query =
            format!("SELECT ?s WHERE {{ ?s <{EX_VAL}> ?v . FILTER(<{EX_SCORE}>(?v) > 0.5) }}");

        let got = run_subject_set(&ds, &query, &registry);
        // score(v) = v / 20 > 0.5  <=>  v > 10.
        let expected: BTreeSet<String> = (11..=N)
            .map(|i| format!("{EX_SUBJECT_PREFIX}{i}"))
            .collect();
        assert_eq!(
            got, expected,
            "FILTER over the native scorer must keep exactly the subjects above threshold"
        );
    }

    /// `ORDER BY DESC(<score>(?v))` yields the correct score-descending order and
    /// is reproducible across two runs — the `Stable` native fn is deterministic
    /// under the evaluator's ordering path.
    #[test]
    fn native_score_in_order_by_is_deterministic() {
        const N: usize = 20;
        let mut registry = UserFunctionRegistry::new();
        registry.register_native(
            EX_SCORE,
            Arity::Exact(1),
            Volatility::Stable,
            ratio_score_native_body(N as f64),
        );
        let ds = scored_dataset(N);
        let query =
            format!("SELECT ?s WHERE {{ ?s <{EX_VAL}> ?v }} ORDER BY DESC(<{EX_SCORE}>(?v))");

        let first = run_subject_rows(&ds, &query, &registry);
        // score is strictly increasing in v, so DESC(score) == DESC(v).
        let expected: Vec<String> = (1..=N)
            .rev()
            .map(|i| format!("{EX_SUBJECT_PREFIX}{i}"))
            .collect();
        assert_eq!(
            first, expected,
            "expected score-descending (val-descending) order"
        );

        let second = run_subject_rows(&ds, &query, &registry);
        assert_eq!(
            first, second,
            "ORDER BY over a Stable native fn must be deterministic across runs"
        );
    }

    /// A scorer returning `xsd:double` `NaN` for some rows still yields a
    /// stable *total* order under `ORDER BY`: every input row is present (no row
    /// is dropped for having an incomparable/NaN key) and the row order is
    /// identical across two runs. SPARQL's ORDER BY total order over the
    /// `xsd:double` value space already gives `NaN` a fixed (deterministic,
    /// syntactic-fallback) slot — see `total_order` in `modifier.rs` — so
    /// this test documents that guarantee for a native-fn-derived score rather
    /// than surfacing a latent bug.
    #[test]
    fn native_score_order_by_is_total_over_nan() {
        let mut b = RdfDatasetBuilder::new();
        let val_pred = b.intern_iri(EX_VAL);
        let vals: Vec<i64> = (-5..=5).collect();
        for v in &vals {
            let s = b.intern_iri(&format!("{EX_SUBJECT_PREFIX}{v}"));
            let lit = b.intern_literal(RdfLiteral::typed(v.to_string(), XSD_INTEGER.to_owned()));
            b.push_quad(s, val_pred, lit, None);
        }
        let ds = b.freeze().expect("freeze");

        let mut registry = UserFunctionRegistry::new();
        registry.register_native(
            EX_SCORE_NAN,
            Arity::Exact(1),
            Volatility::Stable,
            nan_score_native_body(),
        );
        let query =
            format!("SELECT ?s WHERE {{ ?s <{EX_VAL}> ?v }} ORDER BY ASC(<{EX_SCORE_NAN}>(?v))");

        let first = run_subject_rows(&ds, &query, &registry);
        let expected_set: BTreeSet<String> = vals
            .iter()
            .map(|v| format!("{EX_SUBJECT_PREFIX}{v}"))
            .collect();
        assert_eq!(
            first.len(),
            vals.len(),
            "every input row (including NaN-scored ones) must survive ORDER BY"
        );
        assert_eq!(
            first.iter().cloned().collect::<BTreeSet<String>>(),
            expected_set,
            "no row may be dropped or duplicated by a NaN-valued sort key"
        );

        let second = run_subject_rows(&ds, &query, &registry);
        assert_eq!(
            first, second,
            "ORDER BY must be a stable, deterministic total order even with NaN keys present"
        );
    }

    /// FILTER over a `Stable` native scorer, over a dataset large enough to
    /// cross [`crate::parallel::PARALLEL_MIN_ROWS`], returns exactly the
    /// expected rows — proving the native fn is safely usable on the fork-join
    /// parallel path at scale (same answer as the sequential semantics).
    #[test]
    fn native_score_filter_over_parallel_threshold() {
        // Comfortably above PARALLEL_MIN_ROWS (1024) so the FILTER's row count
        // actually crosses the fork-join threshold.
        const N: usize = 1500;
        let mut registry = UserFunctionRegistry::new();
        registry.register_native(
            EX_SCORE,
            Arity::Exact(1),
            Volatility::Stable,
            ratio_score_native_body(N as f64),
        );
        let ds = scored_dataset(N);
        let query =
            format!("SELECT ?s WHERE {{ ?s <{EX_VAL}> ?v . FILTER(<{EX_SCORE}>(?v) > 0.5) }}");

        let first = run_subject_set(&ds, &query, &registry);
        // score(v) = v / 1500 > 0.5  <=>  v > 750.
        let expected: BTreeSet<String> = (751..=N)
            .map(|i| format!("{EX_SUBJECT_PREFIX}{i}"))
            .collect();
        assert_eq!(
            first, expected,
            "parallel-path FILTER over the native scorer must match the sequential answer"
        );

        let second = run_subject_set(&ds, &query, &registry);
        assert_eq!(
            first, second,
            "the parallel-path result must be deterministic across runs"
        );
    }
}
