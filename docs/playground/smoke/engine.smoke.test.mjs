// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Engine smoke gate for the PurRDF console. This asserts the SAME engine calls
// each pane of the console makes actually work against the real wasm package.
// It is the CI gate for the playground: `node --test docs/playground/smoke/*.test.mjs`
// from the repo root. The browser DOM wiring is verified separately (headless);
// this file proves the engine integration end-to-end.

import { test } from "node:test";
import assert from "node:assert/strict";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

import {
  ready,
  Dataset,
  DataFactory,
  shaclValidateToSarif,
  shaclEntail,
  version,
} from "../../../crates/rdf-wasm/js/index.mjs";

// The exact preloaded document the Input pane ships with: an RDF-1.2 graph in the
// example.org namespace carrying a quoted triple in object position plus base-
// direction literals (ltr + rtl).
const PRELOADED = `@prefix ex: <http://example.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:alice ex:says <<( ex:bob ex:likes ex:carol )>> .
ex:alice ex:statedOn "2026-07-06"^^xsd:date .
ex:carol ex:name "Carol" .
ex:greetingEn ex:text "Hello, world"@en--ltr .
ex:greetingAr ex:text "مرحبا بالعالم"@ar--rtl .
`;

const SHAPES = `@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .

ex:PersonShape
  a sh:NodeShape ;
  sh:targetClass ex:Person ;
  sh:property [ sh:path ex:name ; sh:minCount 1 ] ;
  sh:rule [
    a sh:TripleRule ;
    sh:subject sh:this ;
    sh:predicate ex:adult ;
    sh:object ex:yes ;
  ] .
`;

// A tiny data graph with an ex:Person that lacks ex:name (guarantees a violation)
// and is a valid sh:rule target (guarantees an entailment).
const PERSON_DATA = `@prefix ex: <http://example.org/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .

ex:dave a ex:Person .
`;

const SERIALIZE_FORMATS = [
  "turtle",
  "ntriples",
  "nquads",
  "trig",
  "rdfxml",
  "jsonld",
  "yamlld",
];

await ready();

const workerMessages = [];
let workerMessageHandler = null;
globalThis.self = {
  location: { origin: "https://playground.example" },
  addEventListener(type, handler) {
    if (type === "message") workerMessageHandler = handler;
  },
  postMessage(message) {
    workerMessages.push(message);
  },
};
const playgroundOut = process.env.PLAYGROUND_OUT;
assert.ok(playgroundOut, "PLAYGROUND_OUT must point at the assembled console");
await import(pathToFileURL(join(playgroundOut, "engine.worker.mjs")).href);
assert.equal(typeof workerMessageHandler, "function");

let workerCallId = 0;
async function workerCall(op, args = {}) {
  const id = `smoke-${workerCallId++}`;
  await workerMessageHandler({ origin: "", data: { id, op, args } });
  const message = workerMessages.find((candidate) => candidate.id === id);
  assert.ok(message, `worker did not answer ${op}`);
  assert.equal(message.ok, true, message.error);
  return message.result;
}

test("version() returns a SemVer string", () => {
  const v = version();
  assert.match(v, /^\d+\.\d+\.\d+/, `expected SemVer, got ${v}`);
});

test("parse the preloaded doc (Input pane)", () => {
  const ds = Dataset.parse(PRELOADED, "turtle");
  assert.equal(ds.size, 5);
  const hasQuoted = ds.quads().some((q) => q.object.termType === "Quad");
  assert.ok(hasQuoted, "expected an object-position quoted triple");
  const hasRtl = ds
    .quads()
    .some((q) => q.object.termType === "Literal" && q.object.direction === "rtl");
  assert.ok(hasRtl, "expected a directional (rtl) literal");
});

// A plain graph (no quoted-triple object) — every serializer, incl. JSON-LD,
// can encode it. This is the graph the "all 7 formats" claim rests on.
const PLAIN = `@prefix ex: <http://example.org/> .
ex:alice ex:knows ex:bob .
ex:bob ex:name "Bob" .
`;

test("serialize a plain graph to all 7 formats (Round-trip pane)", () => {
  const ds = Dataset.parse(PLAIN, "turtle");
  for (const f of SERIALIZE_FORMATS) {
    const text = ds.serialize(f);
    assert.equal(typeof text, "string");
    assert.ok(text.length > 0, `empty serialization for ${f}`);
  }
  // Both linked-data carrier syntaxes are first-class bidirectional codecs.
  for (const format of ["jsonld", "yamlld"]) {
    const encoded = ds.serialize(format);
    const back = Dataset.parse(encoded, format);
    assert.ok(ds.isomorphic(back), `${format} round-trip must be isomorphic`);
  }
});

test("configured JSON-LD and YAML-LD follow worker dispatch", async () => {
  await workerCall("parse", { text: PLAIN, format: "turtle" });
  const result = await workerCall("serializeAll", {
    jsonldOptions: {
      version: 1,
      mode: "context",
      prefixes: { ex: "http://example.org/" },
    },
  });
  assert.equal(JSON.parse(result.formats.jsonld.text)["@graph"][0]["@id"], "ex:alice");
  assert.match(result.formats.yamlld.text, /ex:alice/);
  assert.equal(result.formats.jsonld.roundtrips, true);
  assert.equal(result.formats.yamlld.roundtrips, true);
});

test("preloaded (quoted-triple) doc: all 7 formats serialize; JSON-LD round-trips", () => {
  const ds = Dataset.parse(PRELOADED, "turtle");
  for (const f of SERIALIZE_FORMATS) {
    const text = ds.serialize(f);
    assert.ok(text.length > 0, `empty serialization for ${f}`);
  }
  // JSON-LD-star losslessly encodes the object-position quoted triple via its
  // distinguishable `@triple` form, so the round-trip preserves it (surfaced,
  // never silently dropped).
  const jsonld = ds.serialize("jsonld");
  const back = Dataset.parse(jsonld, "jsonld");
  assert.ok(
    back.quads().some((q) => q.object.termType === "Quad"),
    "JSON-LD must preserve the quoted triple in object position",
  );
  assert.ok(ds.isomorphic(back), "JSON-LD round-trip must be isomorphic");
});

test("N-Quads round-trips the object-position quoted triple", () => {
  const ds = Dataset.parse(PRELOADED, "turtle");
  const nq = ds.serialize("nquads");
  const back = Dataset.parse(nq, "nquads");
  assert.ok(
    back.quads().some((q) => q.object.termType === "Quad"),
    "N-Quads must preserve the quoted triple in object position",
  );
  assert.ok(ds.isomorphic(back), "N-Quads round-trip must be isomorphic");
});

test("SELECT query returns SRJ with bindings (SPARQL pane)", () => {
  const ds = Dataset.parse(PRELOADED, "turtle");
  const out = ds.query("SELECT ?s ?p ?o WHERE { ?s ?p ?o }");
  const srj = JSON.parse(out);
  assert.ok(Object.hasOwn(srj, "head"), "SELECT result must be SRJ (has .head)");
  assert.deepEqual(srj.head.vars, ["s", "p", "o"]);
  assert.ok(srj.results.bindings.length > 0, "expected bindings");
});

test("ASK query returns an SRJ boolean", () => {
  const ds = Dataset.parse(PRELOADED, "turtle");
  const srj = JSON.parse(
    ds.query("ASK { <http://example.org/carol> <http://example.org/name> ?n }"),
  );
  assert.ok(Object.hasOwn(srj, "boolean"));
  assert.equal(srj.boolean, true);
});

test("CONSTRUCT returns Turtle (not SRJ)", () => {
  const ds = Dataset.parse(PRELOADED, "turtle");
  const out = ds.query("CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o } LIMIT 1");
  let parsedAsSrj = false;
  try {
    const j = JSON.parse(out);
    parsedAsSrj = Object.hasOwn(j, "head") || Object.hasOwn(j, "boolean");
  } catch {
    parsedAsSrj = false;
  }
  assert.equal(parsedAsSrj, false, "CONSTRUCT output must not parse as SRJ");
});

test("SHACL validate → SARIF 2.1.0 with a violation (SHACL pane)", () => {
  // The pane serializes the CURRENT dataset to N-Triples first, then validates.
  const ds = Dataset.parse(PERSON_DATA, "turtle");
  const dataNt = ds.serialize("ntriples");
  const sarif = JSON.parse(shaclValidateToSarif(SHAPES, dataNt));
  assert.equal(sarif.version, "2.1.0");
  const results = sarif.runs[0].results;
  assert.ok(results.length >= 1, "expected at least one SHACL violation");
  const r = results[0];
  assert.ok(r.ruleId, "result must carry a ruleId");
  assert.ok(r.level, "result must carry a level");
  assert.ok(r.message?.text, "result must carry a message");
});

test("shaclEntail materializes the sh:rule inference", () => {
  const ds = Dataset.parse(PERSON_DATA, "turtle");
  const dataNt = ds.serialize("ntriples");
  const entailed = shaclEntail(SHAPES, dataNt);
  // Assert the exact triple the sh:rule produces (ex:dave ex:adult ex:yes) by a
  // structural match on the parsed N-Triples — not a bare-URL substring check.
  const out = Dataset.parse(entailed, "ntriples");
  const f = new DataFactory();
  const matched = out.match(
    f.namedNode("http://example.org/dave"),
    f.namedNode("http://example.org/adult"),
    f.namedNode("http://example.org/yes"),
  );
  assert.equal(
    matched.size,
    1,
    "expected the ex:dave ex:adult ex:yes entailment in the materialized graph",
  );
});

test("canonicalize is non-empty and stable (Round-trip / identity)", () => {
  const ds = Dataset.parse(PRELOADED, "turtle");
  const c1 = ds.canonicalize();
  const c2 = ds.canonicalize();
  assert.ok(c1.length > 0, "canonical form must be non-empty");
  assert.equal(c1, c2, "canonical form must be stable");
});

test("isomorphic: true for a relabeled pair, false for a different graph", () => {
  const a = Dataset.parse(
    `@prefix ex: <http://example.org/> .
     _:a ex:knows _:b . _:b ex:name "Bob" . ex:root ex:member _:a .`,
    "turtle",
  );
  const b = Dataset.parse(
    `@prefix ex: <http://example.org/> .
     ex:root ex:member _:x . _:y ex:name "Bob" . _:x ex:knows _:y .`,
    "turtle",
  );
  const c = Dataset.parse(
    `@prefix ex: <http://example.org/> . ex:root ex:member ex:different .`,
    "turtle",
  );
  assert.equal(a.isomorphic(b), true, "relabeled pair must be isomorphic");
  assert.equal(a.isomorphic(c), false, "different graph must not be isomorphic");
});

test("match() filters the dataset (Quad table pane)", () => {
  const ds = Dataset.parse(PRELOADED, "turtle");
  const f = new DataFactory();
  const filtered = ds.match(f.namedNode("http://example.org/carol"));
  assert.equal(filtered.size, 1);
  assert.equal(filtered.quads()[0].subject.value, "http://example.org/carol");
});

test("DataFactory builds a directional literal and quoted triple", () => {
  const f = new DataFactory();
  const dl = f.directionalLiteral("مرحبا", "ar", "rtl");
  assert.equal(dl.direction, "rtl");
  assert.equal(dl.language, "ar");
  const qt = f.quotedTriple(
    f.namedNode("http://example.org/b"),
    f.namedNode("http://example.org/likes"),
    f.namedNode("http://example.org/c"),
  );
  assert.equal(qt.termType, "Quad");
  const ds = new Dataset();
  ds.add(
    f.quad(
      f.namedNode("http://example.org/a"),
      f.namedNode("http://example.org/says"),
      qt,
    ),
  );
  assert.equal(ds.size, 1);
  assert.equal(ds.quads()[0].object.termType, "Quad");
});

test("malformed input throws (never a silent empty result)", () => {
  assert.throws(() => Dataset.parse("@prefix ex: <http://example.org/", "turtle"));
});

test("SERVICE query throws (never-silent federation hard-fail)", () => {
  const ds = Dataset.parse(PRELOADED, "turtle");
  assert.throws(() =>
    ds.query(
      "SELECT ?x WHERE { SERVICE <http://example.org/sparql> { ?x ?p ?o } }",
    ),
  );
});
