# Issue #484: Planner statistics: per-predicate, per-class, distinct and multi-column counts, including reifier and annotation rows, mergeable across segments

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Summary

Give the SPARQL planner (and the rule engine) exact, mergeable statistics that describe RDF's dependent columns and RDF 1.2's reifier and annotation rows: per-predicate counts, per-class counts, distinct-value counts per position, a small set of multi-column statistics, and reifier/annotation counts that the estimator actually reads. Expose a seam so a consumer that maintains these with its own immutable segments can hand them to the planner.

## Current behaviour (`main` at `6273b6173`)

- The cost model uses two numbers: a per-pattern constants-only count from `DatasetView::cardinality_estimate`, and one global distinct-term count `T` (`crates/sparql-eval/src/bgp.rs:878-882`, `927`, `949-963`). Joins assume independence (`|p| / T^j`). There is no per-position distinct count, no histogram, no multi-column statistic.
- Where the counts come from:
  - `RdfDataset`: the length of the matching index range (`rdf-core/src/ir/dataset.rs:1252-1261`), exact when the bound positions form an index prefix.
  - Paged: `PageSummary` (`rdf-core/src/ir/paged/summary.rs:421-460`) already stores exact per-term subject/predicate/object occurrence counts and reifier and annotation counts per page, but the estimate takes only the smallest single-position marginal (`paged/admission.rs:171-190`) and reads only `PageStream::Base` (`admission.rs:182-183`), so the stored reifier counts never reach the planner.
  - Segmented: summed per-block row counts (`ir/segmented/session.rs:1242-1296`).
  - Trait default: materialize and count (`rdf-core/src/dataset_view.rs:679-689`).
- Reifier and annotation rows are produced at execution (`bgp.rs:503-517`, `emit_virtual_candidates`) but quoted-triple positions are unconstrained in the estimate (`bgp.rs:968-972`, `Pos::Slot(_) | Pos::Triple(_) => None`). Only the delta view pads its estimate with `rdf_row_count() - quad_count()` (`ir/mutable/delta_view.rs:705-722`).
- The retrieval planner has a separate, caller-supplied `Statistics` trait (`retrieval/src/statistics.rs:81-156`); the SPARQL planner has no equivalent.

## Why it matters

Katamari stores every claim as an RDF 1.2 reifier with six to eight annotations (standpoint, valid time, three clocks, producer, confidence, operation): about 300 million claims and 2-3 billion naive triples in one mail vault. Almost every interesting query joins through `rdf:reifies` and annotation predicates, which are exactly the rows the estimator cannot see. RDF columns are also strongly dependent (a predicate determines its object's kind; `rdf:type` dominates every count), so an independence model mis-estimates by orders of magnitude on the hot predicates, and the most common way RDF stores fail at scale is a bad estimate turning into a cross product (see #479 and #483).

Katamari builds its index as immutable sorted segments rebuilt on purge, so its statistics can be **exact**, not sampled: counts and histograms add across segments; distinct counts are exact within a segment and merge across segments over sorted ids. Exact statistics turn many plan decisions into facts and let some questions be answered from statistics alone. That only pays off if purrdf's planner can consume them.

## Proposed direction (a sketch)

1. **A statistics seam for `DatasetView`** (default: derived from what the view already has), carrying:
   - per-predicate triple count and distinct subjects / distinct objects;
   - per-class instance count (`rdf:type` object);
   - reifier count per triple-term predicate and annotation count per annotation predicate;
   - a small number of multi-column statistics: (predicate, object kind/datatype), and (predicate, object) frequencies for low-cardinality objects such as classes and enum-like values.
   Each carries whether it is exact or an upper bound.
2. **Mergeability as part of the contract.** Counts sum; distinct counts carry either an exact sorted-id merge path or a mergeable sketch, flagged as such. `PageSummary` and segmented summaries implement the seam from data they already store.
3. **The estimator reads them**, including for reifier/annotation patterns and quoted-triple positions, and replaces the global `1/T` with per-predicate distinct counts where available.
4. **The rule engine** can use the same seam for its connected-order tie-break (#479).
5. Statistics stay deterministic and snapshot-bound, as the estimate is today (G10's "cost estimation is a function of the snapshot alone").

## Acceptance criteria

- On a dataset with 10^6 reified claims, the estimate for `?r rdf:reifies ?t . ?r :validFrom ?v` is within 2x of the actual row count (today the quoted-triple position is unconstrained).
- A paged dataset's plan uses the stored reifier counts (test: plan changes when only reifier counts change).
- `?x a :RareClass . ?x :p ?y` is planned class-first when `:RareClass` has few instances and `:p` many, on both `RdfDataset` and paged backends.
- A consumer-supplied statistics implementation can be passed in and changes the plan; EXPLAIN shows which statistics were exact.
- Summaries from two segments merge to the same statistics as one segment holding both (property test).

## Related

#349 (a selectivity statistic recorded on a retrieval snapshot), #479, #483 (planner safety, which needs these numbers for its guaranteed bounds), #491 (reifier and annotation indexes).


## Comments (0)

