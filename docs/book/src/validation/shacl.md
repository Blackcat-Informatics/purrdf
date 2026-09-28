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
  evaluate them. The refusal is `ShapesError::ShaclJs`, and it applies where a
  shape reaches SHACL-JS: `sh:js` or a SHACL-JS type on the shape, a
  `sh:JSTarget` or an instance of a `sh:JSTargetType` as its target, a
  `sh:JSRule` among its rules, and a call to a `sh:JSFunction` from a node
  expression or from SPARQL the shape runs. A library that only declares
  `sh:JSLibrary`s and `sh:JSFunction`s nothing calls loads, and those
  declarations are inert. A `sh:JSValidator` attached to a constraint component
  is not inert: it is neither an ASK- nor a SELECT-based validator, so as a
  value of `sh:validator` (`validator-class`), `sh:nodeValidator`
  (`nodeValidator-class`) or `sh:propertyValidator` (`propertyValidator-class`)
  it makes the shapes graph ill-formed, and the load fails with
  `ShapesError::IllFormed` whether or not a shape uses the component;
- `sh:describe` and `sh:update` on a shape, a node expression, a SPARQL-based
  constraint, a validator or a rule. The SHACL 1.2 vocabulary declares them as
  the queries of `sh:SPARQLDescribeExecutable` and `sh:SPARQLUpdateExecutable`,
  but no SHACL specification executes either class, so in those positions they
  would otherwise be silently ignored. A resource that is only such an executable, and
  that no shape reads, loads.

Two of the terms the engine evaluates need a note:

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

The W3C SHACL 1.0 `data-shapes` suite passes as approved (129/129, zero ledgered
gaps at the time of writing), and so does the approved W3C SHACL 1.2 suite apart
from six entries whose approved result spells a decimal non-canonically, which are
counted on their own line (538/544); the live
numbers are in
[`docs/CONFORMANCE.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/CONFORMANCE.md),
and [SHACL 1.2 conformance](#shacl-12-conformance) below says how to
reproduce them.

A parsed shapes graph can also be compiled once and written out as a
deterministic, authenticated byte artifact, so a later process restores a
prepared validator instead of re-parsing Turtle — see
[Prepared Shapes Products](shapes-products.md).

## `owl:imports` in a shapes graph

A shapes document may carry an `owl:Ontology` header or a `sh:ShapesGraph`
declaration that `owl:imports` other documents, and the shapes it constrains with may live entirely in those imports.
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

Not every `owl:imports` triple is an import. OWL 2 reads a document's imports
off its ontology header (*Mapping to RDF Graphs* §3.1.2, Table 4:
`x rdf:type owl:Ontology . x owl:imports y`). SHACL 1.2 Core follows them from
the shapes graph's own IRI along `^owl:versionIRI?/owl:imports`, and names the
class that declares a shapes graph: "The sh:ShapesGraph class MAY be used as an
rdf:type of the IRI of a graph that typically acts in the role of a shapes
graph." PurRDF takes OWL 2 as the floor of the rule and SHACL's shapes graphs on
top of it. So an `owl:imports` is an import only when its subject is one of
these:

- the shapes document's own IRI (its `file://` retrieval IRI, `--shapes-base`,
  `shacl pack --base` or an in-document `@base`; for an imported document, the
  IRI it was imported by);
- a node the document types `owl:Ontology`;
- a node the document types `sh:ShapesGraph`. Every such node counts, and so does
  a `sh:RulesGraph` or a node of a class the document declares
  `rdfs:subClassOf sh:ShapesGraph`;
- a node naming one of those as its `owl:versionIRI`.

On any other node it is data: it stays in the shapes graph as written, and no
document is looked for. A node that is only a `sh:DataGraph` is such a node.
SHACL 1.2 Core §6.2 says "owl:imports in the data graph is not enacted", and
its note asks a data graph that means an import to be typed `owl:Ontology` as
well. The data graph's own `owl:imports` are never read at all. `purrdf shapes
lint` lists every `owl:imports` that is data under `unanchored-imports`. The
same rule decides entailment's imports, and the node set it reads is the one
the implicit SPARQL prefixes below are collected from.

SHACL-SPARQL's prefix path is such data. A query collects its prefixes along
`sh:prefixes/owl:imports*/sh:declare` within the shapes graph, and the W3C test
suite writes

```turtle
<http://example.com/ns#> sh:declare [ sh:prefix "ex" ; sh:namespace "http://example.com/ns#"^^xsd:anyURI ] .
ex:TestPrefixes owl:imports <http://example.com/ns#> ;
  sh:declare [ sh:prefix "test" ; sh:namespace "http://test.com/ns#"^^xsd:anyURI ] .
```

`ex:TestPrefixes` is neither the document's IRI nor an ontology header nor a
shapes graph, so that `owl:imports` is a prefix edge, not an import, and the test validates as
written. So does `sparql/component/validator-001`, whose
`owl:imports <http://datashapes.org/dash>` sits on a node that is neither.

An import whose document or ontology is **already loaded** needs no pair. It
is resolved when it names a document that was read: the shapes document's
own base (its `file://` retrieval IRI, `--shapes-base`, or `shacl pack
--base`), an in-document `@base`, or an `--import` document's IRI. An import
is also resolved when the graph holds `<X> a owl:Ontology`,
`<X> a sh:ShapesGraph`, or an ontology whose `owl:versionIRI` is `<X>`.

Every one of these rules applies across the whole closure, so a declaration
that arrives in an imported document counts too. A
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

A closure must also hold **one version of each series**. SHACL 1.2 Core §1.3 makes
a shapes graph ill-formed when its import closure holds two graphs that "are
different versions of the same series (i.e., they share the same shapes graph IRI
but have different owl:versionIRI values), or one contains an owl:incompatibleWith
annotation whose value is equal to either the shapes graph IRI or the
owl:versionIRI of the other", and §6.1 says the closure "SHOULD NOT" hold them.
PurRDF reads that as a MUST NOT. Such a closure is refused with the typed import
error, kind `incompatible-import-versions`, naming both graphs, on every host.
Entailment refuses the same closure (OWL 2 §3.4 states the same two conditions).

### A data graph names its shapes graphs

SHACL 1.2 Core §6.4 lets the DATA graph suggest shapes graphs:

```turtle
<http://example.com/myDataGraph> a sh:DataGraph ;
  sh:shapesGraph ex:graph-shapes1 ;
  sh:shapesGraph ex:graph-shapes2 .
```

"Every value of sh:shapesGraph is an IRI representing a graph that SHOULD be
included into the shapes graph used to validate the data graph", and PurRDF
reads the SHOULD as a MUST. A `sh:shapesGraph` triple is such a link when its
subject is the data graph's own IRI — `--base`, or the data file's `file://`
retrieval IRI, on the command line — or a `sh:DataGraph` node (N-Triples text
handed to a binding has no IRI, so there the type is the anchor). Each linked
graph is resolved through the same import table as an `owl:imports`: a table
entry, a graph the shapes graph already declares, or an ontology whose
`owl:versionIRI` is the link — "the same strategy of resolving a shapes graph IRI
from a version IRI … applies here" — and the linked graph's own `owl:imports`
are followed. The linked graphs are unioned into the supplied shapes graph; an
empty shapes document makes them the whole shapes graph. A table entry only a
link names is used, not unreached.

A link nothing resolves is refused as `unresolved-shapes-graph-link`, naming
the IRI (and, on the command line, the `--import IRI=FILE` pair that resolves
it). A link value that is not an IRI is `invalid-shapes-graph-link`. A
`sh:shapesGraph` on any other node is data, like any other triple. The data
graph's own `owl:imports` stay unenacted (§6.2): a link names a graph for the
shapes graph, and nothing about it turns the data graph's imports into
directives.

A shapes graph prepared before the data graph was known — a parsed `Shapes`
validated against many graphs, a `PreparedShapes`, a prepared product —
cannot take a graph in. Validating with one checks that every link is a graph
it already holds: its base, a document its closure or links folded in (a
product records those IRIs, so it holds them after a restore too), or a
shapes graph, ontology or version IRI it declares. Any other link is refused as
`unheld-shapes-graph-link` rather than validated against a smaller shapes graph
than the data graph names. On the incremental change path, a change that adds
or retracts a link changes the shapes graph itself, so the run validates the
whole mutated graph and its scope says why.

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
| Rust | a `ShapesImports` (`from_turtle`, `insert`, `declare_loaded`, `link_data_graph`) passed to `parse_shapes_with_config`, `from_dataset_with_base`, `validate_graphs_with_options`, `lint::lint` or `FreeExpression`; the `purrdf-validate` boundary takes `(IRI, Turtle)` pairs | `ShapesError::Imports(ShapesImportError::Unresolved \| Unreached \| InvalidEntry \| UnresolvedLink \| UnheldLink \| InvalidLink)` |
| CLI | `--import IRI=FILE`, repeatable | exit 1 for an unresolved import; exit 2 for a pair nothing imports or a malformed pair |
| Python | `imports=[(iri, turtle), ...]` on `validate`, `entail`, `apply_rules`, `eval_node_expr`, `lint_shapes`, `pack_product` and `Shapes(...)` | `purrdf.shapes.ShapesImportError` (a `ValueError`) with `.kind` and `.iris` |
| WebAssembly | trailing `importIris`, `importDocuments` arrays on every `shacl*` function that takes a shapes graph | `ShaclImportError` with `kind`, `iris` and `message` |
| C ABI | `import_iris`, `import_documents`, `import_count` before the out-parameters | `PURRDF_STATUS_SHAPES_IMPORT_ERROR`, read with `purrdf_shapes_import_error_kind`, `_iri_count` and `_iri` |

Each imported document is Turtle, parsed with its ontology IRI as its base. The
kind is `unresolved-import`, `unreached-import` (a table entry neither an import
nor a data-graph link names — refused on every host, because its shapes would
be read and never applied) or `invalid-import` (a key that is not an absolute
IRI, a key named twice, or a document that does not parse); the three
`*-shapes-graph-link` kinds below belong to the data graph's links. Branch on
the kind, not on the message.

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

## `IMPORTS` in a SPARQL 1.2 RL rule set

A SPARQL 1.2 RL rule set can `IMPORTS` other rule sets. PurRDF fetches none of
them. The rules tool takes the imported rule sets from the same import table it
takes for a shapes graph, with rule-set text in place of Turtle: `--import
IRI=FILE` on `purrdf rules --srl`, `imports=[(iri, text), ...]` on Python's
`apply_rules(srl=...)`, `importIris` / `importDocuments` on `shaclApplyRules`,
and `import_iris` / `import_documents` / `import_count` on
`purrdf_shacl_apply_rules`. Every host resolves the table through
`RuleSetDocument::resolve_import_table`. The imports are followed
transitively and each IRI is read once. An imported rule set is parsed with its
IRI as its base, and its rules run after the importer's.

Every host refuses an import that no table entry supplies, with the same
message, which names the IRI. It also refuses a table entry that the import
closure never names, because that rule set would be read and never used. The
command line exits 1 for the first case and 2 for the second.

```python
import purrdf

inferred = purrdf.shapes.apply_rules(
    data_nt, srl=rules_srl, imports=[("https://example.org/more", more_srl)]
)["inferred"]
```

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
  counts too. These are the same nodes the `owl:imports` rule reads: one
  classifier decides both.
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

## The shapes graph as a named graph: `$shapesGraph`

SHACL 1.0 pre-binds `$shapesGraph` in every SHACL-SPARQL query to the IRI of the
shapes graph, so a query can read the shapes graph with `GRAPH $shapesGraph { … }`.
SHACL 1.2 removed that pre-binding, and there `$shapesGraph` is an ordinary
variable. PurRDF keeps the SHACL 1.0 behaviour as a caller's choice. Name the
shapes graph's IRI and `$shapesGraph` is pre-bound to it, with the shapes graph
exposed under that name. Name none and `$shapesGraph` stays an ordinary, unbound
variable. A relative IRI resolves against the shapes document's base, so it names
what `sh:shapesGraph <that reference>` in the document would name. A relative IRI
with no base in scope names no graph and is refused (`iri-relative-no-base`).

Every host takes the IRI:

| Host | Spelling |
|---|---|
| CLI | `--shapes-graph IRI` on `validate`, `shapes lint`, `shacl pack` and `rules` |
| Python | `shapes_graph=` on `validate`, `Shapes(...)`, `pack_product`, `lint_shapes`, `apply_rules` and `entail` |
| WebAssembly | a trailing `shapesGraph` on `shaclValidateToSarif`, `shaclValidateChangesToSarif`, `shaclPackProduct`, `shaclLintShapes`, `shaclApplyRules` and `shaclEntail` |
| C | `shapes_graph_iri` after `shapes_base_iri` on `purrdf_shacl_validate_to_sarif`, `purrdf_shacl_validate_changes_to_sarif`, `purrdf_shapes_product_encode`, `purrdf_shacl_lint_shapes`, `purrdf_shacl_apply_rules` and `purrdf_shacl_entail_to_ntriples` |
| Rust | `engine::parse_shapes_with_graph`, `engine::validate_graphs_with_shapes_graph`, `engine::entail_graphs_with_shapes_graph`, `RulesRequest::shapes_graph` and the `purrdf-validate` `*_with_shapes_graph` entry points |

A `Shapes` parsed under the IRI carries it into every validation, every
`PreparedShapes` binding and every prepared product, whose identity binds it.
SHACL-AF SPARQL rules run in the same shapes-graph context: a `sh:SPARQLRule`'s
`$shapesGraph` is pre-bound to the IRI a rules run names, and is an ordinary
variable when it names none. A SPARQL 1.2 RL rule set has no shapes graph, so an
IRI named beside one is refused rather than ignored.
The W3C SHACL 1.0 test `sparql/pre-binding/shapesGraph-001` shows the difference.
With the IRI named, it reports the one approved result. With none, `$shapesGraph`
is unbound, the query selects nothing, and the data graph conforms.

## Class hierarchies kept in the shapes graph: `subClassOfInShapesGraph`

SHACL type — whether a node is a SHACL instance of a class, which `sh:targetClass`,
implicit class targets, `sh:class`, `sh:rootClass` and `shnex:instancesOf` all
ask — is read from the data graph's `rdf:type` and `rdfs:subClassOf` triples. The
class hierarchy often lives beside the shapes instead, so SHACL 1.2 Core §6.3 says
"SHACL processors SHOULD offer a parameter subClassOfInShapesGraph that, if set to
true, should alter the definition of SHACL Type so that the rdfs:subClassOf triples
are queried from the shapes graph in addition to the data graph." PurRDF offers it
on every host. It is off by default, which is the specification's default.

With it on, the shapes graph's `rdfs:subClassOf` triples (its whole import closure's)
join the data graph's wherever SHACL type is decided, and nowhere else. They do not
become data-graph triples, so a path, a `sh:targetSubjectsOf rdfs:subClassOf` or a
SPARQL pattern still reads the data graph alone. `rdf:type` triples are always read
from the data graph.

| Host | Spelling |
|---|---|
| CLI | `--subclass-of-in-shapes-graph` on `validate` (document and product routes) |
| Python | `subclass_of_in_shapes_graph=True` on `validate` and `Shapes(...)` |
| WebAssembly | a trailing `subClassOfInShapesGraph` on `shaclValidateToSarif` |
| C | `bool subclass_of_in_shapes_graph` after the import table on `purrdf_shacl_validate_to_sarif` |
| Rust | `ValidationOptions::with_subclass_of_in_shapes_graph` |

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
  DASH declare them for SHACL Core components. A SPARQL alternative must be a
  well-formed SPARQL validator of its attachment: an ASK validator under
  `sh:propertyValidator` (`propertyValidator-class`), a SHACL-JS
  `sh:JSValidator` under any attachment (`validator-class`,
  `nodeValidator-class`, `propertyValidator-class`) or an unparsable query
  (`ask-sparql`) fails the load. It is never executed, so its query may call a
  function the engine does not have, and a `MINUS` in its pre-bound query does
  not fail the load: `purrdf shapes lint` lists it under `unexecuted`;
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

A SHACL-SPARQL or SHACL-AF declaration that violates a syntax rule fails the
load whether or not any shape reaches it. SHACL 1.2 Core says "A SHACL processor
SHOULD produce a failure in this case" and does not limit that to what a shape
uses. The rules are those of SHACL 1.2 SPARQL Extensions and SHACL Advanced
Features, and the refusal names each one by its id. A value of
`sh:nodeValidator` or `sh:propertyValidator` must be a SELECT validator and a
value of `sh:validator` an ASK validator (`nodeValidator-class`,
`propertyValidator-class`, `validator-class`). A component parameter must not be
named `this`, `path`, `PATH` or `value` (`parameter-name-not-in`). A
`sh:SPARQLFunction` parameter must not have one of those names, nor
`shapesGraph` or `currentShape`. A `sh:SPARQLFunction` needs exactly one
`sh:ask` or `sh:select` (`SPARQLFunction-query`). A `sh:select` body returns the
binding of its one result variable, and SHACL Advanced Features says "such SELECT
queries should only return at most one solution"; PurRDF reads that as a must, so a
call whose body returns a second solution fails the validation rather than answering
with whichever row came first. The refusal is
`ShapesError::IllFormed` and lists every violation in the graph, so a shapes
graph that imports a library with ill-formed declarations, as DASH has, fails
with all of them named.

A pre-binding violation is judged where the query runs. SHACL 1.2 SPARQL
Extensions requires a failure for a query "executed with pre-bound variables"
that contains a `MINUS`, a `VALUES` or an `AS ?var` for a pre-bound variable.
The load therefore fails, with `ShapesError::Prebinding`, when a shape's
`sh:sparql` constraint violates one, when a use of a custom component selects such
a validator, when a node expression or a query a shape reaches calls such a
`sh:SPARQLFunction`, when a shape instantiates such a `sh:SPARQLTargetType`, or
when a shape reaches such a `sh:select` node expression. A function's or target
type's parameters are its pre-bound variables, and a select expression's is
`$this`. A validator of a built-in component never runs, and neither does a
validator no use selects, a function nothing calls or a target type no shape
instantiates. Those load, and `purrdf shapes lint` lists each one as a finding.

The same appendix says "SPARQL queries SHOULD not contain a federated query
(SERVICE)", and "Implementations that do not permit SERVICE MUST report a failure".
PurRDF permits `SERVICE` in no SHACL-SPARQL query, including a `sh:SPARQLTarget`
and a function with no parameters, which pre-bind nothing. A verdict that
depended on what a remote endpoint answered would not be a verdict about the data
graph, and PurRDF fetches nothing. Such a query fails the load where it runs, with
`ShapesError::Prebinding`, and is listed by `lint` where nothing runs it. The word
inside a string literal is not a `SERVICE`.

A shape whose `sh:target` is a custom target PurRDF cannot compute is refused
with `ShapesError::UnsupportedTarget`, naming the shape and the target. PurRDF
computes a `sh:SPARQLTarget` and an instance of a declared `sh:SPARQLTargetType`.
SHACL Advanced Features says an engine that "cannot handle a given custom target
SHOULD at least report a warning". A warning beside a report about focus nodes
nobody computed would still be that report, so PurRDF refuses, which is stronger.
A SHACL-JS target is refused as `ShapesError::ShaclJs`.

A `sh:SPARQLTarget` may also carry one `sh:ask`. SHACL Advanced Features §3.1
says "SPARQL-based targets have at most one value for the property sh:ask", for
a SELECT that cannot be turned into an equivalent ASK: "A SHACL engine can then
determine whether a given shape applies to a given node by executing the ASK
query with the variable this pre-bound to the node." PurRDF does exactly that
wherever it checks given nodes instead of enumerating the target:
`PreparedValidator::validate_focus_nodes` and its id-native twin. (A change-path
validation over a shapes graph with a SPARQL target has no bounded footprint, so
it validates the whole graph.) The candidate is pre-bound to `$this`, so the ASK
must meet the pre-binding restrictions, and a node the data graph does not hold is
answered too. The ASK does not decide alone: its answer is confirmed against the
target's own SELECT results for the same data graph, because the two state one
target. A node they answer differently for is refused with
`ShapesError::SparqlTargetDisagreement`, naming the shape, both queries, the node
and both answers, since checking it by either answer would not give the verdict a
whole validation gives. A whole validation still enumerates the target with its
SELECT, and never runs the ASK.
A second `sh:ask`, a value that is not an ASK query, or an ASK that breaks a
pre-binding restriction is refused at load. A target without `sh:ask` checks a
candidate by looking it up in its SELECT's results.

## SHACL 1.2 conformance

The complete W3C `shacl12-test-suite` is vendored byte-exact under
`vectors/shacl12/` and run through the library API: 547 tests, covering 174
`sht:Validate`, 143 `sht:EvalNodeExpr` and 27 `sht:Infer` tests, and 203
SPARQL 1.2 RL syntax, well-formedness, stratification and evaluation tests. An
upstream manifest lists 544 of them. Of those, 538 pass as approved and 6 are
counted apart because their approved result spells a decimal non-canonically, as
described below, and the expected-failure ledger is empty. The other 3 are entries of
vendored files that no manifest includes, and they are reported apart. A
negative SPARQL 1.2 RL test must fail at the stage its test type names, not at
an earlier one.
Reproduce it with:

```bash
cargo test -p purrdf-shapes --test w3c12_conformance -- --nocapture
```

The last line of the approved suite's scoreboard is
`W3C12 TOTAL: passed 538, non-canonical-expected-decimal 6, xfailed 0, ledger 0`,
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

In six node-expression tests the approved result spells a computed
`xsd:decimal` in a lexical form that is not canonical: `"4.0"`, `"3.0"`,
`"42.0"` and `"00"`, where XSD 1.1 Part 2 (section 3.3.3.1 and the
`decimalCanonicalMap` of E.1) makes the canonical form `"4"`, `"3"`, `"42"` and
`"0"`. (`"00"` is canonical in neither XSD 1.1 nor XSD 1.0, whose canonical
decimal zero is `"0.0"`.) The expected value is correct in each, and the suite
says the results must be "equal" without saying whether that is term or value
equality. PurRDF emits the XSD 1.1 canonical form, which is also what the
approved W3C SPARQL tests `ceil01`, `floor01`, `round01` and `seconds` expect.
The harness grades these six by substituting the canonical spelling for the
expected one and comparing term for term, which for these six is the same as
comparing values. They are counted on their own line,
`non-canonical-expected-decimal`, not among the passes.

`sparql/component/validator-001`, in this suite and in the SHACL 1.0 suite,
passes by name with no import supplied. Its
`owl:imports <http://datashapes.org/dash>` sits on a node that is neither the
document's own IRI nor an `owl:Ontology` nor a `sh:ShapesGraph`, nor names one
as its `owl:versionIRI`, so under OWL 2's mapping to RDF (§3.1.2) and SHACL 1.2 Core
the triple is data, not an import (see
[`owl:imports` in a shapes graph](#owlimports-in-a-shapes-graph)). The harness
loads the case with an empty import table.

SPARQL 1.2 RL grammar rule [2],
`RuleOrDataBlock ::= Prologue ( RuleOrData+ ( Prologue1 RuleOrData? )* )?`, is
implemented as its evident intent: after the initial prologue, declarations and
rule or data blocks interleave freely, so any number of rule or data blocks may
follow each declaration. Read literally, the rule would refuse
`RULE … PREFIX … RULE … RULE …` while accepting the same rules with one more
declaration between the last two.

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

A property shape that judges its values against shapes projects each shape as
one value's schema, compiled as `sh:memberShape`'s is. `sh:node` and `sh:and`
become `allOf`, `sh:or` becomes `anyOf`, `sh:xone` becomes `oneOf`, and
`sh:not` becomes `not`. `sh:someValue` becomes `contains` over an array of
values, and the value schema itself on a lone value, which it then requires.
A shape's property shapes judge the value's own properties, which live on the
value's own `@graph` node, so they are recorded. `oneOf` and `not` reject a
value when a member schema accepts more than its shape does. They are
therefore emitted only when every member shape is exact, that is, built from
`sh:datatype`, `sh:nodeKind`, `sh:in`, `sh:hasValue` and further exact shapes,
with nothing recorded. Otherwise the `sh:xone` or `sh:not` is recorded and
left out. `sh:class` is not exact, because a value's types live on its own
node. When one value is written unwrapped beside the array of several, the
value schema also rejects an array, so every value of an array is judged.

Pydantic enforces `allOf`, `oneOf`, `not` and `contains` with its runtime check
over the raw JSON input, so it agrees with validation on each of these. LinkML
states them as `all_of`, `any_of`, `exactly_one_of`, `none_of` and
`has_member`, whose expression is the whole value schema, class ranges
included, so nothing is recorded. The official LinkML 1.11.1 JSON Schema
generator, which the LinkML validator also runs, turns a `has_member` into
`contains` only for its scalar constraints. Its output therefore accepts an
array with no conforming value, although the LinkML schema states the
constraint. TypeScript states intersections and unions, and records `oneOf`,
`not` and `contains`, which it has no type for. GraphQL delegates the
composition to its custom scalar. Each emitter oracle runs these constraints
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

Four tools sit beside validation. Every host reaches the same library entry
point, so the command line, Python, WebAssembly and C cannot disagree about the
answer.

| Tool | Rust | CLI | Python (`purrdf.shapes`) | WebAssembly | C ABI |
|---|---|---|---|---|---|
| Run rules, write the inference graph | `purrdf_shapes::infer`, `srl::infer` | `purrdf rules` | `apply_rules` | `shaclApplyRules` | `purrdf_shacl_apply_rules` |
| Check a SPARQL 1.2 RL rule set, run nothing | `srl::check` | `purrdf rules --srl FILE --check` | `check_rules` | `shaclCheckRules` | `purrdf_shacl_check_rules` |
| Evaluate one node expression | `free_expression::evaluate` | `purrdf node-expr` | `eval_node_expr` | `shaclEvalNodeExpr` | `purrdf_shacl_eval_node_expr` |
| Certify a shapes graph | `lint::lint` | `purrdf shapes lint` | `lint_shapes` | `shaclLintShapes` | `purrdf_shacl_lint_shapes` |

**Rules.** The rule source is either a shapes graph, whose SHACL 1.2 rules run
(its default rule set), or a SPARQL 1.2 RL rule set, but not both. The result is
the **inference graph**: the inferred triples only, never the data graph, as
N-Triples 1.2 in one canonical order. On request, each host also returns the
proof of every inferred triple. The proof is one block per triple: `derived S P
O .`, then `  rule R` and one `  premise S P O .` line for each fact the rule's
body matched. A SPARQL 1.2 RL data-block triple has `  data-block` instead.

Every host takes the same four **rule-evaluation limits**, on both of its SHACL
rules entry points: the rules run above and SHACL-AF entailment (Python
`shapes.entail`, WebAssembly `shaclEntail`, C `purrdf_shacl_entail_to_ntriples`),
which bounds its run exactly as the rules run does:

- The **term-generating round limit** bounds the evaluation rounds that infer a
  term the graph did not already hold. The default is 16384 rounds. A counter
  stepping to 10,000 completes. A counter with no bound infers one new term per
  round and is refused in well under a second.
- The **generated-term budget** bounds the terms inferred beyond the input's.
  The default is `max(65536, 4 × N)`, where `N` is the number of distinct terms
  in the data graph (and in a SPARQL 1.2 RL rule set's data blocks). A rule set
  whose new terms double every iteration reaches it within a few iterations,
  long before the round limit.
- The **stored-fact limit** bounds the facts the evaluation store holds: the
  data graph, a rule set's data and every inferred triple. The default is
  4,194,304 facts natively and 131,072 in WebAssembly, where the store lives in
  one linear memory. A rule that copies each of 70,000 `ex:p` triples to
  `ex:q`, or the transitive closure of a thousand-node chain (500,500 triples), completes
  natively.
- The **join-step limit** bounds the candidate solutions the rule bodies
  enumerate. The default is 1,048,576 on every host. It is the limit that
  refuses a rule minting a new term every round promptly, and it bounds a rule
  body that enumerates far more candidates than it infers triples. A rule set
  that needs more work — the non-linear transitive closure of a thousand-node
  chain enumerates over 67 million candidates — states a larger limit.

A run past any limit is refused. The error names the limit, the numbers, the
knob that raises it and, for the two term limits, the rules that inferred a new
term in the last iteration. The knob is spelled the way the calling host spells
it: `--max-term-generating-rounds`, `--max-generated-terms`,
`--max-stored-facts` and `--max-join-steps` on the command line; the
`max_term_generating_rounds`, `max_generated_terms`, `max_stored_facts` and
`max_join_steps` keyword arguments in Python; `maxTermGeneratingRounds`,
`maxGeneratedTerms`, `maxStoredFacts` and `maxJoinSteps` in WebAssembly; and
the `max_term_generating_rounds`, `max_generated_terms`, `max_stored_facts` and
`max_join_steps` parameters in C. The entailment entry point names its own
knob: `entail(max_stored_facts=...)` in Python, `shaclEntail's maxStoredFacts` in
WebAssembly, `purrdf_shacl_entail_to_ntriples's max_stored_facts` in C. Whether a
rule set that keeps inferring new
terms would stop is undecidable, so the error never calls it divergent. A
stated limit is exact, and a limit can only refuse: a run it admits infers
exactly what it would under any larger limit.

A global SPARQL rule whose query is a conjunctive pattern (triple patterns,
`FILTER` and `BIND`, with no `OPTIONAL`, `UNION`, `MINUS`, sub-query or graph
access) runs as rule elements. An iteration then costs only what is new, instead
of a pass over the whole evaluation graph, and infers the same triples. This is
why a 10,000-step countdown completes in well under a second. A `BIND` variable
must appear in every template triple, and a template must have no blank node.
Otherwise the rule runs as a producer over the whole graph, as before.

```python
import purrdf

out = purrdf.shapes.apply_rules(data_nt, shapes_ttl, explain=True,
                                max_term_generating_rounds=1024)
print(out["inferred"])   # the inference graph, N-Triples
print(out["proof"])      # the proof text
out = purrdf.shapes.apply_rules(data_nt, srl=srl_text)  # SPARQL 1.2 RL
```

**Checking a SPARQL 1.2 RL rule set.** A rule set can be checked without being
run. The check applies the static checks up to a level, and each level includes
the ones before it. `syntax` checks the SPARQL 1.2 RL grammar, for the rule set
and every document its `IMPORTS` closure reads. `well-formed` also checks that
every rule, imported ones included, is well formed. `stratified` also checks that
the combined rule set can be stratified. It is the default, and it is exactly the
set of checks a rules run applies before it evaluates: every host's rules run
passes through the same check first. The imports resolve from the same import
table a run takes. No data graph is read and no rule runs. A rule set that passes
returns its rules, data-block triple count, imported rule sets, `VERSION` labels
and strata, and a one-line summary every host prints alike. A refused rule set
fails with the error of the stage that refused it. The W3C SPARQL 1.2 RL suite's
syntax, well-formedness and stratification tests are graded through this check.
A positive syntax test is valid "regardless of well-formedness and
stratification", so it is graded at `syntax`. Fifteen of them fail the full
check, by a later stage.

The `syntax` stage also holds a rule set to where it announces its version.
SPARQL 1.2 RL §7.1 says "The version announcement SHOULD be made early in the
document", and PurRDF reads that as a must. A rule set whose first `VERSION`
directive follows a `RULE` or `DATA` block is refused at `syntax`, naming the
sentence. A rule set may announce no version, and a later `VERSION` after an early
one is still a directive for the part of the document that follows it.

```sh
purrdf rules --srl closure.srl --check              # every static check
purrdf rules --srl closure.srl --check=syntax       # the grammar alone
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
once, on request, and reports seven sections:

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
5. `unexecuted`: every query that violates a pre-binding restriction and that
   nothing runs: a validator of a built-in component, a validator no use of its
   component selects, a `sh:SPARQLFunction` nothing calls, or a
   `sh:SPARQLTargetType` no shape instantiates. The load accepts
   them, and each is a finding here. A declaration that breaks a syntax rule is
   never listed here, because it fails the load.
6. `diagnostics`: every mandatory diagnostic, one `diagnostic RULE SHAPE` line
   per shape whose `sh:in` or `sh:xone` list is empty. `RULE` is the syntax
   rule id, `in-minListLength` or `xone-minListLength` ("Each such list SHOULD
   have at least one member"). Such a shapes graph is well-formed and validates
   (see below), but the rule tells the author something, so lint always reports
   it, whether or not the load succeeded, and each line is a finding.
   `shacl-shacl.ttl` warns about the same empty list; that result is listed
   marked `diagnosed RULE` and is not counted a second time.
7. `unanchored-imports`: every `owl:imports` triple of the closure that is not
   an import, because its subject is no anchor of its document (see
   [`owl:imports` in a shapes graph](#owlimports-in-a-shapes-graph)). Each is
   data, so no document was looked for. These lines are never findings. The
   case the W3C flags is reported by `shacl-shacl` instead: its
   `shsh:DataGraphImportsShape` reports, at severity `sh:Info`, a
   `sh:DataGraph` that uses `owl:imports` without the type `owl:Ontology`.

A report is clean when the loader accepted the graph, every `shacl-shacl`
result is superseded (an `sh:Info` result counts like any other), no
unexecuted query violates a pre-binding restriction and no mandatory diagnostic
applies. `unanchored-imports` never affects it. Every host renders the same deterministic text; the
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
RDF 1.2 term the report holds (a triple-term focus node included). Rendering
the report in any syntax is then `serialize_dataset(&report.to_dataset(), …)`,
which is exactly what `purrdf validate --format` does.

A report carries blank nodes from two graphs: a focus node, a value and an
annotation value come from the data graph, and a source shape from the shapes
graph. Blank-node labels are local to the graph that uses them, so the two
documents often use the same label for different nodes. The report therefore
writes each graph's blank nodes in its own label space: `_:dg0`, `_:dg1`, … for
the data graph and `_:sg0`, `_:sg1`, … for the shapes graph, numbered in the
order they first appear. Two different nodes never share a label, and one node
keeps one label everywhere it appears, including in the SARIF projection. The
labels do not depend on which parser read the documents, so the same shapes
graph written in Turtle, TriG or RDF/XML gives the same report bytes.
`ValidationReport::with_report_blank_labels()` returns the relabelled report
with a `ReportBlankLabels` map from each report label back to its graph and
original label. The in-memory `results` keep the original labels. The blank
nodes the report *mints* (the report node, one per result, and the interior
nodes of a complex `sh:path`) are distinct from both label spaces.

Every report a validation produces also states `sh:shapesGraphWellFormed`. SHACL
1.2 Core §6.7.1.4 says a processor that checks the shapes graph "SHOULD use the
property sh:shapesGraphWellFormed to inform the consumer of the validation report
about this fact", and that `true` means "the processor was certain that the shapes
graph that was used for the validation process is well-formed". PurRDF refuses an
ill-formed shapes graph before validating, so the value is always `true`. An empty
`sh:in` or `sh:xone` list does not change that. Appendix A's `in-minListLength`
and `xone-minListLength` say "Each such list SHOULD have at least one member", and
PurRDF applies them as a mandatory diagnostic: the shapes graph is well-formed,
validation proceeds as the approved W3C tests `core/node/in-002`, `in-003`,
`xone-002` and `xone-003` require, and `purrdf shapes lint` (and every host's lint
entry point) always reports each empty list as a finding naming the rule id.
`ValidationReport::shapes_graph_well_formed` carries the value, and the SARIF run
carries it as `properties.shaclShapesGraphWellFormed`.

Every run reports the same diagnostics, not only the lint. A validation report
carries them in `ValidationReport::diagnostics` — one `MandatoryDiagnostic` per
empty list, the rule id and the shape — beside the results and never among them:
a diagnostic is not a `sh:ValidationResult`, it changes neither `sh:conforms` nor
any result, and the W3C grading is unaffected. It is not written into the report
graph either. SHACL defines no report term for a statement about the shapes graph,
and PurRDF mints no vocabulary, so the RDF report is exactly what it would be
without the diagnostic. Each host carries it in its own slot instead. The SARIF
log puts each in `invocations[0].toolExecutionNotifications` as a notification at
level `note` ("The notification is purely informational. There is no required
action", SARIF 2.1.0 §3.58.6), whose `descriptor` names the rule in
`tool.driver.notifications`. The command line writes `shacl diagnostic RULE
SHAPE` to stderr after the verdict lines. Python's `validate` dict and
`ValidationReport` object have a `diagnostics` list of `{"rule", "shape"}`
dicts. A rules or entailment run reports the same diagnostics: `purrdf rules`
writes the same stderr line, Python's `apply_rules` and `entail` dicts carry
`"diagnostics"`, WebAssembly's `ShaclRulesInference` and `ShaclEntailment` have a
`diagnostics` array of `RULE SHAPE` strings, and C's `purrdf_shacl_apply_rules` and
`purrdf_shacl_entail_to_ntriples` write `diagnostic RULE SHAPE` lines to a
non-NULL `out_diagnostics`.

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
