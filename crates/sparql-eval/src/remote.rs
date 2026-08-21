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
use purrdf_sparql_algebra::{GraphPattern, GroundTerm, NamedNodePattern, Variable};

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
    /// Exact cell count of the first response prefix the supplied ceiling refused, when
    /// the source deliberately stopped before materializing it. `rows` is the ordered
    /// prefix that fit.
    pub cell_limit_exceeded_at: Option<u64>,
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
    /// stays `true`, preserving the relation needed to resume under a deterministic
    /// ceiling. A deaf transport *completes* the exchange and the evaluator then discards its response,
    /// so rows that would have been established are absent from the middle of the answer
    /// rather than from its end; the positional claim is withdrawn (`false`) and with it
    /// the resumption licence. The multiset bound is unaffected — the certificate is
    /// [`PartialAnswers::Certain`](crate::PartialAnswers::Certain) either way — because
    /// every row handed back was genuinely established.
    ///
    /// Both halves are worth stating plainly: honouring `stop` is not required for
    /// soundness, and it does buy the caller something concrete.
    ///
    /// `max_intermediate_cells` is the executing query's inclusive peak-cell ceiling. A
    /// source that decodes or evaluates bindings itself must stop before materializing a
    /// response prefix wider than that bound and report the exact first refused size in
    /// [`ResolvedBindings::cell_limit_exceeded_at`]. The evaluator repeats the check during
    /// ingest, so a source that cannot implement bounded decode remains sound, but it gives
    /// up the allocation protection at its own side of the seam.
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
        max_intermediate_cells: Option<u64>,
    ) -> Result<ResolvedBindings, RemoteError>;
}

/// Whether `pattern` reaches a WRITTEN `LATERAL` keyword anywhere in the FULL
/// forwardable body — including inside a nested `SERVICE` (fixed-IRI or
/// variable-endpoint) and inside an expression-embedded `EXISTS` — shared by
/// [`eval_service`]'s forward guard. Follows the same soundness-visitor idiom
/// as `crate::property_fn_eval::pattern_reaches_property_function`, and the
/// generic fallback below already recurses into `GraphPattern::Service`'s
/// `inner` via `visit_pattern_parts`'s `Service` arm regardless of whether the
/// endpoint is fixed or a variable — a written `LATERAL` nested inside
/// `SERVICE ?g { … }` (which is itself nested arbitrarily deep) is therefore
/// found no differently than one nested inside `SERVICE <fixed> { … }` or any
/// other child position.
///
/// A `Lateral` whose right operand is a property-function call or a
/// variable-endpoint `SERVICE` does NOT itself count as a written `LATERAL`:
/// `purrdf_sparql_algebra::parser` wraps BOTH shapes in `Lateral`
/// unconditionally as an internal representation detail (every
/// property-function call, and every `SERVICE ?g`, is `Lateral`-wrapped even
/// with no `LATERAL` keyword ever written), and its serializer's own
/// `parser_rebuilds_the_lateral` mirrors this exact exclusion when deciding
/// whether the `LATERAL` keyword needs to be emitted on re-parse. Forwarding
/// either shape emits no `LATERAL` text AT THIS NODE — but the search does
/// not stop there: `left` and the auto-wrapped `right` (a property-function
/// call is a leaf with no children of its own; a variable-endpoint `Service`
/// has an `inner` that may itself contain a written `LATERAL`, arbitrarily
/// deeply nested) are both still searched, which is what closes the bypass
/// where a written `LATERAL` sits inside a `SERVICE ?g { … }` auto-wrap.
fn pattern_reaches_lateral(pattern: &GraphPattern) -> bool {
    if let GraphPattern::Lateral { left, right } = pattern {
        let parser_reconstructs_it = matches!(
            right.as_ref(),
            GraphPattern::PropertyFunction(_)
                | GraphPattern::Service {
                    name: NamedNodePattern::Variable(_),
                    ..
                }
        );
        return if parser_reconstructs_it {
            pattern_reaches_lateral(left) || pattern_reaches_lateral(right)
        } else {
            true
        };
    }
    let mut found = false;
    crate::governor::soundness::visit_pattern_parts(pattern, &mut |part| {
        found |= match part {
            crate::governor::soundness::PatternPart::Child(child, _edge) => {
                pattern_reaches_lateral(child)
            }
            crate::governor::soundness::PatternPart::Expression(expr) => {
                expression_reaches_lateral(expr)
            }
        };
        found
    });
    found
}

/// [`pattern_reaches_lateral`] through an expression's embedded patterns
/// (an `EXISTS`'s inner pattern, recursively).
fn expression_reaches_lateral(expr: &purrdf_sparql_algebra::Expression) -> bool {
    let mut found = false;
    crate::governor::soundness::visit_expression_parts(expr, &mut |part| {
        found |= match part {
            crate::governor::soundness::ExpressionPart::Exists(pattern) => {
                pattern_reaches_lateral(pattern)
            }
            crate::governor::soundness::ExpressionPart::Sub(inner) => {
                expression_reaches_lateral(inner)
            }
            crate::governor::soundness::ExpressionPart::Call(_) => false,
        };
        found
    });
    found
}

/// Strip every blank-node-carrying `VALUES` pushdown restriction from `pattern` before it
/// is serialized for a `SERVICE` request — the forwarding path's ONE consumer
/// ([`eval_service`], immediately before `pattern_to_select_query`). Never called from
/// local evaluation, which walks the unsanitized `pattern` returned by
/// [`crate::expr::substitute_pattern`] directly.
///
/// # What is being stripped, and why it is safe to drop rather than forward or refuse
///
/// [`crate::expr::substitute_pattern`]'s Values-Insertion rewrite (`join_leaf_with_values`,
/// `wrap_with_expr_term_only_values`) wraps a correlated `LATERAL`'s substituted leaves in
/// `Join(leaf, Values { v → outer_term })`: a SEMI-JOIN PUSHDOWN that restricts the
/// evaluation to the outer row's bindings. It is an OPTIMIZATION, not the sole source of
/// that restriction — [`crate::binop::eval_lateral`]'s own compatibility merge, joining
/// every row this node returns against the outer row μ on their shared variables, filters
/// to the identical rows regardless of whether the remote endpoint was ever told about the
/// restriction.
///
/// Blank-node scope is dataset-local (RDF 1.1 §3.4 / RDF 1.2 §3.5): the outer row's blank
/// node is a value THIS engine minted over THIS dataset, so no solution a remote endpoint
/// returns can ever bind the shared variable to that same node — the compatibility merge
/// above reduces every such row to incompatible, hence dropped, regardless of what the
/// remote sent back. Dropping the restriction from the FORWARDED TEXT therefore changes
/// nothing about the FINAL answer: the remote instead answers unrestricted for that
/// variable (a superset the local merge narrows back down to the same rows a satisfiable
/// restriction would have produced), while a request that included the restriction could
/// only ever have been satisfied by an endpoint that happens to share the exact same blank
/// node under the exact same scope — which, by construction, no other engine's dataset
/// does. Either way, the merge — not the pushdown — is what makes the joined result
/// correct; the pushdown is disposable.
///
/// The two alternatives are worse. Serializing the cell as `_:label` produces syntax the
/// `VALUES`/`DataBlock` grammar does not admit — it permits IRIs, literals, `UNDEF`, and,
/// in SPARQL 1.2, ground triple terms, but never a blank node — so a conforming endpoint
/// syntax-errors the request; under `SERVICE SILENT` that rejection degrades to the join
/// identity, which is exactly the silent-wrong-answer hazard this module's other `SILENT`
/// guards exist to close. A non-silent `SERVICE` would instead surface a confusing remote
/// syntax error for a query that has a perfectly well-defined answer. Refusing to forward
/// at all would deliver a real answer by refusal. Stripping is the only one of the three
/// that is both legal SPARQL and loses no information the local merge does not already
/// supply.
///
/// # Every stripped column is injection-produced, never user-written
///
/// The SPARQL `DataBlockValue` grammar admits no blank-node spelling, so
/// [`purrdf_sparql_algebra::parser`] never produces a `VALUES`/`BIND` block whose cell is
/// [`GroundTerm::BlankNode`] — see that variant's own doc ("injection-only"). The only
/// producer is the Values-Insertion rewrite named above, so a column this function strips
/// was never something the query author wrote; it is always bookkeeping this evaluator
/// itself added a moment earlier, for its own optimization purposes, and safe to remove
/// again for the same reason it was safe to add.
///
/// # Recursion
///
/// Exhaustive over [`GraphPattern`], so an injected `Values` node is found no differently
/// wherever Values-Insertion placed it — wrapped around a leaf at any depth, inside a
/// `Filter`/`Extend`/`OrderBy`/`Group`'s term-only wrapper, nested inside another `SERVICE`
/// or `LATERAL`, and so on. [`GroundTerm::Triple`] nests, so a cell is inspected
/// transitively: a ground quoted triple containing a blank node ANYWHERE in its
/// subject/object tree is stripped exactly like a bare blank-node cell.
fn sanitize_forwarded_body(pattern: &GraphPattern) -> GraphPattern {
    match pattern {
        // Leaves with no child pattern and no `Values` cells to inspect.
        GraphPattern::Bgp { patterns } => GraphPattern::Bgp {
            patterns: patterns.clone(),
        },
        GraphPattern::Path {
            subject,
            path,
            object,
        } => GraphPattern::Path {
            subject: subject.clone(),
            path: path.clone(),
            object: object.clone(),
        },
        GraphPattern::PropertyFunction(call) => GraphPattern::PropertyFunction(call.clone()),
        // The one node kind that can carry the hazard directly.
        GraphPattern::Values {
            variables,
            bindings,
        } => {
            let (variables, bindings) = strip_blank_columns(variables, bindings);
            GraphPattern::Values {
                variables,
                bindings,
            }
        }
        // The one node kind whose child, once sanitized, may need collapsing out of the
        // tree — see `join_dropping_empty_values`.
        GraphPattern::Join { left, right } => {
            let left = sanitize_forwarded_body(left);
            let right = sanitize_forwarded_body(right);
            join_dropping_empty_values(left, right)
        }
        // Every other node kind is a plain structural recursion: it carries no `Values`
        // cells of its own, and the wrapper Values-Insertion may have added around it sits
        // as one of ITS children, reached through the `Join` arm above.
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => GraphPattern::LeftJoin {
            left: Box::new(sanitize_forwarded_body(left)),
            right: Box::new(sanitize_forwarded_body(right)),
            expression: expression.clone(),
        },
        GraphPattern::Lateral { left, right } => GraphPattern::Lateral {
            left: Box::new(sanitize_forwarded_body(left)),
            right: Box::new(sanitize_forwarded_body(right)),
        },
        GraphPattern::Filter { expr, inner } => GraphPattern::Filter {
            expr: expr.clone(),
            inner: Box::new(sanitize_forwarded_body(inner)),
        },
        GraphPattern::Union { left, right } => GraphPattern::Union {
            left: Box::new(sanitize_forwarded_body(left)),
            right: Box::new(sanitize_forwarded_body(right)),
        },
        GraphPattern::Graph { name, inner } => GraphPattern::Graph {
            name: name.clone(),
            inner: Box::new(sanitize_forwarded_body(inner)),
        },
        GraphPattern::Extend {
            inner,
            variable,
            expression,
        } => GraphPattern::Extend {
            inner: Box::new(sanitize_forwarded_body(inner)),
            variable: variable.clone(),
            expression: expression.clone(),
        },
        GraphPattern::Minus { left, right } => GraphPattern::Minus {
            left: Box::new(sanitize_forwarded_body(left)),
            right: Box::new(sanitize_forwarded_body(right)),
        },
        GraphPattern::Service {
            name,
            inner,
            silent,
        } => GraphPattern::Service {
            name: name.clone(),
            inner: Box::new(sanitize_forwarded_body(inner)),
            silent: *silent,
        },
        GraphPattern::OrderBy { inner, expression } => GraphPattern::OrderBy {
            inner: Box::new(sanitize_forwarded_body(inner)),
            expression: expression.clone(),
        },
        GraphPattern::Project { inner, variables } => GraphPattern::Project {
            inner: Box::new(sanitize_forwarded_body(inner)),
            variables: variables.clone(),
        },
        GraphPattern::Distinct { inner } => GraphPattern::Distinct {
            inner: Box::new(sanitize_forwarded_body(inner)),
        },
        GraphPattern::Reduced { inner } => GraphPattern::Reduced {
            inner: Box::new(sanitize_forwarded_body(inner)),
        },
        GraphPattern::Slice {
            inner,
            start,
            length,
        } => GraphPattern::Slice {
            inner: Box::new(sanitize_forwarded_body(inner)),
            start: *start,
            length: *length,
        },
        GraphPattern::Group {
            inner,
            variables,
            aggregates,
        } => GraphPattern::Group {
            inner: Box::new(sanitize_forwarded_body(inner)),
            variables: variables.clone(),
            aggregates: aggregates.clone(),
        },
    }
}

/// Whether `term` is a blank node, or a ground RDF 1.2 quoted triple that transitively
/// contains one anywhere in its subject/object tree — [`GroundTriple::predicate`] is
/// always an IRI, so only the two recursive positions need inspecting.
fn ground_term_has_blank_node(term: &GroundTerm) -> bool {
    match term {
        GroundTerm::NamedNode(_) | GroundTerm::Literal(_) => false,
        GroundTerm::BlankNode(_) => true,
        GroundTerm::Triple(t) => {
            ground_term_has_blank_node(&t.subject) || ground_term_has_blank_node(&t.object)
        }
    }
}

/// Remove every column of a `Values` block whose cell, in ANY row, is a blank node or a
/// ground triple transitively containing one — [`sanitize_forwarded_body`]'s core rewrite.
/// Preserves the order of the surviving columns and the row count exactly (an `UNDEF`
/// cell, `None`, never carries a blank node, so it never causes a column to be dropped).
///
/// A `Values` block with no blank-carrying column (the overwhelmingly common case: every
/// user-written block, and every injected IRI/literal/ground-triple-without-a-blank
/// pushdown) is returned with its `Vec`s cloned but otherwise byte-for-byte unchanged —
/// there is no column to remove, so `keep` is all-`true` and the filter below is a no-op
/// pass.
fn strip_blank_columns(
    variables: &[Variable],
    bindings: &[Vec<Option<GroundTerm>>],
) -> (Vec<Variable>, Vec<Vec<Option<GroundTerm>>>) {
    let keep: Vec<bool> = (0..variables.len())
        .map(|column| {
            !bindings.iter().any(|row| {
                row.get(column)
                    .and_then(Option::as_ref)
                    .is_some_and(ground_term_has_blank_node)
            })
        })
        .collect();
    let new_variables = variables
        .iter()
        .zip(&keep)
        .filter(|&(_, &k)| k)
        .map(|(v, _)| v.clone())
        .collect();
    let new_bindings = bindings
        .iter()
        .map(|row| {
            row.iter()
                .zip(&keep)
                .filter(|&(_, &k)| k)
                .map(|(cell, _)| cell.clone())
                .collect()
        })
        .collect();
    (new_variables, new_bindings)
}

/// `Join(x, Values { variables: [], bindings: [[]] })` — a `Values` block
/// [`strip_blank_columns`] emptied of every column — is exactly the join identity
/// (`identity_seq`: one row, zero bindings), so `Join(x, that)` and `Join(that, x)` both
/// collapse to `x` rather than being serialized as a `VALUES { }` block, which the SPARQL
/// grammar does not admit as a standalone group graph pattern element. The row-count
/// guard (not merely `variables.is_empty()`) is deliberate: [`strip_blank_columns`]
/// preserves row count exactly, so an all-columns-stripped block still carries its
/// original row count, and an emptied block is the join identity only when that count is
/// the single row Values-Insertion always injects — a hypothetical zero-row all-columns
/// block (the empty relation, `FALSE`) is NOT the identity and must not be collapsed away.
fn join_dropping_empty_values(left: GraphPattern, right: GraphPattern) -> GraphPattern {
    if is_join_identity_values(&right) {
        left
    } else if is_join_identity_values(&left) {
        right
    } else {
        GraphPattern::Join {
            left: Box::new(left),
            right: Box::new(right),
        }
    }
}

/// See [`join_dropping_empty_values`].
fn is_join_identity_values(pattern: &GraphPattern) -> bool {
    matches!(
        pattern,
        GraphPattern::Values { variables, bindings }
            if variables.is_empty() && bindings.len() == 1 && bindings[0].is_empty()
    )
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
    // A `LATERAL` clause inside a forwarded body is refused ONLY under `SERVICE
    // SILENT` — scoped to the hazard it actually guards against, the same
    // treatment the custom-scalar-function refusal further down gets, rather than
    // the unconditional treatment the property-function and custom-aggregate
    // refusals get. `LATERAL` is a SEP-0006/Jena syntax extension most SPARQL
    // 1.1/1.2-only endpoints do not implement, so a forwarded `LATERAL` is likely
    // to be rejected by an endpoint that lacks it — and under `SERVICE SILENT`,
    // that rejection would degrade to the join identity: a result that looks
    // complete and is wrong. A plain, non-silent `SERVICE` has no such hazard to
    // close: the endpoint's verdict — an answer from a `LATERAL`-capable endpoint
    // (Jena's own extension; a Jena-backed endpoint answers it), or an honest
    // `EvalError::Remote` from one that rejects it — surfaces exactly as it does
    // for any other forwarded construct the remote might not support, so the body
    // (including its `LATERAL { … }` text) is forwarded rather than refused.
    if silent && pattern_reaches_lateral(inner) {
        return Err(EvalError::unsupported(
            "a LATERAL clause inside a SERVICE SILENT body: LATERAL is a SEP-0006/Jena syntax \
             extension most remote SPARQL endpoints do not implement, and SILENT would swallow \
             the endpoint's rejection into the join identity — a result that looks complete and \
             is wrong; drop SILENT and the query forwards the LATERAL text, surfacing the \
             endpoint's verdict (an answer, or an honest failure) instead",
        ));
    }
    // A property-function call inside a forwarded body is a HARD refusal, and it is
    // tested before anything else — before `SILENT`, before the endpoint, before any
    // charge. The body is serialized and sent as SPARQL text, and a call serializes as
    // an ordinary triple: the remote endpoint would match it against ITS data and return
    // rows that are not the relation's, with no symptom anywhere. Silencing that under
    // `SILENT` would be worse still, because `SILENT` promises an empty result from a
    // failed endpoint, not a full one from a misread query.
    if crate::property_fn_eval::pattern_reaches_property_function(inner) {
        return Err(EvalError::unsupported(
            "a property-function call inside a SERVICE body: the call would be forwarded as \
             an ordinary triple pattern and matched against the remote endpoint's data, so \
             the relation would never be invoked and the answer would be silently wrong",
        ));
    }
    // A custom aggregate call inside a forwarded body is refused the same way, for the
    // same reason: `AGG(<iri>, …)` serializes as text the remote endpoint has no
    // registered meaning for, and this engine's registry — the one place the IRI is
    // actually resolved — never sees the call at all once it has been shipped away.
    // Hard, unconditional on `SILENT`, before any charge — the exact treatment the
    // property-function refusal above gets, and for the identical reason: `SILENT`
    // promises an empty result from an unreachable ENDPOINT, never a wrong (or
    // endpoint-syntax-error) one from a request that could never have meant what it
    // meant locally.
    if crate::property_fn_eval::pattern_reaches_custom_aggregate(inner) {
        return Err(EvalError::unsupported(
            "a custom-aggregate call inside a SERVICE body: `AGG(<iri>, …)` would be forwarded \
             as text the remote endpoint has no registered meaning for, so this engine's \
             aggregate registry would never resolve the call and the answer would be silently \
             wrong",
        ));
    }
    // A custom SCALAR function call inside a forwarded body is NOT the unconditional twin
    // of the two refusals above — it is scoped to `SILENT` only, and deliberately so.
    //
    // `Function::Custom` serializes as ordinary function-call syntax (`<iri>(args…)`), and
    // that is exactly the shape the SPARQL specification expects for a remote endpoint's
    // OWN extension functions: `purrdf_sparql_algebra::parser` falls through to
    // `Function::Custom` for every call-position IRI that is not a builtin and not under a
    // *configured* extension namespace (see its module docs), so this call form is the
    // normal, spec-sanctioned way to invoke a function the LOCAL engine does not know but
    // the endpoint might. A property-function call and a custom-aggregate call have no such
    // meaning at a remote endpoint — a relation IRI just matches ITS data as an ordinary
    // triple with no symptom, and `AGG(<iri>, …)` is this engine's own registry syntax — so
    // those two stay unconditional refusals (see above). A custom scalar function is
    // different: an endpoint that has no such function fails LOUDLY on its own account
    // (`pattern_reaches_custom_function`'s doc: this engine's own evaluation of an
    // unresolved `Function::Custom` already raises a typed error rather than silently
    // matching unrelated data), and the non-silent path below turns any such endpoint
    // failure into an honest [`EvalError::Remote`] — there is no silent-wrong-answer hazard
    // to close for a plain, non-silent `SERVICE`.
    //
    // The hazard is real ONLY under `SERVICE SILENT`: `SILENT` swallows an endpoint failure
    // to the join identity, so a loud, honest "no such function" failure at the endpoint
    // would be swallowed into a silent wrong answer. So the refusal is gated on `silent`,
    // and its message names the escape — dropping `SILENT` makes the query forward and
    // work — so the refusal is actionable rather than a dead end.
    if silent && crate::property_fn_eval::pattern_reaches_custom_function(inner) {
        return Err(EvalError::unsupported(
            "a custom scalar-function call inside a SERVICE SILENT body: the call would be \
             forwarded as ordinary function-call syntax, and if the remote endpoint has no \
             such function the failure would degrade to the join identity under SILENT — a \
             silent wrong answer rather than an honest one; drop SILENT and the query \
             forwards and answers normally, with an unrecognized function surfacing as an \
             honest remote failure instead",
        ));
    }
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
            SolutionSeq::empty(crate::eval::syntactic_schema(inner)),
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
            SolutionSeq::empty(crate::eval::syntactic_schema(inner)),
            tripped,
        )));
    }

    // Strip any blank-node-carrying `VALUES` pushdown correlated substitution injected
    // into `inner`, BEFORE serialization — see [`sanitize_forwarded_body`]'s doc for why
    // dropping it (never refusing, never emitting the illegal `_:` cell) is the sound,
    // maximal-utility fix. Local evaluation never runs this: it walks `inner` fresh, not
    // the sanitized copy.
    let sanitized = sanitize_forwarded_body(inner);
    let query_text = purrdf_sparql_algebra::pattern_to_select_query(&sanitized);
    // The signal travels WITH the call: while the evaluator is blocked inside it, nothing
    // else is in a position to poll.
    let stop = ctx.stop_signal().map(Arc::clone);
    let max_intermediate_cells = ctx.governor_state().and_then(|state| {
        let dimension = purrdf_core::ResourceDimension::IntermediateCells;
        state
            .is_engaged_in(dimension)
            .then(|| state.limits().get(dimension))
    });
    let response = source.query(
        &endpoint,
        &query_text,
        stop.as_ref(),
        max_intermediate_cells,
    );
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
                SolutionSeq::empty(crate::eval::syntactic_schema(inner)),
                ctx.record_trip(governor),
            )));
        }
        Err(RemoteError::GovernedAfterCompletion(governor)) => {
            return Ok(Evaluated::Truncated(Truncation::bag_only_origin(
                SolutionSeq::empty(crate::eval::syntactic_schema(inner)),
                ctx.record_trip(governor),
            )));
        }
        Err(e) => {
            // A real endpoint failure outranks a simultaneous stop. Under SILENT the
            // endpoint failure is deliberately erased, so the stop becomes the surviving
            // fact and must remain non-silenceable.
            if silent && let Some(tripped) = post_return_trip {
                return Ok(Evaluated::Truncated(Truncation::bag_only_origin(
                    SolutionSeq::empty(crate::eval::syntactic_schema(inner)),
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
/// # The cell ceiling is tested before each row is interned
///
/// A remote bag is an intermediate bag like any other, so it is measured against
/// [`ResourceDimension::IntermediateCells`](purrdf_core::ResourceDimension::IntermediateCells)
/// — and measured before the next interned row grows the local bag. Every other operator's
/// bag is bounded by data the ceiling was sized against; this one is bounded by whatever a
/// remote endpoint chose to send, which is the only bag in the evaluator an attacker (or
/// a mistake) can size directly. The source seam receives the same bound so a built-in
/// decoder can avoid constructing an over-limit owned response in the first place; this
/// check remains the mandatory backstop for injected sources.
fn ingest<D: DatasetView + Sync>(
    resolved: ResolvedBindings,
    ctx: &mut EvalCtx<'_, D>,
) -> (SolutionSeq<D::Id>, Option<TrippedGovernor>) {
    let ResolvedBindings {
        variables,
        rows: resolved_rows,
        cell_limit_exceeded_at,
    } = resolved;
    let schema = Arc::new(VarSchema::from_vars(variables));
    // The shared admission sequence (see `crate::row_ingest`): ceiling observed before
    // the row is interned, then the per-row charge, then the intern. `SERVICE` names its
    // own charge point here; the sequence itself is the one a property-function call
    // also runs, so the two cannot drift apart.
    let ingest = crate::row_ingest::GovernedRowIngest::new(
        ctx,
        schema.len(),
        Some(crate::governor::ChargePoint::RemoteRowIngested),
    );
    let mut rows = Vec::with_capacity(ingest.capacity_for(resolved_rows.len()));
    let mut tripped = None;
    for binding in resolved_rows {
        match ingest.admit(ctx, rows.len()) {
            crate::row_ingest::RowAdmission::Abandoned(governor) => {
                tripped = governor;
                break;
            }
            crate::row_ingest::RowAdmission::Admitted => {}
        }
        let row = ingest.intern_row(ctx, binding);
        rows.push(row);
    }
    if tripped.is_none()
        && let Some(attempted_cells) = cell_limit_exceeded_at
    {
        tripped = ctx.observe_cell_count(attempted_cells).err();
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
    /// The forwarded evaluation carries the caller's intermediate-cell ceiling, so an
    /// in-memory endpoint cannot materialize a bag the caller already bounded. It carries
    /// no *charge* ceilings: fuel spent here is already charged at the calling seam, per
    /// request and per ingested row, and charging it twice would make one query's budget
    /// depend on how a federation happened to be split up.
    fn query(
        &self,
        endpoint: &str,
        query_text: &str,
        stop: Option<&Arc<dyn StopSignal>>,
        max_intermediate_cells: Option<u64>,
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
        if stop.is_some() || max_intermediate_cells.is_some() {
            let mut governors = QueryGovernors::UNBOUNDED;
            if let Some(signal) = stop {
                governors = governors.with_stop_signal(Arc::clone(signal));
            }
            if let Some(cells) = max_intermediate_cells {
                governors = governors.with_max_intermediate_cells(cells);
            }
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
                    cell_limit_exceeded_at: None,
                })
            }
            EvaluatedOutcome::Complete(_) => Err(RemoteError::Decode(
                "SERVICE expects a SELECT query".to_owned(),
            )),
            EvaluatedOutcome::Truncated {
                outcome: Outcome::Solutions(seq),
                certificate,
            } if matches!(
                certificate.tripped(),
                TrippedGovernor::Budget {
                    dimension: purrdf_core::ResourceDimension::IntermediateCells,
                    ..
                }
            ) =>
            {
                // The forwarded evaluator may have crossed its ceiling below a
                // non-monotone operator, so do not promote its possibly-unknown rows to a
                // remote prefix. The empty prefix is always a sound lower bound; the flag
                // makes the outer evaluator record the same typed cell trip.
                Ok(ResolvedBindings {
                    variables: seq.schema.vars().to_vec(),
                    rows: Vec::new(),
                    cell_limit_exceeded_at: match certificate.tripped() {
                        TrippedGovernor::Budget { consumed, .. } => Some(consumed),
                        _ => None,
                    },
                })
            }
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
        BlankScope, RdfDatasetBuilder, RdfLiteral, ResourceDimension, SparqlEngine, SparqlRequest,
        SparqlResult, StopCause,
    };
    use purrdf_sparql_algebra::{
        BlankNode, GroundTerm, GroundTriple, NamedNode, TermPattern, TriplePattern,
    };
    use std::sync::Mutex;
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

    // ── custom-aggregate / custom-function SERVICE forwarding refusals ────────

    #[test]
    fn custom_aggregate_inside_a_service_body_is_refused_at_the_forwarding_boundary() {
        let source = LocalRemoteQuerySource::new();
        let err = run_with_source(
            &local(),
            &source,
            "SELECT ?s WHERE { SERVICE <http://ep> { \
             SELECT ?s (AGG(<http://ex/customAgg>, ?n) AS ?v) WHERE { ?s <http://ex/name> ?n } \
             GROUP BY ?s } }",
        )
        .unwrap_err();
        assert!(matches!(err, EvalError::Unsupported { .. }), "got {err:?}");
        assert!(
            err.to_string()
                .contains("custom-aggregate call inside a SERVICE body"),
            "got {err}"
        );
    }

    #[test]
    fn custom_aggregate_inside_a_silent_service_body_is_refused_too() {
        let source = LocalRemoteQuerySource::new();
        let err = run_with_source(
            &local(),
            &source,
            "SELECT ?s WHERE { SERVICE SILENT <http://ep> { \
             SELECT ?s (AGG(<http://ex/customAgg>, ?n) AS ?v) WHERE { ?s <http://ex/name> ?n } \
             GROUP BY ?s } }",
        )
        .unwrap_err();
        assert!(
            err.to_string()
                .contains("custom-aggregate call inside a SERVICE body"),
            "SILENT promises an empty result from a failed endpoint, never a wrong one from a \
             misread AGG(<iri>, …) call: {err}"
        );
    }

    // ── LATERAL / SERVICE forwarding guard ─────────────────────────────────────

    #[test]
    fn silent_service_with_a_lateral_body_is_refused() {
        // A `LATERAL` clause inside a `SERVICE SILENT` body is refused: `LATERAL`
        // is a SEP-0006/Jena syntax extension most remote endpoints do not
        // implement, and under `SILENT` a rejection would degrade to the join
        // identity — a result that looks complete and is wrong. Never reaches the
        // source (the source below has no registered endpoints, so a forwarded
        // attempt would surface as a transport error instead of this typed one):
        // the refusal is typed and happens before dispatch.
        let source = LocalRemoteQuerySource::new();
        let query = "SELECT ?s WHERE { SERVICE SILENT <https://example.org/lateral-forward#ep> { \
                      ?s <https://example.org/lateral-forward#knows> ?o \
                      LATERAL { ?o <https://example.org/lateral-forward#name> ?n } } }";
        let err = run_with_source(&local(), &source, query).unwrap_err();
        assert!(matches!(err, EvalError::Unsupported { .. }), "got {err:?}");
        assert!(
            err.to_string()
                .contains("LATERAL clause inside a SERVICE SILENT body"),
            "got {err}"
        );
        assert!(
            err.to_string().contains("SILENT"),
            "the refusal must name SILENT as the reason it fires: {err}"
        );
    }

    #[test]
    fn nonsilent_service_forwards_lateral_text() {
        // The regression this pins: a non-silent SERVICE body containing a WRITTEN
        // LATERAL clause must forward the body — LATERAL text included — to the
        // endpoint, because the hazard the SILENT-scoped refusal guards against (a
        // SILENT clause swallowing the endpoint's rejection into the join
        // identity) does not exist here: the endpoint's verdict, whatever it is,
        // surfaces honestly. This also demonstrates that
        // `purrdf_sparql_algebra::serialize`'s `LATERAL` emission arm is live
        // production code, not a producer with no consumer: LATERAL is Apache
        // Jena's own extension, and a Jena-backed (or otherwise LATERAL-capable)
        // endpoint answers it.
        let posts = AtomicUsize::new(0);
        let source = HttpRemoteQuerySource::new(|request: HttpRequest<'_>| {
            posts.fetch_add(1, Ordering::Relaxed);
            assert!(
                request.query_text.contains("LATERAL {"),
                "the LATERAL clause must reach the endpoint verbatim: {}",
                request.query_text
            );
            Ok(br#"{"head":{"vars":["n"]},"results":{"bindings":[
                {"n":{"type":"literal","value":"X"}}
            ]}}"#
                .to_vec())
        });
        let result = run_with_source(
            &local(),
            &source,
            "SELECT ?n WHERE { SERVICE <http://ep> { \
             ?s <http://ex/name> ?n LATERAL { ?n <http://ex/knows> ?x } } }",
        )
        .expect("a non-silent SERVICE body must forward a written LATERAL clause");
        assert_eq!(
            posts.load(Ordering::Relaxed),
            1,
            "the request must actually be issued, not refused locally"
        );
        assert_eq!(row_strings(&result), vec![vec!["X".to_owned()]]);
    }

    #[test]
    fn silent_service_lateral_nested_in_variable_endpoint_is_refused() {
        // The bypass this pins closed: a written LATERAL sitting inside a nested
        // `SERVICE ?g { … }` (a variable-endpoint SERVICE, itself an auto-wrap
        // `Lateral` the parser reconstructs without a keyword) used to escape the
        // guard, because the guard's own `Lateral` arm recursed into `left` only
        // and never inspected the auto-wrapped `right`'s `Service::inner`. A
        // written LATERAL nested that way must still be found and refused under
        // `SERVICE SILENT`.
        let source = LocalRemoteQuerySource::new();
        let query = "SELECT ?s WHERE { SERVICE SILENT <https://example.org/lateral-forward#ep> { \
                      ?a <https://example.org/lateral-forward#hasEndpoint> ?g \
                      SERVICE ?g { ?c <https://example.org/lateral-forward#q> ?d \
                      LATERAL { ?e <https://example.org/lateral-forward#f> ?f } } } }";
        let err = run_with_source(&local(), &source, query).unwrap_err();
        assert!(matches!(err, EvalError::Unsupported { .. }), "got {err:?}");
        assert!(
            err.to_string()
                .contains("LATERAL clause inside a SERVICE SILENT body"),
            "the LATERAL nested inside the variable-endpoint SERVICE ?g must still be found: \
             got {err}"
        );
    }

    #[test]
    fn custom_function_inside_a_non_silent_service_body_forwards_and_answers() {
        // The regression this pins: `Function::Custom` is exactly how the SPARQL grammar
        // expects a remote endpoint's OWN extension functions to be written (the parser
        // falls through to `Function::Custom` for any call-position IRI outside a
        // configured namespace), so a non-silent SERVICE body containing one must still be
        // forwarded — an endpoint that does not recognize it fails loudly on its own
        // account, and that failure already surfaces as an honest `EvalError::Remote`
        // rather than a silent wrong answer.
        let posts = AtomicUsize::new(0);
        let source = HttpRemoteQuerySource::new(|request: HttpRequest<'_>| {
            posts.fetch_add(1, Ordering::Relaxed);
            assert!(
                request.query_text.contains("http://ex/customFn"),
                "the custom function call must reach the endpoint verbatim: {}",
                request.query_text
            );
            Ok(br#"{"head":{"vars":["n"]},"results":{"bindings":[
                {"n":{"type":"literal","value":"X"}}
            ]}}"#
                .to_vec())
        });
        let result = run_with_source(
            &local(),
            &source,
            "SELECT ?n WHERE { SERVICE <http://ep> { \
             ?s <http://ex/name> ?n . FILTER(<http://ex/customFn>(?n) > 0) } }",
        )
        .expect("a non-silent SERVICE body must forward a custom scalar-function call");
        assert_eq!(
            posts.load(Ordering::Relaxed),
            1,
            "the request must actually be issued, not refused locally"
        );
        assert_eq!(row_strings(&result), vec![vec!["X".to_owned()]]);
    }

    #[test]
    fn custom_function_inside_a_silent_service_body_is_refused_with_the_escape_named() {
        let source = LocalRemoteQuerySource::new();
        let err = run_with_source(
            &local(),
            &source,
            "SELECT ?s WHERE { SERVICE SILENT <http://ep> { \
             ?s <http://ex/name> ?n . FILTER(<http://ex/customFn>(?n) > 0) } }",
        )
        .unwrap_err();
        assert!(matches!(err, EvalError::Unsupported { .. }), "got {err:?}");
        assert!(
            err.to_string()
                .contains("custom scalar-function call inside a SERVICE SILENT body"),
            "SILENT must not launder a forwarded custom-function call into a degrade-to-\
             join-identity wrong answer: {err}"
        );
        assert!(
            err.to_string().contains("drop SILENT"),
            "the refusal must name the escape so it is actionable rather than a dead end: {err}"
        );
    }

    #[test]
    fn a_custom_function_nested_inside_another_calls_arguments_still_trips_the_silent_guard() {
        // The regression this pins: `visit_expression_parts`'s `FunctionCall` arm visits
        // each argument as `ExpressionPart::Sub`, and `expression_reaches_custom_function`
        // recurses into every `Sub`, so a `Function::Custom` call need not be the
        // OUTERMOST call in the expression tree to be found — it can be buried inside
        // another call's argument list. `CONCAT(<http://ex/customFn>(?n), "a")` nests the
        // custom call one level down; if the walk only inspected an expression's own
        // `Call` part and never descended into `FunctionCall` arguments, this query would
        // slip the guard and forward under `SILENT`.
        let source = LocalRemoteQuerySource::new();
        let err = run_with_source(
            &local(),
            &source,
            "SELECT ?s WHERE { SERVICE SILENT <http://ep> { \
             ?s <http://ex/name> ?n . \
             FILTER(CONCAT(<http://ex/customFn>(?n), \"a\") = \"Xa\") } }",
        )
        .unwrap_err();
        assert!(matches!(err, EvalError::Unsupported { .. }), "got {err:?}");
        assert!(
            err.to_string()
                .contains("custom scalar-function call inside a SERVICE SILENT body"),
            "a custom call nested inside CONCAT's argument list must trip the same guard a \
             top-level custom call does: {err}"
        );
    }

    #[test]
    fn custom_aggregate_refusal_precedes_the_no_source_check() {
        // No remote source configured at all: the forwarding refusal must still win —
        // it is checked before the endpoint/source resolution, exactly like the
        // property-function refusal it mirrors.
        let engine = NativeSparqlEngine::new();
        let err = engine
            .query(
                &local(),
                SparqlRequest {
                    query: "SELECT ?s WHERE { SERVICE <http://ep> { \
                            SELECT ?s (AGG(<http://ex/customAgg>, ?n) AS ?v) WHERE { \
                            ?s <http://ex/name> ?n } GROUP BY ?s } }",
                    base_iri: None,
                    substitutions: &[],
                },
            )
            .unwrap_err();
        assert!(
            err.message
                .contains("custom-aggregate call inside a SERVICE body"),
            "got: {}",
            err.message
        );
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
            max_intermediate_cells: Option<u64>,
        ) -> Result<ResolvedBindings, RemoteError> {
            self.count.fetch_add(1, Ordering::Relaxed);
            self.inner
                .query(endpoint, query_text, stop, max_intermediate_cells)
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
            max_intermediate_cells: Option<u64>,
        ) -> Result<ResolvedBindings, RemoteError> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            if let Some(cause) = stop.and_then(|signal| signal.poll()) {
                return Err(RemoteError::Governed(TrippedGovernor::Stopped { cause }));
            }
            let admitted = max_intermediate_cells
                .and_then(|cells| usize::try_from(cells).ok())
                .unwrap_or(self.rows)
                .min(self.rows);
            Ok(ResolvedBindings {
                variables: vec![Variable::new("n")],
                rows: (0..admitted)
                    .map(|i| vec![Some(TermValue::Iri(format!("{EX}r{i}")))])
                    .collect(),
                cell_limit_exceeded_at: (self.rows > admitted)
                    .then(|| (admitted as u64).saturating_add(1)),
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
            _max_intermediate_cells: Option<u64>,
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
        assert_eq!(
            run.columns, 3,
            "an immediate trip still carries the SERVICE pattern's declared schema"
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
        assert_eq!(
            run.columns, 3,
            "a source-reported trip still carries the SERVICE pattern's declared schema"
        );
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
        assert_eq!(
            run.columns, 3,
            "a pre-dispatch stop still carries the SERVICE pattern's declared schema"
        );

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
                None,
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

        // A cell ceiling below the response's size trips the cardinality governor. The
        // source materializes only the admitted prefix and reports the first refused row;
        // ingest interns that prefix but never allocates the overflowing row.
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
            capped.rows,
            N - 1,
            "the admitted prefix is useful, and the limit+1 row is never materialized"
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

    // ── Forwarded-body sanitizer: blank-node VALUES pushdown stripping ────────────

    /// `:s <http://ex/hasAnon> _:bn` — an outer row whose bound variable is a blank
    /// node, the shape that drives correlated substitution's injected `Values`
    /// pushdown into a forwarded `SERVICE` body.
    fn local_with_blank() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let has_anon = b.intern_iri("http://ex/hasAnon");
        let s = b.intern_iri("http://ex/s");
        let bn = b.intern_blank("bn", BlankScope::DEFAULT);
        b.push_quad(s, has_anon, bn, None);
        b.freeze().expect("freeze")
    }

    /// `:s <http://ex/hasTarget> <http://ex/t>` — the same LATERAL/SERVICE shape as
    /// [`local_with_blank`], but the outer-bound variable is an IRI: the pushdown
    /// restriction over it IS legal `VALUES` syntax and must survive forwarding.
    fn local_with_iri() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let has_target = b.intern_iri("http://ex/hasTarget");
        let s = b.intern_iri("http://ex/s");
        let t = b.intern_iri("http://ex/t");
        b.push_quad(s, has_target, t, None);
        b.freeze().expect("freeze")
    }

    #[test]
    fn forwarded_service_text_never_carries_a_blank_node_values_cell() {
        // The regression this pins: correlated substitution
        // injects `Join(leaf, Values { bn -> _:blank })` into a LATERAL SERVICE body
        // when the outer row binds a blank node. Forwarding that unsanitized would
        // serialize the cell as `_:label`, which the VALUES/DataBlock grammar does not
        // admit — a conforming endpoint syntax-errors the request, and under SILENT
        // that degrades to the join identity: a silent wrong answer.
        let posts = AtomicUsize::new(0);
        let captured = Mutex::new(String::new());
        let source = HttpRemoteQuerySource::new(|request: HttpRequest<'_>| {
            posts.fetch_add(1, Ordering::Relaxed);
            *captured.lock().expect("lock") = request.query_text.to_owned();
            Ok(br#"{"head":{"vars":["x","bn"]},"results":{"bindings":[
                {"x":{"type":"uri","value":"http://ex/remoteX"},
                 "bn":{"type":"uri","value":"http://ex/notOurBlank"}}
            ]}}"#
                .to_vec())
        });
        let result = run_with_source(
            &local_with_blank(),
            &source,
            "SELECT * WHERE { ?s <http://ex/hasAnon> ?bn \
             LATERAL { SERVICE <http://ep> { ?x <http://ex/rel> ?bn } } }",
        )
        .expect("a forwarded blank-node pushdown must be stripped, never refused");
        assert_eq!(
            posts.load(Ordering::Relaxed),
            1,
            "the request must actually be issued, not refused locally"
        );
        let text = captured.lock().expect("lock").clone();
        assert!(
            !text.contains("_:"),
            "no blank-node cell may reach the wire: {text}"
        );
        assert!(
            !text.contains("VALUES {  }") && !text.contains("VALUES { }"),
            "no empty VALUES block may reach the wire either: {text}"
        );
        // The remote's row does not (and cannot) carry OUR blank node, so the LATERAL
        // compatibility merge — not the forwarded restriction — is what filters it to
        // zero rows: the local merge, not the dropped pushdown, is what keeps this correct.
        assert_eq!(row_strings(&result), Vec::<Vec<String>>::new());
    }

    #[test]
    fn forwarded_service_text_keeps_iri_and_literal_pushdown() {
        // Same LATERAL/SERVICE shape, but the outer-bound variable is an IRI: the
        // pushdown restriction is legal syntax and must survive forwarding — the
        // sanitizer targets blank-node cells only, not every injected restriction.
        let posts = AtomicUsize::new(0);
        let captured = Mutex::new(String::new());
        let source = HttpRemoteQuerySource::new(|request: HttpRequest<'_>| {
            posts.fetch_add(1, Ordering::Relaxed);
            *captured.lock().expect("lock") = request.query_text.to_owned();
            Ok(br#"{"head":{"vars":["x"]},"results":{"bindings":[]}}"#.to_vec())
        });
        let _ = run_with_source(
            &local_with_iri(),
            &source,
            "SELECT * WHERE { ?s <http://ex/hasTarget> ?t \
             LATERAL { SERVICE <http://ep> { ?x <http://ex/rel> ?t } } }",
        )
        .expect("query");
        assert_eq!(
            posts.load(Ordering::Relaxed),
            1,
            "the request must actually be issued"
        );
        let text = captured.lock().expect("lock").clone();
        assert!(
            text.contains("VALUES") && text.contains("<http://ex/t>"),
            "the IRI pushdown restriction must survive forwarding: {text}"
        );
    }

    #[test]
    fn sanitize_forwarded_body_strips_a_ground_triple_containing_a_blank_node() {
        // Unit test on the sanitizer directly: a Values cell that is not itself a blank
        // node, but a GROUND TRIPLE whose subject is one, must still be found and its
        // whole column stripped — `GroundTerm::Triple` nests, and the walk must follow it.
        let pattern = GraphPattern::Join {
            left: Box::new(GraphPattern::Bgp {
                patterns: vec![TriplePattern {
                    subject: TermPattern::Variable(Variable::new("s")),
                    predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked("http://ex/p")),
                    object: TermPattern::Variable(Variable::new("o")),
                }],
            }),
            right: Box::new(GraphPattern::Values {
                variables: vec![Variable::new("t"), Variable::new("k")],
                bindings: vec![vec![
                    Some(GroundTerm::Triple(Box::new(GroundTriple {
                        subject: GroundTerm::BlankNode(BlankNode::new("b1")),
                        predicate: NamedNode::new_unchecked("http://ex/embeds"),
                        object: GroundTerm::NamedNode(NamedNode::new_unchecked("http://ex/o")),
                    }))),
                    Some(GroundTerm::NamedNode(NamedNode::new_unchecked(
                        "http://ex/keep",
                    ))),
                ]],
            }),
        };
        let sanitized = sanitize_forwarded_body(&pattern);
        let GraphPattern::Join { right, .. } = &sanitized else {
            panic!("expected a Join, got {sanitized:?}");
        };
        let GraphPattern::Values {
            variables,
            bindings,
        } = right.as_ref()
        else {
            panic!("expected the right operand to remain a Values node, got {right:?}");
        };
        assert_eq!(
            variables,
            &vec![Variable::new("k")],
            "the ?t column (its cell embeds a blank node) must be stripped, ?k kept"
        );
        assert_eq!(
            bindings,
            &vec![vec![Some(GroundTerm::NamedNode(NamedNode::new_unchecked(
                "http://ex/keep"
            )))]]
        );
    }

    #[test]
    fn sanitize_forwarded_body_collapses_an_all_blank_values_out_of_its_join() {
        // The all-columns-stripped case: `Join(leaf, Values { bn -> _:blank })` must
        // collapse to `leaf` alone, never serialize as `Join(leaf, VALUES { })` — which
        // is not valid SPARQL as a standalone group graph pattern element.
        let leaf = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: TermPattern::Variable(Variable::new("x")),
                predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked("http://ex/rel")),
                object: TermPattern::Variable(Variable::new("bn")),
            }],
        };
        let pattern = GraphPattern::Join {
            left: Box::new(leaf.clone()),
            right: Box::new(GraphPattern::Values {
                variables: vec![Variable::new("bn")],
                bindings: vec![vec![Some(GroundTerm::BlankNode(BlankNode::new("bn")))]],
            }),
        };
        let sanitized = sanitize_forwarded_body(&pattern);
        assert_eq!(
            sanitized, leaf,
            "an all-columns-stripped injected Values must collapse out of the Join entirely"
        );
    }
}
