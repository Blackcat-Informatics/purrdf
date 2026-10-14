<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Retrieval and Geo test-caller corrections

The original full gate stopped on four real-producer cursor API errors and one
Geo metric exact-integer fixture. Source review of the proposed retrieval diff
finds no material wiring gap: the inner and wrapping cursor now retain their
WorkspaceCursor owners; the wrapper uses the actual admitted factory, replacing
its duplicate manual box/grant. The resident seam explicitly extracts only a
resident cursor. Existing counters, attachment evidence, refusal behavior, inputs
and assertions are unchanged. Capacity-only builder changes from the other
checkout were not copied.

One correction is required in the initial Geo diff: an unsuffixed large integer
followed by `.into()` no longer has its former i128 field context. Exact Integer
also implements From<i32>, allowing default integer inference to choose i32.
Use the original number with an explicit i128 source type (or from_i128) before
compilation. Preserve the MXCSR refusal-before-conversion control and original
Float neighbor. No build was run by this reviewer; affected strict/runtime checks
and the complete gate remain required. This scoped review is not full delivery
approval.

Source readback: the fixture now uses `9_007_199_254_740_993_i128.into()`,
resolving the inference concern while preserving the exact original value.
Scoped source verdict PASS; strict/runtime execution remains pending.
