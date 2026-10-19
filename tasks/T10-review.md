# T10 review: Stage 1 Task 10 (PurRDF #570)

## Scope
Uncommitted tree over 1af88c643. Part (a): frozen root vectors/shacl-reports/, its registrations, and test
w3c_2017_earl_matches_wg_inventory. Part (b): the 1.2 reports-page evidence in the W3C workspace. No source modified.

## Evidence
Part (a)
- Byte equality: `git show 4cd469867...:data-shapes-test-suite/reports/alltests.ttl | cmp - vectors/shacl-reports/alltests.ttl` -> equal. sha256 aa5b294b... matches vectors-shacl-reports.sha256.
- Registries: roots.toml, the .sha256 manifest, LICENSING.md (table row plus check-corpus-frozen list), license-inventory.toml (own [[material]] with the same license, notices and provenance README as shacl), .gitattributes (-text -diff, like shacl12). `vectors/shacl` has no .gitattributes line, consistent with it being text-normalised; shacl12 precedent followed. No PROVENANCE.md entry exists for vectors/shacl or shacl12 (shacl12's is per-directory), none needed. check-corpus-frozen and check-licenses pass ("30 vendored corpora byte-frozen"; "13 vendored root(s) license-clean").
- README: gives upstream, pinned commit, path, license, copy command, freeze mechanism and consumer test. No issue numbers or process text. Accurate (120 approved, 1 proposed confirmed by the test).
- Test vs U2: parse errors fail (`expect` on inventory, statements, assertions); `assertions` errors on a second assertion for a test (each once); asserted key set == the inventory's approved subjects exactly (BTreeSet equality, so omission or addition fails); count of earl:test triples == approved; proposed subjects absent; approved count 120 and proposed == W3C_PROPOSED_UNINCLUDED.len(); passed/failed/total from the report equal the parsed printed `W3C 2017 subset` line, total 120. The report is produced by the same code path as the scoreboard (earl_report/earl_bytes_for_sample), so the equality is non-vacuous.
- Runs: `cargo test -p purrdf-shapes --test w3c_conformance` 12/12 pass (printed line: passed 120, spec-contradicted 0 of 120); check-corpus-frozen OK; check-licenses OK; helper-census --no-features rc 0, no output; clippy -D warnings -p purrdf-shapes --all-targets clean.

Part (b)
- check-purrdf-earl.cjs throws (exit 1) unless all of: a messages.trim()==='' ; b 0 pageErrors and 0 consoleErrors (listeners attached before goto); c exactly one PurRDF section name and one PurRDF summary column; d zero 'no data' cells over every group table, with a non-empty outcome tally (all 382 are `passed`); e no PurRDF section containing 'Unknown tests'; f each class's Summary cell starts with `N / N` for Core 162, SPARQLValidation 33, NodeExpr 146, InferenceRules 31, SPARQLNodeExpr 5, SPARQLCustomFunctions 5. These are assertions, not only records. Result JSON shows all six checks true, messages empty, no errors.
- Expected counts match the harness lines from `cargo test --test w3c12_conformance` (same six class lines, passed N of N).

## Findings (non-blocking)
1. The class counts in the script are hard-coded, not read from the harness output; they match today. Fine for a one-time local check.
2. Check (c) also hard-codes "PurRDF 3.0.1"; fine for the recorded run.
3. docs/CONFORMANCE.md's vendored-corpora list (lines ~158-172) does not mention vectors/shacl-reports; optional doc addition, not required by the plan's Task 10 list.
4. validation.md Task 9/10 text elsewhere quotes older matrix numbers (571/6/6); the current w3c12 run prints 591 passed, 0 spec-contradicted. Evidence for Task 10b (382 over six classes) is consistent with the current run. Worth refreshing the stale lines before the final.

No blocking defects.

VERDICT: PASS
