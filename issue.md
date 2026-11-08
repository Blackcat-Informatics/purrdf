# Issue #1181: Preserve shared term DAGs through columnar dictionary preparation and decoding

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

Campaign: #1174.

## Problem

The columnar codec converts a compact graph of nested triple terms into recursively owned trees. Memoization still deep-clones each resolved value. A valid source with few distinct terms can therefore require exponentially more intermediate storage than its source or Parquet dictionary.

## Evidence

- [Writer preparation](https://github.com/Blackcat-Informatics/purrdf/blob/5d25c3a7d8e76064aa716e3e168d9f443aba7b13/crates/columnar/src/writer.rs#L200) serves `write`, `write_to` and `project`. Its resolver [clones memo hits](https://github.com/Blackcat-Informatics/purrdf/blob/5d25c3a7d8e76064aa716e3e168d9f443aba7b13/crates/columnar/src/writer.rs#L342), constructs owned triple children and clones values into the memo. [Dictionary construction](https://github.com/Blackcat-Informatics/purrdf/blob/5d25c3a7d8e76064aa716e3e168d9f443aba7b13/crates/columnar/src/writer.rs#L446) clones those values into ordered collections again.
- [Reader resolution](https://github.com/Blackcat-Informatics/purrdf/blob/5d25c3a7d8e76064aa716e3e168d9f443aba7b13/crates/columnar/src/reader.rs#L513) repeats deep memo cloning before the decoded dictionary is interned into the builder.
- [`TermValue::try_clone`](https://github.com/Blackcat-Informatics/purrdf/blob/5d25c3a7d8e76064aa716e3e168d9f443aba7b13/crates/rdf-core/src/ir/term_walk.rs#L1163) allocates owned children. Iterative traversal avoids stack recursion but does not preserve sharing.

Let `t0` be an IRI and `tn = Triple(t(n-1), p, t(n-1))` for a fixed predicate IRI. There are O(n) distinct records and reference edges, but the expanded tree obeys `S(n) = 2*S(n-1) + O(1)`, or Theta(2^n). Even a chain with only one nested component creates quadratic aggregate owned dictionary content. These are source-derived complexity findings; no allocation curve was executed during the audit.

## Required change

Keep validated flat term records keyed by source or local IDs throughout preparation and decoding. Implement structural equality and ordering over the shared graph with memoization and exact collision resolution, then perform one canonical remapping. Intern reader terms bottom-up by ID rather than expanding each dictionary value into a tree.

Preserve the current `TermValue` total order, canonical table order, emitted Parquet bytes, cycle/dangling/type refusal behavior and original source-error ownership. Sorting by source IDs or hashes is not a replacement for structural ordering. Reuse applicable existing flat dictionary and GTS mechanisms without adding a second generic term engine.

## Acceptance

- Existing byte goldens and canonical read acceptance/refusal remain exact under reordered source IDs and ingest order.
- Native nested-chain and shared-diamond cases exercise `write`, `write_to`, `project` and `read_with_ordinals`.
- Retained term storage follows distinct records and edges; no intermediate expanded owned tree remains in these paths.
- Report structural comparison work, allocation calls, requested bytes and retained/peak bytes as chain depth and sharing grow. State remaining ordering costs separately from storage costs.
- Preserve typed failures and source identity for invalid references, cycles and illegal term positions.

Classification: compatible internal optimization. No required public API break and no dependency on the pre-v4 migration campaign.

## Verification

Source audit at `5d25c3a7d8e76064aa716e3e168d9f443aba7b13`; no runtime measurement or test was run for this finding. Extend focused native columnar coverage and compile actual affected callers under the required repository profile. Preserve mandatory gates and WASM test ownership. Performance acceptance uses work and resource evidence, without elapsed-time targets or binary-size goals.


## Comments (0)

