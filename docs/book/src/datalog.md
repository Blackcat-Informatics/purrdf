<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Datalog: the Fixpoint Engine

[`purrdf-datalog`](https://docs.rs/purrdf-datalog) is deterministic, `wasm32`-clean
Datalog evaluation: a columnar relation store, an index-selecting planner, and a
semi-naive fixpoint, carrying no ambient I/O, no wall clock, and no RNG.

It exists so that a rule set is *data* rather than a hand-written loop.
[`purrdf-entail`](entailment.md) declares the RDF, RDFS, OWL 2 RL, and D calculi
as DL-clause programs and evaluates them here, which is what lets a reasoning
report carry a **contract hash** of the exact program that ran instead of a claim
about which rules were meant to run.

Unlike the other engines in this book, `purrdf-datalog` is **not** re-exported by
the umbrella `purrdf` crate. Depend on it directly
(`purrdf-datalog = "…"`) when you want the fixpoint alone.

## One rule IR: the DL-clause

Every rule has the shape

```text
U₁ ∧ … ∧ Uₙ  →  ∃ȳ. (C₁ ∨ … ∨ Cₘ)
```

where each disjunct `Cᵢ` is itself a conjunction of head atoms. That single shape
holds all five head forms — atomic (an ordinary Datalog rule), existential,
disjunctive, conjunctive, and empty (`false`) — so an axiom like `A ⊑ ∃r.C`,
which lowers to `∃y. (r(x, y) ∧ C(y))` with one *shared* witness, is one rule
rather than two unrelated ones.

The semi-naive evaluator runs the atomic form and refuses the other four **by
name**. That refusal is the reason the four RDF/RDFS residuals in the entailment
rule inventory are a gap and not a silent approximation: a rule that concludes
about a fresh blank node has an existential head, and inventing a surrogate for
that witness would answer a question the caller did not ask.

## Determinism

- Per-key rows keep insertion order; the arrangement is sorted; no map iteration
  order reaches an output path.
- Plans are content-addressed by a BLAKE3 digest over the planner version, the
  caller's contract hash, and a canonical digest of the clause program. The cache
  is owned by the caller, never a process global — a global would make an answer
  depend on evaluation history.
- Where work is parallelised it uses indexed `par_iter`/`par_chunks` reduced in
  source order, never `par_sort` or `par_bridge`, and degrades to
  inline-sequential on `wasm32-unknown-unknown`.

Identical input yields byte-identical output, on every target.

## Budgets are constants, not knobs

Three ceilings bound every run, and all three are fixed workspace constants whose
consumption is *reported*, never configured — two callers with the same input
always get the same answer:

| Ceiling | Bounds |
| --- | --- |
| `MAX_JOIN_STEPS` | candidate solutions enumerated |
| `MAX_STORED_FACTS` | facts seeded or derived |
| `MAX_TERM_ARENA_BYTES` | interned term surface bytes |

There is deliberately no wall-clock budget: it would break both `wasm32` and
reproducibility.

Passing a ceiling is a **total** refusal, not a truncation. There is no partial
fixpoint to hand back with a note attached, and a truncated closure presented as
a complete one is exactly the failure a reasoning report exists to prevent. The
error names which ceiling was hit and what the run had consumed when it stopped;
in `purrdf-entail` that surfaces as `EntailError::Evaluate`.
