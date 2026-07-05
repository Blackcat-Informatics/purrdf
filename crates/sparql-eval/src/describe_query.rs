// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `DESCRIBE` evaluation (§16.4).
//!
//! `DESCRIBE` returns a graph *describing* one or more resources; SPARQL leaves the
//! exact description implementation-defined. This engine uses the repo's canonical
//! **Symmetric Concise Bounded Description** ([`purrdf_core::describe`]) — the same
//! CBD the docs multi-format export uses — so `DESCRIBE`, the `purrdf` CLI, and the
//! offline browser playground all agree on what "describe" means (one authority,
//! dogfooded).
//!
//! Targets resolve to a set of subject IRIs:
//! - `DESCRIBE <iri> …` — each concrete IRI directly (no `WHERE` evaluation needed);
//! - `DESCRIBE ?v WHERE { … }` — every IRI bound to `?v` across the `WHERE` solutions;
//! - `DESCRIBE *` — every IRI bound to any variable the `WHERE` projects.
//!
//! The union SCBD of that subject set is returned as a frozen dataset.

use std::collections::BTreeSet;
use std::sync::Arc;

use purrdf_core::describe::Describer;
use purrdf_core::{RdfDataset, TermValue};
use purrdf_sparql_algebra::{GraphPattern, NamedNodePattern};

use crate::error::EvalError;
use crate::eval::{EvalCtx, eval, materialize_solutions};

/// Evaluate a `DESCRIBE` query to a frozen IR dataset: the union Symmetric CBD of its
/// resolved subject IRIs.
pub(crate) fn eval_describe(
    pattern: &GraphPattern,
    targets: &[NamedNodePattern],
    ctx: &mut EvalCtx<'_>,
) -> Result<Arc<RdfDataset>, EvalError> {
    // A `BTreeSet` gives a deterministic, deduplicated subject order.
    let mut subjects: BTreeSet<String> = BTreeSet::new();
    let mut var_targets: Vec<&str> = Vec::new();
    for target in targets {
        match target {
            NamedNodePattern::NamedNode(nn) => {
                subjects.insert(nn.as_str().to_owned());
            }
            NamedNodePattern::Variable(v) => var_targets.push(v.as_str()),
        }
    }

    // `DESCRIBE *` (no explicit targets) describes every variable the `WHERE`
    // projects; an explicit variable describes just that one. Either way we need the
    // solutions. Concrete `DESCRIBE <iri>` targets skip evaluation entirely.
    let describe_all = targets.is_empty();
    if describe_all || !var_targets.is_empty() {
        let seq = eval(pattern, ctx)?;
        let (vars, rows) = materialize_solutions(&seq, ctx);
        for (col, name) in vars.iter().enumerate() {
            if !describe_all && !var_targets.contains(&name.as_str()) {
                continue;
            }
            for row in &rows {
                // Only IRI bindings are describable subjects; a literal, blank, or
                // unbound cell contributes nothing.
                if let Some(Some(TermValue::Iri(iri))) = row.get(col) {
                    subjects.insert(iri.clone());
                }
            }
        }
    }

    Describer::new(ctx.dataset)
        .describe_iris(subjects.iter().map(String::as_str))
        .map_err(|d| EvalError::internal(format!("DESCRIBE output failed to build: {d:?}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_core::RdfDatasetBuilder;
    use purrdf_sparql_algebra::{NamedNode, TermPattern, TriplePattern, Variable};

    const KNOWS: &str = "http://ex/knows";
    const REFERS: &str = "http://ex/refersTo";

    #[test]
    fn describe_concrete_iri_is_symmetric_cbd() {
        // :a :knows :b .   :x :refersTo :a .   DESCRIBE <a> keeps BOTH — the outgoing
        // edge and the incoming one (symmetric CBD, not a forward-only CBD).
        let mut b = RdfDatasetBuilder::new();
        let knows = b.intern_iri(KNOWS);
        let refers = b.intern_iri(REFERS);
        let a = b.intern_iri("http://ex/a");
        let bb = b.intern_iri("http://ex/b");
        let x = b.intern_iri("http://ex/x");
        b.push_quad(a, knows, bb, None);
        b.push_quad(x, refers, a, None);
        let ds = b.freeze().expect("freeze");
        let mut ctx = EvalCtx::new(&ds);

        let targets = vec![NamedNodePattern::NamedNode(NamedNode::new_unchecked(
            "http://ex/a",
        ))];
        let out = eval_describe(&GraphPattern::Bgp { patterns: vec![] }, &targets, &mut ctx)
            .expect("describe");
        assert_eq!(
            out.quad_count(),
            2,
            "symmetric CBD of :a keeps the outgoing and incoming edges"
        );
    }

    #[test]
    fn describe_variable_resolves_where_bindings() {
        // :a :knows :b ; :a :knows :c .   DESCRIBE ?o WHERE { :a :knows ?o } describes
        // :b and :c — each pulls in its incoming :a :knows edge, union = the two edges.
        let mut b = RdfDatasetBuilder::new();
        let knows = b.intern_iri(KNOWS);
        let a = b.intern_iri("http://ex/a");
        let bb = b.intern_iri("http://ex/b");
        let cc = b.intern_iri("http://ex/c");
        b.push_quad(a, knows, bb, None);
        b.push_quad(a, knows, cc, None);
        let ds = b.freeze().expect("freeze");
        let mut ctx = EvalCtx::new(&ds);

        let targets = vec![NamedNodePattern::Variable(Variable::new("o"))];
        let pattern = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: TermPattern::NamedNode(NamedNode::new_unchecked("http://ex/a")),
                predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked(KNOWS)),
                object: TermPattern::Variable(Variable::new("o")),
            }],
        };
        let out = eval_describe(&pattern, &targets, &mut ctx).expect("describe");
        assert_eq!(
            out.quad_count(),
            2,
            "describing both bound objects unions their symmetric CBDs"
        );
    }

    #[test]
    fn describe_unknown_iri_is_empty() {
        let mut b = RdfDatasetBuilder::new();
        let knows = b.intern_iri(KNOWS);
        let a = b.intern_iri("http://ex/a");
        let bb = b.intern_iri("http://ex/b");
        b.push_quad(a, knows, bb, None);
        let ds = b.freeze().expect("freeze");
        let mut ctx = EvalCtx::new(&ds);

        let targets = vec![NamedNodePattern::NamedNode(NamedNode::new_unchecked(
            "http://ex/absent",
        ))];
        let out = eval_describe(&GraphPattern::Bgp { patterns: vec![] }, &targets, &mut ctx)
            .expect("describe");
        assert_eq!(out.quad_count(), 0, "describing an absent subject is empty");
    }
}
