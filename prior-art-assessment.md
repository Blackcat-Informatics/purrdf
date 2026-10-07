<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Prior-art assessment

Read stagectl's captured prior-art and coverage, issues 478 and 469, and the actual production row-loop/checkpoint/block implementation. The prior arbitrary-precision work introduced ordered small blocks to bound speculative worker spend. That correctness goal must survive the scheduling repair: replacing blocks indiscriminately with far-apart chunks would reintroduce unnecessary work beyond an early ceiling.

The existing homes are parallel.rs for fork/chunk scheduling, row_checkpoint.rs for row/item ledgers and ordered replay, governor/mod.rs for counters and trip selection, and testkit's bench harness for measurements. Reuse these rather than adding a second execution or accounting path. The per-chunk mapper already provides source-order reduction and one harvest per initialized chunk; it is the appropriate existing stop-only/unlimited-work path.

The captured link crawl covers five issues within two hops, fourteen related forge items, and the last 200 merge records. Its twelve `none` defect-class trailers are not a measured recurrence of this scheduling defect; recurrence of this exact defect remains unknown. Historical timing tables characterize a prior run and do not qualify the current candidate.

Existing governed_eval row-loop lanes and numeric_eval grouped exact lanes provide the requested A/B workload with native results verified outside timing. Focused governor/numeric tests and frozen profile vectors provide the correctness controls. The optional-filter fork assertion must be evaluated against the CLI's actual metered-base governor configuration, not a library fuel-only example. The assigned work owns current defects it changes or depends on; unrelated archived branches remain preserved.
