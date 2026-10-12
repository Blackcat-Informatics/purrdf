<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Independent #536 plan review

**VERDICT: PASS for plan/design scope; implementation and issue acceptance remain OPEN.** Reviewed `plan.md`, `validation.md`, `issue.md` body/comment, prior-art coverage, main `dd79ef77e` source at the Shapes SCC call, core HRTB, hash dispatch parser/registry, XSD `FloatProductBackend` and both original failing tests, plus the two donor fragments. No build, transfer, compiler install, tooling or production edit was made.

The plan covers **both** production failures and the comment's extra library/integration shards. Explicit `BTreeSet<usize>` at the Shapes call resolves the named inference ambiguity without changing the shared SCC algorithm or its semantic controls. Adding `numeric` to the sole dispatch registry admits the existing real backend family while retaining unknown family/path refusal and the original numeric replay. The proposed donor scope is precise: only the one Shapes hunk of `7ee086ffd` and the retained hash registry patch; no #470 numeric branch or #531 stack work belongs here. The plan correctly requires a source-ready PR to obtain unchanged hosted MSRV/Arm checks and forbids treating native x86 success as their substitute.

No material plan omission found. The completion criteria must remain literal in execution: pin/read back hosted compiler 1.98, run the unchanged Arm `numeric:portable,numeric:neon` certified replay, verify the forced-path library and directed integration shards, keep original expectations, pass local mandatory gate and hosted checks, and use normal hooks/`ghprsq`. A failing or unavailable check remains FAILED/OPEN, not deferred. Current `validation.md` correctly records that this checkout has **no transferred source or qualification**; this verdict does not mark either production task complete or authorize merge.
