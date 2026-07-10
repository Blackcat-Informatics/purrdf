// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import test from "node:test";

import { DataFactory, Dataset, ready } from "../index.mjs";

await ready();

const EX = "https://example.org/";
const RDF_REIFIES = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

function rdf12Dataset() {
  const f = new DataFactory();
  const alice = f.namedNode(`${EX}alice`);
  const knows = f.namedNode(`${EX}knows`);
  const bob = f.namedNode(`${EX}bob`);
  const claim = f.namedNode(`${EX}claim`);
  const quoted = f.quotedTriple(alice, knows, bob);
  return Dataset.from([
    f.quad(alice, knows, bob, f.namedNode(`${EX}facts`)),
    f.quad(
      claim,
      f.namedNode(RDF_REIFIES),
      quoted,
      f.namedNode(`${EX}claims`),
    ),
    f.quad(
      claim,
      f.namedNode(`${EX}confidence`),
      f.literal("0.8", f.namedNode("http://www.w3.org/2001/XMLSchema#decimal")),
      f.namedNode(`${EX}provenance`),
    ),
  ]);
}

test("visualization APIs preserve RDF 1.2 statement semantics", () => {
  const dataset = rdf12Dataset();
  const options = {
    mode: "compact",
    vocabulary: [{ prefix: "ex", namespace: EX }],
  };
  const model = dataset.visualModel(options);
  assert.equal(model.statements.length, 1);
  assert.equal(model.assertions.length, 1);
  assert.deepEqual(
    model.relations.map(({ kind }) => kind).sort(),
    ["annotation", "reifies"],
  );
  assert.equal(model.statements[0].asserted_in.length, 1);

  const exported = dataset.visualExport(options);
  assert.equal(exported.schema_version, "purrdf-viz-export-1");
  assert.deepEqual(exported.model, model);
  assert.equal(exported.scene.mode, "compact");
  assert.ok(exported.element_index.length > 0);
  assert.doesNotThrow(() => structuredClone(exported));

  const document = dataset.visualSvg({
    ...options,
    svg: { title: "Claim graph", embedMetadata: true },
  });
  assert.match(document.svg, /^<svg /);
  assert.match(document.svg, /<metadata id="purrdf-viz-export"/);
  assert.match(document.svg, /reifies/);
  assert.equal(document.export.model_hash, exported.model_hash);
});

test("visualization modes share the same model", () => {
  const dataset = rdf12Dataset();
  const compact = dataset.visualExport({ mode: "compact" });
  const incidence = dataset.visualExport({ mode: "incidence" });
  const table = dataset.visualExport({ mode: "table" });
  assert.equal(compact.model_hash, incidence.model_hash);
  assert.equal(compact.model_hash, table.model_hash);
  assert.equal(incidence.scene.mode, "incidence");
  assert.equal(table.scene.mode, "table");
  assert.ok(table.scene.table.rows.length > 0);
});
