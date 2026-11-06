// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: JavaScript exception prototypes and wasm-bindgen cause-handle
// ownership across synchronous and asynchronous exports require the real host.

import { test } from "node:test";
import assert from "node:assert/strict";
import { ready, Dataset, QueryEngine, entailRules } from "../index.mjs";

await ready();

test("reasoning errors retain native handles across direct and query exports", async () => {
  const dataset = new Dataset();
  const engine = new QueryEngine();
  const retained = [];
  const capture = (error) => {
    assert.ok(error instanceof Error);
    assert.equal(error instanceof WebAssembly.RuntimeError, false);
    const fields = JSON.parse(error.nativeCauseJson);
    assert.equal(fields.code, "reasoning-request");
    assert.equal(fields.fields.cause.value.fields.offered.value, "RDFS");
    retained.push([error.nativeCause, error.nativeCauseJson]);
    return true;
  };
  try {
    assert.throws(() => entailRules("RDFS"), capture);
    assert.throws(
      () => engine.queryEntailmentGoverned(dataset, "ASK {}", "RDFS"),
      capture,
    );
    await assert.rejects(
      engine.queryEntailmentGovernedAsync(dataset, "ASK {}", "RDFS"),
      capture,
    );
    assert.ok(Array.isArray(entailRules("rdfs")));
  } finally {
    engine.free();
    dataset.free();
  }
  for (const [cause, json] of retained) {
    try {
      assert.equal(cause.json, json);
    } finally {
      cause.free();
    }
  }
});
