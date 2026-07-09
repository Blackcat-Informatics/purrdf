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
