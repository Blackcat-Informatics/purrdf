// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SPARQL `SERVICE` federation: the `eval_service` handler and the
//! [`RemoteQuerySource`] seam.
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
//!
//! # `SILENT` is about the endpoint, never about this engine's budget
//!
//! `SILENT` says "a federated endpoint I do not control may be unreachable, and I would
//! rather have the rest of my answer than an error". It says nothing about the caller's
//! own governors, so a governor trip reached through a `SERVICE` clause is
//! **non-silenceable**: it propagates as a truncation whether or not `SILENT` is present
//! (see [`RemoteError::Governed`]). Swallowing a trip to the join identity would leave the
//! surrounding join a no-op and the final result indistinguishable from a complete one —
//! an answer that looks complete and is wrong, which is worse than either an error or an
//! honest partial.
//!
//! # A federated call is governed at both ends
//!
//! The seam takes the execution's [`StopSignal`], and it is polled **before** dispatch, so
//! an expired deadline prevents the request rather than observing it after the network
//! call has already gone out. The signal is also handed to the source itself, so a host
//! transport can consult it while this evaluator is blocked inside the call — the one
//! window in which nothing else can. Both the request and every ingested row are charged,
//! and the response is measured against the intermediate-cell ceiling before a single row
//! of it is interned, so a response arriving from outside the dataset cannot walk past
//! ceilings that bound everything computed inside it.

use std::collections::HashMap;
use std::sync::Arc;

use purrdf_core::{DatasetView, RdfDataset, TermValue, TrippedGovernor, ViewTermId};
use purrdf_sparql_algebra::{GraphPattern, NamedNodePattern, Variable};

use crate::error::EvalError;
use crate::eval::{EvalCtx, EvaluatedOutcome, Outcome, materialize_solutions};
use crate::governor::lift::{Evaluated, Truncation};
use crate::governor::{GovernorState, QueryGovernors, StopSignal};
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
/// swallowed is decided by `eval_service` from the `SILENT` flag, not here — with the one
/// exception of [`Self::Governed`], which `SILENT` cannot swallow.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RemoteError {
    /// The endpoint was unreachable / the request failed at the transport layer.
    Transport(String),
    /// The endpoint responded, but the body could not be decoded into bindings.
    Decode(String),
    /// Federation is disabled for this source.
    Disabled,
    /// **This engine's own** governor stopped the exchange: the caller's stop signal
    /// fired, or a ceiling was crossed inside the forwarded evaluation.
    ///
    /// Not an endpoint failure and therefore not silenceable — see the module
    /// documentation. A source returns this only for a governor it was handed (the stop
    /// signal threaded into [`RemoteQuerySource::query`]); an endpoint's own overload,
    /// throttling, or timeout is [`Self::Transport`], because that is the caller's
    /// endpoint misbehaving rather than the caller's budget running out.
    Governed(TrippedGovernor),
    /// The host exchange completed after this execution's stop signal fired, so its
    /// response was discarded before decode or ingest.
    ///
    /// This is distinct from [`Self::Governed`] because completing and discarding a
    /// response removes the positional-prefix/resumption claim even though the lower
    /// answer bound remains sound. `SERVICE SILENT` may not swallow either variant.
    GovernedAfterCompletion(TrippedGovernor),
}

impl core::fmt::Display for RemoteError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Transport(m) => write!(f, "transport: {m}"),
            Self::Decode(m) => write!(f, "decode: {m}"),
            Self::Disabled => write!(f, "federation disabled"),
            Self::Governed(governor) => write!(f, "governed: {governor}"),
            Self::GovernedAfterCompletion(governor) => {
                write!(f, "governed after completed exchange: {governor}")
            }
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
    /// `stop` is the executing query's [`StopSignal`], or `None` when the caller set no
    /// deadline and no cancellation. An implementation **must** poll it before it starts
    /// the exchange, and should poll it again anywhere it would otherwise wait: this is
    /// the only governor that can act while the evaluator is blocked inside this call, and
    /// a federated call is where an unbounded wait is most likely. Report a fired signal as
    /// [`RemoteError::Governed`] — never as [`RemoteError::Transport`], which `SILENT` is
    /// entitled to swallow.
    ///
    /// # What a transport that cannot abandon an exchange still gets, and what it loses
    ///
    /// Polling mid-exchange is a capability, not an obligation: plenty of HTTP clients
    /// cannot cancel a request they are already inside, and nothing here forces a host to
    /// write one that can. So the contract is stated for both cases, and the difference
    /// between them is pinned by the frozen governor corpus
    /// (`vectors/sparql-governors/`, the `service-*-transport-*` cases) rather than left
    /// to a reader's judgement.
    ///
    /// A transport that **ignores** `stop` still degrades only to **per-request
    /// granularity**, never to unboundedness. The signal is polled by the evaluator
    /// immediately before dispatch and the request is charged before it is issued, so a
    /// signal that was already firing prevents the call, and one that fires during the
    /// call is observed the moment control returns. The corpus pins that a cancellation
    /// raised while a deaf transport is mid-exchange reaches the same outcome
    /// (`stopped cancelled`) and the same exchange count (one) as it does through an
    /// honouring transport.
    ///
    /// What the deaf transport loses is the **positional-prefix claim** on the
    /// certificate. The distinction is not about how much was computed — both cases carry
    /// the same rows — but about what a caller may do next. An honouring transport
    /// *abandons* the exchange, so the answer that would have followed was never
    /// established and the rows in hand are still the true output's first rows, in order:
    /// [`PartialSparqlResult::is_positional_prefix`](crate::PartialSparqlResult::is_positional_prefix)
    /// stays `true` and re-running under a larger budget resumes from them. A deaf
    /// transport *completes* the exchange and the evaluator then discards its response,
    /// so rows that would have been established are absent from the middle of the answer
    /// rather than from its end; the positional claim is withdrawn (`false`) and with it
    /// the resumption licence. The multiset bound is unaffected — the certificate is
    /// [`PartialAnswers::Certain`](crate::PartialAnswers::Certain) either way — because
    /// every row handed back was genuinely established.
    ///
    /// Both halves are worth stating plainly: honouring `stop` is not required for
    /// soundness, and it does buy the caller something concrete.
    ///
    /// # Errors
    ///
    /// Returns [`RemoteError`] on transport or decode failure, which `eval_service` may
    /// swallow under `SILENT`, or [`RemoteError::Governed`], which it never swallows.
    fn query(
        &self,
        endpoint: &str,
        query_text: &str,
        stop: Option<&Arc<dyn StopSignal>>,
    ) -> Result<ResolvedBindings, RemoteError>;
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
///
/// # Under a truncation
///
/// The inner pattern is serialized and sent to the endpoint rather than evaluated here,
/// so there is no child result to lift: this node is a leaf as far as the partial-lift
/// channel is concerned. Its result is a complete bag, a typed failure, or a truncation
/// this node itself originates — from the stop signal, from the request charge, from the
/// cell ceiling, or from a governor the source reports through [`RemoteError::Governed`].
/// None of those four is silenceable.
pub(crate) fn eval_service<D: DatasetView + Sync>(
    node: &GraphPattern,
    name: &NamedNodePattern,
    inner: &GraphPattern,
    silent: bool,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    let _ = node;
    // Resolve the endpoint IRI. A variable endpoint needs per-row (lateral)
    // resolution, which the engine defers — so it is a hard error unless SILENT.
    let endpoint = match name {
        NamedNodePattern::NamedNode(n) => n.as_str().to_owned(),
        NamedNodePattern::Variable(_) => {
            return silent_or_err(silent, || {
                "SERVICE with a variable endpoint is not supported (needs lateral evaluation)"
                    .to_owned()
            })
            .map(Evaluated::Complete);
        }
    };

    // `Option<&dyn _>` is `Copy`, so this does NOT borrow `ctx` — leaving `&mut
    // ctx` free for interning the result below.
    let Some(source) = ctx.remote else {
        return silent_or_err(silent, || {
            format!("no remote query source configured for SERVICE <{endpoint}>")
        })
        .map(Evaluated::Complete);
    };

    // Poll the stop signal immediately before dispatch. The node-entry poll happens before
    // this node's work begins; a deadline that expires in between must still PREVENT the
    // request, because a signal observed only after the call returned is not a governor —
    // the call is the wait it exists to bound. Highest precedence, so it is tested ahead
    // of the charge below.
    if let Some(tripped) = ctx.stop_check() {
        return Ok(Evaluated::Truncated(Truncation::origin(
            SolutionSeq::empty(Arc::new(VarSchema::new())),
            tripped,
        )));
    }

    // The `remote-request-issued` charge point, plus the request against the remote
    // request ceiling — charged **before** the call, so an exhausted budget prevents the
    // request rather than merely observing it afterwards. A budget trip is deliberately
    // NOT silenceable here: `SILENT` is a statement about the endpoint, not about this
    // engine's budget, and swallowing a trip to the join identity would return a result
    // that looks complete and is wrong.
    if let Err(tripped) = ctx
        .charge(crate::governor::ChargePoint::RemoteRequestIssued)
        .and_then(|()| ctx.charge_amount(purrdf_core::ResourceDimension::RemoteRequests, 1))
    {
        // The empty bag, never the join identity: the identity row is what makes a
        // surrounding join a no-op, so returning it here would claim the remote endpoint
        // had been consulted and had imposed nothing. An empty bag claims only that no
        // remote row was established, which is exactly true and is a sound lower bound.
        return Ok(Evaluated::Truncated(Truncation::origin(
            SolutionSeq::empty(Arc::new(VarSchema::new())),
            tripped,
        )));
    }

    let query_text = purrdf_sparql_algebra::pattern_to_select_query(inner);
    // The signal travels WITH the call: while the evaluator is blocked inside it, nothing
    // else is in a position to poll.
    let stop = ctx.stop_signal().map(Arc::clone);
    let response = source.query(&endpoint, &query_text, stop.as_ref());
    // Always inspect the signal immediately after control returns. A source is permitted
    // to be unable to abandon an in-flight request; without this checkpoint a terminal
    // SERVICE could launder a cancellation into `Complete` because no later operator
    // would ever poll it.
    let post_return_trip = ctx.stop_check();
    let resolved = match response {
        Ok(resolved) => {
            if let Some(tripped) = post_return_trip {
                let schema = Arc::new(VarSchema::from_vars(resolved.variables));
                return Ok(Evaluated::Truncated(Truncation::bag_only_origin(
                    SolutionSeq::empty(schema),
                    tripped,
                )));
            }
            resolved
        }
        // A governor the source was handed. Not the endpoint's failure and so not
        // silenceable — latched into the evidence so the receipt names the same governor
        // the result does.
        Err(RemoteError::Governed(governor)) => {
            return Ok(Evaluated::Truncated(Truncation::origin(
                SolutionSeq::empty(Arc::new(VarSchema::new())),
                ctx.record_trip(governor),
            )));
        }
        Err(RemoteError::GovernedAfterCompletion(governor)) => {
            return Ok(Evaluated::Truncated(Truncation::bag_only_origin(
                SolutionSeq::empty(Arc::new(VarSchema::new())),
                ctx.record_trip(governor),
            )));
        }
        Err(e) => {
            // A real endpoint failure outranks a simultaneous stop. Under SILENT the
            // endpoint failure is deliberately erased, so the stop becomes the surviving
            // fact and must remain non-silenceable.
            if silent && let Some(tripped) = post_return_trip {
                return Ok(Evaluated::Truncated(Truncation::bag_only_origin(
                    SolutionSeq::empty(Arc::new(VarSchema::new())),
                    tripped,
                )));
            }
            return silent_or_err(silent, || format!("SERVICE <{endpoint}>: {e}"))
                .map(Evaluated::Complete);
        }
    };

    let (seq, tripped) = ingest(resolved, ctx);
    Ok(match tripped {
        None => Evaluated::Complete(seq),
        Some(tripped) => Evaluated::Truncated(Truncation::origin(seq, tripped)),
    })
}

/// On `SILENT`, return the join identity (one empty row, a no-op for the
/// surrounding join); otherwise raise [`EvalError::Remote`] with `msg()`.
fn silent_or_err<I: ViewTermId>(
    silent: bool,
    msg: impl FnOnce() -> String,
) -> Result<SolutionSeq<I>, EvalError> {
    if silent {
        Ok(identity_seq())
    } else {
        Err(EvalError::remote(msg()))
    }
}

/// The join identity: a single empty-binding row. `Join(left, identity) == left`,
/// so a swallowed `SERVICE SILENT` leaves the surrounding query unchanged.
fn identity_seq<I: ViewTermId>() -> SolutionSeq<I> {
    SolutionSeq {
        schema: Arc::new(VarSchema::new()),
        rows: vec![smallvec::smallvec![]],
    }
}

/// Intern a remote result's owned [`TermValue`]s into the per-query scratch space,
/// yielding a [`SolutionSeq`] over the result schema. (Mirrors `modifier::eval_values`
/// but carries `TermValue` directly, so remote blank nodes survive — `GroundTerm`
/// has no blank-node variant.)
/// Returns the interned bag together with the governor that stopped the ingest, if one
/// did: the `remote-row-ingested` charge point is charged per row **as it is interned**,
/// so an unbounded remote response cannot walk past the caller's ceilings by arriving
/// from outside the dataset. Charging in row order makes the ingested prefix a positional
/// prefix of the endpoint's answer, which is what lets the caller certify it.
///
/// # The cell ceiling is tested before the first row is interned
///
/// A remote bag is an intermediate bag like any other, so it is measured against
/// [`ResourceDimension::IntermediateCells`](purrdf_core::ResourceDimension::IntermediateCells)
/// — and measured **up front**, from the response's own row count and width, rather than
/// after interning. Every other operator's bag is bounded by the data the ceiling was
/// sized against; this one is bounded by whatever a remote endpoint chose to send, which
/// is the only bag in the evaluator an attacker (or a mistake) can size directly. A
/// ceiling that reports the breach after the allocation has already been made is not a
/// memory ceiling, and on wasm the allocation trap it is meant to prevent kills the module
/// instance before any typed outcome can be returned at all.
fn ingest<D: DatasetView + Sync>(
    resolved: ResolvedBindings,
    ctx: &mut EvalCtx<'_, D>,
) -> (SolutionSeq<D::Id>, Option<TrippedGovernor>) {
    let schema = Arc::new(VarSchema::from_vars(resolved.variables));
    let width = schema.len();
    if let Err(governor) = ctx.observe_cells(resolved.rows.len(), width) {
        return (SolutionSeq::empty(schema), Some(governor));
    }
    let mut rows = Vec::with_capacity(resolved.rows.len());
    let mut tripped = None;
    for binding in resolved.rows {
        if let Err(governor) = ctx.charge(crate::governor::ChargePoint::RemoteRowIngested) {
            tripped = Some(governor);
            break;
        }
        let mut row = smallvec::smallvec![None; width];
        for (i, cell) in binding.into_iter().enumerate().take(width) {
            if let Some(value) = cell {
                row[i] = Some(ctx.scratch.intern(ctx.dataset, value));
            }
        }
        rows.push(row);
    }
    (SolutionSeq { schema, rows }, tripped)
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
    /// # The forwarded evaluation is governed by the caller's signal
    ///
    /// The forwarded query is a whole evaluation of its own, and an in-memory endpoint is
    /// not a bounded amount of work — a cyclic property path or a cross product costs the
    /// same here as it does anywhere else. So the caller's [`StopSignal`] is installed on
    /// the forwarded context, which polls it at every operator boundary, and a signal that
    /// fires part-way through is reported as [`RemoteError::Governed`] rather than as a
    /// decode failure. Reporting it as a failure would make it silenceable, which is
    /// exactly the laundering `SILENT` must not perform.
    ///
    /// The forwarded evaluation carries no *ceilings* — only the signal — because fuel
    /// spent here is already charged at the calling seam, per request and per ingested
    /// row: charging it twice would make one query's budget depend on how a federation
    /// happened to be split up.
    fn query(
        &self,
        endpoint: &str,
        query_text: &str,
        stop: Option<&Arc<dyn StopSignal>>,
    ) -> Result<ResolvedBindings, RemoteError> {
        if let Some(cause) = stop.and_then(|signal| signal.poll()) {
            return Err(RemoteError::Governed(TrippedGovernor::Stopped { cause }));
        }
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
        let mut ctx = EvalCtx::new(&**dataset).with_remote(self);
        if let Some(signal) = stop {
            let governors = QueryGovernors::UNBOUNDED.with_stop_signal(Arc::clone(signal));
            ctx = ctx.with_governors(Arc::new(GovernorState::new(&governors)));
        }
        match crate::eval::evaluate_query_evaluated(&parsed, &mut ctx)
            .map_err(|e| RemoteError::Decode(e.to_string()))?
        {
            EvaluatedOutcome::Complete(Outcome::Solutions(seq)) => {
                let (variables, rows) = materialize_solutions(&seq, &ctx);
                Ok(ResolvedBindings {
                    variables: variables.into_iter().map(Variable::new).collect(),
                    rows,
                })
            }
            EvaluatedOutcome::Complete(_) => Err(RemoteError::Decode(
                "SERVICE expects a SELECT query".to_owned(),
            )),
            EvaluatedOutcome::Truncated { certificate, .. } => {
                Err(RemoteError::Governed(certificate.tripped()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NativeSparqlEngine;
    use crate::governor::WallDeadline;
    use crate::remote_http::{HttpRemoteQuerySource, HttpRequest};
    use purrdf_core::{
        RdfDatasetBuilder, RdfLiteral, ResourceDimension, SparqlEngine, SparqlRequest,
        SparqlResult, StopCause,
    };
    use purrdf_sparql_algebra::{GroundTerm, NamedNode, TermPattern, TriplePattern};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

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
        source: &(dyn RemoteQuerySource + Sync),
        query: &str,
    ) -> Result<SparqlResult, EvalError> {
        use crate::eval::evaluate_query;
        let parsed = purrdf_sparql_algebra::SparqlParser::new()
            .parse_query(query)
            .expect("parse");
        let mut ctx = EvalCtx::new(ds).with_remote(source);
        let outcome = evaluate_query(&parsed, &mut ctx)?;
        Ok(match outcome {
            Outcome::Solutions(seq) => {
                let (variables, rows) = materialize_solutions(&seq, &ctx);
                let aux = ctx.constructed_dataset(&rows);
                SparqlResult::Solutions {
                    variables,
                    rows,
                    aux,
                }
            }
            Outcome::Boolean(b) => SparqlResult::Boolean(b),
            Outcome::Graph(g) => SparqlResult::Graph(g),
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

    // ── LATERAL variable-endpoint SERVICE forwarding counts ───────────────────

    /// `:row{i} :endpoint <http://ex/ep{i}>` for `i` in `0..n`.
    fn multi_endpoint_local(n: usize) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let endpoint = b.intern_iri("http://ex/endpoint");
        for i in 0..n {
            let row = b.intern_iri(&format!("http://ex/row{i}"));
            let ep = b.intern_iri(&format!("http://ex/ep{i}"));
            b.push_quad(row, endpoint, ep, None);
        }
        b.freeze().expect("freeze")
    }

    /// Endpoint `i` contains `:s :name "ep{i}"`.
    fn multi_endpoint(i: usize) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let name = b.intern_iri("http://ex/name");
        let s = b.intern_iri("http://ex/s");
        let lit = b.intern_literal(RdfLiteral::simple(format!("ep{i}")));
        b.push_quad(s, name, lit, None);
        b.freeze().expect("freeze")
    }

    /// Register `n` distinct in-memory endpoints.
    fn multi_source(n: usize) -> LocalRemoteQuerySource {
        let mut src = LocalRemoteQuerySource::new();
        for i in 0..n {
            src = src.with_endpoint(format!("http://ex/ep{i}"), multi_endpoint(i));
        }
        src
    }

    /// A `RemoteQuerySource` wrapper that counts `query()` calls.
    struct CountingSource<'a> {
        inner: &'a (dyn RemoteQuerySource + Sync),
        count: AtomicUsize,
    }

    impl<'a> CountingSource<'a> {
        fn new(inner: &'a (dyn RemoteQuerySource + Sync)) -> Self {
            Self {
                inner,
                count: AtomicUsize::new(0),
            }
        }
        fn count(&self) -> usize {
            self.count.load(Ordering::Relaxed)
        }
    }

    impl RemoteQuerySource for CountingSource<'_> {
        fn query(
            &self,
            endpoint: &str,
            query_text: &str,
            stop: Option<&Arc<dyn StopSignal>>,
        ) -> Result<ResolvedBindings, RemoteError> {
            self.count.fetch_add(1, Ordering::Relaxed);
            self.inner.query(endpoint, query_text, stop)
        }
    }

    #[test]
    fn variable_service_forwards_once_per_distinct_endpoint_binding() {
        // Four left rows, each binding `?g` to a distinct endpoint → the LATERAL
        // path must forward once for every distinct endpoint (four forwards).
        let n = 4;
        let ds = multi_endpoint_local(n);
        let source = multi_source(n);
        let counting = CountingSource::new(&source);
        let result = run_with_source(
            &ds,
            &counting,
            "SELECT * WHERE { ?x <http://ex/endpoint> ?g \
             SERVICE ?g { ?s <http://ex/name> ?name } }",
        )
        .expect("query");
        assert_eq!(
            counting.count(),
            n,
            "variable SERVICE should forward once per distinct endpoint"
        );
        assert_eq!(row_strings(&result).len(), n);
    }

    // ── Federation under governors ────────────────────────────────────────────

    /// The namespace every governed-federation fixture uses.
    const EX: &str = "http://example.org/";

    /// A source that answers any endpoint with `rows` fixed single-column rows, counting
    /// every call it receives.
    ///
    /// Fixed rows rather than a forwarded evaluation, so a test can state the response
    /// size — and therefore the exact charge — directly. The call counter is incremented
    /// **before** the signal is polled, so a test asserting zero calls is really asserting
    /// that the request was never issued, not that the source declined it.
    #[derive(Debug)]
    struct FixtureSource {
        /// How many rows every response carries.
        rows: usize,
        /// How many times `query` has been called.
        calls: AtomicUsize,
    }

    impl FixtureSource {
        /// A source answering with `rows` rows.
        fn new(rows: usize) -> Self {
            Self {
                rows,
                calls: AtomicUsize::new(0),
            }
        }

        /// How many requests this source has been given.
        fn calls(&self) -> usize {
            self.calls.load(Ordering::Relaxed)
        }
    }

    impl RemoteQuerySource for FixtureSource {
        fn query(
            &self,
            _endpoint: &str,
            _query_text: &str,
            stop: Option<&Arc<dyn StopSignal>>,
        ) -> Result<ResolvedBindings, RemoteError> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            if let Some(cause) = stop.and_then(|signal| signal.poll()) {
                return Err(RemoteError::Governed(TrippedGovernor::Stopped { cause }));
            }
            Ok(ResolvedBindings {
                variables: vec![Variable::new("n")],
                rows: (0..self.rows)
                    .map(|i| vec![Some(TermValue::Iri(format!("{EX}r{i}")))])
                    .collect(),
            })
        }
    }

    /// A source that reports a governor of the **executing engine** rather than an
    /// endpoint failure — what a real source returns when the stop signal it was handed
    /// fires mid-exchange.
    #[derive(Debug)]
    struct StoppedSource(StopCause);

    impl RemoteQuerySource for StoppedSource {
        fn query(
            &self,
            _endpoint: &str,
            _query_text: &str,
            _stop: Option<&Arc<dyn StopSignal>>,
        ) -> Result<ResolvedBindings, RemoteError> {
            Err(RemoteError::Governed(TrippedGovernor::Stopped {
                cause: self.0,
            }))
        }
    }

    /// `SERVICE [SILENT] <http://example.org/sparql> { ?s ?p ?o }`.
    fn service_pattern(silent: bool) -> GraphPattern {
        GraphPattern::Service {
            name: NamedNodePattern::NamedNode(NamedNode::new_unchecked(format!("{EX}sparql"))),
            inner: Box::new(GraphPattern::Bgp {
                patterns: vec![TriplePattern {
                    subject: TermPattern::Variable(Variable::new("s")),
                    predicate: NamedNodePattern::Variable(Variable::new("p")),
                    object: TermPattern::Variable(Variable::new("o")),
                }],
            }),
            silent,
        }
    }

    /// The same query with the `SERVICE` clause gone and its `rows` rows supplied inline:
    /// one node entry and the same committed rows over the same one-column schema, and
    /// nothing else. Every charge the two patterns share therefore cancels, which is what
    /// makes the difference between them exactly the federation's own charges.
    fn inline_pattern(rows: usize) -> GraphPattern {
        GraphPattern::Values {
            variables: vec![Variable::new("n")],
            bindings: (0..rows)
                .map(|i| {
                    vec![Some(GroundTerm::NamedNode(NamedNode::new_unchecked(
                        format!("{EX}r{i}"),
                    )))]
                })
                .collect(),
        }
    }

    /// An empty dataset: every fixture here answers from the remote source, so the local
    /// side contributes no rows and no charges of its own.
    fn empty_dataset() -> Arc<RdfDataset> {
        RdfDatasetBuilder::new()
            .freeze()
            .expect("an empty dataset is positionally valid")
    }

    /// What one governed federated evaluation observed.
    #[derive(Debug)]
    struct GovernedRun {
        /// Whether the result came back truncated rather than complete.
        truncated: bool,
        /// The rows it kept.
        rows: usize,
        /// The columns those rows carry.
        columns: usize,
        /// The governor that stopped it, if one did.
        tripped: Option<TrippedGovernor>,
        /// Fuel charged.
        fuel: u64,
    }

    /// Evaluate `pattern` over an empty dataset under `governors`, with `source` injected.
    fn run_governed(
        pattern: &GraphPattern,
        source: &(dyn RemoteQuerySource + Sync),
        governors: &QueryGovernors,
    ) -> GovernedRun {
        let dataset = empty_dataset();
        let state = Arc::new(GovernorState::new(governors));
        let mut ctx = EvalCtx::new(&*dataset)
            .with_remote(source)
            .with_governors(Arc::clone(&state));
        let evaluated =
            crate::eval::eval_evaluated(pattern, &mut ctx).expect("evaluation must not fail");
        let evidence = state.evidence();
        GovernedRun {
            truncated: evaluated.is_truncated(),
            rows: evaluated.rows().len(),
            columns: evaluated.rows().schema.len(),
            tripped: evidence.tripped(),
            fuel: evidence.consumed_in(ResourceDimension::Fuel),
        }
    }

    #[test]
    fn service_silent_cannot_swallow_a_budget_trip() {
        // A remote-request ceiling of zero trips at the request charge, before dispatch —
        // and does so under SILENT.
        let source = FixtureSource::new(3);
        let run = run_governed(
            &service_pattern(true),
            &source,
            &QueryGovernors::UNBOUNDED.with_max_remote_requests(0),
        );

        assert_eq!(source.calls(), 0, "the refused request must not be issued");
        assert!(
            run.truncated,
            "SILENT is a statement about the endpoint, never about this engine's budget"
        );
        assert_eq!(
            run.tripped,
            Some(TrippedGovernor::Budget {
                dimension: ResourceDimension::RemoteRequests,
                limit: 0,
                consumed: 1,
            })
        );
        assert_eq!(
            run.rows, 0,
            "the empty bag, never the join identity: an identity row makes the surrounding \
             join a no-op, so the final result would look complete and be wrong"
        );

        // The identical SILENT clause with the ceiling lifted really does complete, so the
        // assertions above are about the governor and not about the query.
        let unbounded = run_governed(
            &service_pattern(true),
            &FixtureSource::new(3),
            &QueryGovernors::UNBOUNDED,
        );
        assert!(!unbounded.truncated);
        assert_eq!(unbounded.rows, 3);
    }

    #[test]
    fn a_governor_the_source_reports_is_not_silenceable_either() {
        // A source handed the stop signal can observe it firing DURING the exchange, when
        // nothing else can. What it reports back is a governor, not an endpoint failure,
        // so SILENT does not swallow it — and the evidence names the same governor the
        // result does.
        let run = run_governed(
            &service_pattern(true),
            &StoppedSource(StopCause::Cancelled),
            &QueryGovernors::UNBOUNDED,
        );
        assert!(run.truncated);
        assert_eq!(
            run.tripped,
            Some(TrippedGovernor::Stopped {
                cause: StopCause::Cancelled
            })
        );
        assert_eq!(run.rows, 0);
    }

    #[test]
    fn an_expired_deadline_prevents_the_remote_request_from_being_issued() {
        let source = FixtureSource::new(3);
        let deadline: Arc<dyn StopSignal> = Arc::new(WallDeadline::after(Duration::ZERO));
        let run = run_governed(
            &service_pattern(false),
            &source,
            &QueryGovernors::UNBOUNDED.with_stop_signal(Arc::clone(&deadline)),
        );

        assert_eq!(
            source.calls(),
            0,
            "zero invocations is the only assertion that pins ordering: an implementation \
             that polled AFTER the network call returned would also report a trip"
        );
        assert!(run.truncated);
        assert_eq!(
            run.tripped,
            Some(TrippedGovernor::Stopped {
                cause: StopCause::Deadline
            })
        );
        assert_eq!(run.rows, 0);

        // …and the seam refuses on its own account, not merely the evaluator around it: a
        // source handed an already-latched signal never reaches its transport.
        let posts = AtomicUsize::new(0);
        let source = HttpRemoteQuerySource::new(|request: HttpRequest<'_>| {
            assert!(
                request.stop.is_some(),
                "the signal must travel with the call"
            );
            posts.fetch_add(1, Ordering::Relaxed);
            Ok(Vec::new())
        });
        let err = source
            .query(
                &format!("{EX}sparql"),
                "SELECT * WHERE { ?s ?p ?o }",
                Some(&deadline),
            )
            .expect_err("an expired deadline refuses the request");
        assert_eq!(
            err,
            RemoteError::Governed(TrippedGovernor::Stopped {
                cause: StopCause::Deadline
            })
        );
        assert_eq!(
            posts.load(Ordering::Relaxed),
            0,
            "the transport must never be reached"
        );
    }

    #[test]
    fn remote_requests_and_ingested_rows_are_charged() {
        const N: usize = 5;

        // METERED, not UNBOUNDED: an ungoverned execution charges nothing by design, so
        // UNBOUNDED would report zero for both runs and the comparison would be vacuous.
        let source = FixtureSource::new(N);
        let federated = run_governed(&service_pattern(false), &source, &QueryGovernors::METERED);
        assert_eq!(federated.tripped, None, "the metering run must complete");
        assert_eq!(federated.rows, N);
        assert_eq!(source.calls(), 1);

        let inline = run_governed(
            &inline_pattern(N),
            &FixtureSource::new(0),
            &QueryGovernors::METERED,
        );
        assert_eq!(inline.tripped, None);
        assert_eq!(inline.rows, N);

        // Schedule v1: one `remote-request-issued`, then one `remote-row-ingested` per row
        // of the response. Everything else the two runs do is identical and cancels.
        assert_eq!(
            federated.fuel,
            inline.fuel + 1 + N as u64,
            "the federation costs exactly one request plus one unit per ingested row"
        );

        // A cell ceiling below the response's size trips the cardinality governor — and
        // trips it BEFORE a row is interned, so the ceiling bounds the allocation instead
        // of reporting on one already made.
        let source = FixtureSource::new(N);
        let cells = N as u64 - 1; // one column, so cells == rows
        let capped = run_governed(
            &service_pattern(false),
            &source,
            &QueryGovernors::METERED.with_max_intermediate_cells(cells),
        );
        assert!(capped.truncated);
        assert_eq!(
            capped.tripped,
            Some(TrippedGovernor::Budget {
                dimension: ResourceDimension::IntermediateCells,
                limit: cells,
                consumed: N as u64,
            }),
            "a remote bag is an intermediate bag: arriving from outside the dataset is not \
             a way past the memory ceiling"
        );
        assert_eq!(
            capped.rows, 0,
            "not one row of an oversized response is interned"
        );
    }

    #[test]
    fn a_transport_error_under_silent_still_yields_the_join_identity() {
        // The regression guard: `SILENT` is about the endpoint, and an unreachable
        // endpoint IS the endpoint. Governed or not, the behaviour is exactly what it was
        // before governors existed — one empty row, so the surrounding join is a no-op.
        let source = LocalRemoteQuerySource::new(); // nothing registered
        for governors in [QueryGovernors::UNBOUNDED, QueryGovernors::METERED] {
            let run = run_governed(&service_pattern(true), &source, &governors);
            assert!(
                !run.truncated,
                "a transport error under SILENT is a swallowed failure, not a truncation"
            );
            assert_eq!(run.tripped, None);
            assert_eq!(run.rows, 1, "the join identity is one row");
            assert_eq!(run.columns, 0, "…and it binds nothing");
        }
    }

    #[test]
    fn fixed_service_forwards_exactly_once_regardless_of_left_rows() {
        // Four left rows, but the endpoint is fixed → one forward total.
        let n = 4;
        let ds = multi_endpoint_local(n);
        let source = multi_source(n);
        let counting = CountingSource::new(&source);
        let result = run_with_source(
            &ds,
            &counting,
            "SELECT * WHERE { ?x <http://ex/endpoint> ?g \
             SERVICE <http://ex/ep0> { ?s <http://ex/name> ?name } }",
        )
        .expect("query");
        assert_eq!(
            counting.count(),
            1,
            "fixed SERVICE should forward exactly once"
        );
        assert_eq!(row_strings(&result).len(), n);
    }
}
