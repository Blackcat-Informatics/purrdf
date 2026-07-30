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

/** Closed, versioned options document consumed by the shared Rust JSON-LD engine. */
export type JsonLdSerializeOptions =
  | { readonly version: 1; readonly mode: "expanded"; readonly yaml_schema_url?: string }
  | { readonly version: 1; readonly mode: "derived"; readonly yaml_schema_url?: string }
  | {
      readonly version: 1;
      readonly mode: "context";
      readonly prefixes?: Readonly<Record<string, string>>;
      readonly context?: unknown;
      readonly document_iri?: string;
      readonly registry?: Readonly<Record<string, unknown>>;
      readonly yaml_schema_url?: string;
    };

export type QueryBindingRow = Record<string, RdfTerm | undefined>;

export interface QueryBindingRows extends IterableIterator<QueryBindingRow> {
  /** Total row count, including rows already consumed. */
  readonly length: number;
  /** Rows not yet consumed. */
  readonly remaining: number;
  /** Move one row out by result index. Each index can be consumed once. */
  take(index: number): QueryBindingRow | undefined;
  /** Materialize all remaining rows and exhaust the stream. */
  toArray(): QueryBindingRow[];
  /** Release unconsumed wasm result storage. */
  free(): void;
}

export interface SelectResult {
  readonly kind: "select";
  readonly variables: string[];
  readonly rowCount: number;
  readonly rows: QueryBindingRows;
  /** Release unconsumed wasm result storage. */
  free(): void;
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

export type VisualMode = "compact" | "incidence" | "table";
export type VisualLabelPolicy = "compact" | "full";
export type VisualTableField =
  | "statement"
  | "assertedIn"
  | "reifiers"
  | "annotations"
  | "referencedBy"
  | "depth"
  | "diagnostics";

export interface VisualVocabularyMapping {
  readonly prefix: string;
  readonly namespace: string;
}

export interface VisualRoleRule {
  readonly predicateIri: string;
  readonly role: string;
}

export interface VisualLayoutOptions {
  readonly margin?: number;
  readonly rankSpacing?: number;
  readonly nodeSpacing?: number;
  readonly componentSpacing?: number;
  readonly componentWrapWidth?: number;
  readonly crossingSweeps?: number;
  readonly maxNodeWidth?: number;
}

export interface VisualSvgOptions {
  readonly embedMetadata?: boolean;
  readonly includeStyles?: boolean;
  readonly title?: string;
}

export interface VisualizationOptions {
  readonly mode?: VisualMode;
  readonly focus?: string | null;
  readonly roleRules?: readonly VisualRoleRule[];
  readonly vocabulary?: readonly VisualVocabularyMapping[];
  readonly graph?: string | null;
  readonly graphs?: readonly string[];
  readonly labelPolicy?: VisualLabelPolicy;
  readonly maxStatements?: number;
  readonly maxTerms?: number;
  readonly tableFields?: readonly VisualTableField[];
  readonly layout?: VisualLayoutOptions;
  readonly svg?: VisualSvgOptions;
}

export type VisualValueRef =
  | { readonly kind: "term"; readonly id: string }
  | { readonly kind: "statement"; readonly id: string };

export type VisualTermValue =
  | { readonly kind: "iri"; readonly value: string }
  | { readonly kind: "blank"; readonly label: string; readonly scope: number }
  | {
      readonly kind: "literal";
      readonly lexical_form: string;
      readonly datatype: string;
      readonly language: string | null;
      readonly direction: LiteralDirection | null;
    };

export interface VisualTerm {
  readonly id: string;
  readonly value: VisualTermValue;
  readonly label: string;
  readonly roles: readonly unknown[];
}

export interface VisualStatement {
  readonly id: string;
  readonly subject: VisualValueRef;
  readonly predicate: string;
  readonly object: VisualValueRef;
  readonly asserted_in: readonly string[];
  readonly nesting_depth: number;
  readonly incoming_references: number;
  readonly dialect: "rdf12" | "symmetricRdf12" | "generalizedRdf";
  readonly roles: readonly unknown[];
}

export type VisualRelation =
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
      readonly object: VisualValueRef;
      readonly graph: string;
    };

export interface VisualDiagnostic {
  readonly id: string;
  readonly code: string;
  readonly message: string;
  readonly target: string | null;
  readonly dialect: VisualStatement["dialect"];
}

export interface VisualModel {
  readonly terms: readonly VisualTerm[];
  readonly statements: readonly VisualStatement[];
  readonly assertions: readonly Readonly<Record<string, unknown>>[];
  readonly relations: readonly VisualRelation[];
  readonly graphs: readonly Readonly<Record<string, unknown>>[];
  readonly references: readonly Readonly<Record<string, unknown>>[];
  readonly table: Readonly<Record<string, unknown>>;
  readonly diagnostics: readonly VisualDiagnostic[];
}

export interface VisualScene {
  readonly schema_version: "purrdf-viz-scene-1";
  readonly mode: VisualMode;
  readonly nodes: readonly Readonly<Record<string, unknown>>[];
  readonly edges: readonly Readonly<Record<string, unknown>>[];
  readonly groups: readonly Readonly<Record<string, unknown>>[];
  readonly legend: readonly Readonly<Record<string, unknown>>[];
  readonly table: Readonly<Record<string, unknown>> | null;
}

export interface VisualLayout {
  readonly schema_version: "purrdf-viz-layout-1";
  readonly mode: VisualMode;
  readonly width: number;
  readonly height: number;
  readonly nodes: readonly Readonly<Record<string, unknown>>[];
  readonly edges: readonly Readonly<Record<string, unknown>>[];
  readonly table: Readonly<Record<string, unknown>> | null;
  readonly legend: readonly Readonly<Record<string, unknown>>[];
}

export interface VisualExport {
  readonly schema_version: "purrdf-viz-export-1";
  readonly spec: Readonly<Record<string, unknown>>;
  readonly spec_hash: string;
  readonly model_hash: string;
  readonly scene_hash: string;
  readonly model: VisualModel;
  readonly scene: VisualScene;
  readonly layout: VisualLayout;
  readonly element_index: readonly Readonly<Record<string, unknown>>[];
  readonly diagnostics: readonly VisualDiagnostic[];
}

export interface VisualSvgDocument {
  readonly svg: string;
  readonly export: VisualExport;
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
  project(profile: ProjectionProfile, configJson: string): ProjectionPackage;
  projectWithAssets(
    profile: "ro-crate-1.3",
    configJson: string,
    assetsArchive: Uint8Array,
  ): ProjectionPackage;
  visualModel(options?: VisualizationOptions | null): VisualModel;
  visualExport(options?: VisualizationOptions | null): VisualExport;
  visualSvg(options?: VisualizationOptions | null): VisualSvgDocument;
  serialize(format: string): string;
  serializeConfigured(format: "jsonld" | "yamlld" | string, optionsJson: string): string;
  serializeWithContext(
    format: "jsonld" | "yamlld" | string,
    context: CompiledJsonLdContext,
    yamlSchemaUrl?: string | null,
  ): string;
  query(sparql: string, base?: string | null): string;
  canonicalize(): string;
  isomorphic(other: Dataset): boolean;
  toStream(): AsyncIterableIterator<Quad>;
  [Symbol.iterator](): IterableIterator<Quad>;
  free(): void;
}

export class CompiledJsonLdContext {
  constructor(optionsJson: string);
  canonicalContextJson(): string;
  free(): void;
}

export type ProjectionProfile =
  | "lpg-csv"
  | "neo4j-csv"
  | "open-cypher"
  | "graphml"
  | "csvw-exact"
  | "csvw-terms"
  | "okf-terms"
  | "obo-graphs"
  | "skos"
  | "croissant-1.1"
  | "ro-crate-1.3"
  | "datacite-4.6"
  | "dcat-3"
  | "dcat-rdf"
  | "void"
  | "frictionless-data-package-1";

export type LiftProfile = Exclude<
  ProjectionProfile,
  "csvw-terms" | "okf-terms" | "obo-graphs" | "skos" | "dcat-rdf" | "void"
>;

export interface ProjectionLossLedger {
  schema_version: 1;
  losses: Array<{
    code: string;
    from: string;
    to: string;
    intentional: boolean;
    note: string;
    location?: string;
  }>;
}

export class ProjectionPackage {
  private constructor();
  readonly profile: ProjectionProfile;
  readonly archive: Uint8Array;
  readonly lossLedgerJson: string;
  free(): void;
}

export class ProjectionLift {
  private constructor();
  readonly lossLedgerJson: string;
  takeDataset(): Dataset | undefined;
  free(): void;
}

/**
 * One closure of one document under one SPARQL entailment regime: the canonical
 * N-Quads and the byte-stable rendered reasoning report. Returned by
 * `entailMaterialize`.
 */
export class RegimeClosure {
  private constructor();
  /** The materialized dataset (input quads + inferred triples) as canonical N-Quads. */
  readonly nquads: string;
  /** What the run did: rules fired, rules missing, boundaries, budget, contract hash. */
  readonly report: string;
  free(): void;
}

/**
 * One Description-Logic reasoning service's answer and the certificate of the run
 * that produced it. Returned by `entailConsistency`, `entailClassify`,
 * `entailRealize`, `entailInstances`, `entailEntails`, `entailProfile`,
 * `entailExtractModule`, `entailJustify`, and `entailExplainConclusion`.
 */
export class ReasoningAnswer {
  private constructor();
  /**
   * The service's answer, in the service's own line-oriented rendering. A
   * dataset-valued answer (`entailExtractModule`, `entailJustify`) is canonical
   * (RDFC-1.0) N-Quads instead.
   */
  readonly answer: string;
  /**
   * How completely the service decided: the `purrdf-dl-certificate 1` block for a
   * tableau service (`entailConsistency`, `entailClassify`, `entailRealize`,
   * `entailInstances`, `entailEntails`), or that service's own certificate grammar
   * for the three that run no tableau (`entailProfile`, `entailExtractModule`,
   * `entailExplainConclusion`).
   */
  readonly certificate: string;
  free(): void;
}

/**
 * A reasoning session over one ontology.
 *
 * Every `entail*` function takes the document as a string and rebuilds everything it
 * needs, so asking three questions parses and reverse-maps the ontology three times.
 * This class holds the parsed document instead: constructing it parses once, the first
 * question needing a knowledge base reverse-maps once, and later questions reuse both.
 * That matters more in a browser than anywhere else — this cost lands on the main
 * thread.
 *
 * The methods answer exactly what the same-named functions answer; they are the same
 * shared boundary session those functions now wrap.
 *
 * The constructor does NOT reason. `profile` answers for any parseable document, so
 * building the knowledge base eagerly would make it inherit every way that can fail;
 * an ontology whose knowledge base cannot be built still constructs and throws on the
 * first question that needs one.
 */
export class Reasoner {
  /**
   * @param document N-Quads (or N-Triples).
   * @param stepCap Narrows the per-decision tableau step cap for every question asked
   *   through this session. 0 means the knowledge base's own cap, NOT a cap of zero
   *   steps, and it can only narrow.
   */
  constructor(document: string, stepCap: number);
  consistency(): ReasoningAnswer;
  classify(): ReasoningAnswer;
  realize(): ReasoningAnswer;
  instances(class_: string): ReasoningAnswer;
  entails(axiom: string): ReasoningAnswer;
  profile(): ReasoningAnswer;
  extractModule(signature: string, method: string): ReasoningAnswer;
  justify(axiom: string): ReasoningAnswer;
  explainConclusion(regime: string, conclusion: string): ReasoningAnswer;
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
  queryRawConfigured(
    dataset: Dataset,
    sparql: string,
    base: string | null | undefined,
    format: "jsonld" | "yamlld" | string,
    optionsJson: string,
  ): string;
  queryRawWithContext(
    dataset: Dataset,
    sparql: string,
    base: string | null | undefined,
    format: "jsonld" | "yamlld" | string,
    context: CompiledJsonLdContext,
    yamlSchemaUrl?: string | null,
  ): string;
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
export function liftProjection(
  archive: Uint8Array,
  profile: LiftProfile,
  configJson: string,
): ProjectionLift;
/**
 * The CLI/C-ABI/Python/JS spellings of the SPARQL entailment regimes. ALL SEVEN
 * materialize through `entailMaterialize`; none is refused for being the regime
 * it is. One of them — `"rif"` — needs an extra INPUT rather than permission,
 * and the `program` argument is where it goes — see `entailMaterialize`.
 */
export type EntailmentRegime =
  | "simple"
  | "rdf"
  | "rdfs"
  | "owl-rl"
  | "owl-direct"
  | "rif"
  | "d";

/**
 * Close `document` (N-Quads, which accepts N-Triples) under `regime`.
 *
 * `program` is the regime's own rule document. `"rif"` entails under the
 * *caller's* rules, which PurRDF does not declare, so its `program` is a
 * normative RIF-in-XML document; every other regime's rule table is the
 * specification's, so its `program` is `""` and a non-empty one throws rather
 * than being silently discarded.
 *
 * `"owl-direct"` takes no program either: its extra input is a *query's* class
 * expressions, and this is a document boundary with no query, so it runs the
 * query-independent tableau augmentation.
 */
export function entailMaterialize(
  document: string,
  regime: EntailmentRegime | string,
  program: string,
): RegimeClosure;
export function entailRules(regime: EntailmentRegime | string): string[];
export function entailImplementedRules(regime: EntailmentRegime | string): string[];
/**
 * The rules this build fires BEYOND `regime`'s specification table.
 *
 * Disjoint from both `entailRules` and `entailImplementedRules`: the normative table
 * is a statement about the specification and does not move because this build fires a
 * sound rule the table happens not to list. `[]` for a lane with nothing added to it.
 */
export function entailExtensions(regime: EntailmentRegime | string): string[];
export function entailCheckGoldenVectors(): void;
export function entailCheckInconsistentRefusal(): void;

/** The `bot`/`top`/`star` locality-module extraction methods `entailExtractModule` accepts. */
export type ModuleExtractionMethod = "bot" | "top" | "star";

/**
 * `entailConsistency(document, stepCap)` → is the knowledge base consistent?
 *
 * `stepCap` narrows the per-decision tableau step cap; `0` means the knowledge
 * base's own cap, not a cap of zero steps. The one DL service that answers for an
 * unsatisfiable ontology, because it is the one that detects one.
 *
 * Throws if `document` fails to parse or the reverse mapping fails.
 */
export function entailConsistency(document: string, stepCap: number): ReasoningAnswer;

/**
 * `entailClassify(document, stepCap)` → the entailed subsumption hierarchy over the
 * ontology's named classes: `equivalent`, `subclass` (the full transitively-closed
 * relation), `direct` (its transitive reduction) and `unsatisfiable` lines.
 *
 * Throws on a malformed document, or on an ontology with no model.
 */
export function entailClassify(document: string, stepCap: number): ReasoningAnswer;

/**
 * `entailRealize(document, stepCap)` → the entailed types of the ontology's named
 * individuals, and the most specific of them (`type` then `direct-type` lines).
 *
 * Throws on a malformed document or an ontology with no model.
 */
export function entailRealize(document: string, stepCap: number): ReasoningAnswer;

/**
 * `entailInstances(document, classTerm, stepCap)` → the named individuals entailed
 * to be instances of `classTerm`, as `instance <term>` lines.
 *
 * `classTerm` is ONE N-Triples term — `"<http://example.org/Cat>"`, angle brackets
 * included. A class the ontology never mentions is not an error: the empty answer
 * for it is a real answer.
 *
 * Throws on a malformed document, a `classTerm` that is not one N-Triples term, or
 * an ontology with no model.
 */
export function entailInstances(
  document: string,
  classTerm: string,
  stepCap: number,
): ReasoningAnswer;

/**
 * `entailEntails(document, axiom, stepCap)` → does the ontology entail `axiom`?
 *
 * `axiom` is ONE triple of the OWL 2 RDF mapping, in N-Triples syntax; seven
 * reserved predicates select the seven named axiom kinds and any other predicate is
 * an object-property assertion. The answer is `entails true|false|unknown` followed
 * by the axiom as it was read.
 *
 * Throws on a malformed document, an `axiom` that is not one triple, or an ontology
 * with no model.
 */
export function entailEntails(
  document: string,
  axiom: string,
  stepCap: number,
): ReasoningAnswer;

/**
 * `entailProfile(document)` → which OWL 2 profiles the ontology is provably in
 * (`certified <profile>` lines, most restrictive first), and what blocked the
 * others. Purely syntactic: no tableau, no closure, no budget.
 *
 * Throws on a malformed document.
 */
export function entailProfile(document: string): ReasoningAnswer;

/**
 * `entailExtractModule(document, signature, method)` → the locality module of the
 * ontology for a seed signature, as canonical (RDFC-1.0) N-Quads.
 *
 * `signature` is one N-Triples term per line (blank lines ignored). `method` is
 * `bot`, `top` or `star`; an unknown spelling throws with the accepted set named.
 */
export function entailExtractModule(
  document: string,
  signature: string,
  method: ModuleExtractionMethod | string,
): ReasoningAnswer;

/**
 * `entailJustify(document, axiom)` → WHY a Description-Logic axiom is entailed: a
 * minimal subset of the ontology (as canonical N-Quads) that still entails it.
 *
 * Throws if the ontology does not entail the axiom, or if the tableau could not
 * decide it.
 */
export function entailJustify(document: string, axiom: string): ReasoningAnswer;

/**
 * `entailExplainConclusion(document, regime, conclusion)` → WHY one triple of a
 * chase closure holds: which rules, from which premises.
 *
 * `conclusion` is ONE N-Quads statement; its graph, if it names one, selects the
 * closure to explain.
 *
 * Throws for `rdf` and `rdfs` (four of whose rules conclude about a fresh blank
 * node with no checkable proof term), or for a conclusion that is neither asserted
 * nor derived.
 */
export function entailExplainConclusion(
  document: string,
  regime: EntailmentRegime | string,
  conclusion: string,
): ReasoningAnswer;

export function shaclEntail(shapesTtl: string, dataNt: string): string;
export function shaclValidateToSarif(shapesTtl: string, dataNt: string): string;
export function version(): string;
