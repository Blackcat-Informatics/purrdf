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

`purrdf-datalog` is re-exported by the umbrella `purrdf` crate as the
`purrdf::datalog` module — the entailment surface CARRIES its types (a
[`ReasoningReport`](entailment.md) hands out a `datalog::cache::ContractHash`
and a `datalog::seminaive::BudgetReport`), so a consumer that matches on them
needs no second dependency on `purrdf-datalog`. Depend on the crate directly
(`purrdf-datalog = "…"`) only when you want the fixpoint alone, with no other
`purrdf` surface in the build.

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
name**. The existential form is not lost by that refusal: the restricted chase
consumes it, minting frontier-addressed Skolem witnesses, which is how the four
existential RDF/RDFS patterns fire and the rule inventory reads complete. What
never happens is a surrogate leaking into an answer — the entailment layer above
withholds every conclusion that mentions a witness at the materialization
boundary and reports the withholding, rather than inventing an answer the caller
did not ask for.

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

## Limits refuse; they never truncate

Three limits bound what every run holds and enumerates. Two are the caller's,
on `EvalOptions`, with a default sized for the target; the third is a constant:

| Limit | Bounds | Set with | Default on `wasm32` | Default elsewhere |
| --- | --- | --- | --- | --- |
| join steps | candidate solutions enumerated | `EvalOptions::with_max_join_steps` | 1,048,576 | 1,048,576 |
| stored facts | facts seeded or derived | `EvalOptions::with_max_stored_facts` | 131,072 | 4,194,304 |
| term arena bytes | interned term surface bytes | `MAX_TERM_ARENA_BYTES` (fixed) | 16,777,216 | 16,777,216 |

The stored-fact defaults differ because the memory the store lives in does: a
`wasm32` evaluation shares one linear memory with the rest of its page, and its
default is the value every target used while the limit was fixed. A native
process can hold a far larger least model, and a ceiling sized for a browser
refused ordinary terminating rule sets there — a single rule that copies
each of 70,000 `ex:p` triples to `ex:q`. The default is chosen at compile time from the
target architecture, never by a Cargo feature.

The join-step default is the same on every target, for a measured reason: a
body's candidate solutions for a round are materialised before the limit is
checked. A body joining two unrelated atoms over 20,000 nodes (400 million
candidates) is refused at the default in about two seconds, holding under half
a gigabyte; at 268,435,456 join steps it had allocated about 20 GB, still
enumerating, when it was stopped at a 24 GB address-space cap. A larger default
would let a Cartesian body exhaust memory before it could be refused. Reach is
the caller's to buy: the non-linear transitive closure of a thousand-node chain
enumerates over 67 million candidates and completes when the caller raises the
limit (`--max-join-steps` on the command line), while the linear closure of the
same chain fits the default.

The join-step limit is not what stops a divergent rule. A SHACL shape rule
minting a new focus node every round is re-executed only for the focus nodes
whose inputs changed, not for every earlier one, so it is refused by the term
limits below within a few seconds, whatever the join-step limit: by the
term-generating round limit when its new terms are of bounded length, and by
the term-arena ceiling when each is longer than the last (a rule minting an IRI
one character longer every round reaches 16 MiB of term surfaces after about
5,800 rounds).

A limit can only refuse. A run inside its limits returns the program's least
model, the same model under every limit that admits it; a run past one returns
no model at all. Every caller passing the same options gets the same answer or
the same refusal, and the effective limits are folded into the result's
contract hash, so a closure computed under one set of limits, or on one target
under its defaults, never claims the identity of another. A join-step count
also depends on the evaluator's plan, so a limit sized tightly against one
release can refuse under the next; a limit with headroom cannot change a
completed answer.

There is deliberately no wall-clock budget: it would break both `wasm32` and
reproducibility.

Passing a limit is a **total** refusal, not a truncation. There is no partial
fixpoint to hand back with a note attached, and a truncated closure presented as
a complete one is exactly the failure a reasoning report exists to prevent. The
error names the limit, the numbers observed and permitted, and the knob that
raises it; in `purrdf-entail` that surfaces as `EntailError::Evaluate` or
`EntailError::Chase`.
