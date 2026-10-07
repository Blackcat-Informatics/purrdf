# Prior art — issue #478: SPARQL governor: governed parallel row loops regressed 1.2–1.65× at 32 threads (block scheduling)

Assembled by `stagectl brief`. Every hash and path below came from this
repository; nothing here is recalled or inferred. Read the Coverage
section at the bottom before concluding anything is absent — an empty
section can mean 'nothing found' or 'could not look'.

## Governing ADRs

The issue cites no ADRs.

## Linked issues (2-hop `#N` crawl)

- #441: xsd: additive arbitrary-precision numeric tower (exact Integer, Decimal, Rational) beside the bounded types  [MERGED]
- #423: Arbitrary-precision numeric tower for exact reasoned mathematics  [CLOSED]
- #469: SPARQL governor: metered governed evaluation costs 1.4–1.7× ungoverned (target 1.2×)  [OPEN]
- #422: Bounded numerics that never fail silently: documented xsd:integer/xsd:decimal limits, typed overflow errors, exact comparison at any size  [CLOSED]
- #440: Bounded numerics that never fail silently, exact comparison at any size  [CLOSED]

## Related items on the forge

- #469: SPARQL governor: metered governed evaluation costs 1.4–1.7× ungoverned (target 1.2×)  [OPEN]
- #454: rdflib shim: algebra-level reassignment rewrite so every case matches rdflib 7.6  [OPEN]
- #395: sparql-eval: prepared + governed entry point for operational (FallibleDatasetView) storage  [CLOSED]
- #369: Python binding surface: follow the Rust changes from one-implementation-per-job  [CLOSED]
- #406: Native XPath regex profile with operational resource refusal  [OPEN]
- #393: EXPLAIN aborts the process on a large join  [CLOSED]
- #392: A pattern after OPTIONAL or BIND, or an OPTIONAL body, is evaluated without the bindings before it  [CLOSED]
- #408: geo: deterministic geodesic distance in metres for CRS84 (geof:metric*)  [OPEN]
- #387: Explore stronger blank-node scope representation and hazard detection  [CLOSED]
- #394: sparql-eval: fallible EXPLAIN entry point for operational (FallibleDatasetView) storage  [CLOSED]
- #358: Expose async SERVICE resolution to JavaScript/Wasm hosts  [CLOSED]
- #400: Opt-in remembered empty named graphs for the mutable dataset and SPARQL Update  [CLOSED]
- #179: SPARQL evaluation budgets: fuel/deadline/row-cap governors in EvalOptions with a typed budget-exhausted outcome  [CLOSED]
- #345: perf(sparql-eval): remove the per-query execution setup allocations on the SHACL change path  [CLOSED]

## Recurrence

Counted from `GhpRsq-Defect-Class` trailers — these are tallies, not estimates:

- `none`: 12 prior instance(s)

## Coverage

What was searched, and what was not:

- issue #478 + 0 comment(s) via github
- link crawl: 5 issue(s) within 2 hop(s) of `#N` references
- forge search on ['SPARQL', 'governor:', 'governed']: 14 other item(s)
- GhpRsq-Defect-Class trailers: 1 distinct class(es) in the last 200 commits
