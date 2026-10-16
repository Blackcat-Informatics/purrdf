Retain segmented scan blocks across rows

Closes #563

Quad, reifier and annotation scans now retain their authenticated row allocation
and validated byte range across yields. The shared iterator performs readiness,
cache lookup and header validation at block boundaries, then decodes through the
existing row decoder. It releases the old owner before admitting another block.

Retaining a row must not occupy a scarce cache slot for the entire nested scan.
QUADS entries can therefore leave the cache while the iterator keeps their
original bytes and physical grant. Lexical pins retain their existing eviction
refusal. This fixes the two layered-consumer regressions exposed during focused
qualification, without raising any cache or memory limit.

The two new deterministic controls cover all three streams, empty/full/tail
blocks, one-slot transitions, block-scaled readiness/cache work, early drop,
exact live bytes after eviction and the sticky first fault at the next block
boundary. All 33 existing segmented/layered tests and all 18 scalar WASM cases
pass. Strict affected Clippy, fresh helper hygiene and normal hooks/push pass.
No timing measurements were used.

The metadata refresh changes only the shared constructor binding on 40 proof
rows. The standard gate includes the changed session/cache files in that source
domain; all authority sources, sinks, classes, lines and reasons are unchanged.
No checker, exemption, dependency, feature, public API or image format changes.

Hosted source review covers all four changed files and reports no actionable
defect. The required ledger connection is documented in the PR and review reply.
Its default private-function docstring percentage is declined with the existing
ownership comments and repository public-documentation policy explained. There
are no inline review comments or unresolved threads in the captured review.
No independent agent review is claimed; hosted source review is retained.

Final CI: all 49 checks succeed, with four expected optional profile/projection
skips. Complete native, conformance, host, portable, architecture and source
review checks pass. The current integration is clean and main has not advanced
from the tested base. Final checks/reviews are captured with no unresolved debt.

The most sensitive behavior is nested ownership: the initial cache-slot pin
regression was reproduced and corrected, then all affected consumers passed.
Readiness is checked at a block boundary as requested; already authenticated
rows can finish their block after a provider stop. A detected first fault still
refuses complete publication. No required work was moved to a follow-up issue.

Plan and selected evidence: .stage/segmented-scans-fetch-their-block/plan.md and
the matching Stage directory. Integration uses ghprsq; evidence/notes, closure,
mirror and cleanup are verified after landing.

Defect-Class: performance
