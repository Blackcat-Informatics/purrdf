# Issue #515: sparql-eval/datalog: per-row evidence -- information state per standpoint, completeness, frame -- combined by the bilattice

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Summary

Per-row **evidence** through `sparql-eval` and `datalog`. Each solution row carries a small id into a per-query interned evidence table. Each entry holds three independent parts:

1. **information state per standpoint**: GMEOW's four values, `supported`, `opposed`, `both`, `neither` (Belnap's FOUR), from the matched claims' polarity;
2. **completeness**: `complete`, `undetermined` or `not-evaluated`, with ledger reasons;
3. **frame**: the non-actual contexts the row rests on (counterfactual, assumed, imagined, licensed falsehood), or none.

Every operator combines its inputs' evidence into its output's. Id 0 is the common case (`supported`, `complete`, actual), so the fast path pays nothing. Combination is a lookup memoised over pairs of ids, and the table is canonically ordered, so output stays byte-identical.

Today the closest thing is `FallibleDatasetView::operation_status`, whose `Evidence` is per **view**. This asks for evidence per **row**.

## Why

Katamari ADR-0045 45.20 (Proposed; Patrick approved this design 2026-10-09). Design note: `katamari/docs/design/PURRDF-SEAM.md` §3 at `e65fe50`. Four reasons:
- A join of an incomplete input with a complete one must not come out looking complete.
- `both` (a contradiction within a standpoint) must not be read as "unknown".
- Negation over missing data must not succeed: deception by omission.
- No conclusion from a counterfactual or imagined premise may reach the actual frame.

## Combination rules (per standpoint, pointwise)

| Operator | Information state | Completeness | Frame |
|---|---|---|---|
| Join | truth-order meet (t∧t=t, t∧f=f, t∧both=both, t∧neither=neither, both∧neither=f, f∧x=f) | the worse; ledgers united | the meet of frames |
| Duplicate solutions merged | information-order join (t⊕f=both, t⊕neither=t, x⊕x=x) | the better | actual if either is actual |
| `UNION` | each branch's row keeps its own | own | own |
| `FILTER` that decides | unchanged | unchanged | unchanged |
| `FILTER` the data cannot decide | unchanged | `undetermined`; the row moves to a separate undecided partition, counted and enumerable on demand | unchanged |
| `OPTIONAL` with no match | left side's | `undetermined` for the optional part unless the right side was complete | left side's |
| `EXISTS P` | information-order join over matches; `neither` if none | P's | union over matches |
| `NOT EXISTS P` / `MINUS` | negation of `EXISTS` (¬t=f, ¬f=t, ¬both=both, ¬neither=neither) **only if** P is complete **and** a closure is declared (`logic:ClosureEntry`); otherwise `undetermined` | as stated | P's |
| Property path | a found path is a witness, and **complete** | "no path" complete only under the `NOT EXISTS` conditions | along the witness |
| Aggregates | per standpoint | `COUNT`, and `SUM` over non-negatives, are lower bounds; `MAX` a lower bound, `MIN` an upper bound; `AVG` undetermined; each labelled | union |

The principle: **incompleteness never weakens a found witness, only conclusions from absence and totals.**

**Standpoints** follow GMEOW's `gmeow:sharpens` poset (Standpoint Logic): a claim according to S2 counts for S1 whenever S1 sharpens S2. `gmeow:unspecifiedStandpoint` sits outside the order and is reported as unscoped evidence. **Frames** join at their meet: the intersection of standpoints, or the conjunction of scenarios. An actual premise joined with a scenario premise holds in the scenario.

## Acceptance criteria

- Property tests: evidence combines per the table for random algebras.
- A join with a locked or incomplete input is labelled incomplete.
- `NOT EXISTS` over incomplete input is `undetermined`, and true only with completeness and a declared closure.
- A found property path is complete even when other data is missing.
- Output is byte-identical across runs and targets.
- With all-default evidence, throughput is within noise of today's.
- No compiled-in limit.


## Comments (1)

### paudley — 2026-10-09T21:07:06Z

## Addendum: answer tiers, so a yes or no stays cheap

"Was there a blue thing here before?" wants `yes`, `no`, `both` or `unknown`, not thousands of annotated rows. So the evaluator should take the **tier** a query asks for and compute only that:
- **verdict**: one value, or rows plus one answer-level status;
- **summary**: plus per-reason counts;
- **rows**: per-row evidence;
- **ledger**: the RDF 1.2 ledger (#516).

The witness rule gives early termination at the verdict tier:
- `yes` stops at the first supported witness;
- `both` stops at a supported and an opposed witness;
- `no` needs a complete search under a declared closure, and the evaluator stops as soon as `no` is unreachable (an incomplete input), answering `unknown`.

Evidence ids are still tracked at every tier, since `no` *is* a completeness judgement, but nothing is rendered below the tier asked for. Design: Katamari `docs/design/PURRDF-SEAM.md` §3.7.


