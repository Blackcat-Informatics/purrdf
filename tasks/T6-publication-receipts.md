# Concrete transport and publication receipts

Signed normal-hook commit 52988974f2d11a40214281648983f9145af7a3fb, tree 43b0817e30a3ad110cd013796a064c177a27e067, signature status G. Explicit six-file staging; standalone cached whitespace check terminal 0 before separate commit invocation. Normal commit terminal 0 (raw/T6-commit.log), normal push terminal 0 (raw/T6-push.log), exact remote readback raw/T6-remote.txt. All 52 qualified source hashes still match; only selected untracked Stage evidence remains.

stagectl pr-create created https://github.com/Blackcat-Informatics/purrdf/pull/464, but its wrapper exited 1 while parsing gh's successful plain-text response as JSON. Exact error is raw/T6-pr-create.log. Read-only implementation inspection found forge.py create_pr invokes the JSON-only _gh helper for gh pr create. No retry, alternate creation or tool edit occurred. Stagectl branch lookup independently resolved 464 and live PR/meta readbacks confirm OPEN, nondraft, main base, expected issue branch and head 52988974f2d11a40214281648983f9145af7a3fb. The concrete PR body is the corrected candidate 1f3619160b8b2395237f979d621f070627f487ad1029c24f55851812d4e6219f. Source setup and creation used the required Stage tools; the tool response error is not presented as a terminal-success command.

Plan publication on issue deduplicated the exact immutable original comment:
https://github.com/Blackcat-Informatics/purrdf/issues/458#issuecomment-6013901476

Plan on PR:
https://github.com/Blackcat-Informatics/purrdf/pull/464#issuecomment-6019626579

Required confidence answers on issue:
https://github.com/Blackcat-Informatics/purrdf/issues/458#issuecomment-6019631272

Required confidence answers on PR:
https://github.com/Blackcat-Informatics/purrdf/pull/464#issuecomment-6019634912

All four stagectl comment commands terminated 0, with exact receipts in raw/T6-{plan-issue,plan-pr,confidence-issue,confidence-pr}-publication.log. Complete live bodies and metadata are raw/T6-issue-publication-readback.json, raw/T6-pr-publication-readback.json and raw/T6-pr-publication-meta.json. Original failed timestamp finding remains visible and will receive its verified closure checkpoint after final Task 6 receipt review.

Initial actual check output raw/T6-initial-pr-checks.txt has state pending, stagectl terminal 1. It records queued/running hosted jobs; CodeRabbit reports SUCCESS as a check but its actual comment says usage cap reached with no review. No hosted full-pass or independent CodeRabbit review is inferred. These are Stage 2/3 states to resolve, distinct from Stage 1 publication.

Independent final Task 6 receipt review is pending. This report does not itself mark Task 6 complete or establish current-base integration/Stage 2/3 readiness.
