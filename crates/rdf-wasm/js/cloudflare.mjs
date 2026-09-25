// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// @blackcatinformatics/purrdf/cloudflare — a SPARQL 1.1 Protocol endpoint and
// host-resolved federation for Cloudflare Workers (and any host with `fetch`, `Request`,
// `Response`, `AbortSignal.any` and `crypto.subtle`).
//
//   * `createFetchServiceResolver` — a `resolveService` handler that sends each `SERVICE`
//     request with `fetch` (or through a service binding), with an optional Cache API
//     layer.
//   * `createFetchLoadResolver` — a `resolveLoad` handler that fetches each `LOAD`
//     document.
//   * `handleSparqlRequest` — a whole `/sparql` endpoint: an HTTP `Request` in, a
//     `Response` out.
//
// Policy and protocol live in Rust. Which endpoints may be called, with which headers
// and credential, is the `ServiceCatalog`'s decision — enforced inside the job before
// any `SERVICE` effect reaches a resolver, and asked of it (`authorizeLoad`) before any
// `LOAD` fetch. How an HTTP request becomes an operation, how its dataset parameters are
// applied and which format answers it is `SparqlProtocolRequest`'s. This module moves
// bytes between those decisions and the network, and maps their typed outcomes onto
// HTTP statuses. It imports nothing but the package root.

import { Dataset, QueryEngine, ServiceCatalog, SparqlProtocolRequest } from "./index.mjs";

// The Cache API keys requests by URL. A service answer is keyed under a fixed origin on
// the reserved `.invalid` top-level domain (RFC 2606), which can never name a real host,
// so a cached SPARQL answer can never be mistaken for, or served as, a real resource.
const CACHE_ORIGIN = "https://purrdf-service-cache.invalid/";

// Header names whose presence makes a request user-specific. A shared cache must never
// answer one: the answer is the credential holder's, not the endpoint's.
const CREDENTIAL_HEADERS = new Set(["authorization", "cookie", "proxy-authorization"]);

const GOVERNOR_KEYS = [
  "fuel",
  "deadlineMs",
  "maxAnswers",
  "maxIntermediateCells",
  "maxScratchBytes",
  "maxRemoteRequests",
];

const HANDLER_KEYS = [
  "engine",
  "dataset",
  "governors",
  "resolveService",
  "resolveLoad",
  "catalog",
  "localServices",
  "cors",
  "yieldEveryPolls",
  "stackBytes",
];

// RFC 9110 §15 reason phrases: an `about:blank` problem's `title` is its status's phrase
// (RFC 9457 §4.2.1).
const STATUS_TITLES = {
  400: "Bad Request",
  405: "Method Not Allowed",
  406: "Not Acceptable",
  415: "Unsupported Media Type",
  422: "Unprocessable Content",
  500: "Internal Server Error",
  503: "Service Unavailable",
};

const encoder = new TextEncoder();

// ---------------------------------------------------------------------------
// Option validation: every option is checked when the factory or handler is called, and
// every option that would be ignored is refused by name.
// ---------------------------------------------------------------------------

function isPresent(value) {
  return value !== undefined && value !== null;
}

function plainObject(value, what) {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new TypeError(`${what} must be an object`);
  }
  return value;
}

function refuseUnknownKeys(source, accepted, what) {
  for (const key of Object.keys(source)) {
    if (!accepted.includes(key) && isPresent(source[key])) {
      throw new TypeError(
        `unknown ${what} option ${JSON.stringify(key)} (accepted: ${accepted.join(", ")})`,
      );
    }
  }
}

function requireCatalog(value, caller) {
  if (!(value instanceof ServiceCatalog)) {
    throw new TypeError(
      `${caller} requires catalog, a ServiceCatalog: it is the policy every request is ` +
        `authorized against, and nothing is sent without one`,
    );
  }
  return value;
}

function positiveInteger(value, name, caller) {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value <= 0) {
    throw new TypeError(`${caller}: ${name} must be a positive integer, got ${String(value)}`);
  }
  return value;
}

function requireTimeout(value, caller) {
  if (!isPresent(value)) {
    throw new TypeError(
      `${caller} requires timeoutMs: every request needs a bound of its own, independent ` +
        `of the query's deadline`,
    );
  }
  return positiveInteger(value, "timeoutMs", caller);
}

function fetchOption(value, caller) {
  const fetchImpl = isPresent(value) ? value : globalThis.fetch;
  if (typeof fetchImpl !== "function") {
    throw new TypeError(`${caller}: fetch must be a function (no global fetch exists here)`);
  }
  return fetchImpl;
}

/** `bindings`: an object mapping an origin (`https://host[:port]`) to a service binding. */
function bindingsOption(value, caller) {
  const bindings = new Map();
  if (!isPresent(value)) return bindings;
  plainObject(value, `${caller}: bindings`);
  for (const [origin, binding] of Object.entries(value)) {
    let parsed;
    try {
      parsed = new URL(origin);
    } catch {
      parsed = undefined;
    }
    if (parsed === undefined || parsed.origin !== origin) {
      throw new TypeError(
        `${caller}: every bindings key must be an origin such as "https://example.org"; ` +
          `${JSON.stringify(origin)} is not`,
      );
    }
    if (typeof binding?.fetch !== "function") {
      throw new TypeError(
        `${caller}: bindings[${JSON.stringify(origin)}] must be a service binding (an ` +
          `object with a fetch method)`,
      );
    }
    bindings.set(origin, binding);
  }
  return bindings;
}

function cacheOptions(source, caller) {
  const { cache, cacheTtlSeconds, waitUntil } = source;
  if (!isPresent(cache)) {
    if (isPresent(cacheTtlSeconds)) {
      throw new TypeError(`${caller}: cacheTtlSeconds needs cache; without one it caches nothing`);
    }
    if (isPresent(waitUntil)) {
      throw new TypeError(`${caller}: waitUntil defers cache writes and needs cache`);
    }
    return undefined;
  }
  if (typeof cache.match !== "function" || typeof cache.put !== "function") {
    throw new TypeError(`${caller}: cache must be a Cache (an object with match and put)`);
  }
  if (!isPresent(cacheTtlSeconds)) {
    throw new TypeError(
      `${caller}: cache requires cacheTtlSeconds: how long a remote answer may be reused ` +
        `is the host's decision, never a default`,
    );
  }
  if (isPresent(waitUntil) && typeof waitUntil !== "function") {
    throw new TypeError(`${caller}: waitUntil must be a function, e.g. (p) => ctx.waitUntil(p)`);
  }
  return {
    cache,
    ttl: positiveInteger(cacheTtlSeconds, "cacheTtlSeconds", caller),
    waitUntil: isPresent(waitUntil) ? waitUntil : undefined,
  };
}

// ---------------------------------------------------------------------------
// The exchange both resolvers share
// ---------------------------------------------------------------------------

/** The `fetch` a request to `url` goes through: its origin's service binding, or `fetch`. */
function route(url, bindings, fetchImpl) {
  const binding = bindings.get(url.origin);
  return binding === undefined ? fetchImpl : (input, init) => binding.fetch(input, init);
}

/** Flattened `[name, value, …]` pairs → `[[name, value], …]`. */
function pairs(flat) {
  const out = [];
  for (let index = 0; index + 1 < flat.length; index += 2) out.push([flat[index], flat[index + 1]]);
  return out;
}

function errorText(error) {
  return error instanceof Error ? `${error.name}: ${error.message}` : String(error);
}

/**
 * Issue one request and read its whole body. Every way it can fail to produce a 2xx body
 * — an unparsable URL, a network error, the timeout, the job abandoning the request, a
 * non-2xx status — is a `{ kind: "transport" }` failure, which `SILENT` may swallow; the
 * body of a refused response is cancelled, never read.
 */
async function exchange({ what, url, method, headers, body, timeoutMs, signal, bindings, fetchImpl }) {
  let parsed;
  try {
    parsed = new URL(url);
  } catch (error) {
    return { kind: "transport", message: `${what}: not a fetchable URL (${errorText(error)})` };
  }
  const timeout = AbortSignal.timeout(timeoutMs);
  const combined = AbortSignal.any([signal, timeout]);
  let response;
  try {
    response = await route(parsed, bindings, fetchImpl)(url, {
      method,
      headers,
      body,
      signal: combined,
      redirect: "follow",
    });
  } catch (error) {
    if (timeout.aborted) {
      return { kind: "transport", message: `${what}: no response within ${timeoutMs} ms` };
    }
    if (signal.aborted) {
      return { kind: "transport", message: `${what}: abandoned by the query` };
    }
    return { kind: "transport", message: `${what}: ${errorText(error)}` };
  }
  if (!response.ok) {
    await response.body?.cancel().catch(() => undefined);
    return {
      kind: "transport",
      message: `${what}: HTTP ${response.status}${response.statusText ? ` ${response.statusText}` : ""}`,
    };
  }
  try {
    const bytes = new Uint8Array(await response.arrayBuffer());
    return { bytes, contentType: response.headers.get("Content-Type"), url: response.url };
  } catch (error) {
    if (timeout.aborted) {
      return { kind: "transport", message: `${what}: the body did not arrive within ${timeoutMs} ms` };
    }
    return { kind: "transport", message: `${what}: reading the body failed (${errorText(error)})` };
  }
}

// ---------------------------------------------------------------------------
// SERVICE
// ---------------------------------------------------------------------------

function hex(buffer) {
  return Array.from(new Uint8Array(buffer), (byte) => byte.toString(16).padStart(2, "0")).join("");
}

/** The Cache API key of one `SERVICE` request: a SHA-256 over everything that shapes its answer. */
async function cacheKey(request) {
  const material = JSON.stringify([
    request.endpoint,
    request.queryText,
    request.accept,
    request.headers,
  ]);
  const digest = await crypto.subtle.digest("SHA-256", encoder.encode(material));
  return new Request(new URL(CACHE_ORIGIN + hex(digest)));
}

/**
 * A `resolveService` handler that POSTs each `SERVICE` request (SPARQL 1.1 Protocol
 * §2.1.3) with `fetch`, or with `bindings[origin].fetch` when the endpoint's origin has a
 * service binding.
 *
 * Headers are sent in this order: `Content-Type`, `Accept` and `User-Agent` from the
 * request, then the catalog profile's headers and its credential header, each appended
 * (a repeated name is kept, never merged). The request is abandoned at `timeoutMs` or at
 * the request's own `timeoutMs` (the catalog profile's, or the engine's default),
 * whichever comes first, and whenever the query abandons it (`ctx.signal`). Everything
 * that stops a 2xx answer arriving is a `{ kind: "transport" }` failure — never a throw,
 * which would be a fault that fails even a `SERVICE SILENT`.
 *
 * With `cache` (a Cache API object such as `caches.default`) and `cacheTtlSeconds`,
 * answers are reused: keyed under a fixed `.invalid` origin by a SHA-256 of the endpoint,
 * query text, `Accept` and headers, stored with `Cache-Control: max-age=<ttl>` (through
 * `waitUntil` when given, else awaited). A request that carries a credential — an
 * `Authorization`, `Cookie` or `Proxy-Authorization` header, or any credential the
 * catalog profile sets — is refused with a `TypeError` rather than answered from or
 * written to a shared cache.
 */
export function createFetchServiceResolver(options) {
  const caller = "createFetchServiceResolver";
  const source = plainObject(options, `${caller} options`);
  refuseUnknownKeys(
    source,
    ["catalog", "fetch", "bindings", "timeoutMs", "cache", "cacheTtlSeconds", "waitUntil"],
    caller,
  );
  const catalog = requireCatalog(source.catalog, caller);
  const timeoutMs = requireTimeout(source.timeoutMs, caller);
  const fetchImpl = fetchOption(source.fetch, caller);
  const bindings = bindingsOption(source.bindings, caller);
  const caching = cacheOptions(source, caller);

  return async function resolveService(request, ctx) {
    let key;
    if (caching !== undefined) {
      const credentialHeader = request.headers.find(([name]) =>
        CREDENTIAL_HEADERS.has(name.toLowerCase()),
      );
      if (credentialHeader !== undefined || catalog.carriesCredential(request.endpoint)) {
        throw new TypeError(
          `SERVICE <${request.endpoint}> carries a credential ` +
            `(${credentialHeader === undefined ? "the catalog profile's" : credentialHeader[0]}); ` +
            `its answer belongs to the credential holder and is never read from or written to ` +
            `a shared cache`,
        );
      }
      key = await cacheKey(request);
      const hit = await caching.cache.match(key);
      if (hit) return new Uint8Array(await hit.arrayBuffer());
    }
    const answer = await exchange({
      what: `SERVICE <${request.endpoint}>`,
      url: request.endpoint,
      method: "POST",
      headers: [
        ["Content-Type", request.contentType],
        ["Accept", request.accept],
        ["User-Agent", request.userAgent],
        ...request.headers,
      ],
      body: request.queryText,
      timeoutMs: Math.min(timeoutMs, request.timeoutMs),
      signal: ctx.signal,
      bindings,
      fetchImpl,
    });
    if (answer.kind !== undefined) return answer;
    if (caching !== undefined) {
      const stored = new Response(answer.bytes, {
        headers: [
          ["Content-Type", answer.contentType ?? request.accept],
          ["Cache-Control", `max-age=${caching.ttl}`],
        ],
      });
      const put = caching.cache.put(key, stored);
      if (caching.waitUntil !== undefined) caching.waitUntil(put);
      else await put;
    }
    return answer.bytes;
  };
}

// ---------------------------------------------------------------------------
// LOAD
// ---------------------------------------------------------------------------

/**
 * A `resolveLoad` handler that GETs each `LOAD` document with `fetch` (or the origin's
 * service binding) and answers `{ bytes, mediaType, base }`: the media type from the
 * response's `Content-Type` (parameters stripped), the base from the final URL after
 * redirects.
 *
 * Before any fetch the catalog authorizes the IRI (`ServiceCatalog.authorizeLoad`: the
 * `network` capability, and `credentials` for a credential); a refusal is a
 * `{ kind: "denied" }` failure, which fails even a `LOAD SILENT`. The request sends an
 * `Accept` naming every RDF syntax the engine parses, the profile's `User-Agent`, and the
 * profile's headers and credential, and is bounded by `timeoutMs` and the profile's
 * timeout, whichever is shorter. Everything else that stops a document arriving —
 * including a response with no `Content-Type` — is a `{ kind: "transport" }` failure.
 */
export function createFetchLoadResolver(options) {
  const caller = "createFetchLoadResolver";
  const source = plainObject(options, `${caller} options`);
  refuseUnknownKeys(source, ["catalog", "fetch", "bindings", "timeoutMs"], caller);
  const catalog = requireCatalog(source.catalog, caller);
  const timeoutMs = requireTimeout(source.timeoutMs, caller);
  const fetchImpl = fetchOption(source.fetch, caller);
  const bindings = bindingsOption(source.bindings, caller);

  return async function resolveLoad(request, ctx) {
    const authorization = catalog.authorizeLoad(request.iri);
    let denial;
    let headers;
    let profileTimeout;
    try {
      denial = authorization.denial;
      headers = [["Accept", authorization.accept]];
      const userAgent = authorization.userAgent;
      if (userAgent !== undefined) headers.push(["User-Agent", userAgent]);
      headers.push(...pairs(authorization.headers));
      profileTimeout = authorization.timeoutMs;
    } finally {
      authorization.free();
    }
    if (denial !== undefined) return { kind: "denied", message: denial };
    const what = `LOAD <${request.iri}>`;
    const answer = await exchange({
      what,
      url: request.iri,
      method: "GET",
      headers,
      body: undefined,
      timeoutMs: profileTimeout === undefined ? timeoutMs : Math.min(timeoutMs, profileTimeout),
      signal: ctx.signal,
      bindings,
      fetchImpl,
    });
    if (answer.kind !== undefined) return answer;
    const mediaType = answer.contentType?.split(";")[0].trim();
    if (!mediaType) {
      return { kind: "transport", message: `${what}: the response has no Content-Type` };
    }
    return { bytes: answer.bytes, mediaType, base: answer.url || request.iri };
  };
}

// ---------------------------------------------------------------------------
// The endpoint
// ---------------------------------------------------------------------------

function handlerOptions(options) {
  const caller = "handleSparqlRequest";
  const source = plainObject(options, `${caller} options`);
  refuseUnknownKeys(source, HANDLER_KEYS, caller);
  if (!(source.engine instanceof QueryEngine)) {
    throw new TypeError(`${caller} requires engine, a QueryEngine`);
  }
  if (!(source.dataset instanceof Dataset)) {
    throw new TypeError(`${caller} requires dataset, the Dataset the endpoint answers from`);
  }
  if (!isPresent(source.governors)) {
    throw new TypeError(
      `${caller} requires governors, with at least deadlineMs: an endpoint runs every ` +
        `request governed, under ceilings its host chose`,
    );
  }
  const governors = plainObject(source.governors, `${caller}: governors`);
  refuseUnknownKeys(governors, GOVERNOR_KEYS, `${caller} governors`);
  if (!isPresent(governors.deadlineMs)) {
    throw new TypeError(
      `${caller}: governors requires deadlineMs, so no request can hold the endpoint ` +
        `indefinitely`,
    );
  }
  let cors;
  if (isPresent(source.cors)) {
    const given = plainObject(source.cors, `${caller}: cors`);
    refuseUnknownKeys(given, ["origins", "maxAgeSeconds"], `${caller} cors`);
    const { origins } = given;
    if (
      origins !== "*" &&
      !(Array.isArray(origins) && origins.every((origin) => typeof origin === "string"))
    ) {
      throw new TypeError(`${caller}: cors.origins must be "*" or an array of origin strings`);
    }
    if (
      isPresent(given.maxAgeSeconds) &&
      !(Number.isSafeInteger(given.maxAgeSeconds) && given.maxAgeSeconds >= 0)
    ) {
      throw new TypeError(`${caller}: cors.maxAgeSeconds must be a non-negative integer`);
    }
    cors = { origins, maxAgeSeconds: given.maxAgeSeconds ?? undefined };
  }
  // The host options the twins take are passed through untouched; the twins validate
  // them (and refuse a catalog with no resolveService).
  const host = {};
  for (const key of ["resolveService", "resolveLoad", "catalog", "localServices", "yieldEveryPolls", "stackBytes"]) {
    if (isPresent(source[key])) host[key] = source[key];
  }
  return { engine: source.engine, dataset: source.dataset, governors: { ...governors }, cors, host };
}

/** The CORS response headers for a request from `origin`; none without `cors`. */
function corsHeaders(cors, origin) {
  if (cors === undefined) return [];
  if (cors.origins === "*") return [["Access-Control-Allow-Origin", "*"]];
  if (origin !== null && cors.origins.includes(origin)) {
    return [["Access-Control-Allow-Origin", origin]];
  }
  return [];
}

function allowedMethods(cors) {
  return cors === undefined ? "GET, POST" : "GET, POST, OPTIONS";
}

/** `Server-Timing` (W3C Server Timing) from a job's `evidence.async`. */
function serverTiming(evidence) {
  if (evidence === undefined) return [];
  const dur = (ms) => Number(ms.toFixed(3));
  return [
    [
      "Server-Timing",
      [
        `freeze;dur=${dur(evidence.freezeMs)}`,
        `eval;dur=${dur(evidence.evaluateMs)}`,
        `serialize;dur=${dur(evidence.serializeMs)}`,
        `service;dur=${dur(evidence.serviceWaitMs)}`,
        `load;dur=${dur(evidence.loadWaitMs)}`,
        `yield;dur=${dur(evidence.yieldWaitMs)}`,
      ].join(", "),
    ],
  ];
}

/** A 64-bit governor count, as an exact JSON number. */
function exactNumber(value) {
  return JSON.rawJSON(String(value));
}

/** An RFC 9457 `application/problem+json` response. */
function problem(status, detail, extensions, headers) {
  const body = { type: "about:blank", title: STATUS_TITLES[status], status, detail };
  for (const [name, value] of Object.entries(extensions)) {
    if (value !== undefined) body[name] = value;
  }
  return new Response(JSON.stringify(body), {
    status,
    headers: [["Content-Type", "application/problem+json"], ...headers],
  });
}

/** The response for a governor that stopped the request: `503` for a stop signal, else `422`. */
function trippedProblem(tripped, headers) {
  if (tripped.kind === "stopped") {
    return problem(503, tripped.message, { code: tripped.label, cause: tripped.cause }, headers);
  }
  return problem(
    422,
    tripped.message,
    {
      code: tripped.label,
      dimension: tripped.dimension,
      limit: tripped.limit === undefined ? undefined : exactNumber(tripped.limit),
      consumed: tripped.consumed === undefined ? undefined : exactNumber(tripped.consumed),
      estimate: tripped.estimate === undefined ? undefined : exactNumber(tripped.estimate),
    },
    headers,
  );
}

/** A `SparqlProtocolRequest` refusal (it carries `status`, `name`, `parameter`) as its problem. */
function protocolProblem(error, headers, cors) {
  const extra = error.status === 405 ? [["Allow", allowedMethods(cors)]] : [];
  return problem(
    error.status,
    error.message,
    { code: error.name, parameter: error.parameter },
    [...extra, ...headers],
  );
}

/** Run `step`, turning a protocol refusal into its problem response. */
function protocolStep(step, headers, cors) {
  try {
    return { value: step() };
  } catch (error) {
    if (error instanceof Error && typeof error.status === "number") {
      return { response: protocolProblem(error, headers, cors) };
    }
    throw error;
  }
}

/** The response for a twin that rejected. */
function failure(error, signal, headers) {
  const timing = serverTiming(error?.evidence?.async);
  if (error?.name === "NotAcceptableError") {
    return problem(
      406,
      error.message,
      { code: "NotAcceptable", offered: SparqlProtocolRequest.offeredMediaTypes("dataset") },
      [...timing, ...headers],
    );
  }
  if (signal.aborted || error?.name === "AbortError" || error?.name === "TimeoutError") {
    return problem(503, errorText(error), { code: "cancelled" }, [...timing, ...headers]);
  }
  return problem(
    500,
    error instanceof Error ? error.message : String(error),
    { code: error instanceof Error ? error.name : "Error" },
    [...timing, ...headers],
  );
}

async function requestBody(request) {
  if (request.body === null) return new Uint8Array(0);
  return new Uint8Array(await request.arrayBuffer());
}

/**
 * Answer one SPARQL 1.1 Protocol request against `dataset`.
 *
 * `governors` is required and must set `deadlineMs`; every request runs governed. A
 * query runs through `queryGovernedNegotiatedAsync`, whose document goes straight into
 * the response body; an update through `updateGovernedAsync` (`governors.maxAnswers`
 * bounds query answers and is not applied to an update, which has none). The request's
 * `signal` cancels the job.
 *
 * Statuses: `200` with the negotiated document; `204` for an applied update; `400` for a
 * malformed request or operation (`405` for a method the protocol does not bind, `415`
 * for a `Content-Type` it does not define); `406` when the `Accept` header allows no
 * format that can carry the result; `422` when a deterministic ceiling (fuel, answers,
 * intermediate cells, scratch bytes, remote requests) stopped it; `503` when the deadline
 * or a cancellation did (with no `Retry-After`: the same request would stop again);
 * `500` when evaluation failed. Never a `200` with a partial body. Every error is an
 * RFC 9457 `application/problem+json` document (`type: "about:blank"`, `title`, `status`,
 * `detail`, and `code` — the refusal's stable name — plus `parameter`, `dimension`,
 * `limit`, `consumed`, `estimate` where they apply). Every evaluated response carries
 * `Server-Timing` from the job's `evidence.async`.
 *
 * With `cors: { origins: "*" | string[], maxAgeSeconds? }` it answers `OPTIONS`
 * preflights with `204` and adds `Access-Control-Allow-Origin` (`*`, or the request's
 * `Origin` when listed) and `Vary: Origin`; without `cors` it adds no CORS header and
 * `OPTIONS` is a `405`.
 */
export async function handleSparqlRequest(request, options) {
  const o = handlerOptions(options);
  const cors = corsHeaders(o.cors, request.headers.get("Origin"));
  const vary = o.cors === undefined ? [] : [["Vary", "Origin"]];
  if (o.cors !== undefined && request.method === "OPTIONS") {
    const preflight = [
      ...cors,
      ["Access-Control-Allow-Methods", "GET, POST, OPTIONS"],
      ["Access-Control-Allow-Headers", "Content-Type, Accept"],
      ...vary,
    ];
    if (o.cors.maxAgeSeconds !== undefined) {
      preflight.push(["Access-Control-Max-Age", String(o.cors.maxAgeSeconds)]);
    }
    return new Response(null, { status: 204, headers: preflight });
  }
  const headers = [...cors, ...vary];
  const url = new URL(request.url);
  const body = await requestBody(request);
  const parsed = protocolStep(
    () =>
      SparqlProtocolRequest.parse(
        request.method,
        request.headers.get("Content-Type") ?? undefined,
        url.search === "" ? undefined : url.search.slice(1),
        body,
      ),
    headers,
    o.cors,
  );
  if (parsed.response !== undefined) return parsed.response;
  const operation = parsed.value;
  try {
    const text = protocolStep(() => operation.effectiveText(), headers, o.cors);
    if (text.response !== undefined) return text.response;
    const host = { ...o.host, signal: request.signal };
    if (operation.kind === "update") {
      const { maxAnswers: _queryOnly, ...governors } = o.governors;
      let outcome;
      try {
        outcome = await o.engine.updateGovernedAsync(o.dataset, text.value, { ...governors, ...host });
      } catch (error) {
        return failure(error, request.signal, headers);
      }
      const timing = serverTiming(outcome.evidence.async);
      if (!outcome.isApplied) return trippedProblem(outcome.tripped, [...timing, ...headers]);
      return new Response(null, { status: 204, headers: [...timing, ...headers] });
    }
    const accept = request.headers.get("Accept") ?? undefined;
    const negotiated = protocolStep(() => operation.negotiate(accept), headers, o.cors);
    if (negotiated.response !== undefined) return negotiated.response;
    if (negotiated.value === undefined) {
      const kind = operation.resultKind;
      return problem(
        406,
        SparqlProtocolRequest.notAcceptableDetail(kind),
        { code: "NotAcceptable", offered: SparqlProtocolRequest.offeredMediaTypes(kind) },
        [["Vary", "Accept"], ...headers],
      );
    }
    let outcome;
    try {
      outcome = await o.engine.queryGovernedNegotiatedAsync(o.dataset, text.value, {
        ...o.governors,
        ...host,
        accept,
      });
    } catch (error) {
      return failure(error, request.signal, [["Vary", "Accept"], ...headers]);
    }
    const timing = serverTiming(outcome.evidence.async);
    if (!outcome.isComplete) {
      return trippedProblem(outcome.tripped, [...timing, ["Vary", "Accept"], ...headers]);
    }
    return new Response(outcome.body.bytes, {
      status: 200,
      headers: [
        ["Content-Type", outcome.body.mediaType],
        ...timing,
        ["Vary", "Accept"],
        ...headers,
      ],
    });
  } finally {
    operation.free();
  }
}
