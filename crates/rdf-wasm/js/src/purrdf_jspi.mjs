// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// The JavaScript half of the asynchronous operation runtime: the suspending import the
// wasm module calls, and the per-instance scheduler that drives asynchronous jobs.
//
// The wasm-bindgen glue imports this module by relative path (`./purrdf_jspi.mjs`) and
// hands `purrdf_jspi_suspend` (and `purrdf_jspi_panicked`, which the panic hook calls)
// to the instance as raw imports, so this file ships next to the glue in `pkg/`. It imports nothing: the package root passes the instance's
// exports in through `installAsync` once `ready()` has instantiated it. The glue itself
// hands this module its handle on those exports (`purrdf_jspi_bind_glue`, which
// `make wasm-pkg` wires into the glue), so a trap or a panic can bar every entry point
// at once.
//
// # The protocol (the Rust side is `crates/rdf-wasm/src/async_query.rs`)
//
// A job is begun in Rust (`QueryEngine.beginAsync`) and run here (`runJob`) through the
// raw export `purrdf_jspi_run`, wrapped by `WebAssembly.promising`. Every effect the job
// performs — a `SERVICE` request, a `LOAD` document, a turn of the event loop — posts a
// ticket in Rust and calls `purrdf_jspi_suspend(job, seq, out)`, which is
// `suspendImpl` below wrapped in `WebAssembly.Suspending`. `suspendImpl` takes the
// ticket, awaits the host's answer, delivers it, writes `0` to the `u32` at `out` and
// returns a status: 0 answered, 1 abandoned on the job's stop signal, 2 a fault is
// latched on the job.
//
// # Nothing crosses a suspended frame
//
// A JavaScript exception thrown into a suspended wasm frame bricks the instance, so
// `suspendImpl` never throws and never rejects. Every condition becomes a status: a host
// that throws, rejects or returns something unrecognizable is a *fault*, latched on the
// job with `job.fault(...)` — never a transport failure that `SERVICE SILENT` could
// swallow. A resolver catches its own network errors and reports them as
// `{ kind: "transport", message }`.
//
// # Stack regions
//
// A suspended job's frames stay live in linear memory, so every job runs on its own
// region and the shadow-stack pointer (`__stack_pointer`, reached through the exported
// `__wbindgen_add_to_stack_pointer`) is switched by these rules:
//
// 1. At the start of a run, the context's pointer is captured as the job's `outer`, the
//    pointer is set to the job's region top, and the promising export is called.
// 2. At every suspension, before anything else, the pointer is switched back to `outer`.
// 3. Just before returning into wasm after every resumption, `outer` is captured again:
//    a resumption runs in the resumer's context, not the starter's.
// 4. When a run settles, a pointer the run left inside its region is put back to
//    `outer`.
//
// The resumed job restores its *own* pointer from its `suspend` frame's epilogue — the
// build gate `scripts/check-wasm-jspi-frame.py` proves that restore is the first
// stack-relevant instruction after the import returns — so the host's restores only have
// to keep the pointer out of regions while JavaScript runs.
//
// A run that finishes after a resumption returns from wasm with the pointer at its
// region's top, and the reaction that applies rule 4 is a microtask later; another job
// resumed in between would otherwise capture that stale pointer as its own `outer` and,
// after the first region is freed, run synchronous calls on freed memory. So every
// capture first checks whether the pointer lies inside a live job's region and, if it
// does, puts it back to that job's `outer` — inside a region is never a legitimate place
// for the pointer while JavaScript runs.

/** Thrown by every asynchronous method on an engine without JSPI. */
export const NO_JSPI_MESSAGE =
  "asynchronous queries need WebAssembly JavaScript Promise Integration " +
  "(WebAssembly.Suspending and WebAssembly.promising), which this JavaScript engine " +
  "does not provide; the synchronous API is unaffected";

const NO_YIELD_MESSAGE =
  "asynchronous queries need a macrotask primitive to yield to the event loop " +
  "(scheduler.yield, setImmediate or MessageChannel), and this JavaScript environment " +
  "provides none; the synchronous API is unaffected";

const NOT_INSTALLED_MESSAGE =
  "the asynchronous runtime is not installed; await ready() before any asynchronous call";

// `purrdf_jspi_run`: the job's frames ran past its region's overrun zone; memory
// outside the job's allocation may be overwritten, so the instance is poisoned.
const RUN_OVERRAN = 4;

// Statuses of `purrdf_jspi_suspend` (returned to wasm).
const SUSPEND_ANSWERED = 0;
const SUSPEND_ABANDONED = 1;
const SUSPEND_FAULT = 2;

// Statuses of the `AsyncJob` delivery methods (1 stale and 3 finished never answer the
// outstanding effect, so they are only ever reported, never branched on).
const DELIVERY_ACCEPTED = 0;
const DELIVERY_FAULT = 2;

// `AsyncEffectKind`.
const EFFECT_SERVICE = 1;
const EFFECT_LOAD = 2;
const EFFECT_YIELD = 3;

const DEFAULT_MAX_CONCURRENT_JOBS = 16;

/** The longest delay `setTimeout` honours (a signed 32-bit millisecond count). */
const MAX_TIMER_MS = 2 ** 31 - 1;

const HAS_JSPI =
  typeof WebAssembly === "object" &&
  typeof WebAssembly.Suspending === "function" &&
  typeof WebAssembly.promising === "function";

// A poisoned instance's release functions (`__wbg_<class>_free`) do nothing: the glue calls
// them from `free()`, `[Symbol.dispose]()` and its finalization registries, and a
// finalization callback has no caller to report an error to. The instance's memory is
// abandoned whole, so there is nothing left for them to release.
const RELEASE_EXPORT = /^__wbg_[a-z0-9_]+_free$/;

// ---------------------------------------------------------------------------
// Instance state (one module instance per wasm instance: the glue imports this module
// once, and the package root instantiates the wasm once)
// ---------------------------------------------------------------------------

let glue = null; // { exports, retarget }: the glue's handle on the instance's exports
let installed = null; // { exports, promisingRun, addToStackPointer }
let idleTop = 0;
let poisonReason = null;
let maxConcurrentJobs = DEFAULT_MAX_CONCURRENT_JOBS;
const records = new Map(); // job id -> record
const singleFlight = new WeakMap(); // resolveService -> Map<key, shared exchange[]>
const datasetQueues = new Map(); // dataset id -> tail promise

const encoder = new TextEncoder();

// ---------------------------------------------------------------------------
// The yield primitive, chosen once
// ---------------------------------------------------------------------------

const yielder = chooseYield();

function chooseYield() {
  const scheduler = globalThis.scheduler;
  if (scheduler != null && typeof scheduler.yield === "function") {
    return { name: "scheduler.yield", once: () => scheduler.yield() };
  }
  if (typeof globalThis.setImmediate === "function") {
    const setImmediate = globalThis.setImmediate;
    return {
      name: "setImmediate",
      once: () => new Promise((resolve) => setImmediate(resolve)),
    };
  }
  if (typeof globalThis.MessageChannel === "function") {
    const channel = new globalThis.MessageChannel();
    const waiting = [];
    // Node keeps the process alive while a port with a listener is referenced; hold the
    // reference only while a turn is pending.
    const ref = () => channel.port1.ref?.();
    const unref = () => channel.port1.unref?.();
    channel.port1.onmessage = () => {
      const resolve = waiting.shift();
      if (waiting.length === 0) unref();
      resolve?.();
    };
    unref();
    return {
      name: "MessageChannel",
      once: () =>
        new Promise((resolve) => {
          waiting.push(resolve);
          if (waiting.length === 1) ref();
          channel.port2.postMessage(0);
        }),
    };
  }
  return null;
}

// ---------------------------------------------------------------------------
// Public API (consumed by the package root)
// ---------------------------------------------------------------------------

/**
 * Bind the wasm-bindgen glue's handle on the instance's exports to the poison gate.
 *
 * `make wasm-pkg` rewrites the glue's `wasm = instance.exports;` into a call of this
 * function, so every call the glue makes into the instance — a constructor, a method, a
 * getter, a static, a free function, a finalizer — reads the exports through the one
 * variable `retarget` reassigns. Poisoning retargets it at an object that refuses every
 * call, which bars the whole package surface at once, synchronous calls and objects
 * created before the trap included, while a live instance pays nothing per call.
 * Returns `exports`, which the glue keeps as its handle.
 */
export function purrdf_jspi_bind_glue(exports, retarget) {
  if (glue !== null) {
    throw new Error("the wasm-bindgen glue bound a second instance; one module instance drives one wasm instance");
  }
  if (exports == null || typeof exports !== "object") {
    throw new TypeError("purrdf_jspi_bind_glue expects the wasm instance's exports");
  }
  if (typeof retarget !== "function") {
    throw new TypeError("purrdf_jspi_bind_glue expects a function that reassigns the glue's exports");
  }
  glue = { exports, retarget };
  return exports;
}

/**
 * The raw import the instance's panic hook calls (`crates/rdf-wasm/src/panic_poison.rs`)
 * before a Rust panic aborts: `reason` points at `len` bytes of UTF-8 describing the
 * panic. The panic's trap is about to unwind the call — a synchronous call as much as an
 * asynchronous job — and leave the instance's state half-changed, so the instance is
 * poisoned here, naming the panic, before the trap reaches any JavaScript that could call
 * in again.
 *
 * This runs inside the panicking wasm frame, so it never calls into the instance and
 * never throws: an exception thrown into that frame would unwind it instead of the trap,
 * and a failure to read the reason still poisons.
 */
export function purrdf_jspi_panicked(reason, len) {
  let text = "a Rust panic";
  try {
    const memory = glue?.exports.memory;
    if (memory instanceof WebAssembly.Memory) {
      text = new TextDecoder().decode(new Uint8Array(memory.buffer, reason >>> 0, len >>> 0).slice());
    }
  } catch {
    // Keep the generic reason: the poisoning is what matters.
  }
  try {
    if (glue !== null) poison(text);
  } catch {
    // Nothing may cross the panicking frame; the trap that follows still reaches the
    // caller.
  }
}

/**
 * Install the scheduler over the instance's raw exports (the object the glue's `init`
 * returns). Called once by `ready()`. Throws when the exports lack the runtime's entry
 * points — an artifact built without the asynchronous lane is a build defect, not a
 * mode — or when the glue did not bind them to the poison gate.
 */
export function installAsync(exports) {
  if (installed !== null) {
    if (installed.exports === exports) return;
    throw new Error("the asynchronous runtime is already installed over another instance");
  }
  if (exports == null || typeof exports !== "object") {
    throw new TypeError("installAsync expects the wasm instance's exports");
  }
  if (glue === null || glue.exports !== exports) {
    throw new Error(
      "the wasm-bindgen glue did not bind these exports to the poison gate " +
        "(purrdf_jspi_bind_glue); the package artifact was not built by make wasm-pkg",
    );
  }
  for (const name of ["memory", "__wbindgen_add_to_stack_pointer", "purrdf_jspi_run"]) {
    if (!(name in exports)) {
      throw new Error(`the wasm instance does not export ${name}; the package artifact is incomplete`);
    }
  }
  installed = {
    exports,
    addToStackPointer: exports.__wbindgen_add_to_stack_pointer,
    promisingRun: HAS_JSPI ? WebAssembly.promising(exports.purrdf_jspi_run) : null,
  };
  idleTop = sp();
}

/** Whether this engine can run asynchronous jobs: JSPI and a yield primitive exist. */
export function hasAsyncQueries() {
  return HAS_JSPI && yielder !== null;
}

/**
 * Throw the one clear error that explains why an asynchronous call cannot run here, or
 * return when it can: no JSPI, no yield primitive, not installed, or a poisoned
 * instance.
 */
export function assertAsyncQueries() {
  if (!HAS_JSPI) throw new Error(NO_JSPI_MESSAGE);
  if (yielder === null) throw new Error(NO_YIELD_MESSAGE);
  if (installed === null) throw new Error(NOT_INSTALLED_MESSAGE);
  if (poisonReason !== null) throw poisonError();
}

/**
 * Throw the poison error when a trap has poisoned the instance, or return. The package
 * root's own entry points that reach no wasm export call this first; every other entry
 * point is barred by the glue's retargeted exports.
 */
export function assertNotPoisoned() {
  if (poisonReason !== null) throw poisonError();
}

/**
 * The macrotask primitive jobs yield through — `"scheduler.yield"`, `"setImmediate"` or
 * `"MessageChannel"`, chosen once when this module loads — or `undefined` when none
 * exists.
 */
export function asyncYieldPrimitive() {
  return yielder?.name;
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
 * `host` carries the job's effect handlers and stop sources:
 * - `resolveService(request, ctx)` — answers a `SERVICE` effect. `request` is
 *   `{ kind: "service", endpoint, queryText, accept, contentType, userAgent, timeoutMs,
 *   headers }` (`headers`: `[name, value]` pairs, in sending order); `ctx` is
 *   `{ signal, remainingDeadlineMs, silent, maxIntermediateCells }`. It returns (or
 *   resolves to) SPARQL Results JSON as a `Uint8Array`, `ArrayBuffer` or `string`; a
 *   `Response` (a non-ok status is a transport failure and its body is cancelled); or
 *   `{ kind: "transport" | "denied", message }`. A throw, a rejection or any other value
 *   is a fault that fails the job. A job asks at most once for a request it repeats
 *   (the same request, `silent` and `maxIntermediateCells`), and concurrent jobs share
 *   one call when the host would see an equivalent context for each (see
 *   `joinSingleFlight`); `ctx.signal` is then the shared call's.
 * - `resolveLoad(request, ctx)` — answers a `LOAD` effect. `request` is
 *   `{ kind: "load", iri }`, `ctx` is `{ signal }`. It returns `{ bytes | text,
 *   mediaType, base? }` (`base` defaults to the IRI), a `Response` (its `Content-Type`
 *   names the media type; its URL, or the IRI, is the base), a `Dataset`, or a typed
 *   failure as above. Every `LOAD` is its own call.
 * - `signal` — an `AbortSignal` that cancels the job.
 * - `deadlineMs` — the job's deadline, so an awaited effect is abandoned when it passes
 *   (the job itself reports the trip as a deadline).
 *
 * Resolves to the run's status: 0 an outcome is stored, 1 an error is stored (read
 * `job.errorKind` / `job.takeError()`), 2 the job is unknown, 3 it was already started.
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

/**
 * Serialize asynchronous updates on one dataset: `fn` runs once every earlier update
 * queued on `datasetId` has settled, and its result (or rejection) is returned. Queries
 * never take this lock.
 */
export function withDatasetUpdateLock(datasetId, fn) {
  const previous = datasetQueues.get(datasetId) ?? Promise.resolve();
  const current = previous.then(() => fn());
  const tail = current.then(
    () => undefined,
    () => undefined,
  );
  datasetQueues.set(datasetId, tail);
  tail.then(() => {
    if (datasetQueues.get(datasetId) === tail) datasetQueues.delete(datasetId);
  });
  return current;
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
    // be trusted about its stack, so it is never resumed.
    if (poisonReason === null) {
      poison(`purrdf_jspi_suspend named job ${jobId}, which the scheduler is not running`);
    }
    setSp(idleTop);
    return never();
  }
  let status;
  try {
    status = await answer(record, seq);
  } catch (error) {
    // A poisoned instance refuses every call, `job.fault` included, and its suspended
    // runs are never resumed.
    if (poisonReason !== null) return never();
    // `answer` catches everything itself; this is the last line that keeps a rejection
    // out of the suspended frame.
    status = latchFault(record, `the asynchronous bridge failed: ${describe(error)}`);
  }
  if (poisonReason !== null) return never();
  // Rule 3: the resumer's context is this job's outer context from now on.
  record.outer = captureOuter();
  new DataView(installed.exports.memory.buffer).setUint32(outPtr, 0, true);
  return status;
}

/** Answer effect `seq` of `record`'s job; resolves to the status for wasm. */
async function answer(record, seq) {
  const { job } = record;
  // Rule 2, preceded by the region check, both before any await or wasm call.
  const here = sp();
  const memory = new DataView(installed.exports.memory.buffer);
  const inRegion = here > record.base && here <= record.top;
  const canaryIntact = memory.getUint32(record.base, true) === record.canary;
  setSp(record.outer);
  if (!inRegion) {
    return latchFault(
      record,
      `asynchronous job ${record.id} suspended outside its stack region ` +
        `(stack pointer ${here}, region (${record.base}, ${record.top}])`,
    );
  }
  if (!canaryIntact) {
    const reason = `asynchronous job ${record.id} overwrote the canary at the base of its stack region`;
    latchFault(record, reason);
    poison(reason);
    return SUSPEND_FAULT;
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
      case EFFECT_YIELD:
        await yielder.once();
        return SUSPEND_ANSWERED;
      case EFFECT_SERVICE:
        return await answerService(record, effect);
      case EFFECT_LOAD:
        return await answerLoad(record, effect);
      default:
        return latchFault(record, `effect ${seq} has unknown kind ${String(effect.kind)}`);
    }
  } finally {
    effect.free();
  }
}

// ---------------------------------------------------------------------------
// SERVICE
// ---------------------------------------------------------------------------

async function answerService(record, effect) {
  const seq = effect.seq;
  const { resolveService } = record.handlers;
  if (resolveService === undefined) {
    return latchFault(record, `SERVICE effect ${seq} was issued, but no resolveService handler was given`);
  }
  if (record.stop.fired) return abandon(record, seq);
  const request = serviceRequest(effect);
  const ctx = {
    remainingDeadlineMs: effect.remainingDeadlineMs,
    silent: effect.silent,
    maxIntermediateCells: effect.maxIntermediateCells,
  };
  // Every field the host can observe except the deadline and the signal. The deadline is
  // matched by `joinSingleFlight`; the per-job memo needs no deadline component, because
  // one job's deadline is one instant for every request it issues.
  const key = JSON.stringify([
    request.endpoint,
    request.queryText,
    request.accept,
    request.contentType,
    request.userAgent,
    request.timeoutMs,
    request.headers,
    ctx.silent,
    ctx.maxIntermediateCells === undefined ? null : String(ctx.maxIntermediateCells),
  ]);
  let settled = record.memo.get(key);
  if (settled === undefined) {
    const shared = joinSingleFlight(resolveService, key, request, ctx);
    const winner = await Promise.race([shared.promise, record.stop.promise]);
    leaveSingleFlight(shared);
    if (winner === STOPPED) return abandon(record, seq);
    settled = winner;
    if (settled.type !== "fault") record.memo.set(key, settled);
  }
  switch (settled.type) {
    case "bytes":
      return delivered(record, seq, record.job.deliverBindings(seq, settled.bytes));
    case "failure":
      return delivered(record, seq, record.job.deliverFailure(seq, settled.kind, settled.message));
    default:
      return latchFault(record, settled.message);
  }
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
  };
}

/**
 * One host call shared by every job that issues the same request through the same
 * `resolveService` while it is in flight, when the host would see the same context for
 * each of them.
 *
 * A job joins an open exchange only when its key matches — the request, `silent` and
 * `maxIntermediateCells` — and the exchange's deadline is no earlier than the job's own:
 * the host was told `remainingDeadlineMs` when the exchange started, so the instant it
 * bounds the call by is `start + remainingDeadlineMs` (no bound when the job that started
 * it had no deadline), and the joining job's is `now + its remainingDeadlineMs`. A host
 * that bounds its work by the deadline it was told therefore gives up no earlier than it
 * would on the joining job's own call, so a failure the shared call reports by running
 * out of time is one the job's own call would have reported too. Any other job starts an
 * exchange of its own, called with its own context. A job without a deadline joins only
 * an exchange without one.
 *
 * Because the contexts are equivalent, the answer — rows, a transport or denied failure,
 * or a fault — is delivered to every waiting job as it stands: it is what the same remote
 * answered the same request. The exchange's promise never rejects: the host's answer is
 * normalized, and a host bug becomes a fault every waiting job latches.
 *
 * The host's `signal` belongs to the exchange, not to any one job. A job that is stopped
 * while it waits abandons only its own effect; the signal aborts once every waiting job
 * has abandoned the exchange, and never after it has settled.
 */
function joinSingleFlight(resolveService, key, request, ctx) {
  let exchanges = singleFlight.get(resolveService);
  if (exchanges === undefined) {
    exchanges = new Map();
    singleFlight.set(resolveService, exchanges);
  }
  const deadline = absoluteDeadline(ctx.remainingDeadlineMs);
  let open = exchanges.get(key);
  let shared = open?.find((candidate) => candidate.deadline >= deadline);
  if (shared === undefined) {
    const controller = new AbortController();
    shared = { controller, deadline, waiters: 0, open: true, exchanges, key, promise: undefined };
    const current = shared;
    if (open === undefined) {
      open = [];
      exchanges.set(key, open);
    }
    open.push(shared);
    shared.promise = invokeHost(
      "resolveService",
      () => resolveService({ ...request, headers: request.headers.map((pair) => [...pair]) }, { ...ctx, signal: controller.signal }),
      (value) => normalizeService(value, request.endpoint, controller.signal),
    ).finally(() => closeSingleFlight(current));
  }
  shared.waiters += 1;
  return shared;
}

/** The instant a job's remaining deadline ends at, or `Infinity` when it has none. */
function absoluteDeadline(remainingDeadlineMs) {
  if (remainingDeadlineMs === undefined || remainingDeadlineMs === null) return Infinity;
  return now() + Number(remainingDeadlineMs);
}

function now() {
  return typeof globalThis.performance?.now === "function" ? globalThis.performance.now() : Date.now();
}

function leaveSingleFlight(shared) {
  shared.waiters -= 1;
  if (shared.waiters === 0 && shared.open) {
    closeSingleFlight(shared);
    shared.controller.abort(abortReason("every job waiting on this request was stopped", "AbortError"));
  }
}

function closeSingleFlight(shared) {
  if (!shared.open) return;
  shared.open = false;
  const open = shared.exchanges.get(shared.key);
  if (open === undefined) return;
  const index = open.indexOf(shared);
  if (index !== -1) open.splice(index, 1);
  if (open.length === 0) shared.exchanges.delete(shared.key);
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
  const iri = effect.iri;
  const signal = record.controller.signal;
  const pending = invokeHost(
    "resolveLoad",
    () => resolveLoad({ kind: "load", iri }, { signal }),
    (value) => normalizeLoad(value, iri, signal),
  );
  const settled = await Promise.race([pending, record.stop.promise]);
  if (settled === STOPPED) return abandon(record, seq);
  const { job } = record;
  switch (settled.type) {
    case "document":
      return delivered(record, seq, job.deliverGraph(seq, settled.bytes, settled.mediaType, settled.base));
    case "dataset": {
      let status;
      try {
        status = job.deliverGraphDataset(seq, settled.dataset);
      } catch {
        return latchFault(record, `resolveLoad returned an unrecognized value (${kindOf(settled.dataset)}) for ${iri}`);
      }
      return delivered(record, seq, status);
    }
    case "failure":
      return delivered(record, seq, job.deliverFailure(seq, settled.kind, settled.message));
    default:
      return latchFault(record, settled.message);
  }
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
 * throw, a rejection, or a failure while normalizing is a fault. The host's own promise
 * is always given a rejection handler, so abandoning it never surfaces as unhandled.
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
  // An unknown kind is delivered as is: the job latches the fault naming it.
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

function fault(message) {
  return { type: "fault", message };
}

// ---------------------------------------------------------------------------
// Delivery helpers (all return a status for wasm)
// ---------------------------------------------------------------------------

function latchFault(record, message) {
  record.job.fault(message);
  return SUSPEND_FAULT;
}

function abandon(record, seq) {
  const status = record.job.deliverGoverned(seq);
  if (status !== DELIVERY_ACCEPTED) {
    return latchFault(record, `abandoning effect ${seq} was refused with status ${status}`);
  }
  return SUSPEND_ABANDONED;
}

function delivered(record, seq, status) {
  if (status === DELIVERY_ACCEPTED) return SUSPEND_ANSWERED;
  // 2: the job latched its own fault naming the cause. 1 or 3 cannot happen for the
  // outstanding effect and are latched here.
  if (status === DELIVERY_FAULT) return SUSPEND_FAULT;
  return latchFault(record, `the delivery for effect ${seq} was refused with status ${status}`);
}

// ---------------------------------------------------------------------------
// Runs, records and stop sources
// ---------------------------------------------------------------------------

function validateHost(host) {
  if (host == null || typeof host !== "object") {
    throw new TypeError("runJob expects a host object");
  }
  const { resolveService, resolveLoad, signal, deadlineMs } = host;
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
  let deadline;
  if (deadlineMs !== undefined) {
    deadline = typeof deadlineMs === "bigint" ? Number(deadlineMs) : deadlineMs;
    if (typeof deadline !== "number" || !Number.isFinite(deadline) || deadline < 0) {
      throw new RangeError(`deadlineMs must be a non-negative number, got ${String(deadlineMs)}`);
    }
  }
  return { resolveService, resolveLoad, signal, deadlineMs: deadline };
}

function newRecord(job, handlers) {
  const record = {
    id: job.id,
    job,
    handlers,
    top: job.stackTop,
    base: job.stackBase,
    canary: job.stackCanary,
    outer: 0,
    memo: new Map(),
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
  const { signal, deadlineMs } = handlers;
  let onAbort;
  if (signal !== undefined) {
    onAbort = () => {
      record.job.cancel();
      fire(signal.reason);
    };
    if (signal.aborted) onAbort();
    else signal.addEventListener("abort", onAbort, { once: true });
  }
  let timer;
  // A delay beyond the timer range fires at once in some engines; a deadline that far
  // off is left to the job's own clock.
  if (deadlineMs !== undefined && deadlineMs <= MAX_TIMER_MS) {
    timer = setTimeout(() => {
      // The deadline is latched before the signal aborts, so the trip reads as a
      // deadline rather than a cancellation.
      record.job.tripDeadline();
      fire(abortReason(`the job's deadline of ${deadlineMs} ms passed`, "TimeoutError"));
    }, deadlineMs);
  }
  record.dispose = () => {
    if (timer !== undefined) clearTimeout(timer);
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
    // Rule 1.
    record.outer = captureOuter();
    setSp(record.top);
    let pending;
    try {
      pending = installed.promisingRun(record.id);
    } catch (error) {
      pending = Promise.reject(error);
    }
    // The promising call returns once the job has suspended (the pointer is already
    // back at `outer`), finished (it is at the region's top) or trapped (it is anywhere).
    if (sp() !== record.outer) setSp(record.outer);
    pending.then(
      (status) => {
        leaveRegion(record);
        if (status >>> 0 === RUN_OVERRAN) {
          poison(record.job.takeError() ?? `asynchronous job ${record.id} overran its stack region`);
          reject(poisonError());
          return;
        }
        resolve(status >>> 0);
      },
      (error) => {
        leaveRegion(record);
        const reason = describe(error);
        poison(reason);
        reject(poisonError());
      },
    );
  });
}

/** Rule 4: put back a pointer the settled run left inside its region. */
function leaveRegion(record) {
  const here = sp();
  if (here >= record.base && here <= record.top) setSp(record.outer);
}

/**
 * The stack pointer of the running JavaScript context, with any pointer a settled run
 * left inside its region put back first.
 */
function captureOuter() {
  const here = sp();
  for (const record of records.values()) {
    if (here >= record.base && here <= record.top) {
      setSp(record.outer);
      return record.outer;
    }
  }
  return here;
}

function sp() {
  return installed.addToStackPointer(0) >>> 0;
}

function setSp(value) {
  installed.addToStackPointer((value - sp()) | 0);
}

// ---------------------------------------------------------------------------
// Poisoning
// ---------------------------------------------------------------------------

/**
 * A trap out of a run leaves the instance in an unknown state: the job's stack context is
 * still in place of the caller's, every `RefCell` its frames borrowed stays borrowed, and
 * linear memory may hold a half-applied mutation. A Rust panic in any call — synchronous
 * or asynchronous — leaves the same state behind it, and its hook poisons before its trap
 * unwinds (`purrdf_jspi_panicked`). Nothing repairs that, so the instance is dead: the glue's exports are retargeted at an object that refuses every call —
 * synchronous calls, constructors, and objects created before the trap included — every
 * in-flight job is rejected, and every later call refuses with the same error. Suspended
 * runs are never resumed.
 */
function poison(reason) {
  if (poisonReason !== null) return;
  poisonReason = reason;
  glue.retarget(poisonedExports());
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
 * What the glue reads the instance's exports through once it is poisoned: reading any
 * export throws the poison error, so no call reaches the instance, except the release
 * functions (see `RELEASE_EXPORT`), which do nothing.
 */
function poisonedExports() {
  const releaseNothing = () => undefined;
  return new Proxy(Object.freeze(Object.create(null)), {
    get(_target, name) {
      if (typeof name === "string" && RELEASE_EXPORT.test(name)) return releaseNothing;
      throw poisonError();
    },
  });
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
