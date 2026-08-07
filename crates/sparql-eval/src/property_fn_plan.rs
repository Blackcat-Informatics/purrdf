// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! **Prepare-time feasibility ordering** for property-function calls: the rewrite that
//! turns a group's textual order into one the registered relations can actually serve.
//!
//! A relation is rarely computable in every direction (see
//! [`crate::property_fn`]). `split(?whole, ?part)` enumerates parts from a whole and
//! not the reverse; a full-text relation wants its needle bound. So the order a query
//! author writes is not necessarily an order the engine can run:
//!
//! ```text
//! { ?doc ex:contains ("needle" ?score) . ?doc ex:section ex:intro }
//! ```
//!
//! reads left to right as "invoke the relation with everything free, then filter", and
//! for an `fb`-only relation that is not merely slow — it is infeasible. Ordering the
//! data pattern first makes `?doc` bound and the call `bf`.
//!
//! # What this pass is, and is deliberately not
//!
//! It is **statistics-free and deterministic**. It reads only the algebra and the
//! registry's declarations — never the dataset, never a cardinality estimate — so the
//! order a query gets is a pure function of its text and the host's configuration, and
//! two runs of the same query against different data plan identically. It is not a
//! cost-based join planner: `crate::bgp` already reorders triple patterns inside a BGP
//! using real cardinalities, and this pass never reaches inside one.
//!
//! # Where it runs, and why there
//!
//! At **prepare** time, on the parsed algebra, before evaluation begins — so the
//! admission failures below (an unregistered IRI, an arity mismatch, an order no
//! relation can serve) are raised before a governed execution has spent a single unit
//! of its budget on a query that could never have run. A caller's ceiling is for the
//! work its query does, not for discovering that the query is misconfigured.

use purrdf_core::binding_pattern::BindingPattern;
use purrdf_sparql_algebra::{
    AggregateExpression, Expression, GraphPattern, NamedNodePattern, OrderExpression,
    PropertyFunctionCall, Query, TermPattern, TriplePattern, Variable,
};

use crate::DetHashSet;
use crate::error::EvalError;
use crate::property_fn::{PfArity, PropertyFunctionRegistry};

/// Rewrite every property-function chain in `query` into a feasible order.
///
/// The query is returned unchanged — and no work is done at all — when it carries no
/// call node, which is every query on a host that has not configured the seam.
///
/// # Errors
///
/// [`EvalError::Function`] for an unregistered predicate IRI, an arity mismatch between
/// the call site and the relation's declaration, or a chain no total order can serve.
pub(crate) fn plan_query(
    query: &Query,
    relations: Option<&PropertyFunctionRegistry>,
) -> Result<Option<Query>, EvalError> {
    let pattern = match query {
        Query::Select { pattern, .. }
        | Query::Ask { pattern, .. }
        | Query::Construct { pattern, .. }
        | Query::Describe { pattern, .. } => pattern,
    };
    if !crate::property_fn_eval::pattern_reaches_property_function(pattern) {
        return Ok(None);
    }
    let planned = plan_pattern(pattern, relations, &DetHashSet::default())?;
    let mut planned_query = query.clone();
    match &mut planned_query {
        Query::Select { pattern, .. }
        | Query::Ask { pattern, .. }
        | Query::Construct { pattern, .. }
        | Query::Describe { pattern, .. } => *pattern = planned,
    }
    Ok(Some(planned_query))
}

// ---------------------------------------------------------------------------
// The chain
// ---------------------------------------------------------------------------

/// One member of a chain: the operand, and how it re-attaches when the chain is
/// rebuilt.
#[derive(Debug)]
struct Atom<'a> {
    /// The pattern itself.
    pattern: &'a GraphPattern,
    /// The call, when this atom is one — the only kind that can be infeasible.
    call: Option<&'a PropertyFunctionCall>,
    /// The atom's position in the chain as written, the last tie-break.
    position: usize,
}

/// Rewrite `pattern`, and everything under it, into a feasible order.
///
/// `outer` is the set of variables CERTAINLY bound by the enclosing context — see
/// [`collect_certainly_bound`] for what earns a variable a place in it.
fn plan_pattern(
    pattern: &GraphPattern,
    relations: Option<&PropertyFunctionRegistry>,
    outer: &DetHashSet<Variable>,
) -> Result<GraphPattern, EvalError> {
    // A chain is a left-deep spine of `Lateral`s (a call's join) and `Join`s (the
    // residual data written between two calls), which is exactly the shape the parser
    // assembles a triples block containing calls into. Anything else recurses
    // structurally.
    let mut atoms = Vec::new();
    if collect_chain(pattern, &mut atoms) && atoms.iter().any(|atom| atom.call.is_some()) {
        return order_chain(atoms, relations, outer);
    }
    map_children(pattern, relations, outer)
}

/// Peel the chain spine, pushing its atoms in TEXTUAL order (base first).
///
/// Returns whether `pattern` is a chain node at all: a bare `Bgp` or any other leaf is
/// not, so an ordinary query never allocates past the empty vector above.
fn collect_chain<'a>(pattern: &'a GraphPattern, atoms: &mut Vec<Atom<'a>>) -> bool {
    match pattern {
        GraphPattern::Lateral { left, right } | GraphPattern::Join { left, right } => {
            let call = match (&**right, pattern) {
                (GraphPattern::PropertyFunction(call), GraphPattern::Lateral { .. }) => Some(call),
                // A call under a `Join` rather than a `Lateral` would lose the
                // dependency the `Lateral` encodes, so it is not treated as a chain
                // member; the structural recursion handles it.
                (GraphPattern::PropertyFunction(_), _) => return false,
                _ => None,
            };
            if !collect_chain(left, atoms) {
                push_atom(left, None, atoms);
            }
            push_atom(right, call, atoms);
            true
        }
        _ => false,
    }
}

/// Push one chain member, dropping the EMPTY `Bgp` the parser leaves where a call opens
/// its block. It is the identity table `Z`, so joining it back in would only widen the
/// rebuilt tree with a node that binds nothing and matches everything.
fn push_atom<'a>(
    pattern: &'a GraphPattern,
    call: Option<&'a PropertyFunctionCall>,
    atoms: &mut Vec<Atom<'a>>,
) {
    if matches!(pattern, GraphPattern::Bgp { patterns } if patterns.is_empty()) {
        return;
    }
    let position = atoms.len();
    atoms.push(Atom {
        pattern,
        call,
        position,
    });
}

/// The greedy feasibility order over one chain's atoms, and the rebuilt spine.
///
/// The algorithm, in full:
///
/// 1. Start with the variables certainly bound by the enclosing context (`outer`).
/// 2. Repeatedly pick the next atom that is FEASIBLE given what is bound so far. A
///    non-call atom is always feasible; a call is feasible iff its relation
///    [`admits`](crate::property_fn::PropertyFunction::admits) the access pattern in
///    which a position is bound exactly when its term is a constant, or a variable
///    already bound by an earlier atom.
/// 3. Break ties by lowest declared `rows_per_invocation`, then by IRI, then by textual
///    position — every one of them total, so the chosen order is a pure function of the
///    input.
/// 4. Add the chosen atom's certainly-bound variables to the bound set and repeat.
///
/// A non-call atom sorts as `rows_per_invocation = 0` under an empty IRI, so data
/// patterns are scheduled ahead of calls. That is the whole point of the pass: a data
/// atom can never be infeasible and can only ever ADD bindings, so running it first
/// maximizes the access patterns available to the calls that follow — and it costs
/// nothing, because a chain member is evaluated once regardless of where it sits.
fn order_chain(
    atoms: Vec<Atom<'_>>,
    relations: Option<&PropertyFunctionRegistry>,
    outer: &DetHashSet<Variable>,
) -> Result<GraphPattern, EvalError> {
    let mut bound = outer.clone();
    let mut remaining: Vec<Atom<'_>> = atoms;
    let mut ordered: Vec<&GraphPattern> = Vec::with_capacity(remaining.len());
    let mut is_call: Vec<bool> = Vec::with_capacity(remaining.len());

    while !remaining.is_empty() {
        let mut best: Option<(usize, (u64, &str, usize))> = None;
        for (index, atom) in remaining.iter().enumerate() {
            let Some(call) = atom.call else {
                let key = (0_u64, "", atom.position);
                if best.is_none_or(|(_, current)| key < current) {
                    best = Some((index, key));
                }
                continue;
            };
            let relation = resolve(call, relations)?;
            check_arity(call, relation.arity())?;
            let mode = invocation_mode(call, &bound);
            if !relation.admits(mode) {
                continue;
            }
            let key = (
                relation.rows_per_invocation(mode),
                call.iri.as_str(),
                atom.position,
            );
            if best.is_none_or(|(_, current)| key < current) {
                best = Some((index, key));
            }
        }
        let Some((index, _)) = best else {
            return Err(stuck(&remaining, relations, &bound));
        };
        let atom = remaining.remove(index);
        collect_certainly_bound(atom.pattern, &mut bound);
        ordered.push(atom.pattern);
        is_call.push(atom.call.is_some());
    }

    // Rebuild the left-deep spine in the chosen order: a call re-attaches through a
    // `Lateral` (it depends on what is to its left), everything else through a `Join`.
    let mut chain: Option<GraphPattern> = None;
    for (pattern, call) in ordered.into_iter().zip(is_call) {
        let planned = plan_pattern(pattern, relations, &bound)?;
        chain = Some(match chain {
            None => planned,
            Some(left) => {
                if call {
                    GraphPattern::Lateral {
                        left: Box::new(left),
                        right: Box::new(planned),
                    }
                } else {
                    GraphPattern::Join {
                        left: Box::new(left),
                        right: Box::new(planned),
                    }
                }
            }
        });
    }
    Ok(chain.unwrap_or(GraphPattern::Bgp { patterns: vec![] }))
}

/// The invocation access pattern a call would have with `bound` already established: a
/// position is bound iff its term is fully determined by constants and bound variables.
///
/// Shared with the plan survey (`crate::bgp::survey_pattern_plans`), which needs the same
/// answer to read the relation's declared row bound for the mode a call is actually
/// invoked in: a relation that is cheap bound and expensive free would otherwise be
/// admitted, or refused, against a mode the query never uses.
pub(crate) fn invocation_mode(
    call: &PropertyFunctionCall,
    bound: &DetHashSet<Variable>,
) -> BindingPattern {
    BindingPattern::from_bools(
        call.subject_args
            .iter()
            .chain(&call.object_args)
            .map(|term| term_is_bound(term, bound)),
    )
}

/// Whether an argument term denotes a known value under `bound`.
///
/// A blank node is a non-distinguished variable and is never bound; a quoted triple is
/// bound only when every component is.
fn term_is_bound(term: &TermPattern, bound: &DetHashSet<Variable>) -> bool {
    match term {
        TermPattern::NamedNode(_) | TermPattern::Literal(_) => true,
        TermPattern::BlankNode(_) => false,
        TermPattern::Variable(variable) => bound.contains(variable),
        TermPattern::Triple(triple) => {
            term_is_bound(&triple.subject, bound)
                && match &triple.predicate {
                    NamedNodePattern::NamedNode(_) => true,
                    NamedNodePattern::Variable(variable) => bound.contains(variable),
                }
                && term_is_bound(&triple.object, bound)
        }
    }
}

/// The admission failure for a chain with no feasible total order, naming the stuck
/// atoms, the positions they cannot fill, and the modes they declare.
fn stuck(
    remaining: &[Atom<'_>],
    relations: Option<&PropertyFunctionRegistry>,
    bound: &DetHashSet<Variable>,
) -> EvalError {
    let mut described: Vec<String> = Vec::new();
    for atom in remaining {
        let Some(call) = atom.call else {
            continue;
        };
        let mode = invocation_mode(call, bound);
        let free: Vec<String> = mode
            .code()
            .char_indices()
            .filter(|&(_, code)| code == 'f')
            .map(|(position, _)| position.to_string())
            .collect();
        let declared: Vec<String> = relations
            .and_then(|registry| registry.resolve(&call.iri))
            .map(|relation| relation.modes().iter().map(|mode| mode.code()).collect())
            .unwrap_or_default();
        described.push(format!(
            "<{}> reachable only as `{}` (free position(s) {}), declaring [{}]",
            call.iri,
            mode.code(),
            if free.is_empty() {
                "none".to_owned()
            } else {
                free.join(", ")
            },
            declared.join(", ")
        ));
    }
    EvalError::function(format!(
        "no feasible evaluation order exists for this group's property-function call(s): {}",
        described.join("; ")
    ))
}

/// Resolve a call's IRI, or report the admission failure that an unregistered IRI is.
///
/// An absent registry is the same failure: the parser mints a call node only under a
/// caller-configured namespace, so a call with nothing to resolve against is a host
/// configuration that names a relation it never supplied — never a silently empty one.
fn resolve<'r>(
    call: &PropertyFunctionCall,
    relations: Option<&'r PropertyFunctionRegistry>,
) -> Result<&'r std::sync::Arc<dyn crate::property_fn::PropertyFunction>, EvalError> {
    relations
        .and_then(|registry| registry.resolve(&call.iri))
        .ok_or_else(|| {
            EvalError::function(format!(
                "no property function is registered for <{}>",
                call.iri
            ))
        })
}

/// Check a call site's argument counts against the relation's declaration.
fn check_arity(call: &PropertyFunctionCall, declared: PfArity) -> Result<(), EvalError> {
    let supplied = PfArity::new(call.subject_args.len(), call.object_args.len());
    if declared == supplied {
        return Ok(());
    }
    Err(EvalError::function(format!(
        "property function <{}> is declared with {declared} argument(s); the call site supplies \
         {supplied}",
        call.iri
    )))
}

// ---------------------------------------------------------------------------
// Structural recursion
// ---------------------------------------------------------------------------

/// Rewrite every child of a non-chain node, threading the variables each child's left
/// siblings certainly bind.
fn map_children(
    pattern: &GraphPattern,
    relations: Option<&PropertyFunctionRegistry>,
    outer: &DetHashSet<Variable>,
) -> Result<GraphPattern, EvalError> {
    let recurse = |child: &GraphPattern, outer: &DetHashSet<Variable>| {
        plan_pattern(child, relations, outer).map(Box::new)
    };
    Ok(match pattern {
        GraphPattern::Bgp { .. }
        | GraphPattern::Path { .. }
        | GraphPattern::Values { .. }
        | GraphPattern::PropertyFunction(_) => pattern.clone(),
        // The right side of a join sees what the left side certainly binds; a `Lateral`
        // makes that correlation explicit, and an ordinary `Join` is evaluated with the
        // same left-to-right binding availability.
        GraphPattern::Join { left, right } => {
            let mut inner = outer.clone();
            collect_certainly_bound(left, &mut inner);
            GraphPattern::Join {
                left: recurse(left, outer)?,
                right: recurse(right, &inner)?,
            }
        }
        GraphPattern::Lateral { left, right } => {
            let mut inner = outer.clone();
            collect_certainly_bound(left, &mut inner);
            GraphPattern::Lateral {
                left: recurse(left, outer)?,
                right: recurse(right, &inner)?,
            }
        }
        // `OPTIONAL`'s right side and `MINUS`'s right side are evaluated against the
        // left, but their own bindings do NOT escape as certain, which
        // `certainly_bound` accounts for; the correlation INTO them is still real.
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => {
            let mut inner = outer.clone();
            collect_certainly_bound(left, &mut inner);
            // The inline condition is evaluated only on candidate JOINED rows, so
            // both sides' bindings are available to it.
            let mut condition_scope = inner.clone();
            collect_certainly_bound(right, &mut condition_scope);
            GraphPattern::LeftJoin {
                left: recurse(left, outer)?,
                right: recurse(right, &inner)?,
                expression: expression
                    .as_ref()
                    .map(|expr| plan_expression(expr, relations, &condition_scope))
                    .transpose()?,
            }
        }
        GraphPattern::Minus { left, right } => GraphPattern::Minus {
            left: recurse(left, outer)?,
            right: recurse(right, outer)?,
        },
        // A `UNION` branch cannot rely on its sibling.
        GraphPattern::Union { left, right } => GraphPattern::Union {
            left: recurse(left, outer)?,
            right: recurse(right, outer)?,
        },
        // A `FILTER`'s expression is evaluated over the rows its inner pattern
        // produced, so an `EXISTS` inside it sees everything that pattern certainly
        // binds — which is exactly what makes a relation inside a correlated `EXISTS`
        // invocable with the outer row's values.
        GraphPattern::Filter { expr, inner } => {
            let mut scope = outer.clone();
            collect_certainly_bound(inner, &mut scope);
            GraphPattern::Filter {
                expr: plan_expression(expr, relations, &scope)?,
                inner: recurse(inner, outer)?,
            }
        }
        GraphPattern::Extend {
            inner,
            variable,
            expression,
        } => {
            let mut scope = outer.clone();
            collect_certainly_bound(inner, &mut scope);
            GraphPattern::Extend {
                inner: recurse(inner, outer)?,
                variable: variable.clone(),
                expression: plan_expression(expression, relations, &scope)?,
            }
        }
        GraphPattern::Graph { name, inner } => GraphPattern::Graph {
            name: name.clone(),
            inner: recurse(inner, outer)?,
        },
        GraphPattern::OrderBy { inner, expression } => {
            let mut scope = outer.clone();
            collect_certainly_bound(inner, &mut scope);
            GraphPattern::OrderBy {
                inner: recurse(inner, outer)?,
                expression: expression
                    .iter()
                    .map(|order| {
                        Ok(match order {
                            OrderExpression::Asc(expr) => {
                                OrderExpression::Asc(plan_expression(expr, relations, &scope)?)
                            }
                            OrderExpression::Desc(expr) => {
                                OrderExpression::Desc(plan_expression(expr, relations, &scope)?)
                            }
                        })
                    })
                    .collect::<Result<Vec<_>, EvalError>>()?,
            }
        }
        // A sub-`SELECT` is its own scope: a variable bound outside it is not visible
        // inside, so the correlation set is emptied on the way in.
        GraphPattern::Project { inner, variables } => GraphPattern::Project {
            inner: recurse(inner, &DetHashSet::default())?,
            variables: variables.clone(),
        },
        GraphPattern::Distinct { inner } => GraphPattern::Distinct {
            inner: recurse(inner, outer)?,
        },
        GraphPattern::Reduced { inner } => GraphPattern::Reduced {
            inner: recurse(inner, outer)?,
        },
        GraphPattern::Slice {
            inner,
            start,
            length,
        } => GraphPattern::Slice {
            inner: recurse(inner, outer)?,
            start: *start,
            length: *length,
        },
        GraphPattern::Group {
            inner,
            variables,
            aggregates,
        } => {
            let mut scope = outer.clone();
            collect_certainly_bound(inner, &mut scope);
            GraphPattern::Group {
                inner: recurse(inner, outer)?,
                variables: variables.clone(),
                aggregates: aggregates
                    .iter()
                    .map(|(variable, aggregate)| {
                        Ok((
                            variable.clone(),
                            plan_aggregate(aggregate, relations, &scope)?,
                        ))
                    })
                    .collect::<Result<Vec<_>, EvalError>>()?,
            }
        }
        // A `SERVICE` body is forwarded to a remote endpoint rather than evaluated
        // here, and `crate::remote` refuses to forward a call at all — so its body is
        // left exactly as written.
        GraphPattern::Service {
            name,
            inner,
            silent,
        } => GraphPattern::Service {
            name: name.clone(),
            inner: inner.clone(),
            silent: *silent,
        },
    })
}

/// Rewrite the patterns embedded in an expression (an `EXISTS`, recursively).
fn plan_expression(
    expr: &Expression,
    relations: Option<&PropertyFunctionRegistry>,
    outer: &DetHashSet<Variable>,
) -> Result<Expression, EvalError> {
    if !crate::property_fn_eval::expression_reaches_property_function(expr) {
        return Ok(expr.clone());
    }
    let sub = |expr: &Expression| plan_expression(expr, relations, outer).map(Box::new);
    Ok(match expr {
        // A correlated `EXISTS` sees its enclosing group's bindings, so `outer` carries
        // straight in: that is what lets a relation inside one be invoked bound.
        Expression::Exists(pattern) => {
            Expression::Exists(Box::new(plan_pattern(pattern, relations, outer)?))
        }
        Expression::Or(a, b) => Expression::Or(sub(a)?, sub(b)?),
        Expression::And(a, b) => Expression::And(sub(a)?, sub(b)?),
        Expression::Equal(a, b) => Expression::Equal(sub(a)?, sub(b)?),
        Expression::SameTerm(a, b) => Expression::SameTerm(sub(a)?, sub(b)?),
        Expression::Greater(a, b) => Expression::Greater(sub(a)?, sub(b)?),
        Expression::GreaterOrEqual(a, b) => Expression::GreaterOrEqual(sub(a)?, sub(b)?),
        Expression::Less(a, b) => Expression::Less(sub(a)?, sub(b)?),
        Expression::LessOrEqual(a, b) => Expression::LessOrEqual(sub(a)?, sub(b)?),
        Expression::Add(a, b) => Expression::Add(sub(a)?, sub(b)?),
        Expression::Subtract(a, b) => Expression::Subtract(sub(a)?, sub(b)?),
        Expression::Multiply(a, b) => Expression::Multiply(sub(a)?, sub(b)?),
        Expression::Divide(a, b) => Expression::Divide(sub(a)?, sub(b)?),
        Expression::UnaryPlus(a) => Expression::UnaryPlus(sub(a)?),
        Expression::UnaryMinus(a) => Expression::UnaryMinus(sub(a)?),
        Expression::Not(a) => Expression::Not(sub(a)?),
        Expression::If(c, t, e) => Expression::If(sub(c)?, sub(t)?, sub(e)?),
        Expression::In(needle, haystack) => Expression::In(
            sub(needle)?,
            haystack
                .iter()
                .map(|item| plan_expression(item, relations, outer))
                .collect::<Result<Vec<_>, EvalError>>()?,
        ),
        Expression::Coalesce(items) => Expression::Coalesce(
            items
                .iter()
                .map(|item| plan_expression(item, relations, outer))
                .collect::<Result<Vec<_>, EvalError>>()?,
        ),
        Expression::FunctionCall(function, args) => Expression::FunctionCall(
            function.clone(),
            args.iter()
                .map(|arg| plan_expression(arg, relations, outer))
                .collect::<Result<Vec<_>, EvalError>>()?,
        ),
        Expression::NamedNode(_)
        | Expression::Literal(_)
        | Expression::Variable(_)
        | Expression::Bound(_) => expr.clone(),
    })
}

/// Rewrite the expression an aggregate reduces over.
fn plan_aggregate(
    aggregate: &AggregateExpression,
    relations: Option<&PropertyFunctionRegistry>,
    outer: &DetHashSet<Variable>,
) -> Result<AggregateExpression, EvalError> {
    Ok(match aggregate {
        AggregateExpression::CountStar { distinct } => AggregateExpression::CountStar {
            distinct: *distinct,
        },
        AggregateExpression::FunctionCall {
            function,
            expression,
            distinct,
        } => AggregateExpression::FunctionCall {
            function: function.clone(),
            expression: Box::new(plan_expression(expression, relations, outer)?),
            distinct: *distinct,
        },
    })
}

// ---------------------------------------------------------------------------
// Certainly-bound variables
// ---------------------------------------------------------------------------

/// Add to `out` every variable `pattern` binds in **every** solution it produces.
///
/// This is deliberately narrower than a scope walk. A variable that is merely *in
/// scope* may still be unbound in a given row (`OPTIONAL`'s right side, a `UNION` branch
/// that does not mention it, a `BIND` whose expression errored), and treating one of
/// those as bound would let this pass admit an invocation the evaluator then cannot
/// make. Erring the other way is harmless: at worst a feasible order is missed and the
/// query is refused at prepare time with a message that names exactly what could not be
/// bound.
pub(crate) fn collect_certainly_bound(pattern: &GraphPattern, out: &mut DetHashSet<Variable>) {
    match pattern {
        GraphPattern::Bgp { patterns } => {
            for triple in patterns {
                collect_triple_vars(triple, out);
            }
        }
        GraphPattern::Path {
            subject,
            path: _,
            object,
        } => {
            collect_term_vars(subject, out);
            collect_term_vars(object, out);
        }
        // Every flattened argument position of a call receives a value on every row it
        // emits, so its variables are certainly bound by it.
        GraphPattern::PropertyFunction(call) => {
            for term in call.subject_args.iter().chain(&call.object_args) {
                collect_term_vars(term, out);
            }
        }
        GraphPattern::Join { left, right } | GraphPattern::Lateral { left, right } => {
            collect_certainly_bound(left, out);
            collect_certainly_bound(right, out);
        }
        // The right side may contribute nothing to a row.
        GraphPattern::LeftJoin { left, .. } | GraphPattern::Minus { left, right: _ } => {
            collect_certainly_bound(left, out);
        }
        // Only what BOTH branches bind is bound in every row.
        GraphPattern::Union { left, right } => {
            let mut l = DetHashSet::default();
            let mut r = DetHashSet::default();
            collect_certainly_bound(left, &mut l);
            collect_certainly_bound(right, &mut r);
            out.extend(l.intersection(&r).cloned());
        }
        GraphPattern::Filter { expr: _, inner }
        | GraphPattern::OrderBy {
            inner,
            expression: _,
        }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. }
        // A `BIND`'s own variable is NOT certain: an expression that errors leaves it
        // unbound (SPARQL 1.1 §18.6), so only the inner pattern's bindings carry.
        | GraphPattern::Extend { inner, .. } => collect_certainly_bound(inner, out),
        GraphPattern::Graph { name, inner } => {
            if let NamedNodePattern::Variable(variable) = name {
                out.insert(variable.clone());
            }
            collect_certainly_bound(inner, out);
        }
        // Only what the projection keeps escapes, and only if the inner pattern bound
        // it certainly.
        GraphPattern::Project { inner, variables } => {
            let mut inner_bound = DetHashSet::default();
            collect_certainly_bound(inner, &mut inner_bound);
            out.extend(
                variables
                    .iter()
                    .filter(|variable| inner_bound.contains(*variable))
                    .cloned(),
            );
        }
        // A grouping key is bound in every group row; an aggregate's output may not be
        // (an empty group's MIN is unbound).
        GraphPattern::Group {
            inner: _,
            variables,
            aggregates: _,
        } => out.extend(variables.iter().cloned()),
        // A `VALUES` cell may be UNDEF, and a remote endpoint may omit a column, so
        // neither promises anything.
        GraphPattern::Values { .. } | GraphPattern::Service { .. } => {}
    }
}

/// Add a triple pattern's variables (recursing through quoted triples).
fn collect_triple_vars(triple: &TriplePattern, out: &mut DetHashSet<Variable>) {
    collect_term_vars(&triple.subject, out);
    if let NamedNodePattern::Variable(variable) = &triple.predicate {
        out.insert(variable.clone());
    }
    collect_term_vars(&triple.object, out);
}

/// Add a term position's variables (recursing through quoted triples).
fn collect_term_vars(term: &TermPattern, out: &mut DetHashSet<Variable>) {
    match term {
        TermPattern::Variable(variable) => {
            out.insert(variable.clone());
        }
        TermPattern::Triple(triple) => collect_triple_vars(triple, out),
        TermPattern::NamedNode(_) | TermPattern::BlankNode(_) | TermPattern::Literal(_) => {}
    }
}

// ---------------------------------------------------------------------------
// The registry fingerprint
// ---------------------------------------------------------------------------

/// A deterministic fingerprint of everything about `relations` that can change the
/// plan this module produces: every registered IRI, its arity, and its declared modes
/// with their row bounds, IRI-sorted.
///
/// This belongs in the plan cache's key. The rewrite above is a function of the query
/// text AND the registry's declarations, so two differently-configured registries can
/// order the same text differently — and a cache keyed on the text alone would hand the
/// second host the first host's plan. Derived from
/// [`PropertyFunctionRegistry::describe`], which is already IRI-sorted, so the
/// fingerprint is a pure function of the registry's contents rather than of its
/// construction order.
pub(crate) fn registry_fingerprint(relations: Option<&PropertyFunctionRegistry>) -> String {
    let Some(registry) = relations.filter(|registry| !registry.is_empty()) else {
        return String::new();
    };
    let mut out = String::new();
    for descriptor in registry.describe() {
        out.push_str(&descriptor.iri);
        out.push('\u{2}');
        out.push_str(&descriptor.subject_arity.to_string());
        out.push(',');
        out.push_str(&descriptor.object_arity.to_string());
        for mode in &descriptor.modes {
            out.push('\u{3}');
            out.push_str(&mode.code);
            out.push(':');
            out.push_str(&mode.rows_per_invocation.to_string());
        }
        out.push('\u{4}');
    }
    out
}
