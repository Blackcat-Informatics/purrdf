// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Child process for the synchronous-panic poisoning test.
//
// A Rust panic aborts on wasm32: the panic hook runs, then the module traps. The hook
// PurRDF installs at start-up poisons the instance, naming the panic, before that trap
// unwinds — whether the panicking call was an asynchronous job or an ordinary
// synchronous export. No PurRDF entry point panics on any input, so the panic comes from
// `__purrdf_test_panic`, a raw export of the instance (never exported by the package
// root) that panics only while `globalThis.__purrdfArmTestPanic` is `true` — the flag
// this fixture, and only this fixture, sets.
//
// Before the panic, the valid neighbours on the very objects the panic later poisons:
// the unarmed test export does nothing, and synchronous calls that throw typed errors —
// a parse error, a query's syntax error — leave the instance serving every lane exactly.
// After it, every entry point must refuse with the poison naming the panic, a job that
// was in flight when the panic happened included.
//
// Prints one JSON line; the parent test asserts on it.

import * as purrdf from "../../index.mjs";
import init from "../../pkg/purrdf_wasm.js";
import { enumerateSurface, settle, settleSync } from "./poisoned-surface.mjs";

const { DataFactory, Dataset, QueryEngine, ready, version } = purrdf;

await ready();
// Already instantiated: `init` hands back the one instance's raw exports.
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
report.unarmed = settleSync(() => raw.__purrdf_test_panic());
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
report.sizeBefore = settleSync(() => data.size);
report.versionBefore = settleSync(() => version());

// A job in flight — suspended on a SERVICE the host never answers — when the panic
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

// The panic, out of a synchronous call.
globalThis.__purrdfArmTestPanic = true;
report.panicked = settleSync(() => raw.__purrdf_test_panic());
report.inFlight = await inFlight;

// Every later call refuses with the poison naming the panic.
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
