# Actual native LUBM campaign preparation — 2026-10-08

Status: PREPARED; execution NOT RUN. Root's glossary full/render lane is active.
No build, test, campaign, Java, Git/index mutation or forge action ran in this
preparation. Task3 is not qualified. Root must admit the heavy lane explicitly.

## Governing source and prerequisites

Read the sole accepted plan, current production lane, acquisition/cache boundary,
Make knobs, native checker/Make fixtures and main-root baseline/goals. Current
source is `b994b61d0`, pushed after normal Task2 hooks; working tree is clean
except Stage evidence. Task2's report/review and source/binary identities are the
prior focused evidence, not substitutes for actual `make lubm` execution.

Execution directory:
`/home/paudley/Active/purrdf/.worktrees/475-benchmarks-replace-the-java-lubm`.
Campaign artifacts will be fresh `/opt/purrdf-lubm-task3-20261008.XXXXXXXX` output.
The worktree's `target` resolves to its own managed disk directory
`/opt/.cargo/target/475-benchmarks-replace-the-java-lubm-589dcf4f`.
`target/bench-artifacts` was absent at preparation: no existing cache is deleted.
Only this lane's two digest-pinned external inputs are acquired; never UBA/Java.
No unlicensed external bytes enter Stage, source, commits or report prose.

Before execution, capture HEAD/status, all source checksums, `rustc -vV`,
`cargo --version`, Python/Git/coreutils identities, available CPU/memory/disk,
the actual native executable artifact identities and existing sibling state.
Verify Task2 source receipts before reusing its CLI/checker; default Make still
does its own fresh Cargo-selected build. Refuse insufficient owned disk or
unavailable required tools rather than skip a trial.

All trials run serially under a task-owned transient systemd scope with
`MemoryMax=64G`, `MemorySwapMax=0`, inherited `CARGO_BUILD_JOBS=8` and existing
production Cargo `--jobs 8`. No persistent service or model lifecycle action.
Root admission and functional user-scope availability are prerequisites. Keep
actual exit status and log for every command; a timeout/OOM is a failure, not a
fast result. No competing owned heavy build during query timing. Timings remain
report-only and are never a hosted CI equivalence or speed assertion.

## Four finite actual Make trials

Define `campaign_root` by fresh `mktemp -d /opt/purrdf-lubm-task3-20261008.XXXXXXXX`.
Resolve `cli` from a fresh production Cargo build receipt via `lubm-check artifact
purrdf`, never a guessed target pathname. Freeze its digest before prebuilt trials.
Each Make command is run with the bounded scope wrapper and eight-job environment.

1. D1: native defaults, relative output, fresh two-input cache:

   `make lubm LUBM_OUT=target/lubm-task3-20261008/default`

2. D2: identical default config/output parent, warm unchanged cache and fresh
   run child; preserve D1 as a sibling:

   `make lubm LUBM_OUT=target/lubm-task3-20261008/default`

3. N1: nonzero seed/index, two universities, custom schema/document base,
   absolute owned output and actual prebuilt CLI:

   `make lubm LUBM_UNIVERSITIES=2 LUBM_SEED=42 LUBM_INDEX=2
   LUBM_ONTO=https://example.org/lubm-task3-schema
   LUBM_DOC_BASE=https://example.org/lubm-task3-documents/
   LUBM_OUT=${campaign_root}/nondefault LUBM_BIN=${cli}`

4. N2: identical N1 config/parent, canonical decimal aliases and a hostile
   caller locale; production's shared locale home must produce the same bytes:

   `LC_ALL=C.UTF-8 LANG=C.UTF-8 make lubm LUBM_UNIVERSITIES=2 LUBM_SEED=0042
   LUBM_INDEX=0002 LUBM_ONTO=https://example.org/lubm-task3-schema
   LUBM_DOC_BASE=https://example.org/lubm-task3-documents/
   LUBM_OUT=${campaign_root}/nondefault LUBM_BIN=${cli}`

These are separate process executions of the production Make entrypoint. The
standalone Task2 Q1/Q14 smoke cannot replace them. Do not compare D and N answer
counts as identical workloads: profile/config/schema/base/native graph differ.
Compare D1/D2 and N1/N2 exact generation names/bytes/receipts, converted aggregate,
projected ontology, query set and regime map. The original external ontology/query
digests must match in every trial while the custom projected schema identity is
separate. Check every schema RDF position using the qualified native projector;
retain its all-position/quoted-triple fixture evidence, not text substitution.

## Strict acceptance and independent observations

Every positive trial must actually execute all fourteen published queries, under
the retained normalization/regime mapping, with fourteen valid `OK`/`full` rows,
`comparable rows 14 of 14`, and `qualification COMPLETE`. Exit zero alone is
insufficient. Any `PARTIAL`, `CANNOT-EXECUTE`, `BAD-RESULTS`, subset rung, exhausted
closure, missing row or malformed success blocks Task3; preserve diagnostics and
fix the actual required behavior before retrying. Never label unsupported rows
passing or substitute a smaller workload for full qualification.

Recompute native graph acceptance with the same native checker against each
actual run's generated/converted/full aggregate files and admitted config.
Q1 uses the original fixed University0/Department0/GraduateCourse0; N1/N2 must
legitimately produce Q1's empty set, with positive full-graph Q14. Q1/Q14 full
URI-set comparisons already occur inside actual Make before summary; verify
those checks and acceptance identities, not only row counts. Preserve all full,
one-file and slice ladder identities and status/diagnostic rows. One-file/slice
are required retained artifacts, not accepted full answers.

Capture exact binary/generator/checker/source/compiler/profile/config/input/query/
receipt/acceptance/projected-schema/full-file-slice hashes and source/run identity.
After each run recheck source, tool and retained artifacts; older run siblings
must remain byte-identical. Record actual per-query regime/rung/status/rows/time
and finite trial parameters. Never report a campaign result from mutable inputs.

## Negative and sibling controls

Run these after positive evidence and outside its measured query timings.

- Actual Make admission: zero/malformed count, seed overflow, index+count
  overflow, malformed schema/base and missing/unusable prebuilt CLI must fail
  before acquisition/generation. Reuse the actual native Spec/Make fixture
  boundary; freeze cache/run inventories before/after the negative calls.
- Verified cache hit: D2/N1/N2 must report existing digest-verified inputs, with
  no fetched bytes. Record actual acquisition diagnostics; no fake downloader.
- Corrupt cache: snapshot exact verified originals from this isolated worktree
  cache into the owned campaign directory. For each of its two input files,
  corrupt only that task-owned file and run actual Make. Require digest-specific
  hard refusal before native generation, quarantined failed bytes, no refetch,
  no success summary/acceptance. Retain quarantine; restore only the original
  exact verified snapshot and validate through the acquisition home before next
  trial. Never remove/reset another worktree's cache or silently repair a miss.
- Native receipt/data tamper: copy a qualified run into fresh owned fault paths;
  never mutate qualified originals. Exercise missing/extra payload, wrong config,
  source/converted alteration, duplicate aggregate bytes and acceptance alteration
  through `lubm-check verify/recheck`. Each must refuse. Task2's actual five
  checker fixtures provide the finite fault matrix and are rerun on settled source.
- Malformed results/wrong URI set/duplicates/unbound cells: actual native result
  checker fixtures must fail. Production Make's result path must emit BAD-RESULTS
  for malformed successful engine output, while a nonzero engine exit remains
  CANNOT-EXECUTE. Exercise these real Make status branches through fault injection
  delegating all unfaulted version/conversion/query calls to the same actual CLI;
  the injection is not an alternate generator/checker or a passing query result.
  Also require Q1/Q14 failure to revoke graph acceptance and suppress completion.
  If the existing fixture surface does not reach these production branches, add
  the smallest original Rust qualification fixture in the bench home; no new
  Python/JavaScript algorithm or canned passing checker. This requirement cannot
  be silently replaced by parser-only tests.
- Retain failed-run artifacts and verify older successful siblings untouched.
  Failure control checks absence/revocation of reusable graph acceptance, not
  destruction of the generator's independently bound source receipt.
- Affected WatDiv/scale/shared-law controls: rerun actual bench lane-law,
  Make-lane and Make-scale/scale-CLI tests plus acquisition offline self-test.
  No full WatDiv network/performance campaign is necessary: its implementation
  is unchanged; its actual Make refusal/probe/cache-law paths are exercised.

Focused command:
`CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=1 cargo test --locked --jobs 8
-p purrdf-bench --test lubm_check_cli --test lane_common_laws
--test make_bench_lanes --test make_scale_corpus --test corpus_cli`.
`python3 scripts/benchmark-acquire.py --self-test` preserves the actual offline
cache corruption/hit/ignore and WatDiv pin laws. Run relevant strict clippy/fmt/
hygiene/metadata again only if actual campaign failures require source changes.

## Handoff

### Source preparation after initial report

Root added the qualification-only original Rust example
`crates/bench/examples/lubm_fault_cli.rs`. It delegates unchanged arguments and
inherited streams to `LUBM_FAULT_REAL_BIN`, intercepting only the exact query
bytes read from `LUBM_FAULT_QUERY_FILE` (with the production shell's trailing-LF
removal). `LUBM_FAULT_MODE=malformed|exit|wrong-uri` produces only deliberate
failure output, never a passing answer. Missing/invalid configuration and recursive
self-delegation fail actionably. All version/conversion/regime-probe/unselected
query calls still reach the real CLI. No production lane algorithm changed.

Source has been rustfmt formatted and whitespace checked. Compilation, strict
clippy, actual delegation, selected-query faults and campaign acceptance remain
NOT RUN while the glossary gate owns the heavy lane. Build this example normally
after admission, retain its Cargo-selected artifact identity alongside the actual
delegated CLI identity and selected-query/config identities, and exercise actual
Make with fresh owned fault output. Select Q1 and Q14 by the qualified positive
run's actual `regimes.tsv` paths, not guessed filenames. Freeze/recheck both
executables and query bytes before/after every fault run. Require actual
BAD-RESULTS/CANNOT-EXECUTE diagnostics, no COMPLETE summary, revoked graph
acceptance and untouched successful siblings. A source addition is not execution
evidence or a Task3 PASS.

Write `tasks/T3-implementation.md` with actual commands/terminal statuses,
all named trials/faults, measured rows/times and source/artifact receipts;
update validation.md without replacing prior Task1/Task2 evidence. Failed
required behavior remains FAILED until fixed and rerun. Release the shared heavy
lane only after the active command terminates. Root owns independent review,
normal commit/push/progress, full Task4 gate and all forge/integration actions.
