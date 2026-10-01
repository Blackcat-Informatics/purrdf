// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import { test } from "node:test";
import assert from "node:assert/strict";
import { ready, Dataset, QueryEngine, CancellationToken, ServiceCatalog } from "../index.mjs";
import { AsyncJobOptions, AsyncOperationKind } from "../pkg/purrdf_wasm.js";
await ready();
const query = "SELECT (1 AS ?n) WHERE {}";
const update = "INSERT DATA { <https://example.org/s> <https://example.org/p> 1 }";
const aliases = {
  json: ["json", "srj", "sparql-json", "application/sparql-results+json"],
  xml: ["xml", "sparql-xml", "application/sparql-results+xml"],
  csv: ["csv", "text/csv"],
  tsv: ["tsv", "text/tab-separated-values"],
};

test("every results alias has the same bytes through all raw format APIs", async () => {
  const engine = new QueryEngine();
  const ds = new Dataset();
  for (const [canonical, names] of Object.entries(aliases)) {
    const expected = engine.queryRaw(ds, query, { format: canonical });
    for (const name of names) for (const format of [name, ` \t${name.toUpperCase()}\n`]) {
      assert.equal(engine.queryRaw(ds, query, { format }), expected, format);
      assert.equal(await engine.queryRawAsync(ds, query, { format }), expected, format);
      assert.equal(new TextDecoder().decode(await engine.queryRawBytesAsync(ds, query, { format })), expected, format);
    }
  }
});

function governedCalls(engine) {
  return [
    ["queryGoverned", (ds, opts) => engine.queryGoverned(ds, query, opts)],
    ["queryEntailmentGoverned", (ds, opts) => engine.queryEntailmentGoverned(ds, query, "rdfs", opts)],
    ["updateGoverned", (ds, opts) => engine.updateGoverned(ds, update, opts)],
    ["queryGovernedAsync", (ds, opts) => engine.queryGovernedAsync(ds, query, opts)],
    ["queryEntailmentGovernedAsync", (ds, opts) => engine.queryEntailmentGovernedAsync(ds, query, "rdfs", opts)],
    ["updateGovernedAsync", (ds, opts) => engine.updateGovernedAsync(ds, update, opts)],
    ["queryGovernedNegotiatedAsync", (ds, opts) => engine.queryGovernedNegotiatedAsync(ds, query, opts)],
  ];
}

test("every governed twin exposes metered and explicit no-ceiling execution", async () => {
  const engine = new QueryEngine();
  for (const [name, call] of governedCalls(engine)) {
    const initial = await call(new Dataset(), { noCeiling: false });
    const baseline = initial.outcome ?? initial;
    assert.ok(baseline.evidence.consumed.fuel > 0n, name);
    const unlimited = await call(new Dataset(), { noCeiling: true });
    const unbounded = unlimited.outcome ?? unlimited;
    assert.equal(unbounded.evidence.consumed.fuel, 0n, name);
    for (const key of ["fuel", "maxAnswers", "maxIntermediateCells", "maxScratchBytes", "maxRemoteRequests"]) {
      await assert.rejects(async () => call(new Dataset(), { noCeiling: true, [key]: 0 }), /no.ceiling|maxAnswers/i, name);
    }
    for (const noCeiling of [null, 0, 1, "true", {}, []]) {
      await assert.rejects(async () => call(new Dataset(), { noCeiling }), /boolean/, name);
    }
    const expired = await call(new Dataset(), { noCeiling: true, deadlineMs: 0 });
    assert.equal(expired.tripped.cause, "deadline-exceeded", name);
  }
});

test("no-ceiling execution preserves cancellation and refuses ignored options", async () => {
  const engine = new QueryEngine();
  const cancel = new CancellationToken(); cancel.cancel();
  for (const call of [
    opts => engine.queryGoverned(new Dataset(), query, opts),
    opts => engine.queryEntailmentGoverned(new Dataset(), query, "rdfs", opts),
    opts => engine.updateGoverned(new Dataset(), update, opts),
  ]) assert.equal(call({ noCeiling: true, cancel }).tripped.cause, "cancelled");
  const service = "SELECT * WHERE { SERVICE <https://example.org/remote> { ?s ?p ?o } }";
  for (const call of [
    opts => engine.queryGovernedAsync(new Dataset(), service, opts),
    opts => engine.queryEntailmentGovernedAsync(new Dataset(), service, "rdfs", opts),
    opts => engine.queryGovernedNegotiatedAsync(new Dataset(), service, opts),
    opts => engine.updateGovernedAsync(new Dataset(), "LOAD <https://example.org/doc>", opts),
  ]) {
    const controller = new AbortController();
    const stop = () => { controller.abort(); return new Promise(() => {}); };
    const stopped = await call({ noCeiling: true, signal: controller.signal, resolveService: stop, resolveLoad: stop });
    assert.equal(stopped.tripped.cause, "cancelled");
  }
  for (const noCeiling of [true, false]) {
    assert.throws(() => engine.query(new Dataset(), query, { noCeiling }), /execution governor/);
    await assert.rejects(engine.queryAsync(new Dataset(), query, { noCeiling }), /execution governor/);
  }
});

test("strict JSON options refuse repeated members and arrays with diagnostic pointers", () => {
  const ds = new Dataset();
  for (const [source, pointer] of [
    ['{"mode":"compact","mode":"table"}', '/mode'],
    ['{"layout":{"crossingSweeps":1,"crossingSweeps":2}}', '/layout/crossingSweeps'],
    ['{"layout":[]}', '/layout'],
    ['[]', ''],
  ]) {
    for (const method of ["visualModelJson", "visualExportJson", "visualSvgJson"]) {
      assert.throws(() => ds[method](source), error => {
        assert.match(error.message, /^visualization options: /);
        if (pointer) assert.ok(error.message.includes(`at ${pointer}`), error.message);
        else assert.doesNotMatch(error.message, / at \/\S+/);
        return true;
      });
    }
  }
  const catalog = new ServiceCatalog();
  for (const [source, pointer] of [
    ['{"capabilities":["query"],"capabilities":[]}', '/capabilities'],
    ['{"capabilities":[],"credential":{"header":"X-Key","header":"X-Other","value":"v"}}', '/credential/header'],
    ['{"capabilities":[],"credential":[]}', '/credential'],
    ['[]', ''],
  ]) for (const call of [() => catalog.addService("https://example.org/s", source), () => catalog.setFallback(source)]) {
    assert.throws(call, error => {
      assert.equal(error.code, "purrdf-wasm-options");
      assert.match(error.message, /^service profile: /);
      if (pointer) assert.ok(error.message.includes(`at ${pointer}`), error.message);
        else assert.doesNotMatch(error.message, / at \/\S+/);
      return true;
    });
  }
});


test("generated host Reflect.get preserves throwing getter refusal", async () => {
  const engine = new QueryEngine();
  const options = { get noCeiling() { throw new Error("getter failed"); } };
  assert.throws(() => AsyncJobOptions.fromJs(AsyncOperationKind.Governed, options, undefined, false, undefined), error => {
    assert.equal(error.code, "purrdf-wasm-options");
    assert.match(error.message, /noCeiling could not be read/);
    return true;
  });
  await assert.rejects(engine.queryGovernedAsync(new Dataset(), query, options), error => {
    assert.equal(error.code, "purrdf-wasm-options");
    assert.match(error.message, /noCeiling could not be read/);
    return true;
  });
});
