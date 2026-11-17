// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: this checks JavaScript Error prototypes across the generated
// wasm-bindgen boundary, which a native Rust test cannot observe.

import { test } from "node:test";
import assert from "node:assert/strict";
import { ready, Dataset } from "../index.mjs";
import { canonicalValue } from "./fixtures/poisoned-surface.mjs";
await ready();
const CIRCULANT = Array.from({ length: 13 }, (_, u) => Array.from({ length: 13 - u - 1 }, (_, k) => u + k + 1).filter((v) => [1, 3, 4, 9, 10, 12].includes((v - u) % 13))
  .map((v) => `_:v${u} <http://example.org/p> _:v${v} .\n_:v${v} <http://example.org/p> _:v${u} .\n`).join("")).join("");
test("isomorphic refusal crosses the wasm boundary as a JavaScript Error", () => {
  const dataset = Dataset.parse(CIRCULANT, "turtle");
  let cause;
  assert.throws(() => dataset.isomorphic(dataset, "0"), (error) => {
    assert.ok(error instanceof Error);
    assert.equal(error instanceof WebAssembly.RuntimeError, false);
    assert.equal(JSON.parse(error.canonicalizationJson).at(-1).policy.finite, "0");
    cause = error.nativeCause;
    assert.match(error.message, /labelling refused/);
    return true;
  });
  dataset.free(); assert.match(cause.json, /canonical-refused/); cause.free();
  assert.equal(canonicalValue(Dataset.parse(CIRCULANT, "turtle").isomorphic(Dataset.parse(CIRCULANT, "turtle"), "unlimited")), true);
});
