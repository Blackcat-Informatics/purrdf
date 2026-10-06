# G3b exact hosted document projection

Status: SUCCESS for the complete assigned G3b source task. Source is frozen and
ownership released for independent review and normal signed/hooked publication.
The full hosted projection/report/aggregate passed; final ordinary PR CI on the
published final source and integration candidate remains required. This report
does not claim final G3, issue completion or merge acceptance.

## Source and artifact identity

Worktree: `/home/paudley/Active/purrdf/.worktrees/457-paged-tier-an-lsm-style-stack-of-sealed`.
Verified unchanged G3a HEAD: `8364b29c9fb70195f123e2bc084160d83f78850a`.
Its tree: `cb77b4d6beb3a5b7f17e3979570e651dbebe0564`.
This is also the projection checkout HEAD/tree recorded by the hosted job.

Hosted workflow run `37532305791`, projection job `112504470023`, artifact
`asm-projection` ID `11445534280`. Native artifact metadata binds that exact run,
branch and HEAD. Downloaded ZIP SHA-256 independently recomputed here:
`cda92e37b3cb12f92735f2e3ab8880ffa5db36193d2e039ee2c9867e87923e89`,
equal to the native `digest` in `raw/G3-projection-artifacts.json`.

Artifact root: `raw/G3-projection-artifact/`.
Native generator patch `asm-projection/projection.patch` SHA-256:
`beeac41c6acb5d419762f566f05988cef7a746bdea8c66ce384e0af2723f3772`.
Exact generated document SHA-256:
`7285a4c4d1f401aec02d880622cdf7f9893fee4b9d781d27a61c076cd6a94356`.
Unchanged manifest SHA-256:
`c96ab1574d88cae967d67bcb642a7a4b7716555aaf9ca66e1b2e87fac36f760a`.
Complete checked report `asm-projection/reports/all-configurations.json` SHA-256:
`2f7a2ff71b2511f8cfaaf8a78321a762ba4d4519276820ffa42bf290c7e8c3c2`.

Actual hosted compiler: `rustc 1.101.0-nightly (ea137335b 2026-10-05)`, full commit
`ea137335b78829b4514bf1b4c16302f74fab8581`, host
`x86_64-unknown-linux-gnu`, LLVM `23.1.3`. Before/after receipts equal each other
and the report's complete compiler string byte-for-byte.
Post-generation report source identity:
`fe73fce1722108d47b7fcb7a8bcbf6214318a23800304c759282753c41b005c0`.
Root's retained clean replay applies only the exact generated document patch,
verifies every other tracked byte unchanged and uses the existing runtime's
source-identity home to establish this same content digest. Receipts include
`raw/G3-clean-replay-generated-source.txt`,
`G3-clean-replay-generated-status.txt` and the input checks below. This task
never touched the clean replay or substituted the issue checkout's untracked
`.stage/` contents into hosted source qualification.

## Exact application and sole diff

`git apply --check` accepted the native generator patch against the verified G3a
head. `git apply` then applied those exact bytes. `cmp` establishes that the
resulting document is byte-for-byte identical to the artifact's complete generated
document. No cell was hand-edited.

The only tracked delta is `docs/design/purrdf-simd.md`, one insertion/one deletion:
`core.paged-map-quad` x86-v3 changes `v14·f0·r0` to `v0·f0·r0`, and x86-v4
changes `v10·f0·r0` to `v0·f0·r0`. All five other configuration cells, descriptions,
shared homes, source locations, selectors and instruction laws remain unchanged.
`raw/G3b-source-sha256.txt` binds the document and unchanged manifest.
Complete local binary source patch: `raw/G3b-source.patch`, SHA-256
`dd41c2fb51a065f10b622976eca465350218b1b10ae53fc5ac2c7acd2d73f78f`.
Its diff formatting differs from the native artifact patch's shorter index hashes;
both represent the same sole document transformation, proved by the byte comparison.

## Meaningful qualification

The actual hosted generator and subsequent separate checked report both completed
successfully for 110 sites on all seven configurations. Its unchanged report merger
then accepted seven matching successful configuration columns and document parity.
Generation, qualification and aggregate logs are retained in
`raw/G3-projection-artifact/asm-projection/`. The report has `status: passed` and
exactly all 110 manifest site IDs in each of its seven columns. Its compiler and
manifest match the retained hosted receipts and current unchanged manifest.

Root verified all 16 internal artifact checksums, all 13 pre-generation input
checksums in the clean replay, document bytes and the post-generation source digest.
The complete fail-fast verification is `raw/G3-projection-root-verification.md`.
Supporting attributable receipts remain `raw/G3-projection-artifact-check.log`,
`G3-projection-replay-input-check.log` and the clean replay files. This task re-read
the report, successful generator/qualification/aggregate results and replay identity;
it independently recomputed the ZIP/report/patch/document hashes.

The independent `reviews/G3-hosted-body-diagnosis.md` establishes actual v3/v4
production eager-seal parent bodies and scalar page-to-global gathers, with correct
compiler-command/assembly/context/graph provenance and no selector or instruction-law
defect. Each body has zero counted vector work under the unchanged reader law.
No historical cause for the old positive counts is inferred from those observations.

Local pure metadata checks call the existing `load_manifest`, `parse_table`,
`doc_checks` and `workspace_world` homes directly. Full document parity/coverage
accepts the hosted 110-by-seven report; old/current parsed rows prove only the two
named measured cells changed. This is byte correspondence and metadata acceptance,
not a local compiler measurement or a substitute report-merger invocation.
`raw/G3b-hosted-report-summary.json` records exact identity, coverage and mapping cells.

Every executed source/check command exited 0:

| Command/check | Retained log |
| --- | --- |
| `git apply --check` on the native projection patch | `raw/G3b-apply-check.log` |
| `git apply` on that exact patch | `raw/G3b-apply.log` |
| `cmp` document against native generated document | `raw/G3b-document-compare.log` |
| Existing pure document parity/coverage and parsed-delta checks | `raw/G3b-metadata-parity.log` |
| `git diff --exit-code -- . ':(exclude)docs/design/purrdf-simd.md'` | `raw/G3b-sole-source-path.log` |
| `git diff --check` | `raw/G3b-whitespace.log` |

No new tooling, dependency, test or production alternative was added. No local
Cargo/compiler/driver/Runner/report_identity invocation, toolchain selection/install,
Stage configuration change, staging, commit, push, workflow dispatch or forge post
occurred. Source ownership is released. Root's final normal seven independent
assembly shards and unchanged aggregate must qualify the published final source;
the manual projection's successful full report does not substitute for that gate.
