// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// `QueryEngine.blankScope`, the declared option for how a blank node's scope crosses into
// JS. The engine holds a blank node as a (label, scope) pair. By default (`"keep"`) two
// blank nodes that share a label in different scopes stay two nodes in JS; `"merge"`
// drops the scope, so they are one node with the bare label.
//
// The scoped nodes are put into a dataset as their scope envelopes (`purrdfesc<scope>_<label>`
// is the label `<label>` at scope `<scope>`, and a plain label is the default scope), which
// the dataset takes back into (label, scope) pairs. So the dataset holds `(b, 1) p (b, 2)`:
// two blank nodes with the label `b` that only their scopes tell apart.

import { test } from "node:test";
import assert from "node:assert/strict";

import { ready, DataFactory, Dataset, QueryEngine } from "../index.mjs";

await ready();

const factory = new DataFactory();
const P = factory.namedNode("https://example.org/p");
const QUERY = "SELECT ?x ?y WHERE { ?x <https://example.org/p> ?y }";

/** `(b, 1) p (b, 2)`, and `(b, default) p (b, 1)`. */
function scopedDataset() {
  const dataset = new Dataset();
  const blank = (label) => factory.blankNode(label);
  dataset.add(factory.quad(blank("purrdfesc1_b"), P, blank("purrdfesc2_b"), factory.defaultGraph()));
  dataset.add(factory.quad(blank("b"), P, blank("purrdfesc1_b"), factory.defaultGraph()));
  return dataset;
}

/** Both rows of QUERY, as `[x, y]` term pairs, in a fixed order. */
function rows(engine) {
  return engine
    .select(scopedDataset(), `${QUERY} ORDER BY ?x ?y`)
    .rows.toArray()
    .map((row) => [row.x, row.y]);
}

test("a fresh engine keeps scope: two same-label blanks in different scopes stay distinct", () => {
  const engine = new QueryEngine();
  assert.equal(engine.blankScope, "keep");
  const [[unscoped, scoped], [first, second]] = rows(engine);
  for (const term of [first, second, unscoped, scoped]) assert.equal(term.termType, "BlankNode");
  assert.equal(first.equals(second), false, "(b, 1) and (b, 2) collapsed under keep");
  // The unscoped blank keeps its bare label, and the same (b, 1) is the same node in both rows.
  assert.equal(unscoped.value, "b");
  assert.equal(scoped.equals(first), true);
  assert.equal(unscoped.equals(scoped), false);
});

test('blankScope = "merge" drops the scope: the same blanks become one node', () => {
  const engine = new QueryEngine();
  engine.blankScope = "merge";
  assert.equal(engine.blankScope, "merge");
  for (const [x, y] of rows(engine)) {
    assert.equal(x.value, "b");
    assert.equal(y.value, "b");
    assert.equal(x.equals(y), true);
  }
});

test("the mode is per engine, and can be set back to keep", () => {
  const merging = new QueryEngine();
  merging.blankScope = "merge";
  const keeping = new QueryEngine();
  assert.equal(rows(keeping)[0][0].equals(rows(keeping)[0][1]), false);
  assert.equal(rows(merging)[0][0].equals(rows(merging)[0][1]), true);
  merging.blankScope = "keep";
  assert.equal(rows(merging)[0][0].equals(rows(merging)[0][1]), false);
});

test("a value that is not a mode throws with the options code and leaves the mode alone", () => {
  const engine = new QueryEngine();
  engine.blankScope = "merge";
  for (const bad of ["Merge", "collapse", ""]) {
    assert.throws(
      () => {
        engine.blankScope = bad;
      },
      (error) => {
        assert.equal(error.code, "purrdf-wasm-options");
        assert.match(error.message, /"keep" or "merge"/);
        return true;
      },
    );
  }
  assert.equal(engine.blankScope, "merge");
});

test("an unscoped blank and an IRI are the same under both modes", () => {
  const dataset = new Dataset();
  dataset.add(factory.quad(factory.blankNode("b"), P, factory.namedNode("https://example.org/o"), factory.defaultGraph()));
  const seen = [];
  for (const mode of ["keep", "merge"]) {
    const engine = new QueryEngine();
    engine.blankScope = mode;
    const [only] = engine.select(dataset, QUERY).rows.toArray();
    seen.push([only.x.termType, only.x.value, only.y.value]);
  }
  assert.deepEqual(seen[0], ["BlankNode", "b", "https://example.org/o"]);
  assert.deepEqual(seen[1], seen[0]);
});

test("the dataset's own quads are not affected by the option", () => {
  const engine = new QueryEngine();
  engine.blankScope = "merge";
  const dataset = scopedDataset();
  engine.select(dataset, QUERY);
  assert.deepEqual(
    dataset.quads().map((quad) => quad.subject.value).sort(),
    ["b", "purrdfesc1_b"],
  );
});

test("an asynchronous job takes the mode in force when it begins", async () => {
  const engine = new QueryEngine();
  engine.blankScope = "merge";
  const merged = (await engine.selectAsync(scopedDataset(), `${QUERY} ORDER BY ?x ?y`)).rows.toArray()[0];
  assert.equal(merged.x.equals(merged.y), true);
  engine.blankScope = "keep";
  const kept = (await engine.selectAsync(scopedDataset(), `${QUERY} ORDER BY ?x ?y`)).rows.toArray()[0];
  assert.equal(kept.x.equals(kept.y), false);
});


test("typed scoped blanks retain identity through insertion, serialization and parsing", async () => {
  for (const mode of ["keep", "merge"]) {
    const engine = new QueryEngine(); engine.blankScope = mode;
    for (const async of [false, true]) {
      const original = scopedDataset();
      const result = async ? await engine.selectAsync(original, `${QUERY} ORDER BY ?x ?y`) : engine.select(original, `${QUERY} ORDER BY ?x ?y`);
      const captured = result.rows.toArray();
      const copied = Dataset.from(captured.map(({ x, y }) => factory.quad(x, P, y, factory.defaultGraph())));
      const parsed = Dataset.parse(copied.serialize("ntriples"), "ntriples");
      assert.equal(copied.isomorphic(parsed), true);
      const recovered = engine.select(parsed, `${QUERY} ORDER BY ?x ?y`).rows.toArray();
      assert.equal(recovered.length, mode === "keep" ? 2 : 1);
      assert.deepEqual(recovered.map(({ x, y }) => [x.value, y.value]), mode === "keep" ? captured.map(({ x, y }) => [x.value, y.value]) : [["b", "b"]]);
    }
  }
});
