<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# SHACL

[`purrdf-shapes`](https://docs.rs/purrdf-shapes) (re-exported as
`purrdf::shapes`) is PurRDF's native SHACL validator: the **complete SHACL
Core feature set** — all constraint components, full property paths,
qualified value shapes, property pairs — plus **SHACL-SPARQL** constraints
and targets and the **SHACL-AF** surface, running entirely on PurRDF's own
interned IR and native SPARQL engine (no oxigraph, no PyO3).

It validates an RDF 1.2 data graph against a SHACL shapes graph with **no
inference** (parity with pySHACL `inference="none"`); combine with
[Entailment](../entailment.md) if you want to validate a materialized closure.

## What it covers

- **SHACL Core** — every constraint component, full property paths, qualified
  value shapes, property pairs. The W3C `data-shapes` suite passes clean
  (126/126, zero ledgered gaps at the time of writing — the live number is in
  [`docs/CONFORMANCE.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/CONFORMANCE.md)).
- **SHACL-SPARQL** — SPARQL-based constraints and targets, custom constraint
  components with pre-binding semantics, user-defined `sh:SPARQLFunction`
  calls, and `sh:SPARQLTargetType`, evaluated on the native SPARQL engine.
- **SHACL-AF** — node expressions (including
  `sh:ExpressionConstraintComponent`) and **SHACL Rules** (`sh:TripleRule` and
  `sh:SPARQLRule`, with `sh:condition`, `sh:order`, `sh:deactivated`): rules
  fire in an iterative fixpoint and the derivation is materialized as a new
  dataset (`base ⊎ derived`), leaving the input graph untouched. Some
  node-expression conveniences (`sh:if`, aggregations, ordering wrappers) are
  DASH/TopBraid conventions with no normative RDF definition; PurRDF documents
  its adopted reading and pins it with a frozen corpus — see the
  [SHACL-AF section of docs/CONFORMANCE.md](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/CONFORMANCE.md#shacl-af-node-expressions-normative-surface-vs-owned-extensions).

## The SHACL 1.2 reifier-shape draft scope

The crate implements a **scoped** SHACL 1.2 Working Draft feature:
`sh:reifierShape` and `sh:reificationRequired` for direct IRI property paths,
so shapes can constrain the RDF 1.2 reifier metadata attached to statements
(see [RDF 1.2 Features](../concepts/rdf12.md)). The relevant SHACL 1.2 Core
draft is dated 2026-06-02. **This is not a claim of full SHACL 1.2
conformance** — it is one draft feature, explicitly scoped and tested.

## Schema → SHACL imports

The schema-projection surface is bidirectional. `SchemaImportConfig` requires
the caller's namespace table and the complete RDF datatype mapping for JSON
scalars; there is no default vocabulary. The five production reverse directions
are JSON Schema draft 2020-12 (`import_json_schema`), native LinkML 1.11
(`import_linkml`), and verified PurRDF-emitted Pydantic v2, TypeScript 7.0, and
GraphQL September 2025 packages (`import_*_package`). All five lower through one
ordered JSON-Schema semantic model and return typed shapes plus an
always-computed, located reverse `LossLedger`.

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

## LinkML 1.11 projection

The same `CompiledSchema` carrier can be projected to canonical LinkML 1.11
with `emit_linkml`. `LinkmlConfig` requires the caller's schema IRI, name,
description, default prefix, and complete prefix map, so PurRDF never mints a
consumer vocabulary or identity. The returned `LinkmlPackage` includes the
typed document, deterministic YAML, a reversible `$defs`-key mapping, and a
located `json-schema` → `linkml-1.11` loss ledger.

Classes and exact property aliases, types, enums, local references, inline
objects, requiredness, homogeneous arrays, patterns, inclusive bounds, and
LinkML boolean expressions are represented directly. Every unsupported
assertion is classified by a closed capability table; malformed inputs,
external/dynamic/dangling references, prefix mistakes, and deterministic-name
collisions fail closed. `parse_linkml` and `write_linkml` preserve all
JSON-compatible metamodel fields and provide byte-stable read/write round trips
while rejecting YAML-only tags, duplicate keys, non-string keys, and non-finite
numbers.

`import_linkml` consumes that validated native document; the emitted-package
variant `import_linkml_package` first verifies canonical YAML and the reversible
element map. Both use the same caller-owned SHACL import configuration.

The Rust production path has no LinkML-toolkit dependency. CI uses the locked
official LinkML 1.11.1 Python packages only as a differential oracle:

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
references, unions, intersections, homogeneous arrays, and bounded tuples.
There are no runtime enums, mergeable interfaces, branded pseudo-validators,
or `any` escape hatches. Invalid keywords, open/dangling references, and name
collisions fail before bytes are emitted.

Runtime assertions outside TypeScript structural assignability are never
silently erased: integer, numeric/string predicate, closure, pattern-property,
dependency, conditional, negation, contains/unique, evaluation-state, and
bounded-expansion gaps receive stable codes and JSON Pointer locations. CI
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
references and aliases, descriptions, and inline object helpers. One global
collision-checked namespace covers types, helpers, and the fallback scalar;
fields and enum symbols are checked in their GraphQL-local namespaces. The
typed/canonical name maps retain the source definition keys, property keys, and
finite JSON values.

`GraphqlPackage::encode_input` maps source JSON keys and finite values to input
field names and enum symbols. `decode_output` performs the inverse for fields
present in a GraphQL response, without inventing omitted selections. Unknown or
incompatible values fail. This package codec is the precise value boundary;
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
| applicators | conditionals, dependencies, intersections, unions, `oneOf`, negation |
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
the name map and production codec, and deliberate corruption failures:

```bash
make graphql-oracle
```

GraphQL.js is dev-only. Emission and value translation remain filesystem-free,
wasm-clean Rust.

## From Python

```python
from purrdf_native import shacl

report = shacl.validate(shapes_ttl="...", data_nt="...")
print(report["conforms"])  # True / False
print(report["results"])   # list of violation dicts
```

Each result dict keeps the stable keys `focus`, `path`, `value`, `severity`,
`component`, `source_shape`, and `message`.

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

let sarif = validate_to_sarif_string(shapes, data, &SarifOptions::default())
    .expect("sarif produced");
assert!(sarif.contains("\"version\": \"2.1.0\""));
```

Lower-level entry points (`build_report_sarif`, `build_diagnostics_sarif`)
build a `SarifLog` value instead of a string, so a host can merge runs before
serializing.

## Conformance

The validator is gated by the vendored W3C `data-shapes` suite, a vendored
DASH SHACL-AF/rules corpus, and a first-party frozen corpus of 69 cases with
byte-frozen expected reports; SHACL Rules output is compared to expected
inferred graphs by RDFC-1.0 isomorphism. See
[Conformance & Testing](../project/conformance.md).
