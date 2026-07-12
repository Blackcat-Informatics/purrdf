# Changelog

All notable changes to the PurRDF crate suite are recorded here. The suite
ships one lockstep version across crates.io, PyPI, and npm; pre-1.0, a minor
bump may carry breaking changes and a patch bump is bugfix-only.

## [0.5.0] - 2026-07-11

### Breaking Changes

- **npm/wasm SELECT rows are now single-owner streams.** `SelectResult.rows` is
  a `QueryBindingRows` iterable rather than an array. This prevents the raw wasm
  layer from cloning every row and term before the package wrapper materializes
  them. Consume an indexed row with `result.rows.take(index)`, iterate remaining
  rows with `for...of`, or explicitly materialize them with
  `result.rows.toArray()`. Each row can be consumed once. `rows.length` and
  `result.rowCount` are the original total; `rows.remaining` is the unconsumed
  count. Call `result.free()` when abandoning unconsumed rows.
- **The raw wasm cloning getter was removed.** Replace `raw.rows` with
  `raw.rowCount`, `raw.takeRow(index)`, or `raw.nextRow()`. Move cells from a raw
  row with `row.takeValue(index)`; the non-consuming `row.get(variable)` remains
  available when cloning one individual term is intentional.

Before:

```js
const result = engine.select(dataset, query);
const first = result.rows[0];
const allRows = result.rows;
```

After:

```js
const result = engine.select(dataset, query);
const first = result.rows.take(0);
const remainingRows = result.rows.toArray();

// For bounded-memory processing, stream instead:
for (const row of engine.select(dataset, query).rows) {
  consume(row);
}
```

Python query-result classes and the C ABI function signatures are unchanged.

### Performance

- **Core IR:** the common provenance-free freeze path sorts quads directly;
  provenance remapping uses a dense vector; interners use fixed-key `ahash`; and
  borrowed IRI, blank, literal, and structural triple lookup avoids temporary
  `TermValue` trees. On the deterministic 3,200-quad fixture, allocated bytes
  fell from 1,312,246 to 1,211,366 (7.7%) and peak live bytes from 471,104 to
  415,296 (11.8%).
- **SPARQL and XSD:** parser tokens move out of the cursor, whitespace and fixed
  temporal fields avoid intermediate collections, DISTINCT/GROUP BY own keys
  and rows once, and graph-result serialization writes ID-native terms directly
  to the output buffer.
- **Reasoning:** SPARQL, entailment, and OWL concept interners store canonical
  values once. RIF joins use subject/predicate/object postings, choose the
  smallest available candidate set, and backtrack one reusable binding buffer.
- **SHACL and ShEx:** deterministic term sorting renders one key per value, and
  ShEx structural expressions and predicate-direction maps compile once per
  validation engine rather than once per focus node.
- **GTS:** canonical CBOR map keys are sorted by borrowed encoded keys and values
  stream directly to writers and BLAKE3. On the deterministic 2,000-quad
  snapshot, allocations fell from 32,040 to 14,027 (56.2%) and allocated bytes
  from 2,277,740 to 982,840 (56.8%).
- **Visualization and slices:** incoming statement references are counted in one
  pass. Slice ownership parses each RDF artifact once, walks ID-native terms once,
  and reuses the catalog's parsed manifest projection.
- **Bindings:** Python moves SELECT rows while sharing one immutable variable
  array; wasm moves rows, cells, strings, and nested triple terms; C pattern
  cursors no longer allocate a result-sized `Vec<QuadIds>`.

### Rust API

- Added `RdfDataset::quads_for_pattern_cursor` and `QuadPatternCursor`, an
  `Arc`-pinned owned iterator over the same selected quad index and residual
  filter used by borrowed pattern queries. It remains valid after other dataset
  handles are dropped and does not collect matching rows.
- Added borrowed dataset term writers and borrowed term lookup methods for
  allocation-sensitive crate consumers.

### Compatibility

- RDF, SPARQL, SHACL, ShEx, visualization, and GTS semantic ordering remains
  deterministic. Graph serialization is byte-equal to the prior owned path, and
  every frozen GTS item re-encodes byte-for-byte.
- The optimization evidence above uses allocation counters, exact bytes, and
  bounded candidate-work assertions. No wall-clock claim is made from the shared
  high-contention development host.

## [0.4.3] - 2026-07-10

### Bug Fixes

- **rdf:** Complete deterministic viz semantics
- **rdf:** Make RDF visualization graph-readable
- **rdf:** Refine visualization routing and labels
- **rdf:** Make dense visualization routes traceable
- **wasm:** Accept npm 12 pack output
- **rdf:** Address visualization review findings

### Documentation

- **rdf:** Qualify visualization projection link

### Features

- **rdf:** Add statement incidence viz projection
- **rdf:** Add renderer-neutral viz scenes
- **rdf:** Add deterministic layered viz layout
- **rdf:** Emit semantic RDF 1.2 SVG
- **wasm:** Expose RDF visualization exports
- **rdf:** Publish generated visualization samples

### Performance

- **rdf:** Reuse visualization projection scratch state

## [0.4.2] - 2026-07-10

### Bug Fixes

- Refresh lockfile for RIF parser

### Features

- Expose entailed SPARQL and RIF parsing

### Other

- Harden npm wasm RDF 1.2 toolkit

## [0.4.1] - 2026-07-09

### Bug Fixes

- **shapes:** Project external object-class values to a node-ref, not a string, in JSON Schema
- **npm:** Align package-root RDFJS typings
- **capi:** Refresh generated ABI header
- **npm:** Accept null dataset inputs
- **npm:** Correct ecosystem probe evidence

### Documentation

- **npm:** Add ecosystem probe evidence

### Features

- **npm:** Add reusable SPARQL query engine

### Performance

- **wasm:** Benchmark query engine reuse

### Testing

- **npm:** Gate packed wasm package
- **npm:** Pin package gate toolchain

## [0.4.0] - 2026-07-07

### Bug Fixes

- **hygiene:** Exclude rustdoc inline-code spans from the issue-ref lint
- **makefile:** Use POSIX sed for wasm-bindgen pin extraction (macOS grep -oP)
- **makefile:** Use awk not tr to parse wc byte counts in wasm-pkg-size
- **hygiene:** Restrict issue-ref inline-code exclusion to Rust doc comments
- **playground:** Clear CodeQL alerts — structural entailment assertion + worker same-origin guard
- **shapes:** Polarity-sound sh:not projection in json_schema emitter
- **shapes:** Negate sh:not inner as a whole conjunction (De Morgan sound)
- **shapes:** Route sh:not maxCount property inner to a loss (no vacuous not)
- **shapes:** Route array-unsafe value-restriction sh:not inners to a loss
- **shapes:** Route existential sh:hasValue sh:not inner to a loss
- **shapes:** Restrict sh:not negand to exact-complement projections
- **shapes:** Polarity-sound sh:not projection (kill vacuous class negation)

### CI & Build

- **capi:** Gate the purrdf.h C-ABI header against drift
- **wasm:** Gate optimized artifact size via a pinned wasm-toolchain composite action
- **release:** Share the pinned wasm-toolchain action and enforce the size budget on release
- **wasm:** Drop unpinned twiggy source-build diagnostics step
- **docs:** Deploy the RDF-1.2 console at /playground in the Pages artifact

### Documentation

- Uplift product docs to top-tier Rust project standard
- **agents:** Document the wasm size-budget gate and deliberate-raise procedure
- Link the RDF-1.2 playground from the root and package READMEs
- **wasm:** List the shacl module in the lib.rs surface doc comment
- **shapes:** Strip issue-ref tokens from emitted schema descriptions
- Reconcile README and docs for 0.4.0

### Features

- **capi:** Make purrdf.h reproducible via cargo-c `capi` marker + regenerate canonically
- **capi:** Make purrdf.h reproducible via cargo-c `capi` marker + gate it in CI
- **wasm:** Add reproducible wasm-pkg-size budget gate (binaryen pinned)
- **wasm:** CI-gated wasm artifact size budget
- **wasm:** Expose SHACL + RDFC-1.0 canonicalize/isomorphic on the package surface
- **playground:** Standalone client-side RDF-1.2 console (engine in a Web Worker)
- **playground:** Drop the post-load wasm-size probe so the console makes zero network requests after assets load
- **playground:** Assert the SARIF 2.1.0 contract in the SHACL pane instead of echoing the engine version
- **playground:** Standalone deployed RDF-1.2 console over purrdf-wasm

### Other

- **shapes:** Satisfy fmt, clippy docs, and issue-ref hygiene gates

### Performance

- **shapes:** Cache the sort key for sh:not negand ordering

### Testing

- **shapes:** Add trusted external JSON-Schema validator harness
- **shapes:** Behavioral accept/reject tests for polarity-sound sh:not

## [0.3.3] - 2026-07-05

### Documentation

- **capi:** Regenerate purrdf.h with SHACL validate + entail declarations

### Performance

- **rdf:** Memoize the line index so parser diagnostics stay linear
- **rdf:** Memoize parser line index — fix quadratic diagnostics scan

## [0.3.2] - 2026-07-05

### Bug Fixes

- **shapes:** Reconcile SHACL-AF work with the merged 0.3.1 baseline

### Features

- **shapes:** SHACL Rules — 100% SHACL-AF coverage

## [0.3.1] - 2026-07-05

### Bug Fixes

- **shapes:** Pre-bind $shapesGraph in sh:SPARQLRule CONSTRUCT execution
- **build:** Optimize parse-hot workspace crates in dev/test profile to remove ~300x regression

### CI & Build

- **release:** Edition 2024, publish purrdf-entail, expose entail+validate, bump 0.3.1
- **release:** Edition 2024, publish purrdf-entail, expose entail+validate, bump 0.3.1

### Documentation

- **conformance:** Add SHACL Rules scoreboard row; SHACL-AF is 100% complete
- **release:** Changelog for 0.3.1

### Features

- **shapes:** SHACL Rules engine — sh:TripleRule, sh:SPARQLRule, fixpoint entailment
- **shapes:** Cartesian-product multi-valued function-call node-expression args
- **bindings:** Expose SHACL rule entailment on Python, wasm, and C-API surfaces

### Performance

- **shapes:** Key the rules fixpoint divergence universe on Term, not String
- **shapes:** Reuse bindings buffer and hoist arg keys in function-call cartesian product

### Testing

- **shapes:** SHACL Rules conformance corpus + inferred-graph harness
- **shapes:** Audit every node-expression kind in sh:TripleRule subject/predicate/object positions
- **shapes:** Cover blank-focus blank minting and multi-round fixpoint convergence

## [0.3.0] - 2026-07-05

### Benchmarks

- **sparql-eval:** Isolate Solution row construction and join
- **rdf:** Add report-only span-tracking arm for the NoSpans zero-cost claim

### Bug Fixes

- **sparql-eval:** Correct correlated-EXISTS over address-keyed cache reuse
- **rdf:** Verify content-chain inclusion via blob/segment-head/MMR-leaf union
- **shex:** Route numeric_value through the XSD-1.0 float/double restriction
- **rdf:** Accept empty predicateObjectList item in Turtle parser
- **rdf:** Accept empty predicateObjectList item in Turtle parser
- **shex:** Fire group semantic actions only for participating groups
- **shex:** Make result-shape-map JSON fully round-trip through parse_shape_map
- **shex:** Distinguish unresolved IMPORT from conflicting redefinition
- **sparql-algebra:** Accept trailing top-level VALUES clause
- **sparql-conformance:** Dedupe named graphs by IRI + case-insensitive media-type
- **sparql-conformance:** Compare SELECT solutions up to W3C whole-set bnode isomorphism
- **sparql-eval:** XSD constructor casts emit canonical value forms
- **sparql-eval:** Correct built-in value spaces and XSD 1.1 canonical decimal
- **sparql-eval:** Scope UPDATE template blank nodes per request, not per operation
- **sparql-algebra:** Rewind trailing dot after blank-node label
- **sparql:** Exclude MINUS-right-only variables from in-scope set
- **shex:** Surface the concrete import parse error instead of swallowing it
- **shex:** Propagate concrete import cause through ImportResolver
- **conformance:** Make the byte-freeze manifest cross-platform deterministic
- **conformance:** Normalize CRLF in the matrix doc drift-check
- **conformance:** Honest matrix reporting for compile errors and first-party corpus
- **shacl-af:** Record sh:expression as a lossy JSON Schema projection
- **shacl-af:** SPARQL-value orderby, canonical set outputs, value-true is_true
- **shacl-af:** Keep sh:desc and described constant IRIs out of function-call parsing
- **conformance:** Repair --group dev gate, lock compat ratchet, refresh matrix
- **shacl-af:** Unbound mandatory sh:SPARQLFunction parameter yields no result
- **shacl-af:** Treat sh:returnType as informational, not an enforced datatype
- **shacl-af:** Reject empty or SHACL-reserved sh:SPARQLFunction parameter names
- **shacl-af:** Hard-fail malformed sh:order/sh:optional and multi-projection bodies
- **shacl-af:** Merge sh:SPARQLFunction body state back into the caller
- **shapes:** Repair broken merge — stray conflict markers and missed run_select rename
- **shapes:** Repair non-compiling SHACL-SPARQL component merge
- **shapes:** Make the validation-report sort total for byte determinism
- **entail:** Reject non-range-restricted RIF rules instead of panicking
- **entail:** Deterministic RDFS/OWL inferred-triple emission order
- **rdf:** Report located diagnostics at the offending token, not the next one
- **rdf:** Report Turtle/TriG located diagnostics at the offending token
- **rdf,validate:** Emit real byteOffset for N-Triples/N-Quads source spans
- **validate:** Wire SarifOptions::source_root_uri into the emitted SARIF
- **rdf:** Standardize blanks apart on native quad merge to stop cross-source collapse
- **release:** Stamp the pending version in the generated changelog
- **release:** Treat registry-restricted crates as publishable
- **release:** Anchor the package.json version bump to the top-level key
- **release:** Ignore commented-out crates in the publish-list parser
- **release:** Guard release-tags against a missing CHANGELOG section before tagging
- **release:** Assert per-crate version coherence in check-versions.py
- **hygiene:** Purge issue refs from python shim docstring and lint docstrings
- **rdf:** Pass the span collector in the empty-namespace turtle test
- **rdf-core:** Reject rdf:_0 and leading-zero container membership ordinals

### CI & Build

- **release-npm:** Guard binaryen --enable-simd + document SIMD baseline
- **wasm-pkg:** Hard-fail the build if the artifact carries no SIMD opcodes
- **wasm-pkg:** Append to RUSTFLAGS instead of overwriting it
- **release:** Enforce cross-registry version coherence and complete the publish list
- **release:** Pin the git-cliff version in the changelog target
- **release:** Make set-version.py rewrite every version location
- **release:** Gate internal dependency pins, at commit AND publish time

### Documentation

- **deps:** Correct memchr comment; drop issue refs from sparql-algebra
- **wasm-pkg:** Align the SIMD Node floor with the package engine (18)
- **wasm-pkg:** Describe the parse bench as report-only, not a regression gate
- **sparql-eval:** Correct fork_for_worker doc to the portable-row merge mechanism
- Describe behavior, not process, in content-addressing comments
- **rdf:** Document trust-on-first-use semantics of verify_content_chain
- **xsd:** Describe the i128/scale bound instead of a "deferred enhancement"
- Scrub GitHub issue-number references from shapes and sparql-eval comments
- **shex:** Clarify IMPORT conflict and inert-extension doctrine
- **conformance:** Reconcile rdflib LSP gate ledger scoreboard to live 62/24
- **iri:** Harden conformance-vector provenance for W3C IRI gate
- **conformance:** Finalize unified matrix at full-corpus SPARQL numbers
- **sparql-conformance:** Frame SPARQL 1.2 as a complete first-class spec
- **sparql-conformance:** Document that entailment simple1-8 are OWL-Direct, not simple-entailment
- **conformance:** Refresh SPARQL matrix counts to live harness (614 pass / 36 xfail)
- **conformance:** Correct SPARQL 1.2 provenance to zero ledgered residuals
- **sparql:** Finalize W3C SPARQL 1.1 syntax-suite provenance
- **conformance:** Reconcile the ledger and drift-guard the published matrix
- **conformance:** Distinguish normative SHACL-AF node expressions from owned extensions
- **conformance:** Correct stale SHACL first-party corpus count (64 to 69)
- **entail:** Fix misleading comment in the RIF emit path
- **validate:** Fix stale intra-doc link to a non-existent locate module
- **release:** Document MSRV and pre-1.0 semver policy
- **release:** Docs.rs metadata, front-page example, and workspace doc gate
- **ci:** Reconcile doc-target crate count to 16 (15 publishable + purrdf-entail)
- **rdf,shapes:** Fix private/broken intra-doc links failing the doc gate
- **conformance:** Regenerate matrix block for the added SPARQL fixtures
- **release:** Changelog for 0.3.0 and correct the published-crate count

### Features

- **rdf:** Memchr the parallel chunker newline split
- **rdf:** Scan-first serializer escape fast path
- **21:** Memchr/SWAR scan sweep — codec scans + IRI char-class LUT
- **sparql-eval:** Add rayon dep and deterministic two-phase parallel scaffold
- **rdf-core:** Add Blake3ContentId newtype with shared hex decode
- **rdf-core:** Add caller-supplied content-addressing config surface
- **rdf-core:** Recognize content-addressed IRIs at intern time
- **rdf-core:** Carry the content-id side table into the frozen dataset
- **rdf-core:** Add suppression-target and derivation-link traversal helpers
- **rdf-core:** Add a derived predecessor index over derivation annotations
- **rdf:** Add verify_content_chain GTS bridge over content-addressed terms
- **rdf-core:** Content-addressed term support in the IR (GTS-aligned)
- **xsd:** Add opt-in XSD-1.0 float/double lexical restriction
- **sparql-eval:** Pin xsd:float/double cast to XSD 1.0 lexicals
- **xsd:** Shared XSD-1.0 float/double lexical restriction
- **shex:** Resolve transitive cycle-tolerant IMPORT
- **shex:** Dispatch semantic actions via a Test extension registry
- **shex:** Query shape maps with FOCUS triple-pattern selectors
- **shex:** Serialize result shape maps to deterministic JSON
- **shex:** Populate SemActContext value and predicate per matched triple
- **shex:** Add validate_shape_map end-to-end entry point
- **sparql-conformance:** Harden conformance gate — no silent skips + license hygiene
- **sparql-conformance:** Support W3C UpdateEvaluationTest cases
- **sparql-eval:** LATERAL evaluation seam for variable-endpoint and nested SERVICE
- **entail:** Native wasm-clean RDFS + OWL-RL materialization reasoner
- **sparql-conformance:** Wire entailment regime into conformance harness
- **sparql:** Implement RDF-1.2 base-direction functions
- **sparql:** Parse RDF 1.2 triple terms, reifiers, and annotation blocks
- **sparql:** Complete RDF 1.2 triple-term/reifier support across parser, codec, and evaluator
- **sparql-eval:** Evaluate negated-inverse and set-repetition property paths
- **sparql:** Group-by projection check, EXISTS graph scope, GRAPH ?g over empty graphs
- **rdf:** Reifier-consistent CONSTRUCT/UPDATE emission and triple-term equality
- **rdf:** Give the RDF 1.2 reifier/annotation model a graph dimension
- **conformance:** Full W3C SPARQL 1.1/1.2 eval + native entailment + lateral SERVICE
- **shex:** Imports, Test-extension semantic actions, query shape maps
- **sparql:** Vendor W3C SPARQL 1.1 syntax-query suite
- **sparql:** Vendor W3C SPARQL 1.1 syntax-update-1/2 suites
- **sparql:** Vendor W3C SPARQL 1.1 syntax-fed conformance suite
- **sparql:** Vendor W3C SPARQL 1.1 syntax suite as parser conformance fixtures
- **conformance:** Enforce a monotone-shrink ledger ratchet in the gate
- **conformance:** SHA-256 byte-freeze the vendored conformance corpora
- **conformance:** Monotone ledger ratchet, drift-proof published matrix, and byte-freeze verification for the SHACL/shexTest gates
- **shacl-af:** Add node-expression IR skeleton and AF vocabulary
- **shacl-af:** Parse node expressions from the shapes graph
- **shacl-af:** Wire sh:ExpressionConstraintComponent end-to-end
- **shacl-af:** Evaluate built-in function-call node expressions
- **shacl-af:** Evaluate aggregation, paging, and ordering node expressions
- **shacl-af:** Evaluate filterShape and exists with cycle-safe re-entry
- **shacl-af:** Wire vectors/shacl/af seam and refresh conformance matrix
- **shacl-af:** Authority-grounded sh:orderby with sort-key expr and sh:desc
- **shacl-af:** Dispatch XPath-namespace keyword builtins in function calls
- **python:** Complete rdflib plugin entry-point discovery and acceptance matrix
- **shacl-af:** Sh:expression node constraints + node-expression evaluator
- **sparql-eval:** Dynamic SHACL-AF SPARQL function registry seam
- **shapes:** Parse sh:SPARQLFunction declarations into a function registry
- **shapes:** Resolve sh:SPARQLFunction calls in validation, remove the stub
- **shapes:** Implement SHACL-SPARQL custom constraint components
- **shacl-sparql:** Pre-binding substitution semantics and shapes-graph variables
- **shacl-af:** Sh:SPARQLFunction user-defined SPARQL functions
- **shapes:** Complete SHACL-AF validation coverage
- **core:** Expose shared FastHasher/FastMap/FastSet + smallvec primitives
- **shapes:** Id-native SHACL engine over interned TermIds
- **sparql-algebra:** Zero-copy lexer tokens borrowing the source
- **entail:** Bare-RDF axiomatic predicate-typing entailment (rdf01)
- **entail:** ALCOIQ OWL-Direct tableau reasoner core (concept, parser, tableau)
- **entail:** Query-directed OWL-Direct DL materialization clears 25 conformance cases
- **entail:** RIF-Core rule engine clears rif01/03/04/06 (zero entailment xfails)
- **entail:** Native OWL-DL tableau + RIF-Core engine + bare-RDF axiomatic entailment
- Close public-maturity epic
- **iri:** Add shared source-position primitive (LineIndex/Position)
- **diagnostics:** Resolve lexer byte offsets to line/column
- **codec:** Attach line/column locations to RDF text parse errors
- **codec:** Opt-in triple->source span table
- **validate:** Scaffold purrdf-validate SARIF boundary crate
- **validate:** Hand-rolled deterministic SARIF 2.1.0 model
- **validate:** Map reports and diagnostics to SARIF results
- **validate:** Source-traced SARIF physical and logical locations
- **validate:** To_sarif surface + Python binding + schema validation
- **bindings:** SARIF surfaces for WASM and C-ABI
- **validate:** SARIF rule metadata with W3C SHACL help links
- **validate:** SARIF 2.1.0 source-traced reporting
- **perf:** Id-native SHACL engine, zero-copy lexer tokens, workspace small-vec + hasher sweep
- **rdf-core:** Graph-scoped rdf:first/rest/nil + container traversal on DatasetView
- **slice:** Graph-scoped nav cursor, RDF-1.2 triple-term interiors, list/container materializer
- **release:** Generate the changelog and GitHub Release notes with git-cliff
- **release:** Single-command version bump and coherent tag cut
- **hygiene:** Extend issue-ref lint to workflow yaml and python comments
- **python:** Complete rdflib drop-in epic
- **release:** Docs.rs polish, MSRV/semver docs, version-coherence gate + changelog
- **rdf-query:** Graph-scoped nav cursor, list/container materializer, one-path blank-safe merge, FILTER/UNION regressions

### Other

- **iri,rdf:** Rustfmt the LUT/escape hot paths
- Remove old docs
- **rdf:** Fmt import order in ser_model tests
- Ignore more
- **sparql-eval:** Apply cargo fmt to expr.rs
- **shapes:** Apply cargo fmt to the pinned-lexical test
- Integrate main (parallel eval, content-addressed terms, XSD-1.0 float/double) into the W3C conformance branch
- Strip stale gmeow-ontology issue refs from Rust comments + lint against regression
- **conformance:** Integrate origin/main; fix issue-ref lint to scan only tracked source
- **shacl-af:** Rustfmt wrapping and register new corpus fixtures
- Rustfmt normalization across the SARIF work

### Performance

- **iri:** Const char-class LUT for ASCII validation
- **sparql:** Byte-cursor + memchr tokenizer
- **rdf:** Hex-LUT UCHAR escape, drop write! from the hot path
- **rdf:** Borrow clean input in escape_scan via Cow
- **sparql-eval:** Memoize constant expression atoms per query
- **sparql-eval:** Memoize dataset literal XSD parses per query
- **sparql-eval:** Allocation-free single-column hash-join keys + pre-sized build map
- **sparql-eval:** O(1) visited-set for property-path transitive closure
- **sparql-eval:** Hoist loop-invariant BGP probe permutation selection
- **sparql-eval:** Pre-parse quoted-triple ORDER BY sort keys
- **sparql-eval:** Single-threaded optimization backlog (items 3–7 + ORDER BY triple residual)
- **wasm-pkg:** Add Node parse-throughput benchmark
- **wasm-pkg:** Build the npm artifact with +simd128
- **wasm-pkg:** Build the npm artifact with +simd128
- **sparql-eval:** Parallelize BGP inner loop and read-only join probes
- **sparql-eval:** Parallelize FILTER/filtered-left-join with forked per-worker contexts
- **sparql-eval:** Parallelize UNION, BIND, and per-group aggregates with deterministic scratch merge
- **sparql-eval:** Chunk-based parallel collects to cut per-row allocation
- **sparql-eval:** Reintern minted rows by value to drop the per-cell TermValue clone
- **sparql-eval:** Deterministic parallel evaluation (UNION, joins, BGP, FILTER, aggregates)
- **rdf-core:** Add report-only bench for intern-time content-id overhead
- **rdf-core:** Hash predecessor_chain visited set with ahash
- **entail:** Genuine semi-naive delta chase with new-vertex reflexive derivation
- **sparql:** Reuse blank-label set across update-operation iterations
- **shacl-af:** Hoist recursion guard, reuse intersection set, cache sort keys
- **sparql-eval,shapes:** Adopt small-vectors for hot per-row/per-node collections
- **entail:** Reuse frontier buffers in the RDFS chase loop
- **entail:** Reuse frontier buffers in the RIF chase loop
- **shapes:** Pre-resolve rdf:type id once per Class constraint
- **shapes:** Carry id-native value nodes through the constraint layer
- **shapes:** Adopt fixed-key ahash for the remaining membership sets
- **shapes:** Cache report sort key and borrow the sparql dataset
- **rdf:** Validate UTF-8 lazily in the text-format span-tracking arm
- **rdf-core:** Resolve container type once in is_typed_container

### Refactor

- **sparql-eval:** Rc→Arc on SolutionSeq/ExistsInner for Send+Sync
- **sparql-eval:** Make EvalCtx Send+Sync (Arc caches, RwLock order cache, Sync remote)
- **rdf:** Reuse purrdf_gts::wire::hex in the verify bridge
- **shex:** Route datatype checks through parse_xsd10
- **shapes:** Fold double/float lexical check into purrdf-xsd
- **entail:** Split reasoner into vocab/interner/rdfs modules + Regime::Rif scaffolding
- **shapes:** Collapse the non-interned path walker to reflexive inclusion
- **validate:** Hoist shared validate-to-SARIF helper into purrdf-validate
- **rdf:** Centralize native-codec format dispatch behind an RdfCodec trait
- **rdf:** Centralize native-codec format dispatch behind an RdfCodec trait

### Testing

- **rdf:** Bench the serializer escape boundary path
- **iri:** First parse criterion bench (dev-dep only)
- **sparql-eval:** Forced-parallel byte-identity determinism gate over a query corpus
- **rdf:** Prove content addressing does not perturb serialized bytes
- **shapes:** Cover xsd:float +INF at the SHACL layer; clarify pinned accept-set test
- **shex:** Drop Turtle-parser workaround in validation conformance harness
- **rdf:** Lock leading-empty rejection in nested callers and drain-to-pipe run
- **shex:** Empty the validation trait-skip list and assert zero skips
- **shex:** Lock Import and SemanticAction trait coverage with exact counts
- **sparql-conformance:** Vendor full W3C SPARQL 1.1 QUERY suite with typed non-pass ledger
- **sparql-conformance:** Vendor W3C SPARQL 1.1 UPDATE evaluation suite
- **sparql-conformance:** Vendor W3C SPARQL 1.2 DRAFT suite + classify
- **rdflib-gate:** Ledger test_group_by — purrdf stricter than rdflib on GROUP BY projection
- **sparql:** Cover blank-node reuse inside RDF-1.2 quoted triples
- **shacl-af:** End-to-end goldens for sh:min/max/distinct/offset
- **shapes:** First-party sh:SPARQLFunction conformance corpus cases
- **shacl-af:** Negative-path coverage for sh:SPARQLFunction
- **bench:** Add SHACL pattern-lookup and value-token lexer micro-benches
- **entail:** Lock in deterministic inferred-triple emission order
- **validate:** Cover attribution UnitId -> slice IRI resolution (S0.5)
- **rdf:** Lock parallel line numbering for a newline-less final line
- **validate:** Lock SHACL helpUri anchors against the live spec format
- **validate:** Avoid a literal #N token in the S0.5 test
- **sparql:** Regression-cover FILTER-NOT-EXISTS arithmetic and all-FILTER UNION branch

## [0.2.1] - 2026-07-02

### Benchmarks

- **python:** Published rdflib-vs-shim benchmark harness + docs

### Bug Fixes

- **python/compat:** Reject bare-form xsd duration instead of zeroing it
- **python/compat:** Honor base/initBindings/native kwargs in SPARQL processors
- **python/compat:** Unwrap Resource-typed predicate and index arguments
- **python:** Sample the host wall clock for NOW and RAND/UUID
- **capi:** Sample the host wall clock for NOW and RAND/UUID
- **sparql:** NOW is the wall clock and RAND is real entropy — by default, everywhere
- **sparql:** Thread standpoint + order cache into the UPDATE WHERE context
- **sparql:** NOW/RAND sample the host wall clock, not the epoch

### CI & Build

- **python:** Run the acceptance matrix under the acceptance dep group

### Documentation

- **python/compat:** Strip in-repo tracker references from the compat shim

### Features

- **python:** Python test harness + xfail-ledger gate for the rdflib drop-in
- **python:** Top-level engine exports mirroring the Rust umbrella crate
- **python:** Term-model completeness — value coercion, RDF 1.2 direction, from_n3
- **python:** NamespaceManager + Namespace parity with rdflib
- **python:** Graph/Dataset facade parity with rdflib
- **python:** Rdflib plugin registry + entry-point discovery
- **python:** SPARQL result serialization, native substitutions, property paths
- **python:** Opt-in top-level `rdflib` shadow (import rdflib -> purrdf)
- **build:** Single conformance matrix — native W3C suites + rdflib gate
- **xsd:** Native binary decode + whitespace facets for the compat value map
- **rdf:** Native TriX and HexTuples codecs for the compat plugin registry
- **sparql-eval:** Add wasm-clean QueryEnv seam for NOW/RNG injection
- **11:** Purrdf as an RDF-1.2-first drop-in rdflib replacement

### Other

- **python:** Rustfmt the native SPARQL-results + term-direction bindings
- Release 0.2.1: rdflib drop-in hardening — native xsd coercion + TriX/HexTuples codecs

### Testing

- **python:** Gate — rdflib's own test suite against the compat shim
- **python:** Downstream acceptance matrix (pyshacl / SPARQLWrapper / sssom)

## [0.2.0] - 2026-07-02

### Other

- Release 0.2.0: complete umbrella facade, OntologyProfile, ShExC serializer, drop openEHR OPT

## [0.1.5] - 2026-07-02

### Bug Fixes

- Fix release lanes: wasm-opt post-MVP features, workspace-inherited version

### Documentation

- Add the PurRDF DOI (10.67342/pkg8gpp4no/v1) to CITATION.cff and the README badge row

### Other

- Package README + metadata, js package 0.1.4
- Shex in flight
- Full ShEx 2.1 + complete SHACL Core + de-gmeow the library namespaces
- Purge the invented namespace: purrdf is a toolkit, not an ontology
- Parallel parse + parallel GTS verification (deterministic by construction)
- Python bindings: shex module, engine configuration, GIL release
- Release 0.1.5: full SHACL/ShEx, de-gmeow'd namespaces, SPARQL eval speedups

## [0.1.3] - 2026-07-02

### Bug Fixes

- Include Python sdist toolchain
- Build PyPI manylinux wheels

### Other

- Parameterize jsonld prefix
- Release 0.1.3: brand, first-class docs, strict lints, perf, npm lane
- Stabilize the toolchain: stable-Rust-clean workspace, real MSRV

## [0.1.1] - 2026-07-01

### Bug Fixes

- Set crates.io release user agent
- Pace crates.io bootstrap publishes
- Set crates.io workflow user agent
- Make purrdf the umbrella crate
- Pace only new crate publishes

### Other

- First commit
