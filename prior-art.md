# Prior art — issue #491: RDF 1.2: index reifiers by triple term and annotations by (predicate, object); reifiers_of is a linear scan

Assembled by `stagectl brief`. Every hash and path below came from this
repository; nothing here is recalled or inferred. Read the Coverage
section at the bottom before concluding anything is absent — an empty
section can mean 'nothing found' or 'could not look'.

## Governing ADRs

### ADR-0038 — (title not parsed)

- NOT FOUND on disk

## Linked issues (2-hop `#N` crawl)

- #484: Planner statistics: per-predicate, per-class, distinct and multi-column counts, including reifier and annotation rows, mergeable across segments  [OPEN]  (mentions)
- #288: fix(rdf): canonical output of a reifier is refused by the canonicalizer  [CLOSED]  (mentions)
- #401: Update transfers preserve RDF 1.2 identity: per-document LOAD blank scoping and typed record transfer  [CLOSED]  (mentions)
- #479: rules: two rdf:type patterns with disjoint subjects are joined as a cross product (quadratic memory in SPARQL 1.2 RL)  [CLOSED]  (2-hop)
- #483: sparql-eval: never materialize a cross product (factorized components) and give every plan a guaranteed output bound  [OPEN]  (2-hop)
- #349: A selectivity-only statistic narrows a retrieval plan but is not recorded on its snapshot  [CLOSED]  (2-hop)
- #491: RDF 1.2: index reifiers by triple term and annotations by (predicate, object); reifiers_of is a linear scan  [OPEN]  (2-hop)

## Prior attempts at this issue

Commits citing it (including inside squash merges):

- None found.

Other branches named for it:

- None.

PRs citing it:

- None found.

## Related items on the forge

- #496: Benchmarks: measured evidence at 10^8-10^9 triples, including a reifier-heavy RDF 1.2 claims workload  [OPEN]
- #487: sparql-eval: vectorized execution over batches of term ids with selection vectors  [OPEN]
- #492: Triple-only carrier for RDF 1.2 consumers that use no named graphs (rows, permutations, Datalog atoms, identities)  [CLOSED]
- #490: Typed capacity errors instead of u32 overflow panics, and 64-bit room for Datalog ids and global dictionary text  [CLOSED]
- #286: perf: retain shared dataset views through pipeline bundles and GTS ingestion  [CLOSED]
- #457: Paged tier: an LSM-style stack of sealed delta generations with removals, read as one snapshot  [CLOSED]
- #383: Read one named graph of a native dataset as the default graph  [CLOSED]
- #93: Discussion: HDT-style succinct queryable-while-compressed codec for read-only reference bundles  [CLOSED]
- #495: Layered stack over segmented snapshots: open and snapshot without a whole-dictionary rebuild, per-segment dictionaries that purge with their segment  [CLOSED]
- #508: sparql-eval: complete bounded workspace admission for stored operational queries and EXPLAIN  [CLOSED]
- #80: Add semantic RDF visualization SVG exports and library APIs  [CLOSED]
- #92: gts replication: inventory-diff → segment-range fetch helper and streaming-fold index-builder conveniences  [CLOSED]
- #105: feat(rdf): unify JSON-LD/YAML-LD into the native format registry  [CLOSED]
- #356: SHACL Processor does not distinguish between built-in and custom functions  [CLOSED]

## Recurrence

Counted from `GhpRsq-Defect-Class` trailers — these are tallies, not estimates:

- `none`: 24 prior instance(s)
- `bug`: 2 prior instance(s)
- `performance`: 2 prior instance(s)
- `semantic-identity-loss`: 2 prior instance(s)

## Coverage

What was searched, and what was not:

- issue #491 + 0 comment(s) via github
- link crawl: 7 issue(s) within 2 hop(s) of `#N` references
- ADR-0038 cited by the issue but no file in docs/adr/
- ADRs: 1 cited; 0 resolved on disk
- forge search on ['index', 'reifiers', 'triple']: 14 other item(s)
- GhpRsq-Defect-Class trailers: 4 distinct class(es) in the last 200 commits
- prior attempts: 0 commit(s) citing #491 (second parents walked), 0 other branch(es) named for it
- PRs citing #491: 0
- requirement candidates: 4 line(s); 0 comment(s) matched no rule
- referenced artifacts: 11 path(s) (5 on disk), 9 identifier(s), 1 workspace package(s), 0 URL(s)
