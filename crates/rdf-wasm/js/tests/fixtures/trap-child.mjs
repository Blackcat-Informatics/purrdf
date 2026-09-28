// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Child process for the trap-poisoning test. Run with
// `--wasm-stack-switching-stack-size=32`: V8 runs every JSPI-promised call on a
// secondary native stack of that many KiB (984 by default), while synchronous calls keep
// the full main stack. The evaluator recurses once per nested `OPTIONAL`, and 50 of them
// are far inside the host-stack budget it admits (284 nested graph patterns, budgeted
// against V8's default stack), so both lanes admit the query; evaluating it needs more
// than 32 KiB of native stack, so the asynchronous run overflows V8's stack — a genuine
// `RangeError` trap out of the promising call — while the synchronous run of the very
// same query, on the full main stack, answers it. (Measured on the shipped artifact: 20
// nested `OPTIONAL`s still answer on the 32 KiB stack, and 50 overflow it.)
//
// Before the trap, the same objects serve every lane through jobs that finish, jobs
// whose host rejects and jobs whose host reports a typed failure — none of which may
// poison anything. After it, every entry point of the package must refuse: the
// asynchronous ones with the poison error naming the trap, the synchronous ones at the
// poison gate linked into the module — on the objects created before the trap, new
// ones, statics and free functions, enumerated from the package root's own exports
// rather than listed by hand.
//
// Prints one JSON line; the parent test asserts on it.

import * as purrdf from "../../index.mjs";
import init from "../../pkg/purrdf_wasm.js";
import { enumerateSurface, settle, settleSync } from "./poisoned-surface.mjs";

const { DataFactory, Dataset, QueryEngine, ready, version } = purrdf;

await ready();

const EX = "http://example.org/";
const data = Dataset.parse(`<${EX}a> <${EX}p> <${EX}o> .\n`, "nquads");
const engine = new QueryEngine();
const factory = new DataFactory();
const extra = factory.quad(factory.namedNode(`${EX}e`), factory.namedNode(`${EX}p`), factory.namedNode(`${EX}o`));
const SHALLOW = `SELECT ?s WHERE { ?s <${EX}p> ?o }`;
const SERVICE_QUERY = `SELECT ?x WHERE { SERVICE <${EX}sparql> { ?s ?p ?x } }`;

/** `depth` nested `OPTIONAL`s, each matching the triple the level above matched. */
function nestedOptional(depth) {
  return `SELECT ?s WHERE { ?s <${EX}p> ?o ${`OPTIONAL { ?s <${EX}p> ?o `.repeat(depth)}${"}".repeat(depth)} }`;
}
const TOO_DEEP = nestedOptional(50);

const report = {};

// Before the trap: an ordinary asynchronous query answers on the small secondary stack.
report.asyncBefore = await settle(() => engine.queryAsync(data, SHALLOW));
// A job whose host rejects (its SERVICE invocation fails as the host's fault, reported
// under the host-fault code without the handler's words) and one whose host reports a
// typed transport failure: both are errors of their own job, never a poisoning.
const faultedRun = engine.queryAsync(data, SERVICE_QUERY, {
  resolveService: () => Promise.reject(new Error("the example.org endpoint refused")),
});
faultedRun.catch((error) => {
  report.faultedCode = error.code;
});
report.faulted = await settle(() => faultedRun);
report.typedFailure = await settle(() =>
  engine.queryAsync(data, SERVICE_QUERY, {
    resolveService: () => ({ kind: "transport", message: "the example.org endpoint is unreachable" }),
  }),
);
// After them, every lane still answers exactly, on the objects that ran the jobs and on
// new ones: an asynchronous update commits, and the synchronous and asynchronous lanes
// both see it.
report.updateBefore = await settle(() => engine.updateAsync(data, `INSERT DATA { <${EX}b> <${EX}p> <${EX}o> }`));
report.syncBefore = await settle(() => engine.query(data, SHALLOW));
report.asyncAfterFaults = await settle(() => engine.queryAsync(data, SHALLOW));
report.newEngineBefore = await settle(() => new QueryEngine().query(Dataset.parse(`<${EX}c> <${EX}p> <${EX}o> .\n`, "nquads"), SHALLOW));
report.sizeBefore = settleSync(() => data.size);
// The synchronous lane's stack context is its own after those jobs: the query that traps
// on a job's 32 KiB native stack answers on the main stack — twice, so the first run
// poisoned nothing — never a stack exhaustion read off a job's freed region.
report.syncDeep = await settle(() => engine.select(data, TOO_DEEP));
report.syncDeepAgain = await settle(() => engine.select(data, TOO_DEEP));

// A job in flight — suspended on a SERVICE the host never answers — when another traps.
let serviceAsked;
const asked = new Promise((resolve) => {
  serviceAsked = resolve;
});
const inFlight = settle(() =>
  engine.queryAsync(data, SERVICE_QUERY, {
    resolveService: () => {
      serviceAsked();
      return new Promise(() => {});
    },
  }),
);
await asked;

// The trap.
report.trapped = await settle(() => engine.queryAsync(data, TOO_DEEP));
report.inFlight = await inFlight;

// Every later call refuses: asynchronous ones and `ready()` with the poison; synchronous
// calls and getters on objects created before the trap, new objects, statics and free
// functions at the gate.
report.asyncAfter = await settle(() => engine.queryAsync(data, SHALLOW));
report.updateAfter = await settle(() => engine.updateAsync(data, `INSERT DATA { <${EX}d> <${EX}p> <${EX}o> }`));
report.syncAfter = settleSync(() => engine.query(data, SHALLOW));
report.syncDeepAfter = settleSync(() => engine.query(data, TOO_DEEP));
report.sizeAfter = settleSync(() => data.size);
report.addAfter = settleSync(() => data.add(extra));
report.iterateAfter = settleSync(() => [...data]);
report.quadAfter = settleSync(() => extra.subject);
report.newEngineAfter = settleSync(() => new QueryEngine());
report.newDatasetAfter = settleSync(() => new Dataset());
report.parseAfter = settleSync(() => Dataset.parse(`<${EX}a> <${EX}p> <${EX}o> .\n`, "nquads"));
report.versionAfter = settleSync(() => version());
report.readyAfter = await settle(() => ready());
// Releasing an object is the one entry that returns: the release exports' gate is inert
// on a poisoned instance, so `free()` and `[Symbol.dispose]()` release nothing and throw
// nothing (the memory is abandoned whole, and a finalizer has no caller to report to).
report.freeAfter = settleSync(() => data.free());
report.disposeAfter =
  typeof engine[Symbol.dispose] === "function" ? settleSync(() => engine[Symbol.dispose]()) : { settled: "absent" };
// The gate's globals: the trapped run's entry never returned, and the flag is set.
const raw = await init();
report.gateAfter = {
  poisoned: raw.purrdf_poisoned.value,
  balanced: raw.purrdf_active.value - raw.purrdf_parked.value === raw.purrdf_outbound.value,
};

// The whole package root, enumerated, and every method and getter of the objects created
// before the trap.
report.surface = await enumerateSurface(purrdf, data, engine);

process.stdout.write(`${JSON.stringify(report)}\n`);
// The in-flight job's host promise never settles; nothing else is pending.
process.exit(0);
