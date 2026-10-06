<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

G1 is fixed in signed/pushed commit [ad0d8e3ebcbe8410e91c41f5d7a80316ffa73ef6](https://github.com/Blackcat-Informatics/purrdf/commit/ad0d8e3ebcbe8410e91c41f5d7a80316ffa73ef6), tree `dacde4562e8f79d857ce3aa57dbe464ddaf651db`. Fresh remote branch readback matches.

The shared reader now observes provenance only when its actual segment header declares streamable layout. Both eager and evented readers use that capability. All streamable facts, including foreign predicates before/after type, still reach the same classifier. Materialized RDF projection retains its independent classification. New public controls exercise both segment orders, exact detached authorship, mandatory composite packaging, Ed repacking and actual certification.

The unchanged original 50,000-subject allocation probe removes exactly 15 allocations and 2,228,396 requested bytes in each ordinary reader mode, restoring original-base totals. Repeated subjects remove six allocations/4,380 bytes. Genuine-pack allocation totals and accepted row counts remain unchanged. The independent reviewer reproduced the result; no timing percentage is claimed.

Affected native qualification passed 1,362 cases plus 13 docs. After repairing a new assertion-style lint, the frozen public target passed all 12 groups natively and in actual Node/WebAssembly, with zero failures/ignores. Workspace all-target denied-warning clippy, frozen target clippy, strict affected docs, helper/layer/profile/generator/format checks and normal release GTS wasm build passed. The affected package run's unchanged groups are attributed to their earlier test identity; the entire changed public target was rerun on final bytes. Old full-gate/CI results retain their original identities.

Independent `tasks/G1-review.md` PASS SHA256 `ab8f3a721aa6aa8fd8e7d8cd596f2b532aafa135b721a418f3da16fa775f7771`; implementation report `0b3916c8b5047e8b4f52e397ed86592f7bc12b3b26c14a0993744fc45b01a91c`. Branch file manifest SHA256 `7b6b35e4674b44fdecbce3ead80825705b56766a9ed23fb36db4e4b10abe35d0` still matches after transport. Explicit two-file staging, separate cached whitespace check, normal signed commit/hooks and normal push all succeeded. Commit log SHA256 `ed3c5743aa488cecd653be1c6b77a274fa089489de74aeb5881ac9aae4172bdc`; push log `7c7773054867d724af7413d5486b0c56acbc0469c238873b3e41e533864c16e0`. Source/index are clean except selected Stage evidence.

Original failures and BLOCKED performance evidence remain preserved. Parent's initial evidence-manifest check used the Stage directory for worktree-relative paths and failed; the corrected worktree invocation verified all 58 entries. No missing file or failed check is represented as passing.

G3 owned SHAKE secret-state cleanup is next and remains required/open. Fresh hosted checks, current-base integration, independent Stage 2 exit and Stage 3 merge/archive/cleanup remain unverified.

