Final merge readiness: PASS. Hosted run 38876291016 on 225e078da completed successfully: 49 successful checks and four expected skips. Both material review threads are resolved; final-review-threads.json binds their dispositions to this head. Current-head review has no actionable findings. The clean integration tree is 5a3000f31bfaa2063190de341c77e23fb58e9f02. All affected acceptance passes; historical pending statements below are superseded by this result.

Current material verdict: no unresolved material feedback on 225e078da. Both actual threads are confirmed resolved by replies 4292242894 and 4292709461; resolved-review-threads.json preserves the complete record. Current-head summary has no actionable findings. Its generic docstring warning identifies no missing public contract; the additional quota and generated follow-ons are declined in issuecomment-6212302455, with repository documentation checks passing. Required CI remains pending.

Current correction is published at 225e078da after all affected checks and normal hooks pass. Reply https://github.com/Blackcat-Informatics/purrdf/pull/561#discussion_r4292667750 records the reproduced larger-cache failure, non-evicting policy and native/portable evidence. Hosted run 38876291016 is active; refresh actual thread disposition and current-head checks before merge. Older a94af3ab2 CI run 38870539013 passed but does not qualify this production delta.

Current material status: FINDING OPEN. Comment 4291214207 is confirmed addressed
by reviewer reply 4292242894 and its thread is resolved. New comment 4292239479
identifies eviction of payloads to fund optional routing growth. The extended
actual pressure case reproduces this at 128 cache slots: bookkeeping grows from
94,624 to 102,944 despite only 4096 spare bytes. The bounded non-evicting correction
is implemented and qualifying, as indexed in validation.md. Publish and reply with
its actual evidence before resolving the new finding. No independent approval is
invented and no current readiness is claimed from the old hosted source.

# Current material review disposition

Comment 4291214207 is fixed in published ee13e8327. The original pressure
reproduction fails at requested 2,006,315 versus limit 2,000,000. The correction
passes the affected native units, full eight-lane workload and native integration;
its regression then moves into the supported public target and actually passes
natively and on scalar/SIMD WASM in follow-up 57a12dbd1. Only optional hint-index
Residency omits a hint; payload/request/source/allocator failures remain terminal.
Exact capacity, value, peak, healthy state and subsequent progress are asserted.

Land merged statistics into this branch, qualify its actual combined source,
publish, and reply with the final evidence. Final hosted checks and fresh review
remain pending. This is owner assessment, not invented independent approval.
The historical dispositions below do not override this current state.

# Current material review

PR 561 is published through ee96cc213. Hosted full checks and actual review are in
progress; owner assessment is not independent approval.

- Hosted inline comment 4291214207, session.rs routing-entry admission:
  REPRODUCED AND CORRECTED LOCALLY; final qualification/publication pending.
  The pressure regression fails on published behavior with requested 2,006,315
  bytes against 2,000,000, then passes with all 20 affected units on the correction.
  Only optional routing-growth Residency omits the unadmitted hint. Mandatory
  payload/request/source/allocator faults retain original propagation, and
  cache_insert guards insertion with the admitted decision. Original workload,
  native integration, portable, strict/helper checks and publication remain.
  The reviewer identifies that routing fences survive payload eviction and their
  memory refusal therefore latches a terminal operation failure even when a new
  payload could otherwise fit. The authenticated lower-bound search can produce
  correct results without a newly learned fence. Reproduce tight-budget growth
  through the real reader and adjudicate whether route insertion should be
  skipped on this particular Residency refusal while all mandatory payload,
  source and allocation faults retain their existing first-error behavior.
  Do not weaken the original full eight-lane workload, drop accounting or add
  an unbounded directory. The proposed code is review data, not proof. No final
  reviewer approval or completed hosted qualification is claimed yet.
- Automated walkthrough is informational; the original constructor and byte-fit
  contract is covered by the owner's current implementation/evidence assessment.

Shared deprecation/character-pattern compatibility fixes from PR 557 are published.
Its subsequent three local retrieval pin changes are also applied locally here;
they retain polling behavior while removing unnecessary heap pinning. The failed
workspace log was successfully retrieved and confirms the original deprecation
errors. Corrected-head hosted full qualification remains required.
