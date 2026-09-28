// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// The entry module `run-worker-recipe.mjs` boots under workerd. Its imports name the
// harness's module layout, not this directory: `./workerd-recipe.mjs` is the README's
// `js worker-recipe` fence with its package specifiers pointed at the package files, and
// `./index.mjs` is the package root that recipe imports, so both share one instance.
//
// Every request goes to the recipe untouched except these harness routes, which the
// yielding proof is built from:
//   * `/__probe` reports what the runtime gave the package: `hasAsyncQueries()`;
//   * `/__yield?lane=L&size=N` is one lane's whole experiment, inside ONE request: it
//     queues an observer — one `setTimeout(…, 0)` task, an ordinary event-loop task that
//     records whether the long job is done when it runs — and only then starts the long
//     job, the pair count over N nodes. On the ASYNCHRONOUS lane (`lane=async`) the job
//     runs with `yieldEveryPolls: 0`, every governor poll a yield, and each yield is a
//     `setTimeout(…, 0)` task queued behind the observer's, so the observer runs at the
//     job's first yield, before the job finishes. On the SYNCHRONOUS lane (`lane=sync`,
//     the control) the count never turns the event loop, so the observer runs only after
//     it returns. The response carries the count, the job's evidence (polls, yields) and
//     what the observer recorded.
//
// Nothing in one request waits on another request's promise: workerd cancels a request
// whose code awaits I/O or a promise owned by a different request's context ("the Workers
// runtime canceled this request because it detected that your Worker's code had hung").
// The observer and the job it observes therefore share one request, and their order is
// caused by the timer queue alone — no clock is read and nothing is calibrated.

import recipe from "./workerd-recipe.mjs";
import { Dataset, QueryEngine, hasAsyncQueries } from "./index.mjs";

const engine = new QueryEngine();
const datasets = new Map(); // size -> Dataset

/**
 * A quadratic self-join folded to one row, size·(size−1)/2. Every candidate pair is
 * governor work, so the job polls many times while it runs.
 */
const PAIR_COUNT =
  "SELECT (COUNT(*) AS ?n) WHERE { ?a <https://example.org/v> ?x . ?b <https://example.org/v> ?y . FILTER(?x < ?y) }";

/** A dataset of `size` integer-valued nodes: the operand of `PAIR_COUNT`. */
function pairDataset(size) {
  let dataset = datasets.get(size);
  if (dataset === undefined) {
    const lines = Array.from(
      { length: size },
      (_, index) =>
        `<https://example.org/n${index}> <https://example.org/v> "${index}"^^<http://www.w3.org/2001/XMLSchema#integer> .`,
    );
    dataset = Dataset.parse(`${lines.join("\n")}\n`, "nquads");
    datasets.set(size, dataset);
  }
  return dataset;
}

export default {
  async fetch(request, env, ctx) {
    const url = new URL(request.url);
    const { pathname } = url;
    if (pathname === "/__probe") {
      return Response.json({ hasAsyncQueries: hasAsyncQueries() });
    }
    if (pathname === "/__yield") {
      const lane = url.searchParams.get("lane");
      const dataset = pairDataset(Number(url.searchParams.get("size")));
      // `null` until the long job starts, `false` while it runs, `true` once it finished.
      let longDone = null;
      // The observer: queued before the job starts, so ahead of every task the job queues.
      const observed = new Promise((resolve) => {
        setTimeout(() => resolve({ lane, longDone }), 0);
      });
      longDone = false;
      if (lane === "sync") {
        const n = engine.select(dataset, PAIR_COUNT).rows.take(0).n.value;
        longDone = true;
        return Response.json({ lane, n, observed: await observed });
      }
      const outcome = await engine.queryGovernedAsync(dataset, PAIR_COUNT, { yieldEveryPolls: 0 });
      longDone = true;
      if (!outcome.isComplete) {
        return Response.json({ lane, tripped: outcome.tripped, observed: await observed }, { status: 500 });
      }
      return Response.json({
        lane,
        n: outcome.result.rows.take(0).n.value,
        async: outcome.evidence.async,
        observed: await observed,
      });
    }
    return recipe.fetch(request, env, ctx);
  },
};
