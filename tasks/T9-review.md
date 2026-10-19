# T9 independent review (Stage 1 Task 9, PurRDF #570; G7, G13, U5; rows R22, R37)

Scope: uncommitted diff over 0177c4422 (Makefile, release-cargo.yaml, check-release-notes.py,
publish-github-release.py, test_github_release.py, docs/RELEASE.md, docs/CONFORMANCE.md).
Nothing was pushed, tagged, or released; `make release-tags` was only run as `make -n`.

## Evidence

1. CI clean-tree risk (release-cargo.yaml)
   - Steps before `make w3c-earl`: checkout; rust install; tag/version check (cargo metadata);
     release-notes check; crates.io record check; "Verify workspace" (fmt --check, check --locked,
     helper-census, check-generated.sh, check-versions/publish-order, wasm check/build, two cargo
     tests); "Package release crates" (cargo package --locked --no-verify -> target/package);
     "Audit" (fetch-locked-deps.py = `cargo fetch --locked` into ~/.cargo, package-licenses
     --check/--audit read-only, receipt -> target/); cargo install cargo-c (~/.cargo);
     capi-bundle (target/dist, private target dir under target/); package-text-lexicons (target/dist).
   - Every output lands under `target` (.gitignore `/target`, `**/target`) or outside the tree.
     check-generated.sh generates into a scratch beside the build output and diffs; package-licenses
     runs only in check/audit mode (write mode is `make license-bundles`, not used in CI).
     `--locked` everywhere, so Cargo.lock cannot change. `make release-tags` already asserts a clean
     tree after `capi-bundle` locally, which corroborates this. No plausible dirt source found.
   - Failure timing: the step runs BEFORE any `cargo publish` / GitHub Release step, so a dirty-tree
     refusal or test failure aborts before the irreversible crates.io loop, not at publish time.
   - Toolchain: the job env selects the stable channel. No `#![feature` in crates/shapes or
     conformance-kit; the same job already runs `cargo check --workspace --lib --tests --locked`
     (compiles these test targets and their dev-deps from the lockfile) and `cargo test -p purrdf-gts`
     on stable. cargo is present (it builds the C SDK). Registry network is available (cargo install
     cargo-c). LIMITATION: this machine has only the pinned nightly, so the harness was run on
     nightly, not stable/1.98; stable is inferred (see F2).
2. YAML: `actionlint` exits 0 on the workflow (python yaml not installed). Diff indentation checked:
   the added lines are inside existing `run: |` and `subject-path: |` / `path: |` block scalars.
3. Asset flow: `checksum_bytes` includes EARL_FILES from target/dist in the sorted SHA256SUMS (6
   non-manifest files); `publish()` builds `paths` from `asset_names()` (which unions EARL_FILES)
   under target/dist, so upload finds them; `verify_assets` count and name set are derived (7).
   Step order (w3c-earl before `--checksum-output`) is correct. Ran:
   `check-release-notes.py --self-test` OK; `publish-github-release.py --self-test` OK;
   `python3 -m unittest scripts.test_github_release` 31 tests OK (expects "exactly 7").
   Upload uses `--clobber`; EARL output is deterministic (two runs, absolute and relative dir:
   identical sha256), so a workflow re-run into an existing draft re-uploads identical bytes.
4. Ratchet: `helper-census --non-rust-ratchet --merge-base-with origin/main --target worktree`
   passes (7 changed paths vs merge-base 410a927d9e97).
5. Makefile: `make help` lists `w3c-earl`; `make -n release-tags VERSION=9.9.9` orders
   capi-bundle -> w3c-earl -> clean-tree check -> branch check -> fetch/main-sync -> tags -> push;
   `make w3c-earl` on the dirty tree prints the refusal and exits 2. In a scratch local clone at
   0177c4422 with the diff applied and marked clean, `make w3c-earl W3C_EARL_DIR=/abs/path` and
   `W3C_EARL_DIR=rel/dir` both exit 0, run both harnesses (11 tests each pass), print absolute
   paths, and produce byte-identical files. check-toolchain-pin.py passes. (Scratch removed.)

## Findings

F1 (low, non-blocking): stale comment in `.github/workflows/release-cargo.yaml` (~line 283) says
   "uploads all five assets"; now seven. Fix: change to "all seven assets" (comment only).
F2 (low, residual risk, non-blocking): the harness was not executed on stable/MSRV here. Mitigated
   by the stable `--tests` check in the same job and no nightly features. Optional hardening: have
   the ci.yaml msrv job also run `cargo test --locked -p purrdf-shapes --test w3c12_conformance
   --test w3c_conformance` so a stable-only break is caught before a tag.
F3 (info): the clean-tree refusal is new in CI; a future dirtying step would abort the release
   before publishing (safe failure). Optionally print `git status --porcelain` in the refusal
   message for attribution, or move EARL generation to its own step right after the release-notes
   validation. Not required.
F4 (info): check-release-notes.py swapped `hashlib.file_digest` for `sha256(read_bytes())`
   (unrelated churn, same digest, files are small). Acceptable.

No blocking finding. All specified checks pass.

VERDICT: PASS
