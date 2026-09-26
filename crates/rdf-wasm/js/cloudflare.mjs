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
  "maxRequestBytes",
  "onInternalError",
];

// RFC 9110 §15 reason phrases: an `about:blank` problem's `title` is its status's phrase
// (RFC 9457 §4.2.1). RFC 9110 renamed 413 from RFC 7231's "Payload Too Large" to "Content
// Too Large"; this table follows the current RFC.
const STATUS_TITLES = {
  400: "Bad Request",
  405: "Method Not Allowed",
  406: "Not Acceptable",
  413: "Content Too Large",
  415: "Unsupported Media Type",
  422: "Unprocessable Content",
  500: "Internal Server Error",
  503: "Service Unavailable",
};

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
 * The default `onCacheError`: the cache is an optimisation, so its own failure is never
 * the query's answer, but it must never be silent either. One `console.warn` line names
 * the operation and the endpoint whose answer was still served (or fetched fresh);
 * Workers routes `console.warn` to its logs, so this is visible without any host wiring.
 */
function defaultCacheErrorReporter(error, { operation, endpoint }) {
  console.warn(`createFetchServiceResolver: cache ${operation} failed for <${endpoint}>: ${errorText(error)}`);
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
 * Call a host-supplied reporter (`onCacheError`, `onInternalError`) so that the reporter
 * itself can never change what this module answers. A reporter is observability, not
 * control flow: a cache failure must still fall through to the remote, a `put` failure
 * must still return the answer, and an internal error must still become a sanitized `500`
 * — so a reporter that throws synchronously, or returns a promise that rejects, is caught
 * here rather than rejecting the resolver or escaping as an unhandled rejection.
 *
 * Catching it is not swallowing it: the reporter's failure means the original error was
 * never reported, so both go to the default console path in one `console.error` line —
 * the error the reporter was handed and the reporter's own failure. Both are written only
 * to the Worker log, never to a response.
 *
 * Returns a promise that always fulfils once the reporter has settled (immediately for a
 * synchronous one), so a caller that wants an asynchronous reporter's work kept alive —
 * the `put` path under `waitUntil` — can hand it on without ever seeing it reject.
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
  const { cache, cacheTtlSeconds, waitUntil, onCacheError } = source;
  if (!isPresent(cache)) {
    if (isPresent(cacheTtlSeconds)) {
      throw new TypeError(`${caller}: cacheTtlSeconds needs cache; without one it caches nothing`);
    }
    if (isPresent(waitUntil)) {
      throw new TypeError(`${caller}: waitUntil defers cache writes and needs cache`);
    }
    if (isPresent(onCacheError)) {
      throw new TypeError(`${caller}: onCacheError reports cache failures and needs cache`);
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
  if (isPresent(onCacheError) && typeof onCacheError !== "function") {
    throw new TypeError(
      `${caller}: onCacheError must be a function, (error, { operation, endpoint }) => void`,
    );
  }
  return {
    cache,
    ttl: positiveInteger(cacheTtlSeconds, "cacheTtlSeconds", caller),
    waitUntil: isPresent(waitUntil) ? waitUntil : undefined,
    onCacheError: isPresent(onCacheError) ? onCacheError : defaultCacheErrorReporter,
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

// Statuses the Fetch algorithm classifies as a redirect (WHATWG Fetch §4.3): the ones a
// `Location` response header retargets. 304 (Not Modified) is not among them.
const REDIRECT_STATUSES = new Set([301, 302, 303, 307, 308]);

// How many redirect hops `createFetchLoadResolver` follows before giving up (its
// `maxRedirects` option). Chosen, not derived: generous enough for a real document
// mirror, small enough that a redirect loop fails fast.
const DEFAULT_MAX_LOAD_REDIRECTS = 5;

/**
 * Whether `response` is a redirect this adapter refuses to follow blindly: a 3xx with a
 * `Location`, or the opaque-redirect filtering a browser's `fetch` applies to
 * `redirect: "manual"` (`type === "opaqueredirect"`, `status === 0`, headers withheld).
 */
function isRedirectResponse(response) {
  return response.type === "opaqueredirect" || REDIRECT_STATUSES.has(response.status);
}

/** `response`'s redirect status and `Location` (`location` is `null` for an opaque one). */
function redirectTarget(response) {
  if (response.type === "opaqueredirect") return { status: 0, location: null };
  return { status: response.status, location: response.headers.get("Location") };
}

/**
 * Issue one request with `redirect: "manual"` and read its whole body. This never
 * follows a redirect itself: a redirect response is handed back as `{ redirect: { status,
 * location } }` for the caller to decide — a transport failure that names the endpoint
 * (`SERVICE`, which must never send a second request to an origin the catalog never
 * authorized), or a hop to re-authorize and re-fetch (`LOAD`). Every other way it can fail
 * to produce a 2xx body — an unparsable URL, a network error, the timeout, the job
 * abandoning the request, a non-2xx non-redirect status — is a `{ kind: "transport" }`
 * failure; the body of a refused or redirect response is cancelled, never read.
 */
async function fetchOnce({ what, url, method, headers, body, timeoutMs, signal, bindings, fetchImpl }) {
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
      redirect: "manual",
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
  if (isRedirectResponse(response)) {
    await response.body?.cancel().catch(() => undefined);
    return { redirect: redirectTarget(response) };
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

/** A message for a redirect `fetchOnce` refuses to follow: its status and, when readable, its `Location`. */
function redirectMessage({ status, location }) {
  const where = location ? ` to ${location}` : " (Location withheld by an opaque redirect)";
  return `redirected (HTTP ${status || "opaque"}${where}); a catalogued endpoint's redirect is never followed`;
}

/** A message for a redirect that carries no usable `Location` to follow (opaque, or unparsable). */
function noLocationMessage(status) {
  return `an opaque redirect (HTTP ${status || "opaque"}) withheld its Location; nothing to follow`;
}

/**
 * `fetchOnce`, for a caller that never follows a redirect itself: `SERVICE`, whose
 * profile headers and credential must never be sent to any origin beyond the one the
 * catalog authorized. A redirect response becomes a `{ kind: "transport" }` failure
 * naming the endpoint and the redirect's status and `Location`; no request is ever sent
 * to the `Location`, and neither its headers nor its body leave this function.
 */
async function exchange(args) {
  const result = await fetchOnce(args);
  if (result.redirect !== undefined) {
    return { kind: "transport", message: `${args.what}: ${redirectMessage(result.redirect)}` };
  }
  return result;
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
 * The fetch is always sent with `redirect: "manual"` and never follows one: a 3xx (or a
 * browser's opaque-redirect response to a cross-origin `redirect: "manual"` fetch) is
 * itself a `{ kind: "transport" }` failure naming the endpoint and the redirect's status
 * and, when readable, its `Location`. No second request is ever sent — to the `Location`
 * or anywhere else — so the profile's headers and credential (an `X-Api-Key`, a
 * `Cookie`, …) can never reach an origin the catalog did not authorize.
 *
 * With `cache` (a Cache API object such as `caches.default`) and `cacheTtlSeconds`,
 * answers are reused: keyed under a fixed `.invalid` origin by a SHA-256 of the endpoint,
 * query text, `Accept` and headers, stored with `Cache-Control: max-age=<ttl>` (through
 * `waitUntil` when given, else awaited). A request that carries a credential — an
 * `Authorization`, `Cookie` or `Proxy-Authorization` header, or any credential the
 * catalog profile sets — is refused with a `TypeError` rather than answered from or
 * written to a shared cache.
 *
 * The cache is an optimisation and never decides the answer: a `cache.match` rejection
 * is treated as a miss (the request still goes to the remote), and a `cache.put` failure
 * never discards an answer the remote already returned — with `waitUntil` the rejected
 * put is still handed to it, and without one the put is awaited inside a `try`, so either
 * way the answer is returned regardless. Every cache failure is reported through
 * `onCacheError(error, { operation: "match" | "put", endpoint })`, which defaults to one
 * `console.warn` line (Workers routes it to logs) so a failure is visible, never silent.
 * The reporter cannot change the answer either: one that throws or returns a rejecting
 * promise still falls through to the remote on a `match` failure and still returns the
 * answer on a `put` failure (with or without `waitUntil`), and its own failure goes to
 * `console.error` together with the cache error it was handed. A `waitUntil` that throws
 * is reported the same way, and the put is then awaited instead of deferred.
 */
export function createFetchServiceResolver(options) {
  const caller = "createFetchServiceResolver";
  const source = plainObject(options, `${caller} options`);
  refuseUnknownKeys(
    source,
    ["catalog", "fetch", "bindings", "timeoutMs", "cache", "cacheTtlSeconds", "waitUntil", "onCacheError"],
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
      let hit;
      try {
        hit = await caching.cache.match(key);
      } catch (error) {
        // Not awaited: the fetch below never waits on the reporter, and `report` never
        // rejects, so a throwing or rejecting reporter cannot turn this miss into a failure.
        void report("onCacheError", caching.onCacheError, error, {
          operation: "match",
          endpoint: request.endpoint,
        });
        hit = undefined;
      }
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
      // `cache.put` never decides the answer: a failure — sync throw or rejection — is
      // reported, never discards `answer.bytes`, which the remote already returned.
      let put;
      try {
        put = Promise.resolve(caching.cache.put(key, stored));
      } catch (error) {
        put = Promise.reject(error);
      }
      // `reportedPut` never rejects — `report` isolates the reporter too — and it settles
      // only once an asynchronous reporter has, so `waitUntil` keeps that work alive.
      const reportedPut = put.catch((error) =>
        report("onCacheError", caching.onCacheError, error, {
          operation: "put",
          endpoint: request.endpoint,
        }),
      );
      let deferred = false;
      if (caching.waitUntil !== undefined) {
        try {
          caching.waitUntil(reportedPut);
          deferred = true;
        } catch (error) {
          // A `waitUntil` that throws never held the put: await it here instead, so the
          // write still completes, and say so — the host's deferral is broken.
          console.error(
            `${caller}: waitUntil threw, so the cache put for <${request.endpoint}> is ` +
              `awaited instead:`,
            error,
          );
        }
      }
      if (!deferred) await reportedPut;
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
 * Before any fetch — and before *every* redirect hop — the catalog authorizes the IRI
 * (`ServiceCatalog.authorizeLoad`: the `network` capability, and `credentials` for a
 * credential); a refusal is a `{ kind: "denied" }` failure, which fails even a
 * `LOAD SILENT`, whether it is the initial IRI or one a redirect retargeted to. Each
 * hop's request sends an `Accept` naming every RDF syntax the engine parses, and the
 * `User-Agent`, headers and credential of *that hop's own* authorization — never the
 * previous hop's, so a redirect to a different origin never carries along a credential or
 * header the target's own catalog profile did not itself grant. Each hop is bounded by
 * `timeoutMs` and its own profile's timeout, whichever is shorter.
 *
 * The fetch is always sent with `redirect: "manual"`. A 3xx (or a browser's
 * opaque-redirect response) is followed by hand: its `Location` is resolved against the
 * current URL and re-authorized from scratch, up to `maxRedirects` hops (5 by default,
 * an option here); a 307/308 on this always-`GET` request is simply re-fetched with
 * `GET`, and the loaded document's `base` is the *final* authorized URL, not the
 * originally requested one. Exceeding `maxRedirects`, or a redirect whose `Location` is
 * missing or unparsable (including an opaque redirect, which withholds it), is a
 * `{ kind: "transport" }` failure. Everything else that stops a document arriving —
 * including a response with no `Content-Type` — is a `{ kind: "transport" }` failure too.
 */
export function createFetchLoadResolver(options) {
  const caller = "createFetchLoadResolver";
  const source = plainObject(options, `${caller} options`);
  refuseUnknownKeys(source, ["catalog", "fetch", "bindings", "timeoutMs", "maxRedirects"], caller);
  const catalog = requireCatalog(source.catalog, caller);
  const timeoutMs = requireTimeout(source.timeoutMs, caller);
  const fetchImpl = fetchOption(source.fetch, caller);
  const bindings = bindingsOption(source.bindings, caller);
  const maxRedirects = isPresent(source.maxRedirects)
    ? positiveInteger(source.maxRedirects, "maxRedirects", caller)
    : DEFAULT_MAX_LOAD_REDIRECTS;

  return async function resolveLoad(request, ctx) {
    let iri = request.iri;
    let hop = 0;
    for (;;) {
      const authorization = catalog.authorizeLoad(iri);
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
      const what = iri === request.iri ? `LOAD <${iri}>` : `LOAD <${iri}> (redirected from <${request.iri}>)`;
      const result = await fetchOnce({
        what,
        url: iri,
        method: "GET",
        headers,
        body: undefined,
        timeoutMs: profileTimeout === undefined ? timeoutMs : Math.min(timeoutMs, profileTimeout),
        signal: ctx.signal,
        bindings,
        fetchImpl,
      });
      if (result.redirect !== undefined) {
        if (hop >= maxRedirects) {
          return {
            kind: "transport",
            message:
              `LOAD <${request.iri}>: exceeded ${maxRedirects} redirect hop${maxRedirects === 1 ? "" : "s"}; ` +
              `the last was ${what}`,
          };
        }
        const { location, status } = result.redirect;
        if (!location) {
          return { kind: "transport", message: `${what}: ${noLocationMessage(status)}` };
        }
        let next;
        try {
          next = new URL(location, iri).toString();
        } catch (error) {
          return {
            kind: "transport",
            message: `${what}: Location ${JSON.stringify(location)} is not a resolvable URL (${errorText(error)})`,
          };
        }
        hop += 1;
        iri = next;
        continue;
      }
      if (result.kind !== undefined) return result;
      const mediaType = result.contentType?.split(";")[0].trim();
      if (!mediaType) {
        return { kind: "transport", message: `${what}: the response has no Content-Type` };
      }
      return { bytes: result.bytes, mediaType, base: result.url || iri };
    }
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
  // The host options the twins take are passed through untouched; the twins validate
  // them (and refuse a catalog with no resolveService).
  const host = {};
  for (const key of ["resolveService", "resolveLoad", "catalog", "localServices", "yieldEveryPolls", "stackBytes"]) {
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

/**
 * The response for a twin (`updateGovernedAsync`/`queryGovernedNegotiatedAsync`) that
 * rejected while running the operation text straight from `operation.text` — the case
 * with no dataset parameters, where nothing was parsed before the engine's own parse (see
 * `handleSparqlRequest`). The twin's rejection carries no distinction between a syntax
 * failure and an evaluation one, so it is reclassified by asking the protocol reading
 * once, here on the failure path only: a text that fails there is the client's malformed
 * request — the same `400` this endpoint always gave a malformed operation, just
 * discovered a step later — and a text that reads fine there failed for a real evaluation
 * reason, which `failure` answers exactly as it always has. A request that carried dataset
 * parameters skips the recheck: its text was already parsed and validated by the splice
 * before the engine ever saw it, so a rejection there is never a syntax failure.
 */
function reclassifiedFailure(operation, error, signal, headers, cors) {
  if (!operation.hasDatasetParameters) {
    const recheck = protocolStep(() => operation.effectiveText(), headers, cors);
    if (recheck.response !== undefined) return recheck.response;
  }
  return failure(error, signal, headers);
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
 * The `500` problem for an error this module never puts in front of a client: a fixed
 * detail plus `correlationId`, never `error.message` or `error.stack` — those went to
 * `onInternalError` alone (see `reportInternalError`), which is the only place a fault's
 * real words, or an unexpected exception's, are ever written.
 */
function internalErrorProblem(correlationId, headers) {
  return problem(
    500,
    `internal error; see the Worker log for correlation id ${correlationId}`,
    { code: "InternalError", correlationId },
    headers,
  );
}

/**
 * Wrap a host-supplied `resolveService`/`resolveLoad` so that a bug in *it* — a throw, a
 * rejection — never reaches the client as its own words, while still reaching the engine
 * as exactly the fault it always was. `./pkg/purrdf_jspi.mjs` already treats a throwing or
 * rejecting resolver as a fault, latched on the job: a fault is about this endpoint's own
 * correctness, never the remote's, so `SERVICE SILENT`/`LOAD SILENT` — a promise about the
 * *remote*, "it may be unreachable" — does not, and must not, swallow it. That invariant
 * has to survive this wrapper, so it never *answers* the effect with a value (a
 * `{ kind: "transport" }` failure is exactly the shape `SILENT` swallows, which would turn
 * a host bug into a quietly incomplete `200`). Instead the real error goes to
 * `onInternalError` with a fresh correlation id, and a sanitized `Error` carrying only
 * that id is re-thrown — the same throw the host's own bug would have produced, so it
 * becomes the same non-silenceable fault it always did, just with the host's words
 * replaced before they ever reach Rust. `sawFault` reports whether this ever fired, and
 * with which id, so the catch around the twin call — which a fault always reaches,
 * `SILENT` or not, because a fault rejects the whole job — can answer `500` with that same
 * id rather than whatever text the engine built around the sanitized message.
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
      throw new Error(`internal error; see the Worker log for correlation id ${correlationId}`);
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
 * Statuses: `200` with the negotiated document; `204` for an applied update; `400` for a
 * malformed request or operation (`405` for a method the protocol does not bind, `415`
 * for a `Content-Type` it does not define); `406` when the `Accept` header allows no
 * format that can carry the result; `413` when the body exceeds `maxRequestBytes`; `422`
 * when a deterministic ceiling (fuel, answers, intermediate cells, scratch bytes, remote
 * requests) stopped it; `503` when the deadline or a cancellation did (with no
 * `Retry-After`: the same request would stop again); `500` when evaluation failed — with
 * the engine's own words in `detail`, exactly as a SPARQL client is owed the reason its
 * query failed — or, for an error this module cannot attribute to the query itself (a bug
 * in a host-supplied `resolveService`/`resolveLoad`, or an unexpected exception anywhere
 * in this adapter), a fixed generic `detail` and a `correlationId` instead: the real error
 * goes to `onInternalError(error, { correlationId, request })` (one `console.error` line
 * by default) and never into the response. An `onInternalError` that itself throws or
 * rejects changes none of that: the response is still the sanitized `500`, and the
 * reporter's failure goes to `console.error` together with the error it was handed.
 * Never a `200` with a partial body. Every error
 * is an RFC 9457 `application/problem+json` document (`type: "about:blank"`, `title`,
 * `status`, `detail`, and `code` — the refusal's stable name — plus `parameter`,
 * `dimension`, `limit`, `consumed`, `estimate`, `correlationId` where they apply). Every
 * evaluated response carries `Server-Timing` from the job's `evidence.async`.
 *
 * The operation text is parsed exactly once on the success path: without dataset
 * parameters it goes to the engine exactly as the request carried it (the engine takes
 * text and parses it once, itself; there is no pre-parsed form to hand it), and with
 * dataset parameters the `FROM`/`USING` splice — which only the protocol module may
 * compute — parses it once to rewrite the dataset clause before the engine parses the
 * rewritten text again.
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
  // `reclassifiedFailure`, `trippedProblem`) or a `500`: nothing past this point may ever
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
      // engine as the same non-silenceable fault an unwrapped throw always was — `SILENT`
      // swallows a remote's failure, never a host bug (see `wrapHostHandler`). So a fault
      // always rejects the twin call below, `SILENT` or not, and `hostFault()` reports its
      // correlation id for as long as this request runs, for the catch to answer with.
      const resolveService = wrapHostHandler(o.host.resolveService, o.onInternalError, request);
      const resolveLoad = wrapHostHandler(o.host.resolveLoad, o.onInternalError, request);
      const hostFault = () => resolveService.sawFault() ?? resolveLoad.sawFault();
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
          const fault = hostFault();
          if (fault !== undefined) return internalErrorProblem(fault.correlationId, headers);
          return reclassifiedFailure(operation, error, request.signal, headers, o.cors);
        }
        // Defensive: a fault always rejects (see `wrapHostHandler`), so `outcome` here
        // should never coexist with a recorded fault — but a `200`-adjacent response is
        // exactly the outcome a fault must never produce, so this is checked anyway.
        {
          const fault = hostFault();
          if (fault !== undefined) return internalErrorProblem(fault.correlationId, headers);
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
        outcome = await o.engine.queryGovernedNegotiatedAsync(o.dataset, text, {
          ...o.governors,
          ...host,
          accept,
        });
      } catch (error) {
        const fault = hostFault();
        if (fault !== undefined) {
          return internalErrorProblem(fault.correlationId, [["Vary", "Accept"], ...headers]);
        }
        return reclassifiedFailure(operation, error, request.signal, [["Vary", "Accept"], ...headers], o.cors);
      }
      // Defensive: see the update branch above — `outcome` here should never coexist
      // with a recorded fault, since a fault always rejects.
      {
        const fault = hostFault();
        if (fault !== undefined) {
          return internalErrorProblem(fault.correlationId, [["Vary", "Accept"], ...headers]);
        }
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
  } catch (error) {
    const correlationId = reportInternalError(error, o.onInternalError, request);
    return internalErrorProblem(correlationId, headers);
  }
}
