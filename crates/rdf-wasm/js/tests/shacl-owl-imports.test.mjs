// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// One shapes graph, one owl:imports verdict, on every JavaScript entry point.
//
// A shapes graph that `owl:imports <http://example.org/lib>` is driven through every
// shapes-graph function the package root exports. Without an import table each rejects
// with the same `ShaclImportError` (kind "unresolved-import", naming the IRI) — the
// refusal the Rust API, the command line, Python and C raise for the same graph. With the
// imported document in `importIris` / `importDocuments` each APPLIES it, observed through
// a result the importing document alone cannot produce.

import { test } from "node:test";
import assert from "node:assert/strict";

import {
  ready,
  ShaclImportError,
  shaclApplyRules,
  shaclEntail,
  shaclEvalNodeExpr,
  shaclLintShapes,
  shaclPackProduct,
  shaclProductValidateToSarif,
  shaclValidateChangesToSarif,
  shaclValidateToSarif,
} from "../index.mjs";

await ready();

const LIB = "http://example.org/lib";

// The importing shapes graph: an ontology header and its import, no shape of its own.
const IMPORTER = `@prefix owl: <http://www.w3.org/2002/07/owl#> .
<http://example.org/shapes> a owl:Ontology ;
  owl:imports <http://example.org/lib> .
`;

// The imported document: a shape needing ex:name, a rule tagging every ex:Person
// ex:checked ex:yes, and a node expression ex:Who reading the scope variable `who`.
const IMPORTED = `@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix shnex: <http://www.w3.org/ns/shacl-node-expr#> .
@prefix ex: <http://example.org/> .
ex:NameShape a sh:NodeShape ;
  sh:targetClass ex:Person ;
  sh:property [ sh:path ex:name ; sh:minCount 1 ] ;
  sh:rule [ a sh:TripleRule ; sh:subject sh:this ; sh:predicate ex:checked ;
            sh:object ex:yes ] .
ex:Who shnex:var "who" .
`;

// One ex:Person with no ex:name.
const PERSON =
  "<http://example.org/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .\n";

const INFERRED =
  "<http://example.org/alice> <http://example.org/checked> <http://example.org/yes> .";

// Each entry point, called with the table `[iris, documents]` (both `undefined` = none).
const ENTRY_POINTS = {
  shaclValidateToSarif: (iris, documents) =>
    shaclValidateToSarif(IMPORTER, PERSON, undefined, undefined, iris, documents),
  shaclValidateChangesToSarif: (iris, documents) =>
    shaclValidateChangesToSarif(IMPORTER, "", PERSON, undefined, undefined, iris, documents),
  shaclEntail: (iris, documents) => shaclEntail(IMPORTER, PERSON, undefined, iris, documents),
  shaclApplyRules: (iris, documents) =>
    shaclApplyRules(
      PERSON,
      IMPORTER,
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      iris,
      documents,
    ),
  shaclEvalNodeExpr: (iris, documents) =>
    shaclEvalNodeExpr(
      IMPORTER,
      PERSON,
      "http://example.org/Who",
      "http://example.org/alice",
      ["who=http://example.org/bob"],
      undefined,
      iris,
      documents,
    ),
  shaclLintShapes: (iris, documents) => shaclLintShapes(IMPORTER, undefined, iris, documents),
  shaclPackProduct: (iris, documents) => shaclPackProduct(IMPORTER, undefined, iris, documents),
};

for (const [name, call] of Object.entries(ENTRY_POINTS)) {
  test(`${name} rejects an unresolved owl:imports with the typed ShaclImportError`, () => {
    let caught;
    try {
      call(undefined, undefined);
    } catch (err) {
      caught = err;
    }
    assert.ok(caught instanceof ShaclImportError, `${name}: the thrown value is the typed class`);
    assert.equal(caught.kind, "unresolved-import");
    assert.deepEqual(caught.iris, [LIB]);
    assert.match(caught.message, /<http:\/\/example\.org\/lib>/);
    caught.free();
  });
}

test("every entry point applies a supplied import", () => {
  const iris = [LIB];
  const documents = [IMPORTED];

  assert.match(ENTRY_POINTS.shaclValidateToSarif(iris, documents), /MinCountConstraintComponent/);

  const change = ENTRY_POINTS.shaclValidateChangesToSarif(iris, documents);
  assert.match(change.sarif, /MinCountConstraintComponent/);
  change.free();

  assert.ok(ENTRY_POINTS.shaclEntail(iris, documents).includes(INFERRED));

  const rules = ENTRY_POINTS.shaclApplyRules(iris, documents);
  assert.equal(rules.inferred.trim(), INFERRED);
  rules.free();

  assert.deepEqual(ENTRY_POINTS.shaclEvalNodeExpr(iris, documents), ["<http://example.org/bob>"]);

  const lint = ENTRY_POINTS.shaclLintShapes(iris, documents);
  assert.equal(lint.clean, true, lint.report);
  lint.free();

  const product = ENTRY_POINTS.shaclPackProduct(iris, documents);
  assert.match(shaclProductValidateToSarif(product, PERSON), /MinCountConstraintComponent/);
});

test("an ontology declared in place resolves the import, and its neighbour does not", () => {
  const merged = IMPORTER + IMPORTED;
  for (const declaration of [
    "<http://example.org/lib> a <http://www.w3.org/2002/07/owl#Ontology> .\n",
    "<http://example.org/lib-series> <http://www.w3.org/2002/07/owl#versionIRI> <http://example.org/lib> .\n",
  ]) {
    assert.match(shaclValidateToSarif(merged + declaration, PERSON), /MinCountConstraintComponent/);
  }
  assert.throws(
    () => shaclValidateToSarif(merged, PERSON),
    (err) => err instanceof ShaclImportError && err.kind === "unresolved-import",
  );
});

test("a table entry nothing imports is refused, and a reached one is not", () => {
  const plain = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n<http://example.org/S> a sh:NodeShape .\n";
  assert.throws(
    () => shaclValidateToSarif(plain, PERSON, undefined, undefined, [LIB], [IMPORTED]),
    (err) => err instanceof ShaclImportError && err.kind === "unreached-import",
  );
  assert.match(
    shaclValidateToSarif(IMPORTER, PERSON, undefined, undefined, [LIB], [IMPORTED]),
    /MinCountConstraintComponent/,
  );
});

test("import arrays of different lengths are refused, never truncated", () => {
  assert.throws(
    () => shaclValidateToSarif(IMPORTER, PERSON, undefined, undefined, [LIB], []),
    /PAIR/,
  );
});
