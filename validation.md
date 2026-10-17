# Current acceptance

Initial source: fetched origin/main at 0c8ceac5259fbfe2aa256b59b8c9e5b6b1db322e.
The unchanged exact CLI test reproduced the reported failure at entails_cli.rs
line 1073, with the same malformed-proof message. See tasks/T1-cli-reproducer.log
(exit 101, one failed case, thirty filtered out). A temporary bounded comparison
probe established the term-coordinate mismatch and was removed before qualification.
The correction and current evidence are indexed below. Review, publication and
integration remain open. No full local gate,
external corpus, optimized producer, resource probe or timing performance
measurement is authorized here.

## Correction and current qualification

The probe confirms a source-coordinate mismatch: recorded individual 85 becomes
owl:imports in the canonical reparse; Tom is now individual 87. The consumer now
checks against its own resolved frozen snapshot through ReasonerSession::check_proof.
Both public entry points use one extracted checker. Document decoding/source-load
priority is preserved, and the session's checked imports status is retained.
No producer context/search state is shared. Canonical data export bytes remain
unchanged; its inaccurate proof-context advice is corrected. The validator,
support journal, wire encoding and CLI test assertions have no changes. The
temporary probe is removed (proof.rs has no diff).

| Criterion | Current evidence |
|---|---|
| Reproduce reported defect | PASS as reproduction: initial exact CLI exit 101 and same line/message; T1-cli-reproducer.log |
| Native public boundary | PASS: new case covers IRI, separate-document blank labels and role-derived membership; rejects wrong verdict, changed source and modified proof; T2-public-native.log |
| Existing shared artifacts | PASS: all sixteen producer/checker vectors unchanged; same native log |
| Original actual CLI | PASS with unchanged assertion; T2-cli-regression.log |
| Existing native current support | PASS: all five source identity/service/generated/assumption/equality controls; T2-current-support-native.log |
| Strict compilation | PASS: affected validate/CLI library/tests and new public target; T2-source-context-strict.log and T2-public-boundary-strict.log |
| Repeated CLI selection | PASS: the consistency filter selected the same one case, thirty filtered; no additional coverage; T2-cli-consistency-controls.log |
| Actual scalar WASM | PASS: both exact cases executed, none filtered; T2-public-wasm.log. One selector added to the existing target/invocation, so inventory increases by one name and execution (146/151) with targets/invocations unchanged. No general inventory checker for these counts was found. |
| Helper hygiene | PASS: 82 enforced ledger jobs, 118 distinct rows, no open copies or out-of-crate includes; T2-helpers.log. No core capacity authority source changes. |
| Normal commit hooks | PASS: c86272091, five explicit source files; T2-commit.log |
| Publication | PASS: normal push of c86272091, PR #593 exists; T2-push.log. Creation response was unparseable after success; branch lookup confirmed #593 without duplicate creation. |
| Hosted review/CI, merge/closure | CodeQL on c86272091 fails on two full-report test failure messages; published dc569c874 removes only those dumps. G1 strict, both native/scalar-WASM cases and normal hooks/push PASS. Refreshed hosted checks remain pending. See remediation-plan.md, tasks/G1-published.md and tasks/T3-codeql-annotations.json. Initial merge-tree computation was clean with unchanged base; recompute for the new head before merge. |

Owner assessments (tasks/T2-review.md and gap-analysis.md) are not independent approval. Final hosted qualification is
now PASS, with five configured skips. Both CodeQL threads are resolved and no
actionable CodeRabbit source finding remains. See final-merge-brief.json,
pr-checks.txt, raw review captures and review-debt.md. No full local gate or
unrelated corpus was run. Only protected integration/post-merge verification
remain; no issue requirement is deferred.
