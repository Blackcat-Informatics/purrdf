// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: these cases exercise JavaScript integer conversion, live Promise suspension,
// wasm-bindgen ownership and the shipped Cloudflare host adapter against the optimized module.

import { test } from "node:test";
import assert from "node:assert/strict";
import { CompiledJsonLdContext, Dataset, QueryEngine, ready } from "../index.mjs";
import { AsyncJobOptions, AsyncOperationKind, RunStatus } from "../pkg/purrdf_wasm.js";
import { runJob } from "../pkg/purrdf_jspi.mjs";
import { handleSparqlRequest } from "../cloudflare.mjs";

await ready();

const NOW = `-12345-06-07T08:09:10.${"123456789".repeat(128)}7+05:30`;
const INPUTS = { now: NOW, entropySeed: 18446744073709551615n };
const QUERY = "SELECT (NOW() AS ?t) (RAND() AS ?r) (UUID() AS ?u) (STRUUID() AS ?s) WHERE {}";
const GRAPH = "CONSTRUCT { <https://example.org/s> <https://example.org/time> ?t } WHERE { BIND(NOW() AS ?t) }";
const decoder = new TextDecoder();
const cells = (result) => Object.fromEntries(
  Object.entries(result.rows.take(0)).map(([name, term]) => [name, term.value]),
);
const rawCells = (text) => Object.fromEntries(
  Object.entries(JSON.parse(text).results.bindings[0]).map(([name, term]) => [name, term.value]),
);

test("async coordinates cross every query kind and output alias", async () => {
  const engine = new QueryEngine();
  const data = new Dataset();
  const optionsJson = JSON.stringify({ version: 1, mode: "context", prefixes: { ex: "https://example.org/" } });
  const context = new CompiledJsonLdContext(optionsJson);
  const random = Math.random;
  let hostDraws = 0;
  Math.random = () => { hostDraws += 1; return 0.25; };
  try {
    const result = await engine.queryAsync(data, QUERY, INPUTS); const expected = cells(result); assert.equal(result.executionReceipt.now, NOW); assert.equal(result.executionReceipt.entropySeed, String(INPUTS.entropySeed));
    assert.equal(expected.t, NOW);
    assert.deepEqual(cells(await engine.selectAsync(data, QUERY, INPUTS)), expected);
    assert.equal(await engine.askAsync(data, `ASK { FILTER(NOW() = "${NOW}"^^<http://www.w3.org/2001/XMLSchema#dateTime>) }`, INPUTS).then(result => result.boolean), true);
    for (const method of ["constructAsync", "describeAsync"]) {
      const query = method === "constructAsync" ? GRAPH : `DESCRIBE <https://example.org/s> WHERE { FILTER(NOW() = "${NOW}"^^<http://www.w3.org/2001/XMLSchema#dateTime>) }`;
      const source = method === "constructAsync" ? data : Dataset.parse('<https://example.org/s> <https://example.org/time> "present" .', "ntriples");
      const graph = (await engine[method](source, query, INPUTS)).dataset;
      assert.equal(graph.size, 1);
      if (method === "constructAsync") assert.ok(graph.serialize("ntriples").includes(NOW));
      graph.free();
      if (source !== data) source.free();
    }
    assert.deepEqual(rawCells(await engine.queryRawAsync(data, QUERY, INPUTS).then(result => result.body)), expected);
    assert.deepEqual(rawCells(decoder.decode(await engine.queryRawBytesAsync(data, QUERY, INPUTS).then(result => result.body))), expected);
    assert.deepEqual(rawCells(await data.queryAsync(QUERY, INPUTS).then(result => result.body)), expected);
    const provenanceNamespace = { prefix: "prov", iri: "https://example.org/provenance#" };
    const raw = await engine.queryRawAsync(data, QUERY, { ...INPUTS, provenanceNamespace }).then(result => result.body);
    assert.deepEqual(rawCells(raw), expected);
    assert.ok(raw.includes("https://example.org/provenance#"));
    const configured = await engine.queryRawAsync(data, GRAPH, { ...INPUTS, format: "jsonld", optionsJson }).then(result => result.body);
    const contextual = await engine.queryRawWithContextAsync(data, GRAPH, "jsonld", context, INPUTS).then(result => result.body);
    assert.equal(configured, contextual);
    assert.ok(configured.includes(NOW));
    const governed = await engine.queryGovernedAsync(data, QUERY, { ...INPUTS, fuel: 100000 });
    assert.equal(governed.isComplete, true);
    assert.deepEqual(cells(governed.result), expected);
    assert.ok(governed.evidence.async.polls > 0);
    const entailed = await engine.queryEntailmentGovernedAsync(data, QUERY, "rdfs", INPUTS);
    assert.equal(entailed.phase, "answered");
    assert.deepEqual(cells(entailed.outcome.result), expected);
    const negotiated = await engine.queryGovernedNegotiatedAsync(data, QUERY, INPUTS);
    assert.equal(negotiated.isComplete, true);
    assert.deepEqual(rawCells(decoder.decode(negotiated.body.bytes)), expected);
    const explanation = await engine.explainQueryAsync(data, `SELECT ?t WHERE { BIND(NOW() AS ?t) FILTER(?t = "${NOW}"^^<http://www.w3.org/2001/XMLSchema#dateTime>) }`, INPUTS).then(result => result.body);
    assert.match(explanation, /Filter fuel=\d+ rows=1 /);
    assert.equal(hostDraws, 0, "supplied seeds never call the host entropy import");
  } finally {
    Math.random = random;
    context.free(); data.free(); engine.free();
  }
});

test("async coordinates reject lossy host inputs and unsupported routes", async () => {
  const engine = new QueryEngine();
  const data = new Dataset();
  try {
    for (const entropySeed of [-1n, 18446744073709551616n, -1, 1.5, Number.MAX_SAFE_INTEGER + 1, NaN, Infinity, "7", true]) {
      await assert.rejects(engine.queryAsync(data, QUERY, { entropySeed }), /entropySeed/);
    }
    for (const now of ["not a date", 7, new Date()]) {
      await assert.rejects(engine.queryAsync(data, QUERY, { now }), /now/);
    }
    const zero = cells(await engine.queryAsync(data, QUERY, { now: NOW, entropySeed: 0n }));
    assert.deepEqual(cells(await engine.queryAsync(data, QUERY, { now: NOW, entropySeed: 0 })), zero);
    const maximum = cells(await engine.queryAsync(data, QUERY, INPUTS));
    assert.notEqual(maximum.r, zero.r);
    assert.equal(data.size, 0);
    const stopped = await engine.queryGovernedAsync(data, QUERY, { ...INPUTS, fuel: 0 });
    assert.equal(stopped.isComplete, false);
    assert.equal(stopped.tripped.dimension, "fuel");
  } finally { data.free(); engine.free(); }
});

test("async coordinate owners survive real suspension and interleaved jobs", async () => {
  const engine = new QueryEngine();
  const data = new Dataset();
  const query = "SELECT ?before ?after ?t ?u WHERE { BIND(RAND() AS ?before) SERVICE <https://example.org/service> { ?x ?p ?o } BIND(RAND() AS ?after) BIND(NOW() AS ?t) BIND(UUID() AS ?u) }";
  const answer = JSON.stringify({ head: { vars: [] }, results: { bindings: [{}] } });
  const controls = { ...INPUTS };
  // Low-level callers own resolver identities; do not alias the package's first handler.
  const options = AsyncJobOptions.fromJs(AsyncOperationKind.Raw, controls, 1_000_000, false, undefined);
  const job = engine.beginAsync(data, AsyncOperationKind.Raw, query, options);
  options.free();
  controls.now = "2000-01-01T00:00:00Z";
  controls.entropySeed = 0n;
  let entered;
  const suspended = new Promise((resolve) => { entered = resolve; });
  let release;
  const pending = runJob(job, { resolveService: () => new Promise((resolve) => { release = resolve; entered(); }) });
  try {
    await suspended;
    const other = rawCells(await engine.queryRawAsync(data, query, { ...controls, resolveService: async () => answer }).then(result => result.body));
    assert.equal(other.t, controls.now);
    release(answer);
    assert.equal(await pending, RunStatus.Outcome);
    const raw = job.takeRawResult(); const result = rawCells(raw.body); assert.equal(JSON.parse(raw.executionReceiptJson).state, "started"); raw.free();
    const replay = rawCells(await engine.queryRawAsync(data, query, { ...INPUTS, resolveService: async () => answer }).then(result => result.body));
    assert.deepEqual(result, replay);
    assert.equal(result.t, NOW);
    assert.notEqual(result.before, result.after);
    assert.notEqual(result.before, other.before);
  } finally {
    if (release) release(answer);
    await pending;
    job.finish(); job.free(); data.free(); engine.free();
  }
});

test("async jobs select omitted coordinates anew when validated options are reused", async () => {
  const engine = new QueryEngine();
  const data = new Dataset();
  const options = AsyncJobOptions.fromJs(AsyncOperationKind.Raw, {}, undefined, false, undefined);
  const dateNow = Date.now;
  const random = Math.random;
  let draws = 0;
  Math.random = () => { draws += 1; return draws / 100; };
  const run = async (milliseconds) => {
    Date.now = () => milliseconds;
    const job = engine.beginAsync(data, AsyncOperationKind.Raw, QUERY, options);
    try {
      assert.equal(await runJob(job, {}), RunStatus.Outcome);
      const raw = job.takeRawResult(); try { return rawCells(raw.body); } finally { raw.free(); }
    } finally { job.finish(); job.free(); }
  };
  try {
    const first = await run(946684800000);
    const firstDraws = draws;
    const second = await run(978307200000);
    assert.notEqual(first.t, second.t);
    assert.notEqual(first.r, second.r);
    assert.ok(firstDraws > 0 && draws > firstDraws);
  } finally {
    Date.now = dateNow; Math.random = random;
    options.free(); data.free(); engine.free();
  }
});

test("Cloudflare query coordinates reach negotiated output", async () => {
  const engine = new QueryEngine();
  const dataset = new Dataset();
  const options = { engine, dataset, governors: { deadlineMs: 60000 }, ...INPUTS };
  const request = (text, type) => new Request("https://example.org/sparql", { method: "POST", headers: { "Content-Type": type }, body: text });
  try {
    const response = await handleSparqlRequest(request(QUERY, "application/sparql-query"), options);
    assert.equal(response.status, 200); assert.equal(JSON.parse(response.headers.get("Purrdf-Execution-Receipt")).state, "started");
    assert.deepEqual(rawCells(await response.text()), rawCells(await engine.queryRawAsync(dataset, QUERY, INPUTS).then(result => result.body)));
    assert.equal(dataset.size, 0);
  } finally { dataset.free(); engine.free(); }
});
