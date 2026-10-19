# Plan completion adversary: Stage 1

- **Reviewed plan:** `.stage/submit-w3c-shacl-implementation-reports/plan.md` (revision 2)
- **Pass:** 2 of 3. This is a recheck of pass-1 findings F1–F8 and of the text added to fix them.
- **Inputs read:**
  - the plan and the pass-1 report;
  - `issue.md` (body, 0 comments) and `issue-analysis.md` (R1–R36);
  - `raw/w3c12-measurement-4cd46986.log`;
  - repository `CLAUDE.md`, `AGENTS.md` and `vectors/shacl/README.md`;
  - in the W3C checkout `/home/paudley/Active/w3c/data-shapes` (gh-pages): `shacl12-test-suite/reports.js`, `shacl12-test-suite/tests/manifest.ttl`, `tests/sparql-rl/` and `data-shapes-test-suite/index.html`.
- **User scope decisions:** U1 and U2. Both are quoted only in the plan (Governing decisions). No other Stage file records them, and the caller states they are Patrick's.
  - U1: "this issue is separate from our submission later to the W3C data-shapes group" and "We'll still want to meet all the submission criteria we need to on this side in this issue."
  - U2: "Mechanical check (Recommended)", which is pass-1 F6 option (a).
  - The plan also says Patrick authorized editing #570's text. It does not quote his words for that authorization.

## Summary

Revision 2 fixes most of pass 1:

- G1 now has a falsifiable eligibility test and a fix-only list.
- `SPEC_CONTRADICTED` gives a visible failure path that keeps `make conformance` green.
- The 1.2 page check can no longer pass vacuously.
- U2 settles the 2017 page question.

Five defects remain:

1. **U1 overclaim and unspecified issue edit.** U1 is applied more broadly than Patrick's words support. The plan says R22, R23, R34 and R35 are "met up to" two publication actions. That silently drops one verification step, the page check on the tag-generated files. The edit to #570 that makes `Closes #570` honest is also unspecified.
2. **Disguised-deferral route through G1 (4).** G1 eligibility (4) is met by *asking* Patrick to file. Combined with an unconditional `Closes #570`, an unfiled or rejected G1 case can ship inside a closed issue.
3. **R31 mutation test cannot fail as written.** Both projections come from one `Report`, so flipping a record changes both. The 1.2 "printed line" is also unnamed, and the existing `W3C12 TOTAL` line includes SPARQL-RL entries that the EARL excludes.
4. **2017 suite commit unchecked.** No metadata check asserts the 2017 file's suite commit.
5. **Outcome-set check fails on success.** "Outcome set exactly {passed, failed}" is false in the target state (2017 at 120/120).

## Disposition of pass-1 findings

| Finding | Verdict | Evidence |
|---|---|---|
| F1 G1 eligibility could be met without fixing anything | **DISCHARGED** | G1 (1): "shows that *no* processor conforming to it can produce the approved expected result. 'The specification also permits PurRDF's output' is not enough; such a case is fixed." G1 (2) names the six fix-only cases. The R5/R6 row says "The six fix-only cases pass, with no exception." Task 5 says "There is no G1 path." The Checks for Tasks 3, 4, 6 and 7 now agree with their bodies ("passes unamended or meets G1"). The (4) residual is tracked under F2. |
| F2 A WG issue draft counted as the WG issue; no task files one | **STILL-OPEN** | Task 10 now files after approval, and G3 says "Drafts are not counted." However, eligibility (4) is met by "Patrick has been asked to approve filing it." The R7/R9/R10 row accepts the existence of the (1)–(4) record, and the R27–R29 row says "Any WG issues filed", which is true when none are filed. Task 11 says `Closes #570` with no condition. See F2 below. |
| F3 No way for a G1 case to fail visibly while `make conformance` passes | **DISCHARGED** | G1 Harness representation provides a per-harness `SPEC_CONTRADICTED` table: pinned by count, graded unamended, an error on XPASS, its own scoreboard label, and `earl:failed` with `earl:info`. `XFAIL` stays empty and the amended-grade path is removed. The R7 row tests all of this. R9 and R26 are now jointly satisfiable. |
| F4 Task 10 could not run before merge, yet Task 11 closed the issue | **DISCHARGED** | Under U1, the tag and the W3C PR are outside #570. Task 10 no longer tags; it runs `make w3c-earl` on the branch, which is executable. The new U1 text has its own defect (N1). |
| F5 Task 9 page check could pass vacuously | **DISCHARGED** | The R32/R33 (1.2) row asserts `#messages` empty, which catches load failures (`reports.js` L446, L450–458). It also asserts a PurRDF column present, which closes the no-column vacuity, and "no 'Unknown tests' row" (L379, L386), plus zero no-data cells for approved entries. The count mapping is still undefined; see N2. |
| F6 G10 substituted an acceptance criterion without a user decision | **DISCHARGED** | U2 is quoted and matches option (a). The row names `w3c_2017_earl_matches_wg_inventory`. That check fails on a parse error, compares the key set with the 120 approved `alltests.ttl` subjects, and compares counts with the G9 printed line. Advisory A1 notes that the test's location is hedged. |
| F7 R17–R19 metadata test checked presence, not content | **STILL-OPEN** | Exact `doap:name`, homepage, 1.2 suite commit and assertor values are now asserted. The 2017 file's suite commit is asserted nowhere. See F7 below. |
| F8 R15 claimed-class assertion left an `inapplicable` escape hatch | **DISCHARGED** | G5 pins the six SHACL-root classes, and G12 and R15 close the outcome set, so neither `inapplicable` nor `cantTell`/`untested` can carry a failure. The new wording has an error of its own (N3). |

## Special attention 1: is U1 applied faithfully to R22, R23, R34 and R35?

Partly.

- **What Patrick's words support.** They clearly place "our submission later to the W3C data-shapes group" outside #570. They keep "all the submission criteria we need to on this side" inside it. Moving the W3C PR out of #570 is therefore faithful, and so is the `Closes` keyword *for that reason*.
- **The tag.** Patrick did not mention a tag. A tag is a PurRDF-side action. Placing it outside #570 is the plan's own inference. The inference is defensible: `docs/RELEASE.md` allows tags only from a synced `main`, so a tag containing this work cannot exist before merge, and the tag exists for the submission. But the plan presents it as Patrick's decision, and it should be shown to him as an inference.
- **Overclaim of "met up to those two publication actions".**
  - R22 says "check them in the WG's actual reports page" of the *tag-generated* files. Task 9 checks branch-HEAD output. Re-running the checks on the tag output is not a publication action, and no list names it.
  - R23 ("prepare the upstream submission PR") is not prepared by any task. Under U1 it moves with the submission.
- **The issue edit.** `Closes #570` is honest only against edited issue text, and that text is unspecified.

Finding N1 below fixes this.

## Special attention 2: does SPEC_CONTRADICTED with eligibility (1)–(4) still permit a disguised deferral?

- **Eligibility (1)–(3) holds.** (1) requires impossibility, not mere permission, and the plan says outright that permissiveness means the case is fixed. (2) removes every self-declared unsupported-feature case. In the measurement log, the remaining eight non-decimal failures are mismatches or over-acceptances, not "not supported" refusals (log L50, L60, L64, L68, L70, L72, L76, L78).
- **Eligibility (4) is the remaining route.** A request to Patrick satisfies (4). The outcomes then diverge:
  - If Patrick declines because the rationale is wrong, the case still meets (4) and stays in the table. `make conformance` stays green and `Closes #570` closes the issue.
  - "Never called settled" does not prevent this, because no G1 case is ever settled within #570; the WG settles it.

Finding F2 below fixes this.

The SPARQL-RL EARL decline is not a descope:

- No SPARQL-RL manifest carries `mf:status` (0 occurrences under `tests/sparql-rl/`), so those entries are not "approved entries" (R13).
- The page's root manifest includes only `core/`, `inference-rules/`, `node-expr/` and `sparql/` (`tests/manifest.ttl` L7–10).
- The scoreboard still grades SPARQL-RL in full, and Task 6 still fixes `#test_13` and `#test_14`.

---

## N1. U1 overclaims R22 and R23, and the issue edit that makes `Closes #570` honest has no text. Grounds: UNFALSIFIABLE CRITERION, PRE-EMPTIVE DESCOPE (honest handoff)

Plan text:

> U1: "So R22, R23, R34 and R35 are met up to those two publication actions, which are recorded as remaining external steps and never claimed. The PurRDF PR can therefore say `Closes #570`."

> U1: "Patrick authorized editing #570's text to match after the final plan review (Task 11, step 1)."

> Task 11: "1. Edit #570's text to match U1 and U2 (authorized)."

> R22, R23, R34, R35 row: "The tag and the W3C PR are the U1 external steps, reported open."

Defects:

1. **The steps are incomplete.** Re-checking the tag-generated files on the page, which is R22's own clause, is neither a publication action nor listed.
2. **R23 is claimed but not delivered.** R23 is said to be "met up to" those actions, but no task prepares the submission PR. Under U1 it belongs to the submission.
3. **The tag's placement is the plan's inference.** It is not in Patrick's quoted words.
4. **The issue edit is unfalsifiable.** "To match U1 and U2" names no text. An edit that simply deletes the last checkbox would "match", and the authorization is unquoted.

**REWRITE TO (U1, replacing "So R22, R23, R34 and R35 are met … can therefore say `Closes #570`."):**

"#570 delivers and verifies, on `make w3c-earl` output at the PR head:

- the keys and metadata (R16–R21);
- the 1.2 page check and the 2017 inventory check (R32/R33, Task 9);
- R35's mechanism: the files state `CARGO_PKG_VERSION`, the PurRDF commit and the suite commit.

These steps remain after merge. They are outside #570 under U1, listed open by name in the PR body, and never claimed:

- (i) `make release-tags` from a synced `main` (`docs/RELEASE.md`);
- (ii) `make w3c-earl` at that tag;
- (iii) re-running the Task 9 1.2 page check and `w3c_2017_earl_matches_wg_inventory` on those tag-generated files;
- (iv) preparing and opening the W3C PR (R23, R34).

Patrick's words name only the W3C submission. Placing (i)–(iii) outside #570 is this plan's reading, based on `docs/RELEASE.md`'s rule that tags come only after merge. Patrick confirms or rejects that reading by approving the Task 11 step 1 text at the final plan review."

**REWRITE TO (Task 11 step 1):**

"1. Replace #570's step 4 and its last two acceptance criteria with exactly the following text, which Patrick approves at the final plan review.

- **Step 4:** '4. Add `make w3c-earl`, which generates both reports from a clean checkout, stating the PurRDF version, the PurRDF commit and the suite commit. Check the generated reports in the WG's actual reports pages. Tagging a release, regenerating and re-checking the reports from that tag, and preparing and opening the upstream submission PR are part of the later W3C submission, not this issue.'
- **Acceptance:**
  - '- [ ] The 1.2 EARL file loads in `shacl12-test-suite/reports.html` with no page errors, no "no data" rows for approved entries, and per-criterion counts equal to the scoreboard. The 2017 EARL file's keys and counts equal the approved entries of `data-shapes-test-suite/reports/alltests.ttl` and the 2017 scoreboard line, because the 2017 page is static.'
  - '- [ ] `make w3c-earl` generates both reports from a clean checkout, stating the PurRDF version, PurRDF commit and suite commit, and refuses a dirty tree.'

If Patrick rejects this text, the PR says `Refs #570`. #570 then stays open until steps (i)–(iii) are done and recorded in `validation.md`."

**REWRITE TO (R22, R23, R34, R35 row, acceptance column):**

"`make w3c-earl` on a clean tree writes `purrdf-shacl12-earl.ttl` and `purrdf-shacl-earl.ttl`. Each names `CARGO_PKG_VERSION`, the HEAD commit and its suite commit. On a dirty tree the command refuses. This is documented in `docs/CONFORMANCE.md`. U1 steps (i)–(iv) are listed open by name in the PR body."

## F2 (still open). G1 (4) is met by a request, and closure has no condition. Grounds: PLANNED-BY-REFUSAL, PRE-EMPTIVE DESCOPE (honest handoff)

Plan text:

> G1 (4): "A WG issue is drafted in the W3C workspace and Patrick has been asked to approve filing it. It is filed after approval (Task 10). If approval is withheld, the case stays an open item in the handoff and is never called settled."

> R7, R9, R10 row: "Per case, the `TN-triage.md` G1 (1)–(4) record exists."

> R27–R29 row: "Any WG issues filed (G3) after Patrick's approval."

> Task 11 step 6: "`Closes #570`;"

R28 requires "reported as failed with a WG issue that gives its specification rationale". Under the current text:

- A pending or declined filing still satisfies eligibility (4) and the R7 row.
- "Any WG issues filed" holds when there are none.
- The PR closes #570 regardless.

That is a case left unfixed, with no WG issue, inside a closed issue.

**REWRITE TO (G1 (4)):**

"4. The WG issue is filed on `w3c/data-shapes` after Patrick approves its exact text (Task 10), and its URL is recorded in `TN-triage.md`. A drafted or pending issue does not satisfy (4).

- If Patrick withholds approval because he disputes the rationale, (1) is not met and the case is fixed.
- If he withholds approval for any other reason, the case stays in `SPEC_CONTRADICTED`, and R27–R29 are reported unmet for that case."

**REWRITE TO (R7, R9, R10 row, acceptance column):**

"For each `SPEC_CONTRADICTED` entry, `TN-triage.md` records G1 (1)–(3) and the filed `w3c/data-shapes` issue URL (4). `SPEC_CONTRADICTED` entry tests: count pinned; each fails unamended; XPASS is refused. `xfail_ledger_is_empty` and `a_listed_entry_is_never_amended` pass."

**REWRITE TO (R27–R29 row, acceptance column):**

"Scoreboard plus triage files. 2017: 120/120. Either `SPEC_CONTRADICTED` is empty in both harnesses, or every entry has a filed WG issue URL in `TN-triage.md`. Drafts and pending requests do not count (G3)."

**REWRITE TO (Task 11 step 6, the `Closes #570` bullet):**

"`Closes #570` only when both of these hold:

- every `SPEC_CONTRADICTED` entry has a filed WG issue URL, or the table is empty;
- Patrick approved the step 1 issue text.

Otherwise the PR says `Refs #570`, and the body lists each unmet item by case id and R-id."

## N2. The R31 mutation test cannot fail as written, and the 1.2 comparison line is unnamed. Ground: UNFALSIFIABLE CRITERION

Plan text:

> G11: "Its EARL graph and its scoreboard totals line are two projections of the same records, so R31 holds by construction. A test also checks the projections against each other independently: the printed scoreboard line is parsed and compared with the EARL."

> R30, R31 row: "test `scoreboard_line_equals_earl_counts`, which parses the printed line and the serialized EARL and compares per entry and totals. Mutation test: flipping one record's verdict makes that test fail, not only `to_earl`'s guard."

> R32/R33 (1.2) row: "per-`mf:requires`-group passed counts equal the harness's per-class tally (G5)."

Defects:

- **The mutation cannot fail.** Flipping a `Report` record changes both projections the same way, so the comparison still passes. The mutation acceptance can only be met by reinterpreting it.
- **The 1.2 line is unnamed.** The existing `W3C12 TOTAL` line (log L44) counts the SPARQL-RL entries, which G6 excludes from the EARL, so it can never equal the EARL. G9 names the 2017 line; the plan names no 1.2 equivalent.
- **"Per entry" has nothing to compare.** A totals line carries no per-entry data.
- **The per-class tally is undefined.** The page assigns an entry's group from `mf:requires`, or else from a folder default (`reports.js` L29–33, L200–204). G5 defines `requires` only from `mf:requires`. An entry with no `mf:requires` therefore also passes G5's "classes fall within" check vacuously.

**REWRITE TO (R30, R31 row, acceptance column):**

"`w3c12_conformance` prints `W3C 1.2 SHACL root: passed N, spec-contradicted M of K`. It also prints one line per conformance class: `W3C 1.2 class <C>: passed n of k`. Each entry's classes come from `mf:requires`, or, if it has none, from the page's folder default (`core`→Core, `inference-rules`→InferenceRules, `node-expr`→NodeExpr). `w3c_conformance` prints the G9 2017 line.

`scoreboard_line_equals_earl_counts` parses those printed lines and the serialized EARL, and asserts equal totals and per-class counts. Per-entry agreement is R14's check.

Mutation tests target the comparison itself:

- flipping one `earl:outcome` in the serialized EARL makes it fail;
- deleting one assertion from the serialized EARL makes it fail."

**REWRITE TO (R32/R33 (1.2) row, last assertion):**

"Each Summary row's `passed / N` equals the matching printed `W3C 1.2 class <C>: passed n of k` line."

## F7 (still open). The 2017 file's suite commit is asserted nowhere. Ground: UNFALSIFIABLE CRITERION

Plan text:

> R17, R18 row: "`doap:description` contains the configured 40-hex commit and `4cd469867a1cc6d2fff80e8a40723b26c695d67a`."

R18 and R30 cover both suites, and G8 names `08adb377…` as the 2017 suite commit. With this row, a 2017 description that omits its suite commit, or names `4cd46986` instead, still passes. The row also does not say the assertions run on both files.

**REWRITE TO (R17, R18 row, acceptance column):**

"The following is asserted on each file (`w3c12_earl_*` and `w3c2017_earl_*`):

- `doap:name` equals `"PurRDF " + CARGO_PKG_VERSION`;
- `doap:homepage` is `<https://github.com/Blackcat-Informatics/purrdf>`;
- `doap:description` contains the configured 40-hex PurRDF commit and that file's suite commit: `4cd469867a1cc6d2fff80e8a40723b26c695d67a` in the 1.2 file, and `08adb3776709a014bc3062ede793c36275b22446` (from `vectors/shacl/README.md`) in the 2017 file.

A missing or malformed commit is refused (negative test)."

## N3. "Outcome set exactly {passed, failed}" fails in the target state. Ground: UNFALSIFIABLE CRITERION

Plan text:

> G5: "the outcome set is asserted to be exactly {passed, failed}."

> R15 row: "The outcome set is asserted to be exactly {passed, failed}."

The R27–R29 row's target for 2017 is 120/120, and the 1.2 target is an empty `SPEC_CONTRADICTED` table. In both cases the outcome set is {passed}. Read literally, the R15 check fails exactly when the work succeeds, so it has to be reinterpreted.

**REWRITE TO (G5 and R15 row):**

"Every assertion has exactly one `earl:outcome`, which is `earl:passed` or `earl:failed`. `earl:failed` occurs exactly for the `SPEC_CONTRADICTED` entries."

---

## Advisory (non-blocking)

**A1. Task 9 hedges where the 2017 check lives.** Task 9 says "No source commit unless the 2017 test lives in the repository" and "vendor `alltests.ttl` via the vendor path".

- There is no 2017 vendor script. The only one is `scripts/vendor-shacl12.py`, and `vectors/shacl/README.md` names no script.
- `alltests.ttl` sits in `data-shapes-test-suite/reports/`, outside the vendored `tests/` subset.

The check itself is defined either way, so this is not a completeness defect. The constraints reviewer should confirm the vendoring route, and Task 9 should name one location.

## Items checked and cleared in revision-2 text

- **SPARQL-RL EARL decline:** not a descope; see Special attention 2.
- **Declined EARL from other harnesses:** no #570 requirement asks for it.
- **Proposed entries in 1.2:** the 1.2 suite has no `sht:proposed` entries (0 occurrences under `shacl12-test-suite/tests`). R16's exact key-set equality with the approved entries therefore covers R21's 1.2 half.
- **G12 mapping:** the closed record mapping is falsifiable as stated.
- **G1 Patrick's visibility:** the "don't ledger the test away" reading is surfaced to Patrick, not hidden.
- **U2:** applied faithfully.

## Required user decision

There is no new decision for this pass. N1's rewrite puts concrete #570 text in front of Patrick at the final plan review, which the plan already schedules (Task 11, step 1). His approval or rejection settles where the tag belongs. If he rejects the text, the rewrite's own fallback applies: `Refs #570`.

Role-protocol status: FINDINGS-OPEN. This is pass 2 of 3, not an escalation. The caller asked for the PASS or BLOCKED vocabulary, so it is reported as BLOCKED.

VERDICT: BLOCKED
