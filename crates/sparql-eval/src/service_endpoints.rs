// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A variable-endpoint `SERVICE ?e { … }` answered over the endpoints the pattern before
//! it binds.
//!
//! # What the specification says
//!
//! SPARQL 1.1 Federated Query §4, "SERVICE Variables", is explicitly informative: "we do
//! not present official evaluation semantics for the SPARQL pattern `SERVICE VAR`". What
//! it does say is that "a variable used in place of a service IRI indicates that the
//! service call for any solution depends on that variable's binding in that solution",
//! that such a clause "can be executed as a series of separate invocations of SPARQL
//! query services" whose results "are combined using union", and that "the query engine
//! must determine the possible target SPARQL query services … Execution order may also be
//! used to determine the list of services to be tried".
//!
//! # What this engine does with it
//!
//! Two evaluation routes, one meaning:
//!
//! * **A pattern earlier in the same group** — `?s ex:endpoint ?e . SERVICE ?e { … }`,
//!   including `VALUES`, `BIND` and an explicit `LATERAL` — is parsed into a `LATERAL`
//!   join, and [`crate::binop::eval_lateral`] substitutes each left solution's IRI into
//!   the clause (the per-solution dependency the section describes).
//! * **The left side of a group join, an `OPTIONAL` or a `MINUS`** — `{ ?s ex:endpoint
//!   ?e } { SERVICE ?e { … } }`, `?s ex:endpoint ?e OPTIONAL { SERVICE ?e { … } }`,
//!   `?s ex:endpoint ?e MINUS { SERVICE ?e { … } }` — is the case this module serves. The
//!   algebra evaluates those right operands independently of the left, so no solution is
//!   substituted in. Instead the list of services to be tried is taken from the
//!   execution order, as the section permits: the distinct IRIs `?e` takes in the left
//!   operand's solutions, in first-occurrence order. The clause becomes the union, over
//!   that list, of `{?e ↦ iri} ⋈ Invocation(iri, P)` — one request per distinct
//!   endpoint — and the enclosing operator then runs its own, unmodified algebra over it.
//!
//! Restricting the union to that list is not an approximation, and that is what licenses
//! it. Every left solution binds `?e` (a left operand that leaves `?e` unbound in some
//! solution is refused, see below), and every row the clause produces binds `?e` to the
//! endpoint that produced it. So a row from an endpoint outside the list is incompatible
//! with every left solution: it could join none of them (group join, `OPTIONAL`), and it
//! could remove none of them (`MINUS`, whose removal requires compatibility). The answer
//! is therefore exactly the one the union over every conceivable endpoint would give.
//! That argument holds only while every row the clause produces carries `?e` up to the
//! enclosing operator, which is why the positions it is used in are the ones
//! [`served_endpoint_variables`] classifies as direct.
//!
//! A `SILENT` endpoint that fails contributes the single empty solution for **that
//! endpoint only** — `{?e ↦ iri}` — so it neither joins with the left solutions bound to
//! the other endpoints nor pads them. An `OPTIONAL` left solution whose endpoint answers
//! nothing keeps its left bindings, as `OPTIONAL` always does.
//!
//! # What is refused, and why `SILENT` does not change it
//!
//! A `SERVICE ?e` evaluated where no solution names an endpoint has no endpoint to be sent
//! to. Returning the join identity there would make the enclosing join a no-op and the
//! answer look complete when nothing was asked, so the refusal is a hard
//! [`EvalError::Unsupported`] that names the shapes that do evaluate and the rewrite —
//! and it holds under `SILENT` as well, because `SILENT` tolerates an endpoint that fails,
//! not a query that names none.

use std::sync::Arc;

use purrdf_core::{DatasetView, TermValue};
use purrdf_sparql_algebra::{Expression, GraphPattern, NamedNodePattern, Variable};

use crate::error::EvalError;
use crate::eval::{EvalCtx, eval_evaluated};
use crate::governor::lift::{Evaluated, Truncation};
use crate::governor::soundness::ExpressionPart;
use crate::scratch::SolutionTerm;
use crate::solution::{Solution, SolutionSeq, VarSchema};

/// Whether the query being evaluated contains a variable-endpoint `SERVICE` anywhere.
///
/// Computed once per query by [`crate::eval::prepare_query_context`] (and by
/// [`crate::eval::eval`] for a bare pattern), so a query without one — nearly every
/// query — pays one allocation-free walk of its algebra and nothing per join.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EndpointScan {
    /// Not known: every join analyses its right operand. The state of a context no query
    /// was prepared into.
    Unknown,
    /// The query has no variable-endpoint `SERVICE`: nothing to analyse.
    Absent,
}

/// What a variable-endpoint `SERVICE` finds for its variable in the enclosing frames.
#[derive(Debug, Clone)]
pub(crate) enum EndpointBinding<I> {
    /// The distinct terms the left operand of the enclosing join binds the variable to,
    /// in first-occurrence order.
    Endpoints(Arc<[SolutionTerm<I>]>),
    /// The left operand binds the variable in some of its solutions but not all.
    PartlyUnbound,
    /// A sub-`SELECT` between the frame and the `SERVICE` does not project the variable:
    /// the `SERVICE` names a different variable of the same name.
    Hidden,
}

/// One enclosing operator's binding for one endpoint variable.
#[derive(Debug, Clone)]
pub(crate) struct EndpointFrame<I> {
    /// The endpoint variable.
    variable: Variable,
    /// What the operator binds it to.
    binding: EndpointBinding<I>,
}

/// How a `SERVICE ?v` inside an operand is reached from the operand's root.
#[derive(Debug)]
struct EndpointUse {
    /// The endpoint variable.
    variable: Variable,
    /// Reached through operators whose every row carries `?v` upwards.
    direct: bool,
    /// Also reached under an operator that could absorb, drop or re-select rows carrying
    /// `?v` (a nested `OPTIONAL` or `MINUS` right side, an `EXISTS`, a `LIMIT`/`OFFSET`,
    /// an aggregate that does not group by `?v`).
    conflict: bool,
}

/// A projection the walk has descended through.
#[derive(Clone, Copy)]
enum Scope<'a> {
    /// A `SELECT` list: a variable it does not name is a different variable below it.
    Project(&'a [Variable]),
    /// A `GROUP BY` list: rows below it reach the enclosing operator only through an
    /// aggregate unless the variable is a grouping key.
    Group(&'a [Variable]),
}

/// The endpoint variables of `right` this module can serve from the enclosing operator's
/// left operand: those of a `SERVICE ?v` reached only in direct positions (see the module
/// doc for why only those).
fn served_endpoint_variables(
    right: &GraphPattern,
    placeholders: Option<&crate::deferred_exists::DeferredMap>,
) -> Vec<Variable> {
    let mut uses = Vec::new();
    classify(right, true, &mut Vec::new(), &mut uses, placeholders);
    uses.into_iter()
        .filter(|u| u.direct && !u.conflict)
        .map(|u| u.variable)
        .collect()
}

/// Record one reached `SERVICE ?variable`.
fn record(variable: &Variable, direct: bool, scopes: &[Scope<'_>], uses: &mut Vec<EndpointUse>) {
    let mut direct = direct;
    for scope in scopes {
        match scope {
            Scope::Project(vars) if !vars.contains(variable) => return,
            Scope::Group(vars) if !vars.contains(variable) => direct = false,
            Scope::Project(_) | Scope::Group(_) => {}
        }
    }
    let index = if let Some(i) = uses.iter().position(|u| &u.variable == variable) {
        i
    } else {
        uses.push(EndpointUse {
            variable: variable.clone(),
            direct: false,
            conflict: false,
        });
        uses.len() - 1
    };
    if direct {
        uses[index].direct = true;
    } else {
        uses[index].conflict = true;
    }
}

/// Classify every `SERVICE ?v` reachable from `pattern`. `direct` says whether `pattern`
/// itself is in a direct position.
///
/// The match is wildcard-free so a new algebra variant is a compile error here rather
/// than a position silently classified.
fn classify<'a>(
    pattern: &'a GraphPattern,
    direct: bool,
    scopes: &mut Vec<Scope<'a>>,
    uses: &mut Vec<EndpointUse>,
    placeholders: Option<&crate::deferred_exists::DeferredMap>,
) {
    // A walk over an operand that may be deep; run inside a `crate::stack::walk` scope,
    // which discards the partial answer when a level refuses.
    if crate::stack::walk_is_low("SERVICE endpoint analysis") {
        return;
    }
    match pattern {
        GraphPattern::Bgp { .. }
        | GraphPattern::Path { .. }
        | GraphPattern::Values { .. }
        | GraphPattern::PropertyFunction(_) => {}
        // The body is forwarded as text, never evaluated here, so a `SERVICE` nested in
        // it is the remote endpoint's to resolve.
        GraphPattern::Service { name, .. } => {
            if let NamedNodePattern::Variable(variable) = name {
                record(variable, direct, scopes, uses);
            }
        }
        GraphPattern::Join { left, right } | GraphPattern::Lateral { left, right } => {
            classify(left, direct, scopes, uses, placeholders);
            classify(right, direct, scopes, uses, placeholders);
        }
        GraphPattern::Union { arms } => {
            for arm in arms {
                classify(arm, direct, scopes, uses, placeholders);
            }
        }
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => {
            classify(left, direct, scopes, uses, placeholders);
            classify(right, false, scopes, uses, placeholders);
            if let Some(expression) = expression {
                classify_expression(expression, scopes, uses, placeholders);
            }
        }
        GraphPattern::Minus { left, right } => {
            classify(left, direct, scopes, uses, placeholders);
            classify(right, false, scopes, uses, placeholders);
        }
        GraphPattern::Filter { expr, inner } => {
            classify(inner, direct, scopes, uses, placeholders);
            classify_expression(expr, scopes, uses, placeholders);
        }
        GraphPattern::Extend {
            inner, expression, ..
        }
        | GraphPattern::Unfold {
            inner, expression, ..
        } => {
            classify(inner, direct, scopes, uses, placeholders);
            classify_expression(expression, scopes, uses, placeholders);
        }
        GraphPattern::Graph { inner, .. }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner } => classify(inner, direct, scopes, uses, placeholders),
        GraphPattern::OrderBy { inner, expression } => {
            classify(inner, direct, scopes, uses, placeholders);
            for key in expression {
                match key {
                    purrdf_sparql_algebra::OrderExpression::Asc(e)
                    | purrdf_sparql_algebra::OrderExpression::Desc(e) => {
                        classify_expression(e, scopes, uses, placeholders);
                    }
                }
            }
        }
        GraphPattern::Project { inner, variables } => {
            scopes.push(Scope::Project(variables));
            classify(inner, direct, scopes, uses, placeholders);
            scopes.pop();
        }
        // A slice keeps a positional selection of its input, and which rows it keeps
        // depends on every row before them — including rows from endpoints outside the
        // list. The identity slice selects nothing.
        GraphPattern::Slice {
            inner,
            start,
            length,
        } => {
            let identity = *start == 0 && length.is_none();
            classify(inner, direct && identity, scopes, uses, placeholders);
        }
        GraphPattern::Group {
            inner,
            variables,
            aggregates,
        } => {
            scopes.push(Scope::Group(variables));
            classify(inner, direct, scopes, uses, placeholders);
            scopes.pop();
            for (_, aggregate) in aggregates {
                for e in aggregate.args().iter().chain(
                    aggregate
                        .order_by()
                        .iter()
                        .map(crate::modifier::order_sort_key),
                ) {
                    classify_expression(e, scopes, uses, placeholders);
                }
            }
        }
    }
}

/// Classify the `SERVICE ?v` clauses inside an expression's `EXISTS` patterns: never
/// direct, since an `EXISTS` answers a boolean rather than carrying rows upwards.
fn classify_expression<'a>(
    expr: &'a Expression,
    scopes: &mut Vec<Scope<'a>>,
    uses: &mut Vec<EndpointUse>,
    placeholders: Option<&crate::deferred_exists::DeferredMap>,
) {
    if crate::stack::walk_is_low("SERVICE endpoint analysis") {
        return;
    }
    crate::governor::soundness::visit_expression_parts(expr, &mut |part| {
        match part {
            ExpressionPart::Sub(sub) => classify_expression(sub, scopes, uses, placeholders),
            // A substituted copy's placeholder stands for a body the walk cannot see: its
            // site lists the body's `SERVICE ?v` clauses, and the substitution it is owed
            // decides which of them are still variable endpoints.
            ExpressionPart::Exists(pattern) => match placeholders
                .and_then(|map| map.get(&(std::ptr::from_ref(pattern) as usize)))
            {
                Some(slot) => {
                    for variable in &slot.site.service_uses {
                        if !slot.env.resolves_endpoint(variable) {
                            record(variable, false, scopes, uses);
                        }
                    }
                }
                None => classify(pattern, false, scopes, uses, placeholders),
            },
            ExpressionPart::Call(_) => {}
        }
        false
    });
}

/// Whether `pattern` contains a variable-endpoint `SERVICE` anywhere a local evaluation
/// can reach — every position, projections and `EXISTS` included, so the answer is
/// never "absent" for a clause some inner join could serve.
pub(crate) fn mentions_variable_endpoint(pattern: &GraphPattern) -> bool {
    if crate::stack::walk_is_low("SERVICE endpoint analysis") {
        return true;
    }
    if let GraphPattern::Service { name, .. } = pattern {
        return matches!(name, NamedNodePattern::Variable(_));
    }
    crate::governor::soundness::visit_pattern_parts(pattern, &mut |part| match part {
        crate::governor::soundness::PatternPart::Child(child, _) => {
            mentions_variable_endpoint(child)
        }
        crate::governor::soundness::PatternPart::Expression(expr) => {
            expression_mentions_variable_endpoint(expr)
        }
    })
}

/// [`mentions_variable_endpoint`] through an expression's `EXISTS` patterns. Recursive
/// rather than worklist-driven, so the once-per-query scan allocates nothing.
fn expression_mentions_variable_endpoint(expr: &Expression) -> bool {
    if crate::stack::walk_is_low("SERVICE endpoint analysis") {
        return true;
    }
    crate::governor::soundness::visit_expression_parts(expr, &mut |part| match part {
        ExpressionPart::Sub(sub) => expression_mentions_variable_endpoint(sub),
        ExpressionPart::Exists(pattern) => mentions_variable_endpoint(pattern),
        ExpressionPart::Call(_) => false,
    })
}

/// The scan [`crate::eval::prepare_query_context`] installs for `pattern`.
pub(crate) fn scan(pattern: &GraphPattern) -> EndpointScan {
    match crate::stack::walk(|| mentions_variable_endpoint(pattern)) {
        Ok(false) => EndpointScan::Absent,
        // Present, or the walk ran out of stack: analyse at every join, which is always
        // correct.
        Ok(true) | Err(_) => EndpointScan::Unknown,
    }
}

/// The innermost frame's binding for `variable`, if any frame names it.
fn binding_for<'c, I>(
    frames: &'c [EndpointFrame<I>],
    variable: &Variable,
) -> Option<&'c EndpointBinding<I>> {
    frames
        .iter()
        .rev()
        .find(|frame| &frame.variable == variable)
        .map(|frame| &frame.binding)
}

/// Evaluate `right`, the right operand of a group join, an `OPTIONAL` or a `MINUS` whose
/// left operand produced `left`, with every variable-endpoint `SERVICE` in a direct
/// position of `right` answered over the endpoints `left` binds.
///
/// # Errors
///
/// Whatever evaluating `right` raises, and [`EvalError::StackExhausted`] from the
/// analysis walk.
pub(crate) fn eval_right_operand<D: DatasetView + Sync>(
    left: &SolutionSeq<D::Id>,
    right: &GraphPattern,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    if ctx.endpoint_scan == EndpointScan::Absent {
        return eval_evaluated(right, ctx);
    }
    let placeholders = ctx.deferred_exists.clone();
    let served = crate::stack::walk(|| served_endpoint_variables(right, placeholders.as_deref()))?;
    if served.is_empty() {
        return eval_evaluated(right, ctx);
    }
    let depth = ctx.endpoint_frames.len();
    for variable in served {
        let column = left.schema.index_of(&variable);
        let binding = match column {
            Some(c) if left.rows.iter().all(|row| row[c].is_some()) => {
                let mut seen = crate::DetHashSet::default();
                let endpoints: Vec<SolutionTerm<D::Id>> = left
                    .rows
                    .iter()
                    .filter_map(|row| row[c])
                    .filter(|term| seen.insert(*term))
                    .collect();
                EndpointBinding::Endpoints(endpoints.into())
            }
            // An enclosing operator already lists this variable's endpoints, and this
            // operand is in a direct position of that operator's right side (it would not
            // have listed them otherwise), so its rows reach that operator's join on the
            // variable: its list stays the one in force.
            _ if matches!(
                binding_for(&ctx.endpoint_frames[..depth], &variable),
                Some(EndpointBinding::Endpoints(_))
            ) =>
            {
                continue;
            }
            Some(_) => EndpointBinding::PartlyUnbound,
            None => continue,
        };
        ctx.endpoint_frames
            .push(EndpointFrame { variable, binding });
    }
    let evaluated = eval_evaluated(right, ctx);
    ctx.endpoint_frames.truncate(depth);
    evaluated
}

/// Evaluate a sub-`SELECT`'s `inner` with every endpoint variable it does not project
/// hidden, so a `SERVICE ?v` below it — a different `?v` — is never answered over the
/// outer operator's endpoints.
///
/// # Errors
///
/// Whatever evaluating `inner` raises.
pub(crate) fn eval_projected<D: DatasetView + Sync>(
    inner: &GraphPattern,
    variables: &[Variable],
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    if ctx.endpoint_frames.is_empty() {
        return eval_evaluated(inner, ctx);
    }
    let depth = ctx.endpoint_frames.len();
    let hidden: Vec<Variable> = ctx.endpoint_frames[..depth]
        .iter()
        .filter(|frame| !variables.contains(&frame.variable))
        .map(|frame| frame.variable.clone())
        .collect();
    for variable in hidden {
        ctx.endpoint_frames.push(EndpointFrame {
            variable,
            binding: EndpointBinding::Hidden,
        });
    }
    let evaluated = eval_evaluated(inner, ctx);
    ctx.endpoint_frames.truncate(depth);
    evaluated
}

/// Evaluate `SERVICE ?variable { … }` (`node`) over the endpoints the enclosing operator
/// lists, or refuse it when none does.
///
/// # Errors
///
/// [`EvalError::Unsupported`] when no enclosing solution names an endpoint (under `SILENT`
/// too); [`EvalError::Remote`] when a non-`SILENT` clause's variable is bound to a term
/// that is not an IRI; and whatever each endpoint's evaluation raises.
pub(crate) fn eval_variable_endpoint<D: DatasetView + Sync>(
    node: &GraphPattern,
    variable: &Variable,
    silent: bool,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    match binding_for(&ctx.endpoint_frames, variable) {
        Some(EndpointBinding::Endpoints(endpoints)) => {
            let endpoints = Arc::clone(endpoints);
            eval_over_endpoints(node, variable, &endpoints, silent, ctx)
        }
        Some(EndpointBinding::PartlyUnbound) => Err(unbound_endpoint(
            variable,
            "some solutions of the pattern before it leave it unbound",
        )),
        Some(EndpointBinding::Hidden) | None => Err(unbound_endpoint(
            variable,
            "it is not bound to an IRI where the SERVICE is evaluated",
        )),
    }
}

/// The refusal for a `SERVICE ?variable` no solution names an endpoint for.
pub(crate) fn unbound_endpoint(variable: &Variable, cause: &str) -> EvalError {
    let v = variable.as_str();
    EvalError::unsupported(format!(
        "SERVICE ?{v} with no endpoint: ?{v} names the endpoint, but {cause}, so there is \
         no endpoint to send the request to. A variable endpoint is evaluated once per \
         distinct IRI ?{v} is bound to, wherever ?{v} is bound in every solution that \
         reaches the SERVICE: by a pattern earlier in the same group (a triple pattern, \
         VALUES, BIND or LATERAL), or by the left side of the OPTIONAL, MINUS or group join \
         whose right side holds the SERVICE — but not through a further OPTIONAL or MINUS \
         right side, an EXISTS, a LIMIT/OFFSET, an aggregate that does not group by ?{v}, \
         or a sub-SELECT that does not project ?{v}. Bind ?{v} before the SERVICE, e.g. \
         `?s <p> ?{v} . SERVICE ?{v} {{ … }}` or `?s <p> ?{v} LATERAL {{ SERVICE ?{v} {{ … }} \
         }}`. SILENT does not change this: it tolerates an endpoint that fails, not a query \
         that names none"
    ))
}

/// The refusal for a `SERVICE ?variable` whose variable is bound to a term that is not an
/// IRI: it names no endpoint, so there is nothing to send the request to. Under `SILENT`
/// the message says why `SILENT` does not apply.
pub(crate) fn non_iri_endpoint(variable: &Variable, value: &TermValue, silent: bool) -> EvalError {
    let v = variable.as_str();
    let mut message = format!(
        "SERVICE ?{v}: ?{v} is bound to {}, which is not an IRI, so there is no endpoint to \
         send the request to",
        describe_non_iri(value)
    );
    if silent {
        message.push_str(
            "; SILENT does not apply: it tolerates an endpoint that fails, not a value that \
             names none",
        );
    }
    EvalError::remote(message)
}

/// A short description of a non-IRI term, for the error naming it.
fn describe_non_iri(value: &TermValue) -> String {
    match value {
        TermValue::Iri(iri) => format!("<{iri}>"),
        TermValue::Blank { .. } => "a blank node".to_owned(),
        TermValue::Literal { lexical_form, .. } => format!("the literal \"{lexical_form}\""),
        TermValue::Triple { .. } => "a triple term".to_owned(),
    }
}

/// Each endpoint's own answer, in endpoint order.
type EndpointBlocks<I> = Vec<(SolutionTerm<I>, SolutionSeq<I>)>;

/// `⋃ { ?variable ↦ e } ⋈ Invocation(e, node's body)` over `endpoints`, in their order.
fn eval_over_endpoints<D: DatasetView + Sync>(
    node: &GraphPattern,
    variable: &Variable,
    endpoints: &[SolutionTerm<D::Id>],
    silent: bool,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    let key = VarSchema::from_vars([variable.clone()]);
    let mut blocks: EndpointBlocks<D::Id> = Vec::with_capacity(endpoints.len());
    let mut stopped = None;
    // A value that is not an IRI names no endpoint. That is the query's own fault, not an
    // endpoint that failed, so it is refused under `SILENT` too — the join identity would
    // claim an endpoint had been consulted — and it is refused before any request goes
    // out, so whether one does never depends on the order the endpoints are listed in.
    for &endpoint in endpoints {
        let value = ctx.scratch.value_of(ctx.dataset, endpoint);
        if !matches!(value, TermValue::Iri(_)) {
            return Err(non_iri_endpoint(variable, &value, silent));
        }
    }
    for &endpoint in endpoints {
        // The same substitution a `LATERAL` makes for one solution: the IRI becomes the
        // clause's endpoint and is injected into its body wherever the body names it.
        let row = crate::expr::outer_bindings_for_substitution(&[Some(endpoint)], &key, ctx);
        let substituted = crate::expr::substitute_pattern(node, &row)?;
        let GraphPattern::Service {
            name: name @ NamedNodePattern::NamedNode(_),
            inner,
            silent: substituted_silent,
        } = substituted.as_ref()
        else {
            return Err(EvalError::internal(
                "substituting an IRI endpoint into a variable-endpoint SERVICE did not \
                 resolve its endpoint",
            ));
        };
        match crate::remote::eval_service(&substituted, name, inner, *substituted_silent, ctx)? {
            Evaluated::Complete(seq) => blocks.push((endpoint, seq)),
            // Commit per endpoint: the block this endpoint was producing is discarded
            // whole, and every block before it is complete — a prefix of this clause's
            // ordered output.
            Evaluated::Truncated(truncation) => {
                stopped = Some(truncation.split().1);
                break;
            }
        }
    }

    let mut schema = VarSchema::from_vars([variable.clone()]);
    for (_, block) in &blocks {
        for v in block.schema.vars() {
            schema.push(v.clone());
        }
    }
    let schema = Arc::new(schema);
    let width = schema.len();
    let mut rows: Vec<Solution<D::Id>> = Vec::new();
    for (endpoint, block) in &blocks {
        let to_out: Vec<usize> = block
            .schema
            .vars()
            .iter()
            .map(|v| schema.index_of(v).unwrap_or(0))
            .collect();
        let own = block.schema.index_of(variable);
        for row in &block.rows {
            // A body that binds the endpoint variable itself was given the endpoint's IRI
            // by the substitution; a row binding it to anything else is not a solution
            // for this endpoint.
            if own.is_some_and(|j| row[j].is_some_and(|t| t != *endpoint)) {
                continue;
            }
            let mut out: Solution<D::Id> = smallvec::smallvec![None; width];
            out[0] = Some(*endpoint);
            for (j, cell) in row.iter().enumerate() {
                if cell.is_some() {
                    out[to_out[j]] = *cell;
                }
            }
            rows.push(out);
        }
    }
    let seq = SolutionSeq { schema, rows };
    Ok(match stopped {
        None => Evaluated::Complete(seq),
        Some(certificate) => Evaluated::Truncated(Truncation::new(seq, certificate)),
    })
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermValue};
    use purrdf_sparql_algebra::Variable;

    use crate::error::EvalError;
    use crate::eval::{EvalCtx, Outcome, evaluate_query, materialize_solutions};
    use crate::remote::{RemoteError, ResolvedBindings, ServiceRequest, ServiceResolver};

    const EX: &str = "http://example.org/";
    const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

    /// `ex:g1 → ex:e1`, `ex:g2 → ex:e2`, `ex:g3 → ex:e1` (a repeated endpoint), `ex:g4 →
    /// ex:e3` (an endpoint that answers nothing) and `ex:g5 → ex:down` (an endpoint that
    /// fails), all through `ex:endpoint`; and `ex:g1 ex:other ex:o`.
    fn local() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let endpoint = b.intern_iri(&format!("{EX}endpoint"));
        for (g, e) in [
            ("g1", "e1"),
            ("g2", "e2"),
            ("g3", "e1"),
            ("g4", "e3"),
            ("g5", "down"),
        ] {
            let g = b.intern_iri(&format!("{EX}{g}"));
            let e = b.intern_iri(&format!("{EX}{e}"));
            b.push_quad(g, endpoint, e, None);
        }
        let g1 = b.intern_iri(&format!("{EX}g1"));
        let other = b.intern_iri(&format!("{EX}other"));
        let o = b.intern_iri(&format!("{EX}o"));
        b.push_quad(g1, other, o, None);
        b.freeze().expect("freeze")
    }

    /// Every endpoint answers `?x = "answer-from-<local name>"`, so a row joined to the
    /// wrong endpoint is visible in its value; `ex:e3` answers no rows and `ex:down` fails
    /// at the transport. Every request is recorded, in order.
    #[derive(Debug, Default)]
    struct Endpoints {
        requests: Mutex<Vec<String>>,
    }

    impl Endpoints {
        fn requests(&self) -> Vec<String> {
            self.requests.lock().expect("lock").clone()
        }
    }

    impl ServiceResolver for Endpoints {
        fn resolve(&self, request: ServiceRequest<'_>) -> Result<ResolvedBindings, RemoteError> {
            self.requests
                .lock()
                .expect("lock")
                .push(request.endpoint.to_owned());
            let name = request
                .endpoint
                .strip_prefix(EX)
                .unwrap_or(request.endpoint);
            if name == "down" {
                return Err(RemoteError::Transport("connection refused".to_owned()));
            }
            let rows = if name == "e3" {
                Vec::new()
            } else {
                vec![vec![Some(TermValue::Literal {
                    lexical_form: format!("answer-from-{name}"),
                    datatype: XSD_STRING.to_owned(),
                    language: None,
                    direction: None,
                })]]
            };
            Ok(ResolvedBindings {
                variables: vec![Variable::new("x")],
                rows,
                cell_limit_exceeded_at: None,
            })
        }
    }

    /// Run `query` over [`local`] with `source`, rendering each row as `name=value` cells
    /// joined by `&` (unbound names omitted, local names for IRIs), sorted.
    fn run(source: &Endpoints, query: &str) -> Result<Vec<String>, EvalError> {
        let ds = local();
        let parsed = purrdf_sparql_algebra::SparqlParser::new()
            .parse_query(query)
            .expect("parse");
        let mut ctx = EvalCtx::new(&ds).with_remote(source);
        let Outcome::Solutions(seq) = evaluate_query(&parsed, &mut ctx)? else {
            panic!("a SELECT answers solutions");
        };
        let (variables, rows) = materialize_solutions(&seq, &ctx);
        let mut out: Vec<String> = rows
            .iter()
            .map(|row| {
                let mut cells: Vec<String> = variables
                    .iter()
                    .zip(row)
                    .filter_map(|(v, cell)| {
                        cell.as_ref().map(|value| {
                            let text = match value {
                                TermValue::Iri(iri) => {
                                    iri.strip_prefix(EX).unwrap_or(iri).to_owned()
                                }
                                TermValue::Literal { lexical_form, .. } => lexical_form.clone(),
                                other => format!("{other:?}"),
                            };
                            format!("{v}={text}")
                        })
                    })
                    .collect();
                cells.sort();
                cells.join("&")
            })
            .collect();
        out.sort();
        Ok(out)
    }

    fn q(pattern: &str) -> String {
        use std::fmt::Write as _;

        // `ex:name` → `<http://example.org/name>`, one token at a time.
        let mut out = String::new();
        for token in pattern.split(' ') {
            if let Some(local) = token.strip_prefix("ex:") {
                let _ = write!(out, "<{EX}{local}>");
            } else {
                out.push_str(token);
            }
            out.push(' ');
        }
        format!("SELECT ?g ?e ?x WHERE {{ {out}}}")
    }

    fn requested(source: &Endpoints) -> Vec<String> {
        let mut endpoints: Vec<String> = source
            .requests()
            .into_iter()
            .map(|e| e.strip_prefix(EX).unwrap_or(&e).to_owned())
            .collect();
        endpoints.sort();
        endpoints
    }

    /// Every endpoint but the failing one, bound by the left side.
    const BOUND: &str = "VALUES ?e { ex:e1 ex:e2 ex:e3 } ?g ex:endpoint ?e";

    #[test]
    fn optional_answers_per_endpoint_and_keeps_a_left_row_its_endpoint_answers_nothing_for() {
        let source = Endpoints::default();
        let rows = run(
            &source,
            &q(&format!("{BOUND} OPTIONAL {{ SERVICE ?e {{ ?s ?p ?x }} }}")),
        )
        .expect("an endpoint bound by the OPTIONAL's left side is evaluated");
        assert_eq!(
            rows,
            [
                "e=e1&g=g1&x=answer-from-e1",
                "e=e1&g=g3&x=answer-from-e1",
                "e=e2&g=g2&x=answer-from-e2",
                "e=e3&g=g4",
            ],
            "each left row joins its own endpoint's answer; ex:g4's endpoint answered \
             nothing, so the OPTIONAL keeps its left bindings"
        );
        assert_eq!(
            requested(&source),
            ["e1", "e2", "e3"],
            "one request per distinct endpoint: ex:e1 is bound twice and asked once"
        );
    }

    #[test]
    fn a_group_join_answers_per_endpoint() {
        let source = Endpoints::default();
        let rows = run(
            &source,
            &q(&format!("{{ {BOUND} }} {{ SERVICE ?e {{ ?s ?p ?x }} }}")),
        )
        .expect("an endpoint bound by the group join's left side is evaluated");
        assert_eq!(
            rows,
            [
                "e=e1&g=g1&x=answer-from-e1",
                "e=e1&g=g3&x=answer-from-e1",
                "e=e2&g=g2&x=answer-from-e2",
            ],
            "the inner join drops ex:g4, whose endpoint answered nothing"
        );
        assert_eq!(requested(&source), ["e1", "e2", "e3"]);

        // The same join written flat is the LATERAL route: the same rows.
        let flat = Endpoints::default();
        assert_eq!(
            run(&flat, &q(&format!("{BOUND} SERVICE ?e {{ ?s ?p ?x }}"))).expect("flat"),
            rows,
            "the group join and the flat join are the same join"
        );
    }

    #[test]
    fn minus_removes_the_rows_whose_endpoint_has_a_compatible_answer() {
        let source = Endpoints::default();
        let rows = run(
            &source,
            &q(&format!("{BOUND} MINUS {{ SERVICE ?e {{ ?s ?p ?x }} }}")),
        )
        .expect("an endpoint bound by the MINUS's left side is evaluated");
        assert_eq!(
            rows,
            ["e=e3&g=g4"],
            "every row whose endpoint answered is removed; ex:g4's answered nothing"
        );
        assert_eq!(requested(&source), ["e1", "e2", "e3"]);

        // The FILTER NOT EXISTS reading substitutes each row instead: the same rows.
        let exists = Endpoints::default();
        assert_eq!(
            run(
                &exists,
                &q(&format!(
                    "{BOUND} FILTER NOT EXISTS {{ SERVICE ?e {{ ?s ?p ?x }} }}"
                ))
            )
            .expect("not exists"),
            rows
        );
    }

    #[test]
    fn a_silent_endpoint_failure_is_the_identity_for_that_endpoint_only() {
        let all = "?g ex:endpoint ?e";
        let source = Endpoints::default();
        let rows = run(
            &source,
            &q(&format!(
                "{all} OPTIONAL {{ SERVICE SILENT ?e {{ ?s ?p ?x }} }}"
            )),
        )
        .expect("SILENT swallows the failing endpoint");
        assert_eq!(
            rows,
            [
                "e=down&g=g5",
                "e=e1&g=g1&x=answer-from-e1",
                "e=e1&g=g3&x=answer-from-e1",
                "e=e2&g=g2&x=answer-from-e2",
                "e=e3&g=g4",
            ]
        );
        assert_eq!(requested(&source), ["down", "e1", "e2", "e3"]);

        // In a group join the failing endpoint keeps its own left row and nobody else's:
        // an identity row without the endpoint binding would pad every left row.
        let join = Endpoints::default();
        assert_eq!(
            run(
                &join,
                &q(&format!(
                    "{{ {all} }} {{ SERVICE SILENT ?e {{ ?s ?p ?x }} }}"
                ))
            )
            .expect("group join"),
            [
                "e=down&g=g5",
                "e=e1&g=g1&x=answer-from-e1",
                "e=e1&g=g3&x=answer-from-e1",
                "e=e2&g=g2&x=answer-from-e2",
            ]
        );

        // Without SILENT the same failure is the query's failure.
        let loud = Endpoints::default();
        let error = run(
            &loud,
            &q(&format!("{all} OPTIONAL {{ SERVICE ?e {{ ?s ?p ?x }} }}")),
        )
        .expect_err("a failing endpoint fails a non-silent SERVICE");
        assert!(
            matches!(&error, EvalError::Remote(m) if m.contains("http://example.org/down")),
            "{error}"
        );
    }

    #[test]
    fn an_endpoint_no_solution_binds_is_refused_even_under_silent() {
        for shape in [
            "SERVICE ?e { ?s ?p ?x }",
            "SERVICE SILENT ?e { ?s ?p ?x }",
            "?g ex:other ?o OPTIONAL { SERVICE SILENT ?e { ?s ?p ?x } }",
            "?g ex:other ?o MINUS { SERVICE SILENT ?e { ?s ?p ?x } }",
            "{ ?g ex:other ?o } { SERVICE SILENT ?e { ?s ?p ?x } }",
        ] {
            let source = Endpoints::default();
            let error = run(&source, &q(shape)).expect_err(shape);
            assert!(
                matches!(&error, EvalError::Unsupported { what, .. }
                    if what.contains("SERVICE ?e with no endpoint")
                        && what.contains("LATERAL")),
                "{shape}: {error}"
            );
            assert_eq!(
                source.requests(),
                Vec::<String>::new(),
                "{shape}: nothing was asked"
            );
        }
        // The bound neighbours answer from the endpoint under SILENT: a swallowed refusal
        // would have left `ex:g1` alone, with no `?x` and no request.
        for shape in [
            "?g ex:other ?o . ?g ex:endpoint ?e OPTIONAL { SERVICE SILENT ?e { ?s ?p ?x } }",
            "?g ex:other ?o . ?g ex:endpoint ?e SERVICE SILENT ?e { ?s ?p ?x }",
        ] {
            let source = Endpoints::default();
            assert_eq!(
                run(&source, &q(shape)).expect(shape),
                ["e=e1&g=g1&x=answer-from-e1"],
                "{shape}"
            );
            assert_eq!(requested(&source), ["e1"], "{shape}");
        }
    }

    #[test]
    fn a_left_side_that_leaves_the_endpoint_unbound_somewhere_is_refused() {
        let source = Endpoints::default();
        let error = run(
            &source,
            &q("{ ?g ex:endpoint ?e } UNION { ?g ex:other ?o } OPTIONAL { SERVICE ?e { ?s ?p ?x } }"),
        )
        .expect_err("a left row without an endpoint names none");
        assert!(
            matches!(&error, EvalError::Unsupported { what, .. }
                if what.contains("some solutions of the pattern before it leave it unbound")),
            "{error}"
        );
        assert_eq!(source.requests(), Vec::<String>::new());
        // The neighbour whose every left row binds it answers.
        let bound = Endpoints::default();
        assert_eq!(
            run(
                &bound,
                &q("{ ?g ex:endpoint ?e FILTER ( ?e = ex:e2 ) } UNION { ?g ex:endpoint ?e FILTER ( ?e = ex:e3 ) } OPTIONAL { SERVICE ?e { ?s ?p ?x } }")
            )
            .expect("bound in every row"),
            ["e=e2&g=g2&x=answer-from-e2", "e=e3&g=g4"]
        );
    }

    #[test]
    fn a_service_under_a_further_optional_or_an_unprojecting_subselect_is_refused() {
        for shape in [
            // Under a further OPTIONAL whose own left side does not bind ?e.
            "?g ex:endpoint ?e { ?g ex:other ?o OPTIONAL { SERVICE ?e { ?s ?p ?x } } }",
            // A sub-SELECT that does not project ?e: its ?e is a different variable.
            "?g ex:endpoint ?e { SELECT ?x { SERVICE ?e { ?s ?p ?x } } }",
        ] {
            let source = Endpoints::default();
            let error = run(&source, &q(shape)).expect_err(shape);
            assert!(
                matches!(&error, EvalError::Unsupported { what, .. }
                    if what.contains("SERVICE ?e with no endpoint")),
                "{shape}: {error}"
            );
            assert_eq!(source.requests(), Vec::<String>::new(), "{shape}");
        }
        // The neighbours: the OPTIONAL with ?e bound on its own left, and the sub-SELECT
        // that projects ?e.
        for shape in [
            "VALUES ?e { ex:e1 } ?g ex:endpoint ?e { ?g ex:other ?o . ?g ex:endpoint ?e OPTIONAL { SERVICE ?e { ?s ?p ?x } } }",
            "VALUES ?e { ex:e1 } ?g ex:endpoint ?e { SELECT ?e ?x { SERVICE ?e { ?s ?p ?x } } }",
        ] {
            let source = Endpoints::default();
            let rows = run(&source, &q(shape)).expect(shape);
            assert!(
                rows.contains(&"e=e1&g=g1&x=answer-from-e1".to_owned()),
                "{shape}: {rows:?}"
            );
            assert!(
                rows.iter().all(|row| !row.contains("answer-from-e2")),
                "{shape}: only ex:e1 is bound on the left: {rows:?}"
            );
        }
    }

    #[test]
    fn an_endpoint_bound_to_a_literal_is_refused_under_silent_too() {
        // A literal names no endpoint: refused, SILENT or not, on the endpoint-list route
        // (a VALUES or a pattern before the SERVICE) and the substitution route
        // (LATERAL), with no request.
        for silent in ["", "SILENT "] {
            for shape in [
                "VALUES ?e { \"not-an-iri\" } OPTIONAL { SERVICE SILENT ?e { ?s ?p ?x } }",
                "VALUES ?e { \"not-an-iri\" } SERVICE SILENT ?e { ?s ?p ?x }",
                "BIND(\"not-an-iri\" AS ?e) SERVICE SILENT ?e { ?s ?p ?x }",
                "VALUES ?e { \"not-an-iri\" } LATERAL { SERVICE SILENT ?e { ?s ?p ?x } }",
            ] {
                let shape = shape.replace("SILENT ", silent);
                let source = Endpoints::default();
                let error = run(&source, &q(&shape)).expect_err(&shape);
                let text = error.to_string();
                assert!(
                    text.contains("?e is bound to the literal \"not-an-iri\", which is not an IRI")
                        || text.contains("?e names the endpoint, but it is not bound to an IRI"),
                    "{shape}: {text}"
                );
                if !silent.is_empty() {
                    assert!(
                        text.contains("SILENT does not apply")
                            || text.contains("SILENT does not change this"),
                        "{shape}: {text}"
                    );
                }
                assert_eq!(source.requests(), Vec::<String>::new(), "{shape}");
            }
        }
        // The valid neighbours: an IRI endpoint under SILENT answers, one request; and a
        // listed endpoint that fails at the transport is the identity under SILENT, for
        // that endpoint alone.
        let source = Endpoints::default();
        assert_eq!(
            run(
                &source,
                &q("VALUES ?e { <http://example.org/e1> } SERVICE SILENT ?e { ?s ?p ?x }")
            )
            .expect("an IRI endpoint answers"),
            ["e=e1&x=answer-from-e1"]
        );
        assert_eq!(source.requests(), [format!("{EX}e1")]);
        let source = Endpoints::default();
        assert_eq!(
            run(
                &source,
                &q("VALUES ?e { <http://example.org/down> } SERVICE SILENT ?e { ?s ?p ?x }")
            )
            .expect("a failing endpoint is the identity under SILENT"),
            ["e=down"]
        );
        assert_eq!(source.requests(), [format!("{EX}down")]);
    }
}
