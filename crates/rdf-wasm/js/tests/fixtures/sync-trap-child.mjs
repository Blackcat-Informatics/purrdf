// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Child process for the synchronous-trap poisoning test.
//
// A trap that is not a panic — allocation failure, a stray `unreachable` — runs no panic
// hook: the engine raises a `WebAssembly.RuntimeError` straight out of the export, after
// unwinding every wasm frame of the call without restoring anything. The glue's guarded
// exports see it and poison the instance, naming the trap. No PurRDF entry point traps
// on any input, so the trap comes from `__purrdf_test_trap`, a raw export of the instance
// (never exported by the package root) that executes `unreachable` only while
// `globalThis.__purrdfArmTestTrap` is `true` — the flag this fixture, and only this
// fixture, sets.
//
// Before the trap, the valid neighbours on the very objects the trap later poisons: the
// unarmed test export does nothing, synchronous calls that throw typed errors leave the
// instance serving every lane, and an asynchronous job answers on the guarded instance.
// After it, every entry point must refuse with the poison naming the trap, a job that
// was in flight when the trap happened included.
//
// Prints one JSON line; the parent test asserts on it.

import * as purrdf from "../../index.mjs";
import init from "../../pkg/purrdf_wasm.js";
import { enumerateSurface, settle, settleSync } from "./poisoned-surface.mjs";

const { DataFactory, Dataset, QueryEngine, ready, version } = purrdf;

await ready();
// Already instantiated: `init` hands back the one instance's exports, as the glue holds
// them.
const raw = await init();

const EX = "http://example.org/";
const data = Dataset.parse(`<${EX}a> <${EX}p> <${EX}o> .\n`, "nquads");
const engine = new QueryEngine();
const factory = new DataFactory();
const extra = factory.quad(factory.namedNode(`${EX}e`), factory.namedNode(`${EX}p`), factory.namedNode(`${EX}o`));
const SHALLOW = `SELECT ?s WHERE { ?s <${EX}p> ?o }`;
const SERVICE_QUERY = `SELECT ?x WHERE { SERVICE <${EX}sparql> { ?s ?p ?x } }`;

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
// commits, and both lanes see it — the asynchronous one through the unguarded runner and
// the guarded exports its host calls use.
report.updateBefore = await settle(() => engine.update(data, `INSERT DATA { <${EX}b> <${EX}p> <${EX}o> }`));
report.syncBefore = await settle(() => engine.query(data, SHALLOW));
report.asyncBefore = await settle(() => engine.queryAsync(data, SHALLOW));
report.asyncUpdateBefore = await settle(() => engine.updateAsync(data, `INSERT DATA { <${EX}c> <${EX}p> <${EX}o> }`));
report.asyncAfterUpdate = await settle(() => engine.queryAsync(data, SHALLOW));
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

// The trap, out of a synchronous call.
globalThis.__purrdfArmTestTrap = true;
report.trapped = settleSync(() => raw.__purrdf_test_trap());
report.inFlight = await inFlight;

// Every later call refuses with the poison naming the trap.
report.asyncAfter = await settle(() => engine.queryAsync(data, SHALLOW));
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
