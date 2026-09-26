// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Child process for the trap-poisoning test. Run with
// `--wasm-stack-switching-stack-size=32`: V8 runs every JSPI-promised call on a
// secondary native stack of that many KiB (984 by default), while synchronous calls keep
// the full main stack. The parser recurses once per nested `{ … }` and, on wasm32,
// refuses a group past its host-stack budget (283 groups inside the WHERE group) with its
// own typed error; 300 nested groups recurse through far more native stack than 32 KiB
// before reaching that refusal, so the asynchronous run overflows V8's stack — a genuine
// `RangeError` trap out of the promising call — while the synchronous run of the very
// same query, on the full main stack, reports the parser's own refusal.
//
// Before the trap, the same objects serve every lane through jobs that finish, jobs that
// fault and jobs whose host reports a typed failure — none of which may poison anything.
// After it, every entry point of the package must refuse: the objects created before
// the trap, new ones, statics and free functions, enumerated from the package root's
// own exports rather than listed by hand.
//
// Prints one JSON line; the parent test asserts on it.

import * as purrdf from "../../index.mjs";
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

function nested(depth) {
  let pattern = `?s <${EX}p> ?o`;
  for (let level = 0; level < depth; level += 1) pattern = `{ ${pattern} }`;
  return `SELECT ?s WHERE ${pattern}`;
}
const TOO_DEEP = nested(300);

const report = {};

// Before the trap: an ordinary asynchronous query answers on the small secondary stack.
report.asyncBefore = await settle(() => engine.queryAsync(data, SHALLOW));
// A job that faults (its host rejects) and one whose host reports a typed transport
// failure: both are errors of their own job, never a poisoning.
report.faulted = await settle(() =>
  engine.queryAsync(data, SERVICE_QUERY, {
    resolveService: () => Promise.reject(new Error("the example.org endpoint refused")),
  }),
);
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
// The synchronous lane's stack context is its own after those jobs: the deep query is
// the parser's own typed refusal (300 nested groups, past the host-stack budget) — twice,
// so it is an error and not a trap that poisoned anything — never a stack exhaustion read
// off a job's freed region.
report.syncDeep = await settle(() => engine.query(data, TOO_DEEP));
report.syncDeepAgain = await settle(() => engine.query(data, TOO_DEEP));

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

// Every later call refuses with the same poison: asynchronous ones, synchronous calls and
// getters on objects created before the trap, new objects, statics, free functions, and
// `ready()` itself.
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
// Releasing an object is the one call that does not throw: the instance's memory is
// abandoned whole, and a finalizer has no caller to report an error to.
report.freeAfter = settleSync(() => data.free());

// The whole package root, enumerated, and every method and getter of the objects created
// before the trap.
report.surface = await enumerateSurface(purrdf, data, engine);

process.stdout.write(`${JSON.stringify(report)}\n`);
// The in-flight job's host promise never settles; nothing else is pending.
process.exit(0);
