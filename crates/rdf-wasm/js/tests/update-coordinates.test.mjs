// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: these cases exercise the shipped JavaScript UPDATE options, raw
// wasm-bindgen ABI, JSPI suspension/claims and Cloudflare request/response protocol.
import { test } from "node:test";
import assert from "node:assert/strict";
import { Dataset, QueryEngine, ready } from "../index.mjs";
import { AsyncJobOptions, AsyncOperationKind, RunStatus } from "../pkg/purrdf_wasm.js";
import { runJob } from "../pkg/purrdf_jspi.mjs";
import { handleSparqlRequest } from "../cloudflare.mjs";

const rawUpdate = QueryEngine.prototype.update;
const rawGoverned = QueryEngine.prototype.updateGoverned;
await ready();
const NOW = `-12345-06-07T08:09:10.${"123456789".repeat(128)}7+05:30`;
const INPUTS = { now: NOW, entropySeed: 18446744073709551615n };
const LAWS = [undefined, "xpath-2.0-2010-12-14", "xpath-3.1-2017-03-21"];
const operation = (subject, service = "") => `INSERT { <https://example.org/${subject}> <https://example.org/time> ?t; <https://example.org/random> ?r; <https://example.org/uuid> ?u; <https://example.org/string> ?s } WHERE { BIND(NOW() AS ?t) BIND(RAND() AS ?r) ${service} BIND(UUID() AS ?u) BIND(STRUUID() AS ?s) FILTER(REGEX("aa", "a+")) }`;
const UPDATE = `${operation("first")}; ${operation("second")}`;
const SUSPENDING = `${operation("first")}; ${operation("second", "SERVICE <https://example.org/service> { ?x ?p ?o }")}`;
const ANSWER = JSON.stringify({ head: { vars: [] }, results: { bindings: [{}] } });
const snapshot = (dataset) => dataset.serialize("ntriples").trim().split("\n").sort();
const methods = ["update", "updateGoverned", "updateAsync", "updateGovernedAsync"];
const kinds = [AsyncOperationKind.Update, AsyncOperationKind.UpdateGoverned];

async function lowLevel(engine, data, kind, text, inputs) {
  const options = AsyncJobOptions.fromJs(kind, inputs, undefined, false, undefined);
  let job;
  try {
    job = engine.beginAsync(data, kind, text, options);
    assert.equal(await runJob(job, {}), RunStatus.Outcome);
    job.commitUpdate(data);
  } finally { if (job) { job.finish(); job.free(); } options.free(); }
}
const routes = [
  ...methods.map((name) => [name, async (engine, data, text, inputs) => {
    const result = await engine[name](data, text, inputs);
    if (name.includes("Governed")) assert.equal(result.isApplied, true);
    else { assert.equal(result.dataset, data); assert.equal(result.executionReceipt.state, "started"); }
  }]),
  ["raw update", (engine, data, text, o) => rawUpdate.call(engine, data, text, undefined, o.xpathRegex, o.now, o.entropySeed)],
  ["raw governed", (engine, data, text, o) => {
    const result = rawGoverned.call(engine, data, text, ...Array(10).fill(undefined), o.xpathRegex, o.now, o.entropySeed);
    try { assert.equal(result.isApplied, true); } finally { result.free(); }
  }],
  ...kinds.map((kind) => [`low-level ${kind}`, (engine, data, text, o) => lowLevel(engine, data, kind, text, o)]),
];

test("UPDATE coordinates cross every package raw and low-level host route", async () => {
  const engine = new QueryEngine();
  const random = Math.random;
  let draws = 0;
  Math.random = () => { draws += 1; return 0.25; };
  try {
    for (const xpathRegex of LAWS) for (const entropySeed of [0n, INPUTS.entropySeed]) {
      let expected;
      for (const [name, run] of routes) {
        const data = new Dataset();
        try {
          await run(engine, data, UPDATE, { now: NOW, entropySeed, xpathRegex });
          const actual = snapshot(data);
          assert.equal(actual.length, 8, name);
          assert.equal(actual.filter((line) => line.includes(NOW)).length, 2, name);
          if (expected) assert.deepEqual(actual, expected, name);
          else expected = actual;
        } finally { data.free(); }
      }
    }
    assert.equal(draws, 0, "supplied seeds avoid the actual host entropy import");
  } finally { Math.random = random; engine.free(); }
});

test("UPDATE host coordinate errors precede mutation and release every claim", async () => {
  const engine = new QueryEngine();
  try {
    for (const [name, run] of routes) {
      const data = new Dataset();
      try {
        const invalid = [
          ...[7, new Date(), true, "not a date"].map((now) => ({ ...INPUTS, now })),
          ...[-1n, 18446744073709551616n, Number.MAX_SAFE_INTEGER + 1, 1.5, "7"].map((entropySeed) => ({ ...INPUTS, entropySeed })),
        ];
        for (const controls of invalid) {
          await assert.rejects(async () => run(engine, data, UPDATE, controls), (error) => error.code === "purrdf-wasm-options", name);
          assert.equal(data.size, 0, name);
        }
        await run(engine, data, UPDATE, INPUTS);
        assert.equal(data.size, 8, `${name} remains usable after refusals`);
      } finally { data.free(); }
    }
  } finally { engine.free(); }
});

test("UPDATE omitted coordinates reach independent host clock and entropy defaults", async () => {
  const engine = new QueryEngine();
  const dateNow = Date.now;
  const random = Math.random;
  let draws = 0;
  Math.random = () => { draws += 1; return draws / 1000; };
  try {
    for (const method of methods) {
      const run = async (controls) => {
        const data = new Dataset();
        try { await engine[method](data, operation("one"), controls); return snapshot(data); }
        finally { data.free(); }
      };
      Date.now = () => 946684800000;
      const first = await run(undefined);
      const firstDraws = draws;
      Date.now = () => 978307200000;
      const second = await run({ now: null, entropySeed: null });
      assert.notDeepEqual(first, second);
      assert.ok(firstDraws > 0 && draws > firstDraws);
      assert.ok((await run({ now: NOW })).some((line) => line.includes(NOW)));
      const beforeSeedOnly = draws;
      const seedOnly = await run({ entropySeed: 0n });
      assert.equal(draws, beforeSeedOnly);
      assert.ok(seedOnly.some((line) => line.includes("2001-01-01T00:00:00")));
    }
  } finally { Date.now = dateNow; Math.random = random; engine.free(); }
});

test("UPDATE low-level coordinates survive suspension freed options and interleaved mutation", async () => {
  const engine = new QueryEngine();
  try {
    for (const kind of kinds) {
      const data = new Dataset();
      const other = new Dataset();
      const replay = new Dataset();
      const controls = { ...INPUTS };
      const options = AsyncJobOptions.fromJs(kind, controls, 1_000_000, false, undefined);
      const job = engine.beginAsync(data, kind, SUSPENDING, options);
      options.free();
      controls.now = "2000-01-01T00:00:00Z";
      controls.entropySeed = 0n;
      let entered, release;
      const suspended = new Promise((resolve) => { entered = resolve; });
      const pending = runJob(job, { resolveService: () => new Promise((resolve) => { release = resolve; entered(); }) });
      try {
        await suspended;
        assert.equal(data.size, 0, "suspended updates have not published their first operation");
        await engine.updateAsync(other, SUSPENDING, { ...controls, resolveService: async () => ANSWER }).then(result => result.dataset);
        release(ANSWER);
        assert.equal(await pending, RunStatus.Outcome);
        job.commitUpdate(data);
        await engine.updateGovernedAsync(replay, SUSPENDING, { ...INPUTS, resolveService: async () => ANSWER });
        assert.deepEqual(snapshot(data), snapshot(replay));
        assert.notDeepEqual(snapshot(data), snapshot(other));
      } finally {
        if (release) release(ANSWER);
        await pending;
        job.finish(); job.free(); data.free(); other.free(); replay.free();
      }
    }
  } finally { engine.free(); }
});

test("UPDATE supplied coordinates preserve asynchronous abort atomicity and reuse", async () => {
  const engine = new QueryEngine();
  const data = new Dataset();
  const controller = new AbortController();
  const reason = new Error("stop supplied UPDATE while host is suspended");
  let entered, release;
  const suspended = new Promise((resolve) => { entered = resolve; });
  const pending = engine.updateAsync(data, SUSPENDING, { ...INPUTS, signal: controller.signal,
    resolveService: () => new Promise((resolve) => { release = resolve; entered(); }),
  }).then(result => result.dataset);
  const rejected = assert.rejects(pending, (error) => error.cause === reason);
  try {
    await suspended;
    controller.abort(reason);
    release(ANSWER);
    await rejected;
    assert.equal(data.size, 0);
    await engine.updateAsync(data, UPDATE, INPUTS).then(result => result.dataset);
    assert.equal(data.size, 8);
  } finally { if (release) release(ANSWER); await rejected; data.free(); engine.free(); }
});

test("Cloudflare UPDATE coordinates reach mutation and typed refusal responses", async () => {
  const engine = new QueryEngine();
  const dataset = new Dataset();
  const errors = [];
  const options = { engine, dataset, governors: { deadlineMs: 60000 }, ...INPUTS,
    onInternalError: (error) => errors.push(error),
  };
  const request = (text, type = "application/sparql-update") => new Request("https://example.org/sparql", { method: "POST", headers: { "Content-Type": type }, body: text });
  try {
    const response = await handleSparqlRequest(request(UPDATE), options);
    assert.equal(response.status, 204); assert.equal(JSON.parse(response.headers.get("Purrdf-Execution-Receipt")).state, "started");
    const read = await handleSparqlRequest(request("SELECT ?t WHERE { <https://example.org/first> <https://example.org/time> ?t }", "application/sparql-query"), options);
    assert.equal(read.status, 200); assert.equal(JSON.parse(read.headers.get("Purrdf-Execution-Receipt")).state, "started");
    assert.equal((await read.json()).results.bindings[0].t.value, NOW);
    const before = snapshot(dataset);
    for (const controls of [{ now: 7 }, { now: "not a date" }, { entropySeed: -1n }]) {
      const refused = await handleSparqlRequest(request(operation("bad")), { ...options, ...controls });
      assert.equal(refused.status, 500, "bad endpoint configuration is the host's fault");
      assert.equal(errors.at(-1).code, "purrdf-wasm-options");
      assert.ok((await refused.json()).correlationId);
      assert.deepEqual(snapshot(dataset), before);
    }
  } finally { dataset.free(); engine.free(); }
});
