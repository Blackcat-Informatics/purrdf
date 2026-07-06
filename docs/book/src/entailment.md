<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Entailment

[`purrdf-entail`](https://docs.rs/purrdf-entail) (re-exported as
`purrdf::entail`) is native, `wasm32`-clean entailment for the PurRDF
`RdfDataset` IR. A family of engines sits behind one facade, each the right
tool for its SPARQL entailment regime — closing a dataset to its inferred
fixpoint entirely in interned `TermId` space, with **no** external reasoner,
no async runtime, and no string round-trip.

## Surface map

| Entry point | Regime(s) | Engine |
| --- | --- | --- |
| `materialize(ds, regime)` | `Simple`, `RDF`, `RDFS`, `OWL-RL` | Forward materialization ("chase") over a fixed rule set via a native semi-naive fixpoint. |
| `materialize_dl(...)` | `OWL-Direct` | Open-world OWL DL over an ALCOIQ tableau — it needs the query's class expressions, so it is not reachable through the plain `materialize` facade. |
| `materialize_rif(...)` | `RIF` | RIF-Core rule entailment over a parsed `RuleSet`. |
| `Regime::from_iri(iri)` | — | Parse a `sparql:entailmentRegime` IRI to its enum. |

```rust,ignore
use purrdf::entail::{materialize, Regime};

// Close a frozen dataset to its RDFS fixpoint; the result is a new dataset.
let closed = materialize(&ds, Regime::Rdfs).expect("materializes");
```

## The chase (Simple / RDF / RDFS / OWL-RL)

`materialize` runs a forward-materialization chase: a fixed rule set for the
selected regime, applied by a semi-naive fixpoint until no new quads appear.
Because it runs over the frozen IR, it is deterministic — a given input and
regime always yields the same closure — and because it works in `TermId`
space, no term is ever re-parsed or re-serialized along the way.

Typical use: materialize first, then query with the plain
[SPARQL engine](sparql/querying.md) or validate the closure with
[SHACL](validation/shacl.md) (the SHACL validator itself performs no
inference).

## OWL-Direct: the tableau

`OWL-Direct` semantics is open-world Description Logic, which a forward chase
cannot answer. `materialize_dl` runs an **ALCOIQ tableau** instead — answering
instance and subsumption queries via classification, realization, and
query-directed materialization. Because it needs the query's class
expressions, it has its own entry point rather than hiding behind
`materialize`.

## RIF

`materialize_rif` evaluates **RIF-Core** rules over a parsed `RuleSet`,
covering the SPARQL RIF entailment regime.

## The D-entailment boundary

`D` (datatype) entailment is a typed, spec-inherent boundary: requesting it
returns `EntailError::Unsupported` rather than silently defaulting to a
weaker regime. (No case in the vendored W3C corpus exercises D-entailment
alone.) This is the workspace-wide hard-fail discipline: an unsupported
regime is an error you can handle, never a wrong answer.

## Invariants

- **No minted vocabulary.** Every constant in the crate's `vocab` module is a
  standard `rdf:`/`rdfs:`/`owl:` IRI drawn from the entailment specs
  themselves — the crate fabricates none, per the
  [toolkit-not-ontology rule](project/design-rules.md).
- **Dependency-lean.** The only dependency is `purrdf-core`, so the engines
  carry into Rust, WebAssembly, and C unchanged.
- **Deterministic.** Same input + regime → same closure, always.

## Conformance

All 70 W3C entailment cases pass at the time of writing — RDF/RDFS/OWL-RL
chase, OWL-Direct (DL) tableau, RIF-rule, and RDF-axiomatic predicate typing —
run through the SPARQL conformance harness. The live scoreboard is
[`docs/CONFORMANCE.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/CONFORMANCE.md).
