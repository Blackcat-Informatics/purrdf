@coderabbitai review

Commit `2def24ab9` addresses the reader-buffer finding from review
`4bff5b17-43a4-4ff3-87b4-db9c3ecf3d86`.

The existing eviction body now has one `State::evict_one` home. Both ordinary
room checks and actual cache/request buffer admission use it. Buffer admission
retries after reclaiming an unpinned block, under the already-held State lock;
there is no recursive reservation backlink. Block room is checked again after
cache growth, using its actual combined live layouts. Other errors and pinned
owners retain their original behavior.

The new tight-budget case fails before the fix and now passes for request-ledger
and cache-array growth. All six session-owner cases, 27 provider/stack cases,
22 neighboring public query cases, affected warning-denied all-target Clippy and
the actual 82-job/118-row helper gate pass. Only 40 constructor identities are
refreshed; source closure paths, all authorities, sinks, classes and reasons are
unchanged. Please verify the finding is resolved and inspect this bounded delta.

No further source edits are queued. Remaining broad local execution resumes
after this material recheck; current-head hosted checks are pending.
