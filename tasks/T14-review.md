# T14 review: SHACL law selection on every host (Addendum A, U7, A-G2, A-R1)

Reviewer: independent. Source not modified. Worktree uncommitted since b67f851c4.

## Scope

Reviewed Task 14 files only: crates/cli (cli.rs, lib.rs, validate.rs, README, tests/shacl_profile_cli.rs,
tests/w3c12_validate_cli.rs), bindings/python (src/shacl.rs, __init__.pyi, READMEs,
tests/test_shacl_profile.py), crates/rdf-wasm (src/shacl.rs, src/async_query.rs, xpath_regex/exports.rs,
js/index.d.ts, READMEs, js/tests/shacl-profile.test.mjs), crates/rdf-capi (src/shacl.rs, src/version.rs,
include/purrdf.h, abi snapshot, smoke.c, README), crates/validate (complete.rs, lib.rs,
tests/complete_shacl.rs), `report_graph_outcome` in crates/shapes/src/shacl_corpora/mod.rs,
crates/shapes/tests/support/w3c_earl.rs and the two w3c harness tests, docs/book shacl.md and zh-Hans.po.
Task 15's edits were ignored.

## Evidence (commands run)

| Check | Result |
|---|---|
| `cargo test --locked -p purrdf-cli --test shacl_profile_cli --test w3c12_validate_cli` | 7 + 2 passed |
| `cargo test --locked -p purrdf-validate` | all passed (247 lib, plus all integration binaries) |
| `cargo test --locked -p purrdf-capi` | all passed, including abi_signatures and c_smoke |
| `cargo test --locked -p purrdf-shapes --test w3c12_conformance --test w3c_conformance` | 29 + 12 passed (incl. exact EARL description tests) |
| `python3 scripts/capi-header.py --check crates/rdf-capi/include/purrdf.h` | OK, header matches the cargo-c artifact |
| `python3 scripts/check-shared-helpers.py` | OK (83 ledger jobs hold) |
| helper-census `--non-rust-ratchet --merge-base-with origin/main --target worktree` | OK, 15 changed non-Rust paths, nothing legacy grew |
| clippy `-D warnings --all-targets` on purrdf-cli, -validate, -capi, -python | clean |
| clippy `-D warnings --all-targets` on purrdf-wasm (native) | clean |
| clippy purrdf-wasm for wasm32-unknown-unknown | fails in untouched `purrdf-sparql-algebra` (4 unfulfilled lint expectations); not Task 14, no file there is modified |
| `cargo test -p purrdf-wasm --lib shacl` | 43 passed, including the three new law tests |
| `make check-i18n` | OK, 3045 translated, 0 fuzzy, 0 untranslated, no drift, all 6 gates |
| python `pytest tests/test_shacl_profile.py` (uv sync --locked --group dev) | 8 passed |
| JS `shacl-profile.test.mjs` | NOT run (needs the built npm package); the Rust wasm tests cover the same answers |

## Verification of the brief

1. One decoding home. PASS. `purrdf_validate::requested_shacl_profile` (complete.rs) calls `complete_profile` and
   then `resolve_xpath` for the conflict. The CLI, Python, WASM and C all call it; grep finds no other
   `ShaclProfile::from_id` / `complete_profile` caller outside crates/validate. Unit test
   `a_host_request_selects_one_exact_law_beside_its_xpath_law` covers None, exact, unknown, conflict and REC.
2. Default unchanged. PASS. Every host only touches options when the id is `Some`. CLI keeps the
   `from_dataset_with_base` path for LEGACY and the old product admission path. C keeps `(None, None)` on the
   old body, and the three old exports pass null. Per-host tests assert the default answer: CLI (009 refused by
   default, product restore default exit 0, `--xpath-regex` alone), Python, WASM (sync and async null options),
   C (NULL law refuses 009).
3. Refusal and neighbour per host. PASS for unknown id and XPath conflict on CLI, Python, WASM sync, C
   (each has its own-law neighbour and the REC reverse pair). See F1 for the one untested refusal.
4. C twin. PASS. `purrdf_shacl_validate_to_sarif_law` is additive; existing signatures are unchanged in the
   snapshot diff (one line added). Header is generated and verified by the tool. ABI version: not bumping is
   correct. The repo policy (abi_signatures.rs WHAT TO DO, releases.md pre-1.0 policy) bumps only for an
   incompatible change; version.rs shows 0.9.0 shipped as 3.0.1 and 0.10.0/0.11.0 unshipped, so the new symbol
   rides the open minor, the same way the `*_xpath_regex` and geo families did. The doc paragraph and README
   record it. (F4 note on placement.)
5. CLI 1.2 lane. PASS. `w3c12_validate_cli.rs` discovers via `shacl12_cases()`, asserts exactly 213 Validate
   entries, runs the built binary (`CARGO_BIN_EXE_purrdf`) with `--shacl-profile shacl-ed-2026-10-16`, passes
   `--shapes-graph` and `--conformance-disallows`, and grades with `report_grading::grade`, the same function the
   in-process harness calls. `sht:Failure` entries must exit 1. The control test grades a produced report against
   its own expectation (pass), then a flipped verdict, a dropped result and `sht:Failure` (each must fail), so the
   grader is shown able to fail. Result: 213/213.
6. `report_graph_outcome` is not a second grader. PASS. It is a reducer only: it feeds `expected_multiset` /
   `expected_tuple` (the manifest reader's own functions) and returns `(bool, Multiset)` for `grade`. The CLI lane
   does not check messages, annotations or details that `grade_report_details` checks in-process; this is stated
   in the doc comment and the lane header (F3).
7. `--shapes-product` deviation. SOUND. `shacl pack` parses under the compatibility law, so 009 cannot be packed.
   The test packs `prefixes-003` and restores one product three ways: default exit 0, WD exit 1 (undeclared
   prefix), ED exit 0 with a report equal to the default. The WD refusal of a product the default accepts proves
   the law comes from the request. The same WD/ED pair is used on all hosts, matching the Task 13 correction.
8. EARL description. PASS. `description()` states commit, suite commit, law, host spellings for all four hosts,
   and, for the 1.2 suite only, the law-independence of node-expression and SPARQL 1.2 RL records.
   `check_descriptions` rebuilds the full sentence independently and compares for equality (not `contains`), for
   both the 1.2 (true) and 2017 (false) suites. The law-independence claim matches the harness: the node-expr
   and `srl` runners take no profile (comment in w3c12_conformance.rs), while `sht:Validate` and rule cases run
   under the ED. The 2017 description correctly omits it.
9. WASM async option refusal on other twins. Implemented (`select_shacl_profile` returns an error for any
   variant without a law slot; `begin_shacl_job` maps it to OPTIONS_CODE) but untested. See F1.
10. Python stub and README. PASS. `.pyi` signatures and comments for `Shapes.__init__` and `shapes.validate`
    match the Rust signature. README and zh-Hans README agree. Manually confirmed that a `Shapes` built with the ED law
    refuses `validate_nt(xpath_regex="xpath-2.0-...")` with the "requires xpath-3.1" message (F2: untested).
11. Docs. PASS. Book section "Dated SHACL laws", CLI/C/WASM/Python READMEs, index.d.ts (`ShaclLaw`,
    `AsyncShaclValidationOptions.shaclProfile`) agree with the code; `check-i18n` passes with zero drift.

## Findings

- F1 (MEDIUM, blocking): the WASM async refusal of `shaclProfile` on a non-validation SHACL twin has no test.
  `crates/rdf-wasm/src/shacl.rs` `select_shacl_profile` (`"shaclProfile applies only to a validation twin"`) and
  its call in `begin_shacl_job` are unreferenced by any Rust or JS test (`grep` of the message and of
  `shaclProfile` in tests finds only the validate twin). Every other refusal in this task has a test and a valid
  neighbour; this one is a new refusal path (a silent-ignore guard) shipped unproven. Fix: add one test (Rust in
  async_query.rs tests, or in shacl-profile.test.mjs) that passes `{ shaclProfile: ED }` to another twin (for
  example the lint or entail async twin) and expects the options-code rejection, beside the validate twin
  accepting it.
- F2 (LOW): Python `Shapes(..., shacl_profile=ED)` followed by `validate_nt(xpath_regex=<other law>)` is refused
  correctly (verified by hand) but not pinned by a test. Add one assertion to
  `test_the_shapes_constructor_carries_the_law_into_every_validation`.
- F3 (LOW, informational): the CLI lane grades verdict and result tuples only, not messages or details. It
  matches the documented claim ("graded by the same grader" is true of `grade`, not of `grade_report_details`).
  The book sentence "graded against the approved results by the same grader" is accurate but could say
  "result tuples". No action required.
- F4 (INFO): the C-twin sentence is added inside the `0.9.0 -> 0.10.0` section of version.rs although the live
  minor is 0.11. That section already holds the other additive families that ride the unshipped minor, so it is
  consistent; no bump is correct.
- F5 (INFO): `cargo clippy -p purrdf-wasm --target wasm32-unknown-unknown -D warnings` fails in
  `purrdf-sparql-algebra` (unfulfilled lint expectations on 32-bit). Pre-existing, outside Task 14.
- F6 (INFO): the C smoke test pins the twin's behaviour but does not compare `law(NULL, NULL)` byte for byte with
  the old export on the same input. The old exports pass null into the same body, so risk is low.
- F7 (INFO): the JS test was not executed in this review (needs the built package).

## Verdict

One medium finding (F1: an untested new refusal path) against the plan's rule that every refusal has a tested
neighbour. Everything else verified.

VERDICT: BLOCKED

## Recheck (orchestrator)
F1 resolved: crates/rdf-wasm/src/async_query.rs test a_shacl_profile_option_is_refused_by_a_twin_that_takes_no_law (refusal with options code; entail without option and validate with option both complete); purrdf-wasm 275 passed. F2 resolved: bindings/python/tests/test_shacl_profile.py pins the ED Shapes xpath conflict and its neighbours (9 passed). wasm32 clippy failure in purrdf-sparql-algebra reproduced identically on main 9a6055e8a: pre-existing, crate untouched by this branch.
VERDICT: PASS
