# RDFC heap search

Frozen behavior: replace the recursive Hash N-Degree Quads call chain with
explicit owned heap frames on the sole production path. Preserve related-hash
order, permutation order, both pruning points, temporary issuer ownership,
canonical hashes/bytes/labels, call/permutation charging and stop checkpoints.

Dependencies: current CanonState, IdIssuer, lazy Permutations and WorkList.
Use no new package, compatibility path or external implementation. The profile
version stays unchanged because the successful algorithm and bounds stay exact.

Exclusions: configurable budgets, work/profile request identity, dataset-state
controls and heap UDF execution remain under the parent contract. This child
closes only its native-stack obligation and never closes the parent.

Acceptance: existing native canonical/profile/stop/refusal cases and W3C frozen
vectors; one deep n-degree traversal through production frames. Strict affected
compilation, reviewed constructor/source proofs, normal hooks, independent review
and hosted gates. No generic semantic WASM execution or whole-workspace replay.

Risk: issuer transfer, resumption and path pruning can silently change canonical
identity. Retain no recursive fallback or test-only second algorithm. Existing
frozen byte oracles and deliberate reordered fixtures qualify identity.

Active engineering begins 2026-10-24 22:38 UTC. Reassess after two active hours;
maximum four active engineering/local-validation hours. At most two delivery
lanes; the oversized-term lane remains local pending exact-byte review.
