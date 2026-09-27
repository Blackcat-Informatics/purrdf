// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// The README's Worker recipe, run under real workerd through Miniflare. The recipe (the
// `js worker-recipe` fence) is taken from `../../README.md` verbatim; the only rewrite is
// the one a bundler performs, pointing its package specifiers at this package's files.
// It is served from the built package in `../../pkg/`, so run `make wasm-pkg` first.
//
// Nothing here reaches the network. The recipe's `REMOTE` service binding is answered
// in this process, and every other outbound `fetch` is answered here too and recorded,
// so a request that escaped the catalog would fail the run rather than leave the
// machine. Only example.org names appear.
//
// It needs Miniflare, which downloads a workerd binary, so it is not an `npm test` file
// (`tests/*.test.mjs` does not match it) and the package declares no dependency on it.
// CI installs an exactly pinned Miniflare into a scratch directory and names that
// directory in PURRDF_MINIFLARE_DIR; without it this exits non-zero rather than
// skipping, so the CI step cannot pass without having run.
//
// Asserted:
//   * the runtime gives the package JSPI and the yield primitive a Worker needs,
//     `setTimeout` (recorded in the log and the job summary, beside a table of which
//     candidate primitives let a concurrent request in between turns);
//   * a SERVICE the catalog admits is joined: 200 and SPARQL Results JSON;
//   * SERVICE SILENT over an endpoint that answered 500 is the join identity, and the
//     endpoint was really asked;
//   * a SERVICE the catalog does not admit is 403 `native-sparql-service-denied`, and
//     nothing was sent;
//   * an endpoint that answers 500 is 502 `native-sparql-service-failed`;
//   * while a long query runs, a second request is served before it finishes — and, as
//     the control, not while the same query runs on the synchronous lane.

import assert from "node:assert/strict";
import { appendFileSync, readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const JS_ROOT = resolve(fileURLToPath(new URL("../../", import.meta.url)));
const WORKER = "https://worker.example.org";
const REMOTE = "https://remote.example.org/sparql";
const DENIED = "https://denied.example.org/sparql";
const RUN_TIMEOUT_MS = 300_000;

const QUERY = `SELECT ?s ?x WHERE { ?s <https://example.org/p> ?o SERVICE <${REMOTE}> { ?o <https://example.org/q> ?x } }`;
// The upstream answers any query naming this predicate with a 500.
const FAILING = QUERY.replace("<https://example.org/q>", "<https://example.org/fail>");
const SILENT_FAILING = FAILING.replace("SERVICE <", "SERVICE SILENT <");
const DENIED_QUERY = QUERY.replace(REMOTE, DENIED);
const SHORT = "SELECT ?s WHERE { ?s <https://example.org/p> ?o }";

const watchdog = setTimeout(() => {
  console.error(`the workerd Worker-recipe run did not finish within ${RUN_TIMEOUT_MS} ms`);
  process.exit(1);
}, RUN_TIMEOUT_MS);
watchdog.unref();

/** Miniflare, resolved from the scratch directory CI installed it into. */
async function loadMiniflare() {
  const dir = process.env.PURRDF_MINIFLARE_DIR;
  if (!dir) {
    throw new Error(
      "PURRDF_MINIFLARE_DIR must name a directory with miniflare installed in its node_modules",
    );
  }
  const require = createRequire(join(resolve(dir), "package.json"));
  const mod = await import(pathToFileURL(require.resolve("miniflare")).href);
  const Miniflare = mod.Miniflare ?? mod.default?.Miniflare;
  const Response = mod.Response ?? mod.default?.Response ?? globalThis.Response;
  assert.equal(typeof Miniflare, "function", "miniflare exports Miniflare");
  return { Miniflare, Response };
}

/** The README's Worker recipe, with its package specifiers pointed at the package files. */
function workerRecipe() {
  const readme = readFileSync(join(JS_ROOT, "README.md"), "utf8");
  const found = [...readme.matchAll(/^```js worker-recipe\n([\s\S]*?)^```$/gm)].map((m) => m[1]);
  assert.equal(found.length, 1, `README.md must carry one js worker-recipe fence (found ${found.length})`);
  const source = found[0]
    .replaceAll('"@blackcatinformatics/purrdf/purrdf_wasm_bg.wasm"', '"./pkg/purrdf_wasm_bg.wasm"')
    .replaceAll('"@blackcatinformatics/purrdf/cloudflare"', '"./cloudflare.mjs"')
    .replaceAll('"@blackcatinformatics/purrdf"', '"./index.mjs"');
  assert.ok(!source.includes("@blackcatinformatics/"), "every package specifier was rewritten");
  return source;
}

/** One module of the Worker, named by its path under the package root. */
function esModule(name, contents) {
  return {
    type: "ESModule",
    path: join(JS_ROOT, name),
    contents: contents ?? readFileSync(join(JS_ROOT, name), "utf8"),
  };
}

/** SPARQL Results JSON joining `?o` (the local object) to `?x`. */
function remoteAnswer() {
  return JSON.stringify({
    head: { vars: ["o", "x"] },
    results: {
      bindings: [
        {
          o: { type: "uri", value: "https://example.org/o1" },
          x: { type: "literal", value: "joined" },
        },
      ],
    },
  });
}

/**
 * The update that makes the recipe's dataset hold `size` integer-valued nodes (and
 * nothing else under `<https://example.org/v>`): the operand of `PAIR_COUNT`.
 */
function pairData(size) {
  const lines = Array.from(
    { length: size },
    (_, index) => `<https://example.org/n${index}> <https://example.org/v> ${index} .`,
  );
  return `DELETE WHERE { ?s <https://example.org/v> ?o } ; INSERT DATA { ${lines.join("\n")} }`;
}

/**
 * A quadratic self-join folded to one row, size·(size−1)/2. Every candidate pair is a
 * governor poll, so the job yields many times while it runs; a `VALUES` cross product
 * of the same size polls a few hundred times in all and would not yield once.
 */
const PAIR_COUNT =
  "SELECT (COUNT(*) AS ?n) WHERE { ?a <https://example.org/v> ?x . ?b <https://example.org/v> ?y . FILTER(?x < ?y) }";

const sleep = (ms) => new Promise((done) => setTimeout(done, ms));

/** Diagnostics, printed before the assertions they explain. */
function diagnostics(lines) {
  for (const line of lines) console.log(`[workerd diagnostics] ${line}`);
  if (process.env.GITHUB_STEP_SUMMARY) {
    appendFileSync(process.env.GITHUB_STEP_SUMMARY, `${lines.map((line) => `- ${line}`).join("\n")}\n`);
  }
}

/**
 * For each candidate yield primitive the Worker has, whether a request dispatched while
 * a spinner awaits that primitive between chunks of synchronous work is served before
 * the spinner finishes, and at which of its turns.
 */
async function yieldExperiment(mf, candidates) {
  const turns = 100;
  const units = 20;
  const rows = [`yield experiment (${turns} turns of ${units} work units each):`];
  for (const name of candidates) {
    const started = performance.now();
    const spinQuery = new URLSearchParams({ primitive: name, turns: String(turns), work: String(units) });
    let spinDone = null;
    let pingDone = null;
    const spin = mf.dispatchFetch(`${WORKER}/__spin?${spinQuery}`).then(async (response) => {
      await response.text();
      spinDone = performance.now() - started;
    });
    await sleep(30);
    const ping = mf.dispatchFetch(`${WORKER}/__ping`).then(async (response) => {
      const body = await response.json();
      pingDone = performance.now() - started;
      return body;
    });
    const [, pinged] = await Promise.all([spin, ping]);
    rows.push(
      `  ${name.padEnd(24)} spin ${Math.round(spinDone)} ms, ping answered at ${Math.round(pingDone)} ms, ` +
        `servedAtTurn ${pinged.servedAtTurn}, interleaved ${pingDone < spinDone && pinged.servedAtTurn !== null}`,
    );
  }
  return rows;
}

function summary(lines) {
  console.log(lines.join("\n"));
  if (process.env.GITHUB_STEP_SUMMARY) {
    appendFileSync(process.env.GITHUB_STEP_SUMMARY, `${lines.map((line) => `- ${line}`).join("\n")}\n`);
  }
}

async function main() {
  const { Miniflare, Response } = await loadMiniflare();
  const upstream = [];
  const stray = [];
  const mf = new Miniflare({
    modulesRoot: JS_ROOT,
    modules: [
      esModule("workerd-entry.mjs", readFileSync(join(JS_ROOT, "tests/workerd/worker.mjs"), "utf8")),
      esModule("workerd-recipe.mjs", workerRecipe()),
      esModule("index.mjs"),
      esModule("cloudflare.mjs"),
      esModule("pkg/purrdf_wasm.js"),
      esModule("pkg/purrdf_jspi.mjs"),
      {
        type: "CompiledWasm",
        path: join(JS_ROOT, "pkg/purrdf_wasm_bg.wasm"),
        contents: readFileSync(join(JS_ROOT, "pkg/purrdf_wasm_bg.wasm")),
      },
    ],
    compatibilityDate: "2026-07-01",
    serviceBindings: {
      async REMOTE(request) {
        const body = await request.text();
        upstream.push({
          url: request.url,
          method: request.method,
          contentType: request.headers.get("Content-Type"),
          body,
        });
        if (body.includes("<https://example.org/fail>")) {
          return new Response("the example.org upstream is failing", { status: 500 });
        }
        return new Response(remoteAnswer(), {
          headers: { "Content-Type": "application/sparql-results+json" },
        });
      },
    },
    outboundService(request) {
      stray.push(request.url);
      return new Response("this harness has no network", { status: 599 });
    },
  });

  const post = (query, path = "/sparql", contentType = "application/sparql-query") =>
    mf.dispatchFetch(`${WORKER}${path}`, {
      method: "POST",
      headers: { "Content-Type": contentType, Accept: "application/sparql-results+json" },
      body: query,
    });
  const rowsOf = async (response) =>
    JSON.parse(await response.text()).results.bindings.map((row) => [row.s?.value, row.x?.value]);
  const problemOf = async (response) => {
    assert.match(response.headers.get("Content-Type"), /^application\/problem\+json/);
    return JSON.parse(await response.text());
  };

  try {
    // The runtime gives the package what the asynchronous lane needs.
    const probe = await (await mf.dispatchFetch(`${WORKER}/__probe`)).json();
    diagnostics([
      `probe: hasAsyncQueries() = ${probe.hasAsyncQueries}, asyncYieldPrimitive() = ${probe.primitive}`,
      `probe: typeof setImmediate = ${probe.typeofSetImmediate}, typeof MessageChannel = ${probe.typeofMessageChannel}, ` +
        `typeof scheduler.wait = ${probe.typeofSchedulerWait}, typeof scheduler.yield = ${probe.typeofSchedulerYield}`,
      `probe: navigator.userAgent = ${JSON.stringify(probe.userAgent)}`,
    ]);
    diagnostics(await yieldExperiment(mf, probe.candidates));
    assert.equal(probe.hasAsyncQueries, true, "workerd provides JSPI and a yield primitive");
    // workerd delivers a MessageChannel message without letting another request in, so
    // a Worker must yield through the timer (the experiment above shows both).
    assert.equal(probe.userAgent, "Cloudflare-Workers", "workerd reports the documented Workers user agent");
    assert.equal(probe.primitive, "setTimeout", "a Worker yields through setTimeout");

    // A SERVICE the catalog admits is joined.
    const joined = await post(QUERY);
    assert.equal(joined.status, 200);
    assert.match(joined.headers.get("Content-Type"), /^application\/sparql-results\+json/);
    assert.deepEqual(await rowsOf(joined), [["https://example.org/a", "joined"]]);
    assert.equal(upstream.length, 1, "the admitted endpoint was asked once");
    assert.equal(upstream[0].url, REMOTE);
    assert.equal(upstream[0].method, "POST");
    assert.equal(upstream[0].contentType, "application/sparql-query");
    assert.match(upstream[0].body, /<https:\/\/example\.org\/q>/);

    // SERVICE SILENT over an endpoint that failed: the local row, unextended — which
    // differs from the joined row above — and the endpoint was really asked.
    const asked = upstream.length;
    const silent = await post(SILENT_FAILING);
    assert.equal(silent.status, 200);
    assert.deepEqual(await rowsOf(silent), [["https://example.org/a", undefined]]);
    assert.equal(upstream.length, asked + 1, "the failing endpoint was contacted");
    assert.match(upstream.at(-1).body, /<https:\/\/example\.org\/fail>/);

    // The same failure without SILENT: 502, the endpoint's failure.
    const failed = await post(FAILING);
    assert.equal(failed.status, 502);
    assert.equal((await problemOf(failed)).code, "native-sparql-service-failed");

    // An endpoint the catalog does not admit: 403, and nothing was sent anywhere.
    const beforeDenied = upstream.length;
    const denied = await post(DENIED_QUERY);
    assert.equal(denied.status, 403);
    assert.equal((await problemOf(denied)).code, "native-sparql-service-denied");
    assert.equal(upstream.length, beforeDenied, "the denied endpoint was never asked");
    assert.deepEqual(stray, [], "no request left through the outbound fetch");

    // Calibrate a query long enough that a request sent while it runs is clearly inside
    // it. The recipe's dataset is grown through the recipe's own update route.
    let size = 1000;
    const long = PAIR_COUNT;
    let longAloneMs;
    for (;;) {
      const loaded = await post(pairData(size), "/sparql", "application/sparql-update");
      assert.equal(loaded.status, 204, `the ${size}-node update applies`);
      const started = performance.now();
      const alone = await post(long);
      longAloneMs = performance.now() - started;
      assert.equal(alone.status, 200, `the ${size}-value pair count answers`);
      const n = JSON.parse(await alone.text()).results.bindings[0].n.value;
      assert.equal(n, String((size * (size - 1)) / 2));
      if (longAloneMs >= 400 || size >= 4000) break;
      size *= 2;
    }
    const lane = await (await mf.dispatchFetch(`${WORKER}/__async?size=${size}`)).json();
    diagnostics([
      `long query: ${size} nodes, ${Math.round(longAloneMs)} ms alone through the recipe`,
      `long query on the async lane in the Worker: ${JSON.stringify(lane)}`,
    ]);
    assert.ok(longAloneMs >= 400, `the long query runs long enough to overlap (${longAloneMs} ms)`);
    const head = Math.min(100, longAloneMs / 4);

    // The asynchronous lane: the short request is served while the long one runs.
    const finished = [];
    const timeline = {};
    const longStarted = performance.now();
    const at = () => Math.round(performance.now() - longStarted);
    const longRun = post(long).then(async (response) => {
      timeline.longStatus = response.status;
      await response.text();
      timeline.longFinished = at();
      finished.push("long");
    });
    await sleep(head);
    const shortStarted = performance.now();
    timeline.shortSent = at();
    const shortRun = post(SHORT).then(async (response) => {
      timeline.shortStatus = response.status;
      await response.text();
      timeline.shortFinished = at();
      finished.push("short");
    });
    await Promise.all([shortRun, longRun]);
    const shortMs = performance.now() - shortStarted;
    diagnostics([
      `async lane timeline (ms from the long request): ${JSON.stringify(timeline)}; order ${JSON.stringify(finished)}`,
    ]);
    assert.equal(timeline.longStatus, 200);
    assert.equal(timeline.shortStatus, 200);
    assert.deepEqual(finished, ["short", "long"], "the short request was served during the long query");

    // The control: the same query on the synchronous lane never turns the event loop, so
    // the short request waits for it. Without this, an order the check above cannot fail
    // would pass as evidence.
    const controlFinished = [];
    const syncRun = post(long, `/__sync?size=${size}`).then(async (response) => {
      assert.equal(response.status, 200);
      assert.equal((await response.json()).n, String((size * (size - 1)) / 2));
      controlFinished.push("sync");
    });
    await sleep(head);
    const controlShort = post(SHORT).then(async (response) => {
      assert.equal(response.status, 200);
      await response.text();
      controlFinished.push("short");
    });
    await Promise.all([syncRun, controlShort]);
    assert.deepEqual(controlFinished, ["sync", "short"], "the synchronous control blocks the short request");

    summary([
      `workerd Worker recipe: asyncYieldPrimitive() = ${probe.primitive}`,
      `long query: a ${size}-node self-join, ${Math.round(longAloneMs)} ms alone`,
      `a request sent ${Math.round(head)} ms into it answered in ${Math.round(shortMs)} ms, before it finished`,
    ]);
  } finally {
    await mf.dispose();
  }
}

try {
  await main();
} catch (error) {
  console.error(error);
  process.exitCode = 1;
}
