// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// The JavaScript half of the asynchronous operation runtime: the suspending import the
// wasm module calls, and the per-instance scheduler that drives asynchronous jobs.
//
// The wasm-bindgen glue imports this module by relative path (`./purrdf_jspi.mjs`) and
// hands `purrdf_jspi_suspend` (and `purrdf_jspi_panicked`, which the panic hook calls)
// to the instance as raw imports, so this file ships next to the glue in `pkg/`. It
// imports nothing: the package root passes the instance's exports in through
// `installAsync` once `ready()` has instantiated it.
//
// # The protocol (the Rust side is `crates/rdf-wasm/src/async_query.rs`)
//
// A job is begun in Rust (`QueryEngine.beginAsync`) and run here (`runJob`) through the
// raw export `purrdf_jspi_run`, wrapped by `WebAssembly.promising`. Every effect the job
// performs — a `SERVICE` request, a wait on another job's identical request, a `LOAD`
// document, a turn of the event loop — posts a ticket in Rust and calls
// `purrdf_jspi_suspend(job, seq, out)`, which is `suspendImpl` below wrapped in
// `WebAssembly.Suspending`. `suspendImpl` takes the ticket, awaits the host's answer,
// delivers it, writes `0` to the `u32` at `out` and returns a `SuspendStatus`.
//
// Every decision is Rust's: which jobs share a `SERVICE` host call, what a job answers
// from its memo, when an awaited effect is abandoned and whether that is the deadline or
// the request's own timeout, whether a `LOAD` source (and every redirect it names) is
// authorized. This module moves an effect to the host's handler and the answer back, and
// keeps the one table Rust cannot: each open shared call's promise and `AbortController`.
//
// # Nothing crosses a suspended frame
//
// A JavaScript exception thrown into a suspended wasm frame bricks the instance, so
// `suspendImpl` never throws and never rejects. Every condition becomes a status, under
// one rule: a handler's failure is that effect's failure, and a broken delivery protocol
// is the job's fault.
//
// A handler that throws, rejects, returns a value the protocol does not define, or names
// a failure kind it does not define has failed the invocation it was answering. That
// failure is delivered to the effect with kind `"fault"`, beside `"transport"` (the
// endpoint could not be reached or read) and `"denied"` (the host's policy refused it),
// and the clause that issued the effect decides what it means: a `SERVICE SILENT` or
// `LOAD SILENT` answers it with the join identity, or loads nothing, and records a
// silenced invocation of kind `"fault"`; without `SILENT` the request fails with
// `native-sparql-host-fault` (`SERVICE`) or `native-sparql-load-fault` (`LOAD`). Only a
// protocol violation — a status wasm cannot have sent, an effect suspended on without a
// ticket, an exchange no call is answering, a delivery the job refuses — is latched on the
// job with `job.fault(...)`, which no clause can absorb.
//
// # The linked module
//
// The module `make wasm-pkg` ships is rewritten after `wasm-opt` by `crates/wasm-link`,
// which makes two obligations of this lane properties of the wasm itself. This module
// never moves the shadow-stack pointer and never wraps an export:
//
// - `purrdf_jspi_run(job, regionTop)` runs a job on the region whose top is its last
//   argument. The linked wrapper records the caller's stack pointer as the idle one, sets
//   the pointer to the region top, and puts the idle pointer back before returning; the
//   linked `$suspend`, which every call of the suspending import goes through, parks the
//   job on the idle pointer and restores the job's own pointer when the import returns.
//   A suspended job's frames stay live in linear memory on its region, and every other
//   call — a synchronous one between resumptions, a job started from inside a sink
//   callback — runs on the pointer it found.
// - Every exported function stands behind the poison gate. Its `WebAssembly.Global`s are
//   exported and this module reads them: `purrdf_poisoned` (non-zero once the instance
//   may not be entered again), `purrdf_active` (entries that have not returned),
//   `purrdf_parked` (suspensions waiting on this module) and `purrdf_outbound` (import
//   calls, a sink callback's included, waiting to return), beside `purrdf_stack_pointer`
//   and `purrdf_idle`. Whenever JavaScript runs with no wasm frame unwound,
//   `active − parked = outbound`. A trap, or a JavaScript exception thrown through wasm
//   frames, unwinds an entry without its exit, so the next entry sees the imbalance,
//   sets `purrdf_poisoned` and traps; from then on every entry traps. A call the glue or
//   a sink callback makes back into the instance from inside an import passes, because
//   the import's trampoline counted it in `purrdf_outbound`.
//
// This module's part: it passes the region top; it sets `purrdf_poisoned` when it learns
// of a fault first (a Rust panic's hook, a run whose promise rejected); it treats a
// `WebAssembly.RuntimeError` out of an export while the gate is closed as the gate's
// refusal and answers with the poison error; and it never resumes a suspended run of a
// poisoned instance.

/** Thrown by every asynchronous method on an engine without JSPI. */
export const NO_JSPI_MESSAGE =
  "asynchronous queries need WebAssembly JavaScript Promise Integration " +
  "(WebAssembly.Suspending and WebAssembly.promising), which this JavaScript engine " +
  "does not provide; the synchronous API is unaffected";

const NO_YIELD_MESSAGE =
  "asynchronous queries yield to the event loop through setTimeout, and this JavaScript " +
  "environment provides no globalThis.setTimeout; the synchronous API is unaffected";

const NOT_INSTALLED_MESSAGE =
  "the asynchronous runtime is not installed; await ready() before any asynchronous call";

const DEFAULT_MAX_CONCURRENT_JOBS = 16;

const HAS_JSPI =
  typeof WebAssembly === "object" &&
  typeof WebAssembly.Suspending === "function" &&
  typeof WebAssembly.promising === "function";

// The `WebAssembly.Global`s the linker exports; `installAsync` requires every one.
const GATE_GLOBALS = [
  "purrdf_stack_pointer",
  "purrdf_idle",
  "purrdf_poisoned",
  "purrdf_active",
  "purrdf_parked",
  "purrdf_outbound",
];

// The poison reason when this module learns of the poisoning from the gate's globals
// rather than from an error in its hands: some earlier entry was unwound and never
// returned.
const GATE_CLOSED_REASON =
  "a call into the instance was unwound by a trap or a thrown exception, and its poison gate closed";

// ---------------------------------------------------------------------------
// Instance state (one module instance per wasm instance: the glue imports this module
// once, and the package root instantiates the wasm once)
// ---------------------------------------------------------------------------

// { exports, promisingRun, AsyncJob, RunStatus, SuspendStatus, DeliveryStatus,
//   EffectKind }: the instance and the protocol's classes and statuses
let installed = null;
let poisonReason = null;
let maxConcurrentJobs = DEFAULT_MAX_CONCURRENT_JOBS;
const records = new Map(); // job id -> record
// Shared SERVICE exchange id -> { promise, controller, settled }: the host call every job
// waiting on the exchange awaits. Which jobs wait, and when the call is abandoned, is
// Rust's registry; this is the one table Rust cannot hold.
const exchanges = new Map();

const encoder = new TextEncoder();

// ---------------------------------------------------------------------------
// The yield primitive
// ---------------------------------------------------------------------------

// Every yield is one `setTimeout(…, 0)` task, on every host. A timer task queues behind
// the tasks already waiting — other requests, timers, network responses — so a job that
// keeps yielding lets each of them run between its turns. The global is bound once, when
// this module loads; a host without one has no asynchronous lane (`hasAsyncQueries()` is
// `false`, and every asynchronous call refuses with `NO_YIELD_MESSAGE`).
const setTimeoutImpl = typeof globalThis.setTimeout === "function" ? globalThis.setTimeout : null;

/** One turn of the event loop: resolves from a `setTimeout(…, 0)` task. */
function yieldOnce() {
  return new Promise((resolve) => setTimeoutImpl(resolve, 0));
}

// ---------------------------------------------------------------------------
// Public API (consumed by the package root)
// ---------------------------------------------------------------------------

/**
 * The raw import the instance's panic hook calls (`crates/rdf-wasm/src/panic_poison.rs`)
 * before a Rust panic aborts: `reason` points at `len` bytes of UTF-8 describing the
 * panic. The panic's trap is about to unwind the call — a synchronous call as much as an
 * asynchronous job — and leave the instance's state half-changed, so the instance is
 * poisoned here, naming the panic, before the trap reaches any JavaScript that could call
 * in again: `purrdf_poisoned` is set, every job in flight rejects, and the gate refuses
 * every later entry.
 *
 * This runs inside the panicking wasm frame, so it never calls into the instance and
 * never throws: an exception thrown into that frame would unwind it instead of the trap,
 * and a failure to read the reason still poisons.
 */
export function purrdf_jspi_panicked(reason, len) {
  let text = "a Rust panic";
  try {
    const memory = installed?.exports.memory;
    if (memory instanceof WebAssembly.Memory) {
      text = new TextDecoder().decode(new Uint8Array(memory.buffer, reason >>> 0, len >>> 0).slice());
    }
  } catch {
    // Keep the generic reason: the poisoning is what matters.
  }
  try {
    poison(text);
  } catch {
    // Nothing may cross the panicking frame; the trap that follows still reaches the
    // caller.
  }
}

/**
 * Install the scheduler over the instance's raw exports (the object the glue's `init`
 * returns) and the protocol's glue classes — `{ AsyncJob, RunStatus, SuspendStatus,
 * DeliveryStatus, EffectKind }`, the statuses the Rust side exports. Called once by
 * `ready()`. Throws when the exports lack the runtime's entry points or any of the
 * linker's globals — an artifact built without the asynchronous lane, or not linked by
 * `wasm-link`, is a build defect, not a mode — or when a protocol class is missing.
 */
export function installAsync(exports, protocol) {
  if (installed !== null) {
    if (installed.exports === exports) return;
    throw new Error("the asynchronous runtime is already installed over another instance");
  }
  if (exports == null || typeof exports !== "object") {
    throw new TypeError("installAsync expects the wasm instance's exports");
  }
  for (const name of ["memory", "purrdf_jspi_run"]) {
    if (!(name in exports)) {
      throw new Error(`the wasm instance does not export ${name}; the package artifact is incomplete`);
    }
  }
  for (const name of GATE_GLOBALS) {
    if (!(exports[name] instanceof WebAssembly.Global)) {
      throw new Error(
        `the wasm instance does not export the WebAssembly.Global ${name}; ` +
          "the package artifact was not linked by wasm-link (make wasm-pkg)",
      );
    }
  }
  for (const name of ["AsyncJob", "RunStatus", "SuspendStatus", "DeliveryStatus", "EffectKind"]) {
    if (protocol?.[name] === undefined) {
      throw new Error(`installAsync needs the protocol's ${name}; the package root passes it`);
    }
  }
  installed = {
    exports,
    promisingRun: HAS_JSPI ? WebAssembly.promising(exports.purrdf_jspi_run) : null,
    AsyncJob: protocol.AsyncJob,
    RunStatus: protocol.RunStatus,
    SuspendStatus: protocol.SuspendStatus,
    DeliveryStatus: protocol.DeliveryStatus,
    EffectKind: protocol.EffectKind,
  };
  // A panic reported before the exports were installed is written to the gate now.
  if (poisonReason !== null) exports.purrdf_poisoned.value = 1;
}

/** Whether this engine can run asynchronous jobs: JSPI and `setTimeout` exist. */
export function hasAsyncQueries() {
  return HAS_JSPI && setTimeoutImpl !== null;
}

/**
 * Throw the one clear error that explains why an asynchronous call cannot run here, or
 * return when it can: no JSPI, no `setTimeout`, not installed, or a poisoned instance.
 */
export function assertAsyncQueries() {
  if (!HAS_JSPI) throw new Error(NO_JSPI_MESSAGE);
  if (setTimeoutImpl === null) throw new Error(NO_YIELD_MESSAGE);
  if (installed === null) throw new Error(NOT_INSTALLED_MESSAGE);
  assertNotPoisoned();
}

/**
 * Throw the poison error when the instance is poisoned, or return. The package root's
 * entry points call this first, so an asynchronous twin, `ready()` or `configureAsync`
 * answers a poisoned instance with the poison error rather than the gate's trap; a
 * synchronous call into the instance meets the gate itself. The gate's globals are read
 * here, so a trap out of a synchronous call that no JavaScript of this module saw is
 * learnt of at the next call: the instance is poisoned, every job in flight rejects, and
 * the reason names the gate.
 */
export function assertNotPoisoned() {
  if (poisonReason === null && gateClosed()) poison(GATE_CLOSED_REASON);
  if (poisonReason !== null) throw poisonError();
}

/**
 * Configure the scheduler. `maxConcurrentJobs` (an integer ≥ 1; default 16) bounds how
 * many jobs may be in flight at once on this instance. Unknown keys are refused.
 */
export function configureAsync(options) {
  if (options == null || typeof options !== "object" || Array.isArray(options)) {
    throw new TypeError("configureAsync expects an options object");
  }
  for (const key of Object.keys(options)) {
    if (key !== "maxConcurrentJobs") {
      throw new TypeError(`unknown configureAsync option ${JSON.stringify(key)} (expected maxConcurrentJobs)`);
    }
  }
  if (options.maxConcurrentJobs !== undefined) {
    const value = options.maxConcurrentJobs;
    if (!Number.isSafeInteger(value) || value < 1) {
      throw new RangeError(`maxConcurrentJobs must be an integer ≥ 1, got ${String(value)}`);
    }
    maxConcurrentJobs = value;
  }
}

/**
 * Run a begun job (an `AsyncJob` from `QueryEngine.beginAsync`) to completion.
 *
 * `host` carries the job's effect handlers and its stop source:
 * - `resolveService(request, ctx)` — answers a `SERVICE` effect. `request` is
 *   `{ kind: "service", endpoint, queryText, accept, contentType, userAgent, timeoutMs,
 *   headers, cacheable }` (`headers`: `[name, value]` pairs, in sending order;
 *   `cacheable`: whether a shared cache may answer it); `ctx` is `{ signal,
 *   remainingDeadlineMs, silent, maxIntermediateCells }`. It returns (or resolves to)
 *   SPARQL Results JSON as a `Uint8Array`, `ArrayBuffer` or `string`; a `Response` (a
 *   non-ok status is a transport failure and its body is cancelled); or
 *   `{ kind: "transport" | "denied" | "fault", message }`. A throw, a rejection or any
 *   other value is the handler's fault, delivered to the effect as a `"fault"` failure:
 *   the invocation failed, and the clause that issued it decides whether `SILENT`
 *   absorbs it. The job answers a request it repeats from its own memo, and concurrent
 *   jobs share one call when Rust says they may; `ctx.signal` is the shared call's, which
 *   aborts once no job waits on it.
 * - `resolveLoad(request, ctx)` — answers one hop of a `LOAD`. `request` is
 *   `{ kind: "load", iri, accept, userAgent, headers, timeoutMs }`, `ctx` is
 *   `{ signal }`. It returns `{ bytes | text, mediaType, base? }` (`base` defaults to the
 *   IRI), a `Response` (its `Content-Type` names the media type; its URL, or the IRI, is
 *   the base), a `Dataset`, `{ kind: "redirect", location }` (the job resolves the
 *   location, authorizes it and asks again), or a typed failure as above.
 * - `signal` — an `AbortSignal` that cancels the job.
 *
 * Every `SERVICE` and `LOAD` effect names the instant it is abandoned at if still
 * unanswered (`effect.abandonAfterMs`, Rust's: the request's timeout or the job's
 * deadline); the handler's `ctx.signal` aborts then too.
 *
 * Resolves to the run's `RunStatus`: an outcome or an error is stored (read
 * `job.errorKind`, `job.errorCode` and `job.errorMessage`), the job is unknown, or it was
 * already started.
 * Rejects only when the call cannot run at all (see `assertAsyncQueries`), when
 * `maxConcurrentJobs` jobs are already in flight, for a malformed `host`, or when the
 * instance traps or the job's frames ran past its region's overrun zone — either of
 * which poisons it. The job stays the caller's to `finish()` and
 * `free()` in every case.
 */
export async function runJob(job, host = {}) {
  assertAsyncQueries();
  const handlers = validateHost(host);
  if (records.size >= maxConcurrentJobs) {
    throw new Error(
      `too many asynchronous jobs in flight (the limit is ${maxConcurrentJobs}); ` +
        "await one, or raise the limit with configureAsync({ maxConcurrentJobs })",
    );
  }
  const id = job.id;
  if (records.has(id)) {
    throw new Error(`asynchronous job ${id} is already running`);
  }
  const record = newRecord(job, handlers);
  records.set(id, record);
  try {
    return await startRun(record);
  } finally {
    record.dispose();
    records.delete(id);
  }
}

// ---------------------------------------------------------------------------
// The suspending import
// ---------------------------------------------------------------------------

/**
 * The raw import the wasm module suspends through. A plain throwing function when the
 * engine lacks JSPI, so the module (and the synchronous API) still loads there; no job
 * can reach it, because `runJob` refuses first.
 */
export const purrdf_jspi_suspend = HAS_JSPI
  ? new WebAssembly.Suspending(suspendImpl)
  : function purrdf_jspi_suspend() {
      throw new Error(NO_JSPI_MESSAGE);
    };

async function suspendImpl(rawJob, rawSeq, rawOut) {
  // Raw wasm `i32` arguments arrive signed; the protocol's values are `u32`.
  const jobId = rawJob >>> 0;
  const seq = rawSeq >>> 0;
  const outPtr = rawOut >>> 0;
  const record = records.get(jobId);
  if (record === undefined) {
    // A run the scheduler did not start (or one abandoned by a poisoning): nothing can
    // be trusted about its state, so it is never resumed.
    if (poisonReason === null) {
      poison(`purrdf_jspi_suspend named job ${jobId}, which the scheduler is not running`);
    }
    return never();
  }
  let status;
  try {
    status = await answer(record, seq);
  } catch (error) {
    // A poisoned instance refuses every call, `job.fault` included, and its suspended
    // runs are never resumed.
    if (poisonedBy(error) || poisonReason !== null) return never();
    // `answer` catches everything itself; this is the last line that keeps a rejection
    // out of the suspended frame.
    status = latchFault(record, `the asynchronous bridge failed: ${describe(error)}`);
  }
  // A trap out of a synchronous call while this job was parked closed the gate without
  // this module seeing it: the run is never resumed, and the job rejects with the poison.
  if (poisonReason === null && gateClosed()) poison(GATE_CLOSED_REASON);
  if (poisonReason !== null) return never();
  new DataView(installed.exports.memory.buffer).setUint32(outPtr, 0, true);
  return status;
}

/** Answer effect `seq` of `record`'s job; resolves to the status for wasm. */
async function answer(record, seq) {
  const { job } = record;
  const { SuspendStatus, EffectKind } = installed;
  // The canary at the region's base, read before any await or wasm call.
  const memory = new DataView(installed.exports.memory.buffer);
  const canaryIntact = memory.getUint32(record.base, true) === record.canary;
  if (!canaryIntact) {
    const reason = `asynchronous job ${record.id} overwrote the canary at the base of its stack region`;
    latchFault(record, reason);
    poison(reason);
    return SuspendStatus.Fault;
  }
  const effect = job.takeEffect();
  if (effect === undefined) {
    return latchFault(record, `effect ${seq} was suspended on without a posted ticket`);
  }
  try {
    if (effect.seq !== seq) {
      return latchFault(record, `the posted effect is ${effect.seq}, but the job suspended on ${seq}`);
    }
    switch (effect.kind) {
      case EffectKind.Yield:
        await yieldOnce();
        return SuspendStatus.Answered;
      case EffectKind.Service:
        return await answerService(record, effect);
      case EffectKind.AwaitExchange:
        return await answerAwaitExchange(record, effect);
      case EffectKind.Load:
        return await answerLoad(record, effect);
      default:
        return latchFault(record, `effect ${seq} has unknown kind ${String(effect.kind)}`);
    }
  } finally {
    effect.free();
  }
}

// ---------------------------------------------------------------------------
// Abandonment instants
// ---------------------------------------------------------------------------

const EXPIRED = Symbol("expired");

/**
 * The instant effect `effect` is abandoned at if still unanswered — Rust's
 * `abandonAfterMs`, the request's own timeout or the job's deadline — as a signal and a
 * promise that resolves to `EXPIRED` when it fires.
 */
function expiry(effect) {
  const signal = AbortSignal.timeout(effect.abandonAfterMs);
  const promise = new Promise((resolve) => {
    signal.addEventListener("abort", () => resolve(EXPIRED), { once: true });
  });
  return { signal, promise };
}

// ---------------------------------------------------------------------------
// SERVICE
// ---------------------------------------------------------------------------

/** A `SERVICE` effect opens its shared exchange: one host call, awaited by every job Rust joins to it. */
async function answerService(record, effect) {
  const seq = effect.seq;
  const { resolveService } = record.handlers;
  if (resolveService === undefined) {
    return latchFault(record, `SERVICE effect ${seq} was issued, but no resolveService handler was given`);
  }
  if (record.stop.fired) return abandon(record, seq);
  const id = effect.exchangeId;
  if (exchanges.has(id)) {
    return latchFault(record, `SERVICE effect ${seq} opens exchange ${id}, which is already open`);
  }
  const request = serviceRequest(effect);
  const ctx = {
    remainingDeadlineMs: effect.remainingDeadlineMs,
    silent: effect.silent,
    maxIntermediateCells: effect.maxIntermediateCells,
  };
  const controller = new AbortController();
  const entry = { controller, settled: false, promise: undefined };
  entry.promise = invokeHost(
    "resolveService",
    () => resolveService({ ...request, headers: request.headers.map((pair) => [...pair]) }, { ...ctx, signal: controller.signal }),
    (value) => normalizeService(value, request.endpoint, controller.signal),
  ).then((settled) => settleExchange(id, entry, settled));
  exchanges.set(id, entry);
  return awaitExchange(record, effect, entry);
}

/** A wait on an exchange another job opened. */
function answerAwaitExchange(record, effect) {
  const seq = effect.seq;
  if (record.stop.fired) return abandon(record, seq);
  const id = effect.exchangeId;
  const entry = exchanges.get(id);
  if (entry === undefined) {
    return latchFault(record, `effect ${seq} waits on exchange ${id}, which no call is answering`);
  }
  return awaitExchange(record, effect, entry);
}

/**
 * Wait for exchange `entry`'s answer, the job's stop, or the effect's abandonment
 * instant. Rust delivers the answer to every job still waiting and says which status to
 * resume with; a job that stops or expires leaves, and a call no job waits on any more
 * is aborted.
 */
async function awaitExchange(record, effect, entry) {
  const seq = effect.seq;
  const { job } = record;
  const expired = expiry(effect);
  const winner = await Promise.race([entry.promise, record.stop.promise, expired.promise]);
  let status;
  if (winner === SETTLED) {
    status = job.settledStatus(seq);
  } else if (winner === STOPPED) {
    status = abandon(record, seq);
  } else {
    status = job.expireEffect(seq);
  }
  if (!entry.settled && !installed.AsyncJob.exchangeIsOpen(effect.exchangeId)) {
    exchanges.delete(effect.exchangeId);
    entry.controller.abort(abortReason("every job waiting on this request was stopped", "AbortError"));
  }
  return status;
}

const SETTLED = Symbol("settled");

/** Deliver a shared call's normalized answer to every job Rust still has waiting on it. */
function settleExchange(id, entry, settled) {
  entry.settled = true;
  exchanges.delete(id);
  const { AsyncJob } = installed;
  try {
    switch (settled.type) {
      case "bytes":
        AsyncJob.deliverExchangeBindings(id, settled.bytes);
        break;
      case "failure":
        AsyncJob.deliverExchangeFailure(id, settled.kind, settled.message);
        break;
      default:
        AsyncJob.faultExchange(id, settled.message);
        break;
    }
  } catch (error) {
    // A poisoned instance refuses every call; the poisoning already rejected every job.
    if (poisonedBy(error) || poisonReason !== null) return SETTLED;
    throw error;
  }
  return SETTLED;
}

function serviceRequest(effect) {
  const flat = effect.headers();
  const headers = [];
  for (let index = 0; index + 1 < flat.length; index += 2) {
    headers.push([flat[index], flat[index + 1]]);
  }
  return {
    kind: "service",
    endpoint: effect.endpoint,
    queryText: effect.queryText,
    accept: effect.accept,
    contentType: effect.contentType,
    userAgent: effect.userAgent,
    timeoutMs: effect.timeoutMs,
    headers,
    cacheable: effect.cacheable,
  };
}

async function normalizeService(value, endpoint, signal) {
  if (value instanceof Uint8Array) return { type: "bytes", bytes: value };
  if (value instanceof ArrayBuffer) return { type: "bytes", bytes: new Uint8Array(value) };
  if (typeof value === "string") return { type: "bytes", bytes: encoder.encode(value) };
  if (isResponseLike(value)) {
    if (!value.ok) {
      await cancelBody(value);
      return { type: "failure", kind: "transport", message: `HTTP ${value.status} from ${endpoint}` };
    }
    return readBody(value, endpoint, signal, (bytes) => ({ type: "bytes", bytes }));
  }
  const failure = typedFailure(value, "resolveService");
  if (failure !== undefined) return failure;
  return fault(`resolveService returned an unrecognized value (${kindOf(value)}) for ${endpoint}`);
}

// ---------------------------------------------------------------------------
// LOAD
// ---------------------------------------------------------------------------

async function answerLoad(record, effect) {
  const seq = effect.seq;
  const { resolveLoad } = record.handlers;
  if (resolveLoad === undefined) {
    return latchFault(record, `LOAD effect ${seq} was issued, but no resolveLoad handler was given`);
  }
  if (record.stop.fired) return abandon(record, seq);
  const request = loadRequest(effect);
  const { iri } = request;
  const expired = expiry(effect);
  const signal = AbortSignal.any([record.controller.signal, expired.signal]);
  const pending = invokeHost(
    "resolveLoad",
    () => resolveLoad(request, { signal }),
    (value) => normalizeLoad(value, iri, signal),
  );
  const settled = await Promise.race([pending, record.stop.promise, expired.promise]);
  const { job } = record;
  if (settled === STOPPED) return abandon(record, seq);
  if (settled === EXPIRED) return job.expireEffect(seq);
  switch (settled.type) {
    case "document":
      return delivered(record, seq, job.deliverGraph(seq, settled.bytes, settled.mediaType, settled.base));
    case "dataset": {
      let status;
      try {
        status = job.deliverGraphDataset(seq, settled.dataset);
      } catch {
        // Not a `Dataset` of this package: the handler's answer is not one the protocol
        // defines, which is the invocation's fault.
        status = job.deliverFailure(
          seq,
          "fault",
          `resolveLoad returned an unrecognized value (${kindOf(settled.dataset)}) for ${iri}`,
        );
      }
      return delivered(record, seq, status);
    }
    case "redirect":
      return delivered(record, seq, job.deliverRedirect(seq, settled.location));
    case "failure":
      return delivered(record, seq, job.deliverFailure(seq, settled.kind, settled.message));
    default:
      return delivered(record, seq, job.deliverFailure(seq, "fault", settled.message));
  }
}

function loadRequest(effect) {
  const flat = effect.headers();
  const headers = [];
  for (let index = 0; index + 1 < flat.length; index += 2) {
    headers.push([flat[index], flat[index + 1]]);
  }
  return {
    kind: "load",
    iri: effect.iri,
    accept: effect.accept,
    userAgent: effect.userAgent,
    headers,
    timeoutMs: effect.timeoutMs,
  };
}

async function normalizeLoad(value, iri, signal) {
  if (isResponseLike(value)) {
    if (!value.ok) {
      await cancelBody(value);
      return { type: "failure", kind: "transport", message: `HTTP ${value.status} from ${iri}` };
    }
    const contentType = value.headers?.get?.("content-type");
    if (typeof contentType !== "string" || contentType.trim() === "") {
      await cancelBody(value);
      return { type: "failure", kind: "transport", message: `the LOAD response from ${iri} carries no Content-Type` };
    }
    const mediaType = contentType.split(";")[0].trim().toLowerCase();
    const base = typeof value.url === "string" && value.url !== "" ? value.url : iri;
    return readBody(value, iri, signal, (bytes) => ({ type: "document", bytes, mediaType, base }));
  }
  if (value instanceof Uint8Array || value instanceof ArrayBuffer || typeof value === "string") {
    return fault(
      `resolveLoad answered ${iri} without a media type; return { bytes | text, mediaType } ` +
        "or a Response carrying a Content-Type",
    );
  }
  if (value != null && typeof value === "object" && value.kind === "redirect") {
    if (typeof value.location !== "string") {
      return fault(`resolveLoad answered ${iri} with a redirect whose location is not a string`);
    }
    return { type: "redirect", location: value.location };
  }
  const failure = typedFailure(value, "resolveLoad");
  if (failure !== undefined) return failure;
  if (value != null && typeof value === "object" && "mediaType" in value) {
    return normalizeDocument(value, iri);
  }
  if (value != null && typeof value === "object" && !Array.isArray(value)) {
    // A `Dataset` from this package; `deliverGraphDataset` checks the class itself.
    return { type: "dataset", dataset: value };
  }
  return fault(`resolveLoad returned an unrecognized value (${kindOf(value)}) for ${iri}`);
}

function normalizeDocument(value, iri) {
  const { mediaType, base } = value;
  if (typeof mediaType !== "string" || mediaType === "") {
    return fault(`resolveLoad answered ${iri} with a mediaType that is not a non-empty string`);
  }
  if (base !== undefined && typeof base !== "string") {
    return fault(`resolveLoad answered ${iri} with a base that is not a string`);
  }
  const hasBytes = value.bytes !== undefined;
  const hasText = value.text !== undefined;
  if (hasBytes === hasText) {
    return fault(`resolveLoad answered ${iri} with ${hasBytes ? "both bytes and text" : "neither bytes nor text"}`);
  }
  let bytes;
  if (hasText) {
    if (typeof value.text !== "string") {
      return fault(`resolveLoad answered ${iri} with text that is not a string`);
    }
    bytes = encoder.encode(value.text);
  } else if (value.bytes instanceof Uint8Array) {
    bytes = value.bytes;
  } else if (value.bytes instanceof ArrayBuffer) {
    bytes = new Uint8Array(value.bytes);
  } else {
    return fault(`resolveLoad answered ${iri} with bytes that are not a Uint8Array or ArrayBuffer`);
  }
  return { type: "document", bytes, mediaType, base: base ?? iri };
}

// ---------------------------------------------------------------------------
// Host answers
// ---------------------------------------------------------------------------

const STOPPED = Symbol("stopped");

/**
 * Call a host handler and normalize its answer. The returned promise never rejects: a
 * throw, a rejection, or a failure while normalizing is the handler's fault, which the
 * caller delivers to the effect as a `"fault"` failure. The host's own promise is always
 * given a rejection handler, so abandoning it never surfaces as unhandled.
 */
function invokeHost(name, call, normalize) {
  let raw;
  try {
    raw = call();
  } catch (error) {
    return Promise.resolve(fault(`${name} threw: ${describe(error)}`));
  }
  const pending = Promise.resolve(raw);
  pending.catch(() => {});
  return pending.then(
    (value) =>
      Promise.resolve()
        .then(() => normalize(value))
        .catch((error) => fault(`${name}'s answer could not be read: ${describe(error)}`)),
    (error) => fault(`${name} rejected: ${describe(error)}`),
  );
}

function typedFailure(value, name) {
  if (value == null || typeof value !== "object" || !("kind" in value)) return undefined;
  if (typeof value.kind !== "string") {
    return fault(`${name} returned a failure whose kind is not a string`);
  }
  if (typeof value.message !== "string") {
    return fault(`${name} returned a ${JSON.stringify(value.kind)} failure without a string message`);
  }
  // An unknown kind is delivered as is: Rust fails the effect with a fault naming it.
  return { type: "failure", kind: value.kind, message: value.message };
}

function isResponseLike(value) {
  return (
    value != null &&
    typeof value === "object" &&
    typeof value.ok === "boolean" &&
    typeof value.status === "number" &&
    typeof value.arrayBuffer === "function"
  );
}

async function readBody(response, source, signal, wrap) {
  let buffer;
  try {
    buffer = await response.arrayBuffer();
  } catch (error) {
    if (signal.aborted) {
      return { type: "failure", kind: "transport", message: `reading the response from ${source} was aborted` };
    }
    return { type: "failure", kind: "transport", message: `reading the response from ${source} failed: ${describe(error)}` };
  }
  return wrap(new Uint8Array(buffer));
}

async function cancelBody(response) {
  try {
    await response.body?.cancel?.();
  } catch {
    // The body is being discarded; a stream that cannot be cancelled has nothing left to
    // release.
  }
}

/**
 * The handler's own fault — it threw, rejected or answered with something the protocol
 * does not define — as a normalized answer. Delivered to the effect it was answering as
 * a `"fault"` failure (`deliverFailure`, or `faultExchange` for a shared call).
 */
function fault(message) {
  return { type: "fault", message };
}

// ---------------------------------------------------------------------------
// Delivery helpers (all return a status for wasm)
// ---------------------------------------------------------------------------

/**
 * Latch a protocol violation as the job's fault: a delivery the job cannot make sense of,
 * never a handler's answer. Nothing a clause writes absorbs it.
 */
function latchFault(record, message) {
  record.job.fault(message);
  return installed.SuspendStatus.Fault;
}

function abandon(record, seq) {
  const status = record.job.deliverGoverned(seq);
  if (status !== installed.DeliveryStatus.Accepted) {
    return latchFault(record, `abandoning effect ${seq} was refused with status ${status}`);
  }
  return installed.SuspendStatus.Abandoned;
}

function delivered(record, seq, status) {
  const { DeliveryStatus, SuspendStatus } = installed;
  if (status === DeliveryStatus.Accepted) return SuspendStatus.Answered;
  // The job latched its own fault naming the cause. A stale or finished status cannot
  // happen for the outstanding effect and is latched here.
  if (status === DeliveryStatus.Fault) return SuspendStatus.Fault;
  return latchFault(record, `the delivery for effect ${seq} was refused with status ${status}`);
}

// ---------------------------------------------------------------------------
// Runs, records and stop sources
// ---------------------------------------------------------------------------

function validateHost(host) {
  if (host == null || typeof host !== "object") {
    throw new TypeError("runJob expects a host object");
  }
  const { resolveService, resolveLoad, signal } = host;
  for (const [name, value] of [["resolveService", resolveService], ["resolveLoad", resolveLoad]]) {
    if (value !== undefined && typeof value !== "function") {
      throw new TypeError(`${name} must be a function`);
    }
  }
  if (
    signal !== undefined &&
    (signal == null || typeof signal.aborted !== "boolean" || typeof signal.addEventListener !== "function")
  ) {
    throw new TypeError("signal must be an AbortSignal");
  }
  return { resolveService, resolveLoad, signal };
}

function newRecord(job, handlers) {
  const record = {
    id: job.id,
    job,
    handlers,
    top: job.stackTop,
    base: job.stackBase,
    canary: job.stackCanary,
    controller: new AbortController(),
    stop: { fired: false, promise: undefined, resolve: undefined },
    reject: undefined,
    dispose: undefined,
  };
  record.stop.promise = new Promise((resolve) => {
    record.stop.resolve = resolve;
  });
  const fire = (reason) => {
    if (record.stop.fired) return;
    record.stop.fired = true;
    record.stop.resolve(STOPPED);
    record.controller.abort(reason);
  };
  const { signal } = handlers;
  let onAbort;
  if (signal !== undefined) {
    onAbort = () => {
      try {
        record.job.cancel();
      } catch (error) {
        // The gate refusing a poisoned instance: the poisoning already rejected the job.
        if (!poisonedBy(error) && poisonReason === null) throw error;
      }
      fire(signal.reason);
    };
    if (signal.aborted) onAbort();
    else signal.addEventListener("abort", onAbort, { once: true });
  }
  record.dispose = () => {
    if (onAbort !== undefined) signal.removeEventListener("abort", onAbort);
    if (!record.stop.fired) {
      record.stop.fired = true;
      record.stop.resolve(STOPPED);
    }
  };
  return record;
}

function startRun(record) {
  return new Promise((resolve, reject) => {
    record.reject = reject;
    let pending;
    try {
      // The linked run wrapper takes the region top as its last argument: it sets the
      // stack pointer to it for the run and puts the idle pointer back when the run
      // returns, suspended, finished or unwound.
      pending = installed.promisingRun(record.id, record.top);
    } catch (error) {
      pending = Promise.reject(error);
    }
    pending.then(
      (status) => {
        if (status >>> 0 === installed.RunStatus.Overran) {
          poison(record.job.errorMessage ?? `asynchronous job ${record.id} overran its stack region`);
          reject(poisonError());
          return;
        }
        resolve(status >>> 0);
      },
      (error) => {
        // A run that rejected was unwound — a trap, or a JavaScript exception thrown
        // through its frames — and left its gate entry without an exit, which the gate
        // records. A rejection that left the gate balanced never entered the instance
        // (the promising call itself refused) and is the caller's error as it stands.
        if (poisonReason === null && !gateClosed()) {
          reject(error);
          return;
        }
        poison(describe(error));
        reject(poisonError());
      },
    );
  });
}

// ---------------------------------------------------------------------------
// Poisoning
// ---------------------------------------------------------------------------

/**
 * A trap out of a run leaves the instance in an unknown state: every `RefCell` its
 * frames borrowed stays borrowed, and linear memory may hold a half-applied mutation. A
 * trap out of a synchronous call, or a JavaScript exception thrown through wasm frames,
 * leaves the same state behind it; a Rust panic in any call — synchronous or
 * asynchronous — poisons from its hook, before its trap unwinds (`purrdf_jspi_panicked`).
 * Nothing repairs that, so the instance is dead: `purrdf_poisoned` is set, so the gate
 * linked into the module traps every later entry — synchronous calls, constructors, and
 * objects created before the trap included — every in-flight job is rejected with the
 * poison error, every later asynchronous call refuses with it, and suspended runs are
 * never resumed. The first reason is kept: a panic's trap, arriving after its hook, stays
 * named as the panic.
 */
function poison(reason) {
  if (poisonReason !== null) return;
  poisonReason = reason;
  if (installed !== null) installed.exports.purrdf_poisoned.value = 1;
  const error = poisonError();
  for (const record of records.values()) {
    record.reject?.(error);
  }
}

function poisonError() {
  return new Error(
    `the wasm instance trapped (${poisonReason}) and cannot be used again; ` +
      "load the package in a fresh JavaScript realm (a new page, Worker isolate or process)",
  );
}

/**
 * Whether the gate's globals say the instance may not be entered again: it is marked
 * poisoned, or an entry was unwound without its exit (`active − parked ≠ outbound`).
 * Read only while JavaScript runs with no wasm frame unwound by this module's own doing,
 * which is every point this module reads it at.
 */
function gateClosed() {
  if (installed === null) return false;
  const { purrdf_poisoned, purrdf_active, purrdf_parked, purrdf_outbound } = installed.exports;
  return purrdf_poisoned.value !== 0 || purrdf_active.value - purrdf_parked.value !== purrdf_outbound.value;
}

/**
 * Whether `error`, thrown out of an export this module called, is the gate refusing a
 * poisoned instance — a `WebAssembly.RuntimeError` while the gate is closed — or the very
 * trap that closed it. Either poisons the instance (naming the trap, unless a reason is
 * already kept) and returns `true`; any other error returns `false` untouched.
 */
function poisonedBy(error) {
  if (!(error instanceof WebAssembly.RuntimeError) || !gateClosed()) return false;
  poison(describe(error));
  return true;
}

function never() {
  return new Promise(() => {});
}

// ---------------------------------------------------------------------------
// Small utilities
// ---------------------------------------------------------------------------

function describe(error) {
  if (error instanceof Error) return `${error.name}: ${error.message}`;
  try {
    return String(error);
  } catch {
    return kindOf(error);
  }
}

function kindOf(value) {
  if (value === null) return "null";
  if (Array.isArray(value)) return "an array";
  if (typeof value === "object") return `an object${value.constructor?.name ? ` (${value.constructor.name})` : ""}`;
  return `a ${typeof value}`;
}

/** An abort reason with a DOMException's name where the engine has one. */
function abortReason(message, name) {
  if (typeof globalThis.DOMException === "function") {
    return new globalThis.DOMException(message, name);
  }
  const error = new Error(message);
  error.name = name;
  return error;
}
