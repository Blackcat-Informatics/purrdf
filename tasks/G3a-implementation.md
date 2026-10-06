# G3a hosted assembly mechanism implementation

Status: SUCCESS for the complete assigned G3a mechanism and static metadata task.
Source is frozen; implementation ownership is released to root for independent
review, normal signed/hooked publication and actual hosted execution. G3 overall
remains OPEN: this report establishes no measured assembly, hosted execution,
seven-configuration projection qualification, final CI or merge acceptance.

## Identity and exact scope

Worktree: `/home/paudley/Active/purrdf/.worktrees/457-paged-tier-an-lsm-style-stack-of-sealed`.
Unchanged committed HEAD: `a0f8a7ed8bd293d4e9d84f182fe4f6c0afa64cbb`.
Its source tree: `fc174197e0ca974f5a5ecd0c55aea760419cadd0`.
Three unstaged changed files, 118 insertions/2 deletions:

| File | SHA-256 |
| --- | --- |
| `.github/workflows/ci.yaml` | `3a240ca1303a4d3089fb35a875a2a8a34d6ca76910fde15b7ae4cb9d2d287cc4` |
| `scripts/simd-asm-manifest.toml` | `c96ab1574d88cae967d67bcb642a7a4b7716555aaf9ca66e1b2e87fac36f760a` |
| `docs/design/purrdf-simd.md` | `30db86879aa83c616b0b4572d5feab583919f161115496e28a807419bce325c0` |

Manifest: `raw/G3a-source-sha256.txt`.
Complete binary source patch: `raw/G3a-source.patch`, SHA-256
`904234fb90c20182df846a8bc78c6597f14388c36896fbfab182628fdad3b45d`.
No other tracked file changed. Local `.stage/` remains untracked/unignored and
intact. No Cargo, toolchain installation/selection, Stage payload/configuration,
staging, commit, push, dispatch or forge posting occurred in this task.

## Executable mechanism

The existing workflow now has a default-false Boolean `simd_projection` input.
The independent projection job runs only on an explicit workflow dispatch with
that input true. Existing ordinary jobs, seven mandatory assembly shards and
their aggregate retain their production commands, dependencies and success laws.
Diagnostic names `asm-diagnostics-*` and `asm-projection` remain outside the
aggregate's existing `simd-asm-*` download pattern.

The new lane uses current pinned checkout/Rust/upload actions, the repository's
floating nightly, existing wasm32/aarch64 standard targets and C cross tools,
read-only workflow permissions and checkout credentials disabled. It has a
120-minute bound and a two-job total Cargo budget shared by two configuration
workers. The Boolean input is used in the job condition and is never interpolated
into shell commands or regexes. Nothing commits or pushes generated results.

Receipts bind the actual compiler, clean initial checkout HEAD/tree, workflow,
Makefile, root dependency/toolchain inputs, existing driver/runtime/manifest,
shared mapping and affected paging source plus initial document bytes. The lane
captures actual x86-v3/v4 parent/mapping body dumps, generates the whole seven
configuration projection, requires no other tracked delta or untracked unignored
file, and exports the exact generated document patch and document bytes. Separate
whole-matrix report qualification and report aggregation then require the generated
source and the same before/after compiler. Generation and report writing are
distinct invocations, as the existing reader requires.

The mandatory shards record small compiler/checkout/clean-source/input receipts
before measurement. Their existing measurement step now has `id: measure`; a
failure-only upload runs exactly when that step fails, preserving an incomplete
report as incomplete. The projection artifact uploads on success or failure unless
cancelled; checksum capture preserves an attributable file inventory. Each upload
selects only the explicit output directory, public `purrdf_core*.s`, graph command
logs and assembly provenance receipts. No checkout, full build directory, home,
environment, credentials or cache dump is included. The paths match the existing
runtime's `target/simd-asm/contexts/<key>` layout on a hosted non-Stage runner;
`raw/G3a-runtime-layout.txt` records the inspected home.

All measurement/generation/report failures retain their ordinary failing exit.
There is no retry, continue-on-error, exception suppression or automatic publication.

## Shared-home metadata

The manifest's one stale summary and the document's one row now identify
`QuadIds::map_ids` through `PageTranslation::to_global`, with the actual shared
definition at `dataset.rs:151` and eager-seal call at `paged/mod.rs:469`. The work
description says up to four gathers because an absent graph maps three columns.
The row describes counts as selected parent bodies of `PagedDataset::from_provider`,
without asserting isolated gather vectorization or inlining. Its wasm description
also names that enclosing scope. No outlining or scalarization explanation is claimed.

Every selector, minimum vector floor, FMA/relaxed law, mnemonic condition and all
seven measured document columns are unchanged. Existing schema and full document
coverage functions independently establish this bounded descriptive diff. New
measured cells will come only from the hosted whole-matrix generator and will be
reviewed against its exact emitted source/compiler/body evidence.

## Static qualification and corrected draft

All final checks exit 0; commands/exits/log locations are captured in
`raw/G3a-check-exits.json`:

- `actionlint .github/workflows/ci.yaml`, including shellcheck.
- Existing toolchain-pin check and its self-test.
- Existing gate-parity check and its self-test.
- Existing SIMD reader self-test: 17 tests passed, all refusal/neighbor checks pass.
- Read-only calls to existing `load_manifest`, `parse_table`, `doc_checks` and
  `workspace_world`: complete manifest/document coverage passes. Parsed old/current
  measures compare equal; exactly one summary and five descriptive cells change.
- Four `make -n simd-asm` dry runs expose the exact supported probe, generation,
  separate report and aggregate commands without invoking Cargo or the compiler.
- `git diff --check` passes.

The initial draft used direct Python calls, which the existing gate-parity census
correctly refused as additional invocations absent from `make check`. The frozen
implementation uses the existing registered `make simd-asm` entry point instead.
No exception register, Makefile or gate implementation changed. An initial unquoted
tree expression also triggered shellcheck; both receipt commands now quote
`HEAD^{tree}`. Those real failed drafts are preserved in
`raw/G3a-{actionlint,gate-parity,gate-parity-selftest}-initial.log`; they are not passes.

The wrapper adds `--doc`. Actual reader branches and parser accept that prefix with
`--probe`, `--write-doc` and `--merge-reports`: preflight checks document metadata;
probes then return supporting body evidence; generation writes before parity;
merge validates reports and document coverage. The parser explicitly refuses
`--write-doc` combined with `--report`; no workflow command combines them.
`raw/G3a-reader-mode-support.txt` and the four dry-run logs bind that assessment.

## Dependent hosted qualification

Root will independently review this frozen exact patch, publish it normally and
dispatch its branch workflow. Actual hosted body inspection, artifact download and
clean replay/source identity verification then provide the input for the assigned
generated-projection source task. Final ordinary PR CI still must pass all seven
independent shards and the unchanged aggregate on final source/compiler/candidate.
There is no blocker to taking those authorized next actions. No local stale
compiler, historical failed count, static workflow check or manual dispatch will
be represented as final G3 or issue acceptance.
