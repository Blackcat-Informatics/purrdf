# Final review and assembly documentation remediation

Current base: published 2443eec1b. Scope remains frozen to PR 691.

Review comment 4364860635 identified a genuine LinkML projection gap: annotation
keys and local class-reference branches prevented an otherwise object-only named
choice from using the class-expression renderer. The repair reuses the existing
annotation predicate and guarded, cached element-kind resolver. Only class
references qualify; scalar and enum references remain types. No separate resolver,
public API or recursion mode is added. Native LinkML tests exercise both anyOf and
oneOf, retained descriptions, inline fields, class aliases and non-class controls.
Test results are recorded in review-choice-tests.log when complete.

CI run 40033960962 measured shapes.path-eval as v4/f0/r0 in all five native
configurations and v0/f0/r0 in both WASM configurations. The manifest correctly
identifies the production symbol and is unchanged by this remediation. Only the
five stale documentation cells and inaccurate scalar-only prose are corrected.
Authoritative failed-job logs:
- ci-asm-x86-2443.log / job 120688915220
- ci-logs/github-job-120688915365.log (x86_64-v3)
- ci-logs/github-job-120688915238.log (x86_64-v4)
- ci-asm-arm-2443.log / job 120688914939
- ci-logs/github-job-120688915242.log (aarch64-neoverse-v1)
Both WASM assembly jobs passed on this same head. Counts are transcribed from
hosted measurements; no claim of a local seven-platform assembly rerun is made.
No gate threshold, required instruction or compiler input is weakened.

All later check/review results must bind the next pushed head before merge.
