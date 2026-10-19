# Plan enhancement audit (pass 2) — #570 Submit W3C SHACL implementation reports (EARL)

Reviewed plan: `/home/paudley/Active/purrdf/.worktrees/570-submit-w3c-shacl-implementation-reports/.stage/submit-w3c-shacl-implementation-reports/plan.md` (revision 2)

Inputs read this pass: plan revision 2, the pass-1 report, `issue-analysis.md`,
`prior-art-assessment.md`; in the worktree `scripts/conformance-matrix.py` (`_scrape`,
`_no_scoreboard`, the SHACL rows at 263-320, self-test fixtures at 1719-1750),
`scripts/check-doc-claims.py` (3166-3271), `scripts/check-corpus-frozen.py` and
`scripts/conformance-frozen/roots.toml`, `scripts/vendor-*` (listing), `vendor-shacl12.py:41`,
`Makefile` (156, 478-479), `AGENTS.md` (221-248), `crates/shapes/tests/w3c12_conformance.rs`
(scoreboard lines 693-705, 783), `crates/shapes/src/shacl_corpora/mod.rs` (86-129),
`vectors/shacl12/tests/manifest.ttl`; in the W3C workspace
`data-shapes/shacl12-test-suite/reports.js` (28-33, 186-215, 340-399) and `README.md` (14-30).
No web tools were used.

Facts established this pass:

- The worktree's `vendor-shacl12.py` pin is already `4cd46986…` and the vendored tree carries
  `mf:requires` on the `sparql/` entries (`SPARQLValidation`, `SPARQLNodeExpr`,
  `SPARQLCustomFunctions`); `core/`, `inference-rules/` and `node-expr/` entries carry none and
  get their class from the page's `DEFAULT_CRITERIA` folder table (`reports.js:29-33`), with
  `'Unspecified'` for any other folder (`reports.js:203`). The README also lists `SRLEvaluation`
  and `SRLSyntax` as criteria that the SHACL root never reaches.
- `vectors/shacl12/tests/manifest.ttl` includes only `core/`, `inference-rules/`, `node-expr/`
  and `sparql/` (lines 7-10). `sparql-rl/` is not reachable from the page's root.
- `conformance-matrix.py` `_scrape` (215-232) does `re.search(pattern, out)` and, on no match,
  returns `_no_scoreboard`, a hard-RED row with `failed=1`. The SHACL 1.2 row's pattern
  (296-297) is `W3C12 TOTAL: passed (\d+), non-canonical-expected-decimal (\d+), xfailed (\d+),
  ledger (\d+)`; the 2017 row's (268) is `TOTAL: passed (\d+), xfailed (\d+), ledger (\d+)`. The
  self-test fixture at 1738 repeats the 1.2 line verbatim; `make check` runs `--self-test`
  (Makefile:156) and `make conformance` runs the matrix (Makefile:479).
- `check-doc-claims.py` `load_shacl12_category_pins` (3227-3240) reads the non-canonical table's
  pin from the harness source and `shacl12_claims` (3252-3271) requires the matrix XFail/Skip
  column to equal it. `docs/CONFORMANCE.md`'s matrix block is generated (`--write-doc`,
  3504), not hand-written.
- `vectors/shacl` is a frozen root (`roots.toml:10`, receipt `vectors-shacl.sha256`) and no
  `scripts/vendor-*` script exists for it; `check-corpus-frozen.py` only hashes.
- The non-Rust ratchet (AGENTS.md 244-246) forbids a ratcheted non-Rust path holding MORE lines
  than BASE; same-count edits are allowed.

## Pass-1 disposition table

| # | Pass-1 item | Where folded in | Status |
|---|---|---|---|
| 1 | TRANSFORMATION: suite-agnostic `Report` in the kit; EARL and scoreboard as projections; `community-conformance` migrates | G11; Task 8 "Kit" bullets; R30/R31 row (`scoreboard_line_equals_earl_counts` plus the mutation test); G15 dev-dependency | **DISCHARGED** for the structure and both projections. Two pieces of the item are not in any task or check and are carried forward as item 4 below: the kit unit test over a synthetic non-SHACL manifest (the only evidence for the "pass-the-records" claim the first decline rests on), and the per-entry criteria the scoreboard projection needs to produce the per-class tallies Task 9 compares. |
| 2 | Disputed category keeping the exact-count gate green | G1 "Harness representation": `SPEC_CONTRADICTED` table, pinned count, XPASS is an error, own label, `earl:failed` + `earl:info`, XFAIL empty, amended path removed; R7/R9/R10 and R8 rows | **DISCHARGED** |
| 3 | Read `mf:requires`/`mf:status`; tally per criterion | G5; Task 1 discovery (`root`, `requires`, `status` on `Case12`); R15, R21; R32/R33 (1.2) per-`mf:requires`-group comparison | **DISCHARGED** in substance. The page's folder default and the exactness of the claimed-set test are not stated; see item 2 below. |
| 4 | Rust `testKey` port and key-set equality | G6; R16 `w3c12_earl_keys_equal_wg_page_keys`; Task 9 "no Unknown tests row" | **DISCHARGED** |
| 5 | Spell the `Observation` mapping; close the outcome set | G12; R15 "outcome set is exactly {passed, failed}" | **DISCHARGED** |
| 6 | `earl:info` rationale on failed results | G1; G10 vocabulary additions | **DISCHARGED** |
| 7 | Machine-enforce clean tree and recorded commit | G7 (env pair, 40-hex check, either-alone hard failure); Task 10 `make w3c-earl` refuses a dirty tree; R17/R18 negative test | **DISCHARGED** as adapted: U1 moves the tag outside #570, so the tag precondition is rightly gone; the clean-tree and commit checks remain enforced. |
| 8 | One serialization plus a golden | G13 (sorted N-Triples as `.ttl`, `assert_golden!` over a fixed fixture, double-generation check kept) | **DISCHARGED**. The golden is over a fixture rather than the full W3C graph; the exact-count gate already makes any verdict flip a test failure, so nothing is lost. |
| 9 | Fuller DOAP subject | G10: `doap:name`, `doap:homepage`, `doap:programming-language`, `doap:description` | **STILL-OPEN (minor)**: `doap:release [ doap:revision ]`, `doap:license`, `doap:repository`, `doap:bug-database` and `doap:developer` were dropped with no technical reason recorded. See item 5. Not blocking on its own. |

## The two declines

- **EARL from the SPARQL, ShEx, RDFC, CSVW and JSON-LD harnesses.** Compelling. Each consumer
  has its own key and format conventions and an implementation report is only a claim once it is
  verified against the page that reads it; adding five unverified files to a SHACL issue would be
  five unsupported claims. The decline's supporting statement, "G11's `Report` makes each a
  pass-the-records change", is only true if the kit proves it is suite-agnostic; item 4 asks for
  that one test so the decline stands on evidence.
- **SPARQL-RL EARL.** Compelling and verified: `tests/manifest.ttl` includes only the four
  non-SRL folders, so the page would list every SRL key under "Unknown tests". The `root` field on
  `Case12` (G6) means a later upstream inclusion of `sparql-rl/` is picked up by discovery without
  a design change. No further action.

## Items

### 1. ROBUSTNESS (blocking) — the scoreboard line has three Python consumers the plan never names, and the planned relabelling turns `make conformance` RED

- **Enhancement.** R8 removes the `non-canonical-expected-decimal` label from the `W3C12 TOTAL`
  line and G1/G9 add `spec-contradicted` labels to both harnesses' lines. Three gated consumers
  read those lines by exact text:
  - `scripts/conformance-matrix.py` 296-297 (1.2 row) and 268 (2017 row): on no match
    `_no_scoreboard` returns a hard-RED row, so `make conformance` fails, which R24-R26 and
    `CONTRIBUTING.md` require to pass. The self-test fixture at 1738 also fails `make check`.
  - `scripts/check-doc-claims.py` 3227-3271: reads the non-canonical table pin from
    `w3c12_conformance.rs` and asserts the XFail/Skip column equals it; both the pin name and the
    column rule change.
  - `docs/CONFORMANCE.md`: its matrix block is generated by the matrix script, so Task 1's "update
    `docs/CONFORMANCE.md` counts" must regenerate it, not hand-edit it, or `check-doc-claims`
    reports the mismatch.
  Add to Task 7 (where the label disappears) and Task 8 (where the new labels appear): update the
  two regexes and `build` lambdas, the self-test fixtures, the `check-doc-claims` pin loader to
  read `SPEC_CONTRADICTED`'s pin, and regenerate the matrix block. Decide the column:
  `spec-contradicted` must never be folded into Pass; the consistent choice with today's handling
  of a non-passing-but-pinned category is the XFail/Skip column with the detail naming the count
  as "spec-contradicted" and the row `ok` iff the harness test is green. All edits are same-line
  or fewer edits to ratcheted Python, which AGENTS.md 244-246 permits (no growth).
- **Cost.** S.
- **Payoff.** Without it the plan's own gate (R24-R26, final `make check` + `make conformance`)
  cannot pass; with it the matrix, the docs and the EARL all restate the same `Report` counts.
- **Plan change.** Task 7 "Check" and Task 8 "Check" lists; the R24-R26 row; G16's
  "`docs/CONFORMANCE.md` states current counts only" becomes "is regenerated by the matrix".

### 2. ROBUSTNESS — mirror the page's class-assignment rule exactly, and make the claimed-set test an equality

- **Enhancement.** `reports.js:200-203` assigns an entry without `mf:requires` the class of its
  top-level folder (`core`→Core, `inference-rules`→InferenceRules, `node-expr`→NodeExpr) and
  otherwise `Unspecified`. The vendored tree at `4cd46986` has `mf:requires` on `sparql/` entries
  only, so most entries take the default. G5 says `requires` comes "from `mf:requires`" and says
  nothing about the default; Task 9's "per-`mf:requires`-group passed counts equal the harness's
  per-class tally" is then not like with like. Port the three-row default and the `Unspecified`
  fallback into discovery, and assert `Unspecified` never occurs in the SHACL root. G5 also says
  the claimed set is "exactly the classes the SHACL root uses" while the test asserts only that
  each entry's classes "fall within it"; make it set equality so an unused claim is a failure too.
- **Cost.** S.
- **Payoff.** The R32/R33 (1.2) per-group comparison is deterministic; no browser pass is needed
  to find a mis-grouped entry.
- **Plan change.** G5 second sentence and Task 1 discovery bullet; R15 row.

### 3. ROBUSTNESS — Task 9's "vendor `alltests.ttl` via the vendor path" has no path

- **Enhancement.** There is no vendor script for the 2017 suite (`scripts/vendor-*` lists none
  for `vectors/shacl`), `vectors/shacl` is a frozen root whose receipt R3 asserts is *unchanged*,
  and the R3 drift check compares `vectors/shacl` with upstream `tests/`, whereas `alltests.ttl`
  lives in upstream `reports/`; adding it under `vectors/shacl` would both change the receipt and
  create a drift. Put the pinned copy where U2's mechanical check lives: a fixture under
  `crates/shapes/tests/` with its SHA-256 asserted in `w3c_2017_earl_matches_wg_inventory` and its
  upstream commit named beside the suite-commit constant. `vectors/` is untouched and the
  "vectors change only through the vendor script" rule is not strained.
- **Cost.** S.
- **Payoff.** Task 9 becomes executable without a constraint exception.
- **Plan change.** Task 9 "2017" bullet; R32/R33 (2017) row ("a vendored copy of that file").

### 4. LEVERAGE — finish item 1: prove the kit is suite-agnostic and let the per-class tally be a projection

- **Enhancement.** Add the kit unit test from pass-1 item 1: a `Report` over an `example.org`
  SPARQL-style `manifest#test` IRI and a shexTest-style IRI, with its own subject and criteria,
  rendering EARL and a scoreboard line, so nothing SHACL-specific lives in the kit. Give the
  report's entry a `criteria: BTreeSet<String>` so `scoreboard()` can emit the per-criterion
  tallies Task 9 compares with the page; otherwise the per-class tally is computed beside the
  report in the harness and R31's "by construction" does not cover it.
- **Cost.** S.
- **Payoff.** The first decline's justification becomes a tested fact; the next harness to adopt
  EARL copies one test.
- **Plan change.** Task 8 "Kit" bullets and "Tests" list; G11.

### 5. UTILITY — the remaining DOAP terms (pass-1 item 9)

- **Enhancement.** Supply `doap:release [ a doap:Version ; doap:revision "<version>" ]`,
  `doap:license`, `doap:repository`, `doap:bug-database` and `doap:developer <https://github.com/paudley>`
  as caller data in the harness test code. All are constants or already-available strings; none
  is a timestamp. If the planner has a reason to omit them, record it.
- **Cost.** S.
- **Payoff.** The report doubles as PurRDF's machine-readable project description, which the WG
  and downstream catalogues read even though the page ignores it.
- **Plan change.** G10 "Subject" bullet.

No further TRANSFORMATION item: the pass-1 structure was adopted and the plan is maximal at that
altitude. None of the items above conflicts with `.goals` (Rust-first is preserved: the Python
edits in item 1 shrink or hold line counts and are forced by the ratcheted scripts' existing
role; everything hard-fails; nothing is optional).

VERDICT: BLOCKED
