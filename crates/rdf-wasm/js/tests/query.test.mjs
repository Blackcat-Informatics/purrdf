// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Node real-execution conformance for the OFFLINE SPARQL surface (Dataset.query):
// drives the ACTUAL compiled wasm evaluator through SELECT / ASK / CONSTRUCT and the
// SERVICE hard-fail path, exactly as the docs SPARQL playground runs it in a browser.

import { test } from "node:test";
import assert from "node:assert/strict";

import { ready, Dataset, QueryEngine, provenanceFromJson, provenanceFromXml } from "../index.mjs";

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
  assert.equal(result.rowCount, 2);
  assert.equal(result.rows.length, 2);
  assert.equal(result.rows.remaining, 2);
  const first = result.rows.take(0);
  assert.equal(first.person.termType, "NamedNode");
  assert.equal(first.person.value, "https://e/a");
  assert.equal(first.name.termType, "Literal");
  assert.equal(first.name.value, "Ann");
  assert.deepEqual([...result.rows].map((row) => row.name.value), ["Bob"]);
  assert.equal(result.rows.remaining, 0);
});

test("QueryEngine SELECT rows are a single-owner stream", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  const result = engine.select(
    ds,
    "PREFIX ex: <https://e/> SELECT ?name WHERE { ?p ex:name ?name } ORDER BY ?name",
  );
  assert.deepEqual(result.rows.toArray().map((row) => row.name.value), ["Ann", "Bob"]);
  assert.deepEqual(result.rows.toArray(), []);
  assert.equal(result.rows.take(0), undefined);
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

test("queryRaw provenanceNamespace populates and round-trips through JSON", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  const query = "PREFIX ex: <https://e/> SELECT ?name WHERE { ?p ex:name ?name } ORDER BY ?name";

  const json = engine.queryRaw(ds, query, {
    format: "json",
    provenanceNamespace: { prefix: "prov", iri: "https://example.org/ns/prov#" },
  });
  assert.match(json, /"prov":\{/);
  assert.match(json, /"engine":"purrdf-sparql-eval"/);
  assert.match(json, /"queryHash":"sha256:/);

  const decoded = provenanceFromJson(json, "prov", "https://example.org/ns/prov#");
  assert.equal(decoded.engine, "purrdf-sparql-eval");
  assert.ok(decoded.queryHash.startsWith("sha256:"));
  decoded.free();
});

test("queryRaw provenanceNamespace populates and round-trips through XML", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  const query = "PREFIX ex: <https://e/> SELECT ?name WHERE { ?p ex:name ?name } ORDER BY ?name";

  const xml = engine.queryRaw(ds, query, {
    format: "xml",
    provenanceNamespace: { prefix: "prov", iri: "https://example.org/ns/prov#" },
  });
  assert.match(xml, /<prov:provenance/);

  const decoded = provenanceFromXml(xml, "prov", "https://example.org/ns/prov#");
  assert.equal(decoded.engine, "purrdf-sparql-eval");
  decoded.free();
});

test("omitting provenanceNamespace emits pure W3C output", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  const json = engine.queryRaw(
    ds,
    "PREFIX ex: <https://e/> SELECT ?name WHERE { ?p ex:name ?name }",
    { format: "json" },
  );
  assert.ok(!json.includes('"prov"'));
});

test("a lone provenanceNamespace half is refused", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  assert.throws(() =>
    engine.queryRaw(ds, "PREFIX ex: <https://e/> ASK { ex:a ex:knows ex:b }", {
      provenanceNamespace: { prefix: "prov" },
    }),
  );
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

// ── A quad-template CONSTRUCT, through the DEFAULT entry point ──────────────────
//
// `Dataset#query` passes no format at all, and the documented default was Turtle.
// Turtle has no `GRAPH` construct, so a CONSTRUCT whose template names a graph
// serialized to a well-formed EMPTY document and returned it with no error — the exact
// "silent empty result" the method's own contract says can never happen.
//
// The default now widens to TriG (Turtle's dataset superset) for a result that carries
// a named graph, and stays Turtle for one that does not. Naming a single-graph syntax
// EXPLICITLY throws instead: that caller asked for a syntax, most likely because
// something downstream reads only that syntax, so neither answering with TriG bytes nor
// answering with Turtle bytes that omit the query's own statements is honest.

const GRAPH_CONSTRUCT =
  "PREFIX ex: <https://e/> CONSTRUCT { GRAPH ex:out { ?s ex:knows ?o } } WHERE { ?s ex:knows ?o }";
const PLAIN_CONSTRUCT =
  "PREFIX ex: <https://e/> CONSTRUCT { ?s ex:knows ?o } WHERE { ?s ex:knows ?o }";

test("the default query() format never returns an empty string for a named-graph CONSTRUCT", () => {
  const ds = Dataset.parse(TRIG, "trig");
  const out = ds.query(GRAPH_CONSTRUCT);
  assert.notEqual(out.trim(), "", "the documented default must never be a silent empty result");
  assert.ok(out.includes("https://e/out"), `the graph the query named must survive: ${out}`);
  assert.ok(out.includes("https://e/a"), `the constructed statement must survive: ${out}`);
  // TriG round-trips back into a dataset that still carries the graph.
  const reparsed = Dataset.parse(out, "trig");
  assert.equal(reparsed.size, 1);
  assert.equal(reparsed.quads()[0].graph.value, "https://e/out");
});

test("the default query() format is still Turtle for a default-graph CONSTRUCT", () => {
  const ds = Dataset.parse(TRIG, "trig");
  const engine = new QueryEngine();
  assert.equal(ds.query(PLAIN_CONSTRUCT), engine.queryRaw(ds, PLAIN_CONSTRUCT, { format: "turtle" }));
});

test("an explicit single-graph format throws for a named-graph CONSTRUCT", () => {
  const engine = new QueryEngine();
  for (const format of ["turtle", "ntriples", "rdfxml"]) {
    const ds = Dataset.parse(TRIG, "trig");
    assert.throws(
      () => engine.queryRaw(ds, GRAPH_CONSTRUCT, { format }),
      (error) => {
        assert.ok(
          error.message.includes("carrying 1 named graph (<https://e/out>)"),
          `the refusal names the graph: ${error.message}`,
        );
        assert.ok(
          error.message.includes(format),
          `the refusal names the offending format: ${error.message}`,
        );
        assert.ok(
          error.message.includes("trig/nquads/trix/hextuples/jsonld/yamlld"),
          `the refusal names the alternatives: ${error.message}`,
        );
        return true;
      },
      `${format} must refuse a graph-carrying result`,
    );
  }
});

test("an explicit quad-capable format carries a named-graph CONSTRUCT", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  const nquads = engine.queryRaw(ds, GRAPH_CONSTRUCT, { format: "nquads" });
  assert.ok(nquads.includes("<https://e/out> ."), nquads);
});

test("an explicit single-graph format still serializes a default-graph CONSTRUCT", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  const ntriples = engine.queryRaw(ds, PLAIN_CONSTRUCT, { format: "ntriples" });
  assert.equal(
    ntriples.trim(),
    "<https://e/a> <https://e/knows> <https://e/b> .",
  );
});

// ── The transcode lane counts what it drops ────────────────────────────────────
//
// `Dataset#serialize` cannot refuse the way the CONSTRUCT lane does — asking a TriG
// document for N-Triples is a legitimate "give me the default graph" — so the only
// honest alternative is to make the loss readable. `serializeWithLoss` is the same
// serialization with the three realized counts attached, the JS twin of the C ABI's
// `purrdf_serialize` out-params and Python's `Store.dump_with_loss`.

test("serializeWithLoss reports the named-graph rows a single-graph syntax drops", () => {
  const ds = Dataset.parse(TRIG, "trig");
  const lossy = ds.serializeWithLoss("ntriples");
  // N-Triples is star-capable, so the statement-layer count is silent about graphs…
  assert.equal(lossy.statementRowsDropped, 0);
  assert.equal(lossy.directionalLiteralsDropped, 0);
  // …and the named-graph count is the one that reports the vanished row.
  assert.equal(lossy.namedGraphRowsDropped, 1);
  assert.ok(!lossy.text.includes("https://e/g"), lossy.text);
  // The bytes are exactly what the plain entry point produces.
  assert.equal(lossy.text, ds.serialize("ntriples"));
  lossy.free();

  const lossless = ds.serializeWithLoss("nquads");
  assert.equal(lossless.statementRowsDropped, 0);
  assert.equal(lossless.directionalLiteralsDropped, 0);
  assert.equal(lossless.namedGraphRowsDropped, 0);
  assert.ok(lossless.text.includes("https://e/g"), lossless.text);
  lossless.free();
});
