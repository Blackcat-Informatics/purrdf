// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The synchronous, offline SPARQL query surface over the wasm [`Dataset`].
//!
//! Binds the native multiset SPARQL evaluator
//! ([`NativeSparqlEngine`](purrdf_sparql_eval::NativeSparqlEngine)) to JavaScript so a
//! page can run SELECT / ASK / CONSTRUCT / DESCRIBE entirely client-side, with no
//! server and no network. The engine is the same one the native query gate uses,
//! with no baked-in HTTP client.
//!
//! ## Two lanes: this one is offline
//!
//! Every method here is the *synchronous* lane: it runs to completion inside one wasm
//! call and installs **no** [`ServiceResolver`](purrdf_sparql_eval::remote) and no
//! `GraphResolver`. A `SERVICE` or `LOAD` clause therefore **hard-fails** with a JsError
//! rather than silently returning an empty or partial result: a synchronous call cannot
//! wait for the network, and a false answer is worse than an error.
//!
//! The one exception is the caller's own: `SERVICE SILENT` and `LOAD SILENT` succeed
//! with nothing fetched. `SILENT` is the query author writing "an invocation that fails
//! is not an error" into the request, and SPARQL 1.1 (Federated Query §3.2, Update
//! §3.1.4) requires it to be honoured: `SERVICE SILENT` contributes the join identity (so
//! the surrounding pattern's own solutions come back, unaugmented) and `LOAD SILENT`
//! leaves the dataset untouched. The governed entries record each such invocation on
//! [`GovernorEvidence::silenced`]. Drop `SILENT` to get the hard failure.
//!
//! The second lane is the `async_query` module: every evaluating method here has an
//! asynchronous twin that runs the same evaluator as a job suspending through JSPI on
//! host-resolved `SERVICE` and `LOAD` effects, so federation is the host's to supply
//! there and never a hidden network client here.
//!
//! ## Result encoding
//!
//! - SELECT / ASK → **SPARQL Results JSON** (the W3C SRJ format) via
//!   [`purrdf_sparql_results`].
//! - CONSTRUCT / DESCRIBE → **Turtle**, or **TriG** when the result carries a named
//!   graph, via the `native_codecs` serializer (the one serialization seam; never
//!   the `purrdf-gts` crate). See [`default_graph_format`]: a
//!   quad-template CONSTRUCT would serialize to an EMPTY Turtle document, so the
//!   no-format default widens to Turtle's dataset superset rather than answering with
//!   nothing. An EXPLICIT single-graph format for such a result throws instead — see
//!   [`refuse_uncarriable_named_graphs`].
//!
//! ## The governed lane
//!
//! [`QueryEngine::query_governed`] and [`QueryEngine::update_governed`] bind the
//! evaluator's governed entries — caller-supplied ceilings on fuel, the answer sequence,
//! the largest intermediate bag, the scratch arena, and remote requests, plus a wall
//! deadline and a [`CancellationToken`] the page can flip.
//!
//! **A tripped governor is an outcome, never a thrown error.** A trip is neither a
//! complete answer nor a failure: thrown, it would discard the rows the budget already
//! paid for and tell the caller the engine misbehaved; reported as complete, a truncated
//! answer would be silently wrong. So both governed entries return a typed
//! [`QueryOutcome`] / [`UpdateOutcome`] object on **both** paths, and only a genuine parse
//! or evaluation error (or a malformed ceiling, which is a caller bug rather than an
//! execution outcome) leaves this seam as a `JsError`.
//!
//! The wall deadline is the one host-platform clock read on this path. It lives in
//! [`WallDeadline`], which is written per target inside `purrdf-sparql-eval` — a wasm
//! build reads the host's `Date.now()` rather than `std::time::Instant`, which would compile
//! here and panic at run time. The Node round-trip lane (`js/tests/governors.test.mjs`)
//! executes a real deadline trip against the optimized module so that split is *observed*
//! rather than merely compiled.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use purrdf::{GovernedEntailment, JsonLdSerializeOptions, SerializeGraph, serialize_dataset};
use purrdf_core::named_graph::{distinct_graph_names, named_graph_refusal};
use purrdf_core::{SparqlRequest, SparqlResult};
use purrdf_sparql_eval::protocol::FailureCode;
use purrdf_sparql_eval::{
    AggregateRegistry, BudgetExhausted, CancellationFlag, DivisionPolicy, GovernedOutcome,
    GovernedUpdateOutcome, GovernorEvidence as EvidenceValue, HostStopWatch, NativeSparqlEngine,
    PartialAnswers as PartialValue, QueryGovernors, ResourceDimension, StopSignal,
    TrippedGovernor as TrippedValue, WallDeadline,
};
use purrdf_sparql_results::{
    ResultProvenance, SparqlResultsFormat, serialize as serialize_results,
};
use purrdf_validate::governors::{
    GovernorParts, GovernorPartsError, from_parts, from_update_parts,
};
use wasm_bindgen::prelude::*;

use crate::async_query::AsyncOperationKind;
use crate::codec::resolve_format;
use crate::convert::BlankScopeMode;
use crate::dataset::{Dataset, serialize_frozen_with_options};
use crate::jsonld::{CompiledJsonLdContext, context_options, decode_options};
use crate::operation::{
    ClosureInputs, JobOutcome, OPTIONS_CODE, OperationInput, coded_error, diagnostic_to_js,
};
use crate::term::Term;

/// The typed result kind exposed to the package-root JavaScript wrapper.
#[derive(Debug, Clone, Copy)]
enum QueryResultKind {
    Select,
    Ask,
    Graph,
}

impl QueryResultKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Select => "select",
            Self::Ask => "ask",
            Self::Graph => "graph",
        }
    }
}

/// One SELECT binding row.
#[wasm_bindgen]
#[derive(Debug)]
pub struct SelectRow {
    variables: Rc<[String]>,
    values: Vec<Option<Term>>,
}

#[wasm_bindgen]
impl SelectRow {
    /// Variables projected by this row, in SELECT projection order.
    #[wasm_bindgen(getter)]
    pub fn variables(&self) -> Vec<String> {
        self.variables.iter().cloned().collect()
    }

    /// Return the bound term for a variable name, or `undefined` for unbound/absent.
    pub fn get(&self, variable: &str) -> Option<Term> {
        self.variables
            .iter()
            .position(|v| v == variable)
            .and_then(|i| self.values.get(i))
            .cloned()
            .flatten()
    }

    /// Move one value out by projection index, or return `undefined` when the
    /// cell is unbound, absent, or was already consumed.
    #[wasm_bindgen(js_name = takeValue)]
    pub fn take_value(&mut self, index: usize) -> Option<Term> {
        self.values.get_mut(index)?.take()
    }
}

/// A typed SELECT result returned by the raw wasm binding.
#[wasm_bindgen]
#[derive(Debug)]
pub struct SelectResult {
    variables: Rc<[String]>,
    rows: Vec<Option<SelectRow>>,
    next: usize,
    remaining: usize,
}

#[wasm_bindgen]
impl SelectResult {
    /// The result discriminator.
    #[wasm_bindgen(getter)]
    pub fn kind(&self) -> String {
        QueryResultKind::Select.as_str().to_owned()
    }

    /// Projected variables, in SELECT projection order.
    #[wasm_bindgen(getter)]
    pub fn variables(&self) -> Vec<String> {
        self.variables.iter().cloned().collect()
    }

    /// Total number of SELECT rows, including rows already consumed.
    #[wasm_bindgen(getter = rowCount)]
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Number of rows that have not yet been consumed.
    #[wasm_bindgen(getter)]
    pub fn remaining(&self) -> usize {
        self.remaining
    }

    /// Move a row out by result index. Each row can be consumed once.
    #[wasm_bindgen(js_name = takeRow)]
    pub fn take_row(&mut self, index: usize) -> Option<SelectRow> {
        let row = self.rows.get_mut(index)?.take()?;
        self.remaining -= 1;
        Some(row)
    }

    /// Move the next unconsumed row out of the result.
    #[wasm_bindgen(js_name = nextRow)]
    pub fn next_row(&mut self) -> Option<SelectRow> {
        while self.next < self.rows.len() {
            let index = self.next;
            self.next += 1;
            if let Some(row) = self.take_row(index) {
                return Some(row);
            }
        }
        None
    }
}

#[derive(Debug)]
enum QueryResultValue {
    Select(SelectResult),
    Ask(bool),
    Graph(Box<Dataset>),
}

/// A typed SPARQL result returned by the raw wasm binding.
#[wasm_bindgen]
#[derive(Debug)]
pub struct QueryResult {
    kind: QueryResultKind,
    value: Option<QueryResultValue>,
}

#[wasm_bindgen]
impl QueryResult {
    /// The result discriminator: `"select"`, `"ask"`, or `"graph"`.
    #[wasm_bindgen(getter)]
    pub fn kind(&self) -> String {
        self.kind.as_str().to_owned()
    }

    /// The ASK boolean when `kind === "ask"`, otherwise `undefined`.
    #[wasm_bindgen(getter)]
    pub fn boolean(&self) -> Option<bool> {
        match self.value {
            Some(QueryResultValue::Ask(value)) => Some(value),
            _ => None,
        }
    }

    /// Move the SELECT result out of this wrapper.
    #[wasm_bindgen(js_name = takeSelect)]
    pub fn take_select(&mut self) -> Option<SelectResult> {
        let value = self.value.take()?;
        match value {
            QueryResultValue::Select(result) => Some(result),
            other => {
                self.value = Some(other);
                None
            }
        }
    }

    /// Move the graph dataset out of this wrapper.
    #[wasm_bindgen(js_name = takeDataset)]
    pub fn take_dataset(&mut self) -> Option<Dataset> {
        let value = self.value.take()?;
        match value {
            QueryResultValue::Graph(dataset) => Some(*dataset),
            other => {
                self.value = Some(other);
                None
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The additive provenance extension: read-back
// ---------------------------------------------------------------------------

/// The `queryHash`/`engine` halves of a decoded provenance extension — the inverse of
/// `queryRaw`'s `provenanceNamespace` option. `undefined` in either slot means the
/// document carried no member under `prefix`, or the member omitted that field.
///
/// Per-solution source provenance (`ResultProvenance::solutions`) is not exposed here:
/// no writer on this surface (nor `purrdf_validate::query::provenance`, which every host
/// emits through) populates it — it is the evaluator/S11 derivation graph's progressive fill
/// (see `purrdf_sparql_results::ResultProvenance`'s module docs) — so there is nothing
/// yet for this binding to round-trip beyond `queryHash`/`engine`.
#[wasm_bindgen]
#[derive(Debug)]
pub struct ProvenanceInfo {
    query_hash: Option<String>,
    engine: Option<String>,
}

#[wasm_bindgen]
impl ProvenanceInfo {
    /// The decoded `queryHash`, or `undefined` if absent.
    #[wasm_bindgen(getter, js_name = queryHash)]
    #[must_use]
    pub fn query_hash(&self) -> Option<String> {
        self.query_hash.clone()
    }

    /// The decoded `engine` label, or `undefined` if absent.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn engine(&self) -> Option<String> {
        self.engine.clone()
    }
}

impl From<ResultProvenance> for ProvenanceInfo {
    fn from(provenance: ResultProvenance) -> Self {
        Self {
            query_hash: provenance.query_hash,
            engine: provenance.engine,
        }
    }
}

/// Decode the additive `purrdf` provenance extension a SPARQL-results JSON document
/// carries under the namespace `prefix`/`iri`, the inverse of `queryRaw`'s
/// `provenanceNamespace` option. A document with no member under `prefix` (never
/// written, or written under a different namespace) decodes to an empty
/// [`ProvenanceInfo`] rather than erroring.
///
/// # Errors
///
/// An invalid `prefix`/`iri`, malformed JSON, or a member under `prefix` whose shape
/// does not match the writer's.
#[wasm_bindgen(js_name = provenanceFromJson)]
pub fn provenance_from_json(
    json: &str,
    prefix: &str,
    iri: &str,
) -> Result<ProvenanceInfo, JsError> {
    let namespace = purrdf_sparql_results::ProvenanceNamespace::new(prefix, iri)
        .map_err(|e| JsError::new(&e.to_string()))?;
    let provenance = purrdf_sparql_results::provenance_from_json(json.as_bytes(), &namespace)
        .map_err(|e| JsError::new(&e.to_string()))?;
    Ok(provenance.into())
}

/// Decode the additive `purrdf` provenance extension a SPARQL-results XML document
/// carries under the namespace `prefix`/`iri` — the XML twin of [`provenance_from_json`].
///
/// # Errors
///
/// An invalid `prefix`/`iri`, malformed XML, or a non-`<sparql>` root.
#[wasm_bindgen(js_name = provenanceFromXml)]
pub fn provenance_from_xml(xml: &str, prefix: &str, iri: &str) -> Result<ProvenanceInfo, JsError> {
    let namespace = purrdf_sparql_results::ProvenanceNamespace::new(prefix, iri)
        .map_err(|e| JsError::new(&e.to_string()))?;
    let provenance = purrdf_sparql_results::provenance_from_xml(xml.as_bytes(), &namespace)
        .map_err(|e| JsError::new(&e.to_string()))?;
    Ok(provenance.into())
}

// ---------------------------------------------------------------------------
// The governed lane: configuration
// ---------------------------------------------------------------------------

/// The kernel's governed resource dimensions, by their stable kebab-case labels, in the
/// kernel's own declaration order.
///
/// This order is the index order of the [`GovernorEvidence`] consumption and ceiling
/// vectors, so a consumer zips the three together rather than hard-coding a vocabulary the
/// engine owns. Exposed as a function rather than restated in JavaScript for exactly that
/// reason: a dimension added to the kernel appears here without the package root being
/// edited, and cannot silently drop out of a caller's evidence map.
#[wasm_bindgen(js_name = governorDimensions)]
#[must_use]
pub fn governor_dimensions() -> Vec<String> {
    ResourceDimension::ALL
        .into_iter()
        .map(|dimension| dimension.label().to_owned())
        .collect()
}

/// A cancellation bit a page can flip, shared with every governed call it is handed to.
///
/// Latching is by construction: the bit only ever moves from clear to set, and nothing
/// clears it. Build a fresh token per query rather than reusing one.
///
/// # Cancelling a *running* wasm call
///
/// A synchronous governed call holds the thread for its whole duration, so a token
/// flipped on that same thread cannot run until the call returns and is never observed
/// by it. For a synchronous call the token is therefore for the two shapes that
/// genuinely work: cancelling *before* the call (a queued query the user has since
/// navigated away from), and cancelling a worker's query from another thread when the
/// token is shared through a `SharedArrayBuffer`-backed worker split. Both observe the
/// same latching bit, and both report the same `"cancelled"` trip.
///
/// The asynchronous lane (`crate::async_query`) is different: its jobs give the event
/// loop back at every yield and every host effect, so the page's own code does run
/// mid-query. It is cancelled through the job itself (the package root wires an
/// `AbortSignal` to it) and observes the cancellation at the next yield or effect — a
/// token is neither needed nor accepted there.
#[wasm_bindgen]
#[derive(Debug, Default)]
pub struct CancellationToken {
    /// The shared monotone bit the evaluator polls.
    flag: CancellationFlag,
}

#[wasm_bindgen]
impl CancellationToken {
    /// A fresh, uncancelled token.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Cancel every governed call holding this token. Idempotent, and never reversible.
    pub fn cancel(&self) {
        self.flag.cancel();
    }

    /// Whether this token has been cancelled.
    #[wasm_bindgen(getter, js_name = isCancelled)]
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.flag.is_cancelled()
    }

    /// A second handle onto the **same** bit — not a copy of it.
    ///
    /// Cancelling either handle cancels both, and neither can un-cancel. This exists
    /// because a governed call **consumes** the token handle it is given: wasm-bindgen
    /// moves an owned exported value across the boundary, which invalidates the JavaScript
    /// object. The package root therefore hands the engine a share and keeps the caller's
    /// own token alive, so one token governs a whole sequence of calls. The alternative —
    /// a token that silently stops working after its first use — is a cancellation the
    /// caller believes they still hold.
    #[wasm_bindgen(js_name = share)]
    #[must_use]
    pub fn share(&self) -> Self {
        Self {
            flag: self.flag.clone(),
        }
    }
}

/// The ceilings one governed call carries, after decoding and before they are engaged.
///
/// `None` in a resource slot preserves the metered baseline without a reachable cap.
/// Zero is a valid ceiling that trips on the first charged unit of work. Explicit
/// `no_ceiling` removes resource caps and accounting while retaining stop signals.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct GovernorArgs {
    /// Explicitly remove resource ceilings and accounting while retaining stop signals.
    no_ceiling: bool,
    /// Abstract execution steps.
    fuel: Option<u64>,
    /// Wall-clock budget in milliseconds. Zero expires on the first poll.
    deadline_ms: Option<u64>,
    /// Units committed to the answer sequence: solution rows for `SELECT`, output
    /// statements for `CONSTRUCT`/`DESCRIBE`. Inclusive, and nothing for `ASK`.
    max_answers: Option<u64>,
    /// The largest intermediate bag, in cells (`rows * columns`).
    max_intermediate_cells: Option<u64>,
    /// Bytes minted into the per-query scratch arena by value-constructing operations.
    max_scratch_bytes: Option<u64>,
    /// Requests issued to remote or federated endpoints.
    max_remote_requests: Option<u64>,
}

impl GovernorArgs {
    /// Decode every ceiling from the JavaScript boundary, in the order the governed
    /// entries name them. The one decoding path for the synchronous and asynchronous
    /// lanes alike, so a ceiling cannot mean one thing on one lane and another on the
    /// other.
    ///
    /// # Errors
    ///
    /// The first negative ceiling, in [`decode_ceiling_message`]'s words.
    pub(crate) fn decode(
        fuel: Option<i64>,
        deadline_ms: Option<i64>,
        max_answers: Option<i64>,
        max_intermediate_cells: Option<i64>,
        max_scratch_bytes: Option<i64>,
        max_remote_requests: Option<i64>,
    ) -> Result<Self, String> {
        Ok(Self {
            no_ceiling: false,
            fuel: decode_ceiling_message("fuel", fuel)?,
            deadline_ms: decode_ceiling_message("deadlineMs", deadline_ms)?,
            max_answers: decode_ceiling_message("maxAnswers", max_answers)?,
            max_intermediate_cells: decode_ceiling_message(
                "maxIntermediateCells",
                max_intermediate_cells,
            )?,
            max_scratch_bytes: decode_ceiling_message("maxScratchBytes", max_scratch_bytes)?,
            max_remote_requests: decode_ceiling_message("maxRemoteRequests", max_remote_requests)?,
        })
    }

    /// Select the shared decoder's explicit no-ceiling mode.
    pub(crate) const fn with_no_ceiling(mut self, no_ceiling: bool) -> Self {
        self.no_ceiling = no_ceiling;
        self
    }

    /// The caller's wall-clock budget, for a stop signal that is not a [`HostStopWatch`]
    /// to arm itself from.
    pub(crate) const fn deadline_ms(&self) -> Option<u64> {
        self.deadline_ms
    }

    /// These ceilings as the shared decoder's [`GovernorParts`]: a governed call from this
    /// package exposes an explicit no-ceiling choice.
    const fn parts(self) -> GovernorParts {
        GovernorParts {
            fuel: self.fuel,
            max_answers: self.max_answers,
            max_intermediate_cells: self.max_intermediate_cells,
            max_scratch_bytes: self.max_scratch_bytes,
            max_remote_requests: self.max_remote_requests,
            no_ceiling: self.no_ceiling,
        }
    }

    /// These ceilings over the `METERED` base, with no stop signal attached yet, through
    /// [`purrdf_validate::governors::from_parts`].
    ///
    /// The one construction path for a query's ceilings: the synchronous lane attaches its
    /// [`HostStopWatch`] to it, and the asynchronous lane attaches its own watch instead.
    /// See [`Self::stop_watch`] for why the base is `METERED`.
    ///
    /// # Errors
    ///
    /// The parts' refusal, in [`governor_parts_message`]'s words.
    pub(crate) fn ceilings(self) -> Result<QueryGovernors, String> {
        from_parts(&self.parts(), None).map_err(governor_parts_message)
    }

    /// [`Self::ceilings`] for a governed UPDATE, through
    /// [`purrdf_validate::governors::from_update_parts`], which refuses `maxAnswers`.
    ///
    /// # Errors
    ///
    /// [`UPDATE_REFUSES_MAX_ANSWERS`] for an answer cap.
    pub(crate) fn update_ceilings(self) -> Result<QueryGovernors, String> {
        from_update_parts(&self.parts(), None).map_err(governor_parts_message)
    }

    /// The stop signal a synchronous governed call runs under: the caller's wall deadline
    /// and cancellation token composed into one [`HostStopWatch`], or `None` when the
    /// caller supplied neither.
    ///
    /// # Why a governed call's base is `METERED` rather than `UNBOUNDED`
    ///
    /// Two reasons, and both are about what a governed call promises. First, every outcome
    /// — including a complete one — carries evidence a caller can size the next budget
    /// from; `UNBOUNDED` reports nothing, because it charges nothing. Second, the evaluator
    /// polls the stop signal every `STOP_POLL_FUEL` units of fuel *and* at each algebra
    /// node it enters; with fuel disengaged only the second of those runs, so a query
    /// spending a long time inside one operator would notice a deadline or a cancellation
    /// late. Metering costs a saturating add per charge point and buys prompt interruption
    /// on every query shape, which is the trade a caller who asked for governors has
    /// already chosen. The **ungoverned** entries (`query`, `select`, `update`, …) are
    /// untouched by any of this: they take the engine's ungoverned entry, install no
    /// governor state and charge nothing at all.
    pub(crate) fn stop_watch(
        self,
        cancel: Option<&CancellationToken>,
    ) -> Option<Arc<dyn StopSignal>> {
        HostStopWatch::new(
            cancel.map(|token| token.flag.clone()),
            self.deadline_ms
                .map(|ms| WallDeadline::after(Duration::from_millis(ms))),
        )
        .into_signal()
    }
}

/// Why an UPDATE refuses `maxAnswers`, shared verbatim by the synchronous and
/// asynchronous governed UPDATE entries.
pub(crate) const UPDATE_REFUSES_MAX_ANSWERS: &str = "maxAnswers is not accepted by updateGoverned: an UPDATE has no answer \
     sequence to bound. Bound the work that computes the request with fuel, \
     maxIntermediateCells, or maxScratchBytes instead";

/// This package's wording of a [`GovernorPartsError`], in its option spelling.
fn governor_parts_message(error: GovernorPartsError) -> String {
    match error {
        GovernorPartsError::AnswerCapOnUpdate => UPDATE_REFUSES_MAX_ANSWERS.to_owned(),
        other => other.to_string(),
    }
}

/// Decode one ceiling from the JavaScript boundary.
///
/// Returns a plain `String` error (not a `JsError`) so the refusal is unit-testable on
/// the native build, where constructing a `JsError` panics.
///
/// The boundary type is a signed 64-bit integer (a JS `bigint`) rather than a `u64`
/// precisely so that a negative can be *seen* and refused here. Taking a `u64` would make
/// `{ fuel: -1 }` arrive as `u64::MAX` — a governor the caller believes they set and that
/// nothing can ever trip, which is the silent-hole failure this whole surface exists to
/// close. `undefined` is the only way to decline a dimension.
///
/// # Errors
///
/// A negative ceiling, which is not a smaller budget but an unrepresentable one.
fn decode_ceiling_message(name: &str, value: Option<i64>) -> Result<Option<u64>, String> {
    match value {
        None => Ok(None),
        Some(raw) if raw < 0 => Err(format!(
            "governor ceiling `{name}` must be a non-negative integer, got {raw} \
             (omit it to decline the ceiling; 0 is a valid ceiling that trips on the \
             first charged unit of work)"
        )),
        Some(raw) => Ok(Some(raw.unsigned_abs())),
    }
}

/// [`GovernorArgs::decode`] at the synchronous `#[wasm_bindgen]` boundary.
fn decode_governor_args(
    fuel: Option<i64>,
    deadline_ms: Option<i64>,
    max_answers: Option<i64>,
    max_intermediate_cells: Option<i64>,
    max_scratch_bytes: Option<i64>,
    max_remote_requests: Option<i64>,
) -> Result<GovernorArgs, JsError> {
    GovernorArgs::decode(
        fuel,
        deadline_ms,
        max_answers,
        max_intermediate_cells,
        max_scratch_bytes,
        max_remote_requests,
    )
    .map_err(|message| JsError::new(&message))
}

// ---------------------------------------------------------------------------
// The governed lane: outcomes
// ---------------------------------------------------------------------------

/// The governor that stopped one execution: which one, on which dimension, against which
/// ceiling.
#[wasm_bindgen]
#[derive(Debug)]
pub struct TrippedGovernor {
    /// The kernel value this object renders.
    inner: TrippedValue,
}

#[wasm_bindgen]
impl TrippedGovernor {
    /// Which kind of governor stopped the execution: `"budget"` (a ceiling was reached),
    /// `"stopped"` (a stop signal fired), or `"refused"` (the planner's estimate already
    /// exceeded a ceiling, so nothing ran).
    ///
    /// # The wildcard arm, here and on every accessor below
    ///
    /// The kernel's `TrippedGovernor` is `#[non_exhaustive]`, so this crate — foreign to
    /// the one that defines it — must carry a wildcard even though the enum is exhaustive
    /// today. A governor a future kernel adds and this build cannot name therefore reads
    /// `"unknown"` here and `undefined` on every field accessor, rather than being
    /// silently folded into a kind it is not. `label` and `message` still describe it
    /// exactly, because both come from the kernel rather than from this match.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn kind(&self) -> String {
        match self.inner {
            TrippedValue::Budget { .. } => "budget",
            TrippedValue::Stopped { .. } => "stopped",
            TrippedValue::Refused { .. } => "refused",
            _ => "unknown",
        }
        .to_owned()
    }

    /// The stable kebab-case discriminant, e.g. `"deadline-exceeded"`. A pinned contract:
    /// match on this rather than on the prose of `message`.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn label(&self) -> String {
        self.inner.label().to_owned()
    }

    /// The governed dimension, e.g. `"fuel"` — `undefined` when a stop signal fired, which
    /// belongs to no dimension.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn dimension(&self) -> Option<String> {
        match self.inner {
            TrippedValue::Budget { dimension, .. } | TrippedValue::Refused { dimension, .. } => {
                Some(dimension.label().to_owned())
            }
            _ => None,
        }
    }

    /// The inclusive ceiling in force, or `undefined` when a stop signal fired.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn limit(&self) -> Option<u64> {
        match self.inner {
            TrippedValue::Budget { limit, .. } | TrippedValue::Refused { limit, .. } => Some(limit),
            _ => None,
        }
    }

    /// Consumption charged before the refused work — a **measurement**, and present only
    /// on the `"budget"` kind.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn consumed(&self) -> Option<u64> {
        match self.inner {
            TrippedValue::Budget { consumed, .. } => Some(consumed),
            _ => None,
        }
    }

    /// The planner's estimate that exceeded the ceiling — **not** a measurement, and
    /// present only on the `"refused"` kind, where nothing ran to measure.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn estimate(&self) -> Option<u64> {
        match self.inner {
            TrippedValue::Refused { estimate, .. } => Some(estimate),
            _ => None,
        }
    }

    /// Which stop signal fired — `"cancelled"` or `"deadline-exceeded"` — or `undefined`
    /// when a ceiling rather than a signal stopped the execution.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn cause(&self) -> Option<String> {
        match self.inner {
            TrippedValue::Stopped { .. } => Some(self.inner.label().to_owned()),
            _ => None,
        }
    }

    /// A human-readable rendering. Prose, not a contract — match on `label`.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }
}

/// One governed execution's receipt: what it was allowed and what it spent.
///
/// Returned on the complete path as well as the exhausted one — "completed, cost N fuel,
/// peak M cells" is how a caller sizes the next call's budget in the first place.
///
/// [`Self::consumed`] and [`Self::limits`] are positional vectors indexed by
/// [`governor_dimensions`], not maps, because the wasm boundary has no exact map of 64-bit
/// integers to hand over: a JSON object would round `2**64 - 2` — the ceiling a metered but
/// unbounded dimension actually carries — to something it is not.
#[wasm_bindgen]
#[derive(Debug)]
pub struct GovernorEvidence {
    /// The kernel value this object renders.
    inner: EvidenceValue,
}

#[wasm_bindgen]
impl GovernorEvidence {
    /// Consumption charged per dimension, positionally by [`governor_dimensions`].
    ///
    /// A peak-tracked dimension (`intermediate-cells`, `udf-depth`) reports the largest
    /// single observation; every other dimension reports the running sum.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn consumed(&self) -> Vec<u64> {
        ResourceDimension::ALL
            .into_iter()
            .map(|dimension| self.inner.consumed_in(dimension))
            .collect()
    }

    /// The inclusive ceilings in force, positionally by [`governor_dimensions`].
    ///
    /// A governed call meters every caller-settable dimension, so a dimension the caller
    /// declined reads `2**64 - 2` — engaged, at a ceiling no execution can reach.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn limits(&self) -> Vec<u64> {
        ResourceDimension::ALL
            .into_iter()
            .map(|dimension| self.inner.limit_for(dimension))
            .collect()
    }

    /// Whether the execution completed with every governor intact.
    ///
    /// The governor itself is on the outcome rather than duplicated here: one trip, one
    /// object, so a consumer cannot read two and wonder which is authoritative.
    #[wasm_bindgen(getter, js_name = isComplete)]
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.inner.is_complete()
    }

    /// Every invocation a `SERVICE SILENT` or `LOAD SILENT` absorbed: the answer the
    /// specification requires for it is indistinguishable from an endpoint with nothing
    /// to add, and this is where the difference is kept. Empty when nothing failed.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn silenced(&self) -> Vec<SilencedInvocation> {
        silenced_records(self.inner.silenced())
    }

    /// Every XPath F&O numeric error the execution absorbed into an unbound value (an
    /// expression error is not a query error, SPARQL 1.1 §17.2), counted positionally by
    /// [`expression_error_codes`]: `1/0` counts under `err:FOAR0001`. All zero when no
    /// expression failed.
    #[wasm_bindgen(getter, js_name = expressionErrors)]
    #[must_use]
    pub fn expression_errors(&self) -> Vec<u64> {
        expression_error_counts(self.inner.expression_errors())
    }
}

/// The XPath F&O error codes, `err:`-prefixed (`"err:FOAR0001"`), in the order the
/// evidence's `expressionErrors` vectors are indexed by.
#[wasm_bindgen(js_name = expressionErrorCodes)]
#[must_use]
pub fn expression_error_codes() -> Vec<String> {
    purrdf::xsd::ErrorCode::ALL
        .into_iter()
        .map(|code| code.qname().to_owned())
        .collect()
}

/// `counts` (per code, in code order, absent when zero) as a vector positional by
/// [`expression_error_codes`].
pub(crate) fn expression_error_counts(counts: &[(purrdf::xsd::ErrorCode, u64)]) -> Vec<u64> {
    purrdf::xsd::ErrorCode::ALL
        .into_iter()
        .map(|code| {
            counts
                .iter()
                .filter(|(counted, _)| *counted == code)
                .map(|(_, count)| *count)
                .sum()
        })
        .collect()
}

/// One invocation a `SILENT` clause absorbed: a `SERVICE SILENT` that answered the single
/// empty solution, or a `LOAD SILENT` that succeeded with nothing loaded.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct SilencedInvocation {
    /// The kernel record this object renders.
    inner: purrdf_core::SilencedInvocation,
}

#[wasm_bindgen]
impl SilencedInvocation {
    /// `"service"` or `"load"`.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn target(&self) -> String {
        self.inner.target.label().to_owned()
    }

    /// The `SERVICE` endpoint, or `undefined` for a `LOAD`. A variable endpoint bound to
    /// a term that is not an IRI reports that term in N-Triples form.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn endpoint(&self) -> Option<String> {
        match &self.inner.target {
            purrdf_core::SilencedTarget::Service { endpoint } => Some(endpoint.clone()),
            _ => None,
        }
    }

    /// The `LOAD` source IRI, or `undefined` for a `SERVICE`.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn iri(&self) -> Option<String> {
        match &self.inner.target {
            purrdf_core::SilencedTarget::Load { iri } => Some(iri.clone()),
            _ => None,
        }
    }

    /// Why the invocation failed: `"transport"`, `"decode"`, `"disabled"`,
    /// `"unconfigured"`, `"denied"`, `"host-denied"`, `"not-an-iri"` or `"fault"`.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn kind(&self) -> String {
        self.inner.kind.label().to_owned()
    }

    /// The failure's message, as the error would have read without `SILENT`.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn message(&self) -> String {
        self.inner.message.clone()
    }
}

/// The JavaScript objects for `records`, in their order.
pub(crate) fn silenced_records(
    records: &[purrdf_core::SilencedInvocation],
) -> Vec<SilencedInvocation> {
    records
        .iter()
        .map(|record| SilencedInvocation {
            inner: record.clone(),
        })
        .collect()
}

/// What the rows a truncated execution reached bound, relative to the query's true answer.
///
/// A three-way interval, not a yes/no: `"certain"` rows are a certified **lower** bound and
/// are safe to admit as answers; `"at-most"` rows are a certified **upper** bound and are
/// sound only for the negative reading (a row absent from them is definitively not an
/// answer); `"unknown"` means neither bound survived, so **no row is handed over at all**
/// and [`Self::barrier`] names the operator that withheld them instead.
#[wasm_bindgen]
#[derive(Debug)]
pub struct PartialAnswers {
    /// `"certain"`, `"at-most"`, or `"unknown"`.
    certainty: &'static str,
    /// The materialized rows, absent on the `"unknown"` class.
    result: Option<QueryResult>,
    /// Whether those rows are the true answer's first rows, in order.
    positional_prefix: Option<bool>,
    /// The operator that withheld the rows, on the `"unknown"` class.
    barrier: Option<String>,
}

#[wasm_bindgen]
impl PartialAnswers {
    /// What these rows certify: `"certain"`, `"at-most"`, or `"unknown"`.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn certainty(&self) -> String {
        self.certainty.to_owned()
    }

    /// Whether these rows are certified answers — i.e. whether they may be admitted.
    #[wasm_bindgen(getter, js_name = isCertain)]
    #[must_use]
    pub fn is_certain(&self) -> bool {
        self.certainty == "certain"
    }

    /// Move the rows in hand out of this certificate, or `undefined` on the `"unknown"`
    /// class, where rows that bound the answer on neither side offer no sound use and one
    /// unsound one.
    #[wasm_bindgen(js_name = takeResult)]
    pub fn take_result(&mut self) -> Option<QueryResult> {
        self.result.take()
    }

    /// Whether these rows are the true answer's **first** rows, in order. This licenses
    /// resumption by raising a deterministic ceiling; a wall-deadline rerun is fresh and
    /// may stop sooner. `undefined` on the `"unknown"` class.
    #[wasm_bindgen(getter, js_name = isPositionalPrefix)]
    #[must_use]
    pub fn is_positional_prefix(&self) -> Option<bool> {
        self.positional_prefix
    }

    /// The algebra operator that withheld the rows, on the `"unknown"` class — which is
    /// what says whether a larger budget or a different query is the way forward.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn barrier(&self) -> Option<String> {
        self.barrier.clone()
    }
}

/// The outcome of one governed query: a complete answer, or an exhausted budget carrying
/// the partial answers the execution actually reached.
///
/// Exactly two shapes, and **neither is a thrown error**. Check `isComplete`, take
/// `takeResult()` when it holds, and take `takeTripped()` with `takePartial()` when it
/// does not. Every accessor that hands over a wasm-owned object moves it, so each can be
/// taken once.
#[wasm_bindgen]
#[derive(Debug)]
pub struct QueryOutcome {
    /// The complete result, present on the complete path only.
    result: Option<QueryResult>,
    /// What the rows in hand bound, present on the exhausted path only.
    partial: Option<PartialAnswers>,
    /// The governor that stopped the execution, present on the exhausted path only.
    tripped: Option<TrippedGovernor>,
    /// This execution's consumption and ceilings, present on both paths.
    evidence: Option<GovernorEvidence>,
    /// Whether the execution completed, latched at construction so it survives the moves
    /// above.
    complete: bool,
}

#[wasm_bindgen]
impl QueryOutcome {
    /// Whether every governor stayed intact and this is the query's complete answer.
    #[wasm_bindgen(getter, js_name = isComplete)]
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.complete
    }

    /// Move the **complete** result out, or `undefined` when a governor stopped the
    /// execution.
    ///
    /// Deliberately never the partial rows: a caller that stopped reading the outcome one
    /// level too early receives nothing rather than a truncated answer wearing a complete
    /// answer's type. The rows a trip reached are behind `takePartial()`, with the
    /// certificate that says what they bound.
    #[wasm_bindgen(js_name = takeResult)]
    pub fn take_result(&mut self) -> Option<QueryResult> {
        self.result.take()
    }

    /// Move the partial-answer certificate out, or `undefined` when the query completed.
    #[wasm_bindgen(js_name = takePartial)]
    pub fn take_partial(&mut self) -> Option<PartialAnswers> {
        self.partial.take()
    }

    /// Move the tripped governor out, or `undefined` when the query completed.
    #[wasm_bindgen(js_name = takeTripped)]
    pub fn take_tripped(&mut self) -> Option<TrippedGovernor> {
        self.tripped.take()
    }

    /// Move this execution's receipt out. Present on both paths.
    #[wasm_bindgen(js_name = takeEvidence)]
    pub fn take_evidence(&mut self) -> Option<GovernorEvidence> {
        self.evidence.take()
    }
}

/// The outcome of one governed query whose complete answer was serialized in the format
/// negotiated from an `Accept` header (`queryGovernedNegotiatedAsync`).
///
/// The same two shapes as [`QueryOutcome`], with the complete answer as a document rather
/// than a typed result: `takeBody()` with its `format` and `mediaType` when every governor
/// held, `takePartial()` with `takeTripped()` when one did not. The partial rows stay
/// typed — they are a certificate to inspect, not a document to send.
#[wasm_bindgen]
#[derive(Debug)]
pub struct NegotiatedOutcome {
    /// The serialized complete answer, present on the complete path only.
    body: Option<Vec<u8>>,
    /// The negotiated format token, present on the complete path only.
    format: Option<&'static str>,
    /// What the rows in hand bound, present on the exhausted path only.
    partial: Option<PartialAnswers>,
    /// The governor that stopped the execution, present on the exhausted path only.
    tripped: Option<TrippedGovernor>,
    /// This execution's consumption and ceilings, present on both paths.
    evidence: Option<GovernorEvidence>,
    /// Whether the execution completed, latched at construction.
    complete: bool,
}

#[wasm_bindgen]
impl NegotiatedOutcome {
    /// Whether every governor stayed intact and the body is the query's complete answer.
    #[wasm_bindgen(getter, js_name = isComplete)]
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.complete
    }

    /// The negotiated format token (`json`, `xml`, `csv`, `tsv`, `turtle`, `trig`,
    /// `ntriples`, `nquads` or `jsonld`), or `undefined` when a governor stopped the
    /// execution.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn format(&self) -> Option<String> {
        self.format.map(str::to_owned)
    }

    /// The body's media type, for the response's `Content-Type`, or `undefined` when a
    /// governor stopped the execution.
    #[wasm_bindgen(getter, js_name = mediaType)]
    #[must_use]
    pub fn media_type(&self) -> Option<String> {
        self.format
            .and_then(purrdf_sparql_eval::protocol::format_media_type)
            .map(str::to_owned)
    }

    /// Move the complete answer's UTF-8 bytes out, or `undefined` when a governor stopped
    /// the execution.
    #[wasm_bindgen(js_name = takeBody)]
    pub fn take_body(&mut self) -> Option<Vec<u8>> {
        self.body.take()
    }

    /// Move the partial-answer certificate out, or `undefined` when the query completed.
    #[wasm_bindgen(js_name = takePartial)]
    pub fn take_partial(&mut self) -> Option<PartialAnswers> {
        self.partial.take()
    }

    /// Move the tripped governor out, or `undefined` when the query completed.
    #[wasm_bindgen(js_name = takeTripped)]
    pub fn take_tripped(&mut self) -> Option<TrippedGovernor> {
        self.tripped.take()
    }

    /// Move this execution's receipt out. Present on both paths.
    #[wasm_bindgen(js_name = takeEvidence)]
    pub fn take_evidence(&mut self) -> Option<GovernorEvidence> {
        self.evidence.take()
    }
}

/// What a negotiated governed query produced, before it crosses to JavaScript: the
/// complete answer already serialized, or the exhausted outcome untouched.
#[derive(Debug)]
pub(crate) enum NegotiatedValue {
    /// Every governor held; the answer is serialized in `format`.
    Complete {
        bytes: Vec<u8>,
        format: &'static str,
        evidence: EvidenceValue,
    },
    /// A governor stopped the execution.
    Exhausted(BudgetExhausted),
}

/// Convert a [`NegotiatedValue`] into the JS-facing [`NegotiatedOutcome`].
pub(crate) fn negotiated_outcome_from_value(
    value: NegotiatedValue,
    blank_scope: BlankScopeMode,
) -> Result<NegotiatedOutcome, JsError> {
    match value {
        NegotiatedValue::Complete {
            bytes,
            format,
            evidence,
        } => Ok(NegotiatedOutcome {
            body: Some(bytes),
            format: Some(format),
            partial: None,
            tripped: None,
            evidence: Some(GovernorEvidence { inner: evidence }),
            complete: true,
        }),
        NegotiatedValue::Exhausted(BudgetExhausted {
            tripped,
            evidence,
            partial,
            ..
        }) => Ok(NegotiatedOutcome {
            body: None,
            format: None,
            partial: Some(partial_answers_from_native(partial, blank_scope)?),
            tripped: Some(TrippedGovernor { inner: tripped }),
            evidence: Some(GovernorEvidence { inner: evidence }),
            complete: false,
        }),
    }
}

/// The protocol result kind of an evaluated result: solutions, a graph, or a graph that
/// carries named graphs — the shape that decides which formats can hold it.
pub(crate) fn negotiable_result_kind(
    result: &SparqlResult,
) -> purrdf_sparql_eval::protocol::ResultKind {
    use purrdf_sparql_eval::protocol::ResultKind;
    match result {
        SparqlResult::Graph(graph) if !distinct_graph_names(&**graph).is_empty() => {
            ResultKind::Dataset
        }
        SparqlResult::Graph(_) => ResultKind::Graph,
        SparqlResult::Solutions { .. } => ResultKind::Solutions,
        SparqlResult::Boolean(_) => ResultKind::Boolean,
    }
}

/// The two-phase outcome of a governed entailment-aware query.
///
/// `takeOutcome()` and `report` are present together only after closure completed. A
/// closure-phase stop carries neither, preventing a host from treating an incomplete
/// closure as queryable data or as a reasoning certificate.
#[wasm_bindgen]
#[derive(Debug)]
pub struct EntailmentQueryOutcome {
    outcome: Option<QueryOutcome>,
    report: Option<String>,
    tripped: Option<TrippedGovernor>,
    complete: bool,
    closure_stopped: bool,
}

#[wasm_bindgen]
impl EntailmentQueryOutcome {
    /// Whether both closure and query completed under every governor.
    #[wasm_bindgen(getter, js_name = isComplete)]
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.complete
    }

    /// Whether the stop happened before a closure existed.
    #[wasm_bindgen(getter, js_name = closureStopped)]
    #[must_use]
    pub fn closure_stopped(&self) -> bool {
        self.closure_stopped
    }

    /// Byte-stable reasoning report, absent when closure stopped.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn report(&self) -> Option<String> {
        self.report.clone()
    }

    /// Move phase two's governed query outcome out, or `undefined` if closure stopped.
    #[wasm_bindgen(js_name = takeOutcome)]
    pub fn take_outcome(&mut self) -> Option<QueryOutcome> {
        self.outcome.take()
    }

    /// Move the governor that stopped either phase out, or `undefined` on completion.
    #[wasm_bindgen(js_name = takeTripped)]
    pub fn take_tripped(&mut self) -> Option<TrippedGovernor> {
        self.tripped.take()
    }
}

/// The outcome of one governed SPARQL UPDATE.
///
/// Deliberately not a [`QueryOutcome`] and deliberately without a partial arm: a query's
/// partial answer is a certifiable thing, a partial *mutation* is not. A tripped request
/// applied **nothing** — not "not all of it" — and left the dataset exactly as it found it.
#[wasm_bindgen]
#[derive(Debug)]
pub struct UpdateOutcome {
    /// The governor that stopped the request, present on the exhausted path only.
    tripped: Option<TrippedGovernor>,
    /// This request's consumption and ceilings, present on both paths.
    evidence: Option<GovernorEvidence>,
    /// Whether every operation of the request applied, latched at construction.
    applied: bool,
}

#[wasm_bindgen]
impl UpdateOutcome {
    /// Whether every operation of the request applied.
    ///
    /// `false` means **nothing** applied, never "not all of it applied".
    #[wasm_bindgen(getter, js_name = isApplied)]
    #[must_use]
    pub fn is_applied(&self) -> bool {
        self.applied
    }

    /// Move the tripped governor out, or `undefined` when the request applied.
    #[wasm_bindgen(js_name = takeTripped)]
    pub fn take_tripped(&mut self) -> Option<TrippedGovernor> {
        self.tripped.take()
    }

    /// Move this request's receipt out. Present on both paths.
    #[wasm_bindgen(js_name = takeEvidence)]
    pub fn take_evidence(&mut self) -> Option<GovernorEvidence> {
        self.evidence.take()
    }
}

/// A reusable SPARQL engine that keeps the native plan cache alive across calls.
///
/// The engine is reference-counted so an asynchronous job can keep evaluating on it
/// after the JavaScript handle that started the job has been freed, and so the
/// synchronous methods can run on it while a job is suspended.
#[wasm_bindgen]
pub struct QueryEngine {
    inner: Rc<NativeSparqlEngine>,
    /// How a blank node's scope crosses to JS in the typed results this engine returns
    /// (`blankScope`). An asynchronous job takes the mode in force when it begins.
    blank_scope: Cell<BlankScopeMode>,
    /// The precision of every `xsd:integer`/`xsd:decimal` quotient this engine forms
    /// (`divisionPolicy`). An asynchronous job takes the policy in force when it begins.
    division: Cell<DivisionPolicy>,
}

impl Default for QueryEngine {
    fn default() -> Self {
        Self {
            inner: Rc::default(),
            blank_scope: Cell::default(),
            division: Cell::new(DivisionPolicy::xsd_default()),
        }
    }
}

impl std::fmt::Debug for QueryEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryEngine").finish_non_exhaustive()
    }
}

#[wasm_bindgen]
impl QueryEngine {
    /// How a blank node's scope crosses to JS in the typed results this engine returns:
    /// `"keep"` (the default) or `"merge"`.
    ///
    /// The engine holds a blank node as a label and a scope, so two nodes that share a
    /// label in different scopes are two nodes. Under `"keep"` a scoped blank crosses as
    /// its scope envelope and they stay two nodes in JS; an unscoped blank keeps its bare
    /// label. Under `"merge"` the scope is dropped and every blank crosses as its bare
    /// label, so nodes that share a label in different scopes are ONE node in JS. Set it
    /// before running a query: it applies to `query`, `select`, `queryGoverned` and
    /// `queryEntailmentGoverned`, and an asynchronous job takes the value in force when
    /// it begins. The serialized (`queryRaw`) results are unaffected.
    #[wasm_bindgen(getter = blankScope)]
    pub fn blank_scope_name(&self) -> String {
        self.blank_scope.get().name().to_owned()
    }

    /// Set [`Self::blank_scope_name`]. Throws on any value but `"keep"` or `"merge"`.
    ///
    /// # Errors
    ///
    /// A value that is neither `"keep"` nor `"merge"`; the mode is left as it was.
    #[wasm_bindgen(setter = blankScope)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn set_blank_scope(&self, mode: String) -> Result<(), JsValue> {
        let parsed =
            BlankScopeMode::parse(&mode).map_err(|message| coded_error(&message, OPTIONS_CODE))?;
        self.blank_scope.set(parsed);
        Ok(())
    }

    /// The precision every `xsd:integer`/`xsd:decimal` quotient (`/` and `AVG`) this
    /// engine forms is computed at, in its one text form: `"exact"`, or `"N:ROUNDING"`
    /// — `N` fractional digits rounded in the named direction. The default is
    /// `"18:toward-zero"`, XSD's eighteen fractional digits truncated toward zero.
    ///
    /// Under `"exact"` a quotient with no finite decimal expansion (`1/3`) is not
    /// rounded: it is a SPARQL expression error, unbound like `1/0` and counted as
    /// `err:FOAR0002` in a governed outcome's evidence, while a terminating one (`1/8`)
    /// answers exactly. The policy applies to every query,
    /// update, explain, governed, entailment, negotiated and serialized entry of this
    /// engine, and an asynchronous job takes the policy in force when it begins.
    /// `Dataset.query` and `Dataset.update` run on a fresh engine, so under the default.
    #[wasm_bindgen(getter = divisionPolicy)]
    pub fn division_policy(&self) -> String {
        self.division.get().to_string()
    }

    /// Set [`Self::division_policy`] from its text form: `"exact"`, `"N"` (`N`
    /// fractional digits truncated toward zero) or `"N:ROUNDING"`, with `ROUNDING` one
    /// of `toward-zero`, `away-from-zero`, `floor`, `ceiling`, `half-even`,
    /// `half-away-from-zero`, `half-toward-zero`, `half-ceiling` or `half-floor`.
    ///
    /// # Errors
    ///
    /// Text in none of those forms; the policy is left as it was.
    #[wasm_bindgen(setter = divisionPolicy)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn set_division_policy(&self, policy: String) -> Result<(), JsValue> {
        self.set_division(&policy)
            .map_err(|message| coded_error(&message, OPTIONS_CODE))
    }

    /// Create a reusable offline SPARQL engine.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Run any SPARQL query and return a typed raw wasm result wrapper.
    #[wasm_bindgen(js_name = query)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn query(
        &self,
        dataset: &Dataset,
        sparql: &str,
        base: Option<String>,
    ) -> Result<QueryResult, JsValue> {
        let result = self.run_query(dataset, sparql, base.as_deref())?;
        Ok(query_result_from_sparql(result, self.blank_scope())?)
    }

    /// Run a SELECT query and return typed rows.
    #[wasm_bindgen(js_name = select)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn select(
        &self,
        dataset: &Dataset,
        sparql: &str,
        base: Option<String>,
    ) -> Result<SelectResult, JsValue> {
        let result = self.run_query(dataset, sparql, base.as_deref())?;
        Ok(select_result_from_sparql(result, self.blank_scope())?)
    }

    /// Run an ASK query and return the boolean result.
    #[wasm_bindgen(js_name = ask)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn ask(
        &self,
        dataset: &Dataset,
        sparql: &str,
        base: Option<String>,
    ) -> Result<bool, JsValue> {
        match self.run_query(dataset, sparql, base.as_deref())? {
            SparqlResult::Boolean(value) => Ok(value),
            other => Err(kind_mismatch("ASK boolean", &other).into()),
        }
    }

    /// Run a CONSTRUCT query and return its result dataset.
    #[wasm_bindgen(js_name = construct)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn construct(
        &self,
        dataset: &Dataset,
        sparql: &str,
        base: Option<String>,
    ) -> Result<Dataset, JsValue> {
        Ok(graph_result_from_sparql(self.run_query(
            dataset,
            sparql,
            base.as_deref(),
        )?)?)
    }

    /// Run a DESCRIBE query and return its result dataset.
    #[wasm_bindgen(js_name = describe)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn describe(
        &self,
        dataset: &Dataset,
        sparql: &str,
        base: Option<String>,
    ) -> Result<Dataset, JsValue> {
        Ok(graph_result_from_sparql(self.run_query(
            dataset,
            sparql,
            base.as_deref(),
        )?)?)
    }

    /// Apply a SPARQL UPDATE atomically to the supplied dataset.
    #[wasm_bindgen(js_name = update)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn update(
        &self,
        dataset: &mut Dataset,
        sparql: &str,
        base: Option<String>,
    ) -> Result<(), JsValue> {
        let input = self.input(dataset, AsyncOperationKind::Update, sparql, base.as_deref())?;
        match input.run_offline(None)? {
            JobOutcome::Updated(frozen) => {
                dataset
                    .replace(frozen)
                    .map_err(|error| crate::dataset::diag_to_err(&error))?;
                Ok(())
            }
            other => Err(unexpected_outcome("update", &other)),
        }
    }

    /// Run any SPARQL query and serialize its raw result.
    ///
    /// `format` is optional: omitted, a SELECT/ASK result is SPARQL-Results JSON and a
    /// CONSTRUCT/DESCRIBE result is Turtle, or TriG when it carries a named graph
    /// (`default_graph_format`). Naming a single-graph RDF syntax for a
    /// graph-carrying result throws rather than emitting a document missing exactly
    /// what the query asked for (`refuse_uncarriable_named_graphs`).
    ///
    /// `provenance_prefix`/`provenance_iri` (both `undefined`, or both a string) anchor
    /// the additive `purrdf` provenance extension on a SELECT/ASK result serialized to
    /// SPARQL-results JSON/XML, under that `PREFIX`/`IRI`. `undefined` (the default)
    /// leaves the output pure W3C; a CONSTRUCT/DESCRIBE graph result and CSV/TSV never
    /// carry the extension. Read it back with `provenanceFromJson`/`provenanceFromXml`
    /// under the SAME namespace.
    ///
    /// # Errors
    ///
    /// A parse/evaluation failure, an unsupported format, or exactly one of
    /// `provenance_prefix`/`provenance_iri` supplied without the other.
    #[wasm_bindgen(js_name = queryRaw)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    #[allow(
        clippy::too_many_arguments,
        reason = "each parameter is a distinct, independently-named input at the wasm boundary"
    )]
    pub fn query_raw(
        &self,
        dataset: &Dataset,
        sparql: &str,
        base: Option<String>,
        format: Option<String>,
        provenance_prefix: Option<String>,
        provenance_iri: Option<String>,
    ) -> Result<String, JsValue> {
        let namespace = build_provenance_namespace(provenance_prefix, provenance_iri)?;
        let mut input = self.input(dataset, AsyncOperationKind::Raw, sparql, base.as_deref())?;
        input.format = format;
        input.provenance = namespace;
        raw_text(input.run_offline(None)?)
    }

    /// Serialize a CONSTRUCT/DESCRIBE result with configured JSON-LD/YAML-LD.
    #[wasm_bindgen(js_name = queryRawConfigured)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn query_raw_configured(
        &self,
        dataset: &Dataset,
        sparql: &str,
        base: Option<String>,
        format: &str,
        options_json: &str,
    ) -> Result<String, JsValue> {
        let options = decode_options(options_json)?;
        self.query_raw_with_options(
            dataset,
            AsyncOperationKind::Raw,
            sparql,
            base.as_deref(),
            format,
            options,
        )
    }

    /// Run a SPARQL query under caller-supplied execution governors, returning a
    /// [`QueryOutcome`] rather than the answers directly.
    ///
    /// Every ceiling is optional; `undefined` leaves that dimension metered without a
    /// reachable cap. `no_ceiling` removes resource caps and accounting explicitly,
    /// refuses a simultaneous resource cap, and retains deadline/cancellation signals:
    /// `fuel` bounds abstract execution steps, `deadline_ms` a wall-clock budget in
    /// milliseconds, `max_answers` the answer sequence (solution rows for SELECT, output
    /// statements for CONSTRUCT/DESCRIBE — including RDF 1.2 reifier and annotation
    /// statements — and nothing for ASK), `max_intermediate_cells` the largest intermediate
    /// bag in `rows * columns`, `max_scratch_bytes` the per-query scratch arena, and
    /// `max_remote_requests` federated requests. Every ceiling is **inclusive**:
    /// consumption equal to it is admitted, and zero is a valid ceiling that trips on the
    /// first charged unit of work.
    ///
    /// **A tripped governor is an outcome, not a thrown error** — see the module header.
    ///
    /// `aggregate_namespace` registers PurRDF's first-party statistical aggregate set
    /// (`MEDIAN`, `PERCENTILE`, `STDDEV`, `STDDEV_POP`, `VARIANCE`, `VAR_POP`, `MODE`,
    /// `FIRST`, `LAST`, `TOPK`) under that IRI namespace, so the query text can call
    /// `AGG(<{NAMESPACE}NAME>, args…)` (see `purrdf_validate::query::statistical_aggregates`). `None` (the default)
    /// leaves every one of the ten names an ordinary unregistered custom-aggregate IRI.
    ///
    /// # Errors
    ///
    /// A parse or evaluation failure, and a negative ceiling. A governor trip is neither.
    #[wasm_bindgen(js_name = queryGoverned)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    #[allow(
        clippy::too_many_arguments,
        reason = "each governed dimension is named explicitly at the boundary; a bag \
                  argument would make an unset ceiling and a misspelt one look alike, \
                  which is precisely the silent-hole failure a governor must not have"
    )]
    pub fn query_governed(
        &self,
        dataset: &Dataset,
        sparql: &str,
        base: Option<String>,
        aggregate_namespace: Option<String>,
        fuel: Option<i64>,
        deadline_ms: Option<i64>,
        max_answers: Option<i64>,
        max_intermediate_cells: Option<i64>,
        max_scratch_bytes: Option<i64>,
        max_remote_requests: Option<i64>,
        cancel: Option<CancellationToken>,
        no_ceiling: Option<bool>,
    ) -> Result<QueryOutcome, JsValue> {
        let args = decode_governor_args(
            fuel,
            deadline_ms,
            max_answers,
            max_intermediate_cells,
            max_scratch_bytes,
            max_remote_requests,
        )?
        .with_no_ceiling(no_ceiling.unwrap_or(false));
        let mut input = self.input(
            dataset,
            AsyncOperationKind::Governed,
            sparql,
            base.as_deref(),
        )?;
        input.aggregate_namespace = aggregate_namespace;
        input.ceilings = args;
        match input.run_offline(args.stop_watch(cancel.as_ref()))? {
            JobOutcome::Governed(outcome) => {
                Ok(query_outcome_from_governed(*outcome, self.blank_scope())?)
            }
            other => Err(unexpected_outcome("queryGoverned", &other)),
        }
    }

    /// Run a governed SPARQL query over a closure produced by `regime`, carrying the
    /// closure report and query outcome together.
    ///
    /// `aggregate_namespace` behaves exactly as on [`Self::query_governed`]: it registers
    /// PurRDF's first-party statistical aggregate set under that IRI namespace for the
    /// closure query's PARSE and its evaluation, so `AGG(<{NAMESPACE}NAME>, args…)` reaches
    /// the entailment-aware lane exactly as it reaches the ordinary one. `undefined` (the
    /// default) leaves every one of the ten names an ordinary unregistered custom-aggregate
    /// IRI.
    ///
    /// `import_iris`, `import_documents` and `premise_iris` are `entailCertainAnswers`'s, and
    /// apply to `dataset`: OWL 2 defines an ontology's imports closure to BE the ontology, so
    /// the closure the query runs over is materialized over the dataset merged with every
    /// N-Quads document the table supplies, and the report then states
    /// `ontology-import-resolved`. An `owl:imports` the table does not resolve, and the
    /// dataset does not already hold, throws by name — never a closure of a smaller premise —
    /// and so does a table entry the closure never reaches. Empty arrays are the ordinary
    /// "imports nothing" case, and all three are required rather than defaulted.
    ///
    /// `max_stored_facts` and `max_join_steps` (each a `bigint`, or `undefined` for this
    /// target's default of 131072 facts and 1048576 join steps) are the closure's evaluation
    /// limits for the `rdf`, `rdfs`, `owl-rl` and `d` regimes, exactly as `entailMaterialize`
    /// takes them. A closure past one throws naming the limit, the numbers and this method's
    /// argument (`queryEntailmentGoverned's maxStoredFacts`, …); the governors price the
    /// evaluation over the closure and never become a limit on it.
    ///
    /// # Errors
    ///
    /// An invalid regime/program, an import-table refusal, query parse/evaluation failure,
    /// entailment failure, or malformed ceiling. A governor trip is returned in
    /// [`EntailmentQueryOutcome`].
    #[wasm_bindgen(js_name = queryEntailmentGoverned)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    #[allow(
        clippy::too_many_arguments,
        reason = "the regime plus each governed dimension is named explicitly at the boundary"
    )]
    pub fn query_entailment_governed(
        &self,
        dataset: &Dataset,
        sparql: &str,
        base: Option<String>,
        regime: &str,
        program: Option<String>,
        import_iris: Vec<String>,
        import_documents: Vec<String>,
        premise_iris: Vec<String>,
        max_stored_facts: Option<u64>,
        max_join_steps: Option<u64>,
        aggregate_namespace: Option<String>,
        fuel: Option<i64>,
        deadline_ms: Option<i64>,
        max_answers: Option<i64>,
        max_intermediate_cells: Option<i64>,
        max_scratch_bytes: Option<i64>,
        max_remote_requests: Option<i64>,
        cancel: Option<CancellationToken>,
        no_ceiling: Option<bool>,
    ) -> Result<EntailmentQueryOutcome, JsValue> {
        let args = decode_governor_args(
            fuel,
            deadline_ms,
            max_answers,
            max_intermediate_cells,
            max_scratch_bytes,
            max_remote_requests,
        )?
        .with_no_ceiling(no_ceiling.unwrap_or(false));
        let mut input = self.input(
            dataset,
            AsyncOperationKind::EntailmentGoverned,
            sparql,
            base.as_deref(),
        )?;
        input.regime = Some(regime.to_owned());
        input.program = program;
        input.closure = ClosureInputs {
            import_iris,
            import_documents,
            premise_iris,
            max_stored_facts,
            max_join_steps,
        };
        input.aggregate_namespace = aggregate_namespace;
        input.ceilings = args;
        match input.run_offline(args.stop_watch(cancel.as_ref()))? {
            JobOutcome::Entailment(outcome) => Ok(entailment_query_outcome_from_native(
                *outcome,
                self.blank_scope(),
            )?),
            other => Err(unexpected_outcome("queryEntailmentGoverned", &other)),
        }
    }

    /// Apply a SPARQL UPDATE under caller-supplied execution governors, returning an
    /// [`UpdateOutcome`] rather than mutating unconditionally.
    ///
    /// The ceilings are those of [`Self::query_governed`] minus `max_answers`, which bounds
    /// an answer sequence an UPDATE does not have — passing it is refused rather than
    /// ignored, because a governor a caller believes they set and that nothing enforces is
    /// worse than no governor at all. A request's size is bounded by the ceilings on the
    /// work that computes it.
    ///
    /// **A tripped request applies nothing.** Not "not all of it": the dataset is left
    /// exactly as it was found, whichever operation the governor stopped and however much
    /// work the earlier operations of the same request had already done.
    ///
    /// `aggregate_namespace` behaves exactly as on [`Self::query_governed`], reachable
    /// from a `DELETE`/`INSERT … WHERE` clause through a nested `SELECT … GROUP BY` —
    /// the only place SPARQL UPDATE's grammar admits an aggregate.
    ///
    /// # Errors
    ///
    /// A parse or evaluation failure, a negative ceiling, and `max_answers`. A governor
    /// trip is none of these.
    #[wasm_bindgen(js_name = updateGoverned)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    #[allow(
        clippy::too_many_arguments,
        reason = "each governed dimension is named explicitly at the boundary; a bag \
                  argument would make an unset ceiling and a misspelt one look alike, \
                  which is precisely the silent-hole failure a governor must not have"
    )]
    pub fn update_governed(
        &self,
        dataset: &mut Dataset,
        sparql: &str,
        base: Option<String>,
        aggregate_namespace: Option<String>,
        fuel: Option<i64>,
        deadline_ms: Option<i64>,
        max_answers: Option<i64>,
        max_intermediate_cells: Option<i64>,
        max_scratch_bytes: Option<i64>,
        max_remote_requests: Option<i64>,
        cancel: Option<CancellationToken>,
        no_ceiling: Option<bool>,
    ) -> Result<UpdateOutcome, JsValue> {
        let args = decode_governor_args(
            fuel,
            deadline_ms,
            max_answers,
            max_intermediate_cells,
            max_scratch_bytes,
            max_remote_requests,
        )?
        .with_no_ceiling(no_ceiling.unwrap_or(false));
        args.update_ceilings()
            .map_err(|message| coded_error(&message, OPTIONS_CODE))?;
        let mut input = self.input(
            dataset,
            AsyncOperationKind::UpdateGoverned,
            sparql,
            base.as_deref(),
        )?;
        input.aggregate_namespace = aggregate_namespace;
        input.ceilings = args;
        match input.run_offline(args.stop_watch(cancel.as_ref()))? {
            JobOutcome::UpdateGoverned { outcome, frozen } => {
                // The engine publishes into its own `Arc` only on the applied path, so
                // there is a base to adopt only then.
                if let Some(frozen) = frozen {
                    dataset
                        .replace(frozen)
                        .map_err(|error| crate::dataset::diag_to_err(&error))?;
                }
                Ok(update_outcome_from_governed(&outcome))
            }
            other => Err(unexpected_outcome("updateGoverned", &other)),
        }
    }

    /// The engine's charge ledger for a query: the join orders it chose, the plan
    /// estimates it surveyed, and what each algebra node actually cost, rendered as text.
    ///
    /// This is how a caller sizes a budget before setting one. The query is evaluated
    /// under the metering configuration — every dimension counted, no dimension bounded —
    /// so the numbers are measurements of this query over this data, not predictions.
    ///
    /// # Errors
    ///
    /// A parse or evaluation failure.
    #[wasm_bindgen(js_name = explainQuery)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn explain_query(
        &self,
        dataset: &Dataset,
        sparql: &str,
        base: Option<String>,
    ) -> Result<String, JsValue> {
        let input = self.input(
            dataset,
            AsyncOperationKind::Explain,
            sparql,
            base.as_deref(),
        )?;
        raw_text(input.run_offline(None)?)
    }

    /// Serialize a CONSTRUCT/DESCRIBE result with a reusable compiled context.
    #[wasm_bindgen(js_name = queryRawWithContext)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn query_raw_with_context(
        &self,
        dataset: &Dataset,
        sparql: &str,
        base: Option<String>,
        format: &str,
        context: &CompiledJsonLdContext,
        yaml_schema_url: Option<String>,
    ) -> Result<String, JsValue> {
        let mut options = context_options(context);
        if let Some(url) = yaml_schema_url {
            options = options
                .with_yaml_schema_url(&url)
                .map_err(|error| coded_error(&error.to_string(), OPTIONS_CODE))?;
        }
        self.query_raw_with_options(
            dataset,
            AsyncOperationKind::RawWithContext,
            sparql,
            base.as_deref(),
            format,
            options,
        )
    }
}

impl QueryEngine {
    /// The blank-scope mode a typed result of this engine is converted under.
    pub(crate) fn blank_scope(&self) -> BlankScopeMode {
        self.blank_scope.get()
    }

    /// The division policy an operation of this engine runs under.
    pub(crate) fn division(&self) -> DivisionPolicy {
        self.division.get()
    }

    /// Set the division policy from its text form, through [`DivisionPolicy`]'s own
    /// reader; a refusal leaves the policy in force as it was.
    ///
    /// # Errors
    ///
    /// The reader's message for text in none of the policy's forms.
    pub(crate) fn set_division(&self, policy: &str) -> Result<(), String> {
        let parsed = policy
            .parse::<DivisionPolicy>()
            .map_err(|error| error.to_string())?;
        self.division.set(parsed);
        Ok(())
    }

    /// The shared engine, for an asynchronous job to hold for its own lifetime.
    pub(crate) const fn engine(&self) -> &Rc<NativeSparqlEngine> {
        &self.inner
    }

    /// An operation of `kind` over a snapshot of `dataset`.
    fn input<'a>(
        &self,
        dataset: &Dataset,
        kind: AsyncOperationKind,
        sparql: &'a str,
        base: Option<&'a str>,
    ) -> Result<OperationInput<'a>, JsValue> {
        let frozen = dataset.view().freeze().map_err(diagnostic_to_js)?;
        let mut input = OperationInput::new(kind, &self.inner, frozen, sparql, base);
        input.division = self.division();
        Ok(input)
    }

    fn run_query(
        &self,
        dataset: &Dataset,
        sparql: &str,
        base: Option<&str>,
    ) -> Result<SparqlResult, JsValue> {
        match self
            .input(dataset, AsyncOperationKind::Query, sparql, base)?
            .run_offline(None)?
        {
            JobOutcome::Query(result) => Ok(result),
            other => Err(unexpected_outcome("query", &other)),
        }
    }

    fn query_raw_with_options(
        &self,
        dataset: &Dataset,
        kind: AsyncOperationKind,
        sparql: &str,
        base: Option<&str>,
        format: &str,
        options: JsonLdSerializeOptions,
    ) -> Result<String, JsValue> {
        let mut input = self.input(dataset, kind, sparql, base)?;
        input.format = Some(format.to_owned());
        input.jsonld = Some(options);
        raw_text(input.run_offline(None)?)
    }
}

/// The text of a serializing operation's outcome.
fn raw_text(outcome: JobOutcome) -> Result<String, JsValue> {
    match outcome {
        JobOutcome::Raw(text) => Ok(text),
        other => Err(unexpected_outcome("a serialized query", &other)),
    }
}

/// The error for an operation that answered with an outcome of another kind than its
/// own — an invariant of [`OperationInput::execute`], reported rather than assumed.
fn unexpected_outcome(what: &str, outcome: &JobOutcome) -> JsValue {
    coded_error(
        &format!("{what} answered with a {outcome:?} outcome"),
        FailureCode::HostFault.code(),
    )
}

/// Serialize a CONSTRUCT/DESCRIBE result with configured JSON-LD/YAML-LD options — the
/// `queryRawConfigured`/`queryRawWithContext` egress, shared by the synchronous and
/// asynchronous lanes, with a plain `String` error.
pub(crate) fn serialize_configured_graph(
    result: SparqlResult,
    format: &str,
    options: &JsonLdSerializeOptions,
) -> Result<String, String> {
    match result {
        // WHICH BASE: none. The query's `base` resolves relative IRI references inside
        // the query TEXT. The trailing argument here is the *document* base a result is
        // WRITTEN under. They are different things, and a query that happened to need a
        // base to parse is no evidence about how its answer should be spelled, so
        // forwarding it would silently relativize the result against the query's base.
        //
        // This stays `None` even though the core seam can now express a document base
        // together with a graph selection and the statement layer: the blocker was never
        // the seam, it is that this function has no document base to pass. Giving the
        // caller one would mean a NEW parameter on `queryRawConfigured`, not reusing the
        // query's.
        SparqlResult::Graph(graph) => serialize_frozen_with_options(&graph, format, options, None),
        other => Err(kind_mismatch_message(
            "CONSTRUCT/DESCRIBE graph for configured JSON-LD serialization",
            &other,
        )),
    }
}

#[wasm_bindgen]
impl Dataset {
    /// `query(sparql, base?)` → run a SPARQL query against this dataset, offline.
    ///
    /// Returns **SPARQL Results JSON** for SELECT / ASK, and for CONSTRUCT / DESCRIBE
    /// **Turtle** — widened to **TriG** when the result carries a named graph, because
    /// Turtle has no `GRAPH` construct and would render such a result as an empty
    /// document. TriG is Turtle's dataset superset, so a default-graph-only result is
    /// byte-identical Turtle exactly as before; see `default_graph_format`.
    ///
    /// A parse error, an evaluation error, or a `SERVICE` / `LOAD` clause (unresolvable
    /// on this lane) throws a JsError — never a silent empty result. The `SILENT` forms
    /// are the caller's own opt-out and succeed with nothing fetched, as SPARQL 1.1
    /// requires; see this module's federation note.
    ///
    /// It runs on a fresh `QueryEngine`, so under the default division policy
    /// (`"18:toward-zero"`); set `divisionPolicy` on a `QueryEngine` for another.
    #[wasm_bindgen(js_name = query)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn query(&self, sparql: &str, base: Option<String>) -> Result<String, JsValue> {
        QueryEngine::new().query_raw(self, sparql, base, None, None, None)
    }
}

pub(crate) fn sparql_request<'a>(sparql: &'a str, base: Option<&'a str>) -> SparqlRequest<'a> {
    SparqlRequest {
        query: sparql,
        base_iri: base,
        substitutions: &[],
    }
}

/// The extension environment a query carrying `aggregates` is interpreted in.
///
/// One value rather than a loose registry, because whether a predicate IRI in a
/// query text is a data edge or a relation call is decided by the environment the
/// text is read against — and a door that takes the registries separately is a door
/// that can forget one.
///
/// # Errors
///
/// A message if a registered aggregate's declaration methods panic: deriving the
/// environment reads every declaration.
pub(crate) fn aggregate_env_message(
    aggregates: Option<&AggregateRegistry>,
) -> Result<purrdf_sparql_eval::ExtensionEnv, String> {
    purrdf_sparql_eval::ExtensionEnv::over_aggregates(
        aggregates.cloned().unwrap_or(AggregateRegistry::EMPTY),
    )
    .map_err(|e| format!("extension environment: {e}"))
}

pub(crate) fn query_result_from_sparql(
    result: SparqlResult,
    blank_scope: BlankScopeMode,
) -> Result<QueryResult, JsError> {
    Ok(match result {
        SparqlResult::Solutions {
            variables, rows, ..
        } => QueryResult {
            kind: QueryResultKind::Select,
            value: Some(QueryResultValue::Select(select_result(
                variables,
                rows,
                blank_scope,
            )?)),
        },
        SparqlResult::Boolean(value) => QueryResult {
            kind: QueryResultKind::Ask,
            value: Some(QueryResultValue::Ask(value)),
        },
        SparqlResult::Graph(graph) => QueryResult {
            kind: QueryResultKind::Graph,
            value: Some(QueryResultValue::Graph(Box::new(Dataset::from_frozen(
                graph,
            )?))),
        },
    })
}

fn select_result_from_sparql(
    result: SparqlResult,
    blank_scope: BlankScopeMode,
) -> Result<SelectResult, JsError> {
    match result {
        SparqlResult::Solutions {
            variables, rows, ..
        } => select_result(variables, rows, blank_scope),
        other => Err(kind_mismatch("SELECT solutions", &other)),
    }
}

fn graph_result_from_sparql(result: SparqlResult) -> Result<Dataset, JsError> {
    match result {
        SparqlResult::Graph(graph) => Dataset::from_frozen(graph),
        other => Err(kind_mismatch("CONSTRUCT/DESCRIBE graph", &other)),
    }
}

/// Convert a native governed outcome into the JS-facing [`QueryOutcome`] object.
///
/// Both arms produce an object; neither produces an error. That asymmetry with
/// `Result` is the whole point of the type.
pub(crate) fn query_outcome_from_governed(
    outcome: GovernedOutcome,
    blank_scope: BlankScopeMode,
) -> Result<QueryOutcome, JsError> {
    match outcome {
        GovernedOutcome::Complete {
            result, evidence, ..
        } => Ok(QueryOutcome {
            result: Some(query_result_from_sparql(result, blank_scope)?),
            partial: None,
            tripped: None,
            evidence: Some(GovernorEvidence { inner: evidence }),
            complete: true,
        }),
        GovernedOutcome::BudgetExhausted(BudgetExhausted {
            tripped,
            evidence,
            partial,
            ..
        }) => Ok(QueryOutcome {
            result: None,
            partial: Some(partial_answers_from_native(partial, blank_scope)?),
            tripped: Some(TrippedGovernor { inner: tripped }),
            evidence: Some(GovernorEvidence { inner: evidence }),
            complete: false,
        }),
    }
}

pub(crate) fn entailment_query_outcome_from_native(
    outcome: GovernedEntailment,
    blank_scope: BlankScopeMode,
) -> Result<EntailmentQueryOutcome, JsError> {
    match outcome {
        GovernedEntailment::Answered { outcome, report } => {
            let tripped = outcome.tripped().map(|inner| TrippedGovernor { inner });
            Ok(EntailmentQueryOutcome {
                complete: tripped.is_none(),
                outcome: Some(query_outcome_from_governed(outcome, blank_scope)?),
                report: Some(purrdf_validate::render_reasoning_report(&report)),
                tripped,
                closure_stopped: false,
            })
        }
        GovernedEntailment::ClosureStopped { tripped } => Ok(EntailmentQueryOutcome {
            outcome: None,
            report: None,
            tripped: Some(TrippedGovernor { inner: tripped }),
            complete: false,
            closure_stopped: true,
        }),
        _ => Err(JsError::new("unsupported governed entailment outcome")),
    }
}

/// Convert a native governed UPDATE outcome into the JS-facing [`UpdateOutcome`] object.
pub(crate) fn update_outcome_from_governed(outcome: &GovernedUpdateOutcome) -> UpdateOutcome {
    UpdateOutcome {
        tripped: outcome.tripped().map(|inner| TrippedGovernor { inner }),
        evidence: Some(GovernorEvidence {
            inner: outcome.evidence().clone(),
        }),
        applied: outcome.is_applied(),
    }
}

/// Convert the evaluator's certificate into the JS-facing [`PartialAnswers`] object.
fn partial_answers_from_native(
    partial: PartialValue,
    blank_scope: BlankScopeMode,
) -> Result<PartialAnswers, JsError> {
    let certainty = match partial {
        PartialValue::Certain(_) => "certain",
        PartialValue::AtMost(_) => "at-most",
        PartialValue::Unknown(_) => "unknown",
    };
    let barrier = partial
        .barrier()
        .map(|barrier| barrier.operator().to_owned());
    let (result, positional_prefix) = match partial.into_result() {
        Some(rows) => {
            let positional_prefix = rows.is_positional_prefix();
            (
                Some(query_result_from_sparql(rows.into_result(), blank_scope)?),
                Some(positional_prefix),
            )
        }
        None => (None, None),
    };
    Ok(PartialAnswers {
        certainty,
        result,
        positional_prefix,
        barrier,
    })
}

fn select_result(
    variables: Vec<String>,
    rows: Vec<Vec<Option<purrdf::TermValue>>>,
    blank_scope: BlankScopeMode,
) -> Result<SelectResult, JsError> {
    let variables: Rc<[String]> = Rc::from(variables.into_boxed_slice());
    let rows: Vec<Option<SelectRow>> = rows
        .into_iter()
        .map(|row| select_row(Rc::clone(&variables), row, blank_scope).map(Some))
        .collect::<Result<Vec<_>, _>>()?;
    let remaining = rows.len();
    Ok(SelectResult {
        variables,
        rows,
        next: 0,
        remaining,
    })
}

fn select_row(
    variables: Rc<[String]>,
    row: Vec<Option<purrdf::TermValue>>,
    blank_scope: BlankScopeMode,
) -> Result<SelectRow, JsError> {
    let values = row
        .into_iter()
        .map(|value| {
            value
                .map(|value| term_from_value(value, blank_scope))
                .transpose()
                .map_err(|e| JsError::new(&e))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SelectRow { variables, values })
}

fn term_from_value(value: purrdf::TermValue, blank_scope: BlankScopeMode) -> Result<Term, String> {
    Term::from_value(value, blank_scope)
}

/// Decode `provenance_prefix`/`provenance_iri` (both `None`, or both `Some`) into an
/// optional [`purrdf_sparql_results::ProvenanceNamespace`]. Exactly one `Some` is a
/// usage error: a namespace needs both halves, and silently treating a lone prefix or
/// IRI as "no namespace" would be the exact silent-drop this binding refuses elsewhere.
pub(crate) fn build_provenance_namespace(
    prefix: Option<String>,
    iri: Option<String>,
) -> Result<Option<purrdf_sparql_results::ProvenanceNamespace>, JsError> {
    match (prefix, iri) {
        (None, None) => Ok(None),
        (Some(prefix), Some(iri)) => {
            let namespace = purrdf_sparql_results::ProvenanceNamespace::new(prefix, iri)
                .map_err(|e| JsError::new(&e.to_string()))?;
            Ok(Some(namespace))
        }
        _ => Err(JsError::new(
            "provenance_prefix and provenance_iri must be both supplied or both omitted",
        )),
    }
}

/// Serialize a query result to text in `format`, or the kind's default when `None`.
///
/// Returns a plain `String` error so the asynchronous lane can record it on a job and
/// the native build can test it; the synchronous entry maps it to a `JsError`.
pub(crate) fn serialize_query_result(
    result: &SparqlResult,
    format: Option<&str>,
    provenance_namespace: Option<&purrdf_sparql_results::ProvenanceNamespace>,
    query: &str,
) -> Result<String, String> {
    match result {
        SparqlResult::Graph(graph) => {
            serialize_graph_result(graph, format.unwrap_or_else(|| default_graph_format(graph)))
        }
        SparqlResult::Solutions { .. } | SparqlResult::Boolean(_) => {
            let results_format = match format {
                None => SparqlResultsFormat::Json,
                Some(format) => SparqlResultsFormat::from_name(format).ok_or_else(|| {
                    format!(
                        "unsupported SPARQL results format {:?} \
                         (use json/xml/csv/tsv or graph formats for CONSTRUCT/DESCRIBE)",
                        format.trim()
                    )
                })?,
            };
            serialize_tabular_result(result, results_format, provenance_namespace, query)
        }
    }
}

fn serialize_tabular_result(
    result: &SparqlResult,
    format: SparqlResultsFormat,
    provenance_namespace: Option<&purrdf_sparql_results::ProvenanceNamespace>,
    query: &str,
) -> Result<String, String> {
    let provenance = purrdf_validate::query::provenance(provenance_namespace, query);
    let outcome = serialize_results(result, format, &provenance, provenance_namespace)
        .map_err(|e| e.to_string())?;
    String::from_utf8(outcome.bytes).map_err(|e| format!("SPARQL result is not valid UTF-8: {e}"))
}

/// The closing imperative of every named-graph refusal on this binding: the
/// quad-capable spellings [`resolve_media_type`] accepts.
///
/// The rest of the sentence is [`named_graph_refusal`], shared verbatim with the CLI
/// and Python hosts; only the remedy is per-host, because these format tokens are the
/// ones a JS caller actually passes.
const QUAD_CAPABLE_REMEDY: &str =
    "Re-serialize with a quad-capable format (trig/nquads/trix/hextuples/jsonld/yamlld)";

/// The format a CONSTRUCT/DESCRIBE result is serialized to when the JS caller named
/// NONE: `turtle`, or `trig` when the result carries a named graph.
///
/// # Why the default WIDENS instead of throwing
///
/// A caller who passed no format asked for "the readable default", not for Turtle
/// specifically — and Turtle cannot hold a named graph, so a graph-carrying result met
/// with the historical `turtle` default produced a well-formed EMPTY document and no
/// error at all: `dataset.query('… CONSTRUCT { GRAPH ex:g { … } } …')` returned `""`.
/// That is the silent empty result the [`query`](Dataset::query) contract promises can
/// never happen.
///
/// TriG is Turtle's dataset superset — same prefixes, same abbreviations, same
/// blank-node syntax, plus a `GRAPH` block — so widening the default costs the caller
/// nothing they asked for and gives them the graph names the query spelled out. A
/// result with no named graph still gets `turtle`, byte-for-byte as before, so no
/// existing SPARQL 1.1 CONSTRUCT or DESCRIBE changes shape.
///
/// This is deliberately NOT the answer for an EXPLICIT format: naming `turtle` for a
/// graph-carrying result is a request this binding cannot honour, and silently
/// answering with TriG would hand a triple-only consumer bytes it cannot read. That
/// case throws — see [`refuse_uncarriable_named_graphs`].
fn default_graph_format(graph: &Arc<purrdf::RdfDataset>) -> &'static str {
    if distinct_graph_names(&**graph).is_empty() {
        "turtle"
    } else {
        "trig"
    }
}

/// Refuse to serialize a graph result that carries named graphs to a single-graph RDF
/// syntax, naming the graphs, the format, and what to use instead.
///
/// # Why this THROWS rather than quietly widening the format
///
/// [`default_graph_format`] widens `turtle` to `trig` when the caller named no format,
/// because there was no request to contradict. An EXPLICIT `serialize("turtle")` is the
/// opposite situation: the caller named a syntax, most likely because something
/// downstream reads only that syntax, so answering with TriG bytes would break them and
/// answering with Turtle bytes would drop exactly the statements the query asked for
/// (the single-graph serializers DROP graph-scoped rows — they do not fold them into
/// the default graph). Both silent answers are wrong, so this one is loud, exactly as
/// the `purrdf query` lane and the Python `QueryQuads.serialize` are.
fn refuse_uncarriable_named_graphs(
    graph: &Arc<purrdf::RdfDataset>,
    fmt: purrdf::NativeRdfFormat,
    token: &str,
) -> Result<(), String> {
    match uncarriable_named_graphs(graph, fmt, token) {
        Some(message) => Err(message),
        None => Ok(()),
    }
}

/// The refusal message [`refuse_uncarriable_named_graphs`] would throw, or `None` when
/// the pair is carriable.
///
/// Split out of the throwing wrapper so the wording is unit-testable on the NATIVE
/// build: constructing a `JsError` calls a wasm-only import that panics off wasm, the
/// same reason `codec::resolve_media_type` returns a `String` error. The Node lane
/// (`js/tests/query.test.mjs`) then observes the real thrown message on the real
/// module, so both halves of the refusal are pinned.
fn uncarriable_named_graphs(
    graph: &Arc<purrdf::RdfDataset>,
    fmt: purrdf::NativeRdfFormat,
    token: &str,
) -> Option<String> {
    if fmt.supports_datasets() {
        return None;
    }
    let names = distinct_graph_names(&**graph);
    if names.is_empty() {
        return None;
    }
    Some(named_graph_refusal(&names, token, QUAD_CAPABLE_REMEDY))
}

/// Serialize a CONSTRUCT/DESCRIBE answer graph.
///
/// WHICH BASE: none, and that is not an oversight. The only base anywhere near this
/// call is the SPARQL *query* base, which resolves relative IRI references inside the
/// query TEXT — a different thing from the *document* base an answer is written under.
/// This helper is handed no document base by any caller, so it emits absolute IRIs.
/// Giving callers one means a new parameter on the `queryRaw`/`queryGraph` surface, not
/// reusing the query base. `serialize_dataset` applies `StatementLayer::Emit`, so an
/// answer graph carrying RDF 1.2 statement rows keeps them.
fn serialize_graph_result(graph: &Arc<purrdf::RdfDataset>, format: &str) -> Result<String, String> {
    let fmt = resolve_format(format)?;
    // Refused BEFORE the serializer runs: a result the requested syntax would silently
    // empty out never becomes a string.
    refuse_uncarriable_named_graphs(graph, fmt, format)?;
    let bytes = serialize_dataset(graph, fmt.media_type(), SerializeGraph::Dataset)
        .map_err(|e| e.to_string())?;
    String::from_utf8(bytes).map_err(|e| format!("SPARQL graph result is not valid UTF-8: {e}"))
}

pub(crate) fn kind_mismatch(expected: &str, actual: &SparqlResult) -> JsError {
    JsError::new(&kind_mismatch_message(expected, actual))
}

/// The words of [`kind_mismatch`], for a caller that records rather than throws.
pub(crate) fn kind_mismatch_message(expected: &str, actual: &SparqlResult) -> String {
    format!("expected {expected}, got {}", sparql_result_kind(actual))
}

fn sparql_result_kind(result: &SparqlResult) -> &'static str {
    match result {
        SparqlResult::Solutions { .. } => "SELECT solutions",
        SparqlResult::Boolean(_) => "ASK boolean",
        SparqlResult::Graph(_) => "CONSTRUCT/DESCRIBE graph",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::term::TermInner;

    #[test]
    fn select_rows_are_moved_once_and_share_variables() {
        let rows = vec![
            vec![Some(purrdf::TermValue::Iri("https://e/a".to_owned()))],
            vec![Some(purrdf::TermValue::Iri("https://e/b".to_owned()))],
        ];
        let mut result = select_result(vec!["value".to_owned()], rows, BlankScopeMode::Keep)
            .expect("select result");

        assert_eq!(result.row_count(), 2);
        assert_eq!(result.remaining(), 2);
        let mut second = result.take_row(1).expect("indexed row");
        assert!(Rc::ptr_eq(&result.variables, &second.variables));
        assert!(result.take_row(1).is_none());
        assert_eq!(result.remaining(), 1);
        assert!(matches!(
            second.take_value(0).expect("bound value").inner,
            TermInner::Named(iri) if iri == "https://e/b"
        ));
        assert!(second.take_value(0).is_none());

        let first = result.next_row().expect("remaining row");
        assert!(Rc::ptr_eq(&result.variables, &first.variables));
        assert_eq!(result.remaining(), 0);
        assert!(result.next_row().is_none());
    }

    /// Two blank nodes that share the label `b` in different scopes: the engine's
    /// values, as one row of a SELECT.
    fn scoped_blank_row() -> Vec<Vec<Option<purrdf::TermValue>>> {
        let blank = |scope| purrdf::TermValue::Blank {
            label: "b".to_owned(),
            scope: purrdf::BlankScope(scope),
        };
        vec![vec![Some(blank(1)), Some(blank(2)), Some(blank(0))]]
    }

    #[test]
    fn blank_scope_keep_leaves_two_scoped_blanks_two_nodes_in_a_select_row() {
        let mut result = select_result(
            vec!["x".to_owned(), "y".to_owned(), "z".to_owned()],
            scoped_blank_row(),
            BlankScopeMode::Keep,
        )
        .expect("select result");
        let row = result.take_row(0).expect("the row");
        let (x, y, z) = (
            row.get("x").expect("x"),
            row.get("y").expect("y"),
            row.get("z").expect("z"),
        );
        assert_ne!(x.value(), y.value(), "two scopes collapsed under keep");
        assert!(!x.equals(&y));
        // The unscoped blank keeps its bare label, distinct from both scoped ones.
        assert_eq!(z.value(), "b");
        assert_ne!(x.value(), z.value());
    }

    #[test]
    fn blank_scope_merge_makes_them_one_node_and_a_neighbour_is_unchanged() {
        let mut result = select_result(
            vec!["x".to_owned(), "y".to_owned(), "z".to_owned()],
            scoped_blank_row(),
            BlankScopeMode::Merge,
        )
        .expect("select result");
        let row = result.take_row(0).expect("the row");
        let (x, y, z) = (
            row.get("x").expect("x"),
            row.get("y").expect("y"),
            row.get("z").expect("z"),
        );
        assert_eq!(x.value(), "b");
        assert!(x.equals(&y), "merge keeps the scopes apart");
        assert!(x.equals(&z));
        // A term that is not a blank node is unchanged by the mode.
        let iri = select_result(
            vec!["s".to_owned()],
            vec![vec![Some(purrdf::TermValue::iri("https://e/s"))]],
            BlankScopeMode::Merge,
        )
        .expect("select result")
        .take_row(0)
        .expect("the row")
        .get("s")
        .expect("s");
        assert_eq!(iri.value(), "https://e/s");
    }

    #[test]
    fn an_engine_holds_keep_until_told_otherwise() {
        let engine = QueryEngine::new();
        assert_eq!(engine.blank_scope(), BlankScopeMode::Keep);
        assert_eq!(engine.blank_scope_name(), "keep");
        engine
            .set_blank_scope("merge".to_owned())
            .expect("merge is a mode");
        assert_eq!(engine.blank_scope(), BlankScopeMode::Merge);
        assert_eq!(engine.blank_scope_name(), "merge");
        engine
            .set_blank_scope("keep".to_owned())
            .expect("keep is a mode");
        assert_eq!(engine.blank_scope(), BlankScopeMode::Keep);
    }

    /// A quad-template `CONSTRUCT` hands JS the graph names, and they survive
    /// serialization.
    ///
    /// The JS egress for a graph result is a `Dataset` wrapping the frozen IR the
    /// engine produced, with nothing projected out of it — so unlike a triple-shaped
    /// result surface it has somewhere to PUT a graph name. This pins that: the graph
    /// the query named is still on the quad when the `Dataset` is serialized to
    /// N-Quads, so a JS caller can never silently receive a CONSTRUCT result with its
    /// graph names removed.
    #[test]
    fn construct_graph_result_keeps_its_graph_name() {
        let source = Dataset::parse(
            "<https://example.org/s> <https://example.org/p> <https://example.org/o> .",
            "ntriples",
            None,
        )
        .expect("seed dataset parses");
        let constructed = QueryEngine::new()
            .construct(
                &source,
                "CONSTRUCT { GRAPH <https://example.org/g> { ?s ?p ?o } } WHERE { ?s ?p ?o }",
                None,
            )
            .expect("quad-template CONSTRUCT evaluates");
        let nquads = constructed
            .serialize("nquads", None)
            .expect("N-Quads serializes");
        assert_eq!(
            nquads.trim(),
            "<https://example.org/s> <https://example.org/p> <https://example.org/o> \
             <https://example.org/g> ."
        );
    }

    const SEED_NT: &str =
        "<https://example.org/s> <https://example.org/p> <https://example.org/o> .";
    const GRAPH_CONSTRUCT: &str =
        "CONSTRUCT { GRAPH <https://example.org/g> { ?s ?p ?o } } WHERE { ?s ?p ?o }";
    const PLAIN_CONSTRUCT: &str = "CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }";

    fn seed() -> Dataset {
        Dataset::parse(SEED_NT, "ntriples", None).expect("seed dataset parses")
    }

    /// The DEFAULT entry point never answers a graph-carrying result with nothing.
    ///
    /// `Dataset.query` passes no format, and the historical `turtle` default rendered a
    /// quad-template CONSTRUCT as a well-formed EMPTY document with no error — the exact
    /// silent empty result the method's own contract forbids.
    #[test]
    fn the_default_format_carries_a_named_graph_instead_of_emptying_it() {
        let out = seed()
            .query(GRAPH_CONSTRUCT, None)
            .expect("quad-template CONSTRUCT serializes under the default format");
        assert!(!out.trim().is_empty(), "the default must never be empty");
        assert!(
            out.contains("<https://example.org/g>"),
            "the graph the query named must be in the output: {out}"
        );
        assert!(
            out.contains("<https://example.org/s>"),
            "the constructed statement must be in the output: {out}"
        );
    }

    /// A result with no named graph still defaults to Turtle, byte-for-byte: widening
    /// the default is conditional on what the result carries, not a blanket switch.
    #[test]
    fn the_default_format_is_still_turtle_for_a_default_graph_result() {
        let out = seed()
            .query(PLAIN_CONSTRUCT, None)
            .expect("plain CONSTRUCT serializes");
        let turtle = seed()
            .query_with_format(PLAIN_CONSTRUCT, "turtle")
            .expect("explicit turtle serializes");
        assert_eq!(out, turtle);
        assert!(
            !out.contains("GRAPH"),
            "no GRAPH block for a default-graph result: {out}"
        );
    }

    /// An EXPLICIT single-graph format is a request this binding cannot honour, so it
    /// throws — naming the graphs, the format, and the quad-capable alternatives.
    ///
    /// Asserted through the message builder rather than the thrown `JsError`, because
    /// constructing a `JsError` panics off wasm; `js/tests/query.test.mjs` observes the
    /// real throw on the real module.
    #[test]
    fn an_explicit_single_graph_format_refuses_a_graph_carrying_result() {
        let constructed = QueryEngine::new()
            .construct(&seed(), GRAPH_CONSTRUCT, None)
            .expect("quad-template CONSTRUCT evaluates");
        let frozen = constructed.view().freeze().expect("freeze");
        for token in ["turtle", "ntriples", "rdfxml"] {
            let fmt = resolve_format(token).expect("format resolves");
            let message = uncarriable_named_graphs(&frozen, fmt, token)
                .expect("a single-graph syntax must refuse a graph-carrying result");
            assert!(
                message.contains("carrying 1 named graph (<https://example.org/g>)"),
                "the refusal names the graph: {message}"
            );
            assert!(
                message.contains(token),
                "the refusal names the offending format: {message}"
            );
            assert!(
                message.contains("trig/nquads/trix/hextuples/jsonld/yamlld"),
                "the refusal names the alternatives: {message}"
            );
        }
        // Nothing to refuse on the quad-capable side, or for a default-graph result.
        for token in ["trig", "nquads", "trix", "hextuples", "jsonld", "yamlld"] {
            let fmt = resolve_format(token).expect("format resolves");
            assert!(uncarriable_named_graphs(&frozen, fmt, token).is_none());
        }
        let plain = QueryEngine::new()
            .construct(&seed(), PLAIN_CONSTRUCT, None)
            .expect("plain CONSTRUCT evaluates");
        let plain = plain.view().freeze().expect("freeze");
        let fmt = resolve_format("turtle").expect("format resolves");
        assert!(uncarriable_named_graphs(&plain, fmt, "turtle").is_none());
    }

    /// The quad-capable syntaxes are untouched by the refusal.
    #[test]
    fn a_quad_capable_format_serializes_a_graph_carrying_result() {
        let out = seed()
            .query_with_format(GRAPH_CONSTRUCT, "nquads")
            .expect("N-Quads carries a named graph");
        assert!(out.contains("<https://example.org/g>"), "{out}");
    }

    /// A single-graph syntax is still fine for a result that names no graph.
    #[test]
    fn a_single_graph_format_still_serializes_a_default_graph_result() {
        let out = seed()
            .query_with_format(PLAIN_CONSTRUCT, "ntriples")
            .expect("N-Triples carries a default-graph result");
        assert_eq!(out.trim(), SEED_NT);
    }

    /// The default is chosen from what the RESULT carries, not from the query text.
    #[test]
    fn the_default_format_widens_only_for_a_graph_carrying_result() {
        let graphed = QueryEngine::new()
            .construct(&seed(), GRAPH_CONSTRUCT, None)
            .expect("quad-template CONSTRUCT evaluates")
            .view()
            .freeze()
            .expect("freeze");
        assert_eq!(default_graph_format(&graphed), "trig");
        let plain = QueryEngine::new()
            .construct(&seed(), PLAIN_CONSTRUCT, None)
            .expect("plain CONSTRUCT evaluates")
            .view()
            .freeze()
            .expect("freeze");
        assert_eq!(default_graph_format(&plain), "turtle");
    }

    /// A graph-scoped RDF 1.2 statement layer: base quad, reifier declaration and
    /// annotation all asserted in `ex:g`.
    const GRAPH_STAR_TRIG: &str = concat!(
        "@prefix ex: <https://example.org/> .\n",
        "GRAPH ex:g { ex:s ex:p ex:o ~ex:r {| ex:note \"n\" |} . }\n",
    );

    /// A `DESCRIBE` reaches the SAME egress a quad-template `CONSTRUCT` does: the
    /// description's graphs survive to JS, the default format widens to TriG rather
    /// than emitting nothing, and an explicit single-graph format refuses.
    ///
    /// `describe` and `construct` are one function behind two names
    /// (`graph_result_from_sparql`), and a description is graph-faithful at every layer
    /// — base quad, reifier declaration and annotation alike — so the DESCRIBE lane
    /// carries named graphs even though no `DESCRIBE` ever names one.
    #[test]
    fn a_describe_carries_its_graphs_and_refuses_a_single_graph_format() {
        let source = Dataset::parse(GRAPH_STAR_TRIG, "trig", None).expect("TriG parses");
        let described = QueryEngine::new()
            .describe(&source, "DESCRIBE <https://example.org/s>", None)
            .expect("DESCRIBE evaluates");
        let nquads = described
            .serialize("nquads", None)
            .expect("N-Quads serializes");
        for row in [
            "<https://example.org/s> <https://example.org/p> <https://example.org/o> \
             <https://example.org/g> .",
            "<https://example.org/r> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> \
             <<( <https://example.org/s> <https://example.org/p> <https://example.org/o> )>> \
             <https://example.org/g> .",
            "<https://example.org/r> <https://example.org/note> \"n\" \
             <https://example.org/g> .",
        ] {
            assert!(
                nquads.contains(row),
                "the description must carry `{row}`:\n{nquads}"
            );
        }

        let frozen = described.view().freeze().expect("freeze");
        assert_eq!(
            default_graph_format(&frozen),
            "trig",
            "the default must widen, not empty the description out"
        );
        for token in ["turtle", "ntriples", "rdfxml"] {
            let fmt = resolve_format(token).expect("format resolves");
            let message = uncarriable_named_graphs(&frozen, fmt, token)
                .expect("a single-graph syntax must refuse a graph-carrying description");
            assert!(
                message.contains("carrying 1 named graph (<https://example.org/g>)"),
                "the refusal names the graph: {message}"
            );
        }
    }

    /// The lexical form `quotient` answers on `engine`, through `select`.
    fn quotient(engine: &QueryEngine, quotient: &str) -> String {
        let mut rows = engine
            .select(&seed(), &format!("SELECT ({quotient} AS ?x) {{}}"), None)
            .expect("the quotient evaluates");
        rows.next_row()
            .expect("one row")
            .take_value(0)
            .expect("bound")
            .value()
    }

    /// An engine starts at XSD's eighteen digits truncated toward zero; `exact` answers
    /// a terminating quotient exactly, and `5:half-even` rounds. Text in no policy form
    /// is refused through the reader and leaves the policy in force: the getter is
    /// unchanged and the next query still rounds by it. (The refused text is a thrown
    /// `JsValue`, which cannot be built off wasm, so `operation::tests` observes the
    /// policy at the operation, and `js/tests` on the module.)
    #[test]
    fn an_engine_carries_its_division_policy_into_every_query() {
        let engine = QueryEngine::new();
        assert_eq!(engine.division_policy(), "18:toward-zero");
        assert_eq!(quotient(&engine, "2/3"), "0.666666666666666666");

        engine.set_division("exact").expect("exact is a policy");
        assert_eq!(engine.division_policy(), "exact");
        assert_eq!(quotient(&engine, "1/8"), "0.125");
        let input = engine
            .input(&seed(), AsyncOperationKind::Governed, "ASK {}", None)
            .expect("an input");
        assert_eq!(input.division, DivisionPolicy::Exact);

        engine
            .set_division("5:half-even")
            .expect("5:half-even is a policy");
        assert_eq!(engine.division_policy(), "5:half-even");
        assert_eq!(quotient(&engine, "2/3"), "0.66667");

        for refused in ["5:sideways", "", "inexact", "-1"] {
            let message = engine.set_division(refused).expect_err("not a policy");
            assert!(message.contains("division policy"), "{message}");
            assert_eq!(engine.division_policy(), "5:half-even");
        }
        assert_eq!(quotient(&engine, "2/3"), "0.66667");
        engine
            .set_division("7")
            .expect("a bare digit count is a policy");
        assert_eq!(engine.division_policy(), "7:toward-zero");
        assert_eq!(quotient(&engine, "2/3"), "0.6666666");
    }

    /// A governed query's evidence counts the expression error it absorbed into an
    /// unbound value, positionally by `expressionErrorCodes`, and a valid neighbour's is
    /// all zero.
    #[test]
    fn governed_evidence_counts_absorbed_expression_errors() {
        assert_eq!(
            expression_error_codes(),
            [
                "err:FOAR0001",
                "err:FOAR0002",
                "err:FOCA0001",
                "err:FOCA0002",
                "err:FOCA0003",
                "err:FOCA0006",
                "err:FORG0001",
                "err:XPTY0004",
            ]
        );
        let engine = QueryEngine::new();
        let evidence = |sparql: &str| {
            engine
                .query_governed(
                    &seed(),
                    sparql,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                )
                .expect("the governed query evaluates")
                .take_evidence()
                .expect("evidence")
                .expression_errors()
        };
        assert_eq!(evidence("SELECT (1/0 AS ?x) {}"), [1, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(evidence("SELECT (1/2 AS ?x) {}"), [0; 8]);
    }

    impl Dataset {
        /// `queryRaw` with an explicit format and no provenance namespace — the
        /// two-argument shape these tests exercise.
        fn query_with_format(&self, sparql: &str, format: &str) -> Result<String, JsValue> {
            QueryEngine::new().query_raw(self, sparql, None, Some(format.to_owned()), None, None)
        }
    }
}
