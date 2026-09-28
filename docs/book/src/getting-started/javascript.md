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
  inferences and returns a `ShaclEntailment`: `ntriples` holds the result as
  N-Triples, and `diagnostics` holds the shapes graph's mandatory diagnostics.
  Call `free()` on it.
- **`Sink`** — a streaming consumer (`push(quad)` / `finish() → Dataset`);
  `datasetToStream` / `streamToDataset` are the async RDF/JS Stream/Sink
  helpers.

More on the RDF/JS mapping in [RDF/JS in JavaScript](../interop/rdfjs.md).

## Asynchronous queries and federation

The synchronous methods are the offline lane: they install no `SERVICE` or
`LOAD` source, so a `SERVICE` or `LOAD` fails by name unless written `SILENT`,
which succeeds with nothing fetched, as SPARQL 1.1 requires for an invocation
that fails. Every
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
`shaclValidateChangesToSarifAsync`, `shaclEntailAsync`, `shaclApplyRulesAsync`,
`shaclEvalNodeExprAsync`, and `shaclProductValidateToSarifAsync` with its
`Rebuild`, `Expecting` and `RebuildExpecting` forms. Each takes exactly its
synchronous twin's arguments followed by the host options and returns exactly
what the synchronous twin returns; a refused product rejects with the same
`ShaclProductRefusal`, and an `owl:imports` closure not in hand with the same
`ShaclImportError`. The `signal` is polled between focus nodes as well as inside
queries, so a validation with no SPARQL in it still yields and stops.
SHACL-SPARQL admits no `SERVICE` in any query, a `sh:SPARQLTarget`'s included;
a query that uses it is refused, on either lane.

The twins run over WebAssembly JavaScript Promise Integration (JSPI), on by
default in Chrome and Edge 137+, Firefox 139+, Safari 27, Node 24.20+ and
Cloudflare Workers (workerd). `hasAsyncQueries()` reports whether the current
engine has it; where it does not, every twin rejects with one error before it
touches wasm, and the synchronous API works as before.

### Answering `SERVICE`

`resolveService(request, ctx)` receives the SPARQL 1.1 Protocol POST to send:
`endpoint`, `queryText`, `contentType` (`application/sparql-query`), `accept`
(`application/sparql-results+json`), `userAgent`, `timeoutMs`, `headers` —
the catalog profile's headers and credential as `[name, value]` pairs in sending
order — and `cacheable` (`false` for a request carrying a credential). `ctx`
carries `signal` (fires on cancellation, the request's timeout or the deadline),
`remainingDeadlineMs`, `silent` and `maxIntermediateCells`. The handler answers
with SPARQL Results JSON (bytes or a string), a `Response` (a non-2xx status is a
transport failure), `{ kind: "transport", message }` or
`{ kind: "denied", message }`. A handler that throws, rejects, returns anything
else or names a failure kind the protocol does not define has faulted, and the
fault is the failure of the one invocation it was answering, `{ kind: "fault" }`.
Any failure fails the query — a fault with `native-sparql-host-fault`, its
message carrying the handler's words; under `SERVICE SILENT` it is the join
identity, and the evidence's `silenced` records the endpoint and the failure's
`kind` (`transport`, `denied`, `fault`, …). `ctx.silent` is for information
only: an empty answer is not the handler's to invent.

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
      // Fires at the request's timeout, the job's deadline or its cancellation.
      signal,
    });
  } catch (error) {
    // A network error is the endpoint's failure; a throw would be reported as this
    // handler's own fault instead.
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

A `ServiceCatalog` passed as `catalog` authorizes every request — every
`SERVICE`, every `LOAD` source and every location a `LOAD` is redirected to —
before the handler is called (deny by default, one profile per endpoint, an
optional fallback), and its profile's `timeoutMs` bounds each request; a denial
fails the query, and under `SERVICE SILENT` is the join identity.
`localServices` answers named endpoints in process from a `Dataset`.
`resolveLoad` answers one hop of a `LOAD` the same way, with a document and its
media type, a `Response`, a `Dataset`, `{ kind: "redirect", location }` (the job
resolves and re-authorizes the location, up to five hops), or a typed failure:
each failure fails the request, and `LOAD SILENT` succeeds over it with nothing
loaded. Every error the package throws carries its stable code as `error.code`.

In a browser, the remote endpoint's CORS policy governs whether `fetch` can
read its answer: an endpoint that does not allow the page's origin surfaces as a
network error, which the handler above reports as a transport failure. Write
`SERVICE SILENT` where an endpoint may be unreachable and its rows are optional.

A variable endpoint, `SERVICE ?e { … }`, answers every row with the `?e` of the
endpoint that produced it. `?e` must be bound in every solution that reaches the
clause, and where it is bound decides how many requests the clause makes. Bound
by a pattern earlier in the same group (a triple pattern, `VALUES`, `BIND`, or
`LATERAL { SERVICE ?e { … } }`), the clause is evaluated once per solution with
that solution's bindings substituted into the forwarded query: one request per
solution, so two solutions that name the same IRI send it two requests, which
differ in the bindings they carry. Bound by the left side of the `OPTIONAL`,
`MINUS` or group join whose right side holds the clause —
`?g ex:endpoint ?e OPTIONAL { SERVICE ?e { … } }`, and likewise with `MINUS` or
`{ ?g ex:endpoint ?e } { SERVICE ?e { … } }` — or, since a join is commutative,
by the other side of the group join holding it, as in
`{ SERVICE ?e { … } ?g ex:endpoint ?e }` (not an `OPTIONAL` or `MINUS` right
side, which need not bind it), the right side is still evaluated on its own, the
left side supplies only the list of endpoints to ask, and each distinct IRI is
asked once. A left row whose endpoint answers nothing keeps its bindings under
`OPTIONAL` and is not removed under `MINUS`. Under `SERVICE SILENT`, an endpoint
that fails is the join identity for its own left rows alone: under a group join
or `OPTIONAL` it contributes one row binding only `?e`, so its left rows survive
unextended and no other endpoint's rows change; under `MINUS` the left rows are
subtracted one endpoint at a time, so it removes none of its own rows while an
answering endpoint still removes its matches. An `?e` bound to a literal or a
blank node names no endpoint: an error, and under `SILENT` the join identity for
its rows. A clause for
which no solution
binds `?e` — bound nowhere, bound in only some left solutions, or bound only
outside a further `OPTIONAL` or `MINUS` right side, an `EXISTS`, a
`LIMIT`/`OFFSET`, an aggregate not grouped by `?e`, or a sub-`SELECT` that does
not project it — is refused, `SILENT` or not: no invocation is made for `SILENT`
to absorb. The error names the rewrite: bind `?e`
before the clause, as in `?s ex:endpoint ?e . SERVICE ?e { … }`.

### Yielding, cancellation and concurrency

- A job gives the event loop one turn every `yieldEveryPolls` governor polls
  (65 536 by default; `0` yields at every poll). Every turn is one
  `setTimeout(…, 0)` task, on every host, so the requests, timers and fetch
  responses already waiting run between a job's turns. Only evaluation yields:
  freezing the dataset and serializing the result run to completion, and
  `evidence.async` reports what each phase cost.
- `signal: AbortSignal` cancels a job at its next yield or host effect. On the
  governed twins, `deadlineMs` includes the time spent waiting for the handlers,
  and a trip — a deadline or a cancellation included — is an outcome, not a
  rejection.
- A query reads its own snapshot. One asynchronous update of a dataset may be in
  flight at a time: another begun meanwhile is refused with
  `native-sparql-update-in-flight`, not queued, and an update is applied only if
  the dataset was not mutated while it ran. `configureAsync({ maxConcurrentJobs })` bounds the jobs
  in flight (16 by default).
- Each job evaluates on its own stack region, exactly as large as the module's
  own shadow stack — the stack the synchronous lane runs on; no option sizes it.
  `evidence.async.stackHighWaterBytes` reports how deep the job went, and a
  request too deep for the region fails with the same typed stack refusal as one
  too deep for the synchronous lane. Each lane spends a little stack on its own
  frames, so the deepest request each evaluates can differ by a few levels.
- A region is the size of the shadow stack only. V8 gives a job's own call stack
  the same size as the synchronous lane's (984 KiB by default, set for the whole
  process), and PurRDF keeps a fixed budget of it for evaluating a request, so on
  both lanes a request's graph patterns nest at most 284 levels deep. Past that
  is `native-sparql-host-stack-exhausted`, refused before evaluation on either
  lane: nest the request less deeply. Parsing keeps a request's nesting in
  linear memory and spends neither stack on it.
- A job whose frames overwrite the canary word at its region's base fails with a
  fault naming the region. A trap, a Rust panic or a JavaScript exception thrown
  through the module's frames poisons the instance, on either lane: from then on
  every call into the package — synchronous ones and objects created before the
  trap included — throws, the asynchronous twins and `ready()` reject, and
  releasing an object (`free()`, `[Symbol.dispose]()`) does nothing. Only a fresh
  JavaScript realm (a new page, Worker isolate or process) can load it again.

### Cloudflare Workers

`@blackcatinformatics/purrdf/cloudflare` provides
`createFetchServiceResolver`, `createFetchLoadResolver` and
`handleSparqlRequest`, which answers one SPARQL 1.1 Protocol request with a
`Response`: `200` with the negotiated document, `413` when the body exceeds
`maxRequestBytes` (1 MiB by default — a query or update's text is a program,
not a payload), `422` or `503` when a governor stopped it (never a `200`
with a partial body), `application/problem+json` errors, `Server-Timing`
from the job's evidence, and CORS when asked for. A request the engine
refuses to evaluate as written — an unsupported construct, an unregistered
function, nesting past the host-stack budget — is a `400` whose `code` is
the engine's diagnostic code, and so is an operation that does not parse. A
`SERVICE` endpoint or `LOAD` source the host refuses to contact — the catalog
withholds a capability, or the resolver's own policy refuses — is a `403`, an
update begun while another update of the dataset is in flight is a `409`, and
an endpoint or source that was contacted and gave no usable answer is a `502`;
each carries the engine's diagnostic code as its
`code` and a fixed `detail`, never the engine's message, which would echo
the catalog's policy or a resolver's or remote's own words. A `500`'s
`detail` is the engine's own words for the query's own failures (a parse, an
evaluation, a tripped governor); a fault this endpoint cannot attribute to
the query — no resolver reaching a named endpoint, a
`resolveService`/`resolveLoad` that throws, or any other exception the
adapter did not otherwise classify — never reaches the response as its own
message or stack: the client gets a fixed generic `detail` and a
`correlationId`, and the real error goes to `onInternalError` (one
`console.error` line by default) alone. A throwing handler under `SERVICE
SILENT` or `LOAD SILENT` is the clause's own answer instead — the response is
that answer (`200`, or `204` for an update), and the bug still goes to
`onInternalError` under its correlation id. A `Content-Length` over the bound is
refused before anything is read; a missing or understated one is still
caught by counting bytes as the body streams in, so a lying header never
buys a larger body than an honest one would. Both resolvers fetch with
`redirect: "manual"`: a `SERVICE` request never follows a redirect (a 3xx is
a typed transport failure, so a catalogued endpoint's headers and credential
can never reach a different origin), and a `LOAD` resolver hands a redirect
back to the job, which re-authorizes the location against the catalog before
every hop, up to five. A complete Worker:

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
  JSON.stringify({ capabilities: ["query", "network"], timeoutMs: 5_000 }),
);

export default {
  fetch(request, env) {
    const resolveService = createFetchServiceResolver({
      bindings: { "https://remote.example.org": env.REMOTE },
      cache: caches.default,
      cacheTtlSeconds: 300,
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
more subrequests than the ceiling. A `SERVICE ?e` bound by a pattern earlier in
its group is charged once per solution that reaches it, not once per endpoint,
so size the ceiling to those solutions. The Cache API does nothing on `workers.dev`
hostnames, so caching is effectively off there; on a custom domain it works. A
runtime started without a cache configured (a locally run workerd, for
example) rejects `cache.match` and `cache.put` with "No Cache was configured",
and a failed cache call is that request's transport failure — the join
identity under `SERVICE SILENT`, the query's failure without it — so configure
no `cache` there. A request carrying a credential never touches the shared
cache. On Workers `Date.now()` does not advance during
CPU-bound execution, so a synchronous `deadlineMs` cannot trip during
CPU-bound work there; the asynchronous lane observes the deadline at every
yield and every effect.

The package
[README](https://github.com/Blackcat-Informatics/purrdf/tree/main/crates/rdf-wasm/js#asynchronous-queries-federation-and-the-cloudflare-adapter)
is the complete reference for these contracts.

## Scope and current limitations

- **In-memory only.** SPARQL queries run over the in-memory dataset. The
  synchronous methods install no `SERVICE` or `LOAD` source, so there a remote
  `SERVICE` or `LOAD` fails explicitly unless written `SILENT`; the
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
