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

/// Rewrite every property-function chain in `query` into a feasible order, after
/// joining every blank node label shared between the pieces of one basic graph pattern
/// (`crate::blank_scope`).
///
/// The query is returned unchanged — after one read-only walk — when it carries no call
/// node and no such shared label, which is every query on a host that has not
/// configured the seam and writes no blank node into two pieces of one block.
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
/// arity mismatch, an infeasible chain), same shared-blank join and feasibility
/// rewrite, same "untouched when the pattern carries neither a call node nor a shared
/// blank label" contract — an UPDATE WHERE is a triple-pattern context exactly like a
/// query's, so it is planned exactly like one.
///
/// Returns `Ok(None)` unchanged (not merely equal) in that case, so a caller can keep
/// evaluating its own borrowed `pattern` rather than a clone that happens to match
/// it.
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
    // A blank node label written in two pieces of one basic graph pattern — a
    // triple and a call, a triple and a path, two calls — is one variable across
    // them. Settled first, and for every pattern whether or not it carries a call,
    // because a call's arguments are admitted below by what is bound, and a
    // shared blank a sibling binds IS bound. See `crate::blank_scope`.
    let joined = crate::blank_scope::join_shared_blanks(pattern);
    let pattern = joined.as_ref().unwrap_or(pattern);
    // Either hazard alone must still run the walk: a query with a `Custom`
    // aggregate and no property-function call would otherwise skip this pass
    // entirely on the property-function-only check, and its admission (below,
    // via `plan_aggregate`) would never happen.
    if !crate::property_fn_eval::pattern_needs_admission(pattern) {
        return Ok(joined);
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
///   both operands of a `Join`, a `UNION` and a `LATERAL`, the inner of a `GRAPH`,
///   `FILTER`, `BIND`, `DISTINCT`, `REDUCED` and `ORDER BY`, the left operand of an
///   `OPTIONAL` and a `MINUS`, and a sub-`SELECT` for the parameters it projects.
///   Everywhere else — an `OPTIONAL`'s or a `MINUS`'s right arm, the body of an
///   `EXISTS` beneath the core, a sub-`SELECT` that does not project the parameter,
///   the inner of a slice or a `GROUP BY` — the pushdown does not write, and a call
///   there is invoked with the parameter free.
///   Admitting it as though it were bound would exchange a refusal at prepare for a
///   refusal on every run.
///
/// A run under the SHACL pre-binding rewrite reaches further: it binds a parameter in
/// every call in the query, and that is [`Self::Everywhere`].
///
/// Neither half extends `outer`, which stays the set of variables the pattern ITSELF
/// certainly binds: a promise is consulted only where a call is admitted, and where a
/// `BIND` or an aggregate reads a parameter — each only where the rewrite really puts
/// the value (see [`Written`]): the seed's rows, on the descent, or every expression,
/// under the SHACL pre-binding rewrite. A `BIND($this AS ?x)` feeding a call binds
/// `?x` exactly where the run hands the `BIND` the value, and nowhere else.
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

    /// What the run writes into the rows and expressions of a pattern evaluated at
    /// this node — see [`Written`].
    const fn written(self) -> Written<'a> {
        Written {
            seed: self.in_rows(),
            everywhere: self.everywhere(),
        }
    }

    /// What the run writes into every expression beneath this node: every parameter,
    /// under the SHACL pre-binding rewrite, and nothing otherwise.
    const fn everywhere(self) -> Option<&'a DetHashSet<Variable>> {
        match self {
            Self::Everywhere(parameters) => Some(parameters),
            Self::None | Self::Descent(_) | Self::Pushed(_) => None,
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
///
/// # What joins into one chain
///
/// Every member of a chain is joined to every other, and a join is associative and
/// commutative — which is what licenses [`order_chain`] to reorder them at all. So a
/// `Join` whose operand is ITSELF a chain spine is one chain with it, not an opaque
/// member: `{ VALUES ?q { … } ?q ex:rel ?out }` parses to `Join(VALUES, Lateral(Z,
/// call))`, and the call is fed by the `VALUES` exactly as it would be by a triple
/// written in its own block. Treating the right operand as a sealed atom evaluated on
/// its own would leave the call seeing none of it, and refuse at prepare a query the
/// data can serve.
///
/// A `LATERAL` whose right operand is NOT a call is the opposite case, and is not a
/// chain node: its right operand is evaluated once per left row with that row in
/// hand, a dependency a `Join` does not carry. Flattening it into a chain would
/// rebuild it through a `Join` and evaluate the right operand without the left rows;
/// the structural recursion keeps it a `Lateral` and plans each side in its own scope.
fn collect_chain<'a>(pattern: &'a GraphPattern, atoms: &mut Vec<Atom<'a>>) -> bool {
    match pattern {
        GraphPattern::Lateral { left, right } => {
            let Some((node, call)) = planned_lateral_call(right) else {
                return false;
            };
            if !collect_chain(left, atoms) {
                push_atom(left, None, atoms);
            }
            push_atom(node, Some(call), atoms);
            true
        }
        GraphPattern::Join { left, right } => {
            // A call under a `Join` rather than a `Lateral` would lose the dependency
            // the `Lateral` encodes, so it is not treated as a chain member; the
            // structural recursion handles it.
            if matches!(&**right, GraphPattern::PropertyFunction(_)) {
                return false;
            }
            if !collect_chain(left, atoms) {
                push_atom(left, None, atoms);
            }
            if !collect_chain(right, atoms) {
                push_atom(right, None, atoms);
            }
            true
        }
        _ => false,
    }
}

/// The call a `Lateral`'s right operand becomes once planned, when it becomes one — and
/// the node that holds it.
///
/// The pushdown writes a pre-bound value into a call that is a `Lateral`'s right
/// operand in place ([`crate::substitute::lateral_call`]), and recurses into any other
/// right operand — and it runs on the PLANNED query, not the text. The two differ in
/// exactly one way this pass controls: a group whose only member is a call parses to
/// `Lateral(Z, call)`
/// (the empty `Bgp` the parser opens every block with), and [`order_chain`] rebuilds
/// that chain as the bare call, because [`push_atom`] drops `Z`. So
/// `?s ?p ?o LATERAL { ?q <rel> ?out }` is PLANNED as `Lateral(?s ?p ?o, call)` — the
/// position the pushdown writes into — though its text puts a `Lateral` between the
/// two.
///
/// Admitting that call as though the pushdown did not reach it refused, at prepare, a
/// call every run invokes bound. Reading the right operand through the identity
/// wrappers the rebuild removes — and then asking the pushdown's own predicate — makes
/// the admission and the write the same decision: `Lateral(Z, r)` is `r` for every
/// `r` (the identity table joined laterally with anything is that thing), so the peel
/// changes no answer, and the call becomes a member of the enclosing chain, admitted
/// under the promise the pushdown keeps there.
fn planned_lateral_call(right: &GraphPattern) -> Option<(&GraphPattern, &PropertyFunctionCall)> {
    match right {
        GraphPattern::Lateral { left, right } if is_identity(left) => planned_lateral_call(right),
        other => crate::substitute::lateral_call(other).map(|call| (other, call)),
    }
}

/// Whether `pattern` is the identity table `Z` — the empty `Bgp`.
const fn is_identity(pattern: &GraphPattern) -> bool {
    matches!(pattern, GraphPattern::Bgp { patterns } if patterns.is_empty())
}

/// Push one chain member, dropping the EMPTY `Bgp` the parser leaves where a call opens
/// its block. It is the identity table `Z`, so joining it back in would only widen the
/// rebuilt tree with a node that binds nothing and matches everything.
fn push_atom<'a>(
    pattern: &'a GraphPattern,
    call: Option<&'a PropertyFunctionCall>,
    atoms: &mut Vec<Atom<'a>>,
) {
    if is_identity(pattern) {
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
        // Every atom is evaluated with the enclosing context in hand (the chain as a
        // whole is), so a `BIND` inside one reading only `outer` binds its target —
        // but never with an earlier sibling's rows: siblings are joined, not correlated.
        collect_bound(atom.pattern, outer, promise.written(), &mut bound);
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
        // operand plans to a call is a chain ([`planned_lateral_call`]) and never
        // reaches this arm; any other right operand is one the pushdown recurses into.
        GraphPattern::Lateral { left, right } => {
            let mut inner = outer.clone();
            collect_bound(left, outer, here.written(), &mut inner);
            GraphPattern::Lateral {
                left: recurse(left, outer, here)?,
                // The pushdown enters every right operand too: it is re-evaluated per
                // left row and inner-joined with it, so restricting a leaf inside it
                // restricts the node (see `crate::substitute`'s `push_probes`).
                right: recurse(right, &inner, here)?,
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
            collect_bound(left, outer, here.written(), &mut condition_scope);
            collect_bound(right, outer, here.written(), &mut condition_scope);
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
            collect_bound(inner, outer, promise.written(), &mut scope);
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
            collect_bound(inner, outer, promise.written(), &mut scope);
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
            collect_bound(inner, outer, promise.written(), &mut scope);
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
            collect_bound(inner, outer, promise.written(), &mut scope);
            GraphPattern::OrderBy {
                // The pushdown enters an `ORDER BY` beneath the core as well as above it.
                inner: recurse(inner, outer, promise)?,
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
        // A sub-`SELECT` is its own scope: a variable bound outside it is visible inside
        // only when it projects it — the correlated substitution (a `LATERAL`'s right
        // operand, an `EXISTS` body) writes the outer row into it for exactly the
        // projected variables — so the correlation set is narrowed to them on the way
        // in. The projection a caller's own `SELECT` produces sits on the descent to the
        // core, so the promise survives it there; a sub-`SELECT` anywhere else is a
        // scope nothing binds a parameter in.
        //
        // Beneath the core the pushdown enters a sub-`SELECT` too, for exactly the
        // parameters it projects: a projected variable is the same variable inside, and
        // restricting the inner rows restricts the output the same way. A parameter it
        // does not project is a different variable inside, and nothing is promised for it.
        GraphPattern::Project { inner, variables } => {
            let projected: DetHashSet<Variable>;
            let into = match promise {
                Promise::Pushed(parameters)
                    if parameters
                        .iter()
                        .all(|parameter| variables.contains(parameter)) =>
                {
                    promise
                }
                Promise::Pushed(parameters) => {
                    projected = narrowed_to(parameters, variables);
                    if projected.is_empty() {
                        Promise::None
                    } else {
                        Promise::Pushed(&projected)
                    }
                }
                other => other,
            };
            GraphPattern::Project {
                inner: recurse(inner, &narrowed_to(outer, variables), into)?,
                variables: variables.clone(),
            }
        }
        // Row-for-row wrappers the pushdown enters beneath the core as well as above it.
        GraphPattern::Distinct { inner } => GraphPattern::Distinct {
            inner: recurse(inner, outer, promise)?,
        },
        GraphPattern::Reduced { inner } => GraphPattern::Reduced {
            inner: recurse(inner, outer, promise)?,
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
            collect_bound(inner, outer, promise.written(), &mut scope);
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

/// Add to `out` every variable `pattern` binds in **every** solution it produces, save
/// only a solution in which a `BIND`'s own expression raised an error.
///
/// This is deliberately narrower than a scope walk. A variable that is merely *in
/// scope* may be unbound in a row for a reason the query text fixes — `OPTIONAL`'s
/// right side, a `UNION` branch that does not mention it, an `UNDEF` cell of a `VALUES`
/// column, an aggregate over an empty group — and treating one of those as bound
/// would admit an invocation the text itself guarantees some row cannot make.
///
/// # The rule, exactly
///
/// A variable is a binding source here when it is:
///
/// * a variable of a triple pattern, a path's endpoints, or a property-function
///   argument;
/// * a `VALUES` column with a term — no `UNDEF` — in every row;
/// * the target of a `BIND` whose expression reads only variables its inner pattern
///   binds by this same rule, or variables the context it is evaluated in holds bound
///   — the left operand of an enclosing `LATERAL` (see [`expression_reads_only_bound`]
///   and [`collect_certainly_bound_in`]);
/// * a variable a `FILTER`'s condition requires bound for it to be true
///   ([`truth_requires`]): `FILTER(BOUND(?v))`, `FILTER(sameTerm(?v, ?o))`,
///   `FILTER(isIRI(?v))` and their conjunctions bind `?v`; a disjunction binds only
///   what both of its sides do, and `FILTER(!BOUND(?v))` binds nothing;
/// * the variable naming a `GRAPH`;
/// * a grouping key every row of the grouped pattern binds;
/// * an aggregate's output when [`aggregate_certainly_binds`] says every group row
///   holds one — `COUNT` always, `SAMPLE`/`MIN`/`MAX` under an explicit `GROUP BY` of
///   an argument that cannot error on what every row binds, nothing else;
/// * bound by both operands of a `UNION`, by either operand of a `Join` or a
///   `LATERAL` (the right one judged with the left one's bindings in hand), by the
///   left operand of an `OPTIONAL` or a `MINUS`, by the inner pattern of a `FILTER`,
///   `BIND`, `UNFOLD`, `ORDER BY`, `DISTINCT`, `REDUCED` or slice, or by the inner
///   pattern of a sub-`SELECT` that projects it.
///
/// # Why a `BIND` counts although its expression can error
///
/// An expression that errors leaves its variable unbound in that row (SPARQL 1.1
/// §18.6). When every variable it reads is bound, that is an outcome of evaluating one
/// row, not a shape of the query: the same `BIND(CONCAT(?a, ?b) AS ?q)` binds `?q` on
/// every row its operands are strings, and whether any row errors is a fact about the
/// data. Refusing it here would refuse, at prepare, every call fed by a computed value
/// — including every run that would never have erred. So such a `BIND` target counts,
/// and a row that did err is handled where rows are: the evaluator re-derives every
/// invocation's access pattern from the row in hand and refuses one its relation does
/// not declare, with a typed error naming both patterns (`admit_mode` in
/// `crate::property_fn_eval`). A relation is therefore never invoked with a position
/// free that its declared modes require bound; a relation that ALSO declares the free
/// mode is invoked free on that row, as the row's own access pattern is.
///
/// A `BIND` that reads a variable its inner pattern does NOT certainly bind —
/// `BIND(?x AS ?q)` after an `OPTIONAL` that may leave `?x` unbound, or over an
/// aggregate's output that may be unbound (a `MIN` with no `GROUP BY`, over an input
/// that may be empty) — inherits that structural absence, and its target does not
/// count.
///
/// An `UNFOLD` target is not the same case, and does not count: a SEP-0009 `null`
/// element is a legitimate member of the composite, not an error, and the row it yields
/// with the target unbound is a normal answer.
///
/// Erring on the narrow side is otherwise harmless: at worst a feasible order is
/// missed and the query is refused at prepare time with a message that names exactly
/// what could not be bound.
pub(crate) fn collect_certainly_bound(pattern: &GraphPattern, out: &mut DetHashSet<Variable>) {
    collect_certainly_bound_in(pattern, &DetHashSet::default(), out);
}

/// [`collect_certainly_bound`] for a pattern evaluated with `context` already bound:
/// every variable of `context` holds a value in every row the pattern is evaluated
/// over, written into it before it runs.
///
/// That is what the right operand of a `LATERAL` is — evaluated once per left row with
/// that row substituted in — and what a correlated `EXISTS` body is. So a `BIND` there
/// reading only `context` (or what its own inner pattern binds) binds its target on
/// every row: `?s ?p ?v LATERAL { BIND(?v AS ?q) }` binds `?q` exactly as
/// `?s ?p ?v BIND(?v AS ?q)` does. `context` itself is never added to `out` — the
/// pattern does not bind it, its enclosing context does.
///
/// `context` reaches everywhere the substitution does: both operands of a `Join`, a
/// `UNION`, an `OPTIONAL` and a `MINUS`, every wrapper's inner pattern, and the inner
/// pattern of a sub-`SELECT` for the variables it projects — a variable it does not
/// project is a different variable inside it. A nested `LATERAL`'s right operand sees
/// `context` and its own left operand's certain bindings.
pub(crate) fn collect_certainly_bound_in(
    pattern: &GraphPattern,
    context: &DetHashSet<Variable>,
    out: &mut DetHashSet<Variable>,
) {
    collect_bound(pattern, context, Written::NOTHING, out);
}

/// What a prepared execution's run writes into a pattern the planner is judging, beyond
/// the pattern's own bindings — the two ways a promised parameter reaches an
/// expression. See [`Promise`] for where each holds.
#[derive(Clone, Copy, Debug)]
struct Written<'a> {
    /// The parameters the `VALUES` seed binds, when the pattern is on the
    /// solution-modifier descent the seed is joined beneath: they are bound in every
    /// row of the core pattern, so every wrapper between the pattern and the core —
    /// a `BIND` above the core, say — reads them bound. `None` below the core, where
    /// the seed is a sibling joined afterwards.
    seed: Option<&'a DetHashSet<Variable>>,
    /// The parameters the SHACL pre-binding rewrite writes into EVERY expression of
    /// the query (a constant for an IRI or a literal, a driven value for a blank node
    /// or a quoted triple — `crate::substitute`'s `drive_expression_reads`), sub-`SELECT`s
    /// that do not project them included.
    everywhere: Option<&'a DetHashSet<Variable>>,
}

impl Written<'_> {
    /// Nothing is written: the pattern is judged by its own bindings and its context.
    const NOTHING: Self = Self {
        seed: None,
        everywhere: None,
    };

    /// What reaches a child of a node that is not a solution-modifier wrapper: the
    /// seed is joined at or above that node, so never beneath it.
    const fn beneath(self) -> Self {
        Self {
            seed: None,
            everywhere: self.everywhere,
        }
    }

    /// Whether an expression reads `variable` as a value the run wrote in.
    fn writes(self, variable: &Variable) -> bool {
        self.everywhere
            .is_some_and(|parameters| parameters.contains(variable))
    }
}

/// [`collect_certainly_bound_in`], with what the run writes in — see [`Written`].
fn collect_bound(
    pattern: &GraphPattern,
    context: &DetHashSet<Variable>,
    written: Written<'_>,
    out: &mut DetHashSet<Variable>,
) {
    let beneath = written.beneath();
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
        GraphPattern::Join { left, right } => {
            collect_bound(left, context, beneath, out);
            collect_bound(right, context, beneath, out);
        }
        // The right operand is evaluated once per left row with that row in hand, so it
        // sees the left operand's certain bindings as well as the enclosing context's.
        GraphPattern::Lateral { left, right } => {
            let mut left_bound = DetHashSet::default();
            collect_bound(left, context, beneath, &mut left_bound);
            let mut right_context = context.clone();
            right_context.extend(left_bound.iter().cloned());
            collect_bound(right, &right_context, beneath, out);
            out.extend(left_bound);
        }
        // The right side may contribute nothing to a row.
        GraphPattern::LeftJoin { left, .. } | GraphPattern::Minus { left, right: _ } => {
            collect_bound(left, context, beneath, out);
        }
        // Only what BOTH branches bind is bound in every row.
        GraphPattern::Union { left, right } => {
            let mut l = DetHashSet::default();
            let mut r = DetHashSet::default();
            collect_bound(left, context, beneath, &mut l);
            collect_bound(right, context, beneath, &mut r);
            out.extend(l.intersection(&r).cloned());
        }
        // A `FILTER` passes only rows its condition is true on, and every variable the
        // condition requires bound for that is therefore bound in each row it passes —
        // see [`truth_requires`]. A variable the run writes into the condition itself
        // (under the SHACL pre-binding rewrite) is read there as the written value, not
        // from the row, so the condition constrains nothing about the row's binding of
        // it.
        GraphPattern::Filter { expr, inner } => {
            collect_bound(inner, context, written, out);
            out.extend(
                truth_requires(expr)
                    .into_iter()
                    .filter(|variable| !written.writes(variable)),
            );
        }
        // The solution-modifier wrappers the seed descends through keep `written`.
        GraphPattern::OrderBy {
            inner,
            expression: _,
        }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. }
        // `UNFOLD`s own targets are NOT certainly bound: a SEP-0009 `null` element
        // (or a null map value) yields the row with that variable unbound, so only
        // what the inner pattern certainly binds escapes.
        | GraphPattern::Unfold { inner, .. } => collect_bound(inner, context, written, out),
        // A `BIND`'s target counts when its expression reads only what the inner
        // pattern certainly binds, what the enclosing context already holds, or what
        // the run writes in: it is then unbound only in a row whose expression errored
        // on the data, and that row is refused per row by the evaluator rather than
        // invoked free. See [`collect_certainly_bound`]'s doc for the whole argument.
        GraphPattern::Extend {
            inner,
            variable,
            expression,
        } => {
            let mut inner_bound = DetHashSet::default();
            collect_bound(inner, context, written, &mut inner_bound);
            if expression_reads_only_bound(expression, &|read| {
                inner_bound.contains(read) || context.contains(read) || written.writes(read)
            }) {
                out.insert(variable.clone());
            }
            out.extend(inner_bound);
        }
        GraphPattern::Graph { name, inner } => {
            if let NamedNodePattern::Variable(variable) = name {
                out.insert(variable.clone());
            }
            collect_bound(inner, context, beneath, out);
        }
        // Only what the projection keeps escapes, and only if the inner pattern bound
        // it certainly. The context reaches the inner pattern only through the
        // variables the projection names; what the SHACL pre-binding rewrite writes is
        // written past the projection too.
        GraphPattern::Project { inner, variables } => {
            let inner_context = narrowed_to(context, variables);
            let mut inner_bound = DetHashSet::default();
            collect_bound(inner, &inner_context, written, &mut inner_bound);
            out.extend(
                variables
                    .iter()
                    .filter(|variable| inner_bound.contains(*variable))
                    .cloned(),
            );
        }
        // A grouping key is bound in a group's row when every row of the group binds
        // it; an aggregate's output by [`aggregate_certainly_binds`].
        GraphPattern::Group {
            inner,
            variables,
            aggregates,
        } => {
            let mut inner_bound = DetHashSet::default();
            collect_bound(inner, context, written, &mut inner_bound);
            let row_binds =
                |read: &Variable| inner_bound.contains(read) || context.contains(read);
            out.extend(variables.iter().filter(|key| row_binds(key)).cloned());
            let grouped = !variables.is_empty();
            for (variable, aggregate) in aggregates {
                if aggregate_certainly_binds(aggregate, grouped, &|read| {
                    row_binds(read) || written.writes(read)
                }) {
                    out.insert(variable.clone());
                }
            }
        }
        // A `VALUES` column binds its variable in every row exactly when no row holds
        // `UNDEF` there. An empty table produces no row at all, so every column of it
        // qualifies vacuously — and nothing downstream is ever invoked from it.
        GraphPattern::Values {
            variables,
            bindings,
        } => {
            for (column, variable) in variables.iter().enumerate() {
                if bindings
                    .iter()
                    .all(|row| row.get(column).is_some_and(Option::is_some))
                {
                    out.insert(variable.clone());
                }
            }
        }
        // A remote endpoint may omit a column, so it promises nothing.
        GraphPattern::Service { .. } => {}
    }
    // The core the seed is joined onto — the first node that is not a wrapper — binds
    // the seed's parameters in every row it produces.
    if let Some(seed) = written.seed
        && !is_descent_wrapper(pattern)
    {
        out.extend(seed.iter().cloned());
    }
}

/// Whether `pattern` is a solution-modifier wrapper the `VALUES` seed is joined
/// BENEATH — the same wrappers `Query::map_core_pattern` descends, and
/// [`map_children`] hands the descent on through.
const fn is_descent_wrapper(pattern: &GraphPattern) -> bool {
    matches!(
        pattern,
        GraphPattern::Project { .. }
            | GraphPattern::Distinct { .. }
            | GraphPattern::Reduced { .. }
            | GraphPattern::Slice { .. }
            | GraphPattern::OrderBy { .. }
            | GraphPattern::Group { .. }
            | GraphPattern::Extend { .. }
            | GraphPattern::Filter { .. }
            | GraphPattern::Unfold { .. }
    )
}

/// Whether an aggregate's output is bound in every row its `GROUP BY` produces, given
/// `row_binds` — what every row of every group binds.
///
/// # The rule, and why each case is sound
///
/// * `COUNT` — always. It answers `0` for an empty group, including the one group an
///   aggregate without `GROUP BY` produces over an empty input, and never errors.
/// * `SAMPLE`, `MIN`, `MAX` — only under an explicit `GROUP BY`, and only over an
///   argument that yields a value on every row: one reading only variables `row_binds`
///   holds, and unable to error on them ([`cannot_error`]) — a variable, a constant,
///   `BOUND`, `EXISTS`, `sameTerm`, a type test, `COALESCE` with such an argument, and
///   the boolean and conditional forms over such arguments. A group a `GROUP BY`
///   produces holds at least one row, so the argument yields at least one value;
///   `SAMPLE` answers the first, and `MIN`/`MAX` fold under the total SPARQL term
///   order, which orders any two terms — neither can answer unbound over a non-empty
///   list. WITHOUT a `GROUP BY` the one implicit group exists even over an empty
///   input, where all three answer unbound, so none of them counts.
/// * `SUM`, `AVG` — never: a non-numeric value (or an arithmetic failure) poisons the
///   fold to unbound, and whether one occurs is a fact about the data.
/// * `GROUP_CONCAT` — never: a blank node or a quoted triple has no lexical form, and
///   one among the values poisons the fold to unbound.
/// * A custom aggregate and `FOLD` — never: their answers are the host's and the
///   composite's, and neither promises one.
///
/// An argument that CAN error on bound input does not count, even when it reads only
/// bound variables: it can error on every row of a group, and a group whose every
/// argument errored has no value. `STR(?x)` errors on a blank node or a quoted triple,
/// `IRI(STR(?x))` on a string that is not an IRI, `?x + 1` on a non-number — whether
/// one does is a fact about the data, and the aggregate over it is then unbound in that
/// group, so `MAX(STR(?q))` and `SAMPLE(IRI(STR(?q)))` are not sources.
/// `SAMPLE(COALESCE(IRI(STR(?q)), ?q))` is: its last argument cannot error.
fn aggregate_certainly_binds(
    aggregate: &AggregateExpression,
    grouped: bool,
    row_binds: &dyn Fn(&Variable) -> bool,
) -> bool {
    let total = |expr: &Expression| cannot_error(expr, row_binds);
    match aggregate.function() {
        AggregateFunction::Count => true,
        AggregateFunction::Sample | AggregateFunction::Min | AggregateFunction::Max => {
            grouped && aggregate.args().iter().all(total)
        }
        AggregateFunction::Sum
        | AggregateFunction::Avg
        | AggregateFunction::GroupConcat
        | AggregateFunction::Custom(_)
        | AggregateFunction::Fold => false,
    }
}

/// The part of `context` a sub-`SELECT` projecting `variables` lets into its inner
/// pattern: an enclosing binding is substituted into it only for a variable it
/// projects.
fn narrowed_to(context: &DetHashSet<Variable>, variables: &[Variable]) -> DetHashSet<Variable> {
    context
        .iter()
        .filter(|variable| variables.contains(*variable))
        .cloned()
        .collect()
}

/// Whether `expr` can be left without a value only by the data it reads — never
/// because the query text leaves a variable it reads unbound.
///
/// `is_bound` answers for each variable `expr` reads whether it holds a value in every
/// row `expr` is evaluated over: what those rows certainly bind, what the enclosing
/// context holds, or what the run writes in. A variable read outside all three — the
/// right side of an `OPTIONAL`, an `UNDEF` column, an aggregate's output over a
/// possibly empty group — makes the expression error on exactly the rows the text
/// leaves it unbound in, which is a structural absence, not a per-row one.
///
/// Exact where an operator's evaluation reads every operand (arithmetic, comparisons,
/// function calls), and narrow where it may not: `||`, `&&`, `IF` and `IN` require
/// every operand to qualify even though SPARQL's error-masking can sometimes spare one,
/// while `COALESCE` qualifies when any argument does, because it answers with the first
/// argument that evaluates. `BOUND` and `EXISTS` never error on an unbound variable, so
/// they always qualify.
fn expression_reads_only_bound(expr: &Expression, is_bound: &dyn Fn(&Variable) -> bool) -> bool {
    let reads = |expr: &Expression| expression_reads_only_bound(expr, is_bound);
    match expr {
        Expression::NamedNode(_)
        | Expression::Literal(_)
        | Expression::Bound(_)
        | Expression::Exists(_) => true,
        Expression::Variable(variable) => is_bound(variable),
        Expression::Or(a, b)
        | Expression::And(a, b)
        | Expression::Equal(a, b)
        | Expression::SameTerm(a, b)
        | Expression::Greater(a, b)
        | Expression::GreaterOrEqual(a, b)
        | Expression::Less(a, b)
        | Expression::LessOrEqual(a, b)
        | Expression::Add(a, b)
        | Expression::Subtract(a, b)
        | Expression::Multiply(a, b)
        | Expression::Divide(a, b) => reads(a) && reads(b),
        Expression::UnaryPlus(a) | Expression::UnaryMinus(a) | Expression::Not(a) => reads(a),
        Expression::If(condition, then, otherwise) => {
            reads(condition) && reads(then) && reads(otherwise)
        }
        Expression::In(needle, haystack) => reads(needle) && haystack.iter().all(reads),
        Expression::Coalesce(items) => items.iter().any(reads),
        Expression::FunctionCall(_, args) => args.iter().all(reads),
    }
}

/// Whether `expr` yields a value on every row on which `is_bound` holds for every
/// variable it reads — it can neither read an unbound variable nor error on a bound one.
///
/// Narrow on purpose, and exact for what it admits:
///
/// * a constant, a variable `is_bound` holds, `BOUND(…)` and `EXISTS { … }` — the last
///   two answer a boolean for any row;
/// * `sameTerm(a, b)` and a type test (`isIRI`, `isURI`, `isBlank`, `isLiteral`,
///   `isNumeric`, `isTRIPLE`) over arguments that cannot error: each compares or
///   classifies any two terms, or any one, and answers a boolean;
/// * `!`, `&&` and `||` over [`boolean_cannot_error`] operands: their effective boolean
///   value is defined for a boolean, and it is an error for an IRI or a blank node, so
///   an operand must be known to be a boolean;
/// * `IF(c, t, e)` with such a condition and branches that cannot error;
/// * `COALESCE(…)` with any argument that cannot error: it answers the first argument
///   that evaluates.
///
/// Everything else — `=` and the orderings, arithmetic, `STR`, `IRI`, `IN`, every other
/// function — can error on some bound input, and does not qualify.
fn cannot_error(expr: &Expression, is_bound: &dyn Fn(&Variable) -> bool) -> bool {
    match expr {
        Expression::NamedNode(_)
        | Expression::Literal(_)
        | Expression::Bound(_)
        | Expression::Exists(_) => true,
        Expression::Variable(variable) => is_bound(variable),
        Expression::SameTerm(a, b) => cannot_error(a, is_bound) && cannot_error(b, is_bound),
        Expression::FunctionCall(function, args) => {
            is_type_test(function) && args.iter().all(|arg| cannot_error(arg, is_bound))
        }
        Expression::Not(_) | Expression::And(..) | Expression::Or(..) => {
            boolean_cannot_error(expr, is_bound)
        }
        Expression::If(condition, then, otherwise) => {
            boolean_cannot_error(condition, is_bound)
                && cannot_error(then, is_bound)
                && cannot_error(otherwise, is_bound)
        }
        Expression::Coalesce(items) => items.iter().any(|item| cannot_error(item, is_bound)),
        Expression::Equal(..)
        | Expression::Greater(..)
        | Expression::GreaterOrEqual(..)
        | Expression::Less(..)
        | Expression::LessOrEqual(..)
        | Expression::Add(..)
        | Expression::Subtract(..)
        | Expression::Multiply(..)
        | Expression::Divide(..)
        | Expression::UnaryPlus(_)
        | Expression::UnaryMinus(_)
        | Expression::In(..) => false,
    }
}

/// Whether `expr` [`cannot_error`] AND answers a boolean, so its effective boolean value
/// is defined: `BOUND`, `EXISTS`, `sameTerm`, a type test, a boolean literal, and `!`,
/// `&&`, `||` over such operands.
fn boolean_cannot_error(expr: &Expression, is_bound: &dyn Fn(&Variable) -> bool) -> bool {
    match expr {
        Expression::Literal(literal) => {
            literal.datatype().as_str() == "http://www.w3.org/2001/XMLSchema#boolean"
                && matches!(literal.value(), "true" | "false")
        }
        Expression::Bound(_) | Expression::Exists(_) => true,
        Expression::SameTerm(..) | Expression::FunctionCall(..) => cannot_error(expr, is_bound),
        Expression::Not(a) => boolean_cannot_error(a, is_bound),
        Expression::And(a, b) | Expression::Or(a, b) => {
            boolean_cannot_error(a, is_bound) && boolean_cannot_error(b, is_bound)
        }
        _ => false,
    }
}

/// Whether `function` is a type test: total over terms, answering a boolean.
const fn is_type_test(function: &purrdf_sparql_algebra::Function) -> bool {
    use purrdf_sparql_algebra::Function;
    matches!(
        function,
        Function::IsIri
            | Function::IsUri
            | Function::IsBlank
            | Function::IsLiteral
            | Function::IsNumeric
            | Function::IsTriple
    )
}

/// The variables `expr` must have bound for it to be TRUE — which a `FILTER` over `expr`
/// therefore binds in every row it passes.
///
/// # The rule, and why each case is sound
///
/// A variable read where the expression has no value unless it is bound — an operand of
/// `=`, `sameTerm`, a comparison or arithmetic, or the expression itself — errors when
/// unbound, and an error is not true. So:
///
/// * `?v` alone, `BOUND(?v)`: `?v`;
/// * `a && b`: both are true, so the union of what each requires;
/// * `a || b`: at least one is true, and which is not known, so the INTERSECTION of
///   what each requires — `BOUND(?a) || BOUND(?b)` binds neither;
/// * `!a`: `a` is false — what `a` needs to be false ([`falsity_requires`]), so
///   `!BOUND(?v)` binds nothing and `!!BOUND(?v)` binds `?v`;
/// * a type test `isIRI(a)` and its kin: `a` has a value of that type, so what `a`
///   needs for a value;
/// * `IF(c, t, e)`: `c` has a value, and one of the branches is true;
/// * `COALESCE(…)`: the first argument with a value is true, so the intersection over
///   the arguments;
/// * `a IN (…)`: `a` has a value;
/// * every other operator and comparison: its value needs its operands' values, so
///   what the whole expression needs for a value.
///
/// `EXISTS` requires nothing, and a function this crate does not classify is taken to
/// require nothing — narrow, never wrong.
fn truth_requires(expr: &Expression) -> DetHashSet<Variable> {
    match expr {
        Expression::Variable(variable) | Expression::Bound(variable) => {
            std::iter::once(variable.clone()).collect()
        }
        Expression::And(a, b) => {
            let mut both = truth_requires(a);
            both.extend(truth_requires(b));
            both
        }
        Expression::Or(a, b) => intersection(truth_requires(a), &truth_requires(b)),
        Expression::Not(a) => falsity_requires(a),
        Expression::FunctionCall(function, args) if is_type_test(function) => {
            args.iter().flat_map(value_requires).collect()
        }
        Expression::If(condition, then, otherwise) => {
            let mut required = value_requires(condition);
            required.extend(intersection(
                truth_requires(then),
                &truth_requires(otherwise),
            ));
            required
        }
        Expression::Coalesce(items) => items
            .iter()
            .map(truth_requires)
            .reduce(|left, right| intersection(left, &right))
            .unwrap_or_default(),
        Expression::In(needle, _) => value_requires(needle),
        _ => value_requires(expr),
    }
}

/// The variables `expr` must have bound for it to be FALSE — the dual of
/// [`truth_requires`], which reads it through `!`.
///
/// `BOUND(?v)` is false exactly when `?v` is unbound, and a type test is false on an
/// unbound argument too, so neither requires anything; `EXISTS` requires nothing. `!a`
/// is false when `a` is true ([`truth_requires`]); `a && b` when at least one is
/// false, so what both require; `a || b` when both are, so what either requires.
/// `IF`, `COALESCE` and `IN` follow [`truth_requires`]'s reasoning, and every other
/// expression needs what it needs for any value ([`value_requires`]).
fn falsity_requires(expr: &Expression) -> DetHashSet<Variable> {
    match expr {
        Expression::Variable(variable) => std::iter::once(variable.clone()).collect(),
        Expression::Bound(_) | Expression::Exists(_) => DetHashSet::default(),
        Expression::FunctionCall(function, _) if is_type_test(function) => DetHashSet::default(),
        Expression::Not(a) => truth_requires(a),
        Expression::And(a, b) => intersection(falsity_requires(a), &falsity_requires(b)),
        Expression::Or(a, b) => {
            let mut both = falsity_requires(a);
            both.extend(falsity_requires(b));
            both
        }
        Expression::If(condition, then, otherwise) => {
            let mut required = value_requires(condition);
            required.extend(intersection(
                falsity_requires(then),
                &falsity_requires(otherwise),
            ));
            required
        }
        Expression::Coalesce(items) => items
            .iter()
            .map(falsity_requires)
            .reduce(|left, right| intersection(left, &right))
            .unwrap_or_default(),
        Expression::In(needle, _) => value_requires(needle),
        _ => value_requires(expr),
    }
}

/// The variables `expr` must have bound for it to have any value at all — see
/// [`truth_requires`].
///
/// `BOUND`, `EXISTS` and a type test answer for any row. `&&` and `||` can answer
/// with one operand in error (`false && error` is false, `true || error` is true), so
/// they need only what BOTH operands need; `IF` needs its condition and what both
/// branches need; `COALESCE` what every argument needs; `IN` its needle. Every other
/// operator — a comparison, `sameTerm`, arithmetic — needs every operand. A function
/// call other than a type test needs nothing here: this crate does not classify how
/// each treats an unbound argument, and requiring nothing is the narrow answer.
fn value_requires(expr: &Expression) -> DetHashSet<Variable> {
    match expr {
        Expression::Variable(variable) => std::iter::once(variable.clone()).collect(),
        Expression::NamedNode(_)
        | Expression::Literal(_)
        | Expression::Bound(_)
        | Expression::Exists(_)
        | Expression::FunctionCall(..) => DetHashSet::default(),
        Expression::And(a, b) | Expression::Or(a, b) => {
            intersection(value_requires(a), &value_requires(b))
        }
        Expression::Equal(a, b)
        | Expression::SameTerm(a, b)
        | Expression::Greater(a, b)
        | Expression::GreaterOrEqual(a, b)
        | Expression::Less(a, b)
        | Expression::LessOrEqual(a, b)
        | Expression::Add(a, b)
        | Expression::Subtract(a, b)
        | Expression::Multiply(a, b)
        | Expression::Divide(a, b) => {
            let mut both = value_requires(a);
            both.extend(value_requires(b));
            both
        }
        Expression::UnaryPlus(a) | Expression::UnaryMinus(a) | Expression::Not(a) => {
            value_requires(a)
        }
        Expression::If(condition, then, otherwise) => {
            let mut required = value_requires(condition);
            required.extend(intersection(
                value_requires(then),
                &value_requires(otherwise),
            ));
            required
        }
        Expression::Coalesce(items) => items
            .iter()
            .map(value_requires)
            .reduce(|left, right| intersection(left, &right))
            .unwrap_or_default(),
        Expression::In(needle, _) => value_requires(needle),
    }
}

/// `left ∩ right`, reusing `left`.
fn intersection(
    mut left: DetHashSet<Variable>,
    right: &DetHashSet<Variable>,
) -> DetHashSet<Variable> {
    left.retain(|variable| right.contains(variable));
    left
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

/// **The pushdown's promise is the pushdown's reach.** Under the ordinary rewrite a
/// declared parameter counts as bound at a call exactly where the run writes it into
/// that call — so for every shape, a relation serving only the bound mode is admitted
/// with the parameter declared if and only if the rewrite of the plan a free-capable
/// relation is given writes the parameter's value into the call's argument.
///
/// Both halves are read, neither is restated: admission is [`plan_query`] itself, and
/// the write is `crate::substitute::apply_probes` run over the plan [`plan_query`]
/// produced, observed by looking at the call's argument afterwards. A shape the planner
/// admits and the rewrite leaves free would be refused on every run; a shape the
/// rewrite writes into and the planner refuses is refused at prepare although every run
/// would serve it — the drift that refused `?s ?p ?o LATERAL { ?q <rel> ?out }`.
///
/// The corpus holds no `EXISTS`: a call there is reached by the rows it filters rather
/// than by the pushdown, and the integration tests observe that half by invocation.
#[cfg(test)]
mod pushdown_reach_tests {
    use std::sync::Arc;

    use purrdf_core::binding_pattern::BindingPattern;
    use purrdf_sparql_algebra::{
        GraphPattern, GroundTerm, NamedNode, ParserOptions, PropertyFunctionCall, Query,
        SparqlParser, TermPattern, Variable,
    };

    use super::{parameter_set, plan_query};
    use crate::agg_fn::AggregateRegistry;
    use crate::engine::ShaclPrebinding;
    use crate::error::EvalError;
    use crate::property_fn::{
        PfArgs, PfArity, PfCursor, PfRow, PropertyFunction, PropertyFunctionRegistry,
    };
    use crate::user_fn::Volatility;

    const REL: &str = "http://example.org/rel";
    const VALUE: &str = "http://example.org/alpha";

    /// A `(1, 1)` relation declaring `modes`, never dispatched.
    struct Declared(Vec<BindingPattern>);

    struct Empty;

    impl PfCursor for Empty {
        fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
            Ok(None)
        }
    }

    impl PropertyFunction for Declared {
        fn volatility(&self) -> Volatility {
            Volatility::Stable
        }

        fn arity(&self) -> PfArity {
            PfArity::new(1, 1)
        }

        fn modes(&self) -> &[BindingPattern] {
            &self.0
        }

        fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
            1
        }

        fn open(
            &self,
            _args: &PfArgs<'_>,
            _ceiling: Option<u64>,
        ) -> Result<Box<dyn PfCursor>, EvalError> {
            Ok(Box::new(Empty))
        }
    }

    fn registry(modes: &[&str]) -> PropertyFunctionRegistry {
        let mut registry = PropertyFunctionRegistry::new();
        registry.register(
            REL,
            Arc::new(Declared(
                modes
                    .iter()
                    .map(|code| BindingPattern::from_code(code))
                    .collect(),
            )),
        );
        registry
    }

    fn parse(body: &str) -> Query {
        let options = ParserOptions {
            property_fn_iris: vec![REL.to_owned()],
            ..ParserOptions::default()
        };
        SparqlParser::new()
            .parse_query_with(&format!("SELECT * WHERE {{ {body} }}"), &options)
            .unwrap_or_else(|error| panic!("{body} parses: {error}"))
    }

    /// Every call in `pattern` (none of the corpus's shapes holds one in an expression).
    fn calls<'a>(pattern: &'a GraphPattern, out: &mut Vec<&'a PropertyFunctionCall>) {
        match pattern {
            GraphPattern::PropertyFunction(call) => out.push(call),
            GraphPattern::Join { left, right }
            | GraphPattern::Union { left, right }
            | GraphPattern::Lateral { left, right }
            | GraphPattern::LeftJoin { left, right, .. }
            | GraphPattern::Minus { left, right } => {
                calls(left, out);
                calls(right, out);
            }
            GraphPattern::Filter { inner, .. }
            | GraphPattern::Graph { inner, .. }
            | GraphPattern::Extend { inner, .. }
            | GraphPattern::Unfold { inner, .. }
            | GraphPattern::OrderBy { inner, .. }
            | GraphPattern::Project { inner, .. }
            | GraphPattern::Distinct { inner }
            | GraphPattern::Reduced { inner }
            | GraphPattern::Slice { inner, .. }
            | GraphPattern::Group { inner, .. }
            | GraphPattern::Service { inner, .. } => calls(inner, out),
            GraphPattern::Bgp { .. } | GraphPattern::Path { .. } | GraphPattern::Values { .. } => {}
        }
    }

    /// Whether the ordinary rewrite of `body`'s plan writes `?q` into every call.
    fn rewrite_writes(body: &str) -> bool {
        let parameters = parameter_set(&["q"]);
        let query = parse(body);
        let planned = plan_query(
            &query,
            &registry(&["bf", "ff"]),
            &AggregateRegistry::EMPTY,
            &parameters,
            ShaclPrebinding::None,
        )
        .unwrap_or_else(|error| panic!("{body}: a free-capable relation is admitted: {error}"))
        .unwrap_or(query);
        let rewritten = crate::substitute::apply_probes(
            planned,
            vec![(
                Variable::new("q"),
                GroundTerm::NamedNode(NamedNode::new_unchecked(VALUE)),
            )],
        );
        let Query::Select { pattern, .. } = &rewritten else {
            panic!("a SELECT stays one");
        };
        let mut found = Vec::new();
        calls(pattern, &mut found);
        assert!(!found.is_empty(), "{body}: a call");
        let written: Vec<bool> = found
            .iter()
            .map(|call| {
                matches!(&call.subject_args[0], TermPattern::NamedNode(node) if node.as_str() == VALUE)
            })
            .collect();
        assert!(
            written.iter().all(|w| *w == written[0]),
            "{body}: the rewrite writes every call of this corpus's shapes or none — {written:?}"
        );
        written[0]
    }

    /// Whether a bound-only relation is admitted in `body` with `?q` declared.
    fn admitted(body: &str) -> bool {
        plan_query(
            &parse(body),
            &registry(&["bf"]),
            &AggregateRegistry::EMPTY,
            &parameter_set(&["q"]),
            ShaclPrebinding::None,
        )
        .is_ok()
    }

    #[test]
    fn the_ordinary_promise_holds_exactly_where_the_rewrite_writes() {
        let call = format!("?q <{REL}> ?out");
        let atom = "?s <http://example.org/p> ?o";
        let other = "?s2 <http://example.org/p> ?o2";
        // (shape, whether the rewrite reaches the call)
        let corpus: Vec<(String, bool)> = vec![
            (call.clone(), true),
            (format!("{atom} . {call}"), true),
            (format!("{atom} LATERAL {{ {call} }}"), true),
            (format!("{atom} LATERAL {{ LATERAL {{ {call} }} }}"), true),
            // A `LATERAL`'s right side, whatever surrounds the call there, wherever the
            // rewrite's restriction is a join-equivalent one.
            (
                format!("{atom} LATERAL {{ {other} LATERAL {{ {call} }} }}"),
                true,
            ),
            (format!("{atom} LATERAL {{ {other} . {call} }}"), true),
            (format!("{atom} LATERAL {{ BIND(1 AS ?one) {call} }}"), true),
            (
                format!("{atom} LATERAL {{ VALUES ?z {{ 1 }} {call} }}"),
                true,
            ),
            (
                format!("{atom} LATERAL {{ {call} FILTER(BOUND(?out)) }}"),
                true,
            ),
            (
                format!("{atom} LATERAL {{ SELECT ?q ?out WHERE {{ {call} }} }}"),
                true,
            ),
            (
                format!("{atom} LATERAL {{ SELECT * WHERE {{ {call} }} }}"),
                true,
            ),
            (
                format!(
                    "{atom} LATERAL {{ SELECT DISTINCT ?q ?out WHERE {{ {call} }} ORDER BY ?out }}"
                ),
                true,
            ),
            (
                format!("{atom} LATERAL {{ {{ {call} }} UNION {{ {call} }} }}"),
                true,
            ),
            (format!("{atom} LATERAL {{ GRAPH ?g {{ {call} }} }}"), true),
            // ... and where it is not: a sub-`SELECT` that does not project the
            // variable, a slice or a `GROUP BY` beneath the projection, an `OPTIONAL` or
            // a `MINUS` right arm inside the right side.
            (
                format!("{atom} LATERAL {{ SELECT ?out WHERE {{ {call} }} }}"),
                false,
            ),
            (
                format!("{atom} LATERAL {{ SELECT ?q ?out WHERE {{ {call} }} LIMIT 1 }}"),
                false,
            ),
            (
                format!(
                    "{atom} LATERAL {{ SELECT ?q (COUNT(?out) AS ?n) WHERE {{ {call} }} GROUP BY ?q }}"
                ),
                false,
            ),
            (format!("{atom} LATERAL {{ OPTIONAL {{ {call} }} }}"), false),
            (
                format!("{atom} LATERAL {{ {other} OPTIONAL {{ {call} }} }}"),
                false,
            ),
            (
                format!("{atom} LATERAL {{ {other} MINUS {{ {call} }} }}"),
                false,
            ),
            (format!("{{ {atom} }} {{ {call} }}"), true),
            (format!("{{ {{ {call} }} }}"), true),
            (format!("{call} FILTER(?out != 1)"), true),
            (format!("BIND(1 AS ?one) {call}"), true),
            (format!("{call} OPTIONAL {{ {atom} }}"), true),
            (format!("{atom} OPTIONAL {{ {call} }}"), false),
            (format!("{call} MINUS {{ {atom} }}"), true),
            (format!("{atom} MINUS {{ {call} }}"), false),
            (format!("{{ {call} }} UNION {{ {atom} }}"), true),
            (format!("GRAPH ?g {{ {call} }}"), true),
            (
                format!("{{ SELECT ?q ?out WHERE {{ {call} }} LIMIT 1 }}"),
                true,
            ),
            (
                format!("{atom} {{ SELECT ?q ?out WHERE {{ {call} }} }}"),
                true,
            ),
            (
                format!("{atom} {{ SELECT ?out WHERE {{ {call} }} }}"),
                false,
            ),
            (format!("VALUES ?w {{ 1 2 }} {call}"), true),
            (format!("{{ {call} }} LATERAL {{ BIND(1 AS ?one) }}"), true),
        ];
        for (body, reaches) in &corpus {
            assert_eq!(
                rewrite_writes(body),
                *reaches,
                "{body}: the rewrite's reach is what this corpus says"
            );
            assert_eq!(
                admitted(body),
                *reaches,
                "{body}: a bound-only relation is admitted exactly where the rewrite writes \
                 the parameter into its call"
            );
        }
    }
}
