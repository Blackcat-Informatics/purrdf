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
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0%20OR%20MulanPSL--2.0-blue.svg)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
[![Repository](https://img.shields.io/badge/repo-Blackcat--Informatics%2Fpurrdf-181717.svg)](https://github.com/Blackcat-Informatics/purrdf)

Deterministic, wasm-clean Datalog evaluation: a columnar relation store and a
stratified semi-naive fixpoint, carrying no ambient I/O, no wall clock and no
RNG.

A rule set is *data* — a table of clauses over a relation store — rather than a
hand-written loop. Its consumers are
[`purrdf-entail`](https://crates.io/crates/purrdf-entail) — the RDF, RDFS, OWL 2
RL and D calculi are declared as DL-clause programs and evaluated here, which is
what lets a reasoning report carry a *contract hash* of the exact program that
ran instead of a claim about which rules were meant to — and
[`purrdf-shapes`](https://crates.io/crates/purrdf-shapes), whose SHACL rules
and SPARQL 1.2 RL rule sets run on this crate's ordered schedule.

## Design commitments

* **One rule IR: the DL-clause.** Every rule is
  `U₁ ∧ … ∧ Uₙ → ∃ȳ. (C₁ ∨ … ∨ Cₘ)`, where each disjunct `Cᵢ` is itself a
  conjunction of head atoms — so `A ⊑ ∃r.C`, which lowers to
  `∃y. (r(x, y) ∧ C(y))` with one *shared* witness, is a single rule rather
  than two unrelated ones. That single shape holds all five head forms —
  atomic (a Datalog rule), existential, disjunctive, conjunctive and empty
  (`false`) — so a consumer of any of them needs no second representation.
  The semi-naive evaluator runs the atomic form; the chase consumes the
  existential and conjunctive forms; the disjunctive and inconsistency forms
  are refused **by name** at the plan pipeline's entrance: never silently
  accepted, never silently dropped. No evaluator here case-splits — a case
  split is not a least fixpoint — and the consumer of the disjunctive form is
  [`purrdf-entail`](https://crates.io/crates/purrdf-entail)'s OWL-Direct
  hypertableau, which classifies its own `SHOIQ(D)` DL-clauses through this
  crate's `HeadForm` and branches on exactly that form.
* **Guards and an ordered schedule, still one evaluator.** A guard literal is a
  body literal whose meaning a caller supplies at evaluation time — a SPARQL
  `FILTER` or assignment, a SHACL node expression, a CONSTRUCT query — so a rule
  language with an expression sublanguage lowers onto the same join, commit and
  budgets as a pure Datalog program. The ordered schedule runs such a program in
  the layers, run-once rules and concurrently evaluated groups SHACL 1.2 Inference
  Rules and SPARQL 1.2 RL define; the SPARQL 1.2 RL rule-level stratifier builds
  that schedule, or names the cycle that makes one impossible.
* **Plans are content-addressed.** A compiled program is keyed by a BLAKE3
  digest over the planner version, the caller's contract hash and a canonical
  digest of the clause program. The cache is owned by the caller, never a
  process global — a global would make an answer depend on evaluation history.
* **Deterministic by construction.** Per-key rows keep insertion order, the
  arrangement is sorted, and no map iteration order reaches an output path.
  Identical input yields byte-identical output, on every target.
* **Limits refuse; they never truncate.** The STORED-FACT and JOIN-STEP limits
  are the caller's (`EvalOptions::with_max_stored_facts`,
  `EvalOptions::with_max_join_steps`), with a default sized for the target:
  131,072 facts and 1,048,576 join steps on `wasm32`, where the store lives in
  one linear memory, and 4,194,304 facts and 1,048,576 join steps everywhere
  else (`DEFAULT_MAX_STORED_FACTS`, `DEFAULT_MAX_JOIN_STEPS`, chosen at compile
  time from the target architecture). The term-arena ceiling
  (`MAX_TERM_ARENA_BYTES`) stays a constant. A limit can only refuse: a run
  inside its limits returns the least model, the same under every limit that
  admits it, and a run past one returns `EvalError::BudgetExhausted` naming the
  limit, the numbers and the knob that raises it — never a truncated model.
  Every caller passing the same options gets the same answer or the same
  refusal, and the effective limits are folded into every program's contract
  hash, so a result computed under one set of limits never claims another's
  identity. A join-step count is also a property of the plan, so a limit sized
  tightly against one release can refuse under the next; headroom cannot change
  a completed answer. The limits on term generation by a guarded program are the
  caller's too: whether such a program terminates is undecidable, so any fixed
  limit refuses some program that terminates (`EvalOptions`). The
  TERM-GENERATING ROUND limit defaults to `DEFAULT_MAX_TERM_GENERATING_ROUNDS`
  (16,384). The GENERATED-TERM budget, the terms added beyond the seeded store's
  `N`, defaults to `max(65,536, 4 × N)`. A run past either is refused as
  `EvalError::TermLimitExceeded`, naming the limit, the numbers and the rules
  that generated a term in the last round, and never as divergent. The limits
  count rounds and terms rather than pricing work, only ever refuse, cannot bind
  a guard-free program, and are folded into the program's contract hash.
* **A stop signal is admitted, because it is answer-blind.** `StopSignal` is a
  two-line trait polled at round boundaries the fixpoint was going to reach
  anyway. It carries no number and cannot be asked *where* to stop, only whether
  to: an unstopped run returns exactly what it would have with no signal, and a
  stopped one returns a typed refusal and **no model at all**. There is no third
  outcome, so there is no schedule to version and no partial closure to mistake
  for a complete one. This crate still reads no clock — a host's wall deadline
  arrives already reduced to a yes/no question, which is what keeps the
  nondeterministic input outside the crate.
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
C (the GTS container itself reaches Python and C, not the wasm package). It is the evaluator beneath
[`purrdf-entail`](https://crates.io/crates/purrdf-entail) and
[`purrdf-shapes`](https://crates.io/crates/purrdf-shapes)'s rules engine, and is published
separately so a caller can depend on the fixpoint alone. Note that it is not
re-exported by the umbrella [`purrdf`](https://crates.io/crates/purrdf) crate.

There are deliberately no Cargo feature flags anywhere in the workspace. MSRV
follows the workspace `rust-version` (currently 1.98, stable toolchain only).

## License

Licensed under any one of the following, at your option:

- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [Mulan Permissive Software License, Version 2 (MulanPSL-2.0)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MULAN)
