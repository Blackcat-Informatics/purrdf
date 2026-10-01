// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// No watchdog or process.exit: the deadline must keep an otherwise idle Node alive,
// and successful, failed or cancelled effects must leave no referenced expiry timer.
import assert from "node:assert/strict";

const timers = new Set();
const originalSet = globalThis.setTimeout;
const originalClear = globalThis.clearTimeout;
globalThis.setTimeout = (callback, delay, ...args) => {
  const handle = originalSet(() => {
    timers.delete(handle);
    callback(...args);
  }, delay);
  timers.add(handle);
  return handle;
};
globalThis.clearTimeout = (handle) => {
  timers.delete(handle);
  originalClear(handle);
};

const { Dataset, QueryEngine, ServiceCatalog, ready } = await import("../../index.mjs");
await ready();
const mode = process.argv[2];
const EX = "https://example.org/";
const body = JSON.stringify({
  head: { vars: ["x"] },
  results: { bindings: [{ x: { type: "uri", value: `${EX}answer` } }] },
});
const query = `SELECT ?x WHERE { SERVICE <${EX}sparql> { ?s ?p ?x } }`;
const engine = new QueryEngine();
const dataset = new Dataset();
const isLoad = mode.includes("load");
const action = mode.split("-")[0];
const controller = new AbortController();
let asked;
const invocation = new Promise((resolve) => { asked = resolve; });
const handler = (_request, context) => {
  asked(context);
  if (action === "hung" || action === "cancel") return new Promise(() => {});
  if (action === "failure") return { kind: "transport", message: "endpoint unavailable" };
  return isLoad ? { text: `<${EX}s> <${EX}p> <${EX}o> .`, mediaType: "text/turtle" } : body;
};
if (action === "siblings") {
  const catalog = new ServiceCatalog();
  catalog.addService(`${EX}sparql`, JSON.stringify({ capabilities: ["network", "query"] }));
  let release;
  let context;
  let calls = 0;
  const resolveService = (_request, ctx) => {
    calls += 1;
    context = ctx;
    asked();
    return new Promise((resolve) => { release = resolve; });
  };
  const first = engine.queryGovernedAsync(dataset, query, { catalog, resolveService, deadlineMs: 60_000 });
  await invocation;
  const second = engine.queryGovernedAsync(dataset, query, { catalog, resolveService, deadlineMs: 80 });
  const expired = await second;
  assert.equal(expired.isComplete, false);
  assert.equal(expired.tripped.cause, "deadline-exceeded");
  assert.equal(context.signal.aborted, false, "one expired waiter does not abort its sibling");
  release(body);
  const answered = await first;
  assert.equal(answered.isComplete, true);
  assert.equal(answered.result.rowCount, 1);
  assert.equal(calls, 1, "both jobs share the host exchange");
} else {
  const options = {
    deadlineMs: action === "hung" ? 30 : 60_000,
    signal: controller.signal,
    [isLoad ? "resolveLoad" : "resolveService"]: handler,
  };
  const pending = isLoad
    ? engine.updateGovernedAsync(dataset, `LOAD <${EX}doc>`, options)
    : engine.queryGovernedAsync(dataset, query, options);
  if (action === "cancel") {
    await invocation;
    controller.abort();
  }
  let outcome;
  let error;
  try { outcome = await pending; } catch (caught) { error = caught; }
  if (action === "failure") {
    assert.ok(error, "transport failure rejects");
    assert.match(error.code, /(?:service|load)-failed$/);
  } else if (action === "hung" || action === "cancel") {
    assert.equal(outcome[isLoad ? "isApplied" : "isComplete"], false);
    assert.equal(outcome.tripped.cause, action === "hung" ? "deadline-exceeded" : "cancelled");
  } else {
    assert.equal(outcome[isLoad ? "isApplied" : "isComplete"], true);
  }
}
assert.equal(timers.size, 0, "every effect expiry timer is released");
assert.equal(engine.select(dataset, "SELECT * WHERE {} ").rowCount, 1);
process.stdout.write(`${JSON.stringify({ mode, timers: timers.size })}\n`);
