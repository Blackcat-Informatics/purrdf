// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Node real-execution conformance for the prepared-shapes-PRODUCT surface reached
// through the PUBLIC package root — `shaclPackProduct` / `shaclProductExplain` /
// `shaclProductCertify` / `shaclProductValidateToSarif` / its `Rebuild` and
// `Expecting` twins — exactly as a host that caches a compiled shapes graph would
// call them. `shacl.test.mjs` covers the plain `shaclValidateToSarif` / `shaclEntail`
// pair; this file drives every product entry point ACROSS THE SAME wasm-bindgen
// boundary: the `Uint8Array` product, the thrown `ShaclProductRefusal` class, and its
// `.dimension` getter surviving intact rather than collapsing into a message string.

import { test } from "node:test";
import assert from "node:assert/strict";

import {
  ready,
  shaclValidateToSarif,
  shaclPackProduct,
  shaclProductExplain,
  shaclProductCertify,
  shaclProductValidateToSarif,
  shaclProductValidateToSarifRebuild,
  shaclProductValidateToSarifExpecting,
  shaclProductValidateToSarifRebuildExpecting,
  ShaclProductRefusal,
} from "../index.mjs";

await ready();

// Shapes as Turtle, data as N-Triples — the exact input contract shared with
// `shaclValidateToSarif` / `shaclPackProduct`.
const SHAPES = `@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
ex:PersonShape a sh:NodeShape ;
  sh:targetClass ex:Person ;
  sh:property [ sh:path ex:age ; sh:datatype xsd:integer ] .
`;

const DATA = `<http://example.org/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .
<http://example.org/alice> <http://example.org/age> "nope" .
`;

// A second shapes graph over a different class, so a product packed from it carries a
// genuinely different identity binding than one packed from SHAPES.
const OTHER_SHAPES = `@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .
ex:WidgetShape a sh:NodeShape ;
  sh:targetClass ex:Widget ;
  sh:property [ sh:path ex:maker ; sh:minCount 1 ] .
`;

const HEX_64 = /^[0-9a-f]{64}$/;

// Read the `identity-digest` line the way a JavaScript host reads it: out of
// `shaclProductExplain`'s own deterministic `key value` rendering.
function identityDigestOf(product) {
  const explained = shaclProductExplain(product);
  const line = explained.split("\n").find((l) => l.startsWith("identity-digest "));
  assert.ok(line, "explain output must carry an identity-digest line");
  return line.slice("identity-digest ".length);
}

test("shaclPackProduct -> shaclProductValidateToSarif round trips to the SAME verdict shaclValidateToSarif reaches", () => {
  const product = shaclPackProduct(SHAPES);
  assert.ok(product instanceof Uint8Array, "a packed product is a Uint8Array");

  const viaProduct = shaclProductValidateToSarif(product, DATA);
  const viaDocument = shaclValidateToSarif(SHAPES, DATA);

  // Equality on CONTENT, not merely "it didn't throw": restoring a product and
  // re-parsing the shapes graph it came from are two routes to one verdict.
  assert.equal(viaProduct, viaDocument);

  const parsed = JSON.parse(viaProduct);
  assert.equal(parsed.version, "2.1.0");
  const results = parsed.runs.flatMap((r) => r.results ?? []);
  assert.ok(results.length >= 1, "the ill-typed age must produce at least one result");
  assert.ok(results.some((r) => r.level === "error"));
});

test("shaclProductExplain reports well-formed identity/format fields without admitting the product", () => {
  const product = shaclPackProduct(SHAPES, "https://example.org/shapes");
  const explained = shaclProductExplain(product);
  const lines = explained.split("\n").filter((l) => l.length > 0);

  const asMap = new Map(lines.map((l) => [l.split(" ", 1)[0], l]));

  assert.equal(asMap.get("format-version"), "format-version 1");
  assert.match(asMap.get("stage-known"), /^stage-known (true|false)$/);

  const stageId = asMap.get("stage-id").slice("stage-id ".length);
  assert.match(stageId, HEX_64, "stage-id is 64 lowercase hex digits");

  const identityDigest = asMap.get("identity-digest").slice("identity-digest ".length);
  assert.match(identityDigest, HEX_64, "identity-digest is 64 lowercase hex digits");

  // The base supplied to shaclPackProduct is recorded verbatim, so a restore
  // resolves the shapes graph's relative references without the document.
  assert.equal(asMap.get("parse-base"), "parse-base https://example.org/shapes");
});

test("shaclProductCertify succeeds on a genuine product", () => {
  const product = shaclPackProduct(SHAPES);
  assert.equal(shaclProductCertify(product), undefined);
});

test("shaclProductValidateToSarifRebuild validates and agrees with the admit path", () => {
  const product = shaclPackProduct(SHAPES);
  const viaAdmit = shaclProductValidateToSarif(product, DATA);
  const viaRebuild = shaclProductValidateToSarifRebuild(product, DATA);

  // Rebuilding re-derives the preparation from the shapes dataset the product
  // carries rather than restoring the memo — a second DOOR onto one product, never
  // a second, divergent answer.
  assert.equal(viaRebuild, viaAdmit);
});

test("*Expecting variants accept the product's own identity digest", () => {
  const product = shaclPackProduct(SHAPES);
  const own = identityDigestOf(product);

  const bound = shaclProductValidateToSarifExpecting(product, DATA, own);
  const unbound = shaclProductValidateToSarif(product, DATA);
  assert.equal(bound, unbound, "stating which product you meant changes the door, not the answer");

  const boundRebuild = shaclProductValidateToSarifRebuildExpecting(product, DATA, own);
  const unboundRebuild = shaclProductValidateToSarifRebuild(product, DATA);
  assert.equal(boundRebuild, unboundRebuild);
});

test("*Expecting variants refuse a product bound to a DIFFERENT identity digest, with a neighbouring valid case beside it", () => {
  const held = shaclPackProduct(SHAPES);
  const wanted = identityDigestOf(shaclPackProduct(OTHER_SHAPES));
  assert.notEqual(wanted, identityDigestOf(held));

  try {
    shaclProductValidateToSarifExpecting(held, DATA, wanted);
    assert.fail("a product bound to a different identity must be refused");
  } catch (err) {
    assert.ok(err instanceof ShaclProductRefusal);
    assert.equal(err.dimension, "shapes-graph");
    err.free();
  }

  try {
    shaclProductValidateToSarifRebuildExpecting(held, DATA, wanted);
    assert.fail("the rebuild door must refuse the same mismatch");
  } catch (err) {
    assert.ok(err instanceof ShaclProductRefusal);
    assert.equal(err.dimension, "shapes-graph");
    err.free();
  }

  // The neighbouring VALID case: the very same bytes, unbound, still validate — the
  // gap `*Expecting` closes is asking WHICH product, not whether the bytes are good.
  shaclProductValidateToSarif(held, DATA);
  shaclProductValidateToSarifRebuild(held, DATA);
});

test("a corrupted product is refused as a ShaclProductRefusal carrying its dimension label across the boundary, with a valid neighbour beside it", () => {
  const product = shaclPackProduct(SHAPES);

  // Flip one byte in the middle of an otherwise-valid product — corruption IN
  // PLACE, not truncation and not a foreign magic number at byte 0.
  const corrupted = Uint8Array.from(product);
  const middle = Math.floor(corrupted.length / 2);
  corrupted[middle] ^= 0xff;

  try {
    shaclProductValidateToSarif(corrupted, DATA);
    assert.fail("a product corrupted in place must not validate");
  } catch (err) {
    assert.ok(err instanceof ShaclProductRefusal, "the thrown value is the structured refusal class");
    // The structured label, not a substring match on the message: this is the
    // exact boundary a JsError would have erased.
    assert.equal(err.dimension, "section-digest");
    assert.ok(err.message.length > 0);
    assert.equal(err.toString(), `section-digest: ${err.message}`);
    err.free();
  }

  // The neighbouring VALID case: the unmodified product, produced the same way,
  // still validates — a suite of refusals alone would be satisfied by a binding
  // that throws on everything.
  shaclProductValidateToSarif(product, DATA);
});
