// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

export type LiteralDirection = "ltr" | "rtl";

export type TermType =
  | "NamedNode"
  | "BlankNode"
  | "Literal"
  | "Variable"
  | "DefaultGraph"
  | "Quad";

export interface DirectionalLanguage {
  readonly language: string;
  readonly direction: LiteralDirection;
}

export type LanguageOrDatatype =
  | string
  | NamedNode
  | DirectionalLanguage
  | null
  | undefined;

export type QuadGraph = NamedNode | BlankNode | DefaultGraph;
export type RdfTerm = NamedNode | BlankNode | Literal | Variable | DefaultGraph | QuotedTriple;
export type QueryGraphFormat =
  | "turtle"
  | "ttl"
  | "ntriples"
  | "nt"
  | "nquads"
  | "nq"
  | "trig"
  | "rdfxml"
  | "jsonld"
  | "text/turtle"
  | "application/n-triples"
  | "application/n-quads"
  | "application/trig"
  | "application/rdf+xml"
  | "application/ld+json";
export type QueryResultsFormat =
  | "json"
  | "srj"
  | "xml"
  | "csv"
  | "tsv"
  | "application/sparql-results+json"
  | "application/sparql-results+xml"
  | "text/csv"
  | "text/tab-separated-values";
export type QueryRawFormat = QueryGraphFormat | QueryResultsFormat;

export interface QueryOptions {
  readonly base?: string | null;
}

export interface QueryRawOptions extends QueryOptions {
  readonly format?: QueryRawFormat | string | null;
}

export type QueryBindingRow = Record<string, RdfTerm | undefined>;

export interface SelectResult {
  readonly kind: "select";
  readonly variables: string[];
  readonly rows: QueryBindingRow[];
}

export interface AskResult {
  readonly kind: "ask";
  readonly boolean: boolean;
}

export interface GraphResult {
  readonly kind: "graph";
  readonly dataset: Dataset;
}

export type QueryResult = SelectResult | AskResult | GraphResult;

export type VizMode = "compact" | "incidence" | "table";
export type VizDialect = "rdf12" | "symmetricRdf12" | "generalizedRdf";
export type VizRole =
  | "focus"
  | "reifier"
  | "graphName"
  | "predicate"
  | "quotedStatement"
  | "assertedStatement"
  | "annotatedStatement"
  | { custom: string };

export interface VisualOptions {
  readonly mode?: VizMode;
  readonly focus?: string | null;
  readonly graph?: string | readonly string[] | null;
  readonly maxDepth?: number;
  readonly maxStatements?: number;
  readonly maxTerms?: number;
  readonly width?: number;
  readonly margin?: number;
  readonly rowHeight?: number;
  readonly embedMetadata?: boolean;
  readonly includeStyles?: boolean;
  readonly spec?: Record<string, unknown>;
  readonly svg?: Record<string, unknown>;
}

export type VizValueRef =
  | { readonly kind: "term"; readonly id: string }
  | { readonly kind: "statement"; readonly id: string };

export type VizTermValue =
  | { readonly kind: "iri"; readonly value: string }
  | { readonly kind: "blank"; readonly label: string; readonly scope: number }
  | {
      readonly kind: "literal";
      readonly lexical_form: string;
      readonly datatype: string;
      readonly language: string | null;
      readonly direction: LiteralDirection | null;
    };

export interface VizTerm {
  readonly id: string;
  readonly value: VizTermValue;
  readonly label: string;
  readonly roles: VizRole[];
}

export interface VizStatement {
  readonly id: string;
  readonly subject: VizValueRef;
  readonly predicate: string;
  readonly object: VizValueRef;
  readonly asserted_in: string[];
  readonly nesting_depth: number;
  readonly incoming_references: number;
  readonly dialect: VizDialect;
  readonly roles: VizRole[];
}

export interface VizAssertion {
  readonly id: string;
  readonly statement: string;
  readonly graph: string;
}

export type VizRelation =
  | {
      readonly kind: "reifies";
      readonly id: string;
      readonly reifier: string;
      readonly statement: string;
      readonly graph: string;
    }
  | {
      readonly kind: "annotation";
      readonly id: string;
      readonly reifier: string;
      readonly predicate: string;
      readonly object: VizValueRef;
      readonly graph: string;
    };

export interface VizGraph {
  readonly id: string;
  readonly term: string | null;
  readonly label: string;
}

export interface VizReference {
  readonly id: string;
  readonly statement: string;
  readonly source: string;
}

export interface VizTableRow {
  readonly statement: string;
  readonly asserted_in: string[];
  readonly reifier_count: number;
  readonly annotation_count: number;
  readonly referenced_by: number;
  readonly depth: number;
}

export interface VizDiagnostic {
  readonly code: string;
  readonly message: string;
  readonly target: string | null;
  readonly dialect: VizDialect;
}

export interface VizModel {
  readonly terms: VizTerm[];
  readonly statements: VizStatement[];
  readonly assertions: VizAssertion[];
  readonly relations: VizRelation[];
  readonly graphs: VizGraph[];
  readonly references: VizReference[];
  readonly table: VizTableRow[];
  readonly diagnostics: VizDiagnostic[];
}

export interface VizLayoutRecord {
  readonly id: string;
  readonly x: number;
  readonly y: number;
}

export interface VizElementIndexEntry {
  readonly element_id: string;
  readonly model_id: string;
  readonly kind: string;
}

export interface VizExport {
  readonly schema_version: "purrdf-viz-export-1";
  readonly spec_hash: string;
  readonly model: VizModel;
  readonly layout: VizLayoutRecord[];
  readonly element_index: VizElementIndexEntry[];
  readonly diagnostics: VizDiagnostic[];
}

export class Term {
  private constructor();
  free(): void;
  readonly termType: TermType;
  readonly value: string;
  readonly language: string;
  readonly direction: "" | LiteralDirection;
  readonly datatype: NamedNode | undefined;
  readonly subject: RdfTerm | undefined;
  readonly predicate: NamedNode | undefined;
  readonly object: RdfTerm | undefined;
  readonly graph: DefaultGraph | undefined;
  equals(other: Term | null | undefined): boolean;
}

export interface NamedNode extends Term {
  readonly termType: "NamedNode";
  readonly value: string;
  readonly language: "";
  readonly direction: "";
  readonly datatype: undefined;
}

export interface BlankNode extends Term {
  readonly termType: "BlankNode";
  readonly value: string;
  readonly language: "";
  readonly direction: "";
  readonly datatype: undefined;
}

export interface Literal extends Term {
  readonly termType: "Literal";
  readonly value: string;
  readonly language: string;
  readonly direction: "" | LiteralDirection;
  readonly datatype: NamedNode;
}

export interface Variable extends Term {
  readonly termType: "Variable";
  readonly value: string;
  readonly language: "";
  readonly direction: "";
  readonly datatype: undefined;
}

export interface DefaultGraph extends Term {
  readonly termType: "DefaultGraph";
  readonly value: "";
  readonly language: "";
  readonly direction: "";
  readonly datatype: undefined;
}

export interface QuotedTriple extends Term {
  readonly termType: "Quad";
  readonly value: "";
  readonly language: "";
  readonly direction: "";
  readonly datatype: undefined;
  readonly subject: RdfTerm;
  readonly predicate: NamedNode;
  readonly object: RdfTerm;
  readonly graph: DefaultGraph;
}

export class Quad {
  private constructor();
  free(): void;
  readonly termType: "Quad";
  readonly value: "";
  readonly subject: RdfTerm;
  readonly predicate: NamedNode;
  readonly object: RdfTerm;
  readonly graph: QuadGraph;
  asTerm(): QuotedTriple;
  equals(other: Quad | null | undefined): boolean;
}

export class DataFactory {
  constructor();
  namedNode(value: string): NamedNode;
  blankNode(value?: string | null): BlankNode;
  literal(value: string, languageOrDatatype?: LanguageOrDatatype): Literal;
  typedLiteral(value: string, datatype: NamedNode): Literal;
  directionalLiteral(
    value: string,
    language: string,
    direction: LiteralDirection,
  ): Literal;
  variable(value: string): Variable;
  defaultGraph(): DefaultGraph;
  quad(
    subject: RdfTerm,
    predicate: NamedNode,
    object: RdfTerm,
    graph?: QuadGraph | null,
  ): Quad;
  quotedTriple(subject: RdfTerm, predicate: NamedNode, object: RdfTerm): QuotedTriple;
  fromTerm<T extends Term>(original: T): T;
  fromQuad(original: Quad): Quad;
  dataset(quads?: Iterable<Quad> | null): Dataset;
}

export class Dataset implements Iterable<Quad> {
  constructor();
  static parse(input: string, format: string, base?: string | null): Dataset;
  static from(quads?: Iterable<Quad> | null): Dataset;
  readonly size: number;
  add(quad: Quad): this;
  delete(quad: Quad): this;
  has(quad: Quad): boolean;
  match(
    subject?: Term | null,
    predicate?: Term | null,
    object?: Term | null,
    graph?: Term | null,
  ): Dataset;
  quads(): Quad[];
  serialize(format: string): string;
  query(sparql: string, base?: string | null): string;
  canonicalize(): string;
  isomorphic(other: Dataset): boolean;
  visualModel(options?: VisualOptions | string | null): VizModel;
  visualExport(options?: VisualOptions | string | null): VizExport;
  visualSvg(options?: VisualOptions | string | null): string;
  visualModelJson(optionsJson?: string | null): string;
  visualExportJson(optionsJson?: string | null): string;
  toStream(): AsyncIterableIterator<Quad>;
  [Symbol.iterator](): IterableIterator<Quad>;
  free(): void;
}

export class QueryEngine {
  constructor();
  query(dataset: Dataset, sparql: string, options?: QueryOptions | null): QueryResult;
  select(dataset: Dataset, sparql: string, options?: QueryOptions | null): SelectResult;
  ask(dataset: Dataset, sparql: string, options?: QueryOptions | null): boolean;
  construct(dataset: Dataset, sparql: string, options?: QueryOptions | null): Dataset;
  describe(dataset: Dataset, sparql: string, options?: QueryOptions | null): Dataset;
  update(dataset: Dataset, sparql: string, options?: QueryOptions | null): Dataset;
  queryRaw(dataset: Dataset, sparql: string, options?: QueryRawOptions | null): string;
  free(): void;
}

export class Sink {
  constructor();
  push(quad: Quad): void;
  finish(): Dataset;
  free(): void;
}

export function ready(wasmBytesOrUrl?: BufferSource | URL | string): Promise<void>;
export function datasetToStream(dataset: Dataset): AsyncIterableIterator<Quad>;
export function streamToDataset(
  quadStream: AsyncIterable<Quad> | Iterable<Quad>,
): Promise<Dataset>;
export function shaclEntail(shapesTtl: string, dataNt: string): string;
export function shaclValidateToSarif(shapesTtl: string, dataNt: string): string;
export function version(): string;
