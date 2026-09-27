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
// Every ordering asserted here is caused, never timed: no clock is read, nothing is
// calibrated, and no sleep stands in for a synchronisation. Asserted:
//   * the runtime gives the package JSPI and `setTimeout`, so `hasAsyncQueries()`;
//   * a SERVICE the catalog admits is joined: 200 and SPARQL Results JSON;
//   * SERVICE SILENT over an endpoint that answered 500 is the join identity, and the
//     endpoint was really asked;
//   * a SERVICE the catalog does not admit is 403 `native-sparql-service-denied`, and
//     nothing was sent;
//   * an endpoint that answers 500 is 502 `native-sparql-service-failed`;
//   * suspension: a SERVICE whose upstream answer this harness holds open leaves its job
//     suspended, and a second request is answered — completely — while it is held; the
//     upstream is released only afterwards, and the held request then completes joined;
//   * yielding: a job run with `yieldEveryPolls: 0` gives the event loop a turn at every
//     poll, so a request parked in the Worker before the job began, whose answer comes
//     from an ordinary `setTimeout(…, 0)` task, is answered while the job runs and
//     records `longDone === false`; the control runs the same count on the synchronous
//     lane, which never turns the event loop, and the same parked request records
//     `longDone === true`.

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
// The upstream holds its answer to any query naming this predicate until the harness
// releases it.
const GATED = QUERY.replace("<https://example.org/q>", "<https://example.org/gated>");
const SHORT = "SELECT ?s WHERE { ?s <https://example.org/p> ?o }";

// The yielding experiment's operand: size·(size−1)/2 pairs, each a governor poll, so a
// job at `yieldEveryPolls: 0` yields thousands of times — far more than the one turn the
// observer's task needs to be reached.
const LONG_SIZE = 100;

const watchdog = setTimeout(() => {
  console.error(`the workerd Worker-recipe run did not finish within ${RUN_TIMEOUT_MS} ms`);
  process.exit(1);
}, RUN_TIMEOUT_MS);
watchdog.unref();

/** A promise settled by hand, from anywhere. */
function deferred() {
  let resolve;
  const promise = new Promise((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

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

/** Diagnostics, printed before the assertions they explain. */
function diagnostics(lines) {
  for (const line of lines) console.log(`[workerd diagnostics] ${line}`);
  if (process.env.GITHUB_STEP_SUMMARY) {
    appendFileSync(process.env.GITHUB_STEP_SUMMARY, `${lines.map((line) => `- ${line}`).join("\n")}\n`);
  }
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
  // The gate the upstream holds a GATED answer behind: `asked` settles when the request
  // reaches the upstream, `release` lets the answer go.
  const asked = deferred();
  const release = deferred();
  let released = false;
  // lane -> settled when that lane's observer is parked in the Worker.
  const observers = new Map();
  const observerWaiting = (lane) => {
    let entry = observers.get(lane);
    if (entry === undefined) {
      entry = deferred();
      observers.set(lane, entry);
    }
    return entry;
  };
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
        if (body.includes("<https://example.org/gated>")) {
          asked.resolve();
          await release.promise;
        }
        return new Response(remoteAnswer(), {
          headers: { "Content-Type": "application/sparql-results+json" },
        });
      },
      async MARK(request) {
        const lane = new URL(request.url).searchParams.get("lane");
        observerWaiting(lane).resolve();
        return new Response(null, { status: 204 });
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
    diagnostics([`probe: hasAsyncQueries() = ${probe.hasAsyncQueries}`]);
    assert.equal(probe.hasAsyncQueries, true, "workerd provides JSPI and setTimeout");

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
    const askedBefore = upstream.length;
    const silent = await post(SILENT_FAILING);
    assert.equal(silent.status, 200);
    assert.deepEqual(await rowsOf(silent), [["https://example.org/a", undefined]]);
    assert.equal(upstream.length, askedBefore + 1, "the failing endpoint was contacted");
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

    // Suspension. The GATED query's job reaches its SERVICE and suspends while the
    // upstream holds the answer. Once the request has reached the upstream, a SHORT
    // request is sent and answered — to the last byte — while the gate is still shut;
    // only then is the gate opened, and the held request completes joined. Nothing here
    // depends on how long anything took: the gate is closed until the short answer is in
    // hand, so "short before gated" is the only order the run can produce.
    const finished = [];
    const gatedRun = post(GATED).then(async (response) => {
      const rows = await rowsOf(response);
      finished.push("gated");
      return { status: response.status, rows };
    });
    await asked.promise;
    assert.equal(released, false);
    const short = await post(SHORT);
    const shortRows = await rowsOf(short);
    finished.push("short");
    const shortAnsweredWhileHeld = !released;
    released = true;
    release.resolve();
    const gated = await gatedRun;
    diagnostics([
      `suspension: the short request answered ${short.status} with ${JSON.stringify(shortRows)} while the SERVICE answer was held; ` +
        `the held request then answered ${gated.status}; order ${JSON.stringify(finished)}`,
    ]);
    assert.equal(short.status, 200);
    assert.deepEqual(shortRows, [["https://example.org/a", undefined]]);
    assert.equal(shortAnsweredWhileHeld, true, "the short request was answered while the SERVICE answer was held");
    assert.equal(gated.status, 200);
    assert.deepEqual(gated.rows, [["https://example.org/a", "joined"]], "the held request completed joined once released");
    assert.deepEqual(finished, ["short", "gated"]);
    assert.match(upstream.at(-1).body, /<https:\/\/example\.org\/gated>/);

    // Yielding, on the asynchronous lane and then its synchronous control. Each lane's
    // observer is parked in the Worker (it reports that through MARK, awaited here) before
    // the lane's long request is sent, so the observer is waiting when the job begins; it
    // answers from a `setTimeout(…, 0)` task queued once the job has begun, and records
    // whether the job was done by then. A job yielding at every poll runs thousands of
    // turns after that task is queued, so the task runs — and the observer answers — while
    // the job is still in progress; a synchronous run holds the isolate until it returns,
    // so the task runs only afterwards.
    const expectedCount = String((LONG_SIZE * (LONG_SIZE - 1)) / 2);
    const experiment = async (lane) => {
      const observing = mf.dispatchFetch(`${WORKER}/__observe?lane=${lane}`).then((response) => response.json());
      await observerWaiting(lane).promise;
      const long = await (await mf.dispatchFetch(`${WORKER}/__long?lane=${lane}&size=${LONG_SIZE}`)).json();
      const observed = await observing;
      return { long, observed };
    };

    const asyncLane = await experiment("async");
    diagnostics([
      `yielding (async lane): the long job reported ${JSON.stringify(asyncLane.long)}`,
      `yielding (async lane): the observer recorded ${JSON.stringify(asyncLane.observed)}`,
    ]);
    assert.equal(asyncLane.long.lane, "async");
    assert.equal(asyncLane.long.n, expectedCount, "the asynchronous lane counted every pair");
    assert.ok(
      asyncLane.long.async.yields >= 2,
      `the job yielded ${asyncLane.long.async.yields} times; the observer's task is queued before every yield but the first`,
    );
    assert.deepEqual(asyncLane.observed, { lane: "async", longDone: false }, "the observer answered while the yielding job ran");

    const syncLane = await experiment("sync");
    diagnostics([
      `yielding (sync control): the long job reported ${JSON.stringify(syncLane.long)}`,
      `yielding (sync control): the observer recorded ${JSON.stringify(syncLane.observed)}`,
    ]);
    assert.equal(syncLane.long.lane, "sync");
    assert.equal(syncLane.long.n, expectedCount, "the synchronous lane counted every pair");
    assert.deepEqual(syncLane.observed, { lane: "sync", longDone: true }, "the observer answered only after the synchronous run");

    assert.deepEqual(stray, [], "no request left through the outbound fetch");

    summary([
      "workerd Worker recipe: JSPI and setTimeout present; joined, SILENT, 502 and 403 answered as the recipe promises",
      "suspension: a request was answered while a SERVICE answer was held open",
      `yielding: a ${LONG_SIZE}-node self-join at yieldEveryPolls 0 yielded ${asyncLane.long.async.yields} times and the observer answered mid-job; ` +
        "the synchronous control answered it only afterwards",
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
