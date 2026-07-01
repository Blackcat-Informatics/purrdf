// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Anti-rot EXECUTION gate for the vendored purrdf wasm engine.
//
// The docs SPARQL playground ships a PINNED copy of the wasm package under
// crates/docs/assets/purrdf/. This test loads THAT vendored copy (not the freshly
// built js/pkg/) and runs a real SPARQL query, proving the shipped engine actually
// evaluates — catching behaviour rot that the structural Rust gate cannot. Runs on
// `make wasm-pkg-test`.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";

const VENDORED = new URL("../../../docs/assets/purrdf/", import.meta.url);

// The vendored bindings are wasm-bindgen `--target web`: `default` is the async init
// that accepts the wasm bytes; the classes are named exports.
const { default: init, Dataset } = await import(
  new URL("purrdf_wasm.js", VENDORED).href
);
await init({
  module_or_path: await readFile(
    fileURLToPath(new URL("purrdf_wasm_bg.wasm", VENDORED)),
  ),
});

test("the VENDORED engine evaluates a SPARQL SELECT", () => {
  const ds = Dataset.parse(
    '@prefix ex: <https://e/> . ex:a ex:name "Ann" . ex:b ex:name "Bob" .',
    "turtle",
  );
  const json = JSON.parse(
    ds.query("PREFIX ex: <https://e/> SELECT ?name WHERE { ?p ex:name ?name } ORDER BY ?name"),
  );
  const names = json.results.bindings.map((b) => b.name.value);
  assert.deepEqual(names, ["Ann", "Bob"]);
});

test("the VENDORED engine evaluates a SPARQL DESCRIBE", () => {
  // The docs per-term/per-slice export links are DESCRIBE-based, so a DESCRIBE
  // regression would break the exported playground flow while SELECT still worked.
  const ds = Dataset.parse(
    '@prefix ex: <https://e/> . ex:a ex:name "Ann" . ex:a ex:knows ex:b .',
    "turtle",
  );
  const ttl = ds.query("PREFIX ex: <https://e/> DESCRIBE ex:a");
  const back = Dataset.parse(ttl, "turtle");
  assert.ok(back.size > 0, "DESCRIBE returns a non-empty graph");
});

test("the VENDORED engine hard-fails a malformed query", () => {
  const ds = Dataset.parse("<https://e/s> <https://e/p> <https://e/o> .", "ntriples");
  assert.throws(() => ds.query("SELECT ?x WHERE { not sparql"));
});

test("the VENDORED engine transcodes to every 'copy as' format", () => {
  const ds = Dataset.parse('@prefix ex: <https://e/> . ex:a ex:p ex:o .', "turtle");
  // The docs export buttons transcode a term's Turtle client-side to each of these.
  for (const fmt of ["turtle", "ntriples", "nquads", "trig", "rdfxml", "jsonld"]) {
    const out = ds.serialize(fmt);
    assert.ok(out.length > 0, `${fmt} produced output`);
  }
  // JSON-LD is valid JSON carrying the IRIs.
  assert.ok(JSON.parse(ds.serialize("jsonld")));
});
