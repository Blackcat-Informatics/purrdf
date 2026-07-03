// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SPARQL `SERVICE` federation: the [`RemoteQuerySource`] seam and the
//! [`eval_service`] handler.
//!
//! `SERVICE [SILENT] <endpoint> { pattern }` evaluates `pattern` at a remote
//! endpoint and joins the result into the surrounding query. The evaluator stays
//! transport-agnostic: it forwards the inner pattern (serialized to a `SELECT *`
//! query via [`purrdf_sparql_algebra::pattern_to_select_query`]) to an injected
//! [`RemoteQuerySource`] and interns the returned bindings into a
//! [`SolutionSeq`]. The parser wraps `SERVICE` in `Join(left, Service)`, so
//! `eval_service` returns *only* the remote bag — the existing hash join performs
//! the federation join.
//!
//! # Seam, not a baked client
//!
//! [`RemoteQuerySource`] is the dependency-inversion seam:
//! [`crate::remote_http::HttpRemoteQuerySource`] builds/decodes SPARQL Protocol
//! requests through a caller-supplied HTTP transport, and [`LocalRemoteQuerySource`]
//! dog-foods the local engine in memory. This keeps the core query path wasm-clean
//! and makes `SERVICE` deterministically testable offline.
//!
//! # Hard-fail vs SILENT
//!
//! With no source configured, a variable endpoint, a transport error, or an
//! undecodable response: a **non-silent** `SERVICE` raises [`EvalError::Remote`]
//! (the query aborts), while `SERVICE SILENT` swallows the failure to the join
//! identity (one empty row) so the surrounding query proceeds unchanged.

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use purrdf_core::{RdfDataset, TermValue};
use purrdf_sparql_algebra::{GraphPattern, NamedNodePattern, Variable};

use crate::error::EvalError;
use crate::eval::{materialize_solutions, EvalCtx};
use crate::solution::{SolutionSeq, VarSchema};

/// One remote `SELECT` result set, dataset-independent (egress [`TermValue`]
/// space). Dense over `variables`; a `None` cell is an unbound binding.
#[derive(Debug, Clone)]
pub struct ResolvedBindings {
    /// The result variables, in result order.
    pub variables: Vec<Variable>,
    /// One row per solution; `rows[i][j]` is the value of `variables[j]`.
    pub rows: Vec<Vec<Option<TermValue>>>,
}

/// A failure while resolving a `SERVICE` step. Whether it aborts the query or is
/// swallowed is decided by [`eval_service`] from the `SILENT` flag, not here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteError {
    /// The endpoint was unreachable / the request failed at the transport layer.
    Transport(String),
    /// The endpoint responded, but the body could not be decoded into bindings.
    Decode(String),
    /// Federation is disabled for this source.
    Disabled,
}

impl core::fmt::Display for RemoteError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Transport(m) => write!(f, "transport: {m}"),
            Self::Decode(m) => write!(f, "decode: {m}"),
            Self::Disabled => write!(f, "federation disabled"),
        }
    }
}

impl std::error::Error for RemoteError {}

/// A source that resolves a forwarded SPARQL `SELECT` query at a `SERVICE`
/// endpoint. Object-safe so [`EvalCtx`] can hold a `&dyn RemoteQuerySource`.
pub trait RemoteQuerySource {
    /// Forward `query_text` (a complete `SELECT * WHERE { … }`) to `endpoint` and
    /// return its bindings.
    ///
    /// # Errors
    ///
    /// Returns [`RemoteError`] on transport or decode failure; `eval_service`
    /// decides whether `SILENT` swallows it.
    fn query(&self, endpoint: &str, query_text: &str) -> Result<ResolvedBindings, RemoteError>;
}

/// Evaluate a `SERVICE [SILENT] name { inner }` node to the remote result bag.
///
/// The surrounding `Join` performs the federation join, so this returns only the
/// remote bindings (or the join identity on a swallowed `SILENT` failure).
///
/// # Errors
///
/// Returns [`EvalError::Remote`] for a non-silent failure (no source, variable
/// endpoint, transport/decode error).
pub(crate) fn eval_service(
    name: &NamedNodePattern,
    inner: &GraphPattern,
    silent: bool,
    ctx: &mut EvalCtx<'_>,
) -> Result<SolutionSeq, EvalError> {
    // Resolve the endpoint IRI. A variable endpoint needs per-row (lateral)
    // resolution, which the engine defers — so it is a hard error unless SILENT.
    let endpoint = match name {
        NamedNodePattern::NamedNode(n) => n.as_str().to_owned(),
        NamedNodePattern::Variable(_) => {
            return silent_or_err(silent, || {
                "SERVICE with a variable endpoint is not supported (needs lateral evaluation)"
                    .to_owned()
            });
        }
    };

    // `Option<&dyn _>` is `Copy`, so this does NOT borrow `ctx` — leaving `&mut
    // ctx` free for interning the result below.
    let Some(source) = ctx.remote else {
        return silent_or_err(silent, || {
            format!("no remote query source configured for SERVICE <{endpoint}>")
        });
    };

    let query_text = purrdf_sparql_algebra::pattern_to_select_query(inner);
    let resolved = match source.query(&endpoint, &query_text) {
        Ok(resolved) => resolved,
        Err(e) => {
            return silent_or_err(silent, || format!("SERVICE <{endpoint}>: {e}"));
        }
    };

    Ok(ingest(resolved, ctx))
}

/// On `SILENT`, return the join identity (one empty row, a no-op for the
/// surrounding join); otherwise raise [`EvalError::Remote`] with `msg()`.
fn silent_or_err(silent: bool, msg: impl FnOnce() -> String) -> Result<SolutionSeq, EvalError> {
    if silent {
        Ok(identity_seq())
    } else {
        Err(EvalError::remote(msg()))
    }
}

/// The join identity: a single empty-binding row. `Join(left, identity) == left`,
/// so a swallowed `SERVICE SILENT` leaves the surrounding query unchanged.
fn identity_seq() -> SolutionSeq {
    SolutionSeq {
        schema: Rc::new(VarSchema::new()),
        rows: vec![vec![]],
    }
}

/// Intern a remote result's owned [`TermValue`]s into the per-query scratch space,
/// yielding a [`SolutionSeq`] over the result schema. (Mirrors `modifier::eval_values`
/// but carries `TermValue` directly, so remote blank nodes survive — `GroundTerm`
/// has no blank-node variant.)
fn ingest(resolved: ResolvedBindings, ctx: &mut EvalCtx<'_>) -> SolutionSeq {
    let schema = Rc::new(VarSchema::from_vars(resolved.variables));
    let width = schema.len();
    let mut rows = Vec::with_capacity(resolved.rows.len());
    for binding in resolved.rows {
        let mut row = vec![None; width];
        for (i, cell) in binding.into_iter().enumerate().take(width) {
            if let Some(value) = cell {
                row[i] = Some(ctx.scratch.intern(ctx.dataset, value));
            }
        }
        rows.push(row);
    }
    SolutionSeq { schema, rows }
}

/// An in-memory [`RemoteQuerySource`] that **dog-foods the native engine**: each
/// endpoint IRI maps to a local [`RdfDataset`], and a forwarded query is parsed
/// and evaluated against it with [`NativeSparqlEngine`](crate::NativeSparqlEngine)
/// semantics. Deterministic and network-free — the test/conformance vehicle for
/// `SERVICE`.
#[derive(Debug, Default)]
pub struct LocalRemoteQuerySource {
    datasets: HashMap<String, Arc<RdfDataset>>,
}

impl LocalRemoteQuerySource {
    /// An empty source with no endpoints.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register `dataset` as the contents of `endpoint`.
    #[must_use]
    pub fn with_endpoint(mut self, endpoint: impl Into<String>, dataset: Arc<RdfDataset>) -> Self {
        self.datasets.insert(endpoint.into(), dataset);
        self
    }
}

impl RemoteQuerySource for LocalRemoteQuerySource {
    fn query(&self, endpoint: &str, query_text: &str) -> Result<ResolvedBindings, RemoteError> {
        let dataset = self
            .datasets
            .get(endpoint)
            .ok_or_else(|| RemoteError::Transport(format!("no in-memory endpoint <{endpoint}>")))?;
        let parsed = purrdf_sparql_algebra::SparqlParser::new()
            .parse_query(query_text)
            .map_err(|e| RemoteError::Decode(e.to_string()))?;
        // Thread this source into the forwarded evaluation so a nested SERVICE
        // inside the forwarded query resolves against the same in-memory sources
        // rather than hard-failing on a missing remote.
        let mut ctx = EvalCtx::new(dataset).with_remote(self);
        match crate::eval::evaluate_query(&parsed, &mut ctx)
            .map_err(|e| RemoteError::Decode(e.to_string()))?
        {
            crate::eval::Outcome::Solutions(seq) => {
                let (variables, rows) = materialize_solutions(&seq, &ctx);
                Ok(ResolvedBindings {
                    variables: variables.into_iter().map(Variable::new).collect(),
                    rows,
                })
            }
            _ => Err(RemoteError::Decode(
                "SERVICE expects a SELECT query".to_owned(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NativeSparqlEngine;
    use purrdf_core::{RdfDatasetBuilder, RdfLiteral, SparqlEngine, SparqlRequest, SparqlResult};

    /// `:a :knows :x`, `:a :knows :y` (the local graph).
    fn local() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let knows = b.intern_iri("http://ex/knows");
        let a = b.intern_iri("http://ex/a");
        let x = b.intern_iri("http://ex/x");
        let y = b.intern_iri("http://ex/y");
        b.push_quad(a, knows, x, None);
        b.push_quad(a, knows, y, None);
        b.freeze().expect("freeze")
    }

    /// `:x :name "X"` (the remote endpoint graph) — only :x has a name.
    fn endpoint() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let name = b.intern_iri("http://ex/name");
        let x = b.intern_iri("http://ex/x");
        let xn = b.intern_literal(RdfLiteral::simple("X"));
        b.push_quad(x, name, xn, None);
        b.freeze().expect("freeze")
    }

    fn run_with_source(
        ds: &Arc<RdfDataset>,
        source: &dyn RemoteQuerySource,
        query: &str,
    ) -> Result<SparqlResult, EvalError> {
        use crate::eval::evaluate_query;
        let parsed = purrdf_sparql_algebra::SparqlParser::new()
            .parse_query(query)
            .expect("parse");
        let mut ctx = EvalCtx::new(ds).with_remote(source);
        let outcome = evaluate_query(&parsed, &mut ctx)?;
        Ok(match outcome {
            crate::eval::Outcome::Solutions(seq) => {
                let (variables, rows) = materialize_solutions(&seq, &ctx);
                let aux = ctx.constructed_dataset(&rows);
                SparqlResult::Solutions {
                    variables,
                    rows,
                    aux,
                }
            }
            crate::eval::Outcome::Boolean(b) => SparqlResult::Boolean(b),
            crate::eval::Outcome::Graph(g) => SparqlResult::Graph(g),
        })
    }

    fn row_strings(result: &SparqlResult) -> Vec<Vec<String>> {
        match result {
            SparqlResult::Solutions { rows, .. } => {
                let mut out: Vec<Vec<String>> = rows
                    .iter()
                    .map(|r| {
                        r.iter()
                            .map(|c| match c {
                                None => "UNBOUND".to_owned(),
                                Some(TermValue::Iri(i)) => format!("<{i}>"),
                                Some(TermValue::Literal { lexical_form, .. }) => {
                                    lexical_form.clone()
                                }
                                Some(_) => "other".to_owned(),
                            })
                            .collect()
                    })
                    .collect();
                out.sort();
                out
            }
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    #[test]
    fn service_joins_remote_bindings_on_shared_var() {
        let source = LocalRemoteQuerySource::new().with_endpoint("http://ep", endpoint());
        let result = run_with_source(
            &local(),
            &source,
            "SELECT ?s ?o ?n WHERE { ?s <http://ex/knows> ?o \
             SERVICE <http://ep> { ?o <http://ex/name> ?n } }",
        )
        .expect("query");
        // Only ?o = :x has a remote name → exactly one joined row.
        assert_eq!(
            row_strings(&result),
            vec![vec![
                "<http://ex/a>".to_owned(),
                "<http://ex/x>".to_owned(),
                "X".to_owned()
            ]]
        );
    }

    #[test]
    fn service_silent_unknown_endpoint_is_a_noop() {
        // SILENT against an unconfigured endpoint → identity → all left rows kept.
        let source = LocalRemoteQuerySource::new(); // no endpoints registered
        let result = run_with_source(
            &local(),
            &source,
            "SELECT ?s ?o WHERE { ?s <http://ex/knows> ?o \
             SERVICE SILENT <http://missing> { ?o <http://ex/name> ?n } }",
        )
        .expect("query");
        assert_eq!(
            row_strings(&result),
            vec![
                vec!["<http://ex/a>".to_owned(), "<http://ex/x>".to_owned()],
                vec!["<http://ex/a>".to_owned(), "<http://ex/y>".to_owned()],
            ]
        );
    }

    #[test]
    fn non_silent_service_without_source_hard_fails() {
        // The engine's default EvalCtx has no remote source: a non-silent SERVICE
        // must raise EvalError::Remote rather than silently contributing nothing.
        let engine = NativeSparqlEngine::new();
        let err = engine
            .query(
                &local(),
                SparqlRequest {
                    query: "SELECT ?o WHERE { ?s <http://ex/knows> ?o \
                            SERVICE <http://ep> { ?o <http://ex/name> ?n } }",
                    base_iri: None,
                    substitutions: &[],
                },
            )
            .unwrap_err();
        assert_eq!(err.code, "native-sparql-query-eval");
        assert!(err.message.contains("SERVICE"), "got: {}", err.message);
    }

    #[test]
    fn non_silent_unknown_endpoint_with_source_hard_fails() {
        let source = LocalRemoteQuerySource::new(); // endpoint not registered
        let err = run_with_source(
            &local(),
            &source,
            "SELECT ?o WHERE { ?s <http://ex/knows> ?o \
             SERVICE <http://missing> { ?o <http://ex/name> ?n } }",
        )
        .unwrap_err();
        assert!(matches!(err, EvalError::Remote(_)), "got {err:?}");
    }
}
