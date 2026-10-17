# Prior art — issue #538: purrdf-text: update a TextIndex by delta instead of rebuilding it per snapshot

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

- #518: Optimizer: a ShapeProvider extension for exact physical operators (ordered window, bucketed series, host shapes)  [CLOSED]
- #512: text/sparql-eval: dictionary-match shape -- a VALUES block of phrases on one predicate as one automaton, longest-first spans  [CLOSED]
- #482: text: rank inputs past QUERY_TERMS_MAX, DOCUMENTS_MAX and FIELD_LENGTH_MAX exactly instead of refusing them  [CLOSED]
- #337: purrdf-retrieval: a fused answer can contain the same entity twice — what a consumer needs from duplicate handling  [CLOSED]
- #338: purrdf-retrieval: a fused top-k drains every stream — the documented reading bound does not hold  [CLOSED]
- #319: Design: a composition layer over the ranked property-function producers  [CLOSED]
- #339: Contract for an index-backed ranked producer, and three gaps found meeting it  [CLOSED]

## Recurrence

Counted from `GhpRsq-Defect-Class` trailers — these are tallies, not estimates:

- `none`: 21 prior instance(s)
- `performance`: 3 prior instance(s)
- `bug`: 2 prior instance(s)
- `correctness`: 1 prior instance(s)
- `semantic-identity-loss`: 1 prior instance(s)

## Coverage

What was searched, and what was not:

- issue #538 + 0 comment(s) via github
- link crawl: no `#N` references in the issue or its comments
- forge search on ['purrdf-text:', 'update', 'TextIndex']: 7 other item(s)
- GhpRsq-Defect-Class trailers: 5 distinct class(es) in the last 200 commits
- prior attempts: 0 commit(s) citing #538 (second parents walked), 0 other branch(es) named for it
- PRs citing #538: 0
- requirement candidates: 1 line(s); 0 comment(s) matched no rule
- referenced artifacts: 0 path(s) (0 on disk), 10 identifier(s), 1 workspace package(s), 0 URL(s)
