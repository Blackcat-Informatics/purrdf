# Frozen publication and review scope

The user explicitly superseded the earlier no-partial-PR workflow: publish the existing work now, make an issue for regressions, address review and merge this PR without expanding its scope. The wider backlog goal is paused; this review/merge task is the current authorization.

PR 691 publishes branch work through 2e42e8792. Normal hooks passed; the branch is pushed. Stagectl created the PR but then failed to parse the response; an authoritative forge lookup confirmed PR 691 exists, so creation was not retried. Issue 690 records the failing acceptance and its complete 41-test inventory. Issue 569 remains open for its full original contract.

Allowed changes here are review and qualification corrections to the published behavior, such as its existing product stamp/fixture and caller parity. Do not add LFP/GFP solver functionality, new formats, new host capabilities or other backlog work to this PR. Required gates and original assertions are not waived by issue creation.

Current head is ea133b288, pushed with a clean source worktree. Follow-on commits repair existing catalog contexts/callers, projections and inspection fixtures, refresh audited proofs and the mutable product fixture, merge main's README-only delta, disambiguate MSRV inference and correct the census mutation probe. Final LinkML repairs preserve annotated/class-reference choices and original slot carriers, with cycle/stack guards and a source-byte-based expression budget. Every commit and the main merge passed normal hooks. No solver, format or host capability was added.

The four recorded inline review findings are fixed, replied to and resolved. Review of ea133b288 is complete with no actionable comments. The reviewer independently reconciled and withdrew its general risk proposals after checking source and baseline; the separate existing OpenAPI packaging bug is 692. Final-head run 40041027523 is completed/success, and the entire PR rollup reports pass, including all platform/assembly and aggregate gates. No earlier head's green check is substituted for this final evidence.

All selected local checks pass. The authoritative current acceptance index and precise reuse limits are in validation.md. The original 41 library failures and later integration failures have passing targeted repairs. Current allocation/catalog/product controls retain their ceilings, stage 81d0 and the 5608-byte mutable fixture; the frozen format epoch is intact. No fresh whole make check execution or completed parent-issue contract is claimed.
