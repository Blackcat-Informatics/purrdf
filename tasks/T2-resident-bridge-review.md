<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Task 2 resident bridge: independent source review

**Source reviewed:** `065790157` plus the three uncommitted changes in `crates/sparql-eval/src/property_fn.rs`, `crates/hnsw/src/relation.rs`, and `crates/retrieval/tests/real_producers.rs`. This is a bounded source verdict, not a runtime or issue-completion verdict.

**VERDICT: PASS (source only).** The exported `resident_property_open!` generates the original resident `PropertyFunction::open` bridge body once: `open_admitted(args, ceiling, WorkspaceCapability::resident()).and_then(WorkspaceCursor::into_resident)`. The macro uses `$crate` for the public evaluator types, accepts method attributes (including the HNSW method documentation), and adds no new runtime helper or allocation. Both call sites implement `open_admitted` explicitly; this matters because the trait's default `open_admitted` calls `open` and would recurse with this macro.

The HNSW invocation replaces only its identical handwritten bridge and retains the existing `open_admitted` implementation and method documentation. The retrieval fixture uses the same bridge and retains its admitted cursor adaptation: the inner admitted cursor remains under the supplied workspace, the optional wrapper is constructed with `WorkspaceCursor::new`, and resident extraction occurs at the bridge. I found no source-level change to arguments, error propagation, cursor ownership, or query inputs from the bridge centralization.

**Qualification boundary:** The unchanged helper census, strict all-target evaluator/HNSW/retrieval checks, original HNSW library tests, and original real-producer controls were running at review time; their terminal results are not credited here. The full gate and hosted MSRV/architecture checks remain open. This review does not discharge those requirements or claim the #536 issue complete.
