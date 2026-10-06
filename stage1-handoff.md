# Stage 1 publication and acceptance handoff

PR https://github.com/Blackcat-Informatics/purrdf/pull/466 is OPEN. HEAD b2edf7450cf654d20ae97bd856d67102d0916b7b, source tree 13496228db0e5ec79074acad9020cac5aab7556e, base origin/main b6f7c9b0f6b84ffe496719e39f2d2ba52d5ed3ac. Clean merge-tree prediction is the same source tree (base is an ancestor). Task 2 commit signature G, fingerprint AF5E0032F7494CEBCAA7BBBE9B87CFBBCFDBAF11; normal hook, push and remote readback all succeeded. Task 2 issue update: https://github.com/Blackcat-Informatics/purrdf/issues/457#issuecomment-6024639301. Plan posted to PR: https://github.com/Blackcat-Informatics/purrdf/pull/466#issuecomment-6024649053.

The creation wrapper returned a JSON parsing failure after GitHub created the PR. stagectl PR resolution and independent native readback verified a single correct PR; no create retry was performed.

Independent tasks/T1-review.md and tasks/T2-review.md PASS. Task 2 patch/hash manifest and reviewed tree equal the committed source. No source changes remain; only ignored process .stage directory is untracked. Task 1 hashes are unchanged by base synchronization and Task 2. Qualification details/logs are in validation.md and task reports.

Correction recorded during G2 evidence preparation: the selected `.stage` folder is
untracked and **unignored**, as verified by `git ls-files --others --exclude-standard`.
The earlier “ignored” wording above was incorrect. It is excluded from source
commits and captured separately by ghprsq, which is a different property. Functional
qualification is unchanged; hosted assembly source identities must be reconstructed
from a clean checkout plus the generated document patch, without these local process
files. The current acceptance index and hosted execution path govern that replay.

Stage 2 inputs: raw/S2-pr.json includes body/comments/reviews/checks; raw/S2-reviews.json and S2-inline.json are paginated native review surfaces; raw/S2-threads.json is paginated GraphQL with nested comment pagination flags checked. Initial native surfaces are empty, and checks are running. raw/S2-issue-comments.json refreshes full issue discussion. review-brief.json and branch-under-review.md bind 16 changed files/two logical commits. Mechanical review-brief gate passed, independently assessed reports remain required.

Least confident: retained dictionary/reprojection scaling with deep stacks. The implemented costs are documented; no throughput improvement is claimed and the issue does not require a benchmark target. Consumers own depth and compaction scheduling.

User needs to know: consumer-owned storage/log and atomic publication policies are explicit issue boundaries. Replacing a stack with an ordinary canonical base retains current graph membership but cannot encode hidden populated explicit-versus-implicit graph lifetime policy; callers retaining that policy must retain its separate data. Wasm compilation passed; runtime wasm and hosted CI remain unverified until actual checks establish them. No issue requirement was deferred or cut.
