// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The native [`SparqlEngine`] implementation and its parse-memoizing plan cache.
//!
//! [`NativeSparqlEngine`] is the single required impl of the `purrdf-core`
//! `SparqlEngine` seam — the native replacement for the oxigraph-family
//! `spareval` on the query path. Its `Dataset` is the concrete frozen
//! [`RdfDataset`]: the evaluator needs `term_id_by_value` (P4), which is an
//! inherent method on the dataset rather than part of the `DatasetView` trait.
//!
//! The [`PlanCache`] memoizes parsing so the static generated query corpus compiles
//! to algebra once, not per run. Full cost-based planning is out of scope here; the
//! cache holds only the parsed [`Query`].

use std::cell::RefCell;
use std::sync::Arc;

use purrdf_core::{
    DatasetView, GraphMatch, MutableDataset, RdfDataset, RdfDiagnostic, SparqlEngine,
    SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_algebra::{ParserOptions, Query, SparqlParser};

use crate::DetHashMap;
use crate::dataset_spec::ActiveDataset;
use crate::eval::{
    BgpOrderCache, EvalCtx, EvalOptions, LossVocabulary, Outcome, StandpointPredicates,
    evaluate_query,
};
use crate::update::{GraphResolver, eval_update};

/// A parsed, ready-to-evaluate query (the cached unit of the [`PlanCache`]).
#[derive(Debug)]
pub struct PreparedQuery {
    /// The parsed algebra.
    pub query: Query,
}

/// A parse-memoizing cache keyed on `(base IRI, extension-function namespace set,
/// query text)`.
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
        // The cache key must include the base IRI AND the extension-function
        // namespace set: the same text under a different base or namespace
        // configuration parses to a different algebra.
        let key = format!(
            "{}\u{0}{}\u{0}{}",
            base_iri.unwrap_or(""),
            options.extension_fn_namespaces.join("\u{1}"),
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
        let prepared = Arc::new(PreparedQuery { query: parsed });
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
        self.cache
            .borrow_mut()
            .prepare_with(query, base_iri, &self.parser_options)
    }

    /// Evaluate a plan returned by [`Self::prepare_query`].
    ///
    /// # Errors
    ///
    /// Propagates evaluation errors as an [`RdfDiagnostic`].
    pub fn query_prepared(
        &self,
        dataset: &Arc<RdfDataset>,
        prepared: &PreparedQuery,
        substitutions: &[(String, TermValue)],
    ) -> Result<SparqlResult, RdfDiagnostic> {
        self.query_prepared_view(&**dataset, prepared, substitutions)
    }

    /// [`Self::query_prepared`] over any [`DatasetView`] backend whose id type is the
    /// production [`TermId`](purrdf_core::TermId). The concrete [`Self::query_prepared`] is a thin wrapper
    /// that derefs its `Arc<RdfDataset>` and calls this.
    ///
    /// # Errors
    ///
    /// Propagates evaluation errors as an [`RdfDiagnostic`].
    pub fn query_prepared_view<D: DatasetView + Sync>(
        &self,
        dataset: &D,
        prepared: &PreparedQuery,
        substitutions: &[(String, TermValue)],
    ) -> Result<SparqlResult, RdfDiagnostic> {
        let mut ctx = self.eval_ctx(dataset);
        let outcome = evaluate_with_substitutions(prepared, substitutions, &mut ctx)?;
        Ok(materialize(outcome, &ctx))
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

    /// Explain the cost-based BGP join order the engine would choose for
    /// `query_text` against `dataset`.
    ///
    /// Returns an ordered list of triple-pattern strings: for every BGP in the
    /// query with at least two triple patterns, the patterns are listed in the
    /// order the planner selected. BGPs are visited in a left-to-right DFS over
    /// the algebra, so subqueries, OPTIONAL/UNION branches, and GRAPH blocks are
    /// all represented in query-text order. This is a pure-introspection API: it
    /// does not evaluate the query and does not mutate the engine state.
    ///
    /// # Errors
    ///
    /// Returns an [`RdfDiagnostic`] if the query text does not parse.
    pub fn explain_query(
        &self,
        dataset: &Arc<RdfDataset>,
        query_text: &str,
        base_iri: Option<&str>,
    ) -> Result<Vec<String>, RdfDiagnostic> {
        self.explain_query_view(&**dataset, query_text, base_iri)
    }

    /// [`Self::explain_query`] over any [`DatasetView`] backend whose id type is the
    /// production [`TermId`](purrdf_core::TermId). The cost-based join order is computed against the given
    /// view's cardinalities exactly as the concrete path does.
    ///
    /// # Errors
    ///
    /// Returns an [`RdfDiagnostic`] if the query text does not parse.
    pub fn explain_query_view<D: DatasetView + Sync>(
        &self,
        dataset: &D,
        query_text: &str,
        base_iri: Option<&str>,
    ) -> Result<Vec<String>, RdfDiagnostic> {
        let prepared =
            self.cache
                .borrow_mut()
                .prepare_with(query_text, base_iri, &self.parser_options)?;
        let active_dataset = ActiveDataset::from_query_dataset(prepared.query.dataset(), dataset);
        let mut out = Vec::new();
        let pattern = match &prepared.query {
            Query::Select { pattern, .. } | Query::Ask { pattern, .. } => pattern,
            Query::Construct { pattern, .. } | Query::Describe { pattern, .. } => pattern,
        };
        crate::bgp::explain_pattern_orders(
            dataset,
            &active_dataset,
            GraphMatch::Default,
            pattern,
            &mut out,
        )
        .map_err(|e| RdfDiagnostic::error("native-sparql-query-explain", e.to_string()))?;
        Ok(out)
    }

    /// Evaluate a SPARQL query under SHACL-SPARQL pre-binding semantics.
    ///
    /// Pre-bound variables are substituted into FILTER/EXISTS expressions and
    /// `BOUND($v)` is rewritten to `true` for pre-bound variables, while
    /// triple-pattern positions still receive the VALUES-join rewrite. This is
    /// the path used by the SHACL validator for `sh:select` / `sh:ask` bodies;
    /// normal SPARQL evaluation uses [`SparqlEngine::query`].
    ///
    /// # Errors
    ///
    /// Propagates parse/evaluation errors as an [`RdfDiagnostic`].
    pub fn query_with_shacl_prebinding(
        &self,
        dataset: &Arc<RdfDataset>,
        query: &str,
        base_iri: Option<&str>,
        substitutions: &[(String, TermValue)],
    ) -> Result<SparqlResult, RdfDiagnostic> {
        self.query_with_shacl_prebinding_view(&**dataset, query, base_iri, substitutions)
    }

    /// [`Self::query_with_shacl_prebinding`] over any [`DatasetView`] backend whose id
    /// type is the production [`TermId`](purrdf_core::TermId).
    ///
    /// # Errors
    ///
    /// Propagates parse/evaluation errors as an [`RdfDiagnostic`].
    pub fn query_with_shacl_prebinding_view<D: DatasetView + Sync>(
        &self,
        dataset: &D,
        query: &str,
        base_iri: Option<&str>,
        substitutions: &[(String, TermValue)],
    ) -> Result<SparqlResult, RdfDiagnostic> {
        let prepared =
            self.cache
                .borrow_mut()
                .prepare_with(query, base_iri, &self.parser_options)?;
        let mut ctx = self.eval_ctx(dataset);
        let outcome = evaluate_with_shacl_prebinding(&prepared, substitutions, &mut ctx)?;
        Ok(materialize(outcome, &ctx))
    }

    /// Like [`NativeSparqlEngine::query_with_shacl_prebinding`], but with a SHACL-AF
    /// function registry in scope so `sh:sparql` bodies can call declared functions.
    ///
    /// # Errors
    ///
    /// Propagates parse/evaluation errors as an [`RdfDiagnostic`].
    pub fn query_with_shacl_prebinding_and_functions(
        &self,
        dataset: &Arc<RdfDataset>,
        query: &str,
        base_iri: Option<&str>,
        substitutions: &[(String, TermValue)],
        registry: &crate::user_fn::UserFunctionRegistry,
    ) -> Result<SparqlResult, RdfDiagnostic> {
        self.query_with_shacl_prebinding_and_functions_view(
            &**dataset,
            query,
            base_iri,
            substitutions,
            registry,
        )
    }

    /// [`Self::query_with_shacl_prebinding_and_functions`] over any [`DatasetView`]
    /// backend whose id type is the production [`TermId`](purrdf_core::TermId).
    ///
    /// # Errors
    ///
    /// Propagates parse/evaluation errors as an [`RdfDiagnostic`].
    pub fn query_with_shacl_prebinding_and_functions_view<D: DatasetView + Sync>(
        &self,
        dataset: &D,
        query: &str,
        base_iri: Option<&str>,
        substitutions: &[(String, TermValue)],
        registry: &crate::user_fn::UserFunctionRegistry,
    ) -> Result<SparqlResult, RdfDiagnostic> {
        let prepared =
            self.cache
                .borrow_mut()
                .prepare_with(query, base_iri, &self.parser_options)?;
        let mut ctx = self.eval_ctx(dataset).with_user_functions(registry);
        let outcome = evaluate_with_shacl_prebinding(&prepared, substitutions, &mut ctx)?;
        Ok(materialize(outcome, &ctx))
    }

    /// Like [`SparqlEngine::query`], but with a
    /// [`RemoteQuerySource`](crate::remote::RemoteQuerySource) injected so
    /// `SERVICE` clauses resolve through it. Without this, the default
    /// [`SparqlEngine::query`] path has no source and a non-silent `SERVICE`
    /// hard-fails. This is the public entry the conformance harness and
    /// federated callers use.
    ///
    /// # Errors
    ///
    /// Propagates parse and evaluation errors as an [`RdfDiagnostic`].
    pub fn query_with_source(
        &self,
        dataset: &Arc<RdfDataset>,
        request: SparqlRequest<'_>,
        source: &(dyn crate::remote::RemoteQuerySource + Sync),
    ) -> Result<SparqlResult, RdfDiagnostic> {
        self.query_with_source_view(&**dataset, request, source)
    }

    /// [`Self::query_with_source`] over any [`DatasetView`] backend whose id type is
    /// the production [`TermId`](purrdf_core::TermId).
    ///
    /// # Errors
    ///
    /// Propagates parse and evaluation errors as an [`RdfDiagnostic`].
    pub fn query_with_source_view<D: DatasetView + Sync>(
        &self,
        dataset: &D,
        request: SparqlRequest<'_>,
        source: &(dyn crate::remote::RemoteQuerySource + Sync),
    ) -> Result<SparqlResult, RdfDiagnostic> {
        let prepared = self.cache.borrow_mut().prepare_with(
            request.query,
            request.base_iri,
            &self.parser_options,
        )?;
        let mut ctx = self.eval_ctx(dataset).with_remote(source);
        let outcome = evaluate_with_substitutions(&prepared, request.substitutions, &mut ctx)?;
        Ok(materialize(outcome, &ctx))
    }

    /// Like [`SparqlEngine::query`], but with a caller-supplied SHACL-AF function
    /// registry (`sh:SPARQLFunction`) injected so a call-position IRI resolving to a
    /// declared function evaluates its body at eval time. This is the entry the
    /// shapes validator uses; the registry is built once per shapes graph and
    /// borrowed for the call. An empty registry behaves exactly like [`Self::query`].
    ///
    /// # Errors
    ///
    /// Propagates parse and evaluation errors as an [`RdfDiagnostic`].
    pub fn query_with_user_functions(
        &self,
        dataset: &Arc<RdfDataset>,
        request: SparqlRequest<'_>,
        registry: &crate::user_fn::UserFunctionRegistry,
    ) -> Result<SparqlResult, RdfDiagnostic> {
        self.query_with_user_functions_view(&**dataset, request, registry)
    }

    /// [`Self::query_with_user_functions`] over any [`DatasetView`] backend whose id
    /// type is the production [`TermId`](purrdf_core::TermId).
    ///
    /// # Errors
    ///
    /// Propagates parse and evaluation errors as an [`RdfDiagnostic`].
    pub fn query_with_user_functions_view<D: DatasetView + Sync>(
        &self,
        dataset: &D,
        request: SparqlRequest<'_>,
        registry: &crate::user_fn::UserFunctionRegistry,
    ) -> Result<SparqlResult, RdfDiagnostic> {
        let prepared = self.cache.borrow_mut().prepare_with(
            request.query,
            request.base_iri,
            &self.parser_options,
        )?;
        let mut ctx = self.eval_ctx(dataset).with_user_functions(registry);
        let outcome = evaluate_with_substitutions(&prepared, request.substitutions, &mut ctx)?;
        Ok(materialize(outcome, &ctx))
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
        self.query_prepared(dataset, &prepared, request.substitutions)
    }

    fn update(
        &self,
        dataset: &mut Self::Dataset,
        request: SparqlRequest<'_>,
    ) -> Result<(), RdfDiagnostic> {
        // UPDATE deliberately bypasses the plan cache: these requests are
        // side-effecting and are not the hot static-query set the cache exists for;
        // caching a mutating statement would be a correctness hazard.
        let mut parser = SparqlParser::new();
        if let Some(base) = request.base_iri {
            parser = parser.with_base_iri(base);
        }
        let update = parser
            .parse_update_with(request.query, &self.parser_options)
            .map_err(|e| RdfDiagnostic::error("native-sparql-update-parse", e.to_string()))?;
        // Atomicity is structural: branch a COW MutableDataset off the frozen base,
        // apply every op to the delta, and only on FULL success freeze back. Any
        // error drops `m` and leaves `*dataset` untouched.
        let mut m = MutableDataset::new(Arc::clone(dataset));
        let cfg = crate::update::UpdateEvalConfig {
            standpoint_predicates: self.standpoint_predicates.as_ref(),
            order_cache: &self.order_cache,
        };
        eval_update(&update, &mut m, self.resolver.as_deref(), &cfg)?;
        *dataset = m.freeze()?;
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_core::{BlankScope, RdfDatasetBuilder, RdfLiteral, TermValue};

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
            .query_with_shacl_prebinding(&ds, query, None, substitutions)
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
        fn resolve(&self, _iri: &str) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
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

    #[test]
    fn group_concat_with_separator() {
        let r = run_on(
            &numbers(),
            "SELECT (GROUP_CONCAT(?a; SEPARATOR=\"|\") AS ?g) \
             WHERE { ?r <http://ex/a> ?a }",
        );
        // Implicit single group; lexical values of ?a joined by '|', some order.
        match r {
            SparqlResult::Solutions { rows, .. } => {
                assert_eq!(rows.len(), 1);
                let Some(TermValue::Literal { lexical_form, .. }) = &rows[0][0] else {
                    panic!("expected a literal");
                };
                let mut parts: Vec<&str> = lexical_form.split('|').collect();
                parts.sort_unstable();
                assert_eq!(parts, vec!["1", "1", "2"]);
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
        let plan = engine
            .explain_query(
                &ds,
                "SELECT ?o ?n WHERE { \
                 <http://ex/a> <http://ex/knows> ?o . \
                 <http://ex/a> <http://ex/name> ?n }",
                None,
            )
            .expect("explain");
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
