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
    #[must_use]
    pub fn reaches(&self, iri: &str) -> bool {
        self.sites.values().any(|used| used.calls.contains(iri))
    }

    /// Record one site, merging if the same label appears twice (two constraints on
    /// one shape, say). Merging rather than overwriting: a report that silently kept
    /// the last of two texts would under-report the graph.
    fn record(&mut self, site: String, text: &str, env: &ExtensionEnv) {
        // A text that does not parse under this environment contributes nothing: the
        // shapes loader already refused every unparsable body, so reaching this with
        // one means the environment's own declarations changed what the grammar
        // accepts — which is a question about the environment, not about usage, and
        // one the validation itself will raise by name.
        let Ok(query) = parse(text, env) else {
            return;
        };
        let used = predicate_use(&query);
        let entry = self.sites.entry(SparqlSite(site)).or_default();
        entry.calls.extend(used.calls);
        entry.data.extend(used.data);
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

        for shape in &self.node_shapes {
            walk_shape(shape, &mut usage, env);
        }
        usage
    }
}

/// Record every SPARQL text one node shape and its descendants carry.
fn walk_shape(shape: &Shape, usage: &mut ExtensionUsage, env: &ExtensionEnv) {
    let id = shape.id.to_string();

    for target in &shape.targets {
        if let Target::Sparql { select, .. } = target {
            usage.record(format!("sh:target on {id}"), select, env);
        }
    }
    for rule in &shape.rules {
        if let RuleBody::Sparql { construct } = &rule.body {
            usage.record(format!("sh:rule on {id}"), construct, env);
        }
    }
    walk_constraints(&shape.constraints, &id, usage, env);

    for property in &shape.property_shapes {
        walk_property(property, usage, env);
    }
}

/// Record every SPARQL text one property shape and its descendants carry.
fn walk_property(property: &PropertyShape, usage: &mut ExtensionUsage, env: &ExtensionEnv) {
    let id = property.id.to_string();
    walk_constraints(&property.constraints, &id, usage, env);
    for nested in &property.property_shapes {
        walk_property(nested, usage, env);
    }
    for reifier in &property.reifier_shapes {
        walk_shape(reifier, usage, env);
    }
}

/// Record the SPARQL texts a constraint list carries: `sh:sparql` bodies, and the
/// `sh:select`/`sh:sparqlExpr` node expressions inside `sh:expression`.
fn walk_constraints(
    constraints: &[Constraint],
    owner: &str,
    usage: &mut ExtensionUsage,
    env: &ExtensionEnv,
) {
    for constraint in constraints {
        match constraint {
            Constraint::Sparql { select, .. } => {
                usage.record(format!("sh:sparql on {owner}"), select, env);
            }
            Constraint::Expression { expr, .. } => {
                walk_node_expr(expr, owner, usage, env);
            }
            _ => {}
        }
    }
}

/// Record the SPARQL texts one node expression carries, at any depth.
fn walk_node_expr(expr: &NodeExpr, owner: &str, usage: &mut ExtensionUsage, env: &ExtensionEnv) {
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
                walk_node_expr(arg, owner, usage, env);
            }
        }
        _ => {}
    }
}
