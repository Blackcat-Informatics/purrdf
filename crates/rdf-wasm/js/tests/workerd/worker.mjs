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
    if (pathname === "/__sync") {
      const size = Number(new URL(request.url).searchParams.get("size"));
      const text = await request.text();
      const result = engine.select(controlDataset(size), text);
      return Response.json({ n: result.rows.take(0).n.value });
    }
    return recipe.fetch(request, env, ctx);
  },
};
