# T15 review (Addendum A Task 15, A-R3 / G-03)

## Scope
Reviewed only Task 15's uncommitted edits: product_corpus_equivalence.rs, vocabulary_import_invariance.rs,
shacl_shacl_differential.rs (doc only), prebinding.rs (VALUES message), shacl_corpora/mod.rs (`profile`
doc comment only), docs/CONFORMANCE.md row ~129. Task 14's edits were not judged.

## Evidence (commands run)
- `cargo test --locked -p purrdf-shapes --test product_corpus_equivalence --test vocabulary_import_invariance --test shacl_shacl_differential --test product_determinism`: 1 + 2 + 1 passed, product_determinism 7 passed / 1 ignored (pre-existing ignore), 0 failed.
- `cargo test --locked -p purrdf-shapes --lib prebinding`: 18 passed.
- `python3 scripts/check-doc-claims.py`: OK, 154 claims agree.
- `cargo clippy --locked -p purrdf-shapes --all-targets -- -D warnings`: clean.

## Verification of each point
1. Both lanes run under `tc.profile`. Product lane: `load_w3c` uses `from_dataset_with_options(... with_profile(case.profile))`;
   `Loaded.profile` feeds `admit_with_options` and `rebuild_with_options`; the first-party corpus pins `ShaclProfile::LEGACY`
   (its previous implicit law), so first-party cases are unchanged. Vocabulary lane: `Input.profile` feeds the parse
   (`from_dataset_with_options`) and the admit/rebuild restore; first-party is LEGACY. Shapes carry their profile into
   validation (`shapes.rs` stores it), and the 6 identical validation errors below are only reachable if the
   editor's-draft law reaches validation in all three lanes.
2. Deleted ledgers: grep for `REFUSED_AT_LOAD` / `W3C12_REFUSED_AT_LOAD` finds no live reference in any .rs/.md/.py/.po outside
   `.stage` logs. Each ledger held exactly one entry (`pre-binding-009`), and that case is graded by the profile now.
   `REFUSAL_LEDGER` stays `&[]`.
3. Declared-failure count is 14, not 13. Counted `mf:result sht:Failure` on `rdf:type sht:Validate` entries: SHACL 1.0 = 7
   (all under sparql/pre-binding). SHACL 1.2 = 7: unsupported-sparql-001/002/003/005/006, sparql/node/sparql-failure-001,
   sparql/node/sparql-syntax-error-001. Four further `sht:Failure` files under inference-rules are `sht:Infer`, not `sht:Validate`,
   and are outside these lanes. Of the 7 (1.2), `sparql-syntax-error-001` is refused at load (W3C12_DECLARED_FAILURES = 1)
   and the other 6 load and fail typed at validation (W3C12_DECLARED_VALIDATION_FAILURES = 6, matching the doc's id list).
   `pre-binding-009` is a plain `sht:Validate` with no Failure; its manifest label says VALUES on a non-pre-bound variable "is permitted".
   Total 7 + 7 = 14. The plan's "14, not 13" claim holds.
4. Re-pinned counts: unloadable 14 -> 8 (7 + 1); W3C12_DECLARED_FAILURES 6 -> 1; agreed 401 -> 407 (415 - 8 - 0);
   agreed on report stays 401, plus 6 agreed errors = 407; DECLARED_REFUSAL_CASES 12 -> 13 (14 declared minus the one
   INVARIANCE_EXCEPTION), and 401 + 13 + 1 exception = 415 = TOTAL. All consistent with inspection.
5. Exactness: new `ERRORED` check fails any agreed error on a case without a declared `sht:Failure`; the new
   `assert_eq!(agreed - agreed_on_report, 6)` fails if the typed-failure set drifts either way; the vocabulary lane asserts
   `refused_at_load.is_empty()` and `not_on_report.len() == 13`. A case flipping between report and error moves 401/6 and fails.
6. shacl-shacl lane: stays on the compatibility law (`from_dataset_with_base`), with the rationale documented on
   `purrdf_refusal` and in the `profile` doc comment in shacl_corpora/mod.rs. Sound: the oracle judges structural validity once
   per shapes file, and dated-law pre-binding refusals are not structural. Test passes.
7. Prebinding message. REC (https://www.w3.org/TR/shacl/, Appendix A pre-binding limitations) says verbatim: "SPARQL queries
   must not contain a VALUES clause". New text: "the compatibility law refuses every VALUES because the 2017 Recommendation's
   pre-binding rules ban VALUES outright in a query with pre-bound variables, which is stricter than the Working Draft's
   mentions-a-pre-bound-variable rule". Accurate. The old text ("frozen W3C pre-binding corpus requires the stricter reading")
   survives only in a `.stage/.../raw/w3c12-measurement-*.log` historical log. No test asserts the old text
   (`prebinding_lanes.rs:85` and the unit test match only the stable prefix).
8. docs/CONFORMANCE.md row ~129: statements check out (8 refused at load = 7 + 1, 6 agreed errors, 401 + 6 = 407, 14 declared);
   check-doc-claims passes. `docs/book/src/validation/shacl.md:1479` still says the compatibility law refuses 009 at load: true.

## Findings
- INFO (no action): the `W3C12_DECLARED_VALIDATION_FAILURES` doc comment says the five pre-binding ones "were refused at load" under the
  compatibility law; correct, and they sit in the agreed bucket now.
- INFO: the `legacy_message` Values text names the "compatibility law" even though only `Rules::Strict` emits it; accurate for that path.
- No LOW/MEDIUM/HIGH findings.

VERDICT: PASS
