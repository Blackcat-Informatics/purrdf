# Changelog

All notable changes to the PurRDF crate suite are recorded here. The suite
ships one lockstep version across crates.io, PyPI, and npm; pre-1.0, a minor
bump may carry breaking changes and a patch bump is bugfix-only.

## [0.3.1] - 2026-07-05

### CI & Build

- **release:** Edition 2024, publish purrdf-entail, expose entail+validate, bump 0.3.1
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

This change removes every stale `#NNN` GitHub issue-reference token from
Rust comments and in-tree markdown documentation, then adds a CI/pre-commit
lint that rejects new ones so the convention holds.

What changed:
- `scripts/check-issue-refs.py` (new): Rust-aware linter that scans `//`,
 `///`, and `//!` comments plus `.md` files under `crates/`, `bindings/`,
 and `docs/`. String literals, character literals, raw strings, fenced code
 blocks, inline code spans, hex colors, markdown anchors, and section numbers
 like `.1` are excluded. The regex matches `#` followed by 1–5 decimal
 digits.
- `.github/workflows/ci.yaml`: runs the new lint in CI.
- `Makefile`: wires the lint into `make check` and provides a standalone
 `check-issue-refs` target.
- Rust comments (138 files): stripped stale `#NNN` tokens while preserving
 explanatory prose, then repaired the broken parentheses, dangling slashes,
 and sentence fragments left by the initial mechanical pass.
- Markdown docs (READMEs, test fixture SOURCE.md, etc.): stripped stale
 issue-reference tokens.

Requirements satisfied (issue ):
1. `#NNN` tokens removed from `//`/`///` comments across the workspace.
2. `#NNN` refs swept from in-tree READMEs and design docs.
3. CI/pre-commit lint added to block new `#NNN` tokens in comments and docs.

Standing constraints respected (`.goals`):
- No optional features or `cfg`-gated behavior was introduced.
- Core Rust semantics are unchanged; the Python surface is untouched except
 for comment-only cleanups in bindings source files.
- Wasm-able crate set remains wasm-buildable.
- No new namespaces or ontology terms were minted.

Verification:
- `make check` passes (fmt, clippy, check, tests, license/generated/issue-ref
 hygiene).
- `make rdf-core-hygiene` passes.
- `make wasm` passes.
- `python3 scripts/check-issue-refs.py` reports no violations.
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

Bumps the workspace, python, capi, and npm package versions to 0.2.1. Ships
the stage-2 hardening of the rdflib drop-in: mandatory acceptance-matrix gate,
the three reviewer fixes (bare-form xsd:duration, SPARQL processor kwarg
forwarding, Resource predicate/index unwrapping), in-repo tracker-reference
cleanup, native purrdf-xsd binary/whitespace value coercion, and first-party
native TriX and HexTuples codecs replacing the NotImplementedError plugin stubs.

### Testing

- **python:** Gate — rdflib's own test suite against the compat shim
- **python:** Downstream acceptance matrix (pyshacl / SPARQLWrapper / sssom)
## [0.2.0] - 2026-07-02

### Other

- Release 0.2.0: complete umbrella facade, OntologyProfile, ShExC serializer, drop openEHR OPT

- purrdf umbrella now re-exports the full published surface (gts, sparql, xsd, iri, events) so consumers depend on purrdf alone; merges the purrdf_gts engine with the purrdf_rdf GTS adapter under one gts module.
- Surface SliceVocab / Namespaces / StatementMetadataVocab at the root and unify them behind purrdf::OntologyProfile (build once, project into each emitter's native config).
- purrdf-shex: add to_shexc (compact-syntax serializer); round-trips all 425 shexTest schemas.
- purrdf-shapes: remove openehr_opt (healthcare-domain vocabulary) + its test/fixture and the now-unused roxmltree dep.
- Bump workspace + bindings + capi + js package to 0.2.0.
## [0.1.5] - 2026-07-02

### Bug Fixes

- Fix release lanes: wasm-opt post-MVP features, workspace-inherited version

- release-npm: Ubuntu's binaryen rejects the bulk-memory/nontrapping-fptoint
 ops rustc emits by default for wasm32; pass the --enable flags to wasm-opt
- release-pypi: bindings/python inherits version.workspace = true, so the
 tag check must resolve the crate version via cargo metadata, not read the
 member Cargo.toml directly

### Documentation

- Add the PurRDF DOI (10.67342/pkg8gpp4no/v1) to CITATION.cff and the README badge row

### Other

- Package README + metadata, js package 0.1.4
- Shex in flight
- Full ShEx 2.1 + complete SHACL Core + de-gmeow the library namespaces

ShEx (new crate purrdf-shex, gated on vendored shexTest v2.1.0):
- ShExC lexer/parser + ShExJ serde + structural checks (dangling refs,
 ref cycles, negation stratification via hand-rolled Tarjan SCC):
 99/99 negativeSyntax, 14/14 negativeStructure, 425/425 schemas parse,
 420/420 ShExJ round-trips, 419/420 ShExC==ShExJ AST ground truth
 (1 documented upstream corpus bug)
- validator: node constraints (purrdf-xsd lexical/numeric machinery),
 value sets with stems/ranges/exclusions, EachOf/OneOf/TC matching with
 EXTRA/CLOSED (interval fast path covers 98.9%; budget-bounded
 backtracker for the rest), coinductive typing recursion, fixed shape
 maps: 1051/1051 attempted validation tests pass, zero xfails
 (54 skipped: Import/SemanticAction traits, phase 2)

SHACL (crates/shapes):
- full property paths (sequence/alternative/zeroOrMore/oneOrMore/
 zeroOrOne, cycle-safe closures, algebraic inverse rewriting),
 property-pair constraints (equals/disjoint/lessThan/lessThanOrEquals),
 qualified value shapes with sibling-disjoint semantics, property-shape
 deactivation; frozen corpus 42 -> 48 cases
- W3C data-shapes suite vendored (vectors/shacl) + manifest-driven
 harness: 113 passed / 7 xfailed (standalone property shapes,
 sh:value defaults, pre-binding rejection, dateTime value-space,
 custom severities, structured sh:resultPath all fixed)

Namespace integrity (the extraction had blind-renamed the published
gmeow: namespace to an unpublished purrdf: one inside library code):
- vocab/purrdf.ttl: a real, tiny purrdf carrier vocabulary (JSON-LD-star
 qSubject/qPredicate/qObject/qObjectLiteral encoding keys, the 7 SPARQL
 extension functions, neutral StatementMetadata fallback)
- json_schema: compile(&shapes, &Namespaces) - caller-supplied prefix
 map + primary namespace (unblocks gmeow regenerate)
- SPARQL: extension-fn namespaces are ParserOptions (gmeow aliases its
 ns); heldIn standpoint predicates are a caller-supplied table with a
 hard error when unconfigured
- slice: SliceVocab::for_namespace threads through catalog/ownership/
 fix_deps/analysis/emitters + Python bindings (discover takes the
 namespace); gts_view LanguageVocab; fno ProjectionFunction from the
 catalog namespace; test fixtures moved to example.org

Also vendored shexTest v2.1.0 + W3C data-shapes with provenance READMEs;
purrdf-shex added to the wasm gates; 11 library-scoped issues migrated
from gmeow-ontology (purrdf-).

1523 tests passing; clippy -D warnings clean; fmt/no-features/generated/
kernel-hygiene/wasm32 gates green.
- Purge the invented namespace: purrdf is a toolkit, not an ontology

Policy (final): purrdf mints no vocabulary IRIs. Every vocabulary the
library reads or writes is caller-supplied configuration with no
fabricated default — a feature exercised unconfigured hard-errors or
stays inactive. The GMEOW ontology is a consumer; the dependency arrow
never points from purrdf to it. (vocab/purrdf.ttl, briefly authored, is
gone.)

- sparql-algebra: ParserOptions::default = no extension namespaces;
 Function::Purrdf(PurrdfCall) keeps the ORIGINAL call IRI so output
 round-trips the caller's namespace verbatim; PURRDF_NS deleted
- sparql-eval/conformance: engine defaults namespace-free; extension +
 standpoint suites rewritten to example.org/ext/ with harness config
- jsonld codec: StatementMetadataVocab has no Default; star downcast
 without a configured vocab hard-fails; @vocab no longer emitted;
 the W3C rdf:reifies/@annotation lane mints nothing and stays free
- shapes: BoxRoleVocab::for_namespace caller config (unconfigured =
 roles inactive; conformance unaffected); openehr emitter takes
 caller prefixes; corpus 38-42 fixtures moved to example.org/meta/
- slice: ownership dependency edges use the parsed original extension
 IRI; template prose renaming (PURRDF -> caller prefix, `purrdf
 regenerate` -> `{prefix} regenerate`) so emitted artifacts carry no
 template label; parity tests point at the gmeow namespace
- reverted the extraction's blind namespace rename in committed
 fixtures: generated/queries (54), queries/ (~90), the accessibility
 SSSOM mapping (renamed gmeow-accessibility.sssom.tsv), gts
 agent_memory example -> example.org/memory/
- docs/CONFORMANCE.md: full scoreboard + ledger discipline; README
 conformance table (SHACL 114/120 after the lessThan per-pair fix);
 AGENTS.md/CLAUDE.md carry the not-an-ontology doctrine; purrdf-shex
 wired into the umbrella crate (purrdf::shex), release lanes, and
 wasm gates; issues - filed for the remaining ledger/roadmap

grep 'blackcatinformatics.ca/purrdf' across crates/bindings/generated/
queries = the project homepage URL only. Full gate green: 1,600+ tests,
clippy -D warnings, fmt, no-features, generated, hygiene, wasm32.
- Parallel parse + parallel GTS verification (deterministic by construction)

- N-Triples/N-Quads: two-phase chunk-parallel parse — line-aligned chunks
 tokenized in parallel through the untouched per-line pipeline, then
 collected in document order into the store-once interner, so term ids,
 quad order, diagnostics (first error in document order), and canonical
 bytes are identical to the sequential path. 1 MiB threshold; Turtle/TriG
 stay sequential (documented: prefix/bnode state). ~1.75x on the 50k-row
 bench (55 -> 96 MiB/s); proof tests compare every pipeline stage and
 sweep chunk geometries 1..4096.
- GTS: frame content-ids recomputed concurrently per spec §9.1 with a
 sequential prev-equality pass; cross-segment folds parallel per §3.1;
 payloads >= 128 KiB hash via blake3 update_rayon (workspace blake3 gains
 the rayon feature — inline-sequential on wasm32, gates unaffected).
 ~1.83x on the new 32 MiB gts_verify bench (1.50 -> 2.75 GiB/s); the
 35-vector corpus folds byte-identically at RAYON_NUM_THREADS=1/3/32.
- README: ShEx quickstart line for the Python bindings.
- Python bindings: shex module, engine configuration, GIL release

- purrdf_native.shex: validate (fixed shape maps over ShExC/ShExJ +
 Turtle/N-Triples/N-Quads; focus nodes as IRIs, _:blanks, or Turtle
 literal tokens decoded through the native codec) and parse (canonical
 ShExJ), mirroring the shacl submodule; typed stubs in the .pyi
- Store/MutableDataset query+update gain keyword-only extension_namespaces
 and standpoint_predicates (the caller-supplied SPARQL seam config —
 purrdf mints no vocabulary); from_json_ld gains statement_vocab
- every heavy entry point (47: parse/serialize/canonicalize, SPARQL
 query/update, GTS emit/fold/relational, SHACL/ShEx validate, slice
 discovery/analysis, SSSOM) releases the GIL via Python::detach — a
 second Python thread ran ~21.6M loop iterations during a 0.41s parse
 in the two-thread smoke (previously blocked)
- Release 0.1.5: full SHACL/ShEx, de-gmeow'd namespaces, SPARQL eval speedups

Feature work (landed across this cycle, gated at 0.1.5):
- ShEx 2.1: new purrdf-shex crate (ShExC + ShExJ schema layer, structural
 checks, fixed-shape-map validator) — 1,051/1,051 attempted shexTest
 validation tests, 99/99 negative-syntax, 14/14 negative-structure, empty
 xfail ledgers; exposed via the Python purrdf_native.shex submodule.
- SHACL Core completed: full property paths (sequence/alternative/
 zero-or-more/one-or-more/zero-or-one), property-pair constraints,
 qualified value shapes; W3C data-shapes harness at 114/120 with a
 6-entry reasoned xfail ledger (custom SPARQL components + pre-binding
 corners tracked in /).
- Namespaces are caller-supplied everywhere (Namespaces/SliceVocab/
 LanguageVocab/StandpointPredicates/ParserOptions); purrdf mints no
 vocabulary IRIs. Fixes downstream json_schema::compile for any target
 namespace.
- SPARQL evaluation speedups (measurement-driven, results byte-identical,
 W3C conformance green): shared Rc<Regex> two-level cache (no per-row DFA
 pool churn), precomputed ORDER BY sort keys, pre-interned boolean
 constants, borrowed xsd_of_term on the compare hot path, u64-packed
 hash-join keys.

Version: 0.1.5 across all crates, PyPI, and the npm js package; CITATION
and RELEASE docs updated. purrdf-shex is a new crates.io record — its
first publish needs the token bootstrap before trusted publishing.
## [0.1.3] - 2026-07-02

### Bug Fixes

- Include Python sdist toolchain
- Build PyPI manylinux wheels

### Other

- Parameterize jsonld prefix
- Release 0.1.3: brand, first-class docs, strict lints, perf, npm lane

Brand & repo:
- PurRDF casing standardized; docs/BRAND.md defines the family rule
- logo rebuilt with the purrdf service object (RDF triple) replacing the
 copied GTS chain; social preview corrected and re-rendered
- first-class README (badges, quickstarts kept honest by a twin test,
 crate map, measured-perf story); CITATION.cff, SECURITY.md,
 CONTRIBUTING.md, CODE_OF_CONDUCT.md, LICENSING.md, AGENTS.md, CLAUDE.md

Rust workspace:
- [workspace.dependencies] single-sources every version; sha1 0.10/0.11
 skew fixed; hashbrown unified on 0.15; serde_yaml migrated to the
 maintained serde_yaml_ng via package rename
- [workspace.lints]: clippy pedantic+nursery with a reasoned allow list;
 4341 warnings fixed to zero; MSRV 1.96 declared; clippy.toml msrv pin
- API shape: intern_iri/intern_blank take &str (~480 call sites,
 caller-side clones removed); ref_option/needless_pass_by_value fixed
 workspace-wide with only 6 scoped binding-ABI allows
- codec hot path: tokens moved (not cloned) out of the buffer; codec
 interner rebuilt on the store-once ahash pattern: +45-50% N-Quads
 parse throughput; format_push_string swept from all serializers
- profiles: bench keeps symbols for profiling; dev deps at opt-level 2

CI & release:
- CI now runs the test suite (previously compile-only), clippy
 -D warnings, and an MSRV job
- Makefile rebuilt as the real gate (fmt+clippy+tests+hygiene+wasm) and
 recovers the lost wasm-pkg/capi-*/rdf-core-hygiene targets
- npm lane: package renamed @blackcatinformatics/purrdf, release-npm.yaml
 publishes on npm-v* tags (NPM_TOKEN bootstrap, then OIDC trusted
 publishing) with provenance + SBOM attestations
- version 0.1.3 across all 16 crates, PyPI, npm, and citation metadata

Removed: vendored_asset.test.mjs (gated a gmeow-ontology playground
asset that is not part of this extraction).
- Stabilize the toolchain: stable-Rust-clean workspace, real MSRV

purrdf-iri was the workspace's only nightly dependency
(#![feature(portable_simd)] for the IRI delimiter scan). CI silently ran
nightly everywhere because rustup obeys rust-toolchain.toml over the
action's toolchain input, which made the "CI builds on stable" story and
the MSRV job fiction.

- replace the portable_simd scan with a zero-dep SWAR scan over u64
 words (from_le_bytes lane order: platform-independent, wasm-clean);
 scalar-tail behavior and the full RFC 3986/3987 suite unchanged
- drop #![feature(portable_simd)]; the workspace now builds on stable
- pin rust-toolchain.toml to stable so local dev, CI, and rust-version
 (MSRV 1.96) agree; release-pypi builds on stable too
- gts: chunks_exact(2) -> as_chunks::<2> (clippy on current toolchains)
- docs: correct the nightly claims (CONTRIBUTING, AGENTS, shapes README)

Full gate green on stable: fmt, clippy -D warnings, tests, no-features,
generated artifacts, kernel hygiene, wasm32.
## [0.1.1] - 2026-07-01

### Bug Fixes

- Set crates.io release user agent
- Pace crates.io bootstrap publishes
- Set crates.io workflow user agent
- Make purrdf the umbrella crate
- Pace only new crate publishes

### Other

- First commit

