// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Node real-execution conformance for the OFFLINE SPARQL surface (Dataset.query):
// drives the ACTUAL compiled wasm evaluator through SELECT / ASK / CONSTRUCT and the
// SERVICE hard-fail path, exactly as the docs SPARQL playground runs it in a browser.

import { test } from "node:test";
import assert from "node:assert/strict";

import { ready, Dataset, QueryEngine, provenanceFromJson, provenanceFromXml } from "../index.mjs";
import { NESTING_SHAPES, NUMBERS, attempt, realLimit } from "./fixtures/nesting.mjs";

// One-time wasm instantiation before any test runs.
await ready();

// A tiny two-graph TriG asset, the shape the docs playground loads offline.
const TRIG = `
@prefix ex: <https://example.org/> .
ex:a ex:knows ex:b .
ex:a ex:name "Ann" .
ex:b ex:name "Bob" .
graph <https://example.org/g> { ex:c ex:knows ex:a . }
`;

test("SELECT returns SPARQL Results JSON bindings", () => {
  const ds = Dataset.parse(TRIG, "trig");
  const json = JSON.parse(
    ds.query("PREFIX ex: <https://example.org/> SELECT ?name WHERE { ?p ex:name ?name } ORDER BY ?name"),
  );
  assert.deepEqual(json.head.vars, ["name"]);
  const names = json.results.bindings.map((b) => b.name.value);
  assert.deepEqual(names, ["Ann", "Bob"]);
});

test("SELECT over the default graph does not see named-graph triples", () => {
  const ds = Dataset.parse(TRIG, "trig");
  const json = JSON.parse(
    ds.query("PREFIX ex: <https://example.org/> SELECT ?o WHERE { ?s ex:knows ?o }"),
  );
  // Only ex:a ex:knows ex:b is in the default graph; ex:c ex:knows ex:a is in <g>.
  const objs = json.results.bindings.map((b) => b.o.value);
  assert.deepEqual(objs, ["https://example.org/b"]);
});

test("ASK returns a boolean result document", () => {
  const ds = Dataset.parse(TRIG, "trig");
  const yes = JSON.parse(ds.query("PREFIX ex: <https://example.org/> ASK { ex:a ex:knows ex:b }"));
  assert.equal(yes.boolean, true);
  const no = JSON.parse(ds.query("PREFIX ex: <https://example.org/> ASK { ex:b ex:knows ex:a }"));
  assert.equal(no.boolean, false);
});

test("QueryEngine SELECT returns typed package-root bindings", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  const result = engine.select(
    ds,
    "PREFIX ex: <https://example.org/> SELECT ?person ?name WHERE { ?person ex:name ?name } ORDER BY ?name",
  );
  assert.equal(result.kind, "select");
  assert.deepEqual(result.variables, ["person", "name"]);
  assert.equal(result.rowCount, 2);
  assert.equal(result.rows.length, 2);
  assert.equal(result.rows.remaining, 2);
  const first = result.rows.take(0);
  assert.equal(first.person.termType, "NamedNode");
  assert.equal(first.person.value, "https://example.org/a");
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
    "PREFIX ex: <https://example.org/> SELECT ?name WHERE { ?p ex:name ?name } ORDER BY ?name",
  );
  assert.deepEqual(result.rows.toArray().map((row) => row.name.value), ["Ann", "Bob"]);
  assert.deepEqual(result.rows.toArray(), []);
  assert.equal(result.rows.take(0), undefined);
});

test("QueryEngine query routes ASK and graph results into discriminated objects", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  const ask = engine.query(ds, "PREFIX ex: <https://example.org/> ASK { ex:a ex:knows ex:b }");
  assert.deepEqual(ask, { kind: "ask", boolean: true });

  const graph = engine.query(
    ds,
    "PREFIX ex: <https://example.org/> CONSTRUCT { ?p ex:label ?name } WHERE { ?p ex:name ?name }",
  );
  assert.equal(graph.kind, "graph");
  assert.equal(graph.dataset.size, 2);
});

test("QueryEngine raw serialization supports result and graph formats", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  const xml = engine.queryRaw(ds, "PREFIX ex: <https://example.org/> ASK { ex:a ex:knows ex:b }", {
    format: "xml",
  });
  assert.match(xml, /^<\?xml/);

  const nquads = engine.queryRaw(
    ds,
    "PREFIX ex: <https://example.org/> CONSTRUCT { ?p ex:label ?name } WHERE { ?p ex:name ?name }",
    { format: "nquads" },
  );
  // Exact: the whole N-Quads document, not a substring that a wrong IRI could contain.
  assert.equal(
    nquads,
    '<https://example.org/a> <https://example.org/label> "Ann" .\n' +
      '<https://example.org/b> <https://example.org/label> "Bob" .\n',
  );

  assert.throws(() =>
    engine.queryRaw(ds, "PREFIX ex: <https://example.org/> ASK { ex:a ex:knows ex:b }", {
      format: "nquads",
    }),
  );
});

test("CONSTRUCT returns Turtle", () => {
  const ds = Dataset.parse(TRIG, "trig");
  const ttl = ds.query(
    "PREFIX ex: <https://example.org/> CONSTRUCT { ?p ex:label ?name } WHERE { ?p ex:name ?name }",
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
      "PREFIX ex: <https://example.org/> SELECT ?o WHERE { SERVICE <https://remote.example.org/sparql> { ?s ex:knows ?o } }",
    ),
  );
});

test("QueryEngine UPDATE mutates atomically and LOAD hard-fails without a resolver", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(
    "@prefix ex: <https://example.org/> . ex:a ex:p ex:b .",
    "turtle",
  );
  const before = ds.canonicalize();

  assert.equal(
    engine.update(
      ds,
      "INSERT DATA { <https://example.org/c> <https://example.org/p> <https://example.org/d> }",
    ),
    ds,
  );
  assert.equal(ds.size, 2);

  const stable = ds.canonicalize();
  assert.throws(() =>
    engine.update(
      ds,
      "INSERT DATA { <https://example.org/x> <https://example.org/p> <https://example.org/y> } ; LOAD <https://example.org/doc>",
    ),
  );
  assert.equal(ds.canonicalize(), stable);
  assert.notEqual(ds.canonicalize(), before);
});

test("queryRaw provenanceNamespace populates and round-trips through JSON", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  const query = "PREFIX ex: <https://example.org/> SELECT ?name WHERE { ?p ex:name ?name } ORDER BY ?name";

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
  const query = "PREFIX ex: <https://example.org/> SELECT ?name WHERE { ?p ex:name ?name } ORDER BY ?name";

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
    "PREFIX ex: <https://example.org/> SELECT ?name WHERE { ?p ex:name ?name }",
    { format: "json" },
  );
  assert.ok(!json.includes('"prov"'));
});

test("a lone provenanceNamespace half is refused", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  assert.throws(() =>
    engine.queryRaw(ds, "PREFIX ex: <https://example.org/> ASK { ex:a ex:knows ex:b }", {
      provenanceNamespace: { prefix: "prov" },
    }),
  );
});

test("serialize supports JSON-LD (the docs 'copy as' transcode surface)", () => {
  const ds = Dataset.parse('@prefix ex: <https://example.org/> . ex:a ex:p ex:o .', "turtle");
  const jsonld = ds.serialize("jsonld");
  const doc = JSON.parse(jsonld); // must be valid JSON
  // Exact: the document carries each term IRI in its own position, compared whole.
  assert.deepEqual(
    doc,
    {
      "@context": {},
      "@graph": [{ "@id": "https://example.org/a", "https://example.org/p": { "@id": "https://example.org/o" } }],
    },
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
  "PREFIX ex: <https://example.org/> CONSTRUCT { GRAPH ex:out { ?s ex:knows ?o } } WHERE { ?s ex:knows ?o }";
const PLAIN_CONSTRUCT =
  "PREFIX ex: <https://example.org/> CONSTRUCT { ?s ex:knows ?o } WHERE { ?s ex:knows ?o }";

test("the default query() format never returns an empty string for a named-graph CONSTRUCT", () => {
  const ds = Dataset.parse(TRIG, "trig");
  const out = ds.query(GRAPH_CONSTRUCT);
  assert.notEqual(out.trim(), "", "the documented default must never be a silent empty result");
  // TriG round-trips back into a dataset that still carries the graph and the statement;
  // every term is compared whole, never as a substring of the serialized text.
  const reparsed = Dataset.parse(out, "trig");
  assert.equal(reparsed.size, 1, `the constructed statement must survive: ${out}`);
  const [quad] = reparsed.quads();
  assert.equal(quad.graph.value, "https://example.org/out", `the graph the query named must survive: ${out}`);
  assert.equal(quad.subject.value, "https://example.org/a");
  assert.equal(quad.predicate.value, "https://example.org/knows");
  assert.equal(quad.object.value, "https://example.org/b");
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
          error.message.includes("carrying 1 named graph (<https://example.org/out>)"),
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
  assert.ok(nquads.includes("<https://example.org/out> ."), nquads);
});

test("an explicit single-graph format still serializes a default-graph CONSTRUCT", () => {
  const engine = new QueryEngine();
  const ds = Dataset.parse(TRIG, "trig");
  const ntriples = engine.queryRaw(ds, PLAIN_CONSTRUCT, { format: "ntriples" });
  assert.equal(
    ntriples.trim(),
    "<https://example.org/a> <https://example.org/knows> <https://example.org/b> .",
  );
});

// ── A DESCRIBE reaches the same egress, because a description carries graphs ────
//
// No DESCRIBE names a graph — there is no template to name one in — but the Symmetric
// CBD keeps every layer of a statement (the base quad, the reifier declaration and the
// annotation) in the graph that asserted it, so a description over graph-scoped data
// carries graph names too. `describe` and `construct` are ONE egress here, so the
// default must widen for a description exactly as it does for a quad template, and an
// explicit single-graph format must throw rather than hand JS an empty document.

const GRAPH_STAR_TRIG = `
@prefix ex: <https://example.org/> .
graph <https://example.org/g> { ex:s ex:p ex:o ~ex:r {| ex:note "n" |} . }
`;
const GRAPH_DESCRIBE = "DESCRIBE <https://example.org/s>";

test("the default query() format carries a named-graph DESCRIBE instead of emptying it", () => {
  const ds = Dataset.parse(GRAPH_STAR_TRIG, "trig");
  const out = ds.query(GRAPH_DESCRIBE);
  assert.notEqual(out.trim(), "", "a description must never come back as a silent empty result");
  // Every described statement stays in the graph the source asserted it in, compared whole.
  assert.deepEqual(
    Dataset.parse(out, "trig").quads().map((quad) => quad.graph.value),
    ["https://example.org/g", "https://example.org/g", "https://example.org/g"],
    `the graph the source asserted must survive: ${out}`,
  );
  const engine = new QueryEngine();
  const nquads = engine.queryRaw(ds, GRAPH_DESCRIBE, { format: "nquads" });
  // Exactly these rows, each compared whole as a line — no more, no fewer.
  assert.deepEqual(
    nquads.split("\n").filter((line) => line !== "").sort(),
    [
      "<https://example.org/s> <https://example.org/p> <https://example.org/o> <https://example.org/g> .",
      "<https://example.org/r> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> <<( <https://example.org/s> <https://example.org/p> <https://example.org/o> )>> <https://example.org/g> .",
      '<https://example.org/r> <https://example.org/note> "n" <https://example.org/g> .',
    ].sort(),
    `the description must carry exactly its rows: ${nquads}`,
  );
});

test("an explicit single-graph format throws for a named-graph DESCRIBE", () => {
  const engine = new QueryEngine();
  for (const format of ["turtle", "ntriples", "rdfxml"]) {
    const ds = Dataset.parse(GRAPH_STAR_TRIG, "trig");
    assert.throws(
      () => engine.queryRaw(ds, GRAPH_DESCRIBE, { format }),
      (error) => {
        assert.ok(
          error.message.includes("carrying 1 named graph (<https://example.org/g>)"),
          `the refusal names the graph: ${error.message}`,
        );
        assert.ok(
          error.message.includes(format),
          `the refusal names the offending format: ${error.message}`,
        );
        return true;
      },
      `${format} must refuse a graph-carrying description`,
    );
  }
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
  // Exactly the three default-graph rows: the named-graph row is gone, nothing else is.
  assert.equal(
    lossy.text,
    "<https://example.org/a> <https://example.org/knows> <https://example.org/b> .\n" +
      '<https://example.org/a> <https://example.org/name> "Ann" .\n' +
      '<https://example.org/b> <https://example.org/name> "Bob" .\n',
  );
  // The bytes are exactly what the plain entry point produces.
  assert.equal(lossy.text, ds.serialize("ntriples"));
  lossy.free();

  const lossless = ds.serializeWithLoss("nquads");
  assert.equal(lossless.statementRowsDropped, 0);
  assert.equal(lossless.directionalLiteralsDropped, 0);
  assert.equal(lossless.namedGraphRowsDropped, 0);
  // Exactly the same rows plus the named-graph row, carried in its graph.
  assert.equal(
    lossless.text,
    "<https://example.org/a> <https://example.org/knows> <https://example.org/b> .\n" +
      '<https://example.org/a> <https://example.org/name> "Ann" .\n' +
      '<https://example.org/b> <https://example.org/name> "Bob" .\n' +
      "<https://example.org/c> <https://example.org/knows> <https://example.org/a> <https://example.org/g> .\n",
  );
  lossless.free();
});

// The `SILENT` forms are the query author's own opt-out, and SPARQL 1.1 (§10 for
// SERVICE, §3.1.4 for LOAD) requires them to succeed with nothing fetched. The
// package docs claimed SERVICE / LOAD hard-fail unconditionally; they hard-fail only
// without `SILENT`, and this pins both halves beside each other so the prose and the
// behaviour cannot drift apart again.
test("SERVICE SILENT and LOAD SILENT succeed with nothing fetched", () => {
  const ds = Dataset.parse("@prefix ex: <https://example.org/> . ex:a ex:p ex:b .", "turtle");

  // Without SILENT: a hard failure, because no resolver is installed.
  assert.throws(() =>
    ds.query("SELECT * WHERE { ?s ?p ?o SERVICE <https://example.org/endpoint> { ?a ?b ?c } }"),
  );

  // With SILENT: the surrounding pattern's own solutions come back, joined against
  // the identity — so the local row survives and nothing remote is bound.
  const json = JSON.parse(
    ds.query("SELECT * WHERE { ?s ?p ?o SERVICE SILENT <https://example.org/endpoint> { ?a ?b ?c } }"),
  );
  assert.equal(json.results.bindings.length, 1);
  assert.equal("a" in json.results.bindings[0], false, "nothing remote may be bound");
  assert.equal(json.results.bindings[0].s.value, "https://example.org/a");

  const engine = new QueryEngine();
  const before = ds.canonicalize();
  assert.throws(() => engine.update(ds, "LOAD <https://example.org/doc>"));
  engine.update(ds, "LOAD SILENT <https://example.org/doc>");
  assert.equal(ds.canonicalize(), before, "LOAD SILENT must leave the dataset untouched");
});

// How deep a request may nest is bounded by the stacks it runs on, not by a fixed count.
// The parser is recursive descent, so before it measured its stack a FILTER nested about
// 950 parentheses deep ran the wasm shadow stack out of linear memory and trapped the
// instance. Every recursive level now measures the shadow stack it has left, and — since
// V8's own call stack, which wasm code cannot read, runs out first for some constructs —
// is also charged what it costs that stack against a host-stack budget
// (`WASM_HOST_STACK_BUDGET`, 640 KiB: 2 304 bytes a group, 1 408 a function call, 1 024 a
// bracket, 576 a path group, …). Past either, the request is a typed stack refusal and the
// instance answers the next one.

// Every shape at 128, 500 and 1 000 levels on the synchronous lane: 128 levels of every
// shape answer; 500 parentheses, negations and path groups answer, while 500 nested calls
// or groups do not fit; of 1 000, only path groups answer. Each answer is the one the
// nesting computes, and each refusal is typed.
test("nesting answers on the synchronous lane as deep as its stacks hold it, and is a typed refusal past that", async () => {
  const ds = Dataset.parse(NUMBERS, "nquads");
  const engine = new QueryEngine();
  const run = (query) => engine.select(ds, query);
  const answers = {
    128: new Set(NESTING_SHAPES.map(([what]) => what)),
    500: new Set(["nested parentheses", "nested -(", "nested property-path groups"]),
    1000: new Set(["nested property-path groups"]),
  };
  for (const [what, text, expected] of NESTING_SHAPES) {
    for (const depth of [128, 500, 1_000]) {
      const outcome = await attempt(run, text(depth), `${what} ${depth} deep`);
      if (answers[depth].has(what)) {
        assert.deepEqual(outcome.subjects, expected, `${what} ${depth} deep answers what it computes`);
      } else {
        assert.ok(outcome.refused, `${what} ${depth} deep is refused`);
      }
    }
  }
});

// The real limit of every shape on the synchronous lane, found by bisection: the deepest
// level that answers holds its computed value and one level more is the typed refusal —
// a refusal pair at the lane's real end. Past the WHERE group's own 2 304 bytes, the
// host-stack budget admits 637 brackets, 537 negations, 283 groups and 1 133 path groups;
// nested calls run out of shadow stack in their evaluation first.
test("on the synchronous lane the deepest answer and the first refusal are neighbours", async () => {
  const ds = Dataset.parse(NUMBERS, "nquads");
  const engine = new QueryEngine();
  const run = (query) => engine.select(ds, query);
  const limits = {};
  for (const shape of NESTING_SHAPES) limits[shape[0]] = await realLimit(run, shape);
  const calls = limits["nested ABS("];
  assert.ok(calls >= 128 && calls <= 463, `nested ABS( answers ${calls} deep`);
  delete limits["nested ABS("];
  assert.deepEqual(limits, {
    "nested parentheses": 637,
    "nested -(": 537,
    "nested groups": 283,
    "nested property-path groups": 1133,
  });
});

test("a FILTER nested 10 000 parentheses deep is a typed stack refusal, and the engine answers afterwards", () => {
  const ds = Dataset.parse(TRIG, "trig");
  const engine = new QueryEngine();
  const deep = `SELECT ?s WHERE { ?s ?p ?o FILTER(${"(".repeat(10_000)}?o${")".repeat(10_000)} = ?o) }`;
  // Twice: a trap would have left the instance unusable, so the second call would not
  // reach the parser to refuse it the same way.
  for (let attempt = 0; attempt < 2; attempt += 1) {
    assert.throws(() => engine.select(ds, deep), /SPARQL parse stack exhausted .*bracketted expression/);
  }
  // The synchronous lane names the remedy: the asynchronous twin with a larger stack
  // region. The refusal's own text is unchanged in front of it.
  assert.throws(() => engine.select(ds, deep), (error) => {
    assert.match(error.message, /^error native-sparql-query-parse: SPARQL parse stack exhausted/);
    assert.match(error.message, SYNC_STACK_HINT);
    return true;
  });
  // The instance is intact: an ordinary query answers exactly.
  const names = engine
    .select(ds, "PREFIX ex: <https://example.org/> SELECT ?name WHERE { ?p ex:name ?name } ORDER BY ?name")
    .rows.toArray()
    .map((row) => row.name.value);
  assert.deepEqual(names, ["Ann", "Bob"]);
});

// Nesting the parser admits can still be more than the stack can EVALUATE: one written
// level of `LATERAL` costs the evaluator about 9 KB of shadow stack, so 126 of them ran
// the synchronous lane's 1 MiB shadow stack below its floor — the call trapped ("memory
// access out of bounds") and left the instance's memory in an unknown state. The
// evaluator now measures the stack it has left at every recursive entry and refuses,
// typed, before it runs out. (63 nested `FILTER NOT EXISTS` trapped the same way; a
// nested `EXISTS` body is now substituted when it is evaluated rather than copied into
// every level around it, so 63 levels answer — see the test after this one.)
const STACK_REFUSAL = /native-sparql-evaluation-stack-exhausted.*evaluation stack exhausted/;
// What the synchronous lane appends to a stack refusal: the remedy only it has to name.
const SYNC_STACK_HINT = /asynchronous twin of this call \(selectAsync, queryAsync, updateAsync, …\) and a larger stackBytes region$/;
const NEST_DATA = [1, 2, 3, 4]
  .map((n) => `<https://example.org/s${n}> <https://example.org/p> <https://example.org/o${n}> .`)
  .concat(["<https://example.org/s1> <https://example.org/q> <https://example.org/o1> ."])
  .join("\n");
/** `open` written `depth` times around `?s <q> ?z`, closed as often. */
const nestedAround = (open, depth) =>
  `SELECT ?s WHERE { ${open.repeat(depth)}?s <https://example.org/q> ?z${" }".repeat(depth)} }`;
const nestedLateral = (depth) => nestedAround("?s <https://example.org/p> ?o LATERAL { ", depth);
const subjectsOf = (result) =>
  result.rows
    .toArray()
    .map((row) => row.s.value.replace("https://example.org/", ""))
    .sort();

for (const [what, deep, shallow, expected] of [
  // Only `s1` has a `<q>`, at every level.
  ["126 nested LATERAL", nestedLateral(126), nestedLateral(40), ["s1"]],
]) {
  test(`${what} is a typed refusal on the synchronous lane, and the engine answers afterwards`, () => {
    const ds = Dataset.parse(NEST_DATA, "nquads");
    const engine = new QueryEngine();
    const before = ds.canonicalize();
    // Twice: a trap would have left the instance unusable, so the second call would not
    // reach the evaluator to refuse it the same way.
    for (let attempt = 0; attempt < 2; attempt += 1) {
      assert.throws(() => engine.select(ds, deep), STACK_REFUSAL);
    }
    // The evaluator's refusal keeps its code and names the synchronous lane's remedy.
    assert.throws(() => engine.select(ds, deep), (error) => {
      assert.match(error.message, /^error native-sparql-evaluation-stack-exhausted: /);
      assert.match(error.message, SYNC_STACK_HINT);
      return true;
    });
    // No memory was overwritten: the dataset's canonical form is byte-identical, and
    // queries over it and over a freshly parsed one answer exactly.
    assert.equal(ds.canonicalize(), before);
    assert.deepEqual(subjectsOf(engine.select(ds, "SELECT ?s WHERE { ?s <https://example.org/q> ?o }")), ["s1"]);
    const names = engine
      .select(Dataset.parse(TRIG, "trig"), "PREFIX ex: <https://example.org/> SELECT ?name WHERE { ?p ex:name ?name } ORDER BY ?name")
      .rows.toArray()
      .map((row) => row.name.value);
    assert.deepEqual(names, ["Ann", "Bob"]);
    // The valid neighbour: the same form, nested as deep as the stack evaluates, answers
    // with the rows the deep one has on a larger stack (asynchronous tests).
    assert.deepEqual(subjectsOf(engine.select(ds, shallow)), expected);
  });
}

// Nested `FILTER NOT EXISTS` on the synchronous lane: it answers as deep as the stack
// evaluates it, with the rows its semantics give, and one level more is the evaluator's
// typed stack refusal, after which the instance still answers.
//
// Level `k` steps along `<next>` from the node level `k - 1` reached, requires the step
// not to land back on the outermost node `?x0`, and negates the level below. The graph
// has a 3-cycle (`n0 n1 n2`), a tail into it (`n4 → n3 → n0`), and a line that ends
// (`n9 → n8 → n5 → n6 → n7`). A walk from the tail never ends, so whether `n3`/`n4`
// answer is the parity of the whole depth — every level counts; a walk from the line ends
// where the line does; a walk from the cycle is cut by the `?x0` comparison three levels
// down, an expression position reading the outermost row's binding.
const WALK = [
  ["n0", "n1"], ["n1", "n2"], ["n2", "n0"], ["n3", "n0"], ["n4", "n3"],
  ["n5", "n6"], ["n6", "n7"], ["n8", "n5"], ["n9", "n8"],
];
const walkData = () =>
  Dataset.parse(WALK.map(([from, to]) => `<https://example.org/${from}> <https://example.org/next> <https://example.org/${to}> .`).join("\n"), "nquads");
const nestedWalk = (depth) => {
  let body = "";
  for (let k = depth; k >= 1; k -= 1) {
    body = `FILTER NOT EXISTS { ?x${k} <https://example.org/next> ?x${k + 1} FILTER(?x${k + 1} != ?x0) ${body}}`;
  }
  return `SELECT ?x0 WHERE { ?x0 <https://example.org/next> ?x1 ${body}}`;
};
/** The rows of `nestedWalk(depth)`, read over `WALK` directly. */
const walkByHand = (depth) => {
  const next = new Map(WALK);
  const holds = (k, x0, x) => {
    if (k > depth) return true;
    const y = next.get(x);
    return !(y !== undefined && y !== x0 && holds(k + 1, x0, y));
  };
  return WALK.filter(([x0, x1]) => holds(1, x0, x1)).map(([x0]) => x0).sort();
};
const walkAnswer = (engine, ds, depth) => {
  try {
    return {
      rows: engine
        .select(ds, nestedWalk(depth))
        .rows.toArray()
        .map((row) => row.x0.value.replace("https://example.org/", ""))
        .sort(),
    };
  } catch (error) {
    // Far past the limit the parser refuses before the evaluator is reached; either is
    // typed, and the pair below asserts the evaluator's own refusal at the limit.
    assert.match(
      error.message,
      /native-sparql-evaluation-stack-exhausted|SPARQL parse stack exhausted/,
      `${depth} nested FILTER NOT EXISTS: a typed stack refusal, not a trap`,
    );
    return { refused: error.message };
  }
};

test("nested FILTER NOT EXISTS answers on the synchronous lane as deep as its stack evaluates it, and one level more is a typed refusal", () => {
  const ds = walkData();
  const engine = new QueryEngine();
  // The oracle itself, pinned: the rows differ between neighbouring depths.
  assert.deepEqual(walkByHand(1), ["n6"]);
  assert.deepEqual(walkByHand(2), ["n3", "n4", "n6", "n8", "n9"]);
  assert.deepEqual(walkByHand(62), ["n3", "n4", "n6", "n8"]);
  assert.deepEqual(walkByHand(63), ["n6", "n8"]);
  for (const depth of [1, 2, 3, 4, 5, 6, 62, 63]) {
    assert.deepEqual(walkAnswer(engine, ds, depth).rows, walkByHand(depth), `${depth} nested FILTER NOT EXISTS`);
  }
  // The real limit, by bisection between 63 levels (which answer) and 20 000 (refused,
  // by the parser).
  let [deepest, refused] = [63, 20_000];
  assert.ok(walkAnswer(engine, ds, refused).refused, "20 000 levels are refused");
  while (refused - deepest > 1) {
    const mid = (deepest + refused) >> 1;
    if (walkAnswer(engine, ds, mid).rows) deepest = mid;
    else refused = mid;
  }
  // The pair at the lane's real end: the deepest level answers what it computes, and one
  // level more is the evaluator's own refusal — twice, since a trap would have left the
  // instance unable to refuse the same way again.
  assert.deepEqual(walkAnswer(engine, ds, deepest).rows, walkByHand(deepest), `${deepest} levels answer`);
  for (let attempt = 0; attempt < 2; attempt += 1) {
    assert.match(walkAnswer(engine, ds, refused).refused ?? "", /native-sparql-evaluation-stack-exhausted/);
  }
  assert.equal(refused, deepest + 1);
  // Measured on this build: 85 levels answer and 86 are refused. The bound is the
  // evaluator's shadow-stack frames, which the compiler sizes, so the test asserts the
  // pair wherever it falls and only that it lies past the old 63-level trap.
  assert.ok(deepest > 63, `${deepest} levels answer`);
  // Not poisoned: the same engine answers an ordinary query exactly.
  assert.deepEqual(
    engine
      .select(ds, "SELECT ?s WHERE { ?s <https://example.org/next> <https://example.org/n0> }")
      .rows.toArray()
      .map((row) => row.s.value.replace("https://example.org/", ""))
      .sort(),
    ["n2", "n3"],
  );
});

// Triple terms nest as deep as the stacks hold them, like every other construct. A
// pattern nested deeper than any triple term a dataset holds (16 levels) matches nothing
// — the neighbour of the same shape at the stored depth matches the stored statement —
// and a triple term written into the dataset past 16 levels is refused with the
// dataset's own limit, while 16 are inserted and matched.
const TT = "https://example.org/";
/** `<<( <s> <p> … core … )>>`, `levels` triple terms deep. */
const tripleChain = (levels, core) =>
  `${`<<( <${TT}s> <${TT}p> `.repeat(levels)}${core}${" )>>".repeat(levels)}`;

test("a pattern nested past the dataset's triple terms answers nothing, and its stored-depth neighbour matches", () => {
  const ds = Dataset.parse("", "nquads");
  const engine = new QueryEngine();
  engine.update(ds, `INSERT DATA { <${TT}a> <${TT}q> ${tripleChain(16, `<${TT}o>`)} }`);
  const select = (levels) =>
    engine
      .select(ds, `SELECT ?a ?o WHERE { ?a <${TT}q> ${tripleChain(levels, "?o")} }`)
      .rows.toArray()
      .map((row) => [row.a.value, row.o.value]);
  assert.deepEqual(select(16), [[`${TT}a`, `${TT}o`]], "the stored depth matches");
  assert.deepEqual(select(200), [], "200 levels match nothing");
  assert.equal(engine.ask(ds, `ASK { ?a <${TT}q> ${tripleChain(200, "?o")} }`), false);
  assert.equal(engine.ask(ds, `ASK { ?a <${TT}q> ${tripleChain(16, "?o")} }`), true);
});

test("inserting a triple term past the dataset's limit is the dataset's typed refusal, and 16 levels insert", () => {
  const ds = Dataset.parse("", "nquads");
  const engine = new QueryEngine();
  const insert = (levels) => `INSERT DATA { <${TT}b> <${TT}q> ${tripleChain(levels, `<${TT}o>`)} }`;
  assert.throws(() => engine.update(ds, insert(17)), (error) => {
    assert.match(error.message, /^error rdf-ir-triple-nesting-limit: /);
    return true;
  });
  assert.equal(ds.canonicalize(), "", "the refusal wrote nothing");
  engine.update(ds, insert(16));
  const rows = engine
    .select(ds, `SELECT ?o WHERE { <${TT}b> <${TT}q> ${tripleChain(16, "?o")} }`)
    .rows.toArray()
    .map((row) => row.o.value);
  assert.deepEqual(rows, [`${TT}o`], "the 16-deep insert is matched");
});
