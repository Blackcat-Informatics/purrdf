# Prior art — issue #492: Triple-only carrier for RDF 1.2 consumers that use no named graphs (rows, permutations, Datalog atoms, identities)

Assembled by `stagectl brief`. Every hash and path below came from this
repository; nothing here is recalled or inferred. Read the Coverage
section at the bottom before concluding anything is absent — an empty
section can mean 'nothing found' or 'could not look'.

## Governing ADRs

### ADR-0038 — (title not parsed)

- NOT FOUND on disk

### ADR-0032 — (title not parsed)

- NOT FOUND on disk

## Linked issues (2-hop `#N` crawl)

- #479: rules: two rdf:type patterns with disjoint subjects are joined as a cross product (quadratic memory in SPARQL 1.2 RL)  [CLOSED]  (mentions)
- #471: v4.0: remembered empty named graphs become the default  [CLOSED]  (mentions)
- #383: Read one named graph of a native dataset as the default graph  [CLOSED]  (mentions)
- #400: Opt-in remembered empty named graphs for the mutable dataset and SPARQL Update  [CLOSED]  (mentions)
- #492: Triple-only carrier for RDF 1.2 consumers that use no named graphs (rows, permutations, Datalog atoms, identities)  [OPEN]  (mentions)
- #292: Add fallible borrowed flat-assertion canonicalization for shared dataset views  [CLOSED]  (mentions)
- #544: Return typed capacity failures and add sparse segmented dataset stacks  [MERGED]  (mentions)
- #549: Add a graph-free RDF carrier with triple rule execution  [OPEN]  (mentions)
- #551: Segmented stack queries are quadratic in terms touched: Memory::reserve grows by one with a full copy  [CLOSED]  (mentions)
- #490: Typed capacity errors instead of u32 overflow panics, and 64-bit room for Datalog ids and global dictionary text  [CLOSED]  (2-hop)
- #495: Layered stack over segmented snapshots: open and snapshot without a whole-dictionary rebuild, per-segment dictionaries that purge with their segment  [CLOSED]  (2-hop)

## Prior attempts at this issue

Commits citing it (including inside squash merges):

- None found.

Other branches named for it:

- None.

PRs citing it:

- #549: Add a graph-free RDF carrier with triple rule execution  [OPEN]  (this stage's own PR)

## Related items on the forge

- #495: Layered stack over segmented snapshots: open and snapshot without a whole-dictionary rebuild, per-segment dictionaries that purge with their segment  [CLOSED]
- #292: Add fallible borrowed flat-assertion canonicalization for shared dataset views  [CLOSED]
- #454: rdflib shim: algebra-level reassignment rewrite so every case matches rdflib 7.6  [CLOSED]
- #516: Loss ledger as queryable RDF 1.2: content-identified reasons, and merging accepted reasons into the answer  [CLOSED]
- #490: Typed capacity errors instead of u32 overflow panics, and 64-bit room for Datalog ids and global dictionary text  [CLOSED]
- #286: perf: retain shared dataset views through pipeline bundles and GTS ingestion  [CLOSED]
- #511: retrieval: field scoping applied inside each stratum's candidate generation, before depth, with undecided counts  [CLOSED]
- #515: sparql-eval/datalog: per-row evidence -- information state per standpoint, completeness, frame -- combined by the bilattice  [CLOSED]
- #457: Paged tier: an LSM-style stack of sealed delta generations with removals, read as one snapshot  [CLOSED]
- #479: rules: two rdf:type patterns with disjoint subjects are joined as a cross product (quadratic memory in SPARQL 1.2 RL)  [CLOSED]
- #387: Explore stronger blank-node scope representation and hazard detection  [CLOSED]
- #508: sparql-eval: complete bounded workspace admission for stored operational queries and EXPLAIN  [CLOSED]
- #136: csvw: project_csvw_exact has no curated/summary profile (only the lossless full-carrier package)  [CLOSED]
- #330: Serializers build one whole-output allocation; offer an incremental io::Write sink  [CLOSED]

## Recurrence

Counted from `GhpRsq-Defect-Class` trailers — these are tallies, not estimates:

- `none`: 22 prior instance(s)
- `performance`: 3 prior instance(s)
- `bug`: 2 prior instance(s)
- `semantic-identity-loss`: 2 prior instance(s)

## Coverage

What was searched, and what was not:

- issue #492 + 5 comment(s) via github
- link crawl: 11 issue(s) within 2 hop(s) of `#N` references
- ADR-0038 cited by the issue but no file in docs/adr/
- ADR-0032 cited by the issue but no file in docs/adr/
- ADRs: 2 cited; 0 resolved on disk
- forge search on ['Triple-only', 'carrier', 'consumers']: 14 other item(s)
- GhpRsq-Defect-Class trailers: 4 distinct class(es) in the last 200 commits
- prior attempts: 0 commit(s) citing #492 (second parents walked), 0 other branch(es) named for it
- PRs citing #492: 1
- requirement candidates: 23 line(s); 2 comment(s) matched no rule
- referenced artifacts: 13 path(s) (10 on disk), 14 identifier(s), 0 workspace package(s), 0 URL(s)
