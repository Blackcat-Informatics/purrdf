# Issue #418: Vendor and run the W3C SPARQL 1.0 data-r2 conformance suites

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github
Labels: enhancement, conformance

## Body

## Problem
The vendored W3C SPARQL conformance corpus (`crates/sparql-conformance/suite/w3c-sparql11/`) contains only the SPARQL 1.1 test directories (aggregates, bind, property-path, update, and so on). None of the SPARQL 1.0 `data-r2` suites are vendored or run: `expr-builtin`, `expr-equals`, `expr-ops`, `optional`, `optional-filter`, `algebra`, `basic`, `triple-match`, `open-world`, `regex`, `sort`, `distinct`, `solution-seq`, `dataset`, `graph`, `i18n`, `type-promotion`, `cast`, `boolean-effective-value`, `bound`, `reduced`, `construct`, `ask`, `syntax-sparql1..5`. The official SPARQL 1.1 test suite manifest still includes these, and SPARQL 1.2 keeps their semantics.

So "SPARQL conformance green" never runs them. One example of what this hides: `LANGMATCHES("", "*")` returned true, against SPARQL 1.1/1.2 §17.4.3.11, and the official tests `expr-builtin/q-langMatches-3` and `q-langMatches-4` fail. Nothing in the corpus caught it.

## Proposed solution
- Vendor the W3C `rdf-tests/sparql/sparql10` data-r2 suites verbatim, with the same provenance, licensing and REUSE treatment as the existing `w3c-sparql11` corpus (PROVENANCE.md with the pinned upstream commit; frozen vendor files are never hand-edited).
- Wire them into the conformance runner and `scripts/conformance-matrix.py` so they run in the sparql shard and in CI.
- Triage every failure: fix product bugs, and handle anything that conflicts with RDF 1.2 / SPARQL 1.2 semantics through the existing ledger with a cited normative reason. Never change a vendored expectation.

## Acceptance
- The data-r2 suites run in `make conformance` and CI, reporting pass/fail/ledgered tallies.
- No unledgered failures, and every ledger entry cites the normative text that justifies it.
- Every product fix comes with its own failing-first test.


## Comments (7)

### paudley — 2026-10-05T01:37:32Z

Issue summary

Import the W3C SPARQL 1.0 data-r2 corpus and execute it through the existing native SPARQL conformance runner, so green conformance includes the older semantics retained by SPARQL 1.2. The live issue is open, has no comments and has no competing PR or worktree owner.

Baseline defaults

Apply the populated `.baseline`: adversarial source review, complete delivery, ordinary hook-verified signed commits and exact-source evidence. `.deficiencies` must remain notice-only. CONSTITUTION.md is absent. Preserve unrelated work and the explicitly protected in-flight parser, SPARQL and serializer branches.

Standing constraints

Apply `.goals`: Rust owns behavior and tests; one home per job, deterministic RDF 1.2 carrier semantics, typed hard failures, portability and no semantic Cargo features. Vendored W3C bytes are immutable. No general WASM test expansion, host inspection or cache cleanup. Final merge uses ghprsq under the root merge lock.

Completeness contract

| Requirement | Task |
| --- | --- |
| Verbatim W3C data-r2 files, pinned upstream commit, provenance and per-file REUSE sidecars | 1 |
| All 29 official groups, including bnode-coreference and syntax-sparql1..5, plus the separately supplied extended sorting case | 1, 2 |
| Native harness discovery without double counting root includes and their children | 2 |
| Corpus freeze hashes, exact group/case inventory and duplicate-IRI tripwires | 1, 2 |
| Run in make conformance, the sparql shard and existing CI; honest pass/fail/ledger tallies | 2, 4 |
| Triage every failure; product fixes each have a native failing-first regression | 3 |
| Only cited RDF/SPARQL normative divergences enter the existing ledger; XPASS/stale/ambiguous entries fail | 3, 4 |
| Full native qualification, ordinary task commits/pushes, issue receipts, PR, CodeRabbit disposition and locked merge | 0–5 |

Enhancement audit and design

The useful structure is an immutable upstream manifest graph with exact native coverage, not another conformance engine. Reuse the production loader, evaluator, comparator, ledger and scoreboard. Pin the same W3C rdf-tests commit as the existing SPARQL 1.1 import, `426c7df4b5d5d292e3ba09dc22e622ea301f230a`; its `sparql/sparql10/` tree contains the complete data-r2 corpus.

Preserve every imported payload byte. Map the upstream root `manifest.ttl` to `manifest-all.ttl`, documenting that filename mapping in provenance and its license sidecar: the loader already requires aggregators to use a different name so discovering children cannot execute every case twice. Run the 29 standard group manifests separately. Extend the existing Rust discovery's exact leaf filename rule to include `extended-manifest.ttl`, thereby executing the upstream sort extension without copying its case into a new selector or runner. Native inventory checks compare the complete official aggregator closure with the discovered standard cases, pin actual counts and require the extended case exactly once.

Adopt byte-fidelity verification against the pinned archive, complete case inventory, corpus-freeze refusal and existing XPASS discipline. A new downloader, parser, fixture format or parallel oracle is declined because the existing homes already implement each job. Renaming/re-authoring test contents and weakening expected answers are rejected; only the documented root filename mapping is necessary to reconcile upstream organization with the existing discovery contract. Product bugs owned by PR428/419/410 are not edited or reviewed here; only their already merged main results may be integrated.

Task 0 — isolated setup

After independent plan/compliance review, fetch merged origin/main and create `.worktrees/418-sparql10-data-r2` on `paudley/418-sparql10-data-r2`. Verify branch, root instructions, clean state and emergency ledger. Reuse the admitted exact db8f076d2 compiler environment through the Stage Cargo wrapper without modifying global toolchain configuration.

Task 1 — frozen corpus import

Fetch the pinned W3C archive into task-owned ignored target storage. Copy the full sparql10 payload tree with the documented root manifest filename mapping, add a PROVENANCE.md and the existing LicenseRef-W3C-Test-Suite sidecar treatment. Register its root in the existing corpus-freeze declaration and generate its sorted SHA-256 receipt. Verify payload bytes independently against the archive and run REUSE/freeze checks. Commit with normal hooks, push and post the committed inventory/fidelity receipt to the issue.

Task 2 — native discovery and coverage

Extend the one Rust discovery filename rule for the upstream extended leaf, with exact-name positive/near-neighbour native regressions. Add native tests for all 29 groups, actual manifest entry counts and root-closure/discovered-case equivalence without duplicates. Run the corpus first and record every failure unchanged. Update the matrix source description and native conformance documentation without adding another suite runner. Commit/push the complete wiring and post the exact inventory plus observed baseline failures.

Task 3 — complete failure triage

For every unledgered failure, distinguish a product bug, harness interpretation bug or RDF 1.2/SPARQL 1.2 normative divergence. Add an independent native regression that fails before each product repair and passes afterwards; repair the single production home. Keep all vendored expectations unchanged. A legitimate divergence receives a narrowly qualified existing-ledger entry with primary normative citation and an explicit reason; qualify existing tails by suite if the new corpus exposes ambiguity. Preserve hard XPASS and stale-entry rejection. Integrate fixes already merged by protected owners if they resolve overlapping cases, without modifying their work. Commit/push each coherent repair and post its failing-first evidence.

Task 4 — final qualification and scoreboard

Require zero unledgered failures across the complete new corpus, then run the complete native conformance package, affected product tests, warning-denied Clippy/rustdoc, inventory/ledger/freeze/REUSE checks and the real make conformance sparql shard. Refresh measured documentation/baseline using the existing generator only after successful measured tallies; never raise a budget for a product gap. Run required local gates and verify release portability through the existing build lane. Reconcile generated projections, clear deferral scans and verify a clean signed/pushed head. Post exact-source receipts.

Task 5 — PR, feedback and integration

Open a regular issue-linked PR after all implementation/qualification tasks pass and post the plan. Read actual CodeRabbit feedback, repair actionable findings with normal verified commits/pushes and rerun affected gates; require final-head disposition and successful hosted checks. Sync only merged origin/main, revalidate integration, write/post squash notes beside the worktree and merge exclusively with `/home/paudley/stage/root/bin/ghprsq` after acquiring the root merge lock. Verify signed result/audit refs, issue closure and remote branch deletion, and remove only the owned issue worktree/branch after preserving its evidence.


### paudley — 2026-10-05T01:46:44Z

Task 1 is committed with normal verification hooks, signed, verified and pushed as `5cbf8c64de918350de3e0efa02947ec4cc7ac9fd` on `paudley/418-sparql10-data-r2`.

All 875 payload files (709,822 bytes) from W3C rdf-tests commit `426c7df4b5d5d292e3ba09dc22e622ea301f230a`, `sparql/sparql10/`, are imported with per-file W3C sidecars, license text and provenance. Every payload byte was checked against the pinned upstream archive (SHA-256 `47c66c938621cfc96c0939256967100c98a28faf2e320be94347b84082e28329`). Only the root filename is mapped from `manifest.ttl` to `manifest-all.ttl`, preserving its bytes and relative includes to avoid duplicate execution under the existing harness contract.

The complete existing freeze registry was extracted coherently to TOML data to allow a new declaration without growing legacy Python. Before adding SPARQL 1.0, all 26 existing root/receipt values and their order compared exactly equal; afterwards those same entries remain unchanged and the new root is 27th. All nine production guard functions are AST-identical; only registry loading changes. Native Rust owns the expectations for exact registration, all 875 frozen paths, missing/malformed-registry refusal and its valid neighbour. No Python tests or second freeze implementation were added.

Validation: two native tests passed, focused warning-denied Clippy passed, all 27 corpus freezes passed, all 12 vendored license roots passed, workspace formatting/diff checks and normal commit hooks passed. The existing data-r2 root reaches all 29 standard groups; native discovery/count wiring and complete execution/triage are Task 2/3. This receipt does not claim those cases have passed.


### paudley — 2026-10-05T02:02:25Z

Task 2 is implemented and pushed: complete native discovery of all 29 data-r2 groups and the separately declared extended sort leaf, with exact spelling and the same loader refusal for either auto-discovered aggregator name. The original root filename maps to `manifest-all.ttl`; native inventory proves its 482-case closure equals the standard leaves exactly, and the extended leaf contributes one unique case (483 total). The existing matrix runner automatically executes all 30 manifests and identifies sparql10 in its source description.

Failing-first native regressions caught both omitted extended-leaf discovery and duplicate execution through an extended-name aggregator. The repaired source passes all 23 focused discovery/include/inventory tests, warning-denied package Clippy, formatting, diff checks, and normal signed commit hooks.

The first complete unmodified corpus execution is deliberately recorded as a failing baseline: **395 passed, 88 failed, zero xfail/XPASS/unmodeled**, across all 30 manifests. This is not conformance completion. Task 3 is now repairing concrete RDF result-reader, indexed-order and lax-cardinality harness gaps, then triaging every remaining engine failure. Vendored expectations and the product-gap budget are unchanged.


### paudley — 2026-10-05T02:24:39Z

Native harness remediation is signed and pushed at 0baeba2dc. The existing reader now models DAWG RDF ASK booleans and Turtle/RDF/XML SELECT rows, preserves wholly unbound rows, follows explicit `rs:index` ordering and refuses malformed shapes. The manifest's `mf:LaxCardinality` rule is enforced with bounded expected multiplicities and one global blank-node bijection. FROM/FROM NAMED fixture documents are supplied under their parsed IRIs to the unchanged evaluator. The global result encoding now preserves original blank scopes and unbound-solution multiplicity.

Failing-first tests demonstrated the old carrier, source-loading and comparison defects. The repaired source passes **55 focused native Rust tests**, warning-denied all-target Clippy, format/diff/frozen-payload checks and ordinary signed commit hooks. All **44 legacy manifests** remain qualified at **862 passed, 5 existing ledgered fixture errata, zero failure/XPASS/unmodeled**.

Complete data-r2 execution is now **456 passed / 27 failed**, zero new xfail/XPASS/unmodeled. This checkpoint fixes 61 of the original 88 failures without changing any vendored expectation, product code or product-gap budget. The remaining exact cases are still active work: native semantic-fork proof, arithmetic lexical evidence and independent product-gap triage. Issue completion and PR creation remain contingent on complete qualification.


### paudley — 2026-10-05T03:40:29Z

# Native OPTIONAL and equality repairs

Signed, hook-verified checkpoint: `302abc9e9c543e4ca92a2f2b9f279497c6416be5`
on `paudley/418-sparql10-data-r2`.

The two OPTIONAL defects are repaired in the parser's existing translation home:
it retains the number of filters owned by the group and lifts exactly those into
one LeftJoin conjunction in written order under
[SPARQL 1.2 §18.3.2.7–9](https://www.w3.org/TR/sparql12-query/#sparqlTranslateGraphPatterns).
Nested filters remain inside the
independent right operand. Non-OPTIONAL groups retain their existing filter chains;
the remote forwarding invariants and the 20,000-fuel nested-EXISTS bound remain
unchanged and pass.

The borrowed and owned equality homes now distinguish known language-value
inequality from unknown or ill-typed literal errors under
[sameValue §17.4.2.2](https://www.w3.org/TR/sparql12-query/#func-sameValue).
A known dateTime/date pair
is unequal independently of ordering. Numeric promotion, cross-type NaN,
same-term equality, timezone uncertainty and unknown/ill-typed errors remain
covered by native tests. The new path regressions cover dataset, computed and
mixed operands, both directions, IN and triple components.

The native regressions failed before their repairs. Final focused qualification:

- 451 algebra unit tests and 1,370 evaluator unit tests pass, including all three
  parser ownership regressions and the existing forwarding/fuel neighbors.
- All 11 independent data-r2 result/value/type/source oracles pass. The two 8×8
  equality tables pin complete subject-pair sets; arithmetic pins 64 binary
  pairs and eight unary rows; date inequality pins all three exact source values.
- All three exact/stale/ambiguous ledger integrity tests pass.
- Warning-denied all-target Clippy and Rustdoc for algebra/eval/conformance pass.
- Formatting, helper hygiene, diff checks and all 27 frozen corpus receipts pass.
- The normal commit hook passes; the checkpoint has a good Git signature.

The complete 74-manifest run attempts all 1,350 cases:

| Corpus | Pass | Exact ledger | Fail | XPASS | Unmodeled |
| --- | ---: | ---: | ---: | ---: | ---: |
| SPARQL 1.0 data-r2: 30 manifests / 483 cases | 459 | 11 | 13 | 0 | 0 |
| Existing corpus: 44 manifests / 867 cases | 862 | 5 | 0 | 0 | 0 |
| Combined | 1,321 | 16 | 13 | 0 | 0 |

The eleven new expectations are narrowly cited historical semantics (one old
plain/typed string ordering and four `KnownTypesDefault2Neq` ill-typed literal
expectations) or permitted numeric representations (six arithmetic fixtures),
following [RDF 1.2 §3.4.1](https://www.w3.org/TR/rdf12-concepts/#section-Graph-Literal)
and [XSD 1.1 §3.3.4–5](https://www.w3.org/TR/xmlschema11-2/#float).
Strict comparison, immutable upstream answers and XPASS discipline remain intact.
The four old comparison fixtures have 42/52/52/10 rows, while the independently
verified current truth tables require 34/44/44/18. Genuine language/date and
OPTIONAL bugs are repaired, rather than put in the ledger. The existing five
expectations and the current budget remain unchanged.

The 13 remaining unledgered failures are the protected parser owner's TRUE and
twelve grammar cases: `case-insensitive-booleans`; `syntax-forms-02`;
`syntax-lists-03/04/05` in syntax-sparql1; `syntax-lists-05` in syntax-sparql2;
and `filter-missing-parens`, `syn-bad-02/03/05/06/07/14` in syntax-sparql3.
Their fixes are owned by [PR #428](https://github.com/Blackcat-Informatics/purrdf/pull/428)
and will be integrated only after merging to main. No protected worktree or PR
was altered. Final issue qualification and PR creation remain open until all
483 cases have zero unledgered failures and the remaining package/matrix/gates
pass; no product-gap budget is increased to hide these cases.

Local evidence: `/tmp/purrdf-418-optional-shape-before.log`,
`/tmp/purrdf-418-equality-paths-before.log`,
`/tmp/purrdf-418-product-native-final.log`,
`/tmp/purrdf-418-triage-full-corpus.log`,
`/tmp/purrdf-418-eleven-expectations-ledger.log`,
`/tmp/purrdf-418-product-{clippy,rustdoc,fmt,helpers,freeze,commit,push}.log`.

All new validation used ordinary repository commands at four Cargo jobs. No
toolchain state or adapters were changed or specially selected, and no generic
WASM semantic tests were added.


### paudley — 2026-10-05T04:19:55Z

Qualification checkpoint at signed, pushed commit `a68877ec3c2b0218f1477f6710427338c19bce7b` (includes merged parser-diagnostics main `85d172070`).

The native inventory regression found that a syntactically valid empty `[roots]` registry made the existing byte-freeze guard return exit 0 with `OK: 0 vendored corpora byte-frozen.` The same regression now passes after narrowly validating the nonempty root-to-receipt mapping at import. It also checks missing/malformed data, non-string/empty receipt values, and the valid exact-path neighbor. Every registered root/receipt and the freeze algorithm are unchanged. The legacy Python guard remains 47 lines smaller than before the registry extraction; all new expectations are native Rust.

Post-main qualification:

| Native surface | Result |
| --- | --- |
| Algebra / evaluator / harness unit tests | 454 / 1370 / 39 passed |
| Independent data-r2 carrier/value/type/source oracles | 11 passed |
| Exact import/discovery/registry inventory | 3 passed |
| Strict ledger uniqueness/anchoring | 3 passed |
| All-target Clippy, warning-denied rustdoc, formatting, normal commit hooks | Passed |
| Frozen corpora / vendored license roots | 27 / 12 passed |

The full corpus rerun still grades all 74 manifests / 1350 cases: **1321 pass, 16 precise expectations, 13 fail, zero XPASS and unmodeled cases**. The new data-r2 tree is **459 pass / 11 expectations / 13 fail** across all 483 cases; the existing tree is **862 pass / 5 expectations / zero fail** across 867 cases. The 13 parser failures remain unledgered and belong to protected PR #428. They will be regraded only after its changes reach merged main. No product-gap budget was raised and no upstream payload was edited.

The crate description, corpus location/inventory, actual native commands and changelog now explain this import and the OPTIONAL/equality repairs. The DAWG contract for [literal-node equality](https://www.w3.org/2001/sw/DataAccess/tests/README.html) is cited beside the exact representation expectations. Generated scoreboards, complete package/matrix gates and PR creation remain pending zero unledgered failures; this is a verified checkpoint, not an issue-completion claim.


### paudley — 2026-10-05T04:55:00Z

Findings from a parallel check of the #418 branch, for the session finishing this issue. No changes were made to its worktree.
- **Vendored bytes:** all 875 files match upstream rdf-tests `426c7df4b5d5d292e3ba09dc22e622ea301f230a`. The one exception is the documented rename of the root `manifest.ttl` to `manifest-all.ttl`.
- **The 13 remaining failures are all fixed by PR #428.** They are case-insensitive-booleans; syntax-forms-02 and syntax-lists-03/04/05 in syntax-sparql1; syntax-lists-05 in syntax-sparql2; and filter-missing-parens and syn-bad-02/03/05/06/07/14 in syntax-sparql3. With #428 merged into a scratch copy, the sparql shard is 1,334 pass, 16 ledgered, 0 fail, and the new corpus is 472 pass, 11 ledgered. Merging #428 conflicts in two places, `crates/sparql-eval/src/expr.rs` and `crates/sparql-algebra/src/parser/machine.rs`, and both resolve by keeping both sides. These failures cannot be ledgered: the ledger takes only cited normative divergences. So #418 should land after #428 merges, with main merged back in.
- **The six numeric-representation divergences (`"6"` vs `"6.0E0"`) must stay ledgered, not be compared by value.** The W3C DAWG test README requires results to have identical IRI and literal nodes, and the existing runner already compares SPARQL 1.1 literals exactly. The four open-eq entries and dawg-sort-11 already carry RDF 1.2 / SPARQL 1.2 citations.
- **Still to do after the merge:** raise the baseline ledger budget (5 → 16, with a reason), regenerate the CONFORMANCE.md table and the README and book counts, adjust the check-doc-claims patterns for the new wording, and add CHANGELOG entries.

