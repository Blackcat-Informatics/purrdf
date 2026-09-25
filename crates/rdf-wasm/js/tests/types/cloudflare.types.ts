// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Compile-only checks for the `./cloudflare` subpath and the protocol surface it rests on
// (`npm run typecheck`). Nothing here runs.

import {
  Dataset,
  QueryEngine,
  ready,
  ServiceCatalog,
  SparqlProtocolRequest,
  type AsyncLoadResolver,
  type AsyncNegotiatedQueryOutcome,
  type AsyncServiceResolver,
  type LoadAuthorization,
  type ProtocolFormat,
  type ProtocolResultKind,
  type SparqlProtocolError,
} from "@blackcatinformatics/purrdf";
import {
  createFetchLoadResolver,
  createFetchServiceResolver,
  handleSparqlRequest,
  type CacheLike,
  type EndpointGovernors,
  type ExecutionContextLike,
  type FetchLike,
  type ServiceBindingLike,
  type SparqlEndpointOptions,
  type SparqlProblem,
} from "@blackcatinformatics/purrdf/cloudflare";

declare const wasmModule: WebAssembly.Module;
declare const cache: CacheLike;
declare const ctx: ExecutionContextLike;
declare const remote: ServiceBindingLike;

await ready(wasmModule);

const engine = new QueryEngine();
const dataset = new Dataset();
const catalog = new ServiceCatalog();
catalog.addService(
  "https://remote.example.org/sparql",
  JSON.stringify({ capabilities: ["query", "network"] }),
);
const carries: boolean = catalog.carriesCredential("https://remote.example.org/sparql");
const load: LoadAuthorization = catalog.authorizeLoad("https://remote.example.org/doc.ttl");
const denial: string | undefined = load.denial;
const accept: string = load.accept;
load.free();

const fetchLike: FetchLike = (input, init) => fetch(input, init);
const resolveService: AsyncServiceResolver = createFetchServiceResolver({
  catalog,
  timeoutMs: 5000,
  fetch: fetchLike,
  bindings: { "https://remote.example.org": remote },
  cache,
  cacheTtlSeconds: 60,
  waitUntil: (promise) => ctx.waitUntil(promise),
});
const resolveLoad: AsyncLoadResolver = createFetchLoadResolver({ catalog, timeoutMs: 5000 });

const governors: EndpointGovernors = { deadlineMs: 10_000, maxRemoteRequests: 40, fuel: 1_000_000n };
const options: SparqlEndpointOptions = {
  engine,
  dataset,
  governors,
  catalog,
  resolveService,
  resolveLoad,
  cors: { origins: ["https://app.example.org"], maxAgeSeconds: 600 },
};
const response: Response = await handleSparqlRequest(new Request("https://example.org/sparql"), options);
if (response.headers.get("Content-Type") === "application/problem+json") {
  const problem = (await response.json()) as SparqlProblem;
  const code: string = problem.code;
  void code;
}

const request = SparqlProtocolRequest.parse("GET", undefined, "query=ASK%20%7B%7D", new Uint8Array(0));
const kind: "query" | "update" = request.kind;
const resultKind: "solutions" | "boolean" | "graph" | undefined = request.resultKind;
const format: ProtocolFormat | undefined = request.negotiate("application/sparql-results+xml");
const text: string = request.effectiveText();
const offered: string[] = SparqlProtocolRequest.offeredMediaTypes("dataset" satisfies ProtocolResultKind);
const media: string | undefined = SparqlProtocolRequest.formatMediaType("trig");
request.free();
try {
  SparqlProtocolRequest.parse("PUT", null, null, new Uint8Array(0));
} catch (error) {
  const refusal = error as SparqlProtocolError;
  const status: 400 | 405 | 415 = refusal.status;
  const parameter: string | undefined = refusal.parameter;
  void status;
  void parameter;
}

const negotiated: AsyncNegotiatedQueryOutcome = await engine.queryGovernedNegotiatedAsync(
  dataset,
  "SELECT * WHERE { ?s ?p ?o }",
  { accept: "text/csv", deadlineMs: 1000 },
);
if (negotiated.isComplete && negotiated.body !== undefined) {
  const bytes: Uint8Array = negotiated.body.bytes;
  const tokenFormat: ProtocolFormat = negotiated.body.format;
  void bytes;
  void tokenFormat;
}
const serviceWait: number = negotiated.evidence.async.serviceWaitMs;

void carries;
void denial;
void accept;
void kind;
void resultKind;
void format;
void text;
void offered;
void media;
void serviceWait;
