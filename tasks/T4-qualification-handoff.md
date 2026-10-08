# Task4 settled qualification handoff — 2026-10-08

Status: HISTORICAL PREPARATION, superseded by [settled Task4 implementation PASS](T4-implementation.md) and [independent completion PASS](T4-completion-audit.md). Original prelaunch protocol below is preserved as historical evidence.

Historical status: PREPARED, full qualification NOT RUN. Root must explicitly admit the
sole local lane after308's current bounded work settles. This preparation used
source/file reads only; no Cargo query/build/test, runtime probe, campaign,
source/Git/forge mutation or process management. Other-repository submissions
remain excluded and untouched.

## Binding scope and prerequisites

Accepted sole plan Task4 requires one settled full local gate, normal hooks,
independent completeness audit before PR creation, then Stage2 hosted/review and
Stage3 predicted-merge/evidence/ghprsq integration. Source is normally committed/
pushed a82d2426e, following ordinary main integration through448. Governing
main-root .baseline/.goals, worktree AGENTS and empty .deficiencies marker read.
Stage1/stagectl validation and archive policy read; a task/stage transition alone
does not require another whole-suite run. No full475 gate has yet qualified this
source. Earlier unrelated479/260/448 full gates do not replace it.

The existing managed worktree target symlink resolves to
`/opt/.cargo/target/475-benchmarks-replace-the-java-lubm-589dcf4f`; use normal Stage
Cargo selection/cache containment. The wrapper strips explicit target/build-dir
overrides, so do not pretend an arbitrary environment target establishes private
ownership. No raw-Cargo bypass, shared clean, cold-cache reset or new cache policy
is needed for this correctness gate. Own logs on disk, not RAM or /tmp.

Read-only PATH inventory found cargo/rustc/rustup/node/python3 in Stage root/bin,
make `/usr/bin/make`, wasm-opt `/usr/bin/wasm-opt`, C compiler `/usr/local/bin/cc`.
The active-toolchain wasm32 standard-library directory exists. This is availability
evidence, not a fresh version/runtime check. Actual Make prerequisite recipes
require runnable Node and Binaryen130; fullgate must enforce them. Before admission/
execution inspect free disk/memory and sibling state read-only; do not stop siblings.
After admission capture actual `rustc -vV`, `cargo --version`, Node/Binaryen/Python/
Make/cc identity and `rustup target list --installed` in the log directory.
Prior T3 compiler was rustc1.100.0-nightly4b6d04e70/cargo1.100.0-nightly7941be6fb;
floating nightly remains prescribed, with no global dated pin. Diagnose a changed
toolchain normally, rather than suppressing new findings.

The real `make check` runs workspace and preserve-order consumer fmt/clippy/check,
all registered hygiene/selftests/metadata drift checks, workspace/consumer tests,
kernel ring fence and every declared release-library wasm build, including the
host-tool bench library. Its prerequisite/harness graph needs Node, Binaryen,
Python, C compiler and installed wasm target. Existing full Make recipes inherit
`CARGO_BUILD_JOBS=8` (overrides user config jobs24); do not edit recipes merely to
duplicate --jobs flags. Direct auxiliary Cargo commands use --jobs8 as well.
Gate profile is opt3 with assertions/overflow enabled and warnings denied; never
substitute release tests or relax flags. Limit libtest concurrency explicitly8.

## One exact fullgate launch after root admission

Use a fresh no-overwrite disk log directory, for example
`mktemp -d /opt/purrdf-lubm-task4-20261008.XXXXXXXX`, and record its path in Stage.
From the resolved475 worktree, execute once (LOG_DIR is that admitted owned path):

```bash
systemd-run --user --scope --same-dir \
  --unit=purrdf-lubm-475-task4-full-20261008 \
  --property=MemoryMax=64G --property=MemorySwapMax=0 \
  env CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=8 CI=1 make check \
  >"${LOG_DIR}/make-check.log" 2>&1
qualification_status=$?
printf '%s\n' "$qualification_status" >"${LOG_DIR}/make-check.exit"
```

Keep the actual tool session/child terminal and command in T4 report. Observe the
active scope's MemoryMax/MemorySwapMax and retain that output. `CI=1` makes the
existing wasm target-absence branch hard-fail, rather than silently SKIP. No
Make -j or parallel campaign; the command's Cargo graph is capped8. Preserve
full output and first true failure. OOM/abort/timeout/nonzero or an essential SKIP
is not PASS. Root receives milestones and actual terminal; lane is released only
after child termination. If a source-dependent failure occurs, own/repair the
specific defect and affected checks before deciding whether required fullgate must
be repeated; do not silently waive it or automatically add a repeat count.

## Evidence reuse and completion map

| Required surface | Applicable settled evidence | What remains |
|---|---|---|
| Original native profile/generator, published graph/cardinality constraints | T1 implementation/review; native corpus matrix, parsed graph qualification and retained profile source | Independent final all-criteria source/caller audit; fullgate library tests |
| Production checker/oracles/schema projection/admission/Java retirement | T2 implementation/review,154 focused tests, actual entrypoints, metadata/hygiene; no source changes to native algorithm since | Audit complete actual callers/docs/Make/CI and absence of obsolete Java execution; fullgate graph/layer/ratchet gates |
| Real14-query default/nondefault/cache/parity/fault behavior | T3 implementation/current-coverage-audit PASS, four14full campaigns, exact graph rechecks,6 realMake faults,13 admissions,2 corruption/quarantine controls,8 tamper controls,106 focused tests/offline and explicit result neighbors | No campaign repeat solely for Task4; retain exact finite100m budget and identities |
| Generated artifacts/dependency/host graph | T2 make metadata PASS with no drift; current four-path join-budget patch adds no Cargo/generated dependency shape | Fullgate check-generated/layer/license/profile/feature/ratchet results. Regenerate via make metadata only for real drift, never hand-edit |
| Workspace warning/test/wasm portability | Earlier focused coverage only | This one full makecheck actualexit0, no required SKIP; do not substitute otherbranch CI |
| Normal hook claim | Root's source corrections and a82 normalhooks28411 exit0/push53831 exit0 | Any later actual source commit uses unchanged normalhooks; no empty verification commit or bypass |
| Complete Stage handoff | plan,issue-analysis,prior-art(+assessment),plan-review,T1/T2reviews,T3current-coverage-audit,validation present | T4 implementation/logs and genuine independent final completion criterion matrix; substantive verdict before PR |
| Hosted/public host acceptance | T3 actual production native CLI/Make and real RDF conversion are qualified | Mandatory PR CI and reviewed integration; Python/npm/C public host claims remain their own actual jobs, not inferred from makecheck |

`make check` is not `pytest`, npm package runtime, C distribution publishing,
simd-asm measurement, translation rendering or release. None is newly invented
as a discretionary local gate for this unchanged host-only benchmark task.
Existing mandatory hosted jobs must actually finish; any dependency/caller change
revealed by the final audit can require affected host qualification. No Java,
WatDiv network/performance, external submission, release or issue280 execution.

The independent final auditor must read the complete accepted criteria, actual
issue/prior-art surface, T1/T2/T3 source/caller evidence, current fullgate terminal
and integration assessment. It must distinguish native graph oracle proof from
SPARQL circularity, zero-index versus legitimate Q1zero, custom external/projected
schema provenance, full versus subset ladder and report-only timing. Verify all
required behavior is implemented and demonstrated; unsupported/partial is not
success. Prior known failed initialcaller/partialbudget runs stay historical.
Presence/size gates never replace verdicts. Missing required contract evidence
blocks PR creation; no deficiency ledger descope or arbitrary reviewer quota.

## Selected Stage archive constraints

Current measured receipts and full payloads are outside Stage in owned /opt;
current reports link them. Before eventual worktree cleanup, root must select the
authoritative Stage and preserve the evidence needed to replay each claim durably,
not rely on symlinks into disposable worktree target or absolute links alone.
Inventory essential command/terminal/compiler/config/query/receipt/acceptance/
manifest/parity/fault/rawdiagnostic records and final fullgate log. Retain complete
required issue/review adjudication. Source is the signed normal source history;
do not indiscriminately archive whole targets/checkouts. External ontology/query
licensed bytes and generated payloads remain ignored build output and must not be
copied indiscriminately into source/Stage commits. Preserve permitted replay
identities and durable owned artifacts; disclose the archive's reconstruction
dependencies. Security/content guards apply to raw evidence as well as prose;
lossless packaging cannot be used to evade them. No archive mutation is performed
by this preparation.

After qualified local completion root owns stagectl PR creation/plan publication,
live complete hosted/review remediation, current merge-tree reuse judgment and
ghprsq with exact selected Stage. Freeze evidence writers before capture, confirm
archive durability before cleanup, and delete only the merged475 delivery. No
submission elsewhere is resumed by this handoff.
