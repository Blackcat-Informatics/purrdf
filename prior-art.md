# Prior art — issue #542: rdf-core: MutableDataset::freeze uses fixed default ViewLimits; allow caller limits

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

- #400: Opt-in remembered empty named graphs for the mutable dataset and SPARQL Update  [CLOSED]
- #286: perf: retain shared dataset views through pipeline bundles and GTS ingestion  [CLOSED]
- #90: Storage-backend seam: unseal DatasetView (or add a paged-dataset trait) with a u64 global dictionary path  [CLOSED]
- #358: Expose async SERVICE resolution to JavaScript/Wasm hosts  [CLOSED]
- #331: Paged tier: graph-selective queries materialize every page  [CLOSED]
- #56: feat(rdf): public RDF Collection/list materializer (rdf:first/rdf:rest) over RdfDataset / slice::Dataset  [CLOSED]
- #192: SEP-0008 SHA-3 builtins  [CLOSED]
- #190: CONSTRUCT GRAPH (quads) + CONSTRUCT/DESCRIBE conformance corpora  [CLOSED]
- #59: test(sparql): confirm + regression-cover FILTER-NOT-EXISTS-with-arithmetic and all-FILTER UNION branch (0.2.0 empty-result quirks)  [CLOSED]
- #58: fix(rdf): blank-node scope collapse in flat_dataset_from_quads risks conflating anonymous nodes on merge  [CLOSED]
- #57: feat(slice): named-graph selection + RDF-1.2 triple-term interiors in rdf_query::Dataset  [CLOSED]
- #179: SPARQL evaluation budgets: fuel/deadline/row-cap governors in EvalOptions with a typed budget-exhausted outcome  [CLOSED]
- #22: SPARQL NOW() returns Unix epoch (1970-01-01) instead of the current query time  [CLOSED]
- #9: Full W3C conformance (SPARQL 1.1/1.2 · RDFC-1.0 · IRI · syntax) + rdflib test-suite gate  [CLOSED]

## Recurrence

Counted from `GhpRsq-Defect-Class` trailers — these are tallies, not estimates:

- `none`: 26 prior instance(s)
- `semantic-identity-loss`: 2 prior instance(s)
- `performance`: 2 prior instance(s)

## Coverage

What was searched, and what was not:

- issue #542 + 0 comment(s) via github
- link crawl: no `#N` references in the issue or its comments
- forge search on ['rdf-core:', 'MutableDataset::freeze', 'fixed']: 14 other item(s)
- GhpRsq-Defect-Class trailers: 3 distinct class(es) in the last 200 commits
- prior attempts: 0 commit(s) citing #542 (second parents walked), 0 other branch(es) named for it
- PRs citing #542: 0
- requirement candidates: 0 line(s); 0 comment(s) matched no rule
- referenced artifacts: 1 path(s) (1 on disk), 5 identifier(s), 0 workspace package(s), 0 URL(s)
