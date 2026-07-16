// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

import {
  ready,
  DataFactory,
  Dataset,
  liftProjection,
  type ProjectionLift,
  type ProjectionLossLedger,
  type ProjectionPackage,
  QueryEngine,
  type DirectionalLanguage,
  type Literal,
  type NamedNode,
  type Quad,
  type QueryResult,
  type SelectResult,
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
const canonical: string = matched.canonicalize();
const same: boolean = matched.isomorphic(Dataset.parse(serialized, "nquads"));
const projection: ProjectionPackage = matched.project("lpg-csv", JSON.stringify({
  profile: "lpg-csv",
  config: {
    rdf_type: "https://example.org/type",
    limits: {
      max_artifacts: 16,
      max_artifact_bytes: 1_000_000,
      max_total_bytes: 4_000_000,
      max_archive_bytes: 5_000_000,
      max_term_depth: 16,
    },
    max_records: 1_000,
  },
}));
const projectionLedger: ProjectionLossLedger = JSON.parse(projection.lossLedgerJson);
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
      limits: {
        max_artifacts: 16,
        max_artifact_bytes: 1_000_000,
        max_total_bytes: 4_000_000,
        max_archive_bytes: 5_000_000,
        max_term_depth: 16,
      },
      max_records: 1_000,
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
const rawGraph: string = engine.queryRaw(
  matched,
  "CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }",
  { format: "nquads" },
);
const updated: Dataset = engine.update(
  new Dataset(),
  "INSERT DATA { <https://example.org/u> <https://example.org/p> <https://example.org/o> }",
);
const result: QueryResult = engine.query(matched, "ASK { ?s ?p ?o }");
if (result.kind === "ask") {
  const narrowed: boolean = result.boolean;
  void narrowed;
}

void stream;
void canonical;
void same;
void projectionLedger;
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
void rebuiltFromNull;
void fromFactoryNull;
