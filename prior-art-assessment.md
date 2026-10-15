<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# #492 prior-art assessment — 2026-10-13

VERDICT: sufficient bounded intake, not a recurrence or implementation-completeness claim. `prior-art.md` captures the full issue and one approved comment, a two-hop four-issue link crawl, fourteen related forge items, zero commits/branches/PRs citing #492, and a 200-commit defect-trailer tally. The recurrence tally is confined to that window. It does not prove that no earlier graph-free attempt ever existed.

The Stage brief could not resolve Katamari ADR-0038 inside this repository. The approved comment supplies its explicit one-graph constraint and the external ADR location; that consumer law motivates this issue but does not change PurRDF's additive quad semantics. #383 and #292 provide existing default-projection and flat-view canonical paths, not physical triple storage. #471 makes empty named graph declarations significant; conversion must inspect declarations as well as rows. PR #544/#490/#495 provides the new dictionary and operational-view interfaces at the selected published dependency head, but remains a separate hosted/integration contract. #479's join lesson supports actual predicate partition probes and variable-predicate controls; it does not qualify the proposed arity-three path.

Existing original homes are concrete enough to start the complete implementation: `ir/builder.rs` owns term mint/rollback/validation, `ir/dataset.rs` owns frozen arena/reverse lookup and quad tables, `dataset_view.rs` owns operational checked reads, `ir/segmented/session.rs` demonstrates a session/status split, `datalog/{plan,store,seminaive,chase,proof}.rs` own static positions and execution, and `text/{index,ranking}.rs` own graph-bearing corpus identities. No alternative algorithm or second rule engine is warranted by the captured prior art.
