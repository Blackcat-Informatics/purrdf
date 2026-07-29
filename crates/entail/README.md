<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg" alt="PurRDF logo" width="120" height="120">
  </a>
</p>

# `purrdf-entail` — Native Entailment Engines

[![crates.io](https://img.shields.io/crates/v/purrdf-entail.svg)](https://crates.io/crates/purrdf-entail)
[![docs.rs](https://docs.rs/purrdf-entail/badge.svg)](https://docs.rs/purrdf-entail)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
[![Repository](https://img.shields.io/badge/repo-Blackcat--Informatics%2Fpurrdf-181717.svg)](https://github.com/Blackcat-Informatics/purrdf)

`purrdf-entail` is native, `wasm32`-clean entailment for the PurRDF
[`RdfDataset`](https://docs.rs/purrdf-core) IR. A family of engines sits behind
one façade, each the right tool for its SPARQL entailment regime — closing a
dataset to its inferred fixpoint entirely in interned `TermId` space, with **no**
external reasoner, no `tokio`, and no string round-trip.

## Surface Map

| Entry point | Regime(s) | Engine |
| --- | --- | --- |
| `materialize(ds, regime)` | `Simple`, `RDF`, `RDFS`, `OWL-RL` | Forward-materialization ("chase") over a fixed rule set via a native semi-naive fixpoint. Returns `(closure, ReasoningReport)` — the report is not optional. |
| `materialize_dl(...)` | `OWL-Direct` | Open-world OWL DL over an ALCOIQ tableau — needs the query's class expressions, so it is not reachable through the plain `materialize` façade. |
| `materialize_rif(...)` | `RIF` | RIF-Core rule entailment over a parsed `RuleSet`. |
| `parse_rif_xml(...)` / `resolve_rif_imports(...)` | `RIF` | Normative RIF-XML parsing with caller-owned, I/O-free import resolution. |
| `Regime::from_iri(iri)` | — | Parse a `sparql:entailmentRegime` IRI to its enum. |
| `rules(regime)` / `implemented(regime)` | — | The rule table a regime is *defined by*, and the subset this crate fires. Their difference is the measurable gap. |
| `calculus_program(regime)` | — | The regime's calculus as DL-clause data, so its `purrdf-datalog` contract hash is recomputable by a consumer. |

`D` (datatype) entailment is a typed, spec-inherent boundary
(`EntailError::Unsupported`) rather than a silent default.

## Invariants

* **No minted vocabulary.** Every constant in `vocab` is a standard
  `rdf:`/`rdfs:`/`owl:` IRI drawn from the entailment spec itself — this crate
  fabricates none.
* **Every run states what it did.** `materialize` returns a `ReasoningReport`
  with every closure. It carries the regime's `Completeness` — *derived* from
  `rules(regime)` minus `implemented(regime)`, so it improves by itself as rules
  are added — which rules fired and how many conclusions each contributed, the
  `Boundary`s the run met and why, what it consumed of the evaluation ceilings,
  and the contract hash of the calculus it ran, so a cached closure minted under
  a different rule set can be refused rather than trusted. A report that claims
  `Exact` while naming a boundary is a test failure.
* **wasm-clean and dependency-lean.** Dependencies are `purrdf-core`,
  `purrdf-datalog`, `roxmltree` and two fixed-key hashers (all
  `wasm32-unknown-unknown`-clean), so this crate carries into Rust, WebAssembly,
  and C without a threads/filesystem/RNG dependency.
* **Determinism.** The chase is a fixpoint over the frozen IR; a given input and
  regime always yields the same closure — and the same report, byte for byte.

## Local Checks

```bash
cargo test -p purrdf-entail
```

## Part of PurRDF

This crate is one member of the [PurRDF](https://github.com/Blackcat-Informatics/purrdf)
workspace — an RDF 1.2 toolkit with native codecs, SPARQL, SHACL, ShEx,
entailment, and the GTS graph transport, carried into Python, WebAssembly, and
C. Most applications should depend on the umbrella
[`purrdf`](https://crates.io/crates/purrdf) crate, which re-exports this crate
as `purrdf::entail`; depend on `purrdf-entail` directly only when you want the
entailment engines alone.

There are deliberately no Cargo feature flags anywhere in the workspace. MSRV
follows the workspace `rust-version` (currently 1.96, stable toolchain only).

## License

Licensed under either of

- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)

at your option.
