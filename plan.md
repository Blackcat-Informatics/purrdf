# Borrowed native SPARQL equality

Prerequisite for the observed GMEOW1843 single-value metadata failure. Full logic goal stays active. Actual downstream tiny production-gate test reports7 passes and one failure: distinct IRI values yield no comparison violation. Existing ORDER BY total ordering is not FILTER equality. Reuse the native owned-value equality implementation, which already handles recursive triple components and SEP-0009; do not copy semantics into GMEOW, render query text, construct a dataset per production comparison, or weaken errors.

## Task 1: Expose the owning implementation

Publish equal_values over two borrowed TermValues, explicit LanguageStringEquality and caller-supplied WorkspaceCapability. Expose the existing language-string enum instead of inventing another profile type. Core and Disjoint remain deliberate choices. Delegate to rdf_equal_in_admitted; preserve boolean/expression-error/operational-refusal states. Re-export from the evaluator facade; umbrella already re-exports it. No new dependency or stored relation, no GMEOW namespace in native code. Review actual private helper and admission paths first.

## Task 2: Prove the public native and WASM doors

Add one portable case in the existing numeric_wasm_determinism harness. Compare public equality with the actual query equality operator over tiny native datasets containing the original values: IRIs, scopes, numeric lexical variants/NaN, typed/language/directional literals, triple terms and composites, including unknown/ill-typed expression errors. Include explicit disjoint profile and admitted storage refusal where the supported admission fixture permits. Existing equality-path unit controls should also reach the public wrapper. Register the additional portable case and update exact inventory counts. Focused native/public-runtime and WASM tests, strict affected Clippy, helper hygiene and normal hooks. No GMEOW corpus, fixture factory, optimized producer or full gate; no host/resource probes/caps/banned commands.

## Task 3: Publish and integrate

Self-review only: no independent dispatcher available and none claimed. Gather full issue comments/prior art and PR reviews/threads/CodeRabbit, resolve actual findings, commit/push normally and merge only via ghprsq. Existing user authority permits hosted CI pending. Archive exact Stage before owned cleanup. Then update the downstream compiler/runtime to actually execute equality/distinctness and make the failing required-single test pass; public API alone does not complete GMEOW1843.
