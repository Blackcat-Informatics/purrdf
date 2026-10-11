# Issue #511: retrieval: field scoping applied inside each stratum's candidate generation, before depth, with undecided counts

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Summary

Field scoping in `purrdf-retrieval`: a request may constrain terms to declared fields (`from:'John'`, `subject:'project wingus'`) and may carry hard constraints (time, place, entity). The scope **must be applied inside each stratum's candidate generation, before that stratum's depth.** Applying it to a fused or depth-limited list loses, invisibly, the candidates the depth already cut.

## Why: measured

Katamari ADR-0033 33.14 was amended on 2026-10-09 (Patrick vetted). The evidence is `lillith_memetics`' recall measurement (`docs/design/recall-measurement.md`), on composite questions (place, time and entity):
- post-filtering ranked **153** survivors where pre-filtering ranked **2,681**;
- hard constraints beat soft fusion, nDCG@10 **0.58 against 0.33**;
- pre-filtering was cheap on the lexical side (about 30.7k postings, 20 ms).

## What it needs

- Request terms may carry a field scope, resolved from the predicate at query time; postings store no field. A filter restricts membership and is never a ranking weight.
- The scope compiles into each stratum's unit, ahead of the depth bound.
- **Undecided is not false.** A constraint the data cannot decide (a record with no time, a place with no position in a containment hierarchy, an instant whose declared precision is coarser than the constraint) must not silently drop the record as a failure. The answer reports, per constraint, how many candidates it could not decide, so a confident empty answer is distinguishable from "no data". In the measurement, 2,279 records had no time at all, and unknown-as-false left one question with no survivors.

## Acceptance criteria

- A scoped request returns every record that satisfies the scope among each stratum's top `depth` *after* scoping, not before.
- The undecided count per constraint is reported, and the records it counts are not reported as failing.
- Output is byte-identical across runs and targets.


## Comments (0)

