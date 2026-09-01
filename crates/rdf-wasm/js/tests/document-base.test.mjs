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
