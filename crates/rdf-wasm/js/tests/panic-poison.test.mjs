// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Node real-execution test: a Rust panic out of a SYNCHRONOUS call poisons the instance,
// exactly as a trap out of an asynchronous job does. The panic hook reaches the runtime
// before the trap unwinds, so the poison names the panic; the gate linked into the module
// then traps every synchronous entry, and the runtime answers every asynchronous one with
// the poison error. Poisoning is permanent for the JavaScript realm, so the panic runs in
// a child process (`fixtures/panic-child.mjs`), which reports what every call settled as.

import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import * as packageRoot from "../index.mjs";
import { assertSurfacePoisoned, threwAtGate } from "./fixtures/poisoned-surface.mjs";

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
  assert.deepEqual(report.gateBefore, { poisoned: 0, balanced: true }, "the gate is open and balanced before the panic");

  // The poison names the panic — its location and its message — as the runtime learnt it
  // from the hook, before the trap.
  const match = /^the wasm instance trapped \((panicked at [^)]+)\) and cannot be used again; /.exec(
    report.asyncAfter.message ?? "",
  );
  assert.ok(match, `the asynchronous call after the panic did not reject with the poison: ${JSON.stringify(report.asyncAfter)}`);
  assert.match(match[1], /^panicked at crates\/rdf-wasm\/src\/lib\.rs:\d+:\d+: the armed test panic fired$/);
  const POISON =
    `the wasm instance trapped (${match[1]}) and cannot be used again; ` +
    "load the package in a fresh JavaScript realm (a new page, Worker isolate or process)";
  const REJECTED = { settled: "rejected", name: "Error", message: POISON };

  // The panicking call itself throws the trap that follows the hook; the hook has already
  // set the gate's flag by then, with the panicking entry still counted as active.
  assert.deepEqual(report.panicked, { settled: "threw", name: "RuntimeError", message: "unreachable" });
  assert.deepEqual(report.gateAfter, { poisoned: 1, balanced: false });

  // The job in flight when the panic happened rejects with the poison, and so does every
  // later asynchronous call and `ready()` itself.
  for (const name of ["inFlight", "asyncAfter", "readyAfter"]) {
    assert.deepEqual(report[name], REJECTED, name);
  }
  // Every synchronous entry — the typed-error call included — on objects created before
  // the panic, new objects, statics, free functions and `free()` traps at the gate.
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
    "freeAfter",
  ]) {
    assert.deepEqual(report[name], threwAtGate, name);
  }
  assertSurfacePoisoned(report.surface, packageRoot, POISON);
});
