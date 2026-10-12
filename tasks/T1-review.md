<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# T1 independent source and wiring review

**VERDICT: PASS for Task 1 source design and wiring by inspection; execution acceptance remains OPEN.** Reviewed the actual two-file dirty diff on main `dd79ef77e`, the source homes and existing consumers. `git diff --check` produced no error. No build, source edit, transport or tooling change was made by this reviewer. Runtime, pinned MSRV and Arm acceptance remain unrun in this checkout.

The Shapes change passes the already declared adjacency element `BTreeSet<usize>` to `scc_component_index<E>` explicitly. Its `for<'e> &'e E: IntoIterator<Item=&'e usize>` bound, input graph, SCC implementation, resource limit, cycle/chain traversal and result use are unchanged. This is the sole Shapes hunk and fits the E0275 inference failure.

The hash change adds `numeric` to the single sorted `FAMILIES` registry and documents `FloatProductBackend` as its owner. The parser still rejects unknown families and paths before availability checks; `assert_required_available::<FloatProductBackend>` in both original XSD tests still reads the actual backend list, detects availability and replays the original endpoint bits. No backend choice, NEON implementation, test vectors, digest, or architecture predicate changed. Registering the family addresses the observed failure point without making an unavailable path pass.

**Resolved finding:** The first review found that the new hash test labelled `numeric:avx512` unknown although `Avx512` is a real numeric backend. The current diff now asks the parser to refuse `numeric:bogus` and checks the matching `bogus` diagnostic. That name is genuinely absent from `FloatProductBackend::ALL`; the positive portable/NEON parser control and existing generic refusal tests remain. The source-review blocker is resolved. `git diff --check` still passes; the corrected test subsequently passed 1/1, terminal 0 (`native-hash-registry.log`).

The source design has no other material finding from this review. The remaining native controls and affected strict checks are classified in `T2-review.md`. Task 3 unchanged hosted MSRV 1.98, Arm numeric portable/NEON, affected hosted shards, full gate and integration remain **OPEN**. This source review is not issue acceptance.
