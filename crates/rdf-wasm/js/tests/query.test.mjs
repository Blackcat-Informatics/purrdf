// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Node real-execution conformance for the OFFLINE SPARQL surface (Dataset.query):
// drives the ACTUAL compiled wasm evaluator through SELECT / ASK / CONSTRUCT and the
// SERVICE hard-fail path, exactly as the docs SPARQL playground runs it in a browser.

import { test } from "node:test";
import assert from "node:assert/strict";

import { ready, Dataset, QueryEngine } from "../index.mjs";

// One-time wasm instantiation before any test runs.
await ready();

// A tiny two-graph TriG asset, the shape the docs playground loads offline.
const TRIG = `
@prefix ex: <https://e/> .
ex:a ex:knows ex:b .
ex:a ex:name "Ann" .
ex:b ex:name "Bob" .
graph <https://e/g> { ex:c ex:knows ex:a . }
`;

test("SELECT returns SPARQL Results JSON bindings", () => {
  const ds = Dataset.parse(TRIG, "trig");
  const json = JSON.parse(
    ds.query("PREFIX ex: <https://e/> SELECT ?name WHERE { ?p ex:name ?name } ORDER BY ?name"),
  );
  assert.deepEqual(json.head.vars, ["name"]);
  const names = json.results.bindings.map((b) => b.name.value);
  assert.deepEqual(names, ["Ann", "Bob"]);
});

test("SELECT over the default graph does not see named-graph triples", () => {
  const ds = Dataset.parse(TRIG, "trig");
  const json = JSON.parse(
    ds.query("PREFIX ex: <https://e/> SELECT ?o WHERE { ?s ex:knows ?o }"),
  );
  // Only ex:a ex:knows ex:b is in the default graph; ex:c ex:knows ex:a is in <g>.
  const objs = json.results.bindings.map((b) => b.o.value);
  assert.deepEqual(objs, ["https://e/b"]);
});

test("ASK returns a boolean result document", () => {
  const ds = Dataset.parse(TRIG, "trig");
  const yes = JSON.parse(ds.query("PREFIX ex: <https://e/> ASK { ex:a ex:knows ex:b }"));
  assert.equal(yes.boolean, true);
  const no = JSON.parse(ds.query("PREFIX ex: <https://e/> ASK { ex:b ex:knows ex:a }"));
  assert.equal(no.boolean, false);
});

test("QueryEngine SELECT returns typed package-root bindings", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  const result = engine.select(
    ds,
    "PREFIX ex: <https://e/> SELECT ?person ?name WHERE { ?person ex:name ?name } ORDER BY ?name",
  );
  assert.equal(result.kind, "select");
  assert.deepEqual(result.variables, ["person", "name"]);
  assert.equal(result.rows.length, 2);
  assert.equal(result.rows[0].person.termType, "NamedNode");
  assert.equal(result.rows[0].person.value, "https://e/a");
  assert.equal(result.rows[0].name.termType, "Literal");
  assert.equal(result.rows[0].name.value, "Ann");
});

test("QueryEngine query routes ASK and graph results into discriminated objects", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  const ask = engine.query(ds, "PREFIX ex: <https://e/> ASK { ex:a ex:knows ex:b }");
  assert.deepEqual(ask, { kind: "ask", boolean: true });

  const graph = engine.query(
    ds,
    "PREFIX ex: <https://e/> CONSTRUCT { ?p ex:label ?name } WHERE { ?p ex:name ?name }",
  );
  assert.equal(graph.kind, "graph");
  assert.equal(graph.dataset.size, 2);
});

test("QueryEngine raw serialization supports result and graph formats", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  const xml = engine.queryRaw(ds, "PREFIX ex: <https://e/> ASK { ex:a ex:knows ex:b }", {
    format: "xml",
  });
  assert.match(xml, /^<\?xml/);

  const nquads = engine.queryRaw(
    ds,
    "PREFIX ex: <https://e/> CONSTRUCT { ?p ex:label ?name } WHERE { ?p ex:name ?name }",
    { format: "nquads" },
  );
  assert.match(nquads, /https:\/\/e\/label/);

  assert.throws(() =>
    engine.queryRaw(ds, "PREFIX ex: <https://e/> ASK { ex:a ex:knows ex:b }", {
      format: "nquads",
    }),
  );
});

test("CONSTRUCT returns Turtle", () => {
  const ds = Dataset.parse(TRIG, "trig");
  const ttl = ds.query(
    "PREFIX ex: <https://e/> CONSTRUCT { ?p ex:label ?name } WHERE { ?p ex:name ?name }",
  );
  // The result is Turtle text (not JSON); re-parse it to prove it is well-formed.
  const back = Dataset.parse(ttl, "turtle");
  assert.equal(back.size, 2);
});

test("a malformed query throws, never a silent empty result", () => {
  const ds = Dataset.parse(TRIG, "trig");
  assert.throws(() => ds.query("SELECT ?x WHERE { this is not sparql"));
});

test("a SERVICE clause hard-fails offline (no resolver in the browser)", () => {
  const ds = Dataset.parse(TRIG, "trig");
  assert.throws(() =>
    ds.query(
      "PREFIX ex: <https://e/> SELECT ?o WHERE { SERVICE <https://remote/sparql> { ?s ex:knows ?o } }",
    ),
  );
});

test("QueryEngine UPDATE mutates atomically and LOAD hard-fails without a resolver", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(
    "@prefix ex: <https://e/> . ex:a ex:p ex:b .",
    "turtle",
  );
  const before = ds.canonicalize();

  assert.equal(
    engine.update(
      ds,
      "INSERT DATA { <https://e/c> <https://e/p> <https://e/d> }",
    ),
    ds,
  );
  assert.equal(ds.size, 2);

  const stable = ds.canonicalize();
  assert.throws(() =>
    engine.update(
      ds,
      "INSERT DATA { <https://e/x> <https://e/p> <https://e/y> } ; LOAD <https://e/doc>",
    ),
  );
  assert.equal(ds.canonicalize(), stable);
  assert.notEqual(ds.canonicalize(), before);
});

test("serialize supports JSON-LD (the docs 'copy as' transcode surface)", () => {
  const ds = Dataset.parse('@prefix ex: <https://e/> . ex:a ex:p ex:o .', "turtle");
  const jsonld = ds.serialize("jsonld");
  const doc = JSON.parse(jsonld); // must be valid JSON
  assert.ok(
    JSON.stringify(doc).includes("https://e/"),
    "the JSON-LD document must carry the term IRIs",
  );
});
