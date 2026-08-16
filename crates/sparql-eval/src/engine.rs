// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The native [`SparqlEngine`] implementation and its parse-memoizing plan cache.
//!
//! [`NativeSparqlEngine`] is the single required impl of the `purrdf-core`
//! `SparqlEngine` seam — the native replacement for the oxigraph-family
//! `spareval` on the query path. Ordinary query entry points accept operationally
//! infallible [`DatasetView`] backends such as [`RdfDataset`] and validated pack
//! views. Lazy backends that can fail during execution use the distinct
//! [`FallibleDatasetView`] entry points, which return a completeness certificate or
//! a typed operational failure with evidence.
//!
//! One entry point per outcome type, and per-call governors only. The ordinary `query*`
//! methods are exactly ungoverned and exactly complete; a caller that wants execution
//! ceilings names them at the call
//! ([`NativeSparqlEngine::query_governed`]) and receives a [`GovernedOutcome`], which
//! distinguishes a complete result from an exhausted budget with certified partial
//! answers. No governor state is ever held on the engine.
//!
//! The [`PlanCache`] memoizes parsing so the static generated query corpus compiles
//! to algebra once, not per run. Full cost-based planning is out of scope here; the
//! cache holds only the parsed [`Query`].

use std::borrow::Cow;
use std::cell::RefCell;
use std::sync::Arc;

use purrdf_core::{
    DatasetView, FallibleDatasetView, GraphMatch, MutableDataset, RdfDataset, RdfDiagnostic,
    SparqlEngine, SparqlRequest, SparqlResult, TermValue, ViewOperationStatus,
};
use purrdf_sparql_algebra::{ParserOptions, Query, SparqlParser};

use crate::DetHashMap;
use crate::dataset_spec::ActiveDataset;
use crate::eval::{
    BgpOrderCache, EvalCtx, EvalOptions, EvaluatedOutcome, LossVocabulary, Outcome,
    StandpointPredicates, evaluate_query, evaluate_query_evaluated, query_pattern,
};
use crate::governor::ledger::ChargeLedger;
use crate::governor::soundness::SpineClass;
use crate::governor::{GovernorState, NonMonotoneBarrier, QueryExplanation, QueryGovernors};
use crate::update::{GraphResolver, UpdateAbort, eval_update};
use crate::{
    BudgetExhausted, CompleteSparqlResult, FallibleSparqlError, FallibleSparqlResult,
    GovernedEvidence, GovernedOutcome, GovernedUpdateOutcome, PartialAnswers, PartialSparqlResult,
    RelationIdentity,
};

/// A parsed, ready-to-evaluate query (the cached unit of the [`PlanCache`]).
#[derive(Debug)]
pub struct PreparedQuery {
    /// The parsed algebra.
    pub query: Query,
    /// The identity of the property-function registry this plan was parsed and
    /// feasibility-ordered against — empty when there was none.
    ///
    /// Carried on the plan rather than left implicit because a plan and a registry can
    /// DISAGREE, and the disagreement is silent in the worst direction: a plan parsed
    /// with no registry lowered a registered relation's predicate to an ordinary triple
    /// pattern, so handing that plan to a governed entry along with the registry would
    /// evaluate a BGP scan that matches nothing and answer the empty bag. The
    /// prepared-plan entries compare this against the registry in their
    /// [`QueryOptions`] and refuse the mismatch (see
    /// [`NativeSparqlEngine::query_prepared_governed_view`]), which turns the one
    /// remaining way to reach that wrong answer into a diagnostic.
    relations: String,
    /// The identity of the custom-aggregate registry this plan was parsed and
    /// admitted against — empty when there was none. The exact twin of
    /// [`Self::relations`], for the exact same reason: a plan admitted a
    /// `Custom` aggregate call against ONE registry (its arity checked, its
    /// registration confirmed) must not silently run against ANOTHER registry
    /// that happens to resolve the same IRI to a DIFFERENT aggregate — see
    /// `check_plan_matches_relations`, which now checks this alongside
    /// [`Self::relations`].
    aggregates: String,
}

impl PreparedQuery {
    /// A plan for an algebra a caller built or rewrote itself, tagged with the
    /// registry identity the plan is valid under.
    ///
    /// The ordinary way to get a [`PreparedQuery`] is
    /// [`NativeSparqlEngine::prepare_query`] or
    /// [`NativeSparqlEngine::prepare_query_with_options`]; this is for a caller that
    /// rewrites a prepared plan's algebra (the entailment lane restricts chase-minted
    /// witnesses) and must hand the rewrite back to a governed entry. `options` must be
    /// the options the ORIGINAL plan was prepared under, and the same options the
    /// rewrite will be evaluated under.
    ///
    /// # Errors
    ///
    /// An [`RdfDiagnostic`] (`native-sparql-property-function`) if a relation in
    /// `options.property_functions`, or an aggregate in `options.aggregates`, panics
    /// while its declaration is read to compute its registry's fingerprint.
    pub fn rewritten(query: Query, options: QueryOptions<'_>) -> Result<Self, RdfDiagnostic> {
        let relations = crate::property_fn_plan::registry_fingerprint(options.property_functions)
            .map_err(|e| {
            RdfDiagnostic::error("native-sparql-property-function", e.to_string())
        })?;
        let aggregates = crate::agg_fn::registry_fingerprint(options.aggregates)
            .map_err(|e| RdfDiagnostic::error("native-sparql-aggregate-function", e.to_string()))?;
        Ok(Self {
            query,
            relations,
            aggregates,
        })
    }
}

/// A parse-memoizing cache keyed on `(base IRI, extension-function namespace set,
/// property-function namespace set, property-function exact-IRI set,
/// property-function registry fingerprint, query text)`.
///
/// The last two are there because a cached entry is not merely a parse: a query
/// carrying a property-function call is also **feasibility-ordered** against the
/// registry's declarations before it is stored (the feasibility-ordering pass). Two
/// differently-configured registries can order the same text differently, so a key
/// without the registry's fingerprint would hand the second host the first host's plan.
/// The exact-IRI set joins the namespace set for the same reason as the namespace
/// set itself: it too decides which triples become calls
/// ([`ParserOptions::property_fn_iris`]), so two configurations that agree on
/// everything else but differ there must not share a plan.
#[derive(Debug, Default)]
pub struct PlanCache {
    entries: DetHashMap<String, Arc<PreparedQuery>>,
}

impl PlanCache {
    /// A fresh, empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse `query` (memoized) into a [`PreparedQuery`], under
    /// [`ParserOptions::default`].
    pub fn prepare(
        &mut self,
        query: &str,
        base_iri: Option<&str>,
    ) -> Result<Arc<PreparedQuery>, RdfDiagnostic> {
        self.prepare_with(query, base_iri, &ParserOptions::default())
    }

    /// Parse `query` (memoized) into a [`PreparedQuery`] with explicit
    /// [`ParserOptions`] (e.g. an extension-function namespace alias).
    pub fn prepare_with(
        &mut self,
        query: &str,
        base_iri: Option<&str>,
        options: &ParserOptions,
    ) -> Result<Arc<PreparedQuery>, RdfDiagnostic> {
        self.prepare_with_relations(
            query,
            base_iri,
            options,
            &crate::property_fn::PropertyFunctionRegistry::EMPTY,
            &crate::agg_fn::AggregateRegistry::EMPTY,
        )
    }

    /// [`Self::prepare_with`], resolving any property-function call against `relations`
    /// and any custom-aggregate call against `aggregates`.
    ///
    /// This is where a call's **admission** happens, and it happens here rather than at
    /// evaluation time on purpose: an unregistered IRI, an arity mismatch, and a chain
    /// no relation can serve are all configuration errors, and a caller's governor
    /// budget is for the work its query does, not for discovering the query could never
    /// have run. An empty `relations`/`aggregates` (including
    /// [`PropertyFunctionRegistry::EMPTY`](crate::property_fn::PropertyFunctionRegistry::EMPTY)/[`AggregateRegistry::EMPTY`](crate::agg_fn::AggregateRegistry::EMPTY),
    /// the canonical "no registry" value) does not soften any of them — a call node
    /// with nothing to resolve against IS the unregistered case.
    ///
    /// # Errors
    ///
    /// An [`RdfDiagnostic`] if the query text does not parse (`native-sparql-query-parse`),
    /// if a property-function call cannot be admitted (`native-sparql-property-function`),
    /// or if a custom-aggregate call cannot be admitted (`native-sparql-aggregate-function`).
    pub fn prepare_with_relations(
        &mut self,
        query: &str,
        base_iri: Option<&str>,
        options: &ParserOptions,
        relations: &crate::property_fn::PropertyFunctionRegistry,
        aggregates: &crate::agg_fn::AggregateRegistry,
    ) -> Result<Arc<PreparedQuery>, RdfDiagnostic> {
        // The cache key must include the base IRI AND the extension-function
        // namespace set: the same text under a different base or namespace
        // configuration parses to a different algebra. The property-function
        // namespace set, the property-function exact-IRI set, and the two registry
        // fingerprints join it for the same reason one step later: together they
        // decide which triples become calls, how those calls are ordered, and which
        // `Custom` aggregate IRIs are admitted at all. Without the aggregate
        // fingerprint in the key, two callers configuring DIFFERENT aggregate
        // registries but sharing every other cache-key component would receive the
        // SAME cached `PreparedQuery` — whichever caller populated the cache first —
        // and the second caller's evaluation would then fail
        // `check_plan_matches_relations` against a plan it never actually prepared.
        let fingerprint = crate::property_fn_plan::registry_fingerprint(relations)
            .map_err(|e| RdfDiagnostic::error("native-sparql-property-function", e.to_string()))?;
        let agg_fingerprint = crate::agg_fn::registry_fingerprint(aggregates)
            .map_err(|e| RdfDiagnostic::error("native-sparql-aggregate-function", e.to_string()))?;
        let key = format!(
            "{}\u{0}{}\u{0}{}\u{0}{}\u{0}{}\u{0}{}\u{0}{}",
            base_iri.unwrap_or(""),
            options.extension_fn_namespaces.join("\u{1}"),
            options.property_fn_namespaces.join("\u{1}"),
            options.property_fn_iris.join("\u{1}"),
            fingerprint,
            agg_fingerprint,
            query
        );
        if let Some(prepared) = self.entries.get(&key) {
            return Ok(prepared.clone());
        }
        let mut parser = SparqlParser::new();
        if let Some(base) = base_iri {
            parser = parser.with_base_iri(base);
        }
        let parsed = parser
            .parse_query_with(query, options)
            .map_err(|e| RdfDiagnostic::error("native-sparql-query-parse", e.to_string()))?;
        let planned = crate::property_fn_plan::plan_query(&parsed, relations, aggregates)
            .map_err(|e| RdfDiagnostic::error(e.diagnostic_code(), e.to_string()))?;
        let prepared = Arc::new(PreparedQuery {
            query: planned.unwrap_or(parsed),
            relations: fingerprint,
            aggregates: agg_fingerprint,
        });
        self.entries.insert(key, prepared.clone());
        Ok(prepared)
    }
}

/// The native, RDF-1.2-first multiset SPARQL engine (purrdf S6).
///
/// Domain-vocabulary seams are **caller configuration**, never engine constants:
///
/// - [`Self::with_parser_options`] configures the extension-function namespace
///   set (default: EMPTY — extension functions are off and a call-position IRI
///   is an ordinary custom function). A deployment whose queries spell the closed
///   function set under its own namespace (e.g. `http://example.org/ns/gmeow/`)
///   supplies that namespace here so that prefix's function calls parse as
///   extension calls.
/// - [`Self::with_standpoint_predicates`] supplies the `accordingTo`/`sharpens`
///   predicate table that `heldIn` and loss-aware `CONSTRUCT` read from
///   the caller's data. Without it, `heldIn` is a hard evaluation error.
/// - [`Self::with_loss_vocabulary`] supplies the `ProjectionLoss` vocabulary IRIs
///   emitted by loss-aware `CONSTRUCT` when a reifier is dropped. Without it,
///   loss declarations stay inactive.
#[derive(Default)]
pub struct NativeSparqlEngine {
    cache: RefCell<PlanCache>,
    /// The dataset-aware BGP join-order cache, shared across this engine's queries so
    /// the static query corpus re-plans each BGP once per dataset (see [`BgpOrderCache`]).
    order_cache: BgpOrderCache,
    resolver: Option<Arc<dyn GraphResolver>>,
    /// Parse-time configuration (the extension-function namespace set), applied to
    /// every query and update this engine parses. Defaults to empty (no extension
    /// namespaces — the seam is caller configuration).
    parser_options: ParserOptions,
    /// The caller-supplied standpoint predicate table threaded into every
    /// evaluation context. `None` (the default) means `heldIn` hard-errors
    /// and `CONSTRUCT` emits no standpoint-scope loss attribution.
    standpoint_predicates: Option<StandpointPredicates>,
    /// The caller-supplied loss-declaration vocabulary threaded into every
    /// evaluation context. `None` (the default) means loss-aware `CONSTRUCT`
    /// emits no in-band loss declarations.
    loss_vocabulary: Option<LossVocabulary>,
    /// Evaluation-time options threaded into every per-query context. Defaults to
    /// production settings; tests and benches override individual flags through
    /// [`Self::with_eval_options`].
    eval_options: EvalOptions,
}

// `dyn GraphResolver` is not `Debug`, so derive can't apply; report its presence by
// hand (`Some(..)`/`None`) and keep the cache's own `Debug`.
impl std::fmt::Debug for NativeSparqlEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeSparqlEngine")
            .field("cache", &self.cache)
            .field("order_cache", &self.order_cache)
            .field(
                "resolver",
                match &self.resolver {
                    Some(_) => &"Some(..)",
                    None => &"None",
                },
            )
            .field("parser_options", &self.parser_options)
            .field("standpoint_predicates", &self.standpoint_predicates)
            .field("loss_vocabulary", &self.loss_vocabulary)
            .field("eval_options", &self.eval_options)
            .finish()
    }
}

impl NativeSparqlEngine {
    /// Parse a query through this engine's memoizing plan cache.
    ///
    /// Callers that inspect query algebra before evaluation can retain the
    /// returned plan and pass it to [`Self::query_prepared`], avoiding a second
    /// parse or cache lookup.
    ///
    /// # Errors
    ///
    /// Returns an [`RdfDiagnostic`] if the query text does not parse.
    pub fn prepare_query(
        &self,
        query: &str,
        base_iri: Option<&str>,
    ) -> Result<Arc<PreparedQuery>, RdfDiagnostic> {
        self.prepare_query_with_options(query, base_iri, QueryOptions::EMPTY)
    }

    /// [`Self::prepare_query`] against the [`QueryOptions`] the plan will be evaluated
    /// under — the registry-aware parse.
    ///
    /// A caller that holds a plan across executions (the CLI's view-op seam, the
    /// entailment lane's rewritten plan) must prepare it here when it has a
    /// property-function registry, and hand the SAME options to the governed entry that
    /// runs it: the registry decides which predicates became call nodes, so a plan
    /// prepared without it has already lost every relation the query named.
    ///
    /// # Errors
    ///
    /// Returns an [`RdfDiagnostic`] if the query text does not parse, or if a
    /// property-function call cannot be admitted against the registry.
    pub fn prepare_query_with_options(
        &self,
        query: &str,
        base_iri: Option<&str>,
        options: QueryOptions<'_>,
    ) -> Result<Arc<PreparedQuery>, RdfDiagnostic> {
        self.prepare_for(
            query,
            base_iri,
            options.property_functions,
            options.aggregates,
        )
    }

    /// Evaluate a plan returned by [`Self::prepare_query`] or
    /// [`Self::prepare_query_with_options`].
    ///
    /// `options` must name the same property-function registry the plan was prepared
    /// against — the identical requirement [`Self::query_prepared_governed_view`]
    /// states, checked the same way: a plan/registry disagreement refuses the plan
    /// rather than silently evaluating it under the wrong (or no) registry, because a
    /// plan prepared without a registry has already lowered every relation's predicate
    /// to an ordinary triple pattern.
    ///
    /// # Errors
    ///
    /// Propagates evaluation errors as an [`RdfDiagnostic`], and refuses
    /// (`native-sparql-property-function`) a plan prepared against a different registry
    /// than `options` supplies.
    pub fn query_prepared(
        &self,
        dataset: &Arc<RdfDataset>,
        prepared: &PreparedQuery,
        substitutions: &[(String, TermValue)],
        options: QueryOptions<'_>,
    ) -> Result<SparqlResult, RdfDiagnostic> {
        self.query_prepared_view(&**dataset, prepared, substitutions, options)
    }

    /// [`Self::query_prepared`] over any operationally infallible [`DatasetView`]
    /// backend. The concrete [`Self::query_prepared`] is a thin wrapper that derefs its
    /// `Arc<RdfDataset>` and calls this.
    ///
    /// Do not use this entry point for a lazy view whose backing storage can fail after
    /// construction: iterator exhaustion would then be indistinguishable from missing
    /// data. Use [`Self::query_prepared_fallible_view`] for that contract.
    ///
    /// # Errors
    ///
    /// Propagates evaluation errors as an [`RdfDiagnostic`], and refuses
    /// (`native-sparql-property-function`) a plan prepared against a different registry
    /// than `options` supplies (see [`Self::query_prepared`]).
    pub fn query_prepared_view<'d, D: DatasetView + Sync>(
        &'d self,
        dataset: &'d D,
        prepared: &PreparedQuery,
        substitutions: &[(String, TermValue)],
        options: QueryOptions<'d>,
    ) -> Result<SparqlResult, RdfDiagnostic> {
        check_plan_matches_relations(prepared, options)?;
        let ctx = self.eval_ctx(dataset);
        let mut ctx = apply_query_options(ctx, options)?;
        let outcome = match options.prebinding {
            ShaclPrebinding::Applied => {
                evaluate_with_shacl_prebinding(prepared, substitutions, &mut ctx)?
            }
            ShaclPrebinding::None => {
                evaluate_with_substitutions(prepared, substitutions, &mut ctx)?
            }
        };
        Ok(materialize(outcome, &ctx))
    }

    /// Evaluate a prepared query over an operationally fallible view and return a
    /// result only after the view supplies a final ready checkpoint.
    ///
    /// Evaluation is sequential inside this operation so page-request order and exact
    /// budget boundaries are deterministic. The ordinary resident/pack entry points
    /// remain unchanged and retain their parallel fast path.
    ///
    /// `options` must name the same property-function registry the plan was prepared
    /// against — the identical requirement [`Self::query_prepared`] states, checked the
    /// same way: a plan/registry disagreement refuses the plan rather than silently
    /// evaluating it under the wrong (or no) registry.
    ///
    /// # Errors
    ///
    /// Returns [`FallibleSparqlError::Operational`] when either checkpoint or any
    /// lazy read fails. That root cause takes precedence over an evaluator diagnostic,
    /// and all internal partial rows are discarded. Returns
    /// [`FallibleSparqlError::Query`] when the view remains ready and either ordinary
    /// evaluation fails or `prepared` was prepared against a different registry than
    /// `options` supplies (`native-sparql-property-function`).
    pub fn query_prepared_fallible_view<'d, D>(
        &'d self,
        dataset: &'d D,
        prepared: &PreparedQuery,
        substitutions: &[(String, TermValue)],
        options: QueryOptions<'d>,
    ) -> FallibleSparqlResult<D::Error, D::Evidence>
    where
        D: FallibleDatasetView + Sync,
    {
        preflight_fallible_view(dataset)?;
        let evaluation = {
            let _sequential = crate::parallel::force_sequential_operation();
            (|| {
                check_plan_matches_relations(prepared, options)?;
                let ctx = self.eval_ctx(dataset);
                let mut ctx = apply_query_options(ctx, options)?;
                let outcome = match options.prebinding {
                    ShaclPrebinding::Applied => {
                        evaluate_with_shacl_prebinding(prepared, substitutions, &mut ctx)?
                    }
                    ShaclPrebinding::None => {
                        evaluate_with_substitutions(prepared, substitutions, &mut ctx)?
                    }
                };
                Ok(materialize(outcome, &ctx))
            })()
        };
        finish_fallible_query(dataset, evaluation)
    }

    /// Parse and execute one request over an operationally fallible view.
    ///
    /// This is the convenience sibling of
    /// [`query_prepared_fallible_view`](Self::query_prepared_fallible_view). Parsing
    /// still uses the engine's memoizing plan cache, through the same registry-aware
    /// path [`Self::prepare_query_with_options`] uses, so `options.property_functions`
    /// decides which predicates are calls; execution applies the same preflight/final
    /// completeness checkpoints and deterministic sequential scope.
    ///
    /// # Errors
    ///
    /// Returns a typed [`FallibleSparqlError`] carrying evidence for either an
    /// operational root cause or an ordinary parse/evaluation diagnostic.
    pub fn query_fallible_view<'d, D>(
        &'d self,
        dataset: &'d D,
        request: SparqlRequest<'_>,
        options: QueryOptions<'d>,
    ) -> FallibleSparqlResult<D::Error, D::Evidence>
    where
        D: FallibleDatasetView + Sync,
    {
        preflight_fallible_view(dataset)?;
        let prepared = match self.prepare_for(
            request.query,
            request.base_iri,
            options.property_functions,
            options.aggregates,
        ) {
            Ok(prepared) => prepared,
            Err(diagnostic) => return finish_fallible_query(dataset, Err(diagnostic)),
        };
        self.query_prepared_fallible_view(dataset, &prepared, request.substitutions, options)
    }

    /// Parse and execute one request under caller-supplied execution governors.
    ///
    /// The governed sibling of [`SparqlEngine::query`], and the only public entry point
    /// through which a governor trip is an *outcome* rather than a failure: it returns a
    /// [`GovernedOutcome`], which is either the complete result or the exhausted budget
    /// with the partial answers the execution reached (see [`PartialAnswers`] for what
    /// those are allowed to claim). A genuine parse or evaluation failure is still an
    /// [`RdfDiagnostic`] — a query that has no answer must never be reported as a query
    /// that ran out of budget.
    ///
    /// # Governors are per call, never per engine
    ///
    /// The live accounting state is built fresh here, for this execution, and dropped with
    /// it. There is deliberately no `with_governors` builder on the engine: consumption is
    /// cumulative, so a state held on the engine would drain one query's budget into the
    /// next and produce an intermittent "this query was fine yesterday" bug that no single
    /// test run can catch. This is the same rule the demand-paging tier states for
    /// [`PagedQueryView`](purrdf_core::ir::PagedQueryView) — caches, evidence, and limits
    /// are operation-local.
    ///
    /// Pass [`QueryGovernors::UNBOUNDED`] to decline every ceiling explicitly, or
    /// [`QueryGovernors::METERED`] to measure an execution without bounding it.
    ///
    /// # Options are a parameter, not an overload
    ///
    /// `options` carries the registries and rewrites this execution runs under —
    /// pass [`QueryOptions::EMPTY`] to configure none. It is required rather than
    /// defaulted because a [`PropertyFunctionRegistry`](crate::property_fn::PropertyFunctionRegistry)
    /// is *parse* configuration: an entry that could not be handed one would parse a
    /// registered relation's predicate as an ordinary triple pattern and answer the
    /// empty bag, silently. See [`QueryOptions`].
    ///
    /// # Errors
    ///
    /// Propagates parse and evaluation errors as an [`RdfDiagnostic`]. A tripped governor
    /// is **not** an error and does not surface here.
    pub fn query_governed(
        &self,
        dataset: &Arc<RdfDataset>,
        request: SparqlRequest<'_>,
        options: QueryOptions<'_>,
        governors: &QueryGovernors,
    ) -> Result<GovernedOutcome, RdfDiagnostic> {
        let prepared = self.prepare_for(
            request.query,
            request.base_iri,
            options.property_functions,
            options.aggregates,
        )?;
        self.query_prepared_governed_view(
            &**dataset,
            &prepared,
            request.substitutions,
            options,
            governors,
        )
    }

    /// [`Self::query_governed`] over any operationally infallible [`DatasetView`] backend,
    /// for a plan already returned by [`Self::prepare_query`] or
    /// [`Self::prepare_query_with_options`].
    ///
    /// Governors are per call here too — `governors` configures this one execution and
    /// nothing else, so the same `prepared` plan can be run under different budgets
    /// without one run's consumption reaching the next.
    ///
    /// `options` must name the same property-function registry the plan was prepared
    /// against: the registry decides both which predicates became call nodes and which
    /// relation each call resolves to, and the admission estimate prices a call from its
    /// declared row bound.
    ///
    /// # Errors
    ///
    /// Propagates evaluation errors as an [`RdfDiagnostic`]. A tripped governor is **not**
    /// an error and does not surface here.
    pub fn query_prepared_governed_view<D: DatasetView + Sync>(
        &self,
        dataset: &D,
        prepared: &PreparedQuery,
        substitutions: &[(String, TermValue)],
        options: QueryOptions<'_>,
        governors: &QueryGovernors,
    ) -> Result<GovernedOutcome, RdfDiagnostic> {
        let state = Arc::new(GovernorState::new(governors));
        self.query_governed_prepared_in_state(
            dataset,
            prepared,
            substitutions,
            options,
            None,
            &state,
        )
    }

    /// The ONE governed evaluation body: admit the plan under `state`'s ceilings, build
    /// the context, apply `options`, evaluate on the trip-aware channel, materialize.
    ///
    /// Every governed entry — per-call or operation-scoped, local or federated, over an
    /// infallible or a fallible view — reaches evaluation through here, so the charge
    /// points, the admission estimate, and the options application are wired once. The
    /// two axes the entries differ on are parameters: `source` (a federated entry injects
    /// one) and who owns `state` (a per-call entry built it; an operation entry was
    /// handed the one its whole operation charges).
    fn query_governed_prepared_in_state<'d, D: DatasetView + Sync>(
        &'d self,
        dataset: &'d D,
        prepared: &PreparedQuery,
        substitutions: &[(String, TermValue)],
        options: QueryOptions<'d>,
        source: Option<&'d (dyn crate::remote::RemoteQuerySource + Sync)>,
        state: &Arc<GovernorState>,
    ) -> Result<GovernedOutcome, RdfDiagnostic> {
        check_plan_matches_relations(prepared, options)?;
        // `prepared.relations` is the registry fingerprint computed once at prepare and
        // just validated against `options.property_functions` above — reused rather than
        // re-derived, so this receipt's identity and the plan cache's key never disagree.
        let identity = relation_identity(prepared, options.property_functions)?;
        if let Some(refused) = self.admit(
            dataset,
            &prepared.query,
            options.property_functions,
            state,
            &identity,
        ) {
            return refused;
        }
        let mut ctx = self.eval_ctx(dataset).with_governors(Arc::clone(state));
        if let Some(source) = source {
            ctx = ctx.with_remote(source);
        }
        let mut ctx = apply_query_options(ctx, options)?;
        let evaluated = match options.prebinding {
            ShaclPrebinding::Applied => {
                evaluate_governed_with_shacl_prebinding(prepared, substitutions, &mut ctx)?
            }
            ShaclPrebinding::None => {
                evaluate_governed_with_substitutions(prepared, substitutions, &mut ctx)?
            }
        };
        Ok(materialize_governed(evaluated, &ctx, state, identity))
    }

    /// [`Self::query_governed`] with a
    /// [`RemoteQuerySource`](crate::remote::RemoteQuerySource) injected, so `SERVICE`
    /// clauses resolve through it.
    ///
    /// The governed sibling of [`Self::query_with_source`], and the entry a federated
    /// caller needs: without it, governing a federated query would mean choosing between
    /// a budget and a `SERVICE` clause. The source is handed this execution's stop signal
    /// through the evaluation context exactly as the ungoverned path hands it nothing, so
    /// a deadline can *prevent* a remote request rather than only be noticed once one has
    /// returned.
    ///
    /// # Errors
    ///
    /// Propagates parse and evaluation errors as an [`RdfDiagnostic`]. A tripped governor
    /// is **not** an error and does not surface here.
    pub fn query_governed_with_source(
        &self,
        dataset: &Arc<RdfDataset>,
        request: SparqlRequest<'_>,
        source: &(dyn crate::remote::RemoteQuerySource + Sync),
        options: QueryOptions<'_>,
        governors: &QueryGovernors,
    ) -> Result<GovernedOutcome, RdfDiagnostic> {
        self.query_governed_with_source_view(&**dataset, request, source, options, governors)
    }

    /// [`Self::query_governed_with_source`] over any [`DatasetView`] backend whose id type
    /// is the production [`TermId`](purrdf_core::TermId).
    ///
    /// # Errors
    ///
    /// Propagates parse and evaluation errors as an [`RdfDiagnostic`]. A tripped governor
    /// is **not** an error and does not surface here.
    pub fn query_governed_with_source_view<'d, D: DatasetView + Sync>(
        &'d self,
        dataset: &'d D,
        request: SparqlRequest<'_>,
        source: &'d (dyn crate::remote::RemoteQuerySource + Sync),
        options: QueryOptions<'d>,
        governors: &QueryGovernors,
    ) -> Result<GovernedOutcome, RdfDiagnostic> {
        let prepared = self.prepare_for(
            request.query,
            request.base_iri,
            options.property_functions,
            options.aggregates,
        )?;
        let state = Arc::new(GovernorState::new(governors));
        self.query_governed_prepared_in_state(
            dataset,
            &prepared,
            request.substitutions,
            options,
            Some(source),
            &state,
        )
    }

    /// One governed query of a **multi-query operation**, charged against a
    /// `state` the caller owns and reuses across every query of that operation.
    ///
    /// # Why a shared state, when `query_governed` forbids exactly that
    ///
    /// [`Self::query_governed`] builds its state per call and has no `with_governors`
    /// builder, because a state held on the *engine* would drain one caller's budget into
    /// an unrelated caller's query. That rule is about the **engine**, and it is not
    /// weakened here: the state is still owned by whoever is running the operation, still
    /// built per operation, and still dropped with it.
    ///
    /// The distinction this entry exists for is that some operations are not one query.
    /// A SHACL validation runs one `sh:sparql` query **per focus node** — hundreds of them
    /// for one call — and every one of those is the same caller's single request. Handing
    /// each a fresh [`QueryGovernors`] would give an N-focus validation N times the budget
    /// it asked for, which is not a budget. So the operation builds one
    /// [`GovernorState`] and every query inside it charges the same one; the state is
    /// `Sync`, so the operation may run its queries on workers.
    ///
    /// `options` carries what the operation configured: the SHACL-AF function registry
    /// and the property-function registry if it has them, the deterministic blank-mint
    /// prefix a rules run gives each focus node
    /// ([`EvalCtx::with_bnode_mint_prefix`]), and
    /// [`QueryOptions::prebinding`], which selects the SHACL pre-binding rewrite
    /// (`sh:sparql` constraint and component bodies, `sh:SPARQLRule`, `sh:ask`/`sh:select`
    /// validators) over the ordinary substitution rewrite (SHACL-AF node expressions and
    /// `sh:SPARQLTarget`).
    ///
    /// # Errors
    ///
    /// Propagates parse and evaluation errors as an [`RdfDiagnostic`]. A tripped governor
    /// is **not** an error and does not surface here.
    pub fn query_governed_in_operation<'d, D: DatasetView + Sync>(
        &'d self,
        dataset: &'d D,
        request: SparqlRequest<'_>,
        options: QueryOptions<'d>,
        state: &Arc<GovernorState>,
    ) -> Result<GovernedOutcome, RdfDiagnostic> {
        let prepared = self.prepare_for(
            request.query,
            request.base_iri,
            options.property_functions,
            options.aggregates,
        )?;
        self.query_governed_prepared_in_state(
            dataset,
            &prepared,
            request.substitutions,
            options,
            None,
            state,
        )
    }

    /// Parse and execute one request under caller-supplied governors over an
    /// operationally fallible view.
    ///
    /// The governed sibling of [`Self::query_fallible_view`], carrying both meters: the
    /// evidence type is a [`GovernedEvidence`] pair, so every outcome reports the view's
    /// pages and bytes **and** this execution's fuel, rows, and cells. Evaluation is
    /// sequential inside this operation for the same reason the ungoverned fallible path
    /// is — page-request order and exact budget boundaries stay deterministic.
    ///
    /// A trip is reported as [`FallibleSparqlError::BudgetExhausted`] rather than as an
    /// `Ok`, because [`CompleteSparqlResult`] is a completeness certificate and is never
    /// built from partial rows. The partial answers travel with it.
    ///
    /// # Errors
    ///
    /// Returns [`FallibleSparqlError::Operational`] when either checkpoint or any lazy
    /// read fails — that root cause outranks both an evaluator diagnostic and a governor
    /// trip derived after data became unavailable. Returns
    /// [`FallibleSparqlError::Query`] when the view remained ready and parsing or
    /// evaluation failed, and [`FallibleSparqlError::BudgetExhausted`] when the view
    /// remained ready and a governor stopped the execution.
    #[allow(
        clippy::result_large_err,
        reason = "the Err side carries the two receipts a governed caller reads (the \
                  view's evidence and this execution's ResourceVectors) plus the certified \
                  partial answers; boxing it would put an allocation on the reporting path \
                  of every non-complete outcome to save one move per query"
    )]
    pub fn query_governed_fallible_view<'d, D>(
        &'d self,
        dataset: &'d D,
        request: SparqlRequest<'_>,
        options: QueryOptions<'d>,
        governors: &QueryGovernors,
    ) -> FallibleSparqlResult<D::Error, GovernedEvidence<D::Evidence>>
    where
        D: FallibleDatasetView + Sync,
    {
        self.query_governed_fallible_view_inner(dataset, request, None, options, governors)
    }

    /// [`Self::query_governed_fallible_view`] with a federation source injected.
    ///
    /// This is the composed carrier boundary for a lazy local view plus remote
    /// `SERVICE` calls: the returned [`GovernedEvidence`] contains the local view's
    /// page/byte receipt and the evaluator's remote-request, fuel, row, and cell receipt
    /// from the same operation. Operational failure keeps its usual precedence over an
    /// evaluator diagnostic or governor trip.
    ///
    /// # Errors
    ///
    /// Returns the same typed outcomes as [`Self::query_governed_fallible_view`]. A
    /// federation failure while the view remains ready is a query diagnostic; a local
    /// view failure remains [`FallibleSparqlError::Operational`].
    #[allow(
        clippy::result_large_err,
        reason = "the Err side carries both operation receipts and certified partial answers"
    )]
    pub fn query_governed_fallible_with_source_view<'d, D>(
        &'d self,
        dataset: &'d D,
        request: SparqlRequest<'_>,
        source: &'d (dyn crate::remote::RemoteQuerySource + Sync),
        options: QueryOptions<'d>,
        governors: &QueryGovernors,
    ) -> FallibleSparqlResult<D::Error, GovernedEvidence<D::Evidence>>
    where
        D: FallibleDatasetView + Sync,
    {
        self.query_governed_fallible_view_inner(dataset, request, Some(source), options, governors)
    }

    #[allow(
        clippy::result_large_err,
        reason = "the Err side carries both operation receipts and certified partial answers"
    )]
    fn query_governed_fallible_view_inner<'d, D>(
        &'d self,
        dataset: &'d D,
        request: SparqlRequest<'_>,
        source: Option<&'d (dyn crate::remote::RemoteQuerySource + Sync)>,
        options: QueryOptions<'d>,
        governors: &QueryGovernors,
    ) -> FallibleSparqlResult<D::Error, GovernedEvidence<D::Evidence>>
    where
        D: FallibleDatasetView + Sync,
    {
        // Built before the preflight so that a view that failed on the way in still
        // reports a (zeroed, honest) governor receipt beside its root cause.
        let state = Arc::new(GovernorState::new(governors));
        if let ViewOperationStatus::Failed { error, evidence } = dataset.operation_status() {
            return Err(FallibleSparqlError::Operational {
                error,
                evidence: GovernedEvidence::new(evidence, state.evidence()),
            });
        }
        let prepared = match self.prepare_for(
            request.query,
            request.base_iri,
            options.property_functions,
            options.aggregates,
        ) {
            Ok(prepared) => prepared,
            Err(diagnostic) => {
                return finish_governed_fallible_query(dataset, &state, Err(diagnostic));
            }
        };
        let evaluation = {
            let _sequential = crate::parallel::force_sequential_operation();
            self.query_governed_prepared_in_state(
                dataset,
                &prepared,
                request.substitutions,
                options,
                source,
                &state,
            )
        };
        finish_governed_fallible_query(dataset, &state, evaluation)
    }

    /// Parse and execute one SPARQL **UPDATE** request under caller-supplied execution
    /// governors.
    ///
    /// The governed sibling of [`SparqlEngine::update`], and the only entry through which a
    /// governor trip on a mutation is an *outcome* rather than a failure: it returns a
    /// [`GovernedUpdateOutcome`], which says the request applied or that a governor stopped
    /// it. A genuine parse or evaluation failure is still an [`RdfDiagnostic`].
    ///
    /// # A trip applies nothing — that is the contract, not a best effort
    ///
    /// On [`GovernedUpdateOutcome::BudgetExhausted`] the caller's `dataset` handle is left
    /// **exactly** as it was found: not re-frozen, not equal-but-rebuilt, the same `Arc`.
    /// That holds however far the request got — a five-operation request whose fifth
    /// operation trips discards the first four with it, because a store carrying part of a
    /// request nobody was told about is corrupt in a way no later query can detect. It is
    /// structural rather than defensive: every operation applies to a copy-on-write branch
    /// of the frozen base, and the single assignment that publishes that branch sits on the
    /// applied path alone (see [`crate::update`] for the full mutation model).
    ///
    /// There is deliberately no partial-mutation payload on the outcome, for the reason
    /// [`GovernedUpdateOutcome`] states.
    ///
    /// # Which ceilings bind a mutation
    ///
    /// Everything an UPDATE's `WHERE` clause does is charged exactly as the same pattern
    /// inside a `SELECT` is — fuel, the intermediate-cell peak, scratch bytes, remote
    /// requests, the recursion guard — and the stop signal is polled at the evaluator's
    /// charge points, before each operation of the request, and before the `LOAD` host seam
    /// issues any I/O.
    ///
    /// [`QueryGovernors::with_max_answers`] is the one ceiling that does **not** apply: it
    /// bounds the answer sequence a caller receives, and an UPDATE has none. A request's
    /// size is bounded by the ceilings on the work that computes it, which is what those
    /// other dimensions are. Setting only an answer cap on an update therefore governs
    /// nothing, and is stated here rather than silently approximated by capping some
    /// unrelated quantity.
    ///
    /// # Governors are per call, never per engine
    ///
    /// The live accounting state is built here, for this request, and dropped with it —
    /// the same rule, for the same reason, as [`Self::query_governed`].
    ///
    /// # Options are a parameter, not an overload
    ///
    /// `options` carries the registries this request's `WHERE` clauses run under —
    /// pass [`QueryOptions::EMPTY`] to configure none. Required for the identical
    /// reason [`Self::query_governed`] requires it: a
    /// [`PropertyFunctionRegistry`](crate::property_fn::PropertyFunctionRegistry) is
    /// *parse* configuration, and an UPDATE's `WHERE` is a triple-pattern context —
    /// an entry that could not be handed a registry would parse a registered
    /// relation's predicate as an ordinary triple pattern and either match nothing
    /// or hard-error, never dispatch the call. See [`QueryOptions`].
    ///
    /// # Errors
    ///
    /// Propagates parse and evaluation errors as an [`RdfDiagnostic`]. A tripped governor
    /// is **not** an error and does not surface here.
    pub fn update_governed(
        &self,
        dataset: &mut Arc<RdfDataset>,
        request: SparqlRequest<'_>,
        options: QueryOptions<'_>,
        governors: &QueryGovernors,
    ) -> Result<GovernedUpdateOutcome, RdfDiagnostic> {
        let update = self.parse_update(&request, options.property_functions)?;
        let state = Arc::new(GovernorState::new(governors));
        let mut m = MutableDataset::new(Arc::clone(dataset));
        let cfg = crate::update::UpdateEvalConfig {
            standpoint_predicates: self.standpoint_predicates.as_ref(),
            order_cache: &self.order_cache,
            governors: Some(&state),
            options,
        };
        let tripped = match eval_update(&update, &mut m, self.resolver.as_deref(), &cfg) {
            // A request that ran to the end still does not publish while a trip is latched
            // on the state. The two are normally the same fact — every site that observes a
            // governor also aborts the request — but "normally" is the wrong strength for
            // the one assignment that can corrupt a store, so the publish seam asks the
            // state directly rather than inferring the answer from the control flow that
            // reached it. This is what makes `Applied` mean "no governor stopped this",
            // rather than "no governor stopped this on any path anybody has thought of".
            Ok(()) => {
                // A stop that fired during the final operation must be observed before
                // the only publication assignment. Polling here is deliberately
                // independent of fuel engagement.
                let _ = state.poll_stop();
                state.tripped()
            }
            Err(UpdateAbort::Failed(diagnostic)) => return Err(diagnostic),
            Err(UpdateAbort::Tripped(tripped)) => Some(tripped),
        };
        match tripped {
            None => {
                // The one place the branch is published, and it is on this arm only.
                *dataset = m.freeze()?;
                Ok(GovernedUpdateOutcome::Applied {
                    evidence: state.evidence(),
                })
            }
            Some(tripped) => {
                // `m` is dropped here with every mutation the request had reached in it.
                // Dropping the branch IS the rollback; `*dataset` was never written.
                drop(m);
                Ok(GovernedUpdateOutcome::BudgetExhausted {
                    tripped,
                    evidence: state.evidence(),
                })
            }
        }
    }

    /// Parse one UPDATE request into algebra, under the [`ParserOptions`] `registry`
    /// derives (see [`Self::parser_options_for`]) — the `prepare_for`-equivalent for
    /// updates: a registered relation's predicate is recognized as a call node by
    /// EXACT IRI here exactly as it is on the query lane, whether or not the engine
    /// also declared the relation's namespace via [`Self::with_parser_options`].
    ///
    /// UPDATE deliberately bypasses the plan cache: these requests are side-effecting and
    /// are not the hot static-query set the cache exists for; caching a mutating statement
    /// would be a correctness hazard. Shared by the two UPDATE seams so that a governed and
    /// an ungoverned request of the same text, under the same registry, parse identically —
    /// including the base IRI and the engine's [`ParserOptions`].
    fn parse_update(
        &self,
        request: &SparqlRequest<'_>,
        registry: &crate::property_fn::PropertyFunctionRegistry,
    ) -> Result<purrdf_sparql_algebra::Update, RdfDiagnostic> {
        let mut parser = SparqlParser::new();
        if let Some(base) = request.base_iri {
            parser = parser.with_base_iri(base);
        }
        let options = self.parser_options_for(registry)?;
        parser
            .parse_update_with(request.query, &options)
            .map_err(|e| RdfDiagnostic::error("native-sparql-update-parse", e.to_string()))
    }

    /// The one **ungoverned** options-carrying UPDATE entry, parameterized by
    /// [`QueryOptions`]: injects a property-function registry (and, symmetrically, a
    /// SHACL-AF function registry and a deterministic blank-mint prefix) into the
    /// `WHERE` evaluation of every `DELETE`/`INSERT … WHERE` in the request.
    /// [`SparqlEngine::update`] is this entry under [`QueryOptions::EMPTY`] — the
    /// ungoverned sibling of [`Self::update_governed`], the same relationship
    /// [`Self::query_with_options_view`] has to [`Self::query_governed`].
    ///
    /// # Errors
    ///
    /// Propagates parse and evaluation errors as an [`RdfDiagnostic`].
    pub fn update_with_options(
        &self,
        dataset: &mut Arc<RdfDataset>,
        request: SparqlRequest<'_>,
        options: QueryOptions<'_>,
    ) -> Result<(), RdfDiagnostic> {
        let update = self.parse_update(&request, options.property_functions)?;
        // Atomicity is structural: branch a COW MutableDataset off the frozen base,
        // apply every op to the delta, and only on FULL success freeze back. Any
        // error drops `m` and leaves `*dataset` untouched.
        let mut m = MutableDataset::new(Arc::clone(dataset));
        let cfg = crate::update::UpdateEvalConfig {
            standpoint_predicates: self.standpoint_predicates.as_ref(),
            order_cache: &self.order_cache,
            // Exactly ungoverned, exactly as this seam was before governors existed —
            // `None` is the absence of the state, not an unbounded one, so no charge site
            // or stop poll is reachable from here at all. A caller who wants ceilings on a
            // mutation names them at `NativeSparqlEngine::update_governed`.
            governors: None,
            options,
        };
        match eval_update(&update, &mut m, self.resolver.as_deref(), &cfg) {
            Ok(()) => {}
            Err(UpdateAbort::Failed(diagnostic)) => return Err(diagnostic),
            // Unreachable by construction: a trip can only originate from the
            // `GovernorState` this seam declines to build, so there is no governor here to
            // stop anything. Stated as an invariant rather than re-rendered as a
            // diagnostic, because a diagnostic on this arm would be a place for a real
            // trip to hide if the invariant ever broke.
            Err(UpdateAbort::Tripped(tripped)) => unreachable!(
                "the ungoverned UPDATE seam attaches no GovernorState, yet {} was reported",
                tripped.label()
            ),
        }
        *dataset = m.freeze()?;
        Ok(())
    }

    /// A fresh engine with an empty plan cache and no `LOAD` resolver.
    pub fn new() -> Self {
        Self::default()
    }

    /// Install a host `GraphResolver` so SPARQL `LOAD <iri>` can fetch its source.
    /// Without one, LOAD hard-fails (`native-sparql-load-no-resolver`) unless SILENT.
    #[must_use]
    pub fn with_resolver(mut self, resolver: Arc<dyn GraphResolver>) -> Self {
        self.resolver = Some(resolver);
        self
    }

    /// Set the parse-time configuration ([`ParserOptions`]) this engine uses for
    /// every query and update — most notably the extension-function namespace set.
    /// The default set is EMPTY (extension functions off); a deployment whose
    /// queries spell the closed function set under its own namespace (e.g.
    /// `gmeow:heldIn(...)`) supplies that namespace here.
    #[must_use]
    pub fn with_parser_options(mut self, options: ParserOptions) -> Self {
        self.parser_options = options;
        self
    }

    /// Supply the caller's standpoint predicate table (see
    /// [`StandpointPredicates`]): the `accordingTo`/`sharpens` domain predicate
    /// IRIs that `heldIn` and loss-aware `CONSTRUCT` read from the queried
    /// data. There is **no built-in default** — evaluating `heldIn` on an engine
    /// without this configuration is a hard error.
    #[must_use]
    pub fn with_standpoint_predicates(mut self, predicates: StandpointPredicates) -> Self {
        self.standpoint_predicates = Some(predicates);
        self
    }

    /// Supply the caller's loss-declaration vocabulary (see [`LossVocabulary`]):
    /// the `ProjectionLoss`/`lossCode`/`lostReifies` IRIs emitted by loss-aware
    /// `CONSTRUCT` when a reifier is dropped by the template. There is **no
    /// built-in default** — without this configuration loss declarations stay
    /// inactive and a dropped reifier is projected like a plain `CONSTRUCT`.
    #[must_use]
    pub fn with_loss_vocabulary(mut self, vocab: LossVocabulary) -> Self {
        self.loss_vocabulary = Some(vocab);
        self
    }

    /// Set the evaluation-time options threaded into every per-query context.
    /// Production callers should leave the defaults; tests and benchmarks use
    /// this to flip individual measurement seams (e.g. the differential planner-
    /// correctness test forces the structural BGP order).
    #[must_use]
    pub fn with_eval_options(mut self, options: EvalOptions) -> Self {
        self.eval_options = options;
        self
    }

    /// Parse and feasibility-order one request against the property-function registry
    /// that will be in scope for its evaluation.
    ///
    /// **The single point where a registry becomes parse configuration.** A relation is
    /// reachable from SPARQL only if the parser lowered its predicate IRI to a call
    /// node, and the parser does that only for an IRI under a configured
    /// [`ParserOptions::property_fn_namespaces`] entry OR an entry of
    /// [`ParserOptions::property_fn_iris`]. Deriving [`ParserOptions::property_fn_iris`]
    /// here, from the very registry the evaluation will resolve against, is what keeps
    /// the two from drifting: a host cannot register a relation the parser does not
    /// recognize, and cannot configure an IRI whose call resolves against a different
    /// table.
    ///
    /// Registered IRIs go into [`ParserOptions::property_fn_iris`] — EXACT match — and
    /// deliberately never into [`ParserOptions::property_fn_namespaces`] — PREFIX
    /// match. A registry's keys are exact IRIs, not namespaces: folding
    /// `https://example.org/rel/a` in as a prefix would reclassify the unrelated,
    /// merely-same-prefixed data predicate `https://example.org/rel/ab` as a call to
    /// an unregistered relation, which then hard-errors — a previously-working query
    /// breaking with a diagnostic that names the wrong cause. A host that wants a
    /// whole namespace recognized — including IRIs it has deliberately left
    /// unregistered, so that spelling one is a hard error rather than a silent data
    /// triple — still declares that namespace through [`Self::with_parser_options`];
    /// the two sets (caller-declared namespaces, registry-derived exact IRIs) are
    /// unioned, never conflated.
    ///
    /// No registry (or an empty one) contributes nothing, so a query on a host that has
    /// not configured the seam parses under exactly the options it always did.
    fn prepare_for(
        &self,
        query: &str,
        base_iri: Option<&str>,
        relations: &crate::property_fn::PropertyFunctionRegistry,
        aggregates: &crate::agg_fn::AggregateRegistry,
    ) -> Result<Arc<PreparedQuery>, RdfDiagnostic> {
        let options = self.parser_options_for(relations)?;
        self.cache
            .borrow_mut()
            .prepare_with_relations(query, base_iri, &options, relations, aggregates)
    }

    /// This engine's [`ParserOptions`], augmented with `registry`'s exact predicate
    /// IRIs (EXACT match — [`ParserOptions::property_fn_iris`] — never PREFIX; see
    /// [`Self::prepare_for`]'s doc comment for why folding a registered IRI in as a
    /// namespace prefix would hijack an unrelated, merely-same-prefixed data
    /// predicate).
    ///
    /// Shared by [`Self::prepare_for`] (the query lane) and [`Self::parse_update`]
    /// (the UPDATE lane) so a registered relation's predicate is recognized as a
    /// call node identically in a `SELECT` and in an UPDATE's `WHERE` — an UPDATE
    /// WHERE clause is a triple-pattern context exactly like a query's, and a
    /// registry that can drive one but not the other is a registry with two
    /// meanings depending on which clause spelled the predicate.
    ///
    /// `describe()` is IRI-sorted, so the derived set is a pure function of the
    /// registry's contents rather than of its registration order. Returns the
    /// engine's own options unmodified — no clone, no allocation — when `registry`
    /// is absent or empty, which is every request on a host that has not
    /// configured the seam.
    ///
    /// # Errors
    ///
    /// An [`RdfDiagnostic`] (`native-sparql-property-function`) if a registered
    /// relation's declaration methods panic.
    fn parser_options_for(
        &self,
        registry: &crate::property_fn::PropertyFunctionRegistry,
    ) -> Result<Cow<'_, ParserOptions>, RdfDiagnostic> {
        if registry.is_empty() {
            return Ok(Cow::Borrowed(&self.parser_options));
        }
        let mut options = self.parser_options.clone();
        let described = registry
            .describe()
            .map_err(|e| RdfDiagnostic::error("native-sparql-property-function", e.to_string()))?;
        for descriptor in described {
            if !options.property_fn_iris.contains(&descriptor.iri) {
                options.property_fn_iris.push(descriptor.iri);
            }
        }
        Ok(Cow::Owned(options))
    }

    /// Build the per-query evaluation context, threading the engine-level
    /// configuration (order cache + standpoint predicate table + loss vocabulary +
    /// eval options) into it. `NOW()`/`RAND()`/`UUID()`/`STRUUID()` are already
    /// correct by construction: [`EvalCtx::new`] samples the real host wall clock
    /// and OS entropy itself.
    fn eval_ctx<'d, D: DatasetView + Sync>(&'d self, dataset: &'d D) -> EvalCtx<'d, D> {
        let mut ctx = EvalCtx::new(dataset)
            .with_order_cache(&self.order_cache)
            .with_eval_options(self.eval_options);
        if let Some(predicates) = &self.standpoint_predicates {
            ctx = ctx.with_standpoint_predicates(predicates.clone());
        }
        if let Some(vocab) = &self.loss_vocabulary {
            ctx = ctx.with_loss_vocabulary(vocab.clone());
        }
        ctx
    }

    /// Explain what the engine will do with `query_text` against `dataset`, and what it
    /// costs: the cost-based BGP join orders, the per-node charge ledger, the planner's
    /// prediction beside the cardinality that actually materialised, and the identity of
    /// the charge schedule all of it was priced under.
    ///
    /// [`QueryExplanation::join_orders`] is the value this method used to return: for every
    /// BGP with at least two triple patterns, its patterns in the order the planner
    /// selected, with BGPs visited in a left-to-right DFS over the algebra so subqueries,
    /// `OPTIONAL`/`UNION` branches, and `GRAPH` blocks all appear in query-text order.
    ///
    /// # This **evaluates** the query
    ///
    /// It has to. A ledger of what a query cost cannot be derived from its text, and the
    /// planner's error — the one thing an EXPLAIN is for — is only observable by putting
    /// the estimate beside the count. The evaluation runs under
    /// [`QueryGovernors::METERED`], which engages every counter at a ceiling nothing can
    /// reach: the query is measured, never bounded, so the explanation describes the run a
    /// caller would actually get. Engine state is still not mutated (the plan cache's
    /// memoized parse aside, exactly as every other entry point warms it).
    ///
    /// The consequence to expect is that a query which fails to *evaluate* now fails to
    /// explain, with the same diagnostic — a `SERVICE` clause on an engine with no
    /// federation source being the practical case. That is the honest report: there is no
    /// cost to describe for work that cannot be done.
    ///
    /// # Errors
    ///
    /// Returns an [`RdfDiagnostic`] if the query text does not parse, or if evaluating it
    /// fails.
    pub fn explain_query(
        &self,
        dataset: &Arc<RdfDataset>,
        query_text: &str,
        base_iri: Option<&str>,
    ) -> Result<QueryExplanation, RdfDiagnostic> {
        self.explain_query_view(&**dataset, query_text, base_iri)
    }

    /// [`Self::explain_query`] over any [`DatasetView`] backend whose id type is the
    /// production [`TermId`](purrdf_core::TermId). The cost-based join order is computed against the given
    /// view's cardinalities exactly as the concrete path does.
    ///
    /// # Errors
    ///
    /// Returns an [`RdfDiagnostic`] if the query text does not parse, or if evaluating it
    /// fails.
    pub fn explain_query_view<D: DatasetView + Sync>(
        &self,
        dataset: &D,
        query_text: &str,
        base_iri: Option<&str>,
    ) -> Result<QueryExplanation, RdfDiagnostic> {
        self.explain_query_with_options_view(dataset, query_text, base_iri, QueryOptions::EMPTY)
    }

    /// [`Self::explain_query`] with the full [`QueryOptions`] a query evaluation takes —
    /// the SHACL-AF function registry, the property-function registry, AND the
    /// custom-aggregate registry together in one call, so a query that calls into more
    /// than one of them can be explained AT ALL, correctly, in one shot.
    ///
    /// This is the entry [`Self::explain_query`] is defined in terms of, with every
    /// field at [`QueryOptions::EMPTY`]. There is no narrower per-registry explain
    /// entry to fall into instead: a single-registry entry could under-declare a
    /// query that needs more than one registry and silently explain a narrower query
    /// than the one that was asked about (see [`QueryOptions`]'s own documentation),
    /// so every caller holding a property-function registry, a custom-aggregate
    /// registry, or both names them here, exactly as [`Self::query_with_options_view`]
    /// is the non-explain twin that already requires this for [`SparqlEngine::query`]'s
    /// extension seams.
    ///
    /// `options.prebinding` and `options.bnode_mint_prefix` are accepted for shape
    /// parity with [`QueryOptions`] but have no observable effect here: EXPLAIN neither
    /// applies a SHACL pre-binding substitution nor mints blank nodes into a caller-
    /// visible result the receipt reports on.
    ///
    /// # Errors
    ///
    /// Returns an [`RdfDiagnostic`] if the query text does not parse, or if evaluating it
    /// fails.
    pub fn explain_query_with_options(
        &self,
        dataset: &Arc<RdfDataset>,
        query_text: &str,
        base_iri: Option<&str>,
        options: QueryOptions<'_>,
    ) -> Result<QueryExplanation, RdfDiagnostic> {
        self.explain_query_with_options_view(&**dataset, query_text, base_iri, options)
    }

    /// [`Self::explain_query_with_options`] over any [`DatasetView`] backend whose id
    /// type is the production [`TermId`](purrdf_core::TermId).
    ///
    /// # Errors
    ///
    /// Returns an [`RdfDiagnostic`] if the query text does not parse, or if evaluating it
    /// fails.
    pub fn explain_query_with_options_view<D: DatasetView + Sync>(
        &self,
        dataset: &D,
        query_text: &str,
        base_iri: Option<&str>,
        options: QueryOptions<'_>,
    ) -> Result<QueryExplanation, RdfDiagnostic> {
        self.explain_for(
            dataset,
            query_text,
            base_iri,
            options.functions,
            options.property_functions,
            options.aggregates,
        )
    }

    /// The one explain body, parameterized by the SHACL-AF function registry, the
    /// property-function registry, AND the custom-aggregate registry in scope.
    ///
    /// All three are threaded everywhere they change the answer: into the parse (which is
    /// where a relation's predicate becomes a call node and a `Custom` aggregate's IRI is
    /// admitted, and where the plan is feasibility-ordered), into the survey (a relation's
    /// declared row bound is the only prediction a call has — an aggregate contributes no
    /// survey prediction, since it is ordinary algebra rather than an injected row
    /// source), into the evaluation context (so a registered aggregate and a
    /// registered/native user function actually resolve rather than exploding at
    /// `AGG(<iri>, …)`/`<iri>(…)`), and into the receipt (which names the registered
    /// IRIs of the relation and aggregate registries).
    fn explain_for<D: DatasetView + Sync>(
        &self,
        dataset: &D,
        query_text: &str,
        base_iri: Option<&str>,
        functions: &crate::user_fn::UserFunctionRegistry,
        relations: &crate::property_fn::PropertyFunctionRegistry,
        aggregates: &crate::agg_fn::AggregateRegistry,
    ) -> Result<QueryExplanation, RdfDiagnostic> {
        let prepared = self.prepare_for(query_text, base_iri, relations, aggregates)?;
        let survey = self.survey_plan(dataset, &prepared.query, relations)?;
        // The ledger's node table is fixed against the plan that is about to be evaluated.
        // No substitutions are applied on this path, so the addresses the ledger records
        // are the addresses the evaluator visits.
        let ledger = Arc::new(ChargeLedger::for_plan(
            query_pattern(&prepared.query),
            &survey.estimates,
        ));
        let state = Arc::new(GovernorState::new(&QueryGovernors::METERED));
        let mut ctx = self
            .eval_ctx(dataset)
            .with_governors(Arc::clone(&state))
            .with_charge_ledger(Arc::clone(&ledger))
            .with_user_functions(functions)
            .with_property_functions(relations)
            .with_aggregates(aggregates);
        evaluate_query_evaluated(&prepared.query, &mut ctx)
            .map_err(|e| RdfDiagnostic::error("native-sparql-query-explain", e.to_string()))?;
        // `describe()` is IRI-sorted, so the receipt's relation list is a function of what
        // was registered and not of the order it was registered in. The full descriptor
        // travels, not just the IRI: arity, declared modes, and volatility are all part of
        // what a relation IS, and two impls sharing an IRI but disagreeing on any of them
        // must render as two different receipts.
        let registered = relations
            .describe()
            .map_err(|e| RdfDiagnostic::error("native-sparql-property-function", e.to_string()))?;
        // The exact twin, for the custom-aggregate registry.
        let registered_aggregates = aggregates
            .describe()
            .map_err(|e| RdfDiagnostic::error("native-sparql-aggregate-function", e.to_string()))?;
        Ok(QueryExplanation::new(
            survey.orders,
            ledger.snapshot(),
            registered,
            registered_aggregates,
            state.evidence(),
        ))
    }

    /// Walk `query`'s plan against `dataset`'s statistics without evaluating it: the join
    /// order the cost model chooses for every BGP, and the cardinality it predicts.
    ///
    /// One walk feeds both consumers — admission control, which refuses a plan whose
    /// predicted peak already exceeds the caller's ceiling, and the ledger, which prints
    /// that prediction beside the count that materialised.
    ///
    /// `relations` is the caller's property-function registry, when it supplied one. It is
    /// what lets a call node be priced at all: a relation's declared row bound is the only
    /// prediction there is for a bag no index sized.
    fn survey_plan<D: DatasetView + Sync>(
        &self,
        dataset: &D,
        query: &Query,
        relations: &crate::property_fn::PropertyFunctionRegistry,
    ) -> Result<crate::bgp::PlanSurvey, RdfDiagnostic> {
        let _ = self;
        let active_dataset = ActiveDataset::from_query_dataset(query.dataset(), dataset);
        let mut survey = crate::bgp::PlanSurvey::default();
        crate::bgp::survey_pattern_plans(
            dataset,
            &active_dataset,
            GraphMatch::Default,
            query_pattern(query),
            relations,
            &mut survey,
        )
        .map_err(|e| RdfDiagnostic::error("native-sparql-query-explain", e.to_string()))?;
        Ok(survey)
    }

    /// Decide whether `query` may be evaluated at all under `state`'s ceilings, refusing it
    /// when the cost planner already predicts a breach.
    ///
    /// # Why refusing beats reporting
    ///
    /// Every other governor is a *meter*: it observes consumption and stops the execution
    /// once a ceiling is crossed, which means the work that crossed it has already been
    /// done. For fuel or a row count that is fine — the overshoot is one charge point. For
    /// the intermediate-cardinality ceiling it is not, because the work that crosses it is
    /// a single allocation of the bag that crossed it: by the time the meter can report a
    /// materialized cross product, the cross product is in memory. On `wasm32` the
    /// distinction is total rather than merely uncomfortable — an allocation trap aborts
    /// the module, so there is no execution left to return a [`GovernedOutcome`] at all.
    /// Admission is the only mechanism that can act before that point.
    ///
    /// # Determinism, and what an estimate is allowed to claim
    ///
    /// The decision is a pure function of the query as written, the dataset's cardinality
    /// statistics, and the ceiling. The cost model probes the same statistics the join
    /// planner probes, composes them with order-stable arithmetic, and the plan walk visits
    /// nodes in algebra order — so the same query over the same data under the same ceiling
    /// is refused, or admitted, identically on every run and every machine. Nothing here
    /// reads a clock, a thread count, or a hash-map iteration order.
    ///
    /// "As written" is exact: pre-binding substitutions are applied *after* this decision,
    /// so the estimate is the un-substituted plan's. That direction is the safe one — a
    /// substitution can only bind a variable, never free one, so the plan it produces is no
    /// larger than the one priced here — and it is also what keeps the decision independent
    /// of which substitutions a caller happened to pass. The cost is that a heavily
    /// pre-bound query can be refused on its unbound shape; the live ceiling would then have
    /// admitted it, and raising the ceiling to the reported estimate is what gets it run.
    ///
    /// An estimate is an estimate, so the refusal is stated as one: it is reported as
    /// [`TrippedGovernor::Refused`](purrdf_core::TrippedGovernor::Refused), which carries
    /// the *estimate* rather than a consumption it never measured, and it is never a
    /// completeness claim — the outcome is a budget-exhausted one whose certified partial
    /// answer is empty for row- and graph-producing forms. A refused `ASK` is unknown:
    /// its materialized `false` shape cannot soundly represent an unsettled boolean. An
    /// over-estimate therefore costs a caller an answer; it cannot hand them a wrong one.
    /// An under-estimate changes nothing: the live ceiling is still in force and still
    /// trips.
    fn admit<D: DatasetView + Sync>(
        &self,
        dataset: &D,
        query: &Query,
        relations: &crate::property_fn::PropertyFunctionRegistry,
        state: &GovernorState,
        identity: &RelationIdentity,
    ) -> Option<Result<GovernedOutcome, RdfDiagnostic>> {
        let dimension = purrdf_core::ResourceDimension::IntermediateCells;
        if !state.is_engaged_in(dimension) {
            return None;
        }
        let limit = state.limits().get(dimension);
        let estimate = match self.survey_plan(dataset, query, relations) {
            Ok(survey) => survey.peak_cells(),
            Err(diagnostic) => return Some(Err(diagnostic)),
        };
        if estimate <= limit {
            return None;
        }
        // Latched through the same door an evaluator-side trip uses, so a stop signal that
        // was already firing outranks the refusal exactly as `resolve_precedence` says it
        // should, and the evidence reports one governor rather than two.
        let tripped = state.record_trip(purrdf_core::TrippedGovernor::Refused {
            dimension,
            limit,
            estimate,
        });
        Some(Ok(GovernedOutcome::BudgetExhausted(BudgetExhausted {
            tripped,
            evidence: state.evidence(),
            relations: identity.clone(),
            // For SELECT and graph forms this is a certified lower bound over nothing:
            // every item here is an answer, and there are none. ASK has no public witness
            // set, however; its `false` value would read as a settled answer, so the helper
            // withholds it as unknown.
            partial: certain_partial(empty_result_for(query), true),
        })))
    }

    /// The one **ungoverned** options-carrying query entry, parameterized by
    /// [`QueryOptions`]: selects the substitution rewrite
    /// ([`QueryOptions::prebinding`]), optionally injects a SHACL-AF function
    /// registry and a property-function registry, and optionally installs a
    /// deterministic blank-mint prefix ([`EvalCtx::with_bnode_mint_prefix`]) so every
    /// blank this evaluation mints carries a caller-supplied identity — the seam the
    /// SHACL rules engine uses to make distinct focus nodes mint distinct `CONSTRUCT`
    /// blanks at mint time.
    ///
    /// This is the ungoverned sibling of [`Self::query_governed_in_operation`], and
    /// the body every narrower ungoverned entry above delegates to. Under
    /// [`QueryOptions::EMPTY`] it behaves exactly like [`SparqlEngine::query`].
    ///
    /// # Errors
    ///
    /// Propagates parse/evaluation errors as an [`RdfDiagnostic`].
    pub fn query_with_options_view<'d, D: DatasetView + Sync>(
        &'d self,
        dataset: &'d D,
        request: SparqlRequest<'_>,
        options: QueryOptions<'d>,
    ) -> Result<SparqlResult, RdfDiagnostic> {
        let prepared = self.prepare_for(
            request.query,
            request.base_iri,
            options.property_functions,
            options.aggregates,
        )?;
        let ctx = self.eval_ctx(dataset);
        let mut ctx = apply_query_options(ctx, options)?;
        let outcome = match options.prebinding {
            ShaclPrebinding::Applied => {
                evaluate_with_shacl_prebinding(&prepared, request.substitutions, &mut ctx)?
            }
            ShaclPrebinding::None => {
                evaluate_with_substitutions(&prepared, request.substitutions, &mut ctx)?
            }
        };
        Ok(materialize(outcome, &ctx))
    }

    /// Like [`SparqlEngine::query`], but with a
    /// [`RemoteQuerySource`](crate::remote::RemoteQuerySource) injected so
    /// `SERVICE` clauses resolve through it. Without this, the default
    /// [`SparqlEngine::query`] path has no source and a non-silent `SERVICE`
    /// hard-fails. This is the public entry the conformance harness and
    /// federated callers use.
    ///
    /// # Options are a parameter, not an overload
    ///
    /// `options` carries the registries and rewrites this execution runs under —
    /// pass [`QueryOptions::EMPTY`] to configure none — for the identical reason
    /// [`Self::query_governed`] requires it: a property-function registry is *parse*
    /// configuration, so an entry that could not be handed one would parse a
    /// registered relation's predicate as an ordinary triple pattern and answer the
    /// empty bag, silently, for every call whose `SERVICE` body is federated. The
    /// query's OUTER pattern resolves against `options.property_functions` exactly
    /// as [`Self::query_with_options_view`] resolves it; a call node inside the
    /// `SERVICE` body itself is refused at forwarding regardless (see
    /// [`crate::remote::RemoteQuerySource`]).
    ///
    /// # Errors
    ///
    /// Propagates parse and evaluation errors as an [`RdfDiagnostic`].
    pub fn query_with_source(
        &self,
        dataset: &Arc<RdfDataset>,
        request: SparqlRequest<'_>,
        source: &(dyn crate::remote::RemoteQuerySource + Sync),
        options: QueryOptions<'_>,
    ) -> Result<SparqlResult, RdfDiagnostic> {
        self.query_with_source_view(&**dataset, request, source, options)
    }

    /// [`Self::query_with_source`] over any [`DatasetView`] backend whose id type is
    /// the production [`TermId`](purrdf_core::TermId).
    ///
    /// # Errors
    ///
    /// Propagates parse and evaluation errors as an [`RdfDiagnostic`].
    pub fn query_with_source_view<'d, D: DatasetView + Sync>(
        &'d self,
        dataset: &'d D,
        request: SparqlRequest<'_>,
        source: &'d (dyn crate::remote::RemoteQuerySource + Sync),
        options: QueryOptions<'d>,
    ) -> Result<SparqlResult, RdfDiagnostic> {
        let prepared = self.prepare_for(
            request.query,
            request.base_iri,
            options.property_functions,
            options.aggregates,
        )?;
        let ctx = self.eval_ctx(dataset).with_remote(source);
        let mut ctx = apply_query_options(ctx, options)?;
        let outcome = match options.prebinding {
            ShaclPrebinding::Applied => {
                evaluate_with_shacl_prebinding(&prepared, request.substitutions, &mut ctx)?
            }
            ShaclPrebinding::None => {
                evaluate_with_substitutions(&prepared, request.substitutions, &mut ctx)?
            }
        };
        Ok(materialize(outcome, &ctx))
    }
}

fn preflight_fallible_view<D>(dataset: &D) -> Result<(), FallibleSparqlError<D::Error, D::Evidence>>
where
    D: FallibleDatasetView + Sync,
{
    match dataset.operation_status() {
        ViewOperationStatus::Ready { .. } => Ok(()),
        ViewOperationStatus::Failed { error, evidence } => {
            Err(FallibleSparqlError::Operational { error, evidence })
        }
    }
}

fn finish_fallible_query<D>(
    dataset: &D,
    evaluation: Result<SparqlResult, RdfDiagnostic>,
) -> FallibleSparqlResult<D::Error, D::Evidence>
where
    D: FallibleDatasetView + Sync,
{
    match dataset.operation_status() {
        ViewOperationStatus::Failed { error, evidence } => {
            Err(FallibleSparqlError::Operational { error, evidence })
        }
        ViewOperationStatus::Ready { evidence } => match evaluation {
            Ok(result) => Ok(CompleteSparqlResult { result, evidence }),
            Err(diagnostic) => Err(FallibleSparqlError::Query {
                diagnostic,
                evidence,
            }),
        },
    }
}

/// Report a governed query over a fallible view at its final checkpoint.
///
/// The precedence is the ungoverned lane's, extended by one rule: an operational failure
/// still outranks everything derived after data became unavailable — including a governor
/// trip, because a budget that ran out while the data was already gone says nothing about
/// the budget.
#[allow(
    clippy::result_large_err,
    reason = "reports the same typed outcome its caller returns; see \
              `NativeSparqlEngine::query_governed_fallible_view`"
)]
fn finish_governed_fallible_query<D>(
    dataset: &D,
    state: &GovernorState,
    evaluation: Result<GovernedOutcome, RdfDiagnostic>,
) -> FallibleSparqlResult<D::Error, GovernedEvidence<D::Evidence>>
where
    D: FallibleDatasetView + Sync,
{
    let governors = state.evidence();
    match dataset.operation_status() {
        ViewOperationStatus::Failed { error, evidence } => Err(FallibleSparqlError::Operational {
            error,
            evidence: GovernedEvidence::new(evidence, governors),
        }),
        ViewOperationStatus::Ready { evidence } => {
            let evidence = GovernedEvidence::new(evidence, governors);
            match evaluation {
                // The governor evidence inside the complete outcome is the same snapshot
                // as the one paired above, so it is read from one place rather than
                // carried twice.
                Ok(GovernedOutcome::Complete { result, .. }) => {
                    Ok(CompleteSparqlResult { result, evidence })
                }
                Ok(GovernedOutcome::BudgetExhausted(exhausted)) => {
                    Err(FallibleSparqlError::BudgetExhausted {
                        tripped: exhausted.tripped,
                        partial: exhausted.partial,
                        evidence,
                    })
                }
                Err(diagnostic) => Err(FallibleSparqlError::Query {
                    diagnostic,
                    evidence,
                }),
            }
        }
    }
}

/// Evaluate `prepared`, applying any pre-binding `substitutions` first (GAP-A).
///
/// When there are no substitutions the cached parse is evaluated directly (the hot
/// path). Otherwise the cached parse is **cloned** and rewritten — the substitution
/// must never poison the shared, un-substituted plan-cache entry.
fn evaluate_with_substitutions<D: DatasetView + Sync>(
    prepared: &PreparedQuery,
    substitutions: &[(String, TermValue)],
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Outcome<D::Id>, RdfDiagnostic> {
    let eval_err = |e: crate::error::EvalError| {
        RdfDiagnostic::error("native-sparql-query-eval", e.to_string())
    };
    if substitutions.is_empty() {
        return evaluate_query(&prepared.query, ctx).map_err(eval_err);
    }
    let substituted =
        crate::substitute::apply_substitutions(prepared.query.clone(), substitutions)?;
    evaluate_query(&substituted, ctx).map_err(eval_err)
}

/// [`evaluate_with_substitutions`], on the trip-aware channel.
///
/// The one difference is the evaluator entry point: this one may answer "a governor
/// stopped here, and these rows are what it left", which the completion-only
/// [`evaluate_query`] refuses by contract.
fn evaluate_governed_with_substitutions<D: DatasetView + Sync>(
    prepared: &PreparedQuery,
    substitutions: &[(String, TermValue)],
    ctx: &mut EvalCtx<'_, D>,
) -> Result<EvaluatedOutcome<D::Id>, RdfDiagnostic> {
    let eval_err = |e: crate::error::EvalError| {
        RdfDiagnostic::error("native-sparql-query-eval", e.to_string())
    };
    if substitutions.is_empty() {
        return evaluate_query_evaluated(&prepared.query, ctx).map_err(eval_err);
    }
    let substituted =
        crate::substitute::apply_substitutions(prepared.query.clone(), substitutions)?;
    evaluate_query_evaluated(&substituted, ctx).map_err(eval_err)
}

/// Which rewrite [`NativeSparqlEngine::query_governed_in_operation`] applies to a
/// request's substitutions before evaluating it.
///
/// A named two-state enum rather than a `bool`, because the two rewrites are not "on and
/// off" — they are different SPARQL semantics (SHACL §5.3's pre-binding versus the
/// ordinary substitution path), and a call site reading `true` says neither of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaclPrebinding {
    /// Apply the SHACL pre-binding rewrite: `sh:sparql` constraint and component bodies,
    /// `sh:SPARQLRule`, and `sh:ask`/`sh:select` validators.
    Applied,
    /// Apply the ordinary substitution rewrite: SHACL-AF node expressions,
    /// `sh:SPARQLTarget`, and every non-SHACL caller.
    None,
}

/// The per-call configuration every options-carrying query entry takes — governed
/// and ungoverned, SHACL-facing and not.
///
/// Bundles the four independently optional pieces a query evaluation selects from:
/// which substitution rewrite to apply, whether a SHACL-AF function registry is in
/// scope, whether a **property-function registry** is in scope, and whether minted
/// blank-node labels carry a deterministic caller-supplied prefix (see
/// [`EvalCtx::with_bnode_mint_prefix`](crate::eval::EvalCtx::with_bnode_mint_prefix)).
///
/// # Why one struct on every entry rather than a registry-free sibling per lane
///
/// The property-function registry is not merely an evaluation-time table: it is
/// *parse* configuration — a registered IRI becomes a call node only because the
/// registry contributed it to
/// [`ParserOptions::property_fn_iris`](purrdf_sparql_algebra::ParserOptions). An entry that
/// cannot be handed one parses a registered relation's predicate as an ORDINARY
/// triple pattern, which matches no data and returns an empty bag — a wrong answer
/// with no diagnostic. Making the options a required parameter of every entry means
/// a host that registered relations cannot reach an entry that would silently drop
/// them: there is no registry-free overload to fall into. Callers with nothing to
/// configure pass [`QueryOptions::EMPTY`], which is exactly the behavior those
/// entries had before the seam existed.
///
/// Construct with struct-update syntax over the empty value, e.g.
/// `QueryOptions { property_functions: &registry, ..QueryOptions::EMPTY }`.
#[derive(Debug, Clone, Copy)]
pub struct QueryOptions<'a> {
    /// Which substitution rewrite to apply (see [`ShaclPrebinding`]).
    pub prebinding: ShaclPrebinding,
    /// The SHACL-AF function registry in scope.
    /// [`UserFunctionRegistry::EMPTY`](crate::user_fn::UserFunctionRegistry::EMPTY) —
    /// the default — behaves exactly like the registry-free entries; there is no
    /// separate "no registry" spelling to disagree with it.
    pub functions: &'a crate::user_fn::UserFunctionRegistry,
    /// The property-function registry in scope.
    /// [`PropertyFunctionRegistry::EMPTY`](crate::property_fn::PropertyFunctionRegistry::EMPTY) —
    /// the default — behaves exactly like every other empty registry (see
    /// [`EvalCtx::with_property_functions`](crate::eval::EvalCtx::with_property_functions));
    /// there is no separate "no registry" spelling to disagree with it.
    pub property_functions: &'a crate::property_fn::PropertyFunctionRegistry,
    /// The custom-aggregate registry in scope.
    /// [`AggregateRegistry::EMPTY`](crate::agg_fn::AggregateRegistry::EMPTY) — the
    /// default — behaves exactly like every other empty registry (see
    /// [`EvalCtx::with_aggregates`](crate::eval::EvalCtx::with_aggregates)); there
    /// is no separate "no registry" spelling to disagree with it. Like
    /// [`Self::property_functions`], this is *admission* configuration as much as
    /// evaluation configuration: an `AggregateFunction::Custom(iri)` call is
    /// admitted (registered, correct arity) against THIS registry at prepare time
    /// (see `crate::property_fn_plan::plan_aggregate`), and the prepared plan is
    /// refused at evaluation if a different registry is supplied later (see
    /// `check_plan_matches_relations`).
    pub aggregates: &'a crate::agg_fn::AggregateRegistry,
    /// A deterministic prefix for every blank-node label the evaluation mints;
    /// `None` (the pre-existing behavior) leaves minted labels unprefixed. The
    /// prefix is caller-supplied data — the SHACL rules engine passes a
    /// per-focus-node identity tag — and must never be derived from time, RNG,
    /// or iteration order.
    pub bnode_mint_prefix: Option<&'a str>,
}

impl QueryOptions<'_> {
    /// Configure nothing: the ordinary substitution rewrite, every registry the
    /// canonical empty value, unprefixed blank mints. What every entry did before
    /// it took options.
    pub const EMPTY: Self = Self {
        prebinding: ShaclPrebinding::None,
        functions: &crate::user_fn::UserFunctionRegistry::EMPTY,
        property_functions: &crate::property_fn::PropertyFunctionRegistry::EMPTY,
        aggregates: &crate::agg_fn::AggregateRegistry::EMPTY,
        bnode_mint_prefix: None,
    };
}

impl Default for QueryOptions<'_> {
    fn default() -> Self {
        Self::EMPTY
    }
}

/// Refuse a plan that was prepared against a different property-function registry, OR a
/// different custom-aggregate registry, than the ones it is about to be evaluated under.
///
/// The last way to reach a silently-empty answer from a registered relation, closed. A
/// plan prepared with no registry lowered the relation's predicate to an ORDINARY triple
/// pattern; evaluating it with the registry attached would scan a dataset that holds no
/// such triple and answer the empty bag, with every governor reporting a clean complete
/// run. The plan carries the identity it was parsed under
/// ([`PreparedQuery::relations`]), so the disagreement is a fact rather than an
/// inference, and it is reported as a configuration error — which is what it is.
///
/// The identical hazard exists for a custom aggregate — admitted at prepare time (its
/// registration and arity checked, see `crate::property_fn_plan::plan_aggregate`) against
/// [`PreparedQuery::aggregates`] — but with a SHARPER failure mode than an unregistered
/// relation's: two DIFFERENT registries can both resolve the SAME IRI to two DIFFERENT
/// aggregates, so a plan admitted under registry A and evaluated under registry B would
/// not fail loudly at all — it would silently compute registry B's aggregate's answer
/// under a plan that was only ever checked against registry A's arity. Checking here
/// closes exactly that: a prepared plan is admitted under registry A is never silently
/// executed under registry B, for either registry.
///
/// The function-registry pair ([`QueryOptions::functions`]) is deliberately NOT checked
/// here: a mismatched `UserFunctionRegistry` has no analogous silent-wrong-answer channel
/// — `Function::Custom` is resolved dynamically at evaluation time (never at parse time,
/// unlike a property-function predicate or a `Custom` aggregate's admission), so an
/// IRI unknown to the supplied registry fails LOUDLY there (an XSD-cast attempt or a typed
/// "undefined function" error), and a resolved-but-different-under-registry-B function
/// call is exactly the risk every caller of [`QueryOptions`] already accepts by supplying
/// registries at every governed call rather than once at prepare time.
///
/// # Errors
///
/// An [`RdfDiagnostic`] (`native-sparql-property-function` or
/// `native-sparql-aggregate-function`) when either identity differs, or when a relation or
/// an aggregate panics while its declaration is read to compute the supplied registry's
/// fingerprint.
fn check_plan_matches_relations(
    prepared: &PreparedQuery,
    options: QueryOptions<'_>,
) -> Result<(), RdfDiagnostic> {
    let supplied = crate::property_fn_plan::registry_fingerprint(options.property_functions)
        .map_err(|e| RdfDiagnostic::error("native-sparql-property-function", e.to_string()))?;
    if supplied != prepared.relations {
        return Err(RdfDiagnostic::error(
            "native-sparql-property-function",
            "this plan was prepared against a different property-function registry than the \
             one supplied for its evaluation; prepare it with \
             `NativeSparqlEngine::prepare_query_with_options` under the SAME `QueryOptions` the \
             evaluation uses, because the registry is what decides which predicates are calls",
        ));
    }
    let supplied_aggregates = crate::agg_fn::registry_fingerprint(options.aggregates)
        .map_err(|e| RdfDiagnostic::error("native-sparql-aggregate-function", e.to_string()))?;
    if supplied_aggregates != prepared.aggregates {
        return Err(RdfDiagnostic::error(
            "native-sparql-aggregate-function",
            "this plan was prepared against a different custom-aggregate registry than the \
             one supplied for its evaluation; prepare it with \
             `NativeSparqlEngine::prepare_query_with_options` under the SAME `QueryOptions` the \
             evaluation uses, because the registry is what a `Custom` aggregate IRI resolves \
             against",
        ));
    }
    Ok(())
}

/// The [`RelationIdentity`] a governed outcome carries: `prepared`'s already-computed
/// registry fingerprint — validated against `relations` by
/// [`check_plan_matches_relations`] at every call site before this runs, so it is safe to
/// reuse rather than re-derive — paired with the registered IRIs the fingerprint was
/// taken over, sorted.
///
/// Called once per governed execution rather than per outcome arm, so the refusal path
/// ([`NativeSparqlEngine::admit`]), the complete path, and the truncated path all carry
/// the SAME identity for the SAME execution — there is exactly one place this is computed
/// and every [`GovernedOutcome`] arm reads from it.
///
/// # Errors
///
/// An [`RdfDiagnostic`] (`native-sparql-property-function`) if a registered relation's
/// declaration methods panic.
fn relation_identity(
    prepared: &PreparedQuery,
    relations: &crate::property_fn::PropertyFunctionRegistry,
) -> Result<RelationIdentity, RdfDiagnostic> {
    let iris = if relations.is_empty() {
        Vec::new()
    } else {
        relations
            .describe()
            .map_err(|e| RdfDiagnostic::error("native-sparql-property-function", e.to_string()))?
            .into_iter()
            .map(|descriptor| descriptor.iri)
            .collect()
    };
    Ok(RelationIdentity {
        fingerprint: prepared.relations.clone(),
        iris,
    })
}

/// Apply the [`QueryOptions`] pieces to a freshly built evaluation context: the
/// SHACL-AF function registry ([`EvalCtx::with_user_functions`]), the
/// property-function registry ([`EvalCtx::with_property_functions`]), the
/// custom-aggregate registry ([`EvalCtx::with_aggregates`]), and the deterministic
/// blank-mint prefix ([`EvalCtx::with_bnode_mint_prefix`]) — the first three
/// unconditionally (each is always a valid registry reference now, `EMPTY` standing
/// in for "none" rather than an `Option` this function would need to branch on).
///
/// The ONE application seam — every options-carrying entry, governed and ungoverned,
/// routes through here, so a future [`QueryOptions`] field is wired once instead of
/// at each call site. [`QueryOptions::prebinding`] is deliberately not applied here:
/// it selects an *evaluator entry point* rather than a context field, so it is read
/// at the two evaluation sites instead.
///
/// # Errors
///
/// Returns [`RdfDiagnostic`] when `options.bnode_mint_prefix` is not a legal
/// `BLANK_NODE_LABEL` prefix.
pub(crate) fn apply_query_options<'d, D: DatasetView + Sync>(
    mut ctx: EvalCtx<'d, D>,
    options: QueryOptions<'d>,
) -> Result<EvalCtx<'d, D>, RdfDiagnostic> {
    ctx = ctx
        .with_user_functions(options.functions)
        .with_property_functions(options.property_functions)
        .with_aggregates(options.aggregates);
    if let Some(prefix) = options.bnode_mint_prefix {
        ctx = ctx
            .with_bnode_mint_prefix(prefix)
            .map_err(|e| RdfDiagnostic::error("native-sparql-bnode-mint-prefix", e.to_string()))?;
    }
    Ok(ctx)
}

/// [`evaluate_with_shacl_prebinding`], on the trip-aware channel — the same relationship
/// [`evaluate_governed_with_substitutions`] has to [`evaluate_with_substitutions`].
fn evaluate_governed_with_shacl_prebinding<D: DatasetView + Sync>(
    prepared: &PreparedQuery,
    substitutions: &[(String, TermValue)],
    ctx: &mut EvalCtx<'_, D>,
) -> Result<EvaluatedOutcome<D::Id>, RdfDiagnostic> {
    let substituted =
        crate::substitute::apply_shacl_prebinding(prepared.query.clone(), substitutions)?;
    evaluate_query_evaluated(&substituted, ctx)
        .map_err(|e| RdfDiagnostic::error("native-sparql-query-eval", e.to_string()))
}

fn evaluate_with_shacl_prebinding<D: DatasetView + Sync>(
    prepared: &PreparedQuery,
    substitutions: &[(String, TermValue)],
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Outcome<D::Id>, RdfDiagnostic> {
    let substituted =
        crate::substitute::apply_shacl_prebinding(prepared.query.clone(), substitutions)?;
    evaluate_query(&substituted, ctx)
        .map_err(|e| RdfDiagnostic::error("native-sparql-query-eval", e.to_string()))
}

impl SparqlEngine for NativeSparqlEngine {
    type Dataset = Arc<RdfDataset>;

    fn query(
        &self,
        dataset: &Self::Dataset,
        request: SparqlRequest<'_>,
    ) -> Result<SparqlResult, RdfDiagnostic> {
        let prepared = self.prepare_query(request.query, request.base_iri)?;
        self.query_prepared(
            dataset,
            &prepared,
            request.substitutions,
            QueryOptions::EMPTY,
        )
    }

    fn update(
        &self,
        dataset: &mut Self::Dataset,
        request: SparqlRequest<'_>,
    ) -> Result<(), RdfDiagnostic> {
        // The trait seam configures nothing: a caller who wants a property-function
        // registry (or any other `QueryOptions` piece) reachable from an UPDATE's
        // `WHERE` names it at `NativeSparqlEngine::update_with_options`.
        self.update_with_options(dataset, request, QueryOptions::EMPTY)
    }
}

/// Materialize an evaluation [`Outcome`] into the dataset-independent
/// `SparqlResult` egress model (the interned-id space ends here: every solution
/// cell becomes an owned [`TermValue`](purrdf_core::TermValue)).
fn materialize<D: DatasetView + Sync>(
    outcome: Outcome<D::Id>,
    ctx: &EvalCtx<'_, D>,
) -> SparqlResult {
    match outcome {
        Outcome::Solutions(seq) => {
            let (variables, rows) = crate::eval::materialize_solutions(&seq, ctx);
            let aux = ctx.constructed_dataset(&rows);
            SparqlResult::Solutions {
                variables,
                rows,
                aux,
            }
        }
        Outcome::Graph(graph) => SparqlResult::Graph(graph),
        Outcome::Boolean(value) => SparqlResult::Boolean(value),
    }
}

/// A frozen dataset with nothing in it — the auxiliary graph of a solution set that
/// invented no terms, and the whole result of a graph-producing query that did not run.
fn empty_dataset() -> Arc<RdfDataset> {
    purrdf_core::RdfDatasetBuilder::new()
        .freeze()
        .expect("an empty dataset is positionally valid")
}

/// The empty result of `query`'s form — what an execution that produced nothing produced.
///
/// Shaped by form rather than always `Solutions`, so a caller matching on the result of a
/// refused `CONSTRUCT` finds the `Graph` arm it would find on every other path. The
/// variable list is empty because a refused query never chose one: this engine fixes a
/// solution's column ORDER during evaluation (a BGP's columns appear in the order the
/// cost-based join order visits them), so naming columns for a plan that was never run
/// would be a guess — the same reason, stated in [`crate::governor`], that a truncated
/// binary operator reports its left arm's columns and no more.
fn empty_result_for(query: &Query) -> SparqlResult {
    match query {
        Query::Select { .. } => SparqlResult::Solutions {
            variables: Vec::new(),
            rows: Vec::new(),
            aux: empty_dataset(),
        },
        Query::Ask { .. } => SparqlResult::Boolean(false),
        Query::Construct { .. } | Query::Describe { .. } => SparqlResult::Graph(empty_dataset()),
    }
}

/// Restate a certified lower bound without forging a settled `ASK false` answer.
///
/// Row and graph results have a useful empty lower bound. A boolean result does not expose
/// the witness rows that make that interpretation possible: `false` is the complete
/// negative answer in [`SparqlResult`], so emitting it under [`PartialAnswers::Certain`]
/// would let a stopped search deny an answer it had not reached. `ASK true` remains a
/// certain lower bound because one reached witness settles the boolean positively.
fn certain_partial(result: SparqlResult, positional_prefix: bool) -> PartialAnswers {
    if matches!(result, SparqlResult::Boolean(false)) {
        PartialAnswers::Unknown(NonMonotoneBarrier::named("ask-unsettled"))
    } else {
        PartialAnswers::Certain(PartialSparqlResult::new(result, positional_prefix))
    }
}

/// Materialize a trip-aware [`EvaluatedOutcome`] into the public [`GovernedOutcome`].
///
/// This is the boundary the whole governed surface is built around. Two things happen here
/// and nowhere else:
///
/// 1. **The rows are materialized while `ctx` is still alive.** A partial row may bind a
///    term minted into this execution's scratch arena, which dies with the context, so the
///    partial answers are turned into owned [`TermValue`]s here through the very same
///    [`materialize`] a complete result goes through. Nothing dataset-dependent crosses.
/// 2. **The certificate is restated in the egress vocabulary.** The evaluator's internal
///    three-way classification maps one-for-one onto [`PartialAnswers`], so the public
///    claim is the analysis's claim rather than a second, hand-maintained reading of it.
fn materialize_governed<D: DatasetView + Sync>(
    evaluated: EvaluatedOutcome<D::Id>,
    ctx: &EvalCtx<'_, D>,
    state: &GovernorState,
    relations: RelationIdentity,
) -> GovernedOutcome {
    match evaluated {
        EvaluatedOutcome::Complete(outcome) => {
            let result = materialize(outcome, ctx);
            let evidence = state.evidence();
            match evidence.tripped {
                None => GovernedOutcome::Complete {
                    result,
                    evidence,
                    relations,
                },
                Some(tripped) => GovernedOutcome::BudgetExhausted(BudgetExhausted {
                    tripped,
                    evidence,
                    relations,
                    partial: certain_partial(result, true),
                }),
            }
        }
        EvaluatedOutcome::Truncated {
            outcome,
            certificate,
        } => {
            let partial = match certificate.bound() {
                // No bound survived, so no row may cross and there is nothing to
                // materialize: the actionable half is the operator that withheld them. A
                // collapsed bound is only ever reached through the ascent that records
                // that operator at the same moment it collapses the class, so the two are
                // one fact and cannot disagree.
                SpineClass::Unknown => PartialAnswers::Unknown(
                    certificate
                        .barrier()
                        .expect("a collapsed bound names the operator that collapsed it"),
                ),
                bound => {
                    let result = materialize(outcome, ctx);
                    if bound == SpineClass::Certain {
                        certain_partial(result, certificate.is_positional_prefix())
                    } else {
                        PartialAnswers::AtMost(PartialSparqlResult::new(
                            result,
                            certificate.is_positional_prefix(),
                        ))
                    }
                }
            };
            GovernedOutcome::BudgetExhausted(BudgetExhausted {
                tripped: certificate.tripped(),
                evidence: state.evidence(),
                relations,
                partial,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_core::{BlankScope, RdfDatasetBuilder, RdfLiteral, TermValue};
    use purrdf_sparql_algebra::GraphPattern;

    /// Regression: `=` is RDFterm-equality, so `?a != ?b` over two *distinct IRIs*
    /// must be `true` (the row survives), NOT a type error. Routing `=` through the
    /// ordering comparator made every distinct-IRI `!=` evaluate to an error and drop
    /// the row, so this triangle+FILTER query (the LOGIC `non-entailment-counterpart`
    /// verify) wrongly returned 0 rows. See `expr::equal`.
    #[test]
    fn neq_on_distinct_iris_is_true_not_error() {
        // A→B→C plus the forbidden transitive A→C, all purrdf:counterpartOf.
        let mut b = RdfDatasetBuilder::new();
        let cp = b.intern_iri("http://ex/cp");
        let a = b.intern_iri("http://ex/a");
        let bn = b.intern_iri("http://ex/b");
        let c = b.intern_iri("http://ex/c");
        b.push_quad(a, cp, bn, None);
        b.push_quad(bn, cp, c, None);
        b.push_quad(a, cp, c, None);
        let ds = b.freeze().expect("freeze");
        let q = "PREFIX ex: <http://ex/>\n\
                 SELECT ?a ?b ?c WHERE {\n\
                   ?a ex:cp ?b . ?b ex:cp ?c . ?a ex:cp ?c .\n\
                   FILTER(?a != ?b && ?b != ?c && ?a != ?c)\n\
                 } ORDER BY ?a ?b ?c";
        match run_on(&ds, q) {
            SparqlResult::Solutions { rows, .. } => {
                // The forbidden transitive triangle (a,b,c) is the one violating row.
                assert_eq!(rows.len(), 1, "expected exactly the A,B,C row: {rows:?}");
            }
            other => panic!("expected solutions, got {other:?}"),
        }
        // Direct check: `!=` on two distinct IRIs is TRUE, not an error → the row survives.
        match run_on(
            &ds,
            "PREFIX ex: <http://ex/>\n\
             SELECT ?a ?b WHERE { ?a ex:cp ?b . FILTER(?a != ?b) }",
        ) {
            SparqlResult::Solutions { rows, .. } => {
                assert_eq!(
                    rows.len(),
                    3,
                    "all three distinct-IRI edges survive `!=`: {rows:?}"
                );
            }
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    fn social() -> Arc<RdfDataset> {
        // :a :knows :b ; :a :name "Ann" .
        let mut b = RdfDatasetBuilder::new();
        let knows = b.intern_iri("http://ex/knows");
        let name = b.intern_iri("http://ex/name");
        let a = b.intern_iri("http://ex/a");
        let bb = b.intern_iri("http://ex/b");
        let ann = b.intern_literal(RdfLiteral::simple("Ann"));
        b.push_quad(a, knows, bb, None);
        b.push_quad(a, name, ann, None);
        b.freeze().expect("freeze")
    }

    fn run(query: &str) -> SparqlResult {
        let ds = social();
        let engine = NativeSparqlEngine::new();
        engine
            .query(
                &ds,
                SparqlRequest {
                    query,
                    base_iri: None,
                    substitutions: &[],
                },
            )
            .expect("query")
    }

    // ── substitution / pre-binding (GAP-A) ────────────────────────────────

    /// A dataset for substitution tests:
    ///   :a   :p  :x    (IRI subject)
    ///   :b   :p  :y
    ///   _:bn :p  :z    (blank-node subject — a SHACL blank focus)
    fn subst_ds() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let p = b.intern_iri("http://ex/p");
        let a = b.intern_iri("http://ex/a");
        let bb = b.intern_iri("http://ex/b");
        let x = b.intern_iri("http://ex/x");
        let y = b.intern_iri("http://ex/y");
        let z = b.intern_iri("http://ex/z");
        let bn = b.intern_blank("bn", BlankScope::DEFAULT);
        b.push_quad(a, p, x, None);
        b.push_quad(bb, p, y, None);
        b.push_quad(bn, p, z, None);
        b.freeze().expect("freeze")
    }

    /// Run `query` with `substitutions` and return the sorted first-column debug
    /// strings of the SELECT result.
    fn run_subst(query: &str, substitutions: &[(String, TermValue)]) -> Vec<String> {
        let ds = subst_ds();
        let engine = NativeSparqlEngine::new();
        let result = engine
            .query(
                &ds,
                SparqlRequest {
                    query,
                    base_iri: None,
                    substitutions,
                },
            )
            .expect("query");
        col0(result)
    }

    #[test]
    fn substitute_iri_focus_constrains_the_subject() {
        // `$this :p ?o` with $this := :a must bind ?o to ONLY :x (not :y/:z).
        let got = run_subst(
            "SELECT ?o WHERE { ?this <http://ex/p> ?o }",
            &[("this".to_owned(), TermValue::Iri("http://ex/a".to_owned()))],
        );
        assert_eq!(got.len(), 1, "exactly one row for the :a focus: {got:?}");
        assert!(got[0].contains("http://ex/x"), "?o = :x : {got:?}");
    }

    #[test]
    fn substitute_keeps_the_focus_var_projectable() {
        // `SELECT ?this ?o`: the substituted var must still appear in the result
        // (the seed join is below the projection, not a drop of ?this).
        let ds = subst_ds();
        let engine = NativeSparqlEngine::new();
        let result = engine
            .query(
                &ds,
                SparqlRequest {
                    query: "SELECT ?this ?o WHERE { ?this <http://ex/p> ?o }",
                    base_iri: None,
                    substitutions: &[("this".to_owned(), TermValue::Iri("http://ex/a".to_owned()))],
                },
            )
            .expect("query");
        let SparqlResult::Solutions {
            variables, rows, ..
        } = result
        else {
            panic!("expected solutions");
        };
        assert_eq!(variables, vec!["this".to_owned(), "o".to_owned()]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][0], Some(TermValue::Iri("http://ex/a".to_owned())));
        assert_eq!(rows[0][1], Some(TermValue::Iri("http://ex/x".to_owned())));
    }

    #[test]
    fn substitute_blank_focus_constrains_the_subject() {
        // A blank-node focus (`_:bn`) must pre-bind through the injection-only blank
        // VALUES seed and select ONLY its object (:z).
        let got = run_subst(
            "SELECT ?o WHERE { ?this <http://ex/p> ?o }",
            &[(
                "this".to_owned(),
                TermValue::Blank {
                    label: "bn".to_owned(),
                    scope: BlankScope::DEFAULT,
                },
            )],
        );
        assert_eq!(got.len(), 1, "exactly one row for the blank focus: {got:?}");
        assert!(got[0].contains("http://ex/z"), "?o = :z : {got:?}");
    }

    #[test]
    fn substitute_ask_is_pre_binding() {
        // ASK over the blank focus: true (it has a :p edge); a focus absent from the
        // data is false. Proves pre-binding flows into the boolean form too.
        let ds = subst_ds();
        let engine = NativeSparqlEngine::new();
        let ask = |focus: TermValue| -> bool {
            let r = engine
                .query(
                    &ds,
                    SparqlRequest {
                        query: "ASK { ?this <http://ex/p> ?o }",
                        base_iri: None,
                        substitutions: &[("this".to_owned(), focus)],
                    },
                )
                .expect("ask");
            matches!(r, SparqlResult::Boolean(true))
        };
        assert!(ask(TermValue::Blank {
            label: "bn".to_owned(),
            scope: BlankScope::DEFAULT,
        }));
        assert!(!ask(TermValue::Iri("http://ex/absent".to_owned())));
    }

    #[test]
    fn substitution_does_not_poison_the_plan_cache() {
        // Two queries with the SAME text but different focus nodes must each return
        // their own focus's row — proving the cached parse is cloned per call and the
        // substitution is not baked into the shared cache entry.
        let ds = subst_ds();
        let engine = NativeSparqlEngine::new();
        let q = "SELECT ?o WHERE { ?this <http://ex/p> ?o }";
        let run = |focus: &str| {
            let r = engine
                .query(
                    &ds,
                    SparqlRequest {
                        query: q,
                        base_iri: None,
                        substitutions: &[("this".to_owned(), TermValue::Iri(focus.to_owned()))],
                    },
                )
                .expect("query");
            col0(r)
        };
        assert!(run("http://ex/a")[0].contains("http://ex/x"));
        assert!(run("http://ex/b")[0].contains("http://ex/y"));
        // And an un-substituted run still sees all three rows.
        let all = engine
            .query(
                &ds,
                SparqlRequest {
                    query: q,
                    base_iri: None,
                    substitutions: &[],
                },
            )
            .expect("query");
        assert_eq!(col0(all).len(), 3, "the cached parse is unmodified");
    }

    // ── SHACL-SPARQL pre-binding (Stage 1) ───────────────────────────────────

    /// Run a SELECT query through the SHACL pre-binding path and return the
    /// sorted first-column debug strings.
    fn run_shacl_subst(query: &str, substitutions: &[(String, TermValue)]) -> Vec<String> {
        let ds = subst_ds();
        let engine = NativeSparqlEngine::new();
        let result = engine
            .query_with_options_view(
                &*ds,
                SparqlRequest {
                    query,
                    base_iri: None,
                    substitutions,
                },
                QueryOptions {
                    prebinding: ShaclPrebinding::Applied,
                    ..QueryOptions::EMPTY
                },
            )
            .expect("shacl prebinding query");
        col0(result)
    }

    #[test]
    fn shacl_prebinding_bound_in_filter_only_group() {
        // SHACL pre-binding-005 shape: a FILTER-only group that checks bound($this).
        let got = run_shacl_subst(
            "SELECT ?this WHERE { { FILTER(bound(?this)) } ?this <http://ex/p> ?o }",
            &[("this".to_owned(), TermValue::Iri("http://ex/a".to_owned()))],
        );
        assert_eq!(
            got.len(),
            1,
            "FILTER(bound($this)) must see the pre-bound focus"
        );
        assert!(got[0].contains("http://ex/a"), "{got:?}");
    }

    #[test]
    fn shacl_prebinding_union_filter_only_branch() {
        // SHACL pre-binding-002 shape: $this referenced only inside a FILTER-only
        // UNION branch must be substituted, so the equality test succeeds.
        let got = run_shacl_subst(
            "SELECT ?this WHERE { \
             { FILTER(false) } \
             UNION \
             { FILTER(?this = <http://ex/a>) } \
             }",
            &[("this".to_owned(), TermValue::Iri("http://ex/a".to_owned()))],
        );
        assert_eq!(
            got.len(),
            1,
            "the UNION branch with FILTER($this = :a) must match"
        );
        assert!(got[0].contains("http://ex/a"), "{got:?}");
    }

    #[test]
    fn shacl_prebinding_does_not_change_normal_query_path() {
        // The same query on the generic `query` path with no substitutions must
        // still evaluate normally (here it returns all three :p rows).
        let q = "SELECT ?this ?o WHERE { ?this <http://ex/p> ?o }";
        let ds = subst_ds();
        let engine = NativeSparqlEngine::new();
        let normal = engine
            .query(
                &ds,
                SparqlRequest {
                    query: q,
                    base_iri: None,
                    substitutions: &[],
                },
            )
            .expect("normal query");
        let SparqlResult::Solutions { rows, .. } = normal else {
            panic!("expected solutions");
        };
        assert_eq!(rows.len(), 3, "normal path must see all three subjects");
    }

    #[test]
    fn shacl_query_options_mint_prefix_prefixes_construct_blanks() {
        // The public plumbing test for the deterministic blank-mint prefix: the
        // options-driven SHACL entry must install the prefix on the evaluation so
        // a CONSTRUCT template blank mints `{prefix}c{n}` — the seam the SHACL
        // rules engine uses to give each focus node its own mint identity.
        let ds = subst_ds();
        let engine = NativeSparqlEngine::new();
        let result = engine
            .query_with_options_view(
                &*ds,
                SparqlRequest {
                    query: "CONSTRUCT { ?s <http://ex/derived> _:b } WHERE { ?s <http://ex/p> <http://ex/x> }",
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions {
                    prebinding: ShaclPrebinding::Applied,
                    bnode_mint_prefix: Some("fTag_"),
                    ..QueryOptions::EMPTY
                },
            )
            .expect("construct");
        let SparqlResult::Graph(graph) = result else {
            panic!("CONSTRUCT must return a graph");
        };
        assert_eq!(graph.quad_count(), 1);
        let quad = graph.quads().next().expect("one quad");
        let purrdf_core::TermRef::Blank { label, .. } = graph.resolve(quad.o) else {
            panic!("the object must be the minted blank");
        };
        assert_eq!(
            label, "fTag_c1",
            "minted labels must carry the caller-supplied prefix"
        );
    }

    /// An EMPTY property-function registry is indistinguishable from none: the same
    /// query over the same data returns byte-identical results through
    /// [`NativeSparqlEngine::query_with_options_view`] and through the plain
    /// registry-free entry.
    ///
    /// This pins the equivalence the registry's `None` handling rests on. Without it,
    /// "no registry configured" and "a registry with nothing in it" could drift into
    /// two different evaluation paths, and a host that wires an empty table — the
    /// normal state of a host that has declared no relation yet — would silently get a
    /// different engine from one that wires none.
    #[test]
    fn an_empty_property_function_registry_is_indistinguishable_from_none() {
        let ds = subst_ds();
        let engine = NativeSparqlEngine::new();
        let query = "SELECT ?s ?o WHERE { ?s <http://ex/p> ?o } ORDER BY ?s ?o";
        let request = || SparqlRequest {
            query,
            base_iri: None,
            substitutions: &[],
        };

        let without = engine.query(&ds, request()).expect("registry-free query");
        let registry = crate::property_fn::PropertyFunctionRegistry::new();
        assert!(registry.is_empty());
        let with_empty = engine
            .query_with_options_view(
                &*ds,
                request(),
                QueryOptions {
                    property_functions: &registry,
                    ..QueryOptions::EMPTY
                },
            )
            .expect("empty-registry query");

        let (
            SparqlResult::Solutions {
                variables: without_vars,
                rows: without_rows,
                ..
            },
            SparqlResult::Solutions {
                variables: with_vars,
                rows: with_rows,
                ..
            },
        ) = (without, with_empty)
        else {
            panic!("both queries must return solutions");
        };
        assert_eq!(without_vars, with_vars);
        assert_eq!(
            without_rows, with_rows,
            "an empty registry must not change a single row"
        );
    }

    /// The explain receipt NAMES the relations that were in scope, sorted, so two runs
    /// of the same query text over the same dataset under two different registries are
    /// two distinguishable receipts.
    ///
    /// The rows a relation emits are host code's, not the dataset's: without the list, a
    /// receipt would attribute an answer to a query and a dataset that between them do
    /// not determine it.
    #[test]
    fn the_explain_receipt_names_the_registered_relations() {
        let ds = subst_ds();
        let engine = NativeSparqlEngine::new();
        let query = "SELECT ?s ?o WHERE { ?s <http://ex/p> ?o } ORDER BY ?s ?o";
        let table = || {
            Arc::new(
                crate::property_fn::MemoryRelation::new(
                    1,
                    1,
                    vec![vec![
                        TermValue::iri("http://example.org/a"),
                        TermValue::iri("http://example.org/1"),
                    ]],
                )
                .expect("a one-row two-column table"),
            )
        };

        // Registered in the reverse of their sorted order, so a receipt that echoed
        // registration order would disagree with one that sorted.
        let mut left = crate::property_fn::PropertyFunctionRegistry::new();
        left.register("http://example.org/rel/second", table());
        left.register("http://example.org/rel/first", table());
        let mut right = crate::property_fn::PropertyFunctionRegistry::new();
        right.register("http://example.org/rel/first", table());

        let explain = |registry: &crate::property_fn::PropertyFunctionRegistry| {
            engine
                .explain_query_with_options(
                    &ds,
                    query,
                    None,
                    QueryOptions {
                        property_functions: registry,
                        ..QueryOptions::EMPTY
                    },
                )
                .expect("explain")
        };
        let left_receipt = explain(&left);
        let right_receipt = explain(&right);

        let left_iris: Vec<&str> = left_receipt
            .relations()
            .iter()
            .map(|descriptor| descriptor.iri.as_str())
            .collect();
        assert_eq!(
            left_iris,
            [
                "http://example.org/rel/first",
                "http://example.org/rel/second"
            ],
            "the receipt lists every registered IRI, sorted"
        );
        let right_iris: Vec<&str> = right_receipt
            .relations()
            .iter()
            .map(|descriptor| descriptor.iri.as_str())
            .collect();
        assert_eq!(right_iris, ["http://example.org/rel/first"]);

        // The receipt is more than the bare IRI: arity, volatility, and declared modes
        // all ride along, because two impls sharing an IRI can disagree on any of them.
        let first = &left_receipt.relations()[0];
        assert_eq!(first.subject_arity, 1);
        assert_eq!(first.object_arity, 1);
        assert_eq!(first.volatility, crate::Volatility::Stable);
        assert!(!first.modes.is_empty());

        assert_ne!(
            left_receipt.render(),
            right_receipt.render(),
            "two registries must not render as the same receipt"
        );
        assert!(
            left_receipt
                .render()
                .contains("http://example.org/rel/first arity=1,1 volatility=stable"),
            "the rendering names them, with arity and volatility: {}",
            left_receipt.render()
        );

        // With no registry EITHER block is present and empty — "nothing was in scope",
        // not "this build does not report what was".
        let bare = engine.explain_query(&ds, query, None).expect("explain");
        assert!(bare.relations().is_empty());
        assert!(bare.aggregates().is_empty());
        assert!(bare.render().contains("relations\naggregates\njoin-orders"));
    }

    /// A minimal `Commutative` `SUM`-alike over a single `xsd:integer`-lexical argument,
    /// for [`the_explain_receipt_names_the_registered_aggregates`] alone. Merges through
    /// `other.finish()` because the running total already IS the accumulator's whole
    /// mergeable state (see [`crate::agg_fn::AggregateAccumulator::combine`]'s doc
    /// comment on when that shortcut is sound).
    struct ExplainTestSumAccumulator {
        total: i64,
    }

    impl crate::agg_fn::AggregateAccumulator for ExplainTestSumAccumulator {
        fn step(&mut self, args: &[TermValue]) -> Result<(), crate::error::EvalError> {
            if let Some(TermValue::Literal { lexical_form, .. }) = args.first()
                && let Ok(n) = lexical_form.parse::<i64>()
            {
                self.total += n;
            }
            Ok(())
        }

        fn combine(
            &mut self,
            other: Box<dyn crate::agg_fn::AggregateAccumulator>,
        ) -> Result<(), crate::error::EvalError> {
            if let Some(TermValue::Literal { lexical_form, .. }) = other.finish()?
                && let Ok(n) = lexical_form.parse::<i64>()
            {
                self.total += n;
            }
            Ok(())
        }

        fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
            self
        }

        fn finish(self: Box<Self>) -> Result<Option<TermValue>, crate::error::EvalError> {
            Ok(Some(TermValue::typed_literal(
                self.total.to_string(),
                "http://www.w3.org/2001/XMLSchema#integer",
            )))
        }
    }

    struct ExplainTestSumAggregate;

    impl crate::agg_fn::CustomAggregate for ExplainTestSumAggregate {
        fn arity(&self) -> crate::user_fn::Arity {
            crate::user_fn::Arity::Exact(1)
        }
        fn volatility(&self) -> crate::user_fn::Volatility {
            crate::user_fn::Volatility::Stable
        }
        fn algebraic_class(&self) -> crate::agg_fn::AlgebraicClass {
            crate::agg_fn::AlgebraicClass::Commutative
        }
        fn state_bound(&self) -> u64 {
            0
        }
        fn init(
            &self,
            _scalarvals: &[(String, TermValue)],
        ) -> Box<dyn crate::agg_fn::AggregateAccumulator> {
            Box::new(ExplainTestSumAccumulator { total: 0 })
        }
    }

    /// The exact twin of [`the_explain_receipt_names_the_registered_relations`], for the
    /// custom-aggregate registry [`NativeSparqlEngine::explain_query_with_options`]
    /// takes: [`QueryExplanation::aggregates`] lists every registered IRI, IRI-sorted
    /// rather than registration-ordered, with its full declaration (arity, volatility,
    /// algebraic class, state bound, scalarvals) riding along.
    #[test]
    fn the_explain_receipt_names_the_registered_aggregates() {
        let ds = subst_ds();
        let engine = NativeSparqlEngine::new();
        let query = "SELECT (AGG(<http://example.org/agg/first>, ?o) AS ?f) \
                     WHERE { ?s <http://ex/p> ?o }";

        // Registered in the reverse of their sorted order, so a receipt that echoed
        // registration order would disagree with one that sorted.
        let mut registry = crate::agg_fn::AggregateRegistry::new();
        registry.register(
            "http://example.org/agg/second",
            Arc::new(ExplainTestSumAggregate),
        );
        registry.register(
            "http://example.org/agg/first",
            Arc::new(ExplainTestSumAggregate),
        );

        let receipt = engine
            .explain_query_with_options(
                &ds,
                query,
                None,
                QueryOptions {
                    aggregates: &registry,
                    ..QueryOptions::EMPTY
                },
            )
            .expect("explain");
        let iris: Vec<&str> = receipt
            .aggregates()
            .iter()
            .map(|descriptor| descriptor.iri.as_str())
            .collect();
        assert_eq!(
            iris,
            [
                "http://example.org/agg/first",
                "http://example.org/agg/second"
            ],
            "the receipt lists every registered IRI, sorted"
        );
        assert!(
            receipt.render().contains("http://example.org/agg/first"),
            "the rendering names the registered aggregate: {}",
            receipt.render()
        );
        // The `relations` block is unaffected — this seam is aggregates-only.
        assert!(receipt.relations().is_empty());
    }

    /// A query that needs BOTH a registered relation and a registered custom aggregate
    /// at once — the shape that under-declaring either registry answers wrong for
    /// rather than refuses. The engine declares the relation's namespace once via
    /// [`NativeSparqlEngine::with_parser_options`] (see
    /// [`purrdf_sparql_algebra::ParserOptions::property_fn_namespaces`]'s own
    /// documentation on why: "so that spelling one is a hard error rather than a
    /// silent data triple"), so the predicate is recognized as a call REGARDLESS of
    /// which registries a given [`QueryOptions`] value carries — the one variable
    /// across [`explain_query_with_options_reports_a_correct_receipt_for_a_query_needing_both_registries`]
    /// and its refusal siblings below.
    fn dual_registry_explain_fixture() -> (
        Arc<RdfDataset>,
        NativeSparqlEngine,
        &'static str,
        crate::property_fn::PropertyFunctionRegistry,
        crate::agg_fn::AggregateRegistry,
    ) {
        const REL_NS: &str = "https://example.org/dual-registry-explain/rel/";
        let ds = subst_ds();
        let engine = NativeSparqlEngine::new().with_parser_options(ParserOptions {
            property_fn_namespaces: vec![REL_NS.to_owned()],
            ..ParserOptions::default()
        });
        let query = "SELECT (AGG(<http://example.org/agg/dual>, ?v) AS ?s) \
                     WHERE { ?x <https://example.org/dual-registry-explain/rel/emit> ?v }";
        let mut relations = crate::property_fn::PropertyFunctionRegistry::new();
        relations.register(
            format!("{REL_NS}emit"),
            Arc::new(
                crate::property_fn::MemoryRelation::new(
                    1,
                    1,
                    vec![vec![
                        TermValue::iri("http://example.org/a"),
                        TermValue::typed_literal("5", "http://www.w3.org/2001/XMLSchema#integer"),
                    ]],
                )
                .expect("a one-row two-column table"),
            ),
        );
        let mut aggregates = crate::agg_fn::AggregateRegistry::new();
        aggregates.register(
            "http://example.org/agg/dual",
            Arc::new(ExplainTestSumAggregate),
        );
        (ds, engine, query, relations, aggregates)
    }

    /// The dual-registry entry gives a CORRECT receipt for a query that needs
    /// both a registered relation and a registered custom aggregate — populated
    /// `relations` and `aggregates` blocks, and the actual row the relation fires
    /// materializes (matching what [`SparqlEngine::query`]/`query_with_options` would
    /// return for the same query under the same two registries).
    #[test]
    fn explain_query_with_options_reports_a_correct_receipt_for_a_query_needing_both_registries() {
        let (ds, engine, query, relations, aggregates) = dual_registry_explain_fixture();
        let receipt = engine
            .explain_query_with_options(
                &ds,
                query,
                None,
                QueryOptions {
                    property_functions: &relations,
                    aggregates: &aggregates,
                    ..QueryOptions::EMPTY
                },
            )
            .expect("a query naming both registries at the combined entry must explain");
        assert_eq!(
            receipt
                .relations()
                .iter()
                .map(|d| d.iri.as_str())
                .collect::<Vec<_>>(),
            ["https://example.org/dual-registry-explain/rel/emit"],
            "the relation actually in scope is named"
        );
        assert_eq!(
            receipt
                .aggregates()
                .iter()
                .map(|d| d.iri.as_str())
                .collect::<Vec<_>>(),
            ["http://example.org/agg/dual"],
            "the aggregate actually in scope is named"
        );
        let rendered = receipt.render();
        assert!(
            rendered.contains("property-function-invocation"),
            "the relation's charge point fired at least once, proving the row-producing \
             call actually ran rather than being silently dropped: {rendered}"
        );
        // Cross-check against the real, non-explain evaluation of the identical query
        // under the identical two registries: the explain receipt and the query it
        // describes must agree on the row count the relation actually produced.
        let real = engine
            .query_with_options_view(
                &*ds,
                SparqlRequest {
                    query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions {
                    property_functions: &relations,
                    aggregates: &aggregates,
                    ..QueryOptions::EMPTY
                },
            )
            .expect("the same query, evaluated directly under the same two registries");
        let SparqlResult::Solutions { rows, .. } = real else {
            panic!("a SELECT evaluates to Solutions");
        };
        assert_eq!(
            rows.len(),
            1,
            "the relation fires exactly once, so AGG sees exactly one input row"
        );
    }

    /// A [`QueryOptions`] value that carries the aggregate registry but leaves
    /// `property_functions` at its `EMPTY` default REFUSES the dual-need query rather
    /// than silently answering a receipt for the narrower query "no relation fired".
    /// The predicate is recognized as a call (the engine declared the namespace), so
    /// admission finds nothing in the EMPTY relations registry to resolve it against
    /// and reports that as an error, exactly the outcome [`crate::property_fn_plan`]'s
    /// `resolve` documents: "a call with nothing to resolve against is a host
    /// configuration that names a relation it never supplied — never a silently empty
    /// one."
    #[test]
    fn explain_query_with_options_refuses_a_query_that_also_needs_a_relation() {
        let (ds, engine, query, _relations, aggregates) = dual_registry_explain_fixture();
        let err = engine
            .explain_query_with_options(
                &ds,
                query,
                None,
                QueryOptions {
                    aggregates: &aggregates,
                    ..QueryOptions::EMPTY
                },
            )
            .expect_err(
                "options that omit the property-function registry must refuse rather than \
                 report a receipt for a query with its relation silently dropped",
            );
        let message = err.to_string();
        assert!(
            message.contains("no property function is registered")
                || message.contains("property-function"),
            "the refusal must name the property-function seam as the cause: {message}"
        );
    }

    /// The exact symmetric case: a [`QueryOptions`] value that carries the
    /// property-function registry but leaves `aggregates` at its `EMPTY` default also
    /// refuses the SAME dual-need query, because `AGG(<iri>, …)` is fixed syntax
    /// admitted against the aggregate registry at prepare time regardless of which
    /// registry is empty. Asserted here, on the identical fixture, so the two refusal
    /// paths are proven symmetric rather than merely asserted so in prose.
    #[test]
    fn explain_query_with_options_refuses_a_query_that_also_needs_an_aggregate() {
        let (ds, engine, query, relations, _aggregates) = dual_registry_explain_fixture();
        let err = engine
            .explain_query_with_options(
                &ds,
                query,
                None,
                QueryOptions {
                    property_functions: &relations,
                    ..QueryOptions::EMPTY
                },
            )
            .expect_err(
                "options that omit the aggregate registry must refuse rather than report a \
                 receipt for a query with its aggregate silently dropped",
            );
        let message = err.to_string();
        assert!(
            message.contains("aggregate") || message.contains("unregistered"),
            "the refusal must name the aggregate seam as the cause: {message}"
        );
    }

    #[test]
    fn shacl_query_options_without_prefix_mint_exact_c_labels() {
        // `bnode_mint_prefix: None` must be byte-identical to the pre-options
        // entries: the first minted template blank is exactly `c1`.
        let ds = subst_ds();
        let engine = NativeSparqlEngine::new();
        let result = engine
            .query_with_options_view(
                &*ds,
                SparqlRequest {
                    query: "CONSTRUCT { ?s <http://ex/derived> _:b } WHERE { ?s <http://ex/p> <http://ex/x> }",
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions {
                    prebinding: ShaclPrebinding::Applied,
                    ..QueryOptions::EMPTY
                },
            )
            .expect("construct");
        let SparqlResult::Graph(graph) = result else {
            panic!("CONSTRUCT must return a graph");
        };
        let quad = graph.quads().next().expect("one quad");
        let purrdf_core::TermRef::Blank { label, .. } = graph.resolve(quad.o) else {
            panic!("the object must be the minted blank");
        };
        assert_eq!(label, "c1", "no prefix ⇒ byte-identical mint labels");
    }

    #[test]
    fn substitute_visible_inside_filter_exists_disjunction() {
        // SHACL ExpectedCell uses this shape: a focus is invalid iff it has both
        // value properties or neither. The pre-bound focus must be visible inside
        // FILTER/EXISTS; otherwise the EXISTS probes become whole-dataset globals.
        let mut b = RdfDatasetBuilder::new();
        let iri_prop = b.intern_iri("http://ex/cellValueIri");
        let lit_prop = b.intern_iri("http://ex/cellValueLiteral");
        let one = b.intern_iri("http://ex/one");
        let both = b.intern_iri("http://ex/both");
        let value = b.intern_iri("http://ex/value");
        let lit = b.intern_literal(RdfLiteral::simple("literal"));
        b.push_quad(one, iri_prop, value, None);
        b.push_quad(both, iri_prop, value, None);
        b.push_quad(both, lit_prop, lit, None);
        let ds = Arc::new(b.freeze().expect("freeze"));
        let engine = NativeSparqlEngine::new();
        let q = "SELECT ?this WHERE { \
                 FILTER( \
                   (EXISTS { ?this <http://ex/cellValueIri> ?i } && EXISTS { ?this <http://ex/cellValueLiteral> ?l }) || \
                   (!EXISTS { ?this <http://ex/cellValueIri> ?i } && !EXISTS { ?this <http://ex/cellValueLiteral> ?l }) \
                 ) \
               }";
        let run = |focus: &str| {
            let r = engine
                .query(
                    &ds,
                    SparqlRequest {
                        query: q,
                        base_iri: None,
                        substitutions: &[("this".to_owned(), TermValue::Iri(focus.to_owned()))],
                    },
                )
                .expect("query");
            col0(r)
        };

        assert!(
            run("http://ex/one").is_empty(),
            "a cell with exactly one value property must conform"
        );
        assert_eq!(run("http://ex/both").len(), 1);
        assert_eq!(run("http://ex/neither").len(), 1);
    }

    #[test]
    fn filter_not_exists_antijoin_returns_correct_rows() {
        // The class-without-stereotype anti-join shape end-to-end through the parser: FILTER NOT EXISTS whose inner
        // references the outer var only in a triple position. In `social()`, :a knows
        // :b and has a name; :b has neither. The anti-join keeps subjects that are a
        // knows-subject but have NO name → none here (:a has a name), so zero rows;
        // flip to a name-less subject to confirm a positive row.
        let ds = social();
        // Subjects with a name: only :a. NOT EXISTS { ?s :name ?n } over knows-subjects
        // ({:a}) → :a is excluded → empty.
        let empty = run_on(
            &ds,
            "SELECT ?s WHERE { ?s <http://ex/knows> ?o \
             FILTER NOT EXISTS { ?s <http://ex/name> ?n } }",
        );
        assert!(
            col0(empty).is_empty(),
            ":a has a name, so the anti-join is empty"
        );

        // EXISTS (the positive form): knows-subjects that DO have a name → :a.
        let got = col0(run_on(
            &ds,
            "SELECT ?s WHERE { ?s <http://ex/knows> ?o \
             FILTER EXISTS { ?s <http://ex/name> ?n } }",
        ));
        assert_eq!(got.len(), 1);
        assert!(got[0].contains("http://ex/a"));
    }

    #[test]
    fn select_returns_solutions() {
        let result = run("SELECT ?o WHERE { <http://ex/a> <http://ex/knows> ?o }");
        match result {
            SparqlResult::Solutions {
                variables, rows, ..
            } => {
                assert_eq!(variables, vec!["o".to_owned()]);
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0][0], Some(TermValue::Iri("http://ex/b".to_owned())));
            }
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    /// A dataset with a default graph plus two named graphs that share a triple.
    ///   default: (a,p,dflt)
    ///   ex:g1:   (a,p,x), (a,p,shared)
    ///   ex:g2:   (a,p,y), (a,p,shared)
    fn multigraph() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let p = b.intern_iri("http://ex/p");
        let a = b.intern_iri("http://ex/a");
        let dflt = b.intern_iri("http://ex/dflt");
        let x = b.intern_iri("http://ex/x");
        let y = b.intern_iri("http://ex/y");
        let shared = b.intern_iri("http://ex/shared");
        let g1 = b.intern_iri("http://ex/g1");
        let g2 = b.intern_iri("http://ex/g2");
        b.push_quad(a, p, dflt, None);
        b.push_quad(a, p, x, Some(g1));
        b.push_quad(a, p, shared, Some(g1));
        b.push_quad(a, p, y, Some(g2));
        b.push_quad(a, p, shared, Some(g2));
        b.freeze().expect("freeze")
    }

    fn run_on(ds: &Arc<RdfDataset>, query: &str) -> SparqlResult {
        NativeSparqlEngine::new()
            .query(
                ds,
                SparqlRequest {
                    query,
                    base_iri: None,
                    substitutions: &[],
                },
            )
            .expect("query")
    }

    /// `ORDER BY ADJUST(?dt, timezone)` orders by the ADJUSTed *value* (the
    /// underlying instant), proving `ADJUST` reached `is_builtin_function`'s
    /// `ORDER BY` lookahead (`order_key_ahead`) and evaluates without hitting
    /// the `Unsupported` fallback. Three same-local-clock-reading dateTimes at
    /// three different offsets denote three different UTC instants; ordering
    /// them normalizes every row to a fixed offset first, so the visible
    /// order is exactly the instant order, not the lexical order.
    #[test]
    fn order_by_adjust_orders_by_the_adjusted_instant() {
        let mut b = RdfDatasetBuilder::new();
        let dt = b.intern_iri("http://ex/dt");
        let a = b.intern_iri("http://ex/a"); // 09:00-05:00 = 14:00Z (latest)
        let bb = b.intern_iri("http://ex/b"); // 09:00+05:00 = 04:00Z (earliest)
        let c = b.intern_iri("http://ex/c"); // 09:00Z (middle)
        let xsd_datetime = "http://www.w3.org/2001/XMLSchema#dateTime";
        let a_dt = b.intern_literal(RdfLiteral::typed("2024-01-01T09:00:00-05:00", xsd_datetime));
        let b_dt = b.intern_literal(RdfLiteral::typed("2024-01-01T09:00:00+05:00", xsd_datetime));
        let c_dt = b.intern_literal(RdfLiteral::typed("2024-01-01T09:00:00Z", xsd_datetime));
        b.push_quad(a, dt, a_dt, None);
        b.push_quad(bb, dt, b_dt, None);
        b.push_quad(c, dt, c_dt, None);
        let ds = b.freeze().expect("freeze");
        let query = "SELECT ?s WHERE { ?s <http://ex/dt> ?dt } \
                     ORDER BY ADJUST(?dt, \"PT0H\"^^<http://www.w3.org/2001/XMLSchema#dayTimeDuration>)";
        let SparqlResult::Solutions { rows, .. } = run_on(&ds, query) else {
            panic!("expected solutions");
        };
        let subjects: Vec<String> = rows
            .iter()
            .map(|r| match &r[0] {
                Some(TermValue::Iri(s)) => s.clone(),
                other => panic!("expected an IRI, got {other:?}"),
            })
            .collect();
        assert_eq!(
            subjects,
            vec![
                "http://ex/b".to_owned(),
                "http://ex/c".to_owned(),
                "http://ex/a".to_owned(),
            ],
            "earliest-to-latest UTC instant order, not lexical order"
        );
    }

    /// The first-column values of a solutions result, as sorted debug strings.
    fn col0(result: SparqlResult) -> Vec<String> {
        match result {
            SparqlResult::Solutions { rows, .. } => {
                let mut v: Vec<String> = rows.iter().map(|r| format!("{:?}", r[0])).collect();
                v.sort();
                v
            }
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    #[test]
    fn from_merges_named_graphs_into_default_excluding_store_default() {
        let ds = multigraph();
        // FROM g1 FROM g2 → active default = RDF-merge(g1, g2); the store default graph
        // (dflt) is excluded; the shared triple is unioned to a single solution.
        let got = col0(run_on(
            &ds,
            "SELECT ?o FROM <http://ex/g1> FROM <http://ex/g2> \
             WHERE { <http://ex/a> <http://ex/p> ?o }",
        ));
        assert_eq!(got.len(), 3, "x, y, shared (deduped), NOT dflt: {got:?}");
        assert!(
            !got.iter().any(|s| s.contains("dflt")),
            "store default excluded: {got:?}"
        );
        assert_eq!(
            got.iter().filter(|s| s.contains("shared")).count(),
            1,
            "RDF-merge unions the shared triple to one solution"
        );
    }

    #[test]
    fn no_from_clause_uses_store_default_graph() {
        let ds = multigraph();
        let got = col0(run_on(
            &ds,
            "SELECT ?o WHERE { <http://ex/a> <http://ex/p> ?o }",
        ));
        assert_eq!(got.len(), 1, "only the store default graph: {got:?}");
        assert!(got[0].contains("dflt"));
    }

    #[test]
    fn from_named_restricts_graph_var() {
        let ds = multigraph();
        // FROM NAMED g1 → GRAPH ?g binds only to g1 (g2 not addressable); the default
        // graph is empty (no plain FROM).
        let got = col0(run_on(
            &ds,
            "SELECT ?g FROM NAMED <http://ex/g1> \
             WHERE { GRAPH ?g { <http://ex/a> <http://ex/p> ?o } }",
        ));
        assert!(!got.is_empty(), "g1 IS addressable");
        assert!(got.iter().all(|s| s.contains("g1")), "only g1: {got:?}");
        assert!(
            !got.iter().any(|s| s.contains("g2")),
            "g2 not in FROM NAMED"
        );
    }

    #[test]
    fn from_nonexistent_graph_is_empty_not_error() {
        let ds = multigraph();
        let got = col0(run_on(
            &ds,
            "SELECT ?o FROM <http://ex/absent> WHERE { <http://ex/a> <http://ex/p> ?o }",
        ));
        assert!(
            got.is_empty(),
            "absent FROM graph → empty default → no rows"
        );
    }

    #[test]
    fn ask_returns_boolean() {
        let yes = run("ASK { <http://ex/a> <http://ex/knows> <http://ex/b> }");
        assert!(matches!(yes, SparqlResult::Boolean(true)));
        let no = run("ASK { <http://ex/a> <http://ex/knows> <http://ex/nobody> }");
        assert!(matches!(no, SparqlResult::Boolean(false)));
    }

    #[test]
    fn construct_returns_graph() {
        let result =
            run("CONSTRUCT { ?s <http://ex/related> ?o } WHERE { ?s <http://ex/knows> ?o }");
        match result {
            SparqlResult::Graph(g) => assert_eq!(g.quad_count(), 1),
            other => panic!("expected graph, got {other:?}"),
        }
    }

    #[test]
    fn plan_cache_memoizes_parse() {
        let mut cache = PlanCache::new();
        let q = "SELECT ?x WHERE { ?x ?p ?o }";
        let a = cache.prepare(q, None).expect("first");
        let b = cache.prepare(q, None).expect("second");
        // Same text → the same cached Arc.
        assert!(Arc::ptr_eq(&a, &b));
    }

    #[test]
    fn parse_error_becomes_diagnostic() {
        let ds = social();
        let engine = NativeSparqlEngine::new();
        let err = engine
            .query(
                &ds,
                SparqlRequest {
                    query: "this is not sparql",
                    base_iri: None,
                    substitutions: &[],
                },
            )
            .unwrap_err();
        assert_eq!(err.code, "native-sparql-query-parse");
    }

    // ── UPDATE seam (engine end-to-end) ────────────────────────────────────────

    /// A test-only resolver returning a fixed one-quad dataset for any LOAD source.
    struct TestResolver {
        ds: Arc<RdfDataset>,
    }
    impl GraphResolver for TestResolver {
        fn resolve(
            &self,
            _request: crate::update::GraphResolveRequest<'_>,
        ) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
            Ok(self.ds.clone())
        }
    }

    fn loadable() -> Arc<RdfDataset> {
        // :loaded :p "v" .
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("http://ex/loaded");
        let p = b.intern_iri("http://ex/p");
        let o = b.intern_literal(RdfLiteral::simple("v"));
        b.push_quad(s, p, o, None);
        b.freeze().expect("freeze loadable")
    }

    /// An empty default-graph dataset.
    fn empty() -> Arc<RdfDataset> {
        RdfDatasetBuilder::new().freeze().expect("freeze empty")
    }

    fn update(engine: &NativeSparqlEngine, ds: &mut Arc<RdfDataset>, query: &str) {
        engine
            .update(
                ds,
                SparqlRequest {
                    query,
                    base_iri: None,
                    substitutions: &[],
                },
            )
            .expect("update applies");
    }

    /// The effective quads as a comparable set of value tuples.
    fn quad_set(ds: &RdfDataset) -> std::collections::BTreeSet<String> {
        ds.quads()
            .map(|q| {
                format!(
                    "{:?}|{:?}|{:?}|{:?}",
                    ds.resolve(q.s),
                    ds.resolve(q.p),
                    ds.resolve(q.o),
                    q.g.map(|g| format!("{:?}", ds.resolve(g)))
                )
            })
            .collect()
    }

    #[test]
    fn insert_data_adds_quad() {
        let engine = NativeSparqlEngine::new();
        let mut ds = empty();
        update(
            &engine,
            &mut ds,
            "INSERT DATA { <http://ex/a> <http://ex/p> <http://ex/b> }",
        );
        assert_eq!(ds.quad_count(), 1);
        assert!(
            ds.term_id_by_value(&TermValue::Iri("http://ex/a".to_owned()))
                .is_some()
        );
    }

    #[test]
    fn delete_data_removes_quad() {
        let engine = NativeSparqlEngine::new();
        let mut ds = social();
        update(
            &engine,
            &mut ds,
            "DELETE DATA { <http://ex/a> <http://ex/knows> <http://ex/b> }",
        );
        // The :knows quad is gone; the :name quad survives.
        assert_eq!(ds.quad_count(), 1);
        assert!(
            ds.term_id_by_value(&TermValue::Iri("http://ex/knows".to_owned()))
                .is_none()
        );
    }

    #[test]
    fn delete_insert_where_rewrites() {
        let engine = NativeSparqlEngine::new();
        let mut ds = social();
        update(
            &engine,
            &mut ds,
            "DELETE { ?s <http://ex/knows> ?o } INSERT { ?s <http://ex/met> ?o } \
             WHERE { ?s <http://ex/knows> ?o }",
        );
        // :knows replaced by :met; :name untouched.
        assert!(
            ds.term_id_by_value(&TermValue::Iri("http://ex/knows".to_owned()))
                .is_none()
        );
        assert!(
            ds.term_id_by_value(&TermValue::Iri("http://ex/met".to_owned()))
                .is_some()
        );
        assert_eq!(ds.quad_count(), 2);
    }

    #[test]
    fn clear_default_empties_target() {
        let engine = NativeSparqlEngine::new();
        let mut ds = social();
        update(&engine, &mut ds, "CLEAR DEFAULT");
        assert_eq!(ds.quad_count(), 0);
    }

    #[test]
    fn load_with_resolver_inserts_resolved_quads() {
        let engine =
            NativeSparqlEngine::new().with_resolver(Arc::new(TestResolver { ds: loadable() }));
        let mut ds = empty();
        update(&engine, &mut ds, "LOAD <http://ex/doc>");
        assert_eq!(ds.quad_count(), 1);
        assert!(
            ds.term_id_by_value(&TermValue::Iri("http://ex/loaded".to_owned()))
                .is_some()
        );
    }

    #[test]
    fn load_without_resolver_is_a_hard_error() {
        let engine = NativeSparqlEngine::new();
        let mut ds = empty();
        let err = engine
            .update(
                &mut ds,
                SparqlRequest {
                    query: "LOAD <http://ex/doc>",
                    base_iri: None,
                    substitutions: &[],
                },
            )
            .unwrap_err();
        assert_eq!(err.code, "native-sparql-load-no-resolver");
    }

    #[test]
    fn load_silent_without_resolver_is_a_noop_ok() {
        let engine = NativeSparqlEngine::new();
        let mut ds = social();
        let before = ds.quad_count();
        update(&engine, &mut ds, "LOAD SILENT <http://ex/doc>");
        assert_eq!(ds.quad_count(), before, "silent load no-ops");
    }

    #[test]
    fn update_is_atomic_on_a_later_op_failure() {
        // A two-operation request whose FIRST op would insert and whose SECOND op
        // hard-fails (LOAD with no resolver, not SILENT). Branch-then-freeze atomicity
        // requires the whole request to roll back: the dataset must be byte-identical
        // (same quad set) to before, with the first op's INSERT NOT leaked.
        let engine = NativeSparqlEngine::new();
        let mut ds = social();
        let before = quad_set(&ds);

        let err = engine
            .update(
                &mut ds,
                SparqlRequest {
                    query: "INSERT DATA { <http://ex/x> <http://ex/y> <http://ex/z> } ; \
                            LOAD <http://ex/doc>",
                    base_iri: None,
                    substitutions: &[],
                },
            )
            .unwrap_err();
        assert_eq!(err.code, "native-sparql-load-no-resolver");

        let after = quad_set(&ds);
        assert_eq!(
            after, before,
            "the failed request left the dataset untouched"
        );
    }

    /// GAP 3 regression: the UPDATE path must thread the SAME `EvalCtx` wiring the
    /// query path uses, so a `NOW()` bound inside a `DELETE/INSERT … WHERE` is the
    /// live wall clock — not some frozen/epoch default — mirroring
    /// `default_engine_now_is_current_wall_clock` but through `engine.update`.
    #[test]
    fn now_is_live_in_update_where() {
        let engine = NativeSparqlEngine::new();
        let mut ds = empty();
        update(
            &engine,
            &mut ds,
            "INSERT { <http://ex/s> <http://ex/p> ?n } WHERE { BIND(NOW() AS ?n) }",
        );
        let r = engine
            .query(
                &ds,
                SparqlRequest {
                    query: "SELECT (YEAR(?o) AS ?y) WHERE { <http://ex/s> <http://ex/p> ?o }",
                    base_iri: None,
                    substitutions: &[],
                },
            )
            .expect("query");
        match r {
            SparqlResult::Solutions { rows, .. } => {
                assert_eq!(rows.len(), 1);
                let year: i64 = render_cell(rows[0][0].as_ref())
                    .parse()
                    .expect("YEAR(?o) must render as an integer");
                assert!(
                    year >= 2025,
                    "NOW() inside an UPDATE WHERE must be the live wall clock, got year {year}"
                );
            }
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    /// GAP 3 regression: `heldIn` inside an UPDATE `WHERE` must see the engine's
    /// configured [`StandpointPredicates`] table, the same as the query path
    /// (`gmeow_namespace_and_predicate_table_flow_through_configuration`). Before the
    /// fix, `engine::update` dropped the table on the floor and any `heldIn` in a
    /// `DELETE/INSERT … WHERE` hard-errored even on a standpoint-configured engine.
    #[test]
    fn heldin_in_update_where_uses_configured_standpoint_predicates() {
        let ds = gmeow_standpoint_ds();
        let configured = NativeSparqlEngine::new()
            .with_parser_options(ParserOptions {
                extension_fn_namespaces: vec![GMEOW_NS.to_owned()],
                property_fn_namespaces: Vec::new(),
                property_fn_iris: Vec::new(),
            })
            .with_standpoint_predicates(StandpointPredicates::new(
                format!("{GMEOW_NS}accordingTo"),
                format!("{GMEOW_NS}sharpens"),
            ));
        let q = format!(
            "PREFIX gmeow: <{GMEOW_NS}>\n\
             INSERT {{ <http://ex/hit> <http://ex/in> <http://ex/T1> }} \
             WHERE {{ FILTER( gmeow:heldIn(<http://ex/r>, <http://ex/T1>) ) }}"
        );
        let mut configured_ds = Arc::clone(&ds);
        configured
            .update(
                &mut configured_ds,
                SparqlRequest {
                    query: &q,
                    base_iri: None,
                    substitutions: &[],
                },
            )
            .expect("heldIn in an UPDATE WHERE must see the configured standpoint table");
        assert!(
            configured_ds
                .term_id_by_value(&TermValue::Iri("http://ex/hit".to_owned()))
                .is_some(),
            "the WHERE matched and the INSERT landed"
        );

        // Same UPDATE, unconfigured engine: heldIn hard-errors (never a silent default).
        let unconfigured = NativeSparqlEngine::new().with_parser_options(ParserOptions {
            extension_fn_namespaces: vec![GMEOW_NS.to_owned()],
            property_fn_namespaces: Vec::new(),
            property_fn_iris: Vec::new(),
        });
        let mut unconfigured_ds = Arc::clone(&ds);
        let err = unconfigured
            .update(
                &mut unconfigured_ds,
                SparqlRequest {
                    query: &q,
                    base_iri: None,
                    substitutions: &[],
                },
            )
            .unwrap_err();
        assert_eq!(err.code, "native-sparql-update-eval");
        assert!(
            err.message
                .contains("requires a standpoint predicate configuration"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn update_is_atomic_on_a_where_eval_failure() {
        // A second atomicity proof through a different failure mode: a modify whose
        // WHERE hits an unsupported construct (SERVICE → `native-sparql-update-eval`)
        // after a successful INSERT. The INSERT must not leak.
        let engine = NativeSparqlEngine::new();
        let mut ds = empty();
        let before = quad_set(&ds);

        let err = engine
            .update(
                &mut ds,
                SparqlRequest {
                    query: "INSERT DATA { <http://ex/x> <http://ex/y> <http://ex/z> } ; \
                            DELETE { ?s <http://ex/p> ?o } \
                            WHERE { SERVICE <http://ex/svc> { ?s <http://ex/p> ?o } }",
                    base_iri: None,
                    substitutions: &[],
                },
            )
            .unwrap_err();
        assert_eq!(err.code, "native-sparql-update-eval");
        assert_eq!(
            quad_set(&ds),
            before,
            "INSERT must not leak past the failure"
        );
    }

    #[test]
    fn update_parse_error_becomes_diagnostic() {
        let engine = NativeSparqlEngine::new();
        let mut ds = empty();
        let err = engine
            .update(
                &mut ds,
                SparqlRequest {
                    query: "this is not an update",
                    base_iri: None,
                    substitutions: &[],
                },
            )
            .unwrap_err();
        assert_eq!(err.code, "native-sparql-update-parse");
    }

    #[test]
    fn update_unrecognized_version_refused_and_applies_nothing() {
        // The mutating counterpart of the query-side admission refusal
        // (`crate::eval::tests`, or the query test just above): an UPDATE whose
        // prologue declares a `VERSION` this evaluator does not recognize must be
        // refused through the SAME `native-sparql-update-eval` diagnostic code the
        // WHERE-eval failure tests above use, and — the load-bearing half — must
        // leave the dataset byte-for-byte unchanged. `Arc::ptr_eq` proves the handle
        // was never even re-frozen to an equal value.
        let engine = NativeSparqlEngine::new();
        let mut ds = empty();
        let before = Arc::clone(&ds);
        let before_quads = quad_set(&ds);
        let err = engine
            .update(
                &mut ds,
                SparqlRequest {
                    query: "VERSION \"9.9\" INSERT DATA { <http://ex/x> <http://ex/y> <http://ex/z> }",
                    base_iri: None,
                    substitutions: &[],
                },
            )
            .unwrap_err();
        assert_eq!(err.code, "native-sparql-update-eval");
        assert!(err.message.contains("VERSION \"9.9\""));
        assert!(
            Arc::ptr_eq(&before, &ds),
            "an unrecognized VERSION must not even re-freeze the dataset"
        );
        assert_eq!(quad_set(&ds), before_quads, "no mutation applied");
    }

    #[test]
    fn update_governed_unrecognized_version_refused_and_applies_nothing() {
        // The governed sibling: `update_governed` reports the SAME admission
        // refusal as an `Err`, not as a `GovernedUpdateOutcome::BudgetExhausted` —
        // an unrecognized `VERSION` is a request the evaluator does not know how to
        // honor at all, never a resource ceiling.
        let engine = NativeSparqlEngine::new();
        let mut ds = empty();
        let before = Arc::clone(&ds);
        let before_quads = quad_set(&ds);
        let err = engine
            .update_governed(
                &mut ds,
                SparqlRequest {
                    query: "VERSION \"9.9\" INSERT DATA { <http://ex/x> <http://ex/y> <http://ex/z> }",
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::EMPTY,
                &QueryGovernors::UNBOUNDED,
            )
            .unwrap_err();
        assert_eq!(err.code, "native-sparql-update-eval");
        assert!(err.message.contains("VERSION \"9.9\""));
        assert!(
            Arc::ptr_eq(&before, &ds),
            "an unrecognized VERSION must not even re-freeze the dataset"
        );
        assert_eq!(quad_set(&ds), before_quads, "no mutation applied");
    }

    #[test]
    fn update_recognized_versions_still_execute() {
        // Both spellings this evaluator recognizes must run exactly as an
        // undeclared-version UPDATE would — admission only refuses `Other`.
        for version in ["1.2", "1.2-basic"] {
            let engine = NativeSparqlEngine::new();
            let mut ds = empty();
            update(
                &engine,
                &mut ds,
                &format!(
                    "VERSION \"{version}\" INSERT DATA {{ <http://ex/x> <http://ex/y> <http://ex/z> }}"
                ),
            );
            assert_eq!(
                ds.quad_count(),
                1,
                "VERSION {version:?} must evaluate the update normally"
            );
        }
    }

    #[test]
    fn engine_has_no_resolver_by_default() {
        assert!(NativeSparqlEngine::new().resolver.is_none());
        assert!(NativeSparqlEngine::default().resolver.is_none());
    }

    // ── configurable extension namespace + standpoint predicate table ─────────

    /// The gmeow ontology namespace — a deployment alias for the same closed
    /// extension-function set, with its own domain standpoint predicates.
    const GMEOW_NS: &str = "http://example.org/ns/gmeow/";

    /// A standpoint dataset in the GMEOW vocabulary: reifier `:r` held in `:T1`
    /// (via `gmeow:accordingTo`), and `:T1 gmeow:sharpens :T2`.
    fn gmeow_standpoint_ds() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let r = b.intern_iri("http://ex/r");
        let s = b.intern_iri("http://ex/s");
        let p = b.intern_iri("http://ex/p");
        let o = b.intern_iri("http://ex/o");
        let t1 = b.intern_iri("http://ex/T1");
        let t2 = b.intern_iri("http://ex/T2");
        let according_to = b.intern_iri(&format!("{GMEOW_NS}accordingTo"));
        let sharpens = b.intern_iri(&format!("{GMEOW_NS}sharpens"));
        let triple = b.intern_triple(s, p, o);
        b.push_reifier(r, triple);
        b.push_annotation(r, according_to, t1);
        b.push_quad(t1, sharpens, t2, None);
        b.freeze().expect("freeze")
    }

    /// The gmeow migration path end-to-end: the namespace alias flows through
    /// [`ParserOptions`] (so `gmeow:heldIn(...)` still parses) and the domain
    /// predicates flow through [`StandpointPredicates`] (so the evaluator reads
    /// `gmeow:accordingTo`/`gmeow:sharpens` from the data) — no engine constants.
    #[test]
    fn gmeow_namespace_and_predicate_table_flow_through_configuration() {
        let ds = gmeow_standpoint_ds();
        let engine = NativeSparqlEngine::new()
            .with_parser_options(ParserOptions {
                extension_fn_namespaces: vec![GMEOW_NS.to_owned()],
                property_fn_namespaces: Vec::new(),
                property_fn_iris: Vec::new(),
            })
            .with_standpoint_predicates(StandpointPredicates::new(
                format!("{GMEOW_NS}accordingTo"),
                format!("{GMEOW_NS}sharpens"),
            ));
        let ask = |standpoint: &str| {
            let q = format!(
                "PREFIX gmeow: <{GMEOW_NS}>\n\
                 ASK {{ FILTER( gmeow:heldIn(<http://ex/r>, <http://ex/{standpoint}>) ) }}"
            );
            let r = engine
                .query(
                    &ds,
                    SparqlRequest {
                        query: &q,
                        base_iri: None,
                        substitutions: &[],
                    },
                )
                .expect("query");
            matches!(r, SparqlResult::Boolean(true))
        };
        assert!(ask("T1"), "held directly in its vantage standpoint");
        assert!(ask("T2"), "held via the direct gmeow:sharpens edge");
        assert!(!ask("T9"), "not held in an unrelated standpoint");
    }

    #[test]
    fn held_in_without_a_predicate_table_is_a_hard_diagnostic() {
        // heldIn parses under a caller-configured namespace, but evaluation must
        // hard-fail when no standpoint predicate table is configured — never
        // guess a default.
        let ds = gmeow_standpoint_ds();
        let engine = NativeSparqlEngine::new().with_parser_options(ParserOptions {
            extension_fn_namespaces: vec![GMEOW_NS.to_owned()],
            property_fn_namespaces: Vec::new(),
            property_fn_iris: Vec::new(),
        });
        let q = format!(
            "PREFIX gmeow: <{GMEOW_NS}>\n\
             ASK {{ FILTER( gmeow:heldIn(<http://ex/r>, <http://ex/T1>) ) }}"
        );
        let err = engine
            .query(
                &ds,
                SparqlRequest {
                    query: &q,
                    base_iri: None,
                    substitutions: &[],
                },
            )
            .unwrap_err();
        assert_eq!(err.code, "native-sparql-query-eval");
        assert!(
            err.message
                .contains("requires a standpoint predicate configuration"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn plan_cache_keys_on_the_extension_namespace_set() {
        // The SAME text under two namespace configurations must be two cache
        // entries — a gmeow-alias parse must not be served to a default parse.
        let mut cache = PlanCache::new();
        let q = format!(
            "PREFIX gmeow: <{GMEOW_NS}>\nSELECT (gmeow:listLength(?l) AS ?n) WHERE {{ ?s ?p ?l }}"
        );
        let with_alias = ParserOptions {
            extension_fn_namespaces: vec![GMEOW_NS.to_owned()],
            property_fn_namespaces: Vec::new(),
            property_fn_iris: Vec::new(),
        };
        let a = cache
            .prepare_with(&q, None, &with_alias)
            .expect("parse with the alias configured");
        // Under the DEFAULT options the gmeow IRI is a plain custom function.
        let b = cache
            .prepare_with(&q, None, &ParserOptions::default())
            .expect("parse without the alias");
        assert!(
            !Arc::ptr_eq(&a, &b),
            "different namespace configurations must not share a cache entry"
        );
    }

    #[test]
    fn plan_cache_keys_on_the_property_function_exact_iri_set() {
        // Two configurations that agree on everything else — same text, same base,
        // same extension/namespace sets, same registry — but differ only in
        // `property_fn_iris` must not share a cache entry: one recognizes the
        // predicate as a call (and resolves/feasibility-orders it against the
        // registry), the other reads the very same text as an ordinary triple
        // pattern. Sharing a plan would hand one host's algebra to the other.
        let mut cache = PlanCache::new();
        let iri = format!("{GMEOW_NS}rel");
        let q = format!("SELECT ?s ?o WHERE {{ ?s <{iri}> ?o }}");

        let mut registry = crate::property_fn::PropertyFunctionRegistry::new();
        registry.register(
            iri.clone(),
            Arc::new(
                crate::property_fn::MemoryRelation::new(1, 1, vec![])
                    .expect("an empty table is a valid one-in-one-out relation"),
            ),
        );

        let with_exact_iri = ParserOptions {
            extension_fn_namespaces: Vec::new(),
            property_fn_namespaces: Vec::new(),
            property_fn_iris: vec![iri],
        };
        let a = cache
            .prepare_with_relations(
                &q,
                None,
                &with_exact_iri,
                &registry,
                &crate::agg_fn::AggregateRegistry::EMPTY,
            )
            .expect("parse with the exact IRI recognized and resolved against the registry");
        // Under the DEFAULT options the same predicate is an ordinary triple.
        let b = cache
            .prepare_with_relations(
                &q,
                None,
                &ParserOptions::default(),
                &registry,
                &crate::agg_fn::AggregateRegistry::EMPTY,
            )
            .expect("parse without the exact IRI configured");
        assert!(
            !Arc::ptr_eq(&a, &b),
            "different property_fn_iris configurations must not share a cache entry"
        );
        let Query::Select {
            pattern: b_pattern, ..
        } = &b.query
        else {
            panic!("a SELECT's algebra root is a Select, got {:?}", b.query);
        };
        let GraphPattern::Project { inner: b_inner, .. } = b_pattern else {
            panic!("a SELECT's algebra root is a Project, got {b_pattern:?}");
        };
        assert!(
            matches!(&**b_inner, GraphPattern::Bgp { .. }),
            "without property_fn_iris the predicate stays an ordinary BGP triple: {b_inner:?}"
        );
        let Query::Select {
            pattern: a_pattern, ..
        } = &a.query
        else {
            panic!("a SELECT's algebra root is a Select, got {:?}", a.query);
        };
        let GraphPattern::Project { inner: a_inner, .. } = a_pattern else {
            panic!("a SELECT's algebra root is a Project, got {a_pattern:?}");
        };
        assert!(
            matches!(&**a_inner, GraphPattern::PropertyFunction(_)),
            "with property_fn_iris configured the predicate becomes a call: {a_inner:?}"
        );
    }

    /// H12: pins that `QueryOptions`'s three registries becoming non-optional
    /// (`&Registry`, `AggregateRegistry::EMPTY`/`PropertyFunctionRegistry::EMPTY`
    /// standing in for the old `Option::None`) did not weaken
    /// [`check_plan_matches_relations`]'s plan-identity guard. Three cases:
    ///
    /// 1. A plan prepared under no registry at all, evaluated under
    ///    [`QueryOptions::EMPTY`] (the canonical empty registries): must MATCH —
    ///    this is the ordinary registry-free path every query has always taken.
    /// 2. The SAME plan, evaluated under INDEPENDENTLY constructed, still-empty
    ///    registries (a different instance than the `EMPTY` constant): must ALSO
    ///    match — an empty registry resolves no IRI regardless of which instance
    ///    is asked, so no plan's admitted behavior can depend on which one it was
    ///    prepared against (see [`crate::registry_id::RegistryId::EMPTY`]'s docs).
    /// 3. The SAME plan, evaluated under a REGISTERED (non-empty) relation: must
    ///    be REFUSED — sharing `EMPTY`'s reserved instance id across every empty
    ///    registry must never let a genuinely different, non-empty registry be
    ///    mistaken for it.
    #[test]
    fn h12_empty_registries_are_interchangeable_but_a_real_one_is_still_distinct() {
        let query = "SELECT ?s WHERE { ?s <http://example.org/p> ?o }";
        let mut cache = PlanCache::new();
        let prepared = cache
            .prepare_with(query, None, &ParserOptions::default())
            .expect("parses under no registry at all");

        assert!(
            check_plan_matches_relations(&prepared, QueryOptions::EMPTY).is_ok(),
            "a plan prepared registry-free must match QueryOptions::EMPTY"
        );

        let fresh_relations = crate::property_fn::PropertyFunctionRegistry::new();
        let fresh_aggregates = crate::agg_fn::AggregateRegistry::new();
        assert!(
            check_plan_matches_relations(
                &prepared,
                QueryOptions {
                    property_functions: &fresh_relations,
                    aggregates: &fresh_aggregates,
                    ..QueryOptions::EMPTY
                }
            )
            .is_ok(),
            "an independently constructed EMPTY registry must be interchangeable with EMPTY"
        );

        let mut real_relations = crate::property_fn::PropertyFunctionRegistry::new();
        real_relations.register(
            "http://example.org/rel",
            Arc::new(
                crate::property_fn::MemoryRelation::new(1, 1, vec![])
                    .expect("an empty table is a valid one-in-one-out relation"),
            ),
        );
        assert!(
            check_plan_matches_relations(
                &prepared,
                QueryOptions {
                    property_functions: &real_relations,
                    ..QueryOptions::EMPTY
                }
            )
            .is_err(),
            "plan identity must still refuse a genuinely different, non-empty registry"
        );
    }

    /// A one-in-one-out relation whose declared [`Volatility`](crate::Volatility) is the
    /// only thing the constructor varies — built to prove that the registry fingerprint
    /// (and therefore the plan cache, and the governed receipt) is sensitive to
    /// volatility rather than only to arity and declared modes.
    #[derive(Debug)]
    struct FixedVolatilityRelation {
        volatility: crate::Volatility,
        modes: [crate::BindingPattern; 1],
    }

    impl FixedVolatilityRelation {
        fn new(volatility: crate::Volatility) -> Self {
            let arity = crate::PfArity::new(1, 1);
            Self {
                volatility,
                modes: [arity.all_free_mode()],
            }
        }
    }

    /// The empty cursor `FixedVolatilityRelation::open` hands out: these fixtures exist
    /// to be registered and described, never dispatched.
    struct EmptyCursor;

    impl crate::PfCursor for EmptyCursor {
        fn next(&mut self) -> Result<Option<crate::property_fn::PfRow>, crate::EvalError> {
            Ok(None)
        }
    }

    impl crate::property_fn::PropertyFunction for FixedVolatilityRelation {
        fn volatility(&self) -> crate::Volatility {
            self.volatility
        }

        fn arity(&self) -> crate::PfArity {
            crate::PfArity::new(1, 1)
        }

        fn modes(&self) -> &[crate::BindingPattern] {
            &self.modes
        }

        fn rows_per_invocation(&self, _mode: crate::BindingPattern) -> u64 {
            0
        }

        fn open(
            &self,
            _args: &crate::PfArgs<'_>,
            _ceiling: Option<u64>,
        ) -> Result<Box<dyn crate::PfCursor>, crate::EvalError> {
            Ok(Box::new(EmptyCursor))
        }
    }

    /// GAP-7 (registry fingerprint): two registries that agree on IRI, arity, and every
    /// declared mode — differing ONLY in volatility — must not share a plan-cache entry.
    ///
    /// Volatility is not read by the feasibility-ordering pass at all (it decides
    /// whether a call may run on a fork-join worker, an EVALUATION-time question), so
    /// before this fingerprint carried it, two such registries planned identically AND
    /// shared a cache slot — silently handing one registry's plan to a call the other
    /// registry declared unsafe to parallelize.
    #[test]
    fn plan_cache_keys_on_registry_volatility() {
        let mut cache = PlanCache::new();
        let iri = format!("{GMEOW_NS}rel");
        let q = format!("SELECT ?s ?o WHERE {{ ?s <{iri}> ?o }}");
        let options = ParserOptions {
            extension_fn_namespaces: Vec::new(),
            property_fn_namespaces: Vec::new(),
            property_fn_iris: vec![iri.clone()],
        };

        let mut stable = crate::property_fn::PropertyFunctionRegistry::new();
        stable.register(
            iri.clone(),
            Arc::new(FixedVolatilityRelation::new(crate::Volatility::Stable)),
        );
        let mut volatile = crate::property_fn::PropertyFunctionRegistry::new();
        volatile.register(
            iri,
            Arc::new(FixedVolatilityRelation::new(crate::Volatility::Volatile)),
        );

        assert_ne!(
            crate::property_fn_plan::registry_fingerprint(&stable).expect("no panic"),
            crate::property_fn_plan::registry_fingerprint(&volatile).expect("no panic"),
            "the fingerprint itself must be sensitive to volatility"
        );

        let a = cache
            .prepare_with_relations(
                &q,
                None,
                &options,
                &stable,
                &crate::agg_fn::AggregateRegistry::EMPTY,
            )
            .expect("the stable registry admits and plans the call");
        let b = cache
            .prepare_with_relations(
                &q,
                None,
                &options,
                &volatile,
                &crate::agg_fn::AggregateRegistry::EMPTY,
            )
            .expect("the volatile registry admits and plans the call");
        assert!(
            !Arc::ptr_eq(&a, &b),
            "registries differing only in declared volatility must not share a cache entry"
        );
    }

    /// GAP-7 (governed receipt): two relation implementations registered under the SAME
    /// IRI, differing only in declared volatility, must produce DISTINGUISHABLE governed
    /// receipts and distinguishable explanations — never bytes that could be mistaken for
    /// the same execution.
    #[test]
    fn same_iri_different_volatility_produces_distinguishable_governed_receipts() {
        let ds = subst_ds();
        let engine = NativeSparqlEngine::new();
        let iri = "http://example.org/rel/shared";
        let query = format!("SELECT ?s ?o WHERE {{ ?s <{iri}> ?o }}");

        let mut stable = crate::property_fn::PropertyFunctionRegistry::new();
        stable.register(
            iri,
            Arc::new(FixedVolatilityRelation::new(crate::Volatility::Stable)),
        );
        let mut volatile = crate::property_fn::PropertyFunctionRegistry::new();
        volatile.register(
            iri,
            Arc::new(FixedVolatilityRelation::new(crate::Volatility::Volatile)),
        );

        let request = || SparqlRequest {
            query: query.as_str(),
            base_iri: None,
            substitutions: &[],
        };
        let run = |registry: &crate::property_fn::PropertyFunctionRegistry| {
            engine
                .query_governed(
                    &ds,
                    request(),
                    QueryOptions {
                        property_functions: registry,
                        ..QueryOptions::EMPTY
                    },
                    &QueryGovernors::METERED,
                )
                .expect("METERED bounds nothing, so both registries must admit and complete")
        };
        let stable_outcome = run(&stable);
        let volatile_outcome = run(&volatile);

        assert_ne!(
            stable_outcome.relations().fingerprint,
            volatile_outcome.relations().fingerprint,
            "two registries differing only in volatility must not carry the same \
             governed-receipt identity"
        );
        assert_eq!(
            stable_outcome.relations().iris,
            volatile_outcome.relations().iris,
            "the IRI list itself is identical — only the fingerprint tells them apart"
        );

        // The explain surface must disagree the same way.
        let explain = |registry: &crate::property_fn::PropertyFunctionRegistry| {
            engine
                .explain_query_with_options(
                    &ds,
                    &query,
                    None,
                    QueryOptions {
                        property_functions: registry,
                        ..QueryOptions::EMPTY
                    },
                )
                .expect("explain")
        };
        assert_ne!(
            explain(&stable).render(),
            explain(&volatile).render(),
            "two relation impls sharing an IRI must not render as the same explanation"
        );
    }

    // ── exotic aggregation ────────────────────────────────────────────────────

    const XSD_INT: &str = "http://www.w3.org/2001/XMLSchema#integer";

    /// A dataset for grouping/aggregation:
    /// `:r1 :a 1 ; :b 2`, `:r2 :a 1 ; :b 2`, `:r3 :a 2 ; :b 3`.
    fn numbers() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let pa = b.intern_iri("http://ex/a");
        let pb = b.intern_iri("http://ex/b");
        let int = |b: &mut RdfDatasetBuilder, n: &str| {
            b.intern_literal(RdfLiteral::typed(n.to_owned(), XSD_INT.to_owned()))
        };
        for (subj, a, bv) in [("r1", "1", "2"), ("r2", "1", "2"), ("r3", "2", "3")] {
            let s = b.intern_iri(&format!("http://ex/{subj}"));
            let av = int(&mut b, a);
            b.push_quad(s, pa, av, None);
            let bvv = int(&mut b, bv);
            b.push_quad(s, pb, bvv, None);
        }
        b.freeze().expect("freeze")
    }

    /// Render a result's rows as a sorted `Vec<Vec<String>>` for stable multiset
    /// comparison (IRIs as `<iri>`, literals as their lexical form).
    fn sorted_rows(result: SparqlResult) -> Vec<Vec<String>> {
        match result {
            SparqlResult::Solutions { rows, .. } => {
                let mut out: Vec<Vec<String>> = rows
                    .iter()
                    .map(|r| r.iter().map(|c| render_cell(c.as_ref())).collect())
                    .collect();
                out.sort();
                out
            }
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    fn render_cell(cell: Option<&TermValue>) -> String {
        match cell {
            None => "UNBOUND".to_owned(),
            Some(TermValue::Iri(i)) => format!("<{i}>"),
            Some(TermValue::Literal { lexical_form, .. }) => lexical_form.clone(),
            Some(TermValue::Blank { label, .. }) => format!("_:{label}"),
            Some(TermValue::Triple { .. }) => "<<triple>>".to_owned(),
        }
    }

    #[test]
    fn group_by_expression_with_as_binding() {
        // ?a+?b ∈ {3 (×2), 5 (×1)} → two groups counted.
        let r = run_on(
            &numbers(),
            "SELECT ?z (COUNT(*) AS ?c) WHERE { ?r <http://ex/a> ?a . ?r <http://ex/b> ?b } \
             GROUP BY (?a + ?b AS ?z)",
        );
        assert_eq!(sorted_rows(r), vec![vec!["3", "2"], vec!["5", "1"]]);
    }

    #[test]
    fn group_by_expression_without_projecting_the_synthetic_var() {
        // Selecting ONLY the aggregate must not leak the grouping column.
        let r = run_on(
            &numbers(),
            "SELECT (COUNT(*) AS ?c) WHERE { ?r <http://ex/a> ?a . ?r <http://ex/b> ?b } \
             GROUP BY (?a + ?b AS ?z)",
        );
        // Two groups → two count rows, single column each.
        assert_eq!(sorted_rows(r), vec![vec!["1"], vec!["2"]]);
    }

    #[test]
    fn group_by_bare_builtin_expression() {
        // `GROUP BY STR(?a)` (no AS → anonymous key) groups by the string form of
        // ?a ∈ {"1","1","2"} → two groups of sizes 2 and 1. The key is not
        // user-visible, so only the aggregate is projected.
        let r = run_on(
            &numbers(),
            "SELECT (COUNT(*) AS ?c) WHERE { ?r <http://ex/a> ?a } GROUP BY STR(?a)",
        );
        assert_eq!(sorted_rows(r), vec![vec!["1"], vec!["2"]]);
    }

    /// GROUP_CONCAT's deterministic reading (see `crate::modifier`'s "Aggregate
    /// semantics" docs): concatenation follows INPUT ROW ORDER, not "some
    /// order" — `numbers()` pushes `r1`(?a=1), `r2`(?a=1), `r3`(?a=2), in that
    /// order, and a single-pattern BGP scan visits them in insertion order, so
    /// the exact result is pinned to `"1|1|2"`.
    #[test]
    fn group_concat_with_separator() {
        let r = run_on(
            &numbers(),
            "SELECT (GROUP_CONCAT(?a; SEPARATOR=\"|\") AS ?g) \
             WHERE { ?r <http://ex/a> ?a }",
        );
        match r {
            SparqlResult::Solutions { rows, .. } => {
                assert_eq!(rows.len(), 1);
                let Some(TermValue::Literal { lexical_form, .. }) = &rows[0][0] else {
                    panic!("expected a literal");
                };
                assert_eq!(lexical_form, "1|1|2", "input-row-order concatenation");
            }
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    /// `DISTINCT` keeps the FIRST occurrence in row order (§18.6.1's `Dedup`):
    /// `numbers()`'s ?a sequence is `1, 1, 2` — the duplicate second `1` is
    /// dropped, leaving `"1|2"`, never `"2|1"`.
    #[test]
    fn group_concat_distinct_keeps_first_occurrence_in_row_order() {
        let r = run_on(
            &numbers(),
            "SELECT (GROUP_CONCAT(DISTINCT ?a; SEPARATOR=\"|\") AS ?g) \
             WHERE { ?r <http://ex/a> ?a }",
        );
        match r {
            SparqlResult::Solutions { rows, .. } => {
                assert_eq!(rows.len(), 1);
                let Some(TermValue::Literal { lexical_form, .. }) = &rows[0][0] else {
                    panic!("expected a literal");
                };
                assert_eq!(lexical_form, "1|2", "first occurrence per distinct value");
            }
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    /// Multi-group exact-string pin: two categories, each with its own row
    /// order, each group's GROUP_CONCAT concatenates ONLY its own rows in
    /// THEIR first-seen/inner-operator order — groups themselves in
    /// first-seen order (`cat/A` before `cat/B`, since `A`'s first row
    /// precedes `B`'s first row in the insertion order below).
    #[test]
    fn group_concat_multi_group_exact_order() {
        use purrdf_core::{RdfDatasetBuilder, RdfLiteral};
        let mut b = RdfDatasetBuilder::new();
        let cat = b.intern_iri("http://ex/cat");
        let item = b.intern_iri("http://ex/item");
        let a = b.intern_iri("http://ex/A");
        let bb = b.intern_iri("http://ex/B");
        // Row order: (A,"x"),(B,"p"),(A,"y"),(B,"q"),(A,"z").
        for (subj, catv, val) in [
            ("s1", a, "x"),
            ("s2", bb, "p"),
            ("s3", a, "y"),
            ("s4", bb, "q"),
            ("s5", a, "z"),
        ] {
            let s = b.intern_iri(&format!("http://ex/{subj}"));
            b.push_quad(s, cat, catv, None);
            let v = b.intern_literal(RdfLiteral::simple(val));
            b.push_quad(s, item, v, None);
        }
        let ds = b.freeze().expect("freeze");
        let r = run_on(
            &ds,
            "SELECT ?c (GROUP_CONCAT(?v; SEPARATOR=\",\") AS ?g) \
             WHERE { ?s <http://ex/cat> ?c . ?s <http://ex/item> ?v } \
             GROUP BY ?c",
        );
        match r {
            SparqlResult::Solutions { rows, .. } => {
                assert_eq!(rows.len(), 2);
                let render = |cell: &Option<TermValue>| match cell {
                    Some(TermValue::Iri(iri)) => iri.clone(),
                    Some(TermValue::Literal { lexical_form, .. }) => lexical_form.clone(),
                    other => format!("{other:?}"),
                };
                let mut got: Vec<(String, String)> = rows
                    .iter()
                    .map(|r| (render(&r[0]), render(&r[1])))
                    .collect();
                got.sort();
                assert_eq!(
                    got,
                    vec![
                        ("http://ex/A".to_owned(), "x,y,z".to_owned()),
                        ("http://ex/B".to_owned(), "p,q".to_owned()),
                    ]
                );
            }
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    /// A group containing a blank node poisons GROUP_CONCAT to unbound — a
    /// blank node has no lexical form, and `STR()` of one is a SPARQL type
    /// error, so this crate's owned reading treats it like SUM/AVG's poison-
    /// on-non-numeric rather than silently dropping the value (see
    /// `crate::modifier::lexical_of`'s docs). No W3C conformance case exercises
    /// GROUP_CONCAT over a blank node (verified against the frozen corpus), so
    /// this decision has no upstream evidence to reconcile against.
    #[test]
    fn group_concat_over_blank_node_poisons_to_unbound() {
        use purrdf_core::{BlankScope, RdfDatasetBuilder, RdfLiteral};
        let mut b = RdfDatasetBuilder::new();
        let item = b.intern_iri("http://ex/item");
        let s = b.intern_iri("http://ex/s");
        let lit = b.intern_literal(RdfLiteral::simple("lit1"));
        let bn = b.intern_blank("bn", BlankScope::DEFAULT);
        b.push_quad(s, item, lit, None);
        b.push_quad(s, item, bn, None);
        let ds = b.freeze().expect("freeze");
        let r = run_on(
            &ds,
            "SELECT (GROUP_CONCAT(?v; SEPARATOR=\"|\") AS ?g) WHERE { <http://ex/s> <http://ex/item> ?v }",
        );
        match r {
            SparqlResult::Solutions { rows, .. } => {
                assert_eq!(rows.len(), 1);
                assert!(
                    rows[0][0].is_none(),
                    "GROUP_CONCAT over a blank node must be unbound, got {:?}",
                    rows[0][0]
                );
            }
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    /// A group containing an RDF-star triple term likewise poisons
    /// GROUP_CONCAT to unbound — same reasoning as the blank-node case above.
    #[test]
    fn group_concat_over_triple_term_poisons_to_unbound() {
        let ds = numbers();
        let r = run_on(
            &ds,
            "PREFIX ex: <http://ex/> \
             SELECT (GROUP_CONCAT(?v; SEPARATOR=\"|\") AS ?g) WHERE { \
               VALUES ?v { \"lit1\" <<ex:s ex:p ex:o>> } \
             }",
        );
        match r {
            SparqlResult::Solutions { rows, .. } => {
                assert_eq!(rows.len(), 1);
                assert!(
                    rows[0][0].is_none(),
                    "GROUP_CONCAT over a triple term must be unbound, got {:?}",
                    rows[0][0]
                );
            }
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    /// `MIN`/`MAX` over a group mixing every term kind — a plain literal, an
    /// `xsd:integer`, an IRI, and an RDF-star triple term — order per the
    /// SPARQL `ORDER BY` total order (§15.1), exactly as SPARQL 1.2 §18.6.1.5
    /// (`Min`) / §18.6.1.6 (`Max`) define them ("ordered as per the ORDER BY
    /// ASC/DESC clause" then take the first element): kind rank blank < IRI <
    /// literal < triple, so `MIN` (ascending) picks the IRI (the lowest-ranked
    /// bound kind present) and `MAX` (descending) picks the triple term (the
    /// highest-ranked). See `crate::modifier`'s "Aggregate semantics" docs,
    /// `MIN`/`MAX` section, for the spec citation and why this crate's
    /// `term_value_order` needs no change to match it.
    #[test]
    fn min_max_over_mixed_kind_group_follow_order_by_total_order() {
        let ds = numbers();
        let r = run_on(
            &ds,
            "PREFIX ex: <http://ex/> \
             SELECT (MIN(?v) AS ?mn) (MAX(?v) AS ?mx) WHERE { \
               VALUES ?v { \"plain\" 5 ex:iri <<ex:s ex:p ex:o>> } \
             }",
        );
        match r {
            SparqlResult::Solutions { rows, .. } => {
                assert_eq!(rows.len(), 1);
                match &rows[0][0] {
                    Some(TermValue::Iri(iri)) => assert_eq!(iri, "http://ex/iri"),
                    other => panic!("MIN over a mixed-kind group must be the IRI, got {other:?}"),
                }
                match &rows[0][1] {
                    Some(TermValue::Triple { .. }) => {}
                    other => {
                        panic!("MAX over a mixed-kind group must be the triple term, got {other:?}")
                    }
                }
            }
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    #[test]
    fn sample_returns_a_group_member() {
        let r = run_on(
            &numbers(),
            "SELECT (SAMPLE(?a) AS ?s) WHERE { ?r <http://ex/a> ?a }",
        );
        let rows = sorted_rows(r);
        assert_eq!(rows.len(), 1);
        assert!(
            rows[0][0] == "1" || rows[0][0] == "2",
            "got {:?}",
            rows[0][0]
        );
    }

    #[test]
    fn sum_of_expression_inside_aggregate() {
        // SUM(?a + ?b) over the three rows = (1+2)+(1+2)+(2+3) = 11.
        let r = run_on(
            &numbers(),
            "SELECT (SUM(?a + ?b) AS ?t) WHERE { ?r <http://ex/a> ?a . ?r <http://ex/b> ?b }",
        );
        assert_eq!(sorted_rows(r), vec![vec!["11"]]);
    }

    #[test]
    fn arithmetic_across_aggregate_results() {
        // (SUM(?a) / COUNT(?a)) = (1+1+2)/3 — exercises an Extend over two
        // aggregate-result variables. Assert it produces a single bound row.
        let r = run_on(
            &numbers(),
            "SELECT (SUM(?a) AS ?s) (COUNT(?a) AS ?n) ((SUM(?a)/COUNT(?a)) AS ?avg) \
             WHERE { ?r <http://ex/a> ?a }",
        );
        match r {
            SparqlResult::Solutions { rows, .. } => {
                assert_eq!(rows.len(), 1);
                assert_eq!(render_cell(rows[0][0].as_ref()), "4"); // SUM = 1+1+2
                assert_eq!(render_cell(rows[0][1].as_ref()), "3"); // COUNT = 3
                assert!(rows[0][2].is_some(), "the ratio must be bound");
            }
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    #[test]
    fn having_over_an_aggregate_not_in_select() {
        // Group ?a; keep only groups whose SUM(?b) exceeds 3. Group ?a=1 has
        // rows r1,r2 (?b=2 each) → SUM=4 > 3 (kept); group ?a=2 has r3 (?b=3) →
        // SUM=3, not > 3 (dropped).
        let r = run_on(
            &numbers(),
            "SELECT ?a WHERE { ?r <http://ex/a> ?a . ?r <http://ex/b> ?b } \
             GROUP BY ?a HAVING (SUM(?b) > 3)",
        );
        assert_eq!(sorted_rows(r), vec![vec!["1"]]);
    }

    #[test]
    fn complex_having_conjunction() {
        // COUNT(*) > 1 && AVG(?b) < 5 — only the ?a=1 group (count 2, avg 2).
        let r = run_on(
            &numbers(),
            "SELECT ?a WHERE { ?r <http://ex/a> ?a . ?r <http://ex/b> ?b } \
             GROUP BY ?a HAVING (COUNT(*) > 1 && AVG(?b) < 5)",
        );
        assert_eq!(sorted_rows(r), vec![vec!["1"]]);
    }

    // ── dataset-aware BGP order cache ──────────────────────────────────────────

    /// social() plus an extra `:a :knows :c` edge — same predicates, a different quad
    /// count (3 vs 2) so a different `stats_fingerprint`.
    fn social_plus() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let knows = b.intern_iri("http://ex/knows");
        let name = b.intern_iri("http://ex/name");
        let a = b.intern_iri("http://ex/a");
        let bb = b.intern_iri("http://ex/b");
        let c = b.intern_iri("http://ex/c");
        let ann = b.intern_literal(RdfLiteral::simple("Ann"));
        b.push_quad(a, knows, bb, None);
        b.push_quad(a, knows, c, None);
        b.push_quad(a, name, ann, None);
        b.freeze().expect("freeze")
    }

    const TWO_PATTERN_BGP: &str = "SELECT ?o ?n WHERE { \
         <http://ex/a> <http://ex/knows> ?o . <http://ex/a> <http://ex/name> ?n }";

    /// A repeated query against the same dataset plans its BGP once and reuses the
    /// cached order: the engine holds a single entry whose `Arc` is the *same
    /// allocation* before and after the second run (a cache miss would replace it).
    #[test]
    fn order_cache_populates_and_reuses() {
        let ds = social();
        let engine = NativeSparqlEngine::new();
        let req = || SparqlRequest {
            query: TWO_PATTERN_BGP,
            base_iri: None,
            substitutions: &[],
        };

        engine.query(&ds, req()).expect("first query");
        assert_eq!(
            engine
                .order_cache
                .read()
                .expect("order cache lock poisoned")
                .len(),
            1,
            "one BGP cached"
        );
        let first = engine
            .order_cache
            .read()
            .expect("order cache lock poisoned")
            .values()
            .next()
            .expect("cached order")
            .clone();

        engine.query(&ds, req()).expect("second query");
        assert_eq!(
            engine
                .order_cache
                .read()
                .expect("order cache lock poisoned")
                .len(),
            1,
            "no duplicate entry"
        );
        let second = engine
            .order_cache
            .read()
            .expect("order cache lock poisoned")
            .values()
            .next()
            .expect("cached order")
            .clone();

        assert!(
            Arc::ptr_eq(&first, &second),
            "the second run reused the cached order, not re-planned"
        );
    }

    /// The same query text against two datasets with different stats fingerprints keys
    /// to two distinct cache entries (a cost-based order is dataset-specific), and both
    /// runs return correct results.
    #[test]
    fn order_cache_misses_on_a_different_dataset() {
        let engine = NativeSparqlEngine::new();
        let req = || SparqlRequest {
            query: TWO_PATTERN_BGP,
            base_iri: None,
            substitutions: &[],
        };

        let small = social(); // 2 quads → :a knows {:b}; :a name "Ann"  ⇒ 1 row.
        let r1 = engine.query(&small, req()).expect("small query");
        let SparqlResult::Solutions { rows, .. } = r1 else {
            panic!("expected solutions");
        };
        assert_eq!(rows.len(), 1);

        let big = social_plus(); // 3 quads → :a knows {:b,:c} ⇒ 2 rows.
        let r2 = engine.query(&big, req()).expect("big query");
        let SparqlResult::Solutions { rows, .. } = r2 else {
            panic!("expected solutions");
        };
        assert_eq!(rows.len(), 2);

        assert_eq!(
            engine
                .order_cache
                .read()
                .expect("order cache lock poisoned")
                .len(),
            2,
            "distinct datasets ⇒ distinct fingerprints ⇒ two cache entries"
        );
    }

    /// `NativeSparqlEngine::new()` needs no injected clock: `NOW()` reads the real
    /// host wall clock by construction (`EvalCtx::new` → `crate::clock::wall_clock_now`).
    #[test]
    fn default_engine_now_is_current_wall_clock() {
        let r = run_on(&social(), "SELECT (YEAR(NOW()) AS ?y) WHERE {}");
        match r {
            SparqlResult::Solutions { rows, .. } => {
                assert_eq!(rows.len(), 1);
                let year: i64 = render_cell(rows[0][0].as_ref())
                    .parse()
                    .expect("YEAR(NOW()) must render as an integer");
                assert!(year >= 2025, "expected a current year, got {year}");
            }
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    /// All `NOW()` call sites within a single query observe the same instant
    /// (SPARQL 1.1 §17.4.5.1): `EvalCtx::new` samples the wall clock exactly once
    /// per query, not once per `NOW()` call site.
    #[test]
    fn now_is_constant_within_one_query() {
        let r = run_on(&social(), "SELECT (NOW() AS ?a) (NOW() AS ?b) WHERE {}");
        match r {
            SparqlResult::Solutions { rows, .. } => {
                assert_eq!(rows.len(), 1);
                assert_eq!(
                    render_cell(rows[0][0].as_ref()),
                    render_cell(rows[0][1].as_ref()),
                    "?a and ?b must see the same sampled instant: {rows:?}"
                );
            }
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    /// The `explain_query` API returns a non-empty ordered list of triple-pattern
    /// strings for a multi-pattern BGP, proving the planner chose an order and that
    /// the introspection path does not leak internal types.
    #[test]
    fn explain_query_returns_non_empty_order_for_multi_pattern_bgp() {
        let ds = social();
        let engine = NativeSparqlEngine::new();
        let explanation = engine
            .explain_query(
                &ds,
                "SELECT ?o ?n WHERE { \
                 <http://ex/a> <http://ex/knows> ?o . \
                 <http://ex/a> <http://ex/name> ?n }",
                None,
            )
            .expect("explain");
        let plan = explanation.join_orders();
        assert!(
            plan.len() >= 2,
            "expected at least two triple-pattern strings, got {plan:?}"
        );
        // Both patterns are present (order may vary with cardinality, but both IRIs
        // are constants so the planner still has to schedule both).
        let has_knows = plan.iter().any(|s| s.contains("<http://ex/knows>"));
        let has_name = plan.iter().any(|s| s.contains("<http://ex/name>"));
        assert!(has_knows, "explain output missing knows pattern: {plan:?}");
        assert!(has_name, "explain output missing name pattern: {plan:?}");
    }

    /// `explain_query` errors cleanly on malformed SPARQL.
    #[test]
    fn explain_query_rejects_malformed_sparql() {
        let ds = social();
        let engine = NativeSparqlEngine::new();
        let err = engine.explain_query(&ds, "SELECT ?x WHERE { not sparql }", None);
        assert!(err.is_err(), "malformed query must produce a diagnostic");
    }

    /// `RAND()`/`UUID()`/`STRUUID()` are seeded from live OS entropy, not a fixed
    /// default: fresh engines across repeated runs must not all agree. A single pair
    /// differing is overwhelmingly likely but not guaranteed, so run a handful of
    /// times and require not-all-identical.
    #[test]
    fn rand_is_live_across_queries() {
        let values: Vec<String> = (0..4)
            .map(|_| {
                let r = run_on(&social(), "SELECT (STRUUID() AS ?u) WHERE {}");
                match r {
                    SparqlResult::Solutions { rows, .. } => {
                        assert_eq!(rows.len(), 1);
                        render_cell(rows[0][0].as_ref())
                    }
                    other => panic!("expected solutions, got {other:?}"),
                }
            })
            .collect();
        assert!(
            values.windows(2).any(|w| w[0] != w[1]),
            "expected live entropy to vary across queries, got identical values: {values:?}"
        );
    }
}
