# Prior art — issue #556: Segmented stack reads an ordinary quad as an annotation when its subject is a reifier in another layer

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

- #494: Evaluate at caller-supplied coordinates: host-supplied NOW(), as-of snapshots of a PagedStack, and range scans on annotation values  [OPEN]
- #553: Segmented stack view: bound-reifier patterns drop annotations and scan the reifier table; block cache lookup is linear  [CLOSED]
- #551: Segmented stack queries are quadratic in terms touched: Memory::reserve grows by one with a full copy  [CLOSED]
- #490: Typed capacity errors instead of u32 overflow panics, and 64-bit room for Datalog ids and global dictionary text  [CLOSED]
- #495: Layered stack over segmented snapshots: open and snapshot without a whole-dictionary rebuild, per-segment dictionaries that purge with their segment  [CLOSED]
- #508: sparql-eval: complete bounded workspace admission for stored operational queries and EXPLAIN  [CLOSED]
- #492: Triple-only carrier for RDF 1.2 consumers that use no named graphs (rows, permutations, Datalog atoms, identities)  [CLOSED]
- #375: Prep v3.0 release  [CLOSED]
- #425: Keep general semantic coverage native and focus existing WASM execution on target behavior  [CLOSED]

## Recurrence

Counted from `GhpRsq-Defect-Class` trailers — these are tallies, not estimates:

- `none`: 24 prior instance(s)
- `bug`: 2 prior instance(s)
- `performance`: 2 prior instance(s)
- `semantic-identity-loss`: 2 prior instance(s)

## Coverage

What was searched, and what was not:

- issue #556 + 0 comment(s) via github
- link crawl: no `#N` references in the issue or its comments
- forge search on ['Segmented', 'stack', 'reads']: 9 other item(s)
- GhpRsq-Defect-Class trailers: 4 distinct class(es) in the last 200 commits
- prior attempts: 0 commit(s) citing #556 (second parents walked), 0 other branch(es) named for it
- PRs citing #556: 0
- requirement candidates: 0 line(s); 0 comment(s) matched no rule
- referenced artifacts: 2 path(s) (0 on disk), 9 identifier(s), 1 workspace package(s), 0 URL(s)
