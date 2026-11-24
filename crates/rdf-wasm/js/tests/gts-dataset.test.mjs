// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: verify byte ownership and the native error handle in the shipped JavaScript binding.

import assert from "node:assert/strict";
import { test } from "node:test";
import { Dataset, ready } from "../index.mjs";

await ready();

test("GTS dataset methods transfer owned bytes and a native read cause", () => {
  const statement = "<https://example.org/s> <https://example.org/p> <https://example.org/o> .";
  const dataset = Dataset.parse(statement, "ntriples");
  const bytes = dataset.toGtsV2();
  dataset.free();
  assert.ok(bytes instanceof Uint8Array);

  const restored = Dataset.fromGts(bytes);
  try {
    assert.equal(restored.serialize("ntriples").trim(), statement);
  } finally {
    restored.free();
  }

  assert.throws(() => Dataset.fromGts(bytes.subarray(0, -1)), error => {
    assert.ok(error instanceof Error);
    try {
      assert.match(error.nativeCauseJson, /gts-v2-torn-tail/);
      assert.equal(error.nativeCause.json, error.nativeCauseJson);
      return true;
    } finally {
      error.nativeCause?.free();
    }
  });
});
