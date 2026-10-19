# T13 independent review (Addendum A: A-G1, A-G1a, A-R2, A-R13)

## Scope

Uncommitted changes since `114b98530` (35 files). Read: `git diff` for the whole change; the
dated-law contract in `crates/shapes/src/profile.rs`; `tasks/T13-triage.md`; gap-analysis
G-02 and G-13. Spec text read from `/home/paudley/Active/w3c/data-shapes`. No source modified.

## Verdicts per check

1. **ED law follows the dated-law contract: PASS.**
   - `profile.rs`: `Identity::EditorsDraft`; id `shacl-ed-2026-10-16`; `from_id` exact match
     (`shacl-ed-2026-10-17` refused in `profile_admission.rs`); core and sparql specification
     URLs pinned to commit `4cd469867a1cc6d2fff80e8a40723b26c695d67a`; XPath URL and profile
     are XPath 3.1 (the `WorkingDraft | EditorsDraft` arms). No nearby-edition mapping.
     `git show -s --format=%cs 4cd46986` prints `2026-10-16`, so the id date is right.
   - Census: every enumeration of `REC_20170720`/`WD_20260918` now includes the ED:
     `admission.rs` `DATED`, `complete_reports.rs` (five loops/tables), `prebinding.rs` test loop,
     `dated_execution.rs` `PROFILES`, `profile_admission.rs` `DATED`, `xpath_bundle.rs`,
     `sparql-conformance/src/community.rs` `DATED` (catalog `select` is by exact specification
     URL, so the ED resolves only by its own pinned URL), `validate/tests/complete_shacl.rs`
     (`complete_profile` is id-driven through `from_id`; id round-trip and unknown-id tests
     present), `closed_property_shape.rs`. The CLI, Python, wasm and C hosts pass the id string
     to `from_id` and enumerate no law, so nothing to add there.
   - Sites that remain `== WD_20260918` are intentional: `prefixes.rs:248` (the WD-only path
     variant; the ED and LEGACY go through `reads_editors_draft_semantics`), and tests.
     `complete_profile`/`select` need no per-law edits.

2. **`is_shacl12()` replaced exactly the five WD-equality sites: PASS.** `prebinding.rs:219`,
   `sparql.rs:1662`, `sparql.rs:2452`, `report.rs:1125`, `constraints.rs:3529`. No remaining
   `== WD_20260918` outside `prefixes.rs:248`. LEGACY is not `is_shacl12()`, so its behaviour
   at those five sites is unchanged (it fell through before too).

3. **WD/ED classification: PASS.** `32318223` is dated 2026-09-19 07:33 +1000 (2026-09-18 UTC),
   so it is the WD-era text. Greps of `32318223` against `4cd46986`:
   - SPARQL constraint `sh:deactivated`: WD "this value is either true or false" (line 783-784);
     ED "as for shapes" plus the `expr` evaluation sentence. Post-WD.
   - Prefix path: WD `(^owl:versionIRI?/owl:imports)*`; ED `(^owl:versionIRI)?/(owl:imports/(^owl:versionIRI)?)*`. Post-WD.
   - Blank-node argument: sentence absent at `32318223`, present at `4cd46986` (line 2182). Post-WD.
   - Rule sets as `sh:entailment`: "Entailment using Rule Sets" heading absent at `32318223`,
     present at `4cd46986`; commit `2ada6ac4` (2026-10-12) is not an ancestor of `32318223`.
     `07458ebb` is not an ancestor either.
   - Already WD (spot-checked): Appendix A "MUST not contain a VALUES clause that mentions any
     potentially pre-bound variable" (WD line 2028); Core "Deactivating Shapes" node expression on
     shapes and on reifiers (WD core lines 1674-1696); the "exactly one member" function-call rule
     (WD line 1988) and the optional-argument "unbound in the scope" rule (WD 1978-1979).
   The four post-WD rows and the WD rows in the triage hold.

4. **Each refusal has a neighbour; tests depend on the gate: PASS.**
   - Deactivation (`shape_deactivation_expression.rs`): four positions; REC refused everywhere,
     WD refused at SPARQL and `sh:expression` positions and accepted at shape and reifier
     (neighbours), booleans load under REC/WD/ED; ED and LEGACY accept; validating test shows
     WD `true`/`false` constants behave and the expression is refused. `expect_err` under WD fails if the gate goes.
   - Prefixes (`sparql_prefixes.rs`): case 1 separates REC from WD/ED, case 2 separates REC and
     WD from ED, plus two all-law neighbours (declaration on the node, plain import).
   - Blank argument (`custom_node_expressions.rs`): ED `"false ..."` against WD `"true ..."`, with
     the IRI call beside it unchanged under both; removing the gate changes the WD value.
   - Rule set entailment (`rules_engine.rs`): WD and REC refuse with a message naming the ED id,
     ED and LEGACY run it; neighbours `sh:RulesEntailment` and no entailment pass under every law.
   - `w3c12_conformance.rs::the_entries_resting_on_post_wd_text_need_the_editors_draft_law` pins
     6 post-WD approved entries (must fail under WD) and 7 WD-text entries (must pass under both).
   Reasoned, not mutation-run (no source modification allowed): every refusal assertion would
   change outcome if the `reads_editors_draft_semantics` gate were removed.

5. **REC prefix path: PASS.** `reached_by_imports(.., versioned=false)` is `owl:imports*` from the
   `sh:prefixes` value; no `owl:versionIRI` step. WD variant (`versioned=true`) and the ED
   `reached_by_editors_draft_path` match their quoted property paths when traced.

6. **LEGACY unchanged: PASS.** `reads_editors_draft_semantics` is true for LEGACY, so it keeps
   the prior path, blank-node, deactivation and entailment behaviour. Full
   `cargo test --locked -p purrdf-shapes` passes (0 failures; includes the 12 `w3c_conformance`
   tests with the `af/` lane on LEGACY) and `purrdf-sparql-conformance --test community_conformance` passes.

7. **2017 harness under REC: PASS.** `w3c_conformance.rs`: `LAW = REC_20170720`, applied to
   non-first-party entries, `Suite.law = LAW.id()`; a scratch pass shows 120/120. `w3c_earl.rs`
   adds `law` to `Suite`, the description reads "...at commit X, graded under the Y profile", and
   `check_descriptions` demands the sentence. The 1.2 suite uses `ED_20261016.id()`. Sentence
   correct for 2017 and 1.2.

8. **Docs and comments: PASS with notes.**
   - `CHANGELOG.md` is hand-authored in `[Unreleased]` (earlier commits edited it by hand;
     `make changelog` refuses to regenerate over it via `scripts/check-changelog-handwritten.py`).
     The hand edit is consistent with the repo's practice; CONTRIBUTING also asks for
     conventional-commit subjects, which is a commit-time matter.
   - `docs/CONFORMANCE.md`, `docs/book/src/validation/shacl.md`, `crates/shapes/README.md`,
     `zh-Hans.po` (translations present, `msgfmt --check` ok, old entry moved to obsolete `#~`)
     all attribute the four rules to the ED. No remaining claim that post-WD text is in the WD.
   - No issue numbers or process text in added lines.

## Commands run

- `cargo test --locked -p purrdf-shapes --test w3c12_conformance --test w3c_conformance --test shape_deactivation_expression --test sparql_prefixes --test custom_node_expressions --test rules_engine --test profile_admission --test dated_execution`: all ok (29, 12, 12, 10, 30, 33, 14, 16 passed).
- `cargo test --locked -p purrdf-validate`: all ok.
- `cargo test --locked -p purrdf-shapes` (full) and `-p purrdf-sparql-conformance --test community_conformance`: ok.
- `python3 scripts/check-shared-helpers.py`: OK. `python3 scripts/check-doc-claims.py`: OK.
- `cargo clippy --locked -p purrdf-shapes --all-targets -- -D warnings`: clean.

## Findings

| # | Severity | Finding |
|---|---|---|
| F1 | LOW | `scripts/publish-github-release.py` changes an error string ("four distribution assets" to "distribution assets"). It has nothing to do with Task 13; confirm it belongs to the commit, or split it out. |
| F2 | LOW | The 1.2 EARL subject description says all records were "graded under the shacl-ed-2026-10-16 profile", but the node-expression (`sht:EvalNodeExpr`) records are run by a loader that takes no law. `w3c12_conformance.rs` carries a comment saying so; the published report text does not. Accurate enough, not misleading about the validate/rule records. |
| F3 | INFO | `crates/cli/README.md:822` quotes SHACL 1.2's `^owl:versionIRI?/owl:imports` for import closure; that is the WD wording, the ED spells it differently. It is about import anchoring, not prefix collection, and no post-WD claim is made. |
| F4 | INFO | Cosmetic: a few rewrapped doc comments (`rules.rs` ~315, `rule_parse.rs` ~154, `custom_node_expressions.rs` ~501) have uneven line lengths. |
| F5 | INFO | Check 4 is by reasoning plus the tests passing under all laws, not by running a mutation. |

No blocking findings.

VERDICT: PASS
