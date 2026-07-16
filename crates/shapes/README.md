<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg" alt="PurRDF logo" width="120" height="120">
  </a>
</p>

<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# `purrdf-shapes` — Rust SHACL Core Validator

[![crates.io](https://img.shields.io/crates/v/purrdf-shapes.svg)](https://crates.io/crates/purrdf-shapes)
[![docs.rs](https://docs.rs/purrdf-shapes/badge.svg)](https://docs.rs/purrdf-shapes)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
[![Repository](https://img.shields.io/badge/repo-Blackcat--Informatics%2Fpurrdf-181717.svg)](https://github.com/Blackcat-Informatics/purrdf)

> **An LLM output is a claim, not a truth.**

`purrdf-shapes` is the native SHACL validator of the PurRDF toolkit — the
complete SHACL Core feature set plus SHACL-SPARQL constraints and targets,
running entirely on PurRDF's own interned IR and native SPARQL engine (no
oxigraph, no PyO3). It validates an RDF 1.2 data graph against a SHACL shapes
graph with no inference (parity with pySHACL `inference="none"`).

In four-box terms, the data graph is usually the ABox, the shapes graph is a
TBox/RBox validation surface, and RDF 1.2 reifier metadata is the CBox. The
crate preserves existing report keys while adding optional box-role metadata for
callers that want richer diagnostics.

The crate implements a scoped SHACL 1.2 Working Draft feature:
`sh:reifierShape` and `sh:reificationRequired` for direct IRI property paths.
The relevant SHACL 1.2 Core draft is dated 2026-06-02. This is not a claim of
full SHACL 1.2 conformance.

The Python SHACL surface is exposed from `bindings/python` as part of the
`purrdf_native` extension. The engine core (`engine.rs`, `shapes.rs`,
`constraints.rs`, `path.rs`, `report.rs`, `model.rs`) is deliberately
**PyO3-free** — it links as a plain `rlib` into any Rust consumer without
any Python dependency.

This crate is gated by a SHACL conformance corpus.

---

## Build

> **Toolchain:** stable Rust (the repo ships a `rust-toolchain.toml` at the
> root pinning `stable`; the MSRV floor is `rust-version` in the workspace
> `Cargo.toml`). `cargo` and `rustup` pick this up automatically.

```bash
cargo build -p purrdf-shapes
```

## Test

```bash
cargo test -p purrdf-shapes
```

## Pydantic v2 packages

The Rust emitter consumes the same in-memory `CompiledSchema` produced by the
SHACL-to-JSON-Schema compiler. It returns deterministic package bytes and never
reads or writes the filesystem. Package identity and all module docstrings are
required caller input:

```rust,ignore
use purrdf_shapes::{PydanticConfig, emit_pydantic};

let config = PydanticConfig::new(
    "example_models",
    "Models for the caller's public package.",
    "Generated validation models for the caller's schema.",
)?;
let package = emit_pydantic(&compiled_schema, &config)?;

assert!(package.artifacts.contains_key("example_models/models.py"));
assert_eq!(
    package.model_paths.get("Person").map(String::as_str),
    Some("example_models.models.Person"),
);
```

Generated classes validate the representable JSON Schema subset and expose the
originating definition through Pydantic v2's standard
`model_json_schema(by_alias=True)` surface. Assertions without an exact
Pydantic runtime annotation remain in that schema and are recorded, with JSON
pointer locations, in `package.losses`; the source SHACL-to-JSON-Schema losses
remain separately available on `CompiledSchema::losses`. The checked
`json-schema` → `pydantic-v2` loss profile makes every widening auditable.

The dev-only executable oracle imports an emitted package, compares its live
`model_json_schema()` output with the source definitions, and probes validation
and alias round trips:

```bash
make pydantic-oracle
```

## LinkML 1.11 schemas

The LinkML emitter projects `CompiledSchema` directly to one fixed LinkML
metamodel dialect (`metamodel_version: 1.11.0`). It is an in-memory Rust API:
PurRDF does not shell out, select behavior with a feature flag, read a repository,
or invent a schema identity. The caller supplies the schema IRI, name,
description, default prefix, and complete prefix map; there is deliberately no
`Default` configuration.

```rust,ignore
use std::collections::BTreeMap;
use purrdf_shapes::{LinkmlConfig, emit_linkml, parse_linkml, write_linkml};

let config = LinkmlConfig::new(
    "https://example.org/schema/linkml",
    "Example-Schema",
    "Schema documentation owned by the caller.",
    "ex",
    BTreeMap::from([
        ("ex".into(), "https://example.org/".into()),
        ("linkml".into(), "https://w3id.org/linkml/".into()),
    ]),
)?;
let package = emit_linkml(&compiled_schema, &config)?;

let parsed = parse_linkml(&package.yaml)?;
assert_eq!(write_linkml(&parsed)?, package.yaml);
assert_eq!(
    package.element_names.get("Person").map(String::as_str),
    Some("Person"),
);
```

`LinkmlPackage` returns the validated document tree, canonical YAML, a
reversible source-`$defs`-key to LinkML-element map, and the always-computed
`json-schema` → `linkml-1.11` loss ledger. The YAML writer sorts every mapping
and emits exactly one trailing newline. The reader preserves every
JSON-compatible LinkML field, including metamodel extensions PurRDF does not
author, while rejecting duplicate keys, YAML tags, non-string mapping keys,
non-finite numbers, and resource-limit violations. Thus read → write and write
→ read → write are stable without pretending YAML-only semantics can cross a
language-neutral boundary.

The projection grammar is:

| JSON Schema input | LinkML 1.11 representation |
|---|---|
| object `$defs` | named `classes` with caller-prefix-derived `class_uri` |
| scalar `$defs` | named `types` |
| string and `{"@id": ...}` enum `$defs` | named `enums` and `permissible_values` |
| `properties` | class-scoped `attributes`; the exact JSON key remains the attribute key and `alias` |
| local `$ref` | class, type, or enum `range`; class aliases use `is_a` |
| inline object | deterministic synthesized class with `inlined: true` |
| `required` | attribute `required` |
| homogeneous array | `multivalued`, item range, ordered/unique flags, and min/max cardinality |
| `anyOf` / `allOf` / `oneOf` / `not` | `any_of` / `all_of` / `exactly_one_of` / `none_of` |
| `pattern`, `minimum`, `maximum` | `pattern`, `minimum_value`, `maximum_value` |
| `additionalProperties` | LinkML 1.11 `extra_slots`, including a typed `range_expression` |
| titles and descriptions | the corresponding LinkML element fields |

Every schema assertion passes through the same closed capability audit used by
the renderer. A construct that LinkML cannot express is never silently ignored:
it produces a stable code and JSON Pointer location. The profile covers
conditional/dependency/contains/unevaluated rules, tuple widening, string and
property counts, exclusive and multiple-of bounds, format differences,
non-scalar enum carriers, and a closed catch-all. Malformed values,
external/dynamic/dangling references, inconsistent `required` declarations,
unknown caller prefixes, reserved names, and normalized-name collisions hard
fail instead of entering the ledger. Source SHACL → JSON Schema losses remain
separate on `CompiledSchema::losses`.

The production emitter has no Python dependency and remains wasm-clean. The
dev-only oracle pins the official `linkml` and `linkml-runtime` packages to
1.11.1, loads the emitted YAML through `SchemaDefinition` and `SchemaView`,
regenerates JSON Schema, compares the exact fixture's `$defs` and accept/reject
corpus, and probes every lossy family:

```bash
make linkml-oracle
```

## TypeScript 7.0 declarations

`emit_typescript` projects `CompiledSchema` directly into one deterministic
`index.d.ts` artifact. `TypeScriptConfig` requires the caller's package name and
package/module prose; no downstream identity or vocabulary is fabricated. The
result includes the declaration bytes, an exact source-`$defs`-key to exported
type-name map, and the always-computed `json-schema` → `typescript-7.0` loss
ledger.

```rust,ignore
use purrdf_shapes::{
    TYPESCRIPT_DECLARATION_PATH, TypeScriptConfig, emit_typescript,
};

let config = TypeScriptConfig::new(
    "example-schema-types",
    "Types published by the caller.",
    "Declarations generated from the caller's compiled schema.",
)?;
let package = emit_typescript(&compiled_schema, &config)?;
let declaration = std::str::from_utf8(
    &package.artifacts[TYPESCRIPT_DECLARATION_PATH],
)?;

assert!(declaration.contains("export type Person"));
assert_eq!(package.type_names.get("Person").map(String::as_str), Some("Person"));
```

The closed dialect is TypeScript 7.0 under `strict` and
`exactOptionalPropertyTypes`. It represents JSON primitives and literals, exact
required-versus-optional fields, explicit JSON `null`, local recursive
references, `anyOf` unions, `allOf` intersections, homogeneous arrays, and
bounded tuples. It emits type aliases rather than runtime enums or mergeable
interfaces and never uses `any`. Malformed keyword values, external/dynamic or
dangling references, reserved names, and normalized-name collisions fail
closed.

TypeScript's structural assignability cannot encode every runtime JSON Schema
assertion. Integer subsets, numeric/string predicates, object closure with
named fields, regex-selected properties, dependencies, conditionals,
negation, contains/uniqueness, evaluation state, and expansion-budget
widenings are therefore classified at their JSON Pointer locations by the
closed loss profile. The compiler oracle checks both fresh literals and values
passed through variables, so excess-property checks cannot hide structural
widening:

```bash
make typescript-oracle
```

There is deliberately no general TypeScript → JSON Schema reader. Arbitrary
TypeScript declarations have no unique runtime acceptance relation, and many
different schemas project to the same declaration. The retained
`CompiledSchema`, paired with `type_names`, is the authoritative reverse
surface. The production emitter has no JavaScript dependency, performs no
filesystem I/O, and remains wasm-clean; TypeScript is dev-only oracle tooling.

## GraphQL September 2025 SDL

`emit_graphql` projects `CompiledSchema` into the fixed
`graphql-september-2025` dialect. `GraphqlConfig` requires the caller's schema
name, package and module prose, and non-built-in fallback-scalar name; it has no
`Default` implementation. The output is a deterministic type-system fragment,
not an executable GraphQL service: PurRDF emits no operation roots, resolvers,
pagination, authorization, federation, or application directives.

The compiling [`graphql_package` example](./examples/graphql_package.rs) shows
configuration, artifact access, the typed name map, and the value codec:

```rust,ignore
use purrdf_shapes::{
    GRAPHQL_NAME_MAP_PATH, GRAPHQL_SCHEMA_PATH, GraphqlConfig, emit_graphql,
};

let config = GraphqlConfig::new(
    "ExampleSchema",
    "GraphQL schema package owned by the caller.",
    "Types generated from the caller's compiled schema.",
    "JsonCarrier",
)?;
let package = emit_graphql(&compiled_schema, &config)?;
let graphql_value = package.encode_input("Person", &source_json)?;
let source_value = package.decode_output("Person", &graphql_value)?;

let sdl = &package.artifacts[GRAPHQL_SCHEMA_PATH];
let name_map = &package.artifacts[GRAPHQL_NAME_MAP_PATH];
assert_eq!(package.names.definitions["Person"].input_type, "PersonInput");
```

Each structural object has a paired output `type` and input `input` object.
The emitter directly represents booleans, strings, numbers, the exact signed
32-bit `Int` domain, explicit nullability, finite `const`/`enum` value sets,
closed named fields, required fields, homogeneous lists, local direct
`#/$defs/...` references, aliases, descriptions, and deterministic inline
object helpers. Types, helpers, and the fallback scalar share one global
collision-checked namespace; fields and enum symbols are collision-checked in
their GraphQL-local namespaces. The canonical `name-map.json` and identical
typed `GraphqlNameMap` retain every source definition key, JSON property key,
and finite JSON value alongside its GraphQL representation.

`GraphqlPackage::encode_input` translates source JSON property names and
finite values into GraphQL input names and enum symbols.
`GraphqlPackage::decode_output` reverses present response fields and symbols;
it permits GraphQL's normal partial field selection and does not invent omitted
fields. Unknown definitions, keys, values, symbols, or incompatible carriers
fail. This generated codec is the bidirectional boundary. Arbitrary GraphQL SDL
does not define one unique JSON Schema acceptance relation, so PurRDF does not
claim or provide a general SDL-to-JSON-Schema reader.

GraphQL coercion cannot represent every JSON Schema assertion. Every semantic
difference is intentional, stable-coded, and located at its source JSON
Pointer on the closed `json-schema` → `graphql-september-2025` loss ledger:

| Boundary | Closed loss codes |
|---|---|
| fixed input fields and object-key rules | `additional-properties-validation-narrowed`, `pattern-properties-validation-changed`, `property-name-validation-changed`, `property-count-validation-dropped` |
| requiredness, null, and recursive inputs | `nullable-presence-validation-widened`, `recursive-input-nullability-relaxed` |
| lists and positional/evaluation rules | `singleton-list-coercion-widened`, `array-cardinality-validation-dropped`, `array-contains-validation-dropped`, `unique-items-validation-dropped`, `tuple-array-validation-delegated`, `unevaluated-validation-dropped` |
| scalar domains and predicates | `integer-domain-validation-delegated`, `numeric-validation-dropped`, `string-validation-dropped` |
| applicators and cross-field rules | `conditional-validation-dropped`, `dependency-validation-dropped`, `intersection-validation-delegated`, `union-validation-delegated`, `one-of-validation-delegated`, `negation-validation-delegated` |
| caller/runtime validation boundary | `custom-scalar-validation-delegated`, `keyword-validation-delegated` |

The fallback scalar is only declared in SDL. Its `parseValue`, `parseLiteral`,
and serialization behavior belongs to the caller, and every use that delegates
source validation is ledgered. A package with an empty ledger is exact for the
represented source acceptance relation; a non-empty ledger is never silently
treated as validation.

Emission hard-fails invalid caller names or prose, built-in fallback-scalar
names, malformed keyword values, `$id` rebasing, external, indirect, or
dangling `$ref`, `$dynamicRef`/`$recursiveRef`, pure-alias cycles,
unsatisfiable closed required fields, and any generated type, field, helper,
scalar, or enum name collision. The value codec likewise rejects unmapped or
structurally incompatible values. Fixed platform-independent limits make
resource behavior deterministic:

| Resource | Limit |
|---|---:|
| input `schema_json`, each emitted artifact, or one codec JSON value | 16 MiB |
| definitions, fields in one object, or values in one finite set | 65,536 |
| schema-expression or codec-value depth | 128 |
| generated or caller-supplied GraphQL name | 255 bytes |

The dev-only oracle uses `boon` as the draft 2020-12 source classifier and the
locked official GraphQL.js 16.14.0 implementation to build the generated SDL
and execute real variable coercion. It checks lossless agreement, every closed
loss family and location, codec/name-map agreement, and deliberate corruption
failures:

```bash
make graphql-oracle
```

GraphQL.js is not a release dependency. The production emitter and codec are
filesystem-free Rust and remain wasm-clean.

---

## Python extension

```bash
cd ../../bindings/python
maturin develop
```

```python
from purrdf_native import shacl

report = shacl.validate(shapes_ttl="...", data_nt="...")
print(report["conforms"])  # True / False
print(report["results"])   # list of violation dicts
```

Each result dict keeps the stable keys `focus`, `path`, `value`, `severity`,
`component`, `source_shape`, and `message`. When the shapes or path terms carry
`purrdf:graphBoxRole`, result dicts may also include `source_box_roles`,
`path_box_roles`, and `result_box_roles`.

---

## Project and community

`purrdf-shapes` is developed by [Blackcat Informatics® Inc.](https://blackcatinformatics.ca)
as one member of the [PurRDF](https://github.com/Blackcat-Informatics/purrdf)
RDF 1.2 toolkit. Most applications should depend on the umbrella
[`purrdf`](https://crates.io/crates/purrdf) crate, which re-exports this crate
as `purrdf::shapes`.

Related crates:

- [`purrdf-sparql-eval`](https://crates.io/crates/purrdf-sparql-eval) — the native
  SPARQL engine that powers SHACL-SPARQL constraints and targets
- [`purrdf-validate`](https://crates.io/crates/purrdf-validate) — renders
  validation reports as byte-deterministic SARIF 2.1.0
- [`purrdf-gts`](https://crates.io/crates/purrdf-gts) — the GTS graph-transport
  container engine

---

## License and copyright

Copyright © 2026 Blackcat Informatics® Inc.

This crate is licensed under **MIT OR Apache-2.0** — see
[`LICENSE-MIT`](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
and
[`LICENSE-APACHE`](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
in the repository root. Separate proprietary/commercial terms are available;
contact `licensing@blackcatinformatics.ca`.
