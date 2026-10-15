@coderabbitai review

Commit `f7862dccc` fixes both findings from review
`839234b1-45af-4654-a363-7a2eaf5c45d7` in the existing homes:

- `Memory::reserve` destroys and refunds the rejected replacement on allocation
  refusal and capacity-certificate failure, preserving the original error/data.
- `SegmentedSession::block` rechecks the first fault under its mutex before cache
  reuse or preflight. It preserves both the returned error and sticky status.

Both new regressions failed before the fixes. The corrected source passes all
25 allocation cases, five session-owner cases, 27 segmented-provider/stack cases,
22 neighboring SPARQL cases and the full file-backed 200,000-claim control on all
three fitting layouts. Exact image/peak quantities still match the guide. Fresh
warning-denied workspace Clippy and the actual helper gate pass. The ledger only
refreshes 40 constructor identities and 37 allocation-source authorities, with
all sinks, classes, other authorities and reasons unchanged.

Please verify the two findings are resolved and inspect this bounded delta.
Remaining local execution, current-head hosted checks and final integration are
still pending. Source is settled; no further edits are queued.
