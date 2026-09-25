// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Child process for the trap-poisoning test. Run with
// `--wasm-stack-switching-stack-size=32`: V8 runs every JSPI-promised call on a
// secondary native stack of that many KiB (984 by default), while synchronous calls keep
// the full main stack. The parser recurses once per nested `{ … }` and refuses nesting
// deeper than 128 with its own typed error; at 129 levels, the recursion plus that
// refusal's path needs more native stack than 32 KiB, so the asynchronous run overflows
// V8's stack — a genuine `RangeError` trap out of the promising call — while the
// synchronous run of the very same query reports the parser's own refusal. (Measured on
// the shipped artifact: 128 levels still answer asynchronously on a 32 KiB stack, and
// 129 trap; the same 129-level query refuses synchronously on the main stack.)
//
// Prints one JSON line; the parent test asserts on it.

import { Dataset, QueryEngine, ready } from "../../index.mjs";

await ready();

const EX = "http://example.org/";
const data = Dataset.parse(`<${EX}a> <${EX}p> <${EX}o> .\n`, "nquads");
const engine = new QueryEngine();
const SHALLOW = `SELECT ?s WHERE { ?s <${EX}p> ?o }`;

function nested(depth) {
  let pattern = `?s <${EX}p> ?o`;
  for (let level = 0; level < depth; level += 1) pattern = `{ ${pattern} }`;
  return `SELECT ?s WHERE ${pattern}`;
}
const TOO_DEEP = nested(129);

const describe = (error) => ({ name: error?.name, message: error?.message });
async function settle(promise) {
  try {
    const value = await promise;
    return { settled: "resolved", rowCount: value?.rowCount };
  } catch (error) {
    return { settled: "rejected", ...describe(error) };
  }
}

const report = {};

// Before the trap: an ordinary asynchronous query answers on the small secondary stack,
// and the deep one refuses synchronously with the parser's own error — not a trap.
report.asyncBefore = await settle(engine.queryAsync(data, SHALLOW));
try {
  engine.query(data, TOO_DEEP);
  report.syncDeep = { settled: "resolved" };
} catch (error) {
  report.syncDeep = { settled: "rejected", ...describe(error) };
}
report.syncDeepAgain = (() => {
  try {
    engine.query(data, TOO_DEEP);
    return { settled: "resolved" };
  } catch (error) {
    return { settled: "rejected", ...describe(error) };
  }
})();

// A job in flight — suspended on a SERVICE the host never answers — when another traps.
let serviceAsked;
const asked = new Promise((resolve) => {
  serviceAsked = resolve;
});
const inFlight = settle(
  engine.queryAsync(data, `SELECT ?x WHERE { SERVICE <${EX}sparql> { ?s ?p ?x } }`, {
    resolveService: () => {
      serviceAsked();
      return new Promise(() => {});
    },
  }),
);
await asked;

// The trap.
report.trapped = await settle(engine.queryAsync(data, TOO_DEEP));
report.inFlight = await inFlight;
// Every later asynchronous call refuses with the same poison.
report.asyncAfter = await settle(engine.queryAsync(data, SHALLOW));
report.updateAfter = await settle(engine.updateAsync(data, `INSERT DATA { <${EX}b> <${EX}p> <${EX}o> }`));

process.stdout.write(`${JSON.stringify(report)}\n`);
// The in-flight job's host promise never settles; nothing else is pending.
process.exit(0);
