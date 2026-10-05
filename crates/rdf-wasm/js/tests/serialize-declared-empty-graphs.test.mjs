// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: drives the published wasm npm package's serializeWithLoss through Node module loading.

import { test } from "node:test";
import assert from "node:assert/strict";

import { ready, Dataset } from "../index.mjs";

await ready();

const TRIG = `
@prefix ex: <https://example.org/> .
ex:a ex:knows ex:b .
ex:a ex:name "Ann" .
ex:b ex:name "Bob" .
graph <https://example.org/g> { ex:c ex:knows ex:a . }
`;

// A graph the document declares with no row (`<g> { }`) owns nothing a row count can
// see, so a target that cannot write an empty graph would drop it silently.
// `emptyNamedGraphsDropped` is that count; the neighbouring dataset reports 0.
const EMPTY_GRAPH = "https://example.org/empty";

// The named graphs a serialized document declares, read back by parsing it and
// enumerating `GRAPH ?g` — compared by exact IRI, never by substring.
function graphNames(text, format) {
  const parsed = Dataset.parse(text, format);
  const json = JSON.parse(parsed.query("SELECT ?g WHERE { GRAPH ?g { } }"));
  parsed.free();
  return json.results.bindings.map((b) => b.g.value);
}

test("serializeWithLoss counts the declared empty graphs a target cannot write", () => {
  const declared = Dataset.parse(
    "<https://example.org/s> <https://example.org/p> <https://example.org/o> .\n" +
      "<https://example.org/g1> { <https://example.org/s> <https://example.org/p> <https://example.org/o2> . }\n" +
      "<https://example.org/empty> { }\n" +
      "_:bg { }\n",
    "trig",
  );
  for (const format of ["nquads", "hextuples", "turtle", "ntriples"]) {
    const loss = declared.serializeWithLoss(format);
    assert.equal(loss.emptyNamedGraphsDropped, 2, format);
    assert.equal(loss.namedGraphRowsDropped, format === "nquads" || format === "hextuples" ? 0 : 1, format);
    assert.ok(!graphNames(loss.text, format).some((g) => g === EMPTY_GRAPH), format);
    loss.free();
  }
  for (const format of ["trig", "trix", "jsonld", "yamlld"]) {
    const loss = declared.serializeWithLoss(format);
    assert.equal(loss.emptyNamedGraphsDropped, 0, format);
    assert.ok(graphNames(loss.text, format).some((g) => g === EMPTY_GRAPH), format);
    loss.free();
  }

  const plain = Dataset.parse(TRIG, "trig");
  for (const format of ["nquads", "hextuples", "turtle", "trig"]) {
    const loss = plain.serializeWithLoss(format);
    assert.equal(loss.emptyNamedGraphsDropped, 0, format);
    loss.free();
  }
});
