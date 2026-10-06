<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
-->

# `purrdf-capi` — libpurrdf

The **purrdf semantic C-ABI**: a stable,
SemVer-disciplined `extern "C"` surface over the native `purrdf` RDF-1.2
stack. It is the rich companion to the permissive `libgts` C-ABI — where
`libgts` is transport/format only, **libpurrdf** exposes parse, serialize,
pattern iteration, copy-on-write mutation, SPARQL, SHACL validation/entailment,
and GTS container round-trip.

**One shared library, not two.** libpurrdf statically reuses the permissive
`purrdf-gts` Rust crate, so a language shim links **`libpurrdf` alone** and still
reads/writes `.gts` containers — no second `.so` to coordinate.

## Building

The C library, header, and pkg-config file are produced by
[`cargo-c`](https://github.com/lu-zero/cargo-c):

```sh
make capi-build                 # cargo capi build: libpurrdf.{so,a} + purrdf.h + purrdf.pc
make capi-install PREFIX=/usr   # cargo capi install into a prefix
make capi-check                 # verify the committed header is current + run the C smoke
make capi-header                # regenerate the committed include/purrdf.h after an ABI change
```

The committed header `include/purrdf.h` **is the ABI contract**; CI fails if it
drifts from the crate (`make capi-check`).

`make capi-bundle` produces a native release tarball with the library, header,
pkg-config file and recipient license notices. Its pkg-config paths follow the
extracted bundle: set `PKG_CONFIG_PATH` to its `lib/pkgconfig` directory and use
`pkg-config --cflags --libs purrdf` to compile a consumer. Set the platform's
library search path to the bundle's `lib` directory when running the consumer,
or install the library under your application's normal runtime search path.
The bundle build uses a private Cargo output directory and checks the installed
header and libraries against the outputs reported by that build. The bundle gate
then links and runs the C smoke test from a fresh extraction of the actual tarball,
including the frozen entailment and import fixtures.

## Configured JSON-LD and YAML-LD

`purrdf_jsonld_context_compile` decodes the shared versioned options document
and returns an immutable, thread-safe `PurrdfJsonLdContext`. Reuse that handle
with `purrdf_serialize_jsonld_configured`, then free it with
`purrdf_jsonld_context_free`. The serializer accepts exactly one options byte
slice or compiled handle and preserves an optional caller YAML schema URL.

The three modes are explicit: byte-frozen `expanded`, caller-owned `context`,
and deterministic dataset-IRI `derived`. Context IRI and `@import` resolution is
restricted to the immutable registry in the options document; libpurrdf never
performs network context loading.

## Graph, tabular, dataset-description, and research-object projection carriers

`purrdf_project` and `purrdf_lift` expose the same canonical archive engine as
Rust, Python, WebAssembly, and the CLI. `purrdf_project` accepts all sixteen
profiles; `purrdf_lift` accepts the ten structurally bidirectional ones:

| Profile | Project | Lift |
| --- | :---: | :---: |
| `lpg-csv` | yes | yes |
| `neo4j-csv` | yes | yes |
| `open-cypher` | yes | yes |
| `graphml` | yes | yes |
| `csvw-exact` | yes | yes |
| `csvw-terms` | yes | no |
| `okf-terms` | yes | no |
| `obo-graphs` | yes | no |
| `skos` | yes | no |
| `croissant-1.1` | yes | yes |
| `ro-crate-1.3` | yes | yes |
| `datacite-4.6` | yes | yes |
| `dcat-3` | yes | yes |
| `dcat-rdf` | yes | no |
| `void` | yes | no |
| `frictionless-data-package-1` | yes | yes |

Configuration is mandatory profile-tagged JSON with caller-owned vocabulary,
identity, limits, and policy. Projection returns two independent caller-owned
`PurrdfBuffer` handles: canonical deterministic USTAR bytes and the versioned
loss-ledger JSON. Lift returns a caller-owned `PurrdfDataset` plus an independent
ledger buffer. Free every buffer with `purrdf_buffer_free` and the dataset with
`purrdf_dataset_free`.

`dcat-rdf` and `void` produce a single native RDF description member and have
no inverse. Their strict JSON carries the selected syntax, complete
caller-owned vocabularies and identities, and all mapping/resource policy. The
portable configurations under `crates/rdf/tests/fixtures/dataset-description/`
can be passed directly as `config_json`; `void-source.trig` supplies the matching
example dataset.

`purrdf_project_with_assets` is the attached RO-Crate entry point. It accepts a
canonical payload-only USTAR under the configuration limits and routes it through
the same Rust ownership, byte-size, preview, and determinism checks. The profile
must be `ro-crate-1.3` and its configuration must declare `packaging: "attached"`.

The complete compiled
[`projection_roundtrip.c`](https://github.com/Blackcat-Informatics/purrdf/blob/main/crates/rdf-capi/examples/projection_roundtrip.c)
example parses Turtle, projects LPG CSV, writes the archive, lifts it, verifies
the quad count, and releases every handle. `make capi-check` compiles and
executes that example against the generated shared library and committed header.

## ABI contract (every entry point)

`purrdf_error_presentation_json(error)` returns a borrowed, nullable UTF-8 JSON
record with stable diagnostic message identities, named typed parameters and
exact logical anchors. It remains valid until `purrdf_error_free(error)`. This
accessor is included in the ABI 0.8.0 surface; existing status and
English-message functions retain their signatures.

A SPARQL parse refusal from `purrdf_query`, `purrdf_query_json`,
`purrdf_query_governed`, `purrdf_query_entailment_governed` or
`purrdf_update_governed` (status
`PURRDF_STATUS_QUERY_ERROR`, code `native-sparql-query-parse` or
`native-sparql-update-parse`) carries the parser's condition in the record's
`presentation` object. Its `messageId` is one of `sparql-parse-lex`,
`sparql-parse-syntax`, `sparql-parse-unsupported`, `sparql-parse-iri` or
`sparql-parse-cdt-arity`, and its typed parameters include the byte offset as
`"at":{"kind":"unsigned","value":"5"}`. An IRI refusal that `purrdf-iri`
raised nests that condition under `presentation.detail`, for example
`iri-bad-percent-encoding` with a typed unsigned `offset`, so a C host can read
the IRI failure without parsing English. The error message and the record's
`code`, `message` and other keys are unchanged.

- **No unwinding across the boundary.** Every function runs inside
  `catch_unwind`; a caught panic becomes `PURRDF_STATUS_PANIC` (never a process
  abort across FFI).
- **`int32_t` status + out-params.** Fallible functions return a
  `PurrdfStatus` value (as `int32_t`) and write results through out-pointers. On
- **SemVer-frozen ABI.** The status enum is append-only; new fields/functions are
  additive. The current ABI is **0.9.0 (beta)** — the freeze *discipline* is in
  place, but the version stays pre-1.0 until a real C consumer and the rdflib
  shim exercise it. `purrdf_abi_version` reports it.
- **An incompatible change is declared, not smuggled.** Pre-1.0 the project
  policy (`docs/book/src/project/releases.md`, "Pre-1.0 semver policy") puts
  breaking changes on the **minor** component, so removing, renaming, retyping,
  reordering, or mid-list-inserting a parameter on an exported function bumps
  `PURRDF_ABI_MINOR`. `tests/abi_signatures.rs` snapshots the complete exported
  prototype list against the version triple in
  [`tests/abi_signatures.snapshot`](https://github.com/Blackcat-Informatics/purrdf/blob/main/crates/rdf-capi/tests/abi_signatures.snapshot),
  so a signature edit fails the ordinary `cargo test` gate until the version and
  the snapshot are both updated on purpose. (`make capi-check` only proves the
  header agrees with the source; it cannot see that the contract *moved*.)
  The minor number therefore tracks the exported signatures: any change to a
  parameter list, a return contract, or the documented behaviour of an exported
  symbol bumps it, **additive changes included**, because an additive parameter is
  still a recompile for every C consumer.
  `0.6.0` → `0.7.0` carries exactly four such breaks, bundled into one bump so a
  consumer recompiles once rather than four times for the same reason:
  `purrdf_shacl_validate_to_sarif` and `purrdf_shacl_entail_to_ntriples` gained a
  `shapes_base_iri` parameter between `shapes_ttl` and `data_nt`,
  `purrdf_serialize_jsonld_configured` gained `base_iri` after `media_type` — the
  slot it already occupies on `purrdf_serialize` — and `purrdf_serialize` itself
  gained `out_directional_literals_dropped` and `out_named_graph_rows_dropped`
  before `out_error`. Recompile against the new header; there is no `_v2` alias,
  because two entry points for one job is the duplication this library exists to
  avoid.
  `0.7.0` → `0.8.0` adds eight prepared-shapes-product entry points,
  `purrdf_shacl_validate_changes_to_sarif` (the SHACL change path) and the three
  shapes-graph tools (`purrdf_shacl_apply_rules`, `purrdf_shacl_eval_node_expr`,
  `purrdf_shacl_lint_shapes`), appends one
  status discriminant, and carries four breaks: `purrdf_shacl_validate_to_sarif`
  gained `conformance_disallows` / `conformance_disallows_count` (the SHACL 1.2
  conformance-disallow set; count `0` is the default set) between `data_nt` and
  `out_buffer`, and `purrdf_entail_certain_answers`, `purrdf_entail_graph_entails`
  and `purrdf_entail_verify_entailment` gained `premise_iris` /
  `premise_iri_count` (the IRIs the premise document was read from; count `0` is
  bare text) between `import_count` and `out_answer`, so a `0.7.0` host recompiles.
  The same bump gives every shapes-graph entry point — `purrdf_shacl_validate_to_sarif`,
  `purrdf_shacl_validate_changes_to_sarif`, `purrdf_shacl_entail_to_ntriples`, the three
  shapes-graph tools and `purrdf_shapes_product_encode` — the shapes graph's
  `owl:imports` table, `import_iris` / `import_documents` / `import_count`, before its
  out-parameters; appends `PURRDF_STATUS_SHAPES_IMPORT_ERROR`; and adds its three
  accessors (see [The shapes graph's `owl:imports`](#the-shapes-graphs-owlimports)).
  The same bump gives `purrdf_shacl_apply_rules` the nullable `max_stored_facts` /
  `max_join_steps` limits between `max_generated_terms` and `import_iris`, and
  `purrdf_entail_materialize_to_nquads` the same two between `program` and
  `out_nquads`, and gives `purrdf_shacl_entail_to_ntriples` all four of
  `purrdf_shacl_apply_rules`' limits (`max_term_generating_rounds`,
  `max_generated_terms`, `max_stored_facts`, `max_join_steps`) between
  `import_count` and `out_buffer`, and both rules entry points a nullable
  `out_diagnostics` before `out_error`. It also adds `purrdf_shacl_check_rules` and its
  `PurrdfSrlCheckLevel` discriminant, and gives `purrdf_shacl_validate_to_sarif`,
  `purrdf_shacl_validate_changes_to_sarif`, `purrdf_shacl_lint_shapes` and
  `purrdf_shapes_product_encode` a nullable `shapes_graph_iri` immediately after
  `shapes_base_iri` (see [The shapes-graph IRI](#the-shapes-graph-iri)), as it does
  the two rules entry points `purrdf_shacl_apply_rules` and
  `purrdf_shacl_entail_to_ntriples`, and
  `purrdf_shacl_validate_to_sarif` a `bool subclass_of_in_shapes_graph` between
  `import_count` and `out_buffer` (see
  [`subClassOfInShapesGraph`](#subclassofinshapesgraph)).
  It bumps in any case because `0.7.0` is the ABI of the released
  `2.0.x` libraries, which export twelve fewer symbols — leaving the triple still
  would have two shippable libraries answering `purrdf_abi_version` identically
  while offering different surfaces, and telling a host they agree right before it
  fails to resolve a symbol is the one thing this number exists to prevent.
  `0.8.0` → `0.9.0` adds symbols and one status and changes no existing prototype:
  `purrdf_serialize_empty_named_graphs_dropped(dataset, media_type, out_count,
  out_error)`, the number of declared empty named graphs a whole-dataset
  `purrdf_serialize` to that target drops (N-Quads, HexTuples and the single-graph
  syntaxes cannot write a graph that holds no row); the fourteen `*_xpath_regex` entry
  points and the appended `PURRDF_STATUS_REGEX_RESOURCE_ERROR` (see
  [Dated regular-expression laws](#dated-regular-expression-laws)). `purrdf_serialize`
  and every other existing entry point keep their prototypes and behaviour. It bumps
  because `0.8.0` is the ABI of the released `3.0.x` libraries, which do not export
  these symbols.

## Shapes-graph tools

Beside validation, four entry points reach the same engine every other PurRDF host
does. Each takes the shapes graph as Turtle and the data graph as N-Triples.

- `purrdf_shacl_apply_rules(data_nt, shapes_ttl, shapes_base_iri, shapes_graph_iri, srl, srl_base_iri,
  max_term_generating_rounds, max_generated_terms, max_stored_facts, max_join_steps,
  import_iris, import_documents, import_count, out_inferred, out_proof, out_diagnostics,
  out_error)`
  runs exactly one
  rule source — the SHACL 1.2 rules of `shapes_ttl`, or the SPARQL 1.2 RL rule set
  `srl` — and writes the **inference graph** (the inferred triples only, never the
  data graph) as canonical N-Triples. A non-NULL `out_proof` also receives the proof
  of every inferred triple. `max_term_generating_rounds` and `max_generated_terms`
  are nullable `uint64_t *` limits on the rounds that infer a new term and on the
  terms inferred beyond the input's. NULL keeps the default: 16384 rounds, and
  `max(65536, 4 × N)` terms for `N` distinct input terms. A run past either fails
  naming the limit, the numbers, the rules that inferred a new term last, and the
  parameter that raises it. `max_stored_facts` and `max_join_steps` are nullable
  `uint64_t *` limits on the facts the evaluation store holds and the candidate
  solutions the rule bodies enumerate; NULL keeps the default of 4194304 facts and
  1048576 join steps. A run past either fails naming the limit, the numbers and
  the parameter. `purrdf_entail_materialize_to_nquads(document, regime, program,
  max_stored_facts, max_join_steps, out_nquads, out_report, out_error)` takes the
  same two limits for the `rdf`, `rdfs`, `owl-rl` and `d` regimes.
  `purrdf_shacl_entail_to_ntriples(shapes_ttl, shapes_base_iri, shapes_graph_iri,
  data_nt, import_iris, import_documents, import_count, max_term_generating_rounds,
  max_generated_terms, max_stored_facts, max_join_steps, out_buffer, out_diagnostics,
  out_error)` — SHACL-AF entailment, the base graph plus every inference — takes all
  four, with the same defaults, and a refusal names `purrdf_shacl_entail_to_ntriples`'
  own parameter.
- **Mandatory diagnostics on every run.** A shape whose `sh:in` or `sh:xone` list is
  empty (SHACL 1.2 Core Appendix A, "Each such list SHOULD have at least one member")
  is reported by every run, beside its outcome and never inside it: the SARIF log of
  `purrdf_shacl_validate_to_sarif` and its change and product twins carries each as a
  `level: "note"` notification in `invocations[0].toolExecutionNotifications`, whose
  `descriptor` names the rule (`in-minListLength`, `xone-minListLength`) in
  `tool.driver.notifications`; `purrdf_shacl_apply_rules` and
  `purrdf_shacl_entail_to_ntriples` write one `diagnostic RULE SHAPE` line per
  diagnostic to a non-NULL `out_diagnostics` (an empty buffer when there is none).
  The verdict, the results and the report graph are unchanged.
- `purrdf_shacl_check_rules(srl, srl_base_iri, level, import_iris,
  import_documents, import_count, out_summary, out_error)` checks a SPARQL 1.2 RL
  rule set WITHOUT evaluating it — the grammar, the `IMPORTS` closure resolved
  from the table exactly as `purrdf_shacl_apply_rules` resolves it,
  well-formedness and stratification — and writes the one-line summary every host
  reports. `level` is a `PurrdfSrlCheckLevel`: `PURRDF_SRL_CHECK_LEVEL_SYNTAX`,
  `PURRDF_SRL_CHECK_LEVEL_WELL_FORMED` or `PURRDF_SRL_CHECK_LEVEL_STRATIFIED`
  (every static check a rules run applies before it evaluates), each including
  the ones before it; any other value is a `ParseError`. A refused rule set is a
  `ParseError` naming the stage, and `*out_summary` is left untouched.
- `purrdf_shacl_eval_node_expr(shapes_ttl, shapes_base_iri, data_nt, expr,
  expr_at, expr_via, expr_via_count, expr_turtle, focus, scope, scope_count,
  import_iris, import_documents, import_count, out_terms, out_error)` evaluates
  one node expression of the shapes graph. Exactly one selector is non-NULL:
  `expr` is an IRI or `_:label`; `expr_at` with `expr_via` walks from a named
  node, each predicate reaching exactly one value, to an anonymous `[ … ]`
  expression; `expr_turtle` is the expression inline as Turtle, whose one root
  blank node is the expression. `focus` is an IRI or an N-Triples term, and each
  `scope` entry a `NAME=TERM` binding. The output nodes come back one N-Triples
  term per line, in sequence order.
- `purrdf_shacl_lint_shapes(shapes_ttl, shapes_base_iri, shapes_graph_iri, import_iris,
  import_documents, import_count, out_report, out_clean, out_findings, out_error)`
  certifies a shapes graph — its whole `owl:imports` closure: the loader's verdict, the
  W3C `shacl-shacl.ttl` results, which implementation every function call binds to, and
  every validator declared for a built-in component (superseded by the native
  implementation, never run).
  It writes the same deterministic report text as `purrdf shapes lint`. A malformed
  shapes graph is a report with findings and `*out_clean == 0`, not an error; a
  closure that is not in hand is `PURRDF_STATUS_SHAPES_IMPORT_ERROR` and no report.

## The shapes graph's `owl:imports`

Every entry point that takes a Turtle shapes graph also takes the shapes graph's
`owl:imports` table, as `import_iris` / `import_documents` / `import_count` — the
parallel-array convention the `purrdf_entail_*` services use. Entry `i` declares that
the ontology IRI `import_iris[i]` names the Turtle document `import_documents[i]`,
parsed with that IRI as its base; `import_count == 0` (the arrays may then be NULL) is
the empty table. PurRDF fetches nothing.

An `owl:imports` triple is an import only when its subject is the shapes graph's own IRI
(`shapes_base_iri`, or the document's own `@base`; for an imported document, the IRI it
was imported by), a node the document types `owl:Ontology`, every `sh:ShapesGraph` it
declares (`sh:RulesGraph` and subclasses included), or a node naming one of those as its
`owl:versionIRI`. On any other node — a node that is only a `sh:DataGraph`, and SHACL's
`sh:prefixes/owl:imports*/sh:declare` prefix edges, among them — it is data, and
`purrdf_shacl_lint_shapes` lists it under `unanchored-imports`. An import `<X>` is resolved
by a table entry for `<X>`, by `shapes_base_iri` (or the document's own `@base`) being
`<X>`, or by the closure declaring it (`<X> a owl:Ontology`, `<X> a sh:ShapesGraph`, or an
ontology whose `owl:versionIRI` is `<X>`).
Anything else —
and a table entry no import names — fails the call with
`PURRDF_STATUS_SHAPES_IMPORT_ERROR`: the same refusal the Rust API, the `purrdf` command
line, Python and WebAssembly raise for the same shapes graph, rather than a verdict about
a smaller shapes graph than the one named. `purrdf_shapes_import_error_kind(err)` reads
its kind (`unresolved-import`, `unreached-import`, `incompatible-import-versions` — the
closure holds two versions of one series, or a graph another declares
`owl:incompatibleWith` — or `invalid-import`), and
`purrdf_shapes_import_error_iri_count` / `purrdf_shapes_import_error_iri` the IRIs it
names. A data graph's `sh:shapesGraph` links (SHACL 1.2 Core section 6.4) are resolved
through the same `import_iris` / `import_documents` table and unioned into the shapes
graph: a link on the data graph's `sh:DataGraph` node that nothing resolves is kind
`unresolved-shapes-graph-link`, a link a prepared product does not hold is
`unheld-shapes-graph-link`, and a link value that is not an IRI is
`invalid-shapes-graph-link`. A `sh:shapesGraph` on any other node is data. An imported document's shapes, rules and functions take part exactly as the
importing document's do, and a prepared product carries the merged closure.

## The shapes-graph IRI

`purrdf_shacl_validate_to_sarif`, `purrdf_shacl_validate_changes_to_sarif`,
`purrdf_shacl_lint_shapes`, `purrdf_shapes_product_encode` and the two rules entry
points, `purrdf_shacl_apply_rules` and `purrdf_shacl_entail_to_ntriples`, take a
nullable `shapes_graph_iri` right after `shapes_base_iri`: the IRI SHACL-SPARQL sees
the shapes graph under, which `purrdf validate --shapes-graph` and `purrdf rules
--shapes-graph` name. A `sh:SPARQLRule` sees it too. On `purrdf_shacl_apply_rules`
it is a `ParseError` beside `srl`: a SPARQL 1.2 RL rule set has no shapes graph. `$shapesGraph` is
pre-bound to it and `GRAPH $shapesGraph { … }` reads the shapes graph. That is SHACL
1.0's pre-binding, which SHACL 1.2 removed. NULL names no graph, and `$shapesGraph`
is then an ordinary variable. A relative IRI resolves against `shapes_base_iri`; one
with no base is a `ParseError` (`iri-relative-no-base`). A product records the IRI
and its identity binds it, so a restore exposes the shapes graph under it.

## `subClassOfInShapesGraph`

`purrdf_shacl_validate_to_sarif` takes `bool subclass_of_in_shapes_graph` after the
import table: SHACL 1.2 Core §6.3's parameter of that name. `true` reads the shapes
graph's `rdfs:subClassOf` triples, in addition to the data graph's, wherever SHACL type
decides class membership (`sh:targetClass`, implicit class targets, `sh:class`,
`sh:rootClass`, `shnex:instancesOf`). `false` is the specification's default, the data
graph alone. Only class membership changes: the shapes graph's triples do not become
data-graph triples, and `rdf:type` triples are always read from the data graph.

## Base IRIs across the surface

Every entry point that reads or writes an RDF **syntax admitting a relative IRI**
takes a nullable base, in the slot beside the document it qualifies:
`purrdf_parse`, `purrdf_serialize`, `purrdf_serialize_jsonld_configured`, the
SHACL entry points' `shapes_base_iri` (and `purrdf_shacl_apply_rules`' `srl_base_iri`),
and the SPARQL request base on `purrdf_query`,
`purrdf_query_json`, `purrdf_query_governed`,
`purrdf_query_entailment_governed`, and `purrdf_update_governed`.

The entry points that take none take none *because the grammar admits none*, not
because the parameter was dropped. The `purrdf_entail_*` and `purrdf_reasoner_*`
documents are parsed as N-Quads (which accepts N-Triples); `purrdf_term_to_ntriples`
emits N-Triples. Both rows carry `admits_relative_iri: false` / `emits_base: false`
in the format registry, so a base could only be ignored. A C host holding Turtle or
RDF/XML reaches those services through `purrdf_parse`, which does take a base.
`purrdf_to_gts` / `purrdf_from_gts` move a binary CBOR container whose terms are
already absolute, and the `purrdf_graph_*` mutators take structured term views
whose IRIs are absolute by contract.

Passing a base to a syntax that cannot express one changes nothing in the bytes;
passing one that is not an absolute IRI is a hard `PURRDF_STATUS_SERIALIZE_ERROR`
for **every** format, including those that would not have applied it.

### Status codes

| Code | Value | Meaning |
|------|-------|---------|
| `PURRDF_STATUS_OK` | 0 | success |
| `PURRDF_STATUS_NULL_POINTER` | 1 | a required pointer was null |
| `PURRDF_STATUS_INVALID_UTF8` | 2 | a C string was not valid UTF-8 |
| `PURRDF_STATUS_INVALID_ARGUMENT` | 3 | a structurally invalid argument |
| `PURRDF_STATUS_UNSUPPORTED_FORMAT` | 4 | unknown media type / format id |
| `PURRDF_STATUS_PARSE_ERROR` | 5 | parse failed |
| `PURRDF_STATUS_SERIALIZE_ERROR` | 6 | serialize failed |
| `PURRDF_STATUS_QUERY_ERROR` | 7 | SPARQL evaluation failed |
| `PURRDF_STATUS_FREEZE_ERROR` | 8 | freezing a mutable graph failed |
| `PURRDF_STATUS_CURSOR_EXHAUSTED` | 9 | no more rows (a non-error terminal signal, `> 0`) |
| `PURRDF_STATUS_GTS_ERROR` | 10 | GTS container read/write failed |
| `PURRDF_STATUS_SHAPES_PRODUCT_ERROR` | 11 | the prepared-shapes-product boundary refused; read `purrdf_shapes_product_error_dimension` |
| `PURRDF_STATUS_SHAPES_IMPORT_ERROR` | 12 | a shapes graph's `owl:imports` closure is not in hand, or its import table cannot be used; read `purrdf_shapes_import_error_kind` |
| `PURRDF_STATUS_REGEX_RESOURCE_ERROR` | 13 | a selected dated XPath regular-expression law withheld a resource; the message names it (`xpath-pattern-bytes`, `xpath-match-steps`, ...) |
| `PURRDF_STATUS_PANIC` | 100 | a panic was caught at the boundary |

## Dated regular-expression laws

Every entry point that evaluates SPARQL `REGEX`/`REPLACE` or SHACL `sh:pattern` has a
`*_xpath_regex` twin that takes one more argument, a nullable NUL-terminated
`regex_profile`, before its out-parameters (before `governors` on the governed ones):

| Entry point | Twin |
| --- | --- |
| `purrdf_query` | `purrdf_query_xpath_regex` |
| `purrdf_query_json` | `purrdf_query_json_xpath_regex` |
| `purrdf_query_governed` | `purrdf_query_governed_xpath_regex` |
| `purrdf_query_entailment_governed` | `purrdf_query_entailment_governed_xpath_regex` |
| `purrdf_update_governed` | `purrdf_update_governed_xpath_regex` |
| `purrdf_shacl_validate_to_sarif` | `purrdf_shacl_validate_to_sarif_xpath_regex` |
| `purrdf_shacl_validate_changes_to_sarif` | `purrdf_shacl_validate_changes_to_sarif_xpath_regex` |
| `purrdf_shapes_product_admit` | `purrdf_shapes_product_admit_xpath_regex` |
| `purrdf_shapes_product_admit_expecting` | `purrdf_shapes_product_admit_expecting_xpath_regex` |
| `purrdf_shapes_product_rebuild` | `purrdf_shapes_product_rebuild_xpath_regex` |
| `purrdf_shapes_product_rebuild_expecting` | `purrdf_shapes_product_rebuild_expecting_xpath_regex` |
| `purrdf_shacl_entail_to_ntriples` | `purrdf_shacl_entail_to_ntriples_xpath_regex` |
| `purrdf_shacl_apply_rules` | `purrdf_shacl_apply_rules_xpath_regex` |
| `purrdf_shacl_eval_node_expr` | `purrdf_shacl_eval_node_expr_xpath_regex` |

`regex_profile` names the law exactly:

- `xpath-2.0-2010-12-14` — XPath and XQuery Functions and Operators 2.0 (Second
  Edition, 14 December 2010);
- `xpath-3.1-2017-03-21` — XPath and XQuery Functions and Operators 3.1 (21 March 2017).

NULL is the entry point without the suffix, byte for byte: the compatibility regular
expressions. There is no default law and no alias, so any other name — `xpath-3.1`,
`XPATH-3.1-2017-03-21`, the empty string — is `PURRDF_STATUS_INVALID_ARGUMENT`, and
the message lists the accepted names. A selected law runs under the production native
limits (a 64 KiB pattern source, and finite compile, match and output bounds).

What the law decides is the language: `(?:a)b` is a non-capturing group under 3.1 and
an invalid pattern under 2.0, and `^(a)\1$` matches `"aa"` under both dated laws,
while the compatibility expressions refuse back-references. A pattern the selected law
does not admit is reported the way an invalid pattern always is — a SPARQL expression
error, so a `FILTER` keeps no row and a `BIND` leaves its variable unbound, and a SHACL
result in the SARIF log. A pattern or input the law cannot process within its limits
is different: the call fails with `PURRDF_STATUS_REGEX_RESOURCE_ERROR` and writes no
result, answer or report, so a refusal is never read as an empty answer, a `false`
or a conforming graph. For SPARQL the error's presentation record carries the
resource's code. The governors of a governed call are independent of these limits.

On the shapes-graph tools the law decides every pattern a run evaluates: on
`purrdf_shacl_entail_to_ntriples_xpath_regex` and `purrdf_shacl_apply_rules_xpath_regex`
each `REGEX`/`REPLACE` of a `sh:SPARQLRule`, a SHACL-AF function, a node expression or a
SPARQL 1.2 RL filter or assignment, and each `sh:pattern` of a rule condition; on
`purrdf_shacl_eval_node_expr_xpath_regex` each `sh:pattern` of a filter shape and each
`REGEX`/`REPLACE` of a function call or SPARQL-based expression. A refusal is
`PURRDF_STATUS_REGEX_RESOURCE_ERROR` with no dataset, inference or output written.
`purrdf_shacl_lint_shapes` has no twin: it certifies a shapes graph without compiling or
matching any of its patterns. The entailment services (`purrdf_entail_*`) take no law.

## Governed execution

`purrdf_query_governed` returns either a complete query result or a typed exhausted
outcome with governor evidence and a partial-answer certificate.
`purrdf_query_entailment_governed` additionally keeps the closure-phase report or stop
beside that query outcome. Budget exhaustion is not a query error: depending on the
certificate, the call may still populate the `PurrdfRowCursor` or graph-result
out-parameter. Every populated row cursor must be released with `purrdf_rowcursor_free`,
every graph dataset with `purrdf_dataset_free`, and every report buffer with
`purrdf_buffer_free`, on complete and exhausted paths alike.

## Ownership

- Every handle / buffer / error / cursor the library hands out has **exactly one
  matching `*_free`** (`purrdf_dataset_free`, `purrdf_graph_free`,
  `purrdf_cursor_free`, `purrdf_rowcursor_free`, `purrdf_buffer_free`,
  `purrdf_error_free`). Free each exactly once; freeing `NULL` is a no-op.
- **The C side never `free()`s a `PurrdfStr.ptr`.** A `PurrdfStr` borrows
  library-owned memory; copy the bytes out if you need them to outlive the
  borrow.

## Lifetimes (borrowed slices)

- A term view from `purrdf_cursor_next` borrows into the dataset arena; its
  `PurrdfStr` pointers are valid until the next `purrdf_cursor_next` on that
  cursor or `purrdf_cursor_free`. The cursor pins the dataset's `Arc`, so it
  stays valid even after every `PurrdfDataset` handle is freed. Pattern rows
  are pulled lazily from the selected core index rather than collected when
  the cursor opens.
- A term view from `purrdf_rowcursor_term` borrows into the current row's owned
  value; valid until the next `purrdf_rowcursor_next` or `purrdf_rowcursor_free`.
- A buffer's bytes (`purrdf_buffer_data`) are valid until `purrdf_buffer_free`.
- An error message (`purrdf_error_message`) is valid until `purrdf_error_free`.

## Thread-safety (per handle)

| Handle | Safety |
|--------|--------|
| `PurrdfDataset` | `Send + Sync` — frozen; may be read concurrently from many threads |
| `PurrdfJsonLdContext` | `Send + Sync` — immutable compiled context; may be reused concurrently |
| `PurrdfGraph` | single-threaded mutable (COW delta); external locking required to share |
| `PurrdfCursor` / `PurrdfRowCursor` | single-threaded |
| `PurrdfReasoner` | `Send`, **not `Sync`** — answering mutates the shared knowledge base, so one handle may move between threads but not be used by two at once; open one per thread |
| `PurrdfCancellation` | `Send + Sync` — a shared monotone cancellation bit; one thread may cancel while another runs a governed call |
| `PurrdfBuffer` / `PurrdfError` | immutable once returned; read from any thread, free once |

## Term crossing

Three representations are offered (per-row N-Triples reparse is **not** the only
path): structured borrowed term views (`PurrdfTermView`), a cursor-scoped opaque
`term_id` for re-addressing a term (notably a quoted triple, whose components do
not fit a flat view), and the `purrdf_term_to_ntriples` convenience function.

## GTS star-layer round-trip

The GTS **star layer** round-trip (`purrdf_to_gts` → `purrdf_from_gts` of a
dataset containing quoted triples / reifier bindings) succeeds with
`PURRDF_STATUS_OK`, the same as a star-free round-trip. The C ABI calls the
canonical kernel `to_gts` → `read_graph` → `import_gts_graph` path; the
reifier-binding gap that once dropped those rows on read-back is closed. A
characterization test, `gts_star_roundtrip_preserves_the_statement_layer` in
`tests/abi.rs`, pins the restored dataset's quoted triple and reifier
binding so a regression in either layer fails there.

## License

Licensed under any one of the following, at your option:

- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [Mulan Permissive Software License, Version 2 (MulanPSL-2.0)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MULAN)

The `purrdf-gts` I/O core this layer statically reuses carries the same offer and
remains independently usable under it.
