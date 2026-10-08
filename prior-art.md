# Prior art — issue #479: rules: two rdf:type patterns with disjoint subjects are joined as a cross product (quadratic memory in SPARQL 1.2 RL)

Assembled by `stagectl brief`. Every hash and path below came from this
repository; nothing here is recalled or inferred. Read the Coverage
section at the bottom before concluding anything is absent — an empty
section can mean 'nothing found' or 'could not look'.

## Governing ADRs

The issue cites no ADRs.

## Linked issues (2-hop `#N` crawl)

- #364: Post-v3.0: evaluation limits, host-parity additions and SHACL/SPARQL follow-ons from #356  [OPEN]
- #483: sparql-eval: never materialize a cross product (factorized components) and give every plan a guaranteed output bound  [OPEN]
- #484: Planner statistics: per-predicate, per-class, distinct and multi-column counts, including reifier and annotation rows, mergeable across segments  [OPEN]
- #488: Datalog, chase and entailment: certified partial models and resumable evaluation instead of total refusal  [OPEN]
- #356: SHACL Processor does not distinguish between built-in and custom functions  [CLOSED]
- #360: fix(shapes)!: built-in declarations bind natively, and SHACL 1.2 conformance  [MERGED]
- #479: rules: two rdf:type patterns with disjoint subjects are joined as a cross product (quadratic memory in SPARQL 1.2 RL)  [OPEN]
- #393: EXPLAIN aborts the process on a large join  [CLOSED]
- #372: Sequence and inverse property paths are not translated to triple patterns (SPARQL 1.1 section 18.2.2.4), so linked patterns become a cross product  [CLOSED]
- #485: sparql-eval: merge joins over sorted inputs as the default, and push build-side id ranges into probe-side scans  [OPEN]
- #486: One measured memory budget for every allocation, with spill to a host-provided seam instead of refusal  [OPEN]
- #349: A selectivity-only statistic narrows a retrieval plan but is not recorded on its snapshot  [CLOSED]
- #491: RDF 1.2: index reifiers by triple term and annotations by (predicate, object); reifiers_of is a linear scan  [OPEN]
- #179: SPARQL evaluation budgets: fuel/deadline/row-cap governors in EvalOptions with a typed budget-exhausted outcome  [CLOSED]
- #103: query: propagate typed page-provider failures, cancellation, and budgets through paged SPARQL execution  [CLOSED]

## Related items on the forge

- #157: SHACL engine: close asserted rdfs:subClassOf for SPARQL-based constraints/targets (or offer opt-in rdf:type materialization)  [CLOSED]
- #356: SHACL Processor does not distinguish between built-in and custom functions  [CLOSED]
- #366: Centralize overlapping helpers: one home per job, gates against regrowth, and the defects the survey found  [CLOSED]
- #59: test(sparql): confirm + regression-cover FILTER-NOT-EXISTS-with-arithmetic and all-FILTER UNION branch (0.2.0 empty-result quirks)  [CLOSED]
- #49: Structural optimization pass: id-native SHACL engine, zero-copy lexer tokens, small-vector + hasher sweep  [CLOSED]
- #173: feat(entail): close the 16 ledgered W3C OWL 2 RL entailment-corpus gaps  [CLOSED]
- #58: fix(rdf): blank-node scope collapse in flat_dataset_from_quads risks conflating anonymous nodes on merge  [CLOSED]
- #57: feat(slice): named-graph selection + RDF-1.2 triple-term interiors in rdf_query::Dataset  [CLOSED]
- #56: feat(rdf): public RDF Collection/list materializer (rdf:first/rdf:rest) over RdfDataset / slice::Dataset  [CLOSED]
- #167: Expose OWL-RL materialize in the python library  [CLOSED]
- #323: perf(sparql-eval,shapes): audit SHACL validation allocation profile on the change path  [CLOSED]

## Recurrence

Counted from `GhpRsq-Defect-Class` trailers — these are tallies, not estimates:

- `none`: 12 prior instance(s)
- `performance`: 1 prior instance(s)

## Coverage

What was searched, and what was not:

- issue #479 + 1 comment(s) via github
- link crawl: 15 issue(s) within 2 hop(s) of `#N` references
- forge search on ['rules:', 'rdf:type', 'patterns']: 11 other item(s)
- GhpRsq-Defect-Class trailers: 2 distinct class(es) in the last 200 commits
