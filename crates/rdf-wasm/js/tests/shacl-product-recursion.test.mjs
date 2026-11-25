// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: The package imports, JS BigInts, receipts and native cause ownership cross wasm-bindgen.
import assert from "node:assert/strict";
import { test } from "node:test";
import {
  CancellationToken,
  CanonicalCause,
  ready,
  shaclPackProduct,
  shaclPackProductWithRecursion,
  shaclProductExplain,
  ShaclProductRefusal,
  shaclProductValidateToSarif,
  shaclProductValidateToSarifAsync,
  shaclProductValidateToSarifExpectingAsync,
  shaclProductValidateToSarifRebuildExpectingAsync,
  shaclProductValidateToSarifWithRecursion,
} from "../index.mjs";
import { canonicalValue } from "./fixtures/poisoned-surface.mjs";

await ready();

const shapes = `@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .
ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:property ex:P .
ex:P sh:path ex:p ; sh:property ex:P .
`;
const data = "<http://example.org/a> <http://example.org/p> <http://example.org/a> .\n";
const max = (1n << 64n) - 1n;
const packArgs = [shapes, undefined, undefined, undefined, undefined, "unlimited"];
const xpathRegex = "xpath-2.0-2010-12-14";
const options = (recursion, recursionMaxPairs = max) =>
  ({ recursion, recursionMaxPairs, recursionMaxIterations: max, xpathRegex });
const conforms = (sarif) => JSON.parse(sarif).runs[0].properties.shaclConforms;

function causeCode(error, code, field, value) {
  const owner = error.nativeCause;
  try {
    assert.ok(owner);
    const json = error.nativeCauseJson;
    if (error instanceof ShaclProductRefusal) error.free();
    assert.equal(owner.json, json);
    let cause = JSON.parse(json);
    while (cause && cause.code !== code) {
      cause = (cause.fields.cause ?? cause.fields.reason)?.value;
    }
    assert.equal(cause?.code, code);
    assert.equal(cause.fields[field].value, value);
    return true;
  } finally {
    owner?.free();
  }
}

function wrongIdentity(error) {
  try {
    assert.ok(error instanceof ShaclProductRefusal);
    assert.equal(error.dimension, "shapes-graph");
    return true;
  } finally {
    if (error instanceof ShaclProductRefusal) error.free();
  }
}

test("prepared SHACL fixed-point requests cross sync and async Wasm routes", async () => {
  for (const [mode, expected] of [["lfp", false], ["gfp", true]]) {
    const packed = shaclPackProductWithRecursion(...packArgs, mode, max, max);
    let product;
    try {
      assert.ok(JSON.parse(packed.canonicalizationJson).length > 0);
      product = packed.value;
      assert.ok(product instanceof Uint8Array);
    } finally {
      packed.free();
    }
    const identity = shaclProductExplain(product).match(/^identity-digest ([a-f0-9]{64})$/mu)?.[1];
    assert.ok(identity);
    const sync = (rebuild, expectedIdentity = identity, pairs = max) =>
      shaclProductValidateToSarifWithRecursion(product, data, rebuild, expectedIdentity, xpathRegex, mode, pairs, max);
    assert.equal(conforms(sync(false)), expected);
    assert.equal(sync(true), sync(false));
    assert.equal(await shaclProductValidateToSarifExpectingAsync(product, data, identity, options(mode)), sync(false));
    assert.equal(await shaclProductValidateToSarifRebuildExpectingAsync(product, data, identity, options(mode)), sync(true));

    const other = "0".repeat(64);
    assert.throws(() => sync(false, other), wrongIdentity);
    await assert.rejects(shaclProductValidateToSarifRebuildExpectingAsync(product, data, other, options(mode)), wrongIdentity);
    const pairLimit = (error) => causeCode(error, "shacl-recursion-pair-limit", "limit", "1");
    assert.throws(() => sync(false, identity, 1n), pairLimit);
    await assert.rejects(shaclProductValidateToSarifAsync(product, data, options(mode, 1n)), pairLimit);
    const zero = (error) => causeCode(error, "shacl-recursion-pair-limit", "limit", "0");
    await assert.rejects(shaclProductValidateToSarifAsync(product, data, options(mode, 0n)), zero);
  }

  const missing = (error) => {
    assert.ok(error.nativeCauseJson);
    error.nativeCause?.free();
    assert.deepEqual(JSON.parse(error.canonicalizationJson), []);
    return true;
  };
  assert.throws(() => shaclPackProductWithRecursion(...packArgs, "gfp", undefined, max), missing);
  const stop = new CancellationToken();
  try {
    stop.cancel();
    assert.throws(() => shaclPackProductWithRecursion(...packArgs, "gfp", max, max, stop.share()), (error) => {
      const owner = error.nativeCause;
      try {
        assert.ok(error instanceof ShaclProductRefusal);
        assert.ok(owner instanceof CanonicalCause);
        assert.ok(Array.isArray(JSON.parse(error.canonicalizationJson)));
        assert.equal(JSON.parse(owner.json).code, "canonical-stopped");
        return true;
      } finally {
        owner?.free();
        if (error instanceof ShaclProductRefusal) error.free();
      }
    });
  } finally {
    stop.free();
  }

  const plain = shapes.replace("ex:P sh:path ex:p ; sh:property ex:P .", "ex:P sh:path ex:p .");
  const plainProduct = canonicalValue(shaclPackProduct(plain, ...packArgs.slice(1)));
  assert.equal(await shaclProductValidateToSarifAsync(plainProduct, data), shaclProductValidateToSarif(plainProduct, data));
});
