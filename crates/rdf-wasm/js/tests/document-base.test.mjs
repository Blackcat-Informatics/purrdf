// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// The wasm surface's DOCUMENT BASE contract, exercised on a real wasm host.
//
// The Rust unit tests in `crates/rdf-wasm/src/dataset.rs` can only cover success
// paths — `JsError` panics on a non-wasm target — so the ERROR behaviour of the base
// is pinned here: a relative reference with no base in scope throws, and a base that
// is not an absolute IRI throws on both legs. No base is ever fabricated.

import { test } from "node:test";
import assert from "node:assert/strict";

import { CompiledJsonLdContext, Dataset, ready } from "../index.mjs";

await ready();

const BASE = "https://example.org/base/";
// A Turtle document whose subject is a relative reference and which declares no
// `@base` of its own, so the caller's base is the only one that can be in scope.
const RELATIVE_TURTLE = "<rel> <https://example.org/p> <https://example.org/o> .\n";
// The shared `purrdf-iri` diagnostic code, identical on every surface.
const NO_BASE_CODE = "iri-relative-no-base";

const EXPANDED = JSON.stringify({ version: 1, mode: "expanded" });
// `CompiledJsonLdContext` compiles only `mode: "context"` options.
const CONTEXT_OPTIONS = JSON.stringify({
  version: 1,
  mode: "context",
  prefixes: { ex: "https://example.org/" },
});

test("parse resolves a relative IRI against the supplied base", () => {
  const dataset = Dataset.parse(RELATIVE_TURTLE, "turtle", BASE);
  assert.equal(dataset.size, 1);
  assert.match(dataset.serialize("ntriples"), new RegExp(`<${BASE}rel>`));
});

test("parse without a base refuses a relative IRI with the shared code", () => {
  assert.throws(
    () => Dataset.parse(RELATIVE_TURTLE, "turtle"),
    (error) => error.message.includes(NO_BASE_CODE),
    "a relative reference with no base in scope must throw",
  );
});

test("parse rejects a base that is not an absolute IRI", () => {
  assert.throws(() => Dataset.parse(RELATIVE_TURTLE, "turtle", "not-absolute/"));
});

test("an in-document base wins over the supplied one", () => {
  const doc =
    "@base <https://example.org/inner/> .\n" +
    "<rel> <https://example.org/p> <https://example.org/o> .\n";
  const dataset = Dataset.parse(doc, "turtle", BASE);
  assert.match(dataset.serialize("ntriples"), /<https:\/\/example\.org\/inner\/rel>/);
});

test("serializeConfigured carries the document base into JSON-LD", () => {
  const dataset = Dataset.parse(RELATIVE_TURTLE, "turtle", BASE);
  const text = dataset.serializeConfigured("jsonld", EXPANDED, BASE);
  assert.ok(text.includes(BASE), `the JSON-LD document must carry the base: ${text}`);
});

test("serializeWithContext carries the same document base", () => {
  const dataset = Dataset.parse(RELATIVE_TURTLE, "turtle", BASE);
  const context = new CompiledJsonLdContext(CONTEXT_OPTIONS);
  const withContext = dataset.serializeWithContext("jsonld", context, undefined, BASE);
  // Reusing a compiled context is the same serialization as decoding the options
  // inline, base included.
  assert.equal(
    withContext,
    dataset.serializeConfigured("jsonld", CONTEXT_OPTIONS, BASE),
  );
  assert.ok(withContext.includes(BASE), `the base must survive: ${withContext}`);
});

test("serializeConfigured rejects a base that is not an absolute IRI", () => {
  const dataset = Dataset.parse(RELATIVE_TURTLE, "turtle", BASE);
  assert.throws(() => dataset.serializeConfigured("jsonld", EXPANDED, "not-absolute/"));
});

test("omitting the egress base leaves the document absolute", () => {
  const dataset = Dataset.parse(RELATIVE_TURTLE, "turtle", BASE);
  const text = dataset.serializeConfigured("jsonld", EXPANDED);
  assert.ok(
    text.includes(`${BASE}rel`),
    `without a base the JSON-LD must stay absolute: ${text}`,
  );
});

// ── serialize(format, base?): the generic egress leg ────────────────────────────
//
// Each case pairs the based call with a NO-BASE CONTROL on the same dataset, so the
// observed base declaration is attributable to the argument.

const ABSOLUTE_NT =
  `<${BASE}s> <${BASE}p> <${BASE}o> .\n`;

test("serialize emits the base declaration, with a no-base control", () => {
  const dataset = Dataset.parse(ABSOLUTE_NT, "ntriples");
  const withBase = dataset.serialize("turtle", BASE);
  const control = dataset.serialize("turtle");

  assert.ok(withBase.includes(`@base <${BASE}> .`), `expected @base: ${withBase}`);
  assert.ok(!control.includes("@base"), "the control must not already carry a base");
  // The base is applied, not merely declared.
  assert.ok(withBase.includes("<s>"), `expected a relative subject: ${withBase}`);
  assert.ok(control.includes(`<${BASE}s>`));
});

test("serialize to a base-incapable format stays absolute rather than throwing", () => {
  const dataset = Dataset.parse(ABSOLUTE_NT, "ntriples");
  const text = dataset.serialize("nquads", BASE);
  assert.ok(text.includes(`<${BASE}s>`));
  assert.ok(!text.includes("@base"));
});

test("serialize rejects a base that is not an absolute IRI", () => {
  const dataset = Dataset.parse(ABSOLUTE_NT, "ntriples");
  assert.throws(() => dataset.serialize("turtle", "not-absolute/"));
});

test("serialize without a base is byte-identical to omitting the argument", () => {
  const dataset = Dataset.parse(ABSOLUTE_NT, "ntriples");
  assert.equal(dataset.serialize("turtle"), dataset.serialize("turtle", undefined));
  assert.equal(dataset.serialize("turtle"), dataset.serialize("turtle", null));
});

test("serialize keeps the RDF 1.2 statement layer under a base", () => {
  // The fidelity answer: gaining a base must not cost reifier/annotation rows.
  const star =
    `<${BASE}r> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> ` +
    `<<( <${BASE}s> <${BASE}p> <${BASE}o> )>> .\n` +
    `<${BASE}r> <${BASE}confidence> "0.9" .\n`;
  const dataset = Dataset.parse(star, "ntriples");

  const turtle = dataset.serialize("turtle", BASE);
  assert.ok(turtle.includes(`@base <${BASE}> .`));
  assert.ok(turtle.includes("reifies"), `reifier binding must survive: ${turtle}`);
  assert.ok(turtle.includes("confidence"), `annotation must survive: ${turtle}`);

  // RDF/XML renders it as rdf:parseType="Triple" and still takes the base.
  const xml = dataset.serialize("rdfxml", BASE);
  assert.ok(xml.includes(`xml:base="${BASE}"`), `expected xml:base: ${xml}`);
  assert.ok(xml.includes("Triple"), `reifier binding must survive: ${xml}`);
});

test("a format with no triple-term surface fails closed rather than dropping rows", () => {
  // TriX and HexTuples cannot represent a triple term at all. Emitting is the
  // fidelity answer, so they throw instead of silently thinning the document.
  const star =
    `<${BASE}r> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> ` +
    `<<( <${BASE}s> <${BASE}p> <${BASE}o> )>> .\n`;
  const dataset = Dataset.parse(star, "ntriples");
  for (const format of ["trix", "hextuples"]) {
    assert.throws(
      () => dataset.serialize(format, BASE),
      `${format} must refuse rather than drop the statement layer`,
    );
  }
});
