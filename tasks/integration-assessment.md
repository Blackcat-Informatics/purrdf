# Integration with the current main

FINAL RESOLUTION: CI test merge 465c34b6e has parents d839dd72a and ede07f859.
Its tree matches the clean prediction exactly. The workspace checkout log and
GitHub commit metadata are retained in final-workspace-job-raw.log and
final-ci-test-merge.json. All 50 checks succeed with five configured skips.
The earlier pending qualification requirements below are now satisfied.

Main advanced from 0994332d9 to d839dd72a through PR 581. Its complete source
delta is one role-error conversion in entailment's CeExtractor::role, a public
regression and one Makefile portable selection. The new conversion calls the
existing source-witness helper before constructing EntailError. It does not
change native URI parsing/ranges, SPARQL cast admission, or text delta processing.
Our parser-file change removes a stale datatype comment in another function.
The Makefile selections are in separate target invocations. merge-tree reports
a clean combination; the initial prediction is in current-integration-tree.txt.
The incoming PR's recorded execution covers 17 public role cases, the library
and service controls and three actual WASM cases. It was merged with broad
hosted qualification still pending; that pending run is not treated as a pass.

The incoming portable case increased the combined inventory by one, but main's
manual inventory still said sixteen role cases. Our edited inventory therefore
needs 138 named cases, 143 executions and seventeen role cases: the old 135/140
plus our two cases and main's one. These counts describe the combined shipping
tree. The feature branch remains behind main; there is no source conflict or
semantic reason to create a synchronization commit. Head-only selections lack
the one base-only case, as expected for that behind-base branch.

The text implementation and URI repair retain their qualified local behavior.
The final local release WASM build is on f329f8120, not a newly claimed execution
of the incoming role correction. Fresh hosted test-merge qualification must
cover the final documentation head combined with current main, including the
new native/portable role regression, before integration. Refresh actual merge
inputs and the prediction before ghprsq; changed inputs require an affected
delta assessment. No full local suite is repeated merely for a behind branch.
