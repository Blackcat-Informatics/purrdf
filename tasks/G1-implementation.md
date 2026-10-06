<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# G1 reader provenance implementation and qualification

Status: SUCCESS

Assigned G1 implementation and local qualification are complete. Source is frozen for independent review. This report does not close Stage 2, G3, integration, hosted acceptance, or parent-owned signed commit/push/publication.

## Authority and exact input

Issue458, PR464; branch `paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519`.
Worktree: /home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519.
Stage paths were resolved with stagectl; existing `open.json` owns setup, with no additional worktree creation.
HEAD remains `52988974f2d11a40214281648983f9145af7a3fb`, tree `43b0817e30a3ad110cd013796a064c177a27e067`.
Observed origin/main remains `090b14bb8c00e2504078cb694d278ee2d614443a`; no synchronization or combined-tree execution occurred.
Original implementation base is `ce3c07192aba1e36666062c00f958670a827cfb5`.
Immutable plan SHA256 remains `dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a`, including its historical footer.

Read applicable AGENTS (root inventory found no nested instructions), root .baseline/.goals, Stage2/stagectl and quality/validation/delegation/no-deferrals, approved plan, published remediation-plan, gap analysis, complete S2 performance review and material T5/T6 correction/publication history. The accepted G1 finding is reader-local unused collection, not an authorization to filter materialized provenance or change G3 cryptographic storage. No child delegation, index mutation, commit/push/forge transport, memory, model or lifecycle action occurred.

## Shipping change and real wiring

Only these two tracked files differ from HEAD (80 insertions, 9 deletions):

| File | Frozen SHA256 |
|---|---|
| crates/gts/src/reader.rs | `14878fe996f2fa28ced78a6a723291bd8208084eadcbfa06ec2daf58fe9be7a1` |
| crates/rdf/tests/gts_composite_compaction.rs | `1a5280a077cbf45c0e1fca9e789d9755137abc1dc240b0eec8e81d04329841c7` |

Folder carries the actual current segment header's streamable capability. Shared h_quads calls ProvenanceSubjects::observe only when that capability can be consumed; eager and evented signature-role construction use the same field. Every streamable quad is still observed, including foreign facts before or after rdf:type. Each streaming segment retains its fresh map/header lifecycle; snapshot quads re-enter this same handler. Invalid headers and term-position diagnostics retain their existing paths.

ProvenanceSubjects, packaging_role, materialized from_graph/content_projection, UTC parsing and the closed mandatory shape remain their existing single implementations. There is no key/type-only filtering, new parser, public API, dependency, feature, signer/provider fallback or change to emitted goldens/vectors.

The existing public test adds foreign-after-type alongside foreign-before-type. A new registered group joins a genuine pack and an ordinary segment in both orders; the ordinary segment deliberately contains a complete valid Compaction shape. Its index stays authored, while independent materialized RDF projection still recognizes the same shape. Both readers agree on signatures and exactly one packaging index. Actual compact_and_certify composite packaging, Ed repacking, detached byte pairs and verify_compaction all-six outcomes are asserted in both orders. The existing genuine-pack→fresh-streamable-history group separately checks state reset without changing header capability.

## Frozen identities

- Complete tracked working-source manifest, 16,045 paths: `raw/G1-frozen-source-manifest.sha256`, SHA256 `16a33d0a750f78972c6434cab78aa87af08d7ef1e6b83de46cec44bc0cca23eb`; actual full check exited0, `raw/G1-frozen-source-check.log`.
- Whole 52-file branch manifest against original implementation base: `raw/G1-frozen-branch-files.sha256`, SHA256 `7b6b35e4674b44fdecbce3ead80825705b56766a9ed23fb36db4e4b10abe35d0`.
- G1-only binary patch against HEAD: `raw/G1-frozen-source.patch`, SHA256 `c7c53213fb172987729417d26417cee406658ca811f41bbd8b32cd5d090ceb56`.
- Whole branch binary patch: `raw/G1-frozen-whole-branch.patch`, SHA256 `537303140d31c3e6b67efdc0d8bd23269298f320a3693da4c54008716187cc96`.
- Changed-file hashes: `raw/G1-frozen-changed-files.sha256`.
- Index remains unchanged/empty: `raw/G1-index.patch`; git diff --cached --exit-code and git diff --check both0. Main, siblings, governed vectors and .deficiencies are unchanged; no emergency entries exist.
- Build/policy/plan identities: `raw/G1-policy-and-build-inputs.sha256`, SHA256 `08e46666fedaf74f99c0a3c0fab9f1db454a86571ce3581216abb3a64fbfee4b`.
- Exact command statuses/input phases: `raw/G1-command-statuses.json`, SHA256 `394fe31bcf433ae637bf2470ae0246483c866e9c8c0d945b003583b82f886914`.

## Unchanged allocation replay

Executed from the worktree, locked/offline, with CARGO_BUILD_JOBS=2:

`cargo test --locked --offline --manifest-path <Stage>/raw/S2-performance-head-probe/Cargo.toml --test performance -- head <Stage>/raw/S2-performance-corpus`

Exit0. Common probe, manifest, lock and corpus were not edited or regenerated. `raw/G1-performance-inputs.sha256` SHA256 `4be8e0fe41a1bb34d1441234d60a337329841f719093e650fd68954369ea02b3` binds all six original inputs; frozen recheck0. Probe SHA256 `f9646a8549b8ed5f27e5bdbe822c5885771fd1555f87447cf03c4424a6089f87`.
Manifest SHA `7775cc0784d08507cdab6a2a7387efa7699ec395da2115118009f04f492d608d`; lock SHA `7dc7b6f85ec271cb32d396560a85f65d2fb378e1477ee5bee52b61b4449f0c5e`.
Actual corpus is `raw/S2-performance-corpus`, with unique/repeated/pack SHA256 respectively `04dadb1222c4e8d6ca815448f982322e7e6a91287012ad7cec00f4da18a51789`, `304769b8d924deeba126f62d51388b865c6ab70ddfd96bc0dfa6d6e365aaed8b`, `8070e9d66478af989c3cabf7cb23bd4cccb6b5cb77f3025ba6a8b9889f5c03e7`.

| Input/caller | Original head allocations / requested bytes | Corrected allocations / requested bytes | Disposition |
|---|---:|---:|---|
| unique / eager | 700328 / 117477991 | 700313 / 115249595 | exact original-base metrics; -15 / -2228396 |
| unique / evented | 850408 / 118429105 | 850393 / 116200709 | exact original-base metrics; -15 / -2228396 |
| repeated / eager | 101474 / 40982023 | 101468 / 40977643 | exact original-base metrics; -6 / -4380 |
| repeated / evented | 101854 / 35756337 | 101848 / 35751957 | exact original-base metrics; -6 / -4380 |
| genuine pack / eager | 700732 / 117515312 | 700732 / 117515312 | original-head metrics preserved |
| genuine pack / evented | 850928 / 118471657 | 850928 / 118471657 | original-head metrics preserved |

Ordinary rows50000 and genuine-pack rows50004 remain exact, with no diagnostics. Retained bytes after drop remain0 for all six. Unique eager peak is47701413 versus original head48447725; evented peak43302396 is unchanged. These are requested layout/working-demand metrics, not RSS. Timing samples are observations only, taken while other compilation could be active; no percentage/latency claim or gate is inferred. The probe itself establishes row/allocation behavior; packaging authenticity is established by the production consumer suite below.

Log `raw/G1-performance-head-corrected.log` SHA256 `087b8151e46f3737a72a644eb70c1d597f52c24ca0de2cfd97a5ab322cccf917`.
Actual executed probe SHA256 `d49a9ea08f879cbacc82749c092781681a2deb3abec4bee9ca11034f0bd331b7`, path recorded in frozen runtime manifest. Original reviewer base/head logs and all S2/T6 inputs are preserved.

## Checks, profiles and actual runtime

Current rustc1.100.0-nightly commit `4b6d04e706108ccfeafe2547fbe857dfe8972bad`, LLVM23.1.1; Cargo1.100.0-nightly7941be6fb. Native target x86_64-unknown-linux-gnu; test/dev opt3, debug assertions and overflow checks ON, test debug0. Profile hygiene observed974 units across the two normal gate graphs. Actual wasm uses wasm32-unknown-unknown, Node26.10.0, wasm-bindgen0.2.125 and the unchanged repository scripts/wasm-test-runner.sh. Release uses normal opt3/fatLTO/one codegen unit/strip policy. No SKIP, weakening, kill, service action or gate bypass occurred. All waits were at most60seconds.

| Check | Actual result / log |
|---|---|
| CARGO_BUILD_JOBS=2 cargo test --locked --offline -p purrdf-gts -p purrdf-rdf | exit0; native1362/86 groups + docs13/3 groups, failed0/ignored0; `raw/G1-affected-native.log` SHA `c717aab1767b4e73489851ab5ebc73fe57eabef5032fcc516006ee172f46ece1` |
| Frozen public native target gts_composite_compaction | exit0; 12/12, failed0/ignored0/filtered0; `raw/G1-public-native-frozen.log` SHA `9f8767e6a07db58aa480d83c215389fa57d78af573af4007c1b081f27477d4d7` |
| Frozen same target with actual wasm runner | exit0; 12/12 execute in Node, failed0/ignored0/filtered0; `raw/G1-public-wasm-frozen.log` SHA `152729f00e097b9bd43320967aedc6e667a1ef2b520c054b8b4b77edbe7c7efc` |
| cargo clippy --workspace --all-targets --locked --offline -- -D warnings | corrected assertion input exit0; `raw/G1-clippy-final.log` SHA `f99fd6294ab3bc8d6c11440d89aa3b1aa8b165a2f8f18f71276225f80364f038` |
| Frozen cargo clippy --locked --offline -p purrdf-rdf --test gts_composite_compaction -- -D warnings | exit0; `raw/G1-clippy-frozen.log` SHA `6bca73980767af312110639aa57e32158a666f73e8fbd0beb9ad2db28d062a64` |
| RUSTDOCFLAGS='-D warnings' cargo doc --locked --offline -p purrdf-gts -p purrdf-rdf --no-deps | exit0; `raw/G1-docs.log`; shipping source identical to frozen |
| cargo build --locked --offline --release --target wasm32-unknown-unknown -p purrdf-gts --lib | exit0; `raw/G1-release-wasm.log`; shipping source identical to frozen |
| make helpers-hygiene layer-hygiene build-profile-hygiene; then corrected make helpers-hygiene | both0; `raw/G1-hygiene.log`, `raw/G1-helpers-final.log`; final delta only whitespace |
| bash scripts/check-generated.sh | exit0; `raw/G1-generated.log`; all projections unchanged, no regeneration needed |
| Frozen cargo fmt --all --check, git diff --check, full source/probe hash rechecks | each0; raw format/source/input check logs |

Every public group executes actual eager/read_to_sink paths, source/pack/repack and certify/verify paths, including positive/negative algorithm/key refusals, reserved-class and foreign-before/after-type shapes, valid6/invalid5 timestamp neighbors, producer preflight before entropy, head/literal/cardinality controls, malformed/ambiguous carried COSE and mandatory authentic packaging. No assertion/golden was weakened. All affected native reader frame/snapshot/adapter suites also executed through the full two-package selection; umbrella/validate/CLI/C/WASM/Python and other dependency consumers received warning-free all-target compiler coverage.

Frozen runtime manifest `raw/G1-frozen-runtime-artifacts.sha256` SHA256 `a3f6f0cb4fa10a389db6b1638e900683cb85680561af0702fb3ac43b0cb28df5` records the exact performance executable and final public native/wasm artifacts:
native SHA `d1e506c2570c0aa275b4590a7c4098bbb33d075d2ddb97934f535ecfc181a4e2`;
wasm SHA `d337206256ec76b47742bf7dc9f180b4d19563a6ed65ec5d98aefb8a153782f8`.
Broad native86 artifact identities were captured before further edits in `raw/G1-initial-affected-native-artifacts.sha256`, SHA `389af4f9c4b37893ff1874d802d83d1d00bcd6e70aa97d4d8249eb8b30840858`.
Release wasm GTS rlib SHA `6bdf4a12894e01b95fb15e5bdd284dd201e828bdea8d1bedfccb4656b0eee911`, exact path in `raw/G1-release-artifacts.sha256` SHA `cbedf43c26ef7ed39595d3d894c107631827d7530513cfb6303bd2e1e1f22f7f`.

## Development history, invalidation and bounded reuse

Initial clippy exited101 solely for the new test's assert!(quads.is_empty()) style. After all running checks terminated normally, it became assert_eq!(quads, []), preserving the condition and improving failure output. The subsequent formatter check exited1 for a line break; normal cargo fmt joined that line. Original failure logs, initial source/manifest/runtime and each successful intermediate run remain untouched. The first metadata-path extraction also encountered directory paths; the corrected Running-only extraction is retained separately and supplies the executable manifests above.

Reader SHA never changed after the implementation. Initial test SHA was `a8164a4c2e8a5818596dd2d09bfe284d94d905a8efd3b2d31defcde5ae3b1d18`; assertion-corrected SHA `6aad299220bdfb2e83f8a8762af290d82cbe7942716d9e01b20df06e2a86f309`; frozen SHA appears above. `raw/G1-last-format-delta.patch` and the format failure show the final whitespace-only delta. Both final native and actual wasm public runs and target clippy executed on frozen bytes.

The full affected-package log is attributed to its initial test identity, not relabeled a frozen full rerun. Its unchanged reader/shipping inputs, 85 other native groups and13 docs remain qualifying; the changed12-group public suite was rerun on frozen bytes. The warning-free workspace compilation and corrected helper census remain qualifying across the final whitespace-only test delta; final target clippy and formatting also ran on frozen bytes. The unchanged shipping-source rustdoc/release and generator/probe/configuration checks remain applicable.

Old Stage1 fullgate and initial hosted CI37487670367 remain proofs of their original inputs. They are not executions at this working identity or current-base integration. All new reader-dependent runtime/compiler checks are above. Unchanged SHAKE/MLDSA/COSE construction/known-answer/corpus and unrelated workspace matrix behavior retain original Stage1/CI provenance; the two-package run additionally executed its native crypto tests. No discretionary full make check, whole-workspace runtime suite or 31-crate wasm rebuild was added: this private reader-local capability change is bounded by the shared handler, two package runtime closure, all-target consumer compilation and actual public wasm/release GTS paths. No timing/constant-time, external shared-vector, hosted-new-head or current-base claims are inferred.

## Deferral/laundering scan and terminal scope

`raw/G1-frozen-diff-scan.txt`: zero keyword/laundering hits in the G1 patch.
`raw/G1-commit-message-scan.txt`: zero hits in current commit messages.
`raw/G1-complete-input-scan.txt`: six complete-branch hits, zero in approved plan:
- Pure-message ML-DSA docs distinguish HashML-DSA, a different algorithm outside the approved pure-message construction; no required issue implementation is deferred.
- Two ordering comments use “later” for subsequent signed history/index calls, not future engineering.
- “No OpenSSL source” accidentally matches the broad no-op pattern; it accurately states first-party implementation provenance.
- Two “later” fixture values exercise actual successive history.
These are unchanged semantic facts/fixtures, inspected at actual source/context. No TODO/FIXME/stub, borrowed baseline excuse, assertion reduction or new open engineering item exists in G1.

No G1 blocker remains. Independent G1 review and parent normal signed transport/publications are the next authorized sequence. G3 remains its separately owned accepted finding; this report makes no claim to fix it. Complete feedback/current-base integration/Stage2 exit/Stage3 completion remain parent-owned. Source/index/plan/governed evidence are frozen as above.
