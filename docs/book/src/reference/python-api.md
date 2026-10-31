<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Python API guide

Use the public `purrdf` package. Its Store, terms and query results call the same native engine as the Rust API. Import validation and entailment through `purrdf.shapes`, `purrdf.shex` and `purrdf.entail`.

The [complete signature reference](python-api-signatures.md) records the canonical
stubs for both the main package and `purrdf.geo`, including properties, overloads,
namespace aliases and supporting types. Read the sections below for ownership,
configuration, result interpretation and failure behavior. Underscore types in
the signatures describe implementation or return types; their presence does not
make them public constructors. For example, use `purrdf.shapes.PreparedShapes`
in annotations, rather than importing `_PreparedShapes`.

`TypedDict` declarations describe dictionary result schemas; do not assume that
each schema name is an exported native constructor. Obtain those records from
the documented producer and read their named fields.

## Load and query

`Store.load(input, format, *, path, base)` accepts text or bytes, or a file selected by `path`. Supply exactly one input source and a format. Its return value is the parsed document's resolved prefix map, sorted by prefix; it is empty for formats without prefix declarations. Relative references require the document's own base declaration or an explicit absolute `base`.

```python
import purrdf

store = purrdf.Store()
prefixes = store.load(
    '@prefix ex: <https://example.org/> . ex:alice ex:name "Alice" .',
    purrdf.RdfFormat.TURTLE,
)
assert prefixes == [("ex", "https://example.org/")]
rows = list(store.query(
    "SELECT ?name WHERE { <https://example.org/alice> "
    "<https://example.org/name> ?name }"
))
assert rows[0]["name"].value == "Alice"
```

`SELECT` returns `QuerySolutions`. A row supports variable names, `Variable` objects and integer positions, including negative indexing. An unbound projected cell is `None`; it is not omitted. Missing variable names raise `KeyError`, and out-of-range positions raise `IndexError`. `ASK` returns `QueryBoolean`. Default-graph `CONSTRUCT`/`DESCRIBE` results iterate over `Triple`; a result carrying named graphs iterates over `Quad`.

## RDF terms and exact values

`NamedNode.value` is its IRI. `Literal.value` preserves its lexical form; datatype, language and direction are separate properties. Language-tag identity folds case. Construct typed literals with `datatype=NamedNode(iri)`, language literals with `language=`, and directional language literals with both `language=` and `direction=`.

```python
name = purrdf.NamedNode("https://example.org/alice")
text = purrdf.Literal("你好", language="zh-Hans")
assert name.value == "https://example.org/alice"
assert text.value == "你好"
assert text.language == "zh-Hans"
```

The default preserves every digit when a quotient terminates. Otherwise it rounds to 18 fractional digits, half to even. `division="exact"` leaves a non-terminating quotient unbound as an expression error; `division="N"` truncates to N fractional digits, and `division="N:ROUNDING"` selects explicit rounding. `exact-or-N[:ROUNDING]` keeps exact terminating quotients and rounds only repeating ones. An invalid policy raises `ValueError` before execution.

```python
result = store.query("SELECT ((1 / 1048576) AS ?value) {}")
assert list(result)[0]["value"].value == "0.00000095367431640625"
result = store.query("SELECT ((2 / 3) AS ?value) {}", division="2:half-even")
assert list(result)[0]["value"].value == "0.67"
xsd = "http://www.w3.org/2001/XMLSchema#"
assert purrdf.xsd_canonical_lexical("+00042", xsd + "integer") == "42"
assert purrdf.xsd_value_compare("1.00", xsd + "decimal", "1", xsd + "integer") == 0

result = store.query(
    'PREFIX xsd: <http://www.w3.org/2001/XMLSchema#> '
    'SELECT (xsd:dateTimeStamp("2024-02-29T12:30:00.123456789012345678901Z"'
    '^^xsd:dateTime) AS ?value) {}'
)
calendar = list(result)[0]["value"]
assert calendar.value == "2024-02-29T12:30:00.123456789012345678901Z"
assert calendar.datatype.value == xsd + "dateTimeStamp"
```

Use lexical forms for exact decimal and temporal values rather than converting through Python `float` or a representation with fewer fractional digits. The XSD helpers return `None` for unsupported datatypes or malformed lexical forms; comparison also returns `None` when the value space does not order the pair, such as NaN.

## Prepared queries

Declare parameter names with `Store.prepare(..., parameters=[...])`. Every run must bind every declared name to an RDF term. An unknown name or a missing binding raises an error. Parser and provider configuration is fixed at preparation; source rows and optional time/entropy inputs are selected for each run. Parameter names are supplied without `?`.

```python
prepared = store.prepare(
    "SELECT ?name WHERE { ?person <https://example.org/name> ?name }",
    parameters=["person"],
)
assert prepared.parameters == ["person"]
rows = list(prepared.run({"person": purrdf.NamedNode("https://example.org/alice")}))
assert rows[0]["name"].value == "Alice"
```

## Supplied query and UPDATE time and entropy

`query`, `query_rdflib`, `query_governed`, `query_entailment_governed`,
`update` and `update_governed`
accept keyword-only `now: str | None` and `entropy_seed: int | None` on both
`Store` and `MutableDataset`. The RDFLib-compatible `Graph.query`,
`Graph.update` and their processors forward the same inputs, including through
Dataset and context receivers. `now` is a native `xsd:dateTime` lexical
form, preserving exact fractional seconds, declared timezone and negative
years without conversion through Python `datetime` or `float`. The seed is
an unsigned 64-bit Python integer; zero is valid. Omission or `None` requests
the native clock or entropy default for that execution.

A prepared handle accepts fresh inputs on each call:

```python
rows = prepared.run(
    {"person": purrdf.NamedNode("https://example.org/alice")},
    now="2024-01-02T03:04:05.123456789012345678901+05:30",
    entropy_seed=0,
)
```

Bindings are an explicit dictionary, so parameter names such as `now`,
`entropy_seed` and `bindings` do not collide with execution options. Each call
replaces all bindings and both coordinate selections; omitted values never
reuse a preceding run's settings. Preparing a query samples neither input.

Malformed time raises `ValueError` naming `now`, with `message_id` and
`presentation` set to `None`. Wrong Python argument types raise `TypeError`;
negative or overflowing seeds raise `OverflowError`. Coordinate conversion
precedes snapshot and execution work. Governor trips, partial answers and
entailment reports retain their existing contracts. These controls do not
serialize an execution receipt or change the serializer's caller-supplied
opaque `query_hash`.

UPDATE uses one request-wide entropy stream across its operations and the
same supplied time in each WHERE clause. Repeating an identical request on
identical starting data with the same coordinates repeats its generated values.
A governed trip still applies nothing. Malformed coordinates fail before any
snapshot, provider execution or mutation; existing engine-configuration errors
retain their precedence. RDFLib first performs its nonmutating namespace and
`initBindings` string preprocessing, then forwards coordinates for native
decoding. Successful updates retain the existing literal reconciliation.

## Governors and incomplete answers

`query_governed` adds `fuel`, `deadline_ms`, `max_answers`, `max_intermediate_cells`, `max_scratch_bytes`, `max_remote_requests`, `max_udf_depth`, `unlimited_udf_depth`, `no_ceiling` and `cancel` to the query options. Ceilings are inclusive. Omitted ceilings retain metering and the default UDF depth limit of 32. Set `max_udf_depth` to a finite logical invocation limit, or `unlimited_udf_depth=True` to remove it; supplying both is a typed conflict. `no_ceiling=True` declines accounting and resource caps, including UDF depth, retains deadline/cancellation, and conflicts with explicitly supplied finite caps.

```python
stopped = store.query_governed("SELECT ?s ?p ?o WHERE { ?s ?p ?o }", fuel=0)
assert not stopped.is_complete
assert stopped.result is None
assert stopped.tripped is not None
complete = store.query_governed(
    "SELECT ?name WHERE { <https://example.org/alice> <https://example.org/name> ?name }")
assert complete.is_complete
assert list(complete.result)[0]["name"].value == "Alice"
```

A complete result is available only when `is_complete` is true. On interruption inspect `tripped`, `partial` and `evidence`; never treat partial answers as the whole result. `update_governed` returns its own outcome, and an interrupted update applies nothing.

## Named graphs and engine configuration

The default Store remembers declared empty named graphs. `remember_empty_graphs=False` selects the explicit row-driven alternative. These modes change graph existence, not the interpretation of populated RDF statements.

```python
remembered = purrdf.Store()
remembered.update("CREATE GRAPH <https://example.org/empty>")
assert remembered.named_graphs() == [purrdf.NamedNode("https://example.org/empty")]
implicit = purrdf.Store(remember_empty_graphs=False)
implicit.update("CREATE GRAPH <https://example.org/empty>")
assert implicit.named_graphs() == []
```

Query, update and prepared entry points accept explicit extension namespaces, property-function namespaces, standpoint predicates, relation tables, aggregate namespace, dated XPath regex profile, geo configuration and division policy. Namespace-dependent extensions need caller-supplied configuration. Relation declarations cross the boundary as data, not Python callbacks.

### Relation tables, graph tables and traversals

`relations` maps an exact predicate IRI to `(input_arity, output_arity, rows)`.
Rows contain native RDF terms and must have the flattened width the declaration
names. `relations_from_graph` supplies a list head and the same arities, reading
the table from the query's dataset. `path_relations` supplies the permitted
predicate/direction steps, minimum and maximum hops, path and expansion guards,
and either `walk` or `shortest`. These are data declarations, not Python
callbacks; evaluation can therefore release the GIL.

A traversal emits one row per hop, binding the endpoint, path identity, length,
step, node and traversed statement. The statement is an RDF 1.2 triple term that
can join back to ordinary graph patterns. Every traversal limit is explicit;
zero minimum hops, invalid or duplicated steps and unusable guards raise
`ValueError`. A valid alternative with no matching edge contributes no row.

The optional attestation is `(generation, incompleteness)`, each a string or
`None`. It records the index version and the host's own reason for incomplete
data, independently. `None` means no declaration, not certification of a whole
index. A governed query returns these declarations in `relation_witness`;
an ungoverned call cannot report an attested shortfall and refuses it. Duplicate
registration across the three relation maps, ragged tables and torn RDF lists
are errors. An exact registered IRI needs no namespace setting; declaring
`property_fn_namespaces` makes an unregistered IRI under those prefixes a hard
error instead of an ordinary graph pattern.

`aggregate_namespace` registers the ten built-in statistical aggregates:
`MEDIAN`, `PERCENTILE`, `STDDEV`, `STDDEV_POP`, `VARIANCE`, `VAR_POP`, `MODE`,
`FIRST`, `LAST` and `TOPK`. It does not register arbitrary Python accumulator
callbacks. `xpath_regex` selects one of `XPATH_REGEX_PROFILES`; omission retains
the compatibility law. Pattern/flag rejection is an expression error, while a
native `xpath-*` resource refusal raises `ValueError`. `standpoint_predicates`
supplies the caller's two predicate IRIs; the library invents none.

### Reading execution evidence

`CancellationToken.cancel()` is monotone; inspect `cancelled` and create a new
token for another independent cancellation lifetime. A governor trip returns an
outcome, while `KeyboardInterrupt` remains an exception. `TrippedGovernor`
exposes `kind`, `label`, `dimension`, `limit`, `consumed`, `estimate` and `cause`;
an admission estimate is not measured consumption.

`GovernorEvidence.consumed` and `limits` are per-dimension mappings, also readable
through `consumed_in()` and `limit_for()`. `expression_errors` counts absorbed
numeric expression errors by qualified XPath code, with zero entries retained.
For example, a projected division error can produce an unbound cell while
the query itself completes; `COALESCE` may recover it.

`PartialAnswers.certainty` distinguishes `certain` (every returned row is an
answer), `at-most` (every answer is among the returned candidates) and `unknown`
(no rows may be used). `is_positional_prefix` is a separate property; a sound
submultiset need not be the first rows of the complete result. `barrier` names
the operator that blocked certification. Read `result` only through its
certificate. `UpdateOutcome.is_applied=False` means no mutation was published.

The always-present `QueryOutcome.relation_witness` maps invoked relation IRIs to
their invocation count, distinct generations and declared incompleteness reasons.
An empty mapping does not certify completeness. Invocation counts describe the
execution schedule and must not be compared as stable index identities across
thread counts or budgets. See the [governor profile](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/SPARQL-GOVERNOR-PROFILE.md)
for accounting, precedence and interval guarantees.

## Containers, mutation and snapshots

| Type | Use and ownership |
|---|---|
| `Store` | Mutable quad store with query, update, load, prepare and checkpoint operations. Its `add` and `remove` return `None`. |
| `MutableDataset` | The same shared query/update interface, with boolean `add`/`remove`, `contains`, `quads_for_pattern` and explicit `compact`. |
| `Dataset` | A mutable quad collection with iteration, length and in-place blank-node canonicalization. It is not the frozen `RdfDataset`. |
| `RdfDataset` | An immutable parsed RDF 1.2 dataset with counts and canonical/text/GTS/columnar export. |
| `QuadIter` | The quad iterator returned by the mutable containers; consume it through the normal iterator protocol. |

`load` and `bulk_load` return the document's final resolved prefix map, not the
number of statements. `add_graph` implements the constructor's graph-existence
mode; `silent=True` suppresses an existing remembered slot, not an invalid graph
name. `named_graphs` excludes the default graph and includes declared empty slots
when the store remembers them.

After bulk loading, `Store.checkpoint()` folds the current delta into the frozen
base without changing contents. `change_size()` then reports additions and
removals since that checkpoint. Checkpoint once after loading before an
incremental validation sequence; otherwise the initial load is itself the
pending change. Compaction is explicit work, not something to perform after
every individual mutation.

A prepared query retains its originating store and reads that store's current
contents on each `run`. Graph-derived relation tables and traversals are rebuilt
for that run's dataset and the plan is admitted under them again. Fixed
configuration does not mean a frozen copy of the data. A `PreparedQuery` is not
thread-safe and cannot be re-entered while a run is in flight.

Queries materialize owned results before returning. `QuerySolutions`,
`QueryTriples` and `QueryQuads` are iterators; copy their rows when a later pass
needs them again. `QuerySolutions.variables` supplies the ordered schema.
`QueryQuads.graph_names` returns sorted graph tokens in N-Triples term syntax.
Serializing a graph-carrying query result to a single-graph syntax raises
`ValueError`; it does not silently erase graph names. Quad-template CONSTRUCT
is a first-party extension, not defined by SPARQL 1.2.

## Terms, formats and serialization

`NamedNode` and `BlankNode` expose `value`; `Variable.value` is its variable name.
`Triple` exposes `subject`, `predicate` and `object`; a `Quad` adds `graph_name`.
RDF 1.2 permits triple terms in object position, not subject position. A default
graph is represented by `DefaultGraph`; the quad constructor also accepts
`None` for that graph. Terms support their documented equality and hashing;
language-tag equality folds case, while `Literal.value` preserves its lexical
form. A plain literal's datatype getter is `xsd:string`; language and directed
language literals expose `rdf:langString` and `rdf:dirLangString` respectively.

The `RdfFormat` members are `TURTLE`, `N_TRIPLES`, `N_QUADS`, `TRIG`, `TRIX`,
`HEXTUPLES`, `JSON_LD`, `YAML_LD` and `RDF_XML`. A document's own base declaration
wins over a supplied `base`. A relative reference with neither raises
`ValueError` carrying `iri-relative-no-base`; no retrieval IRI is fabricated.
On output, formats without a base declaration emit absolute IRIs.

`parse` returns a list of quads. `serialize` and the store `dump` overloads
either write bytes to a binary output or return bytes when no output is passed.
`from_graph` selects the dump's graph; `base` selects the output base and composes
with that selection. `CompiledJsonLdContext` accepts strict options JSON,
can be built with `from_prefixes`, and exposes `canonical_context_json` for
retention. Supply either the supported options or compiled context as declared;
do not assume ordinary Python dictionaries are interchangeable with JSON text.

`dump_with_loss` returns `SerializeLoss`: `bytes` plus
`statement_rows_dropped`, `directional_literals_dropped`,
`named_graph_rows_dropped` and `empty_named_graphs_dropped`. The first three
partition lost rows or decorations by cause; the fourth counts declaration-only
graphs, which have no row to count. Read all counters. They are realized loss
for this output, unlike the static format-pair matrix returned by
`loss_matrix_json()` or `rdf_gts_loss_matrix_json()`.

`to_json_ld`, `serialize_jsonld`, `from_json_ld`, `to_rdf_xml` and `from_rdf_xml`
bridge RDF bytes and their text forms. `statement_vocab` is an explicit
caller-supplied metadata vocabulary; its `class`, `subject`, `predicate`,
`object` and `objectLiteral` values are absolute IRIs. There is no invented
namespace fallback. `project_statements_rdf12` and `normalize_rdf12_to_owl`
provide the statement-codec conversions; `canonicalize_turtle` returns canonical
bytes with optional extra prefix mappings.

`Dataset.canonicalize(canonical_search="unlimited")` relabels blank nodes in
place and returns a `CanonicalResult` whose `value` is `None`. Refusal leaves the
dataset unchanged. `RdfDataset.to_nquads(canonical_search="unlimited")` returns
canonical N-Quads in its result's `value`, using the flat assertion presentation.
It agrees with RDFC-1.0 on the RDF 1.1 subset; the RDF 1.2 profile has its own
reserved-vocabulary, statement-layer and refusal contract. Do not label an RDF
1.2 digest as RDFC-1.0.

Both calls require an explicit `canonical_search`: `"unlimited"` or a canonical
unsigned decimal `u64` string for a finite per-run limit. Both accept an optional
`cancel` token. Each result carries immutable `canonicalization_json` receipt
text. Canonicalization refusal raises `ValueError` with the finalized receipts;
when a native canonical cause exists, `native_cause` and `native_cause_json`
preserve its metadata. Read the
[canonicalization chapter](../concepts/canonicalization.md) before using bytes
as identities.

The XSD helpers work on lexical strings and datatype IRIs:
`xsd_value_compare` returns an order or `None` when unsupported, malformed or
unordered; `xsd_canonical_lexical`, `xsd_decode_binary` and
`xsd_normalize_whitespace` return their value or `None`. Avoid converting an
exact decimal or fractional second through Python `float` first.

### SPARQL Results and provenance

`serialize_sparql_solutions` takes the ordered variable list and rows with one
cell per variable, retaining `None` for unbound cells.
`serialize_sparql_boolean` handles ASK. Both emit deterministic bytes in JSON,
XML, CSV or TSV, accepting the aliases and media types in `SparqlResultsFormat`.
`parse_sparql_results` reads JSON or XML and returns a discriminated tuple:
`("SELECT", variables, rows)` or `("ASK", boolean)`. CSV and TSV have no native
reader here.

Provenance is additive and requires `provenance_namespace=(prefix, iri)`;
`query_hash` supplies the caller's identity. `provenance_from_json` and
`provenance_from_xml` read that namespace back as `query_hash` and `engine`.
Absent provenance yields `None` fields, not an invented identity.

## SHACL validation, preparation and rules

Use `purrdf.shapes`; `purrdf.shacl` is the compatibility alias. The one-call
`shapes.validate` takes Turtle shapes and N-Triples data and returns a report
dictionary. `shapes.Shapes` parses reusable shapes. `validate_nt` returns a
`ValidationReport`; `validate_store` reads a native snapshot from `Store` or
`MutableDataset` without a text round trip. A later mutation changes the next
report, not a report already returned.

`ValidationReport` exposes `conforms`, `shapes_graph_well_formed`, `diagnostics`
and `results`, plus `to_ntriples()` and `to_sarif()`. Result messages retain
their text, language, direction and datatype. Empty `sh:in` or `sh:xone` lists
produce mandatory diagnostics beside results; they do not change conformance
or the report graph. The one-call report records `conformance_disallows`:
omission uses SHACL's Violation/Warning/Info default set, which excludes Debug
and Trace. An empty disallow set or non-IRI severity raises.

```python
shapes_ttl = '''
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <https://example.org/> .
ex:PersonShape a sh:NodeShape;
    sh:targetNode ex:alice;
    sh:property [ sh:path ex:name; sh:minCount 1 ] .
'''
shapes = purrdf.shapes.Shapes(shapes_ttl)
assert shapes.validate_store(store).conforms
prepared_shapes = shapes.prepare()
product = purrdf.shapes.ShapesProduct.open(prepared_shapes.to_product())
restored = product.admit_expecting(product.identity_digest())
assert restored.validate_nt(
    '<https://example.org/alice> <https://example.org/name> "Alice" .'
).conforms
```

### Imports and execution environment

Every shapes entry point resolves imports from the supplied ordered `imports`
table of `(IRI, Turtle document)` pairs. It fetches nothing. Unresolved or
invalid imports, unused entries and incompatible versions raise
`shapes.ShapesImportError`; branch on `kind` and inspect `iris`. Data-graph
`sh:shapesGraph` links must resolve from material in hand. Previously prepared
shapes cannot silently acquire an additional linked graph when data arrives;
an unheld link is refused.

`shapes_base` (or `Shapes(..., base=...)`) resolves relative shapes IRIs.
`shapes_graph` names the graph read by SHACL-SPARQL through `$shapesGraph`;
omission invents no graph. This is the SHACL 1.0 pre-binding removed from
SHACL 1.2. `subclass_of_in_shapes_graph=True` also reads shapes-graph subclass
triples for class membership; `rdf:type` still comes from the data graph.
`xpath_regex` applies the selected dated law to facets and embedded SPARQL.
Resource refusal raises rather than returning an empty or conforming report.

`Shapes.extension_usage` inspects declarations before validation: `sites`
distinguishes calls from data predicates under the supplied namespaces and
registrations; `unreadable` and `complete` expose admission failure. This
distinguishes an unrecognized relation from a valid relation over no rows.
`shapes.lint_shapes` performs a cold audit and reports findings, load errors,
mandatory diagnostics, superseded validators, unexecuted query declarations,
import anchoring and deterministic report text. Neither is data validation.

### Prepared products and incremental reports

`Shapes.prepare()` returns `PreparedShapes`; `to_product()` writes deterministic
bytes. `shapes.pack_product()` combines parsing, preparation and writing.
`ShapesProduct.open()` verifies the envelope and reads its self-description,
without admitting executable preparation. Inspect `format_version`, `stage_id`,
`stage_known`, `identity_digest`, `identity_components` and `explain`;
`to_bytes` returns the envelope. `admit` restores a known stage; `rebuild`
reconstructs it when necessary. Their `_expecting` forms bind the expected
identity and reject mismatches with dimension `shapes-graph`. `certify` checks
the product's certification obligations. Branch on
`ShapesProductError.dimension`, not exception text. `PreparedShapes.provenance()`
distinguishes parsed, restored-admitted and restored-rebuilt preparations.

`PreparedShapes.validate_store_changes(store)` returns `ChangeValidation`.
Read `bounded` before `report.conforms`: when true, the report covers affected
focus nodes only, with no claim about unreachable old violations. When false,
it covers a full validation of the mutated graph. `focus_nodes` is a count on
the bounded arm and `None` otherwise; `reason` identifies the construct requiring
a full run. The pending change comes from the caller's last `checkpoint`;
validation does not redefine that checkpoint.

### Rules and node expressions

`shapes.entail` applies a shapes graph's default rule set and returns base plus
inferred triples. It differs from `purrdf.entail.materialize`, which uses a
regime's rule table without a shapes graph. `shapes.apply_rules` runs a shapes graph or SRL
document and returns only the inference graph, with proof text when
`explain=True`. `check_rules` admits a rule set at the selected checking level:
`level` accepts `syntax`, `well-formed` or `stratified`; omission selects
`stratified`. It reports declarations, imports, versions and strata. Stored-fact, join-step,
generated-term and generating-round controls are explicit keyword arguments;
exhaustion raises a limit-specific `ValueError`.

`shapes.entail` and `shapes.apply_rules` accept keyword-only `now` and
`entropy_seed`, including the SHACL and SRL rule sources and the `shacl` aliases.
They use the same exact `xsd:dateTime` strings and unsigned 64-bit integers as
query and UPDATE execution. Both inputs are independently optional; zero is a
valid seed. A supplied time reaches every rule query without losing fractional
seconds, timezone or negative years. The seed selects a reproducible sequence
for the rule execution's queries. Repeating identical input documents and
coordinates repeats the inferred output.

These controls work with the default or an explicitly selected `xpath_regex`
law. Coordinate decoding follows XPath selection and precedes document/import
parsing and detached execution. Malformed time raises `ValueError` naming `now`,
wrong Python types raise `TypeError`, and negative or overflowing seeds raise
`OverflowError`. The four rule limits and their diagnostic keyword names remain
in force. Imports, shapes-graph selection, proof output and mandatory diagnostics
retain their existing contracts. Static `check_rules` refuses these execution
keywords. These controls do not add execution receipts or extend shape validation,
node expressions or the separate `purrdf.entail` regime API.

`eval_node_expr` selects exactly one expression: a named IRI/blank label,
a unique walk from `expr_at` through `expr_via`, or a one-root inline
`expr_turtle` document. `focus` and `scope` supply terms; the reserved
`focusNode` cannot be rebound. Ambiguous selectors, missing/multivalued walk
steps and evaluation failures raise. Outputs are ordered N-Triples term strings
beside mandatory diagnostics. See [SHACL](../validation/shacl.md) and
[prepared products](../validation/shapes-products.md) for the full contracts.

## ShEx

`purrdf.shex.parse` converts ShExC or ShExJ into canonical ShExJ text.
`shex.validate` takes a schema, data and a fixed shape map. Each `(node, shape)`
pair is echoed in `ShexResultEntry`, with `conformant` and optional `reason`.
Nodes may be bare/bracketed IRIs, `_:` labels or Turtle literal tokens; `START`
selects the start shape. Schema formats are `shexc` and `shexj`; data formats
are `turtle`, `ntriples` and `nquads`. `base` resolves relative IRIs in both
inputs. A pattern rejected by the selected XPath law makes its entry
nonconformant; a native resource refusal raises for the whole map. See
[ShEx](../validation/shex.md).

## Entailment, decisions and proofs

`purrdf.entail.Regime` names `SIMPLE`, `RDF`, `RDFS`, `OWL_RL`, `OWL_DIRECT`,
`RIF` and `D`; functions also accept the lowercase CLI spellings.
Unknown spellings raise. `materialize` returns `(RdfDataset, report)` and
`materialize_nt` returns `(canonical_text, report)`; the report is mandatory.
`program` is the RIF document and must be empty for other regimes. Required
`imports` and `premise_iris` describe the premise's import closure; `[]` means
none. Non-absolute premise IRIs, unresolved imports and unreachable table entries
raise rather than being fetched or ignored. `rules`, `implemented_rules` and
`extensions` expose the specification table, implemented subset and named
non-normative additions.

Chase reports describe rule coverage; OWL Direct-Semantics services return
decision certificates. An incomplete rule table differs from a decision that
exhausted its work cap. Unpack and retain the certificate with the answer.

| Service | Question |
|---|---|
| `consistency` | Whether the knowledge base has a model; false and unknown are distinct. |
| `classify` | Named-class equivalence, transitive subsumption, direct edges and unsatisfiable classes. |
| `realize` | Entailed individual types and most-specific direct types. |
| `instances` | Named instances of one class, supplied as an N-Triples term. |
| `entails` | Whether one OWL RDF-mapping axiom is entailed; different from `graph_entails`. |
| `profile` | Syntactic OWL profile certification and blockers, without decision search. |
| `extract_module` | A locality module for a seed signature under `bot`, `top` or `star`. |
| `justify` | A sufficient minimal axiom subset, with sufficiency/minimality re-decided. |
| `explain_conclusion` | A checkable chase derivation, not a tableau justification. |

`step_cap` and `work_cap` only narrow the decision engine's derived caps;
zero selects its own cap, not zero work. Certificates report rounds, work,
decision count, peak nodes, case splits and depth. Budget exhaustion is
`unknown`, not a negative theorem. Except for `consistency`, decision services
refuse an ontology without a model instead of presenting its vacuous entailments
as ordinary answers. An unmentioned class can have an empty instance answer
without an error.

`entail.Reasoner` retains parsed data, lazily builds a knowledge base when needed
and reuses both across corresponding service methods. Caps are fixed at
construction. `proofs=True` opts into traces; `records_proofs` reports that
choice. `prove` returns answer, certificate and proof; `proof_services` lists
supported service names. A non-recording session refuses proof requests.
`check_proof` checks against the caller's ontology, question, answer and
certificate; an unrecorded proof is never verified. Empty answer/certificate
requests a weaker check whose report says so. A justification and an execution
proof answer different questions.

`certain_answers` asks for substitutions true in every model, not just matches
in one closure. Predicate variables and variables inside triple terms bind;
blank nodes are non-distinguished. Read `limit` lines for non-exhaustive
boundaries. `graph_entails` asks about a conclusion graph, with distinct
entailed, not-entailed and undecided verdicts. `verify_entailment` rechecks a
present warrant against the premise as written. These calls require imports
and premise IRIs, and refuse regimes whose inputs their signatures cannot
express. A variable in a literal's datatype position is refused.

`Store.query_entailment_governed` has two phases. Stopped closure construction
produces `EntailmentQueryOutcome.phase == "closure-stopped"`, without a query
outcome or closure report. Otherwise phase is `answered`, with an ordinary
governed query outcome and report. Graph-backed relations read the closure.
An OWL Direct closure exposing generated existential witnesses through a
traversal is refused with `reasoning-closure-relation-witness`. See
[entailment](../entailment.md) for the full service and certificate grammars.

## Geographic sessions

`purrdf.geo.GeoProfile` owns an immutable strict geographic profile; `to_json`
returns its canonical version-one record with explicit limits. `GeoSession`
accepts that profile, another session, profile JSON or `None`; omission selects
the standard `CRS84` profile. `profile()` returns the selected profile. Query,
update and prepared methods accept these same choices through `geo`.

`GeoSession.call` evaluates a strict scalar/batch JSON request; `point_index`
constructs immutable native point buckets and returns `GeoPointIndex`, whose
`call` refines searches through the same comparison law. Semantic outcomes
remain canonical JSON; operational refusal raises `ValueError`. These paths use
the shared Rust codec, with no Python geometry implementation. See
[GeoSPARQL](../sparql/geosparql.md).

## GTS containers and immutable datasets

The GTS functions are available at the package root and grouped under
`purrdf.gts`; the namespace aliases refer to the same operations and types.
`gts_from_quads` and `gts_from_rdf12_bytes` ingest RDF bytes into a snapshot
under the requested `profile`, `transform` and `base`.
`snapshot_content_id_native` reads the corresponding snapshot identity.

`compile_gts_native` accepts the base document, optional RDF 1.2 layer, named
graphs and their scopes, document/report blobs and slice artifacts. Signing
inputs and the rsyncable threshold are explicit compiler arguments. Blob rows
carry bytes, media type and representation; slice artifact rows additionally
name the slice IRI, name, role and logical path. Keep these tuple positions as
declared instead of guessing from the final element's type.

`compile_gts_with_report` returns `GtsCompileResult`, pairing `snapshot_bytes`
with the ingestion receipt from that same parse and intern operation.
`gts_ingest_report` is the receipt-only accessor. `GtsIngestReport` records
`rows_consumed`, `terms_interned`, `declarations_omitted` and `scratch_bytes`.
Declaration-only named graphs have no row in the frozen distribution payload;
their IRIs are reported as omitted instead of being silently interned and
changing the snapshot identity. Do not treat an empty quad table as evidence
that no declaration was lost. `feedback_bundle_native` attaches report blobs
to the supplied RDF input.

`RdfDataset` owns a frozen parsed dataset. Its `quad_count`, `term_count` and
length are inspection methods. `to_nquads` provides the canonical flat text
described above; `serialize_jsonld` supports the declared JSON-LD/YAML-LD
options and compiled context. `to_gts`, `columnar_rows` and `to_parquet_files`
export the same native carrier. A dictionary's numeric term identifiers are
local to that owner; resolve a term when crossing independent datasets.

```python
frozen = purrdf.RdfDataset(
    '<https://example.org/alice> <https://example.org/name> "Alice" .',
    purrdf.RdfFormat.N_TRIPLES,
)
container = frozen.to_gts()
view = purrdf.GtsFoldViewNative.from_bytes(container)
assert view.quad_count() == 1
alice_id = view.tid_of_iri("https://example.org/alice")
assert alice_id is not None
assert view.public_text(alice_id, "https://example.org/name") == "Alice"
```

### Fold views and language selection

`GtsFoldViewNative.from_bytes` reads a container;
`from_parts` admits caller-supplied term, quad, reifier and annotation tables.
Both refuse a term that reaches itself with `gts-self-reaching-term`, rather
than exposing a renderer that recurses forever. `curie_prefixes` supplies
the host's CURIE mappings; only the documented W3C namespaces are built in.

The four count methods report table sizes. `term_tuple`, `is_iri`, `is_bnode`,
`is_literal`, `iri`, `lex`, `lang`, `datatype`, `nq_token` and `python_value`
inspect a local term; `tid_of_iri` performs reverse lookup and `curie` compacts
an IRI. `quads`, `subjects_by_type`, `objects`, `value`, `predicate_objects`,
`has` and `rdf_list` query the selected `scope`. `reifiers` and `annotations`
return their statement-layer tables. Do not compare their integer IDs with
those of another independently constructed view.

Omitted scope or `None` selects the default graph. `purrdf.GTS_ALL_SCOPE` selects
all graphs; a named-graph IRI selects that graph, and an unknown IRI has no
matching rows. The all-graph sentinel is not a named-graph IRI.

`tag_map` and `available_languages` expose the view's language choices.
`public_text` and `public_literal` select the public value;
the `_with_fallback` forms also take requested languages and report whether
fallback was used. `public_texts` returns the requested alternatives with that
metadata. Retain the language and fallback fields when the presentation depends
on whether the requested translation actually existed.

### Relational and Parquet exports

`relational_rows` and `gts_relational_rows_from_bytes` return
`GtsRelationalRows`: terms, quads, reifiers, annotations and blobs, with the
`directions` array positionally parallel to `terms`. Direction is not an extra
field appended to the positional term tuple. `ColumnarRows` instead carries
the native v1 schema and projected five-table rows, whose dense value-order
identifiers need not equal the original GTS IDs.

`gts_columnar_rows_from_bytes` returns that projection;
`gts_columnar_parquet_from_bytes` and `RdfDataset.to_parquet_files` return
filename-to-bytes mappings. `gts_to_sqlite`, `gts_to_duckdb` and `gts_to_parquet`
write the five-table projection to the requested destination. SQLite uses the
standard library, DuckDB needs its optional extra, and Parquet writing uses the
native codec; PyArrow is an interoperability reader, not the writer. See
[GTS](../gts.md) and the [Columnar profile](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/COLUMNAR.md).

## Graph and tabular projection packages

`project` takes RDF text/bytes, its `RdfFormat`, an explicit `ProjectionProfile`
and strict configuration JSON. It returns `ProjectionPackage` with the selected
`profile`, deterministic USTAR `archive` and structured `losses`.
Each `ProjectionLoss` has `code`, `source`, `target`, `note` and optional
`location`; inspect the records rather than assuming every profile round-trips.
The optional `assets` input belongs to the package configuration.

The profiles are `lpg-csv`, `neo4j-csv`, `open-cypher`, `graphml`, `csvw-exact`,
`csvw-terms`, `okf-terms`, `obo-graphs`, `skos`, `croissant-1.1`, `ro-crate-1.3`,
`datacite-4.6`, `dcat-3`, `dcat-rdf`, `void` and
`frictionless-data-package-1`. `lift` supports the `LiftProfile` subset and
returns `ProjectionLift.dataset` plus losses; a one-way profile is not made
bidirectional by passing its name to `lift`.

For streamed LPG artifacts, `project_artifacts` invokes the artifact callback
with `(event, path, chunk)`. The `ArtifactEvent` values delimit the lifecycle:
`begin-package`, `begin-artifact`, `chunk`, `finish-artifact`, `commit-package`
and `abort-package`. Publish retained artifacts only after commit, and handle
abort. The optional progress callback receives immutable `ProjectionProgress`
snapshots with phase and node/edge/artifact/byte counts. The returned
`ProjectionStream` reports its profile and losses. These are explicit artifact
callbacks, unlike the data-only SPARQL relation declarations. See
[projections](../concepts/projections.md) for each profile's strict configuration
and round-trip contract.

## SSSOM mapping documents

`validate_sssom` returns `SssomDiagnostic` dictionaries containing severity,
code, message, check and optional instance. A clean document yields `[]`;
a parse failure is a `FATAL` diagnostic with check `parse`, not a successful
empty validation. `sssom_to_rdf` produces RDF from TSV;
`sssom_roundtrip_tsv` writes the parsed mapping document, and
`sssom_default_validation_types` exposes the native default validation types.
These are the native SSSOM codec, not an import of the external `sssom` package.

## Slice catalogs and ownership

`purrdf.slice.SliceCatalog.discover(root, namespace)` reads manifests using the
caller's namespace. `records` contains `SliceRecord` values with manifest,
manifest path, slice directory and artifact inventory; `core_slice_iris`
reports the core members. `ManifestView` exposes identifier, slice IRI, label,
title, tier, creators and consumers. `ArtifactRecord` exposes role, logical
path, media type, raw digest, semantic digest and content.

`fix_deps()` returns `ManifestPatch` values with original and proposed text;
the caller decides when to write those patches. `OwnershipAnalyzer` performs
the analysis eagerly at construction; `analyze()` returns the retained
`OwnershipReport`, and `analysis_graph_turtle()` serializes its graph.
The report exposes ownership errors, a defect flag and `DependencyEdge` values
with source, destination, semantic/physical status and reconciliation. This is
manifest/artifact ownership, not an inference from directory names. See
[slices](../slices.md).

## Ranked retrieval and exact fusion

`purrdf.retrieval` exposes `plan`, `compile` and `search`, plus cold document
checks and fusion-law design queries. `plan` executes nothing. It records a
canonical request, per-stratum depths, consulted statistics, producer decisions
and unserved terms. `compile` admits that plan and emits its per-stratum SPARQL.
`search` runs the full plan/compile/execute/fuse ladder. There are no separate
Python stream-handle entry points for execute and fuse.

Text producers map a producer IRI to `(stratum, predicate, graph, analyzer)`.
The analyzer requires its `lexicon` choice and declares the documented lexicon
artifacts or entries, scalar guards, accent/stemming/input/projection modes and
substring budgets as needed. A graph selector is `any`, `default` or a named
graph IRI. Optional trailing positions are domains, index attestation and
search fidelity, in that order; an ambiguous or unsupported tuple width raises.
The index attestation is `(generation, incompleteness)`, while fidelity is
`(completeness, order)`. These pairs describe different facts.

`knn_producers` and `hnsw_producers` register exact scan and approximate graph
search over caller-supplied `(IRI, vector)` rows, seeded by an entity request.
Vectors have the same finite width and unique valid IRIs; cosine refuses a
zero vector. Both candidate and neighbor guards are explicit and positive.
The producer metrics are `cosine`, `negative_dot` and `squared_euclidean`.
HNSW additionally declares `(m, m0, ef_construction, ef_search)` and validates
`m >= 2`, `m0 >= m`, `ef_construction >= m0` and `ef_search >= 1`.
A producer IRI or stratum cannot be registered
twice across the three maps. The caller also supplies the statistics `source`
and `revision`; cardinalities and integer parts-per-million selectivities are
optional. Unknown measurements are `None`, not zero.

The request is a sequence of typed tuples; producer applicability is reported
by planning, so writing a request kind does not invent a producer for it.

| Kind | Tuple |
|---|---|
| Lexical | `("lexical", text, language_or_none, predicate_or_none)` |
| Vector | `("vector", components, metric, hint_or_none)`, with request metric `cosine`, `dot` or `euclidean` |
| Spatial | `("spatial", geometry, predicate, max_distance_raw_or_none)` |
| Temporal | `("temporal", predicate, lower_or_none, upper_or_none)` |
| Numeric | `("numeric", predicate, lower_raw_or_none, upper_raw_or_none)` |
| Entity | `("entity", term)`, including an N-Triples IRI term for a vector-space seed |

Candidate domains are either unrestricted (`None`) or one distinct host domain
tag. Empty or multiple distinct tags are refused; repeated copies of one tag
are one domain. A domain declaration licenses exclusion, so a producer naming
a candidate in disjoint domains is an error, not a declaration silently widened
after scores have been certified. Repeated candidates from a unique stream
are also refused.

```python
retrieval = purrdf.retrieval
notes = '''
<https://example.org/a> <https://example.org/note> "the quick brown fox" .
<https://example.org/b> <https://example.org/note> "a quick red fox" .
'''
ranking = retrieval.search(
    notes,
    [("lexical", "quick fox", None, "https://example.org/note")],
    text_producers={
        "https://example.org/pf/search": (
            "https://example.org/stratum/lexical",
            "https://example.org/note", "any", {"lexicon": "empty"},
        )
    },
    weights={"https://example.org/stratum/lexical": retrieval.SCALE},
    statistics={"source": "host-statistics", "revision": "r1"},
    k=60, decay="reciprocal_rank", top_k=10,
)
assert len(ranking["rows"]) == 2
assert isinstance(ranking["rows"][0]["score"], str)
```

### Law, depths and document checks

Weights are Python integers in raw fixed-point units: `SCALE` is one whole
unit and `SCALE_DIGITS` its exponent. Scores and contributions return exact
decimal strings, never floating-point approximations. `k`, `decay` and
`top_k` are explicit. `reciprocal_rank` truncates the reciprocal before
applying weight; `weighted_reciprocal_rank` rounds the weighted quotient once.
They are different content-addressed laws. Mixing a raw weight of `1` with
weights expressed as multiples of `SCALE` changes the weighting by that factor.

`top_k` is also a planning input. Over disjoint candidate domains it can narrow
reads while preserving the answer. A compiled unit's `depth` is the reportable
row bound; its SPARQL `LIMIT` includes one extra probe row. Do not report that
probe as a value or read the bound from the text. `declared_rows=None` means
no row count was declared, not zero. On `compile`, `weights`, `k` and `decay`
are supplied together or omitted together; without a law, no rank-resolution
claim is invented.

`canonical_bytes` and `plan_id` identify a plan. `certify_plan` checks a received
document's recorded depth derivations and consulted-statistics projections;
it does not certify that external statistics are current. `explain_depth`
reads the recorded limiting cause and is not the certification operation.
Branch on `retrieval.PlanDocumentError.refusal` for rejected documents.

The design-time queries are `weight_for_depth`, `class_width`,
`deepest_rank_within_width` and `crossing_rank_at`. They ask respectively for
the minimum separating weight, a rank's equal-contribution class width, the
deepest prefix within a width tolerance, and the head rank that clears a fusion
threshold. `None` means no finite result within the expressible rank domain,
not an enormous measured number. Inputs must satisfy the law's positive
weight/rank/smoothing/tolerance constraints. The width-tolerance query walks
the nonmonotone curve and holds the GIL; use it for design, not in a hot loop.

### Reading ranking evidence

Keep the three identities: `plan_id` names the question and planning inputs,
`profile_id` names the fusion law, and `evidence_id` names the answering index
generations and exclusion evidence. `planned_resolution` describes the depths
before execution; `observed_resolution` reports ranks pulled, collisions,
exclusion lookups and rows materialized. Early certification can make the
observed work smaller; that is not a correction to the plan.

Read each stratum's terminal `statuses` with its `fidelities`. `exhausted`
means its search produced no more rows, not that a lossy search found every
possible match. `depth_reached`, `row_bound_reached` and `ceiling_reached`
name different stopping conditions. The shared vocabulary also includes
`supplied_query_ended`, `execution_failed` and `terms_rejected` for the broader
Rust producer/bundle seam; do not collapse these to an empty-result flag.

`attestations` records index generation and admitted incompleteness;
`fidelities` independently records complete/lossy search and faithful/perturbed
order, with the producer's own reasons. Silence about index completeness does
not certify a whole corpus. HNSW remains a lossy producer even if its graph was
built exactly from the supplied vectors.

`exactness` names deficit, inflation and unbounded strata. Missing candidates
can move other candidates up in rank, so degraded evidence can make scores
too high as well as too low. Each row's `interval` quantifies a bounded error
or names perturbed-order strata; `certain_prefix` counts leading rows whose
places survive the declared degradation. `unemitted_ceiling` bounds what a
candidate outside the answer could be worth, or is `None` when perturbed order
prevents a finite bound. These claims concern membership, not proof that every
unreturned item was absent. Keep `domains` and `exclusion_bases` with the
evidence that licensed skipped streams. See the
[retrieval design](https://github.com/Blackcat-Informatics/purrdf/blob/main/crates/retrieval/README.md).

## Errors and boundary choices

Invalid configuration, resource refusal, expression error and certified partial
results are different outcomes. Preserve structured diagnostic attributes when
provided: `message_id` and `presentation` identify native failures;
`ShapesImportError.kind`, `ShapesProductError.dimension` and
`PlanDocumentError.refusal` are their respective stable discriminants.
Do not parse localized message text to decide whether a result is complete.
An unbound SELECT cell is neither a missing column nor necessarily an exception.

Use the [rdflib compatibility chapter](../interop/rdflib.md) when porting an
rdflib consumer. `query_rdflib` accepts its named/default graph controls, while
the native entry points retain the contracts described here. The separate
`purrdf-rdflib` shadow distribution claims the `rdflib` import name and must not
share an environment with the real rdflib. Direct public calls go through
`purrdf`, never the internal `purrdf_native` extension module.
