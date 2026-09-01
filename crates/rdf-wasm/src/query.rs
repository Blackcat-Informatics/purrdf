// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The offline, in-browser SPARQL query surface over the wasm [`Dataset`].
//!
//! Binds the native multiset SPARQL evaluator
//! ([`NativeSparqlEngine`](purrdf_sparql_eval::NativeSparqlEngine)) to JavaScript so a
//! page can run SELECT / ASK / CONSTRUCT / DESCRIBE entirely client-side, with no
//! server and no network. The engine is the same one the native query gate uses,
//! with no baked-in HTTP client.
//!
//! ## Federation is intentionally absent
//!
//! This binds the plain [`SparqlEngine::query`](purrdf_core::SparqlEngine::query)
//! entry — the one with **no** [`RemoteQuerySource`](purrdf_sparql_eval::remote)
//! installed. A `SERVICE` or `LOAD` clause therefore **hard-fails** with a JsError
//! rather than silently returning an empty or partial result: in a browser there is
//! no resolver to fetch a remote graph, and a false answer is worse than an error.
//!
//! ## Result encoding
//!
//! - SELECT / ASK → **SPARQL Results JSON** (the W3C SRJ format) via
//!   [`purrdf_sparql_results`].
//! - CONSTRUCT / DESCRIBE → **Turtle** via the `native_codecs` serializer (the one
//!   serialization seam; never `oxigraph::io`, never the `purrdf-gts` crate).
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
//! build reads `js_sys::Date::now()` rather than `std::time::Instant`, which would compile
//! here and panic at run time. The Node round-trip lane (`js/tests/governors.test.mjs`)
//! executes a real deadline trip against the optimized module so that split is *observed*
//! rather than merely compiled.

use std::rc::Rc;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use purrdf::ir::MutableDataset;
use purrdf::{
    GovernedEntailment, JsonLdSerializeOptions, QueryEntailmentPlan, SerializeGraph,
    query_with_entailment_governed, serialize_dataset,
};
use purrdf_core::{SparqlEngine, SparqlRequest, SparqlResult};
use purrdf_sparql_eval::{
    AggregateRegistry, BudgetExhausted, CancellationFlag, GovernedOutcome, GovernedUpdateOutcome,
    GovernorEvidence as EvidenceValue, NativeSparqlEngine, PartialAnswers as PartialValue,
    QueryGovernors, QueryOptions, ResourceDimension, StopCause, StopSignal,
    TrippedGovernor as TrippedValue, WallDeadline,
};
use purrdf_sparql_results::{
    ResultProvenance, SparqlResultsFormat, serialize as serialize_results,
};
use wasm_bindgen::prelude::*;

use crate::codec::resolve_media_type;
use crate::convert::term_value_into_rdf_term;
use crate::dataset::{Dataset, diag_to_err};
use crate::jsonld::{CompiledJsonLdContext, context_options, decode_options};
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
    Graph(Dataset),
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
            QueryResultValue::Graph(dataset) => Some(dataset),
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
/// no writer on this surface (or the CLI's/C ABI's matching `build_query_provenance`)
/// populates it today — it is the evaluator/S11 derivation graph's progressive fill
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
/// A JavaScript host is single-threaded and the wasm boundary is synchronous, so a token
/// flipped on the same thread that is inside `queryGoverned` can never be observed by it —
/// the flip cannot run until the call returns. The token is therefore for the two shapes
/// that genuinely work: cancelling *before* a call (a queued query the user has since
/// navigated away from), and cancelling a worker's query from the main thread when the
/// token is shared through a `SharedArrayBuffer`-backed worker split. Both observe the
/// same latching bit, and both report the same `"cancelled"` trip.
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

/// The composed [`StopSignal`] every governed wasm call runs under.
///
/// Two sources, one signal, because `QueryGovernors::with_stop_signal` takes one and
/// composing them is the host's job: the caller's [`CancellationToken`], when one was
/// supplied, and the caller's wall deadline, when one was.
///
/// # Latching
///
/// The trait's contract is that a fired signal stays fired, so the resolved cause is
/// written once into a `OnceLock` and every later poll returns it without consulting a
/// source again. A simultaneous fire resolves the way the kernel ranks it — a cancellation
/// (an explicit decision) ahead of a deadline (an elapsed measurement).
#[derive(Debug)]
struct WasmStopWatch {
    /// The resolved cause, written once. See the latching note above.
    latched: OnceLock<StopCause>,
    /// The caller's cancellation bit, when one was supplied.
    cancel: Option<CancellationFlag>,
    /// The caller's wall deadline, when one was supplied. This is the wasm clock read.
    deadline: Option<WallDeadline>,
}

impl WasmStopWatch {
    /// A watch over the caller's `deadline_ms` budget and `cancel` token, if any.
    fn new(deadline_ms: Option<u64>, cancel: Option<&CancellationToken>) -> Self {
        Self {
            latched: OnceLock::new(),
            cancel: cancel.map(|token| token.flag.clone()),
            deadline: deadline_ms.map(|ms| WallDeadline::after(Duration::from_millis(ms))),
        }
    }

    /// Whether this watch has anything at all to watch.
    const fn is_armed(&self) -> bool {
        self.cancel.is_some() || self.deadline.is_some()
    }

    /// Poll every source once and resolve a simultaneous fire by the kernel's precedence.
    fn observe(&self) -> Option<StopCause> {
        if self
            .cancel
            .as_ref()
            .is_some_and(CancellationFlag::is_cancelled)
        {
            return Some(StopCause::Cancelled);
        }
        self.deadline.as_ref().and_then(StopSignal::poll)
    }
}

impl StopSignal for WasmStopWatch {
    fn poll(&self) -> Option<StopCause> {
        if let Some(&cause) = self.latched.get() {
            return Some(cause);
        }
        let cause = self.observe()?;
        Some(*self.latched.get_or_init(|| cause))
    }
}

/// The ceilings one governed call carries, after decoding and before they are engaged.
///
/// `None` in a slot means the caller declined that ceiling — never zero, which is a
/// perfectly valid ceiling that trips on the first charged unit of work.
#[derive(Debug, Clone, Copy, Default)]
struct GovernorArgs {
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
    /// Engage these ceilings, plus the caller's stop sources, as one call's governors.
    ///
    /// # Why the base is `METERED` rather than `UNBOUNDED`
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
    /// untouched by any of this and still charge nothing at all.
    fn engage(self, cancel: Option<&CancellationToken>) -> QueryGovernors {
        let mut governors = QueryGovernors::METERED;
        if let Some(fuel) = self.fuel {
            governors = governors.with_fuel(fuel);
        }
        if let Some(rows) = self.max_answers {
            governors = governors.with_max_answers(rows);
        }
        if let Some(cells) = self.max_intermediate_cells {
            governors = governors.with_max_intermediate_cells(cells);
        }
        if let Some(bytes) = self.max_scratch_bytes {
            governors = governors.with_max_scratch_bytes(bytes);
        }
        if let Some(requests) = self.max_remote_requests {
            governors = governors.with_max_remote_requests(requests);
        }
        let watch = WasmStopWatch::new(self.deadline_ms, cancel);
        if watch.is_armed() {
            let signal: Arc<dyn StopSignal> = Arc::new(watch);
            governors = governors.with_stop_signal(signal);
        }
        governors
    }
}

/// Decode one ceiling from the JavaScript boundary.
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
fn decode_ceiling(name: &str, value: Option<i64>) -> Result<Option<u64>, JsError> {
    match value {
        None => Ok(None),
        Some(raw) if raw < 0 => Err(JsError::new(&format!(
            "governor ceiling `{name}` must be a non-negative integer, got {raw} \
             (omit it to decline the ceiling; 0 is a valid ceiling that trips on the \
             first charged unit of work)"
        ))),
        Some(raw) => Ok(Some(raw.unsigned_abs())),
    }
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
#[wasm_bindgen]
pub struct QueryEngine {
    inner: NativeSparqlEngine,
}

impl std::fmt::Debug for QueryEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryEngine").finish_non_exhaustive()
    }
}

#[wasm_bindgen]
impl QueryEngine {
    /// Create a reusable offline SPARQL engine.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: NativeSparqlEngine::new(),
        }
    }

    /// Run any SPARQL query and return a typed raw wasm result wrapper.
    #[wasm_bindgen(js_name = query)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn query(
        &self,
        dataset: &Dataset,
        sparql: &str,
        base: Option<String>,
    ) -> Result<QueryResult, JsError> {
        let result = self.run_query(dataset, sparql, base.as_deref())?;
        query_result_from_sparql(result)
    }

    /// Run a SELECT query and return typed rows.
    #[wasm_bindgen(js_name = select)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn select(
        &self,
        dataset: &Dataset,
        sparql: &str,
        base: Option<String>,
    ) -> Result<SelectResult, JsError> {
        let result = self.run_query(dataset, sparql, base.as_deref())?;
        select_result_from_sparql(result)
    }

    /// Run an ASK query and return the boolean result.
    #[wasm_bindgen(js_name = ask)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn ask(
        &self,
        dataset: &Dataset,
        sparql: &str,
        base: Option<String>,
    ) -> Result<bool, JsError> {
        match self.run_query(dataset, sparql, base.as_deref())? {
            SparqlResult::Boolean(value) => Ok(value),
            other => Err(kind_mismatch("ASK boolean", &other)),
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
    ) -> Result<Dataset, JsError> {
        graph_result_from_sparql(self.run_query(dataset, sparql, base.as_deref())?)
    }

    /// Run a DESCRIBE query and return its result dataset.
    #[wasm_bindgen(js_name = describe)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn describe(
        &self,
        dataset: &Dataset,
        sparql: &str,
        base: Option<String>,
    ) -> Result<Dataset, JsError> {
        graph_result_from_sparql(self.run_query(dataset, sparql, base.as_deref())?)
    }

    /// Apply a SPARQL UPDATE atomically to the supplied dataset.
    #[wasm_bindgen(js_name = update)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn update(
        &self,
        dataset: &mut Dataset,
        sparql: &str,
        base: Option<String>,
    ) -> Result<(), JsError> {
        let mut frozen = dataset.inner.freeze().map_err(|e| diag_to_err(&e))?;
        self.inner
            .update(&mut frozen, sparql_request(sparql, base.as_deref()))
            .map_err(|e| diag_to_err(&e))?;
        dataset.inner = MutableDataset::new(frozen);
        Ok(())
    }

    /// Run any SPARQL query and serialize its raw result.
    ///
    /// `provenance_prefix`/`provenance_iri` (both `undefined`, or both a string) anchor
    /// the additive `purrdf` provenance extension on a SELECT/ASK result serialized to
    /// SPARQL-results JSON/XML, under that `PREFIX`/`IRI`. `undefined` (the default)
    /// leaves the output pure W3C, exactly as before these parameters existed; a
    /// CONSTRUCT/DESCRIBE graph result and CSV/TSV never carry the extension. Read it
    /// back with `provenanceFromJson`/`provenanceFromXml` under the SAME namespace.
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
    ) -> Result<String, JsError> {
        let result = self.run_query(dataset, sparql, base.as_deref())?;
        let namespace = build_provenance_namespace(provenance_prefix, provenance_iri)?;
        serialize_query_result(&result, format.as_deref(), namespace.as_ref(), sparql)
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
    ) -> Result<String, JsError> {
        let options = decode_options(options_json)?;
        self.query_raw_with_options(dataset, sparql, base.as_deref(), format, &options)
    }

    /// Run a SPARQL query under caller-supplied execution governors, returning a
    /// [`QueryOutcome`] rather than the answers directly.
    ///
    /// Every ceiling is optional and `undefined` means "no ceiling on that dimension":
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
    /// `aggregate_namespace` registers purrdf's first-party statistical aggregate set
    /// (`MEDIAN`, `PERCENTILE`, `STDDEV`, `STDDEV_POP`, `VARIANCE`, `VAR_POP`, `MODE`,
    /// `FIRST`, `LAST`, `TOPK`) under that IRI namespace, so the query text can call
    /// `AGG(<{NAMESPACE}NAME>, args…)` (see `build_aggregates`). `None` (the default)
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
    ) -> Result<QueryOutcome, JsError> {
        let args = GovernorArgs {
            fuel: decode_ceiling("fuel", fuel)?,
            deadline_ms: decode_ceiling("deadlineMs", deadline_ms)?,
            max_answers: decode_ceiling("maxAnswers", max_answers)?,
            max_intermediate_cells: decode_ceiling("maxIntermediateCells", max_intermediate_cells)?,
            max_scratch_bytes: decode_ceiling("maxScratchBytes", max_scratch_bytes)?,
            max_remote_requests: decode_ceiling("maxRemoteRequests", max_remote_requests)?,
        };
        let governors = args.engage(cancel.as_ref());
        let frozen = dataset.inner.freeze().map_err(|e| diag_to_err(&e))?;
        let aggregates = build_aggregates(aggregate_namespace);
        let outcome = self
            .inner
            .query_governed(
                &frozen,
                sparql_request(sparql, base.as_deref()),
                QueryOptions {
                    aggregates: aggregates.as_ref().unwrap_or(&AggregateRegistry::EMPTY),
                    ..QueryOptions::EMPTY
                },
                &governors,
            )
            .map_err(|e| diag_to_err(&e))?;
        query_outcome_from_governed(outcome)
    }

    /// Run a governed SPARQL query over a closure produced by `regime`, carrying the
    /// closure report and query outcome together.
    ///
    /// `aggregate_namespace` behaves exactly as on [`Self::query_governed`]: it registers
    /// purrdf's first-party statistical aggregate set under that IRI namespace for the
    /// closure query's PARSE and its evaluation, so `AGG(<{NAMESPACE}NAME>, args…)` reaches
    /// the entailment-aware lane exactly as it reaches the ordinary one. `undefined` (the
    /// default) leaves every one of the ten names an ordinary unregistered custom-aggregate
    /// IRI.
    ///
    /// # Errors
    ///
    /// An invalid regime/program, query parse/evaluation failure, entailment failure, or
    /// malformed ceiling. A governor trip is returned in [`EntailmentQueryOutcome`].
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
        aggregate_namespace: Option<String>,
        fuel: Option<i64>,
        deadline_ms: Option<i64>,
        max_answers: Option<i64>,
        max_intermediate_cells: Option<i64>,
        max_scratch_bytes: Option<i64>,
        max_remote_requests: Option<i64>,
        cancel: Option<CancellationToken>,
    ) -> Result<EntailmentQueryOutcome, JsError> {
        let plan = QueryEntailmentPlan::parse(regime, program.as_deref().unwrap_or(""))
            .map_err(|error| JsError::new(&error))?;
        let args = GovernorArgs {
            fuel: decode_ceiling("fuel", fuel)?,
            deadline_ms: decode_ceiling("deadlineMs", deadline_ms)?,
            max_answers: decode_ceiling("maxAnswers", max_answers)?,
            max_intermediate_cells: decode_ceiling("maxIntermediateCells", max_intermediate_cells)?,
            max_scratch_bytes: decode_ceiling("maxScratchBytes", max_scratch_bytes)?,
            max_remote_requests: decode_ceiling("maxRemoteRequests", max_remote_requests)?,
        };
        let governors = args.engage(cancel.as_ref());
        let frozen = dataset.inner.freeze().map_err(|e| diag_to_err(&e))?;
        let aggregates = build_aggregates(aggregate_namespace);
        let outcome = query_with_entailment_governed(
            &self.inner,
            &frozen,
            sparql_request(sparql, base.as_deref()),
            plan.entailment(),
            QueryOptions {
                aggregates: aggregates.as_ref().unwrap_or(&AggregateRegistry::EMPTY),
                ..QueryOptions::EMPTY
            },
            &governors,
        )
        .map_err(|error| JsError::new(&error.to_string()))?;
        entailment_query_outcome_from_native(outcome)
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
    ) -> Result<UpdateOutcome, JsError> {
        if max_answers.is_some() {
            return Err(JsError::new(
                "maxAnswers is not accepted by updateGoverned: an UPDATE has no answer \
                 sequence to bound. Bound the work that computes the request with fuel, \
                 maxIntermediateCells, or maxScratchBytes instead",
            ));
        }
        let args = GovernorArgs {
            fuel: decode_ceiling("fuel", fuel)?,
            deadline_ms: decode_ceiling("deadlineMs", deadline_ms)?,
            max_answers: None,
            max_intermediate_cells: decode_ceiling("maxIntermediateCells", max_intermediate_cells)?,
            max_scratch_bytes: decode_ceiling("maxScratchBytes", max_scratch_bytes)?,
            max_remote_requests: decode_ceiling("maxRemoteRequests", max_remote_requests)?,
        };
        let governors = args.engage(cancel.as_ref());
        let mut frozen = dataset.inner.freeze().map_err(|e| diag_to_err(&e))?;
        let aggregates = build_aggregates(aggregate_namespace);
        let outcome = self
            .inner
            .update_governed(
                &mut frozen,
                sparql_request(sparql, base.as_deref()),
                QueryOptions {
                    aggregates: aggregates.as_ref().unwrap_or(&AggregateRegistry::EMPTY),
                    ..QueryOptions::EMPTY
                },
                &governors,
            )
            .map_err(|e| diag_to_err(&e))?;
        // The engine publishes into its own `Arc` only on the applied path, so adopting
        // the returned base on a trip would adopt a base nothing was written to.
        if outcome.is_applied() {
            dataset.inner = MutableDataset::new(frozen);
        }
        Ok(update_outcome_from_governed(&outcome))
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
    ) -> Result<String, JsError> {
        let frozen = dataset.inner.freeze().map_err(|e| diag_to_err(&e))?;
        Ok(self
            .inner
            .explain_query(&frozen, sparql, base.as_deref())
            .map_err(|e| diag_to_err(&e))?
            .render())
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
    ) -> Result<String, JsError> {
        let mut options = context_options(context);
        if let Some(url) = yaml_schema_url {
            options = options
                .with_yaml_schema_url(&url)
                .map_err(|error| JsError::new(&error.to_string()))?;
        }
        self.query_raw_with_options(dataset, sparql, base.as_deref(), format, &options)
    }
}

impl Default for QueryEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryEngine {
    fn run_query(
        &self,
        dataset: &Dataset,
        sparql: &str,
        base: Option<&str>,
    ) -> Result<SparqlResult, JsError> {
        let frozen = dataset.inner.freeze().map_err(|e| diag_to_err(&e))?;
        self.inner
            .query(&frozen, sparql_request(sparql, base))
            .map_err(|e| diag_to_err(&e))
    }

    fn query_raw_with_options(
        &self,
        dataset: &Dataset,
        sparql: &str,
        base: Option<&str>,
        format: &str,
        options: &JsonLdSerializeOptions,
    ) -> Result<String, JsError> {
        match self.run_query(dataset, sparql, base)? {
            SparqlResult::Graph(graph) => Dataset {
                inner: MutableDataset::new(graph),
            }
            // No egress base, and `base` above is deliberately NOT reused as one. That
            // parameter is the SPARQL *query* base: it resolves relative IRI references
            // inside the query TEXT. The document base a result is WRITTEN under is a
            // different thing, and a query that happened to need one to parse is no
            // evidence about how its answer should be spelled. Passing it here would
            // silently relativize the result against the query's base.
            .serialize_with_options(format, options, None),
            other => Err(kind_mismatch(
                "CONSTRUCT/DESCRIBE graph for configured JSON-LD serialization",
                &other,
            )),
        }
    }
}

#[wasm_bindgen]
impl Dataset {
    /// `query(sparql, base?)` → run a SPARQL query against this dataset, offline.
    ///
    /// Returns **SPARQL Results JSON** for SELECT / ASK and **Turtle** for
    /// CONSTRUCT / DESCRIBE. A parse error, an evaluation error, or a `SERVICE` /
    /// `LOAD` clause (unresolvable in-browser) throws a JsError — never a silent
    /// empty result.
    #[wasm_bindgen(js_name = query)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn query(&self, sparql: &str, base: Option<String>) -> Result<String, JsError> {
        QueryEngine::new().query_raw(self, sparql, base, None, None, None)
    }
}

fn sparql_request<'a>(sparql: &'a str, base: Option<&'a str>) -> SparqlRequest<'a> {
    SparqlRequest {
        query: sparql,
        base_iri: base,
        substitutions: &[],
    }
}

/// Build the statistical-aggregate registry `aggregateNamespace` requests, or `None`
/// when the JS caller supplied none.
///
/// This is the ENTIRE wasm surface for purrdf's first-party statistical aggregate set
/// (`MEDIAN`, `PERCENTILE`, `STDDEV`, `STDDEV_POP`, `VARIANCE`, `VAR_POP`, `MODE`,
/// `FIRST`, `LAST`, `TOPK` — see `purrdf_sparql_eval::stat_agg`):
/// `AggregateRegistry::register_statistical_aggregates` takes only an IRI namespace
/// string, so it crosses the wasm boundary with no callback and no per-aggregate
/// marshaling. The GENERAL custom-aggregate seam
/// (`purrdf_sparql_eval::agg_fn::AggregateRegistry::register`, an arbitrary
/// `init`/`step`/`combine`/`finish` closure) is Rust-host-only and has no string-shaped
/// surface at all — it cannot cross into JavaScript — and this crate does not attempt to
/// expose it.
fn build_aggregates(namespace: Option<String>) -> Option<AggregateRegistry> {
    let namespace = namespace?;
    let mut registry = AggregateRegistry::new();
    registry.register_statistical_aggregates(&namespace);
    Some(registry)
}

fn query_result_from_sparql(result: SparqlResult) -> Result<QueryResult, JsError> {
    Ok(match result {
        SparqlResult::Solutions {
            variables, rows, ..
        } => QueryResult {
            kind: QueryResultKind::Select,
            value: Some(QueryResultValue::Select(select_result(variables, rows)?)),
        },
        SparqlResult::Boolean(value) => QueryResult {
            kind: QueryResultKind::Ask,
            value: Some(QueryResultValue::Ask(value)),
        },
        SparqlResult::Graph(graph) => QueryResult {
            kind: QueryResultKind::Graph,
            value: Some(QueryResultValue::Graph(Dataset {
                inner: MutableDataset::new(graph),
            })),
        },
    })
}

fn select_result_from_sparql(result: SparqlResult) -> Result<SelectResult, JsError> {
    match result {
        SparqlResult::Solutions {
            variables, rows, ..
        } => select_result(variables, rows),
        other => Err(kind_mismatch("SELECT solutions", &other)),
    }
}

fn graph_result_from_sparql(result: SparqlResult) -> Result<Dataset, JsError> {
    match result {
        SparqlResult::Graph(graph) => Ok(Dataset {
            inner: MutableDataset::new(graph),
        }),
        other => Err(kind_mismatch("CONSTRUCT/DESCRIBE graph", &other)),
    }
}

/// Convert a native governed outcome into the JS-facing [`QueryOutcome`] object.
///
/// Both arms produce an object; neither produces an error. That asymmetry with
/// `Result` is the whole point of the type.
fn query_outcome_from_governed(outcome: GovernedOutcome) -> Result<QueryOutcome, JsError> {
    match outcome {
        GovernedOutcome::Complete {
            result, evidence, ..
        } => Ok(QueryOutcome {
            result: Some(query_result_from_sparql(result)?),
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
            partial: Some(partial_answers_from_native(partial)?),
            tripped: Some(TrippedGovernor { inner: tripped }),
            evidence: Some(GovernorEvidence { inner: evidence }),
            complete: false,
        }),
    }
}

fn entailment_query_outcome_from_native(
    outcome: GovernedEntailment,
) -> Result<EntailmentQueryOutcome, JsError> {
    match outcome {
        GovernedEntailment::Answered { outcome, report } => {
            let tripped = outcome.tripped().map(|inner| TrippedGovernor { inner });
            Ok(EntailmentQueryOutcome {
                complete: tripped.is_none(),
                outcome: Some(query_outcome_from_governed(outcome)?),
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
fn update_outcome_from_governed(outcome: &GovernedUpdateOutcome) -> UpdateOutcome {
    UpdateOutcome {
        tripped: outcome.tripped().map(|inner| TrippedGovernor { inner }),
        evidence: Some(GovernorEvidence {
            inner: outcome.evidence().clone(),
        }),
        applied: outcome.is_applied(),
    }
}

/// Convert the evaluator's certificate into the JS-facing [`PartialAnswers`] object.
fn partial_answers_from_native(partial: PartialValue) -> Result<PartialAnswers, JsError> {
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
                Some(query_result_from_sparql(rows.into_result())?),
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
) -> Result<SelectResult, JsError> {
    let variables: Rc<[String]> = Rc::from(variables.into_boxed_slice());
    let rows: Vec<Option<SelectRow>> = rows
        .into_iter()
        .map(|row| select_row(Rc::clone(&variables), row).map(Some))
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
) -> Result<SelectRow, JsError> {
    let values = row
        .into_iter()
        .map(|value| {
            value
                .map(term_from_value)
                .transpose()
                .map_err(|e| JsError::new(&e))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SelectRow { variables, values })
}

fn term_from_value(value: purrdf::TermValue) -> Result<Term, String> {
    let term = term_value_into_rdf_term(value)?;
    Ok(Term::from_canonical_rdf_term(term))
}

/// Decode `provenance_prefix`/`provenance_iri` (both `None`, or both `Some`) into an
/// optional [`purrdf_sparql_results::ProvenanceNamespace`]. Exactly one `Some` is a
/// usage error: a namespace needs both halves, and silently treating a lone prefix or
/// IRI as "no namespace" would be the exact silent-drop this binding refuses elsewhere.
fn build_provenance_namespace(
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

/// Build the [`ResultProvenance`] a tabular emission carries: empty when no namespace
/// was supplied (pure-W3C output), or populated with a content hash of the query text
/// plus this engine's label when one was. Mirrors the CLI's and C ABI's
/// `build_query_provenance`. `solutions` stays empty: per-solution source provenance is
/// the evaluator/S11 derivation graph's progressive fill, not something this binding can
/// populate on its own.
fn build_query_provenance(
    namespace: Option<&purrdf_sparql_results::ProvenanceNamespace>,
    query: &str,
) -> ResultProvenance {
    use sha2::{Digest as _, Sha256};

    if namespace.is_none() {
        return ResultProvenance::default();
    }
    let digest = Sha256::digest(query.as_bytes());
    ResultProvenance {
        query_hash: Some(format!("sha256:{digest:x}")),
        engine: Some("purrdf-sparql-eval".to_owned()),
        solutions: Vec::new(),
    }
}

fn serialize_query_result(
    result: &SparqlResult,
    format: Option<&str>,
    provenance_namespace: Option<&purrdf_sparql_results::ProvenanceNamespace>,
    query: &str,
) -> Result<String, JsError> {
    match result {
        SparqlResult::Graph(graph) => serialize_graph_result(graph, format.unwrap_or("turtle")),
        SparqlResult::Solutions { .. } | SparqlResult::Boolean(_) => {
            let results_format = match format {
                None => SparqlResultsFormat::Json,
                Some(format) => resolve_results_format(format)?,
            };
            serialize_tabular_result(result, results_format, provenance_namespace, query)
        }
    }
}

fn resolve_results_format(format: &str) -> Result<SparqlResultsFormat, JsError> {
    let normalized = format.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "json" | "srj" | "sparql-json" | "application/sparql-results+json" => {
            Ok(SparqlResultsFormat::Json)
        }
        "xml" | "sparql-xml" | "application/sparql-results+xml" => Ok(SparqlResultsFormat::Xml),
        "csv" | "text/csv" => Ok(SparqlResultsFormat::Csv),
        "tsv" | "text/tab-separated-values" => Ok(SparqlResultsFormat::Tsv),
        other => Err(JsError::new(&format!(
            "unsupported SPARQL results format {other:?} \
             (use json/xml/csv/tsv or graph formats for CONSTRUCT/DESCRIBE)"
        ))),
    }
}

fn serialize_tabular_result(
    result: &SparqlResult,
    format: SparqlResultsFormat,
    provenance_namespace: Option<&purrdf_sparql_results::ProvenanceNamespace>,
    query: &str,
) -> Result<String, JsError> {
    let provenance = build_query_provenance(provenance_namespace, query);
    let outcome = serialize_results(result, format, &provenance, provenance_namespace)
        .map_err(|e| JsError::new(&e.to_string()))?;
    String::from_utf8(outcome.bytes)
        .map_err(|e| JsError::new(&format!("SPARQL result is not valid UTF-8: {e}")))
}

fn serialize_graph_result(
    graph: &Arc<purrdf::RdfDataset>,
    format: &str,
) -> Result<String, JsError> {
    let media_type = resolve_media_type(format).map_err(|e| JsError::new(&e))?;
    let bytes = serialize_dataset(graph, media_type, SerializeGraph::Dataset)
        .map_err(|e| diag_to_err(&e))?;
    String::from_utf8(bytes)
        .map_err(|e| JsError::new(&format!("SPARQL graph result is not valid UTF-8: {e}")))
}

fn kind_mismatch(expected: &str, actual: &SparqlResult) -> JsError {
    JsError::new(&format!(
        "expected {expected}, got {}",
        sparql_result_kind(actual)
    ))
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
        let mut result = select_result(vec!["value".to_owned()], rows).expect("select result");

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
}
