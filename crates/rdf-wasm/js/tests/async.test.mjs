// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Node real-execution tests for the ASYNCHRONOUS query surface: the Promise-returning
// twins (`queryAsync` … `updateGovernedAsync`, `Dataset#queryAsync`), driven through the
// package root against the actual optimized wasm module under WebAssembly JavaScript
// Promise Integration.
//
// Every host callback here is a deterministic mock — no endpoint is ever contacted — and
// every mock RECORDS what it was handed (endpoint, forwarded text, headers, context), so
// the assertions observe what PurRDF actually forwarded rather than only what came back.
// Every refusal is paired with a valid neighbour whose result observably differs.
//
// Wall-clock bounds are never asserted: the machine running this is not quiet. Where a
// test is about *when* something settled, the oracle is an ordering fact instead — "the
// twin settled before the resolver's own timer fired" — which a loaded machine cannot
// flip.

import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import {
  CompiledJsonLdContext,
  Dataset,
  QueryEngine,
  ServiceCatalog,
  asyncYieldPrimitive,
  hasAsyncQueries,
  ready,
} from "../index.mjs";
import { AsyncJobOptions, AsyncOperationKind } from "../pkg/purrdf_wasm.js";
import { NO_JSPI_MESSAGE, runJob } from "../pkg/purrdf_jspi.mjs";
import { expectedCrossCount, measureYielding } from "./fixtures/yield-workload.mjs";

// One-time wasm instantiation before any test runs.
await ready();

const EX = "http://example.org/";
const XSD_INTEGER = "http://www.w3.org/2001/XMLSchema#integer";
const encoder = new TextEncoder();

// Two local rows, `?s ex:p ?o`, whose objects the remote side knows about.
const LOCAL_NT = [`<${EX}a> <${EX}p> <${EX}o1> .`, `<${EX}b> <${EX}p> <${EX}o2> .`, ""].join("\n");
const local = () => Dataset.parse(LOCAL_NT, "nquads");

/** SPARQL Results JSON for `rows` (IRIs for values starting `http`, plain literals else). */
function srj(vars, rows) {
  const cell = (value) =>
    value.startsWith("http") ? { type: "uri", value } : { type: "literal", value };
  return JSON.stringify({
    head: { vars },
    results: {
      bindings: rows.map((row) =>
        Object.fromEntries(Object.entries(row).map(([name, value]) => [name, cell(value)])),
      ),
    },
  });
}

// The remote side's answer to the forwarded `{ ?o ex:q ?x }`.
const REMOTE_OX = srj(
  ["o", "x"],
  [
    { o: `${EX}o1`, x: "x1" },
    { o: `${EX}o2`, x: "x2" },
  ],
);

const joinQuery = (silent = false, endpoint = `${EX}sparql`) =>
  `SELECT ?s ?x WHERE { ?s <${EX}p> ?o . SERVICE ${silent ? "SILENT " : ""}<${endpoint}> { ?o <${EX}q> ?x } }`;

const JOINED = [`s=${EX}a&x=x1`, `s=${EX}b&x=x2`];
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

/** A governed partial's (or result's) rows, the same way. */
const partialRows = (outcome) => rowsOf(outcome.partial.result);

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

const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

/** Turn the event loop until `predicate()` holds (bounded, never a sleep). */
async function turnUntil(predicate, label) {
  for (let turns = 0; turns < 10_000; turns += 1) {
    if (predicate()) return;
    await new Promise((resolve) => setImmediate(resolve));
  }
  assert.fail(`the event loop turned 10 000 times without ${label}`);
}

/**
 * A `resolveService` that records every call — request and context, with the signal's
 * state — and answers with `answer(request, ctx)`.
 */
function recordingResolver(answer) {
  const calls = [];
  const resolveService = (request, ctx) => {
    calls.push({ request, ctx, abortedAtCall: ctx.signal.aborted });
    return answer(request, ctx);
  };
  return { calls, resolveService };
}

/** A resolver whose answers the test releases by hand, in any order. */
function deferredResolver() {
  const pending = [];
  const resolveService = (request, ctx) =>
    new Promise((resolve) => {
      pending.push({ request, ctx, resolve });
    });
  return { pending, resolveService };
}

// ---------------------------------------------------------------------------
// AC1 / AC2: an awaited SERVICE, and the host's answers
// ---------------------------------------------------------------------------

test("queryAsync joins remote bindings with local rows", async () => {
  const engine = new QueryEngine();
  const mock = recordingResolver(async () => {
    await delay(5);
    return encoder.encode(REMOTE_OX);
  });
  const result = await engine.queryAsync(local(), joinQuery(), { resolveService: mock.resolveService });
  assert.equal(result.kind, "select");
  assert.deepEqual(rowsOf(result), JOINED);
  assert.equal(mock.calls.length, 1, "one SERVICE clause, one remote request");
  const [{ request, ctx }] = mock.calls;
  assert.equal(request.kind, "service");
  assert.equal(request.endpoint, `${EX}sparql`);
  assert.match(request.queryText, /^SELECT/);
  assert.match(request.queryText, new RegExp(`<${EX}q>`));
  assert.equal(request.contentType, "application/sparql-query");
  assert.equal(request.accept, "application/sparql-results+json");
  assert.deepEqual(request.headers, []);
  assert.equal(ctx.silent, false);
  assert.ok(ctx.signal instanceof AbortSignal);
});

test("a delayed promise is awaited, not raced", async () => {
  const engine = new QueryEngine();
  let answeredAt;
  const started = performance.now();
  const result = await engine.queryAsync(local(), joinQuery(), {
    resolveService: async () => {
      // Waits until at least 50 ms have passed by this clock, however the timer rounds.
      const called = performance.now();
      while (performance.now() - called < 50) await delay(50 - (performance.now() - called));
      answeredAt = performance.now();
      return REMOTE_OX;
    },
  });
  const settledAt = performance.now();
  assert.deepEqual(rowsOf(result), JOINED, "the delayed answer is the one joined");
  assert.ok(answeredAt !== undefined, "the resolver ran to its answer");
  assert.ok(settledAt >= answeredAt, "the twin settled only after the resolver answered");
  assert.ok(settledAt - started >= 50, `elapsed ${settledAt - started} ms`);
});

test("a transport failure is a thrown error without SILENT", async () => {
  const engine = new QueryEngine();
  const mock = recordingResolver(async () => ({ kind: "transport", message: "endpoint unreachable" }));
  const error = await rejection(engine.queryAsync(local(), joinQuery(false), { resolveService: mock.resolveService }));
  assert.ok(error instanceof Error);
  assert.match(error.message, /SERVICE <http:\/\/example\.org\/sparql>: transport: endpoint unreachable/);
  assert.equal(mock.calls.length, 1);
  assert.equal(mock.calls[0].ctx.silent, false);
  assert.equal(typeof error.evidence.async.polls, "number", "a twin's error carries the job's evidence");
});

test("a transport failure is the join identity under SILENT", async () => {
  const engine = new QueryEngine();
  const mock = recordingResolver(async () => ({ kind: "transport", message: "endpoint unreachable" }));
  const result = await engine.queryAsync(local(), joinQuery(true), { resolveService: mock.resolveService });
  assert.deepEqual(rowsOf(result), IDENTITY, "the local rows survive, nothing remote is bound");
  assert.equal(mock.calls.length, 1, "SILENT still asked the endpoint");
  assert.equal(mock.calls[0].ctx.silent, true, "the host is told the clause is SILENT");
});

test("a resolver that throws is a job fault even under SILENT", async () => {
  const engine = new QueryEngine();
  const thrown = await rejection(
    engine.queryAsync(local(), joinQuery(true), {
      resolveService: () => {
        throw new Error("host bug");
      },
    }),
  );
  assert.ok(thrown instanceof Error);
  assert.match(thrown.message, /resolveService threw: Error: host bug/);
  assert.equal(typeof thrown.evidence.async.stackHighWaterBytes, "number");

  const rejected = await rejection(
    engine.queryAsync(local(), joinQuery(true), {
      resolveService: async () => {
        throw new Error("async host bug");
      },
    }),
  );
  assert.match(rejected.message, /resolveService rejected: Error: async host bug/);

  // The valid neighbour: the same SILENT clause, the resolver reporting its failure as a
  // typed transport failure, is the join identity.
  const neighbour = await engine.queryAsync(local(), joinQuery(true), {
    resolveService: async () => ({ kind: "transport", message: "caught its own network error" }),
  });
  assert.deepEqual(rowsOf(neighbour), IDENTITY);
});

test("a host denial with no catalog installed is never silenced, and never invents a catalog cause", async () => {
  const engine = new QueryEngine();
  for (const silent of [false, true]) {
    const error = await rejection(
      engine.queryAsync(local(), joinQuery(silent), {
        resolveService: async () => ({ kind: "denied", message: "tenant may not federate" }),
      }),
    );
    // No `catalog` option is installed at all: whatever this host reports is its own
    // policy, never a capability a catalog withheld — see the neighbour below, which
    // installs a real catalog and keeps the OLD "withholds the ... capability" wording.
    assert.match(
      error.message,
      /SERVICE <http:\/\/example\.org\/sparql>: the host denied the request: tenant may not federate/,
      `silent=${silent}`,
    );
    assert.doesNotMatch(
      error.message,
      /withholds the .* capability/,
      `silent=${silent}: a bare host denial must not invent a catalog-capability cause`,
    );
  }
  // The neighbour: a transport failure under the same SILENT is the identity.
  const neighbour = await engine.queryAsync(local(), joinQuery(true), {
    resolveService: async () => ({ kind: "transport", message: "down" }),
  });
  assert.deepEqual(rowsOf(neighbour), IDENTITY);
});

test("a catalog denies an endpoint it does not list, before any host call", async () => {
  const engine = new QueryEngine();
  const catalog = new ServiceCatalog();
  catalog.addService(
    `${EX}sparql`,
    JSON.stringify({
      capabilities: ["query", "network", "credentials"],
      headers: [
        ["X-Tenant", "a"],
        ["X-Trace", "t-1"],
      ],
      credential: { header: "Authorization", value: "Bearer token" },
      userAgent: "example-agent/1",
      timeoutMs: 1234,
    }),
  );
  const unlisted = recordingResolver(async () => REMOTE_OX);
  for (const silent of [false, true]) {
    const denied = await rejection(
      engine.queryAsync(local(), joinQuery(silent, `${EX}elsewhere`), { catalog, resolveService: unlisted.resolveService }),
    );
    // A real catalog refusal keeps its own wording — distinct from a bare host denial's
    // "the host denied the request" (see the neighbour test above) — and SILENT never
    // swallows it either.
    assert.match(
      denied.message,
      /SERVICE federation denied: <http:\/\/example\.org\/elsewhere> withholds the query capability: no profile is configured for this service/,
      `silent=${silent}`,
    );
    assert.doesNotMatch(denied.message, /the host denied the request/, `silent=${silent}`);
  }
  assert.equal(unlisted.calls.length, 0, "the host is never asked about a denied endpoint");

  // The neighbour: the listed endpoint is forwarded, with the profile's headers in order
  // and then the credential, and the profile's user agent and timeout.
  const listed = recordingResolver(async () => REMOTE_OX);
  const result = await engine.queryAsync(local(), joinQuery(false), { catalog, resolveService: listed.resolveService });
  assert.deepEqual(rowsOf(result), JOINED);
  assert.equal(listed.calls.length, 1);
  const { request } = listed.calls[0];
  assert.deepEqual(request.headers, [
    ["X-Tenant", "a"],
    ["X-Trace", "t-1"],
    ["Authorization", "Bearer token"],
  ]);
  assert.equal(request.userAgent, "example-agent/1");
  assert.equal(request.timeoutMs, 1234);
});

test("SERVICE SILENT with a resolver still forwards the request", async () => {
  const engine = new QueryEngine();
  const mock = recordingResolver(async () => REMOTE_OX);
  const result = await engine.queryAsync(local(), joinQuery(true), { resolveService: mock.resolveService });
  assert.deepEqual(rowsOf(result), JOINED, "the answer is joined, not replaced by the identity");
  assert.equal(mock.calls.length, 1);
  assert.equal(mock.calls[0].ctx.silent, true);
  assert.match(mock.calls[0].request.queryText, /^SELECT/);
});

// ---------------------------------------------------------------------------
// AC3: variable and nested endpoints
// ---------------------------------------------------------------------------

// The remote side of a variable endpoint: every endpoint answers with its own name, so a
// row joined to the wrong endpoint's answer is visible.
const answerWithEndpointName = async (request) =>
  srj(["x"], [{ x: `answer-from-${request.endpoint.slice(EX.length)}` }]);

test("a variable endpoint bound by VALUES resolves per row", async () => {
  const engine = new QueryEngine();
  // Both spellings the evaluator accepts: the explicit LATERAL, and the plain join whose
  // VALUES binding reaches the SERVICE as a correlated substitution.
  for (const shape of [
    `SELECT ?g ?x WHERE { VALUES ?g { <${EX}e1> <${EX}e2> } LATERAL { SERVICE ?g { ?s ?p ?x } } }`,
    `SELECT ?g ?x WHERE { VALUES ?g { <${EX}e1> <${EX}e2> } SERVICE ?g { ?s ?p ?x } }`,
  ]) {
    const mock = recordingResolver(answerWithEndpointName);
    const result = await engine.queryAsync(local(), shape, { resolveService: mock.resolveService });
    assert.deepEqual(
      rowsOf(result),
      [`g=${EX}e1&x=answer-from-e1`, `g=${EX}e2&x=answer-from-e2`],
      shape,
    );
    assert.deepEqual(
      mock.calls.map((call) => call.request.endpoint).sort(),
      [`${EX}e1`, `${EX}e2`],
      "one request per bound endpoint",
    );
    for (const call of mock.calls) assert.match(call.request.queryText, /^SELECT/);
  }
});

test("an endpoint variable nothing binds is refused, under SILENT too", async () => {
  const engine = new QueryEngine();
  const MESSAGE = /unsupported in sparql-eval \(S6 scope\): SERVICE \?g with no endpoint: .*LATERAL/;
  for (const shape of [
    `SELECT ?g ?x WHERE { SERVICE ?g { ?s ?p ?x } }`,
    // A LATERAL whose left side never binds the endpoint variable.
    `SELECT ?g ?x WHERE { ?a <${EX}p> ?o . LATERAL { SERVICE ?g { ?s ?p ?x } } }`,
    // SILENT tolerates an endpoint that fails, not a query that names none: before this
    // was refused, both of these answered the left rows alone and asked nobody.
    `SELECT ?s ?g ?x WHERE { ?s <${EX}p> ?o . SERVICE SILENT ?g { ?a ?b ?x } }`,
    `SELECT ?s ?g ?x WHERE { ?s <${EX}p> ?o . OPTIONAL { SERVICE SILENT ?g { ?a ?b ?x } } }`,
  ]) {
    const mock = recordingResolver(answerWithEndpointName);
    const error = await rejection(engine.queryAsync(local(), shape, { resolveService: mock.resolveService }));
    assert.match(error.message, MESSAGE, shape);
    assert.match(syncThrow(() => engine.query(local(), shape)).message, MESSAGE, "the sync lane says the same");
    assert.equal(mock.calls.length, 0, "an endpoint that was never bound is never asked");
  }
  // The neighbours: the same SILENT clauses with the endpoint bound on the left answer
  // from the endpoint, not with the left rows alone.
  for (const shape of [
    `SELECT ?s ?g ?x WHERE { ?s <${EX}p> ?o . VALUES ?g { <${EX}e1> } SERVICE SILENT ?g { ?a ?b ?x } }`,
    `SELECT ?s ?g ?x WHERE { ?s <${EX}p> ?o . VALUES ?g { <${EX}e1> } OPTIONAL { SERVICE SILENT ?g { ?a ?b ?x } } }`,
  ]) {
    const mock = recordingResolver(answerWithEndpointName);
    const result = await engine.queryAsync(local(), shape, { resolveService: mock.resolveService });
    assert.deepEqual(
      rowsOf(result),
      [`g=${EX}e1&s=${EX}a&x=answer-from-e1`, `g=${EX}e1&s=${EX}b&x=answer-from-e1`],
      shape,
    );
    assert.deepEqual([...new Set(mock.calls.map((call) => call.request.endpoint))], [`${EX}e1`], shape);
  }
});

// SILENT tolerates an endpoint that fails. It does not tolerate a query run with nowhere
// to send the request (no source), nor a SERVICE whose endpoint value is not an IRI (it
// names no endpoint): before, both answered the local rows joined with an identity no
// endpoint produced. Both lanes refuse them, with the same message, and ask nobody.
test("SERVICE SILENT with no source, or with an endpoint that is not an IRI, is refused on both lanes", async () => {
  const engine = new QueryEngine();
  // No source: the job has no resolveService, the synchronous lane never has one.
  const noSource = joinQuery(true);
  const asyncNoSource = await rejection(engine.queryAsync(local(), noSource));
  assert.equal(
    asyncNoSource.message,
    `error native-sparql-query-eval: SERVICE federation error: no remote query source configured for SERVICE <${EX}sparql>; ` +
      "SILENT does not apply: it tolerates an endpoint that fails, and no endpoint was reached — configure a remote query source for it",
  );
  assert.equal(syncThrow(() => engine.query(local(), noSource)).message, asyncNoSource.message);
  // Local services and no handler: an unlisted endpoint has no source either.
  const unlisted = await rejection(
    engine.queryAsync(local(), noSource, { localServices: { [`${EX}elsewhere`]: local() } }),
  );
  assert.match(unlisted.message, /no remote query source configured: the endpoint is not a local service.*SILENT does not apply/);

  // An endpoint value that is not an IRI, bound before the clause and on the left of an
  // OPTIONAL.
  for (const [shape, message] of [
    [
      `SELECT ?s ?e ?x WHERE { ?s <${EX}p> ?o . BIND("x" AS ?e) SERVICE SILENT ?e { ?a ?b ?x } }`,
      /SERVICE \?e: \?e is bound to the literal "x", which is not an IRI, so there is no endpoint to send the request to; SILENT does not apply/,
    ],
    [
      `SELECT ?s ?e ?x WHERE { ?s <${EX}p> ?o . VALUES ?e { "x" } OPTIONAL { SERVICE SILENT ?e { ?a ?b ?x } } }`,
      /SERVICE \?e: \?e is bound to the literal "x", which is not an IRI, so there is no endpoint to send the request to; SILENT does not apply/,
    ],
  ]) {
    const mock = recordingResolver(answerWithEndpointName);
    const error = await rejection(engine.queryAsync(local(), shape, { resolveService: mock.resolveService }));
    assert.match(error.message, message, shape);
    assert.equal(syncThrow(() => engine.query(local(), shape)).message, error.message, shape);
    assert.equal(mock.calls.length, 0, `${shape}: nobody is asked`);
  }

  // The valid neighbours: with a source whose endpoint fails, SILENT is the join identity
  // (one call); with an IRI endpoint it answers from the endpoint (one call).
  const failing = recordingResolver(async () => ({ kind: "transport", message: "the example.org host is down" }));
  assert.deepEqual(rowsOf(await engine.queryAsync(local(), noSource, { resolveService: failing.resolveService })), IDENTITY);
  assert.equal(failing.calls.length, 1);
  const answering = recordingResolver(answerWithEndpointName);
  const bound = `SELECT ?s ?e ?x WHERE { ?s <${EX}p> ?o . BIND(<${EX}e1> AS ?e) SERVICE SILENT ?e { ?a ?b ?x } }`;
  assert.deepEqual(rowsOf(await engine.queryAsync(local(), bound, { resolveService: answering.resolveService })), [
    `e=${EX}e1&s=${EX}a&x=answer-from-e1`,
    `e=${EX}e1&s=${EX}b&x=answer-from-e1`,
  ]);
  assert.deepEqual([...new Set(answering.calls.map((call) => call.request.endpoint))], [`${EX}e1`]);
});

// A left side binding `?e` to endpoints: `ex:e1` twice, `ex:e2`, `ex:e3` (which answers
// nothing) and `ex:down` (which fails at the transport).
const ENDPOINTS_NT = [
  ["g1", "e1"],
  ["g2", "e2"],
  ["g3", "e1"],
  ["g4", "e3"],
  ["g5", "down"],
]
  .map(([g, e]) => `<${EX}${g}> <${EX}endpoint> <${EX}${e}> .`)
  .concat([""])
  .join("\n");
const endpointsLocal = () => Dataset.parse(ENDPOINTS_NT, "nquads");

// A left solution that names no endpoint — `?e` left unbound by an OPTIONAL, or bound to a
// literal — refuses the clause before any endpoint is asked, in either row order. Before,
// the solutions ahead of it had already sent their requests (with any credentials the
// handler attaches), so whether a refused query contacted a remote depended on row order.
test("a variable endpoint some solution cannot name refuses the query before any request, in either row order", async () => {
  const engine = new QueryEngine();
  for (const shape of [
    `SELECT * WHERE { VALUES ?g { <${EX}g1> <${EX}nobody> } OPTIONAL { ?g <${EX}endpoint> ?e } SERVICE ?e { ?s ?p ?x } }`,
    `SELECT * WHERE { VALUES ?g { <${EX}nobody> <${EX}g1> } OPTIONAL { ?g <${EX}endpoint> ?e } SERVICE ?e { ?s ?p ?x } }`,
    `SELECT * WHERE { VALUES ?e { <${EX}e1> "x" } SERVICE ?e { ?s ?p ?x } }`,
    `SELECT * WHERE { VALUES ?e { "x" <${EX}e1> } SERVICE ?e { ?s ?p ?x } }`,
  ]) {
    const mock = recordingResolver(answerWithEndpointName);
    const error = await rejection(engine.queryAsync(endpointsLocal(), shape, { resolveService: mock.resolveService }));
    assert.match(error.message, /some solutions of the pattern before it leave it unbound|which is not an IRI/, shape);
    assert.equal(mock.calls.length, 0, `${shape}: nobody is asked`);
    assert.equal(syncThrow(() => engine.query(endpointsLocal(), shape)).message, error.message, shape);
  }
  // The valid neighbour: every solution names an endpoint, and each is asked once.
  const mock = recordingResolver(answerWithEndpointName);
  const answered = await engine.queryAsync(
    endpointsLocal(),
    `SELECT ?g ?e ?x WHERE { VALUES ?g { <${EX}g1> <${EX}g2> } OPTIONAL { ?g <${EX}endpoint> ?e } SERVICE ?e { ?s ?p ?x } }`,
    { resolveService: mock.resolveService },
  );
  assert.deepEqual(rowsOf(answered), [
    `e=${EX}e1&g=${EX}g1&x=answer-from-e1`,
    `e=${EX}e2&g=${EX}g2&x=answer-from-e2`,
  ]);
  assert.deepEqual(mock.calls.map((call) => call.request.endpoint).sort(), [`${EX}e1`, `${EX}e2`]);
});
const answerPerEndpoint = async (request) => {
  const name = request.endpoint.slice(EX.length);
  if (name === "down") return { kind: "transport", message: "endpoint unreachable" };
  if (name === "e3") return srj(["x"], []);
  return srj(["x"], [{ x: `answer-from-${name}` }]);
};
const BOUND_LEFT = `VALUES ?e { <${EX}e1> <${EX}e2> <${EX}e3> } ?g <${EX}endpoint> ?e`;
const distinctEndpoints = (mock) => [...new Set(mock.calls.map((call) => call.request.endpoint))].sort();
const row = (g, e, x) => [`e=${EX}${e}`, `g=${EX}${g}`].concat(x ? [`x=${x}`] : []).join("&");

test("a variable endpoint bound by an OPTIONAL, group join or MINUS left side is asked once per endpoint", async () => {
  const engine = new QueryEngine();
  const cases = [
    {
      shape: `${BOUND_LEFT} OPTIONAL { SERVICE ?e { ?s ?p ?x } }`,
      // ex:g4's endpoint answered nothing: the OPTIONAL keeps its left bindings.
      rows: [
        row("g1", "e1", "answer-from-e1"),
        row("g2", "e2", "answer-from-e2"),
        row("g3", "e1", "answer-from-e1"),
        row("g4", "e3"),
      ],
    },
    {
      shape: `{ ${BOUND_LEFT} } { SERVICE ?e { ?s ?p ?x } }`,
      rows: [row("g1", "e1", "answer-from-e1"), row("g2", "e2", "answer-from-e2"), row("g3", "e1", "answer-from-e1")],
    },
    {
      // Every row whose endpoint has an answer is removed; ex:g4's has none.
      shape: `${BOUND_LEFT} MINUS { SERVICE ?e { ?s ?p ?x } }`,
      rows: [row("g4", "e3")],
    },
  ];
  for (const { shape, rows } of cases) {
    const query = `SELECT ?g ?e ?x WHERE { ${shape} }`;
    const mock = recordingResolver(answerPerEndpoint);
    const result = await engine.queryAsync(endpointsLocal(), query, { resolveService: mock.resolveService });
    assert.deepEqual(rowsOf(result), [...rows].sort(), shape);
    assert.equal(mock.calls.length, 3, `${shape}: one request per distinct endpoint`);
    assert.deepEqual(distinctEndpoints(mock), [`${EX}e1`, `${EX}e2`, `${EX}e3`], shape);
    for (const call of mock.calls) assert.match(call.request.queryText, /^SELECT/);
  }
});

test("SILENT swallows one variable endpoint's failure for that endpoint only", async () => {
  const engine = new QueryEngine();
  const left = `?g <${EX}endpoint> ?e`;
  const answered = [row("g1", "e1", "answer-from-e1"), row("g2", "e2", "answer-from-e2"), row("g3", "e1", "answer-from-e1")];
  for (const [shape, rows] of [
    [`${left} OPTIONAL { SERVICE SILENT ?e { ?s ?p ?x } }`, [...answered, row("g4", "e3"), row("g5", "down")]],
    // The failed endpoint contributes its own left row and pads nobody else's.
    [`{ ${left} } { SERVICE SILENT ?e { ?s ?p ?x } }`, [...answered, row("g5", "down")]],
  ]) {
    const mock = recordingResolver(answerPerEndpoint);
    const result = await engine.queryAsync(endpointsLocal(), `SELECT ?g ?e ?x WHERE { ${shape} }`, {
      resolveService: mock.resolveService,
    });
    assert.deepEqual(rowsOf(result).sort(), rows.sort(), shape);
    assert.equal(mock.calls.length, 4, shape);
    assert.ok(mock.calls.every((call) => call.ctx.silent === true), shape);
  }
  // Without SILENT the failure fails the query.
  const mock = recordingResolver(answerPerEndpoint);
  const error = await rejection(
    engine.queryAsync(endpointsLocal(), `SELECT ?g ?e ?x WHERE { ${left} OPTIONAL { SERVICE ?e { ?s ?p ?x } } }`, {
      resolveService: mock.resolveService,
    }),
  );
  assert.match(error.message, /SERVICE <http:\/\/example\.org\/down>/);
});

test("a nested SERVICE is forwarded inside the outer request text", async () => {
  const engine = new QueryEngine();
  const mock = recordingResolver(async () => srj(["x", "y"], [{ x: "outer-x", y: "inner-y" }]));
  const result = await engine.queryAsync(
    local(),
    `SELECT ?x ?y WHERE { SERVICE <${EX}outer> { ?s ?p ?x SERVICE <${EX}inner> { ?x ?q ?y } } }`,
    { resolveService: mock.resolveService },
  );
  assert.deepEqual(rowsOf(result), ["x=outer-x&y=inner-y"]);
  assert.equal(mock.calls.length, 1, "only the outer endpoint is asked; it owns the inner clause");
  assert.equal(mock.calls[0].request.endpoint, `${EX}outer`);
  assert.match(mock.calls[0].request.queryText, new RegExp(`SERVICE <${EX}inner> \\{`));
});

// ---------------------------------------------------------------------------
// AC4: the synchronous lane is unchanged
// ---------------------------------------------------------------------------

test("queryAsync without resolveService fails like the sync path", async () => {
  const engine = new QueryEngine();
  const asyncError = await rejection(engine.queryAsync(local(), joinQuery()));
  const syncError = syncThrow(() => engine.query(local(), joinQuery()));
  assert.equal(asyncError.message, syncError.message);
  assert.match(asyncError.message, /no remote query source configured for SERVICE <http:\/\/example\.org\/sparql>/);
  // The neighbour: the same query with a resolver answers.
  const answered = await engine.queryAsync(local(), joinQuery(), { resolveService: async () => REMOTE_OX });
  assert.deepEqual(rowsOf(answered), JOINED);
});

test("synchronous query methods are unchanged", () => {
  const TRIG = `
@prefix ex: <https://example.org/> .
ex:a ex:knows ex:b .
ex:a ex:name "Ann" .
ex:b ex:name "Bob" .
graph <https://example.org/g> { ex:c ex:knows ex:a . }
`;
  // A SERVICE clause hard-fails offline.
  assert.throws(() =>
    Dataset.parse(TRIG, "trig").query(
      "PREFIX ex: <https://example.org/> SELECT ?o WHERE { SERVICE <https://remote.example.org/sparql> { ?s ex:knows ?o } }",
    ),
  );

  // SERVICE SILENT and LOAD SILENT are refused too: the synchronous lane has no source,
  // so no endpoint or document was reached for SILENT to tolerate.
  const ds = Dataset.parse("@prefix ex: <https://example.org/> . ex:a ex:p ex:b .", "turtle");
  assert.throws(() => ds.query("SELECT * WHERE { ?s ?p ?o SERVICE <https://example.org/endpoint> { ?a ?b ?c } }"));
  assert.throws(
    () => ds.query("SELECT * WHERE { ?s ?p ?o SERVICE SILENT <https://example.org/endpoint> { ?a ?b ?c } }"),
    /no remote query source configured for SERVICE <https:\/\/example\.org\/endpoint>; SILENT does not apply/,
  );
  const engine = new QueryEngine();
  const before = ds.canonicalize();
  assert.throws(() => engine.update(ds, "LOAD <https://example.org/doc>"));
  assert.throws(() => engine.update(ds, "LOAD SILENT <https://example.org/doc>"), /native-sparql-load-no-resolver: .*SILENT does not apply/);
  assert.equal(ds.canonicalize(), before, "a refused LOAD SILENT leaves the dataset untouched");

  // A genuine query error still throws on the governed lane.
  const governed = Dataset.parse(TRIG, "trig");
  assert.throws(() => engine.queryGoverned(governed, "SELECT ?x WHERE { this is not sparql"));
  assert.throws(() =>
    engine.queryGoverned(
      governed,
      "PREFIX ex: <https://example.org/> SELECT ?o WHERE { SERVICE <https://remote.example.org/sparql> { ?s ex:knows ?o } }",
    ),
  );
});

// ---------------------------------------------------------------------------
// U1: every evaluating surface has a twin
// ---------------------------------------------------------------------------

const TWIN_NT = [
  `<${EX}a> <${EX}p> <${EX}o1> .`,
  `<${EX}b> <${EX}p> <${EX}o2> .`,
  `<${EX}a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <${EX}Cat> .`,
  `<${EX}Cat> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <${EX}Animal> .`,
  "",
].join("\n");
const twinData = () => Dataset.parse(TWIN_NT, "nquads");

test("every async twin evaluates", async () => {
  const engine = new QueryEngine();
  const data = twinData();
  const SELECT = `SELECT ?s ?o WHERE { ?s <${EX}p> ?o }`;
  const ASK = `ASK { ?s <${EX}p> ?o }`;
  const CONSTRUCT = `CONSTRUCT { ?o <${EX}back> ?s } WHERE { ?s <${EX}p> ?o }`;
  const DESCRIBE = `DESCRIBE <${EX}a>`;

  const query = await engine.queryAsync(data, SELECT);
  assert.equal(query.kind, "select");
  assert.deepEqual(rowsOf(query), rowsOf(engine.query(data, SELECT)));
  assert.equal(query.rowCount, 2);

  const select = await engine.selectAsync(data, SELECT);
  const selectSync = engine.select(data, SELECT);
  assert.deepEqual(select.variables, selectSync.variables);
  assert.deepEqual(rowsOf(select), rowsOf(selectSync));

  assert.equal(await engine.askAsync(data, ASK), true);
  assert.equal(await engine.askAsync(data, `ASK { ?s <${EX}absent> ?o }`), false);
  assert.equal(engine.ask(data, ASK), true);

  const constructed = await engine.constructAsync(data, CONSTRUCT);
  assert.ok(constructed instanceof Dataset);
  assert.equal(constructed.size, 2);
  assert.ok(constructed.isomorphic(engine.construct(data, CONSTRUCT)));

  const described = await engine.describeAsync(data, DESCRIBE);
  const describedSync = engine.describe(data, DESCRIBE);
  assert.ok(described.size > 0);
  assert.ok(described.isomorphic(describedSync));

  // queryRawAsync: JSON and Turtle, the default format, and a provenance namespace.
  assert.equal(await engine.queryRawAsync(data, SELECT, { format: "json" }), engine.queryRaw(data, SELECT, { format: "json" }));
  const turtle = await engine.queryRawAsync(data, CONSTRUCT, { format: "turtle" });
  assert.equal(turtle, engine.queryRaw(data, CONSTRUCT, { format: "turtle" }));
  assert.match(turtle, /back/);
  assert.equal(await engine.queryRawAsync(data, SELECT), engine.queryRaw(data, SELECT));
  const provenanceNamespace = { prefix: "prov", iri: `${EX}ns/prov#` };
  assert.equal(
    await engine.queryRawAsync(data, ASK, { format: "json", provenanceNamespace }),
    engine.queryRaw(data, ASK, { format: "json", provenanceNamespace }),
  );

  // queryRawBytesAsync: the UTF-8 of the string.
  const json = await engine.queryRawAsync(data, SELECT, { format: "json" });
  const bytes = await engine.queryRawBytesAsync(data, SELECT, { format: "json" });
  assert.ok(bytes instanceof Uint8Array);
  assert.deepEqual(bytes, encoder.encode(json));

  // The configured and compiled-context JSON-LD lanes.
  const optionsJson = JSON.stringify({ version: 1, mode: "context", prefixes: { ex: EX } });
  assert.equal(
    await engine.queryRawAsync(data, CONSTRUCT, { format: "jsonld", optionsJson }),
    engine.queryRawConfigured(data, CONSTRUCT, undefined, "jsonld", optionsJson),
  );
  const context = new CompiledJsonLdContext(optionsJson);
  assert.equal(
    await engine.queryRawWithContextAsync(data, CONSTRUCT, "jsonld", context),
    engine.queryRawWithContext(data, CONSTRUCT, undefined, "jsonld", context),
  );

  // Dataset#queryAsync, with and without a base.
  assert.equal(await data.queryAsync(SELECT), data.query(SELECT));
  assert.equal(
    await data.queryAsync("SELECT ?s WHERE { ?s <p> ?o }", { base: EX }),
    data.query("SELECT ?s WHERE { ?s <p> ?o }", EX),
  );

  // The governed query twin: equal answers, the async evidence attached.
  const governed = await engine.queryGovernedAsync(data, SELECT, { fuel: 1_000_000 });
  const governedSync = engine.queryGoverned(data, SELECT, { fuel: 1_000_000 });
  assert.equal(governed.isComplete, true);
  assert.deepEqual(rowsOf(governed.result), rowsOf(governedSync.result));
  assert.equal(governed.evidence.limits.fuel, governedSync.evidence.limits.fuel);
  assert.ok(governed.evidence.async.polls > 0);
  const capped = await engine.queryGovernedAsync(data, SELECT, { maxAnswers: 1 });
  const cappedSync = engine.queryGoverned(data, SELECT, { maxAnswers: 1 });
  assert.equal(capped.isComplete, false);
  assert.equal(capped.tripped.label, cappedSync.tripped.label);
  assert.equal(capped.partial.result.rowCount, cappedSync.partial.result.rowCount);

  // The entailment twin.
  const ANIMALS = `SELECT ?x WHERE { ?x a <${EX}Animal> }`;
  const entailed = await engine.queryEntailmentGovernedAsync(data, ANIMALS, "rdfs", { fuel: 10_000_000 });
  const entailedSync = engine.queryEntailmentGoverned(data, ANIMALS, "rdfs", { fuel: 10_000_000 });
  assert.equal(entailed.phase, "answered");
  assert.equal(entailed.phase, entailedSync.phase);
  const entailedRows = rowsOf(entailed.outcome.result);
  assert.deepEqual(entailedRows, [`x=${EX}a`]);
  assert.deepEqual(entailedRows, rowsOf(entailedSync.outcome.result));
  assert.equal(entailed.report, entailedSync.report);
  assert.ok(entailed.evidence.async.polls > 0);
  assert.equal(entailed.outcome.evidence.async, entailed.evidence.async);

  // The update twins apply exactly what the synchronous ones do.
  const INSERT = `INSERT { ?o <${EX}inv> ?s } WHERE { ?s <${EX}p> ?o }`;
  const target = twinData();
  const generation = target.generation;
  assert.equal(await engine.updateAsync(target, INSERT), target);
  const control = engine.update(twinData(), INSERT);
  assert.ok(target.isomorphic(control));
  assert.equal(target.size, 6);
  assert.equal(target.generation, generation + 1);
  const governedTarget = twinData();
  const applied = await engine.updateGovernedAsync(governedTarget, INSERT, { fuel: 1_000_000 });
  const governedControl = twinData();
  assert.equal(engine.updateGoverned(governedControl, INSERT, { fuel: 1_000_000 }).isApplied, true);
  assert.equal(applied.isApplied, true);
  assert.ok(governedTarget.isomorphic(governedControl));
  assert.equal(typeof applied.evidence.async.polls, "number");
  const trippedTarget = twinData();
  const tripped = await engine.updateGovernedAsync(trippedTarget, INSERT, { fuel: 0 });
  assert.equal(tripped.isApplied, false, "a tripped update applies nothing");
  assert.equal(trippedTarget.size, 4);
  assert.equal(trippedTarget.generation, 0);

  // Kind mismatches and parse errors carry the synchronous messages.
  for (const [asyncCall, syncCall] of [
    [() => engine.selectAsync(data, ASK), () => engine.select(data, ASK)],
    [() => engine.askAsync(data, SELECT), () => engine.ask(data, SELECT)],
    [() => engine.constructAsync(data, SELECT), () => engine.construct(data, SELECT)],
    [() => engine.describeAsync(data, SELECT), () => engine.describe(data, SELECT)],
    [() => engine.queryAsync(data, "SELEKT"), () => engine.query(data, "SELEKT")],
  ]) {
    const asyncError = await rejection(asyncCall());
    assert.equal(asyncError.message, syncThrow(syncCall).message);
  }

  // updateGovernedAsync refuses maxAnswers with the synchronous message; the neighbour
  // without it applies.
  const refusedTarget = twinData();
  const refused = await rejection(engine.updateGovernedAsync(refusedTarget, INSERT, { maxAnswers: 1 }));
  assert.equal(refused.message, syncThrow(() => engine.updateGoverned(twinData(), INSERT, { maxAnswers: 1 })).message);
  assert.equal(refusedTarget.size, 4, "the refused update applied nothing");
  const neighbour = twinData();
  const neighbourOutcome = await engine.updateGovernedAsync(neighbour, INSERT, { fuel: 1_000_000 });
  assert.equal(neighbourOutcome.isApplied, true);
  assert.equal(neighbour.size, 6);
});

// ---------------------------------------------------------------------------
// U2: yielding and cancellation
// ---------------------------------------------------------------------------

test("a long query yields to the event loop", async () => {
  const measured = await measureYielding(new QueryEngine(), 1000);
  assert.equal(measured.syncCount, expectedCrossCount(1000));
  assert.equal(measured.syncTicks, 0, "the synchronous control never turns the event loop");
  assert.equal(measured.asyncComplete, true);
  assert.equal(measured.asyncCount, measured.syncCount);
  assert.ok(measured.asyncTicks > 0, `the interval ticked ${measured.asyncTicks} times during the async run`);
  assert.ok(measured.yields > 0, `evidence.async.yields = ${measured.yields}`);
  assert.ok(
    ["scheduler.yield", "setImmediate"].includes(asyncYieldPrimitive()),
    `a preferred macrotask primitive is chosen here (${asyncYieldPrimitive()})`,
  );
});

// A dataset whose three-way self cross product cannot finish in any time a test would
// wait for, so a cancellation always lands while it runs.
function wideDataset(size) {
  const lines = [];
  for (let index = 0; index < size; index += 1) {
    lines.push(`<${EX}w${index}> <${EX}v> "${index}"^^<${XSD_INTEGER}> .`);
  }
  return Dataset.parse(`${lines.join("\n")}\n`, "nquads");
}
const ENDLESS_COUNT = `SELECT (COUNT(*) AS ?n) WHERE { ?a <${EX}v> ?x . ?b <${EX}v> ?y . ?c <${EX}v> ?z }`;
// A query whose SERVICE is its leading pattern: a trip while the exchange is awaited
// truncates at the origin, where the positional-prefix claim is exact.
const LEADING_SERVICE = (silent = false) =>
  `SELECT ?o ?x WHERE { SERVICE ${silent ? "SILENT " : ""}<${EX}sparql> { ?o <${EX}q> ?x } }`;
const ENDLESS_ROWS = `SELECT ?a ?b ?c WHERE { ?a <${EX}v> ?x . ?b <${EX}v> ?y . ?c <${EX}v> ?z }`;

test("AbortSignal cancels a running query", async () => {
  const engine = new QueryEngine();
  const wide = wideDataset(1000);

  // Ungoverned: the twin rejects with the signal's reason, an AbortError.
  const controller = new AbortController();
  setTimeout(() => controller.abort(), 20);
  const error = await rejection(engine.queryAsync(wide, ENDLESS_COUNT, { signal: controller.signal, yieldEveryPolls: 1024 }));
  assert.equal(error.name, "AbortError");
  assert.equal(error, controller.signal.reason);

  // Governed: the trip is an outcome carrying certain rows.
  const governedController = new AbortController();
  setTimeout(() => governedController.abort(), 20);
  const outcome = await engine.queryGovernedAsync(wideDataset(100), ENDLESS_ROWS, {
    signal: governedController.signal,
    yieldEveryPolls: 1024,
  });
  assert.equal(outcome.isComplete, false);
  assert.equal(outcome.tripped.kind, "stopped");
  assert.equal(outcome.tripped.cause, "cancelled");
  assert.equal(outcome.partial.isCertain, true);
  assert.ok(outcome.evidence.async.yields > 0, "the cancellation was observed between slices");

  // A cancellation that lands while a SERVICE is awaited is the honouring row: the
  // exchange is abandoned where it stands, so the (empty) rows in hand keep their
  // positional-prefix claim.
  const serviceController = new AbortController();
  const awaited = engine.queryGovernedAsync(local(), LEADING_SERVICE(), {
    signal: serviceController.signal,
    resolveService: (request, ctx) =>
      new Promise((resolve) => {
        serviceController.abort();
        ctx.signal.addEventListener("abort", () => resolve(REMOTE_OX));
      }),
  });
  const serviceOutcome = await awaited;
  assert.equal(serviceOutcome.tripped.cause, "cancelled");
  assert.equal(serviceOutcome.partial.isPositionalPrefix, true);
  assert.deepEqual(partialRows(serviceOutcome), []);

  // The neighbour: the same governed call with a signal that never fires completes.
  const neighbour = await engine.queryGovernedAsync(local(), LEADING_SERVICE(), {
    signal: new AbortController().signal,
    resolveService: async () => REMOTE_OX,
  });
  assert.equal(neighbour.isComplete, true);
  assert.deepEqual(rowsOf(neighbour.result), [`o=${EX}o1&x=x1`, `o=${EX}o2&x=x2`]);
});

test("a resolver that ignores ctx.signal still cancels promptly", async () => {
  const engine = new QueryEngine();
  let resolverAnswered = false;
  const controller = new AbortController();
  setTimeout(() => controller.abort(), 20);
  const error = await rejection(
    engine.queryAsync(local(), joinQuery(), {
      signal: controller.signal,
      // Never looks at its signal: answers after 500 ms whatever happens.
      resolveService: () =>
        new Promise((resolve) =>
          setTimeout(() => {
            resolverAnswered = true;
            resolve(REMOTE_OX);
          }, 500),
        ),
    }),
  );
  assert.equal(error.name, "AbortError");
  assert.equal(resolverAnswered, false, "the twin settled on the abort, not on the resolver's answer");

  // Through the scheduler directly: the late answer, delivered by the mock to the job it
  // was asked for, is refused — the job has finished (status 3), nothing is resumed.
  const options = new AsyncJobOptions();
  options.setFormat("json");
  options.setHandlers(true, false);
  const job = engine.beginAsync(local(), AsyncOperationKind.Raw, joinQuery(), options);
  options.free();
  const late = {};
  let lateDone;
  const lateDelivered = new Promise((resolve) => {
    lateDone = resolve;
  });
  const lowLevel = new AbortController();
  setTimeout(() => lowLevel.abort(), 20);
  try {
    const status = await runJob(job, {
      signal: lowLevel.signal,
      resolveService: () =>
        new Promise((resolve) =>
          setTimeout(() => {
            late.status = job.deliverBindings(1, encoder.encode(REMOTE_OX));
            resolve(REMOTE_OX);
            lateDone();
          }, 500),
        ),
    });
    assert.equal(status, 1, "the run stored an error");
    assert.equal(job.errorKind, "cancelled");
    await lateDelivered;
    assert.equal(late.status, 3, "a delivery after the job finished is refused");
    assert.equal(job.isFinished, true);
  } finally {
    job.finish();
    job.free();
  }

  // The neighbour: an answer that arrives before the abort is accepted and joined.
  const neighbourController = new AbortController();
  const answered = engine.queryAsync(local(), joinQuery(), {
    signal: neighbourController.signal,
    resolveService: async () => REMOTE_OX,
  });
  const result = await answered;
  neighbourController.abort();
  assert.deepEqual(rowsOf(result), JOINED);
});

test("deadlineMs spans the awaited SERVICE time", async () => {
  const engine = new QueryEngine();
  for (const silent of [false, true]) {
    let resolverAnswered = false;
    const outcome = await engine.queryGovernedAsync(local(), LEADING_SERVICE(silent), {
      deadlineMs: 30,
      resolveService: () =>
        new Promise((resolve) =>
          setTimeout(() => {
            resolverAnswered = true;
            resolve(REMOTE_OX);
          }, 60),
        ),
    });
    assert.equal(outcome.isComplete, false, `silent=${silent}: a deadline is never the join identity`);
    assert.equal(outcome.tripped.kind, "stopped");
    assert.equal(outcome.tripped.cause, "deadline-exceeded");
    assert.equal(outcome.partial.isPositionalPrefix, true, "the abandoned exchange leaves a prefix");
    assert.deepEqual(partialRows(outcome), []);
    assert.equal(resolverAnswered, false, "the deadline, not the 60 ms answer, ended the wait");
    assert.equal(outcome.evidence.async.serviceEffects, 1);
  }
  // The neighbour: a deadline beyond the resolver's answer completes.
  const neighbour = await engine.queryGovernedAsync(local(), LEADING_SERVICE(), {
    deadlineMs: 60_000,
    resolveService: () => new Promise((resolve) => setTimeout(() => resolve(REMOTE_OX), 60)),
  });
  assert.equal(neighbour.isComplete, true);
  assert.deepEqual(rowsOf(neighbour.result), [`o=${EX}o1&x=x1`, `o=${EX}o2&x=x2`]);
});

// ---------------------------------------------------------------------------
// Governors over the awaited exchange
// ---------------------------------------------------------------------------

const TWO_BLOCKS = `SELECT ?x ?y WHERE { SERVICE <${EX}e1> { ?s ?p ?x } SERVICE <${EX}e2> { ?t ?q ?y } }`;

test("maxRemoteRequests governs async SERVICE dispatch", async () => {
  const engine = new QueryEngine();
  const answer = async (request) => srj(request.endpoint.endsWith("e1") ? ["x"] : ["y"], [
    request.endpoint.endsWith("e1") ? { x: "x1" } : { y: "y1" },
  ]);
  const capped = recordingResolver(answer);
  const outcome = await engine.queryGovernedAsync(local(), TWO_BLOCKS, {
    maxRemoteRequests: 1,
    resolveService: capped.resolveService,
  });
  assert.equal(outcome.isComplete, false);
  assert.equal(outcome.tripped.kind, "budget");
  assert.equal(outcome.tripped.dimension, "remote-requests");
  assert.equal(outcome.tripped.label, "remote-exhausted");
  assert.equal(outcome.tripped.limit, 1n);
  assert.equal(capped.calls.length, 1, "the second request was prevented, not made and discarded");

  const neighbour = recordingResolver(answer);
  const complete = await engine.queryGovernedAsync(local(), TWO_BLOCKS, {
    maxRemoteRequests: 2,
    resolveService: neighbour.resolveService,
  });
  assert.equal(complete.isComplete, true);
  assert.deepEqual(rowsOf(complete.result), ["x=x1&y=y1"]);
  assert.equal(neighbour.calls.length, 2);
  assert.equal(complete.evidence.consumed["remote-requests"], 2n);
});

test("maxIntermediateCells bounds the decoded remote answer", async () => {
  const engine = new QueryEngine();
  const thousand = srj(
    ["x"],
    Array.from({ length: 1000 }, (_, index) => ({ x: `v${index}` })),
  );
  const query = `SELECT ?x WHERE { SERVICE <${EX}e> { ?s ?p ?x } }`;
  const capped = recordingResolver(async () => thousand);
  const outcome = await engine.queryGovernedAsync(local(), query, {
    maxIntermediateCells: 10,
    resolveService: capped.resolveService,
  });
  assert.equal(capped.calls[0].ctx.maxIntermediateCells, 10n, "the host is told the ceiling");
  assert.equal(outcome.isComplete, false);
  assert.equal(outcome.tripped.dimension, "intermediate-cells");
  assert.equal(outcome.tripped.limit, 10n);
  assert.ok(outcome.partial.result.rowCount <= 10, `${outcome.partial.result.rowCount} rows kept`);

  const neighbour = recordingResolver(async () => thousand);
  const complete = await engine.queryGovernedAsync(local(), query, {
    maxIntermediateCells: 100_000,
    resolveService: neighbour.resolveService,
  });
  assert.equal(neighbour.calls[0].ctx.maxIntermediateCells, 100_000n);
  assert.equal(complete.isComplete, true);
  assert.equal(complete.result.rowCount, 1000);
});

test("an unconfigured cell ceiling never reaches the resolver as the metering sentinel", async () => {
  // `queryAsync` and a `queryGovernedAsync` that sets no `maxIntermediateCells` both run
  // the query under the engine's internal METERED governor, which engages the
  // intermediate-cell dimension purely to keep its counter running. That is a bookkeeping
  // value, not a ceiling any caller asked for, and a host that read it as one (sizing a
  // remote `LIMIT` from it, say) would size real infrastructure off a number that means
  // "measure this, bound nothing" — so the resolver must see `undefined`, never a number.
  const query = `SELECT ?x WHERE { SERVICE <${EX}e> { ?s ?p ?x } }`;
  const oneRow = async () => srj(["x"], [{ x: "x1" }]);

  const ungoverned = recordingResolver(oneRow);
  const engine = new QueryEngine();
  const plain = await engine.queryAsync(local(), query, { resolveService: ungoverned.resolveService });
  assert.equal(plain.kind, "select");
  assert.equal(
    ungoverned.calls[0].ctx.maxIntermediateCells,
    undefined,
    "an ungoverned query set no ceiling, so the resolver must see none — not the metering value",
  );

  const deadlineOnly = recordingResolver(oneRow);
  const timed = await engine.queryGovernedAsync(local(), query, {
    deadlineMs: 60_000,
    resolveService: deadlineOnly.resolveService,
  });
  assert.equal(timed.isComplete, true);
  assert.equal(
    deadlineOnly.calls[0].ctx.maxIntermediateCells,
    undefined,
    "a deadline-only governed query configured no cell ceiling either",
  );

  // Neighbour, over the same deadline-only shape: a caller who DOES configure a cell
  // ceiling must still have the resolver see it exactly — the fix must not turn every
  // ceiling into `undefined`, only the one nobody configured.
  const withCeiling = recordingResolver(oneRow);
  const bounded = await engine.queryGovernedAsync(local(), query, {
    deadlineMs: 60_000,
    maxIntermediateCells: 7,
    resolveService: withCeiling.resolveService,
  });
  assert.equal(bounded.isComplete, true);
  assert.equal(withCeiling.calls[0].ctx.maxIntermediateCells, 7n);
});

// ---------------------------------------------------------------------------
// LOAD and UPDATE
// ---------------------------------------------------------------------------

const DOC = `${EX}doc`;

test("LOAD resolves through resolveLoad", async () => {
  const engine = new QueryEngine();
  const answers = {
    text: async () => ({ text: `<${EX}s> <${EX}p> "from text" .`, mediaType: "text/turtle" }),
    bytes: async () => ({ bytes: encoder.encode(`<${EX}s> <${EX}p> "from bytes" .\n`), mediaType: "application/n-triples" }),
    dataset: async () => Dataset.parse(`<${EX}s> <${EX}p> "from a Dataset" .\n`, "nquads"),
    response: async () =>
      new Response(`<${EX}s> <${EX}p> "from a Response" .`, {
        headers: { "content-type": "text/turtle; charset=utf-8" },
      }),
  };
  const expected = {
    text: "from text",
    bytes: "from bytes",
    dataset: "from a Dataset",
    response: "from a Response",
  };
  for (const [shape, answer] of Object.entries(answers)) {
    const target = new Dataset();
    const requests = [];
    const returned = await engine.updateAsync(target, `LOAD <${DOC}> INTO GRAPH <${EX}g>`, {
      resolveLoad: (request, ctx) => {
        requests.push({ request, signal: ctx.signal });
        return answer();
      },
    });
    assert.equal(returned, target);
    assert.equal(requests.length, 1, shape);
    assert.deepEqual(requests[0].request, { kind: "load", iri: DOC });
    assert.ok(requests[0].signal instanceof AbortSignal);
    const loaded = engine.select(target, `SELECT ?o WHERE { GRAPH <${EX}g> { <${EX}s> <${EX}p> ?o } }`);
    assert.deepEqual(rowsOf(loaded), [`o=${expected[shape]}`], shape);
    assert.equal(target.generation, 1, "the committed LOAD advanced the generation once");
  }
});

test("LOAD without resolveLoad fails, SILENT or not", async () => {
  const engine = new QueryEngine();
  const target = Dataset.parse(`<${EX}a> <${EX}p> <${EX}o> .\n`, "nquads");
  const before = target.canonicalize();
  for (const load of [`LOAD <${DOC}>`, `LOAD SILENT <${DOC}>`]) {
    const error = await rejection(engine.updateAsync(target, load));
    const syncError = syncThrow(() => engine.update(Dataset.parse(`<${EX}a> <${EX}p> <${EX}o> .\n`, "nquads"), load));
    assert.equal(error.message, syncError.message, load);
    assert.match(error.message, /^error native-sparql-load-no-resolver: /, load);
    assert.equal(/SILENT does not apply/.test(error.message), load.includes("SILENT"), load);
    assert.equal(target.canonicalize(), before);
  }
  // The neighbour: with a resolveLoad whose document cannot be fetched, LOAD SILENT
  // succeeds with nothing fetched — the failure SILENT tolerates.
  const resolveLoad = async () => ({ kind: "transport", message: "the example.org host is down" });
  assert.equal(await engine.updateAsync(target, `LOAD SILENT <${DOC}>`, { resolveLoad }), target);
  assert.equal(target.canonicalize(), before);
  await rejection(engine.updateAsync(target, `LOAD <${DOC}>`, { resolveLoad }));
});

test("LOAD: a host policy denial reads distinctly from a catalog capability denial, and neither is silenced", async () => {
  const engine = new QueryEngine();

  // A bare host denial: no catalog anywhere, the host's own resolveLoad decides on its
  // own. Must never invent a catalog-capability cause — see the catalog neighbour below,
  // which keeps the "withholds the ... capability" wording because a real catalog is
  // what decided it.
  for (const silent of [false, true]) {
    const target = new Dataset();
    const error = await rejection(
      engine.updateAsync(target, silent ? `LOAD SILENT <${DOC}>` : `LOAD <${DOC}>`, {
        resolveLoad: async () => ({ kind: "denied", message: "tenant may not fetch" }),
      }),
    );
    assert.match(
      error.message,
      /LOAD <http:\/\/example\.org\/doc>: the host denied the request: tenant may not fetch/,
      `silent=${silent}`,
    );
    assert.doesNotMatch(error.message, /withholds the .* capability/, `silent=${silent}`);
  }

  // The neighbour: a real catalog refusal. `resolveLoad` consults
  // `ServiceCatalog.authorizeLoad` itself (the same pattern the Cloudflare LOAD resolver
  // uses) — the profile for DOC withholds the network capability, so the catalog's own
  // wording survives into the message, and it too is never silenced.
  const catalog = new ServiceCatalog();
  catalog.addService(DOC, JSON.stringify({ capabilities: ["query"] }));
  const resolveLoad = async (request) => {
    const authorization = catalog.authorizeLoad(request.iri);
    const denial = authorization.denial;
    authorization.free();
    if (denial !== undefined) return { kind: "denied", message: denial };
    throw new Error("unreachable: this test only exercises the denial branch");
  };
  for (const silent of [false, true]) {
    const target = new Dataset();
    const error = await rejection(
      engine.updateAsync(target, silent ? `LOAD SILENT <${DOC}>` : `LOAD <${DOC}>`, { resolveLoad }),
    );
    assert.match(error.message, /withholds the network capability/, `silent=${silent}`);
  }
});

test("a LOAD parse error is a LOAD failure", async () => {
  const engine = new QueryEngine();
  const resolveLoad = async () => ({ text: "this is not turtle", mediaType: "text/turtle" });
  const target = new Dataset();
  const error = await rejection(engine.updateAsync(target, `LOAD <${DOC}>`, { resolveLoad }));
  assert.match(error.message, /native-sparql-load-failed: LOAD <http:\/\/example\.org\/doc>: .*native-codec-parse/);
  assert.equal(target.size, 0);
  // The SILENT neighbour: the same unparseable document is swallowed — a no-op.
  const silentTarget = new Dataset();
  assert.equal(await engine.updateAsync(silentTarget, `LOAD SILENT <${DOC}>`, { resolveLoad }), silentTarget);
  assert.equal(silentTarget.size, 0);
  assert.equal(silentTarget.generation, 1, "the SILENT update committed (an empty change)");
});

test("INSERT WHERE federates through resolveService", async () => {
  const engine = new QueryEngine();
  const target = local();
  const mock = recordingResolver(async () => REMOTE_OX);
  await engine.updateAsync(
    target,
    `INSERT { ?s <${EX}x> ?x } WHERE { ?s <${EX}p> ?o . SERVICE <${EX}sparql> { ?o <${EX}q> ?x } }`,
    { resolveService: mock.resolveService },
  );
  assert.equal(mock.calls.length, 1);
  assert.match(mock.calls[0].request.queryText, /^SELECT/);
  assert.deepEqual(rowsOf(engine.select(target, `SELECT ?s ?x WHERE { ?s <${EX}x> ?x }`)), JOINED);
  // The neighbour: the synchronous update has no source and refuses.
  const syncError = syncThrow(() =>
    engine.update(local(), `INSERT { ?s <${EX}x> ?x } WHERE { ?s <${EX}p> ?o . SERVICE <${EX}sparql> { ?o <${EX}q> ?x } }`),
  );
  assert.match(syncError.message, /no remote query source configured/);
});

const FEDERATED_INSERT = `INSERT { ?s <${EX}x> ?x } WHERE { ?s <${EX}p> ?o . SERVICE <${EX}sparql> { ?o <${EX}q> ?x } }`;

test("dataset mutation during an async update is refused", async () => {
  const engine = new QueryEngine();
  const target = local();
  const deferred = deferredResolver();
  const pending = engine.updateAsync(target, FEDERATED_INSERT, { resolveService: deferred.resolveService });
  await turnUntil(() => deferred.pending.length === 1, "the update suspending on its SERVICE");
  target.delete(target.quads()[0]);
  assert.equal(target.generation, 1);
  deferred.pending[0].resolve(REMOTE_OX);
  const error = await rejection(pending);
  assert.equal(
    error.message,
    "dataset mutated while an asynchronous update was in flight (generation 0 → 1); the update was not applied",
  );
  assert.equal(target.size, 1, "only the synchronous deletion took effect");

  // The neighbour: a re-add of a quad already present changes nothing, so the generation
  // stays and the update applies.
  const untouched = local();
  const neighbourDeferred = deferredResolver();
  const neighbourPending = engine.updateAsync(untouched, FEDERATED_INSERT, {
    resolveService: neighbourDeferred.resolveService,
  });
  await turnUntil(() => neighbourDeferred.pending.length === 1, "the neighbour suspending");
  untouched.add(untouched.quads()[0]);
  assert.equal(untouched.generation, 0);
  neighbourDeferred.pending[0].resolve(REMOTE_OX);
  assert.equal(await neighbourPending, untouched);
  assert.equal(untouched.size, 4);
  assert.equal(untouched.generation, 1);
});

test("commitUpdate refuses a different dataset", async () => {
  const engine = new QueryEngine();
  const target = local();
  const other = local();
  const run = async () => {
    const options = new AsyncJobOptions();
    options.setHandlers(false, false);
    const job = engine.beginAsync(target, AsyncOperationKind.Update, `INSERT DATA { <${EX}n> <${EX}p> <${EX}o> }`, options);
    options.free();
    assert.equal(await runJob(job, {}), 0);
    return job;
  };
  const job = await run();
  try {
    const error = syncThrow(() => job.commitUpdate(other));
    assert.match(error.message, /commit targets a different dataset/);
    assert.equal(other.size, 2);
    assert.equal(other.generation, 0);
    // The neighbour: the dataset the job was begun on accepts the commit.
    job.commitUpdate(target);
    assert.equal(target.size, 3);
    assert.equal(target.generation, 1);
  } finally {
    job.finish();
    job.free();
  }
  assert.notEqual(target.id, other.id);
});

test("queries see a snapshot; mutation during a query is allowed", async () => {
  const engine = new QueryEngine();
  const data = local();
  const deferred = deferredResolver();
  const pending = engine.queryAsync(data, joinQuery(), { resolveService: deferred.resolveService });
  await turnUntil(() => deferred.pending.length === 1, "the query suspending on its SERVICE");
  // A third row whose object the remote side also knows — visible only to a later query.
  data.add(Dataset.parse(`<${EX}c> <${EX}p> <${EX}o1> .\n`, "nquads").quads()[0]);
  assert.equal(data.size, 3);
  assert.equal(data.generation, 1);
  deferred.pending[0].resolve(REMOTE_OX);
  assert.deepEqual(rowsOf(await pending), JOINED, "the in-flight query answered over its snapshot");
  // The neighbour: a query begun after the mutation sees the new row.
  const after = await engine.queryAsync(data, joinQuery(), { resolveService: async () => REMOTE_OX });
  assert.deepEqual(rowsOf(after), [...JOINED, `s=${EX}c&x=x1`].sort());
});

test("concurrent updates on one dataset are serialized", async () => {
  const engine = new QueryEngine();
  const insertAs = (predicate) =>
    `INSERT { ?s <${EX}${predicate}> ?x } WHERE { ?s <${EX}p> ?o . SERVICE <${EX}sparql> { ?o <${EX}q> ?x } }`;

  // One dataset: the second update is not even begun until the first has committed.
  const shared = local();
  const first = deferredResolver();
  const second = deferredResolver();
  const both = [
    engine.updateAsync(shared, insertAs("first"), { resolveService: first.resolveService }),
    engine.updateAsync(shared, insertAs("second"), { resolveService: second.resolveService }),
  ];
  await turnUntil(() => first.pending.length === 1, "the first update suspending");
  for (let turns = 0; turns < 50; turns += 1) await new Promise((resolve) => setImmediate(resolve));
  assert.equal(second.pending.length, 0, "the second update waits for the first");
  first.pending[0].resolve(REMOTE_OX);
  await turnUntil(() => second.pending.length === 1, "the second update starting after the first");
  second.pending[0].resolve(REMOTE_OX);
  await Promise.all(both);
  assert.equal(shared.generation, 2, "both applied, one generation each");
  assert.equal(shared.size, 6);

  // The neighbour: two datasets — both updates are in flight at once.
  const left = local();
  const right = local();
  const onLeft = deferredResolver();
  const onRight = deferredResolver();
  const pair = [
    engine.updateAsync(left, insertAs("first"), { resolveService: onLeft.resolveService }),
    engine.updateAsync(right, insertAs("second"), { resolveService: onRight.resolveService }),
  ];
  await turnUntil(
    () => onLeft.pending.length === 1 && onRight.pending.length === 1,
    "both updates suspending together",
  );
  onRight.pending[0].resolve(REMOTE_OX);
  onLeft.pending[0].resolve(REMOTE_OX);
  await Promise.all(pair);
  assert.equal(left.generation, 1);
  assert.equal(right.generation, 1);
});

// ---------------------------------------------------------------------------
// Hosts without JSPI or without a preferred yield primitive (child processes)
// ---------------------------------------------------------------------------

/** Run a fixture script in a child `node` with `--import <preload>`; its one JSON line. */
function runChild(preload, script) {
  const child = spawnSync(
    process.execPath,
    ["--import", fileURLToPath(new URL(`./fixtures/${preload}`, import.meta.url)), fileURLToPath(new URL(`./fixtures/${script}`, import.meta.url))],
    { encoding: "utf8", timeout: 120_000 },
  );
  assert.equal(child.status, 0, `the child exited ${child.status}: ${child.stderr}`);
  return JSON.parse(child.stdout.trim());
}

test("hasAsyncQueries reports JSPI", () => {
  assert.equal(typeof WebAssembly.Suspending, "function");
  assert.equal(typeof WebAssembly.promising, "function");
  assert.equal(hasAsyncQueries(), true);
});

test("async methods hard-fail without JSPI", async () => {
  const report = runChild("no-jspi-preload.mjs", "no-jspi-child.mjs");
  assert.equal(report.suspendingPresent, "undefined");
  assert.equal(report.promisingPresent, "undefined");
  assert.equal(report.hasAsyncQueries, false);
  assert.equal(report.noJspiMessage, NO_JSPI_MESSAGE);
  assert.equal(report.syncRows, 1, "the synchronous API is unaffected");
  for (const call of ["queryAsync", "updateAsync", "datasetQueryAsync", "beforeWasm"]) {
    assert.deepEqual(report[call], { settled: "rejected", name: "Error", message: NO_JSPI_MESSAGE }, call);
  }
  assert.equal(report.sizeAfter, 1);
  // The neighbour for "before touching wasm": here, with JSPI, the same bogus dataset
  // reaches wasm and is refused for what it is.
  const reached = await rejection(new QueryEngine().queryAsync({ notADataset: true }, "SELECT * WHERE { ?s ?p ?o }"));
  assert.notEqual(reached.message, NO_JSPI_MESSAGE);
});

test("the yield primitive falls back to MessageChannel", () => {
  const report = runChild("no-macrotask-preload.mjs", "message-channel-child.mjs");
  assert.equal(report.setImmediate, "undefined");
  assert.equal(report.scheduler, "undefined");
  assert.equal(report.hasAsyncQueries, true);
  assert.equal(report.primitive, "MessageChannel");
  // The long-query yielding assertions, unchanged, over the fallback primitive.
  assert.equal(report.syncCount, expectedCrossCount(1000));
  assert.equal(report.syncTicks, 0);
  assert.equal(report.asyncComplete, true);
  assert.equal(report.asyncCount, report.syncCount);
  assert.ok(report.asyncTicks > 0, `the interval ticked ${report.asyncTicks} times`);
  assert.ok(report.yields > 0);
});

// ---------------------------------------------------------------------------
// The governor corpus's federated vectors, through the asynchronous lane
// ---------------------------------------------------------------------------

const CORPUS = new URL("../../../../vectors/sparql-governors/", import.meta.url);
const corpusText = (path) => readFileSync(new URL(path, CORPUS), "utf8");

/** The rows of a `.tsv` sidecar, comments and blank lines dropped. */
function tsvRows(path) {
  return corpusText(path)
    .split("\n")
    .filter((line) => line.trim() !== "" && !line.startsWith("#"))
    .map((line) => line.split("\t"));
}

function escapeLexical(lexical) {
  return lexical.replace(/[\\"\n\r\t]/g, (character) => ({ "\\": "\\\\", '"': '\\"', "\n": "\\n", "\r": "\\r", "\t": "\\t" })[character]);
}

/** The corpus's term rendering (`render_term`), for the terms a federated case can bind. */
function renderTerm(term) {
  switch (term.termType) {
    case "NamedNode":
      return `<${term.value}>`;
    case "Literal": {
      const body = `"${escapeLexical(term.value)}"`;
      if (term.language !== "") return term.direction === "" ? `${body}@${term.language}` : `${body}@${term.language}--${term.direction}`;
      return `${body}^^<${term.datatype.value}>`;
    }
    case "Quad":
      return `<<( ${renderTerm(term.subject)} ${renderTerm(term.predicate)} ${renderTerm(term.object)} )>>`;
    default:
      throw new Error(`the federated vectors bind no ${term.termType}`);
  }
}

/** The corpus's `render_answer`, over a governed outcome from the package root. */
function renderAnswer(outcome) {
  const result = outcome.isComplete ? outcome.result : outcome.partial.result;
  let out = "";
  if (outcome.isComplete) {
    out += "outcome\tcomplete\n";
  } else {
    out += "outcome\tbudget-exhausted\n";
    const tripped = outcome.tripped;
    out +=
      tripped.kind === "stopped"
        ? `tripped\tstopped\t${tripped.cause === "deadline-exceeded" ? "deadline" : tripped.cause}\n`
        : `tripped\t${tripped.kind}\t${tripped.dimension}\t${tripped.label}\tlimit=${tripped.limit}\t${tripped.kind === "refused" ? `estimate=${tripped.estimate}` : `consumed=${tripped.consumed}`}\n`;
    out += `certificate\t${outcome.partial.certainty}\tpositional-prefix=${outcome.partial.isPositionalPrefix}\n`;
  }
  assert.equal(result.kind, "select", "the federated vectors are SELECT queries");
  out += `variables${result.variables.map((name) => `\t${name}`).join("")}\n`;
  for (const row of result.rows.toArray()) {
    out += `row${result.variables.map((name) => `\t${row[name] === undefined ? "UNBOUND" : renderTerm(row[name])}`).join("")}\n`;
  }
  return out;
}

test("governor vectors: service transport cases certify identically on the async lane", async () => {
  const engine = new QueryEngine();
  const transports = new Map(tsvRows("transport.tsv").map(([name, stopHandling, onFirstPost, posts]) => [name, { stopHandling, onFirstPost, posts: Number(posts) }]));
  const cases = tsvRows("manifest.tsv").filter(([name]) => name.startsWith("service-"));
  assert.equal(cases.length, 3, "the corpus's three service transport cases");
  const mismatches = [];
  for (const [name, dataPath, queryPath, source, governors] of cases) {
    assert.equal(source, "http", name);
    const transport = transports.get(name);
    assert.ok(transport !== undefined, `${name} has a transport row`);
    const data = Dataset.parse(corpusText(dataPath), "turtle");
    const sparql = corpusText(queryPath);
    // Endpoint responses are found from the query fixture's stem, keyed by the
    // endpoint IRI's last path segment — exactly as the corpus harness finds them.
    const stem = queryPath.replace(/^cases\//, "").replace(/\.rq$/, "");
    const responses = new Map(["a", "b"].map((key) => [key, readFileSync(new URL(`cases/${stem}.${key}.srj`, CORPUS))]));

    const options = {};
    const ceilings = new Map();
    for (const setting of governors.split(",")) {
      const [key, value] = setting.split("=");
      if (key === "remote-requests") {
        options.maxRemoteRequests = Number(value);
        ceilings.set("remote-requests", BigInt(value));
      } else if (key === "stop") {
        assert.equal(value, "cancellation", `${name}: a live cancellation signal`);
      } else {
        assert.fail(`${name}: unmapped governor ${setting}`);
      }
    }
    const controller = new AbortController();
    if (governors.includes("stop=cancellation")) options.signal = controller.signal;

    let posts = 0;
    options.resolveService = (request, ctx) => {
      posts += 1;
      // `cancel`: the exchange cancels the caller's signal, standing in for a host that
      // cancels while the evaluator waits on it.
      if (posts === 1 && transport.onFirstPost === "cancel") controller.abort();
      // `honours`: the transport reads the caller's signal and abandons the exchange —
      // it answers nothing until its own exchange signal is aborted; `ignores`: it never
      // looks and answers regardless.
      if (transport.stopHandling === "honours" && controller.signal.aborted) {
        return new Promise((resolve) => {
          ctx.signal.addEventListener("abort", () => resolve({ kind: "transport", message: "abandoned" }));
        });
      }
      const key = request.endpoint.slice(request.endpoint.lastIndexOf("/") + 1);
      return responses.get(key);
    };

    const outcome = await engine.queryGovernedAsync(data, sparql, options);
    const answer = renderAnswer(outcome);
    // The asynchronous lane cannot be deaf to a stop: the scheduler races every awaited
    // effect against the job's own signal and abandons the exchange before a byte of it
    // is delivered, whatever the host does. So a case whose transport ignores a stop is
    // answered here as its honouring counterpart is, and it is held to that counterpart's
    // pinned answer — which the corpus itself says differs from the deaf one only in the
    // positional-prefix claim the discarded exchange withdrew.
    let expectedName = name;
    if (transport.stopHandling === "ignores" && governors.includes("stop=")) {
      expectedName = name.replace("-deaf-transport-", "-honouring-transport-");
      assert.notEqual(expectedName, name, `${name} names its honouring counterpart`);
      const deafLines = corpusText(`expected/${name}.answer`).split("\n");
      const honouringLines = corpusText(`expected/${expectedName}.answer`).split("\n");
      const differing = deafLines.filter((line, index) => line !== honouringLines[index]);
      assert.deepEqual(
        differing,
        ["certificate\tcertain\tpositional-prefix=false"],
        `${name} and ${expectedName} differ only in the withdrawn positional-prefix claim`,
      );
    }
    const expected = corpusText(`expected/${expectedName}.answer`);
    if (answer !== expected) mismatches.push(`${name}.answer\n--- expected\n${expected}--- observed\n${answer}`);
    if (posts !== transport.posts) mismatches.push(`${name}: ${posts} exchanges, the corpus pins ${transport.posts}`);

    // The spend record: a dimension the case engages is pinned exactly; the package
    // meters every other dimension too (the corpus's harness leaves them unmetered and
    // records 0), so those are held to the case's full metered cost instead.
    const spend = new Map(tsvRows(`expected/${name}.spend`).map(([dimension, value]) => [dimension, BigInt(value)]));
    const metered = new Map(tsvRows(`expected/${name}.metered`).map(([dimension, value]) => [dimension, BigInt(value)]));
    for (const [dimension, pinned] of spend) {
      const consumed = outcome.evidence.consumed[dimension];
      if (ceilings.has(dimension)) {
        if (consumed !== pinned) mismatches.push(`${name}: ${dimension} spent ${consumed}, the corpus pins ${pinned}`);
      } else if (consumed > metered.get(dimension)) {
        mismatches.push(`${name}: ${dimension} spent ${consumed}, above the metered cost ${metered.get(dimension)}`);
      }
    }
    assert.equal(outcome.evidence.async.serviceEffects, transport.posts, `${name}: effects issued`);
  }
  assert.deepEqual(mismatches, []);
});

// ---------------------------------------------------------------------------
// Refusal pairs: each invalid input beside a valid neighbour whose result differs
// ---------------------------------------------------------------------------

test("refusal pair: yieldEveryPolls -1 is refused, 0 yields at every poll", async () => {
  const engine = new QueryEngine();
  const query = `SELECT ?s ?o WHERE { ?s <${EX}p> ?o }`;
  const refused = await rejection(engine.queryGovernedAsync(local(), query, { yieldEveryPolls: -1 }));
  assert.match(refused.message, /yieldEveryPolls must be an integer from 0 to 4294967295 inclusive, got -1/);
  const every = await engine.queryGovernedAsync(local(), query, { yieldEveryPolls: 0 });
  assert.equal(every.isComplete, true);
  assert.equal(every.result.rowCount, 2);
  assert.equal(every.evidence.async.yields, every.evidence.async.polls, "0 yields at every poll");
  assert.ok(every.evidence.async.yields > 0);
});

test("refusal pair: stackBytes 4096 is refused, 524288 answers", async () => {
  const engine = new QueryEngine();
  const query = `SELECT ?s ?o WHERE { ?s <${EX}p> ?o }`;
  const refused = await rejection(engine.queryAsync(local(), query, { stackBytes: 4096 }));
  assert.match(refused.message, /stackBytes must be an integer from 524288 to 4294967295 inclusive, got 4096/);
  const answered = await engine.queryGovernedAsync(local(), query, { stackBytes: 524288 });
  assert.equal(answered.result.rowCount, 2);
  assert.ok(answered.evidence.async.stackHighWaterBytes > 0);
  assert.ok(answered.evidence.async.stackHighWaterBytes < 524288);
});

test("refusal pair: a failure kind \"nope\" is a fault, \"denied\" is a denial", async () => {
  const engine = new QueryEngine();
  const nope = await rejection(
    engine.queryAsync(local(), joinQuery(true), { resolveService: async () => ({ kind: "nope", message: "?" }) }),
  );
  assert.match(nope.message, /^unknown failure kind "nope" for effect 1 \(expected "transport" or "denied"\)$/);
  const denied = await rejection(
    engine.queryAsync(local(), joinQuery(true), { resolveService: async () => ({ kind: "denied", message: "policy" }) }),
  );
  assert.match(denied.message, /SERVICE <.*>: the host denied the request: policy/);
  assert.doesNotMatch(denied.message, /withholds the .* capability/);
  assert.notEqual(nope.message, denied.message);
});

test("refusal pair: mediaType text/x-unknown fails the LOAD, text/turtle loads", async () => {
  const engine = new QueryEngine();
  const text = `<${EX}s> <${EX}p> "loaded" .`;
  const unknownTarget = new Dataset();
  const refused = await rejection(
    engine.updateAsync(unknownTarget, `LOAD <${DOC}>`, { resolveLoad: async () => ({ text, mediaType: "text/x-unknown" }) }),
  );
  assert.match(refused.message, /native-sparql-load-failed: LOAD <http:\/\/example\.org\/doc>: unsupported RDF format "text\/x-unknown"/);
  assert.equal(unknownTarget.size, 0);
  const turtleTarget = new Dataset();
  await engine.updateAsync(turtleTarget, `LOAD <${DOC}>`, { resolveLoad: async () => ({ text, mediaType: "text/turtle" }) });
  assert.equal(turtleTarget.size, 1);
});

test("refusal pair: the cancel option is refused, signal is accepted", async () => {
  const engine = new QueryEngine();
  const query = `SELECT ?s ?o WHERE { ?s <${EX}p> ?o }`;
  const refused = await rejection(engine.queryAsync(local(), query, { cancel: {} }));
  assert.ok(refused instanceof TypeError);
  assert.match(refused.message, /pass an AbortSignal as signal instead/);
  const answered = await engine.queryAsync(local(), query, { signal: new AbortController().signal });
  assert.equal(answered.rowCount, 2);
  // An already-aborted signal is honoured before the job begins.
  const already = AbortSignal.abort();
  assert.equal(await rejection(engine.queryAsync(local(), query, { signal: already })), already.reason);
});
