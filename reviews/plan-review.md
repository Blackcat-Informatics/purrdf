# Plan final review (Stage 1): #570 Submit W3C SHACL implementation reports

- **Reviewed plan:** `/home/paudley/Active/purrdf/.worktrees/570-submit-w3c-shacl-implementation-reports/.stage/submit-w3c-shacl-implementation-reports/plan.md`, revision 3 (first pass: after the U5 addition; recheck: after the "Final review → revision 3 final" disposition).
- **Reports examined (all pass 2, against revision 2):** `reviews/plan-completeness.md` (N1–N5, A1–A6), `reviews/plan-constraints.md` (N1, P1, A1', N2–N5, A7'), `reviews/plan-design.md` (BLOCKING-4, BLOCKING-5, ADVISORY-1–7), `reviews/plan-enhancement.md` (items 1–5, two declines), `reviews/plan-adversary.md` (N1, F2, N2, F7, N3, A1).
- **Dispositions examined:** plan "Review dispositions", "Revision 2 → 3" and "Final review → revision 3 final"; user decisions U1–U5 as quoted in the plan.
- **Source checked for the U5 addition:** `Makefile:217-259` (`release-tags`: preflight chain, lightweight `git tag`, atomic push), `.github/workflows/release-cargo.yaml` (whole; asset build at 163-168, publisher call at 281-282), `scripts/publish-github-release.py:196-204, 227-281, 284-354`, `scripts/check-release-notes.py:68-90`, `scripts/test_github_release.py:186-215, 241-244, 760-795`, `docs/RELEASE.md:24-63, 310-333`, `AGENTS.md:243-249` (ratchet scope), `crates/helper-census/src/non_rust.rs:49-64` (ratchet counts `.py`, JS/TS and shell; YAML is not counted).

## 1. Fixes are real

Every BLOCKING finding of the five pass-2 reports has matching plan text:

| Finding | Plan text that discharges it |
|---|---|
| Completeness N1 (task order) | Task 9 is `make w3c-earl`; Task 10 runs the page and inventory checks "against the Task 9 output". Contract rows R16 → 8, 10; R22 → 9; R32/R33 → 10. |
| Completeness N2 (per-task structure, final task) | "Task structure (applies to every task, 1–11)"; Task 12 steps 8 (publish plan), 9, 10 (handoff). |
| Completeness N3 / Adversary F2 (`Closes` unconditional) | G3; G1 (4) "Until it is filed, R28 is unmet for that case"; Task 12 step 7 "`Closes #570` or `Refs #570`, per G3"; R27–R29 row. |
| Completeness N4 / Adversary N2 (R31 unfalsifiable) | G11 "Mutation tests act on the serialized EARL only: flip one outcome, remove one assertion, add one extra assertion"; R30/R31 row; G9a names the 1.2 EARL-scope line `W3C12 SHACL root: passed N, spec-contradicted N of K`. |
| Completeness N5 / Adversary F7 (2017 suite commit) | G8 full `08adb3776709a014bc3062ede793c36275b22446`; R17/R18 row "that file's full suite commit". |
| Constraints N1 / Design BLOCKING-5 / Enhancement 3 (`alltests.ttl` home) | Task 10: new frozen root `vectors/shacl-reports/`, README, `roots.toml`, hash manifest, `LICENSING.md` row; R3 row keeps `vectors-shacl.sha256` test hashes unchanged. |
| Constraints P1 (R10 reading) | U3 quoted; "Issue text alignment" adds the step-2 sentence. |
| Constraints N2 (`vocab::sht`) | G5 and Task 1 "add … the `vocab::sht` class constants". |
| Constraints N3, N4 | G7 Makefile conventions; G13 temp paths. |
| Constraints N5 (stale README) | G17 with U4 quoted. |
| Design BLOCKING-4 / Enhancement 1 (scoreboard consumers) | G9a names both formats, the three consumers with line references, same-commit updates, no Python growth; Tasks 1, 7, 8 each cite G9a. |
| Adversary N1 (U1 overclaim, issue text) | U1 rewritten with three named later steps including the re-check; R23 delivered as `drafts/w3c-pr.md` (Task 11); concrete "Issue text alignment"; tag placement now Patrick's U5. |
| Adversary N3 (outcome set) | G5 "Each emitted outcome is asserted to be `passed` or `failed`"; R15 row. |

Advisories adopted and verified in text: console/page-error observable (R32 row), K pinned (Task 1), `gh-pages` re-check (Task 12 step 6), unknown-id refusal (G1), `Report` in `outcome.rs` with classes in the harness, EARL-only community migration, `support/w3c_earl.rs`, `status` enum and "0 'no data' cells across all page rows", `earl:info` = rationale text, `OnceLock`, G15 unconditional with `AGENTS.md` §6 and both `release-crates.sh` lists, G1 (4) evidence separation.

## 2. Declines are justified

- EARL from the five other harnesses and SPARQL-RL EARL: both carry a technical reason and a visible cost (unverified claims; "Unknown tests" row). Sound.
- `doap:license`, `doap:release`: reasons recorded.

## 3. First-pass finding (the U5 addition)

U5 is faithfully quoted and the tag stays outside #570, which closes the U1-tag question. The expanded Task 9 was added after the lens passes and, at the first pass, had these gaps. Initial verdict before recheck: BLOCKED.

### BLOCKING-1. Task 9's release integration named one of three Python sites, never named the output files, and had no contract row

**(a) The two EARL files had no names anywhere in the plan.** The names are a contract for `asset_names`, `checksum_bytes`, the `SHA256SUMS` manifest, the Chromium `?reports=<generated file>` check, `docs/CONFORMANCE.md`, `docs/RELEASE.md` and `drafts/w3c-pr.md`. REWRITE given: name them in G13.

**(b) The asset set is defined in two Python files and one Python test, not one.** `scripts/publish-github-release.py::verify_assets` (lines 274-279) requires the `SHA256SUMS` manifest to equal the hashes of every other asset; that manifest is produced by `scripts/check-release-notes.py::checksum_bytes` (lines 68-90), which hashes exactly four fixed paths. As first written, the lane would upload seven assets and fail with "retained release checksums do not match the four distribution assets" at publication. The publisher's three count strings (234, 295, 354), `scripts/test_github_release.py:244` (`"exactly five"`) and `check-release-notes.py:149-170` (self-test fixture) also change, all under the ratchet. REWRITE given: list all three sites with no-growth, and the three self-tests as the check.

**(c) Workflow placement.** The publisher reads assets from `target/dist/` (`publish-github-release.py:315-319`) while `make w3c-earl` defaults to `$(CARGO_TARGET_DIR)/w3c-earl/`; the `$GITHUB_SHA` wording half-contradicted G7's `git rev-parse HEAD` (equal in practice, since `release-tags` creates lightweight tags, `Makefile:255-257`); the files must exist before `check-release-notes.py --checksum-output` at `release-cargo.yaml:168`. REWRITE given: `make w3c-earl W3C_EARL_DIR=target/dist` in the "Build C distribution" step, plus attestation subject-path and artifact path.

**(d) No contract row, and the issue text did not mention the deliverable.** REWRITE given: row R37 (U5) and one sentence in the issue text.

### Advisories raised at the first pass

A1 claimed-set equality; A2 kit suite-agnostic test missing from text despite the disposition; A3 per-class tally had no printed observable; A4 matrix column undecided; A5 binding `vectors/` line vs Task 10's `git show` copy; A6 G1 (4) rationale-rejection sentence; A7 `lane-env` hedge; A8 stale "U1 to U3" / "U1-tag" / U1 step 2 text; A9 `doap:bug-database` dropped without reason; A10 per-R-id handoff.

## 4. Recheck (scoped to BLOCKING-1 and A1–A10)

| Item | Status | Evidence in the current plan text |
|---|---|---|
| BLOCKING-1 (a) file names | **DISCHARGED** | G13 "File names": "`purrdf-shacl12-earl.ttl` … `purrdf-shacl-earl.ttl`. They carry no version, because the version is inside each file (`doap:name`) and a GitHub Release is already per version. Every mention of 'both files' means these two names." |
| BLOCKING-1 (b) three asset-set sites | **DISCHARGED** | U5 "Where the release asset set is spelled (three places)": `publish-github-release.py` (`asset_names`, three messages), `check-release-notes.py` (`checksum_bytes`, self-test fixture), `test_github_release.py` (`exactly five`); "All three are edited in the same commit, each with no net line growth … If no-growth is impossible, it goes back to Patrick; the ratchet is not weakened." Task 9 check names the three self-tests and "seven assets, including both EARL files", plus `helper-census --non-rust-ratchet`. |
| BLOCKING-1 (c) workflow placement | **DISCHARGED** | Task 9: "`release-cargo.yaml` runs `make w3c-earl W3C_EARL_DIR=target/dist` in the 'Build C distribution' step, before the `SHA256SUMS` line (around line 168) … the commit is `git rev-parse HEAD` exactly as in G7. That equals `$GITHUB_SHA`, because `release-tags` creates lightweight tags"; "both files are added to the attestation `subject-path` and the evidence artifact `path`". U5 "Attestation" records the default and Patrick's override. Consistent with G7's `lane-env` declaration `W3C_EARL_DIR ?= target/w3c-earl`. |
| BLOCKING-1 (d) contract row and issue text | **DISCHARGED** | Row "R37 (U5) \| 9 \| `make -n release-tags VERSION=x` shows `w3c-earl` in the preflight before any `git tag` …". Issue text step 4 now ends: "Every release from v4.0 on generates both reports in its preflight, and publishes them as verified GitHub Release assets." |
| A1 claimed-set equality | **DISCHARGED** | G5: "A test asserts that the union of every SHACL-root approved entry's classes equals the claimed set." (R15's row still says "within it"; equality implies it, so no contradiction, only a looser restatement.) |
| A2 kit suite-agnostic test | **DISCHARGED** | Task 8 kit bullet: "add a kit unit test that builds a `Report` over `example.org` IRIs in two styles, SPARQL-suite (`manifest#test`) and shexTest, and renders both projections. The decline of other-harness EARL rests on this test." |
| A3 per-class observable | **DISCHARGED** | G9a: "`W3C12 class <C>: passed n of k`, one line per claimed class, printed by the 1.2 harness"; R32/R33 (1.2) row: "each Summary row's `passed / N` equals the matching printed `W3C12 class` line". |
| A4 matrix column | **DISCHARGED** | G9a: "`spec-contradicted` is reported in the XFail/Skip column with the detail `spec-contradicted N`, never folded into Pass. The row is `ok` if and only if the harness test is green." |
| A5 binding exceptions | **DISCHARGED** | Binding documents: "with two recorded exceptions: the U4 README fix (G17); the verbatim `git show` copy that creates `vectors/shacl-reports/` (Task 10) … records the commit and path in its README." |
| A6 G1 (4) rejection | **DISCHARGED** | G1 (4): "If Patrick rejects the rationale itself, (1) is not met, and the case is fixed rather than tabled." |
| A7 `lane-env` | **DISCHARGED** | G7: "`W3C_EARL_DIR ?= target/w3c-earl` is declared through `lane-env` (`Makefile:383-412`) and exported as `PURRDF_W3C_EARL_DIR`." |
| A8 stale text | **DISCHARGED** | Header "U1 to U5"; U1 step 2 "the release lane generates and publishes both files (U5); re-run Task 10's page and inventory checks on the published assets"; disposition "N1: U1 plus 'Issue text alignment' plus U5". |
| A9 `doap:bug-database` | **DISCHARGED** | G10: "`doap:bug-database <https://github.com/Blackcat-Informatics/purrdf/issues>` (a constant)". |
| A10 handoff | **DISCHARGED** | Task 12 step 10: "`$STAGE/handoff.md` lists every R-id as MET or OPEN with its evidence path, plus the PR, the branch, the checks run, the unrun Stage 2/3 gates and the open external steps." |

No contradiction was introduced by these edits: G7, G13, U5, Task 9 and R37 agree on the directory override, the commit source and the two file names; the binding `vectors/` line now matches Tasks 1 and 10; G5, G9a and the R32 row agree on the class lines. No other area was reopened.

## 5. Open user decisions

1. **Issue text alignment.** Patrick approves the exact replacement text in "Issue text alignment" (already scheduled with the plan presentation). It now includes the U5 sentence.
2. **Attestation of the EARL assets.** Adopted as the recommended default (attested alongside the C distribution); the plan records that Patrick may override it. No action needed unless he objects.

U1-tag is closed by U5. Nothing else is pending.

VERDICT: PASS
