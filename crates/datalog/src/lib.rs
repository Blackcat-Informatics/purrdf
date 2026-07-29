// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

#![forbid(unsafe_code)]

//! Deterministic, wasm-clean Datalog evaluation: a columnar relation store and a
//! stratified semi-naive fixpoint.
//!
//! This crate is the shared execution substrate beneath PurRDF's rule-driven
//! engines. A rule set is *data* — a table of clauses over a relation store — so
//! the RDF, RDFS and OWL 2 RL calculi, RIF-Core rules and SHACL-AF `sh:rule`
//! entailment become rule tables over one evaluator instead of four hand-written
//! fixpoints, each with its own indexes, its own termination argument and its own
//! opportunity to diverge.
//!
//! # Determinism
//!
//! Identical input yields byte-identical output, on every target. Per-key rows
//! keep insertion order, the arrangement is sorted, and **no map iteration order
//! reaches an output path**. Where evaluation is parallelised it uses indexed
//! `par_chunks`/`par_iter` reduced in source order — never `par_sort` or
//! `par_bridge`, which are not order-stable — and degrades to inline-sequential
//! on `wasm32-unknown-unknown`.
//!
//! # Budgets are constants, not knobs
//!
//! Step, fact and arena ceilings are fixed constants and their consumption is
//! *reported*, never configured. A caller-supplied budget would mean two callers
//! running the same program over the same input get different answers — the same
//! semantic optionality that the no-Cargo-features rule exists to prevent, merely
//! arriving through a parameter instead. There is no wall-clock budget: it would
//! break both wasm and reproducibility.
//!
//! # Portability
//!
//! No filesystem, no clock, no RNG, no ambient I/O. The crate builds for
//! `wasm32-unknown-unknown` and is part of the workspace wasm gate.
