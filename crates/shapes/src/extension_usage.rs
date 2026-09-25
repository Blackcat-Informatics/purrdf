// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Which of a shapes graph's predicate IRIs would reach a registered relation, under
//! a given extension environment — asked and answered BEFORE any validation runs.
//!
//! # The question this exists to answer
//!
//! A host wires a relation, writes a shapes graph that names it, validates, and gets
//! `conforms: true`. Did the relation run?
//!
//! Running the validation cannot tell it. A relation IRI the environment does not
//! recognize lowers to a perfectly ordinary triple pattern, matches whatever the data
//! graph holds for that predicate — usually nothing — and answers. An empty answer is
//! also exactly what a correctly-resolved relation over no matching rows returns. The
//! two outcomes are indistinguishable from the report, which is why the defect this
//! module was written alongside went unnoticed: a `sh:SPARQLFunction` body could not
//! reach a relation at all, and nothing said so.
//!
//! # Why this rather than a refusal at load
//!
//! Refusing a shapes graph that names a registered relation IRI is unimplementable in
//! the direction it would have to run: a `Shapes` is parsed once, with no registry in
//! scope, and validated many times under whatever registry each caller installs.
//! There is nothing at load time for a refusal to consult.
//!
//! The inverse direction is answerable, and is strictly more useful: given the
//! environment a host is ABOUT to validate under, report what that environment makes
//! of every SPARQL text the shapes graph carries. A host that expected a call and is
//! told it has a data edge has the answer the silent verdict could never give it.

use std::collections::BTreeMap;

use purrdf_sparql_algebra::{Query, SparqlParser};
use purrdf_sparql_eval::{ExtensionEnv, PredicateUse, predicate_use};

use crate::expression::NodeExpr;
use crate::rules::RuleBody;
use crate::shapes::{Constraint, PropertyShape, Shape, Shapes, Target};

/// Which declaration a SPARQL text came from.
///
/// Rendered rather than structured: a site is a label a human reads when asking "why
/// did my relation not run?", and every construct names itself differently (a
/// function by its IRI, a constraint by its shape). One rendering keeps the report
/// readable without a consumer having to match on six shapes to print it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SparqlSite(String);

impl SparqlSite {
    /// The label, as a report prints it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl core::fmt::Display for SparqlSite {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

/// What one environment makes of every SPARQL text a shapes graph carries.
///
/// Site-keyed and site-sorted, so the report is a pure function of the shapes graph
/// and the environment rather than of traversal order.
#[derive(Clone, Debug, Default)]
pub struct ExtensionUsage {
    sites: BTreeMap<SparqlSite, PredicateUse>,
    unreadable: BTreeMap<SparqlSite, String>,
}

impl ExtensionUsage {
    /// Every site, with what its predicates became, in site order.
    pub fn sites(&self) -> impl Iterator<Item = (&SparqlSite, &PredicateUse)> {
        self.sites.iter()
    }

    /// What `site`'s predicates became, if this shapes graph has such a site.
    #[must_use]
    pub fn site(&self, site: &str) -> Option<&PredicateUse> {
        self.sites
            .iter()
            .find_map(|(key, used)| (key.as_str() == site).then_some(used))
    }

    /// Every predicate IRI that became a relation call, anywhere in the graph.
    #[must_use]
    pub fn calls(&self) -> std::collections::BTreeSet<&str> {
        self.sites
            .values()
            .flat_map(|used| used.calls.iter().map(String::as_str))
            .collect()
    }

    /// Every predicate IRI that stayed an ordinary triple pattern, anywhere.
    ///
    /// An IRI a host believes it registered appearing here is the answer it came for.
    #[must_use]
    pub fn data(&self) -> std::collections::BTreeSet<&str> {
        self.sites
            .values()
            .flat_map(|used| used.data.iter().map(String::as_str))
            .collect()
    }

    /// Whether `iri` would reach a relation anywhere in this shapes graph.
    ///
    /// Read [`Self::unreadable`] alongside this: a `false` from a graph with
    /// unreadable sites means "not in the sites we could read", which is a weaker
    /// claim than "not anywhere".
    #[must_use]
    pub fn reaches(&self, iri: &str) -> bool {
        self.sites.values().any(|used| used.calls.contains(iri))
    }

    /// The sites whose SPARQL text this environment could not parse, each with the
    /// parser's own diagnostic, in site order.
    ///
    /// Non-empty means the shapes graph and the environment genuinely disagree, and
    /// that validating under this environment will fail at these sites. The loader
    /// accepted every one of them, so the disagreement is the environment's
    /// declarations changing what the grammar accepts — a `CONSTRUCT` template
    /// naming a declared relation IRI is the reachable case, because the parser
    /// refuses a call in a template.
    ///
    /// Reported rather than skipped, and rather than collapsing the whole report
    /// into one error: a host asking "what will my environment do to this graph?"
    /// is better served by the complete picture with the broken sites named than by
    /// the first failure and nothing else.
    pub fn unreadable(&self) -> impl Iterator<Item = (&SparqlSite, &str)> {
        self.unreadable
            .iter()
            .map(|(site, why)| (site, why.as_str()))
    }

    /// Whether every SPARQL text in the graph could be read under this environment.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.unreadable.is_empty()
    }

    /// Record one site, merging if the same label appears twice (two constraints on
    /// one shape, say). Merging rather than overwriting: a report that silently kept
    /// the last of two texts would under-report the graph.
    fn record(&mut self, site: String, text: &str, env: &ExtensionEnv) {
        // A text that does not parse under THIS environment is a real answer, not a
        // nothing, and it is recorded rather than skipped.
        //
        // The shapes loader parsed every body blind, so a text that reaches here and
        // fails did so BECAUSE of the environment's own declarations — the parser
        // refuses a property-function call in a `CONSTRUCT` template, for instance,
        // so a `sh:rule` whose template names a declared relation IRI loads fine and
        // then cannot be read under the environment that declares it.
        //
        // Dropping the site made the report say the shape carries no SPARQL at all,
        // which is indistinguishable from a shape that genuinely carries none — a
        // silent answer from the one instrument that exists to end silent answers,
        // and the same failure this module documents in its header.
        match parse(text, env) {
            Ok(query) => {
                let used = predicate_use(&query);
                let entry = self.sites.entry(SparqlSite(site)).or_default();
                entry.calls.extend(used.calls);
                entry.data.extend(used.data);
            }
            Err(why) => {
                self.unreadable.insert(SparqlSite(site), why);
            }
        }
    }
}

/// Parse `text` under `env`'s effective options — the same options the evaluation
/// will use, which is the whole point of asking.
fn parse(text: &str, env: &ExtensionEnv) -> Result<Query, String> {
    SparqlParser::new()
        .parse_query_with(text, env.parser_options())
        .map_err(|e| e.to_string())
}

impl Shapes {
    /// What `env` makes of every SPARQL text this shapes graph carries.
    ///
    /// See the [module docs](crate::extension_usage) for why this is the answerable
    /// direction of "will my relations be reached?".
    #[must_use]
    pub fn extension_usage(&self, env: &ExtensionEnv) -> ExtensionUsage {
        let mut usage = ExtensionUsage::default();

        // `sh:SPARQLFunction` bodies — the construct whose blindness occasioned all
        // of this. Their text is the declaration's own, prefix header included.
        for (iri, body) in self.functions.sparql_bodied_texts() {
            usage.record(format!("sh:SPARQLFunction <{iri}>"), &body, env);
        }

        // `sh:SPARQLTargetType` declarations. The header is injected per instance, so
        // the declaration's own text is what is on record here.
        for (iri, declaration) in &self.target_types {
            usage.record(
                format!("sh:SPARQLTargetType <{iri}>"),
                &declaration.select,
                env,
            );
        }

        let mut flight = InFlight::default();
        for shape in &self.node_shapes {
            walk_shape(shape, &mut usage, env, &mut flight);
        }
        usage
    }
}

/// What one `extension_usage` walk is currently inside.
///
/// # Why one guard for the whole walk rather than one per expression
///
/// The shapes graph is a GRAPH, not a tree, and the two edges that make it one were
/// both added when the shape-valued and custom-function-body fields started being
/// followed: a node expression reaches a shape (`sh:filterShape`), and a shape
/// reaches a node expression (`sh:expression`). Either alone is finite. Together they
/// close a cycle -- a function whose body filters through a shape whose expression
/// calls that function -- and a shapes graph may legally contain one, because the
/// evaluator bounds the same recursion at run time with a depth limit rather than
/// refusing it at load.
///
/// A guard scoped to a single node-expression walk cannot see that cycle: crossing
/// into a shape starts a new walk, which starts a new guard, which has forgotten
/// every function already in flight. The traversal then recurses until the process
/// aborts -- and a stack overflow is an ABORT, not an error, so no caller can catch
/// it and the Python surface simply dies. One guard for the whole walk is what makes
/// the cycle visible at the point it closes.
///
/// # The two halves are deliberately NOT the same shape
///
/// `fns` is a STACK: an entry is removed on the way out. It has to be, because a
/// function body's site label embeds the owner it was reached through
/// (`{owner} via <iri>`), so the same function called from two disjoint branches
/// produces two DIFFERENT sites and dropping the second is a silent under-report.
/// `a_function_called_from_two_branches_is_walked_both_times` fails if this becomes a
/// visited set.
///
/// `shapes` is a VISITED SET: an entry is never removed. A shape's contents are
/// recorded under the shape's OWN id, never the caller's, so visiting one twice
/// merges into the same entry and contributes nothing the first visit did not. Re-walking
/// it is therefore pure cost — and on a graph where several expressions filter through
/// one shared shape, cost that compounds with depth. Cutting on first visit is not an
/// approximation here; it is the same report for strictly less work.
///
/// Spelling them alike would mean one of them is wrong: a stack for shapes buys
/// re-walks nobody can observe, and a visited set for functions loses sites.
#[derive(Default)]
struct InFlight {
    /// Custom-function IRIs whose `sh:bodyExpression` is currently being walked.
    /// Removed on the way out — see the type docs.
    fns: std::collections::BTreeSet<String>,
    /// Shape ids already walked. Never removed — see the type docs.
    shapes: std::collections::BTreeSet<String>,
}

/// Record every SPARQL text one node shape and its descendants carry.
fn walk_shape(
    shape: &Shape,
    usage: &mut ExtensionUsage,
    env: &ExtensionEnv,
    flight: &mut InFlight,
) {
    let id = shape.id.to_string();
    if !flight.shapes.insert(id.clone()) {
        // Already walked. Its constraints are recorded under its own id, so a second
        // visit would merge into the identical entry -- the same report, for the cost
        // of walking the whole subtree again. This also cuts the shape-to-expression
        // cycle a `sh:filterShape` can close.
        return;
    }

    walk_targets(&shape.targets, &id, usage, env, flight);
    for rule in &shape.rules {
        if let RuleBody::Sparql { construct } = &rule.body {
            usage.record(format!("sh:rule on {id}"), construct, env);
        }
    }
    walk_constraints(&shape.constraints, &id, usage, env, flight);

    for property in &shape.property_shapes {
        walk_property(property, usage, env, flight);
    }
}

/// Record every SPARQL text one property shape and its descendants carry.
fn walk_property(
    property: &PropertyShape,
    usage: &mut ExtensionUsage,
    env: &ExtensionEnv,
    flight: &mut InFlight,
) {
    let id = property.id.to_string();
    if let Some(expr) = &property.values {
        walk_node_expr(expr, &format!("sh:values on {id}"), usage, env, flight);
    }
    if let Some(expr) = &property.default_value {
        walk_node_expr(
            expr,
            &format!("sh:defaultValue on {id}"),
            usage,
            env,
            flight,
        );
    }
    walk_constraints(&property.constraints, &id, usage, env, flight);
    for nested in &property.property_shapes {
        walk_property(nested, usage, env, flight);
    }
    for reifier in &property.reifier_shapes {
        walk_shape(reifier, usage, env, flight);
    }
}

/// Record the SPARQL texts a shape's target declarations carry: a SHACL-SPARQL
/// `sh:target`'s query, the `sh:select` inside a node-expression `sh:targetNode`,
/// and whatever the `sh:targetWhere` shape carries. Wildcard-free, so a target
/// kind added later is a compile error here rather than a silent gap.
fn walk_targets(
    targets: &[Target],
    owner: &str,
    usage: &mut ExtensionUsage,
    env: &ExtensionEnv,
    flight: &mut InFlight,
) {
    for target in targets {
        match target {
            Target::Sparql { select, .. } => {
                usage.record(format!("sh:target on {owner}"), select, env);
            }
            Target::NodeExpression(expr) => {
                walk_node_expr(
                    expr,
                    &format!("sh:targetNode on {owner}"),
                    usage,
                    env,
                    flight,
                );
            }
            Target::Where(shape) => walk_shape(shape, usage, env, flight),
            Target::Class(_)
            | Target::SubjectsOf(_)
            | Target::ObjectsOf(_)
            | Target::Node(_)
            | Target::ImplicitClass(_) => {}
        }
    }
}

/// Record the SPARQL texts a constraint list carries: `sh:sparql` bodies, the
/// `sh:select`/`sh:sparqlExpr` node expressions inside `sh:expression` and
/// `sh:nodeByExpression`, and every shape a constraint nests — an INLINE nested
/// shape (`sh:node [ … ]`, `sh:or ( [ … ] )`) is anonymous and never appears in
/// `Shapes::node_shapes`, so nothing else reaches it. Wildcard-free for the reason
/// [`walk_node_expr`] is.
fn walk_constraints(
    constraints: &[Constraint],
    owner: &str,
    usage: &mut ExtensionUsage,
    env: &ExtensionEnv,
    flight: &mut InFlight,
) {
    for constraint in constraints {
        match constraint {
            Constraint::Sparql { select, .. } => {
                usage.record(format!("sh:sparql on {owner}"), select, env);
            }
            Constraint::Expression { expr, .. } | Constraint::NodeByExpression { expr, .. } => {
                walk_node_expr(expr, owner, usage, env, flight);
            }
            Constraint::Not(shape)
            | Constraint::Node(shape)
            | Constraint::MemberShape(shape)
            | Constraint::SomeValue(shape) => walk_shape(shape, usage, env, flight),
            Constraint::And(shapes) | Constraint::Or(shapes) | Constraint::Xone(shapes) => {
                for shape in shapes {
                    walk_shape(shape, usage, env, flight);
                }
            }
            Constraint::QualifiedValueShape {
                shape, siblings, ..
            } => {
                walk_shape(shape, usage, env, flight);
                for sibling in siblings {
                    walk_shape(sibling, usage, env, flight);
                }
            }
            Constraint::UniqueValuesFor { targets, .. } => {
                walk_targets(targets, owner, usage, env, flight);
            }
            // A custom constraint component's validator is query text this shapes
            // graph carries and every validation of the shape executes.
            Constraint::Component {
                component,
                validator,
                ..
            } => {
                let (form, text) = match validator {
                    crate::shapes::ComponentValidator::Ask { ask } => ("sh:ask", ask),
                    crate::shapes::ComponentValidator::Select { select } => ("sh:select", select),
                };
                usage.record(
                    format!("{form} validator of <{}> on {owner}", component.as_str()),
                    text,
                    env,
                );
            }
            Constraint::Class(_)
            | Constraint::Datatype(_)
            | Constraint::NodeKind(_)
            | Constraint::MinCount(_)
            | Constraint::MaxCount(_)
            | Constraint::In(_)
            | Constraint::HasValue(_)
            | Constraint::Pattern { .. }
            | Constraint::MinLength(_)
            | Constraint::MaxLength(_)
            | Constraint::UniqueLang(_)
            | Constraint::LanguageIn(_)
            | Constraint::Closed { .. }
            | Constraint::MinInclusive(_)
            | Constraint::MaxInclusive(_)
            | Constraint::MinExclusive(_)
            | Constraint::MaxExclusive(_)
            | Constraint::Equals(_)
            | Constraint::Disjoint(_)
            | Constraint::SubsetOf(_)
            | Constraint::LessThan(_)
            | Constraint::LessThanOrEquals(_)
            | Constraint::MinListLength(_)
            | Constraint::MaxListLength(_)
            | Constraint::UniqueMembers(_)
            | Constraint::SingleLine(_)
            | Constraint::RootClass(_) => {}
        }
    }
}

/// Record the SPARQL texts one node expression carries, at any depth.
///
/// `flight` is the ONE guard for the whole walk; see [`InFlight`] for why it cannot
/// be scoped to this call.
fn walk_node_expr(
    expr: &NodeExpr,
    owner: &str,
    usage: &mut ExtensionUsage,
    env: &ExtensionEnv,
    flight: &mut InFlight,
) {
    match expr {
        NodeExpr::Select { query, key, .. } => {
            usage.record(format!("{key} node expression on {owner}"), query, env);
        }
        // Every call kind carries its arguments as node expressions, and a `sh:select`
        // can sit inside any of them. The three arms are spelled out rather than
        // wildcarded so a fourth call kind fails to compile here instead of silently
        // dropping out of the report.
        NodeExpr::Call(
            crate::expression::FnCall::Builtin { args, .. }
            | crate::expression::FnCall::UserDefined { args, .. }
            | crate::expression::FnCall::Sparql { args, .. },
        ) => {
            for arg in args {
                walk_node_expr(arg, owner, usage, env, flight);
            }
        }
        // Every remaining OPERAND-bearing variant, because a `sh:select` can sit
        // inside any of them. This used to end in a `_ => {}`, which silently
        // dropped the whole `Union`/`If`/`Filter`/`FlatMap`/`CustomCall` family --
        // a report that answered "this graph reaches no relation" about a graph it
        // had not finished reading.
        NodeExpr::Union(items) | NodeExpr::Intersection(items) | NodeExpr::Concat(items) => {
            for item in items {
                walk_node_expr(item, owner, usage, env, flight);
            }
        }
        NodeExpr::If { cond, then, els } => {
            walk_node_expr(cond, owner, usage, env, flight);
            walk_node_expr(then, owner, usage, env, flight);
            walk_node_expr(els, owner, usage, env, flight);
        }
        NodeExpr::Count { of, .. }
        | NodeExpr::Distinct(of)
        | NodeExpr::Min(of)
        | NodeExpr::Max(of)
        | NodeExpr::Sum(of)
        | NodeExpr::Limit { of, .. }
        | NodeExpr::Offset { of, .. }
        | NodeExpr::Exists(of) => walk_node_expr(of, owner, usage, env, flight),
        NodeExpr::OrderBy { of, key, .. } => {
            walk_node_expr(of, owner, usage, env, flight);
            walk_node_expr(key, owner, usage, env, flight);
        }
        NodeExpr::Filter { nodes, shape }
        | NodeExpr::FindFirst { nodes, shape }
        | NodeExpr::MatchAll { nodes, shape } => {
            walk_node_expr(nodes, owner, usage, env, flight);
            walk_shape(shape, usage, env, flight);
        }
        NodeExpr::Remove { nodes, remove } => {
            walk_node_expr(nodes, owner, usage, env, flight);
            walk_node_expr(remove, owner, usage, env, flight);
        }
        NodeExpr::FlatMap { nodes, map } => {
            walk_node_expr(nodes, owner, usage, env, flight);
            walk_node_expr(map, owner, usage, env, flight);
        }
        NodeExpr::PathValues { focus, .. } => {
            walk_node_expr(focus, owner, usage, env, flight);
        }
        NodeExpr::ConformsToShape { node, shape } => {
            walk_node_expr(node, owner, usage, env, flight);
            match shape {
                crate::expression::ShapeArg::Named(shape) => walk_shape(shape, usage, env, flight),
                // The shape IRI is COMPUTED per evaluation, so which shape this
                // reaches is not a fact about the graph; the expression that
                // computes it is, and it is walked.
                crate::expression::ShapeArg::Computed { expr, .. } => {
                    walk_node_expr(expr, owner, usage, env, flight);
                }
            }
        }
        NodeExpr::CustomCall { func, args } => {
            for (_, arg) in args {
                walk_node_expr(arg, owner, usage, env, flight);
            }
            // The function's own `sh:bodyExpression`. A `sh:select` inside it is
            // SPARQL this shapes graph carries and this environment will read, so
            // leaving it out made the report answer "reaches nothing" about a graph
            // whose evaluation invokes the relation. The body is interned once the
            // whole graph has parsed, so an unfilled cell means the declaration never
            // linked and there is nothing to read.
            let iri = func.iri.as_str().to_owned();
            if flight.fns.insert(iri.clone()) {
                if let Some(body) = func.body.get() {
                    walk_node_expr(body, &format!("{owner} via <{iri}>"), usage, env, flight);
                }
                // Removed whether or not a body was there. Leaving it in on the
                // `None` arm would quietly turn this stack into a visited set for
                // that one IRI, so a second, unrelated call to the same function
                // later in the graph would be skipped and its body go unreported.
                flight.fns.remove(&iri);
            }
        }
        // Leaves: no nested node expression, so nothing to walk. Spelled out rather
        // than wildcarded so a variant added later lands here as a compile error --
        // which is the property the `_ => {}` this replaced had thrown away.
        //
        // The SHAPE-valued fields above ARE followed. The earlier reasoning -- that a
        // shape is reached by the shape walk anyway, so following it here would
        // double-report -- is false for the case that matters: an INLINE shape
        // (`sh:filterShape [ ... ]`) is anonymous and never appears in
        // `Shapes::node_shapes`, which collects top-level shape ids only, so nothing
        // else reaches it and its `sh:sparql` vanished from the report entirely while
        // the evaluator went on invoking the relation inside it.
        //
        // Following a NAMED shape twice is harmless rather than inflationary: a site
        // is keyed by the shape's own id, so both visits land on one entry and merge
        // set-wise. Recording the same predicate twice is the same report.
        NodeExpr::Constant(_)
        | NodeExpr::This
        | NodeExpr::Path(_)
        | NodeExpr::Empty
        | NodeExpr::Var(_)
        | NodeExpr::Arg(_)
        | NodeExpr::List(_)
        | NodeExpr::InstancesOf(_) => {}
        NodeExpr::NodesMatching(shape) => walk_shape(shape, usage, env, flight),
    }
}
