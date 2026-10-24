# Merge preparation — final head

Head: ea133b28828cbae033249eeae0c40dfe4d1c7c84. Known fetched base:
d26bc6e640ae116754aebb0cadcf40cb6a19933c. Main was already merged normally;
its source delta was README prose. The qualified merge-tree command exits zero
and predicts 0ec565f87b2d3be0bdb92d12f776a47dd709faf8. Raw output is
merge-tree-ea133b-qualified.txt. Because main is already incorporated, this
candidate adds no untested code to the current branch. Refresh the actual base
before merging, and recompute/reassess if it advances.

Local checks and reuse limits are validation.md and slot-carrier-validation.md.
Final-head hosted run 40041027523 and independent review are pending. The review
request is posted; captured bot status confirms it processes the three changed
files from 2443eec1b through ea133b288. Four earlier findings are fixed/resolved.
review-debt.md deliberately remains BLOCKED until fresh evidence is reconciled.

The selected Stage currently holds 8.1 MiB, has no symlinks and no individual file
over 2 MiB. It contains raw failure/acceptance logs, complete public feedback,
current decisions and necessary input/output evidence, without a copied checkout
or build tree. Freeze writers only after final checks/review are captured. The
presence-only merge-ready gate passes; it does not establish substantive readiness.

Sibling squash notes remain explicitly DRAFT. Finalize them only after hosted
checks, independent review and merge-plan assessment pass, post them through
stagectl to issue 569 and PR 691, and merge only through ghprsq with this exact
Stage directory and sibling notes. Keep issues 569/690 open as documented.
Verify archive and landing before supported cleanup; preserve donor worktrees,
the original stash, and the user-paused backlog.
