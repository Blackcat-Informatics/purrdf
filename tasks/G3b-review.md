# G3b independent hosted projection and frozen source review

Verdict: **PASS**, scoped to the successful complete hosted generated projection,
its artifact/compiler/source/body correspondence, and the frozen sole-document
patch described below. No blocking artifact, source, selector or instruction-law
defect was found. This is not overall G3, issue completion, final ordinary CI or
merge acceptance. Final normal seven independent assembly shards, their unchanged
aggregate, and all other required CI remain mandatory on the published final source.

## Exact applicability

The implementer's freeze and complete `tasks/G3b-implementation.md` were read before
this verdict. Independently verified unchanged parent HEAD:
`8364b29c9fb70195f123e2bc084160d83f78850a`, tree
`cb77b4d6beb3a5b7f17e3979570e651dbebe0564`.

The index remained unchanged at the initial source-freeze review. The only
tracked working-tree delta was `docs/design/purrdf-simd.md`;
`git diff --check` passed. The complete frozen default-format binary patch exactly
equals `raw/G3b-source.patch`, SHA-256
`dd41c2fb51a065f10b622976eca465350218b1b10ae53fc5ac2c7acd2d73f78f`.
The resulting document SHA-256 is
`7285a4c4d1f401aec02d880622cdf7f9893fee4b9d781d27a61c076cd6a94356`.
After that initial observation, root staged the sole document path. Independent
final readback additionally binds this scoped PASS to **staged tree
`5d150fa4076cc75e293b517f79dd080b6ea807ed`**, with no unstaged tracked delta.
The cached patch passes whitespace checks and exactly equals both the producer's
patch and `raw/G3b-staged-source.patch`, with the same patch SHA-256 above.
The cached document equals the artifact and working document byte-for-byte.
`raw/G3b-review-staged-binding.json` retains this independent final binding.
This review qualifies that staged tree and complete delta, not a new commit.
Publication must preserve those reviewed bytes.

## Native artifact and compiler qualification

Successful hosted run `37532305791`, job `112504470023`, artifact
`asm-projection` ID `11445534280`, identifies that exact branch HEAD. Native
metadata and all recorded job steps report success. Independently recomputed ZIP
SHA-256 `cda92e37b3cb12f92735f2e3ab8880ffa5db36193d2e039ee2c9867e87923e89`
equals its native digest. All 550 extracted file bytes equal their ZIP entries;
archive paths have no absolute/traversing names or symlinks. All 16 internal
projection-file checksums pass without modifying their original receipt.

All 13 hosted before-input hashes equal the corresponding committed parent blobs,
including workflow, Makefile, Cargo inputs, reader/runtime, manifest, production
mapping sources and original document. Both after-input hashes equal the artifact,
clean replay and frozen issue-worktree bytes. Manifest SHA-256 remains
`c96ab1574d88cae967d67bcb642a7a4b7716555aaf9ca66e1b2e87fac36f760a`.

Compiler-before, compiler-after, successful report compiler and the independently
diagnosed normal-run compiler receipt are byte-identical: rustc
`1.101.0-nightly`, full commit
`ea137335b78829b4514bf1b4c16302f74fab8581`, LLVM `23.1.3`.
No local compiler qualification was substituted.

The complete report SHA-256 is
`2f7a2ff71b2511f8cfaaf8a78321a762ba4d4519276820ffa42bf290c7e8c3c2`.
Its status is `passed`, schema 1, with exactly 110 registered site IDs in every
one of the seven configurations. The existing `runtime.merge_reports` home accepts
this artifact directory with its actual hosted identity supplied explicitly; the
existing full `doc_checks` coverage/parity home returns no problems. Every site's
seven report cells also equals its complete row in both actual generation and
separate qualification logs. Both logs end with the complete gate result, and the
actual aggregate log accepts seven matching successful configuration columns.

## Clean source replay and exact generator application

Root's already-owned clean replay contains only the generated document delta and
no untracked unignored files. Reading it through the existing
`runtime.source_identity` home independently returns
`fe73fce1722108d47b7fcb7a8bcbf6214318a23800304c759282753c41b005c0`,
exactly the successful hosted report source identity. The local issue worktree's
untracked/unignored `.stage/` was preserved and never used as a replacement source
identity. This was pure byte reconstruction; the replay was not changed.

The native generator patch SHA-256 remains
`beeac41c6acb5d419762f566f05988cef7a746bdea8c66ce384e0af2723f3772`.
Existing `write_doc(original_parent_document, hosted_cells)` returns the exact
uploaded document, byte-identical to the clean replay and frozen issue-worktree
document. Existing `parse_table` proves only `core.paged-map-quad` x86-v3/v4
cells change, respectively `v14·f0·r0` and `v10·f0·r0` to `v0·f0·r0`.
All other cells and all descriptive fields are unchanged. The manifest, selectors,
floors, reader and counting laws are unchanged. No partial-shard rewrite or
hand-edited cell is involved.

## Body correspondence and supported hosted path

The projection's actual public v3/v4 core assembly files are byte-identical to
the exact normal CI files already analyzed in
`reviews/G3-hosted-body-diagnosis.md`:

| Configuration | Assembly SHA-256 |
| --- | --- |
| x86-v3 | `29d938338b5bc639ce1b09c47b4e405cc283ccc836ba707bf1c8c04e32205754` |
| x86-v4 | `6e3ab72d4572540b4133ae5e4ef3157a18e7830550de559327101644ffa9eccf` |

That diagnosis proves the selected production `PagedDataset::from_provider`
parents include the scalar page-to-global mapping loops and correctly count zero
vector work under the existing law. Moves/zeroing and transitive callees are
excluded according to the corrected enclosing-parent descriptive scope. The
projection preserves those exact compiler-command/context/graph/body bindings;
it does not replace a missing or unrelated generic body. Historical causation
for the old positive counts remains unclaimed.

The actual job log uses the registered `make simd-asm` path for both body probes,
full `--write-doc --jobs 2`, a separate full `--jobs 2 --report`, then
`--merge-reports`. The existing Makefile supplies `--doc`. Actual successful job
steps enforced clean starting source, sole-document tracked delta, no untracked
unignored outputs, and unchanged compiler before artifact upload. Qualification
explicitly reused compiler-validated build artifacts and recomputed verdicts;
it is not described as a second cold compilation. The opt-in projection artifact
does not replace or enter the mandatory ordinary shard aggregate.

## Independent execution receipts and limits

`raw/G3b-review-artifact-analysis.json` and `.log` retain the complete independent
identities, checksums, coverage, changed cells, actual body correspondence and
executed existing pure homes. Corrected analysis exits 0. The initial read-only
analysis exited 1 because it compared full-index diff formatting with the recorded
abbreviated-index diff. The corrected exact `git diff --binary` representation
matches; that failed representation assertion is not relabeled as a passed check.
Root's parallel representation disposition is retained in
`raw/G3b-root-source-verification.md`.

No source/index/history, clean replay, compiler/toolchain, Stage configuration,
Cargo, forge or sibling mutation occurred. No driver, `Runner`, `report_identity`,
local probe, new tooling file or duplicate gate implementation was used. Only
assigned Stage review/analysis receipts were written. Signed hooked transport and
the final ordinary CI/aggregate remain root-owned next steps.
