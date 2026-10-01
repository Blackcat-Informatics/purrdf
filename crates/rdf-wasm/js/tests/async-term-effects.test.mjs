// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

function child(script, mode) {
  const result = spawnSync(process.execPath,
    [...process.execArgv, fileURLToPath(new URL(`./fixtures/${script}`, import.meta.url)), ...mode],
    { encoding: "utf8", timeout: 120_000 });
  assert.equal(result.error, undefined, result.error?.message);
  assert.equal(result.status, 0, result.stderr);
  return JSON.parse(result.stdout.trim());
}

test("host-effect deadlines settle in otherwise idle Node, with every timer released", () => {
  for (const kind of ["service", "load"]) {
    for (const action of ["hung", "success", "failure", "cancel"]) {
      const mode = `${action}-${kind}`;
      assert.deepEqual(child("effect-lifecycle-child.mjs", [mode]), { mode, timers: 0 });
    }
  }
  assert.deepEqual(child("effect-lifecycle-child.mjs", ["siblings-service"]), { mode: "siblings-service", timers: 0 });
});

test("a 20,000-level SERVICE term can be walked, cloned, compared and dropped without poisoning", () => {
  assert.deepEqual(child("deep-service-term-child.mjs", []), { depth: 20_000, lanes: 2 });
});
