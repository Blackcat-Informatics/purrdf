// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: this checks JavaScript Error prototypes across the generated
// wasm-bindgen boundary, which a native Rust test cannot observe.

import { test } from "node:test";
import assert from "node:assert/strict";
import { ready, Dataset } from "../index.mjs";

await ready();
test("isomorphic refusal crosses the wasm boundary as a JavaScript Error", () => {
  const dataset = Dataset.parse(
    "<https://example.org/s> <https://example.org/p> <urn:purrdf:rdfc:bad> .",
    "turtle",
  );
  let cause;
  assert.throws(() => dataset.isomorphic(dataset, "18446744073709551615"), (error) => {
    assert.ok(error instanceof Error);
    assert.equal(error instanceof WebAssembly.RuntimeError, false);
    assert.equal(JSON.parse(error.canonicalizationJson).at(-1).policy.finite, "18446744073709551615");
    cause = error.nativeCause;
    assert.match(error.message, /urn:purrdf:rdfc:bad/);
    return true;
  });
  dataset.free(); assert.match(cause.json, /urn:purrdf:rdfc:bad/); cause.free();
});
