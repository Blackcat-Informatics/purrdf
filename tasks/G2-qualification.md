# G2 focused qualification — PASS on current source

Disposition: complete assigned focused/source-dependent qualification PASS.
Independent completion delta, normal hooks/commit/push, fresh hosted review debt
and integration remain root-owned and incomplete. Heavy lane is FREE; no active
build/test/runtime process remains owned by this task.

Current source: four Rust paths identified in G2-investigation; base HEAD
87d6c32729d30391f09673b66a8221edeeb44f0c. Sole admitted local lane, eight Cargo
jobs, scoped same-active SDK raw Cargo PATH. CARGO_BUILD_BUILD_DIR is
/opt/purrdf-479-t6-full/build and CARGO_TARGET_DIR is its sibling target.
Initial source/config SHA256 and compiler/Cargo identities live in G2-logs.
No hooks/commit/push/forge action is included.

Actual terminal results so far:

- Initial Datalog package exit101:337 unit tests passed, new worker-budget
  assertion compared Limits intentionally different between default and explicit
  exact ceiling. Every observed consumption/fact count matched. Corrected the
  oracle to compare identical explicit exact-ceiling options; similarly adjusted
  runtime baseline to compare observed join/stored-fact counts, not different
  caller limits. Initial failure retained in datalog-tests.log.
- Corrected cargo test --locked --jobs8 -p purrdf-datalog exit0:338 unit tests,
  two one-test integration suites, five native rules_runtime cases and one
  doctest:346 total. Includes quadratic recursive/default, partial additive,
  exhaustive hybrid/masking/negative/witness, workers1/4/32 and allocation cases.
- Initial strict Datalog+CLI all-target clippy exit101: collector self-test used
  format/collect, triggering format_collect. Replaced with writeln accumulation.
  Corrected identical clippy command exit0. Both logs retained.
- Actual wasm32 rules_runtime via original runner exit0:all five cases passed
  under Node/wasm-bindgen, including new recursive partial factor proof/governor
  case. wasm-runtime.log contains actual module path and terminal result.

Further actual terminal results:

- Release CLI/collector Cargo JSON build exit0; actual selected executables copied
  into fresh /opt/purrdf-rules-479-g2-qualification, SHA256 frozen. Collector
  self-test exit0. Paired recursive collector exit0 with complete validation.
  n32 pre-fix/fixed both exit0 (465 inferred facts each). n256 pre-fix exit1,
  diagnostic1,048,577 join steps observed vs1,048,576 default; its partial artifacts
  explicitly unaccepted. Fixed n256 exits0 with32,385 complete inferred facts and
  all authored proof blocks. Both use identical before/after-bound input/rules.
  Immutable pre-fix SHA256 remains6ac961a55f29b658b1ed67541808981d700a03615a49efa9d4841f1a0d0a2e88.
  Timing/RSS observations remain report-only, not a portable performance promise.
- SHACL selected five integration targets exit0:59 tests. CLI shapes_tools_cli
  exit0:14 tests, including existing nonlinear/default and term-capacity controls.
- Entire entailment package exit0:793 tests; three intentional maintainer-only
  artifact writers ignored (regenerate_goldens, write_owlrl_corpus_inputs,
  rewrite_owlrl_divergence_artifact), not qualification passes.
- Native shared validate/C/WASM golden filter exit0:9 tests, one intentional
  regenerate_regime_golden_vectors ignore. Full package targets filtered by this
  selection are not claimed run. Mechanism oracle selected test exit0:1 test.
  Owned report/closure/proof goldens pass unchanged; no generator or pin edits
  were necessary and no frozen external vector was changed.
- Fresh isolated Python locked/backend build exit0; actual ten-case shared-golden
  pytest selection exit0 (77 unrelated deselected). Installed source is this WT,
  environment /opt/purrdf-479-g2-python.
- Initial workspace fmt exit1 required only new test assertion wrapping; direct
  rustfmt corrected it. Final workspace fmt exit0. Both logs retained; whitespace
  correction does not change matcher/collector/binary behavior.

Final actual terminal results:

- Optimized public WASM package make wasm-pkg exit0 with strict private output,
  original post-link validation and138100SIMD opcodes. All affected shared crates
  built for wasm32. Public Node artifact/boundary/proof selection exit0:3 tests,
  no skips/failures. Package and actual installed Python native module identities
  are captured, rather than inferred from a cargo build alone.
- Strict all-target clippy for Datalog/CLI/shapes/entail/validate/C/WASM/
  sparql-conformance exit0 on settled source.
- helpers-hygiene/layer-hygiene/terminal-hygiene/build-profile-hygiene exit0.
  Native census/shared-helper/hash-domain controls passed;207normal edges and
  1007profile units matched. Non-Rust working-tree ratchet exit0, zero changed
  non-Rust paths against actual merge base6273b6173f3b. Actual gate parity exit0.
- Original check-generated.sh exit0: generated metadata, corpus and lexical
  projections verified unchanged. No artifact was hand-edited or regenerated
  merely to suppress a failure. Final diff whitespace check exit0.
- Final source/config/host/artifact receipt contains13SHA256 entries; independent
  command readback exit0, all13match. Initial receipt command named a nonexistent
  JSON fixture, exited1 and produced a partial file retained separately; settled
  receipt uses actual owning regime-boundary.vectors and dl-proof.vectors paths.

## Current source delta and evidence applicability

Exactly four tracked Rust files differ. Production semantics are the reviewed
shared JoinScan extension and lazy partial factor modes in seminaive.rs/factors.rs.
The remaining source is differential/governor/runtime coverage and the focused
collector route. The original twenty-case fixture bytes/route remain unchanged;
its historical guarded/connectivity source paths are also exercised by current
native/WASM runtime and CLI checks. No new dependency/feature, generated artifact,
golden/pin, external vector, callback contract or public API changed.

Qualification corrections touched only new assertions (compare consumption or
identical caller limits rather than different Limits), collector self-test string
accumulation and test whitespace. They did not change the repaired production
matcher. Recursive native/default-limit full fact/proof parity passes through
n256; exact partial frontiers/negatives/cycles/refused workers are current passes.
The actual default CLI regression is reproduced and repaired without raised limits.

The prior settled full make check belongs to committed87d6c3272. It is not claimed
as a full gate on this new source. The accepted sole plan's one full gate already
ran; the accepted Stage2 remediation explicitly reuses valid unchanged evidence
and does not mandate another blanket full gate. This changed shared matcher is
qualified by the whole Datalog and entail packages, all prescribed SHACL/CLI
consumers, native/C/Python/public WASM matrices, actual recursive paired campaign,
strict affected clippy and hygiene/metadata above. No new finding remains that
requires another full suite locally. Root decides completion/hosted/integration
acceptance after independent delta review and normal hooks; those are not local
passes or silently deferred work.

Canonical logs are tasks/G2-logs; actual campaign retained under
/opt/purrdf-rules-479-g2-qualification/recursive-campaign. Failed initial tests,
clippy/fmt/receipt and pre-fix refusal remain distinguishable from settled passes.
