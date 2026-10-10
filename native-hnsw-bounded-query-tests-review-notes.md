# Bounded native HNSW query acceptance unit

The standard patch appends three acceptance tests to the existing real-artifact
SPARQL integration binary and adds the existing dev-only counting allocator.
It changes no shipping traversal or scoring implementation.

Coverage: resident/bounded answers for ranked retrieval, count-free membership,
missing seeds and a candidate behind the rank prefix under LIMIT; direct text,
prepared and governed-prepared routes; allocator peak versus the source account;
allocation-free cloned/extracted publication surviving all input owners; and a
binary search for the lowest cold-source capacity that enters native traversal.
That final capacity must preserve the typed Residency cause, fail before a
successful candidate-work publication, and release all query scratch grants.

The source-opening request ledger deliberately has 32 entries. A large unused
source ledger could otherwise conceal an unpriced native allocation in a peak
comparison. Artifact preparation and immutable plan construction occur outside
measurement; source opening and query execution occur inside it.

Verification so far: git apply --check passed against the live selected worktree;
the complete candidate parses with the managed rustfmt. Compilation and test
execution remain unrun. Healthy assertions intentionally reject the legacy
unpriced-configuration refusal. No bounded HNSW completion is claimed.

Integration belongs to the existing sole source writer. Cargo.lock must reflect
the normal dev-dependency resolution. Keep existing query laws and every required
acceptance assertion when reconciling concurrent retained-result/schema changes.
