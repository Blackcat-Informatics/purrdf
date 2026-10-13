// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// @blackcatinformatics/purrdf/cloudflare — a SPARQL 1.1 Protocol endpoint and
// host-resolved federation for Cloudflare Workers (and any host with `fetch`, `Request`,
// `Response`, `AbortSignal.any` and `crypto.subtle`).
//
//   * `createFetchServiceResolver` — a `resolveService` handler that sends each `SERVICE`
//     request with `fetch` (or through a service binding), with an optional Cache API
//     layer.
//   * `createFetchLoadResolver` — a `resolveLoad` handler that fetches each hop of a
//     `LOAD`.
//   * `handleSparqlRequest` — a whole `/sparql` endpoint: an HTTP `Request` in, a
//     `Response` out.
//
// Policy and protocol live in Rust. Which endpoints and `LOAD` sources may be reached,
// with which headers, credential and timeout, is the `ServiceCatalog`'s decision —
// enforced inside the job before any effect reaches a resolver, for every redirect hop
// too — and so is whether a request may be cached. How an HTTP request becomes an
// operation, how its dataset parameters are applied, which format answers it, and which
// HTTP problem answers a failure is `SparqlProtocolRequest`'s. This module moves bytes
// between those decisions and the network, builds the `Response`, and keeps the one hook
// Rust cannot: reporting a host fault's real error under a correlation id. It imports
// the package root, and the runtime's `isPoisoned` from the shipped `pkg/purrdf_jspi.mjs`
// (the same module instance the package root uses): once the wasm instance is poisoned
// every call into it traps, so the one response that must still be built — the
// sanitized `500` — is then built here, in JavaScript alone.

import { Dataset, QueryEngine, SparqlProtocolRequest } from "./index.mjs";
import { isPoisoned } from "./pkg/purrdf_jspi.mjs";

// The Cache API keys requests by URL. A service answer is keyed under a fixed origin on
// the reserved `.invalid` top-level domain (RFC 2606), which can never name a real host,
// so a cached SPARQL answer can never be mistaken for, or served as, a real resource.
const CACHE_ORIGIN = "https://purrdf-service-cache.invalid/";

const GOVERNOR_KEYS = [
  "noCeiling",
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
  "maxRequestBytes",
  "onInternalError",
  "xpathRegex",
];

// A SPARQL query or update's text is a program, not a payload: even a large one — a
// VALUES clause pasted with thousands of rows — is kilobytes. 1 MiB comfortably covers
// that while still bounding what an unauthenticated POST can make a Worker buffer before
// the operation is even parsed; a Worker's own memory ceiling is shared across every
// request it is concurrently serving, so an unbounded body is a resource an attacker
// controls for free.
const DEFAULT_MAX_REQUEST_BYTES = 1024 * 1024;

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

function positiveInteger(value, name, caller) {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value <= 0) {
    throw new TypeError(`${caller}: ${name} must be a positive integer, got ${String(value)}`);
  }
  return value;
}

function fetchOption(value, caller) {
  const fetchImpl = isPresent(value) ? value : globalThis.fetch;
  if (typeof fetchImpl !== "function") {
    throw new TypeError(`${caller}: fetch must be a function (no global fetch exists here)`);
  }
  return fetchImpl;
}

/**
 * `onInternalError(error, { correlationId, request })`, defaulting to
 * `defaultInternalErrorReporter`. It is the one place this module ever writes an internal
 * error's real message (or a broken `resolveService`/`resolveLoad`'s own exception) —
 * never the HTTP response, which gets a generic detail and the same `correlationId`.
 */
function internalErrorOption(value, caller) {
  if (!isPresent(value)) return defaultInternalErrorReporter;
  if (typeof value !== "function") {
    throw new TypeError(
      `${caller}: onInternalError must be a function, (error, { correlationId, request }) => void`,
    );
  }
  return value;
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

/**
 * The default `onInternalError`: every error this module refuses to describe to an HTTP
 * client — a bug in a host-supplied `resolveService`/`resolveLoad`, or any exception
 * `handleSparqlRequest` did not otherwise classify — is still never silent. One
 * `console.error` line carries the error itself (so its stack prints, exactly as an
 * uncaught exception's would) and the `correlationId` the response body also carries, so
 * the two are joinable from a Worker's own logs.
 */
function defaultInternalErrorReporter(error, { correlationId }) {
  console.error(error, correlationId);
}

/**
 * Call the host-supplied `onInternalError` so that the reporter itself can never change
 * what this module answers. A reporter is observability, not control flow: an internal
 * error must still become a sanitized `500` — so a reporter that throws synchronously, or
 * returns a promise that rejects, is caught here rather than escaping as an unhandled
 * rejection.
 *
 * Catching it is not swallowing it: the reporter's failure means the original error was
 * never reported, so both go to the default console path in one `console.error` line —
 * the error the reporter was handed and the reporter's own failure. Both are written only
 * to the Worker log, never to a response.
 *
 * Returns a promise that always fulfils once the reporter has settled (immediately for a
 * synchronous one).
 */
function report(hook, reporter, error, context) {
  const reporterFailed = (reporterError) => {
    console.error(
      `@blackcatinformatics/purrdf/cloudflare: the ${hook} reporter failed, so this ` +
        `error went unreported by it:`,
      error,
      `— the reporter's own failure:`,
      reporterError,
    );
  };
  let outcome;
  try {
    outcome = reporter(error, context);
  } catch (reporterError) {
    reporterFailed(reporterError);
    return Promise.resolve();
  }
  // `Promise.resolve` adopts a thenable (including one whose `then` getter throws, which
  // becomes a rejection) and passes any other return value through unchanged.
  return Promise.resolve(outcome).then(() => undefined, reporterFailed);
}

function cacheOptions(source, caller) {
  const { cache, cacheTtlSeconds } = source;
  if (!isPresent(cache)) {
    if (isPresent(cacheTtlSeconds)) {
      throw new TypeError(`${caller}: cacheTtlSeconds needs cache; without one it caches nothing`);
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
  return { cache, ttl: positiveInteger(cacheTtlSeconds, "cacheTtlSeconds", caller) };
}

// ---------------------------------------------------------------------------
// The exchange both resolvers share
// ---------------------------------------------------------------------------

/** The `fetch` a request to `url` goes through: its origin's service binding, or `fetch`. */
function route(url, bindings, fetchImpl) {
  const binding = bindings.get(url.origin);
  return binding === undefined ? fetchImpl : (input, init) => binding.fetch(input, init);
}

function errorText(error) {
  return error instanceof Error ? `${error.name}: ${error.message}` : String(error);
}

// Statuses the Fetch algorithm classifies as a redirect (WHATWG Fetch §4.3): the ones a
// `Location` response header retargets. 304 (Not Modified) is not among them.
const REDIRECT_STATUSES = new Set([301, 302, 303, 307, 308]);

/**
 * Issue one request with `redirect: "manual"` and read its whole body. This never
 * follows a redirect itself: a redirect response is handed back as `{ redirect: { status,
 * location } }` (`location` is `null` for a browser's opaque-redirect response, which
 * withholds it). Every other way it can fail to produce a 2xx body — an unparsable URL, a
 * network error, the job abandoning the request (its own timeout or deadline included,
 * through `signal`), a non-2xx non-redirect status — is a `{ kind: "transport" }` failure;
 * the body of a refused or redirect response is cancelled, never read.
 */
async function fetchOnce({ what, url, method, headers, body, signal, bindings, fetchImpl }) {
  let parsed;
  try {
    parsed = new URL(url);
  } catch (error) {
    return { kind: "transport", message: `${what}: not a fetchable URL (${errorText(error)})` };
  }
  let response;
  try {
    response = await route(parsed, bindings, fetchImpl)(url, {
      method,
      headers,
      body,
      signal,
      redirect: "manual",
    });
  } catch (error) {
    if (signal.aborted) {
      return { kind: "transport", message: `${what}: abandoned by the query` };
    }
    return { kind: "transport", message: `${what}: ${errorText(error)}` };
  }
  if (response.type === "opaqueredirect" || REDIRECT_STATUSES.has(response.status)) {
    await response.body?.cancel().catch(() => undefined);
    const location = response.type === "opaqueredirect" ? null : response.headers.get("Location");
    return { redirect: { status: response.status, location } };
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
    if (signal.aborted) {
      return { kind: "transport", message: `${what}: abandoned by the query while the body arrived` };
    }
    return { kind: "transport", message: `${what}: reading the body failed (${errorText(error)})` };
  }
}

/** A message for a redirect a `SERVICE` never follows: its status and, when readable, its `Location`. */
function redirectMessage({ status, location }) {
  const where = location ? ` to ${location}` : " (Location withheld by an opaque redirect)";
  return `redirected (HTTP ${status || "opaque"}${where}); a catalogued endpoint's redirect is never followed`;
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
 * (a repeated name is kept, never merged). The request is abandoned when the query
 * abandons it (`ctx.signal`): at its catalog profile's timeout, or the query's deadline,
 * whichever comes first. Everything that stops a 2xx answer arriving is a
 * `{ kind: "transport" }` failure — never a throw, which would report the endpoint's
 * failure as this handler's own fault.
 *
 * The fetch is always sent with `redirect: "manual"` and never follows one: a 3xx (or a
 * browser's opaque-redirect response to a cross-origin `redirect: "manual"` fetch) is
 * itself a `{ kind: "transport" }` failure naming the endpoint and the redirect's status
 * and, when readable, its `Location`. No second request is ever sent — to the `Location`
 * or anywhere else — so the profile's headers and credential (an `X-Api-Key`, a
 * `Cookie`, …) can never reach an origin the catalog did not authorize.
 *
 * With `cache` (a Cache API object such as `caches.default`) and `cacheTtlSeconds`, the
 * answer to every request the job marks `cacheable` is reused: keyed under a fixed
 * `.invalid` origin by a SHA-256 of the endpoint, query text, `Accept` and headers, and
 * stored with `Cache-Control: max-age=<ttl>`. A request that carries a credential is not
 * cacheable and goes to the endpoint with no cache involved. A `cache.match` or
 * `cache.put` that fails is the invocation's transport failure: `SERVICE SILENT` answers
 * it with the join identity (and records it), and without `SILENT` it fails the query.
 */
export function createFetchServiceResolver(options) {
  const caller = "createFetchServiceResolver";
  const source = plainObject(isPresent(options) ? options : {}, `${caller} options`);
  refuseUnknownKeys(source, ["fetch", "bindings", "cache", "cacheTtlSeconds"], caller);
  const fetchImpl = fetchOption(source.fetch, caller);
  const bindings = bindingsOption(source.bindings, caller);
  const caching = cacheOptions(source, caller);

  return async function resolveService(request, ctx) {
    const what = `SERVICE <${request.endpoint}>`;
    const cached = caching !== undefined && request.cacheable;
    let key;
    if (cached) {
      key = await cacheKey(request);
      let hit;
      try {
        hit = await caching.cache.match(key);
        if (hit) return new Uint8Array(await hit.arrayBuffer());
      } catch (error) {
        return { kind: "transport", message: `${what}: the cache lookup failed (${errorText(error)})` };
      }
    }
    const answer = await fetchOnce({
      what,
      url: request.endpoint,
      method: "POST",
      headers: [
        ["Content-Type", request.contentType],
        ["Accept", request.accept],
        ["User-Agent", request.userAgent],
        ...request.headers,
      ],
      body: request.queryText,
      signal: ctx.signal,
      bindings,
      fetchImpl,
    });
    if (answer.redirect !== undefined) {
      return { kind: "transport", message: `${what}: ${redirectMessage(answer.redirect)}` };
    }
    if (answer.kind !== undefined) return answer;
    if (cached) {
      const stored = new Response(answer.bytes, {
        headers: [
          ["Content-Type", answer.contentType ?? request.accept],
          ["Cache-Control", `max-age=${caching.ttl}`],
        ],
      });
      try {
        await caching.cache.put(key, stored);
      } catch (error) {
        return { kind: "transport", message: `${what}: the cache write failed (${errorText(error)})` };
      }
    }
    return answer.bytes;
  };
}

// ---------------------------------------------------------------------------
// LOAD
// ---------------------------------------------------------------------------

/**
 * A `resolveLoad` handler that GETs one hop of a `LOAD` with `fetch` (or the origin's
 * service binding) and answers `{ bytes, mediaType, base }`: the media type from the
 * response's `Content-Type` (parameters stripped), the base the hop's URL.
 *
 * The job has already authorized the hop against its catalog, and hands over what it
 * sends: an `Accept` naming every RDF syntax the engine parses, and the `User-Agent`,
 * headers and credential of that hop's own catalog profile. The fetch is sent with
 * `redirect: "manual"`, and a redirect is answered `{ kind: "redirect", location }`: the
 * job resolves the location, re-authorizes it and asks for it as a fresh hop, with that
 * hop's own headers — so a redirect to another origin never carries a credential its
 * profile did not grant. A redirect whose `Location` is withheld (a browser's opaque
 * redirect) or absent is a `{ kind: "transport" }` failure, and so is everything else
 * that stops a document arriving — including a response with no `Content-Type`.
 */
export function createFetchLoadResolver(options) {
  const caller = "createFetchLoadResolver";
  const source = plainObject(isPresent(options) ? options : {}, `${caller} options`);
  refuseUnknownKeys(source, ["fetch", "bindings"], caller);
  const fetchImpl = fetchOption(source.fetch, caller);
  const bindings = bindingsOption(source.bindings, caller);

  return async function resolveLoad(request, ctx) {
    const { iri } = request;
    const what = `LOAD <${iri}>`;
    const headers = [["Accept", request.accept]];
    if (request.userAgent !== undefined) headers.push(["User-Agent", request.userAgent]);
    headers.push(...request.headers);
    const result = await fetchOnce({
      what,
      url: iri,
      method: "GET",
      headers,
      body: undefined,
      signal: ctx.signal,
      bindings,
      fetchImpl,
    });
    if (result.redirect !== undefined) {
      const { status, location } = result.redirect;
      if (!location) {
        return {
          kind: "transport",
          message: `${what}: a redirect (HTTP ${status || "opaque"}) withheld its Location; nothing to follow`,
        };
      }
      return { kind: "redirect", location };
    }
    if (result.kind !== undefined) return result;
    const mediaType = result.contentType?.split(";")[0].trim();
    if (!mediaType) {
      return { kind: "transport", message: `${what}: the response has no Content-Type` };
    }
    return { bytes: result.bytes, mediaType, base: result.url || iri };
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
  const maxRequestBytes = isPresent(source.maxRequestBytes)
    ? positiveInteger(source.maxRequestBytes, "maxRequestBytes", caller)
    : DEFAULT_MAX_REQUEST_BYTES;
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
  if (isPresent(source.xpathRegex) && typeof source.xpathRegex !== "string") {
    throw new TypeError(`${caller}: xpathRegex must be the stable name of a dated XPath law`);
  }
  // The host options the twins take are passed through untouched; the twins validate
  // them (and refuse a catalog with no resolveService, or an xpathRegex that names no
  // dated law).
  const host = {};
  for (const key of [
    "resolveService",
    "resolveLoad",
    "catalog",
    "localServices",
    "yieldEveryPolls",
    "xpathRegex",
  ]) {
    if (isPresent(source[key])) host[key] = source[key];
  }
  const onInternalError = internalErrorOption(source.onInternalError, caller);
  return {
    engine: source.engine,
    dataset: source.dataset,
    governors: { ...governors },
    cors,
    maxRequestBytes,
    onInternalError,
    host,
  };
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

/** An RFC 9457 `application/problem+json` response for a refusal this module builds itself. */
function problem(status, detail, extensions, headers) {
  const body = { type: "about:blank", title: SparqlProtocolRequest.statusTitle(status), status, detail };
  for (const [name, value] of Object.entries(extensions)) {
    if (value !== undefined) body[name] = value;
  }
  return new Response(JSON.stringify(body), {
    status,
    headers: [["Content-Type", "application/problem+json"], ...headers],
  });
}

/**
 * The response for a governor that stopped the request, as Rust answers the outcome's
 * `tripped` record (`SparqlProtocolRequest.problemForTrip`): a ceiling's `422` with its
 * dimension and measurements, or the `503` of a cancellation or a deadline.
 */
function tripResponse(tripped, headers) {
  const found = SparqlProtocolRequest.problemForTrip(tripped);
  try {
    return new Response(found.body(undefined), {
      status: found.status,
      headers: [["Content-Type", found.contentType], ...headers],
    });
  } finally {
    found.free();
  }
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

/**
 * The response for an operation that failed with `error` — a twin's rejection, or any
 * exception this adapter did not classify. Rust decides the problem from the error's
 * `code` (`SparqlProtocolRequest.problemFor`): its status, its `code`, and whether its
 * `detail` may be the error's own words. A failure that is this endpoint's own is
 * sanitized, and its real error goes to `onInternalError` under a correlation id —
 * `correlationId` when a host handler's fault was already reported under one, a fresh id
 * otherwise.
 */
function failureResponse(error, signal, headers, internal, correlationId) {
  const timing = serverTiming(error?.evidence?.async);
  // A poisoned instance traps on every call, `problemFor` included; so does one whose
  // poisoning this call is the first to see. Either way the failure is this endpoint's
  // own: the sanitized `500`, built without the instance, reporting `error` once.
  let found;
  try {
    if (!isPoisoned()) found = SparqlProtocolRequest.problemFor(error, signal?.aborted === true);
  } catch {
    found = undefined;
  }
  if (found === undefined) return internalFailure(error, [...timing, ...headers], internal, correlationId);
  try {
    let id;
    if (found.internal) {
      id = correlationId ?? reportInternalError(error, internal.onInternalError, internal.request);
    }
    return new Response(found.body(id), {
      status: found.status,
      headers: [["Content-Type", found.contentType], ...timing, ...headers],
    });
  } finally {
    found.free();
  }
}

/**
 * The sanitized `500` for an exception nothing classified — a bug in this adapter, or
 * a poisoned wasm instance — or for a host handler's fault whose real error was already
 * reported under `correlationId`. The exception's own words, and any `code` it happens to
 * carry, never reach the response: they go to `onInternalError` alone, exactly once — the
 * report happens before the body is built, and however the body is built.
 *
 * Rust builds the body (`SparqlProtocolRequest.problemFor`) while the instance can serve;
 * once it is poisoned — or if that call throws for any reason — the same document is
 * built here without it (`poisonSafeInternalProblem`), so the endpoint still answers.
 */
function internalFailure(error, headers, internal, correlationId) {
  const id = correlationId ?? reportInternalError(error, internal.onInternalError, internal.request);
  let found;
  try {
    if (!isPoisoned()) found = SparqlProtocolRequest.problemFor(undefined, false);
  } catch {
    found = undefined;
  }
  if (found === undefined) return poisonSafeInternalProblem(id, headers);
  try {
    return new Response(found.body(id), {
      status: found.status,
      headers: [["Content-Type", found.contentType], ...headers],
    });
  } finally {
    found.free();
  }
}

/**
 * The sanitized `500` document `SparqlProtocolRequest.problemFor(undefined, false)` builds
 * for correlation id `id`, built in JavaScript alone: the response of an endpoint whose
 * wasm instance is poisoned and can no longer build anything. The detail is fixed; the
 * one variable part is the correlation id the log line also carries.
 */
function poisonSafeInternalProblem(id, headers) {
  const body = {
    type: "about:blank",
    title: "Internal Server Error",
    status: 500,
    detail: `internal error; see the Worker log for correlation id ${id}`,
    code: "InternalError",
    correlationId: id,
  };
  return new Response(JSON.stringify(body), {
    status: 500,
    headers: [["Content-Type", "application/problem+json"], ...headers],
  });
}

/**
 * Report `error` through `onInternalError` with a fresh correlation id, and return the
 * id: the one thing the response is allowed to carry back to the client, so an operator
 * can join what the client saw to what the log actually says.
 */
function reportInternalError(error, onInternalError, request) {
  const correlationId = crypto.randomUUID();
  // Not awaited, and never rejecting: a throwing or rejecting `onInternalError` still
  // yields the sanitized `500` (see `report`), never an unhandled rejection.
  void report("onInternalError", onInternalError, error, { correlationId, request });
  return correlationId;
}

/**
 * Wrap a host-supplied `resolveService`/`resolveLoad` so that a bug in *it* — a throw, a
 * rejection — never reaches the client as its own words, while still reaching the engine
 * as exactly what it is: the handler's fault, the failure of the one invocation it was
 * answering. The real error goes to `onInternalError` with a fresh correlation id, and
 * the effect is answered with `{ kind: "fault" }` carrying only that id — the host's
 * words replaced before they reach Rust. The clause that issued the effect decides what
 * the fault means: without `SILENT` the request fails with `native-sparql-host-fault` or
 * `native-sparql-load-fault`, a `500` whose `correlationId` is this one (`sawFault`
 * reports whether the wrapper fired, and with which id); under `SERVICE SILENT` or
 * `LOAD SILENT` the clause answers with the join identity, or loads nothing, and the
 * response is that answer, the invocation recorded on the evidence as a silenced
 * `"fault"` and the bug already reported under its id.
 *
 * A `handler` that is not a function (the option was never given) passes through
 * unchanged: `undefined` must stay `undefined`, or the engine would believe a handler was
 * configured when none was.
 */
function wrapHostHandler(handler, onInternalError, request) {
  if (typeof handler !== "function") return { handler, sawFault: () => undefined };
  let fault;
  const wrapped = async (effectRequest, ctx) => {
    try {
      return await handler(effectRequest, ctx);
    } catch (error) {
      const correlationId = reportInternalError(error, onInternalError, request);
      fault = { correlationId };
      return { kind: "fault", message: `internal error; see the Worker log for correlation id ${correlationId}` };
    }
  };
  return { handler: wrapped, sawFault: () => fault };
}

/** The `413` problem for a request body over `maxRequestBytes`, in the adapter's own shape. */
function bodyTooLarge(maxRequestBytes, headers) {
  return problem(
    413,
    `the request body exceeds ${maxRequestBytes} bytes`,
    { code: "ContentTooLarge", limit: exactNumber(maxRequestBytes) },
    headers,
  );
}

/**
 * `request`'s body, bounded at `maxRequestBytes`. A `Content-Length` above the bound is
 * refused before anything is read. A missing or understated one — a lying header can
 * never buy a larger body than an honest one would — is still caught: the body streams in
 * chunk by chunk with a running byte count, and the stream is cancelled the moment that
 * count would exceed the bound, before the excess is ever buffered. Either way the
 * refusal is the same `413` problem.
 */
async function boundedRequestBody(request, maxRequestBytes, headers) {
  const declared = request.headers.get("Content-Length");
  if (declared !== null) {
    const length = Number(declared);
    if (Number.isFinite(length) && length > maxRequestBytes) {
      return { response: bodyTooLarge(maxRequestBytes, headers) };
    }
  }
  if (request.body === null) return { value: new Uint8Array(0) };
  const reader = request.body.getReader();
  const chunks = [];
  let total = 0;
  for (;;) {
    const step = await reader.read();
    if (step.done) break;
    total += step.value.byteLength;
    if (total > maxRequestBytes) {
      await reader.cancel(`request body exceeds ${maxRequestBytes} bytes`).catch(() => undefined);
      return { response: bodyTooLarge(maxRequestBytes, headers) };
    }
    chunks.push(step.value);
  }
  const body = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    body.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return { value: body };
}

/**
 * Answer one SPARQL 1.1 Protocol request against `dataset`.
 *
 * `governors` is required and must set `deadlineMs`; every request runs governed. A
 * query runs through `queryGovernedNegotiatedAsync`, whose document goes straight into
 * the response body; an update through `updateGovernedAsync` (`governors.maxAnswers`
 * bounds query answers and is not applied to an update, which has none). The request's
 * `signal` cancels the job. `maxRequestBytes` (1 MiB by default) bounds the request body:
 * a `Content-Length` above it is refused before anything is read, and a missing or
 * understated one is still caught while the body streams in, so a body can never be made
 * to buffer past the bound by lying about its size.
 *
 * Statuses: `200` with the negotiated document; `204` for an applied update; the
 * protocol's own refusals (`400`, `405` for a method the protocol does not bind, `415`
 * for a `Content-Type` it does not define); `406` when the `Accept` header allows no
 * format that can carry the result; `413` when the body exceeds `maxRequestBytes`; `422`
 * when a deterministic ceiling (fuel, answers, intermediate cells, scratch bytes, remote
 * requests) stopped it; `503` when the deadline or a cancellation did (with no
 * `Retry-After`: the same request would stop again). A failed operation is answered with
 * the problem Rust maps its code to (`SparqlProtocolRequest.problemFor`): `400` for an
 * operation that does not parse or that the engine refuses to evaluate as written (its
 * code — `native-sparql-query-parse`, `native-sparql-unsupported`, … — is the problem's
 * `code`, and its message the `detail`); `403` when a `SERVICE` endpoint or `LOAD` source
 * was refused before anything was contacted (`native-sparql-service-denied` and
 * `native-sparql-load-denied`: the catalog; `native-sparql-service-host-denied` and
 * `native-sparql-load-host-denied`: the resolver's own policy); `409` when another
 * update of the dataset is in flight (`native-sparql-update-in-flight`); `502` when a
 * `SERVICE` endpoint or `LOAD` source was contacted and gave no usable answer
 * (`native-sparql-service-failed`, `native-sparql-load-failed`,
 * `native-sparql-load-decode`); `500` when evaluation failed, with the engine's code and
 * its own words. A `403` or `502` never carries the engine's message: it would echo the
 * catalog's policy or a resolver's or remote's own words, so `detail` is a fixed
 * description of the code. This endpoint's own faults are a `500` with a fixed `detail`
 * and a `correlationId`, and their real error goes to `onInternalError(error, {
 * correlationId, request })` (one `console.error` line by default) and never into the
 * response: no resolver reaches a named endpoint or source
 * (`native-sparql-service-unconfigured`, `native-sparql-load-no-resolver`), or — with
 * `code: "InternalError"` — a bug in a host-supplied `resolveService`/`resolveLoad`, an
 * exception no code classifies, an unexpected exception anywhere in this adapter, or a
 * poisoned wasm instance (whose `500` this module builds without calling into it). An
 * `onInternalError` that itself throws or rejects changes none of that: the response is
 * still the sanitized `500`, and the reporter's failure goes to `console.error` together
 * with the error it was handed. Never a `200` with a partial body. Every error is an RFC
 * 9457 `application/problem+json` document (`type: "about:blank"`, `title`, `status`,
 * `detail`, and `code` — the refusal's stable name — plus `parameter`, `dimension`,
 * `limit`, `consumed`, `estimate`, `offered`, `correlationId` where they apply). Every
 * evaluated response carries `Server-Timing` from the job's `evidence.async`.
 *
 * The operation text is parsed exactly once: without dataset parameters it goes to the
 * engine exactly as the request carried it, and a text that does not parse is the
 * engine's own parse refusal — the `400` above; with dataset parameters the
 * `FROM`/`USING` splice — which only the protocol module may compute — parses it once to
 * rewrite the dataset clause before the engine parses the rewritten text again.
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
  // Everything below is either an already-classified refusal (`protocolStep`,
  // `failureResponse`, `tripResponse`) or a `500`: nothing past this point may ever
  // let an exception escape as anything but a problem response. An error this catch
  // reaches unclassified — a bug in this adapter, or `protocolStep` re-throwing something
  // that was never a protocol refusal — is exactly `handleSparqlRequest`'s own version of
  // "a host bug", so it gets the same generic detail and correlation id as one.
  try {
    const url = new URL(request.url);
    const bounded = await boundedRequestBody(request, o.maxRequestBytes, headers);
    if (bounded.response !== undefined) return bounded.response;
    const body = bounded.value;
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
      // Parsed exactly once on the success path (see the docstring above): without dataset
      // parameters `operation.text` already IS the effective text — Rust never splices when
      // there is nothing to splice — so it is handed to the engine unparsed by this module,
      // and the engine's own parse is the only one that ever runs. With dataset parameters
      // the splice needs its own parse first, to rewrite the `FROM`/`USING` clause.
      let text;
      if (operation.hasDatasetParameters) {
        const spliced = protocolStep(() => operation.effectiveText(), headers, o.cors);
        if (spliced.response !== undefined) return spliced.response;
        text = spliced.value;
      } else {
        text = operation.text;
      }
      // `resolveService`/`resolveLoad` are host code, not this engine's: a bug in either
      // is wrapped so its own words never reach the client, while it still reaches the
      // engine as the handler's fault — the failure of that one invocation (see
      // `wrapHostHandler`). Without `SILENT` the twin call below rejects with it, and
      // `hostFault()` reports its correlation id for the catch to answer with; under
      // `SILENT` the clause absorbs it and the outcome below is the answer.
      const resolveService = wrapHostHandler(o.host.resolveService, o.onInternalError, request);
      const resolveLoad = wrapHostHandler(o.host.resolveLoad, o.onInternalError, request);
      const hostFault = () => resolveService.sawFault() ?? resolveLoad.sawFault();
      const internal = { onInternalError: o.onInternalError, request };
      const host = {
        ...o.host,
        resolveService: resolveService.handler,
        resolveLoad: resolveLoad.handler,
        signal: request.signal,
      };
      if (operation.kind === "update") {
        const { maxAnswers: _queryOnly, ...governors } = o.governors;
        let outcome;
        try {
          outcome = await o.engine.updateGovernedAsync(o.dataset, text, { ...governors, ...host });
        } catch (error) {
          return failureResponse(error, request.signal, headers, internal, hostFault()?.correlationId);
        }
        const timing = serverTiming(outcome.evidence.async);
        if (!outcome.isApplied) return tripResponse(outcome.tripped, [...timing, ...headers]);
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
        outcome = await o.engine.queryGovernedNegotiatedAsync(o.dataset, text, {
          ...o.governors,
          ...host,
          accept,
        });
      } catch (error) {
        return failureResponse(
          error,
          request.signal,
          [["Vary", "Accept"], ...headers],
          internal,
          hostFault()?.correlationId,
        );
      }
      const timing = serverTiming(outcome.evidence.async);
      if (!outcome.isComplete) {
        return tripResponse(outcome.tripped, [...timing, ["Vary", "Accept"], ...headers]);
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
  } catch (error) {
    return internalFailure(error, headers, { onInternalError: o.onInternalError, request });
  }
}
