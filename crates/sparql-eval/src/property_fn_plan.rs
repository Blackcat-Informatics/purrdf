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

use purrdf_cdt::CdtFn;
use purrdf_core::ContentDigest;
use purrdf_core::binding_pattern::BindingPattern;
use purrdf_hash::Domain;
use purrdf_sparql_algebra::Child;
use purrdf_sparql_algebra::{
    AggregateExpression, AggregateFunction, Expression, Function, GraphPattern, Literal,
    NamedNodePattern, OrderExpression, PropertyFunctionCall, PurrdfFn, Query, TermPattern,
    TriplePattern, Variable,
};

use crate::DetHashSet;
use crate::agg_fn::{AggregateRegistry, ScalarvalKind, ScalarvalSpec};
use crate::convert::literal_to_value;
use crate::error::EvalError;
use crate::expr::xsd_of;
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

    /// Preserve an operational cause's code, falling back to the ordinary admission
    /// seam only when the cause has no distinct code. The single chokepoint
    /// both `crate::engine::PlanCache::prepare_with_relations` and
    /// `crate::update::delete_insert` read, so the two admission call sites can never
    /// again drift into hard-coding the wrong seam's code.
    pub(crate) fn diagnostic_code(&self) -> &str {
        self.error.code().unwrap_or(match self.seam {
            PlanSeam::PropertyFunction => "native-sparql-property-function",
            PlanSeam::Aggregate => "native-sparql-aggregate-function",
        })
    }
}

impl core::fmt::Display for PlanError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&self.error, f)
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod diagnostic_code_tests {
    use purrdf_core::RdfDiagnostic;
    use purrdf_core::xsd_regex::xpath::{Error, Resource};

    use super::{EvalError, PlanError};

    #[test]
    fn admission_keeps_operational_codes_and_ordinary_seam_fallbacks() {
        for error in [
            EvalError::function_operational("host metadata panic"),
            EvalError::source_read("source refused"),
            EvalError::XPathRegex(Error::Allocation {
                resource: Resource::CompileSlots,
                units: 1,
            }),
        ] {
            let property = PlanError::property_function(error.clone());
            let aggregate = PlanError::aggregate(error.clone());
            assert_eq!(Some(property.diagnostic_code()), error.code());
            assert_eq!(Some(aggregate.diagnostic_code()), error.code());
        }
        assert_eq!(
            PlanError::property_function(EvalError::function("wrong arity")).diagnostic_code(),
            "native-sparql-property-function"
        );
        assert_eq!(
            PlanError::aggregate(EvalError::function("missing registration")).diagnostic_code(),
            "native-sparql-aggregate-function"
        );
    }

    #[test]
    fn a_borrowed_dataset_code_becomes_an_owned_diagnostic() {
        let diagnostic = {
            let error = PlanError::property_function(EvalError::Dataset(RdfDiagnostic::error(
                String::from("caller-storage-failed"),
                "source refused",
            )));
            RdfDiagnostic::error(error.diagnostic_code(), error.to_string())
        };
        assert_eq!(diagnostic.code, "caller-storage-failed");
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
/// charge (see [`Planner::enter_aggregate`]).
pub(crate) fn plan_query(
    query: &Query,
    relations: &PropertyFunctionRegistry,
    agg_registry: &AggregateRegistry,
    parameters: &DetHashSet<Variable>,
) -> Result<Option<Query>, PlanError> {
    plan_query_with(query, relations, agg_registry, parameters, true)
}

/// Immutable prepared plans retain their positive topology. Registry and promised
/// parameter feasibility still run, without normalizing an already admitted tree.
pub(crate) fn recheck_query(
    query: &Query,
    relations: &PropertyFunctionRegistry,
    agg_registry: &AggregateRegistry,
    parameters: &DetHashSet<Variable>,
) -> Result<Option<Query>, PlanError> {
    plan_query_with(query, relations, agg_registry, parameters, false)
}

fn plan_query_with(
    query: &Query,
    relations: &PropertyFunctionRegistry,
    agg_registry: &AggregateRegistry,
    parameters: &DetHashSet<Variable>,
    normalize_positive: bool,
) -> Result<Option<Query>, PlanError> {
    let pattern = match query {
        Query::Select { pattern, .. }
        | Query::Ask { pattern, .. }
        | Query::Construct { pattern, .. }
        | Query::Describe { pattern, .. } => pattern,
    };
    let Some(planned) = plan_where_pattern_with(
        pattern,
        relations,
        agg_registry,
        parameters,
        normalize_positive,
    )?
    else {
        return Ok(None);
    };
    Ok(Some(crate::blank_scope::query_with_pattern(query, planned)))
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
/// `parameters` names the variables a prepared execution binds on every run. The one
/// pre-binding rewrite every run applies writes them into every property-function
/// call in the query, so each counts as bound everywhere ([`Promise::Everywhere`]).
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
) -> Result<Option<GraphPattern>, PlanError> {
    plan_where_pattern_with(pattern, relations, agg_registry, parameters, true)
}

fn plan_where_pattern_with(
    pattern: &GraphPattern,
    relations: &PropertyFunctionRegistry,
    agg_registry: &AggregateRegistry,
    parameters: &DetHashSet<Variable>,
    normalize_positive: bool,
) -> Result<Option<GraphPattern>, PlanError> {
    // A blank node label written in two pieces of one basic graph pattern — a
    // triple and a call, a triple and a path, two calls — is one variable across
    // them. Settled first, and for every pattern whether or not it carries a call,
    // because a call's arguments are admitted below by what is bound, and a
    // shared blank a sibling binds IS bound. See `crate::blank_scope`.
    let joined = crate::blank_scope::join_shared_blanks(pattern);
    let pattern = joined.as_ref().unwrap_or(pattern);
    let normalized = normalize_positive
        .then(|| crate::join_plan::normalize(pattern))
        .flatten();
    let pattern = normalized.as_ref().unwrap_or(pattern);
    // Either hazard alone must still run the walk: a query with a `Custom`
    // aggregate and no property-function call would otherwise skip this pass
    // entirely on the property-function-only check, and its admission (below,
    // as the planner enters the aggregate) would never happen.
    if !crate::property_fn_eval::pattern_needs_admission(pattern) {
        return Ok(normalized.or(joined));
    }
    // The plan is measured against the evaluator's depth envelope before any of it is
    // copied or an admitted call chain is traversed; one that does not fit is refused,
    // typed.
    crate::governor::soundness::validate_graph_pattern_depth(pattern)
        .map_err(PlanError::property_function)?;
    plan_pattern(
        pattern,
        relations,
        agg_registry,
        &DetHashSet::default(),
        Promise::descent(parameters),
    )
    .map(Some)
}

/// Where a prepared execution's declared parameters count as bound while a call is
/// admitted.
///
/// Every run binds its parameters through the one pre-binding rewrite
/// (`crate::substitute::apply_shacl_probes`): the `VALUES` seed and the pushdown, PLUS
/// a walk with no boundary that writes an IRI or literal parameter into every
/// property-function argument and every expression that names it — an `OPTIONAL`'s
/// or a `MINUS`'s right arm, a sub-`SELECT`, an `EXISTS` body — and drives every other
/// value (a blank node, a quoted triple) into the same calls through a one-row
/// `VALUES`. So a call ANYWHERE receives a parameter bound, and admitting it as free
/// would refuse, at prepare, a call every run serves: that is [`Self::Everywhere`].
///
/// A promise never extends `outer`, which stays the set of variables the pattern
/// ITSELF certainly binds: it is consulted only where a call is admitted, and where a
/// `BIND` or an aggregate reads a parameter (see [`Written`]).
#[derive(Clone, Copy, Debug)]
enum Promise<'a> {
    /// Nothing is promised: the execution declares no parameters.
    None,
    /// Anywhere in the query, because every run writes these in.
    Everywhere(&'a DetHashSet<Variable>),
}

impl<'a> Promise<'a> {
    /// The promise a plan starts from: every declared parameter, everywhere, or
    /// nothing when none is declared.
    fn descent(parameters: &'a DetHashSet<Variable>) -> Self {
        if parameters.is_empty() {
            Self::None
        } else {
            Self::Everywhere(parameters)
        }
    }

    /// What the run writes into the rows and expressions of a pattern evaluated at
    /// this node — see [`Written`].
    const fn written(self) -> Written<'a> {
        Written {
            everywhere: self.for_calls(),
        }
    }

    /// The parameters a call admitted at this node may count as bound.
    const fn for_calls(self) -> Option<&'a DetHashSet<Variable>> {
        match self {
            Self::Everywhere(parameters) => Some(parameters),
            Self::None => None,
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
///
/// The rewrite runs on a [`Planner`], over work lists rather than by recursion, so a
/// plan of any depth needs no more machine stack. Every node is entered once — the
/// admission it performs before its parts are planned (a bare call's, a chain's
/// order, a custom aggregate's) runs then — and assembled once every part under it
/// is planned, the parts rebuilt in the order the node's fields are written. The
/// first admission failure met in that order is the one returned.
fn plan_pattern(
    pattern: &GraphPattern,
    relations: &PropertyFunctionRegistry,
    agg_registry: &AggregateRegistry,
    outer: &DetHashSet<Variable>,
    promise: Promise<'_>,
) -> Result<GraphPattern, PlanError> {
    let mut planner = Planner {
        relations,
        agg_registry,
        steps: vec![Step::Pattern(
            pattern,
            SetRef::Given(outer),
            Prom::of(promise),
        )],
        values: Vec::new(),
        scopes: Vec::new(),
    };
    while let Some(step) = planner.steps.pop() {
        match step {
            Step::Pattern(node, scope, promise) => planner.enter_pattern(node, scope, promise)?,
            Step::Expression(expr, scope, promise) => {
                planner.enter_expression(expr, scope, promise);
            }
            Step::Aggregate(aggregate, scope, promise) => {
                planner.enter_aggregate(aggregate, scope, promise)?;
            }
            Step::Order(order, scope, promise) => planner.enter_order(order, scope, promise),
            Step::Assemble(assemble) => planner.assemble(assemble),
        }
    }
    Ok(planner.finish())
}

/// One node of a chain's spine: a call's join, or the residual data written between
/// two calls.
enum ChainNode<'a> {
    /// A `Lateral` whose right operand plans to a call: the operand to its left, then
    /// the node holding the call and the call itself.
    Lateral {
        left: &'a GraphPattern,
        node: &'a GraphPattern,
        call: &'a PropertyFunctionCall,
    },
    /// A `Join` whose right operand is not a bare call.
    Join {
        left: &'a GraphPattern,
        right: &'a GraphPattern,
    },
}

/// `pattern` read as a node of a chain's spine, or `None` when it is not one.
///
/// A `Lateral` whose right operand does not plan to a call is not a chain node: its
/// right operand is evaluated once per left row with that row in hand, a dependency a
/// `Join` does not carry, so flattening it into a chain would rebuild it through a
/// `Join` and evaluate the right operand without the left rows. A call under a `Join`
/// rather than a `Lateral` would lose the dependency the `Lateral` encodes, so it is
/// not a chain member either. The structural rebuild handles both.
fn chain_node(pattern: &GraphPattern) -> Option<ChainNode<'_>> {
    match pattern {
        GraphPattern::Lateral { left, right } => {
            planned_lateral_call(right).map(|(node, call)| ChainNode::Lateral { left, node, call })
        }
        GraphPattern::Join { left, right }
            if !matches!(&**right, GraphPattern::PropertyFunction(_)) =>
        {
            Some(ChainNode::Join { left, right })
        }
        _ => None,
    }
}

/// Peel the chain spine, pushing its atoms in TEXTUAL order (base first).
///
/// Returns whether `pattern` is a chain node at all: a bare `Bgp` or any other leaf is
/// not, so an ordinary query never allocates past the empty vector above.
///
/// The peel runs over a work list, so a spine of any length needs no more machine
/// stack: a chain node's operands are pushed to be peeled in turn, an operand that is
/// itself a chain node is peeled the same way, and any other operand is an atom.
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
/// chain node: see [`chain_node`].
fn collect_chain<'a>(pattern: &'a GraphPattern, atoms: &mut Vec<Atom<'a>>) -> bool {
    /// One pending step of the peel.
    enum Step<'a> {
        /// A chain node whose operands are still to be peeled.
        Node(ChainNode<'a>),
        /// An operand: a chain node to peel further, or an atom.
        Operand(&'a GraphPattern),
        /// A call, pushed once everything written before it is.
        Call(&'a GraphPattern, &'a PropertyFunctionCall),
    }
    let Some(root) = chain_node(pattern) else {
        return false;
    };
    let mut pending = vec![Step::Node(root)];
    while let Some(step) = pending.pop() {
        match step {
            Step::Node(ChainNode::Lateral { left, node, call }) => {
                pending.push(Step::Call(node, call));
                pending.push(Step::Operand(left));
            }
            Step::Node(ChainNode::Join { left, right }) => {
                pending.push(Step::Operand(right));
                pending.push(Step::Operand(left));
            }
            Step::Operand(operand) => match chain_node(operand) {
                Some(node) => pending.push(Step::Node(node)),
                None => push_atom(operand, None, atoms),
            },
            Step::Call(node, call) => push_atom(node, Some(call), atoms),
        }
    }
    true
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
/// under the promise the pushdown keeps there. The wrappers are peeled in a loop, one
/// per iteration.
fn planned_lateral_call(right: &GraphPattern) -> Option<(&GraphPattern, &PropertyFunctionCall)> {
    let mut operand = right;
    while let GraphPattern::Lateral { left, right } = operand
        && left.is_empty_bgp()
    {
        operand = right;
    }
    crate::substitute::lateral_call(operand).map(|call| (operand, call))
}

/// Push one chain member, dropping the EMPTY `Bgp` the parser leaves where a call opens
/// its block. It is the identity table `Z`, so joining it back in would only widen the
/// rebuilt tree with a node that binds nothing and matches everything.
fn push_atom<'a>(
    pattern: &'a GraphPattern,
    call: Option<&'a PropertyFunctionCall>,
    atoms: &mut Vec<Atom<'a>>,
) {
    if pattern.is_empty_bgp() {
        return;
    }
    let position = atoms.len();
    atoms.push(Atom {
        pattern,
        call,
        position,
    });
}

/// One chain's feasible order: its atoms as they re-attach, each with whether it is a
/// call and — for the calls — the variables bound when it was chosen.
struct ChainOrder<'a> {
    /// The atoms, in the chosen order.
    ordered: Vec<&'a GraphPattern>,
    /// Whether each atom of `ordered` is a call.
    is_call: Vec<bool>,
    /// The set `bound` held at the moment each atom was CHOSEN — i.e. exactly the
    /// variables a CALL atom is driven with when it is planned. Captured here (rather
    /// than read from the fully-accumulated `bound` after the loop) is what keeps a call
    /// from being admitted as though sibling atoms chosen AFTER it — and everything they
    /// bind — were already in scope.
    bound_before: Vec<DetHashSet<Variable>>,
}

/// The greedy feasibility order over one chain's atoms.
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
///
/// The atoms are then planned each in turn and the spine rebuilt in this order by the
/// [`Planner`] ([`Assemble::Chain`]).
fn order_chain<'a>(
    atoms: Vec<Atom<'a>>,
    relations: &PropertyFunctionRegistry,
    outer: &DetHashSet<Variable>,
    promise: Promise<'_>,
) -> Result<ChainOrder<'a>, PlanError> {
    let mut bound = outer.clone();
    let mut remaining: Vec<Atom<'a>> = atoms;
    let mut ordered: Vec<&'a GraphPattern> = Vec::with_capacity(remaining.len());
    let mut is_call: Vec<bool> = Vec::with_capacity(remaining.len());
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
    Ok(ChainOrder {
        ordered,
        is_call,
        bound_before,
    })
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
/// bound only when every component is, its nested triples walked over a work list.
fn term_is_bound(term: &TermPattern, bound: &DetHashSet<Variable>) -> bool {
    let mut pending: purrdf_core::SmallVec<[_; 8]> = purrdf_core::smallvec![term];
    while let Some(term) = pending.pop() {
        match term {
            TermPattern::NamedNode(_) | TermPattern::Literal(_) => {}
            TermPattern::BlankNode(_) => return false,
            TermPattern::Variable(variable) => {
                if !bound.contains(variable) {
                    return false;
                }
            }
            TermPattern::Triple(triple) => {
                if let NamedNodePattern::Variable(variable) = &triple.predicate
                    && !bound.contains(variable)
                {
                    return false;
                }
                pending.push(&triple.object);
                pending.push(&triple.subject);
            }
        }
    }
    true
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
// The rebuild
// ---------------------------------------------------------------------------

/// A variable set a planner frame reads: one [`plan_pattern`]'s caller holds, or one a
/// node built for the parts under it and the planner owns ([`Planner::scopes`]).
#[derive(Clone, Copy)]
enum SetRef<'a> {
    /// A set the caller holds.
    Given(&'a DetHashSet<Variable>),
    /// The set at this index of the planner's arena.
    Owned(usize),
}

impl<'a> SetRef<'a> {
    /// The set itself, read from `scopes` when the planner owns it.
    fn get<'s>(self, scopes: &'s [DetHashSet<Variable>]) -> &'s DetHashSet<Variable>
    where
        'a: 's,
    {
        match self {
            Self::Given(set) => set,
            Self::Owned(index) => &scopes[index],
        }
    }
}

/// A [`Promise`] whose parameter set is a [`SetRef`]: what a planner frame stores, since
/// a set a node narrowed for the parts under it lives in the planner's arena, and a
/// frame cannot borrow from an arena that grows while the frame waits.
#[derive(Clone, Copy)]
enum Prom<'a> {
    /// [`Promise::None`].
    None,
    /// [`Promise::Everywhere`].
    Everywhere(SetRef<'a>),
}

impl<'a> Prom<'a> {
    /// `promise`, its set held as given.
    const fn of(promise: Promise<'a>) -> Self {
        match promise {
            Promise::None => Self::None,
            Promise::Everywhere(parameters) => Self::Everywhere(SetRef::Given(parameters)),
        }
    }

    /// The [`Promise`] this stands for, its set read from `scopes` when the planner owns
    /// it.
    fn view<'s>(self, scopes: &'s [DetHashSet<Variable>]) -> Promise<'s>
    where
        'a: 's,
    {
        match self {
            Self::None => Promise::None,
            Self::Everywhere(parameters) => Promise::Everywhere(parameters.get(scopes)),
        }
    }
}

/// One planned part on the planner's value stack.
enum Planned {
    Pattern(GraphPattern),
    Expression(Expression),
    Aggregate(AggregateExpression),
    Order(OrderExpression),
}

impl Planned {
    /// The pattern this part is; a pattern is planned wherever one was entered.
    fn pattern(self) -> GraphPattern {
        match self {
            Self::Pattern(pattern) => pattern,
            Self::Expression(_) | Self::Aggregate(_) | Self::Order(_) => {
                unreachable!("a pattern was entered here, so a pattern is planned here")
            }
        }
    }

    /// The expression this part is; an expression is planned wherever one was entered.
    fn expression(self) -> Expression {
        match self {
            Self::Expression(expression) => expression,
            Self::Pattern(_) | Self::Aggregate(_) | Self::Order(_) => {
                unreachable!("an expression was entered here, so an expression is planned here")
            }
        }
    }

    /// The aggregate this part is; an aggregate is planned wherever one was entered.
    fn aggregate(self) -> AggregateExpression {
        match self {
            Self::Aggregate(aggregate) => aggregate,
            Self::Pattern(_) | Self::Expression(_) | Self::Order(_) => {
                unreachable!("an aggregate was entered here, so an aggregate is planned here")
            }
        }
    }

    /// The sort key this part is; a sort key is planned wherever one was entered.
    fn order(self) -> OrderExpression {
        match self {
            Self::Order(order) => order,
            Self::Pattern(_) | Self::Expression(_) | Self::Aggregate(_) => {
                unreachable!("a sort key was entered here, so a sort key is planned here")
            }
        }
    }
}

/// The next planned pattern of `parts`, which holds one for every pattern entered.
fn next_pattern(parts: &mut impl Iterator<Item = Planned>) -> GraphPattern {
    parts
        .next()
        .expect("every part a node entered is planned before the node is assembled")
        .pattern()
}

/// The next planned expression of `parts`, which holds one for every expression entered.
fn next_expression(parts: &mut impl Iterator<Item = Planned>) -> Expression {
    parts
        .next()
        .expect("every part a node entered is planned before the node is assembled")
        .expression()
}

/// One step of the planner's work list.
enum Step<'a> {
    /// Enter a pattern: admit what it admits before its parts, and push them.
    Pattern(&'a GraphPattern, SetRef<'a>, Prom<'a>),
    /// Enter an expression: plan it as written, or push its operands.
    Expression(&'a Expression, SetRef<'a>, Prom<'a>),
    /// Enter an aggregate: admit a custom one, and push its arguments and sort keys.
    Aggregate(&'a AggregateExpression, SetRef<'a>, Prom<'a>),
    /// Enter a sort key: push its expression.
    Order(&'a OrderExpression, SetRef<'a>, Prom<'a>),
    /// Rebuild a node from the planned parts under it.
    Assemble(Assemble<'a>),
}

/// A node to rebuild once every part under it is planned.
enum Assemble<'a> {
    /// A non-chain pattern node. The arena is cut back to `mark`, releasing the sets the
    /// node built for its parts.
    Pattern { node: &'a GraphPattern, mark: usize },
    /// A chain: its atoms re-attach in the chosen order, a call through a `Lateral` (it
    /// depends on what is to its left), everything else through a `Join`. `is_call`
    /// says which each atom is; the arena is cut back to `mark`.
    Chain { is_call: Vec<bool>, mark: usize },
    /// An expression, over its planned operands.
    Expression(&'a Expression),
    /// An aggregate, over its planned arguments and sort keys.
    Aggregate(&'a AggregateExpression),
    /// A sort key, over its planned expression.
    Order(&'a OrderExpression),
}

/// The feasibility rewrite's state: a step stack, a value stack of the planned parts,
/// and an arena of the variable sets nodes build for the parts under them.
///
/// The arena is used as a stack. A node pushes the sets its parts read when it is
/// entered and cuts the arena back when it is assembled, and every part is assembled
/// before the node it is under, so a set is released exactly when the last frame
/// reading it is gone.
struct Planner<'a, 'r> {
    relations: &'r PropertyFunctionRegistry,
    agg_registry: &'r AggregateRegistry,
    steps: Vec<Step<'a>>,
    values: Vec<Planned>,
    scopes: Vec<DetHashSet<Variable>>,
}

impl<'a> Planner<'a, '_> {
    /// Own `set` in the arena, and the reference to read it by.
    fn own(&mut self, set: DetHashSet<Variable>) -> SetRef<'a> {
        self.scopes.push(set);
        SetRef::Owned(self.scopes.len() - 1)
    }

    /// Widen `scope` by what `pattern` certainly binds under `promise`: the scope an
    /// expression evaluated over `pattern`'s rows sees.
    fn widen(
        &self,
        scope: &mut DetHashSet<Variable>,
        outer: SetRef<'a>,
        pattern: &GraphPattern,
        promise: Prom<'a>,
    ) {
        collect_bound(
            pattern,
            outer.get(&self.scopes),
            promise.view(&self.scopes).written(),
            scope,
        );
    }

    /// The top `count` planned parts, in the order they were planned.
    fn take(&mut self, count: usize) -> std::vec::IntoIter<Planned> {
        let at = self.values.len() - count;
        self.values.split_off(at).into_iter()
    }

    /// The root's plan: the one value left once every step has run.
    fn finish(mut self) -> GraphPattern {
        let root = self
            .values
            .pop()
            .expect("the root is assembled last")
            .pattern();
        debug_assert!(
            self.values.is_empty(),
            "every planned part is consumed by the node it is under"
        );
        root
    }

    /// Enter `node`: admit a bare call and plan it as written; order a chain and push its
    /// atoms; or push a structural node's parts.
    fn enter_pattern(
        &mut self,
        node: &'a GraphPattern,
        scope: SetRef<'a>,
        promise: Prom<'a>,
    ) -> Result<(), PlanError> {
        // Compiler-produced algebra may be a bare call, without the parser's Lateral
        // wrapper. Apply the same admission as a chain member before cloning it.
        if let GraphPattern::PropertyFunction(call) = node {
            let bound = call_scope(scope.get(&self.scopes), promise.view(&self.scopes));
            if admitted_row_bound(call, self.relations, &bound)?.is_none() {
                return Err(stuck(
                    &[Atom {
                        pattern: node,
                        call: Some(call),
                        position: 0,
                    }],
                    self.relations,
                    &bound,
                ));
            }
            self.values.push(Planned::Pattern(node.clone()));
            return Ok(());
        }
        // A chain is a left-deep spine of `Lateral`s (a call's join) and `Join`s (the
        // residual data written between two calls), which is exactly the shape the parser
        // assembles a triples block containing calls into. Anything else is rebuilt
        // structurally.
        let mut atoms = Vec::new();
        if collect_chain(node, &mut atoms) && atoms.iter().any(|atom| atom.call.is_some()) {
            return self.enter_chain(atoms, scope, promise);
        }
        self.enter_parts(node, scope, promise);
        Ok(())
    }

    /// Order a chain's atoms and push each to be planned: a call against the variables
    /// bound when it was chosen, anything else against the chain's enclosing context.
    fn enter_chain(
        &mut self,
        atoms: Vec<Atom<'a>>,
        scope: SetRef<'a>,
        promise: Prom<'a>,
    ) -> Result<(), PlanError> {
        let ChainOrder {
            ordered,
            is_call,
            bound_before,
        } = order_chain(
            atoms,
            self.relations,
            scope.get(&self.scopes),
            promise.view(&self.scopes),
        )?;
        let mark = self.scopes.len();
        self.scopes.extend(bound_before);
        self.steps.push(Step::Assemble(Assemble::Chain {
            is_call: is_call.clone(),
            mark,
        }));
        // Pushed last to first, so the atoms are planned in the chosen order.
        for (index, (pattern, call)) in ordered.into_iter().zip(is_call).enumerate().rev() {
            // A call is re-attached through a `Lateral`, which drives it with the rows of
            // every atom before it, so it is admitted against the set bound when it was
            // chosen. Any other atom is re-attached through a `Join`, which evaluates it
            // on its own — so a call NESTED inside it (a `UNION` arm's own chain, say)
            // sees only what the chain's enclosing context binds.
            let atom_scope = if call {
                SetRef::Owned(mark + index)
            } else {
                scope
            };
            self.steps.push(Step::Pattern(pattern, atom_scope, promise));
        }
        Ok(())
    }

    /// Push the parts of a non-chain node, threading to each the variables its left
    /// siblings certainly bind and the promise that reaches it.
    fn enter_parts(&mut self, node: &'a GraphPattern, scope: SetRef<'a>, promise: Prom<'a>) {
        // The one rewrite writes the parameters everywhere, so every part of the node
        // inherits the same promise. See [`Promise`].
        let mark = self.scopes.len();
        let assemble = Step::Assemble(Assemble::Pattern { node, mark });
        match node {
            GraphPattern::Apply {
                left,
                right,
                policy,
            } => {
                let mut certain = scope.get(&self.scopes).clone();
                self.widen(&mut certain, scope, left, promise);
                let mut right_scope = scope.get(&self.scopes).clone();
                let mut supplied = promise
                    .view(&self.scopes)
                    .for_calls()
                    .cloned()
                    .unwrap_or_default();
                for (input, _) in &policy.inputs {
                    right_scope.remove(input);
                    supplied.remove(input);
                }
                for (input, driver) in &policy.inputs {
                    if (certain.contains(driver)
                        || promise.view(&self.scopes).written().writes(driver))
                        && policy.optional.as_ref().is_none_or(|optional| {
                            optional
                                .retry_inputs
                                .iter()
                                .any(|(retry_input, retry_driver)| {
                                    retry_input == input
                                        && (certain.contains(retry_driver)
                                            || promise
                                                .view(&self.scopes)
                                                .written()
                                                .writes(retry_driver))
                                })
                        })
                    {
                        right_scope.insert(input.clone());
                        supplied.insert(input.clone());
                    }
                }
                if let Some(optional) = &policy.optional {
                    right_scope.insert(optional.forget_marker.clone());
                    supplied.insert(optional.forget_marker.clone());
                }
                let right_scope = self.own(right_scope);
                // Apply substitutes these values into the whole RHS, through
                // projections too; they are writes, not narrowable outer context.
                let right_promise = Prom::Everywhere(self.own(supplied));
                self.steps.push(assemble);
                self.steps
                    .push(Step::Pattern(right, right_scope, right_promise));
                self.steps.push(Step::Pattern(left, scope, promise));
            }
            // A leaf is planned as written. So is a `SERVICE`: its body is forwarded to a
            // remote endpoint rather than evaluated promise, and `crate::remote` refuses to
            // forward a call at all — so its body is left exactly as written.
            GraphPattern::Bgp { .. }
            | GraphPattern::Path { .. }
            | GraphPattern::Values { .. }
            | GraphPattern::PropertyFunction(_)
            | GraphPattern::Service { .. } => {
                self.values.push(Planned::Pattern(node.clone()));
            }
            // An ordinary `Join` evaluates its operands independently and joins the
            // results, so a call inside the right operand is invoked with nothing the
            // left operand binds: it sees what the enclosing context binds and no more.
            // Only a `Lateral` hands its right operand the left rows — which is why a
            // call that depends on an earlier atom is rebuilt through one (see
            // [`order_chain`]).
            GraphPattern::Join { left, right } => {
                self.steps.push(assemble);
                self.steps.push(Step::Pattern(right, scope, promise));
                self.steps.push(Step::Pattern(left, scope, promise));
            }
            // The right side of a `Lateral` is evaluated once per left row with that row
            // in hand, so it sees what the left side certainly binds. A `Lateral` whose
            // right operand plans to a call is a chain ([`planned_lateral_call`]) and
            // never reaches this arm; any other right operand is one the pushdown
            // recurses into: it is re-evaluated per left row and inner-joined with it,
            // so restricting a leaf inside it restricts the node (see
            // `crate::substitute`'s `push_probes`).
            GraphPattern::Lateral { left, right } => {
                let mut inner = scope.get(&self.scopes).clone();
                self.widen(&mut inner, scope, left, promise);
                let inner = self.own(inner);
                self.steps.push(assemble);
                self.steps.push(Step::Pattern(right, inner, promise));
                self.steps.push(Step::Pattern(left, scope, promise));
            }
            // `OPTIONAL`'s right side and `MINUS`'s right side are evaluated
            // independently of the left and then matched against it, exactly as a
            // `Join`'s right operand is, so a call inside either sees only what the
            // enclosing context binds. Their own bindings do not escape as certain
            // either, which `certainly_bound` accounts for.
            GraphPattern::LeftJoin {
                left,
                right,
                expression,
            } => {
                // The inline condition is evaluated only on candidate JOINED rows, so
                // both sides' bindings are available to it.
                let mut condition_scope = scope.get(&self.scopes).clone();
                self.widen(&mut condition_scope, scope, left, promise);
                self.widen(&mut condition_scope, scope, right, promise);
                let condition_scope = self.own(condition_scope);
                self.steps.push(assemble);
                if let Some(expression) = expression {
                    self.steps
                        .push(Step::Expression(expression, condition_scope, promise));
                }
                self.steps.push(Step::Pattern(right, scope, promise));
                self.steps.push(Step::Pattern(left, scope, promise));
            }
            GraphPattern::Minus { left, right } => {
                self.steps.push(assemble);
                self.steps.push(Step::Pattern(right, scope, promise));
                self.steps.push(Step::Pattern(left, scope, promise));
            }
            // A `UNION` branch cannot rely on its sibling.
            GraphPattern::Union { arms } => {
                self.steps.push(assemble);
                for arm in arms.iter().rev() {
                    self.steps.push(Step::Pattern(arm, scope, promise));
                }
            }
            // A `FILTER`'s expression is evaluated over the rows its inner pattern
            // produced, so an `EXISTS` inside it sees everything that pattern certainly
            // binds — which is exactly what makes a relation inside a correlated `EXISTS`
            // invocable with the outer row's values. The expression is planned first,
            // then the inner pattern.
            GraphPattern::Filter { expr, inner } => {
                let mut rows = scope.get(&self.scopes).clone();
                self.widen(&mut rows, scope, inner, promise);
                let rows = self.own(rows);
                self.steps.push(assemble);
                self.steps.push(Step::Pattern(inner, scope, promise));
                self.steps.push(Step::Expression(expr, rows, promise));
            }
            GraphPattern::Extend {
                inner, expression, ..
            } => {
                let mut rows = scope.get(&self.scopes).clone();
                self.widen(&mut rows, scope, inner, promise);
                let rows = self.own(rows);
                self.steps.push(assemble);
                self.steps.push(Step::Expression(expression, rows, promise));
                self.steps.push(Step::Pattern(inner, scope, promise));
            }
            GraphPattern::Unfold {
                inner, expression, ..
            } => {
                let mut rows = scope.get(&self.scopes).clone();
                self.widen(&mut rows, scope, inner, promise);
                let rows = self.own(rows);
                self.steps.push(assemble);
                self.steps.push(Step::Expression(expression, rows, promise));
                self.steps.push(Step::Pattern(inner, scope, promise));
            }
            GraphPattern::Graph { inner, .. } => {
                self.steps.push(assemble);
                self.steps.push(Step::Pattern(inner, scope, promise));
            }
            GraphPattern::OrderBy { inner, expression } => {
                let mut rows = scope.get(&self.scopes).clone();
                self.widen(&mut rows, scope, inner, promise);
                let rows = self.own(rows);
                self.steps.push(assemble);
                for key in expression.iter().rev() {
                    self.steps.push(Step::Order(key, rows, promise));
                }
                self.steps.push(Step::Pattern(inner, scope, promise));
            }
            // A sub-`SELECT` is its own scope: a variable bound outside it is visible
            // inside only when it projects it — the correlated substitution (a
            // `LATERAL`'s right operand, an `EXISTS` body) writes the outer row into it
            // for exactly the projected variables — so the correlation set is narrowed
            // to them on the way in. A prepared execution's parameters are written past
            // the projection, so the promise passes unchanged.
            GraphPattern::Project { inner, variables } => {
                let narrowed = narrowed_to(scope.get(&self.scopes), variables);
                let narrowed = self.own(narrowed);
                self.steps.push(assemble);
                self.steps.push(Step::Pattern(inner, narrowed, promise));
            }
            GraphPattern::Distinct { inner } | GraphPattern::Reduced { inner } => {
                self.steps.push(assemble);
                self.steps.push(Step::Pattern(inner, scope, promise));
            }
            GraphPattern::Slice { inner, .. } => {
                self.steps.push(assemble);
                self.steps.push(Step::Pattern(inner, scope, promise));
            }
            GraphPattern::Group {
                inner, aggregates, ..
            } => {
                let mut rows = scope.get(&self.scopes).clone();
                self.widen(&mut rows, scope, inner, promise);
                let rows = self.own(rows);
                self.steps.push(assemble);
                for (_, aggregate) in aggregates.iter().rev() {
                    self.steps.push(Step::Aggregate(aggregate, rows, promise));
                }
                self.steps.push(Step::Pattern(inner, scope, promise));
            }
        }
    }

    /// Enter `expr`: an expression that reaches neither a property-function call nor a
    /// custom aggregate is planned as written; any other has its operands pushed, and an
    /// `EXISTS` its pattern.
    fn enter_expression(&mut self, expr: &'a Expression, scope: SetRef<'a>, promise: Prom<'a>) {
        // Either hazard alone must still walk `expr` — see `plan_where_pattern`'s
        // identical widening for the same reason: an `EXISTS` whose inner `GROUP BY`
        // has a `Custom` aggregate but no property-function call must still reach
        // that aggregate's admission (through the `Expression::Exists` arm).
        if !crate::property_fn_eval::expression_reaches_property_function(expr)
            && !crate::property_fn_eval::expression_reaches_custom_aggregate(expr)
        {
            self.values.push(Planned::Expression(expr.clone()));
            return;
        }
        let assemble = Step::Assemble(Assemble::Expression(expr));
        match expr {
            // A correlated `EXISTS` sees its enclosing group's bindings, so the scope
            // carries straight in: that is what lets a relation inside one be invoked
            // bound. A prepared execution's parameters reach it too: the pre-binding
            // rewrite binds them in every call everywhere (see [`Promise::Everywhere`]).
            Expression::Exists(pattern) => {
                self.steps.push(assemble);
                self.steps.push(Step::Pattern(pattern, scope, promise));
            }
            Expression::Or(operands) | Expression::And(operands) => {
                self.steps.push(assemble);
                for operand in operands.iter().rev() {
                    self.steps.push(Step::Expression(operand, scope, promise));
                }
            }
            Expression::Arithmetic(first, steps) => {
                self.steps.push(assemble);
                for (_, operand) in steps.iter().rev() {
                    self.steps.push(Step::Expression(operand, scope, promise));
                }
                self.steps.push(Step::Expression(first, scope, promise));
            }
            Expression::Equal(a, b)
            | Expression::SameTerm(a, b)
            | Expression::Greater(a, b)
            | Expression::GreaterOrEqual(a, b)
            | Expression::Less(a, b)
            | Expression::LessOrEqual(a, b) => {
                self.steps.push(assemble);
                self.steps.push(Step::Expression(b, scope, promise));
                self.steps.push(Step::Expression(a, scope, promise));
            }
            Expression::UnaryPlus(a) | Expression::UnaryMinus(a) | Expression::Not(a) => {
                self.steps.push(assemble);
                self.steps.push(Step::Expression(a, scope, promise));
            }
            Expression::If(condition, then, otherwise) => {
                self.steps.push(assemble);
                self.steps.push(Step::Expression(otherwise, scope, promise));
                self.steps.push(Step::Expression(then, scope, promise));
                self.steps.push(Step::Expression(condition, scope, promise));
            }
            Expression::In(needle, haystack) => {
                self.steps.push(assemble);
                for item in haystack.iter().rev() {
                    self.steps.push(Step::Expression(item, scope, promise));
                }
                self.steps.push(Step::Expression(needle, scope, promise));
            }
            Expression::Coalesce(items) | Expression::FunctionCall(_, items) => {
                self.steps.push(assemble);
                for item in items.iter().rev() {
                    self.steps.push(Step::Expression(item, scope, promise));
                }
            }
            Expression::NamedNode(_)
            | Expression::Literal(_)
            | Expression::Variable(_)
            | Expression::Bound(_) => {
                self.values.push(Planned::Expression(expr.clone()));
            }
        }
    }

    /// Enter `aggregate`: for [`AggregateFunction::Custom`], ADMIT the call at prepare
    /// time — refuse an unregistered IRI, a positional-argument count the registry's
    /// entry does not declare, or an invalid `; NAME=value` scalarval clause (see
    /// [`validate_scalarvals`]), before any governor charge — then push the expression
    /// it reduces over and its sort keys. The built-in-vs-custom admission asymmetry
    /// mirrors a relation's: a built-in's arity is checked structurally by the parser
    /// (`SUM`/`AVG`/… accept exactly one expression), so only `Custom`'s host-declared
    /// arity and scalarvals need checking here.
    ///
    /// # Errors
    ///
    /// A [`PlanError`] tagged [`PlanSeam::Aggregate`] naming the IRI, for an
    /// unregistered `AggregateFunction::Custom` IRI, a supplied argument count its
    /// registered entry's declared [`crate::user_fn::Arity`] does not accept, or an
    /// invalid scalarval (see [`validate_scalarvals`]'s docs for the four ways one is
    /// refused).
    fn enter_aggregate(
        &mut self,
        aggregate: &'a AggregateExpression,
        scope: SetRef<'a>,
        promise: Prom<'a>,
    ) -> Result<(), PlanError> {
        if let AggregateFunction::Custom(iri) = aggregate.function() {
            let iri_str = iri.as_str();
            let Some(custom) = self.agg_registry.resolve(iri_str) else {
                return Err(PlanError::aggregate(EvalError::function(format!(
                    "no custom aggregate is registered for <{iri_str}>"
                ))));
            };
            let declared = crate::agg_fn::arity_contained(custom.as_ref(), iri_str)
                .map_err(PlanError::aggregate)?;
            let supplied = aggregate.args().len();
            if !declared.accepts(supplied) {
                return Err(PlanError::aggregate(EvalError::function(format!(
                    "custom aggregate <{iri_str}> is declared with {declared} argument(s); the \
                     call site supplies {supplied}"
                ))));
            }
            let declared_scalarvals = crate::agg_fn::scalarvals_contained(custom.as_ref(), iri_str)
                .map_err(PlanError::aggregate)?;
            validate_scalarvals(iri_str, aggregate.scalarvals(), &declared_scalarvals)
                .map_err(PlanError::aggregate)?;
        }
        self.steps
            .push(Step::Assemble(Assemble::Aggregate(aggregate)));
        // A `FOLD`'s own sort keys are per-row expressions read from the same
        // solutions its arguments are, so they are planned too: a property function or
        // custom aggregate reachable from `FOLD(?v ORDER BY f(?w))` would otherwise
        // skip this walk's prepare-time admission entirely. Pushed after the arguments'
        // reverse, so the arguments are planned first.
        for order in aggregate.order_by().iter().rev() {
            self.steps.push(Step::Order(order, scope, promise));
        }
        for arg in aggregate.args().iter().rev() {
            self.steps.push(Step::Expression(arg, scope, promise));
        }
        Ok(())
    }

    /// Enter a sort key: its expression is planned, and the key rebuilt with its
    /// `ASC`/`DESC` direction.
    fn enter_order(&mut self, order: &'a OrderExpression, scope: SetRef<'a>, promise: Prom<'a>) {
        self.steps.push(Step::Assemble(Assemble::Order(order)));
        let (OrderExpression::Asc(expr) | OrderExpression::Desc(expr)) = order;
        self.steps.push(Step::Expression(expr, scope, promise));
    }

    /// Rebuild one node from the planned parts under it, in the order its fields are
    /// written.
    fn assemble(&mut self, assemble: Assemble<'a>) {
        match assemble {
            Assemble::Pattern { node, mark } => {
                let planned = self.assemble_pattern(node);
                self.scopes.truncate(mark);
                self.values.push(Planned::Pattern(planned));
            }
            // Rebuild the left-deep spine in the chosen order: a call re-attaches
            // through a `Lateral` (it depends on what is to its left), everything else
            // through a `Join`.
            Assemble::Chain { is_call, mark } => {
                let mut chain: Option<GraphPattern> = None;
                for (part, call) in self.take(is_call.len()).zip(is_call) {
                    let planned = part.pattern();
                    chain = Some(match chain {
                        None => planned,
                        Some(left) => {
                            if call {
                                GraphPattern::Lateral {
                                    left: Child::new(left),
                                    right: Child::new(planned),
                                }
                            } else {
                                GraphPattern::Join {
                                    left: Child::new(left),
                                    right: Child::new(planned),
                                }
                            }
                        }
                    });
                }
                self.scopes.truncate(mark);
                self.values.push(Planned::Pattern(
                    chain.unwrap_or(GraphPattern::Bgp { patterns: vec![] }),
                ));
            }
            Assemble::Expression(expr) => {
                let planned = self.assemble_expression(expr);
                self.values.push(Planned::Expression(planned));
            }
            Assemble::Aggregate(aggregate) => {
                let mut parts = self.take(aggregate.args().len() + aggregate.order_by().len());
                let args: Vec<Expression> = parts
                    .by_ref()
                    .take(aggregate.args().len())
                    .map(Planned::expression)
                    .collect();
                let order_by: Vec<OrderExpression> = parts.map(Planned::order).collect();
                // Planning an argument rewrites it in place and never changes the
                // argument COUNT, and planning a sort key never removes one, so this can
                // never turn a valid `aggregate` into an invalid one — the
                // `AggregateExpression::new` call below cannot fail.
                let planned = AggregateExpression::new(
                    aggregate.function().clone(),
                    args,
                    aggregate.scalarvals().to_vec(),
                    order_by,
                    aggregate.distinct,
                )
                .expect("planning preserves argument count, so arity stays valid");
                self.values.push(Planned::Aggregate(planned));
            }
            Assemble::Order(order) => {
                let expr = next_expression(&mut self.take(1));
                self.values.push(Planned::Order(match order {
                    OrderExpression::Asc(_) => OrderExpression::Asc(expr),
                    OrderExpression::Desc(_) => OrderExpression::Desc(expr),
                }));
            }
        }
    }

    /// `node` rebuilt over its planned parts, taken in the order they were planned.
    fn assemble_pattern(&mut self, node: &'a GraphPattern) -> GraphPattern {
        match node {
            GraphPattern::Apply { policy, .. } => {
                let mut parts = self.take(2);
                let left = next_pattern(&mut parts);
                let right = next_pattern(&mut parts);
                let optional = policy.optional.as_ref().map(|optional| {
                    purrdf_sparql_algebra::algebra::OptionalApplication {
                        retry_inputs: optional.retry_inputs.clone(),
                        forget_marker: optional.forget_marker.clone(),
                    }
                });
                GraphPattern::Apply {
                    left: Child::new(left),
                    right: Child::new(right),
                    policy: Box::new(purrdf_sparql_algebra::algebra::ApplicationPolicy {
                        dataset_required: policy.dataset_required,
                        row_pipeline: policy.row_pipeline,
                        reduced_adjacent: policy.reduced_adjacent,
                        group_domain: policy.group_domain.clone(),
                        inputs: policy.inputs.clone(),
                        optional,
                    }),
                }
            }
            GraphPattern::Join { .. } => {
                let mut parts = self.take(2);
                let left = next_pattern(&mut parts);
                let right = next_pattern(&mut parts);
                GraphPattern::Join {
                    left: Child::new(left),
                    right: Child::new(right),
                }
            }
            GraphPattern::Lateral { .. } => {
                let mut parts = self.take(2);
                let left = next_pattern(&mut parts);
                let right = next_pattern(&mut parts);
                GraphPattern::Lateral {
                    left: Child::new(left),
                    right: Child::new(right),
                }
            }
            GraphPattern::LeftJoin { expression, .. } => {
                let mut parts = self.take(2 + usize::from(expression.is_some()));
                let left = next_pattern(&mut parts);
                let right = next_pattern(&mut parts);
                let expression = expression.as_ref().map(|_| next_expression(&mut parts));
                GraphPattern::LeftJoin {
                    left: Child::new(left),
                    right: Child::new(right),
                    expression,
                }
            }
            GraphPattern::Minus { .. } => {
                let mut parts = self.take(2);
                let left = next_pattern(&mut parts);
                let right = next_pattern(&mut parts);
                GraphPattern::Minus {
                    left: Child::new(left),
                    right: Child::new(right),
                }
            }
            GraphPattern::Union { arms } => {
                let mut parts = self.take(arms.len());
                GraphPattern::Union {
                    arms: arms.map_ref(|_| next_pattern(&mut parts)),
                }
            }
            GraphPattern::Filter { .. } => {
                let mut parts = self.take(2);
                let expr = next_expression(&mut parts);
                let inner = next_pattern(&mut parts);
                GraphPattern::Filter {
                    expr,
                    inner: Child::new(inner),
                }
            }
            GraphPattern::Extend { variable, .. } => {
                let mut parts = self.take(2);
                let inner = next_pattern(&mut parts);
                let expression = next_expression(&mut parts);
                GraphPattern::Extend {
                    inner: Child::new(inner),
                    variable: variable.clone(),
                    expression,
                }
            }
            GraphPattern::Unfold {
                element, companion, ..
            } => {
                let mut parts = self.take(2);
                let inner = next_pattern(&mut parts);
                let expression = next_expression(&mut parts);
                GraphPattern::Unfold {
                    inner: Child::new(inner),
                    expression,
                    element: element.clone(),
                    companion: companion.clone(),
                }
            }
            GraphPattern::Graph { name, .. } => GraphPattern::Graph {
                name: name.clone(),
                inner: Child::new(next_pattern(&mut self.take(1))),
            },
            GraphPattern::OrderBy { expression, .. } => {
                let mut parts = self.take(1 + expression.len());
                let inner = next_pattern(&mut parts);
                GraphPattern::OrderBy {
                    inner: Child::new(inner),
                    expression: parts.map(Planned::order).collect(),
                }
            }
            GraphPattern::Project { variables, .. } => GraphPattern::Project {
                inner: Child::new(next_pattern(&mut self.take(1))),
                variables: variables.clone(),
            },
            GraphPattern::Distinct { .. } => GraphPattern::Distinct {
                inner: Child::new(next_pattern(&mut self.take(1))),
            },
            GraphPattern::Reduced { .. } => GraphPattern::Reduced {
                inner: Child::new(next_pattern(&mut self.take(1))),
            },
            GraphPattern::Slice { start, length, .. } => GraphPattern::Slice {
                inner: Child::new(next_pattern(&mut self.take(1))),
                start: *start,
                length: *length,
            },
            GraphPattern::Group {
                variables,
                aggregates,
                ..
            } => {
                let mut parts = self.take(1 + aggregates.len());
                let inner = next_pattern(&mut parts);
                GraphPattern::Group {
                    inner: Child::new(inner),
                    variables: variables.clone(),
                    aggregates: aggregates
                        .iter()
                        .zip(parts)
                        .map(|((variable, _), part)| (variable.clone(), part.aggregate()))
                        .collect(),
                }
            }
            GraphPattern::Bgp { .. }
            | GraphPattern::Path { .. }
            | GraphPattern::Values { .. }
            | GraphPattern::PropertyFunction(_)
            | GraphPattern::Service { .. } => {
                unreachable!("a leaf is planned as it is entered and never assembled")
            }
        }
    }

    /// `expr` rebuilt over its planned operands, taken in the order they were planned.
    fn assemble_expression(&mut self, expr: &'a Expression) -> Expression {
        match expr {
            Expression::Exists(_) => {
                Expression::Exists(Child::new(next_pattern(&mut self.take(1))))
            }
            Expression::Or(operands) => {
                let mut parts = self.take(operands.len());
                Expression::Or(operands.map_ref(|_| next_expression(&mut parts)))
            }
            Expression::And(operands) => {
                let mut parts = self.take(operands.len());
                Expression::And(operands.map_ref(|_| next_expression(&mut parts)))
            }
            Expression::Arithmetic(_, steps) => {
                let mut parts = self.take(1 + steps.len());
                let first = next_expression(&mut parts);
                Expression::Arithmetic(
                    Child::new(first),
                    steps.map_ref(|(operator, _)| (*operator, next_expression(&mut parts))),
                )
            }
            Expression::Equal(..) => {
                let (a, b) = self.take_pair();
                Expression::Equal(a, b)
            }
            Expression::SameTerm(..) => {
                let (a, b) = self.take_pair();
                Expression::SameTerm(a, b)
            }
            Expression::Greater(..) => {
                let (a, b) = self.take_pair();
                Expression::Greater(a, b)
            }
            Expression::GreaterOrEqual(..) => {
                let (a, b) = self.take_pair();
                Expression::GreaterOrEqual(a, b)
            }
            Expression::Less(..) => {
                let (a, b) = self.take_pair();
                Expression::Less(a, b)
            }
            Expression::LessOrEqual(..) => {
                let (a, b) = self.take_pair();
                Expression::LessOrEqual(a, b)
            }
            Expression::UnaryPlus(_) => {
                Expression::UnaryPlus(Child::new(next_expression(&mut self.take(1))))
            }
            Expression::UnaryMinus(_) => {
                Expression::UnaryMinus(Child::new(next_expression(&mut self.take(1))))
            }
            Expression::Not(_) => Expression::Not(Child::new(next_expression(&mut self.take(1)))),
            Expression::If(..) => {
                let mut parts = self.take(3);
                let condition = next_expression(&mut parts);
                let then = next_expression(&mut parts);
                let otherwise = next_expression(&mut parts);
                Expression::If(
                    Child::new(condition),
                    Child::new(then),
                    Child::new(otherwise),
                )
            }
            Expression::In(_, haystack) => {
                let mut parts = self.take(1 + haystack.len());
                let needle = next_expression(&mut parts);
                Expression::In(Child::new(needle), parts.map(Planned::expression).collect())
            }
            Expression::Coalesce(items) => {
                Expression::Coalesce(self.take(items.len()).map(Planned::expression).collect())
            }
            Expression::FunctionCall(function, args) => Expression::FunctionCall(
                function.clone(),
                self.take(args.len()).map(Planned::expression).collect(),
            ),
            Expression::NamedNode(_)
            | Expression::Literal(_)
            | Expression::Variable(_)
            | Expression::Bound(_) => {
                unreachable!("a constant or a variable is planned as it is entered")
            }
        }
    }

    /// The two planned operands of a binary operator, in order.
    fn take_pair(&mut self) -> (Child<Expression>, Child<Expression>) {
        let mut parts = self.take(2);
        let a = next_expression(&mut parts);
        let b = next_expression(&mut parts);
        (Child::new(a), Child::new(b))
    }
}

/// Validate a [`AggregateFunction::Custom`] call's `; NAME=value` scalarval
/// clauses (`supplied`) against `<iri>`'s registered
/// [`crate::agg_fn::CustomAggregate::scalarvals`] declaration (`declared`), at
/// PREPARE time — called from [`Planner::enter_aggregate`], before any governor charge.
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
/// SAME [`purrdf_xsd`] numeric-tower classification (`xsd_of` + `XsdValue::is_numeric`)
/// the evaluator itself uses to classify a runtime `TermValue`, rather than a
/// hand-rolled datatype-IRI string comparison, so a numeric scalarval's
/// admission rule can never drift from what the numeric tower actually accepts
/// elsewhere in this crate.
fn scalarval_value_matches_kind(value: &Literal, kind: ScalarvalKind) -> bool {
    match kind {
        ScalarvalKind::Numeric => xsd_of(&literal_to_value(value))
            .as_ref()
            .is_some_and(purrdf_xsd::XsdValue::is_numeric),
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
/// only a solution in which a `BIND`'s own expression raised an error, or an aggregate
/// failed on the values its group handed it.
///
/// This is deliberately narrower than a scope walk. A variable that is merely *in
/// scope* may be unbound in a row for a reason the query text fixes — `OPTIONAL`'s
/// right side, a `UNION` branch that does not mention it, an `UNDEF` cell of a `VALUES`
/// column, a `SAMPLE` over the one implicit group an aggregate without `GROUP BY`
/// forms over an empty input — and treating one of those as bound would admit an
/// invocation the text itself guarantees some row cannot make.
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
///   ([`Outcome::Truth`]): `FILTER(BOUND(?v))`, `FILTER(sameTerm(?v, ?o))`,
///   `FILTER(isIRI(?v))`, a built-in strict in `?v` such as
///   `FILTER(REGEX(STR(?v), "x"))`, and their conjunctions bind `?v`; a disjunction
///   binds only what both of its sides do, and `FILTER(!BOUND(?v))` binds nothing;
/// * the variable naming a `GRAPH`;
/// * a grouping key every row of the grouped pattern binds;
/// * an aggregate's output when [`aggregate_certainly_binds`] says every group row
///   holds one save a row whose aggregate failed on present values — `COUNT`, `SUM`,
///   `AVG`, `GROUP_CONCAT` and `FOLD` always, `SAMPLE`/`MIN`/`MAX` and a custom
///   aggregate under an explicit `GROUP BY` of arguments reading only what every row
///   binds;
/// * bound by both operands of a `UNION`, by either operand of a `Join` or a
///   `LATERAL` (the right one judged with the left one's bindings in hand), by the
///   left operand of an `OPTIONAL` or a `MINUS`, by the inner pattern of a `FILTER`,
///   `BIND`, `UNFOLD`, `ORDER BY`, `DISTINCT`, `REDUCED` or slice, or by the inner
///   pattern of a sub-`SELECT` that projects it.
///
/// # Why a `BIND` or an aggregate counts although it can fail
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
/// An aggregate's output follows the same rule: it counts when every value it is
/// computed from is certainly present, and a group whose aggregate failed on those
/// values — a `MAX(STR(?x))` whose every `?x` is a blank node, a `SUM` over a string —
/// is a row refused (or, for a relation that also serves the free mode, invoked free)
/// exactly as a row whose `BIND` erred. [`aggregate_certainly_binds`] states which
/// absences of input the text itself fixes.
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
/// the pattern's own bindings. See [`Promise`].
#[derive(Clone, Copy, Debug)]
struct Written<'a> {
    /// The parameters the one pre-binding rewrite writes into EVERY expression of the
    /// query (a constant for an IRI or a literal, a driven value for a blank node or a
    /// quoted triple — `crate::substitute`'s `drive_expression_reads`), sub-`SELECT`s
    /// included.
    everywhere: Option<&'a DetHashSet<Variable>>,
}

impl Written<'_> {
    /// Nothing is written: the pattern is judged by its own bindings and its context.
    const NOTHING: Self = Self { everywhere: None };

    /// Whether an expression reads `variable` as a value the run wrote in.
    fn writes(self, variable: &Variable) -> bool {
        self.everywhere
            .is_some_and(|parameters| parameters.contains(variable))
    }
}

/// [`collect_certainly_bound_in`], with what the run writes in — see [`Written`].
///
/// A post-order walk over a work list, so a pattern of any depth needs no more machine
/// stack: each node's certainly-bound set is computed from its children's, under the
/// context and the writes that reach it, and the root's is added to `out`. A `LATERAL`
/// binds its left operand first, then its right operand under the context the left one
/// widens.
fn collect_bound(
    pattern: &GraphPattern,
    context: &DetHashSet<Variable>,
    given_written: Written<'_>,
    out: &mut DetHashSet<Variable>,
) {
    /// The context a node is evaluated under: the caller's, or one a `LATERAL` or a
    /// sub-`SELECT` built for its operand, held in the arena.
    #[derive(Clone, Copy)]
    enum Context {
        Given,
        Owned(usize),
    }
    #[derive(Clone, Copy)]
    enum Writes {
        Given,
        Owned(std::num::NonZeroUsize),
    }
    // The original caller writes remain outside the worklist. A one-based arena
    // index uses its zero niche, preserving the former per-frame pointer width.
    const _: () = assert!(size_of::<Writes>() == size_of::<Written<'_>>());
    impl Writes {
        fn set<'s>(
            self,
            given: Written<'s>,
            arena: &'s [DetHashSet<Variable>],
        ) -> Option<&'s DetHashSet<Variable>> {
            match self {
                Self::Given => given.everywhere,
                Self::Owned(index) => Some(&arena[index.get() - 1]),
            }
        }
        fn writes(
            self,
            variable: &Variable,
            given: Written<'_>,
            arena: &[DetHashSet<Variable>],
        ) -> bool {
            self.set(given, arena)
                .is_some_and(|set| set.contains(variable))
        }
    }
    /// One step of the walk.
    enum Step<'p> {
        /// Enter a node: a leaf's set is computed at once, a node's children are pushed.
        Enter(&'p GraphPattern, Context, Writes),
        /// Combine a node's set from its children's; the arena is cut back to `mark`.
        Exit(&'p GraphPattern, Context, Writes, usize),
        /// A `LATERAL` whose left operand is bound: widen the context for its right
        /// operand, which is named.
        LateralLeft(&'p GraphPattern, Context, Writes),
        /// A contextual driver is bound before its renamed inputs reach the RHS.
        ApplyLeft(
            &'p GraphPattern,
            &'p purrdf_sparql_algebra::algebra::ApplicationPolicy,
            Context,
            Writes,
        ),
        /// A `LATERAL` whose right operand is bound too: its set is the union of both,
        /// the left operand's carried here; the arena is cut back to `mark`.
        LateralRight(DetHashSet<Variable>, usize),
    }
    /// The set `context` names, read from `arena` when a node built it.
    fn resolve<'c>(
        context: Context,
        given: &'c DetHashSet<Variable>,
        arena: &'c [DetHashSet<Variable>],
    ) -> &'c DetHashSet<Variable> {
        match context {
            Context::Given => given,
            Context::Owned(index) => &arena[index],
        }
    }
    let mut arena: Vec<DetHashSet<Variable>> = Vec::new();
    let mut values: Vec<DetHashSet<Variable>> = Vec::new();
    let mut steps = vec![Step::Enter(pattern, Context::Given, Writes::Given)];
    while let Some(step) = steps.pop() {
        match step {
            Step::Enter(node, ctx, written) => {
                let mark = arena.len();
                match node {
                    GraphPattern::Bgp { patterns } => {
                        let mut bound = DetHashSet::default();
                        for triple in patterns {
                            collect_triple_vars(triple, &mut bound);
                        }
                        values.push(bound);
                    }
                    GraphPattern::Path {
                        subject,
                        path: _,
                        object,
                    } => {
                        let mut bound = DetHashSet::default();
                        collect_term_vars(subject, &mut bound);
                        collect_term_vars(object, &mut bound);
                        values.push(bound);
                    }
                    // Every flattened argument position of a call receives a value on
                    // every row it emits, so its variables are certainly bound by it.
                    GraphPattern::PropertyFunction(call) => {
                        let mut bound = DetHashSet::default();
                        for term in call.subject_args.iter().chain(&call.object_args) {
                            collect_term_vars(term, &mut bound);
                        }
                        values.push(bound);
                    }
                    // A `VALUES` column binds its variable in every row exactly when no
                    // row holds `UNDEF` there. An empty table produces no row at all, so
                    // every column of it qualifies vacuously — and nothing downstream is
                    // ever invoked from it.
                    GraphPattern::Values {
                        variables,
                        bindings,
                    } => {
                        let bound = variables
                            .iter()
                            .enumerate()
                            .filter(|(column, _)| {
                                bindings
                                    .iter()
                                    .all(|row| row.get(*column).is_some_and(Option::is_some))
                            })
                            .map(|(_, variable)| variable.clone())
                            .collect();
                        values.push(bound);
                    }
                    // A remote endpoint may omit a column, so it promises nothing.
                    GraphPattern::Service { .. } => {
                        values.push(DetHashSet::default());
                    }
                    GraphPattern::Join { left, right } => {
                        steps.push(Step::Exit(node, ctx, written, mark));
                        steps.push(Step::Enter(right, ctx, written));
                        steps.push(Step::Enter(left, ctx, written));
                    }
                    GraphPattern::Apply {
                        left,
                        right,
                        policy,
                    } => {
                        if policy.optional.is_some() {
                            steps.push(Step::Exit(node, ctx, written, mark));
                        } else {
                            steps.push(Step::ApplyLeft(right, policy, ctx, written));
                        }
                        steps.push(Step::Enter(left, ctx, written));
                    }
                    // The right operand is evaluated once per left row with that row in
                    // hand, so it sees the left operand's certain bindings as well as the
                    // enclosing context's: the left operand is bound first.
                    GraphPattern::Lateral { left, right } => {
                        steps.push(Step::LateralLeft(right, ctx, written));
                        steps.push(Step::Enter(left, ctx, written));
                    }
                    // The right side may contribute nothing to a row.
                    GraphPattern::LeftJoin { left, .. } | GraphPattern::Minus { left, .. } => {
                        steps.push(Step::Exit(node, ctx, written, mark));
                        steps.push(Step::Enter(left, ctx, written));
                    }
                    GraphPattern::Union { arms } => {
                        steps.push(Step::Exit(node, ctx, written, mark));
                        for arm in arms.iter().rev() {
                            steps.push(Step::Enter(arm, ctx, written));
                        }
                    }
                    // The solution-modifier wrappers the seed descends through keep
                    // `written`.
                    GraphPattern::Filter { inner, .. }
                    | GraphPattern::OrderBy { inner, .. }
                    | GraphPattern::Distinct { inner }
                    | GraphPattern::Reduced { inner }
                    | GraphPattern::Slice { inner, .. }
                    | GraphPattern::Unfold { inner, .. }
                    | GraphPattern::Extend { inner, .. }
                    | GraphPattern::Group { inner, .. } => {
                        steps.push(Step::Exit(node, ctx, written, mark));
                        steps.push(Step::Enter(inner, ctx, written));
                    }
                    GraphPattern::Graph { inner, .. } => {
                        steps.push(Step::Exit(node, ctx, written, mark));
                        steps.push(Step::Enter(inner, ctx, written));
                    }
                    // The context reaches a sub-`SELECT`'s inner pattern only through the
                    // variables the projection names; what the SHACL pre-binding rewrite
                    // writes is written past the projection too.
                    GraphPattern::Project { inner, variables } => {
                        let narrowed = narrowed_to(resolve(ctx, context, &arena), variables);
                        arena.push(narrowed);
                        steps.push(Step::Exit(node, ctx, written, mark));
                        steps.push(Step::Enter(inner, Context::Owned(mark), written));
                    }
                }
            }
            Step::ApplyLeft(right, policy, ctx, written) => {
                let left_bound = values.pop().expect("the driver is bound first");
                let outer = resolve(ctx, context, &arena);
                let mut right_context = outer.clone();
                let mut supplied = written
                    .set(given_written, &arena)
                    .cloned()
                    .unwrap_or_default();
                for (input, _) in &policy.inputs {
                    right_context.remove(input);
                    supplied.remove(input);
                }
                for (input, driver) in &policy.inputs {
                    if left_bound.contains(driver)
                        || outer.contains(driver)
                        || written.writes(driver, given_written, &arena)
                    {
                        right_context.insert(input.clone());
                        supplied.insert(input.clone());
                    }
                }
                let mark = arena.len();
                arena.push(right_context);
                arena.push(supplied);
                steps.push(Step::LateralRight(left_bound, mark));
                steps.push(Step::Enter(
                    right,
                    Context::Owned(mark),
                    Writes::Owned(
                        std::num::NonZeroUsize::new(
                            mark.checked_add(2)
                                .expect("an allocated arena index is representable"),
                        )
                        .expect("one-based arena index"),
                    ),
                ));
            }
            Step::LateralLeft(right, ctx, written) => {
                let left_bound = values.pop().expect("the left operand is bound first");
                let mut right_context = resolve(ctx, context, &arena).clone();
                right_context.extend(left_bound.iter().cloned());
                let mark = arena.len();
                arena.push(right_context);
                steps.push(Step::LateralRight(left_bound, mark));
                steps.push(Step::Enter(right, Context::Owned(mark), written));
            }
            Step::LateralRight(left_bound, mark) => {
                let mut bound = values.pop().expect("the right operand is bound second");
                bound.extend(left_bound);
                arena.truncate(mark);
                values.push(bound);
            }
            Step::Exit(node, ctx, written, mark) => {
                let bound = match node {
                    GraphPattern::Apply { policy, .. } => {
                        if policy.optional.is_some() {
                            values.pop().expect("the driver is bound")
                        } else {
                            let right = values.pop().expect("the operand is bound");
                            let mut left = values.pop().expect("the driver is bound");
                            left.extend(right);
                            left
                        }
                    }
                    GraphPattern::Join { .. } => {
                        let right = values.pop().expect("the right operand is bound");
                        let mut bound = values.pop().expect("the left operand is bound");
                        bound.extend(right);
                        bound
                    }
                    GraphPattern::LeftJoin { .. } | GraphPattern::Minus { .. } => {
                        values.pop().expect("the left operand is bound")
                    }
                    // Only what EVERY arm binds is bound in every row.
                    GraphPattern::Union { arms } => {
                        let at = values.len() - arms.len();
                        let mut arms_bound = values.drain(at..);
                        let first = arms_bound.next().unwrap_or_default();
                        arms_bound.fold(first, |common, bound| {
                            common.intersection(&bound).cloned().collect()
                        })
                    }
                    // A `FILTER` passes only rows its condition is true on, and every
                    // variable the condition requires bound for that is therefore bound
                    // in each row it passes — see [`Outcome::Truth`]. A variable the run
                    // writes into the condition itself (under the SHACL pre-binding
                    // rewrite) is read there as the written value, not from the row, so
                    // the condition constrains nothing about the row's binding of it. A
                    // condition that is never true passes no row; it is taken to bind
                    // nothing, the narrow answer.
                    GraphPattern::Filter { expr, .. } => {
                        let mut bound = values.pop().expect("the inner pattern is bound");
                        bound.extend(
                            requires(expr, Outcome::Truth)
                                .unwrap_or_default()
                                .into_iter()
                                .filter(|variable| {
                                    !written.writes(variable, given_written, &arena)
                                }),
                        );
                        bound
                    }
                    // `UNFOLD`s own targets are NOT certainly bound: a SEP-0009 `null`
                    // element (or a null map value) yields the row with that variable
                    // unbound, so only what the inner pattern certainly binds escapes.
                    GraphPattern::OrderBy { .. }
                    | GraphPattern::Distinct { .. }
                    | GraphPattern::Reduced { .. }
                    | GraphPattern::Slice { .. }
                    | GraphPattern::Unfold { .. } => {
                        values.pop().expect("the inner pattern is bound")
                    }
                    // A `BIND`'s target counts when its expression reads only what the
                    // inner pattern certainly binds, what the enclosing context already
                    // holds, or what the run writes in: it is then unbound only in a row
                    // whose expression errored on the data, and that row is refused per
                    // row by the evaluator rather than invoked free. See
                    // [`collect_certainly_bound`]'s doc for the whole argument.
                    GraphPattern::Extend {
                        variable,
                        expression,
                        ..
                    } => {
                        let mut bound = values.pop().expect("the inner pattern is bound");
                        let context = resolve(ctx, context, &arena);
                        if expression_reads_only_bound(expression, &|read| {
                            bound.contains(read)
                                || context.contains(read)
                                || written.writes(read, given_written, &arena)
                        }) {
                            bound.insert(variable.clone());
                        }
                        bound
                    }
                    GraphPattern::Graph { name, .. } => {
                        let mut bound = values.pop().expect("the inner pattern is bound");
                        if let NamedNodePattern::Variable(variable) = name {
                            bound.insert(variable.clone());
                        }
                        bound
                    }
                    // Only what the projection keeps escapes, and only if the inner
                    // pattern bound it certainly.
                    GraphPattern::Project { variables, .. } => {
                        let inner_bound = values.pop().expect("the inner pattern is bound");
                        variables
                            .iter()
                            .filter(|variable| inner_bound.contains(*variable))
                            .cloned()
                            .collect()
                    }
                    // A grouping key is bound in a group's row when every row of the
                    // group binds it; an aggregate's output by
                    // [`aggregate_certainly_binds`].
                    GraphPattern::Group {
                        variables,
                        aggregates,
                        ..
                    } => {
                        let inner_bound = values.pop().expect("the inner pattern is bound");
                        let context = resolve(ctx, context, &arena);
                        let row_binds =
                            |read: &Variable| inner_bound.contains(read) || context.contains(read);
                        let mut bound: DetHashSet<Variable> = variables
                            .iter()
                            .filter(|key| row_binds(key))
                            .cloned()
                            .collect();
                        let grouped = !variables.is_empty();
                        for (variable, aggregate) in aggregates {
                            if aggregate_certainly_binds(aggregate, grouped, &|read| {
                                row_binds(read) || written.writes(read, given_written, &arena)
                            }) {
                                bound.insert(variable.clone());
                            }
                        }
                        bound
                    }
                    GraphPattern::Bgp { .. }
                    | GraphPattern::Path { .. }
                    | GraphPattern::PropertyFunction(_)
                    | GraphPattern::Values { .. }
                    | GraphPattern::Service { .. }
                    | GraphPattern::Lateral { .. } => {
                        unreachable!(
                            "a leaf is bound as it is entered, and a LATERAL through its own \
                             two steps"
                        )
                    }
                };
                arena.truncate(mark);
                values.push(bound);
            }
        }
    }
    out.extend(values.pop().expect("the root's set is computed last"));
}

/// Whether an aggregate's output is bound in every row its `GROUP BY` produces, save
/// only a row whose aggregate failed on the values it was handed — given `row_binds`,
/// what every row of every group binds.
///
/// # One rule for every computed value
///
/// A value computed from inputs counts as certainly bound when every input it is
/// computed from is certainly PRESENT: then it is unbound only where its evaluation
/// failed on present values, which is a fact about the data met one row at a time,
/// not a shape of the query. That is the rule a `BIND` target follows (see
/// [`collect_certainly_bound`]), and an aggregate's output follows it exactly: a
/// group row whose aggregate failed reaches a call with the input unbound, where the
/// evaluator re-derives the invocation's access pattern from the row in hand and
/// refuses one the relation does not declare, with a typed error (`admit_mode` in
/// `crate::property_fn_eval`). That check reads the call's arguments off whatever row
/// arrives, so it covers a row a `GROUP BY` produced as it covers any other: a
/// relation serving only the bound mode is never invoked free, and one that also
/// serves the free mode is invoked free on that row, as the row's own access pattern
/// is.
///
/// What an aggregate is computed from is its group's argument values. Absence of
/// those — as opposed to a failure on them — has two causes the query text fixes:
/// an argument reading a variable the grouped pattern may leave unbound (the right
/// side of an `OPTIONAL`, an `UNDEF` column), and the one implicit group an aggregate
/// WITHOUT `GROUP BY` forms even over an empty input. Whether either matters depends
/// on what the aggregate answers when handed no value at all:
///
/// * `COUNT`, `SUM`, `AVG`, `GROUP_CONCAT` and `FOLD` answer a value over no values —
///   `0`, `0`, `0`, `""` and the empty composite — and skip (or, for `FOLD`, keep as
///   a `null` element) a row whose argument is unbound. Absence therefore never
///   leaves them unbound, and they count with or without `GROUP BY`, whatever their
///   argument reads. `SUM` and `AVG` are unbound only when a present value is not a
///   number or the arithmetic fails, and `GROUP_CONCAT` only when a present value is
///   a blank node or a quoted triple, which has no lexical form: failures on present
///   values, met per row. `COUNT` and `FOLD` never answer unbound.
/// * `SAMPLE`, `MIN`, `MAX` and a custom aggregate may answer unbound over no values —
///   the first three always do, and a host's aggregate may. They count only under an
///   explicit `GROUP BY`, whose every group holds at least one row, and only over
///   arguments that read nothing but what every row binds (or constants —
///   [`expression_reads_only_bound`], the test a `BIND` expression passes). Every row
///   of a group then hands the aggregate a value unless its argument errored on
///   present values — `STR(?x)` on a blank node, `IRI(STR(?x))` on a string that is
///   not an IRI — and the aggregate is unbound only if every row's argument errored,
///   or a custom aggregate declined to answer over the values it was given. So
///   `MAX(STR(?q))` and `SAMPLE(IRI(STR(?q)))` count exactly as
///   `BIND(IRI(STR(?q)) AS ?y)` followed by `SAMPLE(?y)` does, and `SAMPLE(?w)` over
///   an `OPTIONAL`'s `?w` does not, as `BIND(?w AS ?y)` does not. WITHOUT a
///   `GROUP BY` the implicit group may hold no row, where these answer unbound for
///   want of any input: the same structural absence, so none of them counts there.
fn aggregate_certainly_binds(
    aggregate: &AggregateExpression,
    grouped: bool,
    row_binds: &dyn Fn(&Variable) -> bool,
) -> bool {
    match aggregate.function() {
        AggregateFunction::Count
        | AggregateFunction::Sum
        | AggregateFunction::Avg
        | AggregateFunction::GroupConcat
        | AggregateFunction::Fold => true,
        AggregateFunction::Sample
        | AggregateFunction::Min
        | AggregateFunction::Max
        | AggregateFunction::Custom(_) => {
            grouped
                && aggregate
                    .args()
                    .iter()
                    .all(|arg| expression_reads_only_bound(arg, row_binds))
        }
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
///
/// A post-order walk over a work list, so an expression of any depth needs no more
/// machine stack: every operand answers before the operator over it does.
fn expression_reads_only_bound(expr: &Expression, is_bound: &dyn Fn(&Variable) -> bool) -> bool {
    /// One step of the walk.
    enum Step<'e> {
        /// Enter an expression: a leaf answers at once, an operator pushes its operands.
        Enter(&'e Expression),
        /// Combine an operator's answer from its operands', `count` of them.
        Exit(&'e Expression, usize),
    }
    let mut steps = vec![Step::Enter(expr)];
    let mut values: Vec<bool> = Vec::new();
    while let Some(step) = steps.pop() {
        match step {
            Step::Enter(expr) => match expr {
                Expression::NamedNode(_)
                | Expression::Literal(_)
                | Expression::Bound(_)
                | Expression::Exists(_) => values.push(true),
                Expression::Variable(variable) => values.push(is_bound(variable)),
                Expression::Or(operands) | Expression::And(operands) => {
                    steps.push(Step::Exit(expr, operands.len()));
                    for operand in operands.iter().rev() {
                        steps.push(Step::Enter(operand));
                    }
                }
                Expression::Arithmetic(first, rest) => {
                    steps.push(Step::Exit(expr, 1 + rest.len()));
                    for (_, operand) in rest.iter().rev() {
                        steps.push(Step::Enter(operand));
                    }
                    steps.push(Step::Enter(first));
                }
                Expression::Equal(a, b)
                | Expression::SameTerm(a, b)
                | Expression::Greater(a, b)
                | Expression::GreaterOrEqual(a, b)
                | Expression::Less(a, b)
                | Expression::LessOrEqual(a, b) => {
                    steps.push(Step::Exit(expr, 2));
                    steps.push(Step::Enter(b));
                    steps.push(Step::Enter(a));
                }
                Expression::UnaryPlus(a) | Expression::UnaryMinus(a) | Expression::Not(a) => {
                    steps.push(Step::Exit(expr, 1));
                    steps.push(Step::Enter(a));
                }
                Expression::If(condition, then, otherwise) => {
                    steps.push(Step::Exit(expr, 3));
                    steps.push(Step::Enter(otherwise));
                    steps.push(Step::Enter(then));
                    steps.push(Step::Enter(condition));
                }
                Expression::In(needle, haystack) => {
                    steps.push(Step::Exit(expr, 1 + haystack.len()));
                    for item in haystack.iter().rev() {
                        steps.push(Step::Enter(item));
                    }
                    steps.push(Step::Enter(needle));
                }
                Expression::Coalesce(items) | Expression::FunctionCall(_, items) => {
                    steps.push(Step::Exit(expr, items.len()));
                    for item in items.iter().rev() {
                        steps.push(Step::Enter(item));
                    }
                }
            },
            Step::Exit(expr, count) => {
                let at = values.len() - count;
                let answer = {
                    let mut operands = values.drain(at..);
                    // `COALESCE` qualifies when any argument does; every other operator
                    // when every operand does.
                    if matches!(expr, Expression::Coalesce(_)) {
                        operands.any(|reads| reads)
                    } else {
                        operands.all(|reads| reads)
                    }
                };
                values.push(answer);
            }
        }
    }
    values
        .pop()
        .expect("the root's answer is the last one computed")
}

/// Whether `function` is a type test: total over terms, answering a boolean.
const fn is_type_test(function: &Function) -> bool {
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

/// Whether a call of `function` has no value when its argument at `position` is
/// unbound, whatever its other arguments hold — the argument positions the function is
/// STRICT in.
///
/// Read off the evaluator (`crate::expr`'s `apply_function` and the dispatch tables it
/// hands on to), and held to it by a drift test there that evaluates every built-in
/// with each argument unbound in turn: a position classified strict must yield no value
/// on every sample, and one classified otherwise must yield a value on at least one —
/// so the classification is neither wider nor narrower than the evaluator.
///
/// What is NOT strict, and why (`REGEX`'s and `REPLACE`'s flags and `SUBSTR`'s
/// length are optional, but a supplied one that is unbound is an error, so all three
/// are strict in every position):
///
/// * a type test (`isIRI` and its kin) answers `false` for an unbound argument;
/// * `cdt:List` and `cdt:Map` keep an unbound argument as a `null` element, and
///   `cdt:put` an unbound value as a `null` entry;
/// * a custom function is resolved at run time against what the host registered — a
///   SPARQL-bodied function may accept an unbound argument, and a registration may
///   even shadow an XSD cast's IRI — so nothing is known about it here, and it is taken
///   to need nothing.
///
/// A function taking no arguments (`RAND`, `NOW`, `UUID`, `STRUUID`, `BNODE()`) has no
/// position to be strict in; it is listed with the strict ones only because the answer
/// for a position it does not have is never asked.
pub(crate) const fn strict_in_argument(function: &Function, position: usize) -> bool {
    match function {
        Function::IsIri
        | Function::IsUri
        | Function::IsBlank
        | Function::IsLiteral
        | Function::IsNumeric
        | Function::IsTriple
        | Function::Custom(_) => false,
        Function::Cdt(call) => match call.fn_kind {
            CdtFn::ListConstructor | CdtFn::MapConstructor => false,
            CdtFn::Put => position < 2,
            CdtFn::Concat
            | CdtFn::Contains
            | CdtFn::Get
            | CdtFn::Head
            | CdtFn::Tail
            | CdtFn::Reverse
            | CdtFn::Size
            | CdtFn::Subseq
            | CdtFn::ContainsKey
            | CdtFn::Keys
            | CdtFn::Merge
            | CdtFn::Remove => true,
        },
        Function::Purrdf(call) => match call.fn_kind {
            PurrdfFn::HeldIn
            | PurrdfFn::ListLength
            | PurrdfFn::ListGet
            | PurrdfFn::ListIndexOf
            | PurrdfFn::ListContains
            | PurrdfFn::ListSlice
            | PurrdfFn::ListConcat => true,
        },
        Function::Str
        | Function::Regex
        | Function::Replace
        | Function::SubStr
        | Function::Lang
        | Function::LangMatches
        | Function::Datatype
        | Function::Iri
        | Function::Uri
        | Function::BNode
        | Function::Rand
        | Function::Abs
        | Function::Ceil
        | Function::Floor
        | Function::Round
        | Function::Concat
        | Function::StrLen
        | Function::UCase
        | Function::LCase
        | Function::EncodeForUri
        | Function::Contains
        | Function::StrStarts
        | Function::StrEnds
        | Function::StrBefore
        | Function::StrAfter
        | Function::Year
        | Function::Month
        | Function::Day
        | Function::Hours
        | Function::Minutes
        | Function::Seconds
        | Function::Timezone
        | Function::Tz
        | Function::Adjust
        | Function::Now
        | Function::Uuid
        | Function::StrUuid
        | Function::Md5
        | Function::Sha1
        | Function::Sha256
        | Function::Sha384
        | Function::Sha512
        | Function::Sha3_224
        | Function::Sha3_256
        | Function::Sha3_384
        | Function::Sha3_512
        | Function::StrLang
        | Function::StrDt
        | Function::Triple
        | Function::Subject
        | Function::Predicate
        | Function::Object
        | Function::LangDir
        | Function::StrLangDir
        | Function::HasLang
        | Function::HasLangDir => true,
    }
}

/// What an expression needs bound to reach an outcome — to be true, to be false, or to
/// have a value — or `None` when it can never reach it.
///
/// `None` is what a constant that is never true (`false`, `0`, an IRI, which has no
/// effective boolean value) answers for truth, and anything that reaches the outcome
/// only through it inherits it: it absorbs a conjunction of requirements
/// ([`all_of`]) and is the identity of an alternative between them ([`one_of`]). So
/// `IF(BOUND(?v), ?v = ?v, false)` is true only through its first branch, and needs
/// `?v`.
type Requires = Option<DetHashSet<Variable>>;

/// Needs nothing.
fn needs_nothing() -> Requires {
    Some(DetHashSet::default())
}

/// Needs `variable`.
fn needs(variable: &Variable) -> Requires {
    Some(std::iter::once(variable.clone()).collect())
}

/// Both requirements hold: the union, and never if either is never.
fn all_of(left: Requires, right: Requires) -> Requires {
    let (mut left, right) = (left?, right?);
    left.extend(right);
    Some(left)
}

/// One of the requirements holds, and which is not known: the intersection, over the
/// ones that can hold at all.
fn one_of(left: Requires, right: Requires) -> Requires {
    match (left, right) {
        (None, other) | (other, None) => other,
        (Some(mut left), Some(right)) => {
            left.retain(|variable| right.contains(variable));
            Some(left)
        }
    }
}

/// The outcome an expression is asked what it must have bound for — see [`Requires`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    /// TRUE — which a `FILTER` over the expression therefore binds in every row it
    /// passes.
    ///
    /// # The rule, and why each case is sound
    ///
    /// A variable read where the expression has no value unless it is bound — an operand
    /// of `=`, `sameTerm`, a comparison or arithmetic, an argument a built-in function is
    /// strict in ([`strict_in_argument`]), or the expression itself — errors when
    /// unbound, and an error is not true. So:
    ///
    /// * `?v` alone, `BOUND(?v)`: `?v`;
    /// * a constant: nothing if its effective boolean value is true, never otherwise;
    /// * `a && b`: both are true, so the union of what each requires;
    /// * `a || b`: at least one is true, and which is not known, so the INTERSECTION of
    ///   what each requires — `BOUND(?a) || BOUND(?b)` binds neither;
    /// * `!a`: `a` is false — what `a` needs to be false ([`Self::Falsity`]), so
    ///   `!BOUND(?v)` binds nothing and `!!BOUND(?v)` binds `?v`;
    /// * a type test `isIRI(a)` and its kin: `a` has a value of that type, so what `a`
    ///   needs for a value;
    /// * `IF(c, t, e)`: `c` has a value, and either `c` is true and `t` is, or `c` is
    ///   false and `e` is — so `IF(BOUND(?v), true, true)` binds nothing, and
    ///   `IF(BOUND(?v), ?v = ?v, false)` binds `?v`;
    /// * `COALESCE(…)`: the first argument with a value is true, so the intersection over
    ///   the arguments;
    /// * `a IN (…)`: `a` has a value;
    /// * every other operator, comparison and function call: its value needs its
    ///   operands' values, so what the whole expression needs for a value
    ///   ([`Self::Value`]) — `REGEX(STR(?v), "x")` and `STRLEN(STR(?v)) > 0` bind `?v`.
    ///
    /// `EXISTS` requires nothing. Every case needs a value somewhere, so no answer is
    /// wider than the evaluator's; a custom function, whose strictness the host decides,
    /// is taken to need nothing of its arguments.
    Truth,
    /// FALSE — the dual of [`Self::Truth`], which reads it through `!`.
    ///
    /// `BOUND(?v)` is false exactly when `?v` is unbound, and a type test is false on an
    /// unbound argument too, so neither requires anything; `EXISTS` requires nothing. A
    /// constant requires nothing if its effective boolean value is false, and is never
    /// false otherwise. `!a` is false when `a` is true ([`Self::Truth`]); `a && b` when
    /// at least one is false, so what both require; `a || b` when both are, so what
    /// either requires. `IF`, `COALESCE` and `IN` follow [`Self::Truth`]'s reasoning,
    /// and every other expression needs what it needs for any value ([`Self::Value`]).
    Falsity,
    /// Any value at all.
    ///
    /// `BOUND`, `EXISTS`, a constant and a type test answer for any row. `&&` and `||`
    /// can answer with one operand in error (`false && error` is false, `true || error`
    /// is true), so they need only what BOTH operands need; `IF` needs its condition and
    /// what the branch it takes needs; `COALESCE` what all its arguments need in common;
    /// `IN` its needle. A built-in function call needs what its arguments in the
    /// positions it is strict in need ([`strict_in_argument`]), so `STRLEN(STR(?v))`
    /// needs `?v`. Every other operator — a comparison, `sameTerm`, arithmetic, a unary
    /// sign — needs every operand.
    Value,
}

/// How an operator's requirement is combined from its operands'.
#[derive(Clone, Copy)]
enum Combine {
    /// Every operand's requirement holds: [`all_of`], folded from nothing needed.
    All,
    /// One operand's requirement holds, and which is not known: [`one_of`], folded from
    /// never.
    One,
    /// [`Self::One`] over the operands there are, or nothing needed when there are none —
    /// an empty chain is its identity constant, which has a value for any row.
    OneOrNothing,
    /// `IF`: its condition has a value, and either the condition is true and the first
    /// branch reaches the outcome, or it is false and the second does — over the five
    /// operand requirements pushed in that order.
    Conditional,
}

/// One step of [`requires`]'s walk.
enum RequirementStep<'e> {
    /// Ask `expr` what it needs to reach the outcome.
    Enter(&'e Expression, Outcome),
    /// Combine the top `count` requirements as `Combine` says.
    Exit(Combine, usize),
}

/// The variables `expr` must have bound to reach `outcome`, or `None` when it never does
/// — see [`Requires`], and the rules on each [`Outcome`].
///
/// A post-order walk over a work list, so an expression of any depth needs no more
/// machine stack: an operator asks each operand what it needs in the mode that operand
/// is read in, and combines the answers as its rule says.
fn requires(expr: &Expression, outcome: Outcome) -> Requires {
    let mut steps = vec![RequirementStep::Enter(expr, outcome)];
    let mut values: Vec<Requires> = Vec::new();
    while let Some(step) = steps.pop() {
        match step {
            RequirementStep::Enter(expr, outcome) => {
                enter_requirement(expr, outcome, &mut steps, &mut values);
            }
            RequirementStep::Exit(combine, count) => {
                let at = values.len() - count;
                let combined = {
                    let mut operands = values.drain(at..);
                    match combine {
                        Combine::All => operands.fold(needs_nothing(), all_of),
                        Combine::One => operands.fold(None, one_of),
                        Combine::OneOrNothing => {
                            operands.reduce(one_of).unwrap_or_else(needs_nothing)
                        }
                        Combine::Conditional => {
                            let mut next = || {
                                operands
                                    .next()
                                    .expect("an IF asks five operand requirements")
                            };
                            let condition_value = next();
                            let condition_true = next();
                            let then = next();
                            let condition_false = next();
                            let otherwise = next();
                            all_of(
                                condition_value,
                                one_of(
                                    all_of(condition_true, then),
                                    all_of(condition_false, otherwise),
                                ),
                            )
                        }
                    }
                };
                values.push(combined);
            }
        }
    }
    values
        .pop()
        .expect("the root's requirement is the last one combined")
}

/// Ask `expr` what it needs to reach `outcome`: a leaf's requirement is pushed onto
/// `values` at once; an operator's operands are pushed onto `steps`, each in the mode it
/// is read in, behind the step that combines their requirements.
fn enter_requirement<'e>(
    expr: &'e Expression,
    outcome: Outcome,
    steps: &mut Vec<RequirementStep<'e>>,
    values: &mut Vec<Requires>,
) {
    let mut outcome = outcome;
    loop {
        match (outcome, expr) {
            (Outcome::Truth, Expression::Variable(variable) | Expression::Bound(variable)) => {
                values.push(needs(variable));
            }
            (Outcome::Truth, Expression::Exists(_)) => values.push(needs_nothing()),
            (Outcome::Truth, Expression::NamedNode(_)) => values.push(None),
            (Outcome::Truth, Expression::Literal(literal)) => values
                .push((crate::expr::constant_ebv(literal) == Some(true)).then(DetHashSet::default)),
            // A chain is its binary operator folded from the operator's identity, whose
            // requirement is the fold's: `true` (`&&`) needs nothing, `false` (`||`) is
            // never true.
            (Outcome::Truth, Expression::And(operands)) => push_operands(
                steps,
                Combine::All,
                operands.iter().map(|operand| (operand, Outcome::Truth)),
            ),
            (Outcome::Truth, Expression::Or(operands)) => push_operands(
                steps,
                Combine::One,
                operands.iter().map(|operand| (operand, Outcome::Truth)),
            ),
            (Outcome::Truth, Expression::Not(operand)) => {
                steps.push(RequirementStep::Enter(operand, Outcome::Falsity));
            }
            (Outcome::Truth, Expression::FunctionCall(function, args))
                if is_type_test(function) =>
            {
                push_operands(
                    steps,
                    Combine::All,
                    args.iter().map(|arg| (arg, Outcome::Value)),
                );
            }
            (Outcome::Truth, Expression::Coalesce(items)) => push_operands(
                steps,
                Combine::One,
                items.iter().map(|item| (item, Outcome::Truth)),
            ),
            (Outcome::Falsity, Expression::Variable(variable)) => values.push(needs(variable)),
            (Outcome::Falsity, Expression::Bound(_) | Expression::Exists(_)) => {
                values.push(needs_nothing());
            }
            (Outcome::Falsity, Expression::NamedNode(_)) => values.push(None),
            (Outcome::Falsity, Expression::Literal(literal)) => values.push(
                (crate::expr::constant_ebv(literal) == Some(false)).then(DetHashSet::default),
            ),
            (Outcome::Falsity, Expression::FunctionCall(function, _)) if is_type_test(function) => {
                values.push(needs_nothing());
            }
            (Outcome::Falsity, Expression::Not(operand)) => {
                steps.push(RequirementStep::Enter(operand, Outcome::Truth));
            }
            (Outcome::Falsity, Expression::And(operands)) => push_operands(
                steps,
                Combine::One,
                operands.iter().map(|operand| (operand, Outcome::Falsity)),
            ),
            (Outcome::Falsity, Expression::Or(operands)) => push_operands(
                steps,
                Combine::All,
                operands.iter().map(|operand| (operand, Outcome::Falsity)),
            ),
            (Outcome::Falsity, Expression::Coalesce(items)) => push_operands(
                steps,
                Combine::One,
                items.iter().map(|item| (item, Outcome::Falsity)),
            ),
            (Outcome::Truth | Outcome::Falsity, Expression::If(condition, then, otherwise)) => {
                push_conditional(steps, condition, then, otherwise, outcome);
            }
            (Outcome::Truth | Outcome::Falsity, Expression::In(needle, _)) => {
                steps.push(RequirementStep::Enter(needle, Outcome::Value));
            }
            // Every other operator, comparison and function call reaches the outcome
            // only through a value, so it needs what it needs for one.
            (
                Outcome::Truth | Outcome::Falsity,
                Expression::Arithmetic(..)
                | Expression::Equal(..)
                | Expression::SameTerm(..)
                | Expression::Greater(..)
                | Expression::GreaterOrEqual(..)
                | Expression::Less(..)
                | Expression::LessOrEqual(..)
                | Expression::UnaryPlus(_)
                | Expression::UnaryMinus(_)
                | Expression::FunctionCall(..),
            ) => {
                outcome = Outcome::Value;
                continue;
            }
            (Outcome::Value, Expression::Variable(variable)) => values.push(needs(variable)),
            (
                Outcome::Value,
                Expression::NamedNode(_)
                | Expression::Literal(_)
                | Expression::Bound(_)
                | Expression::Exists(_),
            ) => values.push(needs_nothing()),
            (Outcome::Value, Expression::FunctionCall(function, args)) => push_operands(
                steps,
                Combine::All,
                args.iter()
                    .enumerate()
                    .filter(|(position, _)| strict_in_argument(function, *position))
                    .map(|(_, arg)| (arg, Outcome::Value)),
            ),
            // The pairwise `one_of`, folded; an empty chain is its identity constant,
            // which has a value for any row.
            (Outcome::Value, Expression::And(operands) | Expression::Or(operands)) => {
                push_operands(
                    steps,
                    Combine::OneOrNothing,
                    operands.iter().map(|operand| (operand, Outcome::Value)),
                );
            }
            (Outcome::Value, Expression::Arithmetic(first, rest)) => push_operands(
                steps,
                Combine::All,
                std::iter::once(&**first)
                    .chain(rest.iter().map(|(_, operand)| operand))
                    .map(|operand| (operand, Outcome::Value)),
            ),
            (
                Outcome::Value,
                Expression::Equal(a, b)
                | Expression::SameTerm(a, b)
                | Expression::Greater(a, b)
                | Expression::GreaterOrEqual(a, b)
                | Expression::Less(a, b)
                | Expression::LessOrEqual(a, b),
            ) => push_operands(
                steps,
                Combine::All,
                [(&**a, Outcome::Value), (&**b, Outcome::Value)].into_iter(),
            ),
            (
                Outcome::Value,
                Expression::UnaryPlus(operand)
                | Expression::UnaryMinus(operand)
                | Expression::Not(operand),
            ) => steps.push(RequirementStep::Enter(operand, Outcome::Value)),
            (Outcome::Value, Expression::If(condition, then, otherwise)) => {
                push_conditional(steps, condition, then, otherwise, Outcome::Value);
            }
            (Outcome::Value, Expression::Coalesce(items)) => push_operands(
                steps,
                Combine::One,
                items.iter().map(|item| (item, Outcome::Value)),
            ),
            (Outcome::Value, Expression::In(needle, _)) => {
                steps.push(RequirementStep::Enter(needle, Outcome::Value));
            }
        }
        return;
    }
}

/// Push `operands` to be asked, in reading order, behind the step combining their
/// requirements as `combine` says.
fn push_operands<'e>(
    steps: &mut Vec<RequirementStep<'e>>,
    combine: Combine,
    operands: impl Iterator<Item = (&'e Expression, Outcome)>,
) {
    let exit = steps.len();
    steps.push(RequirementStep::Exit(combine, 0));
    for (operand, outcome) in operands {
        steps.push(RequirementStep::Enter(operand, outcome));
    }
    let count = steps.len() - exit - 1;
    steps[exit] = RequirementStep::Exit(combine, count);
    // Pushed in reading order, so reversed to pop in it.
    steps[exit + 1..].reverse();
}

/// Push an `IF`'s five operand requirements ([`Combine::Conditional`]): the condition's
/// value, its truth, the first branch's `outcome`, the condition's falsity, and the
/// second branch's `outcome`.
fn push_conditional<'e>(
    steps: &mut Vec<RequirementStep<'e>>,
    condition: &'e Expression,
    then: &'e Expression,
    otherwise: &'e Expression,
    outcome: Outcome,
) {
    push_operands(
        steps,
        Combine::Conditional,
        [
            (condition, Outcome::Value),
            (condition, Outcome::Truth),
            (then, outcome),
            (condition, Outcome::Falsity),
            (otherwise, outcome),
        ]
        .into_iter(),
    );
}

/// Add a triple pattern's variables, its nested quoted triples walked over a work list.
fn collect_triple_vars(triple: &TriplePattern, out: &mut DetHashSet<Variable>) {
    let mut pending = vec![triple];
    while let Some(triple) = pending.pop() {
        for term in [&triple.subject, &triple.object] {
            match term {
                TermPattern::Variable(variable) => {
                    out.insert(variable.clone());
                }
                TermPattern::Triple(nested) => pending.push(nested),
                TermPattern::NamedNode(_) | TermPattern::BlankNode(_) | TermPattern::Literal(_) => {
                }
            }
        }
        if let NamedNodePattern::Variable(variable) = &triple.predicate {
            out.insert(variable.clone());
        }
    }
}

/// Add a term position's variables, a quoted triple's through [`collect_triple_vars`].
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
/// [`EvalError::FunctionOperational`] if a registered relation's declaration methods panic —
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
const CONTENT_DOMAIN: Domain = Domain::new(b"purrdf-sparql-eval/property-function-registry");

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
/// [`EvalError::FunctionOperational`] if a registered relation's declaration methods panic —
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
        PropertyFunction, PropertyFunctionRegistry, RankArithmetic, RankFidelity,
        RankedDeclaration,
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
            arithmetic: RankArithmetic::FloatFree,
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

/// **The pushdown's promise is the pushdown's reach.** On every lane a
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
            | GraphPattern::Lateral { left, right }
            | GraphPattern::Apply { left, right, .. }
            | GraphPattern::LeftJoin { left, right, .. }
            | GraphPattern::Minus { left, right } => {
                calls(left, out);
                calls(right, out);
            }
            GraphPattern::Union { arms } => {
                for arm in arms {
                    calls(arm, out);
                }
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

    /// `query` with every sub-`SELECT` projection over a `GROUP BY` removed, so the
    /// `GROUP BY` itself is what the rewrite and the promise meet — a shape the parser
    /// never produces from text, where the projection alone would already stop a
    /// variable that is not a key.
    ///
    /// Also answers how many projections it removed.
    fn without_group_projections(query: Query) -> (Query, usize) {
        fn over_group(pattern: &GraphPattern) -> bool {
            match pattern {
                GraphPattern::Group { .. } => true,
                GraphPattern::Extend { inner, .. } | GraphPattern::Filter { inner, .. } => {
                    over_group(inner)
                }
                _ => false,
            }
        }
        fn strip(pattern: &mut GraphPattern) -> usize {
            if let GraphPattern::Project { inner, .. } = pattern
                && over_group(inner)
            {
                let inner = std::mem::replace(
                    inner.as_mut(),
                    GraphPattern::Bgp {
                        patterns: Vec::new(),
                    },
                );
                *pattern = inner;
                return 1 + strip(pattern);
            }
            match pattern {
                GraphPattern::Join { left, right }
                | GraphPattern::Lateral { left, right }
                | GraphPattern::Apply { left, right, .. }
                | GraphPattern::LeftJoin { left, right, .. }
                | GraphPattern::Minus { left, right } => strip(left) + strip(right),
                GraphPattern::Union { arms } => arms.iter_mut().map(strip).sum(),
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
                | GraphPattern::Service { inner, .. } => strip(inner),
                GraphPattern::Bgp { .. }
                | GraphPattern::Path { .. }
                | GraphPattern::Values { .. }
                | GraphPattern::PropertyFunction(_) => 0,
            }
        }
        let mut query = query;
        let Query::Select { pattern, .. } = &mut query else {
            panic!("a SELECT stays one");
        };
        let removed = strip(pattern);
        (query, removed)
    }

    /// Whether the pre-binding rewrite of `body`'s plan writes `?q` into every call.
    fn rewrite_writes(body: &str) -> bool {
        rewrite_writes_in(parse(body), body)
    }

    /// [`rewrite_writes`], over an already-built `query`.
    fn rewrite_writes_in(query: Query, body: &str) -> bool {
        let parameters = parameter_set(&["q"]);
        let planned = plan_query(
            &query,
            &registry(&["bf", "ff"]),
            &AggregateRegistry::EMPTY,
            &parameters,
        )
        .unwrap_or_else(|error| panic!("{body}: a free-capable relation is admitted: {error}"))
        .unwrap_or(query);
        let rewritten = crate::substitute::apply_shacl_probes(
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
        admitted_in(&parse(body))
    }

    /// [`admitted`], over an already-built `query`.
    fn admitted_in(query: &Query) -> bool {
        plan_query(
            query,
            &registry(&["bf"]),
            &AggregateRegistry::EMPTY,
            &parameter_set(&["q"]),
        )
        .is_ok()
    }

    #[test]
    fn the_promise_holds_exactly_where_the_rewrite_writes() {
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
            // A `GROUP BY` the variable is a key of.
            (
                format!(
                    "{atom} LATERAL {{ SELECT ?q (COUNT(?out) AS ?n) WHERE {{ {call} }} GROUP BY ?q }}"
                ),
                true,
            ),
            // ... and where the ordinary pushdown alone would not reach: a sub-`SELECT`
            // that does not project the variable, a slice beneath the projection, an
            // `OPTIONAL` or a `MINUS` right arm inside the right side. The one rewrite
            // writes the parameter there too.
            (
                format!("{atom} LATERAL {{ SELECT ?out WHERE {{ {call} }} }}"),
                true,
            ),
            (
                format!("{atom} LATERAL {{ SELECT ?q ?out WHERE {{ {call} }} LIMIT 1 }}"),
                true,
            ),
            (format!("{atom} LATERAL {{ OPTIONAL {{ {call} }} }}"), true),
            (
                format!("{atom} LATERAL {{ {other} OPTIONAL {{ {call} }} }}"),
                true,
            ),
            (
                format!("{atom} LATERAL {{ {other} MINUS {{ {call} }} }}"),
                true,
            ),
            (format!("{{ {atom} }} {{ {call} }}"), true),
            (format!("{{ {{ {call} }} }}"), true),
            (format!("{call} FILTER(?out != 1)"), true),
            (format!("BIND(1 AS ?one) {call}"), true),
            (format!("{call} OPTIONAL {{ {atom} }}"), true),
            (format!("{atom} OPTIONAL {{ {call} }}"), true),
            (format!("{call} MINUS {{ {atom} }}"), true),
            (format!("{atom} MINUS {{ {call} }}"), true),
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
            (format!("{atom} {{ SELECT ?out WHERE {{ {call} }} }}"), true),
            (format!("VALUES ?w {{ 1 2 }} {call}"), true),
            (format!("{{ {call} }} LATERAL {{ BIND(1 AS ?one) }}"), true),
            // A `GROUP BY` beneath the core: entered for a key, whether the groups are
            // joined, reached through a `LATERAL`, filtered by a `HAVING`, or fed by a
            // `UNION` arm that leaves the key unbound ...
            (
                format!(
                    "{atom} {{ SELECT ?q (COUNT(?out) AS ?n) WHERE {{ {call} }} GROUP BY ?q }}"
                ),
                true,
            ),
            (
                format!(
                    "{atom} {{ SELECT ?q (COUNT(*) AS ?n) WHERE {{ {{ {call} }} UNION {{ {other} }} }} \
                     GROUP BY ?q }}"
                ),
                true,
            ),
            (
                format!(
                    "{atom} {{ SELECT ?q ?out WHERE {{ {call} }} GROUP BY ?q ?out \
                     HAVING(COUNT(*) > 0) }}"
                ),
                true,
            ),
            // ... and for a variable only an expression key or an aggregate reads, or that
            // no key names: the rewrite writes it there too.
            (
                format!(
                    "{atom} {{ SELECT ?k (COUNT(?out) AS ?n) WHERE {{ {call} }} \
                     GROUP BY (STR(?q) AS ?k) }}"
                ),
                true,
            ),
            (
                format!(
                    "{atom} {{ SELECT ?out (COUNT(?q) AS ?n) WHERE {{ {call} }} GROUP BY ?out }}"
                ),
                true,
            ),
            (
                format!("{atom} {{ SELECT (COUNT(?q) AS ?n) WHERE {{ {call} }} }}"),
                true,
            ),
            (
                format!(
                    "{atom} {{ SELECT ?out (SAMPLE(?q) AS ?n) WHERE {{ {call} }} GROUP BY ?out }}"
                ),
                true,
            ),
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

    /// The `GROUP BY` rule on its own, without the projection a sub-`SELECT` puts over
    /// it — which from text already stops every variable that is not a key, so the
    /// corpus above alone could not tell "a key" from "any variable". Each shape's
    /// projection is removed, leaving the `GROUP BY` the first node between the core
    /// and the call that could stop the rewrite; the rewrite and the promise must still
    /// agree, and both reach the call whatever the keys are.
    #[test]
    fn the_group_by_rule_holds_without_a_projection_over_it() {
        let call = format!("?q <{REL}> ?out");
        let atom = "?s <http://example.org/p> ?o";
        let other = "?s2 <http://example.org/p> ?o2";
        let corpus: Vec<(String, bool)> = vec![
            (
                format!(
                    "{atom} {{ SELECT ?q (COUNT(?out) AS ?n) WHERE {{ {call} }} GROUP BY ?q }}"
                ),
                true,
            ),
            (
                format!(
                    "{atom} LATERAL {{ SELECT ?q (COUNT(*) AS ?n) WHERE {{ {{ {call} }} UNION \
                     {{ {other} }} }} GROUP BY ?q }}"
                ),
                true,
            ),
            (
                format!(
                    "{atom} {{ SELECT ?out (COUNT(?q) AS ?n) WHERE {{ {call} }} GROUP BY ?out }}"
                ),
                true,
            ),
            (
                format!(
                    "{atom} {{ SELECT ?out (SAMPLE(?q) AS ?n) WHERE {{ {call} }} GROUP BY ?out }}"
                ),
                true,
            ),
            (
                format!("{atom} {{ SELECT (COUNT(?q) AS ?n) WHERE {{ {call} }} }}"),
                true,
            ),
            (
                format!(
                    "{atom} {{ SELECT ?k (COUNT(?out) AS ?n) WHERE {{ {call} }} \
                     GROUP BY (STR(?q) AS ?k) }}"
                ),
                true,
            ),
        ];
        for (body, reaches) in &corpus {
            let (query, removed) = without_group_projections(parse(body));
            assert_eq!(
                removed, 1,
                "{body}: the one projection over the GROUP BY is gone"
            );
            assert_eq!(
                rewrite_writes_in(query.clone(), body),
                *reaches,
                "{body}: the rewrite writes into a GROUP BY's input whatever its keys"
            );
            assert_eq!(
                admitted_in(&query),
                *reaches,
                "{body}: the promise follows the rewrite through a GROUP BY"
            );
        }
    }
}

#[cfg(test)]
mod iterative_walk_tests {
    //! The walks of this module against recursive references, over generated shapes and
    //! at a depth no machine stack holds.
    //!
    //! Every reference below is the walk written as a recursion, one call per node —
    //! the shape the production walk replaces with a work list — so the two agree
    //! exactly when the work list visits, combines and refuses as the recursion did.
    //! The shapes are drawn from a deterministic choice sequence, so a disagreement
    //! names the seed that reproduces it.

    use std::sync::Arc;

    use purrdf_core::TermValue;
    use purrdf_core::binding_pattern::BindingPattern;
    use purrdf_sparql_algebra::{
        ArithmeticOperator, BlankNode, Chain, GroundTerm, NamedNode, NonEmpty,
        PropertyPathExpression,
    };

    use super::*;
    use crate::agg_fn::{AggregateAccumulator, AlgebraicClass, CustomAggregate};
    use crate::property_fn::{PfArgs, PfCursor, PfRow, PropertyFunction};
    use crate::user_fn::{Arity, Volatility};

    // ── The recursive references ───────────────────────────────────────────────────

    /// Test-only recursive reference for `super::plan_pattern`.
    fn reference_plan_pattern(
        pattern: &GraphPattern,
        relations: &PropertyFunctionRegistry,
        agg_registry: &AggregateRegistry,
        outer: &DetHashSet<Variable>,
        promise: Promise<'_>,
    ) -> Result<GraphPattern, PlanError> {
        // Compiler-produced algebra may be a bare call, without the parser's Lateral
        // wrapper. Apply the same admission as a chain member before cloning it.
        if let GraphPattern::PropertyFunction(call) = pattern {
            let scope = call_scope(outer, promise);
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
        if reference_collect_chain(pattern, &mut atoms)
            && atoms.iter().any(|atom| atom.call.is_some())
        {
            return reference_order_chain(atoms, relations, agg_registry, outer, promise);
        }
        reference_map_children(pattern, relations, agg_registry, outer, promise)
    }

    /// Test-only recursive reference for `super::collect_chain`.
    fn reference_collect_chain<'a>(pattern: &'a GraphPattern, atoms: &mut Vec<Atom<'a>>) -> bool {
        match pattern {
            GraphPattern::Lateral { left, right } => {
                let Some((node, call)) = reference_planned_lateral_call(right) else {
                    return false;
                };
                if !reference_collect_chain(left, atoms) {
                    push_atom(left, None, atoms);
                }
                push_atom(node, Some(call), atoms);
                true
            }
            GraphPattern::Apply { .. } => false,
            GraphPattern::Join { left, right } => {
                // A call under a `Join` rather than a `Lateral` would lose the dependency
                // the `Lateral` encodes, so it is not treated as a chain member; the
                // structural recursion handles it.
                if matches!(&**right, GraphPattern::PropertyFunction(_)) {
                    return false;
                }
                if !reference_collect_chain(left, atoms) {
                    push_atom(left, None, atoms);
                }
                if !reference_collect_chain(right, atoms) {
                    push_atom(right, None, atoms);
                }
                true
            }
            _ => false,
        }
    }

    /// Test-only recursive reference for `super::planned_lateral_call`.
    fn reference_planned_lateral_call(
        right: &GraphPattern,
    ) -> Option<(&GraphPattern, &PropertyFunctionCall)> {
        match right {
            GraphPattern::Lateral { left, right } if left.is_empty_bgp() => {
                reference_planned_lateral_call(right)
            }
            other => crate::substitute::lateral_call(other).map(|call| (other, call)),
        }
    }

    /// Test-only recursive reference for `super::order_chain`.
    fn reference_order_chain(
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
            reference_collect_bound(atom.pattern, outer, promise.written(), &mut bound);
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
            let planned = reference_plan_pattern(
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
                            left: Child::new(left),
                            right: Child::new(planned),
                        }
                    } else {
                        GraphPattern::Join {
                            left: Child::new(left),
                            right: Child::new(planned),
                        }
                    }
                }
            });
        }
        Ok(chain.unwrap_or(GraphPattern::Bgp { patterns: vec![] }))
    }

    /// Test-only recursive reference for `super::term_is_bound`.
    fn reference_term_is_bound(term: &TermPattern, bound: &DetHashSet<Variable>) -> bool {
        match term {
            TermPattern::NamedNode(_) | TermPattern::Literal(_) => true,
            TermPattern::BlankNode(_) => false,
            TermPattern::Variable(variable) => bound.contains(variable),
            TermPattern::Triple(triple) => {
                reference_term_is_bound(&triple.subject, bound)
                    && match &triple.predicate {
                        NamedNodePattern::NamedNode(_) => true,
                        NamedNodePattern::Variable(variable) => bound.contains(variable),
                    }
                    && reference_term_is_bound(&triple.object, bound)
            }
        }
    }

    /// Test-only recursive reference for `super::map_children`.
    fn reference_map_children(
        pattern: &GraphPattern,
        relations: &PropertyFunctionRegistry,
        agg_registry: &AggregateRegistry,
        outer: &DetHashSet<Variable>,
        promise: Promise<'_>,
    ) -> Result<GraphPattern, PlanError> {
        let recurse = |child: &GraphPattern, outer: &DetHashSet<Variable>, promise: Promise<'_>| {
            reference_plan_pattern(child, relations, agg_registry, outer, promise).map(Child::new)
        };
        // The one rewrite writes the parameters everywhere, so every part of the node
        // inherits the same promise. See [`Promise`].
        Ok(match pattern {
            GraphPattern::Bgp { .. }
            | GraphPattern::Path { .. }
            | GraphPattern::Values { .. }
            | GraphPattern::PropertyFunction(_) => pattern.clone(),
            // An ordinary `Join` evaluates its operands independently and joins the results,
            // so a call inside the right operand is invoked with nothing the left operand
            // binds: it sees what the enclosing context binds and no more. Only a `Lateral`
            // hands its right operand the left rows — which is why a call that depends on
            // an earlier atom is rebuilt through one (see [`reference_order_chain`]).
            GraphPattern::Apply {
                left,
                right,
                policy,
            } => {
                let mut certain = outer.clone();
                reference_collect_bound(left, outer, promise.written(), &mut certain);
                let mut right_scope = outer.clone();
                let mut supplied = promise.for_calls().cloned().unwrap_or_default();
                for (input, _) in &policy.inputs {
                    right_scope.remove(input);
                    supplied.remove(input);
                }
                for (input, driver) in &policy.inputs {
                    if (certain.contains(driver) || promise.written().writes(driver))
                        && policy.optional.as_ref().is_none_or(|optional| {
                            optional
                                .retry_inputs
                                .iter()
                                .any(|(retry_input, retry_driver)| {
                                    retry_input == input
                                        && (certain.contains(retry_driver)
                                            || promise.written().writes(retry_driver))
                                })
                        })
                    {
                        right_scope.insert(input.clone());
                        supplied.insert(input.clone());
                    }
                }
                if let Some(optional) = &policy.optional {
                    right_scope.insert(optional.forget_marker.clone());
                    supplied.insert(optional.forget_marker.clone());
                }
                let optional = policy.optional.clone();
                GraphPattern::Apply {
                    left: recurse(left, outer, promise)?,
                    right: recurse(right, &right_scope, Promise::Everywhere(&supplied))?,
                    policy: Box::new(purrdf_sparql_algebra::algebra::ApplicationPolicy {
                        dataset_required: policy.dataset_required,
                        row_pipeline: policy.row_pipeline,
                        reduced_adjacent: policy.reduced_adjacent,
                        group_domain: policy.group_domain.clone(),
                        inputs: policy.inputs.clone(),
                        optional,
                    }),
                }
            }
            GraphPattern::Join { left, right } => GraphPattern::Join {
                left: recurse(left, outer, promise)?,
                right: recurse(right, outer, promise)?,
            },
            // The right side of a `Lateral` is evaluated once per left row with that row in
            // hand, so it sees what the left side certainly binds. A `Lateral` whose right
            // operand plans to a call is a chain ([`reference_planned_lateral_call`]) and never
            // reaches this arm; any other right operand is one the pushdown recurses into.
            GraphPattern::Lateral { left, right } => {
                let mut inner = outer.clone();
                reference_collect_bound(left, outer, promise.written(), &mut inner);
                GraphPattern::Lateral {
                    left: recurse(left, outer, promise)?,
                    // The pushdown enters every right operand too: it is re-evaluated per
                    // left row and inner-joined with it, so restricting a leaf inside it
                    // restricts the node (see `crate::substitute`'s `push_probes`).
                    right: recurse(right, &inner, promise)?,
                }
            }
            // `OPTIONAL`'s right side and `MINUS`'s right side are evaluated independently
            // of the left and then matched against it, exactly as a `Join`'s right operand
            // is, so a call inside either sees only what the enclosing context binds. Their
            // own bindings do not escape as certain either, which `certainly_bound`
            // accounts for.
            GraphPattern::LeftJoin {
                left,
                right,
                expression,
            } => {
                // The inline condition is evaluated only on candidate JOINED rows, so
                // both sides' bindings are available to it.
                let mut condition_scope = outer.clone();
                reference_collect_bound(left, outer, promise.written(), &mut condition_scope);
                reference_collect_bound(right, outer, promise.written(), &mut condition_scope);
                GraphPattern::LeftJoin {
                    left: recurse(left, outer, promise)?,
                    right: recurse(right, outer, promise)?,
                    expression: expression
                        .as_ref()
                        .map(|expr| {
                            reference_plan_expression(
                                expr,
                                relations,
                                agg_registry,
                                &condition_scope,
                                promise,
                            )
                        })
                        .transpose()?,
                }
            }
            GraphPattern::Minus { left, right } => GraphPattern::Minus {
                left: recurse(left, outer, promise)?,
                right: recurse(right, outer, promise)?,
            },
            // A `UNION` branch cannot rely on its sibling.
            GraphPattern::Union { arms } => GraphPattern::Union {
                arms: arms
                    .try_map_ref(|arm| recurse(arm, outer, promise).map(Child::into_inner))?,
            },
            // A `FILTER`'s expression is evaluated over the rows its inner pattern
            // produced, so an `EXISTS` inside it sees everything that pattern certainly
            // binds — which is exactly what makes a relation inside a correlated `EXISTS`
            // invocable with the outer row's values.
            GraphPattern::Filter { expr, inner } => {
                let mut scope = outer.clone();
                reference_collect_bound(inner, outer, promise.written(), &mut scope);
                GraphPattern::Filter {
                    expr: reference_plan_expression(
                        expr,
                        relations,
                        agg_registry,
                        &scope,
                        promise,
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
                reference_collect_bound(inner, outer, promise.written(), &mut scope);
                GraphPattern::Extend {
                    inner: recurse(inner, outer, promise)?,
                    variable: variable.clone(),
                    expression: reference_plan_expression(
                        expression,
                        relations,
                        agg_registry,
                        &scope,
                        promise,
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
                reference_collect_bound(inner, outer, promise.written(), &mut scope);
                GraphPattern::Unfold {
                    inner: recurse(inner, outer, promise)?,
                    expression: reference_plan_expression(
                        expression,
                        relations,
                        agg_registry,
                        &scope,
                        promise,
                    )?,
                    element: element.clone(),
                    companion: companion.clone(),
                }
            }
            GraphPattern::Graph { name, inner } => GraphPattern::Graph {
                name: name.clone(),
                inner: recurse(inner, outer, promise)?,
            },
            GraphPattern::OrderBy { inner, expression } => {
                let mut scope = outer.clone();
                reference_collect_bound(inner, outer, promise.written(), &mut scope);
                GraphPattern::OrderBy {
                    // The pushdown enters an `ORDER BY` beneath the core as well as above it.
                    inner: recurse(inner, outer, promise)?,
                    expression: expression
                        .iter()
                        .map(|order| {
                            Ok(match order {
                                OrderExpression::Asc(expr) => {
                                    OrderExpression::Asc(reference_plan_expression(
                                        expr,
                                        relations,
                                        agg_registry,
                                        &scope,
                                        promise,
                                    )?)
                                }
                                OrderExpression::Desc(expr) => {
                                    OrderExpression::Desc(reference_plan_expression(
                                        expr,
                                        relations,
                                        agg_registry,
                                        &scope,
                                        promise,
                                    )?)
                                }
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
            GraphPattern::Project { inner, variables } => GraphPattern::Project {
                inner: recurse(inner, &narrowed_to(outer, variables), promise)?,
                variables: variables.clone(),
            },
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
                inner: recurse(inner, outer, promise)?,
                start: *start,
                length: *length,
            },
            GraphPattern::Group {
                inner,
                variables,
                aggregates,
            } => {
                let mut scope = outer.clone();
                reference_collect_bound(inner, outer, promise.written(), &mut scope);
                GraphPattern::Group {
                    inner: recurse(inner, outer, promise)?,
                    variables: variables.clone(),
                    aggregates: aggregates
                        .iter()
                        .map(|(variable, aggregate)| {
                            Ok((
                                variable.clone(),
                                reference_plan_aggregate(
                                    aggregate,
                                    relations,
                                    agg_registry,
                                    &scope,
                                    promise,
                                )?,
                            ))
                        })
                        .collect::<Result<Vec<_>, PlanError>>()?,
                }
            }
            // A `SERVICE` body is forwarded to a remote endpoint rather than evaluated
            // promise, and `crate::remote` refuses to forward a call at all — so its body is
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

    /// Test-only recursive reference for `super::plan_expression`.
    fn reference_plan_expression(
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
            reference_plan_expression(expr, relations, agg_registry, outer, promise).map(Box::new)
        };
        Ok(match expr {
            // A correlated `EXISTS` sees its enclosing group's bindings, so `outer` carries
            // straight in: that is what lets a relation inside one be invoked bound. A
            // prepared execution's parameters reach it too: the rewrite binds them in every
            // call everywhere (see [`Promise::Everywhere`]).
            Expression::Exists(pattern) => Expression::Exists(Child::new(reference_plan_pattern(
                pattern,
                relations,
                agg_registry,
                outer,
                promise,
            )?)),
            Expression::Or(operands) => Expression::Or(operands.try_map_ref(|operand| {
                reference_plan_expression(operand, relations, agg_registry, outer, promise)
            })?),
            Expression::And(operands) => Expression::And(operands.try_map_ref(|operand| {
                reference_plan_expression(operand, relations, agg_registry, outer, promise)
            })?),
            Expression::Arithmetic(first, steps) => Expression::Arithmetic(
                sub(first)?.into(),
                steps.try_map_ref(|(op, operand)| {
                    reference_plan_expression(operand, relations, agg_registry, outer, promise)
                        .map(|planned| (*op, planned))
                })?,
            ),
            Expression::Equal(a, b) => Expression::Equal(sub(a)?.into(), sub(b)?.into()),
            Expression::SameTerm(a, b) => Expression::SameTerm(sub(a)?.into(), sub(b)?.into()),
            Expression::Greater(a, b) => Expression::Greater(sub(a)?.into(), sub(b)?.into()),
            Expression::GreaterOrEqual(a, b) => {
                Expression::GreaterOrEqual(sub(a)?.into(), sub(b)?.into())
            }
            Expression::Less(a, b) => Expression::Less(sub(a)?.into(), sub(b)?.into()),
            Expression::LessOrEqual(a, b) => {
                Expression::LessOrEqual(sub(a)?.into(), sub(b)?.into())
            }
            Expression::UnaryPlus(a) => Expression::UnaryPlus(sub(a)?.into()),
            Expression::UnaryMinus(a) => Expression::UnaryMinus(sub(a)?.into()),
            Expression::Not(a) => Expression::Not(sub(a)?.into()),
            Expression::If(c, t, e) => {
                Expression::If(sub(c)?.into(), sub(t)?.into(), sub(e)?.into())
            }
            Expression::In(needle, haystack) => Expression::In(
                sub(needle)?.into(),
                haystack
                    .iter()
                    .map(|item| {
                        reference_plan_expression(item, relations, agg_registry, outer, promise)
                    })
                    .collect::<Result<Vec<_>, PlanError>>()?
                    .into(),
            ),
            Expression::Coalesce(items) => Expression::Coalesce(
                items
                    .iter()
                    .map(|item| {
                        reference_plan_expression(item, relations, agg_registry, outer, promise)
                    })
                    .collect::<Result<Vec<_>, PlanError>>()?
                    .into(),
            ),
            Expression::FunctionCall(function, args) => Expression::FunctionCall(
                function.clone(),
                args.iter()
                    .map(|arg| {
                        reference_plan_expression(arg, relations, agg_registry, outer, promise)
                    })
                    .collect::<Result<Vec<_>, PlanError>>()?
                    .into(),
            ),
            Expression::NamedNode(_)
            | Expression::Literal(_)
            | Expression::Variable(_)
            | Expression::Bound(_) => expr.clone(),
        })
    }

    /// Test-only recursive reference for `super::plan_aggregate`.
    fn reference_plan_aggregate(
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
            .map(|e| reference_plan_expression(e, relations, agg_registry, outer, promise))
            .collect::<Result<Vec<_>, PlanError>>()?;
        // A `FOLD`'s own sort keys are per-row expressions read from the same
        // solutions its arguments are, so they must be planned too: a property
        // function or custom aggregate reachable from `FOLD(?v ORDER BY f(?w))`
        // would otherwise skip this walk's prepare-time admission entirely.
        let order_by = aggregate
            .order_by()
            .iter()
            .map(|order| {
                reference_plan_order_expression(order, relations, agg_registry, outer, promise)
            })
            .collect::<Result<Vec<_>, PlanError>>()?;
        // `reference_plan_expression` rewrites each argument in place and never changes the
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
        .expect("reference_plan_expression preserves argument count, so arity stays valid"))
    }

    /// Test-only recursive reference for `super::plan_order_expression`.
    fn reference_plan_order_expression(
        order: &OrderExpression,
        relations: &PropertyFunctionRegistry,
        agg_registry: &AggregateRegistry,
        outer: &DetHashSet<Variable>,
        promise: Promise<'_>,
    ) -> Result<OrderExpression, PlanError> {
        Ok(match order {
            OrderExpression::Asc(expr) => OrderExpression::Asc(reference_plan_expression(
                expr,
                relations,
                agg_registry,
                outer,
                promise,
            )?),
            OrderExpression::Desc(expr) => OrderExpression::Desc(reference_plan_expression(
                expr,
                relations,
                agg_registry,
                outer,
                promise,
            )?),
        })
    }

    /// Test-only recursive reference for `super::collect_bound`.
    fn reference_collect_bound(
        pattern: &GraphPattern,
        context: &DetHashSet<Variable>,
        written: Written<'_>,
        out: &mut DetHashSet<Variable>,
    ) {
        let beneath = written;
        match pattern {
            GraphPattern::Bgp { patterns } => {
                for triple in patterns {
                    reference_collect_triple_vars(triple, out);
                }
            }
            GraphPattern::Path {
                subject,
                path: _,
                object,
            } => {
                reference_collect_term_vars(subject, out);
                reference_collect_term_vars(object, out);
            }
            // Every flattened argument position of a call receives a value on every row it
            // emits, so its variables are certainly bound by it.
            GraphPattern::PropertyFunction(call) => {
                for term in call.subject_args.iter().chain(&call.object_args) {
                    reference_collect_term_vars(term, out);
                }
            }
            GraphPattern::Apply { left, right, policy } => {
                let mut left_bound = DetHashSet::default();
                reference_collect_bound(left, context, beneath, &mut left_bound);
                if policy.optional.is_none() {
                    let mut right_context = context.clone();
                    let mut supplied = beneath.everywhere.cloned().unwrap_or_default();
                    for (input, _) in &policy.inputs {
                        right_context.remove(input);
                        supplied.remove(input);
                    }
                    for (input, driver) in &policy.inputs {
                        if left_bound.contains(driver) || context.contains(driver) || beneath.writes(driver) {
                            right_context.insert(input.clone());
                            supplied.insert(input.clone());
                        }
                    }
                    reference_collect_bound(right, &right_context, Written { everywhere: Some(&supplied) }, out);
                }
                out.extend(left_bound);
            }
            GraphPattern::Join { left, right } => {
                reference_collect_bound(left, context, beneath, out);
                reference_collect_bound(right, context, beneath, out);
            }
            // The right operand is evaluated once per left row with that row in hand, so it
            // sees the left operand's certain bindings as well as the enclosing context's.
            GraphPattern::Lateral { left, right } => {
                let mut left_bound = DetHashSet::default();
                reference_collect_bound(left, context, beneath, &mut left_bound);
                let mut right_context = context.clone();
                right_context.extend(left_bound.iter().cloned());
                reference_collect_bound(right, &right_context, beneath, out);
                out.extend(left_bound);
            }
            // The right side may contribute nothing to a row.
            GraphPattern::LeftJoin { left, .. } | GraphPattern::Minus { left, right: _ } => {
                reference_collect_bound(left, context, beneath, out);
            }
            // Only what EVERY arm binds is bound in every row.
            GraphPattern::Union { arms } => {
                let mut common: Option<DetHashSet<Variable>> = None;
                for arm in arms {
                    let mut bound = DetHashSet::default();
                    reference_collect_bound(arm, context, beneath, &mut bound);
                    common = Some(match common {
                        None => bound,
                        Some(before) => before.intersection(&bound).cloned().collect(),
                    });
                }
                out.extend(common.unwrap_or_default());
            }
            // A `FILTER` passes only rows its condition is true on, and every variable the
            // condition requires bound for that is therefore bound in each row it passes —
            // see [`reference_truth_requires`]. A variable the run writes into the condition itself
            // (under the SHACL pre-binding rewrite) is read there as the written value, not
            // from the row, so the condition constrains nothing about the row's binding of
            // it.
            GraphPattern::Filter { expr, inner } => {
                reference_collect_bound(inner, context, written, out);
                // A condition that is never true passes no row; it is taken to bind nothing,
                // the narrow answer.
                out.extend(
                    reference_truth_requires(expr)
                        .unwrap_or_default()
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
            | GraphPattern::Unfold { inner, .. } => reference_collect_bound(inner, context, written, out),
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
                reference_collect_bound(inner, context, written, &mut inner_bound);
                if reference_expression_reads_only_bound(expression, &|read| {
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
                reference_collect_bound(inner, context, beneath, out);
            }
            // Only what the projection keeps escapes, and only if the inner pattern bound
            // it certainly. The context reaches the inner pattern only through the
            // variables the projection names; what the SHACL pre-binding rewrite writes is
            // written past the projection too.
            GraphPattern::Project { inner, variables } => {
                let inner_context = narrowed_to(context, variables);
                let mut inner_bound = DetHashSet::default();
                reference_collect_bound(inner, &inner_context, written, &mut inner_bound);
                out.extend(
                    variables
                        .iter()
                        .filter(|variable| inner_bound.contains(*variable))
                        .cloned(),
                );
            }
            // A grouping key is bound in a group's row when every row of the group binds
            // it; an aggregate's output by [`reference_aggregate_certainly_binds`].
            GraphPattern::Group {
                inner,
                variables,
                aggregates,
            } => {
                let mut inner_bound = DetHashSet::default();
                reference_collect_bound(inner, context, written, &mut inner_bound);
                let row_binds =
                    |read: &Variable| inner_bound.contains(read) || context.contains(read);
                out.extend(variables.iter().filter(|key| row_binds(key)).cloned());
                let grouped = !variables.is_empty();
                for (variable, aggregate) in aggregates {
                    if reference_aggregate_certainly_binds(aggregate, grouped, &|read| {
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
    }

    /// Test-only recursive reference for `super::aggregate_certainly_binds`.
    fn reference_aggregate_certainly_binds(
        aggregate: &AggregateExpression,
        grouped: bool,
        row_binds: &dyn Fn(&Variable) -> bool,
    ) -> bool {
        match aggregate.function() {
            AggregateFunction::Count
            | AggregateFunction::Sum
            | AggregateFunction::Avg
            | AggregateFunction::GroupConcat
            | AggregateFunction::Fold => true,
            AggregateFunction::Sample
            | AggregateFunction::Min
            | AggregateFunction::Max
            | AggregateFunction::Custom(_) => {
                grouped
                    && aggregate
                        .args()
                        .iter()
                        .all(|arg| reference_expression_reads_only_bound(arg, row_binds))
            }
        }
    }

    /// Test-only recursive reference for `super::expression_reads_only_bound`.
    fn reference_expression_reads_only_bound(
        expr: &Expression,
        is_bound: &dyn Fn(&Variable) -> bool,
    ) -> bool {
        let reads = |expr: &Expression| reference_expression_reads_only_bound(expr, is_bound);
        match expr {
            Expression::NamedNode(_)
            | Expression::Literal(_)
            | Expression::Bound(_)
            | Expression::Exists(_) => true,
            Expression::Variable(variable) => is_bound(variable),
            Expression::Or(operands) | Expression::And(operands) => operands.iter().all(reads),
            Expression::Arithmetic(first, steps) => {
                reads(first) && steps.iter().all(|(_, operand)| reads(operand))
            }
            Expression::Equal(a, b)
            | Expression::SameTerm(a, b)
            | Expression::Greater(a, b)
            | Expression::GreaterOrEqual(a, b)
            | Expression::Less(a, b)
            | Expression::LessOrEqual(a, b) => reads(a) && reads(b),
            Expression::UnaryPlus(a) | Expression::UnaryMinus(a) | Expression::Not(a) => reads(a),
            Expression::If(condition, then, otherwise) => {
                reads(condition) && reads(then) && reads(otherwise)
            }
            Expression::In(needle, haystack) => reads(needle) && haystack.iter().all(reads),
            Expression::Coalesce(items) => items.iter().any(reads),
            Expression::FunctionCall(_, args) => args.iter().all(reads),
        }
    }

    /// Test-only recursive reference for `super::truth_requires`.
    fn reference_truth_requires(expr: &Expression) -> Requires {
        match expr {
            Expression::Variable(variable) | Expression::Bound(variable) => needs(variable),
            Expression::Exists(_) => needs_nothing(),
            Expression::NamedNode(_) => None,
            Expression::Literal(literal) => {
                (crate::expr::constant_ebv(literal) == Some(true)).then(DetHashSet::default)
            }
            // A chain is its binary operator folded from the operator's identity, whose
            // requirement is the fold's: `true` (`&&`) needs nothing, `false` (`||`) is never
            // true.
            Expression::And(operands) => operands
                .iter()
                .map(reference_truth_requires)
                .fold(needs_nothing(), all_of),
            Expression::Or(operands) => operands
                .iter()
                .map(reference_truth_requires)
                .fold(None, one_of),
            Expression::Not(a) => reference_falsity_requires(a),
            Expression::FunctionCall(function, args) if is_type_test(function) => args
                .iter()
                .map(reference_value_requires)
                .fold(needs_nothing(), all_of),
            Expression::If(condition, then, otherwise) => all_of(
                reference_value_requires(condition),
                one_of(
                    all_of(
                        reference_truth_requires(condition),
                        reference_truth_requires(then),
                    ),
                    all_of(
                        reference_falsity_requires(condition),
                        reference_truth_requires(otherwise),
                    ),
                ),
            ),
            Expression::Coalesce(items) => items
                .iter()
                .map(reference_truth_requires)
                .fold(None, one_of),
            Expression::In(needle, _) => reference_value_requires(needle),
            _ => reference_value_requires(expr),
        }
    }

    /// Test-only recursive reference for `super::falsity_requires`.
    fn reference_falsity_requires(expr: &Expression) -> Requires {
        match expr {
            Expression::Variable(variable) => needs(variable),
            Expression::Bound(_) | Expression::Exists(_) => needs_nothing(),
            Expression::NamedNode(_) => None,
            Expression::Literal(literal) => {
                (crate::expr::constant_ebv(literal) == Some(false)).then(DetHashSet::default)
            }
            Expression::FunctionCall(function, _) if is_type_test(function) => needs_nothing(),
            Expression::Not(a) => reference_truth_requires(a),
            Expression::And(operands) => operands
                .iter()
                .map(reference_falsity_requires)
                .fold(None, one_of),
            Expression::Or(operands) => operands
                .iter()
                .map(reference_falsity_requires)
                .fold(needs_nothing(), all_of),
            Expression::If(condition, then, otherwise) => all_of(
                reference_value_requires(condition),
                one_of(
                    all_of(
                        reference_truth_requires(condition),
                        reference_falsity_requires(then),
                    ),
                    all_of(
                        reference_falsity_requires(condition),
                        reference_falsity_requires(otherwise),
                    ),
                ),
            ),
            Expression::Coalesce(items) => items
                .iter()
                .map(reference_falsity_requires)
                .fold(None, one_of),
            Expression::In(needle, _) => reference_value_requires(needle),
            _ => reference_value_requires(expr),
        }
    }

    /// Test-only recursive reference for `super::value_requires`.
    fn reference_value_requires(expr: &Expression) -> Requires {
        match expr {
            Expression::Variable(variable) => needs(variable),
            Expression::NamedNode(_)
            | Expression::Literal(_)
            | Expression::Bound(_)
            | Expression::Exists(_) => needs_nothing(),
            Expression::FunctionCall(function, args) => args
                .iter()
                .enumerate()
                .filter(|(position, _)| strict_in_argument(function, *position))
                .map(|(_, arg)| reference_value_requires(arg))
                .fold(needs_nothing(), all_of),
            // The pairwise `one_of`, folded; an empty chain is its identity constant, which
            // has a value for any row.
            Expression::And(operands) | Expression::Or(operands) => operands
                .iter()
                .map(reference_value_requires)
                .reduce(one_of)
                .unwrap_or_else(needs_nothing),
            Expression::Arithmetic(first, steps) => steps
                .iter()
                .map(|(_, operand)| reference_value_requires(operand))
                .fold(reference_value_requires(first), all_of),
            Expression::Equal(a, b)
            | Expression::SameTerm(a, b)
            | Expression::Greater(a, b)
            | Expression::GreaterOrEqual(a, b)
            | Expression::Less(a, b)
            | Expression::LessOrEqual(a, b) => {
                all_of(reference_value_requires(a), reference_value_requires(b))
            }
            Expression::UnaryPlus(a) | Expression::UnaryMinus(a) | Expression::Not(a) => {
                reference_value_requires(a)
            }
            Expression::If(condition, then, otherwise) => all_of(
                reference_value_requires(condition),
                one_of(
                    all_of(
                        reference_truth_requires(condition),
                        reference_value_requires(then),
                    ),
                    all_of(
                        reference_falsity_requires(condition),
                        reference_value_requires(otherwise),
                    ),
                ),
            ),
            Expression::Coalesce(items) => items
                .iter()
                .map(reference_value_requires)
                .fold(None, one_of),
            Expression::In(needle, _) => reference_value_requires(needle),
        }
    }

    /// Test-only recursive reference for `super::collect_triple_vars`.
    fn reference_collect_triple_vars(triple: &TriplePattern, out: &mut DetHashSet<Variable>) {
        reference_collect_term_vars(&triple.subject, out);
        if let NamedNodePattern::Variable(variable) = &triple.predicate {
            out.insert(variable.clone());
        }
        reference_collect_term_vars(&triple.object, out);
    }

    /// Test-only recursive reference for `super::collect_term_vars`.
    fn reference_collect_term_vars(term: &TermPattern, out: &mut DetHashSet<Variable>) {
        match term {
            TermPattern::Variable(variable) => {
                out.insert(variable.clone());
            }
            TermPattern::Triple(triple) => reference_collect_triple_vars(triple, out),
            TermPattern::NamedNode(_) | TermPattern::BlankNode(_) | TermPattern::Literal(_) => {}
        }
    }

    // ── Fixtures ───────────────────────────────────────────────────────────────────

    use purrdf_xsd::datatype::XSD_BOOLEAN;
    use purrdf_xsd::datatype::XSD_INTEGER;
    /// A `(1, 1)` relation computable only with its subject bound.
    const REL_BOUND: &str = "http://example.org/rel/bound";
    /// A `(1, 1)` relation computable in every access pattern.
    const REL_ANY: &str = "http://example.org/rel/any";
    /// A `(2, 1)` relation computable with its first subject argument bound and one
    /// more position bound.
    const REL_WIDE: &str = "http://example.org/rel/wide";
    /// A relation no registry holds.
    const REL_MISSING: &str = "http://example.org/rel/missing";
    /// A registered custom aggregate of one argument declaring one numeric scalarval.
    const AGG_SUMMARY: &str = "http://example.org/agg/summary";
    /// A custom aggregate no registry holds.
    const AGG_MISSING: &str = "http://example.org/agg/missing";
    /// The variables every generated shape is written over.
    const VARIABLES: [&str; 6] = ["a", "b", "c", "d", "e", "q"];
    /// The depth every deep case is written at.
    const DEPTH: usize = 100_000;

    /// A relation declaring `modes` at `arity`, never dispatched.
    struct Declared {
        arity: PfArity,
        modes: Vec<BindingPattern>,
        rows: u64,
    }

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
            self.arity
        }

        fn modes(&self) -> &[BindingPattern] {
            &self.modes
        }

        fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
            self.rows
        }

        fn open(
            &self,
            _args: &PfArgs<'_>,
            _ceiling: Option<u64>,
        ) -> Result<Box<dyn PfCursor>, EvalError> {
            Ok(Box::new(Empty))
        }
    }

    fn relations() -> PropertyFunctionRegistry {
        let mut registry = PropertyFunctionRegistry::new();
        registry.register(
            REL_BOUND,
            Arc::new(Declared {
                arity: PfArity::new(1, 1),
                modes: vec![BindingPattern::from_code("bf")],
                rows: 1,
            }),
        );
        registry.register(
            REL_ANY,
            Arc::new(Declared {
                arity: PfArity::new(1, 1),
                modes: vec![BindingPattern::from_code("ff")],
                rows: 10,
            }),
        );
        registry.register(
            REL_WIDE,
            Arc::new(Declared {
                arity: PfArity::new(2, 1),
                modes: vec![
                    BindingPattern::from_code("bbf"),
                    BindingPattern::from_code("bfb"),
                ],
                rows: 5,
            }),
        );
        registry
    }

    /// An accumulator that answers nothing: only the aggregate's declaration is read here.
    struct Idle;

    impl AggregateAccumulator for Idle {
        fn step(&mut self, _args: &[TermValue]) -> Result<(), EvalError> {
            Ok(())
        }

        fn combine(&mut self, _other: Box<dyn AggregateAccumulator>) -> Result<(), EvalError> {
            Ok(())
        }

        fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
            self
        }

        fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
            Ok(None)
        }
    }

    /// The custom aggregate at [`AGG_SUMMARY`].
    struct Summary;

    impl CustomAggregate for Summary {
        fn arity(&self) -> Arity {
            Arity::Exact(1)
        }

        fn volatility(&self) -> Volatility {
            Volatility::Stable
        }

        fn algebraic_class(&self) -> AlgebraicClass {
            AlgebraicClass::Commutative
        }

        fn state_bound(&self) -> u64 {
            1
        }

        fn scalarvals(&self) -> &[ScalarvalSpec] {
            &[ScalarvalSpec {
                name: "limit",
                kind: ScalarvalKind::Numeric,
            }]
        }

        fn init(&self, _scalarvals: &[(String, TermValue)]) -> Box<dyn AggregateAccumulator> {
            Box::new(Idle)
        }
    }

    fn aggregates() -> AggregateRegistry {
        let mut registry = AggregateRegistry::default();
        registry.register(AGG_SUMMARY, Arc::new(Summary));
        registry
    }

    fn variable(name: &str) -> Variable {
        Variable::new(name)
    }

    fn iri(local: &str) -> NamedNode {
        NamedNode::new_unchecked(format!("http://example.org/{local}"))
    }

    fn call_of(relation: &str, subject: &str, object: &str) -> GraphPattern {
        GraphPattern::PropertyFunction(PropertyFunctionCall {
            iri: relation.to_owned(),
            subject_args: vec![TermPattern::Variable(variable(subject))],
            object_args: vec![TermPattern::Variable(variable(object))],
        })
    }

    fn set(names: &[&str]) -> DetHashSet<Variable> {
        names.iter().map(|name| variable(name)).collect()
    }

    // ── The choice sequence ────────────────────────────────────────────────────────

    /// A deterministic choice sequence: every shape drawn from it is a pure function of
    /// the seed, so a disagreement names the seed that reproduces it.
    struct Choices(purrdf_testkit::rng::SplitMix64);

    impl Choices {
        fn below(&mut self, bound: usize) -> usize {
            self.0.below_usize(bound)
        }

        fn one_in(&mut self, bound: usize) -> bool {
            self.below(bound) == 0
        }

        fn variable(&mut self) -> Variable {
            variable(VARIABLES[self.below(VARIABLES.len())])
        }

        fn variables(&mut self, most: usize) -> Vec<Variable> {
            let count = self.below(most + 1);
            (0..count).map(|_| self.variable()).collect()
        }

        fn set(&mut self, most: usize) -> DetHashSet<Variable> {
            self.variables(most).into_iter().collect()
        }

        fn iri(&mut self) -> NamedNode {
            let local = format!("n{}", self.below(3));
            iri(&local)
        }

        fn literal(&mut self) -> Literal {
            match self.below(4) {
                0 => Literal::new_typed("true", NamedNode::new_unchecked(XSD_BOOLEAN)),
                1 => Literal::new_typed("false", NamedNode::new_unchecked(XSD_BOOLEAN)),
                2 => Literal::new_typed("7", NamedNode::new_unchecked(XSD_INTEGER)),
                _ => Literal::new_simple("x"),
            }
        }

        fn predicate(&mut self) -> NamedNodePattern {
            if self.one_in(3) {
                NamedNodePattern::Variable(self.variable())
            } else {
                NamedNodePattern::NamedNode(self.iri())
            }
        }

        /// A term, quoting a triple up to `depth` levels deep.
        fn term(&mut self, depth: usize) -> TermPattern {
            match self.below(if depth == 0 { 4 } else { 5 }) {
                0 => TermPattern::Variable(self.variable()),
                1 => TermPattern::NamedNode(self.iri()),
                2 => TermPattern::Literal(self.literal()),
                3 => TermPattern::BlankNode(BlankNode::new(format!("b{}", self.below(2)))),
                _ => TermPattern::Triple(Child::new(self.triple(depth - 1))),
            }
        }

        fn triple(&mut self, depth: usize) -> TriplePattern {
            TriplePattern {
                subject: self.term(depth),
                predicate: self.predicate(),
                object: self.term(depth),
            }
        }

        fn ground(&mut self) -> Option<GroundTerm> {
            match self.below(3) {
                0 => None,
                1 => Some(GroundTerm::NamedNode(self.iri())),
                _ => Some(GroundTerm::Literal(self.literal())),
            }
        }

        /// A call: mostly admissible, sometimes unregistered, sometimes of the wrong
        /// arity.
        fn call(&mut self) -> PropertyFunctionCall {
            let (relation, subjects) = match self.below(8) {
                0 | 1 => (REL_BOUND, 1),
                2..=4 => (REL_ANY, 1),
                5 => (REL_WIDE, 2),
                6 => (REL_MISSING, 1),
                _ => (REL_BOUND, 2),
            };
            PropertyFunctionCall {
                iri: relation.to_owned(),
                subject_args: (0..subjects).map(|_| self.term(1)).collect(),
                object_args: vec![self.term(1)],
            }
        }

        fn leaf(&mut self) -> GraphPattern {
            match self.below(6) {
                0 => GraphPattern::Bgp {
                    patterns: Vec::new(),
                },
                1 | 2 => {
                    let count = 1 + self.below(2);
                    GraphPattern::Bgp {
                        patterns: (0..count).map(|_| self.triple(1)).collect(),
                    }
                }
                3 => GraphPattern::Path {
                    subject: self.term(0),
                    path: PropertyPathExpression::NamedNode(self.iri()),
                    object: self.term(0),
                },
                4 => {
                    let variables: Vec<Variable> = self.set(2).into_iter().collect();
                    let rows = self.below(3);
                    let bindings = (0..rows)
                        .map(|_| (0..variables.len()).map(|_| self.ground()).collect())
                        .collect();
                    GraphPattern::Values {
                        variables,
                        bindings,
                    }
                }
                _ => GraphPattern::PropertyFunction(self.call()),
            }
        }

        fn child(&mut self, budget: &mut usize) -> Child<GraphPattern> {
            Child::new(self.pattern(budget))
        }

        /// A pattern of at most `budget` inner nodes, over every variant.
        fn pattern(&mut self, budget: &mut usize) -> GraphPattern {
            if *budget == 0 {
                return self.leaf();
            }
            *budget -= 1;
            match self.below(20) {
                19 => GraphPattern::Apply {
                    left: self.child(budget),
                    right: self.child(budget),
                    policy: crate::rdflib::test_application_policy(
                        vec![(self.variable(), self.variable())],
                        self.one_in(2),
                    ),
                },
                0 => GraphPattern::Join {
                    left: self.child(budget),
                    right: self.child(budget),
                },
                1 => {
                    let left = self.child(budget);
                    let right = if self.one_in(2) {
                        Child::new(GraphPattern::PropertyFunction(self.call()))
                    } else {
                        self.child(budget)
                    };
                    GraphPattern::Lateral { left, right }
                }
                // The shape the parser gives a group whose only member is a call.
                2 => GraphPattern::Lateral {
                    left: Child::new(GraphPattern::Bgp {
                        patterns: Vec::new(),
                    }),
                    right: Child::new(GraphPattern::PropertyFunction(self.call())),
                },
                3 => {
                    let left = self.child(budget);
                    let right = self.child(budget);
                    let expression = if self.one_in(2) {
                        Some(self.expression(budget))
                    } else {
                        None
                    };
                    GraphPattern::LeftJoin {
                        left,
                        right,
                        expression,
                    }
                }
                4 => GraphPattern::Minus {
                    left: self.child(budget),
                    right: self.child(budget),
                },
                5 => {
                    let count = 2 + self.below(2);
                    let arms: Vec<GraphPattern> =
                        (0..count).map(|_| self.pattern(budget)).collect();
                    GraphPattern::Union {
                        arms: Chain::try_from(arms).expect("two or more arms"),
                    }
                }
                6 => GraphPattern::Filter {
                    expr: self.expression(budget),
                    inner: self.child(budget),
                },
                7 => GraphPattern::Extend {
                    inner: self.child(budget),
                    variable: self.variable(),
                    expression: self.expression(budget),
                },
                8 => {
                    let inner = self.child(budget);
                    let expression = self.expression(budget);
                    let element = self.variable();
                    let companion = if self.one_in(2) {
                        Some(self.variable())
                    } else {
                        None
                    };
                    GraphPattern::Unfold {
                        inner,
                        expression,
                        element,
                        companion,
                    }
                }
                9 => GraphPattern::Graph {
                    name: self.predicate(),
                    inner: self.child(budget),
                },
                10 => {
                    let inner = self.child(budget);
                    let count = 1 + self.below(2);
                    let expression = (0..count)
                        .map(|_| {
                            let key = self.expression(budget);
                            if self.one_in(2) {
                                OrderExpression::Asc(key)
                            } else {
                                OrderExpression::Desc(key)
                            }
                        })
                        .collect();
                    GraphPattern::OrderBy { inner, expression }
                }
                11 => GraphPattern::Project {
                    inner: self.child(budget),
                    variables: self.variables(3),
                },
                12 => GraphPattern::Distinct {
                    inner: self.child(budget),
                },
                13 => GraphPattern::Reduced {
                    inner: self.child(budget),
                },
                14 => {
                    let inner = self.child(budget);
                    let start = self.below(2);
                    let length = if self.one_in(2) {
                        None
                    } else {
                        Some(self.below(3))
                    };
                    GraphPattern::Slice {
                        inner,
                        start,
                        length,
                    }
                }
                15 | 16 => {
                    let inner = self.child(budget);
                    let variables = self.variables(2);
                    let count = self.below(3);
                    let aggregates = (0..count)
                        .map(|_| (self.variable(), self.aggregate(budget)))
                        .collect();
                    GraphPattern::Group {
                        inner,
                        variables,
                        aggregates,
                    }
                }
                17 => GraphPattern::Service {
                    name: self.predicate(),
                    inner: self.child(budget),
                    silent: self.one_in(2),
                },
                _ => GraphPattern::Filter {
                    expr: Expression::Exists(self.child(budget)),
                    inner: self.child(budget),
                },
            }
        }

        /// An aggregate: a built-in, or a custom one that is registered, unregistered,
        /// of the wrong arity, or carrying a right, wrong-typed or unknown scalarval.
        fn aggregate(&mut self, budget: &mut usize) -> AggregateExpression {
            let integer = Literal::new_typed("3", NamedNode::new_unchecked(XSD_INTEGER));
            let built = match self.below(7) {
                0 => AggregateExpression::new(
                    AggregateFunction::Count,
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    false,
                ),
                1 => AggregateExpression::new(
                    AggregateFunction::Sum,
                    vec![self.expression(budget)],
                    Vec::new(),
                    Vec::new(),
                    false,
                ),
                2 => AggregateExpression::new(
                    AggregateFunction::Sample,
                    vec![self.expression(budget)],
                    Vec::new(),
                    Vec::new(),
                    true,
                ),
                3 => AggregateExpression::new(
                    AggregateFunction::Fold,
                    vec![self.expression(budget)],
                    Vec::new(),
                    vec![OrderExpression::Asc(self.expression(budget))],
                    false,
                ),
                4 => {
                    let args = (0..=usize::from(self.one_in(4)))
                        .map(|_| self.expression(budget))
                        .collect();
                    let scalarvals = match self.below(4) {
                        0 => Vec::new(),
                        1 => vec![("limit".to_owned(), integer)],
                        2 => vec![("limit".to_owned(), Literal::new_simple("many"))],
                        _ => vec![("other".to_owned(), integer)],
                    };
                    AggregateExpression::new(
                        AggregateFunction::Custom(NamedNode::new_unchecked(AGG_SUMMARY)),
                        args,
                        scalarvals,
                        Vec::new(),
                        false,
                    )
                }
                5 => AggregateExpression::new(
                    AggregateFunction::Custom(NamedNode::new_unchecked(AGG_MISSING)),
                    vec![self.expression(budget)],
                    Vec::new(),
                    Vec::new(),
                    false,
                ),
                _ => AggregateExpression::new(
                    AggregateFunction::Max,
                    vec![self.expression(budget)],
                    Vec::new(),
                    Vec::new(),
                    false,
                ),
            };
            built.expect("every generated aggregate is well formed")
        }

        fn expression_leaf(&mut self) -> Expression {
            match self.below(4) {
                0 => Expression::Variable(self.variable()),
                1 => Expression::Bound(self.variable()),
                2 => Expression::Literal(self.literal()),
                _ => Expression::NamedNode(self.iri()),
            }
        }

        fn operand(&mut self, budget: &mut usize) -> Child<Expression> {
            Child::new(self.expression(budget))
        }

        fn operands(&mut self, budget: &mut usize, most: usize) -> Vec<Expression> {
            let count = self.below(most + 1);
            (0..count).map(|_| self.expression(budget)).collect()
        }

        /// An expression of at most `budget` inner nodes, over every variant.
        fn expression(&mut self, budget: &mut usize) -> Expression {
            if *budget == 0 {
                return self.expression_leaf();
            }
            *budget -= 1;
            match self.below(16) {
                0 | 1 => {
                    let count = 2 + self.below(2);
                    let operands: Vec<Expression> =
                        (0..count).map(|_| self.expression(budget)).collect();
                    let chain = Chain::try_from(operands).expect("two or more operands");
                    if self.one_in(2) {
                        Expression::Or(chain)
                    } else {
                        Expression::And(chain)
                    }
                }
                2 => Expression::Equal(self.operand(budget), self.operand(budget)),
                3 => Expression::Less(self.operand(budget), self.operand(budget)),
                4 => {
                    let first = self.operand(budget);
                    let step = (ArithmeticOperator::Add, self.expression(budget));
                    let rest: Vec<(ArithmeticOperator, Expression)> = (0..self.below(2))
                        .map(|_| (ArithmeticOperator::Multiply, self.expression(budget)))
                        .collect();
                    Expression::Arithmetic(first, NonEmpty::from_parts(step, rest))
                }
                5 => Expression::UnaryMinus(self.operand(budget)),
                6 => Expression::Not(self.operand(budget)),
                7 => Expression::If(
                    self.operand(budget),
                    self.operand(budget),
                    self.operand(budget),
                ),
                8 => Expression::In(self.operand(budget), self.operands(budget, 2).into()),
                9 => Expression::Coalesce(self.operands(budget, 2).into()),
                10 => Expression::FunctionCall(Function::Str, vec![self.expression(budget)].into()),
                11 => {
                    Expression::FunctionCall(Function::IsIri, vec![self.expression(budget)].into())
                }
                12 => Expression::FunctionCall(
                    Function::Regex,
                    (0..2 + usize::from(self.one_in(2)))
                        .map(|_| self.expression(budget))
                        .collect(),
                ),
                13 => Expression::FunctionCall(Function::Concat, self.operands(budget, 2).into()),
                14 => Expression::FunctionCall(
                    Function::Custom(self.iri()),
                    self.operands(budget, 2).into(),
                ),
                _ => Expression::Exists(self.child(budget)),
            }
        }
    }

    // ── Agreement over generated shapes ────────────────────────────────────────────

    /// The planner and its reference rebuild the same plan, or refuse the same way.
    fn assert_plans_agree(
        pattern: &GraphPattern,
        outer: &DetHashSet<Variable>,
        promise: Promise<'_>,
        seed: u64,
    ) {
        let relations = relations();
        let aggregates = aggregates();
        let planned = plan_pattern(pattern, &relations, &aggregates, outer, promise);
        let reference = reference_plan_pattern(pattern, &relations, &aggregates, outer, promise);
        match (planned, reference) {
            (Ok(planned), Ok(reference)) => assert_eq!(
                planned, reference,
                "seed {seed}, {promise:?}: the planner and its reference rebuild the same plan"
            ),
            (Err(planned), Err(reference)) => {
                assert_eq!(planned.seam, reference.seam, "seed {seed}, {promise:?}");
                assert_eq!(
                    planned.to_string(),
                    reference.to_string(),
                    "seed {seed}, {promise:?}: the first refusal met is the same"
                );
            }
            (planned, reference) => panic!(
                "seed {seed}, {promise:?}: the planner and its reference disagree on admission: \
                 {planned:?} against {reference:?}"
            ),
        }
    }

    #[test]
    fn generated_patterns_plan_as_the_recursive_reference_plans_them() {
        for seed in 0..200 {
            let mut choices = Choices(purrdf_testkit::rng::SplitMix64::new(seed));
            let mut budget = 10;
            let pattern = choices.pattern(&mut budget);
            let outer = choices.set(2);
            let parameters = choices.set(3);
            assert_plans_agree(&pattern, &outer, Promise::None, seed);
            assert_plans_agree(&pattern, &outer, Promise::Everywhere(&parameters), seed);
        }
    }

    #[test]
    fn generated_patterns_bind_what_the_recursive_reference_says_they_bind() {
        for seed in 0..200 {
            let mut choices = Choices(purrdf_testkit::rng::SplitMix64::new(seed));
            let mut budget = 10;
            let pattern = choices.pattern(&mut budget);
            let context = choices.set(2);
            let parameters = choices.set(3);
            let writes = [
                Written::NOTHING,
                Written {
                    everywhere: Some(&parameters),
                },
            ];
            for (which, written) in writes.into_iter().enumerate() {
                let mut bound = context.clone();
                collect_bound(&pattern, &context, written, &mut bound);
                let mut reference = context.clone();
                reference_collect_bound(&pattern, &context, written, &mut reference);
                assert_eq!(
                    bound, reference,
                    "seed {seed}, writes {which}: the same variables are certainly bound"
                );
            }
        }
    }

    #[test]
    fn contextual_input_shadowing_survives_nested_projection_and_retry() {
        let input = Variable::new("input");
        let driver = Variable::new("driver");
        let parent_driver = Variable::new("parent_driver");
        let output = Variable::new("output");
        let iri = GroundTerm::NamedNode(NamedNode::new_unchecked("http://example.org/value"));
        let inherited = DetHashSet::from_iter([input.clone()]);
        for (first, retry, optional, expected) in [
            (false, false, false, false),
            (true, true, false, true),
            (true, false, true, false),
            (true, true, true, true),
        ] {
            let retry_driver = Variable::new("retry_driver");
            let mut policy = crate::rdflib::test_application_policy(
                vec![(input.clone(), driver.clone())],
                optional,
            );
            if let Some(optional) = &mut policy.optional {
                optional.retry_inputs = vec![(input.clone(), retry_driver.clone())];
            }
            let projected = GraphPattern::Project {
                inner: Child::new(GraphPattern::Lateral {
                    left: Child::new(GraphPattern::Extend {
                        inner: Child::new(GraphPattern::empty_bgp()),
                        variable: output.clone(),
                        expression: Expression::Variable(input.clone()),
                    }),
                    right: Child::new(GraphPattern::PropertyFunction(PropertyFunctionCall {
                        iri: REL_BOUND.to_owned(),
                        subject_args: vec![TermPattern::Variable(output.clone())],
                        object_args: vec![TermPattern::Variable(Variable::new("answer"))],
                    })),
                }),
                variables: vec![output.clone()],
            };
            let nested = GraphPattern::Apply {
                left: Child::new(GraphPattern::Values {
                    variables: vec![driver.clone(), retry_driver],
                    bindings: vec![vec![first.then(|| iri.clone()), retry.then(|| iri.clone())]],
                }),
                right: Child::new(projected),
                policy,
            };
            // An outer mapped value and a prepared parameter both supply `input`,
            // but the inner declared input shadows them even when its driver is UNDEF.
            let pattern = GraphPattern::Apply {
                left: Child::new(GraphPattern::Values {
                    variables: vec![parent_driver.clone()],
                    bindings: vec![vec![Some(iri.clone())]],
                }),
                right: Child::new(nested),
                policy: crate::rdflib::test_application_policy(
                    vec![(input.clone(), parent_driver.clone())],
                    false,
                ),
            };
            assert_eq!(
                plan_pattern(
                    &pattern,
                    &relations(),
                    &aggregates(),
                    &inherited,
                    Promise::Everywhere(&inherited)
                )
                .is_ok(),
                expected
            );
            assert_plans_agree(&pattern, &inherited, Promise::Everywhere(&inherited), 0);
            let mut certainty = pattern;
            let GraphPattern::Apply { right: nested, .. } = &mut certainty else {
                unreachable!()
            };
            let GraphPattern::Apply {
                right: projected, ..
            } = &mut **nested
            else {
                unreachable!()
            };
            let GraphPattern::Project { inner, .. } = &mut **projected else {
                unreachable!()
            };
            let GraphPattern::Lateral { left, .. } = &**inner else {
                unreachable!()
            };
            *inner = left.clone();
            let written = Written {
                everywhere: Some(&inherited),
            };
            let mut iterative = DetHashSet::default();
            collect_bound(&certainty, &inherited, written, &mut iterative);
            let mut recursive = DetHashSet::default();
            reference_collect_bound(&certainty, &inherited, written, &mut recursive);
            assert_eq!(iterative, recursive);
            assert_eq!(iterative.contains(&output), first && !optional);
        }
    }

    #[test]
    fn contextual_inputs_drive_certain_bindings_and_downstream_call_admission() {
        let driver = Variable::new("driver");
        let input = Variable::new("renamed_input");
        let output = Variable::new("output");
        for (left_bound, outer_bound, optional, expected) in [
            (true, false, false, true),
            (false, true, false, true),
            (false, false, false, false),
            (true, false, true, false),
        ] {
            let application = GraphPattern::Apply {
                left: Child::new(GraphPattern::Values {
                    variables: vec![driver.clone()],
                    bindings: vec![vec![left_bound.then(|| {
                        GroundTerm::NamedNode(NamedNode::new_unchecked("http://example.org/value"))
                    })]],
                }),
                right: Child::new(GraphPattern::Project {
                    inner: Child::new(GraphPattern::Extend {
                        inner: Child::new(GraphPattern::Bgp {
                            patterns: Vec::new(),
                        }),
                        variable: output.clone(),
                        expression: Expression::Variable(input.clone()),
                    }),
                    variables: vec![output.clone()],
                }),
                policy: crate::rdflib::test_application_policy(
                    vec![(input.clone(), driver.clone())],
                    optional,
                ),
            };
            let mut context = DetHashSet::default();
            if outer_bound {
                context.insert(driver.clone());
            }
            let mut bound = DetHashSet::default();
            collect_bound(&application, &context, Written::NOTHING, &mut bound);
            assert_eq!(bound.contains(&output), expected);
            let consumer = GraphPattern::Lateral {
                left: Child::new(application),
                right: Child::new(GraphPattern::PropertyFunction(PropertyFunctionCall {
                    iri: REL_BOUND.to_owned(),
                    subject_args: vec![TermPattern::Variable(output.clone())],
                    object_args: vec![TermPattern::Variable(Variable::new("answer"))],
                })),
            };
            assert_eq!(
                plan_pattern(
                    &consumer,
                    &relations(),
                    &aggregates(),
                    &context,
                    Promise::None
                )
                .is_ok(),
                expected,
                "{left_bound}/{outer_bound}/{optional}"
            );
        }
    }

    #[test]
    fn generated_expressions_require_what_the_recursive_reference_says_they_require() {
        for seed in 0..300 {
            let mut choices = Choices(purrdf_testkit::rng::SplitMix64::new(seed));
            let mut budget = 8;
            let expression = choices.expression(&mut budget);
            let bound = choices.set(3);
            assert_eq!(
                requires(&expression, Outcome::Truth),
                reference_truth_requires(&expression),
                "seed {seed}: what the expression needs to be true"
            );
            assert_eq!(
                requires(&expression, Outcome::Falsity),
                reference_falsity_requires(&expression),
                "seed {seed}: what the expression needs to be false"
            );
            assert_eq!(
                requires(&expression, Outcome::Value),
                reference_value_requires(&expression),
                "seed {seed}: what the expression needs for a value"
            );
            let is_bound = |variable: &Variable| bound.contains(variable);
            assert_eq!(
                expression_reads_only_bound(&expression, &is_bound),
                reference_expression_reads_only_bound(&expression, &is_bound),
                "seed {seed}: whether the expression reads only bound variables"
            );
        }
    }

    #[test]
    fn generated_spines_peel_into_the_atoms_the_recursive_reference_peels() {
        for seed in 0..200 {
            let mut choices = Choices(purrdf_testkit::rng::SplitMix64::new(seed));
            let mut budget = 10;
            let pattern = choices.pattern(&mut budget);
            let mut atoms = Vec::new();
            let chained = collect_chain(&pattern, &mut atoms);
            let mut reference = Vec::new();
            let reference_chained = reference_collect_chain(&pattern, &mut reference);
            assert_eq!(
                chained, reference_chained,
                "seed {seed}: whether the root is a chain"
            );
            assert_eq!(atoms.len(), reference.len(), "seed {seed}: the same atoms");
            for (atom, expected) in atoms.iter().zip(&reference) {
                assert!(
                    std::ptr::eq(atom.pattern, expected.pattern),
                    "seed {seed}: the same atom at position {}",
                    expected.position
                );
                assert_eq!(atom.position, expected.position, "seed {seed}");
                match (atom.call, expected.call) {
                    (Some(call), Some(expected)) => assert!(std::ptr::eq(call, expected)),
                    (None, None) => {}
                    (call, expected) => panic!("seed {seed}: {call:?} against {expected:?}"),
                }
            }
        }
    }

    #[test]
    fn generated_terms_are_bound_and_collected_as_the_recursive_reference_says() {
        for seed in 0..300 {
            let mut choices = Choices(purrdf_testkit::rng::SplitMix64::new(seed));
            let term = choices.term(3);
            let bound = choices.set(4);
            assert_eq!(
                term_is_bound(&term, &bound),
                reference_term_is_bound(&term, &bound),
                "seed {seed}: whether the term is bound under {bound:?}"
            );
            let mut collected = DetHashSet::default();
            collect_term_vars(&term, &mut collected);
            let mut reference = DetHashSet::default();
            reference_collect_term_vars(&term, &mut reference);
            assert_eq!(collected, reference, "seed {seed}: the term's variables");
        }
    }

    // ── Depth ──────────────────────────────────────────────────────────────────────

    /// A hundred thousand `DISTINCT` wrappers over one admitted call: planned, on a
    /// 128 KiB stack, to the same plan under no promise and under every parameter, and bound
    /// to the call's arguments.
    #[test]
    fn a_hundred_thousand_wrappers_are_planned_and_bound_on_a_128_kib_thread() {
        purrdf_stack::on_stack(128 * 1024, || {
            let mut pattern = call_of(REL_ANY, "a", "b");
            for _ in 0..DEPTH {
                pattern = GraphPattern::Distinct {
                    inner: Child::new(pattern),
                };
            }
            let relations = relations();
            let aggregates = aggregates();
            let outer = DetHashSet::default();
            let parameters = set(&["a"]);
            for promise in [Promise::None, Promise::Everywhere(&parameters)] {
                let planned = plan_pattern(&pattern, &relations, &aggregates, &outer, promise)
                    .unwrap_or_else(|error| panic!("{promise:?}: the call is admitted: {error}"));
                let mut depth = 0;
                let mut node = &planned;
                while let GraphPattern::Distinct { inner } = node {
                    depth += 1;
                    node = inner;
                }
                assert_eq!(depth, DEPTH, "{promise:?}: every wrapper is rebuilt");
                assert_eq!(
                    *node,
                    call_of(REL_ANY, "a", "b"),
                    "{promise:?}: the call intact"
                );
                assert_eq!(planned, pattern, "{promise:?}: the plan is the pattern");
            }
            let mut bound = DetHashSet::default();
            collect_certainly_bound(&pattern, &mut bound);
            assert_eq!(bound, set(&["a", "b"]));
        })
        .expect("spawn");
    }

    /// A hundred thousand `FILTER(BOUND(?a))` wrappers over one triple: bound, on a
    /// 128 KiB stack, to the triple's variables.
    #[test]
    fn a_hundred_thousand_filters_are_bound_on_a_128_kib_thread() {
        purrdf_stack::on_stack(128 * 1024, || {
            let mut pattern = GraphPattern::Bgp {
                patterns: vec![TriplePattern {
                    subject: TermPattern::Variable(variable("a")),
                    predicate: NamedNodePattern::NamedNode(iri("p")),
                    object: TermPattern::Variable(variable("b")),
                }],
            };
            for _ in 0..DEPTH {
                pattern = GraphPattern::Filter {
                    expr: Expression::Bound(variable("a")),
                    inner: Child::new(pattern),
                };
            }
            let mut bound = DetHashSet::default();
            collect_certainly_bound(&pattern, &mut bound);
            assert_eq!(bound, set(&["a", "b"]));
        })
        .expect("spawn");
    }

    /// A hundred thousand operators over one variable: what the expression needs for
    /// each outcome, and whether it reads only bound variables, on a 128 KiB stack.
    #[test]
    fn a_hundred_thousand_operators_are_required_and_read_on_a_128_kib_thread() {
        purrdf_stack::on_stack(128 * 1024, || {
            let mut negated = Expression::Bound(variable("a"));
            for _ in 0..DEPTH {
                negated = Expression::Not(Child::new(negated));
            }
            // An even number of negations: true exactly when `BOUND(?a)` is.
            assert_eq!(requires(&negated, Outcome::Truth), Some(set(&["a"])));
            assert_eq!(requires(&negated, Outcome::Falsity), Some(set(&[])));
            assert_eq!(requires(&negated, Outcome::Value), Some(set(&[])));
            let mut signed = Expression::Variable(variable("a"));
            for _ in 0..DEPTH {
                signed = Expression::UnaryPlus(Child::new(signed));
            }
            assert_eq!(requires(&signed, Outcome::Truth), Some(set(&["a"])));
            let a = variable("a");
            assert!(expression_reads_only_bound(&signed, &|read| *read == a));
            assert!(!expression_reads_only_bound(&signed, &|_| false));
        })
        .expect("spawn");
    }

    /// A hundred thousand joins peel into one chain, and a hundred thousand identity
    /// `LATERAL` wrappers peel down to the call they hold, on a 128 KiB stack.
    #[test]
    fn a_hundred_thousand_joins_peel_into_one_chain_on_a_128_kib_thread() {
        purrdf_stack::on_stack(128 * 1024, || {
            let leaf = || GraphPattern::Bgp {
                patterns: vec![TriplePattern {
                    subject: TermPattern::Variable(variable("a")),
                    predicate: NamedNodePattern::NamedNode(iri("p")),
                    object: TermPattern::Variable(variable("b")),
                }],
            };
            let mut spine = leaf();
            for _ in 0..DEPTH {
                spine = GraphPattern::Join {
                    left: Child::new(spine),
                    right: Child::new(leaf()),
                };
            }
            let mut atoms = Vec::new();
            assert!(collect_chain(&spine, &mut atoms));
            assert_eq!(atoms.len(), DEPTH + 1);
            assert!(
                atoms
                    .iter()
                    .enumerate()
                    .all(|(index, atom)| atom.position == index && atom.call.is_none())
            );
            let mut wrapped = call_of(REL_ANY, "a", "b");
            for _ in 0..DEPTH {
                wrapped = GraphPattern::Lateral {
                    left: Child::new(GraphPattern::Bgp {
                        patterns: Vec::new(),
                    }),
                    right: Child::new(wrapped),
                };
            }
            let (node, call) = planned_lateral_call(&wrapped).expect("the call at the bottom");
            assert_eq!(call.iri, REL_ANY);
            assert!(matches!(node, GraphPattern::PropertyFunction(_)));
        })
        .expect("spawn");
    }

    /// A quoted triple nested a hundred thousand levels deep: whether it is bound, and
    /// which variables it holds, on a 128 KiB stack.
    #[test]
    fn a_hundred_thousand_quoted_triples_are_bound_and_collected_on_a_128_kib_thread() {
        purrdf_stack::on_stack(128 * 1024, || {
            let mut term = TermPattern::Variable(variable("a"));
            for _ in 0..DEPTH {
                term = TermPattern::Triple(Child::new(TriplePattern {
                    subject: term,
                    predicate: NamedNodePattern::NamedNode(iri("p")),
                    object: TermPattern::Variable(variable("a")),
                }));
            }
            assert!(term_is_bound(&term, &set(&["a"])));
            assert!(!term_is_bound(&term, &set(&["b"])));
            let mut collected = DetHashSet::default();
            collect_term_vars(&term, &mut collected);
            assert_eq!(collected, set(&["a"]));
        })
        .expect("spawn");
    }
}
