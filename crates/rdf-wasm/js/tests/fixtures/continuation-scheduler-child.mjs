// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Child process for the continuation-priority test: run under
// `continuation-scheduler-preload.mjs`. Prints one JSON line; the parent asserts on it.

import { QueryEngine, asyncYieldPrimitive, hasAsyncQueries, ready } from "../../index.mjs";
import { CROSS_COUNT, crossDataset } from "./yield-workload.mjs";

await ready();

// Count the ticks of a 1 ms interval while `body` runs.
async function ticksDuring(body) {
  let ticks = 0;
  const interval = setInterval(() => {
    ticks += 1;
  }, 1);
  try {
    const value = await body();
    return { ticks, value };
  } finally {
    clearInterval(interval);
  }
}

// The oracle's own control: yielding through the installed `scheduler.yield` for 50 ms
// turns no timer at all, so a scheduler that yielded through it would be caught.
const starved = await ticksDuring(async () => {
  const until = performance.now() + 50;
  let turns = 0;
  while (performance.now() < until) {
    await globalThis.scheduler.yield();
    turns += 1;
  }
  return turns;
});

const engine = new QueryEngine();
const dataset = crossDataset(1000);
try {
  const sync = await ticksDuring(async () => engine.query(dataset, CROSS_COUNT).rows.take(0).c.value);
  const asyncRun = await ticksDuring(async () => {
    // Node delivers up to a thousand queued MessagePort messages per event-loop turn, so
    // yield often enough that the timer phase comes round many times.
    const result = await engine.queryAsync(dataset, CROSS_COUNT, { yieldEveryPolls: 64 });
    return result.rows.take(0).c.value;
  });
  process.stdout.write(
    `${JSON.stringify({
      setImmediate: typeof globalThis.setImmediate,
      scheduler: typeof globalThis.scheduler?.yield,
      hasAsyncQueries: hasAsyncQueries(),
      primitive: asyncYieldPrimitive(),
      schedulerYieldTurns: starved.value,
      schedulerYieldTicks: starved.ticks,
      syncCount: sync.value,
      syncTicks: sync.ticks,
      asyncCount: asyncRun.value,
      asyncTicks: asyncRun.ticks,
    })}\n`,
  );
} finally {
  dataset.free();
}
