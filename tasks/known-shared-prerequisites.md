# Applicable shared source and fixture corrections

Root extracted known-shared-prerequisites.patch from the actual
5144f5c1a..20ede055f diff, selecting only12 existing source/fixture paths.
Read-only application check against the main-based531 evaluator transport
passes. No source was changed while91403 compiles.

The patch moves the unchanged counters tests below production; removes the
needless borrowed Copy geometry closure; fixes actual evaluator/retrieval
rustdoc links and relation_identity's misplaced documentation; updates the
existing nested RDF-star producer matcher; and carries the final shared core
SSE2/i586 and sequential/Rayon HNSW fixture corrections. Their exact inputs,
bit assertions, public admission and environment-restoration controls remain.
These are the already independently reviewed corrections qualified on542's
current hosted source, not new algorithms or tooling changes.

The selected source matches the reviewed542 delta and its scoped receipts in
.stage/rdf-core-mutabledataset-freeze-uses: shared-strict-clippy.log,
shared-counters-tests.log, shared-mxcsr-native.log, shared-rustdoc.log,
shared-standard-doc-gate.log, shared-hnsw-native.log and current20ed CI records.
The current hosted541/539 i586 distance39/39 and542 architecture controls
qualify the exact fixture; final531 current-head CI remains required.

No MutableDataset compaction API, mutable/facade acceptance test, measured SIMD
table or Python scoreboard was copied. Manual/generated claims must be derived
from531's actual final source/results. No Cargo, cache, profile, toolchain,
target directory, workflow, checker or threshold change is in this patch.

Applied once by the source owner after91403 reached terminal0,13/13. Inspect the
combined diff, run invalidated affected strict/docs/fixture controls as needed,
and include these known prerequisites before final full qualification. Preserve
failed historical receipts; application-check success is not runtime acceptance.
