// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Child process for the synchronous-trap poisoning test.
//
// A trap that is not a panic — allocation failure, a stray `unreachable` — runs no panic
// hook: the engine raises a `WebAssembly.RuntimeError` straight out of the export, after
// unwinding every wasm frame of the call without restoring anything. The trapped entry
// never returns through the poison gate linked into the module, so the gate's counters
// stop balancing, and the next entry into the instance marks it poisoned and traps; every
// entry after that traps too. No PurRDF entry point traps on any input, so the trap comes
// from `__purrdf_test_trap`, a raw export of the instance (never exported by the package
// root) that executes `unreachable` only while `globalThis.__purrdfArmTestTrap` is `true`
// — the flag this fixture, and only this fixture, sets.
//
// Before the trap, the valid neighbours on the very objects the trap later poisons: the
// unarmed test export does nothing, synchronous calls that throw typed errors leave the
// instance serving every lane, a job started from inside a synchronous sink callback
// answers, and an asynchronous job answers. After it, every synchronous entry traps at
// the gate, and every asynchronous entry — a job that was in flight when the trap
// happened included — rejects with the poison error the runtime builds once it reads the
// gate. The runtime takes no part in a synchronous call, so it learns of the trap at the
// next call it does take part in; the in-flight job is awaited after that call.
//
// Prints one JSON line; the parent test asserts on it.

import * as purrdf from "../../index.mjs";
import init from "../../pkg/purrdf_wasm.js";
import { enumerateSurface, settle, settleSync } from "./poisoned-surface.mjs";

const { DataFactory, Dataset, QueryEngine, ready, version } = purrdf;

await ready();
// Already instantiated: `init` hands back the one instance's exports.
const raw = await init();

const EX = "http://example.org/";
const data = Dataset.parse(`<${EX}a> <${EX}p> <${EX}o> .\n`, "nquads");
const engine = new QueryEngine();
const factory = new DataFactory();
const extra = factory.quad(factory.namedNode(`${EX}e`), factory.namedNode(`${EX}p`), factory.namedNode(`${EX}o`));
const SHALLOW = `SELECT ?s WHERE { ?s <${EX}p> ?o }`;
const SERVICE_QUERY = `SELECT ?x WHERE { SERVICE <${EX}sparql> { ?s ?p ?x } }`;

/** The gate's globals: its flag, and whether its counters balance. */
const gate = () => ({
  poisoned: raw.purrdf_poisoned.value,
  balanced: raw.purrdf_active.value - raw.purrdf_parked.value === raw.purrdf_outbound.value,
});

const report = {};

// The valid neighbours. Unarmed, the test export returns 0 and does nothing.
report.unarmed = settleSync(() => raw.__purrdf_test_trap());
// Synchronous calls that throw typed errors — twice each, so a poisoning after the first
// would show as the poison on the second.
report.parseError = settleSync(() => Dataset.parse(`<${EX}a> <${EX}p> .\n`, "nquads"));
report.parseErrorAgain = settleSync(() => Dataset.parse(`<${EX}a> <${EX}p> .\n`, "nquads"));
report.queryError = await settle(() => engine.query(data, "SELECT ?s WHERE {"));
report.queryErrorAgain = await settle(() => engine.query(data, "SELECT ?s WHERE {"));
// After them, every lane answers exactly on the same objects: a synchronous update
// commits, and both lanes see it.
report.updateBefore = await settle(() => engine.update(data, `INSERT DATA { <${EX}b> <${EX}p> <${EX}o> }`));
report.syncBefore = await settle(() => engine.query(data, SHALLOW));
report.asyncBefore = await settle(() => engine.queryAsync(data, SHALLOW));
report.asyncUpdateBefore = await settle(() => engine.updateAsync(data, `INSERT DATA { <${EX}c> <${EX}p> <${EX}o> }`));
report.asyncAfterUpdate = await settle(() => engine.queryAsync(data, SHALLOW));
// A job started from inside a synchronous sink callback — JavaScript running inside a
// wasm call, which the gate counts as an outbound import call — begins and answers.
let fromCallback;
const chunks = [];
data.serializeToSink("nquads", undefined, {
  write(chunk) {
    if (fromCallback === undefined) fromCallback = engine.queryAsync(data, SHALLOW);
    chunks.push(chunk.slice());
  },
});
report.serializedInCallback = new TextDecoder().decode(Buffer.concat(chunks)) === data.serialize("nquads");
report.asyncFromCallback = await settle(() => fromCallback);
report.gateBefore = gate();
report.sizeBefore = settleSync(() => data.size);
report.versionBefore = settleSync(() => version());

// A job in flight — suspended on a SERVICE the host never answers — when the trap
// happens.
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

// The trap, out of a synchronous call: the caller gets the trap itself.
globalThis.__purrdfArmTestTrap = true;
report.trapped = settleSync(() => raw.__purrdf_test_trap());

// The first asynchronous call after it reads the gate, poisons the instance and rejects
// — and so does the job that was in flight, whose host never answers.
report.asyncAfter = await settle(() => engine.queryAsync(data, SHALLOW));
report.inFlight = await inFlight;
report.gateAfter = gate();

// Every synchronous entry traps at the gate; `ready()` rejects with the poison.
report.syncAfter = settleSync(() => engine.query(data, SHALLOW));
report.parseErrorAfter = settleSync(() => Dataset.parse(`<${EX}a> <${EX}p> .\n`, "nquads"));
report.sizeAfter = settleSync(() => data.size);
report.addAfter = settleSync(() => data.add(extra));
report.iterateAfter = settleSync(() => [...data]);
report.quadAfter = settleSync(() => extra.subject);
report.newEngineAfter = settleSync(() => new QueryEngine());
report.newDatasetAfter = settleSync(() => new Dataset());
report.versionAfter = settleSync(() => version());
report.readyAfter = await settle(() => ready());
report.freeAfter = settleSync(() => data.free());
report.surface = await enumerateSurface(purrdf, data, engine);

process.stdout.write(`${JSON.stringify(report)}\n`);
// The in-flight job's host promise never settles; nothing else is pending.
process.exit(0);
