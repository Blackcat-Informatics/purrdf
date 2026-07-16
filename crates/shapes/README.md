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
