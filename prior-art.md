# Prior art — issue #596: OWL DL: expose checked source-bound causal clash dependencies

Assembled by `stagectl brief`. Every hash and path below came from this
repository; nothing here is recalled or inferred. Read the Coverage
section at the bottom before concluding anything is absent — an empty
section can mean 'nothing found' or 'could not look'.

## Governing ADRs

The issue cites no ADRs.

## Linked issues (2-hop `#N` crawl)

- #501: Entailment: reusable snapshot-bound proof and justification query sessions  [OPEN]  (mentions)
- #596: OWL DL: expose checked source-bound causal clash dependencies  [OPEN]  (mentions)
- #597: Share owned-or-borrowed storage to repair native helper hygiene  [CLOSED]  (mentions)
- #598: Share inline immutable owned-or-borrowed storage  [MERGED]  (mentions)
- #493: datalog: incremental maintenance of a materialized model under insertions and retractions (Z-set weights already compile)  [OPEN]  (2-hop)
- #488: Datalog, chase and entailment: certified partial models and resumable evaluation instead of total refusal  [OPEN]  (2-hop)
- #476: OWL 2 DL: close the four ledgered W3C OWL 2 consistency divergences  [CLOSED]  (2-hop)

## Prior attempts at this issue

Commits citing it (including inside squash merges):

- None found.

Other branches named for it:

- None.

PRs citing it:

- #598: Share inline immutable owned-or-borrowed storage  [MERGED]

## Related items on the forge

- #558: Structured access to error variants and fields, so consumers never render errors to text  [OPEN]
- #586: Replay ordinary DL clash reachability from recorded current support  [CLOSED]
- #563: Segmented scans fetch their block through the cache once per row  [CLOSED]
- #508: sparql-eval: complete bounded workspace admission for stored operational queries and EXPLAIN  [CLOSED]
- #404: Additive complete-state dataset digest  [CLOSED]
- #495: Layered stack over segmented snapshots: open and snapshot without a whole-dictionary rebuild, per-segment dictionaries that purge with their segment  [CLOSED]
- #408: geo: deterministic geodesic distance in metres for CRS84 (geof:metric*)  [CLOSED]
- #457: Paged tier: an LSM-style stack of sealed delta generations with removals, read as one snapshot  [CLOSED]
- #478: SPARQL governor: governed parallel row loops regressed 1.2–1.65× at 32 threads (block scheduling)  [CLOSED]
- #490: Typed capacity errors instead of u32 overflow panics, and 64-bit room for Datalog ids and global dictionary text  [CLOSED]
- #482: text: rank inputs past QUERY_TERMS_MAX, DOCUMENTS_MAX and FIELD_LENGTH_MAX exactly instead of refusing them  [CLOSED]
- #499: Entailment: prepare schema-invariant cardinality contradictions once per class  [CLOSED]
- #469: SPARQL governor: metered governed evaluation costs 1.4–1.7× ungoverned (target 1.2×)  [CLOSED]
- #518: Optimizer: a ShapeProvider extension for exact physical operators (ordered window, bucketed series, host shapes)  [CLOSED]

## Recurrence

Counted from `GhpRsq-Defect-Class` trailers — these are tallies, not estimates:

- `none`: 19 prior instance(s)
- `performance`: 3 prior instance(s)
- `correctness`: 3 prior instance(s)
- `bug`: 2 prior instance(s)
- `duplicate-implementation`: 1 prior instance(s)
- `source-identity`: 1 prior instance(s)
- `missing-intrinsic-runtime`: 1 prior instance(s)
- `proof-artifact-drift`: 1 prior instance(s)
- `proof-premise-reachability`: 1 prior instance(s)
- `semantic-identity-loss`: 1 prior instance(s)

## Coverage

What was searched, and what was not:

- issue #596 + 18 comment(s) via github
- link crawl: 7 issue(s) within 2 hop(s) of `#N` references
- forge search on ['expose', 'checked', 'source-bound']: 14 other item(s)
- GhpRsq-Defect-Class trailers: 10 distinct class(es) in the last 200 commits
- prior attempts: 0 commit(s) citing #596 (second parents walked), 0 other branch(es) named for it
- PRs citing #596: 1
- requirement candidates: 28 line(s); 3 comment(s) matched no rule
- referenced artifacts: 0 path(s) (0 on disk), 0 identifier(s), 0 workspace package(s), 1 URL(s)
