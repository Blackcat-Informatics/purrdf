// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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

use purrdf_core::ContentDigest;
use purrdf_core::binding_pattern::BindingPattern;
use purrdf_sparql_algebra::{
    AggregateExpression, AggregateFunction, Expression, GraphPattern, Literal, NamedNodePattern,
    OrderExpression, PropertyFunctionCall, Query, TermPattern, TriplePattern, Variable,
};

use crate::DetHashSet;
use crate::agg_fn::{AggregateRegistry, ScalarvalKind, ScalarvalSpec};
use crate::convert::literal_to_value;
use crate::engine::ShaclPrebinding;
use crate::error::EvalError;
use crate::expr::xsd_of;
use crate::modifier::is_numeric_xsd;
use crate::property_fn::{NOT_RANKED_CANONICAL, PfArity, PropertyFunctionRegistry};
use crate::registry_id::append_framed_part;

/// Which admission seam a [`plan_query`]/[`plan_where_pattern`] failure came from.
///
/// The planner admits two independent hazards in the same walk — a property-function
/// call and a custom-aggregate call — and they are reported under two different
/// diagnostic codes. A caller that reduced both to one opaque error (as this module
/// used to) could only guess which contract to check a failure against; this is what a
/// planner failure carries instead, so `crate::engine` and `crate::update`'s two
/// admission call sites read it rather than hard-coding the property-function code for
/// every failure this module can raise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PlanSeam {
    /// A property-function call could not be admitted: an unregistered predicate IRI,
    /// an arity mismatch between the call site and the relation's declaration, or a
    /// chain no relation's declared modes can serve.
    PropertyFunction,
    /// A custom-aggregate call could not be admitted: an unregistered
    /// `AggregateFunction::Custom` IRI, or a positional-argument count its registered
    /// entry does not declare.
    Aggregate,
}

/// A [`plan_query`]/[`plan_where_pattern`] admission failure, tagged with the
/// [`PlanSeam`] that raised it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlanError {
    pub(crate) seam: PlanSeam,
    pub(crate) error: EvalError,
}

impl PlanError {
    /// Tag `error` as a property-function admission failure.
    fn property_function(error: EvalError) -> Self {
        Self {
            seam: PlanSeam::PropertyFunction,
            error,
        }
    }

    /// Tag `error` as a custom-aggregate admission failure.
    fn aggregate(error: EvalError) -> Self {
        Self {
            seam: PlanSeam::Aggregate,
            error,
        }
    }

    /// The diagnostic code this failure must be reported under. The single chokepoint
    /// both `crate::engine::PlanCache::prepare_with_relations` and
    /// `crate::update::delete_insert` read, so the two admission call sites can never
    /// again drift into hard-coding the wrong seam's code.
    pub(crate) const fn diagnostic_code(&self) -> &'static str {
        match self.seam {
            PlanSeam::PropertyFunction => "native-sparql-property-function",
            PlanSeam::Aggregate => "native-sparql-aggregate-function",
        }
    }
}

impl core::fmt::Display for PlanError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&self.error, f)
    }
}

/// The promised-parameter set [`plan_query`] and [`plan_where_pattern`] take, built
/// from the names a prepare declared.
///
/// Derived here rather than at each caller so "a declared parameter" has one spelling:
/// the name without its sigil, exactly as
/// [`NativeSparqlEngine::prepare_execution`](crate::NativeSparqlEngine::prepare_execution)
/// declares it and `crate::substitute` binds it.
pub(crate) fn parameter_set(names: &[&str]) -> DetHashSet<Variable> {
    names.iter().map(|name| Variable::new(*name)).collect()
}

/// Rewrite every property-function chain in `query` into a feasible order.
///
/// The query is returned unchanged — and no work is done at all — when it carries no
/// call node, which is every query on a host that has not configured the seam.
///
/// `parameters` are the variables an execution has PROMISED to supply through the
/// substitution channel. They are treated as bound where the substitution really binds
/// them — see [`plan_where_pattern`] — so a call whose argument is one of them is
/// admitted in the access pattern it will actually be invoked in rather than in the
/// all-free one the text alone shows. An empty set is the ordinary prepare, where
/// nothing is promised and nothing is assumed.
///
/// # Errors
///
/// A [`PlanError`] tagged [`PlanSeam::PropertyFunction`] for an unregistered predicate
/// IRI, an arity mismatch between the call site and the relation's declaration, or a
/// chain no total order can serve. The same class of refusal, tagged
/// [`PlanSeam::Aggregate`], for an `AggregateFunction::Custom(iri)` reached anywhere in
/// `query`: an unregistered IRI, or a positional-argument count `agg_registry`'s
/// registered entry does not declare — refused HERE, at prepare time, before any governor
/// charge (see [`plan_aggregate`]).
pub(crate) fn plan_query(
    query: &Query,
    relations: &PropertyFunctionRegistry,
    agg_registry: &AggregateRegistry,
    parameters: &DetHashSet<Variable>,
    reach: ShaclPrebinding,
) -> Result<Option<Query>, PlanError> {
    let pattern = match query {
        Query::Select { pattern, .. }
        | Query::Ask { pattern, .. }
        | Query::Construct { pattern, .. }
        | Query::Describe { pattern, .. } => pattern,
    };
    let Some(planned) = plan_where_pattern(pattern, relations, agg_registry, parameters, reach)?
    else {
        return Ok(None);
    };
    let mut planned_query = query.clone();
    match &mut planned_query {
        Query::Select { pattern, .. }
        | Query::Ask { pattern, .. }
        | Query::Construct { pattern, .. }
        | Query::Describe { pattern, .. } => *pattern = planned,
    }
    Ok(Some(planned_query))
}

/// [`plan_query`] on a standalone [`GraphPattern`] rather than a full [`Query`] — the
/// entry an UPDATE's `WHERE` clause uses, because a `DELETE`/`INSERT … WHERE`
/// operation has no `Query` wrapper to hand in. Same admission (unregistered IRI,
/// arity mismatch, an infeasible chain), same feasibility rewrite, same "untouched,
/// no work at all, when the pattern carries no call node" contract — an UPDATE
/// WHERE is a triple-pattern context exactly like a query's, so it is planned
/// exactly like one.
///
/// Returns `Ok(None)` unchanged (not merely equal) when `pattern` carries no call
/// node, so a caller can keep evaluating its own borrowed `pattern` rather than a
/// clone that happens to match it.
///
/// # Where a promised parameter counts as bound
///
/// `parameters` names the variables a prepared execution binds on every run, and each
/// counts as bound exactly where a run's rewrite really binds it — see [`Promise`] for
/// the two places that is under the ordinary rewrite, and why a call anywhere else is
/// admitted as though the parameter were free. `reach` names the rewrite every run
/// applies: under [`ShaclPrebinding::Applied`] a parameter reaches every
/// property-function call in the query ([`Promise::Everywhere`]).
///
/// # Errors
///
/// A [`PlanError`] tagged [`PlanSeam::PropertyFunction`] for an unregistered predicate
/// IRI, an arity mismatch between the call site and the relation's declaration, or a
/// chain no total order can serve. The same class of refusal, tagged
/// [`PlanSeam::Aggregate`], for a reachable `AggregateFunction::Custom` — see
/// [`plan_query`]'s doc.
pub(crate) fn plan_where_pattern(
    pattern: &GraphPattern,
    relations: &PropertyFunctionRegistry,
    agg_registry: &AggregateRegistry,
    parameters: &DetHashSet<Variable>,
    reach: ShaclPrebinding,
) -> Result<Option<GraphPattern>, PlanError> {
    // Either hazard alone must still run the walk: a query with a `Custom`
    // aggregate and no property-function call would otherwise skip this pass
    // entirely on the property-function-only check, and its admission (below,
    // via `plan_aggregate`) would never happen.
    if !crate::property_fn_eval::pattern_needs_admission(pattern) {
        return Ok(None);
    }
    // The rewriting walk is recursive just like evaluation. Apply its existing
    // execution envelope before cloning or traversing an admitted call chain.
    crate::governor::soundness::validate_graph_pattern_depth(pattern)
        .map_err(PlanError::property_function)?;
    plan_pattern(
        pattern,
        relations,
        agg_registry,
        &DetHashSet::default(),
        Promise::descent(parameters, reach),
    )
    .map(Some)
}

/// Where a prepared execution's declared parameters count as bound while a call is
/// admitted, and where they do not.
///
/// A run binds its parameters in two ways at once (`crate::substitute::apply_probes`),
/// and a parameter is honestly "bound" only where one of them reaches:
///
/// * **The seed.** A single-row `VALUES` is joined onto the core `WHERE` pattern —
///   the first node beneath the solution-modifier wrappers `Query::map_core_pattern`
///   descends. Every row a wrapper ABOVE the core sees therefore carries the
///   parameters, so an expression there (a correlated `EXISTS`, say) sees them bound.
///   That is [`Self::Descent`]. The seed is joined, not correlated: nothing at or
///   below the core is evaluated with its row in hand.
/// * **The pushdown.** At and below the core, the rewrite writes each parameter's
///   value INTO the leaves it can reach — triple patterns, paths, and
///   property-function calls — so a call there is invoked with that argument bound.
///   That is [`Self::Pushed`], and it follows the pushdown's own reach exactly: into
///   both operands of a `Join` and a `UNION`, the inner of a `GRAPH`, `FILTER` and
///   `BIND`, the left operand of an `OPTIONAL`, a `MINUS` and a `LATERAL`, and the
///   call a `LATERAL` drives. Everywhere else — an `OPTIONAL`'s or a `MINUS`'s right
///   arm, the body of an `EXISTS` beneath the core, a nested sub-`SELECT` — the
///   pushdown does not write, and a call there is invoked with the parameter free.
///   Admitting it as though it were bound would exchange a refusal at prepare for a
///   refusal on every run.
///
/// A run under the SHACL pre-binding rewrite reaches further: it binds a parameter in
/// every call in the query, and that is [`Self::Everywhere`].
///
/// Neither half extends `outer`, which stays the set of variables the pattern ITSELF
/// certainly binds: a promise is consulted only where a call is admitted, and only
/// where it holds.
#[derive(Clone, Copy, Debug)]
enum Promise<'a> {
    /// Nothing is promised here.
    None,
    /// On the solution-modifier descent to the core, which the seed will be joined
    /// beneath.
    Descent(&'a DetHashSet<Variable>),
    /// At or below the core, at a position the pushdown writes these into.
    Pushed(&'a DetHashSet<Variable>),
    /// Anywhere in the query, because the run applies the SHACL pre-binding rewrite.
    ///
    /// That rewrite (`crate::substitute::apply_shacl_probes`) is the pushdown and the
    /// seed PLUS a walk with no boundary: it writes an IRI or literal parameter into
    /// every property-function argument that names it — an `OPTIONAL`'s or a
    /// `MINUS`'s right arm, a sub-`SELECT` that does not project it, an `EXISTS` body
    /// below the core — and drives every other value (a blank node, a quoted triple)
    /// into the same calls through a one-row `VALUES`. So under that rewrite a call
    /// ANYWHERE receives the parameter bound, and admitting it as free would refuse,
    /// at prepare, a call every run would have served.
    ///
    /// Only sound behind an execution that refuses to run under the ordinary
    /// rewrite, whose reach is narrower — which
    /// [`PreparedExecution`](crate::PreparedExecution) does for a plan admitted
    /// under this promise.
    Everywhere(&'a DetHashSet<Variable>),
}

impl<'a> Promise<'a> {
    /// The promise a plan starts from: `parameters` on the descent, or everywhere
    /// when the run applies the SHACL pre-binding rewrite, or nothing.
    fn descent(parameters: &'a DetHashSet<Variable>, reach: ShaclPrebinding) -> Self {
        if parameters.is_empty() {
            Self::None
        } else {
            match reach {
                ShaclPrebinding::Applied => Self::Everywhere(parameters),
                ShaclPrebinding::None => Self::Descent(parameters),
            }
        }
    }

    /// The promise at a position the pushdown does not write into: nothing, unless
    /// the SHACL pre-binding rewrite reaches it anyway.
    const fn beyond_pushdown(self) -> Self {
        match self {
            Self::Everywhere(_) => self,
            Self::None | Self::Descent(_) | Self::Pushed(_) => Self::None,
        }
    }

    /// The promise at a node that is not a solution-modifier wrapper — which, if the
    /// descent is still under way, makes this node the core the seed is joined onto
    /// and the pushdown starts from.
    const fn at_node(self) -> Self {
        match self {
            Self::Descent(parameters) => Self::Pushed(parameters),
            other => other,
        }
    }

    /// The parameters a row seen by an expression at this node carries, which is the
    /// seed's on the descent and nothing anywhere else.
    const fn in_rows(self) -> Option<&'a DetHashSet<Variable>> {
        match self {
            Self::Descent(parameters) => Some(parameters),
            Self::None | Self::Pushed(_) | Self::Everywhere(_) => None,
        }
    }

    /// The parameters a call admitted at this node may count as bound.
    const fn for_calls(self) -> Option<&'a DetHashSet<Variable>> {
        match self {
            Self::Pushed(parameters) | Self::Everywhere(parameters) => Some(parameters),
            Self::None | Self::Descent(_) => None,
        }
    }
}

/// `bound`, widened by the parameters `promise` lets a call count as bound here.
///
/// Borrowed whenever there is nothing to add, so an ordinary prepare — which promises
/// nothing — builds no set.
fn call_scope<'s>(
    bound: &'s DetHashSet<Variable>,
    promise: Promise<'_>,
) -> std::borrow::Cow<'s, DetHashSet<Variable>> {
    match promise.for_calls() {
        Some(parameters) if !parameters.iter().all(|variable| bound.contains(variable)) => {
            let mut widened = bound.clone();
            widened.extend(parameters.iter().cloned());
            std::borrow::Cow::Owned(widened)
        }
        _ => std::borrow::Cow::Borrowed(bound),
    }
}

/// `scope`, widened by the parameters rows at this node carry — see
/// [`Promise::in_rows`].
fn with_rows(scope: &mut DetHashSet<Variable>, promise: Promise<'_>) {
    if let Some(parameters) = promise.in_rows() {
        scope.extend(parameters.iter().cloned());
    }
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
///
/// `promise` is where a prepared execution's declared parameters count as bound — see
/// [`Promise`].
fn plan_pattern(
    pattern: &GraphPattern,
    relations: &PropertyFunctionRegistry,
    agg_registry: &AggregateRegistry,
    outer: &DetHashSet<Variable>,
    promise: Promise<'_>,
) -> Result<GraphPattern, PlanError> {
    // Compiler-produced algebra may be a bare call, without the parser's Lateral
    // wrapper. Apply the same admission as a chain member before cloning it.
    if let GraphPattern::PropertyFunction(call) = pattern {
        let scope = call_scope(outer, promise.at_node());
        if admitted_row_bound(call, relations, &scope)?.is_none() {
            return Err(stuck(
                &[Atom {
                    pattern,
                    call: Some(call),
                    position: 0,
                }],
                relations,
                &scope,
            ));
        }
        return Ok(pattern.clone());
    }
    // A chain is a left-deep spine of `Lateral`s (a call's join) and `Join`s (the
    // residual data written between two calls), which is exactly the shape the parser
    // assembles a triples block containing calls into. Anything else recurses
    // structurally.
    let mut atoms = Vec::new();
    if collect_chain(pattern, &mut atoms) && atoms.iter().any(|atom| atom.call.is_some()) {
        return order_chain(atoms, relations, agg_registry, outer, promise.at_node());
    }
    map_children(pattern, relations, agg_registry, outer, promise)
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
///
/// A call is admitted with `promise`'s parameters counted as bound too: every atom of
/// a chain is a place the pushdown writes into — the first atom and each `Join`
/// operand, and each call a `Lateral` drives — so they are handed on to every atom.
fn order_chain(
    atoms: Vec<Atom<'_>>,
    relations: &PropertyFunctionRegistry,
    agg_registry: &AggregateRegistry,
    outer: &DetHashSet<Variable>,
    promise: Promise<'_>,
) -> Result<GraphPattern, PlanError> {
    let mut bound = outer.clone();
    let mut remaining: Vec<Atom<'_>> = atoms;
    let mut ordered: Vec<&GraphPattern> = Vec::with_capacity(remaining.len());
    let mut is_call: Vec<bool> = Vec::with_capacity(remaining.len());
    // The set `bound` held at the moment each atom was CHOSEN — i.e. exactly the
    // variables a CALL atom is driven with when it is re-admitted below. Capturing it
    // here (rather than reusing the fully-accumulated `bound` after the loop) is what
    // keeps a call from being admitted as though sibling atoms chosen AFTER it — and
    // everything they bind — were already in scope.
    let mut bound_before: Vec<DetHashSet<Variable>> = Vec::with_capacity(remaining.len());

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
            let Some(rows_bound) =
                admitted_row_bound(call, relations, &call_scope(&bound, promise))?
            else {
                continue;
            };
            let key = (rows_bound, call.iri.as_str(), atom.position);
            if best.is_none_or(|(_, current)| key < current) {
                best = Some((index, key));
            }
        }
        let Some((index, _)) = best else {
            return Err(stuck(&remaining, relations, &call_scope(&bound, promise)));
        };
        let atom = remaining.remove(index);
        bound_before.push(bound.clone());
        collect_certainly_bound(atom.pattern, &mut bound);
        ordered.push(atom.pattern);
        is_call.push(atom.call.is_some());
    }

    // Rebuild the left-deep spine in the chosen order: a call re-attaches through a
    // `Lateral` (it depends on what is to its left), everything else through a `Join`.
    let mut chain: Option<GraphPattern> = None;
    for ((pattern, call), scope) in ordered.into_iter().zip(is_call).zip(bound_before) {
        // A call is re-attached through a `Lateral`, which drives it with the rows of
        // every atom before it, so it is admitted against `scope`. Any other atom is
        // re-attached through a `Join`, which evaluates it on its own — so a call
        // NESTED inside it (a `UNION` arm's own chain, say) sees only what the chain's
        // enclosing context binds.
        let planned = plan_pattern(
            pattern,
            relations,
            agg_registry,
            if call { &scope } else { outer },
            promise,
        )?;
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
    relations: &PropertyFunctionRegistry,
    bound: &DetHashSet<Variable>,
) -> PlanError {
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
        // Best-effort: this is already an admission failure being reported, so a
        // relation whose `modes` ALSO panics degrades the diagnostic to an empty
        // declared-modes list rather than losing the admission failure itself.
        let declared: Vec<String> = relations
            .resolve(&call.iri)
            .and_then(|relation| {
                crate::property_fn::declaration_contained(&call.iri, "declared modes", || {
                    relation
                        .modes()
                        .iter()
                        .copied()
                        .map(BindingPattern::code)
                        .collect::<Vec<_>>()
                })
                .ok()
            })
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
    PlanError::property_function(EvalError::function(format!(
        "no feasible evaluation order exists for this group's property-function call(s): {}",
        described.join("; ")
    )))
}

/// Resolve a call's IRI, or report the admission failure that an unregistered IRI is.
///
/// An EMPTY registry is the same failure: the parser mints a call node only under a
/// caller-configured namespace, so a call with nothing to resolve against is a host
/// configuration that names a relation it never supplied — never a silently empty one.
fn resolve<'r>(
    call: &PropertyFunctionCall,
    relations: &'r PropertyFunctionRegistry,
) -> Result<&'r std::sync::Arc<dyn crate::property_fn::PropertyFunction>, PlanError> {
    relations.resolve(&call.iri).ok_or_else(|| {
        PlanError::property_function(EvalError::function(format!(
            "no property function is registered for <{}>",
            call.iri
        )))
    })
}

/// Resolve and admit a call under its actual lexical binding scope. `None` means
/// its access mode is infeasible; a chain may first bind more variables.
fn admitted_row_bound(
    call: &PropertyFunctionCall,
    relations: &PropertyFunctionRegistry,
    bound: &DetHashSet<Variable>,
) -> Result<Option<u64>, PlanError> {
    let relation = resolve(call, relations)?;
    let arity = crate::property_fn::declaration_contained(&call.iri, "arity", || relation.arity())
        .map_err(PlanError::property_function)?;
    check_arity(call, arity)?;
    let mode = invocation_mode(call, bound);
    let admitted = crate::property_fn::declaration_contained(&call.iri, "declared modes", || {
        relation
            .modes()
            .iter()
            .any(|declared| declared.subsumes(mode))
    })
    .map_err(PlanError::property_function)?;
    if !admitted {
        return Ok(None);
    }
    crate::property_fn::declaration_contained(&call.iri, "row bound", || {
        relation.rows_per_invocation(mode)
    })
    .map(Some)
    .map_err(PlanError::property_function)
}

/// Check a call site's argument counts against the relation's declaration.
fn check_arity(call: &PropertyFunctionCall, declared: PfArity) -> Result<(), PlanError> {
    let supplied = PfArity::new(call.subject_args.len(), call.object_args.len());
    if declared == supplied {
        return Ok(());
    }
    Err(PlanError::property_function(EvalError::function(format!(
        "property function <{}> is declared with {declared} argument(s); the call site supplies \
         {supplied}",
        call.iri
    ))))
}

// ---------------------------------------------------------------------------
// Structural recursion
// ---------------------------------------------------------------------------

/// Rewrite every child of a non-chain node, threading the variables each child's left
/// siblings certainly bind.
fn map_children(
    pattern: &GraphPattern,
    relations: &PropertyFunctionRegistry,
    agg_registry: &AggregateRegistry,
    outer: &DetHashSet<Variable>,
    promise: Promise<'_>,
) -> Result<GraphPattern, PlanError> {
    let recurse = |child: &GraphPattern, outer: &DetHashSet<Variable>, promise: Promise<'_>| {
        plan_pattern(child, relations, agg_registry, outer, promise).map(Box::new)
    };
    // A solution-modifier wrapper passes the descent on to its inner pattern; beneath
    // the core, only the wrappers the pushdown descends pass on what it writes. See
    // [`Promise`].
    let wrapped = match promise {
        Promise::Descent(_) | Promise::Everywhere(_) => promise,
        Promise::None | Promise::Pushed(_) => Promise::None,
    };
    // The node itself, when it is not a wrapper: the core if the descent is still
    // under way.
    let here = promise.at_node();
    Ok(match pattern {
        GraphPattern::Bgp { .. }
        | GraphPattern::Path { .. }
        | GraphPattern::Values { .. }
        | GraphPattern::PropertyFunction(_) => pattern.clone(),
        // An ordinary `Join` evaluates its operands independently and joins the results,
        // so a call inside the right operand is invoked with nothing the left operand
        // binds: it sees what the enclosing context binds and no more. Only a `Lateral`
        // hands its right operand the left rows — which is why a call that depends on
        // an earlier atom is rebuilt through one (see [`order_chain`]).
        GraphPattern::Join { left, right } => GraphPattern::Join {
            left: recurse(left, outer, here)?,
            right: recurse(right, outer, here)?,
        },
        // The right side of a `Lateral` is evaluated once per left row with that row in
        // hand, so it sees what the left side certainly binds. A `Lateral` whose right
        // operand is a call is a chain and never reaches this arm, so the right operand
        // here is a pattern the pushdown does not enter.
        GraphPattern::Lateral { left, right } => {
            let mut inner = outer.clone();
            collect_certainly_bound(left, &mut inner);
            GraphPattern::Lateral {
                left: recurse(left, outer, here)?,
                right: recurse(right, &inner, promise.beyond_pushdown())?,
            }
        }
        // `OPTIONAL`'s right side and `MINUS`'s right side are evaluated independently
        // of the left and then matched against it, exactly as a `Join`'s right operand
        // is, so a call inside either sees only what the enclosing context binds. Their
        // own bindings do not escape as certain either, which `certainly_bound`
        // accounts for. Neither right arm is one the pushdown writes into.
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => {
            // The inline condition is evaluated only on candidate JOINED rows, so
            // both sides' bindings are available to it.
            let mut condition_scope = outer.clone();
            collect_certainly_bound(left, &mut condition_scope);
            collect_certainly_bound(right, &mut condition_scope);
            GraphPattern::LeftJoin {
                left: recurse(left, outer, here)?,
                right: recurse(right, outer, promise.beyond_pushdown())?,
                expression: expression
                    .as_ref()
                    .map(|expr| {
                        plan_expression(
                            expr,
                            relations,
                            agg_registry,
                            &condition_scope,
                            promise.beyond_pushdown(),
                        )
                    })
                    .transpose()?,
            }
        }
        GraphPattern::Minus { left, right } => GraphPattern::Minus {
            left: recurse(left, outer, here)?,
            right: recurse(right, outer, promise.beyond_pushdown())?,
        },
        // A `UNION` branch cannot rely on its sibling.
        GraphPattern::Union { left, right } => GraphPattern::Union {
            left: recurse(left, outer, here)?,
            right: recurse(right, outer, here)?,
        },
        // A `FILTER`'s expression is evaluated over the rows its inner pattern
        // produced, so an `EXISTS` inside it sees everything that pattern certainly
        // binds — which is exactly what makes a relation inside a correlated `EXISTS`
        // invocable with the outer row's values.
        GraphPattern::Filter { expr, inner } => {
            let mut scope = outer.clone();
            collect_certainly_bound(inner, &mut scope);
            with_rows(&mut scope, promise);
            GraphPattern::Filter {
                expr: plan_expression(
                    expr,
                    relations,
                    agg_registry,
                    &scope,
                    promise.beyond_pushdown(),
                )?,
                // The pushdown descends a `FILTER` beneath the core as well as above it.
                inner: recurse(inner, outer, promise)?,
            }
        }
        GraphPattern::Extend {
            inner,
            variable,
            expression,
        } => {
            let mut scope = outer.clone();
            collect_certainly_bound(inner, &mut scope);
            with_rows(&mut scope, promise);
            // The pushdown descends a `BIND` beneath the core as well as above it, and
            // does not carry the variable the `BIND` itself binds into its operand.
            let narrowed: DetHashSet<Variable>;
            let into = match promise {
                Promise::Pushed(parameters) if parameters.contains(variable) => {
                    narrowed = parameters
                        .iter()
                        .filter(|parameter| *parameter != variable)
                        .cloned()
                        .collect();
                    Promise::Pushed(&narrowed)
                }
                other => other,
            };
            GraphPattern::Extend {
                inner: recurse(inner, outer, into)?,
                variable: variable.clone(),
                expression: plan_expression(
                    expression,
                    relations,
                    agg_registry,
                    &scope,
                    promise.beyond_pushdown(),
                )?,
            }
        }
        GraphPattern::Unfold {
            inner,
            expression,
            element,
            companion,
        } => {
            let mut scope = outer.clone();
            collect_certainly_bound(inner, &mut scope);
            with_rows(&mut scope, promise);
            GraphPattern::Unfold {
                inner: recurse(inner, outer, wrapped)?,
                expression: plan_expression(
                    expression,
                    relations,
                    agg_registry,
                    &scope,
                    promise.beyond_pushdown(),
                )?,
                element: element.clone(),
                companion: companion.clone(),
            }
        }
        GraphPattern::Graph { name, inner } => GraphPattern::Graph {
            name: name.clone(),
            inner: recurse(inner, outer, here)?,
        },
        GraphPattern::OrderBy { inner, expression } => {
            let mut scope = outer.clone();
            collect_certainly_bound(inner, &mut scope);
            with_rows(&mut scope, promise);
            GraphPattern::OrderBy {
                inner: recurse(inner, outer, wrapped)?,
                expression: expression
                    .iter()
                    .map(|order| {
                        Ok(match order {
                            OrderExpression::Asc(expr) => OrderExpression::Asc(plan_expression(
                                expr,
                                relations,
                                agg_registry,
                                &scope,
                                promise.beyond_pushdown(),
                            )?),
                            OrderExpression::Desc(expr) => OrderExpression::Desc(plan_expression(
                                expr,
                                relations,
                                agg_registry,
                                &scope,
                                promise.beyond_pushdown(),
                            )?),
                        })
                    })
                    .collect::<Result<Vec<_>, PlanError>>()?,
            }
        }
        // A sub-`SELECT` is its own scope: a variable bound outside it is not visible
        // inside, so the correlation set is emptied on the way in. The projection a
        // caller's own `SELECT` produces sits on the descent to the core, so the
        // promise survives it there; a sub-`SELECT` anywhere else is a scope nothing
        // binds a parameter in.
        GraphPattern::Project { inner, variables } => GraphPattern::Project {
            inner: recurse(inner, &DetHashSet::default(), wrapped)?,
            variables: variables.clone(),
        },
        GraphPattern::Distinct { inner } => GraphPattern::Distinct {
            inner: recurse(inner, outer, wrapped)?,
        },
        GraphPattern::Reduced { inner } => GraphPattern::Reduced {
            inner: recurse(inner, outer, wrapped)?,
        },
        GraphPattern::Slice {
            inner,
            start,
            length,
        } => GraphPattern::Slice {
            inner: recurse(inner, outer, wrapped)?,
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
            with_rows(&mut scope, promise);
            GraphPattern::Group {
                inner: recurse(inner, outer, wrapped)?,
                variables: variables.clone(),
                aggregates: aggregates
                    .iter()
                    .map(|(variable, aggregate)| {
                        Ok((
                            variable.clone(),
                            plan_aggregate(
                                aggregate,
                                relations,
                                agg_registry,
                                &scope,
                                promise.beyond_pushdown(),
                            )?,
                        ))
                    })
                    .collect::<Result<Vec<_>, PlanError>>()?,
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
    relations: &PropertyFunctionRegistry,
    agg_registry: &AggregateRegistry,
    outer: &DetHashSet<Variable>,
    promise: Promise<'_>,
) -> Result<Expression, PlanError> {
    // Either hazard alone must still walk `expr` — see `plan_where_pattern`'s
    // identical widening for the same reason: an `EXISTS` whose inner `GROUP BY`
    // has a `Custom` aggregate but no property-function call must still reach
    // that aggregate's admission below (through the `Expression::Exists` arm).
    if !crate::property_fn_eval::expression_reaches_property_function(expr)
        && !crate::property_fn_eval::expression_reaches_custom_aggregate(expr)
    {
        return Ok(expr.clone());
    }
    let sub = |expr: &Expression| {
        plan_expression(expr, relations, agg_registry, outer, promise).map(Box::new)
    };
    Ok(match expr {
        // A correlated `EXISTS` sees its enclosing group's bindings, so `outer` carries
        // straight in: that is what lets a relation inside one be invoked bound. A
        // prepared execution's parameters are in `outer` here exactly when the rows the
        // expression is evaluated over carry them (see [`Promise::in_rows`]); the
        // pushdown never writes into an `EXISTS` body, so nothing more is promised —
        // unless the SHACL pre-binding rewrite runs, which binds them in every call
        // everywhere (see [`Promise::Everywhere`]).
        Expression::Exists(pattern) => Expression::Exists(Box::new(plan_pattern(
            pattern,
            relations,
            agg_registry,
            outer,
            promise.beyond_pushdown(),
        )?)),
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
                .map(|item| plan_expression(item, relations, agg_registry, outer, promise))
                .collect::<Result<Vec<_>, PlanError>>()?,
        ),
        Expression::Coalesce(items) => Expression::Coalesce(
            items
                .iter()
                .map(|item| plan_expression(item, relations, agg_registry, outer, promise))
                .collect::<Result<Vec<_>, PlanError>>()?,
        ),
        Expression::FunctionCall(function, args) => Expression::FunctionCall(
            function.clone(),
            args.iter()
                .map(|arg| plan_expression(arg, relations, agg_registry, outer, promise))
                .collect::<Result<Vec<_>, PlanError>>()?,
        ),
        Expression::NamedNode(_)
        | Expression::Literal(_)
        | Expression::Variable(_)
        | Expression::Bound(_) => expr.clone(),
    })
}

/// Rewrite the expression an aggregate reduces over, and — for
/// [`AggregateFunction::Custom`] — ADMIT the call at prepare time: refuse an
/// unregistered IRI, a positional-argument count `agg_registry`'s registered
/// entry does not declare, or an invalid `; NAME=value` scalarval clause (see
/// [`validate_scalarvals`]), before any governor charge. The built-in-vs-custom
/// admission asymmetry mirrors `crate::property_fn_plan`'s own for a relation: a
/// built-in's arity is checked structurally by the parser (`SUM`/`AVG`/…
/// accept exactly one expression), so only `Custom`'s host-declared arity and
/// scalarvals need checking here.
///
/// # Errors
///
/// A [`PlanError`] tagged [`PlanSeam::Aggregate`] naming the IRI, for an unregistered
/// `AggregateFunction::Custom` IRI, a supplied argument count its registered entry's
/// declared [`crate::user_fn::Arity`] does not accept, or an invalid scalarval (see
/// [`validate_scalarvals`]'s docs for the four ways one is refused). Also propagates
/// [`plan_expression`]'s own errors from rewriting the argument list.
fn plan_aggregate(
    aggregate: &AggregateExpression,
    relations: &PropertyFunctionRegistry,
    agg_registry: &AggregateRegistry,
    outer: &DetHashSet<Variable>,
    promise: Promise<'_>,
) -> Result<AggregateExpression, PlanError> {
    if let AggregateFunction::Custom(iri) = aggregate.function() {
        let iri_str = iri.as_str();
        let Some(custom) = agg_registry.resolve(iri_str) else {
            return Err(PlanError::aggregate(EvalError::function(format!(
                "no custom aggregate is registered for <{iri_str}>"
            ))));
        };
        let declared = crate::agg_fn::arity_contained(custom.as_ref(), iri_str)
            .map_err(PlanError::aggregate)?;
        let supplied = aggregate.args().len();
        if !declared.accepts(supplied) {
            return Err(PlanError::aggregate(EvalError::function(format!(
                "custom aggregate <{iri_str}> is declared with {declared} argument(s); the call \
                 site supplies {supplied}"
            ))));
        }
        let declared_scalarvals = crate::agg_fn::scalarvals_contained(custom.as_ref(), iri_str)
            .map_err(PlanError::aggregate)?;
        validate_scalarvals(iri_str, aggregate.scalarvals(), &declared_scalarvals)
            .map_err(PlanError::aggregate)?;
    }
    let args = aggregate
        .args()
        .iter()
        .map(|e| plan_expression(e, relations, agg_registry, outer, promise))
        .collect::<Result<Vec<_>, PlanError>>()?;
    // A `FOLD`'s own sort keys are per-row expressions read from the same
    // solutions its arguments are, so they must be planned too: a property
    // function or custom aggregate reachable from `FOLD(?v ORDER BY f(?w))`
    // would otherwise skip this walk's prepare-time admission entirely.
    let order_by = aggregate
        .order_by()
        .iter()
        .map(|order| plan_order_expression(order, relations, agg_registry, outer, promise))
        .collect::<Result<Vec<_>, PlanError>>()?;
    // `plan_expression` rewrites each argument in place and never changes the
    // argument COUNT, and planning a sort key never removes one, so this can
    // never turn a valid `aggregate` into an invalid one — the
    // `AggregateExpression::new` call below cannot fail.
    Ok(AggregateExpression::new(
        aggregate.function().clone(),
        args,
        aggregate.scalarvals().to_vec(),
        order_by,
        aggregate.distinct,
    )
    .expect("plan_expression preserves argument count, so arity stays valid"))
}

/// [`plan_expression`] lifted to one [`OrderExpression`] sort key, preserving
/// its `ASC`/`DESC` direction.
fn plan_order_expression(
    order: &OrderExpression,
    relations: &PropertyFunctionRegistry,
    agg_registry: &AggregateRegistry,
    outer: &DetHashSet<Variable>,
    promise: Promise<'_>,
) -> Result<OrderExpression, PlanError> {
    Ok(match order {
        OrderExpression::Asc(expr) => OrderExpression::Asc(plan_expression(
            expr,
            relations,
            agg_registry,
            outer,
            promise,
        )?),
        OrderExpression::Desc(expr) => OrderExpression::Desc(plan_expression(
            expr,
            relations,
            agg_registry,
            outer,
            promise,
        )?),
    })
}

/// Validate a [`AggregateFunction::Custom`] call's `; NAME=value` scalarval
/// clauses (`supplied`) against `<iri>`'s registered
/// [`crate::agg_fn::CustomAggregate::scalarvals`] declaration (`declared`), at
/// PREPARE time — called from [`plan_aggregate`], before any governor charge.
///
/// Four ways a call is refused, checked in this order:
/// 1. **Duplicate name** — the same upper-cased `NAME` supplied twice.
/// 2. **Unknown name** — a supplied name `declared` does not list.
/// 3. **Wrong-typed value** — a supplied value whose datatype does not match
///    its [`ScalarvalSpec::kind`] (e.g. a string where [`ScalarvalKind::Numeric`]
///    is declared).
/// 4. **Missing required name** — every declared name is required (see
///    [`crate::agg_fn::CustomAggregate::scalarvals`]'s docs): a `declared` name
///    with no matching entry in `supplied`.
///
/// # Errors
///
/// [`EvalError::Function`] naming `iri` and the offending scalarval, for any of
/// the four cases above.
fn validate_scalarvals(
    iri: &str,
    supplied: &[(String, Literal)],
    declared: &[ScalarvalSpec],
) -> Result<(), EvalError> {
    let mut seen: DetHashSet<&str> = DetHashSet::default();
    for (name, value) in supplied {
        if !seen.insert(name.as_str()) {
            return Err(EvalError::function(format!(
                "custom aggregate <{iri}> was supplied the scalarval `{name}` more than once"
            )));
        }
        let Some(spec) = declared.iter().find(|spec| spec.name == name) else {
            return Err(EvalError::function(format!(
                "custom aggregate <{iri}> does not accept a scalarval named `{name}`"
            )));
        };
        if !scalarval_value_matches_kind(value, spec.kind) {
            return Err(EvalError::function(format!(
                "custom aggregate <{iri}>'s scalarval `{name}` must be {}, found datatype \
                 <{}>",
                spec.kind.label(),
                value.datatype().as_str()
            )));
        }
    }
    for spec in declared {
        if !supplied.iter().any(|(name, _)| name == spec.name) {
            return Err(EvalError::function(format!(
                "custom aggregate <{iri}> requires a scalarval named `{}`, which the call site \
                 did not supply",
                spec.name
            )));
        }
    }
    Ok(())
}

/// Whether `value`'s datatype matches `kind` — the per-literal check
/// [`validate_scalarvals`] applies to each supplied scalarval. Goes through the
/// SAME [`purrdf_xsd`] numeric-tower classification (`xsd_of` + `is_numeric_xsd`)
/// the evaluator itself uses to classify a runtime `TermValue`, rather than a
/// hand-rolled datatype-IRI string comparison, so a numeric scalarval's
/// admission rule can never drift from what the numeric tower actually accepts
/// elsewhere in this crate.
fn scalarval_value_matches_kind(value: &Literal, kind: ScalarvalKind) -> bool {
    match kind {
        ScalarvalKind::Numeric => xsd_of(&literal_to_value(value))
            .as_ref()
            .is_some_and(is_numeric_xsd),
        ScalarvalKind::String => {
            value.language().is_none()
                && value.datatype().as_str() == purrdf_sparql_algebra::ast::XSD_STRING
        }
    }
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
        // `UNFOLD`s own targets are NOT certainly bound: a SEP-0009 `null` element
        // (or a null map value) yields the row with that variable unbound, so only
        // what the inner pattern certainly binds escapes — the same rule `Extend`
        // follows for a `BIND` whose expression can error.
        | GraphPattern::Unfold { inner, .. }
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

/// A deterministic fingerprint of everything about `relations` that can change either
/// the plan this module produces OR the answer/parallel-safety of the execution that
/// runs it: the registry's own [`RegistryId`](crate::registry_id::RegistryId), followed
/// by every registered IRI's arity, its declared volatility, and its declared modes with
/// their row bounds, IRI-sorted.
///
/// # The instance id comes first, and is load-bearing
///
/// Declared metadata alone cannot tell two registries apart when they happen to agree on
/// every declaration for a shared IRI while resolving it to two DIFFERENT
/// [`PropertyFunction`](crate::property_fn::PropertyFunction) implementations — two
/// relations can declare the identical arity, volatility, and modes while returning
/// entirely different rows. The [`RegistryId`](crate::registry_id::RegistryId) each
/// registry mints at construction ([`PropertyFunctionRegistry::instance_id`]) closes
/// that hole: two registries can never share a fingerprint unless they are the SAME
/// instance (or a [`Clone`] of it, which shares the identical
/// `Arc<dyn PropertyFunction>` implementations — see that type's docs), regardless of
/// how identical their declarations read.
///
/// This belongs in the plan cache's key. The rewrite above is a function of the query
/// text AND the registry's declarations, so two differently-configured — or merely
/// differently CONSTRUCTED — registries can order the same text differently, and a cache
/// keyed on the text alone would hand the second host the first host's plan. Volatility
/// belongs here for a reason the ORDERING pass never sees: it is not read while
/// planning, but it IS read at evaluation time to decide whether a call may run on a
/// fork-join worker (see [`Volatility`](crate::user_fn::Volatility)), so two registries
/// that agree on every arity and mode but disagree about which relation is stable must
/// still be treated as two distinct configurations — sharing a cache entry between them
/// would be silent only until one relation actually depended on sequential evaluation.
///
/// # The content half is [`content_fingerprint`], rendered
///
/// Everything after the instance id is [`content_fingerprint`]'s digest in hex, not a
/// second walk over the same declarations. One fold means the two tiers can never come
/// to disagree about which declared fields matter: a field added to the durable digest
/// is, in the same edit, a field this fingerprint separates plans on. It also makes the
/// join unambiguous by construction — a hex digest is a fixed-width alphabet that cannot
/// contain the delimiter, so no registry's declarations can forge the boundary between
/// the two halves.
///
/// Derived from [`PropertyFunctionRegistry::describe`], which is already IRI-sorted, so
/// the content half of the fingerprint is a pure function of the registry's contents
/// rather than of its construction order (the instance id half is, by construction, a
/// pure function of WHICH construction). Also the identity a governed receipt carries
/// (see `RelationIdentity` in `crate::governed`) — the same reason it belongs in the
/// cache key applies to the receipt: two registries that produce different fingerprints
/// can produce different answers, so a receipt that cannot tell them apart is not a
/// receipt.
///
/// # Errors
///
/// [`EvalError::Function`] if a registered relation's declaration methods panic —
/// [`PropertyFunctionRegistry::describe`]'s own failure, propagated unchanged. Never
/// raised when `relations` is empty (which [`PropertyFunctionRegistry::EMPTY`] — the
/// canonical "no registry" value — always is): that case returns before any
/// relation's declaration is read at all.
pub(crate) fn registry_fingerprint(
    relations: &PropertyFunctionRegistry,
) -> Result<String, EvalError> {
    if relations.is_empty() {
        return Ok(String::new());
    }
    let mut out = String::new();
    out.push_str(&relations.instance_id().stable_encoding().to_string());
    out.push('\u{5}');
    out.push_str(&content_fingerprint(relations)?.to_hex());
    Ok(out)
}

/// The domain separator every property-function content fingerprint opens with, so
/// a digest of this registry kind can never equal a digest of another kind that
/// happens to fold a structurally identical field sequence (an empty
/// property-function registry and an empty aggregate registry would otherwise
/// collide, and a caller binding both would be unable to tell which it had).
const CONTENT_DOMAIN: &str = "purrdf-sparql-eval/property-function-registry";

/// The schema version of the field sequence this fingerprint folds.
///
/// The domain separator above keeps a property-function digest from colliding
/// with *another kind* of registry's digest. It does nothing about a collision
/// between two *versions of this one*, and that is a real gap: the fields below
/// are length-framed, which makes each version's encoding self-delimiting and
/// injective **within** a version, but says nothing across versions. A field
/// added between two existing ones shifts every byte after it, and there is no
/// argument that the resulting string cannot equal some older declaration's —
/// only the observation that nobody has found a pair. A digest that two
/// different schemas can produce is a digest a caller cannot rely on, so the
/// version is folded in and the question stops being open.
///
/// Bumped to 2 when a ranked declaration gained its fidelity term: what a
/// producer promises about the completeness and order of its own rows now
/// changes the registry's identity, because a plan drawn from producers that
/// approximate is not the plan drawn from producers that do not.
const CONTENT_VERSION: u16 = 2;

/// A **content-only** fingerprint of `relations`: every registered IRI's subject
/// and object arity, its declared volatility, its declared modes with their row
/// bounds, and its ranked-retrieval declaration, IRI-sorted — with the registry's
/// instance id **omitted**, digested as a [`ContentDigest`].
///
/// This is the one fold of a property-function registry's declarations in this
/// crate. The crate-internal plan-cache fingerprint renders it in hex behind the
/// instance id rather than walking the descriptors a second time, and
/// [`PropertyFunctionRegistry::content_fingerprint`] hands hosts the same hex, so
/// "which declared fields make two registries different?" has exactly one answer
/// and cannot drift into two.
///
/// # Why a second fingerprint rather than a change to the first
///
/// `registry_fingerprint` leads with the registry's
/// `RegistryId` (`crate::registry_id::RegistryId`), and must keep doing so: that id
/// is the only thing that can tell apart two registries which declare identically
/// but resolve an IRI to two different implementations, and it is what stops a plan
/// prepared against one registry from silently running against another. But the id
/// is a process-lifetime counter. Written into a persisted artifact and read back in
/// another process, it compares two readings of two unrelated sequences — a
/// comparison whose outcome is meaningless in both directions. So an artifact that
/// must name the host registries it requires, and have that requirement checked
/// somewhere else, cannot use `registry_fingerprint` at all; it needs a value that
/// is a pure function of declarations. The two answer different questions and both
/// remain: this one is additive and changes nothing about the first.
///
/// # What it binds, and what it deliberately does not
///
/// It binds **declarations**, not implementations. Two independently built
/// registries that declare the same IRIs with the same arities, volatilities,
/// modes and ranked-retrieval declarations produce the same digest even when their
/// [`PropertyFunction`](crate::property_fn::PropertyFunction) implementations return
/// entirely different rows — exactly the hole the instance id closes in-process, and
/// one no content-derived value can close, because a trait object exposes no content
/// to digest. A caller crossing a process boundary must therefore pair this digest
/// with whatever separately identifies the implementations behind the declarations,
/// and must not read a match as proof that two registries compute the same answers.
///
/// # The rejected alternative: a hash of the instance id's persisted rendering
///
/// Persisting `RegistryId::stable_encoding` and comparing it on restore was rejected
/// outright. It does not merely fail to help — it fails in the shape that is hardest
/// to see: the restoring process's registries carry ids drawn from a counter that
/// restarted at `1`, so a bare `2` may match a completely unrelated registry
/// (accepting an artifact that should be refused) while an identically-declared
/// rebuild of the exact registry the artifact was produced against gets a different
/// id and is refused (refusing one that should be accepted). Both failures are silent
/// and neither is reproducible.
///
/// # Encoding
///
/// Framed through `append_framed_part` (`crate::registry_id::append_framed_part`),
/// whose length-prefixing makes the byte sequence injective, then digested. An empty
/// registry is not special-cased: it digests the domain separator alone, so
/// [`PropertyFunctionRegistry::EMPTY`](crate::property_fn::PropertyFunctionRegistry::EMPTY)
/// and any other empty registry agree, exactly as they do under
/// `registry_fingerprint`'s empty-string short circuit.
///
/// # Errors
///
/// [`EvalError::Function`] if a registered relation's declaration methods panic —
/// [`PropertyFunctionRegistry::describe`](crate::property_fn::PropertyFunctionRegistry::describe)'s
/// own failure, propagated unchanged.
pub fn content_fingerprint(
    relations: &PropertyFunctionRegistry,
) -> Result<ContentDigest, EvalError> {
    let mut bytes = Vec::new();
    append_framed_part(&mut bytes, "domain", CONTENT_DOMAIN.as_bytes());
    append_framed_part(&mut bytes, "version", &CONTENT_VERSION.to_be_bytes());
    for descriptor in relations.describe()? {
        append_framed_part(&mut bytes, "iri", descriptor.iri.as_bytes());
        append_framed_part(
            &mut bytes,
            "subject-arity",
            &(descriptor.subject_arity as u64).to_be_bytes(),
        );
        append_framed_part(
            &mut bytes,
            "object-arity",
            &(descriptor.object_arity as u64).to_be_bytes(),
        );
        append_framed_part(
            &mut bytes,
            "volatility",
            descriptor.volatility.label().as_bytes(),
        );
        append_framed_part(
            &mut bytes,
            "mode-count",
            &(descriptor.modes.len() as u64).to_be_bytes(),
        );
        for mode in &descriptor.modes {
            append_framed_part(&mut bytes, "mode-code", mode.code.as_bytes());
            append_framed_part(
                &mut bytes,
                "mode-rows",
                &mode.rows_per_invocation.to_be_bytes(),
            );
        }
        // The ranked-retrieval declaration the host supplied at registration. A
        // producer's participation in fusion is a declaration exactly as arity,
        // volatility and modes are: it decides whether a request can draw ranked
        // candidates from this IRI at all, and under which stratum, ordering and
        // duplicate guarantee they arrive. A relation registered without one
        // contributes the fixed `NOT_RANKED_CANONICAL` description rather than
        // nothing, so a producer that does not fuse still occupies the field and
        // cannot be confused with one whose declaration was simply not read.
        append_framed_part(
            &mut bytes,
            "ranked",
            match descriptor.ranked.as_ref() {
                None => NOT_RANKED_CANONICAL.to_owned(),
                Some(declaration) => declaration.canonical_description(),
            }
            .as_bytes(),
        );
    }
    Ok(ContentDigest::of(&bytes))
}

#[cfg(test)]
mod registry_fingerprint_tests {
    use std::sync::Arc;

    use super::registry_fingerprint;
    use crate::property_fn::{MemoryRelation, PropertyFunctionRegistry};

    const EX_REL: &str = "http://example.org/ns#rel";

    /// [`PropertyFunctionRegistry::EMPTY`] (the canonical "no registry" value
    /// [`crate::engine::QueryOptions::property_functions`]/[`crate::eval::EvalCtx::property_functions`]/[`crate::parallel::SafetyRegistries::relations`]
    /// now carry in place of `Option::None`) and a freshly built, still-empty
    /// [`PropertyFunctionRegistry::new`] are two DIFFERENT instances (different
    /// underlying maps, different constructions) yet must fingerprint IDENTICALLY:
    /// both resolve every IRI to `None`, so no plan's admitted behavior can ever
    /// depend on which one it was prepared against — see
    /// [`crate::registry_id::RegistryId::EMPTY`]'s docs for why sharing one fixed
    /// instance id is the deliberately correct semantics here, not a weakening of
    /// the plan-identity guard the test right below this one (over two
    /// DIFFERENT, non-empty registries) continues to pin.
    #[test]
    fn empty_const_and_a_freshly_built_empty_registry_share_the_same_fingerprint() {
        assert_eq!(
            registry_fingerprint(&PropertyFunctionRegistry::EMPTY).expect("ok"),
            ""
        );
        let fresh = PropertyFunctionRegistry::new();
        assert_eq!(registry_fingerprint(&fresh).expect("ok"), "");
    }

    /// Registry instance identity: two INDEPENDENTLY constructed registries
    /// that register the SAME IRI to relations with byte-identical declared
    /// metadata (arity, volatility, modes, row bounds) — [`MemoryRelation`]s
    /// holding the SAME NUMBER of rows, so `describe()` reports identically, but
    /// different actual row content — must still produce DIFFERENT fingerprints.
    /// Declared metadata alone cannot prove the two registries answer the same
    /// way; only the instance id can.
    #[test]
    fn two_independently_built_registries_with_identical_declarations_still_differ() {
        let mut a = PropertyFunctionRegistry::new();
        a.register(
            EX_REL,
            Arc::new(
                MemoryRelation::new(
                    1,
                    1,
                    vec![vec![
                        purrdf_core::TermValue::iri("http://example.org/ns#one"),
                        purrdf_core::TermValue::iri("http://example.org/ns#alpha"),
                    ]],
                )
                .expect("one row, two values wide"),
            ),
        );
        let mut b = PropertyFunctionRegistry::new();
        b.register(
            EX_REL,
            Arc::new(
                // Same row COUNT (so `describe()` — arity, volatility, modes, and
                // the row-count-derived bound — is byte-identical to `a`'s), but a
                // DIFFERENT row, so the two relations answer a query differently.
                MemoryRelation::new(
                    1,
                    1,
                    vec![vec![
                        purrdf_core::TermValue::iri("http://example.org/ns#one"),
                        purrdf_core::TermValue::iri("http://example.org/ns#beta"),
                    ]],
                )
                .expect("one row, two values wide"),
            ),
        );

        assert_eq!(a.describe().expect("ok"), b.describe().expect("ok"));
        assert_ne!(
            registry_fingerprint(&a).expect("ok"),
            registry_fingerprint(&b).expect("ok"),
            "two independently constructed registries must never share a fingerprint, \
             even when every declaration they report is identical"
        );
    }

    /// A [`Clone`] shares the source's `Arc<dyn PropertyFunction>` implementations,
    /// so it must produce the SAME fingerprint as its source.
    #[test]
    fn a_clone_shares_its_source_registrys_fingerprint() {
        let mut registry = PropertyFunctionRegistry::new();
        registry.register(
            EX_REL,
            Arc::new(
                MemoryRelation::new(
                    1,
                    1,
                    vec![vec![
                        purrdf_core::TermValue::iri("http://example.org/ns#one"),
                        purrdf_core::TermValue::iri("http://example.org/ns#alpha"),
                    ]],
                )
                .expect("one row, two values wide"),
            ),
        );
        let cloned = registry.clone();
        assert_eq!(
            registry_fingerprint(&registry).expect("ok"),
            registry_fingerprint(&cloned).expect("ok"),
            "a clone shares the source's actual implementations, so it is the same \
             registry instance for fingerprint purposes"
        );
    }
}

#[cfg(test)]
mod content_fingerprint_tests {
    use std::sync::Arc;

    use purrdf_core::binding_pattern::BindingPattern;

    use super::{content_fingerprint, registry_fingerprint};
    use crate::error::EvalError;
    use crate::property_fn::{
        CandidateDomains, DuplicatePolicy, ExclusionBasis, PfArgs, PfArity, PfCursor, PfRow,
        PropertyFunction, PropertyFunctionRegistry, RankFidelity, RankedDeclaration,
    };
    use crate::user_fn::Volatility;

    const EX_REL: &str = "http://example.org/ns#rel";
    const EX_OTHER: &str = "http://example.org/ns#other";
    const EX_STRATUM: &str = "http://example.org/stratum/a";
    const EX_STRATUM_B: &str = "http://example.org/stratum/b";

    /// A relation that declares exactly what its constructor was handed and nothing
    /// else, so a test can vary ONE declared field at a time and watch the content
    /// fingerprint move. Never dispatched — these fixtures exist to be described.
    #[derive(Debug)]
    struct DeclaredRelation {
        arity: PfArity,
        volatility: Volatility,
        modes: [BindingPattern; 1],
    }

    impl DeclaredRelation {
        fn new(subject: usize, object: usize, volatility: Volatility) -> Self {
            let arity = PfArity::new(subject, object);
            Self {
                arity,
                volatility,
                modes: [arity.all_free_mode()],
            }
        }
    }

    /// The empty cursor [`DeclaredRelation::open`] hands out.
    struct EmptyCursor;

    impl PfCursor for EmptyCursor {
        fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
            Ok(None)
        }
    }

    impl PropertyFunction for DeclaredRelation {
        fn volatility(&self) -> Volatility {
            self.volatility
        }

        fn arity(&self) -> PfArity {
            self.arity
        }

        fn modes(&self) -> &[BindingPattern] {
            &self.modes
        }

        fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
            0
        }

        fn open(
            &self,
            _args: &PfArgs<'_>,
            _ceiling: Option<u64>,
        ) -> Result<Box<dyn PfCursor>, EvalError> {
            Ok(Box::new(EmptyCursor))
        }
    }

    /// A registry holding one [`DeclaredRelation`] under `iri`.
    fn one(
        iri: &str,
        subject: usize,
        object: usize,
        volatility: Volatility,
    ) -> PropertyFunctionRegistry {
        let mut registry = PropertyFunctionRegistry::new();
        registry.register(
            iri,
            Arc::new(DeclaredRelation::new(subject, object, volatility)),
        );
        registry
    }

    /// A minimal, valid ranked declaration claiming `stratum` and nothing else —
    /// no accepted terms, no depth placement, and its candidate at position `0`,
    /// which every relation registered through [`one_ranked`] declares (both
    /// arguments are always present). The only field a test varies across two
    /// calls is `stratum`, passed explicitly.
    fn minimal_ranked_declaration(stratum: &str) -> RankedDeclaration {
        RankedDeclaration {
            stratum: purrdf_core::parse_iri(stratum).expect("fixture IRI"),
            accepted_terms: Vec::new(),
            depth_placement: None,
            candidate_position: 0,
            duplicates: DuplicatePolicy::Unique,
            // The fixture producer is an in-memory table read end to end.
            fidelity: RankFidelity::EXACT,
            domains: CandidateDomains::Unrestricted,
            block_position: None,
            exclusion: ExclusionBasis::Unavailable,
            mandatory: false,
        }
    }

    /// A registry holding one [`DeclaredRelation`] under `iri`, wired up as a
    /// ranked producer under `stratum` — the ranked counterpart to [`one`], built
    /// from the SAME relation constructor so only the ranked declaration differs.
    fn one_ranked(
        iri: &str,
        subject: usize,
        object: usize,
        volatility: Volatility,
        stratum: &str,
    ) -> PropertyFunctionRegistry {
        let mut registry = PropertyFunctionRegistry::new();
        registry.register_ranked(
            iri,
            Arc::new(DeclaredRelation::new(subject, object, volatility)),
            minimal_ranked_declaration(stratum),
        );
        registry
    }

    /// A relation like [`DeclaredRelation`] but with an explicit, caller-chosen
    /// mode list and row bound, so a test can vary the declared MODES (or their
    /// row bounds) alone while the IRI, arity and volatility stay fixed. Never
    /// dispatched, exactly like [`DeclaredRelation`].
    #[derive(Debug)]
    struct ModedRelation {
        arity: PfArity,
        modes: Vec<BindingPattern>,
        rows_per_invocation: u64,
    }

    impl ModedRelation {
        fn new(arity: PfArity, modes: Vec<BindingPattern>, rows_per_invocation: u64) -> Self {
            Self {
                arity,
                modes,
                rows_per_invocation,
            }
        }
    }

    impl PropertyFunction for ModedRelation {
        fn volatility(&self) -> Volatility {
            Volatility::Stable
        }

        fn arity(&self) -> PfArity {
            self.arity
        }

        fn modes(&self) -> &[BindingPattern] {
            &self.modes
        }

        fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
            self.rows_per_invocation
        }

        fn open(
            &self,
            _args: &PfArgs<'_>,
            _ceiling: Option<u64>,
        ) -> Result<Box<dyn PfCursor>, EvalError> {
            Ok(Box::new(EmptyCursor))
        }
    }

    /// A registry holding one [`ModedRelation`] under `iri`.
    fn one_moded(
        iri: &str,
        arity: PfArity,
        modes: Vec<BindingPattern>,
        rows_per_invocation: u64,
    ) -> PropertyFunctionRegistry {
        let mut registry = PropertyFunctionRegistry::new();
        registry.register(
            iri,
            Arc::new(ModedRelation::new(arity, modes, rows_per_invocation)),
        );
        registry
    }

    /// The whole point of the content tier: two registries built INDEPENDENTLY — two
    /// distinct instances, two distinct `RegistryId`s — that declare the same thing
    /// must produce the SAME content fingerprint, so a consumer process can reproduce
    /// a producer's value from a rebuilt registry.
    ///
    /// The second half is what proves this function is not merely a rename of
    /// `registry_fingerprint`: over the very same pair, the instance-bearing
    /// fingerprint must still DIFFER, because the guarantee it makes in-process is
    /// unchanged by the addition of the content tier.
    #[test]
    fn content_fingerprint_is_stable_across_registry_instances() {
        let a = one(EX_REL, 1, 1, Volatility::Stable);
        let b = one(EX_REL, 1, 1, Volatility::Stable);

        assert_eq!(
            content_fingerprint(&a).expect("ok"),
            content_fingerprint(&b).expect("ok"),
            "the content fingerprint must be a pure function of declarations, so two \
             independently built registries declaring the same thing agree"
        );
        assert_ne!(
            registry_fingerprint(&a).expect("ok"),
            registry_fingerprint(&b).expect("ok"),
            "the instance-bearing fingerprint must still tell the same two registries \
             apart — the content tier is additive, not a replacement"
        );
    }

    /// Registration order must not reach the digest: the fold reads
    /// `describe()`, which is IRI-sorted.
    #[test]
    fn content_fingerprint_ignores_registration_order() {
        let mut a = PropertyFunctionRegistry::new();
        a.register(
            EX_REL,
            Arc::new(DeclaredRelation::new(1, 1, Volatility::Stable)),
        );
        a.register(
            EX_OTHER,
            Arc::new(DeclaredRelation::new(2, 1, Volatility::Volatile)),
        );
        let mut b = PropertyFunctionRegistry::new();
        b.register(
            EX_OTHER,
            Arc::new(DeclaredRelation::new(2, 1, Volatility::Volatile)),
        );
        b.register(
            EX_REL,
            Arc::new(DeclaredRelation::new(1, 1, Volatility::Stable)),
        );

        assert_eq!(
            content_fingerprint(&a).expect("ok"),
            content_fingerprint(&b).expect("ok")
        );
    }

    #[test]
    fn content_fingerprint_separates_iri() {
        assert_ne!(
            content_fingerprint(&one(EX_REL, 1, 1, Volatility::Stable)).expect("ok"),
            content_fingerprint(&one(EX_OTHER, 1, 1, Volatility::Stable)).expect("ok"),
            "two registries declaring DIFFERENT IRIs must never share a digest"
        );
    }

    #[test]
    fn content_fingerprint_separates_arity() {
        assert_ne!(
            content_fingerprint(&one(EX_REL, 1, 1, Volatility::Stable)).expect("ok"),
            content_fingerprint(&one(EX_REL, 1, 2, Volatility::Stable)).expect("ok"),
            "object arity is a declared field and must reach the digest"
        );
        assert_ne!(
            content_fingerprint(&one(EX_REL, 1, 1, Volatility::Stable)).expect("ok"),
            content_fingerprint(&one(EX_REL, 2, 1, Volatility::Stable)).expect("ok"),
            "subject arity is a declared field and must reach the digest"
        );
        // The framing is injective across the two arity fields: (1, 2) and (2, 1)
        // have the same total, and must still differ.
        assert_ne!(
            content_fingerprint(&one(EX_REL, 1, 2, Volatility::Stable)).expect("ok"),
            content_fingerprint(&one(EX_REL, 2, 1, Volatility::Stable)).expect("ok"),
        );
    }

    #[test]
    fn content_fingerprint_separates_volatility() {
        assert_ne!(
            content_fingerprint(&one(EX_REL, 1, 1, Volatility::Stable)).expect("ok"),
            content_fingerprint(&one(EX_REL, 1, 1, Volatility::Volatile)).expect("ok"),
            "volatility decides whether a call may run on a fork-join worker, so two \
             registries that disagree about it are two configurations"
        );
    }

    #[test]
    fn content_fingerprint_separates_ranked() {
        assert_ne!(
            content_fingerprint(&one(EX_REL, 1, 1, Volatility::Stable)).expect("ok"),
            content_fingerprint(&one_ranked(EX_REL, 1, 1, Volatility::Stable, EX_STRATUM))
                .expect("ok"),
            "a relation's ranked-retrieval declaration decides whether a request can draw \
             ranked candidates from it at all, so a plainly registered relation and the SAME \
             relation wired up as a ranked producer must never share a digest"
        );
        assert_ne!(
            content_fingerprint(&one_ranked(EX_REL, 1, 1, Volatility::Stable, EX_STRATUM))
                .expect("ok"),
            content_fingerprint(&one_ranked(EX_REL, 1, 1, Volatility::Stable, EX_STRATUM_B))
                .expect("ok"),
            "two ranked declarations that differ only in the stratum they claim must still \
             produce different digests: the whole declaration is folded, not merely its \
             presence"
        );
    }

    #[test]
    fn content_fingerprint_separates_modes() {
        let arity = PfArity::new(1, 1);
        let all_free = arity.all_free_mode();
        let subject_bound = BindingPattern::from_bound_positions(arity.total(), [0]);

        assert_ne!(
            content_fingerprint(&one_moded(EX_REL, arity, vec![all_free], 0)).expect("ok"),
            content_fingerprint(&one_moded(EX_REL, arity, vec![subject_bound], 0)).expect("ok"),
            "two relations declaring the SAME arity and volatility but DIFFERENT access \
             patterns must never share a digest"
        );
        assert_ne!(
            content_fingerprint(&one_moded(EX_REL, arity, vec![all_free], 0)).expect("ok"),
            content_fingerprint(&one_moded(EX_REL, arity, vec![all_free], 5)).expect("ok"),
            "a mode's declared row bound is folded independently of its code, so two \
             relations serving the SAME access pattern with different declared bounds must \
             never share a digest"
        );
    }

    /// The empty registry's digest is pinned to a literal, because it is the value a
    /// consumer in another process computes for a registry it never received — if it
    /// ever moves, every previously persisted artifact silently stops matching.
    ///
    /// The canonical `EMPTY` constant and a freshly built empty registry agree, the
    /// same way they do under `registry_fingerprint`'s empty-string short circuit.
    ///
    /// # Why this literal moved, and why that is the mechanism working
    ///
    /// "Silently" is the word the paragraph above is really about, and it is what
    /// `CONTENT_VERSION` exists to remove. The digest folds a *schema* — a fixed
    /// sequence of framed fields — and when a field is added to that sequence every
    /// digest under it changes whether or not the new field carries a value; an empty
    /// registry has no declarations at all and still moves, because what moved is the
    /// schema, not the contents. Before the version was folded in there was nothing to
    /// distinguish "computed under an older schema" from "computed over different
    /// declarations", and a stale artifact could only ever present as the second.
    /// Bumping the constant is the deliberate, visible act that invalidates the old
    /// value; a literal here that never moved across a schema change would mean the
    /// schema change had gone unrecorded.
    ///
    /// The re-pin is therefore expected on any release that changes the field
    /// sequence, and it is not licence to re-pin one that changes only behaviour: a
    /// digest that moves without a version bump beside it is a defect.
    #[test]
    fn content_fingerprint_empty_registry_is_pinned() {
        let empty = content_fingerprint(&PropertyFunctionRegistry::EMPTY).expect("ok");
        assert_eq!(
            content_fingerprint(&PropertyFunctionRegistry::new()).expect("ok"),
            empty,
            "every empty registry resolves every IRI to None, so all of them agree"
        );
        assert_eq!(
            empty.to_hex(),
            "d73476de4655573c5093e8c6a85b5653d03b7562d9bbcd8846b9d5c7b930c62a",
            "the empty property-function registry digest is a persisted constant"
        );
        assert_ne!(
            content_fingerprint(&one(EX_REL, 1, 1, Volatility::Stable)).expect("ok"),
            empty,
            "a non-empty registry must never digest as the empty one"
        );
    }
}
