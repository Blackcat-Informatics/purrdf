// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: These cases exercise the JavaScript package's positional and
// async options, exact BigInts, native error fields and owned results through Wasm.
import assert from "node:assert/strict";
import { test } from "node:test";
import {
  ready,
  shaclProductValidateToSarifAsync,
  shaclValidateToSarifWithRecursion,
  shaclValidateChangesToSarifWithRecursion,
  shaclValidateToSarifAsync,
  shaclValidateChangesToSarifAsync,
} from "../index.mjs";

await ready();

// Same positive recursive fixture and verdicts as the Python boundary.
const shapes = `@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .
ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:property ex:P .
ex:P sh:path ex:p ; sh:property ex:P .
`;
const data = "<http://example.org/a> <http://example.org/p> <http://example.org/a> .\n";
const max = (1n << 64n) - 1n;
const whole = [shapes, data, ...Array(6).fill(undefined)];
const change = [shapes, "", data, ...Array(5).fill(undefined)];
const controls = (recursion, recursionMaxPairs = max, recursionMaxIterations = max) =>
  ({ recursion, recursionMaxPairs, recursionMaxIterations });

function scope(result) {
  try {
    return {
      sarif: result.sarif,
      bounded: result.bounded,
      focusNodes: result.focusNodes,
      reason: result.reason,
    };
  } finally {
    result.free();
  }
}

function conforms(sarif) {
  return JSON.parse(sarif).runs[0].properties.shaclConforms;
}

test("SHACL fixed-point text requests cross sync and async Wasm routes", async () => {
  for (const [mode, expected] of [["lfp", false], ["gfp", true]]) {
    const sync = shaclValidateToSarifWithRecursion(...whole, undefined, undefined, mode, max, max);
    assert.equal(conforms(sync), expected);
    assert.equal(await shaclValidateToSarifAsync(...whole, controls(mode)), sync);

    const changed = scope(shaclValidateChangesToSarifWithRecursion(...change, undefined, mode, max, max));
    assert.equal(conforms(changed.sarif), expected);
    if (changed.bounded) {
      assert.equal(changed.focusNodes, 1);
      assert.equal(changed.reason, undefined);
    } else {
      assert.equal(changed.focusNodes, undefined);
      assert.ok(changed.reason);
    }
    assert.deepEqual(
      scope(await shaclValidateChangesToSarifAsync(...change, controls(mode))),
      changed,
    );
  }
});

function nativeRefusal(code, field, value) {
  return (error) => {
    try {
      assert.equal(error.nativeCause.json, error.nativeCauseJson);
      let cause = JSON.parse(error.nativeCauseJson);
      while (cause && cause.code !== code) {
        cause = (cause.fields.cause ?? cause.fields.reason)?.value;
      }
      assert.equal(cause?.code, code);
      assert.equal(cause.fields[field].value, value);
      return true;
    } finally {
      error.nativeCause?.free();
    }
  };
}

test("SHACL fixed-point host refusals preserve native cause and unsigned limits", async () => {
  for (const [pairs, iterations, field] of [[max, undefined, "max_pairs"], [undefined, max, "max_iterations"]]) {
    const missing = nativeRefusal("shacl-recursion-request-missing-limit", field, max.toString());
    assert.throws(
      () => shaclValidateToSarifWithRecursion(...whole, undefined, undefined, "gfp", pairs, iterations),
      missing,
    );
    await assert.rejects(shaclValidateToSarifAsync(...whole, {
      recursion: "gfp", recursionMaxPairs: pairs, recursionMaxIterations: iterations,
    }), missing);
  }
  for (const [pairs, iterations, code] of [[0, max, "pair"], [max, 0n, "iteration"]]) {
    const zero = nativeRefusal(`shacl-recursion-${code}-limit`, "limit", "0");
    assert.throws(
      () => shaclValidateChangesToSarifWithRecursion(...change, undefined, "gfp", pairs, iterations),
      zero,
    );
    await assert.rejects(shaclValidateChangesToSarifAsync(...change, controls("gfp", pairs, iterations)), zero);
  }
  const oversized = max + 1n;
  assert.throws(
    () => shaclValidateToSarifWithRecursion(...whole, undefined, undefined, "gfp", oversized, max),
    /recursionMaxPairs/,
  );
  await assert.rejects(
    shaclValidateToSarifAsync(...whole, controls("gfp", oversized, max)),
    /recursionMaxPairs/,
  );
  await assert.rejects(
    shaclProductValidateToSarifAsync(new Uint8Array(), "", controls("gfp")),
    /recursion options apply only to SHACL text validation/,
  );
  assert.equal(conforms(scope(await shaclValidateChangesToSarifAsync(...change, controls("gfp"))).sarif), true);
});
