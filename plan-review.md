# Independent plan review

**PASS.** Reviewed `plan.md`, `issue-analysis.md`, `prior-art-assessment.md`, the generated issue acceptance, current C smoke/Cargo manifest/workflow/Makefile, main `.baseline`/`.goals` and applicable AGENTS constraints. This was a read-only design review except for this Stage artifact; no builds, measurements, source edits or forge mutations were performed.

## Stage planning rubric

| Dimension | Finding |
|---|---|
| Complete acceptance and prior art | The plan explicitly preserves the already-merged six-shard, dependency-cache and profile-reuse implementation instead of doing it again. It requires matched cold/warm phase receipts, actual full native coverage, real C header/link/runtime and a demonstrated reduction before closure. Instrumentation alone cannot satisfy completion. |
| Coherent bounded design | One original Rust process/receipt helper reused by the existing C test and profiling controller avoids an alternate C gate or duplicate target-selection implementation. Existing lexical/hash dev dependencies exist in `crates/rdf-capi/Cargo.toml`; no shipping dependency or semantic feature is proposed. Existing Makefile commands and source/configuration authority remain intact. |
| Evidence and comparison validity | Current-source, narrowly recorded counterfactuals isolate merged profile savings; native aggregate work and six-runner critical path are explicitly different outputs. Actual compiler, target, flags, Cargo/libtest concurrency and cache states must match. Local jobs=8 is honestly identified as local evidence. Cargo remains freshness authority on every invocation, with source/config invalidation witnesses. |
| Verification and finishability | Focused fixtures cover invalid artifacts/comparisons and failed children. The settled gate preserves O3/assertions/overflow, all six shards, dedicated C lane, downstream consumer and strict lint/hygiene. Hosted failure artifacts, independent completion audit, Stage integration and evidence preservation are included. No criterion is silently deferred. |

## Implementation obligations already implied by the plan

These are concrete checks for the implementing agent, not blocking design corrections:

- Declare the new host profiling example explicitly with `test = false`. The current target-coverage gate rejects newly test-enabled examples outside its three named examples; normal doc-shard example compilation still builds the controller. Do not weaken that gate to accommodate the tool.
- Resolve measured native concurrency from actual runner/configuration evidence, including relevant environment and any captured global Cargo configuration. `available_parallelism()` alone does not prove Cargo's effective job count if configuration overrides it. Keep six independent runners distinct from six local processes.
- Admit enough explicit time/disk bounds for the optional monolithic current-source comparison. A cancelled 30-minute baseline is a recorded failure, not a completed comparison or a speedup. Do not serialize six local shards and call their sum the hosted six-runner critical path. Profiling-only job scheduling must leave ordinary mandatory jobs and their timeout/coverage contract intact.
- Use Cargo's successful build result plus selected exact package/target artifact as authority. A receipt's `fresh` field, file hash or source hash cannot independently authorize skipping Cargo. Keep linker/platform flags equivalent when splitting C compilation/linkage.
- Controller/helper compilation must occur outside measured cold target/build directories, and source/config invalidation witnesses must remain isolated from both performance arms. Their preparation costs must not be silently included in only one side.
- A monolithic-versus-shards comparison necessarily changes the partition and runner count. Record this declared experimental difference rather than forcing a generic comparison validator to reject it as an unexplained selection/config mismatch. Require identical combined target coverage, toolchain/profile and per-runner effective concurrency.

The plan has no blocking correction. Final acceptance remains NOT MET until the campaign and actual full gates produce the specified receipts.
