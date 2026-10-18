# Prior art — issue #597: Share owned-or-borrowed storage to repair native helper hygiene

Assembled by `stagectl brief`. Every hash and path below came from this
repository; nothing here is recalled or inferred. Read the Coverage
section at the bottom before concluding anything is absent — an empty
section can mean 'nothing found' or 'could not look'.

## Governing ADRs

The issue cites no ADRs.

## Linked issues (2-hop `#N` crawl)

- #596: OWL DL: expose checked source-bound causal clash dependencies  [OPEN]  (mentions)
- #501: Entailment: reusable snapshot-bound proof and justification query sessions  [OPEN]  (2-hop)

## Prior attempts at this issue

Commits citing it (including inside squash merges):

- None found.

Other branches named for it:

- None.

PRs citing it:

- None found.

## Related items on the forge

- #558: Structured access to error variants and fields, so consumers never render errors to text  [OPEN]
- #594: OWL DL: borrow the prepared proof checking context  [CLOSED]
- #508: sparql-eval: complete bounded workspace admission for stored operational queries and EXPLAIN  [CLOSED]
- #458: purrdf-gts: composite ML-DSA-65 + Ed25519 signatures under COSE_Sign1  [CLOSED]
- #490: Typed capacity errors instead of u32 overflow panics, and 64-bit room for Datalog ids and global dictionary text  [CLOSED]
- #499: Entailment: prepare schema-invariant cardinality contradictions once per class  [CLOSED]
- #518: Optimizer: a ShapeProvider extension for exact physical operators (ordered window, bucketed series, host shapes)  [CLOSED]
- #512: text/sparql-eval: dictionary-match shape -- a VALUES block of phrases on one predicate as one automaton, longest-first spans  [CLOSED]
- #393: EXPLAIN aborts the process on a large join  [CLOSED]
- #392: A pattern after OPTIONAL or BIND, or an OPTIONAL body, is evaluated without the bindings before it  [CLOSED]
- #375: Prep v3.0 release  [CLOSED]
- #387: Explore stronger blank-node scope representation and hazard detection  [CLOSED]
- #85: Eliminate avoidable Rust allocations across core, query, validation, GTS, and bindings for 0.5.0  [CLOSED]
- #384: Build a portable adversarial SHACL suite and contribute missing W3C tests  [OPEN]

## Recurrence

Counted from `GhpRsq-Defect-Class` trailers — these are tallies, not estimates:

- `none`: 19 prior instance(s)
- `performance`: 3 prior instance(s)
- `correctness`: 3 prior instance(s)
- `bug`: 2 prior instance(s)
- `source-identity`: 1 prior instance(s)
- `missing-intrinsic-runtime`: 1 prior instance(s)
- `proof-artifact-drift`: 1 prior instance(s)
- `proof-premise-reachability`: 1 prior instance(s)
- `semantic-identity-loss`: 1 prior instance(s)

## Coverage

What was searched, and what was not:

- issue #597 + 0 comment(s) via github
- link crawl: 2 issue(s) within 2 hop(s) of `#N` references
- forge search on ['Share', 'owned-or-borrowed', 'storage']: 14 other item(s)
- GhpRsq-Defect-Class trailers: 9 distinct class(es) in the last 200 commits
- prior attempts: 0 commit(s) citing #597 (second parents walked), 0 other branch(es) named for it
- PRs citing #597: 0
- requirement candidates: 1 line(s); 0 comment(s) matched no rule
- referenced artifacts: 0 path(s) (0 on disk), 0 identifier(s), 0 workspace package(s), 1 URL(s)
