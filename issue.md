# Issue #510: retrieval: a union stage beside fuse -- per-stratum candidates with ranks kept, no score, measured depths

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Summary

A `union` stage in `purrdf-retrieval`, beside `fuse`. It takes each stratum's top `depth_s` from `execute` and deduplicates by subject. Every candidate keeps its rank in every stratum that named it, together with each producer's evidence. It orders nothing and adds no score. It is the rung for a caller that orders candidates itself, with a reranker or anything else that knows more than the store.

```text
plan(request, registry, statistics) -> Plan
compile(Plan, environment)          -> per-stratum SPARQL units
execute(units, registry, dataset)   -> per-stratum ranked streams
fuse(streams, profile, k)           -> the top k of one ordered answer
union(streams, depths)              -> one candidate set, ranks per stratum kept   (new)
```

## Why: measured

Katamari ADR-0033 33.16 was amended on 2026-10-09 (Patrick approved) to add this stage. The evidence is `lillith_memetics`' recall measurement (`docs/design/recall-measurement.md`): 111 questions, about 6,600 blind-judged pairs, held out over five 2-fold splits.

- **Union recall:** lexical ∪ vector at depth 50 held 0.90 of the judged relevant records, against 0.75 for lexical alone. On paraphrase questions (relevant records sharing no content word with the question) it held 0.71 against 0.33.
- **No fusion rule ordered that union better than lexical alone** (nDCG@10 0.512):

  | Rule | nDCG@10 |
  |---|---|
  | equal-weight RRF | 0.32-0.35 |
  | fitted weights | 0.508-0.513 (by turning vectors almost off) |
  | calibrated log-odds | 0.39-0.41 |
  | per-term gating | 0.503 |

So fusion's contribution is candidate recall, not order, and the shape that keeps it is a two-stage one: union, then something that orders.

## What it needs

- **Per-stratum depth that is not derived from `k`.** Today `depth_from` gives `min(declared, statistics-narrowed, k)` (`planner.rs:1106` at `3b6ec4d9f`). For `union`, each `depth_s` is a work bound the caller supplies, measured from that stratum's recall curve, and reported in the answer. The measurement shows the depths are asymmetric: lexical keeps gaining past 20, while vector's contribution is nearly all in its top 20.
- **Evidence preserved per producer**, as `fuse` does (Katamari ADR-0033 33.2): "two answered and one could not" must stay distinguishable.
- **A top-k shape,** like `fuse`: a candidate's ranks are complete only when every stratum has emitted to its depth.
- **A place to stop, not a flag on `fuse`.** It is a separate entry point with the same `execute` under it, and `search`'s composition identities stay testable.

## Acceptance criteria

- `union` over the same streams equals the set of subjects in each stratum's top `depth_s`, with each candidate's per-stratum ranks exactly as `execute` produced them.
- Output is byte-identical across runs and targets.
- The answer reports each stratum's depth and evidence.
- `fuse` and `search` are unchanged.


## Comments (0)

