# Prior art — issue #1107: v4 API: retain native GTS composition and projection failures

Assembled by `stagectl brief`. Every hash and path below came from this
repository; nothing here is recalled or inferred. Read the Coverage
section at the bottom before concluding anything is absent — an empty
section can mean 'nothing found' or 'could not look'.

## Governing ADRs

The issue cites no ADRs.

## Linked issues (2-hop `#N` crawl)

- #1104: Epic: v4.0 API freeze — complete breaking changes before release  [OPEN]  (mentions)
- #666: Bump to v4.0  [OPEN]  (mentions)
- #558: Structured access to error variants and fields, so consumers never render errors to text  [OPEN]  (mentions)
- #483: sparql-eval: never materialize a cross product (factorized components) and give every plan a guaranteed output bound  [OPEN]  (2-hop)
- #486: One measured memory budget for every allocation, with spill to a host-provided seam instead of refusal  [OPEN]  (2-hop)
- #498: Datalog chase: scalable JA/RJA termination certificates and source-bound specialization  [OPEN]  (2-hop)
- #671: feat(glob): purrdf-glob for optimized matching, expansion and RDF 1.2 pattern models  [OPEN]  (2-hop)
- #698: Evaluate recursive SHACL with explicit native LFP and GFP fixed points  [OPEN]  (2-hop)
- #701: Support Jelly-RDF  [OPEN]  (2-hop)
- #1004: Expose structured YAML refusal context and lexical scalar emission policy  [OPEN]  (2-hop)
- #569: Design around recursive SHACL validation  [OPEN]  (2-hop)
- #1105: v4 API: retain typed reasoning and host request failures  [OPEN]  (2-hop)
- #1106: v4 API: finalize typed SHACL parsing, validation and configuration failures  [OPEN]  (2-hop)
- #1107: v4 API: retain native GTS composition and projection failures  [OPEN]  (2-hop)
- #1108: v4 API: stabilize typed USTAR reader, iterator and writer errors  [CLOSED]  (2-hop)
- #1109: v4 API: finalize typed visualization layout and presentation refusals  [CLOSED]  (2-hop)
- #1110: v4 API: finalize structured YAML reader refusal context and ownership  [CLOSED]  (2-hop)
- #1111: v4 API: make planner estimates extensible before adding guaranteed bounds  [CLOSED]  (2-hop)
- #1112: v4 API: make governor dimensions and request construction extensible  [CLOSED]  (2-hop)
- #1113: v4 API: generalize prepared chase admission certificates beyond weak acyclicity  [CLOSED]  (2-hop)
- #1114: v4 API: allow compatible format and glob refusal extensions  [CLOSED]  (2-hop)
- #1116: v4 API: retain typed literal-component refusals across native and host callers  [OPEN]  (2-hop)
- #1117: v4 API: expose typed loss-ledger completeness and soundness failures  [CLOSED]  (2-hop)
- #1118: v4 API: retain native query-artifact failures in slice ownership diagnostics  [CLOSED]  (2-hop)
- #1115: v4 API: close the reachable public-interface inventory and compatibility gate  [OPEN]  (2-hop)
- #367: perf(hash): native SHA-2 with lower instruction work, multi-buffer RDFC hashing and wide-schedule SHA-512  [OPEN]  (2-hop)
- #443: Optimize the exact numeric tower: small-value parity and subquadratic large-value arithmetic  [OPEN]  (2-hop)
- #485: sparql-eval: merge joins over sorted inputs as the default, and push build-side id ranges into probe-side scans  [OPEN]  (2-hop)
- #487: sparql-eval: vectorized execution over batches of term ids with selection vectors  [OPEN]  (2-hop)
- #488: Datalog, chase and entailment: certified partial models and resumable evaluation instead of total refusal  [OPEN]  (2-hop)
- #489: Make the RDFC call limit and SPARQL UDF depth removable, and stop canonicalize from panicking on exhaustion  [OPEN]  (2-hop)
- #496: Scale evidence at 10^8–10^9 triples, including a reifier-heavy RDF 1.2 claims workload  [OPEN]  (2-hop)
- #501: Entailment: reusable snapshot-bound proof and justification query sessions  [OPEN]  (2-hop)
- #603: OWL: shared dataset views and source-bound positive world updates  [OPEN]  (2-hop)
- #564: SegmentedBuilder: build an image in memory bounded by its blocks, not its rows  [OPEN]  (2-hop)
- #686: Complete and verify the SPARQL-DL query surface over OWL Direct semantics  [OPEN]  (2-hop)
- #863: Make YAML-LD 1.0 a first-class, source-preserving translation target  [OPEN]  (2-hop)
- #986: sparql: interval relations over values with declared coarse precision, undecidable as undetermined  [OPEN]  (2-hop)
- #1024: Integrate consumer helper policy and capacity proofs with a RAF native ledger trial  [OPEN]  (2-hop)
- #1035: Verify and refuse content BOMs in unquoted native YAML scalars  [OPEN]  (2-hop)
- #634: Retain original geographic refusal operands and native causes  [MERGED]  (2-hop)
- #635: Retain native causes through RDF materialization and pack attachments  [MERGED]  (2-hop)
- #637: Retain structured entailment failures through public reasoning inspection  [MERGED]  (2-hop)
- #649: Make Datalog governors and predicate access explicit; retain native failures  [MERGED]  (2-hop)
- #657: Expose regime term arena limits across hosts  [MERGED]  (2-hop)

## Prior attempts at this issue

Commits citing it (including inside squash merges):

- None found.

Other branches named for it:

- None.

PRs citing it:

- None found.

## Related items on the forge

- #671: feat(glob): purrdf-glob for optimized matching, expansion and RDF 1.2 pattern models  [OPEN]
- #1104: Epic: v4.0 API freeze — complete breaking changes before release  [OPEN]
- #686: Complete and verify the SPARQL-DL query surface over OWL Direct semantics  [OPEN]
- #863: Make YAML-LD 1.0 a first-class, source-preserving translation target  [OPEN]
- #569: Design around recursive SHACL validation  [OPEN]
- #558: Structured access to error variants and fields, so consumers never render errors to text  [OPEN]
- #1125: Preserve native quoted triple objects in snapshot composition  [CLOSED]
- #1014: Expose admitted source-preserving native YAML streams  [CLOSED]
- #967: Admit finite shape outputs in recursive sh:nodeByExpression  [CLOSED]
- #891: Evaluate pure native focus expressions in recursive SHACL conformance  [CLOSED]
- #727: Compile ordered glob pattern sets with aggregate work and allocation-free matching  [CLOSED]
- #1017: Add a native YAML source carrier with strict RDF cover reconstruction  [CLOSED]
- #1013: Implement exact admitted YAML Core numeric resolution  [CLOSED]
- #690: Restore SHACL caller and product parity after identity catalog integration  [CLOSED]

## Recurrence

Counted from `GhpRsq-Defect-Class` trailers — these are tallies, not estimates:

- `none`: 41 prior instance(s)

## Coverage

What was searched, and what was not:

- issue #1107 + 0 comment(s) via github
- link crawl: 45 issue(s) within 2 hop(s) of `#N` references
- forge search on ['retain', 'native', 'composition']: 14 other item(s)
- GhpRsq-Defect-Class trailers: 1 distinct class(es) in the last 200 commits
- prior attempts: 0 commit(s) citing #1107 (second parents walked), 0 other branch(es) named for it
- PRs citing #1107: 0
- requirement candidates: 6 line(s); 0 comment(s) matched no rule
- referenced artifacts: 0 path(s) (0 on disk), 3 identifier(s), 0 workspace package(s), 4 URL(s)
