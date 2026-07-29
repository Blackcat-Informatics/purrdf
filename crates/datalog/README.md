<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg" alt="PurRDF logo" width="120" height="120">
  </a>
</p>

# `purrdf-datalog` — Deterministic Datalog Evaluation

[![crates.io](https://img.shields.io/crates/v/purrdf-datalog.svg)](https://crates.io/crates/purrdf-datalog)
[![docs.rs](https://docs.rs/purrdf-datalog/badge.svg)](https://docs.rs/purrdf-datalog)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
[![Repository](https://img.shields.io/badge/repo-Blackcat--Informatics%2Fpurrdf-181717.svg)](https://github.com/Blackcat-Informatics/purrdf)

Deterministic, wasm-clean Datalog evaluation: a columnar relation store and a
stratified semi-naive fixpoint, carrying no ambient I/O, no wall clock and no
RNG.

A rule set is *data* — a table of clauses over a relation store — rather than a
hand-written loop. Its consumer today is
[`purrdf-entail`](https://crates.io/crates/purrdf-entail): the RDF, RDFS, OWL 2
RL and D calculi are declared as DL-clause programs and evaluated here, which is
what lets a reasoning report carry a *contract hash* of the exact program that
ran instead of a claim about which rules were meant to.

## Design commitments

* **One rule IR: the DL-clause.** Every rule is
  `U₁ ∧ … ∧ Uₙ → ∃ȳ. (C₁ ∨ … ∨ Cₘ)`, where each disjunct `Cᵢ` is itself a
  conjunction of head atoms — so `A ⊑ ∃r.C`, which lowers to
  `∃y. (r(x, y) ∧ C(y))` with one *shared* witness, is a single rule rather
  than two unrelated ones. That single shape holds all five head forms —
  atomic (a Datalog rule), existential, disjunctive, conjunctive and empty
  (`false`) — so the chase and the hypertableau that consume the last four
  need no second representation. The semi-naive evaluator runs the atomic form
  and refuses the other four **by name**: never silently accepted, never
  silently dropped.
* **Plans are content-addressed.** A compiled program is keyed by a BLAKE3
  digest over the planner version, the caller's contract hash and a canonical
  digest of the clause program. The cache is owned by the caller, never a
  process global — a global would make an answer depend on evaluation history.
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

## Part of PurRDF

This crate is one member of the [PurRDF](https://github.com/Blackcat-Informatics/purrdf)
workspace — an RDF 1.2 toolkit with native codecs, SPARQL, SHACL, ShEx,
entailment, and the GTS graph transport, carried into Python, WebAssembly, and
C. It is the evaluator beneath
[`purrdf-entail`](https://crates.io/crates/purrdf-entail) and is published
separately so a caller can depend on the fixpoint alone. Note that it is not
re-exported by the umbrella [`purrdf`](https://crates.io/crates/purrdf) crate.

There are deliberately no Cargo feature flags anywhere in the workspace. MSRV
follows the workspace `rust-version` (currently 1.96, stable toolchain only).

## License

Licensed under either of

- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)

at your option.
