<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# G1 independent performance and caller re-review

VERDICT: PASS

The accepted MEDIUM G1 / S2-P1 reader allocation finding is resolved on the frozen identity below. Required G1 findings remaining: none. This verdict is limited to G1; G3, current-base integration, hosted acceptance and Stage2 completion remain separate.

## Exact reviewed identity

Issue458 / PR464; branch `paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519`.
Worktree `/home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519`.
HEAD `52988974f2d11a40214281648983f9145af7a3fb`; tree `43b0817e30a3ad110cd013796a064c177a27e067`, plus exactly two uncommitted tracked files:

| File | SHA256 |
|---|---|
| crates/gts/src/reader.rs | `14878fe996f2fa28ced78a6a723291bd8208084eadcbfa06ec2daf58fe9be7a1` |
| crates/rdf/tests/gts_composite_compaction.rs | `1a5280a077cbf45c0e1fca9e789d9755137abc1dc240b0eec8e81d04329841c7` |

Approved plan SHA256 `dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a`.
G1 patch `raw/G1-frozen-source.patch` SHA256 `c7c53213fb172987729417d26417cee406658ca811f41bbd8b32cd5d090ceb56`; independently regenerated `git diff --binary --no-ext-diff` has that exact hash.
Complete tracked-source manifest `raw/G1-frozen-source-manifest.sha256` SHA256 `16a33d0a750f78972c6434cab78aa87af08d7ef1e6b83de46cec44bc0cca23eb`.
Whole branch manifest against captured original base `ce3c07192aba1e36666062c00f958670a827cfb5`: `raw/G1-frozen-branch-files.sha256` SHA256 `7b6b35e4674b44fdecbce3ead80825705b56766a9ed23fb36db4e4b10abe35d0`.
Terminal implementation report `tasks/G1-implementation.md` SHA256 `0b3916c8b5047e8b4f52e397ed86592f7bc12b3b26c14a0993744fc45b01a91c`.
Evidence manifest `tasks/G1-evidence-manifest.sha256` SHA256 `2f08a53d0e7d00a0d8ff7c043c688e4cfbed4170b9b55fd911b43f471eb8c7ce`.

I independently checked both source manifests, evidence manifest and frozen runtime artifact manifest from the worktree: every check exited 0. HEAD/tree, source hashes, plan and patch identities match the terminal report. `git diff --check` passes. This is the assigned existing Stage worktree; no reviewer worktree, branch, source/index/forge mutation, commit, push, delegation or private-memory action occurred.

## Actual production caller inspection

The shared Folder::h_quads now observes provenance only when the current segment can use that state for packaging. All three actual Folder constructions derive the private capability from their own segment header's exact `layout == "streamable"` condition. Eager and evented index signature-role construction consume that same capability. This removes the unused ordinary-segment subject map without a second classifier or parser.

Every validated streamable quad still reaches the original ProvenanceSubjects::observe, including foreign predicates before or after the reserved type and mandatory facts. There is no candidate/type-only prefilter that could discard poisoning facts. Term validation, sink delivery and materialized quad insertion remain in the same shared handler. A fresh ActiveStreamingSegment still owns a fresh header/map, and eager segments construct fresh provenance; capability/state cannot leak across either concatenation order.

The sole classifier, closed shape, valid UTC parsing, mandatory agent/timestamp/source-head rules, packaging_role and materialized from_graph/content_projection are unchanged. Materialized RDF projection remains independent of reader-local layout capability. The new public reset case deliberately includes a complete compaction shape in an ordinary segment: its index remains authored, materialized projection still recognizes the shape, and both readers agree on exactly one packaging index in both segment orders. Composite packaging, Ed repacking, exact detached signature pairs and real verify_compaction outcomes are asserted. The expanded existing case exercises foreign facts before and after the type/mandatory fields. Existing UTC positive/negative, mandatory packaging, carried COSE and authored-history neighbors remain in the executed suite.

## Independent unchanged probe replay

I executed the original probe unchanged, with CARGO_BUILD_JOBS=2, locked/offline, from the worktree:

`cargo test --locked --offline --manifest-path <Stage>/raw/S2-performance-head-probe/Cargo.toml --test performance -- head <Stage>/raw/S2-performance-corpus`

Exit 0; `raw/G1-review-performance.log` SHA256 `586230b9488dad4968a02c0205b9039cafa4de2ecaa4670ddef067d11dfd15cb`.
Probe source SHA256 remains `f9646a8549b8ed5f27e5bdbe822c5885771fd1555f87447cf03c4424a6089f87`; corpus, manifest and lock are the original inputs. The executed probe artifact SHA256 is `d49a9ea08f879cbacc82749c092781681a2deb3abec4bee9ca11034f0bd331b7`, independently verified from `raw/G1-frozen-runtime-artifacts.sha256`.

| Input / caller | Corrected allocations / requested bytes | Resolution |
|---|---:|---|
| ordinary unique / eager | 700313 / 115249595 | exact captured base; removes 15 / 2228396 |
| ordinary unique / evented | 850393 / 116200709 | exact captured base; removes 15 / 2228396 |
| ordinary repeated / eager | 101468 / 40977643 | exact captured base; removes 6 / 4380 |
| ordinary repeated / evented | 101848 / 35751957 | exact captured base; removes 6 / 4380 |
| genuine pack / eager | 700732 / 117515312 | exact original head |
| genuine pack / evented | 850928 / 118471657 | exact original head |

Ordinary row counts remain 50000 and genuine pack 50004, with no diagnostics; retained bytes after drop are 0. Unique eager peak returns to 47701413, while evented peak remains 43302396. These are measured allocation/requested-byte/working-demand results, not RSS or a general timing guarantee. Shared-host timing samples support no percentage regression or speedup claim.

The original BLOCKED performance report `reviews/S2-performance.md` SHA256 `9b4ee49c863a103110b84cd9835fa8438d95a1a0ab338ddc44d7b953403401d2` and original base/head evidence remain preserved. This report closes its specific finding rather than replacing that history.

## Executed qualification and bounded reuse

I read actual commands/statuses and terminal logs, not just the SUCCESS statement. Frozen public native `raw/G1-public-native-frozen.log` SHA256 `9f8767e6a07db58aa480d83c215389fa57d78af573af4007c1b081f27477d4d7` and actual Node WebAssembly `raw/G1-public-wasm-frozen.log` SHA256 `152729f00e097b9bd43320967aedc6e667a1ef2b520c054b8b4b77edbe7c7efc` each execute all 12 groups with zero failures/ignored/filtered. Both include foreign order, capability reset and UTC neighbors through real eager/evented, packaging, RDF certification and repack callers.

Frozen native artifact SHA256 `d1e506c2570c0aa275b4590a7c4098bbb33d075d2ddb97934f535ecfc181a4e2`; frozen wasm artifact SHA256 `d337206256ec76b47742bf7dc9f180b4d19563a6ed65ec5d98aefb8a153782f8`. Artifact hash readback passed. Compiler is rustc 1.100.0-nightly `4b6d04e706108ccfeafe2547fbe857dfe8972bad`, LLVM 23.1.1, native x86_64-unknown-linux-gnu and wasm32-unknown-unknown; normal opt3 runtime profiles retain debug assertions and overflow checks. Actual wasm runs through the repository runner in Node 26.10.0 with wasm-bindgen 0.2.125.

The affected GTS/RDF native selection reports 1362 native cases and 13 docs, zero failures/ignored; independent log aggregation confirms 1375 across 89 groups. Its initial test identity is explicitly retained. The later assertion correction preserves the same emptiness condition, and the final whitespace delta is directly recorded; the entire changed 12-group target was rerun on frozen bytes. Unchanged shipping reader and other 85 native groups/13 docs retain attributable qualification.

Corrected workspace all-target clippy with `-D warnings` passes, including C/Python/WASM/other production consumer compilation; frozen target clippy passes separately. Helper/layer/build-profile checks, strict rustdoc, generated projection checks, release GTS wasm build, final formatting and complete hash readbacks pass. Their unchanged shipping inputs and final whitespace-only test delta justify the implementation report's bounded reuse. Initial clippy exit 101 and formatting exit 1 are preserved failures with repaired causes, not relabeled passes or bypassed checks.

No extra whole-workspace runtime gate, all-release-crate wasm rebuild, current-base combined-tree execution, hosted new-head CI or binding runtime run was performed by this reviewer. Unchanged cryptographic/conformance evidence keeps its original identity; this G1 review makes no SHAKE storage cleanup, constant-time or G3 closure claim. Normal parent-owned signed transport can proceed for this accepted G1 unit.
