// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Pure SPARQL evaluation helpers for SHACL-AF, over the native engine.
//!
//! [`eval_target`] runs a `sh:SPARQLTarget` SELECT query and returns the bound
//! `?this` focus nodes. [`eval_sparql_constraint`] runs a `sh:SPARQLConstraint`
//! SELECT query with `$this`/`?this` pre-bound to the focus node and maps each
//! solution row to a [`ValidationResult`].
//!
//! Both run the [`NativeSparqlEngine`] over the borrowed `Arc<RdfDataset>` — there is
//! no oxigraph SPARQL engine and no materialized `Store`. Focus-node substitution
//! uses [`SparqlRequest::substitutions`] (the native replacement for oxigraph's
//! `PreparedSparqlQuery::substitute_variable`,  GAP-A).

use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;

use ::purrdf::{DatasetView, RdfDataset};
use ::purrdf::{SparqlRequest, SparqlResult, TermValue};
use purrdf_sparql_eval::{
    AggregateRegistry, GovernedOutcome, GovernorState, NativeSparqlEngine,
    PropertyFunctionRegistry, QueryOptions, ShaclPrebinding, UserFunctionRegistry,
};

use crate::model::xsd;
use crate::report::{Severity, ValidationResult};
use crate::term::{Literal, NamedNode, Term, term_value_to_native};

// ── Public API ────────────────────────────────────────────────────────────────

/// Execute a SHACL-AF `sh:SPARQLTarget` SELECT query against `dataset`.
///
/// The query **must** be a SELECT that binds `?this` in every solution row; any
/// other query form or a missing `?this` binding is a hard error.
///
/// `substitutions` pre-binds SPARQL variables (used for `sh:SPARQLTargetType`
/// parameter values). Each `(var_name, term)` pair is bound to `?var_name`.
///
/// The returned [`Vec<Term>`] is deduplicated and sorted by string representation
/// so the focus-node set is deterministic across runs.
///
/// # Errors
///
/// Returns `Err(String)` if execution fails, if the result is not a SELECT
/// (`Boolean` / `Graph` are rejected), or if any solution row has no `?this`
/// binding.
pub fn eval_target(
    dataset: &Arc<RdfDataset>,
    select: &str,
    substitutions: &[(String, Term)],
) -> Result<Vec<Term>, String> {
    eval_target_view(&**dataset, select, substitutions)
}

/// Internal view-generic implementation of [`eval_target`].
pub(crate) fn eval_target_view<D: DatasetView + Sync>(
    dataset: &D,
    select: &str,
    substitutions: &[(String, Term)],
) -> Result<Vec<Term>, String> {
    let subs: Vec<(String, TermValue)> = substitutions
        .iter()
        .map(|(name, term)| (name.clone(), term.to_term_value()))
        .collect();
    let solutions =
        run_select_generic_view(dataset, select, &subs).map_err(|e| format!("SPARQLTarget {e}"))?;

    let this_index = column_index(&solutions.0, "this");

    let mut nodes: Vec<Term> = Vec::new();
    for row in &solutions.1 {
        match this_index.and_then(|i| row.get(i)).and_then(Option::as_ref) {
            Some(value) => nodes.push(term_value_to_native(value)),
            None => {
                return Err(
                    "SPARQLTarget query produced a solution row with no ?this binding".to_owned(),
                );
            }
        }
    }

    crate::term::sort_terms_canonical(&mut nodes);
    nodes.dedup();
    Ok(nodes)
}

/// Execute a SHACL-AF `sh:SPARQLConstraint` SELECT query for a single focus node,
/// mapping each solution row to a [`ValidationResult`].
///
/// `$this` is pre-bound to `focus`, and when known `$shapesGraph` and
/// `$currentShape` are pre-bound to `shapes_graph_iri` and `current_shape`, before
/// the query is run through the SHACL-specific pre-binding rewrite. Each solution
/// row produces exactly one result:
///
/// | SPARQL binding | `ValidationResult` field |
/// |---|---|
/// | `?path` | `result_path` (optional) |
/// | `?value` | `value` (optional) |
///
/// `component`, `source_shape`, `severity`, and `message` are taken from the caller
/// and are the same for every row.
///
/// Results are returned in solution order; the caller (engine) sorts the final
/// report deterministically.
///
/// # Errors
///
/// Returns `Err(String)` if execution fails or if the result is not a SELECT.
#[allow(clippy::too_many_arguments)] // Signature mirrors the SHACL-SPARQL parameter set.
pub fn eval_sparql_constraint(
    dataset: &Arc<RdfDataset>,
    focus: &Term,
    select: &str,
    component: &NamedNode,
    source_shape: &Term,
    severity: &Severity,
    message: Option<&String>,
    shapes_graph_iri: Option<&str>,
    current_shape: Option<&Term>,
) -> Result<Vec<ValidationResult>, String> {
    eval_sparql_constraint_view(
        &**dataset,
        focus,
        select,
        component,
        source_shape,
        severity,
        message,
        shapes_graph_iri,
        current_shape,
    )
}

/// Internal view-generic implementation of [`eval_sparql_constraint`].
#[allow(clippy::too_many_arguments)] // Signature mirrors the SHACL-SPARQL parameter set.
pub(crate) fn eval_sparql_constraint_view<D: DatasetView + Sync>(
    dataset: &D,
    focus: &Term,
    select: &str,
    component: &NamedNode,
    source_shape: &Term,
    severity: &Severity,
    message: Option<&String>,
    shapes_graph_iri: Option<&str>,
    current_shape: Option<&Term>,
) -> Result<Vec<ValidationResult>, String> {
    // Pre-bind `$this` to THIS focus node (GAP-A substitution — the native
    // replacement for oxigraph's per-focus `PreparedSparqlQuery::substitute_variable`).
    // This MUST be per-focus substitution, not an unsubstituted run grouped by a free
    // `$this`: a constraint whose `$this` appears only inside a `FILTER NOT EXISTS`/
    // negation has no positive binding for `$this` when run unsubstituted, so the
    // unsubstituted query returns no rows and silently drops the violation. The parse
    // is memoized by the thread-local engine's plan cache, so per-focus evaluation
    // re-runs the plan, not the parse.
    let subs = [("this".to_owned(), focus.to_term_value())];
    let (variables, rows) = run_select_with_shacl_prebinding_view(
        dataset,
        select,
        &subs,
        shapes_graph_iri,
        current_shape,
    )
    .map_err(|e| format!("SPARQLConstraint {e}"))?;
    let path_index = column_index(&variables, "path");
    let value_index = column_index(&variables, "value");

    let mut out: Vec<ValidationResult> = Vec::with_capacity(rows.len());
    for row in &rows {
        let result_path = path_index
            .and_then(|i| row.get(i))
            .and_then(Option::as_ref)
            .map(term_value_to_native);
        // SHACL-SPARQL result mapping (§5.3.1): sh:value is the solution's
        // ?value binding when present, otherwise the FOCUS NODE ($this).
        let value = value_index
            .and_then(|i| row.get(i))
            .and_then(Option::as_ref)
            .map(term_value_to_native)
            .or_else(|| Some(focus.clone()));
        out.push(ValidationResult {
            focus_node: focus.clone(),
            result_path,
            path_structure: None,
            value,
            source_constraint_component: component.clone(),
            source_shape: source_shape.clone(),
            severity: severity.clone(),
            message: message.cloned(),
            source_box_roles: vec![],
            path_box_roles: vec![],
            result_box_roles: vec![],
            attributions: vec![],
        });
    }
    Ok(out)
}

/// Evaluate a single SPARQL scalar expression against `dataset`, with `args`
/// pre-bound as query variables.
///
/// The expression is wrapped in `SELECT ((<sparql_expr>) AS ?result) WHERE {}`
/// so it is evaluated exactly once (the empty `WHERE` yields a single solution
/// row). Each `(var_name, term)` in `args` is pre-bound to the query variable
/// `var_name` via the SAME substitution mechanism [`eval_sparql_constraint`]
/// uses for `$this`, so the expression may reference `?var_name`.
///
/// Returns:
/// - `Ok(Some(term))` when the single row bound `?result` (the expression
///   produced a value);
/// - `Ok(None)` when `?result` is unbound or no row was produced (a SPARQL
///   error/undef result — the correct SHACL-AF "no value" signal);
/// - `Err(String)` on an engine error, a non-SELECT result, or the impossible
///   case of more than one solution row.
///
/// # Errors
///
/// Returns `Err(String)` if execution fails, the result is not a SELECT, or the
/// query somehow yields more than one row.
pub fn eval_scalar_expr(
    dataset: &Arc<RdfDataset>,
    sparql_expr: &str,
    args: &[(String, Term)],
) -> Result<Option<Term>, String> {
    eval_scalar_expr_view(&**dataset, sparql_expr, args)
}

/// Internal view-generic implementation of [`eval_scalar_expr`].
pub(crate) fn eval_scalar_expr_view<D: DatasetView + Sync>(
    dataset: &D,
    sparql_expr: &str,
    args: &[(String, Term)],
) -> Result<Option<Term>, String> {
    let select = format!("SELECT (({sparql_expr}) AS ?result) WHERE {{}}");
    let subs: Vec<(String, TermValue)> = args
        .iter()
        .map(|(name, term)| (name.clone(), term.to_term_value()))
        .collect();
    let (variables, rows) = run_select_generic_view(dataset, &select, &subs)
        .map_err(|e| format!("scalar expression {e}"))?;

    if rows.len() > 1 {
        return Err(format!(
            "scalar expression produced {} solution rows (expected exactly one)",
            rows.len()
        ));
    }
    let Some(row) = rows.first() else {
        // No row at all is a degenerate/undef result → no value.
        return Ok(None);
    };
    let result_index = column_index(&variables, "result");
    let value = result_index
        .and_then(|i| row.get(i))
        .and_then(Option::as_ref)
        .map(term_value_to_native);
    Ok(value)
}

/// Evaluate a SPARQL set aggregate (`"MIN"` / `"MAX"` / `"SUM"`) over an explicit
/// list of operand `values`, on the native engine.
///
/// This keeps *all* SHACL-AF aggregation on the single SPARQL path so numeric
/// type-promotion and ordering match the engine exactly — there is no parallel
/// Rust numeric fold. The operands are inlined into a one-column `VALUES` block:
///
/// ```sparql
/// SELECT (MIN(?v) AS ?result) WHERE { VALUES (?v) { (t0) (t1) ... } }
/// ```
///
/// Each `ti` is rendered through the workspace [`Term`] serializer
/// ([`Term`]'s `Display`, i.e. N-Triples term syntax — `<iri>`, `"lex"^^<dt>`,
/// `"lex"@lang`), which is valid inside a SPARQL `VALUES` block for IRIs and
/// literals. Blank nodes and quoted triples cannot appear in `VALUES` (and are
/// not comparable/numeric aggregation operands), so an operand of either kind is
/// a hard type error.
///
/// The empty operand set is special-cased *before* building the query (an empty
/// `VALUES` block is awkward and the algebra is unambiguous): `SUM` of nothing is
/// `0`^^`xsd:integer`; `MIN`/`MAX` of nothing is unbound (`Ok(None)`).
///
/// Returns `Ok(Some(term))` when the aggregate bound `?result`, `Ok(None)` when
/// it is unbound (e.g. `MIN`/`MAX` of an empty set), or `Err` on an engine error
/// or an un-renderable operand.
///
/// # Errors
///
/// Returns `Err(String)` if `agg` is not one of `"MIN"`/`"MAX"`/`"SUM"`, an
/// operand is a blank node or quoted triple, execution fails, the result is not a
/// SELECT, or the query yields more than one row.
pub fn eval_aggregate(
    dataset: &Arc<RdfDataset>,
    agg: &str,
    values: &[Term],
) -> Result<Option<Term>, String> {
    eval_aggregate_view(&**dataset, agg, values)
}

/// Internal view-generic implementation of [`eval_aggregate`].
pub(crate) fn eval_aggregate_view<D: DatasetView + Sync>(
    dataset: &D,
    agg: &str,
    values: &[Term],
) -> Result<Option<Term>, String> {
    if !matches!(agg, "MIN" | "MAX" | "SUM") {
        return Err(format!(
            "unsupported aggregate {agg} (expected MIN/MAX/SUM)"
        ));
    }

    // Empty operand set: special-case before building the query.
    if values.is_empty() {
        return Ok(match agg {
            "SUM" => Some(Term::Literal(Literal::new_typed_literal(
                "0",
                NamedNode::new_unchecked(xsd::INTEGER),
            ))),
            // MIN/MAX of an empty set is unbound.
            _ => None,
        });
    }

    // Inline each operand into the VALUES block via the workspace Term serializer.
    let mut rows = String::new();
    for term in values {
        match term {
            Term::NamedNode(_) | Term::Literal(_) => {
                // `Term`'s Display renders N-Triples term syntax, valid in VALUES.
                rows.push('(');
                rows.push_str(&term.to_string());
                rows.push_str(") ");
            }
            Term::BlankNode(_) | Term::Triple(_) => {
                return Err(format!(
                    "aggregate {agg} operand {term} cannot appear in a SPARQL VALUES block (not a comparable/numeric value)"
                ));
            }
        }
    }

    let select = format!("SELECT ({agg}(?v) AS ?result) WHERE {{ VALUES (?v) {{ {rows}}} }}");
    let (variables, result_rows) =
        run_select_generic_view(dataset, &select, &[]).map_err(|e| format!("aggregate {e}"))?;

    if result_rows.len() > 1 {
        return Err(format!(
            "aggregate {agg} produced {} solution rows (expected exactly one)",
            result_rows.len()
        ));
    }
    let Some(row) = result_rows.first() else {
        return Ok(None);
    };
    let result_index = column_index(&variables, "result");
    let value = result_index
        .and_then(|i| row.get(i))
        .and_then(Option::as_ref)
        .map(term_value_to_native);
    Ok(value)
}

/// Order an explicit list of operand `values` by SPARQL `ORDER BY` *value*
/// semantics, on the native engine.
///
/// This keeps SHACL-AF `sh:orderby` on the single SPARQL path so typed/numeric
/// ordering matches the engine exactly — e.g. `"2"^^xsd:integer` sorts BEFORE
/// `"10"^^xsd:integer` (value order), unlike a lexical `Term::to_string` sort.
/// The operands are inlined into a one-column `VALUES` block and ordered:
///
/// ```sparql
/// SELECT ?v WHERE { VALUES (?v) { (t0) (t1) ... } } ORDER BY ?v
/// ```
///
/// (`ORDER BY DESC(?v)` when `descending`). `ORDER BY` over `VALUES` returns one
/// row per input row in order, so DUPLICATES are PRESERVED (no `DISTINCT`).
///
/// Each `ti` is rendered through the workspace [`Term`] serializer (N-Triples
/// term syntax), exactly as [`eval_aggregate`]. Blank nodes and quoted triples
/// cannot appear in a `VALUES` block, so an operand of either kind is a hard
/// error. The empty operand set is `Ok(vec![])`.
///
/// # Errors
///
/// Returns `Err(String)` if an operand is a blank node or quoted triple,
/// execution fails, or the result is not a SELECT.
pub fn eval_order(
    dataset: &Arc<RdfDataset>,
    values: &[Term],
    descending: bool,
) -> Result<Vec<Term>, String> {
    eval_order_view(&**dataset, values, descending)
}

/// Internal view-generic implementation of [`eval_order`].
pub(crate) fn eval_order_view<D: DatasetView + Sync>(
    dataset: &D,
    values: &[Term],
    descending: bool,
) -> Result<Vec<Term>, String> {
    if values.is_empty() {
        return Ok(Vec::new());
    }

    // Inline each operand into the VALUES block via the workspace Term serializer.
    let mut rows = String::new();
    for term in values {
        match term {
            Term::NamedNode(_) | Term::Literal(_) => {
                rows.push('(');
                rows.push_str(&term.to_string());
                rows.push_str(") ");
            }
            Term::BlankNode(_) | Term::Triple(_) => {
                return Err(format!(
                    "order-by operand {term} cannot appear in a SPARQL VALUES block (not orderable in VALUES)"
                ));
            }
        }
    }

    let order = if descending { "DESC(?v)" } else { "?v" };
    let select = format!("SELECT ?v WHERE {{ VALUES (?v) {{ {rows}}} }} ORDER BY {order}");
    let (variables, result_rows) =
        run_select_generic_view(dataset, &select, &[]).map_err(|e| format!("order-by {e}"))?;

    let v_index = column_index(&variables, "v");
    let mut out: Vec<Term> = Vec::with_capacity(result_rows.len());
    for row in &result_rows {
        match v_index.and_then(|i| row.get(i)).and_then(Option::as_ref) {
            Some(value) => out.push(term_value_to_native(value)),
            None => {
                return Err("order-by query produced a solution row with no ?v binding".to_owned());
            }
        }
    }
    Ok(out)
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// A materialized SELECT result: the projected variable names and the rows of
/// optional term-value cells.
type SelectRows = (Vec<String>, Vec<Vec<Option<TermValue>>>);

thread_local! {
    /// A per-thread [`NativeSparqlEngine`] reused across SHACL-AF evaluations so its
    /// query plan cache memoizes each `sh:select`/`sh:SPARQLTarget` parse across the
    /// many focus-node calls of one validation (the oxigraph path kept a pre-parsed
    /// `PreparedSparqlQuery`; a fresh engine per call would re-parse every time — a
    /// per-focus blowup on the whole-ontology conformance shapes). Each focus
    /// worker reuses its own cache, keyed on `(base, query text)`.
    static SPARQL_ENGINE: NativeSparqlEngine = NativeSparqlEngine::new();

    /// The SHACL-AF function registry (`sh:SPARQLFunction`) in scope for the current
    /// validation, set by [`enter_function_scope`]. `run_select_generic` reads it to decide
    /// whether a call-position IRI can resolve to a user function. Kept alongside the
    /// engine (this module's established thread-local pattern). Whole-bundle
    /// validation installs one guard per focus chunk, so every rayon worker sees
    /// the same registry without shared mutation. Parallel FILTER workers inside
    /// the SPARQL engine do NOT read this — they receive the registry through
    /// `EvalCtx` (propagated in `fork_for_worker`).
    static CURRENT_FUNCTIONS: RefCell<Option<Arc<UserFunctionRegistry>>> = const { RefCell::new(None) };

    /// The property-function registry in scope for the current validation, set by
    /// [`enter_property_function_scope`]. [`run_query_view`] snapshots it into the
    /// query options so a `sh:select`/`sh:ask` body whose predicate IRI sits under a
    /// configured property-function namespace resolves to a registered relation.
    /// The exact twin of [`CURRENT_FUNCTIONS`], for the exact same reasons: one guard
    /// per focus chunk, so every rayon worker sees the same registry without shared
    /// mutation, and the evaluator's own parallel workers receive it through `EvalCtx`
    /// rather than by reading this thread-local.
    static CURRENT_PROPERTY_FUNCTIONS: RefCell<Option<Arc<PropertyFunctionRegistry>>> = const { RefCell::new(None) };

    /// The custom-aggregate registry in scope for the current validation, set by
    /// [`enter_aggregate_scope`]. [`run_query_view`] snapshots it into the query
    /// options so a `sh:select`/`sh:ask` body whose `GROUP BY` uses `AGG(<iri>, …)`
    /// resolves to a registered aggregate. The exact twin of
    /// [`CURRENT_PROPERTY_FUNCTIONS`], for the exact same reasons: one guard per
    /// focus chunk, so every rayon worker sees the same registry without shared
    /// mutation, and the evaluator's own parallel per-group workers receive it
    /// through `EvalCtx` (propagated in `fork_for_worker`) rather than by reading
    /// this thread-local.
    static CURRENT_AGGREGATES: RefCell<Option<Arc<AggregateRegistry>>> = const { RefCell::new(None) };

    /// The execution governors in force for the current validation, set by
    /// [`enter_governor_scope`]. Every SPARQL query this module runs charges the state
    /// found here, and a validation with nothing installed runs exactly as it always did
    /// — no counters, no atomics, no cost.
    ///
    /// **One state for the whole validation, not one per query.** SHACL runs one query
    /// per focus node, so a per-query budget would silently hand an N-focus validation N
    /// times the ceiling its caller set. Governed focus validation runs serially in source
    /// order so scheduling cannot change which query trips or what the evidence consumed;
    /// the `Arc` still lets nested evaluator workers share the one operation-owned state.
    static CURRENT_GOVERNORS: RefCell<Option<Arc<GovernorState>>> = const { RefCell::new(None) };
}

/// An RAII scope that installs `state` as the governor accounting for every SPARQL query
/// run on this thread for the duration of a validation, restoring the previous value on
/// drop (so nested validations compose). The twin of [`FunctionScope`].
#[must_use]
#[derive(Debug)]
pub struct GovernorScope {
    previous: Option<Arc<GovernorState>>,
    /// A thread-local restoration guard must be dropped on the thread where it
    /// was created; this marker makes that invariant compile-time enforced.
    _not_send: PhantomData<Rc<()>>,
}

impl Drop for GovernorScope {
    fn drop(&mut self) {
        let restore = self.previous.take();
        CURRENT_GOVERNORS.with(|slot| *slot.borrow_mut() = restore);
    }
}

/// Install `state` as the governor accounting for every SPARQL query this thread runs,
/// returning a guard that restores the previous state when dropped.
pub fn enter_governor_scope(state: Arc<GovernorState>) -> GovernorScope {
    let previous = CURRENT_GOVERNORS.with(|slot| slot.borrow_mut().replace(state));
    GovernorScope {
        previous,
        _not_send: PhantomData,
    }
}

/// The governor accounting installed on this thread, if a validation installed one.
///
/// Read on the orchestrating thread and handed to each focus worker's
/// [`enter_governor_scope`], because a thread-local is not visible from the workers a
/// thread forks.
#[must_use]
pub fn current_governors() -> Option<Arc<GovernorState>> {
    CURRENT_GOVERNORS.with(|slot| slot.borrow().clone())
}

/// Run one SHACL-driven SPARQL query against `dataset`, under this validation's governors
/// when one is installed.
///
/// The single place any SHACL path reaches the SPARQL engine. Collapsing the four callers
/// here is what makes governor inheritance a property of the *module* rather than of four
/// separately-remembered call sites: a new SHACL query path cannot be added that quietly
/// runs ungoverned, because there is nowhere else for it to run.
///
/// # A tripped governor is an error *here*
///
/// The evaluator is careful to report a trip as an outcome rather than a failure, because
/// a partial answer is useful to a caller who asked for one. A conformance verdict is not
/// such a caller. "This focus node has no violating solution" computed over a truncated
/// bag is indistinguishable from the same sentence computed over the whole bag, and
/// folding it into a report would produce a `conforms` that means nothing. So the trip
/// becomes an `Err` on the spot; the governed validation entry recovers the *typed* trip
/// from the shared state, which latched it, and reports that instead of a report.
fn run_query_view<D: DatasetView + Sync>(
    dataset: &D,
    query: &str,
    substitutions: &[(String, TermValue)],
    prebind: ShaclPrebinding,
    bnode_mint_prefix: Option<&str>,
) -> Result<SparqlResult, String> {
    // Snapshot both ambient scopes (an `Arc` clone each) BEFORE evaluating, so no
    // `RefCell` borrow is held across the query: a `sh:sparql` body whose evaluation
    // re-enters SHACL validation (a nested shape / SHACL-AF function) installs its own
    // scopes via `borrow_mut`, which would panic ("already borrowed") if an outer
    // immutable borrow were still live.
    let functions = CURRENT_FUNCTIONS.with(|slot| slot.borrow().clone());
    let relations = current_property_functions();
    let aggregates = current_aggregates();
    let governors = current_governors();
    // The ambient thread-local scope is genuinely optional (no `sh:sparql` body has
    // ever installed one); an absent scope and the canonical `EMPTY` registry are the
    // SAME value for every purpose `QueryOptions` cares about (see
    // `AggregateRegistry::EMPTY`'s docs), so there is no longer a separate
    // "empty but present" case to normalize away here — the old `.filter(|reg|
    // !reg.is_empty())` step this replaced existed only to collapse that redundant
    // second spelling, which the non-optional field no longer has.
    //
    // `static`, not a bare `&Registry::EMPTY` temporary: a `HashMap`-backed registry
    // carries drop glue, which blocks Rust's rvalue static promotion for a reference
    // that must outlive this one statement (it is read again below, once per branch),
    // so it needs a genuine `'static` place to borrow from.
    static EMPTY_FUNCTIONS: UserFunctionRegistry = UserFunctionRegistry::EMPTY;
    static EMPTY_RELATIONS: PropertyFunctionRegistry = PropertyFunctionRegistry::EMPTY;
    static EMPTY_AGGREGATES: AggregateRegistry = AggregateRegistry::EMPTY;
    let registry = functions.as_deref().unwrap_or(&EMPTY_FUNCTIONS);
    let property_functions = relations.as_deref().unwrap_or(&EMPTY_RELATIONS);
    let agg_registry = aggregates.as_deref().unwrap_or(&EMPTY_AGGREGATES);
    let request = SparqlRequest {
        query,
        base_iri: None,
        substitutions,
    };
    let options = QueryOptions {
        prebinding: prebind,
        functions: registry,
        property_functions,
        aggregates: agg_registry,
        bnode_mint_prefix,
    };

    let Some(state) = governors else {
        return SPARQL_ENGINE
            .with(|engine| engine.query_with_options_view(dataset, request, options))
            .map_err(|e| format!("query evaluation error: {e}"));
    };

    let outcome = SPARQL_ENGINE
        .with(|engine| engine.query_governed_in_operation(dataset, request, options, &state))
        .map_err(|e| format!("query evaluation error: {e}"))?;
    match outcome {
        GovernedOutcome::Complete { result, .. } => Ok(result),
        GovernedOutcome::BudgetExhausted(exhausted) => Err(format!(
            "validation budget exhausted: {}; a conformance verdict cannot be computed \
             from a truncated solution bag",
            exhausted.tripped
        )),
    }
}

/// An RAII scope that installs `registry` as the current SHACL-AF function table for
/// the duration of a validation, restoring the previous value on drop (so nested
/// validations compose). The engine holds this for the whole validation pass.
#[must_use]
#[derive(Debug)]
pub struct FunctionScope {
    previous: Option<Arc<UserFunctionRegistry>>,
    /// A thread-local restoration guard must be dropped on the thread where it
    /// was created; this marker makes that invariant compile-time enforced.
    _not_send: PhantomData<Rc<()>>,
}

impl Drop for FunctionScope {
    fn drop(&mut self) {
        let restore = self.previous.take();
        CURRENT_FUNCTIONS.with(|slot| *slot.borrow_mut() = restore);
    }
}

/// Install `registry` as the current SHACL-AF function table, returning a guard that
/// restores the previous table when dropped.
pub fn enter_function_scope(registry: Arc<UserFunctionRegistry>) -> FunctionScope {
    let previous = CURRENT_FUNCTIONS.with(|slot| slot.borrow_mut().replace(registry));
    FunctionScope {
        previous,
        _not_send: PhantomData,
    }
}

/// An RAII scope that installs `registry` as the current property-function table for
/// the duration of a validation, restoring the previous value on drop (so nested
/// validations compose). The exact twin of [`FunctionScope`].
#[must_use]
#[derive(Debug)]
pub struct PropertyFunctionScope {
    previous: Option<Arc<PropertyFunctionRegistry>>,
    /// A thread-local restoration guard must be dropped on the thread where it
    /// was created; this marker makes that invariant compile-time enforced.
    _not_send: PhantomData<Rc<()>>,
}

impl Drop for PropertyFunctionScope {
    fn drop(&mut self) {
        let restore = self.previous.take();
        CURRENT_PROPERTY_FUNCTIONS.with(|slot| *slot.borrow_mut() = restore);
    }
}

/// Install `registry` as the current property-function table, returning a guard that
/// restores the previous table when dropped.
pub fn enter_property_function_scope(
    registry: Arc<PropertyFunctionRegistry>,
) -> PropertyFunctionScope {
    let previous = CURRENT_PROPERTY_FUNCTIONS.with(|slot| slot.borrow_mut().replace(registry));
    PropertyFunctionScope {
        previous,
        _not_send: PhantomData,
    }
}

/// The property-function table installed on this thread, if a validation installed
/// one.
///
/// Read on the orchestrating thread and handed to each focus worker's
/// [`enter_property_function_scope`], because a thread-local is not visible from the
/// threads a worker pool forks — the same reason [`current_governors`] exists.
#[must_use]
pub fn current_property_functions() -> Option<Arc<PropertyFunctionRegistry>> {
    CURRENT_PROPERTY_FUNCTIONS.with(|slot| slot.borrow().clone())
}

/// An RAII scope that installs `registry` as the current custom-aggregate table for
/// the duration of a validation, restoring the previous value on drop (so nested
/// validations compose). The exact twin of [`PropertyFunctionScope`].
#[must_use]
#[derive(Debug)]
pub struct AggregateScope {
    previous: Option<Arc<AggregateRegistry>>,
    /// A thread-local restoration guard must be dropped on the thread where it
    /// was created; this marker makes that invariant compile-time enforced.
    _not_send: PhantomData<Rc<()>>,
}

impl Drop for AggregateScope {
    fn drop(&mut self) {
        let restore = self.previous.take();
        CURRENT_AGGREGATES.with(|slot| *slot.borrow_mut() = restore);
    }
}

/// Install `registry` as the current custom-aggregate table, returning a guard that
/// restores the previous table when dropped.
pub fn enter_aggregate_scope(registry: Arc<AggregateRegistry>) -> AggregateScope {
    let previous = CURRENT_AGGREGATES.with(|slot| slot.borrow_mut().replace(registry));
    AggregateScope {
        previous,
        _not_send: PhantomData,
    }
}

/// The custom-aggregate table installed on this thread, if a validation installed
/// one.
///
/// Read on the orchestrating thread and handed to each focus worker's
/// [`enter_aggregate_scope`], because a thread-local is not visible from the
/// threads a worker pool forks — the same reason [`current_governors`] exists.
#[must_use]
pub fn current_aggregates() -> Option<Arc<AggregateRegistry>> {
    CURRENT_AGGREGATES.with(|slot| slot.borrow().clone())
}

/// Run a SELECT query over the dataset using the generic SPARQL `query` path
/// with variable substitutions.
///
/// This is the path used by SHACL-AF node expressions (scalar, aggregate,
/// order-by). It does NOT apply the SHACL-specific pre-binding rewrite used for
/// `sh:sparql` constraint/component bodies.
pub(crate) fn run_select_generic_view<D: DatasetView + Sync>(
    dataset: &D,
    select: &str,
    substitutions: &[(String, TermValue)],
) -> Result<SelectRows, String> {
    let result = run_query_view(dataset, select, substitutions, ShaclPrebinding::None, None)?;
    match result {
        SparqlResult::Solutions {
            variables, rows, ..
        } => Ok((variables, rows)),
        SparqlResult::Boolean(_) => {
            Err("query must be a SELECT, got a boolean (ASK) result".to_owned())
        }
        SparqlResult::Graph(_) => {
            Err("query must be a SELECT, got a graph (CONSTRUCT/DESCRIBE) result".to_owned())
        }
    }
}

/// Run a SELECT query over the dataset using SHACL-SPARQL pre-binding semantics.
///
/// Pre-binds `$this`, and when known `$shapesGraph` and `$currentShape`, then
/// applies the SHACL-specific substitution rewrite (FILTER/EXISTS expression
/// substitution and `BOUND($v)` → `true`).
pub(crate) fn run_select_with_shacl_prebinding_view<D: DatasetView + Sync>(
    dataset: &D,
    select: &str,
    substitutions: &[(String, TermValue)],
    shapes_graph_iri: Option<&str>,
    current_shape: Option<&Term>,
) -> Result<SelectRows, String> {
    let mut subs: Vec<(String, TermValue)> = substitutions.to_vec();
    if let Some(iri) = shapes_graph_iri {
        subs.push(("shapesGraph".to_owned(), TermValue::Iri(iri.to_owned())));
    }
    if let Some(shape) = current_shape {
        subs.push(("currentShape".to_owned(), shape.to_term_value()));
    }

    let result = run_query_view(dataset, select, &subs, ShaclPrebinding::Applied, None)?;
    match result {
        SparqlResult::Solutions {
            variables, rows, ..
        } => Ok((variables, rows)),
        SparqlResult::Boolean(_) => {
            Err("query must be a SELECT, got a boolean (ASK) result".to_owned())
        }
        SparqlResult::Graph(_) => {
            Err("query must be a SELECT, got a graph (CONSTRUCT/DESCRIBE) result".to_owned())
        }
    }
}

/// Run a CONSTRUCT query using SHACL-SPARQL pre-binding semantics, returning the
/// frozen graph of derived triples.
///
/// This is the SHACL-AF `sh:SPARQLRule` execution path: `$this` (and, when known,
/// `$shapesGraph` / `$currentShape`) are pre-bound, then the CONSTRUCT template is
/// instantiated over the WHERE solutions. CONSTRUCT already yields a frozen
/// `Arc<RdfDataset>`, so this is the sibling of
/// [`run_select_with_shacl_prebinding_view`] that returns the `Graph` arm.
///
/// `bnode_mint_prefix` is the deterministic blank-mint prefix installed on the
/// evaluation ([`purrdf_sparql_eval::QueryOptions::bnode_mint_prefix`]): the
/// rules engine passes a per-focus-node identity tag so every blank the CONSTRUCT
/// mints carries the focus's identity at mint time, while data blanks carried
/// through variables pass through untouched.
///
/// # Errors
///
/// Returns `Err(String)` if execution fails or if the result is not a CONSTRUCT
/// (`Solutions` / `Boolean` are rejected).
pub(crate) fn run_construct_with_shacl_prebinding_view<D: DatasetView + Sync>(
    dataset: &D,
    construct: &str,
    substitutions: &[(String, TermValue)],
    shapes_graph_iri: Option<&str>,
    current_shape: Option<&Term>,
    bnode_mint_prefix: Option<&str>,
) -> Result<Arc<RdfDataset>, String> {
    let mut subs: Vec<(String, TermValue)> = substitutions.to_vec();
    if let Some(iri) = shapes_graph_iri {
        subs.push(("shapesGraph".to_owned(), TermValue::Iri(iri.to_owned())));
    }
    if let Some(shape) = current_shape {
        subs.push(("currentShape".to_owned(), shape.to_term_value()));
    }

    let result = run_query_view(
        dataset,
        construct,
        &subs,
        ShaclPrebinding::Applied,
        bnode_mint_prefix,
    )?;
    match result {
        SparqlResult::Graph(graph) => Ok(graph),
        SparqlResult::Solutions { .. } => {
            Err("query must be a CONSTRUCT, got a SELECT result".to_owned())
        }
        SparqlResult::Boolean(_) => {
            Err("query must be a CONSTRUCT, got a boolean (ASK) result".to_owned())
        }
    }
}

/// Run an ASK query using SHACL-SPARQL pre-binding semantics.
pub(crate) fn run_ask_with_shacl_prebinding_view<D: DatasetView + Sync>(
    dataset: &D,
    ask: &str,
    substitutions: &[(String, TermValue)],
    shapes_graph_iri: Option<&str>,
    current_shape: Option<&Term>,
) -> Result<bool, String> {
    let mut subs: Vec<(String, TermValue)> = substitutions.to_vec();
    if let Some(iri) = shapes_graph_iri {
        subs.push(("shapesGraph".to_owned(), TermValue::Iri(iri.to_owned())));
    }
    if let Some(shape) = current_shape {
        subs.push(("currentShape".to_owned(), shape.to_term_value()));
    }

    let result = run_query_view(dataset, ask, &subs, ShaclPrebinding::Applied, None)?;
    match result {
        SparqlResult::Boolean(b) => Ok(b),
        SparqlResult::Solutions { .. } => {
            Err("query must be an ASK, got a SELECT result".to_owned())
        }
        SparqlResult::Graph(_) => {
            Err("query must be an ASK, got a graph (CONSTRUCT/DESCRIBE) result".to_owned())
        }
    }
}

/// The column index of variable `name` in a result header.
fn column_index(variables: &[String], name: &str) -> Option<usize> {
    variables.iter().position(|v| v == name)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ::purrdf::RdfDataset;

    use super::*;
    use crate::report::Severity;
    use crate::term::{NamedNode, Term};

    /// Build a tiny frozen dataset from a slice of N-Triples lines.
    fn dataset_from_ntriples(lines: &[&str]) -> Arc<RdfDataset> {
        let ntriples = lines.join("\n");
        if ntriples.is_empty() {
            return crate::text_ingest::parse_ntriples_to_dataset("").expect("empty dataset");
        }
        crate::text_ingest::parse_ntriples_to_dataset(&ntriples).expect("valid N-Triples")
    }

    fn named_term(iri: &str) -> Term {
        Term::NamedNode(NamedNode::new_unchecked(iri))
    }

    fn dummy_shape() -> Term {
        named_term("http://example.org/Shape")
    }

    fn dummy_component() -> NamedNode {
        NamedNode::new_unchecked("http://www.w3.org/ns/shacl#SPARQLConstraintComponent")
    }

    // ── eval_target ───────────────────────────────────────────────────────────

    #[test]
    fn eval_target_returns_foo_instances() {
        let dataset = dataset_from_ntriples(&[
            "<http://example.org/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Foo> .",
            "<http://example.org/b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Foo> .",
            "<http://example.org/c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Bar> .",
        ]);

        let select = "SELECT ?this WHERE { ?this <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Foo> }";
        let nodes = eval_target(&dataset, select, &[]).expect("eval_target must succeed");

        assert_eq!(nodes.len(), 2, "exactly two Foo instances");
        assert!(nodes.contains(&named_term("http://example.org/a")));
        assert!(nodes.contains(&named_term("http://example.org/b")));
        assert!(!nodes.contains(&named_term("http://example.org/c")));
        // Verify sorted order.
        let sorted = {
            let mut v = nodes.clone();
            crate::term::sort_terms_canonical(&mut v);
            v
        };
        assert_eq!(nodes, sorted, "result must be sorted");
    }

    #[test]
    fn eval_target_deduplicates() {
        let dataset = dataset_from_ntriples(&[]);
        let select =
            "SELECT ?this WHERE { VALUES ?this { <http://example.org/x> <http://example.org/x> } }";
        let nodes = eval_target(&dataset, select, &[]).expect("eval_target must succeed");
        assert_eq!(nodes.len(), 1, "duplicate binding must be deduped");
    }

    // ── eval_sparql_constraint ────────────────────────────────────────────────

    #[test]
    fn eval_sparql_constraint_self_reference() {
        let dataset = dataset_from_ntriples(&[
            "<http://example.org/self-node> <http://example.org/self> <http://example.org/self-node> .",
        ]);

        let select = "SELECT $this WHERE { $this <http://example.org/self> $this }";

        // Focus = the self-referencing node → one result.
        let focus_self = named_term("http://example.org/self-node");
        let results = eval_sparql_constraint(
            &dataset,
            &focus_self,
            select,
            &dummy_component(),
            &dummy_shape(),
            &Severity::Violation,
            None,
            None,
            None,
        )
        .expect("eval must succeed for self-referencing focus");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].focus_node, focus_self);
        assert_eq!(results[0].severity, Severity::Violation);
        assert_eq!(results[0].result_path, None);
        // SHACL-SPARQL §5.3.1: no ?value binding ⇒ sh:value defaults to the
        // focus node ($this).
        assert_eq!(results[0].value, Some(focus_self));

        // Focus = an unrelated node → zero results.
        let focus_other = named_term("http://example.org/other");
        let results_other = eval_sparql_constraint(
            &dataset,
            &focus_other,
            select,
            &dummy_component(),
            &dummy_shape(),
            &Severity::Violation,
            None,
            None,
            None,
        )
        .expect("eval must succeed for non-matching focus");
        assert_eq!(results_other.len(), 0);
    }

    // ── eval_scalar_expr ──────────────────────────────────────────────────────

    #[test]
    fn eval_scalar_expr_strlen() {
        let dataset = dataset_from_ntriples(&[]);
        let result = eval_scalar_expr(&dataset, "STRLEN(\"abc\")", &[])
            .expect("scalar eval must succeed")
            .expect("bound result");
        assert_eq!(
            result,
            Term::Literal(Literal::new_typed_literal(
                "3",
                NamedNode::new_unchecked("http://www.w3.org/2001/XMLSchema#integer"),
            ))
        );
    }

    #[test]
    fn eval_scalar_expr_with_arg_substitution() {
        let dataset = dataset_from_ntriples(&[]);
        let arg = Term::Literal(Literal::new_typed_literal(
            "abcd",
            NamedNode::new_unchecked("http://www.w3.org/2001/XMLSchema#string"),
        ));
        let result = eval_scalar_expr(&dataset, "STRLEN(?a0)", &[("a0".to_owned(), arg)])
            .expect("scalar eval must succeed")
            .expect("bound result");
        assert_eq!(
            result,
            Term::Literal(Literal::new_typed_literal(
                "4",
                NamedNode::new_unchecked("http://www.w3.org/2001/XMLSchema#integer"),
            ))
        );
    }

    // ── eval_aggregate ────────────────────────────────────────────────────────

    fn int_lit(n: &str) -> Term {
        Term::Literal(Literal::new_typed_literal(
            n,
            NamedNode::new_unchecked("http://www.w3.org/2001/XMLSchema#integer"),
        ))
    }

    #[test]
    fn eval_aggregate_min_max_sum_over_integers() {
        let dataset = dataset_from_ntriples(&[]);
        let vals = [int_lit("1"), int_lit("2"), int_lit("3")];
        assert_eq!(
            eval_aggregate(&dataset, "MIN", &vals).expect("min"),
            Some(int_lit("1"))
        );
        assert_eq!(
            eval_aggregate(&dataset, "MAX", &vals).expect("max"),
            Some(int_lit("3"))
        );
        assert_eq!(
            eval_aggregate(&dataset, "SUM", &vals).expect("sum"),
            Some(int_lit("6"))
        );
    }

    #[test]
    fn eval_aggregate_empty_set() {
        let dataset = dataset_from_ntriples(&[]);
        // SUM of empty = xsd:integer 0; MIN/MAX of empty = unbound.
        assert_eq!(
            eval_aggregate(&dataset, "SUM", &[]).expect("sum empty"),
            Some(int_lit("0"))
        );
        assert_eq!(
            eval_aggregate(&dataset, "MIN", &[]).expect("min empty"),
            None
        );
        assert_eq!(
            eval_aggregate(&dataset, "MAX", &[]).expect("max empty"),
            None
        );
    }

    #[test]
    fn eval_aggregate_promotes_int_and_decimal() {
        let dataset = dataset_from_ntriples(&[]);
        let decimal = Term::Literal(Literal::new_typed_literal(
            "2.5",
            NamedNode::new_unchecked("http://www.w3.org/2001/XMLSchema#decimal"),
        ));
        let vals = [int_lit("1"), decimal];
        // 1 (int) + 2.5 (decimal) promotes to xsd:decimal 3.5.
        assert_eq!(
            eval_aggregate(&dataset, "SUM", &vals).expect("sum"),
            Some(Term::Literal(Literal::new_typed_literal(
                "3.5",
                NamedNode::new_unchecked("http://www.w3.org/2001/XMLSchema#decimal"),
            )))
        );
    }

    #[test]
    fn eval_aggregate_blank_node_operand_is_error() {
        let dataset = dataset_from_ntriples(&[]);
        let err = eval_aggregate(&dataset, "SUM", &[Term::blank("b0")]).unwrap_err();
        assert!(
            err.contains("cannot appear in a SPARQL VALUES block"),
            "got: {err}"
        );
    }

    #[test]
    fn eval_aggregate_rejects_unknown_agg() {
        let dataset = dataset_from_ntriples(&[]);
        let err = eval_aggregate(&dataset, "AVG", &[int_lit("1")]).unwrap_err();
        assert!(err.contains("unsupported aggregate"), "got: {err}");
    }

    // ── eval_order ────────────────────────────────────────────────────────────

    #[test]
    fn eval_order_integers_by_value_not_lexical() {
        let dataset = dataset_from_ntriples(&[]);
        // Lexically "10" < "2", but by VALUE 2 < 10 — the engine must order by value.
        let vals = [int_lit("2"), int_lit("10")];
        assert_eq!(
            eval_order(&dataset, &vals, false).expect("asc order"),
            vec![int_lit("2"), int_lit("10")]
        );
        assert_eq!(
            eval_order(&dataset, &vals, true).expect("desc order"),
            vec![int_lit("10"), int_lit("2")]
        );
    }

    #[test]
    fn eval_order_preserves_duplicates() {
        let dataset = dataset_from_ntriples(&[]);
        let vals = [int_lit("2"), int_lit("2"), int_lit("10")];
        assert_eq!(
            eval_order(&dataset, &vals, false).expect("asc order"),
            vec![int_lit("2"), int_lit("2"), int_lit("10")],
            "ORDER BY over VALUES preserves one row per input (no DISTINCT)"
        );
    }

    #[test]
    fn eval_order_empty_is_empty() {
        let dataset = dataset_from_ntriples(&[]);
        assert!(
            eval_order(&dataset, &[], false)
                .expect("empty order")
                .is_empty()
        );
    }

    #[test]
    fn eval_order_blank_node_operand_is_error() {
        let dataset = dataset_from_ntriples(&[]);
        let err = eval_order(&dataset, &[Term::blank("b0")], false).unwrap_err();
        assert!(
            err.contains("cannot appear in a SPARQL VALUES block"),
            "got: {err}"
        );
    }

    #[test]
    fn eval_scalar_expr_type_error_is_none() {
        let dataset = dataset_from_ntriples(&[]);
        // STRLEN of an integer is a type error → the projection expression is
        // an error → ?result is unbound → Ok(None).
        let result = eval_scalar_expr(&dataset, "STRLEN(1 + 2)", &[])
            .expect("scalar eval must succeed despite the SPARQL type error");
        assert_eq!(result, None);
    }

    // ── property functions in a SHACL body ────────────────────────────────────

    /// A relation that answers "is this value on the deny list?": invoked with its
    /// subject bound (mode `bf`), it emits one row naming the reason when the value is
    /// the banned literal, and nothing otherwise.
    ///
    /// Two things about it are load-bearing for the test below. It declares ONLY `bf`,
    /// so the query is evaluable at all only if the engine invokes it with its argument
    /// bound; and its argument is a LITERAL, which the IRI-only outer-binding
    /// substitution could not have carried — so a passing test is evidence the dispatch
    /// reads the row itself.
    #[derive(Debug)]
    struct DenyListRelation {
        modes: [purrdf_sparql_eval::BindingPattern; 1],
    }

    impl DenyListRelation {
        fn new() -> Self {
            Self {
                modes: [purrdf_sparql_eval::BindingPattern::from_code("bf")],
            }
        }
    }

    impl purrdf_sparql_eval::PropertyFunction for DenyListRelation {
        fn volatility(&self) -> purrdf_sparql_eval::Volatility {
            purrdf_sparql_eval::Volatility::Stable
        }

        fn arity(&self) -> purrdf_sparql_eval::PfArity {
            purrdf_sparql_eval::PfArity::new(1, 1)
        }

        fn modes(&self) -> &[purrdf_sparql_eval::BindingPattern] {
            &self.modes
        }

        fn rows_per_invocation(&self, _mode: purrdf_sparql_eval::BindingPattern) -> u64 {
            1
        }

        fn open(
            &self,
            args: &purrdf_sparql_eval::PfArgs<'_>,
            _ceiling: Option<u64>,
        ) -> Result<Box<dyn purrdf_sparql_eval::PfCursor>, purrdf_sparql_eval::EvalError> {
            let banned = matches!(
                args.get(0),
                Some(TermValue::Literal { lexical_form, .. }) if lexical_form == "banned"
            );
            let rows = if banned {
                let subject = args.get(0).cloned().expect("the bound subject");
                vec![vec![
                    subject,
                    TermValue::Literal {
                        lexical_form: "on the deny list".to_owned(),
                        datatype: "http://www.w3.org/2001/XMLSchema#string".to_owned(),
                        language: None,
                        direction: None,
                    },
                ]]
            } else {
                Vec::new()
            };
            Ok(Box::new(DenyListCursor { rows, next: 0 }))
        }
    }

    struct DenyListCursor {
        rows: Vec<purrdf_sparql_eval::PfRow>,
        next: usize,
    }

    impl purrdf_sparql_eval::PfCursor for DenyListCursor {
        fn next(
            &mut self,
        ) -> Result<Option<purrdf_sparql_eval::PfRow>, purrdf_sparql_eval::EvalError> {
            let row = self.rows.get(self.next).cloned();
            self.next += 1;
            Ok(row)
        }
    }

    /// A `sh:sparql` constraint whose body calls a property function actually INVOKES
    /// the relation: the violation fires for exactly the focus node whose value the
    /// relation reports, and not for the other one.
    ///
    /// The negative half is the point. A constraint whose body silently degraded — the
    /// predicate read as an ordinary data triple that matches nothing — would report
    /// zero violations and be indistinguishable from a conforming graph, which is the
    /// always-passing failure mode this test exists to forbid.
    #[test]
    fn a_sparql_constraint_body_invokes_a_registered_property_function() {
        let shapes_ttl = r#"
            @prefix ex: <http://example.org/> .
            @prefix sh: <http://www.w3.org/ns/shacl#> .
            ex:Shape a sh:NodeShape ;
                sh:targetClass ex:Thing ;
                sh:sparql ex:Constraint .
            ex:Constraint sh:select """
                SELECT $this ?value
                WHERE {
                    $this <http://example.org/tag> ?value .
                    ?value <http://example.org/pf/denied> ?why .
                }
            """ .
        "#;
        let shapes_dataset =
            crate::text_ingest::parse_turtle_to_dataset(shapes_ttl, None).expect("valid shapes");
        let prefixes = crate::text_ingest::extract_prefixes(shapes_ttl);
        let shapes = crate::shapes::from_dataset_with_config(&shapes_dataset, &prefixes, None)
            .expect("parse shapes");

        let data = dataset_from_ntriples(&[
            "<http://example.org/n1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
             <http://example.org/Thing> .",
            "<http://example.org/n1> <http://example.org/tag> \"ok\" .",
            "<http://example.org/n2> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
             <http://example.org/Thing> .",
            "<http://example.org/n2> <http://example.org/tag> \"banned\" .",
        ]);

        let mut registry = PropertyFunctionRegistry::new();
        registry.register(
            "http://example.org/pf/denied",
            Arc::new(DenyListRelation::new()),
        );
        let _scope = enter_property_function_scope(Arc::new(registry));

        let report = crate::engine::validate_dataset(data.as_ref(), &shapes).expect("validate");
        assert!(
            !report.conforms,
            "the relation reports the banned value, so the constraint must fire"
        );
        assert_eq!(
            report.results.len(),
            1,
            "exactly the focus node whose value the relation named: {:?}",
            report.results
        );
        assert_eq!(
            report.results[0].focus_node,
            named_term("http://example.org/n2")
        );
    }

    /// The same shapes and data with an EMPTY registry in scope: the seam is off, the
    /// predicate is ordinary data, and the constraint body matches nothing.
    ///
    /// The pair of tests is what makes each one mean something. This one shows the
    /// graph is NOT intrinsically violating, so the violation the test above reports can
    /// only have come from the relation actually running.
    #[test]
    fn an_empty_registry_leaves_the_constraint_body_s_predicate_as_ordinary_data() {
        let shapes_ttl = r#"
            @prefix ex: <http://example.org/> .
            @prefix sh: <http://www.w3.org/ns/shacl#> .
            ex:Shape a sh:NodeShape ;
                sh:targetClass ex:Thing ;
                sh:sparql ex:Constraint .
            ex:Constraint sh:select """
                SELECT $this ?value
                WHERE {
                    $this <http://example.org/tag> ?value .
                    ?value <http://example.org/pf/denied> ?why .
                }
            """ .
        "#;
        let shapes_dataset =
            crate::text_ingest::parse_turtle_to_dataset(shapes_ttl, None).expect("valid shapes");
        let prefixes = crate::text_ingest::extract_prefixes(shapes_ttl);
        let shapes = crate::shapes::from_dataset_with_config(&shapes_dataset, &prefixes, None)
            .expect("parse shapes");
        let data = dataset_from_ntriples(&[
            "<http://example.org/n2> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
             <http://example.org/Thing> .",
            "<http://example.org/n2> <http://example.org/tag> \"banned\" .",
        ]);

        // With an EMPTY registry in scope the predicate is not a configured
        // property-function IRI at all, so the body reads it as data and matches
        // nothing — the seam is off, exactly as it is for every host that never
        // configured it.
        let _scope = enter_property_function_scope(Arc::new(PropertyFunctionRegistry::new()));
        let report = crate::engine::validate_dataset(data.as_ref(), &shapes).expect("validate");
        assert!(
            report.conforms,
            "with the seam off the predicate is ordinary data that matches nothing"
        );
    }

    #[test]
    fn eval_sparql_constraint_shapes_graph_and_current_shape() {
        let shapes_ttl = r#"
            @prefix ex: <http://example.org/> .
            @prefix sh: <http://www.w3.org/ns/shacl#> .
            ex:Shape a sh:NodeShape ;
                sh:targetNode ex:Node ;
                ex:prop 42 ;
                sh:sparql ex:Constraint .
            ex:Constraint sh:select """
                SELECT $this
                WHERE {
                    FILTER bound($shapesGraph)
                    GRAPH $shapesGraph {
                        FILTER bound($currentShape)
                        $currentShape ex:prop 42 .
                    }
                }
            """ .
        "#;
        let shapes_dataset =
            crate::text_ingest::parse_turtle_to_dataset(shapes_ttl, None).expect("valid shapes");
        let prefixes = crate::text_ingest::extract_prefixes(shapes_ttl);
        let shapes = crate::shapes::from_dataset_with_config_and_graph(
            &shapes_dataset,
            &prefixes,
            None,
            Some("http://example.org/shapes".to_owned()),
        )
        .expect("parse shapes");

        let data = dataset_from_ntriples(&[
            "<http://example.org/Node> <http://example.org/p> <http://example.org/o> .",
        ]);
        let report =
            crate::engine::validate_dataset_with_shapes_graph(data.as_ref(), &shapes, None)
                .expect("validate");
        assert!(!report.conforms, "constraint must fire");
        assert_eq!(report.results.len(), 1);
        assert_eq!(
            report.results[0].focus_node,
            named_term("http://example.org/Node")
        );
    }
}
