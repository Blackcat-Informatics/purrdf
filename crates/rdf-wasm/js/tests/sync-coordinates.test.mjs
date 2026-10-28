// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: these cases exercise the shipped JavaScript wrappers, wasm-bindgen
// positional ABI, checked BigInt conversion and actual host clock/entropy imports.
import { test } from "node:test";
import assert from "node:assert/strict";
import { CompiledJsonLdContext, Dataset, QueryEngine, ready } from "../index.mjs";

// Capture the generated positional exports before ready installs package wrappers.
const raw = Object.fromEntries([
  "query", "select", "ask", "construct", "describe", "queryRaw", "queryRawConfigured",
  "queryRawWithContext", "queryGoverned", "queryEntailmentGoverned", "explainQuery",
].map((name) => [name, QueryEngine.prototype[name]]));
await ready();
const NOW = `-12345-06-07T08:09:10.${"123456789".repeat(128)}7+05:30`;
const INPUTS = { now: NOW, entropySeed: 18446744073709551615n };
const QUERY = "SELECT (NOW() AS ?t) (RAND() AS ?r) (UUID() AS ?u) (STRUUID() AS ?s) WHERE {}";
const GRAPH = "CONSTRUCT { <https://example.org/s> <https://example.org/time> ?t } WHERE { BIND(NOW() AS ?t) }";
const FILTER = `FILTER(NOW() = "${NOW}"^^<http://www.w3.org/2001/XMLSchema#dateTime>)`;
const cells = (result) => Object.fromEntries(
  Object.entries(result.rows.take(0)).map(([name, term]) => [name, term.value]),
);
const rawCells = (text) => Object.fromEntries(
  Object.entries(JSON.parse(text).results.bindings[0]).map(([name, term]) => [name, term.value]),
);

test("synchronous coordinates cross every query export and package output", () => {
  const engine = new QueryEngine();
  const data = Dataset.parse('<https://example.org/s> <https://example.org/p> "present" .', "ntriples");
  const optionsJson = JSON.stringify({ version: 1, mode: "context", prefixes: { ex: "https://example.org/" } });
  const context = new CompiledJsonLdContext(optionsJson);
  const random = Math.random;
  let draws = 0;
  Math.random = () => { draws += 1; return 0.25; };
  try {
    const expected = cells(engine.query(data, QUERY, INPUTS));
    assert.equal(expected.t, NOW);
    assert.deepEqual(cells(engine.select(data, QUERY, INPUTS)), expected);
    assert.equal(engine.ask(data, `ASK { ${FILTER} }`, INPUTS), true);
    for (const [method, query] of [["construct", GRAPH], ["describe", `DESCRIBE <https://example.org/s> WHERE { ${FILTER} }`]]) {
      const graph = engine[method](data, query, INPUTS);
      try {
        assert.equal(graph.size, 1);
        if (method === "construct") assert.ok(graph.serialize("ntriples").includes(NOW));
      } finally { graph.free(); }
    }
    assert.deepEqual(rawCells(engine.queryRaw(data, QUERY, INPUTS)), expected);
    assert.deepEqual(rawCells(data.query(QUERY, undefined, undefined, NOW, INPUTS.entropySeed)), expected);
    const provenanceNamespace = { prefix: "prov", iri: "https://example.org/provenance#" };
    const serialized = engine.queryRaw(data, QUERY, { ...INPUTS, provenanceNamespace });
    assert.deepEqual(rawCells(serialized), expected);
    assert.ok(serialized.includes(provenanceNamespace.iri));
    const configured = engine.queryRawConfigured(data, GRAPH, undefined, "jsonld", optionsJson, undefined, NOW, INPUTS.entropySeed);
    assert.ok(configured.includes(NOW));
    assert.equal(engine.queryRawWithContext(data, GRAPH, undefined, "jsonld", context, undefined, undefined, NOW, INPUTS.entropySeed), configured);
    const governed = engine.queryGoverned(data, QUERY, { ...INPUTS, fuel: 100000 });
    assert.equal(governed.isComplete, true);
    assert.deepEqual(cells(governed.result), expected);
    const entailed = engine.queryEntailmentGoverned(data, QUERY, "rdfs", INPUTS);
    assert.equal(entailed.phase, "answered");
    assert.deepEqual(cells(entailed.outcome.result), expected);
    assert.match(engine.explainQuery(data, `SELECT ?t WHERE { BIND(NOW() AS ?t) ${FILTER} }`, INPUTS), /Filter fuel=\d+ rows=1 /);
    assert.equal(draws, 0, "every supplied seed avoids the actual host entropy import");
  } finally { Math.random = random; context.free(); data.free(); engine.free(); }
});

test("synchronous positional coordinates are checked and UPDATE controls are refused", () => {
  const engine = new QueryEngine();
  const data = new Dataset();
  const optionsJson = JSON.stringify({ version: 1, mode: "context", prefixes: { ex: "https://example.org/" } });
  const context = new CompiledJsonLdContext(optionsJson);
  const prefixes = {
    query: [data, QUERY, undefined, undefined], select: [data, QUERY, undefined, undefined],
    ask: [data, "ASK {}", undefined, undefined], construct: [data, GRAPH, undefined, undefined],
    describe: [data, "DESCRIBE <https://example.org/s>", undefined, undefined],
    queryRaw: [data, QUERY, undefined, undefined, undefined, undefined, undefined],
    queryRawConfigured: [data, GRAPH, undefined, "jsonld", optionsJson, undefined],
    queryRawWithContext: [data, GRAPH, undefined, "jsonld", context, undefined, undefined],
    queryGoverned: [data, QUERY, ...Array(11).fill(undefined)],
    queryEntailmentGoverned: [data, QUERY, undefined, "rdfs", undefined, [], [], [], ...Array(13).fill(undefined)],
    explainQuery: [data, QUERY, undefined, undefined],
  };
  try {
    const refused = (error) => error instanceof TypeError && error.code === "purrdf-wasm-options" && /entropySeed/.test(error.message);
    for (const seed of [-1n, 18446744073709551616n, -1, 1.5, Number.MAX_SAFE_INTEGER + 1, NaN, Infinity, "7", true]) {
      for (const [name, prefix] of Object.entries(prefixes)) {
        assert.throws(() => raw[name].call(engine, ...prefix, NOW, seed), refused, name);
      }
      assert.throws(() => data.query(QUERY, undefined, undefined, NOW, seed), refused, "Dataset.query");
    }
    assert.throws(() => engine.query(data, QUERY, { now: "not a date" }), (error) => error.code === "purrdf-wasm-options" && /now/.test(error.message));
    for (const now of [7, new Date(), true]) {
      const refusedTime = (error) => error instanceof TypeError && error.code === "purrdf-wasm-options" && /now/.test(error.message);
      for (const [name, prefix] of Object.entries(prefixes)) {
        assert.throws(() => raw[name].call(engine, ...prefix, now, 0n), refusedTime, name);
      }
      assert.throws(() => data.query(QUERY, undefined, undefined, now, 0n), refusedTime, "Dataset.query");
      assert.throws(() => engine.query(data, QUERY, { now }), refusedTime);
    }
    const zero = rawCells(engine.queryRaw(data, QUERY, { now: NOW, entropySeed: 0n }));
    assert.deepEqual(rawCells(engine.queryRaw(data, QUERY, { now: NOW, entropySeed: 0 })), zero);
    assert.notEqual(rawCells(engine.queryRaw(data, QUERY, INPUTS)).r, zero.r);
    for (const method of ["update", "updateGoverned"]) {
      for (const controls of [{ now: NOW }, { entropySeed: 0n }]) {
        assert.throws(() => engine[method](data, "INSERT DATA {}", controls), (error) => error instanceof TypeError && error.code === "purrdf-wasm-options" && /UPDATE option/.test(error.message));
      }
    }
    const stopped = engine.queryGoverned(data, QUERY, { ...INPUTS, fuel: 0 });
    assert.equal(stopped.isComplete, false);
    assert.equal(stopped.tripped.dimension, "fuel");
    assert.equal(data.size, 0);
  } finally { context.free(); data.free(); engine.free(); }
});

test("synchronous omitted coordinates use fresh independent host defaults", () => {
  const engine = new QueryEngine();
  const data = new Dataset();
  const dateNow = Date.now;
  const random = Math.random;
  let draws = 0;
  Math.random = () => { draws += 1; return draws / 100; };
  try {
    Date.now = () => 946684800000;
    const first = rawCells(engine.queryRaw(data, QUERY));
    const firstDraws = draws;
    Date.now = () => 978307200000;
    const second = rawCells(engine.queryRaw(data, QUERY, { now: null, entropySeed: null }));
    assert.notEqual(first.t, second.t);
    assert.notEqual(first.r, second.r);
    assert.ok(firstDraws > 0 && draws > firstDraws);
    const timeOnly = rawCells(engine.queryRaw(data, QUERY, { now: NOW }));
    assert.equal(timeOnly.t, NOW);
    const beforeSeedOnly = draws;
    const seedOnly = rawCells(engine.queryRaw(data, QUERY, { entropySeed: 0n }));
    assert.equal(seedOnly.t, second.t);
    assert.equal(draws, beforeSeedOnly);
    assert.equal(rawCells(data.query(QUERY, undefined, undefined, null, null)).t, second.t);
  } finally { Date.now = dateNow; Math.random = random; data.free(); engine.free(); }
});
