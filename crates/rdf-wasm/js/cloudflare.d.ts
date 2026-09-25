// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Types for `@blackcatinformatics/purrdf/cloudflare`. The host objects are described
// structurally, so the adapter type-checks against the Workers runtime types, the DOM
// library, or a test double alike.

import type {
  AsyncLoadResolver,
  AsyncServiceResolver,
  Dataset,
  GovernorCeiling,
  QueryEngine,
  ServiceCatalog,
} from "./index.js";

/** A `fetch`-shaped function: `globalThis.fetch`, or a test double. */
export type FetchLike = (input: string | URL | Request, init?: RequestInit) => Promise<Response>;

/** A Workers service binding (or anything with a `fetch` method). */
export interface ServiceBindingLike {
  fetch(input: string | URL | Request, init?: RequestInit): Promise<Response>;
}

/** A Cache API object such as `caches.default`. */
export interface CacheLike {
  match(request: Request): Promise<Response | undefined>;
  put(request: Request, response: Response): Promise<void>;
}

/** A Workers `ExecutionContext`: `waitUntil` keeps a promise alive past the response. */
export interface ExecutionContextLike {
  waitUntil(promise: Promise<unknown>): void;
}

/** Which Cache API call failed, and the `SERVICE` endpoint whose answer it was for. */
export interface CacheErrorContext {
  readonly operation: "match" | "put";
  readonly endpoint: string;
}

/**
 * Reports a Cache API failure. The cache is an optimisation: a `match` rejection is
 * treated as a miss (the request still goes to the remote) and a `put` failure never
 * discards an answer the remote already returned — this is called either way, so a
 * failure is never silent. Defaults to one `console.warn` line (Workers routes it to
 * logs).
 */
export type CacheErrorReporter = (error: unknown, context: CacheErrorContext) => void;

/** Service bindings by origin (`"https://example.org"`, no path, no trailing slash). */
export type ServiceBindings = Readonly<Record<string, ServiceBindingLike>>;

export interface FetchServiceResolverOptions {
  /** The policy every request is authorized against (required; the same catalog the query runs with). */
  readonly catalog: ServiceCatalog;
  /** Each request's own bound, in milliseconds (a positive integer; required). */
  readonly timeoutMs: number;
  /** Defaults to `globalThis.fetch`. */
  readonly fetch?: FetchLike;
  /** A service binding per origin; a request to that origin goes through it instead of `fetch`. */
  readonly bindings?: ServiceBindings;
  /** Reuse answers through this cache. Needs `cacheTtlSeconds`. */
  readonly cache?: CacheLike;
  /** How long a cached answer may be reused, in seconds (a positive integer). */
  readonly cacheTtlSeconds?: number;
  /** Defers cache writes past the response, e.g. `(p) => ctx.waitUntil(p)`. Needs `cache`. */
  readonly waitUntil?: ExecutionContextLike["waitUntil"];
  /**
   * Reports a `cache.match` or `cache.put` failure. Needs `cache`. Defaults to one
   * `console.warn` line; a cache failure never fails the request or discards an answer.
   */
  readonly onCacheError?: CacheErrorReporter;
}

export interface FetchLoadResolverOptions {
  /** The policy every `LOAD` is authorized against — the initial IRI, and every redirect hop (required). */
  readonly catalog: ServiceCatalog;
  /** Each hop's own bound, in milliseconds (a positive integer; required). */
  readonly timeoutMs: number;
  readonly fetch?: FetchLike;
  readonly bindings?: ServiceBindings;
  /** Redirect hops a `LOAD` follows before failing typed transport. Defaults to 5. */
  readonly maxRedirects?: number;
}

/**
 * A `resolveService` handler that sends each `SERVICE` request with `fetch`, with
 * `redirect: "manual"`: a 3xx is a `{ kind: "transport" }` failure, never followed, so a
 * catalogued endpoint's headers and credential can never reach an origin the catalog did
 * not authorize.
 */
export function createFetchServiceResolver(
  options: FetchServiceResolverOptions,
): AsyncServiceResolver;

/**
 * A `resolveLoad` handler that fetches each `LOAD` document with `redirect: "manual"`,
 * following a redirect by hand — re-authorizing every hop against `catalog` and
 * re-deriving that hop's own headers, up to `maxRedirects` hops — rather than letting
 * `fetch` carry headers or a credential across an origin change on its own.
 */
export function createFetchLoadResolver(options: FetchLoadResolverOptions): AsyncLoadResolver;

/** The ceilings every request runs under. `deadlineMs` is required. */
export interface EndpointGovernors {
  readonly deadlineMs: GovernorCeiling;
  readonly fuel?: GovernorCeiling | null;
  /** Bounds query answers; not applied to an update, which has none. */
  readonly maxAnswers?: GovernorCeiling | null;
  readonly maxIntermediateCells?: GovernorCeiling | null;
  readonly maxScratchBytes?: GovernorCeiling | null;
  readonly maxRemoteRequests?: GovernorCeiling | null;
}

export interface EndpointCors {
  /** `"*"`, or the origins whose requests are echoed in `Access-Control-Allow-Origin`. */
  readonly origins: "*" | readonly string[];
  /** `Access-Control-Max-Age` on preflight responses, in seconds. */
  readonly maxAgeSeconds?: number;
}

export interface SparqlEndpointOptions {
  readonly engine: QueryEngine;
  readonly dataset: Dataset;
  readonly governors: EndpointGovernors;
  readonly resolveService?: AsyncServiceResolver | null;
  readonly resolveLoad?: AsyncLoadResolver | null;
  /** Authorizes host-resolved `SERVICE` requests; needs `resolveService`. */
  readonly catalog?: ServiceCatalog | null;
  readonly localServices?: Readonly<Record<string, Dataset>> | null;
  /** Without it, no CORS header is sent and `OPTIONS` is a `405`. */
  readonly cors?: EndpointCors | null;
  readonly yieldEveryPolls?: number | null;
  readonly stackBytes?: number | null;
  /**
   * Bounds the request body, in bytes (a positive integer). A `Content-Length` above it is
   * refused with a `413` before anything is read; a missing or understated one is still
   * caught by counting bytes as the body streams in, so a lying header can never buy a
   * larger body than an honest one would. Defaults to 1 MiB — a SPARQL query or update's
   * text is a program, not a payload, and 1 MiB comfortably covers even a large one while
   * bounding what an unauthenticated request can make the host buffer.
   */
  readonly maxRequestBytes?: number | null;
}

/**
 * The `application/problem+json` body of every error response (RFC 9457). `code` is the
 * refusal's stable name: a protocol error's name (`"MissingOperation"`, …), a tripped
 * governor's label (`"fuel-exhausted"`, `"deadline-exceeded"`, …), `"NotAcceptable"`,
 * `"ContentTooLarge"` (the body exceeded `maxRequestBytes`), or the evaluation error's
 * name.
 */
export interface SparqlProblem {
  readonly type: "about:blank";
  readonly title: string;
  readonly status: 400 | 405 | 406 | 413 | 415 | 422 | 500 | 503;
  readonly detail: string;
  readonly code: string;
  readonly parameter?: string;
  readonly dimension?: string;
  readonly limit?: number;
  readonly consumed?: number;
  readonly estimate?: number;
  readonly cause?: string;
  readonly offered?: string[];
}

/** Answer one SPARQL 1.1 Protocol request. */
export function handleSparqlRequest(
  request: Request,
  options: SparqlEndpointOptions,
): Promise<Response>;
