# Prior art — issue #490: Typed capacity errors instead of u32 overflow panics, and 64-bit room for Datalog ids and global dictionary text

Assembled by `stagectl brief`. Every hash and path below came from this
repository; nothing here is recalled or inferred. Read the Coverage
section at the bottom before concluding anything is absent — an empty
section can mean 'nothing found' or 'could not look'.

## Governing ADRs

The issue cites no ADRs.

## Linked issues (2-hop `#N` crawl)

- #90: Storage-backend seam: unseal DatasetView (or add a paged-dataset trait) with a u64 global dictionary path  [CLOSED]
- #457: Paged tier: an LSM-style stack of sealed delta generations with removals, read as one snapshot  [CLOSED]
- #364: Post-v3.0: evaluation limits, host-parity additions and SHACL/SPARQL follow-ons from #356  [CLOSED]
- #477: xsd: one binary arbitrary-precision integer for the whole workspace  [CLOSED]
- #356: SHACL Processor does not distinguish between built-in and custom functions  [CLOSED]
- #360: fix(shapes)!: built-in declarations bind natively, and SHACL 1.2 conformance  [MERGED]
- #463: geo: deterministic geodesic distance in metres for CRS84 and the native cell grid  [OPEN]
- #443: Optimize the exact numeric tower: small-value parity and subquadratic large-value arithmetic  [OPEN]
- #423: Arbitrary-precision numeric tower for exact reasoned mathematics  [CLOSED]
- #408: geo: deterministic geodesic distance in metres for CRS84 (geof:metric*)  [OPEN]

## Related items on the forge

- #518: Optimizer: a ShapeProvider extension for exact physical operators (ordered window, bucketed series, host shapes)  [OPEN]
- #512: text/sparql-eval: dictionary-match shape -- a VALUES block of phrases on one predicate as one automaton, longest-first spans  [OPEN]
- #508: sparql-eval: complete bounded workspace admission for stored operational queries and EXPLAIN  [CLOSED]
- #394: sparql-eval: fallible EXPLAIN entry point for operational (FallibleDatasetView) storage  [CLOSED]
- #478: SPARQL governor: governed parallel row loops regressed 1.2–1.65× at 32 threads (block scheduling)  [CLOSED]
- #401: Update transfers preserve RDF 1.2 identity: per-document LOAD blank scoping and typed record transfer  [CLOSED]
- #482: text: rank inputs past QUERY_TERMS_MAX, DOCUMENTS_MAX and FIELD_LENGTH_MAX exactly instead of refusing them  [CLOSED]
- #454: rdflib shim: algebra-level reassignment rewrite so every case matches rdflib 7.6  [CLOSED]
- #479: rules: two rdf:type patterns with disjoint subjects are joined as a cross product (quadratic memory in SPARQL 1.2 RL)  [CLOSED]
- #404: Additive complete-state dataset digest  [CLOSED]
- #286: perf: retain shared dataset views through pipeline bundles and GTS ingestion  [CLOSED]
- #295: fix(shapes): sh:pattern compiles as a Rust regex, not an XSD one  [CLOSED]
- #107: feat(columnar): first-party bidirectional Parquet codec  [CLOSED]
- #407: text: analyzer stages for stemming, accent folding, dictionary segmentation, void stripping, token bounds, phonetic codes and substring matching  [CLOSED]

## Recurrence

Counted from `GhpRsq-Defect-Class` trailers — these are tallies, not estimates:

- `none`: 24 prior instance(s)
- `performance`: 2 prior instance(s)
- `semantic-identity-loss`: 1 prior instance(s)

## Coverage

What was searched, and what was not:

- issue #490 + 0 comment(s) via github
- link crawl: 10 issue(s) within 2 hop(s) of `#N` references
- forge search on ['Typed', 'capacity', 'errors']: 14 other item(s)
- GhpRsq-Defect-Class trailers: 3 distinct class(es) in the last 200 commits
