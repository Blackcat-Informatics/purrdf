// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Node-side measurement of the poison gate's per-call cost.
//
// Report-only (never a gate). Every exported function of the shipped module runs behind
// the gate the post-link step (`crates/wasm-link`) injects: on entry a read of the
// poisoned flag, a read-subtract-compare over the active, parked and outbound counters,
// and an increment of the active counter; on exit a decrement. This drives the most
// trivial gated export the module has, `__wbindgen_add_to_stack_pointer(0)` — the
// guarded body is five instructions, so what is measured is almost entirely the
// JavaScript-to-wasm call and the gate — in a tight loop, and prints the absolute cost
// per call in nanoseconds.
//
// The ungated baseline is not available at runtime: the shipped module has no export
// outside the gate, and the package is built once. So this number is read against a
// prior run to notice a change in the gate's cost, never compared against an ungated
// call and never asserted. The two things it does assert are correctness: the counters
// balance after the loop, and the stack pointer is where it started.
//
// Usage:
//   node bench/gate.bench.mjs
// Tunables (env):
//   BENCH_GATE_ITERS   calls per timed round        (default 5000000)
//   BENCH_GATE_ROUNDS  timed rounds after warm-up   (default 5)

import { ready } from "../index.mjs";
import init from "../pkg/purrdf_wasm.js";

await ready();
// Already instantiated: `init` hands back the one instance's raw exports.
const exports = await init();

const ITERS = Number(process.env.BENCH_GATE_ITERS ?? 5_000_000);
const ROUNDS = Number(process.env.BENCH_GATE_ROUNDS ?? 5);
const WARMUP_CALLS = 200_000;

for (const name of ["purrdf_poisoned", "purrdf_active", "purrdf_parked", "purrdf_outbound", "purrdf_stack_pointer"]) {
  if (!(exports[name] instanceof WebAssembly.Global)) {
    throw new Error(`the package does not export ${name}; it was not linked by wasm-link (build it with make wasm-pkg)`);
  }
}
if (typeof exports.__wbindgen_add_to_stack_pointer !== "function") {
  throw new Error("the package does not export __wbindgen_add_to_stack_pointer");
}

const gated = exports.__wbindgen_add_to_stack_pointer;
const stackPointerBefore = exports.purrdf_stack_pointer.value;

function median(xs) {
  const s = [...xs].sort((a, b) => a - b);
  const mid = s.length >> 1;
  return s.length % 2 ? s[mid] : (s[mid - 1] + s[mid]) / 2;
}

for (let i = 0; i < WARMUP_CALLS; i++) gated(0);

const nsPerCall = [];
for (let round = 0; round < ROUNDS; round++) {
  const start = process.hrtime.bigint();
  for (let i = 0; i < ITERS; i++) gated(0);
  const elapsedNs = Number(process.hrtime.bigint() - start);
  nsPerCall.push(elapsedNs / ITERS);
}

const active = exports.purrdf_active.value;
const parked = exports.purrdf_parked.value;
const outbound = exports.purrdf_outbound.value;
const poisoned = exports.purrdf_poisoned.value;
if (poisoned !== 0 || active - parked !== outbound) {
  throw new Error(
    `the gate's counters do not balance after ${ROUNDS * ITERS + WARMUP_CALLS} calls: ` +
      `poisoned=${poisoned} active=${active} parked=${parked} outbound=${outbound}`,
  );
}
if (exports.purrdf_stack_pointer.value !== stackPointerBefore) {
  throw new Error(
    `the stack pointer moved from ${stackPointerBefore} to ${exports.purrdf_stack_pointer.value} across gated calls`,
  );
}

console.log(
  `gate.bench (report-only): __wbindgen_add_to_stack_pointer(0) through the poison gate, ` +
    `${ROUNDS} rounds x ${ITERS} calls: median ${median(nsPerCall).toFixed(2)} ns/call ` +
    `(min ${Math.min(...nsPerCall).toFixed(2)}, max ${Math.max(...nsPerCall).toFixed(2)}); ` +
    `counters balanced (active=${active} parked=${parked} outbound=${outbound}), stack pointer unchanged`,
);
