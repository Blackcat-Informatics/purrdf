<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# purrdf-entail

`purrdf-entail` is native, `wasm32`-clean entailment for the PurRDF
[`RdfDataset`](https://docs.rs/purrdf-core) IR. A family of engines sits behind
one façade, each the right tool for its SPARQL entailment regime — closing a
dataset to its inferred fixpoint entirely in interned `TermId` space, with **no**
external reasoner, no `tokio`, and no string round-trip.

## Surface Map

| Entry point | Regime(s) | Engine |
| --- | --- | --- |
| `materialize(ds, regime)` | `Simple`, `RDF`, `RDFS`, `OWL-RL` | Forward-materialization ("chase") over a fixed rule set via a native semi-naive fixpoint. |
| `materialize_dl(...)` | `OWL-Direct` | Open-world OWL DL over an ALCOIQ tableau — needs the query's class expressions, so it is not reachable through the plain `materialize` façade. |
| `materialize_rif(...)` | `RIF` | RIF-Core rule entailment over a parsed `RuleSet`. |
| `Regime::from_iri(iri)` | — | Parse a `sparql:entailmentRegime` IRI to its enum. |

`D` (datatype) entailment is a typed, spec-inherent boundary
(`EntailError::Unsupported`) rather than a silent default.

## Invariants

* **No minted vocabulary.** Every constant in `vocab` is a standard
  `rdf:`/`rdfs:`/`owl:` IRI drawn from the entailment spec itself — this crate
  fabricates none.
* **wasm-clean and dependency-lean.** The only dependency is `purrdf-core`
  (itself `wasm32-unknown-unknown`-clean), so this crate carries into Rust,
  WebAssembly, and C without a threads/filesystem/RNG dependency.
* **Determinism.** The chase is a fixpoint over the frozen IR; a given input and
  regime always yields the same closure.

## Local Checks

```bash
cargo test -p purrdf-entail
```
