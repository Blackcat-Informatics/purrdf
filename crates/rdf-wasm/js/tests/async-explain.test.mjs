// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Node real-execution tests for `QueryEngine#explainQueryAsync`, the asynchronous twin of
// `explainQuery`, driven through the package root against the actual optimized wasm
// module under WebAssembly JavaScript Promise Integration.
//
// EXPLAIN evaluates the query it explains — the ledger is a measurement, not a
// prediction — so its twin is a job like every other evaluating twin: it suspends on
// the host's SERVICE handler, yields to the event loop, and stops on its signal. Every
// host callback is a deterministic mock that records what it was handed, and every
// oracle compares the ledger's own numbers against an independent observation (the
// synchronous twin's text, or the rows `queryAsync` returns for the same query).
//
// Wall-clock bounds are never asserted: the machine running this is not quiet.

import { test } from "node:test";
import assert from "node:assert/strict";

import { Dataset, QueryEngine, ready } from "../index.mjs";
import { CROSS_COUNT, crossDataset } from "./fixtures/yield-workload.mjs";

await ready();

const EX = "http://example.org/";
const XSD_INTEGER = "http://www.w3.org/2001/XMLSchema#integer";
const ENDPOINT = `${EX}sparql`;

// Two local rows, `?s ex:p ?o`.
const LOCAL_NT = [`<${EX}a> <${EX}p> <${EX}o1> .`, `<${EX}b> <${EX}p> <${EX}o2> .`, ""].join("\n");
const local = () => Dataset.parse(LOCAL_NT, "nquads");

// The remote answer to the forwarded `{ ?o ex:q ?x }`: three rows, one of which (`o3`)
// joins no local row — so the SERVICE node's row count (3) and the answer's (2) differ,
// and the ledger can be read for both.
const REMOTE_OX = JSON.stringify({
  head: { vars: ["o", "x"] },
  results: {
    bindings: [
      { o: { type: "uri", value: `${EX}o1` }, x: { type: "literal", value: "x1" } },
      { o: { type: "uri", value: `${EX}o2` }, x: { type: "literal", value: "x2" } },
      { o: { type: "uri", value: `${EX}o3` }, x: { type: "literal", value: "x3" } },
    ],
  },
});

const joinQuery = (silent = false) =>
  `SELECT ?s ?x WHERE { ?s <${EX}p> ?o . SERVICE ${silent ? "SILENT " : ""}<${ENDPOINT}> { ?o <${EX}q> ?x } }`;

const LOCAL_ONLY = `SELECT ?s ?o WHERE { ?s <${EX}p> ?o . OPTIONAL { ?o <${EX}q> ?x } } ORDER BY ?s`;

/** A `resolveService` that records every call and answers with `answer(request, ctx)`. */
function recordingResolver(answer) {
  const calls = [];
  const resolveService = (request, ctx) => {
    calls.push({ request, ctx });
    return answer(request, ctx);
  };
  return { calls, resolveService };
}

/** Resolves to the rejection reason, or fails the test if `promise` resolves. */
async function rejection(promise) {
  try {
    await promise;
  } catch (error) {
    return error;
  }
  assert.fail("expected the promise to reject");
}

/** The synchronous throw of `fn`, or fails the test if it returns. */
function syncThrow(fn) {
  try {
    fn();
  } catch (error) {
    return error;
  }
  assert.fail("expected the call to throw");
}

/**
 * The ledger's node lines as `{ ordinal, label, fuel, rows, cells }`, in plan order —
 * parsed from the rendered text, so the oracle reads exactly what a caller reads.
 */
function ledgerNodes(text) {
  const lines = text.split("\n");
  const start = lines.indexOf("ledger");
  const end = lines.indexOf("relations");
  assert.ok(start >= 0 && end > start, `the explanation has a ledger block:\n${text}`);
  const nodes = [];
  for (const line of lines.slice(start + 1, end)) {
    const match = /^\s+#(\d+) (\w+) fuel=(\d+) rows=(\d+) cells=(\d+)/.exec(line);
    if (match) {
      nodes.push({
        ordinal: Number(match[1]),
        label: match[2],
        fuel: Number(match[3]),
        rows: Number(match[4]),
        cells: Number(match[5]),
      });
    }
  }
  assert.ok(nodes.length > 0, `the ledger lists nodes:\n${text}`);
  return nodes;
}

/** The one ledger node labelled `label`. */
function node(text, label) {
  const found = ledgerNodes(text).filter((candidate) => candidate.label === label);
  assert.equal(found.length, 1, `exactly one ${label} node:\n${text}`);
  return found[0];
}

/** The `consumed` block as a map from dimension label to units. */
function consumed(text) {
  const lines = text.split("\n");
  const start = lines.indexOf("consumed");
  assert.ok(start >= 0, `the explanation has a consumed block:\n${text}`);
  return Object.fromEntries(
    lines
      .slice(start + 1)
      .filter((line) => line.trim() !== "")
      .map((line) => line.trim().split("\t"))
      .map(([label, units]) => [label, Number(units)]),
  );
}

test("explainQueryAsync on a SERVICE-free query is byte-identical to explainQuery", async () => {
  const engine = new QueryEngine();
  const dataset = local();
  const sync = engine.explainQuery(dataset, LOCAL_ONLY);
  const explained = await engine.explainQueryAsync(dataset, LOCAL_ONLY);
  assert.equal(typeof explained, "string");
  assert.equal(explained, sync, "the whole rendered ledger, not merely its presence");
  // The ledger is not vacuous: the projection materialised both local rows.
  assert.equal(node(explained, "Project").rows, 2);

  // `base` is forwarded exactly as the synchronous twin forwards it.
  const relative = "SELECT ?o WHERE { <a> <p> ?o }";
  const based = { base: EX };
  assert.equal(await engine.explainQueryAsync(dataset, relative, based), engine.explainQuery(dataset, relative, based));
  assert.equal(node(engine.explainQuery(dataset, relative, based), "Project").rows, 1);
});

test("explainQueryAsync explains a SERVICE join over the host's answer; the synchronous twin refuses it", async () => {
  const engine = new QueryEngine();
  const query = joinQuery(false);

  // The offline lane's refusal, by name.
  const refused = syncThrow(() => engine.explainQuery(local(), query));
  assert.match(refused.message, /no remote query source configured/);

  // The asynchronous lane explains it, over the resolver's answer.
  const explainMock = recordingResolver(async () => REMOTE_OX);
  const explained = await engine.explainQueryAsync(local(), query, { resolveService: explainMock.resolveService });
  assert.equal(explainMock.calls.length, 1, "one SERVICE clause, one remote request");
  const [{ request, ctx }] = explainMock.calls;
  assert.equal(request.kind, "service");
  assert.equal(request.endpoint, ENDPOINT);
  assert.match(request.queryText, /^SELECT/);
  assert.match(request.queryText, new RegExp(`<${EX}q>`));
  assert.equal(ctx.silent, false);

  // The measured run is the evaluation `queryAsync` performs: the same forwarded text,
  // and a projection whose row count is the number of rows `queryAsync` answers.
  const queryMock = recordingResolver(async () => REMOTE_OX);
  const result = await engine.queryAsync(local(), query, { resolveService: queryMock.resolveService });
  assert.equal(queryMock.calls[0].request.queryText, request.queryText);
  const answered = result.rows.toArray().length;
  assert.equal(answered, 2);
  assert.equal(node(explained, "Project").rows, answered, explained);
  // The SERVICE node measured all three remote rows, including the one that joined none.
  assert.equal(node(explained, "Service").rows, 3, explained);
  assert.equal(consumed(explained)["remote-requests"], 1, explained);
});

test("SERVICE SILENT under explainQueryAsync: a failing resolver is the join identity, as on queryAsync", async () => {
  const engine = new QueryEngine();
  const query = joinQuery(true);
  const failing = () => recordingResolver(async () => ({ kind: "transport", message: "endpoint unreachable" }));

  const explainMock = failing();
  const explained = await engine.explainQueryAsync(local(), query, { resolveService: explainMock.resolveService });
  assert.equal(explainMock.calls.length, 1, "SILENT still asked the endpoint");
  assert.equal(explainMock.calls[0].ctx.silent, true);

  const queryMock = failing();
  const result = await engine.queryAsync(local(), query, { resolveService: queryMock.resolveService });
  const identityRows = result.rows.toArray();
  assert.equal(identityRows.length, 2);
  assert.ok(identityRows.every((row) => row.x === undefined), "nothing remote is bound");
  assert.equal(node(explained, "Project").rows, identityRows.length, explained);
  // The identity is not the answered join: the SERVICE node measured no remote row.
  assert.notEqual(node(explained, "Service").rows, 3, explained);

  // The non-SILENT neighbour of the same failure rejects, with the transport's words.
  const loud = await rejection(
    engine.explainQueryAsync(local(), joinQuery(false), { resolveService: failing().resolveService }),
  );
  assert.match(loud.message, /SERVICE <http:\/\/example\.org\/sparql>: transport: endpoint unreachable/);
});

test("a CPU-heavy explainQueryAsync yields to the event loop; the synchronous twin does not", async () => {
  const engine = new QueryEngine();
  const dataset = crossDataset(1000);
  let ticks = 0;
  const interval = setInterval(() => {
    ticks += 1;
  }, 5);
  try {
    const sync = engine.explainQuery(dataset, CROSS_COUNT);
    const syncTicks = ticks;
    ticks = 0;
    const explained = await engine.explainQueryAsync(dataset, CROSS_COUNT, { yieldEveryPolls: 1024 });
    const asyncTicks = ticks;
    assert.equal(syncTicks, 0, "the synchronous twin never turns the event loop");
    assert.ok(asyncTicks > 0, `the interval ticked ${asyncTicks} times during the asynchronous explain`);
    assert.equal(explained, sync, "yielding changed nothing the ledger measured");
    // The measured run is the heavy one: the filter saw every one of the million pairs.
    assert.equal(node(explained, "Filter").rows, (1000 * 999) / 2, explained);
  } finally {
    clearInterval(interval);
    dataset.free();
  }
});

// A dataset whose three-way self cross product cannot finish in any time a test would
// wait for, so a stop always lands while the measuring run is under way.
function wideDataset(size) {
  const lines = [];
  for (let index = 0; index < size; index += 1) {
    lines.push(`<${EX}w${index}> <${EX}v> "${index}"^^<${XSD_INTEGER}> .`);
  }
  return Dataset.parse(`${lines.join("\n")}\n`, "nquads");
}
const ENDLESS_COUNT = `SELECT (COUNT(*) AS ?n) WHERE { ?a <${EX}v> ?x . ?b <${EX}v> ?y . ?c <${EX}v> ?z }`;

test("an AbortSignal aborted mid-explain rejects with the abort reason", async () => {
  const engine = new QueryEngine();
  const wide = wideDataset(1000);
  try {
    const controller = new AbortController();
    const reason = new Error("the caller gave up");
    setTimeout(() => controller.abort(reason), 20);
    const error = await rejection(
      engine.explainQueryAsync(wide, ENDLESS_COUNT, { signal: controller.signal, yieldEveryPolls: 1024 }),
    );
    assert.equal(error, reason, "the twin rejects with the signal's own reason");

    // Without a reason of its own, the rejection is the signal's default AbortError.
    const bare = new AbortController();
    setTimeout(() => bare.abort(), 20);
    const aborted = await rejection(
      engine.explainQueryAsync(wide, ENDLESS_COUNT, { signal: bare.signal, yieldEveryPolls: 1024 }),
    );
    assert.equal(aborted.name, "AbortError");
    assert.equal(aborted, bare.signal.reason);

    // An already-aborted signal rejects before any job is begun.
    const early = new AbortController();
    early.abort(reason);
    assert.equal(await rejection(engine.explainQueryAsync(local(), LOCAL_ONLY, { signal: early.signal })), reason);

    // The neighbour: a signal that never fires resolves to the synchronous twin's ledger.
    const dataset = local();
    assert.equal(
      await engine.explainQueryAsync(dataset, LOCAL_ONLY, { signal: new AbortController().signal }),
      engine.explainQuery(dataset, LOCAL_ONLY),
    );
  } finally {
    wide.free();
  }
});

test("a deadline stops explainQueryAsync as it stops the other ungoverned twins", async () => {
  const engine = new QueryEngine();
  const wide = wideDataset(1000);
  try {
    // The ungoverned twins' deadline is a timeout signal, and it rejects with the
    // signal's TimeoutError — exactly as on `queryAsync`.
    const deadline = AbortSignal.timeout(20);
    const explainError = await rejection(
      engine.explainQueryAsync(wide, ENDLESS_COUNT, { signal: deadline, yieldEveryPolls: 1024 }),
    );
    assert.equal(explainError.name, "TimeoutError");
    assert.equal(explainError, deadline.reason);
    const queryDeadline = AbortSignal.timeout(20);
    const queryError = await rejection(
      engine.queryAsync(wide, ENDLESS_COUNT, { signal: queryDeadline, yieldEveryPolls: 1024 }),
    );
    assert.equal(queryError.name, explainError.name, "the same trip on both twins");

    // A ceiling is refused by name: EXPLAIN measures a run that is metered, never
    // bounded, exactly as the synchronous twin's is, so a `deadlineMs` would be ignored.
    const refused = await rejection(engine.explainQueryAsync(local(), LOCAL_ONLY, { deadlineMs: 20 }));
    assert.ok(refused instanceof TypeError);
    assert.match(refused.message, /deadlineMs is an execution governor/);
    const unknown = await rejection(engine.explainQueryAsync(local(), LOCAL_ONLY, { format: "json" }));
    assert.ok(unknown instanceof TypeError);
    assert.match(unknown.message, /unknown query option "format"/);

    // The neighbour: a generous timeout signal over the same small query resolves.
    const dataset = local();
    assert.equal(
      await engine.explainQueryAsync(dataset, LOCAL_ONLY, { signal: AbortSignal.timeout(600_000) }),
      engine.explainQuery(dataset, LOCAL_ONLY),
    );
  } finally {
    wide.free();
  }
});

test("a parse error rejects with the synchronous twin's words", async () => {
  const engine = new QueryEngine();
  const dataset = local();
  const syncError = syncThrow(() => engine.explainQuery(dataset, "SELEC"));
  const asyncError = await rejection(engine.explainQueryAsync(dataset, "SELEC"));
  assert.equal(asyncError.message, syncError.message);
  assert.equal(typeof asyncError.evidence.async.polls, "number", "the rejection carries the job's evidence");
});
