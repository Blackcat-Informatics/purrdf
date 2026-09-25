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
  assert.equal(
    typeof createFetchServiceResolver({
      ...base,
      cache: fakeCache(),
      cacheTtlSeconds: 60,
      waitUntil: () => {},
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
  const cors = { origins: ["https://app.example"], maxAgeSeconds: 600 };
  const preflight = await handleSparqlRequest(
    httpRequest({ method: "OPTIONS", origin: "https://app.example" }),
    { ...base, cors },
  );
  assert.equal(preflight.status, 204);
  assert.equal(preflight.headers.get("Access-Control-Allow-Origin"), "https://app.example");
  assert.equal(preflight.headers.get("Access-Control-Allow-Methods"), "GET, POST, OPTIONS");
  assert.equal(preflight.headers.get("Access-Control-Allow-Headers"), "Content-Type, Accept");
  assert.equal(preflight.headers.get("Access-Control-Max-Age"), "600");
  assert.equal(preflight.headers.get("Vary"), "Origin");

  const unlisted = await handleSparqlRequest(
    httpRequest({ method: "OPTIONS", origin: "https://evil.example" }),
    { ...base, cors },
  );
  assert.equal(unlisted.status, 204);
  assert.equal(unlisted.headers.get("Access-Control-Allow-Origin"), null);

  const echoed = await handleSparqlRequest(
    httpRequest({ query: q(SELECT_S), origin: "https://app.example" }),
    { ...base, cors },
  );
  assert.equal(echoed.status, 200);
  assert.equal(echoed.headers.get("Access-Control-Allow-Origin"), "https://app.example");
  assert.equal(echoed.headers.get("Vary"), "Accept, Origin");

  const star = await handleSparqlRequest(
    httpRequest({ query: q(SELECT_S), origin: "https://any.example" }),
    { ...base, cors: { origins: "*" } },
  );
  assert.equal(star.headers.get("Access-Control-Allow-Origin"), "*");

  const problemWithCors = await handleSparqlRequest(
    httpRequest({ query: "x=1", origin: "https://app.example" }),
    { ...base, cors },
  );
  assert.equal(problemWithCors.status, 400);
  assert.equal(problemWithCors.headers.get("Access-Control-Allow-Origin"), "https://app.example");

  const plain = await handleSparqlRequest(
    httpRequest({ query: q(SELECT_S), origin: "https://app.example" }),
    base,
  );
  assert.equal(plain.status, 200);
  for (const [name] of plain.headers) assert.ok(!name.startsWith("access-control-"), name);
  assert.equal(plain.headers.get("Vary"), "Accept");
  const options = await handleSparqlRequest(
    httpRequest({ method: "OPTIONS", origin: "https://app.example" }),
    base,
  );
  assert.equal(options.status, 405);
  assert.equal(options.headers.get("Allow"), "GET, POST");

  const badCors = await rejection(
    handleSparqlRequest(httpRequest({ query: q(SELECT_S) }), { ...base, cors: { origins: "https://app.example" } }),
  );
  assert.match(badCors.message, /cors.origins must be "\*" or an array/);
});
