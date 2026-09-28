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
  `hasAsyncQueries()` and `configureAsync(...)` describe and
  configure the scheduler, and `SparqlProtocolRequest` reads a SPARQL 1.1 Protocol
  request. See
  [Asynchronous queries, federation and the Cloudflare adapter](#asynchronous-queries-federation-and-the-cloudflare-adapter).
- `QueryEngine.explainQuery(dataset, sparql, options?)` / `governorDimensions()` — the
  metered charge ledger a budget is sized from (join orders, plan estimates, per-node
  cost) and the engine's dimension vocabulary, which keys every `evidence` map.
  EXPLAIN evaluates the query to measure it, so `explainQueryAsync` is its
  Promise-returning twin: it explains a `SERVICE` query over the host's answer, yields
  while it measures, and stops on its `signal`.
- `shaclValidateToSarif(shapesTtl, dataNt, shapesBase?)` /
  `shaclEntail(shapesTtl, dataNt, shapesBase?)` — SHACL validation to a SARIF
  2.1.0 report and SHACL-AF `sh:rule` entailment, returned as a
  `ShaclEntailment` (`ntriples`, `diagnostics`; call `free()`). A shape whose
  `sh:in` or `sh:xone` list is empty is a mandatory diagnostic every run reports,
  beside the verdict and never among the results: the SARIF log carries each as a
  `level: "note"` notification in `invocations[0].toolExecutionNotifications`,
  and `ShaclEntailment.diagnostics`, `ShaclRulesInference.diagnostics` and
  `ShaclNodeExprOutcome.diagnostics` carry `ShaclDiagnostic` values — `rule`
  (`in-minListLength`, `xone-minListLength`) and `shape` as separate fields, the one
  encoding every host shares. `shapesBase` is
  the base the shapes document's relative IRI references resolve against; a
  browser or Node host has no retrieval IRI of its own, so omit it and a
  relative reference throws rather than being mis-parsed (`dataNt` needs no
  counterpart — N-Triples admits no relative IRI by grammar). `Dataset.parse`
  takes the same optional third argument. `shaclValidateToSarif` also takes
  `conformanceDisallows?`, `importIris?`, `importDocuments?` and a trailing
  `shapesGraph?`: the IRI SHACL-SPARQL sees the shapes graph under, as `purrdf
  validate --shapes-graph` names it. `$shapesGraph` is pre-bound to it and `GRAPH
  $shapesGraph { … }` reads the shapes graph (SHACL 1.0's pre-binding, which SHACL
  1.2 removed); omitted, `$shapesGraph` is an ordinary variable. A relative IRI
  resolves against `shapesBase`, and one with no base throws.
  `shaclValidateChangesToSarif`, `shaclPackProduct` (which records it in the
  product), `shaclLintShapes`, `shaclApplyRules` and `shaclEntail` take the same
  `shapesGraph?`, right after the import table; on the two rules entry points a
  `sh:SPARQLRule`'s `$shapesGraph` is pre-bound to it, and `shaclApplyRules` throws
  when it is named beside `srl`, which has no shapes graph. After `shapesGraph?`, both
  rules entry points take `maxTermGeneratingRounds?`, `maxGeneratedTerms?`,
  `maxStoredFacts?` and `maxJoinSteps?` (each a `bigint`), contiguous and in that
  order: the four rule-evaluation limits, with the same defaults, bounding an
  entailment run the way they bound a rules run. A run past one throws naming the limit, the numbers and the argument
  that raises it (`shaclEntail's maxStoredFacts`, …).
  `shaclValidateToSarif` takes one more, `subClassOfInShapesGraph?`: SHACL 1.2
  Core §6.3's parameter of that name. `true` reads the shapes graph's
  `rdfs:subClassOf` triples, in addition to the data graph's, wherever SHACL type
  decides class membership (`sh:targetClass`, implicit class targets, `sh:class`,
  `sh:rootClass`, `shnex:instancesOf`); omitted or `false`, the specification's
  default, the data graph alone.
- `shaclValidateChangesToSarif(shapesTtl, dataNt, addedNt?, removedNt?, shapesBase?)`
  — validates a CHANGE to `dataNt` rather than the whole graph: hand it the rows
  joining and the rows leaving, and the engine re-validates only the focus nodes
  that change can move. Returns a `ShaclChangeValidation`; read `bounded` before
  the log, because it decides what the log MEANS. `true` and an empty log means
  *this change introduced no violation*; `false` means the shapes graph reads
  through SPARQL query text, no bounded footprint exists for it, the call fell
  back to a FULL validation, and an empty log means *the graph conforms*. Call
  `free()` when done.
- SHACL evaluates SPARQL — `sh:SPARQLTarget` queries, SHACL-SPARQL constraints,
  SHACL-AF node expressions and rules — so every SHACL entry that can evaluate SPARQL
  has a Promise-returning twin: `shaclValidateToSarifAsync`,
  `shaclValidateChangesToSarifAsync`, `shaclEntailAsync`, `shaclApplyRulesAsync`,
  `shaclEvalNodeExprAsync` and the four `shaclProductValidateToSarif…Async`. Each takes
  exactly its synchronous twin's arguments and then the host options, yields while it
  validates and stops on its `signal`. SHACL-SPARQL admits no `SERVICE` in any query, so
  a shapes graph with one is refused while it loads, on either lane. See
  [Asynchronous queries, federation and the Cloudflare adapter](#asynchronous-queries-federation-and-the-cloudflare-adapter).
- `shaclApplyRules(dataNt, shapesTtl?, srl?, shapesBase?, srlBase?, explain?,
  importIris?, importDocuments?, shapesGraph?, maxTermGeneratingRounds?,
  maxGeneratedTerms?, maxStoredFacts?, maxJoinSteps?)` —
  runs exactly one rule source, the SHACL 1.2 rules of `shapesTtl` or the SPARQL 1.2
  RL rule set `srl`, and returns a `ShaclRulesInference`: `inferred` is the
  inference graph (the inferred triples only) as N-Triples, `proof` is the proof
  of every inferred triple when `explain` is set, and `diagnostics` the shapes
  graph's mandatory diagnostics as `ShaclDiagnostic` values (`rule`, `shape`). Two limits stop a rule set that
  keeps inferring new terms. `maxTermGeneratingRounds` (a `bigint`) bounds the
  evaluation rounds that infer a term the graph did not hold (default 16384), and
  `maxGeneratedTerms` (a `bigint`) the terms inferred beyond the input's (default
  `max(65536, 4 × N)` for `N` distinct input terms). A run past either throws
  naming the limit, the numbers, the rules that inferred a new term last, and the
  argument that raises it; a rule set that needs more states it here.
  `maxStoredFacts` (a `bigint`) bounds the facts the evaluation store holds — the
  data graph, a rule set's data and every inferred triple — and `maxJoinSteps` (a
  `bigint`) the candidate solutions the rule bodies enumerate. Omitted, each is the
  WebAssembly default: 131072 facts and 1048576 join steps, sized for one linear
  memory (a native build's defaults are 4194304 and 1048576). A rule copying a
  predicate over 70,000 triples holds 140,000 facts and is refused here naming
  `maxStoredFacts`; stating `140000n` admits it. A run past either throws naming
  the limit, the numbers and the argument.
  `importIris` / `importDocuments` are the rule source's import table: the shapes
  graph's `owl:imports` table (Turtle documents) beside `shapesTtl`, the rule set's
  `IMPORTS` table (SPARQL 1.2 RL texts) beside `srl`, followed transitively. An
  imported document's rules run. An import no entry supplies, and an entry the
  import closure never names, throw. Call `free()` when done.
- `shaclCheckRules(srl, srlBase?, importIris?, importDocuments?, level?)` — checks
  the SPARQL 1.2 RL rule set `srl` WITHOUT evaluating it: the grammar, the `IMPORTS`
  closure resolved from `importIris` / `importDocuments` exactly as `shaclApplyRules`
  resolves it, well-formedness and stratification, with no data graph read and no
  rule run. `level` is `"syntax"`, `"well-formed"` or `"stratified"` (the default,
  every static check `shaclApplyRules` applies before it runs), each including the
  ones before it. Returns a `ShaclRulesCheck` — `level`, `rules`, `dataTriples`,
  `imported`, `versions`, `strata` (`undefined` below `"stratified"`) and the
  one-line `summary` every host reports; a refused rule set throws naming the
  stage. Call `free()` when done.
- `shaclEvalNodeExpr(shapesTtl, dataNt, expr, focus, scope?, shapesBase?,
  importIris?, importDocuments?, exprAt?, exprVia?, exprTurtle?)` — evaluates
  one node expression of the shapes graph against a focus node, with `scope` as
  `"NAME=TERM"` strings, and returns a `ShaclNodeExprOutcome`: `outputs`, the
  output nodes as N-Triples terms in sequence order, and `diagnostics`, the shapes
  graph's mandatory diagnostics as `ShaclDiagnostic` values (`rule`, `shape`). The expression is named one way: `expr` is an IRI or
  `"_:label"`; or `expr` is `undefined` and `exprAt` plus `exprVia` walk from a
  named node to an anonymous expression, each step reaching exactly one value;
  or `exprTurtle` gives the expression inline as Turtle, whose one root blank
  node is the expression.
- `shaclLintShapes(shapesTtl, shapesBase?, importIris?, importDocuments?, shapesGraph?)` — certifies a shapes graph: the
  loader's verdict, every result of validating it against the W3C
  `shacl-shacl.ttl`, which implementation every function call binds to, and
  every validator declared for a built-in component (superseded by the native
  implementation, never run). Returns a `ShaclLintReport` with `clean`, `findings`, `loadError` and the
  deterministic `report` text. Call `free()` when done.
- `entailMaterialize(document, regime, program, importIris, importDocuments, premiseIris, maxStoredFacts?, maxJoinSteps?)` —
  SPARQL entailment-**regime**
  materialization over all SEVEN regimes (`"simple"` / `"rdf"` / `"rdfs"` /
  `"owl-rl"` / `"d"` / `"owl-direct"` / `"rif"` — none is refused), returning
  `{ nquads, report }`: the canonical N-Quads closure and a byte-stable reasoning
  report. Unlike `shaclEntail` it takes no shapes graph — it closes the document
  under the regime's own specification rule table. The report is never optional:
  it names which rules fired, which specification rules did **not**, which
  constructs were left at a boundary, the evaluation budget and the calculus's
  contract hash, so "OWL-RL entailment" can never be claimed without saying how
  much of OWL-RL actually ran. `maxStoredFacts` and `maxJoinSteps` (`bigint`s) are
  the evaluation limits of the `rdf`, `rdfs`, `owl-rl` and `d` lanes — omitted, the
  WebAssembly defaults of 131072 facts and 1048576 join steps. A run past either
  throws naming the limit and the argument; the report's contract hash names the
  calculus under the limits in force, so it differs from a native build's report
  unless the native defaults (4194304 and 1048576) are stated. `importIris` /
  `importDocuments` / `premiseIris` are the document's `owl:imports` table (`[]`,
  `[]`, `[]` imports nothing): the closure is taken over the document merged with
  every imported N-Quads document, and an import the table does not resolve throws
  by name rather than closing a smaller ontology. `entailConsistency(document,
  importIris, importDocuments, premiseIris, stepCap, workCap)` and
  `queryEntailmentGoverned`'s `importIris`/`importDocuments`/`premiseIris` options
  take the same table.
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
source, so a `SERVICE` or `LOAD` fails by name. `SERVICE SILENT` is the join identity
there, since an invocation that cannot succeed is what `SILENT` answers that way
(SPARQL 1.1 Federated Query §3.2); a governed method's evidence lists it under
`silenced`. Every evaluating method
also has a Promise-returning twin that takes the host's handlers for those two
clauses:

- on `QueryEngine`: `queryAsync`, `selectAsync`, `askAsync`, `constructAsync`,
  `describeAsync`, `queryRawAsync`, `queryRawBytesAsync`, `queryRawWithContextAsync`,
  `queryGovernedAsync`, `queryEntailmentGovernedAsync`, `updateAsync`,
  `updateGovernedAsync` and `explainQueryAsync`, plus `queryGovernedNegotiatedAsync` (a
  governed query answered as a document in the format negotiated from an HTTP `Accept`
  header, which has no synchronous twin);
- on `Dataset`: `queryAsync`;
- the SHACL functions: `shaclValidateToSarifAsync`, `shaclValidateChangesToSarifAsync`,
  `shaclEntailAsync`, `shaclApplyRulesAsync`, `shaclEvalNodeExprAsync`,
  `shaclProductValidateToSarifAsync`,
  `shaclProductValidateToSarifRebuildAsync`, `shaclProductValidateToSarifExpectingAsync`
  and `shaclProductValidateToSarifRebuildExpectingAsync`.

Each twin runs the same evaluator as its synchronous twin, over a snapshot of the
dataset taken when the call starts, and resolves to exactly the shape the synchronous
twin returns. A SHACL twin takes its synchronous twin's arguments, then the host options
(and no ceiling, as no synchronous SHACL entry takes one; a deadline is a `signal`).
The `signal` is polled between focus nodes as well as inside queries, so a validation
with no SPARQL in it still yields and stops. SHACL-SPARQL admits no `SERVICE` in any
query, a `sh:SPARQLTarget`'s included: a shapes graph with one is refused while it
loads, on either lane. A refused product rejects with the same `ShaclProductRefusal` the
synchronous twin throws, and an `owl:imports` closure not in hand with the same
`ShaclImportError`. It runs as a *job*: the job suspends while the host answers a `SERVICE`
or `LOAD`, and it gives the event loop back at regular intervals while it evaluates.
The host does the I/O and owns its policy. PurRDF keeps the parsing, the evaluation,
the joins, the `SILENT` semantics and the result encoding.

### Hosts

The twins run over WebAssembly JavaScript Promise Integration (JSPI), which is on by
default in Chrome and Edge 137+, Firefox 139+, Safari 27, Node 24.20+ and Cloudflare
Workers (workerd). `hasAsyncQueries()` reports whether the current engine has it. Where
it does not, every twin rejects with one error naming what is missing before it touches
wasm, and the synchronous API works as before.

### Errors carry their code

Every error the package throws for a failure it classifies carries the stable code it
is reported under as `error.code`, on the synchronous and the asynchronous lane alike:
an engine diagnostic's own (`native-sparql-query-parse`, `native-sparql-load-denied`,
…), a job's stop or fault (`native-sparql-cancelled`, `native-sparql-deadline`,
`native-sparql-not-acceptable`, `native-sparql-update-in-flight`,
`native-sparql-host-fault`), or one of the package's own refusals
(`purrdf-wasm-options`, `purrdf-wasm-usage`, …). Switch on `code`, never on the
message. A cancellation with a `signal.reason` rejects with that reason itself.

### Answering `SERVICE`: `resolveService`

`resolveService(request, ctx)` is called once for each distinct `SERVICE` request a job
issues, and may return its answer or a Promise of it. `request` is the SPARQL 1.1
Protocol POST to send:

- `endpoint`: the service IRI;
- `queryText`: the forwarded query;
- `contentType`: `application/sparql-query`;
- `accept`: `application/sparql-results+json`;
- `userAgent`;
- `timeoutMs`: the catalog profile's timeout, or the default (30 s);
- `headers`: the catalog profile's headers and then its credential header, as
  `[name, value]` pairs in sending order. Append each pair and never merge repeated
  names. Without a catalog the list is empty;
- `cacheable`: whether a shared cache may answer the request or keep its answer —
  `false` when it carries a credential, whose answer belongs to the credential's holder.

`ctx` carries four fields:

- `signal`: an `AbortSignal` that fires once no job waits on the call any more: every
  job waiting on it was cancelled, or reached its request's timeout or its deadline,
  whichever comes first (the job decides that instant, not the handler). For a call that
  several jobs share (see [Concurrency](#concurrency)), it fires only once every one of
  them has stopped waiting.
- `remainingDeadlineMs`: the time left before the deadline, when the job has one.
- `silent`: whether the clause was written `SERVICE SILENT`.
- `maxIntermediateCells`: the query's cell ceiling, when one is set.

The answer is one of:

- SPARQL Results JSON as a `Uint8Array`, an `ArrayBuffer` or a string;
- a `Response`, whose 2xx body is read as SPARQL Results JSON. Any other status is a
  transport failure;
- `{ kind: "transport", message }` when the endpoint could not be reached or read;
- `{ kind: "denied", message }` when the host's policy refuses the request;
- `{ kind: "fault", message }` when the handler itself could not answer.

Each failure fails the query: `native-sparql-service-failed`,
`native-sparql-service-host-denied` and `native-sparql-host-fault`. Under
`SERVICE SILENT` each contributes the join identity instead, so the surrounding
pattern's own solutions come back unextended, and the job's `evidence.async.silenced`
records the endpoint and the failure's `kind`.

A handler that throws, rejects, or returns anything else has failed the invocation it
was answering, and is delivered exactly as `{ kind: "fault" }`: the invocation's own
failure, never the job's. The error a query fails with carries the handler's words,
for the host that wrote the handler. `ctx.silent` is for information only: an empty answer is not the handler's to
invent, and the failure it reports decides what `SILENT` does with it.

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

A `ServiceCatalog` passed as `catalog` authorizes every request — every `SERVICE`, and
every `LOAD` source and redirect — before the handler is called. It denies by default,
holds one profile per endpoint and an optional fallback, and a profile grants the
capabilities `query`, `network` and `credentials` and may add headers, a credential
header, a `User-Agent` and a `timeoutMs` (at most 2 147 483 647, the longest delay a
JavaScript timer honours): the job abandons a request still unanswered at that timeout,
as a transport failure, or at its deadline when that falls first. A denied request fails
the query, and under `SERVICE SILENT` is the join identity recorded as `"denied"`. The
catalog is copied, so it stays usable for later operations.
`localServices: { [endpoint]: dataset }` answers the
named endpoints in process from a snapshot of a `Dataset`, without calling the handler.

In a browser, the remote endpoint's CORS policy governs whether `fetch` can read its
answer: a cross-origin endpoint that does not allow the page's origin surfaces as a
network error, which the handler above reports as a transport failure. Write
`SERVICE SILENT` where an endpoint may be unreachable and its rows are optional.

### Variable endpoints: `SERVICE ?e`

A `SERVICE ?e { … }` answers every row with the `?e` of the endpoint that produced it.
`?e` must be bound in every solution that reaches the clause, and where it is bound
decides how many requests the clause makes. It is bound by one of:

- a pattern earlier in the same group — a triple pattern, `VALUES`, `BIND`, or an
  explicit `LATERAL { SERVICE ?e { … } }`. The clause is evaluated once per solution,
  with that solution's bindings substituted into the forwarded query: one request per
  solution, so two solutions that name the same IRI send it two requests, which differ
  in the bindings they carry. (A job asks `resolveService` only once for a request it
  repeats exactly; see [Concurrency](#concurrency).)
- the left side of the `OPTIONAL`, `MINUS` or group join whose right side holds the
  clause: `?g ex:endpoint ?e OPTIONAL { SERVICE ?e { … } }`,
  `?g ex:endpoint ?e MINUS { SERVICE ?e { … } }`,
  `{ ?g ex:endpoint ?e } { SERVICE ?e { … } }`. The right side is still evaluated on its
  own, as SPARQL evaluates it; the left side supplies only the list of endpoints to ask,
  each distinct IRI is asked once, and a row from any other endpoint could match no left
  row anyway. A left row whose
  endpoint answers nothing keeps its own bindings under `OPTIONAL` and is not removed
  under `MINUS`;
- the other side of the group join that holds the clause: `{ SERVICE ?e { … } ?g
  ex:endpoint ?e }`, `{ SERVICE ?e { … } VALUES ?e { … } }`. A join is commutative, so
  these answer the rows, and ask the endpoints, of `{ ?g ex:endpoint ?e } { SERVICE ?e
  { … } }`. `OPTIONAL` and `MINUS` are not: `{ SERVICE ?e { … } OPTIONAL { ?g ex:endpoint
  ?e } }` is refused, since the optional side need not bind `?e`.

Under `SERVICE SILENT` an endpoint that fails is the join identity for its own left rows
alone. Under a group join or `OPTIONAL` it contributes one row binding only `?e`, so its
left rows survive unextended and no other endpoint's rows change. Under `MINUS` the left
rows are subtracted one endpoint at a time, and a failing endpoint subtracts nothing from
its own rows while the others' answers still remove theirs. An `?e` bound to a literal or
a blank node names no endpoint: an error before any request is made, and under `SILENT`
the join identity for that value's rows.

A clause for which no solution binds `?e` is refused, `SILENT` or not: no invocation is
made for `SILENT` to absorb, and an empty answer would look complete when nothing was
asked. That covers an `?e` bound nowhere, bound in only some of
the left side's solutions, or bound only outside a further `OPTIONAL` or `MINUS` right
side, an `EXISTS`, a `LIMIT`/`OFFSET`, an aggregate not grouped by `?e`, or a sub-`SELECT`
that does not project `?e` between the binding and the clause. The error names the
rewrite: bind `?e` before the clause, as in `?s ex:endpoint ?e . SERVICE ?e { … }` or
`?s ex:endpoint ?e LATERAL { SERVICE ?e { … } }`.

SPARQL 1.1 Federated Query leaves `SERVICE` with a variable informative (§4). This is the
reading that section describes — one invocation per binding, the results combined by
union — with the endpoints to try taken from the evaluation order, which it allows.

### Answering `LOAD`: `resolveLoad`

`resolveLoad(request, { signal })` answers one hop of a `LOAD`. `request` is the `GET` to
send: `{ kind: "load", iri, accept, userAgent, headers, timeoutMs }`, where `accept`
names every RDF syntax a `LOAD` parses and `userAgent`, `headers` and `timeoutMs` are
the source's catalog profile's (no headers and the default timeout without a catalog).
With a catalog, the job authorizes the IRI before the handler is called: a source the
catalog does not grant `network` fails with `native-sparql-load-denied`, and nothing is
fetched. `signal` fires at the job's cancellation, the request's timeout or the job's
deadline. The answer is any of:

- `{ bytes, mediaType, base? }` or `{ text, mediaType, base? }`. `mediaType` is a media
  type or any format name `Dataset.parse` accepts, and `base` defaults to the IRI;
- a `Response`, whose `Content-Type` names the media type;
- a `Dataset`;
- `{ kind: "redirect", location }`: the job resolves `location` against the IRI,
  authorizes it against the catalog as a source of its own, and asks for it as a fresh
  hop with that source's own profile — up to five hops; a sixth redirect fails the
  `LOAD` as a transport failure;
- `{ kind: "transport" }` when the document could not be fetched or read;
- `{ kind: "denied" }` when the host's policy refuses the request
  (`native-sparql-load-host-denied`, apart from the catalog's own denial);
- `{ kind: "fault" }` when the handler itself could not answer
  (`native-sparql-load-fault`).

Each failure fails the request. `LOAD SILENT` succeeds over any of them — and over a
missing `resolveLoad` — with nothing loaded (SPARQL 1.1 Update §3.1.4), and the job's
`evidence.async.silenced` records the source's `iri` and the failure's `kind`.

A handler that throws or rejects, and a bare string or bytes (which carry no media
type), are the handler's fault, delivered as `{ kind: "fault" }`. A document that does
not parse — or names a media type no parser reads — is the `LOAD`'s decode failure
(`native-sparql-load-decode`).

### Yielding, cancellation and deadlines

A job counts the evaluator's governor polls and gives the event loop one turn every
`yieldEveryPolls` polls: 65 536 by default, and `0` yields at every poll. The count, not
the clock, decides when to yield. Every yield is one `setTimeout(…, 0)` task, on every
host — Node, a browser, a Cloudflare Worker alike. A timer task queues behind the tasks
already waiting, so other requests, timers and the fetch responses other jobs await all
run between a job's turns. `globalThis.setTimeout` is bound once, when the module loads;
a host without it has no asynchronous lane, and `hasAsyncQueries()` says so. Only
evaluation yields (an entailment closure included). Freezing the dataset before the job and serializing
the result after are linear passes that run to completion. `evidence.async` reports
what each phase cost (`freezeMs`, `evaluateMs`, `serializeMs`).

`signal: AbortSignal` cancels a job. The job observes it at the next yield or host
effect, and the twin rejects with `signal.reason` (an `AbortError` when the signal has no
reason). A governed twin reports the cancellation as a tripped governor instead. Passing
`cancel` (a `CancellationToken`) to a twin is a `TypeError` that names `signal`. On the
governed twins, `deadlineMs` includes the time spent waiting for the handlers as well as
evaluation. The job checks it at every yield and every host effect, and every effect
names the instant it is abandoned at if still unanswered — its request's timeout or the
deadline, whichever falls first — so an effect still pending at the deadline trips it.

On Cloudflare Workers, `Date.now()` does not advance during CPU-bound execution; it moves
only across I/O. A synchronous `deadlineMs` therefore cannot trip during CPU-bound work
there. The asynchronous lane observes the deadline at every yield and every effect, which
is where the clock moves.

### Concurrency

Jobs interleave at every yield and effect, and synchronous calls may run between them. A
query reads the snapshot taken when it started, so a later mutation never shows up in a
running job. One asynchronous update of a dataset may be in flight at a time: beginning
another while it runs rejects at once with the code `native-sparql-update-in-flight`, and
applies and asks nothing — await the first, then send the next. Each update reads a
snapshot and is applied only if the dataset was not mutated while it ran; otherwise it
rejects with the same code and applies nothing. `dataset.id` identifies a dataset within the
wasm instance, and `dataset.generation` counts the mutations it has seen.
`configureAsync({ maxConcurrentJobs })` bounds how many jobs may be in flight (16 by
default); a twin started beyond the bound rejects.

A job asks `resolveService` once for each distinct `SERVICE` request. When it repeats a
request with the same `ctx.silent` and `ctx.maxIntermediateCells`, it reuses the first
answer when that answer was rows; a failure or a fault is never reused, so a repeat asks
again. Another job, even one issuing the same request later, asks again.

Concurrent jobs share one call only when the handler would see an equivalent context.
A job joins a call already in flight through the same `resolveService` when all of these
hold:

- the request is identical: endpoint, query text, `Accept`, `Content-Type`,
  `User-Agent`, timeout and headers;
- `ctx.silent` is the same;
- `ctx.maxIntermediateCells` is the same, or unset for both;
- the job's deadline falls no later than the instant the call was told about. That
  instant is when the call started plus the `remainingDeadlineMs` it received. A job
  without a deadline joins only a call that was given none.

Any other job gets a call of its own, with its own `ctx`. A handler that bounds its work
by `ctx.remainingDeadlineMs` therefore never gives up on a joined job sooner than it
would on that job's own call. Every waiting job receives the shared answer as it stands:
rows, a `transport` or `denied` failure, or a fault. The call's `ctx.signal` belongs to
the shared call. When one waiting job is cancelled or passes its deadline, only that job
stops waiting. The signal fires once every waiting job has stopped. `LOAD` requests are
never shared or reused.

### Stack regions and faults

Each job evaluates on its own stack region, exactly as large as the module's own shadow
stack — the stack the synchronous lane runs on (1 MiB in the shipped module). No option
sizes it. `evidence.async.stackHighWaterBytes` reports the deepest the job went below the
region's top. The two stacks are the same size, so the two lanes evaluate requests to
nearly the same depth — each spends a little stack on its own frames, so the deepest
request each evaluates can differ by a few levels — and a request too deep for either
fails with the evaluator's own typed stack refusal,
`native-sparql-evaluation-stack-exhausted`, which names no lane-specific remedy. The
construct its message names is the one whose frame found the stack low. The evaluator checks the stack left above the
region's base at every recursive step and refuses while 64 KiB remain. Parsing keeps a
request's nesting in linear memory and spends neither stack on it.

A region is the size of the shadow stack in linear memory only. Every wasm call also takes frames
on the JavaScript engine's own call stack, which no wasm code can read, and V8 (Node.js,
Chromium, Cloudflare Workers) gives a job's suspendable stack the same size as the
synchronous lane's: the smaller of its `--stack-size` and
`--wasm-stack-switching-stack-size` flags, 984 KiB by default, set for the whole process.
PurRDF keeps a fixed 640 KiB budget of it for evaluating a request, so on both lanes a
request's graph patterns nest at most 284 levels deep, and a plan taller than the
evaluator's host-stack bounds is refused before it runs. Past either is
`native-sparql-host-stack-exhausted`, which names the budget; no lane raises it, and the
remedy is a request nested less deeply. Either way the job fails and the instance stays
usable. A canary word at each region's base is read before every suspension and when
the run returns: a job whose frames ran past the base fails with a fault naming its
region, and never suspends or commits. If a job traps, the instance's state can no longer
be trusted: the trap leaves anything the job was mutating half-changed. The instance is
then *poisoned*, and it cannot be used again. Every in-flight job rejects with the poison error, and so does every later
asynchronous twin, `hasAsyncQueries()`, `configureAsync()` and `ready()` — because there
is one instance per JavaScript realm. Every later synchronous call into the instance — a
constructor, a static, a free function, a method or getter of an object created before
the trap — traps at the instance's own entry with a `WebAssembly.RuntimeError`: the
package build links a poison gate into the wasm module itself, so no JavaScript stands
between a caller and the refusal. `free()` and `[Symbol.dispose]()` are the exception:
they return without entering the instance and release nothing, because the instance's
memory is abandoned whole and a finalizer has no caller to report an error to. Only a
fresh realm (a new page, Worker isolate or process) can load the package again.

A trap poisons the instance the same way when it comes out of a synchronous call. The
trapping call throws the trap itself, the engine's `WebAssembly.RuntimeError`; the entry
it unwound never returned through the gate, so the next entry finds the instance
poisoned and traps, and so does every entry after it. A typed PurRDF error, a parse
error for instance, is thrown after the call has returned and never poisons anything. A
JavaScript exception thrown through wasm frames — a sink callback that throws, or V8
reporting its own native stack exhausted as a `RangeError` inside a call — unwinds the
instance the same way and poisons it too; the same exception thrown by the caller's own
code, outside any call into the instance, does not. Out of PurRDF's own code, the traps
left are memory exhaustion, which aborts without running the panic hook, and a Rust
panic; PurRDF's parser and evaluator refuse nesting before either stack runs out (the
host-stack budget above keeps V8's).

A Rust panic poisons the instance too, in an asynchronous job or in a synchronous call
alike. A panic aborts on wasm32 and leaves whatever it interrupted half-changed, so the
panic hook PurRDF installs when the instance starts poisons the instance, naming the
panic's location and message, before the panic's trap unwinds. The call that panicked
throws that trap; every asynchronous entry afterwards rejects with the poison error,
which names the panic rather than the trap that followed it. PurRDF is written not to
panic on any input, so a poison that names a panic is a PurRDF defect to report.

### The Cloudflare adapter

`@blackcatinformatics/purrdf/cloudflare` builds a SPARQL 1.1 Protocol endpoint from these
pieces:

- `createFetchServiceResolver({ fetch?, bindings?, cache?, cacheTtlSeconds? })`
  returns a `resolveService` that POSTs each request with `fetch`, or through the
  service binding registered for the endpoint's origin, always with `redirect: "manual"`
  and bounded by `ctx.signal` (the request's catalog timeout or the query's deadline). A
  network error, an abandoned request or a non-2xx status is reported as
  `{ kind: "transport" }`, never thrown — and so is a 3xx (or a browser's opaque-redirect
  response): it is never followed, so the profile's headers and credential (an
  `X-Api-Key`, a `Cookie`, …) can never reach an origin the catalog did not authorize.
  With `cache` and `cacheTtlSeconds` it reuses the answer of every request the job marks
  `cacheable` through the Cache API; a request that carries a credential is never read
  from or written to the shared cache, and goes to the endpoint instead. A `cache.match`
  or `cache.put` that fails (e.g. "No Cache was configured" from a workerd runtime
  started without a cache) is that request's transport failure: under `SERVICE SILENT`
  the join identity, recorded, and without it the query's failure. Pass no `cache` where
  none is configured.
- `createFetchLoadResolver({ fetch?, bindings? })` returns a `resolveLoad` that GETs
  each hop the job asks for — with the `Accept`, `User-Agent`, headers and credential of
  that hop's own catalog profile, which the job authorized first — also with
  `redirect: "manual"`, and answers a redirect `{ kind: "redirect", location }` for the
  job to resolve, re-authorize and ask for as a fresh hop. A redirect whose `Location`
  is withheld (an opaque one) or missing is a `{ kind: "transport" }` failure.
- `handleSparqlRequest(request, options)` answers one protocol request (`GET ?query=`,
  or a `POST` of `application/sparql-query`, `application/sparql-update` or a form) with
  a `Response`, by the status table below. A partial answer is never sent with a `200`.
  Every error body is `application/problem+json` (RFC 9457) with a stable `code`, and
  every evaluated response carries `Server-Timing` from the job's evidence. The `cors`
  option answers preflights and adds `Access-Control-Allow-Origin`; without it no CORS
  header is sent. `governors.deadlineMs` is required. `maxRequestBytes` (1 MiB by
  default: a SPARQL query or update's text is a program, not a payload, and 1 MiB
  comfortably covers even a large one) bounds the request body — a `Content-Length`
  above it is refused before anything is read, and a missing or understated one is still
  caught by counting bytes as the body streams in, so a lying header never buys a larger
  body than an honest one would. An error's `detail` is the engine's own words only when
  the failure is the query's — a refusal to evaluate it as written, its evaluation, a
  tripped governor: a SPARQL client is owed the reason its request failed. A `403` or
  `502` gets a fixed `detail` for its code instead, because the engine's message would
  echo the catalog's policy or a resolver's or remote's own words. A fault of this
  endpoint's own — no resolver reaching a named endpoint or source, a `resolveLoad`
  answer that is not one, `resolveService`/`resolveLoad` throwing or rejecting, a
  rejection no engine code classifies, any other exception this adapter did not
  otherwise classify — never puts its own message or stack in the response: it gets a
  fixed `detail` and a fresh `correlationId`, while the real error goes to exactly one
  place, `onInternalError(error, { correlationId, request })` (one
  `console.error(error, correlationId)` line by default), so an operator can always join
  what the client saw to what actually broke. A `resolveService`/`resolveLoad` that
  throws under `SERVICE SILENT` or `LOAD SILENT` is reported there too, under its
  correlation id, while the response is the clause's own answer — the join identity, or
  nothing loaded — with the invocation recorded on the job's evidence as a silenced
  `"fault"`.

| Status | `code` | When |
|---|---|---|
| `200` | — | a query answered, with the negotiated document |
| `204` | — | an update applied |
| `400` | the protocol refusal's name | a malformed request, or dataset parameters applied to an operation that does not parse |
| `400` | `native-sparql-query-parse`, `native-sparql-update-parse` | the operation does not parse; `detail` is the parser's message |
| `400` | `native-sparql-unsupported`, `native-sparql-custom-function`, `native-sparql-quoted-triple-term-variable`, `native-sparql-host-stack-exhausted` | the engine refuses to evaluate the request as written (a `SERVICE ?e` no solution names an endpoint for, an unregistered function, nesting past the host-stack budget); `detail` is the engine's message |
| `403` | `native-sparql-service-denied` | the catalog withholds a capability from a `SERVICE` endpoint the query names; no endpoint was contacted |
| `403` | `native-sparql-service-host-denied` | `resolveService` refused the request by its own policy (`{ kind: "denied" }`); no endpoint was contacted |
| `403` | `native-sparql-load-denied` | the catalog does not authorize a `LOAD` source, or a location it redirected to; nothing was fetched |
| `403` | `native-sparql-load-host-denied` | `resolveLoad` refused the source by its own policy (`{ kind: "denied" }`) |
| `405` | the protocol refusal's name | a method the protocol does not bind |
| `406` | `NotAcceptable` | no acceptable format can carry the result |
| `409` | `native-sparql-update-in-flight` | another update of the dataset is in flight, or the dataset was mutated while this one ran; nothing was applied |
| `413` | `ContentTooLarge` | the body exceeds `maxRequestBytes` |
| `415` | the protocol refusal's name | a `Content-Type` the protocol does not define |
| `422` | the governor's label | a deterministic ceiling stopped the request |
| `500` | the engine's diagnostic code (`native-sparql-query-eval`, `native-sparql-evaluation-stack-exhausted`, …) | the query's evaluation failed; `detail` is the engine's message |
| `500` | `native-sparql-service-unconfigured`, `native-sparql-load-no-resolver` | no resolver reaches a named endpoint or source; with a `correlationId` |
| `500` | `InternalError` | a host-supplied resolver threw, rejected or answered with something that is not an answer, an exception no code classifies, or any other unexpected exception; with a `correlationId` |
| `502` | `native-sparql-service-failed` | a `SERVICE` endpoint was contacted and gave no usable answer: a network error, the resolver's timeout, an HTTP error status, a redirect, an undecodable body |
| `502` | `native-sparql-load-failed` | a `LOAD` source could not be fetched: a network error, its timeout, an HTTP error status, more than five redirects |
| `502` | `native-sparql-load-decode` | a `LOAD` source was fetched and could not be parsed |
| `503` | the governor's label, or `cancelled` | the deadline or a cancellation stopped the request (no `Retry-After`: the same request would stop again) |

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

Workers limits how many subrequests one invocation may make. `maxRemoteRequests` is the
exact control for that limit. Every `SERVICE` request and every `LOAD` is charged against
it before it reaches the handler, including one the cache then answers, so a request
never makes more subrequests than the ceiling. Set it to the subrequests you allow one
query. A `SERVICE ?e` bound by a pattern earlier in its group is charged once per
solution that reaches it, not once per endpoint (see
[Variable endpoints](#variable-endpoints-service-e)), so size the ceiling to those
solutions. The Cache API does nothing on `workers.dev` hostnames, so caching is effectively
off there; on a Worker served from a custom domain it works. A runtime started without a
cache configured (a locally run workerd, for example) rejects `cache.match` and
`cache.put` with "No Cache was configured", which fails every request that consults the
cache: configure no `cache` there.

## Scope

In-memory only, by design: no persistent store and no network I/O inside the
wasm module. The synchronous methods install no `SERVICE` or `LOAD` source, so
there a remote `SERVICE` or `LOAD` fails explicitly, even when it is written `SILENT`:
`SILENT` tolerates an endpoint or document that fails, and none was reached.
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
