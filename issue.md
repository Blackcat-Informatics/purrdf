# Issue #1281: 4.0.2: SHACL and SPARQL evaluation on a frozen dataset is 1.5x slower than at `4a23c91e`; first use pays a full-content hash

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

Follow-up to #1249. `freeze()` is fast again, but the time after the freeze grew.

## Summary

One frozen `RdfDataset` with 10.3M quads in the default graph and 11.7M quads in 117 named graphs. We
validate it with SHACL shapes that use SPARQL targets, then run 67 SPARQL SELECT queries. Only the
purrdf version changes, and the results are byte-identical.

| Part | `4a23c91e` | 4.0.2 | 4.0.2, `snapshot_identity()` patched to `None` |
|---|---:|---:|---:|
| Snapshot identity on first use | — | +12 s | 0 |
| SPARQL target resolution inside SHACL binds | 8.3 s | 24.8 s | 25.7 s |
| SHACL validation of the bound shapes | 29.4 s | 41.3 s | 40.6 s |
| 67 SPARQL SELECT queries | 29.4 s | 43.9 s | 46.2 s |
| Whole run | 122.2 s | 187.1 s | 175.2 s |

The identity explains about 12 s. The other 45 s is slower SPARQL and SHACL evaluation.

## 1. Snapshot identity

The first call to `snapshot_identity()` hashes every row on one thread, about 12 s on 10M rows. It sits
behind a `OnceLock`, so concurrent callers all wait for it. Every run reaches it through two callers:

- `class_snapshot` (`purrdf-shapes/src/class_membership.rs:482-490`), on every SHACL bind;
- `capture_request` (`purrdf-sparql-eval/src/engine/coordinates.rs:221-224`), on every SPARQL query.

Both callers already handle `None`, and `DatasetView::snapshot_identity` defaults to `Ok(None)`
(`purrdf-core/src/dataset_view.rs:714`). Only `RdfDataset` always returns `Some` (`:2019`).

**Proposal.** A public way to get an `RdfDataset` whose `snapshot_identity()` returns `None`, for
example a freeze option. We use neither execution receipts nor a cross-run class-membership cache. With
that patch the run is 12 s faster and the results do not change. If the identity must stay, hashing in
parallel would shorten the wait.

## 2. SPARQL evaluation is 1.1–2.1x slower

| Query | `4a23c91e` | 4.0.2 | Ratio | Constructs |
|---|---:|---:|---:|---|
| Q1 | 2.34 s | 5.00 s | 2.14 | NOT EXISTS, GROUP BY, HAVING, COUNT |
| Q2 | 0.93 s | 1.55 s | 1.67 | FILTER, property path |
| Q3 | 3.94 s | 6.52 s | 1.65 | NOT EXISTS, GROUP BY, subquery |
| Q4 | 5.50 s | 7.82 s | 1.42 | FILTER |
| Q5 | 4.88 s | 5.48 s | 1.12 | NOT EXISTS, FILTER |

The 17 queries over 0.2 s take 27.6 s and 41.2 s. Every construct is slower, plain FILTER included, so
this looks like a per-solution cost. Many of the queries compare `xsd:float` values. Could the exact
numerics of 4.0.0, the planner or per-row progress checks cause it?

The data is private. We can run any instrumented build or patch you send and report the numbers.

## Acceptance we suggest

End to end on a 10M-quad dataset, compared with `4a23c91e`: freeze, the first SHACL bind with class
membership, then SPARQL SELECT queries with FILTER, NOT EXISTS and GROUP BY. #1249 was accepted on
`freeze()` alone, which does not show this cost.


## Comments (7)

### paudley — 2026-11-12T16:55:22Z

Almost there!  Thanks for the second report.

### paudley — 2026-11-12T17:00:16Z

Thank you for the table: separating the identity from the rest, and patching it out to measure, is what makes this actionable. This is taken for the next release, 4.0.3.

The two parts are being handled separately, because they are not the same work.

**Snapshot identity.** The cost is as you describe: a full-content hash on first use, on one thread, with every concurrent caller waiting behind it. Both callers you name already accept an absent identity. What has to be decided is the shape that keeps 4.x compatibility: an additive way to obtain a dataset that reports no identity, a cheaper identity, or both. We are checking first what else depends on the identity being present, so that making it optional does not quietly disable something another caller relies on.

**Evaluation after the freeze.** We do not have a cause yet and will not guess at one. There are about 2,300 commits between `4a23c91e` and 4.0.2 on those three paths, and the first step is to read what changed on each: target resolution inside a SHACL bind, validation of the bound shapes, and SELECT evaluation. Your observation that every construct is slower, plain FILTER included, and that many queries compare `xsd:float` values, is a useful lead and will be tested, not assumed.

A bench that reproduces each row of your table on synthetic data, at a size we can run, goes in before any fix, so that each change is measured against it. Your suggested acceptance is the right one and we are adopting it: freeze, the first SHACL bind with class membership, then SELECT queries with FILTER, NOT EXISTS and GROUP BY, end to end, against `4a23c91e`.

Three things would help the bench match your load, none of which needs the data itself:

1. The shape of the SPARQL targets: roughly how many shapes use a SPARQL target, and whether the target queries are simple class or property selections or carry FILTER and joins.
2. For Q1, Q3 and Q4, the query text with identifiers replaced, or just the structure: the number of triple patterns, where the FILTER sits, and what is compared with what.
3. Whether the queries run one at a time or concurrently, and on how many threads.

We will take you up on the offer to run an instrumented build once there is a candidate worth your time.


### paudley — 2026-11-12T17:16:12Z

A first reading of the history between `4a23c91e` and 4.0.2 is done. Nothing is measured yet, so what follows is where we are looking, not a diagnosis.

**The identity.** Your "+12 s" is the same full hash that the fix for #1249 moved out of `freeze()`: it was eager there before, and is now paid on first use. Both callers you name take it on every query and every bind, whether or not anything consumes it. The first change we intend is for them to take it only when a receipt or a cache will use it, which should remove the cost for a run like yours with no new option. Hashing the rows in parallel is a second step, and it does not change the digest. An explicit way to obtain a dataset with no identity is possible and additive, but it would silently disable execution receipts, so it comes last and only if it is still wanted after the first two.

**The evaluation.** The strongest candidate from reading, for the SELECT queries, is related to the same fix. Planning a query with several triple patterns now asks the dataset for per-pattern statistics. On a dataset frozen in the ordinary way those are no longer built during `freeze()`, and each request appears to scan the quads under a lock. At `4a23c91e` the planner counted from index ranges. If that is the cause, the cost moved from `freeze()` into the first planning of each distinct query, which would fit "freeze is fast again, but the time after the freeze grew". It would not explain the SHACL target resolution, which uses a view that reports no statistics, and that part is unexplained so far.

What the reading did not support: the float comparison reaches the same comparison as before, an ungoverned query still skips the per-row checks, and the release profile is unchanged. Those will be measured anyway.

Three answers would tell us whether that candidate even applies to your run:

1. Do the 67 SELECT queries run directly on the frozen `RdfDataset`, or on a view over it (a composite, a named-graph selection, a SHACL view)?
2. Which entry point do you call for them, and do you pass governors or a memory budget, or the defaults?
3. Are the 67 queries distinct texts, each run once? If any is run twice in one process, is the second run as slow as the first?

If it is easy for you, one experiment would confirm or kill the candidate quickly: build the dataset with `freeze_with_memory` in place of `freeze()` (it builds the statistics during the freeze, so the freeze itself gets slower) and time the 67 queries again. If they return to about 29 s, the candidate is right. If they do not move, it is wrong and we look elsewhere. Either result helps.

The bench that reproduces each row of your table is next, and each change will be measured against it and against `4a23c91e`.


### dshchyhlinski — 2026-11-13T09:18:38Z

Thank you. We ran the experiment and checked the questions against our code.

**`freeze_with_memory` in place of `freeze()`.** Same binary, same input, results byte-identical. The candidate does not hold for the SELECT queries.

| | `4a23c91e` | 4.0.2 `freeze()` | 4.0.2 `freeze_with_memory` |
|---|---:|---:|---:|
| 67 SELECT queries | 32.1 s | 43.2 s | 43.2 s |
| Freeze (two datasets, in parallel) | 0.7 s | 0.7 s | 30.0 s |
| SPARQL target resolution, all SHACL binds | 9.4 s | 42.8 s | 24.1 s |
| SHACL calls in total | 58.6 s | 98.5 s | 84.8 s |
| Whole run | 130.7 s | 186.6 s | 198.5 s |

The eager freeze takes the identity wait out of the SHACL binds, as the `None` patch did. The rest of the gap stays. Target resolution is still 2.6x slower, and the queries do not move.

**Answers**

1. *Where the queries run.* On `ShaclDatasetView::native(Arc<RdfDataset>)` over the frozen dataset, not on the dataset directly. The view does not implement `statistics()` or `statement_cardinality_estimate()`. Its `cardinality_estimate()` forwards to the native index ranges. This may be why the candidate does not apply.
2. *Entry point.* Each text is prepared once with `NativeSparqlEngine::prepare_query`, then run with `query_prepared_governed_view(view, &prepared, &[], &QueryOptions::EMPTY, &governors)`. The governors are `QueryGovernors::UNBOUNDED` with `max_scratch_bytes`, `max_intermediate_cells` and a wall-clock `StopSignal`. So the queries are governed, and the per-row checks run.
3. *Distinct texts.* All 67 texts are distinct, and each runs once per process. We have not run one twice in the same process.
4. *Concurrency.* By default up to 8 queries or SHACL binds run at the same time. All numbers in this issue come from one thread, so one query runs at a time.
5. *SPARQL targets.* 19 shapes use `sh:SPARQLTarget`, beside about 1,450 `sh:targetClass`, 340 `sh:targetSubjectsOf` and 13 `sh:targetNode`. Most target queries have two triple patterns joined by a sequence path, with no FILTER. Two use GROUP BY, two use FILTER NOT EXISTS, and three have six or seven patterns.
6. *Structure of Q1, Q3 and Q4.* A correction first: Q1 has no NOT EXISTS, as our table said.

Q1, 2.34 s → 5.00 s. One triple pattern over every `rdf:type` triple, then a group and a count:

```sparql
SELECT ?x (COUNT(?v) AS ?n) ?v
WHERE { ?v rdf:type ?o . }
GROUP BY ?x ?v
HAVING (?n > 1)
```

Q3, 3.94 s → 6.52 s. An aggregating subquery and an outer filter on decimals:

```sparql
SELECT ?x ?sumP ?sumQ WHERE {
  { SELECT ?x (SUM(?p) AS ?sumP) (SUM(?q) AS ?sumQ) WHERE {
      ?x ^ex:a/^ex:b ?f .
      ?f ex:p ?p ; ex:q ?q ; ex:b ?t .
      FILTER NOT EXISTS { ?t ex:c ?e . ?e ex:d false . ?e a ?k . FILTER (?k IN (ex:K1, ex:K2)) }
    } GROUP BY ?x }
  BIND (EXISTS { ?x ^ex:e ?i } AS ?has)
  FILTER (?has = false && ((ABS(?sumP) > 0.1 && ABS(?sumP) < 0.5) || (ABS(?sumQ) > 0.1 && ABS(?sumQ) < 0.5)))
}
```

Q4, 5.50 s → 7.82 s. Eleven triple patterns, one alternative path, three `BIND(IF(...))`, and a FILTER of products and quotients of `xsd:float` values:

```sparql
SELECT ?x ?v1 ?v2 ?v3 ?p ?q WHERE {
  ?l ex:set ?x ; a ?cls ; ex:type ?lt .
  ?lt ex:kind ex:K .
  ?l (ex:v1|ex:v2|ex:v3) ?lv .
  ?x ex:path ?t .
  ?f ex:t ?t .  ?t ex:n ?n .
  ?s ex:n ?n ; ex:v ?v .
  ?f ex:p ?p .  ?f ex:q ?q .
  BIND (IF(?cls = ex:C1, ?lv, 1/0) AS ?v1)
  BIND (IF(?cls = ex:C2, ?lv, 1/0) AS ?v2)
  BIND (IF(?cls = ex:C3, ?lv, 1/0) AS ?v3)
  BIND ((?p*?p + ?q*?q) AS ?a2)
  FILTER ((?cls = ex:C1 && ?a2 != 0 && ?lv*?lv < (?a2/(3*(?v*?v)))*1000000)
       || (?cls = ex:C2 && ?a2 != 0 && ?lv*?lv < ?a2)
       || (?cls = ex:C3 && ?p != 0 && ?lv < ABS(?p)))
}
```

Q1 may be the simplest lead. It has a single triple pattern and needs no join planning, but it is 2.1x slower.


### paudley — 2026-11-14T01:50:33Z

Thank you, this reply changes what we measure. It should have been answered sooner.

## What your answers correct on our side

Our figures so far were taken without governors on the dataset itself. You run the prepared governed entry on `ShaclDatasetView::native(...)` with a scratch ceiling, an intermediate-cell ceiling and a stop signal, each text once. We are now measuring that call as you wrote it, against `4a23c91e`. Acceptance for this issue is your case, not ours.

Read from the code so far, not yet measured:

- **Planner statistics do not reach you.** `ShaclDatasetView` forwards `cardinality_estimate`, `stats_fingerprint` and `snapshot_identity`, and leaves `statistics()`, `statement_cardinality_estimate()` and `statement_indexes()` on the trait defaults, as you found. The change that reads planner statistics from built index runs lives behind `statistics()`, so it does not apply through this view. Planning through the view uses the per-pattern `cardinality_estimate` over index ranges, as before, and has no domain forecasts or statement-layer counts. The view will forward these methods.
- **Your governors select a different loop from the one we measured.** A stop signal forces the sequential chunk driver, and `max_intermediate_cells` selects its cell-bounded form and keeps a `FILTER` from forking. No fuel is engaged, so no per-item charge records are written. The stop signal is polled once per candidate.
- **The snapshot identity.** A change merged for this issue requests it only when a receipt or a cache reads it. That holds for the plain query entries and for SHACL class membership. For the prepared governed entry you call, the code appears to request it on every query still; this is read and not yet confirmed by a run. If it is so, your `None` patch and the eager freeze are measuring a cost we have not removed for you, and that will be fixed here too.

## What is merged or in review for this issue

Compared by instructions per call against `4a23c91e`, on our own fixtures, without governors. Treat these as not yet shown for your configuration.

- Merged: a constant `FILTER` operand is parsed once per query, not once per row. A small gain.
- In review: `NOT EXISTS` evaluation that no longer rebuilds its seed schema and plan for every outer row. On our fixture `FILTER NOT EXISTS` went from 1.81 times the old cost to 0.97, and a `GROUP BY` over a `NOT EXISTS` filter from 1.75 to 1.04.
- In review: cheaper row building for results.
- Not fixed yet: a plain scan costs about twice as many instructions per candidate triple as it did (443 then, 898 now, with no result rows and no aggregate). This is in the pattern scan itself, so it reaches every query, and it is the likeliest reason your Q1 is slow. Plain `GROUP BY` is at 1.48 times on our fixture.
- Not measured by us yet: SPARQL target resolution in SHACL binds. It is being read now.

## What we reconstructed, and one thing we need

The driver calls `NativeSparqlEngine::prepare_query` once, then `query_prepared_governed_view(view, &prepared, &[], &QueryOptions::EMPTY, &governors)` on `ShaclDatasetView::native(Arc<RdfDataset>)`, with `QueryGovernors::UNBOUNDED` plus `max_scratch_bytes`, `max_intermediate_cells` and a stop signal that never trips, on one thread, each text as the first and only run of its process.

For Q1 we assumed data: 20,000 subjects with one `rdf:type` triple each, and a second set with three each. That is a guess. Could you tell us, roughly:

1. how many `rdf:type` triples the dataset has, and how many distinct types;
2. how many quads in all, and whether statements carry reifiers or annotations;
3. for the SHACL binds, about how many focus nodes a typical `sh:SPARQLTarget` shape resolves to.

A synthetic generator you would accept as representative would serve as well. With that, the figures we post here will be for your shape of data and not ours.


### paudley — 2026-11-14T05:24:00Z

First figures from your configuration, and one correction to my last comment.

## Measured on your call

`ShaclDatasetView::native(Arc<RdfDataset>)`, `prepare_query` once, `query_prepared_governed_view` with `QueryGovernors::UNBOUNDED` plus `max_scratch_bytes`, `max_intermediate_cells` and a stop signal, one thread. Instructions per call in millions, `4a23c91e` against the current branch, both built with one compiler. The data is ours: 20,000 subjects, so treat the ratios as indicative until we have your shape.

"Fresh text" is your case as we understand it: another query has already run in the process, then this text is prepared and run once. "First query" is the very first query in the process.

| Shape | Fresh text: then | now | ratio | First query: then | now | ratio |
|---|---:|---:|---:|---:|---:|---:|
| one pattern | 52.2 | 67.7 | 1.30 | 118.8 | 579.2 | 4.88 |
| one pattern and `FILTER` | 66.9 | 92.8 | 1.39 | 133.1 | 604.0 | 4.54 |
| join | 154.0 | 180.3 | 1.17 | 220.6 | 691.5 | 3.14 |
| `FILTER NOT EXISTS` | 107.8 | 217.5 | 2.02 | 174.3 | 728.7 | 4.18 |
| `GROUP BY` with `COUNT` | 34.8 | 67.3 | 1.93 | 100.7 | 578.6 | 5.74 |
| your Q1, one type per subject | 139.5 | 197.2 | 1.41 | 205.5 | 708.5 | 3.45 |
| your Q1, three types per subject | 206.8 | 322.1 | 1.56 | 321.3 | 1,105.8 | 3.44 |

The branch measured here does not yet contain the `NOT EXISTS` and `GROUP BY` work, which is on a second branch. On our plain fixtures that work brings `FILTER NOT EXISTS` to 0.97 of the old cost and `GROUP BY` to 1.48; it has not been measured in your configuration yet.

So in the steady state a query costs 1.2 to 2.0 times what it did, which matches your overall figure, and the worst shapes are the two that work addresses.

## Correction: the snapshot identity

I wrote that the prepared governed entry appears to request the snapshot identity on every query. It does request it on every query, but the dataset caches it, so only the first query on each dataset pays for it. That first query pays a full content hash of the dataset: 433 million instructions on this fixture, whether or not the caller reads the identity from the result. With that capture switched off in a test build, the first query is back in the old range (145.8 for one pattern).

That is the cost your `None` patch and your eager freeze both moved. It is one payment per dataset, not per query, so it explains the SHACL bind figures you posted more than the 67 queries. It will be removed for callers that do not read the identity; how is being decided.

## Still open

- The pattern scan itself: about twice the instructions per candidate triple, all of it in building, storing and releasing the row. This is the common factor behind the 1.2 to 1.6 on the plain shapes and your Q1.
- Planner statistics through `ShaclDatasetView`, as in my last comment.
- SPARQL target resolution in SHACL binds, not measured yet.

The three questions about your data in my last comment still stand.


### dshchyhlinski — 2026-11-14T07:36:44Z

Thank you for the figures. Here is the shape of our data.

1. **`rdf:type`.** The default graph has 1.81M `rdf:type` triples over 61 distinct types. Every typed subject has exactly one type. The five largest types have 405k, 385k, 135k, 134k and 119k subjects. For Q1 this means 1.81M rows and 1.81M groups of size one, and no result rows.
2. **Quads.** 10.3M quads in the default graph, which is the dataset the 67 queries run on. The second dataset holds the same input as 11.7M quads in 117 named graphs, one per file. About one row in six of the default graph is an `rdf:type` triple. There are no reifiers and no annotations.
3. **Focus nodes of the `sh:SPARQLTarget` shapes.** A correction to my last comment: most of the 19 target queries are not sequence-path joins. Most select a file header, `?this a ex:Header ; ex:profile "…"`. They resolve to 0–38 nodes each. All 19:

| Targets | Query shape | Focus nodes |
|---:|---|---:|
| 8 | header type and a literal profile | 0–38 each |
| 1 | `?h ex:profile "…" . ?this a ex:T .`, a cross product with 12 headers | 405k |
| 2 | one pattern over 405k rows, `GROUP BY ?this`, `HAVING` on `COUNT`, `MIN`, `MAX`, `SUM` | 170k and few |
| 1 | `?s ?p ?this` over every quad, with `FILTER(isIRI)`, string tests (`STRSTARTS`, `CONTAINS`, `STRENDS`) and `FILTER NOT EXISTS { ?this a ?t }` | about 70 |
| 1 | two patterns, one with a boolean literal | 11k |
| 3 | type, two inverse links and three to five more patterns, including an alternative path | a subset of 4.7k |
| 3 | a header type with `FILTER NOT EXISTS`, a header type, and one query whose body is all comments | 0 |

The `?s ?p ?this` target and the cross product read the most rows. We expect them to dominate target resolution, but we have not timed the targets one by one.


