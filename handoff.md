# Handoff: PurRDF #570, PR #599 (final-head record)

Replaces the pre-PR snapshot at bc38ae4da.

## 1. Header

- Branch: `paudley/570-submit-w3c-shacl-implementation-reports`
- Head: `142c8faf411157263f7b79ff138cb83a4dc39936` (verified with `git rev-parse HEAD`). It is a docs-only commit that regenerates the conformance matrix. The last code commit is `9b6608f60`.
- PR: #599 (Blackcat-Informatics/purrdf). Base: `origin/main`, merged at `1049784f2`.
- PR body says `Closes #570` (U11).
- Hosted CI on this head is NOT green and not finished. At the last look 3 checks passed, 14 were pending and 5 were skipping.
  - The `doc` job FAILED at "Doc (deny warnings)" (`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps ...`). The error is `public documentation for builtin_keyword_arity links to private item builtin_arity / form_arity`, at `crates/sparql-algebra/src/parser.rs:5235-5236`. It comes from commit `5698dac5a`.
  - "Build The PurRDF Book" was pending.
  - Local `make check` exit 0 did not catch this. The fix is to name the private helpers without intra-doc links, as `08495db09` did for shapes. It is a code change, so rerun the gates on the new head afterwards.
- Evidence gap: `validation.md` has no per-task sections for Tasks 2-8 or 14-21. Their evidence is `tasks/T*-review.md` plus the head-level gates below.

## 2. Requirement table

Status key: MET, OPEN-EXTERNAL (outside this PR by U1/U5), OPEN.

Head-level evidence, used in the rows below:
- `make check`: exit 0 at 9b6608f60 (validation.md, "Task 22 full gates").
- `make conformance --write-doc`: GREEN at 9b6608f60 (validation.md, "Task 22 full gates"). SHACL 1.2 is 590 + 1 spec-contradicted, SHACL 1.0 is 129, prepared-product is 407, Python is 4232.
- Matrix committed as 142c8faf4.

| ID | Status | Evidence |
|---|---|---|
| R1 | MET | `vendor-shacl12.py` and `PROVENANCE.md` pin 4cd469867a1c...; `the_suite_commit_is_the_vendored_one`; validation.md "Task 1". |
| R2 | MET | `check-corpus-frozen.py` OK, 29 roots (validation.md "Task 1"). gh-pages still 4cd46986 (validation.md "gh-pages recheck"). |
| R3 | MET | `diff -rq` of core/ and sparql/ gives 0 lines (validation.md "Task 1"). |
| R4 | MET | `git log 08adb377..4cd46986 -- data-shapes-test-suite/tests` is empty. No re-vendor was needed. |
| R5 | MET | `w3c_shacl12_conformance` and `every_planned_inventory_id_passes` in `crates/shapes/tests/w3c12_conformance.rs`. 590 pass + 1 spec-contradicted (hasLang-example, U10). conformance GREEN at 9b6608f60. |
| R6 | MET | `crates/shapes/tests/w3c_conformance.rs`: 2017 subset 120/120, graded under `shacl-rec-2017-07-20`. SHACL 1.0 is 129 in the matrix. |
| R7 | MET | T3-T7 triage records and `tasks/T13-triage.md`; the one spec-contradicted entry is recorded under U10. |
| R8 | MET | The six fix-only cases pass unamended. No tracked source, script or doc mentions `non-canonical-expected-decimal`. The only hit is an obsolete `#~` msgid in `docs/book/po/zh-Hans.po`. |
| R9 | MET | `a_listed_entry_is_never_amended` and `a_spec_contradicted_entry_is_reported_failed_with_its_rationale` in `w3c12_conformance.rs`. |
| R10 | MET | `xfail_ledger_is_empty` in `w3c12_conformance.rs`. |
| R11 | MET | Both harnesses build a `Report` and call `to_earl` (`w3c12_earl_*` and `w3c2017_earl_*` tests; T8 review PASS). |
| R12 | MET | Same as R11. |
| R13 | MET | Assertion counts are pinned: 382 for 1.2 and 120 for 2017. Page re-check shows 381 + 1 = 382. |
| R14 | MET | `w3c12_earl_outcomes_equal_the_unamended_grades`. The hasLang entry is `earl:failed`. |
| R15 | MET | `W3C12_CLAIMED_CLASSES` (6 classes) in `shacl12.rs`; no `inapplicable` is emitted. |
| R16 | MET | `w3c12_earl_keys_equal_wg_page_keys`; page re-check has 0 no-data cells and no Unknown-tests row. |
| R17 | MET | Exact-value `check_descriptions` tests; T8 review. |
| R18 | MET | Same as R17. |
| R19 | MET | Same as R17 (assertor). |
| R20 | MET | Double-generation byte-equality test and the `earl-report.nt` golden. |
| R21 | MET | 2017 `alltests.ttl` proposed subject absent (`w3c_2017_earl_matches_wg_inventory`); proposed 1.2 entries excluded. |
| R22 | MET (PurRDF side); OPEN-EXTERNAL (tag build) | `make w3c-earl` on a clean tree at 142c8faf4 passed its output checks. The dirty-tree refusal is recorded in validation.md "Task 9". Generating on the v4.0 tag is external. |
| R23 | MET (draft); OPEN-EXTERNAL (opening) | `/home/paudley/Active/w3c/contributions/data-shapes/purrdf-earl/drafts/w3c-pr.md` exists and is unposted. Posting needs Patrick's approval of the exact text. |
| R24 | MET | Same evidence as R1 and R2. |
| R25 | MET locally; hosted OPEN | `make check` exit 0 at 9b6608f60. Hosted CI on the exact head is pending and one job (`doc`) is failing, see section 1. |
| R26 | MET | `make conformance` GREEN at 9b6608f60; matrix at 142c8faf4. |
| R27 | MET | 1.2 scoreboard 590 + 1 spec-contradicted; 2017 120/120. |
| R28 | OPEN-EXTERNAL | The WG hasLang issue is drafted in `drafts/wg-findings.md` and not filed. Filing needs Patrick's approval. |
| R29 | MET | By U11 the PR says `Closes #570` whether or not the WG issue is filed. The issue stays a draft in the W3C workspace. |
| R30 | MET | `earl_agrees_with_scoreboard` in `w3c_conformance.rs`, with its mutation tests; harnesses run in `make conformance`. |
| R31 | MET | Same as R30. |
| R32 | MET (local page); OPEN-EXTERNAL (published assets) | Page re-check PASSED at 142c8faf4 (validation.md "Task 22: WG 1.2 reports page re-check"). Messages empty, 0 errors, one PurRDF column, 0 no-data cells, no Unknown row. Core 162/162, SPARQLValidation 33/33, NodeExpr 145/146, InferenceRules 31/31, SPARQLNodeExpr 5/5, SPARQLCustomFunctions 5/5. 381 passed + 1 failed (hasLang-example). Re-check on published assets is external. |
| R33 | MET | 2017: `w3c_2017_earl_matches_wg_inventory`. 1.2: the page counts equal the scoreboard. |
| R34 | OPEN-EXTERNAL | Tag v4.0 and the published assets. Not done by design (U1/U5). |
| R35 | OPEN-EXTERNAL | Regenerate on the tag, re-check on the page, and open the W3C PR. Not done by design (U1/U5). |
| R36 | OPEN (minor) | No repo file mentions #384, which is correct. The plan wanted it referenced in the PR body, and the current body does not mention it. Add a line to the body if wanted. |
| R37 | MET | `Makefile` `release-tags` preflight runs `w3c-earl`. `release-cargo.yaml` builds, attests and publishes both reports. `docs/RELEASE.md`. Only self-tested; the first real run is v4.0. |
| A-R1 | MET | T14 review PASS after recheck. wasm `a_shacl_profile_option_is_refused_by_a_twin_that_takes_no_law`; Python `bindings/python/tests/test_shacl_profile.py` (9 pass). CLI lane 213/213 under the ED law (per T14 and the PR body). The `validation.md` section for it is missing. |
| A-R2 | MET | T13 review PASS. `shacl-ed-2026-10-16` added; WD fidelity restored; 1.2 harness graded under it; 2017 prefix path gated. validation.md "Task 13". |
| A-R3 | MET | T15 review PASS. The lanes run under `tc.profile`; `REFUSED_AT_LOAD` entries removed. |
| A-R4 | MET | T16 review PASS; order-independence tests. |
| A-R5 | MET | T17 review PASS after recheck. Evaluation failures are typed apart from processor failures. |
| A-R6 | MET | T16 review PASS; the closed-path reifier test. |
| A-R7 | MET | T18 review PASS. `SECONDS` keeps its lexical field (`05.500`). Governor v22. |
| A-R8 | MET | T16 review PASS. A second expression reifier is refused. |
| A-R9 | MET | T19 review PASS. Includes are resolved at load. |
| A-R10 | MET | T19 review PASS. `validate_change` is correct under entailment. |
| A-R11 | MET | T20 review PASS. One assertion per test and subject; the community runner uses per-route subjects. |
| A-R12 | MET | T21 review PASS after recheck. Cache keys cover specs and manifests. |
| A-R13 | MET | T13 and T21 reviews PASS. The 2017 harness runs under `REC_20170720`; the footprint, closed-path and refusal-position pins are in. |
| A-R14 | MET | T18 review PASS. Unary plus is the identity; `numeric_render_cost` prices the emitted form. |
| A-R15 | MET | T21 review PASS. The doc and pin corrections are in. |
| A-R16 | MET | T21 review PASS. `make w3c-earl` fails on a missing or empty output. |

Reviewer notes:
- T14, T17 and T21 were BLOCKED first, then fixed and rechecked to PASS.
- T19 passed with a recommended CHANGELOG/CONFORMANCE note. I did not verify that the note was written.

## 3. Completion audit (reviews/completion-audit-stage3.md, VERDICT: BLOCKED at aec3a6dcb)

- **F1 (no complete `make check` at the head).** Addressed locally: `make check` exit 0 at 9b6608f60. STILL OPEN for the required hosted CI on the exact head SHA, which is pending, with the `doc` job failing (section 1). It is not closed until every required check is green on the final head.
- **F2 (Book check failing).** Addressed in code by the zh-Hans catalog commit `114b98530` and later book commits. "Build The PurRDF Book" was still pending on the current head, so it is unconfirmed.
- **F3 (PR body and issue text).** Checked in the current body.
  - It says `Closes #570`.
  - It does not claim hosted green.
  - It states that the community `earl.nt` names route subjects.
  - It lists tag, re-check and the W3C PR as not done, and does not say submitted or published.
  - It does not mention #384 (R36).
  - Commit bodies in `origin/main..HEAD` contain no issue numbers or agent-credit lines, by grep.
  - I did not re-read the live issue text.
- **F4 (overstatement of the earlier run).** `validation.md` carries the correction note. The PR body now states only the 9b6608f60 results.
- **F5 (gh-pages recheck).** Recorded in validation.md: origin/gh-pages is still 4cd469867a1c.
- **F6 (stale asset count).** Addressed by `be1021160` ("name the distribution assets without a stale count").
- **F7 (the 2017 suite commit in the W3C PR draft).** The plan assigned this to Task 22 in the W3C workspace draft. I did not re-read the draft, so it is unverified.
- **F8 (stale final records).** This file replaces the stale handoff. The Task 1 block in validation.md is still the c5f2529a6 interim state and carries no supersession marker.
- Advisory A1 is superseded by the EARL description naming the law (A-R1).

## 4. Disclosures

- `conformance-baseline.json`: the ledger count went 0 to 1 as a forced consequence of U10. The hasLang-example entry is spec-contradicted, so the ledger is no longer empty. `SPEC_CONTRADICTED` in `crates/shapes/src/shacl_corpora/shacl12.rs` has 1 entry and `SPEC_CONTRADICTED_COUNT` is 1.
- The community `earl.nt` bytes changed: it now has one subject per (profile, surface), so the output differs from before.
- Five new zh-Hans book entries are flagged for a native reviewer.
- `GOVERNOR_PROFILE_VERSION` is 22.
- Wrong-arity keyword built-in calls (e.g. `ABS(1,2)`) are now syntax errors. This is a breaking change for v4.0 and is listed in the PR body.
- External and unposted, approval-gated: the v4.0 tag, the published assets, the W3C PR, and the WG hasLang issue with its eight further discussion items.
