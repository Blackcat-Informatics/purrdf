Inspected completed post-merge failures from native587. All four share the same stale realization proof artifact:

- wasm-package job117458981838: entailCheckProofGoldenVectors reports case realize proof mismatch.
- pytest job117458981966: test_the_committed_proof_bytes_reproduce_through_this_host fails on realize.
- conformance-python job117458982296: the same Python shared-artifact assertion fails on realize.
- combined conformance job117461999007: the same Python assertion fails on realize.

Full decoded logs are retained. Native588 owns the shared artifact repair, explicit native refresh operation and cross-host qualification; no check is disabled or weakened. Broader GMEOW1843 node-kind work remains in its clean isolated worktree and is resumed after this introduced regression is repaired. There is no resource/load diagnosis or process restart involved.
