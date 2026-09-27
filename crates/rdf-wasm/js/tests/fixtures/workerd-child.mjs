// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Child process for the Workers yield-primitive test: run under `workerd-preload.mjs`.
// Prints one JSON line; the parent asserts on it.

import { QueryEngine, asyncYieldPrimitive, hasAsyncQueries, ready } from "../../index.mjs";
import { measureYielding } from "./yield-workload.mjs";

await ready();

// The oracle's own control: 50 ms of turns through the installed `MessageChannel` let no
// timer run, so a scheduler that yielded through it would be caught.
let starvedTicks = 0;
const interval = setInterval(() => {
  starvedTicks += 1;
}, 1);
const channel = new MessageChannel();
let starvedTurns = 0;
const until = performance.now() + 50;
while (performance.now() < until) {
  await new Promise((resolve) => {
    channel.port1.onmessage = resolve;
    channel.port2.postMessage(0);
  });
  starvedTurns += 1;
}
clearInterval(interval);

const report = {
  setImmediate: typeof globalThis.setImmediate,
  userAgent: globalThis.navigator?.userAgent,
  hasAsyncQueries: hasAsyncQueries(),
  primitive: asyncYieldPrimitive(),
  starvedTurns,
  starvedTicks,
  // Every `setTimeout` turn passes the timer phase (and Node holds each for at least a
  // millisecond), so a job needs far fewer yields than the MessageChannel tests take for
  // the timer to be seen; about 250 here.
  ...(await measureYielding(new QueryEngine(), 1000, 4096)),
};
process.stdout.write(`${JSON.stringify(report)}\n`);
