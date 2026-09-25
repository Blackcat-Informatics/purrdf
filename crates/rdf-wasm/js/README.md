<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
-->

# @blackcatinformatics/purrdf

**PurRDF** — an in-memory RDF 1.2 engine for JavaScript, compiled to
WebAssembly from the [purrdf](https://github.com/Blackcat-Informatics/purrdf)
Rust workspace, with an idiomatic [RDF/JS](https://rdf.js.org/)
(`DataFactory` / `DatasetCore` / `Stream`) API.

It is the same engine, byte-for-byte behavior, that ships as the `purrdf`
Rust crates, the `purrdf` PyPI package, and `libpurrdf` — PurRDF's rule is
**one engine, one behavior, every language**.

> **Try it live** — the [RDF-1.2 playground](https://blackcat-informatics.github.io/purrdf/playground/)
> runs this package in your browser: parse, SPARQL, SHACL, serialize, and
> canonicalize/compare RDF-1.2 graphs client-side, with no install.

## Why this instead of an incumbent RDF/JS library?

No incumbent RDF/JS library carries the RDF 1.2 features:

- **Quoted-triple terms** (RDF-star / RDF 1.2 triple terms), usable in the
  object position;
- **Base-direction literals** (`rdf:dirLangString`) — language *plus*
  `ltr`/`rtl` direction;
- Byte-deterministic serializers backed by W3C conformance corpora
  (SPARQL 1.1, RDFC-1.0 canonicalization fixtures) in the parent workspace.

## Install

```sh
npm install @blackcatinformatics/purrdf
```

Runs in Node ≥ 18 and modern browsers (ESM, `web`-target wasm-bindgen).

The wasm artifact is built with WebAssembly SIMD (`+simd128`) for higher parse
throughput, so it requires a runtime with wasm SIMD support: every major browser
since ~2021 (Chrome/Edge 91+, Firefox 89+, Safari 16.4+) and Node ≥ 18.

Asynchronous operations — jobs that suspend while the host answers a `SERVICE`
or `LOAD`, and that yield to the event loop while they evaluate — run over
WebAssembly JavaScript Promise Integration (JSPI): Chrome/Edge 137+, Firefox
139+, Safari 27, Node ≥ 24.20 and Cloudflare Workers. On a runtime without JSPI
the synchronous API is unchanged, and the asynchronous methods refuse with one
clear error before touching wasm.

## Quickstart

```js
import { ready, DataFactory, Dataset, QueryEngine } from "@blackcatinformatics/purrdf";

await ready(); // one-time async wasm instantiation

const f = new DataFactory();

// A quoted triple, usable as a subject/object (RDF-star / RDF 1.2).
const quoted = f.quotedTriple(
  f.namedNode("https://ex/alice"),
  f.namedNode("https://ex/knows"),
  f.namedNode("https://ex/bob"),
);

// A base-direction literal (rdf:dirLangString).
const rtl = f.directionalLiteral("مرحبا", "ar", "rtl");

const ds = new Dataset();
ds.add(f.quad(f.namedNode("https://ex/s"), f.namedNode("https://ex/says"), rtl));
ds.add(f.quad(f.namedNode("https://ex/stmt"), f.namedNode("https://ex/asserts"), quoted));

// Quoted triples + directions survive a round-trip through N-Quads.
const nq = ds.serialize("nquads");
const reparsed = Dataset.parse(nq, "nquads");

const engine = new QueryEngine();
const names = engine.select(
  reparsed,
  "SELECT ?message WHERE { <https://ex/s> <https://ex/says> ?message }",
);
console.log(names.rows.take(0)?.message.value);
```

Configured JSON-LD/YAML-LD calls the same Rust context engine as native PurRDF:

```js
import { CompiledJsonLdContext } from "@blackcatinformatics/purrdf";

const options = JSON.stringify({
  version: 1,
  mode: "context",
  prefixes: { ex: "https://ex/" },
});
const context = new CompiledJsonLdContext(options);
const jsonld = reparsed.serializeWithContext("jsonld", context);
```

`serializeConfigured` handles one-shot expanded, context, registry-backed, or
derived requests. Matching `QueryEngine.queryRawConfigured` and
`queryRawWithContext` methods serialize CONSTRUCT/DESCRIBE graph results.

## Graph, tabular, and research-object projection archives

Projection and lift run entirely in memory through the native Rust engine. The
caller supplies strict profile-tagged JSON; PurRDF does not fabricate vocabulary,
identity, or resource limits.

```js
import { Dataset, liftProjection, ready } from "@blackcatinformatics/purrdf";

await ready();
const config = JSON.stringify({
  profile: "lpg-csv",
  config: {
    rdf_type: "https://example.org/type",
    scope: { mode: "all" },
    limits: {
      max_artifacts: 16,
      max_artifact_bytes: 1_000_000,
      max_total_bytes: 4_000_000,
      max_archive_bytes: 5_000_000,
      max_term_depth: 16,
    },
    execution_limits: {
      max_input_records: 1_000,
      max_model_records: 1_000,
      max_nodes: 1_000,
      max_edges: 1_000,
    },
  },
});
const dataset = Dataset.parse(
  "@prefix ex: <https://example.org/> . ex:alice ex:knows ex:bob .",
  "turtle",
);
const projected = dataset.project("lpg-csv", config);
const lifted = liftProjection(projected.archive, "lpg-csv", config);
const roundTrip = lifted.takeDataset();
console.log(roundTrip?.size, JSON.parse(projected.lossLedgerJson));
```

`lpg-csv`, `neo4j-csv`, `open-cypher`, `graphml`, `csvw-exact`, `croissant-1.1`,
`ro-crate-1.3`, `datacite-4.6`, `dcat-3`, and `frictionless-data-package-1` are
bidirectional. `csvw-terms`, `okf-terms`, `obo-graphs`, `skos`, `dcat-rdf`, and
`void` are write-only, loss-ledgered views and are excluded from the
`LiftProfile` TypeScript union.
Research-object contexts, vocabularies, identities, and
profiles are mandatory caller configuration. Archives are canonical
deterministic USTAR bytes. Package/lift objects own wasm memory; call `free()`
when finished, and remember that `takeDataset()` transfers its dataset exactly
once. A runnable Node example is
[`projection-roundtrip.mjs`](https://github.com/Blackcat-Informatics/purrdf/blob/main/crates/rdf-wasm/js/examples/projection-roundtrip.mjs).

For native RDF dataset descriptions, parse the complete source dataset and pass
the same strict caller-owned JSON used by Rust and the other hosts:

```js
const dataset = Dataset.parse(sourceTrig, "trig");
const description = dataset.project("void", voidConfigJson);
const archive = description.archive;
description.free();
dataset.free();
```

The `dcat-rdf` configuration selects mapped or bounded CONSTRUCT mode; the
`void` configuration names exact graphs, every target role, dataset-prefix
ownership, and all limits. Complete examples are in
`crates/rdf/tests/fixtures/dataset-description/`.

## API surface

- `ready(bytesOrUrl?)` — one-time async wasm instantiation, from bytes, a URL, or a
  compiled `WebAssembly.Module` (what a Cloudflare Worker's `.wasm` import yields). There
  is one instance per JavaScript realm; a poisoned one (see
  [Stack regions and faults](#stack-regions-and-faults)) is never replaced.
- `DataFactory` — `namedNode`, `blankNode`, `literal`, `typedLiteral`,
  `directionalLiteral`, `variable`, `defaultGraph`, `quad`, `quotedTriple`,
  `fromTerm`, `fromQuad`.
- `Dataset` — `Dataset.parse(input, format, base?)`, `serialize(format)`,
  `serializeWithLoss(format)`, `serializeConfigured(format, optionsJson)`,
  `serializeWithContext(format, context)`,
  `add` / `delete` / `has` / `match` / `quads` / `size`, iteration.
  Formats: `turtle`, `ntriples`, `nquads`, `trig`, `rdfxml` (`serialize` also `jsonld`).
- `Dataset.serializeWithLoss(format)` — the same document `serialize` returns plus the
  realized loss of producing it (`statementRowsDropped`,
  `directionalLiteralsDropped`, `namedGraphRowsDropped`). A single-graph target drops
  every named graph and a star-incapable one drops the RDF 1.2 statement layer; these
  counts are how many, partitioned so their sum is the total.
- `Dataset.canonicalize()` / `Dataset.isomorphic(other)` — RDFC-1.0 canonical N-Quads
  and RDF graph-identity (isomorphism under blank-node relabeling).
- `Dataset.project(profile, configJson)` / `liftProjection(archive, profile,
  configJson)` — canonical graph/tabular/research-object USTAR carriers with structured,
  always-computed loss-ledger JSON.
- `Dataset.visualModel(options?)` / `visualExport(options?)` /
  `visualSvg(options?)` — the renderer-neutral RDF 1.2 model, complete semantic
  scene and deterministic geometry, or self-contained SVG paired with that export.
  Returned objects are structured-clone-safe and preserve triple-term identity,
  assertion graphs, reifier/annotation graph context, nesting, and diagnostics.
- `QueryEngine` — a reusable SPARQL execution context with a native plan cache,
  typed `select` / `ask` / `construct` / `describe` helpers, atomic `update`,
  and `queryRaw` serialization for SPARQL Results JSON/XML/CSV/TSV plus graph
  formats. SELECT `rows` are a single-owner iterable; use `take(index)`,
  `toArray()`, or iteration, and call `free()` when abandoning unconsumed rows.
  `Dataset.query(...)` remains as the compatibility raw-string helper. With no
  `format`, a graph result is Turtle — or TriG when it carries a named graph, since
  Turtle has no `GRAPH` construct and would otherwise render a quad-template
  `CONSTRUCT` as an empty document. Naming a single-graph syntax for such a result
  throws, listing the graphs and the quad-capable alternatives, rather than returning
  bytes that omit exactly what the query asked for.
- `QueryEngine.queryGoverned(dataset, sparql, options?)` /
  `updateGoverned(dataset, sparql, options?)` — the same evaluator under caller-supplied
  execution governors: `fuel`, `deadlineMs`, `maxAnswers`, `maxIntermediateCells`,
  `maxScratchBytes`, `maxRemoteRequests`, and a `CancellationToken`. Every ceiling is
  inclusive, and `0` is a valid ceiling that trips on the first charged unit of work.
  **A trip is a returned outcome, never a throw**: read `isComplete`, then `result`, or
  `tripped` with `partial` — where `partial.certainty` states what the rows in hand
  certify (`"certain"` = a lower bound, safe to admit; `"at-most"` = an upper bound;
  `"unknown"` = no rows at all, plus the operator that withheld them). Both paths carry
  `evidence`, the per-dimension consumption and ceiling maps a caller sizes the next
  budget from. A tripped UPDATE applies **nothing**. This is the ceiling a browser tab
  needs: the evaluator runs on the UI thread, so an accidental cross product with no
  deadline freezes the page.
- `QueryEngine.queryAsync` … `updateGovernedAsync`, `Dataset.queryAsync` — the
  Promise-returning twins, which take `resolveService` / `resolveLoad` handlers, a
  `ServiceCatalog`, `localServices`, an `AbortSignal` and the yielding and stack options;
  `hasAsyncQueries()`, `asyncYieldPrimitive()` and `configureAsync(...)` describe and
  configure the scheduler, and `SparqlProtocolRequest` reads a SPARQL 1.1 Protocol
  request. See
  [Asynchronous queries, federation and the Cloudflare adapter](#asynchronous-queries-federation-and-the-cloudflare-adapter).
- `QueryEngine.explainQuery(dataset, sparql, options?)` / `governorDimensions()` — the
  metered charge ledger a budget is sized from (join orders, plan estimates, per-node
  cost) and the engine's dimension vocabulary, which keys every `evidence` map.
- `shaclValidateToSarif(shapesTtl, dataNt, shapesBase?)` /
  `shaclEntail(shapesTtl, dataNt, shapesBase?)` — SHACL validation to a SARIF
  2.1.0 report and SHACL-AF `sh:rule` entailment to N-Triples. `shapesBase` is
  the base the shapes document's relative IRI references resolve against; a
  browser or Node host has no retrieval IRI of its own, so omit it and a
  relative reference throws rather than being mis-parsed (`dataNt` needs no
  counterpart — N-Triples admits no relative IRI by grammar). `Dataset.parse`
  takes the same optional third argument.
- `shaclValidateChangesToSarif(shapesTtl, dataNt, addedNt?, removedNt?, shapesBase?)`
  — validates a CHANGE to `dataNt` rather than the whole graph: hand it the rows
  joining and the rows leaving, and the engine re-validates only the focus nodes
  that change can move. Returns a `ShaclChangeValidation`; read `bounded` before
  the log, because it decides what the log MEANS. `true` and an empty log means
  *this change introduced no violation*; `false` means the shapes graph reads
  through SPARQL query text, no bounded footprint exists for it, the call fell
  back to a FULL validation, and an empty log means *the graph conforms*. Call
  `free()` when done.
- `entailMaterialize(document, regime, program)` — SPARQL entailment-**regime**
  materialization over all SEVEN regimes (`"simple"` / `"rdf"` / `"rdfs"` /
  `"owl-rl"` / `"d"` / `"owl-direct"` / `"rif"` — none is refused), returning
  `{ nquads, report }`: the canonical N-Quads closure and a byte-stable reasoning
  report. Unlike `shaclEntail` it takes no shapes graph — it closes the document
  under the regime's own specification rule table. The report is never optional:
  it names which rules fired, which specification rules did **not**, which
  constructs were left at a boundary, the evaluation budget and the calculus's
  contract hash, so "OWL-RL entailment" can never be claimed without saying how
  much of OWL-RL actually ran.
- `entailRules(regime)` / `entailImplementedRules(regime)` — the rule table the
  specification *defines* the regime by, and the subset this build fires. The
  difference is the measurable gap, and is exactly the report's `missing` lines.
- `entailCheckGoldenVectors()` — run the project's committed cross-host golden
  vector artifact through the wasm you actually loaded and throw on the first
  byte that differs from the reference (native Rust) implementation.
- `Sink`, `datasetToStream`, `streamToDataset` — the async RDF/JS
  Stream/Sink primitives over the synchronous engine surface.
- SPARQL evaluation over the in-memory dataset (no server required).

Full typings ship in `index.d.ts`.

```js
const { svg, export: graph } = reparsed.visualSvg({
  mode: "compact",
  vocabulary: [{ prefix: "ex", namespace: "https://ex/" }],
  svg: { title: "RDF 1.2 graph", embedMetadata: true },
});

console.log(graph.model.statements, graph.model.relations);
document.querySelector("#graph").innerHTML = svg;
```

Compact mode keeps ordinary asserted RDF as directed predicate-labelled edges and
promotes statements only when they need identity. Incidence mode exposes exact
subject/predicate/object ports. Table mode scales statement inspection without
discarding the same underlying model.

## Asynchronous queries, federation and the Cloudflare adapter

The synchronous methods are the offline lane: they install no `SERVICE` or `LOAD`
source, so a non-`SILENT` `SERVICE` or `LOAD` fails by name. Every evaluating method
also has a Promise-returning twin that takes the host's handlers for those two
clauses:

- on `QueryEngine`: `queryAsync`, `selectAsync`, `askAsync`, `constructAsync`,
  `describeAsync`, `queryRawAsync`, `queryRawBytesAsync`, `queryRawWithContextAsync`,
  `queryGovernedAsync`, `queryEntailmentGovernedAsync`, `updateAsync` and
  `updateGovernedAsync`, plus `queryGovernedNegotiatedAsync` (a governed query answered
  as a document in the format negotiated from an HTTP `Accept` header, which has no
  synchronous twin);
- on `Dataset`: `queryAsync`.

Each twin runs the same evaluator as its synchronous twin, over a snapshot of the
dataset taken when the call starts, and resolves to exactly the shape the synchronous
twin returns. It runs as a *job*: the job suspends while the host answers a `SERVICE`
or `LOAD`, and it gives the event loop back at regular intervals while it evaluates.
The host does the I/O and owns its policy. PurRDF keeps the parsing, the evaluation,
the joins, the `SILENT` semantics and the result encoding.

### Hosts

The twins run over WebAssembly JavaScript Promise Integration (JSPI), which is on by
default in Chrome and Edge 137+, Firefox 139+, Safari 27, Node 24.20+ and Cloudflare
Workers (workerd). `hasAsyncQueries()` reports whether the current engine has it. Where
it does not, every twin rejects with one error naming what is missing before it touches
wasm, and the synchronous API works as before.

### Answering `SERVICE`: `resolveService`

`resolveService(request, ctx)` is called once for each `SERVICE` request a job issues,
and may return its answer or a Promise of it. `request` is the SPARQL 1.1 Protocol POST
to send:

- `endpoint`: the service IRI;
- `queryText`: the forwarded query;
- `contentType`: `application/sparql-query`;
- `accept`: `application/sparql-results+json`;
- `userAgent`;
- `timeoutMs`: the catalog profile's timeout, or the default;
- `headers`: the catalog profile's headers and then its credential header, as
  `[name, value]` pairs in sending order. Append each pair and never merge repeated
  names. Without a catalog the list is empty.

`ctx` carries four fields:

- `signal`: an `AbortSignal` that fires when the job is cancelled or its deadline
  passes. From then on the job no longer waits for the handler.
- `remainingDeadlineMs`: the time left before the deadline, when the job has one.
- `silent`: whether the clause was written `SERVICE SILENT`.
- `maxIntermediateCells`: the query's cell ceiling, when one is set.

The answer is one of:

- SPARQL Results JSON as a `Uint8Array`, an `ArrayBuffer` or a string;
- a `Response`, whose 2xx body is read as SPARQL Results JSON. Any other status is a
  transport failure;
- `{ kind: "transport", message }` when the endpoint could not be reached or read.
  `SERVICE SILENT` swallows this failure and contributes the join identity, so the
  surrounding pattern's own solutions come back unextended. Without `SILENT` the
  query fails;
- `{ kind: "denied", message }` when the host's policy refuses the request. This
  failure fails the query even under `SERVICE SILENT`.

A handler that throws, rejects, or returns anything else has *faulted*. A fault fails
the job even under `SERVICE SILENT`, because a fault is not an answer. `ctx.silent` is
for information only: an empty answer is not the handler's to invent, and the failure
it reports decides what `SILENT` does with it.

This handler sends each request with `fetch`:

```js resolve-service-recipe
import { ready, Dataset, QueryEngine } from "@blackcatinformatics/purrdf";

await ready();

// One SERVICE request, sent as a SPARQL 1.1 Protocol POST.
async function resolveService(request, { signal }) {
  try {
    // A Response is an answer as it stands: a 2xx body is read as SPARQL Results
    // JSON, and any other status is a transport failure.
    return await fetch(request.endpoint, {
      method: "POST",
      headers: [
        ["Content-Type", request.contentType], // application/sparql-query
        ["Accept", request.accept], // application/sparql-results+json
        ...request.headers, // the catalog profile's headers, in sending order
      ],
      body: request.queryText,
      signal: AbortSignal.any([signal, AbortSignal.timeout(request.timeoutMs)]),
    });
  } catch (error) {
    // Never rethrow: a throw is a fault, which fails the query even under SERVICE SILENT.
    return { kind: "transport", message: String(error) };
  }
}

const engine = new QueryEngine();
const dataset = Dataset.parse(
  "<https://example.org/a> <https://example.org/p> <https://example.org/o1> .\n",
  "nquads",
);
const { rows } = await engine.selectAsync(
  dataset,
  `SELECT ?s ?x WHERE {
     ?s <https://example.org/p> ?o
     SERVICE <https://remote.example.org/sparql> { ?o <https://example.org/q> ?x }
   }`,
  { resolveService },
);
for (const row of rows) console.log(row.s.value, row.x.value);
```

A `ServiceCatalog` passed as `catalog` authorizes every request before the handler is
called. It denies by default, holds one profile per endpoint and an optional fallback,
and a profile grants the capabilities `query`, `network` and `credentials` and may add
headers, a credential header, a `User-Agent` and a timeout. A denied request fails the
query even under `SERVICE SILENT`. `localServices: { [endpoint]: dataset }` answers the
named endpoints in process from a snapshot of a `Dataset`, without calling the handler.

In a browser, the remote endpoint's CORS policy governs whether `fetch` can read its
answer: a cross-origin endpoint that does not allow the page's origin surfaces as a
network error, which the handler above reports as a transport failure. Write
`SERVICE SILENT` where an endpoint may be unreachable and its rows are optional.

### Answering `LOAD`: `resolveLoad`

`resolveLoad({ kind: "load", iri }, { signal })` answers one `LOAD` with any of:

- `{ bytes, mediaType, base? }` or `{ text, mediaType, base? }`. `mediaType` is a media
  type or any format name `Dataset.parse` accepts, and `base` defaults to the IRI;
- a `Response`, whose `Content-Type` names the media type;
- a `Dataset`;
- `{ kind: "transport" }`, which `LOAD SILENT` swallows;
- `{ kind: "denied" }`, which fails the request even under `LOAD SILENT`.

A bare string or bytes carry no media type and are a fault. A document that does not
parse is the `LOAD`'s own failure.

### Yielding, cancellation and deadlines

A job counts the evaluator's governor polls and gives the event loop one turn every
`yieldEveryPolls` polls: 65 536 by default, and `0` yields at every poll. The count, not
the clock, decides when to yield. It yields through one macrotask primitive, chosen when
the module loads and reported by `asyncYieldPrimitive()`: `scheduler.yield`,
`setImmediate` or `MessageChannel`, in that order of preference. Only evaluation yields
(an entailment closure included). Freezing the dataset before the job and serializing
the result after are linear passes that run to completion. `evidence.async` reports
what each phase cost (`freezeMs`, `evaluateMs`, `serializeMs`).

`signal: AbortSignal` cancels a job. The job observes it at the next yield or host
effect, and the twin rejects with `signal.reason` (an `AbortError` when the signal has no
reason). A governed twin reports the cancellation as a tripped governor instead. Passing
`cancel` (a `CancellationToken`) to a twin is a `TypeError` that names `signal`. On the
governed twins, `deadlineMs` includes the time spent waiting for the handlers as well as
evaluation. The job checks it at every yield and every host effect, and a timer aborts an
effect still pending when it passes.

On Cloudflare Workers, `Date.now()` does not advance during CPU-bound execution; it moves
only across I/O. A synchronous `deadlineMs` therefore cannot trip during CPU-bound work
there. The asynchronous lane observes the deadline at every yield and every effect, which
is where the clock moves.

### Concurrency

Jobs interleave at every yield and effect, and synchronous calls may run between them. A
query reads the snapshot taken when it started, so a later mutation never shows up in a
running job. Asynchronous updates on one dataset run one at a time, in call order. Each
update reads a snapshot and is applied only if the dataset was not mutated while it ran;
otherwise it rejects and applies nothing. `dataset.id` identifies a dataset within the
wasm instance, and `dataset.generation` counts the mutations it has seen.
`configureAsync({ maxConcurrentJobs })` bounds how many jobs may be in flight (16 by
default); a twin started beyond the bound rejects. Identical `SERVICE` requests in flight
through the same `resolveService` share one call.

### Stack regions and faults

Each job evaluates on its own stack region of `stackBytes` bytes (2 MiB by default, at
least 524 288). `evidence.async.stackHighWaterBytes` reports the deepest the job went, so
the region can be sized from a real run. A request that nests deeper than the region
allows fails with a typed error that says to raise `stackBytes`. The job fails and the
instance stays usable. If a job traps, or its frames ever run past the guard zone below
its region, the instance's state can no longer be trusted: the trap leaves the job's
stack context in place of the caller's and anything the job was mutating half-changed,
and an overrun may have overwritten memory outside the job. The instance is then
*poisoned*, and it cannot be used again: every in-flight job rejects, and every later
call — synchronous or asynchronous, a constructor, a static, a free function, or a
method of an object created before the trap — throws the same error. `ready()` rejects
with it too, because there is one instance per JavaScript realm; only a fresh realm (a
new page, Worker isolate or process) can load the package again. `free()` is the one
call that does not throw: it releases nothing, because the instance's memory is
abandoned whole.

### The Cloudflare adapter

`@blackcatinformatics/purrdf/cloudflare` builds a SPARQL 1.1 Protocol endpoint from these
pieces:

- `createFetchServiceResolver({ catalog, timeoutMs, fetch?, bindings?, cache?, cacheTtlSeconds?, waitUntil?, onCacheError? })`
  returns a `resolveService` that POSTs each request with `fetch`, or through the
  service binding registered for the endpoint's origin, always with `redirect: "manual"`.
  A network error, a timeout or a non-2xx status is reported as `{ kind: "transport" }`,
  never thrown — and so is a 3xx (or a browser's opaque-redirect response): it is never
  followed, so the profile's headers and credential (an `X-Api-Key`, a `Cookie`, …) can
  never reach an origin the catalog did not authorize. With `cache` and `cacheTtlSeconds`
  it reuses answers through the Cache API, and it refuses a request that carries a
  credential with a `TypeError` (a fault) rather than read it from or write it to a shared
  cache. The cache is an optimisation and never decides the answer: a `cache.match`
  rejection (e.g. a workerd Worker with no Cache configured) is treated as a miss, and a
  `cache.put` failure never discards an answer the remote already returned — with
  `waitUntil` the failed put is still handed to it, and without one it is awaited inside a
  `try`. Every cache failure is reported through `onCacheError(error, { operation, endpoint })`,
  which defaults to one `console.warn` line, so a failure is visible, never silent —
  including under `SERVICE SILENT`, which still yields the join identity when the remote
  itself fails, not a fault, regardless of the cache's own health.
- `createFetchLoadResolver({ catalog, timeoutMs, fetch?, bindings?, maxRedirects? })`
  returns a `resolveLoad` that authorizes each IRI against the catalog and then GETs it,
  also with `redirect: "manual"`. A redirect is followed by hand, up to `maxRedirects`
  hops (5 by default): its `Location` is resolved and re-authorized against the catalog
  exactly as the initial IRI is — an unauthorized hop is the same `{ kind: "denied" }`
  failure as an unauthorized initial `LOAD` — and that hop's own headers and credential
  are sent, never the previous hop's. The loaded document's `base` is the final, redirected
  and authorized URL. Exceeding `maxRedirects`, or a redirect with no usable `Location`
  (an opaque one withholds it), is a `{ kind: "transport" }` failure.
- `handleSparqlRequest(request, options)` answers one protocol request (`GET ?query=`,
  or a `POST` of `application/sparql-query`, `application/sparql-update` or a form) with
  a `Response`. The statuses are `200` with the negotiated document, `204` for an
  applied update, `400`/`405`/`415` for a malformed request, `406` when no acceptable
  format can carry the result, `413` when the body exceeds `maxRequestBytes`, `422` when
  a deterministic ceiling stopped the request, `503` when the deadline or a cancellation
  did, and `500` when evaluation failed. A partial answer is never sent with a `200`.
  Every error body is `application/problem+json` (RFC 9457) with a stable `code`, and
  every evaluated response carries `Server-Timing` from the job's evidence. The `cors`
  option answers preflights and adds `Access-Control-Allow-Origin`; without it no CORS
  header is sent. `governors.deadlineMs` is required. `maxRequestBytes` (1 MiB by
  default: a SPARQL query or update's text is a program, not a payload, and 1 MiB
  comfortably covers even a large one) bounds the request body — a `Content-Length`
  above it is refused before anything is read, and a missing or understated one is still
  caught by counting bytes as the body streams in, so a lying header never buys a larger
  body than an honest one would. A `500`'s `detail` is the engine's own words only when
  the failure is the query's — a parse, an evaluation, a tripped governor: a SPARQL
  client is owed the reason its request failed. A bug this endpoint cannot attribute to
  the query itself — `resolveService`/`resolveLoad` throwing or rejecting, or any other
  exception this adapter did not otherwise classify — never puts its own message or stack
  in the response: it gets a fixed generic `detail`, `code: "InternalError"` and a fresh
  `correlationId`, while the real error goes to exactly one place, `onInternalError(error,
  { correlationId, request })` (one `console.error(error, correlationId)` line by
  default), so an operator can always join what the client saw to what actually broke.

A complete Worker:

```js worker-recipe
import wasm from "@blackcatinformatics/purrdf/purrdf_wasm_bg.wasm";
import { ready, Dataset, QueryEngine, ServiceCatalog } from "@blackcatinformatics/purrdf";
import { createFetchServiceResolver, handleSparqlRequest } from "@blackcatinformatics/purrdf/cloudflare";

await ready(wasm);
const engine = new QueryEngine();
const dataset = Dataset.parse(
  "<https://example.org/a> <https://example.org/p> <https://example.org/o1> .\n",
  "nquads",
);
const catalog = new ServiceCatalog();
catalog.addService(
  "https://remote.example.org/sparql",
  JSON.stringify({ capabilities: ["query", "network"] }),
);

export default {
  fetch(request, env, ctx) {
    const resolveService = createFetchServiceResolver({
      catalog,
      timeoutMs: 5_000,
      bindings: { "https://remote.example.org": env.REMOTE },
      cache: caches.default,
      cacheTtlSeconds: 300,
      waitUntil: (promise) => ctx.waitUntil(promise),
    });
    return handleSparqlRequest(request, {
      engine,
      dataset,
      catalog,
      resolveService,
      cors: { origins: "*" },
      governors: { deadlineMs: 10_000, maxRemoteRequests: 40 },
    });
  },
};
```

Workers limits how many subrequests one invocation may make. `maxRemoteRequests` is the
exact control for that limit. Every `SERVICE` request and every `LOAD` is charged against
it before it reaches the handler, including one the cache then answers, so a request
never makes more subrequests than the ceiling. Set it to the subrequests you allow one
query. The Cache API does nothing on `workers.dev` hostnames, so the `cache` option takes
effect only on a Worker served from a custom domain — there `cache.match` and `cache.put`
reject with "No Cache was configured", which is exactly the failure `onCacheError` reports
while the query still answers from the remote.

## Scope

In-memory only, by design: no persistent store and no network I/O inside the
wasm module. The synchronous methods install no `SERVICE` or `LOAD` source, so
there a remote `SERVICE` or `LOAD` fails explicitly unless it is written `SILENT`.
The asynchronous twins reach remote endpoints only through the handlers the host
passes them (`resolveService`, `resolveLoad`), or through the Cloudflare adapter's
`fetch`-based handlers. For the container transport (GTS), native APIs, and the
rest of the toolkit, see the
[main repository](https://github.com/Blackcat-Informatics/purrdf).

## Supply chain

Published from GitHub Actions via npm trusted publishing with sigstore
provenance, a GitHub build-provenance attestation, and an SPDX SBOM per
release.

## License

[MIT OR Apache-2.0 OR MulanPSL-2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSING.md),
© 2026 Blackcat Informatics® Inc.
