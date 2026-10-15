# #492 shared carrier declarations — source readback, 2026-10-13

**Scoped source verdict: PASS.** This read-only review covers only the duplicate consolidation; owner batch 17768 strict/runtime was live and no test result is inferred here.

`ir::dataset::source_location_method` is one declaration for the original `RdfDataset::location_of` and the compact `TripleDataset::location_of`. Both expansions perform the same binary search over the resident handle-sorted `locations` table and return a borrow of the matched location. The triple call retains its original `#[must_use]` and sparse frozen-ordinal contract. There is no extra lookup body, allocation, or dynamic dispatch in this delta.

`dataset_view::frozen_term_nesting_bound` is one declaration used only by the two native-validator-backed views, `RdfDataset` and `TripleRead`. It returns the same `MAX_TERM_NESTING_DEPTH` certificate. The generic `DatasetView` default and composed-view bound logic remain separate and unchanged; the ledger's `frozen-nesting-bound` home/spec now names this actual shared declaration and its two validated carriers. This does not claim that an arbitrary custom view has a frozen bound.

In `srl_language`, `single_inferred_case` expands the existing assigned-variable and empty-head tests with their original rule text, data text, expected triple and assertion. `infer` remains the single helper that runs the original quad route and compares successful compact inference by flat canonical result before returning the original assertions; the testkit harness still registers both named cases. No duplicated evaluator or changed error-route assertion appears in this narrow consolidation. Runtime and whole #492 qualification remain open at this source readback.

Owner batch 17768 subsequently terminated 0: checker seed/strict, affected core/SRL strict and the complete selected `srl_language` runtime 25/25 passed on this source. This discharges the narrow source-readback runtime gap above; wider #492 gates and integration remain open.
