# Independent review and current dispositions

PR #681: https://github.com/Blackcat-Informatics/purrdf/pull/681
Review submission 5619456291, 2026-10-22T19:05:26Z, COMMENTED.
The independent automated review selected all 62 changed files over
base 6848fc886..578bb65da. Incremental review through 9a695a072 selected four
integration files; three were skipped as already-similar changes and no new
actionable comment was produced. This does not dismiss the original finding.
This artifact projects material review text and scope; promotional UI, embedded
commands, billing and HTML copies are omitted. Full forge feedback remains linked.

## Substantive finding

Thread PRRT_kwDOTKq-Ms6vVlf3, comment 4352393124:
https://github.com/Blackcat-Informatics/purrdf/pull/681#discussion_r4352393124
At crates/datalog/src/store.rs:1131–1176, a refused insertion can retain a seal
preparation. Refusal may occur inside the dictionary transaction, after later
admission, or at a work pause. Relation::remove did not invalidate that frame.
With 63 tail cells, refused insertion then removal then insertion changes Shape
and panics in Preparation::reserve; unchanged shapes can retain removed rows in
copied publication buffers. The requested correction is to retire the frame on
successful removal, account its bytes and forward retirement through
remove_native. The review's overall risk statement repeats this same defect.

Confirmed against actual production entry points. Public regression initially
failed with the exact 64 versus 63 Shape panic; the correction passes native and
actual WASM execution. A supplemental native control covers zero/partial/complete
preparation, absent removal retaining valid work, sealed-batch withdrawal,
exact frame retirement transfer and resumed insertion without the removed row.
All 467 library tests and 49 runtime tests pass natively. The WASM run passes
49 runtime tests and 16 refutation tests; affected strict checks also pass.
The correction is committed and pushed at 55e3b0527 with normal hooks.
Independent incremental review c774c290-3f03-446c-bbd6-c82ef21af2a2 covers
9a695a072..55e3b0527, selecting all six changed files. It reports no actionable
comments and minimal merge risk, explicitly confirming that removal no longer
permits a stale preparation panic or row resurrection. The reviewer also confirms
the fix in reply 4352937347. Thread PRRT_kwDOTKq-Ms6vVlf3 is resolved.
All 51 current hosted checks now pass; four configured checks are skipped.

## Other review observations

The linked-issue check could not identify the random-program/random-batch
property control. Existing maintained_transactions_match_seeded_random_signed_histories
generates 8 seeded recursive programs with 8 random recursive rule choices each,
two strata of negation and 96 signed transactions per program. Every transaction
compares complete facts, exact ModelDelta and canonical chosen derivations with
cold evaluation and independently checks grounding. It is registered in the
supported runtime harness and passed in the current 49-case native and WASM runs.
It is a deterministic generated property control; no shrinking claim is made.

The updated automated docstring warning is 46.88% over 576 touched functions,
against that review's generic 80% threshold. No repository 80% threshold is
established. Public supplied constructors, ownership/work methods and failure
boundaries are documented and pass repository docs/strict gates. No padding or
mechanical private-function documentation is introduced to satisfy a generic
percentage. Required repository documentation gates remain mandatory.

The review positively classified title, description and scope; related-issue
links do not expand this delivery or close those parents. The stated missing
property pointer is addressed above. Updated linked-issue and scope checks pass.
LanguageTool reports incomplete prose checking for docs/design/purrdf-simd.md
after its five-request per-file limit; this is not a repository acceptance gate.
The actual assembly and repository documentation gates remain mandatory.

Paginated REST capture exhausted all reviews (5619456291, 5620122174,
5620126451), all inline comments (4352393124, 4352933903, 4352937347), and both
issue comments (6348666658, 6349554994). GraphQL reports hasNextPage false for
both review threads and thread comments, with one resolved thread and no open
thread. Latest review-submission timestamp is 2026-10-22T20:02:07Z.
Full material feedback and dispositions are projected in pr-comments.md.
The final paginated refresh after CI completion reports the same review and
comment identities and timestamps, with no additional feedback.
