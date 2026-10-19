# Validation log

## Task 1: re-vendor and harness bookkeeping

### Drift against upstream (R3/R4)

- `diff -rq --strip-trailing-cr /home/paudley/Active/w3c/data-shapes/data-shapes-test-suite/tests/core vectors/shacl/core`: 0 lines.
- Same for `.../tests/sparql` against `vectors/shacl/sparql`: 0 lines.
- `git -C /home/paudley/Active/w3c/data-shapes log --oneline 08adb3776709a014bc3062ede793c36275b22446..4cd469867a1cc6d2fff80e8a40723b26c695d67a -- data-shapes-test-suite/tests`: empty.

### Focused checks

- `python3 scripts/check-corpus-frozen.py --update` then `python3 scripts/check-corpus-frozen.py`: OK, 29 roots. Only `shapes-spec.sha256` and `vectors-shacl12.sha256` changed. `vectors-shacl.sha256` is unchanged because the freeze excludes `README.md`.
- `cargo test -p purrdf-shapes --test w3c12_conformance`: 17 pass; `w3c_shacl12_conformance` and `every_planned_inventory_id_passes` fail, on exactly the 14 expected entries (`closed-001`; 4 `rules-entailment-ruleset-*`; `blankNodeArgument-001`, `singleInstanceOf-001`; `deactivated-select-001`, `prefixes-003`, `sparql-failure-001`, `sparql-message-001`; `pre-binding-009`; stratification `test_13`, `test_14`). The inventory test fails only on `test_13`/`test_14`. Scoreboard: `W3C12 TOTAL: passed 571, non-canonical-expected-decimal 6, xfailed 0, ledger 0`; `W3C12 UNLISTED: passed 1, exact 1, with-delta 0, total 1`.
- `cargo test -p purrdf-shapes --test w3c_conformance`: 129/129.
- `cargo test -p purrdf-iri`: pass.
- `cargo test -p purrdf-shapes` (all targets): only `w3c12_conformance` fails (above). Re-vendor fallout fixed: `product_corpus_equivalence`, `vocabulary_import_invariance`, `spec_census`, `shacl_shacl_differential`, `host_entry_points` all pass.
- `cargo test -p purrdf-cli --test w3c_node_expr_cli --test validate_cli --test shapes_tools_cli`, `-p purrdf-validate`, `-p purrdf-conformance-kit`: pass. `--test w3c_srl_check_cli` fails only on stratification `test_13`/`test_14`.
- `python3 scripts/conformance-matrix.py --self-test`: OK.
- `conformance-matrix.py --shard shapes`: only the SHACL 1.2 row is RED (571 / 6 / 6 / 0); unlisted 1; prepared-product 395.
- `cargo run -q --locked -p helper-census -- --no-features`: clean. `cargo run -q --locked -p helper-census -- --non-rust-ratchet --merge-base-with origin/main --target worktree`: OK.
- `cargo clippy -p purrdf-shapes -p purrdf-iri -p purrdf-cli --all-targets`: warning-free. `cargo fmt --check` on the three crates: clean.
- `python3 scripts/check-doc-claims.py`: fails at the invariant "matrix pass + XFail/Skip + unlisted = discovered" (577 + 1 != 592), because 14 approved entries do not pass.

## Task 9 clean-tree run (orchestrator)
`make w3c-earl` at 1af88c643 on a clean tree: exit 0; wrote target/w3c-earl/purrdf-shacl12-earl.ttl (281286 bytes) and purrdf-shacl-earl.ttl (88253 bytes); git status clean afterwards. Dirty-tree refusal prints the porcelain status. actionlint clean on ci.yaml and release-cargo.yaml.

## Task 10b: WG 1.2 reports page (W3C workspace, local only)
Page `shacl12-test-suite/reports.html` at data-shapes 4cd46986 (detached local worktree, removed afterwards), `?reports=topquadrant-shacl-earl.ttl,purrdf-shacl12-earl.ttl`, EARL from `make w3c-earl` at 1af88c643, Playwright v1.64.0 Chromium.
- #messages empty; 0 page errors; 0 console errors; one PurRDF 3.0.1 column; 0 "no data" cells; no "Unknown tests" row.
- Per class: Core 162/162, SPARQLValidation 33/33, NodeExpr 146/146, InferenceRules 31/31, SPARQLNodeExpr 5/5, SPARQLCustomFunctions 5/5 (382).
- Evidence: /home/paudley/Active/w3c/contributions/data-shapes/purrdf-earl/browser/{check-purrdf-earl.cjs,purrdf-earl-page-result.json}.

## Task 11 (W3C workspace drafts; no PurRDF commit)
SPEC_CONTRADICTED is empty in both harnesses, so no WG issue is required for R28. Drafts (not posted; WG-facing, approval-gated): /home/paudley/Active/w3c/contributions/data-shapes/purrdf-earl/drafts/w3c-pr.md (R23) and drafts/wg-findings.md (discussion items and editorial-clarification candidates from T3-T7 triage).

## Final gates
- make conformance: GREEN at 4e007c462 (all suites; Python binding suite 4223 pass; SHACL 1.2 591/591; SHACL 1.0 129; SPARQL 1393).
- make check: the run at 4e007c462 stopped at its first failing target, cdt_lexical_divergence (Makefile:172); the targets after it (preserve_order_consumer tests, rdf-core-hygiene, wasm) did not run there. The failure was (pin 4161 vs 4162, the new vectors/shacl-reports file, which has no CDT literals); fixed in aec3a6dcb and that test passes. A full make check rerun at aec3a6dcb was stopped by Patrick before completion (he directed proceeding to the PR); it is not claimed green.

- Correction (Stage 3 completion audit F1/F4): an earlier note said that run failed "only" on the CDT pin; make stops at the first failure, so later targets were not exercised. Qualification at the final head is hosted CI on the exact SHA.
- gh-pages recheck (Stage 3): origin/gh-pages is still 4cd469867a1cc6d2fff80e8a40723b26c695d67a; no newer suite commits.

## Task 13: the editor's-draft law (A-G1)
- Law date: `git -C /home/paudley/Active/w3c/data-shapes show -s --format=%cs 4cd469867a1cc6d2fff80e8a40723b26c695d67a` prints `2026-10-16` (author/commit time 2026-10-16 11:40:14 +1000). The law id is `shacl-ed-2026-10-16`, as planned.
- WD-era reference: `32318223` (2026-09-19 07:33 +1000, i.e. 2026-09-18 UTC), the last commit that touches `shacl12-core`, `shacl12-sparql` and `shacl12-inference-rules` on or before the SPARQL WD date of 2026-09-18 UTC; `shacl12-node-expr` last changed at `4199e812` before it.
- Audit: `tasks/T13-triage.md`.
- Checks on the Task 13 working tree (uncommitted): `cargo test --locked -p purrdf-shapes --no-fail-fast`: 2189 passed, 0 failed; `W3C12 TOTAL: passed 591, spec-contradicted 0, xfailed 0` (all graded under `shacl-ed-2026-10-16`); `W3C 2017 subset: passed 120` of 120 and `TOTAL: passed 129` (the 120 under `shacl-rec-2017-07-20`). `-p purrdf-validate` and `-p purrdf-sparql-conformance --test community_conformance`: pass. `-p purrdf-cli --test shacl_cli --test shapes_tools_cli --test validate_cli --test w3c_node_expr_cli --test w3c_srl_check_cli`: pass. clippy `-D warnings` (shapes, validate, sparql-conformance, all targets): clean. `cargo fmt --check`, `cargo check --workspace --all-targets`, `helper-census --no-features`, `check-doc-claims.py`, `check-issue-refs`, `check-spec-attribution`, `make check-i18n` (3036 messages, 0 fuzzy): pass.

## Task 22 full gates (merged head)
- make check: exit 0 at 9b6608f60.
- make conformance --write-doc: VERDICT GREEN at 9b6608f60 (SHACL 1.2 590 + 1 spec-contradicted; SHACL 1.0 129; prepared-product 407; Python 4232). Matrix block regenerated and committed as 142c8faf411157263f7b79ff138cb83a4dc39936 (docs only).

## Task 22: WG 1.2 reports page re-check (W3C workspace, local only)
EARL from `make w3c-earl` at 142c8faf4 (clean tree; output checks passed), data-shapes 4cd46986 detached worktree (removed after), `?reports=topquadrant-shacl-earl.ttl,purrdf-shacl12-earl.ttl`, Playwright v1.64.0 Chromium. PASS: messages empty; 0 page/console errors; one PurRDF 3.0.1 column; 0 no-data cells; no Unknown-tests row; classes Core 162/162, SPARQLValidation 33/33, NodeExpr 145/146, InferenceRules 31/31, SPARQLNodeExpr 5/5, SPARQLCustomFunctions 5/5; outcomes passed 381 + failed 1, the failed cell exactly node-expr/shnex-sparql/hasLang-example (U10). Evidence: purrdf-earl/browser/check-purrdf-earl.cjs and purrdf-earl-page-result.json (v1 copies kept).
