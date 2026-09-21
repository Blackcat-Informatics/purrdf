// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Pure SPARQL evaluation helpers for SHACL-AF, over the native engine.
//!
//! [`eval_target`] runs a `sh:SPARQLTarget` SELECT query and returns the bound
//! `?this` focus nodes. [`eval_sparql_constraint`] runs a `sh:SPARQLConstraint`
//! SELECT query with `$this`/`?this` pre-bound to the focus node and maps each
//! solution row to a [`ValidationResult`].
//!
//! Both run the [`NativeSparqlEngine`] over the borrowed `Arc<RdfDataset>` — there is
//! no oxigraph SPARQL engine and no materialized `Store`. Focus-node substitution
//! uses [`Prebinding`] (the native replacement for oxigraph's
//! `PreparedSparqlQuery::substitute_variable`,  GAP-A) — the borrowed-name
//! pre-binding list the evaluator's interned entry points take, so a validation
//! does not re-allocate the shape's variable names once per focus node.

use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;

use ::purrdf::TermValue;
use ::purrdf::{DatasetView, FastMap, RdfDataset};
use purrdf_sparql_eval::{
    AggregateRegistry, GovernorState, InternedGoverned, InternedOutcome, InternedRequest,
    InternedSolutions, NativeSparqlEngine, Prebinding, PreparedExecution, PropertyFunctionRegistry,
    QueryOptions, ShaclPrebinding, UserFunctionRegistry, ValueAggregate, fold_values, order_values,
};

use crate::report::{Severity, ValidationResult};
use crate::term::{NamedNode, Term, term_value_to_native};

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
    eval_target_view(dataset, select, substitutions)
}

/// Internal view-generic implementation of [`eval_target`].
pub(crate) fn eval_target_view<D: DatasetView + Sync + FocusGraphSource>(
    dataset: &D,
    select: &str,
    substitutions: &[(String, Term)],
) -> Result<Vec<Term>, String> {
    // The NAME borrows: a target's variable names are text out of the shapes
    // graph, already allocated there, and identical on every evaluation.
    let subs: Vec<Prebinding<'_>> = substitutions
        .iter()
        .map(|(name, term)| Prebinding {
            variable: name.as_str(),
            value: term.to_term_value(),
        })
        .collect();
    let mut nodes = run_select_generic_view(dataset, select, &subs, |solutions| {
        let this_index = solutions.column("this");
        let mut nodes: Vec<Term> = Vec::with_capacity(solutions.len());
        for row in solutions.rows() {
            match this_index.and_then(|i| solutions.cell(row, i)) {
                Some(value) => nodes.push(term_value_to_native(&value)),
                // Unprefixed: the `SPARQLTarget ` prefix is added by the one
                // `map_err` below, so both the engine's errors and this one are
                // spelled the same way for a reader.
                None => {
                    return Err("query produced a solution row with no ?this binding".to_owned());
                }
            }
        }
        Ok(nodes)
    })
    .map_err(|e| format!("SPARQLTarget {e}"))?;

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
/// `component`, `source_shape` and `severity` are taken from the caller and are the
/// same for every row. `message` is taken from the caller but is rendered PER ROW:
/// SHACL-SPARQL §5.3.3 substitutes `{?var}`/`{$var}` templates from the solution's
/// own variable bindings, so two rows of the same constraint carry different text.
/// A placeholder naming a variable the solution does not bind is left verbatim.
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
        dataset,
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
pub(crate) fn eval_sparql_constraint_view<D: DatasetView + Sync + FocusGraphSource>(
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
    //
    // The pre-bindings are written straight into a PREPARED handle's slots rather
    // than into a per-focus-node list: the parameter names are shape text, constant
    // across every focus node, and the query text is constant too, so a `&str` run
    // would re-hash the whole query to probe the plan cache and re-intern the same
    // names on every focus node. The handle is this worker's, checked out and put
    // back around the run, because focus nodes are fanned across `rayon` workers and
    // no single handle can span the focus set.
    let parameters = this_and_shape_context_names(shapes_graph_iri, current_shape);
    let bind = |execution: &mut ShaclExecution| {
        execution.bind(0, focus.to_term_value())?;
        bind_shape_context(execution, 1, shapes_graph_iri, current_shape)?;
        Ok(())
    };
    let project = |solutions: &InternedSolutions<'_, '_, D>| {
        let path_index = solutions.column("path");
        let value_index = solutions.column("value");

        // Message templating (§5.3.3) needs the solution's own bindings, so the
        // buffer is built once and refilled per row — and only when there is a
        // message to render, so the overwhelmingly common message-less constraint
        // pays nothing.
        let mut template_bindings: Vec<(String, Term)> = if message.is_some() {
            Vec::with_capacity(solutions.variables().len() + 1)
        } else {
            Vec::new()
        };

        let mut out: Vec<ValidationResult> = Vec::with_capacity(solutions.len());
        for row in solutions.rows() {
            let result_path = path_index
                .and_then(|i| solutions.cell(row, i))
                .as_ref()
                .map(term_value_to_native);
            // SHACL-SPARQL result mapping (§5.3.1): sh:value is the solution's
            // ?value binding when present, otherwise the FOCUS NODE ($this).
            let value = value_index
                .and_then(|i| solutions.cell(row, i))
                .as_ref()
                .map(term_value_to_native)
                .or_else(|| Some(focus.clone()));
            // §5.3.3: render the message against THIS solution. The row's own
            // bindings come first so a projected `?this` wins; `$this` is added only
            // as a fallback because it is pre-bound above whether or not it is
            // projected. Nothing else is synthesized — in particular `{$value}` is
            // NOT filled from the focus-node default above, because that default is
            // a rule about `sh:value`, not a variable binding the solution actually
            // carries.
            //
            // This is also the ONE reader of every column, and the reason the
            // interned egress converts per cell rather than per row: a constraint
            // with no `sh:message` never asks for a cell it does not report.
            let message = message.map(|m| {
                template_bindings.clear();
                for (index, var) in solutions.variables().iter().enumerate() {
                    if let Some(value) = solutions.cell(row, index) {
                        template_bindings
                            .push((var.as_str().to_owned(), term_value_to_native(&value)));
                    }
                }
                if !template_bindings.iter().any(|(n, _)| n == "this") {
                    template_bindings.push(("this".to_owned(), focus.clone()));
                }
                crate::components::substitute_message_templates(m, &template_bindings)
            });
            out.push(ValidationResult {
                focus_node: focus.clone(),
                result_path,
                path_structure: None,
                value,
                source_constraint_component: component.clone(),
                source_shape: source_shape.clone(),
                severity: severity.clone(),
                message,
                source_box_roles: vec![],
                path_box_roles: vec![],
                result_box_roles: vec![],
                attributions: vec![],
            });
        }
        Ok(out)
    };
    run_cached_select_with_shacl_prebinding_view(dataset, select, parameters, bind, project)
        .map_err(|e| format!("SPARQLConstraint {e}"))
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
    eval_scalar_expr_view(dataset, sparql_expr, args)
}

/// Internal view-generic implementation of [`eval_scalar_expr`].
///
/// Assembles the wrapper query text and delegates to [`eval_scalar_query_view`].
/// Every in-crate caller is on the plan path and calls that directly with text
/// assembled once; this spelling exists for [`eval_scalar_expr`]'s published
/// signature, which takes a bare expression.
pub(crate) fn eval_scalar_expr_view<D: DatasetView + Sync + FocusGraphSource>(
    dataset: &D,
    sparql_expr: &str,
    args: &[(String, Term)],
) -> Result<Option<Term>, String> {
    eval_scalar_query_view(dataset, &scalar_expr_query(sparql_expr), args)
}

/// Wrap a SPARQL scalar expression in the single-row SELECT that evaluates it.
///
/// The empty `WHERE` yields exactly one solution, so `?result` is bound to the
/// expression's single value (or left unbound on a SPARQL error). The text is a
/// function of the EXPRESSION alone, which is a constant of the node expression —
/// see [`eval_scalar_query_view`] for why that matters.
pub(crate) fn scalar_expr_query(sparql_expr: &str) -> String {
    format!("SELECT (({sparql_expr}) AS ?result) WHERE {{}}")
}

/// Evaluate a pre-assembled single-row scalar SELECT, with `args` pre-bound.
///
/// # Why the text is a parameter
///
/// The query text is a constant of the node expression: the callee IRI and the
/// argument arity are both fixed when the shapes graph is lowered. Assembling it
/// per CALL — which is what taking a bare expression here forced — put a `format!`
/// inside the cartesian-product loop over argument tuples in
/// [`crate::expression`], and made the engine's never-evicted plan cache hash a
/// freshly allocated key on every one of those calls. It is now assembled once, at
/// plan time, for the same reason the aggregate and order-by paths bypass query
/// text altogether.
pub(crate) fn eval_scalar_query_view<D: DatasetView + Sync + FocusGraphSource>(
    dataset: &D,
    select: &str,
    args: &[(String, Term)],
) -> Result<Option<Term>, String> {
    let subs: Vec<Prebinding<'_>> = args
        .iter()
        .map(|(name, term)| Prebinding {
            variable: name.as_str(),
            value: term.to_term_value(),
        })
        .collect();
    run_select_generic_view(dataset, select, &subs, project_scalar)
        .map_err(|e| format!("scalar expression {e}"))
}

/// The single `?result` binding a scalar expression's wrapper SELECT produces.
///
/// Shared by the `&str` scalar door and both prepared ones, so "no row at all is a
/// degenerate/undef result" and "more than one row is a hard error" are one decision
/// rather than three copies of one.
fn project_scalar<D: DatasetView + Sync>(
    solutions: &InternedSolutions<'_, '_, D>,
) -> Result<Option<Term>, String> {
    if solutions.len() > 1 {
        return Err(format!(
            "produced {} solution rows (expected exactly one)",
            solutions.len()
        ));
    }
    let Some(row) = solutions.rows().first() else {
        // No row at all is a degenerate/undef result → no value.
        return Ok(None);
    };
    Ok(solutions
        .column("result")
        .and_then(|i| solutions.cell(row, i))
        .as_ref()
        .map(term_value_to_native))
}

/// [`eval_scalar_query_view`] on this worker's cached prepared handle.
///
/// The door for a scalar probe reached once per focus node from a loop `rayon` fans
/// across workers, where no single handle can span the focus set.
///
/// # Errors
///
/// As [`eval_scalar_query_view`].
pub(crate) fn eval_cached_scalar_query_view<D: DatasetView + Sync + FocusGraphSource>(
    dataset: &D,
    select: &str,
    parameters: &[&str],
    bind: impl FnOnce(&mut ShaclExecution) -> Result<(), String>,
) -> Result<Option<Term>, String> {
    run_cached_select_generic_view(dataset, select, parameters, bind, project_scalar)
        .map_err(|e| format!("scalar expression {e}"))
}

/// Run a SHACL 1.2 SPARQL-based node expression (SPARQL Extensions §6.1
/// `sh:select` / §6.2 `sh:sparqlExpr`) and return the bindings of its single
/// projected `variable`.
///
/// `bindings` pre-binds the focus node (`this`) and every scope variable through
/// the SAME [`Prebinding`] mechanism [`eval_scalar_expr`] uses
/// for its arguments — the specification's "focusNode pre-bound to variable
/// `$this` and scope variables pre-bound with matching names".
///
/// A solution row that leaves `variable` unbound contributes no output node (an
/// unbound projection is an absence, which is what an empty output list means);
/// a row that binds it contributes exactly that node. Solution ORDER is
/// preserved and duplicates are kept: §6.1 makes the query's own `ORDER BY` /
/// `LIMIT` the author's instrument, and sorting the answer here would destroy the
/// very thing they wrote.
///
/// # Errors
///
/// Returns `Err(String)` if execution fails, the result is not a SELECT, or the
/// query's result header does not carry `variable` at all (a shapes-load check
/// already established the projection, so this can only mean the header and the
/// projection disagree).
pub(crate) fn eval_select_nodes_view<D: DatasetView + Sync + FocusGraphSource>(
    dataset: &D,
    select: &str,
    variable: &str,
    bindings: &[(String, Term)],
) -> Result<Vec<Term>, String> {
    let subs: Vec<Prebinding<'_>> = bindings
        .iter()
        .map(|(name, term)| Prebinding {
            variable: name.as_str(),
            value: term.to_term_value(),
        })
        .collect();
    run_select_generic_view(dataset, select, &subs, |solutions| {
        let index = solutions.column(variable).ok_or_else(|| {
            format!("SELECT result has no ?{variable} column, but that is the projected variable")
        })?;
        Ok(solutions
            .rows()
            .iter()
            .filter_map(|row| {
                solutions
                    .cell(row, index)
                    .as_ref()
                    .map(term_value_to_native)
            })
            .collect())
    })
}

/// Evaluate a SPARQL set aggregate (`"MIN"` / `"MAX"` / `"SUM"`) over an explicit
/// list of operand `values`.
///
/// This keeps *all* SHACL-AF aggregation on the evaluator's own accumulators, so
/// numeric type-promotion and ordering match a `GROUP BY` fold exactly — there is
/// no parallel Rust numeric fold. The operands go to
/// [`purrdf_sparql_eval::fold_values`] as VALUES, not as text.
///
/// # Why not a query
///
/// The operands used to be serialized to N-Triples and spliced into a one-column
/// `VALUES` block, which imposed two limits that belonged to the string bridge
/// rather than to the aggregate. It could not carry a blank node or an RDF 1.2
/// triple term, because `VALUES` cannot spell them — so `sh:min`/`sh:max` over
/// triple terms hard-errored even though the comparator ranks them perfectly
/// well. And because the spliced text embedded the operand DATA, every call
/// minted a distinct key in the never-evicted plan cache, making memory grow with
/// the input instead of with the program. Calling the fold directly removes both,
/// and removes an `O(k)` query parse per evaluation with them.
///
/// The empty operand set needs no special case: the accumulators define it.
/// `SUM` of nothing is `0`^^`xsd:integer`; `MIN`/`MAX` of nothing is unbound
/// (`Ok(None)`).
///
/// # Errors
///
/// Returns `Err(String)` if `agg` is not one of `"MIN"`/`"MAX"`/`"SUM"`, or if
/// the accumulator refuses an operand.
pub fn eval_aggregate(
    dataset: &Arc<RdfDataset>,
    agg: &str,
    values: &[Term],
) -> Result<Option<Term>, String> {
    eval_aggregate_view(dataset, agg, values)
}

/// Internal view-generic implementation of [`eval_aggregate`].
///
/// `dataset` is unused — the fold is a function of the operand VALUES alone —
/// but the parameter stays so every SHACL-AF evaluation helper keeps one shape
/// and a caller does not have to remember which of them reads the graph.
pub(crate) fn eval_aggregate_view<D: DatasetView + Sync + FocusGraphSource>(
    _dataset: &D,
    agg: &str,
    values: &[Term],
) -> Result<Option<Term>, String> {
    let aggregate = match agg {
        "MIN" => ValueAggregate::Min,
        "MAX" => ValueAggregate::Max,
        "SUM" => ValueAggregate::Sum,
        other => {
            return Err(format!(
                "unsupported aggregate {other} (expected MIN/MAX/SUM)"
            ));
        }
    };
    let operands: Vec<TermValue> = values.iter().map(Term::to_term_value).collect();
    let folded = fold_values(aggregate, &operands)
        .map_err(|e| format!("aggregate {agg} evaluation error: {e}"))?;
    Ok(folded.as_ref().map(term_value_to_native))
}

/// Order an explicit list of operand `values` by SPARQL `ORDER BY` *value*
/// semantics.
///
/// This keeps SHACL-AF `sh:orderby` on the evaluator's own comparator so
/// typed/numeric ordering matches a query's `ORDER BY` exactly — e.g.
/// `"2"^^xsd:integer` sorts BEFORE `"10"^^xsd:integer` (value order), unlike a
/// lexical `Term::to_string` sort — and so blank nodes and RDF 1.2 triple terms
/// order by the same total order every other term does (blank < IRI < literal <
/// triple, triples componentwise). The sort is stable, so DUPLICATES are
/// PRESERVED and equal-comparing values keep their input order.
///
/// See [`eval_aggregate`] for why the operands are handed to
/// [`purrdf_sparql_eval::order_values`] as values rather than spliced into a
/// `VALUES` block as text.
///
/// # Errors
///
/// Returns `Err(String)` never in the current implementation; the signature keeps
/// the fallible shape its callers already thread so a future comparator that can
/// refuse an operand does not become a breaking change.
pub fn eval_order(
    dataset: &Arc<RdfDataset>,
    values: &[Term],
    descending: bool,
) -> Result<Vec<Term>, String> {
    eval_order_view(dataset, values, descending)
}

/// Internal view-generic implementation of [`eval_order`].
///
/// `dataset` is unused, for the reason given on [`eval_aggregate_view`].
pub(crate) fn eval_order_view<D: DatasetView + Sync + FocusGraphSource>(
    _dataset: &D,
    values: &[Term],
    descending: bool,
) -> Result<Vec<Term>, String> {
    let operands: Vec<TermValue> = values.iter().map(Term::to_term_value).collect();
    Ok(order_values(operands, descending)
        .iter()
        .map(term_value_to_native)
        .collect())
}

// ── Internal helpers ──────────────────────────────────────────────────────────

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

    /// The custom node-expression function call depth in force on this thread — the
    /// counter [`crate::expression::RecursionGuard::enter_call`] charges.
    ///
    /// It exists because a recursion can LEAVE this evaluator and come back: a
    /// custom function's body may be a `sh:select` node expression whose query calls
    /// the same function again through the SPARQL registration of SHACL 1.2 SPARQL
    /// Extensions §7.3. The fresh evaluation context that query builds would restart
    /// its own counter at zero, so the cycle would never reach any ceiling. Publishing
    /// the depth here and seeding it into
    /// [`QueryOptions::call_depth`](purrdf_sparql_eval::QueryOptions::call_depth) is
    /// what makes the chain finite.
    ///
    /// A plain `Cell<u32>`, not a registry: it is a COUNTER, so it has no staleness
    /// dimension and nothing about a graph is captured in it.
    static CURRENT_CALL_DEPTH: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// A dataset a SHACL query runs over that can also lend the frozen graph behind it.
///
/// The one thing SHACL 1.2 SPARQL Extensions §7.3 needs from the query's dataset
/// that [`DatasetView`] cannot express: a `focusGraph` an expression-bodied function
/// body can be evaluated against. `DatasetView` carries an associated `Id` type, so
/// `&dyn DatasetView` is not a legal type and the generic view cannot be erased
/// behind a trait object — but every backend SHACL actually queries is a frozen
/// [`RdfDataset`] behind one wrapper or another, and THAT is the erased handle.
///
/// The graph travels with the QUERY, which is what keeps it fresh: the rules fixpoint
/// rebuilds its dataset each round and runs that round's queries over it, so the
/// focus graph a function sees is the graph the calling query is reading, never a
/// snapshot captured when the shapes were loaded.
pub(crate) trait FocusGraphSource {
    /// The frozen graph behind this view, when there is one.
    fn focus_graph(&self) -> Option<&Arc<RdfDataset>>;
}

impl FocusGraphSource for Arc<RdfDataset> {
    fn focus_graph(&self) -> Option<&Self> {
        Some(self)
    }
}

impl FocusGraphSource for RdfDataset {
    /// A bare `&RdfDataset` borrow cannot produce the `Arc` that owns it, so a
    /// caller holding one supplies no focus graph and an expression-bodied function
    /// call over it is refused rather than answered. Every SHACL path that can call
    /// one passes the `Arc` (or the class-membership view over it) instead.
    fn focus_graph(&self) -> Option<&Arc<Self>> {
        None
    }
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
///
/// # The door is an INTERNED one
///
/// `visit` is run inside the evaluation, on
/// [`purrdf_sparql_eval::InternedOutcome`] — the rows as the evaluator holds them,
/// over interned ids. It is not run on a [`purrdf_core::SparqlResult`], and that
/// is deliberate: building one copies every projected cell into an owned
/// `TermValue`, every row into a `Vec` and every variable name into a `String`,
/// then builds the auxiliary constructed-cell graph — and SHACL reads at most
/// three columns (`?this`, `?path`, `?value`) out of any of it, once per focus
/// node. Crossing that boundary per focus node threw away every interning gain the
/// evaluator had just made. An additive accessor on `SparqlResult` could not have
/// fixed it: `SparqlResult::Solutions` OWNS its rows, so the copy has already
/// happened by the time any accessor exists to be called.
///
/// `SparqlResult` is untouched and remains the egress for every generic
/// `SparqlEngine` consumer. SHACL is simply a different consumer, not a mode.
fn run_query_view<D: DatasetView + Sync + FocusGraphSource, R>(
    dataset: &D,
    query: &str,
    substitutions: &[Prebinding<'_>],
    prebind: ShaclPrebinding,
    bnode_mint_prefix: Option<&str>,
    visit: impl FnOnce(InternedOutcome<'_, '_, D>) -> Result<R, String>,
) -> Result<R, String> {
    let scopes = AmbientScopes::snapshot();
    let request = InternedRequest {
        query,
        base_iri: None,
        substitutions,
    };
    let options = scopes.options(dataset, prebind, bnode_mint_prefix);

    let Some(state) = scopes.governors.as_ref() else {
        return SPARQL_ENGINE
            .with(|engine| engine.query_interned_view(dataset, request, options, visit))
            .map_err(|e| format!("query evaluation error: {e}"))?;
    };

    let outcome = SPARQL_ENGINE
        .with(|engine| {
            engine.query_governed_interned_in_operation(dataset, request, options, state, visit)
        })
        .map_err(|e| format!("query evaluation error: {e}"))?;
    certify_governed(outcome)
}

/// Read a governed run's receipt and reduce it to the answer a conformance verdict
/// may be computed from, or to the reason it may not be.
///
/// Shared by the `&str` door ([`run_query_view`]) and the prepared one
/// ([`run_bound_view`]) rather than written out in each, because the two
/// conditions it refuses are exactly the two a second copy could quietly stop
/// checking. A `conforms` computed over a truncated bag, or over a relation that
/// declared its index was not whole, is not a verdict; both are refused here by
/// name.
fn certify_governed<R>(outcome: InternedGoverned<Result<R, String>>) -> Result<R, String> {
    match outcome {
        InternedGoverned::Complete {
            value, relations, ..
        } => {
            // The governed lane RECORDS a relation that declared its index was not
            // whole, instead of refusing it at the seam the way the ungoverned lane
            // does, because its receipt has a slot to report it on — and this is where
            // that receipt is read. A conformance verdict computed over an index that
            // was not whole is not a verdict, for exactly the reason the truncated arm
            // below is not one, so the incompleteness is refused here by name rather
            // than discarded along with the receipt it rode in on.
            let incomplete: Vec<String> = relations
                .witness
                .iter()
                .flat_map(|(iri, attested)| {
                    attested
                        .incompleteness
                        .iter()
                        .map(move |reason| format!("<{iri}>: {reason}"))
                })
                .collect();
            if !incomplete.is_empty() {
                return Err(format!(
                    "a relation this query invoked declares its index was not whole ({}); a \
                     conformance verdict cannot be computed over an index that was not whole",
                    incomplete.join("; ")
                ));
            }
            value
        }
        InternedGoverned::BudgetExhausted(exhausted) => Err(format!(
            "validation budget exhausted: {}; a conformance verdict cannot be computed \
             from a truncated solution bag",
            exhausted.tripped
        )),
    }
}

/// An absent [`UserFunctionRegistry`] scope, as a place a `'static` borrow can name.
///
/// `static`, not a bare `&Registry::EMPTY` temporary: a `HashMap`-backed registry
/// carries drop glue, which blocks Rust's rvalue static promotion for a reference
/// that must outlive the statement that spells it.
static EMPTY_FUNCTIONS: UserFunctionRegistry = UserFunctionRegistry::EMPTY;
/// An absent [`PropertyFunctionRegistry`] scope. See [`EMPTY_FUNCTIONS`].
static EMPTY_RELATIONS: PropertyFunctionRegistry = PropertyFunctionRegistry::EMPTY;
/// An absent [`AggregateRegistry`] scope. See [`EMPTY_FUNCTIONS`].
static EMPTY_AGGREGATES: AggregateRegistry = AggregateRegistry::EMPTY;

/// Every ambient thread-local scope a SHACL query runs under, read once.
///
/// The four scopes are snapshotted (an `Arc` clone each) BEFORE evaluating, so no
/// `RefCell` borrow is held across the query: a `sh:sparql` body whose evaluation
/// re-enters SHACL validation (a nested shape / SHACL-AF function) installs its own
/// scopes via `borrow_mut`, which would panic ("already borrowed") if an outer
/// immutable borrow were still live.
///
/// A struct rather than four locals because there are now TWO doors onto the engine
/// — the `&str` one and the prepared one — and the options they build must be the
/// same options. Reading the scopes in one place is what makes that a fact about the
/// type rather than a resemblance between two blocks.
struct AmbientScopes {
    /// The SHACL-AF function table in scope, if a validation installed one.
    functions: Option<Arc<UserFunctionRegistry>>,
    /// The property-function registry in scope, if a validation installed one.
    relations: Option<Arc<PropertyFunctionRegistry>>,
    /// The custom-aggregate registry in scope, if a validation installed one.
    aggregates: Option<Arc<AggregateRegistry>>,
    /// The operation budget in force, if a validation installed one.
    governors: Option<Arc<GovernorState>>,
    /// The custom-function call depth, so a recursion that reaches SPARQL and comes
    /// back keeps counting instead of restarting at zero.
    call_depth: u32,
}

impl AmbientScopes {
    /// Read every scope in force on this thread.
    fn snapshot() -> Self {
        Self {
            functions: CURRENT_FUNCTIONS.with(|slot| slot.borrow().clone()),
            relations: current_property_functions(),
            aggregates: current_aggregates(),
            governors: current_governors(),
            call_depth: current_call_depth(),
        }
    }

    /// The function registry this query resolves call-position IRIs against.
    ///
    /// The ambient thread-local scope is genuinely optional (no `sh:sparql` body has
    /// ever installed one); an absent scope and the canonical `EMPTY` registry are
    /// the SAME value for every purpose `QueryOptions` cares about (see
    /// `AggregateRegistry::EMPTY`'s docs), so there is no "empty but present" case to
    /// normalize away.
    fn functions(&self) -> &UserFunctionRegistry {
        self.functions.as_deref().unwrap_or(&EMPTY_FUNCTIONS)
    }

    /// The property-function registry this query's predicates resolve against.
    fn relations(&self) -> &PropertyFunctionRegistry {
        self.relations.as_deref().unwrap_or(&EMPTY_RELATIONS)
    }

    /// The custom-aggregate registry this query's `AGG(<iri>, …)` resolves against.
    fn aggregates(&self) -> &AggregateRegistry {
        self.aggregates.as_deref().unwrap_or(&EMPTY_AGGREGATES)
    }

    /// The [`QueryOptions`] a SHACL query over `dataset` runs under.
    fn options<'a, D: FocusGraphSource>(
        &'a self,
        dataset: &'a D,
        prebinding: ShaclPrebinding,
        bnode_mint_prefix: Option<&'a str>,
    ) -> QueryOptions<'a> {
        let functions = self.functions();
        QueryOptions {
            prebinding,
            functions,
            property_functions: self.relations(),
            aggregates: self.aggregates(),
            bnode_mint_prefix,
            // The graph THIS query is reading, handed to any expression-bodied
            // function it calls (SHACL 1.2 SPARQL Extensions §7.3). Per-query, so a
            // fixpoint round that rebuilt its dataset supplies the rebuilt one.
            focus_graph: functions
                .requires_focus_graph()
                .then(|| dataset.focus_graph())
                .flatten(),
            call_depth: self.call_depth,
        }
    }

    /// The two registries a prepared plan's admission depends on, held so a handle
    /// can tell whether it is still running under them.
    fn plan_configuration(&self) -> PlanConfiguration {
        PlanConfiguration {
            relations: self.relations.clone(),
            aggregates: self.aggregates.clone(),
        }
    }
}

/// The registries a plan was ADMITTED under, kept beside the handle that holds it.
///
/// A prepared plan is only valid under the property-function and custom-aggregate
/// registries it was prepared against: the registry is what decides which predicates
/// are relation calls and what a `Custom` aggregate IRI resolves to, so the evaluator
/// refuses — correctly — to run a plan under a registry that disagrees with the one
/// it was admitted under.
///
/// That refusal is the reason this exists. A handle cached per worker outlives the
/// validation that minted it, and the next validation on that worker may install
/// different registries; a cache keyed on query text alone would hand that validation
/// a plan the evaluator then refuses, turning a query that should work into a hard
/// error. So the configuration travels WITH the handle, and a handle whose
/// configuration no longer matches is re-prepared rather than run — the refusal is
/// avoided by making the claim true again, not by suppressing it.
///
/// Compared by `Arc` identity rather than by value: a registry's contents are not
/// cheaply comparable, and the `Arc` is held here, so the pointer cannot be an
/// address a freed allocation has since handed to something else. Identity is
/// conservative in the safe direction — two structurally equal registries at
/// different addresses re-prepare, which costs one preparation and answers
/// identically.
struct PlanConfiguration {
    /// The property-function registry in scope when the plan was admitted.
    relations: Option<Arc<PropertyFunctionRegistry>>,
    /// The custom-aggregate registry in scope when the plan was admitted.
    aggregates: Option<Arc<AggregateRegistry>>,
}

impl PlanConfiguration {
    /// Whether `scopes` still names the registries this plan was admitted under.
    fn still_current(&self, scopes: &AmbientScopes) -> bool {
        fn same<T>(a: Option<&Arc<T>>, b: Option<&Arc<T>>) -> bool {
            match (a, b) {
                (None, None) => true,
                (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                _ => false,
            }
        }
        same(self.relations.as_ref(), scopes.relations.as_ref())
            && same(self.aggregates.as_ref(), scopes.aggregates.as_ref())
    }
}

/// A SHACL query prepared once, with its parameters bound and re-bound per run.
///
/// The handle behind [`run_bound_view`]. A `sh:sparql` body, a component
/// validator and a `sh:SPARQLRule` CONSTRUCT all run the SAME query text once per
/// focus node (or per value node, or per argument tuple), changing only the terms
/// pre-bound into it — which is precisely the shape
/// [`purrdf_sparql_eval::PreparedExecution`] exists for. Reaching the engine through
/// `&str` re-probed its plan cache per run, hashing the whole query text to be handed
/// back the same plan every time; reaching it through a handle parses and admits once
/// and binds a cell thereafter.
///
/// **This does not remove the per-run rewrite.** The pre-binding substitution still
/// clones the admitted algebra and rewrites it on every execution, because the handle
/// caches the plan rather than the substituted plan. What it removes is the query-text
/// hash and cache probe per run, the re-interning of the parameter NAMES, and the
/// per-run pre-binding list the `&str` door has to build.
pub(crate) struct ShaclExecution {
    /// The prepared plan and its parameter slots.
    execution: PreparedExecution,
    /// The registries `execution`'s plan was admitted under.
    prepared_under: PlanConfiguration,
}

impl ShaclExecution {
    /// Bind the parameter in `slot` to `value`.
    ///
    /// # Errors
    ///
    /// `Err(String)` if `slot` is not a declared parameter — a caller mistake about
    /// this query's own shape, refused rather than ignored.
    pub(crate) fn bind(&mut self, slot: usize, value: TermValue) -> Result<(), String> {
        self.execution.bind(slot, value).map_err(|e| e.to_string())
    }

    /// Prepare `query` with `parameters` under the registries in `scopes`.
    fn prepare(query: &str, parameters: &[&str], scopes: &AmbientScopes) -> Result<Self, String> {
        debug_assert!(
            parameters_are_distinct(parameters),
            "a repeated parameter name has no single slot to bind and must fall back to the \
             `&str` door, which keeps a per-variable path for it"
        );
        let options = QueryOptions {
            prebinding: ShaclPrebinding::None,
            functions: scopes.functions(),
            property_functions: scopes.relations(),
            aggregates: scopes.aggregates(),
            bnode_mint_prefix: None,
            focus_graph: None,
            call_depth: 0,
        };
        let execution = SPARQL_ENGINE
            .with(|engine| engine.prepare_execution(query, None, parameters, options))
            .map_err(|e| format!("query evaluation error: {e}"))?;
        Ok(Self {
            execution,
            prepared_under: scopes.plan_configuration(),
        })
    }
}

/// Whether `parameters` names every variable at most once.
///
/// A repeated name is the one pre-binding shape a handle cannot express: it has no
/// single slot to bind, so `prepare_execution` refuses it. The `&str` door SUPPORTS
/// it and has a defined answer — two seeds binding the same variable to different
/// terms are incompatible, so it keeps a per-variable path and yields the empty
/// solution.
///
/// A custom component really can produce one. Its `sh:parameter` local names are
/// checked at shapes load against a ban list holding `this`, `path`, `PATH` and
/// `value`, so those four never arrive — but `shapesGraph` and `currentShape` are not
/// on it, and SHACL pre-binds both around every validator. A parameter named either
/// reaches a validator's pre-binding list twice.
///
/// Every prepared call site therefore asks this FIRST and routes a repeated name back
/// to the `&str` door unchanged, so the handle never turns a query with a defined
/// answer into an error.
pub(crate) fn parameters_are_distinct(parameters: &[&str]) -> bool {
    parameters
        .iter()
        .enumerate()
        .all(|(index, name)| !parameters[..index].contains(name))
}

/// Run an already-bound `handle` against `dataset`, under this validation's governors
/// when one is installed.
///
/// The prepared twin of [`run_query_view`], and the ONLY place a prepared SHACL query
/// reaches the engine. It exists beside `run_query_view` rather than inside it because
/// the two doors take their plan and their bindings from different places, and it
/// routes through the same [`AmbientScopes`] and the same [`certify_governed`] so the
/// governed receipt, the registry snapshot and the trip refusal are literally the same
/// code on both.
///
/// `handle` must already carry this run's bindings. Binding is the CALLER's step
/// rather than this one's because a call site that runs the same query several times
/// over mostly-identical bindings — a component validator over a focus node's value
/// nodes, say — binds its invariant slots once and rewrites only the varying one per
/// run, and a bind callback here would force it to re-materialize every term every
/// time. What protects that is [`with_cached_execution`], which clears every slot at
/// CHECKOUT: a slot no one has written since is `None`, and the engine refuses to run
/// with a parameter unbound rather than answering from whatever a previous focus node
/// left there.
///
/// # Errors
///
/// `Err(String)` if evaluation fails, if a parameter is still unbound, or if the
/// governed receipt says no conformance verdict may be computed from this run.
fn run_bound_view<D: DatasetView + Sync + FocusGraphSource, R>(
    dataset: &D,
    handle: &mut ShaclExecution,
    prebind: ShaclPrebinding,
    bnode_mint_prefix: Option<&str>,
    visit: impl FnOnce(InternedOutcome<'_, '_, D>) -> Result<R, String>,
) -> Result<R, String> {
    let scopes = AmbientScopes::snapshot();
    let options = scopes.options(dataset, prebind, bnode_mint_prefix);

    let Some(state) = scopes.governors.as_ref() else {
        return SPARQL_ENGINE
            .with(|engine| engine.execute(&mut handle.execution, dataset, options, visit))
            .map_err(|e| format!("query evaluation error: {e}"))?;
    };

    let outcome = SPARQL_ENGINE
        .with(|engine| {
            engine.execute_governed_in_operation(
                &mut handle.execution,
                dataset,
                options,
                state,
                visit,
            )
        })
        .map_err(|e| format!("query evaluation error: {e}"))?;
    certify_governed(outcome)
}

/// How many distinct query texts one worker keeps prepared.
///
/// The plan cache beside this one is keyed on query TEXT and never evicts, which is
/// correct for a fixed set of authored `sh:select` / `sh:ask` bodies and wrong for any
/// path that manufactures query text out of operand data. This cache is keyed the same
/// way and is reached from a path that DOES manufacture text — `$PATH` substitution
/// renders the shape's path into the query — so it is capped. Reaching the cap clears
/// rather than evicts, because the table is a memo of a pure preparation and losing it
/// costs re-preparation rather than a wrong answer.
const PREPARED_EXECUTION_CAP: usize = 1_024;

/// One cached handle: the parameter names it was prepared with, and the handle itself
/// when it is not checked out.
struct CachedExecution {
    /// The parameter names, in bind order. Owned, because the caller's names can be
    /// borrowed from text the caller itself minted.
    parameters: Box<[Box<str>]>,
    /// The handle, or `None` while a run holds it — see [`checkout_execution`].
    handle: Option<ShaclExecution>,
}

impl CachedExecution {
    /// Whether this entry was prepared with exactly `parameters`, in order.
    ///
    /// Compared against the BORROWED names, so a hit allocates nothing.
    fn declares(&self, parameters: &[&str]) -> bool {
        self.parameters.len() == parameters.len()
            && self
                .parameters
                .iter()
                .zip(parameters)
                .all(|(held, wanted)| &**held == *wanted)
    }
}

thread_local! {
    /// Prepared handles for the queries this worker runs, keyed by query text and
    /// parameter list.
    ///
    /// Per-worker for the same reason [`SPARQL_ENGINE`] is: SHACL fans focus nodes
    /// over `rayon`, a handle is `!Sync` by construction, and the engine a handle's
    /// plan belongs to is itself held per worker.
    ///
    /// Two levels rather than one composite key, because the key is a query text AND
    /// a parameter list and a hit must allocate nothing: `Box<str>: Borrow<str>` lets
    /// the outer map be probed by the borrowed text exactly as the evaluator's
    /// variable interner is, and the inner list is scanned with borrowed comparisons.
    /// A query text carries at most a handful of parameter shapes (the shape context
    /// is present or not), so the scan is over a vector of one or two.
    ///
    /// # The value is an `Option`, and that is the re-entrancy guarantee
    ///
    /// A `sh:sparql` body can re-enter SHACL validation, reach this module again, and
    /// probe this very map for the very query the outer call is mid-evaluation over.
    /// A cache that LENT its handle would hand the inner call a tree the outer call is
    /// still reading. So a run TAKES the handle out — the slot becomes `None` — and
    /// puts it back when it is done. An inner call finds `None`, reads it as a miss,
    /// and prepares its own handle; two handles for one query are simply two equal
    /// plans, and whichever is restored last is the one the next run finds.
    static PREPARED_EXECUTIONS: RefCell<FastMap<Box<str>, Vec<CachedExecution>>> =
        RefCell::new(FastMap::default());
}

/// Take this worker's handle for `(query, parameters)`, preparing one on a miss.
///
/// The `RefCell` borrow is released before preparation and before the caller runs:
/// nothing in this module may hold it across an evaluation.
fn checkout_execution(
    query: &str,
    parameters: &[&str],
    scopes: &AmbientScopes,
) -> Result<ShaclExecution, String> {
    let cached = PREPARED_EXECUTIONS.with(|cache| {
        cache.borrow_mut().get_mut(query).and_then(|entries| {
            entries
                .iter_mut()
                .find(|entry| entry.declares(parameters))
                .and_then(|entry| entry.handle.take())
        })
    });
    let Some(mut handle) = cached else {
        // A fresh preparation starts with every slot `None` already.
        return ShaclExecution::prepare(query, parameters, scopes);
    };
    if !handle.prepared_under.still_current(scopes) {
        return ShaclExecution::prepare(query, parameters, scopes);
    }
    // Every slot back to unbound, HERE, before the caller writes any of them. This is
    // what stops one focus node's term being answered for the next: a cached handle
    // still holds whatever the last checkout bound, and a caller that binds four slots
    // where the previous one bound five would otherwise run with a stale fifth. Clearing
    // at checkout makes that slot `None`, and the engine refuses an unbound parameter
    // rather than answering from it.
    handle.execution.unbind_all();
    Ok(handle)
}

/// Put `handle` back for the next run of `(query, parameters)`.
///
/// Called on BOTH the `Ok` and `Err` paths of a run: a handle dropped on the error
/// path would silently turn every later run of that query into a fresh preparation,
/// which is a performance defect that no test asserting an ANSWER could see.
///
/// Restoring into an existing slot allocates nothing; only a genuinely new
/// `(query, parameters)` pair owns its key.
fn restore_execution(query: &str, parameters: &[&str], handle: ShaclExecution) {
    PREPARED_EXECUTIONS.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(entries) = cache.get_mut(query) {
            if let Some(entry) = entries.iter_mut().find(|entry| entry.declares(parameters)) {
                entry.handle = Some(handle);
            } else {
                entries.push(CachedExecution {
                    parameters: own_parameters(parameters),
                    handle: Some(handle),
                });
            }
            return;
        }
        if cache.len() >= PREPARED_EXECUTION_CAP {
            cache.clear();
        }
        cache.insert(
            Box::from(query),
            vec![CachedExecution {
                parameters: own_parameters(parameters),
                handle: Some(handle),
            }],
        );
    });
}

/// The parameter names, owned, for a cache entry that is genuinely new.
fn own_parameters(parameters: &[&str]) -> Box<[Box<str>]> {
    parameters.iter().map(|name| Box::from(*name)).collect()
}

/// Check out this worker's handle for `(query, parameters)`, run `body` with it, and
/// put it back.
///
/// The entry for a call site that CANNOT hold a handle across its focus set — one
/// reached per focus node from a loop `rayon` fans across workers. `body` may run the
/// handle any number of times, which is what lets a validator that loops over a focus
/// node's value nodes bind its invariant slots ONCE and rewrite only the varying one
/// per run.
///
/// Every slot is unbound before `body` sees the handle; see [`checkout_execution`].
///
/// # Errors
///
/// `Err(String)` if preparation fails, or whatever `body` returns. The handle is
/// restored on BOTH paths: dropping it on the error path would silently turn every
/// later run of that query into a fresh preparation, a performance defect no test
/// asserting an ANSWER could see.
pub(crate) fn with_cached_execution<R>(
    query: &str,
    parameters: &[&str],
    body: impl FnOnce(&mut ShaclExecution) -> Result<R, String>,
) -> Result<R, String> {
    debug_assert!(
        parameters_are_distinct(parameters),
        "a repeated parameter name must fall back to the `&str` door"
    );
    let scopes = AmbientScopes::snapshot();
    let mut handle = checkout_execution(query, parameters, &scopes)?;
    let outcome = body(&mut handle);
    restore_execution(query, parameters, handle);
    outcome
}

/// [`run_bound_view`] once, over this worker's cached handle for
/// `(query, parameters)`.
///
/// The single-run shape of [`with_cached_execution`], for a call site that runs its
/// query exactly once per checkout: `bind` writes the slots, the run follows.
///
/// # Errors
///
/// As [`with_cached_execution`] and [`run_bound_view`].
#[allow(clippy::too_many_arguments)] // The prepared door needs its key, its options and its two callbacks.
fn run_cached_prepared_view<D: DatasetView + Sync + FocusGraphSource, R>(
    dataset: &D,
    query: &str,
    parameters: &[&str],
    prebind: ShaclPrebinding,
    bnode_mint_prefix: Option<&str>,
    bind: impl FnOnce(&mut ShaclExecution) -> Result<(), String>,
    visit: impl FnOnce(InternedOutcome<'_, '_, D>) -> Result<R, String>,
) -> Result<R, String> {
    with_cached_execution(query, parameters, |handle| {
        bind(handle)?;
        run_bound_view(dataset, handle, prebind, bnode_mint_prefix, visit)
    })
}

/// Project a SELECT outcome through `project`, refusing any other query form.
///
/// A free function rather than a closure written out at each door, so the three
/// refusal wordings stay one wording however many doors there are.
fn project_solutions<'a, 'd, D: DatasetView + Sync, R>(
    outcome: InternedOutcome<'a, 'd, D>,
    project: impl FnOnce(&InternedSolutions<'a, 'd, D>) -> Result<R, String>,
) -> Result<R, String> {
    match outcome {
        InternedOutcome::Solutions(solutions) => project(&solutions),
        InternedOutcome::Boolean(_) => {
            Err("query must be a SELECT, got a boolean (ASK) result".to_owned())
        }
        InternedOutcome::Graph(_) => {
            Err("query must be a SELECT, got a graph (CONSTRUCT/DESCRIBE) result".to_owned())
        }
    }
}

/// Take an ASK outcome's boolean, refusing any other query form. See
/// [`project_solutions`].
// By value because this is passed AS a `FnOnce(InternedOutcome<'_, '_, D>) -> R`, the
// shape every run entry's `visit` has; a reference would not satisfy that bound.
#[allow(clippy::needless_pass_by_value)]
fn project_boolean<D: DatasetView + Sync>(
    outcome: InternedOutcome<'_, '_, D>,
) -> Result<bool, String> {
    match outcome {
        InternedOutcome::Boolean(answer) => Ok(answer),
        InternedOutcome::Solutions(_) => {
            Err("query must be an ASK, got a SELECT result".to_owned())
        }
        InternedOutcome::Graph(_) => {
            Err("query must be an ASK, got a graph (CONSTRUCT/DESCRIBE) result".to_owned())
        }
    }
}

/// Take a CONSTRUCT outcome's frozen graph, refusing any other query form. See
/// [`project_solutions`].
// By value for the same reason as [`project_boolean`].
#[allow(clippy::needless_pass_by_value)]
fn project_graph<D: DatasetView + Sync>(
    outcome: InternedOutcome<'_, '_, D>,
) -> Result<Arc<RdfDataset>, String> {
    match outcome {
        // The graph is already frozen and shared by `Arc`; taking it out of the
        // evaluation is a handle clone, not a copy of the derived triples.
        InternedOutcome::Graph(graph) => Ok(Arc::clone(graph)),
        InternedOutcome::Solutions(_) => {
            Err("query must be a CONSTRUCT, got a SELECT result".to_owned())
        }
        InternedOutcome::Boolean(_) => {
            Err("query must be a CONSTRUCT, got a boolean (ASK) result".to_owned())
        }
    }
}

/// Run a SELECT under SHACL-SPARQL pre-binding on this worker's cached handle.
///
/// For a call site reached per focus node from a loop `rayon` fans across workers,
/// where no single handle can span the focus set.
///
/// # Errors
///
/// As [`run_cached_prepared_view`], plus a non-SELECT result.
pub(crate) fn run_cached_select_with_shacl_prebinding_view<
    D: DatasetView + Sync + FocusGraphSource,
    R,
>(
    dataset: &D,
    select: &str,
    parameters: &[&str],
    bind: impl FnOnce(&mut ShaclExecution) -> Result<(), String>,
    project: impl FnOnce(&InternedSolutions<'_, '_, D>) -> Result<R, String>,
) -> Result<R, String> {
    run_cached_prepared_view(
        dataset,
        select,
        parameters,
        ShaclPrebinding::Applied,
        None,
        bind,
        |outcome| project_solutions(outcome, project),
    )
}

/// Run a generic-substitution SELECT on this worker's cached handle.
///
/// The SHACL-AF node-expression path's prepared door: no SHACL pre-binding rewrite,
/// exactly as [`run_select_generic_view`].
///
/// # Errors
///
/// As [`run_cached_prepared_view`], plus a non-SELECT result.
pub(crate) fn run_cached_select_generic_view<D: DatasetView + Sync + FocusGraphSource, R>(
    dataset: &D,
    select: &str,
    parameters: &[&str],
    bind: impl FnOnce(&mut ShaclExecution) -> Result<(), String>,
    project: impl FnOnce(&InternedSolutions<'_, '_, D>) -> Result<R, String>,
) -> Result<R, String> {
    run_cached_prepared_view(
        dataset,
        select,
        parameters,
        ShaclPrebinding::None,
        None,
        bind,
        |outcome| project_solutions(outcome, project),
    )
}

/// Run an ASK under SHACL-SPARQL pre-binding on an already-bound handle.
///
/// The door inside a [`with_cached_execution`] body, for a validator that runs its ASK
/// once per value node over slots it bound before the loop.
///
/// # Errors
///
/// As [`run_bound_view`], plus a non-ASK result.
pub(crate) fn run_bound_ask_with_shacl_prebinding_view<D: DatasetView + Sync + FocusGraphSource>(
    dataset: &D,
    handle: &mut ShaclExecution,
) -> Result<bool, String> {
    run_bound_view(
        dataset,
        handle,
        ShaclPrebinding::Applied,
        None,
        project_boolean,
    )
}

/// Run a CONSTRUCT under SHACL-SPARQL pre-binding on an already-bound handle.
///
/// `bnode_mint_prefix` is per-RUN rather than per-handle: a `sh:SPARQLRule` mints its
/// template blanks under a prefix derived from the focus node, and that prefix is
/// evaluation configuration, not part of the plan. One handle therefore serves a whole
/// focus set whose members each mint under their own prefix.
///
/// # Errors
///
/// As [`run_bound_view`], plus a non-CONSTRUCT result.
pub(crate) fn run_bound_construct_with_shacl_prebinding_view<
    D: DatasetView + Sync + FocusGraphSource,
>(
    dataset: &D,
    handle: &mut ShaclExecution,
    bnode_mint_prefix: Option<&str>,
) -> Result<Arc<RdfDataset>, String> {
    run_bound_view(
        dataset,
        handle,
        ShaclPrebinding::Applied,
        bnode_mint_prefix,
        project_graph,
    )
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

/// An RAII scope that publishes the custom node-expression function call depth in
/// force on this thread, restoring the previous value on drop.
///
/// Installed around every custom-function body evaluation, so a `sh:select` body
/// inside that function starts its query at the depth the caller had reached rather
/// than at zero. See `CURRENT_CALL_DEPTH` for why the counter has to survive the
/// trip out of this evaluator and back.
#[must_use]
#[derive(Debug)]
pub struct CallDepthScope {
    previous: u32,
    /// A thread-local restoration guard must be dropped on the thread where it was
    /// created; this marker makes that invariant compile-time enforced.
    _not_send: PhantomData<Rc<()>>,
}

impl Drop for CallDepthScope {
    fn drop(&mut self) {
        let restore = self.previous;
        CURRENT_CALL_DEPTH.with(|slot| slot.set(restore));
    }
}

/// Publish `depth` as the custom-function call depth in force on this thread,
/// returning a guard that restores the previous depth when dropped.
pub fn enter_call_depth_scope(depth: u32) -> CallDepthScope {
    let previous = CURRENT_CALL_DEPTH.with(|slot| slot.replace(depth));
    CallDepthScope {
        previous,
        _not_send: PhantomData,
    }
}

/// The custom-function call depth in force on this thread (`0` outside any custom
/// function body).
#[must_use]
pub fn current_call_depth() -> u32 {
    CURRENT_CALL_DEPTH.with(std::cell::Cell::get)
}

/// Append the SHACL-SPARQL *shape context* pre-bindings — `$shapesGraph` and
/// `$currentShape` — to a substitution buffer the caller owns.
///
/// It is the CALLER's buffer, deliberately. These two bindings are constants of
/// the shape, so appending them inside the run helper meant re-allocating both
/// names, and copying every substitution the caller had already built, once per
/// focus node (or, on the custom-component ASK path, once per VALUE NODE). A
/// caller that runs the same query for many nodes now builds the whole
/// substitution list once and overwrites only the cells that actually vary.
pub(crate) fn push_shape_context(
    subs: &mut Vec<Prebinding<'_>>,
    shapes_graph_iri: Option<&str>,
    current_shape: Option<&Term>,
) {
    if let Some(iri) = shapes_graph_iri {
        subs.push(Prebinding {
            variable: "shapesGraph",
            value: TermValue::Iri(iri.to_owned()),
        });
    }
    if let Some(shape) = current_shape {
        subs.push(Prebinding {
            variable: "currentShape",
            value: shape.to_term_value(),
        });
    }
}

/// The shape-context parameter NAMES, appended in exactly the order
/// [`push_shape_context`] pushes their values.
///
/// The prepared door needs the name list before it has any values, and the two lists
/// must agree position for position or a run binds `$currentShape`'s term into
/// `$shapesGraph`'s slot. Kept immediately beside [`push_shape_context`] for that
/// reason, and checked against it by `shape_context_names_match_push_shape_context`.
pub(crate) fn push_shape_context_names(
    names: &mut Vec<&str>,
    shapes_graph_iri: Option<&str>,
    current_shape: Option<&Term>,
) {
    if shapes_graph_iri.is_some() {
        names.push("shapesGraph");
    }
    if current_shape.is_some() {
        names.push("currentShape");
    }
}

/// The parameter names for a query whose pre-bindings are `$this` and the shape
/// context — the shape [`eval_sparql_constraint_view`] has.
///
/// Returned as one of four `'static` slices rather than built, because this is read
/// once per FOCUS NODE and building a two-element list there is the per-focus-node
/// allocation the prepared door exists to remove.
pub(crate) fn this_and_shape_context_names(
    shapes_graph_iri: Option<&str>,
    current_shape: Option<&Term>,
) -> &'static [&'static str] {
    match (shapes_graph_iri.is_some(), current_shape.is_some()) {
        (false, false) => &["this"],
        (true, false) => &["this", "shapesGraph"],
        (false, true) => &["this", "currentShape"],
        (true, true) => &["this", "shapesGraph", "currentShape"],
    }
}

/// Bind the shape-context VALUES into `execution`, starting at `slot`, in exactly the
/// order [`push_shape_context_names`] declared their names.
///
/// Returns the next free slot, so a caller with trailing parameters can continue.
///
/// # Errors
///
/// `Err(String)` if a slot is not a declared parameter of `execution` — which means
/// the name list and the value list disagree, the one mistake this pairing exists to
/// make impossible.
pub(crate) fn bind_shape_context(
    execution: &mut ShaclExecution,
    mut slot: usize,
    shapes_graph_iri: Option<&str>,
    current_shape: Option<&Term>,
) -> Result<usize, String> {
    if let Some(iri) = shapes_graph_iri {
        execution.bind(slot, TermValue::Iri(iri.to_owned()))?;
        slot += 1;
    }
    if let Some(shape) = current_shape {
        execution.bind(slot, shape.to_term_value())?;
        slot += 1;
    }
    Ok(slot)
}

/// Run a SELECT query and project its interned solutions through `project`.
///
/// `prebind` selects the rewrite: [`ShaclPrebinding::None`] is the generic
/// substitution path SHACL-AF node expressions and `sh:SPARQLTarget` use;
/// [`ShaclPrebinding::Applied`] adds the SHACL-specific FILTER/EXISTS expression
/// substitution and `BOUND($v)` → `true` that `sh:sparql` constraint and component
/// bodies need.
///
/// A non-SELECT result is refused here rather than inside each caller, so the
/// three wordings stay one wording.
fn run_select_view<D: DatasetView + Sync + FocusGraphSource, R>(
    dataset: &D,
    select: &str,
    substitutions: &[Prebinding<'_>],
    prebind: ShaclPrebinding,
    project: impl FnOnce(&InternedSolutions<'_, '_, D>) -> Result<R, String>,
) -> Result<R, String> {
    run_query_view(dataset, select, substitutions, prebind, None, |outcome| {
        project_solutions(outcome, project)
    })
}

/// Run a SELECT query over the dataset using the generic SPARQL `query` path
/// with variable substitutions, projecting its interned solutions.
///
/// This is the path used by SHACL-AF node expressions (scalar, aggregate,
/// order-by). It does NOT apply the SHACL-specific pre-binding rewrite used for
/// `sh:sparql` constraint/component bodies.
pub(crate) fn run_select_generic_view<D: DatasetView + Sync + FocusGraphSource, R>(
    dataset: &D,
    select: &str,
    substitutions: &[Prebinding<'_>],
    project: impl FnOnce(&InternedSolutions<'_, '_, D>) -> Result<R, String>,
) -> Result<R, String> {
    run_select_view(
        dataset,
        select,
        substitutions,
        ShaclPrebinding::None,
        project,
    )
}

/// Run a SELECT query over the dataset using SHACL-SPARQL pre-binding semantics,
/// projecting its interned solutions.
///
/// `substitutions` is the COMPLETE pre-binding list, shape context included; see
/// [`push_shape_context`] for why it is assembled by the caller.
pub(crate) fn run_select_with_shacl_prebinding_view<D: DatasetView + Sync + FocusGraphSource, R>(
    dataset: &D,
    select: &str,
    substitutions: &[Prebinding<'_>],
    project: impl FnOnce(&InternedSolutions<'_, '_, D>) -> Result<R, String>,
) -> Result<R, String> {
    run_select_view(
        dataset,
        select,
        substitutions,
        ShaclPrebinding::Applied,
        project,
    )
}

/// Run an ASK query using SHACL-SPARQL pre-binding semantics.
///
/// An ASK materializes NO rows on any path: the evaluator answers the boolean from
/// the emptiness of the solution bag and never crosses a cell into the egress
/// model. It still runs through [`run_query_view`] rather than beside it, because
/// the one-door property is about where SHACL may reach the engine, not about
/// which results are expensive.
///
/// `substitutions` is the COMPLETE pre-binding list, shape context included; see
/// [`push_shape_context`].
pub(crate) fn run_ask_with_shacl_prebinding_view<D: DatasetView + Sync + FocusGraphSource>(
    dataset: &D,
    ask: &str,
    substitutions: &[Prebinding<'_>],
) -> Result<bool, String> {
    run_query_view(
        dataset,
        ask,
        substitutions,
        ShaclPrebinding::Applied,
        None,
        project_boolean,
    )
}

/// The number of query plans this thread's SHACL engine has memoized.
///
/// The plan cache is keyed on query TEXT and never evicts, which is correct for
/// a fixed set of authored `sh:select` / `sh:ask` / CONSTRUCT bodies and wrong
/// for any path that manufactures query text out of operand data — that path
/// would grow the cache with the INPUT. Exposed so the aggregate/order-by
/// evaluation path can pin "manufactures no query text" as a test.
#[cfg(test)]
fn cached_plan_count() -> usize {
    SPARQL_ENGINE.with(NativeSparqlEngine::cached_plan_count)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ::purrdf::RdfDataset;

    use super::*;
    use crate::report::Severity;
    use crate::term::{Literal, NamedNode, Term};

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

    // ── the shape-context name/value pairing ─────────────────────────────────

    /// **The name list and the value list agree, position for position, in all four
    /// shapes the context can take.**
    ///
    /// [`push_shape_context`] pushes VALUES and [`push_shape_context_names`] pushes
    /// NAMES, for the `&str` door and the prepared door respectively. Nothing in the
    /// type system relates them: they are two functions that happen to contain the
    /// same two `if let`s in the same order. If one gains a pre-binding the other
    /// does not, or gains it in the other order, a prepared run binds
    /// `$currentShape`'s term into `$shapesGraph`'s slot and answers a query nobody
    /// wrote — silently, because both terms are perfectly valid IRIs and the query
    /// still runs.
    ///
    /// [`this_and_shape_context_names`] is the third spelling, four `'static` slices
    /// chosen by a match, and it must agree with both.
    #[test]
    fn shape_context_names_match_push_shape_context() {
        let shape = dummy_shape();
        let iri = "http://example.org/shapes-graph";
        for graph in [None, Some(iri)] {
            for current in [None, Some(&shape)] {
                let mut values: Vec<Prebinding<'_>> = Vec::new();
                push_shape_context(&mut values, graph, current);

                let mut names: Vec<&str> = Vec::new();
                push_shape_context_names(&mut names, graph, current);

                let pushed: Vec<&str> = values.iter().map(|sub| sub.variable).collect();
                assert_eq!(
                    pushed,
                    names,
                    "push_shape_context and push_shape_context_names disagree for \
                     (shapes_graph = {graph:?}, current_shape = {})",
                    current.is_some()
                );

                let combined = this_and_shape_context_names(graph, current);
                let mut expected: Vec<&str> = vec!["this"];
                expected.extend_from_slice(&names);
                assert_eq!(
                    combined,
                    expected.as_slice(),
                    "this_and_shape_context_names disagrees for (shapes_graph = \
                     {graph:?}, current_shape = {})",
                    current.is_some()
                );
            }
        }
    }

    /// **A repeated parameter name is detected, and a neighbouring distinct one is
    /// not.**
    ///
    /// The predicate every prepared call site consults before choosing its door. The
    /// false case is the one that matters: over-reporting a duplicate would route a
    /// perfectly ordinary component down the `&str` door forever, which nothing else
    /// here would notice.
    #[test]
    fn parameters_are_distinct_separates_a_repeat_from_a_neighbour() {
        assert!(parameters_are_distinct(&[]));
        assert!(parameters_are_distinct(&["this", "value", "flag"]));
        assert!(!parameters_are_distinct(&["this", "value", "value"]));
        assert!(!parameters_are_distinct(&["this", "value", "this"]));
        // A name that merely CONTAINS another is not a repeat.
        assert!(parameters_are_distinct(&["value", "values", "value2"]));
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

    // ── sh:message templating on the sh:sparql path (SHACL-SPARQL §5.3.3) ─────
    //
    // `{$path}` / `{$value}` used to ship unsubstituted in
    // `sh:resultMessage` because the caller's message string was cloned once per
    // row while the row's own bindings sat unread. The substitution itself has
    // always existed (`components::substitute_message_templates`) — it was only
    // ever reached from the CUSTOM COMPONENT path, never from `sh:sparql`.
    //
    // Per CLAUDE.md's refusal rule these tests pin BOTH directions: the
    // placeholders that must now resolve, and the neighbouring ones that must
    // still be left alone rather than blanked or refused.

    /// A three-line dataset whose subject carries two typed and one plain literal.
    fn message_template_dataset() -> Arc<RdfDataset> {
        dataset_from_ntriples(&[
            "<http://example.org/s> <http://www.w3.org/2000/01/rdf-schema#label> \"Stakeholder requirement\" .",
        ])
    }

    /// The reported constraint shape, reduced to its essentials:
    /// project `?path` and `?value`, and reference both from `sh:message`.
    const UNTYPED_LITERAL_SELECT: &str = "SELECT $this ?path ?value WHERE { \
         $this ?path ?value . FILTER(isLiteral(?value)) \
         FILTER(DATATYPE(?value) = <http://www.w3.org/2001/XMLSchema#string>) }";

    #[test]
    fn eval_sparql_constraint_message_substitutes_path_and_value() {
        let dataset = message_template_dataset();
        let focus = named_term("http://example.org/s");
        let message =
            "Property {$path} contains an untyped plain string literal: '{$value}'.".to_owned();

        let results = eval_sparql_constraint(
            &dataset,
            &focus,
            UNTYPED_LITERAL_SELECT,
            &dummy_component(),
            &dummy_shape(),
            &Severity::Violation,
            Some(&message),
            None,
            None,
        )
        .expect("eval must succeed");

        assert_eq!(results.len(), 1, "exactly one untyped literal");
        assert_eq!(
            results[0].message.as_deref(),
            Some(
                "Property http://www.w3.org/2000/01/rdf-schema#label contains an untyped \
                 plain string literal: 'Stakeholder requirement'."
            ),
            "both {{$path}} and {{$value}} must be substituted from the solution"
        );
        // The verdict itself was never wrong; assert it did not move.
        assert_eq!(
            results[0].result_path,
            Some(named_term("http://www.w3.org/2000/01/rdf-schema#label"))
        );
        assert_eq!(results[0].severity, Severity::Violation);
    }

    #[test]
    fn eval_sparql_constraint_message_accepts_question_mark_sigil() {
        // §5.3.3 defines BOTH `{?var}` and `{$var}`. The reporter used `{$…}`;
        // a fix that only handled that sigil would silently keep half the bug.
        let dataset = message_template_dataset();
        let focus = named_term("http://example.org/s");
        let message = "value is '{?value}'".to_owned();

        let results = eval_sparql_constraint(
            &dataset,
            &focus,
            UNTYPED_LITERAL_SELECT,
            &dummy_component(),
            &dummy_shape(),
            &Severity::Violation,
            Some(&message),
            None,
            None,
        )
        .expect("eval must succeed");
        assert_eq!(
            results[0].message.as_deref(),
            Some("value is 'Stakeholder requirement'")
        );
    }

    #[test]
    fn eval_sparql_constraint_message_substitutes_this_even_when_unprojected() {
        // `$this` is PRE-BOUND to the focus node before the query runs, so it is
        // available to the template whether or not the SELECT projects it.
        let dataset = message_template_dataset();
        let focus = named_term("http://example.org/s");
        let message = "focus {$this} failed".to_owned();

        let results = eval_sparql_constraint(
            &dataset,
            &focus,
            "SELECT ?value WHERE { $this ?path ?value }",
            &dummy_component(),
            &dummy_shape(),
            &Severity::Violation,
            Some(&message),
            None,
            None,
        )
        .expect("eval must succeed");
        assert_eq!(
            results[0].message.as_deref(),
            Some("focus http://example.org/s failed"),
            "{{$this}} resolves from the pre-binding, not from the projection"
        );
    }

    #[test]
    fn eval_sparql_constraint_message_leaves_unbound_placeholder_verbatim() {
        // THE NEIGHBOURING VALID CASE. A placeholder naming a variable the
        // solution does not bind must survive untouched — not blanked, not
        // refused. `{$value}` is the sharp edge: `sh:value` DEFAULTS to the focus
        // node when `?value` is unbound (§5.3.1), and it would be easy to leak
        // that default into the message. It is a rule about the result field, not
        // a binding the solution carries, so the template must not see it.
        let dataset = message_template_dataset();
        let focus = named_term("http://example.org/s");
        let message = "unbound {$value}, absent {$nosuchvar}, literal {not-a-var}".to_owned();

        let results = eval_sparql_constraint(
            &dataset,
            &focus,
            "SELECT $this WHERE { $this ?p ?o }",
            &dummy_component(),
            &dummy_shape(),
            &Severity::Violation,
            Some(&message),
            None,
            None,
        )
        .expect("eval must succeed");

        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].message.as_deref(),
            Some("unbound {$value}, absent {$nosuchvar}, literal {not-a-var}"),
            "unbound and non-variable placeholders are left exactly as authored"
        );
        // ... while the RESULT's sh:value still defaults to the focus node.
        assert_eq!(results[0].value, Some(focus));
    }

    #[test]
    fn eval_sparql_constraint_message_is_rendered_per_row() {
        // The defect in its purest form: one message string was cloned for every
        // row. Two rows binding different values must now carry different text.
        let dataset = dataset_from_ntriples(&[
            "<http://example.org/s> <http://example.org/p> \"first\" .",
            "<http://example.org/s> <http://example.org/p> \"second\" .",
        ]);
        let focus = named_term("http://example.org/s");
        let message = "saw {$value}".to_owned();

        let results = eval_sparql_constraint(
            &dataset,
            &focus,
            "SELECT ?value WHERE { $this <http://example.org/p> ?value }",
            &dummy_component(),
            &dummy_shape(),
            &Severity::Violation,
            Some(&message),
            None,
            None,
        )
        .expect("eval must succeed");

        assert_eq!(results.len(), 2);
        let mut rendered: Vec<&str> = results
            .iter()
            .map(|r| r.message.as_deref().expect("message present"))
            .collect();
        rendered.sort_unstable();
        assert_eq!(
            rendered,
            vec!["saw first", "saw second"],
            "each row renders against its OWN bindings"
        );
    }

    #[test]
    fn eval_sparql_constraint_message_without_template_is_passed_through() {
        // The overwhelmingly common case: a plain message must be byte-identical
        // to what shipped before this change.
        let dataset = message_template_dataset();
        let focus = named_term("http://example.org/s");
        let message = "Values must be typed.".to_owned();

        let results = eval_sparql_constraint(
            &dataset,
            &focus,
            UNTYPED_LITERAL_SELECT,
            &dummy_component(),
            &dummy_shape(),
            &Severity::Violation,
            Some(&message),
            None,
            None,
        )
        .expect("eval must succeed");
        assert_eq!(results[0].message.as_deref(), Some("Values must be typed."));
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

    /// An RDF 1.2 triple term whose three components are the given IRIs.
    fn triple_term(s: &str, p: &str, o: &str) -> Term {
        Term::Triple(Box::new(crate::term::Triple::new(
            named_term(s),
            NamedNode::new_unchecked(p),
            named_term(o),
        )))
    }

    /// `MIN`/`MAX` rank a blank node and an RDF 1.2 triple term by the SAME total
    /// order every other term uses — blank < IRI < literal < triple — instead of
    /// refusing them.
    ///
    /// The refusal this replaces was never a property of the aggregate: it came
    /// from serializing operands into a SPARQL `VALUES` block, which cannot spell
    /// either kind. The comparator always could, so the capability asserted here
    /// is the one the evaluator had all along.
    #[test]
    fn eval_aggregate_ranks_blank_nodes_and_triple_terms() {
        let dataset = dataset_from_ntriples(&[]);
        let blank = Term::blank("b0");
        let iri = named_term("http://example.org/i");
        let literal = int_lit("7");
        let triple = triple_term(
            "http://example.org/s",
            "http://example.org/p",
            "http://example.org/o",
        );
        let vals = [triple.clone(), literal, iri, blank.clone()];

        assert_eq!(
            eval_aggregate(&dataset, "MIN", &vals).expect("min over mixed kinds"),
            Some(blank),
            "a blank node is the least term kind, so MIN is the blank node"
        );
        assert_eq!(
            eval_aggregate(&dataset, "MAX", &vals).expect("max over mixed kinds"),
            Some(triple),
            "a triple term is the greatest term kind, so MAX is the triple term"
        );
    }

    /// Two triple terms differing only in their object are ordered by that object,
    /// componentwise — the aggregate reaches INSIDE the RDF 1.2 term rather than
    /// treating every triple as one indivisible blob.
    #[test]
    fn eval_aggregate_orders_triple_terms_componentwise() {
        let dataset = dataset_from_ntriples(&[]);
        let low = triple_term(
            "http://example.org/s",
            "http://example.org/p",
            "http://example.org/a",
        );
        let high = triple_term(
            "http://example.org/s",
            "http://example.org/p",
            "http://example.org/b",
        );
        let vals = [high.clone(), low.clone()];
        assert_eq!(
            eval_aggregate(&dataset, "MIN", &vals).expect("min"),
            Some(low)
        );
        assert_eq!(
            eval_aggregate(&dataset, "MAX", &vals).expect("max"),
            Some(high)
        );
    }

    /// Evaluating the aggregate and the order-by over MANY DISTINCT operand sets
    /// adds nothing to the thread's query-plan cache.
    ///
    /// The cache is keyed on query text and never evicts, so a path that spliced
    /// the operands into a query would insert one permanent entry per distinct
    /// operand set — memory growing with the input data, not with the program.
    /// This asserts the count is unchanged across a hundred distinct sets, which
    /// is only possible if no query is issued at all.
    #[test]
    fn repeated_aggregate_and_order_evaluation_does_not_grow_the_plan_cache() {
        let dataset = dataset_from_ntriples(&[]);
        // Warm the thread-local engine with one real query so the baseline is a
        // populated cache rather than an empty one (an empty cache would pass this
        // test even if the count were being reset rather than left alone).
        eval_scalar_expr(&dataset, "STRLEN(\"warm\")", &[]).expect("warm-up query");
        let baseline = cached_plan_count();
        assert!(baseline > 0, "the warm-up query must have been cached");

        for i in 0..100u32 {
            let vals = [int_lit(&i.to_string()), int_lit(&(i + 1).to_string())];
            eval_aggregate(&dataset, "SUM", &vals).expect("sum");
            eval_aggregate(&dataset, "MIN", &vals).expect("min");
            eval_order(&dataset, &vals, false).expect("order");
        }

        assert_eq!(
            cached_plan_count(),
            baseline,
            "aggregate/order-by evaluation must issue no query, so it must add no plan-cache entry"
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
        assert_eq!(
            eval_order(&dataset, &[], false).expect("empty order"),
            [] as [_; 0]
        );
    }

    /// `sh:orderby` sorts a bag containing a blank node and an RDF 1.2 triple term
    /// into the SPARQL total order, ascending and descending, instead of refusing
    /// them — the capability half of the pair with
    /// `eval_aggregate_ranks_blank_nodes_and_triple_terms`.
    #[test]
    fn eval_order_ranks_blank_nodes_and_triple_terms() {
        let dataset = dataset_from_ntriples(&[]);
        let blank = Term::blank("b0");
        let iri = named_term("http://example.org/i");
        let literal = int_lit("7");
        let triple = triple_term(
            "http://example.org/s",
            "http://example.org/p",
            "http://example.org/o",
        );
        // Deliberately shuffled relative to the expected order.
        let vals = [literal.clone(), triple.clone(), blank.clone(), iri.clone()];

        assert_eq!(
            eval_order(&dataset, &vals, false).expect("ascending"),
            vec![blank.clone(), iri.clone(), literal.clone(), triple.clone()],
            "SPARQL orders term kinds blank < IRI < literal < triple"
        );
        assert_eq!(
            eval_order(&dataset, &vals, true).expect("descending"),
            vec![triple, literal, iri, blank],
            "descending is exactly the reverse of that total order"
        );
    }

    /// Triple terms sort componentwise (subject, then predicate, then object) and
    /// each component is itself ordered by the same total order — so a triple whose
    /// object is the smaller IRI sorts first even though its rendering is longer.
    #[test]
    fn eval_order_sorts_triple_terms_componentwise() {
        let dataset = dataset_from_ntriples(&[]);
        let a = triple_term(
            "http://example.org/s",
            "http://example.org/p",
            "http://example.org/aaa",
        );
        let b = triple_term(
            "http://example.org/s",
            "http://example.org/p",
            "http://example.org/b",
        );
        assert_eq!(
            eval_order(&dataset, &[b.clone(), a.clone()], false).expect("ascending"),
            vec![a, b]
        );
    }

    /// A blank node keeps ordering by its LABEL within the blank-node kind, so a
    /// bag of blanks is a stable, fully-determined sequence rather than an
    /// arbitrary one.
    #[test]
    fn eval_order_orders_blank_nodes_by_label() {
        let dataset = dataset_from_ntriples(&[]);
        let vals = [Term::blank("b2"), Term::blank("b0"), Term::blank("b1")];
        assert_eq!(
            eval_order(&dataset, &vals, false).expect("ascending"),
            vec![Term::blank("b0"), Term::blank("b1"), Term::blank("b2")]
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
