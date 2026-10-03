Task 4 repair — runtime freshness and scratch accounting

Committed and pushed `d6f94f62a80c641f3b51734c73f19c64c0140ebc` with normal commit hooks.

The investigation's execution controls exposed two concrete collisions: BNODE could reuse a dataset's default-scope `bnode1`, and independent SERVICE evaluations could reuse a local/previous response identity. A shared fallible mint now checks dataset and concrete-input identities, including nested RDF 1.2 triple terms and composite literals. Raw, prepared, UPDATE and stateful user-function entry points reserve inputs. SERVICE preserves each response's identity classes across rows and nested cells, while independent responses remain distinct; its allocation effect uses the authoritative sequential classification. Discarded surplus cells do not allocate, and reserved-looking caller prefixes retain their exact semantic identity through the shared label codec.

Template/list allocation uses the same seam. Source-read failures propagate separately from unbound terms and typed governor exhaustion. Scratch growth is accounted per independently evaluated arena, including user-function reservation copies; input registration stops at the first trip. A template budget trip cannot publish an in-flight CONSTRUCT graph or a staged UPDATE mutation. The ordinary `c1` collision control advances to `c2`; no-collision spellings remain unchanged. CONSTRUCT no longer replays the graph to repair a collision.

Validation passed on the final source:

- Core library: 1,124 tests, plus 2 blank-publication controls.
- Evaluator: 1,353 library tests, 25 governed-query, 22 governed-UPDATE, 17 prepared-execution, 16 scope-interaction and 13 fallible-query controls.
- Strict all-target core/evaluator clippy with `-D warnings`.
- Generated-artifact check; frozen official corpora remain unchanged and the deficiency ledger remains marker-only.
- Three existing end-to-end CONSTRUCT benchmarks in each of two source snapshots, all measured with zero failed cases. The report and machine payload record samples, uncertainty, source hashes and host limits; the blank-free control also moved, so no isolated causal speedup is claimed.

Independent final review accepts all six investigation criteria, the 15 interaction rows, measured prototype limits and exact benchmark evidence. Final `make check`, explicit `make wasm` and full unsharded `make conformance` are now running against this commit. Their results will be recorded before the non-draft PR is created, followed by CodeRabbit remediation and the stage-3 structured merge.
