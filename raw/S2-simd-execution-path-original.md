# Hosted assembly projection and diagnosis execution path

Read-only recommendation, 2026-10-06. No source changes, Cargo, Rust installation/selection, Stage configuration/payload changes, workflow dispatch, or forge mutations were performed. The Stage installer/selection rejection remains respected; direct compiler binaries, alternate toolchain homes, and environment overrides on this host are not a workaround. Hosted CPU CI provides an actionable qualification path.

## Supported surface, freshly inspected

The actual workflow is `.github/workflows/ci.yaml`, display name `CI`. It already has `workflow_dispatch`, alongside mandatory pull-request and main-push runs. GitHub workflow metadata is captured in `raw/S2-simd-workflow-metadata.json`. Installed `gh workflow run --help` explicitly documents `--ref` as the branch or tag containing the workflow version to run, and `--raw-field`/`-f` for dispatch inputs. There is no `ci.yml` here.

The existing reader supports all necessary hosted computation without changing its Python implementation:

- `--write-doc` measures **all seven** configurations and writes the generated document cells. A partial `--config` combined with `--write-doc` is refused.
- `--write-doc` and `--report` together are explicitly refused. Generation and report qualification must be separate invocations.
- `--doc --report FILE` checks production kernels, complete document coverage/parity, source/compiler stability, and writes a checked report. With no `--config`, its one report contains all seven configuration columns.
- `--merge-reports DIR` validates matching current source/manifest/compiler identities, successful report status, exact per-site coverage, exactly one column per configuration and all seven configurations. It accepts one full-matrix report or seven disjoint shards. It cannot write a projection and cannot be combined with measurement/writing flags.
- `--probe REGEX --crate purrdf_core --config NAME --dump` prints actual matched bodies and instructions. A probe is diagnostic, not a gate; it cannot write a report.
- `--report` writes an `incomplete` envelope before measurement. Failed document-parity jobs therefore already leave an attributable envelope, even though the current upload step only executes after success.

`scripts/check-toolchain-pin.py` requires new workspace-gate install steps to use the repository's floating `nightly`. The proposed hosted job does that; it does not add a dated channel or local override. Exact emitted evidence is attributed to the actual hosted `rustc -vV`, rather than assuming that a future run uses the old `ea137...` compiler. If the final normal CI compiler differs from projection's recorded compiler, final evidence must be reconciled against that newer identity.

## Minimal lasting patch scope

Change only `.github/workflows/ci.yaml` for the execution mechanism. Correct the already identified stale mapping description in the separately reviewed SIMD manifest/document repair; do not grow the Python driver or invent another projection implementation. Normal seven-shard checks and their mandatory aggregate stay intact.

1. Add one Boolean, default-false dispatch input, for example `simd_projection`. Use it only as the condition for a new `simd-asm-projection` job:

   ```yaml
   workflow_dispatch:
     inputs:
       simd_projection:
         description: Generate and verify the complete SIMD document projection
         type: boolean
         default: false
   ```

   New job condition: `github.event_name == 'workflow_dispatch' && inputs.simd_projection`. Keep workflow permissions `contents: read`; add no write token, auto-commit, auto-push, or PR publication behavior. Existing ordinary jobs still run on this dispatch. The projection job is independent, so existing parity failures do not prevent artifact generation.

2. Add that opt-in Ubuntu job with an explicit sufficient timeout (120 minutes), existing pinned checkout/Rust/upload action revisions, `persist-credentials: false`, floating `nightly`, and both cross-target standard libraries. Use the same existing apt cross-toolchain installation as the SIMD lane. Set a bounded Cargo budget (2) and configuration workers (2). The driver shares its effective Cargo job budget across workers; preserve the existing flags and profile verification.

3. Add before/after-generation receipts and v3/v4 emitted-body probes, then full generation, separate checked report, and aggregate validation. All outputs live under ignored `target/asm-projection/`. The job must fail normally on any command failure; no `continue-on-error`, swallowed exit, or false-success status.

4. Give the existing shard measurement step an `id: measure`. Add a failed-measurement diagnostic artifact upload using `if: failure() && steps.measure.outcome == 'failure'`, with the same pinned upload action. Archive the specific public evidence outputs listed below, not the entire checkout, Cargo home, environment, cache, credentials, or build directory. This retains useful evidence when a kernel or parity gate fails. Leave successful `simd-asm-${{ matrix.config }}` report artifacts and aggregate semantics unchanged.

## Concrete supported command sequence for the opt-in job

Separate sequential workflow steps are preferable for clear failure ownership. These commands illustrate supported invocations; they are not executed by this diagnosis.

**Before any measurement:** create ignored output directories and capture actual compiler and checkout identities. `git rev-parse` supplies the original checked-out commit and tree; hash the fixed source/gate inputs, including the pre-generation document.

```sh
mkdir -p target/asm-projection/reports
rustc -vV > target/asm-projection/compiler.txt
git rev-parse HEAD HEAD^{tree} > target/asm-projection/checkout.txt
git status --porcelain=v1 > target/asm-projection/source-status-before.txt
test ! -s target/asm-projection/source-status-before.txt
sha256sum Cargo.toml Cargo.lock rust-toolchain.toml scripts/check-simd-asm.py scripts/simd_asm_runtime.py scripts/simd-asm-manifest.toml crates/rdf-core/src/ir/dataset.rs crates/rdf-core/src/ir/paged/mod.rs crates/rdf-core/src/ir/paged/query.rs crates/rdf-core/src/ir/paged/translation.rs docs/design/purrdf-simd.md > target/asm-projection/inputs-before.sha256
```

Require the checkout's source status to be empty before generating. The workflow input is Boolean and is never interpolated into shell code or regexes.

**Emitted-body probes, on the checked-out production source:**

```sh
python3 scripts/check-simd-asm.py --config x86_64-v3 --jobs 1 --crate purrdf_core --probe 'PagedDataset.*from_provider|QuadIds.*map_ids|PageTranslation.*to_global' --dump > target/asm-projection/x86-v3-body-probe.log 2>&1
python3 scripts/check-simd-asm.py --config x86_64-v4 --jobs 1 --crate purrdf_core --probe 'PagedDataset.*from_provider|QuadIds.*map_ids|PageTranslation.*to_global' --dump > target/asm-projection/x86-v4-body-probe.log 2>&1
```

The reviewer must inspect the actual selected `from_provider` body and gather call sites. If source uses a corrected production selector after this review, these fixed regexes should name that same selector/home rather than silently losing it. A missing optional `map_ids` out-of-line body may be legitimate inlining; the required parent and its actual instructions must exist. Probe output is supporting evidence only.

**Whole-matrix generation:**

```sh
python3 scripts/check-simd-asm.py --write-doc --jobs 2 > target/asm-projection/generation.log 2>&1
git diff --exit-code -- . ':(exclude)docs/design/purrdf-simd.md'
git diff --binary -- docs/design/purrdf-simd.md > target/asm-projection/projection.patch
cp docs/design/purrdf-simd.md target/asm-projection/purrdf-simd.md
sha256sum docs/design/purrdf-simd.md scripts/simd-asm-manifest.toml > target/asm-projection/inputs-after.sha256
git status --porcelain=v1 > target/asm-projection/source-status-after.txt
```

The excluded-path check ensures generation changed no source outside the one intended projection. `source-status-after` should be empty or show only this tracked document. Both an empty patch (already current) and a generated document delta are valid; neither permits a source mutation elsewhere. `cp` exports an artifact, not another implementation.

**Qualification on the resulting projection source, using the same hosted compiler and warm contexts:**

```sh
python3 scripts/check-simd-asm.py --doc --jobs 2 --report target/asm-projection/reports/all-configurations.json > target/asm-projection/qualification.log 2>&1
python3 scripts/check-simd-asm.py --merge-reports target/asm-projection/reports > target/asm-projection/aggregate.log 2>&1
rustc -vV > target/asm-projection/compiler-after.txt
cmp target/asm-projection/compiler.txt target/asm-projection/compiler-after.txt
```

The checked report binds the **post-generation** source identity; it does not pretend the modified document belongs to the original checkout commit. Original checkout identity plus the sole generated patch reconstructs that source. No Rust installation happens between these steps. The driver itself refuses compiler/source changes during measurement and report writing. Upload after success and failure (`if: !cancelled()`) to preserve any refusal; an incomplete report remains clearly incomplete.

## Credential-safe artifact contents

The opt-in projection artifact should contain:

- `target/asm-projection/`: generated document, exact patch, before/after compiler and source/input receipts, v3/v4 body dumps, generation/qualification/aggregate logs, and the checked full-matrix report or explicitly incomplete report on failure.
- Exact core assembly outputs only: `target/simd-asm/contexts/**/purrdf_core*.s`.
- Their graph-membership and compiler-command context logs: `target/simd-asm/contexts/**/cargo.graph-*.stdout.jsonl` and `cargo.graph-*.stderr.log`.
- Assembly provenance receipts: `target/simd-asm/contexts/**/evidence/*.json`.

These files contain source symbols, generated instructions, compile command lines and hashes from public CI; no environment dump or Cargo/Git credentials. The graph stdout/command receipts are needed to bind each core `.s` to the relevant graph/configuration and verified invocation. Use the compiler/context receipts to select the actual core assembly bodies, not arbitrary files from multiple contexts. Avoid including `target/**`, full context directories, whole parsed caches, home directories, or action temporary directories.

For the existing failed-shard diagnostic artifact, retain `target/asm-reports/${{ matrix.config }}.json` (its `incomplete` status must remain visible), the same narrow core assembly/graph logs/provenance receipt patterns, and an explicitly captured `rustc -vV` plus checkout `HEAD`/tree and gate/manifest hashes. Capture these small identity files before the measurement in a dedicated ignored `target/asm-diagnostics/` directory; this uses the hosted compiler already installed by that shard. Name the artifact `simd-asm-diagnostics-${{ matrix.config }}` so the normal aggregate's `simd-asm-*` glob does not ingest extra JSON accidentally: **also narrow the aggregate download pattern to an explicit successful-report prefix**, or preferably choose a diagnostic name outside the current prefix, such as `asm-diagnostics-${{ matrix.config }}`. The safer minimal choice is the latter, requiring no aggregate edit. Likewise name the opt-in projection artifact `asm-projection`, outside `simd-asm-*`.

Artifact upload does not itself establish qualification. Validate downloaded bytes against metadata and the provided checksums; inspect the report's status, full seven-column/site coverage and identical compiler/manifest/source identities. Verify generated patch applies to the recorded checkout and reconstructs the generated document hash. Inspect both actual body dumps and their `.s`/command provenance before adjudicating the mapping selector.

## Publication and final acceptance

After a fresh G3 implementation is reviewed, normally committed with hooks and pushed, the root can dispatch the existing workflow version on the authorized issue branch:

```sh
gh workflow run ci.yaml --ref paudley/457-paged-tier-an-lsm-style-stack-of-sealed -f simd_projection=true
```

Capture returned run identity and verify its actual `head_sha`/checkout source. The new projection job must not be marked a substitute for normal CI or use the old run's source. Download and verify `asm-projection`, inspect actual bodies, and apply the generated document only to its exact source/manifest context. A generator patch that disagrees with a subsequently changed production source must be regenerated. Review the resulting descriptive/projection diff and normal signed-hook commit; push and verify remote identity.

Final normal PR CI still requires all seven `simd-asm-config` jobs and the unchanged `simd-asm` aggregate, bound to final PR source/candidate and actual compiler identities. Preserve the seven successful independent final reports. The opt-in job's full-matrix report is useful reproducible projection evidence, but neither its manual dispatch status nor any failed old shard is final acceptance. This route avoids Stage mutation and preserves sibling builds throughout.

## Validation for the proposed G3 change

- Review the exact YAML diff: Boolean/default-false opt-in, no weakened existing check or permission, supported action pins and safe artifact names/paths, bounded resources, normal hard failures, no interpolation of untrusted free-form inputs, no new Rust/dependency/Cargo feature or Python tooling.
- Validate YAML syntax using existing repository/host tooling; run applicable workflow/toolchain/gate-parity checks and normal commit hooks. No new test that mirrors YAML is necessary.
- Hosted artifact acceptance is the meaningful behavior check: failure diagnostics exist on a real failing lane; projection runs the whole matrix, captures v3/v4 actual bodies, writes only the expected document, and qualifies its post-generation source with a separate report and aggregate.
- After applying the reviewed projection, require final normal hosted CI and all Stage review/completion gates. No overall PASS is claimed here.
