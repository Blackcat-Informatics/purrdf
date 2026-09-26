// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Node real-execution test: a Rust panic out of a SYNCHRONOUS call poisons the instance,
// exactly as a trap out of an asynchronous job does. Poisoning is permanent for the
// JavaScript realm, so the panic runs in a child process (`fixtures/panic-child.mjs`),
// which reports what every call settled as.

import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import * as packageRoot from "../index.mjs";
import { assertSurfacePoisoned } from "./fixtures/poisoned-surface.mjs";

const EX = "http://example.org/";

test("a panic in a synchronous call poisons every entry point, and typed errors poison nothing", () => {
  const child = spawnSync(process.execPath, [fileURLToPath(new URL("./fixtures/panic-child.mjs", import.meta.url))], {
    encoding: "utf8",
    timeout: 120_000,
  });
  assert.equal(child.status, 0, `the child exited ${child.status}: ${child.stderr}`);
  const report = JSON.parse(child.stdout.trim());

  // The valid neighbours, on the very objects the panic later poisons. Unarmed, the
  // test export does nothing; synchronous calls that throw typed errors throw them
  // again on the second call, not the poison; and afterwards every lane answers exactly.
  assert.deepEqual(report.unarmed, { settled: "returned", value: 0 });
  const PARSE_ERROR = {
    settled: "threw",
    name: "Error",
    message: "error native-codec-parse: expected 3 or 4 terms, got 2 at <unknown>:1:1",
  };
  assert.deepEqual(report.parseError, PARSE_ERROR);
  assert.deepEqual(report.parseErrorAgain, PARSE_ERROR);
  for (const name of ["queryError", "queryErrorAgain"]) {
    assert.equal(report[name].settled, "rejected", name);
    assert.match(report[name].message, /^error native-sparql-query-parse: SPARQL syntax error/, name);
  }
  assert.deepEqual(report.updateBefore, { settled: "resolved" });
  assert.deepEqual(report.syncBefore, { settled: "resolved", subjects: [`${EX}a`, `${EX}b`] });
  assert.deepEqual(report.asyncBefore, { settled: "resolved", subjects: [`${EX}a`, `${EX}b`] });
  assert.deepEqual(report.sizeBefore, { settled: "returned", value: 2 });
  assert.equal(report.versionBefore.settled, "returned");

  // The poison names the panic — its location and its message.
  const match = /^the wasm instance trapped \((panicked at [^)]+)\) and cannot be used again; /.exec(
    report.syncAfter.message ?? "",
  );
  assert.ok(match, `the synchronous call after the panic did not throw the poison: ${JSON.stringify(report.syncAfter)}`);
  assert.match(match[1], /^panicked at crates\/rdf-wasm\/src\/lib\.rs:\d+:\d+: the armed test panic fired$/);
  const POISON =
    `the wasm instance trapped (${match[1]}) and cannot be used again; ` +
    "load the package in a fresh JavaScript realm (a new page, Worker isolate or process)";
  const REJECTED = { settled: "rejected", name: "Error", message: POISON };
  const THREW = { settled: "threw", name: "Error", message: POISON };

  // The panicking call itself: the trap that follows the hook reaches the export's guard,
  // which throws the poison error in its place and keeps the panic as its reason rather
  // than the trap's `unreachable`.
  assert.deepEqual(report.panicked, THREW);

  // The job in flight when the panic happened rejects with it, and so does every later
  // call: asynchronous, synchronous — the typed-error call included — on objects created
  // before the panic, new objects, statics, free functions and `ready()` itself.
  for (const name of ["inFlight", "asyncAfter", "readyAfter"]) {
    assert.deepEqual(report[name], REJECTED, name);
  }
  for (const name of [
    "syncAfter",
    "parseErrorAfter",
    "sizeAfter",
    "addAfter",
    "iterateAfter",
    "quadAfter",
    "newEngineAfter",
    "newDatasetAfter",
    "versionAfter",
  ]) {
    assert.deepEqual(report[name], THREW, name);
  }
  assert.deepEqual(report.freeAfter, { settled: "returned" });
  assertSurfacePoisoned(report.surface, packageRoot, POISON);
});
