<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# purrdf-datalog

Deterministic, wasm-clean Datalog evaluation: a columnar relation store and a
stratified semi-naive fixpoint, carrying no ambient I/O, no wall clock and no
RNG.

This crate is the shared execution substrate beneath PurRDF's rule-driven
engines. A rule set is *data* — a table of clauses over a relation store — so
the RDF, RDFS and OWL 2 RL calculi, RIF-Core rules and SHACL-AF `sh:rule`
entailment are rule tables over one evaluator rather than four hand-written
fixpoints.

## Design commitments

* **Deterministic by construction.** Per-key rows keep insertion order, the
  arrangement is sorted, and no map iteration order reaches an output path.
  Identical input yields byte-identical output, on every target.
* **Budgets are constants, not knobs.** Step, fact and arena ceilings are fixed
  workspace constants and their consumption is *reported*, never configured —
  two callers with the same input always get the same answer. There is no
  wall-clock budget: it would break both wasm and reproducibility.
* **wasm-clean.** No threads-only constructs, no filesystem, no clock, no RNG.
  Where work is parallelised it uses indexed `par_chunks`/`par_iter` reduced in
  source order — never `par_sort` or `par_bridge`, which are not order-stable —
  and degrades to inline-sequential on `wasm32-unknown-unknown`.
* **No optionality.** No Cargo features; no conditional compilation selecting
  between semantics.

## Licence

Dual-licensed under [MIT](../../LICENSES/MIT.txt) or
[Apache-2.0](../../LICENSES/Apache-2.0.txt), at your option.
