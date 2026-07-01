<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# RDF Core IR

This directory is the immutable RDF 1.2 dataset kernel. It owns dense term IDs,
quad handles, reifier/annotation storage, provenance-ready bundle carriers, and
the evented ingestion/output adapters used by the wider Rust workspace.

## Module Families

| Family | Modules | Role |
| --- | --- | --- |
| Dataset construction | `builder`, `validate`, `dataset`, `term` | Intern terms, validate graph structure, freeze datasets, and expose zero-allocation iteration. |
| Mutation and comparison | `mutable`, `compare`, `canon` | Copy-on-write edits, structural comparison, and native canonicalization. |
| Bundle carrier | `pipeline_bundle`, `bundle` | Typed handles, lookaside data, blob storage, provenance, and GTS envelope helpers. |
| Event adapters | `ingest`, `event_sink` | Convert between `purrdf-events` streams and frozen `RdfDataset` values. |
| GTS import | `gts_resolve`, `import_graph`, `import_sink` | Feature-gated GTS term resolution and bundle ingestion. |

## Boundaries

- This kernel must remain independent of oxigraph as a normal dependency.
- Public handles are stable typed IDs; avoid leaking implementation indices into
  caller-visible strings.
- Validation happens before freeze. Once frozen, iteration should be infallible
  and allocation-light.

## Checks

```bash
make rdf-core-hygiene
make rust-test
make rust-docs
```
