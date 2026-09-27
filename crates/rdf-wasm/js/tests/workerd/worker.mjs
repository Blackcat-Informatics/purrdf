// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// The entry module `run-worker-recipe.mjs` boots under workerd. Its imports name the
// harness's module layout, not this directory: `./workerd-recipe.mjs` is the README's
// `js worker-recipe` fence with its package specifiers pointed at the package files, and
// `./index.mjs` is the package root that recipe imports, so both share one instance.
//
// Every request goes to the recipe untouched except these harness routes, which the
// yielding proof is built from. An experiment is one lane's pair of requests, the
// observer and the long job, and its state is `{ begun, longDone }`:
//   * `/__probe` reports what the runtime gave the package: `hasAsyncQueries()`;
//   * `/__observe?lane=L` is the short request of lane L's experiment. It tells the
//     harness it is parked (one round trip through the `MARK` service binding, so the
//     harness sends the long request only once this one is waiting), waits until lane
//     L's long job has begun, then answers from one `setTimeout(…, 0)` task — an ordinary
//     event-loop task, which can run while the long job is in progress only if that job
//     gives the event loop a turn — recording whether the long job was done when it
//     answered;
//   * `/__long?lane=async&size=N` runs the pair count over N nodes on the ASYNCHRONOUS
//     lane with `yieldEveryPolls: 0`, every governor poll a yield, and reports its job
//     evidence (polls, yields);
//   * `/__long?lane=sync&size=N` runs the same count on the SYNCHRONOUS lane: the
//     control. It never turns the event loop, so the observer's task runs only after it.

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

// lane -> { begun, begin, longDone }: `begun` resolves when the lane's long job begins;
// `longDone` is `null` before it, `false` while it runs, `true` once it has finished.
const experiments = new Map();

function experiment(lane) {
  let state = experiments.get(lane);
  if (state === undefined) {
    let begin;
    const begun = new Promise((resolve) => {
      begin = resolve;
    });
    state = { begun, begin, longDone: null };
    experiments.set(lane, state);
  }
  return state;
}

export default {
  async fetch(request, env, ctx) {
    const url = new URL(request.url);
    const { pathname } = url;
    if (pathname === "/__probe") {
      return Response.json({ hasAsyncQueries: hasAsyncQueries() });
    }
    if (pathname === "/__observe") {
      const lane = url.searchParams.get("lane");
      const state = experiment(lane);
      await env.MARK.fetch(`https://mark.example.org/observer-waiting?lane=${lane}`);
      await state.begun;
      // The answer is produced by an ordinary task queued once the long job has begun.
      await new Promise((resolve) => setTimeout(resolve, 0));
      return Response.json({ lane, longDone: state.longDone });
    }
    if (pathname === "/__long") {
      const lane = url.searchParams.get("lane");
      const size = Number(url.searchParams.get("size"));
      const state = experiment(lane);
      const dataset = pairDataset(size);
      state.longDone = false;
      state.begin();
      if (lane === "sync") {
        const n = engine.select(dataset, PAIR_COUNT).rows.take(0).n.value;
        state.longDone = true;
        return Response.json({ lane, n });
      }
      const outcome = await engine.queryGovernedAsync(dataset, PAIR_COUNT, { yieldEveryPolls: 0 });
      state.longDone = true;
      if (!outcome.isComplete) {
        return Response.json({ lane, tripped: outcome.tripped }, { status: 500 });
      }
      return Response.json({ lane, n: outcome.result.rows.take(0).n.value, async: outcome.evidence.async });
    }
    return recipe.fetch(request, env, ctx);
  },
};
