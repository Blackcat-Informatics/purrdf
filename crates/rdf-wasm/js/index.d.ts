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
