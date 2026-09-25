// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// The event-loop yielding workload, shared by the in-process test and the child that
// runs it with only `MessageChannel` to yield through, so both assert the same thing.

import { Dataset } from "../../index.mjs";

const EX = "http://example.org/";

/** A dataset of `size` integer-valued nodes: the operand of a quadratic self-join. */
export function crossDataset(size) {
  const lines = [];
  for (let index = 0; index < size; index += 1) {
    lines.push(`<${EX}n${index}> <${EX}v> "${index}"^^<http://www.w3.org/2001/XMLSchema#integer> .`);
  }
  return Dataset.parse(`${lines.join("\n")}\n`, "nquads");
}

/**
 * A cross product folded to one row: about a million candidate pairs at 1 000 nodes,
 * which runs for hundreds of milliseconds synchronously, so an event loop that is never
 * turned while it runs is plain to see.
 */
export const CROSS_COUNT = `SELECT (COUNT(*) AS ?c) WHERE { ?a <${EX}v> ?x . ?b <${EX}v> ?y . FILTER(?x < ?y) }`;

/** The answer `CROSS_COUNT` must give over `crossDataset(size)`. */
export function expectedCrossCount(size) {
  return String((size * (size - 1)) / 2);
}

/**
 * Run `CROSS_COUNT` once synchronously and once through `queryGovernedAsync` with
 * `yieldEveryPolls: 1024`, counting the ticks of a 5 ms interval during each.
 */
export async function measureYielding(engine, size) {
  const dataset = crossDataset(size);
  let ticks = 0;
  const interval = setInterval(() => {
    ticks += 1;
  }, 5);
  try {
    const sync = engine.select(dataset, CROSS_COUNT);
    const syncCount = sync.rows.take(0).c.value;
    const syncTicks = ticks;
    ticks = 0;
    const outcome = await engine.queryGovernedAsync(dataset, CROSS_COUNT, { yieldEveryPolls: 1024 });
    const asyncTicks = ticks;
    return {
      syncCount,
      syncTicks,
      asyncComplete: outcome.isComplete,
      asyncCount: outcome.isComplete ? outcome.result.rows.take(0).c.value : undefined,
      asyncTicks,
      yields: outcome.evidence.async.yields,
      polls: outcome.evidence.async.polls,
    };
  } finally {
    clearInterval(interval);
    dataset.free();
  }
}
