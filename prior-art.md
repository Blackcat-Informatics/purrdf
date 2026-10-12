# Prior art — issue #536: Restore MSRV shapes build and Arm64 numerical replay gates

Assembled by `stagectl brief`. Every hash and path below came from this
repository; nothing here is recalled or inferred. Read the Coverage
section at the bottom before concluding anything is absent — an empty
section can mean 'nothing found' or 'could not look'.

## Governing ADRs

The issue cites no ADRs.

## Linked issues (2-hop `#N` crawl)

- #535: Use host-managed Cargo outputs for WASM and C packaging  [CLOSED]  (mentions)
- #531: Evaluator delivery follow-up: relocated test artifact and unqualified performance evidence  [OPEN]  (mentions)
- #533: Complete geo qualification and repair final-round findings  [OPEN]  (mentions)
- #515: sparql-eval/datalog: per-row evidence -- information state per standpoint, completeness, frame -- combined by the bilattice  [CLOSED]  (2-hop)
- #516: Loss ledger as queryable RDF 1.2: content-identified reasons, and merging accepted reasons into the answer  [CLOSED]  (2-hop)
- #511: retrieval: field scoping applied inside each stratum's candidate generation, before depth, with undecided counts  [CLOSED]  (2-hop)
- #518: Optimizer: a ShapeProvider extension for exact physical operators (ordered window, bucketed series, host shapes)  [CLOSED]  (2-hop)
- #512: text/sparql-eval: dictionary-match shape -- a VALUES block of phrases on one predicate as one automaton, longest-first spans  [CLOSED]  (2-hop)
- #408: geo: deterministic geodesic distance in metres for CRS84 (geof:metric*)  [CLOSED]  (2-hop)
- #409: geo: deterministic hierarchical spatial cell index for CRS84 points (assignment, parent/child, covering)  [CLOSED]  (2-hop)

## Prior attempts at this issue

Commits citing it (including inside squash merges):

- None found.

Other branches named for it:

- None.

PRs citing it:

- None found.

## Related items on the forge

- #531: Evaluator delivery follow-up: relocated test artifact and unqualified performance evidence  [OPEN]
- #323: perf(sparql-eval,shapes): audit SHACL validation allocation profile on the change path  [CLOSED]
- #317: perf(shapes): restore prepared SHACL products across processes without reparsing  [CLOSED]
- #402: SHACL dated profile selection and complete validation reports (additive)  [CLOSED]
- #348: SHACL-AF sh:SPARQLFunction bodies cannot reach a registered relation  [CLOSED]
- #356: SHACL Processor does not distinguish between built-in and custom functions  [CLOSED]
- #467: XPath regex: large finite counted repetitions are refused on 1–4 MiB input  [CLOSED]
- #424: gts: incremental MMR accumulator (append a frame id in O(log n), root from peaks)  [CLOSED]
- #382: SHACL pre-binding leaves `$this` unbound in a triple pattern inside `OPTIONAL`  [CLOSED]
- #345: perf(sparql-eval): remove the per-query execution setup allocations on the SHACL change path  [CLOSED]
- #366: Centralize overlapping helpers: one home per job, gates against regrowth, and the defects the survey found  [CLOSED]
- #479: rules: two rdf:type patterns with disjoint subjects are joined as a cross product (quadratic memory in SPARQL 1.2 RL)  [CLOSED]
- #393: EXPLAIN aborts the process on a large join  [CLOSED]
- #392: A pattern after OPTIONAL or BIND, or an OPTIONAL body, is evaluated without the bindings before it  [CLOSED]

## Recurrence

Counted from `GhpRsq-Defect-Class` trailers — these are tallies, not estimates:

- `none`: 25 prior instance(s)
- `semantic-identity-loss`: 2 prior instance(s)
- `performance`: 2 prior instance(s)

## Coverage

What was searched, and what was not:

- issue #536 + 1 comment(s) via github
- link crawl: 10 issue(s) within 2 hop(s) of `#N` references
- forge search on ['Restore', 'shapes', 'build']: 14 other item(s)
- GhpRsq-Defect-Class trailers: 3 distinct class(es) in the last 200 commits
- prior attempts: 0 commit(s) citing #536 (second parents walked), 0 other branch(es) named for it
- PRs citing #536: 0
- requirement candidates: 1 line(s); 1 comment(s) matched no rule
- referenced artifacts: 4 path(s) (4 on disk), 7 identifier(s), 2 workspace package(s), 0 URL(s)
