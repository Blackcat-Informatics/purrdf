// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Node real-execution tests for the `./cloudflare` subpath — `createFetchServiceResolver`,
// `createFetchLoadResolver` and `handleSparqlRequest` — and for the Rust protocol class
// underneath them (`SparqlProtocolRequest`) and the negotiated twin
// (`queryGovernedNegotiatedAsync`), against the actual optimized wasm module.
//
// No network is ever touched: every `fetch`, service binding, cache and `waitUntil` is a
// deterministic double that RECORDS what it was handed, so the assertions observe what
// the adapter actually sent. Every refusal is paired with a valid neighbour whose result
// observably differs. Wall-clock bounds are never asserted.

import { test } from "node:test";
import assert from "node:assert/strict";

import {
  Dataset,
  QueryEngine,
  ServiceCatalog,
  SparqlProtocolRequest,
  ready,
} from "../index.mjs";
import {
  createFetchLoadResolver,
  createFetchServiceResolver,
  handleSparqlRequest,
} from "../cloudflare.mjs";

await ready();

const EX = "http://example.org/";
const REMOTE_ORIGIN = "https://remote.example.org";
const REMOTE = `${REMOTE_ORIGIN}/sparql`;
const OTHER = "https://other.example.org/sparql";
const BASE_URL = "https://endpoint.example.org/sparql";
const encoder = new TextEncoder();
const decoder = new TextDecoder();

// Default graph: `ex:a ex:p ex:o1`, `ex:b ex:p ex:o2`. Two named graphs with one row each,
// so default-graph and named-graph selections are each distinguishable.
const TRIG = `
@prefix ex: <${EX}> .
ex:a ex:p ex:o1 .
ex:b ex:p ex:o2 .
graph ex:g1 { ex:c ex:p ex:inG1 . }
graph ex:g2 { ex:d ex:p ex:inG2 . }
`;
const dataset = () => Dataset.parse(TRIG, "trig");

const srj = (vars, rows) =>
  JSON.stringify({
    head: { vars },
    results: {
      bindings: rows.map((row) =>
        Object.fromEntries(
          Object.entries(row).map(([name, value]) => [
            name,
            value.startsWith("http") ? { type: "uri", value } : { type: "literal", value },
          ]),
        ),
      ),
    },
  });

const REMOTE_OX = srj(
  ["o", "x"],
  [
    { o: `${EX}o1`, x: "x1" },
    { o: `${EX}o2`, x: "x2" },
  ],
);

const FEDERATED = (endpoint = REMOTE) =>
  `SELECT ?s ?x WHERE { ?s <${EX}p> ?o . SERVICE <${endpoint}> { ?o <${EX}q> ?x } }`;

/** SPARQL Results JSON bytes/text → sorted `name=value&…` rows. */
function rowsOf(json) {
  const parsed = typeof json === "string" ? JSON.parse(json) : json;
  return parsed.results.bindings
    .map((binding) =>
      Object.keys(binding)
        .sort()
        .map((name) => `${name}=${binding[name].value}`)
        .join("&"),
    )
    .sort();
}

const catalogFor = (...entries) => {
  const catalog = new ServiceCatalog();
  for (const [endpoint, profile] of entries) catalog.addService(endpoint, JSON.stringify(profile));
  return catalog;
};
const QUERY_NETWORK = { capabilities: ["query", "network"] };

/** A fetch double that records `(url, init)` and answers with `answer(url, init)`. */
function recordingFetch(answer) {
  const calls = [];
  const fetch = async (url, init) => {
    calls.push({ url: String(url), init });
    return answer(url, init);
  };
  return { calls, fetch };
}

const srjResponse = (text = REMOTE_OX) =>
  new Response(text, { status: 200, headers: { "Content-Type": "application/sparql-results+json" } });

const neverFetch = async () => assert.fail("the global fetch must not be called");

/** A synthetic `SERVICE` request, as the scheduler hands one to `resolveService`. */
const serviceRequest = (overrides = {}) => ({
  kind: "service",
  endpoint: REMOTE,
  queryText: "SELECT * WHERE { ?o <http://example.org/q> ?x }",
  accept: "application/sparql-results+json",
  contentType: "application/sparql-query",
  userAgent: "purrdf-test/1",
  timeoutMs: 60_000,
  headers: [],
  ...overrides,
});
const liveCtx = () => ({ signal: new AbortController().signal });

/** A fetch double that never answers and rejects once its signal aborts. */
function hangingFetch() {
  const calls = [];
  const fetch = (url, init) =>
    new Promise((_, reject) => {
      calls.push({ url: String(url), init });
      init.signal.addEventListener("abort", () => reject(init.signal.reason));
    });
  return { calls, fetch };
}

/** A Cache API double recording every match and put. */
function fakeCache() {
  const store = new Map();
  const calls = [];
  return {
    store,
    calls,
    async match(request) {
      calls.push(["match", request.url]);
      return store.get(request.url)?.clone();
    },
    async put(request, response) {
      calls.push(["put", request.url, response.headers.get("Cache-Control")]);
      store.set(request.url, response);
    },
  };
}

/** A Cache API double whose `match` always rejects (e.g. workerd with no Cache configured). */
function rejectingMatchCache(error = new Error("No Cache was configured")) {
  const store = new Map();
  const calls = [];
  return {
    store,
    calls,
    async match(request) {
      calls.push(["match", request.url]);
      throw error;
    },
    async put(request, response) {
      calls.push(["put", request.url, response.headers.get("Cache-Control")]);
      store.set(request.url, response);
    },
  };
}

/** A Cache API double whose `put` always rejects (e.g. workerd with no Cache configured). */
function rejectingPutCache(error = new Error("No Cache was configured")) {
  const store = new Map();
  const calls = [];
  return {
    store,
    calls,
    async match(request) {
      calls.push(["match", request.url]);
      return store.get(request.url)?.clone();
    },
    async put(request) {
      calls.push(["put", request.url]);
      throw error;
    },
  };
}

/** An `onCacheError` double that records every `(error, context)` it was handed. */
function recordingCacheErrors() {
  const errors = [];
  const onCacheError = (error, context) => errors.push({ error, ...context });
  return { errors, onCacheError };
}

/** An `onInternalError` double that records every `(error, context)` it was handed. */
function recordingInternalErrors() {
  const errors = [];
  const onInternalError = (error, context) => errors.push({ error, ...context });
  return { errors, onInternalError };
}

function syncThrow(fn) {
  try {
    fn();
  } catch (error) {
    return error;
  }
  assert.fail("expected the call to throw");
}

async function rejection(promise) {
  try {
    await promise;
  } catch (error) {
    return error;
  }
  assert.fail("expected the promise to reject");
}

const GOVERNORS = { deadlineMs: 10_000 };
const SELECT_S = `SELECT ?s WHERE { ?s <${EX}p> ?o } ORDER BY ?s`;

/** Build an HTTP request to the endpoint. */
function httpRequest({ method = "GET", query, body, contentType, accept, origin, signal } = {}) {
  const url = query === undefined ? BASE_URL : `${BASE_URL}?${query}`;
  const headers = {};
  if (contentType !== undefined) headers["Content-Type"] = contentType;
  if (accept !== undefined) headers.Accept = accept;
  if (origin !== undefined) headers.Origin = origin;
  return new Request(url, { method, headers, body, signal });
}

const q = (text) => `query=${encodeURIComponent(text)}`;

async function problemOf(response) {
  assert.equal(response.headers.get("Content-Type"), "application/problem+json");
  const body = await response.json();
  assert.equal(body.type, "about:blank");
  assert.equal(body.status, response.status);
  assert.equal(typeof body.title, "string");
  assert.equal(typeof body.detail, "string");
  return body;
}

// ---------------------------------------------------------------------------
// SparqlProtocolRequest (the Rust protocol class)
// ---------------------------------------------------------------------------

test("SparqlProtocolRequest reads an operation and its dataset parameters", () => {
  const request = SparqlProtocolRequest.parse(
    "GET",
    undefined,
    `${q("SELECT * WHERE { ?s ?p ?o }")}&default-graph-uri=${encodeURIComponent(`${EX}g1`)}&x=1`,
    new Uint8Array(0),
  );
  try {
    assert.equal(request.kind, "query");
    assert.equal(request.text, "SELECT * WHERE { ?s ?p ?o }");
    assert.equal(request.resultKind, "solutions");
    assert.equal(request.hasDatasetParameters, true);
    assert.deepEqual(request.defaultGraphUris, [`${EX}g1`]);
    assert.deepEqual(request.namedGraphUris, []);
    assert.deepEqual(request.extraParameters, ["x", "1"]);
    assert.match(request.effectiveText(), new RegExp(`FROM <${EX}g1>`));
    assert.equal(request.negotiate(undefined), "json");
    assert.equal(request.negotiate("text/csv"), "csv");
    assert.equal(request.negotiate("text/html"), undefined);
  } finally {
    request.free();
  }
  const update = SparqlProtocolRequest.parse(
    "POST",
    "application/sparql-update",
    undefined,
    encoder.encode(`INSERT DATA { <${EX}s> <${EX}p> 1 }`),
  );
  try {
    assert.equal(update.kind, "update");
    assert.equal(update.resultKind, undefined);
    assert.throws(() => update.negotiate(undefined), /an update has no result format/);
  } finally {
    update.free();
  }
  assert.equal(SparqlProtocolRequest.formatMediaType("trig"), "application/trig");
  assert.equal(SparqlProtocolRequest.formatMediaType("html"), undefined);
  assert.deepEqual(SparqlProtocolRequest.offeredMediaTypes("dataset"), [
    "application/trig",
    "application/n-quads",
    "application/ld+json",
  ]);
  assert.throws(() => SparqlProtocolRequest.offeredMediaTypes("rows"), /unknown result kind/);
});

test("SparqlProtocolRequest reads an update's USING and USING NAMED parameters in request order", () => {
  const UPDATE = `DELETE { ?s <${EX}p> ?o } WHERE { ?s <${EX}p> ?o }`;
  // Out of lexical order, and split between the query string and a form body (which the
  // protocol reads in that order), so the getters can only pass by keeping request order.
  const using = (name, iri) => `${name}=${encodeURIComponent(iri)}`;
  const request = SparqlProtocolRequest.parse(
    "POST",
    "application/x-www-form-urlencoded",
    [using("using-graph-uri", `${EX}g2`), using("using-named-graph-uri", `${EX}n2`)].join("&"),
    encoder.encode(
      [
        `update=${encodeURIComponent(UPDATE)}`,
        using("using-named-graph-uri", `${EX}n1`),
        using("using-graph-uri", `${EX}g1`),
      ].join("&"),
    ),
  );
  try {
    assert.equal(request.kind, "update");
    assert.equal(request.hasDatasetParameters, true);
    assert.deepEqual(request.usingGraphUris, [`${EX}g2`, `${EX}g1`]);
    assert.deepEqual(request.usingNamedGraphUris, [`${EX}n2`, `${EX}n1`]);
    assert.deepEqual(request.defaultGraphUris, []);
    assert.deepEqual(request.namedGraphUris, []);
    // The parameters are the ones the effective text applies.
    const effective = request.effectiveText();
    for (const clause of [`USING <${EX}g2>`, `USING <${EX}g1>`, `USING NAMED <${EX}n2>`, `USING NAMED <${EX}n1>`]) {
      assert.ok(effective.includes(clause), `${clause} is missing from ${effective}`);
    }
  } finally {
    request.free();
  }
  // The neighbour: the same update without dataset parameters reads none.
  const bare = SparqlProtocolRequest.parse("POST", "application/sparql-update", undefined, encoder.encode(UPDATE));
  try {
    assert.equal(bare.kind, "update");
    assert.equal(bare.hasDatasetParameters, false);
    assert.deepEqual(bare.usingGraphUris, []);
    assert.deepEqual(bare.usingNamedGraphUris, []);
    assert.equal(bare.effectiveText().includes("USING"), false);
  } finally {
    bare.free();
  }
});

test("refusal pair: a protocol refusal is a typed Error with name, status and parameter", () => {
  const missing = syncThrow(() =>
    SparqlProtocolRequest.parse("GET", undefined, "x=1", new Uint8Array(0)),
  );
  assert.ok(missing instanceof Error);
  assert.equal(missing.name, "MissingOperation");
  assert.equal(missing.status, 400);
  assert.equal(missing.parameter, undefined);

  const badIri = syncThrow(() =>
    SparqlProtocolRequest.parse(
      "GET",
      undefined,
      `${q("SELECT * {}")}&named-graph-uri=relative`,
      new Uint8Array(0),
    ),
  );
  assert.equal(badIri.name, "InvalidGraphIri");
  assert.equal(badIri.parameter, "named-graph-uri");

  const method = syncThrow(() =>
    SparqlProtocolRequest.parse("DELETE", undefined, q("SELECT * {}"), new Uint8Array(0)),
  );
  assert.equal(method.status, 405);
  const media = syncThrow(() =>
    SparqlProtocolRequest.parse("POST", "text/plain", undefined, encoder.encode("ASK {}")),
  );
  assert.equal(media.status, 415);

  // The valid neighbour of each: the same request, well-formed, parses.
  const ok = SparqlProtocolRequest.parse(
    "GET",
    undefined,
    `${q("SELECT * {}")}&named-graph-uri=${encodeURIComponent(`${EX}g`)}`,
    new Uint8Array(0),
  );
  assert.deepEqual(ok.namedGraphUris, [`${EX}g`]);
  ok.free();
  SparqlProtocolRequest.parse("POST", "application/sparql-query", undefined, encoder.encode("ASK {}")).free();
});

test("refusal pair: effectiveText refuses a malformed operation, not a well-formed one", () => {
  const bad = SparqlProtocolRequest.parse("GET", undefined, q("SELEC * {}"), new Uint8Array(0));
  const error = syncThrow(() => bad.effectiveText());
  assert.equal(error.name, "MalformedOperation");
  assert.equal(error.status, 400);
  bad.free();
  const good = SparqlProtocolRequest.parse("GET", undefined, q("SELECT * {}"), new Uint8Array(0));
  assert.equal(good.effectiveText(), "SELECT * {}");
  good.free();
});

// ---------------------------------------------------------------------------
// queryGovernedNegotiatedAsync
// ---------------------------------------------------------------------------

test("the negotiated twin serializes by Accept and names the formats a dataset needs", async () => {
  const engine = new QueryEngine();
  const ds = dataset();
  const select = `SELECT ?s WHERE { ?s <${EX}p> ?o }`;
  const outcome = await engine.queryGovernedNegotiatedAsync(ds, select, { accept: "text/csv" });
  assert.equal(outcome.isComplete, true);
  assert.equal(outcome.body.format, "csv");
  assert.equal(outcome.body.mediaType, "text/csv");
  assert.equal(decoder.decode(outcome.body.bytes), engine.queryRaw(ds, select, { format: "csv" }));
  assert.equal(typeof outcome.evidence.async.evaluateMs, "number");

  const graphs = `CONSTRUCT { GRAPH <${EX}out> { ?s ?p ?o } } WHERE { ?s ?p ?o }`;
  const error = await rejection(
    engine.queryGovernedNegotiatedAsync(ds, graphs, { accept: "text/turtle" }),
  );
  assert.equal(error.name, "NotAcceptableError");
  assert.match(error.message, /application\/trig, application\/n-quads, application\/ld\+json/);
  // The neighbour that accepts TriG is sent TriG.
  const trig = await engine.queryGovernedNegotiatedAsync(ds, graphs, {
    accept: "text/turtle, application/trig;q=0.5",
  });
  assert.equal(trig.body.format, "trig");

  const capped = await engine.queryGovernedNegotiatedAsync(ds, select, { maxAnswers: 1 });
  assert.equal(capped.isComplete, false);
  assert.equal(capped.body, undefined);
  assert.equal(capped.tripped.label, "answer-cap-exhausted");
});

test("refusal pair: accept is refused by every twin but the negotiated one", async () => {
  const engine = new QueryEngine();
  const error = await rejection(
    engine.queryGovernedAsync(dataset(), SELECT_S, { accept: "text/csv" }),
  );
  assert.ok(error instanceof TypeError);
  assert.match(error.message, /unknown query option "accept"/);
  const ok = await engine.queryGovernedNegotiatedAsync(dataset(), SELECT_S, { accept: "text/csv" });
  assert.equal(ok.body.format, "csv");
});

// ---------------------------------------------------------------------------
// createFetchServiceResolver
// ---------------------------------------------------------------------------

test("refusal pair: the service resolver requires catalog and timeoutMs", () => {
  const catalog = catalogFor([REMOTE, QUERY_NETWORK]);
  const noCatalog = syncThrow(() => createFetchServiceResolver({ timeoutMs: 1000, fetch: neverFetch }));
  assert.ok(noCatalog instanceof TypeError);
  assert.match(noCatalog.message, /requires catalog/);
  const noTimeout = syncThrow(() => createFetchServiceResolver({ catalog, fetch: neverFetch }));
  assert.ok(noTimeout instanceof TypeError);
  assert.match(noTimeout.message, /requires timeoutMs/);
  const zero = syncThrow(() => createFetchServiceResolver({ catalog, timeoutMs: 0, fetch: neverFetch }));
  assert.match(zero.message, /positive integer/);
  const unknown = syncThrow(() =>
    createFetchServiceResolver({ catalog, timeoutMs: 1000, fetch: neverFetch, retries: 3 }),
  );
  assert.match(unknown.message, /unknown createFetchServiceResolver option "retries"/);
  assert.equal(
    typeof createFetchServiceResolver({ catalog, timeoutMs: 1000, fetch: neverFetch }),
    "function",
  );
});

test("refusal pair: cache options are all-or-nothing", () => {
  const catalog = catalogFor([REMOTE, QUERY_NETWORK]);
  const base = { catalog, timeoutMs: 1000, fetch: neverFetch };
  assert.match(
    syncThrow(() => createFetchServiceResolver({ ...base, cache: fakeCache() })).message,
    /cache requires cacheTtlSeconds/,
  );
  assert.match(
    syncThrow(() => createFetchServiceResolver({ ...base, cacheTtlSeconds: 60 })).message,
    /cacheTtlSeconds needs cache/,
  );
  assert.match(
    syncThrow(() => createFetchServiceResolver({ ...base, waitUntil: () => {} })).message,
    /waitUntil defers cache writes and needs cache/,
  );
  assert.match(
    syncThrow(() => createFetchServiceResolver({ ...base, cache: {}, cacheTtlSeconds: 60 })).message,
    /cache must be a Cache/,
  );
  assert.match(
    syncThrow(() => createFetchServiceResolver({ ...base, onCacheError: () => {} })).message,
    /onCacheError reports cache failures and needs cache/,
  );
  assert.match(
    syncThrow(() =>
      createFetchServiceResolver({ ...base, cache: fakeCache(), cacheTtlSeconds: 60, onCacheError: "nope" }),
    ).message,
    /onCacheError must be a function/,
  );
  assert.equal(
    typeof createFetchServiceResolver({
      ...base,
      cache: fakeCache(),
      cacheTtlSeconds: 60,
      waitUntil: () => {},
      onCacheError: () => {},
    }),
    "function",
  );
});

test("refusal pair: a bindings key must be an origin", () => {
  const catalog = catalogFor([REMOTE, QUERY_NETWORK]);
  const binding = { fetch: neverFetch };
  const error = syncThrow(() =>
    createFetchServiceResolver({
      catalog,
      timeoutMs: 1000,
      fetch: neverFetch,
      bindings: { [`${REMOTE_ORIGIN}/`]: binding },
    }),
  );
  assert.match(error.message, /must be an origin/);
  const notBinding = syncThrow(() =>
    createFetchServiceResolver({ catalog, timeoutMs: 1000, fetch: neverFetch, bindings: { [REMOTE_ORIGIN]: {} } }),
  );
  assert.match(notBinding.message, /must be a service binding/);
  assert.equal(
    typeof createFetchServiceResolver({
      catalog,
      timeoutMs: 1000,
      fetch: neverFetch,
      bindings: { [REMOTE_ORIGIN]: binding },
    }),
    "function",
  );
});

test("a request to a bound origin goes through its binding; any other through fetch", async () => {
  const catalog = catalogFor([REMOTE, QUERY_NETWORK], [OTHER, QUERY_NETWORK]);
  const binding = recordingFetch(() => srjResponse());
  const global = recordingFetch(() => srjResponse());
  const resolve = createFetchServiceResolver({
    catalog,
    timeoutMs: 1000,
    fetch: global.fetch,
    bindings: { [REMOTE_ORIGIN]: { fetch: binding.fetch } },
  });
  const bound = await resolve(serviceRequest(), liveCtx());
  assert.deepEqual(rowsOf(decoder.decode(bound)), rowsOf(REMOTE_OX));
  assert.equal(binding.calls.length, 1);
  assert.equal(global.calls.length, 0);
  assert.equal(binding.calls[0].url, REMOTE);
  assert.equal(binding.calls[0].init.method, "POST");
  assert.equal(binding.calls[0].init.body, serviceRequest().queryText);

  await resolve(serviceRequest({ endpoint: OTHER }), liveCtx());
  assert.equal(binding.calls.length, 1);
  assert.equal(global.calls.length, 1);
  assert.equal(global.calls[0].url, OTHER);
});

test("headers are sent protocol first, then the profile's, then the credential", async () => {
  const catalog = catalogFor([REMOTE, QUERY_NETWORK]);
  const { calls, fetch } = recordingFetch(() => srjResponse());
  const resolve = createFetchServiceResolver({ catalog, timeoutMs: 1000, fetch });
  await resolve(
    serviceRequest({
      headers: [
        ["X-Trace", "one"],
        ["X-Trace", "two"],
        ["X-Key", "secret"],
      ],
    }),
    liveCtx(),
  );
  assert.deepEqual(calls[0].init.headers, [
    ["Content-Type", "application/sparql-query"],
    ["Accept", "application/sparql-results+json"],
    ["User-Agent", "purrdf-test/1"],
    ["X-Trace", "one"],
    ["X-Trace", "two"],
    ["X-Key", "secret"],
  ]);
});

test("the effect's headers are the catalog profile's, then its credential, end to end", async () => {
  const catalog = catalogFor([
    REMOTE,
    {
      capabilities: ["query", "network", "credentials"],
      headers: [
        ["X-Trace", "one"],
        ["X-Trace", "two"],
      ],
      credential: { header: "X-Key", value: "k" },
      userAgent: "worker/1",
    },
  ]);
  const { calls, fetch } = recordingFetch(() => srjResponse());
  const resolveService = createFetchServiceResolver({ catalog, timeoutMs: 1000, fetch });
  const outcome = await new QueryEngine().queryGovernedAsync(dataset(), FEDERATED(), {
    resolveService,
    catalog,
  });
  assert.equal(outcome.isComplete, true);
  assert.deepEqual(calls[0].init.headers, [
    ["Content-Type", "application/sparql-query"],
    ["Accept", "application/sparql-results+json"],
    ["User-Agent", "worker/1"],
    ["X-Trace", "one"],
    ["X-Trace", "two"],
    ["X-Key", "k"],
  ]);
});

test("a request is abandoned at the shorter of the adapter's and the request's timeout", async () => {
  const catalog = catalogFor([REMOTE, QUERY_NETWORK]);
  const adapter = hangingFetch();
  const shortAdapter = createFetchServiceResolver({ catalog, timeoutMs: 20, fetch: adapter.fetch });
  assert.deepEqual(await shortAdapter(serviceRequest({ timeoutMs: 60_000 }), liveCtx()), {
    kind: "transport",
    message: `SERVICE <${REMOTE}>: no response within 20 ms`,
  });
  const profile = hangingFetch();
  const longAdapter = createFetchServiceResolver({ catalog, timeoutMs: 60_000, fetch: profile.fetch });
  assert.deepEqual(await longAdapter(serviceRequest({ timeoutMs: 25 }), liveCtx()), {
    kind: "transport",
    message: `SERVICE <${REMOTE}>: no response within 25 ms`,
  });
  // The query abandoning the request aborts it too, and is reported as such.
  const abandoned = hangingFetch();
  const resolve = createFetchServiceResolver({ catalog, timeoutMs: 60_000, fetch: abandoned.fetch });
  const controller = new AbortController();
  const pending = resolve(serviceRequest(), { signal: controller.signal });
  controller.abort();
  assert.deepEqual(await pending, {
    kind: "transport",
    message: `SERVICE <${REMOTE}>: abandoned by the query`,
  });
  // The neighbour: an endpoint that answers inside the bound is answered.
  const prompt = createFetchServiceResolver({
    catalog,
    timeoutMs: 20,
    fetch: async () => srjResponse(),
  });
  assert.deepEqual(rowsOf(decoder.decode(await prompt(serviceRequest(), liveCtx()))), rowsOf(REMOTE_OX));
});

test("a non-2xx answer is a transport failure and its body is cancelled, unread", async () => {
  const catalog = catalogFor([REMOTE, QUERY_NETWORK]);
  let cancelled = false;
  let pulled = false;
  const failing = async () =>
    new Response(
      new ReadableStream(
        {
          pull() {
            pulled = true;
          },
          cancel() {
            cancelled = true;
          },
        },
        { highWaterMark: 0 },
      ),
      { status: 503, statusText: "Busy" },
    );
  const resolve = createFetchServiceResolver({ catalog, timeoutMs: 1000, fetch: failing });
  assert.deepEqual(await resolve(serviceRequest(), liveCtx()), {
    kind: "transport",
    message: `SERVICE <${REMOTE}>: HTTP 503 Busy`,
  });
  assert.equal(cancelled, true);
  assert.equal(pulled, false);
  const network = createFetchServiceResolver({
    catalog,
    timeoutMs: 1000,
    fetch: async () => {
      throw new TypeError("fetch failed");
    },
  });
  assert.deepEqual(await network(serviceRequest(), liveCtx()), {
    kind: "transport",
    message: `SERVICE <${REMOTE}>: TypeError: fetch failed`,
  });
});

test("a transport failure fails a query, and SERVICE SILENT swallows it", async () => {
  const catalog = catalogFor([REMOTE, QUERY_NETWORK]);
  const resolveService = createFetchServiceResolver({
    catalog,
    timeoutMs: 1000,
    fetch: async () => new Response("down", { status: 502 }),
  });
  const engine = new QueryEngine();
  const error = await rejection(engine.queryAsync(dataset(), FEDERATED(), { resolveService, catalog }));
  assert.match(error.message, /HTTP 502/);
  const silent = await engine.selectAsync(
    dataset(),
    FEDERATED().replace("SERVICE <", "SERVICE SILENT <"),
    { resolveService, catalog },
  );
  assert.equal(silent.rowCount, 2);
});

// ---------------------------------------------------------------------------
// SERVICE and a redirecting endpoint: `redirect: "manual"`, never followed
// ---------------------------------------------------------------------------

/** Each row's `s`/`x` bindings, `UNBOUND` where a variable is not bound, sorted. */
function summarizeRows(rows) {
  return rows
    .toArray()
    .map((row) => `s=${row.s?.value ?? "UNBOUND"}&x=${row.x?.value ?? "UNBOUND"}`)
    .sort();
}

test("SERVICE redirect: a 302 to an unlisted origin is a transport failure, never fetched; SILENT matches the endpoint being down", async () => {
  const catalog = catalogFor([REMOTE, QUERY_NETWORK]);
  const evilOrigin = "https://evil.example.org";
  const { calls, fetch } = recordingFetch((url) => {
    assert.equal(String(url), REMOTE, "no request is ever sent anywhere but the catalogued endpoint");
    return new Response(null, { status: 302, headers: { Location: `${evilOrigin}/steal` } });
  });
  const resolveService = createFetchServiceResolver({ catalog, timeoutMs: 1000, fetch });
  const answer = await resolveService(serviceRequest(), liveCtx());
  assert.equal(answer.kind, "transport");
  assert.match(answer.message, /^SERVICE <https:\/\/remote\.example\.org\/sparql>: redirected \(HTTP 302 to https:\/\/evil\.example\.org\/steal\)/);
  assert.equal(calls.length, 1);
  assert.equal(calls[0].url, REMOTE);

  // The oracle: SERVICE SILENT over the redirecting endpoint returns exactly the same
  // rows as SERVICE SILENT over an endpoint that is simply down — proving the redirect
  // was refused outright, not partially honoured (which would bind `x` for some rows).
  const engine = new QueryEngine();
  const silentRedirected = await engine.selectAsync(
    dataset(),
    FEDERATED().replace("SERVICE <", "SERVICE SILENT <"),
    { resolveService, catalog },
  );
  const resolveDown = createFetchServiceResolver({
    catalog,
    timeoutMs: 1000,
    fetch: async () => new Response("down", { status: 502 }),
  });
  const silentDown = await engine.selectAsync(
    dataset(),
    FEDERATED().replace("SERVICE <", "SERVICE SILENT <"),
    { resolveService: resolveDown, catalog },
  );
  assert.equal(silentRedirected.rowCount, silentDown.rowCount);
  assert.deepEqual(summarizeRows(silentRedirected.rows), summarizeRows(silentDown.rows));
});

test("SERVICE redirect: a 307 is never followed — no body or header ever reaches the Location", async () => {
  const catalog = catalogFor([REMOTE, QUERY_NETWORK]);
  const { calls, fetch } = recordingFetch(() =>
    new Response(null, { status: 307, headers: { Location: `${REMOTE_ORIGIN}/elsewhere` } }),
  );
  const resolveService = createFetchServiceResolver({ catalog, timeoutMs: 1000, fetch });
  const answer = await resolveService(serviceRequest(), liveCtx());
  assert.equal(answer.kind, "transport");
  assert.match(answer.message, /redirected \(HTTP 307 to https:\/\/remote\.example\.org\/elsewhere\)/);
  assert.equal(calls.length, 1, "the 307's Location is never fetched — a re-sent 307 body would be a second call");
  assert.equal(calls[0].url, REMOTE);
  assert.equal(calls[0].init.body, serviceRequest().queryText, "only the catalogued endpoint ever saw the body");
});

test("SERVICE redirect: an opaque-redirect response is also a transport failure, its Location withheld", async () => {
  const catalog = catalogFor([REMOTE, QUERY_NETWORK]);
  const opaque = { type: "opaqueredirect", status: 0, ok: false, headers: new Headers(), body: null };
  const { calls, fetch } = recordingFetch(() => opaque);
  const resolveService = createFetchServiceResolver({ catalog, timeoutMs: 1000, fetch });
  const answer = await resolveService(serviceRequest(), liveCtx());
  assert.equal(answer.kind, "transport");
  assert.match(answer.message, /redirected \(HTTP opaque \(Location withheld by an opaque redirect\)\)/);
  assert.equal(calls.length, 1);
});

test("the cache answers a repeated request and stores through waitUntil", async () => {
  const catalog = catalogFor([REMOTE, QUERY_NETWORK]);
  const cache = fakeCache();
  const deferred = [];
  const { calls, fetch } = recordingFetch(() => srjResponse());
  const resolve = createFetchServiceResolver({
    catalog,
    timeoutMs: 1000,
    fetch,
    cache,
    cacheTtlSeconds: 120,
    waitUntil: (promise) => deferred.push(promise),
  });
  const miss = await resolve(serviceRequest(), liveCtx());
  assert.equal(calls.length, 1);
  assert.equal(deferred.length, 1, "the put was handed to waitUntil");
  await Promise.all(deferred);
  const [[matchKind, key], [putKind, putKey, cacheControl]] = cache.calls;
  assert.equal(matchKind, "match");
  assert.match(key, /^https:\/\/purrdf-service-cache\.invalid\/[0-9a-f]{64}$/);
  assert.equal(putKind, "put");
  assert.equal(putKey, key);
  assert.equal(cacheControl, "max-age=120");

  const hit = await resolve(serviceRequest(), liveCtx());
  assert.equal(calls.length, 1, "the hit never reached the network");
  assert.deepEqual(hit, miss);

  // The neighbour: a different query text is a different key, and a miss.
  await resolve(serviceRequest({ queryText: "SELECT * WHERE { ?o ?q ?x }" }), liveCtx());
  assert.equal(calls.length, 2);
  assert.notEqual(cache.calls.at(-2)[1], key);
});

test("without waitUntil the cache write is awaited", async () => {
  const catalog = catalogFor([REMOTE, QUERY_NETWORK]);
  const cache = fakeCache();
  const resolve = createFetchServiceResolver({
    catalog,
    timeoutMs: 1000,
    fetch: async () => srjResponse(),
    cache,
    cacheTtlSeconds: 5,
  });
  await resolve(serviceRequest(), liveCtx());
  assert.equal(cache.store.size, 1, "stored before the answer was returned");
});

test("refusal pair: a credentialed request is never cached; an anonymous one is", async () => {
  const catalog = catalogFor(
    [REMOTE, QUERY_NETWORK],
    [OTHER, { capabilities: ["query", "network", "credentials"], credential: { header: "X-Api-Key", value: "k" } }],
  );
  const cache = fakeCache();
  const { calls, fetch } = recordingFetch(() => srjResponse());
  const resolve = createFetchServiceResolver({ catalog, timeoutMs: 1000, fetch, cache, cacheTtlSeconds: 60 });
  for (const name of ["Authorization", "cookie", "Proxy-Authorization"]) {
    const error = await rejection(resolve(serviceRequest({ headers: [[name, "x"]] }), liveCtx()));
    assert.ok(error instanceof TypeError);
    assert.match(error.message, /never read from or written to a shared cache/);
  }
  // A credential whose header name is not credential-shaped is known from the catalog.
  const profiled = await rejection(
    resolve(serviceRequest({ endpoint: OTHER, headers: [["X-Api-Key", "k"]] }), liveCtx()),
  );
  assert.match(profiled.message, /the catalog profile's/);
  assert.equal(calls.length, 0);
  assert.equal(cache.calls.length, 0);
  // The neighbour: the same endpoint with a non-credential header is fetched and cached.
  await resolve(serviceRequest({ headers: [["X-Trace", "x"]] }), liveCtx());
  assert.equal(calls.length, 1);
  assert.equal(cache.store.size, 1);
});

test("a credentialed request through the cache is a job fault, even under SILENT", async () => {
  const catalog = catalogFor([
    REMOTE,
    { capabilities: ["query", "network", "credentials"], credential: { header: "Authorization", value: "Bearer t" } },
  ]);
  const resolveService = createFetchServiceResolver({
    catalog,
    timeoutMs: 1000,
    fetch: neverFetch,
    cache: fakeCache(),
    cacheTtlSeconds: 60,
  });
  const error = await rejection(
    new QueryEngine().queryAsync(dataset(), FEDERATED().replace("SERVICE <", "SERVICE SILENT <"), {
      resolveService,
      catalog,
    }),
  );
  assert.match(error.message, /shared cache/);
});

// ---------------------------------------------------------------------------
// A cache failure is an optimisation-layer fault, never the query's answer (gap G10)
// ---------------------------------------------------------------------------

test("a cache whose match rejects is treated as a miss: the query still answers with the remote rows, and the hook observes exactly one match error", async () => {
  const catalog = catalogFor([REMOTE, QUERY_NETWORK]);
  const cache = rejectingMatchCache();
  const { errors, onCacheError } = recordingCacheErrors();
  const { calls, fetch } = recordingFetch(() => srjResponse());
  const resolve = createFetchServiceResolver({
    catalog,
    timeoutMs: 1000,
    fetch,
    cache,
    cacheTtlSeconds: 60,
    onCacheError,
  });
  const answer = await resolve(serviceRequest(), liveCtx());
  assert.equal(calls.length, 1, "a rejecting match still lets the request reach the remote");
  assert.equal(errors.length, 1);
  assert.equal(errors[0].operation, "match");
  assert.equal(errors[0].endpoint, REMOTE);
  assert.ok(errors[0].error instanceof Error);

  // The oracle: byte-for-byte the same rows a resolver with no cache at all would answer.
  const noCache = createFetchServiceResolver({ catalog, timeoutMs: 1000, fetch: async () => srjResponse() });
  const plain = await noCache(serviceRequest(), liveCtx());
  assert.deepEqual(rowsOf(decoder.decode(answer)), rowsOf(decoder.decode(plain)));
});

test("a cache whose put rejects never discards the answer, with waitUntil: the hook observes the put error", async () => {
  const catalog = catalogFor([REMOTE, QUERY_NETWORK]);
  const cache = rejectingPutCache();
  const { errors, onCacheError } = recordingCacheErrors();
  const deferred = [];
  const resolve = createFetchServiceResolver({
    catalog,
    timeoutMs: 1000,
    fetch: async () => srjResponse(),
    cache,
    cacheTtlSeconds: 60,
    waitUntil: (promise) => deferred.push(promise),
    onCacheError,
  });
  const answer = await resolve(serviceRequest(), liveCtx());
  assert.deepEqual(rowsOf(decoder.decode(answer)), rowsOf(REMOTE_OX));
  assert.equal(deferred.length, 1, "the rejecting put is still handed to waitUntil");
  await Promise.all(deferred); // never rejects: the resolver attaches its own handler first
  assert.equal(errors.length, 1);
  assert.equal(errors[0].operation, "put");
  assert.equal(errors[0].endpoint, REMOTE);
});

test("a cache whose put rejects never discards the answer, without waitUntil: the hook observes the put error", async () => {
  const catalog = catalogFor([REMOTE, QUERY_NETWORK]);
  const cache = rejectingPutCache();
  const { errors, onCacheError } = recordingCacheErrors();
  const resolve = createFetchServiceResolver({
    catalog,
    timeoutMs: 1000,
    fetch: async () => srjResponse(),
    cache,
    cacheTtlSeconds: 60,
    onCacheError,
  });
  const answer = await resolve(serviceRequest(), liveCtx());
  assert.deepEqual(rowsOf(decoder.decode(answer)), rowsOf(REMOTE_OX));
  assert.equal(errors.length, 1);
  assert.equal(errors[0].operation, "put");
  assert.equal(errors[0].endpoint, REMOTE);
});

test("valid neighbour: a healthy cache still answers the second identical request from cache, with no reported error", async () => {
  const catalog = catalogFor([REMOTE, QUERY_NETWORK]);
  const cache = fakeCache();
  const { errors, onCacheError } = recordingCacheErrors();
  const { calls, fetch } = recordingFetch(() => srjResponse());
  const resolve = createFetchServiceResolver({
    catalog,
    timeoutMs: 1000,
    fetch,
    cache,
    cacheTtlSeconds: 60,
    onCacheError,
  });
  await resolve(serviceRequest(), liveCtx());
  await resolve(serviceRequest(), liveCtx());
  assert.equal(calls.length, 1, "the second identical request was served from the cache, not the remote");
  assert.equal(errors.length, 0, "a healthy cache never reports a cache error");
});

test("the default onCacheError writes one console.warn line naming the operation and endpoint", async () => {
  const catalog = catalogFor([REMOTE, QUERY_NETWORK]);
  const cache = rejectingMatchCache();
  const resolve = createFetchServiceResolver({
    catalog,
    timeoutMs: 1000,
    fetch: async () => srjResponse(),
    cache,
    cacheTtlSeconds: 60,
  });
  const calls = [];
  const original = console.warn;
  console.warn = (...args) => calls.push(args.join(" "));
  try {
    await resolve(serviceRequest(), liveCtx());
  } finally {
    console.warn = original;
  }
  assert.equal(calls.length, 1);
  assert.match(calls[0], /match/);
  assert.match(calls[0], new RegExp(REMOTE.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")));
});

test("SERVICE SILENT with the remote down and a rejecting cache yields the join identity, not a fault", async () => {
  const catalog = catalogFor([REMOTE, QUERY_NETWORK]);
  const down = async () => new Response("down", { status: 502 });
  const resolveRejectingCache = createFetchServiceResolver({
    catalog,
    timeoutMs: 1000,
    fetch: down,
    cache: rejectingMatchCache(),
    cacheTtlSeconds: 60,
  });
  const resolveNoCache = createFetchServiceResolver({ catalog, timeoutMs: 1000, fetch: down });
  const engine = new QueryEngine();
  const silentRejectingCache = await engine.selectAsync(
    dataset(),
    FEDERATED().replace("SERVICE <", "SERVICE SILENT <"),
    { resolveService: resolveRejectingCache, catalog },
  );
  const silentNoCache = await engine.selectAsync(
    dataset(),
    FEDERATED().replace("SERVICE <", "SERVICE SILENT <"),
    { resolveService: resolveNoCache, catalog },
  );
  assert.equal(silentRejectingCache.rowCount, silentNoCache.rowCount);
  assert.deepEqual(summarizeRows(silentRejectingCache.rows), summarizeRows(silentNoCache.rows));

  // The neighbour: without SILENT, the same setup still fails the query on the remote's own
  // failure — the cache's rejection never turns into the query's error, nor hides the real one.
  const error = await rejection(
    engine.queryAsync(dataset(), FEDERATED(), { resolveService: resolveRejectingCache, catalog }),
  );
  assert.match(error.message, /HTTP 502/);
});

// ---------------------------------------------------------------------------
// createFetchLoadResolver
// ---------------------------------------------------------------------------

const DOC = `${REMOTE_ORIGIN}/doc.ttl`;
const TTL = `<${EX}loaded> <${EX}p> <${EX}o> .\n`;

test("refusal pair: the load resolver requires catalog and timeoutMs and takes no cache", () => {
  const catalog = catalogFor([DOC, { capabilities: ["network"] }]);
  assert.match(syncThrow(() => createFetchLoadResolver({ timeoutMs: 1000 })).message, /requires catalog/);
  assert.match(syncThrow(() => createFetchLoadResolver({ catalog })).message, /requires timeoutMs/);
  assert.match(
    syncThrow(() => createFetchLoadResolver({ catalog, timeoutMs: 1000, cache: fakeCache() })).message,
    /unknown createFetchLoadResolver option "cache"/,
  );
  assert.equal(typeof createFetchLoadResolver({ catalog, timeoutMs: 1000, fetch: neverFetch }), "function");
});

test("refusal pair: a LOAD the catalog does not authorize is denied before any fetch", async () => {
  const catalog = catalogFor([DOC, { capabilities: ["network"], userAgent: "loader/1", headers: [["X-A", "1"]] }]);
  const { calls, fetch } = recordingFetch(
    () => new Response(TTL, { status: 200, headers: { "Content-Type": "text/turtle; charset=utf-8" } }),
  );
  const resolveLoad = createFetchLoadResolver({ catalog, timeoutMs: 1000, fetch });
  const denied = await resolveLoad({ kind: "load", iri: `${REMOTE_ORIGIN}/secret.ttl` }, liveCtx());
  assert.equal(denied.kind, "denied");
  assert.match(denied.message, /LOAD <https:\/\/remote\.example\.org\/secret\.ttl>/);
  assert.equal(calls.length, 0);

  const loaded = await resolveLoad({ kind: "load", iri: DOC }, liveCtx());
  assert.equal(loaded.mediaType, "text/turtle");
  assert.equal(loaded.base, DOC);
  assert.equal(decoder.decode(loaded.bytes), TTL);
  assert.equal(calls[0].init.method, "GET");
  const [accept, ...rest] = calls[0].init.headers;
  assert.equal(accept[0], "Accept");
  assert.match(accept[1], /text\/turtle/);
  assert.deepEqual(rest, [
    ["User-Agent", "loader/1"],
    ["X-A", "1"],
  ]);
});

test("a LOAD response without a Content-Type is a transport failure; with one it loads", async () => {
  const catalog = catalogFor([DOC, { capabilities: ["network"] }]);
  const bare = createFetchLoadResolver({
    catalog,
    timeoutMs: 1000,
    fetch: async () => new Response(new Blob([TTL]), { status: 200 }),
  });
  assert.deepEqual(await bare({ kind: "load", iri: DOC }, liveCtx()), {
    kind: "transport",
    message: `LOAD <${DOC}>: the response has no Content-Type`,
  });
  const resolveLoad = createFetchLoadResolver({
    catalog,
    timeoutMs: 1000,
    fetch: async () => new Response(TTL, { headers: { "Content-Type": "text/turtle" } }),
  });
  const target = new Dataset();
  await new QueryEngine().updateAsync(target, `LOAD <${DOC}>`, { resolveLoad });
  assert.equal(target.size, 1);
  const missing = await rejection(
    new QueryEngine().updateAsync(new Dataset(), `LOAD <${DOC}>`, { resolveLoad: bare }),
  );
  assert.match(missing.message, /no Content-Type/);
});

// ---------------------------------------------------------------------------
// createFetchLoadResolver and a redirecting document: manual, re-authorized hops
// ---------------------------------------------------------------------------

const SECOND_ORIGIN = "https://second.example.org";
const SECOND_DOC = `${SECOND_ORIGIN}/doc2.ttl`;
// A relative IRI: only correct if resolved against the redirected document's own URL.
const RELATIVE_TTL = `<rel> <${EX}p> <${EX}o> .\n`;

test("LOAD redirect: a redirect to an unlisted origin is denied, and the unlisted origin is never fetched", async () => {
  const catalog = catalogFor([DOC, { capabilities: ["network"] }]);
  const evilOrigin = "https://evil.example.org";
  const { calls, fetch } = recordingFetch((url) => {
    assert.equal(String(url), DOC, "only the authorized DOC is ever fetched");
    return new Response(null, { status: 302, headers: { Location: `${evilOrigin}/steal.ttl` } });
  });
  const resolveLoad = createFetchLoadResolver({ catalog, timeoutMs: 1000, fetch });
  const denied = await resolveLoad({ kind: "load", iri: DOC }, liveCtx());
  assert.equal(denied.kind, "denied");
  assert.match(denied.message, new RegExp(`LOAD <${evilOrigin}/steal\\.ttl>`));
  assert.equal(calls.length, 1);
  assert.equal(calls[0].url, DOC);
});

test("LOAD redirect: the valid neighbour — a redirect to a listed origin loads, and a relative IRI resolves against the final URL", async () => {
  const catalog = catalogFor(
    [DOC, { capabilities: ["network"] }],
    [SECOND_DOC, { capabilities: ["network"] }],
  );
  const { calls, fetch } = recordingFetch((url) => {
    if (String(url) === DOC) {
      return new Response(null, { status: 302, headers: { Location: SECOND_DOC } });
    }
    assert.equal(String(url), SECOND_DOC);
    return new Response(RELATIVE_TTL, { status: 200, headers: { "Content-Type": "text/turtle" } });
  });
  const resolveLoad = createFetchLoadResolver({ catalog, timeoutMs: 1000, fetch });
  const target = new Dataset();
  await new QueryEngine().updateAsync(target, `LOAD <${DOC}>`, { resolveLoad });
  assert.equal(target.size, 1);
  // The oracle: the document's relative `<rel>` IRI is `${SECOND_ORIGIN}/rel` only if it
  // was resolved against the redirected document's own URL — a base of the originally
  // requested DOC would instead (wrongly) resolve it under REMOTE_ORIGIN.
  const rows = new QueryEngine().select(target, `SELECT ?s WHERE { ?s <${EX}p> <${EX}o> }`);
  assert.equal(rows.rowCount, 1);
  assert.equal(rows.rows.toArray()[0].s.value, `${SECOND_ORIGIN}/rel`);
  assert.equal(calls.length, 2);
  assert.equal(calls[0].url, DOC);
  assert.equal(calls[1].url, SECOND_DOC);
});

test("LOAD redirect: a redirect loop fails as typed transport after exactly maxRedirects hops", async () => {
  // A fallback profile authorizes every origin, so only the hop limit — never a denial —
  // can stop this loop: each hop's URL is distinct (a growing query string) but always
  // redirects again.
  const catalog = new ServiceCatalog();
  catalog.setFallback(JSON.stringify({ capabilities: ["network"] }));
  const { calls, fetch } = recordingFetch((url) => {
    const next = `${DOC}?n=${calls.length}`;
    return new Response(null, { status: 302, headers: { Location: next } });
  });
  const resolveLoad = createFetchLoadResolver({ catalog, timeoutMs: 1000, fetch, maxRedirects: 3 });
  const answer = await resolveLoad({ kind: "load", iri: DOC }, liveCtx());
  assert.equal(answer.kind, "transport");
  assert.match(answer.message, /exceeded 3 redirect hops/);
  assert.equal(calls.length, 4, "the initial request plus exactly the 3 hops the limit allows");

  // The valid neighbour: the same loop with a taller limit follows more hops before
  // failing, proving the count is the limit's, not some other fixed constant.
  const taller = recordingFetch((url) => {
    const next = `${DOC}?n=${taller.calls.length}`;
    return new Response(null, { status: 302, headers: { Location: next } });
  });
  const generousLoad = createFetchLoadResolver({
    catalog,
    timeoutMs: 1000,
    fetch: taller.fetch,
    maxRedirects: 6,
  });
  await generousLoad({ kind: "load", iri: DOC }, liveCtx());
  assert.equal(taller.calls.length, 7);
});

test("refusal pair: maxRedirects must be a positive integer when given; without it, 5 is the default", () => {
  const catalog = catalogFor([DOC, { capabilities: ["network"] }]);
  assert.match(
    syncThrow(() => createFetchLoadResolver({ catalog, timeoutMs: 1000, fetch: neverFetch, maxRedirects: 0 }))
      .message,
    /maxRedirects must be a positive integer/,
  );
  assert.equal(
    typeof createFetchLoadResolver({ catalog, timeoutMs: 1000, fetch: neverFetch, maxRedirects: 2 }),
    "function",
  );
});

test("LOAD redirect: the redirected origin's own profile headers are sent, never the source's", async () => {
  const catalog = catalogFor(
    [DOC, { capabilities: ["network"], headers: [["X-Source", "source-only"]], userAgent: "source-agent/1" }],
    [SECOND_DOC, { capabilities: ["network"], headers: [["X-Target", "target-only"]], userAgent: "target-agent/1" }],
  );
  const { calls, fetch } = recordingFetch((url) =>
    String(url) === DOC
      ? new Response(null, { status: 302, headers: { Location: SECOND_DOC } })
      : new Response(TTL, { status: 200, headers: { "Content-Type": "text/turtle" } }),
  );
  const resolveLoad = createFetchLoadResolver({ catalog, timeoutMs: 1000, fetch });
  const loaded = await resolveLoad({ kind: "load", iri: DOC }, liveCtx());
  assert.equal(loaded.base, SECOND_DOC);
  assert.equal(calls.length, 2);
  const sourceHeaders = calls[0].init.headers;
  const targetHeaders = calls[1].init.headers;
  assert.ok(sourceHeaders.some(([name, value]) => name === "X-Source" && value === "source-only"));
  assert.ok(!sourceHeaders.some(([name]) => name === "X-Target"));
  assert.ok(targetHeaders.some(([name, value]) => name === "X-Target" && value === "target-only"));
  assert.ok(!targetHeaders.some(([name]) => name === "X-Source"));
  assert.ok(targetHeaders.some(([name, value]) => name === "User-Agent" && value === "target-agent/1"));
  assert.ok(!targetHeaders.some(([name, value]) => name === "User-Agent" && value === "source-agent/1"));
});

// ---------------------------------------------------------------------------
// handleSparqlRequest
// ---------------------------------------------------------------------------

test("refusal pair: handleSparqlRequest requires governors with deadlineMs", async () => {
  const base = { engine: new QueryEngine(), dataset: dataset() };
  const request = () => httpRequest({ query: q("ASK {}") });
  const noGovernors = await rejection(handleSparqlRequest(request(), base));
  assert.ok(noGovernors instanceof TypeError);
  assert.match(noGovernors.message, /requires governors/);
  const noDeadline = await rejection(handleSparqlRequest(request(), { ...base, governors: { fuel: 10 } }));
  assert.match(noDeadline.message, /requires deadlineMs/);
  const unknownGovernor = await rejection(
    handleSparqlRequest(request(), { ...base, governors: { deadlineMs: 1000, cancel: {} } }),
  );
  assert.match(unknownGovernor.message, /unknown handleSparqlRequest governors option "cancel"/);
  const unknownOption = await rejection(
    handleSparqlRequest(request(), { ...base, governors: GOVERNORS, format: "json" }),
  );
  assert.match(unknownOption.message, /unknown handleSparqlRequest option "format"/);
  const noEngine = await rejection(handleSparqlRequest(request(), { dataset: dataset(), governors: GOVERNORS }));
  assert.match(noEngine.message, /requires engine/);
  const response = await handleSparqlRequest(request(), { ...base, governors: GOVERNORS });
  assert.equal(response.status, 200);
  assert.deepEqual(await response.json(), { head: {}, boolean: true });
});

test("every request form answers", async () => {
  const engine = new QueryEngine();
  const options = { engine, dataset: dataset(), governors: GOVERNORS };
  const expected = rowsOf(engine.queryRaw(dataset(), SELECT_S));
  const forms = [
    httpRequest({ query: q(SELECT_S) }),
    httpRequest({ method: "POST", contentType: "application/sparql-query", body: SELECT_S }),
    httpRequest({
      method: "POST",
      contentType: "application/x-www-form-urlencoded",
      body: q(SELECT_S),
    }),
  ];
  for (const request of forms) {
    const response = await handleSparqlRequest(request, options);
    assert.equal(response.status, 200, request.method);
    assert.equal(response.headers.get("Content-Type"), "application/sparql-results+json");
    assert.deepEqual(rowsOf(await response.text()), expected);
  }

  const target = new Dataset();
  const update = await handleSparqlRequest(
    httpRequest({
      method: "POST",
      contentType: "application/sparql-update",
      body: `INSERT DATA { <${EX}n> <${EX}p> 1 }`,
    }),
    { engine, dataset: target, governors: GOVERNORS },
  );
  assert.equal(update.status, 204);
  assert.equal(await update.text(), "");
  assert.match(update.headers.get("Server-Timing"), /eval;dur=/);
  assert.equal(target.size, 1);
  const formUpdate = await handleSparqlRequest(
    httpRequest({
      method: "POST",
      contentType: "application/x-www-form-urlencoded",
      body: `update=${encodeURIComponent(`INSERT DATA { <${EX}m> <${EX}p> 2 }`)}`,
    }),
    { engine, dataset: target, governors: { ...GOVERNORS, maxAnswers: 5 } },
  );
  assert.equal(formUpdate.status, 204, "maxAnswers bounds queries and is not applied to an update");
  assert.equal(target.size, 2);
});

test("each Accept is answered in its format, byte for byte what queryRaw writes", async () => {
  const engine = new QueryEngine();
  const ds = dataset();
  const options = { engine, dataset: ds, governors: GOVERNORS };
  const CONSTRUCT = `CONSTRUCT WHERE { ?s <${EX}p> ?o }`;
  const cases = [
    [SELECT_S, undefined, "json", "application/sparql-results+json"],
    [SELECT_S, "*/*", "json", "application/sparql-results+json"],
    [SELECT_S, "application/sparql-results+xml", "xml", "application/sparql-results+xml"],
    [SELECT_S, "text/csv", "csv", "text/csv"],
    [SELECT_S, "text/tab-separated-values", "tsv", "text/tab-separated-values"],
    [SELECT_S, "text/csv;q=0.5, application/sparql-results+xml", "xml", "application/sparql-results+xml"],
    ["ASK { ?s ?p ?o }", undefined, "json", "application/sparql-results+json"],
    ["ASK { ?s ?p ?o }", "text/csv, application/sparql-results+xml;q=0.1", "xml", "application/sparql-results+xml"],
    [CONSTRUCT, undefined, "turtle", "text/turtle"],
    [CONSTRUCT, "application/n-triples", "ntriples", "application/n-triples"],
    [CONSTRUCT, "application/n-quads", "nquads", "application/n-quads"],
    [CONSTRUCT, "application/trig", "trig", "application/trig"],
    [CONSTRUCT, "application/ld+json", "jsonld", "application/ld+json"],
  ];
  for (const [query, accept, format, mediaType] of cases) {
    const response = await handleSparqlRequest(httpRequest({ query: q(query), accept }), options);
    assert.equal(response.status, 200, `${accept}`);
    assert.equal(response.headers.get("Content-Type"), mediaType);
    assert.equal(response.headers.get("Vary"), "Accept");
    assert.equal(await response.text(), engine.queryRaw(ds, query, { format }), `${accept}`);
  }
});

test("refusal pair: 406 before evaluation, and after it for named graphs", async () => {
  const engine = new QueryEngine();
  const options = { engine, dataset: dataset(), governors: GOVERNORS };
  const early = await handleSparqlRequest(httpRequest({ query: q(SELECT_S), accept: "text/html" }), options);
  assert.equal(early.status, 406);
  const earlyBody = await problemOf(early);
  assert.equal(earlyBody.code, "NotAcceptable");
  assert.deepEqual(earlyBody.offered, SparqlProtocolRequest.offeredMediaTypes("solutions"));
  assert.equal(early.headers.get("Server-Timing"), null, "nothing was evaluated");
  // CSV is defined for SELECT bindings only, so an ASK is never offered it.
  const ask = await handleSparqlRequest(httpRequest({ query: q("ASK {}"), accept: "text/csv" }), options);
  assert.equal(ask.status, 406);
  assert.deepEqual((await problemOf(ask)).offered, [
    "application/sparql-results+json",
    "application/sparql-results+xml",
  ]);
  const selectCsv = await handleSparqlRequest(httpRequest({ query: q(SELECT_S), accept: "text/csv" }), options);
  assert.equal(selectCsv.status, 200);
  assert.equal(selectCsv.headers.get("Content-Type"), "text/csv");

  const GRAPHS = `CONSTRUCT { GRAPH <${EX}out> { ?s ?p ?o } } WHERE { ?s ?p ?o }`;
  const late = await handleSparqlRequest(httpRequest({ query: q(GRAPHS), accept: "text/turtle" }), options);
  assert.equal(late.status, 406);
  const lateBody = await problemOf(late);
  assert.deepEqual(lateBody.offered, ["application/trig", "application/n-quads", "application/ld+json"]);
  assert.match(lateBody.detail, /named graphs/);

  // The neighbours: no preference gets TriG; a client accepting TriG gets it.
  const unstated = await handleSparqlRequest(httpRequest({ query: q(GRAPHS) }), options);
  assert.equal(unstated.status, 200);
  assert.equal(unstated.headers.get("Content-Type"), "application/trig");
  const trig = await handleSparqlRequest(
    httpRequest({ query: q(SELECT_S), accept: "text/html, application/sparql-results+json;q=0.1" }),
    options,
  );
  assert.equal(trig.status, 200);
});

test("refusal pair: protocol refusals are 400/405/415 problems; the valid neighbour is 200", async () => {
  const options = { engine: new QueryEngine(), dataset: dataset(), governors: GOVERNORS };
  const missing = await handleSparqlRequest(httpRequest({ query: "x=1" }), options);
  assert.equal(missing.status, 400);
  const missingBody = await problemOf(missing);
  assert.equal(missingBody.title, "Bad Request");
  assert.equal(missingBody.code, "MissingOperation");
  assert.equal(missingBody.parameter, undefined);

  const iri = await handleSparqlRequest(
    httpRequest({ query: `${q(SELECT_S)}&default-graph-uri=not-absolute` }),
    options,
  );
  assert.equal(iri.status, 400);
  const iriBody = await problemOf(iri);
  assert.equal(iriBody.code, "InvalidGraphIri");
  assert.equal(iriBody.parameter, "default-graph-uri");

  const malformed = await handleSparqlRequest(httpRequest({ query: q("SELEC ?s {}") }), options);
  assert.equal(malformed.status, 400);
  assert.equal((await problemOf(malformed)).code, "MalformedOperation");

  const conflict = await handleSparqlRequest(
    httpRequest({
      method: "POST",
      contentType: "application/sparql-update",
      query: `using-graph-uri=${encodeURIComponent(`${EX}g1`)}`,
      body: `WITH <${EX}g2> INSERT { ?s ?p ?o } WHERE { ?s ?p ?o }`,
    }),
    options,
  );
  assert.equal(conflict.status, 400);
  const conflictBody = await problemOf(conflict);
  assert.equal(conflictBody.code, "UsingConflictsWithClause");
  assert.equal(conflictBody.parameter, "using-graph-uri");

  const method = await handleSparqlRequest(httpRequest({ method: "PUT", body: "x" }), options);
  assert.equal(method.status, 405);
  assert.equal(method.headers.get("Allow"), "GET, POST");
  assert.equal((await problemOf(method)).code, "UnsupportedMethod");

  const media = await handleSparqlRequest(
    httpRequest({ method: "POST", contentType: "text/plain", body: SELECT_S }),
    options,
  );
  assert.equal(media.status, 415);
  assert.equal((await problemOf(media)).title, "Unsupported Media Type");

  const ok = await handleSparqlRequest(
    httpRequest({ method: "POST", contentType: "application/sparql-query", body: SELECT_S }),
    options,
  );
  assert.equal(ok.status, 200);
});

test("dataset parameters are honoured, and without them the query's own dataset answers", async () => {
  const options = { engine: new QueryEngine(), dataset: dataset(), governors: GOVERNORS };
  const OBJECTS = `SELECT ?o WHERE { ?s <${EX}p> ?o }`;
  const rows = async (query) => {
    const response = await handleSparqlRequest(httpRequest({ query }), options);
    assert.equal(response.status, 200);
    return rowsOf(await response.text());
  };
  assert.deepEqual(await rows(q(OBJECTS)), [`o=${EX}o1`, `o=${EX}o2`]);
  assert.deepEqual(
    await rows(`${q(OBJECTS)}&default-graph-uri=${encodeURIComponent(`${EX}g1`)}`),
    [`o=${EX}inG1`],
  );
  // The protocol's dataset overrides the query's own FROM.
  assert.deepEqual(
    await rows(`${q(OBJECTS.replace("WHERE", `FROM <${EX}g1> WHERE`))}&default-graph-uri=${encodeURIComponent(`${EX}g2`)}`),
    [`o=${EX}inG2`],
  );
  const NAMED = `SELECT ?g ?o WHERE { GRAPH ?g { ?s <${EX}p> ?o } }`;
  assert.deepEqual(await rows(q(NAMED)), [`g=${EX}g1&o=${EX}inG1`, `g=${EX}g2&o=${EX}inG2`]);
  assert.deepEqual(
    await rows(`${q(NAMED)}&named-graph-uri=${encodeURIComponent(`${EX}g2`)}`),
    [`g=${EX}g2&o=${EX}inG2`],
  );

  // An update's using-graph-uri reads its WHERE from that graph only.
  const target = dataset();
  const copy = `INSERT { <${EX}copy> <${EX}has> ?o } WHERE { ?s <${EX}p> ?o }`;
  const update = await handleSparqlRequest(
    httpRequest({
      method: "POST",
      contentType: "application/sparql-update",
      query: `using-graph-uri=${encodeURIComponent(`${EX}g2`)}`,
      body: copy,
    }),
    { ...options, dataset: target },
  );
  assert.equal(update.status, 204);
  const copied = JSON.parse(target.query(`SELECT ?o WHERE { <${EX}copy> <${EX}has> ?o }`));
  assert.deepEqual(rowsOf(copied), [`o=${EX}inG2`]);
  const neighbour = dataset();
  await handleSparqlRequest(
    httpRequest({ method: "POST", contentType: "application/sparql-update", body: copy }),
    { ...options, dataset: neighbour },
  );
  const unrestricted = JSON.parse(neighbour.query(`SELECT ?o WHERE { <${EX}copy> <${EX}has> ?o }`));
  assert.deepEqual(rowsOf(unrestricted), [`o=${EX}o1`, `o=${EX}o2`]);
});

test("the operation text is parsed exactly once on the request path: effectiveText runs only to splice dataset parameters, or to reclassify a failure", async () => {
  // A spy on the Rust-backed prototype method: it counts every call and still runs the
  // real implementation, so this observes exactly what handleSparqlRequest invokes without
  // changing what it computes.
  const original = SparqlProtocolRequest.prototype.effectiveText;
  let calls = 0;
  SparqlProtocolRequest.prototype.effectiveText = function spiedEffectiveText(...args) {
    calls += 1;
    return original.apply(this, args);
  };
  try {
    const options = { engine: new QueryEngine(), dataset: dataset(), governors: GOVERNORS };

    // No dataset parameters, well-formed: the text goes to the engine exactly as carried,
    // and the engine's own parse (which this module cannot observe) is the only one that
    // runs — effectiveText is never called.
    calls = 0;
    const plain = await handleSparqlRequest(httpRequest({ query: q(SELECT_S) }), options);
    assert.equal(plain.status, 200);
    assert.equal(calls, 0);

    // Dataset parameters: the FROM splice needs its own parse to rewrite the clause —
    // exactly one call, and it is still the only parse before the engine's.
    calls = 0;
    const withParams = await handleSparqlRequest(
      httpRequest({ query: `${q(SELECT_S)}&default-graph-uri=${encodeURIComponent(`${EX}g1`)}` }),
      options,
    );
    assert.equal(withParams.status, 200);
    assert.equal(calls, 1);

    // Malformed, no dataset parameters: negotiate's lightweight token scan sees a valid
    // query-form keyword and lets it through, so the engine's own parse is attempted first
    // and fails; effectiveText then runs once, on this failure path only, to reclassify the
    // rejection as the client's 400 rather than a bare 500 evaluation failure.
    calls = 0;
    const malformed = await handleSparqlRequest(httpRequest({ query: q("SELECT WHERE {") }), options);
    assert.equal(malformed.status, 400);
    assert.equal((await problemOf(malformed)).code, "MalformedOperation");
    assert.equal(calls, 1);

    // An update, no dataset parameters, well-formed: zero calls, exactly as the query case.
    calls = 0;
    const update = await handleSparqlRequest(
      httpRequest({
        method: "POST",
        contentType: "application/sparql-update",
        body: `INSERT DATA { <${EX}n> <${EX}p> 1 }`,
      }),
      { engine: new QueryEngine(), dataset: new Dataset(), governors: GOVERNORS },
    );
    assert.equal(update.status, 204);
    assert.equal(calls, 0);
  } finally {
    SparqlProtocolRequest.prototype.effectiveText = original;
  }
});

/** A POST request whose body streams as `chunks` arrive, with an explicit (possibly lying) header. */
function streamedRequest(chunks, { contentLength, contentType = "application/sparql-query" } = {}) {
  const headers = { "Content-Type": contentType };
  if (contentLength !== undefined) headers["Content-Length"] = String(contentLength);
  const body = new ReadableStream({
    start(controller) {
      for (const chunk of chunks) controller.enqueue(encoder.encode(chunk));
      controller.close();
    },
  });
  return new Request(BASE_URL, { method: "POST", headers, body, duplex: "half" });
}

test("refusal pair: maxRequestBytes must be a positive integer when given; without it, 1 MiB is the default", async () => {
  const base = { engine: new QueryEngine(), dataset: dataset(), governors: GOVERNORS };
  const request = () => httpRequest({ query: q("ASK {}") });
  for (const bad of [0, -1, 1.5, "1024"]) {
    const rejected = await rejection(handleSparqlRequest(request(), { ...base, maxRequestBytes: bad }));
    assert.ok(rejected instanceof TypeError);
    assert.match(rejected.message, /maxRequestBytes must be a positive integer/);
  }
  // The valid neighbour: a well-formed positive integer is accepted and answers normally.
  const response = await handleSparqlRequest(request(), { ...base, maxRequestBytes: 1024 });
  assert.equal(response.status, 200);
});

test("refusal pair: a body over maxRequestBytes is a 413, by Content-Length, by streaming past it with none, or with a lying smaller one; the valid neighbour answers at exactly the limit", async () => {
  const options = { engine: new QueryEngine(), dataset: dataset(), governors: GOVERNORS, maxRequestBytes: 32 };
  const over = "ASK { ?s <http://example.org/way-too-long-a-predicate-for-the-bound> ?o }";
  assert.ok(encoder.encode(over).length > 32, "the fixture must actually exceed the bound");

  // Content-Length above the bound: refused before a single byte is read. The fixture's
  // stream never enqueues, closes or errors — reading it would hang forever — so a prompt
  // answer (raced against a deadline no real read could meet) is proof the body was never
  // touched, not just that its data went unused.
  const byLength = new Request(BASE_URL, {
    method: "POST",
    headers: { "Content-Type": "application/sparql-query", "Content-Length": "9999" },
    body: new ReadableStream({ pull() {} }),
    duplex: "half",
  });
  const declared = await Promise.race([
    handleSparqlRequest(byLength, options),
    new Promise((_resolve, reject) =>
      setTimeout(
        () => reject(new Error("Content-Length must refuse the request without ever reading its body")),
        500,
      ),
    ),
  ]);
  assert.equal(declared.status, 413);
  const declaredBody = await problemOf(declared);
  assert.equal(declaredBody.title, "Content Too Large");
  assert.equal(declaredBody.code, "ContentTooLarge");
  assert.equal(declaredBody.limit, 32);

  // No Content-Length at all: the running byte count catches it as the body streams in.
  const streamed = await handleSparqlRequest(streamedRequest([over]), options);
  assert.equal(streamed.status, 413);
  assert.equal((await problemOf(streamed)).code, "ContentTooLarge");

  // A lying, understated Content-Length buys nothing: the same streaming bound still fires.
  const lied = await handleSparqlRequest(streamedRequest([over], { contentLength: 5 }), options);
  assert.equal(lied.status, 413);
  assert.equal((await problemOf(lied)).code, "ContentTooLarge");

  // The valid neighbour: a body of exactly maxRequestBytes bytes still answers, correctly.
  const filler = "x".repeat(32 - encoder.encode("ASK {} #").length);
  const askPadded = `ASK {} #${filler}`;
  assert.equal(encoder.encode(askPadded).length, 32);
  const atLimit = await handleSparqlRequest(
    httpRequest({ method: "POST", contentType: "application/sparql-query", body: askPadded }),
    options,
  );
  assert.equal(atLimit.status, 200);
  assert.deepEqual(await atLimit.json(), { head: {}, boolean: true });

  // And an ordinary request, far under the default bound, still answers normally.
  const ordinary = await handleSparqlRequest(httpRequest({ query: q(SELECT_S) }), {
    engine: new QueryEngine(),
    dataset: dataset(),
    governors: GOVERNORS,
  });
  assert.equal(ordinary.status, 200);
  assert.deepEqual(rowsOf(await ordinary.text()), [`s=${EX}a`, `s=${EX}b`]);
});

test("a deterministic ceiling is a 422 carrying its dimension; the unbounded neighbour is 200", async () => {
  const engine = new QueryEngine();
  const capped = await handleSparqlRequest(httpRequest({ query: q(SELECT_S) }), {
    engine,
    dataset: dataset(),
    governors: { ...GOVERNORS, maxAnswers: 1 },
  });
  assert.equal(capped.status, 422);
  assert.match(capped.headers.get("Server-Timing"), /freeze;dur=[0-9.]+, eval;dur=[0-9.]+, serialize;dur=/);
  const body = await problemOf(capped);
  assert.equal(body.title, "Unprocessable Content");
  assert.equal(body.code, "answer-cap-exhausted");
  assert.equal(body.dimension, "answer-rows");
  assert.equal(body.limit, 1);
  assert.equal(typeof body.consumed, "number");
  assert.equal(body.cause, undefined);

  const fuel = await handleSparqlRequest(httpRequest({ query: q(SELECT_S) }), {
    engine,
    dataset: dataset(),
    governors: { ...GOVERNORS, fuel: 1 },
  });
  assert.equal(fuel.status, 422);
  assert.equal((await problemOf(fuel)).dimension, "fuel");

  const open = await handleSparqlRequest(httpRequest({ query: q(SELECT_S) }), {
    engine,
    dataset: dataset(),
    governors: { ...GOVERNORS, maxAnswers: 2 },
  });
  assert.equal(open.status, 200);
  assert.equal(rowsOf(await open.text()).length, 2);
});

test("a deadline or a cancelled request is a 503 without Retry-After, not a 422", async () => {
  const engine = new QueryEngine();
  const catalog = catalogFor([REMOTE, QUERY_NETWORK]);
  // A resolver that answers only once the job abandons it.
  const stalled = (_request, ctx) =>
    new Promise((resolve) =>
      ctx.signal.addEventListener("abort", () => resolve({ kind: "transport", message: "late" })),
    );
  const deadline = await handleSparqlRequest(httpRequest({ query: q(FEDERATED()) }), {
    engine,
    dataset: dataset(),
    governors: { deadlineMs: 30 },
    resolveService: stalled,
    catalog,
  });
  assert.equal(deadline.status, 503);
  assert.equal(deadline.headers.get("Retry-After"), null);
  assert.match(deadline.headers.get("Server-Timing"), /service;dur=/);
  const deadlineBody = await problemOf(deadline);
  assert.equal(deadlineBody.title, "Service Unavailable");
  assert.equal(deadlineBody.code, "deadline-exceeded");
  assert.equal(deadlineBody.cause, "deadline-exceeded");
  assert.equal(deadlineBody.dimension, undefined);

  const controller = new AbortController();
  const cancelled = await handleSparqlRequest(
    httpRequest({ query: q(FEDERATED()), signal: controller.signal }),
    {
      engine,
      dataset: dataset(),
      governors: GOVERNORS,
      resolveService: (request, ctx) => {
        controller.abort();
        return stalled(request, ctx);
      },
      catalog,
    },
  );
  assert.equal(cancelled.status, 503);
  assert.equal((await problemOf(cancelled)).cause, "cancelled");

  // The neighbour: the same federated request, answered, is a 200.
  const answered = await handleSparqlRequest(httpRequest({ query: q(FEDERATED()) }), {
    engine,
    dataset: dataset(),
    governors: GOVERNORS,
    resolveService: async () => REMOTE_OX,
    catalog,
  });
  assert.equal(answered.status, 200);
});

test("an evaluation failure is a 500 problem", async () => {
  const response = await handleSparqlRequest(httpRequest({ query: q(FEDERATED()) }), {
    engine: new QueryEngine(),
    dataset: dataset(),
    governors: GOVERNORS,
  });
  assert.equal(response.status, 500);
  const body = await problemOf(response);
  assert.equal(body.title, "Internal Server Error");
  assert.match(body.detail, /no remote query source configured/);
});

// ---------------------------------------------------------------------------
// A host bug — a resolveService/resolveLoad exception, or any other unclassified
// exception — never reaches the client as its own words (gap G4, information exposure
// through a stack trace). A typed engine error (the case above, and a tripped governor)
// is the oracle this is a neighbour of: it still gets its own real detail.
// ---------------------------------------------------------------------------

test("refusal pair: a resolveService that throws is a 500 with a correlation id, never the exception's own words; a healthy resolver is the 200 neighbour and the hook is never called for it", async () => {
  const catalog = catalogFor([REMOTE, QUERY_NETWORK]);
  const { errors, onInternalError } = recordingInternalErrors();
  const buggyResolver = async () => {
    throw new Error("secret-token-abc at /internal/path");
  };
  const response = await handleSparqlRequest(httpRequest({ query: q(FEDERATED()) }), {
    engine: new QueryEngine(),
    dataset: dataset(),
    governors: GOVERNORS,
    resolveService: buggyResolver,
    catalog,
    onInternalError,
  });
  assert.equal(response.status, 500);
  const raw = await response.clone().text();
  assert.doesNotMatch(raw, /secret-token-abc/);
  assert.doesNotMatch(raw, /internal\/path/);
  assert.doesNotMatch(raw, /\bat [A-Za-z]/, "no V8 stack frame ever reaches the body");
  const body = await problemOf(response);
  assert.equal(body.code, "InternalError");
  assert.equal(typeof body.correlationId, "string");
  assert.match(body.detail, new RegExp(body.correlationId));
  assert.equal(errors.length, 1, "the hook observed exactly one internal error");
  assert.equal(errors[0].error.message, "secret-token-abc at /internal/path", "the hook got the real error");
  assert.equal(errors[0].correlationId, body.correlationId, "the client and the log share one id");

  // The valid neighbour: the identical federated query, answered by a healthy resolver,
  // is a plain 200 — and never calls the hook at all.
  const healthy = await handleSparqlRequest(httpRequest({ query: q(FEDERATED()) }), {
    engine: new QueryEngine(),
    dataset: dataset(),
    governors: GOVERNORS,
    resolveService: async () => REMOTE_OX,
    catalog,
    onInternalError,
  });
  assert.equal(healthy.status, 200);
  assert.equal(errors.length, 1, "the healthy neighbour never touches onInternalError");
});

test("SERVICE SILENT does not hide a resolveService bug: still a 500 with a correlation id, never a quietly incomplete 200; the valid neighbour is a genuine typed transport failure, which SILENT still swallows to the join identity", async () => {
  const catalog = catalogFor([REMOTE, QUERY_NETWORK]);
  const { errors, onInternalError } = recordingInternalErrors();
  const silentQuery = q(FEDERATED().replace("SERVICE <", "SERVICE SILENT <"));

  // A host bug is about this endpoint's own correctness, never the remote's, so SILENT —
  // a promise about the remote, "it may be unreachable" — must not swallow it: the query
  // still rejects, exactly as it would without SILENT, just with the secret replaced.
  const buggy = await handleSparqlRequest(httpRequest({ query: silentQuery }), {
    engine: new QueryEngine(),
    dataset: dataset(),
    governors: GOVERNORS,
    resolveService: async () => {
      throw new Error("secret-token-abc at /internal/path");
    },
    catalog,
    onInternalError,
  });
  assert.equal(buggy.status, 500);
  assert.doesNotMatch(await buggy.clone().text(), /secret-token-abc/);
  const body = await problemOf(buggy);
  assert.equal(body.code, "InternalError");
  assert.equal(errors.length, 1);
  assert.equal(errors[0].error.message, "secret-token-abc at /internal/path");
  assert.equal(errors[0].correlationId, body.correlationId);

  // The valid neighbour: a genuine, deliberately typed transport failure under the same
  // SILENT clause is exactly what SILENT promises — the join identity, a plain 200 — and
  // never touches onInternalError, which is for host bugs, not remote ones.
  const genuine = await handleSparqlRequest(httpRequest({ query: silentQuery }), {
    engine: new QueryEngine(),
    dataset: dataset(),
    governors: GOVERNORS,
    resolveService: async () => ({ kind: "transport", message: "the endpoint is down" }),
    catalog,
    onInternalError,
  });
  assert.equal(genuine.status, 200);
  assert.deepEqual(rowsOf(await genuine.text()), [`s=${EX}a`, `s=${EX}b`]);
  assert.equal(errors.length, 1, "a genuine transport failure never touches onInternalError");
});

test("the default onInternalError hook writes one console.error line carrying the original error and the correlation id", async () => {
  const catalog = catalogFor([REMOTE, QUERY_NETWORK]);
  const calls = [];
  const original = console.error;
  console.error = (...args) => calls.push(args);
  try {
    const response = await handleSparqlRequest(httpRequest({ query: q(FEDERATED()) }), {
      engine: new QueryEngine(),
      dataset: dataset(),
      governors: GOVERNORS,
      resolveService: async () => {
        throw new Error("boom");
      },
      catalog,
    });
    const body = await problemOf(response);
    assert.equal(calls.length, 1);
    assert.ok(calls[0][0] instanceof Error);
    assert.equal(calls[0][0].message, "boom");
    assert.equal(calls[0][1], body.correlationId);
  } finally {
    console.error = original;
  }
});

test("an unexpected exception anywhere else in the adapter is also a 500 with a correlation id, never a bare rejection", async () => {
  const original = SparqlProtocolRequest.prototype.negotiate;
  SparqlProtocolRequest.prototype.negotiate = function negotiate() {
    throw new Error("a bug in this adapter, not a protocol refusal");
  };
  const { errors, onInternalError } = recordingInternalErrors();
  try {
    const response = await handleSparqlRequest(httpRequest({ query: q(SELECT_S) }), {
      engine: new QueryEngine(),
      dataset: dataset(),
      governors: GOVERNORS,
      onInternalError,
    });
    assert.equal(response.status, 500);
    const body = await problemOf(response);
    assert.equal(body.code, "InternalError");
    assert.doesNotMatch(body.detail, /bug in this adapter/);
    assert.equal(errors.length, 1);
    assert.equal(errors[0].error.message, "a bug in this adapter, not a protocol refusal");
    assert.equal(errors[0].correlationId, body.correlationId);
  } finally {
    SparqlProtocolRequest.prototype.negotiate = original;
  }

  // The valid neighbour: with `negotiate` restored, the identical request answers 200.
  const restored = await handleSparqlRequest(httpRequest({ query: q(SELECT_S) }), {
    engine: new QueryEngine(),
    dataset: dataset(),
    governors: GOVERNORS,
  });
  assert.equal(restored.status, 200);
});

test("the catalog denies an unlisted endpoint before any fetch; a listed one is fetched", async () => {
  const catalog = catalogFor([REMOTE, QUERY_NETWORK]);
  const { calls, fetch } = recordingFetch(() => srjResponse());
  const options = {
    engine: new QueryEngine(),
    dataset: dataset(),
    governors: GOVERNORS,
    catalog,
    resolveService: createFetchServiceResolver({ catalog, timeoutMs: 1000, fetch }),
  };
  const denied = await handleSparqlRequest(httpRequest({ query: q(FEDERATED(OTHER)) }), options);
  assert.equal(denied.status, 500);
  assert.match((await problemOf(denied)).detail, /no profile is configured for this service/);
  assert.equal(calls.length, 0);
  const listed = await handleSparqlRequest(httpRequest({ query: q(FEDERATED()) }), options);
  assert.equal(listed.status, 200);
  assert.equal(calls.length, 1);
});

test("a federated query runs end to end through a service binding", async () => {
  const catalog = catalogFor([REMOTE, QUERY_NETWORK]);
  const binding = recordingFetch(async (_url, init) => {
    assert.match(init.body, /\?o <http:\/\/example\.org\/q> \?x/);
    return srjResponse();
  });
  const response = await handleSparqlRequest(httpRequest({ query: q(FEDERATED()) }), {
    engine: new QueryEngine(),
    dataset: dataset(),
    governors: { ...GOVERNORS, maxRemoteRequests: 4 },
    catalog,
    resolveService: createFetchServiceResolver({
      catalog,
      timeoutMs: 1000,
      fetch: neverFetch,
      bindings: { [REMOTE_ORIGIN]: { fetch: binding.fetch } },
    }),
  });
  assert.equal(response.status, 200);
  assert.deepEqual(rowsOf(await response.text()), [`s=${EX}a&x=x1`, `s=${EX}b&x=x2`]);
  assert.equal(binding.calls.length, 1);
  assert.match(response.headers.get("Server-Timing"), /service;dur=/);
});

test("CORS: preflight, echo and Vary with cors; no CORS header and a 405 OPTIONS without", async () => {
  const base = { engine: new QueryEngine(), dataset: dataset(), governors: GOVERNORS };
  const cors = { origins: ["https://app.example.org"], maxAgeSeconds: 600 };
  const preflight = await handleSparqlRequest(
    httpRequest({ method: "OPTIONS", origin: "https://app.example.org" }),
    { ...base, cors },
  );
  assert.equal(preflight.status, 204);
  assert.equal(preflight.headers.get("Access-Control-Allow-Origin"), "https://app.example.org");
  assert.equal(preflight.headers.get("Access-Control-Allow-Methods"), "GET, POST, OPTIONS");
  assert.equal(preflight.headers.get("Access-Control-Allow-Headers"), "Content-Type, Accept");
  assert.equal(preflight.headers.get("Access-Control-Max-Age"), "600");
  assert.equal(preflight.headers.get("Vary"), "Origin");

  const unlisted = await handleSparqlRequest(
    httpRequest({ method: "OPTIONS", origin: "https://evil.example.org" }),
    { ...base, cors },
  );
  assert.equal(unlisted.status, 204);
  assert.equal(unlisted.headers.get("Access-Control-Allow-Origin"), null);

  const echoed = await handleSparqlRequest(
    httpRequest({ query: q(SELECT_S), origin: "https://app.example.org" }),
    { ...base, cors },
  );
  assert.equal(echoed.status, 200);
  assert.equal(echoed.headers.get("Access-Control-Allow-Origin"), "https://app.example.org");
  assert.equal(echoed.headers.get("Vary"), "Accept, Origin");

  const star = await handleSparqlRequest(
    httpRequest({ query: q(SELECT_S), origin: "https://any.example.org" }),
    { ...base, cors: { origins: "*" } },
  );
  assert.equal(star.headers.get("Access-Control-Allow-Origin"), "*");

  const problemWithCors = await handleSparqlRequest(
    httpRequest({ query: "x=1", origin: "https://app.example.org" }),
    { ...base, cors },
  );
  assert.equal(problemWithCors.status, 400);
  assert.equal(problemWithCors.headers.get("Access-Control-Allow-Origin"), "https://app.example.org");

  const plain = await handleSparqlRequest(
    httpRequest({ query: q(SELECT_S), origin: "https://app.example.org" }),
    base,
  );
  assert.equal(plain.status, 200);
  for (const [name] of plain.headers) assert.ok(!name.startsWith("access-control-"), name);
  assert.equal(plain.headers.get("Vary"), "Accept");
  const options = await handleSparqlRequest(
    httpRequest({ method: "OPTIONS", origin: "https://app.example.org" }),
    base,
  );
  assert.equal(options.status, 405);
  assert.equal(options.headers.get("Allow"), "GET, POST");

  const badCors = await rejection(
    handleSparqlRequest(httpRequest({ query: q(SELECT_S) }), { ...base, cors: { origins: "https://app.example.org" } }),
  );
  assert.match(badCors.message, /cors.origins must be "\*" or an array/);
});
