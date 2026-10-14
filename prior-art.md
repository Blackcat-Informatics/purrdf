# Prior art — issue #545: Complete evaluator acceptance 

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

- #495: Layered stack over segmented snapshots: open and snapshot without a whole-dictionary rebuild, per-segment dictionaries that purge with their segment  [OPEN]
- #481: perf(sparql-eval): reduce grouped-output buffer traffic through fallible admission  [OPEN]
- #443: Optimize the exact numeric tower: small-value parity and subquadratic large-value arithmetic  [OPEN]
- #490: Typed capacity errors instead of u32 overflow panics, and 64-bit room for Datalog ids and global dictionary text  [OPEN]
- #533: Complete geo qualification and repair final-round findings  [OPEN]
- #365: Post-v3.0: asynchronous Wasm lane and parse/serialize follow-ups from #358  [OPEN]
- #492: Triple-only carrier for RDF 1.2 consumers that use no named graphs (rows, permutations, Datalog atoms, identities)  [OPEN]
- #531: Evaluator delivery follow-up: relocated test artifact and unqualified performance evidence  [CLOSED]
- #486: One measured memory budget for every allocation, with spill to a host-provided seam instead of refusal  [OPEN]
- #515: sparql-eval/datalog: per-row evidence -- information state per standpoint, completeness, frame -- combined by the bilattice  [CLOSED]
- #542: rdf-core: MutableDataset::freeze uses fixed default ViewLimits; allow caller limits  [CLOSED]
- #516: Loss ledger as queryable RDF 1.2: content-identified reasons, and merging accepted reasons into the answer  [CLOSED]
- #511: retrieval: field scoping applied inside each stratum's candidate generation, before depth, with undecided counts  [CLOSED]
- #478: SPARQL governor: governed parallel row loops regressed 1.2–1.65× at 32 threads (block scheduling)  [CLOSED]

## Recurrence

Counted from `GhpRsq-Defect-Class` trailers — these are tallies, not estimates:

- `none`: 26 prior instance(s)
- `performance`: 3 prior instance(s)
- `semantic-identity-loss`: 2 prior instance(s)

## Coverage

What was searched, and what was not:

- issue #545 + 0 comment(s) via github
- link crawl: no `#N` references in the issue or its comments
- forge search on ['Complete', 'evaluator', 'acceptance']: 14 other item(s)
- GhpRsq-Defect-Class trailers: 3 distinct class(es) in the last 200 commits
- prior attempts: 0 commit(s) citing #545 (second parents walked), 0 other branch(es) named for it
- PRs citing #545: 0
- requirement candidates: 8 line(s); 0 comment(s) matched no rule
- referenced artifacts: 0 path(s) (0 on disk), 3 identifier(s), 0 workspace package(s), 1 URL(s)
