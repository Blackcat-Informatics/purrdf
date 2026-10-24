# Acceptance output capture

GhpRsq excludes live .log files. All settled nonempty command outputs at capture
time are preserved byte-for-byte in acceptance-evidence/<same-stem>.txt. The
original .log names retain execution provenance; failed and superseded candidates
remain historical and are not claimed as final acceptance.

- ambient-domain-reproduce.log
- catalog-memo-allocation.log
- commit.log
- definition-bound-native.log
- definition-commit.log
- definition-identity-native.log
- definition-merge-brief.log
- definition-push.log
- definition-reports-final.log
- definition-strict-callers.log
- definition-strict-final.log
- identity-candidate.log
- push.log
- report-foreign-logical.log
- reproduce.log
- strict-callers.log

Current native acceptance is definition-bound-native and definition-reports-final;
strict acceptance is definition-strict-final. Normal final commit/push use
definition-commit and definition-push. Full hosted CI 40056143633 and the final complete PR check rollup pass.
