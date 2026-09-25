// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import {
  ready,
  asyncYieldPrimitive,
  configureAsync,
  hasAsyncQueries,
  ServiceCatalog,
  type AsyncEntailmentQueryOutcome,
  type AsyncEvidence,
  type AsyncJobError,
  type AsyncLoadResolver,
  type AsyncQueryOutcome,
  type AsyncServiceResolver,
  type AsyncUpdateOutcome,
  type ServiceProfileJson,
  CancellationToken,
  CompiledJsonLdContext,
  DataFactory,
  Dataset,
  liftProjection,
  type LiftProfile,
  type ProjectionProfile,
  type ProjectionLift,
  type ProjectionLossLedger,
  type ProjectionPackage,
  QueryEngine,
  governorDimensions,
  provenanceFromJson,
  type ProvenanceInfo,
  type DirectionalLanguage,
  type GovernorEvidence,
  type EntailmentQueryOutcome,
  type PartialAnswers,
  type QueryOutcome,
  type TrippedGovernor,
  type UpdateOutcome,
  type Literal,
  type NamedNode,
  type Quad,
  type QueryResult,
  type SelectResult,
  type SerializeLoss,
  type RdfTerm,
  type VisualExport,
  type VisualModel,
  type VisualSvgDocument,
} from "@blackcatinformatics/purrdf";

await ready();

const factory = new DataFactory();
const subject: NamedNode = factory.namedNode("https://example.org/s");
const predicate: NamedNode = factory.namedNode("https://example.org/p");
const datatype: NamedNode = factory.namedNode("http://www.w3.org/2001/XMLSchema#integer");

const typed: Literal = factory.literal("7", datatype);
const language: Literal = factory.literal("hello", "en");
const direction: DirectionalLanguage = { language: "ar", direction: "rtl" };
const directional: Literal = factory.literal("مرحبا", direction);

const quoted: RdfTerm = factory.quotedTriple(subject, predicate, typed);
const quad: Quad = factory.quad(quoted, predicate, directional);

const dataset = new Dataset();
const chained: Dataset = dataset.add(quad).delete(quad).add(quad);
const rebuilt: Dataset = Dataset.from(chained);
const rebuiltFromNull: Dataset = Dataset.from(null);
const fromFactory: Dataset = factory.dataset(rebuilt);
const fromFactoryNull: Dataset = factory.dataset(null);
const matched: Dataset = fromFactory.match(null, predicate, undefined, factory.variable("g"));

for (const item of matched) {
  const term: RdfTerm = item.object;
  term.equals(null);
  item.equals(undefined);
}

const stream: AsyncIterableIterator<Quad> = matched.toStream();
const serialized: string = matched.serialize("nquads");
const serializeLoss: SerializeLoss = matched.serializeWithLoss("ntriples");
const lossText: string = serializeLoss.text;
const lossStatementRows: number = serializeLoss.statementRowsDropped;
const lossDirectional: number = serializeLoss.directionalLiteralsDropped;
const lossNamedGraphRows: number = serializeLoss.namedGraphRowsDropped;
const configured: string = matched.serializeConfigured(
  "jsonld",
  JSON.stringify({ version: 1, mode: "derived" }),
);
const compiledContext = new CompiledJsonLdContext(
  JSON.stringify({
    version: 1,
    mode: "context",
    prefixes: { ex: "https://example.org/" },
  }),
);
const compacted: string = matched.serializeWithContext("jsonld", compiledContext);
const canonical: string = matched.canonicalize();
const same: boolean = matched.isomorphic(Dataset.parse(serialized, "nquads"));
const projection: ProjectionPackage = matched.project("lpg-csv", JSON.stringify({
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
}));
const projectionLedger: ProjectionLossLedger = JSON.parse(projection.lossLedgerJson);
const curatedProfile: ProjectionProfile = "csvw-terms";
const okfTermsProfile: ProjectionProfile = "okf-terms";
const dcatRdfProfile: ProjectionProfile = "dcat-rdf";
const voidProfile: ProjectionProfile = "void";
// @ts-expect-error curated CSVW terms cannot reconstruct arbitrary source RDF
const invalidCuratedLift: LiftProfile = "csvw-terms";
// @ts-expect-error curated OKF terms cannot reconstruct arbitrary source RDF
const invalidOkfTermsLift: LiftProfile = "okf-terms";
// @ts-expect-error native DCAT RDF is a write-only description view
const invalidDcatRdfLift: LiftProfile = "dcat-rdf";
// @ts-expect-error VoID is a write-only description view
const invalidVoidLift: LiftProfile = "void";
const researchProjection: ProjectionPackage = matched.project(
  "frictionless-data-package-1",
  "{}",
);
const projectionLift: ProjectionLift = liftProjection(
  projection.archive,
  "lpg-csv",
  JSON.stringify({
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
  }),
);
const projectedDataset: Dataset | undefined = projectionLift.takeDataset();
const visualModel: VisualModel = matched.visualModel({ mode: "compact" });
const visualExport: VisualExport = matched.visualExport({
  mode: "incidence",
  vocabulary: [{ prefix: "ex", namespace: "https://example.org/" }],
  maxStatements: 500,
});
const visualSvg: VisualSvgDocument = matched.visualSvg({
  mode: "table",
  tableFields: ["statement", "assertedIn", "diagnostics"],
  svg: { title: "RDF 1.2 statements", embedMetadata: true },
});
const queryJson: string = matched.query("ASK { ?s ?p ?o }");
const engine = new QueryEngine();
const select: SelectResult = engine.select(matched, "SELECT ?s WHERE { ?s ?p ?o }");
const maybeTerm: RdfTerm | undefined = select.rows.take(0)?.s;
const ask: boolean = engine.ask(matched, "ASK { ?s ?p ?o }");
const graph: Dataset = engine.construct(
  matched,
  "CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }",
);
const rawResults: string = engine.queryRaw(matched, "ASK { ?s ?p ?o }", { format: "json" });
const rawResultsWithProvenance: string = engine.queryRaw(matched, "ASK { ?s ?p ?o }", {
  format: "json",
  provenanceNamespace: { prefix: "prov", iri: "https://example.org/ns/prov#" },
});
const decodedProvenance: ProvenanceInfo = provenanceFromJson(
  rawResultsWithProvenance,
  "prov",
  "https://example.org/ns/prov#",
);
const decodedEngine: string | undefined = decodedProvenance.engine;
void decodedEngine;
const rawGraph: string = engine.queryRaw(
  matched,
  "CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }",
  { format: "nquads" },
);
const updated: Dataset = engine.update(
  new Dataset(),
  "INSERT DATA { <https://example.org/u> <https://example.org/p> <https://example.org/o> }",
);
const dimensions: string[] = governorDimensions();
const cancel = new CancellationToken();
// @ts-expect-error governor keys are accepted only by governed entry points
engine.query(matched, "ASK { ?s ?p ?o }", { fuel: 1 });
const outcome: QueryOutcome = engine.queryGoverned(
  matched,
  "SELECT ?s WHERE { ?s ?p ?o }",
  { fuel: 100_000, deadlineMs: 250, maxAnswers: 10n, cancel },
);
const receipt: GovernorEvidence = outcome.evidence;
const spentFuel: bigint = receipt.consumed.fuel;
if (!outcome.isComplete) {
  const tripped: TrippedGovernor = outcome.tripped!;
  const label: string = tripped.label;
  const partial: PartialAnswers = outcome.partial!;
  const certainty: string = partial.certainty;
  void label;
  void certainty;
}
const applied: UpdateOutcome = engine.updateGoverned(
  new Dataset(),
  "INSERT DATA { <https://example.org/u> <https://example.org/p> <https://example.org/o> }",
  { fuel: 100_000 },
);
const entailed: EntailmentQueryOutcome = engine.queryEntailmentGoverned(
  matched,
  "SELECT ?s WHERE { ?s ?p ?o }",
  "rdfs",
  { fuel: 100_000, program: null },
);
const entailmentPhase: "answered" | "closure-stopped" = entailed.phase;
const ledger: string = engine.explainQuery(matched, "SELECT ?s WHERE { ?s ?p ?o }");

// The asynchronous twins.
const asyncAvailable: boolean = hasAsyncQueries();
const yieldPrimitive: "scheduler.yield" | "setImmediate" | "MessageChannel" | undefined =
  asyncYieldPrimitive();
configureAsync({ maxConcurrentJobs: 8 });
const profile: ServiceProfileJson = {
  capabilities: ["query", "network"],
  headers: [["X-Tenant", "a"]],
  userAgent: "example-worker/1.0",
  timeoutMs: 5_000,
};
const catalog = new ServiceCatalog();
catalog.addService("https://example.org/sparql", JSON.stringify(profile));
catalog.setFallback(JSON.stringify({ capabilities: [] }));
const resolveService: AsyncServiceResolver = async (request, ctx) => {
  const endpoint: string = request.endpoint;
  const headers: [string, string][] = request.headers;
  const silent: boolean = ctx.silent;
  const cells: bigint | undefined = ctx.maxIntermediateCells;
  const remaining: number | undefined = ctx.remainingDeadlineMs;
  void endpoint;
  void headers;
  void silent;
  void cells;
  void remaining;
  if (ctx.signal.aborted) return { kind: "transport", message: "aborted" };
  return fetch(request.endpoint, {
    method: "POST",
    headers: [["content-type", request.contentType], ["accept", request.accept], ...headers],
    body: request.queryText,
    signal: ctx.signal,
  });
};
const resolveLoad: AsyncLoadResolver = (request) =>
  request.iri.endsWith(".ttl")
    ? { text: "<https://example.org/s> <https://example.org/p> 1 .", mediaType: "text/turtle" }
    : { kind: "denied", message: `not allowed: ${request.iri}` };
const controller = new AbortController();
const asyncResult: Promise<QueryResult> = engine.queryAsync(matched, "ASK { ?s ?p ?o }", {
  resolveService,
  signal: controller.signal,
  catalog,
  yieldEveryPolls: 1_024,
  stackBytes: 4 * 1024 * 1024,
});
const asyncSelect: Promise<SelectResult> = engine.selectAsync(matched, "SELECT ?s WHERE { ?s ?p ?o }", {
  localServices: { "https://example.org/local": matched },
});
const asyncAsk: Promise<boolean> = engine.askAsync(matched, "ASK { ?s ?p ?o }", { base: "https://example.org/" });
const asyncConstruct: Promise<Dataset> = engine.constructAsync(
  matched,
  "CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }",
);
const asyncDescribe: Promise<Dataset> = engine.describeAsync(matched, "DESCRIBE <https://example.org/s>");
const asyncRaw: Promise<string> = engine.queryRawAsync(matched, "ASK { ?s ?p ?o }", {
  format: "json",
  provenanceNamespace: { prefix: "prov", iri: "https://example.org/ns/prov#" },
});
const asyncRawConfigured: Promise<string> = engine.queryRawAsync(
  matched,
  "CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }",
  { format: "jsonld", optionsJson: JSON.stringify({ version: 1, mode: "derived" }) },
);
const asyncRawBytes: Promise<Uint8Array> = engine.queryRawBytesAsync(matched, "ASK { ?s ?p ?o }");
const asyncRawWithContext: Promise<string> = engine.queryRawWithContextAsync(
  matched,
  "CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }",
  "jsonld",
  compiledContext,
  { yamlSchemaUrl: null },
);
const asyncGoverned: AsyncQueryOutcome = await engine.queryGovernedAsync(
  matched,
  "SELECT ?s WHERE { ?s ?p ?o }",
  { fuel: 100_000, deadlineMs: 250, maxRemoteRequests: 4, resolveService, signal: controller.signal },
);
const asyncEvidence: AsyncEvidence = asyncGoverned.evidence.async;
const stackHighWater: number = asyncEvidence.stackHighWaterBytes;
const asyncEntailed: AsyncEntailmentQueryOutcome = await engine.queryEntailmentGovernedAsync(
  matched,
  "SELECT ?s WHERE { ?s ?p ?o }",
  "rdfs",
  { fuel: 100_000, program: null },
);
const entailedEvidence: AsyncEvidence = asyncEntailed.evidence.async;
const asyncUpdated: Promise<Dataset> = engine.updateAsync(
  new Dataset(),
  "LOAD <https://example.org/doc.ttl>",
  { resolveLoad },
);
const asyncApplied: AsyncUpdateOutcome = await engine.updateGovernedAsync(
  new Dataset(),
  "INSERT DATA { <https://example.org/u> <https://example.org/p> <https://example.org/o> }",
  { fuel: 100_000 },
);
const appliedEvidence: AsyncEvidence = asyncApplied.evidence.async;
const datasetAsync: Promise<string> = matched.queryAsync("ASK { ?s ?p ?o }", { resolveService });
const datasetId: number = matched.id;
const datasetGeneration: number = matched.generation;
// @ts-expect-error an asynchronous twin is cancelled through `signal`, never a token
engine.queryAsync(matched, "ASK { ?s ?p ?o }", { cancel });
// @ts-expect-error governor keys are accepted only by the governed asynchronous twins
engine.queryAsync(matched, "ASK { ?s ?p ?o }", { fuel: 1 });
// @ts-expect-error a typed twin returns no document, so it takes no format
engine.selectAsync(matched, "SELECT ?s WHERE { ?s ?p ?o }", { format: "json" });
asyncResult.catch((error: AsyncJobError) => {
  const evidence: AsyncEvidence = error.evidence.async;
  void evidence;
});

const result: QueryResult = engine.query(matched, "ASK { ?s ?p ?o }");
if (result.kind === "ask") {
  const narrowed: boolean = result.boolean;
  void narrowed;
}

void stream;
void canonical;
void lossText;
void lossStatementRows;
void lossDirectional;
void lossNamedGraphRows;
void same;
void projectionLedger;
void curatedProfile;
void okfTermsProfile;
void dcatRdfProfile;
void voidProfile;
void invalidCuratedLift;
void entailmentPhase;
void invalidOkfTermsLift;
void invalidDcatRdfLift;
void invalidVoidLift;
void researchProjection;
void projectedDataset;
void visualModel;
void visualExport;
void visualSvg;
void queryJson;
void language;
void maybeTerm;
void ask;
void graph;
void rawResults;
void rawGraph;
void updated;
void dimensions;
void spentFuel;
void applied;
void ledger;
void rebuiltFromNull;
void fromFactoryNull;
void asyncAvailable;
void yieldPrimitive;
void asyncSelect;
void asyncAsk;
void asyncConstruct;
void asyncDescribe;
void asyncRaw;
void asyncRawConfigured;
void asyncRawBytes;
void asyncRawWithContext;
void stackHighWater;
void entailedEvidence;
void asyncUpdated;
void appliedEvidence;
void datasetAsync;
void datasetId;
void datasetGeneration;
