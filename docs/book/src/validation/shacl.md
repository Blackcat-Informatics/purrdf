<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# SHACL

[`purrdf-shapes`](https://docs.rs/purrdf-shapes) (re-exported as
`purrdf::shapes`) is PurRDF's native SHACL validator and rules engine. It
implements **SHACL 1.2**: Core, SPARQL Extensions, Node Expressions, Inference
Rules and the SPARQL 1.2 RL rule language, plus the SHACL-AF 1.0 spellings
those documents supersede. It runs entirely on PurRDF's own interned IR and
native SPARQL engine (no oxigraph, no PyO3).

It validates an RDF 1.2 data graph against a SHACL shapes graph with **no
inference** (parity with pySHACL `inference="none"`); combine with
[Entailment](../entailment.md) if you want to validate a materialized closure,
or declare `sh:entailment sh:RulesEntailment` to run the shapes graph's own
rules first.

## What it covers

- **SHACL 1.2 Core** — every constraint component the SHACL 1.2 vocabulary
  declares is evaluated natively: full property paths, qualified value shapes,
  the list components (`sh:minListLength`, `sh:maxListLength`,
  `sh:uniqueMembers`, `sh:memberShape`, with `sh:detail` results), list-valued
  `sh:class`, `sh:datatype` and `sh:nodeKind` (including `sh:TripleTerm`),
  `sh:singleLine`, `sh:rootClass`, `sh:someValue`, path-valued property pairs
  and `sh:subsetOf`, `sh:uniqueValuesFor`, `sh:closed sh:ByTypes`,
  `sh:reifierShape` and `sh:reificationRequired`, and `sh:uniqueLang` over
  language tag and base direction. A property shape's `sh:values` and
  `sh:defaultValue` compute value nodes. The targets include implicit class
  targets (`sh:ShapeClass` too), `sh:shape` in the data graph, `sh:targetWhere`
  and node-expression `sh:targetNode`. A reifier of a constraint triple can
  carry `sh:deactivated`, `sh:severity` or `sh:message` for that one
  constraint. The severities include `sh:Debug` and `sh:Trace`, and
  conformance follows the request's `sh:conformanceDisallows` set. Every
  `sh:message` is reported with its language tag and direction.
- **SHACL 1.2 SPARQL Extensions** — SPARQL-based constraints and targets,
  custom constraint components with pre-binding semantics, user-defined
  `sh:SPARQLFunction` calls, `sh:SPARQLTargetType`, `sh:sparqlExpr`, and
  `sh:prefixes` resolved as that document specifies (see
  [below](#prefixes-in-shacl-sparql-queries)), all on the native SPARQL engine.
- **SHACL 1.2 Node Expressions** — the `shnex:` vocabulary and the older
  SHACL-AF `sh:` spelling of a node expression parse to one representation and
  run through one evaluator. Each expression kind keeps the order and
  multiplicity its evaluation clause defines: an ordered sequence, a multiset
  or a set. `sh:if` takes `sh:then` only when its condition is the list
  `( true )`. The `shnex-sparql.ttl` function library is native.
  `sh:nodeByExpression` and `sh:ExpressionConstraintComponent` are validated.
- **SHACL 1.2 Inference Rules** — `sh:TripleRule` and `sh:SPARQLRule`, with
  `sh:condition`, `sh:layer`, `sh:order`, `sh:runOnce`, `sh:deactivated`, rule
  sets and `sh:RulesGraph`, SPARQL rule templates, temporary triples,
  `sh:expectedPredicate` and `sh:ruleProcessor`. Layers run in ascending
  order. In each layer the run-once rules run once, then the iterating rules
  repeat until an iteration infers nothing. Within an iteration, rules run in
  ascending `sh:order`, rules of the same order run together, and each group
  sees the inferences of the groups before it, so swapping two rules' orders
  can change the inferences. An unresolvable `sh:condition` is a load error,
  not a rule that silently never fires.
- **SPARQL 1.2 RL** — the rule language's text syntax is parsed, checked for
  well-formedness, stratified, imported and evaluated (`purrdf_shapes::srl`).
  SHACL rules lower to the same rule-set representation, and both run on one
  rules engine, `purrdf-datalog`.

A shapes graph is either loaded faithfully or refused. An unknown term, an
ill-typed parameter value, or a construct the engine does not evaluate is a
load error that names it, never a constraint that silently drops out. The
same check covers the nodes of SHACL-SPARQL: a SPARQL-based constraint, a
validator and a `sh:SPARQLTarget` may carry only the terms their
specification gives them, so an `sh:ask` beside a constraint's `sh:select`,
or a misspelled `sh:mesage`, is refused rather than ignored.

The terms the SHACL vocabularies define and the engine refuses by name are:

- the SHACL JavaScript Extensions (`sh:js`, `sh:JSConstraint` and the rest),
  which are not part of SHACL 1.2; this engine has no JavaScript engine to
  evaluate them;
- `sh:describe` and `sh:update` on a shape, a node expression, a SPARQL-based
  constraint, a validator or a rule. The SHACL 1.2 vocabulary declares them as
  the queries of `sh:SPARQLDescribeExecutable` and `sh:SPARQLUpdateExecutable`,
  but no SHACL specification executes either class, so in those positions they
  would otherwise be silently ignored. A resource that is only such an executable, and
  that no shape reads, loads.

Two terms the engine used to refuse are now evaluated:

- **SHACL-SPARQL result annotations.** A `sh:resultAnnotation` on a
  SPARQL-based constraint or on a validator of a SPARQL-based constraint
  component adds its `sh:annotationProperty` to every result the query
  produces. The value is the solution's binding of `sh:annotationVarName`, or
  of the property's local name when no name is given. When that variable is
  unbound, the `sh:annotationValue` defaults are used. An ASK validator's
  annotations read `this`, `value` and the component's parameters. The
  annotations appear on `ValidationResult::annotations`, in the report graph,
  in the SARIF property `shaclResultAnnotations`, in prepared products and in
  the Python result dicts. A variable name that no SPARQL query could bind is
  refused at load. So is a SHACL report property such as `sh:focusNode` used
  as the annotation property.
- **`sh:minus`.** The SHACL Advanced Features 1.1 minus expression,
  `[ sh:nodes N ; sh:minus M ]`, evaluates exactly as SHACL 1.2's
  `[ shnex:nodes N ; shnex:remove M ]`: the nodes of N not in M, in N's order.
  Its `sh:nodes` is required.

Every IRI the engine implements is defined by a W3C document; PurRDF mints
none.

The W3C SHACL 1.0 `data-shapes` suite passes as approved (128/129, zero ledgered
gaps at the time of writing; the other case is refused for an import no
document can be supplied for), and so does the approved W3C SHACL 1.2 suite
apart from its upstream errata and that same refused case (537/544); the live
numbers are in
[`docs/CONFORMANCE.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/CONFORMANCE.md),
and [SHACL 1.2 conformance](#shacl-12-conformance) below says how to
reproduce them.

A parsed shapes graph can also be compiled once and written out as a
deterministic, authenticated byte artifact, so a later process restores a
prepared validator instead of re-parsing Turtle — see
[Prepared Shapes Products](shapes-products.md).

## `owl:imports` in a shapes graph

A shapes document may carry an `owl:Ontology` header that `owl:imports` other
documents, and the shapes it constrains with may live entirely in those imports.
PurRDF resolves that closure — **but it never fetches it**. There is no HTTP
client in the workspace, every release crate builds for
`wasm32-unknown-unknown`, and a validation verdict that depends on what a URL
served today is not reproducible. So the closure is caller-supplied
configuration, exactly as it is for `entails` and `shex`:

```bash
purrdf validate --shapes root.ttl \
  --import https://example.org/shapes-a=a.ttl \
  --import https://example.org/shapes-b=b.ttl \
  data.ttl -
```

The table is followed **transitively** — an imported document's own
`owl:imports` are resolved from the same table — and a cycle terminates rather
than looping, because OWL 2 §3.4 defines the imports closure as the transitive
one and explicitly permits `A` to import `B` to import `A`. Each imported
document's own `@prefix` declarations travel with it, so a SHACL-AF `sh:select`
written in an imported file resolves against the prefixes that file declares.

The shapes document parses under its own `file://` retrieval IRI, or under
`--shapes-base IRI` when given. That flag is the same one `shacl pack --base`
is, and it is separate from `validate --base`, which sets only the DATA
graph's base. An `@base` inside the shapes document still wins inside it, as
Turtle specifies.

An import whose document or ontology is **already loaded** needs no pair. It
is resolved when it names a document that was read: the shapes document's
own base (its `file://` retrieval IRI, `--shapes-base`, or `shacl pack
--base`), an in-document `@base`, or an `--import` document's IRI. An import
is also resolved when the graph holds `<X> a owl:Ontology`, or an ontology
whose `owl:versionIRI` is `<X>`.

An import is also resolved when the shapes graph describes its target with
`sh:declare`. This is SHACL's prefix-declaration idiom: a SHACL-SPARQL query
collects its prefixes along `sh:prefixes/owl:imports*/sh:declare` within the
shapes graph, so the target of such an `owl:imports` is a node the shapes graph
declares prefixes on, not a document to fetch. The W3C test suite writes
exactly this, and it validates as written:

```turtle
<http://example.com/ns#> sh:declare [ sh:prefix "ex" ; sh:namespace "http://example.com/ns#"^^xsd:anyURI ] .
ex:TestPrefixes owl:imports <http://example.com/ns#> ;
  sh:declare [ sh:prefix "test" ; sh:namespace "http://test.com/ns#"^^xsd:anyURI ] .
```

Only `sh:declare` counts. A target the shapes graph describes some other way —
only by an `rdfs:label`, say — is still an unresolved import. Every one of
these rules applies across the whole closure, so a declaration that arrives in
an imported document counts too. A
shapes document that merges the W3C SHACL 1.2 vocabularies — `shnex.ttl`
imports `sh:`, and `shacl.ttl` beside it declares `sh:` — is complete as
written.

Every **other** `owl:imports` no pair resolves is refused (exit 1). The refusal
names each missing IRI and the `--import IRI=FILE` pair that resolves it, and
suggests `--shapes-base IRI` for a document that imports its own published
IRI and is read here from a local file.
Validating without the imported document would be a verdict about a smaller
shapes graph than the one named, so there is no warn-and-continue path. A pair
the closure never reaches is refused as unused (exit 2) rather than read and
ignored.

### The same rule on every host

The command line is one caller of a rule that lives in the engine. Every way a
shapes graph becomes a validator — parsing it, validating with it, running its
rules, evaluating one of its node expressions, linting it, packing it into a
product — resolves the closure through one helper,
`purrdf_shapes::imports::resolve_shapes_imports`, over the rule in
`purrdf_core::imports` that entailment applies too. The same shapes graph
therefore gets the same verdict on every host, and each host takes the import
table in its own spelling:

| Host | Import table | Refusal |
|---|---|---|
| Rust | a `ShapesImports` (`from_turtle`, `insert`, `declare_loaded`) passed to `parse_shapes_with_config`, `from_dataset_with_base`, `validate_graphs_with_options`, `lint::lint` or `FreeExpression`; the `purrdf-validate` boundary takes `(IRI, Turtle)` pairs | `ShapesError::Imports(ShapesImportError::Unresolved \| Unreached \| InvalidEntry)` |
| CLI | `--import IRI=FILE`, repeatable | exit 1 for an unresolved import; exit 2 for a pair nothing imports or a malformed pair |
| Python | `imports=[(iri, turtle), ...]` on `validate`, `entail`, `apply_rules`, `eval_node_expr`, `lint_shapes`, `pack_product` and `Shapes(...)` | `purrdf.shapes.ShapesImportError` (a `ValueError`) with `.kind` and `.iris` |
| WebAssembly | trailing `importIris`, `importDocuments` arrays on every `shacl*` function that takes a shapes graph | `ShaclImportError` with `kind`, `iris` and `message` |
| C ABI | `import_iris`, `import_documents`, `import_count` before the out-parameters | `PURRDF_STATUS_SHAPES_IMPORT_ERROR`, read with `purrdf_shapes_import_error_kind`, `_iri_count` and `_iri` |

Each imported document is Turtle, parsed with its ontology IRI as its base. The
kind is `unresolved-import`, `unreached-import` (a table entry no import names —
refused on every host, because its shapes would be read and never applied) or
`invalid-import` (a key that is not an absolute IRI, a key named twice, or a
document that does not parse). Branch on the kind, not on the message.

```python
import purrdf

report = purrdf.shapes.validate(
    shapes_ttl, data_nt, imports=[("https://example.org/lib", lib_ttl)]
)
```

A lint certifies the whole closure: an imported document's shapes are loaded
and checked against `shacl-shacl.ttl` like the importing document's, and a
shapes graph whose imports are not in hand is refused rather than reported
clean.

## Prefixes in SHACL-SPARQL queries

A SPARQL query in a shapes graph (`sh:select`, `sh:ask`, `sh:construct`,
`sh:sparqlExpr`, a validator, a target or a function body) gets a `PREFIX`
header before it is compiled. The header follows SHACL 1.2 SPARQL Extensions,
"Prefix Declarations for SPARQL Queries":

- **A query with `sh:prefixes`** uses the prefix declarations on the path
  `sh:prefixes/(^owl:versionIRI?/owl:imports)*/sh:declare`. The path is read
  from the query node and from the shape or component that carries it.
- **A query without `sh:prefixes`** uses every `sh:declare` of every SHACL
  instance of `owl:Ontology`, `sh:DataGraph`, `sh:ShapesGraph` or
  `sh:RulesGraph` in the shapes graph. A subclass of one of those classes
  counts too.
- **Two different namespaces for one prefix** in the declarations a query
  reaches make the shapes graph ill-formed. The load fails, and the error names
  the prefix and both namespaces. The same prefix declared twice with the same
  namespace is fine. Declarations that no query reaches are not checked.
- **A malformed declaration** that a query reaches also fails the load. A
  declaration needs exactly one `xsd:string` `sh:prefix` and exactly one
  `sh:namespace`.

The shapes document's own `@prefix` / `PREFIX` directives are a fallback. They
only supply a prefix the declarations above leave unbound, and they never
replace one the declarations bind. They come from the Turtle parser's own
record of the document, so a `PREFIX` line inside a query's string literal is
never mistaken for a directive. If the document declares a prefix twice, the
fallback uses the last declaration. A `PREFIX` in a query's own text applies to
that query only.

## Built-in declarations and the W3C vocabularies

The SHACL 1.2 vocabularies (`shacl.ttl`, `shnex.ttl` and `shnex-sparql.ttl`)
declare every built-in constraint component and node-expression function, for
example `sh:SPARQLExprExpression a sh:NamedParameterExpressionFunction`. None
of those declarations has a body or a validator, because the engine provides
the implementation. A shapes graph may merge the vocabularies, and loading
resolves each declaration against the engine's table of what it implements:

- a bare declaration of a built-in binds to the native implementation and adds
  nothing to the custom-function index or the component registry;
- a built-in declared again with a body is a duplicate definition, and so is a
  built-in component given an `sh:ask` or `sh:select` of its own; one declared
  under the wrong class or with a contradicting signature is a mismatch; all of
  these fail the load;
- validators declared for a built-in component are alternatives the native
  implementation supersedes. SHACL 1.2 SPARQL Extensions selects "one of the
  values" of a component's validators, so each is an implementation of the same
  component, and the engine's own is the one that runs. Vocabularies such as
  DASH declare them for SHACL Core components. An alternative must be a
  well-formed SPARQL validator of its attachment: a SHACL-JS `sh:JSValidator`,
  an ASK validator under `sh:propertyValidator` or an unparsable query fails the
  load. It is never executed, so its query may call a function the engine does
  not have;
- any other `sh:` statement on a built-in's declaration fails the load, except
  `sh:message`, `sh:labelTemplate` and the non-validating characteristics
  (`sh:name`, `sh:description`, …): `sh:severity` on
  `sh:MinCountConstraintComponent`, for example, would ask the native
  implementation for something it does not do;
- a function the engine does not implement still needs its body, whatever its
  namespace: a bodiless `ex:f` fails the load, and so does a bodiless
  `sh:NotABuiltin`;
- a custom function whose key parameter is a built-in's key parameter fails the
  load, because the key parameters of all node expression functions must be
  disjoint.

So merging the W3C vocabularies into a shapes graph is a no-op. A test holds
this over every shapes graph in three corpora (the first-party corpus, the W3C
SHACL 1.0 suite and the SHACL 1.2 `sht:Validate` entries): the validation
report is byte-identical with and without the merge. `purrdf_shapes::spec`
exposes what the vocabularies declare and what the engine implements, and a
test pins the difference at zero. `purrdf shapes lint` reports, per function
call site, whether the call bound natively, to a custom body, to a SPARQL
registration or to a host extension, and lists every validator declared for a
built-in component as `superseded-by-native`. Those lines are never findings.

## SHACL 1.2 conformance

The complete W3C `shacl12-test-suite` is vendored byte-exact under
`vectors/shacl12/` and run through the library API: 547 tests, covering 174
`sht:Validate`, 143 `sht:EvalNodeExpr` and 27 `sht:Infer` tests, and 203
SPARQL 1.2 RL syntax, well-formedness, stratification and evaluation tests. An
upstream manifest lists 544 of them. Of those, 537 pass as approved, 6 are
upstream errata and 1 is refused for an unresolvable import, all described
below, and the expected-failure ledger is empty. The other 3 are entries of
vendored files that no manifest includes, and they are reported apart. A
negative SPARQL 1.2 RL test must fail at the stage its test type names, not at
an earlier one.
Reproduce it with:

```bash
cargo test -p purrdf-shapes --test w3c12_conformance -- --nocapture
```

The last line of the approved suite's scoreboard is
`W3C12 TOTAL: passed 537, upstream-errata 6, refused-unresolvable-import 1, xfailed 0, ledger 0`,
and the unlisted files' own line is
`W3C12 UNLISTED: passed 3, exact 2, with-delta 1, total 3`.

Three vendored files carry test entries that no upstream manifest includes:
`core/node/xone-002.ttl`, `core/node/xone-003.ttl` and
`inference-rules/rdfs/rdfs1.ttl`. They are not part of the approved suite, so
they are never counted among its passes. A separate test,
`w3c_shacl12_unlisted_vendored_files`, grades them and reports them under their
own label. Two are graded exactly as written. `core/node/xone-003` expects a
result from a property shape with no `sh:resultPath`, but section 6.7.2.2 says:
"For results produced by a property shape, this SHACL property path is
equivalent to the value of sh:path of the shape, unless stated otherwise." The
engine keeps the shape's `sh:path`. The harness grades the file's report with
exactly that one result amended, quotes the clause beside the amendment, and
proves that a report without the amendment, or with a different one, fails.

A `sh:reifierShape` or `sh:reificationRequired` result carries the value node
as `sh:value`. The textual definition in SHACL 1.2 Core section 7.8.5 uses the
name `t` for two things. It opens "Let t be the triple term (focus node, $path,
value node)", then reports "For each reifier t that does not conform to
$reifierShape, there is a validation result with t as sh:value" and, for a
missing reification, "there is a validation result with t as sh:value". The
approved tests `core/property/reifierShape-001` and `-002` both expect the value
node, and PurRDF follows the approved test suite's reading. A result for a
non-conforming reifier also carries the reifier's own validation results as
`sh:detail`, each with the reifier as its focus node, so the report still names
the reifier and says why it fails. A missing reification has no reifier and
carries no details.

Six node-expression tests expect an integer-valued `xsd:decimal` in a lexical
form that is not canonical, such as `"4.0"` or `"00"`. XSD 1.1 Part 2 (section
3.3.3.1 and the `decimalCanonicalMap` of E.1) makes the canonical form `"4"`
and `"0"`, which is also what the approved W3C SPARQL tests `ceil01`,
`floor01`, `round01` and `seconds` expect. PurRDF emits canonical forms. These
six are upstream errata, not passes: the harness reports them under their own
label, and grades each one exactly against the canonical form, not by
comparing values.

`sparql/component/validator-001`, in this suite and in the SHACL 1.0 suite,
imports DASH (`<http://datashapes.org/dash>`). No document can be supplied for
it. DASH is not well-formed SHACL: some of its `sh:validator` values are
`sh:JSValidator`s, but SHACL 1.2 SPARQL Extensions section 4.2.3 says "The
values of sh:validator must be ASK-based validators". Its `dash:uriTemplate`
also declares a parameter named `value`, which section 4.2.1 forbids. Its
shapes would also change the verdict. PurRDF fetches nothing and refuses an
unresolved import. So the case is graded as an exact expected refusal: loading
must fail with `ShapesImportError::Unresolved` naming exactly that IRI, and a
load that succeeds is a failure. The case is reported as "refused: unresolvable
import", never as a pass.

SPARQL 1.2 RL grammar rule [2],
`RuleOrDataBlock ::= Prologue ( RuleOrData+ ( Prologue1 RuleOrData? )* )?`, is
implemented as written: once a declaration follows a rule or data block, at
most one rule or data block may follow it before the next declaration.

## JSON-LD instance projection and JSON Schema

`json_schema::compile` and `instance::project_graph` are one pair. The schema
describes the JSON-LD `@graph` document the projection writes, and a projected
node validates exactly when it conforms to the shapes, except for the recorded
losses listed below. The projection drops no triple. Every carried term keeps
what a constraint can judge:

- An `@id` is the full IRI, or `_:label` for a blank node. It is never a compact
  IRI, so a lexical constraint judges `str()` of an IRI directly. Keys and
  `@type` values stay compacted.
- A well-formed RDF list is the JSON-LD list object `{"@list": [...]}`, and its
  cells are not `@graph` nodes. `rdf:nil` is `{"@list": []}`. The list
  conversion of JSON-LD 1.1 Processing Algorithms and API section 8.4.2 decides
  which lists convert, read strictly so that no triple is dropped. A branching,
  cyclic, shared, IRI-named or annotated list (a cell typed `rdf:List`
  included) keeps the node-graph form. A list that is a member of another list
  carries its head cell's label as `@index`. `@index` is not RDF-significant,
  and it keeps two distinct member lists distinct.
- A bare JSON scalar appears only where it denotes the literal exactly
  (JSON-LD 1.1 section 8.6): a canonical `xsd:integer` within 64 bits, and
  `xsd:boolean` `true` or `false`. Every other numeric literal keeps its
  `{"@value", "@type"}` object.
- An `rdf:dirLangString` literal carries `@direction`.

The schema projects the SHACL 1.2 list components onto the `@list` array.
`sh:minListLength` becomes `minItems`, `sh:maxListLength` becomes `maxItems`,
`sh:uniqueMembers true` becomes `uniqueItems`, and `sh:memberShape` becomes
`items`, compiled as one value's schema. Each of them requires a list. On a
node shape they judge the focus node's `rdf:first` and `rdf:rest`. Every
`sh:flags` letter, `i` included, is written into the ECMA-262 pattern. `i`
follows XPath's case variants, not simple case folding. Lexical constraints
judge an IRI's `@id`. They judge the constants `rdf:nil`, `true` and `false` at
compile time, and a bare integer's numeral length as an integer range.
Numeric datatypes are told apart over their lexical and value spaces. A range
bound compares a typed integer-family or decimal literal by an order pattern on
its lexical form, with SPARQL's numeric promotion against a double or float
bound. A temporal bound on `xsd:dateTime`, `xsd:date` or `xsd:time` compares a
typed literal of its datatype on the XSD timeline, by order patterns on the
lexical form. A timezone makes the value an instant, and `24:00:00` is the next
day's midnight. A zoned value and a local bound, or the reverse, compare only
when they are more than 14 hours apart. Closer than that they are incomparable,
which the component reports as a violation. A value of any other datatype
violates the bound. A dateTime or time bound's pattern states each of the
1,681 minutes within ±14:00 of it, so it is large: about 100 KB, and more
for a bound with fractional seconds. A node shape's
node kind, `sh:in`, `sh:hasValue` and lexical constraints judge the focus
node's `@id`.

What remains is recorded on the forward ledger, each case with its reason:

- A list kept as linked `@graph` nodes is not judged. A JSON Schema keyword
  judges only the instance location it applies to and the locations beneath
  it, so none reaches another node's `rdf:first`.
- A node shape's `sh:uniqueMembers` does not compare the focus node's own
  member with its tail's members. Those are two instance locations.
- A pattern over a bare integer's numeral is not judged. The integers whose
  numerals start with `1` are no finite union of the intervals and residue
  classes `minimum`, `maximum` and `multipleOf` state.
- A range bound over `xsd:double` or `xsd:float` lexical forms is not judged.
  Those forms on one side of a bound are no regular language.
- A node shape's class membership is not judged. It runs through
  `rdfs:subClassOf*` triples on other nodes.

The Pydantic package enforces the list components, uniqueness included, with a
validator over the raw JSON items. TypeScript states the length bounds exactly
at any size, and distinct elements when the members are finitely many scalars.
It has no type for distinct elements of an unconstrained member, or for an
integer minimum. LinkML states all four on
the `@list` slot. GraphQL carries a list value as a `@oneOf` input object of a
node reference and a list object, and each member as a `@oneOf` input object of
the member shape's alternatives, so member types are checked. GraphQL list types
have no length or uniqueness constraint and its numeric types have no bound, so
those are recorded where they stand. Each emitter oracle runs these components
over projected instances of real data.

Each emitter oracle also runs the temporal bounds over projected instances.
Pydantic and LinkML agree with validation on each of them. TypeScript and
GraphQL record the bound's negation where it stands. Neither has a complement
type. TypeScript cannot write the admitted lexical forms positively either,
because the dates of four-digit years alone exceed its union limit.

## Ontology-complete developer schemas

The public `compile_schema` boundary accepts a `SchemaCompileRequest` that binds
the parsed shapes, exact ontology dataset, caller-owned `Namespaces`, and an
explicit `SchemaSurfaceMode`. `ShapedOnly` retains the active SHACL
target-class surface. `OntologyComplete` adds existing caller-vocabulary
classes and optional OWL/RDFS-derived properties. The result carries JSON
Schema draft 2020-12, OpenAPI 3.1, the normal forward loss ledger, a canonical
property-coverage report, and a deterministic pre-compilation cache key. Its
`CompiledSchema` feeds the LinkML, TypeScript, GraphQL, and Pydantic emitters
without a second schema-discovery pass.

The bounded theory catalogs only schema evidence: direct IRI `sh:path`, RDF/OWL
property declarations, domain/range declarations, and both endpoints of
subproperty, equivalent-property, and inverse-property relations. A predicate
seen only on an instance is not promoted. Class admission is likewise explicit,
and synthesized definitions are limited to namespaces the caller supplied;
PurRDF does not turn builtin compaction prefixes into an ontology boundary.

Subclass/equivalent-class closure determines domain membership. Multiple
domains are conjunctive; OWL union members are alternatives and intersection
members are conjunctive. Subproperties inherit superproperty domains, ranges,
and forward functionality, equivalent properties propagate bidirectionally,
and inverse properties exchange domain and range. Strongly connected cycles
are condensed deterministically. Multiple ranges remain conjunctive in emitted
JSON Schema; union and intersection expressions map to `anyOf` and `allOf`.

Direct SHACL remains authoritative. Ontology-only fields are optional;
`owl:FunctionalProperty` gives a scalar representation with approximation
provenance, while inverse functionality does not. Closed shapes reject
unshaped fields unless they are directly present or ignored. Classes without a
target shape are emitted as open carriers, never as fabricated closed models.
This is not ABox materialization or unrestricted OWL: property chains and
axioms outside the fragment do not create fields.

`SchemaCoverageReport` accounts for every catalogued property once, including
exclusions, with sorted per-class outcomes and source-axiom provenance.
`SchemaCompileRequest::coverage_report` can produce it before emission. The
request cache key binds RDFC-1.0 identities for the shapes and ontology graphs,
caller namespaces, mode, value-vocabulary marker, compiler/policy salts, and
the fixed ceilings: 65,536 properties, 65,536 classes, 1,048,576 relation or
coverage cells, and expression depth 64. Malformed OWL lists, contradictory
property kinds/ranges, key collisions, and limit exhaustion are typed failures.

Run the complete two-mode and four-emitter example with:

```bash
cargo run -p purrdf-shapes --example ontology_schema_surface --locked
```

## Schema → SHACL imports

The schema-projection surface is bidirectional. `SchemaImportConfig` requires
the caller's namespace table and the complete RDF datatype mapping for JSON
scalars; there is no default vocabulary. The five production reverse directions
are JSON Schema draft 2020-12 (`import_json_schema`), native LinkML 1.11
(`import_linkml`), and verified PurRDF-emitted Pydantic v2, TypeScript 7.0, and
GraphQL September 2025 packages (`import_*_package`). All five lower through one
ordered JSON-Schema semantic model and return typed shapes plus an
always-computed, located reverse `LossLedger`.

A range bound reads back as itself. An order pattern states the set of lexical
forms on one side of a bound, not the bound, and different bounds can state the
same set: `< 150` and `≤ 149` admit the same integers. So each rejection the
compiler writes from a bound carries that bound in its `$comment`, as the
facet's SHACL term and the bound as an N-Triples term, such as `sh:minInclusive
"2020-01-01"^^<http://www.w3.org/2001/XMLSchema#date>`. The importer trusts a
comment only after checking it. It regenerates the rejections the named bound
projects to, narrowed by the value's `sh:datatype`, and they must be exactly
the ones that carry the comment. For a numeric bound, the `minimum` and
`maximum` beside it must also be the integers the bound admits. A comment that
fails either check is recorded as `schema-applicator-dropped`. So
`sh:maxExclusive 150`, `sh:minInclusive 1.5` and a bound over `xsd:date`,
`xsd:dateTime` or `xsd:time` survive the round trip, and the re-emitted schema
is byte-identical.

Malformed values, open or dangling references, identity collisions, generated
artifact/map drift, and resource-limit exhaustion fail closed. Valid source
constructs without an exact SHACL interpretation are ledgered at their native
JSON Pointer. Arbitrary Python, TypeScript, and GraphQL SDL are intentionally
outside the inverse boundary because none defines one unique runtime JSON
acceptance relation. LinkML does have a native reader; its schema identity and
documentation can therefore appear as losses even when the validation-bearing
SHACL recompiles byte-exactly.

The executable example constructs caller-owned `example.org` configuration and
exercises all five paths:

```bash
cargo run -p purrdf-shapes --example schema_reverse --locked
```

## Pydantic v2 projection

`purrdf-shapes` can transliterate a compiled SHACL-derived JSON Schema into a
deterministic, typed Pydantic v2 package entirely in memory. The public
`emit_pydantic` function consumes `CompiledSchema`; `PydanticConfig` requires the
caller to supply the package name and package/module prose, so the library does
not invent a vocabulary, namespace, or downstream brand.

Every `$defs` entry gets a stable import path, JSON property names remain exact
through Pydantic aliases, and generated classes expose the originating
definition through `model_json_schema(by_alias=True)`. Pydantic runtime
annotations enforce the representable portion. A JSON Schema assertion with no
exact runtime annotation remains visible on that schema surface and produces a
located entry in the always-computed `json-schema` → `pydantic-v2`
`LossLedger`; a lossless input yields an empty ledger. The renderer itself has no
Python dependency and stays wasm-clean. A dev-only Python oracle executes the
generated code and checks the live reverse/schema surface.
`import_pydantic_package` separately verifies the retained source schema,
generated files, model map, dialect, and forward ledger before importing SHACL.

A JSON Schema `not` has no annotation equivalent. The package checks it at run
time instead: a before-validator evaluates the negated schema over the raw JSON
input. The check follows JSON Schema's own semantics. Numbers compare by exact
value, string lengths count code points, and `pattern` runs through Pydantic's
own regex engine. The check covers a closed keyword table, `$ref` included. A
negated schema that uses any other keyword, such as `propertyNames`, a format,
or a pattern outside the common grammar, stays a recorded loss.

The optional caller-owned `PydanticPackageTopology` is a total partition of
`$defs` entries into portable dotted leaf modules. Each route carries the class
docstring and a sorted, vocabulary-neutral `json_schema_extra` map suitable for
documentation URLs, content digests, and other caller-defined linkage. An
optional `PydanticVersionStamp` adds an exact PEP 440 `__version__` export.
Routed packages share schema/runtime support modules, generate intermediate
package initializers, use explicit symbol tables for one root-level rebuild,
and pass the executable runtime oracle plus strict mypy. Exact route coverage,
portable path/symbol uniqueness, and fixed input/config/output limits all fail
closed. When both topology and version stamping are omitted, the original flat
package bytes remain unchanged. A flat version stamp adds `__about__.py` and
updates the `__init__.py` exports.

## LinkML 1.11 projection

The same `CompiledSchema` carrier can be projected to canonical LinkML 1.11
with `emit_linkml`. `LinkmlConfig` requires the caller's schema IRI, name,
description, default prefix, and complete prefix map, so PurRDF never mints a
consumer vocabulary or identity. The returned `LinkmlPackage` includes the
typed document, deterministic YAML, a reversible `$defs`-key mapping, and a
located `json-schema` → `linkml-1.11` loss ledger. It also carries ordered,
integrity-checked slot rename and skip-diagnostic reports.

Classes and exact property aliases, types, enums, local references, inline
objects, requiredness, homogeneous arrays, patterns, inclusive bounds, and
LinkML boolean expressions are represented directly. An unsafe LinkML slot name
uses `SanitizePolicy::Rename` by default; `Skip` omits only that slot with a
located diagnostic/loss, and `Fail` returns a contextual error. Rename preserves
declared CURIE and absolute-IRI identity byte-exactly in `slot_uri`; an unsafe
bare token or exact caller re-home receives a reported identity under the
caller-supplied default prefix. All valid IRI schemes remain absolute unless
the caller marks that exact token, so custom schemes are not guessed from their
spelling. Safe names reserve first and hash-plus-ordinal collision allocation is
bounded and deterministic.

Every unsupported assertion is classified by a closed capability table;
malformed inputs, external/dynamic/dangling references, stale re-home hints,
semantic identity collisions, and fixed-limit breaches fail closed.
`parse_linkml` and `write_linkml` preserve all
JSON-compatible metamodel fields and provide byte-stable read/write round trips
while rejecting YAML-only tags, duplicate keys, non-string keys, and non-finite
numbers.

`import_linkml` consumes that validated native document; the emitted-package
variant `import_linkml_package` first verifies canonical YAML and the reversible
element map, slot reports, policy losses, aliases, and emitted identities. Both
use the same caller-owned SHACL import configuration. Migration adapters should
pass `CompiledSchema` unchanged, configure exact re-homes, and consume
`slot_renames`; rewriting shared property/required keys is unnecessary.

The Rust production path has no LinkML-toolkit dependency. CI uses the locked
official LinkML 1.11.1 Python packages only as a differential oracle. It loads
safe, lossy, and renamed fixtures through `SchemaDefinition` and `SchemaView`,
regenerates JSON Schema, and verifies reverse predicates:

```bash
make linkml-oracle
```

## TypeScript 7.0 projection

`emit_typescript` projects the same `CompiledSchema` into deterministic
TypeScript 7.0 declarations. The caller supplies the package name and all
package/module prose through `TypeScriptConfig`. The returned package contains
one `index.d.ts`, a reversible `$defs`-key to exported-type map, and a located
`json-schema` → `typescript-7.0` loss ledger; PurRDF invents no consumer
identity or vocabulary.

The fixed declaration dialect uses `strict` plus
`exactOptionalPropertyTypes`. Type aliases preserve JSON primitives and
literals, required versus optional fields, explicit `null`, local recursive
references, unions, intersections, arrays and tuples. There are no runtime
enums, mergeable interfaces, branded pseudo-validators, or `any` escape
hatches. Invalid keywords, open/dangling references, and name collisions fail
before bytes are emitted.

Array length bounds are exact at any size. A tuple element states each
position-specific item. A length beyond those positions is stated as an element
property: an array literal whose contextual type is tuple-like is typed as a
tuple, and a tuple of length `n` has exactly the properties `"0"` to `"n-1"`.
So `minItems: m` is a required property `"m-1"`, and `maxItems: n` is an
optional `never` property `"n"`. `uniqueItems` over items that admit finitely
many JSON scalars is exact too. The generated `JsonDistinct` helper enumerates
the distinct sequences as a union of tuple types. The enumeration stops at the
compiler's limits: an instantiation depth of 100 (TS2589), and 100,000 members
in a union that the compiler spreads into a tuple or distributes an
intersection over (TS2590). Numeric keywords over a finite `const` or `enum`
leave out the numbers that fail them.

Runtime assertions outside TypeScript structural assignability are never
silently erased: integer, numeric/string predicate, closure, pattern-property,
dependency, conditional, negation, contains, uniqueness and evaluation-state
gaps receive stable codes and JSON Pointer locations. A numeric bound over
infinitely many numbers and distinct elements of an infinite item type have no
type. The only proper subtypes of `number` are unions of literals, and a tuple
type constrains each position independently. The oracle compiles the facts
these losses rest on: `Exclude<number, -1>` still admits `-1`, and each
enumeration limit is accepted at its edge and refused one step past it. CI
classifies instances independently with a draft 2020-12 validator and compiles
the generated declarations with the locked TypeScript 7.0.2 compiler, including
fresh-literal and through-variable probes:

```bash
make typescript-oracle
```

The projection intentionally has no arbitrary TypeScript reader. TypeScript
declarations do not define a unique runtime JSON acceptance relation, and the
projection is many-to-one. `import_typescript_package` is the authoritative
reverse surface: it deterministically verifies the retained source schema,
declaration, reversible name map, dialect, and forward ledger. TypeScript is
only a dev-time oracle dependency; the Rust emitter/importer is filesystem-free
and wasm-clean.

## GraphQL September 2025 projection

`emit_graphql` projects `CompiledSchema` into deterministic GraphQL September
2025 SDL. `GraphqlConfig` has no defaults: the caller supplies the schema name,
package and module prose, and a non-built-in fallback-scalar name. The returned
`GraphqlPackage` contains `schema.graphql`, canonical `name-map.json`, the same
name map as typed Rust data, a located `json-schema` →
`graphql-september-2025` loss ledger, and the production value codec.

The SDL is deliberately a type-system fragment. PurRDF emits paired output
`type` and input `input` objects, but no query, mutation, or subscription root,
resolver, pagination rule, authorization policy, federation directive, or
other application behavior. A caller composes the fragment with its own
executable schema.

The exact grammar includes GraphQL booleans, strings, numbers, the signed
32-bit `Int` domain, explicit nullability, finite JSON `const`/`enum` sets,
closed object fields, requiredness, homogeneous lists, direct local `$defs`
references and aliases, descriptions, and inline object helpers. An `anyOf` is
exact when every value selects one alternative: by its JSON kind, or, among
object alternatives, by a required key no other alternative declares. It
becomes a `@oneOf` input object with one field per alternative, and an output
union. An alternative that is not an object type is a union member through a
wrapper type whose `value` field carries it. One global
collision-checked namespace covers types, helpers, and the fallback scalar;
fields and enum symbols are checked in their GraphQL-local namespaces. The
typed/canonical name maps retain the source definition keys, property keys, and
finite JSON values.

`GraphqlPackage::encode_input` maps source JSON keys and finite values to input
field names and enum symbols, and wraps a union value in its alternative's
`@oneOf` field. A non-object value of a kind no alternative admits passes
unchanged, and GraphQL coercion rejects it. `decode_input` is the inverse for
a coerced argument. `encode_output` writes the value a resolver returns, where a
union value names its member in `__typename`. `decode_output` performs the
inverse for fields present in a GraphQL response, without inventing omitted
selections. Unknown or incompatible values fail. This package codec is the precise value boundary;
`import_graphql_package` is the schema reverse boundary and verifies the SDL,
typed/canonical maps, identity, retained source schema, and forward ledger.
Arbitrary GraphQL SDL has no unique JSON Schema acceptance relation and is not
accepted as an inverse format.

GraphQL variable coercion differs from JSON Schema validation at these closed
boundaries:

| Boundary | Located loss families |
|---|---|
| object fields and names | additional properties, pattern properties, property names/counts |
| requiredness and recursion | nullable-presence widening, one deterministic recursive-input nullability relaxation |
| lists | singleton coercion, cardinality, contains, uniqueness, tuples, unevaluated items |
| scalar assertions | integer domain delegation, numeric predicates, string predicates |
| applicators | conditionals, dependencies, intersections, overlapping and type-array unions, `oneOf`, negation |
| runtime boundary | custom-scalar and unknown-keyword validation delegation |

The caller-named fallback scalar is declared but PurRDF does not invent its
`parseValue`, `parseLiteral`, or serialization semantics. Every delegated use
is therefore ledgered. Loss entries carry stable codes and source JSON Pointer
locations; an exact package has an empty ledger.

Emission fails before returning bytes for invalid caller configuration,
malformed schema keywords, `$id` rebasing, external/indirect/dangling `$ref`,
`$dynamicRef`/`$recursiveRef`, alias cycles, unsatisfiable closed required
fields, and generated-name collisions. The fixed limits are 16 MiB for the
input schema, each artifact, and one codec value; 65,536 definitions, fields
per object, or finite values; depth 128; and 255 bytes per GraphQL name.

The independent dev oracle classifies source values with `boon`, builds the
SDL with locked official GraphQL.js 16.14.0, and executes real variable
coercion. It verifies exact agreement, every closed loss family and location,
the name map and production codec, and deliberate corruption failures. Each
valid value is also returned through its output type, and GraphQL.js must
serialize it unchanged:

```bash
make graphql-oracle
```

GraphQL.js is dev-only. Emission and value translation remain filesystem-free,
wasm-clean Rust.

## Rules, node expressions and certifying a shapes graph

Three tools sit beside validation. Every host reaches the same library entry
point, so the command line, Python, WebAssembly and C cannot disagree about the
answer.

| Tool | Rust | CLI | Python (`purrdf.shapes`) | WebAssembly | C ABI |
|---|---|---|---|---|---|
| Run rules, write the inference graph | `purrdf_shapes::infer`, `srl::infer` | `purrdf rules` | `apply_rules` | `shaclApplyRules` | `purrdf_shacl_apply_rules` |
| Evaluate one node expression | `free_expression::evaluate` | `purrdf node-expr` | `eval_node_expr` | `shaclEvalNodeExpr` | `purrdf_shacl_eval_node_expr` |
| Certify a shapes graph | `lint::lint` | `purrdf shapes lint` | `lint_shapes` | `shaclLintShapes` | `purrdf_shacl_lint_shapes` |

**Rules.** The rule source is either a shapes graph, whose SHACL 1.2 rules run
(its default rule set), or a SPARQL 1.2 RL rule set, but not both. The result is
the **inference graph**: the inferred triples only, never the data graph, as
N-Triples 1.2 in one canonical order. On request, each host also returns the
proof of every inferred triple. The proof is one block per triple: `derived S P
O .`, then `  rule R` and one `  premise S P O .` line for each fact the rule's
body matched. A SPARQL 1.2 RL data-block triple has `  data-block` instead.

Every host takes the same **term-generating round limit**. This bounds the
evaluation rounds that infer a term the graph did not already hold, and one more
round fails the run naming the limit. The default is 65,536 rounds. **A host
running untrusted rule sets should lower it**: a rule set whose term generation
diverges (an exponential one in particular) reaches the engine's fixed arena and
join ceilings only slowly under the default.

```python
import purrdf

out = purrdf.shapes.apply_rules(data_nt, shapes_ttl, explain=True,
                                max_term_generating_rounds=1024)
print(out["inferred"])   # the inference graph, N-Triples
print(out["proof"])      # the proof text
out = purrdf.shapes.apply_rules(data_nt, srl=srl_text)  # SPARQL 1.2 RL
```

**Node expressions.** One node expression of a shapes graph is evaluated
against a focus node of a data graph, as SHACL 1.2 Node Expressions'
`evalExpr(expr, focusGraph, focusNode, scope)`. The expression is parsed by the
shapes parser itself, so custom functions, shape references and `sh:prefixes`
bind as they do inside a shape. Scope variables are bound by name and read by
`shnex:var`. The output nodes come back as N-Triples terms in the order the
expression's sequence semantics define.

Every host names the expression in one of three ways:

- **The node itself**: an IRI, or `_:label` for a blank node the shapes
  document labels. An IRI that is the subject of no triple is a constant
  expression and evaluates to itself.
- **A walk from a named node**: a start node and one or more predicates,
  followed in order. Each step must reach exactly one value; a step that
  reaches none or several is refused, and the error names the step and the
  count. This is how an anonymous `[ … ]` expression is named, which is how
  most node expressions are written. `ex:Label` then `sh:values` is that
  property shape's computed-values expression. A W3C `sht:EvalNodeExpr` test
  entry's expression is the entry, then `mf:action`, then `sht:nodeExpr`. All
  143 of the suite's node-expression tests run through `purrdf node-expr` this
  way.
- **Inline Turtle**: a Turtle document read under the shapes document's
  prefixes and base, where its own directives take precedence. It is merged into
  the shapes graph, so its shape references and function calls bind there, and
  its blank nodes are kept apart from the shapes document's. The expression is
  the document's one root: the blank node that is the subject of a triple and
  the object of none. A document with no root or several is refused.

| Host | Node | Walk | Inline Turtle |
|---|---|---|---|
| CLI | `--expr` | `--expr-at NODE --expr-via PREDICATE…` | `--expr-turtle`, `--expr-turtle-file` |
| Python | `expr` | `expr_at=`, `expr_via=[…]` | `expr_turtle=` |
| WebAssembly | `expr` | `exprAt`, `exprVia` | `exprTurtle` |
| C ABI | `expr` | `expr_at`, `expr_via`, `expr_via_count` | `expr_turtle` |
| Rust (`purrdf_validate`) | `ExprSelector::Node` | `ExprSelector::At` | `ExprSelector::Turtle` |

Naming none of the three, or more than one, is refused.

```python
purrdf.shapes.eval_node_expr(shapes_ttl, data_nt, "_:suffix",
                             "http://example.org/a", scope={"suffix": '"!"'})
purrdf.shapes.eval_node_expr(shapes_ttl, data_nt, None, "http://example.org/a",
                             expr_at="http://example.org/Label",
                             expr_via=["http://www.w3.org/ns/shacl#values"])
purrdf.shapes.eval_node_expr(shapes_ttl, data_nt, None, "http://example.org/a",
                             expr_turtle="[ sh:path ex:name ] .")
```

```sh
purrdf node-expr --shapes shapes.ttl --expr-turtle '[ sh:path ex:name ] .' \
  --focus http://example.org/a data.ttl
```

**Certifying a shapes graph.** Loading a shapes graph is the hot path: it
refuses the first construct it cannot evaluate faithfully, and does not pay for
validating the graph against the W3C `shacl-shacl.ttl`. Linting pays that cost
once, on request, and reports four sections:

1. `load`: the loader's verdict, accepted or the refusal it raised.
2. `shacl-shacl`: every result of validating the shapes graph against the
   vendored `shacl-shacl.ttl`. That file predates some SHACL 1.2 Core
   relaxations (`sh:closed sh:ByTypes`, a list-valued `sh:nodeKind`, a
   path-valued `sh:equals`, a node-expression `sh:targetNode`). A result SHACL
   1.2 Core makes well-formed is marked `superseded` and is not a finding, but
   only when the loader accepted the graph.
3. `functions`: which implementation every node-expression function call binds
   to — `native`, `custom`, `sparql-registered` or `host-extension`. For a shapes
   graph that merges the W3C SHACL 1.2 vocabularies, a `sh:sparqlExpr` call
   binds `native` even though the graph declares `sh:SPARQLExprExpression`.
4. `validators`: every validator the shapes graph declares for a built-in
   constraint component, which the native implementation supersedes and never
   runs. These lines are never findings.

A report is clean when the loader accepted the graph and every `shacl-shacl`
result is superseded. Every host renders the same deterministic text; the
[CLI reference](https://github.com/Blackcat-Informatics/purrdf/blob/main/crates/cli/README.md#shapes-lint)
shows it.

## From Python

```python
from purrdf import shacl

report = shacl.validate(shapes_ttl="...", data_nt="...")
print(report["conforms"])  # True / False
print(report["results"])   # list of violation dicts
```

Each result dict keeps the stable keys `focus`, `path`, `value`, `severity`,
`component`, `source_shape`, and `messages` (every `sh:resultMessage`, each a
dict with `text` and its `language` / `direction` / `datatype` when present).
A result that carries SHACL-SPARQL result annotations also has `annotations`, a
list of tuples, each a property IRI and a value in N-Triples syntax.

## The report is a dataset

`ValidationReport::to_dataset()` materializes the W3C validation report —
the `sh:ValidationReport` node and its `sh:ValidationResult`s — as a frozen
`RdfDataset`, built straight from the report's own terms rather than through a
`to_ntriples()` → `parse_dataset()` round-trip. The direct path carries every
RDF 1.2 term the report holds (a triple-term focus node included) with the
report's own blank-node labels, and the blank nodes the report *mints* (the
report node, one per result, the interior nodes of a complex `sh:path`) are
guaranteed distinct from every blank node the data graph *carries*. Rendering
the report in any syntax is then `serialize_dataset(&report.to_dataset(), …)`,
which is exactly what `purrdf validate --format` does.

## SARIF output

Validation reports stay structured in the engine; the SARIF 2.1.0 boundary is
the separate [`purrdf-validate`](https://docs.rs/purrdf-validate) crate
(`purrdf::validate`), which renders a report — or parser diagnostics — as a
source-traced, byte-deterministic SARIF log for editors, CI, and
code-scanning dashboards:

```rust,ignore
use purrdf::validate::{validate_to_sarif_string, SarifOptions};

let shapes = r#"
    @prefix sh:  <http://www.w3.org/ns/shacl#> .
    @prefix ex:  <http://example.org/> .
    @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
    ex:PersonShape a sh:NodeShape ;
      sh:targetClass ex:Person ;
      sh:property [ sh:path ex:age ; sh:datatype xsd:integer ] .
"#;
let data = r#"<http://example.org/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .
<http://example.org/alice> <http://example.org/age> "nope" .
"#;

let sarif = validate_to_sarif_string(shapes, None, data, &SarifOptions::default())
    .expect("sarif produced");
assert!(sarif.contains("\"version\": \"2.1.0\""));
```

Lower-level entry points (`build_report_sarif`, `build_diagnostics_sarif`)
build a `SarifLog` value instead of a string, so a host can merge runs before
serializing.

## Conformance

The validator is gated by the vendored W3C SHACL 1.0 `data-shapes` suite, the
vendored W3C SHACL 1.2 suite, a vendored DASH SHACL-AF/rules corpus, and a
first-party frozen corpus of 73 cases with byte-frozen expected reports; SHACL
Rules output is compared to expected inferred graphs by RDFC-1.0 isomorphism.
The parser is also held to the SHACL 1.2 specification's own SHACL-for-SHACL
shapes graph: over every first-party and W3C shapes graph and a set of
generated mutants, it refuses a graph exactly when `shacl-shacl.ttl` reports a
violation, apart from named cases. See
[Conformance & Testing](../project/conformance.md).
