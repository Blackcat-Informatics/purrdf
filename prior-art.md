# Prior art — issue #484: Planner statistics: per-predicate, per-class, distinct and multi-column counts, including reifier and annotation rows, mergeable across segments

Assembled by `stagectl brief`. Every hash and path below came from this
repository; nothing here is recalled or inferred. Read the Coverage
section at the bottom before concluding anything is absent — an empty
section can mean 'nothing found' or 'could not look'.

## Governing ADRs

The issue cites no ADRs.

## Linked issues (2-hop `#N` crawl)

None referenced.

## Prior attempts at this issue

Commits citing it (including inside squash merges):

- None found.

Other branches named for it:

- None.

PRs citing it:

- None found.

## Related items on the forge

- #485: sparql-eval: merge joins over sorted inputs as the default, and push build-side id ranges into probe-side scans  [OPEN]
- #491: RDF 1.2: index reifiers by triple term and annotations by (predicate, object); reifiers_of is a linear scan  [CLOSED]
- #349: A selectivity-only statistic narrows a retrieval plan but is not recorded on its snapshot  [CLOSED]
- #479: rules: two rdf:type patterns with disjoint subjects are joined as a cross product (quadratic memory in SPARQL 1.2 RL)  [CLOSED]
- #319: Design: a composition layer over the ranked property-function producers  [CLOSED]
- #385: An alternative path of plain predicates is not translated, and a UNION in its place joins by position  [CLOSED]
- #339: Contract for an index-backed ranked producer, and three gaps found meeting it  [CLOSED]
- #346: Bounded read does not reach a multi-modal request: two configurations the narrowing cannot cover  [CLOSED]
- #182: SPARQL breadth roadmap: extensions and standards beyond the W3C 1.2 surface  [CLOSED]
- #344: SIMD sweep: the stable-channel ceiling, fast variant functions, and a full hot-path audit  [CLOSED]
- #185: Property-function (magic predicate) registry  [CLOSED]
- #340: purrdf-retrieval: a fused answer cannot say which strata were served approximately  [CLOSED]
- #10: EPIC — purrdf public maturity: full conformance · optimization · ecosystem  [CLOSED]
- #179: SPARQL evaluation budgets: fuel/deadline/row-cap governors in EvalOptions with a typed budget-exhausted outcome  [CLOSED]

## Recurrence

Counted from `GhpRsq-Defect-Class` trailers — these are tallies, not estimates:

- `none`: 24 prior instance(s)
- `bug`: 2 prior instance(s)
- `performance`: 2 prior instance(s)
- `semantic-identity-loss`: 2 prior instance(s)

## Coverage

What was searched, and what was not:

- issue #484 + 4 comment(s) via github
- link crawl: no `#N` references in the issue or its comments
- forge search on ['Planner', 'statistics:', 'per-predicate,']: 14 other item(s)
- GhpRsq-Defect-Class trailers: 4 distinct class(es) in the last 200 commits
- prior attempts: 0 commit(s) citing #484 (second parents walked), 0 other branch(es) named for it
- PRs citing #484: 0
- requirement candidates: 9 line(s); 2 comment(s) matched no rule
- referenced artifacts: 21 path(s) (2 on disk), 10 identifier(s), 1 workspace package(s), 0 URL(s)
