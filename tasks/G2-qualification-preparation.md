# G2 focused qualification preparation — 2026-10-08

Status: PREPARED; all commands below NOT RUN on G2 source. Root's glossary full
gate/render owns the heavy lane. Explicit admission is required. No source edit,
build, test, benchmark, Git/index or forge action was performed in this preparation.

Use the current isolated worktree, eight Cargo jobs, task-owned fresh disk under
/opt and no competing owned heavy work during actual CLI measurements. Preserve
prior immutable binaries, failures and siblings. Capture HEAD/diff/source hashes,
compiler/tool identities, resource state and every actual exit/log. Earlier full
qualification and twenty-case campaign predate G2, not current merge proof.

## Focused source-dependent commands

Every Cargo invocation sets `CARGO_BUILD_JOBS=8` and `--jobs 8`:

1. `cargo test --locked --jobs 8 -p purrdf-datalog`.
   Includes the recursive32/64/128/256 default-limit differential, partial additive
   heads, exhaustive delta/frontier/negative/hybrid cases, worker refusal matrix,
   integration allocation and negative governor controls; preserve every actual
   count, not an assumed list. Whole package is proportionate to shared matcher.
2. `cargo clippy --locked --jobs 8 -p purrdf-datalog --all-targets -- -D warnings`.
3. `cargo test --locked --jobs 8 -p purrdf-shapes --test rules_engine
   --test srl_language --test rule_capacity_limits --test rules_conformance
   --test rules_serialization_roundtrip`.
4. `cargo test --locked --jobs 8 -p purrdf-cli --test shapes_tools_cli`.
5. `cargo test --locked --jobs 8 -p purrdf-entail`.
   Shared Datalog matching/credits can change report goldens. Any failure must be
   classified using actual payload diffs and owning generators; facts, proofs,
   authored rule programs and frozen external vectors cannot be papered over.
6. `cargo build --locked --jobs 8 --target wasm32-unknown-unknown
   -p purrdf-datalog -p purrdf-shapes -p purrdf-entail -p purrdf-wasm`.
7. `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=$PWD/scripts/wasm-test-runner.sh
   cargo test --locked --jobs 8 --target wasm32-unknown-unknown
   -p purrdf-datalog --test rules_runtime`.
   Run actual Node/wasm-bindgen host; a compile pass is not runtime evidence.
   Include relevant partial-factor/governor shared-kernel runtime coverage;
   parallel-thread tests correctly remain native only.
8. Strict all-target clippy for any changed collector/consumer source, workspace
   fmt and affected shared-helper/layer/profile/parity gates. Regenerate metadata
   normally only if source/generator projections change; audit owning golden
   differences before accepting updated artifacts.

No entire twenty-case performance campaign repeats solely because tests grew.
The native default-limit recursive CLI case is new affected-path evidence;
existing source-bound unrelated original-case measurements remain historical.

Runtime prerequisite is now delivered in source: the existing harness registers
a fifth n16 recursive factor plus independent ground-atom case. It checks every
inferred pair, all three authored premises with graph/rule identity, complete
fact cardinality, exact credits and zero/one-below typed refusals. Run the actual
registered five-case native/WASM harness; the old four alone would not qualify
the changed partial-factor branch. No execution is claimed by source delivery.

## Actual recursive CLI pre-fix/fixed control

Existing immutable pre-G2 CLI:
`/opt/purrdf-rules-479-3ec2dbba8-qualification/purrdf-v3`, SHA256
`6ac961a55f29b658b1ed67541808981d700a03615a49efa9d4841f1a0d0a2e88`.
Verify this hash before/after use. Do not rebuild or mutate the pre-fix binary.
It is feasible to run after admission; availability does not imply a result.

Build the fixed release CLI with actual Cargo JSON artifact selection:
`cargo build --locked --release --jobs 8 -p purrdf-cli --bin purrdf
--example rules_campaign --message-format=json`.
Copy selected artifacts into a fresh owned evidence directory and freeze their
identities. Do not infer their path from a shared mutable target or overwrite v3.

Collector prerequisite is now delivered in its original Rust home:
`--recursive PRE_FIX_BINARY FIXED_BINARY NEW_DIRECTORY` reuses child/status/RSS
capture and complete fact/proof validation. After artifact selection run the
selected collector `--self-test NEW_OWNED_SCRATCH_ROOT`, then its `--recursive`
mode with the immutable pre-fix and freshly selected fixed CLI artifacts and a
new evidence directory. The self-test exercises accepted recursive blocks and
missing/duplicated premise refusals. Include strict all-target CLI clippy. These
are prepared actual entrypoints, not already executed passes.

The focused fixture uses the existing namespace and original Rust generation:
for n=32 and256, seed both `k:reach` and `k:edge` on each adjacent pair
`k:node_i → k:node_{i+1}` plus one ground `k:enabled k:condition k:value` fact.
Authored SRL body is `?x k:reach ?y . ?y k:edge ?z .
k:enabled k:condition k:value .`; head is `?x k:reach ?z`.
Generate input/rules once and pass the identical files to both actual binaries:

`purrdf rules --srl RULESET --explain=PROOF INPUT OUTPUT.nt`.

Do not set/raise join, fact, arena or iteration limits. Unchanged native join
default is1,048,576. Expected complete inferred facts are precisely all ordered
chain pairs of distance≥2, count `(n−1)(n−2)/2`. Each authored proof must include
the recursive reach prefix, unique final adjacent edge and ground condition in
that order, with actual authored rule identity. Compare every fact and every
complete proof block; missing, duplicate, fabricated or dropped premise fails.
Keep stdout/stderr/argv/exit/source/artifact/time/RSS receipts separately.

The pre-fix result is an observation: it may succeed for the small case and
refuse the larger case. Do not assume its exit or diagnosis. If it succeeds,
validate complete output/proof. If it refuses, retain actual typed/CLI diagnostic
and all partial artifacts, without calling them accepted output. The fixed
binary must complete both exact cases at default limits. Native deterministic
quadratic counter growth is established independently by Datalog tests; timing
is report-only. A governed probe failure or unavailable immutable artifact cannot
be relabeled as speedup proof.

After fixtures/checks settle, update G2 investigation/remediation/validation with
actual results and source receipts. Root chooses proportionate final qualification
and owns normal hooks/commits/push, hosted review debt and ghprsq integration.
