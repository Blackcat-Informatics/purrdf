# Plan completeness review: #570 (pass 2)

Reviewed plan: `/home/paudley/Active/purrdf/.worktrees/570-submit-w3c-shacl-implementation-reports/.stage/submit-w3c-shacl-implementation-reports/plan.md` (revision 2)

Inputs read in full: `plan.md` (rev 2), the pass-1 report, `issue.md` (0 comments), `issue-analysis.md` (R1–R36, NEEDS-JUDGMENT 1–9), and the `prior-art-assessment.md` NEEDS-JUDGMENT 1–5.

Source re-checked in the worktree:
- `Makefile`: there is no `w3c-earl` target. Only `check:` (L91) and `conformance:` (L478) exist.
- `scripts/vendor-*`: there is no 2017 (`vectors/shacl`) vendor script. `vendor-shacl12.py` targets only `vectors/shacl12`.
- `vectors/shacl/README.md`: the pin is `08adb3776709a014bc3062ede793c36275b22446`. The vendored subset is `core/`, `sparql/` and `manifest.ttl`, with no `reports/`.

## Pass 1 findings → disposition

| Pass-1 | Subject | Disposition | Evidence in rev 2 |
|---|---|---|---|
| B1 | Per-task commit, push, issue update and review; final-task publication and handoff | **STILL-OPEN (partial)** | The note after Task 1 covers "Every task from 1 to 9". Task 10 has a commit but is left out of that sequence. Task 11 does not publish the plan and does not say what the handoff contains. See N2. |
| B2 | No harness home for a reported-failed case | DISCHARGED | G1 "Harness representation": the `SPEC_CONTRADICTED` table is pinned by count, graded unamended, refused on XPASS, and emitted `earl:failed`. `XFAIL` stays empty. The R7/R9/R10 row matches. |
| B3 | WG issues drafted but never filed | DISCHARGED | G1(4), G3 and Task 10 "File only after his approval". |
| B4 | No source for the PurRDF commit; R18 check could not fail | DISCHARGED for 1.2 | G7 adds `PURRDF_W3C_EARL_COMMIT` (40 hex, paired, malformed values refused). The R17/R18 row asserts exact values. The 2017 suite commit is still unchecked; see N5. |
| B5 | Page check compared incompatible groupings; "page errors" undefined | DISCHARGED | The R32/R33 (1.2) row checks `#messages`, no "Unknown tests" row, zero no-data cells, and per-`mf:requires` group counts against the per-class tally. A1 below is a minor wording advisory. |
| B6 | R31 test only exercised `to_earl`'s guard | **STILL-OPEN (partial)** | `scoreboard_line_equals_earl_counts` now exists. Its mutation is still "flipping one record's verdict", and the count-disagreement case is untested. See N4. |
| B7 | Tag order; `Closes` while open | DISCHARGED for R22/R34 (U1) | Patrick's U1 is quoted. Its remaining R28 part is N3. |
| B8 | Prior-art licence item | DISCHARGED | G14. |
| A1–A8 | Advisories | A2, A3, A4, A5, A6, A7 and A8 DISCHARGED (G12, G10, R3 row, R5/R6 row "1–7", G16, Task 10 docs, Task 9 worktree). A1 is partial: the serialization is fixed (G13) and records are sorted, but the approved 1.2 count is still not written down. See A2 below. | |

## Requirement → task → check

| R-id | Task | Check (rev 2) | Status |
|---|---|---|---|
| R1, R2 | 1 | `COMMIT`/`PROVENANCE.md` name `4cd46986…`; `check-corpus-frozen.py` passes | OK |
| R3, R4 | 1 | `vendored_2017_suite_matches_recorded_upstream`, plus a reproducible `diff -rq --strip-trailing-cr` | OK |
| R5, R6 | 1–7 | `w3c12_conformance`: every approved entry passes unamended or is G1-eligible; the six fix-only cases pass | OK |
| R7, R9, R10 | 3–7 | `TN-triage.md` G1 (1)–(4); `SPEC_CONTRADICTED` count pinned, failing unamended, XPASS refused; `xfail_ledger_is_empty`; `a_listed_entry_is_never_amended` | OK |
| R8 | 7, 10 | Each of the six passes unamended or meets G1; WG issue filed after approval (G3) | OK |
| R11, R12 | 8 | Both harnesses build `Report` and call `to_earl` | OK |
| R13 | 8 | Assertion count = approved SHACL-root count; `earl:automatic` | OK (A2) |
| R14 | 8 | Per-entry outcome = unamended grade | OK |
| R15 | 8 | Claimed set pinned (G5); outcome set exactly {passed, failed}; `Inapplicable` unit test | OK (NJ-5 disposed by G5) |
| R16 | 8, 9 | `w3c12_earl_keys_equal_wg_page_keys`; page check | OK, but the Task 9 entry point does not exist yet (N1) |
| R17 | 8 | `doap:name` = `"PurRDF " + CARGO_PKG_VERSION`; homepage IRI | OK |
| R18 | 8 | Description contains the 40-hex commit and `4cd46986…`; malformed or missing commit refused | GAP for 2017 (N5) |
| R19 | 8 | `earl:assertedBy <https://github.com/paudley>`, `foaf:name "Patrick Audley"` | OK |
| R20 | 8 | Two in-memory generations byte-identical; `assert_golden!`; no date predicate | OK |
| R21 | 8 | 2017 proposed subject absent; 1.2 `status` asserted; exact key-set equality | OK |
| R22, R23 | 10 (U1) | `make w3c-earl` on a clean tree writes both files; tag and W3C PR are U1 external steps | OK (A3) |
| R24, R25 | 1 | As R1; G8 constant | OK (A4) |
| R26 | 1, 11 | Full `make check` + `make conformance` recorded in `validation.md` | OK |
| R27 | 1–7 | Scoreboard; 2017 120/120 | OK |
| R28 | 10, 11 | WG issue filed after approval | GAP: `Closes #570` is unconditional (N3) |
| R29 | 3–7 | `TN-triage.md` normative clause | OK |
| R30 | 8 | Determinism, as R20 | OK |
| R31 | 8 | `scoreboard_line_equals_earl_counts` + mutation | UNFALSIFIABLE as worded (N4) |
| R32, R33 (1.2) | 9 | Chromium checks on `reports.html` at `4cd46986` | Check is falsifiable, but it depends on `make w3c-earl`, which Task 10 creates later (N1) |
| R32, R33 (2017) | 9 (U2) | `w3c_2017_earl_matches_wg_inventory` | OK (A5) |
| R34, R35 | 10 (U1) | As R22 | 2017 suite commit not checked (N5) |
| R36 | 11 | PR body reference | OK |

NEEDS-JUDGMENT: issue ledger items 1–9 are disposed (G1–G9, U1). Prior-art items 1–5 are disposed (G10/G11, Task 8, the R3 row, G14, and the #384 note under prior art). There is no Task 0.

## Findings

### BLOCKING

**N1. STRUCTURE/ordering: Task 9 runs its checks on output from `make w3c-earl`, but Task 10 creates that target.**
- Task 9 says: "Run the Chromium checks from the R32/R33 (1.2) row against the EARL from `make w3c-earl`."
- Task 10 says: "Commit: `build: make w3c-earl regenerates the W3C SHACL implementation reports`."
- The Makefile has no `w3c-earl` target today, so Task 9 cannot be run as written. The R16 page check and the R32/R33 (1.2) check depend on it.

REWRITE TO: swap the two tasks. "Task 9: The regeneration command, and WG issues for spec-contradicted cases" keeps the current Task 10 body. "Task 10: Reports-page and inventory verification" keeps the current Task 9 body. Change the contract rows: R22/R23/R34/R35 point to Task 9; R16 and R32/R33 point to Tasks 8 and 10; R27–R29 point to Tasks 1–7 and 9.

**N2. STRUCTURE (pass-1 B1, still open): Task 10 has no per-task sequence, and the final task neither publishes the plan nor defines the handoff.**
- The note after Task 1 says: "Every task from 1 to 9 then follows the same sequence: …". Task 10, which has its own commit, gets no review, push or issue update.
- Task 11 is titled "PR creation, publication and handoff", but none of steps 1–7 publishes the plan, and no step says what the handoff contains.

REWRITE TO:
- Change the note after Task 1 to: "Every task from 1 to 10 then follows the same sequence: the independent task review `TN-review.md` (`VERDICT: PASS`), a commit with the named message and explicit files (Task 10, the verification task after N1, commits only if it adds the 2017 test), a push, and an update on #570. Pushes and comments wait for Patrick's publication approval; without it the work stays local and the handoff says so."
- Add to Task 11, after step 6:
  > 6a. Publish the plan: post `plan.md` (final revision) to #570 as a comment, with the text approved by Patrick.
  > 6b. Handoff: write `$STAGE/handoff.md`. It lists every R-id with MET or OPEN and its evidence path. It also lists every pending approval or external step: the release tag, the W3C PR, any WG issue not yet filed, and the G1 reading of "ledger the test away" that Patrick has not yet confirmed.

**N3. R28: `Closes #570` is unconditional, but a `SPEC_CONTRADICTED` case whose WG issue was not filed leaves R28 unmet.**
- U1 says: "The PurRDF PR can therefore say `Closes #570`." That covers only the U1 publication steps.
- G1(4) says: "If approval is withheld, the case stays an open item in the handoff and is never called settled."
- Task 11 step 6 lists "the external steps left open (tag, W3C PR)" and leaves out unfiled WG issues.
- R28 requires "reported as failed with a WG issue". A pending draft does not meet it, so closing the issue would claim an unmet criterion.

REWRITE TO (Task 11 step 6, first bullet): "`Closes #570` only if every `SPEC_CONTRADICTED` entry has a filed WG issue (G3) or the table is empty; otherwise `Part of #570`, with each unfiled case listed as OPEN against R28." Add "any unfiled WG issue" to the "external steps left open" bullet.

**N4. R31 (pass-1 B6, still open): the mutation test as worded cannot fail, and count disagreement is not tested.**
- The R30/R31 row says: "Mutation test: flipping one record's verdict makes that test fail, not only `to_earl`'s guard."
- G11 makes both projections derive from the same records. Flipping a record therefore changes the scoreboard line and the EARL together, and `scoreboard_line_equals_earl_counts` still passes.
- R31 also requires failure "in counts", and no negative case drops or adds an assertion.

REWRITE TO: "Mutation tests act on one projection only, the serialized EARL bytes, and leave the records unchanged. Each of these four makes `scoreboard_line_equals_earl_counts` return an error naming the entry: (a) one `earl:outcome` flipped; (b) one assertion removed; (c) one extra assertion with an unknown key; (d) one key misspelled. Cases (b) and (c) cover R31's count disagreement, and (a) and (d) cover per-entry disagreement."

**N5. R18/R35: the 2017 report's suite commit is never checked.**
- The R17/R18 row says: "`doap:description` contains the configured 40-hex commit and `4cd469867a1cc6d2fff80e8a40723b26c695d67a`." That is correct only for the 1.2 file.
- G8 gives the 2017 commit only as the truncated `08adb377…`. R35 requires "the release and suite commit stated" for both reports.

REWRITE TO (R17/R18 row, and G8): "…`doap:description` contains the configured 40-hex commit and the suite commit: `4cd469867a1cc6d2fff80e8a40723b26c695d67a` for `purrdf-shacl12-earl.ttl`, and `08adb3776709a014bc3062ede793c36275b22446` (the `vectors/shacl/README.md` pin) for `purrdf-shacl-earl.ttl`. Each is asserted by its own harness test." In G8, write the 2017 commit in full.

### ADVISORY

- **A1 (R32, 1.2).** "no page errors" should name the observable: "zero Chromium `pageerror` events and zero `console` errors".
- **A2 (R13).** The plan says "the plan's revision records it", but revision 2 records only the 592 discovered cases. Either state the approved SHACL-root count now, or say "Task 1 pins it as a named harness constant used by the R13 test".
- **A3 (R23 "prepare the upstream submission PR").** U1 keeps opening the W3C PR external. A drafted W3C PR description under `contributions/data-shapes/purrdf-earl/drafts/` (file names, release placeholder, both suite commits) would let "prepare" point to an artifact.
- **A4 (R25 "current").** At Task 11, re-check `origin/gh-pages` for new commits under `shacl12-test-suite/tests/` and `data-shapes-test-suite/tests/` since `4cd46986`, and record the result in `validation.md`.
- **A5 (Task 9, 2017).** Two points:
  - "vendor `alltests.ttl` via the vendor path" names no path. No 2017 vendor script exists, and `vectors/shacl/README.md` lists the subset as `core/`, `sparql/` and `manifest.ttl`. Name the destination (for example `vectors/shacl/reports/alltests.ttl`), its source commit, the README subset update, and `check-corpus-frozen.py --update`.
  - The U2 check is a cargo test, so "No source commit unless the 2017 test lives in the repository" should be stated definitely: it does live there.
- **A6 (R7/R9).** Add the pass-1 suggestion that a `SPEC_CONTRADICTED` id naming no discovered case is refused.

Verdict vocabulary: `BLOCKED` is the caller's term for NEEDS-REVISION.

VERDICT: BLOCKED
