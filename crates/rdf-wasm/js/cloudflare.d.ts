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
  XPathRegexLaw,
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

/** The correlation id a `500` response carries, and the request it was answering. */
export interface InternalErrorContext {
  readonly correlationId: string;
  readonly request: Request;
}

/**
 * Reports an error `handleSparqlRequest` never puts in front of the client: a bug in a
 * host-supplied `resolveService`/`resolveLoad` (a throw, a rejection — never a SPARQL
 * client's own concern), or any other exception this adapter did not otherwise classify.
 * The response gets a fixed generic `detail` and `correlationId` instead of `error`'s own
 * words; this is the only place `error` (and, for an `Error`, its `stack`) is ever
 * written. Defaults to one `console.error(error, correlationId)` line. A reporter that
 * throws, or returns a promise that rejects, never changes the response — it is still the
 * sanitized `500` — and its failure goes to `console.error` together with `error`.
 */
export type InternalErrorReporter = (
  error: unknown,
  context: InternalErrorContext,
) => void | PromiseLike<unknown>;

/** Service bindings by origin (`"https://example.org"`, no path, no trailing slash). */
export type ServiceBindings = Readonly<Record<string, ServiceBindingLike>>;

export interface FetchServiceResolverOptions {
  /** Defaults to `globalThis.fetch`. */
  readonly fetch?: FetchLike;
  /** A service binding per origin; a request to that origin goes through it instead of `fetch`. */
  readonly bindings?: ServiceBindings;
  /**
   * Reuse the answer of every request the job marks `cacheable` through this cache. Needs
   * `cacheTtlSeconds`. A failed `match` or `put` is the request's transport failure.
   */
  readonly cache?: CacheLike;
  /** How long a cached answer may be reused, in seconds (a positive integer). */
  readonly cacheTtlSeconds?: number;
}

export interface FetchLoadResolverOptions {
  readonly fetch?: FetchLike;
  readonly bindings?: ServiceBindings;
}

/**
 * A `resolveService` handler that sends each `SERVICE` request with `fetch`, bounded by
 * `ctx.signal`, with `redirect: "manual"`: a 3xx is a `{ kind: "transport" }` failure,
 * never followed, so a catalogued endpoint's headers and credential can never reach an
 * origin the catalog did not authorize.
 */
export function createFetchServiceResolver(
  options?: FetchServiceResolverOptions,
): AsyncServiceResolver;

/**
 * A `resolveLoad` handler that fetches each `LOAD` hop the job asks for with
 * `redirect: "manual"`, and answers a redirect `{ kind: "redirect", location }` for the
 * job to resolve, re-authorize against its catalog and ask for with that hop's own
 * headers — rather than letting `fetch` carry headers or a credential across an origin
 * change on its own.
 */
export function createFetchLoadResolver(options?: FetchLoadResolverOptions): AsyncLoadResolver;

/** The ceilings every request runs under. `deadlineMs` is required. */
export interface EndpointGovernors {
  /** Remove resource caps and accounting; deadlineMs remains mandatory and active. */
  readonly noCeiling?: boolean;
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
  /** Authorizes host-resolved `SERVICE` requests and `LOAD` fetches, and bounds each by its profile's `timeoutMs`. */
  readonly catalog?: ServiceCatalog | null;
  readonly localServices?: Readonly<Record<string, Dataset>> | null;
  /** Without it, no CORS header is sent and `OPTIONS` is a `405`. */
  readonly cors?: EndpointCors | null;
  readonly yieldEveryPolls?: number | null;
  /**
   * Bounds the request body, in bytes (a positive integer). A `Content-Length` above it is
   * refused with a `413` before anything is read; a missing or understated one is still
   * caught by counting bytes as the body streams in, so a lying header can never buy a
   * larger body than an honest one would. Defaults to 1 MiB — a SPARQL query or update's
   * text is a program, not a payload, and 1 MiB comfortably covers even a large one while
   * bounding what an unauthenticated request can make the host buffer.
   */
  readonly maxRequestBytes?: number | null;
  /**
   * Reports an error this endpoint never describes to the client: a bug in
   * `resolveService`/`resolveLoad`, a `SERVICE` or `LOAD` no resolver reaches, a
   * `resolveLoad` answer that is not one, a rejection no engine code classifies, or any
   * other exception this adapter did not otherwise classify. Defaults to one
   * `console.error(error, correlationId)` line; the response still gets a `500` with a
   * fixed `detail` and the same `correlationId`. A `resolveService`/`resolveLoad` bug
   * under `SERVICE SILENT` or `LOAD SILENT` is reported here too, while the response is
   * the clause's own answer and the job's evidence records the silenced `"fault"`.
   */
  readonly onInternalError?: InternalErrorReporter | null;
  /**
   * The dated native XPath law every query and update's `REGEX`/`REPLACE` evaluates under,
   * as `QueryOptions.xpathRegex` takes it. Omitted, the compatibility regex. A resource
   * refusal under the law is the operation's evaluation failure, answered as a `500` with
   * the resource's code (`xpath-pattern-bytes`, …), never a `200` with a partial body.
   */
  readonly xpathRegex?: XPathRegexLaw | null;
}

/**
 * The `application/problem+json` body of every error response (RFC 9457). `code` is the
 * refusal's stable name: a protocol error's name (`"MissingOperation"`, …), a tripped
 * governor's label (`"fuel-exhausted"`, `"deadline-exceeded"`, …), `"NotAcceptable"`,
 * `"ContentTooLarge"` (the body exceeded `maxRequestBytes`), `"InternalError"` (a host bug,
 * a rejection no engine code classifies, or an unexpected exception; `detail` is generic
 * and `correlationId` is the only lead, shared with the matching `onInternalError` call),
 * or the engine's own diagnostic code:
 *
 * - `400`, an operation that does not parse or that the engine refuses to evaluate as
 *   written (`detail` is the engine's message): `"native-sparql-query-parse"`,
 *   `"native-sparql-update-parse"`, `"native-sparql-unsupported"`,
 *   `"native-sparql-custom-function"`, `"native-sparql-quoted-triple-term-variable"`,
 *   `"native-sparql-host-stack-exhausted"`;
 * - `403`, the host refused to contact a `SERVICE` endpoint or `LOAD` source the request
 *   named: `"native-sparql-service-denied"` and `"native-sparql-load-denied"` (the catalog
 *   withheld a capability), `"native-sparql-service-host-denied"` and
 *   `"native-sparql-load-host-denied"` (the resolver's own policy);
 * - `409`, another update of the dataset is in flight: `"native-sparql-update-in-flight"`;
 * - `502`, the endpoint or source was contacted and gave no usable answer (a network
 *   error, its timeout, an HTTP error status, a redirect, an undecodable body):
 *   `"native-sparql-service-failed"`, `"native-sparql-load-failed"`,
 *   `"native-sparql-load-decode"`;
 * - `500` with a `correlationId`, this endpoint's own fault:
 *   `"native-sparql-service-unconfigured"` and `"native-sparql-load-no-resolver"` (no
 *   resolver reaches the named endpoint or source);
 * - `500`, the query's own evaluation failed (`detail` is the engine's message): any other
 *   engine code, such as `"native-sparql-query-eval"` or
 *   `"native-sparql-evaluation-stack-exhausted"`.
 *
 * A `403`, `502` or `500`-with-`correlationId` `detail` is a fixed description of its
 * code, never the engine's message, which would echo the catalog's policy or a
 * resolver's or remote's own words.
 */
export interface SparqlProblem {
  readonly type: "about:blank";
  readonly title: string;
  readonly status: 400 | 403 | 405 | 406 | 409 | 413 | 415 | 422 | 500 | 502 | 503;
  readonly detail: string;
  readonly code: string;
  readonly parameter?: string;
  readonly dimension?: string;
  readonly limit?: number;
  readonly consumed?: number;
  readonly estimate?: number;
  readonly cause?: string;
  readonly offered?: string[];
  /**
   * On a `500` this endpoint answers for its own fault (`code: "InternalError"`,
   * `"native-sparql-service-unconfigured"`, `"native-sparql-load-no-resolver"`): the id
   * `onInternalError` was also handed.
   */
  readonly correlationId?: string;
}

/** Answer one SPARQL 1.1 Protocol request. */
export function handleSparqlRequest(
  request: Request,
  options: SparqlEndpointOptions,
): Promise<Response>;
