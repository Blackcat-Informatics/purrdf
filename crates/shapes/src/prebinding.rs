// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The one iterative SHACL-SPARQL pre-binding audit.
//!
//! [REC20170720 Appendix A](https://www.w3.org/TR/2017/REC-shacl-20170720/#pre-binding)
//! forbids MINUS, SERVICE, every VALUES, assignment to a potential pre-binding,
//! and nested projections hiding required bindings. `shapesGraph`/`currentShape`
//! remain potential bindings for assignment but are optional in projections.
//! [WD20260918 Appendix A](https://www.w3.org/TR/2026/WD-shacl12-sparql-20260918/#pre-binding)
//! restricts VALUES only when it mentions a potential pre-binding and removes
//! the projection restriction. PurRDF retains a local deterministic SERVICE
//! refusal, as the draft permits.
//!
//! Legacy entrypoints retain their existing corpus-decided policy and diagnostic
//! text: strict constraints/validators/rules, narrowed parameterized AF bodies,
//! and SERVICE-only checks for unbound targets/functions. All policies use this
//! same walker, including EXISTS inside aggregate operands and order expressions.

use purrdf_sparql_algebra::{Expression, GraphPattern, OrderExpression, Query};

use crate::profile::{AdmissionReason, AdmissionRefusal, QueryPurpose, ShaclProfile};

/// Check a SHACL-SPARQL **SELECT** query (an `sh:select` constraint / validator
/// body) against the pre-binding restrictions with the given pre-bound
/// variable names (no `?`/`$` sigil).
///
/// The OUTERMOST projection is exempt from the subquery-projection rule (the
/// result mapping reads `$this` from the pre-binding, not the projection);
/// every NESTED `SELECT` must project all pre-bound variables.
///
/// # Errors
///
/// Returns `Err(String)` naming the offending construct.
pub(crate) fn check_select(query: &Query, prebound: &[&str]) -> Result<(), String> {
    let Query::Select { pattern, .. } = query else {
        // Non-SELECT forms are rejected elsewhere (shape-load SELECT-form check).
        return Ok(());
    };
    check_query_body(pattern, prebound, Rules::Strict).map_err(Violation::legacy_message)
}

/// Which reading of the pre-binding restrictions a check applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rules {
    /// The reading every SHACL-SPARQL constraint, validator and rule query gets:
    /// Appendix A's three MUSTs, plus `SERVICE`, EVERY `VALUES` (the corpus-decided
    /// divergence in the module docs) and the subquery-projection rule.
    Strict,
    /// REC20170720 Appendix A, with optional shape-context projection exemptions.
    Recommendation { required_projection: usize },
    /// Appendix A's three MUSTs, verbatim — no `MINUS`, no `VALUES` that mentions a
    /// potentially pre-bound variable, no `AS ?var` for one — and no `SERVICE`. The
    /// reading a `sh:SPARQLFunction` body and a `sh:SPARQLTargetType` query get (see
    /// [`check_function_body`], [`check_target_type`]).
    AppendixA,
    /// `SERVICE` alone: the reading a SHACL-SPARQL query that pre-binds nothing gets
    /// (see [`check_no_service`]). Appendix A's MUSTs are about pre-bound variables, so
    /// a query with none has none to break; its `SERVICE` sentence is not, and is read
    /// for every SHACL-SPARQL query.
    ServiceOnly,
}

/// The refusal of a `SERVICE` in a SHACL-SPARQL query. SHACL 1.2 SPARQL Extensions,
/// Appendix A: "Furthermore, SPARQL queries SHOULD not contain a federated query
/// (SERVICE). Implementations that do not permit SERVICE MUST report a failure as
/// mentioned above." PurRDF reads the SHOULD as a MUST and does not permit it: a
/// validation verdict that depended on what a remote endpoint answered today would not be
/// a verdict about the data graph, and this engine fetches nothing.
const SERVICE_REFUSAL: &str = "a federated query (SERVICE) is not allowed in a SHACL-SPARQL \
     query (SHACL 1.2 SPARQL Extensions, Appendix A: Pre-binding of Variables in SPARQL \
     Queries: \"SPARQL queries SHOULD not contain a federated query (SERVICE)\", read as a \
     must; PurRDF does not permit SERVICE, and reports the failure the same sentence \
     requires)";

/// Check any SHACL-SPARQL query for `SERVICE` alone — a query that pre-binds no
/// variable: a `sh:SPARQLTarget`'s `sh:select`, a `sh:SPARQLFunction` with no
/// parameters. See [`SERVICE_REFUSAL`].
///
/// # Errors
///
/// Returns `Err(String)` naming the `SERVICE`.
pub(crate) fn check_no_service(query: &Query) -> Result<(), String> {
    match query {
        Query::Select { pattern, .. }
        | Query::Construct { pattern, .. }
        | Query::Describe { pattern, .. } => check_query_body(pattern, &[], Rules::ServiceOnly),
        Query::Ask { pattern, .. } => check_pattern(pattern, &[], Rules::ServiceOnly),
    }
    .map_err(Violation::legacy_message)
}

/// Check a `sh:SPARQLTargetType`'s `sh:select` against the pre-binding restrictions,
/// with its parameter variables as the potentially pre-bound ones.
///
/// SHACL Advanced Features, "SPARQL-based Target Types": "Similar to SPARQL-based
/// constraint components, such targets take parameters and the parameter values become
/// pre-bound variables in the associated SPARQL queries", so the query is "executed with
/// pre-bound variables" and Appendix A's MUSTs apply to it, read as a function body's
/// are ([`Rules::AppendixA`]); and no `SERVICE`, as for every SHACL-SPARQL query. A target
/// type with no parameters pre-binds nothing, and only `SERVICE` is refused.
///
/// # Errors
///
/// Returns `Err(String)` naming the offending construct.
pub(crate) fn check_target_type(query: &Query, parameters: &[&str]) -> Result<(), String> {
    if parameters.is_empty() {
        return check_no_service(query);
    }
    match query {
        Query::Select { pattern, .. } => check_query_body(pattern, parameters, Rules::AppendixA)
            .map_err(Violation::legacy_message),
        _ => check_no_service(query),
    }
}

/// Check the body of a SHACL-AF `sh:SPARQLFunction` against the pre-binding
/// restrictions, with its parameter variables as the potentially pre-bound ones.
///
/// SHACL Advanced Features, "SPARQL-based Functions": "When the function is executed,
/// the SPARQL processor needs to pre-bind variables based on the provided arguments of
/// the function call", so a function body is a query "executed with pre-bound
/// variables" and Appendix A's MUSTs apply to it: no `MINUS`, no `VALUES` that mentions
/// a parameter variable, no `AS ?var` for one. The stricter extras the validators get
/// are NOT applied here: the corpus that decides them for validators has no function
/// case, and a body with a `VALUES` over its own local variables or a subquery is a
/// query Appendix A permits. `SERVICE` is refused, as in every SHACL-SPARQL query (see
/// [`SERVICE_REFUSAL`]). A function with no parameters pre-binds nothing, so only
/// `SERVICE` is restricted.
///
/// # Errors
///
/// Returns `Err(String)` naming the offending construct.
pub(crate) fn check_function_body(query: &Query, parameters: &[&str]) -> Result<(), String> {
    if parameters.is_empty() {
        return check_no_service(query);
    }
    match query {
        Query::Select { pattern, .. } => check_query_body(pattern, parameters, Rules::AppendixA)
            .map_err(Violation::legacy_message),
        Query::Ask { pattern, .. } => {
            check_pattern(pattern, parameters, Rules::AppendixA).map_err(Violation::legacy_message)
        }
        _ => Ok(()),
    }
}

/// Check a node expression's `sh:select` / `sh:sparqlExpr` query against the
/// pre-binding restrictions, with every name its evaluation pre-binds — `$this` and
/// the names its context binds or may bind — as the potentially pre-bound ones.
///
/// A node expression's query is "executed with pre-bound variables", so Appendix A's
/// MUSTs apply to it, read as a function body's are ([`Rules::AppendixA`]): no
/// `MINUS`, no `VALUES` that mentions a pre-bound name, no `AS ?var` for one, and no
/// `SERVICE`. The rule is the same for every pre-bound name: `$this`, `$value` in an
/// `sh:expression` constraint, a custom function's arguments and a free evaluation's
/// scope names. This is the SHACL surface's predicate; the engine's lanes (prepared
/// parameters, request substitutions) answer `VALUES` and `MINUS` by join semantics
/// and refuse only the reassignment
/// ([`purrdf_sparql_algebra::Query::assigned_prebound`]).
///
/// # Errors
///
/// Returns `Err(String)` naming the offending construct.
pub(crate) fn check_node_expression(query: &Query, prebound: &[&str]) -> Result<(), String> {
    check_function_body(query, prebound)
}

/// Check a SHACL-AF `sh:construct` CONSTRUCT query (a `sh:SPARQLRule` head)
/// against the pre-binding restrictions. The CONSTRUCT `WHERE` algebra is a
/// solution-producing body exactly like a SELECT's, so the same rules apply; the
/// outermost projection (if any) is exempt.
///
/// # Errors
///
/// Returns `Err(String)` naming the offending construct.
pub(crate) fn check_construct(query: &Query, prebound: &[&str]) -> Result<(), String> {
    let Query::Construct { pattern, .. } = query else {
        // Non-CONSTRUCT forms are rejected elsewhere (rule-load CONSTRUCT check).
        return Ok(());
    };
    check_query_body(pattern, prebound, Rules::Strict).map_err(Violation::legacy_message)
}

/// Build the actual purpose's potential bindings, then apply one dated policy.
pub(crate) fn admit<'a>(
    profile: ShaclProfile,
    purpose: QueryPurpose,
    query: &Query,
    parameters: impl ExactSizeIterator<Item = &'a str>,
) -> Result<(), AdmissionRefusal> {
    if !purpose.accepts(query) {
        return Err(AdmissionRefusal::new(
            profile,
            purpose,
            AdmissionReason::QueryForm,
            None,
        ));
    }
    let mut prebound = Vec::with_capacity(parameters.len() + 4);
    if purpose.binds_this() {
        prebound.push("this");
    }
    if purpose == QueryPurpose::AskValidator {
        prebound.push("value");
    }
    prebound.extend(parameters);
    let required_projection = prebound.len();
    if profile == ShaclProfile::REC_20170720 && purpose.binds_shape_context() {
        prebound.extend(["shapesGraph", "currentShape"]);
    }

    let rules = if profile == ShaclProfile::REC_20170720 {
        Rules::Recommendation {
            required_projection,
        }
    } else if prebound.is_empty() {
        Rules::ServiceOnly
    } else if profile == ShaclProfile::WD_20260918
        || matches!(purpose, QueryPurpose::Function | QueryPurpose::TargetType)
    {
        Rules::AppendixA
    } else {
        Rules::Strict
    };
    match query {
        Query::Select { pattern, .. } | Query::Construct { pattern, .. } => {
            check_query_body(pattern, &prebound, rules)
        }
        Query::Ask { pattern, .. } => check_pattern(pattern, &prebound, rules),
        Query::Describe { .. } => unreachable!("purpose form was checked before admission"),
    }
    .map_err(|violation| {
        AdmissionRefusal::new(profile, purpose, violation.reason(), violation.variable)
    })
}

/// A structural refusal retained independently of any diagnostic wording.
struct Violation {
    kind: ViolationKind,
    variable: Option<String>,
}

#[derive(Clone, Copy)]
enum ViolationKind {
    ContextualApplication,
    Minus,
    Service,
    Values,
    PreboundValues,
    Assignment,
    UnfoldAssignment,
    AggregateAssignment,
    SubqueryProjection,
}

impl Violation {
    fn new(kind: ViolationKind) -> Self {
        Self {
            kind,
            variable: None,
        }
    }

    fn variable(kind: ViolationKind, name: &str) -> Self {
        Self {
            kind,
            variable: Some(name.to_owned()),
        }
    }

    fn reason(&self) -> AdmissionReason {
        match self.kind {
            ViolationKind::ContextualApplication => AdmissionReason::ContextualApplication,
            ViolationKind::Minus => AdmissionReason::Minus,
            ViolationKind::Service => AdmissionReason::Service,
            ViolationKind::Values | ViolationKind::PreboundValues => AdmissionReason::Values,
            ViolationKind::Assignment
            | ViolationKind::UnfoldAssignment
            | ViolationKind::AggregateAssignment => AdmissionReason::Assignment,
            ViolationKind::SubqueryProjection => AdmissionReason::SubqueryProjection,
        }
    }

    /// Preserve the exact existing String-based API's refusal text.
    fn legacy_message(self) -> String {
        let name = self.variable.as_deref().unwrap_or("");
        match self.kind {
            ViolationKind::ContextualApplication => "contextual application algebra requires its typed preparation route; it cannot be pre-bound as ordinary SHACL algebra".to_owned(),
            ViolationKind::Minus => {
                "MINUS is not allowed in a query with pre-bound variables (SHACL 1.2 SPARQL \
             Extensions, Appendix A: Pre-binding of Variables in SPARQL Queries)"
                    .to_owned()
            }
            ViolationKind::Service => SERVICE_REFUSAL.to_owned(),
            ViolationKind::Values => {
                "VALUES is not allowed in a query with pre-bound variables (SHACL 1.2 SPARQL \
             Extensions, Appendix A: Pre-binding of Variables in SPARQL Queries; PurRDF \
             refuses every VALUES, which is stricter than the Working Draft's \
             mentions-a-pre-bound-variable rule, because the frozen W3C pre-binding corpus \
             requires the stricter reading)"
                    .to_owned()
            }
            ViolationKind::PreboundValues => format!(
                "a VALUES clause that mentions the potentially pre-bound variable ?{name} is \
                 not allowed (SHACL 1.2 SPARQL Extensions, Appendix A: Pre-binding of \
                 Variables in SPARQL Queries)"
            ),
            ViolationKind::Assignment => format!(
                "assigning a potentially pre-bound variable (... AS ?{name}) is not allowed \
                 (SHACL 1.2 SPARQL Extensions, Appendix A: Pre-binding of Variables in \
                 SPARQL Queries)"
            ),
            ViolationKind::UnfoldAssignment => format!(
                "assigning a potentially pre-bound variable (UNFOLD(... AS ?{name})) is not \
                 allowed (SHACL-SPARQL §5.2.1)"
            ),
            ViolationKind::AggregateAssignment => format!(
                "assigning a potentially pre-bound variable (aggregate AS ?{name}) is not \
                 allowed (SHACL 1.2 SPARQL Extensions, Appendix A: Pre-binding of \
                 Variables in SPARQL Queries)"
            ),
            ViolationKind::SubqueryProjection => format!(
                "a subquery must project every potentially pre-bound variable; \
                 ?{name} is not in its projection (SHACL 1.2 SPARQL Extensions, \
                 Appendix A: Pre-binding of Variables in SPARQL Queries)"
            ),
        }
    }
}

/// Strip the outer solution modifiers down to the outermost `Project` and check
/// its BODY — nested `Project`s inside the body are subqueries. Shared by
/// [`check_select`] and [`check_construct`].
fn check_query_body(
    pattern: &GraphPattern,
    prebound: &[&str],
    rules: Rules,
) -> Result<(), Violation> {
    let mut node = pattern;
    loop {
        match node {
            GraphPattern::Slice { inner, .. }
            | GraphPattern::Distinct { inner }
            | GraphPattern::Reduced { inner } => node = inner,
            GraphPattern::OrderBy { inner, expression } => {
                for order in expression {
                    let (OrderExpression::Asc(e) | OrderExpression::Desc(e)) = order;
                    check_expression(e, prebound, rules)?;
                }
                node = inner;
            }
            GraphPattern::Project { inner, .. } => return check_pattern(inner, prebound, rules),
            other => return check_pattern(other, prebound, rules),
        }
    }
}

/// Refuse a query that assigns `$shapesGraph` or `$currentShape` — `BIND(… AS
/// ?shapesGraph)`, `(… AS ?currentShape)`, at any depth.
///
/// Both are potentially pre-bound wherever a shape runs a query — a `sh:sparql`
/// constraint, a SPARQL-based component's validator, a `sh:SPARQLRule` — and the
/// evaluation declares both whether or not this run has a value for them
/// (`crate::sparql::absent_shape_context`), so the engine refuses the assignment on
/// every run ([`Query::assigned_prebound`], the one definition of the refusal).
/// Refusing it here, where the shapes graph loads, is what keeps that from being a
/// validation that loads green and then aborts. Only the ASSIGNMENT is refused: a
/// sub-`SELECT` need not project either name, which the strict
/// subquery-projection rule would otherwise demand if they were passed to
/// [`check_select`] as pre-bound.
///
/// # Errors
///
/// Returns `Err(String)` naming the assigned variable.
pub(crate) fn check_shape_context_unassigned(query: &Query) -> Result<(), String> {
    match query.assigned_prebound(&crate::sparql::THIS_AND_SHAPE_CONTEXT[1..]) {
        Some(variable) => Err(format!(
            "the query assigns ?{}, which SHACL-SPARQL pre-binds: a pre-bound variable may \
             not be reassigned",
            variable.as_str()
        )),
        None => Ok(()),
    }
}

/// Check a SHACL-SPARQL **ASK** query (an `sh:ask` validator body) against the
/// pre-binding restrictions. Every `SELECT` inside an ASK body is a subquery,
/// so the subquery-projection rule applies throughout.
///
/// # Errors
///
/// Returns `Err(String)` naming the offending construct.
pub(crate) fn check_ask(query: &Query, prebound: &[&str]) -> Result<(), String> {
    match query {
        Query::Ask { pattern, .. } => {
            check_pattern(pattern, prebound, Rules::Strict).map_err(Violation::legacy_message)
        }
        _ => Ok(()),
    }
}

/// One entry of the pre-binding check's work list.
#[derive(Clone, Copy)]
enum Pending<'a> {
    Pattern(&'a GraphPattern),
    Expr(&'a Expression),
}

/// Walk a graph pattern, rejecting every construct the pre-binding
/// restrictions forbid.
fn check_pattern(pattern: &GraphPattern, prebound: &[&str], rules: Rules) -> Result<(), Violation> {
    check(Pending::Pattern(pattern), prebound, rules)
}

/// Walk an expression tree; `EXISTS { … }` bodies are graph patterns and are
/// checked too.
fn check_expression(expr: &Expression, prebound: &[&str], rules: Rules) -> Result<(), Violation> {
    check(Pending::Expr(expr), prebound, rules)
}

/// Check `root` and everything under it the restrictions reach, depth first over a
/// work list: each node's own restrictions before its operands', its operands in
/// written order, so the first construct refused is the first one written.
fn check(root: Pending<'_>, prebound: &[&str], rules: Rules) -> Result<(), Violation> {
    let mut pending = vec![root];
    while let Some(next) = pending.pop() {
        let first = pending.len();
        match next {
            Pending::Pattern(pattern) => {
                check_pattern_node(pattern, prebound, rules, &mut pending)?;
            }
            Pending::Expr(expr) => check_expression_node(expr, &mut pending),
        }
        pending[first..].reverse();
    }
    Ok(())
}

/// A pattern node's own restrictions; its operands are queued, in written order, on
/// `pending`.
fn check_pattern_node<'a>(
    pattern: &'a GraphPattern,
    prebound: &[&str],
    rules: Rules,
    pending: &mut Vec<Pending<'a>>,
) -> Result<(), Violation> {
    match pattern {
        GraphPattern::Apply { .. } => {
            return Err(Violation::new(ViolationKind::ContextualApplication));
        }
        // A property-function call's argument vectors are term positions, exactly like
        // a BGP triple's or a property path's endpoints: a pre-bound variable there is
        // constrained by the pre-binding rewrite and changes no SPARQL semantics, so
        // there is nothing for Appendix A to forbid. The restricted constructs are the ones
        // whose *evaluation* breaks under pre-binding (`MINUS`, `SERVICE`, `VALUES`) or
        // that would ASSIGN a pre-bound variable; a call does neither.
        GraphPattern::Bgp { .. }
        | GraphPattern::Path { .. }
        | GraphPattern::PropertyFunction(_) => {}
        GraphPattern::Minus { left, right } if rules == Rules::ServiceOnly => {
            pending.extend([Pending::Pattern(left), Pending::Pattern(right)]);
        }
        GraphPattern::Minus { .. } => {
            return Err(Violation::new(ViolationKind::Minus));
        }
        GraphPattern::Service { .. } => return Err(Violation::new(ViolationKind::Service)),
        GraphPattern::Values { .. } if rules == Rules::ServiceOnly => {}
        GraphPattern::Values { variables, .. } if rules == Rules::AppendixA => {
            if let Some(variable) = variables
                .iter()
                .find(|variable| prebound.contains(&variable.as_str()))
            {
                return Err(Violation::variable(
                    ViolationKind::PreboundValues,
                    variable.as_str(),
                ));
            }
        }
        GraphPattern::Values { .. } => {
            return Err(Violation::new(ViolationKind::Values));
        }
        GraphPattern::Join { left, right } | GraphPattern::Lateral { left, right } => {
            pending.extend([Pending::Pattern(left), Pending::Pattern(right)]);
        }
        GraphPattern::Union { arms } => pending.extend(arms.iter().map(Pending::Pattern)),
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => {
            pending.extend([Pending::Pattern(left), Pending::Pattern(right)]);
            pending.extend(expression.iter().map(Pending::Expr));
        }
        GraphPattern::Filter { expr, inner } => {
            pending.extend([Pending::Expr(expr), Pending::Pattern(inner)]);
        }
        GraphPattern::Graph { inner, .. } => pending.push(Pending::Pattern(inner)),
        GraphPattern::Extend {
            inner,
            variable,
            expression,
        } => {
            if prebound.contains(&variable.as_str()) {
                return Err(Violation::variable(
                    ViolationKind::Assignment,
                    variable.as_str(),
                ));
            }
            pending.extend([Pending::Expr(expression), Pending::Pattern(inner)]);
        }
        // `UNFOLD` ASSIGNS its one or two targets exactly as `BIND` assigns its
        // one, so §5.2.1s "must not assign a potentially pre-bound variable" rule
        // applies to both, checked in declaration order.
        GraphPattern::Unfold {
            inner,
            expression,
            element,
            companion,
        } => {
            for variable in std::iter::once(element).chain(companion.as_ref()) {
                if prebound.contains(&variable.as_str()) {
                    return Err(Violation::variable(
                        ViolationKind::UnfoldAssignment,
                        variable.as_str(),
                    ));
                }
            }
            pending.extend([Pending::Expr(expression), Pending::Pattern(inner)]);
        }
        GraphPattern::OrderBy { inner, expression } => {
            for order in expression {
                let (OrderExpression::Asc(e) | OrderExpression::Desc(e)) = order;
                pending.push(Pending::Expr(e));
            }
            pending.push(Pending::Pattern(inner));
        }
        // A nested SELECT (subquery): its projection must expose every
        // potentially pre-bound variable. A `SELECT *` expands (in the
        // algebra) to the body's in-scope variables — a FILTER-only body
        // exposes nothing, so `$this` is NOT projected and the query must be
        // rejected (W3C pre-binding-006).
        GraphPattern::Project { inner, .. }
            if matches!(rules, Rules::AppendixA | Rules::ServiceOnly) =>
        {
            pending.push(Pending::Pattern(inner));
        }
        GraphPattern::Project { inner, variables } => {
            let required = match rules {
                Rules::Recommendation {
                    required_projection,
                } => &prebound[..required_projection],
                _ => prebound,
            };
            for name in required {
                if !variables.iter().any(|v| v.as_str() == *name) {
                    return Err(Violation::variable(ViolationKind::SubqueryProjection, name));
                }
            }
            pending.push(Pending::Pattern(inner));
        }
        GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. } => pending.push(Pending::Pattern(inner)),
        GraphPattern::Group {
            inner,
            variables: _,
            aggregates,
        } => {
            for (variable, aggregate) in aggregates {
                if prebound.contains(&variable.as_str()) {
                    return Err(Violation::variable(
                        ViolationKind::AggregateAssignment,
                        variable.as_str(),
                    ));
                }
                pending.extend(aggregate.args().iter().map(Pending::Expr));
                for order in aggregate.order_by() {
                    let (OrderExpression::Asc(expression) | OrderExpression::Desc(expression)) =
                        order;
                    pending.push(Pending::Expr(expression));
                }
            }
            pending.push(Pending::Pattern(inner));
        }
    }
    Ok(())
}

/// Queue an expression node's operands, in written order, on `pending`; an
/// `EXISTS { … }` body is a graph pattern and is checked as one.
fn check_expression_node<'a>(expr: &'a Expression, pending: &mut Vec<Pending<'a>>) {
    match expr {
        Expression::NamedNode(_)
        | Expression::Literal(_)
        | Expression::Variable(_)
        | Expression::Bound(_) => {}
        Expression::Or(operands) | Expression::And(operands) => {
            pending.extend(operands.iter().map(Pending::Expr));
        }
        Expression::Arithmetic(first, steps) => {
            pending.push(Pending::Expr(first));
            pending.extend(steps.iter().map(|(_, operand)| Pending::Expr(operand)));
        }
        Expression::Equal(a, b)
        | Expression::SameTerm(a, b)
        | Expression::Greater(a, b)
        | Expression::GreaterOrEqual(a, b)
        | Expression::Less(a, b)
        | Expression::LessOrEqual(a, b) => pending.extend([Pending::Expr(a), Pending::Expr(b)]),
        Expression::UnaryPlus(inner) | Expression::UnaryMinus(inner) | Expression::Not(inner) => {
            pending.push(Pending::Expr(inner));
        }
        Expression::In(head, rest) => {
            pending.push(Pending::Expr(head));
            pending.extend(rest.iter().map(Pending::Expr));
        }
        Expression::If(c, t, e) => {
            pending.extend([Pending::Expr(c), Pending::Expr(t), Pending::Expr(e)]);
        }
        Expression::Coalesce(items) | Expression::FunctionCall(_, items) => {
            pending.extend(items.iter().map(Pending::Expr));
        }
        Expression::Exists(pattern) => pending.push(Pending::Pattern(pattern)),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_sparql_algebra::SparqlParser;

    fn parse(q: &str) -> Query {
        SparqlParser::new().parse_query(q).expect("query parses")
    }

    fn check(q: &str) -> Result<(), String> {
        check_select(&parse(q), &["this"])
    }

    #[test]
    fn contextual_application_is_rejected_directly_and_inside_exists() {
        use purrdf_sparql_algebra::algebra::ApplicationPolicy;
        use purrdf_sparql_algebra::tree::Child;

        let application = GraphPattern::Apply {
            left: Child::new(GraphPattern::empty_bgp()),
            right: Child::new(GraphPattern::empty_bgp()),
            policy: Box::new(ApplicationPolicy {
                dataset_required: false,
                row_pipeline: false,
                reduced_adjacent: false,
                group_domain: None,
                inputs: Vec::new(),
                optional: None,
            }),
        };
        let nested = GraphPattern::Filter {
            inner: Child::new(GraphPattern::empty_bgp()),
            expr: Expression::Exists(Child::new(application.clone())),
        };
        for profile in [ShaclProfile::REC_20170720, ShaclProfile::WD_20260918] {
            for pattern in [&application, &nested] {
                let mut query = parse("ASK {}");
                let Query::Ask { pattern: root, .. } = &mut query else {
                    unreachable!("ASK fixture")
                };
                *root = pattern.clone();
                let refusal = profile
                    .admit_query(QueryPurpose::AskValidator, &query, &[])
                    .expect_err("contextual algebra is not a SHACL preparation route");
                assert_eq!(refusal.profile(), profile);
                assert_eq!(refusal.purpose(), QueryPurpose::AskValidator);
                assert_eq!(refusal.reason(), AdmissionReason::ContextualApplication);
                assert_eq!(refusal.variable(), None);
                assert!(refusal.to_string().contains(
                    "contextual application algebra requires its typed preparation route"
                ));
                assert_eq!(
                    check_ask(&query, &["this"]).expect_err("legacy route refusal"),
                    "contextual application algebra requires its typed preparation route; it cannot be pre-bound as ordinary SHACL algebra"
                );
            }
            assert!(
                profile
                    .admit_query(QueryPurpose::AskValidator, &parse("ASK {}"), &[])
                    .is_ok()
            );
        }
        for rules in [Rules::Strict, Rules::AppendixA, Rules::ServiceOnly] {
            for pattern in [&application, &nested] {
                let error =
                    check_pattern(pattern, &["this"], rules).expect_err("typed route required");
                assert!(matches!(error.kind, ViolationKind::ContextualApplication));
                assert_eq!(error.reason(), AdmissionReason::ContextualApplication);
                assert!(error.legacy_message().contains(
                    "contextual application algebra requires its typed preparation route"
                ));
            }
            assert!(check_pattern(&GraphPattern::empty_bgp(), &["this"], rules).is_ok());
        }
    }

    /// The pre-binding audit reaches the SAME verdict whether a relation IRI was
    /// recognized as a call or left as an ordinary triple pattern.
    ///
    /// # Why this has to be pinned rather than assumed
    ///
    /// Every SHACL construct that carries SPARQL text is audited here at shapes-load
    /// time, under DEFAULT parser options — no registered relation IRIs — while the
    /// text it will actually evaluate is re-parsed later against the extension
    /// environment in force. Two parses of the same text, and the audit only ever
    /// sees one of them.
    ///
    /// That is sound today, and not by accident: `check_pattern` carries an explicit
    /// `GraphPattern::PropertyFunction(_) => Ok(())` arm beside `Bgp`/`Path`, and the
    /// parser assembles calls as left-deep `Lateral { Bgp, PropertyFunction }` chains
    /// that the `Join | Lateral` arm recurses through. So a call node is audited as
    /// the leaf it is, and the two parses agree.
    ///
    /// BOTH directions are asserted, because they fail differently:
    ///
    /// * blind-`Ok` but bound-`Err` would REFUSE a shapes graph a host can only load
    ///   by un-registering its relations — an over-refusal, the mirror of the silent
    ///   drop, and invisible because the gate stays green;
    /// * blind-`Err` but bound-`Ok` would ACCEPT at evaluation what the loader
    ///   rejected, which means the audit protecting pre-binding semantics is not
    ///   auditing the query that runs.
    #[test]
    fn the_prebinding_audit_agrees_under_both_parses() {
        use purrdf_sparql_algebra::ParserOptions;

        const REL: &str = "http://example.org/rel/near";

        let bound_options = ParserOptions {
            property_fn_iris: vec![REL.to_owned()],
            ..ParserOptions::default()
        };

        // Fixtures spanning the operators the audit actually decides on, each with
        // the relation IRI in predicate position so the two parses genuinely differ.
        let fixtures = [
            format!("SELECT $this WHERE {{ $this <{REL}> ?o }}"),
            format!("SELECT $this WHERE {{ $this ?p ?o OPTIONAL {{ $this <{REL}> ?o2 }} }}"),
            format!("SELECT $this WHERE {{ {{ $this <{REL}> ?o }} UNION {{ $this ?p ?o }} }}"),
            format!("SELECT $this WHERE {{ $this ?p ?o MINUS {{ $this <{REL}> ?o2 }} }}"),
            format!("SELECT $this WHERE {{ VALUES ?x {{ 1 }} $this <{REL}> ?o }}"),
            format!("SELECT $this WHERE {{ {{ SELECT $this WHERE {{ $this <{REL}> ?o }} }} }}"),
            format!("SELECT $this WHERE {{ $this ?p ?o FILTER EXISTS {{ $this <{REL}> ?o2 }} }}"),
            format!("SELECT $this WHERE {{ GRAPH ?g {{ $this <{REL}> ?o }} }}"),
        ];

        for text in &fixtures {
            let blind = SparqlParser::new()
                .parse_query(text)
                .expect("the fixture parses under default options");
            let bound = SparqlParser::new()
                .parse_query_with(text, &bound_options)
                .expect("the fixture parses under the relation-aware options");

            // The two parses really are different algebra, or this asserts nothing.
            assert_ne!(
                blind, bound,
                "the fixture must lower differently under the two option sets: {text}"
            );

            let blind_verdict = check_select(&blind, &["this"]);
            let bound_verdict = check_select(&bound, &["this"]);
            assert_eq!(
                blind_verdict, bound_verdict,
                "the audit's verdict moved between the blind and the bound parse of \
                 the same text, so the load-time audit is not auditing the query that \
                 runs: {text}",
            );
        }
    }

    /// The template-reading and projection-reading checks are environment-independent
    /// by construction: a relation call is only ever lowered in a WHERE clause, so a
    /// check that reads a CONSTRUCT template or a projection list cannot see one.
    #[test]
    fn a_construct_template_audit_is_unaffected_by_a_recognized_relation() {
        use purrdf_sparql_algebra::ParserOptions;

        const REL: &str = "http://example.org/rel/near";
        let bound_options = ParserOptions {
            property_fn_iris: vec![REL.to_owned()],
            ..ParserOptions::default()
        };
        let text = format!(
            "CONSTRUCT {{ $this <http://example.org/out> ?o }} WHERE {{ $this <{REL}> ?o }}"
        );

        let blind = SparqlParser::new().parse_query(&text).expect("parses");
        let bound = SparqlParser::new()
            .parse_query_with(&text, &bound_options)
            .expect("parses");
        assert_ne!(blind, bound, "the WHERE clause lowers differently");
        assert_eq!(
            check_construct(&blind, &["this"]),
            check_construct(&bound, &["this"]),
        );
    }

    #[test]
    fn plain_bgp_and_filter_pass() {
        assert!(check("SELECT $this WHERE { $this ?p ?o . FILTER($this != ?o) }").is_ok());
    }

    #[test]
    fn minus_is_rejected() {
        let err = check("SELECT $this WHERE { $this ?p ?o . MINUS { $this ?p \"x\" } }")
            .expect_err("MINUS must be rejected");
        assert!(err.contains("MINUS"), "{err}");
    }

    #[test]
    fn values_is_rejected() {
        let err = check("SELECT $this WHERE { $this ?p ?o . VALUES ?o { 1 2 } }")
            .expect_err("VALUES must be rejected");
        assert!(err.contains("VALUES"), "{err}");
    }

    /// A `VALUES` that mentions NO potentially pre-bound variable is still
    /// rejected, and the refusal says which document it is answering to.
    ///
    /// This is the recorded divergence made observable at RUNTIME rather than as a
    /// comment: the Working Draft's Appendix A would admit this query, the frozen
    /// W3C case `unsupported-sparql-002` requires it to be refused, and the corpus
    /// decides. The error names the Appendix so an operator who hits it can read
    /// the text PurRDF is stricter than.
    #[test]
    fn values_over_no_prebound_variable_is_still_rejected_and_cites_the_appendix() {
        let err = check("SELECT $this WHERE { $this ?p ?o . VALUES ?any { true } }")
            .expect_err("the conformance corpus requires every VALUES to be rejected");
        assert!(
            err.contains("Appendix A: Pre-binding of Variables in SPARQL Queries"),
            "the refusal must name the Working Draft appendix it diverges from: {err}"
        );
        assert!(
            err.contains("SHACL 1.2 SPARQL Extensions"),
            "the refusal must name the document: {err}"
        );
    }

    /// Every other pre-binding refusal cites the same Working Draft appendix, so a
    /// caller sees one consistent authority rather than a mix of spec versions.
    #[test]
    fn every_prebinding_refusal_cites_the_working_draft_appendix() {
        const APPENDIX: &str = "Appendix A: Pre-binding of Variables in SPARQL Queries";
        let refusals = [
            check("SELECT $this WHERE { $this ?p ?o . MINUS { $this ?p \"x\" } }"),
            check("SELECT $this WHERE { SERVICE <http://example.org/sparql> { $this ?p ?o } }"),
            check("SELECT $this WHERE { BIND(true AS $this) }"),
            check("SELECT $this WHERE { $this ?x ?any . { SELECT ?o WHERE { ?o ?b ?c } } }"),
            check(
                "SELECT $this WHERE { { SELECT ?g (COUNT(?g) AS $this) WHERE { ?g ?p ?o } \
                 GROUP BY ?g } }",
            ),
        ];
        for refusal in refusals {
            let err = refusal.expect_err("the construct must be rejected");
            assert!(err.contains(APPENDIX), "refusal does not cite it: {err}");
        }
    }

    #[test]
    fn service_is_rejected() {
        let err =
            check("SELECT $this WHERE { SERVICE <http://example.org/sparql> { $this ?p ?o } }")
                .expect_err("SERVICE must be rejected");
        assert!(err.contains("SERVICE"), "{err}");
    }

    #[test]
    fn aggregate_exists_is_audited_with_a_local_graph_neighbor() {
        let refused = "SELECT $this (SUM(IF(EXISTS {
            SERVICE <http://example.org/sparql> { $this ?p ?o }
        }, 1, 0)) AS ?count) WHERE { $this ?p ?o } GROUP BY $this";
        let err = check(refused).expect_err("aggregate EXISTS must not hide SERVICE");
        assert!(err.contains("SERVICE"), "{err}");

        let admitted = "SELECT $this (SUM(IF(EXISTS {
            GRAPH <http://example.org/graph> { $this ?p ?o }
        }, 1, 0)) AS ?count) WHERE { $this ?p ?o } GROUP BY $this";
        check(admitted).expect("the local GRAPH neighbor is admissible");
    }

    #[test]
    fn bind_as_prebound_is_rejected() {
        let err = check("SELECT $this WHERE { BIND(true AS $this) }")
            .expect_err("BIND ... AS $this must be rejected");
        assert!(err.contains("pre-bound"), "{err}");
    }

    #[test]
    fn bind_of_prebound_into_other_var_passes() {
        // Using $this INSIDE the expression is fine (pre-binding-004); only
        // ASSIGNING to it is restricted.
        assert!(check("SELECT $this WHERE { BIND($this AS ?that) }").is_ok());
    }

    #[test]
    fn subquery_not_projecting_this_is_rejected() {
        let err = check(
            "SELECT $this WHERE { $this ?x ?any . { SELECT ?other WHERE { ?other ?b ?c } } }",
        )
        .expect_err("subquery without $this must be rejected");
        assert!(err.contains("subquery"), "{err}");
    }

    #[test]
    fn subquery_projecting_this_passes() {
        assert!(check("SELECT $this WHERE { { SELECT $this WHERE { $this ?p ?o } } }").is_ok());
    }

    #[test]
    fn outer_projection_without_this_is_not_a_subquery() {
        // The OUTERMOST projection is exempt: the result mapping reads $this
        // from the pre-binding, not the projection.
        assert!(check("SELECT ?o WHERE { $this ?p ?o }").is_ok());
    }

    #[test]
    fn ask_bind_as_value_is_rejected() {
        let q = parse("ASK { BIND(true AS ?value) . FILTER(isLiteral(?value)) }");
        let err =
            check_ask(&q, &["this", "value"]).expect_err("ASK BIND AS ?value must be rejected");
        assert!(err.contains("pre-bound"), "{err}");
    }
}
