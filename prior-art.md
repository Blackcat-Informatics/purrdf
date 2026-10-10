# Prior art — issue #482: text: rank inputs past QUERY_TERMS_MAX, DOCUMENTS_MAX and FIELD_LENGTH_MAX exactly instead of refusing them

Assembled by `stagectl brief`. Every hash and path below came from this
repository; nothing here is recalled or inferred. Read the Coverage
section at the bottom before concluding anything is absent — an empty
section can mean 'nothing found' or 'could not look'.

## Governing ADRs

### ADR-0030 — (title not parsed)

- NOT FOUND on disk

## Linked issues (2-hop `#N` crawl)

- #423: Arbitrary-precision numeric tower for exact reasoned mathematics  [CLOSED]
- #477: xsd: one binary arbitrary-precision integer for the whole workspace  [CLOSED]
- #298: text: declare the BM25F score bound and width inside a named ranking-profile identity; decide FM-index ownership under the analyzer-identity contract  [CLOSED]
- #412: text: BM25F per-field document populations for sparse-field corpora  [CLOSED]
- #422: Bounded numerics that never fail silently: documented xsd:integer/xsd:decimal limits, typed overflow errors, exact comparison at any size  [CLOSED]
- #463: geo: deterministic geodesic distance in metres for CRS84 and the native cell grid  [OPEN]
- #443: Optimize the exact numeric tower: small-value parity and subquadratic large-value arithmetic  [OPEN]
- #408: geo: deterministic geodesic distance in metres for CRS84 (geof:metric*)  [OPEN]

## Related items on the forge

None found by title-term search.

## Recurrence

Counted from `GhpRsq-Defect-Class` trailers — these are tallies, not estimates:

- `none`: 19 prior instance(s)
- `semantic-identity-loss`: 1 prior instance(s)
- `performance`: 1 prior instance(s)

## Coverage

What was searched, and what was not:

- issue #482 + 0 comment(s) via github
- link crawl: 8 issue(s) within 2 hop(s) of `#N` references
- ADR-0030 cited by the issue but no file in docs/adr/
- ADRs: 1 cited; 0 resolved on disk
- forge search on ['text:', 'inputs', 'QUERY_TERMS_MAX,']: 0 other item(s)
- GhpRsq-Defect-Class trailers: 3 distinct class(es) in the last 200 commits
