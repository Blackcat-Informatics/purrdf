// SPDX-License-Identifier: MIT OR Apache-2.0
//
// SARIF-contract smoke gate for the PurRDF console. The console renders SHACL
// results as SARIF and asserts the version the UI accepts via
// `assertSarifVersion` in `docs/playground/sarif.mjs`. That module is
// side-effect-free and Node-importable (unlike `app.mjs`, which spawns a Worker
// at module-eval), so this gate exercises the ACTUAL production assertion the UI
// runs — a drift from SARIF 2.1.0 must be surfaced, never silently echoed.

import { test } from "node:test";
import assert from "node:assert/strict";

import {
  assertSarifVersion,
  describeSarif,
  EXPECTED_SARIF_VERSION,
} from "../sarif.mjs";

test("the console's SARIF contract is 2.1.0", () => {
  assert.equal(EXPECTED_SARIF_VERSION, "2.1.0");
});

test("assertSarifVersion accepts SARIF 2.1.0 (no warning)", () => {
  assert.equal(assertSarifVersion("2.1.0"), null);
});

test("assertSarifVersion warns (not silently) on a drifting version", () => {
  const warning = assertSarifVersion("2.0.0");
  assert.equal(typeof warning, "string");
  assert.ok(warning.length > 0, "a drift must produce a non-empty warning");
  assert.match(warning, /2\.1\.0/, "the warning must name the expected version");
  assert.match(warning, /2\.0\.0/, "the warning must name the seen version");
});

test("assertSarifVersion warns on a missing version", () => {
  const warning = assertSarifVersion(undefined);
  assert.equal(typeof warning, "string");
  assert.match(warning, /2\.1\.0/);
});

test("describeSarif summarizes version + result count", () => {
  const sarif = { version: "2.1.0", runs: [{ results: [{}, {}] }] };
  assert.equal(describeSarif(sarif), "SARIF 2.1.0 · 2 result(s).");
});
