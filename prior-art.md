# Prior art — issue #472: Turtle writer: deep blank-node chains recurse on the native stack

Assembled by `stagectl brief`. Every hash and path below came from this
repository; nothing here is recalled or inferred. Read the Coverage
section at the bottom before concluding anything is absent — an empty
section can mean 'nothing found' or 'could not look'.

## Governing ADRs

The issue cites no ADRs.

## Linked issues (2-hop `#N` crawl)

None referenced.

## Related items on the forge

- #366: Centralize overlapping helpers: one home per job, gates against regrowth, and the defects the survey found  [CLOSED]
- #181: SHACL 1.2 Rules SPARQLRule: blank nodes not serialized correctly  [CLOSED]
- #287: fix(sparql): non-ASCII whitespace is absorbed into names and silently changes query results  [CLOSED]
- #330: Serializers build one whole-output allocation; offer an incremental io::Write sink  [CLOSED]
- #356: SHACL Processor does not distinguish between built-in and custom functions  [CLOSED]
- #192: SEP-0008 SHA-3 builtins  [CLOSED]
- #190: CONSTRUCT GRAPH (quads) + CONSTRUCT/DESCRIBE conformance corpora  [CLOSED]
- #317: perf(shapes): restore prepared SHACL products across processes without reparsing  [CLOSED]
- #110: epic(rdf): graph & tabular projections (LPG, CSVW, OBO-Graphs, SKOS)  [CLOSED]
- #284: purrdf-markdown: a structural Markdown-to-RDF 1.2 slicer under a caller-supplied vocabulary  [CLOSED]
- #14: ShEx phase 2: imports, semantic actions (Test extension), query shape maps  [CLOSED]
- #9: Full W3C conformance (SPARQL 1.1/1.2 · RDFC-1.0 · IRI · syntax) + rdflib test-suite gate  [CLOSED]
- #358: Expose async SERVICE resolution to JavaScript/Wasm hosts  [CLOSED]
- #48: SARIF 2.1.0 reporting: source-traced, actionable diagnostics and validation reports (Task 12)  [CLOSED]

## Recurrence

Counted from `GhpRsq-Defect-Class` trailers — these are tallies, not estimates:

- `none`: 22 prior instance(s)
- `performance`: 2 prior instance(s)
- `semantic-identity-loss`: 1 prior instance(s)

## Coverage

What was searched, and what was not:

- issue #472 + 2 comment(s) via github
- link crawl: no `#N` references in the issue or its comments
- forge search on ['Turtle', 'writer:', 'blank-node']: 14 other item(s)
- GhpRsq-Defect-Class trailers: 3 distinct class(es) in the last 200 commits
