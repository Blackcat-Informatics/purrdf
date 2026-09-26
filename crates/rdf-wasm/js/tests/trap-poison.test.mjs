// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// A trap that is not a panic, out of a SYNCHRONOUS call, poisons the instance — exactly
// as a trap out of an asynchronous job and a panic in any call do. The real trap runs in
// a child process (`fixtures/sync-trap-child.mjs`), because poisoning is permanent for
// the JavaScript realm. The guard itself (`purrdf_jspi_bind_glue`) is also exercised on
// a fresh instance of the runtime module over hand-built exports, where every kind of
// throw can be raised on demand and told apart.

import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import * as packageRoot from "../index.mjs";
import { assertSurfacePoisoned } from "./fixtures/poisoned-surface.mjs";

const EX = "http://example.org/";

test("a trap that is not a panic, in a synchronous call, poisons every entry point", () => {
  const child = spawnSync(process.execPath, [fileURLToPath(new URL("./fixtures/sync-trap-child.mjs", import.meta.url))], {
    encoding: "utf8",
    timeout: 120_000,
  });
  assert.equal(child.status, 0, `the child exited ${child.status}: ${child.stderr}`);
  const report = JSON.parse(child.stdout.trim());

  // The valid neighbours, on the very objects the trap later poisons: the unarmed export
  // does nothing, typed errors are thrown again (not the poison) on the second call, and
  // afterwards every lane — the asynchronous one included — answers exactly.
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
  assert.deepEqual(report.asyncUpdateBefore, { settled: "resolved" });
  assert.deepEqual(report.asyncAfterUpdate, { settled: "resolved", subjects: [`${EX}a`, `${EX}b`, `${EX}c`] });
  assert.deepEqual(report.sizeBefore, { settled: "returned", value: 3 });
  assert.equal(report.versionBefore.settled, "returned");

  // The trapping call throws the poison, which names the trap — no panic ran.
  const POISON =
    "the wasm instance trapped (RuntimeError: unreachable) and cannot be used again; " +
    "load the package in a fresh JavaScript realm (a new page, Worker isolate or process)";
  const REJECTED = { settled: "rejected", name: "Error", message: POISON };
  const THREW = { settled: "threw", name: "Error", message: POISON };
  assert.deepEqual(report.trapped, THREW);

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

test("the guard poisons on a trap only, and forwards every argument and value", async () => {
  // A module instance of its own: this one's poisoning reaches nothing else.
  const runtime = await import(`../src/purrdf_jspi.mjs?guard-unit-${process.pid}`);
  const memory = new WebAssembly.Memory({ initial: 1 });
  const global = new WebAssembly.Global({ value: "i32", mutable: true }, 7);
  const table = new WebAssembly.Table({ element: "anyfunc", initial: 1 });
  const run = () => 0;
  const typeError = new TypeError("a host callback refused");
  const hostError = new Error("the example.org host failed");
  const exports = Object.freeze({
    memory,
    global,
    table,
    purrdf_jspi_run: run,
    zero: () => 42,
    sum2: (a, b) => a + b,
    sum10: (a, b, c, d, e, f, g, h, i, j) => [a, b, c, d, e, f, g, h, i, j].join(","),
    returnsError: () => typeError,
    throwsType: () => {
      throw typeError;
    },
    throwsHost: () => {
      throw hostError;
    },
    throwsString: () => {
      throw "a thrown string";
    },
    traps: () => {
      throw new WebAssembly.RuntimeError("memory access out of bounds");
    },
    trapsOther: () => {
      throw new WebAssembly.RuntimeError("unreachable");
    },
  });
  let retargeted;
  const live = runtime.purrdf_jspi_bind_glue(exports, (gated) => {
    retargeted = gated;
  });

  // Every export is there; non-functions and the asynchronous runner are the instance's
  // own objects, every other function is a guard of the same arity.
  assert.deepEqual(Object.keys(live), Object.keys(exports));
  assert.ok(Object.isFrozen(live));
  assert.equal(live.memory, memory);
  assert.equal(live.global, global);
  assert.equal(live.table, table);
  assert.equal(live.purrdf_jspi_run, run);
  for (const name of ["zero", "sum2", "sum10", "returnsError", "throwsType", "throwsHost", "throwsString", "traps", "trapsOther"]) {
    assert.notEqual(live[name], exports[name], name);
    assert.equal(live[name].length, exports[name].length, name);
  }

  // Values and arguments pass through exactly, the rest-parameter path included.
  assert.equal(live.zero(), 42);
  assert.equal(live.sum2(2, 3), 5);
  assert.equal(live.sum10(1, 2, 3, 4, 5, 6, 7, 8, 9, 10), "1,2,3,4,5,6,7,8,9,10");
  assert.equal(live.returnsError(), typeError);

  // Every throw that is not a trap passes through as the very same value, twice, and
  // poisons nothing.
  for (let round = 0; round < 2; round += 1) {
    assert.throws(() => live.throwsType(), (error) => error === typeError);
    assert.throws(() => live.throwsHost(), (error) => error === hostError);
    assert.throws(() => live.throwsString(), (error) => error === "a thrown string");
  }
  assert.doesNotThrow(() => runtime.assertNotPoisoned());
  assert.equal(retargeted, undefined);
  assert.equal(live.sum2(4, 5), 9);

  // A trap poisons, naming it, and the call throws the poison error in its place.
  const POISON =
    "the wasm instance trapped (RuntimeError: memory access out of bounds) and cannot be used again; " +
    "load the package in a fresh JavaScript realm (a new page, Worker isolate or process)";
  assert.throws(() => live.traps(), (error) => error.constructor === Error && error.message === POISON);
  assert.throws(() => runtime.assertNotPoisoned(), { message: POISON });
  assert.notEqual(retargeted, undefined);
  assert.throws(() => retargeted.zero, { message: POISON });
  // A second, different trap keeps the first reason.
  assert.throws(() => live.trapsOther(), (error) => error.message === POISON);
});
