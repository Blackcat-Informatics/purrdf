// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// The entry module `run-worker-recipe.mjs` boots under workerd. Its imports name the
// harness's module layout, not this directory: `./workerd-recipe.mjs` is the README's
// `js worker-recipe` fence with its package specifiers pointed at the package files, and
// `./index.mjs` is the package root that recipe imports, so both share one instance.
//
// Every request goes to the recipe untouched except these harness routes:
//   * `/__probe` reports what the runtime gave the package: `hasAsyncQueries()`,
//     `asyncYieldPrimitive()`, and the globals the primitive is chosen from;
//   * `/__async?size=N` runs the pair count on the ASYNCHRONOUS lane over N nodes and
//     reports its job evidence (polls, yields), so the log shows how many turns it took;
//   * `/__spin?primitive=P&turns=T&work=W` and `/__ping` are the yield experiment: the
//     spinner does W units of synchronous work, then awaits one turn of candidate P, T
//     times; a ping sent while it spins reports the turn it was served at (or null when
//     the spinner was not running), so the driver can tell which candidates let a
//     concurrently dispatched request in between turns;
//   * `/__sync?size=N` runs the POSTed query on the SYNCHRONOUS lane over N nodes shaped
//     as the harness loads them into the recipe's dataset. It is the control for the
//     concurrency check: it never turns the event loop, so a request sent while it runs
//     is served only after it.

import recipe from "./workerd-recipe.mjs";
import { Dataset, QueryEngine, asyncYieldPrimitive, hasAsyncQueries } from "./index.mjs";

const engine = new QueryEngine();
const controls = new Map(); // size -> Dataset

const PAIR_COUNT =
  "SELECT (COUNT(*) AS ?n) WHERE { ?a <https://example.org/v> ?x . ?b <https://example.org/v> ?y . FILTER(?x < ?y) }";

// The yield experiment's shared state: the turn the running spinner is at, or null.
let spinnerTurn = null;
const globalChannel = typeof MessageChannel === "function" ? new MessageChannel() : null;
const globalWaiting = [];
if (globalChannel !== null) {
  globalChannel.port1.onmessage = () => globalWaiting.shift()?.();
}

const CANDIDATES = {
  "MessageChannel(module)":
    globalChannel === null
      ? null
      : () =>
          new Promise((resolve) => {
            globalWaiting.push(resolve);
            globalChannel.port2.postMessage(0);
          }),
  "MessageChannel(fresh)":
    typeof MessageChannel === "function"
      ? () =>
          new Promise((resolve) => {
            const channel = new MessageChannel();
            channel.port1.onmessage = () => resolve();
            channel.port2.postMessage(0);
          })
      : null,
  "setTimeout(0)": () => new Promise((resolve) => setTimeout(resolve, 0)),
  "setTimeout(1)": () => new Promise((resolve) => setTimeout(resolve, 1)),
  "scheduler.wait(0)":
    typeof globalThis.scheduler?.wait === "function" ? () => globalThis.scheduler.wait(0) : null,
  "scheduler.yield()":
    typeof globalThis.scheduler?.yield === "function" ? () => globalThis.scheduler.yield() : null,
  setImmediate:
    typeof globalThis.setImmediate === "function"
      ? () => new Promise((resolve) => globalThis.setImmediate(resolve))
      : null,
};

/** `units` of synchronous work (a count, not a clock: workerd's clock stands still). */
function work(units) {
  let acc = 0;
  for (let index = 0; index < units * 100_000; index += 1) acc = (acc * 31 + index) | 0;
  return acc;
}

function controlDataset(size) {
  let dataset = controls.get(size);
  if (dataset === undefined) {
    const lines = Array.from(
      { length: size },
      (_, index) =>
        `<https://example.org/n${index}> <https://example.org/v> "${index}"^^<http://www.w3.org/2001/XMLSchema#integer> .`,
    );
    dataset = Dataset.parse(`${lines.join("\n")}\n`, "nquads");
    controls.set(size, dataset);
  }
  return dataset;
}

export default {
  async fetch(request, env, ctx) {
    const { pathname } = new URL(request.url);
    if (pathname === "/__probe") {
      return Response.json({
        hasAsyncQueries: hasAsyncQueries(),
        primitive: asyncYieldPrimitive() ?? null,
        typeofSetImmediate: typeof globalThis.setImmediate,
        typeofMessageChannel: typeof globalThis.MessageChannel,
        typeofSchedulerWait: typeof globalThis.scheduler?.wait,
        typeofSchedulerYield: typeof globalThis.scheduler?.yield,
        userAgent: globalThis.navigator?.userAgent ?? null,
        candidates: Object.keys(CANDIDATES).filter((name) => CANDIDATES[name] !== null),
      });
    }
    if (pathname === "/__async") {
      const size = Number(new URL(request.url).searchParams.get("size"));
      const started = performance.now();
      try {
        const outcome = await engine.queryGovernedAsync(controlDataset(size), PAIR_COUNT);
        return Response.json({ inWorkerMs: performance.now() - started, async: outcome.evidence.async });
      } catch (error) {
        return Response.json({ error: String(error) }, { status: 500 });
      }
    }
    if (pathname === "/__spin") {
      const params = new URL(request.url).searchParams;
      const turn = CANDIDATES[params.get("primitive")];
      if (turn == null) return new Response("unknown or absent primitive", { status: 404 });
      const turns = Number(params.get("turns"));
      const units = Number(params.get("work"));
      let sink = 0;
      for (let index = 0; index < turns; index += 1) {
        spinnerTurn = index;
        sink ^= work(units);
        await turn();
      }
      spinnerTurn = null;
      return Response.json({ turns, sink });
    }
    if (pathname === "/__ping") {
      return Response.json({ servedAtTurn: spinnerTurn });
    }
    if (pathname === "/__sync") {
      const size = Number(new URL(request.url).searchParams.get("size"));
      const text = await request.text();
      const result = engine.select(controlDataset(size), text);
      return Response.json({ n: result.rows.take(0).n.value });
    }
    return recipe.fetch(request, env, ctx);
  },
};
