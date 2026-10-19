# Plan: #570, Submit W3C SHACL implementation reports (EARL) against the current test suites

Revision 2. This revision integrates the five Stage 1 lens reviews and Patrick's
decisions U1 and U2.

## Issue summary

PurRDF has no implementation report on either Data Shapes WG reports page. Its
SHACL 1.2 harness grades a suite that is 134 upstream commits stale. Neither W3C
harness emits EARL. #570 does four things:

- re-vendors the suites;
- makes every approved entry pass, or reports it failed with a WG issue that
  carries a specification rationale;
- emits deterministic EARL from the Rust harnesses through `to_earl`;
- delivers everything the W3C submission needs on PurRDF's side.

The release tag and the W3C PR itself happen later (U1).

## Binding documents

- **`.goals` / `CLAUDE.md`:**
  - RUST-FIRST.
  - NO DEFERRALS.
  - No issue or PR numbers, and no process text, in repository files.
  - Hard fails.
  - "A refusal is a claim": every added or tightened refusal is tested alongside
    a neighbouring valid case.
- **`.baseline` defaults** apply.
- **`AGENTS.md` §2 hard constraints:**
  - Tooling and tests are Rust. Under the non-Rust ratchet,
    `scripts/vendor-shacl12.py` changes only its pin line.
  - `vectors/` changes only through the vendor script and
    `check-corpus-frozen.py --update`.
  - W3C terms are spelled only through `purrdf_iri::vocab`, enforced by the
    helper-census `w3c-vocab` check.
  - PurRDF is not an ontology: no DOAP or FOAF IRIs in library `src/`.
  - Byte determinism.
  - Conformance corpora use exact counts and XPASS discipline.
  - One home per job, per `helpers-ledger.toml`.
  - Dependency versions live in the root manifest.
- **`CONTRIBUTING.md`:** `make check` *and* `make conformance` both gate.
  Commit messages are conventional commits.
- **`docs/RELEASE.md`:** tags are cut only from a synced `main` with
  `make release-tags`, so tags are outside this branch (U1).
- **W3C workspace `AGENTS.md`:** every WG-facing action needs Patrick's explicit
  approval of the concrete text. That covers W3C issues, comments, pushes and
  PRs. Page checks run in a worktree under `worktrees/data-shapes/`, never in
  the clean `data-shapes/` base checkout. Drafts go in
  `contributions/data-shapes/purrdf-earl/`.

## Governing decisions

### Patrick's decisions

- **U1 (Patrick, 2026-10-17).** Patrick's words: "this issue is separate from
  our submission later to the W3C data-shapes group" and "We'll still want to
  meet all the submission criteria we need to on this side in this issue."
  - #570 delivers: correctly keyed, deterministic EARL for both suites; subject
    metadata naming the build version, source commit and suite commit; WG-page
    and inventory verification; and one documented regeneration command,
    `make w3c-earl`.
  - Outside #570: tagging a release (`docs/RELEASE.md`) and opening the W3C PR
    with files generated from that tag by the same command.
  - So R22, R23, R34 and R35 are met up to those two publication actions, which
    are recorded as remaining external steps and never claimed. The PurRDF PR
    can therefore say `Closes #570`.
  - Patrick authorized editing #570's text to match after the final plan review
    (Task 11, step 1).
- **U2 (Patrick, 2026-10-17).** Patrick's answer: "Mechanical check
  (Recommended)". For 2017, R32 and R33 are met by
  `w3c_2017_earl_matches_wg_inventory` (Task 9). Whether to regenerate the
  static 2017 table is a question for the later W3C PR.

- **U3 (Patrick, 2026-10-17).** Patrick's answer on the visible
  `SPEC_CONTRADICTED` table, verbatim: "(1) - I assume these are issues we (the
  WG) should fix, right?"
  - Option (1) means: allow the visible table.
  - Clarified by Patrick: "This surfaces an issue of discussion in the WG". A
    case in this table is a disagreement put to the WG, not a confirmed WG
    defect. There are three possible outcomes:
    - the WG fixes the test;
    - the WG fixes or clarifies the specification;
    - the WG upholds the test, in which case PurRDF fixes its engine.
  - Patrick files the issue as a WG member, after approving its text. Draft WG
    issues present the disagreement as a question, keeping three things
    separate: the specification reading, PurRDF's observed output, and the
    approved test data.
  - Each entry is a self-clearing tripwire. When the upstream fix is re-vendored,
    the case passes, the table's unexpected-pass check (XPASS) fails, and the
    entry must be removed.

### G1: pass, or spec-contradicted

- An engine defect is fixed until the case passes against its **unamended**
  expectation.
- **Eligibility.** A case may instead be classed `SPEC_CONTRADICTED` only if
  all four hold, each recorded in `$STAGE/tasks/TN-triage.md`:
  1. The triage quotes the normative sentence (document, dated version,
     section) and shows that *no* processor conforming to it can produce the
     approved expected result. "The specification also permits PurRDF's output"
     is not enough; such a case is fixed.
  2. The failure is not an unsupported-feature or not-evaluated refusal in a
     claimed class. This rules out the four `rules-entailment-ruleset-*` cases,
     `deactivated-select-001` and `pre-binding-009`, which are fix-only.
  3. A refusal offered as correct carries its neighbouring valid case.
  4. A WG issue is drafted in the W3C workspace and Patrick has been asked to
     approve filing it. It is filed after approval (Task 10). If approval is
     withheld, the case stays an open item in the handoff and is never called
     settled.
- **Harness representation.** `SPEC_CONTRADICTED` is a new table in each W3C
  harness: case id plus clause rationale, with no WG issue number or URL in the
  repository. It is pinned by count. Its entries are graded against the
  unamended expectation, and it is an error if one passes (XPASS). Each gets its
  own scoreboard label and is emitted as `earl:failed` with an `earl:info`
  rationale. `XFAIL` stays empty and the amended-grade path is removed.
- **Patrick's visibility.** This is a visible failure category, not hiding a
  test. The issue's "don't ledger the test away" is read as "don't hide or
  pass it", which this table does not do. That reading is Patrick's to
  override.

### G2–G9: grading, keys, metadata and commits

- **G2. The six decimal-spelling cases** are triaged under G1 (Task 7); the
  result is not presumed. The amended-grade category is removed either way.
- **G3. Filed WG issues.** A WG issue counts only once filed, with Patrick's
  approval. Drafts are not counted.
- **G4. "Check them in the WG's actual reports page"** means verifying on the
  page.
- **G5. Conformance classes.** `Case12` gains `requires` (from `mf:requires`)
  and `status` (from `mf:status`). The claimed set is pinned as exactly the
  classes the SHACL root uses: Core, SPARQLValidation, NodeExpr, InferenceRules,
  SPARQLNodeExpr and SPARQLCustomFunctions. Task 1 confirms that set from
  discovery. A test asserts that every SHACL-root approved entry's classes fall
  within it. `inapplicable` is therefore never emitted; the outcome set is
  asserted to be exactly {passed, failed}.
- **G6. Keys.**
  - The WG 1.2 page collects only from `tests/manifest.ttl` (`reports.js:13`,
    `:435`), which excludes `sparql-rl/`. The 1.2 EARL therefore covers only
    approved entries reached from that SHACL root.
  - `Case12` records its discovering root, which replaces the bare `listed`
    flag.
  - Keys use the page's rule: `urn:x-shacl-test:/<path>`, where `<path>` is the
    entry IRI relative to the `tests/` root. That is a Rust port of the page's
    `testKey` (`reports.js:172-181`).
  - A test asserts exact key-set equality with the SHACL root's approved
    entries.
  - The 2017 keys use the same scheme and equal `alltests.ttl`'s approved
    subjects.
- **G7. Version and commit.** Both come from the build:
  - the version from `CARGO_PKG_VERSION`;
  - the commit from `PURRDF_W3C_EARL_COMMIT`, which must be 40 hex digits and
    is paired with `PURRDF_W3C_EARL_DIR`. Either one alone, or a malformed
    value, is a hard failure. Both are parsed once into one struct.
  - `make w3c-earl` refuses a dirty tree, sets both from
    `git rev-parse HEAD`, and runs both harness EARL tests. A later tag build
    runs the same target.
- **G8. Suite commit.** The suite commit is the vendored `gh-pages` commit
  `4cd469867a1cc6d2fff80e8a40723b26c695d67a`, recorded as a constant beside the
  vendored corpus. The 2017 suite commit is `08adb377…`, which is
  content-equal to the current `gh-pages`.
- **G9. 2017 scope and scoreboard.** The 2017 W3C subset is the 120 approved
  `core/`+`sparql/` entries. `af/` (9 PurRDF and DASH extras) is excluded by a
  discovery field, not by a path string. `w3c_conformance` prints a
  `W3C 2017 subset: passed N, spec-contradicted M of 120` line.

### G10–G13: report shape, mapping and serialization

- **G10. Subject and assertor.** `to_earl` takes caller-supplied
  descriptions: `rdf:type` classes plus predicate and term pairs, in
  deterministic order. The kit's `src/` names only EARL and RDF terms, and its
  own tests use `example.org`. The W3C harnesses (test code) supply:
  - **Subject** `<https://github.com/Blackcat-Informatics/purrdf>`: typed
    `earl:TestSubject`, `earl:Software` and `doap:Project`, with
    `doap:name "PurRDF <version>"`, `doap:homepage` (same IRI),
    `doap:programming-language "Rust"`, and `doap:description` naming the
    PurRDF commit and the suite commit.
  - **Assertor** `<https://github.com/paudley>`: typed `earl:Assertor`, named
    "Patrick Audley" via `foaf:name`.
  - The EARL terms `TestSubject`, `Software`, `Assertor` and `info` are added
    to `purrdf_iri::vocab::earl`.
- **G11. One report structure.** The enhancement TRANSFORMATION is accepted
  for the W3C SHACL harnesses and the community runner. A `Report` in
  `purrdf-conformance-kit` holds the subject, assertor, claimed classes and
  sorted records. Its EARL graph and its scoreboard totals line are two
  projections of the same records, so R31 holds by construction. A test also
  checks the projections against each other independently: the printed
  scoreboard line is parsed and compared with the EARL. `community-conformance`
  migrates to it.
- **G12. Mapping.**
  - Harness `Ok` becomes `Record` observed `Compared` = expected `Compared`.
  - Harness `Err` becomes observed `Mismatch` with the error text as message.
  - Expected-failure (`sht:Failure`) cases map expected and observed to
    `Compared` on the harness's own failure grade. `outcome::grade` requires a
    reason for `IntendedRejection`, and the W3C suites name none.
  - A harness-produced record can never yield `cantTell` or `untested`; this
    is asserted.
- **G13. Serialization.** Sorted N-Triples, written as `.ttl` because the WG
  page lists `*.ttl` and N-Triples is valid Turtle. This matches
  `community-conformance`.
  - Determinism: in-memory double generation must be byte-identical, and an
    `assert_golden!` covers `to_earl` over a fixed `example.org` fixture.
  - Changing `community-conformance`'s output changes its `earl.nt` bytes; the
    PR states this and why.
  - Files are written only under `PURRDF_W3C_EARL_DIR`, and any write error is
    a hard failure.

### Further decisions

- **G14. Licence.** The EARL is PurRDF's own output, naming test IRIs only.
  The vendored suites stay under the W3C Software and Document License per
  `vectors/*/LICENSE.md`. No relicensing is involved.
- **G15. Dev-dependency.** `purrdf-shapes` gains `purrdf-conformance-kit` as a
  dev-dependency only. `layers.toml` exempts dev edges, and the kit is
  path-only and unpublished. All EARL code stays in `tests/`. The `AGENTS.md`
  crate-map row for the kit names its new caller. If `release-crates.sh`
  carries a dev-dependency stripping list, it is updated.
- **G16. Intermediate commits.** Commits after Task 1 and before Task 7 leave
  `w3c12_conformance` failing on the named remaining cases. That is visible in
  each commit message. The PR is squash-merged through `ghprsq`, so `main`
  never sees red. `docs/CONFORMANCE.md` states current counts only, with no
  process text.

## Prior art and recurrence

- No prior attempts: 0 commits, branches or PRs.
- #384 (community corpus, cross-engine EARL) is related and is referenced only
  in GitHub text. G11's `Report` is the structure it can reuse.
- `to_earl` exists, with callers `community-conformance` and
  `conformance-kit/tests/grading.rs`. It is extended in place (one home).
- Recurrence: `GhpRsq-Defect-Class` counts are repository-wide (`none` 20,
  `performance` 3, `bug` 2, `correctness` 1, `semantic-identity-loss` 1). None
  is specific to #570.

## Measured baseline

Re-vendored to `4cd46986`, with discovery scratch-relaxed. Log:
`raw/w3c12-measurement-4cd46986.log`.

### SHACL 1.2

- **Discovery: 592 cases (was 547).**
  - Validate 174→213, RulesPositiveSyntaxTest 109→111,
    RulesPositiveStratificationTest 5→7, RulesNegativeStratificationTest 5→7.
    Other types are unchanged.
  - `xone-002` and `xone-003` are now listed upstream.
- **Grading:** 570 passed, 6 non-canonical-expected-decimal, 15 errors.
  - One is bookkeeping: the `xone-003` delta.
  - Fourteen are real:
    - `core/property/closed-001`;
    - `inference-rules/rules-entailment-ruleset-{and-all,includes,multiple,validation}`;
    - `sparql/functions/{blankNodeArgument-001,singleInstanceOf-001}`;
    - `sparql/node/{deactivated-select-001,prefixes-003,sparql-failure-001,sparql-message-001}`;
    - `sparql/pre-binding/pre-binding-009`;
    - `sparql-rl/stratification/manifest#test_13` and `#test_14`.
- **Runtime vocabulary.** The vendor script also rewrote the runtime copies
  `crates/shapes/spec/{shacl,shacl-shacl}.ttl`.

### 2017 suite

- `vectors/shacl` has 0 content differences from upstream ignoring CR, and no
  upstream commit has touched it since the pin.
- 129/129 pass: 120 W3C entries plus 9 `af/`. `alltests.ttl` has 120 approved
  entries and 1 proposed.

## Completeness contract

| R-id | Task | Executable acceptance check |
|---|---|---|
| R1, R2 | 1 | `vendor-shacl12.py` `COMMIT` and `PROVENANCE.md` name `4cd46986…`. `python3 scripts/check-corpus-frozen.py` passes for `vectors-shacl12` and `shapes-spec`. |
| R3, R4 | 1 | Test `vendored_2017_suite_matches_recorded_upstream` compares `vectors/shacl` with `scripts/conformance-frozen/vectors-shacl.sha256` (unchanged). `validation.md` records 0 CR-insensitive content differences from `4cd46986` `data-shapes-test-suite/tests`, from a reproducible `diff -rq --strip-trailing-cr` command. |
| R5, R6 | 1–7 | `cargo test -p purrdf-shapes --test w3c12_conformance`: every approved entry passes unamended, except G1-eligible `SPEC_CONTRADICTED` entries named in `TN-triage.md`. The six fix-only cases pass, with no exception. |
| R7, R9, R10 | 3, 4, 6, 7 | Per case, the `TN-triage.md` G1 (1)–(4) record exists. `SPEC_CONTRADICTED` entry tests: count pinned; each fails unamended; XPASS is refused. `xfail_ledger_is_empty` passes. `a_listed_entry_is_never_amended` passes. |
| R8 | 7 | Each of the six passes unamended, or meets G1. The `non-canonical-expected-decimal` label is gone from the scoreboard source. |
| R11, R12 | 8 | Both harnesses build `purrdf_conformance_kit::Report` and call `to_earl` (tests `w3c12_earl_*`, `w3c2017_earl_*`). |
| R13 | 8 | Assertion count equals the approved SHACL-root entry count, and every assertion has `earl:mode earl:automatic`. The approved count is pinned: Task 1 computes it from discovery and the plan's revision records it. |
| R14 | 8 | Each per-entry EARL outcome equals the unamended grade; `SPEC_CONTRADICTED` entries emit `failed`. |
| R15 | 8 | The claimed class set is pinned (G5). The outcome set is asserted to be exactly {passed, failed}. A `to_earl` unit test still covers `Inapplicable`. |
| R16 | 8, 9 | `w3c12_earl_keys_equal_wg_page_keys`: exact key-set equality using the ported `testKey`. Page check (Task 9). |
| R17, R18 | 8 | Test asserts exact values: `doap:name` equals `"PurRDF " + CARGO_PKG_VERSION`; `doap:homepage` IRI; `doap:description` contains the configured 40-hex commit and `4cd469867a1cc6d2fff80e8a40723b26c695d67a`. A missing or malformed commit is refused (negative test). |
| R19 | 8 | Every assertion's `earl:assertedBy` is `<https://github.com/paudley>`, typed `earl:Assertor`, with `foaf:name "Patrick Audley"`. |
| R20 | 8 | In-memory double generation is byte-identical. `assert_golden!` on `to_earl`. No date predicate is present. |
| R21 | 8 | No `earl:test` for `sht:proposed` entries. 2017: `alltests.ttl`'s one proposed subject is absent. 1.2: the `status` field is asserted. |
| R22, R23, R34, R35 | 10 | `make w3c-earl` on a clean tree writes `purrdf-shacl12-earl.ttl` and `purrdf-shacl-earl.ttl` naming the version and commit. Documented in `docs/CONFORMANCE.md`. The tag and the W3C PR are the U1 external steps, reported open. |
| R24–R26 | 1, 11 | One full `make check` and `make conformance` run at the end, recorded in `validation.md`. |
| R27–R29 | 1–7, 10 | Scoreboard plus triage files. 2017: 120/120. Any WG issues filed (G3) after Patrick's approval. |
| R30, R31 | 8 | `Report` projections plus test `scoreboard_line_equals_earl_counts`, which parses the printed line and the serialized EARL and compares per entry and totals. Mutation test: flipping one record's verdict makes that test fail, not only `to_earl`'s guard. |
| R32, R33 (1.2) | 9 | Chromium on the WG `reports.html` at `4cd46986`, in a worktree, with `?reports=<generated file>`. Asserts: `#messages` empty; no page errors; a PurRDF column present; zero "no data" cells for approved entries; no "Unknown tests" row; per-`mf:requires`-group passed counts equal the harness's per-class tally (G5). |
| R32, R33 (2017) | 9 | U2: `w3c_2017_earl_matches_wg_inventory` parses the EARL (parse error fails), checks key set = the 120 approved `alltests.ttl` subjects (a vendored copy of that file, pinned by hash), and checks counts = the printed `W3C 2017 subset` line. |
| R36 | 11 | Referenced in the PR body only. |

## NEEDS-JUDGMENT dispositions

- Issue ledger 1–9: G1/U1 (1), G2 (2), U1 (3), G4 (4), G5 (5), G6 (6), G7
  (7), G8 (8), G9 (9).
- Prior-art ledger:
  - `to_earl` lacks subject metadata: G10/G11.
  - WG paths are external: verified in the W3C workspace (G6, U2).
  - Licence: G14.
  - Recurrence: repository-wide.
  - `af/`: G9.
- Review findings: see "Review dispositions" at the end.

## Enhancements considered

- **Accepted:**
  - TRANSFORMATION, a shared `Report` (G11);
  - `mf:requires`/`mf:status` discovery (G5);
  - a Rust `testKey` port (G6);
  - the mapping spelled out, with a closed outcome set (G12);
  - `earl:info` rationale (G1);
  - `make w3c-earl` with commit enforcement (G7);
  - one serialization plus a golden (G13);
  - a fuller DOAP subject (G10).
- **Declined: EARL from the SPARQL, ShEx, RDFC, CSVW and JSON-LD harnesses.**
  Those reports go to different consumers (the W3C rdf-tests reports, the
  shexTest reports), each with its own key and format conventions. Each would
  need its own consumer-page verification to be honest, and no #570
  requirement asks for them. G11's `Report` makes each a pass-the-records
  change.
- **Declined: SPARQL-RL EARL.** No WG page or manifest the reports page loads
  includes `sparql-rl/`. Emitting 203 keys the page cannot match would show as
  "Unknown tests" (G6). The scoreboard keeps grading SRL in full.

## Task 1: Re-vendor the SHACL 1.2 suite and update harness bookkeeping

Commit: `test(shapes): vendor the current W3C SHACL 1.2 test suite`.

- **Vendor:**
  - keep the `COMMIT` bump;
  - regenerate `vectors/shacl12`, `PROVENANCE.md` and `crates/shapes/spec`
    with the script;
  - run `check-corpus-frozen.py --update`.
- **Discovery (`shacl_corpora/shacl12.rs`):**
  - add `root`, `requires` and `status` to `Case12`, replacing `listed`;
  - update `W3C12_TOTAL_CASES` and `W3C12_CASES_BY_TYPE`;
  - drop `xone-002` and `xone-003` from `W3C12_UNINCLUDED_MANIFESTS`;
  - update `W3C12_UNLISTED_ENTRIES` and remove the `xone-003` delta;
  - check `INVENTORY`, `INFER_INVENTORY` and `SRL_TOTAL` entry by entry
    against discovery.
- **Baseline record:**
  - add the suite-commit constant;
  - record the 2017 drift-check command and result in `validation.md`;
  - update `docs/CONFORMANCE.md` counts (no process text).
- **Focused check:** all bookkeeping and discovery tests in `w3c12_conformance`
  pass. The grading test fails on exactly the 14 named cases and reports 6
  decimal cases. `w3c_conformance` stays 129/129.

Every task from 1 to 9 then follows the same sequence: the independent task
review `TN-review.md` (`VERDICT: PASS`), a commit with the named message and
explicit files, a push, and an update on #570. Pushes and comments wait for
Patrick's publication approval; without it the work stays local and the
handoff says so.

## Task 2: SHACL 1.2 pre-binding admission

Commit: `fix(shapes): admit SHACL 1.2 VALUES clauses that bind no pre-bound variable`.

- Under the dated 1.2 profile, refuse only a VALUES clause that mentions a
  pre-bound variable. The 2017 profile's total ban stays.
- **Check:**
  - `pre-binding-009` passes;
  - the 2017 `pre-binding` cases stay 14/14;
  - a new neighbour pair under 1.2: a VALUES clause mentioning `$this` is
    refused, and one without it is admitted;
  - under the 2017 profile, both are refused.

## Task 3: SHACL-SPARQL constraint cases

Commit: `fix(shapes): <summary from the actual fixes>`.

- `deactivated-select-001` is fix-only: evaluate a node-expression value of
  `sh:deactivated` per the dated 1.2 text, and keep the neighbour case where a
  non-boolean result is a failure.
- `prefixes-003`, `sparql-failure-001`, `sparql-message-001`,
  `blankNodeArgument-001` and `singleInstanceOf-001` are each triaged in
  `T3-triage.md`, then fixed or classed under G1.
- **Check:** `deactivated-select-001` passes. Each of the other five passes
  unamended or meets G1 (1)–(4). The 2017 `sparql/` set stays all-pass.

## Task 4: Core `closed-001`

Commit: `fix(shapes): <summary>`.

- **Check:** `core/property/closed-001` passes unamended, or meets G1. The 2017
  `closed` cases still pass, and neighbouring closed or open cases are added if
  a refusal changes.

## Task 5: Inference rules `sh:entailment`

Commit: `feat(shapes): <summary>`.

- Implement the entailment declarations the four cases require, per the dated
  1.2 Inference Rules text. There is no G1 path.
- **Check:** all four pass. `rules_conformance` stays green. A neighbouring
  undefined regime still signals the failure the specification requires.

## Task 6: SPARQL-RL stratification

Commit: `fix(shapes): <summary>`.

- **Check:** `#test_13` and `#test_14` pass, or each meets G1. All positive
  stratification cases still pass. Neighbouring valid stratified rule sets are
  still accepted.

## Task 7: The six decimal-spelling cases

Commit: `fix(shapes): <summary>` or `test(shapes): grade node-expression decimals against the approved expectation`.

- Triage each case (one `shnex`, five `shnex-sparql`). Determine whether the
  dated SHACL 1.2 Node Expressions and SPARQL 1.2 texts fix the computed
  literal's lexical form.
- Fix the engine where its form is wrong; otherwise apply G1.
- Remove the amended-grade path for approved entries.
- **Check:** no amended-grade label remains. Each case passes unamended or
  meets G1. `non_canonical_expectations_are_really_non_canonical` is removed or
  rewritten so it no longer guards an amended grade.

## Task 8: A shared report structure, and EARL from both W3C harnesses

Commit: `feat(conformance-kit,shapes): emit W3C SHACL implementation reports as EARL`.

- **Kit:**
  - add `Report`, `SubjectDescription` and `AssertorDescription` (G10, G11);
  - extend `to_earl` with deterministic sorted output and `earl:info`;
  - add the scoreboard-line projection;
  - migrate `community-conformance` and `grading.rs`;
  - add the vocabulary terms;
  - add an `assert_golden!` fixture.
- **Harnesses:**
  - `w3c12_conformance` and `w3c_conformance` build `Report`s from the graded
    records (G12);
  - add the `SPEC_CONTRADICTED` table (initially empty, unless Tasks 3–7 fill
    it);
  - print the totals line, including the 2017 subset line;
  - write files under `PURRDF_W3C_EARL_DIR` (G7, G13).
- **Tests:** all of the R11–R21, R30 and R31 checks in the contract, plus:
  - the negative env-pair tests;
  - a write-error test.
- **Check:** the focused crate tests, `helper-census` (`w3c-vocab`, one-home),
  and `scripts/check-layers.py`.

## Task 9: Reports-page and inventory verification

No source commit unless the 2017 test lives in the repository, in which case
the commit is `test(shapes): check the 2017 EARL against the WG inventory`.

- **1.2:** create a W3C workspace worktree of `data-shapes` at `4cd46986` (base
  passed explicitly) and serve it. Run the Chromium checks from the R32/R33
  (1.2) row against the EARL from `make w3c-earl`. Evidence goes to
  `validation.md`, and the browser harness stays in the W3C workspace.
- **2017:** vendor `alltests.ttl` via the vendor path, pin its hash, and run
  `w3c_2017_earl_matches_wg_inventory`.

## Task 10: The regeneration command, and WG issues for spec-contradicted cases

Commit: `build: make w3c-earl regenerates the W3C SHACL implementation reports`.

- **Makefile:**
  - `w3c-earl` refuses a dirty tree;
  - it sets `PURRDF_W3C_EARL_COMMIT` from `git rev-parse HEAD` and
    `PURRDF_W3C_EARL_DIR`;
  - it runs both harness EARL tests and prints the two paths.
- **Documentation:** `docs/CONFORMANCE.md` documents the command and the two
  output files.
- **WG issues:** for each `SPEC_CONTRADICTED` case, put the drafted WG issue in
  `contributions/data-shapes/purrdf-earl/drafts/` and present it to Patrick.
  File only after his approval.
- **Check:** `make w3c-earl` on a clean tree writes both files with the HEAD
  commit, and on a dirty tree it refuses.

## Task 11: PR creation, publication and handoff

1. Edit #570's text to match U1 and U2 (authorized).
2. Run the artifact inventory.
3. Run the full `make check` and `make conformance` once and record the result
   in `validation.md`.
4. Check source cleanliness, separately from `.stage`.
5. Get a completeness review of the implementation.
6. Open the PR, which needs Patrick's push approval. It is titled
   `feat(shapes): W3C SHACL implementation reports against the current suites`.
   The body includes:
   - `Closes #570`;
   - the changes;
   - the actual validation;
   - the external steps left open (tag, W3C PR);
   - the residual risks;
   - the `earl.nt` byte change.
7. Post the "least confident" note to the issue and the PR.

## Review dispositions (revision 1 → 2)

- **Completeness:**
  - B1 per-task structure: Task 1 note.
  - B2 disputed state: G1 `SPEC_CONTRADICTED`.
  - B3 filing WG issues: Task 10 and G3.
  - B4 commit source: G7.
  - B5 page groups and errors: R32/R33 row.
  - B6 harness disagreement: R31 test.
  - B7 tag order: U1.
  - B8 licence: G14.
  - Advisories: counts pinned (R13), cantTell (G12), IRIs (G10),
    reproducible drift (R3), CONFORMANCE final (G16), worktree (Task 9).
- **Constraints:**
  - B1: G1.
  - B2: U1.
  - A1: G15.
  - A2: G10.
  - A3: G13.
  - A4: neighbour cases in Tasks 2–7.
  - A5: commit messages per task.
  - A6: G16.
  - A7: Task 9.
  - A8: in-memory and temp paths (G13).
- **Design:**
  - SRL keys and roots: G6, and the SRL decline.
  - G1 representable: G1.
  - Commit source: G7.
  - Advisories: G12, G5, G13, G9, write errors.
- **Enhancement:** items 1–9 accepted (see "Enhancements considered").
- **Adversary:**
  - F1: G1 eligibility and the task checks.
  - F2: G3 and Task 10.
  - F3: G1 representation.
  - F4: U1.
  - F5: the R32/R33 (1.2) row.
  - F6: U2.
  - F7: the R17–R19 exact-value checks.
  - F8: G5 and R15.
