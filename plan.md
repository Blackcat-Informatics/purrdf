# Plan: #570, Submit W3C SHACL implementation reports (EARL) against the current test suites

Revision 3. This revision integrates two passes of the five Stage 1 lens reviews and
Patrick's decisions U1 to U6. Revision 2 is kept at `raw/plan-rev2.md`.

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
    `check-corpus-frozen.py --update`, with two recorded exceptions:
    - the U4 README fix (G17);
    - the verbatim `git show` copy that creates `vectors/shacl-reports/`
      (Task 10). It follows `vectors/shacl`'s hand-vendored precedent and
      records the commit and path in its README.
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
  - #570 also prepares the upstream submission (R23): a drafted W3C PR
    description in `contributions/data-shapes/purrdf-earl/drafts/w3c-pr.md`
    (Task 11). It is not opened.
  - Outside #570, as three later steps:
    1. tag a release through `docs/RELEASE.md`, which allows tags only on a
       synced `main` after merge;
    2. the release lane generates and publishes both files (U5); re-run Task
       10's page and inventory checks on the published assets;
    3. open the W3C PR.

    Patrick confirmed this as U5 (below).
  - R22, R34 and R35 are reported met up to these three steps, which are listed
    as open in the handoff and never claimed.
  - Patrick's authorization to edit the issue, verbatim: "You can edit the
    issue when we finish the plan to align." The exact replacement text is in
    "Issue text alignment" below and is shown to Patrick with the final plan.
  - The PR says `Closes #570` only under the rule in G3.
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

- **U5 (Patrick, 2026-10-17).** On whether the tag is part of the later
  submission step, Patrick's answer, verbatim: "(1) v4.0 is coming up as soon
  as we clear the backlog, make sure the w3c-earl generation is part of the
  normal release process please."
  - The tag stays outside #570.
  - #570 makes EARL generation part of the normal release (Task 9):
    - `make release-tags` runs `make w3c-earl` in its preflight, as a gate on
      the clean, synced `main` commit about to be tagged;
    - `release-cargo.yaml` generates both EARL files on the tagged commit;
    - they are published as verified GitHub Release assets.
  - **Where the release asset set is spelled (three places):**
    - `scripts/publish-github-release.py`: `asset_names`, plus the count
      wording in its three messages;
    - `scripts/check-release-notes.py`: `checksum_bytes` and its self-test
      fixture;
    - `scripts/test_github_release.py`: the `exactly five` assertion.

    All three are edited in the same commit, each with no net line growth.
    For example, both scripts can read one shared asset list, or derive the
    counts from `asset_names` instead of spelling them. If no-growth is
    impossible, it goes back to Patrick; the ratchet is not weakened.
  - **Attestation.** Both files are attested with the C distribution's
    `attest-build-provenance` step and added to the `license-evidence-cargo-c`
    artifact list, so they carry the same provenance as the tarballs. This is
    the recommended default; Patrick can override it.
  - `docs/RELEASE.md` documents the step.
  - The v4.0 release will therefore carry submittable reports.

- **U6 (Patrick, 2026-10-17).** Patrick's answer, verbatim: "Permission
  granted to run the entire stage1/2/3 workflow against this issue on purrdf,
  that prohibition applies to repos that we do not OWN (like W3C ones!)".
  - Pushes, #570 comments, PR creation and the Stage 2/3 workflow on PurRDF are
    authorized.
  - Every action in `w3c/data-shapes` or `paudley/data-shapes` (issues, PRs,
    comments, pushes) still needs Patrick's explicit approval of the concrete
    text.
  - The "Task structure" step 4 approval condition is satisfied by this
    decision.

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
  4. A WG issue is drafted in the W3C workspace as a discussion question (U3).
     It keeps the specification reading, PurRDF's observed output and the
     approved test data separate. Patrick approves its text before it is filed
     (Task 11). Until it is filed, R28 is unmet for that case (G3). If Patrick
     rejects the rationale itself, (1) is not met, and the case is fixed rather
     than tabled.
- **Harness representation.** `SPEC_CONTRADICTED` is a new table in each W3C
  harness: case id plus clause rationale, with no WG issue number or URL in the
  repository. It is pinned by count. Its entries are graded against the
  unamended expectation, and it is an error if one passes (XPASS). Each gets its
  own scoreboard label and is emitted as `earl:failed` with an `earl:info`
  rationale. `XFAIL` stays empty and the amended-grade path is removed.
- **Patrick's decision.** U3 covers this table: Patrick chose option (1),
  allowing the visible table. Rationale text is a citation of the specification
  clause plus the case's observable difference. It contains no process wording
  and no issue numbers.
- **Unknown ids.** A table id that matches no discovered case is refused.
- **Triage as findings.** Each `TN-triage.md` is written so it can become a WG
  discussion item with minimal editing: clause, case, expected result, what a
  conforming processor must or may produce, and the open question. This
  applies even when the case is fixed in the engine: any specification text
  that made the case harder than it should be is noted as a candidate editorial
  clarification. Corners found here feed #384's proposed-test candidates. Both
  are recorded in GitHub and workspace text, never in repository files.

### G2–G9: grading, keys, metadata and commits

- **G2. The six decimal-spelling cases** are triaged under G1 (Task 7); the
  result is not presumed. The amended-grade category is removed either way.
- **G3. Filed WG issues, and closing the issue.** A WG issue counts only once
  filed, with Patrick's approval; drafts are not counted. The PurRDF PR says
  `Closes #570` only if `SPEC_CONTRADICTED` is empty, or every entry has a
  filed WG issue. Otherwise it says `Refs #570`, and the unfiled entries stay
  listed as open on #570.
- **G4. "Check them in the WG's actual reports page"** means verifying on the
  page.
- **G5. Conformance classes.** `Case12` gains `requires` (from `mf:requires`)
  and `status` (from `mf:status`). The claimed set is pinned as exactly the
  classes the SHACL root uses: Core, SPARQLValidation, NodeExpr, InferenceRules,
  SPARQLNodeExpr and SPARQLCustomFunctions. Task 1 confirms that set from
  discovery. The class names are added as `purrdf_iri::vocab::sht` constants,
  because `w3c-vocab` refuses string spellings.
  - An entry's classes follow the page's rule (`reports.js:200-203`): its
    `mf:requires` values, or, when there are none, the folder default from
    `DEFAULT_CRITERIA` (core → Core, inference-rules → InferenceRules,
    node-expr → NodeExpr). These are ported as one Rust function.
  - A test asserts that the union of every SHACL-root approved entry's
    classes equals the claimed set.
  - `inapplicable` is therefore never emitted. Each emitted outcome is asserted
    to be `passed` or `failed`.
  - `status` is a closed enum (approved or proposed); any other value is a hard
    failure.
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
    subjects. That file is vendored as a new frozen root,
    `vectors/shacl-reports/`, by Task 10.
- **G7. Version and commit.** Both come from the build:
  - the version from `CARGO_PKG_VERSION`;
  - the commit from `PURRDF_W3C_EARL_COMMIT`, which must be 40 hex digits and
    is paired with `PURRDF_W3C_EARL_DIR`. Either one alone, or a malformed
    value, is a hard failure. Both are parsed once into one struct.
  - `make w3c-earl`:
    - follows Makefile conventions: listed in `.PHONY`, a `##` help line,
      `--locked`;
    - refuses a dirty tree;
    - sets the commit from `git rev-parse HEAD`;
    - writes under `$(CARGO_TARGET_DIR)/w3c-earl/` by default, so it never
      dirties the tree;
    - runs both harness EARL tests and prints both paths.

    A later tag build runs the same target. `W3C_EARL_DIR ?= target/w3c-earl`
    is declared through `lane-env` (`Makefile:383-412`) and exported as
    `PURRDF_W3C_EARL_DIR`.
- **G8. Suite commit.** The suite commit is the vendored `gh-pages` commit
  `4cd469867a1cc6d2fff80e8a40723b26c695d67a`, recorded as a constant beside the
  vendored corpus. The 2017 suite commit is
  `08adb3776709a014bc3062ede793c36275b22446`, the pin in
  `vectors/shacl/README.md`; its content equals the current `gh-pages`. Each
  EARL file's `doap:description` names its own suite commit, and a test
  asserts the full 40-hex value for both files.
- **G9. 2017 scope and scoreboard.** The 2017 W3C subset is the 120 approved
  `core/`+`sparql/` entries. `af/` (9 PurRDF and DASH extras) is excluded by a
  discovery field, not by a path string.
- **G9a. Scoreboard lines.**
  - The existing `TOTAL:` line (2017) keeps its format.
  - `W3C12 TOTAL:` becomes
    `W3C12 TOTAL: passed N, spec-contradicted N, xfailed N, ledger N`, with
    the decimal label removed (Task 7) and the new label added (Task 8).
  - New EARL-scope lines:
    - `W3C 2017 subset: passed N, spec-contradicted N of 120`;
    - `W3C12 SHACL root: passed N, spec-contradicted N of K`, where K is the
      approved SHACL-root count, pinned in Task 1;
    - `W3C12 class <C>: passed n of k`, one line per claimed class, printed by
      the 1.2 harness.
  - In `conformance-matrix.py`, `spec-contradicted` is reported in the
    XFail/Skip column with the detail `spec-contradicted N`, never folded into
    Pass. The row is `ok` if and only if the harness test is green.
  - Each task that changes a line updates its consumers in the same commit:
    - `scripts/conformance-matrix.py` (the regexes near lines 268 and 296, and
      the self-test fixture near line 1738);
    - `scripts/check-doc-claims.py` (the non-canonical table pin near lines
      3219–3284);
    - the generated matrix block in `docs/CONFORMANCE.md`, regenerated with its
      generator rather than hand-edited.

    The Python edits must not grow those files' line counts (non-Rust
    ratchet).

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
  - **Further subject terms:** `doap:repository` (the git repository IRI),
    `doap:developer <https://github.com/paudley>`, and `doap:bug-database
    <https://github.com/Blackcat-Informatics/purrdf/issues>` (a constant).
    - Declined: `doap:license`. PurRDF's multi-licence expression
      (`MIT OR Apache-2.0 OR MulanPSL-2.0`) has no single DOAP licence IRI.
    - Declined: `doap:release`. The version is already in `doap:name`, which is
      what the page displays.
  - **Assertor** `<https://github.com/paudley>`: typed `earl:Assertor`, named
    "Patrick Audley" via `foaf:name`.
  - The EARL terms `TestSubject`, `Software`, `Assertor` and `info` are added
    to `purrdf_iri::vocab::earl`.
- **G11. One report structure.** The enhancement TRANSFORMATION is accepted
  for the W3C SHACL harnesses and the community runner. A `Report` in
  `purrdf-conformance-kit/src/outcome.rs` holds the subject, the assertor and
  the sorted records. Claimed classes stay a harness assertion, because
  `Record` carries no class. Its EARL graph and its EARL-scope scoreboard line
  are two projections of the same records.
  - **R31 is tested independently of that shared source.** The harness parses
    the *printed* EARL-scope line and the *serialized* EARL, then compares
    per-entry outcomes and totals with its own scoreboard tally. The tally is
    computed from the match arms before any `Report` exists.
  - **Mutation tests act on the serialized EARL only:** flip one outcome,
    remove one assertion, add one extra assertion. Each must make the
    comparison fail.
  - **Migration.** `community-conformance` migrates for its EARL output only;
    its `COMMUNITY-SHACL` scoreboard line is unchanged.
  - **Shared harness code.** The shared harness-side code (subject and
    assertor, env parsing, key port, file writing) lives in one
    `crates/shapes/tests/support/w3c_earl.rs` module that both harnesses
    include.
  - **Grading runs once** per harness binary, behind a `OnceLock`.
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
  - **File names.** The 1.2 file is `purrdf-shacl12-earl.ttl` and the 2017
    file is `purrdf-shacl-earl.ttl`. They carry no version, because the
    version is inside each file (`doap:name`) and a GitHub Release is already
    per version. Every mention of "both files" means these two names.
  - Determinism: in-memory double generation must be byte-identical, and an
    `assert_golden!` covers `to_earl` over a fixed `example.org` fixture.
  - Changing `community-conformance`'s output changes its `earl.nt` bytes; the
    PR states this and why.
  - Files are written only under `PURRDF_W3C_EARL_DIR`, and any write error is
    a hard failure; its test uses `purrdf_testkit` temporary paths.
  - `earl:info` holds exactly the table's rationale text, never machine paths
    or harness error text, so the bytes are machine-independent.

### Further decisions

- **G14. Licence.** The EARL is PurRDF's own output, naming test IRIs only.
  The vendored suites stay under the W3C Software and Document License per
  `vectors/*/LICENSE.md`. No relicensing is involved.
- **G15. Dev-dependency.** `purrdf-shapes` gains `purrdf-conformance-kit` as a
  dev-dependency only. `layers.toml` exempts dev edges, and the kit is
  path-only and unpublished. All EARL code stays in `tests/`. Updated in the
  same commit:
  - the `AGENTS.md` crate-map row for the kit;
  - `AGENTS.md` §6 (lines 471–475: path-only dev-dependencies of published
    crates);
  - `scripts/release-crates.sh` lines 15–25. The kit is added to both the
    not-released list and the versionless dev-dependency explanation; that is
    a comment change only.
- **G16. Intermediate commits.** Commits after Task 1 and before Task 7 leave
  `w3c12_conformance` failing on the named remaining cases. That is visible in
  each commit message. The PR is squash-merged through `ghprsq`, so `main`
  never sees red. `docs/CONFORMANCE.md` states current counts only, with no
  process text.
- **G17. Stale 2017 README (U4, Patrick, 2026-10-17: "You can fix the README").**
  `vectors/shacl/README.md` says 6 `af/` entries and 126 total; the harness has
  9 and 129. Task 1 corrects that prose under Patrick's explicit authorization,
  which is a narrow exception to `CLAUDE.md`'s "never hand-edit `vectors/`" for
  this first-party README only. It then regenerates `vectors-shacl.sha256`
  with `check-corpus-frozen.py --update`. No test file changes, and R3 compares
  test files only.

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
| R1, R2 | 1 | `vendor-shacl12.py` `COMMIT` and `PROVENANCE.md` name `4cd469867a1cc6d2fff80e8a40723b26c695d67a`. `python3 scripts/check-corpus-frozen.py` passes for `vectors-shacl12` and `shapes-spec`. |
| R3, R4 | 1 | `validation.md` records `diff -rq --strip-trailing-cr <data-shapes@4cd46986>/data-shapes-test-suite/tests/{core,sparql} vectors/shacl/{core,sparql}` = 0 lines, plus `git log 08adb377..4cd46986 -- data-shapes-test-suite/tests` empty. The test files' hashes in `vectors-shacl.sha256` are unchanged. |
| R5, R6 | 2–7 | `cargo test -p purrdf-shapes --test w3c12_conformance` grading test: every approved entry passes unamended, except `SPEC_CONTRADICTED` entries that meet G1 (1)–(4). The six fix-only cases (four `rules-entailment-ruleset-*`, `deactivated-select-001`, `pre-binding-009`) pass. |
| R7, R9, R10 | 3, 4, 6, 7, 8 | Each such case has a `TN-triage.md` G1 record. `spec_contradicted_entries_fail_unamended` (count pinned; XPASS refused; an unknown id is refused). `xfail_ledger_is_empty` and `a_listed_entry_is_never_amended` pass. |
| R8 | 7 | Each of the six passes unamended or meets G1. `grep -c non-canonical-expected-decimal` over the harness, `conformance-matrix.py` and `check-doc-claims.py` = 0. |
| R11, R12 | 8 | Both harnesses build `Report` and call `outcome::to_earl` (`w3c12_earl_*`, `w3c2017_earl_*` tests). |
| R13 | 8 | Assertion count = K, the approved SHACL-root count pinned in Task 1 (1.2), and = 120 (2017). Every assertion has `earl:mode earl:automatic`. |
| R14 | 8 | Each entry's EARL outcome equals the unamended grade from the match arms; `SPEC_CONTRADICTED` entries are `failed`. |
| R15 | 8 | The claimed set is pinned (G5) and every approved entry's classes are within it. Each outcome is `passed` or `failed`. A kit unit test covers `Inapplicable`. |
| R16 | 8, 10 | `w3c12_earl_keys_equal_wg_page_keys`: the ported `testKey` and default-class rule give exact key-set equality with the SHACL-root approved entries. Page check in Task 10. |
| R17, R18 | 8 | Exact values asserted per file: `doap:name` = `"PurRDF " + CARGO_PKG_VERSION`; `doap:homepage` and `doap:repository` IRIs; `doap:description` contains the configured 40-hex commit and that file's full suite commit (`4cd46986…` for 1.2, `08adb3776709a014bc3062ede793c36275b22446` for 2017). Negative tests: a commit without a dir, a dir without a commit, and a non-40-hex commit are each refused. |
| R19 | 8 | Every `earl:assertedBy` is `<https://github.com/paudley>`, typed `earl:Assertor`, with `foaf:name "Patrick Audley"`. |
| R20 | 8 | In-memory double generation is byte-identical. `assert_golden!` covers `to_earl` on an `example.org` fixture. No date or time predicate appears. |
| R21 | 8 | 2017: `alltests.ttl`'s proposed subject is absent. 1.2: no `earl:test` for an entry whose `status` is proposed (enum-tested with a synthetic proposed case). |
| R22 | 9 (U5), external | `make w3c-earl` on a clean tree writes both files naming that HEAD; on a dirty tree it refuses. The tag build and re-check are the U1 external steps. |
| R23 | 11 | `contributions/data-shapes/purrdf-earl/drafts/w3c-pr.md` exists, naming both files, the suite commits and the regeneration command. Not opened. |
| R24–R26 | 12 | One full `make check` and `make conformance` run at the end, exit 0, recorded in `validation.md`. |
| R27–R29 | 2–7, 11 | 1.2 scoreboard and 2017 subset 120/120. For each `SPEC_CONTRADICTED` entry, a filed WG issue URL recorded in `validation.md` (G3), or the entry listed as open and the PR uses `Refs #570`. |
| R30, R31 | 8 | `earl_agrees_with_scoreboard` parses the printed EARL-scope line and the serialized EARL, and compares per entry and totals with the match-arm tally. Three EARL-only mutations (flip, remove, add) each make it fail. |
| R32, R33 (1.2) | 10 | Chromium on the WG `reports.html` at `4cd46986`, from a worktree under `worktrees/data-shapes/`, with `?reports=<generated file>`. Asserts: `#messages` empty; 0 page errors and 0 console errors; one PurRDF column; 0 "no data" cells across all page rows; no "Unknown tests" row; each Summary row's `passed / N` equals the matching printed `W3C12 class` line (G5). |
| R32, R33 (2017) | 10 | U2: `w3c_2017_earl_matches_wg_inventory` parses the EARL (a parse error fails). It asserts that the key set equals the 120 approved subjects of the frozen `vectors/shacl-reports/alltests.ttl`, each once with no others, and that passed/failed counts equal the printed `W3C 2017 subset` line. |
| R34, R35 | external | Tag, regenerate, re-check, open the W3C PR: the U1 external steps, listed open in the handoff. |
| R36 | 12 | Referenced in the PR body only. |
| R37 (U5) | 9 | `make -n release-tags VERSION=x` shows `w3c-earl` in the preflight before any `git tag`. `release-cargo.yaml` has the `make w3c-earl` step before the `SHA256SUMS` step. The three Python self-tests list seven assets, including both EARL files. `docs/RELEASE.md` names the step and the two assets. |

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

## Task structure (applies to every task, 1–11)

1. A focused check, recorded in `validation.md`.
2. An independent task review, `tasks/TN-review.md`, ending `VERDICT: PASS`.
3. A commit of the explicit files, with the named conventional message: no
   issue numbers, no attribution trailers, signed, hooks intact.
4. A push and an update on #570, once Patrick approves publication.
   Otherwise the work stays local and the handoff says so.

## Task 1: Re-vendor the SHACL 1.2 suite and update harness bookkeeping

Commit: `test(shapes): vendor the current W3C SHACL 1.2 test suite`.

- **Vendor:**
  - keep the `COMMIT` bump;
  - regenerate `vectors/shacl12`, `PROVENANCE.md` and `crates/shapes/spec`
    with the script;
  - run `check-corpus-frozen.py --update` for `vectors-shacl12` and
    `shapes-spec`.
- **Discovery (`shacl_corpora/shacl12.rs`):**
  - `Case12` gains `root`, `requires` and `status` (G5, G6), replacing
    `listed`;
  - update `W3C12_TOTAL_CASES` to 592 and the per-type pins;
  - update `W3C12_UNINCLUDED_MANIFESTS` (drop `xone-002` and `xone-003`),
    `W3C12_UNLISTED_ENTRIES`, and the `xone-003` delta;
  - check `INVENTORY`, `INFER_INVENTORY` and `SRL_TOTAL` entry by entry;
  - pin K, the approved SHACL-root count, and record it in this plan;
  - add the suite-commit constant and the `vocab::sht` class constants.
- **Documentation:** correct `vectors/shacl/README.md`'s counts (U4) and
  update `vectors-shacl.sha256`. Run the R3/R4 command into `validation.md`. Regenerate
  the `docs/CONFORMANCE.md` matrix block, and update `conformance-matrix.py`
  and `check-doc-claims.py` for the new counts (G9a).
- **Check:**
  - discovery and bookkeeping tests pass;
  - the grading test fails on exactly the 14 named cases and reports 6 decimal
    cases;
  - `w3c_conformance` stays 129/129;
  - `conformance-matrix.py --self-test` passes.

## Task 2: SHACL 1.2 pre-binding admission

Commit: `fix(shapes): admit SHACL 1.2 VALUES clauses that bind no pre-bound variable`.

- Under the dated 1.2 profile, refuse only a VALUES clause that mentions a
  pre-bound variable. The 2017 profile's total VALUES ban stays.
- **Check:**
  - `pre-binding-009` passes, and the 2017 `pre-binding` cases stay 14/14;
  - a neighbour pair under 1.2: VALUES mentioning `$this` is refused, VALUES
    without it is admitted;
  - under 2017, both are refused.

## Task 3: SHACL-SPARQL constraint cases

Commit: `fix(shapes): ` followed by the actual fixes.

- `deactivated-select-001` is fix-only: evaluate a node-expression
  `sh:deactivated` per the dated 1.2 text. Neighbour case: a non-boolean result
  signals the failure the text requires.
- `prefixes-003`, `sparql-failure-001`, `sparql-message-001`,
  `blankNodeArgument-001` and `singleInstanceOf-001` are triaged in
  `T3-triage.md` (written as WG findings), then fixed or classed under G1.
- **Check:** `deactivated-select-001` passes. Each of the other five passes
  unamended or meets G1 (1)–(4). The 2017 `sparql/` set stays all-pass.

## Task 4: Core `closed-001`

Commit: `fix(shapes): ` followed by the summary.

- **Check:** `core/property/closed-001` passes unamended or meets G1. The 2017
  `closed` cases still pass, and neighbouring closed or open cases are added if
  a refusal changes.

## Task 5: Inference rules `sh:entailment`

Commit: `feat(shapes): ` followed by the summary.

- Implement the entailment declarations the four cases require, per the dated
  1.2 Inference Rules text. There is no G1 path.
- **Check:** all four pass, and `rules_conformance` stays green. A neighbouring
  undefined regime signals the failure the specification requires.

## Task 6: SPARQL-RL stratification

Commit: `fix(shapes): ` followed by the summary.

- **Check:** `#test_13` and `#test_14` pass or meet G1. Positive stratification
  cases and neighbouring valid stratified rule sets are still accepted.

## Task 7: The six decimal-spelling cases

Commit: `fix(shapes): ` followed by the summary, or `test(shapes): grade node-expression decimals against the approved expectation`.

- Triage each case (one `shnex`, five `shnex-sparql`): do the dated SHACL 1.2
  Node Expressions and SPARQL 1.2 texts fix the computed literal's lexical
  form? Fix the engine where its form is wrong; otherwise apply G1.
- Remove the amended-grade path and its label, updating the consumers
  (G9a).
- **Check:**
  - the R8 grep shows 0 hits;
  - each case passes unamended or meets G1;
  - `non_canonical_expectations_are_really_non_canonical` is removed or
    rewritten so it no longer guards an amended grade.

## Task 8: A shared report structure, and EARL from both W3C harnesses

Commit: `feat(conformance-kit,shapes): emit W3C SHACL implementation reports as EARL`.

- **Kit (`outcome.rs`):**
  - add `Report`, `SubjectDescription` and `AssertorDescription` (G10, G11);
  - `to_earl` gains deterministic sorted output and `earl:info`;
  - add the EARL-scope line projection;
  - migrate `community-conformance` (EARL only) and `grading.rs`;
  - add the `vocab::earl` terms and the golden;
  - add a kit unit test that builds a `Report` over `example.org` IRIs in two
    styles, SPARQL-suite (`manifest#test`) and shexTest, and renders both
    projections. The decline of other-harness EARL rests on this test.
- **Harnesses, via `tests/support/w3c_earl.rs`:**
  - G12 mapping and the `SPEC_CONTRADICTED` table;
  - the G9a lines;
  - the `PURRDF_W3C_EARL_DIR`/`PURRDF_W3C_EARL_COMMIT` writer;
  - grading behind a `OnceLock`.
- **G15 updates:** dev-dependency, `AGENTS.md` and `release-crates.sh`
  comments. Update the scoreboard consumers (G9a).
- **Tests:** every R11–R21 and R30–R31 check in the contract.
- **Check:**
  - the focused tests for `purrdf-conformance-kit`, `purrdf-sparql-conformance`
    (community runner) and both harnesses;
  - `helper-census` (`w3c-vocab`, one-home);
  - `scripts/check-layers.py`;
  - `conformance-matrix.py --self-test`.

## Task 9: The regeneration command, as part of the normal release

Commit: `build: make w3c-earl regenerates the W3C SHACL implementation reports, and every release ships them`.

- Add the `make w3c-earl` target per G7, and document it with its two output
  files in `docs/CONFORMANCE.md`.
- U5 release integration:
  - the `release-tags` preflight runs `make w3c-earl` before any tag is
    created;
  - `release-cargo.yaml` runs `make w3c-earl W3C_EARL_DIR=target/dist` in
    the "Build C distribution" step, before the `SHA256SUMS` line (around
    line 168). The files then sit beside the other assets, and the commit is
    `git rev-parse HEAD` exactly as in G7. That equals `$GITHUB_SHA`, because
    `release-tags` creates lightweight tags;
  - both files are added to the attestation `subject-path` and the evidence
    artifact `path`;
  - the three asset-set sites from U5 are edited with no net Python growth;
  - `docs/RELEASE.md` gains the step.
- **Check:**
  - on a clean tree, `make w3c-earl` writes both files naming HEAD;
  - on a dirty tree it refuses, and the tree is unchanged afterwards;
  - `release-tags` contains the gate (a dry-run or `make -n release-tags` shows
    it);
  - the workflow YAML parses;
  - `python3 scripts/check-release-notes.py --self-test`,
    `python3 scripts/publish-github-release.py --self-test` and
    `python3 scripts/test_github_release.py` pass and show seven assets,
    including both EARL files;
  - `helper-census --non-rust-ratchet` passes.

## Task 10: Reports-page and inventory verification

Commit: `test(shapes): check the 2017 EARL against the WG inventory`.

- **2017:** vendor `alltests.ttl` as a new frozen root, `vectors/shacl-reports/`.
  It is a byte-exact copy from the W3C checkout
  (`git show 4cd46986:data-shapes-test-suite/reports/alltests.ttl`), with a
  README giving provenance and licence (following `vectors/shacl`'s precedent),
  a `roots.toml` entry, a hash manifest and a `LICENSING.md` row. Then add
  `w3c_2017_earl_matches_wg_inventory`.
- **1.2:** create a W3C workspace worktree of `data-shapes` at `4cd46986` (base
  passed explicitly), serve it, and run the Chromium checks from the R32/R33
  (1.2) row against the Task 9 output. The browser harness stays in the W3C
  workspace; evidence goes to `validation.md`.
- **Check:** both the 2017 and 1.2 checks pass.

## Task 11: WG issues and the upstream submission draft (W3C workspace; approval-gated)

No PurRDF commit.

- For each `SPEC_CONTRADICTED` entry, draft the WG issue as a discussion
  question in `contributions/data-shapes/purrdf-earl/drafts/` and present it
  to Patrick. File it only after his approval, and record the URL in
  `validation.md`.
- Draft the W3C PR description (`drafts/w3c-pr.md`, R23). It names both files,
  the suite commits, the regeneration command, and the question about
  regenerating the static 2017 table.
- **Check:** the drafts exist, and filed URLs are recorded or the entries are
  listed as open.

## Task 12: PR creation, publication and handoff

1. Apply the approved #570 text alignment (see "Issue text alignment").
2. Run the artifact inventory, and read each verdict.
3. Run the full `make check` and `make conformance` once, recording them in
   `validation.md`.
4. Check source cleanliness separately from `.stage`.
5. Get a substantive completeness review of the implementation against this
   plan.
6. Re-check `origin/gh-pages` for suite commits newer than `4cd46986`, and
   report any.
7. Open the PR, which needs Patrick's push approval: titled
   `feat(shapes): W3C SHACL implementation reports against the current suites`.
   The body includes:
   - `Closes #570` or `Refs #570`, per G3;
   - the changes;
   - the actual validation;
   - the `earl.nt` byte change;
   - the open external steps (tag, regenerate, re-check, W3C PR);
   - the residual risks.
8. Publish the plan to the PR.
9. Post the "least confident" note to the issue and the PR.
10. Hand off: `$STAGE/handoff.md` lists every R-id as MET or OPEN with its
    evidence path, plus the PR, the branch, the checks run, the unrun Stage 2/3
    gates and the open external steps.

## Issue text alignment (for Patrick's approval with this plan)

Replace #570's step 4 and its last acceptance criterion with:

> 4. Provide everything the W3C submission needs on PurRDF's side:
>    `make w3c-earl` regenerates both reports from any clean build, naming
>    its version and commit; both are verified in the WG's actual 1.2 reports
>    page and against the 2017 `alltests.ttl` inventory; and the upstream PR
>    description is drafted. Tagging a release and opening the W3C PR are
>    separate, later steps. Every release from v4.0 on generates both reports
>    in its preflight, and publishes them as verified GitHub Release assets.
>
> - [ ] `make w3c-earl` produces both reports from a clean build, they pass
>       the WG page and inventory checks, and the upstream PR description is
>       drafted. (The release tag and W3C PR follow separately.)

Add to step 2: "An approved test that appears to contradict the specification
surfaces a question for WG discussion: it is reported failed, visibly, until
the WG resolves it."

## Review dispositions

### Revision 1 → 2

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

### Revision 2 → 3

- **Completeness:**
  - N1 (task order): Task 9 is now `make w3c-earl`, run before the Task 10
    checks.
  - N2 (structure): the task-structure section covers every task, and Task 12
    lists the publication and handoff contents.
  - N3: G3.
  - N4: G11 and the R30/R31 row.
  - N5: G8 and the R17/R18 row.
  - Advisories: the console and page-error observable; K pinned; W3C PR draft
    (Task 11); `gh-pages` re-check (Task 12); `alltests` destination
    (Task 10); unknown-id refusal (G1).
- **Constraints:**
  - N1: Task 10 adds a frozen root.
  - P1: U3, with the issue text in "Issue text alignment".
  - A1: G15.
  - N2: G5 (`vocab::sht`).
  - N3: G7 (Makefile conventions).
  - N4: G13 (temp paths).
  - A7': G1 (4).
  - N5: G17 (Patrick authorized the fix: U4).
- **Design:**
  - B4: G9a.
  - B5: Task 10.
  - Advisories: `Report` in `outcome.rs` with claimed classes left to the
    harness (G11); community migration EARL-only; shared `w3c_earl.rs`;
    `status` as an enum with all page rows counted; `earl:info` = rationale
    text; lane-env; `OnceLock`.
- **Enhancement:**
  - Blocker: G9a.
  - Folder-default class rule: G5.
  - `alltests.ttl`: Task 10.
  - Item 9: G10 (two terms added, two declined with reasons).
  - Item 4: the kit-level `Report` test is part of the R11/R12 tests.
- **Adversary:**
  - F2: G1 (4) plus G3.
  - F7: G8 and the R17/R18 row.
  - N1: U1 plus "Issue text alignment" plus U5.
  - N2: G11.
  - N3: G5.
  - Advisory: Task 10 (2017 check in the repository; `alltests.ttl` as a
    frozen root).

### Final review (`reviews/plan-review.md`) → revision 3 final

- **BLOCKING-1:**
  - (a) file names in G13;
  - (b) the three asset-set sites in U5 and Task 9;
  - (c) the workflow placement, `target/dist` and attestation;
  - (d) the R37 row and the issue text.
- **A1–A10:** all applied, covering the class-set union, the kit test, the
  class lines, the matrix column, the binding exceptions, the G1 (4)
  rejection, `lane-env`, stale text, `doap:bug-database` and `handoff.md`.
- **Attestation:** the recommended default is adopted; Patrick may override.

---

# Addendum A: Stage 3 gap analysis, all findings fixed in #570

Source: `reviews/gap-analysis-stage3.md` (G-01 to G-16) and
`reviews/completion-audit-stage3.md`.

## Patrick's decisions (2026-10-18, verbatim)

- **U7 (G-01).** "Fix in #570": add dated-law selection to the CLI, Python,
  WASM and C hosts in this PR.
- **U8 (G-02).** "Add the ED law in #570": introduce a separately named, dated
  editor's-draft law, grade the 1.2 suite under it, and keep
  `shacl-wd-2026-09-18` faithful to the WD text.
- **U9.** "Fix all in #570" and "Fix all now": G-05 and G-11 are fixed here
  too, not filed as issues.

## Governing decisions for the addendum

- **A-G1. The ED law.** Add a dated editor's-draft law with id
  `shacl-ed-2026-10-16`. Its texts are the data-shapes editor's drafts at
  `4cd469867a1cc6d2fff80e8a40723b26c695d67a`, the suite commit. It follows the
  existing `ShaclProfile` design (an exact edition, no nearby-edition mapping),
  and every specification URL it names resolves to that commit.
  - The 1.2 harness grades `sht:Validate` and rule cases under it.
  - Every post-WD semantic added on this branch moves under the ED law, and
    `WD_20260918` refuses or ignores it as the WD text says. Each case gets a
    neighbour test. The semantics are:
    - node-expression `sh:deactivated` on SPARQL and expression constraints;
    - reifier-expression `sh:deactivated`, if post-WD;
    - the version-IRI prefix path;
    - the blank-node argument rule;
    - rule sets as `sh:entailment` values;
    - any other Task 3–6 semantic the T13 audit finds post-WD.
  - The audit compares every Task 2–7 behaviour against the WD text
    (`git show 32318223:…`) and the ED text at `4cd46986`, and records which
    law each belongs to in `tasks/T13-triage.md`.
  - The REC 2017 law keeps the 2017 prefix path `sh:prefixes/owl:imports*/sh:declare`.
- **A-G1a. ED inherits WD.** The ED law is the WD law plus the post-WD semantics: every site that today selects `WD_20260918` selects the ED too (one predicate, `ShaclProfile::is_shacl12()` or equivalent, replaces the five `== WD_20260918` tests), and only the post-WD semantics test for the ED alone. Task 13 re-points to the ED: `shacl12.rs:530` (`tc.profile`), `w3c12_conformance.rs:353` (rule cases) and `:591` (`Suite.law`), and every existing test that asserts a post-WD behaviour under `WD_20260918` (`shape_deactivation_expression.rs:425`; the audit lists the rest), each gaining a WD refusal neighbour. The `DATED` array and the `complete_reports.rs` law loops include the ED. For G-13, `w3c_conformance.rs:112-115` sets the 120 W3C entries' `profile` to `REC_20170720` and `Suite.law` to `shacl-rec-2017-07-20`, so the 2017 `doap:description` names the Recommendation law. `core_specification`/`sparql_specification` for the ED return the commit-pinned document URLs (`https://github.com/w3c/data-shapes/blob/4cd469867a1cc6d2fff80e8a40723b26c695d67a/shacl12-core/index.html` and `.../shacl12-sparql/index.html`); `xpath_specification` stays XPath 3.1. The id's date is the commit date of `4cd46986` on `gh-pages`, recorded with its `git show -s --format=%cs` output in `validation.md`; if it is not 2026-10-16, the id follows the commit, not the plan.
- **A-G2. Hosts.** Every host can select the law: `purrdf validate
  --shacl-profile <id>`, and the matching Python, WASM and C parameters. Each
  is decoded once through `purrdf_validate::complete_profile`, the one home for
  that decoding.
  - The default stays the compatibility law (behaviour unchanged).
  - A host-level lane runs the 1.2 `sht:Validate` entries through the CLI
    under the ED law and requires every approved entry to pass, which proves
    the 382/382 claim on a shipped surface.
  - The EARL subject's `doap:description` names the law and states that it is
    the same law the hosts expose, so a WG reader can reproduce the result.
  - A dated law requires its XPath bundle. `--shacl-profile shacl-ed-2026-10-16 --xpath-regex xpath20` must surface the existing `XPathProfileConflict` as a usage error, with a test.
  - The C API keeps its ABI by adding a twin (e.g. `purrdf_shacl_validate_to_sarif_law(..., regex_profile, shacl_profile, out)`) with a header entry and the C smoke test; no existing signature changes.
  - Entries covered: `purrdf validate` whole-graph and `--changes`; Python `validate`; wasm `validateToSarif`; the C validate twin. A prepared product stores no SHACL law: a `--shapes-product` restore takes the law from the request via `restore_shapes_product_with_options`, tested with one product under `legacy` and under the ED on `pre-binding-009`; XPath compatibility stays with the existing `resolve_xpath` call.
  - Every host's unsupported-id refusal is `complete_profile`'s `unsupported-profile` status, tested once per host.

- **A-G3. Deactivation evaluation order (G-04).** The text's SHOULD-fail is kept, and the set of `sh:deactivated` node expressions a validation evaluates is made independent of list order and of short-circuiting. Rule: at lowering, each shape plan records whether its reachable closure (through `sh:or`, `sh:and`, `sh:xone`, `sh:not`, `sh:node`, `sh:property`, `sh:qualifiedValueShape`, `sh:filterShape`, `shnex:conformsToShape`, cycle-safe) contains any node-expression `sh:deactivated` on a shape, a constraint reifier or a `sh:property` reifier. Within such a closure, `sh:or` and `sh:and` evaluate every member before combining, and the conformance probe (`AnyViolation`) records the first disallowed violation but returns `Flow::Continue`, so every constraint, property shape and nested shape the data reaches is walked exactly as a full validation walks it. Outside such a closure, evaluation is unchanged (no cost for graphs without deactivation expressions). The memo (`conforms_memoized`) keeps repeated probes cheap. The result: a failure is raised iff some reachable expression is ill-formed at a focus node, whatever the member order.
  The order-independent full-closure design is chosen over the gap analysis's option (a), because it keeps the SHOULD-fail behaviour already chosen and shipped on this branch (T3). Patrick may override.

## Tasks

The task structure is the same as for Tasks 1–12: focused check, independent
review, signed commit, push, and an update on #570. Task 13 precedes 14 and 15
(both consume the ED id and `tc.profile`). Tasks 16–21 depend only on the base.
Task 22 is last. Commit subjects:
- 13: `feat(shapes)!: add the dated SHACL editor's-draft law and restore WD fidelity`
- 14: `feat(cli,python,wasm,capi): select the SHACL law on every host`
- 15: `test(shapes): grade the product and import lanes under the case's law`
- 16: `fix(shapes): make deactivation evaluation order-independent and keep closed paths`
- 17: `fix(shapes): type node-expression evaluation failures apart from processor failures`
- 18: `fix(xsd,sparql-eval): spell SECONDS and computed decimals as documented`
- 19: `fix(shapes): resolve rule-set includes at load and bound entailment changes`
- 20: `feat(conformance-kit,sparql-conformance): one EARL assertion per test and subject`
- 21: `test: pin footprint, closed-path and 2017 refusal positions; cache keys; w3c-earl output check`

- **Task 13 (G-02, A-G1): the ED law.**
  - Add `ShaclProfile::ED_20261016` (id `shacl-ed-2026-10-16`).
  - Do the post-WD audit, then move the semantics. Restore WD fidelity, gate
    the prefix path under REC, and grade the 1.2 harness under the ED law.
  - Correct the docs (`docs/CONFORMANCE.md:690`, the book, code comments), the
    T3/T5 triage attributions and the WG findings draft.
  - Check: 591/591 under ED; WD refusals with their neighbours; REC prefix
    path neighbour; 2017 suite 120/120 under `REC_20170720` (G-13).
- **Task 14 (G-01, A-G2): host law selection.** CLI, Python, WASM and C parameters, documented in the book and the host READMEs, plus the CLI 1.2 lane and the EARL description update. Check: the CLI lane is 213/213 Validate under ED. Each of Python, WASM and C has a neighbour pair on the shipped validate entry: (1) `legacy` versus `shacl-ed-2026-10-16` on `pre-binding-009` (refused at load versus conforms, the two answers G-01 observed); (2) `shacl-wd-2026-09-18` versus `shacl-ed-2026-10-16` on `deactivated-select-001` (the WD refuses a node-expression `sh:deactivated` on a SPARQL constraint; the ED validates it). If the T13 audit moves the VALUES rule itself to the ED, swap (1)'s WD/ED pair in and record why in `tasks/T13-triage.md`. An unknown id (`shacl-ed-2026-10-17`) is refused with `unsupported-profile` on every host.
- **Task 15 (G-03): lanes under the graded law.** The prepared-product and
  vocabulary-import lanes run each case under `tc.profile`. Delete the false
  `REFUSED_AT_LOAD` entries, correct the `docs/CONFORMANCE.md:129` text, and
  reword `prebinding.rs:299`.
- **Task 16 (G-04, G-06, G-08): deactivation and closed.**
  - A-G3 order independence.
  - A reifier-deactivated `sh:property` edge keeps its path in `sh:closed`'s
    permitted set.
  - At most one reifier `sh:deactivated` per statement, for expressions too.
  - Neighbours for each.
- **Task 17 (G-05): evaluation failures are typed.** Split evaluation failures
  (arity, cardinality, type) from processor failures (governor, recursion,
  resource). A custom-function body's evaluation failure becomes that call's
  SPARQL error; a processor failure still fails validation. Neighbour pair: a
  two-node argument leaves the variable unbound, while a fuel trip still fails.
- **Task 18 (G-07, G-14): numeric spelling.**
  - `SECONDS` takes the seconds field from the argument's lexical form
    (`05.500`).
  - Unary plus keeps its operand's spelling, or the stated rule and the test
    comment are corrected; T18 decides on evidence and records it.
  - `numeric_render_cost` prices the emitted form.
  - The docs no longer overclaim (text and retrieval scores).
  - The `seconds-01` vs `seconds-example` conflict is added to
    `drafts/wg-findings.md`.
  - Governor profile and goldens follow the documented procedure if costs
    move.
- **Task 19 (G-09, G-10): rule sets.**
  - The include closure of every declared rule set is resolved at load, and an
    undeclared include is refused whatever regimes are named.
  - One definition of "rule set" (typed) serves both readers.
  - `validate_change` with any declared entailment answers
    `FocusExpansion::Everything` ("rules entailment").
  - Neighbours for each.
- **Task 20 (G-11): one assertion per test and subject.**
  - `Report` refuses duplicate case IRIs per subject.
  - `to_earl` supports several subjects in one graph.
  - The community runner emits one subject per (profile, surface), each with
    its own description.
  - Kit tests: a duplicate is refused; two subjects are rendered.
- **Task 21 (G-12, G-13, G-15, G-16): pins, tests, tooling.**
  - The uv cache keys add `Cargo.toml`, `rust-toolchain.toml`,
    `crates/**/Cargo.toml` and `crates/*/spec/**`.
  - Footprint cases.
  - Complex-path closed cases.
  - The three missing 2017-refusal positions.
  - The `GOLDEN_LEN` history.
  - The `shacl-shacl` rationale.
  - The JSON Schema loss subject.
  - The `sparql-syntax-error-001` comparison with offsets normalized instead of
    the exemption.
  - `make w3c-earl` removes stale outputs and checks `test -s`.
- **Task 22 (Stage 3 resume).** The held items:
  - the SIMD-assembly table (regenerated by `check-simd-asm.py --write-doc`);
  - F6, the publisher wording;
  - A1, now superseded by A-G2's description.

  Also: completion-audit F3–F5, F7 and F8; update the PR body and the published
  plan comment with Addendum A (ED law and id, host selection, the second
  `earl.nt` byte change, the 2017 report naming the Recommendation law); refresh
  `handoff.md` with A-R1..A-R16 as MET/OPEN; doc sites
  `shacl_corpora/mod.rs:243-249` and `w3c12_conformance.rs:276`.

  Then refresh the docs matrix with `--write-doc`, push, and require hosted CI
  fully green on the exact head SHA (clearing completion-audit F1). Re-run the
  Stage 3 binding gates on the new head.

## Completeness additions

| ID | Task | Executable acceptance check |
|---|---|---|
| A-R1 (G-01) | 14 | The CLI lane passes 213/213 under `--shacl-profile shacl-ed-2026-10-16`; the per-host neighbour pairs (legacy vs ED on `pre-binding-009`; WD vs ED on `deactivated-select-001`; unknown id refused) pass on Python, WASM and C; the EARL description test asserts the law sentence. |
| A-R2 (G-02) | 13 | Grep shows no WD citation for post-WD text. WD refusal tests with their neighbours. REC prefix-path neighbour. 1.2 harness under ED: 591/591. |
| A-R3 (G-03) | 15 | Both lanes run under `tc.profile`, and the `REFUSED_AT_LOAD` entries are gone with their tests green. |
| A-R4 (G-04) | 16 | `sh:or`, `sh:and`, `sh:xone` and `sh:not` give identical verdicts in both member orders when the expression sits (i) on the member shape, (ii) on a shape reached through the member's `sh:node`, (iii) on a property shape of the member, and (iv) on a constraint reifier of the member listed after a violating constraint. For each, the probe verdict equals the verdict of validating the member shape directly with `sh:targetNode`. A neighbour graph with the same structure and `sh:deactivated true`/`false` constants keeps its short-circuit (asserted by a count of member walks or a timing-free fuel observation). |
| A-R5 (G-05) | 17 | The two-node-argument probe yields `?value "false"`; the fuel trip still fails. |
| A-R6 (G-06) | 16 | The constant and expression reifier spellings give equal closed results. |
| A-R7 (G-07) | 18 | `SECONDS("…05.500")` is `"05.500"`, or the rule is restated exactly, as T18 decides; the WG draft names `seconds-01`. |
| A-R8 (G-08) | 16 | A second expression reifier is refused; the single-reifier neighbour is accepted. |
| A-R9 (G-09) | 19 | An undeclared include is refused at load, with or without `sh:RulesEntailment`. Untyped sets are read consistently. |
| A-R10 (G-10) | 19 | `validate_change` with entailment gives the bounded/Everything answer and no false diagnosis. |
| A-R11 (G-11) | 20 | A duplicate case is refused per subject. The community EARL has distinct subjects, and no subject has two assertions for one test. |
| A-R12 (G-12) | 21 | The cache keys cover the spec files and manifests, and an edit to a spec file triggers a rebuild. |
| A-R13 (G-13) | 13, 21 | The footprint, complex-path and 2017-position tests exist and pass. The 2017 harness runs under `REC_20170720`. |
| A-R14 (G-14) | 18 | The unary-plus behaviour matches its stated rule and its test. The render-cost test prices the emitted form. |
| A-R15 (G-15) | 21 | Each of the four texts and pins is corrected, and the offsets-normalized comparison replaces the exemption. |
| A-R16 (G-16) | 21 | `make w3c-earl` fails when an output is missing or empty. |

- **U10 (Patrick, 2026-10-18).** On `node-expr/shnex-sparql/hasLang-example` calling
  `sparql:hasLang ( "hello"@en "en" )` with two arguments when the SPARQL 1.2
  grammar's `hasLANG` takes one, Patrick chose verbatim "Spec-contradicted + WG item
  (Recommended)".
  - Refuse the two-argument call like every other wrong-arity call, so no
    argument is silently dropped.
  - Add the case to `SPEC_CONTRADICTED` with the grammar clause as its
    rationale. It is reported `earl:failed`, so the 1.2 EARL shows 381/382.
  - Draft a WG discussion item for Patrick's approval.
  - G3 applies: the PR says `Closes #570` only once that WG issue is filed.
    Otherwise it says `Refs #570`.

- **U11 (Patrick, 2026-10-18), overriding G3.** Patrick's words, verbatim: "No... the issue closes.  Sorry. don't care what says not."
  - The PurRDF PR says `Closes #570` whether or not the hasLang WG issue has been filed.
  - The WG issue stays a draft awaiting Patrick's approval, and is tracked in the W3C workspace rather than as an open item on #570.
