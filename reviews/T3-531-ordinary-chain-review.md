<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Ordinary FILTER/Slice chain source review

Scoped source review of the partial production route compiled in session 41147,
with the affected-source identity recorded in
`tasks/T3-531-ordinary-worklist-compile-receipt.md`. This is not runtime or depth
qualification. Owner: the current evaluator implementation owner.

## Finding R1: production prelude bypassed — HIGH, OPEN

`eval_evaluated` routes FILTER/Slice to `eval_ordinary_chain`. After the existing
NodeBoundary enter/start, that loop chooses its child state directly. It does
not run `eval_node_with`'s expanded-dictionary lookup or `shapes::eval_shape`
before descending. These are actual production consumers: `shapes/runtime.rs`
explicitly admits FILTER without custom providers and considers the built-in
dictionary, window and barrier offers. Skipping them changes which computation,
provider and admission path a request uses.

Preserve the prelude before child suspension. Factor its existing body for both
dispatch callers rather than copy its lookup or shape-selection algorithms.
Recheck the affected real shape/dictionary controls after that correction; the
compiler pass alone cannot establish behavior. The finding was sent directly
to the implementation owner before further qualification.

## Finding R2: unwind cleanup remains unfinished — OPEN

The partial loop restores its entered parent boundaries on normal completion
and Result errors. Its local `first`/`pending` frame storage currently has no
unwind owner that calls the shared nonallocating `NodeBoundary::abort`.
Complete that cleanup as part of the real driver. Copied AST/site/source owners
must remain alive through cursor/search/cap cleanup and corresponding scope/map
restoration, then release. No comparison proving a new panic regression versus
the preceding recursive path was run; this is an explicit unmet driver
requirement, not such a claim.

VM EXISTS and LATERAL child suspension and all three original optimized package
floors also remain open. No Cargo or runtime test was launched for this
read-only review. The inactive Slice body still evaluates its child as before;
no inactive zero-length shortcut defect is asserted.
