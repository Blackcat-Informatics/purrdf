Keep ordinary evaluator child continuations on admitted worklists

Closes #531

FILTER, EXISTS and LATERAL children retain admitted worklist frames instead of
recursively reentering ordinary evaluation. Copied pattern positions project
from their original shared owner, so child traversal is linear while allocation,
scope, grant and owner release remain at their existing homes. The evaluator
uses its safe API over the lexical allocation implementation.

Native admission/refund/last-owner controls and the physical stack fixture cover
actual refusal and recovery. WASM separately retains the correct192KiB answer
and below-margin typed refusal; native retains its192KiB refusal. Original
optimized depth assertions181/281/180 are unchanged. Existing CI recipes gain
45-minute deadlines without altered compiler/profile/cache settings.

The relocated temp-root regression passes after a standard Cargo rebuild. The
ordinary row remains40 bytes. Registered fixed-query DISTINCT observations do
not establish an instruction win or throughput; the canceled36/78 timing
campaign remains excluded.

## Integration

Main synchronization retains remembered-graph defaults and the text SurfaceIndex
repair already landed in main. Four conflicts in retrieval/search.rs were only
rustdoc spellings: qualified function links preserve both link-resolution
intentions. No evaluator algorithm or acceptance assertion changes in resolution.

## Verification and remaining contract

Published production78d32abe0 passed full make check, including preserve-order,
core-hygiene and release WASM. cb164c108 passed50 hosted checks with5 explicit
skips, including library, Python, optimized package and seven assembly jobs.
Native new runner controls3/3 and scalar physical-stack1/1 have scoped passes.
Applicable existing source reviews are reused; no fresh independent audit is
claimed. Integrated remembered-graph controls passed3/3 and the normal merge
hook passed on9e87ee8a2. Main has no required-status-check rule; refreshed optional
new-head CI is pending rather than represented as another pass. The selected
validation index records unchanged evidence applicability and integration.

Complete remaining evaluator acceptance is #545. It includes the verified
callback-unwind NodeBoundary defect and caught-panic/same-context reuse, revised
native/SIMD fixtures and final source-bound generated applicability. The original
full completion assessment remains FINDINGS-OPEN for that remaining contract.
It is not represented as a full acceptance pass of this delivery.

Authoritative plan: .stage/evaluator-delivery-follow-up-relocated/plan.md.
Selected evidence: .stage/evaluator-delivery-follow-up-relocated.
Current assessment: validation.md and review-debt.md.

Defect-Class: performance
