// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// The `shapesGraph` argument — the IRI SHACL-SPARQL sees the shapes graph under, as
// `purrdf validate --shapes-graph` names it — on every shapes-graph entry point that takes
// it, graded on the W3C SHACL 1.0 test `sparql/pre-binding/shapesGraph-001`.
//
// That test's constraint selects its focus node only when `$shapesGraph` is bound and
// `GRAPH $shapesGraph { $currentShape ex:property 42 }` reads the shapes graph. SHACL 1.0
// pre-binds `$shapesGraph` and approves ONE result; SHACL 1.2 removed the pre-binding, so
// with no IRI named `$shapesGraph` is an ordinary, unbound variable and the data graph
// CONFORMS. Every entry point is asked both questions.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { fileURLToPath, pathToFileURL } from "node:url";

import {
  ready,
  Dataset,
  shaclLintShapes,
  shaclPackProduct,
  shaclProductValidateToSarif,
  shaclValidateChangesToSarif,
  shaclValidateToSarif,
} from "../index.mjs";

await ready();

const TEST = fileURLToPath(
  new URL("../../../../vectors/shacl/sparql/pre-binding/shapesGraph-001.ttl", import.meta.url),
);
const BASE = pathToFileURL(TEST).href;
const SHAPES = await readFile(TEST, "utf8");
// The test's data graph is the same file.
const parsed = Dataset.parse(SHAPES, "turtle", BASE);
const DATA = parsed.serialize("ntriples");
parsed.free();
const FOCUS =
  "http://datashapes.org/sh/tests/sparql/pre-binding/shapesGraph-001.test#InvalidResource";

/** `[conforms, results]` of a SARIF log; an empty `results` array is omitted. */
function verdict(sarif) {
  const run = JSON.parse(sarif).runs[0];
  return [run.properties.shaclConforms, run.results ?? []];
}

test("wasm_shacl_shapes_graph: shapesGraph-001 through shaclValidateToSarif", () => {
  const [conforms, results] = verdict(
    shaclValidateToSarif(SHAPES, DATA, BASE, undefined, undefined, undefined, BASE),
  );
  assert.equal(conforms, false);
  assert.equal(results.length, 1);
  assert.ok(JSON.stringify(results[0]).includes(FOCUS));
  // No IRI: SHACL 1.2's ordinary variable, unbound, so the data graph conforms.
  const [unnamed, none] = verdict(shaclValidateToSarif(SHAPES, DATA, BASE));
  assert.equal(unnamed, true);
  assert.equal(none.length, 0);
  // A relative IRI resolves against the shapes document's base...
  const [relative] = verdict(
    shaclValidateToSarif(SHAPES, DATA, BASE, undefined, undefined, undefined, "shapesGraph-001.ttl"),
  );
  assert.equal(relative, false);
});

test("wasm_shacl_shapes_graph: the change path, a packed product and lint carry it", () => {
  for (const [graph, expected] of [
    [BASE, 1],
    [undefined, 0],
  ]) {
    const change = shaclValidateChangesToSarif(
      SHAPES,
      "",
      DATA,
      undefined,
      BASE,
      undefined,
      undefined,
      graph,
    );
    assert.equal(verdict(change.sarif)[1].length, expected, `change path, ${graph}`);
    change.free();
    const product = shaclPackProduct(SHAPES, BASE, undefined, undefined, graph);
    assert.equal(
      verdict(shaclProductValidateToSarif(product, DATA))[1].length,
      expected,
      `product, ${graph}`,
    );
    const lint = shaclLintShapes(SHAPES, BASE, undefined, undefined, graph);
    lint.free();
  }
});

test("wasm_shacl_shapes_graph: a relative shapes graph with no base is refused", () => {
  const shapes =
    "@prefix sh: <http://www.w3.org/ns/shacl#> .\n@prefix ex: <http://example.org/> .\n" +
    "ex:S a sh:NodeShape ; sh:targetNode ex:n ;\n" +
    '  sh:sparql [ sh:select "SELECT $this WHERE { FILTER bound($shapesGraph) }" ] .\n';
  assert.throws(
    () => shaclValidateToSarif(shapes, "", undefined, undefined, undefined, undefined, "shapes"),
    (error) => error.message.startsWith("shapes graph `shapes`: iri-relative-no-base"),
  );
  // The absolute neighbour needs no base, and binds $shapesGraph.
  const [conforms, results] = verdict(
    shaclValidateToSarif(
      shapes,
      "",
      undefined,
      undefined,
      undefined,
      undefined,
      "http://example.org/shapes",
    ),
  );
  assert.equal(conforms, false);
  assert.equal(results.length, 1);
});
