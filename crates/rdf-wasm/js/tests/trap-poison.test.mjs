// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// A trap that is not a panic, out of a SYNCHRONOUS call, poisons the instance — exactly
// as a trap out of an asynchronous job and a panic in any call do. The poison gate is
// linked into the module itself (`crates/wasm-link`): after the trap every entry into the
// instance traps at its gate, synchronous calls and objects created before the trap
// included, while the asynchronous twins and `ready()` reject with the poison error the
// runtime builds from the gate's globals. The real trap runs in a child process
// (`fixtures/sync-trap-child.mjs`), because poisoning is permanent for the JavaScript
// realm. The runtime's reading of the gate is also exercised on a fresh instance of the
// runtime module over a hand-built module of the linked shape, where every state of the
// globals can be set on demand.

import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import * as packageRoot from "../index.mjs";
import { assertSurfacePoisoned, threwAtGate } from "./fixtures/poisoned-surface.mjs";

const EX = "http://example.org/";

// The reason the runtime names when it learns of the poisoning from the gate's globals
// rather than from an error in its hands.
const GATE_CLOSED_REASON =
  "a call into the instance was unwound by a trap or a thrown exception, and its poison gate closed";
const poisonMessage = (reason) =>
  `the wasm instance trapped (${reason}) and cannot be used again; ` +
  "load the package in a fresh JavaScript realm (a new page, Worker isolate or process)";

test("a trap that is not a panic, in a synchronous call, poisons every entry point", () => {
  const child = spawnSync(process.execPath, [fileURLToPath(new URL("./fixtures/sync-trap-child.mjs", import.meta.url))], {
    encoding: "utf8",
    timeout: 120_000,
  });
  assert.equal(child.status, 0, `the child exited ${child.status}: ${child.stderr}`);
  const report = JSON.parse(child.stdout.trim());

  // The valid neighbours, on the very objects the trap later poisons: the unarmed export
  // does nothing, typed errors are thrown again (not the poison) on the second call, a
  // job started from inside a synchronous sink callback answers, and afterwards every
  // lane — the asynchronous one included — answers exactly.
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
  assert.deepEqual(
    report.asyncFromCallback,
    { settled: "resolved", subjects: [`${EX}a`, `${EX}b`, `${EX}c`] },
    "a job started from inside a serializeToSink callback answers",
  );
  assert.equal(report.serializedInCallback, true, "the callback's own serialization finished intact");
  assert.deepEqual(report.gateBefore, { poisoned: 0, balanced: true }, "the gate is open and balanced before the trap");
  assert.deepEqual(report.sizeBefore, { settled: "returned", value: 3 });
  assert.equal(report.versionBefore.settled, "returned");

  // The trapping call throws the trap itself: no JavaScript stands between the caller and
  // the instance, and no panic ran.
  assert.deepEqual(report.trapped, { settled: "threw", name: "RuntimeError", message: "unreachable" });

  // The runtime learns of the trap at the next call it takes part in, from the gate's
  // globals: that call, the job that was in flight and every later asynchronous call —
  // `ready()` included — reject with the poison error naming the gate.
  const REJECTED = { settled: "rejected", name: "Error", message: poisonMessage(GATE_CLOSED_REASON) };
  for (const name of ["asyncAfter", "inFlight", "readyAfter"]) {
    assert.deepEqual(report[name], REJECTED, name);
  }
  // The gate itself: the trapped entry never returned, so the counters no longer balance,
  // and the first entry after it marked the instance poisoned.
  assert.deepEqual(report.gateAfter, { poisoned: 1, balanced: false });

  // Every synchronous entry into the instance — on objects created before the trap, new
  // objects, statics, free functions — traps at the gate.
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
    assert.deepEqual(report[name], threwAtGate, name);
  }
  // Releasing is the one entry that returns: the release exports' gate is inert on a
  // poisoned instance, so `free()` and `[Symbol.dispose]()` return without entering it
  // (the memory is abandoned whole, and a finalizer has no caller to report to).
  assert.deepEqual(report.freeAfter, { settled: "returned" });
  assert.ok(
    report.disposeAfter.settled === "returned" || report.disposeAfter.settled === "absent",
    `[Symbol.dispose]() on the poisoned instance: ${JSON.stringify(report.disposeAfter)}`,
  );
  assertSurfacePoisoned(report.surface, packageRoot, poisonMessage(GATE_CLOSED_REASON));
});

// The exports the linker adds to the module, in the order the hand-built module below
// defines its globals.
const GATE_GLOBALS = [
  "purrdf_stack_pointer",
  "purrdf_idle",
  "purrdf_poisoned",
  "purrdf_active",
  "purrdf_parked",
  "purrdf_outbound",
];

/**
 * A wasm module of the linked shape the runtime installs over: a memory, an exported
 * function `purrdf_jspi_run (i32, i32) -> i32` (a real wasm function, which
 * `WebAssembly.promising` requires) and one mutable `i32` global per linker export, each
 * exported under its name except `omit`.
 */
function linkedShapeModule({ omit } = {}) {
  const encoder = new TextEncoder();
  const leb = (value) => {
    const out = [];
    let rest = value;
    do {
      let byte = rest & 0x7f;
      rest >>>= 7;
      if (rest !== 0) byte |= 0x80;
      out.push(byte);
    } while (rest !== 0);
    return out;
  };
  const vec = (items) => [...leb(items.length), ...items.flat()];
  const name = (text) => {
    const bytes = [...encoder.encode(text)];
    return [...leb(bytes.length), ...bytes];
  };
  const section = (id, body) => [id, ...leb(body.length), ...body];
  const I32 = 0x7f;
  const types = section(1, vec([[0x60, ...vec([[I32], [I32]]), ...vec([[I32]])]]));
  const functions = section(3, vec([[0]]));
  const memories = section(5, vec([[0x00, 0x01]]));
  // Each global: i32, mutable, initialized by `i32.const 0; end`.
  const globals = section(6, vec(GATE_GLOBALS.map(() => [I32, 0x01, 0x41, 0x00, 0x0b])));
  const exported = section(
    7,
    vec([
      [...name("memory"), 0x02, 0x00],
      [...name("purrdf_jspi_run"), 0x00, 0x00],
      ...GATE_GLOBALS.filter((global) => global !== omit).map((global) => [
        ...name(global),
        0x03,
        ...leb(GATE_GLOBALS.indexOf(global)),
      ]),
    ]),
  );
  // One body: no locals; `local.get 0; end`.
  const code = section(10, vec([[...leb(4), 0x00, 0x20, 0x00, 0x0b]]));
  return new Uint8Array([
    0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
    ...types, ...functions, ...memories, ...globals, ...exported, ...code,
  ]);
}

const PROTOCOL = { AsyncJob: class AsyncJob {}, RunStatus: {}, SuspendStatus: {}, DeliveryStatus: {}, EffectKind: {} };

const instantiate = (options) => new WebAssembly.Instance(new WebAssembly.Module(linkedShapeModule(options))).exports;

test("installAsync requires every linker global by name, and the runtime reads the gate as the linker defines it", async () => {
  // A module instance of its own: this one's poisoning reaches nothing else.
  const runtime = await import(`../src/purrdf_jspi.mjs?gate-unit-${process.pid}`);

  // Each global missing in turn: refused, naming it, and nothing is installed.
  for (const omit of GATE_GLOBALS) {
    const exports = instantiate({ omit });
    assert.equal(exports[omit], undefined);
    assert.throws(
      () => runtime.installAsync(exports, PROTOCOL),
      { message: new RegExp(`does not export the WebAssembly.Global ${omit}; .*wasm-link`) },
      omit,
    );
  }
  // The neighbour: every global present installs.
  const exports = instantiate();
  runtime.installAsync(exports, PROTOCOL);
  assert.equal(runtime.hasAsyncQueries(), true);
  assert.doesNotThrow(() => runtime.assertNotPoisoned());

  // Balanced shapes the gate's invariant allows are not a poisoning: a call from inside
  // an import (a sink callback, the glue's allocator) — active 1, outbound 1 — and a job
  // parked in the suspending import — active 1, parked 1.
  exports.purrdf_active.value = 1;
  exports.purrdf_outbound.value = 1;
  assert.doesNotThrow(() => runtime.assertNotPoisoned(), "an entry waiting inside an import call");
  exports.purrdf_outbound.value = 0;
  exports.purrdf_parked.value = 1;
  assert.doesNotThrow(() => runtime.assertNotPoisoned(), "an entry parked in the suspending import");
  exports.purrdf_parked.value = 0;
  exports.purrdf_active.value = 0;
  assert.doesNotThrow(() => runtime.assertNotPoisoned());
  assert.equal(exports.purrdf_poisoned.value, 0, "reading the gate marks nothing");

  // An entry that never returned — active 1, nothing parked, nothing outbound — is the
  // gate's condition: the runtime poisons, naming the gate, and sets the flag the gate
  // traps on.
  exports.purrdf_active.value = 1;
  const POISON = poisonMessage(GATE_CLOSED_REASON);
  assert.throws(() => runtime.assertNotPoisoned(), { constructor: Error, message: POISON });
  assert.equal(exports.purrdf_poisoned.value, 1, "the runtime wrote the poisoned flag");
  // Poisoning is permanent: counters put back in balance change nothing.
  exports.purrdf_active.value = 0;
  assert.throws(() => runtime.assertNotPoisoned(), { message: POISON });
  assert.throws(() => runtime.assertAsyncQueries(), { message: POISON });

  // A second instance of the runtime over a module the gate already marked: the flag
  // alone, with balanced counters, is the poisoning.
  const marked = await import(`../src/purrdf_jspi.mjs?gate-unit-marked-${process.pid}`);
  const markedExports = instantiate();
  marked.installAsync(markedExports, PROTOCOL);
  assert.doesNotThrow(() => marked.assertNotPoisoned());
  markedExports.purrdf_poisoned.value = 1;
  assert.throws(() => marked.assertNotPoisoned(), { message: POISON });
});
