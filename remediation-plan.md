# PR 527 workspace lint remediation

Use the lightweight Stage 2 path for one concrete owned CI finding. The original
complete issue, plan, source judgment and nine-family qualification remain the
authoritative context; this repair does not change production behavior.

Completed workspace job 114221558900 of run 38055010701 failed strict Clippy at
`owl_dl/bounds.rs:677`: `owner_tests` preceded production items. Its job-specific
metadata/logs are preserved as `hosted-workspace-job-1.json/.log`; a full-run log
was not needed. All preceding workspace and preserve-order checks passed.

Move the entire unchanged ownership-test module after the production items. No
test, assertion, implementation, lint or CI gate is removed or suppressed. Run
the managed affected all-target strict entailment lint and both actual ownership
tests, obtain the parent's proportional delta judgment, then commit and push with
configured signing/hooks. Fresh exact hosted CI remains the binding hosted
acceptance. No broad mandatory-gate restart is needed for this test-only move.

The existing local compiler is 1.100 nightly4b6d04e70 (2026-09-13); this CI job used
1.101 nightly32dba69d6 (2026-10-09). A direct isolated Rustup install was rejected
by automatic tool review because Stage owns Rust toolchain installation and
selection. That attempted command did not run. Parent explicitly instructed us
to use the existing managed focused checks and leave exact hosted CI binding;
there is no installer/selection bypass or global toolchain mutation.

The ordinary one-file source delta is `hosted-workspace-layout.patch`. Original
full4/O3/native/portable/host evidence is reused only for unchanged production
and assertions. Published head, hosted status and integration are updated
separately after actual results. Parent owns final review/debt and ghprsq merge.
