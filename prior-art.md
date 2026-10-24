# Prior art — issue #704: Remove panicking canonicalization APIs and propagate native refusals

Assembled by `stagectl brief`. Every hash and path below came from this
repository; nothing here is recalled or inferred. Read the Coverage
section at the bottom before concluding anything is absent — an empty
section can mean 'nothing found' or 'could not look'.

## Governing ADRs

The issue cites no ADRs.

## Linked issues (2-hop `#N` crawl)

- #489: Make the RDFC call limit and SPARQL UDF depth removable, and stop canonicalize from panicking on exhaustion  [OPEN]  (mentions)
- #486: One measured memory budget for every allocation, with spill to a host-provided seam instead of refusal  [OPEN]  (2-hop)
- #364: Post-v3.0: evaluation limits, host-parity additions and SHACL/SPARQL follow-ons from #356  [CLOSED]  (2-hop)
- #488: Datalog, chase and entailment: certified partial models and resumable evaluation instead of total refusal  [OPEN]  (2-hop)

## Prior attempts at this issue

Commits citing it (including inside squash merges):

- None found.

Other branches named for it:

- None.

PRs citing it:

- None found.

## Related items on the forge

- #489: Make the RDFC call limit and SPARQL UDF depth removable, and stop canonicalize from panicking on exhaustion  [OPEN]
- #292: Add fallible borrowed flat-assertion canonicalization for shared dataset views  [CLOSED]
- #286: perf: retain shared dataset views through pipeline bundles and GTS ingestion  [CLOSED]
- #89: GTS-SPEC §10 compaction: verify/complete implementation for signature-preserving repack  [CLOSED]
- #9: Full W3C conformance (SPARQL 1.1/1.2 · RDFC-1.0 · IRI · syntax) + rdflib test-suite gate  [CLOSED]
- #179: SPARQL evaluation budgets: fuel/deadline/row-cap governors in EvalOptions with a typed budget-exhausted outcome  [CLOSED]

## Recurrence

Counted from `GhpRsq-Defect-Class` trailers — these are tallies, not estimates:

- `none`: 16 prior instance(s)
- `source-error-erasure`: 2 prior instance(s)

## Coverage

What was searched, and what was not:

- issue #704 + 0 comment(s) via github
- link crawl: 4 issue(s) within 2 hop(s) of `#N` references
- forge search on ['Remove', 'panicking', 'canonicalization']: 6 other item(s)
- GhpRsq-Defect-Class trailers: 2 distinct class(es) in the last 200 commits
- prior attempts: 0 commit(s) citing #704 (second parents walked), 0 other branch(es) named for it
- PRs citing #704: 0
- requirement candidates: 5 line(s); 0 comment(s) matched no rule
- referenced artifacts: 0 path(s) (0 on disk), 0 identifier(s), 0 workspace package(s), 0 URL(s)
