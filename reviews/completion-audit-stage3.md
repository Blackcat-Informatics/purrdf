# Completion audit, Stage 3 (pre-merge): PurRDF #570, PR #599

## What was reviewed

- **Source:** worktree `/home/paudley/Active/purrdf/.worktrees/570-submit-w3c-shacl-implementation-reports`, branch `paudley/570-submit-w3c-shacl-implementation-reports`, head `aec3a6dcb`. Base `origin/main` is `c75cbf947`: `refs/remotes/origin/main` and `FETCH_HEAD` confirm it, and the reflog shows the merge at `0e4d164b` after `origin/main` moved to `c75cbf947`. Per the caller, the integration tree is the head tree (`dac611a9`).
- **Plan:** `plan.md`, revision 3 (R1–R37, U1–U6, G1–G17).
- **Records read:** `issue.md` / `raw/issue.json` (the original text; see the limits), `validation.md`, `handoff.md`, T2/T7/T8/T9/T10 reviews and triage, `pr-checks.txt` and `pr-comments.md` (the plan comment and the "least confident" comment). `.deficiencies` in the root and the worktree has no entries. Also the W3C workspace `contributions/data-shapes/purrdf-earl/{drafts,browser}`.
- **Code read:**
  - `crates/shapes/tests/support/w3c_earl.rs`;
  - the EARL and scoreboard sections of `w3c12_conformance.rs` and `w3c_conformance.rs`;
  - `crates/conformance-kit/src/outcome.rs` and the golden `earl-report.nt`;
  - the `check`, `release-tags` and `w3c-earl` targets in the `Makefile`;
  - `release-cargo.yaml` and the `ci.yaml` msrv job;
  - `scripts/publish-github-release.py` and `scripts/check-release-notes.py`;
  - `docs/RELEASE.md`, `docs/CONFORMANCE.md`, `docs/book/src/validation/shacl.md` and `vectors/shacl-reports/README.md`.
- **Limits on coverage and authority:**
  - No cargo or make was run: a build is in progress and the caller forbade it.
  - This auditor had no shell, so `git diff origin/main...HEAD`, `git log --format=%B`, `gh pr view 599 --json body` and `gh issue view 570` could not be run.
  - Pass/fail results come from `validation.md` and the task reviews. Each is judged below for whether it still applies to the head.

## Criteria

| R-id | Requirement | Evidence | Status |
|---|---|---|---|
| R1, R2, R24 | Re-vendor 1.2 from a recorded current `gh-pages` commit; update the frozen manifest | `vendor-shacl12.py:41`, `PROVENANCE.md:25` and `shacl12.rs:54` all pin `4cd469867a1c…`. `check-corpus-frozen` is OK (T1); `the_suite_commit_is_the_vendored_one` checks the pin. | Demonstrated. The "current" part was not rechecked at merge time (F5). |
| R3, R4 | Check `vectors/shacl` against upstream and re-vendor if it drifted | `validation.md`: `diff -rq` of `core/` and `sparql/` gives 0 lines, and `git log 08adb377..4cd46986 -- data-shapes-test-suite/tests` is empty. I compared the root `manifest.ttl` myself: identical. | Demonstrated |
| R5, R6, R27 | Every approved 1.2 and 2017 entry passes unamended | `make conformance` was green at `4e007c462` (SHACL 1.2 591/591, SHACL 1.0 129). The only change from there to the head is a CDT pin in `crates/sparql-conformance/tests/cdt_lexical_divergence.rs`, so that result still applies. | Demonstrated |
| R7–R10, R28, R29 | Spec-contradicted cases are reported failed with a WG issue; no amended expectation or ledger | Triage T3–T7 found engine defects only. `SPEC_CONTRADICTED` is empty in both harnesses (`W12:579`, `W17:102`). The mechanism is tested: `check_classification`, `check_spec_table`, and the synthetic non-empty case `a_spec_contradicted_entry_is_reported_failed_with_its_rationale`. XFAIL is empty and `a_listed_entry_is_never_amended` passes. Grepping the tracked tree for `non-canonical-expected-decimal` finds nothing. | Demonstrated (empty by triage result) |
| R11–R21 | EARL through `to_earl`: shape, keys, subject, assertor, determinism, proposed entries excluded | The harness tests are part of the 4e007c462 `make conformance` run. Reading the head code: `earl:info` sits on `earl:TestResult` (T8 F1 fixed), the 2017 `TOTAL` line carries the spec-contradicted count (T8 F2 fixed), and an in-scope XFAIL is an error (T8 F4 fixed). | Demonstrated |
| R30, R31 | Harness tests fail on EARL/scoreboard disagreement | `agree` plus the flip, remove and add mutations on the serialized bytes, each with `assert_ne`. The tally comes from the `classify` match arms. | Demonstrated. These tests can fail. |
| R22 (PurRDF side) | `make w3c-earl`: writes on a clean tree, refuses a dirty one | Run at `1af88c643` (`validation.md` "Task 9"; T9 §5). Not re-run at the head; the case for reusing it is in A2. | Demonstrated, carried forward |
| R32, R33 (1.2) | The WG page loads with no errors, no "no data" and counts equal to the scoreboard | Chromium run on output from `1af88c643`: all six checks true, 382 passed, and the class counts match the harness's printed class lines (T10). | Demonstrated, carried forward (A2) |
| R32, R33 (2017) | U2 mechanical inventory check | `w3c_2017_earl_matches_wg_inventory` against the frozen, byte-equal `alltests.ttl` | Demonstrated |
| R23 | Prepare the upstream submission PR (a draft) | `drafts/w3c-pr.md` exists and is not opened | Demonstrated, with a small gap (F7) |
| R25 | `make check` passes | The last full run, at `4e007c462`, failed. The fix is in `aec3a6dcb`. The rerun was stopped. | **Unverified at the head (F1)** |
| R26 | `make conformance` passes | Green at `4e007c462`, and the change since does not affect it | Demonstrated |
| R37 (U5) | EARL generation is part of the release | `Makefile:247` runs it in the preflight before the tags. `release-cargo.yaml:168` runs it before `SHA256SUMS`, with attestation at `:174-178` and evidence at `:189-191`. The asset set is derived (7). The msrv CI job runs both harnesses (`ci.yaml:508`). | Demonstrated on the PurRDF side. A real release run is external (U5). One stale message (F6). |
| R34, R35 | Tag, regenerate on the tag, submit upstream | External per U1/U5 (quoted below) | Correctly classified as external |
| R36 | #384 mentioned in the PR body only | No issue number in any repository file I checked. I could not read the PR body. | **Unverified (F3)** |
| CI | Required hosted checks | "Build The PurRDF Book" FAILURE; most jobs IN_PROGRESS (`pr-checks.txt`) | **Failing (F2)** |

## Findings

### F1 (BLOCKING). R25: no complete `make check` at the head, and the validation record overstates the last run

- **Evidence.** `validation.md:39` says: "make check: full run at 4e007c462 failed only on cdt_lexical_divergence". That test runs inside `Makefile:172` (`cargo test --workspace --locked --no-fail-fast`). Make stops at the first failing recipe line, so unless the run used `make -k`, the following lines never ran at `4e007c462`:
  - `:173`: the `preserve_order_consumer` tests;
  - `:174`: `$(MAKE) rdf-core-hygiene`;
  - `:175`: `$(MAKE) wasm`.

  Those are exactly the areas that `955db552` ("rebind the rdf-core capacity proofs") and `53b42866` (wasm and capi test expectations) changed.
- **Consequence.** Acceptance criterion 1 ("`make check` and `make conformance` pass") has no evidence at the head. The record and the PR comment present a partial run as one that failed on a single target.
- **Fix.**
  1. On the final head, after the `docs/book/po` commit, either:
     - run `make check` to exit 0 and record the command, the head and the exit status in `validation.md`; or
     - record every required hosted check green on that exact head SHA. `check-gate-parity` keeps CI equivalent to `make check`.
  2. Rewrite `validation.md:39` to: "make check at 4e007c462 stopped at `Makefile:172` (`cargo test --workspace`). The only failing test was `cdt_lexical_divergence`. Lines 173–175 (preserve_order_consumer tests, rdf-core-hygiene, wasm) did not run." Drop the second half if the run used `make -k`, and say so instead.

### F2 (BLOCKING, CI). "Build The PurRDF Book" is failing

- **Evidence.** `pr-checks.txt:27`. The English source is correct: `docs/book/src/validation/shacl.md:596-599` gives `W3C12 TOTAL: passed 591, spec-contradicted 0, xfailed 0, ledger 0`, in the harness's format (`W12:797`). This is a translation-catalog blocker, not an implementation defect.
- **Fix.** Land the `docs/book/po` fix and confirm every required check is green on the final head. This audit's coverage carries over to that head only if the new commit touches nothing outside `docs/book/po/`.

### F3 (UNVERIFIED). I could not read the PR body or the live issue text

- **Evidence.** This auditor had no shell, so `gh pr view 599 --json body` and `gh issue view 570` could not run. `raw/issue.json` is the pre-alignment text. `handoff.md:3` reports that the live text was already aligned.
- **Required check** (plan G3 and Task 12 step 7). The body must:
  1. say `Closes #570`. That is allowed only because `SPEC_CONTRADICTED` is empty in both harnesses and the issue text was aligned under U1. Otherwise it says `Refs #570`;
  2. not claim that `make check` is green at the head until F1 is resolved;
  3. state that `community-conformance`'s `earl.nt` bytes change (subject and assertor type triples, assertion nodes labelled over sorted records), and why (G13, T8 F3);
  4. list the open external steps:
     - tag v4.0;
     - generate on the tag;
     - re-run the page and inventory checks on the published assets;
     - open the W3C PR;
  5. not say the reports were "submitted" or "published";
  6. mention #384 only as related (R36).
- **Fix.** The orchestrator checks the body and the live issue text against items 1–6 and corrects any overclaim before merge. Edits on PurRDF are covered by U6.

### F4 (honesty). The PR comment repeats F1's overstatement

- **Evidence.** PR comment 3: "The last complete `make check` (at `4e007c462`) failed on only one target, a CDT file-count pin." This is subject to the same caveat as F1.
- **Fix.** In the PR body or the next status update: "The last full `make check` (at `4e007c462`) stopped at its workspace test step on one failing test, a CDT file-count pin, so its last three steps did not run there." Then give the final head's result.

### F5 (record gap). The "current `gh-pages`" recheck (plan Task 12 step 6) is not recorded

- **Evidence.**
  - `validation.md` has no entry for it.
  - The W3C clone's `FETCH_HEAD` and `packed-refs` show `origin/gh-pages` = `4cd46986`, but nothing records when that fetch happened.
  - The draft W3C PR (`drafts/w3c-pr.md:15-16`) makes the same recheck a precondition.
- **Fix.** Record `git -C /home/paudley/Active/w3c/data-shapes fetch origin gh-pages` followed by `git log --oneline 4cd469867a1c..origin/gh-pages -- shacl12-test-suite/tests data-shapes-test-suite/tests` (output and date) in `validation.md`. Report any newer suite commits in the PR body.

### F6 (low). A stale asset count in the release publisher

- **Evidence.** `scripts/publish-github-release.py:278` says "retained release checksums do not match the four distribution assets". With the two EARL files there are now six non-manifest assets (`asset_names` at `:196-204`; `docs/RELEASE.md:318` says "six other"). U5 required the count wording in this file to follow the asset set.
- **Fix.** Line-neutral: `"retained release checksums do not match the other release assets"`. Optionally, mention the EARL reports in `:238` ("complete C and lexicon distribution") and in the module docstring.

### F7 (low, W3C workspace draft). The draft W3C PR does not name the 2017 suite commit

- **Evidence.** The R23 check requires the draft to name "the suite commits". `drafts/w3c-pr.md` names `4cd46986` for 1.2 only. The 2017 file is graded at `08adb3776709a014bc3062ede793c36275b22446` (`W17:539`; `vectors/shacl/README.md:13`).
- **Fix.** Add to the 2017 bullet: "graded at `08adb3776709a014bc3062ede793c36275b22446`, whose `tests/` content is unchanged at `4cd46986`."

### F8 (low). Stale final records (plan Task 12 step 10)

- **Evidence.**
  - `handoff.md` describes `bc38ae4da`, with R25/R26 OPEN and `VERDICT: BLOCKED`.
  - The `validation.md` "Task 1" block quotes the interim numbers (571 passed, 6 non-canonical, `check-doc-claims` failing) with no marker that later results supersede them.
  - Tasks 2–8 have no `validation.md` entries; their evidence lives only in the T-reviews.
- **Fix.** Refresh `handoff.md` for the final head with every R-id's status (the table above can be reused). Mark the Task 1 block as the state at `c5f2529a6`, and point each of Tasks 2–8 to its review.

## Advisories (not blocking)

- **A1. The report does not say which profile it was graded under.** The 1.2 `sht:Validate` entries are graded under the dated `shacl-wd-2026-09-18` profile; LEGACY is the library default (T2 §2). `sparql-failure-001`, `sparql-message-001` and `pre-binding-009` pass only under the dated law, but the report's `doap:description` (`w3c_earl.rs:139-141`) does not say so. Consider adding "validated under the SHACL 1.2 profile `shacl-wd-2026-09-18`" so the WG page's claim can be reproduced. The issue does not require it.
- **A2. Why the earlier `make w3c-earl` and page evidence still applies to the head.** Since `1af88c643`, `dad6f14a` refactored code in `shapes` and `conformance-kit`. What the page check depends on is pinned by tests that passed at `4e007c462`:
  - the key set (`w3c12_earl_keys_equal_wg_page_keys`);
  - the exact subject and assertor (`check_descriptions`);
  - every outcome (`w3c12_earl_outcomes_equal_the_unamended_grades`);
  - the renderer's bytes (the `earl-report.nt` golden);
  - the write path (`check_write_errors`, `check_config_parsing`).

  The U1/U5 steps re-run the page check on the published assets in any case. One `make w3c-earl` on the final clean head is cheap and needs no model, so it would turn this reasoning into direct evidence.
- **A3. One harness test passes without checking anything.** `w3c12_earl_report_is_written_when_configured` (and its 2017 twin) returns early when no environment is set, so it checks nothing under `make check` or `make conformance`. That is by design, but it is evidence for R22 only under `make w3c-earl`, not as a standalone pass.

## Clean checks

- **Not dark or test-only.** The only callers of `to_earl` are the two harnesses and `community-conformance`. `make w3c-earl` runs the whole test binaries, so a scoreboard failure fails the target, and the release lane and `release-tags` both call it.
- **Not refused.** `inapplicable` is never emitted because the claimed classes equal the union over the SHACL root (`W12:1002`), and every one of the 382 passes. No requested support is counted as done by refusing it.
- **No duplicate path.** The amended-grade path and its label are gone.
- **Tests not weakened.** Every assertion I read checks something exact. The decimal-spelling expectation changes follow the engine change recorded in T7-triage, with spec citations.
- **Repository text.**
  - No issue numbers, process wording or agent attribution in the EARL-related files I grepped (Rust sources and tests, the Makefile, workflows, scripts, docs and `vectors/shacl-reports`).
  - `check-issue-refs` and `check-doc-claims` ran and passed in the `4e007c462` run (`Makefile:138`, `:141`, both before the failing line).
  - The process wording flagged in handoff N2 has been reworded (`W12:573-578`).
  - DOAP and FOAF IRIs appear only in test code (`w3c_earl.rs`). The one library hit, `crates/slice/src/mapping_support.rs`, predates this change.
- **Commits.**
  - All reflog subjects are conventional commits with no issue numbers. The body of `aec3a6dcb` (`COMMIT_EDITMSG`) is clean.
  - `handoff.md` reports the first 12 commit bodies clean.
  - I could not read the bodies of `0e4d164b`, `dad6f14a`, `955db552`, `53b42866` and `4e007c46`. Required check: `git log --format=%B origin/main..HEAD | grep -nEi 'co-authored|#[0-9]+|claude|anthropic|generated (with|by)'` returns nothing.

## Scope decisions (Patrick's words) and how they were applied

- **U1:** "this issue is separate from our submission later to the W3C data-shapes group" and "We'll still want to meet all the submission criteria we need to on this side in this issue."
- **U5:** "(1) v4.0 is coming up as soon as we clear the backlog, make sure the w3c-earl generation is part of the normal release process please."
- **U2:** "Mechanical check (Recommended)" (the 2017 page).
- **Classification.** These decisions make external only:
  - the tag;
  - generation on the tag;
  - the re-check on the published assets;
  - opening the W3C PR (R34, R35, and those parts of R22, R23, R32/R33 and R37).

  Everything PurRDF-side is built and tested, and nothing else was moved out of scope. No requirement went to a follow-up issue or a local log. The PurRDF-side release integration U5 asked for is implemented.

## What is left before PASS

1. F1: a complete, recorded `make check` on the final head, or every required hosted check green on that exact SHA, and the corrected `validation.md:39`.
2. F2: the book check green after the `docs/book/po` fix, and every other required CI check green on the final head.
3. F3: the PR body and the live issue text checked against items 1–6, and the five unread commit bodies checked.
4. Recommended in the same pass: F4–F8.

VERDICT: BLOCKED
