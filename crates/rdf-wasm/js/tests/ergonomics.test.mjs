// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

import { test } from "node:test";
import assert from "node:assert/strict";

import { ready, DataFactory, Dataset } from "../index.mjs";

await ready();

test("literal(value, { language, direction }) builds an RDF 1.2 directional literal", () => {
  const f = new DataFactory();
  const literal = f.literal("مرحبا", { language: "ar", direction: "rtl" });
  assert.equal(literal.termType, "Literal");
  assert.equal(literal.language, "ar");
  assert.equal(literal.direction, "rtl");
  assert.equal(
    literal.datatype.value,
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString",
  );
});

test("Dataset.from and DataFactory.dataset build chainable datasets from quads", () => {
  const f = new DataFactory();
  const q1 = f.quad(
    f.namedNode("https://e/s1"),
    f.namedNode("https://e/p"),
    f.namedNode("https://e/o1"),
  );
  const q2 = f.quad(
    f.namedNode("https://e/s2"),
    f.namedNode("https://e/p"),
    f.namedNode("https://e/o2"),
  );

  const fromStatic = Dataset.from([q1, q2]);
  assert.equal(fromStatic.size, 2);

  const fromFactory = f.dataset(fromStatic);
  assert.equal(fromFactory.size, 2);

  assert.equal(fromFactory.delete(q1).add(q1), fromFactory);
  assert.equal(fromFactory.size, 2);

  assert.equal(Dataset.from(null).size, 0);
  assert.equal(f.dataset(null).size, 0);
});

test("Dataset#toStream is the instance form of datasetToStream", async () => {
  const f = new DataFactory();
  const q = f.quad(
    f.namedNode("https://e/s"),
    f.namedNode("https://e/p"),
    f.namedNode("https://e/o"),
  );
  const ds = Dataset.from([q]);
  const streamed = [];
  for await (const quad of ds.toStream()) streamed.push(quad);
  assert.equal(streamed.length, 1);
  assert.equal(streamed[0].equals(q), true);
});

test("Dataset visual APIs expose RDF 1.2 statement metadata and SVG", () => {
  const f = new DataFactory();
  const rdfReifies = f.namedNode("http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies");
  const claim = f.namedNode("https://e/claim");
  const quoted = f.quotedTriple(
    f.namedNode("https://e/alice"),
    f.namedNode("https://e/knows"),
    f.namedNode("https://e/bob"),
  );
  const ds = new Dataset();
  ds.add(f.quad(claim, rdfReifies, quoted));

  const model = ds.visualModel({ mode: "compact", maxStatements: 10 });
  assert.equal(model.statements.length, 1);
  assert.equal(model.statements[0].asserted_in.length, 0);
  assert.equal(model.relations[0].kind, "reifies");

  const exported = ds.visualExport({ mode: "compact", width: 720 });
  assert.equal(exported.schema_version, "purrdf-viz-export-1");
  assert.ok(exported.layout.length > 0);
  assert.ok(exported.element_index.some((entry) => entry.kind === "statement"));

  const svg = ds.visualSvg({ mode: "compact", width: 720 });
  assert.match(svg, /<metadata id="purrdf-viz-export"/);
  assert.match(svg, /class="viz-quoted"/);
  assert.doesNotMatch(svg, /class="viz-assertion"/);
});
