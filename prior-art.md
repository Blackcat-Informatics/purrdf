# Prior art — issue #508: sparql-eval: complete bounded workspace admission for stored operational queries and EXPLAIN

Assembled by `stagectl brief`. Every hash and path below came from this
repository; nothing here is recalled or inferred. Read the Coverage
section at the bottom before concluding anything is absent — an empty
section can mean 'nothing found' or 'could not look'.

## Governing ADRs

The issue cites no ADRs.

## Linked issues (2-hop `#N` crawl)

- #394: sparql-eval: fallible EXPLAIN entry point for operational (FallibleDatasetView) storage  [CLOSED]
- #395: sparql-eval: prepared + governed entry point for operational (FallibleDatasetView) storage  [CLOSED]
- #481: perf(sparql-eval): reduce grouped-output buffer traffic through fallible admission  [OPEN]
- #526: Complete bounded operational SPARQL ownership  [OPEN]
- #443: Optimize the exact numeric tower: small-value parity and subquadratic large-value arithmetic  [OPEN]
- #508: sparql-eval: complete bounded workspace admission for stored operational queries and EXPLAIN  [OPEN]

## Related items on the forge

- #485: sparql-eval: merge joins over sorted inputs as the default, and push build-side id ranges into probe-side scans  [OPEN]
- #471: v4.0: remembered empty named graphs become the default  [OPEN]
- #394: sparql-eval: fallible EXPLAIN entry point for operational (FallibleDatasetView) storage  [CLOSED]
- #395: sparql-eval: prepared + governed entry point for operational (FallibleDatasetView) storage  [CLOSED]
- #393: EXPLAIN aborts the process on a large join  [CLOSED]
- #392: A pattern after OPTIONAL or BIND, or an OPTIONAL body, is evaluated without the bindings before it  [CLOSED]
- #454: rdflib shim: algebra-level reassignment rewrite so every case matches rdflib 7.6  [CLOSED]
- #478: SPARQL governor: governed parallel row loops regressed 1.2–1.65× at 32 threads (block scheduling)  [CLOSED]
- #372: Sequence and inverse property paths are not translated to triple patterns (SPARQL 1.1 section 18.2.2.4), so linked patterns become a cross product  [CLOSED]
- #469: SPARQL governor: metered governed evaluation costs 1.4–1.7× ungoverned (target 1.2×)  [CLOSED]
- #425: Keep general semantic coverage native and focus existing WASM execution on target behavior  [CLOSED]
- #406: Native XPath regex profile with operational resource refusal  [CLOSED]
- #426: XSD constructor casts between xsd:dateTime and xsd:date are unbound  [CLOSED]
- #418: Vendor and run the W3C SPARQL 1.0 data-r2 conformance suites  [CLOSED]

## Recurrence

Counted from `GhpRsq-Defect-Class` trailers — these are tallies, not estimates:

- `none`: 21 prior instance(s)
- `performance`: 2 prior instance(s)
- `semantic-identity-loss`: 1 prior instance(s)

## Coverage

What was searched, and what was not:

- issue #508 + 2 comment(s) via github
- link crawl: 6 issue(s) within 2 hop(s) of `#N` references
- forge search on ['sparql-eval:', 'complete', 'bounded']: 14 other item(s)
- GhpRsq-Defect-Class trailers: 3 distinct class(es) in the last 200 commits
