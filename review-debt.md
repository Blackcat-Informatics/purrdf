# Review debt: PurRDF PR #599 (issue #570)

Audited 2026-10-18. Nothing was posted. Read-only inspection only.

## Inventory

| Source | Count |
|---|---|
| Top-level PR comments (`pr-comments.md`) | 3 (coderabbitai notice; paudley plan; paudley handoff notes) |
| Reviews (`raw/pr-reviews.json`) | 0 |
| Inline comments (`raw/pr-inline-comments.json`) | 0 |
| Review threads (`raw/pr-threads.json`) | 0 |
| Issue #570 comments | 12, all by paudley (plan + per-task progress notes); no outside feedback |
| `reviewDecision` | none (empty); `mergeStateStatus` = UNSTABLE (checks not green) |

Note: the brief said 2 top-level comments; `pr-comments.md` holds 3 (the handoff note is a separate comment). All are dispositioned.

## Diff shape (why CodeRabbit skipped)

`git diff --stat origin/main...HEAD`: 502 files changed, 7606 insertions(+), 2171 deletions(-).

| Class | Files |
|---|---|
| Vendored corpora under `vectors/` | 371 (shacl12: 364, sparql-governors: 4, shacl-reports: 2, shacl: 1) |
| Source, tests, docs, scripts, CI | 131 (crates/shapes 54, crates/sparql-eval 13, bindings/python 10, crates/cli 6, scripts/conformance-frozen 5, crates/sparql-conformance 5, others) |

CodeRabbit counted 499 selected files (+2 path-filtered) against a limit of 300. The vendored corpus alone (371) exceeds the limit, so no split of the hand-written work could have avoided it unless the corpus travelled separately. 131 non-corpus files are under the limit.

## Dispositions

### D1. coderabbitai "Review skipped, too many files (499 > 300)"
- Type: bot notice. It delivered zero findings, zero threads, no review.
- Disposition: LEGITIMATELY DECLINED. No code change is possible or implied. Splitting the PR is not warranted: 371 of 502 files are the re-vendored upstream corpus, which `AGENTS.md` requires to change only through the vendor script plus `check-corpus-frozen.py --update` in the same change as the harness that grades it. Separating them would leave a PR that fails the frozen-corpus and conformance gates.
- Reply: not required, because the notice asks for nothing and the `CodeRabbit` status check is SUCCESS. If Patrick wants the skip recorded on the PR, this is the draft (needs his approval before posting; under the W3C-workspace rule and the PurRDF GitHub-comment gate, do not post unprompted):

  > Acknowledging the skipped review. 371 of this PR's 502 files are the re-vendored W3C SHACL 1.2 test-suite corpus (`vectors/shacl12`, `vectors/shacl-reports`, and related), which is machine-vendored and frozen by `check-corpus-frozen.py`, so it cannot be split from the harness change that grades it. The hand-written change is 131 files (mostly `crates/shapes`, `crates/sparql-eval`, bindings, scripts and CI). Nothing is asked of the bot here; human review should focus on those 131 files, and the plan comment lists them by task.

- Evidence: counts above; `pr-comments.md` section 1.

### D2. paudley plan comment (Revision 3, posted plan)
- Type: author's own publication, not a review request.
- Disposition: NO ACTION. It is the plan of record. No reviewer questions attached.

### D3. paudley handoff notes ("least confident about")
- Type: author's own disclosures, not reviewer feedback. They are open risks and are tracked here so none is lost:
  1. Release lane (`make w3c-earl` in preflight, `release-cargo.yaml`, seven attested assets) is unproven until v4.0. Accepted, documented, fails before irreversible publish. Not blocking merge.
  2. `make check` at the final commit was not completely re-run; CI on this PR is the real check. BLOCKING until CI is green (see checks).
  3. Decimal spelling rule follows SPARQL examples, not normative text. Accepted, WG discussion item drafted.
  4. 1.2 profile vs suite (`shacl-wd-2026-09-18` vs suite `4cd46986`). Accepted, documented.
  5. `docs/book/po/zh-Hans.po` hand-updated because `mdbook-i18n-helpers` is not installed. This is the direct cause of the failing book build below. BLOCKING until the fix lands and the check passes.
  6. Rdf-core constructor-closure edits need a capacity-proof rebind. Process note, no action now.

### D4. Reviews / inline comments / threads
- Zero of each. Nothing to disposition. No unresolved threads.

### D5. Issue #570 comments (12)
- All authored by paudley: plan plus Task 1-10 completion notes and the handoff. No outside question or requested change. No debt.

## Checks

Snapshot from `pr-checks.txt` (state: fail). Live roll-up when audited showed 37 checks not-yet-success.

### BLOCKING: failing
- `Build The PurRDF Book`: FAILURE. zh-Hans translation gate drift (hand-edited `docs/book/po/zh-Hans.po`). Fix reported in progress; must pass.

### BLOCKING: pending (IN_PROGRESS) until green
aarch64; test (lib), (doc), (integration-1..5); miri; cross-arch; cross-arch-riscv64 (suites, determinism-serial, determinism-one-worker, determinism-more-workers); simd-asm-config (aarch64-neoverse-v1, aarch64, x86_64-v3, x86_64, x86_64-v4, wasm32-simd128, wasm32); geo-determinism; projection-oracles; capi; msrv; wasm; wasm-package; wasm-exec; wasm-schema-oracles; workspace; conformance (python, shapes, sparql, core); avx512-sde; pytest; Analyze (rust).

### Green / non-blocking
SUCCESS: doc, windows-mtime, python-publisher, Analyze (actions, python, c-cpp, javascript-typescript), CodeRabbit. NEUTRAL: CodeQL. SKIPPED: Deploy to GitHub Pages, simd-asm-projection, native-profile-admission, native-profile, native-profile-comparison (skipped by workflow condition, not failures).

### Branch protection
- `gh api repos/Blackcat-Informatics/purrdf/branches/main/protection` returned 404 "Branch not protected" (classic protection absent).
- Repository ruleset 18446030 applies to main via `rules/branches/main`: deletion, non_fast_forward, required_linear_history, required_signatures, pull_request (0 required approvals). It has NO `required_status_checks` rule, and `gh pr checks --required` reports "no required checks". So no check is formally merge-required by GitHub.
- Treat every check as blocking anyway: the repo's own contract (CONTRIBUTING.md `make check` + `make conformance`; handoff item 2) makes CI the real gate, and merge via `ghprsq` has its own prerequisites. Signed commits and linear history (squash) are ruleset requirements to honor at merge.

## Human-gate reminders (not blockers of this audit)
- Any reply, plan publication, or merge needs Patrick's explicit approval of the concrete text/action. Merge only via `ghprsq`.

## Verdict
No open review feedback exists; the only debt is CI that is failing (book build) or unfinished.

VERDICT: BLOCKED
Blockers (all CI, exact): (1) `Build The PurRDF Book` FAILURE (zh-Hans gate drift) must be fixed and pass; (2) every IN_PROGRESS check listed above must finish SUCCESS, in particular `conformance (core|shapes|sparql|python)`, `test (lib|doc|integration-1..5)`, `workspace`, `pytest`, since handoff item 2 says a full `make check` at the final commit was never completed. No review comment, thread, or bot finding blocks. Once those are green this becomes PASS.
