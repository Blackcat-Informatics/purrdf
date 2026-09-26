// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Node real-execution tests for how the asynchronous runtime shares `SERVICE` host calls:
// concurrent jobs issuing the same request through the same `resolveService` share one
// call only when the host would see an equivalent context for each (the same `silent`,
// the same `maxIntermediateCells`, and a deadline no later than the one the call was
// started with); the shared call's signal aborts only once every waiting job has been
// stopped; and one job asks the host at most once for a request it repeats.
//
// Every resolver here is answered by hand, so which calls exist, what each was told, and
// what its signal said when it was answered are all observed rather than timed. A job
// posts its first `SERVICE` effect before the twin that started it returns, so the calls
// counted right after starting the jobs are all the calls there will be before an answer.

import { test } from "node:test";
import assert from "node:assert/strict";

import { Dataset, QueryEngine, ready } from "../index.mjs";

await ready();

const EX = "http://example.org/";
const LOCAL_NT = [`<${EX}a> <${EX}p> <${EX}o1> .`, `<${EX}b> <${EX}p> <${EX}o2> .`, ""].join("\n");
const local = () => Dataset.parse(LOCAL_NT, "nquads");
const join = (silent = false) =>
  `SELECT ?s ?x WHERE { ?s <${EX}p> ?o . SERVICE ${silent ? "SILENT " : ""}<${EX}sparql> { ?o <${EX}q> ?x } }`;

/** The remote answer binding each local object to `tag`-suffixed values. */
const remote = (tag) =>
  JSON.stringify({
    head: { vars: ["o", "x"] },
    results: {
      bindings: [
        { o: { type: "uri", value: `${EX}o1` }, x: { type: "literal", value: `${tag}1` } },
        { o: { type: "uri", value: `${EX}o2` }, x: { type: "literal", value: `${tag}2` } },
      ],
    },
  });
const joined = (tag) => [`s=${EX}a&x=${tag}1`, `s=${EX}b&x=${tag}2`];
const IDENTITY = [`s=${EX}a`, `s=${EX}b`];

/** A select result's rows as sorted `name=value&…` strings (unbound names omitted). */
function rowsOf(select) {
  return select.rows
    .toArray()
    .map((row) =>
      Object.keys(row)
        .sort()
        .map((name) => `${name}=${row[name].value}`)
        .join("&"),
    )
    .sort();
}

/**
 * One resolver the test answers by hand. Each call is recorded with its request and
 * context; `call.answer(value)` resolves it and records whether its signal had aborted by
 * then.
 */
function handAnswered() {
  const calls = [];
  const resolveService = (request, ctx) =>
    new Promise((resolve) => {
      const call = { request, ctx, abortedWhenAnswered: undefined };
      call.answer = (value) => {
        call.abortedWhenAnswered = ctx.signal.aborted;
        resolve(value);
      };
      calls.push(call);
    });
  return { calls, resolveService };
}

async function rejection(promise) {
  try {
    await promise;
  } catch (error) {
    return error;
  }
  assert.fail("the job was expected to reject");
}

// ---------------------------------------------------------------------------
// Sharing
// ---------------------------------------------------------------------------

test("two concurrent jobs issuing the same request without deadlines share one host call", async () => {
  const engine = new QueryEngine();
  const host = handAnswered();
  const first = engine.queryAsync(local(), join(), { resolveService: host.resolveService });
  const second = engine.queryAsync(local(), join(), { resolveService: host.resolveService });
  assert.equal(host.calls.length, 1, "the second job joined the first job's call");
  assert.equal(host.calls[0].ctx.remainingDeadlineMs, undefined);
  host.calls[0].answer(remote("shared"));
  assert.deepEqual(rowsOf(await first), joined("shared"));
  assert.deepEqual(rowsOf(await second), joined("shared"), "the joining job received the shared answer");
  assert.equal(host.calls.length, 1);
  assert.equal(host.calls[0].abortedWhenAnswered, false);
});

test("a job stopped while it waits abandons only itself; the host's signal waits for the other job", async () => {
  const engine = new QueryEngine();
  const host = handAnswered();
  const stopFirst = new AbortController();
  const stopSecond = new AbortController();
  const first = engine.queryAsync(local(), join(), { resolveService: host.resolveService, signal: stopFirst.signal });
  const second = engine.queryAsync(local(), join(), { resolveService: host.resolveService, signal: stopSecond.signal });
  assert.equal(host.calls.length, 1);
  const hostSignal = host.calls[0].ctx.signal;

  const reason = new Error("the first caller gave up");
  stopFirst.abort(reason);
  assert.equal(await rejection(first), reason, "the stopped job rejects with its own signal's reason");
  assert.equal(hostSignal.aborted, false, "one job still waits, so the host's call goes on");

  host.calls[0].answer(remote("kept"));
  assert.deepEqual(rowsOf(await second), joined("kept"), "the job still waiting receives the answer");
  assert.equal(host.calls[0].abortedWhenAnswered, false);
  assert.equal(hostSignal.aborted, false, "an answered call is never aborted afterwards");
  stopSecond.abort();
});

test("the host's signal aborts once every job waiting on the call has been stopped", async () => {
  const engine = new QueryEngine();
  const host = handAnswered();
  const stopFirst = new AbortController();
  const stopSecond = new AbortController();
  const first = engine.queryAsync(local(), join(), { resolveService: host.resolveService, signal: stopFirst.signal });
  const second = engine.queryAsync(local(), join(), { resolveService: host.resolveService, signal: stopSecond.signal });
  assert.equal(host.calls.length, 1);
  const hostSignal = host.calls[0].ctx.signal;

  stopFirst.abort();
  assert.equal((await rejection(first)).name, "AbortError");
  assert.equal(hostSignal.aborted, false, "after the first stop, the second job still waits");
  stopSecond.abort();
  assert.equal((await rejection(second)).name, "AbortError");
  assert.equal(hostSignal.aborted, true, "with no job left waiting, the host's call is aborted");
  assert.equal(hostSignal.reason.name, "AbortError");
  // The late answer reaches no job.
  host.calls[0].answer(remote("late"));
  assert.equal(host.calls[0].abortedWhenAnswered, true);
});

// ---------------------------------------------------------------------------
// Contexts the host could tell apart are never shared
// ---------------------------------------------------------------------------

test("the same request under SERVICE and SERVICE SILENT is two host calls, each told its own silent", async () => {
  const engine = new QueryEngine();
  const host = handAnswered();
  const plain = engine.queryAsync(local(), join(false), { resolveService: host.resolveService });
  const silent = engine.queryAsync(local(), join(true), { resolveService: host.resolveService });
  assert.equal(host.calls.length, 2);
  assert.equal(host.calls[0].request.queryText, host.calls[1].request.queryText, "the requests are identical");
  assert.equal(host.calls[0].ctx.silent, false);
  assert.equal(host.calls[1].ctx.silent, true);
  host.calls[1].answer(remote("silent"));
  host.calls[0].answer(remote("plain"));
  assert.deepEqual(rowsOf(await plain), joined("plain"));
  assert.deepEqual(rowsOf(await silent), joined("silent"));
});

test("a different cell ceiling is its own host call; the same ceiling is shared", async () => {
  const engine = new QueryEngine();
  const host = handAnswered();
  const capped = engine.queryGovernedAsync(local(), join(), {
    resolveService: host.resolveService,
    maxIntermediateCells: 100_000,
  });
  const uncapped = engine.queryGovernedAsync(local(), join(), { resolveService: host.resolveService });
  const sameCap = engine.queryGovernedAsync(local(), join(), {
    resolveService: host.resolveService,
    maxIntermediateCells: 100_000,
  });
  assert.equal(host.calls.length, 2, "the uncapped job has its own call; the second capped job shares the first's");
  assert.equal(host.calls[0].ctx.maxIntermediateCells, 100_000n);
  assert.equal(host.calls[1].ctx.maxIntermediateCells, undefined);
  host.calls[0].answer(remote("capped"));
  host.calls[1].answer(remote("uncapped"));
  for (const [outcome, tag] of [
    [await capped, "capped"],
    [await uncapped, "uncapped"],
    [await sameCap, "capped"],
  ]) {
    assert.equal(outcome.isComplete, true);
    assert.deepEqual(rowsOf(outcome.result), joined(tag));
  }
});

test("a job whose deadline is later than the in-flight call's is its own call; an earlier one shares it", async () => {
  const engine = new QueryEngine();
  const host = handAnswered();
  const short = engine.queryGovernedAsync(local(), join(), { resolveService: host.resolveService, deadlineMs: 20_000 });
  const long = engine.queryGovernedAsync(local(), join(), { resolveService: host.resolveService, deadlineMs: 60_000 });
  const shorter = engine.queryGovernedAsync(local(), join(), { resolveService: host.resolveService, deadlineMs: 10_000 });
  assert.equal(host.calls.length, 2, "the 60 s job was not bound by the 20 s call; the 10 s job was");
  const [shortCall, longCall] = host.calls;
  assert.ok(shortCall.ctx.remainingDeadlineMs <= 20_000, `told ${shortCall.ctx.remainingDeadlineMs}`);
  assert.ok(longCall.ctx.remainingDeadlineMs > 20_000, `told ${longCall.ctx.remainingDeadlineMs}`);
  assert.ok(longCall.ctx.remainingDeadlineMs <= 60_000);
  assert.notEqual(shortCall.ctx.signal, longCall.ctx.signal);
  shortCall.answer(remote("short"));
  longCall.answer(remote("long"));
  for (const [outcome, tag] of [
    [await short, "short"],
    [await long, "long"],
    [await shorter, "short"],
  ]) {
    assert.equal(outcome.isComplete, true);
    assert.deepEqual(rowsOf(outcome.result), joined(tag));
  }
});

test("a job without a deadline never joins a call bounded by one; a job with one joins an unbounded call", async () => {
  const engine = new QueryEngine();
  const host = handAnswered();
  const bounded = engine.queryGovernedAsync(local(), join(), { resolveService: host.resolveService, deadlineMs: 60_000 });
  const unbounded = engine.queryGovernedAsync(local(), join(), { resolveService: host.resolveService });
  const boundedLater = engine.queryGovernedAsync(local(), join(), {
    resolveService: host.resolveService,
    deadlineMs: 90_000,
  });
  assert.equal(host.calls.length, 2);
  assert.equal(typeof host.calls[0].ctx.remainingDeadlineMs, "number");
  assert.equal(host.calls[1].ctx.remainingDeadlineMs, undefined, "the unbounded job's call is told no deadline");
  host.calls[0].answer(remote("bounded"));
  host.calls[1].answer(remote("unbounded"));
  assert.deepEqual(rowsOf((await bounded).result), joined("bounded"));
  assert.deepEqual(rowsOf((await unbounded).result), joined("unbounded"));
  // The 90 s job's deadline falls after the 60 s call's, so it joined the unbounded call
  // rather than the one bounded by an earlier instant.
  assert.deepEqual(rowsOf((await boundedLater).result), joined("unbounded"));
});

test("a short-deadline job's timed-out call is never a SILENT failure for a job with a longer deadline", async () => {
  const engine = new QueryEngine();
  // A host that bounds its work by the deadline it is told: the remote needs 30 s, so a
  // call told less than that reports the transport failure it would reach by timing out.
  const calls = [];
  const resolveService = (request, ctx) => {
    calls.push({ request, ctx });
    if (ctx.remainingDeadlineMs !== undefined && ctx.remainingDeadlineMs < 30_000) {
      return Promise.resolve({
        kind: "transport",
        message: `the example.org endpoint cannot answer within ${ctx.remainingDeadlineMs} ms`,
      });
    }
    return Promise.resolve(remote("full"));
  };
  const short = engine.queryGovernedAsync(local(), join(true), { resolveService, deadlineMs: 5_000 });
  const long = engine.queryGovernedAsync(local(), join(true), { resolveService, deadlineMs: 60_000 });
  assert.equal(calls.length, 2, "the 60 s job asked the host itself");
  assert.equal(calls[0].ctx.silent, true);
  assert.equal(calls[1].ctx.silent, true);
  const shortOutcome = await short;
  const longOutcome = await long;
  assert.equal(shortOutcome.isComplete, true);
  assert.deepEqual(rowsOf(shortOutcome.result), IDENTITY, "SILENT swallowed the short job's own failure");
  assert.equal(longOutcome.isComplete, true);
  assert.deepEqual(rowsOf(longOutcome.result), joined("full"), "the long job received its real rows");
});

// ---------------------------------------------------------------------------
// The per-job memo
// ---------------------------------------------------------------------------

const TWICE = `SELECT ?o ?x WHERE { { SERVICE <${EX}sparql> { ?o <${EX}q> ?x } } UNION { SERVICE <${EX}sparql> { ?o <${EX}q> ?x } } }`;
const ONCE = [`o=${EX}o1&x=m1`, `o=${EX}o2&x=m2`];

test("one job asks the host once for a request it repeats; sequential jobs each ask", async () => {
  const engine = new QueryEngine();
  let asked = 0;
  const resolveService = async () => {
    asked += 1;
    return remote("m");
  };
  const first = await engine.queryAsync(local(), TWICE, { resolveService });
  assert.equal(asked, 1, "the second SERVICE of the same job was answered from the job's memo");
  assert.deepEqual(rowsOf(first), [...ONCE, ...ONCE].sort(), "both branches joined the answer");
  const second = await engine.queryAsync(local(), TWICE, { resolveService });
  assert.equal(asked, 2, "a later job does not reuse an earlier job's answer");
  assert.deepEqual(rowsOf(second), [...ONCE, ...ONCE].sort());
});
