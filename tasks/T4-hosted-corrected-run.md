# Corrected hosted campaign monitoring

Run37746157677 attempt1 binds corrected committed source
221b1ace8bd30e010dba6b9d732cfc9ff576a0ed.
[Actual run](https://github.com/Blackcat-Informatics/purrdf/actions/runs/37746157677).
Root dispatched; this monitor is read-only and performs no source/Git mutation,
build, cache change, push, cancellation or dispatch.

Initial actual snapshot: run queued, five completed jobs,23in-progress and23queued;
no job failure yet. Raw jobs/status/head/attempt snapshot is retained in
T4-hosted-corrected-run-37746157677.json. Current campaign/configuration admission
and measurements remain PENDING, not passing. The prior failed run37738739777
and its complete refusal evidence remain preserved separately.

Subsequent actual snapshot: run in progress,17completed/34in-progress jobs;
all twelve profiling arms have started, mostly installing the pinned header
generator, with later starts still installing/admitting Rust/Binaryen. No actual
job failure yet. Compiler admission succeeded; exact artifact11535578001 is
retained as T4-hosted-corrected-run-37746157677-evidence/admission.zip. No arm has
yet completed configuration admission or native campaign acceptance.

First settled arm: before-downstream job113208296149 actual SUCCESS. Artifact
11537225139 retained as evidence/arm-before-downstream.zip under the exact run
evidence directory; both cold and unchanged warm receipts have all nine phases
successful, including real configuration admission, source/target identities,
actual native command and attributed Cargo telemetry. Actual projected
build/profile/target states are absent; explicit effective jobs4/threads4 and
private target/build remain bound separately. Receipt source head is corrected
221b1ace8. Mandatory workspace also actually SUCCESS. Other arms and closed
comparison remain pending; one good pair is not complete campaign acceptance.

Genuine failure: after-doc job113208296102 completed FAILURE. Retained artifact
11536587458 is exact evidence/arm-after-doc.zip; completed job log is
evidence/job-after-doc.log. Cold configuration admission/identity, source/target
inventory and actual native command all succeeded. cargo-telemetry-inventory
failed with `JSON byte 0: expected a JSON value`; no warm receipt follows.

Source-confirmed owning cause: profile::collect filters Cargo JSON lines, then
for a CAPI compiler-artifact passes the entire child stdout to phases::cdylib.
That strict selector parses every nonempty line as JSON. Retained successful
doc/example-test children also carry normal libtest lines, including
`running 5 tests`, so this caller violates the selector's JSON-only input.
Archive retains three successful child receipts and three attributed HTML reports;
Cargo-prefixed frames parse, while complete stdout is deliberately mixed.
Required correction must isolate admitted Cargo frames before the strict exact
artifact selector, preserving malformed/ambiguous/missing-artifact hard refusal
and complete raw receipts. No source change/build/rerun was performed here.
Root was notified promptly. This attempt cannot establish complete measurements.

Current settled matrix: before-downstream, after-downstream, after-integration-3
and after-integration-4 SUCCESS, with both cold/warm successful phase receipts.
after-doc and after-integration-1 FAILED mixed-stream collection; before-capi,
after-capi and profile-before-capi FAILED late artifact-liveness collection.
Configuration admission and actual native commands succeeded in these five
failed uploads; collection failure prevents their measurement acceptance.
All nine settled arm archives and individual completed job logs are retained in
the exact corrected-run evidence directory. after-capi also records16/16actual
C phases passing, with its selected library identity, separate from temporary
header artifacts. Three arms remain active: after-integration-2, before-monolithic
and after-lib. Overall terminal/comparison still pending.

Root requested the source-only bounded correction design, recorded separately in
T4-telemetry-correction-plan.md. It does not mutate measured source or certify a
rerun. Current owned failures remain failures; no speedup is claimed.

Actual final terminal: completed FAILURE; no live jobs remain. All40non-profiling
jobs SUCCESS, only optional SIMD projection SKIPPED. All12arms settled:4successful
cold/warm pairs and8failed cold telemetry arms (five mixed-output, three temporary
library-lifetime). In every arm the corrected configuration admission and actual
native command succeeded. Final before-monolithic upload11537698497 and job log
are retained with all prior arms. Comparison113224869377 actually FAILED after
restoration, refusing `failed/incomplete phase cannot qualify a comparison`.
Its exact upload11538730996 is evidence/comparison.zip; completed job log is
evidence/job-comparison.log. Raw terminal snapshot binds corrected source221b1ace8
and attempt1. No accepted complete measurements or speedup exists. Root admitted
subsequent source-only implementation after this actual terminal, while local
build/test admission remains withheld.
