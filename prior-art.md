# Prior art — issue #494: Evaluate at caller-supplied coordinates: host-supplied NOW(), as-of snapshots of a PagedStack, and range scans on annotation values

Assembled by `stagectl brief`. Every hash and path below came from this
repository; nothing here is recalled or inferred. Read the Coverage
section at the bottom before concluding anything is absent — an empty
section can mean 'nothing found' or 'could not look'.

## Governing ADRs

### ADR-0038 — (title not parsed)

- NOT FOUND on disk

## Linked issues (2-hop `#N` crawl)

- #491: RDF 1.2: index reifiers by triple term and annotations by (predicate, object); reifiers_of is a linear scan  [CLOSED]  (mentions)
- #457: Paged tier: an LSM-style stack of sealed delta generations with removals, read as one snapshot  [CLOSED]  (mentions)
- #484: Planner statistics: per-predicate, per-class, distinct and multi-column counts, including reifier and annotation rows, mergeable across segments  [CLOSED]  (mentions)
- #22: SPARQL NOW() returns Unix epoch (1970-01-01) instead of the current query time  [CLOSED]  (mentions)
- #288: fix(rdf): canonical output of a reifier is refused by the canonicalizer  [CLOSED]  (2-hop)
- #401: Update transfers preserve RDF 1.2 identity: per-document LOAD blank scoping and typed record transfer  [CLOSED]  (2-hop)
- #479: rules: two rdf:type patterns with disjoint subjects are joined as a cross product (quadratic memory in SPARQL 1.2 RL)  [CLOSED]  (2-hop)
- #483: sparql-eval: never materialize a cross product (factorized components) and give every plan a guaranteed output bound  [OPEN]  (2-hop)
- #349: A selectivity-only statistic narrows a retrieval plan but is not recorded on its snapshot  [CLOSED]  (2-hop)

## Prior attempts at this issue

Commits citing it (including inside squash merges):

- None found.

Other branches named for it:

- None.

PRs citing it:

- None found.

## Related items on the forge

- #484: Planner statistics: per-predicate, per-class, distinct and multi-column counts, including reifier and annotation rows, mergeable across segments  [CLOSED]
- #518: Optimizer: a ShapeProvider extension for exact physical operators (ordered window, bucketed series, host shapes)  [CLOSED]
- #512: text/sparql-eval: dictionary-match shape -- a VALUES block of phrases on one predicate as one automaton, longest-first spans  [CLOSED]
- #408: geo: deterministic geodesic distance in metres for CRS84 (geof:metric*)  [CLOSED]
- #320: Offer: a deterministic HNSW index for the ANN slot PURREMB declares but PurRDF cannot fill  [CLOSED]
- #366: Centralize overlapping helpers: one home per job, gates against regrowth, and the defects the survey found  [CLOSED]

## Recurrence

Counted from `GhpRsq-Defect-Class` trailers — these are tallies, not estimates:

- `none`: 21 prior instance(s)
- `performance`: 3 prior instance(s)
- `bug`: 2 prior instance(s)
- `correctness`: 1 prior instance(s)
- `semantic-identity-loss`: 1 prior instance(s)

## Coverage

What was searched, and what was not:

- issue #494 + 0 comment(s) via github
- link crawl: 9 issue(s) within 2 hop(s) of `#N` references
- ADR-0038 cited by the issue but no file in docs/adr/
- ADRs: 1 cited; 0 resolved on disk
- forge search on ['Evaluate', 'caller-supplied', 'coordinates:']: 6 other item(s)
- GhpRsq-Defect-Class trailers: 5 distinct class(es) in the last 200 commits
- prior attempts: 0 commit(s) citing #494 (second parents walked), 0 other branch(es) named for it
- PRs citing #494: 0
- requirement candidates: 4 line(s); 0 comment(s) matched no rule
- referenced artifacts: 13 path(s) (4 on disk), 9 identifier(s), 1 workspace package(s), 0 URL(s)
