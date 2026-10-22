Support fallible evidence queries and scoped retrieval

Evidence answer, status, interned and prepared witnessed queries accept the
original fallible dataset view. They share the existing execution, admission,
evidence and answer-tier bodies. The composed scoped retrieval pipeline retains
the original typed read owner through deferred exclusions, fusion and the final
source checkpoint, so provider refusal and snapshot drift cannot publish a
successful partial result or semantic unknown in place of an operational error.

Direct errors from non-latching, unbounded sources are captured at the shared
fallible execution ingress. The first original cause is retained without cloning
or rendering it. Existing infallible doors preserve their original owner path.

Local qualification: 62 native composition tests, 66 read-owner regressions,
45 actual cases on each scalar and SIMD WASM, strict affected all-target clippy,
helper hygiene and normal signed commit/push hooks. Independent review covers
all 12 changed files and has no actionable comments. Full hosted gates remain
pending; fill their actual disposition before final publication.

Closes #682.
