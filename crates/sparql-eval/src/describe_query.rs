// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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

use purrdf_core::describe::Describer;
use purrdf_core::{DatasetView, TermValue};
use purrdf_sparql_algebra::{GraphPattern, NamedNodePattern};

use crate::construct::ConstructedGraph;
use crate::error::EvalError;
use crate::eval::{EvalCtx, eval_evaluated};
use crate::governor::lift::Evaluated;
use crate::solution::{SolutionSeq, VarSchema};

/// Evaluate a `DESCRIBE` query to a frozen IR dataset: the union Symmetric CBD of its
/// resolved subject IRIs.
///
/// The `WHERE`'s certificate travels beside the graph for the same reason it does for
/// `CONSTRUCT`: a description built from a certified lower bound describes a subset of
/// the true subjects, and only the caller holding the certificate can tell that from a
/// complete description.
pub(crate) fn eval_describe<D: DatasetView + Sync>(
    pattern: &GraphPattern,
    targets: &[NamedNodePattern],
    ctx: &mut EvalCtx<'_, D>,
) -> Result<ConstructedGraph<D::Id>, EvalError> {
    // A `BTreeSet` gives a deterministic, deduplicated subject order.
    enum Subject<'a> {
        Authored(&'a str),
        Resolved(crate::WorkspaceTerm),
    }
    impl Subject<'_> {
        fn iri(&self) -> &str {
            match self {
                Self::Authored(iri) => iri,
                Self::Resolved(term) => match &**term {
                    TermValue::Iri(iri) => iri,
                    _ => unreachable!("only IRI terms enter description subjects"),
                },
            }
        }
    }
    let mut subjects = crate::AdmittedVec::new(&ctx.growth);
    let mut var_targets = crate::AdmittedVec::new(&ctx.growth);
    for target in targets {
        match target {
            NamedNodePattern::NamedNode(node) => subjects.push(Subject::Authored(node.as_str()))?,
            NamedNodePattern::Variable(variable) => var_targets.push(variable.as_str())?,
        }
    }
    let describe_all = targets.is_empty();
    let (certificate, where_rows) = if describe_all || !var_targets.is_empty() {
        let (seq, certificate) = match eval_evaluated(pattern, ctx)? {
            Evaluated::Complete(seq) => (seq, None),
            Evaluated::Truncated(truncation) => (truncation.rows().clone(), Some(truncation)),
        };
        for (column, variable) in seq.schema.vars().iter().enumerate() {
            if (!describe_all && !var_targets.contains(&variable.as_str()))
                || (describe_all && crate::blank_scope::is_joined_blank(variable))
            {
                continue;
            }
            for row in &seq.rows {
                let Some(Some(term)) = row.get(column) else {
                    continue;
                };
                let value =
                    ctx.scratch
                        .try_owned_value_of(ctx.dataset, *term, &ctx.growth, |error| {
                            ctx.workspace.source_error(error)
                        })?;
                if matches!(&*value, TermValue::Iri(_)) {
                    subjects.push(Subject::Resolved(value))?;
                }
            }
        }
        (certificate, seq)
    } else {
        (None, SolutionSeq::empty(VarSchema::empty_shared()))
    };
    subjects
        .as_mut_slice()
        .sort_unstable_by(|left, right| left.iri().cmp(right.iri()));
    let mut frame = crate::workspace::LexicalFrame::new(&ctx.growth);
    let native = (|| {
        let mut memory = purrdf_lex::allocation::Memory::new(&mut frame);
        let describer = Describer::try_new_with_memory(ctx.dataset, &mut memory)?;
        let result =
            describer.describe_iris_with_memory(subjects.iter().map(Subject::iri), &mut memory);
        describer.release_with_memory(&mut memory)?;
        let mut graph = result?;
        graph.warm_query_indexes_with_memory(&mut memory)?;
        Ok::<_, purrdf_core::describe::DescribeError<D::ReadError>>(graph)
    })();
    let graph = match native {
        Ok(graph) => graph,
        Err(purrdf_core::describe::DescribeError::Source(error)) => {
            return Err(ctx.workspace.source_error(error));
        }
        Err(purrdf_core::describe::DescribeError::Storage(error)) => {
            return Err(frame.storage_error(error, "description graph storage"));
        }
        Err(purrdf_core::describe::DescribeError::Build(
            purrdf_core::NativeBuildError::Storage(error),
        )) => return Err(frame.storage_error(error, "description graph freeze")),
        Err(purrdf_core::describe::DescribeError::Build(
            purrdf_core::NativeBuildError::Diagnostic(error),
        )) => {
            return Err(EvalError::RetainedDiagnostic(
                frame.finish_diagnostic(&mut Some(error))?,
            ));
        }
        Err(error) => {
            return Err(crate::NativeDiagnostic::error(
                crate::NativeDiagnosticKind::Internal,
                &error,
                &ctx.growth,
            ));
        }
    };
    let graph = frame.finish_dataset(&mut Some(graph))?;
    crate::construct::commit_answer_triples(graph, certificate, &where_rows, ctx)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ungoverned `DESCRIBE`, which is complete by construction.
    fn eval_describe<D: DatasetView + Sync>(
        pattern: &GraphPattern,
        targets: &[NamedNodePattern],
        ctx: &mut EvalCtx<'_, D>,
    ) -> Result<purrdf_core::DatasetHandle, EvalError> {
        let (graph, certificate) = super::eval_describe(pattern, targets, ctx)?;
        assert!(
            certificate.is_none(),
            "an ungoverned DESCRIBE cannot truncate"
        );
        Ok(graph)
    }
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
