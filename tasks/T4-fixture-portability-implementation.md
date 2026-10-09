# Native profile fixture portability correction

Status: SOURCE SETTLED; compilation/runtime/hosted acceptance UNRUN. Root-authorized source-only correction of the retained-case fixture placement. 454 retains the local build lane; no Cargo/build/test/controller/process/commit/push/dispatch was started. Existing campaign outputs and sibling worktrees were not changed.

## Exact owned delta

- `crates/rdf-capi/tests/support/hosted.rs`: exactly four existing retained-case caller holders now use `purrdf_testkit::TempDir::new_in("/opt/purrdf-native-profile-tests")`. All four safety tests and their assertions remain intact. `retained_case`, production `profile::validate_request`, `completed_pair`, receipt validation, reclaim/canonical/symlink checks and measured command selection are unchanged.
- Same file: the existing typed YAML workflow contract requires exactly one normal fixture setup before the existing shard execution with `matrix.shard == 'integration-2'`. Its existing serial campaign contract requires one unconditional fixture setup before the shared setup endpoint, after the observed `setup-start.txt` timestamp in the same shell block. The existing workflow test adds typed-YAML missing/conditional/late mutations for both jobs, plus missing observed-start refusal; no new test function or alternate validator. Existing suite inventory remains35 by source inspection, not runtime discovery.
- `.github/workflows/ci.yaml`: one narrow integration-2 setup step; optional campaign provisions the same parent after setup-start and before resources/controller/measured execution. Only the parent is created/chowned, with no recursive ownership change of fixture children. Cold/warm commands, twelve cases, cache/proof/upload/reclamation coverage remain unchanged.
- `CONTRIBUTING.md`: Development explains the writable owned `/opt` parent requirement, exact one-time provisioning commands, exclusive child cleanup and actionable refusal. No issue/Stage/process reference.

The ordinary GitHub target scratch remains beneath checkout; fixture filesystem now independently lives at its admitted physical location. Tests never invoke sudo or fall back. Existing testkit uniqueness/drop owns only their fresh child. Optional setup cost includes parent provisioning within its observed setup interval. Four test-constructor substitutions are intentionally narrow; the other Cargo-target scratch fixtures remain unchanged.

## Actual static checks

`rustfmt --edition 2024 crates/rdf-capi/tests/support/hosted.rs` exited0. `git diff --check` exited0. Diff review confirmed exactly these three owned shipping paths and no edit of production profile.rs. Root authorized staging only those paths. No compilation or executed fixture pass is claimed.

## Qualification prerequisites and pending commands

Before execution, an administrator-owned `/opt/purrdf-native-profile-tests` parent must be writable by the admitted test user; no world-writable permission or recursive chown. CI provisions it as specified, local raw-SDK/8-job qualification must verify real ownership and use **Cargo-created target/build directories outside `/opt`** to demonstrate the formerly masked valid neighbour. Preserve managed-artifact drift precautions: select controller from real Cargo JSON and freeze it outside the managed slot before later Stage commands.

Once root admits the sole lane, run full existing `cargo test --locked -p purrdf-capi --test native_profile --jobs 8` with `RUST_TEST_THREADS=1`, actual discovery/count, strict `cargo clippy --locked -p purrdf-capi --all-targets --jobs 8 -- -D warnings`, unique controller JSON rebuild, and existing owning fmt/workflow/shard/profile/parity/helpers/glossary/non-Rust/generated hygiene matrix. Inspect that all four safety fixtures pass with non-/opt Cargo scratch, unique owned `/opt` fixture children are removed, and parent/sibling sentinels survive; existing negative neighbours remain real production-validator refusals. Do not broaden to a workspace suite solely for this repair. Source-only YAML fixture checks and runtime compilation remain UNRUN until admission.

Fresh hosted acceptance requires the normally published corrected source and an entire new attributed twelve-case cold/warm campaign with strict comparison; neither past subset success nor these35 focused controls establish numerical/full-plan completion. Initial hosted failure and all previous attempts remain preserved.
