<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Getting Started: JavaScript / WebAssembly

The npm package
[`@blackcatinformatics/purrdf`](https://www.npmjs.com/package/@blackcatinformatics/purrdf)
is the same Rust engine compiled to `wasm32` and surfaced through an
[RDF/JS](https://rdf.js.org/)-shaped API (`DataFactory`, `DatasetCore`,
`Stream`/`Sink`). It runs in the browser and in Node, entirely in memory.

```sh
npm install @blackcatinformatics/purrdf
```

Prefer to try it before installing anything? The
[RDF-1.2 playground](https://blackcat-informatics.github.io/purrdf/playground/)
runs this exact wasm build in your browser — parse, SPARQL, SHACL, serialize, and
canonicalize/compare RDF-1.2 graphs client-side, with no toolchain and no server.

## First dataset

Await `ready()` once before anything else — it performs the one-time async
wasm instantiation:

```js
import { ready, DataFactory, Dataset, QueryEngine } from "@blackcatinformatics/purrdf";

await ready(); // one-time async wasm instantiation

const f = new DataFactory();
const rtl = f.directionalLiteral("مرحبا", "ar", "rtl");

const ds = new Dataset();
ds.add(f.quad(f.namedNode("https://ex/s"), f.namedNode("https://ex/says"), rtl));

const nq = ds.serialize("nquads");           // directions survive the round-trip
const reparsed = Dataset.parse(nq, "nquads");

const engine = new QueryEngine();
const ask = engine.ask(reparsed, "ASK { <https://ex/s> <https://ex/says> ?msg }");
```

## The RDF 1.2 wedge

No incumbent RDF/JS library carries RDF 1.2 **quoted-triple terms** or
**directional literals**. PurRDF's `DataFactory` exposes both:

```js
// A quoted triple, usable as a subject/object (RDF-star / RDF 1.2).
const quoted = f.quotedTriple(
  f.namedNode("https://ex/alice"),
  f.namedNode("https://ex/knows"),
  f.namedNode("https://ex/bob"),
);

// A base-direction literal (rdf:dirLangString).
const hello = f.directionalLiteral("مرحبا", "ar", "rtl");
```

## API surface

- **`ready(bytesOrUrl?)`** — await once before anything else; it also accepts
  a compiled `WebAssembly.Module`.
- **`DataFactory`** — `namedNode`, `blankNode`,
  `literal(value, languageOrDatatype?)`, `typedLiteral`,
  `directionalLiteral`, `variable`, `defaultGraph`, `quad`, `quotedTriple`,
  `fromTerm`, `fromQuad`.
- **`Dataset`** (RDF/JS `DatasetCore`) — `Dataset.parse(input, format, base?)`,
  `serialize(format)`, `add`/`delete`/`has`/`match`/`quads`/`size`, and
  iteration (`for (const quad of dataset)`). Formats: `turtle`, `ntriples`,
  `nquads`, `trig`, `rdfxml` (or their media types); `serialize` additionally
  accepts `jsonld`.
- **Graph identity** — `Dataset.canonicalize()` returns the RDFC-1.0 canonical,
  flat N-Quads for the graph; `Dataset.isomorphic(other)` decides RDF graph
  equality under blank-node relabeling (an exact oracle backed by full RDFC-1.0
  canonicalization).
- **Graph/tabular/research-object carriers** — `Dataset.project(profile, configJson)` returns
  canonical USTAR bytes and loss-ledger JSON; `Dataset.projectWithAssets("ro-crate-1.3",
  configJson, payloadArchive)` adds bounded attached RO-Crate payloads;
  `liftProjection(...)` reconstructs RDF for the bidirectional profiles. See
  [Graph, Tabular & Research-Object Projections](../concepts/projections.md).
- **SPARQL** — `QueryEngine` keeps the native plan cache alive across calls and
  exposes typed `select` / `ask` / `construct` / `describe`, atomic `update`,
  and `queryRaw` serialization. `Dataset.query(...)` remains the compatibility
  raw-string helper. Each evaluating method has a Promise-returning twin that
  takes host handlers for `SERVICE` and `LOAD` — see
  [below](#asynchronous-queries-and-federation).
- **SHACL** — `shaclValidateToSarif(shapesTtl, dataNt)` validates an N-Triples
  data graph against a Turtle shapes graph and returns a SARIF 2.1.0 report;
  `shaclEntail(shapesTtl, dataNt)` materializes the SHACL-AF `sh:rule`
  inferences as N-Triples.
- **`Sink`** — a streaming consumer (`push(quad)` / `finish() → Dataset`);
  `datasetToStream` / `streamToDataset` are the async RDF/JS Stream/Sink
  helpers.

More on the RDF/JS mapping in [RDF/JS in JavaScript](../interop/rdfjs.md).

## Asynchronous queries and federation

The synchronous methods are the offline lane: they install no `SERVICE` or
`LOAD` source, so a non-`SILENT` `SERVICE` or `LOAD` fails by name. Every
evaluating method also has a Promise-returning twin — `queryAsync`,
`selectAsync`, `askAsync`, `constructAsync`, `describeAsync`, `queryRawAsync`,
`queryRawBytesAsync`, `queryRawWithContextAsync`, `queryGovernedAsync`,
`queryEntailmentGovernedAsync`, `updateAsync`, `updateGovernedAsync` and
`explainQueryAsync` on `QueryEngine`, and `Dataset.queryAsync` — plus
`queryGovernedNegotiatedAsync`, which answers a governed query as a document in
the format an HTTP `Accept` header negotiates. A twin runs the same evaluator over a snapshot of the dataset
taken when the call starts, as a job that suspends while the host answers a
`SERVICE` or `LOAD` and gives the event loop back while it evaluates, and it
resolves to exactly what its synchronous twin returns. The host does the I/O
and owns its policy; PurRDF keeps the parsing, the evaluation, the joins, the
`SILENT` semantics and the result encoding.

EXPLAIN is one of these evaluating methods: its charge ledger is measured by
running the query, not predicted from its text. `explainQuery` therefore refuses
a `SERVICE` query for want of a source, while `explainQueryAsync` explains it
over the host's answer. Its measuring run yields and stops on its `signal` like
any other job, and it takes no ceiling, because the run is metered, never
bounded.

SHACL validation evaluates SPARQL as well: `sh:SPARQLTarget` queries,
SHACL-SPARQL constraints, and SHACL-AF node expressions and rules. The SHACL
functions therefore have twins too: `shaclValidateToSarifAsync`,
`shaclValidateChangesToSarifAsync`, `shaclEntailAsync`, and
`shaclProductValidateToSarifAsync` with its `Rebuild`, `Expecting` and
`RebuildExpecting` forms. Each takes its synchronous twin's arguments followed
by the host options and returns exactly what the synchronous twin returns; a
refused product rejects with the same `ShaclProductRefusal`. The `signal` is
polled between focus nodes as well as inside queries, so a validation with no
SPARQL in it still yields and stops. SHACL admits `SERVICE` only in a query that
pre-binds nothing, such as a `sh:SPARQLTarget`; a constraint query that uses
`SERVICE` is refused when the shapes graph loads, on either lane.

The twins run over WebAssembly JavaScript Promise Integration (JSPI), on by
default in Chrome and Edge 137+, Firefox 139+, Safari 27, Node 24.20+ and
Cloudflare Workers (workerd). `hasAsyncQueries()` reports whether the current
engine has it; where it does not, every twin rejects with one error before it
touches wasm, and the synchronous API works as before.

### Answering `SERVICE`

`resolveService(request, ctx)` receives the SPARQL 1.1 Protocol POST to send:
`endpoint`, `queryText`, `contentType` (`application/sparql-query`), `accept`
(`application/sparql-results+json`), `userAgent`, `timeoutMs`, and `headers` —
the catalog profile's headers and credential as `[name, value]` pairs in sending
order. `ctx` carries `signal` (fires on cancellation or the deadline),
`remainingDeadlineMs`, `silent` and `maxIntermediateCells`. The handler answers
with SPARQL Results JSON (bytes or a string), a `Response` (a non-2xx status is a
transport failure), `{ kind: "transport", message }` — which `SERVICE SILENT`
swallows to the join identity — or `{ kind: "denied", message }`, which fails
the query even under `SILENT`. A handler that throws, rejects or returns
anything else has faulted, and a fault fails the job even under `SERVICE
SILENT`, because it is not an answer. `ctx.silent` is for information only: an
empty answer is not the handler's to invent.

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

A `ServiceCatalog` passed as `catalog` authorizes every request before the
handler is called (deny by default, one profile per endpoint, an optional
fallback); a denial fails the query even under `SERVICE SILENT`.
`localServices` answers named endpoints in process from a `Dataset`.
`resolveLoad` answers `LOAD` the same way, with a document and its media type, a
`Response`, a `Dataset`, or a typed failure: `LOAD SILENT` swallows a transport
failure and never a denial.

In a browser, the remote endpoint's CORS policy governs whether `fetch` can
read its answer: an endpoint that does not allow the page's origin surfaces as a
network error, which the handler above reports as a transport failure. Write
`SERVICE SILENT` where an endpoint may be unreachable and its rows are optional.

### Yielding, cancellation and concurrency

- A job gives the event loop one turn every `yieldEveryPolls` governor polls
  (65 536 by default; `0` yields at every poll), through the macrotask primitive
  `asyncYieldPrimitive()` reports. Only evaluation yields: freezing the dataset
  and serializing the result run to completion, and `evidence.async` reports
  what each phase cost.
- `signal: AbortSignal` cancels a job at its next yield or host effect. On the
  governed twins, `deadlineMs` includes the time spent waiting for the handlers,
  and a trip — a deadline or a cancellation included — is an outcome, not a
  rejection.
- A query reads its own snapshot; asynchronous updates on one dataset run one at
  a time, in call order, and an update is applied only if the dataset was not
  mutated while it ran. `configureAsync({ maxConcurrentJobs })` bounds the jobs
  in flight (16 by default).
- Each job evaluates on its own stack region of `stackBytes` bytes (2 MiB by
  default); `evidence.async.stackHighWaterBytes` reports how deep it went, and a
  request too deep for the region fails with a typed error. A job that traps,
  or whose frames run past the region's guard zone, poisons the instance, and so
  does a Rust panic in any call, synchronous or asynchronous: from then on every
  call into the package — synchronous ones and objects created before the trap
  included — throws, and only a fresh JavaScript realm (a new page, Worker
  isolate or process) can load it again.

### Cloudflare Workers

`@blackcatinformatics/purrdf/cloudflare` provides
`createFetchServiceResolver`, `createFetchLoadResolver` and
`handleSparqlRequest`, which answers one SPARQL 1.1 Protocol request with a
`Response`: `200` with the negotiated document, `413` when the body exceeds
`maxRequestBytes` (1 MiB by default — a query or update's text is a program,
not a payload), `422` or `503` when a governor stopped it (never a `200` with
a partial body), `application/problem+json` errors, `Server-Timing` from the
job's evidence, and CORS when asked for. A `500`'s `detail` is the engine's
own words for the query's own failures (a parse, an evaluation, a tripped
governor); a bug this endpoint cannot attribute to the query — a
`resolveService`/`resolveLoad` that throws, or any other exception the
adapter did not otherwise classify — never reaches the response as its own
message or stack: the client gets a fixed generic `detail` and a
`correlationId`, and the real error goes to `onInternalError` (one
`console.error` line by default) alone. A `Content-Length` over the bound is
refused before anything is read; a missing or understated one is still caught
by counting bytes as the body streams in, so a lying header never buys a
larger body than an honest one would. Both resolvers fetch with
`redirect: "manual"`: a `SERVICE` request never follows a redirect (a 3xx is a
typed transport failure, so a catalogued endpoint's headers and credential can
never reach a different origin), and a `LOAD` follows one only by
re-authorizing the redirected IRI against the catalog before every hop, up to
`maxRedirects` (5 by default). A complete Worker:

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

Workers limits how many subrequests one invocation may make, and
`maxRemoteRequests` is the exact control for it: every `SERVICE` request and
every `LOAD` is charged before it reaches the handler, so a request never makes
more subrequests than the ceiling. The Cache API does nothing on `workers.dev`
hostnames, so caching is effectively off there; on a custom domain it works. A
runtime started without a cache configured (a locally run workerd, for
example) rejects `cache.match` and `cache.put` with "No Cache was configured",
which the resolver treats as an optimisation failure, never the query's
answer: a rejected `match` is a miss and a rejected `put` never discards an
answer the remote already returned.
`onCacheError(error, { operation, endpoint })` reports every such failure (one
`console.warn` line by default), so it is visible, never silent, while the
query still answers. On Workers `Date.now()` does not advance during
CPU-bound execution, so a synchronous `deadlineMs` cannot trip during
CPU-bound work there; the asynchronous lane observes the deadline at every
yield and every effect.

The package
[README](https://github.com/Blackcat-Informatics/purrdf/tree/main/crates/rdf-wasm/js#asynchronous-queries-federation-and-the-cloudflare-adapter)
is the complete reference for these contracts.

## Scope and current limitations

- **In-memory only.** SPARQL queries run over the in-memory dataset. The
  synchronous methods install no `SERVICE` or `LOAD` source, so there a remote
  `SERVICE` or `LOAD` fails explicitly unless it is written `SILENT`; the
  asynchronous twins reach remote endpoints only through the handlers the host
  passes them.
- **Triple terms per format.** `serialize` is the writer-native lane: an
  object-position quoted-triple term and the RDF 1.2 statement layer survive
  Turtle, N-Triples, N-Quads and TriG (as `<<( … )>>`), RDF/XML (as
  `rdf:parseType="Triple"`), and JSON-LD / YAML-LD (as `@triple`). TriX and
  HexTuples have no triple-term surface, so serializing a dataset that carries
  one to either of them **throws** rather than dropping the layer silently.
  Single-graph targets (Turtle, N-Triples, RDF/XML) emit the default graph
  alone.

## Building from source

The Rust cdylib lives in
[`crates/rdf-wasm`](https://github.com/Blackcat-Informatics/purrdf/tree/main/crates/rdf-wasm);
the published ESM package is generated from it:

```sh
make wasm-pkg        # release wasm + wasm-bindgen ESM bindings → js/pkg/
make wasm-pkg-test   # the above + TypeScript, Node, and packed-tarball gates
```

This requires the `wasm32-unknown-unknown` Rust target and a
`wasm-bindgen-cli` pinned to the crate's `wasm-bindgen` version.
