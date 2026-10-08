# Project outer bulk source review

2026-10-08. **Source verdict: PASS. Cost verdict: PENDING fresh emission; the preceding 4828 emitted cost remains FAILED.**

## Exact scope

Independently read the modifier-only delta from tree `4828a699a924d840b1c3272853ee7bca3c65943e` to staged tree `57906333c273e8e7dde7a3877e44ee6919cc7983`. The change replaces the outer `for`/`Vec::push` loop with one `rows.extend(seq.rows.iter().map(...))`. It retains `Vec::with_capacity(seq.rows.len())`, source-column mapping, the once-per-row immutable `row.as_slice()` view, the existing inner `SmallVec::extend` projection, schema/order/unbound cells and caller-owned Lift construction/finish. No source, index, build, test, probe or forge mutation was performed by this review.

## Source and SDK adjudication

The actual installed SDK source under `/opt/stage-rust/lib/rustlib/src/rust/library` resolves this exact iterator to the trusted-length specialization: slice iteration has exact length, `Map` preserves `TrustedLen`, and `alloc/src/vec/spec_extend.rs` routes `SpecExtend` for that bound into `Vec::extend_trusted`. `alloc/src/vec/mod.rs:4164` reserves once before traversal, fills reserved slots and advances a local `SetLenOnDrop` length. Its guard publishes the initialized length on completion or unwinding, so already constructed projected rows are dropped correctly if subsequent projection panics. This reuses the standard-library bulk home rather than introducing a private unsafe implementation, guard, second loop or cardinality-changing adapter.

Keeping the original explicit capacity request preserves its allocation strategy, including small counts. Existing row borrowing and indexed access retain their bounds/panic semantics; the source-row view and inner bulk fill are byte-unchanged. The outer iterator visits the same rows in the same order and returns one projected row for each. Lift is outside the unchanged pure helper, so construction, error/absorb/finish order and Drop behavior remain covered by the preceding source review.

This source repair directly addresses the observed 4828 outer per-row capacity comparison/grow fallback and repeated published Vec length store. It is a coherent safe repair, but specialization source does not prove emitted erasure: the previous original `map/collect` generated a live outlined 374-LLVM-instruction standard-library frame. Fresh emission must independently inspect the complete caller and any living `SpecExtend`/`extend_trusted` callee; moving work into another frame cannot satisfy the requirement.

## Qualification applicability and next proof

The root-admitted existing Project/contextual/prepared batch (35 controls) and strict eval clippy/fmt are appropriate for this exact outer-loop change. Prior 5286 governor controls (36), core SmallVec controls (53), worker controls (45), docs and wheel qualification remain applicable to their unchanged source/behavior scope; this review does not claim their execution at 5790 or request a blanket replay.

Fresh candidate native and host emissions against the strictly admitted immutable 85204 baseline are still required. The next independent complete twelve-row cost audit must verify outer bulk SSA/final publication and capacity work, exact inner/source-view and Lift behavior, complete ordinary shared SmallVec caller impact, worker mint/checkpoint erasure, GRAPH/Group/dedup, numeric/Application/control paths, and host ABI/capture/detach/drop/materialization. Unchanged arguments may be reused only with actual source/artifact identity and emitted applicability. Exact 32-record equality, annotations, emitted counts and smaller caller frames alone cannot establish cost acceptance.

The full settled 29326 body review and its outer-work failure remain historical evidence in `portfolio-final-project-view-lift-cost-body-review.md`; no failure is relabeled by this source verdict. No additional source defect was found in the bounded 4828→5790 delta.
